//! Concurrent silo production; storage slots and manufacturing places are distinct.
use crate::{typen::*,welt::*,Welt};
use serde_json::{json,Value};
pub fn plaetze(art:u8)->i64{[1,2,2,3,4][art as usize]}
impl Welt{
 pub(crate) fn silo_parallel_sync(&mut self,pid:usize){
  let mut ids=self.ereignisse.iter().filter_map(|e|match e.art{EreignisArt::RaketenFertig{planet,art,..}if planet==pid as u32=>Some((e.seq,art)),_=>None}).collect::<Vec<_>>();ids.sort_unstable();
  let mut free=self.planeten[pid].gebaeude[Gebaeude::Raketensilo.idx()] as i64-ids.iter().filter_map(|(id,_)|self.ausbau.raketen_zahlung.get(id)).flat_map(|p|&p.lose).map(|b|b.anzahl*b.plaetze).sum::<i64>();
  for(id,art)in &ids{if let Some(p)=self.ausbau.raketen_zahlung.get_mut(id).filter(|p|p.basis>0){let n=(p.rest-p.lose.iter().map(|b|b.anzahl).sum::<i64>()).min(free.max(0)/plaetze(*art));if n>0{p.lose.push(crate::parallel::Los{auftrag:0,anzahl:n,plaetze:plaetze(*art),ende:self.zeit+p.basis,dauer:p.basis});free-=n*plaetze(*art);}}}
  let next=ids.iter().filter_map(|(id,_)|self.ausbau.raketen_zahlung.get(id)).flat_map(|p|&p.lose).map(|b|b.ende).min();
  let queue=std::mem::take(&mut self.ereignisse);self.ereignisse=queue.into_iter().map(|mut e|{if ids.iter().any(|(id,_)|*id==e.seq){if let Some(p)=self.ausbau.raketen_zahlung.get(&e.seq).filter(|p|p.basis>0){e.zeit=p.lose.iter().map(|b|b.ende).min().or(next).unwrap_or(self.zeit+60);}}e}).collect();
 }
 pub(crate) fn silo_parallel_fertig(&mut self,mut event:Ereignis){
  let EreignisArt::RaketenFertig{planet,besitzer,art,..}=event.art else{return;};let pid=planet as usize;
  let Some(mut paid)=self.ausbau.raketen_zahlung.remove(&event.seq)else{return;};
  if self.planeten[pid].besitzer!=besitzer{return;}
  let n=paid.lose.iter().filter(|b|b.ende<=self.zeit).map(|b|b.anzahl).sum::<i64>();paid.lose.retain(|b|b.ende>self.zeit);paid.rest-=n;
  if n>0{self.raketen_fertig(pid,besitzer,art,n);}
  if paid.rest>0{event.art=EreignisArt::RaketenFertig{planet,besitzer,art,anzahl:paid.rest};event.zeit=self.zeit+60;self.ausbau.raketen_zahlung.insert(event.seq,paid);self.ereignisse.push(event);}
  self.silo_parallel_sync(pid);
 }
 pub fn silo_parallel_sicht(&self,pid:usize)->Value{let total=self.planeten[pid].gebaeude[Gebaeude::Raketensilo.idx()] as i64;let used=self.ereignisse.iter().filter(|e|matches!(e.art,EreignisArt::RaketenFertig{planet,..}if planet==pid as u32)).filter_map(|e|self.ausbau.raketen_zahlung.get(&e.seq)).flat_map(|p|&p.lose).map(|b|b.anzahl*b.plaetze).sum::<i64>();json!({"plaetze":total,"belegt":used,"frei":(total-used).max(0)})}
}
