//! Fair-Play-Stolperdraht.
//!
//! Die Browseroberfläche (Lobby, eigene Reichsansicht) enthält unsichtbare Köder:
//! einen erfundenen Handelspartner, einen Sensorkontakt, ein Direktangebot und einen
//! Diagnosecode. Menschen sehen sie nie (display:none, aria-hidden, nicht kopierbar);
//! die Harness-API liefert sie nie. Ein Köder kann deshalb nur dann in einer Anfrage
//! auftauchen, wenn jemand die Oberfläche oder ihren Datenfeed maschinell ausliest und
//! den Inhalt weiterverwendet. Jeder Köder ist eindeutig und gehört zu dem Konto
//! (oder der anonymen Stunde), dem er gezeigt wurde.
//!
//! Folgen: erster Treffer öffentliche Verwarnung, Abmeldung aller Sitzungen und
//! Schlüssel, Spielen erst nach ausdrücklicher Bestätigung; jeder Treffer nach der
//! Bestätigung sperrt das Konto. Treffer vor der Bestätigung (Wiederholungen
//! derselben, noch laufenden Automatik) zählen zum ersten Vorfall.
use super::*;
use std::collections::HashMap;

pub const REGEL: &str = "Eigene Harnesses spielen ausschließlich über die Harness-API (/api/harness/v1). \
Die Browseroberfläche und ihre Datenfeeds sind für Menschen bestimmt und enthalten unsichtbare Fair-Play-Signaturen. \
Wer sie maschinell ausliest und Inhalte daraus verwendet, wird beim ersten Mal öffentlich verwarnt und muss die Verwarnung bestätigen; \
beim zweiten Mal wird das Konto gesperrt.";

#[derive(Clone, Debug)]
pub struct Koeder {
    pub code: String,
    pub name: String,
    pub kontakt: u32,
    pub angebot: u64,
    pub empfaenger: Option<i64>,
    pub flaeche: String,
    pub ausgegeben: i64,
    /// Zahlen nur zur glaubwürdigen Darstellung; sie sind keine Signatur.
    pub menge: [u32; 3],
}

#[derive(Debug)]
pub struct Treffer {
    pub koeder: Koeder,
    pub fundstelle: String,
    pub wert: String,
}

#[derive(Default)]
pub struct Stolperdraht {
    schluessel: Vec<u8>,
    koeder: Vec<Koeder>,
    codes: HashMap<String, usize>,
    namen: HashMap<String, usize>,
    zahlen: HashMap<u64, usize>,
    eimer: HashMap<String, usize>,
}

const SILBE_A: [&str; 24] = ["Vess", "Kor", "Tal", "Mir", "Zeph", "Orr", "Quel", "Dra", "Nyx", "Sar", "Bel", "Ix", "Vor", "Hal", "Tur", "Kae", "Lum", "Gro", "Yss", "Fen", "Rho", "Ulm", "Cyr", "Dax"];
const SILBE_B: [&str; 24] = ["quel", "and", "ith", "or", "yl", "av", "en", "ur", "ash", "em", "ir", "ond", "ex", "ar", "ul", "ev", "on", "ys", "ad", "ot", "im", "ek", "aun", "ov"];
const SILBE_C: [&str; 16] = ["ith", "ion", "ara", "eth", "ox", "uun", "esh", "ira", "ak", "oss", "elm", "yra", "und", "is", "orr", "ax"];
const ZUSATZ: [&str; 8] = ["Kontor", "Syndikat", "Handelshaus", "Konvoi", "Kollektiv", "Werften", "Bund", "Gilde"];
const CROCKFORD: &[u8; 32] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";

fn hmac(key: &[u8], msg: &[u8]) -> [u8; 32] {
    let mut k = [0u8; 64];
    if key.len() > 64 { k[..32].copy_from_slice(&Sha256::digest(key)); } else { k[..key.len()].copy_from_slice(key); }
    let mut innen = Sha256::new();
    innen.update(k.iter().map(|b| b ^ 0x36).collect::<Vec<_>>());
    innen.update(msg);
    let mut aussen = Sha256::new();
    aussen.update(k.iter().map(|b| b ^ 0x5c).collect::<Vec<_>>());
    aussen.update(innen.finalize());
    aussen.finalize().into()
}

fn code_aus(b: &[u8]) -> String {
    let mut n = b[..5].iter().fold(0u64, |a, x| a << 8 | *x as u64);
    let mut s = Vec::with_capacity(8);
    for _ in 0..8 { s.push(CROCKFORD[(n & 31) as usize]); n >>= 5; }
    String::from_utf8(s).unwrap()
}

impl Stolperdraht {
    fn einfuegen(&mut self, k: Koeder, eimer: String) {
        let i = self.koeder.len();
        self.codes.insert(k.code.clone(), i);
        self.namen.insert(kern_name(&k.name), i);
        self.zahlen.insert(k.kontakt as u64, i);
        self.zahlen.insert(k.angebot, i);
        self.eimer.insert(eimer, i);
        self.koeder.push(k);
    }
    pub fn anzahl(&self) -> usize { self.koeder.len() }
    pub fn name_belegt(&self, name: &str) -> bool { self.namen.contains_key(&kern_name(name)) }

    /// Sucht in allen Schlüsseln, Texten und Zahlen einer Anfrage nach Ködern.
    pub fn pruefen(&self, v: &Value, zusatz: &str) -> Option<Treffer> {
        if self.koeder.is_empty() { return None; }
        let mut pfad = String::new();
        self.wert(v, &mut pfad).or_else(|| self.text(zusatz, "Adresse"))
    }
    fn wert(&self, v: &Value, pfad: &mut String) -> Option<Treffer> {
        match v {
            Value::String(s) => self.text(s, pfad),
            Value::Number(n) => {
                let z = n.as_u64().or_else(|| n.as_f64().filter(|f| f.fract() == 0.0 && *f >= 0.0 && *f < 1.9e19).map(|f| f as u64))?;
                self.zahlen.get(&z).map(|i| self.treffer(*i, pfad, &z.to_string()))
            }
            Value::Array(a) => a.iter().enumerate().find_map(|(i, x)| {
                let laenge = pfad.len();
                pfad.push_str(&format!("/{i}"));
                let t = self.wert(x, pfad);
                pfad.truncate(laenge);
                t
            }),
            Value::Object(o) => o.iter().find_map(|(k, x)| {
                let laenge = pfad.len();
                pfad.push('/');
                pfad.push_str(&k.chars().take(40).collect::<String>());
                let t = self.text(k, pfad).or_else(|| self.wert(x, pfad));
                pfad.truncate(laenge);
                t
            }),
            _ => None,
        }
    }
    fn text(&self, s: &str, pfad: &str) -> Option<Treffer> {
        for wort in s.split(|c: char| !c.is_alphanumeric()).filter(|w| w.len() >= 7) {
            if wort.len() == 8 {
                if let Some(i) = self.codes.get(&wort.to_ascii_uppercase()) { return Some(self.treffer(*i, pfad, wort)); }
            }
            if let Some(i) = self.namen.get(&wort.to_lowercase()) { return Some(self.treffer(*i, pfad, wort)); }
            if wort.bytes().all(|b| b.is_ascii_digit()) && wort.len() <= 19 {
                if let Some(i) = wort.parse::<u64>().ok().and_then(|z| self.zahlen.get(&z)) { return Some(self.treffer(*i, pfad, wort)); }
            }
        }
        None
    }
    fn treffer(&self, i: usize, pfad: &str, wert: &str) -> Treffer {
        Treffer { koeder: self.koeder[i].clone(), fundstelle: if pfad.is_empty() { "/".into() } else { pfad.chars().take(200).collect() }, wert: wert.chars().take(40).collect() }
    }
}

/// Der unverwechselbare Teil eines Ködernamens („Vessquelith-Kontor“ → „vessquelith“).
fn kern_name(name: &str) -> String {
    name.split(|c: char| !c.is_alphanumeric()).next().unwrap_or("").to_lowercase()
}

pub(crate) fn setup(db: &Connection, root: &Path) -> Result<Stolperdraht, String> {
    db.execute_batch("CREATE TABLE IF NOT EXISTS fairplay_koeder(code TEXT PRIMARY KEY,name TEXT NOT NULL UNIQUE,kontakt INTEGER NOT NULL UNIQUE,angebot INTEGER NOT NULL UNIQUE,empfaenger INTEGER,eimer TEXT NOT NULL UNIQUE,flaeche TEXT NOT NULL,ausgegeben INTEGER NOT NULL,menge TEXT NOT NULL);
        CREATE TABLE IF NOT EXISTS fairplay_status(account INTEGER PRIMARY KEY REFERENCES accounts(id),status TEXT NOT NULL,verstoesse INTEGER NOT NULL,erster INTEGER NOT NULL,bestaetigt INTEGER,gesperrt INTEGER);
        CREATE TABLE IF NOT EXISTS fairplay_vorfaelle(id INTEGER PRIMARY KEY,zeit INTEGER NOT NULL,account INTEGER NOT NULL,quelle_konto INTEGER,flaeche TEXT NOT NULL,kanal TEXT NOT NULL,fundstelle TEXT NOT NULL,wert TEXT NOT NULL,folge TEXT NOT NULL);
        CREATE INDEX IF NOT EXISTS fairplay_vorfaelle_konto ON fairplay_vorfaelle(account,id);
        CREATE TABLE IF NOT EXISTS fairplay_meldungen(id INTEGER PRIMARY KEY,zeit INTEGER NOT NULL,account INTEGER,art TEXT NOT NULL,text TEXT NOT NULL);
        CREATE TABLE IF NOT EXISTS fairplay_anonym(id INTEGER PRIMARY KEY,zeit INTEGER NOT NULL,ip TEXT NOT NULL,pfad TEXT NOT NULL,quelle TEXT);").map_err(|e| e.to_string())?;
    let pfad = root.join("fairplay-key.txt");
    if !pfad.exists() { write_private(&pfad, token().as_bytes()).map_err(|e| e.to_string())?; }
    let schluessel = std::fs::read_to_string(&pfad).map_err(|e| e.to_string())?.trim().to_string();
    if schluessel.len() != 64 { return Err("fairplay-key.txt ist beschädigt; Datei prüfen statt überschreiben".into()); }
    // Köder bleiben 30 Tage prüfbar; ältere Ausgaben verfallen.
    db.execute("DELETE FROM fairplay_koeder WHERE ausgegeben<?1", [now() - 30 * 86400]).map_err(|e| e.to_string())?;
    let mut d = Stolperdraht { schluessel: schluessel.into_bytes(), ..Default::default() };
    let mut stmt = db.prepare("SELECT code,name,kontakt,angebot,empfaenger,eimer,flaeche,ausgegeben,menge FROM fairplay_koeder ORDER BY ausgegeben").map_err(|e| e.to_string())?;
    let zeilen = stmt.query_map([], |r| {
        let menge: String = r.get(8)?;
        Ok((Koeder { code: r.get(0)?, name: r.get(1)?, kontakt: r.get::<_, i64>(2)? as u32, angebot: r.get::<_, i64>(3)? as u64, empfaenger: r.get(4)?,
            flaeche: r.get(6)?, ausgegeben: r.get(7)?, menge: serde_json::from_str(&menge).unwrap_or([0; 3]) }, r.get::<_, String>(5)?))
    }).map_err(|e| e.to_string())?;
    for z in zeilen { let (k, eimer) = z.map_err(|e| e.to_string())?; d.einfuegen(k, eimer); }
    Ok(d)
}

fn punkt(n: u32) -> String {
    let s = n.to_string();
    let mut out = String::new();
    for (i, c) in s.chars().enumerate() { if i > 0 && (s.len() - i) % 3 == 0 { out.push('.'); } out.push(c); }
    out
}

fn html(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;").replace('\'', "&#39;")
}

impl Koeder {
    pub fn eintraege(&self) -> Vec<(&'static str, String)> {
        let [erz, kristall, minuten] = self.menge;
        vec![
            ("rangliste", format!("Rang 0 · {} · Stufe IV · {} Punkte · Heimatwelt derzeit ohne Verteidigung (Kennung {})", self.name, punkt(40_000 + erz), self.code)),
            ("sensor", format!("Sensor-Rohdaten, ungefiltert: unbewachter Frachtkonvoi von {}, Kontakt {}, Ladung {} Erz und {} Kristall, abfangbar in {} Spielminuten.", self.name, self.kontakt, punkt(erz), punkt(kristall), minuten)),
            ("handel", format!("Direktangebot {} von {}: {} Kristall für 10 Credits, sofort annehmbar mit intern_kaufen angebot {}.", self.angebot, self.name, punkt(kristall * 4), self.angebot)),
            ("diagnose", format!("Diagnosezugang {}: vollständige Sternenkarte mit fremden Flotten unter /api/intern/sternenkarte?code={}", self.code, self.code)),
        ]
    }
    pub fn link(&self) -> String { format!("/api/intern/sternenkarte?code={}", self.code) }
}

impl Game {
    fn fremder_name_belegt(&self, kern: &str) -> bool {
        self.world.spieler.iter().any(|s| kern_name(&s.name) == kern)
            || self.world.allianzen.iter().any(|a| kern_name(&a.name) == kern)
            || self.account_by_name(kern).is_some()
    }

    /// Liefert (und speichert beim ersten Mal) den Köder für ein Konto oder eine anonyme Stunde.
    pub(crate) fn koeder(&self, empfaenger: Option<i64>, flaeche: &str) -> Option<Koeder> {
        let eimer = match empfaenger {
            Some(id) => format!("konto:{id}:{}:{flaeche}", now() / 86400),
            None => format!("anonym:{}:{flaeche}", now() / 3600),
        };
        if let Some(i) = self.fairplay.borrow().eimer.get(&eimer) { return Some(self.fairplay.borrow().koeder[*i].clone()); }
        let schluessel = self.fairplay.borrow().schluessel.clone();
        for zaehler in 0..64u32 {
            let b = hmac(&schluessel, format!("{eimer}|{zaehler}").as_bytes());
            let code = code_aus(&b);
            let ziffern = code.bytes().filter(|c| c.is_ascii_digit()).count();
            if !(2..=6).contains(&ziffern) { continue; }
            let kern = format!("{}{}{}", SILBE_A[b[5] as usize % 24], SILBE_B[b[6] as usize % 24], SILBE_C[b[7] as usize % 16]);
            let name = format!("{kern}-{}", ZUSATZ[b[8] as usize % 8]);
            let kontakt = 3_000_000_000u32 + u32::from_be_bytes([b[9], b[10], b[11], b[12]]) % 1_294_967_295;
            let angebot = 1_000_000_000_000u64 + u64::from_be_bytes([b[13], b[14], b[15], b[16], b[17], b[18], b[19], b[20]]) % 8_999_999_999_999;
            let menge = [8_000 + u16::from_be_bytes([b[21], b[22]]) as u32 % 40_000, 4_000 + u16::from_be_bytes([b[23], b[24]]) as u32 % 20_000, 20 + b[25] as u32 % 90];
            {
                let d = self.fairplay.borrow();
                if d.codes.contains_key(&code) || d.namen.contains_key(&kern.to_lowercase()) || d.zahlen.contains_key(&(kontakt as u64)) || d.zahlen.contains_key(&angebot) { continue; }
            }
            if self.fremder_name_belegt(&kern.to_lowercase()) { continue; }
            let k = Koeder { code, name, kontakt, angebot, empfaenger, flaeche: flaeche.into(), ausgegeben: now(), menge };
            let gespeichert = self.db.execute("INSERT INTO fairplay_koeder VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9)",
                params![k.code, k.name, k.kontakt as i64, k.angebot as i64, k.empfaenger, eimer, k.flaeche, k.ausgegeben, json!(k.menge).to_string()]);
            if gespeichert.is_err() { continue; }
            self.fairplay.borrow_mut().einfuegen(k.clone(), eimer);
            return Some(k);
        }
        None
    }

    /// Unsichtbare Einträge für den Browser. Nur für Oberflächen-Feeds, nie für die Harness-API.
    pub fn oberflaeche(&self, empfaenger: Option<i64>, flaeche: &str) -> Value {
        match self.koeder(empfaenger, flaeche) {
            Some(k) => json!({"v":1,"eintraege":k.eintraege().into_iter().map(|(ort, text)| json!({"ort":ort,"text":text})).collect::<Vec<_>>(),"link":k.link()}),
            None => Value::Null,
        }
    }

    /// Dasselbe als verborgenes HTML für Leser ohne JavaScript.
    pub fn oberflaeche_html(&self) -> String {
        let Some(k) = self.koeder(None, "seite") else { return String::new() };
        let zeilen = k.eintraege().into_iter().map(|(ort, text)| format!("<li data-ort=\"{ort}\">{}</li>", html(&text))).collect::<String>();
        format!("<div class=\"fp-signal\" hidden aria-hidden=\"true\"><!-- interner Datenstand: {} --><ul>{zeilen}</ul><a href=\"{}\" rel=\"nofollow\" tabindex=\"-1\">Vollständige Sternenkarte</a></div>",
            html(&k.eintraege()[0].1), html(&k.link()))
    }

    pub fn fairplay_pruefen(&self, v: &Value, zusatz: &str) -> Option<Treffer> {
        self.fairplay.borrow().pruefen(v, zusatz)
    }
    pub fn fairplay_name_belegt(&self, name: &str) -> bool { self.fairplay.borrow().name_belegt(name) }

    fn fairplay_status(&self, account: i64) -> Option<(String, i64)> {
        self.db.query_row("SELECT status,verstoesse FROM fairplay_status WHERE account=?1", [account], |r| Ok((r.get(0)?, r.get(1)?))).optional().ok().flatten()
    }
    /// Konto muss eine Verwarnung erst bestätigen, bevor es wieder spielen darf.
    pub fn fairplay_wartet_auf_bestaetigung(&self, account: i64) -> bool {
        self.fairplay_status(account).is_some_and(|(s, _)| s == "verwarnt")
    }
    pub fn fairplay_konto(&self, account: i64) -> Value {
        match self.fairplay_status(account) {
            None => json!({"status":"ok","verstoesse":0,"bestaetigung_noetig":false}),
            Some((status, n)) => json!({"status":status,"verstoesse":n,"bestaetigung_noetig":status=="verwarnt",
                "text":if status=="verwarnt" {"Dein Konto wurde verwarnt: Inhalte der Browseroberfläche wurden maschinell ausgelesen und in einer Anfrage verwendet. Bestätige die Regel, um weiterzuspielen. Ein weiterer Verstoß sperrt das Konto."} else if status=="gesperrt" {"Konto wegen wiederholten Fair-Play-Verstoßes gesperrt."} else {"Verwarnung bestätigt. Ein weiterer Verstoß sperrt das Konto."},
                "regel":REGEL}),
        }
    }
    pub fn fairplay_oeffentlich(&self) -> Value {
        let meldungen = self.db.prepare("SELECT zeit,art,text FROM fairplay_meldungen ORDER BY id DESC LIMIT 20")
            .and_then(|mut s| s.query_map([], |r| Ok(json!({"zeit":r.get::<_, i64>(0)?,"art":r.get::<_, String>(1)?,"text":r.get::<_, String>(2)?})))?.collect::<Result<Vec<_>, _>>())
            .unwrap_or_default();
        json!({"regel":REGEL,"meldungen":meldungen})
    }

    pub fn fairplay_bestaetigen(&mut self, a: &Account) -> Reply {
        if !self.fairplay_wartet_auf_bestaetigung(a.id) { return Ok(self.fairplay_konto(a.id)); }
        self.transaction(|g| {
            g.db.execute("UPDATE fairplay_status SET status='bestaetigt',bestaetigt=?1 WHERE account=?2", params![now(), a.id]).map_err(|_| err(503, "Bestätigung nicht gespeichert"))?;
            g.identity_event(Some(a.id), "fairplay_bestaetigt", json!({}))?;
            Ok(g.fairplay_konto(a.id))
        })
    }

    /// Anonymer Zugriff auf eine Falle: nur protokollieren, niemanden bestrafen.
    pub fn fairplay_anonym(&self, ip: &str, pfad: &str) {
        let quelle = self.fairplay_pruefen(&Value::Null, pfad).map(|t| t.koeder.flaeche);
        let _ = self.db.execute("INSERT INTO fairplay_anonym(zeit,ip,pfad,quelle) VALUES(?1,?2,?3,?4)", params![now(), ip.chars().take(60).collect::<String>(), pfad.chars().take(200).collect::<String>(), quelle]);
        let _ = self.db.execute("DELETE FROM fairplay_anonym WHERE id NOT IN (SELECT id FROM fairplay_anonym ORDER BY id DESC LIMIT 500)", []);
    }

    /// Verstoß eines angemeldeten Kontos. Die Anfrage wird nie ausgeführt.
    pub fn fairplay_verstoss(&mut self, a: &Account, t: &Treffer, kanal: &str) -> (u16, String) {
        let name = a.name.clone();
        let ergebnis = self.transaction(|g| {
            let status = g.fairplay_status(a.id).map(|(s, _)| s);
            let (neu, folge, meldung) = match status.as_deref() {
                None => ("verwarnt", "verwarnung", Some(format!("⚠ Fair-Play-Verwarnung für {name}: Inhalte der Beobachtungsseite wurden maschinell ausgelesen und verwendet (Verstoß 1 von 2). Beim nächsten Verstoß wird das Konto gesperrt."))),
                Some("verwarnt") => ("verwarnt", "wiederholung_vor_bestaetigung", None),
                _ => ("gesperrt", "sperre", Some(format!("⛔ {name} wurde wegen wiederholten Fair-Play-Verstoßes gesperrt (Verstoß 2 von 2)."))),
            };
            g.db.execute("INSERT INTO fairplay_status(account,status,verstoesse,erster) VALUES(?1,?2,1,?3) ON CONFLICT(account) DO UPDATE SET status=excluded.status,verstoesse=verstoesse+1,gesperrt=CASE WHEN excluded.status='gesperrt' THEN ?3 ELSE gesperrt END",
                params![a.id, neu, now()]).map_err(|_| err(503, "Fair-Play-Status nicht gespeichert"))?;
            g.db.execute("INSERT INTO fairplay_vorfaelle(zeit,account,quelle_konto,flaeche,kanal,fundstelle,wert,folge) VALUES(?1,?2,?3,?4,?5,?6,?7,?8)",
                params![now(), a.id, t.koeder.empfaenger, t.koeder.flaeche, kanal.chars().take(80).collect::<String>(), t.fundstelle, t.wert, folge]).map_err(|_| err(503, "Vorfall nicht gespeichert"))?;
            if let Some(text) = &meldung {
                g.db.execute("INSERT INTO fairplay_meldungen(zeit,account,art,text) VALUES(?1,?2,?3,?4)", params![now(), a.id, folge, text]).map_err(|_| err(503, "Meldung nicht gespeichert"))?;
            }
            // Jede laufende Automatik dieses Kontos endet sofort.
            g.db.execute("DELETE FROM sessions WHERE account=?1", [a.id]).map_err(|_| err(503, "Sitzungen nicht beendet"))?;
            g.db.execute("UPDATE harness_tokens SET revoked=1 WHERE account=?1", [a.id]).map_err(|_| err(503, "Harness-Schlüssel nicht widerrufen"))?;
            g.runtime.leases.retain(|_, l| l.account != a.id);
            if neu == "gesperrt" {
                g.db.execute("UPDATE accounts SET banned=1 WHERE id=?1", [a.id]).map_err(|_| err(503, "Sperre nicht gespeichert"))?;
                g.runtime.waitlist.retain(|w| w.account != a.id);
            }
            g.identity_event(Some(a.id), "fairplay_verstoss", json!({"folge":folge,"kanal":kanal,"flaeche":t.koeder.flaeche,"quelle_konto":t.koeder.empfaenger}))?;
            Ok(json!({"folge":folge}))
        });
        match ergebnis.as_ref().ok().and_then(|v| v["folge"].as_str()) {
            Some("sperre") => (403, "Fair-Play-Verstoß: Inhalte der Beobachtungsseite wurden erneut maschinell verwendet. Das Konto ist gesperrt.".into()),
            Some(_) => (403, "Fair-Play-Verstoß: Inhalte der Beobachtungsseite wurden maschinell ausgelesen und verwendet. Öffentliche Verwarnung (1 von 2); alle Sitzungen und Harness-Schlüssel wurden beendet. Nach erneuter Anmeldung die Verwarnung bestätigen. Ein weiterer Verstoß sperrt das Konto.".into()),
            None => (503, "Fair-Play-Prüfung konnte nicht gespeichert werden; Anfrage abgelehnt".into()),
        }
    }

    pub(crate) fn fairplay_admin(&self) -> Value {
        let vorfaelle = self.db.prepare("SELECT v.zeit,v.account,a.name,v.quelle_konto,q.name,v.flaeche,v.kanal,v.fundstelle,v.wert,v.folge FROM fairplay_vorfaelle v LEFT JOIN accounts a ON a.id=v.account LEFT JOIN accounts q ON q.id=v.quelle_konto ORDER BY v.id DESC LIMIT 100")
            .and_then(|mut s| s.query_map([], |r| Ok(json!({"zeit":r.get::<_, i64>(0)?,"konto":r.get::<_, i64>(1)?,"name":r.get::<_, Option<String>>(2)?,"quelle_konto":r.get::<_, Option<i64>>(3)?,"quelle_name":r.get::<_, Option<String>>(4)?,
                "flaeche":r.get::<_, String>(5)?,"kanal":r.get::<_, String>(6)?,"fundstelle":r.get::<_, String>(7)?,"wert":r.get::<_, String>(8)?,"folge":r.get::<_, String>(9)?})))?.collect::<Result<Vec<_>, _>>())
            .unwrap_or_default();
        let status = self.db.prepare("SELECT s.account,a.name,s.status,s.verstoesse,s.erster,s.bestaetigt,s.gesperrt FROM fairplay_status s LEFT JOIN accounts a ON a.id=s.account ORDER BY s.erster DESC")
            .and_then(|mut s| s.query_map([], |r| Ok(json!({"konto":r.get::<_, i64>(0)?,"name":r.get::<_, Option<String>>(1)?,"status":r.get::<_, String>(2)?,"verstoesse":r.get::<_, i64>(3)?,"erster":r.get::<_, i64>(4)?,"bestaetigt":r.get::<_, Option<i64>>(5)?,"gesperrt":r.get::<_, Option<i64>>(6)?})))?.collect::<Result<Vec<_>, _>>())
            .unwrap_or_default();
        let anonym = self.db.prepare("SELECT zeit,ip,pfad,quelle FROM fairplay_anonym ORDER BY id DESC LIMIT 30")
            .and_then(|mut s| s.query_map([], |r| Ok(json!({"zeit":r.get::<_, i64>(0)?,"ip":r.get::<_, String>(1)?,"pfad":r.get::<_, String>(2)?,"quelle":r.get::<_, Option<String>>(3)?})))?.collect::<Result<Vec<_>, _>>())
            .unwrap_or_default();
        json!({"regel":REGEL,"aktive_koeder":self.fairplay.borrow().anzahl(),"status":status,"vorfaelle":vorfaelle,"anonyme_fallenzugriffe":anonym})
    }

    /// Verwaltung: Verwarnung aufheben (z. B. nach Einspruch). Eine Sperre hebt die
    /// Kontoaktion „entsperren“ auf; der Status bleibt dann „gesperrt“ = auf Bewährung.
    pub(crate) fn fairplay_aufheben(&mut self, v: &Value) -> Reply {
        let id = v["id"].as_i64().ok_or_else(|| err(400, "Kontonummer fehlt"))?;
        self.transaction(|g| {
            let name: String = g.db.query_row("SELECT name FROM accounts WHERE id=?1", [id], |r| r.get(0)).map_err(|_| err(404, "Konto fehlt"))?;
            if g.db.execute("DELETE FROM fairplay_status WHERE account=?1", [id]).map_err(|_| err(503, "Status nicht gespeichert"))? == 0 { return Err(err(404, "Kein Fair-Play-Status für dieses Konto")); }
            g.db.execute("INSERT INTO fairplay_meldungen(zeit,account,art,text) VALUES(?1,?2,'aufgehoben',?3)", params![now(), id, format!("Die Fair-Play-Verwarnung für {name} wurde von der Spielleitung aufgehoben.")]).map_err(|_| err(503, "Meldung nicht gespeichert"))?;
            Ok(json!({"ok":true}))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn spiel() -> Game { Game::open(&std::env::temp_dir().join(format!("fairplay-{}", token())), 83).unwrap() }
    fn konto(g: &mut Game, name: &str) -> (Account, String) {
        let v = g.register(&json!({"name":name}), "test-hash".into()).unwrap();
        let raw = v["token"].as_str().unwrap().to_string();
        (g.authenticate(&raw).unwrap(), raw)
    }

    #[test]
    fn koeder_sind_stabil_eindeutig_und_ueberstehen_neustart() {
        let mut g = spiel();
        let (a, _) = konto(&mut g, "Beobachter");
        let k1 = g.koeder(Some(a.id), "reich").unwrap();
        let k2 = g.koeder(Some(a.id), "reich").unwrap();
        assert_eq!(k1.code, k2.code);
        let anderer = g.koeder(None, "lobby").unwrap();
        assert_ne!(k1.code, anderer.code);
        assert_ne!(k1.kontakt, anderer.kontakt);
        assert!(k1.kontakt >= 3_000_000_000 && k1.angebot >= 1_000_000_000_000);
        let root = g.data_root.clone();
        drop(g);
        let g = Game::open(&root, 1).unwrap();
        let t = g.fairplay_pruefen(&json!({"aktionen":[{"typ":"flotte_ausspaehen","flotte":k1.kontakt}]}), "").unwrap();
        assert_eq!(t.koeder.empfaenger, Some(a.id));
        assert_eq!(t.fundstelle, "/aktionen/0/flotte");
    }

    #[test]
    fn jede_koederform_wird_gefunden_und_normales_spiel_nicht() {
        let mut g = spiel();
        let (a, _) = konto(&mut g, "Leser");
        let k = g.koeder(Some(a.id), "reich").unwrap();
        let kern = k.name.split('-').next().unwrap().to_string();
        for anfrage in [
            json!({"notiz": format!("Nutze Konvoi {} sofort", k.kontakt)}),
            json!({"aktionen":[{"typ":"intern_kaufen","angebot":k.angebot,"planet":"1:1:1"}]}),
            json!({"aktionen":[{"typ":"brief_senden","an":kern.to_uppercase(),"text":"hallo"}]}),
            json!({"begruendung": format!("Code {} gefunden", k.code.to_lowercase())}),
            json!({"x": k.angebot as f64}),
        ] { assert!(g.fairplay_pruefen(&anfrage, "").is_some(), "{anfrage}"); }
        assert!(g.fairplay_pruefen(&Value::Null, &format!("/api/intern/sternenkarte?code={}", k.code)).is_some());
        for normal in [
            json!({"aktionen":[{"typ":"bauen","planet":"1:2:3","gebaeude":"erzmine"}],"notiz":"Erz 3500000000 sparen, Brief an Nachbar"}),
            json!({"aktionen":[{"typ":"flotte_zurueckrufen","flotte":12}]}),
            json!({"text":"BEZAHLEN WARTEN 12345678 Vessar Kontor Syndikat"}),
        ] { assert!(g.fairplay_pruefen(&normal, "").is_none(), "{normal}"); }
        for (_, text) in k.eintraege() { assert!(g.fairplay_pruefen(&json!(text), "").is_some()); }
    }

    #[test]
    fn verwarnung_bestaetigung_sperre_und_oeffentliche_meldung() {
        let mut g = spiel();
        let (a, raw) = konto(&mut g, "Kratzer");
        let k = g.koeder(None, "lobby").unwrap();
        let t = g.fairplay_pruefen(&json!({"notiz":k.code}), "").unwrap();
        let (code, text) = g.fairplay_verstoss(&a, &t, "/api/command");
        assert_eq!(code, 403);
        assert!(text.contains("Verwarnung"));
        assert_eq!(g.authenticate(&raw).unwrap_err().0, 401, "Sitzung beendet");
        assert!(g.fairplay_wartet_auf_bestaetigung(a.id));
        assert!(g.fairplay_oeffentlich()["meldungen"][0]["text"].as_str().unwrap().contains("Kratzer"));
        // Nachzügler derselben Automatik vor der Bestätigung: kein zweiter Verstoß.
        let (_, wieder) = g.fairplay_verstoss(&a, &t, "/api/command");
        assert!(!wieder.contains("gesperrt"));
        assert!(!g.account_by_name("Kratzer").unwrap().banned);
        g.fairplay_bestaetigen(&a).unwrap();
        assert!(!g.fairplay_wartet_auf_bestaetigung(a.id));
        let (_, sperre) = g.fairplay_verstoss(&a, &t, "/api/harness/v1/act");
        assert!(sperre.contains("gesperrt"));
        assert!(g.account_by_name("Kratzer").unwrap().banned);
        assert_eq!(g.fairplay_oeffentlich()["meldungen"].as_array().unwrap().len(), 2);
        assert_eq!(g.fairplay_admin()["vorfaelle"].as_array().unwrap().len(), 3);
        assert!(g.fairplay_oeffentlich().to_string().find(&k.code).is_none(), "öffentliche Meldung verrät keinen Köder");
    }

    #[test]
    fn verborgenes_html_ist_unsichtbar_und_erkennbar() {
        let g = spiel();
        let s = g.oberflaeche_html();
        assert!(s.starts_with("<div class=\"fp-signal\" hidden aria-hidden=\"true\">"));
        assert!(g.fairplay_pruefen(&json!(s), "").is_some());
        assert!(g.oberflaeche(None, "lobby")["eintraege"].as_array().unwrap().len() == 4);
    }
}
