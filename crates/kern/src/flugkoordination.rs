//! Durable launch scheduling and independently priced return legs.
use crate::{typen::*,Welt};
use serde::{Serialize,Deserialize};
use serde_json::{Value,json};
use std::collections::BTreeMap;
#[derive(Clone,Debug,Serialize,Deserialize)]
pub struct Flugtermin{pub id:u64,pub spieler:SpielerId,pub rolle:Rolle,pub abflug:i64,pub ankunft:i64,pub ruecktempo:f64,pub flug:Value}
#[derive(Clone,Debug,Serialize,Deserialize)]
pub struct Rueckflugplan{pub dauer:i64,pub tempo:f64,pub treibstoff:i64}
impl Welt{
 fn koordinationsdaten(&self,sid:SpielerId,a:&Value,tempo:f64)->Result<(usize,crate::flotte::Flugplan,crate::flotte::Flugplan,bool),String>{
  // 0.4.8: Rückflugtempo 0 = „kein Rückflug“ (nur in Welten mit Orbitflotten): die Flotte bleibt nach dem Auftrag am Ziel.
  if tempo==0.0&&self.orbitflotten_aktiv(){let f=if matches!(a["typ"].as_str(),Some("boden_senden"|"flotte_beladen"|"personen_senden")){&a["flug"]}else{a};
   let k:Koord=f["start"].as_str().ok_or("Start fehlt")?.parse()?;let pid=self.eigener_planet(sid,k)?;let ziel:Koord=f["ziel"].as_str().ok_or("Ziel fehlt")?.parse()?;
   let ships:BTreeMap<Einheit,i64>=serde_json::from_value(f["schiffe"].clone()).map_err(|_|"Ungültige Schiffe")?;let mut s=[0;SCHIFFE];for(e,n)in ships{if !e.ist_schiff(){return Err("Keine Schiffsart".into());}s[e.idx()]=n;}
   let speed=f["geschwindigkeit"].as_f64().unwrap_or(1.0);if !speed.is_finite()||!(0.1..=1.0).contains(&speed){return Err("Hinflugtempo: 10 bis 100 %".into());}
   let hin=self.flugplan(sid,pid,ziel,&s,milli(speed))?;let mut retour=hin.clone();retour.treibstoff=0;return Ok((pid,hin,retour,true));}
  if !tempo.is_finite()||!(0.1..=1.0).contains(&tempo){return Err("Rückflugtempo: 10 bis 100 %".into());}
  let f=if matches!(a["typ"].as_str(),Some("boden_senden"|"flotte_beladen"|"personen_senden")){&a["flug"]}else{a};
  if f["typ"]!="flotte_senden"{return Err("Nur Flotten- oder Bodentransporte können terminiert werden".into());}
  let k:Koord=f["start"].as_str().ok_or("Start fehlt")?.parse()?;let pid=self.eigener_planet(sid,k)?;
  let ziel:Koord=f["ziel"].as_str().ok_or("Ziel fehlt")?.parse()?;
  let ships:BTreeMap<Einheit,i64>=serde_json::from_value(f["schiffe"].clone()).map_err(|_|"Ungültige Schiffe")?;
  let mut s=[0;SCHIFFE];for(e,n)in ships{if !e.ist_schiff(){return Err("Keine Schiffsart".into());}s[e.idx()]=n;}
  let speed=f["geschwindigkeit"].as_f64().unwrap_or(1.0);if !speed.is_finite()||!(0.1..=1.0).contains(&speed){return Err("Hinflugtempo: 10 bis 100 %".into());}
  let hin=self.flugplan(sid,pid,ziel,&s,milli(speed))?;let retour=self.flugplan(sid,pid,ziel,&s,milli(tempo))?;
  let one=matches!(f["mission"].as_str(),Some("stationieren"|"kolonisieren"));Ok((pid,hin,retour,one))
 }
 fn koordiniert_starten(&mut self,sid:SpielerId,rolle:Rolle,a:&Value,tempo:f64)->Result<String,String>{
  let(pid,hin,retour,one)=self.koordinationsdaten(sid,a,tempo)?;
  let kein_rueckflug=tempo==0.0&&self.orbitflotten_aktiv();
  let base0=if matches!(a["typ"].as_str(),Some("boden_senden"|"flotte_beladen"|"personen_senden")){&a["flug"]}else{a};
  let schon_einweg=matches!(base0["mission"].as_str(),Some("stationieren"|"kolonisieren"));
  let one=one&&!(kein_rueckflug&&!schon_einweg);
  let mut staged=self.clone();let id=staged.naechste_flotte;
  // The existing launch validator charges two identical legs. Adjust its reserve
  // on a private transaction, so failure cannot grant fuel or mutate the world.
  let delta=if one{0}else if kein_rueckflug{hin.treibstoff}else{hin.treibstoff-retour.treibstoff};
  let base=if matches!(a["typ"].as_str(),Some("boden_senden"|"flotte_beladen"|"personen_senden")){&a["flug"]}else{a};
  let moon=if base["startart"]=="mond"{staged.monde.monde.values().find(|m|m.besitzer==sid&&m.koord.to_string()==base["start"].as_str().unwrap_or("")).map(|m|m.id)}else{None};
  if let Some(mid)=moon{staged.monde.monde.get_mut(&mid).unwrap().bestand[Gut::Deuterium.idx()]+=delta;}else{staged.abrechnen(pid);staged.planeten[pid].bestand[Gut::Deuterium.idx()]+=delta;}
  let topf=Self::topf_fuer(rolle,Topf::Wirtschaft);if let Some(t)=topf{let mut c=[0;GUETER];c[Gut::Deuterium.idx()]=delta;staged.spieler[sid as usize].toepfe[t.idx()]+=staged.regeln.wert(&c);}
  let mut beladen=a.clone();let base=if matches!(beladen["typ"].as_str(),Some("boden_senden"|"flotte_beladen"|"personen_senden")){&mut beladen["flug"]}else{&mut beladen};base["rueckgeschwindigkeit"]=json!(if kein_rueckflug{1.0}else{tempo});
  let(ok,text)=staged.handeln(sid,rolle,&beladen);if !ok{return Err(text);}
  if kein_rueckflug&&!schon_einweg{staged.einweg_registrieren(id);}
  else if !one{staged.ausbau.rueckflugplaene.insert(id,Rueckflugplan{dauer:retour.dauer,tempo,treibstoff:retour.treibstoff});}
  *self=staged;Ok(text)
 }
 pub fn flotte_planen(&mut self,sid:SpielerId,rolle:Rolle,flug:Value,ankunft:Option<i64>,ruecktempo:f64)->Result<String,String>{
  if self.ausbau.industrie_version==0{return Err("Flottenkoordination benötigt die aktuelle Spielgeneration".into());}
  let(_,hin,_,_)=self.koordinationsdaten(sid,&flug,ruecktempo)?;
  let Some(arrival)=ankunft else{return self.koordiniert_starten(sid,rolle,&flug,ruecktempo);};
  let departure=arrival.checked_sub(hin.dauer).ok_or("Ankunft außerhalb des Zeitbereichs")?;
  if departure<self.zeit||departure>self.zeit+7*TAG{return Err("Ankunft zu früh oder Abflug mehr als sieben Spieltage entfernt".into());}
  if self.ausbau.flugtermine.values().filter(|t|t.spieler==sid).count()>=20{return Err("Höchstens 20 geplante Flüge".into());}
  let mut test=self.clone();test.koordiniert_starten(sid,rolle,&flug,ruecktempo)?;
  if departure==self.zeit{return self.koordiniert_starten(sid,rolle,&flug,ruecktempo);}
  self.seq+=1;let id=self.seq;self.ausbau.flugtermine.insert(id,Flugtermin{id,spieler:sid,rolle,abflug:departure,ankunft:arrival,ruecktempo,flug});
  Ok(format!("Flugplan #{id} gespeichert. Abflug {}, Ankunft {}. Schiffe und Vorräte werden beim Abflug erneut geprüft.",format!("{} (+{} s)",zeittext(departure),departure%60),format!("{} (+{} s)",zeittext(arrival),arrival%60)))
 }
 pub fn flugplan_storno(&mut self,sid:SpielerId,id:u64)->Result<String,String>{
  if !self.ausbau.flugtermine.get(&id).is_some_and(|a|a.spieler==sid){return Err("Kein eigener geplanter Flug".into());}
  self.ausbau.flugtermine.remove(&id);Ok("Geplanten Abflug entfernt; es wurden noch keine Rohstoffe verbraucht".into())
 }
 pub(crate) fn flugtermine_tick(&mut self){
  self.schiffhandel_tick();
  let ids=self.ausbau.flugtermine.iter().filter(|(_,a)|a.abflug<=self.zeit).map(|(id,_)|*id).collect::<Vec<_>>();
  for id in ids{let a=self.ausbau.flugtermine.remove(&id).unwrap();if !self.spieler_aktiv(a.spieler){continue;}
   // Recompute from the current technology: never arrive silently before the chosen time.
   let outcome=self.koordinationsdaten(a.spieler,&a.flug,a.ruecktempo).and_then(|(_,hin,_,_)|{
    let departure=a.ankunft-hin.dauer;if departure>self.zeit{let mut later=a.clone();later.abflug=departure;self.ausbau.flugtermine.insert(id,later);return Ok("Abflug nach neuer Antriebsforschung angepasst".into());}
    if departure<self.zeit{return Err("Geplante Ankunft mit aktueller Flotte nicht mehr erreichbar".into());}
    self.koordiniert_starten(a.spieler,a.rolle,&a.flug,a.ruecktempo)
   });self.vorfall(a.spieler,"flugplan",format!("Flugplan #{id}: {}",outcome.unwrap_or_else(|e|format!("NICHT GESTARTET: {e}"))));
  }
 }
 pub fn flugkoordination_quote(&self,sid:SpielerId,q:&Value)->Result<Value,String>{self.flugkoordination_quote_als(sid,Rolle::Alle,q)}
 /// Vorschau in der Rolle des Anfragenden (Harness/MCP), damit „bereit“ dieselbe Prüfung wie der echte Befehl ist.
 pub fn flugkoordination_quote_als(&self,sid:SpielerId,rolle:Rolle,q:&Value)->Result<Value,String>{
  let a=&q["flug"];let speed=q["rueckgeschwindigkeit"].as_f64().unwrap_or(1.0);let(_,hin,retour,one)=self.koordinationsdaten(sid,a,speed)?;
  let arrival=q["ankunft"].as_i64().unwrap_or(self.zeit+hin.dauer);let departure=arrival-hin.dauer;
  let f=if matches!(a["typ"].as_str(),Some("boden_senden"|"flotte_beladen"|"personen_senden")){&a["flug"]}else{a};let hold=f["haltedauer_stunden"].as_i64().unwrap_or(0)*STUNDE;
  let mut test=self.clone();let valid=test.koordiniert_starten(sid,rolle,a,speed);
  Ok(json!({"hinflug":hin.dauer,"rueckflug":if one{0}else{retour.dauer},"abflug":departure,"ankunft":arrival,"rueckkehr":if one{None}else{Some(arrival+hold+retour.dauer)},"treibstoff_hin":hin.treibstoff as f64/M as f64,"treibstoff_rueck":if one{0.0}else{retour.treibstoff as f64/M as f64},"zeit_gueltig":departure>=self.zeit&&departure<=self.zeit+7*TAG,"bereit":valid.is_ok(),"grund":valid.err(),"rueckkehr_annahme":"Ohne Kampf-, Besetzungs- oder Rettungsverzögerung"}))
 }
 pub(crate) fn rueckdauer(&self,id:FlottenId,basis:i64)->i64{self.ausbau.rueckflugplaene.get(&id).map(|p|p.dauer).unwrap_or(basis)}
}
