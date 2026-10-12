//! Sternenepoche Harness-Cockpit: Harness zusammenstellen, lokal bauen, einstellen und spielen.
//! Rust hält Fenster, Zustand, Bau, Harness-Prozess und Geheimnisse; die Oberfläche ist eine
//! lokale Seite in WebView2 im Look des Spiels (oberflaeche.rs, ui/).
#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]
mod abhaengigkeiten;
mod abnahme;
mod absturz;
mod ansicht;
mod app;
mod bau;
mod browser;
mod entwicklung;
mod katalog;
mod laufzeit;
mod lokale_agenten;
mod lokaler_spielstand;
mod testpiloten;
mod oberflaeche;
mod oeffnen;
mod stimmen;
mod tresor;
mod verbindung;
mod vorlagen;
pub mod quelle { include!(concat!(env!("OUT_DIR"), "/quelle.rs")); }

use app::App;
use eframe::egui;
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::hash::{Hash, Hasher};

struct Fenster {
    /// Beim ersten Bild minimieren (--minimiert, für Test- und Entwicklungsinstanzen).
    minimiert: bool,
    app: App,
    ui: Option<oberflaeche::Oberflaeche>,
    ui_fehler: Option<String>,
    dev: Option<entwicklung::Entwicklung>,
    stand: BTreeMap<&'static str, u64>,
    letzter_push: f64,
    abnahme: Option<abnahme::Abnahme>,
}

fn pruefsumme(s: &str) -> u64 { let mut h = std::collections::hash_map::DefaultHasher::new(); s.hash(&mut h); h.finish() }

impl Fenster {
    fn senden(&self, art: &str, d: &Value) {
        if let Some(u) = &self.ui { u.senden(art, d); }
        if let Some(e) = &self.dev { e.senden(art, d); }
    }
    /// Nur Geändertes schicken: Zustand, Einstellungen, Daten, Katalog; Ereignisse als Strom.
    fn abgleichen(&mut self, sofort: bool) {
        if !sofort && self.app.zeit - self.letzter_push < 0.2 { return; }
        self.letzter_push = self.app.zeit;
        let teile: [(&'static str, Value); 4] = [("katalog", ansicht::katalog(&self.app)), ("auswahl", ansicht::auswahl(&self.app)), ("daten", ansicht::daten(&self.app)), ("zustand", ansicht::zustand(&mut self.app))];
        for (art, wert) in teile {
            let text = wert.to_string();
            let h = pruefsumme(&text);
            if self.stand.get(art) != Some(&h) { self.stand.insert(art, h); self.senden(art, &wert); }
        }
        let neu = std::mem::take(&mut self.app.cockpit.neu);
        if !neu.is_empty() { self.senden("ereignisse", &json!(neu)); }
    }
    fn nachricht(&mut self, roh: &str) {
        let Ok(v) = serde_json::from_str::<Value>(roh) else { return };
        match v["t"].as_str().unwrap_or("") {
            // Neu geladene Seite: alles noch einmal schicken.
            "hallo" => { self.stand.clear(); self.abgleichen(true); }
            "verlauf_detail" => {
                let i = v["i"].as_u64().unwrap_or(0) as usize;
                if let Some(e) = self.app.cockpit.verlauf.get(i).cloned() { self.senden("detail", &json!({"art":"verlauf","i":i,"eintrag":e})); }
            }
            _ => {
                if let Err(e) = ansicht::befehl(&mut self.app, &v) { self.app.meldung = Some((e, true)); }
                self.abgleichen(true);
            }
        }
    }
}

impl eframe::App for Fenster {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        // Ein Fehler in einem Bild darf das Fenster nicht schließen: abfangen, protokollieren
        // (Panic-Haken in absturz.rs), anzeigen und weiterlaufen.
        if let Err(p) = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| self.bild(ctx, frame))) {
            self.app.meldung = Some((format!("Interner Fehler abgefangen, das Cockpit läuft weiter: {} – Details in {}", absturz::nutzlast(p.as_ref()), self.app.auswahl.ordner().join("cockpit-fehler.log").display()), true));
            ctx.request_repaint();
        }
    }
    fn on_exit(&mut self, _: Option<&eframe::glow::Context>) {
        self.app.agenten.take();
        if let Some(l) = &mut self.app.laufzeit { l.stoppen(); }
        self.app.lokal.take();
        absturz::sauber_beendet(&self.app.auswahl.ordner());
    }
}

impl Fenster {
    fn bild(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        #[cfg(target_os = "linux")]
        while gtk::events_pending() { gtk::main_iteration_do(false); }
        if self.minimiert { self.minimiert = false; ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(true)); }
        self.app.zeit = ctx.input(|i| i.time);
        self.app.einsammeln();
        let r = ctx.screen_rect();
        egui::CentralPanel::default().frame(egui::Frame::new().fill(egui::Color32::from_rgb(7, 13, 22))).show(ctx, |ui| {
            if let Some(f) = &self.ui_fehler { ui.colored_label(egui::Color32::from_rgb(255, 120, 120), f); }
        });
        if self.ui.is_none() && self.ui_fehler.is_none() {
            match oberflaeche::Oberflaeche::neu(frame, &self.app.auswahl.ordner().join("oberflaeche-daten"), (r.width(), r.height())) {
                Ok(u) => self.ui = Some(u),
                Err(e) => self.ui_fehler = Some(e),
            }
        }
        if let Some(u) = &self.ui { u.groesse(r.width(), r.height()); }
        let mut eingang: Vec<String> = Vec::new();
        if let Some(u) = &self.ui { eingang.extend(u.nachrichten.try_iter()); }
        if let Some(d) = &self.dev { eingang.extend(d.befehle.try_iter()); }
        for m in eingang { self.nachricht(&m); }
        self.abgleichen(false);
        // Das Spiel liegt als eigener, abgeschotteter Browser über dem Bereich, den die Oberfläche freihält.
        match self.app.spielbereich {
            Some((x, y, b, h)) => {
                let ziel = self.app.browser_ziel.take();
                let daten = self.app.auswahl.ordner().join("browser-daten");
                self.app.browser.zeigen(frame, egui::Rect::from_min_size(egui::pos2(x, y), egui::vec2(b, h)), self.app.auswahl.server.trim_end_matches('/'), ziel, &daten);
            }
            None => self.app.browser.verstecken(),
        }
        if std::mem::take(&mut self.app.minimieren) { ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(true)); }
        if std::mem::take(&mut self.app.beenden) {ctx.send_viewport_cmd(egui::ViewportCommand::Close);}
        if let Some(a) = &mut self.abnahme {
            let befehle = a.schritt(&mut self.app, ctx);
            for b in befehle { self.senden("steuern", &b); }
        }
        let schnell = self.app.laeuft() || self.app.bau_laeuft() || self.abnahme.is_some() || self.app.cockpit.wissen_laeuft;
        ctx.request_repaint_after(std::time::Duration::from_millis(if schnell { 120 } else { 300 }));
    }
}

fn main() -> eframe::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    // Paket-Skript: Stand des eingebetteten Harness-Quellcodes (für sternenepoche-harness.json des vorgebauten Harness).
    if args.iter().any(|a| a == "--quelle-kennung") { println!("{}", quelle::QUELLE_HASH); return Ok(()); }
    let wert = |n: &str| args.iter().position(|a| a == n).and_then(|i| args.get(i + 1)).cloned();
    // Eigene Einstellungsdatei (Tests, Entwicklung, mehrere Profile nebeneinander).
    if let Some(p) = wert("--einstellungen") { std::env::set_var("STERNENEPOCHE_COCKPIT_EINSTELLUNGEN", p); }
    let mut auswahl = katalog::laden();
    if let Some(o) = wert("--arbeitsordner") { auswahl.arbeitsordner = o; auswahl.seelen_laden(); }
    if let Some(s) = wert("--server") { auswahl.server = s; }
    if let Some(u) = wert("--ollama") { auswahl.ollama_url = u; }
    // Linux: Die eingebetteten Webansichten (WebKitGTK) brauchen GTK und X11. Unter Wayland
    // läuft das Fenster deshalb über XWayland; ohne GTK erschiene keine Oberfläche.
    #[cfg(target_os = "linux")]
    {
        if std::env::var_os("WAYLAND_DISPLAY").is_some() && std::env::var_os("DISPLAY").is_some() { std::env::remove_var("WAYLAND_DISPLAY"); }
        if let Err(e) = gtk::init() { absturz::hinweisfenster("Sternenepoche-Cockpit", &format!("GTK konnte nicht starten ({e}). Bitte WebKitGTK 4.1 und GTK 3 installieren (z. B. libwebkit2gtk-4.1-0).")); }
    }
    let unerwartet = absturz::einrichten(&auswahl.ordner());
    let mut app = App::neu(auswahl);
    if let Some(m) = unerwartet { app.meldung = Some((m, true)); }
    let dev = wert("--entwicklung").and_then(|p| p.parse::<u16>().ok()).and_then(|p| match entwicklung::Entwicklung::starten(p, wert("--ui-ordner").map(std::path::PathBuf::from)) {
        Ok(d) => { println!("ENTWICKLUNG {}", d.adresse); let _ = std::fs::create_dir_all(app.auswahl.ordner()); let _ = std::fs::write(app.auswahl.ordner().join("entwicklung.txt"), &d.adresse); Some(d) }
        Err(e) => { eprintln!("{e}"); None }
    });
    let abnahme = wert("--abnahme").map(|o| { let o = std::path::PathBuf::from(o); let _ = std::fs::create_dir_all(&o); abnahme::Abnahme::neu(o) });
    let optionen = eframe::NativeOptions {
        // Test- und Entwicklungsinstanzen tragen einen Zusatz im Titel (--titel) und starten auf Wunsch minimiert (--minimiert),
        // damit niemand sie mit dem echten Cockpit verwechselt.
        viewport: egui::ViewportBuilder::default().with_title(match wert("--titel") { Some(t) => format!("Sternenepoche · Harness-Cockpit · {t}"), None => "Sternenepoche · Harness-Cockpit".into() })
            .with_inner_size([1640.0, 980.0]).with_min_inner_size([1180.0, 720.0]),
        ..Default::default()
    };
    let ergebnis = eframe::run_native("sternenepoche-cockpit", optionen, Box::new(move |cc| {
        cc.egui_ctx.set_visuals(egui::Visuals::dark());
        Ok(Box::new(Fenster { minimiert: args.iter().any(|a| a == "--minimiert"), app, ui: None, ui_fehler: None, dev, stand: BTreeMap::new(), letzter_push: 0.0, abnahme }))
    }));
    if let Err(e) = &ergebnis {
        absturz::hinweisfenster("Sternenepoche-Cockpit", &format!("Das Fenster konnte nicht geöffnet werden: {e}. Häufige Ursache: fehlender oder veralteter Grafiktreiber (OpenGL 2) oder eine Remotedesktop-Sitzung."));
    }
    ergebnis
}
