//! Sternenepoche: Epochen mit Skriptbots fahren, Balance messen, Protokolle nachspielen
//! und die Engine über Standardein- und -ausgabe an den Orchestrator anbinden.

mod bots;

use bots::{Bot, Bottyp};
use kern::regeltext;
use kern::typen::*;
use kern::welt::{Logeintrag, Sieger, Welt};
use kern::Regelwerk;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::io::{BufRead, Write};
use std::path::Path;

const REGELN_EINGEBAUT: &str = include_str!("../../../regeln/regelwerk.ron");

/// Text des Regelwerks: die Datei hinter `--regeln` oder das eingebaute.
fn regeln_text(pfad: Option<&str>) -> Result<String, String> {
    match pfad {
        Some(p) => std::fs::read_to_string(p).map_err(|e| format!("{p}: {e}")),
        None => Ok(REGELN_EINGEBAUT.to_string()),
    }
}

fn regeln_laden(pfad: Option<&str>, tage: Option<i64>) -> Result<Regelwerk, String> {
    let text = regeln_text(pfad)?;
    let mut r = Regelwerk::laden(&text)?;
    if let Some(t) = tage {
        r.welt.epoche_tage = t;
    }
    Ok(r)
}

/// Ein Lauf: die Welt, die Bots der nicht von Agenten gesteuerten Spieler und Messwerte.
#[derive(Serialize, Deserialize)]
struct Lauf {
    welt_bytes: Vec<u8>,
    #[serde(skip)]
    welt: Option<Welt>,
    bots: BTreeMap<SpielerId, Bot>,
    /// Zeitpunkt, zu dem jeder Spieler die Stufen II bis V erreicht hat (0: noch nicht).
    stufenzeit: Vec<[SimZeit; 6]>,
    erste_kolonie: Vec<SimZeit>,
    startwert: u64,
    ki: Vec<SpielerId>,
}

impl Lauf {
    fn neu(regeln: Regelwerk, startwert: u64, spieler: usize, ki: Option<Vec<SpielerId>>, typen: &[Bottyp]) -> Result<Lauf, String> {
        let mut welt = Welt::neu(regeln, startwert, spieler)?;
        let ki: Vec<SpielerId> = ki.unwrap_or_else(|| (0..spieler as SpielerId).collect());
        let mut bots = BTreeMap::new();
        let mut i = 0usize;
        for sid in 0..spieler as SpielerId {
            if !ki.contains(&sid) {
                welt.spieler[sid as usize].ki = false;
                bots.insert(sid, Bot::neu(typen[i % typen.len()], sid));
                i += 1;
            }
        }
        welt.fenster_vorbereiten();
        let mut lauf = Lauf {
            welt_bytes: Vec::new(),
            welt: Some(welt),
            bots,
            stufenzeit: vec![[0; 6]; spieler],
            erste_kolonie: vec![0; spieler],
            startwert,
            ki,
        };
        lauf.bots_ziehen();
        Ok(lauf)
    }

    fn w(&mut self) -> &mut Welt {
        self.welt.as_mut().expect("Welt geladen")
    }

    /// Bots handeln im aktuellen Fenster in der ausgelosten Reihenfolge, vor den Agenten.
    fn bots_ziehen(&mut self) {
        let welt = self.welt.as_mut().expect("Welt geladen");
        for sid in welt.reihenfolge.clone() {
            if let Some(bot) = self.bots.get_mut(&sid) {
                bots::zug(welt, sid, bot);
            }
        }
        for sp in &welt.spieler {
            let i = sp.id as usize;
            let st = sp.stufe as usize;
            if self.stufenzeit[i][st] == 0 && st > 1 {
                self.stufenzeit[i][st] = welt.zeit;
            }
            if self.erste_kolonie[i] == 0 && sp.planeten.len() > 1 {
                self.erste_kolonie[i] = welt.zeit;
            }
        }
    }

    fn weiter(&mut self) -> bool {
        let ok = self.w().schritt();
        if ok {
            self.bots_ziehen();
        }
        ok
    }

    fn speichern(&mut self, pfad: &str) -> Result<(), String> {
        self.welt_bytes = self.w().zu_bytes();
        let bytes = bincode::serialize(&*self).map_err(|e| e.to_string())?;
        self.welt_bytes.clear();
        std::fs::write(pfad, bytes).map_err(|e| format!("{pfad}: {e}"))
    }

    fn laden(pfad: &str) -> Result<Lauf, String> {
        let bytes = std::fs::read(pfad).map_err(|e| format!("{pfad}: {e}"))?;
        let mut lauf: Lauf = bincode::deserialize(&bytes).map_err(|e| format!("{pfad}: {e}"))?;
        lauf.welt = Some(Welt::aus_bytes(&lauf.welt_bytes)?);
        lauf.welt_bytes.clear();
        Ok(lauf)
    }
}

/// Spielt ein Aktionsprotokoll nach: gleicher Startwert und gleiches Protokoll ergeben denselben Zustand.
/// Spielt das Protokoll nach bis zur Zeit `bis`: das Epochenende oder der Zeitpunkt, an dem ein Lauf angehalten wurde.
fn nachspielen(regeln: Regelwerk, startwert: u64, spieler: usize, ki: &[SpielerId], log: &[Logeintrag], bis: SimZeit) -> Result<Welt, String> {
    let mut welt = Welt::neu(regeln, startwert, spieler)?;
    for sid in 0..spieler as SpielerId {
        if !ki.contains(&sid) {
            welt.spieler[sid as usize].ki = false;
        }
    }
    welt.fenster_vorbereiten();
    for e in log {
        while welt.zeit < e.zeit {
            if !welt.schritt() {
                break;
            }
        }
        if welt.zeit != e.zeit {
            return Err(format!("Protokolleintrag zur Zeit {} passt in kein Fenster", e.zeit));
        }
        let v: Value = serde_json::from_str(&e.aktion).map_err(|x| x.to_string())?;
        if v["typ"] == "aufruf_ende" {
            welt.aufruf_ende(e.spieler, e.rolle, v["notiz"].as_str(), v["wecker_sekunden"].as_i64(), &texte(&v["hinweise"]));
        } else {
            welt.handeln(e.spieler, e.rolle, &v);
        }
    }
    while welt.zeit < bis && welt.schritt() {}
    Ok(welt)
}

fn texte(v: &Value) -> Vec<String> {
    v.as_array().map(|a| a.iter().filter_map(|x| x.as_str().map(|s| s.to_string())).collect()).unwrap_or_default()
}

fn arg(args: &[String], name: &str) -> Option<String> {
    args.iter().position(|a| a == name).and_then(|i| args.get(i + 1)).cloned()
}

fn zahl<T: std::str::FromStr>(args: &[String], name: &str, standard: T) -> T {
    arg(args, name).and_then(|s| s.parse().ok()).unwrap_or(standard)
}

fn typen_aus(args: &[String]) -> Result<Vec<Bottyp>, String> {
    match arg(args, "--typen") {
        None => Ok(Bottyp::ALLE.to_vec()),
        Some(s) => s.split(',').map(|t| Bottyp::aus_name(t.trim()).ok_or_else(|| format!("Bottyp '{t}' gibt es nicht"))).collect(),
    }
}

fn median(mut v: Vec<i64>) -> Option<i64> {
    if v.is_empty() {
        return None;
    }
    v.sort();
    Some(v[v.len() / 2])
}

fn tag(t: Option<i64>) -> String {
    t.map(|x| format!("{:>5.1}", x as f64 / TAG as f64)).unwrap_or_else(|| "    -".into())
}

fn jsonl<T: Serialize>(pfad: &Path, zeilen: &[T]) -> Result<(), String> {
    let mut f = std::io::BufWriter::new(std::fs::File::create(pfad).map_err(|e| format!("{}: {e}", pfad.display()))?);
    for z in zeilen {
        serde_json::to_writer(&mut f, z).map_err(|e| e.to_string())?;
        f.write_all(b"\n").map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// Eine Zeile zum Stand eines Spielers, fürs Balancing.
fn spur_zeile(welt: &Welt, sid: SpielerId) -> String {
    let r = &welt.regeln;
    let sp = &welt.spieler[sid as usize];
    let p = &welt.planeten[sp.heimat as usize];
    let punkt = r.wertung.einheit * M;
    let mut lager = 0i64;
    let mut verteidigung = 0i64;
    let mut schiffe = 0i64;
    for pid in &sp.planeten {
        lager += r.wert(&welt.bestand_jetzt(*pid as usize));
        let q = &welt.planeten[*pid as usize];
        schiffe += q.einheiten[..SCHIFFE].iter().sum::<i64>();
        for e in SCHIFFE..EINHEITEN {
            verteidigung += q.einheiten[e] * r.wert(&r.kosten_einheit(Einheit::ALLE[e], sp.volk));
        }
    }
    schiffe += welt.flotten.values().filter(|f| f.besitzer == sid).map(|f| f.schiffe.iter().sum::<i64>()).sum::<i64>();
    let angegriffen = welt.kampfberichte.iter().filter(|k| k.verteidiger.contains(&sid)).count();
    let pu = &sp.punkte;
    format!(
        "Tag {:>3} {} St {} Pkt {:>6} (W {:>5} F {:>5} M {:>5} Z {:>4}) Ew {:>6} Stab {:>3} Cr {:>7}{} Schiffe {:>5} Flotte {:>5} Vert {:>5} Lager {:>5} Beute {:>6} Verlust {:>5} angegr. {:>3} Kol {}",
        welt.zeit / TAG, sp.volk.name(), sp.stufe, pu.gesamt(), pu.wirtschaft, pu.forschung, pu.militaer, pu.zivilisation,
        ganz(welt.einwohner(sid)), p.stabilitaet / M, ganz(sp.credits), if sp.schuld_seit.is_some() { "!" } else { " " },
        schiffe, welt.flottenwert(sid) / punkt, verteidigung / punkt, lager / punkt, sp.statistik.beute / punkt,
        sp.statistik.verluste / punkt, angegriffen, sp.planeten.len() - 1,
    ) + &sp.planeten.iter().map(|pid| {
        let q = &welt.planeten[*pid as usize];
        format!(" | {}{} F {}/{} A {}/{} Q {} Stab {} XE {} RH {} GT {} Erz {} Leg {} Kon {} KW {} Xeno {}/{} je h, Lager {} Grenze {} FK {}/{} Fus {} Aka {} Strom {}/{}", q.koord, if welt.system(q.koord).map(|s| s.nebel).unwrap_or(false) { "N" } else { "" },
            welt.felder_belegt(*pid as usize), welt.felder_gesamt(*pid as usize),
            ganz(q.arbeit_bedarf), ganz(q.arbeit_verfuegbar), q.bauschleife.len(), q.stabilitaet / M,
            q.gebaeude[Gebaeude::Xenoextraktor.idx()], q.gebaeude[Gebaeude::Raumhafen.idx()], q.einheiten[Einheit::GrosserTransporter.idx()],
            ganz(welt.bestand_jetzt(*pid as usize)[Gut::Erz.idx()]), ganz(welt.bestand_jetzt(*pid as usize)[Gut::Legierung.idx()]),
            ganz(welt.bestand_jetzt(*pid as usize)[Gut::Konsumgut.idx()]), q.gebaeude[Gebaeude::Konsumgueterwerk.idx()],
            ganz(welt.bestand_jetzt(*pid as usize)[Gut::Xenokristall.idx()]), ganz(q.rate[Gut::Xenokristall.idx()]), q.gebaeude[Gebaeude::Lager.idx()], ganz(welt.lagergrenze(*pid as usize)[Gut::Xenokristall.idx()]),
            ganz(q.fk_bedarf), ganz(q.fk_verfuegbar), q.gebaeude[Gebaeude::Fusionskraftwerk.idx()], q.gebaeude[Gebaeude::Akademie.idx()], ganz(q.energie_erzeugung), ganz(q.energie_verbrauch))
    }).collect::<String>() + &format!(" | Forschung {:?}", sp.forschung_aktiv.as_ref().map(|a| a.forschung.name()))
        + &format!(" | offen: {}", welt.stufen_bedingungen(sid).into_iter().filter(|(_, ok)| !ok).map(|(t, _)| t).collect::<Vec<_>>().join("; "))
        + &format!(" | Astro {} erlaubt {} Plaetze {} OW {} KS {} Bauteile {}/{}", sp.forschung[Forschung::Astrophysik.idx()], welt.kolonien_erlaubt(sid), welt.flottenplaetze(sid),
            p.gebaeude[Gebaeude::Orbitalwerft.idx()], p.einheiten[Einheit::Kolonieschiff.idx()],
            ganz(welt.bestand_jetzt(sp.heimat as usize)[Gut::Antriebskern.idx()]), ganz(welt.bestand_jetzt(sp.heimat as usize)[Gut::Habitatmodul.idx()]))
        + &format!(" | Xeno {}", sp.planeten.iter().map(|p| ganz(welt.bestand_jetzt(*p as usize)[Gut::Xenokristall.idx()])).sum::<i64>())
}

/// Fährt eine ganze Epoche mit Bots und sammelt das Aktionsprotokoll.
fn epoche_fahren(regeln: Regelwerk, startwert: u64, spieler: usize, typen: &[Bottyp], spur: Option<SpielerId>) -> Result<(Lauf, Vec<Logeintrag>), String> {
    let mut lauf = Lauf::neu(regeln, startwert, spieler, Some(Vec::new()), typen)?;
    let mut log = lauf.w().log_abholen();
    while lauf.weiter() {
        log.extend(lauf.w().log_abholen());
        if let Some(sid) = spur {
            if lauf.w().zeit % (15 * TAG) == 0 {
                let typ = lauf.bots.get(&sid).map(|b| b.typ);
                let w = lauf.w();
                let mut zeile = spur_zeile(w, sid);
                if let Some(typ) = typ {
                    for pid in &w.spieler[sid as usize].planeten {
                        zeile += &format!(" | {} {}", w.planeten[*pid as usize].koord, bots::bau_diagnose(w, sid, *pid as usize, typ));
                    }
                }
                println!("{zeile}");
            }
        }
    }
    log.extend(lauf.w().log_abholen());
    Ok((lauf, log))
}

fn bericht(lauf: &mut Lauf) {
    let typ_von: BTreeMap<SpielerId, Bottyp> = lauf.bots.iter().map(|(s, b)| (*s, b.typ)).collect();
    let stufenzeit = lauf.stufenzeit.clone();
    let erste_kolonie = lauf.erste_kolonie.clone();
    let welt = lauf.w();
    println!("\nBottyp     Spieler   Punkte  Stufe  Tag II  Tag III  Tag IV  Tag V  Kolonie  Angriffe    Beute  Verluste  abgelehnt");
    for typ in Bottyp::ALLE {
        let ids: Vec<SpielerId> = typ_von.iter().filter(|(_, t)| **t == typ).map(|(s, _)| *s).collect();
        if ids.is_empty() {
            continue;
        }
        let n = ids.len() as i64;
        let mittel = |f: &dyn Fn(&kern::welt::Spieler) -> i64| ids.iter().map(|s| f(&welt.spieler[*s as usize])).sum::<i64>() / n;
        let zeit = |st: usize| median(ids.iter().map(|s| stufenzeit[*s as usize][st]).filter(|t| *t > 0).collect());
        let kolonie = median(ids.iter().map(|s| erste_kolonie[*s as usize]).filter(|t| *t > 0).collect());
        let aktionen: u32 = ids.iter().map(|s| welt.spieler[*s as usize].statistik.aktionen).sum();
        let abgelehnt: u32 = ids.iter().map(|s| welt.spieler[*s as usize].statistik.abgelehnt).sum();
        println!(
            "{:<10} {:>7} {:>8} {:>6.1} {}   {}  {}  {}    {} {:>9} {:>8} {:>9} {:>9.1}%",
            typ.name(),
            n,
            mittel(&|s| s.punkte.gesamt()),
            ids.iter().map(|s| welt.spieler[*s as usize].stufe as f64).sum::<f64>() / n as f64,
            tag(zeit(2)),
            tag(zeit(3)),
            tag(zeit(4)),
            tag(zeit(5)),
            tag(kolonie),
            ids.iter().map(|s| welt.spieler[*s as usize].statistik.angriffe).sum::<u32>(),
            mittel(&|s| s.statistik.beute / M),
            mittel(&|s| s.statistik.verluste / M),
            if aktionen > 0 { abgelehnt as f64 * 100.0 / aktionen as f64 } else { 0.0 },
        );
    }
    println!("
Volk        Spieler   Punkte  Stufe  Tag II  Tag III  Tag IV");
    for volk in Volk::ALLE {
        let ids: Vec<usize> = welt.spieler.iter().filter(|s| s.volk == volk).map(|s| s.id as usize).collect();
        if ids.is_empty() {
            continue;
        }
        let n = ids.len() as i64;
        let zeit = |st: usize| median(ids.iter().map(|s| stufenzeit[*s][st]).filter(|t| *t > 0).collect());
        println!(
            "{:<11} {:>7} {:>8} {:>6.1} {}   {}  {}",
            volk.name(),
            n,
            ids.iter().map(|s| welt.spieler[*s].punkte.gesamt()).sum::<i64>() / n,
            ids.iter().map(|s| welt.spieler[*s].stufe as f64).sum::<f64>() / n as f64,
            tag(zeit(2)),
            tag(zeit(3)),
            tag(zeit(4)),
        );
    }
    // Wie ergeht es Angreifern gegen welchen Verteidigertyp?
    let mut gegen: BTreeMap<Bottyp, (i64, i64, i64, i64)> = BTreeMap::new();
    let r = welt.regeln.clone();
    for k in &welt.kampfberichte {
        let Some(vt) = k.verteidiger.first().and_then(|v| typ_von.get(v)) else {
            continue;
        };
        let e = gegen.entry(*vt).or_insert((0, 0, 0, 0));
        e.0 += 1;
        if k.sieger == Sieger::Angreifer {
            e.1 += 1;
        }
        e.2 += r.wert(&k.beute) / M;
        let volk = welt.spieler[k.angreifer as usize].volk;
        for i in 0..SCHIFFE {
            e.3 += (k.ang_vorher[i] - k.ang_nachher[i]) * r.wert(&r.kosten_einheit(Einheit::ALLE[i], volk)) / M;
        }
    }
    if !gegen.is_empty() {
        println!("\nAngriffe nach Typ des Verteidigers");
        println!("Verteidiger  Kämpfe  Siege des Angreifers  Beute je Kampf  Verlust des Angreifers je Kampf");
        for (t, (n, siege, beute, verlust)) in gegen {
            println!("{:<12} {:>6} {:>20.0}% {:>15} {:>31}", t.name(), n, siege as f64 * 100.0 / n as f64, beute / n, verlust / n);
        }
    }
}

fn cmd_epoche(args: &[String]) -> Result<(), String> {
    let startwert: u64 = zahl(args, "--startwert", 1);
    let spieler: usize = zahl(args, "--spieler", 50);
    let tage = arg(args, "--tage").and_then(|s| s.parse().ok());
    let typen = typen_aus(args)?;
    let regeln = regeln_laden(arg(args, "--regeln").as_deref(), tage)?;
    let beginn = std::time::Instant::now();
    let spur: Option<SpielerId> = arg(args, "--spur").and_then(|s| s.parse().ok());
    let (mut lauf, log) = epoche_fahren(regeln.clone(), startwert, spieler, &typen, spur)?;
    let dauer = beginn.elapsed();
    let hash = lauf.w().hash();
    println!(
        "Epoche: Startwert {startwert}, {spieler} Spieler, {} Spieltage, {} Aktionen, {} Kämpfe, Rechenzeit {:.1} s",
        regeln.welt.epoche_tage,
        log.len(),
        lauf.w().kampfberichte.len(),
        dauer.as_secs_f64()
    );
    println!("Regelwerk {} ({}), Zustandshash {}", regeln.version, &regeln.hash[..12], &hash[..16]);
    bericht(&mut lauf);
    let mut gruende: BTreeMap<String, u32> = BTreeMap::new();
    for e in log.iter().filter(|e| !e.ok) {
        let mut kurz = String::new();
        for c in e.text.chars().take(70) {
            // Zahlen vereinheitlichen, damit gleiche Gründe zusammenfallen.
            if c.is_ascii_digit() {
                if !kurz.ends_with('#') {
                    kurz.push('#');
                }
            } else {
                kurz.push(c);
            }
        }
        let typ = serde_json::from_str::<Value>(&e.aktion).ok().and_then(|v| v["typ"].as_str().map(|s| s.to_string())).unwrap_or_default();
        *gruende.entry(format!("{typ}: {kurz}")).or_default() += 1;
    }
    if !gruende.is_empty() {
        let mut v: Vec<(u32, String)> = gruende.into_iter().map(|(t, n)| (n, t)).collect();
        v.sort_by(|a, b| b.cmp(a));
        println!("\nHäufigste Ablehnungsgründe");
        for (n, t) in v.into_iter().take(8) {
            println!("{n:>6}  {t}");
        }
    }
    println!("\nRangliste (erste zehn)");
    for (rang, name, punkte, stufe) in lauf.w().rangliste().into_iter().take(10) {
        let typ = lauf.w().spieler_nach_name(&name).ok().and_then(|s| lauf.bots.get(&s)).map(|b| b.typ.name()).unwrap_or("-");
        println!("{rang:>3}. {name:<14} {punkte:>8} Punkte  Stufe {stufe}  {typ}");
    }
    if args.iter().any(|a| a == "--pruefen") {
        let (mut zweiter, _) = epoche_fahren(regeln.clone(), startwert, spieler, &typen, None)?;
        let h2 = zweiter.w().hash();
        let h3 = nachspielen(regeln.clone(), startwert, spieler, &[], &log, SimZeit::MAX)?.hash();
        println!("\nDeterminismus: zweiter Lauf {}, Nachspielen aus dem Protokoll {}",
            if h2 == hash { "gleich" } else { "ABWEICHEND" },
            if h3 == hash { "gleich" } else { "ABWEICHEND" });
        if h2 != hash || h3 != hash {
            return Err("Determinismusprüfung fehlgeschlagen".into());
        }
    }
    if let Some(aus) = arg(args, "--aus") {
        let dir = Path::new(&aus);
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
        let bots: BTreeMap<String, &str> = lauf.bots.iter().map(|(s, b)| (s.to_string(), b.typ.name())).collect();
        let stat = statistik(&mut lauf);
        let welt = lauf.w();
        jsonl(&dir.join("protokoll.jsonl"), &log)?;
        jsonl(&dir.join("tageswerte.jsonl"), &welt.tageswerte)?;
        jsonl(&dir.join("kampfberichte.jsonl"), &welt.kampfberichte)?;
        jsonl(&dir.join("register.jsonl"), &welt.register)?;
        // Das Regelwerk gehört zum Lauf: so lässt er sich auch nach späteren Regeländerungen nachspielen.
        std::fs::write(dir.join("regelwerk.ron"), regeln_text(arg(args, "--regeln").as_deref())?).map_err(|e| e.to_string())?;
        let schluss = json!({"startwert": startwert, "spieler": spieler, "ki": [], "tage": regeln.welt.epoche_tage,
            "regel_version": regeln.version, "regel_hash": regeln.hash, "hash": hash, "log_hash": welt.log_hash, "bots": bots, "statistik": stat,
            "rangliste": welt.rangliste(), "welt": weltbild(welt)});
        std::fs::write(dir.join("schluss.json"), serde_json::to_string_pretty(&schluss).unwrap()).map_err(|e| e.to_string())?;
        println!("\nDaten in {}", dir.display());
    }
    Ok(())
}

/// Kennzahlen eines Bottyps über alle Epochen eines Balance-Laufs.
#[derive(Default)]
struct Typwerte {
    punkte: Vec<i64>,
    teil: [i64; 4],
    rang: Vec<i64>,
    lager: i64,
    verteidigung: i64,
    angegriffen: i64,
    verloren: i64,
    pluenderungen: i64,
    stufe: [Vec<i64>; 6],
    /// Punkte 60 Tage vor Ende, zum Vergleich mit dem Endstand.
    vorher: i64,
}

/// Mittelwerte je Typ für die Ausgabe und den Vergleich mit den Balancezielen.
struct Typbild {
    typ: Bottyp,
    punkte: i64,
    rang: f64,
}

/// Ergebnis eines Balance-Laufs: Kennzahlen je Bottyp, Kämpfe je Typ des Verteidigers und Kennzahlen je Volk.
type Messung = (BTreeMap<Bottyp, Typwerte>, BTreeMap<Bottyp, (i64, i64, i64, i64)>, BTreeMap<Volk, Typwerte>);

fn balance_messen(regeln: &Regelwerk, laeufe: u64, spieler: usize, typen: &[Bottyp]) -> Result<Messung, String> {
    let fertig = epochen_parallel(regeln, laeufe, spieler, typen)?;
    let mut werte: BTreeMap<Bottyp, Typwerte> = BTreeMap::new();
    let mut voelker: BTreeMap<Volk, Typwerte> = BTreeMap::new();
    let mut gegen: BTreeMap<Bottyp, (i64, i64, i64, i64)> = BTreeMap::new();
    for mut lauf in fertig {
        let typ_von: BTreeMap<SpielerId, Bottyp> = lauf.bots.iter().map(|(s, b)| (*s, b.typ)).collect();
        let stufenzeit = lauf.stufenzeit.clone();
        let welt = lauf.w();
        let r = welt.regeln.clone();
        let punkt = r.wertung.einheit * M;
        let spaet = (welt.zeit / TAG - 60).max(1);
        let mut vorher: BTreeMap<SpielerId, i64> = BTreeMap::new();
        for t in welt.tageswerte.iter().filter(|t| t.tag == spaet) {
            vorher.insert(t.spieler, t.punkte.gesamt());
        }
        for (sid, typ) in &typ_von {
            let sp = &welt.spieler[*sid as usize];
            let w = werte.entry(*typ).or_default();
            w.punkte.push(sp.punkte.gesamt());
            w.teil[0] += sp.punkte.wirtschaft;
            w.teil[1] += sp.punkte.forschung;
            w.teil[2] += sp.punkte.militaer;
            w.teil[3] += sp.punkte.zivilisation;
            w.rang.push(sp.rang as i64);
            w.vorher += vorher.get(sid).copied().unwrap_or(0);
            for pid in &sp.planeten {
                let p = &welt.planeten[*pid as usize];
                w.lager += r.wert(&welt.bestand_jetzt(*pid as usize)) / punkt;
                for e in SCHIFFE..EINHEITEN {
                    w.verteidigung += p.einheiten[e] * r.wert(&r.kosten_einheit(Einheit::ALLE[e], sp.volk)) / punkt;
                }
            }
            for st in 2..=5 {
                let t = stufenzeit[*sid as usize][st];
                w.stufe[st].push(if t > 0 { t } else { i64::MAX });
            }
            let v = voelker.entry(sp.volk).or_default();
            v.punkte.push(sp.punkte.gesamt());
            for st in 2..=5 {
                let t = stufenzeit[*sid as usize][st];
                v.stufe[st].push(if t > 0 { t } else { i64::MAX });
            }
        }
        for k in &welt.kampfberichte {
            let Some(vt) = k.verteidiger.first().and_then(|v| typ_von.get(v)) else {
                continue;
            };
            let beute = r.wert(&k.beute) / punkt;
            let w = werte.entry(*vt).or_default();
            w.angegriffen += 1;
            w.verloren += beute;
            if beute > 0 {
                w.pluenderungen += 1;
            }
            let e = gegen.entry(*vt).or_insert((0, 0, 0, 0));
            e.0 += 1;
            if k.sieger == Sieger::Angreifer {
                e.1 += 1;
            }
            e.2 += beute;
            let volk = welt.spieler[k.angreifer as usize].volk;
            for i in 0..SCHIFFE {
                e.3 += (k.ang_vorher[i] - k.ang_nachher[i]) * r.wert(&r.kosten_einheit(Einheit::ALLE[i], volk)) / punkt;
            }
        }
    }
    Ok((werte, gegen, voelker))
}

fn balance_bild(werte: &BTreeMap<Bottyp, Typwerte>) -> Vec<Typbild> {
    werte
        .iter()
        .map(|(typ, w)| {
            let n = w.punkte.len().max(1) as i64;
            Typbild { typ: *typ, punkte: w.punkte.iter().sum::<i64>() / n, rang: w.rang.iter().sum::<i64>() as f64 / n as f64 }
        })
        .collect()
}

fn mittel(w: &Typwerte) -> i64 {
    w.punkte.iter().sum::<i64>() / w.punkte.len().max(1) as i64
}

fn median_tag(w: &Typwerte, st: usize) -> Option<f64> {
    median(w.stufe[st].iter().copied().filter(|t| *t < i64::MAX).collect()).map(|t| t as f64 / TAG as f64)
}

fn anteil_erreicht(w: &Typwerte, st: usize) -> f64 {
    w.stufe[st].iter().filter(|t| **t < i64::MAX).count() as f64 / w.stufe[st].len().max(1) as f64
}

/// Die mit Codex vereinbarten Abnahmekriterien (COORDINATION.md), über drei Aufstellungen gemessen.
fn cmd_abnahme(args: &[String]) -> Result<(), String> {
    use Bottyp::*;
    let laeufe: u64 = zahl(args, "--laeufe", 12);
    let spieler: usize = zahl(args, "--spieler", 50);
    let tage: i64 = zahl(args, "--tage", 365);
    let regeln = regeln_laden(arg(args, "--regeln").as_deref(), Some(tage))?;
    let beginn = std::time::Instant::now();
    let (misch, _, voelker) = balance_messen(&regeln, laeufe, spieler, &Bottyp::ALLE)?;
    let (ro, ro_kampf, _) = balance_messen(&regeln, laeufe, spieler, &[Raeuber, Wehrlos])?;
    let (ri, ri_kampf, _) = balance_messen(&regeln, laeufe, spieler, &[Raeuber, Igel])?;
    println!(
        "Abnahme: je {laeufe} Epochen, {spieler} Spieler, {tage} Spieltage, Regelwerk {} ({}), Rechenzeit {:.0} s\n",
        regeln.version,
        &regeln.hash[..12],
        beginn.elapsed().as_secs_f64()
    );
    println!("Mischung     Punkte  Wirtsch.  Militär  verloren  Tag II  Tag III  Tag IV  Tag V (erreicht)  Wachstum letzte 60 Tage");
    for (t, w) in &misch {
        let n = w.punkte.len().max(1) as i64;
        let ende = w.punkte.iter().sum::<i64>();
        println!(
            "{:<10} {:>8} {:>9} {:>8} {:>9}  {}   {}   {}   {} ({:>3.0} %)  {:>+6.1} %",
            t.name(),
            ende / n,
            w.teil[0] / n,
            w.teil[2] / n,
            w.verloren / n,
            tag(median_tag(w, 2).map(|x| (x * TAG as f64) as i64)),
            tag(median_tag(w, 3).map(|x| (x * TAG as f64) as i64)),
            tag(median_tag(w, 4).map(|x| (x * TAG as f64) as i64)),
            tag(median_tag(w, 5).map(|x| (x * TAG as f64) as i64)),
            anteil_erreicht(w, 5) * 100.0,
            if w.vorher > 0 { (ende - w.vorher) as f64 * 100.0 / w.vorher as f64 } else { 0.0 }
        );
    }
    println!("
Volk         Punkte  Tag II  Tag III  Tag IV  Tag V (erreicht)");
    for (v, w) in &voelker {
        println!(
            "{:<10} {:>8}   {}   {}   {}   {} ({:>3.0} %)",
            v.name(),
            mittel(w),
            tag(median_tag(w, 2).map(|x| (x * TAG as f64) as i64)),
            tag(median_tag(w, 3).map(|x| (x * TAG as f64) as i64)),
            tag(median_tag(w, 4).map(|x| (x * TAG as f64) as i64)),
            tag(median_tag(w, 5).map(|x| (x * TAG as f64) as i64)),
            anteil_erreicht(w, 5) * 100.0,
        );
    }
    let mut ok_alle = true;
    let mut pruefe = |name: &str, ok: bool, wert: String| {
        ok_alle &= ok;
        println!("  [{}] {name}: {wert}", if ok { "ok" } else { "NEIN" });
    };
    println!("\nKriterien");
    let r = mittel(&misch[&Raeuber]) as f64;
    let bester = misch.iter().filter(|(t, _)| **t != Raeuber).map(|(_, w)| mittel(w)).max().unwrap_or(1) as f64;
    pruefe("Keine Dominanz: Räuber höchstens 25 % über dem besten anderen Typ", r <= 1.25 * bester, format!("Faktor {:.2}", r / bester));
    let (siege, kaempfe) = ro_kampf.get(&Wehrlos).map(|(n, s, _, _)| (*s, *n)).unwrap_or((0, 0));
    let quote = siege as f64 / kaempfe.max(1) as f64;
    pruefe("Raub lohnt sich: gegen wehrlose Ökonomen über 80 % Siege", quote > 0.8, format!("{:.0} % von {kaempfe} Kämpfen", quote * 100.0));
    pruefe(
        "Raub lohnt sich: Räuber liegt vor dem wehrlosen Ökonomen",
        mittel(&ro[&Raeuber]) > mittel(&ro[&Wehrlos]),
        format!("{} gegen {} Punkte", mittel(&ro[&Raeuber]), mittel(&ro[&Wehrlos])),
    );
    // Schutz wirkt (Codex' Formulierung): wer verteidigt, verliert mindestens 80 % weniger an Plünderung als
    // der wehrlose Ökonom. Bleiben Angriffe aus, zählt das als Abschreckung.
    let verlust = |w: &Typwerte| w.verloren as f64 / w.punkte.len().max(1) as f64;
    let (igel_verlust, wehrlos_verlust) = (verlust(&misch[&Igel]), verlust(&ro[&Wehrlos]));
    pruefe(
        "Schutz wirkt: Igel verliert mindestens 80 % weniger an Plünderung als der wehrlose Ökonom",
        igel_verlust <= 0.2 * wehrlos_verlust,
        format!("{igel_verlust:.0} gegen {wehrlos_verlust:.0} Punkte je Epoche"),
    );
    println!(
        "  [info] Ökonom und Händler, die sich nach Plünderungen schützen, verlieren {:.0} und {:.0} Punkte je Epoche",
        verlust(&misch[&Oekonom]),
        verlust(&misch[&Haendler])
    );
    let (ri_siege, ri_n) = ri_kampf.get(&Igel).map(|(n, s, _, _)| (*s, *n)).unwrap_or((0, 0));
    println!(
        "  [info] Duell Räuber gegen Igel: Igel {} gegen Räuber {} Punkte, Räuber greift {} mal an und gewinnt {}",
        mittel(&ri[&Igel]),
        mittel(&ri[&Raeuber]),
        ri_n,
        ri_siege
    );
    let fenster = [(2, 10.0, 20.0), (3, 30.0, 50.0), (4, 60.0, 110.0), (5, 150.0, 250.0)];
    for (st, von, bis) in fenster {
        let mindest = if st == 5 { 0.8 } else { 0.9 };
        // Stufe V braucht Xenokristall, und der ist plünderbar: sie gilt für die wehrhaften Strategien.
        // Ökonom und Händler ohne jeden Schutz sind die Opferkontrolle und stehen nur in der Tabelle.
        let schlecht: Vec<String> = misch
            .iter()
            .filter(|(t, _)| st < 5 || matches!(t, Raeuber | Igel))
            .filter(|(_, w)| anteil_erreicht(w, st) < mindest || median_tag(w, st).map(|d| d < von || d > bis).unwrap_or(true))
            .map(|(t, w)| format!("{} {} ({:.0} %)", t.name(), median_tag(w, st).map(|d| format!("Tag {d:.0}")).unwrap_or("nie".into()), anteil_erreicht(w, st) * 100.0))
            .collect();
        let name = ["", "", "II", "III", "IV", "V"][st];
        let wer = if st == 5 { " (Räuber und Igel)" } else { "" };
        pruefe(
            &format!("Stufe {name}{wer} von mindestens {:.0} % erreicht, Median in Tag {von:.0} bis {bis:.0}", mindest * 100.0),
            schlecht.is_empty(),
            if schlecht.is_empty() { "alle Typen".into() } else { schlecht.join(", ") },
        );
    }
    // Völker sollen sich unterscheiden, aber keines darf das Spiel entscheiden.
    let (vmax, vmin) = (voelker.values().map(mittel).max().unwrap_or(1), voelker.values().map(mittel).min().unwrap_or(1).max(1));
    pruefe("Völker ausgeglichen: bestes Volk höchstens 15 % über dem schwächsten", vmax as f64 <= 1.15 * vmin as f64, format!("Faktor {:.2}", vmax as f64 / vmin as f64));
    let stehen: Vec<&str> = misch.iter().filter(|(_, w)| w.punkte.iter().sum::<i64>() <= w.vorher).map(|(t, _)| t.name()).collect();
    pruefe("Keine Plateaus: alle Typen wachsen in den letzten 60 Tagen", stehen.is_empty(), if stehen.is_empty() { "ja".into() } else { stehen.join(", ") });
    if ok_alle {
        println!("\nAlle Kriterien erfüllt.");
        Ok(())
    } else {
        Err("nicht alle Abnahmekriterien erfüllt".into())
    }
}

fn cmd_balance(args: &[String]) -> Result<(), String> {
    if args.iter().any(|a| a == "--abnahme") {
        return cmd_abnahme(args);
    }
    let laeufe: u64 = zahl(args, "--laeufe", 8);
    let spieler: usize = zahl(args, "--spieler", 50);
    let tage: i64 = zahl(args, "--tage", 150);
    let typen = typen_aus(args)?;
    let regeln = regeln_laden(arg(args, "--regeln").as_deref(), Some(tage))?;
    let beginn = std::time::Instant::now();
    let (werte, gegen, _) = balance_messen(&regeln, laeufe, spieler, &typen)?;
    println!(
        "Balance: {laeufe} Epochen, {spieler} Spieler, {tage} Spieltage, Typen {}, Regelwerk {} ({}), Rechenzeit {:.1} s",
        typen.iter().map(|t| t.name()).collect::<Vec<_>>().join(","),
        regeln.version,
        &regeln.hash[..12],
        beginn.elapsed().as_secs_f64()
    );
    println!("\nBottyp     Punkte   Rang  Wirtsch. Forsch. Militär  Zivil.  Lager  Verteid.  angegriffen  geplündert  verloren   Tag II  Tag III  Tag IV  (IV erreicht)");
    for (typ, w) in &werte {
        let n = w.punkte.len().max(1) as i64;
        let zeit = |st: usize| median(w.stufe[st].iter().copied().filter(|t| *t < i64::MAX).collect());
        let iv = w.stufe[4].iter().filter(|t| **t < i64::MAX).count() as f64 * 100.0 / w.stufe[4].len().max(1) as f64;
        println!(
            "{:<9} {:>7} {:>6.1} {:>9} {:>7} {:>7} {:>7} {:>6} {:>9} {:>12.1} {:>11.1} {:>9}   {}    {}   {}   ({iv:>3.0} %)",
            typ.name(),
            w.punkte.iter().sum::<i64>() / n,
            w.rang.iter().sum::<i64>() as f64 / n as f64,
            w.teil[0] / n,
            w.teil[1] / n,
            w.teil[2] / n,
            w.teil[3] / n,
            w.lager / n,
            w.verteidigung / n,
            w.angegriffen as f64 / n as f64,
            w.pluenderungen as f64 / n as f64,
            w.verloren / n,
            tag(zeit(2)),
            tag(zeit(3)),
            tag(zeit(4)),
        );
    }
    println!("\nAngriffe nach Typ des Verteidigers (alle Epochen, Werte in Punkten)");
    println!("Verteidiger  Kämpfe  Siege des Angreifers  Beute je Kampf  Verlust des Angreifers je Kampf");
    for (t, (n, siege, beute, verlust)) in &gegen {
        println!("{:<12} {:>6} {:>20.0}% {:>15} {:>31}", t.name(), n, *siege as f64 * 100.0 / (*n).max(1) as f64, beute / (*n).max(1), verlust / (*n).max(1));
    }
    let bild = balance_bild(&werte);
    if let (Some(best), Some(schlecht)) = (bild.iter().max_by_key(|b| b.punkte), bild.iter().min_by_key(|b| b.punkte)) {
        println!(
            "\nSpreizung: bester Typ {} mit {} Punkten (Rang {:.1}), schwächster {} mit {} Punkten, Verhältnis {:.2}",
            best.typ.name(),
            best.punkte,
            best.rang,
            schlecht.typ.name(),
            schlecht.punkte,
            best.punkte as f64 / schlecht.punkte.max(1) as f64
        );
    }
    Ok(())
}

/// Fährt mehrere Epochen parallel, je Kern eine, und liefert die Läufe in der Reihenfolge der Startwerte.
/// Die Epochen sind voneinander unabhängig; die Auswertung bleibt dadurch deterministisch.
fn epochen_parallel(regeln: &Regelwerk, laeufe: u64, spieler: usize, typen: &[Bottyp]) -> Result<Vec<Lauf>, String> {
    let kerne = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(4) as u64;
    let mut fertig: Vec<Lauf> = Vec::new();
    let mut s = 1;
    while s <= laeufe {
        let bis = (s + kerne - 1).min(laeufe);
        let block: Vec<Result<Lauf, String>> = std::thread::scope(|sc| {
            let faeden: Vec<_> = (s..=bis)
                .map(|startwert| {
                    let r = regeln.clone();
                    sc.spawn(move || epoche_fahren(r, startwert, spieler, typen, None).map(|(l, _)| l))
                })
                .collect();
            faeden.into_iter().map(|f| f.join().unwrap_or_else(|_| Err("Epoche abgebrochen".into()))).collect()
        });
        for b in block {
            fertig.push(b?);
        }
        s = bis + 1;
    }
    Ok(fertig)
}

/// Woran scheitert Stufe V? Für jeden Bot ohne Stufe V am Ende der erste Grund: fehlende Stufe IV,
/// eine offene Bedingung oder ein Gut, das für die Aufstiegskosten fehlt.
fn cmd_stufe5(args: &[String]) -> Result<(), String> {
    let laeufe: u64 = zahl(args, "--laeufe", 12);
    let spieler: usize = zahl(args, "--spieler", 50);
    let tage: i64 = zahl(args, "--tage", 365);
    let typen = typen_aus(args)?;
    let regeln = regeln_laden(arg(args, "--regeln").as_deref(), Some(tage))?;
    let laeufe_v = epochen_parallel(&regeln, laeufe, spieler, &typen)?;
    let mut je_typ: BTreeMap<Bottyp, (usize, Vec<i64>, BTreeMap<String, usize>)> = BTreeMap::new();
    for mut lauf in laeufe_v {
        let typ_von: BTreeMap<SpielerId, Bottyp> = lauf.bots.iter().map(|(s, b)| (*s, b.typ)).collect();
        let stufenzeit = lauf.stufenzeit.clone();
        let welt = lauf.w();
        let r = welt.regeln.clone();
        for (sid, typ) in typ_von {
            let sp = &welt.spieler[sid as usize];
            let e = je_typ.entry(typ).or_default();
            e.0 += 1;
            if sp.stufe >= 5 {
                e.1.push(stufenzeit[sid as usize][5]);
                continue;
            }
            let grund = if sp.stufe < 4 {
                "Stufe IV nicht erreicht".to_string()
            } else {
                let offen: Vec<String> = welt
                    .stufen_bedingungen(sid)
                    .into_iter()
                    .filter(|(_, ok)| !ok)
                    .map(|(t, _)| t.split(" (").next().unwrap_or("").to_string())
                    .collect();
                if !offen.is_empty() {
                    let mut g = format!("offen: {}", offen.join(", "));
                    if offen.iter().any(|t| t.contains("Kolonien")) {
                        let b = welt.bestand_jetzt(sp.heimat as usize);
                        let teile = format!(
                            " (erlaubt {}, Antriebskern {}, Habitatmodul {})",
                            welt.kolonien_erlaubt(sid),
                            ganz(b[Gut::Antriebskern.idx()]),
                            ganz(b[Gut::Habitatmodul.idx()])
                        );
                        g.push_str(&teile);
                    }
                    g
                } else {
                    let kosten = kern::regeln::preis_array(&r.stufen[3].kosten);
                    let b = welt.bestand_jetzt(sp.heimat as usize);
                    let grenze = welt.lagergrenze(sp.heimat as usize);
                    let fehlt: Vec<String> = Gut::ALLE
                        .iter()
                        .filter(|g| b[g.idx()] < kosten[g.idx()])
                        .map(|g| if grenze[g.idx()] < kosten[g.idx()] { format!("{g} (Lager zu klein)") } else { g.name().to_string() })
                        .collect();
                    if fehlt.is_empty() {
                        format!("alles da, Haltezeit {} h", sp.stufen_zaehler / STUNDE)
                    } else {
                        format!("Kosten: {}", fehlt.join(", "))
                    }
                }
            };
            *e.2.entry(grund).or_default() += 1;
        }
    }
    println!("Stufe V: {laeufe} Epochen, {spieler} Spieler, {tage} Spieltage, Regelwerk {} ({})", regeln.version, &regeln.hash[..12]);
    for (typ, (n, zeiten, gruende)) in je_typ {
        println!(
            "\n{}: {} von {} erreichen Stufe V ({:.0} %), Median Tag {}",
            typ.name(),
            zeiten.len(),
            n,
            zeiten.len() as f64 * 100.0 / n.max(1) as f64,
            tag(median(zeiten.clone())).trim()
        );
        let mut v: Vec<(usize, String)> = gruende.into_iter().map(|(g, k)| (k, g)).collect();
        v.sort_by(|a, b| b.cmp(a));
        for (k, g) in v.into_iter().take(8) {
            println!("  {k:>4}  {g}");
        }
    }
    Ok(())
}

fn cmd_doku(args: &[String]) -> Result<(), String> {
    let regeln = regeln_laden(arg(args, "--regeln").as_deref(), None)?;
    let dir = arg(args, "--aus").unwrap_or_else(|| "docs".into());
    let dir = Path::new(&dir);
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    for (name, text) in [("REGELWERK.md", lauf::doku::regelwerk_md(&regeln)), ("REGELTEXT.md", lauf::doku::regeltext_md(&regeln))] {
        std::fs::write(dir.join(name), text).map_err(|e| e.to_string())?;
        println!("{}", dir.join(name).display());
    }
    Ok(())
}

fn cmd_replay(args: &[String]) -> Result<(), String> {
    let dir = args.get(2).ok_or("replay braucht das Verzeichnis eines Laufs")?;
    let dir = Path::new(dir);
    let schluss: Value = serde_json::from_str(&std::fs::read_to_string(dir.join("schluss.json")).map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
    // Ohne --regeln gilt das Regelwerk, das der Lauf in seinem Ordner abgelegt hat, sonst das eingebaute.
    let eigenes = dir.join("regelwerk.ron");
    let pfad = arg(args, "--regeln").or_else(|| eigenes.is_file().then(|| eigenes.to_string_lossy().into_owned()));
    let regeln = regeln_laden(pfad.as_deref(), schluss["tage"].as_i64())?;
    if schluss["regel_hash"].as_str() != Some(regeln.hash.as_str()) {
        return Err("das Regelwerk hat einen anderen Hash als im Lauf; mit --regeln das Regelwerk des Laufs angeben".into());
    }
    let f = std::fs::File::open(dir.join("protokoll.jsonl")).map_err(|e| e.to_string())?;
    let mut log = Vec::new();
    for zeile in std::io::BufReader::new(f).lines() {
        let zeile = zeile.map_err(|e| e.to_string())?;
        if !zeile.trim().is_empty() {
            log.push(serde_json::from_str::<Logeintrag>(&zeile).map_err(|e| e.to_string())?);
        }
    }
    let ki: Vec<SpielerId> = schluss["ki"].as_array().map(|a| a.iter().filter_map(|x| x.as_u64().map(|n| n as SpielerId)).collect()).unwrap_or_default();
    let spieler = schluss["spieler"].as_u64().unwrap_or(0) as usize;
    // Ein angehaltener Lauf (Budget, Abbruch) endet nicht am Epochenende, sondern bei seiner letzten Zeit.
    let bis = schluss["zeit"].as_i64().unwrap_or(SimZeit::MAX);
    let welt = nachspielen(regeln, schluss["startwert"].as_u64().unwrap_or(0), spieler, &ki, &log, bis)?;
    let hash = welt.hash();
    let gleich = schluss["hash"].as_str() == Some(hash.as_str());
    println!("{} Aktionen nachgespielt, Zustandshash {} – {}", log.len(), &hash[..16], if gleich { "stimmt mit dem Lauf überein" } else { "WEICHT AB" });
    if gleich {
        Ok(())
    } else {
        Err("Nachspielen weicht ab".into())
    }
}

/// Je Spieler: Steuerung, Punkte, Zeiten der Stufen und die Nebenwertungen, von denen die Agenten nichts erfahren.
fn statistik(l: &mut Lauf) -> Value {
    let stufenzeit = l.stufenzeit.clone();
    let erste_kolonie = l.erste_kolonie.clone();
    let typ: BTreeMap<SpielerId, &str> = l.bots.iter().map(|(s, b)| (*s, b.typ.name())).collect();
    let welt = l.w();
    json!(welt.spieler.iter().map(|s| json!({
        "id": s.id, "name": s.name, "volk": s.volk.name(), "steuerung": typ.get(&s.id).copied().unwrap_or("modell"),
        "stufe": s.stufe, "rang": s.rang, "punkte": s.punkte, "statistik": s.statistik,
        "stufenzeit": stufenzeit[s.id as usize][2..].to_vec(), "erste_kolonie": erste_kolonie[s.id as usize],
        "kolonien": s.planeten.len() - 1, "credits": s.credits / M,
    })).collect::<Vec<_>>())
}

/// Öffentliches Bild der Welt für den Betrachter: Galaxie, Planeten, Spieler.
fn weltbild(welt: &Welt) -> Value {
    json!({
        "zeit": welt.zeit,
        "systeme": welt.systeme.iter().map(|s| json!([s.sektor, s.nummer, s.nebel, s.guertel])).collect::<Vec<_>>(),
        "planeten": welt.planeten.iter().map(|p| json!({"koord": p.koord.to_string(), "spieler": p.besitzer, "heimat": p.heimat})).collect::<Vec<_>>(),
        "spieler": welt.spieler.iter().map(|s| json!({"id": s.id, "name": s.name, "volk": s.volk.name(), "ki": s.ki, "stufe": s.stufe,
            "rang": s.rang, "punkte": s.punkte.gesamt()})).collect::<Vec<_>>(),
    })
}

// -------------------------------------------------------------------- Brücke

fn bruecke_befehl(lauf: &mut Option<Lauf>, b: &Value) -> Result<Value, String> {
    let cmd = b["cmd"].as_str().unwrap_or("");
    let mut regelwerk = None;
    if cmd == "neu" {
        let tage = b["tage"].as_i64();
        let text = regeln_text(b["regeln"].as_str())?;
        let mut regeln = Regelwerk::laden(&text)?;
        if let Some(t) = tage {
            regeln.welt.epoche_tage = t;
        }
        regelwerk = Some(text);
        let spieler = b["spieler"].as_u64().unwrap_or(50) as usize;
        let ki = b["ki"].as_array().map(|a| a.iter().filter_map(|x| x.as_u64().map(|n| n as SpielerId)).collect());
        let typen: Vec<Bottyp> = match b["bottypen"].as_array() {
            Some(a) => a.iter().map(|t| Bottyp::aus_name(t.as_str().unwrap_or("")).ok_or_else(|| format!("Bottyp {t} gibt es nicht"))).collect::<Result<_, _>>()?,
            None => Bottyp::ALLE.to_vec(),
        };
        *lauf = Some(Lauf::neu(regeln, b["startwert"].as_u64().unwrap_or(1), spieler, ki, &typen)?);
    } else if cmd == "laden" {
        *lauf = Some(Lauf::laden(b["pfad"].as_str().ok_or("pfad fehlt")?)?);
    }
    let Some(l) = lauf.as_mut() else {
        return Err("zuerst neu oder laden aufrufen".into());
    };
    let sid = b["spieler"].as_u64().map(|n| n as SpielerId);
    let rolle = b["rolle"].as_str().and_then(Rolle::aus_name);
    let braucht_spieler = |welt: &Welt| -> Result<SpielerId, String> {
        let s = sid.ok_or("spieler fehlt")?;
        if (s as usize) < welt.spieler.len() {
            Ok(s)
        } else {
            Err(format!("Spieler {s} gibt es nicht"))
        }
    };
    match cmd {
        "neu" | "laden" | "stand" => {
            let ki = l.ki.clone();
            let bots: BTreeMap<String, &str> = l.bots.iter().map(|(s, b)| (s.to_string(), b.typ.name())).collect();
            let welt = l.w();
            let mut antwort = json!({"zeit": welt.zeit, "zeittext": zeittext(welt.zeit), "ende": welt.beendet(), "faellig": welt.faellig,
                "regel_version": welt.regeln.version, "regel_hash": welt.regeln.hash, "startwert": welt.startwert,
                "tage": welt.regeln.welt.epoche_tage, "ki": ki, "bots": bots,
                "spieler": welt.spieler.iter().map(|s| json!({"id": s.id, "name": s.name, "volk": s.volk.name(), "ki": s.ki})).collect::<Vec<_>>()});
            // Nur bei neu: der Text des Regelwerks, damit der Orchestrator ihn zum Lauf legt.
            if let Some(text) = regelwerk {
                antwort["regelwerk"] = json!(text);
            }
            Ok(antwort)
        }
        "weiter" => {
            let bis_faellig = b["bis_faellig"].as_bool().unwrap_or(true);
            let max = b["max_fenster"].as_u64().unwrap_or(96 * 400);
            let mut n = 0u64;
            loop {
                if !l.weiter() {
                    break;
                }
                n += 1;
                if !bis_faellig || !l.w().faellig.is_empty() || n >= max {
                    break;
                }
            }
            let welt = l.w();
            Ok(json!({"zeit": welt.zeit, "zeittext": zeittext(welt.zeit), "ende": welt.beendet(), "fenster": n, "faellig": welt.faellig}))
        }
        "sicht" => {
            let welt = l.w();
            let s = braucht_spieler(welt)?;
            Ok(json!({"sicht": welt.sicht(s, rolle.ok_or("rolle fehlt")?)}))
        }
        "regeltext" => {
            let welt = l.w();
            let rolle = rolle.unwrap_or(Rolle::Alle);
            Ok(json!({"text": regeltext::regeltext(&welt.regeln, rolle), "aktionstypen": kern::aktion::erlaubte_typen(rolle),
                "schema": kern::aktion::antwortschema(rolle),
                "limits": {"aktionen": welt.regeln.agenten.aktionen_je_aufruf, "abfragen": welt.regeln.agenten.abfragen_je_aufruf,
                    "notiz_zeichen": welt.regeln.agenten.notiz_zeichen, "doktrin_zeichen": welt.regeln.agenten.doktrin_zeichen}}))
        }
        "handeln" => {
            let welt = l.w();
            let s = braucht_spieler(welt)?;
            let rolle = rolle.ok_or("rolle fehlt")?;
            let max = welt.regeln.agenten.aktionen_je_aufruf;
            let aktionen = b["aktionen"].as_array().cloned().unwrap_or_default();
            let mut ergebnisse = Vec::new();
            for (i, a) in aktionen.iter().enumerate() {
                if i >= max {
                    ergebnisse.push(json!({"ok": false, "text": format!("höchstens {max} Aktionen je Aufruf")}));
                    continue;
                }
                let (ok, text) = welt.handeln(s, rolle, a);
                ergebnisse.push(json!({"ok": ok, "text": text}));
            }
            Ok(json!({"ergebnisse": ergebnisse}))
        }
        "aufruf_ende" => {
            let welt = l.w();
            let s = braucht_spieler(welt)?;
            welt.aufruf_ende(s, rolle.ok_or("rolle fehlt")?, b["notiz"].as_str(), b["wecker_sekunden"].as_i64(), &texte(&b["hinweise"]));
            Ok(json!({}))
        }
        "werkzeug" => {
            let welt = l.w();
            let s = braucht_spieler(welt)?;
            match welt.werkzeug(s, &b["abfrage"]) {
                Ok(v) => Ok(json!({"ergebnis": v})),
                Err(e) => Ok(json!({"ergebnis": {"fehler": e}})),
            }
        }
        "log" => Ok(json!({"eintraege": l.w().log_abholen()})),
        "hash" => {
            let welt = l.w();
            Ok(json!({"hash": welt.hash(), "log_hash": welt.log_hash, "log_anzahl": welt.log_anzahl}))
        }
        "rangliste" => Ok(json!({"rangliste": l.w().rangliste()})),
        "tabelle" => {
            // Fortlaufende Tabellen ab einem Index: tageswerte, kampfberichte, register, nachrichten, handel.
            let ab = b["ab"].as_u64().unwrap_or(0) as usize;
            let welt = l.w();
            let zeilen = match b["name"].as_str().unwrap_or("") {
                "tageswerte" => serde_json::to_value(&welt.tageswerte[ab.min(welt.tageswerte.len())..]),
                "kampfberichte" => serde_json::to_value(&welt.kampfberichte[ab.min(welt.kampfberichte.len())..]),
                "register" => serde_json::to_value(&welt.register[ab.min(welt.register.len())..]),
                "nachrichten" => serde_json::to_value(&welt.nachrichten[ab.min(welt.nachrichten.len())..]),
                "handel" => serde_json::to_value(&welt.handel[ab.min(welt.handel.len())..]),
                n => return Err(format!("Tabelle '{n}' gibt es nicht")),
            }
            .map_err(|e| e.to_string())?;
            Ok(json!({"zeilen": zeilen}))
        }
        "statistik" => Ok(json!({"spieler": statistik(l)})),
        "welt" => Ok(weltbild(l.w())),
        "speichern" => {
            l.speichern(b["pfad"].as_str().ok_or("pfad fehlt")?)?;
            Ok(json!({}))
        }
        "ende" => Ok(json!({"ende": true})),
        anderes => Err(format!("Befehl '{anderes}' gibt es nicht")),
    }
}

/// Zeilenweise JSON über Standardein- und -ausgabe. Die Engine bleibt ohne Netz.
fn cmd_bruecke() -> Result<(), String> {
    let stdin = std::io::stdin();
    let mut aus = std::io::stdout().lock();
    let mut lauf: Option<Lauf> = None;
    for zeile in stdin.lock().lines() {
        let zeile = zeile.map_err(|e| e.to_string())?;
        if zeile.trim().is_empty() {
            continue;
        }
        let antwort = match serde_json::from_str::<Value>(&zeile) {
            Err(e) => json!({"ok": false, "fehler": format!("kein JSON: {e}")}),
            Ok(b) => match bruecke_befehl(&mut lauf, &b) {
                Ok(mut v) => {
                    v["ok"] = json!(true);
                    v
                }
                Err(e) => json!({"ok": false, "fehler": e}),
            },
        };
        let ende = antwort["ende"] == json!(true) && antwort.get("zeit").is_none();
        writeln!(aus, "{antwort}").map_err(|e| e.to_string())?;
        aus.flush().map_err(|e| e.to_string())?;
        if ende {
            break;
        }
    }
    Ok(())
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let ergebnis = match args.get(1).map(|s| s.as_str()) {
        Some("epoche") => cmd_epoche(&args),
        Some("balance") => cmd_balance(&args),
        Some("stufe5") => cmd_stufe5(&args),
        Some("replay") => cmd_replay(&args),
        Some("bruecke") => cmd_bruecke(),
        Some("doku") => cmd_doku(&args),
        Some("regeltext") => regeln_laden(arg(&args, "--regeln").as_deref(), None).map(|r| {
            let rolle = arg(&args, "--rolle").and_then(|s| Rolle::aus_name(&s)).unwrap_or(Rolle::Alle);
            println!("{}", regeltext::regeltext(&r, rolle));
        }),
        _ => {
            println!(
                "sternenepoche <befehl>\n\n\
  epoche    [--startwert N] [--spieler N] [--tage N] [--typen a,b] [--pruefen] [--spur SPIELER] [--aus VERZEICHNIS]\n\
            fährt eine Epoche mit Skriptbots\n\
  balance   [--laeufe N] [--spieler N] [--tage N] [--typen a,b]\n\
            fährt mehrere Epochen und vergleicht die Bottypen\n\
  balance   --abnahme [--laeufe N] [--tage N]\n\
            prüft die mit Codex vereinbarten Balancekriterien (COORDINATION.md)\n\
  stufe5    [--laeufe N] [--spieler N] [--tage N] [--typen a,b]\n\
            zeigt je Bottyp, wie viele Stufe V erreichen, wann, und woran die übrigen scheitern\n\
  replay    VERZEICHNIS      spielt das Protokoll eines Laufs nach und prüft den Zustandshash\n\
  regeltext [--rolle R]      gibt den Regeltext für eine Rolle aus\n\
  doku      [--aus ORDNER]   schreibt die Referenzen REGELWERK.md und REGELTEXT.md (Standard: docs)\n\
  bruecke                   zeilenweises JSON für den Orchestrator\n\n\
Bottypen: oekonom, raeuber, igel, haendler. Überall: --regeln PFAD für ein anderes Regelwerk."
            );
            Ok(())
        }
    };
    if let Err(e) = ergebnis {
        eprintln!("Fehler: {e}");
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// docs/REGELWERK.md und docs/REGELTEXT.md sind aus dem eingebauten Regelwerk erzeugt. Ändert sich eine
    /// Zahl oder ein Satz des Regeltexts, schlägt dieser Test an, bis `sternenepoche doku` gelaufen ist.
    #[test]
    fn doku_passt_zum_regelwerk() {
        let regeln = regeln_laden(None, None).unwrap();
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs");
        for (name, erwartet) in [("REGELWERK.md", lauf::doku::regelwerk_md(&regeln)), ("REGELTEXT.md", lauf::doku::regeltext_md(&regeln))] {
            let datei = std::fs::read_to_string(dir.join(name)).unwrap_or_default().replace("\r\n", "\n");
            assert!(datei == erwartet, "docs/{name} passt nicht zum Regelwerk: `sternenepoche doku` ausführen");
        }
    }

    /// Die Prüfung aus dem Konzept: eine Epoche mit Skriptbots zweimal fahren, die Zustandshashes
    /// müssen gleich sein. Dazu das Nachspielen aus Startwert und Aktionsprotokoll.
    #[test]
    fn epoche_mit_bots_ist_deterministisch() {
        let regeln = regeln_laden(None, Some(45)).unwrap();
        let (mut a, log) = epoche_fahren(regeln.clone(), 5, 24, &Bottyp::ALLE, None).unwrap();
        let (mut b, _) = epoche_fahren(regeln.clone(), 5, 24, &Bottyp::ALLE, None).unwrap();
        let hash = a.w().hash();
        assert_eq!(hash, b.w().hash());
        assert_eq!(hash, nachspielen(regeln.clone(), 5, 24, &[], &log, SimZeit::MAX).unwrap().hash());
        // Ein anderer Startwert ergibt eine andere Welt.
        let (mut c, _) = epoche_fahren(regeln, 6, 24, &Bottyp::ALLE, None).unwrap();
        assert_ne!(hash, c.w().hash());
        // Invarianten über die ganze Epoche: keine negativen Bestände, Credits oder Einwohner.
        let welt = a.w();
        assert!(welt.planeten.iter().all(|p| p.bestand.iter().all(|x| *x >= 0) && p.bevoelkerung > 0));
        assert!(welt.spieler.iter().all(|s| s.credits >= 0));
        assert!(welt.spieler.iter().any(|s| s.stufe >= 3), "in 45 Tagen erreicht jemand Stufe III");
    }

    #[test]
    fn stand_speichern_und_laden() {
        let regeln = regeln_laden(None, Some(6)).unwrap();
        let mut lauf = Lauf::neu(regeln, 9, 6, Some(vec![0]), &Bottyp::ALLE).unwrap();
        for _ in 0..200 {
            lauf.weiter();
        }
        let pfad = std::env::temp_dir().join(format!("sternenepoche-test-{}.bin", std::process::id()));
        lauf.speichern(pfad.to_str().unwrap()).unwrap();
        let mut geladen = Lauf::laden(pfad.to_str().unwrap()).unwrap();
        std::fs::remove_file(&pfad).ok();
        assert_eq!(geladen.w().hash(), lauf.w().hash());
        // Beide laufen gleich weiter: auch die Bots sind Teil des gespeicherten Stands.
        for _ in 0..200 {
            lauf.weiter();
            geladen.weiter();
        }
        assert_eq!(geladen.w().hash(), lauf.w().hash());
    }
}
