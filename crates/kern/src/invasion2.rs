//! Invasion v2 (0.4.8, Karl 24.10.2026): Verteidigungslinien statt Belagerungsuhr.
//!
//! „…dann soll man die Kampfwellen sehen … die Truppen auf die Wellen verteilen frei wie man möchte … jede Welle anklicken …
//! weiter Bodentruppen bauen (50 % langsamer), diese sammeln sich alle in der letzten Welle als Rückhalt! Ist der letzte Punkt
//! durchbrochen und der Rückhalt bezwungen, verliert man die Kontrolle … Status ‚Verwüstet‘ … mit Koloschiff als
//! Kolonisierung angeflogen werden, ob als Kampfkolonisierung oder normal, gelten dieselben Regeln.“
//!
//! Ablauf: Nach der Landung (Bomber oder Kampfkolonisieren mit Bodentruppen, Gebäude höchstens 30 %) stehen `wellen`
//! Verteidigungslinien. Der Verteidiger teilt Garnison und Miliz den Linien 1 … N−1 zu; alles andere, alle neu ausgebildeten
//! Truppen und verbündete Hilfstruppen bilden Linie N, den Rückhalt. Alle `wellen_abstand_stunden` greift eine Welle die
//! aktuelle Linie an (höchstens `wellen_runden_max` Bodenrunden mit den Kampfregeln der Invasion v1). Linie tot → gebrochen;
//! Angreifer tot → abgewehrt; sonst hält die Linie bis zur nächsten Welle. Rückhalt gebrochen → „Verwüstet“: ein Kolonieschiff
//! der Besatzerseite (Kampfkolonisieren oder Kolonisieren) landet und übernimmt, sobald ein Kolonieplatz frei ist. Landen neue
//! Verteidiger, steht der Rückhalt wieder.
//!
//! Nur neue Welten (Kernwert `boden.wellen` > 0); Zustand in `ausbau.boden.invasion.{wellen, verwuestet}` (JSON-Erweiterung,
//! leer = nicht geschrieben).
use crate::boden_katalog::{ANZAHL, KAMPFZAHL};
use crate::boden_kampf::{aufgebot_bw, feuern_bw, name, unterstuetzung_bw};
use crate::{kolonisation::Besetzung, typen::*, welt::*};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Wellenkampf {
    pub angreifer: SpielerId,
    pub verteidiger: SpielerId,
    pub seit: SimZeit,
    /// Zeitpunkt der nächsten Angriffswelle.
    pub naechste: SimZeit,
    /// Geschlagene Wellen.
    #[serde(default)]
    pub wellen: u32,
    /// Aktuelle Linie (0-basiert); die letzte ist der Rückhalt.
    #[serde(default)]
    pub linie: usize,
    /// Zuteilung der Linien 0 … N−2 je Kampftyp (Index ANZAHL = Miliz); der Rückhalt ist der Rest.
    #[serde(default)]
    pub zuteilung: Vec<[i64; KAMPFZAHL]>,
}

/// Maßgeblicher Regeltext der Invasion v2 aus den Kernwerten – für Regelseite, KI-Rat, Harness und Katalog (25.10.2026: v2-Welten
/// bekamen bis dahin den v1-Text mit 48-Stunden-Belagerung und Landung nur mit Kolonieschiff).
pub fn regeltext_v2(b: &crate::kernwerte::BodenWerte) -> String {
    let forschung = b.miliz_forschung.iter().map(|(f, s)| format!(", {} {s}", f.name())).collect::<String>();
    let anteile = b.miliz_anteil_promille.iter().map(|(v, p)| format!("{v} {} %", p / 10)).collect::<Vec<_>>().join(", ");
    let rufen = if b.miliz_jederzeit { "jederzeit rufbar (miliz_aufrufen)" } else { "nur während einer Invasion rufbar" };
    let reparatur = if b.reparatur_stufe_promille > 0 { format!("bis zu {} % der Baukosten seiner aktuellen Stufe", b.reparatur_stufe_promille / 10) }
        else { "den beschädigten Anteil seiner Baukosten".to_string() };
    format!("Invasion v2 (neue Welten; ersetzt die Belagerung der Invasion v1): 1. Verteidigung brechen: bombardieren (mindestens ein Bomber) \
oder kampfkolonisieren (Kolonieschiff) kämpfen zuerst gegen alle bewaffneten Schiffe, Verteidigungsanlagen und Planetenschilde, auch gegen \
verbündete Halten-Flotten und gestrandete Flotten, die dem Besitzer beistehen; reine Truppenkonvois kämpfen nicht im Orbit. \
2. Bombardierung: nur Bomber wirken, erste Runde sofort, dann alle {} Spielminuten; alle Gebäude verlieren gleich viel Integrität, nie unter {} %. \
3. Landung: sobald alle Gebäude höchstens {} % haben und die Flotte (bombardieren oder kampfkolonisieren) Bodentruppen in Truppentransportern \
trägt ({} Plätze je Transporter, boden_senden); ein Kolonieschiff braucht es erst für die Übernahme. 4. Linien und Wellen: der Planet hat \
{} Verteidigungslinien, die letzte ist der Rückhalt. Alle {} Spielstunden greift eine Welle die aktuelle Linie an, höchstens {} Bodenrunden; \
leere Linien fallen kampflos. Der Verteidiger teilt Garnison und Miliz frei auf die Linien auf (invasion_linien); nicht Zugeteiltes, neu \
Ausgebildetes, gelandete eigene Truppen und verbündete Bodenhilfe stehen im Rückhalt. Die Kaserne bildet währenddessen mit {} % Tempo aus. \
Verbündete des Angreifers mit Bodentruppen im Orbit kämpfen auf der Angreiferseite mit, Verluste anteilig. Ergebnis je Welle: abgewehrt \
(alle Angreifer gefallen), gebrochen (die nächste Welle trifft die nächste Linie) oder gehalten. 5. Verwüstet: fällt der Rückhalt, verliert \
der Besitzer die Kontrolle; ein Kolonieschiff der Besatzerseite (kampfkolonisieren oder kolonisieren) übernimmt, sobald ein Kolonieplatz frei \
ist, seine Siedler werden Bewohner. Landen neue Verteidiger, steht der Rückhalt wieder. Ursprüngliche Heimatwelten sind geschützt. \
6. Miliz: freigeschaltet mit Kaserne 1{forschung}; {rufen}; Ausbildung {} Spielsekunden je 1.000; kämpfender Anteil \
der Bewohner je Volk: {anteile}; Kapitulation bei {} % Verlusten des Aufgebots. 7. Reparatur: ein beschädigtes Gebäude kostet {reparatur}, \
anteilig zum Schaden; unter Bombardierung und Invasion keine Reparatur.",
        b.runde_minuten, b.integritaet_grenze_promille / 10, b.integritaet_grenze_promille / 10, b.transporter_plaetze, b.wellen,
        b.wellen_abstand_stunden, b.wellen_runden_max, b.invasion_ausbildung_promille / 10, b.miliz_ausbildung_sekunden_je_1000,
        b.miliz_kapitulation_promille / 10)
}

impl Welt {
    /// Regeltext der Invasion dieser Welt (v2 mit Wellen, sonst v1; leer ohne Invasionsregeln).
    pub fn regeltext_invasion(&self) -> String {
        if self.invasion_v2() { regeltext_v2(&self.regeln.kw().boden) }
        else if self.invasion_v1() { crate::invasion::REGELTEXT_INVASION.to_string() } else { String::new() }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Verwuestung {
    pub seit: SimZeit,
    pub besatzer: SpielerId,
    /// Besatzungsflotte (ihre Bodentruppen halten den Boden).
    pub flotte: FlottenId,
}

impl Welt {
    pub fn invasion_v2(&self) -> bool {
        self.invasion_v1() && self.regeln.kw().boden.wellen > 0
    }
    /// Zahl der Verteidigungslinien (einschließlich Rückhalt).
    pub fn linien_anzahl(&self) -> usize {
        self.regeln.kw().boden.wellen.clamp(1, 12) as usize
    }
    pub fn wellen_abstand(&self) -> SimZeit {
        self.regeln.kw().boden.wellen_abstand_stunden.max(1) * STUNDE
    }
    pub fn verwuestet(&self, pid: usize) -> bool {
        self.ausbau.boden.invasion.verwuestet.contains_key(&(pid as PlanetId))
    }
    /// Ist `k` ein verwüsteter Planet, den `sid` (oder ein Verbündeter) besetzt hält? Dann darf sein Kolonieschiff landen.
    pub fn verwuestet_fuer(&self, sid: SpielerId, k: Koord) -> bool {
        self.belegung.get(&k).and_then(|p| self.ausbau.boden.invasion.verwuestet.get(p))
            .is_some_and(|v| v.besatzer == sid || self.verbuendet(sid, v.besatzer))
    }
    /// Läuft auf diesem Planeten eine Invasion v2 (für Kaserne „50 % langsamer“ und Anzeigen)?
    pub fn invasion_laeuft(&self, pid: usize) -> bool {
        self.ausbau.boden.invasion.wellen.contains_key(&(pid as PlanetId))
    }

    /// Landung: Linien anlegen; die erste Welle kommt nach einem Wellenabstand (Zeit zum Verteilen und für die Miliz).
    pub(crate) fn wellenkampf_beginnen(&mut self, pid: usize, fid: FlottenId) {
        let id = pid as PlanetId;
        let Some(f) = self.flotten.get(&fid) else { return };
        let n = self.linien_anzahl();
        let w = Wellenkampf {
            angreifer: f.besitzer, verteidiger: self.planeten[pid].besitzer, seit: self.zeit,
            naechste: self.zeit + self.wellen_abstand(), wellen: 0, linie: 0, zuteilung: vec![[0; KAMPFZAHL]; n.saturating_sub(1)],
        };
        self.ausbau.boden.invasion.wellen.insert(id, w);
    }

    /// Verfügbare Verteidiger je Kampftyp: Garnison und einsatzbereite Miliz (ohne verbündete Hilfstruppen).
    fn verteidiger_bestand(&self, pid: usize) -> [i64; KAMPFZAHL] {
        let g = self.ausbau.boden.bestand.get(&(pid as PlanetId)).copied().unwrap_or_default();
        std::array::from_fn(|i| if i < ANZAHL { g[i] } else { self.miliz_staerke(pid) })
    }
    /// Truppen je Linie (aktuelle und spätere), der Rückhalt zuletzt (ohne verbündete Hilfstruppen). Zuteilungen werden in
    /// Reihenfolge der Linien aus dem Bestand bedient; was übrig bleibt, steht im Rückhalt.
    pub fn linien_verteilung(&self, pid: usize) -> Vec<[i64; KAMPFZAHL]> {
        let id = pid as PlanetId;
        let n = self.linien_anzahl();
        let mut frei = self.verteidiger_bestand(pid);
        let mut out = vec![[0i64; KAMPFZAHL]; n];
        if let Some(w) = self.ausbau.boden.invasion.wellen.get(&id) {
            for l in w.linie.min(n)..n.saturating_sub(1) {
                let z = w.zuteilung.get(l).copied().unwrap_or_default();
                for i in 0..KAMPFZAHL { let n = z[i].clamp(0, frei[i]); out[l][i] = n; frei[i] -= n; }
            }
        }
        out[n - 1] = frei;
        out
    }
    /// Verbündete der Angreiferseite mit Bodentruppen im Orbit (schließen sich den Angreifern an).
    pub fn angriffshelfer(&self, pid: usize) -> Vec<FlottenId> {
        let id = pid as PlanetId;
        let Some(w) = self.ausbau.boden.invasion.wellen.get(&id) else { return vec![] };
        let k = self.planeten[pid].koord;
        self.flotten.values().filter(|f| {
            f.zustand == Flottenzustand::ImOrbit && f.ziel == k && f.besitzer != w.angreifer && f.besitzer != w.verteidiger
                && !self.monde.flottenziel.contains_key(&f.id)
                && self.verbuendet(f.besitzer, w.angreifer) && !self.verbuendet(f.besitzer, w.verteidiger)
                && self.ausbau.boden.flotten.get(&f.id).is_some_and(|a| a.iter().sum::<i64>() > 0)
        }).map(|f| f.id).collect()
    }

    /// Darf `sid` mit Bodentruppen zu diesem Planeten fliegen, um einer verbündeten Invasion beizustehen? (Halten/Bodenhilfe.)
    pub fn angriffshilfe_erlaubt(&self, sid: SpielerId, pid: usize) -> bool {
        self.invasion_v2() && self.ausbau.boden.invasion.wellen.get(&(pid as PlanetId))
            .is_some_and(|w| w.angreifer != sid && w.verteidiger != sid && self.verbuendet(sid, w.angreifer) && !self.verbuendet(sid, w.verteidiger))
    }

    /// Takt je Fenster (aus `kolonisation_tick`): Orbit prüfen, fällige Wellen schlagen, verwüstete Planeten besetzen.
    pub(crate) fn wellen_schritt(&mut self, id: PlanetId, b: Besetzung) {
        let pid = id as usize;
        if !self.orbit_unbewaffnet(pid, b.flotte) {
            if !self.besetzung_verteidigung_bekaempfen(pid, b.flotte) {
                self.besetzung_abbrechen(id, "Bewaffnete Verteidigung nicht ausgeschaltet");
                return;
            }
            if !self.kolonisation.besetzungen.contains_key(&id) { return; }
        }
        if !self.ausbau.boden.invasion.wellen.contains_key(&id) { self.wellenkampf_beginnen(pid, b.flotte); }
        if self.verwuestet(pid) {
            // Neue Verteidiger (gelandeter Nachschub, ausgebildete Truppen) besetzen den Rückhalt wieder.
            if self.verteidiger_bestand(pid).iter().sum::<i64>() > 0 || !self.hilfsflotten(pid).is_empty() {
                self.ausbau.boden.invasion.verwuestet.remove(&id);
                let n = self.linien_anzahl();
                if let Some(w) = self.ausbau.boden.invasion.wellen.get_mut(&id) { w.linie = n - 1; }
                let k = self.planeten[pid].koord;
                for sid in [b.verteidiger, self.flotten.get(&b.flotte).map(|f| f.besitzer).unwrap_or(b.verteidiger)] {
                    self.vorfall(sid, "invasion", format!("{k}: neue Verteidiger stehen im Rückhalt – der Planet ist nicht mehr verwüstet"));
                }
            } else {
                self.verwuestet_landen(pid);
                return;
            }
        }
        for _ in 0..64 {
            let Some(w) = self.ausbau.boden.invasion.wellen.get(&id) else { return };
            if self.zeit < w.naechste { return; }
            self.welle(pid, b.flotte);
            if !self.kolonisation.besetzungen.contains_key(&id) || self.verwuestet(pid) { return; }
        }
    }

    /// Eine Angriffswelle gegen die aktuelle Linie (leere Linien werden überrannt).
    fn welle(&mut self, pid: usize, fid: FlottenId) {
        let id = pid as PlanetId;
        let n = self.linien_anzahl();
        let rg = self.regeln.clone();
        let bw = &rg.kw().boden;
        let aufgebot = aufgebot_bw(bw);
        let k = self.planeten[pid].koord;
        let Some(mut w) = self.ausbau.boden.invasion.wellen.get(&id).cloned() else { return };
        w.wellen += 1;
        w.naechste += self.wellen_abstand();
        // Leere Linien vor der aktuellen fallen kampflos.
        let mut verteilung = self.linien_verteilung(pid);
        let hilfe: Vec<(FlottenId, [i64; ANZAHL])> = self.hilfsflotten(pid).into_iter().map(|f| (f, self.ausbau.boden.flotten[&f])).collect();
        let mit_hilfe = |l: usize, v: &[[i64; KAMPFZAHL]]| -> [i64; KAMPFZAHL] {
            std::array::from_fn(|i| v[l][i] + if l == n - 1 && i < ANZAHL { hilfe.iter().map(|(_, h)| h[i]).sum::<i64>() } else { 0 })
        };
        while w.linie < n && mit_hilfe(w.linie, &verteilung).iter().sum::<i64>() == 0 {
            w.linie += 1;
        }
        self.ausbau.boden.invasion.wellen.insert(id, w.clone());
        if w.linie >= n {
            self.verwuesten(pid, fid);
            return;
        }
        let l = w.linie;
        self.boden_kapazitaet(fid);
        let helfer: Vec<(FlottenId, [i64; ANZAHL])> = std::iter::once(fid).chain(self.angriffshelfer(pid))
            .map(|f| (f, self.ausbau.boden.flotten.get(&f).copied().unwrap_or_default())).collect();
        let mut a: [i64; KAMPFZAHL] = std::array::from_fn(|i| if i < ANZAHL { helfer.iter().map(|(_, t)| t[i]).sum() } else { 0 });
        let mut d = mit_hilfe(l, &verteilung);
        let (a0, d0) = (a, d);
        let mut wa = [0i64; KAMPFZAHL];
        let mut wd = [0i64; KAMPFZAHL];
        let mut al_sum = [0i64; KAMPFZAHL];
        let mut dl_sum = [0i64; KAMPFZAHL];
        let mut runden = vec![];
        let miliz_grenze = self.ausbau.boden.invasion.milizen.get(&id).filter(|m| !m.kapituliert)
            .map(|m| ((m.aufgebot * bw.miliz_kapitulation_promille + 999) / 1000 - m.gefallen).max(0)).unwrap_or(i64::MAX);
        for r in 0..bw.wellen_runden_max.max(1) {
            if a.iter().sum::<i64>() == 0 || d.iter().sum::<i64>() == 0 { break; }
            let asup = unterstuetzung_bw(bw, &a, &mut wa);
            let dsup = unterstuetzung_bw(bw, &d, &mut wd);
            let (mut dl, af, dw) = feuern_bw(bw, &a, &d, wd, aufgebot);
            dl[ANZAHL] = dl[ANZAHL].min((miliz_grenze - dl_sum[ANZAHL]).max(0));
            let (al, df, aw) = feuern_bw(bw, &d, &a, wa, aufgebot);
            for i in 0..KAMPFZAHL { a[i] -= al[i]; d[i] -= dl[i]; al_sum[i] += al[i]; dl_sum[i] += dl[i]; }
            wa = aw;
            wd = dw;
            runden.push(json!({"runde":r+1,"angreifer_feuer":af,"verteidiger_feuer":df,"angreifer_unterstuetzung":asup,"verteidiger_unterstuetzung":dsup,
                "angreifer_verluste":al,"verteidiger_verluste":dl}));
        }
        // Verluste der Angreifer anteilig auf Hauptflotte und Helfer, Rest der Reihe nach.
        let mut quellen: Vec<[i64; ANZAHL]> = helfer.iter().map(|(_, t)| *t).collect();
        for i in 0..ANZAHL { verteile(&mut quellen, i, al_sum[i]); }
        for (q, (f, _)) in quellen.iter().zip(&helfer) { self.ausbau.boden.flotten.insert(*f, *q); }
        // Verluste der Verteidiger: Linie → Garnison (Rückhalt zusätzlich verbündete Hilfstruppen), Miliz → Bewohner.
        let mut garnison = self.ausbau.boden.bestand.get(&id).copied().unwrap_or_default();
        // Herkunft der Verteidiger im Rückhalt (wie v1 `verteidiger_quellen`): Helfer sehen den Bericht und erfahren ihre Verluste
        // (Einsatzprobe 25.10.2026: die Bodenhilfe fiel, ihr Besitzer bekam weder Bericht noch Meldung).
        let mut quellen_json: Vec<Value> = Vec::new();
        let mut helfer_meldung: Vec<(SpielerId, i64, i64)> = Vec::new();
        if l == n - 1 && !hilfe.is_empty() {
            let mut q: Vec<[i64; ANZAHL]> = std::iter::once(garnison).chain(hilfe.iter().map(|(_, h)| *h)).collect();
            for i in 0..ANZAHL { verteile(&mut q, i, dl_sum[i]); }
            quellen_json.push(json!({"quelle":"garnison","spieler":w.verteidiger,"vorher":garnison,"nachher":q[0]}));
            for (k2, (f, h)) in hilfe.iter().enumerate() {
                let b = self.flotten.get(f).map(|x| x.besitzer).unwrap_or(w.verteidiger);
                quellen_json.push(json!({"quelle":"verbuendete","flotte":f,"spieler":b,"name":self.spieler[b as usize].name,"vorher":h,"nachher":q[k2 + 1]}));
                if b != w.verteidiger { helfer_meldung.push((b, h.iter().sum::<i64>() - q[k2 + 1].iter().sum::<i64>(), q[k2 + 1].iter().sum::<i64>())); }
            }
            garnison = q[0];
            for (h, (f, _)) in q[1..].iter().zip(&hilfe) { self.ausbau.boden.flotten.insert(*f, *h); }
        } else {
            for i in 0..ANZAHL { garnison[i] = (garnison[i] - dl_sum[i]).max(0); }
        }
        self.ausbau.boden.bestand.insert(id, garnison);
        // Kapitulation am tatsächlich kämpfenden Aufgebot messen: die Miliz kämpft mit der aktuellen Stärke (wächst mit der
        // Bevölkerung), die Schwelle stand auf dem Aufgebot beim Rufen – sie kapitulierte zu früh, und der Bericht verschwieg es
        // (Einsatzprobe 25.10.2026, M1).
        let mut miliz_kapituliert = false;
        if dl_sum[ANZAHL] > 0 {
            let staerke = self.miliz_staerke(pid);
            let vorher = self.ausbau.boden.invasion.milizen.get(&id).map(|m| m.kapituliert).unwrap_or(true);
            if let Some(m) = self.ausbau.boden.invasion.milizen.get_mut(&id) { m.aufgebot = m.aufgebot.max(staerke); }
            self.miliz_verluste_v1(pid, dl_sum[ANZAHL]);
            miliz_kapituliert = !vorher && self.ausbau.boden.invasion.milizen.get(&id).is_none_or(|m| m.kapituliert);
        }
        if miliz_kapituliert { d[ANZAHL] = 0; }
        if let Some(w) = self.ausbau.boden.invasion.wellen.get_mut(&id) {
            if l < w.zuteilung.len() { for i in 0..KAMPFZAHL { w.zuteilung[l][i] = (w.zuteilung[l][i] - dl_sum[i]).max(0); } }
        }
        verteilung = self.linien_verteilung(pid);
        // Hilfstruppen nach der Verlustverteilung neu lesen (Einsatzprobe 25.10.2026: mit der Kopie vor dem Kampf meldete ein
        // Rückhalt nur aus gefallener Hilfe „gehalten“ und schenkte dem Verteidiger eine Welle).
        let hilfe_rest: i64 = if l == n - 1 {
            hilfe.iter().map(|(f, _)| self.ausbau.boden.flotten.get(f).map(|t| t.iter().sum::<i64>()).unwrap_or(0)).sum()
        } else { 0 };
        let rest_d = verteilung[l].iter().sum::<i64>() + hilfe_rest;
        let rest_a: i64 = std::iter::once(fid).chain(self.angriffshelfer(pid)).map(|f| self.ausbau.boden.flotten.get(&f).map(|t| t.iter().sum::<i64>()).unwrap_or(0)).sum();
        let ergebnis = if rest_a == 0 && rest_d > 0 { "abgewehrt" } else if rest_d == 0 { "gebrochen" } else { "gehalten" };
        let (an, vn) = (self.spieler[w.angreifer as usize].name.clone(), self.spieler[w.verteidiger as usize].name.clone());
        self.ausbau.boden.berichte.push(json!({"zeit":self.zeit,"regeln":"invasion-v2","planet":k.to_string(),"welle":w.wellen,"linie":l+1,"linien":n,
            "angreifer":w.angreifer,"verteidiger":w.verteidiger,"angreifer_name":an,"verteidiger_name":vn,"ergebnis":ergebnis,
            "typen":(0..KAMPFZAHL).map(name).collect::<Vec<_>>(),"angreifer_vorher":a0,"verteidiger_vorher":d0,
            "angreifer_verluste":al_sum,"verteidiger_verluste":dl_sum,"angreifer_nachher":a,"verteidiger_nachher":d,"runden":runden,
            "verteidiger_quellen":quellen_json,"miliz_kapituliert":miliz_kapituliert}));
        self.bodenberichte_kappen();
        let ergebnis_text = match ergebnis { "abgewehrt" => "die Invasion ist abgewehrt", "gebrochen" => "der Rückhalt ist gebrochen", _ => "der Rückhalt hält stand" };
        for (b, gefallen, uebrig) in helfer_meldung {
            self.vorfall(b, "invasion", format!("{k}: Welle {} – deine Bodenhilfe kämpfte im Rückhalt von {vn}: {gefallen} gefallen, {uebrig} übrig; {ergebnis_text}", w.wellen));
        }
        let linie_name = if l == n - 1 { "der Rückhalt".to_string() } else { format!("Linie {}", l + 1) };
        match ergebnis {
            "abgewehrt" => {
                for sid in [w.angreifer, w.verteidiger] { self.vorfall(sid, "invasion", format!("{k}: Welle {} – {linie_name} hält, die Invasion ist abgewehrt", w.wellen)); }
                self.besetzung_abbrechen(id, "Bodenverteidigung hat die Invasion zurückgeschlagen");
            }
            "gebrochen" => {
                if let Some(w) = self.ausbau.boden.invasion.wellen.get_mut(&id) { w.linie = l + 1; }
                if l + 1 >= n {
                    self.verwuesten(pid, fid);
                } else {
                    for sid in [w.angreifer, w.verteidiger] { self.vorfall(sid, "invasion", format!("{k}: Welle {} – {linie_name} ist gebrochen; die nächste Welle trifft Linie {}", w.wellen, l + 2)); }
                    self.wecke(w.verteidiger, Rolle::Feldherr, "Invasion: Linie gebrochen", true);
                }
            }
            _ => {
                for sid in [w.angreifer, w.verteidiger] { self.vorfall(sid, "invasion", format!("{k}: Welle {} – {linie_name} hält stand; nächste Welle in {} Spielstunden", w.wellen, self.wellen_abstand() / STUNDE)); }
            }
        }
    }

    /// Rückhalt gebrochen: der Besitzer verliert die Kontrolle; ein Kolonieschiff der Besatzerseite kann landen.
    fn verwuesten(&mut self, pid: usize, fid: FlottenId) {
        let id = pid as PlanetId;
        let Some(besatzer) = self.flotten.get(&fid).map(|f| f.besitzer) else { return };
        self.ausbau.boden.invasion.verwuestet.insert(id, Verwuestung { seit: self.zeit, besatzer, flotte: fid });
        let k = self.planeten[pid].koord;
        let besitzer = self.planeten[pid].besitzer;
        self.vorfall(besitzer, "invasion", format!("{k}: Der Rückhalt ist gebrochen – du hast die Kontrolle verloren (verwüstet). Landen neue Truppen, steht der Rückhalt wieder"));
        self.vorfall(besatzer, "invasion", format!("{k}: Der Rückhalt ist gebrochen – der Planet ist verwüstet. Ein Kolonieschiff (Kampfkolonisieren oder Kolonisieren) kann jetzt landen und ihn übernehmen"));
        self.wecke(besitzer, Rolle::Stratege, "Planet verwüstet", true);
        self.fachereignis("planet_devastated", besatzer, json!({"planet":pid,"fleet":fid,"owner":besitzer}));
        if !self.verwuestet_landen(pid) {
            // Kolonieschiff im Orbit, aber kein freier Platz: einmal sagen, was fehlt (es landet, sobald einer frei ist).
            let wartet = self.flotten.values().any(|f| f.zustand == Flottenzustand::ImOrbit && f.ziel == k && f.besitzer == besatzer && Self::kolonieschiffe(&f.schiffe) > 0);
            if wartet {
                let text = self.kolonien_voll_text(besatzer);
                self.vorfall(besatzer, "kampfkolonisation", format!("Übernahme von {k} wartet – kein freier Kolonieplatz: {text}. Das Kolonieschiff landet, sobald ein Platz frei ist"));
                let name = self.spieler[besatzer as usize].name.clone();
                self.vorfall(besitzer, "kampfkolonisation", format!("{k}: Übernahme durch {name} vorerst gescheitert – kein freier Kolonieplatz beim Angreifer; neue Truppen besetzen den Rückhalt wieder"));
            }
        }
    }

    /// Landet ein Kolonieschiff der Besatzerseite im Orbit, wird der verwüstete Planet übernommen (freier Kolonieplatz nötig).
    pub(crate) fn verwuestet_landen(&mut self, pid: usize) -> bool {
        let id = pid as PlanetId;
        let Some(v) = self.ausbau.boden.invasion.verwuestet.get(&id).cloned() else { return false };
        let k = self.planeten[pid].koord;
        let kandidaten: Vec<FlottenId> = self.flotten.values().filter(|f| {
            f.zustand == Flottenzustand::ImOrbit && f.ziel == k && !self.monde.flottenziel.contains_key(&f.id)
                && (f.besitzer == v.besatzer || self.verbuendet(f.besitzer, v.besatzer))
                && matches!(f.mission, Mission::Kampfkolonisieren | Mission::Kolonisieren)
                && Self::kolonieschiffe(&f.schiffe) > 0
        }).map(|f| f.id).collect();
        for fid in kandidaten {
            let sid = self.flotten[&fid].besitzer;
            if self.kolonien(sid) >= self.kolonien_erlaubt(sid) {
                continue;
            }
            self.uebernahme_verwuestet(pid, fid, v.flotte);
            return true;
        }
        false
    }

    /// Übernahme eines verwüsteten Planeten durch die Kolonieflotte `fid`; die Besatzungstruppen werden Garnison.
    fn uebernahme_verwuestet(&mut self, pid: usize, fid: FlottenId, besatzung: FlottenId) {
        let id = pid as PlanetId;
        let neu = self.flotten[&fid].besitzer;
        // Bodentruppen der Besatzung (und eigene Angriffshelfer) landen als Garnison des neuen Besitzers.
        let mut truppen = [0i64; ANZAHL];
        for f in std::iter::once(besatzung).chain(self.angriffshelfer(pid)) {
            if self.flotten.get(&f).is_some_and(|x| x.besitzer == neu) {
                if let Some(t) = self.ausbau.boden.flotten.remove(&f) { for i in 0..ANZAHL { truppen[i] += t[i]; } }
            }
        }
        self.ausbau.boden.invasion.verwuestet.remove(&id);
        self.ausbau.boden.invasion.wellen.remove(&id);
        self.ausbau.boden.invasion.bombardements.remove(&id);
        self.belagerung_beenden(pid);
        self.miliz_entlassen(pid);
        self.erobern(pid, fid);
        if self.planeten[pid].besitzer == neu {
            let g = self.ausbau.boden.bestand.entry(id).or_default();
            for i in 0..ANZAHL { g[i] += truppen[i]; }
            // Die Besatzungsflotte des neuen Besitzers landet auf ihrem neuen Planeten.
            if besatzung != fid && self.flotten.get(&besatzung).is_some_and(|f| f.besitzer == neu && f.zustand == Flottenzustand::ImOrbit) {
                self.orbitflotte_landen(besatzung, pid);
            }
            let k = self.planeten[pid].koord;
            self.vorfall(neu, "invasion", format!("{k}: Kolonieschiff gelandet – der Planet gehört dir. Gebäude stehen auf 30 %: jedes Gebäude zeigt „Reparieren“ (75 % der Kosten der aktuellen Stufe)"));
        }
    }

    /// Eine Flotte im Orbit eines eigenen Planeten landet (Schiffe, Ladung, Personen, Truppen).
    pub(crate) fn orbitflotte_landen(&mut self, fid: FlottenId, pid: usize) {
        self.boden_landen(fid, pid);
        self.rueckflug_landen(fid, pid);
    }

    /// Ankunft einer Kolonie- oder Kampfkolonisierungsflotte der Besatzerseite an einem verwüsteten Planeten: in den Orbit,
    /// landen, sobald ein Kolonieplatz frei ist. true = Ankunft erledigt.
    pub(crate) fn verwuestet_ankunft(&mut self, fid: FlottenId, ziel_pid: Option<usize>) -> bool {
        if !self.invasion_v2() { return false; }
        let Some(pid) = ziel_pid else { return false };
        let Some(v) = self.ausbau.boden.invasion.verwuestet.get(&(pid as PlanetId)).cloned() else { return false };
        let Some(f) = self.flotten.get(&fid).cloned() else { return false };
        if !matches!(f.mission, Mission::Kolonisieren | Mission::Kampfkolonisieren) || Self::kolonieschiffe(&f.schiffe) == 0
            || !(f.besitzer == v.besatzer || self.verbuendet(f.besitzer, v.besatzer)) {
            return false;
        }
        if let Some(fl) = self.flotten.get_mut(&fid) { fl.zustand = Flottenzustand::ImOrbit; fl.orbit_ende = 0; }
        if !self.verwuestet_landen(pid) {
            let text = self.kolonien_voll_text(f.besitzer);
            self.vorfall(f.besitzer, "invasion", format!("{}: Kolonieschiff wartet im Orbit des verwüsteten Planeten – {text}", f.ziel));
        }
        true
    }

    /// Verteidiger teilt Truppen den Linien zu (`linien[l][typ]`, Index 12 = Miliz); der Rückhalt ist der Rest.
    pub fn linien_zuteilen(&mut self, sid: SpielerId, k: Koord, linien: Vec<Vec<i64>>) -> Result<String, String> {
        if !self.invasion_v2() { return Err("Verteidigungslinien gibt es in Welten mit Invasion v2".into()); }
        let pid = self.eigener_planet(sid, k)?;
        let id = pid as PlanetId;
        let n = self.linien_anzahl();
        let Some(w) = self.ausbau.boden.invasion.wellen.get(&id).cloned() else { return Err("Auf diesem Planeten läuft keine Invasion".into()) };
        if linien.len() > n.saturating_sub(1) { return Err(format!("Höchstens {} Linien vor dem Rückhalt", n.saturating_sub(1))); }
        let mut z = vec![[0i64; KAMPFZAHL]; n.saturating_sub(1)];
        for (l, zeile) in linien.iter().enumerate() {
            if zeile.len() > KAMPFZAHL { return Err("Zu viele Truppentypen in einer Linie".into()); }
            if l < w.linie && zeile.iter().any(|x| *x != 0) { return Err(format!("Linie {} ist schon gebrochen", l + 1)); }
            for (i, x) in zeile.iter().enumerate() {
                if !(0..=10_000_000).contains(x) { return Err("Truppenzahl außerhalb des Bereichs".into()); }
                z[l][i] = *x;
            }
        }
        let bestand = self.verteidiger_bestand(pid);
        for i in 0..KAMPFZAHL {
            let summe: i64 = z.iter().map(|r| r[i]).sum();
            if summe > bestand[i] { return Err(format!("Mehr {} zugeteilt als vorhanden ({summe} > {})", crate::boden_kampf::name(i), bestand[i])); }
        }
        if let Some(w) = self.ausbau.boden.invasion.wellen.get_mut(&id) { w.zuteilung = z; }
        Ok(format!("Verteidigungslinien auf {k} gesetzt; nicht Zugeteiltes steht im Rückhalt"))
    }

    /// Sicht für Verteidiger, Angreifer und Verbündete: Linien, nächste Welle, Berichte der Wellen.
    pub fn wellen_sicht(&self, pid: usize, sid: SpielerId) -> Value {
        let id = pid as PlanetId;
        let Some(w) = self.ausbau.boden.invasion.wellen.get(&id) else { return Value::Null };
        let n = self.linien_anzahl();
        let verteidiger = sid == w.verteidiger || self.verbuendet(sid, w.verteidiger);
        let verteilung = self.linien_verteilung(pid);
        let hilfe: i64 = self.hilfsflotten(pid).iter().map(|f| self.ausbau.boden.flotten[f].iter().sum::<i64>()).sum();
        let angreifer: [i64; ANZAHL] = std::array::from_fn(|i| std::iter::once(self.kolonisation.besetzungen.get(&id).map(|b| b.flotte))
            .flatten().chain(self.angriffshelfer(pid)).map(|f| self.ausbau.boden.flotten.get(&f).map(|t| t[i]).unwrap_or(0)).sum());
        let linien: Vec<Value> = (0..n).map(|l| {
            let status = if l < w.linie { "gebrochen" } else if l == w.linie { "aktuell" } else { "wartet" };
            let truppen = if verteidiger { json!(verteilung[l]) } else { Value::Null };
            json!({"linie":l+1,"rueckhalt":l==n-1,"status":status,"truppen":truppen,
                "gesamt":if verteidiger {json!(verteilung[l].iter().sum::<i64>() + if l==n-1 {hilfe} else {0})} else {Value::Null}})
        }).collect();
        let k = self.planeten[pid].koord.to_string();
        let berichte: Vec<Value> = self.ausbau.boden.berichte.iter().rev().filter(|b| b["planet"] == k.as_str() && b["regeln"] == "invasion-v2").take(12).cloned().collect();
        json!({"regeln":"invasion-v2","linien_anzahl":n,"linie":w.linie+1,"wellen":w.wellen,"naechste_welle":w.naechste,"abstand_sekunden":self.wellen_abstand(),
            "runden_je_welle":self.regeln.kw().boden.wellen_runden_max,"angreifer_name":self.spieler[w.angreifer as usize].name,"verteidiger_name":self.spieler[w.verteidiger as usize].name,
            "angreifer_truppen":angreifer,"verbuendete_hilfe":if verteidiger {json!(hilfe)} else {Value::Null},"linien":linien,"zuteilung":if verteidiger {json!(w.zuteilung)} else {Value::Null},
            "verwuestet":self.verwuestet(pid),"ausbildung_promille":self.regeln.kw().boden.invasion_ausbildung_promille,"berichte":berichte,
            "hinweis":"Teile Truppen und Miliz den Linien zu; alles andere und neu Ausgebildete stehen im Rückhalt. Bricht der Rückhalt, ist der Planet verwüstet und ein Kolonieschiff der Gegenseite kann landen."})
    }
}

/// Verluste `n` des Typs `i` anteilig auf die Quellen, Rest der Reihe nach.
fn verteile(quellen: &mut [[i64; ANZAHL]], i: usize, n: i64) {
    let summe: i64 = quellen.iter().map(|q| q[i]).sum();
    if n <= 0 || summe == 0 { return; }
    let mut rest = n.min(summe);
    let anteile: Vec<i64> = quellen.iter().map(|q| n.min(summe) * q[i] / summe).collect();
    for (q, a) in quellen.iter_mut().zip(&anteile) { let x = (*a).min(q[i]); q[i] -= x; rest -= x; }
    for q in quellen.iter_mut() { let x = rest.min(q[i]); q[i] -= x; rest -= x; }
}
