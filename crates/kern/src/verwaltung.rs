//! Reichsverwaltung aus dem Reparaturstand (F:) auf den Generation-12-Kern übertragen:
//! Kolonie aufgeben, manuelle Auftragspausen, Soforthandel und Sammel-Systemerkundung
//! mit zurückkehrenden Sonden. Neue Zustände liegen in der optionalen Ausbau-Erweiterung.
use crate::{sternkarte::Scan, typen::*, welt::*};
use serde_json::{json, Value};
use std::collections::BTreeMap;

/// Heimatwelt plus 14 Kolonien.
pub const MAX_PLANETEN: usize = 15;
const PAUSEN_ARTEN: [&str; 4] = ["bau", "werft", "bauteile", "forschung"];
/// Höchstlänge eines Planetennamens in Zeichen (Umlaute zählen einfach).
pub const PLANETENNAME_MAX: usize = 24;

/// Bereinigt einen Planetennamen: Leerraum zusammengefasst, Buchstaben jeder Schrift (auch Umlaute und ß),
/// Ziffern, Leerzeichen sowie `- ' .` erlaubt. Leer heißt Standardname.
pub fn planetenname_pruefen(roh: &str) -> Result<String, String> {
    let name = roh.split_whitespace().collect::<Vec<_>>().join(" ");
    if name.chars().count() > PLANETENNAME_MAX { return Err(format!("Planetenname höchstens {PLANETENNAME_MAX} Zeichen")); }
    if let Some(c) = name.chars().find(|c| !(c.is_alphanumeric() || matches!(c, ' ' | '-' | '\'' | '.' | '’'))) {
        return Err(format!("Zeichen „{c}“ ist im Planetennamen nicht erlaubt (Buchstaben, Ziffern, Leerzeichen, - ' .)"));
    }
    if !name.is_empty() && !name.chars().any(char::is_alphanumeric) { return Err("Planetenname braucht mindestens einen Buchstaben oder eine Ziffer".into()); }
    Ok(name)
}

impl Welt {
    /// Ein aufgegebener Datensatz bleibt als Grabstein erhalten (stabile Kennungen für
    /// Flotten im Flug), produziert aber nicht und gehört niemandem mehr.
    pub fn planet_besiedelt(&self, pid: usize) -> bool {
        self.planeten.get(pid).is_some_and(|p| self.belegung.get(&p.koord) == Some(&p.id))
    }

    /// Besiedelt und Besitzer noch im Spiel.
    pub fn planet_aktiv(&self, pid: usize) -> bool {
        self.planet_besiedelt(pid) && self.spieler_aktiv(self.planeten[pid].besitzer)
    }

    pub fn planetenkapazitaet(&self, sid: SpielerId) -> Value {
        let anzahl = self.spieler[sid as usize].planeten.len();
        let erlaubt = 1 + self.kolonien_erlaubt(sid);
        let reserviert=self.flotten.values().filter(|f|f.besitzer==sid && f.mission==Mission::Kolonisieren && f.zustand==Flottenzustand::Hinflug).map(|f|f.ziel).collect::<std::collections::BTreeSet<_>>().len();
        json!({"anzahl":anzahl,"erlaubt":erlaubt,"maximal":MAX_PLANETEN,"reserviert":reserviert,
            "frei":erlaubt.saturating_sub(anzahl+reserviert),"ueber_limit":anzahl > erlaubt})
    }

    pub fn planet_aufgeben_grund(&self, sid: SpielerId, pid: usize) -> Option<String> {
        let p = &self.planeten[pid];
        if !self.planet_besiedelt(pid) || p.besitzer != sid { return Some("Kein eigener bewohnter Planet".into()); }
        if p.heimat || self.spieler[sid as usize].heimat == p.id { return Some("Die Heimatwelt kann nicht aufgegeben werden".into()); }
        if p.blockade.is_some() || self.kolonisation.besetzungen.contains_key(&p.id) {
            return Some("Während einer Blockade oder laufenden Besetzung kann die Kolonie nicht aufgegeben werden".into());
        }
        None
    }

    pub fn planet_aufgeben_vorschau(&self, sid: SpielerId, k: Koord) -> Result<Value, String> {
        let pid = self.eigener_planet(sid, k)?;
        let p = &self.planeten[pid];
        let grund = self.planet_aufgeben_grund(sid, pid);
        let gebaeude: BTreeMap<_, _> = Gebaeude::ALLE.iter().filter(|g| p.gebaeude[g.idx()] > 0).map(|g| (g.name(), p.gebaeude[g.idx()])).collect();
        let einheiten: BTreeMap<_, _> = Einheit::ALLE.iter().filter(|e| p.einheiten[e.idx()] > 0).map(|e| (e.name(), p.einheiten[e.idx()])).collect();
        let bestand: BTreeMap<_, _> = Gut::ALLE.iter().map(|g| (g.name(), ganz(self.bestand_jetzt(pid)[g.idx()]))).filter(|(_, n)| *n > 0).collect();
        Ok(json!({"planet":k,"moeglich":grund.is_none(),"grund":grund,"verluste":{"gebaeude":gebaeude,"einheiten":einheiten,
            "bestand":bestand,"bevoelkerung":ganz(p.bevoelkerung),"bauauftraege":p.bauschleife.len(),
            "fertigung":p.fertigung.iter().map(|q|q.len()).sum::<usize>()},"kapazitaet":self.planetenkapazitaet(sid),
            "hinweis":"Gebäude, Bestände, Verteidigung und stationierte Schiffe gehen verloren. Rückflotten weichen auf die Heimatwelt aus."}))
    }

    pub fn planet_aufgeben(&mut self, sid: SpielerId, k: Koord, bestaetigung: Koord) -> Result<String, String> {
        let pid = self.eigener_planet(sid, k)?;
        if bestaetigung != k { return Err("Zum Aufgeben die Koordinaten des Planeten bestätigen".into()); }
        if let Some(reason) = self.planet_aufgeben_grund(sid, pid) { return Err(reason); }
        if let Some(reason) = self.mond_aufgeben_grund(pid) { return Err(reason); }
        self.mond_planet_aufgegeben(pid);
        let id = self.planeten[pid].id;
        self.abrechnen(pid);
        self.orders_stornieren_planet(id);
        let offers: Vec<_> = self.kommunikation.angebote.iter().filter(|o| o.planet == id && o.von == sid && o.status == "offen").map(|o| o.id).collect();
        for offer in offers { self.intern_storno(sid, offer)?; }
        let support: Vec<_> = self.flotten.values().filter(|f| f.ziel == k && f.zustand == Flottenzustand::ImOrbit && f.mission == Mission::Halten).map(|f| f.id).collect();
        for fid in support { self.heimflug(fid); }
        self.kolonisation.reparaturen.retain(|(planet, _), _| *planet != id);
        self.kolonisation.integritaet.retain(|(planet, _), _| *planet != id);
        self.kolonisation.besetzungsmarken.remove(&id);
        self.kolonisation.besetzungsangreifer.remove(&id);
        self.ausbau.raketensystem.lager.remove(&id);
        self.ausbau.personen.insert(id, Default::default());
        self.ausbau.wachstumsrest.remove(&id);
        self.ausbau.produktionsserien.remove(&id);self.ausbau.boden.bestand.remove(&id);self.ausbau.boden.miliz.remove(&id);self.ausbau.boden.auftraege.remove(&id);
        for n in 0..2{self.ausbau.fertigung_zahlung.remove(&format!("{id}:{n}"));}
        self.ausbau.ausbildungen.retain(|a| a.planet != id);
        for art in ["bau", "werft", "bauteile"] { self.ausbau.pausen.remove(&format!("{art}:{id}")); }
        self.ausbau.planetennamen.remove(&id);
        self.ereignisse.retain(|e| !matches!(e.art,
            EreignisArt::BauFertig{planet}|EreignisArt::FertigungFertig{planet,..}|EreignisArt::RaketenFertig{planet,..} if planet==id));
        self.belegung.remove(&k);
        self.spieler[sid as usize].planeten.retain(|p| *p != id);
        let home = self.spieler[sid as usize].heimat;
        for (_, planet, _) in &mut self.spieler[sid as usize].forschung_schlange {
            if *planet == id { *planet = home; }
        }
        let p = &mut self.planeten[pid];
        p.bestand = [0; GUETER]; p.rate = [0; GUETER]; p.gebaeude = [0; GEBAEUDE]; p.einheiten = [0; EINHEITEN]; p.raketen = [0; 2];
        self.ausbau.parallel.remove(&format!("{pid}:0"));self.ausbau.parallel.remove(&format!("{pid}:1"));p.bauschleife.clear(); p.fertigung = [vec![], vec![]]; p.fertigung_naechste = [-1, -1];
        p.bevoelkerung = 0; p.wohnraum = 0; p.fp_rate = 0;
        p.prioritaeten.clear(); p.mali.clear(); p.leer_gemeldet = true; p.stand = self.zeit;
        // Der frühere Besitzer weiß sofort, dass der Platz frei ist; andere behalten ihren Nebel.
        let (felder, zone, faktor) = (p.felder, p.zone, p.faktor);
        self.scans.insert((sid, k), Scan { zeit: self.zeit, technik: self.spieler[sid as usize].forschung[Forschung::Spionagetechnik.idx()],
            besitzer: None, felder, zone, erz: [faktor[F_ERZ]; 2], kristall: [faktor[F_KRISTALL]; 2], bestand: vec![[0, 0]; GUETER] });
        if let Some(system) = self.kommunikation.systemkontakte.get_mut(&format!("{sid}:{}:{}", k.sektor, k.system)) { system.insert(k.position, None); }
        self.punkte_neu();
        self.fachereignis("planet_abandoned", sid, json!({"planet":id,"coordinates":k}));
        let text = format!("Kolonie {k} aufgegeben. Jetzt {} / {MAX_PLANETEN} Planeten. Zurückgelassene Bestände und Anlagen gehen verloren; Rückflotten weichen auf die Heimatwelt aus.",
            self.spieler[sid as usize].planeten.len());
        self.vorfall(sid, "kolonie_aufgegeben", text.clone());
        self.wecke(sid, Rolle::Stratege, "Kolonie aufgegeben", false);
        Ok(text)
    }

    // ------------------------------------------------------------------ Planetennamen

    /// Selbst gewählter Name eines Planeten, solange der Benenner ihn noch hält.
    pub fn planetenname(&self, pid: usize) -> Option<&str> {
        let p = self.planeten.get(pid)?;
        self.ausbau.planetennamen.get(&p.id).filter(|(besitzer, _)| *besitzer == p.besitzer && self.planet_besiedelt(pid)).map(|(_, n)| n.as_str())
    }

    /// Name für `sid` sichtbar: eigene Planeten und die von Verbündeten (Allianz, Verteidigungsbündnis).
    /// Fremde sehen ihn nicht, auch nicht nach Spionage – der Name ist eine Notiz des Besitzers, keine Aufklärung.
    pub fn planetenname_fuer(&self, sid: SpielerId, k: Koord) -> Option<&str> {
        let pid = *self.belegung.get(&k)? as usize;
        let owner = self.planeten[pid].besitzer;
        if owner != sid && !self.verbuendet(sid, owner) { return None; }
        self.planetenname(pid)
    }

    pub fn planet_benennen(&mut self, sid: SpielerId, k: Koord, name: &str) -> Result<String, String> {
        if self.ausbau.version == 0 { return Err("Planetennamen gibt es erst in Partien ab Generation 12".into()); }
        let pid = self.eigener_planet(sid, k)?;
        let name = planetenname_pruefen(name)?;
        let id = self.planeten[pid].id;
        if name.is_empty() {
            self.ausbau.planetennamen.remove(&id);
            return Ok(format!("{k} trägt wieder den Standardnamen"));
        }
        let text = format!("{k} heißt jetzt „{name}“");
        self.ausbau.planetennamen.insert(id, (sid, name));
        Ok(text)
    }

    // ------------------------------------------------------------------ Auftragspausen

    pub fn auftrag_pausiert(&self, art: &str, id: u32) -> bool {
        self.ausbau.pausen.contains(&format!("{art}:{id}")) || (["bauteile","werft"].contains(&art) && self.planeten.get(id as usize).is_some_and(|p| {
            let n=usize::from(art=="bauteile");!p.fertigung[n].is_empty() && self.werk_ruht(id as usize,if n==1{Gebaeude::Orbitalwerft}else{Gebaeude::Werft})
        }))
    }

    pub fn auftrag_pause(&mut self, sid: SpielerId, rolle: Rolle, k: Koord, art: &str, pausiert: bool) -> Result<String, String> {
        if self.ausbau.version == 0 { return Err("Pausen gibt es erst in Partien ab Generation 12".into()); }
        let pid = self.eigener_planet(sid, k)?;
        if !PAUSEN_ARTEN.contains(&art) { return Err("Unbekannte Auftragsart".into()); }
        if rolle == Rolle::Feldherr && matches!(art, "bau" | "forschung") { return Err("Bau und Forschung verwaltet der Verwalter".into()); }
        let active = match art {
            "bau" => !self.planeten[pid].bauschleife.is_empty(),
            "forschung" => self.spieler[sid as usize].forschung_aktiv.is_some() || !self.spieler[sid as usize].forschung_schlange.is_empty(),
            _ => !self.planeten[pid].fertigung[usize::from(art == "bauteile")].is_empty(),
        };
        let id = if art == "forschung" { sid as u32 } else { self.planeten[pid].id };
        let key = format!("{art}:{id}");
        if pausiert {
            if !active { return Err("Kein laufender oder wartender Auftrag".into()); }
            self.ausbau.pausen.insert(key);
        } else {
            self.ausbau.pausen.remove(&key);
        }
        Ok(if pausiert { "Auftrag pausiert; Fortschritt und bezahlte Rohstoffe bleiben erhalten" } else { "Auftrag wird fortgesetzt" }.into())
    }

    /// Verschiebt pausierte Fertigstellungen um `dt`; die Ereignisse holen beim Eintreffen nach.
    pub(crate) fn pausen_vorruecken(&mut self, dt: i64) {
        if dt <= 0 { return; }
        // Läuft jede Spielsekunde. `werk_ruht` rechnet die ganze Werksbilanz eines Planeten; innerhalb
        // dieses Aufrufs ändert sich nichts, wovon sie abhängt (nur Fertigstellungstermine werden
        // verschoben). Darum je (Planet, Gebäude) genau einmal rechnen – Ergebnis bitgleich zu vorher.
        let mut ruht_merk: std::collections::BTreeMap<(usize, Gebaeude), bool> = std::collections::BTreeMap::new();
        let mut ruht = |w: &Self, pid: usize, g: Gebaeude| *ruht_merk.entry((pid, g)).or_insert_with(|| w.werk_ruht(pid, g));
        for pid in 0..self.planeten.len() {
            let id = self.planeten[pid].id;
            if self.ausbau.pausen.contains(&format!("bau:{id}")) {
                if let Some(fertig) = self.planeten[pid].bauschleife.first_mut().and_then(|a| a.fertig.as_mut()) { *fertig += dt; }
            }
            for i in 0..2 {
                if self.planeten[pid].fertigung[i].is_empty() || self.planeten[pid].fertigung_naechste[i] < 0 { continue; }
                // Wie `auftrag_pausiert`, aber mit gemerkter Werksbilanz (Planet-ID = Index, siehe dort).
                let art = if i == 0 { "werft" } else { "bauteile" };
                let werk = if i == 1 { Gebaeude::Orbitalwerft } else { Gebaeude::Werft };
                let pausiert = self.ausbau.pausen.contains(&format!("{art}:{id}"))
                    || self.planeten.get(id as usize).is_some_and(|p| !p.fertigung[i].is_empty()) && ruht(self, id as usize, werk);
                if pausiert {
                    self.planeten[pid].fertigung_naechste[i] += dt;
                    if let Some(lots)=self.ausbau.parallel.get_mut(&crate::parallel::key(pid,i)){for b in lots{b.ende+=dt;}}
                }
            }
        }
        let silo_jobs=self.ereignisse.iter().filter_map(|e|match e.art{EreignisArt::RaketenFertig{planet,..}=>Some(planet),_=>None}).collect::<std::collections::BTreeSet<_>>();
        let silo_pause=silo_jobs.into_iter().filter(|pid|ruht(self,*pid as usize,Gebaeude::Raketensilo)).collect::<std::collections::BTreeSet<_>>();
        let queue=std::mem::take(&mut self.ereignisse);self.ereignisse=queue.into_iter().map(|mut e|{if matches!(e.art,EreignisArt::RaketenFertig{planet,..}if silo_pause.contains(&planet)){e.zeit+=dt;if let Some(p)=self.ausbau.raketen_zahlung.get_mut(&e.seq){for b in &mut p.lose{b.ende+=dt;}}}e}).collect();
        let schulen=self.ausbau.ausbildungen.iter().map(|a|a.planet).collect::<std::collections::BTreeSet<_>>();
        let school_pause=schulen.into_iter().filter(|planet|ruht(self,*planet as usize,Gebaeude::Akademie)).collect::<std::collections::BTreeSet<_>>();
        for a in &mut self.ausbau.ausbildungen{if school_pause.contains(&a.planet){a.fertig+=dt;}}
        // Erledigte Aufträge lösen ihre Pause auf, damit spätere Aufträge nicht hängen bleiben.
        let mut keep = std::collections::BTreeSet::new();
        for key in std::mem::take(&mut self.ausbau.pausen) {
            let Some((art, id)) = key.split_once(':') else { continue };
            let Ok(id) = id.parse::<u32>() else { continue };
            let lebt = match art {
                "forschung" => self.spieler.get(id as usize).is_some_and(|s| s.forschung_aktiv.is_some() || !s.forschung_schlange.is_empty()),
                _ => self.planeten.iter().position(|p| p.id == id).is_some_and(|pid| {
                    let p = &self.planeten[pid];
                    match art { "bau" => !p.bauschleife.is_empty(), "werft" => !p.fertigung[0].is_empty(), _ => !p.fertigung[1].is_empty() }
                }),
            };
            if lebt { keep.insert(key); }
        }
        self.ausbau.pausen = keep;
    }

    // ------------------------------------------------------------------ Soforthandel

    /// Sofortiger Tausch im eigenen System; dieselbe Rechnung dient Vorschau und Ausführung.
    pub fn soforthandel_angebot(&self, sid: SpielerId, k: Koord, angebot: &BTreeMap<String, f64>, ziel: &str) -> Result<Value, String> {
        let pid = self.eigener_planet(sid, k)?;
        let handel = &self.regeln.kw().handel;
        let price = |key: &str| handel.kurs_milli(key);
        let gebuehr = milli(handel.soforthandel_gebuehr);
        let gebuehr_text = format!("{}", handel.soforthandel_gebuehr * 100.0).replace('.', ",");
        let target_price = price(ziel).ok_or("Ziel muss Erz, Kristall, Deuterium, Nahrung oder Credits sein")?;
        let stock = self.bestand_jetzt(pid);
        let mut value = 0i128;
        let mut inputs = BTreeMap::new();
        for (g, n) in angebot {
            let p = price(g).ok_or("Unbekanntes Handelsgut")?;
            if !n.is_finite() || *n < 0.0 || *n > 1_000_000_000.0 { return Err("Ungültige Handelsmenge".into()); }
            let amount = milli(*n);
            if amount == 0 { continue; }
            if g == ziel { return Err("Angebot und Ziel müssen verschieden sein".into()); }
            let available = if g == "credits" { self.spieler[sid as usize].credits } else { stock[Gut::aus_name(g).unwrap().idx()] };
            if amount > available { return Err(format!("Nicht genug {g} für das Angebot")); }
            value += amount as i128 * p as i128;
            inputs.insert(g.clone(), amount);
        }
        let output = (value * (1000 - gebuehr) as i128 / 1000 / target_price as i128) as i64;
        if output < M { return Err(format!("Das Angebot muss nach {gebuehr_text} % Gebühr mindestens eine Zieleinheit ergeben")); }
        if ziel != "credits" {
            let g = Gut::aus_name(ziel).unwrap();
            if stock[g.idx()].saturating_add(output) > self.lagergrenze(pid)[g.idx()] { return Err("Im Ziellager ist nicht genug Platz; Angebot verkleinern oder Lager ausbauen".into()); }
        }
        Ok(json!({"angebot_milli":inputs,"ziel":ziel,"erhalt_milli":output,"erhalt":output as f64/M as f64,"gebuehr_prozent":if gebuehr % 10 == 0 { json!(gebuehr / 10) } else { json!(gebuehr as f64 / 10.0) },
            "kurse":(["erz","kristall","deuterium","nahrung","credits"].iter().map(|k|(k.to_string(),json!(handel.soforthandel_kurse.get(*k).copied().unwrap_or(0.0)))).collect::<serde_json::Map<_,_>>())}))
    }

    pub fn soforthandel(&mut self, sid: SpielerId, rolle: Rolle, k: Koord, angebot: &BTreeMap<String, f64>, ziel: &str) -> Result<String, String> {
        let q = self.soforthandel_angebot(sid, k, angebot, ziel)?;
        let pid = self.eigener_planet(sid, k)?;
        let mut cost = [0; GUETER];
        let mut credits = 0;
        for (g, n) in q["angebot_milli"].as_object().unwrap() {
            if g == "credits" { credits = n.as_i64().unwrap(); } else { cost[Gut::aus_name(g).unwrap().idx()] = n.as_i64().unwrap(); }
        }
        let topf = Self::topf_fuer(rolle, Topf::Wirtschaft);
        self.abrechnen(pid);
        self.zahlbar(pid, &cost, topf)?;
        if let Some(t) = topf {
            if self.spieler[sid as usize].toepfe[t.idx()] < self.regeln.wert(&cost).saturating_add(credits) { return Err("Wirtschaftsbudget reicht nicht für den Tausch".into()); }
        }
        self.zahlen(pid, &cost, topf);
        self.spieler[sid as usize].credits -= credits;
        if let Some(t) = topf { self.spieler[sid as usize].toepfe[t.idx()] -= credits; }
        let output = q["erhalt_milli"].as_i64().unwrap();
        if ziel == "credits" { self.spieler[sid as usize].credits += output; } else { self.planeten[pid].bestand[Gut::aus_name(ziel).unwrap().idx()] += output; }
        self.raten_neu(pid);
        self.bau_starten(pid);
        // Deutsch formatiert (Punkt als Dezimalzeichen las sich als Tausender: „326.666 deuterium“) und Gebühr aus den Kernwerten.
        let ganz_teil = crate::schiffhandel::credits_text(output / M);
        let rest = output % M;
        let menge = if rest == 0 { ganz_teil } else { format!("{ganz_teil},{}", format!("{rest:03}").trim_end_matches('0')) };
        let gebuehr = q["gebuehr_prozent"].to_string().replace('.', ",");
        let text = format!("Soforthandel auf {k}: {menge} {ziel} erhalten ({gebuehr} % Gebühr enthalten)");
        self.vorfall(sid, "markt", text.clone());
        Ok(text)
    }

    // ------------------------------------------------------------------ Systemerkundung

    /// Schickt je fremdem Platz eine Spionagesonde los (plus Systemscan,
    /// falls das System noch nicht erfasst ist). Überlebende Sonden kehren zurück.
    pub fn system_aufklaeren(&mut self, sid: SpielerId, rolle: Rolle, start: Koord, ziel: Koord, abbrechen: bool) -> Result<String, String> {
        if abbrechen { return Ok("Kein laufender Sammelauftrag: jede Sonde startet sofort und fliegt allein zu ihrem Ziel.".into()); }
        let pid = self.eigener_planet(sid, start)?;
        if self.system(ziel).is_none() { return Err("Sonnensystem liegt außerhalb der Galaxie".into()); }
        if self.planeten[pid].gebaeude[Gebaeude::Raumhafen.idx()] == 0 { return Err("Ein Raumhafen wird benötigt".into()); }
        if self.planeten[pid].einheiten[Einheit::Spionagesonde.idx()] < 1 { return Err("Mindestens eine Spionagesonde benötigt".into()); }
        let unterwegs: std::collections::BTreeSet<Koord> = self.flotten.values()
            .filter(|f| f.besitzer == sid && matches!(f.mission, Mission::Spionage | Mission::SystemErkunden) && f.zustand == Flottenzustand::Hinflug)
            .map(|f| f.ziel).collect();
        // Ein Tipp erkundet das ganze System (Karl 22.10.2026): der Sammelauftrag reist als Marke in den Flugdaten
        // jeder beteiligten Sonde mit (Ausbau-Welten). Ohne Ausbau bleibt der alte Zweischritt.
        let sammeln = self.ausbau.version > 0;
        let auftrag = json!({"auftrag":self.naechste_flotte,"spieler":sid,"start":start.to_string(),"sektor":ziel.sektor,
            "system":ziel.system,"seit":self.zeit,"versucht":[]});
        // Unkartiertes System: erst nur die Systemsonde – Spionage braucht ein kartiertes System, Planetensonden scheiterten
        // sonst alle („Rest fehlt“). Nach ihrer Ankunft starten die Planetensonden von selbst (sammelauftrag_weiter).
        if !self.system_erfasst(sid, ziel) {
            let mut schiffe = [0; SCHIFFE];
            schiffe[Einheit::Spionagesonde.idx()] = 1;
            let fid = self.naechste_flotte;
            self.flotte_senden(sid, rolle, crate::flotte::Flugauftrag { start, ziel: Koord { position: 1, ..ziel }, mission: Mission::SystemErkunden,
                schiffe, sigma_pm: 1000, ladung: [0; GUETER], haltedauer: 0 })?;
            if sammeln {
                self.sammelmarke_setzen(fid, &auftrag);
                return Ok(format!("System {}:{}: Systemsonde gestartet. Nach der Kartierung fliegen Sonden von selbst alle Plätze an; reichen die Sonden nicht, übernehmen zurückkehrende Sonden die übrigen.", ziel.sektor, ziel.system));
            }
            return Ok(format!("System {}:{}: Systemsonde gestartet – das Sonnensystem wird zuerst kartiert. Nach ihrer Ankunft System erkunden erneut wählen, dann fliegen Sonden zu allen unbekannten Plätzen.", ziel.sektor, ziel.system));
        }
        if sammeln {
            let (gestartet, offen, grund) = self.sammelauftrag_weiter(&auftrag);
            if gestartet == 0 { return Err(if grund.is_empty() { "Kein unbekannter Platz in diesem System".into() } else { grund }); }
            return Ok(if offen == 0 { format!("System {}:{}: {gestartet} Sonden gestartet. Berichte kommen bei Ankunft; überlebende Sonden fliegen zurück.", ziel.sektor, ziel.system) }
                else { format!("System {}:{}: {gestartet} Sonden gestartet; die übrigen {offen} Plätze fliegen zurückkehrende Sonden an.", ziel.sektor, ziel.system) });
        }
        let mut ziele: Vec<(Koord, Mission)> = Vec::new();
        for position in 1..=self.regeln.welt.plaetze_je_system {
            let k = Koord { position, ..ziel };
            let eigen = self.spieler[sid as usize].planeten.iter().any(|p| self.planeten[*p as usize].koord == k);
            if !eigen && !unterwegs.contains(&k) { ziele.push((k, Mission::Spionage)); }
        }
        let gesamt = ziele.len();
        let (mut gestartet, mut grund) = (0, String::new());
        for (k, mission) in ziele {
            let mut schiffe = [0; SCHIFFE];
            schiffe[Einheit::Spionagesonde.idx()] = 1;
            match self.flotte_senden(sid, rolle, crate::flotte::Flugauftrag { start, ziel: k, mission, schiffe, sigma_pm: 1000, ladung: [0; GUETER], haltedauer: 0 }) {
                Ok(_) => gestartet += 1,
                Err(e) => { grund = e; break; }
            }
        }
        if gestartet == 0 { return Err(if grund.is_empty() { "Kein unbekannter Platz in diesem System".into() } else { grund }); }
        let rest = gesamt - gestartet;
        Ok(if rest == 0 { format!("System {}:{}: {gestartet} Sonden gestartet. Berichte kommen bei Ankunft; überlebende Sonden fliegen zurück.", ziel.sektor, ziel.system) }
           else { format!("System {}:{}: {gestartet} von {gesamt} Sonden gestartet; Rest fehlt: {grund}", ziel.sektor, ziel.system) })
    }

    fn sammelmarke_setzen(&mut self, fid: FlottenId, auftrag: &Value) {
        if let Some(v) = self.ausbau.fluganzeigen.get_mut(&fid) { v["sammelauftrag"] = auftrag.clone(); }
    }

    /// Sammelauftrag dieser Flotte, falls sie für eine Systemerkundung fliegt.
    pub(crate) fn sammelauftrag(&self, fid: FlottenId) -> Option<Value> {
        self.ausbau.fluganzeigen.get(&fid).map(|v| v["sammelauftrag"].clone()).filter(|v| v.is_object())
    }

    /// Schickt freie Sonden des Startplaneten zu allen Plätzen des Auftrags, die noch keine Sonde dieses Auftrags
    /// angeflogen hat. Jede Platz bekommt höchstens einen Versuch (eine abgeschossene Sonde wird nicht wiederholt).
    /// Rückgabe: gestartet, noch offen, Grund des Abbruchs.
    pub(crate) fn sammelauftrag_weiter(&mut self, auftrag: &Value) -> (usize, usize, String) {
        let id = auftrag["auftrag"].as_u64().unwrap_or(0);
        let Some(sid) = auftrag["spieler"].as_u64().map(|s| s as SpielerId).filter(|s| (*s as usize) < self.spieler.len()) else { return (0, 0, String::new()) };
        let (sektor, system) = (auftrag["sektor"].as_u64().unwrap_or(0) as u8, auftrag["system"].as_u64().unwrap_or(0) as u8);
        let mut versucht: std::collections::BTreeSet<u8> = auftrag["versucht"].as_array().into_iter().flatten().filter_map(|p| p.as_u64().map(|p| p as u8)).collect();
        for v in self.ausbau.fluganzeigen.values() {
            if v["sammelauftrag"]["auftrag"].as_u64() == Some(id) {
                versucht.extend(v["sammelauftrag"]["versucht"].as_array().into_iter().flatten().filter_map(|p| p.as_u64().map(|p| p as u8)));
            }
        }
        let unterwegs: std::collections::BTreeSet<Koord> = self.flotten.values()
            .filter(|f| f.besitzer == sid && matches!(f.mission, Mission::Spionage | Mission::SystemErkunden) && f.zustand == Flottenzustand::Hinflug)
            .map(|f| f.ziel).collect();
        let ziele: Vec<Koord> = (1..=self.regeln.welt.plaetze_je_system).map(|position| Koord::neu(sektor, system, position))
            .filter(|k| !versucht.contains(&k.position) && !unterwegs.contains(k)
                && !self.spieler[sid as usize].planeten.iter().any(|p| self.planeten[*p as usize].koord == *k))
            .collect();
        // Startplanet verloren: die Heimatwelt übernimmt.
        let start = auftrag["start"].as_str().and_then(|s| s.parse::<Koord>().ok())
            .filter(|k| self.eigener_planet(sid, *k).is_ok())
            .unwrap_or(self.planeten[self.spieler[sid as usize].heimat as usize].koord);
        let (mut gestartet, mut grund) = (Vec::new(), String::new());
        for k in &ziele {
            let Ok(pid) = self.eigener_planet(sid, start) else { break };
            if self.planeten[pid].einheiten[Einheit::Spionagesonde.idx()] < 1 { grund = "Keine Spionagesonde frei".into(); break; }
            let mut schiffe = [0; SCHIFFE];
            schiffe[Einheit::Spionagesonde.idx()] = 1;
            let fid = self.naechste_flotte;
            match self.flotte_senden(sid, Rolle::Feldherr, crate::flotte::Flugauftrag { start, ziel: *k, mission: Mission::Spionage, schiffe, sigma_pm: 1000, ladung: [0; GUETER], haltedauer: 0 }) {
                Ok(_) => gestartet.push((fid, k.position)),
                Err(e) => { grund = e; break; }
            }
        }
        versucht.extend(gestartet.iter().map(|(_, p)| *p));
        let mut marke = auftrag.clone();
        marke["versucht"] = json!(versucht);
        for (fid, _) in &gestartet { self.sammelmarke_setzen(*fid, &marke); }
        (gestartet.len(), ziele.len() - gestartet.len(), grund)
    }

    /// Eine Sonde des Sammelauftrags ist angekommen (Systemsonde) oder gelandet: offene Plätze nachschieben.
    pub(crate) fn sammelauftrag_fortsetzen(&mut self, auftrag: &Value, gelandet: bool) {
        let (gestartet, offen, _) = self.sammelauftrag_weiter(auftrag);
        let Some(sid) = auftrag["spieler"].as_u64().map(|s| s as SpielerId).filter(|s| (*s as usize) < self.spieler.len()) else { return };
        let system = format!("{}:{}", auftrag["sektor"], auftrag["system"]);
        let id = auftrag["auftrag"].as_u64();
        let laufend = self.flotten.keys().any(|f| self.sammelauftrag(*f).is_some_and(|m| m["auftrag"].as_u64() == id));
        if !gelandet && gestartet > 0 {
            self.vorfall(sid, "systemaufklaerung", if offen == 0 { format!("System {system}: {gestartet} Planetensonden gestartet") }
                else { format!("System {system}: {gestartet} Planetensonden gestartet; {offen} Plätze folgen mit zurückkehrenden Sonden") });
        } else if gelandet && gestartet == 0 && !laufend {
            self.vorfall(sid, "systemaufklaerung", if offen == 0 { format!("System {system} vollständig erkundet") }
                else { format!("Systemerkundung {system} angehalten: keine Spionagesonde frei, {offen} Plätze offen") });
        }
    }
}
