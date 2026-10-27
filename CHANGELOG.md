# Changelog

## 1.3.1

- Der Windows-Installer und die Deinstallation zeigen jetzt das App-Icon statt des Standard-Icons des Installers.
- Externe Links sind auf die Projektseite beschränkt; die App kann keine beliebigen Adressen mehr öffnen.
- Installationshinweis für macOS ohne Terminal-Befehl (Freigabe unter „Datenschutz & Sicherheit“).
- Kleine Abhängigkeits-Updates und zusätzliche Tests.

## 1.3.0

- Automatische Updates: Die App sucht beim Start nach einer neuen Version (in den Einstellungen abschaltbar), zeigt einen Hinweis und installiert nach Bestätigung. Im Info-Dialog lässt sich manuell suchen. Während eines Downloads ist das Installieren gesperrt.
- Die Update-Pakete sind signiert. Die Release-Dateien umfassen deshalb zusätzlich `latest.json` und ein macOS-Update-Paket.
- Wer noch 1.2.0 oder älter nutzt, installiert 1.3.0 einmal von Hand.

## 1.2.0

- Neue Info-Seite („Über die App“) mit Zweck, Technik, Version und Link zum Quellcode, erreichbar über das Info-Symbol in der oberen Leiste.

## 1.1.1

- Fristen: Die Abfrage an Moodle fragte zu viele Einträge auf einmal an und scheiterte mit „Limit must be between 1 and 50“. Sie ist jetzt auf 50 begrenzt.

## 1.1.0

Erste öffentliche Version.

- Anmeldung im normalen Moodle-Fenster (auch Uni-SSO und Zwei-Faktor), optional angemeldet bleiben über den Passwortspeicher des Systems (macOS: Schlüsselbund, Windows: Anmeldeinformationsverwaltung).
- Kursliste mit Suche, Filtern und Mehrfachauswahl.
- Download von Dateien, Ordnern, Aufgaben-Anhängen und Links als Ordner oder ZIP je Kurs.
- Vorschau vor dem Download, Delta-Sync, parallele Downloads mit Wiederholungen.
- Neu anmelden und weitermachen, wenn die Sitzung während des Downloads abläuft.
- Fristen aus dem Moodle-Kalender, Export als `.ics`.
- Tray-Betrieb und automatischer Sync (optional).
- Deutsch und Englisch, helles und dunkles Design.
