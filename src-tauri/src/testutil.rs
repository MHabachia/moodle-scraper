//! Ein kleiner HTTP-Server und ein nachgebautes Moodle für die Tests.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

pub struct Request {
    pub method: String,
    pub path: String,
    /// Die kompletten Request-Header, z. B. um das Cookie zu prüfen.
    pub head: String,
    /// Wievielte Anfrage an diesen Pfad das ist (ab 0).
    pub nth: usize,
}

/// Baut eine rohe HTTP-Antwort.
pub fn resp(status: u16, headers: &[(&str, &str)], body: &[u8]) -> Vec<u8> {
    let mut out = format!(
        "HTTP/1.1 {status} X\r\nConnection: close\r\nContent-Length: {}\r\n",
        body.len()
    );
    for (k, v) in headers {
        out.push_str(&format!("{k}: {v}\r\n"));
    }
    out.push_str("\r\n");
    let mut bytes = out.into_bytes();
    bytes.extend_from_slice(body);
    bytes
}

/// Eine Antwort, die weniger Bytes liefert als `Content-Length` verspricht (abgebrochener Transfer).
pub fn truncated(claimed: usize, body: &[u8]) -> Vec<u8> {
    let head = format!("HTTP/1.1 200 OK\r\nConnection: close\r\nContent-Length: {claimed}\r\n\r\n");
    let mut bytes = head.into_bytes();
    bytes.extend_from_slice(body);
    bytes
}

pub async fn spawn<F>(handler: F) -> SocketAddr
where
    F: Fn(&Request) -> Vec<u8> + Send + Sync + 'static,
{
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let handler = Arc::new(handler);
    let hits: Arc<Mutex<HashMap<String, usize>>> = Arc::new(Mutex::new(HashMap::new()));
    tokio::spawn(async move {
        loop {
            let Ok((mut sock, _)) = listener.accept().await else {
                break;
            };
            let handler = handler.clone();
            let hits = hits.clone();
            tokio::spawn(async move {
                let mut buf = Vec::new();
                let mut chunk = [0u8; 1024];
                while !buf.windows(4).any(|w| w == b"\r\n\r\n") {
                    match sock.read(&mut chunk).await {
                        Ok(0) | Err(_) => return,
                        Ok(n) => buf.extend_from_slice(&chunk[..n]),
                    }
                }
                let head = String::from_utf8_lossy(&buf).to_string();
                let method = head.split_whitespace().next().unwrap_or("GET").to_string();
                let path = head.split_whitespace().nth(1).unwrap_or("/").to_string();
                let nth = {
                    let mut h = hits.lock().unwrap();
                    let e = h.entry(path.clone()).or_insert(0);
                    let n = *e;
                    *e += 1;
                    n
                };
                let is_head = method == "HEAD";
                let mut out = handler(&Request {
                    method,
                    path,
                    head,
                    nth,
                });
                if is_head {
                    // HEAD: Header senden, Body weglassen
                    if let Some(pos) = out.windows(4).position(|w| w == b"\r\n\r\n") {
                        out.truncate(pos + 4);
                    }
                }
                let _ = sock.write_all(&out).await;
                let _ = sock.shutdown().await;
            });
        }
    });
    addr
}

use std::sync::atomic::{AtomicBool, Ordering};

const COURSE_HTML: &str = include_str!("../tests/fixtures/moodle3_topics_boost.html");
const FOLDER_HTML: &str = include_str!("../tests/fixtures/folder_page.html");

pub struct FakeMoodle {
    pub addr: SocketAddr,
    /// ETag des Syllabus-PDFs; wer ihn ändert, simuliert eine aktualisierte Datei.
    pub syllabus_etag: Arc<Mutex<String>>,
    /// Alle Anfragen (Methode, Pfad).
    pub requests: Arc<Mutex<Vec<(String, String)>>>,
    /// Wenn gesetzt, liefern Dateien die Login-Seite (Session abgelaufen).
    pub expired: Arc<AtomicBool>,
}

fn host_of(head: &str) -> String {
    head.lines()
        .find_map(|l| l.to_lowercase().strip_prefix("host:").map(|h| h.trim().to_string()))
        .unwrap_or_default()
}

/// Startet ein kleines Moodle mit Kurs 5 (HTML wie Moodle 3/Boost), Ordner, Aufgabe,
/// URL-Aktivität und Dateien.
pub async fn fake_moodle() -> FakeMoodle {
    let syllabus_etag = Arc::new(Mutex::new("\"s1\"".to_string()));
    let requests = Arc::new(Mutex::new(Vec::new()));
    let expired = Arc::new(AtomicBool::new(false));
    let (et, rq, ex) = (syllabus_etag.clone(), requests.clone(), expired.clone());

    let addr = spawn(move |r| {
        rq.lock().unwrap().push((r.method.clone(), r.path.clone()));
        let host = host_of(&r.head);
        let me = format!("http://{host}");
        let fix = |html: &str| html.replace("https://moodle.example.edu", &me);
        let html = |body: String| resp(200, &[("Content-Type", "text/html; charset=utf-8")], body.as_bytes());
        let pdf = |etag: &str, body: &[u8]| resp(200, &[("Content-Type", "application/pdf"), ("ETag", etag)], body);
        let login_page = || html("<html><form action=\"/login/index.php\"></form></html>".into());
        let path = r.path.split('?').next().unwrap_or("");

        match path {
            "/my/" | "/course/view.php" => html(fix(COURSE_HTML)),
            "/lib/ajax/service.php" => resp(
                200,
                &[("Content-Type", "application/json")],
                br#"[{"error":true,"exception":{"errorcode":"unknown","message":"nope"}}]"#,
            ),
            "/mod/resource/view.php" => resp(
                303,
                &[("Location", &format!("{me}/pluginfile.php/55/mod_resource/content/1/Syllabus.pdf"))],
                b"",
            ),
            "/pluginfile.php/55/mod_resource/content/1/Syllabus.pdf" => {
                if ex.load(Ordering::SeqCst) {
                    login_page()
                } else {
                    pdf(&et.lock().unwrap(), b"SYLLABUS-PDF")
                }
            }
            "/mod/folder/view.php" => html(fix(FOLDER_HTML)),
            "/pluginfile.php/88/mod_folder/content/0/Folien 1.pdf" | "/pluginfile.php/88/mod_folder/content/0/Folien%201.pdf" => {
                pdf("\"f1\"", b"FOLIEN")
            }
            "/pluginfile.php/88/mod_folder/content/0/Zusatz/Beispiele.zip" => {
                resp(200, &[("Content-Type", "application/zip")], b"PK-ZIPDATA")
            }
            "/mod/assign/view.php" => html(format!(
                "<html><a href=\"/login/logout.php\">x</a><a href=\"{me}/pluginfile.php/99/mod_assign/introattachment/0/Blatt1.pdf\">Blatt</a></html>"
            )),
            "/pluginfile.php/99/mod_assign/introattachment/0/Blatt1.pdf" => pdf("\"b1\"", b"BLATT1"),
            "/mod/url/view.php" => resp(303, &[("Location", "https://youtu.be/xyz")], b""),
            "/pluginfile.php/55/course/section/301/Zusatz.pdf" => pdf("\"z1\"", b"ZUSATZ"),
            _ => resp(404, &[], b"nicht gefunden"),
        }
    })
    .await;

    FakeMoodle {
        addr,
        syllabus_etag,
        requests,
        expired,
    }
}
