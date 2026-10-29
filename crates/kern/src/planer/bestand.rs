use crate::typen::*;
use serde::Serialize;

#[derive(Clone, Debug, Serialize)]
pub struct Bestandsrechnung {
    pub bestand: [i64; GUETER],
    pub lagerverlust: [i64; GUETER],
}

/// The single stock integrator for mutable accounting and read-only observation.
pub fn fortschreiben(
    bestand: &[i64; GUETER],
    rate: &[i64; GUETER],
    grenze: &[i64; GUETER],
    dt: i64,
) -> Bestandsrechnung {
    let mut out = Bestandsrechnung {
        bestand: *bestand,
        lagerverlust: [0; GUETER],
    };
    if dt <= 0 {
        return out;
    }
    for g in 0..GUETER {
        let delta = (rate[g] as i128 * dt as i128 / STUNDE as i128) as i64;
        if delta >= 0 {
            if bestand[g] < grenze[g] {
                let neu = bestand[g] + delta;
                if neu > grenze[g] {
                    out.lagerverlust[g] = neu - grenze[g];
                    out.bestand[g] = grenze[g];
                } else {
                    out.bestand[g] = neu;
                }
            } else {
                out.lagerverlust[g] = delta;
            }
        } else {
            out.bestand[g] = (bestand[g] + delta).max(0);
        }
    }
    out
}

/// Preserve sub-milli amounts across frequent accounting, including save/reload.
/// At empty/full storage discard fractions flowing out of the valid range.
pub fn fortschreiben_mit_rest(bestand:&[i64;GUETER],rate:&[i64;GUETER],grenze:&[i64;GUETER],dt:i64,rest:&[i64;GUETER])->(Bestandsrechnung,[i64;GUETER]) {
    if dt<=0{return (fortschreiben(bestand,rate,grenze,0),*rest);}
    let mut result=Bestandsrechnung{bestand:*bestand,lagerverlust:[0;GUETER]};let mut next=[0;GUETER];
    for g in 0..GUETER {
        let numerator=rate[g] as i128*dt as i128+rest[g] as i128;
        let delta=(numerator/STUNDE as i128) as i64;
        next[g]=(numerator%STUNDE as i128) as i64;
        if numerator>=0 {
            if bestand[g]>=grenze[g]{result.lagerverlust[g]=delta;next[g]=0;}
            else {let stock=bestand[g]+delta;result.bestand[g]=stock.min(grenze[g]);if stock>=grenze[g]{result.lagerverlust[g]=(stock-grenze[g]).max(0);next[g]=0;}}
        }else {let stock=bestand[g]+delta;result.bestand[g]=stock.max(0);if stock<=0{next[g]=0;}}
    }
    (result,next)
}
