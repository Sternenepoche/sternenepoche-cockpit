//! Beschreibungskatalog mit Rolle, Zweck, Stärke, Nische und Lore je Eintrag (docs/DESIGNREGELN.md §6).
//! Techinfo-Fenster und Kachel-Hover der Oberfläche lesen ihn über `/beschreibungen.json`.

/// Beim Bau eingebettete Fassung (build.rs); leerer Katalog, falls die Datei damals fehlte.
const EINGEBETTET: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/beschreibungen.json"));
/// Arbeitsstand im Projekt, nur im Debug-Bau (Vorschau): die Datei darf sich dort ohne Neubau ändern. Release-Bauten
/// (Server, App, Cockpit-Paket) nehmen die eingebettete Fassung – der Projektpfad stünde sonst im ausgelieferten Binary
/// (Pfadprüfung von harness/paket/paket_bauen.py, 29.10.2026).
#[cfg(debug_assertions)]
const PROJEKTDATEI: Option<&str> = Some(concat!(env!("CARGO_MANIFEST_DIR"), "/../../content/beschreibungen.json"));
#[cfg(not(debug_assertions))]
const PROJEKTDATEI: Option<&str> = None;

/// Gültiges JSON des Katalogs: im Debug-Bau zuerst die aktuelle Projektdatei, sonst die eingebettete Fassung.
pub fn json() -> Vec<u8> {
    if let Some(Ok(mut b)) = PROJEKTDATEI.map(std::fs::read) {
        if b.starts_with(&[0xEF, 0xBB, 0xBF]) {
            b.drain(..3);
        }
        let passt = b.len() < 8 * 1024 * 1024
            && serde_json::from_slice::<serde_json::Value>(&b).is_ok_and(|v| v.get("eintraege").is_some_and(|e| e.is_object()));
        if passt {
            return b;
        }
    }
    EINGEBETTET.to_vec()
}

#[cfg(test)]
mod tests {
    #[test]
    fn katalog_ist_immer_gueltiges_json_mit_eintraegen() {
        let v: serde_json::Value = serde_json::from_slice(&super::json()).expect("gültiges JSON");
        assert!(v["eintraege"].is_object());
    }
}
