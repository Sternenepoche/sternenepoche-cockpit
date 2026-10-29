//! Physical manufacturing places, shared by concurrent paid orders.
use crate::{typen::*,welt::*,Welt};
use serde::{Serialize,Deserialize};
use serde_json::{json,Value};
#[derive(Clone,Debug,Serialize,Deserialize)]
pub struct Los{pub auftrag:usize,pub anzahl:i64,pub plaetze:i64,pub ende:i64,pub dauer:i64}
pub fn platzbedarf(p:Produkt)->i64{match p{
 Produkt::Bauteil(_)=>1,
 Produkt::Einheit(e)=>match e{
  Einheit::Kreuzer|Einheit::Schildkreuzer|Einheit::Raketenkreuzer|Einheit::Flottenkreuzer|Einheit::Kolonieschiff|Einheit::Recycler|Einheit::GrosserTransporter|Einheit::Truppentransporter=>2,
  Einheit::Schlachtschiff|Einheit::Bomber|Einheit::Zerstoerer|Einheit::Schwerlastfrachter|Einheit::Handelsfrachter|Einheit::Pathfinder|Einheit::Bergungsschiff|Einheit::Gausskanone|Einheit::Planetenschild=>3,
  Einheit::Sturmschlachtschiff|Einheit::Traegerschlachtschiff|Einheit::Belagerungsschlachtschiff|Einheit::StrategischerBomber|Einheit::Abfangzerstoerer|Einheit::SchwererZerstoerer|Einheit::GrossesKolonieschiff|Einheit::Plasmawerfer|Einheit::PlanetenschildII|Einheit::PlanetenschildIII=>4,
  _=>1
 }
}}
pub fn key(pid:usize,n:usize)->String{format!("{pid}:{n}")}
impl Welt{
 pub fn fertigungsplaetze(&self,pid:usize,n:usize)->i64{self.planeten[pid].gebaeude[if n==0{Gebaeude::Werft}else{Gebaeude::Orbitalwerft}.idx()] as i64}
 pub(crate) fn parallel_sync(&mut self,pid:usize,n:usize){
  if self.ausbau.industrie_version==0{return;}
  let key=key(pid,n);let q=&self.planeten[pid].fertigung[n];
  if q.is_empty(){self.ausbau.parallel.insert(key.clone(),vec![]);}
  if let Some(lots)=self.ausbau.parallel.get_mut(&key){lots.retain(|b|b.auftrag<q.len());}
  // Preserve the already running piece when upgrading a sequential save.
  if !self.ausbau.parallel.contains_key(&key)&&!q.is_empty()&&self.planeten[pid].fertigung_naechste[n]>=self.zeit{
   self.ausbau.parallel.insert(key.clone(),vec![Los{auftrag:0,anzahl:1,plaetze:platzbedarf(q[0].produkt),ende:self.planeten[pid].fertigung_naechste[n],dauer:q[0].dauer}]);
  }
  let capacity=self.fertigungsplaetze(pid,n);let lots=self.ausbau.parallel.entry(key).or_default();let mut free=capacity-lots.iter().map(|b|b.anzahl*b.plaetze).sum::<i64>();
  for(i,a)in q.iter().enumerate(){let pending=a.rest-lots.iter().filter(|b|b.auftrag==i).map(|b|b.anzahl).sum::<i64>();let weight=platzbedarf(a.produkt);let count=pending.min(free.max(0)/weight);if count>0{lots.push(Los{auftrag:i,anzahl:count,plaetze:weight,ende:self.zeit+a.dauer,dauer:a.dauer});free-=count*weight;}}
  let end=lots.iter().map(|b|b.ende).min();self.planeten[pid].fertigung_naechste[n]=end.unwrap_or(-1);
  self.ereignisse.retain(|e|!matches!(e.art,EreignisArt::FertigungFertig{planet,schleife}if planet==pid as u32&&schleife==n as u8));
  if let Some(t)=end{self.plane(t,EreignisArt::FertigungFertig{planet:pid as u32,schleife:n as u8});}
 }
 pub(crate) fn parallel_fertig(&mut self,pid:usize,n:usize){
  let key=key(pid,n);let lots=self.ausbau.parallel.remove(&key).unwrap_or_default();let(mut done,remaining):(Vec<_>,Vec<_>)=lots.into_iter().partition(|b|b.ende<=self.zeit);self.ausbau.parallel.insert(key.clone(),remaining);
  done.sort_by_key(|b|b.auftrag);self.abrechnen(pid);
  for b in done{let Some(a)=self.planeten[pid].fertigung[n].get(b.auftrag)else{continue;};let product=a.produkt;let count=b.anzahl.min(a.rest);
   for _ in 0..count{self.crew_fertig(pid,product);}
   let p=&mut self.planeten[pid];match product{Produkt::Einheit(e)=>{if matches!(e,Einheit::Planetenschild|Einheit::PlanetenschildII|Einheit::PlanetenschildIII){for s in [Einheit::Planetenschild,Einheit::PlanetenschildII,Einheit::PlanetenschildIII]{p.einheiten[s.idx()]=0;}}p.einheiten[e.idx()]+=count;},Produkt::Bauteil(g)=>p.bestand[g.idx()]+=count*M}
   p.fertigung[n][b.auftrag].rest-=count;
  }
  for index in (0..self.planeten[pid].fertigung[n].len()).rev(){if self.planeten[pid].fertigung[n][index].rest==0{let a=self.planeten[pid].fertigung[n].remove(index);if let Some(q)=self.ausbau.fertigung_zahlung.get_mut(&key){if index<q.len(){q.remove(index);}}
   for b in self.ausbau.parallel.get_mut(&key).unwrap(){if b.auftrag>index{b.auftrag-=1;}}
   let product=match a.produkt{Produkt::Einheit(e)=>e.name(),Produkt::Bauteil(g)=>g.name()};self.vorfall(self.planeten[pid].besitzer,"fertigung",format!("Auftrag {product} auf {} abgeschlossen",self.planeten[pid].koord));
  }}
  self.parallel_sync(pid,n);self.raten_neu(pid);
 }
 pub(crate) fn parallel_storno(&mut self,pid:usize,n:usize,index:usize){
  let key=key(pid,n);let lots=self.ausbau.parallel.entry(key.clone()).or_default();let charged=lots.iter().filter(|b|b.auftrag==index&&(b.ende-self.zeit).max(0)*4<=b.dauer).map(|b|b.anzahl).sum::<i64>();
  lots.retain(|b|b.auftrag!=index);for b in lots{if b.auftrag>index{b.auftrag-=1;}}
  let a=self.planeten[pid].fertigung[n].remove(index);let paid=self.ausbau.fertigung_zahlung.get_mut(&key).and_then(|q|if index<q.len(){Some(q.remove(index))}else{None});
  let cost=paid.map(|(_,c)|c).unwrap_or_else(||match a.produkt{Produkt::Einheit(e)=>self.regeln.kosten_einheit(e,self.spieler[self.planeten[pid].besitzer as usize].volk),Produkt::Bauteil(g)=>self.regeln.kosten_bauteil(g).unwrap()});
  self.erstattung(pid,cost.map(|c|c*(a.rest-charged).max(0)),paid.and_then(|(t,_)|t));
  if n==0&&a.produkt==Produkt::Bauteil(Gut::Habitatmodul){self.planeten[pid].einheiten[Einheit::Recycler.idx()]+=(a.rest-charged).max(0);}
  if let Produkt::Einheit(e)=a.produkt{let crew=self.regeln.einh(e).besatzung*M*a.rest;self.planeten[pid].bevoelkerung+=crew;self.crew_erstatten(pid,a.produkt,crew);self.produktionsserie_stoppen(pid,"einheit",e.name());}else if let Produkt::Bauteil(g)=a.produkt{self.produktionsserie_stoppen(pid,"bauteil",g.name());}
  self.parallel_sync(pid,n);self.raten_neu(pid);
 }
 pub fn parallel_sicht(&self,pid:usize,n:usize)->Value{let lots=self.ausbau.parallel.get(&key(pid,n)).cloned().unwrap_or_default();let occupied=lots.iter().map(|b|b.anzahl*b.plaetze).sum::<i64>();json!({"plaetze":self.fertigungsplaetze(pid,n),"belegt":occupied,"frei":(self.fertigungsplaetze(pid,n)-occupied).max(0),"lose":lots})}
}
