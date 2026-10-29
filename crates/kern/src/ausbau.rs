//! Explicit V12 content mode. Old snapshots remain in their original rule mode.
use crate::{regeln::EinheitRegel, typen::*, Welt};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Ausbau {
    pub version: u8,
    #[serde(default, skip_serializing_if = "industrie_inaktiv")]
    pub industrie_version: u8,
    /// Ablauf-Version (`crate::ablauf`): 0 = laufende Welten unverändert, 1 = neue Welten ab 17.10.2026.
    #[serde(default, skip_serializing_if = "industrie_inaktiv")]
    pub ablauf: u8,
    /// Technik v1 (`crate::technik`): Antriebswechsel und Plasmatechnik, nur neue Welten; fehlt = laufende Welt.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub technik: Option<std::sync::Arc<crate::technik::TechnikRegeln>>,
    #[serde(default,skip_serializing_if="BTreeMap::is_empty")]
    pub bestandsrest: BTreeMap<PlanetId,[i64;GUETER]>,
    /// Kauforders, deren Hinterlegung aus einem Rollenbudget kam: Order-ID → Topf-Index (Kernwert handel.topf_rueckbuchung).
    #[serde(default,skip_serializing_if="BTreeMap::is_empty")]
    pub order_toepfe: BTreeMap<u32,u8>,
    #[serde(default,skip_serializing_if="BTreeMap::is_empty")]
    pub parallel:BTreeMap<String,Vec<crate::parallel::Los>>,
    #[serde(default,skip_serializing_if="BTreeMap::is_empty")]
    pub flugtermine:BTreeMap<u64,crate::flugkoordination::Flugtermin>,
    #[serde(default,skip_serializing_if="BTreeMap::is_empty")]
    pub rueckflugplaene:BTreeMap<FlottenId,crate::flugkoordination::Rueckflugplan>,
    #[serde(default,skip_serializing_if="BTreeMap::is_empty")]
    pub fluganzeigen:BTreeMap<FlottenId,serde_json::Value>,
    /// 0.4.8: gestrandete Flotten, eigene Rückziele (stranden / zurück an den Orbitort), Angriffe auf gestrandete Flotten.
    #[serde(default,skip_serializing_if="BTreeMap::is_empty")]
    pub orbitflotten:BTreeMap<FlottenId,crate::orbitflotten::Orbitlage>,
    #[serde(default,skip_serializing_if="BTreeMap::is_empty")]
    pub rueckziele:BTreeMap<FlottenId,crate::orbitflotten::Rueckziel>,
    #[serde(default,skip_serializing_if="BTreeMap::is_empty")]
    pub zielflotten:BTreeMap<FlottenId,FlottenId>,
    #[serde(default, skip_serializing_if="crate::boden::Boden::leer")]
    pub boden: crate::boden::Boden,
    #[serde(default, skip_serializing_if="BTreeMap::is_empty")]
    pub raketen_zahlung:BTreeMap<u64,crate::raketen::RaketenZahlung>,
    #[serde(default, skip_serializing_if="BTreeMap::is_empty")]
    pub produktionsserien:BTreeMap<PlanetId,BTreeMap<String,crate::serien::Produktionsserie>>,
    #[serde(default, skip_serializing_if="BTreeMap::is_empty")]
    pub scan_details: BTreeMap<String,serde_json::Value>,
    #[serde(default, skip_serializing_if="BTreeMap::is_empty")]
    pub fertigung_zahlung: BTreeMap<String,Vec<(Option<Topf>,[i64;GUETER])>>,
    #[serde(default, skip_serializing_if="BTreeMap::is_empty")]
    pub kampfrunden: BTreeMap<u64, Vec<serde_json::Value>>,
    /// Herkunft der Angreifer je Kampfbericht (Nr. → je angreifender Flotte Besitzer, Startkoordinate, Start vom Mond).
    #[serde(default, skip_serializing_if="BTreeMap::is_empty")]
    pub kampf_herkunft: BTreeMap<u64, Vec<(SpielerId, Koord, bool)>>,
    /// Kampftechnik je Kampfbericht zum Kampfzeitpunkt (Nr. → je Beteiligtem Waffen-, Schild-, Panzerungsstufe).
    #[serde(default, skip_serializing_if="BTreeMap::is_empty")]
    pub kampf_technik: BTreeMap<u64, Vec<(SpielerId, [u8; 3])>>,
    /// Berichtszusatz je Kampfbericht (Nr. → Ziel Mond, Parteien, Wiederaufbau, Werftdock-Reparatur), `crate::berichte`.
    #[serde(default, skip_serializing_if="BTreeMap::is_empty")]
    pub kampf_zusatz: BTreeMap<u64, serde_json::Value>,
    #[serde(default,skip_serializing_if="crate::raketen::Raketensystem::ist_leer")]
    pub raketensystem: crate::raketen::Raketensystem,
    pub expeditionen: BTreeMap<FlottenId, crate::expedition::Expedition>,
    pub rettungsziele: BTreeMap<FlottenId, FlottenId>,
    pub personen: BTreeMap<PlanetId, Personen>,
    #[serde(default)]
    pub siedler: BTreeMap<FlottenId, Personen>,
    pub passagiere: BTreeMap<FlottenId, Personen>,
    pub einwegsonden: BTreeSet<FlottenId>,
    pub ausbildungen: Vec<Ausbildung>,
    pub wachstumsrest: BTreeMap<PlanetId, [i64; VOELKER]>,
    pub crew_reserve: BTreeMap<String, Personen>,
    /// Cumulative transfers out of civilian population, including subsequently lost crews.
    /// Not a second living population or a source of returning residents.
    pub crew_abgaenge: BTreeMap<SpielerId, Personen>,
    /// Manuell pausierte Aufträge ("bau:<planet>", "werft:<planet>", "bauteile:<planet>", "forschung:<spieler>").
    #[serde(default, skip_serializing_if = "BTreeSet::is_empty")]
    pub pausen: BTreeSet<String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub produktionsregler: BTreeMap<PlanetId, BTreeMap<Gebaeude, u8>>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub forschung_zahlung: BTreeMap<SpielerId, (PlanetId, Option<Topf>, [i64; GUETER])>,
    /// Dysonsphäre und Allianzen v2 (neue Welten); leer bleibt das bisherige JSON bytegleich.
    #[serde(default, skip_serializing_if = "crate::dyson::Dysonzustand::leer")]
    pub dyson: crate::dyson::Dysonzustand,
    /// Selbst gewählte Planetennamen (`crate::verwaltung`): Planet → (Besitzer beim Benennen, Name).
    /// Gilt nur, solange derselbe Spieler den Planeten hält; leer bleibt das JSON bytegleich.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub planetennamen: BTreeMap<PlanetId, (SpielerId, String)>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub schiffangebote: BTreeMap<u64, crate::schiffhandel::Schiffangebot>,
    /// Terraforming mit Terraformer-Schiffen (`crate::terraform`, nur Welten mit Terraformer): Stand je Koordinate.
    /// Leer bleibt das JSON bytegleich.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub terraform: BTreeMap<Koord, crate::terraform::TerraformStand>,
    /// Zielvolk je Terraforming-Flug (Flotte → Volk, für das Klima und Kruste geformt werden).
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub terraform_fluege: BTreeMap<FlottenId, Volk>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Ausbildung {
    pub planet: PlanetId,
    pub volk: Volk,
    pub qualifikation: u8,
    pub anzahl: i64,
    pub fertig: SimZeit,
}
/// Disjoint qualifications: residents never count as both specialists and researchers.
/// Fractional residents remain milli-persons; only multiples of M can travel or train.
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct Personen {
    pub gruppen: [[i64; 3]; VOELKER],
}
impl Personen {
    pub fn gesamt(&self) -> i64 {
        self.gruppen
            .iter()
            .flatten()
            .fold(0i64, |a, n| a.saturating_add(*n))
    }
    pub fn organisch(&self) -> i64 {
        self.gesamt() - self.gruppen[Volk::Syntheten.idx()].iter().sum::<i64>()
    }
    pub fn fachkraefte(&self) -> i64 {
        self.gruppen.iter().map(|g| g[1]).sum()
    }
    pub fn forscher(&self) -> i64 {
        self.gruppen.iter().map(|g| g[2]).sum()
    }
    pub fn add(&mut self, p: &Self) {
        for v in 0..VOELKER {
            for q in 0..3 {
                self.gruppen[v][q] += p.gruppen[v][q];
            }
        }
    }
    pub fn nehmen(&mut self, p: &Self) -> Result<(), String> {
        if (0..VOELKER).any(|v| {
            (0..3).any(|q| {
                !(0..=1_000_000_000_000).contains(&p.gruppen[v][q])
                    || p.gruppen[v][q] % M != 0
                    || p.gruppen[v][q] > self.gruppen[v][q]
            })
        }) {
            return Err("Personengruppe nicht vorhanden oder keine ganzen Reisenden".into());
        }
        for v in 0..VOELKER {
            for q in 0..3 {
                self.gruppen[v][q] -= p.gruppen[v][q];
            }
        }
        Ok(())
    }
    pub fn auswahl(&self, anzahl_milli: i64) -> Self {
        let mut result = Self::default();
        let mut rest = anzahl_milli;
        for q in 0..3 {
            for v in 0..VOELKER {
                let n = (self.gruppen[v][q] / M * M).min(rest);
                result.gruppen[v][q] = n;
                rest -= n;
            }
        }
        result
    }
    /// Deterministic proportional loss without recreating extinct origins or qualifications.
    pub fn begrenzen(&mut self, total: i64) {
        let old = self.gesamt();
        if total >= old {
            return;
        }
        let eligible = self.gruppen.map(|g| g.map(|n| n > 0));
        let mut left = total.max(0);
        for group in &mut self.gruppen {
            for value in group {
                let keep = anteil(*value, total.max(0), old);
                *value = keep;
                left -= keep;
            }
        }
        // At most eleven milli-person rounding units, assigned only to surviving groups.
        for v in 0..VOELKER {
            for q in 0..3 {
                if left > 0 && eligible[v][q] {
                    self.gruppen[v][q] += 1;
                    left -= 1;
                }
            }
        }
    }
}
pub fn schiffsregel(e: Einheit) -> &'static EinheitRegel {
    static RULES: std::sync::OnceLock<[EinheitRegel; 2]> = std::sync::OnceLock::new();
    &RULES.get_or_init(||{
        let make=|name:&str,cargo,crew,cost,tempo|EinheitRegel{
            kosten:BTreeMap::from([(Gut::Erz,cost),(Gut::Kristall,cost/2),(Gut::Legierung,cost/4),(Gut::Elektronik,cost/5)]),
            struktur:18000,schild:100,angriff:40,ladung:cargo,tempo,antrieb:Some(Forschung::Impulsantrieb),verbrauch:100,
            besatzung:crew,ab_stufe:3,werft:4,braucht:BTreeMap::new(),schnellfeuer:BTreeMap::new(),max_anzahl:0,wirkung:name.into()};
        [make("Pathfinder: Aufklärung, 20 wiederverwendbare Bordsonden und Expeditionsnavigation",5000,20,18000,12000),
         make("Bergungsschiff: 200 zusätzliche Personenplätze, 7.500 Fracht und begrenzte Notreparatur",7500,30,14000,7000)]
    })[usize::from(e==Einheit::Bergungsschiff)]
}
impl Welt {
    /// One content-generation cut, not a requirement to restart on every future update.
    pub fn aus_bytes_fuer_spiel(bytes: &[u8]) -> Result<Self, String> {
        if let Some(w)=crate::migration034::laden(bytes)?{return Ok(w);}
        if !bytes.starts_with(b"STERNE12") && !bytes.starts_with(b"STERNE13") && !bytes.starts_with(b"STERNE14") {
            return Err("Dieser Spielstand stammt von vor der Core-Umstellung (Generation 12). Neue Partie erforderlich; die alte Datei bleibt erhalten.".into());
        }
        Self::aus_bytes(bytes)
    }
    pub fn crew_reservieren(&mut self, pid: usize, produkt: crate::welt::Produkt, menge: i64) {
        if self.ausbau.version == 0 || menge == 0 {
            return;
        }
        self.personen_abgleichen(pid);
        let people = self.ausbau.personen.get_mut(&(pid as PlanetId)).unwrap();
        let crew = people.auswahl(menge);
        // Build validation guarantees sufficient whole residents; no second population debit.
        people.nehmen(&crew).expect("validated crew");
        let key = format!("{pid}:{produkt:?}");
        self.ausbau.crew_reserve.entry(key).or_default().add(&crew);
    }
    pub fn crew_erstatten(&mut self, pid: usize, produkt: crate::welt::Produkt, menge: i64) {
        if self.ausbau.version == 0 || menge == 0 {
            return;
        }
        let key = format!("{pid}:{produkt:?}");
        if let Some(reserve) = self.ausbau.crew_reserve.get_mut(&key) {
            let crew = reserve.auswahl(menge);
            reserve.nehmen(&crew).expect("reserved crew");
            self.ausbau
                .personen
                .entry(pid as PlanetId)
                .or_default()
                .add(&crew);
        }
    }
    pub fn crew_fertig(&mut self, pid: usize, produkt: crate::welt::Produkt) {
        if self.ausbau.version == 0 {
            return;
        }
        let amount = match produkt {
            crate::welt::Produkt::Einheit(e) => self.regeln.einh(e).besatzung * M,
            _ => 0,
        };
        if let Some(p) = self
            .ausbau
            .crew_reserve
            .get_mut(&format!("{pid}:{produkt:?}"))
        {
            let crew = p.auswahl(amount);
            p.nehmen(&crew).expect("reserved crew");
            self.ausbau
                .crew_abgaenge
                .entry(self.planeten[pid].besitzer)
                .or_default()
                .add(&crew);
        }
    }
    /// Prepaid life support. Inputs are milli-persons, output milli-goods.
    /// Unknown expedition finds reserve both supply kinds for every berth.
    pub fn bordversorgung(
        &self,
        organisch: i64,
        synthetisch: i64,
        stunden: i64,
    ) -> Result<[i64; GUETER], String> {
        if organisch < 0 || synthetisch < 0 || stunden < 0 {
            return Err("Ungültige Bordversorgung".into());
        }
        let mut cost = [0; GUETER];
        cost[Gut::Nahrung.idx()] = i64::try_from(
            (mal(organisch, self.regeln.wirtschaft.nahrung_je_1000) / 1000) as i128
                * stunden as i128,
        )
        .map_err(|_| "Bordversorgung zu groß")?;
        cost[Gut::Deuterium.idx()] = i64::try_from((synthetisch as i128 * stunden as i128 + 3) / 4)
            .map_err(|_| "Bordversorgung zu groß")?;
        Ok(cost)
    }
    pub fn personen_senden(
        &mut self,
        sid: SpielerId,
        rolle: Rolle,
        action: &serde_json::Value,
        gruppen: Personen,
    ) -> Result<String, String> {
        if self.ausbau.version == 0 {
            return Err("Personenverkehr benötigt Generation 12".into());
        }
        let base=if action["typ"]=="boden_senden" {&action["flug"]} else {action};
        if base["startart"]=="mond" || base["zielart"]=="mond" {return Err("Monde besitzen keine zivile Bevölkerung; Personen starten und landen auf Planeten".into());}
        let a: crate::aktion::Aktion = serde_json::from_value(base.clone()).map_err(|e| e.to_string())?;
        let (start, ziel, ships, sigma, mission) = match a {
            crate::aktion::Aktion::FlotteSenden {
                start,
                ziel,
                mission,
                schiffe,
                geschwindigkeit,
                ..
            } if matches!(mission,Mission::Transport|Mission::Stationieren) => (start, ziel, schiffe, geschwindigkeit,mission),
            _ => return Err("Personenverkehr benötigt einen Transportauftrag".into()),
        };
        let pid = self.eigener_planet(sid, start)?;
        if mission==Mission::Stationieren {self.eigener_planet(sid,ziel)?;}
        let mut counts = [0; SCHIFFE];
        for (e, n) in ships {
            if !e.ist_schiff() {
                return Err("Keine Verteidigung transportieren".into());
            }
            counts[e.idx()] = n;
        }
        let plan = self.flugplan(sid, pid, ziel, &counts, milli(sigma))?;
        plan.pruefe_zuladung(0, gruppen.gesamt() / M)?;
        let mut check = self.ausbau.personen[&(pid as PlanetId)].clone();
        check.nehmen(&gruppen)?;
        if gruppen.gesamt() == 0 {
            return Err("Keine Passagiere gewählt".into());
        }
        if check.gesamt() < 500 * M {
            return Err("Am Start müssen mindestens 500 Einwohner bleiben".into());
        }
        let mut next = self.clone();
        let fid = next.naechste_flotte;
        let (ok, why) = next.handeln(sid, rolle, action);
        if !ok {
            return Err(why);
        }
        // Food and mobile power are prepaid separately for both legs; no capacity conversion.
        let retour=if mission==Mission::Stationieren{0}else{self.flugplan(sid,pid,ziel,&counts,milli(base["rueckgeschwindigkeit"].as_f64().unwrap_or(sigma)))?.dauer};
        let hours = (plan.dauer + retour + STUNDE - 1) / STUNDE;
        let supply = next.bordversorgung(
            gruppen.organisch(),
            gruppen.gesamt() - gruppen.organisch(),
            hours,
        )?;
        next.abrechnen(pid);
        next.zahlbar(pid, &supply, None)?;
        next.zahlen(pid, &supply, None);
        next.ausbau.personen.insert(pid as PlanetId, check.clone());
        next.planeten[pid].bevoelkerung = check.gesamt();
        next.ausbau.passagiere.insert(fid, gruppen);
        next.raten_neu(pid);
        *self = next;
        Ok(format!(
            "Personentransport {fid} gestartet; Verpflegung für Hin- und Rückflug reserviert"
        ))
    }
    pub fn personen_landen(&mut self, fid: FlottenId, pid: usize) {
        self.boden_landen(fid,pid);
        // The legacy scalar settlers are added by the caller; preserve their identities here.
        if let Some(people) = self.ausbau.siedler.remove(&fid) {
            self.ausbau
                .personen
                .entry(pid as PlanetId)
                .or_default()
                .add(&people);
        }
        self.ausbau.einwegsonden.remove(&fid);
        if let Some(e) = self.ausbau.expeditionen.get_mut(&fid) {
            e.status = "heimgekehrt".into();
        }

        if let Some(people) = self.ausbau.passagiere.remove(&fid) {
            self.planeten[pid].bevoelkerung += people.gesamt();
            self.ausbau
                .personen
                .entry(pid as PlanetId)
                .or_default()
                .add(&people);
            let excess = (self.planeten[pid].bevoelkerung - self.planeten[pid].wohnraum).max(0);
            self.vorfall(
                self.planeten[pid].besitzer,
                "rettung",
                format!(
                    "{} Personen gelandet. Fehlender Wohnraum: {} Plätze.",
                    people.gesamt() / M,
                    (excess + M - 1) / M
                ),
            );
        }
    }
    /// Wachstumsprognose für die Oberfläche: dieselbe Rechnung wie `personen_tick`, ohne etwas zu verändern, mit den
    /// einzelnen Bremsen in Promille (Karl 19.10.2026: „klar sehen, ob Wohnblocks, Konsumgüter, Nahrung, Akademien oder
    /// Labore fehlen, ohne raten zu müssen“). `je_tag` in Personen je Spieltag (negativ = Rückgang).
    pub fn wachstum_prognose(&self, pid: usize) -> serde_json::Value {
        let p = &self.planeten[pid];
        let Some(old) = self.ausbau.personen.get(&p.id) else { return serde_json::Value::Null };
        let w = &self.regeln.wirtschaft;
        let st = &self.regeln.stabilitaet;
        let total = old.gesamt();
        let housing = p.wohnraum.max(M);
        let food_need = mal(old.organisch(), w.nahrung_je_1000) / 1000;
        let food = if food_need == 0 { 1000 } else {
            anteil(self.bestand_jetzt(pid)[Gut::Nahrung.idx()] + p.rate[Gut::Nahrung.idx()].max(0), 1000, food_need).min(1000)
        };
        let energy = if p.energie_verbrauch == 0 { 1000 } else { anteil(p.energie_erzeugung, 1000, p.energie_verbrauch).min(1000) };
        let stable = anteil(p.stabilitaet - milli(st.wachstum_null_unter), 1000, milli(st.wachstum_voll_ab - st.wachstum_null_unter).max(1)).clamp(0, 1000);
        let frueh = self.fruehphase_faktor(p.besitzer) as i128;
        let xeno = self.xeno_planet(p.koord);
        let (mut klima_summe, mut versorgung_summe, mut gewicht) = (0i128, 0i128, 0i128);
        let mut gruppen = Vec::new(); // (Volk, Rate je Stunde, Versorgung, Eignung)
        let mut menge = [0i128; Volk::ALLE.len()]; // Personen × FX
        for volk in Volk::ALLE {
            let v = volk.idx();
            let amount: i64 = old.gruppen[v].iter().sum();
            if amount == 0 { continue; }
            let k = self.planet_eignung(p.besitzer, p.koord, volk);
            let rate = fx(w.wachstum_je_tag * self.regeln.volk(volk).wachstum) / 24 * frueh;
            let supply = if volk == Volk::Syntheten { energy } else { food };
            let hunger = milli(w.hunger_min) + anteil(1000 - milli(w.hunger_min), supply, 1000);
            klima_summe += amount as i128 * k.wachstum_promille as i128;
            versorgung_summe += amount as i128 * hunger.clamp(-1000, 1000) as i128;
            gewicht += amount as i128;
            menge[v] = amount as i128 * FX as i128;
            gruppen.push((v, rate, hunger, k));
        }
        // Wie personen_tick Stunde für Stunde über einen Spieltag: bei 90 % Grundwert (vor Stufe III doppelt) wächst die
        // Bevölkerung innerhalb des Tages spürbar mit, eine lineare Hochrechnung der ersten Stunde lag 40 % zu tief.
        let start: i128 = menge.iter().sum();
        for _ in 0..24 {
            let total = (menge.iter().sum::<i128>() / FX as i128) as i64;
            let mut neu = menge;
            for (v, rate, hunger, k) in &gruppen {
                let amount = menge[*v] / FX as i128;
                if amount <= 0 { continue; }
                let mut delta = amount * rate * (housing - total) as i128 / housing as i128;
                if total <= housing { delta = delta * (*hunger).max(0) as i128 / 1000 * stable as i128 / 1000; }
                if *hunger < 0 { delta = delta.min(amount * rate * *hunger as i128 / 1000); }
                if delta > 0 { delta = delta * k.wachstum_promille as i128 / 1000; if xeno { delta = delta * self.regeln.kw().klima.xeno_wachstum_promille as i128 / 1000; } }
                if !k.bewohnbar { delta = delta.min(-(amount * FX as i128 * k.klimaverlust_promille_je_stunde as i128 / 1000)); }
                neu[*v] = (neu[*v] + delta).max(0);
            }
            menge = neu;
        }
        let g = gewicht.max(1);
        let wohnraum = if housing > 0 { ((housing - total) as i128 * 1000 / housing as i128) as i64 } else { 0 };
        let je_tag = ((menge.iter().sum::<i128>() - start) / FX as i128 / M as i128) as i64;
        let (klima, versorgung) = ((klima_summe / g) as i64, (versorgung_summe / g) as i64);
        // Hauptbremse: die stärkste unter 1000 ‰ (Wohnraum ab weniger als 20 % frei)
        let mut bremsen = vec![("wohnraum", if wohnraum < 200 { wohnraum * 5 } else { 1000 }), ("versorgung", versorgung), ("stabilitaet", stable), ("klima", klima.min(1000))];
        bremsen.sort_by_key(|b| b.1);
        let haupt = if bremsen[0].1 < 900 { bremsen[0].0 } else { "" };
        serde_json::json!({"je_tag": je_tag, "wohnraum_frei_promille": wohnraum, "versorgung_promille": versorgung,
            "stabilitaet_promille": stable, "klima_promille": klima, "xeno": xeno, "haupt_bremse": haupt,
            "stabilitaet_voll_ab": st.wachstum_voll_ab, "stabilitaet_null_unter": st.wachstum_null_unter})
    }
    pub fn personen_tick(&mut self, pid: usize) {
        self.personen_abgleichen(pid);
        let p = self.planeten[pid].clone();
        let w = &self.regeln.wirtschaft;
        let st = &self.regeln.stabilitaet;
        let old = self.ausbau.personen[&p.id].clone();
        let mut people = old.clone();
        let total = old.gesamt();
        let housing = p.wohnraum.max(M);
        let food_need = mal(old.organisch(), w.nahrung_je_1000) / 1000;
        let food = if food_need == 0 {
            1000
        } else {
            anteil(
                self.bestand_jetzt(pid)[Gut::Nahrung.idx()] + p.rate[Gut::Nahrung.idx()].max(0),
                1000,
                food_need,
            )
            .min(1000)
        };
        let energy = if p.energie_verbrauch == 0 {
            1000
        } else {
            anteil(p.energie_erzeugung, 1000, p.energie_verbrauch).min(1000)
        };
        let stable = anteil(
            p.stabilitaet - milli(st.wachstum_null_unter),
            1000,
            milli(st.wachstum_voll_ab - st.wachstum_null_unter).max(1),
        )
        .clamp(0, 1000);
        let klima = Volk::ALLE.map(|v|self.planet_eignung(p.besitzer,p.koord,v));
        let xeno=self.xeno_planet(p.koord);
        let frueh = self.fruehphase_faktor(p.besitzer) as i128;
        let rest = self.ausbau.wachstumsrest.entry(p.id).or_default();
        for volk in Volk::ALLE {
            let v = volk.idx();
            let amount: i64 = old.gruppen[v].iter().sum();
            if amount == 0 {
                rest[v] = 0;
                continue;
            }
            let rate = fx(w.wachstum_je_tag * self.regeln.volk(volk).wachstum) / 24 * frueh;
            let supply = if volk == Volk::Syntheten {
                energy
            } else {
                food
            };
            let hunger = milli(w.hunger_min) + anteil(1000 - milli(w.hunger_min), supply, 1000);
            let mut delta = amount as i128 * rate * (housing - total) as i128 / housing as i128;
            if total <= housing {
                delta = delta * hunger.max(0) as i128 / 1000 * stable as i128 / 1000;
            }
            if hunger < 0 {
                delta = delta.min(amount as i128 * rate * hunger as i128 / 1000);
            }
            if delta > 0 { delta = delta * klima[v].wachstum_promille as i128 / 1000; if xeno {delta=delta*self.regeln.kw().klima.xeno_wachstum_promille as i128/1000;} }
            if !klima[v].bewohnbar {
                delta = delta.min(-(amount as i128 * FX * klima[v].klimaverlust_promille_je_stunde as i128 / 1000));
            }
            delta += rest[v] as i128;
            let change = (delta / FX) as i64;
            rest[v] = (delta % FX) as i64;
            if change >= 0 {
                people.gruppen[v][0] += change;
            } else {
                let mut one = Personen::default();
                one.gruppen[v] = people.gruppen[v];
                one.begrenzen((amount + change).max(0));
                people.gruppen[v] = one.gruppen[v];
            }
        }
        self.planeten[pid].bevoelkerung = people.gesamt();
        self.ausbau.personen.insert(p.id, people);
        // Paid schooling completes only for residents still present, never recreates losses.
        let mut pending = Vec::new();
        for a in std::mem::take(&mut self.ausbau.ausbildungen) {
            if a.planet != p.id || a.fertig > self.zeit {
                pending.push(a);
                continue;
            }
            let g = &mut self.ausbau.personen.get_mut(&p.id).unwrap().gruppen[a.volk.idx()];
            let available = (g[0] / M).min(a.anzahl);
            g[0] -= available * M;
            g[a.qualifikation as usize] += available * M;
        }
        self.ausbau.ausbildungen = pending;
        if self.ablauf_v1() {
            self.grundausbildung_tick(pid);
        }
    }
    pub fn akademie_sicht(&self, pid: usize) -> serde_json::Value {
        let p=&self.planeten[pid];
        let jobs=self.ausbau.ausbildungen.iter().filter(|a|a.planet==p.id).collect::<Vec<_>>();
        let occupied: i64=jobs.iter().map(|a|a.anzahl).sum();
        let places=if self.integritaet(pid,Gebaeude::Akademie)==1000 {p.gebaeude[Gebaeude::Akademie.idx()] as i64*20}else{0};
        let people=self.ausbau.personen.get(&p.id).cloned().unwrap_or_default();
        let candidates:Vec<_>=Volk::ALLE.iter().map(|v| {
            let reserved:i64=jobs.iter().filter(|a|a.volk==*v).map(|a|a.anzahl).sum();
            serde_json::json!({"volk":v.name(),"frei":(people.gruppen[v.idx()][0]/M-reserved).max(0)})
        }).collect();
        let fk_elektronik=if self.ablauf_v1(){0}else{5};
        let frueh=self.fruehphase_faktor(p.besitzer)>1;
        let (konsum,fk_elektronik,fo_elektronik)=if frueh{(0,0,0)}else{(10,fk_elektronik,10)};
        serde_json::json!({"plaetze":places,"belegt":occupied,"frei":(places-occupied).max(0),"kandidaten":candidates,
            "tempo":self.werk_tempo(pid,Gebaeude::Akademie),"ruht":self.werk_ruht(pid,Gebaeude::Akademie),
            "forscher_erlaubt":p.gebaeude[Gebaeude::Labor.idx()]>0,"auftraege":jobs,
            "kosten_je_person":{"fachkraft":{"konsumgut":konsum,"elektronik":fk_elektronik},"forscher":{"konsumgut":konsum,"elektronik":fo_elektronik}},
            "personal_ab_stufe":crate::planer::wirtschaft::PERSONAL_AB_STUFE,"fruehphase":frueh,
            "grundausbildung":self.grundausbildung_sicht(pid)})
    }
    /// Vor Zivilisationsstufe III ×2 (Wachstum, Grundausbildung; planer::wirtschaft::FRUEHPHASE_FAKTOR), sonst 1.
    pub(crate) fn fruehphase_faktor(&self, besitzer: SpielerId) -> i64 {
        if self.ausbau.version > 0 && self.spieler.get(besitzer as usize).is_some_and(|s| s.stufe < crate::planer::wirtschaft::PERSONAL_AB_STUFE) {
            crate::planer::wirtschaft::FRUEHPHASE_FAKTOR
        } else { 1 }
    }
    pub fn ausbilden(
        &mut self,
        sid: SpielerId,
        planet: Koord,
        volk: Volk,
        qualifikation: u8,
        anzahl: i64,
    ) -> Result<String, String> {
        if self.ausbau.version == 0 {
            return Err("Dauerhafte Ausbildung benötigt Ausbau V12".into());
        }
        if !(1..=2).contains(&qualifikation) || !(1..=1000).contains(&anzahl) {
            return Err("Qualifikation 1=Fachkraft/2=Forscher, 1–1000 Personen".into());
        }
        let pid = self.eigener_planet(sid, planet)?;
        let p = &self.planeten[pid];
        let reserved: i64 = self
            .ausbau
            .ausbildungen
            .iter()
            .filter(|a| a.planet == p.id && a.volk == volk)
            .map(|a| a.anzahl)
            .sum();
        if self.ausbau.personen[&p.id].gruppen[volk.idx()][0] / M - reserved < anzahl {
            return Err("Nicht genügend freie unqualifizierte Bewohner".into());
        }
        let slots = p.gebaeude[Gebaeude::Akademie.idx()] as i64 * 20;
        let occupied: i64 = self
            .ausbau
            .ausbildungen
            .iter()
            .filter(|a| a.planet == p.id)
            .map(|a| a.anzahl)
            .sum();
        if occupied + anzahl > slots || self.integritaet(pid, Gebaeude::Akademie) < 1000 {
            return Err("Zu wenige intakte Akademieplätze (20 je Stufe)".into());
        }
        if qualifikation == 2 && p.gebaeude[Gebaeude::Labor.idx()] == 0 {
            return Err("Forscherausbildung benötigt ein Labor".into());
        }
        let mut cost = [0; GUETER];
        cost[Gut::Konsumgut.idx()] = anzahl * 10 * M;
        // Ablauf v1: Fachkräfte ohne Elektronik (Elektronik entsteht nur mit Fachkräften), Forscher wie bisher.
        let elektronik_je_person = if self.ablauf_v1() && qualifikation == 1 { 0 } else { qualifikation as i64 * 5 };
        cost[Gut::Elektronik.idx()] = anzahl * elektronik_je_person * M;
        cost=self.produktionskosten(pid,Gebaeude::Akademie,cost);
        // Vor Stufe III ohne Konsumgüter und Elektronik (die entstehen erst ab Stufe II; Karl 23.10.2026).
        if self.fruehphase_faktor(self.planeten[pid].besitzer) > 1 { cost = [0; GUETER]; }
        self.abrechnen(pid);
        self.zahlbar(pid, &cost, None)?;
        self.zahlen(pid, &cost, None);
        self.ausbau.ausbildungen.push(Ausbildung {
            planet: pid as PlanetId,
            volk,
            qualifikation,
            anzahl,
            fertig: self.zeit + qualifikation as i64 * TAG*100/self.werk_tempo(pid,Gebaeude::Akademie),
        });
        Ok("Ausbildung bezahlt und begonnen; Abschluss nach 24/48 Spielstunden".into())
    }
    pub fn ausbau_aktivieren(&mut self) -> Result<(), String> {
        if self.ausbau.version == 1 {
            self.industrie_aktivieren();
            return Ok(());
        }
        if self.ausbau.version != 0 {
            return Err("Unbekannte Ausbauversion".into());
        }
        self.flottendoktrin_aktivieren();
        let r = std::sync::Arc::make_mut(&mut self.regeln);
        // Komfortstand 0.4.6: Kreuzer als frühe Eskorte; archivierte Doktrin v1 bleibt unverändert.
        if let Some(e)=r.einheiten.get_mut(&Einheit::Kreuzer){if e.ab_stufe==4&&e.werft==4{e.ab_stufe=3;e.werft=3;}}
        // Komfortstand 0.4.7 (Karl 24.10.2026: „Abbau von Asteroidengürteln muss in Stufe 2 schon möglich sein,
        // Baubedingungen reduzieren“): Bergbauschiff neuer Welten wie in regeln/flottenausbau.json (Stufe II, günstiger).
        // Nur bei Spielbeginn; umgestellte alte Spielstände (Migration) und die archivierte Doktrin v1 bleiben unverändert.
        if self.zeit==0 {if let Some(e)=r.einheiten.get_mut(&Einheit::Bergbauschiff){if e.ab_stufe==3&&e.werft==2{*e=crate::flottenausbau::basis(Einheit::Bergbauschiff).clone();}}}
        for e in [Einheit::Pathfinder, Einheit::Bergungsschiff] {
            r.einheiten.insert(e, schiffsregel(e).clone());
            let mut profile = r.flottendoktrin[&Einheit::Handelsfrachter].clone();
            profile.klasse = "erkundung".into();
            profile.rolle = schiffsregel(e).wirkung.clone();
            profile.personenplaetze = if e == Einheit::Bergungsschiff { 200 } else { 0 };
            profile.voraussetzungen =
                BTreeMap::from([(Forschung::Impulsantrieb, 2), (Forschung::Astrophysik, 2)]);
            r.flottendoktrin.insert(e, profile);
        }
        // This content identity changes once here. Loading keeps the saved identity;
        // future compatible updates must not turn a rule hash into a restart condition.
        {
            use sha2::{Digest, Sha256};
            let mut h = Sha256::new();
            h.update(r.hash.as_bytes());
            h.update(b"core-generation-12-balance-1");
            h.update(serde_json::to_vec(&(&r.einheiten, &r.flottendoktrin)).expect("rules"));
            r.hash = h.finalize().iter().map(|b| format!("{b:02x}")).collect();
        }
        for p in &self.planeten {
            let volk = self.spieler[p.besitzer as usize].volk;
            let mut people = Personen::default();
            people.gruppen[volk.idx()][0] = p.bevoelkerung;
            // Preserve already effective specialist capacity once, without inventing additional residents.
            let qualified = p.fk_verfuegbar.min(p.bevoelkerung) / M * M;
            let research = (qualified / 5 / M) * M;
            people.gruppen[volk.idx()][0] -= qualified;
            people.gruppen[volk.idx()][1] = qualified - research;
            people.gruppen[volk.idx()][2] = research;
            self.ausbau.personen.insert(p.id, people);
        }
        self.ausbau.version = 1;
        // Neue Welt (Zeit 0): Ablauf-Version 1; umgestellte alte Spielstände (Migration) bleiben bei 0.
        self.ablauf_aktivieren();
        self.industrie_aktivieren();
        self.klima_angleichen();
        for p in 0..self.planeten.len() {
            self.raten_neu(p);
        }
        Ok(())
    }
    pub fn personen_abgleichen(&mut self, pid: usize) {
        if self.ausbau.version == 0 {
            return;
        }
        let p = &self.planeten[pid];
        let people = self.ausbau.personen.entry(p.id).or_default();
        let total = people.gesamt();
        if total > p.bevoelkerung {
            people.begrenzen(p.bevoelkerung);
        } else if total < p.bevoelkerung {
            people.gruppen[self.spieler[p.besitzer as usize].volk.idx()][0] +=
                p.bevoelkerung - total;
        }
    }
    pub(crate) fn ausbau_aufraeumen(&mut self) {
        self.ausbau.rueckflugplaene.retain(|id,_|self.flotten.contains_key(id));
        self.ausbau.fluganzeigen.retain(|id,_|self.flotten.contains_key(id));
        self.ausbau.orbitflotten.retain(|id,_|self.flotten.contains_key(id));
        self.ausbau.rueckziele.retain(|id,_|self.flotten.contains_key(id));
        self.ausbau.zielflotten.retain(|id,_|self.flotten.contains_key(id));
        self.ausbau.terraform_fluege.retain(|id,_|self.flotten.get(id).is_some_and(|f|f.zustand==crate::welt::Flottenzustand::Hinflug));
        if self.ausbau.version == 0 {
            return;
        }
        self.ausbau
            .passagiere
            .retain(|id, _| self.flotten.contains_key(id));
        self.ausbau
            .siedler
            .retain(|id, _| self.flotten.contains_key(id));
        self.ausbau
            .einwegsonden
            .retain(|id| self.flotten.contains_key(id));
        self.ausbau.rettungsziele.retain(|id, _| {
            self.flotten
                .get(id)
                .is_some_and(|f| f.zustand != crate::welt::Flottenzustand::Rueckflug)
        });
        let rocket_ids=self.ereignisse.iter().map(|e|e.seq).collect::<BTreeSet<_>>();self.ausbau.raketen_zahlung.retain(|id,_|rocket_ids.contains(id));
        self.miliz_aufraeumen();
        let troop_fleets=self.ausbau.boden.flotten.keys().copied().collect::<Vec<_>>();for fid in troop_fleets{self.boden_kapazitaet(fid);}
        self.ausbau.boden.flotten.retain(|id,_|self.flotten.contains_key(id));
        self.ausbau.boden.letzte_runde.retain(|id,_|self.flotten.contains_key(id));
        self.ausbau.boden.wunden.retain(|id,_|self.flotten.contains_key(id));
        let mut deadlines = Vec::new();
        for (id, e) in &mut self.ausbau.expeditionen {
            e.rettungen
                .retain(|id, _| self.ausbau.rettungsziele.contains_key(id));
            if e.status == "vermisst" {
                let end = (e
                    .rettungen
                    .values()
                    .copied()
                    .max()
                    .unwrap_or(0)
                    .max(e.frist.unwrap_or(0))
                    + 1)
                .max(self.zeit);
                if end < e.naechstes {
                    e.naechstes = end;
                    deadlines.push((*id, end));
                }
            }
        }
        for (id, end) in deadlines {
            self.plane(
                end,
                crate::welt::EreignisArt::ExpeditionSchritt { flotte: id },
            );
        }
    }
    pub fn personen_validieren(&self) -> Result<(), String> {
        if self.ausbau.version != 1 {
            return Err("Unbekannte Ausbauversion".into());
        }
        let valid = |p: &Personen| {
            p.gruppen
                .iter()
                .flatten()
                .all(|n| (0..=1_000_000_000_000).contains(n))
        };
        if self
            .ausbau
            .personen
            .values()
            .chain(self.ausbau.passagiere.values())
            .chain(self.ausbau.siedler.values())
            .chain(self.ausbau.crew_reserve.values())
            .chain(self.ausbau.crew_abgaenge.values())
            .any(|p| !valid(p))
        {
            return Err("Ungültige Personengruppe".into());
        }
        if self.ausbau.ausbildungen.iter().any(|a| {
            a.planet as usize >= self.planeten.len()
                || !(1..=2).contains(&a.qualifikation)
                || !(1..=1000).contains(&a.anzahl)
        }) {
            return Err("Ungültige Ausbildung".into());
        }
        for (fid, p) in &self.ausbau.passagiere {
            let f = self.flotten.get(fid).ok_or("Passagiere ohne Flotte")?;
            let seats: i64 = f
                .schiffe
                .iter()
                .enumerate()
                .map(|(i, n)| n * self.regeln.personenplaetze(Einheit::ALLE[i]))
                .sum();
            if p.gesamt() > seats * M {
                return Err("Personenplätze überschritten".into());
            }
        }
        for (fid, p) in &self.ausbau.siedler {
            if self
                .flotten
                .get(fid)
                .is_none_or(|f| p.gesamt() != f.siedler)
            {
                return Err("Siedlerbilanz ohne passende Flotte".into());
            }
        }
        for (fid, e) in &self.ausbau.expeditionen {
            if e.besitzer as usize >= self.spieler.len()
                || e.ereignisse > 8
                || e.risikowurf >= 1_000_000
                || e.totalwurf >= 1000
                || e.ruempfe.values().any(|n| !(0..=8).contains(n))
            {
                return Err("Ungültiger Expeditionszustand".into());
            }
            if ![
                "hinflug",
                "aktiv",
                "vermisst",
                "rueckflug",
                "heimgekehrt",
                "verloren",
            ]
            .contains(&e.status.as_str())
            {
                return Err("Unbekannte Expeditionsphase".into());
            }
            if !["heimgekehrt", "verloren"].contains(&e.status.as_str())
                && self
                    .flotten
                    .get(fid)
                    .is_none_or(|f| f.besitzer != e.besitzer)
            {
                return Err("Expedition ohne eigene Flotte".into());
            }
        }
        for p in &self.planeten {
            let people = self
                .ausbau
                .personen
                .get(&p.id)
                .ok_or("Personenbestand fehlt")?;
            if people.gruppen.iter().flatten().any(|n| *n < 0) || people.gesamt() != p.bevoelkerung
            {
                return Err("Personenbilanz passt nicht zum Planeten".into());
            }
        }
        Ok(())
    }
}

fn industrie_inaktiv(v:&u8)->bool {*v==0}
