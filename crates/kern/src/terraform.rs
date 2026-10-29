//! Terraforming mit Terraformer-Schiffen (0.4.7, Karl 24.10.2026): „für das Terraforming von Planeten das Schiff
//! ‚Terraformer‘ einführen (mit jeder Forschungsstufe braucht man weniger Terraformerschiffe, um einen Planeten zu
//! terraformen, dass er bewohnbar ist; die Felder können von der Grundgröße maximal 40 Felder vergrößert werden …)“.
//!
//! Nur Welten mit dem Terraformer im Regelwerk (neue Welten mit Monden und Technik v1, Kernwert `terraform.aktiv`;
//! Hülle STERNE14). Der Stand je Koordinate liegt in `ausbau.terraform` (JSON-Erweiterung, leer = nicht geschrieben),
//! das Zielvolk eines Fluges in `ausbau.terraform_fluege`. Alle Rechnungen in Ganzzahlen.
//!
//! - g = Klimalücke des Platzes für das Volk bei Terraforming 0 in Promille (0 = Klimaoptimum, 1000 = ungünstigster
//!   Platz dieses Volkes laut Klimatabelle; andere Systemgrößen über die zwölf Referenzbahnen).
//! - Feldgrenze = felder_max − round(felder_abzug × g / 1000) (40 im Optimum, 30 auf dem ungünstigsten Platz).
//! - W = Terraformer für das volle Projekt auf dem ungünstigsten Platz bei Forschungsstufe L (Kernwert `schiffe`).
//! - Klimaschiffe K = ⌈klima_anteil × W × g⌉, Feldschiffe F = ⌈(1 − klima_anteil) × W × Feldgrenze / felder_bezug⌉.
//! - Ankunft mit n Schiffen: zuerst das Klima (+1000/K Promille je Schiff), der Rest formt die Kruste (+Feldgrenze/F
//!   Felder je Schiff), jeweils gedeckelt. Verbraucht werden nur die nötigen Schiffe; der Rest fliegt heim.
//! - Wirkung: Klimafaktor des Volkes = Forschungswert + (Optimum − Forschungswert) × Klima/1000; bei vollem Klima entfällt
//!   die Siedelsperre (Freigabe) des Platzes für dieses Volk. Höchstfelder nach vollem Terraforming = Grundgröße + Feldgrenze.
use crate::habitabilitaet::{bahn, optimum, Eignung, OPTIMUM, WACHSTUM};
use crate::typen::*;
use crate::Welt;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

/// Stand des Terraformings an einer Koordinate.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct TerraformStand {
    /// Volk, für das das Klima geformt wird (ein Wechsel beginnt das Klima von vorn, Felder bleiben).
    pub volk: Volk,
    /// Geschlossener Anteil der Klimalücke für `volk` in Promille (1000 = Klima optimal angepasst).
    pub klima_promille: u16,
    /// Geformte Felder über der Grundgröße.
    pub felder: u16,
}

/// Bedarf eines Platzes für ein Volk bei einer Forschungsstufe (mit dem bisherigen Stand).
#[derive(Clone, Copy, Debug, Serialize, PartialEq, Eq)]
pub struct TerraformBedarf {
    pub volk: Volk,
    pub stufe: u8,
    /// Klimalücke bei Terraforming 0 in Promille (1000 = ungünstigster Platz dieses Volkes).
    pub luecke_promille: i64,
    pub feldgrenze: u16,
    /// Terraformer für das volle Projekt auf dem ungünstigsten Platz bei dieser Stufe.
    pub projekt: i64,
    /// Terraformer für das ganze Klima bzw. alle Felder dieses Platzes (ohne bisherigen Stand).
    pub schiffe_klima_voll: i64,
    pub schiffe_felder_voll: i64,
    /// Stand (für dieses Volk; ein anderes Volk beginnt das Klima von vorn).
    pub klima_promille: u16,
    pub felder: u16,
    /// Noch nötige Terraformer.
    pub schiffe_klima: i64,
    pub schiffe_felder: i64,
}

impl TerraformBedarf {
    pub fn schiffe_voll(&self) -> i64 {
        self.schiffe_klima + self.schiffe_felder
    }
    /// Angepasstes Klima in Prozent (100, wenn der Platz keine Klimalücke hat).
    pub fn klima_prozent(&self) -> i64 {
        if self.schiffe_klima_voll == 0 { 100 } else { self.klima_promille as i64 / 10 }
    }
}

fn aufrunden(zaehler: i128, nenner: i128) -> i64 {
    if nenner <= 0 || zaehler <= 0 { 0 } else { ((zaehler + nenner - 1) / nenner) as i64 }
}

impl Welt {
    /// Gehört der Terraformer zum Regelwerk dieser Welt (Hülle STERNE14)?
    pub fn terraformer_welt(&self) -> bool {
        self.regeln.einheiten.contains_key(&Einheit::Terraformer)
    }

    /// Länge der Schiffsliste in Spionageberichten dieser Welt: 35 mit Terraformer, sonst wie bisher 34.
    pub fn bericht_schiffe(&self) -> usize {
        if self.terraformer_welt() { SCHIFFE } else { SCHIFFE - 1 }
    }

    /// Klimalücke eines Platzes für ein Volk bei Terraforming 0 in Promille: 0 im Klimaoptimum, 1000 auf dem
    /// ungünstigsten Platz dieses Volkes (Klimatabelle der Welt).
    pub fn klimaluecke(&self, k: Koord, volk: Volk) -> i64 {
        let i = bahn(k.position, self.regeln.welt.plaetze_je_system);
        let (tabelle, opt) = if self.regeln.ablauf >= 1 {
            (self.regeln.kw().klima.tabelle(volk), optimum(self.regeln.ablauf))
        } else {
            (WACHSTUM[volk.idx()], OPTIMUM)
        };
        let schlecht = tabelle.iter().copied().min().unwrap_or(opt);
        if opt <= schlecht {
            return 0;
        }
        let spanne = (opt - schlecht) as i128;
        (((opt - tabelle[i]).max(0) as i128 * 1000 + spanne / 2) / spanne).clamp(0, 1000) as i64
    }

    /// Bedarf an Terraformern für Platz `k`, Volk `volk` bei Terraforming-Stufe `stufe`, mit dem bisherigen Stand.
    pub fn terraform_bedarf(&self, k: Koord, volk: Volk, stufe: u8) -> TerraformBedarf {
        let t = &self.regeln.kw().terraform;
        let g = self.klimaluecke(k, volk);
        let feldgrenze = t.felder_max.saturating_sub(((t.felder_abzug as i64 * g + 500) / 1000) as u16);
        let w = t.schiffe_bei(stufe);
        let klima_voll = aufrunden(t.klima_anteil_promille as i128 * w as i128 * g as i128, 1_000_000);
        let felder_voll = aufrunden((1000 - t.klima_anteil_promille) as i128 * w as i128 * feldgrenze as i128, 1000 * t.felder_bezug.max(1) as i128);
        let (klima, felder) = match self.ausbau.terraform.get(&k) {
            Some(s) if s.volk == volk => (s.klima_promille.min(1000), s.felder),
            Some(s) => (0, s.felder),
            None => (0, 0),
        };
        // Bisheriger Stand in eingesetzten Schiffen gerechnet (aufgerundet): so kostet ein Projekt in Teilflügen genau so
        // viele Schiffe wie in einem Flug.
        // Ein nicht vollständiger Stand braucht immer noch mindestens ein Schiff (auch nach einem Stufenwechsel).
        let klima_da = if klima >= 1000 { klima_voll } else { aufrunden(klima as i128 * klima_voll as i128, 1000).min((klima_voll - 1).max(0)) };
        let felder_da = if felder >= feldgrenze { felder_voll } else { aufrunden(felder as i128 * felder_voll as i128, feldgrenze as i128).min((felder_voll - 1).max(0)) };
        let (schiffe_klima, schiffe_felder) = (klima_voll - klima_da, felder_voll - felder_da);
        TerraformBedarf {
            volk, stufe, luecke_promille: g, feldgrenze, projekt: w, schiffe_klima_voll: klima_voll, schiffe_felder_voll: felder_voll,
            klima_promille: klima, felder, schiffe_klima, schiffe_felder,
        }
    }

    /// Geformte Felder an einer Koordinate (0 ohne Terraforming).
    pub fn terraform_felder(&self, k: Koord) -> u16 {
        if self.ausbau.terraform.is_empty() { 0 } else { self.ausbau.terraform.get(&k).map(|s| s.felder).unwrap_or(0) }
    }

    /// Klimaeignung mit dem Terraforming-Stand der Koordinate (unverändert ohne Stand für dieses Volk).
    pub(crate) fn terraform_eignung(&self, k: Koord, mut e: Eignung) -> Eignung {
        let Some(s) = self.ausbau.terraform.get(&k) else { return e };
        if s.volk != e.volk || s.klima_promille == 0 {
            return e;
        }
        let opt = optimum(self.regeln.ablauf);
        let kp = s.klima_promille.min(1000) as i64;
        if !e.bewohnbar {
            // Erst das voll angepasste Klima hebt die Siedelsperre auf.
            if kp >= 1000 {
                e.bewohnbar = true;
                e.terraforming_noetig = 0;
                e.wachstum_promille = opt;
                e.klimaverlust_promille_je_stunde = 0;
            }
            return e;
        }
        if e.wachstum_promille < opt {
            e.wachstum_promille += (opt - e.wachstum_promille) * kp / 1000;
        }
        e
    }

    /// Prüft einen Terraforming-Flug (Mission `terraformen`) und liefert das Zielvolk.
    pub(crate) fn terraform_flug_pruefen(&self, sid: SpielerId, ziel: Koord, schiffe: &[i64; SCHIFFE], volk: Option<Volk>) -> Result<Volk, String> {
        if !self.terraformer_welt() {
            return Err("Terraforming mit Schiffen gibt es in dieser Welt nicht".into());
        }
        if schiffe[Einheit::Terraformer.idx()] <= 0 {
            return Err("Terraformen braucht Terraformer – in der Werft bauen (ab Stufe IV, Terraforming 1, Impulsantrieb 3)".into());
        }
        if (0..SCHIFFE).any(|e| e != Einheit::Terraformer.idx() && schiffe[e] > 0) {
            return Err("Ein Terraforming-Flug besteht nur aus Terraformern; Begleitschiffe getrennt schicken".into());
        }
        self.terraform_ziel_pruefen(sid, ziel)?;
        Ok(volk.unwrap_or(self.spieler[sid as usize].volk))
    }

    /// Ziel eines Terraformings: eigener Planet oder freier Platz in einem erfassten System.
    fn terraform_ziel_pruefen(&self, sid: SpielerId, ziel: Koord) -> Result<Option<usize>, String> {
        if ziel.position == 0 || self.platz(ziel).is_none() {
            return Err(format!("{ziel} ist kein Planetenplatz: Terraformer formen Planeten und freie Plätze (Position 1 bis {})", self.regeln.welt.plaetze_je_system));
        }
        if let Some(pid) = self.spieler[sid as usize].planeten.iter().map(|p| *p as usize).find(|p| self.planeten[*p].koord == ziel) {
            return Ok(Some(pid));
        }
        if self.aufklaerungsregeln() && !self.system_erfasst(sid, ziel) {
            return Err("Sonnensystem zuerst mit einer Systemsonde kartieren (Mission system_erkunden)".into());
        }
        if self.belegung.contains_key(&ziel) {
            return Err(format!("{ziel} ist besiedelt: Terraformen geht nur auf eigenen Planeten oder freien Plätzen"));
        }
        Ok(None)
    }

    /// Ankunft eines Terraforming-Flugs.
    pub(crate) fn terraform_ankunft(&mut self, fid: FlottenId) {
        let Some(f) = self.flotten.get(&fid).cloned() else { return };
        let sid = f.besitzer;
        let k = f.ziel;
        let volk = self.ausbau.terraform_fluege.remove(&fid).unwrap_or(self.spieler[sid as usize].volk);
        let mut eigener = None;
        if let Some(zp) = self.belegung.get(&k).map(|p| *p as usize) {
            if self.planeten[zp].besitzer != sid {
                self.umkehren(fid, "der Platz wurde inzwischen von einem anderen Reich besiedelt");
                return;
            }
            if self.blockiert_fuer(zp, sid) {
                self.umkehren(fid, "der Orbit ist blockiert");
                return;
            }
            eigener = Some(zp);
        }
        let n = f.schiffe[Einheit::Terraformer.idx()];
        let stufe = self.spieler[sid as usize].forschung[Forschung::Terraforming.idx()];
        let b = self.terraform_bedarf(k, volk, stufe);
        let mut s = self.ausbau.terraform.get(&k).cloned().unwrap_or(TerraformStand { volk, klima_promille: 0, felder: 0 });
        if s.volk != volk {
            s.volk = volk;
            s.klima_promille = 0;
        }
        let bewohnbar_vorher = self.terraform_eignung(k, crate::habitabilitaet::eignung_r(&self.regeln, volk, k.position, stufe)).bewohnbar;
        // Fortschritt kumuliert über alle eingesetzten Schiffe (bisheriger Stand in Schiffen plus neue), gedeckelt.
        let klima_n = n.min(b.schiffe_klima);
        if klima_n > 0 {
            let da = b.schiffe_klima_voll - b.schiffe_klima;
            s.klima_promille = if klima_n == b.schiffe_klima { 1000 } else { ((da + klima_n) * 1000 / b.schiffe_klima_voll.max(1)).clamp(s.klima_promille as i64, 999) as u16 };
        }
        let felder_n = (n - klima_n).min(b.schiffe_felder);
        let felder_vorher = s.felder;
        if felder_n > 0 {
            let da = b.schiffe_felder_voll - b.schiffe_felder;
            s.felder = if felder_n == b.schiffe_felder {
                s.felder.max(b.feldgrenze)
            } else {
                ((da + felder_n) * b.feldgrenze as i64 / b.schiffe_felder_voll.max(1)).clamp(s.felder as i64, (b.feldgrenze as i64 - 1).max(s.felder as i64)) as u16
            };
        }
        let verbraucht = klima_n + felder_n;
        let rest = n - verbraucht;
        if verbraucht > 0 {
            self.ausbau.terraform.insert(k, s.clone());
        }
        let klima_prozent = if b.schiffe_klima_voll == 0 { 100 } else { s.klima_promille as i64 / 10 };
        let mut text = if verbraucht == 0 {
            format!("Terraforming [{k}]: nichts mehr zu formen – Klima ({}) und Felder ({} von {}) sind bereits vollständig; {n} Terraformer kehren heim",
                volk_gross(volk.name()), s.felder, b.feldgrenze)
        } else {
            format!("Terraforming [{k}]: {verbraucht} Terraformer eingesetzt – Klima für {} zu {klima_prozent} % angepasst, +{} Felder ({} von {})",
                volk_gross(volk.name()), s.felder - felder_vorher, s.felder, b.feldgrenze)
        };
        if verbraucht > 0 {
            let jetzt = self.terraform_eignung(k, crate::habitabilitaet::eignung_r(&self.regeln, volk, k.position, stufe)).bewohnbar;
            if jetzt && !bewohnbar_vorher {
                text.push_str(&format!("; der Platz ist jetzt für {} bewohnbar", volk.name()));
            }
            if rest > 0 {
                text.push_str(&format!("; {rest} Terraformer kehren heim"));
            }
        }
        if rest == 0 {
            // Alle Schiffe verbraucht: Der beim Start mitbezahlte Rückflug-Treibstoff (eigenes Rückflugtempo oder gleiche
            // Strecke) geht an den Startplaneten zurück (Karl 24.10.2026: „es darf kein Punkt im Spiel untergehen“).
            let rueck = self.ausbau.rueckflugplaene.get(&fid).map(|p| p.treibstoff)
                .or_else(|| self.ausbau.fluganzeigen.get(&fid).and_then(|v| v["treibstoff_hin"].as_f64()).map(|t| (t * M as f64).round() as i64))
                .unwrap_or(0);
            let sp = f.start as usize;
            if rueck > 0 && self.planeten.get(sp).is_some_and(|p| p.besitzer == sid) {
                self.abrechnen(sp);
                self.planeten[sp].bestand[Gut::Deuterium.idx()] += rueck;
                text.push_str(&format!("; kein Rückflug – {} Deuterium Treibstoff an {} erstattet", ganz(rueck), self.planeten[sp].koord));
            }
        }
        self.vorfall(sid, "terraforming", text);
        self.fachereignis("terraforming", sid, json!({"fleet": fid, "target": k, "volk": volk, "eingesetzt": verbraucht, "zurueck": rest,
            "klima_promille": s.klima_promille, "felder": s.felder, "feldgrenze": b.feldgrenze}));
        if let Some(fl) = self.flotten.get_mut(&fid) {
            fl.schiffe[Einheit::Terraformer.idx()] -= verbraucht;
        }
        if let Some(zp) = eigener {
            self.raten_neu(zp);
        }
        if rest > 0 {
            self.heimflug(fid);
        } else {
            self.flotten.remove(&fid);
        }
    }

    /// Terraforming eines eigenen Planeten für die Oberfläche (null ohne Terraformer im Regelwerk).
    pub fn terraform_planet_sicht(&self, pid: usize) -> Value {
        if !self.terraformer_welt() {
            return Value::Null;
        }
        let p = &self.planeten[pid];
        let sp = &self.spieler[p.besitzer as usize];
        let volk = self.ausbau.terraform.get(&p.koord).map(|s| s.volk).unwrap_or(sp.volk);
        let b = self.terraform_bedarf(p.koord, volk, sp.forschung[Forschung::Terraforming.idx()]);
        self.terraform_felder_json(&b)
    }

    fn terraform_felder_json(&self, b: &TerraformBedarf) -> Value {
        json!({"volk": b.volk, "stufe": b.stufe, "klima_prozent": b.klima_prozent(), "felder": b.felder, "feldgrenze": b.feldgrenze,
            "schiffe_klima": b.schiffe_klima, "schiffe_felder": b.schiffe_felder, "schiffe_voll": b.schiffe_voll()})
    }

    /// Planungswerkzeug `terraformbedarf` (Eingabe `koord`, optional `volk`): Stand und noch nötige Terraformer für einen
    /// eigenen Planeten oder freien Platz eines erfassten Systems, dazu die Schiffszahlen der nächsten Forschungsstufen.
    pub fn terraformbedarf_werkzeug(&self, sid: SpielerId, abfrage: &Value) -> Result<Value, String> {
        if !self.terraformer_welt() {
            return Err("In dieser Welt gibt es keine Terraformer".into());
        }
        let text = abfrage.get("koord").or_else(|| abfrage.get("planet")).and_then(Value::as_str).ok_or("Feld koord fehlt (Sektor:System:Position)")?;
        let k: Koord = text.parse()?;
        let eigen = self.terraform_ziel_pruefen(sid, k)?;
        let sp = &self.spieler[sid as usize];
        let volk = match abfrage.get("volk").filter(|v| !v.is_null()) {
            Some(v) => serde_json::from_value::<Volk>(v.clone()).map_err(|_| "volk: aurelianer, krath, veyari oder syntheten".to_string())?,
            None => self.ausbau.terraform.get(&k).map(|s| s.volk).unwrap_or(sp.volk),
        };
        let stufe = sp.forschung[Forschung::Terraforming.idx()];
        let b = self.terraform_bedarf(k, volk, stufe);
        let mut v = self.terraform_felder_json(&b);
        let t = &self.regeln.kw().terraform;
        let mut naechste = serde_json::Map::new();
        for l in stufe.saturating_add(1)..=(t.schiffe.len().saturating_sub(1) as u8) {
            naechste.insert(l.to_string(), json!(self.terraform_bedarf(k, volk, l).schiffe_voll()));
        }
        let grund = match eigen {
            Some(pid) => Some(self.planeten[pid].felder),
            None => sp.erkundet.get(&k).map(|e| e.felder),
        };
        let eignung = self.planet_eignung(sid, k, volk);
        v["koord"] = json!(k.to_string());
        v["status"] = json!(if eigen.is_some() { "eigen" } else { "frei" });
        v["klimaluecke_promille"] = json!(b.luecke_promille);
        v["grundgroesse"] = json!(grund);
        // Endgröße nach vollem Terraforming = Grundgröße + Feldgrenze (Funktionsprüfung 25.10.2026: + Stand ergab „komplett 180“ statt 220).
        v["felder_hoechst"] = json!(grund.map(|g| g as i64 + b.feldgrenze as i64));
        v["bewohnbar"] = json!(eignung.bewohnbar);
        v["klimafaktor_promille"] = json!(eignung.wachstum_promille);
        v["naechste_stufen"] = Value::Object(naechste);
        let mut hinweis = vec![
            "Zuerst formen die Terraformer das Klima, übrige Schiffe vergrößern die Kruste; nur die nötigen Schiffe werden verbraucht, der Rest fliegt heim.".to_string(),
            format!("Je Stufe Terraforming braucht ein Projekt weniger Schiffe (Stufe {stufe}: {} für den ungünstigsten Platz deines Volkes).", t.schiffe_bei(stufe)),
            format!("Höchstens +{} Felder über der Grundgröße; je größer die Klimalücke, desto weniger (hier {}).", t.felder_max, b.feldgrenze),
        ];
        if !eignung.bewohnbar {
            hinweis.push(format!("Für {} gesperrt: Terraforming {} oder volles Klima durch Terraformer hebt die Sperre auf.", volk.name(), eignung.terraforming_noetig));
        }
        if b.schiffe_voll() == 0 {
            hinweis.push("Dieser Platz ist für dieses Volk vollständig geformt.".into());
        }
        v["hinweis"] = json!(hinweis);
        Ok(v)
    }

    /// Prüfung beim Laden.
    pub(crate) fn terraform_validieren(&self) -> Result<(), String> {
        if !self.terraformer_welt() && (!self.ausbau.terraform.is_empty() || !self.ausbau.terraform_fluege.is_empty()) {
            return Err("Terraforming-Stand ohne Terraformer im Regelwerk".into());
        }
        if self.ausbau.terraform.iter().any(|(k, s)| k.position == 0 || s.klima_promille > 1000 || s.felder > 1000) {
            return Err("Ungültiger Terraforming-Stand".into());
        }
        Ok(())
    }
}

/// Volksname für Meldungen mit großem Anfangsbuchstaben („Aurelianer“), wie die Oberfläche ihn zeigt.
fn volk_gross(s: &str) -> String { let mut c = s.chars(); c.next().map(|f| f.to_uppercase().collect::<String>() + c.as_str()).unwrap_or_default() }
