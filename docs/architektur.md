# Architektur

Dieses Dokument beschreibt, wie Moodle Scraper aufgebaut ist und warum einige Dinge so gelöst sind. Einen Überblick mit Tech-Stack und Projektstruktur gibt die [README](../README.md).

## Überblick

Die App ist eine Tauri-2-Anwendung. Tauri startet einen Rust-Prozess und zeigt darin ein Webview-Fenster mit dem Frontend. Das Frontend (React) ruft Funktionen des Backends auf („Befehle“) und bekommt Fortschritt als Event zurück. Das Backend spricht per HTTPS mit Moodle und schreibt die Dateien auf die Platte.

Moodle selbst wird nicht über die offizielle Web-Service-API angesprochen, denn die braucht einen Token, den viele Unis gar nicht ausgeben. Stattdessen nutzt die App die Anmeldung, die auch dein Browser hat, und ruft dieselben Seiten und internen AJAX-Funktionen ab.

## Ablauf einer Sitzung

**1. Anmeldung.** `open_login` öffnet ein zweites Fenster mit der echten Moodle-Anmeldeseite, ohne dauerhaften Browser-Speicher (inkognito). Dort läuft Uni-SSO und Zwei-Faktor wie gewohnt. Das Frontend fragt alle paar Sekunden `check_login`. Das Backend liest dann die Cookies aus dem Login-Fenster, ruft damit die Moodle-Startseite ab und prüft, ob ein Logout-Link da ist. Wenn ja, übernimmt es Cookies und Sesskey in einen eigenen HTTP-Client und schließt das Fenster. Das Passwort sieht die App dabei nie.

**2. Session halten.** Die Cookies liegen in einem Cookie-Jar. Setzt Moodle neue Werte (Session-Rotation), übernimmt der Jar sie. Alle vier Minuten schickt ein Hintergrund-Task `core_session_touch`, damit Moodle die Session nicht wegen Inaktivität beendet. Optional wird die Session im Passwortspeicher des Systems gespeichert (macOS Schlüsselbund, Windows Anmeldeinformationsverwaltung), damit man beim nächsten Start nicht neu anmelden muss.

**3. Kursliste.** `list_courses` fragt die interne AJAX-Funktion `core_course_get_enrolled_courses_by_timeline_classification` ab. Liefert sie nichts, werden die Kurslinks aus dem HTML der Startseite gelesen.

**4. Scan.** `scan_course` holt die Struktur eines Kurses. Bei Moodle 4.x kommt sie per AJAX (`core_courseformat_get_state`), sonst wird das HTML der Kursseite geparst. Danach wird jede Aktivität aufgelöst: Ressourcen führen über Weiterleitungen zur eigentlichen Datei (zuerst per HEAD, damit nichts heruntergeladen wird), Ordner und Aufgaben werden geöffnet und ihre Dateien gelistet, URL-Aktivitäten liefern einen externen Link. Das Ergebnis wird sechs Stunden auf der Platte zwischengespeichert.

**5. Vorschau.** `preview_job` plant die Zielpfade genauso wie der Download, schickt pro Datei eine HEAD-Anfrage und vergleicht ETag, Änderungsdatum und Größe mit dem Index der letzten Runde. So entstehen die Zahlen für neu, geändert und unverändert.

**6. Download.** `download_courses` arbeitet die Kurse nacheinander ab, die Dateien eines Kurses parallel (1 bis 6). Jede Datei wird in eine `.part`-Datei geschrieben und erst nach dem vollständigen Download umbenannt (oder ins ZIP übernommen). Das Backend schickt alle 250 ms ein Fortschritts-Event.

## Backend (`src-tauri/src`)

| Datei / Ordner | Aufgabe |
|---|---|
| `main.rs` | Startet Tauri, registriert die Befehle, steuert das Schließen des Fensters (Tray). |
| `session.rs` | Login-Fenster, Cookie-Jar, Keep-Alive, AJAX-Aufrufe, Kursliste, gespeicherte Session. Hier liegt der gemeinsame HTTP-Kontext `Ctx`. |
| `scan.rs`, `scan_cache.rs` | Kurs scannen und Dateien auflösen; Cache auf der Platte. |
| `parse/` | Reine Parser für Moodle-HTML und -JSON: Kursseite, Ordner, Aufgaben, URL-Aktivitäten, Kursliste. Sie machen kein Netzwerk und sind mit echten HTML-Beispielen getestet. |
| `download/mod.rs` | Ablauf eines Auftrags: Zielordner prüfen, pro Kurs planen, parallel laden, Ergebnis verbuchen. |
| `download/fetch.rs` | Eine Datei laden: Streaming, Wiederholungen (3 Versuche, wachsende Wartezeit, `Retry-After`), Erkennen einer abgelaufenen Session. |
| `download/index.rs` | Der Delta-Sync-Index und die Entscheidung, ob eine Datei übersprungen wird. |
| `download/preview.rs`, `links.rs`, `sink.rs`, `progress.rs` | Vorschau, `Links.md`, Ziel als Ordner oder ZIP, geteilter Fortschritt. |
| `util/` | Pfade und Dateinamen, URLs und Hosts, Maskieren von Tokens, UTC-Zeit, Kalenderexport. |
| `deadlines.rs` | Fristen aus dem Moodle-Kalender, `.ics`-Export. |
| `secure.rs` | Session im Passwortspeicher des Systems speichern und laden. |
| `tray.rs`, `log.rs`, `diagnostics.rs` | Tray-Symbol, Protokolldatei, Diagnose-Export. |
| `types.rs` | Alle Typen, die zwischen Backend und Frontend gehen. |

### Befehle und Events

Das Frontend ruft diese Befehle auf (`src/api.ts`): `open_login`, `check_login`, `restore_session`, `set_remember`, `logout`, `list_courses`, `scan_course`, `clear_scan_cache`, `preview_job`, `download_courses`, `cancel_download`, `default_download_dir`, `list_deadlines`, `export_ics`, `export_diagnostics`, `set_background`. Dazu kommt das Event `download-progress`.

Die Typen der Befehle stehen nur einmal, in `types.rs`. `ts-rs` erzeugt daraus beim Testlauf die Dateien in `src/generated/`. So können Frontend und Backend nicht unbemerkt auseinanderlaufen, und die CI prüft das.

### Der Delta-Sync

Im Zielordner liegt `.moodle-scraper-index.json`. Der Schlüssel eines Eintrags ist Kurs-ID plus relativer Pfad, der Wert besteht aus Größe, ETag, Änderungsdatum und der (bereinigten) URL. Beim Download sendet der Server diese Header ohnehin mit, deshalb kann die App eine unveränderte Datei erkennen, bevor sie den Inhalt liest. Im Ordner-Modus gilt eine Datei nur dann als unverändert, wenn sie lokal noch existiert. Im ZIP-Modus enthält jedes neue Archiv nur das, was seit dem letzten Lauf neu oder geändert ist.

### Fehler und Überlast

- Netzwerkfehler und 5xx-Antworten werden bis zu dreimal wiederholt, mit wachsender Wartezeit.
- Antwortet der Server mit 429 oder 503, halbiert die App die Zahl der gleichzeitigen Downloads (mindestens 1) und beachtet `Retry-After`.
- Liefert Moodle statt einer Datei die Anmeldeseite, ist die Session abgelaufen. Der Lauf stoppt, alle noch offenen Dateien werden als Fehler mit ihren Daten gemeldet, und das Frontend bietet an, sich neu anzumelden und mit genau diesen Dateien weiterzumachen.

### Dateinamen und Pfade

Jeder Name aus Moodle wird als Pfadbestandteil bereinigt: verbotene Zeichen weg, keine reservierten Windows-Namen, keine Punkte am Ende, Umlaute in einheitlicher Form (NFC), höchstens 100 Zeichen. Der ganze Pfad wird auf 240 Zeichen begrenzt, damit Windows ihn annimmt. Gleiche Namen im selben Ordner bekommen ein „(2)“. Die Ordnerstruktur kommt aus einem Muster (`{section}/{folder}` ist der Standard). Als Zielordner sind Systemordner und Laufwerkswurzeln gesperrt.

## Frontend (`src`)

- `App.tsx` hält Session und Einstellungen und stellt die Sprache bereit, `AppView.tsx` steuert den Ablauf (Verbinden, Anmelden, Kurse, Dialoge).
- `hooks/useSettings.ts` lädt und speichert die Einstellungen in `localStorage` und wendet das Design an. Sie bestehen aus einem globalen Teil (Sprache, Design, zuletzt benutzte Adresse) und einem Profil je Moodle-Instanz (Zielordner, Format, Dateitypen, ...). Gespeichert wird mit einer Versionsnummer, ältere Stände werden beim Laden umgewandelt.
- `hooks/useDownload.ts` kapselt den Ablauf Analysieren, Vorschau, Download, Wiederholen und Neuanmelden. Die Kurse werden mit bis zu drei Workern gleichzeitig gescannt.
- `hooks/useAutoSync.ts` löst den regelmäßigen Hintergrund-Sync aus.
- `lib/` enthält reine Funktionen (Filtern und Sortieren der Kurse, Aufbau des Auftrags fürs Backend), die sich leicht testen lassen.
- `i18n/` hat die Texte auf Deutsch (`de.ts`, Referenz) und Englisch (`en.ts`). Der Typ erzwingt, dass beide dieselben Schlüssel haben.
- `components/` enthält die Bildschirme und Dialoge, `ui.tsx` gemeinsame Bausteine wie Modal und Buttons.

## Daten auf dem Rechner

| Was | Wo | Inhalt |
|---|---|---|
| Delta-Sync-Index | Zielordner, `.moodle-scraper-index.json` | Größe, ETag, Änderungsdatum je Datei |
| Temporäre Dateien | Zielordner, `*.part` und `.moodle-scraper-tmp/` | Nur während eines Laufs |
| Scan-Cache | Cache-Ordner der App | Kursstrukturen, sechs Stunden gültig, nie mit Token-URLs |
| Protokoll | Log-Ordner der App, `moodle-scraper.log` | Ereignisse mit maskierten Tokens, Rotation bei 1 MB |
| Einstellungen | `localStorage` des Webviews | Profile, Sprache, Design |
| Gespeicherte Session | Passwortspeicher des Systems | Nur wenn „Angemeldet bleiben“ an ist, höchstens 30 Tage |

## Warum es so gebaut ist

- **Echtes Login-Fenster statt eigenes Formular:** Uni-Anmeldungen laufen fast immer über SSO und Zwei-Faktor. Nur der Browser kann das zuverlässig, und die App muss das Passwort nie anfassen.
- **Kein API-Token:** Der Web-Service ist an vielen Unis abgeschaltet. Die Webseite ist dagegen für jeden Studierenden zugänglich.
- **HEAD vor GET:** Ob eine Datei neu ist, steht schon in den Headern. Das spart Datenmenge beim Scannen und in der Vorschau.
- **`.part`-Datei und Umbenennen:** Bricht ein Download ab, bleibt keine halbe Datei unter dem richtigen Namen liegen.
- **Parser ohne Netzwerk:** Moodle-Seiten sehen je nach Version und Theme verschieden aus. Mit reinen Funktionen und gesammelten HTML-Beispielen lässt sich jeder neue Fall als Test festhalten.
- **Fester Auftrag statt Zustand im Backend:** Das Frontend baut den ganzen Auftrag (Kurse, Dateien, Optionen) und schickt ihn mit. Das Backend merkt sich zwischen zwei Läufen nur die Session.

## Tests

- **Parser** (`parse/tests.rs`): HTML-Beispiele aus `src-tauri/tests/fixtures/` für Moodle 3, Moodle 4, ein Classic-Theme, Ordner und Aufgaben.
- **Download** (`download/tests.rs`): ein kleiner lokaler HTTP-Server prüft Cookie, Wiederholungen, Drosselung, Abbruch, Größenlimit, abgelaufene Session und den Delta-Vergleich.
- **Ende zu Ende** (`download/e2e.rs`): ein nachgebautes Moodle (`testutil.rs`) mit Kurs, Ordner, Aufgabe und Link. Getestet werden Scan, Download als Ordner und als ZIP, mehrere Läufe nacheinander und das Fortsetzen nach abgelaufener Session.
- **Hilfsfunktionen** (`util/tests.rs`): Pfade, Namen, URLs, Maskieren, Kalenderformat.
- **Frontend** (Vitest): reine Logik, Komponenten, die Hooks mit nachgebautem Backend, und die Übersetzungen (gleiche Schlüssel und Platzhalter in beiden Sprachen).

## Build, CI und Release

- `tests.yml` läuft bei jedem Push und Pull Request und besteht aus drei Jobs: TypeScript-Typprüfung, Frontend-Tests (Vitest) und Rust-Tests auf macOS und Windows. Im Rust-Job laufen außerdem Clippy, die Formatprüfung und der Abgleich der erzeugten TypeScript-Typen.
- `build.yml` startet bei einem Tag `v*` oder von Hand. Es ruft zuerst `tests.yml` auf, baut dann mit `tauri-action` die Installer für macOS (universal) und Windows, veröffentlicht das Release mit dem `.dmg` und dem `-setup.exe` und hängt eine `SHA256SUMS.txt` an.
- Die Actions sind auf Commit-SHAs festgelegt.
