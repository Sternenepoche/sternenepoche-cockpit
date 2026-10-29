//! Regelquellen neuer Welten als Texte (ConfigWelt, 21.10.2026).
//!
//! Der Server bettet die Regeldateien beim Übersetzen ein (`include_str!`). Die ConfigWelt muss geänderte, noch nicht
//! übersetzte Fassungen prüfen und ihre Wirkung zeigen. [`neue_welt`] baut deshalb eine Welt genau wie
//! `sternenepoche_server::fresh_with` (Ausbau, Monde, Invasion, Allianzen v2, Dyson, Technik v1 samt Kernwerten), nur
//! aus den übergebenen Texten. Mit [`RegelQuellen::eingebaut`] entsteht dieselbe Welt wie im Server (Test unten).
use crate::flottenausbau::{Ausbau, FLOTTEN_JSON};
use crate::{typen::*, Regelwerk, Welt};
use std::path::Path;

pub const ONLINE_RON: &str = include_str!("../../../regeln/online-v1.ron");

/// Texte aller Regeldateien neuer Welten.
#[derive(Clone, Debug)]
pub struct RegelQuellen {
    pub online: String,
    pub monde: String,
    pub dyson: String,
    pub allianz: String,
    pub technik: String,
    pub kernwerte: String,
    pub flotten: String,
}

/// Dateinamen unter `regeln/` in der Reihenfolge der Felder.
pub const DATEIEN: [&str; 7] = ["online-v1.ron", "monde.ron", "dyson.ron", "allianz.ron", "technik.ron", "kernwerte.ron", "flottenausbau.json"];

impl RegelQuellen {
    /// Die beim Übersetzen eingebetteten Texte (= Stand des laufenden Programms).
    pub fn eingebaut() -> RegelQuellen {
        RegelQuellen {
            online: ONLINE_RON.into(),
            monde: crate::monde::REGELN_RON.into(),
            dyson: crate::dyson::REGELN_RON.into(),
            allianz: crate::dyson::ALLIANZ_RON.into(),
            technik: crate::technik::REGELN_RON.into(),
            kernwerte: crate::kernwerte::REGELN_RON.into(),
            flotten: FLOTTEN_JSON.into(),
        }
    }
    /// Liest alle Dateien aus einem Ordner `regeln/`.
    pub fn aus_ordner(ordner: &Path) -> Result<RegelQuellen, String> {
        let lies = |n: &str| std::fs::read_to_string(ordner.join(n)).map_err(|e| format!("{n}: {e}"));
        Ok(RegelQuellen {
            online: lies(DATEIEN[0])?,
            monde: lies(DATEIEN[1])?,
            dyson: lies(DATEIEN[2])?,
            allianz: lies(DATEIEN[3])?,
            technik: lies(DATEIEN[4])?,
            kernwerte: lies(DATEIEN[5])?,
            flotten: lies(DATEIEN[6])?,
        })
    }
    pub fn text(&self, datei: &str) -> Option<&str> {
        Some(match datei {
            "online-v1.ron" => &self.online,
            "monde.ron" => &self.monde,
            "dyson.ron" => &self.dyson,
            "allianz.ron" => &self.allianz,
            "technik.ron" => &self.technik,
            "kernwerte.ron" => &self.kernwerte,
            "flottenausbau.json" => &self.flotten,
            _ => return None,
        })
    }
    pub fn text_mut(&mut self, datei: &str) -> Option<&mut String> {
        Some(match datei {
            "online-v1.ron" => &mut self.online,
            "monde.ron" => &mut self.monde,
            "dyson.ron" => &mut self.dyson,
            "allianz.ron" => &mut self.allianz,
            "technik.ron" => &mut self.technik,
            "kernwerte.ron" => &mut self.kernwerte,
            "flottenausbau.json" => &mut self.flotten,
            _ => return None,
        })
    }
}

/// Einheitenwerte und Doktrin aus einer (geänderten) Flottendatei übernehmen – für alle Einheiten, die die Welt nach
/// Ausbau und Mondgeneration kennt, außer Pathfinder und Bergungsschiff (Kernwerte). Danach wie in der Mondgeneration
/// das Schnellfeuer des Zerstörers gegen Schlachtkreuzer. Industrie-Rezepte und Wirkungstexte setzt anschließend
/// `technik_aktivieren_aus`.
pub(crate) fn flotten_uebernehmen(w: &mut Welt, text: &str) -> Result<(), String> {
    let a = Ausbau::laden(text)?;
    let r = std::sync::Arc::make_mut(&mut w.regeln);
    let kennt: Vec<Einheit> = r.einheiten.keys().copied().filter(|e| !matches!(e, Einheit::Pathfinder | Einheit::Bergungsschiff)).collect();
    for e in kennt {
        let u = a.units.get(&e).ok_or_else(|| format!("Flottendatei: Einheit {e} fehlt"))?;
        r.einheiten.insert(e, u.clone());
        if r.flottendoktrin.contains_key(&e) {
            let p = a.profiles.get(&e).ok_or_else(|| format!("Flottendatei: Profil {e} fehlt"))?;
            r.flottendoktrin.insert(e, p.clone());
        }
    }
    if r.monde.is_some() {
        if let Some(z) = r.einheiten.get_mut(&Einheit::Zerstoerer) { z.schnellfeuer.insert(Einheit::Schlachtkreuzer, 2); }
    }
    Ok(())
}

/// Eine neue Welt wie auf dem Server (`fresh_with`), aus den gegebenen Regeltexten. `spieler` Plätze, alle Heimatwelten
/// angepasst und Raten berechnet.
pub fn neue_welt(q: &RegelQuellen, seed: u64, spieler: usize) -> Result<Welt, String> {
    let mut r = Regelwerk::laden(&q.online)?;
    r.welt.fenster_sekunden = 1;
    let mut w = Welt::neu(r, seed, spieler)?;
    w.ausbau_aktivieren()?;
    w.monde_aktivieren_aus(&q.monde)?;
    w.invasion_aktivieren()?;
    w.allianz_v2_aktivieren_aus(&q.allianz)?;
    w.dyson_aktivieren_aus(&q.dyson)?;
    flotten_uebernehmen(&mut w, &q.flotten)?;
    w.technik_aktivieren_aus(&q.technik, &q.kernwerte)?;
    // Der Terraformer kommt erst mit Technik v1 ins Regelwerk: seine Werte ebenfalls aus der (geänderten) Flottendatei.
    if w.terraformer_welt() {
        let a = Ausbau::laden(&q.flotten)?;
        let e = Einheit::Terraformer;
        let r = std::sync::Arc::make_mut(&mut w.regeln);
        r.einheiten.insert(e, a.units.get(&e).ok_or("Flottendatei: Einheit terraformer fehlt")?.clone());
        r.flottendoktrin.insert(e, a.profiles.get(&e).ok_or("Flottendatei: Profil terraformer fehlt")?.clone());
    }
    for sid in 0..w.spieler.len() {
        w.heimat_anpassen(sid as SpielerId);
    }
    for pid in 0..w.planeten.len() {
        w.raten_neu(pid);
    }
    Ok(w)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Mit den eingebauten Texten entsteht dieselbe Regelwelt wie über die gewohnten Aktivierungen (Server).
    #[test]
    fn eingebaute_quellen_gleich_server_kette() {
        let w = neue_welt(&RegelQuellen::eingebaut(), 7, 4).unwrap();
        let mut r = Regelwerk::laden(ONLINE_RON).unwrap();
        r.welt.fenster_sekunden = 1;
        let mut v = Welt::neu(r, 7, 4).unwrap();
        v.ausbau_aktivieren().unwrap();
        v.monde_aktivieren().unwrap();
        v.invasion_aktivieren().unwrap();
        v.allianz_v2_aktivieren().unwrap();
        v.dyson_aktivieren().unwrap();
        v.technik_aktivieren().unwrap();
        let j = |w: &Welt| serde_json::to_string(&(&w.regeln.einheiten, &w.regeln.flottendoktrin, &w.regeln.gebaeude, &w.regeln.forschung, &w.regeln.wirtschaft.bauteile)).unwrap();
        assert_eq!(j(&w), j(&v));
        assert_eq!(w.regeln.hash, v.regeln.hash);
    }

    /// Kernwerte als RON zurückgeschrieben ergeben dieselben Werte.
    #[test]
    fn kernwerte_ron_rundlauf() {
        let k = crate::kernwerte::KernWerte::default();
        let t = ron::ser::to_string_pretty(&k, ron::ser::PrettyConfig::default()).unwrap();
        assert_eq!(crate::kernwerte::KernWerte::laden(&t).unwrap(), k);
    }

    /// Eine Welt mit Standard-Kernwerten rechnet bitgleich wie eine Welt ohne Kernwerte (bisherige Konstanten).
    #[test]
    fn kernwerte_standard_bitgleich() {
        let mut a = neue_welt(&RegelQuellen::eingebaut(), 11, 6).unwrap();
        let setze = |w: &mut Welt, k: Option<crate::kernwerte::KernWerte>| {
            let r = std::sync::Arc::make_mut(&mut w.regeln);
            let mut t = (**r.technik.as_ref().unwrap()).clone();
            t.kernwerte = k;
            r.technik = Some(std::sync::Arc::new(t));
        };
        setze(&mut a, Some(crate::kernwerte::KernWerte::default()));
        let mut b = a.clone();
        setze(&mut b, None);
        let ziel = 2 * TAG;
        while a.zeit < ziel && a.schritt_bis(ziel) {}
        while b.zeit < ziel && b.schritt_bis(ziel) {}
        assert_eq!(a.zeit, b.zeit);
        let p = |w: &Welt| serde_json::to_string(&(w.planeten.iter().map(|p| (&p.bestand, p.bevoelkerung, p.stabilitaet)).collect::<Vec<_>>(),
            w.spieler.iter().map(|s| (s.credits, s.forschung, s.stufe)).collect::<Vec<_>>())).unwrap();
        assert_eq!(p(&a), p(&b));
    }

    /// Hüllen laufender Welten bleiben bytegleich: Felder, die nach 0.4.7 dazukamen, schreiben ihren Standardwert nicht
    /// (Befund huelle_bitgleich 25.10.2026 – sonst tauchten wellen, miliz_* … in jeder geschriebenen Welt neu auf).
    #[test]
    fn neue_kernwerte_schreiben_ihren_standard_nicht() {
        let s = serde_json::to_string(&crate::kernwerte::KernWerte::default()).unwrap();
        for k in ["\"orbitflotten\"", "\"wellen\"", "\"wellen_abstand_stunden\"", "\"wellen_runden_max\"", "\"miliz_anteil_promille\"",
            "\"miliz_jederzeit\"", "\"miliz_forschung\"", "\"miliz_ausbildung_sekunden_je_1000\"", "\"invasion_ausbildung_promille\"",
            "\"reparatur_stufe_promille\"", "\"schnellfeuer_springt\"", "\"topf_rueckbuchung\""] {
            assert!(!s.contains(k), "Standard-Kernwerte schreiben {k} – skip_serializing_if fehlt");
        }
    }
}
