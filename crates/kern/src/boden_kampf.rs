//! The counter matrix shown in the catalog is also the actual firing calculation.
use crate::boden_katalog::*;
use crate::kernwerte::BodenWerte;
use serde_json::{json,Value};
/// Kampfwerte der Miliz (Kampfindex ANZAHL). Der Schaden steht in Promille je Person, damit ein
/// Milizionär auch einen Bruchteil eines Waffenwerts tragen kann.
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub struct MilizWerte{pub angriff_promille:i64,pub struktur:i64}
/// Bisherige Notmiliz (höchstens 25 % der Bevölkerung): Angriff 4, Struktur 20 je Person.
pub const KLASSISCH:MilizWerte=MilizWerte{angriff_promille:4000,struktur:20};
/// Invasion v1: alle Bewohner bewaffnet; Waffenwert 0,3 und Rüstwert 5 je Person
/// (Sturminfanterie: 10 und 50). Begründung und Kampfsimulation: docs/INVASION.md.
pub const AUFGEBOT:MilizWerte=MilizWerte{angriff_promille:crate::invasion::MILIZ_WAFFEN_PROMILLE,struktur:crate::invasion::MILIZ_RUESTUNG};
pub fn name(i:usize)->&'static str{if i==ANZAHL{"miliz"}else{NAMEN[i]}}
pub fn struktur(i:usize)->i64{struktur_mit(i,KLASSISCH)}
pub fn struktur_mit(i:usize,m:MilizWerte)->i64{if i==ANZAHL{m.struktur}else{STRUKTUR[i]}}
pub fn schaden(i:usize)->i64{if i==ANZAHL{4}else{ANGRIFF[i]}}
#[allow(dead_code)]
fn schaden_promille(i:usize,m:MilizWerte)->i64{if i==ANZAHL{m.angriff_promille}else{ANGRIFF[i]*1000}}
pub fn unterstuetzung(units:&[i64;KAMPFZAHL],wounds:&mut [i64;KAMPFZAHL])->Vec<Value>{unterstuetzung_bw(&crate::kernwerte::standard().boden,units,wounds)}
/// Reparaturdrohnen mit den Kernwerten der Welt (Standard: 25 Struktur je Drohne und Runde).
pub fn unterstuetzung_bw(b:&BodenWerte,units:&[i64;KAMPFZAHL],wounds:&mut [i64;KAMPFZAHL])->Vec<Value>{
 let mut repair=units[11]*b.reparatur_je_drohne;let mut log=vec![];
 for i in [6,1,8,7,2,10,11,9]{let heal=repair.min(wounds[i]);if units[i]>0&&heal>0{wounds[i]-=heal;repair-=heal;log.push(json!({"einheit":"drohnen_reparatur","ziel":name(i),"anzahl":units[11],"reparierte_struktur":heal}));}}
 log
}
pub fn feuern(attack:&[i64;KAMPFZAHL],defend:&[i64;KAMPFZAHL],wounds:[i64;KAMPFZAHL])->([i64;KAMPFZAHL],Vec<Value>,[i64;KAMPFZAHL]){feuern_mit(attack,defend,wounds,KLASSISCH)}
/// Gleichzeitiges Feuer einer Seite. Mit `KLASSISCH` bitgleich zur bisherigen Rechnung
/// (Angriff×1000 und die zusätzliche Division durch 1000 heben sich exakt auf),
/// mit `AUFGEBOT` für die Volksmiliz der Invasion v1.
pub fn feuern_mit(attack:&[i64;KAMPFZAHL],defend:&[i64;KAMPFZAHL],wounds:[i64;KAMPFZAHL],m:MilizWerte)->([i64;KAMPFZAHL],Vec<Value>,[i64;KAMPFZAHL]){feuern_bw(&crate::kernwerte::standard().boden,attack,defend,wounds,m)}
/// Struktur je Einheit mit den Kernwerten der Welt.
pub fn struktur_bw(b:&BodenWerte,i:usize,m:MilizWerte)->i64{if i==ANZAHL{m.struktur}else{b.struktur(i)}}
fn schaden_promille_bw(b:&BodenWerte,i:usize,m:MilizWerte)->i64{if i==ANZAHL{m.angriff_promille}else{b.angriff(i)*1000}}
/// Volksmiliz der Invasion v1 mit den Kernwerten der Welt.
pub fn aufgebot_bw(b:&BodenWerte)->MilizWerte{MilizWerte{angriff_promille:b.miliz_waffen_promille,struktur:b.miliz_ruestung}}
/// Gleichzeitiges Feuer mit den Kernwerten der Welt (Angriff, Struktur, Kontermatrix, Koordination).
pub fn feuern_bw(b:&BodenWerte,attack:&[i64;KAMPFZAHL],defend:&[i64;KAMPFZAHL],mut wounds:[i64;KAMPFZAHL],m:MilizWerte)->([i64;KAMPFZAHL],Vec<Value>,[i64;KAMPFZAHL]){
 let mut lost=[0;KAMPFZAHL];let mut fire=vec![];let coordination=1000+attack[9].min(b.koordination_max_drohnen)*b.koordination_je_drohne_promille;
 for i in 0..KAMPFZAHL{let sp=schaden_promille_bw(b,i,m);if attack[i]==0||sp==0{continue;}
  let target=(0..KAMPFZAHL).filter(|j|defend[*j]>0).max_by_key(|j|(b.faktor(i,*j),std::cmp::Reverse(*j)));
  if let Some(j)=target{let damage=attack[i]*sp*b.faktor(i,j)/1_000_000*coordination/1000;wounds[j]+=damage;
   let grund=if sp%1000==0{json!(sp/1000)}else{json!(sp as f64/1000.0)};
   fire.push(json!({"einheit":name(i),"ziel":name(j),"anzahl":attack[i],"grundangriff":grund,"konter_promille":b.faktor(i,j),"koordination_promille":coordination,"schaden":damage}));}
 }
 for i in 0..KAMPFZAHL{let s=struktur_bw(b,i,m);lost[i]=(wounds[i]/s).min(defend[i]);wounds[i]=if lost[i]>=defend[i]{0}else{wounds[i]%s};}
 (lost,fire,wounds)
}
