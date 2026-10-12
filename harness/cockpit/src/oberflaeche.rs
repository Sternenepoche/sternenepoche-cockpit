//! Die Oberfläche des Cockpits: eine lokale Seite in der eingebauten WebView2 (wie der
//! Spielbrowser), gezeichnet im Look des Spiels. Sie kommt ausschließlich aus dem Programm
//! selbst (eigenes Protokoll „cockpit“), erreicht kein Netz (strikte CSP, keine Navigation,
//! keine neuen Fenster) und spricht mit dem Rust-Kern nur über eine Nachrichtenleitung.
//! Aller Zustand, alle Prüfungen und alle Geheimnisse bleiben im Rust-Kern.
use std::borrow::Cow;
use std::sync::mpsc::{channel, Receiver};
use wry::{dpi::{LogicalPosition, LogicalSize}, http::{header::CONTENT_TYPE, Response}, Rect, WebContext, WebView, WebViewBuilder};

mod eingebettet { include!(concat!(env!("OUT_DIR"), "/oberflaeche.rs")); }

pub const URSPRUNG: &str = "http://cockpit.localhost";
pub const CSP: &str = "default-src 'self'; script-src 'self'; style-src 'self'; img-src 'self' blob: data:; connect-src 'self' blob:; font-src 'self'; media-src 'self'; object-src 'none'; frame-src 'none'; base-uri 'none'; form-action 'none'";

pub fn art(pfad: &str) -> &'static str {
    match pfad.rsplit('.').next().unwrap_or("") {
        "html" => "text/html; charset=utf-8", "css" => "text/css; charset=utf-8", "js" => "text/javascript; charset=utf-8",
        "webp" => "image/webp", "png" => "image/png", "jpg" | "jpeg" => "image/jpeg", "svg" => "image/svg+xml",
        "glb" => "model/gltf-binary", "ogg" => "audio/ogg", "wav" => "audio/wav", "json" => "application/json", "woff2" => "font/woff2", _ => "application/octet-stream",
    }
}

/// Eingebettete Datei zu einem Pfad der Oberfläche (ohne führenden Schrägstrich).
pub fn datei(pfad: &str) -> Option<&'static [u8]> {
    let p = pfad.trim_start_matches('/').split(['?', '#']).next().unwrap_or("");
    let p = if p.is_empty() { "index.html" } else { p };
    eingebettet::DATEIEN_UI.iter().find(|(n, _)| *n == p).map(|(_, b)| *b)
}
pub fn vorhanden(praefix: &str) -> Vec<&'static str> { eingebettet::DATEIEN_UI.iter().filter(|(n, _)| n.starts_with(praefix)).map(|(n, _)| *n).collect() }

pub struct Oberflaeche { pub webview: WebView, _kontext: WebContext, pub nachrichten: Receiver<String> }

impl Oberflaeche {
    pub fn neu(frame: &eframe::Frame, daten: &std::path::Path, groesse: (f32, f32)) -> Result<Self, String> {
        let (tx, rx) = channel();
        let mut kontext = WebContext::new(Some(daten.to_path_buf()));
        let webview = WebViewBuilder::new_with_web_context(&mut kontext)
            .with_custom_protocol("cockpit".into(), |_id, anfrage| {
                let pfad = anfrage.uri().path().to_string();
                // Gesprochene Antworten des Rats (live/<id>.wav) liegen nur im Speicher.
                if let Some(b) = crate::stimmen::live(&pfad) {
                    return Response::builder().header(CONTENT_TYPE, "audio/wav").header("Cache-Control", "no-store").body(Cow::Owned(b)).unwrap();
                }
                match datei(&pfad) {
                    Some(b) => Response::builder().header(CONTENT_TYPE, art(if pfad == "/" { "index.html" } else { &pfad }))
                        .header("Content-Security-Policy", CSP).header("X-Content-Type-Options", "nosniff").header("Cache-Control", "no-store")
                        .body(Cow::Borrowed(b)).unwrap(),
                    None => Response::builder().status(404).body(Cow::Borrowed(&b"nicht gefunden"[..])).unwrap(),
                }
            })
            .with_url(format!("{URSPRUNG}/index.html"))
            .with_bounds(Rect { position: LogicalPosition::new(0.0, 0.0).into(), size: LogicalSize::new(groesse.0 as f64, groesse.1 as f64).into() })
            .with_background_color((7, 13, 22, 255))
            .with_devtools(cfg!(debug_assertions))
            .with_clipboard(true)
            .with_hotkeys_zoom(false)
            .with_autoplay(true)
            .with_navigation_handler(|u| u.starts_with(URSPRUNG))
            .with_new_window_req_handler(|_, _| wry::NewWindowResponse::Deny)
            .with_download_started_handler(|_, _| false)
            .with_drag_drop_handler(|_| true)
            .with_ipc_handler(move |anfrage| {
                // Nur Nachrichten der eigenen Seite annehmen.
                if anfrage.uri().to_string().starts_with(URSPRUNG) { let _ = tx.send(anfrage.into_body()); }
            })
            .build_as_child(frame).map_err(|e| format!("Oberfläche (WebView2) nicht verfügbar: {e}"))?;
        Ok(Self { webview, _kontext: kontext, nachrichten: rx })
    }
    pub fn groesse(&self, b: f32, h: f32) {
        let _ = self.webview.set_bounds(Rect { position: LogicalPosition::new(0.0, 0.0).into(), size: LogicalSize::new(b.max(1.0) as f64, h.max(1.0) as f64).into() });
    }
    /// Nachricht an die Seite: window.__cockpit(art, daten).
    pub fn senden(&self, art: &str, daten: &serde_json::Value) {
        let _ = self.webview.evaluate_script(&format!("window.__cockpit&&window.__cockpit({},{})", serde_json::to_string(art).unwrap(), daten));
    }
}
