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
                for art in 0..2 {militaer += p.raketen[art] * r.wert(&crate::regeln::preis_array(&r.zusatz.raketen_kosten[art]));}
            }
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

    /// Rangliste: Name, Gesamtpunkte, Stufe, nach Rang sortiert.
    pub fn rangliste(&self) -> Vec<(u16, String, i64, u8)> {
        let mut v: Vec<(u16, String, i64, u8)> =
            self.spieler.iter().filter(|s|!self.aufklaerung.inaktive_spieler.contains(&s.id)).map(|s| (s.rang, s.name.clone(), s.punkte.gesamt(), s.stufe)).collect();
        v.sort();
        v
    }
}
