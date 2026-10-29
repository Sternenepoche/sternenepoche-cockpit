//! Punktwertung: gezählt wird investierter Wert, nicht gehortete Rohstoffe.

use crate::typen::*;
use crate::welt::*;

impl Welt {
    /// Rechnet Teilpunkte und Rang aller Spieler neu.
    pub fn punkte_neu(&mut self) {
        let r = self.regeln.clone();
        let einheit = r.wertung.einheit * M;
        for sid in 0..self.spieler.len() {
            if self.ist_besiegt(sid as SpielerId) {continue;}
            if !self.spieler_aktiv(sid as SpielerId) {
                self.spieler[sid].punkte = Punkte::default(); self.spieler[sid].rang = 0; continue;
            }
            let sp = &self.spieler[sid];
            let mut wirtschaft = 0i64;
            let mut militaer = self.flottenwert(sid as SpielerId);
            for pid in &sp.planeten {
                let p = &self.planeten[*pid as usize];
                for g in Gebaeude::ALLE {
                    wirtschaft += r.wert_gebaeude_bis(g, p.gebaeude[g.idx()]);
                }
                for e in SCHIFFE..EINHEITEN {
                    if p.einheiten[e] > 0 {
                        militaer += p.einheiten[e] * r.wert(&r.kosten_einheit(Einheit::ALLE[e], sp.volk));
                    }
                }
                if self.ausbau.version>0 {for art in 0..5 {militaer+=self.raketen_bestand(*pid as usize,art)*r.wert(&self.taktisch_kosten(art,1).unwrap());}}else{for art in 0..2 {militaer += p.raketen[art] * r.wert(&crate::regeln::preis_array(&r.zusatz.raketen_kosten[art]));}}
            }
            if self.monde.aktiv { let (w, m) = self.mond_punkte(sid as SpielerId); wirtschaft += w; militaer += m; }
            // Invasion v1 (neue Welten): Bodentruppen zählen mit ihrem Güterwert zum Militär (D6, 18.10.2026).
            if self.invasion_v1() { militaer += self.bodentruppen_wert(sid as SpielerId); }
            let forschung: i64 = Forschung::ALLE.iter().map(|f| r.wert_forschung_bis(*f, sp.forschung[f.idx()])).sum();
            let zivilisation = self.einwohner(sid as SpielerId) / (r.wertung.einwohner_je_punkt * M)
                + r.wertung.stufenbonus[(sp.stufe as usize - 1).min(4)];
            let punkte = Punkte {
                wirtschaft: wirtschaft / einheit,
                forschung: forschung / einheit,
                militaer: militaer / einheit,
                zivilisation,
            };
            self.spieler[sid].punkte = punkte;
        }
        let mut ordnung: Vec<(bool, i64, SpielerId)> = self.spieler.iter().filter(|s|!self.aufklaerung.inaktive_spieler.contains(&s.id)).map(|s| (self.ist_besiegt(s.id), -s.punkte.gesamt(), s.id)).collect();
        ordnung.sort();
        for (rang, (_, _, sid)) in ordnung.iter().enumerate() {
            self.spieler[*sid as usize].rang = rang as u16 + 1;
        }
    }

    /// Güterwert (Tausendstel Werteinheiten) aller fertigen Bodentruppen eines Spielers: Garnisonen auf eigenen
    /// Planeten und verladene Truppen in eigenen Flotten. Aufträge in der Kaserne zählen wie Werftaufträge noch nicht.
    pub fn bodentruppen_wert(&self, sid: SpielerId) -> i64 {
        let r = &self.regeln;
        let wert = |truppen: &[i64; crate::boden::ANZAHL]| -> i64 {
            (0..crate::boden::ANZAHL).map(|t| truppen[t].max(0) * r.wert(&crate::boden::kosten(t))).sum()
        };
        let b = &self.ausbau.boden;
        let planeten: i64 = b.bestand.iter().filter(|(pid, _)| self.planeten.get(**pid as usize).is_some_and(|p| p.besitzer == sid)).map(|(_, t)| wert(t)).sum();
        let flotten: i64 = b.flotten.iter().filter(|(fid, _)| self.flotten.get(fid).is_some_and(|f| f.besitzer == sid)).map(|(_, t)| wert(t)).sum();
        planeten + flotten
    }

    /// Rangliste: Name, Gesamtpunkte, Stufe, nach Rang sortiert.
    pub fn rangliste(&self) -> Vec<(u16, String, i64, u8)> {
        let mut v: Vec<(u16, String, i64, u8)> =
            self.spieler.iter().filter(|s|!self.aufklaerung.inaktive_spieler.contains(&s.id)).map(|s| (s.rang, s.name.clone(), s.punkte.gesamt(), s.stufe)).collect();
        v.sort();
        v
    }
}
