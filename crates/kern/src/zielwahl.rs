//! Verbindliche Zielreihenfolge für Kampf, Simulator und öffentliche Datenblätter.
//! Kein gespeicherter Zustand: laufende Partien brauchen keinen erneuten Neustart.
use crate::{Einheit, Regelwerk};
use serde::Serialize;

#[derive(Clone, Debug, Serialize)]
pub struct Zielprofil {
    pub primaer: Vec<Einheit>,
    pub sekundaer: Vec<Einheit>,
    pub reserve: Vec<Einheit>,
    pub nicht_erfassbar: Vec<Einheit>,
    pub ziele_je_runde: u8,
    pub schadensfaktoren: std::collections::BTreeMap<Einheit,f64>,
    pub schnellfeuer: std::collections::BTreeMap<Einheit,u32>,
    pub verteilung: String,
    /// Anteil der Zielkanäle für Primär, Sekundär, Reserve (nur wenn die Doktrin ihn setzt; sonst Rangfolge).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gewichte: Option<[u32; 3]>,
}

impl Zielprofil {
    pub fn rang(&self, ziel: Einheit) -> Option<u8> {
        if self.primaer.contains(&ziel) { Some(0) }
        else if self.sekundaer.contains(&ziel) { Some(1) }
        else if self.reserve.contains(&ziel) { Some(2) }
        else { None }
    }
}

fn listen(e: Einheit) -> (&'static [Einheit], &'static [Einheit]) {
    use Einheit::*;
    let (primaer, sekundaer): (&[Einheit], &[Einheit]) = match e {
        LeichterJaeger => (&[Raketenkreuzer], &[KleinerTransporter,GrosserTransporter,Schnellfrachter,Schwerlastfrachter,Handelsfrachter]),
        Abfangjaeger => (&[LeichterJaeger,Sturmjaeger], &[SchwererJaeger]),
        SchwererJaeger => (&[Bomber,StrategischerBomber], &[Plasmawerfer]),
        Sturmjaeger => (&[Zerstoerer,SchwererZerstoerer], &[Abfangzerstoerer]),
        Kreuzer => (&[LeichterJaeger,Sturmjaeger], &[Abfangjaeger,SchwererJaeger]),
        Raketenkreuzer => (&[Schlachtschiff,Sturmschlachtschiff], &[Traegerschlachtschiff,Belagerungsschlachtschiff]),
        Schildkreuzer => (&[SchwererJaeger], &[LeichterJaeger,Abfangjaeger,Sturmjaeger]),
        Flottenkreuzer => (&[Bomber,StrategischerBomber], &[Abfangjaeger,LeichterJaeger,Sturmjaeger]),
        Schlachtschiff => (&[Bomber,StrategischerBomber], &[Kreuzer,Schildkreuzer,Flottenkreuzer]),
        Sturmschlachtschiff => (&[Zerstoerer,SchwererZerstoerer], &[Abfangzerstoerer,Schlachtschiff]),
        Traegerschlachtschiff => (&[LeichterJaeger,Abfangjaeger,Sturmjaeger], &[Bomber,StrategischerBomber,SchwererJaeger]),
        Belagerungsschlachtschiff => (&[Gausskanone,Plasmawerfer], &[Planetenschild,PlanetenschildII,PlanetenschildIII]),
        Bomber => (&[Raketenwerfer,Lasergeschuetz,Ionengeschuetz], &[Gausskanone,Plasmawerfer]),
        StrategischerBomber => (&[Planetenschild,PlanetenschildII,PlanetenschildIII], &[Plasmawerfer,Gausskanone]),
        Zerstoerer => (&[Belagerungsschlachtschiff,Schlachtschiff], &[SchwererJaeger,Kreuzer]),
        Abfangzerstoerer => (&[SchwererJaeger], &[Abfangjaeger,LeichterJaeger,Sturmjaeger]),
        SchwererZerstoerer => (&[Belagerungsschlachtschiff,Schlachtschiff], &[Gausskanone]),
        Raketenwerfer => (&[Raketenkreuzer], &[Kreuzer,Schildkreuzer,Flottenkreuzer]),
        Lasergeschuetz => (&[LeichterJaeger,Abfangjaeger], &[Sturmjaeger,SchwererJaeger]),
        Ionengeschuetz => (&[Schildkreuzer,Schlachtschiff,Sturmschlachtschiff,Traegerschlachtschiff,Belagerungsschlachtschiff], &[Kreuzer,Flottenkreuzer,Zerstoerer,SchwererZerstoerer]),
        Gausskanone => (&[Schlachtschiff,Sturmschlachtschiff,Traegerschlachtschiff,Belagerungsschlachtschiff,Zerstoerer,SchwererZerstoerer], &[Kreuzer,Raketenkreuzer,Schildkreuzer,Flottenkreuzer]),
        Plasmawerfer => (&[Bomber,StrategischerBomber], &[Schlachtschiff,Sturmschlachtschiff,Traegerschlachtschiff,Belagerungsschlachtschiff]),
        // Mondgeneration (nur nach monde_aktivieren im Regelwerk vorhanden).
        Schlachtkreuzer => (&[Schlachtschiff,Sturmschlachtschiff,Traegerschlachtschiff,Kreuzer,Schildkreuzer,Flottenkreuzer,SchwererJaeger], &[Spionagesonde,Solarsonde,KleinerTransporter,GrosserTransporter,Schnellfrachter]),
        AbfangSchlachtkreuzer => (&[LeichterJaeger,Abfangjaeger,Sturmjaeger,Bomber,StrategischerBomber], &[SchwererJaeger,Spionagesonde,Kreuzer]),
        SchwererSchlachtkreuzer => (&[Schlachtschiff,Sturmschlachtschiff,Flottenkreuzer,Raketenkreuzer,Schildkreuzer], &[Kreuzer,Traegerschlachtschiff,Zerstoerer,Schlachtkreuzer]),
        Weltentruemmer => (&[Schlachtschiff,Sturmschlachtschiff,Traegerschlachtschiff,Belagerungsschlachtschiff,Zerstoerer,SchwererZerstoerer,Abfangzerstoerer,Schlachtkreuzer,AbfangSchlachtkreuzer,SchwererSchlachtkreuzer],
            &[Kreuzer,Raketenkreuzer,Schildkreuzer,Flottenkreuzer,Bomber,LeichterJaeger,Spionagesonde,KleinerTransporter,GrosserTransporter,Recycler,Solarsonde,Raketenwerfer,Lasergeschuetz,Ionengeschuetz,Gausskanone]),
        Solarsonde => (&[], &[]),
        // Leichte Bordwaffen dienen der Selbstverteidigung; Schilde feuern nie.
        Spionagesonde | KleinerTransporter | GrosserTransporter | Bergbauschiff |
        Recycler | Kolonieschiff | Truppentransporter | Schnellfrachter |
        Schwerlastfrachter | GrossesKolonieschiff | Handelsfrachter | Pathfinder |
        Bergungsschiff | Terraformer | Planetenschild | PlanetenschildII | PlanetenschildIII =>
            (&[LeichterJaeger,Abfangjaeger,Sturmjaeger], &[KleinerTransporter,GrosserTransporter]),
    };
    (primaer,sekundaer)
}

/// Regelabhängige Ergänzung: Inhalte der Mondgeneration erscheinen nur in aktivierten Regelwerken
/// in vorhandenen Listen, damit alte Partien bitgleich bleiben.
/// Feste Ziellisten des Kerns (Standard für die ConfigWelt und für Welten ohne eigene Listen in der Doktrin).
pub fn feste_listen(e: Einheit) -> (Vec<Einheit>, Vec<Einheit>) {
    let (p, s) = listen(e);
    (p.to_vec(), s.to_vec())
}

fn listen_r(r:&Regelwerk,e:Einheit)->(Vec<Einheit>,Vec<Einheit>){
    // Eigene Listen der Doktrin (flottenausbau.json profiles.<einheit>.primaer/sekundaer) gelten genau so, wie sie dastehen.
    if let Some(d)=r.flottendoktrin.get(&e).filter(|d|d.primaer.is_some()||d.sekundaer.is_some()) {
        let (p,s)=listen(e);
        return (d.primaer.clone().unwrap_or_else(||p.to_vec()), d.sekundaer.clone().unwrap_or_else(||s.to_vec()));
    }
    let (p,s)=listen(e);let (p,mut s)=(p.to_vec(),s.to_vec());
    if r.monde.is_some() && e==Einheit::Zerstoerer {s.push(Einheit::Schlachtkreuzer);}
    (p,s)
}

pub fn schadensfaktor(r:&Regelwerk,e:Einheit,z:Einheit)->f64 {
    let (primaer,sekundaer)=listen_r(r,e);
    let basis=if primaer.contains(&z){r.kw().kampf.primaer_faktor}else if sekundaer.contains(&z){r.kw().kampf.sekundaer_faktor}else{return 1.0;};
    r.flottendoktrin.get(&e).and_then(|p|p.schadensfaktoren.get(&z)).copied().filter(|f|f.is_finite() && *f>1.0).unwrap_or(basis)
}

pub fn profil(r: &Regelwerk, e: Einheit) -> Zielprofil {
    let (primaer,sekundaer)=listen_r(r,e);
    let gewichte = r.flottendoktrin.get(&e).and_then(|d| d.verteilung).map(|v| v.gewichte());
    let bewaffnet = r.einh(e).angriff > 0;
    let erlaubt = |z: &Einheit| bewaffnet && r.ziel_erfassbar(e, *z);
    // Mehrfachfeuer (Kernwert kampf.schnellfeuer_springt): nach einem Abschuss weiter auf ein neues Ziel derselben Art.
    let springt = r.kw().kampf.schnellfeuer_springt;
    let sf = if springt { "Schnellfeuer schießt auf dasselbe Ziel weiter, bis es fällt, springt dann auf ein neues Ziel derselben Art und verbraucht keinen weiteren Zielkanal (höchstens 64 Schüsse je Kanal)." }
        else { "Schnellfeuer bleibt auf demselben Ziel und verbraucht keinen weiteren Zielkanal (höchstens 64 Schüsse je Kanal)." };
    let sf2 = if springt { "Schnellfeuer schießt auf dasselbe Ziel weiter, bis es fällt, und springt dann auf ein neues Ziel derselben Art (höchstens 64 Schüsse je Kanal)." }
        else { "Schnellfeuer bleibt auf demselben Ziel (höchstens 64 Schüsse je Kanal)." };
    let primaer: Vec<_> = primaer.iter().copied().filter(erlaubt).collect();
    let sekundaer: Vec<_> = sekundaer.iter().copied().filter(erlaubt).filter(|z| !primaer.contains(z)).collect();
    let reserve = Einheit::ALLE.into_iter().filter(erlaubt).filter(|z| !primaer.contains(z) && !sekundaer.contains(z)).collect();
    let nicht_erfassbar = Einheit::ALLE.into_iter().filter(|z| !erlaubt(z)).collect();
    Zielprofil {
        schnellfeuer: r.einh(e).schnellfeuer.iter().filter(|(z,n)| **n>1 && (primaer.contains(z)||sekundaer.contains(z))).map(|(z,n)|(*z,(*n).min(64))).collect(),
        schadensfaktoren: primaer.iter().chain(&sekundaer).map(|z|(*z,schadensfaktor(r,e,*z))).collect(),
        primaer, sekundaer, reserve, nicht_erfassbar,
        ziele_je_runde: if bewaffnet { r.flottendoktrin.get(&e).map_or(1, |p| p.ziele.clamp(1,4)) } else { 0 },
        verteilung: match gewichte {
            None => format!("Je Einheit und Runde zuerst lebende, noch nicht anvisierte Primärziele; danach Sekundärziele, danach Reserve. Innerhalb einer Stufe hat jedes einzelne Ziel dieselbe Chance. Primär- und Sekundärziele erhalten den ausgewiesenen Bonus. Alle anderen Ziele werden zufällig mit 1× Grundschaden ohne Zielbonus, Abschlag oder Schnellfeuer getroffen. Jeder Zielkanal hat sein eigenes Schadensbudget. {sf}").to_string(),
            Some([p, s, rv]) => format!("Je Einheit und Runde wählt jeder Zielkanal die Stufe zufällig nach Anteil: Primär {p}, Sekundär {s}, Reserve {rv} (Stufen ohne lebendes, noch nicht anvisiertes Ziel fallen heraus; sind alle übrigen Anteile 0, gilt die Rangfolge). Innerhalb einer Stufe hat jedes einzelne Ziel dieselbe Chance. Primär- und Sekundärziele erhalten den ausgewiesenen Bonus, Reserveziele 1× Grundschaden ohne Schnellfeuer. Jeder Zielkanal hat sein eigenes Schadensbudget; {sf2}"),
        },
        gewichte,
    }
}
