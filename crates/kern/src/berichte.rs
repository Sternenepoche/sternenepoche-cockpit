//! Berichtsdaten, die über die gespeicherten Bericht-Strukturen hinausgehen (Karl 22.10.2026: „Spionageberichte,
//! Kampfberichte und so was wirklich ultra gewissenhaft, korrekt, ohne Lücken“). Zusatzangaben liegen nur in
//! `Ausbau`-Nebenkarten (`scan_details`, `kampf_zusatz`), damit alte Spielstände bytegleich bleiben.
use crate::{typen::*, welt::Flottenzustand, Welt};
use serde_json::{json, Value};

/// Promille als deutscher Prozenttext mit einer Nachkommastelle („12,5“).
pub(crate) fn promille_text(p: i64) -> String {
    format!("{},{}", p / 10, (p % 10).abs())
}

/// Fracht als Text („1.200 erz, 300 kristall“), nur Güter ab einer ganzen Einheit; leer = „nichts“.
pub(crate) fn fracht_text(ladung: &[i64; GUETER]) -> String {
    let teile: Vec<String> = Gut::ALLE
        .iter()
        .filter(|g| ladung[g.idx()] >= M)
        .map(|g| format!("{} {}", crate::typen::ganz(ladung[g.idx()]), g))
        .collect();
    if teile.is_empty() { "nichts".into() } else { teile.join(", ") }
}

impl Welt {
    pub(crate) fn scanarchiv_begrenzen(&mut self, sid: SpielerId, art: &str, max: usize) {
        let prefix = format!("{art}{sid}:");
        let mut alt: Vec<_> = self.ausbau.scan_details.iter().filter(|(k,_)|k.starts_with(&prefix))
            .map(|(k,v)|(v["zeit"].as_i64().unwrap_or(0),k.clone())).collect();
        alt.sort();
        let entfernen = alt.len().saturating_sub(max);
        for (_,k) in alt.into_iter().take(entfernen) { self.ausbau.scan_details.remove(&k); }
    }
    pub(crate) fn scanarchiv_merken(&mut self, sid: SpielerId, ziel: Koord) {
        let sp = &self.spieler[sid as usize];
        let bericht = if let Some(b) = sp.berichte.iter().find(|b|b.ziel==ziel) { self.bericht_sicht(sid,b) }
            else if let Some(e) = sp.erkundet.get(&ziel) { self.erkundung_sicht(sid,ziel,e) }
            else { return; };
        self.ausbau.scan_details.insert(format!("archiv:{sid}:{}:{ziel}",self.zeit),bericht);
        self.scanarchiv_begrenzen(sid,"archiv:",300);
    }
    pub fn systemberichte_sicht(&self, sid: SpielerId) -> Value {
        let prefix=format!("system:{sid}:");
        let mut rows:Vec<Value>=self.ausbau.scan_details.iter().filter(|(k,_)|k.starts_with(&prefix))
            .map(|(_,v)|{let mut v=v.clone();v.as_object_mut().unwrap().remove("spieler_id");v}).collect();
        for (&(owner,sektor,system),b) in &self.aufklaerung.systemberichte {
            let ziel=format!("{sektor}:{system}:0");
            if owner==sid && !rows.iter().any(|r|r["ziel"]==ziel && r["zeit"]==b.zeit) {
                rows.push(json!({"art":"systemscan","zeit":b.zeit,"ziel":ziel,"nebel":b.nebel,
                    "asteroidenguertel":b.guertel,"historisch":true}));
            }
        }
        rows.sort_by_key(|r|std::cmp::Reverse(r["zeit"].as_i64().unwrap_or(0)));
        json!(rows)
    }
    /// Flotten im Orbit eines Platzes, wie eine Sonde sie sieht: Halteflotten, Blockaden, Belagerer. Flotten, die
    /// am Mond des Platzes stehen, gehören nicht dazu (der Mond hat eigene Berichte).
    pub(crate) fn orbit_lage(&self, betrachter: SpielerId, k: Koord) -> Value {
        let mut out = Vec::new();
        for f in self.flotten.values() {
            if f.ziel != k || f.zustand != Flottenzustand::ImOrbit || self.monde.flottenziel.contains_key(&f.id) {
                continue;
            }
            let beziehung = if f.besitzer == betrachter {
                "eigen"
            } else if self.verbuendet(f.besitzer, betrachter) {
                "verbuendet"
            } else {
                "fremd"
            };
            out.push(json!({
                "flotte": f.id,
                "spieler": self.spieler[f.besitzer as usize].name,
                "beziehung": beziehung,
                "mission": f.mission.name(),
                "schiffe": crate::sicht::einheiten(&f.schiffe, 0),
                "anzahl": f.schiffe.iter().sum::<i64>(),
            }));
        }
        json!(out)
    }

    /// Flotte kehrt ohne Wirkung um – mit Meldung und Grund statt still (Bericht „ohne Lücken“).
    pub(crate) fn umkehren(&mut self, fid: FlottenId, grund: &str) {
        if let Some(f) = self.flotten.get(&fid) {
            let (besitzer, mission, ziel) = (f.besitzer, f.mission, f.ziel);
            self.vorfall(besitzer, "flotte", format!("Flotte {fid} ({mission}) kehrt bei {ziel} ohne Wirkung um: {grund}"));
        }
        self.heimflug(fid);
    }

    /// Bodenkampfberichte begrenzen: höchstens 60 je Planet (15 Spielstunden Belagerung bei 4 Runden je Stunde) und
    /// 600 insgesamt – vorher schnitt eine einzige globale Grenze von 200 laufende Belagerungen ab.
    pub(crate) fn bodenberichte_kappen(&mut self) {
        let b = &mut self.ausbau.boden.berichte;
        if let Some(planet) = b.last().and_then(|x| x["planet"].as_str()).map(str::to_string) {
            while b.iter().filter(|x| x["planet"] == planet.as_str()).count() > 60 {
                let i = b.iter().position(|x| x["planet"] == planet.as_str()).unwrap();
                b.remove(i);
            }
        }
        while b.len() > 600 {
            b.remove(0);
        }
    }

    /// Kurzer Satz über die Lage im Orbit für Ereignistexte.
    pub(crate) fn orbit_satz(lage: &Value) -> String {
        let n = lage.as_array().map_or(0, |a| a.len());
        match n {
            0 => "keine Flotten im Orbit".into(),
            1 => "1 Flotte im Orbit".into(),
            n => format!("{n} Flotten im Orbit"),
        }
    }

    /// Spionage- und Erkundungsberichte eines Spielers, neueste zuerst; je Platz zählt der neuere Bericht.
    pub(crate) fn berichte_sicht(&self, sid: SpielerId) -> Value {
        let sp = &self.spieler[sid as usize];
        let mut alle: Vec<(SimZeit, Value)> = sp.berichte.iter().map(|b| (b.zeit, self.bericht_sicht(sid, b))).collect();
        for (k, e) in &sp.erkundet {
            if !sp.berichte.iter().any(|b| b.ziel == *k && b.zeit >= e.zeit) {
                alle.push((e.zeit, self.erkundung_sicht(sid, *k, e)));
            }
        }
        let prefix=format!("archiv:{sid}:");
        for (_,v) in self.ausbau.scan_details.iter().filter(|(k,_)|k.starts_with(&prefix)) {
            if !alle.iter().any(|(_,r)|r["ziel"]==v["ziel"] && r["zeit"]==v["zeit"]) {
                let mut row=v.clone();let zeit=row["zeit"].as_i64().unwrap_or(0);
                row["alter_stunden"]=json!((self.zeit-zeit)/STUNDE);alle.push((zeit,row));
            }
        }
        alle.sort_by(|a, b| b.0.cmp(&a.0));
        json!(alle.into_iter().take(300).map(|(_, v)| v).collect::<Vec<_>>())
    }

    /// Erkundungsbericht eines freien Platzes in der Form der Spionageberichte (`berichte`, Art „erkundung“).
    pub(crate) fn erkundung_sicht(&self, sid: SpielerId, k: Koord, e: &crate::welt::Erkundung) -> Value {
        let zusatz = self
            .ausbau
            .scan_details
            .get(&format!("{sid}:{k}"))
            .filter(|v| v["zeit"] == e.zeit && v["art"] == "erkundung");
        json!({
            "art": "erkundung",
            "ziel": k.to_string(),
            "zeit": e.zeit,
            "besitzer": Value::Null,
            "frei": true,
            "felder": e.felder,
            "zone": e.zone.name(),
            "reich_erz": e.reich_erz,
            "reich_kristall": e.reich_kristall,
            "nebel": e.nebel,
            "sonden": zusatz.and_then(|v| v.get("sonden")).cloned().unwrap_or(Value::Null),
            "orbit": zusatz.and_then(|v| v.get("orbit")).cloned().unwrap_or(json!([])),
            "alter_stunden": (self.zeit - e.zeit) / STUNDE,
            "historisch": true,
            "aufklaerung": self.planetenwissen(sid, k),
        })
    }
}
