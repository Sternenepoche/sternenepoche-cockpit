//! Shared roster: UI, training, cargo and battle all use these values.
use crate::{typen::*,ausbau::Personen};
use serde::{Deserialize,Deserializer};
use std::collections::BTreeMap;
pub const ANZAHL:usize=12;
pub const KAMPFZAHL:usize=13;
// The first three slots retain their published identity for existing saves and orders.
pub const NAMEN:[&str;ANZAHL]=["infanterie","panzer","drohnen","infanterie_schwer","infanterie_pionier","infanterie_praezision","panzer_schwer","panzer_belagerung","panzer_flugabwehr","drohnen_aufklaerung","drohnen_brecher","drohnen_reparatur"];
pub const BILDER:[&str;ANZAHL]=["infanterie_sturm","panzer_sturm","drohnen_kampf","infanterie_schwer","infanterie_pionier","infanterie_praezision","panzer_schwer","panzer_belagerung","panzer_flugabwehr","drohnen_aufklaerung","drohnen_brecher","drohnen_reparatur"];
pub const LABELS:[&str;ANZAHL]=["Sturminfanterie","Sturmpanzer","Kampfdrohne","Schwere Infanterie","Belagerungspionier","Präzisionsinfanterie","Schwerer Kampfpanzer","Belagerungspanzer","Flugabwehrpanzer","Aufklärungsdrohne","Belagerungsdrohne","Reparaturdrohne"];
pub const PLAETZE:[i64;ANZAHL]=[1,10,2,2,2,1,15,12,10,1,3,2];
pub const FERTIGUNGSPLAETZE:[i64;ANZAHL]=[1,3,2,2,2,2,4,4,3,1,3,2];
pub const PERSONAL:[i64;ANZAHL]=[1,3,0,1,1,1,4,3,3,0,0,0];
pub const DAUER:[i64;ANZAHL]=[120,600,300,240,480,300,900,900,660,180,480,360];
pub const ANGRIFF:[i64;ANZAHL]=[10,80,30,18,24,22,100,140,55,4,45,0];
pub const STRUKTUR:[i64;ANZAHL]=[50,400,100,120,80,35,800,300,350,45,80,90];
pub const KASERNE:[u8;ANZAHL]=[1,3,2,2,3,3,5,6,4,2,5,4];
pub const EPOCHE:[u8;ANZAHL]=[3,4,3,3,4,4,4,5,4,3,4,4];
pub const ROLLEN:[&str;ANZAHL]=[
 "Günstige Basisinfanterie gegen leichte Drohnen.","Durchbruch gegen Infanterie und Miliz.","Mobiler Angriff auf Panzer.",
 "Robuste Infanterie gegen gegnerische Fußtruppen.","Panzerabwehr gegen schwere und Belagerungspanzer.","Hoher Schaden gegen Fußtruppen, geringe eigene Struktur.",
 "Schwer gepanzert; wirksam gegen Fußtruppen und Panzer, schwach gegen Drohnen.","Hoher Schaden gegen schwere Verteidiger am Boden; anfällig gegen Drohnen.",
 "Schützt Bodentruppen gegen Drohnen. Greift keine Raumflotten an.","Je vorhandener Drohne +10 % koordinierter Schaden der eigenen Bodenarmee, maximal +30 %.",
 "Spezialisiert auf schwere Panzer; wenig Schutz gegen Gegenfeuer.","Repariert je Runde bis zu 25 Struktur an beschädigten Panzern/Drohnen. Keine Wiederbelebung zerstörter Einheiten."];
pub fn kosten(art:usize)->[i64;GUETER]{let mut c=[0;GUETER];c[Gut::Legierung.idx()]=[20,200,50,50,70,45,400,350,240,20,100,80][art]*M;c[Gut::Elektronik.idx()]=[10,80,100,25,50,60,150,160,140,40,160,130][art]*M;c[Gut::Deuterium.idx()]=[0,20,10,0,5,0,60,50,30,5,20,10][art]*M;c}
pub fn faktor(a:usize,d:usize)->i64{
 match a{
  0 if [2,9,11].contains(&d)=>2000,
  1 if [0,3,5,12].contains(&d)=>2000,
  2 if [1,6,7,8].contains(&d)=>2000,
  3 if [0,4,5,12].contains(&d)=>2000,
  4 if [6,7].contains(&d)=>3000,
  4 if [1,8].contains(&d)=>2000,
  5 if [0,3,4,5,12].contains(&d)=>3000,
  5 if [1,6,7,8].contains(&d)=>250,
  5 if [2,9,10,11].contains(&d)=>500,
  6 if d==4=>500,
  6 if [0,3,5,12].contains(&d)=>2000,
  6 if [1,6,7,8].contains(&d)=>1500,
  6 if [2,9,10,11].contains(&d)=>500,
  7 if [3,4,6].contains(&d)=>2500,
  7 if [2,9,10,11].contains(&d)=>250,
  8 if [2,9,10,11].contains(&d)=>3000,
  8 if [1,6,7,8].contains(&d)=>500,
  8=>750,
  10 if [6,7].contains(&d)=>3000,
  10 if [0,3,4,5,12].contains(&d)=>500,
  _=>1000,
 }
}
pub fn erweitern<const N:usize>(values:&[i64])->Result<[i64;N],String>{
 if ![3,ANZAHL,N].contains(&values.len())||values.len()>N||values.iter().any(|x|*x<0){return Err("Ungültiger Bodentruppenbestand".into());}
 let mut out=[0;N];out[..values.len()].copy_from_slice(values);Ok(out)
}
pub fn truppen<'de,D:Deserializer<'de>>(d:D)->Result<[i64;ANZAHL],D::Error>{erweitern(&Vec::<i64>::deserialize(d)?).map_err(serde::de::Error::custom)}
pub fn army_map<'de,D:Deserializer<'de>>(d:D)->Result<BTreeMap<u32,[i64;ANZAHL]>,D::Error>{BTreeMap::<u32,Vec<i64>>::deserialize(d)?.into_iter().map(|(k,v)|Ok((k,erweitern(&v).map_err(serde::de::Error::custom)?))).collect()}
pub fn wounds_map<'de,D:Deserializer<'de>>(d:D)->Result<BTreeMap<u32,[[i64;KAMPFZAHL];2]>,D::Error>{BTreeMap::<u32,[Vec<i64>;2]>::deserialize(d)?.into_iter().map(|(k,v)|Ok((k,[erweitern(&v[0]).map_err(serde::de::Error::custom)?,erweitern(&v[1]).map_err(serde::de::Error::custom)?]))).collect()}
#[derive(Clone,Debug,serde::Serialize,Deserialize)]
pub struct Miliz{pub besitzer:SpielerId,pub personen:Personen}
pub fn katalog()->serde_json::Value{katalog_bw(&crate::kernwerte::standard().boden)}
/// Katalog mit den Kernwerten einer Welt (`Regelwerk::kw().boden`).
pub fn katalog_bw(b:&crate::kernwerte::BodenWerte)->serde_json::Value{serde_json::json!({"typen":(0..ANZAHL).map(|i|serde_json::json!({"id":NAMEN[i],"name":LABELS[i],"bild":BILDER[i],"rolle":ROLLEN[i],"angriff":b.angriff(i),"struktur":b.struktur(i),"kaserne":b.kaserne(i),"ab_stufe":b.ab_stufe(i),"fertigungsplaetze":b.fertigungsplaetze(i),"transportplaetze":b.plaetze(i),"personal":b.personal(i),"dauer_sekunden":b.dauer(i),"kosten_milli":b.kosten(i),"konter_promille":(0..KAMPFZAHL).map(|j|b.faktor(i,j)).collect::<Vec<_>>()})).collect::<Vec<_>>(),"rundenabstand_sekunden":900,"gleichzeitiges_feuer":true,"miliz":{"waffenwert":b.miliz_waffen_promille as f64/1000.0,"ruestwert":b.miliz_ruestung,"kapitulation_prozent":b.miliz_kapitulation_promille/10,"arbeitsausfall_prozent":b.miliz_dienst_promille/10,"legierung_je_bewohner":b.miliz_legierung_je_bewohner}})}
