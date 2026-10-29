//! Shared production controls and early-colony supply chain. No UI-only rates.
use crate::{typen::*, welt::*, Welt};
use serde_json::{json, Value};

pub const PRODUKTION: [Gebaeude; 16] = [Gebaeude::Erzmine, Gebaeude::Kristallmine,
    Gebaeude::Deuteriumsynthesizer, Gebaeude::Farm, Gebaeude::Solarkraftwerk,
    Gebaeude::Fusionskraftwerk, Gebaeude::Giesserei, Gebaeude::Elektronikwerk,
    Gebaeude::Konsumgueterwerk, Gebaeude::Xenoextraktor, Gebaeude::Orbitalwerft, Gebaeude::Werft, Gebaeude::Kaserne, Gebaeude::Raketensilo, Gebaeude::Labor, Gebaeude::Akademie];
pub fn bedarf_prozent(stufe: u8) -> i64 { if stufe <= 100 { stufe as i64 } else { 100 + (stufe as i64 - 100) * 2 } }
impl Welt {
    pub fn werk_ruht(&self,pid:usize,g:Gebaeude)->bool {
        if self.ausbau.industrie_version==0{return false;}
        if self.planeten[pid].gebaeude[g.idx()]==0{return true;}
        if self.produktionsstufen(pid)[g.idx()]==0{return true;}
        let (energie,besetzung)=self.werk_kern(pid);
        energie<1000 || besetzung[g.idx()]<1000
    }
    /// Energiedeckung und wirksame Besetzung je Gebäude aus der Werksbilanz von `planeten[pid]` – über
    /// Spielsekunden gemerkt (`WerkMerk`). Gleich `industrie_berechnen(&self.planeten[pid])`, nur ohne Neurechnung,
    /// solange sich keine Eingabe geändert hat.
    /// Werksbilanz eines Gebäudes, frisch gerechnet (für Anzeigen wie den Ruhegrund der Werft; nicht im Sekundenpfad).
    pub(crate) fn werksbilanz(&self,pid:usize,g:Gebaeude)->Option<crate::planer::wirtschaft::WerksBilanz> {
        let p=&self.planeten[pid];let sp=&self.spieler[p.besitzer as usize];let ein=self.industrie_eingaben(p);
        crate::planer::wirtschaft::berechnen_geregelt_stufe(&self.regeln,p,&sp.forschung,sp.volk,&ein.integritaet,ein.personen,&ein.stufen,ein.xeno,ein.dyson,sp.stufe).werke.into_iter().find(|w|w.gebaeude==g)
    }
    pub(crate) fn werk_kern(&self,pid:usize)->(i64,[i64;GEBAEUDE]) {
        let p=&self.planeten[pid];
        let sp=&self.spieler[p.besitzer as usize];
        let rechnen=|ein:&IndustrieEingaben<'_>|{let e=crate::planer::wirtschaft::berechnen_geregelt_stufe(&self.regeln,p,&sp.forschung,sp.volk,&ein.integritaet,ein.personen,&ein.stufen,ein.xeno,ein.dyson,sp.stufe);
            (e.energie_deckung_promille,std::array::from_fn(|i|e.werke.iter().find(|w|w.gebaeude==Gebaeude::ALLE[i]).map_or(0,|w|w.wirksame_besetzung_promille)))};
        // Schlüssel ohne Lagergrenze (teure Potenz): die Regler samt Lagerruhe folgen aus Reglerzeile, Ablaufversion,
        // Bestand, Lagerstufe, Lagerintegrität und Regelwerk, die alle im Schlüssel stehen.
        let regler=self.ausbau.produktionsregler.get(&self.planeten[p.id as usize].id);
        let integritaet=self.integritaeten(p.id as usize);
        let personen=self.ausbau.personen.get(&p.id);
        let (xeno,dyson,ablauf)=(self.xeno_planet(p.koord),self.dyson_wirkung(p),self.ablauf_v1());
        let mut merk=self.werk_merk.0.lock().unwrap_or_else(|e|e.into_inner());
        if merk.len()<=pid{merk.resize_with(pid+1,||None);}
        if let Some(e)=merk[pid].as_ref().filter(|e|e.passt(&self.regeln,p,sp,regler,&integritaet,personen,xeno,dyson,ablauf)) {
            let ergebnis=(e.energie_deckung_promille,e.besetzung);
            // Tests (Debug-Bau) prüfen jeden Treffer gegen die frische Rechnung: fehlt eine Eingabe im Schlüssel, fällt es hier auf.
            debug_assert_eq!(ergebnis,rechnen(&self.industrie_eingaben(p)),"WerkMerk veraltet für Planet {pid}: Schlüssel in industrie.rs ergänzen");
            return ergebnis;
        }
        let ein=self.industrie_eingaben(p);
        let (energie,besetzung)=rechnen(&ein);
        merk[pid]=Some(WerkMerkEintrag{regeln:self.regeln.clone(),gebaeude:p.gebaeude,bestand:p.bestand,bevoelkerung:p.bevoelkerung,
            einheiten:p.einheiten,faktor:p.faktor,grundwohnraum:p.grundwohnraum,koord:p.koord,stabilitaet:p.stabilitaet,
            prioritaeten:p.prioritaeten.clone(),forschung:sp.forschung,volk:sp.volk,integritaet:ein.integritaet,
            personen:ein.personen.cloned(),regler:regler.cloned(),ablauf,xeno:ein.xeno,dyson:ein.dyson,stufe:sp.stufe,energie_deckung_promille:energie,besetzung});
        (energie,besetzung)
    }
    pub fn werk_tempo(&self,pid:usize,g:Gebaeude)->i64 {let n=self.produktionsstufen(pid)[g.idx()];if n==0{100}else{n as i64}}
    pub fn produktionskosten(&self,pid:usize,g:Gebaeude,basis:[i64;GUETER])->[i64;GUETER]{
        let speed=self.werk_tempo(pid,g);if self.ausbau.industrie_version==0||speed<=100{return basis;}
        std::array::from_fn(|i|if i>=Gut::Antriebskern.idx(){basis[i]}else{(basis[i]*bedarf_prozent(speed as u8)+speed-1)/speed})
    }
    pub fn erstattung(&mut self,pid:usize,kosten:[i64;GUETER],topf:Option<Topf>) {
        self.abrechnen(pid);let sid=self.planeten[pid].besitzer as usize;
        for g in 0..GUETER {self.planeten[pid].bestand[g]+=kosten[g];}
        self.spieler[sid].xeno_verbaut=(self.spieler[sid].xeno_verbaut-kosten[Gut::Xenokristall.idx()]).max(0);
        if let Some(t)=topf {self.spieler[sid].toepfe[t.idx()]+=self.regeln.wert(&kosten);}
    }
    pub fn industrie_aktivieren(&mut self) {
        if self.ausbau.version==0 || self.ausbau.industrie_version>=1 {return;}
        for p in &self.planeten {for n in 0..2 {let receipts=p.fertigung[n].iter().map(|a|(None,match a.produkt{Produkt::Einheit(e)=>self.regeln.kosten_einheit(e,self.spieler[p.besitzer as usize].volk),Produkt::Bauteil(g)=>self.regeln.kosten_bauteil(g).unwrap_or([0;GUETER])})).collect::<Vec<_>>();if !receipts.is_empty(){self.ausbau.fertigung_zahlung.insert(format!("{}:{n}",p.id),receipts);}}}
        let r=std::sync::Arc::make_mut(&mut self.regeln);
        r.gebaeude.get_mut(&Gebaeude::Kaserne).unwrap().ab_stufe=3;
        let e=r.einheiten.get_mut(&Einheit::Kolonieschiff).unwrap();
        e.ab_stufe=3;e.werft=3;e.braucht.remove(&Gebaeude::Orbitalwerft);
        e.kosten=std::collections::BTreeMap::from([(Gut::Legierung,1000),(Gut::Elektronik,500),(Gut::Habitatmodul,1)]);
        if let Some(p)=r.flottendoktrin.get_mut(&Einheit::Kolonieschiff) {p.voraussetzungen=std::collections::BTreeMap::from([(Forschung::Impulsantrieb,2)]);}  // Karl 25.10.2026: ohne Astrophysik
        r.wirtschaft.bauteile.insert(Gut::Habitatmodul,std::collections::BTreeMap::from([(Gut::Legierung,400),(Gut::Elektronik,250)]));
        // Recipe identity changes once, without invalidating or deleting existing worlds.
        {use sha2::{Digest,Sha256};r.hash=format!("{:x}",Sha256::digest(format!("{}:industrie-035-v1",r.hash)));}
        self.ausbau.industrie_version=1;
        for sp in &mut self.spieler {if !self.aufklaerung.inaktive_spieler.contains(&sp.id){sp.schutz_bis=self.zeit;}}
        if self.zeit==0 {
            let homes:std::collections::BTreeSet<_>=self.planeten.iter().filter(|p|p.heimat).map(|p|(p.koord.sektor,p.koord.system)).collect();
            // Every fourth eligible system; homes are excluded before distribution.
            let mut index=0usize;
            for s in &mut self.systeme {s.nebel=if homes.contains(&(s.sektor,s.nummer)){false}else{index+=1;index%4==0};}
            let count=self.regeln.welt.plaetze_je_system as usize;
            for (i,sys) in self.systeme.iter().enumerate() {if sys.nebel {for pos in (2..=count).step_by(2) {let p=&mut self.plaetze[i*count+pos-1];p.felder=(p.felder*65/100).max(40);}}}
        }
        for pid in 0..self.planeten.len(){self.raten_neu(pid);}
    }
    /// Only called for a newly created or just-claimed reserved home, never for established colonies.
    pub fn heimat_anpassen(&mut self,sid:SpielerId) {
        let sp=&self.spieler[sid as usize];let pid=sp.heimat as usize;let old=self.planeten[pid].koord;
        // Ablauf v1: Syntheten auf Platz 10 statt 11 (Startplatz-Fairness, docs/WACHSTUM-UND-HABITABILITAET.md).
        let position=crate::habitabilitaet::heimatposition(sp.volk,self.regeln.ablauf).min(self.regeln.welt.plaetze_je_system);
        let target=Koord{position,..old};if target!=old && self.belegung.contains_key(&target){return;}
        self.belegung.remove(&old);self.belegung.insert(target,pid as PlanetId);self.planeten[pid].koord=target;
        // Cold starts must cover synthetic population with their actual power source.
        if sp.volk==Volk::Syntheten {self.planeten[pid].gebaeude[Gebaeude::Solarkraftwerk.idx()]=self.planeten[pid].gebaeude[Gebaeude::Solarkraftwerk.idx()].max(5);}
    }
    pub fn orbital_automatik(&mut self,pid:usize) {
        let p=&self.planeten[pid];
        if self.ausbau.industrie_version==0 || !self.planet_aktiv(pid) || p.gebaeude[Gebaeude::Orbitalwerft.idx()]==0
            || self.parallel_sicht(pid,1)["frei"].as_i64().unwrap_or(0)==0 || self.auftrag_pausiert("bauteile",p.id)
            || self.produktionsstufen(pid)[Gebaeude::Orbitalwerft.idx()]==0 {return;}
        let (energie,besetzung)=self.werk_kern(pid);
        if energie<1000 || besetzung[Gebaeude::Orbitalwerft.idx()]<1000{return;}
        let p=&self.planeten[pid];let (sid,k)=(p.besitzer,p.koord);let mut goods=[Gut::Habitatmodul,Gut::Antriebskern];
        goods.sort_by_key(|g|p.bestand[g.idx()]);
        for g in goods {
            // Automatic stock is intentionally bounded; no unbounded resource sink.
            let ziel=self.ausbau.produktionsserien.get(&(pid as u32)).and_then(|m|m.get(&format!("bauteil:{}",g.name()))).filter(|c|c.besitzer==sid).map(|c|c.ziel).unwrap_or(10);
            let(stock,free,_)=self.serien_vorrat_plaetze(pid,"bauteil",g.name());let count=(ziel-stock).min(free);if count<=0{continue;}
            for amount in (1..=count).rev(){if self.fertigen(sid,Rolle::Alle,k,Produkt::Bauteil(g),amount).is_ok(){break;}}
        }
    }
    pub fn produktionsstufen(&self, pid: usize) -> [u8; GEBAEUDE] {
        // Läuft in Sekundenpfaden: die Reglerzeile des Planeten einmal durchlaufen statt je Gebäude zu suchen.
        let mut stufen=[100u8;GEBAEUDE];
        if let Some(m)=self.ausbau.produktionsregler.get(&self.planeten[pid].id){for (g,v) in m{stufen[g.idx()]=*v;}}
        stufen
    }
    pub fn produktion_setzen(&mut self, sid: SpielerId, k: Koord, g: Gebaeude, prozent: u8) -> Result<String,String> {
        let pid=self.eigener_planet(sid,k)?;
        if self.ausbau.version==0 || !PRODUKTION.contains(&g) || prozent>120 || prozent%10!=0 {
            return Err("Produktionsgebäude und ganze 10-%-Schritte zwischen 0 und 120 wählen".into());
        }
        if self.planeten[pid].gebaeude[g.idx()]==0 {return Err("Das Gebäude steht auf diesem Planeten noch nicht".into());}
        self.abrechnen(pid);
        let n=usize::from(g==Gebaeude::Orbitalwerft);
        if [Gebaeude::Orbitalwerft,Gebaeude::Werft].contains(&g) && !self.planeten[pid].fertigung[n].is_empty() {
            let previous=self.produktionsstufen(pid)[g.idx()];let previous=if previous==0{100}else{previous} as i64;
            let next=if prozent==0{100}else{prozent} as i64;
            if let Some(lots)=self.ausbau.parallel.get_mut(&crate::parallel::key(pid,n)){for b in lots{b.ende=self.zeit+((b.ende-self.zeit).max(0)*previous+next-1)/next;b.dauer=(b.dauer*previous+next-1)/next;}}
            let p=&mut self.planeten[pid];
            for a in &mut p.fertigung[n] {a.dauer=(a.dauer*previous+next-1)/next;}
            p.fertigung_naechste[n]=self.zeit+((p.fertigung_naechste[n]-self.zeit).max(0)*previous+next-1)/next;
            let end=p.fertigung_naechste[n];let id=p.id;
            self.ereignisse.retain(|e|!matches!(e.art,EreignisArt::FertigungFertig{planet,schleife}if planet==id&&schleife as usize==n));
            self.plane(end,EreignisArt::FertigungFertig{planet:id,schleife:n as u8});
        }
        let old=self.werk_tempo(pid,g);let next=if prozent==0{100}else{prozent as i64};
        if g==Gebaeude::Raketensilo {for e in &self.ereignisse{if matches!(e.art,EreignisArt::RaketenFertig{planet,..}if planet==pid as u32){if let Some(paid)=self.ausbau.raketen_zahlung.get_mut(&e.seq){paid.dauer=(paid.dauer*old+next-1)/next;paid.basis=(paid.basis*old+next-1)/next;for b in &mut paid.lose{b.dauer=(b.dauer*old+next-1)/next;b.ende=self.zeit+((b.ende-self.zeit).max(0)*old+next-1)/next;}}}}let queue=std::mem::take(&mut self.ereignisse);self.ereignisse=queue.into_iter().map(|mut e|{if matches!(e.art,EreignisArt::RaketenFertig{planet,..}if planet==pid as u32){e.zeit=self.zeit+((e.zeit-self.zeit).max(0)*old+next-1)/next;}e}).collect();}
        if g==Gebaeude::Akademie {for a in &mut self.ausbau.ausbildungen{if a.planet==pid as u32{a.fertig=self.zeit+((a.fertig-self.zeit).max(0)*old+next-1)/next;}}}
        self.ausbau.produktionsregler.entry(self.planeten[pid].id).or_default().insert(g,prozent);
        self.raten_neu(pid);
        Ok(format!("{g} auf {k}: {prozent} % Ausstoß, {} % Grundbedarf",bedarf_prozent(prozent)))
    }
    pub fn xeno_planet(&self,k:Koord)->bool {
        k.position>0 && k.position%2==0 && self.system(k).is_some_and(|s|s.nebel)
            && !self.planeten.iter().any(|p|p.heimat && p.koord.sektor==k.sektor && p.koord.system==k.system)
    }
    pub fn industrie_berechnen(&self,p:&Planet)->crate::planer::wirtschaft::WirtschaftErgebnis {
        let sp=&self.spieler[p.besitzer as usize];
        let ein=self.industrie_eingaben(p);
        crate::planer::wirtschaft::berechnen_geregelt_stufe(&self.regeln,p,&sp.forschung,sp.volk,&ein.integritaet,ein.personen,&ein.stufen,ein.xeno,ein.dyson,sp.stufe)
    }
    /// Alle Eingaben der Werksbilanz außer Planet und Spieler – gemeinsam für `industrie_berechnen` und `werk_kern`.
    fn industrie_eingaben(&self,p:&Planet)->IndustrieEingaben<'_> {
        let mut stufen=self.produktionsstufen(p.id as usize);
        // Ablauf-Version 1: Ein Verarbeitungswerk ruht, solange das Lager seines Erzeugnisses voll ist; sonst
        // verbraucht es Erz, Kristall und Deuterium, während das Erzeugnis an der Lagergrenze verfällt.
        if self.ablauf_v1(){
            let grenze=self.lagergrenze(p.id as usize);
            for (werk,gut) in [(Gebaeude::Giesserei,Gut::Legierung),(Gebaeude::Elektronikwerk,Gut::Elektronik),(Gebaeude::Konsumgueterwerk,Gut::Konsumgut)]{
                if p.bestand[gut.idx()]>=grenze[gut.idx()]{stufen[werk.idx()]=0;}
            }
        }
        IndustrieEingaben{integritaet:self.integritaeten(p.id as usize),
            personen:self.ausbau.personen.get(&p.id),stufen,xeno:self.xeno_planet(p.koord),dyson:self.dyson_wirkung(p)}
    }
    pub fn industrie_sicht(&self,pid:usize)->Value {
        let mut p=self.planeten[pid].clone();p.bestand=self.bestand_jetzt(pid);
        let e=self.industrie_berechnen(&p);let stufen=self.produktionsstufen(pid);
        json!({"einheit":"je Spielstunde","gueter":Gut::ALLE.map(|g|g.name()),"forschung_milli":e.fp_rate,"produktion_milli":e.produktion,"verbrauch_milli":e.verbrauch,
            "bevoelkerungsverbrauch_milli":e.verbrauch_bevoelkerung,"werke":e.werke,
            "regler":PRODUKTION.iter().map(|g|(g.name(),stufen[g.idx()])).collect::<std::collections::BTreeMap<_,_>>(),
            "xeno_natuerlich":self.xeno_planet(p.koord),"xeno_synthese_prozent":65,
            "ueberlast":{"110":120,"120":140},"bauteile_automatisch":p.gebaeude[Gebaeude::Orbitalwerft.idx()]>0})
    }
}

struct IndustrieEingaben<'a> {
    integritaet: [i64; GEBAEUDE],
    personen: Option<&'a crate::ausbau::Personen>,
    stufen: [u8; GEBAEUDE],
    xeno: bool,
    dyson: Option<crate::dyson::DysonWirkung>,
}

/// Merkzettel für `werk_ruht` über Spielsekunden hinweg (Welt-Feld, nie serialisiert, Kopien beginnen leer).
/// Die Sekundenpfade (`zeit_vorruecken`, `boden_naechste_fertigstellung`, `produktionsserien_tick`) fragen
/// je Planet viele Male, ob ein Werk ruht; die Antwort hängt nur an der Werksbilanz. Ein Eintrag gilt, solange
/// ALLE Eingaben von `planer::wirtschaft::berechnen_geregelt` exakt gleich sind (gelesene Planetenfelder,
/// Forschung, Volk, Integrität, Personen, Regler samt Lagerruhe, Xenovorkommen, Dyson-Wirkung, Regelwerk).
/// Darum ist das Ergebnis bitgleich zur Neuberechnung. Liest `berechnen_geregelt` künftig ein weiteres
/// Planetenfeld, muss es hier in den Schlüssel – der Debug-Vergleich in `werk_kern` meldet es in den Tests.
#[derive(Default)]
pub struct WerkMerk(std::sync::Mutex<Vec<Option<WerkMerkEintrag>>>);
impl Clone for WerkMerk { fn clone(&self) -> Self { Self::default() } }
impl std::fmt::Debug for WerkMerk { fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { f.write_str("WerkMerk") } }

pub(crate) struct WerkMerkEintrag {
    /// Hält das Regelwerk fest: Jede Regeländerung (`Arc::make_mut`) legt so eine neue Kopie an und trifft nicht mehr.
    regeln: std::sync::Arc<crate::Regelwerk>,
    gebaeude: [u8; GEBAEUDE],
    bestand: [i64; GUETER],
    bevoelkerung: i64,
    einheiten: [i64; EINHEITEN],
    faktor: [i64; 5],
    grundwohnraum: i64,
    koord: Koord,
    stabilitaet: i64,
    prioritaeten: Vec<Gebaeude>,
    forschung: [u8; FORSCHUNGEN],
    volk: Volk,
    integritaet: [i64; GEBAEUDE],
    personen: Option<crate::ausbau::Personen>,
    /// Reglerzeile und Ablaufversion statt der fertigen Regler: die Lagerruhe (Ablauf v1) folgt daraus zusammen
    /// mit Bestand, Gebäuden, Integrität und Regelwerk.
    regler: Option<std::collections::BTreeMap<Gebaeude, u8>>,
    ablauf: bool,
    xeno: bool,
    dyson: Option<crate::dyson::DysonWirkung>,
    /// Zivilisationsstufe des Besitzers: vor Stufe III kein Personalabzug (planer::wirtschaft::PERSONAL_AB_STUFE).
    stufe: u8,
    energie_deckung_promille: i64,
    besetzung: [i64; GEBAEUDE],
}
impl WerkMerkEintrag {
    #[allow(clippy::too_many_arguments)]
    fn passt(&self, regeln: &std::sync::Arc<crate::Regelwerk>, p: &Planet, sp: &Spieler, regler: Option<&std::collections::BTreeMap<Gebaeude, u8>>,
        integritaet: &[i64; GEBAEUDE], personen: Option<&crate::ausbau::Personen>, xeno: bool, dyson: Option<crate::dyson::DysonWirkung>, ablauf: bool) -> bool {
        std::sync::Arc::ptr_eq(&self.regeln, regeln) && self.gebaeude == p.gebaeude && self.bestand == p.bestand
            && self.bevoelkerung == p.bevoelkerung && self.einheiten == p.einheiten && self.faktor == p.faktor
            && self.grundwohnraum == p.grundwohnraum && self.koord == p.koord && self.stabilitaet == p.stabilitaet
            && self.prioritaeten == p.prioritaeten && self.forschung == sp.forschung && self.volk == sp.volk
            && self.integritaet == *integritaet && self.personen.as_ref() == personen && self.regler.as_ref() == regler
            && self.ablauf == ablauf && self.xeno == xeno && self.dyson == dyson && self.stufe == sp.stufe
    }
}
