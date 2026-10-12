//! Abnahme-Modus (--abnahme ORDNER): fährt das Cockpit über genau die Funktionen, die auch
//! die Oberfläche auslöst – einrichten (Spielstil), Abhängigkeiten, Let's go, LOS mit
//! Anmeldung, spielen, Routinen, Wissensquellen, Probezug, Kontext, Arena – und bittet für
//! jede Fläche um ein Bildschirmfoto (Datei foto-NAME.wunsch; das Prüfskript nimmt das Fenster
//! auf und legt foto-NAME.png ab). Zugangsdaten nur aus der Umgebung, nie gespeichert.
use crate::app::App;
use crate::katalog::{Datenbank, Erweiterung, RoutineWahl, Schnittstelle, WissenWahl, ROLLEN};
use eframe::egui;
use serde_json::{json, Value};
use std::path::PathBuf;

pub struct Abnahme { schritt: u8, seit: f64, ordner: PathBuf, fotos: Vec<(&'static str, Value)>, foto: Option<(&'static str, f64, bool)>, angestossen: bool }

fn env(n: &str) -> String { std::env::var(n).unwrap_or_default() }

impl Abnahme {
    pub fn neu(ordner: PathBuf) -> Self {
        let fotos = vec![
            ("01-bruecke", json!({"seite":"bruecke"})),
            ("02-rat-verwalter", json!({"seite":"rat","rolle":"verwalter","stufe":"identitaet"})),
            ("03-rat-kontext", json!({"seite":"rat","rolle":"verwalter","stufe":"kontext"})),
            ("04-rat-modell", json!({"seite":"rat","rolle":"diplomat","stufe":"modell"})),
            ("05-rat-wachposten", json!({"seite":"rat","rolle":"feldherr","stufe":"wachposten"})),
            ("06-strategie", json!({"seite":"strategie"})),
            ("07-plan", json!({"seite":"plan","reiter":"aufgaben"})),
            ("08-plan-team", json!({"seite":"plan","reiter":"team"})),
            ("09-wissen", json!({"seite":"wissen","quelle":"gedaechtnis"})),
            ("10-wissen-rag", json!({"seite":"wissen","quelle":"rag"})),
            ("11-labor-probe", json!({"seite":"labor","reiter":"probe"})),
            ("12-labor-arena", json!({"seite":"labor","reiter":"arena"})),
            ("13-labor-kennzahlen", json!({"seite":"labor","reiter":"kennzahlen"})),
            ("14-codex-schiffe", json!({"seite":"codex","art":"schiffe","wahl":"zerstoerer"})),
            ("15-codex-rat", json!({"seite":"codex","art":"rat","wahl":"verwalter.veyari"})),
            ("16-werkstatt", json!({"seite":"werkstatt"})),
            ("17-spiel", json!({"seite":"spiel"})),
            ("18-rat-stimme", json!({"seite":"rat","rolle":"verwalter","stufe":"identitaet","vorstellen":"verwalter"})),
            ("19-funkfenster", json!({"seite":"bruecke","zuruf":["angriff","feldherr"]})),
        ];
        Self { schritt: 0, seit: 0.0, ordner, fotos, foto: None, angestossen: false }
    }
    fn log(&self, t: &str) {
        println!("ABNAHME {t}");
        use std::io::Write;
        if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(self.ordner.join("abnahme.log")) { let _ = writeln!(f, "{t}"); }
        let _ = std::fs::write(self.ordner.join("abnahme-stand.txt"), t);
    }
    fn weiter(&mut self, app: &App, t: &str) { self.schritt += 1; self.seit = app.zeit; self.log(t); }
    fn zeit(&self, app: &App) -> f64 { app.zeit - self.seit }

    /// Test-Harness einrichten – wie ein Spieler es mit der Oberfläche täte.
    fn einrichten(app: &mut App) {
        let modell = Some(env("STERNENEPOCHE_ABNAHME_MODELL")).filter(|m| !m.is_empty()).unwrap_or_else(|| "mock:1b".into());
        crate::vorlagen::spielstil_anwenden(&mut app.auswahl, &crate::vorlagen::SPIELSTILE[0]);
        let a = &mut app.auswahl;
        for (r, _, _) in ROLLEN {
            let w = a.rollen.get_mut(r).unwrap();
            w.aktiv = true; w.modell = modell.clone(); w.takt_minuten = 3; w.max_aufrufe_stunde = 30;
        }
        a.datenbank = Datenbank::DuckDb;
        a.erweiterung = Erweiterung::WasmSandbox;
        a.schnittstelle = Schnittstelle::LokaleApiMcp;
        a.routinen = vec![
            RoutineWahl { name: "inventur".into(), art: "inventur".into(), aktiv: true, alle_minuten: 5, reihenfolge: vec![], max_auftraege: 3, melden_an: String::new() },
            RoutineWahl { name: "analyse".into(), art: "analyse".into(), aktiv: true, alle_minuten: 5, reihenfolge: vec![], max_auftraege: 3, melden_an: "stratege".into() },
            RoutineWahl { name: "auto_ausbau".into(), art: "auto_ausbau".into(), aktiv: true, alle_minuten: 5, reihenfolge: vec!["erzmine".into(), "kristallmine".into(), "farm".into()], max_auftraege: 2, melden_an: "verwalter".into() },
        ];
        let mut g = WissenWahl::neu("gedaechtnis", "gedaechtnis");
        g.rollen = vec!["stratege".into()];
        a.wissen = vec![g];
        if !env("STERNENEPOCHE_ABNAHME_RAG").is_empty() {
            let mut w = WissenWahl::neu("http", "rag");
            w.url = env("STERNENEPOCHE_ABNAHME_RAG"); w.beschreibung = "Chronik der Galaxie: Spieler, Angriffe, Händler".into();
            w.rollen = vec!["diplomat".into()]; w.mit_schluessel = true; w.schluessel_kopf = "X-Api-Key".into();
            a.wissen.push(w);
            let _ = crate::tresor::schreiben(&crate::tresor::wissen_name("rag"), &env("STERNENEPOCHE_ABNAHME_RAG_SCHLUESSEL"));
        }
        if !env("STERNENEPOCHE_ABNAHME_MCP").is_empty() {
            let mut w = WissenWahl::neu("mcp", "mcp");
            w.url = env("STERNENEPOCHE_ABNAHME_MCP"); w.werkzeug = "suche".into(); w.beschreibung = "Flottenregister über MCP".into(); w.rollen = vec!["feldherr".into()];
            a.wissen.push(w);
        }
        app.uebernehmen();
    }

    /// Ein Schritt je Bild; liefert Steuerbefehle für die Oberfläche (Seitenwechsel).
    pub fn schritt(&mut self, app: &mut App, ctx: &egui::Context) -> Vec<Value> {
        let mut steuern = Vec::new();
        match self.schritt {
            0 => if app.server_info.is_some() && app.katalog_regeln.is_some() && self.zeit(app) > 3.0 {
                    Self::einrichten(app); app.abh_neu();
                    steuern.push(json!({"seite":"werkstatt"}));
                    self.weiter(app, "eingerichtet (Spielstil Friedlicher Aufbau), Abhängigkeiten werden geprüft");
                 } else if self.zeit(app) > 40.0 { self.abbruch(ctx, "Server oder Regelkatalog nicht erreichbar"); },
            1 => if !app.abh_laedt && !app.abh.is_empty() {
                    if app.abh_blockiert() { self.abbruch(ctx, &format!("Abhängigkeiten blockiert: {:?}", app.abh.iter().filter(|a| a.blockiert).map(|a| &a.name).collect::<Vec<_>>())); return steuern; }
                    app.lets_go(); self.weiter(app, "Let's go: Bau läuft");
                 },
            2 => match &app.bau_ende {
                Some(Ok(_)) => { self.weiter(app, "gebaut und geprüft"); }
                Some(Err(e)) => { let e = e.clone(); self.abbruch(ctx, &format!("Bau fehlgeschlagen: {e}")); }
                None => {}
            },
            3 => if self.foto_machen(app, "00-werkstatt-bau") { app.los(); self.weiter(app, "LOS gedrückt"); },
            4 => {
                if app.anmeldung.offen && !app.anmeldung.laeuft && app.sitzung.is_none() && app.anmeldung.fehler.is_none() {
                    app.anmelden(env("STERNENEPOCHE_ABNAHME_NAME"), env("STERNENEPOCHE_ABNAHME_PASSWORT"), String::new());
                    self.log("Anmeldung gesendet");
                }
                if app.anmeldung.platz_noetig && !app.anmeldung.laeuft { app.platz_belegen(); self.log("Platz wird belegt"); }
                if app.laeuft() { steuern.push(json!({"seite":"bruecke"})); self.weiter(app, "Harness läuft"); }
                else if self.zeit(app) > 90.0 { let f = app.anmeldung.fehler.clone().unwrap_or_default(); self.abbruch(ctx, &format!("Harness startet nicht: {f}")); }
            }
            5 => {
                let gezogen = app.cockpit.rollen.values().filter(|r| r.aufrufe > 0).count();
                let routinen = ["inventur", "analyse", "auto_ausbau"].iter().all(|n| app.cockpit.routinen.get(*n).is_some_and(|r| r.get("ok").is_some()));
                if (gezogen >= 4 && !app.cockpit.aufgaben.is_empty() && routinen && self.zeit(app) > 10.0) || self.zeit(app) > 420.0 {
                    self.weiter(app, &format!("{gezogen} Rollen haben gezogen, {} Aufgaben, Routinen {}", app.cockpit.aufgaben.len(), if routinen { "gelaufen" } else { "unvollständig" }));
                }
            }
            6 => {
                if !self.angestossen {
                    for b in [json!({"befehl":"probezug","rolle":"verwalter"}), json!({"befehl":"kontext","rolle":"verwalter"}), json!({"befehl":"kontext","rolle":"diplomat"}),
                        json!({"befehl":"wissen","quelle":"gedaechtnis","frage":"SELECT koord, erz, kristall, deuterium, nahrung, energie_saldo, bauschleife FROM inventur WHERE zeit = (SELECT max(zeit) FROM inventur) ORDER BY koord","id":"abnahme-sql"}),
                        json!({"befehl":"kennzahlen"}), json!({"befehl":"export"}), json!({"befehl":"training_export","nur_erfolgreich":false})] { app.befehl(b); }
                    if app.auswahl.wissen.iter().any(|w| w.name == "rag") { app.befehl(json!({"befehl":"wissen","quelle":"rag","frage":"Was wissen wir über Xator?","id":"abnahme-rag"})); }
                    if app.auswahl.wissen.iter().any(|w| w.name == "mcp") { app.befehl(json!({"befehl":"wissen","quelle":"mcp","frage":"Flotten von Xator","id":"abnahme-mcp"})); app.befehl(json!({"befehl":"wissen_werkzeuge","quelle":"mcp"})); }
                    let modell = app.auswahl.rollen["verwalter"].modell.clone();
                    app.cockpit.arena_laeuft = true;
                    app.befehl(json!({"befehl":"arena","rolle":"verwalter","varianten":[{"name":"Sparsamer Aufbauer","anbieter":"ollama","modell":modell},{"name":"Industrieller","anbieter":"ollama","modell":modell,
                        "seele":crate::vorlagen::VORLAGEN.iter().find(|v| v.name == "Industrieller").map(|v| v.seele).unwrap_or("")}]}));
                    self.angestossen = true;
                    self.log("Probezug, Kontext, Wissen, Kennzahlen, Export, Training und Arena angestoßen");
                }
                let fertig = !app.cockpit.probezuege.is_empty() && app.cockpit.kontext.contains_key("verwalter") && !app.cockpit.arena_laeuft
                    && app.cockpit.wissen.iter().filter(|w| w["rolle"] == "besitzer").count() >= app.auswahl.wissen.len().min(3);
                if (fertig && self.zeit(app) > 3.0) || self.zeit(app) > 240.0 {
                    let ok: Vec<String> = app.cockpit.wissen.iter().map(|w| format!("{}={}", w["quelle"].as_str().unwrap_or("?"), if w["ok"] == true { "ok" } else { "fehler" })).collect();
                    self.weiter(app, &format!("Probezug {}, Kontext {}, Arena {}, Wissen [{}]", app.cockpit.probezuege.len(), app.cockpit.kontext.len(), app.cockpit.arena.is_some(), ok.join(", ")));
                }
            }
            7 => {
                if self.foto.is_none() {
                    let Some((name, ziel)) = self.fotos.first().cloned() else { self.weiter(app, "alle Flächen aufgenommen"); return steuern; };
                    steuern.push(ziel);
                    self.foto = Some((name, app.zeit, false));
                } else if let Some((name, t, _)) = self.foto {
                    // Vorstellung und Zuruf brauchen etwas länger (Kopf-Zoom, Untertitel, Funkfenster).
                    let warten = if name.starts_with("18-") { 4.5 } else if name.starts_with("19-") { 1.5 } else { 3.0 };
                    if app.zeit - t > warten && self.foto_machen(app, name) { self.fotos.remove(0); self.foto = None; }
                }
            }
            8 => {
                if let Some(l) = &mut app.laufzeit { l.stoppen(); }
                app.schluessel_vergessen();
                crate::tresor::loeschen(&crate::tresor::wissen_name("rag"));
                self.weiter(app, "Harness gestoppt, Test-Schlüssel entfernt");
            }
            9 => if self.zeit(app) > 2.0 { self.log("FERTIG"); self.schritt = 99; ctx.send_viewport_cmd(egui::ViewportCommand::Close); },
            _ => {}
        }
        steuern
    }
    /// Foto erbitten und auf die Aufnahme warten (höchstens 25 s). true = erledigt.
    fn foto_machen(&mut self, app: &App, name: &str) -> bool {
        let wunsch = self.ordner.join(format!("foto-{name}.wunsch"));
        let bild = self.ordner.join(format!("{name}.png"));
        if bild.exists() { let _ = std::fs::remove_file(&wunsch); return true; }
        if !wunsch.exists() { let _ = std::fs::write(&wunsch, name); self.log(&format!("Foto {name}")); self.seit = app.zeit; }
        app.zeit - self.seit > 25.0
    }
    fn abbruch(&mut self, ctx: &egui::Context, grund: &str) {
        self.log(&format!("ABBRUCH {grund}"));
        self.schritt = 99;
        crate::tresor::loeschen(&crate::tresor::wissen_name("rag"));
        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
    }
}
