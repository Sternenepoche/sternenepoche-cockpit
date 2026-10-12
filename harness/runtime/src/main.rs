use serde_json::json;
use sternenepoche_harness::gedaechtnis::{self, Speicher};
use sternenepoche_harness::{kaefig, konfig::{Geheimnisse, Konfig}, schleife::{Bus, Harness}, selbsttest, wissen};
use std::io::BufRead;
use std::path::PathBuf;
use std::sync::Arc;

const HILFE: &str = "sternenepoche-harness <befehl> --konfig harness.json [optionen]
  version                      Bausteine dieses Harness als JSON
  selbsttest [--mit-modell]    Lokale Prüfungen (Gedächtnis, Modelle, Server, Wachposten, Quarantäne, Plugins)
  spielen [--ohne-kaefig]      Spielt über die Harness-API, bis stop kommt
  probe --rolle ROLLE          Ein Zug als reine Probe im Serverkern, ohne Wirkung
  mcp                          MCP über stdin/stdout, verbunden mit dem laufenden Harness
  wissen --quelle NAME --frage TEXT    Eine Wissensquelle abfragen (Test ohne laufenden Harness)
  wissen-werkzeuge --quelle NAME       Werkzeuge einer MCP-Wissensquelle auflisten
Geheimnisse: --geheimnisse-stdin (erste Zeile JSON {\"schluessel\":\"sep_…\",\"openrouter\":\"…\",\"openai\":\"…\",\"wissen\":{\"quelle\":\"Bearer …\"}})
oder Umgebung STERNENEPOCHE_HARNESS_SCHLUESSEL und OPENROUTER_API_KEY. Danach liest
spielen weitere stdin-Zeilen als Steuerbefehle; Ende von stdin beendet den Harness.";

fn main() {
    if let Err(e) = lauf() {
        eprintln!("Fehler: {e}");
        println!("{}", json!({"art":"abbruch","text":e}));
        std::process::exit(1);
    }
}

fn lauf() -> Result<(), String> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let befehl = args.first().map(String::as_str).unwrap_or("hilfe");
    let wert = |name: &str| args.iter().position(|a| a == name).and_then(|i| args.get(i + 1)).cloned();
    let schalter = |name: &str| args.iter().any(|a| a == name);
    if befehl == "version" {
        println!("{}", json!({"name":"sternenepoche-harness","version":env!("CARGO_PKG_VERSION"),"bausteine":selbsttest::bausteine()}));
        return Ok(());
    }
    if !["selbsttest", "spielen", "probe", "mcp", "wissen", "wissen-werkzeuge"].contains(&befehl) { println!("{HILFE}"); return Ok(()); }
    let pfad = PathBuf::from(wert("--konfig").ok_or("--konfig fehlt")?);
    let basis = pfad.parent().map(|p| p.to_path_buf()).filter(|p| !p.as_os_str().is_empty()).unwrap_or_else(|| PathBuf::from("."));
    let konfig = Konfig::laden(&pfad)?;
    #[cfg(feature = "lokale-api")]
    if befehl == "mcp" { return sternenepoche_harness::lokale_api::stdio_bruecke(&basis); }
    let geheim: Geheimnisse = if schalter("--geheimnisse-stdin") {
        // Nur diese eine Zeile lesen; die Sperre auf stdin wird sofort wieder frei.
        let mut zeile = String::new();
        std::io::stdin().read_line(&mut zeile).map_err(|e| e.to_string())?;
        serde_json::from_str(zeile.trim()).map_err(|_| "Geheimnis-Zeile ist kein JSON")?
    } else {
        Geheimnisse { schluessel: std::env::var("STERNENEPOCHE_HARNESS_SCHLUESSEL").unwrap_or_default(), openrouter: std::env::var("OPENROUTER_API_KEY").unwrap_or_default(), openai: std::env::var("OPENAI_API_KEY").unwrap_or_default(), wissen: Default::default() }
    };
    if befehl == "wissen" || befehl == "wissen-werkzeuge" {
        let name = wert("--quelle").ok_or("--quelle fehlt")?;
        let w = konfig.wissen.iter().find(|w| w.name == name).cloned()
            .or_else(|| (name == "gedaechtnis").then(|| serde_json::from_value(json!({"name":"gedaechtnis","art":"gedaechtnis"})).unwrap()))
            .ok_or_else(|| format!("Wissensquelle {name} unbekannt"))?;
        let schluessel = geheim.wissen.get(&name).map(String::as_str);
        let v = if befehl == "wissen-werkzeuge" {
            match wissen::mcp_werkzeuge(&w, schluessel) { Ok(v) => json!({"quelle":name,"ok":true,"werkzeuge":v}), Err(e) => json!({"quelle":name,"ok":false,"fehler":e}) }
        } else {
            let speicher = if w.art == "gedaechtnis" {
                let bibliothek = konfig.pfad(&basis, if konfig.gedaechtnis.duckdb_bibliothek.is_empty() { "duckdb.dll" } else { &konfig.gedaechtnis.duckdb_bibliothek });
                Some(std::sync::Mutex::new(Speicher { db: gedaechtnis::oeffnen(&konfig.gedaechtnis.art, &konfig.pfad(&basis, &konfig.gedaechtnis.pfad), &bibliothek)? }))
            } else { None };
            wissen::abfragen(&w, &wert("--frage").unwrap_or_default(), "besitzer", schluessel, speicher.as_ref())
        };
        println!("{v}");
        return Ok(());
    }
    if befehl == "selbsttest" {
        let ergebnisse = selbsttest::ausfuehren(&konfig, &basis, &geheim, schalter("--mit-modell"));
        let ok = ergebnisse.iter().all(|e| e["ok"] == true);
        for e in &ergebnisse { println!("{e}"); }
        println!("{}", json!({"art":"selbsttest","ok":ok,"anzahl":ergebnisse.len()}));
        if !ok { std::process::exit(2); }
        return Ok(());
    }
    if geheim.schluessel.is_empty() { return Err("Harness-Schlüssel fehlt".into()); }
    if !schalter("--ohne-kaefig") {
        match kaefig::einsperren(&basis, 2048) {
            Ok(t) => eprintln!("{t}"),
            Err(e) => eprintln!("Warnung: {e}"),
        }
    }
    let bus = Arc::new(Bus::neu(true));
    let h = Arc::new(Harness::neu(konfig.clone(), basis.clone(), &geheim, bus.clone())?);
    if befehl == "probe" {
        h.nur_probe.store(true, std::sync::atomic::Ordering::SeqCst);
        let rolle = wert("--rolle").ok_or("--rolle fehlt")?;
        return h.clone().einzelzug(&rolle);
    }
    #[cfg(feature = "lokale-api")]
    if konfig.lokale_api.aktiv {
        let url = sternenepoche_harness::lokale_api::starten(h.clone(), konfig.lokale_api.port, &basis)?;
        bus.senden("lokale_api", json!({"url":url,"datei":basis.join("lokale-api.json").display().to_string()}));
    }
    if schalter("--geheimnisse-stdin") {
        // Steuerbefehle des Cockpits; endet das Cockpit, endet auch der Harness.
        let h2 = h.clone();
        std::thread::spawn(move || {
            let stdin = std::io::stdin();
            for zeile in stdin.lock().lines().map_while(Result::ok) {
                if zeile.trim().is_empty() { continue; }
                match serde_json::from_str::<serde_json::Value>(&zeile) {
                    // Abfragen über Netz oder Datenbank blockieren die Steuerung nicht (Stopp bleibt sofort wirksam).
                    Ok(b) if Harness::langsamer_befehl(&b) => {
                        let h3 = h2.clone();
                        std::thread::spawn(move || { if let Err(e) = h3.befehl(&b) { h3.bus.senden("fehler", json!({"text":e,"befehl":b["befehl"],"id":b["id"],"quelle":b["quelle"]})); } });
                    }
                    Ok(b) => { if let Err(e) = h2.befehl(&b) { h2.bus.senden("fehler", json!({"text":e})); } }
                    Err(_) => h2.bus.senden("fehler", json!({"text":"Steuerbefehl ist kein JSON"})),
                }
            }
            let _ = h2.befehl(&json!({"befehl":"stop"}));
        });
    }
    h.spielen()
}
