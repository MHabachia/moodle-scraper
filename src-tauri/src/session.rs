//! Anmeldung und HTTP-Zugriff: Login-Fenster, Cookie-Jar, Keep-Alive und Kursliste.
//! Das Session-Cookie geht nur an den Moodle-Host.

use crate::log;
use crate::parse::{is_logged_in, parse_courses_html, parse_sesskey, parse_user};
use crate::secure;
use crate::types::{Course, SessionInfo};
use crate::util::{is_private_host, is_private_ip, lock, normalize_url, redact};
use reqwest::cookie::{CookieStore, Jar};
use reqwest::dns::{Addrs, Name, Resolve, Resolving};
use std::net::SocketAddr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tauri::{AppHandle, Manager, State, Url, WebviewUrl, WebviewWindowBuilder};

/// Wir geben uns als das aus, was wir sind.
pub const USER_AGENT: &str = concat!("MoodleScraper/", env!("CARGO_PKG_VERSION"));
pub const LOGIN_WINDOW: &str = "login";
pub const SESSION_EXPIRED: &str = "SESSION_EXPIRED";

/// Löst Namen auf und lässt dabei nur öffentliche Adressen zu, außer für den Moodle-Host selbst
/// (der darf auch in einem Uni-Netz liegen). Das schützt davor, dass ein Name auf eine interne
/// Adresse zeigt, etwa bei einer Weiterleitung oder einem Datei-Link.
struct PublicOnlyResolver {
    allowed_host: String,
}

/// Behält nur öffentliche Adressen, wenn der Host nicht ausdrücklich erlaubt ist.
pub(crate) fn filter_addrs(addrs: Vec<SocketAddr>, allowed: bool) -> Vec<SocketAddr> {
    if allowed {
        addrs
    } else {
        addrs.into_iter().filter(|a| !is_private_ip(a.ip())).collect()
    }
}

impl Resolve for PublicOnlyResolver {
    fn resolve(&self, name: Name) -> Resolving {
        let host = name.as_str().to_string();
        let allowed = host.eq_ignore_ascii_case(&self.allowed_host);
        Box::pin(async move {
            let found: Vec<SocketAddr> = tokio::net::lookup_host((host.as_str(), 0)).await?.collect();
            let usable = filter_addrs(found, allowed);
            if usable.is_empty() {
                return Err("Adresse zeigt auf ein internes Netz (blockiert)".into());
            }
            Ok(Box::new(usable.into_iter()) as Addrs)
        })
    }
}

#[derive(Clone)]
pub struct Ctx {
    pub client: reqwest::Client,
    /// Folgt keinen Weiterleitungen, z. B. um das Ziel einer Link-Aktivität zu lesen.
    pub client_no_redirect: reqwest::Client,
    pub base: Url,
    jar: Arc<Jar>,
    pub sesskey: String,
}

impl Ctx {
    /// `cookie_header` kommt als "a=b; c=d" aus dem Login-Fenster. Der Jar schickt Cookies
    /// nur an passende Hosts und übernimmt neue Werte vom Server.
    pub fn new(base: Url, cookie_header: &str) -> Result<Self, String> {
        let jar = Arc::new(Jar::default());
        for pair in cookie_header.split(';').map(str::trim).filter(|p| p.contains('=')) {
            jar.add_cookie_str(&format!("{pair}; Path=/"), &base);
        }

        let base_host = base.host_str().unwrap_or_default().to_string();
        let resolver = Arc::new(PublicOnlyResolver {
            allowed_host: base_host.clone(),
        });
        // Weiterleitungen nur zum selben Host oder zu öffentlichen https-Adressen, nie in interne Netze
        let policy = reqwest::redirect::Policy::custom(move |attempt| {
            if attempt.previous().len() >= 10 {
                return attempt.error("Zu viele Weiterleitungen");
            }
            let u = attempt.url();
            let host = u.host_str().unwrap_or("");
            if host == base_host || (u.scheme() == "https" && !is_private_host(host)) {
                attempt.follow()
            } else {
                attempt.error("Weiterleitung auf ein unsicheres Ziel blockiert")
            }
        });
        let builder = |redirect: reqwest::redirect::Policy| {
            reqwest::Client::builder()
                .user_agent(USER_AGENT)
                .dns_resolver(resolver.clone())
                .cookie_provider(jar.clone())
                .redirect(redirect)
                .connect_timeout(Duration::from_secs(20))
                .read_timeout(Duration::from_secs(60))
                .build()
                .map_err(|e| e.to_string())
        };
        Ok(Ctx {
            client: builder(policy)?,
            client_no_redirect: builder(reqwest::redirect::Policy::none())?,
            base,
            jar,
            sesskey: String::new(),
        })
    }

    pub fn join(&self, rel: &str) -> Result<Url, String> {
        self.base.join(rel).map_err(|e| e.to_string())
    }

    /// Aktuelle Cookies, auch die vom Server erneuerten.
    pub fn cookie_header(&self) -> Option<String> {
        self.jar
            .cookies(&self.base)
            .and_then(|v| v.to_str().ok().map(|s| s.to_string()))
    }

    pub fn get(&self, url: &Url) -> reqwest::RequestBuilder {
        self.client.get(url.clone())
    }

    pub fn head(&self, url: &Url) -> reqwest::RequestBuilder {
        self.client.head(url.clone())
    }

    pub fn get_no_redirect(&self, url: &Url) -> reqwest::RequestBuilder {
        self.client_no_redirect.get(url.clone())
    }

    /// Für Moodles AJAX-Schnittstelle: sie antwortet direkt, eine Weiterleitung wäre ein Fehler.
    pub fn post(&self, url: &Url) -> reqwest::RequestBuilder {
        self.client_no_redirect.post(url.clone())
    }
}

pub struct AppState {
    pub ctx: Mutex<Option<Ctx>>,
    pub pending: Mutex<Option<Url>>,
    pub cancel: Arc<AtomicBool>,
    /// Keep-Alive-Task, hält die Session offen.
    keepalive: Mutex<Option<tauri::async_runtime::JoinHandle<()>>>,
    /// "Angemeldet bleiben": Session im Passwortspeicher des Systems speichern.
    pub remember: AtomicBool,
    /// Beim Schließen des Fensters in den Tray wechseln.
    pub background: AtomicBool,
}

impl AppState {
    pub fn new() -> Self {
        AppState {
            ctx: Mutex::new(None),
            pending: Mutex::new(None),
            cancel: Arc::new(AtomicBool::new(false)),
            keepalive: Mutex::new(None),
            remember: AtomicBool::new(false),
            background: AtomicBool::new(false),
        }
    }

    pub fn ctx(&self) -> Result<Ctx, String> {
        lock(&self.ctx).clone().ok_or_else(|| SESSION_EXPIRED.to_string())
    }

    /// Setzt die aktive Session und startet den Keep-Alive neu.
    pub fn set_session(&self, ctx: Ctx) {
        self.stop_keepalive();
        *lock(&self.keepalive) = Some(spawn_keepalive(ctx.clone()));
        *lock(&self.ctx) = Some(ctx);
    }

    pub fn clear_session(&self) {
        self.stop_keepalive();
        *lock(&self.ctx) = None;
    }

    fn stop_keepalive(&self) {
        if let Some(h) = lock(&self.keepalive).take() {
            h.abort();
        }
    }
}

const KEEPALIVE_SECS: u64 = 240;

/// `core_session_touch` verhindert, dass Moodle die Session wegen Inaktivität beendet.
pub async fn touch(ctx: &Ctx) -> Result<(), String> {
    ajax(ctx, "core_session_touch", serde_json::json!({})).await.map(|_| ())
}

fn spawn_keepalive(ctx: Ctx) -> tauri::async_runtime::JoinHandle<()> {
    tauri::async_runtime::spawn(async move {
        loop {
            tokio::time::sleep(Duration::from_secs(KEEPALIVE_SECS)).await;
            let _ = touch(&ctx).await;
        }
    })
}

pub fn net_err(e: reqwest::Error) -> String {
    if e.is_timeout() {
        "Zeitüberschreitung bei der Verbindung zu Moodle.".into()
    } else if e.is_connect() {
        "Keine Verbindung zu Moodle möglich. Internet/URL prüfen.".into()
    } else {
        redact(&format!("Netzwerkfehler: {e}"))
    }
}

/// Ruft eine AJAX-Funktion von Moodle auf (`lib/ajax/service.php`) und gibt `data` zurück.
pub async fn ajax(ctx: &Ctx, method: &str, args: serde_json::Value) -> Result<serde_json::Value, String> {
    let mut url = ctx.join("lib/ajax/service.php")?;
    url.query_pairs_mut()
        .append_pair("sesskey", &ctx.sesskey)
        .append_pair("info", method);
    let body = serde_json::json!([{"index": 0, "methodname": method, "args": args}]);
    let resp = ctx.post(&url).json(&body).send().await.map_err(net_err)?;
    if !resp.status().is_success() {
        return Err(format!("HTTP {}", resp.status().as_u16()));
    }
    let v: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
    let first = v.get(0).ok_or("Leere Antwort")?;
    if first.get("error").and_then(|e| e.as_bool()).unwrap_or(true) {
        let code = first["exception"]["errorcode"].as_str().unwrap_or("");
        if code == "servicerequireslogin" || code == "invalidsesskey" {
            return Err(SESSION_EXPIRED.into());
        }
        return Err(format!("AJAX-Fehler ({code})"));
    }
    Ok(first["data"].clone())
}

#[tauri::command]
pub async fn open_login(app: AppHandle, state: State<'_, AppState>, moodle_url: String) -> Result<String, String> {
    let base = normalize_url(&moodle_url)?;
    let login = base.join("login/index.php").map_err(|e| e.to_string())?;
    *lock(&state.pending) = Some(base.clone());
    state.clear_session();

    if let Some(win) = app.get_webview_window(LOGIN_WINDOW) {
        win.navigate(login).map_err(|e| e.to_string())?;
        let _ = win.show();
        let _ = win.set_focus();
    } else {
        WebviewWindowBuilder::new(&app, LOGIN_WINDOW, WebviewUrl::External(login))
            .title("Moodle Login")
            .inner_size(1000.0, 760.0)
            .incognito(true)
            .build()
            .map_err(|e| e.to_string())?;
    }
    Ok(base.to_string())
}

/// Fragt eine Moodle-Seite ab und prüft so, ob die Session noch gilt. Merkt sich dabei den Sesskey.
async fn verify_session(ctx: &mut Ctx) -> Result<Option<SessionInfo>, String> {
    let page = ctx.join("my/")?;
    let resp = match ctx.get(&page).send().await {
        Ok(r) => r,
        Err(_) => return Ok(None),
    };
    let html = resp.text().await.unwrap_or_default();
    if !is_logged_in(&html) {
        return Ok(None);
    }
    ctx.sesskey = parse_sesskey(&html).ok_or_else(|| "Login erkannt, aber kein Sesskey gefunden.".to_string())?;
    Ok(Some(SessionInfo {
        base_url: ctx.base.to_string(),
        host: ctx.base.host_str().unwrap_or_default().to_string(),
        user: parse_user(&html),
    }))
}

/// Legt die Session im Passwortspeicher ab, wenn der Nutzer das möchte.
fn persist_if_enabled(state: &AppState, ctx: &Ctx) {
    if !state.remember.load(Ordering::Relaxed) {
        return;
    }
    let host = ctx.base.host_str().unwrap_or_default();
    match ctx.cookie_header() {
        Some(cookie) => {
            if let Err(e) = secure::save(host, &cookie, &ctx.sesskey) {
                log::log(
                    "WARN",
                    &format!("Session konnte nicht im Passwortspeicher gespeichert werden: {e}"),
                );
            }
        }
        None => log::log("WARN", "Keine Cookies zum Speichern vorhanden"),
    }
}

/// Das Frontend ruft das regelmäßig auf, solange das Login-Fenster offen ist.
/// `None` heißt: noch nicht angemeldet.
#[tauri::command]
pub async fn check_login(app: AppHandle, state: State<'_, AppState>) -> Result<Option<SessionInfo>, String> {
    let base = lock(&state.pending)
        .clone()
        .ok_or_else(|| "Kein Login gestartet.".to_string())?;
    let win = app
        .get_webview_window(LOGIN_WINDOW)
        .ok_or_else(|| "LOGIN_WINDOW_CLOSED".to_string())?;

    let cookies = win.cookies_for_url(base.clone()).map_err(|e| e.to_string())?;
    if cookies.is_empty() {
        return Ok(None);
    }
    let header = cookies
        .iter()
        .map(|c| format!("{}={}", c.name(), c.value()))
        .collect::<Vec<_>>()
        .join("; ");

    let mut ctx = Ctx::new(base, &header)?;
    let Some(info) = verify_session(&mut ctx).await? else {
        return Ok(None);
    };
    persist_if_enabled(&state, &ctx);
    state.set_session(ctx);
    let _ = win.close();
    log::log("INFO", &format!("Login erfolgreich ({})", info.host));
    Ok(Some(info))
}

/// Holt eine gespeicherte Session aus dem Passwortspeicher, falls sie noch gilt.
#[tauri::command]
pub async fn restore_session(state: State<'_, AppState>, moodle_url: String) -> Result<Option<SessionInfo>, String> {
    let base = normalize_url(&moodle_url)?;
    let host = base.host_str().unwrap_or_default().to_string();
    let Some(saved) = secure::load(&host) else {
        return Ok(None);
    };
    let mut ctx = Ctx::new(base, &saved.cookie)?;
    ctx.sesskey = saved.sesskey;
    match verify_session(&mut ctx).await? {
        Some(info) => {
            persist_if_enabled(&state, &ctx);
            state.set_session(ctx);
            log::log(
                "INFO",
                &format!("Session aus dem Passwortspeicher wiederhergestellt ({host})"),
            );
            Ok(Some(info))
        }
        None => {
            secure::delete(&host);
            log::log("INFO", "Gespeicherte Session war abgelaufen und wurde gelöscht");
            Ok(None)
        }
    }
}

/// Schaltet "Angemeldet bleiben" um. Beim Ausschalten wird der Eintrag gelöscht.
#[tauri::command]
pub fn set_remember(state: State<'_, AppState>, enabled: bool, moodle_url: Option<String>) {
    state.remember.store(enabled, Ordering::Relaxed);
    if !enabled {
        if let Some(host) = moodle_url
            .and_then(|u| normalize_url(&u).ok())
            .and_then(|u| u.host_str().map(String::from))
        {
            secure::delete(&host);
        }
    }
}

#[tauri::command]
pub async fn logout(app: AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    // Abmelden löscht auch eine gespeicherte Session
    if let Ok(ctx) = state.ctx() {
        secure::delete(ctx.base.host_str().unwrap_or_default());
    }
    state.clear_session();
    *lock(&state.pending) = None;
    if let Some(win) = app.get_webview_window(LOGIN_WINDOW) {
        let _ = win.close();
    }
    Ok(())
}

#[tauri::command]
pub async fn list_courses(state: State<'_, AppState>) -> Result<Vec<Course>, String> {
    let ctx = state.ctx()?;
    match list_courses_ajax(&ctx).await {
        Ok(c) if !c.is_empty() => return Ok(c),
        Err(e) if e == SESSION_EXPIRED => return Err(e),
        _ => {}
    }
    list_courses_html(&ctx).await
}

async fn list_courses_ajax(ctx: &Ctx) -> Result<Vec<Course>, String> {
    let data = ajax(
        ctx,
        "core_course_get_enrolled_courses_by_timeline_classification",
        serde_json::json!({
            "offset": 0, "limit": 0, "classification": "all",
            "sort": "fullname", "customfieldname": "", "customfieldvalue": ""
        }),
    )
    .await?;
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    let list = data["courses"].as_array().cloned().unwrap_or_default();
    let mut out = Vec::new();
    for c in list {
        let id = match &c["id"] {
            serde_json::Value::Number(n) => n.to_string(),
            serde_json::Value::String(s) => s.clone(),
            _ => continue,
        };
        let end = c["enddate"].as_i64().unwrap_or(0);
        out.push(Course {
            url: c["viewurl"]
                .as_str()
                .map(|s| s.to_string())
                .unwrap_or_else(|| format!("{}course/view.php?id={}", ctx.base, id)),
            name: c["fullname"].as_str().unwrap_or("").trim().to_string(),
            short_name: c["shortname"].as_str().unwrap_or("").to_string(),
            category: c["coursecategory"].as_str().unwrap_or("").to_string(),
            progress: c["progress"].as_f64(),
            favourite: c["isfavourite"].as_bool().unwrap_or(false),
            hidden: c["hidden"].as_bool().unwrap_or(false),
            ended: end > 0 && end < now,
            id,
        });
    }
    Ok(out)
}

async fn list_courses_html(ctx: &Ctx) -> Result<Vec<Course>, String> {
    for path in ["my/courses.php", "my/"] {
        let url = ctx.join(path)?;
        let resp = ctx.get(&url).send().await.map_err(net_err)?;
        let html = resp.text().await.map_err(|e| e.to_string())?;
        if !is_logged_in(&html) {
            return Err(SESSION_EXPIRED.into());
        }
        let courses = parse_courses_html(&html, &ctx.base);
        if !courses.is_empty() {
            return Ok(courses);
        }
    }
    Ok(vec![])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::{resp, spawn};

    fn addr(ip: &str) -> SocketAddr {
        format!("{ip}:0").parse().unwrap()
    }

    #[test]
    fn resolver_filter_drops_internal_addresses_for_foreign_hosts() {
        let mixed = vec![
            addr("8.8.8.8"),
            addr("127.0.0.1"),
            addr("10.0.0.5"),
            addr("[::ffff:7f00:1]"),
        ];
        assert_eq!(filter_addrs(mixed.clone(), false), vec![addr("8.8.8.8")]);
        assert!(filter_addrs(vec![addr("192.168.0.1")], false).is_empty());
        // Der Moodle-Host selbst darf auch in einem internen Netz liegen
        assert_eq!(filter_addrs(mixed.clone(), true), mixed);
    }

    #[tokio::test]
    async fn ajax_does_not_follow_redirects() {
        let other = spawn(|_| {
            resp(
                200,
                &[("Content-Type", "application/json")],
                br#"[{"error":false,"data":{}}]"#,
            )
        })
        .await;
        let location = format!("http://{other}/lib/ajax/service.php");
        let moodle = spawn(move |_| resp(302, &[("Location", location.as_str())], b"")).await;
        let ctx = Ctx::new(Url::parse(&format!("http://{moodle}/")).unwrap(), "MoodleSession=abc").unwrap();
        let r = ajax(&ctx, "core_session_touch", serde_json::json!({})).await;
        assert!(r.is_err(), "Weiterleitung darf nicht gefolgt werden");
    }
}
