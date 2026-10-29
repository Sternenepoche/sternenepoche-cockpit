//! Observed snapshots, never a live oracle for foreign planets.
use crate::{typen::*, Welt};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Scan {
    pub zeit: SimZeit,
    pub technik: u8,
    pub besitzer: Option<SpielerId>,
    pub felder: u16,
    pub zone: Zone,
    pub erz: [i64; 2],
    pub kristall: [i64; 2],
    pub bestand: Vec<[i64; 2]>,
}

/// Fixed bins prevent repeated identical scans from revealing an exact value.
pub fn intervall(wert: i64, schritt: i64) -> [i64; 2] {
    let unten = wert.max(0) / schritt * schritt;
    [unten, unten.saturating_add(schritt)]
}

impl Welt {
    /// Alliance members share the resident list of systems containing one of their worlds.
    /// This grants no planet scan, environment, stock, buildings or fleet intelligence.
    pub fn allianzsystem(&self, sid: SpielerId, k: Koord) -> bool {
        let Some(aid) = self.spieler[sid as usize].allianz else { return false; };
        self.allianzen.iter().find(|a|a.id == aid).is_some_and(|a|a.mitglieder.iter().any(|m| {
            !self.aufklaerung.inaktive_spieler.contains(m) && self.spieler[*m as usize].planeten.iter().any(|pid| {
                let p=&self.planeten[*pid as usize];
                p.besitzer == *m && p.koord.sektor == k.sektor && p.koord.system == k.system
            })
        }))
    }
    pub(crate) fn scan_aufzeichnen(&mut self, sid: SpielerId, ziel: Koord) {
        let (Some(p), Some(s)) = (self.platz(ziel), self.system(ziel)) else { return };
        let technik = self.spieler[sid as usize].forschung[Forschung::Spionagetechnik.idx()];
        let schritt = (800 / (1 + technik as i64)).max(25);
        let besitzer = self.belegung.get(&ziel).map(|pid| self.planeten[*pid as usize].besitzer);
        let (felder,zone)=self.belegung.get(&ziel).filter(|_|self.aufklaerungsregeln()).map(|pid|{
            let actual=&self.planeten[*pid as usize];(actual.felder,actual.zone)
        }).unwrap_or((p.felder,p.zone));
        let zone=if self.ausbau.version>0 {crate::habitabilitaet::zone(ziel.position,self.regeln.welt.plaetze_je_system)} else {zone};
        // Generation 12: eine angekommene Sonde liefert exakte Werte (nichts bleibt geschätzt).
        let exakt = self.ausbau.version > 0;
        let genau = |wert: i64, schritt: i64| if exakt { [wert, wert] } else { intervall(wert, schritt) };
        let bestand = self.belegung.get(&ziel).map(|pid| self.bestand_jetzt(*pid as usize)
            .iter().map(|v| genau(ganz(*v), (10000 / (1 + technik as i64)).max(100))).collect()).unwrap_or_default();
        self.scans.insert((sid, ziel), Scan { zeit: self.zeit, technik, besitzer,
            felder, zone, erz: genau(s.reich_erz, schritt),
            kristall: genau(s.reich_kristall, schritt), bestand });
    }

    pub fn planetenwissen(&self, sid: SpielerId, k: Koord) -> Value {
        let eigen = self.belegung.get(&k).map(|p| &self.planeten[*p as usize]).filter(|p| p.besitzer == sid);
        let mut v = json!({"koord":k.to_string(), "position":k.position,"status":"unbekannt","bekannt":false});
        if self.ausbau.version>0 { v["habitabilitaet"]=self.klima_sicht(sid,k); }
        if self.system_erfasst(sid,k) {v["xeno_vorkommen"]=json!(self.xeno_planet(k));}
        if let Some(p) = eigen {
            v["status"]=json!("eigen"); v["bekannt"]=json!(true);
            v["spieler"]=json!(self.spieler[sid as usize].name);
            v["felder"]=json!(p.felder); v["zone"]=json!(if self.ausbau.version>0 {crate::habitabilitaet::zone(k.position,self.regeln.welt.plaetze_je_system).name()}else{p.zone.name()});
            v["heimat"]=json!(p.heimat);v["besiegt"]=json!(self.ist_besiegt(sid));
            v["beziehung"]=self.beziehung(sid,sid);
            if let Some(m)=self.mond_kartensicht(sid,k){v["mond"]=m;}
            if let Some(n)=self.planetenname(p.id as usize){v["name"]=json!(n);}
            // 0.4.8: auch über eigenen Planeten stehende fremde gestrandete Flotten und der Zustand „verwüstet“.
            if self.verwuestet(p.id as usize){v["verwuestet"]=json!(true);}
            if self.orbitflotten_aktiv(){let o=self.orbitflotten_an(sid,k);if o.as_array().is_some_and(|a|!a.is_empty()){v["orbitflotten"]=o;}}
            return v;
        }
        if let Some(s) = self.scans.get(&(sid,k)) {
            v["bekannt"]=json!(true); v["zeit"]=json!(s.zeit);
            v["alter_stunden"]=json!((self.zeit-s.zeit)/STUNDE);
            v["technik"]=json!(s.technik); v["felder"]=json!(s.felder); v["zone"]=json!(if self.ausbau.version>0 {crate::habitabilitaet::zone(k.position,self.regeln.welt.plaetze_je_system).name()}else{s.zone.name()});
            v["ertrag_promille"]=json!({"erz":s.erz,"kristall":s.kristall});
            v["bestand_spannen"]=json!(Gut::ALLE.iter().zip(&s.bestand).map(|(g,b)|(g.name(),b)).collect::<std::collections::BTreeMap<_,_>>());
            v["status"]=json!(match s.besitzer {None=>"frei",Some(owner) if self.verbuendet(sid,owner)=>"freund",Some(_)=>"feind"});
            if let Some(owner)=s.besitzer { v["spieler"]=json!(self.spieler[owner as usize].name);v["besiegt"]=json!(self.ist_besiegt(owner));v["heimat"]=json!(self.planeten[self.spieler[owner as usize].heimat as usize].koord==k); }
        } else if let Some(b)=self.spieler[sid as usize].berichte.iter().find(|b|b.ziel==k) {
            v["bekannt"]=json!(true); v["spieler"]=json!(self.spieler[b.besitzer as usize].name);
            v["status"]=json!(if self.verbuendet(sid,b.besitzer){"freund"}else{"feind"});
            v["alter_stunden"]=json!((self.zeit-b.zeit)/STUNDE);
            v["besiegt"]=json!(self.ist_besiegt(b.besitzer));v["heimat"]=json!(self.planeten[self.spieler[b.besitzer as usize].heimat as usize].koord==k);
        } else if let Some(e)=self.spieler[sid as usize].erkundet.get(&k) {
            v["bekannt"]=json!(true); v["status"]=json!("frei"); v["zone"]=json!(if self.ausbau.version>0 {crate::habitabilitaet::zone(k.position,self.regeln.welt.plaetze_je_system).name()}else{e.zone.name()});
            v["felder"]=json!(e.felder); v["alter_stunden"]=json!((self.zeit-e.zeit)/STUNDE);
            v["ertrag_promille"]=json!({"erz":intervall(e.reich_erz,800),"kristall":intervall(e.reich_kristall,800)});
        } else if self.allianzsystem(sid,k) {
            let owner = self.belegung.get(&k).map(|pid|self.planeten[*pid as usize].besitzer)
                .filter(|owner|!self.aufklaerung.inaktive_spieler.contains(owner));
            v["besiedlung_bekannt"]=json!(true);v["bewohnt"]=json!(owner.is_some());v["sichtgrund"]=json!("allianz");
            if let Some(owner) = owner { v["spieler"]=json!(self.spieler[owner as usize].name); }
        } else if let Some(owner) = self.kommunikation.systemkontakte.get(&format!("{sid}:{}:{}",k.sektor,k.system)).and_then(|s|s.get(&k.position)) {
            v["besiedlung_bekannt"]=json!(true);
            v["bewohnt"]=json!(owner.is_some());
            if let Some(owner) = owner { v["spieler"]=json!(self.spieler[*owner as usize].name); }
        }
        if let Some(name)=v["spieler"].as_str(){if let Ok(owner)=self.spieler_nach_name(name){v["beziehung"]=self.beziehung(sid,owner);}}
        if self.system_erfasst(sid,k){v["rohstoffring"]=json!(self.natuerlicher_ring(k));if let Some(t)=self.truemmer.get(&k){let mut tv=json!({"erz":ganz(t[0]),"kristall":ganz(t[1])});self.truemmer_zusatz(k,&mut tv);v["truemmer"]=tv;}}
        if let Some(m)=self.mond_kartensicht(sid,k){v["mond"]=m;}
        // Invasion v2: verwüstete Planeten sind für alle Kenner des Systems gekennzeichnet (Kolonieschiff kann landen).
        if self.system_erfasst(sid,k){if let Some(pid)=self.belegung.get(&k){if self.verwuestet(*pid as usize){v["verwuestet"]=json!(true);}}}
        // 0.4.8: gestrandete Flotten sind für alle Kenner des Systems sichtbar und angreifbar.
        if self.orbitflotten_aktiv(){let o=self.orbitflotten_an(sid,k);if o.as_array().is_some_and(|a|!a.is_empty()){v["orbitflotten"]=o;}}
        // Planetennamen nur für Verbündete, und nur wo der Besitzer ohnehin bekannt ist.
        if v["spieler"].is_string() { if let Some(n)=self.planetenname_fuer(sid,k){v["name"]=json!(n);} }
        v
    }

    /// Coordinates enter the atlas only through ownership or the viewer's observations.
    pub fn atlas_sicht(&self, sid: SpielerId) -> Vec<Value> {
        let sp = &self.spieler[sid as usize];
        let mut coords: std::collections::BTreeSet<Koord> = sp.planeten.iter().map(|p|self.planeten[*p as usize].koord).collect();
        coords.extend(self.scans.keys().filter(|(s,_)|*s == sid).map(|(_,k)|*k));
        coords.extend(sp.berichte.iter().map(|b|b.ziel));
        coords.extend(sp.erkundet.keys().copied());
        if let Some(aid)=sp.allianz {
            if let Some(a)=self.allianzen.iter().find(|a|a.id == aid) {
                for k in a.mitglieder.iter().flat_map(|m|self.spieler[*m as usize].planeten.iter()).map(|pid|self.planeten[*pid as usize].koord) {
                    coords.extend((1..=self.regeln.welt.plaetze_je_system).map(|p|Koord::neu(k.sektor,k.system,p)));
                }
            }
        }
        for &(viewer, sector, system) in self.aufklaerung.systemberichte.keys().filter(|(s,_,_)|*s == sid) {
            if self.kommunikation.systemkontakte.contains_key(&format!("{viewer}:{sector}:{system}")) {
                coords.extend((1..=self.regeln.welt.plaetze_je_system).map(|p|Koord::neu(sector,system,p)));
            }
        }
        coords.into_iter().filter_map(|k| {
            let p = self.planetenwissen(sid,k);
            if p["bekannt"]!=true && p["besiedlung_bekannt"]!=true { return None; }
            // The map needs empty observations too, but no resource or espionage payload.
            let mut atlas=json!({"koord":k.to_string(),"position":k.position,"bekannt":p["bekannt"]});
            for key in ["besiedlung_bekannt","bewohnt","status","spieler","beziehung","zone","mond"] {
                if let Some(value)=p.get(key){atlas[key]=value.clone();}
            }
            Some(atlas)
        }).collect()
    }
}
