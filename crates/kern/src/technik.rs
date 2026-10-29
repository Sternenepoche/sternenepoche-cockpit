//! Technik v1 (neue Welten ab 18.10.2026; Karl: „Ja, einführen“): Antriebswechsel per Forschung und Plasmatechnik.
//!
//! - **Antriebswechsel:** ab einer Forschungsstufe fliegt ein Schiff mit einem anderen Antrieb, neuem Grundtempo und
//!   neuem Verbrauch (OGame: Kleiner Transporter ab Impuls 5, Recycler ab Impuls/Hyperraum, Bomber ab Hyperraum 8).
//! - **Plasmatechnik:** eine neue Forschung (`Forschung::Plasmatechnik`), die die Förderung von Erz, Kristall und
//!   Deuterium je Stufe erhöht.
//!
//! Regelblock `regeln/technik.ron`; aktiviert nur mit [`Welt::technik_aktivieren`] bei Spielbeginn. Die Regeln liegen in
//! `ausbau.technik` (JSON-Erweiterung der Snapshot-Hülle) und gespiegelt in `regeln.technik` (nicht serialisiert), damit
//! Regeltext und Flugrechnung ohne Welt auskommen. Laufende Welten bleiben unverändert: keine neuen Felder in
//! vorhandenen Regel-Structs, der Binärteil speichert dort weiter 19 Forschungen; nur Welten mit Technik v1 speichern 20
//! (die Länge folgt aus `ausbau.technik`, der Kopf STERNE12/STERNE13 bleibt).
use crate::regeln::{EinheitRegel, ForschungRegel, Regelwerk};
use crate::typen::*;
use crate::Welt;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::sync::Arc;

pub const REGELN_RON: &str = include_str!("../../../regeln/technik.ron");

/// Eine Antriebsstufe: ab `forschung` Stufe `stufe` gilt `antrieb` mit Grundtempo `tempo` und Verbrauch `verbrauch`.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Antriebsstufe {
    pub forschung: Forschung,
    pub stufe: u8,
    pub antrieb: Forschung,
    pub tempo: i64,
    pub verbrauch: i64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TechnikRegeln {
    pub version: u8,
    /// Je Schiff aufsteigend geordnete Antriebsstufen; von mehreren erreichten gilt die letzte.
    pub antriebswechsel: BTreeMap<Einheit, Vec<Antriebsstufe>>,
    pub plasmatechnik: ForschungRegel,
    pub plasmatechnik_braucht: BTreeMap<Forschung, u8>,
    /// Förderbonus je Stufe Plasmatechnik als Anteil (0.02 = +2 Prozent).
    pub plasma_je_stufe: BTreeMap<Gut, f64>,
    /// Berichtigte Wirkungs-/Rollentexte neuer Welten (eingefrorene Texte der Doktrin v1 bleiben in der Datei).
    #[serde(default)]
    pub wirkung: BTreeMap<Einheit, String>,
    /// Energiefaktor der Fusionskraftwerke je Volk anstelle des allgemeinen Volksfaktors `energie` (Karl 20.10.2026:
    /// Fusionskraftwerke der Syntheten liefern 30 Prozent mehr als die anderer Völker, bei gleichen Kosten). Fehlt ein
    /// Volk, gilt sein `energie`. Lebt nur in dieser JSON-Hülle: ältere Welten lesen das Feld leer.
    #[serde(default)]
    pub fusion_volk: BTreeMap<Volk, f64>,
    /// Kernwerte aus `regeln/kernwerte.ron` (ConfigWelt, 21.10.2026): bisher feste Kernkonstanten. Nur in Welten, die
    /// nach diesem Stand starten; ältere Welten lesen `None` und rechnen mit den bisherigen Konstanten. Ohne Wert wird
    /// das Feld nicht geschrieben, damit Hülle und Hash laufender Welten gleich bleiben.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kernwerte: Option<crate::kernwerte::KernWerte>,
}

const ANTRIEBE: [Forschung; 3] = [Forschung::Verbrennungsantrieb, Forschung::Impulsantrieb, Forschung::Hyperraumantrieb];

impl TechnikRegeln {
    pub fn laden(text: &str) -> Result<TechnikRegeln, String> {
        let t: TechnikRegeln = ron::from_str(text).map_err(|e| format!("Technikregeln nicht lesbar: {e}"))?;
        if t.version != 1 {
            return Err("Technikregeln: unbekannte Version".into());
        }
        for (e, stufen) in &t.antriebswechsel {
            if !e.ist_schiff() || stufen.is_empty() {
                return Err(format!("Technikregeln: {e} braucht mindestens eine Antriebsstufe und muss ein Schiff sein"));
            }
            for s in stufen {
                if !ANTRIEBE.contains(&s.antrieb) || !ANTRIEBE.contains(&s.forschung) || s.stufe == 0
                    || s.stufe > crate::regeln::MAX_FORSCHUNG || s.tempo <= 0 || !(0..=1_000_000).contains(&s.verbrauch)
                {
                    return Err(format!("Technikregeln: ungültige Antriebsstufe für {e}"));
                }
            }
        }
        if t.plasma_je_stufe.iter().any(|(g, f)| !matches!(g, Gut::Erz | Gut::Kristall | Gut::Deuterium) || !f.is_finite() || !(0.0..=0.1).contains(f))
            || t.plasmatechnik.faktor < 1.0 || t.plasmatechnik.fp <= 0.0
        {
            return Err("Technikregeln: ungültige Plasmatechnik".into());
        }
        if t.fusion_volk.values().any(|f| !f.is_finite() || !(0.5..=2.0).contains(f)) {
            return Err("Technikregeln: Fusionsfaktor je Volk muss zwischen 0,5 und 2 liegen".into());
        }
        Ok(t)
    }

    /// Wirksame Antriebsstufe eines Schiffs für einen Forschungsstand (None = Grundantrieb laut Einheitenregel).
    pub fn stufe(&self, e: Einheit, forschung: &[u8; FORSCHUNGEN]) -> Option<&Antriebsstufe> {
        self.antriebswechsel.get(&e)?.iter().filter(|s| forschung[s.forschung.idx()] >= s.stufe).last()
    }
}

impl Regelwerk {
    /// Antrieb, Grundtempo und Verbrauch eines Schiffs für einen Forschungsstand. Ohne Technik v1 genau die Werte
    /// der Einheitenregel.
    pub fn antrieb_wirksam(&self, e: Einheit, forschung: &[u8; FORSCHUNGEN]) -> (Option<Forschung>, i64, i64) {
        let er: &EinheitRegel = self.einh(e);
        match self.technik.as_ref().and_then(|t| t.stufe(e, forschung)) {
            Some(s) => (Some(s.antrieb), s.tempo, s.verbrauch),
            None => (er.antrieb, er.tempo, er.verbrauch),
        }
    }
    /// Förderfaktor der Plasmatechnik für ein Gut (None ohne Technik v1, bei Stufe 0 oder für andere Güter).
    /// Energiefaktor der Fusionskraftwerke eines Volkes: mit Technik v1 aus `fusion_volk`, sonst der Volksfaktor `energie`.
    pub fn fusion_volk_faktor(&self, volk: Volk) -> f64 {
        self.technik.as_ref().and_then(|t| t.fusion_volk.get(&volk).copied()).unwrap_or(self.volk(volk).energie)
    }

    pub fn plasma_faktor(&self, gut: Gut, stufe: u8) -> Option<f64> {
        let je = *self.technik.as_ref()?.plasma_je_stufe.get(&gut)?;
        (stufe > 0).then_some(1.0 + je * stufe as f64)
    }
}

/// Regeltext-Abschnitt „antriebe“ (nur mit Technik v1).
pub fn regeltext(r: &Regelwerk, t: &TechnikRegeln) -> String {
    let mut s = String::from(
        "Neue Welten (Technik v1): Manche Schiffe wechseln durch Forschung den Antrieb. Ab der genannten Stufe gelten neuer Antrieb, \
neues Grundtempo und neuer Verbrauch; das Tempo im Flug ist Grundtempo × (1 + Tempobonus des neuen Antriebs × dessen Stufe). \
Von mehreren erreichten Stufen gilt die letzte. Die Flugplanung rechnet mit dem Stand beim Start.\n\
Schiff | ab Forschung | Antrieb | Grundtempo | Verbrauch\n",
    );
    for (e, stufen) in &t.antriebswechsel {
        let er = r.einh(*e);
        s.push_str(&format!("{e} | Start | {} | {} | {}\n", er.antrieb.map(|a| a.name()).unwrap_or("-"), er.tempo, er.verbrauch));
        for st in stufen {
            s.push_str(&format!("{e} | {} {} | {} | {} | {}\n", st.forschung, st.stufe, st.antrieb, st.tempo, st.verbrauch));
        }
    }
    let p = &t.plasmatechnik;
    let je = |g: Gut| t.plasma_je_stufe.get(&g).map(|f| format!("{}", (f * 10000.0).round() / 100.0)).unwrap_or_else(|| "0".into());
    s.push_str(&format!(
        "Plasmatechnik (ab Stufe {}, Labor {}, braucht {}): je Stufe +{} Prozent Erz, +{} Prozent Kristall und +{} Prozent Deuterium \
aus Erzminen, Kristallminen und Deuteriumsynthesizern (auf die Förderung nach Besetzung, Stabilität und Energie). Kosten der Stufe n: {} × {}^(n-1), \
Forschungspunkte {} × {}^(n-1).{}",
        p.ab_stufe,
        p.labor,
        t.plasmatechnik_braucht.iter().map(|(f, n)| format!("{f} {n}")).collect::<Vec<_>>().join(", "),
        je(Gut::Erz),
        je(Gut::Kristall),
        je(Gut::Deuterium),
        p.kosten.iter().map(|(g, m)| format!("{m} {g}")).collect::<Vec<_>>().join(", "),
        p.faktor,
        p.fp,
        p.faktor,
        if t.fusion_volk.is_empty() { String::new() } else {
            format!(" Fusionskraftwerke rechnen je Volk mit eigenem Energiefaktor statt des allgemeinen: {}.",
                t.fusion_volk.iter().map(|(v, f)| format!("{} × {}", v.name(), f)).collect::<Vec<_>>().join(", "))
        }
    ));
    s
}

impl Welt {
    pub fn technik_v1(&self) -> bool {
        self.ausbau.technik.is_some()
    }
    /// Zahl der Forschungen dieser Welt (19 bis Technik v1, 20 mit Plasmatechnik).
    pub fn forschungszahl(&self) -> usize {
        if self.technik_v1() { FORSCHUNGEN } else { crate::snapshot_layout::FORSCHUNGEN_V6 }
    }
    /// Die Forschungen dieser Welt in fester Reihenfolge.
    pub fn forschungen(&self) -> &'static [Forschung] {
        &Forschung::ALLE[..self.forschungszahl()]
    }
    /// Technik v1 einschalten: nur bei Spielbeginn einer Generation-12-Welt (Server, lokale Online-Partie, Messläufe).
    pub fn technik_aktivieren(&mut self) -> Result<(), String> {
        // Kampf- und Doktrinwerte neuer Welten aus regeln/flottenausbau.json wie in quellen::neue_welt (ConfigWelt), nicht
        // aus der eingefrorenen Doktrin v1 – sonst erreichten Änderungen der Flottendatei den Server nicht (Konter-Balance,
        // Karl 24.10.2026). Nur bei Spielbeginn, wie Technik v1 selbst.
        if !self.technik_v1() && self.zeit == 0 && self.ausbau.version > 0 {
            crate::quellen::flotten_uebernehmen(self, crate::flottenausbau::FLOTTEN_JSON)?;
        }
        self.technik_aktivieren_aus(REGELN_RON, crate::kernwerte::REGELN_RON)
    }
    /// Wie [`Welt::technik_aktivieren`], mit gegebenen Regeltexten (ConfigWelt prüft so ungespeicherte Änderungen).
    pub fn technik_aktivieren_aus(&mut self, technik_ron: &str, kernwerte_ron: &str) -> Result<(), String> {
        if self.technik_v1() {
            return Ok(());
        }
        if self.ausbau.version == 0 {
            return Err("Technik v1 benötigt Generation 12 (ausbau_aktivieren)".into());
        }
        if self.zeit != 0 {
            return Err("Technik v1 gilt nur für neue Welten (Spielzeit 0)".into());
        }
        let mut t = TechnikRegeln::laden(technik_ron)?;
        t.kernwerte = Some(crate::kernwerte::KernWerte::laden(kernwerte_ron)?);
        let t = Arc::new(t);
        let r = Arc::make_mut(&mut self.regeln);
        r.forschung.insert(Forschung::Plasmatechnik, t.plasmatechnik.clone());
        for (e, text) in &t.wirkung {
            if let Some(er) = r.einheiten.get_mut(e) { er.wirkung = text.clone(); }
            if let Some(p) = r.flottendoktrin.get_mut(e) { p.rolle = text.clone(); }
        }
        // Kernwerte: Industrie-Rezepte und Expeditionsschiffe neuer Welten (vorher fest in industrie_aktivieren und
        // ausbau_aktivieren gesetzt; die Standardwerte sind genau diese).
        let kw = t.kernwerte.as_ref().expect("eben gesetzt");
        if self.ausbau.industrie_version >= 1 {
            let i = &kw.industrie;
            if let Some(g) = r.gebaeude.get_mut(&Gebaeude::Kaserne) { g.ab_stufe = i.kaserne_ab_stufe; }
            if let Some(e) = r.einheiten.get_mut(&Einheit::Kolonieschiff) {
                e.ab_stufe = i.kolonieschiff_ab_stufe;
                e.werft = i.kolonieschiff_werft;
                e.braucht = i.kolonieschiff_braucht.clone();
                e.kosten = i.kolonieschiff_kosten.clone();
            }
            if let Some(p) = r.flottendoktrin.get_mut(&Einheit::Kolonieschiff) { p.voraussetzungen = i.kolonieschiff_forschung.clone(); }
            r.wirtschaft.bauteile.insert(Gut::Habitatmodul, i.habitatmodul_kosten.clone());
        }
        for (e, regel) in [(Einheit::Pathfinder, &kw.expedition.pathfinder), (Einheit::Bergungsschiff, &kw.expedition.bergungsschiff)] {
            let text = t.wirkung.get(&e).cloned().unwrap_or_else(|| regel.wirkung.clone());
            if r.einheiten.contains_key(&e) {
                let mut regel = regel.clone();
                regel.wirkung = text.clone();
                r.einheiten.insert(e, regel);
            }
            if let Some(p) = r.flottendoktrin.get_mut(&e) {
                p.rolle = text;
                p.voraussetzungen = kw.expedition.forschung.clone();
                if e == Einheit::Bergungsschiff { p.personenplaetze = kw.expedition.bergungsschiff_personenplaetze; }
            }
        }
        // Terraformer (0.4.7): nur Welten mit Monden (Hülle STERNE14) und eingeschaltetem Kernwert terraform.aktiv.
        if kw.terraform.aktiv && r.monde.is_some() {
            r.terraformer_aktivieren();
        }
        r.cache_fuellen();
        r.technik = Some(t.clone());
        {
            use sha2::{Digest, Sha256};
            let mut h = Sha256::new();
            h.update(r.hash.as_bytes());
            h.update(b"technik-v1");
            h.update(technik_ron.replace("\r\n", "\n").as_bytes());
            h.update(b"kernwerte-v1");
            h.update(kernwerte_ron.replace("\r\n", "\n").as_bytes());
            r.hash = h.finalize().iter().map(|b| format!("{b:02x}")).collect();
        }
        self.ausbau.technik = Some(t);
        Ok(())
    }
    /// Fehlende Voraussetzungen der Plasmatechnik (leer, wenn erfüllt oder ohne Technik v1).
    pub fn plasmatechnik_fehlt(&self, sid: SpielerId) -> Vec<String> {
        let Some(t) = &self.regeln.technik else { return vec![] };
        let sp = &self.spieler[sid as usize];
        t.plasmatechnik_braucht.iter().filter(|(f, n)| sp.forschung[f.idx()] < **n).map(|(f, n)| format!("{f} Stufe {n}")).collect()
    }
    /// Antriebsdaten eines Schiffs für die Sicht: wirksamer Antrieb, Verbrauch und alle Wechselstufen.
    pub fn antriebsdaten(&self, sid: SpielerId, e: Einheit) -> serde_json::Value {
        let fo = &self.spieler[sid as usize].forschung;
        let (antrieb, tempo, verbrauch) = self.regeln.antrieb_wirksam(e, fo);
        let wechsel: Vec<serde_json::Value> = self
            .regeln
            .technik
            .as_ref()
            .and_then(|t| t.antriebswechsel.get(&e))
            .map(|v| {
                v.iter()
                    .map(|s| serde_json::json!({"forschung": s.forschung, "stufe": s.stufe, "antrieb": s.antrieb, "grundtempo": s.tempo,
                        "verbrauch": s.verbrauch, "erreicht": fo[s.forschung.idx()] >= s.stufe}))
                    .collect()
            })
            .unwrap_or_default();
        serde_json::json!({"antrieb": antrieb, "grundtempo": tempo, "verbrauch": verbrauch, "antriebswechsel": wechsel})
    }
}
