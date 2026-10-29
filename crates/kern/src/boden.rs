//! Persisted surface units; production, embarkation and occupation use real counts.
use crate::{typen::*,Welt};
use serde::{Serialize,Deserialize};
use serde_json::{json,Value};
use std::collections::BTreeMap;
pub use crate::boden_katalog::*;
#[derive(Clone,Debug,Default,Serialize,Deserialize)]
pub struct Boden {
 #[serde(default,deserialize_with="army_map")] pub bestand:BTreeMap<PlanetId,[i64;ANZAHL]>,
 #[serde(default,deserialize_with="army_map")] pub flotten:BTreeMap<FlottenId,[i64;ANZAHL]>,
 #[serde(default)] pub miliz:BTreeMap<PlanetId,Miliz>,
 pub auftraege:BTreeMap<PlanetId,Vec<BodenAuftrag>>,pub letzte_runde:BTreeMap<FlottenId,SimZeit>,pub berichte:Vec<Value>,
 #[serde(default,deserialize_with="wounds_map")] pub wunden:BTreeMap<FlottenId,[[i64;KAMPFZAHL];2]>,
 /// Invasion v1 (neue Welten); leer bleibt das bisherige JSON bytegleich.
 #[serde(default,skip_serializing_if="crate::invasion::Invasion::leer")] pub invasion:crate::invasion::Invasion,
}
impl Boden{pub fn leer(&self)->bool{self.miliz.is_empty()&&self.bestand.is_empty()&&self.flotten.is_empty()&&self.auftraege.is_empty()&&self.berichte.is_empty()&&self.invasion.leer()}}
#[derive(Clone,Debug,Serialize,Deserialize)]
pub struct BodenLos{pub anzahl:i64,pub arbeit:i64,pub gesamt:i64}
#[derive(Clone,Debug,Serialize,Deserialize)]
pub struct BodenAuftrag{#[serde(default)] pub lose:Vec<BodenLos>,pub id:u64,pub art:usize,pub rest:i64,pub arbeit:i64,pub gesamt:i64,pub bezahlt:[i64;GUETER],#[serde(default)] pub topf:Option<Topf>,pub rekruten:crate::ausbau::Personen,pub besitzer:SpielerId}
impl Welt{
 pub fn boden_bauen(&mut self,sid:SpielerId,k:Koord,art:&str,anzahl:i64)->Result<String,String>{
  self.boden_bauen_rolle(sid,Rolle::Alle,k,art,anzahl)
 }
 pub fn boden_bauen_rolle(&mut self,sid:SpielerId,rolle:Rolle,k:Koord,art:&str,anzahl:i64)->Result<String,String>{
  let pid=self.eigener_planet(sid,k)?;let t=NAMEN.iter().position(|n|*n==art).ok_or("Unbekannte Bodeneinheit")?;
  if self.ausbau.industrie_version==0{return Err("Bodeneinheiten benötigen die aktuelle Spielgeneration".into());}
  if !(1..=1000).contains(&anzahl){return Err("1 bis 1000 Einheiten wählen".into());}
  let rg=self.regeln.clone();let bw=&rg.kw().boden;
  let level=self.planeten[pid].gebaeude[Gebaeude::Kaserne.idx()];if level<bw.kaserne(t)||self.spieler[sid as usize].stufe<bw.ab_stufe(t){return Err(format!("{} benötigt Kaserne {} und Zivilisationsstufe {}",LABELS[t],bw.kaserne(t),bw.ab_stufe(t)));}
  if self.ausbau.boden.auftraege.get(&(pid as u32)).is_some_and(|q|q.len()>=8){return Err("Kasernenschlange ist voll (8 Aufträge)".into());}
  let recruits=bw.personal(t)*anzahl*M;let people=&self.ausbau.personen[&(pid as u32)];if people.gesamt()-recruits<500*M{return Err("Mindestens 500 Einwohner müssen auf dem Planeten bleiben".into());}
  let mut remaining=people.clone();let chosen=people.auswahl(recruits);remaining.nehmen(&chosen)?;
  let cost=self.produktionskosten(pid,Gebaeude::Kaserne,bw.kosten(t));let total=cost.map(|n|n*anzahl);let topf=Self::topf_fuer(rolle,Topf::Militaer);self.abrechnen(pid);self.zahlbar(pid,&total,topf)?;self.zahlen(pid,&total,topf);
  self.ausbau.personen.insert(pid as u32,remaining);self.planeten[pid].bevoelkerung-=recruits;
  let work=bw.dauer(t)*100;
  self.seq+=1;
  self.ausbau.boden.auftraege.entry(pid as u32).or_default().push(BodenAuftrag{lose:vec![],id:self.seq,art:t,rest:anzahl,arbeit:work,gesamt:work,bezahlt:cost,topf,rekruten:chosen,besitzer:sid});self.boden_verteilen(pid);self.raten_neu(pid);
  Ok(format!("{anzahl} {art} in Ausbildung; Material und Personal reserviert"))
 }
 pub fn boden_storno(&mut self,sid:SpielerId,k:Koord,index:usize)->Result<String,String>{
  let pid=self.eigener_planet(sid,k)?;let q=self.ausbau.boden.auftraege.get_mut(&(pid as u32)).ok_or("Keine Aufträge")?;if index>=q.len(){return Err("Auftrag fehlt".into());}
  let a=q.remove(index);let charged=a.lose.iter().filter(|b|b.arbeit*4<=b.gesamt).map(|b|b.anzahl).sum::<i64>();let refund=a.rest-charged;self.erstattung(pid,a.bezahlt.map(|c|c*refund),a.topf);
  self.produktionsserie_stoppen(pid,"boden",NAMEN[a.art]);
  self.planeten[pid].bevoelkerung+=a.rekruten.gesamt();self.ausbau.personen.entry(pid as u32).or_default().add(&a.rekruten);self.boden_verteilen(pid);self.raten_neu(pid);Ok("Kasernenauftrag abgebrochen; unbegonnene Stücke und laufendes Stück vor 75 % erstattet, ungenutzte Rekruten zurück".into())
 }
 pub(crate) fn boden_verteilen(&mut self,pid:usize){
  let rg=self.regeln.clone();let bw=&rg.kw().boden;
  let slots=self.planeten[pid].gebaeude[Gebaeude::Kaserne.idx()] as i64;let Some(q)=self.ausbau.boden.auftraege.get_mut(&(pid as u32))else{return;};
  let mut free=slots-q.iter().map(|a|a.lose.iter().map(|b|b.anzahl*bw.fertigungsplaetze(a.art)).sum::<i64>()).sum::<i64>();
  for a in q{let active=a.lose.iter().map(|b|b.anzahl).sum::<i64>();let count=(a.rest-active).min(free.max(0)/bw.fertigungsplaetze(a.art));if count>0{a.lose.push(BodenLos{anzahl:count,arbeit:a.gesamt,gesamt:a.gesamt});free-=count*bw.fertigungsplaetze(a.art);}a.arbeit=a.lose.iter().map(|b|b.arbeit).min().unwrap_or(a.gesamt);}
 }
 pub(crate) fn boden_vorruecken(&mut self,dt:i64){
  if self.ausbau.industrie_version==0||dt<=0{return;}
  let rg=self.regeln.clone();let bw=&rg.kw().boden;
  let ids=self.ausbau.boden.auftraege.keys().copied().collect::<Vec<_>>();
  for id in ids{let pid=id as usize;if !self.planet_aktiv(pid)||self.werk_ruht(pid,Gebaeude::Kaserne){continue;}
   // Invasion v2 (Karl 24.10.2026): während einer Invasion bildet die Kaserne langsamer aus (Standard 50 %).
   let work=dt*self.produktionsstufen(pid)[Gebaeude::Kaserne.idx()] as i64;let work=if self.invasion_laeuft(pid){work*bw.invasion_ausbildung_promille/1000}else{work};let q=self.ausbau.boden.auftraege.get_mut(&id).unwrap();
   for a in q.iter_mut(){if a.besitzer!=self.planeten[pid].besitzer{a.rest=0;continue;}
    for b in &mut a.lose{b.arbeit-=work;if b.arbeit<=0{self.ausbau.boden.bestand.entry(id).or_default()[a.art]+=b.anzahl;let crew=a.rekruten.auswahl(bw.personal(a.art)*b.anzahl*M);a.rekruten.nehmen(&crew).expect("reserved recruits");a.rest-=b.anzahl;}}
    a.lose.retain(|b|b.arbeit>0);
   }q.retain(|a|a.rest>0);self.boden_verteilen(pid);
  }
 }
 pub(crate) fn boden_naechste_fertigstellung(&self)->Option<SimZeit>{
  self.ausbau.boden.auftraege.iter().filter_map(|(id,q)|{let pid=*id as usize;if !self.planet_aktiv(pid)||self.werk_ruht(pid,Gebaeude::Kaserne){return None;}let speed=self.produktionsstufen(pid)[Gebaeude::Kaserne.idx()] as i64;
   q.iter().flat_map(|a|a.lose.iter()).map(|b|self.zeit+(b.arbeit+speed-1)/speed).min()
  }).min()
 }
 pub fn boden_senden(&mut self,sid:SpielerId,rolle:Rolle,flug:&Value,truppen:[i64;ANZAHL])->Result<String,String>{
  if truppen.iter().any(|n|!(0..=100_000).contains(n))||truppen.iter().sum::<i64>()==0{return Err("Ganze verfügbare Bodeneinheiten wählen".into());}
  if flug["typ"]!="flotte_senden"{return Err("Truppen benötigen einen Flottenauftrag".into());}
  if flug["startart"]=="mond"||flug["zielart"]=="mond"{return Err("Bodentruppen starten und landen auf Planeten".into());}
  let k:Koord=flug["start"].as_str().ok_or("Startplanet fehlt")?.parse()?;let pid=self.eigener_planet(sid,k)?;
  let mut flug=flug.clone();
  // Invasion v1: „verstaerken“ = Bodentruppen zu einem verbündeten Planeten (Mission halten); sie
  // verteidigen dort mit, bleiben Eigentum des Absenders und kehren nach der Belagerung heim.
  if self.invasion_v1()&&matches!(flug["mission"].as_str(),Some("verstaerken")|Some("halten")){
   let ziel:Koord=flug["ziel"].as_str().ok_or("Ziel fehlt")?.parse()?;let zp=*self.belegung.get(&ziel).ok_or("Am Ziel liegt kein Planet")? as usize;let partner=self.planeten[zp].besitzer;
   if (partner==sid||!self.verbuendet(sid,partner))&&!self.angriffshilfe_erlaubt(sid,zp){return Err("Verstärken geht nur zu Planeten von Verbündeten (Allianz oder Verteidigungsbündnis); eigene Planeten mit transport oder stationieren".into());}
   flug["mission"]=json!("halten");if flug.get("haltedauer_stunden").is_none_or(|v|v.is_null()){flug["haltedauer_stunden"]=json!(std::cmp::min(48i64,self.regeln.flug.halten_max_stunden as i64));}
  }
  let flug=&flug;
  let mission=flug["mission"].as_str().ok_or("Mission fehlt")?;let erlaubt:&[&str]=if self.invasion_v1(){&["transport","stationieren","kampfkolonisieren","halten","kolonisieren"]}else{&["transport","stationieren","kampfkolonisieren","kolonisieren"]};
  // Invasion v2: Bomber mit Bodentruppen landen bei 30 % (besetzung_beginnen) – bis 25.10.2026 ließ das Verladen das nicht zu.
  let bomber_landung=mission=="bombardieren"&&self.invasion_v2();
  if !erlaubt.contains(&mission)&&!bomber_landung{return Err(if self.invasion_v2(){"Bodentruppen können transportiert, stationiert, zur Kampfkolonisation oder beim Bombardieren zur Landung mitgeführt oder zu Verbündeten (verstaerken) geschickt werden"}else if self.invasion_v1(){"Bodentruppen können transportiert, stationiert, zur Kampfkolonisation mitgeführt oder zu Verbündeten (verstaerken) geschickt werden"}else{"Bodentruppen können transportiert, stationiert oder zur Kampfkolonisation mitgeführt werden"}.into());}
  // kolonisieren (Karl 24.10.2026): Truppen landen bei der Gründung als Garnison der neuen Kolonie, sonst fliegen sie heim.
  if !matches!(mission,"transport"|"kampfkolonisieren"|"halten"|"kolonisieren"|"bombardieren"){self.eigener_planet(sid,flug["ziel"].as_str().ok_or("Ziel fehlt")?.parse()?)?;}
  let available=self.ausbau.boden.bestand.get(&(pid as u32)).copied().unwrap_or_default();if (0..ANZAHL).any(|i|truppen[i]>available[i]){return Err("Bodeneinheiten fehlen auf dem Startplaneten".into());}
  let je=self.regeln.kw().boden.transporter_plaetze;let capacity=flug["schiffe"]["truppentransporter"].as_i64().unwrap_or(0)*je;let needed=(0..ANZAHL).map(|i|truppen[i]*self.regeln.kw().boden.plaetze(i)).sum::<i64>();if needed>capacity{return Err(format!("{needed} Truppenplätze benötigt, {capacity} vorhanden; {je} je Truppentransporter"));}
  let mut next=self.clone();let fid=next.naechste_flotte;let(ok,msg)=next.handeln(sid,rolle,flug);if !ok{return Err(msg);}
  next.ausbau.boden.bestand.insert(pid as u32,std::array::from_fn(|i|available[i]-truppen[i]));next.ausbau.boden.flotten.insert(fid,truppen);*self=next;Ok(format!("Bodentruppen mit Flotte {fid} gestartet"))
 }
 pub(crate) fn boden_landen(&mut self,fid:FlottenId,pid:usize){self.boden_kapazitaet(fid);if let Some(units)=self.ausbau.boden.flotten.remove(&fid){let g=self.ausbau.boden.bestand.entry(pid as u32).or_default();for i in 0..ANZAHL{g[i]+=units[i];}}}
 pub(crate) fn boden_verstaerken(&mut self,from:FlottenId,to:FlottenId){
  if let Some(units)=self.ausbau.boden.flotten.remove(&from){let target=self.ausbau.boden.flotten.entry(to).or_default();for i in 0..ANZAHL{target[i]+=units[i];}}
  if let Some(people)=self.ausbau.passagiere.remove(&from){self.ausbau.passagiere.entry(to).or_default().add(&people);}
  if let Some(people)=self.ausbau.siedler.remove(&from){self.ausbau.siedler.entry(to).or_default().add(&people);}
 }
 pub(crate) fn boden_kapazitaet(&mut self,fid:FlottenId){
  let rg=self.regeln.clone();let bw=&rg.kw().boden;
  let Some(a)=self.ausbau.boden.flotten.get_mut(&fid)else{return;};let mut space=self.flotten.get(&fid).map(|f|f.schiffe[Einheit::Truppentransporter.idx()]*bw.transporter_plaetze).unwrap_or(0);
  for i in 0..ANZAHL{a[i]=a[i].min(space/bw.plaetze(i));space-=a[i]*bw.plaetze(i);}
 }
 /// Deterministic simultaneous rounds, at most once per 15 game minutes.
 pub(crate) fn boden_abwehr(&mut self,pid:usize,fid:FlottenId)->bool{
  if self.ausbau.industrie_version==0{return true;}
  self.boden_kapazitaet(fid);
  let a0=self.ausbau.boden.flotten.get(&fid).copied().unwrap_or_default();let d0=self.ausbau.boden.bestand.get(&(pid as u32)).copied().unwrap_or_default();
  let a=std::array::from_fn(|i|if i<ANZAHL{a0[i]}else{0});
  let d=std::array::from_fn(|i|if i<ANZAHL{d0[i]}else{self.ausbau.boden.miliz.get(&(pid as u32)).map(|m|m.personen.gesamt()/M).unwrap_or(0)});
  if d.iter().sum::<i64>()==0{return true;}
  if a.iter().sum::<i64>()==0{self.besetzung_abbrechen(pid as u32,"Bodentruppen und Miliz haben die Landung abgewehrt; Angreifer ohne einsatzfähige Truppen");return false;}
  let last=self.ausbau.boden.letzte_runde.get(&fid).copied().unwrap_or_else(||self.kolonisation.besetzungen.get(&(pid as u32)).map(|b|b.seit).unwrap_or(self.zeit));if self.zeit-last<15*MINUTE{return false;}
  let mut wounds=self.ausbau.boden.wunden.get(&fid).copied().unwrap_or_default();
  let vorher_wunden=wounds;
  let rg=self.regeln.clone();let bw=&rg.kw().boden;let kl=crate::boden_kampf::KLASSISCH;
  let asupport=crate::boden_kampf::unterstuetzung_bw(bw,&a,&mut wounds[0]);let dsupport=crate::boden_kampf::unterstuetzung_bw(bw,&d,&mut wounds[1]);
  let (dl,af,dw)=crate::boden_kampf::feuern_bw(bw,&a,&d,wounds[1],kl);let (al,df,aw)=crate::boden_kampf::feuern_bw(bw,&d,&a,wounds[0],kl);
  let an:[i64;ANZAHL]=std::array::from_fn(|i|a[i]-al[i]);let dn:[i64;ANZAHL]=std::array::from_fn(|i|d[i]-dl[i]);
  self.ausbau.boden.wunden.insert(fid,[aw,dw]);self.ausbau.boden.flotten.insert(fid,an);self.ausbau.boden.bestand.insert(pid as u32,dn);self.ausbau.boden.letzte_runde.insert(fid,self.zeit);self.miliz_verluste(pid,dl[ANZAHL]);
  self.ausbau.boden.berichte.push(json!({"zeit":self.zeit,"planet":self.planeten[pid].koord.to_string(),"angreifer":self.flotten[&fid].besitzer,"verteidiger":self.planeten[pid].besitzer,"typen":(0..KAMPFZAHL).map(crate::boden_kampf::name).collect::<Vec<_>>(),"angreifer_name":self.spieler[self.flotten[&fid].besitzer as usize].name,"verteidiger_name":self.spieler[self.planeten[pid].besitzer as usize].name,"struktur_je_einheit":(0..KAMPFZAHL).map(|i|crate::boden_kampf::struktur_bw(bw,i,kl)).collect::<Vec<_>>(),"wunden_vorher":vorher_wunden,"wunden_nachher":[aw,dw],"angreifer_vorher":a,"verteidiger_vorher":d,"angreifer_feuer":af,"verteidiger_feuer":df,"angreifer_unterstuetzung":asupport,"verteidiger_unterstuetzung":dsupport,"angreifer_verluste":al,"verteidiger_verluste":dl,"angreifer_nachher":(0..KAMPFZAHL).map(|i|a[i]-al[i]).collect::<Vec<_>>(),"verteidiger_nachher":(0..KAMPFZAHL).map(|i|d[i]-dl[i]).collect::<Vec<_>>()}));
  self.bodenberichte_kappen();
  let defenders=dn.iter().sum::<i64>()+d[ANZAHL]-dl[ANZAHL];
  if an.iter().sum::<i64>()==0&&defenders>0{self.besetzung_abbrechen(pid as u32,"Bodenverteidigung hat die Invasion zurückgeschlagen");}
  defenders==0
 }
 pub fn boden_sicht(&self,pid:usize)->Value{let bw=&self.regeln.kw().boden;json!({"fertigungsplaetze_je_stueck":bw.spalte(|t|t.fertigungsplaetze),"fertigungsplaetze":self.planeten[pid].gebaeude[Gebaeude::Kaserne.idx()],"bestand":self.ausbau.boden.bestand.get(&(pid as u32)).copied().unwrap_or_default(),"typen":NAMEN,"bilder":BILDER,"namen":LABELS,"rollen":crate::boden_katalog::ROLLEN,"ab_stufe":bw.spalte(|t|t.ab_stufe),"kaserne":bw.spalte(|t|t.kaserne),"kontermatrix_promille":(0..ANZAHL).map(|i|(0..KAMPFZAHL).map(|j|bw.faktor(i,j)).collect::<Vec<_>>()).collect::<Vec<_>>(),"angriff":bw.spalte(|t|t.angriff),"struktur":bw.spalte(|t|t.struktur),"truppenplaetze":bw.spalte(|t|t.plaetze),"personal":bw.spalte(|t|t.personal),"kosten_milli":(0..ANZAHL).map(|i|bw.kosten(i)).collect::<Vec<_>>(),"kosten_aktuell_milli":(0..ANZAHL).map(|i|self.produktionskosten(pid,Gebaeude::Kaserne,bw.kosten(i))).collect::<Vec<_>>(),"grunddauer_sekunden":bw.spalte(|t|t.dauer_sekunden),"auftraege":self.ausbau.boden.auftraege.get(&(pid as u32)),"ruht":self.werk_ruht(pid,Gebaeude::Kaserne),"miliz":self.miliz_sicht(pid),"konter":"Grunddreieck: Sturminfanterie gegen leichte Drohnen, Sturmpanzer gegen Infanterie, Kampfdrohnen gegen Panzer. Spezialisierungen brechen es gezielt auf; genaue Faktoren stehen in jedem Datenblatt. Alle Völker nutzen dieselben Werte und Konter. Bodenrunden alle 15 Spielminuten, gleichzeitiges Feuer, verbleibende Schäden bleiben bestehen."})}
}
