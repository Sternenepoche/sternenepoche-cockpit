//! Bilaterale Schiffsangebote: Annahme vor dem Abflug, Credits bis zur Zustellung hinterlegt.
use crate::{ausbau::Personen,boden_katalog::ANZAHL,typen::*,Welt};
use serde::{Deserialize,Serialize};
use serde_json::{json,Value};

#[derive(Clone,Debug,Serialize,Deserialize)]
pub struct Schiffangebot {
 pub id:u64,pub von:SpielerId,pub an:SpielerId,pub erstellt:i64,pub preis:i64,
 pub flug:Value,pub personen:Personen,pub truppen:[i64;ANZAHL],
 pub status:String,pub flotte:Option<FlottenId>,pub zielplanet:PlanetId,pub zielmond:Option<crate::monde::MondId>,
 /// 0.4.8: Angebot einer gestrandeten Flotte (startet bei Annahme vom Orbitort mit Treibstoff aus der Ladung).
 #[serde(default,skip_serializing_if="Option::is_none")]
 pub orbitflotte:Option<FlottenId>,
}
/// Ganze Credits mit Tausenderpunkten für Meldungen („1.000.000 Credits“).
pub(crate) fn credits_text(ganz:i64)->String{let s=ganz.unsigned_abs().to_string();let mut o=String::new();
 for(i,c)in s.chars().enumerate(){if i>0&&(s.len()-i)%3==0{o.push('.');}o.push(c);}if ganz<0{format!("-{o}")}else{o}}
/// Preis im Meldungstext: 0 heißt Geschenk (Funktionsprüfung 25.10.2026: „0 Credits“).
fn preis_text(ganz:i64)->String{if ganz<=0{"Geschenk, kostenlos".into()}else{format!("{} Credits",credits_text(ganz))}}
fn ganz_preis(preis:f64)->i64{preis.round() as i64}
impl Welt {
 pub fn flotte_beladen(&mut self,sid:SpielerId,rolle:Rolle,flug:&Value,personen:Personen,truppen:[i64;ANZAHL])->Result<String,String>{
  if flug["typ"]!="flotte_senden" {return Err("Beladung benötigt einen Flottenauftrag".into());}
  let a=if truppen.iter().any(|n|*n!=0){json!({"typ":"boden_senden","flug":flug,"truppen":truppen})}else{flug.clone()};
  if personen.gruppen.iter().flatten().any(|n|*n!=0){self.personen_senden(sid,rolle,&a,personen)}else{
   let(ok,msg)=self.handeln(sid,rolle,&a);if ok{Ok(msg)}else{Err(msg)}
  }
 }
 pub fn schiffangebot(&mut self,sid:SpielerId,_rolle:Rolle,flug:&Value,preis:f64,personen:Personen,truppen:[i64;ANZAHL])->Result<String,String>{
  if !preis.is_finite()||!(0.0..=1_000_000_000.0).contains(&preis){return Err("Preis: 0 bis 1 Milliarde Credits; 0 bedeutet Geschenk".into());}
  if flug["typ"]!="flotte_senden"||flug["mission"]!="transport"{return Err("Schiffsübergabe benötigt einen Transportflug".into());}
  let ziel:Koord=flug["ziel"].as_str().ok_or("Ziel fehlt")?.parse()?;
  let pid=*self.belegung.get(&ziel).ok_or("Am Ziel liegt kein bewohnter Planet")?;
  let mid=if flug["zielart"]=="mond" {Some(self.monde.monde.values().find(|m|m.koord==ziel).ok_or("Zielmond fehlt")?.id)}else{None};
  let an=mid.map(|m|self.monde.monde[&m].besitzer).unwrap_or(self.planeten[pid as usize].besitzer);
  if an==sid{return Err("Eigene Schiffe mit Stationieren verlegen".into());}
  if !self.spieler_aktiv(an){return Err("Empfänger ist nicht mehr aktiv".into());}
  if self.ausbau.schiffangebote.values().filter(|a|a.von==sid&&matches!(a.status.as_str(),"offen"|"unterwegs")).count()>=20{return Err("Höchstens 20 offene Schiffsangebote".into());}
  let beladung=json!({"typ":"flotte_beladen","flug":flug,"personen":personen,"truppen":truppen});
  let mut probe=self.clone();probe.flotte_planen(sid,Rolle::Alle,beladung,None,flug["rueckgeschwindigkeit"].as_f64().unwrap_or(1.0))?;
  self.seq+=1;let id=self.seq;
  self.ausbau.schiffangebote.insert(id,Schiffangebot{id,von:sid,an,erstellt:self.zeit,preis:milli(preis),flug:flug.clone(),personen,truppen,status:"offen".into(),flotte:None,zielplanet:pid,zielmond:mid,orbitflotte:None});
  self.vorfall(an,"schiffhandel",format!("Schiffsangebot #{id} von {} nach {ziel}: {}. Im Flottenkommando annehmen oder ablehnen.",self.spieler[sid as usize].name,preis_text(ganz_preis(preis))));
  Ok(format!("Schiffsangebot #{id} gesendet. Schiffe bleiben bis zur Annahme am Startort verfügbar."))
 }
 pub fn schiffangebot_antwort(&mut self,sid:SpielerId,_rolle:Rolle,id:u64,annehmen:bool)->Result<String,String>{
  let a=self.ausbau.schiffangebote.get(&id).cloned().filter(|a|a.an==sid||a.von==sid).ok_or("Kein eigenes Schiffsangebot")?;
  if a.status!="offen"{return Err("Dieses Angebot ist nicht mehr offen".into());}
  if !annehmen {self.ausbau.schiffangebote.get_mut(&id).unwrap().status="abgelehnt".into();return Ok("Schiffsangebot beendet; keine Schiffe oder Credits übertragen".into());}
  if a.an!=sid{return Err("Nur der Empfänger kann das Angebot annehmen".into());}
  if self.zeit>a.erstellt+TAG{return Err("Angebot abgelaufen".into());}
  if !self.schiffangebot_ziel(&a){return Err("Empfänger am Ziel hat gewechselt; ein neues Angebot ist nötig".into());}
  if self.spieler[sid as usize].credits<a.preis{return Err("Nicht genügend Credits für dieses Schiffsangebot".into());}
  let mut next=self.clone();let mut fid=next.naechste_flotte;
  if let Some(of)=a.orbitflotte{
   // Gestrandete Flotte: startet jetzt vom Orbitort als Transport ohne Rückflug (Personen und Truppen sind an Bord).
   fid=of;next.flotte_starten(a.von,&a.flug)?;
  }else{
  next.flotte_planen(a.von,Rolle::Alle,json!({"typ":"flotte_beladen","flug":a.flug,"personen":a.personen,"truppen":a.truppen}),None,a.flug["rueckgeschwindigkeit"].as_f64().unwrap_or(1.0))?;
  }
  next.spieler[sid as usize].credits-=a.preis;
  let angebot=next.ausbau.schiffangebote.get_mut(&id).unwrap();angebot.status="unterwegs".into();angebot.flotte=Some(fid);
  next.vorfall(a.von,"schiffhandel",if a.preis>0{format!("Schiffsangebot #{id} angenommen: Flotte {fid} unterwegs. {} Credits bis zur Zustellung hinterlegt.",credits_text(ganz(a.preis)))}else{format!("Schiffsangebot #{id} angenommen: Flotte {fid} unterwegs (Geschenk).")});
  *self=next;Ok(format!("Schiffsangebot #{id} angenommen; Flotte {fid} gestartet"))
 }
 /// 0.4.8: Flottenübergabe einer gestrandeten Flotte an den Spieler, dem der Zielplanet gehört (auch nach einem Angriff).
 pub fn orbitflotte_anbieten(&mut self,sid:SpielerId,fid:FlottenId,ziel:Koord,preis:f64)->Result<String,String>{
  if !preis.is_finite()||!(0.0..=1_000_000_000.0).contains(&preis){return Err("Preis: 0 bis 1 Milliarde Credits; 0 bedeutet Geschenk".into());}
  if !self.gestrandet(fid)||!self.flotten.get(&fid).is_some_and(|f|f.besitzer==sid){return Err("Keine gestrandete eigene Flotte unter dieser Kennung".into());}
  let pid=*self.belegung.get(&ziel).ok_or("Am Ziel liegt kein bewohnter Planet")?;
  let an=self.planeten[pid as usize].besitzer;
  if an==sid{return Err("Eigene Planeten: mit Stationieren anfliegen".into());}
  if !self.spieler_aktiv(an){return Err("Empfänger ist nicht mehr aktiv".into());}
  if self.ausbau.schiffangebote.values().any(|a|a.orbitflotte==Some(fid)&&a.status=="offen"){return Err("Für diese Flotte liegt schon ein offenes Angebot vor".into());}
  if self.ausbau.schiffangebote.values().filter(|a|a.von==sid&&matches!(a.status.as_str(),"offen"|"unterwegs")).count()>=20{return Err("Höchstens 20 offene Schiffsangebote".into());}
  let flug=json!({"flotte":fid,"ziel":ziel.to_string(),"mission":"transport","geschwindigkeit":1.0,"rueckgeschwindigkeit":0.0,"uebergabe":true});
  let mut probe=self.clone();probe.flotte_starten(sid,&flug)?;
  self.seq+=1;let id=self.seq;
  self.ausbau.schiffangebote.insert(id,Schiffangebot{id,von:sid,an,erstellt:self.zeit,preis:milli(preis),flug,personen:Personen::default(),truppen:[0;ANZAHL],
   status:"offen".into(),flotte:None,zielplanet:pid,zielmond:None,orbitflotte:Some(fid)});
  self.vorfall(an,"schiffhandel",format!("Schiffsangebot #{id} von {}: gestrandete Flotte {fid} nach {ziel}, {}. Im Flottenkommando annehmen oder ablehnen.",self.spieler[sid as usize].name,preis_text(ganz_preis(preis))));
  Ok(format!("Schiffsangebot #{id} gesendet. Die Flotte bleibt bis zur Annahme an ihrem Orbitort."))
 }
 fn schiffangebot_ziel(&self,a:&Schiffangebot)->bool {
  if let Some(mid)=a.zielmond {self.monde.monde.get(&mid).is_some_and(|m|m.besitzer==a.an&&m.koord.to_string()==a.flug["ziel"].as_str().unwrap_or(""))}
  else{self.planeten.get(a.zielplanet as usize).is_some_and(|p|p.besitzer==a.an&&p.koord.to_string()==a.flug["ziel"].as_str().unwrap_or(""))}
 }
 fn schiffangebot_gescheitert(&mut self,id:u64,grund:&str){
  let a=self.ausbau.schiffangebote.get_mut(&id).unwrap();a.status="gescheitert".into();let(von,an,preis)=(a.von,a.an,a.preis);
  self.spieler[an as usize].credits+=preis;
  for sid in [von,an]{self.vorfall(sid,"schiffhandel",format!("Schiffsangebot #{id}: {grund}. {} hinterlegte Credits an Käufer zurückgegeben; Eigentum der Flotte bleibt beim Absender.",credits_text(ganz(preis))));}
 }
 pub(crate) fn schiffhandel_ankunft(&mut self,fid:FlottenId)->bool{
  let Some(a)=self.ausbau.schiffangebote.values().find(|a|a.status=="unterwegs"&&a.flotte==Some(fid)).cloned()else{return false;};
  let f=self.flotten[&fid].clone();
  if !self.schiffangebot_ziel(&a){self.schiffangebot_gescheitert(a.id,"Empfänger am Ziel hat gewechselt");self.heimflug(fid);return true;}
  let blockiert=if let Some(mid)=a.zielmond{self.mond_blockiert_fuer(mid,a.an)}else{self.blockiert_fuer(a.zielplanet as usize,a.an)};
  if blockiert {self.schiffangebot_gescheitert(a.id,"Ziel nicht mehr erreichbar");self.heimflug(fid);return true;}
  if let Some(mid)=a.zielmond{
   let m=self.monde.monde.get_mut(&mid).unwrap();for e in 0..SCHIFFE{m.einheiten[e]+=f.schiffe[e];}for g in 0..GUETER{m.bestand[g]+=f.ladung[g];}
  }else{
   let pid=a.zielplanet as usize;self.abrechnen(pid);self.personen_landen(fid,pid);
   for e in 0..SCHIFFE{self.planeten[pid].einheiten[e]+=f.schiffe[e];}for g in 0..GUETER{self.planeten[pid].bestand[g]+=f.ladung[g];}self.raten_neu(pid);
  }
  self.spieler[a.von as usize].credits+=a.preis;
  self.ausbau.schiffangebote.get_mut(&a.id).unwrap().status="zugestellt".into();
  self.flotten.remove(&fid);self.monde.flottenstart.remove(&fid);self.monde.flottenziel.remove(&fid);self.kolonisation.transportziele.remove(&fid);self.ausbau.rueckflugplaene.remove(&fid);
  for sid in [a.von,a.an]{self.vorfall(sid,"schiffhandel",format!("Schiffsangebot #{} auf {} zugestellt: Schiffe und gesamte Zuladung gehören jetzt {}. {}",a.id,f.ziel,self.spieler[a.an as usize].name,if a.preis>0{format!("{} Credits ausgezahlt.",credits_text(ganz(a.preis)))}else{"Geschenk.".to_string()}));}
  true
 }
 pub(crate) fn schiffhandel_tick(&mut self){
  let fehl=self.ausbau.schiffangebote.values().filter(|a|a.status=="unterwegs"&&a.flotte.is_none_or(|id|self.flotten.get(&id).is_none_or(|f|f.zustand==crate::welt::Flottenzustand::Rueckflug))).map(|a|a.id).collect::<Vec<_>>();
  for id in fehl{self.schiffangebot_gescheitert(id,"Flotte zurückgerufen oder verloren");}
  for a in self.ausbau.schiffangebote.values_mut(){if a.status=="offen"&&self.zeit>a.erstellt+TAG{a.status="abgelaufen".into();}}
  let alt=self.ausbau.schiffangebote.values().filter(|a|!matches!(a.status.as_str(),"offen"|"unterwegs")).map(|a|a.id).collect::<Vec<_>>();let n=alt.len().saturating_sub(100);for id in alt.into_iter().take(n){self.ausbau.schiffangebote.remove(&id);}
 }
 pub(crate) fn schiffangebote_sicht(&self,sid:SpielerId)->Value{
  // Angebot einer gestrandeten Flotte: Schiffe, Ladung und Truppen aus der Flotte selbst (das Angebot speichert nur den Flug) –
  // Einsatzprobe 25.10.2026: der Empfänger kaufte sonst für 1.000.000 Credits „– · Fracht: –“.
  let flug=|a:&Schiffangebot|->Value{let mut f=a.flug.clone();
   if let Some(fl)=a.orbitflotte.and_then(|id|self.flotten.get(&id)){
    f["schiffe"]=Value::Object(Einheit::ALLE[..SCHIFFE].iter().filter(|e|fl.schiffe[e.idx()]>0).map(|e|(e.name().to_string(),json!(fl.schiffe[e.idx()]))).collect());
    f["ladung"]=Value::Object(Gut::ALLE.iter().filter(|g|fl.ladung[g.idx()]>=M).map(|g|(g.name().to_string(),json!(fl.ladung[g.idx()]/M))).collect());
   }f};
  let truppen=|a:&Schiffangebot|a.orbitflotte.and_then(|id|self.ausbau.boden.flotten.get(&id).copied()).filter(|_|a.truppen.iter().all(|n|*n==0)).unwrap_or(a.truppen);
  json!(self.ausbau.schiffangebote.values().filter(|a|a.von==sid||a.an==sid).rev().map(|a|json!({"id":a.id,"von":self.spieler[a.von as usize].name,"an":self.spieler[a.an as usize].name,"eingang":a.an==sid,"preis":a.preis as f64/M as f64,"status":a.status,"flug":flug(a),"personen":a.personen,"truppen":truppen(a),"flotte":a.flotte,"ablauf":a.erstellt+TAG})).collect::<Vec<_>>())
 }
}
