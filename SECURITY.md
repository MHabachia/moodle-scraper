# Sicherheit

## Lücken melden

Bitte keine Sicherheitslücken als öffentliches Issue melden. Nutze stattdessen im GitHub-Repository den Reiter „Security“ und dort „Report a vulnerability“. Beschreibe kurz, was passiert und wie man es nachstellen kann. Unterstützt wird jeweils die neueste Version.

## Was die App schützt

- Das Passwort gibst du im normalen Moodle-Fenster ein. Die App liest es nie.
- Das Session-Cookie bleibt im Arbeitsspeicher und geht nur an den Moodle-Host. Nur wenn du „Angemeldet bleiben“ einschaltest, wird die Session zusätzlich im Passwortspeicher deines Systems gespeichert (macOS Schlüsselbund, Windows Anmeldeinformationsverwaltung) (höchstens 30 Tage, beim Abmelden gelöscht).
- Verbindungen laufen über https. Ausnahme ist `localhost` für Tests.
- Die App ruft nur Adressen der verbundenen Moodle-Instanz ab. Weiterleitungen auf andere Hosts verlieren das Cookie, Weiterleitungen in interne Netze werden blockiert. Externe Links aus Kursen werden nur als Text in `Links.md` geschrieben.
- Programme, Skripte und Makro-Dokumente (`.exe`, `.sh`, `.jar`, `.docm`, ...) werden nur geladen, wenn du die Kategorie einschaltest. Dann werden sie als „aus dem Internet“ markiert, damit dein System vor dem Öffnen warnt.
- Dateinamen werden bereinigt, damit nichts außerhalb des Zielordners landet. Systemordner und Laufwerkswurzeln sind als Ziel gesperrt.
- Protokoll, Diagnose-Export und Delta-Sync-Index enthalten keine Passwörter oder Cookies; Tokens und Sesskeys werden maskiert. Der Scan-Cache speichert nie Inhalte mit Token-URLs.
- Tray-Betrieb und automatischer Sync sind standardmäßig aus.

## Bekannte Grenzen

- Die Installer sind nicht mit einem Entwicklerzertifikat signiert (macOS nur ad-hoc). Die Prüfsummen liegen jedem Release als `SHA256SUMS.txt` bei.
- Es gibt keinen automatischen Updater. Neue Versionen müssen von Hand installiert werden.
