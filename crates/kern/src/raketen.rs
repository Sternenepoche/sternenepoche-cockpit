//! Gemeinsame Siloproduktion, Datenblätter und zeitlich aufgelöste Raketenschläge.
use crate::{typen::*, welt::*};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::BTreeMap;
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Raketensystem {
    pub lager: BTreeMap<PlanetId, [i64; 3]>,
    pub salven: BTreeMap<u64, Salve>,
}
#[derive(Clone,Debug,Serialize,Deserialize)]
pub struct RaketenZahlung{pub kosten:[i64;GUETER],pub topf:Option<Topf>,pub dauer:i64,#[serde(default)]pub basis:i64,#[serde(default)]pub rest:i64,#[serde(default)]pub menge:i64,#[serde(default)]pub lose:Vec<crate::parallel::Los>}
impl Raketensystem {
    pub fn ist_leer(&self) -> bool {
        self.lager.is_empty() && self.salven.is_empty()
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Salve {
    pub von: SpielerId,
    pub gegner: SpielerId,
    pub ziel: Koord,
    pub flotte: Option<FlottenId>,
    pub art: u8,
    pub anzahl: i64,
    pub zieltyp: Einheit,
    pub schaden: i64,
    pub ankunft: SimZeit,
}
#[derive(Clone, Debug, Serialize)]
pub struct Raketenregel {
    pub key: &'static str,
    pub name: &'static str,
    pub silo: u8,
    pub zivilisation: u8,
    pub plaetze: i64,
    pub fertigungsplaetze: i64,
    pub bauzeit_sekunden: i64,
    pub flug_min_sekunden: i64,
    pub flug_je_system_sekunden: i64,
    pub reichweite_je_silo: i64,
    pub schaden: i64,
    pub abfangbedarf: i64,
    pub planet: bool,
    pub flotte: bool,
    pub braucht: BTreeMap<Forschung, u8>,
    pub kosten: BTreeMap<Gut, i64>,
    pub besonderheit: &'static str,
    pub zielprofil: RaketenZielprofil,
}
#[derive(Clone, Debug, Serialize)]
pub struct RaketenZielprofil {
    pub primaer: &'static str,
    pub sekundaer: &'static str,
    pub ziele_je_rakete: i64,
    pub verteilung: &'static str,
    pub schadensfaktoren: BTreeMap<Einheit, f64>,
}
pub fn zielprofil(art: u8) -> RaketenZielprofil {
    RaketenZielprofil {
        primaer: if art == 0 { "Eintreffende feindliche Angriffsrakete" } else { "Vor dem Start ausdrücklich gewählte Zielklasse" },
        sekundaer: "Keine: kein selbstständiger Wechsel der Zielklasse",
        ziele_je_rakete: [1, 16, 4, 4, 16].get(art as usize).copied().unwrap_or(0),
        verteilung: if art == 0 { "Verbraucht sich beim Abfangen. Eine Abfangrakete je gewöhnlicher Angriffsrakete, zwei je Hyperrakete. Salven werden nach Einschlagzeit verarbeitet, bei Gleichstand nach Ereignisnummer." } else { "Gemeinsames Schadensbudget der nicht abgefangenen Raketen, keine Vervielfachung pro Ziel. Maximal Raketenanzahl × Ziele je Rakete innerhalb der gewählten Klasse. Kein Überschaden auf andere Klassen; verschwundene Ziele ergeben keinen Treffer." },
        schadensfaktoren: Einheit::ALLE.into_iter().filter(|e| match art { 1|3 => !e.ist_schiff(), 2 => e.ist_schiff(), 4 => true, _ => false }).map(|e|(e,faktor(art,e) as f64 / 1000.0)).collect(),
    }
}
fn genaue_zeit(t: SimZeit) -> String {
    format!("{}:{:02}", zeittext(t), t.rem_euclid(60))
}
pub fn regeln(r: &crate::Regelwerk) -> Vec<Raketenregel> {
    use Forschung::*;
    use Gut::*;
    let make = |key,
                name,
                silo,
                zivilisation,
                plaetze,
                bau,
                flug,
                je,
                reich,
                schaden,
                abfang,
                planet,
                flotte,
                braucht: Vec<(Forschung, u8)>,
                kosten: Vec<(Gut, i64)>,
                besonderheit| Raketenregel {
        key,
        name,
        silo,
        zivilisation,
        plaetze,
        fertigungsplaetze: crate::parallel_silo::plaetze(art(key).expect("Bekannte Raketenart")),
        bauzeit_sekunden: bau,
        flug_min_sekunden: flug,
        flug_je_system_sekunden: je,
        reichweite_je_silo: reich,
        schaden,
        abfangbedarf: abfang,
        planet,
        flotte,
        braucht: braucht.into_iter().collect(),
        kosten: kosten.into_iter().collect(),
        besonderheit,
        zielprofil: zielprofil(art(key).expect("Bekannte Raketenart")),
    };
    vec![
 make("abfang","Abfangrakete",1,3,1,r.zusatz.raketen_bauzeit_sekunden,0,0,0,0,0,false,false,vec![],r.zusatz.raketen_kosten[0].iter().map(|(g,n)|(*g,*n)).collect(),"Automatischer Schutz des eigenen Planeten aus einem intakten Silo. Eine Rakete je normaler Angriffsrakete, zwei je Hyperrakete."),
 make("interplanetar","Interplanetarrakete",2,3,2,r.zusatz.raketen_bauzeit_sekunden,r.zusatz.raketen_flug_min_sekunden,r.zusatz.raketen_flug_je_system_sekunden,i64::from(r.zusatz.raketen_reichweite_je_silo),r.zusatz.raketen_schaden,1,true,false,vec![(Impulsantrieb,1)],r.zusatz.raketen_kosten[1].iter().map(|(g,n)|(*g,*n)).collect(),"Allzweckschlag gegen eine gewählte Verteidigungsklasse; umgeht Schilde."),
 make("flottenrakete","Flottenrakete",3,3,2,240,60,3,5,10000,1,false,true,vec![(Impulsantrieb,3),(Computertechnik,3)],vec![(Erz,3500),(Kristall,2000),(Deuterium,18000),(Elektronik,300)],"Sehr schneller Abfangschlag gegen erkannte feindliche Anflüge: doppelter Schaden gegen Jäger und beide Bomberklassen, Grundschaden gegen andere Schiffe. Treibstoff wird beim Bau geladen."),
 make("bunkerbrecher","Bunkerbrecher",5,4,3,600,180,6,5,36000,1,true,false,vec![(Hyperraumantrieb,1),(Waffentechnik,6)],vec![(Erz,12000),(Kristall,8000),(Deuterium,30000),(Legierung,500)],"Doppelter Schaden gegen Gauss, Plasma und Planetenschilde; Grundschaden gegen andere Verteidigung. Treibstoff wird beim Bau geladen."),
 make("hyperrakete","Hyperrakete",8,5,4,900,15,1,10,24000,2,true,true,vec![(Hyperraumantrieb,4),(Waffentechnik,10)],vec![(Erz,20000),(Kristall,12000),(Deuterium,60000),(Elektronik,1000),(Legierung,500)],"Extrem schneller Flug; doppelter Schaden gegen schwere Schiffe, Grundschaden gegen Verteidigung; benötigt zwei Abfangraketen. Hoher Treibstoffbedarf wird beim Bau bezahlt.")]
}
pub const ARTEN:[&str;5]=["abfang","interplanetar","flottenrakete","bunkerbrecher","hyperrakete"];
pub fn art(key: &str) -> Result<u8, String> {
    ARTEN.iter()
    .position(|r| *r == key)
    .map(|i| i as u8)
    .ok_or_else(|| "Unbekannte Raketenart".into())
}
pub fn faktor(art: u8, e: Einheit) -> i64 {
    use Einheit::*;
    let jaeger = matches!(
        e,
        LeichterJaeger | SchwererJaeger | Abfangjaeger | Sturmjaeger
    );
    let schwer = matches!(
        e,
        Schlachtschiff
            | Sturmschlachtschiff
            | Traegerschlachtschiff
            | Belagerungsschlachtschiff
            | Zerstoerer
            | Abfangzerstoerer
            | SchwererZerstoerer
    );
    match art {
        2 => {
            if jaeger || matches!(e, Bomber | StrategischerBomber) {
                2000
            } else if schwer {
                1000
            } else {
                1000
            }
        }
        3 => {
            if matches!(
                e,
                Gausskanone | Plasmawerfer | Planetenschild | PlanetenschildII | PlanetenschildIII
            ) {
                2000
            } else {
                1000
            }
        }
        4 => {
            if schwer {
                2000
            } else if !e.ist_schiff() {
                1000
            } else {
                1000
            }
        }
        _ => 1000,
    }
}
/// Wirksame Struktur einer Zieleinheit beim Einschlag (Raketen umgehen Schilde).
pub fn panzerung(r: &crate::Regelwerk, e: Einheit, panzerungsstufe: u8, volk: Volk) -> i64 {
    let f = (1.0 + r.kampf.tech_je_stufe * panzerungsstufe as f64) * r.volk(volk).panzerung;
    mal(r.einh(e).struktur, match r.volk_einheit(volk, e) { Some(v) => f * v.struktur, None => f }).max(1)
}
/// Schaden je Rakete, beim Start mit Waffentechnik und Volksbonus des Schützen festgehalten.
pub fn startschaden(r: &crate::Regelwerk, regel: &Raketenregel, waffentechnik: u8, volk: Volk) -> i64 {
    mal(regel.schaden, (1.0 + r.kampf.tech_je_stufe * waffentechnik as f64) * r.volk(volk).waffen)
}
/// Vollständig zerstörte Einheiten einer Salve; dieselbe Rechnung für Einschlag und Simulator.
pub fn zerstoert(regel: &Raketenregel, art: u8, treffer: i64, schaden: i64, zieltyp: Einheit, armor: i64, vorhanden: i64) -> i64 {
    ((treffer.max(0) as i128 * schaden as i128 * faktor(art, zieltyp) as i128) / (1000 * armor as i128))
        .min(treffer.max(0) as i128 * regel.zielprofil.ziele_je_rakete as i128)
        .min(vorhanden.max(0) as i128) as i64
}
/// Raketenschlag ohne Weltzugriff durchrechnen (Kampfsimulator). Abfangraketen zählen nur bei
/// Planetenzielen und intaktem Silo, wie beim echten Einschlag.
pub fn simulieren(r: &crate::Regelwerk, a: &Value) -> Result<Value, String> {
    let text = |k: &str| a.get(k).and_then(Value::as_str);
    let zahl = |v: Option<&Value>, max: i64| -> Result<i64, String> {
        match v { None | Some(Value::Null) => Ok(0), Some(v) => v.as_i64().filter(|n| (0..=max).contains(n)).ok_or_else(|| format!("Zahl zwischen 0 und {max} erwartet")) }
    };
    let volk = |v: Option<&Value>| -> Result<Volk, String> {
        v.filter(|v| !v.is_null()).map(|v| serde_json::from_value::<Volk>(v.clone()).map_err(|_| "Unbekanntes Volk".to_string())).unwrap_or(Ok(Volk::ALLE[0]))
    };
    let art = art(text("art").unwrap_or("interplanetar"))?;
    if art == 0 { return Err("Abfangraketen werden nicht gestartet; sie verteidigen automatisch den eigenen Planeten".into()); }
    let regel = regeln(r).remove(art as usize);
    let anzahl = zahl(a.get("anzahl"), 100_000)?.max(1);
    let zieltyp = text("zieltyp").and_then(Einheit::aus_name).ok_or("Unbekannte Zielklasse")?;
    let schiff = zieltyp.ist_schiff();
    if (schiff && !regel.flotte) || (!schiff && !regel.planet) {
        return Err(format!("{} trifft {}", regel.name, if regel.planet { "nur Verteidigungsanlagen" } else { "nur Schiffe eines erkannten Anflugs" }));
    }
    let ziel_anzahl = zahl(a.get("ziel_anzahl"), 10_000_000)?;
    let an = a.get("angreifer").cloned().unwrap_or(Value::Null);
    let ve = a.get("verteidiger").cloned().unwrap_or(Value::Null);
    let schaden = startschaden(r, &regel, zahl(an.get("waffentechnik"), 30)? as u8, volk(an.get("volk"))?);
    let armor = panzerung(r, zieltyp, zahl(ve.get("panzerung"), 30)? as u8, volk(ve.get("volk"))?);
    let silo_intakt = ve.get("silo_intakt").and_then(Value::as_bool).unwrap_or(true);
    let abfang = if schiff || !silo_intakt { 0 } else { zahl(ve.get("abfangraketen"), 1_000_000)? };
    let abgefangen = anzahl.min(abfang / regel.abfangbedarf);
    let weg = zerstoert(&regel, art, anzahl - abgefangen, schaden, zieltyp, armor, ziel_anzahl);
    Ok(json!({"art":regel.key,"name":regel.name,"anzahl":anzahl,"zieltyp":zieltyp.name(),"ziel_anzahl":ziel_anzahl,
        "schaden_je_rakete":schaden,"struktur_je_ziel":armor,"faktor":faktor(art, zieltyp) as f64 / 1000.0,
        "abgefangen":abgefangen,"abfangraketen_verbraucht":abgefangen * regel.abfangbedarf,"treffer":anzahl - abgefangen,
        "zerstoert":weg,"verbleibend":ziel_anzahl - weg,"hoechstens_je_rakete":regel.zielprofil.ziele_je_rakete,
        "hinweis":"Raketen umgehen Schilde; nur vollständig zerstörte Einheiten fallen weg. Keine Beute und kein Wiederaufbau. Zerstörte Einheiten bilden Erz- und Kristalltrümmer nach dem Kampfregelwerk."}))
}
impl Welt {
    pub fn raketen_bestand(&self, pid: usize, art: u8) -> i64 {
        if art < 2 {
            self.planeten[pid].raketen[art as usize]
        } else {
            self.ausbau
                .raketensystem
                .lager
                .get(&(pid as PlanetId))
                .map_or(0, |v| v[(art - 2) as usize])
        }
    }
    fn raketen_add(&mut self, pid: usize, art: u8, n: i64) {
        if art < 2 {
            self.planeten[pid].raketen[art as usize] += n;
        } else {
            let key = pid as PlanetId;
            let a = self.ausbau.raketensystem.lager.entry(key).or_default();
            a[(art - 2) as usize] += n;
            if a.iter().all(|n| *n == 0) {
                self.ausbau.raketensystem.lager.remove(&key);
            }
        }
    }
    pub fn silo_kapazitaet(&self, pid: usize) -> i64 {
        self.planeten[pid].gebaeude[Gebaeude::Raketensilo.idx()] as i64
            * self.regeln.zusatz.silo_plaetze_je_stufe
    }
    pub fn taktisch_belegt(&self, pid: usize) -> i64 {
        let rules = regeln(&self.regeln);
        let p = &self.planeten[pid];
        let stock = (0..5)
            .map(|a| self.raketen_bestand(pid, a) * rules[a as usize].plaetze)
            .sum::<i64>();
        stock
            + self
                .ereignisse
                .iter()
                .filter_map(|e| match e.art {
                    EreignisArt::RaketenFertig {
                        planet,
                        besitzer,
                        art,
                        anzahl,
                    } if planet == p.id && besitzer == p.besitzer => {
                        rules.get(art as usize).map(|r| r.plaetze * anzahl)
                    }
                    _ => None,
                })
                .sum::<i64>()
    }
    pub fn taktisch_kosten(&self, a: u8, n: i64) -> Result<[i64; GUETER], String> {
        if !(1..=1_000_000).contains(&n) {
            return Err("Raketenanzahl muss zwischen 1 und 1.000.000 liegen".into());
        }
        let rules = regeln(&self.regeln);
        let r = rules.get(a as usize).ok_or("Unbekannte Raketenart")?;
        let mut k = [0; GUETER];
        for (g, c) in &r.kosten {
            k[g.idx()] = c
                .checked_mul(n)
                .and_then(|v| v.checked_mul(M))
                .ok_or("Kosten zu groß")?;
        }
        Ok(k)
    }
    fn raketen_voraussetzungen(&self, pid: usize, a: u8) -> Result<(), String> {
        let rules = regeln(&self.regeln);
        let r = &rules[a as usize];
        let p = &self.planeten[pid];
        let s = &self.spieler[p.besitzer as usize];
        if p.gebaeude[Gebaeude::Raketensilo.idx()] < r.silo.into()
            || s.stufe < r.zivilisation.into()
        {
            return Err(format!(
                "{} benötigt Silo {} und Zivilisationsstufe {}",
                r.name, r.silo, r.zivilisation
            ));
        }
        if r.braucht
            .iter()
            .any(|(f, n)| s.forschung[f.idx()] < (*n).into())
        {
            return Err(format!("Forschungsvoraussetzungen für {} fehlen", r.name));
        }
        if self.integritaet(pid, Gebaeude::Raketensilo) < 1000 {
            return Err("Raketensilo zuerst vollständig reparieren".into());
        }
        Ok(())
    }
    pub fn taktisch_bauen(
        &mut self,
        sid: SpielerId,
        rolle: Rolle,
        k: Koord,
        key: &str,
        n: i64,
    ) -> Result<String, String> {
        if !(1..=1_000_000).contains(&n){return Err("Raketenanzahl muss zwischen 1 und 1.000.000 liegen".into());}
        let a = art(key)?;
        let pid = self.eigener_planet(sid, k)?;
        let costs = self.produktionskosten(pid,Gebaeude::Raketensilo,self.taktisch_kosten(a,1)?).map(|c|c*n);
        self.raketen_voraussetzungen(pid, a)?;
        let rules = regeln(&self.regeln);
        let r = &rules[a as usize];
        if self.taktisch_belegt(pid) + n * r.plaetze > self.silo_kapazitaet(pid) {
            return Err("Siloplätze einschließlich bezahlter Bauaufträge reichen nicht aus".into());
        }
        if self.planeten[pid].stabilitaet < milli(self.regeln.stabilitaet.unruhen_unter) {
            return Err("Raketenbau bei Unruhen nicht möglich".into());
        }
        let last=self.ereignisse.iter().filter(|e|matches!(e.art,EreignisArt::RaketenFertig{planet,besitzer,..} if planet==pid as PlanetId&&besitzer==sid)).map(|e|e.zeit).max().unwrap_or(self.zeit).max(self.zeit);
        let done = last
            .checked_add((r.bauzeit_sekunden * n*100+self.werk_tempo(pid,Gebaeude::Raketensilo)-1)/self.werk_tempo(pid,Gebaeude::Raketensilo))
            .ok_or("Bauzeit zu groß")?;
        self.abrechnen(pid);
        let topf = Self::topf_fuer(rolle, Topf::Militaer);
        self.zahlbar(pid, &costs, topf)?;
        self.zahlen(pid, &costs, topf);
        self.plane(
            done,
            EreignisArt::RaketenFertig {
                planet: pid as PlanetId,
                besitzer: sid,
                art: a,
                anzahl: n,
            },
        );
        if self.ausbau.industrie_version>0{self.ausbau.raketen_zahlung.insert(self.seq,RaketenZahlung{kosten:costs,topf,dauer:done-last,basis:(r.bauzeit_sekunden*100+self.werk_tempo(pid,Gebaeude::Raketensilo)-1)/self.werk_tempo(pid,Gebaeude::Raketensilo),rest:n,menge:n,lose:vec![]});}
        if self.ausbau.industrie_version>0{self.silo_parallel_sync(pid);}
        Ok(format!(
            "{n} {} auf {k} im Bau; geplante Fertigung in der Silowarteschlange (Grundzeit {})",
            r.name,
            genaue_zeit(done)
        ))
    }
    pub fn raketen_storno(&mut self,sid:SpielerId,k:Koord,auftrag:u64)->Result<String,String>{
        let pid=self.eigener_planet(sid,k)?;
        let event=self.ereignisse.iter().find(|e|e.seq==auftrag&&matches!(e.art,EreignisArt::RaketenFertig{planet,besitzer,..}if planet==pid as u32&&besitzer==sid)).cloned().ok_or("Dieser Silobauauftrag existiert nicht mehr")?;
        let paid=self.ausbau.raketen_zahlung.remove(&auftrag).ok_or("Alter Silobauauftrag ohne Zahlungsbeleg: pausieren oder fertigstellen")?;
        if paid.basis>0{
            let charged=paid.lose.iter().filter(|b|(b.ende-self.zeit).max(0)*4<=b.dauer).map(|b|b.anzahl).sum::<i64>();let refund=paid.rest-charged;
            self.ereignisse.retain(|e|e.seq!=auftrag);self.erstattung(pid,paid.kosten.map(|c|c/paid.menge*refund),paid.topf);
            if let EreignisArt::RaketenFertig{art,..}=event.art{self.produktionsserie_stoppen(pid,"rakete",regeln(&self.regeln)[art as usize].key);}
            self.silo_parallel_sync(pid);return Ok(format!("Silobau abgebrochen: {refund} von {} verbliebenen Stück erstattet; Wiederholung gestoppt",paid.rest));
        }
        let refund=(event.zeit-self.zeit).max(0)*4>paid.dauer;
        let removed_work=(event.zeit-self.zeit).max(0).min(paid.dauer);
        let queue=std::mem::take(&mut self.ereignisse);self.ereignisse=queue.into_iter().filter(|e|e.seq!=auftrag).map(|mut e|{if e.zeit>event.zeit&&matches!(e.art,EreignisArt::RaketenFertig{planet,besitzer,..}if planet==pid as u32&&besitzer==sid){e.zeit-=removed_work;}e}).collect();
        if refund{self.erstattung(pid,paid.kosten,paid.topf);}
        if let EreignisArt::RaketenFertig{art,..}=event.art{self.produktionsserie_stoppen(pid,"rakete",regeln(&self.regeln)[art as usize].key);}
        Ok(if refund{"Silobauauftrag abgebrochen: volle Rohstofferstattung; Wiederholung gestoppt"}else{"Silobauauftrag abgebrochen: ab 75 % keine Rohstofferstattung; Wiederholung gestoppt"}.into())
    }
    pub fn taktisch_fertig(&mut self, pid: usize, sid: SpielerId, a: u8, n: i64) {
        if a > 4 || n <= 0 || !self.planeten.get(pid).is_some_and(|p| p.besitzer == sid) {
            return;
        } // Paid reservations remain valid even if the silo is damaged during production.
        self.raketen_add(pid, a, n);
        self.vorfall(
            sid,
            "raketen",
            format!(
                "{n} {} auf {} fertig",
                regeln(&self.regeln)[a as usize].name,
                self.planeten[pid].koord
            ),
        );
        self.wecke(sid, Rolle::Feldherr, "Raketen fertig", false);
    }
    pub fn taktisch_starten(
        &mut self,
        sid: SpielerId,
        start: Koord,
        ziel: Koord,
        n: i64,
        zieltyp: Einheit,
        key: &str,
        flotte: Option<FlottenId>,
    ) -> Result<String, String> {
        let a = art(key)?;
        let pid = self.eigener_planet(sid, start)?;
        let rules = regeln(&self.regeln);
        let r = &rules[a as usize];
        if n <= 0 || n > 1_000_000 {
            return Err("Ungültige Anzahl: mindestens 1 Rakete".into());
        }
        let bestand = self.raketen_bestand(pid, a);
        if n > bestand {
            return Err(format!("Zu wenige fertige Raketen: {bestand} bereit, {n} angefragt"));
        }
        self.raketen_voraussetzungen(pid, a)?;
        let gegner = if let Some(id) = flotte {
            if !r.flotte || !zieltyp.ist_schiff() {
                return Err(
                    "Dieser Raketentyp bzw. diese Zielklasse ist nicht für Flotten geeignet".into(),
                );
            }
            if !self.sichtbare_angriffe(sid).contains(&id) {
                return Err("Nur erkannte feindliche Anflüge auf eigene oder verbündete Planeten können abgefangen werden".into());
            }
            let f = &self.flotten[&id];
            if f.ziel != ziel {
                return Err("Abfangkoordinate entspricht nicht dem erkannten Anflug".into());
            }
            f.besitzer
        } else {
            if !r.planet || zieltyp.ist_schiff() {
                return Err("Dieser Raketentyp bzw. diese Zielklasse ist nicht für Planetenverteidigung geeignet".into());
            }
            if self.aufklaerungsregeln() && self.planetenwissen(sid, ziel)["bekannt"] != true {
                return Err("Ziel zuerst mit eigener Planetensonde aufklären".into());
            }
            let zp = *self
                .belegung
                .get(&ziel)
                .ok_or("Zielplanet existiert nicht")?;
            self.planeten[zp as usize].besitzer
        };
        if gegner == sid || self.verbuendet(sid, gegner) {
            return Err("Eigene und verbündete Ziele können nicht angegriffen werden".into());
        }
        let target = &self.spieler[gegner as usize];
        if self.zeit < target.schutz_bis {
            return Err("Ziel steht unter Anfängerschutz".into());
        }
        let weak = self.regeln.diplomatie.schwachenschutz_anteil;
        if flotte.is_none()
            && weak > 0.0
            && target.punkte.gesamt() < mal(self.spieler[sid as usize].punkte.gesamt(), weak)
            && !target.angegriffen.contains(&sid)
        {
            return Err("Ziel steht unter Schwachenschutz".into());
        }
        let d = start.system.abs_diff(ziel.system) as i64;
        if start.sektor != ziel.sektor
            || d > r.reichweite_je_silo
                * self.planeten[pid].gebaeude[Gebaeude::Raketensilo.idx()] as i64
        {
            return Err(
                "Raketenreichweite überschritten; nur derselbe Sektor ist erreichbar".into(),
            );
        }
        let ankunft = self
            .zeit
            .checked_add(r.flug_min_sekunden + d * r.flug_je_system_sekunden)
            .ok_or("Ankunft zu spät")?;
        if flotte.is_some_and(|id| ankunft >= self.flotten[&id].ankunft) {
            return Err("Raketen erreichen den Anflug nicht mehr rechtzeitig".into());
        }
        let schaden = startschaden(&self.regeln, &r, self.spieler[sid as usize].forschung[Forschung::Waffentechnik.idx()], self.spieler[sid as usize].volk);
        let cost = self.regeln.wert(&self.taktisch_kosten(a, n)?);
        self.raketen_add(pid, a, -n);
        self.spieler[sid as usize].statistik.verluste += cost;
        self.spieler[sid as usize].statistik.angriffe += 1;
        self.spieler[sid as usize].schutz_bis =
            self.spieler[sid as usize].schutz_bis.min(self.zeit);
        self.bruch_pruefen(sid, gegner);
        if !self.spieler[sid as usize].angegriffen.contains(&gegner) {
            self.spieler[sid as usize].angegriffen.push(gegner);
        }
        let id = self.seq + 1;
        self.ausbau.raketensystem.salven.insert(
            id,
            Salve {
                von: sid,
                gegner,
                ziel,
                flotte,
                art: a,
                anzahl: n,
                zieltyp,
                schaden,
                ankunft,
            },
        );
        self.plane(ankunft, EreignisArt::TaktischeRakete { salve: id });
        self.vorfall(
            gegner,
            "raketen",
            format!(
                "{n} {} im Anflug auf {ziel}; Einschlag {}",
                r.name,
                genaue_zeit(ankunft)
            ),
        );
        self.wecke(gegner, Rolle::Feldherr, "Raketen im Anflug", true);
        Ok(format!(
            "{n} {} gestartet; Einschlag {}",
            r.name,
            genaue_zeit(ankunft)
        ))
    }
    pub fn taktisch_ankunft(&mut self, id: u64) {
        if self.mond_rakete(id) {
            return;
        }
        let Some(s) = self.ausbau.raketensystem.salven.remove(&id) else {
            return;
        };
        let r = regeln(&self.regeln).remove(s.art as usize);
        let valid = self.spieler_aktiv(s.von)
            && self.spieler_aktiv(s.gegner)
            && self.zeit >= self.spieler[s.gegner as usize].schutz_bis
            && !self.verbuendet(s.von, s.gegner)
            && if let Some(fid) = s.flotte {
                self.flotten.get(&fid).is_some_and(|f| {
                    f.besitzer == s.gegner
                        && f.zustand == Flottenzustand::Hinflug
                        && f.ziel == s.ziel
                        && f.mission.feindlich()
                        && f.ankunft > self.zeit
                }) && self.belegung.get(&s.ziel).is_some_and(|p| {
                    self.planeten[*p as usize].besitzer == s.von
                        || self.verbuendet(s.von, self.planeten[*p as usize].besitzer)
                })
            } else {
                self.belegung
                    .get(&s.ziel)
                    .is_some_and(|p| self.planeten[*p as usize].besitzer == s.gegner)
            };
        if !valid {
            let msg=format!("Die {}-Salve auf {} findet kein zulässiges Ziel mehr und zerstört sich ohne Treffer.",r.name,s.ziel);
            self.vorfall(s.von, "raketen", msg.clone());
            self.vorfall(s.gegner, "raketen", msg);
            return;
        }
        let mut intercepted = 0;
        let mut used = 0;
        let available = if let Some(fid) = s.flotte {
            self.flotten[&fid].schiffe[s.zieltyp.idx()]
        } else {
            let pid = self.belegung[&s.ziel] as usize;
            intercepted = if self.integritaet(pid, Gebaeude::Raketensilo) == 1000
                && self.silo_kapazitaet(pid) > 0
            {
                s.anzahl.min(self.planeten[pid].raketen[0] / r.abfangbedarf)
            } else {
                0
            };
            used = intercepted * r.abfangbedarf;
            self.planeten[pid].raketen[0] -= used;
            self.planeten[pid].einheiten[s.zieltyp.idx()]
        };
        let owner = &self.spieler[s.gegner as usize];
        let armor = panzerung(&self.regeln, s.zieltyp, owner.forschung[Forschung::Panzerung.idx()], owner.volk);
        let destroyed = zerstoert(&r, s.art, s.anzahl - intercepted, s.schaden, s.zieltyp, armor, available);
        let salvage_cost = self.regeln.kosten_einheit(s.zieltyp, owner.volk);
        let loss = self
            .regeln
            .wert(&self.regeln.kosten_einheit(s.zieltyp, owner.volk))
            * destroyed
            + if used > 0 {
                self.regeln.wert(&self.taktisch_kosten(0, used).unwrap())
            } else {
                0
            };
        self.spieler[s.gegner as usize].statistik.verluste += loss;
        self.spieler[s.gegner as usize].statistik.verteidigungen += 1;
        // Für die Meldung: angelegte Trümmer und abgeworfene Fracht (vorher nicht genannt).
        let mut tf_neu = [0i64; 2];
        let mut fracht_weg = [0i64; GUETER];
        if destroyed > 0 {
            let field = self.truemmer.entry(s.ziel).or_insert([0,0]);
            for (i,g) in [Gut::Erz,Gut::Kristall].iter().enumerate() {
                let n = mal(salvage_cost[g.idx()] * destroyed,self.regeln.kampf.truemmer_anteil);
                field[i] += n;
                tf_neu[i] = n;
            }
        }
        if let Some(fid) = s.flotte {
            let mut f = self.flotten.remove(&fid).unwrap();
            f.schiffe[s.zieltyp.idx()] -= destroyed;
            let ladung_vorher = f.ladung;
            if f.schiffe.iter().any(|n| *n > 0) {
                let mut capacity = self
                    .flugplan(f.besitzer, f.start as usize, f.ziel, &f.schiffe, 1000)
                    .map_or(0, |p| p.kapazitaet);
                for n in &mut f.ladung {
                    *n = (*n).min(capacity);
                    capacity -= *n;
                }
                for g in 0..GUETER { fracht_weg[g] = ladung_vorher[g] - f.ladung[g]; }
                self.flotten.insert(fid, f);
            } else {
                fracht_weg = ladung_vorher;
            }
            self.ausbau_aufraeumen();
        } else {
            let pid = self.belegung[&s.ziel] as usize;
            self.planeten[pid].einheiten[s.zieltyp.idx()] -= destroyed;
            self.raten_neu(pid);
        }
        // Null Zerstörte: Grund nennen, statt nur „0 … zerstört“ (Spielprobe 20.10.2026).
        let treffer = s.anzahl - intercepted;
        let grund = if destroyed > 0 { String::new() }
            else if available <= 0 { format!(" (kein {} am Ziel)", s.zieltyp) }
            else if treffer <= 0 { " (alle Raketen abgefangen)".to_string() }
            else { format!(" (zu wenig Schaden: {treffer} Treffer wirken {} gegen {} Struktur je {})", treffer as i128 * s.schaden as i128 * faktor(s.art, s.zieltyp) as i128 / 1000, armor, s.zieltyp) };
        let folgen = format!("{}{}",
            if tf_neu[0] >= M || tf_neu[1] >= M { format!(" Trümmerfeld {}.", [(tf_neu[0], "Erz"), (tf_neu[1], "Kristall")].iter().filter(|(m, _)| *m >= M).map(|(m, g)| format!("+{} {g}", ganz(*m))).collect::<Vec<_>>().join(", ")) } else { String::new() },
            if fracht_weg.iter().any(|n| *n >= M) { format!(" Verlorene Fracht: {}.", crate::berichte::fracht_text(&fracht_weg)) } else { String::new() });
        let msg=format!("{} auf {}: {} gestartet, {} abgefangen ({} Abfangraketen), {} {} zerstört{grund}. Keine Beute, kein Wiederaufbau.{folgen}",r.name,s.ziel,s.anzahl,intercepted,used,destroyed,s.zieltyp);
        self.vorfall(s.von, "raketen", msg.clone());
        self.vorfall(s.gegner, "raketen", msg);
        self.wecke(s.gegner, Rolle::Feldherr, "Raketeneinschlag", true);
        self.punkte_neu();
    }
    pub fn silo_sicht(&self, pid: usize) -> Value {
        let p = &self.planeten[pid];
        json!({"fertigung":self.silo_parallel_sicht(pid),"kapazitaet":self.silo_kapazitaet(pid),"belegt":self.taktisch_belegt(pid),"integritaet":self.integritaet(pid,Gebaeude::Raketensilo),"typen":regeln(&self.regeln).into_iter().enumerate().map(|(a,r)|json!({"fertigungsplaetze_je_stueck":crate::parallel_silo::plaetze(a as u8),"kosten_aktuell":Gut::ALLE.iter().map(|g|(g.name(),self.produktionskosten(pid,Gebaeude::Raketensilo,self.taktisch_kosten(a as u8,1).unwrap())[g.idx()] as f64/M as f64)).collect::<std::collections::BTreeMap<_,_>>(),"dauer_aktuell_sekunden":(r.bauzeit_sekunden*100+self.werk_tempo(pid,Gebaeude::Raketensilo)-1)/self.werk_tempo(pid,Gebaeude::Raketensilo),"regel":r,"bestand":self.raketen_bestand(pid,a as u8),"sperre":self.raketen_voraussetzungen(pid,a as u8).err()})).collect::<Vec<_>>(),"warteschlange":self.ereignisse.iter().filter_map(|e|match e.art{EreignisArt::RaketenFertig{planet,besitzer,art,anzahl}if planet==p.id&&besitzer==p.besitzer=>Some(json!({"lose":self.ausbau.raketen_zahlung.get(&e.seq).map(|p|&p.lose),"erstattbare_stueck":self.ausbau.raketen_zahlung.get(&e.seq).filter(|p|p.basis>0).map(|p|p.rest-p.lose.iter().filter(|b|(b.ende-self.zeit).max(0)*4<=b.dauer).map(|b|b.anzahl).sum::<i64>()),"auftrag":e.seq,"stornierbar":self.ausbau.raketen_zahlung.contains_key(&e.seq),"erstattung_prozent":self.ausbau.raketen_zahlung.get(&e.seq).map_or(0,|p|if p.basis>0&&p.rest>0{(p.rest-p.lose.iter().filter(|b|(b.ende-self.zeit).max(0)*4<=b.dauer).map(|b|b.anzahl).sum::<i64>())*100/p.rest}else if (e.zeit-self.zeit).max(0)*4>p.dauer{100}else{0}),"fortschritt_prozent":self.ausbau.raketen_zahlung.get(&e.seq).map(|p|if p.lose.is_empty()||p.rest<=0{0}else{(p.lose.iter().map(|b|b.anzahl*(100-(b.ende-self.zeit).max(0)*100/b.dauer.max(1)).clamp(0,100)).sum::<i64>()/p.rest).clamp(0,100)}),"art":regeln(&self.regeln)[art as usize].key,"anzahl":anzahl,"fertig":e.zeit})),_=>None}).collect::<Vec<_>>()})
    }
    pub fn salven_sicht(&self, sid: SpielerId) -> Value {
        json!(self.ausbau.raketensystem.salven.iter().filter(|(_,s)|s.von==sid||s.gegner==sid).map(|(id,s)|json!({"id":id,"art":regeln(&self.regeln)[s.art as usize].key,"ziel":s.ziel.to_string(),"flotte":s.flotte,"anzahl":s.anzahl,"zieltyp":s.zieltyp,"ankunft":s.ankunft})).collect::<Vec<_>>())
    }
    pub fn raketenplan(&self, sid: SpielerId, a: &Value) -> Result<Value, String> {
        if a["typ"] != "raketen_starten" {
            return Err("Nur Raketenstart kann vorgeschaut werden".into());
        }
        let mut copy = self.clone();
        let (ok, text) = copy.handeln(sid, Rolle::Feldherr, a);
        if !ok {
            return Ok(json!({"status":"blocked","grund":text}));
        }
        let s = copy.ausbau.raketensystem.salven.get(&copy.seq);
        let text = s
            .map(|s| {
                format!(
                    "Start möglich: {} {}. Einschlag {}",
                    s.anzahl,
                    regeln(&self.regeln)[s.art as usize].name,
                    genaue_zeit(s.ankunft)
                )
            })
            .unwrap_or(text);
        Ok(
            json!({"status":"ready","bericht":text,"salve":s.map(|s|json!({"ankunft":s.ankunft,"art":regeln(&self.regeln)[s.art as usize].key,"anzahl":s.anzahl,"ziel":s.ziel.to_string(),"flotte":s.flotte,"zieltyp":s.zieltyp,"schaden_je_rakete":s.schaden})),"hinweis":"Vorschau des Starts. Abfangbestand, Zielbewegung und Panzerung können sich bis zum Einschlag ändern."}),
        )
    }
}
