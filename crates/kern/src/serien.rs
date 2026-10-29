//! Per-planet stock targets. Every repeat uses the ordinary production checks and payments.
use crate::{typen::*,welt::Produkt,Welt};
use serde::{Serialize,Deserialize};
use serde_json::{json,Value};
#[derive(Clone,Debug,Serialize,Deserialize)]
pub struct Produktionsserie{pub besitzer:SpielerId,pub rolle:Rolle,pub art:String,pub produkt:String,pub ziel:i64,pub status:String}
impl Welt{
 /// Stock includes paid orders, so parallel replenishment cannot overshoot the target.
 pub(crate) fn serien_vorrat_plaetze(&self,pid:usize,art:&str,produkt:&str)->(i64,i64,Gebaeude){
  let p=&self.planeten[pid];match art{
   "einheit"|"bauteil"=>{let product=if art=="einheit"{Produkt::Einheit(Einheit::aus_name(produkt).unwrap())}else{Produkt::Bauteil(Gut::aus_name(produkt).unwrap())};let n=usize::from(art=="bauteil"&&p.gebaeude[Gebaeude::Orbitalwerft.idx()]>0);let stock=match product{Produkt::Einheit(e)=>p.einheiten[e.idx()],Produkt::Bauteil(g)=>p.bestand[g.idx()]/M};let pending=p.fertigung[n].iter().filter(|a|a.produkt==product).map(|a|a.rest).sum::<i64>();let used=self.ausbau.parallel.get(&crate::parallel::key(pid,n)).map(|v|v.iter().map(|b|b.anzahl*b.plaetze).sum()).unwrap_or(0);(stock+pending,(self.fertigungsplaetze(pid,n)-used).max(0)/crate::parallel::platzbedarf(product),if n==0{Gebaeude::Werft}else{Gebaeude::Orbitalwerft})},
   "boden"=>{let i=crate::boden::NAMEN.iter().position(|n|*n==produkt).unwrap();let q=self.ausbau.boden.auftraege.get(&p.id).cloned().unwrap_or_default();let stock=self.ausbau.boden.bestand.get(&p.id).map(|a|a[i]).unwrap_or(0)+q.iter().filter(|a|a.art==i).map(|a|a.rest).sum::<i64>();let used=q.iter().map(|a|a.lose.iter().map(|b|b.anzahl*self.regeln.kw().boden.fertigungsplaetze(a.art)).sum::<i64>()).sum::<i64>();(stock,(p.gebaeude[Gebaeude::Kaserne.idx()] as i64-used).max(0)/self.regeln.kw().boden.fertigungsplaetze(i),Gebaeude::Kaserne)},
   "rakete"=>{let a=crate::raketen::art(produkt).unwrap();let stock=self.raketen_bestand(pid,a)+self.ereignisse.iter().filter_map(|e|match e.art{crate::welt::EreignisArt::RaketenFertig{planet,art,anzahl,..}if planet==p.id&&art==a=>Some(anzahl),_=>None}).sum::<i64>();let free=self.silo_parallel_sicht(pid)["frei"].as_i64().unwrap()/crate::parallel_silo::plaetze(a);(stock,free,Gebaeude::Raketensilo)},_=>unreachable!()
  }
 }
 pub(crate) fn produktionsserie_stoppen(&mut self,pid:usize,art:&str,produkt:&str){
  if self.ausbau.industrie_version==0{return;}
  let key=format!("{art}:{produkt}");let sid=self.planeten[pid].besitzer;
  if let Some(c)=self.ausbau.produktionsserien.get_mut(&(pid as u32)).and_then(|m|m.get_mut(&key)){c.ziel=0;c.status="Durch Auftragsabbruch gestoppt".into();}
  else if art=="bauteil"{self.ausbau.produktionsserien.entry(pid as u32).or_default().insert(key,Produktionsserie{besitzer:sid,rolle:Rolle::Alle,art:art.into(),produkt:produkt.into(),ziel:0,status:"Durch Auftragsabbruch gestoppt".into()});}
 }
 pub fn produktionsserie_setzen(&mut self,sid:SpielerId,rolle:Rolle,k:Koord,art:&str,produkt:&str,ziel:i64)->Result<String,String>{
  let pid=self.eigener_planet(sid,k)?;
  if self.ausbau.industrie_version==0||!(0..=10000).contains(&ziel){return Err("Vorratsziel muss zwischen 0 (aus) und 10000 liegen".into());}
  match art {
   "einheit"=>{let e:Einheit=Einheit::aus_name(produkt).ok_or("Unbekannte Einheit")?;if !self.regeln.einheiten.contains_key(&e){return Err("Einheit nicht verfügbar".into());}},
   "bauteil"=>{let g:Gut=Gut::aus_name(produkt).ok_or("Unbekanntes Gut")?;if self.regeln.kosten_bauteil(g).is_none(){return Err("Kein Bauteil".into());}},
   "rakete"=>{crate::raketen::art(produkt)?;if rolle==Rolle::Verwalter{return Err("Raketenserien bestellt der Feldherr".into());}},
   "boden"=>{if !crate::boden::NAMEN.contains(&produkt){return Err("Unbekannte Bodeneinheit".into());}if rolle==Rolle::Verwalter{return Err("Bodentruppen bestellt der Feldherr".into());}},
   _=>return Err("Produktionstyp muss einheit, bauteil, rakete oder boden sein".into())
  }
  self.ausbau.produktionsserien.entry(pid as u32).or_default().insert(format!("{art}:{produkt}"),Produktionsserie{besitzer:sid,rolle,art:art.into(),produkt:produkt.into(),ziel,status:if ziel==0{"Aus · bezahlte Aufträge laufen weiter".into()}else{"Wartet auf Produktionsprüfung".into()}});
  self.produktionsserien_tick();Ok(format!("{produkt} auf {k}: {}",if ziel==0{"Wiederholung ausgeschaltet".into()}else{format!("lokalen Vorrat bis {ziel} automatisch nachfüllen")}))
 }
 pub(crate) fn produktionsserien_tick(&mut self){
  let configs=self.ausbau.produktionsserien.clone();
  for (id,entries) in configs{let pid=id as usize;if !self.planet_aktiv(pid){continue;}
   for (key,c) in entries{
    if self.planeten[pid].besitzer!=c.besitzer{self.ausbau.produktionsserien.get_mut(&id).unwrap().remove(&key);continue;}
    if c.ziel==0{continue;}
    let p=&self.planeten[pid];let k=p.koord;
    let (stock,free,work)=self.serien_vorrat_plaetze(pid,&c.art,&c.produkt);let count=(c.ziel-stock).min(free).min(1000);
    let paused=self.werk_ruht(pid,work)||self.ausbau.pausen.contains(&format!("{}:{id}",if work==Gebaeude::Orbitalwerft{"bauteile"}else{"werft"}));
    let status=if stock>=c.ziel{"Vorratsziel erreicht".into()}else if paused{"Pausiert · Leistung, Energie, Personal oder Auftragspause prüfen".into()}else if free==0{"Wartet auf freie Produktion · vorhandene Aufträge zuerst".into()}else{
     let mut amount=count;let result=loop{let result=match c.art.as_str(){
      "einheit"=>self.fertigen(c.besitzer,c.rolle,k,Produkt::Einheit(Einheit::aus_name(&c.produkt).unwrap()),amount),
      "bauteil"=>self.fertigen(c.besitzer,c.rolle,k,Produkt::Bauteil(Gut::aus_name(&c.produkt).unwrap()),amount),
      "rakete"=>self.taktisch_bauen(c.besitzer,c.rolle,k,&c.produkt,amount),
      "boden"=>self.boden_bauen_rolle(c.besitzer,c.rolle,k,&c.produkt,amount),_=>unreachable!()
     };if result.is_ok()||amount<=1{break result;}amount-=1;};match result{Ok(_)=>format!("{amount} Stück bezahlt und gleichzeitig gestartet"),Err(e)=>format!("Wartet: {e}")}
    };
    if let Some(s)=self.ausbau.produktionsserien.get_mut(&id).and_then(|m|m.get_mut(&key)){s.status=status;}
   }
  }
 }
 pub fn produktionsserien_sicht(&self,pid:usize)->Value{json!(self.ausbau.produktionsserien.get(&(pid as u32)))}
}
