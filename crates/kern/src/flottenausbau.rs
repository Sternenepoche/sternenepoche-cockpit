//! Coordinated tactical roles and build gates, sourced from the reviewed fleet briefs.
use crate::{typen::*,regeln::{Regelwerk,EinheitRegel},Welt};
use serde::{Serialize,Deserialize};
use std::collections::BTreeMap;
#[derive(Clone,Debug,Serialize,Deserialize)]
pub struct FlottenDoktrin {
 pub klasse:String,pub rolle:String,pub ziele:u8,pub schadensfaktoren:BTreeMap<Einheit,f64>,
 pub voraussetzungen:BTreeMap<Forschung,u8>,pub schildfaktor:f64,
 #[serde(default)]pub kolonie_minen:u8,#[serde(default)]pub siedler:i64,
 /// Zusätzliche Personenplätze; unabhängig von Fracht, Volksbonus und Logistik.
 #[serde(default,skip_serializing_if="personenplaetze_leer")]pub personenplaetze:i64,
 /// Eigene Ziellisten (ConfigWelt, 21.10.2026). Fehlen sie, gelten die festen Listen in `zielwahl.rs`.
 #[serde(default,skip_serializing_if="Option::is_none")]pub primaer:Option<Vec<Einheit>>,
 #[serde(default,skip_serializing_if="Option::is_none")]pub sekundaer:Option<Vec<Einheit>>,
 /// Anteil der Zielkanäle je Stufe; fehlt er, strikte Rangfolge (Primär vor Sekundär vor Reserve).
 #[serde(default,skip_serializing_if="Option::is_none")]pub verteilung:Option<Zielverteilung>,
}
/// Gewichte der Zielstufen je Zielkanal (z. B. 70/20/10). Stufen ohne lebendes Ziel fallen heraus; sind alle
/// verbleibenden Gewichte 0, gilt die Rangfolge.
#[derive(Clone,Copy,Debug,Serialize,Deserialize,PartialEq,Eq)]
pub struct Zielverteilung {#[serde(default)]pub primaer:u32,#[serde(default)]pub sekundaer:u32,#[serde(default)]pub reserve:u32}
impl Zielverteilung { pub fn gewichte(&self)->[u32;3]{[self.primaer,self.sekundaer,self.reserve]} }
fn personenplaetze_leer(n:&i64)->bool{*n==0}
/// Inhalt von `regeln/flottenausbau.json`, soweit der Kern ihn liest (Einheitenwerte und Doktrin).
#[derive(Clone,Debug,Deserialize)]pub struct Ausbau {pub units:BTreeMap<Einheit,EinheitRegel>,pub profiles:BTreeMap<Einheit,FlottenDoktrin>}
/// Eingebauter Text von `regeln/flottenausbau.json`.
pub const FLOTTEN_JSON:&str=include_str!("../../../regeln/flottenausbau.json");
impl Ausbau {
 /// Liest eine Flottendatei und prüft, dass jedes Schiff Antrieb und Tempo hat (ConfigWelt).
 pub fn laden(text:&str)->Result<Ausbau,String>{
  let a:Ausbau=serde_json::from_str(text).map_err(|e|format!("Flottendatei nicht lesbar: {e}"))?;
  for (e,r) in &a.units {if e.ist_schiff()&&(r.antrieb.is_none()||r.tempo<=0){return Err(format!("Flottendatei: Schiff {e} braucht Antrieb und Tempo"));}
   if r.struktur<=0{return Err(format!("Flottendatei: {e} braucht Struktur > 0"));}
   if r.kosten.values().any(|n|*n<0){return Err(format!("Flottendatei: negative Kosten bei {e}"));}}
  Ok(a)
 }
}
/// Frozen doctrine-v1 roster (Generation 10/11). Later additions never join it implicitly.
fn in_v1(e:Einheit)->bool{!e.ist_mondinhalt()&&!matches!(e,Einheit::Pathfinder|Einheit::Bergungsschiff|Einheit::Terraformer)}
fn daten_v1()->&'static Ausbau {static DATA:std::sync::OnceLock<Ausbau>=std::sync::OnceLock::new();DATA.get_or_init(||serde_json::from_str(include_str!("../../../regeln/flottenausbau-v1.json")).expect("Archivierte Doktrin v1"))}
fn daten()->&'static Ausbau {static DATA:std::sync::OnceLock<Ausbau>=std::sync::OnceLock::new();DATA.get_or_init(||serde_json::from_str(FLOTTEN_JSON).expect("Geprüfte Flottensteckbriefe"))}
pub(crate) fn basis(e:Einheit)->&'static EinheitRegel{if matches!(e,Einheit::Pathfinder|Einheit::Bergungsschiff){crate::ausbau::schiffsregel(e)}else{&daten().units[&e]}}
impl Regelwerk {
 pub fn flottendoktrin_aktivieren(&mut self){
  if self.flottendoktrin.contains_key(&Einheit::Handelsfrachter){return;}
  if !self.flottendoktrin.is_empty(){
   // Upgrade only this new content. Preserve customized existing rule values.
   self.einheiten.insert(Einheit::Handelsfrachter,daten().units[&Einheit::Handelsfrachter].clone());
   self.flottendoktrin.insert(Einheit::Handelsfrachter,daten().profiles[&Einheit::Handelsfrachter].clone());
   if let Some(p)=self.flottendoktrin.get_mut(&Einheit::GrosserTransporter){p.personenplaetze=10;}
  }else{
  // Doctrine v1 is the frozen roster; later entries in flottenausbau.json arrive via their own activation.
  self.einheiten.extend(daten_v1().units.iter().filter(|(e,_)|in_v1(**e)).map(|(e,r)|(*e,r.clone())));
  self.flottendoktrin=daten_v1().profiles.iter().filter(|(e,_)|in_v1(**e)).map(|(e,p)|(*e,p.clone())).collect();
  }
  // The identity hashes the frozen v1 file, so extending flottenausbau.json keeps every rule hash.
  let mut h=sha2::Sha256::new();use sha2::Digest;h.update(self.hash.as_bytes());h.update(include_bytes!("../../../regeln/flottenausbau-v1.json"));self.hash=h.finalize().iter().map(|b|format!("{b:02x}")).collect();
 }
 /// Mondgeneration: Solarsonde, Weltenkrümmer und Schlachtkreuzer-Klasse aus flottenausbau.json.
 pub(crate) fn mondinhalt_aktivieren(&mut self){
  for e in Einheit::ALLE.into_iter().filter(|e|e.ist_mondinhalt()){
   self.einheiten.insert(e,daten().units[&e].clone());
   self.flottendoktrin.insert(e,daten().profiles[&e].clone());
  }
  // Konter laut Auftrag: Zerstörer mit Schnellfeuer 2 gegen Linien-Schlachtkreuzer.
  if let Some(z)=self.einheiten.get_mut(&Einheit::Zerstoerer){z.schnellfeuer.insert(Einheit::Schlachtkreuzer,2);}
 }
 /// Terraforming-Generation (Karl 24.10.2026): der Terraformer aus flottenausbau.json. Nur neue Welten mit Monden und
 /// Technik v1 (`Welt::technik_aktivieren`); seine Welten speichern in der Hülle STERNE14 (35 Schiffe).
 pub(crate) fn terraformer_aktivieren(&mut self){
  let e=Einheit::Terraformer;
  if !self.einheiten.contains_key(&e){self.einheiten.insert(e,daten().units[&e].clone());}
  if !self.flottendoktrin.contains_key(&e){self.flottendoktrin.insert(e,daten().profiles[&e].clone());}
 }
 pub fn personenplaetze(&self,e:Einheit)->i64{self.flottendoktrin.get(&e).map(|p|p.personenplaetze).unwrap_or(0)}
 /// Präferenzen geben Boni, aber keine Immunität für andere gegnerische Einheiten. Anlagen (Verteidigung, Schilde,
 /// Solarsonde) erfassen nur Schiffe: sie stehen immer auf der Verteidigerseite und treffen nie auf andere Anlagen
 /// (Karl 24.10.2026: „Lasergeschütze sollen Schiffe abwehren, keine anderen Anlagen“). Kampfneutral, nur Datenblatt.
 pub fn ziel_erfassbar(&self,schuetze:Einheit,ziel:Einheit)->bool{schuetze.ist_schiff()||ziel.ist_schiff()}
 pub fn trifft_faktor(&self,s:Einheit,z:Einheit)->f64{crate::zielwahl::schadensfaktor(self,s,z)}
}
impl Welt {
 pub fn natuerlicher_ring(&self,k:Koord)->bool{
  if self.regeln.flottendoktrin.is_empty()||k.position==0{return false;}
  let n=self.startwert.wrapping_add(u64::from(k.sektor)*7919+u64::from(k.system)*104729);
  n%5==0&&k.position==5+(n%4)as u8
 }
 pub fn bergbauziele(&self,sid:SpielerId)->Vec<serde_json::Value>{
  let mut out=vec![];for s in &self.systeme {let k=Koord::neu(s.sektor,s.nummer,1);if self.aufklaerungsregeln()&&!self.system_erfasst(sid,k){continue;}
   // Reichtum des Systems wie abbau_ertrag (Funktionsprüfung 25.10.2026: die Vorschau rechnete mit 1 und lag 27 % daneben).
   if s.guertel{out.push(serde_json::json!({"koord":Koord::neu(s.sektor,s.nummer,0).to_string(),"art":"Asteroidengürtel","reich_erz_promille":s.reich_erz,"reich_kristall_promille":s.reich_kristall}));}
   for p in 1..=self.regeln.welt.plaetze_je_system{let k=Koord::neu(s.sektor,s.nummer,p);if self.natuerlicher_ring(k){out.push(serde_json::json!({"koord":k.to_string(),"art":"Natürlicher Rohstoffring","reich_erz_promille":s.reich_erz,"reich_kristall_promille":s.reich_kristall}));}}
  }out
 }
 pub fn kolonieschiff_typ(schiffe:&[i64;SCHIFFE])->Option<Einheit>{[Einheit::GrossesKolonieschiff,Einheit::Kolonieschiff].into_iter().find(|e|schiffe[e.idx()]>0)}
 pub fn kolonieschiffe(schiffe:&[i64;SCHIFFE])->i64{schiffe[Einheit::Kolonieschiff.idx()]+schiffe[Einheit::GrossesKolonieschiff.idx()]}
 /// Add natural salvage exactly once during the explicit content upgrade, never on reload.
 pub fn flottendoktrin_aktivieren(&mut self){
  let schon_aktiv=!self.regeln.flottendoktrin.is_empty();
  std::sync::Arc::make_mut(&mut self.regeln).flottendoktrin_aktivieren();
  let schiffe=self.bericht_schiffe();
  for sp in &mut self.spieler {for report in &mut sp.berichte {if let Some(s)=report.schiffe.as_mut(){s.resize(schiffe,0);}if let Some(v)=report.verteidigung.as_mut(){v.resize(EINHEITEN-SCHIFFE,0);}}}
  if schon_aktiv{return;}
  for sys in &self.systeme {
   let n=self.startwert.wrapping_add(u64::from(sys.sektor)*7919+u64::from(sys.nummer)*104729);
   if n%4==0 {let k=Koord::neu(sys.sektor,sys.nummer,1+(n%u64::from(self.regeln.welt.plaetze_je_system))as u8);let t=self.truemmer.entry(k).or_insert([0;2]);t[0]+=(12000+(n%40000)as i64)*M;t[1]+=(6000+(n%20000)as i64)*M;}
  }
 }
 pub fn einheit_forschung_fehlend(&self,sid:SpielerId,e:Einheit)->Vec<String>{
  self.regeln.flottendoktrin.get(&e).map(|p|p.voraussetzungen.iter().filter(|(f,n)|self.spieler[sid as usize].forschung[f.idx()]<**n).map(|(f,n)|format!("{f} Stufe {n}")).collect()).unwrap_or_default()
 }
}
