//! Real queue controls with stale-view protection and no restarting paid work on reorder.
use crate::{typen::*,welt::*,Welt};
use serde_json::{json,Value};use sha2::{Digest,Sha256};
impl Welt {
 fn warte_auf_kosten(&self,pid:usize,kosten:&[i64;GUETER],topf:Option<Topf>)->String {
  let bestand=self.bestand_jetzt(pid);
  let mut gruende=Vec::new();
  for g in Gut::ALLE {let fehlt=kosten[g.idx()]-bestand[g.idx()];if fehlt>0 {gruende.push(format!("{} {} fehlen",(fehlt+M-1)/M,g));}}
  if let Some(t)=topf {let fehlt=self.regeln.wert(kosten)-self.topf_guthaben(self.planeten[pid].besitzer,t);if fehlt>0{gruende.push(format!("Budget {t}: {} Werteinheiten fehlen",(fehlt+M-1)/M));}}
  if gruende.is_empty(){"Bereit · automatischer Start im nächsten Simulationstakt".into()}else{format!("Wartet: {}",gruende.join(" · "))}
 }
 /// Retry unpaid plans at simulation boundaries and immediately after deliveries/events.
 /// Research progress remains hourly; this only starts a job and charges it once.
 pub(crate) fn wartende_auftraege_starten(&mut self) {
  if self.ausbau.industrie_version == 0 { return; }
  let pending:Vec<_>=self.planeten.iter().enumerate().filter(|(_,p)|p.bauschleife.first().is_some_and(|a|a.fertig.is_none())).map(|(pid,_)|pid).collect();
  for pid in pending { if self.planet_aktiv(pid) { self.bau_starten(pid); } }
  for sid in 0..self.spieler.len() { if self.spieler_aktiv(sid as SpielerId) { self.forschung_schlange_starten(sid as SpielerId); } }
 }
 pub fn warteschlangen_sicht(&self,sid:SpielerId)->Vec<Value>{
  let s=&self.spieler[sid as usize];let mut q=vec![self.warteschlange_sicht(sid,"forschung",s.heimat as usize)];
  for &pid in &s.planeten{for kind in ["bau","werft","bauteile"]{q.push(self.warteschlange_sicht(sid,kind,pid as usize));}}q
 }
 pub fn warteschlange_sicht(&self,sid:SpielerId,art:&str,pid:usize)->Value {
  let p=&self.planeten[pid];let s=&self.spieler[sid as usize];let mut jobs=vec![];
  match art {
   "bau"=>for (i,a)in p.bauschleife.iter().enumerate(){
    let grund=if i>0{"Wartet auf vorherigen Auftrag".into()}else if self.auftrag_pausiert("bau",p.id){"Bauschlange manuell pausiert".into()}else if self.bau_ruht_unruhen(pid){"Bau ruht wegen Unruhen".into()}else if a.fertig.is_none(){
     if self.kolonisation.reparaturen.keys().any(|(planet,_)|*planet==p.id){"Wartet auf laufende Reparatur".into()}else{self.warte_auf_kosten(pid,&self.regeln.kosten_gebaeude(a.gebaeude,a.stufe),a.topf)}
    }else{String::new()};
    jobs.push(json!({"key":a.gebaeude.name(),"stufe":a.stufe,"aktiv":a.fertig.is_some(),"ende":a.fertig,"grund":grund,"manuell_pausiert":self.auftrag_pausiert("bau",p.id)}));
   },
   "forschung"=>{
    let paused=self.auftrag_pausiert("forschung",sid as u32);
    if let Some(a)=&s.forschung_aktiv{jobs.push(json!({"key":a.forschung.name(),"stufe":a.stufe,"aktiv":true,"rest":a.fp_rest,"manuell_pausiert":paused,"grund":if paused{"Forschung manuell pausiert"}else if self.fp_rate(sid)>0{""}else{"Keine Forschungsleistung: Energie, Personal und Labor prüfen"}}));}
    let mut level=s.forschung;
    if let Some(a)=&s.forschung_aktiv{level[a.forschung.idx()]+=1;}
    for (i,(f,planet,topf)) in s.forschung_schlange.iter().enumerate(){
     level[f.idx()]+=1;
     let source=if self.planeten.get(*planet as usize).is_some_and(|p|p.besitzer==sid){*planet as usize}else{s.heimat as usize};
     let rule=self.regeln.forsch(*f);
     let grund=if paused{"Forschung manuell pausiert".into()}else if s.forschung_aktiv.is_some()||i>0{"Wartet auf vorherige Forschung".into()}else if self.planeten[source].gebaeude[Gebaeude::Labor.idx()]<rule.labor{format!("Benötigt Labor {} auf {}",rule.labor,self.planeten[source].koord)}else if s.stufe<rule.ab_stufe{format!("Benötigt Zivilisationsstufe {}",rule.ab_stufe)}else{self.warte_auf_kosten(source,&self.regeln.kosten_forschung(*f,level[f.idx()]),*topf)};
     jobs.push(json!({"key":f.name(),"stufe":level[f.idx()],"aktiv":false,"manuell_pausiert":paused,"grund":grund}));
    }
   },
   "werft"|"bauteile"=>{let idx=usize::from(art=="bauteile");for(i,a)in p.fertigung[idx].iter().enumerate(){let key=match a.produkt{Produkt::Einheit(e)=>e.name(),Produkt::Bauteil(g)=>g.name()};jobs.push(json!({"key":key,"anzahl":a.rest,"aktiv":i==0,"ende":if i==0{Some(p.fertigung_naechste[idx])}else{None},"grund":if i==0{""}else{"Wartet auf vorherigen Auftrag"}}));}},
   _=>{}
  }
  if self.ausbau.industrie_version>0&&matches!(art,"werft"|"bauteile"){let n=usize::from(art=="bauteile");if let Some(lots)=self.ausbau.parallel.get(&crate::parallel::key(pid,n)){for(i,job)in jobs.iter_mut().enumerate(){let active=lots.iter().filter(|b|b.auftrag==i).collect::<Vec<_>>();job["aktiv"]=json!(!active.is_empty());job["parallel"]=json!(active.iter().map(|b|b.anzahl).sum::<i64>());job["plaetze_je_stueck"]=json!(crate::parallel::platzbedarf(p.fertigung[n][i].produkt));job["ende"]=json!(active.iter().map(|b|b.ende).min());job["grund"]=json!(if active.is_empty(){"Wartet auf freie Fertigungsplätze"}else{"Fertigt gleichzeitig auf belegten Plätzen"});}}}
  for (index,job) in jobs.iter_mut().enumerate() {
   let eligible=match art {
    "bau"=>{let a=&p.bauschleife[index];a.fertig.is_none_or(|t|(t-self.zeit).max(0)*4>a.dauer)},
    "forschung"=>if index==0 && s.forschung_aktiv.is_some(){let a=s.forschung_aktiv.as_ref().unwrap();a.fp_rest*4>self.regeln.fp_forschung(a.forschung,a.stufe)}else{true},
    _=>{let n=usize::from(art=="bauteile");let a=&p.fertigung[n][index];index>0||(p.fertigung_naechste[n]-self.zeit).max(0)*4>a.dauer}
   };
   let eligible=if self.ausbau.industrie_version>0&&matches!(art,"werft"|"bauteile"){let n=usize::from(art=="bauteile");!self.ausbau.parallel.get(&crate::parallel::key(pid,n)).is_some_and(|lots|lots.iter().any(|b|b.auftrag==index&&(b.ende-self.zeit).max(0)*4<=b.dauer))}else{eligible};
   job["erstattung_prozent"]=json!(if eligible{100}else{0});
   if self.ausbau.industrie_version>0&&matches!(art,"werft"|"bauteile"){let n=usize::from(art=="bauteile");let charged=self.ausbau.parallel.get(&crate::parallel::key(pid,n)).map(|lots|lots.iter().filter(|b|b.auftrag==index&&(b.ende-self.zeit).max(0)*4<=b.dauer).map(|b|b.anzahl).sum::<i64>()).unwrap_or(0);job["erstattbare_stueck"]=json!(p.fertigung[n][index].rest-charged);}
   job["stornoregel"]=json!("Vor 75 % Fertigstellung: bezahlte Rohstoffe vollständig zurück; ab 75 % keine Erstattung des laufenden Stücks. Unbegonnene Stücke werden erstattet.");
  }
  // Kennung nur über Reihenfolge und Inhalt, nicht über laufende Größen (Ende, Rest, Erstattung): bei angehaltener
  // Schlange lief `ende` mit der Spielzeit weiter, jeder Pfeil- und ✕-Tipp scheiterte (Nachprüfung 25.10.2026).
  let mut identity=jobs.clone();for job in &mut identity{let o=job.as_object_mut().unwrap();for f in ["grund","ende","rest","anzahl","parallel","erstattung_prozent","erstattbare_stueck"]{o.remove(f);}}
  let hash=Sha256::digest(serde_json::to_vec(&identity).unwrap());let kennung:String=hash.iter().map(|b|format!("{b:02x}")).collect();
  json!({"art":art,"planet":p.koord.to_string(),"kennung":kennung,"jobs":jobs,"max":if art=="forschung"{self.regeln.wirtschaft.forschung_warteschlange+1}else{self.regeln.wirtschaft.warteschlange}})
 }
 pub fn warteschlange_aendern(&mut self,sid:SpielerId,rolle:Rolle,art:&str,k:Koord,index:usize,ziel:Option<usize>,loeschen:bool,kennung:&str)->Result<String,String>{
  if !["bau","forschung","werft","bauteile"].contains(&art){return Err("Unbekannte Warteschlange".into());}
  if rolle==Rolle::Feldherr&&!matches!(art,"werft"|"bauteile"){return Err("Bau und Forschung verwaltet der Verwalter".into());}
  let pid=self.eigener_planet(sid,k)?;let state=self.warteschlange_sicht(sid,art,pid);
  if state["kennung"].as_str()!=Some(kennung){return Err("Die Warteschlange hat sich inzwischen geändert – bitte in der aktualisierten Liste noch einmal tippen.".into());}
  let jobs=state["jobs"].as_array().unwrap();if index>=jobs.len(){return Err("Auftrag existiert nicht mehr".into());}
  if loeschen&&self.ausbau.industrie_version>0&&matches!(art,"werft"|"bauteile"){self.parallel_storno(pid,usize::from(art=="bauteile"),index);return Ok("Fertigungsauftrag abgebrochen; jedes laufende Stück vor 75 % und alle wartenden Stücke erstattet".into());}
  let active=jobs.first().is_some_and(|j|j["aktiv"]==true);
  if !loeschen {let to=ziel.filter(|n|*n<jobs.len()).ok_or("Zielposition fehlt")?;
   if jobs[index]["aktiv"]==true||jobs[to]["aktiv"]==true{return Err("Laufender Auftrag bleibt an erster Stelle; nur wartende Aufträge verschieben".into());}
   match art {
    "bau"=>{let q=&mut self.planeten[pid].bauschleife;let item=q.remove(index);q.insert(to,item);let mut levels=self.planeten[pid].gebaeude;for a in &mut self.planeten[pid].bauschleife{levels[a.gebaeude.idx()]+=1;a.stufe=levels[a.gebaeude.idx()];}},
    "forschung"=>{let off=usize::from(active);let q=&mut self.spieler[sid as usize].forschung_schlange;let item=q.remove(index-off);q.insert(to-off,item);},
    _=>{let n=usize::from(art=="bauteile");if let Some(lots)=self.ausbau.parallel.get_mut(&crate::parallel::key(pid,n)){for b in lots{if b.auftrag==index{b.auftrag=to;}else if index<to&&b.auftrag>index&&b.auftrag<=to{b.auftrag-=1;}else if to<index&&b.auftrag>=to&&b.auftrag<index{b.auftrag+=1;}}}let q=&mut self.planeten[pid].fertigung[n];let item=q.remove(index);q.insert(to,item);if let Some(paid)=self.ausbau.fertigung_zahlung.get_mut(&format!("{pid}:{n}")){if index<paid.len()&&to<paid.len(){let item=paid.remove(index);paid.insert(to,item);}}}
   }
   return Ok("Wartende Aufträge neu geordnet; laufender Fortschritt bleibt erhalten".into());
  }
  match art {
   "bau"=>{let a=self.planeten[pid].bauschleife.remove(index);
    if a.fertig.is_some_and(|end|(end-self.zeit).max(0)*4>a.dauer){let cost=self.regeln.kosten_gebaeude(a.gebaeude,a.stufe);self.erstattung(pid,cost,a.topf);}
    if a.fertig.is_some(){let id=pid as PlanetId;self.ereignisse.retain(|e|!matches!(e.art,EreignisArt::BauFertig{planet}if planet==id));}
    let mut levels=self.planeten[pid].gebaeude;for a in &mut self.planeten[pid].bauschleife{levels[a.gebaeude.idx()]+=1;a.stufe=levels[a.gebaeude.idx()];}self.bau_starten(pid);},
   "forschung"=>{if active&&index==0{
     let a=self.spieler[sid as usize].forschung_aktiv.take().unwrap();
     let paid=self.ausbau.forschung_zahlung.remove(&sid);
     if a.fp_rest*4>self.regeln.fp_forschung(a.forschung,a.stufe){
      let (source,topf,cost)=paid.unwrap_or((self.spieler[sid as usize].heimat,None,self.regeln.kosten_forschung(a.forschung,a.stufe)));
      let source=if self.planeten[source as usize].besitzer==sid && self.planet_besiedelt(source as usize){source as usize}else{self.spieler[sid as usize].heimat as usize};
      self.erstattung(source,cost,topf);
     }
    }else{self.spieler[sid as usize].forschung_schlange.remove(index-usize::from(active));}},
   _=>{let n=usize::from(art=="bauteile");let a=self.planeten[pid].fertigung[n].remove(index);let started_costs=index==0 && (self.planeten[pid].fertigung_naechste[n]-self.zeit).max(0)*4<=a.dauer;let unstarted=a.rest-i64::from(started_costs);let r=&self.regeln;
    let paid=self.ausbau.fertigung_zahlung.get_mut(&format!("{pid}:{n}")).and_then(|q|if index<q.len(){Some(q.remove(index))}else{None});
    let cost=paid.map(|(_,cost)|cost).unwrap_or_else(||match a.produkt{Produkt::Einheit(e)=>r.kosten_einheit(e,self.spieler[sid as usize].volk),Produkt::Bauteil(g)=>r.kosten_bauteil(g).unwrap()});let crew=match a.produkt{Produkt::Einheit(e)=>r.einh(e).besatzung*M, _=>0};
    self.erstattung(pid,std::array::from_fn(|g|cost[g]*unstarted.max(0)),paid.and_then(|(topf,_)|topf));if n==0 && a.produkt==Produkt::Bauteil(Gut::Habitatmodul){self.planeten[pid].einheiten[Einheit::Recycler.idx()]+=unstarted.max(0);}
    self.planeten[pid].bevoelkerung+=crew*a.rest;self.crew_erstatten(pid,a.produkt,crew*a.rest);
    if index==0 {let id=pid as PlanetId;self.ereignisse.retain(|e|!matches!(e.art,EreignisArt::FertigungFertig{planet,schleife}if planet==id&&schleife as usize==n));
     let end=self.planeten[pid].fertigung[n].first().map(|a|self.zeit+a.dauer);self.planeten[pid].fertigung_naechste[n]=end.unwrap_or(-1);if let Some(t)=end{self.plane(t,EreignisArt::FertigungFertig{planet:id,schleife:n as u8});}}
    match a.produkt{Produkt::Einheit(e)=>self.produktionsserie_stoppen(pid,"einheit",e.name()),Produkt::Bauteil(g)=>self.produktionsserie_stoppen(pid,"bauteil",g.name())};
    self.raten_neu(pid);
   }
  }
  Ok("Auftrag entfernt. Vor 75 % wurden bezahlte Rohstoffe erstattet; ab 75 % bleibt das laufende Stück bezahlt. Unbegonnene Stücke und ungenutzte Besatzung sind zurück.".into())
 }
}
