//! Privates Gedächtnis des Harness. Zwei Optionen: SQLite (eingebaut) oder DuckDB
//! (offizielle Laufzeit, geprüft nachgeladen). Alle Werte werden als Parameter gebunden;
//! DuckDB läuft ohne externen Datei-/Netzzugriff und ohne Erweiterungs-Download.
use serde_json::{json, Value};
use std::path::Path;

pub trait Gedaechtnis: Send {
    fn art(&self) -> &'static str;
    fn ausfuehren(&self, sql: &str, werte: &[&str]) -> Result<(), String>;
    fn zeilen(&self, sql: &str, werte: &[&str]) -> Result<Vec<Vec<Option<String>>>, String>;
    /// Lesende Abfrage mit Spaltennamen; wird nach `sekunden` abgebrochen.
    fn lesen(&self, sql: &str, sekunden: u64) -> Result<(Vec<String>, Vec<Vec<Option<String>>>), String>;
}

/// Prüft eine frei formulierte SQL-Abfrage (Cockpit, lokale API, Modelle) und macht sie
/// unschädlich: genau eine SELECT-/WITH-Anweisung, als Unterabfrage mit Zeilengrenze
/// gekapselt. Schreibende Anweisungen sind in einer Unterabfrage gar nicht möglich; die
/// Sperrliste hält zusätzlich Datei- und Erweiterungsfunktionen fern.
pub fn sql_kapseln(sql: &str, max_zeilen: usize) -> Result<String, String> {
    let q = sql.trim().trim_end_matches(';').trim();
    if q.is_empty() || q.chars().count() > 4000 { return Err("Abfrage leer oder länger als 4000 Zeichen".into()); }
    if q.contains(';') { return Err("Nur eine Anweisung erlaubt".into()); }
    if q.contains("--") || q.contains("/*") { return Err("Kommentare sind in Abfragen nicht erlaubt".into()); }
    let klein = q.to_lowercase();
    if !(klein.starts_with("select") || klein.starts_with("with")) { return Err("Nur lesende Abfragen (SELECT oder WITH)".into()); }
    for wort in ["attach", "detach", "pragma", "load_extension", "read_", "glob(", "fts3_tokenizer", "getenv", "install ", "copy ", "export ", "import ", "writefile", "readfile", "edit(", "query_table", "sniff_csv"] {
        if klein.contains(wort) { return Err(format!("„{}“ ist in Abfragen nicht erlaubt", wort.trim())); }
    }
    Ok(format!("SELECT * FROM ({q}) AS abfrage LIMIT {}", max_zeilen.clamp(1, 1000)))
}

const SCHEMA_SQLITE: &str = "CREATE TABLE IF NOT EXISTS notizen(id INTEGER PRIMARY KEY,zeit INTEGER NOT NULL,rolle TEXT NOT NULL,art TEXT NOT NULL,text TEXT NOT NULL);
CREATE TABLE IF NOT EXISTS zuege(id INTEGER PRIMARY KEY,zeit INTEGER NOT NULL,rolle TEXT NOT NULL,request_id TEXT NOT NULL,aktionen TEXT NOT NULL,ergebnis TEXT NOT NULL,begruendung TEXT NOT NULL);
CREATE TABLE IF NOT EXISTS ereignisse(id INTEGER PRIMARY KEY,zeit INTEGER NOT NULL,art TEXT NOT NULL,daten TEXT NOT NULL);
CREATE TABLE IF NOT EXISTS aufgaben(id INTEGER PRIMARY KEY,erstellt INTEGER NOT NULL,aktualisiert INTEGER NOT NULL,titel TEXT NOT NULL,beschreibung TEXT NOT NULL,rolle TEXT NOT NULL,prioritaet INTEGER NOT NULL,status TEXT NOT NULL,von TEXT NOT NULL,notiz TEXT NOT NULL);
CREATE TABLE IF NOT EXISTS team(id INTEGER PRIMARY KEY,zeit INTEGER NOT NULL,von TEXT NOT NULL,an TEXT NOT NULL,text TEXT NOT NULL,gelesen INTEGER NOT NULL);
CREATE TABLE IF NOT EXISTS erinnerungen(id INTEGER PRIMARY KEY,zeit INTEGER NOT NULL,von TEXT NOT NULL,thema TEXT NOT NULL,text TEXT NOT NULL);
CREATE TABLE IF NOT EXISTS verlauf(id INTEGER PRIMARY KEY,zeit INTEGER NOT NULL,rolle TEXT NOT NULL,prompt TEXT NOT NULL,antwort TEXT NOT NULL);
CREATE TABLE IF NOT EXISTS inventur(id INTEGER PRIMARY KEY,zeit INTEGER NOT NULL,koord TEXT NOT NULL,erz INTEGER NOT NULL,kristall INTEGER NOT NULL,deuterium INTEGER NOT NULL,nahrung INTEGER NOT NULL,energie_saldo INTEGER NOT NULL,bevoelkerung INTEGER NOT NULL,stabilitaet INTEGER NOT NULL,bauschleife INTEGER NOT NULL,daten TEXT NOT NULL);
CREATE TABLE IF NOT EXISTS routinen(id INTEGER PRIMARY KEY,zeit INTEGER NOT NULL,name TEXT NOT NULL,art TEXT NOT NULL,ok INTEGER NOT NULL,daten TEXT NOT NULL);
CREATE TABLE IF NOT EXISTS training(id INTEGER PRIMARY KEY,zeit INTEGER NOT NULL,rolle TEXT NOT NULL,modell TEXT NOT NULL,seele TEXT NOT NULL,system TEXT NOT NULL,nutzer TEXT NOT NULL,antwort TEXT NOT NULL,angenommen INTEGER NOT NULL,abgelehnt INTEGER NOT NULL);";
const SCHEMA_DUCKDB: [&str; 20] = [
    "CREATE SEQUENCE IF NOT EXISTS s_training", "CREATE TABLE IF NOT EXISTS training(id BIGINT DEFAULT nextval('s_training'),zeit BIGINT NOT NULL,rolle VARCHAR NOT NULL,modell VARCHAR NOT NULL,seele VARCHAR NOT NULL,system VARCHAR NOT NULL,nutzer VARCHAR NOT NULL,antwort VARCHAR NOT NULL,angenommen BIGINT NOT NULL,abgelehnt BIGINT NOT NULL)",
    "CREATE SEQUENCE IF NOT EXISTS s_inventur", "CREATE TABLE IF NOT EXISTS inventur(id BIGINT DEFAULT nextval('s_inventur'),zeit BIGINT NOT NULL,koord VARCHAR NOT NULL,erz BIGINT NOT NULL,kristall BIGINT NOT NULL,deuterium BIGINT NOT NULL,nahrung BIGINT NOT NULL,energie_saldo BIGINT NOT NULL,bevoelkerung BIGINT NOT NULL,stabilitaet BIGINT NOT NULL,bauschleife BIGINT NOT NULL,daten VARCHAR NOT NULL)",
    "CREATE SEQUENCE IF NOT EXISTS s_routinen", "CREATE TABLE IF NOT EXISTS routinen(id BIGINT DEFAULT nextval('s_routinen'),zeit BIGINT NOT NULL,name VARCHAR NOT NULL,art VARCHAR NOT NULL,ok BIGINT NOT NULL,daten VARCHAR NOT NULL)",
    "CREATE SEQUENCE IF NOT EXISTS s_aufgaben", "CREATE TABLE IF NOT EXISTS aufgaben(id BIGINT DEFAULT nextval('s_aufgaben'),erstellt BIGINT NOT NULL,aktualisiert BIGINT NOT NULL,titel VARCHAR NOT NULL,beschreibung VARCHAR NOT NULL,rolle VARCHAR NOT NULL,prioritaet BIGINT NOT NULL,status VARCHAR NOT NULL,von VARCHAR NOT NULL,notiz VARCHAR NOT NULL)",
    "CREATE SEQUENCE IF NOT EXISTS s_team", "CREATE TABLE IF NOT EXISTS team(id BIGINT DEFAULT nextval('s_team'),zeit BIGINT NOT NULL,von VARCHAR NOT NULL,an VARCHAR NOT NULL,text VARCHAR NOT NULL,gelesen BIGINT NOT NULL)",
    "CREATE SEQUENCE IF NOT EXISTS s_erinnerungen", "CREATE TABLE IF NOT EXISTS erinnerungen(id BIGINT DEFAULT nextval('s_erinnerungen'),zeit BIGINT NOT NULL,von VARCHAR NOT NULL,thema VARCHAR NOT NULL,text VARCHAR NOT NULL)",
    "CREATE SEQUENCE IF NOT EXISTS s_verlauf", "CREATE TABLE IF NOT EXISTS verlauf(id BIGINT DEFAULT nextval('s_verlauf'),zeit BIGINT NOT NULL,rolle VARCHAR NOT NULL,prompt VARCHAR NOT NULL,antwort VARCHAR NOT NULL)",
    "CREATE SEQUENCE IF NOT EXISTS s_notizen", "CREATE TABLE IF NOT EXISTS notizen(id BIGINT DEFAULT nextval('s_notizen'),zeit BIGINT NOT NULL,rolle VARCHAR NOT NULL,art VARCHAR NOT NULL,text VARCHAR NOT NULL)",
    "CREATE SEQUENCE IF NOT EXISTS s_zuege", "CREATE TABLE IF NOT EXISTS zuege(id BIGINT DEFAULT nextval('s_zuege'),zeit BIGINT NOT NULL,rolle VARCHAR NOT NULL,request_id VARCHAR NOT NULL,aktionen VARCHAR NOT NULL,ergebnis VARCHAR NOT NULL,begruendung VARCHAR NOT NULL)",
    "CREATE SEQUENCE IF NOT EXISTS s_ereignisse", "CREATE TABLE IF NOT EXISTS ereignisse(id BIGINT DEFAULT nextval('s_ereignisse'),zeit BIGINT NOT NULL,art VARCHAR NOT NULL,daten VARCHAR NOT NULL)",
];

pub fn jetzt() -> i64 { std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0) }

pub fn oeffnen(art: &str, pfad: &Path, bibliothek: &Path) -> Result<Box<dyn Gedaechtnis>, String> {
    let _ = bibliothek;
    match art {
        #[cfg(feature = "sqlite")]
        "sqlite" => Ok(Box::new(sqlite::Sqlite::oeffnen(pfad)?)),
        #[cfg(feature = "duckdb")]
        "duckdb" => Ok(Box::new(duckdb::DuckDb::oeffnen(pfad, bibliothek)?)),
        andere => Err(format!("Gedächtnis {andere} nicht eingebaut")),
    }
}

/// Gemeinsame, datenbankneutrale Operationen.
pub struct Speicher { pub db: Box<dyn Gedaechtnis> }
impl Speicher {
    pub fn notiz(&self, rolle: &str, art: &str, text: &str) -> Result<(), String> {
        self.db.ausfuehren("INSERT INTO notizen(zeit,rolle,art,text) VALUES(CAST(? AS BIGINT),?,?,?)", &[&jetzt().to_string(), rolle, art, &text.chars().take(4000).collect::<String>()])
    }
    pub fn notizen(&self, rolle: &str, n: usize) -> Result<Vec<Value>, String> {
        let z = self.db.zeilen("SELECT zeit,art,text FROM notizen WHERE rolle=? ORDER BY id DESC LIMIT CAST(? AS BIGINT)", &[rolle, &n.to_string()])?;
        Ok(z.into_iter().map(|r| json!({"zeit":r[0],"art":r[1],"text":r[2]})).collect())
    }
    pub fn zug(&self, rolle: &str, request_id: &str, aktionen: &Value, ergebnis: &Value, begruendung: &str) -> Result<(), String> {
        self.db.ausfuehren("INSERT INTO zuege(zeit,rolle,request_id,aktionen,ergebnis,begruendung) VALUES(CAST(? AS BIGINT),?,?,?,?,?)",
            &[&jetzt().to_string(), rolle, request_id, &aktionen.to_string(), &ergebnis.to_string(), &begruendung.chars().take(1500).collect::<String>()])
    }
    pub fn letzte_zuege(&self, rolle: &str, n: usize) -> Result<Vec<Value>, String> {
        let z = self.db.zeilen("SELECT zeit,aktionen,ergebnis,begruendung FROM zuege WHERE rolle=? ORDER BY id DESC LIMIT CAST(? AS BIGINT)", &[rolle, &n.to_string()])?;
        Ok(z.into_iter().map(|r| {
            let lies = |s: &Option<String>| s.as_deref().and_then(|t| serde_json::from_str::<Value>(t).ok()).unwrap_or(Value::Null);
            json!({"zeit":r[0],"aktionen":lies(&r[1]),"ergebnis":lies(&r[2]),"begruendung":r[3]})
        }).collect())
    }
    pub fn ereignis(&self, art: &str, daten: &Value) -> Result<(), String> {
        self.db.ausfuehren("INSERT INTO ereignisse(zeit,art,daten) VALUES(CAST(? AS BIGINT),?,?)", &[&jetzt().to_string(), art, &daten.to_string()])
    }
    fn zahl(&self, sql: &str, werte: &[&str]) -> Result<i64, String> {
        Ok(self.db.zeilen(sql, werte)?.first().and_then(|r| r[0].as_deref()).and_then(|s| s.parse().ok()).unwrap_or(0))
    }
    fn kurz(s: &str, n: usize) -> String { s.chars().filter(|c| !c.is_control() || *c == '\n').take(n).collect() }

    // ---- Planung: gemeinsame Aufgabenliste der Rollen und des Besitzers
    pub fn aufgabe_neu(&self, titel: &str, beschreibung: &str, rolle: &str, prioritaet: i64, von: &str) -> Result<i64, String> {
        let t = jetzt().to_string();
        self.db.ausfuehren("INSERT INTO aufgaben(erstellt,aktualisiert,titel,beschreibung,rolle,prioritaet,status,von,notiz) VALUES(CAST(? AS BIGINT),CAST(? AS BIGINT),?,?,?,CAST(? AS BIGINT),'offen',?,'')",
            &[&t, &t, &Self::kurz(titel, 120), &Self::kurz(beschreibung, 800), rolle, &prioritaet.clamp(1, 3).to_string(), von])?;
        self.zahl("SELECT max(id) FROM aufgaben", &[])
    }
    pub fn aufgabe_aendern(&self, id: i64, status: Option<&str>, notiz: Option<&str>) -> Result<(), String> {
        if self.zahl("SELECT count(*) FROM aufgaben WHERE id=CAST(? AS BIGINT)", &[&id.to_string()])? == 0 { return Err(format!("Aufgabe {id} unbekannt")); }
        if let Some(st) = status {
            if !["offen", "aktiv", "erledigt", "blockiert"].contains(&st) { return Err("Status offen, aktiv, erledigt oder blockiert".into()); }
            self.db.ausfuehren("UPDATE aufgaben SET status=?,aktualisiert=CAST(? AS BIGINT) WHERE id=CAST(? AS BIGINT)", &[st, &jetzt().to_string(), &id.to_string()])?;
        }
        if let Some(n) = notiz { self.db.ausfuehren("UPDATE aufgaben SET notiz=?,aktualisiert=CAST(? AS BIGINT) WHERE id=CAST(? AS BIGINT)", &[&Self::kurz(n, 400), &jetzt().to_string(), &id.to_string()])?; }
        Ok(())
    }
    pub fn aufgabe_loeschen(&self, id: i64) -> Result<(), String> { self.db.ausfuehren("DELETE FROM aufgaben WHERE id=CAST(? AS BIGINT)", &[&id.to_string()]) }
    pub fn aufgaben(&self, mit_erledigten: bool) -> Result<Vec<Value>, String> {
        let sql = if mit_erledigten { "SELECT id,erstellt,aktualisiert,titel,beschreibung,rolle,prioritaet,status,von,notiz FROM aufgaben ORDER BY CASE status WHEN 'erledigt' THEN 1 ELSE 0 END, prioritaet, id DESC LIMIT 200" }
            else { "SELECT id,erstellt,aktualisiert,titel,beschreibung,rolle,prioritaet,status,von,notiz FROM aufgaben WHERE status<>'erledigt' ORDER BY prioritaet, id LIMIT 60" };
        Ok(self.db.zeilen(sql, &[])?.into_iter().map(|r| {
            let n = |i: usize| r[i].as_deref().and_then(|s| s.parse::<i64>().ok()).unwrap_or(0);
            json!({"id":n(0),"erstellt":n(1),"aktualisiert":n(2),"titel":r[3],"beschreibung":r[4],"rolle":r[5],"prioritaet":n(6),"status":r[7],"von":r[8],"notiz":r[9]})
        }).collect())
    }
    pub fn offene_aufgaben(&self) -> Result<i64, String> { self.zahl("SELECT count(*) FROM aufgaben WHERE status<>'erledigt'", &[]) }

    // ---- Teambesprechung: Nachrichten zwischen den Rollen
    pub fn team_senden(&self, von: &str, an: &str, text: &str) -> Result<(), String> {
        self.db.ausfuehren("INSERT INTO team(zeit,von,an,text,gelesen) VALUES(CAST(? AS BIGINT),?,?,?,0)", &[&jetzt().to_string(), von, an, &Self::kurz(text, 600)])?;
        self.db.ausfuehren("DELETE FROM team WHERE id < (SELECT max(id) - 300 FROM team)", &[])
    }
    /// Ungelesene Teamnachrichten; `markieren` = als gelesen ablegen (beim echten Zug).
    pub fn team_fuer(&self, an: &str, markieren: bool) -> Result<Vec<Value>, String> {
        let z = self.db.zeilen("SELECT id,zeit,von,text FROM team WHERE an=? AND gelesen=0 ORDER BY id LIMIT 20", &[an])?;
        if markieren { self.db.ausfuehren("UPDATE team SET gelesen=1 WHERE an=? AND gelesen=0", &[an])?; }
        Ok(z.into_iter().map(|r| json!({"zeit":r[1],"von":r[2],"text":r[3]})).collect())
    }
    pub fn team_letzte(&self, n: usize) -> Result<Vec<Value>, String> {
        Ok(self.db.zeilen("SELECT id,zeit,von,an,text,gelesen FROM team ORDER BY id DESC LIMIT CAST(? AS BIGINT)", &[&n.to_string()])?.into_iter()
            .map(|r| json!({"id":r[0],"zeit":r[1],"von":r[2],"an":r[3],"text":r[4],"gelesen":r[5].as_deref() == Some("1")})).collect())
    }

    // ---- Langzeitgedächtnis: Erinnerungen zu Spielern, Orten und Erfahrungen
    pub fn merken(&self, von: &str, thema: &str, text: &str) -> Result<(), String> {
        self.db.ausfuehren("INSERT INTO erinnerungen(zeit,von,thema,text) VALUES(CAST(? AS BIGINT),?,?,?)", &[&jetzt().to_string(), von, &Self::kurz(thema, 60), &Self::kurz(text, 500)])?;
        self.db.ausfuehren("DELETE FROM erinnerungen WHERE id < (SELECT max(id) - 400 FROM erinnerungen)", &[])
    }
    pub fn erinnerung_loeschen(&self, id: i64) -> Result<(), String> { self.db.ausfuehren("DELETE FROM erinnerungen WHERE id=CAST(? AS BIGINT)", &[&id.to_string()]) }
    fn erinnerung_zeilen(z: Vec<Vec<Option<String>>>) -> Vec<Value> {
        z.into_iter().map(|r| json!({"id":r[0].as_deref().and_then(|s| s.parse::<i64>().ok()),"zeit":r[1],"von":r[2],"thema":r[3],"text":r[4]})).collect()
    }
    pub fn erinnerungen(&self, stichworte: &[String], n: usize) -> Result<Vec<Value>, String> {
        let mut out: Vec<Value> = Vec::new();
        for w in stichworte.iter().filter(|w| w.chars().count() >= 3).take(6) {
            let muster = format!("%{}%", w.to_lowercase());
            for e in Self::erinnerung_zeilen(self.db.zeilen("SELECT id,zeit,von,thema,text FROM erinnerungen WHERE lower(thema) LIKE ? OR lower(text) LIKE ? ORDER BY id DESC LIMIT 5", &[&muster, &muster])?) {
                if !out.iter().any(|x| x["id"] == e["id"]) { out.push(e); }
            }
        }
        for e in Self::erinnerung_zeilen(self.db.zeilen("SELECT id,zeit,von,thema,text FROM erinnerungen ORDER BY id DESC LIMIT CAST(? AS BIGINT)", &[&n.to_string()])?) {
            if out.len() >= n { break; }
            if !out.iter().any(|x| x["id"] == e["id"]) { out.push(e); }
        }
        out.truncate(n);
        Ok(out)
    }

    // ---- Verlauf: was das Modell gesehen und geantwortet hat
    pub fn verlauf_speichern(&self, rolle: &str, prompt: &str, antwort: &str) -> Result<(), String> {
        self.db.ausfuehren("INSERT INTO verlauf(zeit,rolle,prompt,antwort) VALUES(CAST(? AS BIGINT),?,?,?)", &[&jetzt().to_string(), rolle, &Self::kurz(prompt, 60_000), &Self::kurz(antwort, 20_000)])?;
        self.db.ausfuehren("DELETE FROM verlauf WHERE id < (SELECT max(id) - 300 FROM verlauf)", &[])
    }
    /// Kompaktierung: nur die jüngsten Notizen je Rolle bleiben.
    pub fn kompaktieren(&self, behalten: usize) -> Result<(), String> {
        for r in crate::konfig::ROLLEN {
            self.db.ausfuehren("DELETE FROM notizen WHERE rolle=? AND id NOT IN (SELECT id FROM notizen WHERE rolle=? ORDER BY id DESC LIMIT CAST(? AS BIGINT))", &[r, r, &behalten.to_string()])?;
        }
        Ok(())
    }

    // ---- Routinen: Inventur aller Planeten als Zeitreihe, Ergebnisse jedes Laufs
    pub fn inventur_speichern(&self, planeten: &[Value]) -> Result<(), String> {
        let t = jetzt().to_string();
        let n = |v: &Value| v.as_f64().map(|f| f.round() as i64).unwrap_or(0).to_string();
        for p in planeten {
            self.db.ausfuehren("INSERT INTO inventur(zeit,koord,erz,kristall,deuterium,nahrung,energie_saldo,bevoelkerung,stabilitaet,bauschleife,daten) VALUES(CAST(? AS BIGINT),?,CAST(? AS BIGINT),CAST(? AS BIGINT),CAST(? AS BIGINT),CAST(? AS BIGINT),CAST(? AS BIGINT),CAST(? AS BIGINT),CAST(? AS BIGINT),CAST(? AS BIGINT),?)",
                &[&t, p["koord"].as_str().unwrap_or("?"), &n(&p["bestand"]["erz"]), &n(&p["bestand"]["kristall"]), &n(&p["bestand"]["deuterium"]), &n(&p["bestand"]["nahrung"]),
                  &n(&p["energie"]["saldo"]), &n(&p["bevoelkerung"]), &n(&p["stabilitaet"]), &n(&p["bauschleife"]["belegt"]), &p.to_string()])?;
        }
        self.db.ausfuehren("DELETE FROM inventur WHERE id < (SELECT max(id) - 20000 FROM inventur)", &[])
    }
    /// Letzte Inventur je Planet (für Trends): Koordinate → Datensatz und Zeit.
    pub fn inventur_vorher(&self) -> Result<Vec<Value>, String> {
        let z = self.db.zeilen("SELECT zeit,daten FROM inventur WHERE zeit=(SELECT max(zeit) FROM inventur)", &[])?;
        Ok(z.into_iter().filter_map(|r| { let mut v: Value = serde_json::from_str(r[1].as_deref()?).ok()?; v["inventur_zeit"] = json!(r[0].as_deref()?.parse::<i64>().ok()?); Some(v) }).collect())
    }
    pub fn routine_speichern(&self, name: &str, art: &str, ok: bool, daten: &Value) -> Result<(), String> {
        let text: String = daten.to_string().chars().take(200_000).collect();
        self.db.ausfuehren("INSERT INTO routinen(zeit,name,art,ok,daten) VALUES(CAST(? AS BIGINT),?,?,CAST(? AS BIGINT),?)", &[&jetzt().to_string(), name, art, if ok { "1" } else { "0" }, &text])?;
        self.db.ausfuehren("DELETE FROM routinen WHERE id < (SELECT max(id) - 500 FROM routinen)", &[])
    }
    pub fn routine_letzte(&self, art: &str) -> Result<Option<Value>, String> {
        let z = self.db.zeilen("SELECT zeit,name,daten FROM routinen WHERE art=? AND ok=1 ORDER BY id DESC LIMIT 1", &[art])?;
        Ok(z.into_iter().next().and_then(|r| { let mut v: Value = serde_json::from_str(r[2].as_deref()?).ok()?; v["zeit"] = json!(r[0].as_deref()?.parse::<i64>().ok()?); v["routine"] = json!(r[1]); Some(v) }))
    }
    /// Nur lesende SQL-Abfrage auf das eigene Gedächtnis (Cockpit, lokale API, Wissensquelle „gedaechtnis“).
    pub fn nur_lesen(&self, sql: &str, max_zeilen: usize) -> Result<Value, String> {
        let q = sql_kapseln(sql, max_zeilen)?;
        let start = std::time::Instant::now();
        let (spalten, zeilen) = self.db.lesen(&q, 3)?;
        let zeilen: Vec<Vec<Value>> = zeilen.into_iter().map(|r| r.into_iter().map(|z| match z {
            None => Value::Null,
            Some(t) => t.parse::<i64>().map(|n| json!(n)).or_else(|_| t.parse::<f64>().map(|f| json!(f))).unwrap_or_else(|_| json!(t.chars().take(2000).collect::<String>())),
        }).collect()).collect();
        Ok(json!({"spalten":spalten,"zeilen":zeilen,"anzahl":zeilen.len(),"dauer_ms":start.elapsed().as_millis() as u64,"datenbank":self.db.art()}))
    }
    /// Tabellen und Spalten des Gedächtnisses, damit Menschen und Modelle Abfragen schreiben können.
    pub fn schema_text() -> &'static str {
        "Tabellen: notizen(zeit,rolle,art,text) · zuege(zeit,rolle,request_id,aktionen JSON,ergebnis JSON,begruendung) · aufgaben(id,erstellt,aktualisiert,titel,beschreibung,rolle,prioritaet,status,von,notiz) · team(zeit,von,an,text,gelesen) · erinnerungen(id,zeit,von,thema,text) · verlauf(zeit,rolle,prompt,antwort) · inventur(zeit,koord,erz,kristall,deuterium,nahrung,energie_saldo,bevoelkerung,stabilitaet,bauschleife,daten JSON) · routinen(zeit,name,art,ok,daten JSON) · training(zeit,rolle,modell,seele,angenommen,abgelehnt,system,nutzer,antwort). zeit = Unix-Sekunden."
    }
    /// Ein abgeschlossener Modellzug mit Ergebnis: Grundlage für eigene Trainingsdaten.
    #[allow(clippy::too_many_arguments)]
    pub fn training_speichern(&self, rolle: &str, modell: &str, seele: &str, system: &str, nutzer: &str, antwort: &str, angenommen: usize, abgelehnt: usize) -> Result<(), String> {
        self.db.ausfuehren("INSERT INTO training(zeit,rolle,modell,seele,system,nutzer,antwort,angenommen,abgelehnt) VALUES(CAST(? AS BIGINT),?,?,?,?,?,?,CAST(? AS BIGINT),CAST(? AS BIGINT))",
            &[&jetzt().to_string(), rolle, modell, seele, &Self::kurz(system, 80_000), &Self::kurz(nutzer, 80_000), &Self::kurz(antwort, 20_000), &angenommen.to_string(), &abgelehnt.to_string()])?;
        self.db.ausfuehren("DELETE FROM training WHERE id < (SELECT max(id) - 600 FROM training)", &[])
    }
    /// Chat-JSONL für Fine-Tuning: system, user, assistant; wahlweise nur angenommene Züge ohne Ablehnung.
    pub fn training_export(&self, pfad: &Path, nur_erfolgreich: bool, rolle: Option<&str>) -> Result<usize, String> {
        use std::io::Write;
        let mut f = std::io::BufWriter::new(std::fs::File::create(pfad).map_err(|e| format!("{}: {e}", pfad.display()))?);
        let mut n = 0;
        for r in self.db.zeilen("SELECT rolle,modell,seele,system,nutzer,antwort,angenommen,abgelehnt,zeit FROM training ORDER BY id", &[])? {
            let zahl = |i: usize| r[i].as_deref().and_then(|s| s.parse::<i64>().ok()).unwrap_or(0);
            if rolle.is_some_and(|x| r[0].as_deref() != Some(x)) { continue; }
            if nur_erfolgreich && (zahl(6) == 0 || zahl(7) > 0) { continue; }
            writeln!(f, "{}", json!({"messages":[{"role":"system","content":r[3]},{"role":"user","content":r[4]},{"role":"assistant","content":r[5]}],
                "meta":{"rolle":r[0],"modell":r[1],"seele":r[2],"angenommen":zahl(6),"abgelehnt":zahl(7),"zeit":zahl(8)}})).map_err(|e| e.to_string())?;
            n += 1;
        }
        f.flush().map_err(|e| e.to_string())?;
        Ok(n)
    }
    /// Zeitreihe aus den Inventur-Läufen: Punkte, Credits, Einwohner, Rohstoffe, Energie.
    pub fn kennzahlen(&self) -> Result<Vec<Value>, String> {
        let mut v: Vec<Value> = self.db.zeilen("SELECT zeit,daten FROM routinen WHERE art='inventur' AND ok=1 ORDER BY id DESC LIMIT 400", &[])?.into_iter().filter_map(|r| {
            let d: Value = serde_json::from_str(r[1].as_deref()?).ok()?;
            let s = &d["summe"];
            Some(json!({"zeit":r[0].as_deref()?.parse::<i64>().ok()?,"punkte":d["punkte"],"credits":d["credits"],"rang":d["rang"],"bevoelkerung":s["bevoelkerung"],"erz":s["bestand"]["erz"],
                "kristall":s["bestand"]["kristall"],"deuterium":s["bestand"]["deuterium"],"nahrung":s["bestand"]["nahrung"],"energie_saldo":s["energie_saldo"],"planeten":s["planeten"]}))
        }).collect();
        v.reverse();
        Ok(v)
    }

    /// Verlauf und Züge als JSONL für Auswertung, Vergleich von Modellen oder Training.
    pub fn export_jsonl(&self, pfad: &Path) -> Result<usize, String> {
        use std::io::Write;
        let mut f = std::io::BufWriter::new(std::fs::File::create(pfad).map_err(|e| format!("{}: {e}", pfad.display()))?);
        let mut n = 0;
        for r in self.db.zeilen("SELECT zeit,rolle,prompt,antwort FROM verlauf ORDER BY id", &[])? {
            writeln!(f, "{}", json!({"art":"modellaufruf","zeit":r[0].as_deref().and_then(|s| s.parse::<i64>().ok()),"rolle":r[1],"prompt":r[2],"antwort":r[3]})).map_err(|e| e.to_string())?;
            n += 1;
        }
        for r in self.db.zeilen("SELECT zeit,rolle,aktionen,ergebnis,begruendung FROM zuege ORDER BY id", &[])? {
            let lies = |s: &Option<String>| s.as_deref().and_then(|t| serde_json::from_str::<Value>(t).ok()).unwrap_or(Value::Null);
            writeln!(f, "{}", json!({"art":"zug","zeit":r[0].as_deref().and_then(|s| s.parse::<i64>().ok()),"rolle":r[1],"aktionen":lies(&r[2]),"ergebnis":lies(&r[3]),"begruendung":r[4]})).map_err(|e| e.to_string())?;
            n += 1;
        }
        f.flush().map_err(|e| e.to_string())?;
        Ok(n)
    }

    pub fn statistik(&self) -> Result<Value, String> {
        let mut v = json!({"art":self.db.art()});
        for t in ["notizen", "zuege", "ereignisse", "aufgaben", "team", "erinnerungen", "verlauf", "inventur", "routinen", "training"] {
            let z = self.db.zeilen(&format!("SELECT count(*) FROM {t}"), &[])?;
            v[t] = json!(z.first().and_then(|r| r[0].as_deref()).and_then(|s| s.parse::<i64>().ok()).unwrap_or(0));
        }
        Ok(v)
    }
    /// Schreiben, Lesen, Löschen: Teil der lokalen Prüfungen vor dem ersten Spiel.
    pub fn selbsttest(&self) -> Result<Value, String> {
        let marke = format!("selbsttest-{}", jetzt());
        self.notiz("selbsttest", "pruefung", &marke)?;
        let gelesen = self.notizen("selbsttest", 1)?;
        if gelesen.first().and_then(|n| n["text"].as_str()) != Some(marke.as_str()) { return Err("Gelesener Wert stimmt nicht".into()); }
        self.db.ausfuehren("DELETE FROM notizen WHERE rolle=?", &["selbsttest"])?;
        self.statistik()
    }
}

#[cfg(feature = "sqlite")]
mod sqlite {
    use super::*;
    use std::sync::Mutex;
    pub struct Sqlite(Mutex<rusqlite::Connection>);
    impl Sqlite {
        pub fn oeffnen(pfad: &Path) -> Result<Self, String> {
            let c = rusqlite::Connection::open(pfad).map_err(|e| format!("SQLite {}: {e}", pfad.display()))?;
            c.execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=NORMAL; PRAGMA trusted_schema=OFF;").map_err(|e| e.to_string())?;
            c.execute_batch(SCHEMA_SQLITE).map_err(|e| e.to_string())?;
            Ok(Self(Mutex::new(c)))
        }
    }
    impl Gedaechtnis for Sqlite {
        fn art(&self) -> &'static str { "sqlite" }
        fn ausfuehren(&self, sql: &str, werte: &[&str]) -> Result<(), String> {
            let sql = sql.replace("CAST(? AS BIGINT)", "CAST(? AS INTEGER)");
            self.0.lock().unwrap().execute(&sql, rusqlite::params_from_iter(werte.iter())).map(|_| ()).map_err(|e| e.to_string())
        }
        fn zeilen(&self, sql: &str, werte: &[&str]) -> Result<Vec<Vec<Option<String>>>, String> {
            let sql = sql.replace("CAST(? AS BIGINT)", "CAST(? AS INTEGER)");
            let c = self.0.lock().unwrap();
            let mut s = c.prepare(&sql).map_err(|e| e.to_string())?;
            let n = s.column_count();
            let z = s.query_map(rusqlite::params_from_iter(werte.iter()), |r| {
                (0..n).map(|i| r.get_ref(i).map(|v| match v {
                    rusqlite::types::ValueRef::Null => None,
                    rusqlite::types::ValueRef::Integer(x) => Some(x.to_string()),
                    rusqlite::types::ValueRef::Real(x) => Some(x.to_string()),
                    rusqlite::types::ValueRef::Text(t) | rusqlite::types::ValueRef::Blob(t) => Some(String::from_utf8_lossy(t).into_owned()),
                })).collect::<Result<Vec<_>, _>>()
            }).map_err(|e| e.to_string())?;
            z.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
        }
        fn lesen(&self, sql: &str, sekunden: u64) -> Result<(Vec<String>, Vec<Vec<Option<String>>>), String> {
            let c = self.0.lock().unwrap();
            // Wächter: bricht zu lange Abfragen (z. B. endlose Rekursion) ab.
            let griff = c.get_interrupt_handle();
            let (fertig, warten) = std::sync::mpsc::channel::<()>();
            let waechter = std::thread::spawn(move || { if warten.recv_timeout(std::time::Duration::from_secs(sekunden)).is_err() { griff.interrupt(); } });
            let ergebnis = (|| {
                let mut s = c.prepare(sql).map_err(|e| e.to_string())?;
                if !s.readonly() { return Err("Nur lesende Abfragen".to_string()); }
                let spalten: Vec<String> = s.column_names().into_iter().map(str::to_string).collect();
                let n = spalten.len();
                let z = s.query_map([], |r| (0..n).map(|i| r.get_ref(i).map(|v| match v {
                    rusqlite::types::ValueRef::Null => None,
                    rusqlite::types::ValueRef::Integer(x) => Some(x.to_string()),
                    rusqlite::types::ValueRef::Real(x) => Some(x.to_string()),
                    rusqlite::types::ValueRef::Text(t) => Some(String::from_utf8_lossy(t).into_owned()),
                    rusqlite::types::ValueRef::Blob(b) => Some(format!("[{} Bytes]", b.len())),
                })).collect::<Result<Vec<_>, _>>()).map_err(|e| e.to_string())?;
                Ok((spalten, z.collect::<Result<Vec<_>, _>>().map_err(|e| if e.to_string().contains("interrupt") { format!("Abfrage nach {sekunden} s abgebrochen") } else { e.to_string() })?))
            })();
            let _ = fertig.send(());
            let _ = waechter.join();
            ergebnis
        }
    }
}

#[cfg(feature = "duckdb")]
mod duckdb {
    //! Schmale Hülle um die stabile C-Schnittstelle von DuckDB 1.5.
    use super::*;
    use libloading::Library;
    use std::ffi::{c_char, c_void, CStr, CString};
    use std::ptr::null_mut;
    use std::sync::Mutex;

    type Griff = *mut c_void;
    #[repr(C)]
    struct Ergebnis { spalten: u64, zeilen: u64, geaendert: u64, daten: Griff, fehler: *mut c_char, intern: Griff }
    impl Default for Ergebnis { fn default() -> Self { Self { spalten: 0, zeilen: 0, geaendert: 0, daten: null_mut(), fehler: null_mut(), intern: null_mut() } } }
    struct Api {
        _lib: Library,
        open: unsafe extern "C" fn(*const c_char, *mut Griff, Griff, *mut *mut c_char) -> u32,
        close: unsafe extern "C" fn(*mut Griff),
        connect: unsafe extern "C" fn(Griff, *mut Griff) -> u32,
        disconnect: unsafe extern "C" fn(*mut Griff),
        config_create: unsafe extern "C" fn(*mut Griff) -> u32,
        config_set: unsafe extern "C" fn(Griff, *const c_char, *const c_char) -> u32,
        config_destroy: unsafe extern "C" fn(*mut Griff),
        prepare: unsafe extern "C" fn(Griff, *const c_char, *mut Griff) -> u32,
        prepare_error: unsafe extern "C" fn(Griff) -> *const c_char,
        prepare_destroy: unsafe extern "C" fn(*mut Griff),
        bind: unsafe extern "C" fn(Griff, u64, *const c_char, u64) -> u32,
        execute: unsafe extern "C" fn(Griff, *mut Ergebnis) -> u32,
        destroy: unsafe extern "C" fn(*mut Ergebnis),
        error: unsafe extern "C" fn(*mut Ergebnis) -> *const c_char,
        rows: unsafe extern "C" fn(*mut Ergebnis) -> u64,
        columns: unsafe extern "C" fn(*mut Ergebnis) -> u64,
        value: unsafe extern "C" fn(*mut Ergebnis, u64, u64) -> *mut c_char,
        is_null: unsafe extern "C" fn(*mut Ergebnis, u64, u64) -> bool,
        free: unsafe extern "C" fn(Griff),
        column_name: unsafe extern "C" fn(*mut Ergebnis, u64) -> *const c_char,
        interrupt: unsafe extern "C" fn(Griff),
    }
    /// Verbindungsgriff für den Abbruch-Wächter; duckdb_interrupt ist threadsicher.
    struct Abbruch(Griff, unsafe extern "C" fn(Griff));
    unsafe impl Send for Abbruch {}
    unsafe fn text(p: *const c_char) -> String { if p.is_null() { String::new() } else { CStr::from_ptr(p).to_string_lossy().into_owned() } }
    fn c(s: &str) -> Result<CString, String> { CString::new(s).map_err(|_| "NUL-Zeichen im Wert".to_string()) }

    struct Verbindung { api: Api, db: Griff, conn: Griff }
    // Zugriff ausschließlich über den Mutex unten.
    unsafe impl Send for Verbindung {}
    impl Drop for Verbindung { fn drop(&mut self) { unsafe { (self.api.disconnect)(&mut self.conn); (self.api.close)(&mut self.db); } } }

    pub struct DuckDb(Mutex<Verbindung>);
    impl DuckDb {
        pub fn oeffnen(pfad: &Path, bibliothek: &Path) -> Result<Self, String> {
            unsafe {
                let lib = Library::new(bibliothek).map_err(|e| format!("DuckDB-Laufzeit {}: {e}", bibliothek.display()))?;
                macro_rules! sym { ($n:literal) => { *lib.get(concat!($n, "\0").as_bytes()).map_err(|e| e.to_string())? } }
                let api = Api { open: sym!("duckdb_open_ext"), close: sym!("duckdb_close"), connect: sym!("duckdb_connect"), disconnect: sym!("duckdb_disconnect"),
                    config_create: sym!("duckdb_create_config"), config_set: sym!("duckdb_set_config"), config_destroy: sym!("duckdb_destroy_config"),
                    prepare: sym!("duckdb_prepare"), prepare_error: sym!("duckdb_prepare_error"), prepare_destroy: sym!("duckdb_destroy_prepare"),
                    bind: sym!("duckdb_bind_varchar_length"), execute: sym!("duckdb_execute_prepared"), destroy: sym!("duckdb_destroy_result"),
                    error: sym!("duckdb_result_error"), rows: sym!("duckdb_row_count"), columns: sym!("duckdb_column_count"),
                    value: sym!("duckdb_value_varchar"), is_null: sym!("duckdb_value_is_null"), free: sym!("duckdb_free"),
                    column_name: sym!("duckdb_column_name"), interrupt: sym!("duckdb_interrupt"), _lib: lib };
                let (mut db, mut conn, mut cfg) = (null_mut(), null_mut(), null_mut());
                if (api.config_create)(&mut cfg) != 0 { return Err("DuckDB-Konfiguration fehlgeschlagen".into()); }
                for (k, v) in [("access_mode", "READ_WRITE"), ("threads", "2"), ("memory_limit", "256MB"), ("enable_external_access", "false"),
                    ("autoload_known_extensions", "false"), ("autoinstall_known_extensions", "false"), ("lock_configuration", "true")] {
                    if (api.config_set)(cfg, c(k)?.as_ptr(), c(v)?.as_ptr()) != 0 { (api.config_destroy)(&mut cfg); return Err(format!("DuckDB-Option {k} fehlgeschlagen")); }
                }
                let mut fehler = null_mut();
                let pfad_c = c(&pfad.to_string_lossy())?;
                let status = (api.open)(pfad_c.as_ptr(), &mut db, cfg, &mut fehler);
                (api.config_destroy)(&mut cfg);
                if status != 0 { let m = text(fehler); (api.free)(fehler.cast()); return Err(format!("DuckDB {}: {m}", pfad.display())); }
                if (api.connect)(db, &mut conn) != 0 { (api.close)(&mut db); return Err("DuckDB-Verbindung fehlgeschlagen".into()); }
                let d = DuckDb(Mutex::new(Verbindung { api, db, conn }));
                for s in SCHEMA_DUCKDB { d.ausfuehren(s, &[])?; }
                Ok(d)
            }
        }
        fn abfrage(&self, sql: &str, werte: &[&str]) -> Result<Vec<Vec<Option<String>>>, String> { self.abfrage_mit(sql, werte, None).map(|(_, z)| z) }
        fn abfrage_mit(&self, sql: &str, werte: &[&str], abbruch_nach: Option<u64>) -> Result<(Vec<String>, Vec<Vec<Option<String>>>), String> {
            let v = self.0.lock().unwrap();
            let a = &v.api;
            let sql = c(sql)?;
            let (fertig, warten) = std::sync::mpsc::channel::<()>();
            let waechter = abbruch_nach.map(|s| {
                let griff = Abbruch(v.conn, a.interrupt);
                std::thread::spawn(move || { let g = griff; if warten.recv_timeout(std::time::Duration::from_secs(s)).is_err() { unsafe { (g.1)(g.0) } } })
            });
            let ergebnis = unsafe {
                let mut stmt = null_mut();
                if (a.prepare)(v.conn, sql.as_ptr(), &mut stmt) != 0 { let e = text((a.prepare_error)(stmt)); (a.prepare_destroy)(&mut stmt); Err(e) } else {
                let mut fehler = None;
                for (i, w) in werte.iter().enumerate() {
                    if (a.bind)(stmt, i as u64 + 1, w.as_ptr().cast(), w.len() as u64) != 0 { fehler = Some("DuckDB-Parameter nicht bindbar".to_string()); break; }
                }
                if let Some(e) = fehler { (a.prepare_destroy)(&mut stmt); Err(e) } else {
                let mut r = Ergebnis::default();
                let status = (a.execute)(stmt, &mut r);
                (a.prepare_destroy)(&mut stmt);
                if status != 0 { let e = text((a.error)(&mut r)); (a.destroy)(&mut r); Err(if e.contains("INTERRUPT") || e.contains("nterrupt") { "Abfrage wegen Zeitlimit abgebrochen".into() } else { e }) } else {
                let (zeilen, spalten) = ((a.rows)(&mut r), (a.columns)(&mut r));
                let namen: Vec<String> = (0..spalten).map(|s| text((a.column_name)(&mut r, s))).collect();
                let mut out = Vec::with_capacity(zeilen as usize);
                for z in 0..zeilen {
                    let mut zeile = Vec::with_capacity(spalten as usize);
                    for s in 0..spalten {
                        if (a.is_null)(&mut r, s, z) { zeile.push(None); continue; }
                        let p = (a.value)(&mut r, s, z);
                        zeile.push(Some(text(p)));
                        (a.free)(p.cast());
                    }
                    out.push(zeile);
                }
                (a.destroy)(&mut r);
                Ok((namen, out))
            }}}};
            let _ = fertig.send(());
            if let Some(w) = waechter { let _ = w.join(); }
            ergebnis
        }
    }
    impl Gedaechtnis for DuckDb {
        fn art(&self) -> &'static str { "duckdb" }
        fn ausfuehren(&self, sql: &str, werte: &[&str]) -> Result<(), String> { self.abfrage(sql, werte).map(|_| ()) }
        fn zeilen(&self, sql: &str, werte: &[&str]) -> Result<Vec<Vec<Option<String>>>, String> { self.abfrage(sql, werte) }
        fn lesen(&self, sql: &str, sekunden: u64) -> Result<(Vec<String>, Vec<Vec<Option<String>>>), String> { self.abfrage_mit(sql, &[], Some(sekunden)) }
    }
}

#[cfg(all(test, feature = "duckdb"))]
mod duckdb_tests {
    use super::*;
    /// Nutzt die im Repository liegende offizielle DuckDB-1.5.6-Laufzeit, falls vorhanden.
    #[test]
    fn duckdb_gedaechtnis_mit_echter_laufzeit() {
        let dll = std::env::var("STERNENEPOCHE_DUCKDB_LIB").map(std::path::PathBuf::from)
            .unwrap_or_else(|_| Path::new(env!("CARGO_MANIFEST_DIR")).join("../../wissen/native/duckdb-1.5.6/duckdb.dll"));
        if !dll.exists() { eprintln!("DuckDB-Laufzeit fehlt, Test übersprungen"); return; }
        let p = std::env::temp_dir().join(format!("harness-duck-{}-{}.duckdb", jetzt(), std::process::id()));
        let s = Speicher { db: oeffnen("duckdb", &p, &dll).unwrap() };
        s.selbsttest().unwrap();
        s.notiz("feldherr", "plan", "Sonden zuerst'; DROP TABLE zuege; --").unwrap();
        s.zug("feldherr", "r1", &json!([{"typ":"fertigen"}]), &json!({"ok":true}), "Aufklärung").unwrap();
        assert_eq!(s.notizen("feldherr", 3).unwrap()[0]["text"], "Sonden zuerst'; DROP TABLE zuege; --");
        assert_eq!(s.statistik().unwrap()["zuege"], 1);
        let id = s.aufgabe_neu("Sonden bauen", "", "feldherr", 2, "besitzer").unwrap();
        s.aufgabe_aendern(id, Some("erledigt"), None).unwrap();
        assert!(s.aufgaben(false).unwrap().is_empty() && s.aufgaben(true).unwrap().len() == 1);
        s.merken("feldherr", "2:14:6", "Planet ist unbewacht").unwrap();
        assert_eq!(s.erinnerungen(&["2:14".into()], 3).unwrap().len(), 1);
        for i in 0..20 { s.notiz("feldherr", "notiz", &format!("n{i}")).unwrap(); }
        s.kompaktieren(5).unwrap();
        assert_eq!(s.notizen("feldherr", 50).unwrap().len(), 5);
        // Externer Dateizugriff aus SQL ist abgeschaltet.
        assert!(s.db.zeilen("SELECT * FROM read_csv_auto('C:/Windows/win.ini')", &[]).is_err());
        // Inventur als Zeitreihe und lesendes SQL mit Spaltennamen, Zeitlimit und Schutz.
        s.inventur_speichern(&[json!({"koord":"2:14:6","bestand":{"erz":1500.4,"kristall":10},"energie":{"saldo":-5},"bevoelkerung":1000,"stabilitaet":50,"bauschleife":{"belegt":0}})]).unwrap();
        assert_eq!(s.inventur_vorher().unwrap()[0]["koord"], "2:14:6");
        let v = s.nur_lesen("SELECT koord, erz, energie_saldo FROM inventur ORDER BY zeit DESC", 10).unwrap();
        assert_eq!(v["spalten"], json!(["koord", "erz", "energie_saldo"]));
        assert_eq!(v["zeilen"][0], json!(["2:14:6", 1500, -5]));
        assert!(s.nur_lesen("DELETE FROM inventur", 10).is_err() && s.nur_lesen("SELECT * FROM read_csv_auto('x.csv')", 10).is_err());
        let t = std::time::Instant::now();
        assert!(s.nur_lesen("WITH RECURSIVE c(x) AS (SELECT 1 UNION ALL SELECT x+1 FROM c) SELECT count(*) FROM c", 10).is_err());
        assert!(t.elapsed().as_secs() < 10, "Zeitlimit greift");
        assert_eq!(s.nur_lesen("SELECT 1 AS danach", 10).unwrap()["zeilen"][0][0], 1, "Verbindung nach Abbruch weiter nutzbar");
        s.routine_speichern("analyse", "analyse", true, &json!({"kurz":"1 dringend"})).unwrap();
        assert_eq!(s.routine_letzte("analyse").unwrap().unwrap()["kurz"], "1 dringend");
        // Trainingsdaten: nur erfolgreiche Züge, Chat-Format; Kennzahlen aus den Inventur-Läufen.
        s.training_speichern("verwalter", "qwen3:8b", "ab12cd34", "SYSTEM", "LAGE 1", "{\"aktionen\":[]}", 2, 0).unwrap();
        s.training_speichern("verwalter", "qwen3:8b", "ab12cd34", "SYSTEM", "LAGE 2", "{}", 1, 1).unwrap();
        s.training_speichern("feldherr", "qwen3:8b", "ab12cd34", "SYSTEM", "LAGE 3", "{}", 1, 0).unwrap();
        let datei = std::env::temp_dir().join(format!("sft-{}-{}.jsonl", jetzt(), std::process::id()));
        assert_eq!(s.training_export(&datei, true, None).unwrap(), 2);
        assert_eq!(s.training_export(&datei, false, Some("verwalter")).unwrap(), 2);
        let zeile: Value = serde_json::from_str(std::fs::read_to_string(&datei).unwrap().lines().next().unwrap()).unwrap();
        assert_eq!(zeile["messages"][1]["content"], "LAGE 1");
        assert_eq!(zeile["messages"][2]["role"], "assistant");
        assert_eq!(zeile["meta"]["angenommen"], 2);
        let _ = std::fs::remove_file(&datei);
        s.routine_speichern("inventur", "inventur", true, &json!({"punkte":120,"credits":5000,"rang":3,"summe":{"bevoelkerung":900,"energie_saldo":-4,"planeten":1,"bestand":{"erz":10,"kristall":20,"deuterium":3,"nahrung":40}}})).unwrap();
        s.routine_speichern("inventur", "inventur", true, &json!({"punkte":150,"credits":5200,"rang":2,"summe":{"bevoelkerung":950,"energie_saldo":6,"planeten":1,"bestand":{"erz":30,"kristall":25,"deuterium":4,"nahrung":42}}})).unwrap();
        let k = s.kennzahlen().unwrap();
        assert_eq!(k.len(), 2);
        assert_eq!(k[1]["punkte"], 150, "älteste zuerst");
        assert_eq!(k[0]["erz"], 10);
        assert_eq!(k[1]["energie_saldo"], 6);
    }
}

#[cfg(all(test, feature = "sqlite"))]
mod tests {
    use super::*;
    #[test]
    fn sqlite_gedaechtnis_schreibt_liest_und_zaehlt() {
        let p = std::env::temp_dir().join(format!("harness-gedaechtnis-{}.sqlite3", jetzt() as u128 * 1000 + std::process::id() as u128));
        let s = Speicher { db: oeffnen("sqlite", &p, Path::new("")).unwrap() };
        s.selbsttest().unwrap();
        s.notiz("verwalter", "plan", "Erst Energie, dann Erz'); DROP TABLE notizen;--").unwrap();
        s.zug("verwalter", "r1", &json!([{"typ":"bauen"}]), &json!({"ok":true}), "weil").unwrap();
        assert_eq!(s.notizen("verwalter", 5).unwrap()[0]["text"], "Erst Energie, dann Erz'); DROP TABLE notizen;--");
        assert_eq!(s.letzte_zuege("verwalter", 5).unwrap()[0]["aktionen"][0]["typ"], "bauen");
        assert_eq!(s.statistik().unwrap()["zuege"], 1);
        let id = s.aufgabe_neu("Energie sichern", "Solarkraftwerk auf allen Kolonien", "verwalter", 1, "stratege").unwrap();
        s.aufgabe_aendern(id, Some("aktiv"), Some("läuft")).unwrap();
        assert!(s.aufgabe_aendern(id, Some("fertig?"), None).is_err());
        assert_eq!(s.aufgaben(false).unwrap()[0]["status"], "aktiv");
        s.team_senden("stratege", "feldherr", "Bitte Sonden nach 2:14").unwrap();
        assert_eq!(s.team_fuer("feldherr", false).unwrap().len(), 1, "Vorschau liest, ohne zu verbrauchen");
        assert_eq!(s.team_fuer("feldherr", true).unwrap().len(), 1);
        assert!(s.team_fuer("feldherr", true).unwrap().is_empty(), "gelesen");
        s.merken("diplomat", "Xator", "Xator hat uns am Tag 3 angegriffen").unwrap();
        s.merken("verwalter", "Handel", "Kristall ist billig").unwrap();
        assert_eq!(s.erinnerungen(&["xator".into()], 1).unwrap()[0]["thema"], "Xator");
        for i in 0..60 { s.notiz("verwalter", "notiz", &format!("n{i}")).unwrap(); }
        s.kompaktieren(10).unwrap();
        assert_eq!(s.notizen("verwalter", 100).unwrap().len(), 10);
        s.verlauf_speichern("verwalter", "LAGE", "{}").unwrap();
        assert_eq!(s.statistik().unwrap()["verlauf"], 1);
        // Trainingsdaten: nur erfolgreiche Züge, Chat-Format; Kennzahlen aus den Inventur-Läufen.
        s.training_speichern("verwalter", "qwen3:8b", "ab12cd34", "SYSTEM", "LAGE 1", "{\"aktionen\":[]}", 2, 0).unwrap();
        s.training_speichern("verwalter", "qwen3:8b", "ab12cd34", "SYSTEM", "LAGE 2", "{}", 1, 1).unwrap();
        s.training_speichern("feldherr", "qwen3:8b", "ab12cd34", "SYSTEM", "LAGE 3", "{}", 1, 0).unwrap();
        let datei = std::env::temp_dir().join(format!("sft-{}-{}.jsonl", jetzt(), std::process::id()));
        assert_eq!(s.training_export(&datei, true, None).unwrap(), 2);
        assert_eq!(s.training_export(&datei, false, Some("verwalter")).unwrap(), 2);
        let zeile: Value = serde_json::from_str(std::fs::read_to_string(&datei).unwrap().lines().next().unwrap()).unwrap();
        assert_eq!(zeile["messages"][1]["content"], "LAGE 1");
        assert_eq!(zeile["messages"][2]["role"], "assistant");
        assert_eq!(zeile["meta"]["angenommen"], 2);
        let _ = std::fs::remove_file(&datei);
        s.routine_speichern("inventur", "inventur", true, &json!({"punkte":120,"credits":5000,"rang":3,"summe":{"bevoelkerung":900,"energie_saldo":-4,"planeten":1,"bestand":{"erz":10,"kristall":20,"deuterium":3,"nahrung":40}}})).unwrap();
        s.routine_speichern("inventur", "inventur", true, &json!({"punkte":150,"credits":5200,"rang":2,"summe":{"bevoelkerung":950,"energie_saldo":6,"planeten":1,"bestand":{"erz":30,"kristall":25,"deuterium":4,"nahrung":42}}})).unwrap();
        let k = s.kennzahlen().unwrap();
        assert_eq!(k.len(), 2);
        assert_eq!(k[1]["punkte"], 150, "älteste zuerst");
        assert_eq!(k[0]["erz"], 10);
        assert_eq!(k[1]["energie_saldo"], 6);
    }
}
