//! Flotten: Entfernung, Flugzeit, Treibstoff, Missionen, Kampf am Ziel, Plünderung,
//! Blockade, Invasion, Spionage, Kolonisierung, Recycling und Abbau.

use crate::kampf::{kampf, Gruppe};
use crate::kolonisation::{Rueckweg, Transportziel};
use crate::typen::*;
use crate::welt::*;

#[derive(Clone, Debug)]
pub struct Flugauftrag {
    pub start: Koord,
    pub ziel: Koord,
    pub mission: Mission,
    pub schiffe: [i64; SCHIFFE],
    /// Geschwindigkeitsstufe in Promille, 100 bis 1000.
    pub sigma_pm: i64,
    pub ladung: [i64; GUETER],
    /// Sekunden im Orbit für Halten und Abbau.
    pub haltedauer: i64,
}

#[derive(Clone, Debug)]
pub struct Flugplan {
    pub entfernung: i64,
    pub dauer: i64,
    /// Treibstoff für eine Strecke, in Tausendsteln.
    pub treibstoff: i64,
    pub kapazitaet: i64,
    /// Zusätzliche Personen, ohne Betriebsbesatzung; kein Ersatz für Frachtraum.
    pub personenplaetze: i64,
    pub tempo: i64,
    pub treibstoff_erklaerung: crate::planer::modell::ErklaerterWert,
    pub kapazitaet_erklaerung: crate::planer::modell::ErklaerterWert,
}

impl Flugplan {
    /// Independent limits. Future passenger missions use the same checked plan;
    /// spare freight capacity never creates another seat.
    pub fn pruefe_zuladung(&self, fracht:i64, personen:i64)->Result<(),String>{
        if fracht<0||personen<0{return Err("Zuladung darf nicht negativ sein".into());}
        if fracht>self.kapazitaet{return Err(format!("Ladung {} übersteigt die Kapazität {}",ganz(fracht),ganz(self.kapazitaet)));}
        if personen>self.personenplaetze{return Err(format!("{personen} Personen übersteigen die {} zusätzlichen Personenplätze",self.personenplaetze));}
        Ok(())
    }
}

pub(crate) enum Partei {
    Planet(usize),
    Flotte(FlottenId),
    Mond(crate::monde::MondId),
}

impl Welt {
    pub fn entfernung(&self, a: Koord, b: Koord) -> i64 {
        let f = &self.regeln.flug;
        if a.sektor != b.sektor {
            f.je_sektor * (a.sektor as i64 - b.sektor as i64).abs()
        } else if a.system != b.system {
            f.sektor_basis + f.je_system * (a.system as i64 - b.system as i64).abs()
        } else {
            f.system_basis + f.je_position * (a.position as i64 - b.position as i64).abs()
        }
    }

    pub fn tempo(&self, sid: SpielerId, e: Einheit) -> i64 {
        self.tempo_volk(sid, e, self.spieler[sid as usize].volk)
    }
    /// Tempo mit der Forschung von `sid` und den eigenen Volkswerten von `volk` (Datenblatt für andere Völker).
    pub fn tempo_volk(&self, sid: SpielerId, e: Einheit, volk: Volk) -> i64 {
        // Technik v1: Antriebswechsel per Forschung (crate::technik); sonst Antrieb und Tempo der Einheitenregel.
        let (antrieb, grundtempo, _) = self.regeln.antrieb_wirksam(e, &self.spieler[sid as usize].forschung);
        let Some(antrieb) = antrieb else {
            return 0;
        };
        let bonus = self
            .regeln
            .flug
            .antrieb_bonus
            .get(&antrieb)
            .copied()
            .unwrap_or(0.0);
        let t = mal(
            grundtempo,
            1.0 + bonus * self.spieler[sid as usize].forschung[antrieb.idx()] as f64,
        );
        match self.regeln.volk_einheit(volk, e) { Some(v) if v.tempo != 1.0 => mal(t, v.tempo).max(1), _ => t }
    }

    /// Treibstoffverbrauch eines Schiffs (Technik v1: nach Antriebswechsel), Faktor in der Treibstoffformel.
    pub fn verbrauch(&self, sid: SpielerId, e: Einheit) -> i64 {
        self.verbrauch_volk(sid, e, self.spieler[sid as usize].volk)
    }
    pub fn verbrauch_volk(&self, sid: SpielerId, e: Einheit, volk: Volk) -> i64 {
        let v = self.regeln.antrieb_wirksam(e, &self.spieler[sid as usize].forschung).2;
        match self.regeln.volk_einheit(volk, e) { Some(f) if f.verbrauch != 1.0 => mal(v, f.verbrauch), _ => v }
    }

    /// Flugzeit in Spielsekunden: max(min, k * (10 + 350/sigma * sqrt(10 d / v))).
    pub fn flugdauer(&self, d: i64, v: i64, sigma_pm: i64) -> i64 {
        let f = &self.regeln.flug;
        let w = wurzel(10 * d as i128 * 1_000_000 / v.max(1) as i128) as i64;
        let kw = &self.regeln.kw().flug;
        (f.zeitfaktor * (kw.grundzeit + kw.zeitkonstante * w / sigma_pm.max(1))).max(f.min_sekunden)
    }

    pub fn flugplan(
        &self,
        sid: SpielerId,
        start_pid: usize,
        ziel: Koord,
        schiffe: &[i64; SCHIFFE],
        sigma_pm: i64,
    ) -> Result<Flugplan, String> {
        if !(100..=1000).contains(&sigma_pm) || schiffe.iter().any(|n| *n < 0 || *n > 1_000_000) {
            return Err(
                "ungültige Geschwindigkeit oder Schiffszahl (maximal 1.000.000 je Typ)".into(),
            );
        }
        let p = &self.planeten[start_pid];
        self.flugplan_von(sid, p.koord, p.gebaeude[Gebaeude::Raumhafen.idx()], ziel, schiffe, sigma_pm)
    }
    /// Flugplan ab einer Koordinate (Planet mit seinem Raumhafen oder Orbitort ohne Raumhafen-Ersparnis).
    pub fn flugplan_von(
        &self,
        sid: SpielerId,
        von: Koord,
        raumhafen: u8,
        ziel: Koord,
        schiffe: &[i64; SCHIFFE],
        sigma_pm: i64,
    ) -> Result<Flugplan, String> {
        if !(100..=1000).contains(&sigma_pm) || schiffe.iter().any(|n| *n < 0 || *n > 1_000_000) {
            return Err(
                "ungültige Geschwindigkeit oder Schiffszahl (maximal 1.000.000 je Typ)".into(),
            );
        }
        let r = &self.regeln;
        let sp = &self.spieler[sid as usize];
        let mut tempo = i64::MAX;
        let mut verbrauch = 0i64;
        let mut ladung = 0i64;
        let mut personenplaetze = 0i64;
        for e in 0..SCHIFFE {
            if schiffe[e] > 0 {
                let einheit = Einheit::ALLE[e];
                tempo = tempo.min(self.tempo(sid, einheit));
                verbrauch += schiffe[e] * self.verbrauch(sid, einheit);
                ladung += schiffe[e] * r.ladung_milli(einheit, sp.volk);
                personenplaetze += schiffe[e] * r.personenplaetze(einheit);
            }
        }
        if tempo == i64::MAX || tempo <= 0 {
            return Err("keine Schiffe gewählt".into());
        }
        let d = self.entfernung(von, ziel);
        let dauer = self.flugdauer(d, tempo, sigma_pm);
        let ersparnis = (r.flug.raumhafen_ersparnis * raumhafen as f64)
            .min(r.flug.raumhafen_ersparnis_max);
        let roh = M as i128
            + verbrauch as i128 * M as i128 * d as i128 * (sigma_pm as i128 * sigma_pm as i128)
                / (r.flug.treibstoff_teiler as i128 * 1_000_000);
        let treibstoff_erklaerung = crate::planer::modell::ErklaerterWert::neu(roh as i64,"tausendstel_deuterium_je_strecke")
            .faktor("raumhafen_mit_ersparnisobergrenze",1.0-ersparnis);
        let treibstoff = treibstoff_erklaerung.wert;
        let logistik = 1.0 + r.flug.logistik_bonus * sp.forschung[Forschung::Logistik.idx()] as f64;
        let kapazitaet_erklaerung=crate::planer::modell::ErklaerterWert::neu(ladung,"tausendstel_fracht")
            .faktor("volk_und_logistik",r.volk(sp.volk).ladung*logistik);
        let kapazitaet = kapazitaet_erklaerung.wert;
        Ok(Flugplan {
            entfernung: d,
            dauer,
            treibstoff,
            kapazitaet,
            personenplaetze,
            tempo,
            treibstoff_erklaerung,
            kapazitaet_erklaerung,
        })
    }

    pub fn flottenplaetze(&self, sid: SpielerId) -> usize {
        1 + self.spieler[sid as usize].forschung[Forschung::Computertechnik.idx()] as usize
    }

    /// Erlaubte Kolonien: Generation 12 min(kolonien_max, 14, frei + Stufe der Kolonieforschung) – neue Welten (0.4.7,
    /// Karl 24.10.2026) 2 frei ab Start und je Stufe Terraforming eine mehr, laufende Welten wie bisher je Astrophysikstufe
    /// eine (Kernwerte `kolonien`, Standard frei 0 / Astrophysik); ältere Welten je zwei Astrophysikstufen eine.
    pub fn kolonien_erlaubt(&self, sid: SpielerId) -> usize {
        let kw = &self.regeln.kw().kolonien;
        let stufe = self.spieler[sid as usize].forschung[kw.forschung.idx()] as usize;
        if self.ausbau.version > 0 {
            (kw.frei as usize + stufe).min(self.kolonien_grenze())
        } else if stufe == 0 {
            0
        } else {
            (1 + (stufe - 1) / 2).min(self.regeln.wirtschaft.kolonien_max as usize)
        }
    }

    /// Höchstzahl der Kolonien dieser Welt (unabhängig von der Forschung).
    pub fn kolonien_grenze(&self) -> usize {
        let max = self.regeln.wirtschaft.kolonien_max as usize;
        if self.ausbau.version > 0 { max.min(crate::verwaltung::MAX_PLANETEN - 1) } else { max }
    }

    /// Forschung, deren Stufen weitere Kolonien erlauben (neue Welten Terraforming, laufende Welten Astrophysik).
    pub fn kolonie_forschung(&self) -> Forschung {
        self.regeln.kw().kolonien.forschung
    }

    /// Neue Welten (0.4.7, Karl 24.10.2026): „sonst kann man das Kolonisierungsschiff senden, auch auf den Befehl, aber es wird
    /// nicht kolonisiert“. Kolonie- und Kampfkolonisierungsflüge starten ohne freien Platz; erst die Gründung bzw. die
    /// Übernahme braucht ihn (keine Reservierung beim Start). Laufende Welten (Standard-Kernwerte) prüfen wie bisher beim Start.
    pub fn kolonieplatz_erst_bei_gruendung(&self) -> bool {
        !self.regeln.kw().kolonien.ist_standard()
    }

    /// Kolonieplätze ohne Forschung (neue Welten 2, sonst 0).
    pub fn kolonien_frei(&self) -> u8 {
        if self.ausbau.version > 0 { self.regeln.kw().kolonien.frei } else { 0 }
    }

    /// Nötige Stufe der Kolonieforschung für insgesamt `n` Kolonien, ohne Grenze (Bots und Planung).
    pub fn kolonie_stufe_roh(&self, n: u8) -> u8 {
        if self.ausbau.version > 0 { n.saturating_sub(self.kolonien_frei()) } else { (2 * n as u16).saturating_sub(1).min(u8::MAX as u16) as u8 }
    }

    /// Nötige Stufe der Kolonieforschung für insgesamt `n` Kolonien; None, wenn `n` über der Höchstzahl liegt.
    pub fn kolonie_stufe_fuer(&self, n: usize) -> Option<u8> {
        (n <= self.kolonien_grenze()).then(|| self.kolonie_stufe_roh(n.min(u8::MAX as usize) as u8))
    }

    /// Text für die volle Kolonieobergrenze: was die nächste Kolonie erlaubt.
    pub(crate) fn kolonien_voll_text(&self, sid: SpielerId) -> String {
        let erlaubt = self.kolonien_erlaubt(sid);
        if self.regeln.kw().kolonien.ist_standard() {
            return format!("Astrophysik erlaubt {erlaubt} Kolonien, höhere Stufen erlauben mehr");
        }
        let f = self.kolonie_forschung();
        match self.kolonie_stufe_fuer(erlaubt + 1) {
            Some(n) => format!("Alle {erlaubt} Kolonieplätze sind belegt: {} Stufe {n} erlaubt die nächste Kolonie ({} frei ab Start, je Stufe {} eine mehr, höchstens {})",
                crate::regeltext::forschung_name(f), self.kolonien_frei(), crate::regeltext::forschung_name(f), self.kolonien_grenze()),
            None => format!("Alle {erlaubt} Kolonieplätze sind belegt – mehr als {} Kolonien (mit Heimatwelt {} Planeten) erlaubt diese Welt nicht",
                self.kolonien_grenze(), self.kolonien_grenze() + 1),
        }
    }

    /// Vorwarnzeit eines Planeten in Sekunden.
    pub fn warnzeit(&self, pid: usize) -> i64 {
        if self.aufklaerungsregeln() { return self.online_warnzeit(pid); }
        let k = &self.regeln.kampf;
        (k.warnzeit_basis_minuten
            + k.warnzeit_je_phalanx_minuten
                * self.planeten[pid].gebaeude[Gebaeude::Sensorphalanx.idx()] as i64)
            * MINUTE
    }

    /// Feindliche Flotten im Sensorbereich eigener Planeten und der Planeten von Bündnispartnern.
    pub fn sichtbare_angriffe(&self, sid: SpielerId) -> Vec<FlottenId> {
        let mut v = Vec::new();
        for f in self.flotten.values() {
            if !f.mission.feindlich() || f.zustand != Flottenzustand::Hinflug || f.besitzer == sid {
                continue;
            }
            let Some(zp) = self.belegung.get(&f.ziel) else {
                continue;
            };
            let ziel = &self.planeten[*zp as usize];
            if (ziel.besitzer == sid || self.verbuendet(sid, ziel.besitzer))
                && self.zeit >= f.ankunft - self.warnzeit(*zp as usize)
            {
                v.push(f.id);
            }
        }
        v
    }

    pub(crate) fn blockiert_fuer(&self, pid: usize, sid: SpielerId) -> bool {
        match self.planeten[pid].blockade {
            Some(bf) => self
                .flotten
                .get(&bf)
                .map(|f| {
                    f.besitzer != sid
                        && (!self.kolonisationsregeln_v2()
                            || (f.zustand == Flottenzustand::ImOrbit
                                && !self.verbuendet(sid, f.besitzer)))
                })
                .unwrap_or(false),
            None => false,
        }
    }

    pub fn flotte_senden(
        &mut self,
        sid: SpielerId,
        rolle: Rolle,
        a: Flugauftrag,
    ) -> Result<String, String> {
        self.flotte_senden_intern(sid, rolle, a, None)
    }

    pub(crate) fn flotte_senden_intern(&mut self, sid: SpielerId, rolle: Rolle,
        a: Flugauftrag, sondenziel: Option<FlottenId>) -> Result<String, String> {
        self.flotte_senden_auswahl(sid,rolle,a,sondenziel,None,None)
    }
    pub fn flotte_senden_mit_siedlern(&mut self, sid: SpielerId, rolle: Rolle, a: Flugauftrag, volk: Option<Volk>) -> Result<String,String> {
        self.flotte_senden_auswahl(sid,rolle,a,None,volk,None)
    }
    /// Wie `flotte_senden_mit_siedlern`, dazu die selbst gewählte Siedlermischung [Bewohner, Fachkräfte, Forscher].
    pub fn flotte_senden_mit_mischung(&mut self, sid: SpielerId, rolle: Rolle, a: Flugauftrag, volk: Option<Volk>, mischung: Option<[i64; 3]>) -> Result<String,String> {
        self.flotte_senden_auswahl(sid,rolle,a,None,volk,mischung)
    }
    fn flotte_senden_auswahl(&mut self, sid: SpielerId, rolle: Rolle, a: Flugauftrag, sondenziel: Option<FlottenId>, siedler_volk: Option<Volk>, siedler_mischung: Option<[i64; 3]>) -> Result<String,String> {
        if siedler_volk.is_some() && !matches!(a.mission,Mission::Kolonisieren|Mission::Terraformen) { return Err("Siedlervolk nur bei freier Kolonisation oder als Zielvolk beim Terraformen angeben".into()); }
        if siedler_mischung.is_some() && (a.mission!=Mission::Kolonisieren || self.ausbau.version==0) { return Err("Siedlermischung nur bei freier Kolonisation mit Bevölkerungsgruppen angeben".into()); }
        let r = self.regeln.clone();
        let pid = self.eigener_planet(sid, a.start)?;
        let p = &self.planeten[pid];
        let sp = &self.spieler[sid as usize];
        if p.gebaeude[Gebaeude::Raumhafen.idx()] < 1 || self.integritaet(pid, Gebaeude::Raumhafen) == 0 {
            return Err(format!(
                "Flottenstart braucht einen funktionsfähigen Raumhafen auf {}",
                a.start
            ));
        }
        if !a.schiffe.iter().any(|n| *n > 0) {
            return Err("keine Schiffe gewählt".into());
        }
        for e in 0..SCHIFFE {
            if a.schiffe[e] < 0 || a.schiffe[e] > p.einheiten[e] {
                return Err(format!(
                    "auf {} stehen nur {} x {}",
                    a.start,
                    p.einheiten[e],
                    Einheit::ALLE[e]
                ));
            }
        }
        let plaetze = self.flottenplaetze(sid);
        if self.flotten.values().filter(|f| f.besitzer == sid).count() >= plaetze {
            return Err(format!(
                "alle {plaetze} Flottenplätze sind belegt, Computertechnik erhöht die Zahl"
            ));
        }
        if !(100..=1000).contains(&a.sigma_pm) {
            return Err("geschwindigkeit muss zwischen 0.1 und 1.0 liegen".into());
        }
        if self.system(a.ziel).is_none() || a.ziel.position > r.welt.plaetze_je_system {
            return Err(format!("{} liegt außerhalb der Galaxie", a.ziel));
        }
        if (a.ziel.position == 0 && a.mission != Mission::Abbau && !(self.ausbau.version>0 && a.mission==Mission::Saven))
            || (a.mission == Mission::Abbau && a.ziel.position!=0&&!self.natuerlicher_ring(a.ziel)) {
            return Err("Position 0 ist der Asteroidengürtel: nur die Mission abbau fliegt dorthin, und nur dorthin".into());
        }
        if self.blockiert_fuer(pid, sid) && !(a.mission == Mission::Angriff && a.ziel == a.start) {
            return Err(format!(
                "{} ist blockiert: möglich ist nur ein Angriff auf die Blockadeflotte (Ziel {})",
                a.start, a.start
            ));
        }
        if self.aufklaerungsregeln() {
            let own = sp.planeten.iter().any(|id|self.planeten[*id as usize].koord==a.ziel);
            if matches!(a.mission,Mission::Abbau|Mission::Recyceln) && !self.system_erfasst(sid,a.ziel) {
                return Err("Sonnensystem zuerst mit einer Systemsonde kartieren".into());
            }
            if !own && !matches!(a.mission,Mission::SystemErkunden|Mission::Spionage|Mission::FlottenSpionage|Mission::Saven|Mission::Abbau|Mission::Recyceln|Mission::Terraformen)
                && self.planetenwissen(sid,a.ziel)["bekannt"] != true
                // Invasion v1: Verbündete kennen die Planeten ihrer Partner für Hilfe (halten/verstaerken).
                && !(self.invasion_v1() && a.mission == Mission::Halten
                    && self.belegung.get(&a.ziel).is_some_and(|p| self.verbuendet(sid, self.planeten[*p as usize].besitzer)))
                // Karl 28.10.2026: „Allianzmitglieder haben die Möglichkeit, sich ohne Bericht beliefern zu können.“
                && !(a.mission == Mission::Transport
                    && self.belegung.get(&a.ziel).is_some_and(|p| self.gleiche_allianz(sid, self.planeten[*p as usize].besitzer)))
                // 0.4.8: Wo eine eigene gestrandete Flotte steht, ist das Ziel bekannt (Versorgen, Nachschub).
                && !self.eigene_orbitflotte_an(sid, a.ziel) {
                return Err("Ziel zuerst mit einer eigenen Planetensonde aufklären".into());
            }
        }
        let ziel_pid = self.belegung.get(&a.ziel).map(|x| *x as usize);
        let hat = |e: Einheit| a.schiffe[e.idx()] > 0;
        let mut einweg = false;
        let mut siedler = 0i64;
        let mut haltedauer = 0i64;
        let mut feindlich_gegen: Option<SpielerId> = None;
        let mut terraform_volk: Option<Volk> = None;
        match a.mission {
            Mission::Terraformen => {
                terraform_volk = Some(self.terraform_flug_pruefen(sid, a.ziel, &a.schiffe, siedler_volk)?);
            }
            Mission::SystemErkunden => {
                if !self.aufklaerungsregeln() { return Err("Systemaufklärung benötigt Online-Regeln v1".into()); }
                if (0..SCHIFFE).any(|e|e != Einheit::Spionagesonde.idx() && a.schiffe[e]>0) {
                    return Err("Systemaufklärung verwendet ausschließlich Spionagesonden".into());
                }
            }
            Mission::Saven => {
                if !self.aufklaerungsregeln() { return Err("Saven benötigt Online-Regeln v1".into()); }
                if a.ziel == a.start { return Err("Save-Flug benötigt eine andere Zielkoordinate".into()); }
                if !(0..=72 * STUNDE).contains(&a.haltedauer) { return Err("Save-Wartezeit: 0 bis 72 Spielstunden".into()); }
                haltedauer = a.haltedauer;
            }
            Mission::FlottenSpionage => {
                if !self.aufklaerungsregeln() || sondenziel.is_none() {
                    return Err("Flottensonden mit flotte_ausspaehen starten".into());
                }
                if (0..SCHIFFE).any(|e|e != Einheit::Spionagesonde.idx() && a.schiffe[e]>0) {
                    return Err("Flottenaufklärung verwendet ausschließlich Sonden".into());
                }
            }
            Mission::Angriff
            | Mission::Blockade
            | Mission::Invasion
            | Mission::Bombardieren
            | Mission::Kampfkolonisieren => {
                if !self.kolonisation.aktiv
                    && matches!(
                        a.mission,
                        Mission::Bombardieren | Mission::Kampfkolonisieren
                    )
                {
                    return Err("Mission benötigt Kolonisationsregeln v1".into());
                }
                if self.kolonisation.aktiv && a.mission == Mission::Invasion {
                    return Err("Stabilitäts-Invasion ersetzt: bombardieren und kampfkolonisieren verwenden".into());
                }
                let Some(zp) = ziel_pid else {
                    return Err(format!("auf {} liegt kein Planet", a.ziel));
                };
                let ziel = &self.planeten[zp];
                if self.kolonisationsregeln_v2() {
                    self.offensive_pruefen(sid, zp, a.mission, &a.schiffe)?;
                    feindlich_gegen = self.tatsaechlicher_gegner(zp, sid);
                } else {
                    if ziel.besitzer == sid {
                        if a.mission != Mission::Angriff || !self.blockiert_fuer(zp, sid) {
                            return Err("eigene Planeten kann man nicht angreifen".into());
                        }
                    } else {
                        let gegner = &self.spieler[ziel.besitzer as usize];
                        if self.zeit < gegner.schutz_bis {
                            return Err(format!(
                                "{} steht bis {} unter Anfängerschutz",
                                gegner.name,
                                zeittext(gegner.schutz_bis)
                            ));
                        }
                        let schwach = r.diplomatie.schwachenschutz_anteil;
                        if schwach > 0.0
                            && gegner.punkte.gesamt() < mal(sp.punkte.gesamt(), schwach)
                            && !gegner.angegriffen.contains(&sid)
                        {
                            return Err(format!(
                                "{} ist zu schwach für einen Angriff und hat dich nie angegriffen",
                                gegner.name
                            ));
                        }
                        if a.mission != Mission::Angriff && sp.stufe < 4 {
                            return Err(
                                "Blockade und Invasion gibt es ab Zivilisationsstufe 4".into()
                            );
                        }
                        if a.mission == Mission::Invasion {
                            if ziel.heimat {
                                return Err("Heimatwelten können nicht erobert werden".into());
                            }
                            if !hat(Einheit::Truppentransporter) {
                                return Err("eine Invasion braucht Truppentransporter".into());
                            }
                        }
                        if a.mission == Mission::Kampfkolonisieren {
                            if ziel.heimat {
                                return Err(
                                    "Der ursprüngliche Heimatplanet ist vor Übernahme geschützt"
                                        .into(),
                                );
                            }
                            if Self::kolonieschiff_typ(&a.schiffe).is_none() {
                                return Err("Kampfkolonisierung braucht ein Kolonieschiff".into());
                            }
                            if self.kolonien(sid) >= self.kolonien_erlaubt(sid) {
                                return Err("Kolonielimit erreicht".into());
                            }
                        }
                        feindlich_gegen = Some(ziel.besitzer);
                    }
                }
            }
            Mission::Transport => {
                // 0.4.8: Ohne Planet nur zur eigenen gestrandeten Flotte (Versorgen, z. B. über einem Trümmerfeld).
                if ziel_pid.is_none() && !self.eigene_orbitflotte_an(sid, a.ziel) {
                    return Err(format!("auf {} liegt kein Planet", a.ziel));
                }
            }
            Mission::Stationieren => {
                let zp = self.eigener_planet(sid, a.ziel)?;
                if zp == pid {
                    return Err("Start und Ziel sind derselbe Planet".into());
                }
                einweg = true;
            }
            Mission::Halten => {
                let Some(zp) = ziel_pid else {
                    return Err(format!("auf {} liegt kein Planet", a.ziel));
                };
                let partner = self.planeten[zp].besitzer;
                // Invasion v2: Bodenhilfe für eine verbündete Invasion (Truppen schließen sich den Angreifern an).
                if (partner == sid || !self.verbuendet(sid, partner)) && !self.angriffshilfe_erlaubt(sid, zp) {
                    return Err("Halten geht nur bei einem Partner mit Verteidigungsbündnis oder in derselben Allianz".into());
                }
                haltedauer = a.haltedauer;
                if haltedauer < STUNDE || haltedauer > r.flug.halten_max_stunden * STUNDE {
                    return Err(format!(
                        "haltedauer_stunden muss zwischen 1 und {} liegen",
                        r.flug.halten_max_stunden
                    ));
                }
            }
            Mission::Spionage => {
                if self.aufklaerungsregeln() && !self.system_erfasst(sid,a.ziel) {
                    return Err("Sonnensystem zuerst mit system_erkunden und einer Spionagesonde kartieren".into());
                }
                if (0..SCHIFFE).any(|e| e != Einheit::Spionagesonde.idx() && a.schiffe[e] > 0) {
                    return Err("eine Spionageflotte besteht nur aus Spionagesonden".into());
                }
                if ziel_pid
                    .map(|zp| self.planeten[zp].besitzer == sid)
                    .unwrap_or(false)
                {
                    return Err("eigene Planeten muss man nicht ausspionieren".into());
                }
            }
            Mission::Kolonisieren => {
                self.koloniefracht_pruefen(sid, a.ziel, &a.schiffe, &a.ladung)?;
                if Self::kolonieschiff_typ(&a.schiffe).is_none() {
                    return Err("Kolonisieren braucht ein Kolonieschiff".into());
                }
                if ziel_pid.is_some() && !self.aufklaerungsregeln() {
                    return Err(format!("{} ist schon besiedelt", a.ziel));
                }
                let unterwegs = self
                    .flotten
                    .values()
                    .filter(|f| {
                        f.besitzer == sid
                            && f.mission == Mission::Kolonisieren
                            && f.zustand == Flottenzustand::Hinflug
                    })
                    .count();
                let erlaubt = self.kolonien_erlaubt(sid);
                if !self.kolonieplatz_erst_bei_gruendung() && if self.kolonisationsregeln_v2() {
                    !self.koloniekapazitaet(sid, a.ziel)
                } else {
                    self.kolonien(sid) + unterwegs >= erlaubt
                } {
                    return Err(self.kolonien_voll_text(sid));
                }
                let typ=Self::kolonieschiff_typ(&a.schiffe).unwrap();
                let anzahl=r.flottendoktrin.get(&typ).map(|p|p.siedler).unwrap_or(r.wirtschaft.siedler);
                siedler = anzahl * M;
                if p.bevoelkerung - siedler < 1000 * M {
                    return Err(format!(
                        "für {} Siedler müssen auf {} mindestens 1000 Einwohner zurückbleiben",
                        anzahl, a.start
                    ));
                }
                einweg = true;
            }
            Mission::Recyceln => {
                if !hat(Einheit::Recycler) && !hat(Einheit::Bergungsschiff) {
                    return Err("Trümmerbergung braucht Recycler oder Bergungsschiffe".into());
                }
            }
            Mission::MondZerstoeren => {
                return Err("Mondzerstörung zielt auf einen Mond: flotte_senden mit zielart mond".into());
            }
            Mission::Abbau => {
                if !hat(Einheit::Bergbauschiff) {
                    return Err("Abbau braucht Bergbauschiffe".into());
                }
                if (a.ziel.position==0&&!self.system(a.ziel).map(|s|s.guertel).unwrap_or(false))
                    ||(a.ziel.position>0&&!self.natuerlicher_ring(a.ziel)){
                    return Err(format!(
                        "System {}:{} hat keinen Asteroidengürtel",
                        a.ziel.sektor, a.ziel.system
                    ));
                }
                if a.ziel.position>0&&ziel_pid.is_some_and(|id|self.planeten[id].besitzer!=sid&&!self.verbuendet(sid,self.planeten[id].besitzer)){
                    return Err("Ring einer fremden Welt: erst diplomatischen Zugang herstellen".into());
                }
                haltedauer = a.haltedauer;
                if haltedauer < STUNDE || haltedauer > r.flug.abbau_max_stunden * STUNDE {
                    return Err(format!(
                        "haltedauer_stunden muss zwischen 1 und {} liegen",
                        r.flug.abbau_max_stunden
                    ));
                }
            }
        }

        let plan = self.flugplan(sid, pid, a.ziel, &a.schiffe, a.sigma_pm)?;
        let fracht = a.ladung.iter().try_fold(0i64,|sum,n|sum.checked_add(*n)).ok_or("Ladung ist zu groß")?;
        if a.ladung.iter().any(|m| *m < 0) {
            return Err("ladung darf nicht negativ sein".into());
        }
        plan.pruefe_zuladung(fracht,0)?;
        if matches!(a.mission, Mission::Spionage | Mission::FlottenSpionage | Mission::SystemErkunden) && fracht > 0 {
            return Err("Sonden tragen keine Ladung".into());
        }
        let mut siedlergruppe = crate::ausbau::Personen::default();
        if self.ausbau.version>0 && siedler>0 {
            let volk=siedler_volk.unwrap_or(sp.volk);
            let vorhanden=self.ausbau.personen.get(&(pid as PlanetId)).cloned().unwrap_or_default();
            siedlergruppe=match siedler_mischung {
                // Karl 24.10.2026: gleich nur Forscher, nur Fachkräfte oder normale Bewohner mitschicken können.
                Some(m) => {
                    if m.iter().any(|n|!(0..=1_000_000_000).contains(n)) || m.iter().sum::<i64>()*M!=siedler {
                        return Err(format!("Siedlermischung muss genau {} Siedler ergeben (Bewohner + Fachkräfte + Forscher)",siedler/M));
                    }
                    let mut g=crate::ausbau::Personen::default();
                    for (q,art) in ["Bewohner","Fachkräfte","Forscher"].iter().enumerate() {
                        let da=vorhanden.gruppen[volk.idx()][q]/M;
                        if m[q]>da { return Err(format!("Auf {} gibt es nur {} {} ({})",a.start,da,art,volk.name())); }
                        g.gruppen[volk.idx()][q]=m[q]*M;
                    }
                    g
                }
                None => vorhanden.auswahl_volk(volk,siedler),
            };
            self.siedler_eignung_pruefen(sid,a.ziel,&siedlergruppe,siedler)?;
            let food=mal(siedlergruppe.organisch(),r.wirtschaft.nahrung_je_1000)*24/1000;
            if a.ladung[Gut::Nahrung.idx()]<food{return Err("Koloniefracht muss die tatsächlichen organischen Siedler 24 Spielstunden versorgen".into());}
        }
        let treibstoff = plan.treibstoff * if einweg { 1 } else { 2 };
        self.abrechnen(pid);
        let mut bedarf = a.ladung;
        bedarf[Gut::Deuterium.idx()] = bedarf[Gut::Deuterium.idx()].checked_add(treibstoff).ok_or("Ladung und Treibstoff sind zu groß")?;
        let mut sprit = [0i64; GUETER];
        sprit[Gut::Deuterium.idx()] = treibstoff;
        let topf = Self::topf_fuer(rolle, Topf::Wirtschaft);
        // Bestand muss Ladung und Treibstoff decken, der Topf nur den Treibstoff.
        self.zahlbar(pid, &bedarf, None)?;
        self.zahlbar(pid, &sprit, topf)?;
        self.zahlen(pid, &sprit, topf);
        if self.ausbau.version>0 && siedler>0 {
            let people=self.ausbau.personen.get_mut(&(pid as PlanetId)).expect("population");
            people.nehmen(&siedlergruppe).expect("validated settlers");
            self.ausbau.siedler.insert(self.naechste_flotte,siedlergruppe);
        }
        let p = &mut self.planeten[pid];
        for g in 0..GUETER {
            p.bestand[g] -= a.ladung[g];
        }
        for e in 0..SCHIFFE {
            p.einheiten[e] -= a.schiffe[e];
        }
        p.bevoelkerung -= siedler;
        let id = self.naechste_flotte;
        self.naechste_flotte += 1;
        let ankunft = self.zeit + plan.dauer;
        self.flotten.insert(
            id,
            Flotte {
                id,
                besitzer: sid,
                start: self.planeten[pid].id,
                start_koord: a.start,
                ziel: a.ziel,
                mission: a.mission,
                schiffe: a.schiffe,
                ladung: a.ladung,
                siedler,
                abflug: self.zeit,
                ankunft,
                flugdauer: plan.dauer,
                rueckkehr: 0,
                orbit_ende: 0,
                haltedauer,
                zustand: Flottenzustand::Hinflug,
                gemeldet: false,
                verband: None,
            },
        );
        if let Some(v) = terraform_volk { self.ausbau.terraform_fluege.insert(id, v); }
        if self.ausbau.version>0 {self.ausbau.fluganzeigen.insert(id,serde_json::json!({"einweg":einweg,"hin_tempo_prozent":a.sigma_pm/10,"hin_dauer":plan.dauer,"treibstoff_hin":plan.treibstoff as f64/M as f64,"distanz":plan.entfernung,"reisetempo":plan.tempo}));}
        self.plane(ankunft, EreignisArt::FlotteAnkunft { flotte: id });
        if self.aufklaerungsregeln() {
            self.aufklaerung.abschirmung.insert(id, self.spieler[sid as usize].forschung[Forschung::Abschirmtechnik.idx()]);
            if let Some(target) = sondenziel { self.aufklaerung.sondenziele.insert(id, target); }
            if a.mission.feindlich() {
                if let Some(zp) = ziel_pid {
                    let kontakt = (ankunft - self.warnzeit(zp)).max(self.zeit);
                    if kontakt == self.zeit { self.sensor_kontakt(id); }
                    else { self.plane(kontakt, EreignisArt::SensorKontakt { flotte:id }); }
                }
            }
        }
        if self.kolonisationsregeln_v2() && a.mission == Mission::Transport {
            if let Some(zp) = ziel_pid {
                self.kolonisation.transportziele.insert(
                    id,
                    Transportziel::Planet {
                        planet: zp as PlanetId,
                        empfaenger: self.planeten[zp].besitzer,
                    },
                );
            } else if let Some(of) = self.eigene_orbitflotte_bei(sid, a.ziel) {
                self.kolonisation.transportziele.insert(id, Transportziel::Flotte { flotte: of, empfaenger: sid });
            }
        }
        self.fachereignis(
            "fleet_launched",
            sid,
            serde_json::json!({"fleet":id,"mission":a.mission,"target":a.ziel,"arrival":ankunft}),
        );
        if feindlich_gegen.is_some() {
            // Wer selbst angreift, verliert den Anfängerschutz sofort.
            let jetzt = self.zeit;
            let sp = &mut self.spieler[sid as usize];
            sp.schutz_bis = sp.schutz_bis.min(jetzt);
        }
        if siedler > 0 {
            self.raten_neu(pid);
        }
        Ok(format!(
            "Flotte {id} gestartet: {} nach {}, Ankunft {}, Treibstoff {}",
            a.mission,
            a.ziel,
            zeittext(ankunft),
            ganz(treibstoff)
        ))
    }

    /// Eine eigene ausfliegende Angriffsflotte freiwillig für Allianzpartner öffnen.
    fn tatsaechlicher_gegner(&self, pid: usize, sid: SpielerId) -> Option<SpielerId> {
        if let Some(f) = self.planeten[pid]
            .blockade
            .and_then(|id| self.flotten.get(&id))
            .filter(|f| f.zustand == Flottenzustand::ImOrbit)
        {
            return (f.besitzer != sid && !self.verbuendet(sid, f.besitzer)).then_some(f.besitzer);
        }
        let owner = self.planeten[pid].besitzer;
        (owner != sid && !self.verbuendet(sid, owner)).then_some(owner)
    }
    pub(crate) fn gegner_angreifbar(&self, sid: SpielerId, gegner: SpielerId) -> Result<(), String> {
        if sid == gegner || self.verbuendet(sid, gegner) {
            return Err("Gegner ist inzwischen eigener oder verbündeter Spieler".into());
        }
        let g = &self.spieler[gegner as usize];
        let sp = &self.spieler[sid as usize];
        if self.zeit < g.schutz_bis {
            return Err(format!("{} steht unter Anfängerschutz", g.name));
        }
        let schwach = self.regeln.diplomatie.schwachenschutz_anteil;
        if schwach > 0.0
            && g.punkte.gesamt() < mal(sp.punkte.gesamt(), schwach)
            && !g.angegriffen.contains(&sid)
        {
            return Err("Tatsächlicher Gegner steht unter Schwachenschutz".into());
        }
        Ok(())
    }
    fn offensive_pruefen(
        &self,
        sid: SpielerId,
        pid: usize,
        mission: Mission,
        schiffe: &[i64; SCHIFFE],
    ) -> Result<(), String> {
        let p = &self.planeten[pid];
        let freund = p.besitzer == sid || self.verbuendet(sid, p.besitzer);
        let eigene_blockade = p
            .blockade
            .and_then(|id| self.flotten.get(&id))
            .is_some_and(|f| f.besitzer == sid && f.zustand == Flottenzustand::ImOrbit);
        if freund && (mission != Mission::Angriff || self.tatsaechlicher_gegner(pid, sid).is_none())
        {
            return Err("Eigene oder verbündete Planeten erlauben nur Entsatz gegen eine feindliche Blockade".into());
        }
        if !freund && self.verbuendet(sid, p.besitzer) {
            return Err("Bündnis zuerst ausdrücklich beenden".into());
        }
        if mission != Mission::Angriff && self.spieler[sid as usize].stufe < 4 {
            return Err("Orbitbesetzung benötigt Zivilisationsstufe 4".into());
        }
        if self.invasion_v1() && mission == Mission::Bombardieren && self.bombenlast_flotte(schiffe) == 0 {
            return Err("Bombardieren braucht Bomber (bomber oder strategischer_bomber)".into());
        }
        if mission == Mission::Kampfkolonisieren {
            if p.heimat {
                return Err("Der ursprüngliche Heimatplanet ist vor Übernahme geschützt".into());
            }
            if Self::kolonieschiff_typ(schiffe).is_none() {
                return Err("Kampfkolonisierung braucht ein Kolonieschiff".into());
            }
            if !self.kolonieplatz_erst_bei_gruendung() && !self.koloniekapazitaet(sid, p.koord) {
                return Err("Koloniekapazität bereits belegt oder reserviert".into());
            }
        }
        if let Some(g) = self.tatsaechlicher_gegner(pid, sid) {
            self.gegner_angreifbar(sid, g)?;
        } else if eigene_blockade
            && (mission == Mission::Kampfkolonisieren
                || (self.invasion_v1() && mission == Mission::Bombardieren))
            && !freund
        {
            self.gegner_angreifbar(sid, p.besitzer)?;
        } else if eigene_blockade && !freund {
            // Spielprobe 20.10.2026: „Kein feindlicher Gegner am Ziel“ war bei eigener Blockade unverständlich.
            return Err("Deine eigene Flotte hält den Orbit – ein weiterer Angriff findet dort keinen Gegner. Möglich sind Bombardieren und Kampfkolonisieren.".into());
        } else {
            return Err("Kein feindlicher Gegner am Ziel".into());
        }
        Ok(())
    }
    pub fn flotte_versorgen(
        &mut self,
        sid: SpielerId,
        rolle: Rolle,
        a: Flugauftrag,
        versorgungsflotte: FlottenId,
    ) -> Result<String, String> {
        if !self.kolonisationsregeln_v2() {
            return Err("Flottenversorgung benötigt V4-Regeln".into());
        }
        let f = self
            .flotten
            .get(&versorgungsflotte)
            .ok_or("Versorgungsflotte fehlt")?;
        if f.besitzer != sid
            || f.zustand != Flottenzustand::ImOrbit
            || f.ziel != a.ziel
            || a.mission != Mission::Transport
        {
            return Err(
                "Versorgungsziel muss die gebundene eigene Orbitflotte am Flugziel sein".into(),
            );
        }
        // Gestrandete Flotten (0.4.8): Laderaum des Empfängers vor dem Start prüfen – dieselbe Rechnung wie bei der Ankunft
        // (Funktionsprüfung 25.10.2026: eine Sondenflotte ohne Laderaum nahm den Auftrag an, die Lieferung scheiterte erst dort).
        if self.orbitflotten_aktiv() {
            let kap = self.flugplan(f.besitzer, f.start as usize, f.ziel, &f.schiffe, 1000).map(|p| p.kapazitaet).unwrap_or(0);
            let frei = kap - f.ladung.iter().sum::<i64>();
            let menge: i64 = a.ladung.iter().sum();
            if kap <= 0 { return Err("Diese Flotte hat keinen Laderaum – sie kann keinen Treibstoff aufnehmen".into()); }
            if menge > frei { return Err(format!("Laderaum der Flotte reicht nicht: frei {}, geliefert würden {}", ganz(frei.max(0)), ganz(menge))); }
        }
        let id = self.naechste_flotte;
        let result = self.flotte_senden(sid, rolle, a)?;
        self.kolonisation.transportziele.insert(
            id,
            Transportziel::Flotte {
                flotte: versorgungsflotte,
                empfaenger: sid,
            },
        );
        Ok(result)
    }
    fn transport_ankunft_v2(&mut self, fid: FlottenId) {
        let f = self.flotten[&fid].clone();
        let mut zugestellt = false;
        // Für die Meldung: Personen und Bodentruppen an Bord, bevor sie landen (Funktionsprüfung 25.10.2026: nur die Fracht wurde genannt).
        let an_bord_personen = self.ausbau.passagiere.get(&fid).map_or(0, |p| p.gesamt() / M);
        let an_bord_truppen: i64 = self.ausbau.boden.flotten.get(&fid).map_or(0, |u| u.iter().sum());
        match self.kolonisation.transportziele.remove(&fid) {
            Some(Transportziel::Planet { planet, empfaenger }) => {
                let pid = planet as usize;
                if self
                    .planeten
                    .get(pid)
                    .is_some_and(|p| p.besitzer == empfaenger && p.koord == f.ziel)
                    && !self.blockiert_fuer(pid, empfaenger)
                {
                    self.abrechnen(pid);
                    self.personen_landen(fid,pid);
                    for g in 0..GUETER {
                        self.planeten[pid].bestand[g] += f.ladung[g];
                    }
                    let wert = self.regeln.wert(&f.ladung);
                    if empfaenger != f.besitzer {
                        self.spieler[f.besitzer as usize].statistik.geschenkt += wert;
                        self.spieler[empfaenger as usize].statistik.erhalten += wert;
                    }
                    self.raten_neu(pid);
                    zugestellt = true;
                }
            }
            Some(Transportziel::Flotte { flotte, empfaenger }) => {
                if let Some(z) = self.flotten.get(&flotte).cloned().filter(|z| {
                    z.besitzer == empfaenger
                        && empfaenger == f.besitzer
                        && z.ziel == f.ziel
                        && z.zustand == Flottenzustand::ImOrbit
                        && z.id != fid
                }) {
                    let kap = self
                        .flugplan(z.besitzer, z.start as usize, z.ziel, &z.schiffe, 1000)
                        .map(|p| p.kapazitaet)
                        .unwrap_or(0);
                    let neue = z
                        .ladung
                        .iter()
                        .zip(f.ladung)
                        .map(|(a, b)| a.checked_add(b))
                        .collect::<Option<Vec<_>>>();
                    if let Some(neue) =
                        neue.filter(|n| n.iter().map(|v| *v as i128).sum::<i128>() <= kap as i128)
                    {
                        self.flotten
                            .get_mut(&flotte)
                            .unwrap()
                            .ladung
                            .copy_from_slice(&neue);
                        zugestellt = true;
                    }
                }
            }
            None => {}
        }
        if zugestellt {
            self.flotten.get_mut(&fid).unwrap().ladung = [0; GUETER];
        }
        // Meldung für Absender und Empfänger (online fehlte sie, nur v1 meldete „Lieferung von …“).
        let mut teile = if f.ladung.iter().any(|n| *n >= M) { vec![crate::berichte::fracht_text(&f.ladung)] } else { vec![] };
        if an_bord_personen > 0 { teile.push(format!("{an_bord_personen} Personen")); }
        if an_bord_truppen > 0 { teile.push(format!("{an_bord_truppen} Bodentruppen")); }
        let fracht = if teile.is_empty() { "nichts".to_string() } else { teile.join(", ") };
        let empfaenger = self.belegung.get(&f.ziel).map(|p| self.planeten[*p as usize].besitzer);
        if zugestellt {
            self.vorfall(f.besitzer, "lieferung", format!("Flotte {fid}: Transport nach {} zugestellt: {fracht}", f.ziel));
            if let Some(e) = empfaenger.filter(|e| *e != f.besitzer) {
                let von = self.spieler[f.besitzer as usize].name.clone();
                self.vorfall(e, "lieferung", format!("Lieferung von {von} auf {} angekommen: {fracht}", f.ziel));
            }
        } else {
            self.vorfall(
                f.besitzer,
                "lieferung",
                format!("Flotte {fid}: Transport nach {} nicht zugestellt (Ziel nicht mehr erreichbar, blockiert oder Laderaum voll); Fracht kehrt zurück: {fracht}", f.ziel),
            );
        }
        self.fachereignis(
            if zugestellt {
                "transport_delivered"
            } else {
                "transport_returned"
            },
            f.besitzer,
            serde_json::json!({"fleet":fid,"target":f.ziel,"cargo":f.ladung}),
        );
        self.heimflug(fid);
    }
    pub fn verband_oeffnen(&mut self, sid: SpielerId, fid: FlottenId) -> Result<String, String> {
        let f = self.flotten.get_mut(&fid).ok_or("Flotte fehlt")?;
        if f.besitzer != sid
            || f.mission != Mission::Angriff
            || f.zustand != Flottenzustand::Hinflug
            || f.ankunft <= self.zeit
        {
            return Err(
                "Nur eigene ausfliegende Angriffsflotten können einen Verband öffnen".into(),
            );
        }
        if f.verband.is_some_and(|v| v != fid) {
            return Err("Flotte ist bereits einem anderen Verband beigetreten".into());
        }
        f.verband = Some(fid);
        Ok(format!("Angriffsverband {fid} für Allianzpartner geöffnet"))
    }

    /// Niemals beschleunigen: alle Teilnehmer erreichen das Ziel zur spätesten Ankunft.
    /// Die gesamte Verzögerung gegenüber dem ursprünglichen Flug bleibt <= 6 Stunden.
    pub fn verband_beitreten(
        &mut self,
        sid: SpielerId,
        fid: FlottenId,
        fuehrung: FlottenId,
    ) -> Result<String, String> {
        let f = self.flotten.get(&fid).ok_or("Beitrittsflotte fehlt")?;
        let leader = self.flotten.get(&fuehrung).ok_or("Führungsflotte fehlt")?;
        if fid == fuehrung || f.besitzer != sid || f.verband.is_some() {
            return Err("Nur eine eigene ungebundene Flotte kann beitreten".into());
        }
        if leader.verband != Some(fuehrung) {
            return Err("Führung hat keinen Verband geöffnet".into());
        }
        let gleiche_allianz = self.spieler[sid as usize].allianz.is_some()
            && self.spieler[sid as usize].allianz == self.spieler[leader.besitzer as usize].allianz;
        if sid != leader.besitzer && !gleiche_allianz {
            return Err("Verbandsangriffe erfordern dieselbe Allianz".into());
        }
        if [f, leader].iter().any(|x| {
            x.mission != Mission::Angriff
                || x.zustand != Flottenzustand::Hinflug
                || x.ankunft <= self.zeit
        }) || f.ziel != leader.ziel
        {
            return Err("Beide Flotten müssen im Hinflug dasselbe Angriffsziel haben".into());
        }
        let mut ids: Vec<_> = self
            .flotten
            .values()
            .filter(|x| x.verband == Some(fuehrung) && x.zustand == Flottenzustand::Hinflug)
            .map(|x| x.id)
            .collect();
        if ids.len() >= 16 {
            return Err("Ein Verband umfasst höchstens 16 Flotten".into());
        }
        ids.push(fid);
        let ankunft = ids.iter().map(|id| self.flotten[id].ankunft).max().unwrap();
        if ids
            .iter()
            .any(|id| ankunft - (self.flotten[id].abflug + self.flotten[id].flugdauer) > 6 * STUNDE)
        {
            return Err(
                "Verband würde ursprünglichen Flug um mehr als sechs Stunden verzögern".into(),
            );
        }
        for id in ids {
            let fl = self.flotten.get_mut(&id).unwrap();
            fl.verband = Some(fuehrung);
            fl.ankunft = ankunft;
            self.plane(ankunft, EreignisArt::FlotteAnkunft { flotte: id });
        }
        Ok(format!(
            "Flotte {fid} tritt Verband {fuehrung} bei; gemeinsame Ankunft {}",
            zeittext(ankunft)
        ))
    }

    pub fn flotte_zurueckrufen(
        &mut self,
        sid: SpielerId,
        fid: FlottenId,
    ) -> Result<String, String> {
        let jetzt = self.zeit;
        let Some(f) = self.flotten.get(&fid) else {
            if self.aufklaerungsregeln() {return Err("Keine eigene rückrufbare Flotte unter dieser Kennung".into());}
            return Err(format!("Flotte {fid} gibt es nicht"));
        };
        if f.besitzer != sid {
            if self.aufklaerungsregeln() {return Err("Keine eigene rückrufbare Flotte unter dieser Kennung".into());}
            return Err(format!("Flotte {fid} gehört dir nicht"));
        }
        self.rueckruf_orbit(fid)?;
        let Some(f) = self.flotten.get(&fid) else { return Err("Flotte fehlt".into()) };
        if self.ausbau.expeditionen.get(&fid).is_some_and(|e|e.status=="vermisst"){return Err("Vermisste Expedition kann nicht zurückgerufen werden".into());}
        if self.ausbau.expeditionen.get(&fid).is_some_and(|e|e.status=="aktiv"){
            if self.ausbau.expeditionen[&fid].scan{return Err("Laufendes Expeditionsereignis zuerst abschließen".into());}
            self.expedition_abschluss(fid);return Ok("Expedition vorzeitig beendet; ihr einmaliges Gesamtrisiko wurde ausgewertet".into());
        }
        if f.zustand!=Flottenzustand::Rueckflug {
            if let Some(e)=self.ausbau.expeditionen.get_mut(&fid){e.status="rueckflug".into();}
            if let Some(target)=self.ausbau.rettungsziele.remove(&fid){if let Some(e)=self.ausbau.expeditionen.get_mut(&target){e.rettungen.remove(&fid);}}
        }
        if self.kolonisationsregeln_v2()
            && self
                .kolonisation
                .rueckwege
                .get(&fid)
                .is_some_and(|r| r.wartend)
        {
            return Err("Rückkehrflotte wartet bereits auf die Freigabe ihres Landeorbits".into());
        }
        match f.zustand {
            Flottenzustand::Rueckflug => Err(format!("Flotte {fid} ist schon auf dem Rückflug")),
            Flottenzustand::Hinflug => {
                let zurueck = jetzt + ((jetzt - f.abflug)*self.rueckdauer(fid,f.flugdauer)/f.flugdauer.max(1)).max(MINUTE);
                let start = f.start;
                let fraction=((jetzt-f.abflug) as f64/f.flugdauer.max(1) as f64).clamp(0.0,1.0);let from=f.ziel;
                let f = self.flotten.get_mut(&fid).unwrap();
                f.zustand = Flottenzustand::Rueckflug;
                f.verband = None;
                f.rueckkehr = zurueck;
                if self.kolonisationsregeln_v2() {
                    self.kolonisation.rueckwege.insert(
                        fid,
                        Rueckweg {
                            ziel: start,
                            wartend: false,
                        },
                    );
                }
                self.rueckflug_anzeigen(fid,fraction,from);
                self.plane(zurueck, EreignisArt::FlotteRueckkehr { flotte: fid });
                Ok(format!(
                    "Flotte {fid} kehrt um, Rückkehr {}",
                    zeittext(zurueck)
                ))
            }
            Flottenzustand::ImOrbit => {
                let (mission, ziel, seit) = (f.mission, f.ziel, f.ankunft);
                if mission == Mission::Abbau {
                    self.abbau_ertrag(fid, jetzt - seit);
                }
                if let Some(zp) = self.belegung.get(&ziel).map(|x| *x as usize) {
                    if self.planeten[zp].blockade == Some(fid) {
                        self.planeten[zp].blockade = None;
                        let besitzer = self.planeten[zp].besitzer;
                        self.vorfall(
                            besitzer,
                            "blockade",
                            format!("Die Blockade von {ziel} wurde aufgehoben"),
                        );
                    }
                }
                self.heimflug(fid);
                Ok(format!("Flotte {fid} verlässt den Orbit von {ziel}"))
            }
        }
    }

    pub(crate) fn heimflug(&mut self, fid: FlottenId) {
        if self.ausbau.version>0 {
            if let Some(e)=self.ausbau.expeditionen.get_mut(&fid){if e.status!="verloren"{e.status="rueckflug".into();}}
        }

        if self.kolonisationsregeln_v2() {
            let Some(f) = self.flotten.get(&fid).cloned() else {
                return;
            };
            if f.schiffe.iter().sum::<i64>() <= 0 {
                self.flotten.remove(&fid);
                return;
            }
            let besetzungen = self
                .kolonisation
                .besetzungen
                .iter()
                .filter(|(_, b)| b.flotte == fid)
                .map(|(pid, _)| *pid)
                .collect::<Vec<_>>();
            for pid in besetzungen {
                self.besetzung_abbrechen(pid, "Besetzungsflotte verlässt den Orbit");
            }
            // 0.4.8: ohne Rückflugtreibstoff stranden bzw. zurück an den Orbitort.
            if self.heimflug_orbit(fid) { return; }
            let start = f.start as usize;
            if self.planeten[start].besitzer == f.besitzer {
                let retour = self.zeit + self.rueckdauer(fid,f.flugdauer);
                let fl = self.flotten.get_mut(&fid).unwrap();
                fl.zustand = Flottenzustand::Rueckflug;
                fl.verband = None;
                fl.rueckkehr = retour;
                self.kolonisation.rueckwege.insert(
                    fid,
                    Rueckweg {
                        ziel: f.start,
                        wartend: false,
                    },
                );
                self.rueckflug_anzeigen(fid,1.0,f.ziel);
                self.plane(retour, EreignisArt::FlotteRueckkehr { flotte: fid });
            } else {
                self.rueckflug_umleiten(fid, f.ziel, self.spieler[f.besitzer as usize].heimat);
            }
            return;
        }
        let jetzt = self.zeit;
        let Some(f) = self.flotten.get_mut(&fid) else {
            return;
        };
        if f.schiffe.iter().sum::<i64>() <= 0 {
            self.flotten.remove(&fid);
            return;
        }
        f.zustand = Flottenzustand::Rueckflug;
        f.verband = None;
        f.rueckkehr = jetzt + f.flugdauer;
        let zurueck = f.rueckkehr;let from=f.ziel;
        self.rueckflug_anzeigen(fid,1.0,from);
        self.plane(zurueck, EreignisArt::FlotteRueckkehr { flotte: fid });
    }

    pub fn flotte_rueckkehr(&mut self, fid: FlottenId) {
        // Sonde eines Sammelauftrags gelandet: sie (und andere freie Sonden) fliegen die nächsten offenen Plätze an.
        let auftrag = self.sammelauftrag(fid);
        self.flotte_rueckkehr_landen(fid);
        if let Some(auftrag) = auftrag.filter(|_| !self.flotten.contains_key(&fid)) { self.sammelauftrag_fortsetzen(&auftrag, true); }
    }

    fn flotte_rueckkehr_landen(&mut self, fid: FlottenId) {
        let Some(f) = self.flotten.get(&fid).cloned() else {
            return;
        };
        if f.zustand != Flottenzustand::Rueckflug || f.rueckkehr != self.zeit {
            return;
        }
        if self.rueckkehr_orbit(fid) {
            return;
        }
        if self.mond_rueckkehr(fid) {
            return;
        }
        if self.kolonisationsregeln_v2() {
            let pid = self
                .kolonisation
                .rueckwege
                .get(&fid)
                .map(|r| r.ziel)
                .unwrap_or(f.start) as usize;
            if self.planeten[pid].besitzer != f.besitzer {
                let home = self.spieler[f.besitzer as usize].heimat;
                self.rueckflug_umleiten(fid, self.planeten[pid].koord, home);
                return;
            }
            if self.blockiert_fuer(pid, f.besitzer) {
                let ende = self.zeit + self.regeln.fenster();
                let fl = self.flotten.get_mut(&fid).unwrap();
                fl.zustand = Flottenzustand::ImOrbit;
                fl.ziel = self.planeten[pid].koord;
                fl.orbit_ende = ende;
                self.kolonisation.rueckwege.insert(
                    fid,
                    Rueckweg {
                        ziel: pid as PlanetId,
                        wartend: true,
                    },
                );
                self.plane(ende, EreignisArt::OrbitEnde { flotte: fid });
                self.fachereignis(
                    "return_waiting",
                    f.besitzer,
                    serde_json::json!({"fleet":fid,"planet":pid}),
                );
                return;
            }
            self.rueckflug_landen(fid, pid);
            return;
        }
        // Ist der Startplanet verloren, landet die Flotte auf der Heimatwelt.
        let mut pid = f.start as usize;
        if self.planeten[pid].besitzer != f.besitzer {
            pid = self.spieler[f.besitzer as usize].heimat as usize;
        }
        self.personen_landen(fid,pid);
        self.abrechnen(pid);
        let p = &mut self.planeten[pid];
        for e in 0..SCHIFFE {
            p.einheiten[e] += f.schiffe[e];
        }
        for g in 0..GUETER {
            p.bestand[g] += f.ladung[g];
        }
        p.bevoelkerung += f.siedler;
        let k = p.koord;
        self.flotten.remove(&fid);
        let fracht: Vec<String> = Gut::ALLE
            .iter()
            .filter(|g| f.ladung[g.idx()] >= M)
            .map(|g| format!("{} {}", ganz(f.ladung[g.idx()]), g))
            .collect();
        let text = if fracht.is_empty() {
            format!("Flotte {fid} ({}) ist auf {k} zurück", f.mission)
        } else {
            format!(
                "Flotte {fid} ({}) ist auf {k} zurück mit {}",
                f.mission,
                fracht.join(", ")
            )
        };
        self.vorfall(f.besitzer, "flotte", text);
        self.raten_neu(pid);
    }

    pub(crate) fn rueckflug_anzeigen(&mut self,fid:FlottenId,position:f64,von:Koord){
        if self.ausbau.version==0{return;}
        let f=&self.flotten[&fid];let v=self.ausbau.fluganzeigen.entry(fid).or_insert_with(||serde_json::json!({}));
        v["rueck_beginn"]=serde_json::json!(self.zeit);v["rueck_dauer"]=serde_json::json!((f.rueckkehr-self.zeit).max(1));v["rueck_position"]=serde_json::json!(position);v["rueck_von"]=serde_json::json!(von.to_string());
    }
    fn rueckflug_umleiten(&mut self, fid: FlottenId, von: Koord, ziel: PlanetId) {
        let f = self.flotten[&fid].clone();
        let k = self.planeten[ziel as usize].koord;
        let tempo = Einheit::ALLE[..SCHIFFE]
            .iter()
            .filter(|e| f.schiffe[e.idx()] > 0)
            .map(|e| self.tempo(f.besitzer, *e))
            .min()
            .unwrap_or(1);
        let dauer = self.flugdauer(self.entfernung(von, k), tempo, 1000);
        let ende = self.zeit + dauer;
        let fl = self.flotten.get_mut(&fid).unwrap();
        fl.zustand = Flottenzustand::Rueckflug;
        fl.verband = None;
        fl.rueckkehr = ende;
        fl.ziel = k;
        self.kolonisation.rueckwege.insert(
            fid,
            Rueckweg {
                ziel,
                wartend: false,
            },
        );
        self.rueckflug_anzeigen(fid,1.0,von);
        if let Some(v)=self.ausbau.fluganzeigen.get_mut(&fid){v["umgeleitet"]=serde_json::json!(true);v["rueck_tempo_prozent"]=serde_json::json!(100);}
        self.plane(ende, EreignisArt::FlotteRueckkehr { flotte: fid });
        self.fachereignis(
            "return_redirected",
            f.besitzer,
            serde_json::json!({"fleet":fid,"from":von,"target":k,"arrival":ende}),
        );
    }
    pub(crate) fn rueckflug_landen(&mut self, fid: FlottenId, pid: usize) {
        self.personen_landen(fid,pid);
        let f = self.flotten[&fid].clone();
        self.abrechnen(pid);
        let p = &mut self.planeten[pid];
        for e in 0..SCHIFFE {
            p.einheiten[e] += f.schiffe[e];
        }
        for g in 0..GUETER {
            p.bestand[g] += f.ladung[g];
        }
        p.bevoelkerung += f.siedler;
        self.flotten.remove(&fid);
        self.kolonisation.rueckwege.remove(&fid);
        self.kolonisation.transportziele.remove(&fid);
        self.fachereignis(
            "fleet_returned",
            f.besitzer,
            serde_json::json!({"fleet":fid,"planet":pid}),
        );
        // Mit Fracht (Beute, Recycling, Abbau, nicht zugestellte Lieferung) wie in v1.
        let fracht = crate::berichte::fracht_text(&f.ladung);
        self.vorfall(
            f.besitzer,
            "flotte",
            if fracht == "nichts" {
                format!("Flotte {fid} ({}) auf {} zurück", f.mission, self.planeten[pid].koord)
            } else {
                format!("Flotte {fid} ({}) auf {} zurück mit {fracht}", f.mission, self.planeten[pid].koord)
            },
        );
        self.raten_neu(pid);
    }
    fn abbau_ertrag(&mut self, fid: FlottenId, dauer: i64) {
        let r = self.regeln.clone();
        let Some(f) = self.flotten.get(&fid) else {
            return;
        };
        let Some(sys) = self.system(f.ziel) else {
            return;
        };
        let schiffe = f.schiffe[Einheit::Bergbauschiff.idx()];
        let erz = anteil(
            anteil(schiffe * r.flug.abbau_erz_je_stunde * M, dauer, STUNDE),
            sys.reich_erz,
            1000,
        );
        let kristall = anteil(
            anteil(schiffe * r.flug.abbau_kristall_je_stunde * M, dauer, STUNDE),
            sys.reich_kristall,
            1000,
        );
        let Ok(plan) = self.flugplan(f.besitzer, f.start as usize, f.ziel, &f.schiffe, 1000) else {
            return;
        };
        let frei = (plan.kapazitaet - f.ladung.iter().sum::<i64>()).max(0);
        let summe = erz + kristall;
        let (erz, kristall) = if summe > frei {
            (anteil(erz, frei, summe), anteil(kristall, frei, summe))
        } else {
            (erz, kristall)
        };
        let f = self.flotten.get_mut(&fid).unwrap();
        f.ladung[Gut::Erz.idx()] += erz;
        f.ladung[Gut::Kristall.idx()] += kristall;
        let (besitzer, ziel) = (f.besitzer, f.ziel);
        let voll = summe > frei;
        self.vorfall(
            besitzer,
            "abbau",
            format!(
                "Flotte {fid}: Abbau bei {ziel} beendet: {} Erz, {} Kristall{}",
                ganz(erz),
                ganz(kristall),
                if voll { " (Laderaum voll, Rest bleibt im Gürtel)" } else { "" }
            ),
        );
    }

    pub fn orbit_ende(&mut self, fid: FlottenId) {
        let Some(f) = self.flotten.get(&fid) else {
            return;
        };
        if f.zustand != Flottenzustand::ImOrbit || f.orbit_ende != self.zeit {
            return;
        }
        if self.kolonisationsregeln_v2()
            && self
                .kolonisation
                .rueckwege
                .get(&fid)
                .is_some_and(|r| r.wartend)
        {
            let fl = self.flotten.get_mut(&fid).unwrap();
            fl.zustand = Flottenzustand::Rueckflug;
            fl.rueckkehr = self.zeit;
            self.flotte_rueckkehr(fid);
            return;
        }
        let f = f.clone();
        if self.invasion_orbit_verlaengern(fid) {
            return;
        }
        if f.mission == Mission::Abbau {
            let dauer = f.haltedauer;
            self.abbau_ertrag(fid, dauer);
        }
        self.heimflug(fid);
    }

    pub fn flotte_ankunft(&mut self, fid: FlottenId) {
        let Some(f) = self.flotten.get(&fid).cloned() else {
            return;
        };
        if f.zustand != Flottenzustand::Hinflug || f.ankunft != self.zeit {
            return;
        }
        if self.schiffhandel_ankunft(fid){return;}
        if self.expedition_ankunft(fid){return;}
        if self.mond_ankunft(fid){return;}
        if f.mission == Mission::Angriff && f.verband.is_some() {
            self.verbandskampf(fid);
            return;
        }
        let jetzt = self.zeit;
        let ziel_pid = self.belegung.get(&f.ziel).map(|x| *x as usize);
        if self.orbitkampf_ankunft(fid) { return; }
        if self.verwuestet_ankunft(fid, ziel_pid) { return; }
        if self.invasion_ankunft(fid, ziel_pid) { return; }
        if f.mission==Mission::Stationieren{if let Some(p)=ziel_pid.filter(|p|self.planeten[*p].besitzer==f.besitzer&&!self.blockiert_fuer(*p,f.besitzer)){self.personen_landen(fid,p);}}
        match f.mission {
            Mission::SystemErkunden => self.system_erkunden_ankunft(fid),
            Mission::FlottenSpionage => self.flotten_spionage_ankunft(fid),
            Mission::Saven => {
                if f.haltedauer > 0 {
                    let end = jetzt + f.haltedauer;
                    let fleet = self.flotten.get_mut(&fid).unwrap();
                    fleet.zustand = Flottenzustand::ImOrbit;
                    fleet.orbit_ende = end;
                    self.plane(end, EreignisArt::OrbitEnde { flotte:fid });
                } else { self.heimflug(fid); }
            }
            Mission::Transport => {
                if self.kolonisationsregeln_v2() {
                    self.transport_ankunft_v2(fid);
                    return;
                }
                if let Some(zp) = ziel_pid {
                    if self.blockiert_fuer(zp, f.besitzer) {
                        self.vorfall(
                            f.besitzer,
                            "flotte",
                            format!("Transport nach {} kehrt um: Blockade", f.ziel),
                        );
                    } else {
                        self.abrechnen(zp);
                        let empfaenger = self.planeten[zp].besitzer;
                        for g in 0..GUETER {
                            self.planeten[zp].bestand[g] += f.ladung[g];
                        }
                        let wert = self.regeln.wert(&f.ladung);
                        self.flotten.get_mut(&fid).unwrap().ladung = [0; GUETER];
                        if empfaenger != f.besitzer && wert > 0 {
                            let (von, an) = (
                                self.spieler[f.besitzer as usize].name.clone(),
                                self.spieler[empfaenger as usize].name.clone(),
                            );
                            self.spieler[f.besitzer as usize].statistik.geschenkt += wert;
                            self.spieler[empfaenger as usize].statistik.erhalten += wert;
                            self.vorfall(
                                empfaenger,
                                "lieferung",
                                format!(
                                    "Lieferung von {von} auf {} im Wert von {}",
                                    f.ziel,
                                    ganz(wert)
                                ),
                            );
                            self.vorfall(
                                f.besitzer,
                                "lieferung",
                                format!("Lieferung an {an} auf {} zugestellt", f.ziel),
                            );
                            self.wecke(empfaenger, Rolle::Diplomat, "Lieferung erhalten", false);
                        }
                        self.raten_neu(zp);
                    }
                }
                self.heimflug(fid);
            }
            Mission::Stationieren => match ziel_pid {
                Some(zp)
                    if self.planeten[zp].besitzer == f.besitzer
                        && !self.blockiert_fuer(zp, f.besitzer) =>
                {
                    self.abrechnen(zp);
                    let p = &mut self.planeten[zp];
                    for e in 0..SCHIFFE {
                        p.einheiten[e] += f.schiffe[e];
                    }
                    for g in 0..GUETER {
                        p.bestand[g] += f.ladung[g];
                    }
                    self.flotten.remove(&fid);
                    self.vorfall(
                        f.besitzer,
                        "flotte",
                        format!("Flotte {fid} auf {} stationiert", f.ziel),
                    );
                    self.raten_neu(zp);
                }
                _ => self.heimflug(fid),
            },
            Mission::Halten => match ziel_pid {
                Some(zp)
                    if self.verbuendet(f.besitzer, self.planeten[zp].besitzer)
                        && !self.blockiert_fuer(zp, f.besitzer) =>
                {
                    let fl = self.flotten.get_mut(&fid).unwrap();
                    fl.zustand = Flottenzustand::ImOrbit;
                    fl.orbit_ende = jetzt + fl.haltedauer;
                    let ende = fl.orbit_ende;
                    self.plane(ende, EreignisArt::OrbitEnde { flotte: fid });
                    let partner = self.planeten[zp].besitzer;
                    let name = self.spieler[f.besitzer as usize].name.clone();
                    self.vorfall(
                        partner,
                        "flotte",
                        format!(
                            "Flotte von {name} hält auf {} bis {}",
                            f.ziel,
                            zeittext(ende)
                        ),
                    );
                }
                Some(zp) if self.angriffshilfe_erlaubt(f.besitzer, zp) => {
                    let fl = self.flotten.get_mut(&fid).unwrap();
                    fl.zustand = Flottenzustand::ImOrbit;
                    fl.orbit_ende = jetzt + fl.haltedauer;
                    let ende = fl.orbit_ende;
                    self.plane(ende, EreignisArt::OrbitEnde { flotte: fid });
                    let name = self.spieler[f.besitzer as usize].name.clone();
                    if let Some(w) = self.ausbau.boden.invasion.wellen.get(&(zp as PlanetId)).cloned() {
                        self.vorfall(w.angreifer, "invasion", format!("{}: Bodentruppen von {name} schließen sich deiner Invasion an", f.ziel));
                    }
                    self.vorfall(f.besitzer, "invasion", format!("Flotte {fid}: Bodentruppen kämpfen ab der nächsten Welle mit der verbündeten Invasion auf {}", f.ziel));
                }
                _ => self.heimflug(fid),
            },
            Mission::Spionage => self.spionieren(fid),
            Mission::Angriff
            | Mission::Blockade
            | Mission::Invasion
            | Mission::Bombardieren
            | Mission::Kampfkolonisieren => self.kampf_am_planeten(fid),
            Mission::Kolonisieren => self.kolonisieren(fid),
            Mission::Recyceln => {
                if let Some(t) = self.truemmer.get(&f.ziel).copied() {
                    let r = self.regeln.clone();
                    let sp = &self.spieler[f.besitzer as usize];
                    let logistik = 1.0
                        + r.flug.logistik_bonus * sp.forschung[Forschung::Logistik.idx()] as f64;
                    let kap = mal(
                        [Einheit::Recycler,Einheit::Bergungsschiff].iter().map(|e|f.schiffe[e.idx()]*r.ladung_milli(*e,sp.volk)).sum(),
                        r.volk(sp.volk).ladung * logistik,
                    );
                    let frei = (kap - f.ladung.iter().sum::<i64>()).max(0);
                    let summe = t[0] + t[1];
                    let (erz, kristall) = if summe > frei {
                        { let erz=anteil(t[0], frei, summe); (erz, frei-erz) }
                    } else {
                        (t[0], t[1])
                    };
                    let fl = self.flotten.get_mut(&fid).unwrap();
                    fl.ladung[Gut::Erz.idx()] += erz;
                    fl.ladung[Gut::Kristall.idx()] += kristall;
                    let rest = [t[0] - erz, t[1] - kristall];
                    if rest[0] + rest[1] == 0 {
                        self.truemmer.remove(&f.ziel);
                        self.monde.truemmerringe.retain(|k| *k != f.ziel);
                    } else {
                        self.truemmer.insert(f.ziel, rest);
                    }
                    self.vorfall(
                        f.besitzer,
                        "recycling",
                        format!(
                            "Flotte {fid}: Trümmerfeld {} recycelt: {} Erz, {} Kristall geladen{}",
                            f.ziel,
                            ganz(erz),
                            ganz(kristall),
                            if rest[0] + rest[1] > 0 {
                                format!("; Rest im Feld {} Erz, {} Kristall (Laderaum voll)", ganz(rest[0]), ganz(rest[1]))
                            } else {
                                "; Feld vollständig geräumt".into()
                            }
                        ),
                    );
                } else {
                    self.vorfall(f.besitzer, "recycling", format!("Flotte {fid}: bei {} kein Trümmerfeld mehr vorgefunden", f.ziel));
                }
                self.ueberlebende_bergen(fid);
                self.heimflug(fid);
            }
            Mission::Abbau => {
                let fl = self.flotten.get_mut(&fid).unwrap();
                fl.zustand = Flottenzustand::ImOrbit;
                fl.orbit_ende = jetzt + fl.haltedauer;
                let ende = fl.orbit_ende;
                self.plane(ende, EreignisArt::OrbitEnde { flotte: fid });
            }
            Mission::MondZerstoeren => self.heimflug(fid),
            Mission::Terraformen => self.terraform_ankunft(fid),
        }
    }

    // ---------------------------------------------------------------- Kampf

    /// Trägt einen Kampf aus und schreibt Überlebende, Trümmer, Bericht und Statistik zurück.
    pub(crate) fn gefecht(&mut self, fids: &[FlottenId], parteien: &[Partei], ort: Koord) -> Sieger {
        let r = self.regeln.clone();
        let nr = self.kampf_nr;
        self.kampf_nr += 1;
        let mut rng = strom(self.startwert, KANAL_KAMPF, nr);
        let f = self.flotten[&fids[0]].clone();
        let flotten: Vec<_> = fids.iter().map(|id| self.flotten[id].clone()).collect();
        let mut koloniestand = std::collections::BTreeMap::new();
        if self.kolonisationsregeln_v2() {
            for fl in &flotten {
                koloniestand.insert(fl.id, Self::kolonieschiffe(&fl.schiffe));
            }
            for p in parteien {
                if let Partei::Flotte(id) = p {
                    koloniestand
                        .insert(*id, Self::kolonieschiffe(&self.flotten[id].schiffe));
                }
            }
        }
        let mut ang_e = [0i64; EINHEITEN];
        let ang: Vec<_> = flotten
            .iter()
            .map(|fl| {
                let mut e = [0; EINHEITEN];
                e[..SCHIFFE].copy_from_slice(&fl.schiffe);
                for i in 0..EINHEITEN {
                    ang_e[i] += e[i];
                }
                Gruppe::neu(&r, &self.spieler[fl.besitzer as usize], e)
            })
            .collect();
        let mut vert = Vec::new();
        for p in parteien {
            match p {
                Partei::Planet(zp) => {
                    let pl = &self.planeten[*zp];
                    vert.push(Gruppe::neu(
                        &r,
                        &self.spieler[pl.besitzer as usize],
                        pl.einheiten,
                    ));
                }
                Partei::Flotte(id) => {
                    let fl = &self.flotten[id];
                    let mut e = [0i64; EINHEITEN];
                    e[..SCHIFFE].copy_from_slice(&fl.schiffe);
                    vert.push(Gruppe::neu(&r, &self.spieler[fl.besitzer as usize], e));
                }
                Partei::Mond(mid) => {
                    let m = &self.monde.monde[mid];
                    vert.push(Gruppe::neu(&r, &self.spieler[m.besitzer as usize], m.einheiten));
                }
            }
        }
        let erg = kampf(&r, &ang, &vert, &mut rng);
        // Mondgeneration: Reparaturdock des Mondes an diesem Ort (Schiffe des Mondbesitzers).
        let dock = self.reparaturdock(ort);
        let mut repariert = [0i64; SCHIFFE];
        let mondkampf = parteien.iter().any(|p| matches!(p, Partei::Mond(_)));

        let mut truemmer = [0i64; 2];
        let mut verlorene_besatzung = [0i64; VOELKER];
        let mut truemmer_dazu = |e: usize, verloren: i64, volk: Volk| {
            if e < SCHIFFE { verlorene_besatzung[volk.idx()] += verloren * r.einh(Einheit::ALLE[e]).besatzung; }
            let k = r.kosten_einheit(Einheit::ALLE[e], volk);
            truemmer[0] += mal(k[Gut::Erz.idx()] * verloren, r.kampf.truemmer_anteil);
            truemmer[1] += mal(k[Gut::Kristall.idx()] * verloren, r.kampf.truemmer_anteil);
        };
        let wert_von =
            |e: usize, n: i64, volk: Volk| n * r.wert(&r.kosten_einheit(Einheit::ALLE[e], volk));

        // Angreifer.
        let mut ang_nachher = [0i64; EINHEITEN];
        let mut ang_verluste = Vec::new();
        let mut angreifer_gruppen = Vec::new();
        for (gi, fl) in flotten.iter().enumerate() {
            let ang_volk = self.spieler[fl.besitzer as usize].volk;
            let mut verlust = 0;
            for e in 0..SCHIFFE {
                let rest = erg.angreifer[gi][e];
                let verloren = fl.schiffe[e] - rest;
                ang_nachher[e] += rest;
                if verloren > 0 {
                    truemmer_dazu(e, verloren, ang_volk);
                    verlust += wert_von(e, verloren, ang_volk);
                }
            }
            ang_verluste.push((fl.besitzer, verlust));
            angreifer_gruppen.push(Kampfteilnehmer {
                flotte: fl.id,
                spieler: fl.besitzer,
                vorher: ang[gi].einheiten,
                nachher: erg.angreifer[gi],
            });
            if erg.angreifer[gi].iter().sum::<i64>() == 0 {
                self.flotten.remove(&fl.id);
            } else {
                self.flotten
                    .get_mut(&fl.id)
                    .unwrap()
                    .schiffe
                    .copy_from_slice(&erg.angreifer[gi][..SCHIFFE]);
            }
        }

        // Verteidiger.
        let mut vert_vorher = [0i64; EINHEITEN];
        let mut vert_nachher = [0i64; EINHEITEN];
        let mut vert_spieler: Vec<SpielerId> = Vec::new();
        let mut vert_verlust: Vec<(SpielerId, i64)> = Vec::new();
        // Für den Bericht: wiederaufgebaute Verteidigung getrennt und jede verteidigende Partei einzeln.
        let mut wieder = [0i64; EINHEITEN];
        let mut parteien_bericht: Vec<serde_json::Value> = Vec::new();
        for (gi, p) in parteien.iter().enumerate() {
            let sid = vert[gi].spieler;
            let volk = self.spieler[sid as usize].volk;
            let mut verlust = 0i64;
            let mut nach = [0i64; EINHEITEN];
            if !vert_spieler.contains(&sid) {
                vert_spieler.push(sid);
            }
            match p {
                Partei::Planet(zp) => {
                    for e in 0..EINHEITEN {
                        let vorher = vert[gi].einheiten[e];
                        let mut rest = erg.verteidiger[gi][e];
                        let verloren = vorher - rest;
                        vert_vorher[e] += vorher;
                        if verloren > 0 {
                            if e < SCHIFFE {
                                let rep = match dock { Some((_, owner, pm)) if owner == sid && e != Einheit::Weltentruemmer.idx() => verloren * pm / 1000, _ => 0 };
                                repariert[e] += rep;
                                truemmer_dazu(e, verloren - rep, volk);
                                verlust += wert_von(e, verloren - rep, volk);
                            } else {
                                // Zerstörte Verteidigung wird zum Teil wiederhergestellt.
                                let neu = if (self.kolonisation.aktiv
                                    && matches!(
                                        f.mission,
                                        Mission::Bombardieren | Mission::Kampfkolonisieren
                                    )) || e == Einheit::Solarsonde.idx() {
                                    0
                                } else {
                                    ((verloren as i128 * fx(r.kampf.verteidigung_wiederaufbau)
                                        + FX / 2)
                                        / FX) as i64
                                };
                                rest += neu;
                                wieder[e] += neu;
                                // Salvage only permanent losses; restored defenses retain their material.
                                truemmer_dazu(e, verloren - neu, volk);
                                verlust += wert_von(e, verloren - neu, volk);
                            }
                        }
                        vert_nachher[e] += rest;
                        nach[e] = rest;
                        self.planeten[*zp].einheiten[e] = rest;
                    }
                }
                Partei::Mond(mid) => {
                    for e in 0..EINHEITEN {
                        let vorher = vert[gi].einheiten[e];
                        let mut rest = erg.verteidiger[gi][e];
                        let verloren = vorher - rest;
                        vert_vorher[e] += vorher;
                        if verloren > 0 {
                            if e < SCHIFFE {
                                let rep = match dock { Some((_, owner, pm)) if owner == sid && e != Einheit::Weltentruemmer.idx() => verloren * pm / 1000, _ => 0 };
                                repariert[e] += rep;
                                truemmer_dazu(e, verloren - rep, volk);
                                verlust += wert_von(e, verloren - rep, volk);
                            } else {
                                // Solarsonden sind wehrlose Anlagen: kein Wiederaufbau, volle Trümmer.
                                let neu = if e == Einheit::Solarsonde.idx() { 0 } else {
                                    ((verloren as i128 * fx(r.kampf.verteidigung_wiederaufbau) + FX / 2) / FX) as i64
                                };
                                rest += neu;
                                wieder[e] += neu;
                                truemmer_dazu(e, verloren - neu, volk);
                                verlust += wert_von(e, verloren - neu, volk);
                            }
                        }
                        vert_nachher[e] += rest;
                        nach[e] = rest;
                        if let Some(m) = self.monde.monde.get_mut(mid) { m.einheiten[e] = rest; }
                    }
                }
                Partei::Flotte(id) => {
                    let mut neu = [0i64; SCHIFFE];
                    for e in 0..SCHIFFE {
                        let vorher = vert[gi].einheiten[e];
                        let rest = erg.verteidiger[gi][e];
                        vert_vorher[e] += vorher;
                        vert_nachher[e] += rest;
                        neu[e] = rest;
                        nach[e] = rest;
                        if vorher > rest {
                            let rep = match dock { Some((_, owner, pm)) if owner == sid && e != Einheit::Weltentruemmer.idx() => (vorher - rest) * pm / 1000, _ => 0 };
                            repariert[e] += rep;
                            truemmer_dazu(e, vorher - rest - rep, volk);
                            verlust += wert_von(e, vorher - rest - rep, volk);
                        }
                    }
                    if neu.iter().sum::<i64>() == 0 {
                        self.flotten.remove(id);
                    } else {
                        self.flotten.get_mut(id).unwrap().schiffe = neu;
                    }
                }
            }
            parteien_bericht.push(serde_json::json!({
                "spieler": sid,
                "art": match p { Partei::Planet(_) => "planet", Partei::Mond(_) => "mond", Partei::Flotte(_) => "flotte" },
                "flotte": match p { Partei::Flotte(id) => Some(*id), _ => None },
                "vorher": crate::sicht::einheiten(&vert[gi].einheiten, 0),
                "nachher": crate::sicht::einheiten(&nach, 0),
                "verlust_wert": ganz(verlust),
            }));
            vert_verlust.push((sid, verlust));
        }

        if truemmer[0] + truemmer[1] > 0 {
            let t = self.truemmer.entry(ort).or_insert([0, 0]);
            t[0] += truemmer[0];
            t[1] += truemmer[1];
        }
        // Rettungskapseln gehören zum Trümmerfeld, auch nach einem gewöhnlichen Raumkampf.
        // Nur dauerhaft verlorene Schiffe zählen; Werftdock-Reparaturen erzeugen keine zweite Besatzung.
        if self.ausbau.version > 0 {
            let mut personen=crate::ausbau::Personen::default();
            for volk in Volk::ALLE {
                personen.gruppen[volk.idx()][1]=verlorene_besatzung[volk.idx()]*crate::monde::UEBERLEBENDE_PROMILLE/1000*M;
            }
            if personen.gesamt()>0 {
                if let Some((_,alt))=self.monde.ueberlebende.iter_mut().find(|(k,_)|*k==ort){alt.add(&personen);}
                else{self.monde.ueberlebende.push((ort,personen));}
            }
        }
        if let Some((mid, _, _)) = dock { if repariert.iter().any(|n| *n > 0) { self.reparatur_vormerken(mid, nr, &repariert); } }
        if !mondkampf {
            let mut beteiligte: Vec<SpielerId> = flotten.iter().map(|fl| fl.besitzer).collect();
            beteiligte.extend(vert.iter().map(|g| g.spieler));
            beteiligte.sort();
            beteiligte.dedup();
            self.mondwurf(nr, ort, truemmer[0] + truemmer[1], beteiligte);
        }
        if self.ausbau.version>0 {self.ausbau.kampfrunden.insert(nr,erg.protokoll.clone());}
        // Auch in Welten ohne Ausbau (Version 0): dort nur im Speicher, Hash und Snapshot nehmen `ausbau` erst ab Version 1 auf.
        let herkunft=flotten.iter().map(|fl|(fl.besitzer,fl.start_koord,self.monde.flottenstart.contains_key(&fl.id))).collect();
        self.ausbau.kampf_herkunft.insert(nr,herkunft);
        // Technikstand beider Seiten für den Bericht (Karl 22.10.2026: Wirkung von Forschung und Schilden sehen).
        if self.ausbau.version>0 {
            let mut wer: Vec<SpielerId> = flotten.iter().map(|fl| fl.besitzer).chain(vert_spieler.iter().copied()).collect();
            wer.dedup();
            let technik = wer.into_iter().fold(Vec::new(), |mut v: Vec<(SpielerId, [u8; 3])>, s| {
                if !v.iter().any(|(x, _)| *x == s) { let fo = &self.spieler[s as usize].forschung;
                    v.push((s, [fo[Forschung::Waffentechnik.idx()], fo[Forschung::Schildtechnik.idx()], fo[Forschung::Panzerung.idx()]])); }
                v });
            self.ausbau.kampf_technik.insert(nr, technik);
            // Berichtszusatz (Karl 22.10.2026: Berichte „korrekt, ohne Lücken“): Ziel Mond oder Planet, je Partei,
            // wiederaufgebaute Verteidigung und Werftdock-Reparatur getrennt von den echten Verlusten.
            self.ausbau.kampf_zusatz.insert(nr, serde_json::json!({
                "ziel_mond": mondkampf,
                "ueberlebende": verlorene_besatzung.iter().map(|n|n*crate::monde::UEBERLEBENDE_PROMILLE/1000).sum::<i64>(),
                "wiederaufgebaut": crate::sicht::einheiten(&wieder, 0),
                "repariert": crate::sicht::einheiten(&repariert, 0),
                "parteien": parteien_bericht,
                "wiederaufbau_prozent": (r.kampf.verteidigung_wiederaufbau * 100.0).round(),
                "truemmer_prozent": (r.kampf.truemmer_anteil * 100.0).round(),
            }));
        }
        self.kampfberichte.push(Kampfbericht {
            nr,
            zeit: self.zeit,
            ort,
            mission: f.mission,
            angreifer: f.besitzer,
            angreifer_gruppen,
            verteidiger: vert_spieler.clone(),
            runden: erg.runden,
            sieger: erg.sieger,
            ang_vorher: ang_e,
            ang_nachher,
            vert_vorher,
            vert_nachher,
            beute: [0; GUETER],
            truemmer,
        });

        let ang_name = self.spieler[f.besitzer as usize].name.clone();
        let ausgang = match erg.sieger {
            Sieger::Angreifer => "Sieg des Angreifers",
            Sieger::Verteidiger => "Sieg des Verteidigers",
            Sieger::Unentschieden => "unentschieden",
        };
        // Je Spieler eine Meldung und ein Zähler, auch wenn er mit mehreren Flotten oder Planet und Flotte beteiligt war.
        let je_spieler = |liste: Vec<(SpielerId, i64)>| {
            let mut out: Vec<(SpielerId, i64)> = Vec::new();
            for (s, v) in liste {
                match out.iter_mut().find(|e| e.0 == s) {
                    Some(e) => e.1 += v,
                    None => out.push((s, v)),
                }
            }
            out
        };
        for (sid, verlust) in je_spieler(ang_verluste) {
            let st = &mut self.spieler[sid as usize].statistik;
            st.angriffe += 1;
            st.verluste += verlust;
            self.vorfall(sid,"kampf",if erg.runden==0{format!("Gefecht bei {ort}: {ausgang} ohne Gegenwehr, eigene Verluste im Wert von {}",ganz(verlust))}else{format!("Gefecht bei {ort}: {ausgang} nach {}, eigene Verluste im Wert von {}",crate::typen::runden_text(erg.runden),ganz(verlust))});
            self.wecke(sid, Rolle::Feldherr, "Kampfbericht", false);
            self.wecke(sid, Rolle::Stratege, "Kampfbericht", false);
        }
        for (sid, verlust) in je_spieler(vert_verlust) {
            let st = &mut self.spieler[sid as usize].statistik;
            st.verteidigungen += 1;
            st.verluste += verlust;
            self.vorfall(
                sid,
                "kampf",
                format!("Gefecht bei {ort}: Angriff von {ang_name}, {ausgang} nach {}, eigene Verluste im Wert von {}", crate::typen::runden_text(erg.runden), ganz(verlust)),
            );
            self.wecke(sid, Rolle::Feldherr, "Kampfbericht", true);
            self.wecke(sid, Rolle::Stratege, "Kampfbericht", false);
        }
        for (id, vorher) in koloniestand {
            let nachher = self
                .flotten
                .get(&id)
                .map(|f| Self::kolonieschiffe(&f.schiffe))
                .unwrap_or(0);
            if nachher < vorher {
                let pids = self
                    .kolonisation
                    .besetzungen
                    .iter()
                    .filter(|(_, b)| b.flotte == id)
                    .map(|(pid, _)| *pid)
                    .collect::<Vec<_>>();
                for pid in pids {
                    self.besetzung_abbrechen(pid, "Kolonieschiffbestand im Gefecht gesunken");
                }
            }
        }
        self.fachereignis(
            "combat_resolved",
            f.besitzer,
            serde_json::json!({"combat":nr,"target":ort,"attackers":fids,"winner":erg.sieger}),
        );
        erg.sieger
    }

    fn kampf_am_planeten(&mut self, fid: FlottenId) {
        if self.kolonisationsregeln_v2() {
            self.kampf_am_planeten_v2(&[fid]);
            return;
        }
        let f = self.flotten[&fid].clone();
        let Some(zp) = self.belegung.get(&f.ziel).map(|x| *x as usize) else {
            self.heimflug(fid);
            return;
        };
        self.abrechnen(zp);
        let ziel_besitzer = self.planeten[zp].besitzer;

        // Hält eine fremde Flotte den Orbit, geht der Kampf gegen sie.
        if let Some(bf) = self.planeten[zp].blockade {
            match self.flotten.get(&bf).map(|b| b.besitzer) {
                Some(b_sid) if b_sid != f.besitzer => {
                    let sieger = self.gefecht(&[fid], &[Partei::Flotte(bf)], f.ziel);
                    if !self.flotten.contains_key(&bf) {
                        self.planeten[zp].blockade = None;
                        self.vorfall(
                            ziel_besitzer,
                            "blockade",
                            format!("Die Blockade von {} ist gebrochen", f.ziel),
                        );
                    }
                    let _ = sieger;
                    self.heimflug(fid);
                    return;
                }
                Some(_) => {
                    if self.kolonisation.aktiv
                        && f.mission == Mission::Kampfkolonisieren
                        && self.flotten[&bf].besitzer == f.besitzer
                        && !self.planeten[zp].heimat
                    {
                        if self.flotten[&bf]
                            .schiffe
                            .iter()
                            .zip(f.schiffe)
                            .any(|(a, b)| a.checked_add(b).is_none())
                            || self.flotten[&bf]
                                .ladung
                                .iter()
                                .zip(f.ladung)
                                .any(|(a, b)| a.checked_add(b).is_none())
                        {
                            self.heimflug(fid);
                            return;
                        }
                        let block = self.flotten.get_mut(&bf).unwrap();
                        for e in 0..SCHIFFE {
                            block.schiffe[e] += f.schiffe[e];
                        }
                        for g in 0..GUETER {
                            block.ladung[g] += f.ladung[g];
                        }
                        block.mission = Mission::Kampfkolonisieren;
                        block.siedler += f.siedler;
                        self.boden_verstaerken(fid,bf);
                        self.flotten.remove(&fid);
                        self.besetzung_beginnen(zp, bf);
                        return;
                    }
                    self.heimflug(fid);
                    return;
                }
                None => self.planeten[zp].blockade = None,
            }
        }
        if ziel_besitzer == f.besitzer {
            self.heimflug(fid);
            return;
        }

        self.bruch_pruefen(f.besitzer, ziel_besitzer);
        if !self.spieler[f.besitzer as usize]
            .angegriffen
            .contains(&ziel_besitzer)
        {
            self.spieler[f.besitzer as usize]
                .angegriffen
                .push(ziel_besitzer);
        }
        let mut parteien = vec![Partei::Planet(zp)];
        for fl in self.flotten.values() {
            if fl.zustand == Flottenzustand::ImOrbit
                && fl.mission == Mission::Halten
                && fl.ziel == f.ziel
            {
                parteien.push(Partei::Flotte(fl.id));
            }
        }
        let sieger = self.gefecht(&[fid], &parteien, f.ziel);
        if sieger == Sieger::Angreifer && self.flotten.contains_key(&fid) {
            match f.mission {
                Mission::Angriff => {
                    self.pluendern(fid, zp);
                    self.heimflug(fid);
                }
                _ => {
                    self.flotten.get_mut(&fid).unwrap().zustand = Flottenzustand::ImOrbit;
                    self.planeten[zp].blockade = Some(fid);
                    let name = self.spieler[f.besitzer as usize].name.clone();
                    let art = if f.mission == Mission::Invasion {
                        "belagert"
                    } else {
                        "blockiert"
                    };
                    self.vorfall(
                        ziel_besitzer,
                        "blockade",
                        format!("{name} {art} {}", f.ziel),
                    );
                    self.vorfall(
                        f.besitzer,
                        "blockade",
                        format!("Flotte {fid} hält den Orbit von {}", f.ziel),
                    );
                    self.wecke(ziel_besitzer, Rolle::Stratege, "Blockade", true);
                }
            }
        } else {
            self.heimflug(fid);
        }
        if sieger == Sieger::Angreifer
            && self.flotten.contains_key(&fid)
            && matches!(
                f.mission,
                Mission::Bombardieren | Mission::Kampfkolonisieren
            )
        {
            self.bombardement_abschluss(zp, fid);
        }
        self.raten_neu(zp);
        self.punkte_neu();
    }

    pub(crate) fn planet_verteidiger_v2(&self, pid: usize, fid: FlottenId) -> Vec<Partei> {
        let p = &self.planeten[pid];
        let mut out = vec![Partei::Planet(pid)];
        let angreifer = self.flotten.get(&fid).map(|f| f.besitzer);
        for f in self.flotten.values() {
            // 0.4.8: gestrandete Flotten im Orbit kämpfen an der Seite des Besitzers (nie gegen ihn).
            // Reine Truppenkonvois kämpfen nicht im Orbit (wie Halten unten); ihre Truppen helfen im Rückhalt (hilfsflotten) –
            // Einsatzprobe 25.10.2026: ein gestrandeter Konvoi wurde vor der Landung abgeschossen, seine Truppen halfen nie.
            if f.id != fid && self.orbitflotte_hilft(f, pid, angreifer) && self.bewaffnet(&f.schiffe)
                && !(self.invasion_v1() && crate::invasion::nur_truppenkonvoi(&f.schiffe)) {
                out.push(Partei::Flotte(f.id));
                continue;
            }
            if f.id != fid
                && f.zustand == Flottenzustand::ImOrbit
                && f.mission == Mission::Halten
                && f.ziel == p.koord
                && !self.monde.flottenziel.contains_key(&f.id)
                && (f.besitzer == p.besitzer || self.verbuendet(f.besitzer, p.besitzer))
                && self.bewaffnet(&f.schiffe)
                && !(self.invasion_v1() && crate::invasion::nur_truppenkonvoi(&f.schiffe))
            {
                out.push(Partei::Flotte(f.id));
            }
        }
        out
    }
    pub(crate) fn besetzung_verteidigung_bekaempfen(&mut self, pid: usize, fid: FlottenId) -> bool {
        let Some(f) = self.flotten.get(&fid).cloned() else {
            return false;
        };
        let parteien = self.planet_verteidiger_v2(pid, fid);
        if !self.bewaffnet(&self.planeten[pid].einheiten) && parteien.len() == 1 {
            return true;
        }
        // A current ally in the defending orbit must never be shot automatically.
        if parteien.iter().any(|p|matches!(p,Partei::Flotte(id) if self.verbuendet(f.besitzer,self.flotten[id].besitzer))) {return false;}
        self.abrechnen(pid);
        let gewonnen =
            self.gefecht(&[fid], &parteien, self.planeten[pid].koord) == Sieger::Angreifer;
        if !gewonnen && self.flotten.contains_key(&fid) {
            self.planeten[pid].blockade = None;
            self.heimflug(fid);
        }
        if !self.flotten.contains_key(&fid) {
            self.planeten[pid].blockade = None;
        }
        gewonnen && self.flotten.contains_key(&fid)
    }
    /// Shared dispatch for one fleet and a synchronized attack group.
    fn kampf_am_planeten_v2(&mut self, ids: &[FlottenId]) {
        if ids.is_empty() {
            return;
        }
        let f = self.flotten[&ids[0]].clone();
        let Some(pid) = self.belegung.get(&f.ziel).map(|p| *p as usize) else {
            for id in ids {
                self.umkehren(*id, "das Ziel ist nicht mehr besiedelt");
            }
            return;
        };
        self.abrechnen(pid);
        let owner = self.planeten[pid].besitzer;
        let freundplanet = ids.iter().any(|id| {
            self.flotten[id].besitzer == owner || self.verbuendet(self.flotten[id].besitzer, owner)
        });
        let block = self.planeten[pid].blockade.filter(|id| {
            self.flotten
                .get(id)
                .is_some_and(|b| b.zustand == Flottenzustand::ImOrbit)
        });
        if block.is_none() {
            self.planeten[pid].blockade = None;
        }
        if let Some(bf) = block {
            let b = self.flotten[&bf].clone();
            if ids.len() == 1
                && b.besitzer == f.besitzer
                && !freundplanet
                && (f.mission == Mission::Kampfkolonisieren
                    || (self.invasion_v1() && f.mission == Mission::Bombardieren))
                && !self.planeten[pid].heimat
            {
                if let Err(grund) = self.gegner_angreifbar(f.besitzer, owner) {
                    self.umkehren(f.id, &grund);
                    return;
                }
                if b.schiffe
                    .iter()
                    .zip(f.schiffe)
                    .any(|(a, v)| a.checked_add(v).is_none_or(|n| n > 1_000_000))
                    || b.ladung
                        .iter()
                        .zip(f.ladung)
                        .any(|(a, v)| a.checked_add(v).is_none())
                {
                    self.umkehren(f.id, "die Blockadeflotte kann keine weiteren Schiffe aufnehmen");
                    return;
                }
                let block = self.flotten.get_mut(&bf).unwrap();
                for e in 0..SCHIFFE {
                    block.schiffe[e] += f.schiffe[e];
                }
                for g in 0..GUETER {
                    block.ladung[g] += f.ladung[g];
                }
                if f.mission == Mission::Kampfkolonisieren {
                    block.mission = Mission::Kampfkolonisieren;
                }
                block.siedler += f.siedler;
                self.boden_verstaerken(f.id,bf);
                self.flotten.remove(&f.id);
                self.fachereignis(
                    "blockade_reinforced",
                    f.besitzer,
                    serde_json::json!({"fleet":bf,"reinforcement":f.id}),
                );
                // Existing occupation clocks remain intact; an earlier casualty already removed them.
                self.besetzung_beginnen(pid, bf);
                return;
            }
            if let Some(grund) = ids.iter().find_map(|id| {
                self.gegner_angreifbar(self.flotten[id].besitzer, b.besitzer).err()
            }) {
                for id in ids {
                    self.umkehren(*id, &format!("die Blockadeflotte darf nicht angegriffen werden ({grund})"));
                }
                return;
            }
            if freundplanet
                && ids
                    .iter()
                    .any(|id| self.flotten[id].mission != Mission::Angriff)
            {
                for id in ids {
                    self.umkehren(*id, "der Planet gehört dir oder einem Verbündeten; gegen die Blockade hilft nur die Mission Angriff");
                }
                return;
            }
            for id in ids {
                let sid = self.flotten[id].besitzer;
                self.bruch_pruefen(sid, b.besitzer);
                if !self.spieler[sid as usize].angegriffen.contains(&b.besitzer) {
                    self.spieler[sid as usize].angegriffen.push(b.besitzer);
                }
            }
            self.gefecht(ids, &[Partei::Flotte(bf)], f.ziel);
            if !self.flotten.contains_key(&bf) {
                self.planeten[pid].blockade = None;
                self.vorfall(owner, "blockade", format!("Die Blockade von {} ist gebrochen", f.ziel));
                self.fachereignis(
                    "blockade_broken",
                    owner,
                    serde_json::json!({"planet":pid,"fleet":bf}),
                );
            }
            for id in ids {
                self.heimflug(*id);
            }
            self.raten_neu(pid);
            self.punkte_neu();
            return;
        }
        let sperre = if freundplanet {
            Some("der Planet gehört dir oder einem Verbündeten".to_string())
        } else {
            ids.iter().find_map(|id| self.gegner_angreifbar(self.flotten[id].besitzer, owner).err())
        };
        if let Some(grund) = sperre {
            for id in ids {
                self.umkehren(*id, &grund);
            }
            return;
        }
        let parteien = self.planet_verteidiger_v2(pid, f.id);
        if parteien.iter().any(|p|matches!(p,Partei::Flotte(bf) if ids.iter().any(|id|self.flotten[id].besitzer==self.flotten[bf].besitzer || self.verbuendet(self.flotten[id].besitzer,self.flotten[bf].besitzer)))) {for id in ids {self.umkehren(*id, "eine eigene oder verbündete Flotte verteidigt das Ziel");}return;}
        for id in ids {
            let sid = self.flotten[id].besitzer;
            self.bruch_pruefen(sid, owner);
            if !self.spieler[sid as usize].angegriffen.contains(&owner) {
                self.spieler[sid as usize].angegriffen.push(owner);
            }
        }
        let sieger = self.gefecht(ids, &parteien, f.ziel);
        if sieger == Sieger::Angreifer && f.mission == Mission::Angriff {
            if ids.len() > 1 {
                self.verband_pluendern(ids, pid);
            } else if self.flotten.contains_key(&f.id) {
                self.pluendern(f.id, pid);
            }
        }
        if sieger == Sieger::Angreifer
            && f.mission != Mission::Angriff
            && self.flotten.contains_key(&f.id)
        {
            self.flotten.get_mut(&f.id).unwrap().zustand = Flottenzustand::ImOrbit;
            self.planeten[pid].blockade = Some(f.id);
            // Beide Seiten erfahren, dass der Orbit jetzt gehalten wird (online fehlte das, nur v1 meldete es).
            let name = self.spieler[f.besitzer as usize].name.clone();
            let art = match f.mission {
                Mission::Invasion => "belagert",
                Mission::Bombardieren => "bombardiert",
                Mission::Kampfkolonisieren => "besetzt den Orbit von",
                _ => "blockiert",
            };
            self.vorfall(owner, "blockade", format!("{name} {art} {}", f.ziel));
            self.vorfall(f.besitzer, "blockade", format!("Flotte {} hält den Orbit von {}", f.id, f.ziel));
            self.wecke(owner, Rolle::Stratege, "Blockade", true);
            self.fachereignis(
                "blockade_started",
                f.besitzer,
                serde_json::json!({"planet":pid,"fleet":f.id}),
            );
            if matches!(
                f.mission,
                Mission::Bombardieren | Mission::Kampfkolonisieren
            ) {
                self.bombardement_abschluss(pid, f.id);
            }
        } else {
            for id in ids {
                self.heimflug(*id);
            }
        }
        self.raten_neu(pid);
        self.punkte_neu();
    }
    fn verbandskampf(&mut self, fid: FlottenId) {
        let f = self.flotten[&fid].clone();
        let ids: Vec<_> = self
            .flotten
            .values()
            .filter(|fl| {
                fl.verband == f.verband
                    && fl.mission == Mission::Angriff
                    && fl.zustand == Flottenzustand::Hinflug
                    && fl.ankunft == self.zeit
                    && fl.ziel == f.ziel
            })
            .map(|fl| fl.id)
            .collect();
        if self.kolonisationsregeln_v2() {
            self.kampf_am_planeten_v2(&ids);
            return;
        }
        let Some(zp) = self.belegung.get(&f.ziel).map(|x| *x as usize) else {
            for id in ids {
                self.heimflug(id);
            }
            return;
        };
        self.abrechnen(zp);
        let ziel_sid = self.planeten[zp].besitzer;
        // Ein inzwischen übernommenes eigenes oder verbündetes Ziel wird nicht beschossen.
        if ids.iter().any(|id| {
            let sid = self.flotten[id].besitzer;
            sid == ziel_sid || self.verbuendet(sid, ziel_sid)
        }) {
            for id in ids {
                self.heimflug(id);
            }
            return;
        }
        let mut parteien = Vec::new();
        let mut gegen_blockade = false;
        if let Some(bf) = self.planeten[zp].blockade {
            if let Some(blockade) = self.flotten.get(&bf) {
                if ids.iter().any(|id| {
                    self.flotten[id].besitzer == blockade.besitzer
                        || self.verbuendet(self.flotten[id].besitzer, blockade.besitzer)
                }) {
                    for id in ids {
                        self.heimflug(id);
                    }
                    return;
                }
                parteien.push(Partei::Flotte(bf));
                gegen_blockade = true;
            } else {
                self.planeten[zp].blockade = None;
            }
        }
        if !gegen_blockade {
            parteien.push(Partei::Planet(zp));
            for fl in self.flotten.values() {
                if fl.zustand == Flottenzustand::ImOrbit
                    && fl.mission == Mission::Halten
                    && fl.ziel == f.ziel
                {
                    parteien.push(Partei::Flotte(fl.id));
                }
            }
            let mut besitzer = std::collections::BTreeSet::new();
            for id in &ids {
                besitzer.insert(self.flotten[id].besitzer);
            }
            for sid in besitzer {
                self.bruch_pruefen(sid, ziel_sid);
                if !self.spieler[sid as usize].angegriffen.contains(&ziel_sid) {
                    self.spieler[sid as usize].angegriffen.push(ziel_sid);
                }
            }
        }
        let sieger = self.gefecht(&ids, &parteien, f.ziel);
        if gegen_blockade {
            if self.planeten[zp]
                .blockade
                .is_some_and(|id| !self.flotten.contains_key(&id))
            {
                self.planeten[zp].blockade = None;
            }
        } else if sieger == Sieger::Angreifer {
            self.verband_pluendern(&ids, zp);
        }
        for id in ids {
            self.heimflug(id);
        }
        self.raten_neu(zp);
        self.punkte_neu();
    }

    /// Ein gemeinsamer Beutepool, einmaliger Bunkerschutz und eine einmalige Moralwirkung.
    /// Volksquoten werden nach freiem überlebendem Frachtraum gewichtet.
    fn verband_pluendern(&mut self, ids: &[FlottenId], zp: usize) {
        let r = self.regeln.clone();
        let mut traeger = Vec::new();
        for id in ids {
            if let Some(f) = self.flotten.get(id) {
                if let Ok(plan) =
                    self.flugplan(f.besitzer, f.start as usize, f.ziel, &f.schiffe, 1000)
                {
                    let frei = (plan.kapazitaet - f.ladung.iter().sum::<i64>()).max(0);
                    if frei > 0 {
                        let quote = r
                            .volk(self.spieler[f.besitzer as usize].volk)
                            .pluenderquote
                            .unwrap_or(r.kampf.pluenderquote);
                        traeger.push((*id, f.besitzer, frei, fx(quote)));
                    }
                }
            }
        }
        let gesamt: i64 = traeger.iter().map(|t| t.2).sum();
        if gesamt == 0 {
            return;
        }
        let quote: i128 = traeger.iter().map(|t| t.2 as i128 * t.3).sum::<i128>() / gesamt as i128;
        let schutz = r.bunkerschutz(self.planeten[zp].gebaeude[Gebaeude::Bunker.idx()]);
        let mut pool = [0; GUETER];
        for g in [
            Gut::Erz,
            Gut::Kristall,
            Gut::Deuterium,
            Gut::Nahrung,
            Gut::Legierung,
            Gut::Elektronik,
            Gut::Konsumgut,
            Gut::Xenokristall,
        ] {
            pool[g.idx()] =
                (((self.planeten[zp].bestand[g.idx()] - schutz[g.idx()]).max(0) as i128 * quote)
                    / FX) as i64;
        }
        let summe: i64 = pool.iter().sum();
        if summe > gesamt {
            for g in &mut pool {
                *g = anteil(*g, gesamt, summe);
            }
        }
        let mut ladungen = vec![[0; GUETER]; traeger.len()];
        let mut frei: Vec<_> = traeger.iter().map(|t| t.2).collect();
        for g in 0..GUETER {
            let mut rest = pool[g];
            for i in 0..traeger.len() {
                let n = anteil(pool[g], traeger[i].2, gesamt).min(frei[i]);
                ladungen[i][g] = n;
                frei[i] -= n;
                rest -= n;
            }
            for i in 0..traeger.len() {
                let n = rest.min(frei[i]);
                ladungen[i][g] += n;
                frei[i] -= n;
                rest -= n;
            }
            debug_assert_eq!(rest, 0);
            self.planeten[zp].bestand[g] -= pool[g];
        }
        for (i, (id, sid, _, _)) in traeger.iter().enumerate() {
            for g in 0..GUETER {
                self.flotten.get_mut(id).unwrap().ladung[g] += ladungen[i][g];
            }
            let wert = r.wert(&ladungen[i]);
            self.spieler[*sid as usize].statistik.beute += wert;
            self.vorfall(
                *sid,
                "beute",
                format!(
                    "Verbandsbeute bei {}: eigener Anteil {} (Wert {})",
                    self.planeten[zp].koord,
                    crate::berichte::fracht_text(&ladungen[i]),
                    ganz(wert)
                ),
            );
        }
        if let Some(bericht) = self.kampfberichte.last_mut() {
            bericht.beute = pool;
        }
        let jetzt = self.zeit;
        self.planeten[zp].mali.push(Malus {
            betrag: milli(r.stabilitaet.pluenderung_malus),
            start: jetzt,
            ende: jetzt + r.stabilitaet.pluenderung_tage * TAG,
        });
        self.vorfall(
            self.planeten[zp].besitzer,
            "beute",
            format!(
                "Verbandsangriff auf {}: geplündert {} (Wert {})",
                self.planeten[zp].koord,
                crate::berichte::fracht_text(&pool),
                ganz(r.wert(&pool))
            ),
        );
    }

    fn pluendern(&mut self, fid: FlottenId, zp: usize) {
        let r = self.regeln.clone();
        let f = self.flotten[&fid].clone();
        let quote = r
            .volk(self.spieler[f.besitzer as usize].volk)
            .pluenderquote
            .unwrap_or(r.kampf.pluenderquote);
        let Ok(plan) = self.flugplan(f.besitzer, f.start as usize, f.ziel, &f.schiffe, 1000) else {
            // Ohne Rückflugplan (z. B. Treibstoff fehlt) keine Beute – aber nicht stumm.
            self.vorfall(f.besitzer, "beute", format!("Keine Beute bei {}: Die Flotte {fid} kann keinen Rückflug planen und lädt nichts", f.ziel));
            return;
        };
        let frei = (plan.kapazitaet - f.ladung.iter().sum::<i64>()).max(0);
        let schutz = r.bunkerschutz(self.planeten[zp].gebaeude[Gebaeude::Bunker.idx()]);
        let beute = crate::planer::kampf::beute(&r, &self.planeten[zp].bestand, &schutz, quote, frei);
        for g in 0..GUETER {
            self.planeten[zp].bestand[g] -= beute[g];
            self.flotten.get_mut(&fid).unwrap().ladung[g] += beute[g];
        }
        let wert = r.wert(&beute);
        let st = &r.stabilitaet;
        let jetzt = self.zeit;
        self.planeten[zp].mali.push(Malus {
            betrag: milli(st.pluenderung_malus),
            start: jetzt,
            ende: jetzt + st.pluenderung_tage * TAG,
        });
        if let Some(b) = self.kampfberichte.last_mut() {
            b.beute = beute;
        }
        let opfer = self.planeten[zp].besitzer;
        self.spieler[f.besitzer as usize].statistik.beute += wert;
        let liste: Vec<String> = Gut::ALLE
            .iter()
            .filter(|g| beute[g.idx()] >= M)
            .map(|g| format!("{} {}", ganz(beute[g.idx()]), g))
            .collect();
        let ort = self.planeten[zp].koord;
        self.vorfall(
            f.besitzer,
            "beute",
            format!(
                "Beute bei {ort}: {}",
                if liste.is_empty() {
                    "nichts".into()
                } else {
                    liste.join(", ")
                }
            ),
        );
        self.vorfall(
            opfer,
            "beute",
            format!(
                "{ort} wurde geplündert: {}",
                if liste.is_empty() {
                    "nichts".into()
                } else {
                    liste.join(", ")
                }
            ),
        );
    }

    // ---------------------------------------------------------------- Spionage

    fn spionieren(&mut self, fid: FlottenId) {
        let f = self.flotten[&fid].clone();
        self.scan_aufzeichnen(f.besitzer, f.ziel);
        let nr = self.spionage_nr;
        self.spionage_nr += 1;
        let mut rng = strom(self.startwert, KANAL_SPIONAGE, nr);
        let sonden = f.schiffe[Einheit::Spionagesonde.idx()];
        let jetzt = self.zeit;
        let Some(zp) = self.belegung.get(&f.ziel).map(|x| *x as usize) else {
            // Erkundung eines freien Platzes.
            if let (Some(platz), Some(sys)) =
                (self.platz(f.ziel).cloned(), self.system(f.ziel).cloned())
            {
                let zone=if self.ausbau.version>0 {crate::habitabilitaet::zone(f.ziel.position,self.regeln.welt.plaetze_je_system)} else {platz.zone};
                self.spieler[f.besitzer as usize].erkundet.insert(
                    f.ziel,
                    Erkundung {
                        zeit: jetzt,
                        felder: platz.felder,
                        zone,
                        reich_erz: sys.reich_erz,
                        reich_kristall: sys.reich_kristall,
                        nebel: sys.nebel,
                    },
                );
                // Erkundungsbericht (Karl 22.10.2026: „jetzt müsste ich schon den Bericht haben“): Die Sicht baut
                // daraus unter `berichte` einen Bericht der Art „erkundung“; Sondenzahl und Orbitlage hier festhalten.
                let orbit = self.orbit_lage(f.besitzer, f.ziel);
                if self.ausbau.version > 0 {
                    self.ausbau.scan_details.insert(
                        format!("{}:{}", f.besitzer, f.ziel),
                        serde_json::json!({"zeit": jetzt, "art": "erkundung", "sonden": sonden, "orbit": orbit}),
                    );
                }
                // Ein alter Bericht über einen früheren Besitzer dieses Platzes wäre jetzt falsch.
                self.spieler[f.besitzer as usize].berichte.retain(|b| b.ziel != f.ziel);
                self.scanarchiv_merken(f.besitzer, f.ziel);
                self.vorfall(
                    f.besitzer,
                    "erkundung",
                    format!(
                        "Erkundungsbericht von {}: freier Platz, {} Felder, Zone {}, {}",
                        f.ziel, platz.felder, zone, Self::orbit_satz(&orbit)
                    ),
                );
                self.wecke(f.besitzer, Rolle::Feldherr, "Erkundungsbericht", false);
            }
            self.heimflug(fid);
            return;
        };
        self.abrechnen(zp);
        let ziel = &self.planeten[zp];
        let gegner = ziel.besitzer;
        let tech = |sid: SpielerId| {
            self.spieler[sid as usize].forschung[Forschung::Spionagetechnik.idx()] as i64
        };
        let diff = tech(f.besitzer) - tech(gegner);
        // Probe strength and the research difference govern what this historical scan reveals.
        let info = sonden + diff;
        // Abwehr der Sonden vor dem Bericht würfeln: Sondenzahl, Abwehrchance und Abschuss gehören in den Bericht.
        let ziel_schiffe: i64 = self.planeten[zp].einheiten[..SCHIFFE].iter().sum();
        let mut chance = (ziel_schiffe * sonden * 2).min(1000);
        if diff > 0 {
            chance >>= diff.min(10);
        } else {
            chance = (chance << (-diff).min(10)).min(1000);
        }
        let abgeschossen = (zufall(&mut rng, 1000) as i64) < chance;
        // Halteflotten, Blockaden und Belagerer im Orbit kämpfen mit; ohne sie plant der Angreifer falsch.
        let orbit = if info >= 2 { Some(self.orbit_lage(f.besitzer, f.ziel)) } else { None };
        if self.ausbau.version>0 {
            let mut data=serde_json::Map::new();
            data.insert("sonden".into(),serde_json::json!(sonden));
            data.insert("abwehrchance_promille".into(),serde_json::json!(chance));
            data.insert("abgeschossen".into(),serde_json::json!(abgeschossen));
            if let Some(o)=&orbit {data.insert("orbit".into(),o.clone());}
            data.insert("zeit".into(),serde_json::json!(jetzt));
            data.insert("metadaten".into(),serde_json::json!({"volk":self.spieler[gegner as usize].volk.name(),"zivilisationsstufe":self.spieler[gegner as usize].stufe,"scanstaerke":info,"schwellen":{"schiffe":2,"verteidigung":3,"gebaeude":5,"forschung":7,"vollscan":7}}));
            if info>=7 {let full=self.planet_sicht(zp);for key in ["bevoelkerung","personengruppen","wohnraum","felder","energie","rate","gebaeude_integritaet_prozent","habitabilitaet","raketen","garnison","bodentruppen","stabilitaet"] {data.insert(key.into(),full[key].clone());}
                data.insert("raketenbestand".into(),serde_json::json!(crate::raketen::regeln(&self.regeln).iter().enumerate().map(|(a,r)|(r.key,self.raketen_bestand(zp,a as u8))).collect::<std::collections::BTreeMap<_,_>>()));
            }
            self.ausbau.scan_details.insert(format!("{}:{}",f.besitzer,f.ziel),serde_json::Value::Object(data));
        }
        // Berichtslayout der Welt: 34 Schiffe wie bisher, 35 nur in Welten mit Terraformer (laufende Welten bitgleich).
        let bericht_schiffe = self.bericht_schiffe();
        let bericht = Spionagebericht {
            zeit: jetzt,
            ziel: f.ziel,
            besitzer: gegner,
            bestand: ziel.bestand.to_vec(),
            schiffe: (info >= 2).then(|| ziel.einheiten[..bericht_schiffe].to_vec()),
            verteidigung: (info >= 3).then(|| ziel.einheiten[SCHIFFE..].to_vec()),
            gebaeude: (info >= 5).then(|| ziel.gebaeude.iter().take(if self.aufklaerungsregeln(){GEBAEUDE}else{28}).copied().collect()),
            forschung: (info >= 7).then(|| self.spieler[gegner as usize].forschung.iter().take(if self.aufklaerungsregeln(){self.forschungszahl()}else{17}).copied().collect()),
        };
        let sp = &mut self.spieler[f.besitzer as usize];
        sp.berichte.retain(|b| b.ziel != f.ziel);
        sp.berichte.push(bericht);
        if sp.berichte.len() > 30 {
            sp.berichte.remove(0);
        }
        let name = sp.name.clone();
        self.scanarchiv_merken(f.besitzer, f.ziel);
        self.vorfall(
            gegner,
            "spionage",
            format!(
                "Spionage durch {name} auf {} bemerkt; Sonde gestartet von {}{}",
                f.ziel,
                f.start_koord,
                if abgeschossen {
                    format!("; Abwehr: {} abgeschossen", if sonden == 1 { "1 Sonde".to_string() } else { format!("{sonden} Sonden") })
                } else {
                    String::new()
                }
            ),
        );
        self.wecke(gegner, Rolle::Feldherr, "Spionage bemerkt", false);
        self.wecke(f.besitzer, Rolle::Feldherr, "Spionagebericht", false);
        let sonden_text = if sonden == 1 { "1 Sonde".to_string() } else { format!("{sonden} Sonden") };
        if abgeschossen {
            self.flotten.remove(&fid);
            self.vorfall(
                f.besitzer,
                "spionage",
                format!(
                    "Spionagebericht von {} erhalten, {sonden_text} abgeschossen (Abwehrchance {} %)",
                    f.ziel,
                    crate::berichte::promille_text(chance)
                ),
            );
        } else {
            self.vorfall(
                f.besitzer,
                "spionage",
                format!(
                    "Spionagebericht von {} erhalten; {sonden_text} auf dem Rückflug (Abwehrchance {} %)",
                    f.ziel,
                    crate::berichte::promille_text(chance)
                ),
            );
            self.heimflug(fid);
        }
    }

    // ---------------------------------------------------------------- Kolonisierung

    fn kolonisieren(&mut self, fid: FlottenId) {
        let r = self.regeln.clone();
        let f = self.flotten[&fid].clone();
        let sid = f.besitzer;
        let (Some(platz), Some(sys)) = (self.platz(f.ziel).cloned(), self.system(f.ziel).cloned())
        else {
            self.heimflug(fid);
            return;
        };
        if self.belegung.contains_key(&f.ziel)
            || self.kolonien(sid) >= self.kolonien_erlaubt(sid)
            || self
                .koloniefracht_pruefen(sid, f.ziel, &f.schiffe, &f.ladung)
                .is_err()
        {
            let grund = if !self.kolonieplatz_erst_bei_gruendung() {
                String::new()
            } else if self.belegung.contains_key(&f.ziel) {
                ": der Platz ist inzwischen besiedelt".to_string()
            } else if self.kolonien(sid) >= self.kolonien_erlaubt(sid) {
                format!(": kein freier Kolonieplatz – {}", self.kolonien_voll_text(sid))
            } else {
                ": die Koloniefracht reicht nicht".to_string()
            };
            self.vorfall(
                sid,
                "kolonie",
                format!(
                    "Kolonisierung von {} gescheitert{grund}, die Flotte kehrt um",
                    f.ziel
                ),
            );
            self.heimflug(fid);
            return;
        }
        if self.ausbau.version>0 {
            let settlers=self.ausbau.siedler.get(&fid).cloned().unwrap_or_default();
            if self.siedler_eignung_pruefen(sid,f.ziel,&settlers,f.siedler).is_err() {
                self.vorfall(sid,"kolonie",format!("Gründung auf {} nicht möglich: Siedlergruppe oder Klima ungeeignet. Die Flotte kehrt zurück.",f.ziel));
                self.heimflug(fid);return;
            }
        }
        let zone=if self.ausbau.version>0 {crate::habitabilitaet::zone(f.ziel.position,r.welt.plaetze_je_system)} else {platz.zone};
        let zr = &r.zonen[&zone];
        let typ=Self::kolonieschiff_typ(&f.schiffe).expect("Gründungsflotte besitzt Kolonieschiff");
        let habitate = r
            .einh(typ)
            .kosten
            .get(&Gut::Habitatmodul)
            .copied()
            .unwrap_or(0);
        let mut einheiten = [0i64; EINHEITEN];
        einheiten[..SCHIFFE].copy_from_slice(&f.schiffe);
        // Das Kolonieschiff wird verbraucht: seine Habitatmodule werden der Grundwohnraum.
        einheiten[typ.idx()] -= 1;
        let pid = self.planeten.len();
        self.planeten.push(Planet {
            id: pid as PlanetId,
            besitzer: sid,
            koord: f.ziel,
            zone,
            felder: platz.felder,
            heimat: false,
            faktor: [
                sys.reich_erz,
                sys.reich_kristall,
                milli(zr.deuterium),
                milli(zr.nahrung),
                if self.ausbau.version>0 {crate::habitabilitaet::solar_r(&r,f.ziel)} else {milli(zr.solar)},
            ],
            bestand: f.ladung,
            rate: [0; GUETER],
            stand: self.zeit,
            gebaeude: [0; GEBAEUDE],
            einheiten,
            raketen: [0; 2],
            bevoelkerung: f.siedler,
            stabilitaet: milli(r.welt.start_stabilitaet),
            grundwohnraum: if self.ausbau.industrie_version>0 {(habitate*r.wirtschaft.habitat_wohnraum*M).max(f.siedler*4/3)}else{habitate*r.wirtschaft.habitat_wohnraum*M},
            bauschleife: Vec::new(),
            fertigung: [Vec::new(), Vec::new()],
            fertigung_naechste: [-1, -1],
            prioritaeten: Vec::new(),
            mali: Vec::new(),
            belagerungsmalus: 0,
            blockade: None,
            leer_gemeldet: false,
            gegruendet: self.zeit,
            energie_erzeugung: 0,
            energie_verbrauch: 0,
            arbeit_bedarf: 0,
            arbeit_verfuegbar: 0,
            fk_bedarf: 0,
            fk_verfuegbar: 0,
            nahrung_deckung: 1000,
            konsum_deckung: 0,
            fp_rate: 0,
            stab_ziel: milli(r.welt.start_stabilitaet),
            wohnraum: 0,
        });
        if self.ausbau.version>0 {let people=self.ausbau.siedler.remove(&fid).unwrap_or_default();self.ausbau.personen.insert(pid as PlanetId,people);}
        self.belegung.insert(f.ziel, pid as PlanetId);
        self.spieler[sid as usize].planeten.push(pid as PlanetId);
        // Mitgeführte Bodentruppen werden die Garnison der neuen Kolonie (Karl 24.10.2026).
        self.boden_landen(fid, pid);
        self.flotten.remove(&fid);
        if !r.flottendoktrin.is_empty(){
            let target=r.einh(typ).ladung;
            let p=&mut self.planeten[pid];p.gebaeude[Gebaeude::Lager.idx()]=1;
            while mal(r.wirtschaft.lager_roh,r.wirtschaft.lager_faktor.powi(p.gebaeude[Gebaeude::Lager.idx()]as i32))<target && p.gebaeude[Gebaeude::Lager.idx()]<r.welt.max_gebaeudestufe {p.gebaeude[Gebaeude::Lager.idx()]+=1;}
            p.gebaeude[Gebaeude::Solarkraftwerk.idx()]=1;
            if self.ausbau.personen.get(&(pid as PlanetId)).map(|p|p.organisch()>0).unwrap_or(!r.volk(self.spieler[sid as usize].volk).ohne_nahrung){p.gebaeude[Gebaeude::Farm.idx()]=1;}
            if r.flottendoktrin.get(&typ).is_some_and(|p|p.kolonie_minen==2){p.gebaeude[Gebaeude::Erzmine.idx()]=1;p.gebaeude[Gebaeude::Kristallmine.idx()]=1;}
            // Ablauf v1: Wie die Farm 1 organische Siedler ernährt, deckt die Solarausstattung den Strom synthetischer
            // Siedler auch auf kalten Bahnen (sonst schrumpft eine Synthetenkolonie ab der Landung, Messung 18.10.).
            if self.regeln.ablauf>=1 && self.ausbau.personen.get(&(pid as PlanetId)).is_some_and(|p|p.gesamt()>p.organisch()) {
                for _ in 0..10 {
                    self.raten_neu(pid);
                    let p=&mut self.planeten[pid];
                    if p.energie_erzeugung>=p.energie_verbrauch {break;}
                    p.gebaeude[Gebaeude::Solarkraftwerk.idx()]+=1;
                }
            }
            for _ in 0..if self.ausbau.version>0 {0} else {r.welt.max_gebaeudestufe} {
                self.raten_neu(pid);let p=&mut self.planeten[pid];
                let energy=p.energie_erzeugung<p.energie_verbrauch;let food=self.ausbau.personen.get(&(pid as PlanetId)).map(|p|p.organisch()>0).unwrap_or(!r.volk(self.spieler[sid as usize].volk).ohne_nahrung)&&p.rate[Gut::Nahrung.idx()]<0;
                if !energy&&!food{break;}
                if energy&&p.gebaeude[Gebaeude::Solarkraftwerk.idx()]<r.welt.max_gebaeudestufe{p.gebaeude[Gebaeude::Solarkraftwerk.idx()]+=1;}
                if food&&p.gebaeude[Gebaeude::Farm.idx()]<r.welt.max_gebaeudestufe{p.gebaeude[Gebaeude::Farm.idx()]+=1;}
            }
        }
        self.fachereignis(
            "colony_founded",
            sid,
            serde_json::json!({"planet":pid,"fleet":fid,"target":f.ziel}),
        );
        self.raten_neu(pid);
        self.vorfall(
            sid,
            "kolonie",
            // Siedler und Startfracht nennen (Funktionsprüfung 25.10.2026: die Meldung sagte nur Felder und Zone).
            format!(
                "Kolonie auf {} gegründet: {} Felder, Zone {} · {} Siedler{}",
                f.ziel, platz.felder, zone,
                crate::schiffhandel::credits_text(crate::typen::ganz(self.planeten[pid].bevoelkerung)),
                if f.ladung.iter().any(|n| *n >= M) { format!(" · Startfracht: {}", crate::berichte::fracht_text(&f.ladung)) } else { String::new() }
            ),
        );
        self.wecke(sid, Rolle::Stratege, "Kolonie gegründet", false);
        self.wecke(sid, Rolle::Verwalter, "Kolonie gegründet", false);
        self.punkte_neu();
    }

    // ---------------------------------------------------------------- Eroberung

    pub fn bodentruppen(&self, f: &Flotte) -> i64 {
        let k = &self.regeln.kampf;
        let waffen =
            self.spieler[f.besitzer as usize].forschung[Forschung::Waffentechnik.idx()] as f64;
        mal(
            f.schiffe[Einheit::Truppentransporter.idx()] * k.truppen_je_transporter,
            1.0 + k.tech_je_stufe * waffen,
        )
    }

    pub fn garnison(&self, pid: usize) -> i64 {
        if self.ausbau.industrie_version>0 {let units=self.ausbau.boden.bestand.get(&(pid as u32)).copied().unwrap_or_default();return (0..crate::boden::ANZAHL).map(|i|units[i]*self.regeln.kw().boden.angriff(i)).sum::<i64>()+self.ausbau.boden.miliz.get(&(pid as u32)).map(|m|m.personen.gesamt()/M*4).unwrap_or(0);}
        let k = &self.regeln.kampf;
        let p = &self.planeten[pid];
        let panzer = self.spieler[p.besitzer as usize].forschung[Forschung::Panzerung.idx()] as f64;
        mal(
            p.gebaeude[Gebaeude::Kaserne.idx()] as i64 * k.garnison_je_kaserne,
            1.0 + k.tech_je_stufe * panzer,
        ) + p.bevoelkerung / (100 * M) * k.garnison_je_100_einwohner
    }

    /// Belagerte Kolonien wechseln den Besitzer, wenn die Stabilität gefallen ist und
    /// die Bodentruppen des Angreifers der Garnison überlegen sind.
    pub fn eroberung_pruefen(&mut self) {
        if self.kolonisation.aktiv {
            return;
        }
        let schwelle = milli(self.regeln.stabilitaet.uebernahme_unter);
        for pid in 0..self.planeten.len() {
            let Some(bf) = self.planeten[pid].blockade else {
                continue;
            };
            let Some(f) = self.flotten.get(&bf).cloned() else {
                self.planeten[pid].blockade = None;
                continue;
            };
            if f.mission != Mission::Invasion || self.planeten[pid].heimat {
                continue;
            }
            if self.planeten[pid].stabilitaet < schwelle
                && self.bodentruppen(&f) > self.garnison(pid)
            {
                self.erobern(pid, bf);
            }
        }
    }

    pub(crate) fn erobern(&mut self, pid: usize, fid: FlottenId) {
        let r = self.regeln.clone();
        let f = self.flotten[&fid].clone();
        // Neue Welten (Karl 24.10.2026): auch eine Kampfkolonisierung belegt einen Kolonieplatz – ohne freien Platz kein
        // Besitzerwechsel; die Belagerung endet, die Flotte hält weiter den Orbit.
        if self.kolonieplatz_erst_bei_gruendung() && f.besitzer != self.planeten[pid].besitzer
            && self.kolonien(f.besitzer) >= self.kolonien_erlaubt(f.besitzer)
        {
            let k = self.planeten[pid].koord;
            let alt = self.planeten[pid].besitzer;
            let text = self.kolonien_voll_text(f.besitzer);
            self.vorfall(f.besitzer, "kampfkolonisation", format!("Übernahme von {k} nicht möglich, kein freier Kolonieplatz: {text}. Die Flotte hält weiter den Orbit"));
            let name = self.spieler[f.besitzer as usize].name.clone();
            self.vorfall(alt, "kampfkolonisation", format!("Übernahme von {k} durch {name} gescheitert: kein freier Kolonieplatz beim Angreifer"));
            self.fachereignis("occupation_aborted", alt, serde_json::json!({"planet":pid,"fleet":fid,"attacker":f.besitzer,"defender":alt,"reason":"kein freier Kolonieplatz"}));
            return;
        }
        self.abrechnen(pid);
        // Offene Marktorders auf diesem Planeten enden vor dem Besitzerwechsel.
        let planet_id = self.planeten[pid].id;
        self.orders_stornieren_planet(planet_id);
        let alt = self.planeten[pid].besitzer;
        let neu = f.besitzer;
        let regeln_v2 = self.kolonisationsregeln_v2();
        let neuartig = self.kolonisation.aktiv && (f.mission == Mission::Kampfkolonisieren
            || (self.invasion_v2() && f.mission == Mission::Kolonisieren));
        let p = &mut self.planeten[pid];
        if !neuartig {
            for g in 0..GEBAEUDE {
                p.gebaeude[g] =
                    mal(p.gebaeude[g] as i64, 1.0 - r.kampf.eroberung_stufenverlust) as u8;
            }
        }
        if !regeln_v2 {
            p.bevoelkerung = mal(p.bevoelkerung, r.kampf.eroberung_bevoelkerung).max(100 * M);
        }
        p.einheiten = [0; EINHEITEN];
        for e in 0..SCHIFFE {
            p.einheiten[e] = f.schiffe[e];
        }
        if neuartig {
            if let Some(e)=Self::kolonieschiff_typ(&f.schiffe){p.einheiten[e.idx()]-=1;}
        }
        for g in 0..GUETER {
            p.bestand[g] += f.ladung[g];
        }
        p.bauschleife.clear();
        self.ausbau.parallel.remove(&format!("{}:0",p.id));self.ausbau.parallel.remove(&format!("{}:1",p.id));p.fertigung = [Vec::new(), Vec::new()];
        p.fertigung_naechste = [-1, -1];
        p.prioritaeten.clear();
        p.mali.clear();
        p.belagerungsmalus = 0;
        p.blockade = None;
        p.stabilitaet = milli(r.stabilitaet.unruhen_unter);
        p.besitzer = neu;
        p.raketen = [0; 2];
        self.ausbau.raketensystem.lager.remove(&p.id);
        let k = p.koord;
        let id = p.id;
        self.kolonisation
            .reparaturen
            .retain(|(planet, _), _| *planet != id);
        self.ereignisse.retain(
            |e| !matches!(e.art, EreignisArt::RaketenFertig { planet, .. } if planet == id)
                && !(regeln_v2 && matches!(e.art,EreignisArt::BauFertig{planet}|EreignisArt::FertigungFertig{planet,..} if planet==id)),
        );
        // Invasion v2: Ein Kolonieschiff mit „Kolonisieren“ übernimmt einen verwüsteten Planeten – seine Siedler werden Bewohner.
        if self.invasion_v2() && f.mission == Mission::Kolonisieren && f.siedler > 0 {
            self.planeten[pid].bevoelkerung += f.siedler;
            if let Some(leute) = self.ausbau.siedler.remove(&fid) { self.ausbau.personen.entry(id).or_default().add(&leute); }
        }
        self.boden_landen(fid,pid);
        self.ausbau.boden.auftraege.remove(&id);
        self.ausbau.produktionsserien.remove(&id);
        self.ausbau.fertigung_zahlung.remove(&format!("{pid}:0"));self.ausbau.fertigung_zahlung.remove(&format!("{pid}:1"));
        self.flotten.remove(&fid);
        self.kolonisation.besetzungen.remove(&id);
        self.kolonisation.besetzungsmarken.remove(&id);
        self.kolonisation.besetzungsangreifer.remove(&id);
        self.fachereignis(
            "planet_captured",
            neu,
            serde_json::json!({"planet":id,"fleet":fid,"previous_owner":alt,"owner":neu}),
        );
        self.spieler[alt as usize].planeten.retain(|x| *x != id);
        self.spieler[neu as usize].planeten.push(id);
        let (alt_name, neu_name) = (
            self.spieler[alt as usize].name.clone(),
            self.spieler[neu as usize].name.clone(),
        );
        self.vorfall(
            alt,
            "eroberung",
            format!("{k} wurde von {neu_name} erobert"),
        );
        self.vorfall(neu, "eroberung", format!("Du hast {k} erobert (bisher {alt_name})"));
        self.wecke(alt, Rolle::Stratege, "Kolonie verloren", true);
        self.wecke(neu, Rolle::Stratege, "Kolonie erobert", false);
        self.wecke(neu, Rolle::Verwalter, "Kolonie erobert", false);
        self.mond_planet_erobert(pid, neu);
        self.raten_neu(pid);
        self.punkte_neu();
    }
}
