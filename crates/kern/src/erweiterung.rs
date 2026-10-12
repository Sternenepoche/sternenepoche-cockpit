//! Explicit late-game projects and deterministic, delayed silo warfare.
use crate::{regeln::preis_array, typen::*, welt::*};

impl Welt {
    pub fn raketen_belegt(&self, pid: usize) -> i64 {
        let p=&self.planeten[pid];
        p.raketen.iter().sum::<i64>() + self.ereignisse.iter().filter_map(|e|match e.art {
            EreignisArt::RaketenFertig{planet,besitzer,anzahl,..} if planet==p.id && besitzer==p.besitzer=>Some(anzahl),_=>None
        }).sum::<i64>()
    }
    pub fn raketen_kosten(&self,art:u8,anzahl:i64)->Result<[i64;GUETER],String> {
        if art>1 || anzahl<=0 || anzahl>1_000_000 {return Err("Raketenart oder Anzahl ungültig".into());}
        let mut costs=preis_array(&self.regeln.zusatz.raketen_kosten[art as usize]);
        for c in &mut costs {*c=c.checked_mul(anzahl).ok_or("Raketenkosten zu groß")?;}
        Ok(costs)
    }
    pub fn raketen_bauen(&mut self,sid:SpielerId,rolle:Rolle,k:Koord,art:&str,anzahl:i64)->Result<String,String> {
        let art=match art {"abfang"=>0,"interplanetar"=>1,_=>return Err("Raketenart muss abfang oder interplanetar sein".into())};
        let pid=self.eigener_planet(sid,k)?;
        let costs=self.raketen_kosten(art,anzahl)?;
        let p=&self.planeten[pid];
        let kapazitaet=p.gebaeude[Gebaeude::Raketensilo.idx()] as i64 * self.regeln.zusatz.silo_plaetze_je_stufe;
        if self.spieler[sid as usize].stufe<self.regeln.geb(Gebaeude::Raketensilo).ab_stufe || kapazitaet==0 {return Err("Raketensilo und Zivilisationsstufe III benötigt".into());}
        if self.raketen_belegt(pid)+anzahl>kapazitaet {return Err(format!("Silokapazität {kapazitaet} inklusive Bauaufträge überschritten"));}
        if p.stabilitaet<milli(self.regeln.stabilitaet.unruhen_unter){return Err("Raketenbau bei Unruhen nicht möglich".into());}
        let duration=self.regeln.zusatz.raketen_bauzeit_sekunden.checked_mul(anzahl).ok_or("Bauzeit zu groß")?;
        // Jobs share one silo production line; reservations include all paid jobs.
        let last=self.ereignisse.iter().filter(|e|matches!(e.art,EreignisArt::RaketenFertig{planet,besitzer,..} if planet==p.id && besitzer==sid)).map(|e|e.zeit).max().unwrap_or(self.zeit).max(self.zeit);
        let done=last.checked_add(duration).ok_or("Fertigstellungszeit zu groß")?;
        self.abrechnen(pid);
        let topf=Self::topf_fuer(rolle,Topf::Militaer);
        self.zahlbar(pid,&costs,topf)?;
        self.zahlen(pid,&costs,topf);
        self.plane(done,EreignisArt::RaketenFertig{planet:pid as PlanetId,besitzer:sid,art,anzahl});
        Ok(format!("{anzahl} Raketen auf {k} im Bau; fertig {}",zeittext(done)))
    }
    pub fn raketen_fertig(&mut self,pid:usize,besitzer:SpielerId,art:u8,anzahl:i64) {
        let Some(p)=self.planeten.get_mut(pid) else{return;};
        if p.besitzer!=besitzer || art>1 || anzahl<=0{return;}
        let capacity=p.gebaeude[Gebaeude::Raketensilo.idx()] as i64*self.regeln.zusatz.silo_plaetze_je_stufe;
        let admitted=anzahl.min((capacity-p.raketen.iter().sum::<i64>()).max(0));
        p.raketen[art as usize]+=admitted;
        let k=p.koord;
        self.vorfall(besitzer,"raketen",format!("{admitted} Raketen auf {k} fertig"));
        self.wecke(besitzer,Rolle::Feldherr,"Raketen fertig",false);
    }
    pub fn raketen_starten(&mut self,sid:SpielerId,start:Koord,ziel:Koord,anzahl:i64,zieltyp:Einheit)->Result<String,String> {
        if self.aufklaerungsregeln() && self.planetenwissen(sid,ziel)["bekannt"]!=true {
            return Err("Ziel zuerst mit einer eigenen Planetensonde aufklären".into());
        }
        let pid=self.eigener_planet(sid,start)?;
        if anzahl<=0 || anzahl>self.planeten[pid].raketen[1] {return Err("Nicht genügend fertige Interplanetarraketen oder ungültige Anzahl".into());}
        if zieltyp.ist_schiff(){return Err("Raketen treffen nur ausgewählte Verteidigungsanlagen".into());}
        let zp=*self.belegung.get(&ziel).ok_or("Zielplanet existiert nicht")? as usize;
        let defender=self.planeten[zp].besitzer;
        if defender==sid{return Err("Eigene Planeten können nicht mit Raketen angegriffen werden".into());}
        let z=&self.regeln.zusatz;
        let distance=start.system.abs_diff(ziel.system) as i64;
        let reach=self.planeten[pid].gebaeude[Gebaeude::Raketensilo.idx()] as i64*z.raketen_reichweite_je_silo as i64;
        if start.sektor!=ziel.sektor || distance>reach || reach==0{return Err("Raketenreichweite überschritten; nur derselbe Sektor ist erreichbar".into());}
        let target=&self.spieler[defender as usize];
        if self.zeit<target.schutz_bis{return Err("Ziel steht unter Anfängerschutz".into());}
        let weak=self.regeln.diplomatie.schwachenschutz_anteil;
        if weak>0.0 && target.punkte.gesamt()<mal(self.spieler[sid as usize].punkte.gesamt(),weak) && !target.angegriffen.contains(&sid){return Err("Ziel steht unter Schwachenschutz".into());}
        let arrival=self.zeit+z.raketen_flug_min_sekunden+distance*z.raketen_flug_je_system_sekunden;
        let cost=self.regeln.wert(&self.raketen_kosten(1,anzahl)?);
        self.planeten[pid].raketen[1]-=anzahl;
        self.spieler[sid as usize].statistik.verluste+=cost;
        self.spieler[sid as usize].statistik.angriffe+=1;
        self.spieler[sid as usize].schutz_bis=self.spieler[sid as usize].schutz_bis.min(self.zeit);
        self.bruch_pruefen(sid,defender);
        if !self.spieler[sid as usize].angegriffen.contains(&defender){self.spieler[sid as usize].angegriffen.push(defender);}
        self.plane(arrival,EreignisArt::RaketenAnkunft{von:sid,ziel,anzahl,zieltyp});
        self.vorfall(defender,"raketen",format!("{anzahl} Interplanetarraketen von {} erreichen {ziel} um {}",self.spieler[sid as usize].name,zeittext(arrival)));
        self.wecke(defender,Rolle::Feldherr,"Interplanetarraketen im Anflug",true);
        Ok(format!("{anzahl} Interplanetarraketen nach {ziel}, Ankunft {}",zeittext(arrival)))
    }
    pub fn raketen_ankunft(&mut self,von:SpielerId,ziel:Koord,anzahl:i64,zieltyp:Einheit) {
        let Some(&zp)=self.belegung.get(&ziel) else{return;};let zp=zp as usize;
        let defender=self.planeten[zp].besitzer;
        // A captured colony can change allegiance while the salvo is in flight.
        if defender==von || self.zeit<self.spieler[defender as usize].schutz_bis {
            self.vorfall(von,"raketen",format!("Raketensalve auf {ziel} ohne Schaden beendet: Ziel gehört nun dir oder steht unter Schutz"));return;
        }
        self.bruch_pruefen(von,defender);
        let r=self.regeln.clone();
        let intercepted=anzahl.min(self.planeten[zp].raketen[0]);
        self.planeten[zp].raketen[0]-=intercepted;
        let attack=mal(r.zusatz.raketen_schaden,1.0+r.kampf.tech_je_stufe*self.spieler[von as usize].forschung[Forschung::Waffentechnik.idx()] as f64);
        let owner=&self.spieler[defender as usize];
        let armor=mal(r.einh(zieltyp).struktur,(1.0+r.kampf.tech_je_stufe*owner.forschung[Forschung::Panzerung.idx()] as f64)*r.volk(owner.volk).panzerung).max(1);
        let destroyed=(((anzahl-intercepted) as i128*attack as i128)/armor as i128).min(self.planeten[zp].einheiten[zieltyp.idx()] as i128) as i64;
        let loss=r.wert(&r.kosten_einheit(zieltyp,owner.volk))*destroyed + r.wert(&preis_array(&r.zusatz.raketen_kosten[0]))*intercepted;
        self.planeten[zp].einheiten[zieltyp.idx()]-=destroyed;
        self.spieler[defender as usize].statistik.verluste+=loss;
        self.spieler[defender as usize].statistik.verteidigungen+=1;
        let result=format!("Raketenschlag auf {ziel}: {anzahl} gestartet, {intercepted} abgefangen, {destroyed} {zieltyp} zerstört; keine Beute, kein Wiederaufbau");
        self.vorfall(von,"raketen",result.clone());self.vorfall(defender,"raketen",result);
        self.wecke(defender,Rolle::Feldherr,"Raketenschlag",true);
        self.raten_neu(zp);self.punkte_neu();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Regelwerk;
    use serde_json::json;
    fn setup()->(Welt,usize,usize) {
        let mut w=Welt::neu(Regelwerk::laden(include_str!("../../../regeln/regelwerk.ron")).unwrap(),42,8).unwrap();
        for s in &mut w.spieler {s.ki=false;s.stufe=5;s.schutz_bis=0;s.volk=Volk::Aurelianer;}
        let a=w.spieler[0].heimat as usize;
        let b=w.planeten.iter().position(|p|p.besitzer!=0 && p.koord.sektor==w.planeten[a].koord.sektor).unwrap();
        for p in &mut w.planeten {p.bestand=[1_000_000*M;GUETER];p.gebaeude[Gebaeude::Raketensilo.idx()]=12;}
        w.fenster_vorbereiten();(w,a,b)
    }
    #[test]
    fn silo_reserves_paid_capacity_and_completes_delayed_in_order() {
        let (mut w,a,_)=setup();w.planeten[a].gebaeude[Gebaeude::Raketensilo.idx()]=1;
        let k=w.planeten[a].koord;let old=w.planeten[a].bestand[0];
        assert!(w.raketen_bauen(0,Rolle::Alle,k,"abfang",6).is_ok());
        assert_eq!(w.planeten[a].bestand[0],old-12_000*M);assert_eq!(w.planeten[a].raketen,[0,0]);assert_eq!(w.raketen_belegt(a),6);
        assert!(w.raketen_bauen(0,Rolle::Alle,k,"interplanetar",5).is_err());
        assert!(w.abreissen(0,k,Gebaeude::Raketensilo).is_err());
        assert!(w.raketen_bauen(0,Rolle::Alle,k,"abfang",4).is_ok());
        w.schritt();assert_eq!(w.planeten[a].raketen,[0,0]);w.schritt();assert_eq!(w.planeten[a].raketen,[6,0]);
        w.schritt();assert_eq!(w.planeten[a].raketen,[6,0]);w.schritt();assert_eq!(w.planeten[a].raketen,[10,0]);
    }
    #[test]
    fn rejected_builds_do_not_pay_or_create_events() {
        let (mut w,a,b)=setup();let k=w.planeten[a].koord;let foreign=w.planeten[b].koord;
        let before=w.planeten[a].bestand;let events=w.ereignisse.len();
        assert!(w.raketen_bauen(0,Rolle::Alle,k,"unknown",1).is_err());
        assert!(w.raketen_bauen(0,Rolle::Alle,k,"abfang",-1).is_err());
        assert!(w.raketen_bauen(0,Rolle::Alle,foreign,"abfang",1).is_err());
        assert_eq!(before,w.planeten[a].bestand);assert_eq!(events,w.ereignisse.len());
        w.planeten[a].bestand=[0;GUETER];assert!(w.raketen_bauen(0,Rolle::Alle,k,"abfang",1).is_err());
        assert_eq!(w.raketen_belegt(a),0);
    }
    #[test]
    fn salvos_are_delayed_intercepted_and_do_not_damage_ships_or_people() {
        let (mut w,a,b)=setup();w.planeten[a].raketen[1]=3;w.planeten[b].raketen[0]=2;
        w.planeten[b].einheiten[Einheit::Raketenwerfer.idx()]=100;
        w.planeten[b].einheiten[Einheit::KleinerTransporter.idx()]=7;
        let start=w.planeten[a].koord;let target=w.planeten[b].koord;
        assert!(w.raketen_starten(0,start,target,3,Einheit::Raketenwerfer).is_ok());
        assert_eq!(w.planeten[b].einheiten[Einheit::Raketenwerfer.idx()],100);
        let when=w.ereignisse.iter().find(|e|matches!(e.art,EreignisArt::RaketenAnkunft{..})).unwrap().zeit;
        assert!(when>w.regeln.fenster());
        while w.zeit<when {w.schritt();}
        assert_eq!(w.planeten[a].raketen[1],0);assert_eq!(w.planeten[b].raketen[0],0);
        assert!(w.planeten[b].einheiten[Einheit::Raketenwerfer.idx()]<100);
        assert_eq!(w.planeten[b].einheiten[Einheit::KleinerTransporter.idx()],7);
        assert!(!w.truemmer.contains_key(&target));
        assert!(w.spieler[w.planeten[b].besitzer as usize].statistik.verluste>0);
    }
    #[test]
    fn full_interception_and_armor_reduce_losses() {
        let (mut w,a,b)=setup();let target=w.planeten[b].koord;let owner=w.planeten[b].besitzer as usize;
        w.planeten[b].raketen[0]=10;w.planeten[b].einheiten[Einheit::Raketenwerfer.idx()]=100;
        w.raketen_ankunft(0,target,5,Einheit::Raketenwerfer);assert_eq!(w.planeten[b].einheiten[Einheit::Raketenwerfer.idx()],100);
        w.planeten[b].raketen[0]=0;let mut stronger=w.clone();stronger.spieler[owner].forschung[Forschung::Panzerung.idx()]=20;
        w.raketen_ankunft(0,target,1,Einheit::Raketenwerfer);stronger.raketen_ankunft(0,target,1,Einheit::Raketenwerfer);
        assert!(stronger.planeten[b].einheiten[Einheit::Raketenwerfer.idx()]>w.planeten[b].einheiten[Einheit::Raketenwerfer.idx()]);
        assert_eq!(w.planeten[a].raketen,[0,0]);
    }
    #[test]
    fn protected_or_invalid_target_cannot_consume_ammunition() {
        let (mut w,a,b)=setup();w.planeten[a].raketen[1]=10;let start=w.planeten[a].koord;let target=w.planeten[b].koord;let owner=w.planeten[b].besitzer as usize;
        w.spieler[owner].schutz_bis=TAG;
        assert!(w.raketen_starten(0,start,target,1,Einheit::Raketenwerfer).is_err());w.spieler[owner].schutz_bis=0;
        assert!(w.raketen_starten(0,start,target,1,Einheit::LeichterJaeger).is_err());
        assert!(w.raketen_starten(0,start,start,1,Einheit::Raketenwerfer).is_err());
        assert!(w.raketen_starten(0,start,target,11,Einheit::Raketenwerfer).is_err());assert_eq!(w.planeten[a].raketen[1],10);
    }
    #[test]
    fn pending_missiles_and_flights_survive_checkpoint_exactly() {
        let (mut w,a,b)=setup();let start=w.planeten[a].koord;let target=w.planeten[b].koord;w.planeten[a].raketen[1]=2;
        w.raketen_bauen(0,Rolle::Alle,start,"abfang",5).unwrap();w.raketen_starten(0,start,target,1,Einheit::Raketenwerfer).unwrap();
        let mut resumed=Welt::aus_bytes(&w.zu_bytes()).unwrap();for _ in 0..12{w.schritt();resumed.schritt();}assert_eq!(w.hash(),resumed.hash());
    }
    #[test]
    fn capture_ownership_guard_discards_stale_production() {
        let (mut w,a,_)=setup();w.planeten[a].besitzer=1;w.raketen_fertig(a,0,1,10);assert_eq!(w.planeten[a].raketen,[0,0]);
    }
    #[test]
    fn projects_are_stage_five_once_per_planet_and_ring_cannot_overflow_fields() {
        let (mut w,a,_)=setup();let k=w.planeten[a].koord;w.spieler[0].stufe=4;
        assert!(w.bauen(0,Rolle::Alle,k,Gebaeude::Orbitalring).is_err());w.spieler[0].stufe=5;
        assert!(w.bauen(0,Rolle::Alle,k,Gebaeude::Orbitalring).is_ok());assert!(w.bauen(0,Rolle::Alle,k,Gebaeude::Orbitalring).is_err());
        w.planeten[a].bauschleife.clear();let fields=w.felder_gesamt(a);let housing=w.planeten[a].wohnraum;
        w.planeten[a].gebaeude[Gebaeude::Orbitalring.idx()]=1;w.raten_neu(a);
        assert_eq!(w.felder_gesamt(a),fields+w.regeln.zusatz.orbitalring_felder);assert_eq!(w.planeten[a].wohnraum,housing+w.regeln.zusatz.orbitalring_wohnraum*M);
        w.planeten[a].felder=1;assert!(w.abreissen(0,k,Gebaeude::Orbitalring).is_err());
    }
    #[test]
    fn archive_and_network_apply_real_local_production_bonuses() {
        let (mut w,a,_)=setup();w.planeten[a].bevoelkerung=1_000_000*M;w.planeten[a].gebaeude[Gebaeude::Labor.idx()]=1;w.planeten[a].gebaeude[Gebaeude::Solarkraftwerk.idx()]=10;w.raten_neu(a);
        let fp=w.planeten[a].fp_rate;let energy=w.planeten[a].energie_erzeugung;assert!(fp>0);
        w.planeten[a].gebaeude[Gebaeude::Forschungsarchiv.idx()]=1;w.planeten[a].gebaeude[Gebaeude::Versorgungsnetz.idx()]=1;w.raten_neu(a);
        assert_eq!(w.planeten[a].fp_rate,mal(fp,1.25));assert_eq!(w.planeten[a].energie_erzeugung,mal(energy,1.25));
    }
    #[test]
    fn salvos_and_ammunition_are_private_and_roles_are_enforced() {
        let (mut w,a,b)=setup();let start=w.planeten[a].koord;let target=w.planeten[b].koord;let owner=w.planeten[b].besitzer;
        w.planeten[a].raketen[1]=1;
        assert!(!w.handeln(0,Rolle::Diplomat,&json!({"typ":"raketen_starten","start":start,"ziel":target,"anzahl":1,"zieltyp":"raketenwerfer"})).0);
        assert!(w.handeln(0,Rolle::Feldherr,&json!({"typ":"raketen_starten","start":start,"ziel":target,"anzahl":1,"zieltyp":"raketenwerfer"})).0);
        assert_eq!(w.sicht(owner,Rolle::Alle)["raketensalven"].as_array().unwrap().len(),1);
        let stranger=(0..w.spieler.len() as u16).find(|s|*s!=0 && *s!=owner).unwrap();
        assert_eq!(w.sicht(stranger,Rolle::Alle)["raketensalven"].as_array().unwrap().len(),0);
    }
}
