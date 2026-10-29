//! Read-only player tools. Scenarios use only explicitly entered forces and public rules.
use crate::{kampf::Gruppe, typen::*, welt::*, Regelwerk, Welt};
use serde_json::{json, Value};

pub const SIMULATOR_MAX_EINHEITEN: i64 = 20_000;
pub const SIMULATOR_MAX_LAEUFE: u32 = 64;

pub(crate) fn simulator_grenzen(a: &[i64], d: &[i64]) -> Result<(), String> {
    // Validate before summing: even an i64::MAX input cannot overflow the total.
    if a.iter().chain(d).any(|n| !(0..=SIMULATOR_MAX_EINHEITEN).contains(n)) {
        return Err("Einheitenzahl: ganze Zahl zwischen 0 und 20.000".into());
    }
    let total: i64 = a.iter().chain(d).sum();
    if !(1..=SIMULATOR_MAX_EINHEITEN).contains(&total) {
        return Err("Für die Simulation insgesamt 1 bis 20.000 Einheiten eingeben".into());
    }
    Ok(())
}

fn partei(r: &Regelwerk, input: &Value, defender: bool) -> Result<(Gruppe, Volk), String> {
    let volk = input.get("volk").and_then(Value::as_str).and_then(Volk::aus_name)
        .ok_or("Für beide Seiten ein gültiges Volk wählen")?;
    let units = input.get("einheiten").and_then(Value::as_object).ok_or("Einheiten fehlen")?;
    let mut counts = [0i64; EINHEITEN];
    for (name, amount) in units {
        let e = Einheit::aus_name(name).filter(|e| (defender || e.ist_schiff()) && r.einheiten.contains_key(e))
            .ok_or_else(||format!("{name} ist für diese Seite keine gültige Einheit"))?;
        let n = amount.as_i64().filter(|n| (0..=SIMULATOR_MAX_EINHEITEN).contains(n)).ok_or("Einheitenzahl: ganze Zahl zwischen 0 und 20.000")?;
        counts[e.idx()] = n;
    }
    let tech = |name: &str| -> Result<f64,String> {
        let n = input.get("technik").and_then(|v|v.get(name)).and_then(Value::as_u64)
            .filter(|n|*n<=crate::regeln::MAX_FORSCHUNG as u64).ok_or_else(||format!("{name}: Stufe 0 bis {} angeben",crate::regeln::MAX_FORSCHUNG))?;
        Ok(1.0 + r.kampf.tech_je_stufe*n as f64)
    };
    if !r.flottendoktrin.is_empty()&&[Einheit::Planetenschild,Einheit::PlanetenschildII,Einheit::PlanetenschildIII].iter().map(|e|counts[e.idx()]).sum::<i64>()>1{return Err("Pro verteidigtem Planeten höchstens eine Planetenschildstufe".into());}
    let vr = r.volk(volk);
    let (fa, fs, fp) = (tech("waffentechnik")?*vr.waffen, tech("schildtechnik")?, tech("panzerung")?*vr.panzerung);
    let mut g = Gruppe::mit_werten(r, if defender {1} else {0}, counts, fa, fs, fp);
    g.volk_einheiten(r, volk, fa, fs, fp);
    Ok((g,volk))
}

impl Welt {
    pub fn einheit_daten(&self,sid:SpielerId,e:Einheit,volk:Volk)->Value {
        let r=&self.regeln;let s=&self.spieler[sid as usize];let er=r.einh(e);let vr=r.volk(volk);
        let t=|f:Forschung|1.0+r.kampf.tech_je_stufe*s.forschung[f.idx()] as f64;
        let cost=r.kosten_einheit(e,volk);
        use crate::planer::modell::ErklaerterWert;
        let ve=r.volk_einheit(volk,e).copied();
        let eigen=|w:ErklaerterWert,f:Option<f64>|match f {Some(f) if f!=1.0=>w.faktor("eigener_volkswert",f),_=>w};
        let angriff=eigen(ErklaerterWert::neu(er.angriff,"angriff").faktor("waffentechnik_und_volk",t(Forschung::Waffentechnik)*vr.waffen),ve.map(|v|v.angriff));
        let schild=eigen(ErklaerterWert::neu(er.schild,"schild").faktor("schildtechnik",t(Forschung::Schildtechnik)),ve.map(|v|v.schild));
        let struktur=eigen(ErklaerterWert::neu(er.struktur,"struktur").faktor("panzerung_und_volk",t(Forschung::Panzerung)*vr.panzerung),ve.map(|v|v.struktur));
        let ladung=ErklaerterWert::neu(r.ladung_milli(e,volk),"tausendstel_fracht").faktor("volk_und_logistik",vr.ladung*(1.0+r.flug.logistik_bonus*s.forschung[Forschung::Logistik.idx()] as f64));

        // Technik v1: wirksamer Antrieb und Verbrauch nach Forschung; ohne Technik v1 die Werte der Einheitenregel.
        let (antrieb,_,verbrauch)=if e.ist_schiff(){(r.antrieb_wirksam(e,&s.forschung).0,0,self.verbrauch_volk(sid,e,volk))}else{(er.antrieb,er.tempo,er.verbrauch)};
        let mut daten=json!({"volk":volk.name(),"basis":{"angriff":er.angriff,"schild":er.schild,"struktur":er.struktur,"tempo":er.tempo,"ladung":er.ladung},
            "angriff":angriff.wert,"schild":schild.wert,
            "struktur":struktur.wert,"tempo":if e.ist_schiff(){self.tempo_volk(sid,e,volk)}else{0},
            "ladung":ganz(ladung.wert),"berechnungen":{"angriff":angriff,"schild":schild,"struktur":struktur,"ladung":ladung},
            "verbrauch":verbrauch,"besatzung":er.besatzung,"antrieb":antrieb,"taktik":r.flottendoktrin.get(&e),
            "personenplaetze":r.personenplaetze(e),"personenbeforderung_verfuegbar":self.ausbau.version>0,
            "unterhalt_je_tag":if e.ist_schiff(){mal(r.wert(&cost),r.wirtschaft.unterhalt_schiffe_je_tag) as f64/M as f64}else{0.0},
            "technik":{"waffentechnik":s.forschung[Forschung::Waffentechnik.idx()],"schildtechnik":s.forschung[Forschung::Schildtechnik.idx()],"panzerung":s.forschung[Forschung::Panzerung.idx()],"logistik":s.forschung[Forschung::Logistik.idx()]}});
        if e.ist_schiff() && r.technik.as_ref().is_some_and(|t|t.antriebswechsel.contains_key(&e)) {
            daten["antriebswechsel"]=self.antriebsdaten(sid,e)["antriebswechsel"].clone();
        }
        daten
    }
    pub fn kampfszenario(&self, input: &Value) -> Result<Value,String> {
        let groups = |side: &str, defender: bool| -> Result<Vec<(Gruppe,Volk,u8)>,String> {
            let inputs: Vec<&Value> = match &input[side] {Value::Array(v)=>v.iter().collect(), v=>vec![v]};
            if inputs.is_empty() || inputs.len()>8 {return Err("Je Seite 1 bis 8 Gruppen".into());}
            inputs.into_iter().map(|v| {
                let (g,volk)=partei(&self.regeln,v,defender)?;
                let logistik=match v["technik"].get("logistik") {None=>0,Some(n)=>n.as_u64().filter(|n|*n<=crate::regeln::MAX_FORSCHUNG as u64).ok_or("Logistikstufe ungültig")? as u8};
                Ok((g,volk,logistik))
            }).collect()
        };
        let seed=match input.get("szenarioseed") {None=>crate::planer::kampf::SZENARIO_SEED,Some(v)=>v.as_u64().ok_or("Szenarioseed muss eine nichtnegative ganze Zahl sein")?};
        let bounded=|key:&str,max:u64|->Result<u64,String>{match input.get(key){None=>Ok(0),Some(v)=>v.as_u64().filter(|n|*n<=max).ok_or_else(||format!("{key} außerhalb der Grenze 0–{max}"))}};
        let stock=match input.get("bestand_milli") {None=>None,Some(v)=>{
            let a=v.as_array().filter(|a|a.len()==GUETER).ok_or("bestand_milli benötigt zehn Güter")?;
            let mut s=[0;GUETER];for (i,v) in a.iter().enumerate(){s[i]=v.as_i64().filter(|n|(0..=1_000_000_000_000).contains(n)).ok_or("Bestand außerhalb der Grenze")?;}Some(s)
        }};
        let mut result=crate::planer::kampf::berechnen(&self.regeln,&groups("angreifer",false)?,&groups("verteidiger",true)?,seed,
            stock,bounded("bunker",self.regeln.welt.max_gebaeudestufe as u64)? as u8,bounded("vorladung_milli",1_000_000_000_000)? as i64,bounded("bergung_promille",1000)? as u16)?;
        result["modus"]=json!("eingegebenes_szenario");result["grundlage"]=input.clone();
        result["kontext"]=serde_json::to_value(self.planer_regelkontext()).unwrap();
        Ok(result)
    }

    /// Fremde Flotten, die im Orbit eigener Planeten stehen und mitverteidigen: verbündete Halteflotten (auch Bodenhilfe mit
    /// Truppen) und gestrandete Flotten, die dem Besitzer beistehen (Einsatzprobe 25.10.2026: der Besitzer sah sie nirgends).
    pub fn verbuendete_im_orbit(&self, sid: SpielerId) -> Vec<Value> {
        self.flotten.values().filter_map(|f| {
            if f.besitzer == sid || f.zustand != Flottenzustand::ImOrbit || self.monde.flottenziel.contains_key(&f.id) { return None; }
            let pid = *self.belegung.get(&f.ziel)? as usize;
            if self.planeten[pid].besitzer != sid { return None; }
            let gestrandet = self.gestrandet(f.id);
            if !(self.verbuendet(sid, f.besitzer) && !f.mission.feindlich() || gestrandet && self.orbitflotte_hilft(f, pid, None)) { return None; }
            let truppen = self.ausbau.boden.flotten.get(&f.id).copied().unwrap_or_default();
            Some(json!({"flotte":f.id,"von":self.spieler[f.besitzer as usize].name,"ziel":f.ziel.to_string(),"mission":f.mission.name(),
                "bis_sekunden":if gestrandet || f.orbit_ende <= 0 { Value::Null } else { json!(f.orbit_ende) },"gestrandet":gestrandet,
                "schiffe":crate::sicht::einheiten(&f.schiffe, 0),"anzahl":f.schiffe.iter().sum::<i64>(),
                "truppen":truppen.iter().sum::<i64>(),
                "hinweis":"Verteidigt bei einem Angriff an deiner Seite; Bodentruppen an Bord stehen bei einer Invasion im Rückhalt."}))
        }).collect()
    }

    pub fn verbuendete_anfluege(&self, sid: SpielerId) -> Vec<Value> {
        self.flotten.values().filter(|f| f.besitzer!=sid && self.verbuendet(sid,f.besitzer)
            && f.zustand==Flottenzustand::Hinflug && !f.mission.feindlich()
            && self.belegung.get(&f.ziel).is_some_and(|pid|self.planeten[*pid as usize].besitzer==sid))
            .map(|f|json!({"flotte":f.id,"von":self.spieler[f.besitzer as usize].name,
                "ziel":f.ziel.to_string(),"ankunft_sekunden":f.ankunft,"mission":f.mission.name(),
                "beziehung":"verbuendet","schiffe":null,"schiffstypen":null,
                "hinweis":"Verbündeter Anflug auf deinen Planeten. Zusammensetzung wird nicht automatisch geteilt."})).collect()
    }
}

#[cfg(test)] mod tests {
    use super::*;
    #[test] fn scenario_validates_and_does_not_mutate_world() {
        let r=Regelwerk::laden(include_str!("../../../regeln/online-v1.ron")).unwrap();
        let w=Welt::neu(r,123,1).unwrap();
        let p=json!({"volk":"krath","einheiten":{"leichter_jaeger":10},"technik":{"waffentechnik":3,"schildtechnik":3,"panzerung":3}});
        let q=json!({"angreifer":p,"verteidiger":p});
        let before=bincode::serialize(&w).unwrap();let a=w.kampfszenario(&q).unwrap();
        assert_eq!(a,w.kampfszenario(&q).unwrap());assert_eq!(before,bincode::serialize(&w).unwrap());
        let mut bad=q;bad["angreifer"]["einheiten"]["leichter_jaeger"]=json!(20_001);
        assert!(w.kampfszenario(&bad).is_err());
        bad["angreifer"]["einheiten"]["leichter_jaeger"]=json!(-1);assert!(w.kampfszenario(&bad).is_err());
    }
    #[test] fn unit_sheets_use_real_modifiers_and_only_ships_pay_fleet_upkeep() {
        let r=Regelwerk::laden(include_str!("../../../regeln/online-v1.ron")).unwrap();let mut w=Welt::neu(r,123,1).unwrap();
        w.spieler[0].forschung[Forschung::Waffentechnik.idx()]=3;
        let sheet=w.einheit_daten(0,Einheit::LeichterJaeger,Volk::Krath);
        assert_eq!(sheet["angriff"],mal(w.regeln.einh(Einheit::LeichterJaeger).angriff,1.3*w.regeln.volk(Volk::Krath).waffen));
        assert!(sheet["unterhalt_je_tag"].as_f64().unwrap()>0.0);
        assert_eq!(w.einheit_daten(0,Einheit::Raketenwerfer,Volk::Krath)["unterhalt_je_tag"],0.0);
    }
    #[test] fn allied_approaches_do_not_expose_unrelated_routes_or_manifests() {
        let r=Regelwerk::laden(include_str!("../../../regeln/online-v1.ron")).unwrap();
        let mut w=Welt::neu(r,123,3).unwrap();
        w.spieler[0].allianz=Some(1);w.spieler[1].allianz=Some(1);
        let start=w.spieler[1].heimat;let target=w.planeten[w.spieler[0].heimat as usize].koord;
        let mut units=[0;SCHIFFE];units[Einheit::KleinerTransporter.idx()]=4;
        let f=Flotte{id:1,besitzer:1,start,start_koord:w.planeten[start as usize].koord,ziel:target,mission:Mission::Transport,
            schiffe:units,ladung:[0;GUETER],siedler:0,abflug:0,ankunft:100,flugdauer:100,rueckkehr:0,orbit_ende:0,haltedauer:0,zustand:Flottenzustand::Hinflug,gemeldet:false,verband:None};
        w.flotten.insert(1,f.clone());
        let mut private=f.clone();private.id=2;private.ziel=w.planeten[w.spieler[2].heimat as usize].koord;w.flotten.insert(2,private);
        let mut hostile=f;hostile.id=3;hostile.besitzer=2;w.flotten.insert(3,hostile);
        let known=w.verbuendete_anfluege(0);assert_eq!(known.len(),1);assert_eq!(known[0]["flotte"],1);
        assert!(known[0]["schiffe"].is_null());assert!(known[0].get("ladung").is_none());
    }
    #[test] fn shield_and_armour_technology_change_real_scenario_results() {
        let mut r=Regelwerk::laden(include_str!("../../../regeln/online-v1.ron")).unwrap();
        let e=r.einheiten.get_mut(&Einheit::LeichterJaeger).unwrap();e.angriff=1000;e.struktur=1000;
        let w=Welt::neu(r,123,1).unwrap();
        let p=json!({"volk":"aurelianer","einheiten":{"leichter_jaeger":10},"technik":{"waffentechnik":0,"schildtechnik":0,"panzerung":0}});
        let mut q=json!({"angreifer":p,"verteidiger":p});let baseline=w.kampfszenario(&q).unwrap();
        q["verteidiger"]["technik"]["schildtechnik"]=json!(30);q["verteidiger"]["technik"]["panzerung"]=json!(30);
        let stronger=w.kampfszenario(&q).unwrap();assert_ne!(baseline["eigene_verluste_wert"],stronger["eigene_verluste_wert"]);
    }
    #[test] fn transport_returns_without_deuterium_at_destination() {
        let r=Regelwerk::laden(include_str!("../../../regeln/online-v1.ron")).unwrap();let mut w=Welt::neu(r,123,2).unwrap();
        let start=w.spieler[0].heimat as usize;let destination=w.spieler[1].heimat as usize;
        w.planeten[destination].besitzer=0;w.spieler[0].planeten.push(destination as PlanetId);
        let a=w.planeten[start].koord;let b=w.planeten[destination].koord;
        w.planeten[start].gebaeude[Gebaeude::Raumhafen.idx()]=1;
        w.planeten[start].einheiten[Einheit::KleinerTransporter.idx()]=1;
        w.planeten[start].bestand[Gut::Deuterium.idx()]=100_000*M;
        w.planeten[destination].bestand[Gut::Deuterium.idx()]=0;
        let mut ships=[0;SCHIFFE];ships[Einheit::KleinerTransporter.idx()]=1;let mut cargo=[0;GUETER];cargo[Gut::Erz.idx()]=100*M;
        let plan=w.flugplan(0,start,b,&ships,1000).unwrap();let before=w.planeten[start].bestand[Gut::Deuterium.idx()];
        w.flotte_senden(0,Rolle::Alle,crate::flotte::Flugauftrag{start:a,ziel:b,mission:Mission::Transport,schiffe:ships,sigma_pm:1000,ladung:cargo,haltedauer:0}).unwrap();
        assert_eq!(before-w.planeten[start].bestand[Gut::Deuterium.idx()],plan.treibstoff*2);
        for _ in 0..10000 {if w.flotten.is_empty(){break;}w.schritt();}
        assert!(w.flotten.is_empty());assert_eq!(w.planeten[start].einheiten[Einheit::KleinerTransporter.idx()],1);
    }
}
