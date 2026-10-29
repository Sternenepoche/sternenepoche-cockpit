//! Emergency defenders are reserved civilian workers, not free permanent soldiers.
use crate::{typen::*,Welt,boden_katalog::Miliz,ausbau::Personen};
use serde_json::{json,Value};
impl Welt{
 pub fn miliz_sicht(&self,pid:usize)->Value{
  if self.invasion_v1(){return self.miliz_sicht_v1(pid);}
  let p=&self.planeten[pid];let active=self.ausbau.boden.miliz.get(&p.id).map(|m|m.personen.gesamt()/M).unwrap_or(0);
  let workers=self.ausbau.personen.get(&p.id).map(|p|p.gruppen.iter().map(|g|g[0]).sum::<i64>()/M).unwrap_or(0);
  let free=((p.bevoelkerung/M+active)/4-active).max(0).min(workers).min((p.bevoelkerung/M-500).max(0));
  let besetzt=self.kolonisation.besetzungen.get(&p.id).is_some_and(|b|b.verteidiger==p.besitzer);
  json!({"aktiv":active,"verfuegbar":if besetzt{free}else{0},"belagert":besetzt,"angriff":4,"struktur":20,"kosten":{"legierung":5,"elektronik":1},"beschreibung":"Nur während einer laufenden Invasion: bis zu 25 % der Bevölkerung als Miliz. Nur ungelernte Arbeitskräfte, mindestens 500 Zivilisten bleiben. Sofort einsatzbereit, nicht transportierbar. Eingezogene fehlen in der Produktion. Überlebende kehren nach dem Ende der Belagerung automatisch zurück; Verluste sind dauerhaft. Ausrüstungskosten werden nicht erstattet."})
 }
 pub fn miliz_aufrufen(&mut self,sid:SpielerId,rolle:Rolle,k:Koord,n:i64)->Result<String,String>{
  // Invasion v1: die Volksmiliz bewaffnet immer alle Bewohner; die Anzahl ist ohne Bedeutung.
  if self.invasion_v1(){let _=n;return self.miliz_ausrufen(sid,rolle,k);}
  let pid=self.eigener_planet(sid,k)?;self.personen_abgleichen(pid);let q=self.miliz_sicht(pid);
  if q["belagert"]!=true{return Err("Miliz kann nur während einer laufenden Invasion gerufen werden".into());}
  if n<1||n>100_000||n>q["verfuegbar"].as_i64().unwrap_or(0){return Err("Nicht genug einziehbare Arbeitskräfte; 25-%-Grenze und 500 verbleibende Zivilisten beachten".into());}
  let mut eligible=self.ausbau.personen[&(pid as u32)].clone();for g in &mut eligible.gruppen{g[1]=0;g[2]=0;}
  let recruited=eligible.auswahl(n*M);let mut cost=[0;GUETER];cost[Gut::Legierung.idx()]=5*n*M;cost[Gut::Elektronik.idx()]=n*M;
  let topf=Self::topf_fuer(rolle,Topf::Militaer);self.abrechnen(pid);self.zahlbar(pid,&cost,topf)?;
  self.ausbau.personen.get_mut(&(pid as u32)).unwrap().nehmen(&recruited)?;self.zahlen(pid,&cost,topf);self.planeten[pid].bevoelkerung-=n*M;
  self.ausbau.boden.miliz.entry(pid as u32).or_insert(Miliz{besitzer:sid,personen:Personen::default()}).personen.add(&recruited);self.raten_neu(pid);
  Ok(format!("{n} Einwohner verteidigen als Miliz; Arbeitskräfte und Ausrüstung wurden tatsächlich entnommen"))
 }
 pub(crate) fn miliz_verluste(&mut self,pid:usize,n:i64){if let Some(m)=self.ausbau.boden.miliz.get_mut(&(pid as u32)){let lost=m.personen.auswahl(n*M);m.personen.nehmen(&lost).expect("militia casualties bounded by strength");}}
 pub(crate) fn miliz_entlassen(&mut self,pid:usize){
  if let Some(m)=self.ausbau.boden.miliz.remove(&(pid as u32)){
   if m.besitzer==self.planeten[pid].besitzer&&m.personen.gesamt()>0{
    self.abrechnen(pid);self.planeten[pid].bevoelkerung+=m.personen.gesamt();self.ausbau.personen.entry(pid as u32).or_default().add(&m.personen);self.raten_neu(pid);
    self.vorfall(m.besitzer,"miliz",format!("{} Milizionäre kehren auf {} zur zivilen Arbeit zurück",m.personen.gesamt()/M,self.planeten[pid].koord));
   }
  }
 }
 // Ohne Belagerung entlassen – außer einer stehenden Miliz neuer Welten (`miliz_jederzeit`), die der Spieler selbst entlässt.
 pub(crate) fn miliz_aufraeumen(&mut self){let jederzeit=self.regeln.kw().boden.miliz_jederzeit;let ids=self.ausbau.boden.miliz.keys().copied().filter(|id|!self.kolonisation.besetzungen.contains_key(id)&&!(jederzeit&&self.ausbau.boden.invasion.milizen.get(id).is_some_and(|a|!a.kapituliert&&a.besitzer==self.planeten[*id as usize].besitzer))).collect::<Vec<_>>();for id in ids{self.miliz_entlassen(id as usize);}}
 /// 0.4.8: stehende Miliz entlassen (die Milizionäre kehren zur Arbeit zurück; die Ausrüstung ist verbraucht).
 pub fn miliz_entlassen_befehl(&mut self,sid:SpielerId,k:Koord)->Result<String,String>{
  let pid=self.eigener_planet(sid,k)?;let id=pid as u32;
  if self.kolonisation.besetzungen.contains_key(&id){return Err("Während einer Belagerung bleibt die Miliz im Dienst".into());}
  if !self.ausbau.boden.invasion.milizen.get(&id).is_some_and(|a|!a.kapituliert){return Err("Auf diesem Planeten ist keine Miliz ausgerufen".into());}
  let n=self.ausbau.boden.miliz.get(&id).map(|m|m.personen.gesamt()/M).unwrap_or(0);
  self.ausbau.boden.invasion.milizen.remove(&id);self.miliz_entlassen(pid);
  Ok(format!("Miliz auf {k} entlassen: {n} Milizionäre arbeiten wieder"))
 }
}
