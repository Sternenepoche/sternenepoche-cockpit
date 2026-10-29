//! Ablauf-Version 1 (neue Welten ab 17.10.2026): Sackgassen der Personal- und Wachstumslogik beheben.
//!
//! - Klima: Wachstumsfaktoren relativ zum Grundwert `wachstum_je_tag` (1000 = Grundwert), abgeleitet aus dem
//!   Abstand der Bahn zur Heimatbahn des Volkes (`habitabilitaet::WACHSTUM_V1`, `docs/WACHSTUM-UND-HABITABILITAET.md`).
//! - Grundausbildung: jede intakte Akademie bildet ohne Auftrag und ohne Güter Fachkräfte (und mit Labor Forscher)
//!   aus, bis der Bedarf der eigenen Werke gedeckt ist. Bezahlte Ausbildung (`ausbilden`) bleibt die Beschleunigung;
//!   Fachkräfte kosten dort keine Elektronik mehr (Elektronik entsteht nur mit Fachkräften: Henne-Ei).
//! - Startbesetzung: das Labor der Stufe 1 (Zivilisationsstufe I) ist vom Start an voll mit Forschern besetzt.
//!
//! Laufende Welten bleiben unverändert (Version 0); aktiviert wird nur beim Erzeugen einer neuen Welt
//! (`Welt::ausbau_aktivieren` zur Zeit 0). Der Zustand steht in `ausbau.ablauf` (JSON-Erweiterung des Snapshots),
//! gespiegelt nach `regeln.ablauf` (nicht serialisiert), damit Regeltext und Klima ohne Welt rechnen.
use crate::{typen::*, Welt};
use serde::Serialize;

/// Version, die neue Welten bekommen (1 = 17.10.2026, 2 = 19.10.2026: Solarkurve und Dysonschatten,
/// 3 = 22.10.2026: Grundausbildung ohne Sackgasse – siehe `grundausbildung_plan`).
pub const ABLAUF_NEU: u8 = 3;

/// Reserve über dem rechnerischen Bedarf: 10 Prozent plus fünf Personen.
pub const RESERVE_PROZENT: i64 = 10;
pub const RESERVE_PERSONEN: i64 = 5;

#[derive(Clone, Debug, Default, Serialize, PartialEq, Eq)]
pub struct Grundausbildung {
    /// Ausbildungsleistung je Spieltag in Tausendstel-Personen (nach Besetzung, Energie und Leistungsregler).
    pub je_tag_milli: i64,
    /// Volle Leistung je Spieltag laut Regel (`fachkraefte_je_akademie` × n × Ertragswachstum^n).
    pub voll_je_tag_milli: i64,
    /// Zielbestand (Personen in Tausendstel): Bedarf der Werke / Arbeitsquote + Reserve.
    pub ziel_fachkraefte: i64,
    pub ziel_forscher: i64,
    pub fachkraefte: i64,
    pub forscher: i64,
    /// Bereits bezahlte, noch laufende Ausbildung (Tausendstel-Personen).
    pub bezahlt_fachkraefte: i64,
    pub bezahlt_forscher: i64,
}

impl Grundausbildung {
    pub fn fehlen_fachkraefte(&self) -> i64 {
        (self.ziel_fachkraefte - self.fachkraefte - self.bezahlt_fachkraefte).max(0)
    }
    pub fn fehlen_forscher(&self) -> i64 {
        (self.ziel_forscher - self.forscher - self.bezahlt_forscher).max(0)
    }
}

fn personen_fuer(bedarf: i64, quote: f64) -> i64 {
    // Bedarf ist wirksame Arbeitskraft (Personen × Arbeitsquote); aufrunden auf ganze Personen.
    let q = milli(quote).max(1);
    let roh = (bedarf as i128 * 1000 + q as i128 - 1) / q as i128;
    ((roh as i64 + M - 1) / M) * M
}

/// Gebäude, die bei Unruhen weitergebaut werden (Ablauf v1): sie heben die Stabilität oder tragen Kolonien.
/// Ohne diese Ausnahme sperren zu viele Kolonien ohne Verwaltungszentrum jeden Bau, auch den des Zentrums.
pub const UNRUHEN_AUSNAHMEN: [Gebaeude; 3] = [Gebaeude::Verwaltungszentrum, Gebaeude::Konsumgueterwerk, Gebaeude::Wohnblock];

impl Welt {
    pub fn ablauf_v1(&self) -> bool {
        self.ausbau.ablauf >= 1
    }
    /// Ablauf v2 (neue Welten ab 19.10.2026): Solarkurve `SOLAR_V2` und Dysonschatten (`dyson::SCHATTEN`).
    pub fn ablauf_v2(&self) -> bool {
        self.ausbau.ablauf >= 2
    }
    /// Ablauf v3 (neue Welten ab 22.10.2026, Karl: „die Akademie soll automatisch funktionieren, dass Fachkräfte und
    /// Forschungskräfte kein Problem mehr sind“): die Grundausbildung bildet für eingereihte Ausbauten vor (ein neues
    /// Labor steht nicht mehr ohne Forscher da), und die Heimatwelt bildet auch ohne Akademie in Höhe einer Akademie
    /// der Stufe 1 aus – Forschung kann nie dauerhaft an fehlenden Forschern hängen.
    pub fn ablauf_v3(&self) -> bool {
        self.ausbau.ablauf >= 3
    }

    /// Ruht der vorderste Bauauftrag wegen Unruhen? (Stabilität unter `unruhen_unter`; Ablauf v1 nimmt
    /// die stabilisierenden Gebäude aus.)
    pub fn bau_ruht_unruhen(&self, pid: usize) -> bool {
        let p = &self.planeten[pid];
        if p.stabilitaet >= milli(self.regeln.stabilitaet.unruhen_unter) {
            return false;
        }
        !(self.ablauf_v1() && p.bauschleife.first().is_some_and(|a| UNRUHEN_AUSNAHMEN.contains(&a.gebaeude)))
    }

    /// Hinweise, warum die nächste Zivilisationsstufe nicht vorankommt, obwohl man es nicht direkt sieht:
    /// fehlende Felder, Aufstiegsgüter ohne laufende Herstellung, zu kleines Labor, fehlende Akademie, Unruhen.
    pub fn stufen_hinweise(&self, sid: SpielerId) -> Vec<String> {
        let mut v = Vec::new();
        if self.ausbau.version == 0 {
            return v;
        }
        let r = &self.regeln;
        let sp = &self.spieler[sid as usize];
        let Some(regel) = r.stufen.get(sp.stufe as usize - 1) else { return v };
        let planeten: Vec<usize> = sp.planeten.iter().map(|p| *p as usize).collect();
        let heim = sp.heimat as usize;
        // Felder für die verlangten Gebäudestufen (mit Voraussetzungen wie dem Raumhafen der Werft).
        let mut fehlend = 0u16;
        for (g, n) in &regel.gebaeude {
            let beste = planeten.iter().map(|p| self.planeten[*p].gebaeude[g.idx()]).max().unwrap_or(0);
            if beste >= *n {
                continue;
            }
            let p = &self.planeten[heim];
            fehlend += (*n - p.gebaeude[g.idx()].min(*n)) as u16;
            for (b, st) in &r.geb(*g).braucht {
                fehlend += st.saturating_sub(p.gebaeude[b.idx()]) as u16;
            }
        }
        let frei = planeten.iter().map(|p| self.felder_gesamt(*p).saturating_sub(self.felder_belegt(*p))).max().unwrap_or(0);
        if fehlend > frei {
            // Neue Welten (0.4.7): Felder gibt nicht mehr die Forschung, sondern der Terraformer (Mission terraformen).
            let ausweg = if r.wirtschaft.terraforming_felder > 0 { "Terraforming erforschen" }
                else if self.terraformer_welt() { "mit Terraformern Felder formen (Mission terraformen)" }
                else { "ab Stufe V einen Orbitalring bauen" };
            v.push(format!(
                "Für {} fehlen noch {fehlend} Gebäudestufen, aber auf keinem Planeten sind so viele Felder frei (höchstens {frei}): Gebäude abreißen (Aktion abreissen) oder {ausweg}.",
                regel.name
            ));
        }
        // Aufstiegsgüter, die fehlen und nirgends hergestellt werden.
        let kosten = crate::regeln::preis_array(&regel.kosten);
        let bestand = self.bestand_jetzt(heim);
        for g in Gut::ALLE {
            if kosten[g.idx()] <= bestand[g.idx()] || planeten.iter().any(|p| self.planeten[*p].rate[g.idx()] > 0) {
                continue;
            }
            let werk = match g {
                Gut::Erz => Gebaeude::Erzmine,
                Gut::Kristall => Gebaeude::Kristallmine,
                Gut::Deuterium => Gebaeude::Deuteriumsynthesizer,
                Gut::Legierung => Gebaeude::Giesserei,
                Gut::Elektronik => Gebaeude::Elektronikwerk,
                Gut::Konsumgut => Gebaeude::Konsumgueterwerk,
                Gut::Xenokristall => Gebaeude::Xenoextraktor,
                _ => continue,
            };
            let e = self.industrie_berechnen(&self.planeten[heim]);
            let grund = e.werke.iter().find(|w| w.gebaeude == werk).map(|w| {
                if w.engpaesse.is_empty() { "läuft, aber der Verbrauch ist höher".to_string() } else { format!("Engpass: {}", w.engpaesse.join(", ")) }
            }).unwrap_or_else(|| "ist auf der Heimatwelt nicht gebaut".to_string());
            v.push(format!("Aufstiegskosten: {g} fehlt und wird nirgends hergestellt – {werk} {grund}."));
        }
        // Labor für die verlangte Forschung.
        let labor = planeten.iter().map(|p| self.planeten[*p].gebaeude[Gebaeude::Labor.idx()]).max().unwrap_or(0);
        for (f, n) in &regel.forschung {
            let fr = r.forsch(*f);
            if sp.forschung[f.idx()] < *n && labor < fr.labor {
                v.push(format!("{f} {n} braucht ein Labor der Stufe {} (jetzt {labor}).", fr.labor));
            }
        }
        // Fachkräfte und Forscher ohne Akademie.
        for p in &planeten {
            let g = self.grundausbildung_plan(*p);
            if self.ablauf_v1() && g.fehlen_fachkraefte() + g.fehlen_forscher() > 0 && self.planeten[*p].gebaeude[Gebaeude::Akademie.idx()] == 0 && g.je_tag_milli == 0 {
                v.push(format!(
                    "Auf {} fehlen {} Fachkräfte/Forscher und es gibt keine Akademie: eine Akademie bildet automatisch und kostenlos aus.",
                    self.planeten[*p].koord,
                    (g.fehlen_fachkraefte() + g.fehlen_forscher() + M - 1) / M
                ));
            }
            if self.bau_ruht_unruhen(*p) {
                v.push(format!(
                    "Unruhen auf {}: Bauaufträge ruhen. Stabilität heben mit Konsumgütern, freiem Wohnraum, Soziologie, niedrigerem Steuersatz oder einem Verwaltungszentrum{}.",
                    self.planeten[*p].koord,
                    if self.ablauf_v1() { " (Verwaltungszentrum, Konsumgüterwerk und Wohnblock werden auch bei Unruhen gebaut)" } else { "" }
                ));
            }
        }
        v
    }

    /// Nur für neue Welten (Zeit 0, nach der Generation-12-Umstellung). Laufende Welten bleiben bei Version 0.
    pub fn ablauf_aktivieren(&mut self) {
        if self.ausbau.version == 0 || self.ausbau.ablauf >= 1 || self.zeit != 0 {
            return;
        }
        self.ausbau.ablauf = ABLAUF_NEU;
        {
            use sha2::{Digest, Sha256};
            let r = std::sync::Arc::make_mut(&mut self.regeln);
            r.ablauf = ABLAUF_NEU;
            r.hash = format!("{:x}", Sha256::digest(format!("{}:ablauf-v{ABLAUF_NEU}", r.hash)));
        }
        // Startbesetzung: Labor 1 voll mit Forschern (sonst läuft das einzige Labor der Stufe I auf 45 Prozent).
        let quote = self.regeln.wirtschaft.arbeitsquote;
        let labor1 = personen_fuer(self.regeln.stufenwert(self.regeln.geb(Gebaeude::Labor).fachkraefte, 1), quote);
        for pid in 0..self.planeten.len() {
            let volk = self.spieler[self.planeten[pid].besitzer as usize].volk;
            if let Some(p) = self.ausbau.personen.get_mut(&(pid as PlanetId)) {
                let g = &mut p.gruppen[volk.idx()];
                let fehlt = (labor1 - g[2]).max(0).min(g[0] / M * M);
                g[0] -= fehlt;
                g[2] += fehlt;
            }
        }
    }

    /// Bedarf, Bestand und Leistung der Grundausbildung eines Planeten (gleiche Rechnung für Tick und Sicht).
    pub fn grundausbildung_plan(&self, pid: usize) -> Grundausbildung {
        let p = &self.planeten[pid];
        let mut g = Grundausbildung::default();
        if !self.ablauf_v1() || self.ausbau.version == 0 {
            return g;
        }
        let people = self.ausbau.personen.get(&p.id).cloned().unwrap_or_default();
        g.fachkraefte = people.fachkraefte();
        g.forscher = people.forscher();
        for a in self.ausbau.ausbildungen.iter().filter(|a| a.planet == p.id) {
            if a.qualifikation == 1 {
                g.bezahlt_fachkraefte += a.anzahl * M;
            } else {
                g.bezahlt_forscher += a.anzahl * M;
            }
        }
        let e = self.industrie_berechnen(p);
        let w = &self.regeln.wirtschaft;
        let mut fk_bedarf: i64 = e.werke.iter().filter(|x| x.gebaeude != Gebaeude::Labor).map(|x| x.fachkraefte_bedarf).sum();
        let mut fo_bedarf: i64 = e.werke.iter().filter(|x| x.gebaeude == Gebaeude::Labor).map(|x| x.fachkraefte_bedarf).sum();
        if self.ablauf_v3() {
            // Vorlauf: Personal für eingereihte Ausbauten schon während der Bauzeit ausbilden.
            let mut stufe = p.gebaeude;
            for a in &p.bauschleife {
                let gr = self.regeln.geb(a.gebaeude);
                let alt = stufe[a.gebaeude.idx()];
                stufe[a.gebaeude.idx()] = a.stufe.max(alt);
                let plus = (self.regeln.stufenwert(gr.fachkraefte, stufe[a.gebaeude.idx()]) - self.regeln.stufenwert(gr.fachkraefte, alt)).max(0);
                if a.gebaeude == Gebaeude::Labor { fo_bedarf += plus } else { fk_bedarf += plus }
            }
        }
        let ziel = |bedarf: i64| {
            if bedarf <= 0 {
                0
            } else {
                let n = personen_fuer(bedarf, w.arbeitsquote);
                n + n * RESERVE_PROZENT / 100 / M * M + RESERVE_PERSONEN * M
            }
        };
        g.ziel_fachkraefte = ziel(fk_bedarf);
        g.ziel_forscher = if p.gebaeude[Gebaeude::Labor.idx()] > 0 || fo_bedarf > 0 { ziel(fo_bedarf) } else { 0 };
        let n = p.gebaeude[Gebaeude::Akademie.idx()];
        let heim_grund = self.ablauf_v3() && self.spieler[p.besitzer as usize].heimat as usize == pid;
        if n == 0 || self.integritaet(pid, Gebaeude::Akademie) < 1000 {
            if heim_grund {
                // Grundausbildung der Heimatwelt ohne (intakte) Akademie: wie Akademie 1, ohne Besetzung und Regler.
                g.voll_je_tag_milli = self.regeln.stufenwert(w.fachkraefte_je_akademie, 1) * self.fruehphase_faktor(p.besitzer);
                g.je_tag_milli = g.voll_je_tag_milli;
            }
            return g;
        }
        g.voll_je_tag_milli = self.regeln.stufenwert(w.fachkraefte_je_akademie, n);
        let besetzung = e.werke.iter().find(|x| x.gebaeude == Gebaeude::Akademie).map(|x| x.wirksame_besetzung_promille).unwrap_or(0);
        let regler = self.produktionsstufen(pid)[Gebaeude::Akademie.idx()] as i64 * 10;
        g.je_tag_milli = anteil(anteil(anteil(g.voll_je_tag_milli, besetzung, 1000), e.energie_deckung_promille, 1000), regler, 1000);
        if heim_grund {
            g.je_tag_milli = g.je_tag_milli.max(self.regeln.stufenwert(w.fachkraefte_je_akademie, 1));
        }
        let f = self.fruehphase_faktor(p.besitzer);
        g.voll_je_tag_milli *= f;
        g.je_tag_milli *= f;
        g
    }

    /// Stündliche Grundausbildung (aus `personen_tick`): kostenlos, nur aus freien unqualifizierten Bewohnern,
    /// nur bis zum Zielbestand. Bevorzugt das Reichsvolk, sonst die größte Gruppe mit freien Bewohnern.
    pub(crate) fn grundausbildung_tick(&mut self, pid: usize) {
        let plan = self.grundausbildung_plan(pid);
        let stunde = plan.je_tag_milli / 24;
        let (d_fk, d_fo) = (plan.fehlen_fachkraefte(), plan.fehlen_forscher());
        if stunde <= 0 || d_fk + d_fo == 0 {
            return;
        }
        let fk = anteil(stunde, d_fk, d_fk + d_fo).min(d_fk);
        let fo = (stunde - fk).min(d_fo);
        let id = self.planeten[pid].id;
        let reich = self.spieler[self.planeten[pid].besitzer as usize].volk;
        let reserviert = |v: Volk, w: &Welt| -> i64 {
            w.ausbau.ausbildungen.iter().filter(|a| a.planet == id && a.volk == v).map(|a| a.anzahl * M).sum()
        };
        let mut frei: Vec<(Volk, i64)> = Volk::ALLE
            .iter()
            .map(|v| (*v, self.ausbau.personen.get(&id).map(|p| p.gruppen[v.idx()][0]).unwrap_or(0) - reserviert(*v, self)))
            .filter(|(_, n)| *n > 0)
            .collect();
        frei.sort_by_key(|(v, n)| (*v != reich, std::cmp::Reverse(*n)));
        let Some(people) = self.ausbau.personen.get_mut(&id) else { return };
        for (q, mut menge) in [(1usize, fk), (2usize, fo)] {
            for (v, n) in frei.iter_mut() {
                let t = menge.min(*n);
                if t <= 0 {
                    continue;
                }
                people.gruppen[v.idx()][0] -= t;
                people.gruppen[v.idx()][q] += t;
                *n -= t;
                menge -= t;
            }
        }
    }

    pub fn grundausbildung_sicht(&self, pid: usize) -> serde_json::Value {
        if !self.ablauf_v1() {
            return serde_json::Value::Null;
        }
        let g = self.grundausbildung_plan(pid);
        let p = &self.planeten[pid];
        let akademie = p.gebaeude[Gebaeude::Akademie.idx()];
        let fehlen = g.fehlen_fachkraefte() + g.fehlen_forscher();
        // Deutsche Zahl mit Komma; ein Ziel von 0 wird nicht als „bis 0 Fachkräfte“ genannt (Funktionsprüfung 25.10.2026).
        let zahl = |m: i64| { let z = (m + 50) / 100; if z % 10 == 0 { tausender(z / 10) } else { format!("{},{}", tausender(z / 10), z % 10) } };
        let bis = match ((g.ziel_fachkraefte + M - 1) / M, (g.ziel_forscher + M - 1) / M) {
            (0, 0) => "bis der Bedarf gedeckt ist".to_string(),
            (0, fo) => format!("bis {} Forscher erreicht sind", tausender(fo)),
            (fk, 0) => format!("bis {} Fachkräfte erreicht sind", tausender(fk)),
            (fk, fo) => format!("bis {} Fachkräfte und {} Forscher erreicht sind", tausender(fk), tausender(fo)),
        };
        let hinweis = if self.fruehphase_faktor(p.besitzer) > 1 {
            format!(
                "Vor Zivilisationsstufe III braucht kein Werk Personal – alle laufen voll. Die Grundausbildung läuft trotzdem doppelt so schnell weiter ({} Personen je Spieltag), damit ab Stufe III genug Fachkräfte und Forscher da sind.",
                zahl(g.je_tag_milli)
            )
        } else if fehlen == 0 {
            "Fachkräfte und Forscher decken den Bedarf der Werke.".to_string()
        } else if akademie == 0 && g.je_tag_milli > 0 {
            format!(
                "Grundausbildung der Heimatwelt läuft automatisch und kostenlos: {} Personen je Spieltag, {bis}. Eine Akademie bildet schneller aus.",
                zahl(g.je_tag_milli)
            )
        } else if akademie == 0 {
            format!(
                "Es fehlen {} Fachkräfte/Forscher. Ohne Akademie entstehen keine: eine Akademie bauen (bildet dann automatisch aus) oder qualifizierte Bewohner herbringen.",
                (fehlen + M - 1) / M
            )
        } else if g.je_tag_milli == 0 {
            "Die Akademie ruht (Besetzung, Energie, Integrität oder Leistungsregler): keine Grundausbildung.".to_string()
        } else {
            format!(
                "Grundausbildung läuft automatisch und kostenlos: {} Personen je Spieltag, {bis}. Bezahlte Ausbildung beschleunigt.",
                zahl(g.je_tag_milli)
            )
        };
        serde_json::json!({
            "je_tag": g.je_tag_milli as f64 / M as f64, "voll_je_tag": g.voll_je_tag_milli as f64 / M as f64,
            "ziel_fachkraefte": ganz(g.ziel_fachkraefte), "ziel_forscher": ganz(g.ziel_forscher),
            "fachkraefte": ganz(g.fachkraefte), "forscher": ganz(g.forscher),
            "fehlen_fachkraefte": ganz(g.fehlen_fachkraefte()), "fehlen_forscher": ganz(g.fehlen_forscher()),
            "kostenlos": true, "hinweis": hinweis,
        })
    }
}
