# Mitmachen

Danke, dass du helfen möchtest. Fehlermeldungen, Ideen und Pull Requests sind willkommen.

## Fehler melden

Öffne ein [Issue](../../issues/new/choose) und beschreibe:

- Betriebssystem und Version der App (steht in den Einstellungen unter „Diagnose exportieren“)
- Welche Moodle-Version oder welches Design deine Hochschule nutzt, falls bekannt
- Was du gemacht hast, was passiert ist und was du erwartet hast

Hilfreich ist die Diagnose-Datei (Einstellungen, Werkzeuge, „Diagnose exportieren“). Sie enthält keine Passwörter oder Cookies.

Wenn Dateien oder Abschnitte eines Kurses fehlen, hilft ein anonymisiertes HTML der Kursseite am meisten. Entferne Namen und Links, bevor du es hochlädst. Daraus lässt sich ein Test bauen, der den Fall künftig absichert (siehe `src-tauri/tests/fixtures/`).

## Entwicklung

Voraussetzungen: Node 22.12 oder neuer und Rust (stable), siehe auch den Abschnitt „Entwickeln“ in der [README](README.md).

```bash
npm install
npm run tauri dev
```

Einen lokalen Installer baust du mit `npm run tauri build -- --config '{"bundle":{"createUpdaterArtifacts":false}}'`. Ohne die Option verlangt Tauri den privaten Update-Schlüssel, den nur das Release in GitHub Actions kennt.

Vor einem Pull Request sollte alles durchlaufen:

```bash
npm run lint
npm test
npm run build && cargo test --manifest-path src-tauri/Cargo.toml
cargo fmt --manifest-path src-tauri/Cargo.toml --check
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
```

Dieselben Prüfungen laufen auch in GitHub Actions (`tests.yml`).

## Pull Requests

- Eine Änderung pro Pull Request, mit einer kurzen Beschreibung, was und warum.
- Neues Verhalten braucht einen Test. Für neue Moodle-Seitenformate gehört eine HTML-Datei nach `src-tauri/tests/fixtures/` und ein Test in `src-tauri/src/parse/tests.rs`.
- Texte in der Oberfläche stehen in `src/i18n/de.ts` und `src/i18n/en.ts`. Beide Dateien müssen dieselben Schlüssel haben.
- Ändert sich ein Typ in `src-tauri/src/types.rs`, erzeugt `cargo test` die TypeScript-Typen in `src/generated/` neu. Diese Dateien gehören mit in den Pull Request.
- Commit-Nachrichten kurz und im Präsens, zum Beispiel „Vorschau zeigt Größe pro Kurs“.

## Aufbau des Projekts

Eine Beschreibung der Architektur steht in [docs/architektur.md](docs/architektur.md).

## Sicherheitslücken

Bitte nicht als öffentliches Issue melden, sondern wie in [SECURITY.md](SECURITY.md) beschrieben.
