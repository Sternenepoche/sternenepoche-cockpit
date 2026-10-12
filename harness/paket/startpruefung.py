"""Startprüfung eines fertigen Cockpit-Pakets auf der Zielplattform (GitHub Actions und lokal).

Startet das ausgepackte Cockpit mit eigener Einstellungsdatei und eigenem Arbeitsordner und steuert es wie ein
Spieler über die Entwicklungsansicht (--entwicklung PORT: nur 127.0.0.1, Zufallsschlüssel in der Adresse):
  1. Das Fenster läuft und schickt seinen Zustand.
  2. „Let's go“ richtet den beiliegenden, vorgebauten Harness ein (SHA-256, Konfiguration, Selbsttest). Ein kleiner
     Ollama-Ersatz auf 127.0.0.1 meldet die gewählten Modelle als vorhanden; der Spielserver ist absichtlich nicht
     erreichbar – das darf die Installation nicht scheitern lassen.
  3. Singleplayer mit 30 Bots: Die Welt startet, die Spielzeit läuft, die Spielseite wird ausgeliefert.
  4. Verlassen speichert; Beenden schließt das Cockpit regulär (Marker weg, kein Programmfehler protokolliert).

Aufruf:  python startpruefung.py PFAD/ZUR/sternenepoche-cockpit[.exe] [--zeit SEKUNDEN]
Linux braucht ein X-Display (in CI: xvfb-run). Nur Standardbibliothek."""
import json, os, shutil, socket, subprocess, sys, tempfile, threading, time
import urllib.error, urllib.request
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path

EXE = Path(sys.argv[1]).resolve()
ZEIT = int(sys.argv[sys.argv.index("--zeit") + 1]) if "--zeit" in sys.argv else 900
BOTS = 30
START = time.monotonic()


def frei() -> int:
    with socket.socket() as s:
        s.bind(("127.0.0.1", 0))
        return s.getsockname()[1]


def schritt(text: str):
    print(f"[{time.monotonic() - START:6.1f} s] {text}", flush=True)


class Ollama(BaseHTTPRequestHandler):
    """Antwortet wie ein Ollama mit allen Modellen, die das Cockpit gerade braucht."""
    modelle: list = []

    def antwort(self, code, daten):
        b = json.dumps(daten).encode()
        self.send_response(code); self.send_header("Content-Type", "application/json"); self.send_header("Content-Length", str(len(b)))
        self.end_headers(); self.wfile.write(b)

    def do_GET(self):
        if self.path.startswith("/api/tags"):
            self.antwort(200, {"models": [{"name": m, "model": m, "size": 1_000_000} for m in Ollama.modelle]})
        elif self.path.startswith("/api/ps"): self.antwort(200, {"models": []})
        elif self.path.startswith("/api/version"): self.antwort(200, {"version": "0.0.0-pruefung"})
        else: self.antwort(404, {"error": "nicht vorhanden"})

    def do_POST(self):
        """Modellprobe des Harness-Selbsttests: „Bestätige mit ok=true und rolle="…"“ – Antwort im verlangten JSON."""
        daten = json.loads(self.rfile.read(int(self.headers.get("Content-Length") or 0)) or b"{}")
        if not self.path.startswith("/api/chat"): return self.antwort(404, {"error": "Die Startprüfung lädt keine Modelle"})
        frage = " ".join(m.get("content", "") for m in daten.get("messages", []))
        rolle = frage.split('rolle="', 1)[1].split('"', 1)[0] if 'rolle="' in frage else ""
        self.antwort(200, {"model": daten.get("model"), "done": True, "prompt_eval_count": 20, "eval_count": 8,
                           "message": {"role": "assistant", "content": json.dumps({"ok": True, "rolle": rolle})}})

    def log_message(self, *_): pass


class Cockpit:
    def __init__(self, ordner: Path):
        self.ordner, self.port, self.ollama = ordner, frei(), ThreadingHTTPServer(("127.0.0.1", 0), Ollama)
        threading.Thread(target=self.ollama.serve_forever, daemon=True).start()
        self.arbeit = ordner / "arbeit"
        self.ausgabe = []
        args = [str(EXE), "--entwicklung", str(self.port), "--einstellungen", str(ordner / "cockpit.json"), "--arbeitsordner", str(self.arbeit),
                "--server", "http://127.0.0.1:9", "--ollama", f"http://127.0.0.1:{self.ollama.server_address[1]}", "--titel", "Startprüfung"]
        schritt("Start: " + " ".join(Path(a).name if i == 0 else a for i, a in enumerate(args)))
        self.proc = subprocess.Popen(args, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True, encoding="utf-8", errors="replace")
        threading.Thread(target=self.mitlesen, daemon=True).start()
        self.adresse = self.warten(self.adresse_lesen, 120, "Entwicklungsansicht meldet ihre Adresse")
        self.schluessel = self.adresse.split("t=")[1]

    def mitlesen(self):
        for zeile in self.proc.stdout: self.ausgabe.append(zeile.rstrip())

    def adresse_lesen(self):
        for z in self.ausgabe:
            if z.startswith("ENTWICKLUNG "): return z.split(" ", 1)[1]
        datei = self.arbeit / "entwicklung.txt"
        return datei.read_text(encoding="utf-8").strip() if datei.is_file() else None

    def lebt(self):
        if self.proc.poll() is not None: raise SystemExit(f"FEHLER: Cockpit beendet (Code {self.proc.returncode})")

    def warten(self, pruefe, sekunden, was):
        ende = time.monotonic() + sekunden
        while time.monotonic() < ende:
            self.lebt()
            try:
                r = pruefe()
            except (urllib.error.URLError, ConnectionError, TimeoutError, json.JSONDecodeError):
                r = None
            if r: return r
            time.sleep(0.5)
        raise SystemExit(f"FEHLER: Zeitüberschreitung nach {sekunden} s: {was}")

    def api(self, pfad, daten=None, timeout=10):
        url = f"http://127.0.0.1:{self.port}{pfad}{'&' if '?' in pfad else '?'}t={self.schluessel}"
        anfrage = urllib.request.Request(url, data=None if daten is None else json.dumps(daten).encode(), method="GET" if daten is None else "POST",
                                         headers={"Content-Type": "application/json"})
        with urllib.request.urlopen(anfrage, timeout=timeout) as r:
            return json.loads(r.read())

    def stand(self) -> dict:
        return {art: d for art, d in self.api("/api/zustand?seit=0")["nachrichten"]}

    def befehl(self, t, **felder):
        self.api("/api/befehl", {"t": t, **felder})


def pruefen(ordner: Path):
    c = Cockpit(ordner)
    try:
        s = c.warten(lambda: (lambda st: st if {"zustand", "auswahl", "katalog"} <= st.keys() else None)(c.stand()), 120, "Oberfläche schickt Zustand")
        schritt(f"Fenster läuft · Cockpit {s['katalog'].get('version')} · Zustandsteile: {', '.join(sorted(s))}")

        # 2 Let's go mit dem vorgebauten Harness
        Ollama.modelle = sorted({w["modell"] for w in s["auswahl"]["rollen"].values() if w.get("anbieter") == "ollama"}) or ["qwen3:8b"]
        c.befehl("setze", pfad="/lokale_bots", wert=BOTS)
        c.befehl("lets_go")
        bau = c.warten(lambda: (lambda b: b if b.get("ende") else None)(c.stand()["zustand"]["bau"]), 300, "Let's go beendet")
        for st in bau.get("schritte", []): schritt(f"  Bau · {st.get('name')}: {st.get('stand')} – {st.get('text')}")
        for p in bau.get("pruefungen") or []: schritt(f"  Prüfung · {p.get('pruefung')}: {'ok' if p.get('ok') else 'FEHLER'} – {p.get('text')}")
        if bau["ende"].get("ok") is not True:
            raise SystemExit(f"FEHLER: Let's go fehlgeschlagen: {bau['ende']}\n" + "\n".join(map(str, (bau.get("log") or [])[-30:])))
        harness = c.arbeit / "bin" / ("sternenepoche-harness.exe" if os.name == "nt" else "sternenepoche-harness")
        if not harness.is_file(): raise SystemExit(f"FEHLER: Harness fehlt nach Let's go: {harness}")
        schritt(f"Let's go: vorgebauter Harness eingerichtet und geprüft ({harness.stat().st_size / 1e6:.1f} MB)")

        # 3 Singleplayer
        c.befehl("lokal_start", name="Pruefpilot", volk="veyari", password="")
        lokal = c.warten(lambda: (lambda l: l if l.get("connected") and (l.get("info") or {}).get("world_id") else None)(c.stand()["zustand"]["lokal"]), 300,
                         "Singleplayer verbunden")
        url = lokal["url"]
        info = lambda: json.loads(urllib.request.urlopen(f"{url}/api/local/info", timeout=10).read())
        info1 = info()
        schritt(f"Singleplayer läuft: {url} · Welt {str(info1.get('world_id'))[:8]} · {info1.get('bots')} Bots · "
                f"{info1.get('sektoren')} Sektoren × {info1.get('systeme')} Systeme · Spielzeit {info1.get('sekunden')} s")
        if info1.get("bots") != BOTS: raise SystemExit(f"FEHLER: {info1.get('bots')} Bots statt {BOTS}")
        seite = urllib.request.urlopen(f"{url}/", timeout=10).read()
        if b"<html" not in seite.lower()[:2000]: raise SystemExit("FEHLER: Spielseite wird nicht ausgeliefert")
        info2 = c.warten(lambda: (lambda i: i if i.get("sekunden", 0) > info1.get("sekunden", 0) else None)(info()), 150, "Spielzeit läuft weiter")
        schritt(f"Spielzeit läuft: {info1.get('sekunden')} s → {info2.get('sekunden')} s · Tick {info2.get('tick_ms')} ms")
        if info2.get("fehler"): raise SystemExit(f"FEHLER: Welt meldet Fehler: {info2['fehler']}")

        # 4 Verlassen und Beenden (Befehle während einer Speicherung lehnt das Cockpit ab: wiederholen)
        def verlassen():
            l = c.stand()["zustand"]["lokal"]
            if not l.get("connected") and not l.get("busy"): return True
            if l.get("connected") and not l.get("busy"): c.befehl("lokal_verlassen")
            time.sleep(2)
        c.warten(verlassen, 120, "Sitzung verlassen und gespeichert")
        stand = c.arbeit / "singleplayer" / f"bots-{BOTS}" / "spiel.sqlite3"
        if not stand.is_file(): raise SystemExit(f"FEHLER: kein Spielstand unter {stand}")
        schritt(f"Verlassen: Spielstand gespeichert ({stand.stat().st_size / 1e6:.1f} MB)")
        meldung = c.stand()["zustand"].get("meldung")
        ende = time.monotonic() + 60
        while c.proc.poll() is None and time.monotonic() < ende:
            try: c.befehl("beenden")
            except (urllib.error.URLError, ConnectionError, TimeoutError): pass
            try: c.proc.wait(3)
            except subprocess.TimeoutExpired: pass
        if c.proc.poll() is None: raise SystemExit("FEHLER: Cockpit schließt nach „beenden“ nicht")
        code = c.proc.returncode
        if code != 0: raise SystemExit(f"FEHLER: Cockpit endete mit Code {code}")
        if (c.arbeit / ".cockpit-laeuft").exists(): raise SystemExit("FEHLER: Marker .cockpit-laeuft blieb – kein reguläres Ende")
        log = c.arbeit / "cockpit-fehler.log"
        if log.is_file() and "Programmfehler" in log.read_text(encoding="utf-8", errors="replace"):
            raise SystemExit("FEHLER: Programmfehler protokolliert:\n" + log.read_text(encoding="utf-8", errors="replace")[-4000:])
        schritt(f"Beendet: regulär, kein Programmfehler · letzte Meldung: {meldung}")
    finally:
        if c.proc.poll() is None:
            c.proc.kill(); c.proc.wait(10)
        c.ollama.shutdown()
        for datei in ("bau.log", "cockpit-fehler.log"):
            p = c.arbeit / datei
            if p.is_file() and p.stat().st_size: print(f"--- {datei} (Ende) ---\n" + p.read_text(encoding="utf-8", errors="replace")[-3000:])
        if c.ausgabe: print("--- Ausgabe des Cockpits (Ende) ---\n" + "\n".join(c.ausgabe[-40:]))


def main():
    if not EXE.is_file(): sys.exit(f"FEHLT: {EXE}")
    ordner = Path(tempfile.mkdtemp(prefix="cockpit-startpruefung-"))
    sieger = threading.Timer(ZEIT, lambda: (print(f"FEHLER: Gesamtzeit {ZEIT} s überschritten", flush=True), os._exit(3)))
    sieger.daemon = True; sieger.start()
    try:
        pruefen(ordner)
    except SystemExit as e:
        print(e, flush=True); print(f"Prüfordner bleibt zur Ansicht: {ordner}"); sys.exit(1)
    shutil.rmtree(ordner, ignore_errors=True)
    print("PASS: Startprüfung bestanden", flush=True)


if __name__ == "__main__":
    main()
