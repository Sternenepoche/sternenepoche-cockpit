//! Großziele mit Weg (Karl 27.10.2026: „stelle sicher das die agenten die weg zu großen aufgaben finden und lösen
//! können alleine als einzelspieler agent wie aus agenten in allianzen zusammen optimal interoperieren können!“).
//!
//! Die Agenda meldet, was jetzt wartet. Die Ziele zeigen je Großziel die ganze Kette von heute bis zum Ziel: was fehlt,
//! wer es tut (Rolle des eigenen Reichs, Allianzleitung oder ein bestimmtes Mitglied) und – nur für die fragende Rolle –
//! fertige Vorlagen. Für die Dysonsphäre verteilt der Server die freien Plätze des Zielsystems auf die Mitglieder, damit
//! zehn Reiche nicht alle denselben Platz anfliegen. Nichts hier verändert die Welt.
use super::*;
use crate::agenten::{rollen_fuer, schema_fehler, schemavarianten, vervollstaendigen};

/// Höchstzahl offener Schritte je Ziel und Zeichen aller Ziele zusammen (die Agenda geht in jeden Prompt).
const SCHRITTE_MAX: usize = 8;
const ZEICHEN_MAX: usize = 5_500;

/// Was die Dyson-Rechnung der Agenda zusätzlich meldet.
#[derive(Default)]
pub(crate) struct DysonWeg {
    /// Freier Platz im Zielsystem, den dieses Reich besiedeln soll.
    pub platz: Option<String>,
    /// System, das die Leitung jetzt vorschlagen sollte (kein offenes Projekt, genug Mitglieder).
    pub vorschlag: Option<String>,
}

struct Weg {
    rolle: Rolle,
    ich: String,
    schritte: Vec<Value>,
    erledigt: usize,
    /// Vorlagen der Voraussetzungsschritte, damit dieselbe Voraussetzung nur einmal im Weg steht.
    vorab: Vec<Value>,
}

impl Weg {
    fn neu(rolle: Rolle, ich: &str) -> Self { Weg { rolle, ich: ich.to_string(), schritte: Vec::new(), erledigt: 0, vorab: Vec::new() } }

    /// Fehlende Voraussetzungen vor einem Schritt (aus `Game::bau_voraussetzungen` / `forsch_voraussetzungen`).
    fn voraus(&mut self, liste: Vec<(String, Value)>) {
        for (text, v) in liste {
            if self.vorab.contains(&v) { continue; }
            self.vorab.push(v.clone());
            self.offen(text, &["verwalter"], None, vec![v]);
        }
    }

    /// Offener Schritt. `wer`: zuständige Rollen; `mitglied`: anderes Reich, das ihn tun muss (None = dieses Reich).
    /// Vorlagen bekommt nur die fragende Rolle, und nur solche, die sie selbst senden darf.
    fn offen(&mut self, text: String, wer: &[&str], mitglied: Option<&str>, vorlagen: Vec<Value>) -> usize {
        let eigen = mitglied.is_none_or(|m| m == self.ich);
        let mut s = json!({"text": text, "wer": wer});
        if let Some(m) = mitglied { s["mitglied"] = json!(m); }
        if eigen && wer.contains(&self.rolle.name()) {
            s["du"] = json!(true);
            let v: Vec<Value> = vorlagen.into_iter().filter(|v| v["typ"].as_str().is_some_and(|t| rollen_fuer(t).contains(&self.rolle))).collect();
            if !v.is_empty() { s["vorlagen"] = json!(v); }
        }
        self.schritte.push(s);
        self.schritte.len() - 1
    }

    fn fertig(&mut self) { self.erledigt += 1; }

    fn ziel(self, ziel: &str, titel: String, stand: String) -> Value {
        let offen = self.schritte.len();
        let mut s = self.schritte;
        if s.len() > SCHRITTE_MAX {
            // Reihenfolge behalten, aber Schritte der fragenden Rolle nicht abschneiden.
            let rest = s.split_off(SCHRITTE_MAX);
            for d in rest.into_iter().filter(|x| x["du"] == true) {
                if let Some(i) = s.iter().rposition(|x| x["du"] != true) { s.remove(i); s.push(d); }
            }
        }
        json!({"ziel": ziel, "titel": titel, "stand": stand, "erreicht": offen == 0, "offen": offen, "erledigt": self.erledigt, "schritte": s})
    }
}

fn gut_menge(v: &Value, g: &str) -> i64 { v[g].as_f64().unwrap_or(0.0).floor() as i64 }

/// Alle Werte der Vorlage stehen unverändert in der vervollständigten Fassung (sonst hätte das Schema der Rolle z. B. eine
/// fremde Mission auf den ersten erlaubten Wert umgebogen – dann gehört die Vorlage nicht zu dieser Rolle).
fn erhalten(vorher: &Value, nachher: &Value) -> bool {
    match vorher.as_object() {
        Some(o) => o.iter().all(|(k, x)| nachher.get(k).is_some_and(|y| erhalten(x, y))),
        None => vorher == nachher,
    }
}

/// Vorlagen gegen das aufgelöste Antwortschema der Rolle vervollständigen (alle Pflichtfelder, wie im Aktionskatalog);
/// was die Rolle so nicht senden kann, fällt weg.
fn vorlagen_vervollstaendigen(liste: &mut [Value], schema: &Value) {
    for s in liste.iter_mut().flat_map(|z| z["schritte"].as_array_mut().into_iter().flatten()) {
        let Some(vs) = s.get_mut("vorlagen").and_then(Value::as_array_mut) else { continue };
        let fertig: Vec<Value> = vs.iter().filter_map(|v| {
            let t = v["typ"].as_str()?;
            let varianten = schemavarianten(schema, t);
            varianten.iter().map(|x| (vervollstaendigen(v, x), x)).find(|(c, x)| schema_fehler(c, x, t).is_none() && erhalten(v, c)).map(|(c, _)| c)
        }).collect();
        if fertig.is_empty() { if let Some(o) = s.as_object_mut() { o.remove("vorlagen"); } } else { *vs = fertig; }
    }
}

/// Summe der Stückzahlen einer Einheitenliste aus einem Spionagebericht (`{name: anzahl}`).
fn stueck(v: &Value) -> i64 { v.as_object().map(|o| o.values().filter_map(Value::as_i64).sum()).unwrap_or(0) }

impl Game {
    /// Großziele des Reichs aus Sicht einer Rolle (Feld `ziele` der Agenda).
    pub(crate) fn ziele(&self, sid: SpielerId, rolle: Rolle, lage: &Value) -> (Value, DysonWeg) {
        let ich = lage["name"].as_str().unwrap_or("").to_string();
        let (dyson, weg) = self.ziel_dyson(sid, rolle, &ich).map(|(z, w)| (Some(z), w)).unwrap_or_default();
        let mut liste = Vec::new();
        liste.extend(self.ziel_stufe(sid, rolle, &ich, lage));
        liste.extend(self.ziel_kolonie(sid, rolle, &ich, lage, weg.platz.as_deref()));
        liste.push(self.ziel_rohstoffe(sid, rolle, &ich, lage));
        liste.push(self.ziel_allianz(sid, rolle, &ich, lage));
        // Gemeinsames Projekt zuerst, solange es für diese Rolle eigene Schritte hat (KI-Weg-Lauf 28.10.2026: die Siedler
        // spähten mit ihrer Koloniekette alle denselben Nebenplatz aus, statt ihren Dyson-Anteil ins System zu bringen).
        if let Some(d) = dyson {
            let eigen = d["schritte"].as_array().is_some_and(|s| s.iter().any(|x| x["du"] == true));
            if eigen { liste.insert(0, d); } else { liste.push(d); }
        }
        vorlagen_vervollstaendigen(&mut liste, &kern::aktion::schema_aufloesen(&self.world.antwortschema(rolle)));
        // Größe begrenzen: beim Ziel mit den meisten Schritten zuerst kürzen (eigene Schritte zuletzt).
        while serde_json::to_string(&liste).map(|s| s.len()).unwrap_or(0) > ZEICHEN_MAX {
            let Some(z) = liste.iter_mut().max_by_key(|z| z["schritte"].as_array().map(Vec::len).unwrap_or(0)) else { break };
            let Some(s) = z["schritte"].as_array_mut().filter(|s| s.len() > 1) else { break };
            let i = s.iter().rposition(|x| x["du"] != true).unwrap_or(s.len() - 1);
            s.remove(i);
        }
        (json!({"hinweis": "Großziele mit Weg vom heutigen Stand: schritte sind offen und in Reihenfolge; du=true heißt, diese Rolle ist zuständig (vorlagen sind gültige Aktionen, vor dem Senden mit probe prüfen); wer/mitglied nennt die zuständige Rolle bzw. das Allianzmitglied – Bedarf über die Pinwand (Rat) bzw. bereich allianz weitergeben.",
            "liste": liste}), weg)
    }

    fn heimat(&self, sid: SpielerId) -> String {
        let sp = &self.world.spieler[sid as usize];
        self.world.planeten[sp.heimat as usize].koord.to_string()
    }

    /// Dyson-Anteil bezahlen: Einzahlungs-Vorlagen von eigenen Planeten im Projektsystem (Mindestbuchung oder genau der
    /// Rest); reicht dort nichts, ein Transport vom reichsten eigenen Planeten außerhalb zu einem eigenen Planeten im System.
    fn dyson_zahlweg(&self, sid: SpielerId, p: &kern::dyson::DysonProjekt, rest: &[i64]) -> (Vec<Value>, Vec<(String, Vec<Value>)>) {
        let w = &self.world;
        let min = w.dyson_mindestbuchung();
        let hat = |pid: usize, i: usize| w.planeten[pid].bestand[kern::dyson::ROHSTOFFE[i].idx()].max(0) / M;
        let eigene: Vec<usize> = w.spieler[sid as usize].planeten.iter().map(|x| *x as usize).filter(|x| w.planet_aktiv(*x)).collect();
        let (drin, draussen): (Vec<usize>, Vec<usize>) = eigene.into_iter().partition(|x| p.im_system(w.planeten[*x].koord));
        let mut einzahlen = vec![];
        for pid in &drin {
            let teil = |i: usize| { let x = rest[i].min(hat(*pid, i)); if x >= min || x == rest[i] { x } else { 0 } };
            let m = [teil(0), teil(1), teil(2)];
            if m.iter().any(|x| *x > 0) {
                einzahlen.push(json!({"typ":"dyson_einzahlen","projekt":p.id,"von":w.planeten[*pid].koord.to_string(),"erz":m[0],"kristall":m[1],"deuterium":m[2]}));
            }
        }
        let Some(ziel) = drin.first() else { return (einzahlen, vec![]) };
        if !einzahlen.is_empty() { return (einzahlen, vec![]); }
        if let Some(h) = w.flotten.values().filter(|f| f.besitzer == sid && f.zustand == kern::welt::Flottenzustand::Hinflug && f.mission == Mission::Transport && p.im_system(f.ziel))
            .map(|f| ((f.ankunft - w.zeit).max(0) + STUNDE - 1) / STUNDE).min() {
            return (einzahlen, vec![(format!("Transport ins Dyson-System ist unterwegs (Ankunft in etwa {h} Stunden): abwarten, danach von dort einzahlen"), vec![])]);
        }
        // Ladung: offener Rest, höchstens der Bestand; vom Deuterium bleibt Treibstoff auf dem Startplanet.
        let ladung = |pid: usize| -> [i64; 3] {
            let mut l = [0, 1, 2].map(|i| rest[i].min(hat(pid, i)));
            l[2] = l[2].min((hat(pid, 2) - (hat(pid, 2) / 20).max(10_000)).max(0));
            l
        };
        let Some(start) = draussen.iter().copied().max_by_key(|x| ladung(*x).iter().sum::<i64>()).filter(|x| ladung(*x).iter().sum::<i64>() > 0) else {
            return (einzahlen, vec![(format!("Für den Dyson-Anteil liegt auf keinem eigenen Planeten genug: Ziel rohstoffe (Markt, Allianz, Raub), dann zu {} im System bringen", w.planeten[*ziel].koord), vec![])]);
        };
        let von = w.planeten[start].koord.to_string();
        let nach = w.planeten[*ziel].koord.to_string();
        let voll = ladung(start);
        // Frachtraum für weniger als 90 % der Ladung: zusätzlich große Transporter fertigen (KI-Weg-Lauf 28.10.2026: zwölf
        // Transporter brachten 92 000 je Rohstoff pro Flug – für 5 Mio. Anteil über 50 Flüge).
        let nachbauen = |geladen: i64| -> (String, Vec<Value>) {
            let lr = w.regeln.einheiten.get(&Einheit::GrosserTransporter).map(|r| r.ladung).unwrap_or(0);
            let fehlt = voll.iter().sum::<i64>() - geladen;
            if lr <= 0 || fehlt * 10 <= voll.iter().sum::<i64>() { return (String::new(), vec![]); }
            let n = ((fehlt.saturating_mul(5) / 4 + lr - 1) / lr).min(300);
            let (ort, _) = self.werft_planet(sid);
            (format!("Frachtraum reicht nur für einen Teil des Dyson-Anteils: {n} grosser_transporter auf {ort} fertigen (dann weniger Flüge)"),
                vec![json!({"typ":"fertigen","planet":ort,"einheit":"grosser_transporter","anzahl":n})])
        };
        match self.frachtflug(start, &nach, voll) {
            Some((v, l)) => {
                let (mehr, f) = nachbauen(l.iter().sum());
                let mut schritte = vec![(format!("Dyson-Anteil von {von} zum eigenen Planeten {nach} im System bringen (flotte_senden transport, {} Erz, {} Kristall, {} Deuterium) – einzahlen geht nur von dort", l[0], l[1], l[2]), vec![v])];
                if !f.is_empty() { schritte.push((mehr, f)); }
                (einzahlen, schritte)
            }
            None => {
                let (_, f) = nachbauen(0);
                (einzahlen, vec![(format!("Dyson-Anteil liegt auf {von}, dort fehlen Frachtschiffe für den Transport nach {nach} im System (grosser_transporter fertigen)"), f)])
            }
        }
    }

    /// Allianzhilfe für den Dyson-Anteil (Karl 27.10.2026: Agenten in Allianzen sollen zusammen optimal interoperieren;
    /// KI-Weg-Lauf 28.10.2026: Siedler-Reiche hatten 92 133 je Rohstoff für 5 Mio. Anteil, die Gründer Millionen übrig).
    /// Mitglieder, deren ganzer Bestand den offenen Anteil nicht deckt, gehen reihum an Mitglieder mit Überschuss; für dieses
    /// Reich als Helfer ein Transport zum Planeten des Bedürftigen im System, dort zahlt er selbst ein.
    fn dyson_hilfe(&self, sid: SpielerId, p: &kern::dyson::DysonProjekt) -> Option<(String, Vec<Value>)> {
        let w = &self.world;
        let anteil = w.ausbau.dyson.anteil;
        let planeten = |s: SpielerId| -> Vec<usize> { w.spieler[s as usize].planeten.iter().map(|x| *x as usize).filter(|x| w.planet_aktiv(*x)).collect() };
        let hat = |pid: usize, i: usize| w.planeten[pid].bestand[kern::dyson::ROHSTOFFE[i].idx()].max(0) / M;
        let rest = |s: SpielerId| -> [i64; 3] { p.mitglieder.get(&s).map(|b| [0, 1, 2].map(|i| (anteil - b.eingezahlt[i] / M).max(0))).unwrap_or([0; 3]) };
        // Helfer braucht Frachtschiffe und eine Sonde oder Werft für die Aufklärung, sonst bliebe der Bedürftige ohne Lieferung.
        let kann_liefern = |s: SpielerId| {
            let ps = planeten(s);
            let fracht = ps.iter().any(|x| Einheit::ALLE.iter().any(|e| e.ist_schiff() && !matches!(e, Einheit::Kolonieschiff | Einheit::GrossesKolonieschiff)
                && w.planeten[*x].einheiten[e.idx()] > 0 && w.regeln.einheiten.get(e).is_some_and(|r| r.ladung > 0)));
            let sicht = ps.iter().any(|x| w.planeten[*x].einheiten[Einheit::Spionagesonde.idx()] > 0 || w.planeten[*x].gebaeude[Gebaeude::Werft.idx()] > 0);
            fracht && sicht
        };
        let mut beduerftig = Vec::new();
        let mut helfer = Vec::new();
        for s in p.mitglieder.keys().copied().filter(|s| w.spieler_aktiv(*s)) {
            let r = rest(s);
            let summe = [0, 1, 2].map(|i| planeten(s).iter().map(|x| hat(*x, i)).sum::<i64>());
            let fehlt = [0, 1, 2].map(|i| (r[i] - summe[i]).max(0));
            if fehlt.iter().any(|x| *x > 0) { beduerftig.push((s, fehlt)); }
            else if [0, 1, 2].iter().any(|i| summe[*i] - r[*i] >= anteil / 10) && kann_liefern(s) { helfer.push(s); }
        }
        let ich = helfer.iter().position(|s| *s == sid)?;
        let (n, fehlt) = beduerftig.iter().enumerate().find(|(i, _)| i % helfer.len() == ich).map(|(_, x)| *x)?;
        let nname = &w.spieler[n as usize].name;
        let ziel = planeten(n).into_iter().find(|x| p.im_system(w.planeten[*x].koord))?;
        let nach = w.planeten[ziel].koord;
        let text = format!("Allianzhilfe: {nname} fehlen {}/{}/{} Erz/Kristall/Deuterium für den Dyson-Anteil", fehlt[0], fehlt[1], fehlt[2]);
        if let Some(h) = w.flotten.values().filter(|f| f.besitzer == sid && f.zustand == kern::welt::Flottenzustand::Hinflug && f.mission == Mission::Transport && f.ziel == nach)
            .map(|f| ((f.ankunft - w.zeit).max(0) + STUNDE - 1) / STUNDE).min() {
            return Some((format!("{text} – deine Lieferung nach {nach} ist unterwegs (Ankunft in etwa {h} Stunden), {nname} zahlt dann ein"), vec![]));
        }
        // Transporte an fremde Planeten verlangt der Kern mit eigenem Bericht; Mitglieder derselben Allianz beliefern
        // einander ohne (Karl 28.10.2026) – Projektmitglieder sind das immer, der Zweig bleibt für Bündnispartner.
        if w.aufklaerungsregeln() && !w.gleiche_allianz(sid, n) && w.planetenwissen(sid, nach)["bekannt"] != true {
            let sonde = planeten(sid).into_iter().find(|x| w.planeten[*x].einheiten[Einheit::Spionagesonde.idx()] > 0);
            let kartiert = w.system_erfasst(sid, nach);
            let v = match sonde {
                Some(x) => vec![json!({"typ":"flotte_senden","start":w.planeten[x].koord.to_string(),"ziel":nach.to_string(),"mission":if kartiert { "spionage" } else { "system_erkunden" },
                    "schiffe":{"spionagesonde":1},"geschwindigkeit":1.0,"ladung":{},"haltedauer_stunden":0})],
                // Fertigen auf dem eigenen Planeten mit der größten Werft (KI-Weg-Lauf 28.10.2026: die Heimat von
                // Sonnenrat4 hatte keine Werft, die Vorlage wurde abgelehnt).
                None => planeten(sid).into_iter().filter(|x| w.planeten[*x].gebaeude[Gebaeude::Werft.idx()] > 0)
                    .max_by_key(|x| w.planeten[*x].gebaeude[Gebaeude::Werft.idx()])
                    .map(|x| vec![json!({"typ":"fertigen","planet":w.planeten[x].koord.to_string(),"einheit":"spionagesonde","anzahl":2})]).unwrap_or_default(),
            };
            let spaeht = w.flotten.values().any(|f| f.besitzer == sid && f.zustand == kern::welt::Flottenzustand::Hinflug && matches!(f.mission, Mission::Spionage | Mission::SystemErkunden) && f.ziel == nach);
            if spaeht { return Some((format!("{text} – Sonde nach {nach} ist unterwegs: abwarten, dann liefern"), vec![])); }
            return Some((format!("{text} – erst {nach} mit einer Sonde ausspähen ({}; der Kern verlangt für Transporte an fremde Planeten den eigenen Bericht), dann per transport liefern",
                if kartiert { "spionage" } else { "system_erkunden, dann spionage" }), v));
        }
        // Vom eigenen Planeten mit dem größten Überschuss; der eigene offene Anteil und Treibstoff bleiben.
        let r = rest(sid);
        let frei = |pid: usize| -> [i64; 3] {
            let mut l = [0, 1, 2].map(|i| fehlt[i].min((hat(pid, i) - r[i]).max(0)));
            l[2] = l[2].min((hat(pid, 2) - r[2] - (hat(pid, 2) / 20).max(10_000)).max(0));
            l
        };
        let start = planeten(sid).into_iter().max_by_key(|x| frei(*x).iter().sum::<i64>()).filter(|x| frei(*x).iter().sum::<i64>() > 0)?;
        match self.frachtflug(start, &nach.to_string(), frei(start)) {
            Some((v, l)) => Some((format!("{text} – liefere per flotte_senden transport von {} an {nname}s Planeten {nach} im System ({} Erz, {} Kristall, {} Deuterium); dort zahlt {nname} selbst ein, Absprache über die Allianz-Pinwand",
                w.planeten[start].koord, l[0], l[1], l[2]), vec![v])),
            None => Some((format!("{text} – auf {} fehlen Frachtschiffe für die Lieferung nach {nach} (fertigen, z. B. grosser_transporter)", w.planeten[start].koord), vec![])),
        }
    }

    /// Transport von Planet `start` nach `nach` mit Erz/Kristall/Deuterium `l`: Frachtschiffe nach Laderaum, bis die Ladung
    /// mit Reserve passt; reicht der Raum nicht, wird die Ladung gekürzt. None ohne Frachtschiffe dort.
    fn frachtflug(&self, start: usize, nach: &str, mut l: [i64; 3]) -> Option<(Value, [i64; 3])> {
        let w = &self.world;
        let summe: i64 = l.iter().sum();
        if summe <= 0 { return None; }
        let mut frachter: Vec<(Einheit, i64, i64)> = Einheit::ALLE.iter().filter(|e| e.ist_schiff() && !matches!(e, Einheit::Kolonieschiff | Einheit::GrossesKolonieschiff))
            .filter_map(|e| { let lr = w.regeln.einheiten.get(e).map(|r| r.ladung).unwrap_or(0); let n = w.planeten[start].einheiten[e.idx()]; (lr > 0 && n > 0).then_some((*e, lr, n)) }).collect();
        frachter.sort_by_key(|(_, lr, _)| -lr);
        let mut schiffe = serde_json::Map::new();
        let mut raum = 0i64;
        let noetig = summe.saturating_mul(5) / 4;
        for (e, lr, n) in frachter {
            if raum >= noetig { break; }
            let k = ((noetig - raum + lr - 1) / lr).min(n);
            schiffe.insert(e.name().into(), json!(k));
            raum += k * lr;
        }
        if schiffe.is_empty() { return None; }
        if raum * 4 / 5 < summe {
            let faktor = (raum * 4 / 5) as f64 / summe as f64;
            l = l.map(|x| (x as f64 * faktor).floor() as i64);
        }
        Some((json!({"typ":"flotte_senden","start":w.planeten[start].koord.to_string(),"ziel":nach,"mission":"transport","schiffe":schiffe,
            "ladung":{"erz":l[0],"kristall":l[1],"deuterium":l[2]},"geschwindigkeit":1.0,"haltedauer_stunden":0}), l))
    }

    /// Eigener Planet mit den meisten Spionagesonden (Start der Aufklärung) und deren Zahl; ohne Sonden die Heimat und 0.
    fn sonden_planet(&self, sid: SpielerId) -> (String, i64) {
        let w = &self.world;
        w.spieler[sid as usize].planeten.iter().map(|x| *x as usize).filter(|x| w.planet_aktiv(*x))
            .map(|x| (w.planeten[x].einheiten[Einheit::Spionagesonde.idx()], x)).filter(|(n, _)| *n > 0).max()
            .map(|(n, x)| (w.planeten[x].koord.to_string(), n)).unwrap_or_else(|| (self.heimat(sid), 0))
    }

    /// Eigener Planet mit der größten Werft (Ort zum Fertigen), sonst die Heimat.
    fn werft_planet(&self, sid: SpielerId) -> (String, usize) {
        let w = &self.world;
        let pid = w.spieler[sid as usize].planeten.iter().map(|x| *x as usize).filter(|x| w.planet_aktiv(*x))
            .filter(|x| w.planeten[*x].gebaeude[Gebaeude::Werft.idx()] > 0).max_by_key(|x| w.planeten[*x].gebaeude[Gebaeude::Werft.idx()])
            .unwrap_or(w.spieler[sid as usize].heimat as usize);
        (w.planeten[pid].koord.to_string(), pid)
    }

    /// Stunden bis zur Ankunft der eigenen Aufklärung für `k`: Spionage auf genau diesen Platz, oder – solange das System
    /// noch nicht kartiert ist – system_erkunden in dieses System.
    fn aufklaerung_unterwegs(&self, sid: SpielerId, k: Koord, kartiert: bool) -> Option<i64> {
        self.world.flotten.values()
            .filter(|f| f.besitzer == sid && f.zustand == kern::welt::Flottenzustand::Hinflug)
            .filter(|f| match f.mission {
                Mission::Spionage => f.ziel == k,
                Mission::SystemErkunden => !kartiert && f.ziel.sektor == k.sektor && f.ziel.system == k.system,
                _ => false,
            })
            .map(|f| ((f.ankunft - self.world.zeit).max(0) + STUNDE - 1) / STUNDE)
            .min()
    }

    /// Gebäude, die der Kern vor dem Bau von `g` auf diesem Planeten verlangt (`GebaeudeRegel.braucht`), rekursiv –
    /// KI-Weg-Lauf 28.10.2026: „werft braucht raumhafen Stufe 1“ stand in keinem Weg.
    fn bau_voraussetzungen(&self, pid: usize, g: Gebaeude) -> Vec<(String, Value)> {
        let p = &self.world.planeten[pid];
        let k = p.koord.to_string();
        let mut aus = Vec::new();
        for (b, st) in self.world.regeln.gebaeude.get(&g).map(|r| r.braucht.clone()).unwrap_or_default() {
            if p.gebaeude[b.idx()] >= st { continue; }
            if b != g { aus.extend(self.bau_voraussetzungen(pid, b)); }
            aus.push((format!("{b} Stufe {st} auf {k} (Voraussetzung für {g}, jetzt {})", p.gebaeude[b.idx()]), json!({"typ":"bauen","planet":k,"gebaeude":b.name()})));
        }
        aus
    }

    /// Was vor dem Fertigen von `e` auf diesem Planeten fehlt: Werftstufe (`EinheitRegel.werft`) und weitere Gebäude
    /// (`braucht`), jeweils samt deren Voraussetzungen (KI-Weg-Lauf 28.10.2026: „spionagesonde braucht Werft Stufe 1“).
    fn fertig_voraussetzungen(&self, pid: usize, e: Einheit) -> Vec<(String, Value)> {
        let Some(r) = self.world.regeln.einheiten.get(&e) else { return Vec::new() };
        let p = &self.world.planeten[pid];
        let k = p.koord.to_string();
        let mut noetig: Vec<(Gebaeude, u8)> = r.braucht.iter().map(|(g, n)| (*g, *n)).collect();
        if r.werft > 0 { noetig.push((Gebaeude::Werft, r.werft)); }
        let mut aus = Vec::new();
        for (g, n) in noetig.into_iter().filter(|(g, n)| p.gebaeude[g.idx()] < *n) {
            aus.extend(self.bau_voraussetzungen(pid, g));
            aus.push((format!("{g} Stufe {n} auf {k} (Voraussetzung für {e}, jetzt {})", p.gebaeude[g.idx()]), json!({"typ":"bauen","planet":k,"gebaeude":g.name()})));
        }
        aus
    }

    /// Was vor der Forschung `f` fehlt: die Laborstufe auf der Heimatwelt samt deren Voraussetzungen
    /// (KI-Weg-Lauf 28.10.2026: „impulsantrieb braucht Labor Stufe 3“) und Spionagetechnik für die Überwachungszweige.
    fn forsch_voraussetzungen(&self, sid: SpielerId, f: Forschung) -> Vec<(String, Value)> {
        let sp = &self.world.spieler[sid as usize];
        let pid = sp.heimat as usize;
        let p = &self.world.planeten[pid];
        let k = p.koord.to_string();
        let mut aus = Vec::new();
        if let Some(lab) = self.world.regeln.forschung.get(&f).map(|r| r.labor).filter(|l| p.gebaeude[Gebaeude::Labor.idx()] < *l) {
            aus.extend(self.bau_voraussetzungen(pid, Gebaeude::Labor));
            aus.push((format!("labor Stufe {lab} auf {k} (Voraussetzung für {f}, jetzt {})", p.gebaeude[Gebaeude::Labor.idx()]), json!({"typ":"bauen","planet":k,"gebaeude":"labor"})));
        }
        if matches!(f, Forschung::Ueberwachungstechnik | Forschung::Abschirmtechnik) && sp.forschung[Forschung::Spionagetechnik.idx()] == 0 {
            aus.push((format!("spionagetechnik Stufe 1 erforschen (Voraussetzung für {f})"), json!({"typ":"forschen","forschung":"spionagetechnik","planet":k})));
        }
        aus
    }

    /// Eigener Planet mit der höchsten Stufe des Gebäudes (dort weiterbauen), sonst die Heimatwelt.
    fn bester_planet(&self, sid: SpielerId, g: Gebaeude) -> String {
        let sp = &self.world.spieler[sid as usize];
        sp.planeten.iter().map(|p| &self.world.planeten[*p as usize]).max_by_key(|p| p.gebaeude[g.idx()])
            .map(|p| p.koord.to_string()).unwrap_or_else(|| self.heimat(sid))
    }

    fn ziel_stufe(&self, sid: SpielerId, rolle: Rolle, ich: &str, lage: &Value) -> Option<Value> {
        let ns = lage.get("naechste_stufe").filter(|v| v.is_object())?;
        let sp = &self.world.spieler[sid as usize];
        let regel = self.world.regeln.stufen.get((sp.stufe as usize).checked_sub(1)?)?;
        let heim = self.heimat(sid);
        let mut w = Weg::neu(rolle, ich);
        // Ohne Doktrin hat der Stratege keinen eigenen Schritt und griff im KI-Weg-Lauf (28.10.2026) zu beliebigen
        // Aktionen; die Doktrin hält das Ziel für alle Rollen fest.
        if lage["doktrin"].as_str().is_none_or(|d| d.trim().is_empty()) {
            let anteile: serde_json::Map<String, Value> = lage["toepfe"].as_object().map(|t| t.iter().map(|(k, v)| (k.clone(), v["anteil"].clone())).collect()).unwrap_or_default();
            let text = format!("Ziel: {} erreichen und die nächste Kolonie gründen. Verwalter: Bau und Forschung nach agenda.ziele (stufe, kolonie). Feldherr: Nachbarn aufklären, Eskorte für den Kolonieflug. Diplomat: Allianz oder Nichtangriffspakte mit Nachbarn.",
                ns["name"].as_str().unwrap_or("die nächste Stufe"));
            w.offen("Doktrin des Reichs festlegen: Ziel und Aufgaben der Rollen (doktrin; Anteile der Töpfe zusammen 100)".into(), &["stratege"], None,
                vec![json!({"typ":"doktrin","anteile":anteile,"text":text})]);
        } else { w.fertig(); }
        let bed = ns["bedingungen"].as_array().cloned().unwrap_or_default();
        for b in &bed {
            if b["erfuellt"] == true { w.fertig(); continue; }
            let text = b["text"].as_str().unwrap_or("").to_string();
            if let Some((g, _)) = regel.gebaeude.iter().find(|(g, st)| text.starts_with(&format!("{g} Stufe {st} "))) {
                let ort = self.bester_planet(sid, *g);
                if let Some(pid) = self.world.belegung.get(&ort.parse::<Koord>().unwrap_or(Koord::neu(0, 0, 0))) { w.voraus(self.bau_voraussetzungen(*pid as usize, *g)); }
                w.offen(text, &["verwalter"], None, vec![json!({"typ":"bauen","planet":ort,"gebaeude":g.name()})]);
            } else if let Some((f, _)) = regel.forschung.iter().find(|(f, st)| text.starts_with(&format!("{f} Stufe {st} "))) {
                w.voraus(self.forsch_voraussetzungen(sid, *f));
                w.offen(text, &["verwalter"], None, vec![json!({"typ":"forschen","forschung":f.name(),"planet":heim})]);
            } else if text.contains("Kolonien") {
                w.offen(format!("{text} – Weg im Ziel kolonie"), &["verwalter", "feldherr"], None, vec![]);
            } else {
                w.offen(text, &["verwalter"], None, vec![]);
            }
        }
        for h in ns["hinweise"].as_array().into_iter().flatten().filter_map(Value::as_str) {
            w.offen(h.to_string(), &["verwalter"], None, vec![]);
        }
        // Aufstiegskosten auf der Heimatwelt.
        let bestand = lage["planeten"].as_array().and_then(|p| p.iter().find(|p| p["koord"] == heim.as_str())).map(|p| p["bestand"].clone()).unwrap_or_default();
        let fehlt: Vec<String> = ns["kosten"].as_object().into_iter().flatten()
            .filter_map(|(g, m)| { let d = m.as_f64().unwrap_or(0.0).ceil() as i64 - gut_menge(&bestand, g); (d > 0).then(|| format!("{d} {g}")) }).collect();
        let alle = !bed.is_empty() && bed.iter().all(|b| b["erfuellt"] == true);
        if !fehlt.is_empty() {
            w.offen(format!("Aufstiegskosten fehlen auf der Heimatwelt {heim}: {} – Ziel rohstoffe (Markt, Allianz, Raub) oder Transport von Kolonien", fehlt.join(", ")), &["verwalter", "diplomat"], None, vec![]);
        }
        if alle {
            let seit = ns["erfuellt_seit_stunden"].as_i64().unwrap_or(0);
            let halten = ns["haltezeit_stunden"].as_i64().unwrap_or(0);
            if seit < halten {
                w.offen(format!("Alle Bedingungen erfüllt: noch {} Stunden halten, dann aufsteigen", halten - seit), &["stratege"], None, vec![]);
            } else if fehlt.is_empty() {
                let wer: Vec<&str> = rollen_fuer("stufenaufstieg").iter().map(|r| r.name()).collect();
                w.offen(format!("Aufsteigen zu {}", ns["name"].as_str().unwrap_or("der nächsten Stufe")), &wer, None, vec![json!({"typ":"stufenaufstieg"})]);
            }
        }
        let erfuellt = bed.iter().filter(|b| b["erfuellt"] == true).count();
        Some(w.ziel("stufe", format!("Zivilisationsstufe {}", ns["name"].as_str().unwrap_or("")), format!("{erfuellt}/{} Bedingungen erfüllt", bed.len())))
    }

    /// Weg zur nächsten Kolonie aus `lage.kolonie_weg` (Kern) mit Vorlagen; `platz`: vom Dysonprojekt zugeteilter Platz.
    fn ziel_kolonie(&self, sid: SpielerId, rolle: Rolle, ich: &str, lage: &Value, platz: Option<&str>) -> Option<Value> {
        let weg = lage["kolonie_weg"].as_array()?;
        let k = &lage["kolonien"];
        if weg.is_empty() { return None; }
        let sp = &self.world.spieler[sid as usize];
        let heim = self.heimat(sid);
        let hp = &self.world.planeten[sp.heimat as usize];
        // Sonden und Werft können auf einer Kolonie stehen (KI-Weg-Lauf 28.10.2026: Heimat ohne Werft, Sonden im Dyson-System).
        let (sonden_start, sonden) = self.sonden_planet(sid);
        let (werft_ort, werft_pid) = self.werft_planet(sid);
        let erkundet_frei: Vec<String> = lage["erkundet"].as_array().into_iter().flatten().filter(|e| e["frei"] == true).filter_map(|e| e["koord"].as_str().map(str::to_string)).collect();
        // Ziel: zugeteilter Dysonplatz vor einem erkundeten freien Platz, sonst ein öffentlich freier Platz im eigenen System.
        let ziel = platz.map(str::to_string).or_else(|| erkundet_frei.first().cloned()).or_else(|| {
            (1..=self.world.regeln.welt.plaetze_je_system).map(|pos| Koord::neu(hp.koord.sektor, hp.koord.system, pos))
                .find(|x| self.world.belegung.get(x).is_none_or(|p| !self.world.planet_aktiv(*p as usize))).map(|x| x.to_string())
        });
        let mut w = Weg::neu(rolle, ich);
        // Kette: Vorlagen nur für die gerade machbaren Glieder (KI-Weg-Lauf 28.10.2026: „Kolonieschiff fertigen“ stand ganz
        // oben in der Liste des Feldherrn, obwohl die Forschung dafür fehlte; er tat dann gar nichts). Das Erkunden des
        // Zielplatzes geht unabhängig davon.
        let mut kette_offen = false;
        // Der Kern verlangt einen eigenen Spionagebericht vom konkreten Zielplatz; der Kolonieweg des Kerns zählt dagegen
        // jeden erkundeten freien Platz (KI-Weg-Lauf 28.10.2026: „Ziel zuerst mit einer eigenen Planetensonde aufklären“).
        let ziel_offen = ziel.as_ref().filter(|z| !erkundet_frei.contains(*z)).cloned();
        if let Some(z) = &ziel_offen {
            // Spionage in einem fremden System verlangt erst dessen Karte (KI-Weg-Lauf 28.10.2026: die Siedler der Allianz
            // schickten zuerst system_erkunden, der Weg nannte nur die Spionage). Laufende Aufklärung abwarten statt doppelt
            // zu fliegen.
            let zk: Option<Koord> = z.parse().ok();
            let kartiert = zk.is_none_or(|k| !self.world.aufklaerungsregeln() || self.world.system_erfasst(sid, k));
            let wer = &["feldherr", "verwalter"];
            if let Some(h) = zk.and_then(|k| self.aufklaerung_unterwegs(sid, k, kartiert)) {
                w.offen(format!("Aufklärung für {z} ist unterwegs (Ankunft in etwa {h} Stunden): abwarten, nicht erneut senden"), wer, None, vec![]);
            } else if sonden == 0 {
                w.voraus(self.fertig_voraussetzungen(werft_pid, Einheit::Spionagesonde));
                w.offen(format!("Spionagesonden fertigen, um den Zielplatz {z} aufzuklären"), wer, None, vec![json!({"typ":"fertigen","planet":werft_ort,"einheit":"spionagesonde","anzahl":2})]);
            } else if !kartiert {
                let k = zk.unwrap_or(hp.koord);
                w.offen(format!("System {}:{} mit system_erkunden kartieren (Spionagesonde), danach Zielplatz {z} ausspähen – der Kolonieflug verlangt den eigenen Bericht", k.sektor, k.system), wer, None,
                    vec![json!({"typ":"flotte_senden","start":sonden_start,"ziel":z,"mission":"system_erkunden","schiffe":{"spionagesonde":1},"geschwindigkeit":1.0,"ladung":{},"haltedauer_stunden":0})]);
            } else {
                w.offen(format!("Zielplatz {z} mit eigener Planetensonde ausspähen (spionage) – der Kolonieflug verlangt den eigenen Bericht"), wer, None,
                    vec![json!({"typ":"flotte_senden","start":sonden_start,"ziel":z,"mission":"spionage","schiffe":{"spionagesonde":1},"geschwindigkeit":1.0,"ladung":{},"haltedauer_stunden":0})]);
            }
        }
        for s in weg {
            if s["erledigt"] == true { w.fertig(); continue; }
            let text = s["text"].as_str().unwrap_or("").to_string();
            let unabhaengig = text.starts_with("Freier Platz erkundet") || s["hinweis"] == true;
            if kette_offen && !unabhaengig {
                w.offen(format!("danach: {text}"), &["verwalter", "feldherr"], None, vec![]);
                continue;
            }
            if !unabhaengig { kette_offen = true; }
            if s["hinweis"] == true {
                if let Some(z) = &ziel {
                    let i = w.offen(format!("{text}: werkzeug kolonieplan für {z}"), &["verwalter", "feldherr"], None, vec![]);
                    if w.schritte[i]["du"] == true {
                        w.schritte[i]["werkzeug"] = json!({"typ":"kolonieplan","start":heim,"ziel":z,"schiffe":{"kolonieschiff":1},"ladung":s.get("mindestfracht").cloned().unwrap_or(json!({})),"aufbau":[]});
                    }
                }
                continue;
            }
            let mut vorlagen = vec![];
            let mut wer: &[&str] = &["verwalter"];
            if text.starts_with("Zivilisationsstufe") {
                w.offen(format!("{text} – Weg im Ziel stufe"), wer, None, vec![]);
                continue;
            } else if text.contains("erlaubt die") {
                if let Some(f) = k["forschung"].as_str() {
                    if let Some(f) = Forschung::aus_name(f) { w.voraus(self.forsch_voraussetzungen(sid, f)); }
                    vorlagen.push(json!({"typ":"forschen","forschung":f,"planet":heim}));
                }
            } else if text.starts_with("Raumhafen") {
                w.voraus(self.bau_voraussetzungen(sp.heimat as usize, Gebaeude::Raumhafen));
                vorlagen.push(json!({"typ":"bauen","planet":heim,"gebaeude":"raumhafen"}));
            } else if text.starts_with("Orbitalwerft") {
                w.voraus(self.bau_voraussetzungen(sp.heimat as usize, Gebaeude::Orbitalwerft));
                vorlagen.push(json!({"typ":"bauen","planet":heim,"gebaeude":"orbitalwerft"}));
            } else if text.starts_with("Werft") {
                let ort = self.bester_planet(sid, Gebaeude::Werft);
                if let Some(pid) = self.world.belegung.get(&ort.parse::<Koord>().unwrap_or(Koord::neu(0, 0, 0))) { w.voraus(self.bau_voraussetzungen(*pid as usize, Gebaeude::Werft)); }
                vorlagen.push(json!({"typ":"bauen","planet":ort,"gebaeude":"werft"}));
            } else if text.starts_with("Kolonieschiff gefertigt") {
                // Forschung, die der Kern beim Fertigen prüft (KI-Weg-Lauf 27.10.2026: „kolonieschiff benötigt
                // impulsantrieb Stufe 2“ stand in keinem Weg), samt deren Laborstufe.
                let fehlt: Vec<(Forschung, u8)> = self.world.regeln.flottendoktrin.get(&Einheit::Kolonieschiff)
                    .map(|p| p.voraussetzungen.iter().filter(|(f, n)| sp.forschung[f.idx()] < **n).map(|(f, n)| (*f, *n)).collect()).unwrap_or_default();
                let warten = !fehlt.is_empty();
                for (f, n) in fehlt {
                    w.voraus(self.forsch_voraussetzungen(sid, f));
                    w.offen(format!("{f} Stufe {n} erforschen (Voraussetzung des Kolonieschiffs, jetzt {})", sp.forschung[f.idx()]), &["verwalter"], None,
                        vec![json!({"typ":"forschen","forschung":f.name(),"planet":heim})]);
                }
                wer = &["verwalter", "feldherr"];
                if warten {
                    w.offen(format!("danach: {text} – sobald die Forschung steht"), wer, None, vec![]);
                    continue;
                }
                vorlagen.push(json!({"typ":"fertigen","planet":heim,"einheit":"kolonieschiff","anzahl":1}));
            } else if text.starts_with("Freier Platz erkundet") {
                if ziel_offen.is_some() { continue; }
                wer = &["verwalter", "feldherr"];
                if let (Some(z), true) = (&ziel, sonden > 0) {
                    vorlagen.push(json!({"typ":"flotte_senden","start":sonden_start,"ziel":z,"mission":"spionage","schiffe":{"spionagesonde":1},"geschwindigkeit":1.0,"ladung":{},"haltedauer_stunden":0}));
                } else if sonden == 0 {
                    vorlagen.push(json!({"typ":"fertigen","planet":werft_ort,"einheit":"spionagesonde","anzahl":2}));
                }
            } else if text.starts_with("Kolonieschiff, bewaffnete") {
                wer = &["verwalter", "feldherr"];
                w.offen(format!("{text}; Mindestfracht {}", s["mindestfracht"]), wer, None, vec![]);
                continue;
            } else if text.starts_with("Flotte mit Kolonieschiff") {
                wer = &["verwalter", "feldherr"];
                // Fertiger Kolonieflug (KI-Weg-Lauf 28.10.2026: die Siedler-Reiche der Allianz klärten ihre Dysonplätze auf,
                // den Flug mit Kolonieschiff, Eskorte, Fracht und Siedlern mussten sie bisher selbst zusammensetzen).
                let begleit = Einheit::ALLE.iter().filter(|e| e.ist_schiff() && hp.einheiten[e.idx()] > 0)
                    .filter(|e| !matches!(e, Einheit::Spionagesonde | Einheit::Kolonieschiff | Einheit::GrossesKolonieschiff))
                    .max_by_key(|e| self.world.regeln.einheiten.get(e).map(|r| r.angriff).unwrap_or(0)).copied()
                    .filter(|e| self.world.regeln.einheiten.get(e).is_some_and(|r| r.angriff > 0));
                let fracht = weg.iter().find_map(|x| x.get("mindestfracht")).cloned().unwrap_or(json!({}));
                let flug = match (&ziel, begleit, hp.einheiten[Einheit::Kolonieschiff.idx()] > 0) {
                    (Some(z), Some(b), true) if ziel_offen.is_none() => {
                        let mut schiffe = serde_json::Map::new();
                        schiffe.insert("kolonieschiff".into(), json!(1));
                        schiffe.insert(b.name().into(), json!(1));
                        vec![json!({"typ":"flotte_senden","start":heim,"ziel":z,"mission":"kolonisieren","schiffe":schiffe,"ladung":fracht,
                            "siedler_volk":sp.volk.name(),"geschwindigkeit":1.0,"haltedauer_stunden":0})]
                    }
                    _ => vec![],
                };
                let i = w.offen(match &ziel { Some(z) => format!("{text}: Ziel {z} (vorher werkzeug kolonieplan prüfen; flotte_senden mission kolonisieren mit Kolonieschiff, bewaffneter Eskorte, Mindestfracht und Siedlern)"), None => text }, wer, None, flug);
                if let (Some(z), true) = (&ziel, w.schritte[i]["du"] == true) {
                    w.schritte[i]["werkzeug"] = json!({"typ":"kolonieplan","start":heim,"ziel":z,"schiffe":{"kolonieschiff":1},"ladung":{},"aufbau":[]});
                }
                continue;
            }
            w.offen(text, wer, None, vorlagen);
        }
        let titel = match platz { Some(p) => format!("Neue Kolonie (Dysonplatz {p})"), None => "Neue Kolonie".to_string() };
        Some(w.ziel("kolonie", titel, format!("{} von {} Kolonien, Weg {}/{} erledigt", k["anzahl"], k["erlaubt"], weg.iter().filter(|s| s["erledigt"] == true).count(), weg.len())))
    }

    /// Rohstoffe von anderen beziehen: Markt, Allianz, Handel mit Reichen, Raub (aus eigenen Spionageberichten).
    fn ziel_rohstoffe(&self, sid: SpielerId, rolle: Rolle, ich: &str, lage: &Value) -> Value {
        let heim = self.heimat(sid);
        let bestand = lage["planeten"].as_array().and_then(|p| p.iter().find(|p| p["koord"] == heim.as_str())).map(|p| p["bestand"].clone()).unwrap_or_default();
        // Bedarf: Aufstiegskosten und offener Dysonanteil.
        let mut bedarf: std::collections::BTreeMap<String, i64> = Default::default();
        for (g, m) in lage["naechste_stufe"]["kosten"].as_object().into_iter().flatten() {
            *bedarf.entry(g.clone()).or_default() += m.as_f64().unwrap_or(0.0).ceil() as i64;
        }
        // Offener Dysonanteil aus dem Weltzustand (KI-Weg-Lauf 28.10.2026: aus der Lage gelesen blieb er 0 – Sonnenrat8 hatte
        // 92 133 je Rohstoff für einen Anteil von 5 Mio. und bekam „kein akuter Bedarf“).
        let d = &self.world.ausbau.dyson;
        for p in d.projekte.values().filter(|p| p.phase.offen()) {
            if let Some(b) = p.mitglieder.get(&sid) {
                for (i, g) in kern::dyson::ROHSTOFFE.iter().enumerate() {
                    *bedarf.entry(g.name().to_string()).or_default() += (d.anteil - b.eingezahlt[i] / M).max(0);
                }
            }
        }
        let fehlt: Vec<(String, i64)> = bedarf.iter().map(|(g, n)| (g.clone(), n - gut_menge(&bestand, g))).filter(|(_, d)| *d > 0).collect();
        let mut w = Weg::neu(rolle, ich);
        let fehlt_text = fehlt.iter().map(|(g, d)| format!("{d} {g}")).collect::<Vec<_>>().join(", ");
        for (g, d) in &fehlt {
            if let Some(preis) = lage["markt"]["preise"][g]["verkauf_ab"].as_f64() {
                w.offen(format!("Markt: {d} {g} kaufen ab {preis} Credits je Einheit (Credits: {})", lage["credits"]), &["verwalter"], None,
                    vec![json!({"typ":"markt_order","planet":heim,"gut":g,"seite":"kauf","menge":d,"preis":preis})]);
            }
        }
        let freunde: Vec<&str> = lage["allianz"]["mitglieder"].as_array().into_iter().flatten().filter_map(Value::as_str).collect();
        if !fehlt.is_empty() {
            if lage["allianz"].is_object() {
                w.offen(format!("Allianz um {fehlt_text} bitten: pinwand_schreiben bereich allianz (art anfrage, an alle) – Mitglieder liefern per flotte_senden transport, intern_anbieten/intern_kaufen oder schenken"), &["diplomat", "stratege"], None, vec![]);
            }
            w.offen(format!("Mit anderen Reichen tauschen: brief_senden oder vertrag_anbieten mit Lieferung ({fehlt_text})"), &["diplomat"], None, vec![]);
        }
        // Raubziele: eigene Spionageberichte fremder Reiche (nicht der eigenen Allianz), nach möglicher Beute.
        // Berichte kommen neueste zuerst: je Ziel nur der neueste.
        let mut gesehen = std::collections::BTreeSet::new();
        let mut ziele: Vec<(i64, String)> = lage["berichte"].as_array().into_iter().flatten()
            .filter(|b| b["art"] != "erkundung" && b["besitzer"].as_str().is_some_and(|n| n != ich && !freunde.contains(&n)))
            .filter(|b| gesehen.insert(b["ziel"].as_str().unwrap_or("").to_string()))
            .filter_map(|b| {
                let beute = ["erz", "kristall", "deuterium"].iter().map(|g| gut_menge(&b["bestand"], g)).sum::<i64>();
                (beute > 0).then(|| (beute, format!("{} ({}, Bericht vor {} h): {} Erz, {} Kristall, {} Deuterium; Schiffe {}, Verteidigung {}",
                    b["ziel"].as_str().unwrap_or("?"), b["besitzer"].as_str().unwrap_or("?"), b["alter_stunden"], gut_menge(&b["bestand"], "erz"),
                    gut_menge(&b["bestand"], "kristall"), gut_menge(&b["bestand"], "deuterium"), stueck(&b["schiffe"]), stueck(&b["verteidigung"]))))
            }).collect();
        ziele.sort_by(|a, b| b.0.cmp(&a.0));
        // Flottenvorlagen ab der Heimatwelt (KI-Weg-Lauf 28.10.2026: ohne Vorlage schickte der Feldherr nur Meldungen).
        let sp = &self.world.spieler[sid as usize];
        let hp = &self.world.planeten[sp.heimat as usize];
        let flug = |ziel: &str, mission: &str, schiffe: Value| json!({"typ":"flotte_senden","start":heim,"ziel":ziel,"mission":mission,"schiffe":schiffe,"geschwindigkeit":1.0,"ladung":{},"haltedauer_stunden":0});
        let (sonden_start, sonden) = self.sonden_planet(sid);
        let (werft_ort, werft_pid) = self.werft_planet(sid);
        let sondenflug = |ziel: &str, mission: &str, n: i64| json!({"typ":"flotte_senden","start":sonden_start,"ziel":ziel,"mission":mission,"schiffe":{"spionagesonde":n.min(sonden.max(1))},"geschwindigkeit":1.0,"ladung":{},"haltedauer_stunden":0});
        let mut angriff = serde_json::Map::new();
        for e in Einheit::ALLE.iter().filter(|e| e.ist_schiff() && **e != Einheit::Spionagesonde && hp.einheiten[e.idx()] > 0) {
            let r = self.world.regeln.einheiten.get(e);
            if r.is_some_and(|r| r.angriff > 0 || r.ladung > 0) && !matches!(e, Einheit::Kolonieschiff | Einheit::GrossesKolonieschiff) { angriff.insert(e.name().into(), json!(hp.einheiten[e.idx()])); }
        }
        for (_, t) in ziele.iter().take(3) {
            let k = t.split(' ').next().unwrap_or("").to_string();
            let v = if angriff.is_empty() { vec![] } else { vec![flug(&k, "angriff", Value::Object(angriff.clone()))] };
            w.offen(format!("Raubziel {t} – erst werkzeug kampfsimulator (Vorlage: Kampf- und Frachtschiffe der Heimatwelt, anpassen), Bericht älter als 12 h neu aufklären (spionage), dann flotte_senden mission angriff mit genug Frachtraum"), &["feldherr"], None, v);
        }
        if ziele.len() < 3 {
            // Nächste fremde Planeten (öffentlich wie in der Galaxie): erst das System kartieren, dann ausspähen.
            let berichtet: Vec<&str> = lage["berichte"].as_array().into_iter().flatten().filter_map(|b| b["ziel"].as_str()).collect();
            let (sk, sy) = (hp.koord.sektor, hp.koord.system);
            let mut nah: Vec<(u8, Koord)> = self.world.belegung.iter()
                .filter(|(k, p)| k.sektor == sk && k.system.abs_diff(sy) <= 2 && self.world.planet_aktiv(**p as usize))
                .filter(|(_, p)| { let b = self.world.planeten[**p as usize].besitzer; b != sid && !freunde.contains(&self.world.spieler[b as usize].name.as_str()) })
                .filter(|(k, _)| !berichtet.contains(&k.to_string().as_str()))
                .map(|(k, _)| (k.system.abs_diff(sy), *k)).collect();
            nah.sort();
            for (_, k) in nah.into_iter().take(2) {
                let ks = k.to_string();
                let besitzer = self.world.belegung.get(&k).map(|p| self.world.spieler[self.world.planeten[*p as usize].besitzer as usize].name.clone()).unwrap_or_default();
                let kartieren = self.world.aufklaerungsregeln() && !self.world.system_erfasst(sid, k);
                let (text, v) = if sonden == 0 {
                    w.voraus(self.fertig_voraussetzungen(werft_pid, Einheit::Spionagesonde));
                    (format!("Mögliches Raubziel {ks} ({besitzer}): erst Spionagesonden fertigen"), json!({"typ":"fertigen","planet":werft_ort,"einheit":"spionagesonde","anzahl":3}))
                } else if kartieren {
                    (format!("Mögliches Raubziel {ks} ({besitzer}): System {}:{} erst mit system_erkunden kartieren, dann spionage", k.sektor, k.system), sondenflug(&ks, "system_erkunden", 1))
                } else {
                    (format!("Mögliches Raubziel {ks} ({besitzer}): mit spionage aufklären – der Bericht zeigt Beute und Abwehr"), sondenflug(&ks, "spionage", 2))
                };
                w.offen(text, &["feldherr"], None, vec![v]);
            }
        }
        if ziele.is_empty() && sonden == 0 {
            w.offen("Raubziele aufklären: Nachbarsysteme mit flotte_senden mission system_erkunden kartieren, dann spionage auf fremde Planeten – Berichte (lage.berichte) zeigen Beute und Abwehr".into(), &["feldherr"], None, vec![]);
        }
        let titel = if fehlt.is_empty() { "Rohstoffe von anderen beziehen".to_string() } else { format!("Rohstoffe beschaffen: {fehlt_text}") };
        w.ziel("rohstoffe", titel, if fehlt.is_empty() { "kein akuter Bedarf für Aufstieg oder Dyson".into() } else { format!("Bedarf auf {heim}") })
    }

    fn ziel_allianz(&self, sid: SpielerId, rolle: Rolle, ich: &str, lage: &Value) -> Value {
        let mut w = Weg::neu(rolle, ich);
        let sp = &self.world.spieler[sid as usize];
        let Some(a) = sp.allianz.and_then(|aid| self.world.allianzen.iter().find(|a| a.id == aid)) else {
            let einladungen: Vec<&str> = lage["einladungen"].as_array().into_iter().flatten().filter_map(Value::as_str).collect();
            for e in einladungen.iter().take(3) {
                w.offen(format!("Einladung der Allianz {e} annehmen (allianz_beitreten) – Allianzen teilen Verteidigung, Handel und Großprojekte"), &["diplomat"], None, vec![json!({"typ":"allianz_beitreten","allianz":e})]);
            }
            if einladungen.is_empty() {
                let dyson = self.world.dyson_regeln().map(|r| format!(" – auch Voraussetzung der Dysonsphäre (mindestens {} Mitglieder mit Planet im selben System)", r.mitglieder_min)).unwrap_or_default();
                w.offen(format!("Allianz gründen (allianz_gruenden) und Nachbarn einladen (allianz_einladen) oder bei einer bestehenden Allianz anfragen (brief_senden){dyson}"), &["diplomat"], None,
                    vec![json!({"typ":"allianz_gruenden","name":format!("Bund von {ich}").chars().take(30).collect::<String>()})]);
            }
            w.offen("Nichtangriffspakt mit starken Nachbarn (diplomatie_anfrage bzw. vertrag_anbieten) schützt den Aufbau".into(), &["diplomat"], None, vec![]);
            return w.ziel("allianz", "Allianz und Diplomatie".into(), "ohne Allianz".into());
        };
        let leitung = self.world.allianz_leitung(sid, a.id);
        let hilfe = lage["kommunikation"]["allianzbereich"]["hilfe"].as_array().map(|h| h.iter().filter(|h| h["status"] == "offen" && h["von"] != ich).count()).unwrap_or(0);
        if hilfe > 0 {
            w.offen(format!("{hilfe} offene Hilferufe der Allianz: Kampfflotte mit flotte_senden mission halten schicken (Agenda allianz_hilferuf)"), &["feldherr"], None, vec![]);
        }
        let wand = self.allianz_pinwand_kurz(sid).as_array().map(Vec::len).unwrap_or(0);
        if wand > 0 {
            w.offen(format!("{wand} offene Karten der Allianz-Pinwand an dieses Reich: pinwand_lesen bereich allianz, antworten oder erledigen"), &["stratege", "diplomat", "feldherr", "verwalter"], None, vec![]);
        }
        // Offene Einladungen zuerst abwarten (KI-Weg-Lauf 28.10.2026: der Diplomat verschickte 45 Einladungen in 20 Minuten).
        let offen: Vec<String> = a.eingeladen.iter().map(|s| self.world.spieler[*s as usize].name.clone()).collect();
        if !offen.is_empty() {
            w.offen(format!("{} Einladungen sind offen ({}): auf Antwort warten, nicht erneut einladen; per brief_senden um Beitritt werben", offen.len(), offen.iter().take(6).cloned().collect::<Vec<_>>().join(", ")), &["diplomat"], None, vec![]);
        } else if let Some(r) = self.world.dyson_regeln().filter(|r| a.mitglieder.len() < r.mitglieder_min) {
            w.offen(format!("Allianz auf {} Mitglieder bringen (jetzt {}) – Voraussetzung der Dysonsphäre: Reiche einladen (allianz_einladen spieler)", r.mitglieder_min, a.mitglieder.len()), &["diplomat"], None, vec![]);
        }
        let stand = format!("Mitglied von {} ({} Mitglieder{})", a.name, a.mitglieder.len(), if leitung { ", Allianzleitung" } else { "" });
        w.ziel("allianz", "Allianz und Diplomatie".into(), stand)
    }

    /// Dysonsphäre der eigenen Allianz als Kette für alle Mitglieder; Zielsystem ist das offene Projekt oder das System
    /// mit den meisten Allianzplaneten. Freie Plätze werden reihum auf Mitglieder ohne Planet dort verteilt.
    fn ziel_dyson(&self, sid: SpielerId, rolle: Rolle, ich: &str) -> Option<(Value, DysonWeg)> {
        let r = self.world.dyson_regeln()?;
        let welt = &self.world;
        let mut w = Weg::neu(rolle, ich);
        let mut weg = DysonWeg::default();
        let titel = "Dysonsphäre der Allianz".to_string();
        // Ohne Allianz kein Dyson-Ziel: Gründen und Beitreten stehen im Ziel allianz (KI-Weg-Lauf 27.10.2026: der Stratege
        // eines Einzelreichs versuchte Zug um Zug dyson_beitreten).
        let a = welt.spieler[sid as usize].allianz.and_then(|aid| welt.allianzen.iter().find(|a| a.id == aid))?;
        let n = welt.regeln.welt.plaetze_je_system;
        let mitglieder: Vec<SpielerId> = a.mitglieder.iter().copied().filter(|m| welt.spieler_aktiv(*m)).collect();
        let name = |s: SpielerId| welt.spieler[s as usize].name.clone();
        let leitung: Option<String> = mitglieder.iter().find(|m| welt.allianz_leitung(**m, a.id)).map(|m| name(*m));
        let ich_leitung = welt.allianz_leitung(sid, a.id);
        let wer_leitung = |typ: &str| -> Vec<&'static str> { rollen_fuer(typ).iter().map(|r| r.name()).collect() };
        let projekt = welt.ausbau.dyson.projekte.values().filter(|p| p.allianz == a.id && p.phase != kern::dyson::DysonPhase::Abgebrochen)
            .max_by_key(|p| (p.phase.offen(), p.id));
        if let Some(p) = projekt.filter(|p| p.phase == kern::dyson::DysonPhase::Vollendet) {
            w.fertig();
            return Some((w.ziel("dyson", titel, format!("Sphäre im System {} vollendet", p.koord())), weg));
        }
        let gesperrt = |sk: u8, sy: u8| welt.ausbau.dyson.projekte.values().any(|p| p.sektor == sk && p.system == sy && p.allianz != a.id && p.phase.offen())
            || welt.ausbau.dyson.projekte.values().any(|p| p.sektor == sk && p.system == sy && p.phase == kern::dyson::DysonPhase::Abgebrochen && p.gesperrt_bis > welt.zeit);
        // Zielsystem.
        let (sk, sy) = match projekt.filter(|p| p.phase.offen()) {
            Some(p) => (p.sektor, p.system),
            None => {
                let mut zahl: std::collections::BTreeMap<(u8, u8), (usize, bool)> = Default::default();
                for m in &mitglieder {
                    for pid in &welt.spieler[*m as usize].planeten {
                        if !welt.planet_aktiv(*pid as usize) { continue; }
                        let k = welt.planeten[*pid as usize].koord;
                        let e = zahl.entry((k.sektor, k.system)).or_default();
                        e.0 += 1;
                        e.1 |= *m == sid;
                    }
                }
                let (best, _) = zahl.into_iter().filter(|((sk, sy), _)| !gesperrt(*sk, *sy)).max_by_key(|(k, (z, eigen))| (*z, *eigen, std::cmp::Reverse(*k)))?;
                best
            }
        };
        let sys = format!("{sk}:{sy}");
        // Plätze im Zielsystem (öffentlich wie in der Galaxie).
        let mut frei = Vec::new();
        let mut fremd = Vec::new();
        let mut mit_planet: Vec<SpielerId> = Vec::new();
        for pos in 1..=n {
            let k = Koord::neu(sk, sy, pos);
            match welt.belegung.get(&k).map(|p| *p as usize).filter(|p| welt.planet_aktiv(*p)) {
                None => frei.push(k.to_string()),
                Some(pl) => {
                    let b = welt.planeten[pl].besitzer;
                    if mitglieder.contains(&b) { if !mit_planet.contains(&b) { mit_planet.push(b); } } else { fremd.push((k.to_string(), name(b))); }
                }
            }
        }
        let allianz_plaetze = n as usize - frei.len() - fremd.len();
        // 1. Mitgliederzahl der Allianz.
        if mitglieder.len() < r.mitglieder_min {
            let kandidaten: Vec<&str> = fremd.iter().map(|(_, b)| b.as_str()).collect();
            w.offen(format!("Allianz hat {} von {} nötigen Mitgliedern: Reiche einladen (allianz_einladen){}", mitglieder.len(), r.mitglieder_min,
                if kandidaten.is_empty() { String::new() } else { format!(", zuerst die Besitzer im Zielsystem: {}", kandidaten.join(", ")) }),
                &["diplomat"], None, kandidaten.first().map(|k| vec![json!({"typ":"allianz_einladen","spieler":k})]).unwrap_or_default());
        } else { w.fertig(); }
        // 2. Vorschlag (geht ab einem Allianzplaneten im System; danach Plätze füllen und einzahlen).
        let offen_proj = projekt.filter(|p| p.phase.offen());
        match offen_proj {
            // Ein Projekt bindet das System; vorschlagen erst, wenn die Allianz absehbar genug Mitglieder hat (KI-Weg-Lauf
            // 28.10.2026: ein Einzelreich gründete eine Ein-Reich-Allianz und schlug sofort vor).
            None if mitglieder.len() * 2 < r.mitglieder_min => {
                w.offen(format!("Vorschlag für das System {sys} erst, wenn die Allianz wächst (jetzt {} von {} Mitgliedern)", mitglieder.len(), r.mitglieder_min), &["stratege", "diplomat"], None, vec![]);
            }
            None => {
                let von = if ich_leitung { None } else { leitung.as_deref() };
                w.offen(format!("Leitung schlägt das System {sys} vor (dyson_vorschlagen) – möglich ab einem Allianzplaneten dort, die Allianz hält {allianz_plaetze}/{n} Plätze; danach Plätze füllen und einzahlen"),
                    &wer_leitung("dyson_vorschlagen"), von, vec![json!({"typ":"dyson_vorschlagen","system":sys})]);
                if ich_leitung && mitglieder.len() >= r.mitglieder_min { weg.vorschlag = Some(sys.clone()); }
            }
            Some(_) => w.fertig(),
        }
        // 3. Plätze: freie reihum an Mitglieder ohne Planet im System (dann alle), fremde einladen oder zurückerobern.
        if frei.is_empty() && fremd.is_empty() { w.fertig(); }
        let mut reihe: Vec<SpielerId> = mitglieder.iter().copied().filter(|m| !mit_planet.contains(m)).collect();
        if reihe.is_empty() { reihe = mitglieder.clone(); }
        for (i, k) in frei.iter().enumerate() {
            let m = reihe[i % reihe.len()];
            if m == sid && weg.platz.is_none() { weg.platz = Some(k.clone()); }
            w.offen(format!("{k} ist frei: {} besiedelt den Platz (Ziel kolonie: erkunden, Kolonieschiff mit bewaffneter Eskorte und Mindestfracht)", name(m)), &["verwalter", "feldherr"], Some(&name(m)), vec![]);
        }
        for (k, b) in &fremd {
            w.offen(format!("{k} gehört {b}: in die Allianz einladen (allianz_einladen) oder zurückerobern (system_erkunden, spionage, boden_senden flug.mission bombardieren, kampfkolonisieren)"),
                &["diplomat", "feldherr"], None, vec![json!({"typ":"allianz_einladen","spieler":b})]);
        }
        // 4. Voraussetzungen je Mitglied: Stufe und Dauer der Mitgliedschaft.
        let tage = r.mitgliedschaft_min_tage;
        for m in &mitglieder {
            let s = welt.spieler[*m as usize].stufe;
            if s < r.stufe_min {
                w.offen(format!("{}: Zivilisationsstufe {} nötig (jetzt {s}) – Ziel stufe", name(*m), r.stufe_min), &["verwalter", "stratege"], Some(&name(*m)), vec![]);
            }
            let seit = welt.ausbau.dyson.mitglied_seit.get(m).filter(|(aid, _)| *aid == a.id).map(|(_, t)| (welt.zeit - t) / TAG);
            if seit.is_none_or(|t| t < tage) && tage > 0 {
                w.offen(format!("{}: erst {} von {tage} Spieltagen in der Allianz – bleiben", name(*m), seit.unwrap_or(0)), &["stratege"], Some(&name(*m)), vec![]);
            }
        }
        // 5. Projekt: beitreten, einzahlen, Bau starten bzw. halten.
        if let Some(p) = offen_proj {
            for m in &mit_planet {
                match p.mitglieder.get(m) {
                    None => { w.offen(format!("{}: dem Projekt {} beitreten (dyson_beitreten)", name(*m), p.id), &wer_leitung("dyson_beitreten"), Some(&name(*m)), vec![json!({"typ":"dyson_beitreten","projekt":p.id})]); }
                    Some(b) if b.eingezahlt.iter().any(|x| *x < welt.ausbau.dyson.anteil.saturating_mul(M)) => {
                        let rest: Vec<i64> = b.eingezahlt.iter().map(|x| (welt.ausbau.dyson.anteil - x / M).max(0)).collect();
                        // Einzahlen geht nur von eigenen Planeten im System (KI-Weg-Lauf 28.10.2026: Schildwacht2 hatte dort nur
                        // die frische Kolonie, die Rohstoffe lagen auf der Heimatwelt; es schickte dyson_einzahlen ohne Mengen).
                        let (einzahlen, transport) = if *m == sid { self.dyson_zahlweg(sid, p, &rest) } else { (vec![], vec![]) };
                        for (text, v) in transport {
                            let wer: Vec<&str> = if v.iter().any(|x| x["typ"] == "fertigen") { wer_leitung("fertigen") } else { wer_leitung("flotte_senden") };
                            w.offen(text, &wer, Some(&name(*m)), v);
                        }
                        let danach = if *m == sid && einzahlen.is_empty() { "danach: " } else { "" };
                        let warten = if danach.is_empty() { "" } else { " – erst wenn die Rohstoffe dort liegen; dyson_einzahlen jetzt nicht senden" };
                        w.offen(format!("{danach}{}: Anteil einzahlen, offen {}/{}/{} Erz/Kristall/Deuterium (dyson_einzahlen von einem eigenen Planeten im System {}:{}){warten}", name(*m), rest[0], rest[1], rest[2], p.sektor, p.system),
                            &wer_leitung("dyson_einzahlen"), Some(&name(*m)), einzahlen);
                    }
                    _ => w.fertig(),
                }
            }
            if let Some((text, v)) = self.dyson_hilfe(sid, p) {
                w.offen(text, &wer_leitung("flotte_senden"), Some(&name(sid)), v);
            }
            match p.phase {
                kern::dyson::DysonPhase::Sammeln => {
                    let bed = welt.dyson_bedingungen(p.id);
                    let offen_b: Vec<String> = bed.iter().filter(|b| !b.erfuellt).map(|b| format!("{} ({}/{})", b.text, b.ist, b.soll)).collect();
                    if offen_b.is_empty() {
                        let von = if ich_leitung { None } else { leitung.as_deref() };
                        w.offen("Alle Bedingungen erfüllt: Bau starten (dyson_bau_starten)".into(), &wer_leitung("dyson_bau_starten"), von, vec![json!({"typ":"dyson_bau_starten","projekt":p.id})]);
                    } else {
                        w.offen(format!("Vor dem Bau offen: {}", offen_b.join("; ")), &["stratege"], None, vec![]);
                    }
                }
                kern::dyson::DysonPhase::Bau => {
                    if p.ruht_seit.is_some() {
                        w.offen(format!("Bau ruht: {} – Plätze zurückgewinnen bzw. säumige Mitglieder einzahlen lassen (Agenda dyson_bau_ruht)", p.ruhe_grund), &["feldherr", "diplomat", "stratege"], None, vec![]);
                    } else {
                        w.offen(format!("Bau läuft: System {sys} bis zur Vollendung halten – Angriffe dort abwehren, Hilferufe mit flotte_senden mission halten beantworten, Verteidigung auf den Planeten im System"), &["feldherr"], None, vec![]);
                    }
                }
                _ => {}
            }
        }
        let stand = match offen_proj {
            Some(p) if p.phase == kern::dyson::DysonPhase::Bau => format!("System {sys}: Bau {} %{}", (p.bau_erledigt * 100 / r.bau_sekunden.max(1)).min(100), if p.ruht_seit.is_some() { ", ruht" } else { "" }),
            Some(p) => format!("System {sys}: Projekt {} sammelt, {allianz_plaetze}/{n} Plätze in Allianzhand, {} Mitglieder mit Planet dort (nötig {})", p.id, mit_planet.len(), r.mitglieder_min),
            None => format!("Zielsystem {sys}: {allianz_plaetze}/{n} Plätze in Allianzhand, {} Mitglieder mit Planet dort (nötig {}), noch kein Projekt", mit_planet.len(), r.mitglieder_min),
        };
        Some((w.ziel("dyson", titel, stand), weg))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agenten::{aktionskatalog, schema_fehler, REGIERUNG};

    fn spiel(name: &str, n: usize) -> (Game, Vec<Account>) {
        let mut g = Game::open(&std::env::temp_dir().join(format!("ziele-{name}-{}", token())), 27).unwrap();
        g.runtime.bots_enabled = false;
        g.runtime.admission_limit = 32;
        let mut konten = vec![];
        for i in 0..n {
            let name = format!("{name}{i}");
            let raw = g.register(&json!({"name":name}), "test-hash".into()).unwrap();
            let a = g.authenticate(raw["token"].as_str().unwrap()).unwrap();
            g.claim(&a, &json!({"mode":"gemischt","volk":"krath"})).unwrap();
            konten.push(g.account_by_name(&name).unwrap());
        }
        (g, konten)
    }

    /// Jede Vorlage in den Zielen muss die Eingabeprüfung und das strenge Schema der fragenden Rolle bestehen,
    /// und nur die zuständige Rolle bekommt Vorlagen.
    fn vorlagen_gueltig(g: &Game, a: &Account) -> Vec<String> {
        let sid = a.sid.unwrap();
        let mut ziele_gesehen = vec![];
        for r in REGIERUNG {
            let ag = g.agenda(a.id, sid, r);
            let k = aktionskatalog(&g.world, r, None).unwrap();
            let schema = |t: &str| k["aktionen"].as_array().unwrap().iter().find(|x| x["typ"] == t).map(|x| x["schema"].clone());
            for z in ag["ziele"]["liste"].as_array().unwrap() {
                ziele_gesehen.push(z["ziel"].as_str().unwrap().to_string());
                for s in z["schritte"].as_array().unwrap() {
                    assert!(s["text"].as_str().is_some_and(|t| !t.is_empty()) && s["wer"].is_array(), "{s}");
                    for v in s["vorlagen"].as_array().into_iter().flatten() {
                        assert_eq!(s["du"], true, "Vorlage ohne Zuständigkeit: {s}");
                        let t = v["typ"].as_str().unwrap();
                        let sch = schema(t).unwrap_or_else(|| panic!("{r}: Vorlage {t} gehört nicht zur Rolle"));
                        eingabe::aktion_pruefen(v, 1).unwrap_or_else(|e| panic!("{r}: {v} besteht die Eingabeprüfung nicht: {e}"));
                        if let Some(f) = schema_fehler(v, &sch, t) { panic!("{r}: {v} verletzt das Schema: {f}"); }
                    }
                }
            }
            assert!(serde_json::to_string(&ag["ziele"]).unwrap().len() < ZEICHEN_MAX + 600, "Ziele zu groß");
        }
        ziele_gesehen
    }

    #[test]
    fn einzelspieler_bekommt_wege_mit_gueltigen_vorlagen() {
        let (g, k) = spiel("Solo", 1);
        let gesehen = vorlagen_gueltig(&g, &k[0]);
        for z in ["stufe", "kolonie", "rohstoffe", "allianz"] { assert!(gesehen.iter().any(|x| x == z), "Ziel {z} fehlt: {gesehen:?}"); }
        assert!(!gesehen.iter().any(|x| x == "dyson"), "ohne Allianz kein Dyson-Ziel");
        let sid = k[0].sid.unwrap();
        // Ohne Allianz: der Diplomat bekommt die Gründung als Vorlage, der Verwalter nur den Hinweis.
        let dip = g.agenda(k[0].id, sid, Rolle::Diplomat);
        let allianz = dip["ziele"]["liste"].as_array().unwrap().iter().find(|z| z["ziel"] == "allianz").unwrap().clone();
        assert!(allianz["schritte"].as_array().unwrap().iter().any(|s| s["vorlagen"][0]["typ"] == "allianz_gruenden"), "{allianz}");
        let ver = g.agenda(k[0].id, sid, Rolle::Verwalter);
        let stufe = ver["ziele"]["liste"].as_array().unwrap().iter().find(|z| z["ziel"] == "stufe").unwrap().clone();
        assert!(stufe["schritte"].as_array().unwrap().iter().any(|s| s["du"] == true && s["vorlagen"].is_array()), "Verwalter ohne Bauvorlage für den Aufstieg: {stufe}");
        // Jede forschen-Vorlage hat ihr Labor oder davor den Laborbau (KI-Weg 28.10.2026: „braucht Labor Stufe 3“).
        let labor = g.world.planeten[g.world.spieler[sid as usize].heimat as usize].gebaeude[Gebaeude::Labor.idx()];
        for z in ver["ziele"]["liste"].as_array().unwrap() {
            let s = z["schritte"].as_array().unwrap();
            for (i, x) in s.iter().enumerate() {
                for v in x["vorlagen"].as_array().into_iter().flatten().filter(|v| v["typ"] == "forschen") {
                    let f = Forschung::aus_name(v["forschung"].as_str().unwrap()).unwrap();
                    let noetig = g.world.regeln.forschung[&f].labor;
                    assert!(labor >= noetig || s[..i].iter().any(|y| y["vorlagen"][0]["gebaeude"] == "labor"), "{f} ohne Laborschritt davor: {z}");
                }
            }
        }
        // Kette: nach dem ersten offenen Glied keine Vorlagen mehr (außer dem Erkunden des Zielplatzes).
        let kette = |ag: &Value| ag["ziele"]["liste"].as_array().unwrap().iter().find(|z| z["ziel"] == "kolonie").unwrap()["schritte"].as_array().unwrap().clone();
        let s = kette(&ver);
        let erstes = s.iter().position(|x| !x["text"].as_str().unwrap().starts_with("danach:")).unwrap();
        assert!(s[erstes + 1..].iter().filter(|x| x["text"].as_str().unwrap().starts_with("danach:")).all(|x| x.get("vorlagen").is_none()), "{s:?}");
        // Forschung, die der Kern für das Kolonieschiff verlangt, ist das aktuelle Glied, sobald Stufe, Raumhafen, Werften und
        // Bauteile da sind – mit forschen-Vorlage; das Fertigen selbst wartet.
        let mut g = g;
        {
            let ab = g.world.regeln.einh(Einheit::Kolonieschiff).ab_stufe;
            let sp = &mut g.world.spieler[sid as usize];
            sp.stufe = sp.stufe.max(ab);
            let heim = sp.heimat as usize;
            let p = &mut g.world.planeten[heim];
            for (b, n) in [(Gebaeude::Raumhafen, 1), (Gebaeude::Werft, 6), (Gebaeude::Orbitalwerft, 2)] { p.gebaeude[b.idx()] = n; }
            for gut in Gut::ALLE { p.bestand[gut.idx()] = 1_000_000_000 * M; }
        }
        assert!(!g.world.einheit_forschung_fehlend(sid, Einheit::Kolonieschiff).is_empty(), "Prüfwelt verlangt Forschung für das Kolonieschiff");
        let s = kette(&g.agenda(k[0].id, sid, Rolle::Verwalter));
        assert!(s.iter().any(|x| x["text"].as_str().unwrap().contains("Voraussetzung des Kolonieschiffs") && x["vorlagen"][0]["typ"] == "forschen"), "{s:?}");
        assert!(s.iter().any(|x| x["text"].as_str().unwrap().starts_with("danach: Kolonieschiff gefertigt") && x.get("vorlagen").is_none()), "{s:?}");
    }

    /// Zehn Reiche einer Allianz, sechs davon mit Planet im Zielsystem: Vorschlag kommt früh, die vier Plätze werden
    /// eindeutig auf die vier Mitglieder ohne Planet dort verteilt, nach dem Vorschlag stehen Beitritt und Anteil an.
    #[test]
    fn dyson_weg_schlaegt_frueh_vor_und_verteilt_plaetze() {
        let (mut g, k) = spiel("Sonne", 10);
        let sids: Vec<SpielerId> = k.iter().map(|a| a.sid.unwrap()).collect();
        let w = &mut g.world;
        assert!(w.ausbau.dyson.aktiv);
        let n = w.regeln.welt.plaetze_je_system;
        let sys = (2..w.regeln.welt.systeme_je_sektor).find(|s| (1..=n).all(|p| !w.belegung.contains_key(&Koord::neu(1, *s, p)))).unwrap();
        for (i, s) in sids.iter().enumerate() {
            w.spieler[*s as usize].stufe = 5;
            if i < 6 {
                let pid = w.spieler[*s as usize].heimat as usize;
                let alt = w.planeten[pid].koord;
                w.belegung.remove(&alt);
                let neu = Koord::neu(1, sys, i as u8 + 1);
                w.belegung.insert(neu, pid as PlanetId);
                w.planeten[pid].koord = neu;
            }
        }
        let name = |i: usize| k[i].name.clone();
        assert!(w.handeln(sids[0], Rolle::Alle, &json!({"typ":"allianz_gruenden","name":"Sonnenbund","tag":"SOL"})).0);
        for i in 1..10 {
            assert!(w.handeln(sids[0], Rolle::Alle, &json!({"typ":"allianz_einladen","spieler":name(i)})).0);
            assert!(w.handeln(sids[i], Rolle::Alle, &json!({"typ":"allianz_beitreten","allianz":"Sonnenbund"})).0);
        }
        let alt = w.zeit - 20 * TAG;
        for v in w.ausbau.dyson.mitglied_seit.values_mut() { v.1 = alt; }
        vorlagen_gueltig(&g, &k[0]);
        vorlagen_gueltig(&g, &k[7]);
        let sys_text = format!("1:{sys}");
        // Leitung: Vorschlag als Agenda-Punkt mit Vorlage (vorher erst bei vollem System).
        let st = g.agenda(k[0].id, sids[0], Rolle::Stratege);
        assert!(st["punkte"].as_array().unwrap().iter().any(|p| p["art"] == "dyson_vorschlagen" && p["vorlagen"][0]["system"] == sys_text.as_str()), "{}", st["punkte"]);
        let dyson = |ag: &Value| ag["ziele"]["liste"].as_array().unwrap().iter().find(|z| z["ziel"] == "dyson").unwrap().clone();
        assert!(dyson(&st)["stand"].as_str().unwrap().contains("6/12"), "{}", dyson(&st));
        // Platzverteilung: die sechs freien Plätze gehen reihum an die vier Mitglieder ohne Planet dort.
        let mut plaetze = vec![];
        for i in 6..10 {
            let ag = g.agenda(k[i].id, sids[i], Rolle::Verwalter);
            let p = ag["punkte"].as_array().unwrap().iter().find(|p| p["art"] == "dyson_platz").unwrap_or_else(|| panic!("{}: kein Dysonplatz", name(i)));
            let platz = p["text"].as_str().unwrap().split("Platz ").nth(1).unwrap().split(' ').next().unwrap().to_string();
            assert!(platz.starts_with(&format!("{sys_text}:")), "{platz}");
            let kol = ag["ziele"]["liste"].as_array().unwrap().iter().find(|z| z["ziel"] == "kolonie").unwrap();
            assert!(kol["titel"].as_str().unwrap().contains(&platz), "Koloniekette ohne den Dysonplatz: {kol}");
            plaetze.push(platz);
        }
        let mut eindeutig = plaetze.clone(); eindeutig.sort(); eindeutig.dedup();
        assert_eq!(eindeutig.len(), 4, "jeder bekommt einen anderen Platz: {plaetze:?}");
        // Siedler aus einem anderen System: erst dessen Karte (system_erkunden), solange sie unterwegs ist warten.
        assert!(g.world.aufklaerungsregeln());
        let h6 = g.world.spieler[sids[6] as usize].heimat as usize;
        g.world.planeten[h6].einheiten[Einheit::Spionagesonde.idx()] = 3;
        g.world.planeten[h6].gebaeude[Gebaeude::Raumhafen.idx()] = 1;
        let aufklaerung = |g: &Game| g.agenda(k[6].id, sids[6], Rolle::Feldherr)["ziele"]["liste"].as_array().unwrap().iter()
            .find(|z| z["ziel"] == "kolonie").unwrap()["schritte"].as_array().unwrap().iter()
            .find(|s| s["text"].as_str().is_some_and(|t| t.contains("kartieren") || t.contains("ausspähen") || t.contains("unterwegs"))).cloned().unwrap();
        let s = aufklaerung(&g);
        assert_eq!(s["vorlagen"][0]["mission"], "system_erkunden", "{s}");
        let (ok, text) = g.world.handeln(sids[6], Rolle::Alle, &s["vorlagen"][0]);
        assert!(ok, "{text}");
        let s = aufklaerung(&g);
        assert!(s["text"].as_str().unwrap().contains("unterwegs") && s["vorlagen"].as_array().is_none_or(|v| v.is_empty()), "{s}");
        assert!(g.agenda(k[2].id, sids[2], Rolle::Verwalter)["punkte"].as_array().unwrap().iter().all(|p| p["art"] != "dyson_platz"), "Mitglied mit Planet dort bekommt keinen Platz");
        // Nach dem Vorschlag: Beitritt und Anteil für die Mitglieder mit Planet im System.
        let (ok, text) = g.world.handeln(sids[0], Rolle::Alle, &json!({"typ":"dyson_vorschlagen","system":sys_text}));
        assert!(ok, "{text}");
        let d = dyson(&g.agenda(k[3].id, sids[3], Rolle::Stratege));
        assert!(d["stand"].as_str().unwrap().contains("sammelt"), "{d}");
        assert!(d["schritte"].as_array().unwrap().iter().any(|s| s["mitglied"] == name(3).as_str() && s["text"].as_str().unwrap().contains("beitreten") && s["du"] == true), "{d}");
        assert!(g.agenda(k[0].id, sids[0], Rolle::Stratege)["punkte"].as_array().unwrap().iter().all(|p| p["art"] != "dyson_vorschlagen"), "kein zweiter Vorschlag");
        // Anteil liegt auf der Heimat außerhalb, im System nur eine leere Kolonie: erst Transport, dann einzahlen.
        let kol = g.world.spieler[sids[5] as usize].heimat;
        g.world.spieler[sids[5] as usize].planeten.retain(|p| *p != kol);
        g.world.spieler[sids[6] as usize].planeten.push(kol);
        g.world.planeten[kol as usize].besitzer = sids[6];
        g.world.planeten[kol as usize].bestand = [0; GUETER];
        let (ok, text) = g.world.handeln(sids[6], Rolle::Alle, &json!({"typ":"dyson_beitreten","projekt":1}));
        assert!(ok, "{text}");
        g.world.planeten[h6].einheiten[Einheit::GrosserTransporter.idx()] = 400;
        g.world.spieler[sids[6] as usize].forschung[Forschung::Computertechnik.idx()] = 5;
        for gut in kern::dyson::ROHSTOFFE { g.world.planeten[h6].bestand[gut.idx()] = 9_000_000 * M; }
        let zahlweg = |g: &Game| dyson(&g.agenda(k[6].id, sids[6], Rolle::Feldherr))["schritte"].as_array().unwrap().iter()
            .find(|s| s["text"].as_str().is_some_and(|t| t.contains("im System bringen") || t.contains("Transport ins Dyson-System"))).cloned();
        let s = zahlweg(&g).unwrap_or_else(|| panic!("{}", dyson(&g.agenda(k[6].id, sids[6], Rolle::Feldherr))));
        assert_eq!(s["vorlagen"][0]["mission"], "transport", "{s}");
        // 400 Transporter tragen nicht 3 × 9 Mio.: eigener Schritt „Transporter fertigen“.
        let f = dyson(&g.agenda(k[6].id, sids[6], Rolle::Feldherr))["schritte"].as_array().unwrap().iter().find(|s| s["text"].as_str().is_some_and(|t| t.starts_with("Frachtraum reicht"))).cloned().unwrap();
        assert_eq!((f["vorlagen"][0]["typ"].as_str(), f["vorlagen"][0]["einheit"].as_str()), (Some("fertigen"), Some("grosser_transporter")), "{f}");
        let (ok, text) = g.world.handeln(sids[6], Rolle::Alle, &s["vorlagen"][0]);
        assert!(ok, "{text}");
        let s = zahlweg(&g).unwrap();
        assert!(s["text"].as_str().unwrap().contains("unterwegs"), "{s}");
        // Allianzhilfe: Mitglied 6 deckt seinen Anteil nicht, Mitglied 1 hat Überschuss und liefert an dessen Systemplaneten.
        let _ = g.world.handeln(sids[1], Rolle::Alle, &json!({"typ":"dyson_beitreten","projekt":1}));
        for gut in kern::dyson::ROHSTOFFE { g.world.planeten[h6].bestand[gut.idx()] = 100_000 * M; }
        let h1 = g.world.spieler[sids[1] as usize].heimat as usize;
        for gut in kern::dyson::ROHSTOFFE { g.world.planeten[h1].bestand[gut.idx()] = 20_000_000 * M; }
        g.world.planeten[h1].einheiten[Einheit::GrosserTransporter.idx()] = 2_000;
        g.world.planeten[h1].gebaeude[Gebaeude::Raumhafen.idx()] = 1;
        g.world.spieler[sids[1] as usize].forschung[Forschung::Computertechnik.idx()] = 5;
        g.world.planeten[h1].einheiten[Einheit::Spionagesonde.idx()] = 3;
        let helfer = |g: &Game| -> Vec<(usize, Value)> { (0..10).filter_map(|i| dyson(&g.agenda(k[i].id, sids[i], Rolle::Feldherr))["schritte"].as_array().unwrap().iter()
            .find(|s| s["text"].as_str().is_some_and(|t| t.starts_with(&format!("Allianzhilfe: {}", name(6)))) && s["du"] == true).cloned().map(|s| (i, s))).collect() };
        // Allianzmitglieder beliefern einander ohne eigenen Spionagebericht (Karl 28.10.2026): gleich liefern.
        let h = helfer(&g);
        assert_eq!(h.len(), 1, "genau ein Helfer: {h:?}");
        let (i, s) = &h[0];
        assert_eq!((*i, s["vorlagen"][0]["mission"].as_str(), s["vorlagen"][0]["ziel"].as_str()), (1, Some("transport"), Some(g.world.planeten[kol as usize].koord.to_string().as_str())), "{s}");
        let (ok, text) = g.world.handeln(sids[1], Rolle::Alle, &s["vorlagen"][0]);
        assert!(ok, "{text}");
        assert!(helfer(&g)[0].1["text"].as_str().unwrap().contains("unterwegs"), "{}", helfer(&g)[0].1);
    }
}
