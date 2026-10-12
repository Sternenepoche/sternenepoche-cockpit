//! Eingebetteter Browser im Cockpit-Fenster. Er erreicht ausschließlich die Seite des
//! Spielservers: andere Adressen, neue Fenster, Downloads und Entwicklerwerkzeuge sind
//! gesperrt. Gleiche Seite in neuem Tab öffnet im selben Fenster.
use std::path::Path;
use std::sync::{Arc, Mutex};
use wry::{dpi::{LogicalPosition, LogicalSize}, NewWindowResponse, Rect, WebContext, WebView, WebViewBuilder};

pub fn erlaubt(url: &str, ursprung: &str) -> bool {
    url == "about:blank" || url == ursprung || url.strip_prefix(ursprung).is_some_and(|r| r.starts_with('/') || r.starts_with('#') || r.starts_with('?'))
}

pub struct Einbettung {
    webview: Option<WebView>,
    _kontext: Option<WebContext>,
    ursprung: String,
    nachladen: Arc<Mutex<Option<String>>>,
    pub fehler: Option<String>,
    sichtbar: bool,
}

fn rechteck(r: eframe::egui::Rect) -> Rect {
    Rect { position: LogicalPosition::new(r.min.x as f64, r.min.y as f64).into(), size: LogicalSize::new(r.width().max(1.0) as f64, r.height().max(1.0) as f64).into() }
}

impl Einbettung {
    pub fn neu() -> Self { Self { webview: None, _kontext: None, ursprung: String::new(), nachladen: Default::default(), fehler: None, sichtbar: false } }

    pub fn zeigen(&mut self, frame: &eframe::Frame, r: eframe::egui::Rect, ursprung: &str, adresse: Option<String>, daten: &Path) {
        if self.webview.is_some() && self.ursprung != ursprung { self.webview = None; }
        if let Some(w) = &self.webview {
            let _ = w.set_bounds(rechteck(r));
            if !self.sichtbar { let _ = w.set_visible(true); self.sichtbar = true; }
            if let Some(u) = adresse.filter(|u| erlaubt(u, ursprung)) { let _ = w.load_url(&u); }
            if let Some(u) = self.nachladen.lock().unwrap().take() { let _ = w.load_url(&u); }
            return;
        }
        let start = adresse.unwrap_or_else(|| format!("{ursprung}/"));
        let mut kontext = WebContext::new(Some(daten.to_path_buf()));
        let (o1, o2, nach) = (ursprung.to_string(), ursprung.to_string(), self.nachladen.clone());
        let ergebnis = WebViewBuilder::new_with_web_context(&mut kontext)
            .with_url(&start)
            .with_bounds(rechteck(r))
            .with_devtools(false)
            .with_clipboard(true)
            .with_hotkeys_zoom(true)
            .with_navigation_handler(move |u| erlaubt(&u, &o1))
            .with_new_window_req_handler(move |u, _| { if erlaubt(&u, &o2) { *nach.lock().unwrap() = Some(u); } NewWindowResponse::Deny })
            .with_download_started_handler(|_, _| false)
            .build_as_child(frame);
        match ergebnis {
            Ok(w) => { self.webview = Some(w); self._kontext = Some(kontext); self.ursprung = ursprung.into(); self.sichtbar = true; self.fehler = None; }
            Err(e) => self.fehler = Some(format!("Eingebetteter Browser nicht verfügbar ({e}). „Go to Browser“ nutzt deinen normalen Browser.")),
        }
    }
    pub fn verstecken(&mut self) {
        if let Some(w) = &self.webview { if self.sichtbar { let _ = w.set_visible(false); } }
        self.sichtbar = false;
    }
}

#[cfg(test)]
mod tests {
    use super::erlaubt;
    #[test]
    fn nur_eigene_seite() {
        let o = "https://spiel.example";
        for gut in ["https://spiel.example/", "https://spiel.example/#uebergabe=ab", "https://spiel.example/harness-regeln.html", "about:blank"] { assert!(erlaubt(gut, o), "{gut}"); }
        for boese in ["https://spiel.example.evil.com/", "https://evil.com/?https://spiel.example/", "http://spiel.example/", "https://spiel.examplex/", "file:///C:/", "javascript:alert(1)"] { assert!(!erlaubt(boese, o), "{boese}"); }
    }
}
