use super::modell::{KostenErgebnis, KostenPosition};
use crate::{typen::*, Regelwerk};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "art", rename_all = "snake_case", deny_unknown_fields)]
pub enum KostenAnfrage {
    Gebaeude {
        gebaeude: Gebaeude,
        von: u8,
        bis: u8,
    },
    Forschung {
        forschung: Forschung,
        von: u8,
        bis: u8,
    },
    Einheit {
        einheit: Einheit,
        anzahl: i64,
    },
    Bauteil {
        bauteil: Gut,
        anzahl: i64,
    },
}

/// Uses exactly the prices charged by bauen/forschen/fertigen, including each
/// intermediate level's rounding. Does not reserve or subtract paid orders again.
pub fn berechnen(
    r: &Regelwerk,
    volk: Volk,
    anfragen: &[KostenAnfrage],
) -> Result<KostenErgebnis, String> {
    if anfragen.is_empty() || anfragen.len() > 32 {
        return Err("1 bis 32 Kostenpositionen angeben".into());
    }
    let mut out = KostenErgebnis {
        einheit: "tausendstel_gut",
        positionen: Vec::new(),
        gesamt: [0; GUETER],
    };
    let mut add = |kennung: String,
                   stufe: Option<u8>,
                   anzahl: i64,
                   stueck: [i64; GUETER]|
     -> Result<(), String> {
        if !(1..=10_000).contains(&anzahl) {
            return Err("Serienmenge: 1 bis 10.000".into());
        }
        let mut gesamt = [0; GUETER];
        for g in 0..GUETER {
            gesamt[g] = stueck[g].checked_mul(anzahl).ok_or("Kostenüberlauf")?;
            out.gesamt[g] = out.gesamt[g]
                .checked_add(gesamt[g])
                .ok_or("Kostenüberlauf")?;
        }
        out.positionen.push(KostenPosition {
            kennung,
            stufe,
            anzahl,
            stueck,
            gesamt,
        });
        Ok(())
    };
    let bereich = |von: u8, bis: u8, max: u8| -> Result<(), String> {
        if von >= bis || bis > max || bis - von > 32 {
            return Err(format!(
                "Stufenbereich: 0 ≤ von < bis ≤ {max}, höchstens 32 Ausbauten"
            ));
        }
        Ok(())
    };
    for a in anfragen {
        match *a {
            KostenAnfrage::Gebaeude { gebaeude, von, bis } => {
                if !r.gebaeude.contains_key(&gebaeude) {
                    return Err("Gebäude im Regelprofil nicht aktiv".into());
                }
                bereich(von, bis, r.welt.max_gebaeudestufe)?;
                for stufe in von + 1..=bis {
                    add(
                        gebaeude.name().into(),
                        Some(stufe),
                        1,
                        r.kosten_gebaeude(gebaeude, stufe),
                    )?;
                }
            }
            KostenAnfrage::Forschung {
                forschung,
                von,
                bis,
            } => {
                if !r.forschung.contains_key(&forschung) {
                    return Err("Forschung im Regelprofil nicht aktiv".into());
                }
                bereich(von, bis, crate::regeln::MAX_FORSCHUNG)?;
                for stufe in von + 1..=bis {
                    add(
                        forschung.name().into(),
                        Some(stufe),
                        1,
                        r.kosten_forschung(forschung, stufe),
                    )?;
                }
            }
            KostenAnfrage::Einheit { einheit, anzahl } => {
                if !r.einheiten.contains_key(&einheit) {
                    return Err("Einheit im Regelprofil nicht aktiv".into());
                }
                add(
                    einheit.name().into(),
                    None,
                    anzahl,
                    r.kosten_einheit(einheit, volk),
                )?;
            }
            KostenAnfrage::Bauteil { bauteil, anzahl } => {
                let cost = r
                    .kosten_bauteil(bauteil)
                    .ok_or("Kein herstellbares Bauteil im Regelprofil")?;
                add(bauteil.name().into(), None, anzahl, cost)?;
            }
        }
    }
    Ok(out)
}
