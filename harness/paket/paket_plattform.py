"""Download-Paket des Harness-Cockpits für die laufende Plattform bauen (GitHub Actions: Windows, Linux, macOS).

Ergebnis in harness/paket/aus/:
  Windows: Sternenepoche-Harness-Cockpit-<v>-windows-x64.zip
  Linux:   Sternenepoche-Harness-Cockpit-<v>-linux-x64.tar.gz  und  sternenepoche-cockpit_<v>_amd64.deb
  macOS:   Sternenepoche-Harness-Cockpit-<v>-macos.zip  (App für Apple Silicon und Intel in einer Datei)
  dazu SHA256SUMS-<plattform>.txt und paket-<plattform>.json (Dateien, Größen, Prüfsummen für die Webseite).

Jedes Paket enthält Cockpit, vorgebauten Harness (alle Bausteine) mit sternenepoche-harness.json – der Spieler braucht
weder Rust noch C-Buildwerkzeuge. Entwicklerpfade werden beim Übersetzen ersetzt und danach geprüft.

Aufruf:  python harness/paket/paket_plattform.py [--ohne-bau] [--stempel JJJJMMTT]"""
import hashlib, io, json, os, platform, re, shutil, subprocess, sys, tarfile, time, zipfile
from pathlib import Path

HIER = Path(__file__).resolve().parent
HARNESS = HIER.parent
REPO = HARNESS.parent
COCKPIT, RUNTIME, PLUGIN = HARNESS / "cockpit", HARNESS / "runtime", HARNESS / "beispiele" / "energiewaechter"
AUS = HIER / "aus"
NAME = "Sternenepoche-Harness-Cockpit"
APP = "Sternenepoche Cockpit.app"
ALLE_BAUSTEINE = ["ollama", "openrouter", "openai", "sqlite", "duckdb", "wasm", "lokale-api"]
SYSTEM = {"Windows": "windows", "Linux": "linux", "Darwin": "macos"}[platform.system()]
EXE = ".exe" if SYSTEM == "windows" else ""
MAC_ZIELE = ["aarch64-apple-darwin", "x86_64-apple-darwin"]
HOME = Path.home()
CARGO_HOME = Path(os.environ.get("CARGO_HOME") or HOME / ".cargo")
CARGO = shutil.which("cargo") or str(CARGO_HOME / "bin" / f"cargo{EXE}")
OFFIZIELLE_ADRESSE = "https://sternenepoche.github.io"


def lauf(args, **kw):
    print("  $", " ".join(str(a) for a in args), flush=True)
    r = subprocess.run([str(a) for a in args], **kw)
    if r.returncode != 0: sys.exit(f"FEHLER: {Path(str(args[0])).name} endete mit {r.returncode}")
    return r


def umgebung():
    paare = [(CARGO_HOME / "registry" / "src", "crates"), (CARGO_HOME / "git" / "checkouts", "git"),
             (Path(os.environ.get("RUSTUP_HOME") or HOME / ".rustup"), "rustup"), (REPO, "sternenepoche"), (HOME, "home")]
    flags = [f"--remap-path-prefix={a}={b}" for a, b in paare]
    if SYSTEM == "windows": flags += ["-C", "target-feature=+crt-static"]  # keine Visual-C++-Laufzeit nötig
    umg = {**os.environ, "RUSTFLAGS": " ".join(flags), "CARGO_TERM_COLOR": "never"}
    if SYSTEM == "macos": umg["MACOSX_DEPLOYMENT_TARGET"] = "11.0"
    return umg


def ziele():
    return MAC_ZIELE if SYSTEM == "macos" else [None]


def binaer(projekt: Path, name: str, ziel):
    teile = [projekt / "target" / "paket"] + ([ziel] if ziel else []) + ["release", f"{name}{EXE}"]
    return Path(*teile)


def bauen():
    umg = umgebung()
    print("Beispiel-Plugin (WASM) bauen …", flush=True)
    lauf([CARGO, "build", "--release", "--locked", "--target", "wasm32-unknown-unknown", "--manifest-path", PLUGIN / "Cargo.toml"], env=umg)
    for ziel in ziele():
        z = ["--target", ziel] if ziel else []
        print(f"Cockpit bauen {ziel or ''} …", flush=True)
        lauf([CARGO, "build", "--release", "--locked", *z, "--manifest-path", COCKPIT / "Cargo.toml", "--target-dir", COCKPIT / "target" / "paket"], env=umg)
        print(f"Harness (vorgebaut, alle Bausteine) bauen {ziel or ''} …", flush=True)
        lauf([CARGO, "build", "--release", "--locked", *z, "--no-default-features", "--features", ",".join(ALLE_BAUSTEINE),
              "--manifest-path", RUNTIME / "Cargo.toml", "--target-dir", RUNTIME / "target" / "paket"], env=umg)


def zusammenfuegen(ordner: Path):
    """Die fertigen Programme dieser Plattform; auf macOS je eine Datei für Apple Silicon und Intel (lipo)."""
    out = {}
    for projekt, name in ((COCKPIT, "sternenepoche-cockpit"), (RUNTIME, "sternenepoche-harness")):
        quellen = [binaer(projekt, name, z) for z in ziele()]
        for q in quellen:
            if not q.is_file(): sys.exit(f"FEHLT: {q}")
        ziel = ordner / f"{name}{EXE}"
        if SYSTEM == "macos": lauf(["lipo", "-create", "-output", ziel, *quellen])
        else: shutil.copy2(quellen[0], ziel)
        ziel.chmod(0o755)
        out[name] = ziel
    return out


def sha256(p: Path) -> str:
    h = hashlib.sha256()
    with open(p, "rb") as f:
        for b in iter(lambda: f.read(1 << 20), b""): h.update(b)
    return h.hexdigest()


def pfade_pruefen(dateien):
    """Keine Build-Pfade (Projektordner, Benutzerordner) und keine Tailnet-Adresse außer der offiziellen Spieladresse."""
    erlaubt = re.search(r'STANDARD_SERVER: &str = "https://([^"/]+)', (COCKPIT / "src" / "katalog.rs").read_text(encoding="utf-8"))
    erlaubt = erlaubt.group(1).lower() if erlaubt else ""
    verboten = {str(REPO), str(HOME), REPO.as_posix(), HOME.as_posix()}
    fehler = []
    for d in dateien:
        daten = d.read_bytes()
        for m in sorted(verboten):
            n = daten.count(m.encode("utf-8")) + daten.count(m.encode("utf-16-le"))
            if n: fehler.append(f"{d.name}: „{m}“ {n}×")
        for host in sorted({h.decode().lower() for h in re.findall(rb"[A-Za-z0-9-]+\.[A-Za-z0-9-]+\.ts\.net", daten)}):
            if host != erlaubt: fehler.append(f"{d.name}: fremde Tailnet-Adresse {host.split('.')[0]}… (nur {erlaubt} erlaubt)")
    return fehler


def plattform_filter():
    return {"windows": ["x86_64-pc-windows-msvc"], "linux": ["x86_64-unknown-linux-gnu"], "macos": MAC_ZIELE}[SYSTEM]


def lizenzen():
    """THIRD-PARTY-NOTICES: alle Rust-Bibliotheken, die wirklich ins Programm kommen (ohne Build- und Testabhängigkeiten),
    mit ihren Lizenztexten, dazu three.js und die Schriften von egui. Gleiche Texte stehen nur einmal ausgeschrieben."""
    teile, gesehen, titel_da = [], {}, set()
    kopf = ("Sternenepoche Harness-Cockpit – Hinweise zu Drittkomponenten\n"
            "=============================================================\n\n"
            "Das Paket enthält Software Dritter. Ihre Lizenzen und Urheberrechtsvermerke folgen unten.\n\n")

    def eintrag(titel, texte):
        s = f"\n\n---------------------------------------------------------------------------\n{titel}\n"
        for name, t in texte:
            h = hashlib.sha256(t.encode()).hexdigest()
            if h in gesehen: s += f"\n[{name}: gleicher Text wie bei {gesehen[h]}]\n"
            else: gesehen[h] = titel.split(" — ")[0]; s += f"\n[{name}]\n{t}\n"
        return s

    for manifest in (COCKPIT / "Cargo.toml", RUNTIME / "Cargo.toml"):
        feat = ["--no-default-features", "--features", ",".join(ALLE_BAUSTEINE)] if manifest.parent == RUNTIME else []
        for filt in plattform_filter():
            md = json.loads(subprocess.run([CARGO, "metadata", "--format-version", "1", "--locked", "--manifest-path", str(manifest), "--filter-platform", filt, *feat],
                                           capture_output=True, text=True, encoding="utf-8", check=True).stdout)
            pakete = {p["id"]: p for p in md["packages"]}
            knoten = {n["id"]: n for n in md["resolve"]["nodes"]}
            offen, erreicht = [md["resolve"]["root"]], set()
            while offen:
                i = offen.pop()
                for d in knoten[i]["deps"]:
                    if any(k["kind"] is None for k in d["dep_kinds"]) and d["pkg"] not in erreicht:
                        erreicht.add(d["pkg"]); offen.append(d["pkg"])
            for pid in sorted(erreicht, key=lambda x: (pakete[x]["name"], pakete[x]["version"])):
                p = pakete[pid]
                if p["source"] is None: continue  # eigener Code
                titel = f"{p['name']} {p['version']} — {p.get('license') or 'siehe Lizenzdatei'} — {p.get('repository') or ''}"
                if titel in titel_da: continue
                titel_da.add(titel)
                ordner = Path(p["manifest_path"]).parent
                texte = [(f.name, f.read_text(encoding="utf-8", errors="replace").strip()) for f in sorted(ordner.iterdir())
                         if f.is_file() and re.match(r"(?i)^(licen[cs]e|copying|notice|unlicense)", f.name)]
                if p["name"] == "epaint_default_fonts":
                    texte += [(f.name, f.read_text(encoding="utf-8", errors="replace").strip()) for f in sorted((ordner / "fonts").glob("*.txt"))]
                teile.append(eintrag(titel, texte or [("Lizenz", f"SPDX: {p.get('license') or 'unbekannt'} (kein Lizenztext im Paket)")]))
    three = (REPO / "web-client" / "three.min.js").read_text(encoding="utf-8", errors="replace")[:600]
    m = re.search(r"Copyright[^\n*]*", three)
    teile.append(eintrag("three.js — MIT — https://github.com/mrdoob/three.js", [("LICENSE", (m.group(0).strip() if m else "Copyright Three.js Authors") + MIT)]))
    return kopf + "".join(teile)


MIT = """

Permission is hereby granted, free of charge, to any person obtaining a copy of this software and associated
documentation files (the "Software"), to deal in the Software without restriction, including without limitation the
rights to use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies of the Software, and to permit
persons to whom the Software is furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all copies or substantial portions of the
Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE
WARRANTIES OF MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR
COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR
OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE."""

SPIEL = """Das Harness-Cockpit lässt deinen Rat aus vier KI-Agenten (Stratege, Verwalter, Diplomat, Feldherr) Sternenepoche
für dich spielen – über die offizielle Harness-API des Spielservers, nach den Fair-Play-Regeln.

Offline oder lokal spielen
-------------------------
Im Cockpit ist „Hier auch Singleplayer offline spielen“ direkt sichtbar. „Lokale Sitzung“ öffnet die
lokalen Einstellungen. Standard: 30 Bots, einstellbar von 0 bis 400. Nachrichten, Allianzen und Handel
funktionieren mit deutschen Skriptbots ohne Sprachmodell, Internet, Registrierung oder separates Serverprogramm.
Spielstände werden beim Verlassen und während des Spiels automatisch gespeichert.
Für WLAN/LAN: „Lokalen Host-Server aktivieren“, Masterpasswort (12 bis 128 Zeichen) setzen und Sitzung starten.
Mitspieler wählen „Hosts im Netzwerk suchen“ und treten mit Name und Masterpasswort bei.
Bei gesperrter Netzwerksuche ist der Beitritt über die LAN-IP des Hosts und Port 8892 möglich.
"""

EINRICHTEN = """Einrichten
----------
Der Einrichtungsassistent führt durch fünf Schritte: Server prüfen → Spielstil → Modelle → Bauen → LOS.
- Modelle: auf diesem Rechner (Ollama, https://ollama.com) oder in der Cloud (OpenRouter, https://openrouter.ai).
- „Bauen“ übernimmt den beiliegenden, fertig übersetzten Harness (SHA-256 wird geprüft). Rust und
  C-Buildwerkzeuge sind NICHT nötig. Nur wer den Harness-Code selbst ändern will, schaltet in
  Werkstatt → Bau „selbst übersetzen“ ein.
- Mit LOS meldest du dich einmal mit deinem Spielkonto an; der Rat bekommt seinen Harness-Schlüssel.
Singleplayer und lokale Sitzungen brauchen weder Konto noch Internet.
"""

ORTE_UNIX = """Wo liegt was?
-------------
- Einstellungen: ~/.config/Sternenepoche/cockpit.json
- Harness, Spielstände, Gedächtnis, Seelen (soul.md), Protokolle: ~/.local/share/Sternenepoche/Harness
  (bei Problemen: cockpit-fehler.log und bau.log in diesem Ordner)
- Schlüssel (Spielkonto, Harness, OpenRouter) liegen als Dateien mit Rechten 0600 in diesem Ordner.
"""

LIESMICH = {
    "linux": """Sternenepoche · Harness-Cockpit {version} (Linux x86-64)
==========================================================

{spiel}
Starten
-------
Einfachster Weg (Ubuntu, Debian, Linux Mint, Pop!_OS …): das .deb-Paket von der Webseite öffnen und installieren,
oder im Terminal:  sudo apt install ./sternenepoche-cockpit_*_amd64.deb
Danach steht „Sternenepoche Harness-Cockpit“ im Anwendungsmenü.

Mit diesem Archiv (alle Distributionen):
1. Den Ordner an einen festen Platz entpacken (z. B. ~/Sternenepoche-Harness-Cockpit).
2. Einmal ./installieren.sh ausführen: legt Menüeintrag und Desktop-Verknüpfung an.
   Oder direkt starten: ./sternenepoche-cockpit

{einrichten}
Voraussetzungen
---------------
- 64-Bit-Linux mit grafischer Oberfläche (X11 oder Wayland mit XWayland), glibc 2.35 oder neuer
  (z. B. Ubuntu 22.04+, Debian 12+, Fedora 37+).
- WebKitGTK 4.1 und GTK 3. Fehlen sie, meldet das Cockpit das beim Start. Installieren z. B. mit
    Ubuntu/Debian:  sudo apt install libwebkit2gtk-4.1-0 libgtk-3-0
    Fedora:         sudo dnf install webkit2gtk4.1 gtk3
    Arch:           sudo pacman -S webkit2gtk-4.1 gtk3
- Für lokale Modelle: Ollama und eine Grafikkarte mit mindestens 8 GB Speicher (empfohlen).

{orte}
Prüfsummen und Lizenzen
-----------------------
sternenepoche-harness.json enthält die SHA-256 des vorgebauten Harness; das Cockpit prüft sie vor dem Einrichten.
Hinweise zu Drittkomponenten: THIRD-PARTY-NOTICES.txt. Bilder und 3D-Figuren sind KI-generiert und im Cockpit
gekennzeichnet (Credits).
""",
    "macos": """Sternenepoche · Harness-Cockpit {version} (macOS, Apple Silicon und Intel)
=============================================================================

{spiel}
Starten
-------
1. „Sternenepoche Cockpit“ in den Ordner „Programme“ ziehen.
2. Beim ersten Start: Das Cockpit ist (noch) nicht bei Apple notarisiert. macOS fragt deshalb nach.
   - macOS 15 und neuer: Doppelklick → „Fertig“; dann Systemeinstellungen → Datenschutz & Sicherheit →
     ganz unten bei „Sternenepoche Cockpit“ auf „Dennoch öffnen“ klicken und bestätigen.
   - macOS 11 bis 14: mit gedrückter ctrl-Taste auf die App klicken → „Öffnen“ → „Öffnen“.
   Das ist nur einmal nötig. Danach startet die App normal.
3. Für lokale Sitzungen im WLAN fragt macOS nach dem Zugriff auf das lokale Netzwerk – bitte erlauben.

{einrichten}
Voraussetzungen
---------------
- macOS 11 (Big Sur) oder neuer, Apple Silicon (M1 und neuer) oder Intel.
- Für lokale Modelle: Ollama (https://ollama.com); Apple Silicon mit mindestens 16 GB Speicher empfohlen.

{orte}
Prüfsummen und Lizenzen
-----------------------
sternenepoche-harness.json (in der App unter Contents/MacOS) enthält die SHA-256 des vorgebauten Harness; das
Cockpit prüft sie vor dem Einrichten. Hinweise zu Drittkomponenten: THIRD-PARTY-NOTICES.txt. Bilder und 3D-Figuren
sind KI-generiert und im Cockpit gekennzeichnet (Credits).
""",
    "windows": """Sternenepoche · Harness-Cockpit {version} (Windows x64)
==========================================================

{spiel}
Starten
-------
1. Den Ordner an einen festen Platz entpacken (z. B. Dokumente\\Sternenepoche-Harness-Cockpit).
2. sternenepoche-cockpit.exe starten.

{einrichten}
Voraussetzungen
---------------
- Windows 10 oder 11, 64 Bit.
- Microsoft Edge WebView2 Runtime (bei Windows 11 vorinstalliert; sonst
  https://developer.microsoft.com/microsoft-edge/webview2/).
- Für lokale Modelle: Ollama und eine Grafikkarte mit mindestens 8 GB Speicher (empfohlen).

Wo liegt was?
-------------
- Einstellungen: %APPDATA%\\Sternenepoche\\cockpit.json
- Harness, Spielstände, Gedächtnis, Seelen (soul.md), Protokolle: %LOCALAPPDATA%\\Sternenepoche\\Harness
  (bei Problemen: cockpit-fehler.log und bau.log in diesem Ordner)
- Schlüssel liegen in der Windows-Anmeldeinformationsverwaltung unter „Sternenepoche/…“.

Prüfsummen und Lizenzen
-----------------------
sternenepoche-harness.json enthält die SHA-256 des vorgebauten Harness; das Cockpit prüft sie vor dem Einrichten.
Hinweise zu Drittkomponenten: THIRD-PARTY-NOTICES.txt. Bilder und 3D-Figuren sind KI-generiert und im Cockpit
gekennzeichnet (Credits).
""",
}

DESKTOP = """[Desktop Entry]
Type=Application
Name=Sternenepoche Harness-Cockpit
GenericName=Strategiespiel mit KI-Rat
Comment=Sternenepoche offline, im LAN oder mit deinem KI-Rat spielen
Exec={exe}
Icon={icon}
Terminal=false
Categories=Game;StrategyGame;
StartupWMClass=sternenepoche-cockpit
"""

INSTALLIEREN = """#!/bin/sh
# Menüeintrag und Desktop-Verknüpfung für das entpackte Harness-Cockpit anlegen (nur für diesen Benutzer).
set -e
ORDNER=$(cd "$(dirname "$0")" && pwd)
chmod +x "$ORDNER/sternenepoche-cockpit" "$ORDNER/sternenepoche-harness"
MENUE="${XDG_DATA_HOME:-$HOME/.local/share}/applications"
mkdir -p "$MENUE"
sed -e "s|@EXE@|$ORDNER/sternenepoche-cockpit|" -e "s|@ICON@|$ORDNER/sternenepoche-cockpit.png|" \\
    "$ORDNER/sternenepoche-cockpit.desktop.vorlage" > "$MENUE/sternenepoche-cockpit.desktop"
chmod +x "$MENUE/sternenepoche-cockpit.desktop"
DESKTOP=$(xdg-user-dir DESKTOP 2>/dev/null || echo "$HOME/Desktop")
AUCH=""
if [ -d "$DESKTOP" ]; then
  cp "$MENUE/sternenepoche-cockpit.desktop" "$DESKTOP/"
  chmod +x "$DESKTOP/sternenepoche-cockpit.desktop"
  gio set "$DESKTOP/sternenepoche-cockpit.desktop" metadata::trusted true 2>/dev/null || true
  AUCH=" und auf dem Schreibtisch"
fi
command -v update-desktop-database >/dev/null && update-desktop-database "$MENUE" 2>/dev/null || true
echo "Fertig: „Sternenepoche Harness-Cockpit“ steht im Anwendungsmenü$AUCH."
"""

PLIST = """<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleName</key><string>Sternenepoche Cockpit</string>
  <key>CFBundleDisplayName</key><string>Sternenepoche Harness-Cockpit</string>
  <key>CFBundleIdentifier</key><string>io.github.sternenepoche.cockpit</string>
  <key>CFBundleExecutable</key><string>sternenepoche-cockpit</string>
  <key>CFBundleIconFile</key><string>sternenepoche</string>
  <key>CFBundlePackageType</key><string>APPL</string>
  <key>CFBundleShortVersionString</key><string>{version}</string>
  <key>CFBundleVersion</key><string>{stempel}</string>
  <key>LSMinimumSystemVersion</key><string>11.0</string>
  <key>LSApplicationCategoryType</key><string>public.app-category.strategy-games</string>
  <key>NSHighResolutionCapable</key><true/>
  <key>NSLocalNetworkUsageDescription</key><string>Für lokale Sitzungen im WLAN oder LAN: Hosts finden und Mitspieler beitreten lassen.</string>
  <key>NSAppTransportSecurity</key><dict><key>NSAllowsLocalNetworking</key><true/></dict>
</dict>
</plist>
"""

CONTROL = """Package: sternenepoche-cockpit
Version: {version}
Architecture: amd64
Maintainer: Sternenepoche <{adresse}>
Installed-Size: {kb}
Depends: libwebkit2gtk-4.1-0, libgtk-3-0, libgl1, libxkbcommon-x11-0
Recommends: xdg-utils
Section: games
Priority: optional
Homepage: {adresse}
Description: Sternenepoche Harness-Cockpit
 Sternenepoche offline gegen deutsche Skriptbots, im LAN mit Freunden oder online mit
 deinem KI-Rat (Ollama oder OpenRouter) spielen. Enthält den vorgebauten Harness.
"""


def text(s: str, windows: bool) -> bytes:
    return (s.replace("\r\n", "\n").replace("\n", "\r\n") if windows else s.replace("\r\n", "\n")).encode("utf-8")


def tar_hinzu(t: tarfile.TarFile, name: str, daten: bytes, ausfuehrbar=False):
    i = tarfile.TarInfo(name)
    i.size, i.mode, i.mtime, i.uname, i.gname = len(daten), 0o755 if ausfuehrbar else 0o644, int(time.time()), "root", "root"
    t.addfile(i, io.BytesIO(daten))


def icns(png: Path, ziel: Path):
    """App-Symbol aus dem Sternenepoche-Zeichen (sips und iconutil gehören zu macOS)."""
    satz = ziel.with_suffix(".iconset"); satz.mkdir(parents=True, exist_ok=True)
    for g in (16, 32, 128, 256):
        lauf(["sips", "-z", g, g, png, "--out", satz / f"icon_{g}x{g}.png"], stdout=subprocess.DEVNULL)
        lauf(["sips", "-z", g * 2, g * 2, png, "--out", satz / f"icon_{g}x{g}@2x.png"], stdout=subprocess.DEVNULL)
    lauf(["iconutil", "-c", "icns", satz, "-o", ziel])
    shutil.rmtree(satz)


def main():
    stempel = sys.argv[sys.argv.index("--stempel") + 1] if "--stempel" in sys.argv else time.strftime("%Y%m%d")
    if not re.fullmatch(r"\d{8}", stempel): sys.exit(f"Stempel JJJJMMTT erwartet: {stempel}")
    if "--ohne-bau" not in sys.argv: bauen()
    version = re.search(r'^version\s*=\s*"([^"]+)"', (COCKPIT / "Cargo.toml").read_text(encoding="utf-8"), re.M).group(1)
    voll = f"{version}+{stempel}"
    marke = REPO / "docs" / "branding" / "sternenepoche-mark.png"
    AUS.mkdir(parents=True, exist_ok=True)
    arbeit = AUS / f"bau-{SYSTEM}"
    if arbeit.exists(): shutil.rmtree(arbeit)
    arbeit.mkdir()
    exe = zusammenfuegen(arbeit)
    kennung = subprocess.run([str(exe["sternenepoche-cockpit"]), "--quelle-kennung"], capture_output=True, text=True, timeout=60).stdout.strip()
    if not re.fullmatch(r"[0-9a-f]{16}", kennung): sys.exit(f"Quellkennung unlesbar: {kennung!r}")
    fehler = pfade_pruefen(exe.values())
    if fehler: sys.exit("Entwicklerpfade in den Programmen:\n  " + "\n  ".join(fehler))
    print("Pfadprüfung: keine Entwicklerpfade in den Programmen", flush=True)

    manifest = json.dumps({"quelle": kennung, "features": ALLE_BAUSTEINE, "sha256": sha256(exe["sternenepoche-harness"]), "version": voll}, indent=1).encode()
    win = SYSTEM == "windows"
    liesmich = text(LIESMICH[SYSTEM].format(version=voll, spiel=SPIEL, einrichten=EINRICHTEN, orte=ORTE_UNIX), win)
    hinweise = text(lizenzen(), win)
    begleit = {"sternenepoche-harness.json": manifest, "LIESMICH.txt": liesmich, "THIRD-PARTY-NOTICES.txt": hinweise}
    for name, daten in begleit.items(): (arbeit / name).write_bytes(daten)
    fehler = pfade_pruefen([arbeit / n for n in begleit])
    if fehler: sys.exit("Entwicklerpfade in den Begleitdateien:\n  " + "\n  ".join(fehler))

    dateien = []
    basis = f"{NAME}-{voll.replace('+', '-')}"
    if SYSTEM == "windows":
        z = AUS / f"{basis}-windows-x64.zip"
        with zipfile.ZipFile(z, "w", zipfile.ZIP_DEFLATED, compresslevel=9) as zf:
            for f in sorted(arbeit.iterdir()): zf.write(f, f"{NAME}/{f.name}")
        dateien.append(z)
    elif SYSTEM == "linux":
        z = AUS / f"{basis}-linux-x64.tar.gz"
        vorlage = DESKTOP.format(exe="@EXE@", icon="@ICON@").encode()
        with tarfile.open(z, "w:gz", compresslevel=9) as t:
            for name, p in sorted(exe.items()): tar_hinzu(t, f"{NAME}/{p.name}", p.read_bytes(), True)
            for name, daten in sorted(begleit.items()): tar_hinzu(t, f"{NAME}/{name}", daten)
            tar_hinzu(t, f"{NAME}/sternenepoche-cockpit.png", marke.read_bytes())
            tar_hinzu(t, f"{NAME}/sternenepoche-cockpit.desktop.vorlage", vorlage)
            tar_hinzu(t, f"{NAME}/installieren.sh", INSTALLIEREN.encode(), True)
        dateien.append(z)
        dateien.append(deb(arbeit, exe, begleit, marke, voll))
    else:
        mappe = arbeit / NAME
        inhalt = mappe / APP / "Contents"
        (inhalt / "MacOS").mkdir(parents=True); (inhalt / "Resources").mkdir()
        for p in exe.values(): shutil.copy2(p, inhalt / "MacOS" / p.name)
        (inhalt / "MacOS" / "sternenepoche-harness.json").write_bytes(manifest)
        (inhalt / "Info.plist").write_text(PLIST.format(version=version, stempel=stempel), encoding="utf-8")
        icns(marke, inhalt / "Resources" / "sternenepoche.icns")
        lauf(["codesign", "--force", "--deep", "--sign", "-", mappe / APP])  # ad hoc: Apple Silicon startet nur Signiertes
        lauf(["codesign", "--verify", "--deep", "--strict", mappe / APP])
        (mappe / "LIESMICH.txt").write_bytes(liesmich); (mappe / "THIRD-PARTY-NOTICES.txt").write_bytes(hinweise)
        z = AUS / f"{basis}-macos.zip"
        if z.exists(): z.unlink()
        lauf(["ditto", "-c", "-k", "--sequesterRsrc", "--keepParent", mappe, z])
        dateien.append(z)

    summen = AUS / f"SHA256SUMS-{SYSTEM}.txt"
    summen.write_text("".join(f"{sha256(p)}  {p.name}\n" for p in dateien), encoding="utf-8")
    bericht = {"plattform": SYSTEM, "version": voll, "quelle": kennung,
               "dateien": [{"datei": p.name, "groesse_mb": round(p.stat().st_size / 1e6, 1), "sha256": sha256(p)} for p in dateien]}
    (AUS / f"paket-{SYSTEM}.json").write_text(json.dumps(bericht, ensure_ascii=False, indent=1), encoding="utf-8")
    print(json.dumps(bericht, ensure_ascii=False, indent=1))


def deb(arbeit: Path, exe: dict, begleit: dict, marke: Path, voll: str) -> Path:
    """Debian-Paket: /opt/sternenepoche-cockpit, Menüeintrag, Symbol und /usr/bin/sternenepoche-cockpit."""
    wurzel = arbeit / "deb"
    opt = wurzel / "opt" / "sternenepoche-cockpit"
    opt.mkdir(parents=True)
    for p in exe.values(): shutil.copy2(p, opt / p.name); (opt / p.name).chmod(0o755)
    for name, daten in begleit.items(): (opt / name).write_bytes(daten)
    menue = wurzel / "usr" / "share" / "applications"; menue.mkdir(parents=True)
    (menue / "sternenepoche-cockpit.desktop").write_text(DESKTOP.format(exe="/opt/sternenepoche-cockpit/sternenepoche-cockpit", icon="sternenepoche-cockpit"), encoding="utf-8")
    symbol = wurzel / "usr" / "share" / "icons" / "hicolor" / "256x256" / "apps"; symbol.mkdir(parents=True)
    shutil.copyfile(marke, symbol / "sternenepoche-cockpit.png")
    (wurzel / "usr" / "bin").mkdir(parents=True)
    os.symlink("/opt/sternenepoche-cockpit/sternenepoche-cockpit", wurzel / "usr" / "bin" / "sternenepoche-cockpit")
    kb = sum(p.stat().st_size for p in wurzel.rglob("*") if p.is_file() and not p.is_symlink()) // 1024
    (wurzel / "DEBIAN").mkdir()
    (wurzel / "DEBIAN" / "control").write_text(CONTROL.format(version=voll, kb=kb, adresse=OFFIZIELLE_ADRESSE), encoding="utf-8")
    for p in wurzel.rglob("*"):
        if p.is_dir(): p.chmod(0o755)
    ziel = AUS / f"sternenepoche-cockpit_{voll}_amd64.deb"
    lauf(["dpkg-deb", "--root-owner-group", "-Zxz", "--build", wurzel, ziel])
    lauf(["dpkg-deb", "--info", ziel])
    return ziel


if __name__ == "__main__":
    main()
