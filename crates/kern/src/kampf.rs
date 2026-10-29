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
        let mut g = Self::mit_werten(
            r,
            sp.id,
            einheiten,
            t(Forschung::Waffentechnik) * volk.waffen,
            t(Forschung::Schildtechnik),
            t(Forschung::Panzerung) * volk.panzerung,
        );
        g.volk_einheiten(r, sp.volk, t(Forschung::Waffentechnik) * volk.waffen, t(Forschung::Schildtechnik), t(Forschung::Panzerung) * volk.panzerung);
        g
    }

    /// Eigene Werte einzelner Einheiten des Volks (`kernwerte.volk_einheiten`); ohne Eintrag bleibt alles unverändert.
    pub fn volk_einheiten(&mut self, r: &Regelwerk, volk: Volk, f_angriff: f64, f_schild: f64, f_struktur: f64) {
        if r.kw().volk_einheiten.is_empty() { return; }
        for e in Einheit::ALLE {
            let Some(v) = r.volk_einheit(volk, e) else { continue };
            let er = r.einh(e);
            self.angriff[e.idx()] = mal(er.angriff, f_angriff * v.angriff);
            self.schild[e.idx()] = mal(er.schild, f_schild * v.schild);
            self.struktur[e.idx()] = mal(er.struktur, f_struktur * v.struktur).max(1);
        }
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
    pub protokoll: Vec<serde_json::Value>,
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

/// Bounded, distinct-target combat for the expanded roster. Candidate pools are
/// indexed by type; a 20,000-unit simulator does not scan every defender per shot.
fn feuern_doktrin(r:&Regelwerk,tafel:&mut Zieltafel,schuetzen:&[Kaempfer],sg:&[Gruppe],ziele:&mut [Kaempfer],zg:&[Gruppe],rng:&mut ChaCha8Rng,mit_protokoll:bool)->Vec<serde_json::Value>{
 let mut shots:std::collections::BTreeMap<(u8,u8,u8,u8),[i64;6]>=std::collections::BTreeMap::new();
 let mut pool:Vec<Vec<usize>>=(0..EINHEITEN).map(|_|vec![]).collect();
 let mut positions=vec![usize::MAX;ziele.len()];
 for(i,z)in ziele.iter().enumerate(){if !z.tot {positions[i]=pool[z.typ as usize].len();pool[z.typ as usize].push(i);}}
 // Pools only shrink below, so target types absent now never become eligible; scanning only the present
 // types (in roster order) selects exactly as scanning the whole roster. Buffers are reused per shot.
 let vorhanden:Vec<usize>=(0..EINHEITEN).filter(|typ|!pool[*typ].is_empty()).collect();
 let springt=r.kw().kampf.schnellfeuer_springt;
 let (mut eligible,mut stufe,mut getroffen,mut excluded)=(Vec::<(usize,u8)>::with_capacity(EINHEITEN),Vec::<usize>::with_capacity(EINHEITEN),Vec::<usize>::new(),Vec::<usize>::new());
 for s in schuetzen {
  let e=Einheit::ALLE[s.typ as usize];let base=sg[s.gruppe as usize].angriff[e.idx()];if base<=0{continue;}
  let zeile=tafel.zeile(r,e);let cap=zeile.cap;
  getroffen.clear();
  for _ in 0..cap {
   // Only the highest available tier participates. Previously selected targets
   // are excluded before tier selection, so a second channel can use a secondary.
   eligible.clear();eligible.extend(vorhanden.iter().filter_map(|&typ|{
    let rank=zeile.rang[typ]?;
    pool[typ].iter().any(|id|!getroffen.contains(id)).then_some((typ,rank))
   }));
   let Some(mut best)=eligible.iter().map(|(_,rank)|*rank).min() else{break;};
   // Eigene Verteilung der Doktrin: Stufe nach Anteil würfeln (nur dann ein zusätzlicher Zufallszug).
   if let Some(g)=zeile.gewichte {
    let da=|k:u8|eligible.iter().any(|(_,rank)|*rank==k);
    let summe:u64=(0..3u8).filter(|k|da(*k)).map(|k|g[k as usize] as u64).sum();
    if summe>0 {let mut w=zufall(rng,summe);for k in 0..3u8 {if !da(k){continue;}let gk=g[k as usize] as u64;if w<gk{best=k;break;}w-=gk;}}
   }
   stufe.clear();stufe.extend(eligible.iter().filter(|(_,rank)|*rank==best).map(|(typ,_)|*typ));
   let total:usize=stufe.iter().map(|typ|pool[*typ].len()-getroffen.iter().filter(|id|ziele[**id].typ as usize==*typ && !ziele[**id].tot).count()).sum();
   if total==0{break;}
   let mut roll=zufall(rng,total as u64) as usize;let mut selected=None;
   for &typ in &stufe {
    let remaining=pool[typ].len()-getroffen.iter().filter(|id|ziele[**id].typ as usize==typ && !ziele[**id].tot).count();
    if roll>=remaining {roll-=remaining;continue;}
    // At most four excluded indices. Map an unbiased compact draw to its pool slot.
    excluded.clear();excluded.extend(getroffen.iter().filter(|id|ziele[**id].typ as usize==typ && !ziele[**id].tot).map(|id|positions[*id]));excluded.sort_unstable();
    let mut pos=roll;for &skip in &excluded{if skip<=pos{pos+=1;}}
    selected=Some((typ,pos,pool[typ][pos]));break;
   }
   let Some((typ,mut pos,mut id))=selected else{break;};getroffen.push(id);
   let damage=mal(base,zeile.faktor[typ]).max(0);
   let bevorzugt=zeile.rang[typ].is_some_and(|rank|rank<2);
   let follow=if bevorzugt{zeile.schnellfeuer[typ]}else{1};
   for _ in 0..64 {
    let z=&mut ziele[id];let old_shield=z.schild;let old_structure=z.struktur.max(0);let was_dead=z.tot;let g=&zg[z.gruppe as usize];let shield_damage=mal(damage,if bevorzugt{zeile.schildfaktor}else{1.0});
    if !z.tot && shield_damage as i128*FX>=g.schild[typ]as i128*fx(r.kampf.verpuffen_anteil){
     let absorbed=shield_damage.min(z.schild);z.schild-=absorbed;
     let used=if shield_damage>0 {anteil(damage,absorbed,shield_damage)}else{0};let rest=(damage-used).max(0);
     if rest>0{z.struktur-=rest;let full=g.struktur[typ];if z.struktur<=0||(z.struktur as i128*FX<full as i128*fx(r.kampf.explosion_unter)&&zufall(rng,full as u64)>=z.struktur as u64){z.tot=true;}}
    }
    if mit_protokoll && !was_dead {let row=shots.entry((s.gruppe,s.typ,z.gruppe,z.typ)).or_default();row[0]+=1;row[1]+=damage;row[2]+=old_shield-z.schild;row[3]+=old_structure-if z.tot{0}else{z.struktur.max(0)};row[4]+=i64::from(z.tot);row[5]+=i64::from(z.tot && z.struktur>0);}
    if z.tot {pool[typ].swap_remove(pos);positions[id]=usize::MAX;if pos<pool[typ].len(){positions[pool[typ][pos]]=pos;}
     // Mehrfachfeuer (Kernwert kampf.schnellfeuer_springt, neue Welten ab 24.10.2026): nach einem Abschuss springt das
     // Schnellfeuer auf ein zufälliges lebendes Ziel derselben Art. Sonst endet es wie bisher mit dem Abschuss.
     if !springt||follow<=1||pool[typ].is_empty()||zufall(rng,follow as u64)==0{break;}
     pos=zufall(rng,pool[typ].len() as u64) as usize;id=pool[typ][pos];continue;}
    if follow<=1||zufall(rng,follow as u64)==0{break;}
   }
  }
 }
 shots.into_iter().map(|((a,at,b,bt),n)|serde_json::json!({"schuetze":sg[a as usize].spieler,"schuetzengruppe":a,"einheit":Einheit::ALLE[at as usize].name(),"zielspieler":zg[b as usize].spieler,"zielgruppe":b,"ziel":Einheit::ALLE[bt as usize].name(),"schuesse":n[0],"schaden_angeboten":n[1],"schildschaden":n[2],"strukturschaden":n[3],"zerstoert":n[4],"explosionen":n[5]})).collect()
}

/// Zielwahl je Schützentyp, einmal je Kampf und nur für vorkommende Typen berechnet: dieselben Werte wie
/// `zielwahl::profil` (Rang, Ziele je Runde), `Regelwerk::trifft_faktor`, Schnellfeuer und Schildfaktor der
/// Flottendoktrin – vorher je Feuerphase (Profile) bzw. je Schuss (Faktoren) neu gesucht.
struct Zieltafel { zeilen: Vec<Option<Box<Zielzeile>>> }
struct Zielzeile { cap: u8, rang: [Option<u8>; EINHEITEN], faktor: [f64; EINHEITEN], schnellfeuer: [u32; EINHEITEN], schildfaktor: f64, gewichte: Option<[u32; 3]> }
impl Zieltafel {
 fn neu() -> Self { Zieltafel { zeilen: (0..EINHEITEN).map(|_| None).collect() } }
 fn zeile(&mut self, r: &Regelwerk, e: Einheit) -> &Zielzeile {
  self.zeilen[e.idx()].get_or_insert_with(|| {
   let profile = crate::zielwahl::profil(r, e);
   Box::new(Zielzeile {
    cap: profile.ziele_je_runde,
    // Anlagen gegen Anlagen kommen im Spiel nie vor (Anlagen stehen nur beim Verteidiger) und sind im Datenblatt „nicht
    // erfassbar“; in künstlichen Aufstellungen (Simulator, Bitgleich-Test) schießen sie wie bisher als Ausweichziel.
    rang: std::array::from_fn(|z| profile.rang(Einheit::ALLE[z]).or_else(|| (profile.ziele_je_runde > 0 && !e.ist_schiff() && !Einheit::ALLE[z].ist_schiff()).then_some(2))),
    faktor: std::array::from_fn(|z| r.trifft_faktor(e, Einheit::ALLE[z])),
    schnellfeuer: std::array::from_fn(|z| r.einh(e).schnellfeuer.get(&Einheit::ALLE[z]).copied().unwrap_or(1).clamp(1, 64)),
    schildfaktor: r.flottendoktrin.get(&e).map(|p| p.schildfaktor).unwrap_or(1.0),
    gewichte: profile.gewichte,
   })
  })
 }
}
fn bestand_sicht(k:&[Kaempfer],g:&[Gruppe])->Vec<serde_json::Value>{zaehlen(k,g.len()).iter().enumerate().map(|(i,n)|serde_json::json!({"gruppe":i,"spieler":g[i].spieler,"einheiten":Einheit::ALLE.iter().filter(|e|n[e.idx()]>0).map(|e|(e.name(),n[e.idx()])).collect::<std::collections::BTreeMap<_,_>>()})).collect()}

/// Höchstens sechs Runden. Beide Seiten feuern je Runde gleichzeitig: wer in der Runde
/// zerstört wird, schießt in ihr noch.
pub fn kampf(r: &Regelwerk, ang: &[Gruppe], vert: &[Gruppe], rng: &mut ChaCha8Rng) -> Kampfergebnis {
    kampf_mit(r, ang, vert, rng, true)
}

/// Wie [`kampf`] mit gleichem Zufallsstrom und gleichem Ausgang, aber ohne Rundenprotokoll (`protokoll` leer) –
/// für Schätzungen (Simulator, Bot-Angriffsplanung), die nur Sieger und Überlebende brauchen.
pub fn kampf_ohne_protokoll(r: &Regelwerk, ang: &[Gruppe], vert: &[Gruppe], rng: &mut ChaCha8Rng) -> Kampfergebnis {
    kampf_mit(r, ang, vert, rng, false)
}

fn kampf_mit(r: &Regelwerk, ang: &[Gruppe], vert: &[Gruppe], rng: &mut ChaCha8Rng, mit_protokoll: bool) -> Kampfergebnis {
    let mut a = aufstellen(ang);
    let mut v = aufstellen(vert);
    let mut runden = 0u8;
    let mut protokoll=Vec::new();
    let mut tafel = Zieltafel::neu();
    while runden < r.kampf.runden && !a.is_empty() && !v.is_empty() {
        runden += 1;
        for x in a.iter_mut() {
            x.schild = ang[x.gruppe as usize].schild[x.typ as usize];
        }
        for x in v.iter_mut() {
            x.schild = vert[x.gruppe as usize].schild[x.typ as usize];
        }
        let vorher=if mit_protokoll {serde_json::json!({"angreifer":bestand_sicht(&a,ang),"verteidiger":bestand_sicht(&v,vert)})} else {serde_json::Value::Null};
        let a_shots=feuern_doktrin(r,&mut tafel,&a,ang,&mut v,vert,rng,mit_protokoll);
        let v_shots=feuern_doktrin(r,&mut tafel,&v,vert,&mut a,ang,rng,mit_protokoll);
        a.retain(|x| !x.tot);
        v.retain(|x| !x.tot);
        if mit_protokoll {protokoll.push(serde_json::json!({"runde":runden,"vorher":vorher,"angreifer_feuer":a_shots,"verteidiger_feuer":v_shots,"nachher":{"angreifer":bestand_sicht(&a,ang),"verteidiger":bestand_sicht(&v,vert)}}));}
    }
    let sieger = if a.is_empty() {
        Sieger::Verteidiger
    } else if v.is_empty() {
        Sieger::Angreifer
    } else {
        Sieger::Unentschieden
    };
    Kampfergebnis { runden, protokoll, sieger, angreifer: zaehlen(&a, ang.len()), verteidiger: zaehlen(&v, vert.len()) }
}
