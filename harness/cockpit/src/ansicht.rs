//! Zwischen Rust-Kern und Oberfläche: baut die Daten, die die Seite zeigt, und nimmt ihre
//! Befehle an. Jeder Befehl wird hier geprüft; die Seite kann nichts tun, was hier nicht
//! ausdrücklich vorgesehen ist (keine Dateipfade, keine Programme, keine Adressen).
use crate::app::App;
use crate::verbindung::Verbindung;
use crate::bau::{self, Stand as BauStand};
use crate::katalog::{self, Anbieter, Datenbank, Erweiterung, Schnittstelle, AKTIONEN, AUSBAU_GEBAEUDE, AUSLOESER, AUTONOMIE, GRUPPEN, MISSIONEN, ROLLEN, ROUTINEN, SKILLS, WERKZEUGE};
use crate::vorlagen::{self, MODELLKLASSEN, SPIELSTILE, VORLAGEN};
use serde_json::{json, Value};

/// Häufig wechselnder Zustand (Rat, Lage, Alarme, Bau, Anmeldung).
pub fn zustand(app: &mut App) -> Value {
    let laeuft = app.laeuft();
    let c = &app.cockpit;
    let mut lokal = json!({"busy":app.lokal_busy,"connected":app.online_server.is_some(),"own_host":app.lokal.is_some(),"master_da":crate::tresor::lesen("lokal-master").is_some(),"hosts":app.lokal_hosts,"info":app.lokal_info,"url":if app.online_server.is_some(){Some(&app.auswahl.server)}else{None},"warning":app.lokal.as_ref().and_then(|l|l.discovery_warning.as_ref()),"lan":app.lokal_lan});
    lokal["speichert"]=json!(app.lokal_speichert);
    lokal["steuert"]=json!(app.lokal_steuert);
    lokal["gespeichert"]=json!(app.lokal_gespeichert);
    lokal["auto_resume"]=json!(app.lokal_auto_resume);
    lokal["kapazitaet"]=crate::lokale_agenten::schaetzung(&app.auswahl.lokale_agenten,&app.ollama,&app.gpu,&app.hardware);
    let potential=sternenepoche_server::local::seat_capacity(app.auswahl.lokale_bots).unwrap_or(0);
    lokal["potentielle_plaetze"]=json!(potential);
    let estimate_info=if app.online_server.is_some(){app.lokal_info.clone()}else{json!({"plaetze":potential})};
    lokal["kapazitaet"]["empfohlene_spielerplaetze"]=json!(crate::lokale_agenten::spielerraum(&estimate_info,&app.hardware,&lokal["kapazitaet"]));
    lokal["agenten"]=app.agenten.as_ref().map(|f|f.status.lock().unwrap().clone()).unwrap_or(json!({"laeuft":false}));
    if let Some(n)=crate::lokale_agenten::gemessene_plaetze(&lokal["agenten"],lokal["kapazitaet"]["empfohlene_spielerplaetze"].as_u64().unwrap_or(0)) {
        lokal["kapazitaet"]["empfohlene_plaetze"]=json!(n);
        lokal["kapazitaet"]["empfehlung_gemessen"]=json!(true);
    }
    json!({
        "laeuft": laeuft, "status": c.status, "pausiert": c.status == "pausiert", "quote": c.quote,
        "rollen": c.rollen, "wartend": c.wartend, "alarme": c.alarme.iter().take(30).collect::<Vec<_>>(),
        "lage": c.lage, "konto": {"name":c.konto["name"],"spieler":c.konto["spieler"],"schluessel":c.konto["schluessel"]},
        "codex_allianzen": if app.sitzung.as_ref().is_some_and(|s|app.codex_allianzen["url"]==app.auswahl.server && app.codex_allianzen["name"]==s.name && app.codex_allianzen["world_id"]==s.world_id) {app.codex_allianzen.clone()} else {Value::Null},
        "zaehler": {"zuege":c.zuege,"angenommen":c.angenommen,"abgelehnt":c.abgelehnt},
        "lokale_api": c.lokale_api, "routinen": c.routinen, "wissen_laeuft": c.wissen_laeuft, "arena_laeuft": c.arena_laeuft,
        "geaendert": app.geaendert(), "aenderungen": app.aenderungen(), "gebaut": app.auswahl.gebaut_aktuell(), "bausteine": app.auswahl.features(),
        "vorgebaut": crate::bau::vorgebaut().map(|v| json!({"features": v.features, "mb": (v.groesse as f64 / 1e5).round() / 10.0, "passt": crate::bau::vorgebaut_fuer(&app.auswahl).is_some()})),
        "meldung": app.meldung.as_ref().map(|(t, f)| json!({"text":t,"fehler":f})),
        "server": {"info":app.server_info.as_ref().map(|i| json!({"version":i["version"],"regeln":i["regeln"],"welt":i["welt"],"quoten":i["quoten_je_minute"]})),"fehler":app.server_fehler},
        "schluessel_da": app.harness_schluessel().is_some(), "openrouter_da": !app.openrouter.is_empty(), "eigen_da": crate::tresor::lesen("eigen").is_some(),
        "ollama": app.ollama.iter().map(|(n, g)| json!({"name":n,"groesse":g,"klasse":vorlagen::klasse_aus_name(n)})).collect::<Vec<_>>(), "ollama_fehler": app.ollama_fehler, "gpu": app.gpu,
        "abh": app.abh.iter().map(|a| json!({"name":a.name,"stand":App::abh_stand(a.stand),"detail":a.detail,"groesse":a.groesse,"blockiert":a.blockiert,"hilfe":a.hilfe})).collect::<Vec<_>>(),
        "abh_laedt": app.abh_laedt, "abh_blockiert": app.abh_blockiert(), "modellprobe": app.modellprobe,
        "bau": {"laeuft":app.bau_laeuft(),"schritte":bau::SCHRITTE.iter().enumerate().map(|(i, n)| { let (s, t, f) = app.bau_schritte.get(i).cloned().unwrap_or((BauStand::Wartet, String::new(), 0.0));
                json!({"name":n,"stand":format!("{s:?}").to_lowercase(),"text":t,"fortschritt":f}) }).collect::<Vec<_>>(),
            "pruefungen":app.pruefungen,"ende":app.bau_ende.as_ref().map(|r| match r { Ok(_) => json!({"ok":true}), Err(e) => json!({"ok":false,"text":e}) }),
            "log":app.bau_log.iter().rev().take(80).rev().collect::<Vec<_>>()},
        "anmeldung": {"offen":app.anmeldung.offen,"laeuft":app.anmeldung.laeuft,"fehler":app.anmeldung.fehler,"platz_noetig":app.anmeldung.platz_noetig,"volk":app.anmeldung.volk,"warteliste":app.anmeldung.warteliste,
            "angemeldet":app.sitzung.as_ref().map(|s| s.name.clone())},
        "fairplay": app.fairplay_text, "export": c.export,
        "testpiloten": crate::testpiloten::NAMEN.iter().map(|n| json!({"name":n,"verfuegbar":crate::testpiloten::lesen(&app.auswahl.ordner(),n).is_ok()})).collect::<Vec<_>>(),
        "ordner": app.auswahl.ordner().display().to_string(), "profile": app.profile,
        "stimmen": crate::stimmen::stand(),
        "lokal": lokal,
    })
}

/// Die Einstellungen des Harness samt abgeleiteten Werten (Autonomie, Rechte).
pub fn auswahl(app: &App) -> Value {
    let a = &app.auswahl;
    let mut v = serde_json::to_value(a).unwrap_or(Value::Null);
    let autonomie: serde_json::Map<String, Value> = ROLLEN.iter().map(|(r, _, _)| (r.to_string(), json!(GRUPPEN.iter().map(|(g, _)| (g.to_string(), json!(a.autonomie(r, g)))).collect::<serde_json::Map<_, _>>()))).collect();
    let mut rechte = serde_json::Map::new();
    for (r, _, _) in ROLLEN {
        let mut m = serde_json::Map::new();
        // Je Bereich die Kennungen, die diese Rolle laut Spielkern nutzt (Missionen nur, wenn die Rolle sie fliegen darf).
        for (g, _) in GRUPPEN {
            for (k, rollen) in katalog::kennungen(g) { if rollen.contains(&r) { m.insert(k.clone(), json!(a.recht(r, &k))); } }
        }
        rechte.insert(r.into(), Value::Object(m));
    }
    let klassen: serde_json::Map<String, Value> = a.rollen.iter().map(|(r, w)| (r.clone(), json!(vorlagen::klasse_aus_name(&w.modell)))).collect();
    v["abgeleitet"] = json!({"autonomie":autonomie,"rechte":rechte,"features":a.features(),"eigene_skills":eigene_skills(app),"klassen":klassen});
    v
}

fn eigene_skills(app: &App) -> Vec<Value> {
    let mut v: Vec<Value> = std::fs::read_dir(app.auswahl.ordner().join("skills")).into_iter().flatten().filter_map(Result::ok)
        .filter_map(|e| { let n = e.file_name().to_str()?.strip_suffix(".md")?.to_string(); let text = std::fs::read_to_string(e.path()).ok()?; Some((n, text)) })
        .filter(|(n, _)| !n.is_empty() && n.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_' || b == b'-') && !SKILLS.iter().any(|(k, _)| k == n))
        .map(|(n, t)| json!({"name":n,"text":t.chars().take(3000).collect::<String>()})).collect();
    v.sort_by(|a, b| a["name"].as_str().cmp(&b["name"].as_str()));
    v
}

/// Planung, Gedächtnis, Inventur, Analyse, Wissen, Probezüge, Kontexte, Züge.
pub fn daten(app: &App) -> Value {
    let c = &app.cockpit;
    json!({
        "aufgaben": c.aufgaben, "team": c.team, "erinnerungen": c.erinnerungen, "notizen": c.notizen, "gedaechtnis": c.gedaechtnis,
        "inventur": c.inventur, "analyse": c.analyse, "wissen": c.wissen, "mcp_werkzeuge": c.mcp_werkzeuge,
        "kontext": c.kontext, "probezuege": c.probezuege, "arena": c.arena, "kennzahlen": c.kennzahlen,
        "zuege": c.zugliste.iter().take(60).collect::<Vec<_>>(),
        "verlauf": c.verlauf.iter().enumerate().map(|(i, e)| json!({"i":i,"rolle":e["rolle"],"modell":e["modell"],"anbieter":e["anbieter"],"tokens":e["tokens"],"latenz_ms":e["latenz_ms"],"kosten_usd":e["kosten_usd"],"zeit":e["zeit"],
            "zeichen_system":e["system"].as_str().map(|s| s.chars().count()),"zeichen_nutzer":e["nutzer"].as_str().map(|s| s.chars().count()),"antwort":e["antwort"].as_str().map(|s| s.chars().take(600).collect::<String>())})).collect::<Vec<_>>(),
    })
}

/// Unveränderliches: Spielregeln vom Server und alles, was das Cockpit an Bausteinen kennt.
pub fn katalog(app: &App) -> Value {
    let skills: Vec<Value> = SKILLS.iter().map(|(k, n)| {
        let text = crate::quelle::DATEIEN.iter().find(|(p, _)| *p == format!("skills/{k}.md")).map(|(_, b)| String::from_utf8_lossy(b).into_owned()).unwrap_or_default();
        json!({"name":k,"titel":n,"text":text})
    }).collect();
    json!({
        "regeln": app.katalog_regeln, "regeln_fehler": app.katalog_fehler,
        "rollen": ROLLEN.iter().map(|(r, t, a)| json!({"name":r,"titel":t,"aufgabe":a})).collect::<Vec<_>>(),
        "skills": skills, "werkzeuge": WERKZEUGE.iter().map(|(k, t)| json!({"name":k,"text":t})).collect::<Vec<_>>(),
        "ausloeser": AUSLOESER.iter().map(|(k, t)| json!({"name":k,"text":t})).collect::<Vec<_>>(),
        "aktionen": AKTIONEN.iter().map(|(k, g, r)| json!({"name":k,"gruppe":g,"rollen":r})).collect::<Vec<_>>(), "missionen": MISSIONEN,
        "gruppen": GRUPPEN.iter().map(|(g, t)| json!({"name":g,"text":t})).collect::<Vec<_>>(), "autonomie": AUTONOMIE.iter().map(|(k, t)| json!({"name":k,"text":t})).collect::<Vec<_>>(),
        "routinen": ROUTINEN.iter().map(|(k, n, t)| json!({"name":k,"titel":n,"text":t})).collect::<Vec<_>>(), "ausbau_gebaeude": AUSBAU_GEBAEUDE,
        "vorlagen": VORLAGEN.iter().map(|v| json!({"name":v.name,"rolle":v.rolle,"kurz":v.kurz,"seele":v.seele,"skills":v.skills,"werkzeuge":v.werkzeuge,"ausloeser":v.ausloeser,"temperatur":v.temperatur,"takt":v.takt_minuten})).collect::<Vec<_>>(),
        "spielstile": SPIELSTILE.iter().map(|s| json!({"name":s.name,"titel":s.titel,"kurz":s.kurz,"bild":s.bild,"besetzung":s.besetzung,"doktrin":s.doktrin,"autonomie":s.autonomie})).collect::<Vec<_>>(),
        "modellklassen": MODELLKLASSEN.iter().map(|(k, t)| json!({"name":k,"text":t})).collect::<Vec<_>>(),
        "figuren": crate::oberflaeche::vorhanden("figuren/"),
        "harness_exe": bau::harness_exe(&app.auswahl.ordner()).display().to_string(),
        "mcp_eintrag": json!({"mcpServers":{"sternenepoche":{"command":bau::harness_exe(&app.auswahl.ordner()).display().to_string(),"args":["mcp","--konfig",app.auswahl.ordner().join("harness.json").display().to_string()]}}}),
        "version": env!("CARGO_PKG_VERSION"),
    })
}

fn text(v: &Value, k: &str) -> String { v[k].as_str().unwrap_or("").to_string() }

/// Ein Befehl der Oberfläche. Alles Unbekannte wird abgelehnt.
pub fn befehl(app: &mut App, v: &Value) -> Result<(), String> {
    match v["t"].as_str().unwrap_or("") {
        "codex_allianzen" => {
            let s=app.sitzung.clone().ok_or("Für das Allianzverzeichnis zuerst anmelden")?;
            let url=app.auswahl.server.clone();
            app.hintergrund(move || {let r=Verbindung::neu(&url).and_then(|c|c.allianz_verzeichnis(&s));crate::app::Ergebnis::AllianzCodex(url,s.name,s.world_id,r)});
        },
        "lokal_start" => app.lokal_starten(text(v,"name"),text(v,"volk"),text(v,"password"))?,
        "lokal_agenten_start" => {
            if app.lokal.is_none(){return Err("Agentenplätze werden auf dem eigenen Host oder im Singleplayer gestartet".into());}
            if app.agenten.as_ref().is_some_and(|f|f.status.lock().unwrap()["laeuft"]==true){return Err("Agenten zuerst anhalten; ein laufender Modellzug wird beendet".into());}
            app.agenten.take();
            let s=app.sitzung.as_ref().ok_or("Lokale Anmeldung fehlt")?;
            app.agenten=Some(crate::lokale_agenten::Flotte::starten(app.auswahl.server.clone(),s.token.clone(),app.auswahl.lokale_agenten.clone(),app.ollama.clone(),app.gpu.clone(),app.hardware.clone(),app.auswahl.ollama_url.clone(),app.auswahl.eigen_url.clone(),crate::tresor::lesen("eigen").unwrap_or_default())?);
            app.einstellungen_speichern()?;
        },
        "lokal_agenten_stop" => {if let Some(f)=&app.agenten{f.stoppen();}},
        "lokal_hardware" => {app.ollama_laden();app.hardware_laden();},
        "lokal_verlassen" => {if app.lokal_steuert || app.lokal_speichert || app.lokal_busy {return Err("Änderung oder Speicherung zuerst abwarten".into());}app.lokal_verlassen()},
        "lokal_speichern" => app.lokal_speichern()?,
        "lokal_suchen" => { if app.lokal_busy {return Err("Lokale Aktion läuft noch".into());} app.lokal_busy=true; app.hintergrund(||crate::app::Ergebnis::LokalSuche(sternenepoche_server::local::discover())); },
        "lokal_beitreten" => {
            if app.lokal_busy || app.online_server.is_some() { return Err("Lokale Sitzung zuerst verlassen".into()); }
            if app.laeuft() { return Err("Den Rat vor dem Wechsel anhalten".into()); }
            let url=text(v,"url").trim().trim_end_matches('/').to_string();
            let parsed=reqwest::Url::parse(&url).map_err(|_|"Lokale Adresse ungültig")?;
            if parsed.scheme()!="http" || !parsed.host_str().is_some_and(|h|h.parse::<std::net::IpAddr>().is_ok_and(sternenepoche_server::local::private_ip)) {return Err("Numerische LAN-IP verwenden, z. B. http://192.168.1.20:8892".into());}
            let (name,pass,volk)=(text(v,"name"),text(v,"password"),text(v,"volk")); app.lokal_busy=true;
            app.hintergrund(move || {let r=Verbindung::neu(&url).and_then(|c|c.lokal_anmelden(&name,&pass,&volk));crate::app::Ergebnis::LokalBeitritt(url,r)});
        }
        "testpilot_anmelden" => {
            if app.online_server.is_some() || app.laeuft() || app.anmeldung.laeuft {return Err("Aktuelle Partie zuerst verlassen oder Rat anhalten".into());}
            let name=text(v,"name");let passwort=crate::testpiloten::lesen(&app.auswahl.ordner(),&name)?;let url=app.auswahl.server.clone();app.anmeldung.laeuft=true;
            app.hintergrund(move || crate::app::Ergebnis::Testpilot(Verbindung::neu(&url).and_then(|c|{let s=c.anmelden(&name,&passwort,"")?;let ich=c.ich(&s)?;let adresse=c.browser_adresse(Some(&s));Ok((s,ich,adresse))})));
        }
        "lokal_steuern" => {
            if app.lokal_steuert || app.lokal_busy || app.lokal_speichert {return Err("Änderung der Partie läuft bereits".into());}
            if app.lokal.is_none() {return Err("Nur der Host steuert diese Partie".into());}
            app.lokal_info_generation=app.lokal_info_generation.wrapping_add(1);
            let s=app.sitzung.clone().ok_or("Keine lokale Sitzung")?; let url=app.auswahl.server.clone(); let body=v["body"].clone(); let host=app.lokal.is_some();
            app.lokal_steuert=true;
            app.hintergrund(move || {let r=Verbindung::neu(&url).and_then(|c| {c.lokal_steuern(&s,body)?; let mut info=c.lokal_info()?; if host {info["teilnehmer"]=c.lokal_spieler(&s).map(|t|t["spieler"].clone()).unwrap_or(Value::Null);} Ok(info)});crate::app::Ergebnis::LokalGesteuert(url,r)});
        }
        "lokal_neu" => { let text=app.lokal_neue_partie()?; app.meldung=Some((text,false)); }
        // Nur Entwicklungsbuilds: prüft den Absturzschutz (Fenster bleibt, Protokoll entsteht).
        #[cfg(debug_assertions)]
        "__panik_test" => panic!("Absichtlicher Testfehler aus der Entwicklungsansicht"),
        // ---- Einstellungen
        "setze" => {
            let pfad = text(v, "pfad");
            if !pfad.starts_with('/') || ["/server", "/arbeitsordner", "/gebaut"].iter().any(|p| pfad.starts_with(p)) { return Err("Pfad nicht änderbar".into()); }
            let mut j = serde_json::to_value(&app.auswahl).map_err(|e| e.to_string())?;
            *j.pointer_mut(&pfad).ok_or_else(|| format!("Unbekannte Einstellung {pfad}"))? = v["wert"].clone();
            app.auswahl = serde_json::from_value(j).map_err(|e| format!("Ungültiger Wert: {e}"))?;
        }
        "server" => { app.auswahl.server = text(v, "wert").trim().trim_end_matches('/').to_string(); app.server_pruefen(); }
        "arbeitsordner" => { let o = text(v, "wert"); if o.trim().is_empty() { return Err("Arbeitsordner fehlt".into()); } app.auswahl.arbeitsordner = o; app.auswahl.seelen_laden(); }
        "uebernehmen" => app.uebernehmen(),
        "verwerfen" => app.verwerfen(),
        "vorlage" => {
            let (r, n) = (text(v, "rolle"), text(v, "name"));
            let vorlage = VORLAGEN.iter().find(|x| x.rolle == r && x.name == n).ok_or("Vorlage unbekannt")?;
            vorlagen::anwenden(app.auswahl.rollen.get_mut(&r).ok_or("Rolle unbekannt")?, vorlage);
        }
        "modellklasse" => vorlagen::modellklasse(app.auswahl.rollen.get_mut(&text(v, "rolle")).ok_or("Rolle unbekannt")?, &text(v, "klasse")),
        "spielstil" => { let s = SPIELSTILE.iter().find(|s| s.name == text(v, "name")).ok_or("Spielstil unbekannt")?; vorlagen::spielstil_anwenden(&mut app.auswahl, s); }
        "autonomie" => {
            let (g, s) = (text(v, "gruppe"), text(v, "stufe"));
            if !GRUPPEN.iter().any(|(x, _)| *x == g) || !AUTONOMIE.iter().any(|(x, _)| *x == s) { return Err("Bereich oder Stufe unbekannt".into()); }
            let rolle = v["rolle"].as_str().filter(|r| ROLLEN.iter().any(|(x, _, _)| x == r));
            app.auswahl.autonomie_setzen(rolle, &g, &s);
        }
        "recht" => {
            let (r, k, s) = (text(v, "rolle"), text(v, "kennung"), text(v, "recht"));
            if !["erlauben", "fragen", "verbieten", "standard"].contains(&s.as_str()) || k.len() > 60 { return Err("Recht ungültig".into()); }
            let w = app.auswahl.rollen.get_mut(&r).ok_or("Rolle unbekannt")?;
            if s == "standard" { w.rechte.remove(&k); } else { w.rechte.insert(k, s); }
        }
        "baustein" => match (text(v, "art").as_str(), text(v, "wert").as_str()) {
            ("datenbank", "duckdb") => app.auswahl.datenbank = Datenbank::DuckDb, ("datenbank", _) => app.auswahl.datenbank = Datenbank::Sqlite,
            ("erweiterung", "wasm") => app.auswahl.erweiterung = Erweiterung::WasmSandbox, ("erweiterung", _) => app.auswahl.erweiterung = Erweiterung::Ohne,
            ("schnittstelle", "lokal") => app.auswahl.schnittstelle = Schnittstelle::LokaleApiMcp, ("schnittstelle", _) => app.auswahl.schnittstelle = Schnittstelle::Ohne,
            _ => return Err("Baustein unbekannt".into()),
        },
        "anbieter" => { let w = app.auswahl.rollen.get_mut(&text(v, "rolle")).ok_or("Rolle unbekannt")?; w.anbieter = Anbieter::aus_name(&text(v, "wert")).ok_or("Anbieter unbekannt")?; }
        "wissen_neu" => {
            let art = text(v, "art");
            if !["gedaechtnis", "http", "mcp"].contains(&art.as_str()) { return Err("Art unbekannt".into()); }
            let basis = match art.as_str() { "gedaechtnis" => "gedaechtnis", "http" => "rag", _ => "mcp" };
            let mut name = basis.to_string(); let mut n = 2;
            while app.auswahl.wissen.iter().any(|w| w.name == name) { name = format!("{basis}-{n}"); n += 1; }
            app.auswahl.wissen.push(katalog::WissenWahl::neu(&art, &name));
        }
        "routine_neu" => {
            let art = text(v, "art");
            if !ROUTINEN.iter().any(|(k, _, _)| *k == art) { return Err("Art unbekannt".into()); }
            let mut name = art.clone(); let mut n = 2;
            while app.auswahl.routinen.iter().any(|r| r.name == name) { name = format!("{art}-{n}"); n += 1; }
            app.auswahl.routinen.push(katalog::RoutineWahl { name, art: art.clone(), aktiv: true, alle_minuten: if art == "auto_ausbau" { 15 } else { 30 }, reihenfolge: vec![], max_auftraege: 3, melden_an: String::new() });
        }
        "skill_neu" => {
            let name = text(v, "name").trim().to_lowercase();
            if name.is_empty() || name.len() > 40 || !name.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_' || b == b'-') { return Err("Skillname: a–z, 0–9, - und _".into()); }
            let o = app.auswahl.ordner().join("skills");
            std::fs::create_dir_all(&o).map_err(|e| e.to_string())?;
            let inhalt = text(v, "text");
            std::fs::write(o.join(format!("{name}.md")), if inhalt.trim().is_empty() { format!("Wissen für den Skill {name}: kurze, prüfbare Regeln für das Spiel.\n") } else { inhalt.chars().take(6000).collect() }).map_err(|e| e.to_string())?;
        }
        // ---- Geheimnisse: gehen direkt in den Tresor, nie in Dateien
        "schluessel_wissen" => { let (q, w) = (text(v, "quelle"), text(v, "wert")); if w.trim().is_empty() { crate::tresor::loeschen(&crate::tresor::wissen_name(&q)); } else { crate::tresor::schreiben(&crate::tresor::wissen_name(&q), w.trim())?; } }
        "schluessel_openrouter" => { let w = text(v, "wert"); app.openrouter = w.trim().to_string(); if w.trim().is_empty() { crate::tresor::loeschen("openrouter"); } else { crate::tresor::schreiben("openrouter", w.trim())?; } }
        "schluessel_eigen" => { let w = text(v, "wert"); if w.trim().is_empty() { crate::tresor::loeschen("eigen"); } else { crate::tresor::schreiben("eigen", w.trim())?; } }
        "schluessel_vergessen" => app.schluessel_vergessen(),
        // ---- Bau und Start
        "server_pruefen" => app.server_pruefen(),
        "ollama_laden" => app.ollama_laden(),
        "abh_pruefen" => app.abh_neu(),
        "modellprobe" => app.modellprobe = v["wert"] == true,
        "lets_go" => app.lets_go(),
        "los" => app.los(),
        "stopp" => app.stopp(),
        "anmelden" => { if v["regeln_ok"] != true { return Err("Bitte die Regeln bestätigen".into()); } app.anmelden(text(v, "name"), text(v, "passwort"), text(v, "otp")); }
        "anmeldung_schliessen" => { app.anmeldung.offen = false; }
        "platz" => { app.anmeldung.volk = text(v, "volk"); app.platz_belegen(); }
        "fairplay_ok" => app.fairplay_bestaetigen(),
        // ---- Harness steuern (geht unverändert an den Harness, der jeden Befehl selbst prüft)
        "harness" => {
            let b = v["befehl"].clone();
            if b["befehl"] == "arena" { app.cockpit.arena_laeuft = true; }
            app.befehl(b);
        }
        "entscheiden" => app.wartend_entscheiden(&text(v, "id"), v["ja"] == true),
        "wissen" => app.wissen_abfragen(&text(v, "quelle"), &text(v, "frage"), false),
        "wissen_werkzeuge" => app.wissen_abfragen(&text(v, "quelle"), "", true),
        "seele_neu_laden" => app.seelen_neu_laden(),
        // ---- Ratsstimmen: Gespräch mit einer Figur, Profile in Voicebox, Dienste prüfen
        "gespraech" => gespraech(app, v)?,
        "stimme_einrichten" => crate::stimmen::voicebox_einrichten(),
        "stimmen_pruefen" => crate::stimmen::dienste_pruefen(),
        "meldung_weg" => app.meldung = None,
        // ---- Profile
        "profil_speichern" => match katalog::profil_speichern(&text(v, "name"), &app.auswahl) { Ok(p) => { app.meldung = Some((format!("Profil gespeichert: {}", p.display()), false)); app.profile = katalog::profile(); } Err(e) => return Err(e) },
        "profil_laden" => { app.auswahl = katalog::profil_laden(&text(v, "name"), &app.auswahl)?; app.meldung = Some(("Profil geladen – mit „Übernehmen“ wirksam machen.".into(), false)); }
        "profil_loeschen" => { katalog::profil_loeschen(&text(v, "name")); app.profile = katalog::profile(); }
        // ---- Spiel und Fenster
        "spielbereich" => {
            app.spielbereich = if v["sichtbar"] == true { Some((v["x"].as_f64().unwrap_or(0.0) as f32, v["y"].as_f64().unwrap_or(0.0) as f32, v["b"].as_f64().unwrap_or(1.0) as f32, v["h"].as_f64().unwrap_or(1.0) as f32)) } else { None };
            if v["neu_laden"] == true { app.browser_adresse(false); }
        }
        "browser_extern" => app.browser_adresse(true),
        "spiel_navigieren" => app.browser.navigieren(&text(v,"seite"))?,
        "beenden" => {if app.lokal_busy || app.lokal_speichert || app.lokal_steuert {return Err("Speicherung zuerst abwarten".into());}app.beenden=true;},
        "regeln_lesen" => crate::oeffnen::browser(&format!("{}/harness-regeln.html", app.auswahl.server.trim_end_matches('/'))),
        "oeffnen" => {
            let o = app.auswahl.ordner();
            match text(v, "was").as_str() {
                "arbeitsordner" => crate::oeffnen::ordner(&o),
                "skills" => { let p = o.join("skills"); let _ = std::fs::create_dir_all(&p); crate::oeffnen::ordner(&p); }
                "export" => { let p = o.join("export"); let _ = std::fs::create_dir_all(&p); crate::oeffnen::ordner(&p); }
                "training" => { let p = o.join("training"); let _ = std::fs::create_dir_all(&p); crate::oeffnen::ordner(&p); }
                "profile" => { let p = katalog::profil_ordner(); let _ = std::fs::create_dir_all(&p); crate::oeffnen::ordner(&p); }
                // Die Stimmen der Ratsmitglieder zum Weiterverwenden (Voicebox, Hermes, Qwen3-TTS, ComfyUI).
                "stimmen" => { let p = o.join("stimmen"); crate::stimmen::ausschreiben(&p)?; crate::oeffnen::ordner(&p); }
                "seele" => {
                    let r = text(v, "rolle");
                    if !ROLLEN.iter().any(|(x, _, _)| *x == r) { return Err("Rolle unbekannt".into()); }
                    app.auswahl.seelen_schreiben()?;
                    crate::oeffnen::datei(&o.join("seelen").join(format!("{r}.md")));
                }
                _ => return Err("Unbekanntes Ziel".into()),
            }
        }
        andere => return Err(format!("Unbekannter Befehl {andere}")),
    }
    Ok(())
}

/// Gespräch mit einem Ratsmitglied: Modell der Rolle, Persönlichkeit aus rat.json, kurzes Lagebild.
fn gespraech(app: &App, v: &Value) -> Result<(), String> {
    let rolle = text(v, "rolle");
    if !ROLLEN.iter().any(|(r, _, _)| *r == rolle) { return Err("Rolle unbekannt".into()); }
    let volk = text(v, "volk");
    if !["veyari", "aurelianer", "krath", "syntheten"].contains(&volk.as_str()) { return Err("Volk unbekannt".into()); }
    let w = app.auswahl.rollen.get(&rolle).ok_or("Rolle unbekannt")?;
    let (anbieter, url, schluessel) = match w.anbieter {
        katalog::Anbieter::OpenRouter => ("openrouter", String::new(), app.openrouter.clone()),
        katalog::Anbieter::Eigen => ("openai", app.auswahl.eigen_url.clone(), crate::tresor::lesen("eigen").unwrap_or_default()),
        katalog::Anbieter::Ollama => ("ollama", app.auswahl.ollama_url.clone(), String::new()),
    };
    let verlauf = v["verlauf"].as_array().into_iter().flatten().take(16)
        .filter_map(|x| Some((x["spieler"] == true, x["text"].as_str()?.chars().take(600).collect::<String>()))).collect();
    let l = &app.cockpit.lage;
    let a = app.cockpit.rollen.get(&rolle);
    let lage = json!({
        "reich": l["name"], "volk": l["volk"], "stufe": l["stufe"], "rang": l["rang"], "punkte": l["punkte"], "credits": l["credits"],
        "planeten": l["planeten"].as_array().map(|p| p.iter().map(|x| x["koord"].clone()).collect::<Vec<_>>()),
        "flotten": l["flotten"], "angriffe": l["angriffe"], "warnungen": l["warnungen"], "naechste_stufe": l["naechste_stufe"],
        "rat_tagt": !app.cockpit.status.is_empty() && app.cockpit.status != "gestoppt" && app.cockpit.status != "beendet",
        "warten_auf_erlaubnis": app.cockpit.wartend.len(),
        "meine_letzte_begruendung": a.map(|x| x.begruendung.chars().take(400).collect::<String>()),
        "mein_notizbuch": a.map(|x| x.notiz.chars().take(500).collect::<String>()),
        "meine_bilanz": a.map(|x| json!({"angenommen":x.angenommen,"abgelehnt":x.abgelehnt})),
        "vorraete": app.cockpit.inventur["summe"],
        "befunde": app.cockpit.analyse["kurz"],
    });
    crate::stimmen::fragen(crate::stimmen::Anfrage {
        rolle, volk, frage: text(v, "frage"), verlauf, lage, mit_stimme: v["stimme"] != false,
        modell: crate::stimmen::Modell { anbieter: anbieter.into(), url, modell: w.modell.clone(), schluessel, temperatur: w.temperatur, kontext: app.auswahl.ollama_kontext },
    })
}
