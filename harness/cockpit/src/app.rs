//! Zustand und Abläufe des Cockpits – unabhängig von der Darstellung. Die Oberfläche
//! (oberflaeche.rs, ui/) zeigt diesen Zustand und schickt Befehle; alle Prüfungen,
//! Geheimnisse und Prozesse bleiben hier im Rust-Kern.
use crate::abhaengigkeiten::{self, Abh, Stand as AbhStand};
use crate::bau::{self, Meldung, Stand as BauStand};
use crate::katalog::{self, Auswahl};
use crate::laufzeit::Laufzeit;
use crate::verbindung::{Sitzung, Verbindung};
use serde_json::{json, Value};
use std::collections::{BTreeMap, VecDeque};
use std::sync::mpsc::{channel, Receiver, Sender};

/// Lokaler Zeitstempel (UTC) für Ordnernamen, z. B. 2026-10-11_1805.
pub fn zeitstempel(t: i64) -> String {
    let (tage, rest) = (t.div_euclid(86_400), t.rem_euclid(86_400));
    // Bürgerlicher Kalender aus Tagen seit 1970 (H. Hinnant).
    let z = tage + 719_468; let era = z.div_euclid(146_097); let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365; let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153; let d = doy - (153 * mp + 2) / 5 + 1; let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    format!("{y:04}-{m:02}-{d:02}_{:02}{:02}", rest / 3600, rest % 3600 / 60)
}
pub fn jetzt() -> i64 { std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0) }

pub enum Ergebnis {
    LokalStart(Result<(sternenepoche_server::local::LocalSession, Sitzung), String>),
    LokalBeitritt(String, Result<Sitzung, String>),
    LokalSuche(Result<Vec<Value>, String>),
    LokalInfo(String, u64, Result<Value, String>),
    LokalGesteuert(String, Result<Value, String>),
    /// Die lokale Welt hat ihren letzten Checkpoint geschrieben und ist geschlossen.
    LokalBeendet,
    /// Eine Hintergrundarbeit ist mit einem Programmfehler abgebrochen (protokolliert).
    Panik(String),
    LokalGespeichert(String, Result<(), String>),
    Testpilot(Result<(Sitzung,Value,String),String>),
    ServerInfo(String, Result<Value, String>),
    AllianzCodex(String, String, String, Result<Value, String>),
    Ollama(Result<Vec<(String, u64)>, String>),
    Abhaengigkeiten(Vec<Abh>),
    Angemeldet(Result<(Sitzung, Value), String>),
    Schluessel(Result<String, String>),
    Platz(Result<Value, String>),
    Adresse(String, bool),
    Fairplay(Result<Value, String>),
    Gepruefter(Result<Value, String>),
    Regeln(String, Result<Value, String>),
    Wissen(Value),
    WissenWerkzeuge(Value),
    HardwareAktuell(Value,Value),
}

#[derive(Default, Clone, serde::Serialize)]
pub struct RolleAnzeige { pub status: String, pub modell: String, pub aufrufe: u64, pub tokens: u64, pub latenz: u64, pub begruendung: String, pub fehler: String,
    pub naechster: i64, pub aus: bool, pub notiz: String, pub angenommen: u64, pub abgelehnt: u64, pub kosten: f64, pub seele_version: String, pub stufe: String, pub stufe_seit: i64 }

/// Was der laufende Harness meldet, aufbereitet für die Oberfläche.
#[derive(Default)]
pub struct CockpitZustand {
    pub status: String,
    pub rollen: BTreeMap<String, RolleAnzeige>,
    pub lage: Value,
    pub wartend: Vec<Value>,
    pub alarme: VecDeque<Value>,
    pub quote: String,
    pub gedaechtnis: Value,
    pub konto: Value,
    pub lokale_api: Option<String>,
    pub zuege: u32,
    pub angenommen: u32,
    pub abgelehnt: u32,
    pub zugliste: VecDeque<Value>,
    pub aufgaben: Vec<Value>,
    pub team: Vec<Value>,
    pub erinnerungen: Vec<Value>,
    pub notizen: BTreeMap<String, Vec<Value>>,
    /// Modellaufrufe vollständig (für die Detailansicht); die Oberfläche bekommt nur Kurzfassungen.
    pub verlauf: VecDeque<Value>,
    pub inventur: Value,
    pub analyse: Value,
    pub routinen: BTreeMap<String, Value>,
    pub wissen: VecDeque<Value>,
    pub wissen_laeuft: bool,
    pub mcp_werkzeuge: BTreeMap<String, Value>,
    pub kontext: BTreeMap<String, Value>,
    pub probezuege: VecDeque<Value>,
    pub arena: Option<Value>,
    pub arena_laeuft: bool,
    pub kennzahlen: Value,
    pub export: Option<String>,
    /// Neue Ereignisse für den Live-Strom der Oberfläche (werden beim Senden geleert).
    pub neu: Vec<Value>,
}

/// Kurzfassung eines Ereignisses für den Live-Strom.
fn kurzfassung(e: &Value) -> String {
    let s = |v: &Value| v.as_str().unwrap_or("").chars().take(160).collect::<String>();
    match e["art"].as_str().unwrap_or("") {
        "zug" => format!("{} · {} Aktionen · {}", s(&e["quelle"]), e["aktionen"].as_array().map(Vec::len).unwrap_or(0), s(&e["begruendung"])),
        "lage" => format!("Lage gelesen · Quote {}", s(&e["quote"])),
        "verlauf" => format!("Modell {} · {} Tokens · {} ms", s(&e["modell"]), e["tokens"], e["latenz_ms"]),
        "routine" => format!("{} {} {}", s(&e["name"]), s(&e["status"]), s(&e["kurz"])),
        "wissen_antwort" => format!("{} fragt {}: {}", s(&e["rolle"]), s(&e["quelle"]), if e["ok"] == true { format!("ok, {} ms", e["dauer_ms"]) } else { s(&e["fehler"]) }),
        "inventur" => format!("{} Planeten erfasst", e["inventur"]["summe"]["planeten"]),
        "analyse" => s(&e["analyse"]["kurz"]),
        "denkt" => format!("denkt mit {}{}", s(&e["modell"]), if e["probe"] == true { " (Probezug)" } else { "" }),
        "kontext" => format!("Kontext {} Zeichen ≈ {} Tokens", e["zeichen"], e["tokens_geschaetzt"]),
        "arena" => "Arena-Vergleich fertig".into(),
        _ => ["text", "grund", "kurz", "status"].iter().map(|k| s(&e[*k])).find(|x| !x.is_empty()).unwrap_or_default(),
    }
}

impl CockpitZustand {
    pub fn alarm(&mut self, text: impl Into<String>, stufe: u8) {
        let text = text.into();
        if self.alarme.iter().take(12).any(|a| a["text"] == text.as_str()) { return; }
        self.alarme.push_front(json!({"zeit":jetzt(),"text":text,"stufe":stufe}));
        self.alarme.truncate(60);
    }
    fn stufe(&mut self, rolle: &str, stufe: &str) {
        if rolle.is_empty() { return; }
        let r = self.rollen.entry(rolle.to_string()).or_default();
        r.stufe = stufe.into(); r.stufe_seit = jetzt();
    }
    pub fn verarbeiten(&mut self, e: &Value) {
        let rolle = e["rolle"].as_str().unwrap_or("").to_string();
        let kurz = |v: &Value| v.as_str().unwrap_or("").chars().take(300).collect::<String>();
        let art = e["art"].as_str().unwrap_or("").to_string();
        if !["gedaechtnis", "gedaechtnis_daten", "planung", "konto", "protokoll"].contains(&art.as_str()) {
            self.neu.push(json!({"zeit":e["zeit"].as_i64().unwrap_or_else(jetzt),"art":art,"rolle":rolle,"text":kurzfassung(e)}));
        }
        match art.as_str() {
            "zug" if e["nur_probe"] == true && e["quelle"] == "Probezug" => {
                self.probezuege.push_front(e.clone());
                self.probezuege.truncate(20);
                self.stufe(&rolle, "bereit");
                if let Some(r) = self.rollen.get_mut(&rolle) { r.status = "bereit".into(); }
                self.alarm(format!("Probezug {rolle} fertig – nichts wurde gespielt"), 0);
            }
            "arena" => { self.arena = Some(e.clone()); self.arena_laeuft = false; self.alarm("Arena-Vergleich fertig", 0); }
            "kennzahlen" => self.kennzahlen = e["reihen"].clone(),
            "inventur" => { self.inventur = e["inventur"].clone(); self.inventur["zeit"] = json!(e["zeit"].as_i64().unwrap_or_else(jetzt)); }
            "analyse" => {
                self.analyse = e["analyse"].clone();
                self.analyse["zeit"] = json!(e["zeit"].as_i64().unwrap_or_else(jetzt));
                for b in e["analyse"]["befunde"].as_array().into_iter().flatten().filter(|b| b["stufe"] == 2).take(3) {
                    self.alarm(format!("{} {}: {}", kurz(&b["thema"]), b["planet"].as_str().unwrap_or(""), kurz(&b["text"])), 1);
                }
            }
            "routine" => {
                let name = kurz(&e["name"]);
                if e["status"] == "fertig" {
                    if e["ok"] != true { self.alarm(format!("Routine {name}: {}", kurz(&e["kurz"])), 2); }
                    let mut v = e.clone(); v["laeuft"] = json!(false);
                    self.routinen.insert(name, v);
                } else { self.routinen.entry(name).or_insert_with(|| json!({}))["laeuft"] = json!(true); }
            }
            "wissen_antwort" => { if e["rolle"] == "besitzer" { self.wissen_laeuft = false; } self.wissen.push_front(e.clone()); self.wissen.truncate(40); }
            "wissen_werkzeuge" => { self.wissen_laeuft = false; self.mcp_werkzeuge.insert(kurz(&e["quelle"]), e.clone()); }
            "kontext" => { self.kontext.insert(rolle, e.clone()); }
            "export" => { self.export = e["pfad"].as_str().map(str::to_string); let datei = e["pfad"].as_str().and_then(|p| std::path::Path::new(p).file_name()).map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
                self.alarm(format!("Exportiert: {} Zeilen → {datei}", e["zeilen"]), 0); }
            "status" => { self.status = kurz(&e["status"]); if e["status"] == "beendet" { self.alarm(format!("Harness beendet: {}", kurz(&e["grund"])), 2); } }
            "freigabe" => self.alarm("Exklusive Steuerung des Reichs übernommen", 0),
            "konto" => self.konto = e["konto"].clone(),
            "lokale_api" => self.lokale_api = e["url"].as_str().map(str::to_string),
            "konfig" => self.alarm("Neue Einstellungen im laufenden Harness übernommen", 0),
            "ausloeser" => { self.stufe(&rolle, "ausloeser"); self.alarm(format!("Auslöser „{}“ weckt {rolle}", kurz(&e["grund"])), 1); }
            "lage" => {
                self.lage = e["kurz"].clone();
                self.quote = kurz(&e["quote"]);
                self.stufe(&rolle, "wahrnehmung");
                for a in e["kurz"]["angriffe"].as_array().into_iter().flatten().take(3) {
                    self.alarm(format!("Anflug auf {} – Ankunft {}", a["ziel"].as_str().unwrap_or("?"), a["ankunft"].as_str().unwrap_or("?")), 2);
                }
                for h in e["kurz"]["server_hinweise"].as_array().into_iter().flatten() {
                    self.alarm(format!("Server: {}", kurz(&h["text"])), if h["art"].as_str().unwrap_or("").starts_with("fairplay") { 2 } else { 1 });
                }
            }
            "denkt" => {
                let r = self.rollen.entry(rolle.clone()).or_default(); r.status = "denkt".into(); r.modell = kurz(&e["modell"]);
                if let Some(s) = e["seele_version"].as_str() { r.seele_version = s.into(); }
                self.stufe(&rolle, "modell");
            }
            "verlauf" => {
                self.stufe(&rolle, "werkzeuge");
                self.verlauf.push_front(e.clone()); self.verlauf.truncate(40);
            }
            "korrektur" => { let r = self.rollen.entry(rolle.clone()).or_default(); r.status = "korrigiert".into(); self.stufe(&rolle, "probe"); }
            "zug" => {
                if e["quelle"] == "Modell" { self.zuege += 1; }
                let ok = e["ergebnisse"].as_array().map(|a| a.iter().filter(|x| x["ok"] == true).count()).unwrap_or(0) as u32;
                let alle = e["ergebnisse"].as_array().map(Vec::len).unwrap_or(0) as u32;
                self.angenommen += ok; self.abgelehnt += alle - ok + e["abgelehnt"].as_array().map(Vec::len).unwrap_or(0) as u32;
                let r = self.rollen.entry(rolle.clone()).or_default();
                r.status = "bereit".into(); r.fehler.clear();
                if e["quelle"] == "Modell" { r.aufrufe += 1; r.kosten += e["kosten_usd"].as_f64().unwrap_or(0.0); }
                r.angenommen += ok as u64; r.abgelehnt += (alle - ok) as u64;
                r.tokens = e["tokens"].as_u64().unwrap_or(r.tokens); r.latenz = e["latenz_ms"].as_u64().unwrap_or(r.latenz);
                if let Some(n) = e["naechster_zug"].as_i64() { r.naechster = n; }
                if let Some(m) = e["modell"].as_str() { r.modell = m.into(); }
                if let Some(s) = e["seele_version"].as_str() { r.seele_version = s.into(); }
                if let Some(b) = e["begruendung"].as_str().filter(|b| !b.is_empty()) { r.begruendung = b.chars().take(600).collect(); }
                if let Some(n) = e["notiz"].as_str().filter(|n| !n.is_empty()) { r.notiz = n.chars().take(2000).collect(); }
                self.stufe(&rolle, "lernen");
                self.quote = kurz(&e["quote"]);
                self.zugliste.push_front(e.clone());
                self.zugliste.truncate(120);
            }
            "bestaetigung_noetig" => { self.wartend.push(e.clone()); self.stufe(&rolle, "wachposten"); self.alarm(format!("Der Rat bittet um Erlaubnis: {}", kurz(&e["grund"])), 1); }
            "gedaechtnis" => self.gedaechtnis = e.clone(),
            "planung" => self.aufgaben = e["aufgaben"].as_array().cloned().unwrap_or_default(),
            "gedaechtnis_daten" => {
                self.team = e["team"].as_array().cloned().unwrap_or_default();
                self.erinnerungen = e["erinnerungen"].as_array().cloned().unwrap_or_default();
                self.notizen = e["notizen"].as_object().map(|o| o.iter().map(|(k, v)| (k.clone(), v.as_array().cloned().unwrap_or_default())).collect()).unwrap_or_default();
                if e["statistik"].is_object() { self.gedaechtnis = e["statistik"].clone(); }
            }
            "rolle" => { let r = self.rollen.entry(rolle).or_default(); r.aus = e["aktiv"] == false; r.status = if r.aus { "angehalten".into() } else { "bereit".into() }; }
            "fehler" if e["befehl"] == "wissen" || e["befehl"] == "wissen_werkzeuge" => {
                self.wissen_laeuft = false;
                self.wissen.push_front(json!({"ok":false,"quelle":e["quelle"],"rolle":"besitzer","fehler":e["text"],"dauer_ms":0}));
            }
            "fehler" if e["befehl"] == "arena" => { self.arena_laeuft = false; self.alarm(format!("Arena: {}", kurz(&e["text"])), 2); }
            "fehler" => {
                let text = kurz(&e["text"]);
                if !rolle.is_empty() { let r = self.rollen.entry(rolle.clone()).or_default(); r.status = "Fehler".into(); r.fehler = text.clone(); }
                self.alarm(format!("Fehler {rolle}: {text}"), 2);
            }
            "warnung" => self.alarm(kurz(&e["text"]), 1),
            "abbruch" => self.alarm(format!("Abbruch: {}", kurz(&e["text"])), 2),
            _ => {}
        }
    }
}

#[derive(Default)]
pub struct Anmeldung { pub offen: bool, pub laeuft: bool, pub fehler: Option<String>, pub volk: String, pub platz_noetig: bool, pub warteliste: Option<u64> }

pub struct App {
    pub codex_allianzen: Value,
    pub hardware: Value,
    pub agenten: Option<crate::lokale_agenten::Flotte>,
    pub lokal: Option<sternenepoche_server::local::LocalSession>,
    pub lokal_busy: bool,
    pub lokal_steuert: bool,
    pub lokal_info_generation: u64,
    pub lokal_speichert: bool,
    pub lokal_gespeichert: Option<i64>,
    pub lokal_auto_resume: bool,
    fortsetzung: Option<crate::lokaler_spielstand::Fortsetzung>,
    pub lokal_hosts: Vec<Value>,
    pub lokal_info: Value,
    /// LAN-Adressen des eigenen Hosts mit QR-Code (einmal beim Start berechnet).
    pub lokal_lan: Value,
    lokal_poll: f64,
    lokal_info_pending: bool,
    hardware_poll: f64,
    hardware_pending: bool,
    pub online_server: Option<String>,
    pub auswahl: Auswahl,
    pub gespeichert: Auswahl,
    pub openrouter: String,
    pub ollama: Vec<(String, u64)>,
    pub ollama_fehler: Option<String>,
    pub gpu: Value,
    pub server_info: Option<Value>,
    pub server_fehler: Option<String>,
    pub abh: Vec<Abh>,
    pub abh_laedt: bool,
    pub modellprobe: bool,
    pub bau_rx: Option<Receiver<Meldung>>,
    pub bau_schritte: Vec<(BauStand, String, f32)>,
    pub bau_log: VecDeque<String>,
    pub pruefungen: Vec<Value>,
    pub bau_ende: Option<Result<String, String>>,
    pub laufzeit: Option<Laufzeit>,
    pub cockpit: CockpitZustand,
    pub sitzung: Option<Sitzung>,
    pub anmeldung: Anmeldung,
    pub fairplay_text: Option<String>,
    pub tx: Sender<Ergebnis>,
    pub rx: Receiver<Ergebnis>,
    pub browser: crate::browser::Einbettung,
    pub browser_ziel: Option<String>,
    /// Bereich des Spiels in der Oberfläche (x, y, Breite, Höhe in logischen Pixeln), wenn sichtbar.
    pub spielbereich: Option<(f32, f32, f32, f32)>,
    pub minimieren: bool,
    pub beenden: bool,
    pub zeit: f64,
    pub meldung: Option<(String, bool)>,
    pub katalog_regeln: Option<Value>,
    pub katalog_fehler: Option<String>,
    pub profile: Vec<String>,
}

impl App {
    pub fn neu(auswahl: Auswahl) -> Self {
        let (tx, rx) = channel();
        let mut app = Self { codex_allianzen:Value::Null,lokal_speichert:false,lokal_gespeichert:None,lokal_auto_resume:false,fortsetzung:None,hardware_poll:0.0,hardware_pending:false,lokal_poll:0.0,lokal_info_pending:false,hardware:Value::Null,agenten:None,lokal: None, lokal_busy: false, lokal_steuert:false, lokal_info_generation:0, lokal_hosts: vec![], lokal_info: Value::Null, lokal_lan: Value::Null, online_server: None, gespeichert: auswahl.clone(), auswahl, openrouter: crate::tresor::lesen("openrouter").unwrap_or_default(), ollama: vec![], ollama_fehler: None, gpu: Value::Null,
            server_info: None, server_fehler: None, abh: vec![], abh_laedt: false, modellprobe: true, bau_rx: None, bau_schritte: vec![], bau_log: VecDeque::new(), pruefungen: vec![], bau_ende: None,
            laufzeit: None, cockpit: Default::default(), sitzung: None, anmeldung: Anmeldung { volk: "veyari".into(), ..Default::default() }, fairplay_text: None,
            tx, rx, browser: crate::browser::Einbettung::neu(), browser_ziel: None, spielbereich: None, minimieren: false, beenden:false, zeit: 0.0, meldung: None,
            katalog_regeln: None, katalog_fehler: None, profile: katalog::profile() };
        app.server_pruefen();
        app.ollama_laden();
        app.hardware_laden();
        let root=app.auswahl.ordner();
        let resume=crate::lokaler_spielstand::Fortsetzung::laden(&root);
        match resume {
            Ok(Some(s)) if s.aktiv => {
                app.auswahl.lokale_bots=s.bots; app.auswahl.lokaler_host_aktiv=s.host; app.auswahl.lokaler_port=s.port;
                app.lokal_auto_resume=true;
                if let Err(e)=app.lokal_starten(s.name,s.volk,String::new()) { app.lokal_auto_resume=false; app.meldung=Some((format!("Partie nicht automatisch fortgesetzt: {e}"),true)); }
            }
            // Upgrade an existing offline installation without a resume descriptor.
            Ok(None) if !app.auswahl.lokaler_host_aktiv => {
                let world=root.join("singleplayer").join(format!("bots-{}",app.auswahl.lokale_bots));
                if let Ok(Some(name))=sternenepoche_server::local::saved_host_player(&world) {
                    app.lokal_auto_resume=true;
                    if let Err(e)=app.lokal_starten(name,"veyari".into(),String::new()) { app.lokal_auto_resume=false; app.meldung=Some((e,true)); }
                }
            }
            Err(e)=>app.meldung=Some((format!("Wiederaufnahme nicht lesbar: {e}"),true)),
            _=>{}
        }
        app
    }

    pub fn hintergrund<F: FnOnce() -> Ergebnis + Send + 'static>(&self, f: F) {
        let tx = self.tx.clone();
        // Ein Fehler in einer Hintergrundarbeit darf keinen Zustand „läuft“ für immer hängen lassen.
        std::thread::spawn(move || {
            let e = std::panic::catch_unwind(std::panic::AssertUnwindSafe(f)).unwrap_or_else(|p| Ergebnis::Panik(crate::absturz::nutzlast(p.as_ref())));
            let _ = tx.send(e);
        });
    }
    pub fn lokal_starten(&mut self, name: String, volk: String, password: String) -> Result<(), String> {
        if self.lokal_busy || self.lokal.is_some() || self.online_server.is_some() { return Err("Lokale Sitzung zuerst verlassen".into()); }
        if self.laeuft() { return Err("Den Rat vor dem Wechsel anhalten".into()); }
        sternenepoche_server::local::dimensions(self.auswahl.lokale_bots)?;
        let host = self.auswahl.lokaler_host_aktiv;
        let password = if host {
            if password.is_empty() { crate::tresor::lesen("lokal-master").ok_or("Masterpasswort in den lokalen Einstellungen hinterlegen")? }
            else { sternenepoche_server::password_hash(&password)?; crate::tresor::schreiben("lokal-master", &password)?; password }
        } else { sternenepoche_server::token() };
        let bots = self.auswahl.lokale_bots; let port = self.auswahl.lokaler_port;
        self.einstellungen_speichern()?;
        self.fortsetzung=Some(crate::lokaler_spielstand::Fortsetzung { aktiv:true,name:name.clone(),volk:volk.clone(),bots,host,port });
        let root = self.auswahl.ordner().join(if host { "lokal-host" } else { "singleplayer" }).join(format!("bots-{bots}"));
        self.lokal_busy = true;
        self.hintergrund(move || Ergebnis::LokalStart((|| {
            let runtime = sternenepoche_server::local::LocalSession::start(&root,bots,host,port,&password)?;
            let s = Verbindung::neu(&runtime.url)?.lokal_anmelden(&name,&password,&volk)?;
            Ok((runtime,s))
        })()));
        Ok(())
    }
    pub fn lokal_verbunden(&mut self, url: String, s: Sitzung) {
        self.online_server = Some(self.auswahl.server.clone()); self.auswahl.server = url;
        let url = self.auswahl.server.clone(); let browser_s = s.clone();
        self.hintergrund(move || Ergebnis::Adresse(Verbindung::neu(&url).map(|v|v.browser_adresse(Some(&browser_s))).unwrap_or_default(),false));
        self.sitzung = Some(s); self.server_pruefen();
        self.lokal_info_holen();
    }
    /// Lage der lokalen Welt; der eigene Host bekommt zusätzlich die Teilnehmerliste.
    pub fn lokal_info_holen(&mut self) {
        let (url, sitzung, host) = (self.auswahl.server.clone(), self.sitzung.clone(), self.lokal.is_some());
        self.lokal_poll = self.zeit; self.lokal_info_pending = true;
        let generation=self.lokal_info_generation;
        self.hintergrund(move || {
            let r = Verbindung::neu(&url).and_then(|v| {
                let mut info = v.lokal_info()?;
                if let (true, Some(s)) = (host, &sitzung) { info["teilnehmer"] = v.lokal_spieler(s).map(|t| t["spieler"].clone()).unwrap_or(Value::Null); }
                Ok(info)
            });
            Ergebnis::LokalInfo(url, generation, r)
        });
    }
    pub fn lokal_verlassen(&mut self) {
        if let Some(s)=&mut self.fortsetzung {
            s.aktiv=false;
            if let Err(e)=s.speichern(&self.auswahl.ordner()) {self.meldung=Some((e,true));return;}
        }
        self.lokal_auto_resume=false; self.lokal_gespeichert=None;
        self.agenten.take();
        if let Some(l) = &mut self.laufzeit { l.stoppen(); }
        // Die Welt schreibt beim Schließen ihren letzten Checkpoint; das dauert bei großen
        // Galaxien einen Moment und gehört nicht in den Oberflächen-Thread. Bis dahin bleibt
        // „busy“ gesetzt, damit kein neuer Start auf den noch gesperrten Spielstand trifft.
        if let Some(l) = self.lokal.take() { self.lokal_busy = true; self.hintergrund(move || { drop(l); Ergebnis::LokalBeendet }); }
        self.sitzung = None; self.lokal_info = Value::Null; self.lokal_info_pending=false; self.lokal_lan = Value::Null;
        if let Some(s) = self.online_server.take() { self.auswahl.server = s; self.browser_ziel = Some(format!("{}/",self.auswahl.server)); self.server_pruefen(); }
    }
    /// An acknowledged control transaction durably writes the current checkpoint.
    pub fn lokal_speichern(&mut self) -> Result<(),String> {
        if self.lokal.is_none() {return Err("Der gemeinsame Spielstand wird beim Host gespeichert".into());}
        if self.lokal_speichert || self.lokal_busy || self.lokal_steuert {return Err("Speicherung läuft bereits".into());}
        if let Some(s)=&self.fortsetzung {s.speichern(&self.auswahl.ordner())?;}
        let s=self.sitzung.clone().ok_or("Keine aktive Sitzung")?;
        let url=self.auswahl.server.clone(); self.lokal_speichert=true;
        self.hintergrund(move || {let r=Verbindung::neu(&url).and_then(|c|c.lokal_steuern(&s,json!({"speichern":true})).map(|_|()));Ergebnis::LokalGespeichert(url,r)});
        Ok(())
    }
    /// Neue Partie: Der Spielstand dieser Botzahl wird in `archiv/` verschoben (nichts wird
    /// gelöscht); der nächste Start erzeugt eine neue, zufällige Galaxie.
    pub fn lokal_neue_partie(&mut self) -> Result<String, String> {
        if self.lokal_busy || self.lokal.is_some() || self.online_server.is_some() { return Err("Lokale Sitzung zuerst verlassen".into()); }
        let host = self.auswahl.lokaler_host_aktiv; let bots = self.auswahl.lokale_bots;
        let basis = self.auswahl.ordner().join(if host { "lokal-host" } else { "singleplayer" });
        let alt = basis.join(format!("bots-{bots}"));
        if !alt.join("spiel.sqlite3").exists() { return Ok("Noch kein Spielstand mit dieser Botzahl – der nächste Start beginnt eine neue Partie.".into()); }
        let ziel = basis.join("archiv").join(format!("bots-{bots}-{}", zeitstempel(jetzt())));
        std::fs::create_dir_all(basis.join("archiv")).map_err(|e| e.to_string())?;
        std::fs::rename(&alt, &ziel).map_err(|e| format!("Spielstand nicht archiviert ({e}); läuft er noch in einem anderen Cockpit?"))?;
        Ok(format!("Bisheriger Spielstand archiviert: {}. Der nächste Start beginnt eine neue Galaxie.", ziel.display()))
    }
    pub fn server_pruefen(&mut self) {
        let s = self.auswahl.server.clone();
        self.server_info = None; self.server_fehler = None;
        self.hintergrund(move || {let r=Verbindung::neu(&s).and_then(|v|v.info());Ergebnis::ServerInfo(s,r)});
        let s = self.auswahl.server.clone();
        self.hintergrund(move || {let r=Verbindung::neu(&s).and_then(|v|v.regeln());Ergebnis::Regeln(s,r)});
    }
    pub fn ollama_laden(&self) {
        let u = self.auswahl.ollama_url.clone();
        self.hintergrund(move || Ergebnis::Ollama(abhaengigkeiten::ollama_modelle(&u)));
    }
    pub fn hardware_laden(&mut self){
        if self.hardware_pending{return;}
        self.hardware_pending=true;self.hardware_poll=self.zeit;
        self.hintergrund(||Ergebnis::HardwareAktuell(crate::lokale_agenten::hardware(),abhaengigkeiten::gpu()));
    }
    pub fn laeuft(&mut self) -> bool { self.laufzeit.as_mut().is_some_and(|l| l.laeuft()) }
    pub fn geaendert(&self) -> bool { self.auswahl != self.gespeichert }
    /// Welche Einstellungen sich seit dem letzten Übernehmen geändert haben (Pfade, höchstens 12).
    pub fn aenderungen(&self) -> Vec<String> {
        fn diff(a: &Value, b: &Value, pfad: &str, out: &mut Vec<String>) {
            if out.len() >= 12 || a == b { return; }
            match (a, b) {
                (Value::Object(x), Value::Object(y)) if pfad.matches('/').count() < 3 => {
                    for k in x.keys().chain(y.keys().filter(|k| !x.contains_key(*k))) { diff(&x.get(k).cloned().unwrap_or(Value::Null), &y.get(k).cloned().unwrap_or(Value::Null), &format!("{pfad}/{k}"), out); }
                }
                _ => out.push(pfad.to_string()),
            }
        }
        let mut out = Vec::new();
        diff(&serde_json::to_value(&self.auswahl).unwrap_or(Value::Null), &serde_json::to_value(&self.gespeichert).unwrap_or(Value::Null), "", &mut out);
        out
    }
    pub fn bausteine_geaendert(&self) -> bool { !self.auswahl.gebaut_aktuell() }

    /// Einstellungen speichern und – wenn möglich – sofort im laufenden Harness wirken lassen.
    pub fn einstellungen_speichern(&self) -> Result<(), String> {
        let mut disk=self.auswahl.clone();
        if let Some(s)=&self.online_server {disk.server=s.clone();}
        katalog::speichern(&disk)
    }
    pub fn uebernehmen(&mut self) {
        if let Err(e) = self.auswahl.pruefen() { self.meldung = Some((e, true)); return; }
        if let Err(e) = self.einstellungen_speichern() { self.meldung = Some((e, true)); return; }
        if let Err(e) = self.auswahl.seelen_schreiben() { self.meldung = Some((e, true)); return; }
        self.gespeichert = self.auswahl.clone();
        if self.bausteine_geaendert() {
            self.meldung = Some(("Gespeichert. Die Bausteine haben sich geändert – in der Werkstatt neu bauen, dann gelten sie.".into(), false));
            return;
        }
        let pfad = self.auswahl.ordner().join("harness.json");
        if let Err(e) = std::fs::write(&pfad, serde_json::to_string_pretty(&self.auswahl.harness_json()).unwrap()) { self.meldung = Some((format!("{}: {e}", pfad.display()), true)); return; }
        if self.laeuft() { self.befehl(json!({"befehl":"konfig_neu"})); self.meldung = Some(("Übernommen – der Rat arbeitet ab dem nächsten Zug mit den neuen Einstellungen.".into(), false)); }
        else { self.meldung = Some(("Gespeichert – gilt beim nächsten Start.".into(), false)); }
    }
    pub fn verwerfen(&mut self) { let server=self.auswahl.server.clone(); self.auswahl = self.gespeichert.clone(); if self.online_server.is_some() {self.auswahl.server=server;} self.meldung = None; }

    pub fn abh_neu(&mut self) {
        self.abh_laedt = true;
        let (a, k) = (self.auswahl.clone(), self.openrouter.clone());
        self.hintergrund(move || Ergebnis::Abhaengigkeiten(abhaengigkeiten::ermitteln(&a, &k)));
    }
    pub fn lets_go(&mut self) {
        if let Err(e) = self.auswahl.pruefen() { self.meldung = Some((e, true)); return; }
        let _ = self.einstellungen_speichern();
        let _ = self.auswahl.seelen_schreiben();
        self.gespeichert = self.auswahl.clone();
        self.bau_schritte = bau::SCHRITTE.iter().map(|_| (BauStand::Wartet, String::new(), 0.0)).collect();
        self.bau_log.clear(); self.pruefungen.clear(); self.bau_ende = None;
        self.bau_rx = Some(bau::starten(self.auswahl.clone(), self.openrouter.clone(), self.modellprobe));
    }
    pub fn bau_laeuft(&self) -> bool { self.bau_rx.is_some() && self.bau_ende.is_none() }
    /// Tresorname des Harness-Schlüssels: online je Server, lokal je Welt (der Singleplayer
    /// startet auf einem Zufallsport).
    pub fn harness_tresor(&self) -> String {
        match (&self.online_server, &self.sitzung) {
            (Some(_), Some(s)) if !s.world_id.is_empty() => crate::tresor::harness_lokal(&s.world_id),
            _ => crate::tresor::harness_name(&self.auswahl.server),
        }
    }
    pub fn harness_schluessel(&self) -> Option<String> { crate::tresor::lesen(&self.harness_tresor()) }
    pub fn schluessel_vergessen(&mut self) {
        crate::tresor::loeschen(&self.harness_tresor());
        self.cockpit.alarm("Gespeicherter Harness-Schlüssel entfernt. Beim nächsten Start wird neu angemeldet.", 0);
    }

    /// LOS: Einstellungen übernehmen, mit gespeichertem Schlüssel spielen, sonst anmelden.
    pub fn los(&mut self) {
        if self.laeuft() { return; }
        if self.geaendert() { self.uebernehmen(); }
        if !self.auswahl.gebaut_aktuell() { self.meldung = Some(("Zuerst den Harness bauen: Werkstatt, Abhängigkeiten prüfen, Let's go.".into(), true)); return; }
        let _ = self.auswahl.seelen_schreiben();
        let _ = std::fs::write(self.auswahl.ordner().join("harness.json"), serde_json::to_string_pretty(&self.auswahl.harness_json()).unwrap());
        match self.harness_schluessel() {
            Some(k) => {
                let s = self.auswahl.server.clone();
                self.cockpit.status = "prüft Schlüssel".into();
                self.hintergrund(move || Ergebnis::Gepruefter(Verbindung::neu(&s).and_then(|v| v.harness_ich(&k))));
            }
            None if self.online_server.is_some() && self.sitzung.is_some() => self.schluessel_holen(),
            None => { self.anmeldung.offen = true; self.anmeldung.fehler = None; }
        }
    }
    pub fn stopp(&mut self) {
        if let Some(l) = &mut self.laufzeit { l.stoppen(); }
        self.cockpit.status = "gestoppt".into();
    }
    pub fn harness_starten(&mut self) {
        let Some(k) = self.harness_schluessel() else { self.anmeldung.offen = true; return };
        match Laufzeit::starten(&self.auswahl.ordner(), &k, &self.openrouter, &self.wissen_geheimnisse()) {
            Ok(l) => { self.laufzeit = Some(l); self.cockpit.status = "startet".into(); self.cockpit.alarm("Der Rat tritt zusammen – Harness gestartet", 0); }
            Err(e) => self.cockpit.alarm(format!("Start fehlgeschlagen: {e}"), 2),
        }
    }
    /// Anmeldung: Das Passwort lebt nur für diesen Aufruf im Speicher und wird nie gespeichert.
    pub fn anmelden(&mut self, name: String, passwort: String, otp: String) {
        let s = self.auswahl.server.clone();
        self.anmeldung.laeuft = true; self.anmeldung.fehler = None;
        self.hintergrund(move || Ergebnis::Angemeldet(Verbindung::neu(&s).and_then(|v| { let z = v.anmelden(name.trim(), &passwort, otp.trim())?; let ich = v.ich(&z)?; Ok((z, ich)) })));
    }
    fn schluessel_holen(&mut self) {
        let (Some(z), Some(info)) = (self.sitzung.clone(), self.server_info.clone()) else { return };
        let (s, rollen, manifest, regeln) = (self.auswahl.server.clone(), self.auswahl.aktive(), self.auswahl.manifest(), info["regeln"]["version"].as_str().unwrap_or("").to_string());
        let lokal = self.online_server.is_some();
        self.hintergrund(move || Ergebnis::Schluessel(Verbindung::neu(&s).and_then(|v| if lokal { v.schluessel_ersetzen(&z, &rollen, manifest, &regeln) } else { v.schluessel(&z, &rollen, manifest, &regeln) })));
    }
    pub fn platz_belegen(&mut self) {
        let Some(z) = self.sitzung.clone() else { return };
        let (s, volk) = (self.auswahl.server.clone(), self.anmeldung.volk.clone());
        let modus = if self.auswahl.aktive().len() == 4 { "agent" } else { "gemischt" };
        self.anmeldung.laeuft = true;
        self.hintergrund(move || Ergebnis::Platz(Verbindung::neu(&s).and_then(|v| v.platz(&z, modus, &volk))));
    }
    pub fn browser_adresse(&mut self, extern_oeffnen: bool) {
        let (s, z) = (self.auswahl.server.clone(), self.sitzung.clone());
        self.hintergrund(move || Ergebnis::Adresse(Verbindung::neu(&s).map(|v| v.browser_adresse(z.as_ref())).unwrap_or(s), extern_oeffnen));
    }
    pub fn fairplay_bestaetigen(&mut self) {
        let (Some(z), s) = (self.sitzung.clone(), self.auswahl.server.clone()) else { return };
        self.hintergrund(move || Ergebnis::Fairplay(Verbindung::neu(&s).and_then(|v| v.fairplay_bestaetigen(&z))));
    }
    pub fn befehl(&mut self, b: Value) {
        match &mut self.laufzeit {
            Some(l) => { if let Err(e) = l.befehl(b) { self.cockpit.alarm(e, 2); } }
            None => self.meldung = Some(("Der Rat tagt nicht – erst ▶ LOS drücken.".into(), true)),
        }
    }
    pub fn wartend_entscheiden(&mut self, id: &str, ja: bool) {
        self.befehl(json!({"befehl":if ja { "bestaetigen" } else { "ablehnen" },"id":id}));
        self.cockpit.wartend.retain(|w| w["id"] != id);
    }

    /// Schlüssel der Wissensquellen aus dem Tresor (nur für Quellen, die einen brauchen).
    pub fn wissen_geheimnisse(&self) -> BTreeMap<String, String> {
        self.auswahl.wissen.iter().filter(|w| w.mit_schluessel).filter_map(|w| crate::tresor::lesen(&crate::tresor::wissen_name(&w.name)).map(|s| (w.name.clone(), s))).collect()
    }
    /// Eine Wissensquelle abfragen: über den laufenden Harness oder – wenn er nicht läuft –
    /// über das gebaute Harness-Programm (gleicher Code, gleiche Schutzregeln).
    pub fn wissen_abfragen(&mut self, quelle: &str, frage: &str, werkzeuge: bool) {
        if frage.trim().is_empty() && !werkzeuge { self.meldung = Some(("Frage oder SQL-Abfrage eingeben".into(), true)); return; }
        self.cockpit.wissen_laeuft = true;
        let id = format!("cockpit-{}", (self.zeit * 1000.0) as u64);
        if self.laeuft() {
            self.befehl(if werkzeuge { json!({"befehl":"wissen_werkzeuge","quelle":quelle}) } else { json!({"befehl":"wissen","quelle":quelle,"frage":frage,"id":id}) });
            return;
        }
        let ordner = self.auswahl.ordner();
        let exe = bau::harness_exe(&ordner);
        if !exe.exists() { self.cockpit.wissen_laeuft = false; self.meldung = Some(("Zum Abfragen ohne laufenden Harness muss er gebaut sein (Werkstatt).".into(), true)); return; }
        if let Err(e) = self.auswahl.pruefen() { self.cockpit.wissen_laeuft = false; self.meldung = Some((e, true)); return; }
        let pfad = ordner.join("harness-abfrage.json");
        if let Err(e) = std::fs::write(&pfad, serde_json::to_string_pretty(&self.auswahl.harness_json()).unwrap()) { self.cockpit.wissen_laeuft = false; self.meldung = Some((e.to_string(), true)); return; }
        let geheim = json!({"wissen":self.wissen_geheimnisse()}).to_string();
        let (q, f) = (quelle.to_string(), frage.to_string());
        self.hintergrund(move || {
            use std::io::Write;
            let mut k = bau::kommando(&exe);
            k.arg(if werkzeuge { "wissen-werkzeuge" } else { "wissen" }).arg("--konfig").arg(&pfad).arg("--quelle").arg(&q).arg("--geheimnisse-stdin");
            if !werkzeuge { k.arg("--frage").arg(&f); }
            let r = k.current_dir(pfad.parent().unwrap()).stdin(std::process::Stdio::piped()).stdout(std::process::Stdio::piped()).stderr(std::process::Stdio::piped()).spawn()
                .and_then(|mut c| { if let Some(mut s) = c.stdin.take() { let _ = writeln!(s, "{geheim}"); } c.wait_with_output() });
            let v = match r {
                Ok(o) => String::from_utf8_lossy(&o.stdout).lines().rev().find_map(|z| serde_json::from_str::<Value>(z).ok().filter(|v| v.get("quelle").is_some() || v["art"] == "abbruch"))
                    .map(|v| if v["art"] == "abbruch" { json!({"ok":false,"quelle":q,"fehler":v["text"]}) } else { v })
                    .unwrap_or_else(|| json!({"ok":false,"quelle":q,"fehler":String::from_utf8_lossy(&o.stderr).chars().take(400).collect::<String>()})),
                Err(e) => json!({"ok":false,"quelle":q,"fehler":e.to_string()}),
            };
            if werkzeuge { Ergebnis::WissenWerkzeuge(v) } else { let mut v = v; v["rolle"] = json!("besitzer"); v["id"] = json!(id); Ergebnis::Wissen(v) }
        });
    }
    /// soul.md-Dateien neu einlesen (außerhalb bearbeitet); gilt als übernommen.
    pub fn seelen_neu_laden(&mut self) {
        let neu = self.auswahl.seelen_laden();
        for r in &neu { if let (Some(g), Some(a)) = (self.gespeichert.rollen.get_mut(r), self.auswahl.rollen.get(r)) { g.seele_md = a.seele_md.clone(); } }
        self.meldung = Some((if neu.is_empty() { "Keine geänderten soul.md-Dateien gefunden.".into() } else { format!("soul.md neu geladen: {}", neu.join(", ")) }, false));
    }

    /// Ergebnisse von Hintergrundarbeiten und Harness-Ereignisse einsammeln.
    pub fn einsammeln(&mut self) {
        if self.online_server.is_some()&&self.zeit-self.hardware_poll>=60.0{self.hardware_laden();}
        if self.online_server.is_some()&&!self.lokal_steuert&&!self.lokal_info_pending&&self.zeit-self.lokal_poll>=5.0 {
            self.lokal_info_holen();
        }
        while let Ok(e) = self.rx.try_recv() {
            match e {
                Ergebnis::LokalStart(r) => {
                    self.lokal_busy = false;
                    match r { Ok((l,s)) => {
                        let url=l.url.clone(); if let Some(w)=&l.discovery_warning {self.meldung=Some((w.clone(),false));}
                        self.lokal_lan=json!(l.lan_urls.iter().map(|u|json!({"url":u,"qr":sternenepoche_server::local::qr_svg(u)})).collect::<Vec<_>>());
                        self.lokal=Some(l); self.lokal_verbunden(url,s);
                        if let Some(s)=&self.fortsetzung {if let Err(e)=s.speichern(&self.auswahl.ordner()) {self.meldung=Some((e,true));}}
                    }, Err(e) => {self.lokal_auto_resume=false;self.meldung=Some((e,true));} }
                }
                Ergebnis::LokalBeitritt(url,r) => { self.lokal_busy=false; match r {Ok(s)=>self.lokal_verbunden(url,s),Err(e)=>self.meldung=Some((e,true))} }
                Ergebnis::LokalSuche(r) => { self.lokal_busy=false; match r {Ok(v)=>self.lokal_hosts=v,Err(e)=>self.meldung=Some((e,true))} }
                Ergebnis::LokalBeendet => { self.lokal_busy = false; }
                Ergebnis::Panik(t) => {
                    self.lokal_busy = false; self.lokal_steuert=false; self.lokal_info_pending = false; self.hardware_pending = false;
                    self.abh_laedt = false; self.anmeldung.laeuft = false; self.cockpit.wissen_laeuft = false;
                    self.meldung = Some((format!("Interner Fehler in einer Hintergrundarbeit: {t} – Details in {}", self.auswahl.ordner().join("cockpit-fehler.log").display()), true));
                }
                Ergebnis::LokalGespeichert(url,r) => {self.lokal_speichert=false;if self.online_server.is_some() && url==self.auswahl.server {match r {Ok(())=>{self.lokal_gespeichert=Some(jetzt());self.meldung=Some(("Spielstand gespeichert. Du kannst weiterspielen.".into(),false));},Err(e)=>self.meldung=Some((format!("Spielstand nicht gespeichert: {e}"),true))}}},
                Ergebnis::LokalGesteuert(url,r) => {self.lokal_steuert=false;if self.online_server.is_some()&&url==self.auswahl.server {match r {Ok(v)=>self.lokal_info=v,Err(e)=>self.meldung=Some((e,true))} self.lokal_poll=self.zeit;}},
                Ergebnis::LokalInfo(url,generation,r) => {if self.online_server.is_some()&&url==self.auswahl.server {self.lokal_info_pending=false;if generation==self.lokal_info_generation && !self.lokal_steuert {match r {Ok(v)=>self.lokal_info=v,Err(e)=>self.meldung=Some((e,true))}}}},
                Ergebnis::ServerInfo(s,r) => if s==self.auswahl.server {match r { Ok(v) => { self.server_info = Some(v); self.server_fehler = None; } Err(e) => self.server_fehler = Some(e) }},
                Ergebnis::AllianzCodex(url,name,world,r) => if url==self.auswahl.server && self.sitzung.as_ref().is_some_and(|s|s.name==name && s.world_id==world) {
                    self.codex_allianzen=match r {Ok(v)=>json!({"url":url,"name":name,"world_id":world,"allianzen":v}),Err(e)=>json!({"url":url,"name":name,"world_id":world,"fehler":e})};
                },
                Ergebnis::Ollama(r) => match r {
                    Ok(v) => {
                        self.ollama = v; self.ollama_fehler = None;
                        // Voreinstellung an das eigene Ollama anpassen: Rollen mit dem Standardmodell, das hier
                        // nicht installiert ist, bekommen das passende installierte Modell (nie eine eigene Wahl).
                        let vram = self.gpu["vram_mb"].as_u64();
                        if let Some(e) = abhaengigkeiten::empfehlung(&self.ollama, vram) {
                            let mut geaendert = Vec::new();
                            for (r, w) in self.auswahl.rollen.iter_mut() {
                                if w.anbieter == katalog::Anbieter::Ollama && w.modell == katalog::STANDARD_MODELL && abhaengigkeiten::modell_da(&self.ollama, &w.modell).is_none() { w.modell = e.clone(); geaendert.push(r.clone()); }
                            }
                            if !geaendert.is_empty() { self.meldung = Some((format!("Modell an dein Ollama angepasst: {e} (installiert) statt {}", katalog::STANDARD_MODELL), false)); }
                        }
                    }
                    Err(e) => self.ollama_fehler = Some(e),
                },
                Ergebnis::HardwareAktuell(hw,gpu) => {self.hardware=hw;self.gpu=gpu;self.hardware_pending=false;},
                Ergebnis::Abhaengigkeiten(v) => { self.abh = v; self.abh_laedt = false; }
                Ergebnis::Gepruefter(r) => match r {
                    Ok(_) => self.harness_starten(),
                    Err(e) => {
                        crate::tresor::loeschen(&self.harness_tresor());
                        if self.online_server.is_some() && self.sitzung.is_some() { self.cockpit.alarm(format!("Gespeicherter Schlüssel ungültig ({e}); die lokale Welt stellt einen neuen aus"), 1); self.schluessel_holen(); }
                        else { self.cockpit.alarm(format!("Gespeicherter Schlüssel ungültig ({e}); bitte neu anmelden"), 1); self.anmeldung.offen = true; }
                    }
                },
                Ergebnis::Testpilot(r) => {self.anmeldung.laeuft=false;match r {Ok((s,ich,url))=>{self.sitzung=Some(s);self.cockpit.konto=ich;self.browser_ziel=Some(url);self.anmeldung.offen=false;self.meldung=Some(("Testpilot angemeldet. Dein bestehendes Reich bleibt erhalten.".into(),false));},Err(e)=>self.meldung=Some((e,true))}},
                Ergebnis::Angemeldet(r) => {
                    self.anmeldung.laeuft = false;
                    match r {
                        Err(e) => self.anmeldung.fehler = Some(e),
                        Ok((z, ich)) => {
                            self.sitzung = Some(z);
                            if ich["fairplay"]["bestaetigung_noetig"] == true { self.fairplay_text = Some(ich["fairplay"]["text"].as_str().unwrap_or("").to_string()); }
                            else if ich["spieler"].is_null() { self.anmeldung.platz_noetig = true; self.anmeldung.warteliste = ich["warteliste"]["position"].as_u64(); }
                            else if ich["mode"] == "mensch" { self.platz_belegen(); }
                            else { self.schluessel_holen(); }
                        }
                    }
                }
                Ergebnis::Platz(r) => {
                    self.anmeldung.laeuft = false;
                    match r {
                        Ok(v) if !v["spieler"].is_null() => { self.anmeldung.platz_noetig = false; self.schluessel_holen(); }
                        Ok(v) => { self.anmeldung.warteliste = v["warteliste"]["position"].as_u64(); self.anmeldung.fehler = Some(format!("Warteliste Platz {}. Der Betreiber gibt Plätze der Reihe nach frei.", v["warteliste"]["position"])); }
                        Err(e) => self.anmeldung.fehler = Some(e),
                    }
                }
                Ergebnis::Schluessel(r) => match r {
                    Ok(k) => match crate::tresor::schreiben(&self.harness_tresor(), &k) {
                        Ok(()) => { self.anmeldung = Anmeldung { volk: self.anmeldung.volk.clone(), ..Default::default() }; self.harness_starten(); }
                        Err(e) => self.anmeldung.fehler = Some(e),
                    },
                    Err(e) => self.anmeldung.fehler = Some(e),
                },
                Ergebnis::Adresse(url, extern_oeffnen) => {
                    if extern_oeffnen { crate::oeffnen::browser(&url); self.minimieren = true; }
                    else { self.browser_ziel = Some(url); }
                }
                Ergebnis::Fairplay(r) => match r { Ok(_) => { self.fairplay_text = None; self.schluessel_holen(); } Err(e) => self.meldung = Some((e, true)) },
                Ergebnis::Regeln(s,r) => if s==self.auswahl.server {match r { Ok(v) => { self.katalog_regeln = Some(v); self.katalog_fehler = None; } Err(e) => self.katalog_fehler = Some(e) }},
                Ergebnis::Wissen(v) => { self.cockpit.wissen_laeuft = false; self.cockpit.wissen.push_front(v); self.cockpit.wissen.truncate(40); }
                Ergebnis::WissenWerkzeuge(v) => { self.cockpit.wissen_laeuft = false; let q = v["quelle"].as_str().unwrap_or("").to_string(); self.cockpit.mcp_werkzeuge.insert(q, v); }
            }
        }
        if let Some(rx) = &self.bau_rx {
            while let Ok(m) = rx.try_recv() {
                match m {
                    Meldung::Schritt(i, s, t) => { if let Some(x) = self.bau_schritte.get_mut(i) { x.0 = s; if !t.is_empty() { x.1 = t; } if s == BauStand::Fertig { x.2 = 1.0; } } }
                    Meldung::Fortschritt(i, f) => { if let Some(x) = self.bau_schritte.get_mut(i) { x.2 = f; } }
                    Meldung::Log(t) => { self.bau_log.push_back(t); while self.bau_log.len() > 600 { self.bau_log.pop_front(); } }
                    Meldung::Pruefung(v) => self.pruefungen.push(v),
                    Meldung::Ende(r) => {
                        if let Ok(k) = &r {
                            self.auswahl.gebaut = Some(k.clone()); self.gespeichert.gebaut = Some(k.clone()); let _ = self.einstellungen_speichern();
                            if self.meldung.as_ref().is_some_and(|(m, _)| m.contains("neu bauen") || m.contains("Zuerst den Harness bauen")) { self.meldung = Some(("Gebaut und geprüft – mit ▶ LOS tritt der Rat zusammen.".into(), false)); }
                        }
                        if r.is_err() { for x in &mut self.bau_schritte { if x.0 == BauStand::Laeuft { x.0 = BauStand::Fehler; } } }
                        self.bau_ende = Some(r);
                    }
                }
            }
        }
        if let Some(l) = &mut self.laufzeit {
            let mut neu = Vec::new();
            while let Ok(e) = l.ereignisse.try_recv() { neu.push(e); }
            for e in &neu { self.cockpit.verarbeiten(e); }
            if !l.laeuft() && self.cockpit.status != "gestoppt" && self.cockpit.status != "beendet" { self.cockpit.status = "gestoppt".into(); }
        }
    }

    pub fn abh_blockiert(&self) -> bool { self.abh_laedt || self.abh.is_empty() || self.abh.iter().any(|a| a.blockiert && !(a.name == "OpenRouter-Schlüssel" && !self.openrouter.is_empty())) }
    pub fn abh_stand(s: AbhStand) -> &'static str {
        match s { AbhStand::Vorhanden => "vorhanden", AbhStand::WirdGeladen => "laden", AbhStand::Manuell => "manuell", AbhStand::Fehlt => "fehlt" }
    }
}
