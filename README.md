# Sternenepoche · Harness-Cockpit – Bau für Windows, Linux und macOS

Dieses Repository enthält genau den Quellstand, aus dem das **Sternenepoche Harness-Cockpit** gebaut wird – und den
GitHub-Actions-Lauf, der es für alle drei Systeme baut, testet und startet.

**Spielen?** Den passenden Download gibt es auf der Webseite: <https://sternenepoche.github.io/Harness-Cockpit.html>
bzw. unter [Releases](../../releases). Rust oder andere Werkzeuge braucht man dafür nicht.

| System | Datei | Hinweis |
|---|---|---|
| Windows 10/11 (x64) | `…-windows-x64.zip` | entpacken, `sternenepoche-cockpit.exe` starten |
| Ubuntu, Debian, Mint … | `sternenepoche-cockpit_…_amd64.deb` | öffnen und installieren, danach im Anwendungsmenü |
| andere Linux (x86-64) | `…-linux-x64.tar.gz` | entpacken, `./installieren.sh` (Menüeintrag + Desktop-Verknüpfung) |
| macOS 11+ (Apple Silicon und Intel) | `…-macos.zip` | App nach „Programme“ ziehen; erster Start siehe LIESMICH.txt |

## Was der Lauf prüft (`.github/workflows/cockpit.yml`)

Auf Windows, Linux (Ubuntu 22.04) und macOS (Apple Silicon, Paket für Apple Silicon und Intel):

1. Tests des lokalen Spielkerns (Singleplayer/LAN), des Cockpits und des Harness mit allen Bausteinen
   (inklusive echter DuckDB-Laufzeit und WASM-Beispiel-Plugin).
2. Paketbau (`harness/paket/paket_plattform.py`): Cockpit und vorgebauter Harness, ohne Entwicklerpfade in den
   Programmen, mit Prüfsummen und Lizenzhinweisen.
3. **Startprüfung** des fertigen Pakets (`harness/paket/startpruefung.py`): Fenster öffnen, „Let's go“ mit dem
   beiliegenden Harness (inklusive Selbsttest und Modellprobe gegen einen Ollama-Ersatz), Singleplayer mit 30 Bots
   starten und laufen lassen, speichern, regulär beenden. Unter Linux zusätzlich nach Installation des .deb-Pakets.

Ein Tag `cockpit-v<version>-<JJJJMMTT>` veröffentlicht die geprüften Pakete als Release.

## Herkunft

Der Inhalt wird aus dem Sternenepoche-Projekt exportiert (`harness/paket/quelle_exportieren.py`): nur die Dateien,
die der Bau liest – die Spiel-Crates `kern`, `lauf`, `server`, das Cockpit (`harness/cockpit`), der Harness
(`harness/runtime`), das Beispiel-Plugin sowie die eingebetteten Bilder, 3D-Figuren, Stimmen und die Spielseite.
Änderungen bitte im Hauptprojekt; dieses Repository wird bei jedem Export überschrieben.
