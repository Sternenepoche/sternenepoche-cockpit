//! Shared public simulation stream and the same cargo rounding as actual looting.
use crate::{
    kampf::{kampf_ohne_protokoll, Gruppe},
    typen::*,
    welt::{strom, Sieger, KANAL_SIMULATOR},
    Regelwerk,
};
use serde_json::{json, Value};

pub const SZENARIO_SEED: u64 = 0x5343454e4152494f;

pub fn beute(
    r: &Regelwerk,
    bestand: &[i64; GUETER],
    schutz: &[i64; GUETER],
    quote: f64,
    frei: i64,
) -> [i64; GUETER] {
    let mut result = [0; GUETER];
    for i in 0..8 {
        result[i] = mal((bestand[i] - schutz[i]).max(0), quote);
    }
    let summe: i64 = result.iter().sum();
    if summe > frei {
        for value in &mut result {
            *value = anteil(*value, frei, summe);
        }
    }
    let _ = r; // Rule quote and bunker protection are supplied by the caller.
    result
}

pub fn berechnen(
    r: &Regelwerk,
    a: &[(Gruppe, Volk, u8)],
    d: &[(Gruppe, Volk, u8)],
    seed: u64,
    bestand: Option<[i64; GUETER]>,
    bunker: u8,
    vorladung: i64,
    bergung: u16,
) -> Result<Value, String> {
    if a.is_empty() || d.is_empty() || a.len() > 8 || d.len() > 8 {
        return Err("Je Seite 1 bis 8 Gruppen".into());
    }
    let counts: Vec<i64> = a.iter().chain(d).flat_map(|g| g.0.einheiten).collect();
    crate::spielhilfe::simulator_grenzen(&counts, &[])?;
    let runs = r
        .kampf
        .simulator_laeufe
        .clamp(1, crate::spielhilfe::SIMULATOR_MAX_LAEUFE);
    let attackers: Vec<_> = a.iter().map(|g| g.0.clone()).collect();
    let defenders: Vec<_> = d.iter().map(|g| g.0.clone()).collect();
    let (mut wins, mut draws) = (0, 0);
    let (mut la, mut ld) = (0i128, 0i128);
    let (mut sa, mut sd) = ([0i64; EINHEITEN], [0i64; EINHEITEN]);
    let mut details = Vec::new();
    for i in 0..runs {
        let mut rng = strom(seed, KANAL_SIMULATOR, i as u64);
        let result = kampf_ohne_protokoll(r, &attackers, &defenders, &mut rng);
        if result.sieger == Sieger::Angreifer {
            wins += 1;
        }
        if result.sieger == Sieger::Unentschieden {
            draws += 1;
        }
        let mut losses = [[0i64; EINHEITEN]; 2];
        let mut values = [0i128; 2];
        let mut debris = [0i64; 2];
        for (side, groups, survivors) in [(0, a, &result.angreifer), (1, d, &result.verteidiger)] {
            for (gi, (group, volk, _)) in groups.iter().enumerate() {
                for e in Einheit::ALLE {
                    let ix = e.idx();
                    let n = group.einheiten[ix] - survivors[gi][ix];
                    losses[side][ix] += n;
                    if side == 0 {
                        sa[ix] += survivors[gi][ix];
                    } else {
                        sd[ix] += survivors[gi][ix];
                    }
                    let cost = r.kosten_einheit(e, *volk);
                    values[side] += n as i128 * r.wert(&cost) as i128;
                    // Recovery is a user scenario, never an automatic refund.
                    if e.ist_schiff() {
                        for j in 0..2 {
                            debris[j] = debris[j].saturating_add(n.saturating_mul(cost[j]));
                        }
                    }
                }
            }
        }
        la += values[0];
        ld += values[1];
        let capacity: i64 = a
            .iter()
            .enumerate()
            .map(|(gi, (_, volk, logistik))| {
                let base: i64 = (0..SCHIFFE)
                    .map(|e| result.angreifer[gi][e] * r.ladung_milli(Einheit::ALLE[e], *volk))
                    .sum();
                mal(
                    base,
                    r.volk(*volk).ladung * (1.0 + r.flug.logistik_bonus * *logistik as f64),
                )
            })
            .sum();
        // Group attacks have distinct plunder shares in live missions; do not invent one.
        let loot = if a.len() == 1 {
            bestand.map(|stock| {
                if result.sieger == Sieger::Angreifer {
                    beute(
                        r,
                        &stock,
                        &r.bunkerschutz(bunker),
                        r.volk(a[0].1)
                            .pluenderquote
                            .unwrap_or(r.kampf.pluenderquote),
                        (capacity - vorladung).max(0),
                    )
                } else {
                    [0; GUETER]
                }
            })
        } else {
            None
        };
        details.push(json!({"lauf":i,"sieger":result.sieger,"runden":result.runden,
            "verluste_angreifer":losses[0].to_vec(),"verluste_verteidiger":losses[1].to_vec(),
            "verlustwert_angreifer_milli":values[0],"verlustwert_verteidiger_milli":values[1],
            "restkapazitaet_milli":capacity,"beute_milli":loot,
            "bergung_annahme_promille":bergung,"bergung_rohmaterial_milli":debris.map(|n|anteil(mal(n,r.kampf.truemmer_anteil),bergung as i64,1000))}));
    }
    let n = runs as f64;
    let p = wins as f64 / n;
    let z = 1.959963984540054;
    let divisor = 1.0 + z * z / n;
    let center = (p + z * z / (2.0 * n)) / divisor;
    let margin = z * ((p * (1.0 - p) / n + z * z / (4.0 * n * n)).sqrt()) / divisor;
    let survivors = |v: [i64; EINHEITEN]| -> Value {
        Einheit::ALLE
            .iter()
            .filter(|e| v[e.idx()] > 0)
            .map(|e| (e.name().to_string(), json!(v[e.idx()] as f64 / n)))
            .collect::<serde_json::Map<_, _>>()
            .into()
    };
    let loot_mean: Vec<i64> = (0..GUETER)
        .map(|g| {
            details
                .iter()
                .map(|v| v["beute_milli"][g].as_i64().unwrap_or(0))
                .sum::<i64>()
                / runs as i64
        })
        .collect();
    Ok(
        json!({"beute_mittel_milli":if bestand.is_some()&&a.len()==1{Some(loot_mean)}else{None},"laeufe":runs,"szenarioseed":seed,"siegchance_prozent":p*100.0,"unentschieden_prozent":draws as f64*100.0/n,
        "siegchance_intervall_95_prozent":[100.0*(center-margin).max(0.0),100.0*(center+margin).min(1.0)],
        "eigene_verluste_wert":(la/runs as i128/M as i128),"verluste_gegner_wert":(ld/runs as i128/M as i128),
        "ueberlebende_angreifer":survivors(sa),"ueberlebende_verteidiger":survivors(sd),"einzellaeufe":details,
        "beute_status":if bestand.is_none(){"Bestand nicht eingegeben"}else if a.len()>1{"Verband: Beuteverteilung nicht prognostiziert"}else{"Restfracht je Lauf berücksichtigt"},
        // Kampfsimulator ohne Beute/Bergung: nur das, was eingetragen wurde (Funktionsprüfung 25.10.2026: Bergung-Satz ohne Bergungsfeld).
        "hinweis":if bestand.is_none(){"Zufallsläufe mit öffentlichem Startwert; keine Erfolgsgarantie. Gerechnet mit den eingetragenen Einheiten, Völkern und Technikstufen."}else{"Öffentlicher Szenariozufall; keine Erfolgsgarantie. Bergung ist eine Annahme ohne Gutschrift; Treibstoff und bereits beladene Fracht separat berücksichtigen."}}),
    )
}
