//! Sonnenabstand, herkunftsabhängige Besiedlung und Terraforming aus einer Regelquelle.
use crate::{ausbau::Personen, typen::*, Welt};
use serde::Serialize;

// Zwölf Referenzbahnen; anders große Systeme werden auf dieselbe Distanzskala abgebildet.
pub const WACHSTUM: [[i64; 12]; VOELKER] = [
    [0, 0, 100, 650, 1400, 2000, 2000, 1400, 650, 100, 0, 0],
    [1000, 1600, 2000, 1600, 1200, 1000, 800, 350, 100, 0, 0, 0],
    [0, 0, 0, 250, 750, 1250, 1750, 2000, 1600, 800, 150, 0],
    [0, 0, 200, 450, 650, 800, 1000, 1250, 1500, 1750, 2000, 2000],
];
pub const FREIGABE: [[u8; 12]; VOELKER] = [
    [10, 6, 0, 0, 0, 0, 0, 0, 0, 0, 6, 10],
    [0, 0, 0, 0, 0, 0, 0, 0, 0, 4, 7, 10],
    [10, 7, 4, 0, 0, 0, 0, 0, 0, 0, 0, 6],
    [10, 6, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
];
pub const SOLAR: [i64; 12] = [
    1800, 1600, 1400, 1200, 1100, 1000, 900, 750, 550, 350, 200, 100,
];
pub const ENDSPIEL_TECH: u8 = 10;
pub const OPTIMUM: i64 = 2000;

// ---------------------------------------------------------------- Ablauf-Version 1 (neue Welten)
// Faktoren relativ zum Grundwert `wirtschaft.wachstum_je_tag` in Promille: 1000 = Grundwert = Klimaoptimum.
// Jedes Volk hat seine Heimatbahn (Startplatz aus `Welt::heimat_anpassen`); der Faktor hängt nur vom Abstand
// der Bahn zur Heimatbahn ab (ABSTANDSFAKTOR). Gesperrte Bahnen (FREIGABE_V1 > 0) brauchen Terraforming.
// Beispiel (Karl): Aurelianer (Menschen) auf Platz 1 = Abstand 5 = 700 ‰, also 30 % langsamer.
// Tabelle und Rechenbeispiele: docs/WACHSTUM-UND-HABITABILITAET.md.
/// Heimatbahn je Volk (0-basiert): Aurelianer Platz 6, Krath Platz 3, Veyari Platz 8, Syntheten Platz 10.
/// Krath und Syntheten liegen symmetrisch je zwei Bahnen vom Rand (Syntheten bisher Platz 11 mit 20 % Sonne).
pub const HEIMATBAHN: [usize; VOELKER] = [5, 2, 7, 9];
/// Klimafaktor nach Abstand zur Heimatbahn (0–11 Bahnen).
pub const ABSTANDSFAKTOR: [i64; 12] = [1000, 1000, 950, 850, 780, 700, 620, 550, 480, 410, 350, 300];
pub const OPTIMUM_V1: i64 = 1000;
pub const WACHSTUM_V1: [[i64; 12]; VOELKER] = [
    [700, 780, 850, 950, 1000, 1000, 1000, 950, 850, 780, 700, 620],
    [950, 1000, 1000, 1000, 950, 850, 780, 700, 620, 550, 480, 410],
    [550, 620, 700, 780, 850, 950, 1000, 1000, 1000, 950, 850, 780],
    [410, 480, 550, 620, 700, 780, 850, 950, 1000, 1000, 1000, 950],
];
/// Nötiges Terraforming je Bahn: ab Abstand 8 zur Heimatbahn gesperrt (Abstand 8: Stufe 4, Abstand 9: Stufe 7).
/// Das trifft nur die Randvölker am entgegengesetzten Rand: Krath im tiefen Frost, Syntheten in der inneren Glut.
pub const FREIGABE_V1: [[u8; 12]; VOELKER] = [
    [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 4, 7],
    [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    [7, 4, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
];
/// Terraforming-Bedarf nach Abstand zur Heimatbahn (Regel hinter FREIGABE_V1).
pub const FREIGABE_ABSTAND: [u8; 12] = [0, 0, 0, 0, 0, 0, 0, 0, 4, 7, 10, 10];
/// Heimatposition eines Volkes (1-basiert, bei 12 Plätzen); `ablauf` 0 behält die bisherigen Startplätze.
pub fn heimatposition(volk: Volk, ablauf: u8) -> u8 {
    match volk {
        Volk::Aurelianer => 6,
        Volk::Krath => 3,
        Volk::Veyari => 8,
        Volk::Syntheten => if ablauf >= 1 { 10 } else { 11 },
    }
}
pub fn optimum(ablauf: u8) -> i64 {
    if ablauf >= 1 { OPTIMUM_V1 } else { OPTIMUM }
}
pub fn tabellen(ablauf: u8) -> (&'static [[i64; 12]; VOELKER], &'static [[u8; 12]; VOELKER]) {
    if ablauf >= 1 { (&WACHSTUM_V1, &FREIGABE_V1) } else { (&WACHSTUM, &FREIGABE) }
}
pub const RESTKLIMA: i64 = 100; // zehn Prozent der ursprünglichen Abweichung bleiben
pub const KLIMAVERLUST: i64 = 20; // höchstens 2 % je Stunde ohne bewohnbare Habitate

pub fn bahn(position: u8, plaetze: u8) -> usize {
    ((position.saturating_sub(1) as usize * 11) / (plaetze.saturating_sub(1).max(1) as usize))
        .min(11)
}
pub fn zone(position: u8, plaetze: u8) -> Zone {
    match bahn(position, plaetze) {
        0..=2 => Zone::Glut,
        3..=7 => Zone::Leben,
        _ => Zone::Frost,
    }
}
/// Sonnenfaktor nach der ursprünglichen Tabelle (Version 0, laufende Welten).
pub fn solar(position: u8, plaetze: u8) -> i64 {
    SOLAR[bahn(position, plaetze)]
}
/// Ablauf v1 (Karl 17.10.2026): Platz 6 = 100 % Grundwert, letzter Platz = 20 % (bisher 10 %). Affine Abbildung
/// der bisherigen Kurve (s ↦ 200 + (s − 100) × 8/9): Form und Reihenfolge bleiben, Platz 6 bleibt 1000,
/// Platz 12 steigt von 100 auf 200; innen leicht flacher (Platz 1: 1711 statt 1800).
pub const SOLAR_V1: [i64; 12] = [1711, 1533, 1356, 1178, 1089, 1000, 911, 778, 600, 422, 289, 200];
/// Ablauf v2 (Karl 19.10.2026): „Planet 6 ist Grundlage 100 %, Planet 1 bei 120 % und Planet 12 bei 80 % … weniger
/// Solarenergie, wenn man weiter von der Sonne weg ist.“ Linear in zwei Abschnitten: innen +40 ‰ je Platz, außen −33⅓ ‰.
pub const SOLAR_V2: [i64; 12] = [1200, 1160, 1120, 1080, 1040, 1000, 967, 933, 900, 867, 833, 800];
pub fn solar_v(position: u8, plaetze: u8, ablauf: u8) -> i64 {
    if ablauf >= 2 { SOLAR_V2[bahn(position, plaetze)] } else if ablauf >= 1 { SOLAR_V1[bahn(position, plaetze)] } else { solar(position, plaetze) }
}
/// Sonnenfaktor eines Platzes in Promille mit den Kernwerten der Welt: Ablauf ≥ 2 nimmt die Solarkurve aus
/// `kernwerte.sonne.solar_promille` (Standard `SOLAR_V2`) mal der Strahlkraft der Sternklasse des Systems
/// (Standard 1000 ‰); ältere Abläufe wie bisher `solar_v`.
pub fn solar_r(r: &crate::Regelwerk, k: Koord) -> i64 {
    if r.ablauf >= 2 {
        let s = r.kw().sonne.solar(bahn(k.position, r.welt.plaetze_je_system));
        let stern = r.stern_promille(k);
        if stern == 1000 { s } else { s * stern / 1000 }
    } else {
        solar_v(k.position, r.welt.plaetze_je_system, r.ablauf)
    }
}
pub fn faktoren(r: &crate::Regelwerk, p: &crate::welt::Planet) -> [i64; 5] {
    let mut f = p.faktor;
    let z = &r.zonen[&zone(p.koord.position, r.welt.plaetze_je_system)];
    f[2] = milli(z.deuterium);
    f[3] = milli(z.nahrung);
    f[4] = solar_r(r, p.koord);
    f
}

#[derive(Clone, Debug, Serialize)]
pub struct Eignung {
    pub volk: Volk,
    pub bewohnbar: bool,
    pub terraforming_noetig: u8,
    pub terraforming: u8,
    pub wachstum_promille: i64,
    pub klimaverlust_promille_je_stunde: i64,
}
/// Klimaeignung nach der ursprünglichen Tabelle (Version 0, laufende Welten).
pub fn eignung(volk: Volk, position: u8, plaetze: u8, technik: u8) -> Eignung {
    eignung_v(volk, position, plaetze, technik, 0)
}
/// Klimaeignung für eine Ablauf-Version (0: alte Tabelle, Optimum 2000; 1: relative Tabelle, Optimum 1000).
pub fn eignung_v(volk: Volk, position: u8, plaetze: u8, technik: u8, ablauf: u8) -> Eignung {
    let (wachstum, freigabe) = tabellen(ablauf);
    let opt = optimum(ablauf);
    let i = bahn(position, plaetze);
    let tech = technik.min(ENDSPIEL_TECH);
    let noetig = freigabe[volk.idx()][i];
    let bewohnbar = tech >= noetig;
    let rest = 1000 - (1000 - RESTKLIMA) * tech as i64 / ENDSPIEL_TECH as i64;
    Eignung {
        volk,
        bewohnbar,
        terraforming_noetig: noetig,
        terraforming: technik,
        wachstum_promille: if bewohnbar {
            opt + (wachstum[volk.idx()][i] - opt) * rest / 1000
        } else {
            0
        },
        klimaverlust_promille_je_stunde: if bewohnbar {
            0
        } else {
            KLIMAVERLUST * (noetig - tech) as i64 / noetig.max(1) as i64
        },
    }
}
/// Klimaeignung mit den Kernwerten der Welt (Ablauf ≥ 1: Tabellen `kernwerte.klima`, Standard WACHSTUM_V1/FREIGABE_V1;
/// Ablauf 0 wie bisher). Rechnet bei Standardwerten genau wie [`eignung_v`].
pub fn eignung_r(r: &crate::Regelwerk, volk: Volk, position: u8, technik: u8) -> Eignung {
    if r.ablauf == 0 {
        return eignung_v(volk, position, r.welt.plaetze_je_system, technik, 0);
    }
    let k = &r.kw().klima;
    let i = bahn(position, r.welt.plaetze_je_system);
    let ende = k.endspiel_tech.max(1);
    let tech = technik.min(ende);
    let noetig = k.freigabe(volk)[i];
    let bewohnbar = tech >= noetig;
    let rest = 1000 - (1000 - k.restklima_promille) * tech as i64 / ende as i64;
    Eignung {
        volk,
        bewohnbar,
        terraforming_noetig: noetig,
        terraforming: technik,
        wachstum_promille: if bewohnbar { OPTIMUM_V1 + (k.tabelle(volk)[i] - OPTIMUM_V1) * rest / 1000 } else { 0 },
        klimaverlust_promille_je_stunde: if bewohnbar { 0 } else { k.klimaverlust_promille * (noetig - tech) as i64 / noetig.max(1) as i64 },
    }
}
impl Personen {
    /// A founding complement has exactly one origin, while qualifications are preserved.
    pub fn auswahl_volk(&self, volk: Volk, milli: i64) -> Self {
        let mut one = Self::default();
        one.gruppen[volk.idx()] = self.gruppen[volk.idx()];
        one.auswahl(milli)
    }
}
impl Welt {
    pub fn koloniebedarf(
        &self,
        sid: SpielerId,
        typ: Einheit,
        volk: Volk,
    ) -> Result<[i64; GUETER], String> {
        if !matches!(typ, Einheit::Kolonieschiff | Einheit::GrossesKolonieschiff) {
            return Err("Kolonieschiff wählen".into());
        }
        let mut cargo = self.koloniefracht(sid);
        let anzahl = self
            .regeln
            .flottendoktrin
            .get(&typ)
            .map(|p| p.siedler)
            .unwrap_or(self.regeln.wirtschaft.siedler);
        cargo[Gut::Nahrung.idx()] = if volk == Volk::Syntheten {
            0
        } else {
            mal(anzahl * M, self.regeln.wirtschaft.nahrung_je_1000) * 24 / 1000
        };
        Ok(cargo)
    }

    pub fn planet_eignung(&self, sid: SpielerId, k: Koord, volk: Volk) -> Eignung {
        let e = eignung_r(&self.regeln, volk, k.position, self.spieler[sid as usize].forschung[Forschung::Terraforming.idx()]);
        // Terraformer-Schiffe (0.4.7, `crate::terraform`): geformtes Klima an dieser Koordinate; ohne Stand unverändert.
        if self.ausbau.terraform.is_empty() { e } else { self.terraform_eignung(k, e) }
    }
    /// Höchstes Wachstum je Spieltag einer Herkunft an dieser Koordinate in Promille der Personenzahl
    /// (Grundwert × Volksfaktor × Klimafaktor, vor Wohnraum, Versorgung, Stabilität und Xeno-Abzug).
    pub fn wachstum_je_tag_promille(&self, sid: SpielerId, k: Koord, volk: Volk) -> i64 {
        let e = self.planet_eignung(sid, k, volk);
        let basis = milli(self.regeln.wirtschaft.wachstum_je_tag * self.regeln.volk(volk).wachstum);
        let mut v = basis * e.wachstum_promille / 1000;
        if self.xeno_planet(k) && v > 0 {
            v = v * self.regeln.kw().klima.xeno_wachstum_promille / 1000;
        }
        v
    }
    pub fn klima_sicht(&self, sid: SpielerId, k: Koord) -> serde_json::Value {
        if self.ausbau.version == 0 || k.position == 0 || self.platz(k).is_none() {
            return serde_json::Value::Null;
        }
        let herkunft: Vec<serde_json::Value> = Volk::ALLE.iter().map(|v| {
            let mut e = serde_json::to_value(self.planet_eignung(sid, k, *v)).unwrap_or_default();
            if self.regeln.ablauf >= 1 {
                e["volksfaktor"] = serde_json::json!(self.regeln.volk(*v).wachstum);
                e["wachstum_je_tag_promille"] = serde_json::json!(self.wachstum_je_tag_promille(sid, k, *v));
            }
            e
        }).collect();
        let solar = solar_r(&self.regeln, k);
        let dyson = self.dyson_sonnen_promille(sid, k);
        serde_json::json!({"zone":zone(k.position,self.regeln.welt.plaetze_je_system),
            "solar_promille":solar,"dyson_promille":dyson,"solar_wirksam_promille":solar*dyson/1000,
            "platz":k.position,"klimatabelle":if self.regeln.ablauf>=1{"relativ_v1"}else{"v0"},
            "grundwert_je_tag_promille":milli(self.regeln.wirtschaft.wachstum_je_tag),
            "optimum_promille":optimum(self.regeln.ablauf),
            "herkunft":herkunft,
            "hinweis":"Klima nach Sonnenabstand und eigener Terraforming-Technik. Wachstum zusätzlich abhängig von Herkunft, Versorgung, Stabilität und Wohnraum. Landungen bleiben möglich; unbewohnbare Gruppen verlieren Bewohner."})
    }
    /// Apply compatible climate rules at activation / the next ordinary economic tick.
    /// Loading does not rewrite saved hashes or introduce another restart boundary.
    pub(crate) fn klima_angleichen(&mut self) {
        if self.ausbau.version == 0 {
            return;
        }
        let n = self.regeln.welt.plaetze_je_system;
        for (i, p) in self.plaetze.iter_mut().enumerate() {
            p.zone = zone((i % n as usize + 1) as u8, n);
        }
        let mut changed = Vec::new();
        for (pid, p) in self.planeten.iter_mut().enumerate() {
            let next_zone = zone(p.koord.position, n);
            let next = faktoren(&self.regeln, p);
            if p.zone != next_zone || p.faktor != next {
                changed.push(pid);
            }
            p.zone = next_zone;
            p.faktor = next;
        }
        for sp in &mut self.spieler {
            for (k, e) in &mut sp.erkundet {
                e.zone = zone(k.position, n);
            }
        }
        for ((_, k), s) in &mut self.scans {
            s.zone = zone(k.position, n);
        }
        for pid in changed {
            self.raten_neu(pid);
        }
    }
    pub(crate) fn siedler_eignung_pruefen(
        &self,
        sid: SpielerId,
        ziel: Koord,
        p: &Personen,
        anzahl: i64,
    ) -> Result<(), String> {
        let origins: Vec<_> = Volk::ALLE
            .into_iter()
            .filter(|v| p.gruppen[v.idx()].iter().sum::<i64>() > 0)
            .collect();
        if p.gesamt() != anzahl || origins.len() != 1 {
            return Err(
                "Kolonieschiff benötigt seine vollständige Siedlerzahl aus genau einem Volk".into(),
            );
        }
        let e = self.planet_eignung(sid, ziel, origins[0]);
        if !e.bewohnbar {
            return Err(format!("{} können auf {} erst mit Terraforming {} siedeln; wähle ein geeignetes Siedlervolk",origins[0],ziel,e.terraforming_noetig));
        }
        Ok(())
    }
}
