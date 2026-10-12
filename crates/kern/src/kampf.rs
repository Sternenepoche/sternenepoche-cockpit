//! Kampf als Einzelsimulation je Einheit mit festem Zufallsstrom.

use crate::regeln::Regelwerk;
use crate::typen::*;
use crate::welt::{zufall, Sieger, Spieler};
use rand_chacha::ChaCha8Rng;

/// Eine Kampfpartei mit den Werten ihres Besitzers.
#[derive(Clone, Debug)]
pub struct Gruppe {
    pub spieler: SpielerId,
    pub einheiten: [i64; EINHEITEN],
    pub angriff: [i64; EINHEITEN],
    pub schild: [i64; EINHEITEN],
    pub struktur: [i64; EINHEITEN],
}

impl Gruppe {
    pub fn neu(r: &Regelwerk, sp: &Spieler, einheiten: [i64; EINHEITEN]) -> Gruppe {
        let volk = r.volk(sp.volk);
        let t = |f: Forschung| 1.0 + r.kampf.tech_je_stufe * sp.forschung[f.idx()] as f64;
        Self::mit_werten(
            r,
            sp.id,
            einheiten,
            t(Forschung::Waffentechnik) * volk.waffen,
            t(Forschung::Schildtechnik),
            t(Forschung::Panzerung) * volk.panzerung,
        )
    }

    pub fn mit_werten(
        r: &Regelwerk,
        spieler: SpielerId,
        einheiten: [i64; EINHEITEN],
        f_angriff: f64,
        f_schild: f64,
        f_struktur: f64,
    ) -> Gruppe {
        let mut g = Gruppe { spieler, einheiten, angriff: [0; EINHEITEN], schild: [0; EINHEITEN], struktur: [0; EINHEITEN] };
        for e in Einheit::ALLE {
            let er = r.einh(e);
            g.angriff[e.idx()] = mal(er.angriff, f_angriff);
            g.schild[e.idx()] = mal(er.schild, f_schild);
            g.struktur[e.idx()] = mal(er.struktur, f_struktur).max(1);
        }
        g
    }
}

struct Kaempfer {
    typ: u8,
    gruppe: u8,
    struktur: i64,
    schild: i64,
    tot: bool,
}

#[derive(Clone, Debug)]
pub struct Kampfergebnis {
    pub runden: u8,
    pub sieger: Sieger,
    /// Überlebende je Gruppe.
    pub angreifer: Vec<[i64; EINHEITEN]>,
    pub verteidiger: Vec<[i64; EINHEITEN]>,
}

fn aufstellen(gruppen: &[Gruppe]) -> Vec<Kaempfer> {
    let mut v = Vec::new();
    for (gi, g) in gruppen.iter().enumerate() {
        for e in 0..EINHEITEN {
            for _ in 0..g.einheiten[e].max(0) {
                v.push(Kaempfer { typ: e as u8, gruppe: gi as u8, struktur: g.struktur[e], schild: g.schild[e], tot: false });
            }
        }
    }
    v
}

fn zaehlen(k: &[Kaempfer], gruppen: usize) -> Vec<[i64; EINHEITEN]> {
    let mut v = vec![[0i64; EINHEITEN]; gruppen];
    for x in k {
        v[x.gruppe as usize][x.typ as usize] += 1;
    }
    v
}

#[allow(clippy::too_many_arguments)]
fn feuern(
    schuetzen: &[Kaempfer],
    sg: &[Gruppe],
    ziele: &mut [Kaempfer],
    zg: &[Gruppe],
    schnellfeuer: &[[u32; EINHEITEN]; EINHEITEN],
    verpuffen: i128,
    explosion: i128,
    rng: &mut ChaCha8Rng,
) {
    if ziele.is_empty() {
        return;
    }
    let n = ziele.len() as u64;
    for s in schuetzen {
        let schaden = sg[s.gruppe as usize].angriff[s.typ as usize];
        if schaden <= 0 {
            continue;
        }
        loop {
            let z = &mut ziele[zufall(rng, n) as usize];
            let zgr = &zg[z.gruppe as usize];
            let schild_max = zgr.schild[z.typ as usize];
            // Ein Schuss unter einem Prozent des Schildwerts verpufft.
            if !z.tot && schaden as i128 * FX >= schild_max as i128 * verpuffen {
                let ab = schaden.min(z.schild);
                z.schild -= ab;
                let rest = schaden - ab;
                if rest > 0 {
                    z.struktur -= rest;
                    let voll = zgr.struktur[z.typ as usize];
                    if z.struktur <= 0 {
                        z.tot = true;
                    } else if (z.struktur as i128) * FX < voll as i128 * explosion {
                        // Explosion mit Wahrscheinlichkeit 1 minus Rest geteilt durch Ausgangsstruktur.
                        if zufall(rng, voll as u64) >= z.struktur as u64 {
                            z.tot = true;
                        }
                    }
                }
            }
            let f = schnellfeuer[s.typ as usize][z.typ as usize];
            if f > 1 && zufall(rng, f as u64) != 0 {
                continue;
            }
            break;
        }
    }
}

/// Höchstens sechs Runden. Beide Seiten feuern je Runde gleichzeitig: wer in der Runde
/// zerstört wird, schießt in ihr noch.
pub fn kampf(r: &Regelwerk, ang: &[Gruppe], vert: &[Gruppe], rng: &mut ChaCha8Rng) -> Kampfergebnis {
    let mut sf = [[0u32; EINHEITEN]; EINHEITEN];
    for e in Einheit::ALLE {
        for (ziel, f) in &r.einh(e).schnellfeuer {
            sf[e.idx()][ziel.idx()] = *f;
        }
    }
    let verpuffen = fx(r.kampf.verpuffen_anteil);
    let explosion = fx(r.kampf.explosion_unter);
    let mut a = aufstellen(ang);
    let mut v = aufstellen(vert);
    let mut runden = 0u8;
    while runden < r.kampf.runden && !a.is_empty() && !v.is_empty() {
        runden += 1;
        for x in a.iter_mut() {
            x.schild = ang[x.gruppe as usize].schild[x.typ as usize];
        }
        for x in v.iter_mut() {
            x.schild = vert[x.gruppe as usize].schild[x.typ as usize];
        }
        feuern(&a, ang, &mut v, vert, &sf, verpuffen, explosion, rng);
        feuern(&v, vert, &mut a, ang, &sf, verpuffen, explosion, rng);
        a.retain(|x| !x.tot);
        v.retain(|x| !x.tot);
    }
    let sieger = if a.is_empty() {
        Sieger::Verteidiger
    } else if v.is_empty() {
        Sieger::Angreifer
    } else {
        Sieger::Unentschieden
    };
    Kampfergebnis { runden, sieger, angreifer: zaehlen(&a, ang.len()), verteidiger: zaehlen(&v, vert.len()) }
}
