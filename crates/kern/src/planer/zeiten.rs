use super::modell::ErklaerterWert;
use crate::{typen::*, welt::Planet, Regelwerk};
use serde::Serialize;

#[derive(Clone, Debug, Serialize)]
pub struct ZeitBerechnung {
    pub masse_milli: i64,
    pub werk: Gebaeude,
    pub werk_faktor_promille: String,
    pub nanofabrik_faktor_promille: String,
    pub teiler: String,
    pub sekunden: ErklaerterWert,
}
/// Both construction and manufacturing retain their original order: integer
/// division, then (manufacturing only) faction, then their distinct minimum.
pub fn berechnen(
    r: &Regelwerk,
    p: &Planet,
    kosten: &[i64; GUETER],
    hp: &[i64; GEBAEUDE],
    fertigung: Option<(usize, Volk)>,
) -> ZeitBerechnung {
    let masse =
        kosten[Gut::Erz.idx()] + kosten[Gut::Kristall.idx()] + r.kw().produktion.baumasse_legierung * kosten[Gut::Legierung.idx()];
    let werk = match fertigung {
        None => Gebaeude::Bauhof,
        Some((0, _)) => Gebaeude::Werft,
        Some(_) => Gebaeude::Orbitalwerft,
    };
    let b = 1000 + p.gebaeude[werk.idx()] as i128 * hp[werk.idx()] as i128;
    let nano = p.gebaeude[Gebaeude::Nanofabrik.idx()] as u32;
    let n = 1000 + ((1i128 << nano.min(40)) - 1) * hp[Gebaeude::Nanofabrik.idx()] as i128;
    let teiler = (r.wirtschaft.bauzeit_teiler as i128 * M as i128 * b * n / 1_000_000).max(1);
    let basis = (masse as i128 * STUNDE as i128 / teiler) as i64;
    let mut zeit = ErklaerterWert::neu(basis, "spielsekunden");
    if let Some((_, volk)) = fertigung {
        zeit = zeit
            .faktor("volk_werftzeit", r.volk(volk).werftzeit)
            .mindestens("mindest_fertigungszeit", 30);
    } else {
        zeit = zeit.mindestens("mindest_bauzeit", r.wirtschaft.min_bauzeit_sekunden);
    }
    ZeitBerechnung {
        masse_milli: masse,
        werk,
        werk_faktor_promille: b.to_string(),
        nanofabrik_faktor_promille: n.to_string(),
        teiler: teiler.to_string(),
        sekunden: zeit,
    }
}
