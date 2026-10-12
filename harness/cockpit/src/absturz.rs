//! Absturzschutz. Unter Windows ist das Cockpit ein Fensterprogramm ohne Konsole: Ein
//! interner Fehler (Panic) beendete es bisher spurlos – das Fenster war einfach weg.
//! Jetzt gilt:
//! - jeder Programmfehler landet mit Zeit, Version und Ablauf in `cockpit-fehler.log`
//!   im Arbeitsordner (begrenzt auf etwa 1 MB);
//! - die Oberfläche fängt Fehler eines Bildes ab und läuft weiter (siehe main.rs);
//! - ein Start nach einem nicht regulären Ende meldet das mit dem letzten Eintrag;
//! - kann das Fenster gar nicht erst öffnen (Grafiktreiber), erscheint ein Hinweisfenster.
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

static PROTOKOLL: OnceLock<PathBuf> = OnceLock::new();
static ZULETZT: Mutex<Option<(String, i64)>> = Mutex::new(None);
const MARKER: &str = ".cockpit-laeuft";
const GRENZE: u64 = 1_000_000;

/// Richtet Protokoll und Panic-Haken ein. Liefert eine Meldung, falls die letzte Sitzung
/// nicht regulär endete (Marker der letzten Sitzung noch vorhanden).
pub fn einrichten(ordner: &Path) -> Option<String> {
    let _ = std::fs::create_dir_all(ordner);
    let protokoll = ordner.join("cockpit-fehler.log");
    let _ = PROTOKOLL.set(protokoll.clone());
    let marker = ordner.join(MARKER);
    let seit: Option<i64> = std::fs::read_to_string(&marker).ok().and_then(|t| t.trim().parse().ok());
    let meldung = seit.map(|seit| {
        let eintrag = letzter_eintrag(&protokoll).filter(|(zeit, _)| *zeit >= seit);
        match eintrag {
            Some((_, text)) => format!("Das Cockpit wurde beim letzten Mal nicht regulär beendet. Zuletzt protokolliert: {text} – Protokoll: {}", protokoll.display()),
            None => format!("Das Cockpit wurde beim letzten Mal nicht regulär beendet (kein Programmfehler protokolliert – vermutlich von außen beendet, z. B. Task-Manager, Neustart oder Grafiktreiber). Protokoll: {}", protokoll.display()),
        }
    });
    let _ = std::fs::write(&marker, crate::app::jetzt().to_string());
    let standard = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let ort = info.location().map(|o| format!("{}:{}", o.file(), o.line())).unwrap_or_default();
        let text = format!("{} ({ort})", nutzlast(info.payload()));
        // Ein Fehler, der sich jedes Bild wiederholt, steht nur einmal je Minute im Protokoll.
        let jetzt = crate::app::jetzt();
        let mut z = ZULETZT.lock().unwrap_or_else(|p| p.into_inner());
        if z.as_ref().is_some_and(|(t, zeit)| *t == text && jetzt - zeit < 60) { return; }
        *z = Some((text.clone(), jetzt));
        drop(z);
        notieren(&format!("Programmfehler: {text}\n{}", std::backtrace::Backtrace::force_capture()));
        standard(info);
    }));
    meldung
}

/// Hängt einen Eintrag an das Protokoll an (ohne Geheimnisse: nur Fehlertexte und Abläufe).
pub fn notieren(text: &str) {
    let Some(p) = PROTOKOLL.get() else { return };
    if std::fs::metadata(p).is_ok_and(|m| m.len() > GRENZE) {
        // Nur das jüngste Viertel behalten.
        if let Ok(alt) = std::fs::read(p) {
            let rest = &alt[alt.len().saturating_sub(GRENZE as usize / 4)..];
            let _ = std::fs::write(p, rest);
        }
    }
    if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(p) {
        let jetzt = crate::app::jetzt();
        let _ = writeln!(f, "=== {jetzt} · {} · Cockpit {} · {}\n{text}", crate::app::zeitstempel(jetzt), env!("CARGO_PKG_VERSION"), std::env::consts::OS);
    }
}

/// Reguläres Ende: Marker entfernen.
pub fn sauber_beendet(ordner: &Path) {
    let _ = std::fs::remove_file(ordner.join(MARKER));
}

/// Text einer abgefangenen Panik.
pub fn nutzlast(p: &(dyn std::any::Any + Send)) -> String {
    p.downcast_ref::<&str>().map(|s| s.to_string()).or_else(|| p.downcast_ref::<String>().cloned()).unwrap_or_else(|| "unbekannter Fehler".into())
}

fn letzter_eintrag(p: &Path) -> Option<(i64, String)> {
    let text = std::fs::read_to_string(p).ok()?;
    let block = text.rsplit("=== ").next()?;
    let zeit = block.split_whitespace().next()?.parse().ok()?;
    let inhalt: String = block.lines().nth(1).unwrap_or("").chars().take(300).collect();
    Some((zeit, inhalt))
}

/// Hinweis, wenn das Fenster selbst nicht startet (ohne Konsole sähe man sonst nichts).
pub fn hinweisfenster(titel: &str, text: &str) {
    notieren(&format!("{titel}: {text}"));
    #[cfg(windows)]
    unsafe {
        use windows_sys::Win32::UI::WindowsAndMessaging::{MessageBoxW, MB_ICONERROR, MB_OK};
        let breit = |s: &str| s.encode_utf16().chain(std::iter::once(0)).collect::<Vec<u16>>();
        MessageBoxW(std::ptr::null_mut(), breit(text).as_ptr(), breit(titel).as_ptr(), MB_OK | MB_ICONERROR);
    }
    #[cfg(not(windows))]
    eprintln!("{titel}: {text}");
}

#[cfg(test)]
mod tests {
    #[test]
    fn letzter_eintrag_liefert_zeit_und_erste_zeile() {
        let d = std::env::temp_dir().join(format!("absturz-test-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&d);
        let p = d.join("log");
        std::fs::write(&p, "=== 100 · x\nalt\n=== 200 · y · z\nProgrammfehler: kaputt (a.rs:1)\nstack\n").unwrap();
        assert_eq!(super::letzter_eintrag(&p), Some((200, "Programmfehler: kaputt (a.rs:1)".into())));
        let _ = std::fs::remove_dir_all(&d);
    }
}
