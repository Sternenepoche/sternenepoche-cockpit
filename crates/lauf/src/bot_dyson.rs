//! Dysonsphäre der Skriptbots (docs/DYSONSPHAERE.md §7.8): Vorschlag durch eine Bot-Allianzleitung, Beitritt,
//! Einzahlung aus dem Überschuss der eigenen Planeten im System, Baubeginn, Anschlusswahl, Auszahlung von
//! Guthaben. Alles über dieselben Aktionen wie Spieler und Agenten. Welten ohne Dysonsphäre: nichts.

use super::Bottyp;
use kern::dyson::DysonPhase;
use kern::typen::*;
use kern::welt::*;
use serde_json::{json, Value};

fn tu(w: &mut Welt, sid: SpielerId, a: Value) -> bool { w.handeln(sid, Rolle::Alle, &a).0 }

/// Einmal je Spieltag (im ersten Bot-Takt des Tages).
pub fn zug(w: &mut Welt, sid: SpielerId, typ: Bottyp, takt: i64) {
    if !w.ausbau.dyson.aktiv || w.zeit % TAG >= takt || typ == Bottyp::Wehrlos { return; }
    let Some(r) = w.dyson_regeln().cloned() else { return };
    let Some(aid) = w.spieler[sid as usize].allianz else { auszahlen(w, sid); return };
    let leitung = w.allianz_leitung(sid, aid);
    let projekt = w.ausbau.dyson.projekte.values().filter(|p| p.allianz == aid && p.phase != DysonPhase::Abgebrochen).max_by_key(|p| p.id).cloned();
    match projekt {
        None if leitung => vorschlagen(w, sid, aid, &r),
        None => {}
        Some(p) => {
            let n = w.regeln.welt.plaetze_je_system;
            let eigene_im_system: Vec<usize> = (1..=n).filter_map(|pos| w.belegung.get(&Koord::neu(p.sektor, p.system, pos)).map(|x| *x as usize))
                .filter(|pid| w.planet_aktiv(*pid) && w.planeten[*pid].besitzer == sid).collect();
            // Auch Räuber treten bei, wenn sie im System siedeln: jedes Mitglied mit Planet zählt für die zehn.
            if p.phase.offen() && !p.mitglieder.contains_key(&sid) && !eigene_im_system.is_empty()
                && w.spieler[sid as usize].stufe >= r.stufe_min {
                tu(w, sid, json!({"typ":"dyson_beitreten","projekt":p.id}));
            }
            if p.phase.offen() && w.ausbau.dyson.projekte[&p.id].mitglieder.contains_key(&sid) { einzahlen(w, sid, p.id, &eigene_im_system); }
            if leitung && p.phase == DysonPhase::Sammeln && w.dyson_bedingungen(p.id).iter().all(|b| b.erfuellt) {
                tu(w, sid, json!({"typ":"dyson_bau_starten","projekt":p.id}));
            }
            if leitung && p.phase == DysonPhase::Vollendet { anschluesse(w, sid, p.id); }
        }
    }
    auszahlen(w, sid);
}

fn vorschlagen(w: &mut Welt, sid: SpielerId, aid: u32, r: &kern::dyson::DysonRegeln) {
    let Some(a) = w.allianzen.iter().find(|a| a.id == aid) else { return };
    // 20.10.2026: das im Allianzchat verkündete Sammelsystem vorschlagen, sobald die Allianz fünf Mitglieder hat und dort
    // vertreten ist – so sammeln die Mitglieder schon, während sie noch auf Stufe V wachsen (Bau erst mit allen
    // Bedingungen, siehe `dyson_bedingungen`).
    if a.mitglieder.len() >= 5 {
        if let Some((s, y)) = super::allianz_bot::sammelsystem(w, aid) {
            let vertreten = (1..=w.regeln.welt.plaetze_je_system).any(|pos| w.belegung.get(&Koord::neu(s, y, pos))
                .is_some_and(|pid| w.spieler[w.planeten[*pid as usize].besitzer as usize].allianz == Some(aid)));
            if vertreten {
                if tu(w, sid, json!({"typ":"dyson_vorschlagen","system":format!("{s}:{y}")})) {
                    let text = format!("Dysonsphäre im Sammelsystem {s}:{y} vorgeschlagen. Wer dort einen Planeten hat und Stufe V erreicht, tritt bei und zahlt aus dem Überschuss ein.");
                    tu(w, sid, json!({"typ":"brief_senden","kanal":"allianz","an":"","betreff":"Dysonsphäre","text":text,"antwort_auf":null}));
                }
                return;
            }
        }
    }
    let stufe_v = a.mitglieder.iter().filter(|m| w.spieler[**m as usize].stufe >= r.stufe_min).count();
    if stufe_v < r.mitglieder_min { return; }
    // System, in dem die Allianz die meisten Plätze hält (mindestens 8).
    let mut zaehl: std::collections::BTreeMap<(u8, u8), usize> = Default::default();
    for p in &w.planeten {
        if w.planet_aktiv(p.id as usize) && w.spieler[p.besitzer as usize].allianz == Some(aid) { *zaehl.entry((p.koord.sektor, p.koord.system)).or_default() += 1; }
    }
    if let Some(((s, y), n)) = zaehl.into_iter().max_by_key(|(k, n)| (*n, std::cmp::Reverse(*k))) {
        if n >= 8 { tu(w, sid, json!({"typ":"dyson_vorschlagen","system":format!("{s}:{y}")})); }
    }
}

fn einzahlen(w: &mut Welt, sid: SpielerId, projekt: u32, planeten: &[usize]) {
    let anteil = w.ausbau.dyson.anteil * M;
    let min = w.dyson_mindestbuchung();
    let heimat = w.spieler[sid as usize].heimat as usize;
    for pid in planeten {
        let Some(b) = w.ausbau.dyson.projekte.get(&projekt).and_then(|p| p.mitglieder.get(&sid)).cloned() else { return };
        let bestand = w.bestand_jetzt(*pid);
        let grenze = w.lagergrenze(*pid);
        let mut menge = [0i64; 3];
        // Eine Kolonie im System ist zugleich das Einzahlungsdepot, das `nachschub` von der Heimat füllt: dort geht alles Offene
        // ins Projekt (vorher blieb ein Zehntel der Lagergrenze liegen – genau der Bestand, der fehlte: Tag 121, ein Mitglied mit
        // 63 % Deuterium hatte 360.000 Deuterium im Depot).
        let depot = *pid != heimat;
        for (i, g) in kern::dyson::ROHSTOFFE.iter().enumerate() {
            let rest = (anteil - b.eingezahlt[i]).max(0) / M;
            // Rücklage für den eigenen Ausbau: ein Fünftel der Lagergrenze, höchstens 5 Mio.; vom Überschuss die Hälfte.
            let ruecklage = if depot { 0 } else { (grenze[g.idx()] / 5).min(5_000_000 * M) };
            let frei = ((bestand[g.idx()] - ruecklage).max(0) / M) / if depot { 1 } else { 2 };
            let x = frei.min(rest);
            if x >= min || (x == rest && x > 0) { menge[i] = x; }
        }
        if menge.iter().any(|x| *x > 0) {
            tu(w, sid, json!({"typ":"dyson_einzahlen","projekt":projekt,"von":w.planeten[*pid].koord.to_string(),"erz":menge[0],"kristall":menge[1],"deuterium":menge[2]}));
        }
    }
}

/// Was die Heimat eines Projektmitglieds für die Dysonsphäre zurücklegt, sobald das Projekt alles außer der Einzahlung
/// erfüllt (alle Plätze, zehn Mitglieder, Stufe V, Mitgliedschaft): der offene Anteil an Erz, Kristall und Deuterium,
/// höchstens 90 % der Lagergrenze. Ohne diese Rücklage verbaute jedes Mitglied seinen Überschuss, bevor der Nachschub ihn
/// fahren konnte (Fortsetzung bot_lage 712, Tag 60–80: alle Bedingungen außer der Einzahlung seit Tag 70 erfüllt, eingezahlt
/// 5,9 %, 15.674-mal „nachschub zu klein“).
/// Liegt `pid` im System eines offenen Projekts des Spielers? Solche Planeten sind Depots: die Logistik fährt ihren Bestand
/// nicht zur Heimat zurück.
pub(super) fn im_projektsystem(w: &Welt, sid: SpielerId, pid: usize) -> bool {
    let Some(aid) = w.spieler[sid as usize].allianz else { return false };
    let k = w.planeten[pid].koord;
    w.ausbau.dyson.projekte.values().any(|p| p.allianz == aid && p.phase.offen() && p.mitglieder.contains_key(&sid) && p.sektor == k.sektor && p.system == k.system)
}

pub(super) fn sparziel(w: &Welt, sid: SpielerId) -> [i64; GUETER] {
    let mut ziel = [0i64; GUETER];
    if !w.ausbau.dyson.aktiv { return ziel; }
    let Some(aid) = w.spieler[sid as usize].allianz else { return ziel };
    let Some(p) = w.ausbau.dyson.projekte.values().filter(|p| p.allianz == aid && p.phase == DysonPhase::Sammeln && p.mitglieder.contains_key(&sid))
        .max_by_key(|p| p.id) else { return ziel };
    if !w.dyson_bedingungen(p.id).iter().all(|b| b.erfuellt || b.kennung == "eingezahlt") { return ziel; }
    let anteil = w.ausbau.dyson.anteil * M;
    let grenze = w.lagergrenze(w.spieler[sid as usize].heimat as usize);
    let offen: Vec<i64> = (0..3).map(|i| (anteil - p.mitglieder[&sid].eingezahlt[i]).max(0)).collect();
    // Erz und Kristall bleiben auch für den Tausch gegen das fehlende Deuterium liegen (Soforthandel, `tauschfaktor`): sonst verbaute die
    // Heimat ihr Erz, sobald der Erzanteil voll war, und das Deuterium blieb zurück (frische Welt 712, Tag 85–115: Einzahlung
    // fiel von 1,2 % auf 0,4 % je Tag).
    let (fe, fk) = (tauschfaktor(w, Gut::Erz, Gut::Deuterium), tauschfaktor(w, Gut::Kristall, Gut::Deuterium));
    for (i, g) in kern::dyson::ROHSTOFFE.iter().enumerate() {
        let fuer_tausch = match i {
            0 if fe > 0.0 => (offen[2] as f64 / fe) as i64,
            1 if fk > 0.0 => (offen[2] as f64 / fk) as i64 / 2,
            _ => 0,
        };
        ziel[g.idx()] = (offen[i] + fuer_tausch).min(mal(grenze[g.idx()], 0.9));
    }
    ziel
}

/// Jeder Bot-Takt, nicht nur einmal am Tag: Ein Projektmitglied mit offenem Anteil zahlt vom eigenen Planeten im System ein
/// und fährt Erz, Kristall und Deuterium von der Heimat dorthin. Der Kern nimmt Einzahlungen nur von Planeten und Monden im
/// System an, und eine junge Kolonie dort fördert in Monaten nicht drei volle Anteile (bot_lage 712, 60 Tage: die Mitglieder
/// zahlten nur aus dem Bestand ihrer Kolonie im System). Zwei große Transporter bleiben für die übrige Logistik daheim.
pub fn nachschub(w: &mut Welt, sid: SpielerId, typ: Bottyp) {
    if !w.ausbau.dyson.aktiv || typ == Bottyp::Wehrlos { return; }
    let Some(aid) = w.spieler[sid as usize].allianz else { return };
    let Some(p) = w.ausbau.dyson.projekte.values().filter(|p| p.allianz == aid && p.phase.offen() && p.mitglieder.contains_key(&sid))
        .max_by_key(|p| p.id).cloned() else { return };
    let n = w.regeln.welt.plaetze_je_system;
    let im_system: Vec<usize> = (1..=n).filter_map(|pos| w.belegung.get(&Koord::neu(p.sektor, p.system, pos)).map(|x| *x as usize))
        .filter(|pid| w.planet_aktiv(*pid) && w.planeten[*pid].besitzer == sid).collect();
    let Some(&ziel) = im_system.first() else { return };
    einzahlen(w, sid, p.id, &im_system);
    let heimat = w.spieler[sid as usize].heimat as usize;
    let Some(b) = w.ausbau.dyson.projekte.get(&p.id).and_then(|q| q.mitglieder.get(&sid)).cloned() else { return };
    let anteil = w.ausbau.dyson.anteil * M;
    if (0..3).all(|i| b.eingezahlt[i] >= anteil) { helfen(w, sid, p.id); return; }
    // Tauschen auf der Heimat und auf jedem Planeten im System: dort lagen Kristall und Erz über dem vollen Anteil (Tag 121: ein
    // Depot mit 2,2 Mio. Kristall bei voll eingezahltem Kristall und 63 % Deuterium).
    tauschen(w, sid, heimat, &b.eingezahlt, anteil, true);
    for &pid in im_system.iter().filter(|x| **x != heimat) { tauschen(w, sid, pid, &b.eingezahlt, anteil, false); }
    einzahlen(w, sid, p.id, &im_system);
    if im_system.contains(&heimat) { return; }
    let zk = w.planeten[ziel].koord;
    if w.flotten.values().any(|f| f.besitzer == sid && f.ziel == zk && f.mission == Mission::Transport && f.zustand == Flottenzustand::Hinflug) { super::zaehle("dyson: nachschub unterwegs"); return; }
    if w.planeten[heimat].gebaeude[Gebaeude::Raumhafen.idx()] < 1 || w.planeten[heimat].blockade.is_some() { super::zaehle("dyson: heimat ohne raumhafen oder blockiert"); return; }
    let gt = Einheit::GrosserTransporter;
    let schiffe = w.planeten[heimat].einheiten[gt.idx()];
    // Eigene Frachtflotte fürs Depot: bis zu 24 große Transporter (600.000 je Fahrt), höchstens ein Viertel des Bestands je
    // Bestellung – mit zwölf war die Fahrt zum Depot der Engpass (Mitglieder zahlten ~17 % ihrer Förderung ein).
    if schiffe < 24 && w.planeten[heimat].fertigung[0].is_empty() && super::baubar(w, sid, heimat, gt) {
        let kosten = w.regeln.kosten_einheit(gt, w.spieler[sid as usize].volk);
        let bestand = w.bestand_jetzt(heimat);
        let neu = (0..GUETER).filter(|g| kosten[*g] > 0).map(|g| bestand[g] / 4 / kosten[g]).min().unwrap_or(0).min(24 - schiffe);
        if neu > 0 && tu(w, sid, json!({"typ":"fertigen","planet":w.planeten[heimat].koord.to_string(),"einheit":gt.name(),"anzahl":neu})) {
            super::zaehle("dyson: frachter bestellt");
        }
    }
    let anzahl = schiffe - 2;
    if anzahl < 1 { super::zaehle("dyson: zu wenig frachter"); return; }
    if w.flotten.values().filter(|f| f.besitzer == sid).count() >= w.flottenplaetze(sid) { super::zaehle("dyson: keine flottenplätze"); return; }
    let (hb, hgrenze, dort, dgrenze) = (w.bestand_jetzt(heimat), w.lagergrenze(heimat), w.bestand_jetzt(ziel), w.lagergrenze(ziel));
    let mut menge = [0i64; GUETER];
    // Spart die Heimat für das Projekt (alles außer der Einzahlung erfüllt), fährt der ganze Überschuss über einem Zehntel des
    // Lagers (höchstens 1 Mio.) – sonst die Hälfte über einem Fünftel (höchstens 5 Mio.).
    let spar = sparziel(w, sid).iter().any(|x| *x > 0);
    for (i, g) in kern::dyson::ROHSTOFFE.iter().enumerate() {
        let gi = g.idx();
        let offen = (anteil - b.eingezahlt[i] - dort[gi]).max(0);
        let ruecklage = if spar { (hgrenze[gi] / 10).min(1_000_000 * M) } else { (hgrenze[gi] / 5).min(5_000_000 * M) };
        let frei = (hb[gi] - ruecklage).max(0) / if spar { 1 } else { 2 };
        let platz = (mal(dgrenze[gi], 0.9) - dort[gi]).max(0);
        menge[gi] = offen.min(frei).min(platz);
    }
    // Was noch offen ist (abzüglich Depot): deckt die Ladung den ganzen Rest, fährt sie auch unter der Mindestmenge – sonst blieben
    // die letzten Prozente liegen (Probe Tag 121–145: 99,9 %, 8 von 10 Mitgliedern voll).
    let rest: i64 = kern::dyson::ROHSTOFFE.iter().enumerate().map(|(i, g)| (anteil - b.eingezahlt[i] - dort[g.idx()]).max(0)).sum();
    let anzahl = frachter_fuer(w, sid, heimat, zk, &menge, anzahl);
    let Some(menge) = super::ladung_fuer(w, sid, heimat, zk, anzahl, menge) else { super::zaehle("dyson: kein treibstoff"); return };
    if w.regeln.wert(&menge) < 20_000 * M && menge.iter().sum::<i64>() < rest { super::zaehle(if menge.iter().all(|x| *x == 0) { "dyson: heimat hat nichts übrig" } else { "dyson: nachschub zu klein" }); return; }
    if tu(w, sid, json!({"typ":"flotte_senden","start":w.planeten[heimat].koord.to_string(),"ziel":zk.to_string(),"mission":"transport",
        "schiffe":{gt.name(): anzahl},"ladung":super::ladung_json(&menge)})) {
        super::zaehle("dyson: nachschub gesendet");
    }
}

/// Allianzhilfe: Wer seinen Anteil voll eingezahlt hat, fährt den Überschuss seiner Heimat zum Depot des Mitglieds mit dem
/// größten offenen Rest, solange das Projekt bis auf die Einzahlung bereit ist – der Kern zählt Einzahlungen je Mitglied, das
/// Depot zahlt alles ein und tauscht Erz gegen Deuterium und Kristall (`tauschen`). Anlass: frische Welt 712 bis Tag 180 (r48),
/// neun von elf Mitgliedern voll, zwei schwache mit 8.000 Kristall bzw. 6.000 Deuterium Förderung je Tag hätten noch rund
/// 75 Tage gebraucht, während die vollen Mitglieder Hunderttausende daheim liegen hatten. Was schon zum Depot fliegt (auch von
/// anderen Helfern), zählt mit; fehlt dem Helfer Deuterium oder Kristall, fährt Erz für den Tausch mit.
fn helfen(w: &mut Welt, sid: SpielerId, projekt: u32) {
    let Some(p) = w.ausbau.dyson.projekte.get(&projekt).cloned() else { return };
    if p.phase != DysonPhase::Sammeln || !w.dyson_bedingungen(p.id).iter().all(|b| b.erfuellt || b.kennung == "eingezahlt") { return; }
    let anteil = w.ausbau.dyson.anteil * M;
    let n = w.regeln.welt.plaetze_je_system;
    let mut ziel: Option<(i64, usize, [i64; 3])> = None;
    for (m, b) in &p.mitglieder {
        if *m == sid { continue; }
        let Some(depot) = (1..=n).filter_map(|pos| w.belegung.get(&Koord::neu(p.sektor, p.system, pos)).map(|x| *x as usize))
            .find(|pid| w.planet_aktiv(*pid) && w.planeten[*pid].besitzer == *m) else { continue };
        let dk = w.planeten[depot].koord;
        let d = w.bestand_jetzt(depot);
        let mut unterwegs = [0i64; GUETER];
        for f in w.flotten.values().filter(|f| f.ziel == dk && f.mission == Mission::Transport && f.zustand == Flottenzustand::Hinflug) {
            for g in 0..GUETER { unterwegs[g] += f.ladung[g]; }
        }
        let mut bedarf = [0i64; 3];
        for (i, g) in kern::dyson::ROHSTOFFE.iter().enumerate() { bedarf[i] = (anteil - b.eingezahlt[i] - d[g.idx()] - unterwegs[g.idx()]).max(0); }
        // Erz über dem Erzanteil im Depot oder unterwegs geht in den Tausch: es senkt den Bedarf an Deuterium und Kristall.
        let mut erz_ueber = (b.eingezahlt[0] + d[Gut::Erz.idx()] + unterwegs[Gut::Erz.idx()] - anteil).max(0) as f64;
        for (i, g) in [(2usize, Gut::Deuterium), (1, Gut::Kristall)] {
            let f = tauschfaktor(w, Gut::Erz, g);
            if f <= 0.0 { continue; }
            let x = ((erz_ueber * f) as i64).min(bedarf[i]);
            bedarf[i] -= x;
            erz_ueber -= x as f64 / f;
        }
        let summe: i64 = bedarf.iter().sum();
        if summe > 0 && ziel.as_ref().map_or(true, |(s, _, _)| summe > *s) { ziel = Some((summe, depot, bedarf)); }
    }
    let Some((summe, depot, bedarf)) = ziel else { return };
    let heimat = w.spieler[sid as usize].heimat as usize;
    let zk = w.planeten[depot].koord;
    if w.flotten.values().any(|f| f.besitzer == sid && f.ziel == zk && f.mission == Mission::Transport && f.zustand == Flottenzustand::Hinflug) { return; }
    if w.planeten[heimat].gebaeude[Gebaeude::Raumhafen.idx()] < 1 || w.planeten[heimat].blockade.is_some() { return; }
    let gt = Einheit::GrosserTransporter;
    let schiffe = w.planeten[heimat].einheiten[gt.idx()];
    if schiffe < 12 && w.planeten[heimat].fertigung[0].is_empty() && super::baubar(w, sid, heimat, gt) {
        let kosten = w.regeln.kosten_einheit(gt, w.spieler[sid as usize].volk);
        let bestand = w.bestand_jetzt(heimat);
        let neu = (0..GUETER).filter(|g| kosten[*g] > 0).map(|g| bestand[g] / 4 / kosten[g]).min().unwrap_or(0).min(12 - schiffe);
        if neu > 0 && tu(w, sid, json!({"typ":"fertigen","planet":w.planeten[heimat].koord.to_string(),"einheit":gt.name(),"anzahl":neu})) {
            super::zaehle("dyson: frachter bestellt");
        }
    }
    let anzahl = schiffe - 2;
    if anzahl < 1 { super::zaehle("dyson: hilfe ohne frachter"); return; }
    if w.flotten.values().filter(|f| f.besitzer == sid).count() >= w.flottenplaetze(sid) { super::zaehle("dyson: hilfe ohne flottenplatz"); return; }
    // Die Hälfte über einem Zehntel des Heimatlagers: der Helfer baut weiter, das Projekt hat Vorrang vor dem Lagerüberschuss.
    let (hb, hgrenze, dort, dgrenze) = (w.bestand_jetzt(heimat), w.lagergrenze(heimat), w.bestand_jetzt(depot), w.lagergrenze(depot));
    let frei = |g: Gut| (hb[g.idx()] - hgrenze[g.idx()] / 10).max(0) / 2;
    let platz = |g: Gut| (mal(dgrenze[g.idx()], 0.9) - dort[g.idx()]).max(0);
    let mut menge = [0i64; GUETER];
    for (i, g) in kern::dyson::ROHSTOFFE.iter().enumerate() { menge[g.idx()] = bedarf[i].min(frei(*g)).min(platz(*g)); }
    let e = Gut::Erz.idx();
    for (i, g) in [(2usize, Gut::Deuterium), (1, Gut::Kristall)] {
        let f = tauschfaktor(w, Gut::Erz, g);
        if f <= 0.0 { continue; }
        let luecke = (bedarf[i] - menge[g.idx()]).max(0) as f64;
        let erz = ((luecke / f) as i64).min(frei(Gut::Erz) - menge[e]).min(platz(Gut::Erz) - menge[e]).max(0);
        menge[e] += erz;
    }
    let anzahl = frachter_fuer(w, sid, heimat, zk, &menge, anzahl);
    let Some(menge) = super::ladung_fuer(w, sid, heimat, zk, anzahl, menge) else { super::zaehle("dyson: hilfe ohne treibstoff"); return };
    let wert = w.regeln.wert(&menge);
    if wert < 20_000 * M && menge.iter().sum::<i64>() < summe { super::zaehle("dyson: hilfe zu klein"); return; }
    if tu(w, sid, json!({"typ":"flotte_senden","start":w.planeten[heimat].koord.to_string(),"ziel":zk.to_string(),"mission":"transport",
        "schiffe":{gt.name(): anzahl},"ladung":super::ladung_json(&menge)})) {
        super::zaehle("dyson: hilfe gesendet");
    }
}

/// So viele große Transporter, wie die Ladung braucht (einer mehr gegen den Treibstoffabzug), höchstens `max`: Mit allen
/// Transportern flog jede Fahrt den vollen Treibstoff, Helfer mit wenig Deuterium kamen gar nicht los (Fortsetzung der r48-Welt,
/// Tag 180–186: 2.151-mal „hilfe ohne treibstoff“, 1.831-mal „hilfe ohne frachter“).
fn frachter_fuer(w: &Welt, sid: SpielerId, start: usize, ziel: Koord, menge: &[i64; GUETER], max: i64) -> i64 {
    let mut s = [0i64; SCHIFFE];
    s[Einheit::GrosserTransporter.idx()] = 1;
    let kap = w.flugplan(sid, start, ziel, &s, 1000).map(|p| p.kapazitaet).unwrap_or(0);
    if kap <= 0 { return max; }
    let summe: i64 = menge.iter().sum();
    ((summe + kap - 1) / kap + 1).clamp(1, max.max(1))
}

/// Soforthandel nach den Kernwerten der Welt (regeln/kernwerte.ron, ConfigWelt; Standard Erz 1, Kristall 1,5, Deuterium 3,
/// 2 % Gebühr): Einheiten von `ziel`, die eine Einheit `von` nach der Gebühr ergibt; 0, wenn eines davon nicht tauschbar ist.
pub(super) fn tauschfaktor(w: &Welt, von: Gut, ziel: Gut) -> f64 {
    let kurs = |g: Gut| w.regeln.sofortkurs(g).unwrap_or(0.0);
    let (a, b) = (kurs(von), kurs(ziel));
    if a <= 0.0 || b <= 0.0 { return 0.0; }
    a * (1.0 - w.regeln.kw().handel.soforthandel_gebuehr) / b
}

/// Deuterium ist der Engpass der Einzahlung (Fortsetzung bot_lage 712, Tag 70–95: zehn Mitglieder fördern zusammen 5,5 Mio. Erz,
/// aber 1,0 Mio. Deuterium je Tag; eingezahlt Erz 52 %, Deuterium 16 % – mit dieser Schieflage erst um Tag 340 fertig). Ist das
/// Projekt bis auf die Einzahlung bereit, zahlt der Bot ausgeglichen: Liegt Erz oder Kristall (eingezahlt plus Heimatbestand)
/// mehr als 5 Punkte vor dem Deuterium, tauscht er im Soforthandel so viel, dass beide danach gleich weit gedeckt sind – feste
/// Kurse 1 : 1,5 : 3, 2 % Gebühr, ein Zehntel des Lagers bleibt, höchstens ein Tausch je Zug.
/// Den Rest deckt der echte Überschuss (eingezahlt plus Bestand über dem eigenen Anteil) auch unter 5 Punkten Abstand und unter
/// 1.000 Einheiten, und fehlt Kristall, tauscht überschüssiges Erz dagegen (Fortsetzung der r48-Welt, Tag 182: einem Mitglied
/// fehlten 6.264 Deuterium bei 134.000 Erz im Depot – das letzte von elf Mitgliedern hielt das Projekt an).
fn tauschen(w: &mut Welt, sid: SpielerId, planet: usize, eingezahlt: &[i64; 3], anteil: i64, zehntel_behalten: bool) {
    if sparziel(w, sid).iter().all(|x| *x == 0) { return; }
    deuterium_tauschen(w, sid, planet, eingezahlt, anteil, zehntel_behalten);
    kristall_tauschen(w, sid, planet, eingezahlt, anteil, zehntel_behalten);
}

fn deuterium_tauschen(w: &mut Welt, sid: SpielerId, heimat: usize, eingezahlt: &[i64; 3], anteil: i64, zehntel_behalten: bool) {
    let (hb, grenze) = (w.bestand_jetzt(heimat), w.lagergrenze(heimat));
    let a = anteil.max(1) as f64;
    let gedeckt = |i: usize, g: Gut| ((eingezahlt[i] + hb[g.idx()]) as f64 / a).min(1.0);
    let d = Gut::Deuterium.idx();
    let cd = gedeckt(2, Gut::Deuterium);
    if cd >= 1.0 { return; }
    let platz_d = (mal(grenze[d], 0.9) - hb[d]).max(0) as f64;
    let mut gewinn = 0.0f64;
    let mut angebot = serde_json::Map::new();
    for (i, g) in [(0usize, Gut::Erz), (1, Gut::Kristall)] {
        // f: Deuterium je Einheit des Guts nach der Gebühr (Standard 1/3 · 0,98 für Erz, 1,5/3 · 0,98 für Kristall).
        let f = tauschfaktor(w, g, Gut::Deuterium);
        if f <= 0.0 { continue; }
        let c = gedeckt(i, g);
        // Menge x, nach der Gut und Deuterium gleich weit gedeckt sind: c − x/a = cd + x·f/a
        let ausgleich = if c > cd + 0.05 { a * (c - cd) / (1.0 + f) } else { 0.0 };
        // Echter Überschuss über den eigenen Anteil deckt das ganze restliche Deuterium (eine Einheit Zugabe gegen das Abrunden).
        let ueber = ((eingezahlt[i] + hb[g.idx()]) as f64 - a).max(0.0);
        let d_offen = ((1.0 - cd) * a - gewinn).max(0.0);
        let rest = if d_offen > 0.0 { ((d_offen + M as f64) / f).min(ueber) } else { 0.0 };
        let x = ausgleich.max(rest);
        let frei = (hb[g.idx()] - if zehntel_behalten { grenze[g.idx()] / 10 } else { 0 }).max(0) as f64;
        let raum = ((platz_d - gewinn) / f).max(0.0);
        let y = x.min(frei).min(raum);
        let menge = ((y as i64 + M - 1) / M).min(frei as i64 / M);
        if menge < 1000 && (rest <= 0.0 || (menge * M) as f64 * f < M as f64) { continue; }
        angebot.insert(g.name().into(), json!(menge));
        gewinn += (menge * M) as f64 * f;
    }
    if angebot.is_empty() { return; }
    let k = w.planeten[heimat].koord.to_string();
    if tu(w, sid, json!({"typ":"soforthandel","planet":k,"angebot":angebot,"ziel":"deuterium"})) { super::zaehle("dyson: erz/kristall gegen deuterium"); }
}

/// Fehlt Kristall, tauscht der echte Erzüberschuss (eingezahlt plus Bestand über dem eigenen Anteil) dagegen (Soforthandel,
/// Standard 1 : 1,5 bei 2 % Gebühr) – Allianzhilfe bringt vor allem Erz (die Mitglieder fördern zusammen 3,8 Mio. Erz, aber 2,0 Mio. Kristall je Tag).
fn kristall_tauschen(w: &mut Welt, sid: SpielerId, planet: usize, eingezahlt: &[i64; 3], anteil: i64, zehntel_behalten: bool) {
    let (hb, grenze) = (w.bestand_jetzt(planet), w.lagergrenze(planet));
    let (e, k) = (Gut::Erz.idx(), Gut::Kristall.idx());
    let k_offen = (anteil - eingezahlt[1] - hb[k]).max(0);
    if k_offen == 0 { return; }
    let ueber = (eingezahlt[0] + hb[e] - anteil).max(0) - if zehntel_behalten { grenze[e] / 10 } else { 0 };
    let raum = (mal(grenze[k], 0.9) - hb[k]).max(0);
    let k_ziel = (k_offen + M).min(raum);
    let f = tauschfaktor(w, Gut::Erz, Gut::Kristall);
    if f <= 0.0 { return; }
    let erz = ((k_ziel as f64 / f) as i64).min(ueber.min(hb[e]));
    let menge = erz / M;
    if menge < 2 { return; }
    if tu(w, sid, json!({"typ":"soforthandel","planet":w.planeten[planet].koord.to_string(),"angebot":{"erz":menge},"ziel":"kristall"})) {
        super::zaehle("dyson: erz gegen kristall");
    }
}

fn anschluesse(w: &mut Welt, sid: SpielerId, projekt: u32) {
    let Some(p) = w.ausbau.dyson.projekte.get(&projekt).cloned() else { return };
    let n = w.regeln.welt.plaetze_je_system;
    let mut kandidaten: Vec<(u8, PlanetId, SpielerId)> = (1..=n).filter_map(|pos| w.belegung.get(&Koord::neu(p.sektor, p.system, pos)).copied())
        .filter(|pid| w.planet_aktiv(*pid as usize)).map(|pid| { let pl = &w.planeten[pid as usize]; (pl.gebaeude[Gebaeude::Deuteriumsynthesizer.idx()], pid, pl.besitzer) })
        .filter(|(_, _, b)| p.mitglieder.contains_key(b) && w.spieler[*b as usize].allianz == Some(p.allianz)).collect();
    kandidaten.sort_by_key(|(stufe, pid, _)| (std::cmp::Reverse(*stufe), *pid));
    let mut belegt: Vec<SpielerId> = p.anschluesse.iter().flatten().map(|a| a.besitzer).collect();
    for (i, slot) in p.anschluesse.iter().enumerate() {
        if slot.is_some() || p.slot_sperre_bis[i] > w.zeit { continue; }
        if let Some((_, pid, b)) = kandidaten.iter().find(|(_, _, b)| !belegt.contains(b)).copied() {
            if tu(w, sid, json!({"typ":"dyson_anschluss_waehlen","projekt":projekt,"slot":i + 1,"planet":w.planeten[pid as usize].koord.to_string()})) { belegt.push(b); }
        }
    }
}

fn auszahlen(w: &mut Welt, sid: SpielerId) {
    let Some(g) = w.ausbau.dyson.guthaben.get(&sid).copied() else { return };
    if let Some(pid) = w.spieler[sid as usize].planeten.first().copied() {
        let k = w.planeten[pid as usize].koord.to_string();
        tu(w, sid, json!({"typ":"dyson_auszahlen","planet":k,"erz":g[0] / M,"kristall":g[1] / M,"deuterium":g[2] / M}));
    }
}
