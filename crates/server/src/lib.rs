//! One authoritative Rust world, transactional accounts/commands, no model proxy.
mod monitoring;
mod admission;
mod identity;
mod authenticator;
pub mod mail;
use argon2::{
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
mod communication;
mod workspace;
mod akten;
pub mod agenten;
mod ziele;
pub mod eingabe;
pub mod fairplay;
pub mod harness;
pub mod mcp;
// 20.10.2026: interne Spielerart (Bot, Agent, Mensch, Mensch mit Agent); nur für Server und Spielleitung.
pub mod spielerart;
mod uebergabe;
pub mod local;
pub mod public_catalog;
pub mod web_assets;
pub mod beschreibungen;
use kern::{aktion::Aktion, regeln::Regelwerk, typen::*, Welt};
use lauf::bots::{Bot, Bottyp};
use rand::{rngs::OsRng, RngCore};
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

pub const API_VERSION: u32 = 1;
pub const BOT_COUNT: u16 = 30;
pub const SEATS: u16 = 20;
/// Local rounds persist every five-second tick. Confirmed completions are durable.
pub const LOCAL_TICK_CHECKPOINT_SECS: u64 = 5;
/// Local rounds keep battle reports and daily values of the last 30 game days in memory.
pub const LOCAL_REPORT_DAYS: i64 = 30;
/// Local rounds keep the write-only research archives (actions, domain events) of the last
/// 14 game days on disk. No game function reads them; online servers keep everything.
pub const LOCAL_ARCHIVE_DAYS: i64 = 14;
pub fn now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

#[cfg(test)] mod recovery_tests {
 use super::*;
 fn root()->PathBuf{std::env::temp_dir().join(format!("stern-recovery-{}",token()))}
 #[test] fn panicking_transaction_rolls_back_world_and_sql_and_keeps_accepting_commands(){
  let mut g=Game::open_world(&root(),712,Some(30)).unwrap();let before=g.world.hash();let tempo=g.runtime.tempo;
  let n:i64=g.db.query_row("SELECT count(*) FROM audit",[],|r|r.get(0)).unwrap();
  let reply=g.transaction(|g|{g.runtime.tempo=17;g.world.spieler[0].stufe=4;g.db.execute("INSERT INTO audit(time,kind,result) VALUES(0,'test','rollback')",[]).unwrap();panic!("Controlled recovery fault");});
  assert_eq!(reply.unwrap_err().0,503);assert_eq!(g.world.hash(),before);assert_eq!(g.runtime.tempo,tempo);
  let after:i64=g.db.query_row("SELECT count(*) FROM audit",[],|r|r.get(0)).unwrap();assert_eq!(n,after);
  g.transaction(|g|{g.runtime.tempo=2;Ok(json!({"ok":true}))}).unwrap();assert_eq!(g.runtime.tempo,2);
 }
 #[test] fn simultaneous_research_builds_and_fabrication_are_durable_without_shutdown(){
  let path=root();let mut g=Game::open_world(&path,712,Some(30)).unwrap();g.runtime.bots_enabled=false;
  for sid in 0..8u16 {
   let p=g.world.spieler[sid as usize].heimat as usize;let k=g.world.planeten[p].koord;g.world.spieler[sid as usize].stufe=5;
   g.world.planeten[p].bevoelkerung=100_000*M;g.world.planeten[p].gebaeude[Gebaeude::Akademie.idx()]=10;
   g.world.planeten[p].gebaeude[Gebaeude::Solarkraftwerk.idx()]=20;g.world.planeten[p].gebaeude[Gebaeude::Farm.idx()]=20;g.world.planeten[p].gebaeude[Gebaeude::Labor.idx()]=5;g.world.planeten[p].gebaeude[Gebaeude::Werft.idx()]=1;g.world.planeten[p].bestand=[1_000_000*M;GUETER];g.world.raten_neu(p);
   assert!(g.world.fp_rate(sid)>0,"isolated fixture needs a working laboratory");
   g.world.forschen(sid,Rolle::Alle,Forschung::Energietechnik,None).unwrap();g.world.spieler[sid as usize].forschung_aktiv.as_mut().unwrap().fp_rest=1;
   g.world.bauen(sid,Rolle::Alle,k,Gebaeude::Erzmine).unwrap();g.world.planeten[p].bauschleife[0].fertig=Some(STUNDE);g.world.planeten[p].bauschleife[0].dauer=STUNDE;g.world.plane(STUNDE,kern::welt::EreignisArt::BauFertig{planet:p as u32});
   g.world.fertigen(sid,Rolle::Alle,k,kern::welt::Produkt::Einheit(Einheit::Spionagesonde),1).unwrap();g.world.planeten[p].fertigung_naechste[0]=STUNDE;g.world.plane(STUNDE,kern::welt::EreignisArt::FertigungFertig{planet:p as u32,schleife:0});
  }
  g.checkpoint_now().unwrap();let start=std::time::Instant::now();g.advance(STUNDE as u32).unwrap();println!("24 simultaneous completions: {} ms",start.elapsed().as_millis());
  for sid in 0..8u16{let p=g.world.spieler[sid as usize].heimat as usize;assert_eq!(g.world.spieler[sid as usize].forschung[Forschung::Energietechnik.idx()],1);assert!(g.world.planeten[p].bauschleife.is_empty());assert_eq!(g.world.planeten[p].einheiten[Einheit::Spionagesonde.idx()],1);}
  let hash=g.world.hash();let time=g.world.zeit;drop(g); // no graceful shutdown/final checkpoint
  let resumed=Game::open_world(&path,712,Some(30)).unwrap();assert_eq!(resumed.world.zeit,time);assert_eq!(resumed.world.hash(),hash);
 }
}
pub fn token() -> String {
    let mut b = [0u8; 32];
    OsRng.fill_bytes(&mut b);
    hex(&b)
}
fn hex(b: &[u8]) -> String {
    b.iter().map(|n| format!("{n:02x}")).collect()
}
fn write_private(path:&Path,data:&[u8])->std::io::Result<()> {
    use std::io::Write;
    let mut options=std::fs::OpenOptions::new();options.write(true).create_new(true);
    #[cfg(unix)] {use std::os::unix::fs::OpenOptionsExt;options.mode(0o600);}
    let mut file=options.open(path)?;file.write_all(data)?;file.sync_all()
}
pub fn digest(s: &str) -> String {
    hex(&Sha256::digest(s.as_bytes()))
}
pub fn password_hash(s: &str) -> Result<String, String> {
    if s.len() < 12 || s.len() > 128 {
        return Err("Passwort braucht 12 bis 128 Zeichen".into());
    }
    Argon2::default()
        .hash_password(s.as_bytes(), &SaltString::generate(&mut OsRng))
        .map(|h| h.to_string())
        .map_err(|_| "Passwort konnte nicht verarbeitet werden".into())
}
pub fn password_matches(password: &str, hash: &str) -> bool {
    PasswordHash::new(hash).ok().is_some_and(|h| {
        Argon2::default()
            .verify_password(password.as_bytes(), &h)
            .is_ok()
    })
}
#[derive(Clone, Serialize, Deserialize)]
pub struct Lease {
    account: i64,
    hash: String,
    until: i64,
    roles: Vec<Rolle>,
    /// Harness-Schlüssel, der diese Freigabe hält; None für den Browser.
    #[serde(default)]
    token: Option<i64>,
}
#[derive(Clone, Serialize, Deserialize)]
pub struct Runtime {
    #[serde(default = "admission::legacy_admission_limit")]
    pub seat_capacity: u16,
    pub world_id: String,
    pub tempo: u32,
    pub paused: bool,
    pub bots: BTreeMap<SpielerId, Bot>,
    pub bots_enabled: bool,
    pub revision: u64,
    leases: BTreeMap<SpielerId, Lease>,
    #[serde(default = "default_bot_period")]
    pub bot_period_secs: i64,
    /// Opferschutz gegen Räuberbots (Startoption `--opferschutz`, Admin `settings.opferschutz`); neue Welten: aus,
    /// ältere Laufzeiten ohne das Feld: an (wie damals).
    #[serde(default)]
    pub opferschutz: lauf::bots::Opferschutz,
    agent_status: BTreeMap<SpielerId, Value>,
    pub bot_actions: u64,
    pub bot_rejected: u64,
    #[serde(default)]
    bot_trace: BTreeMap<SpielerId, Vec<Value>>,
    #[serde(default)]
    public_history: Vec<Value>,
    #[serde(default)]
    pub maintenance: String,
    #[serde(default = "admission::legacy_admission_limit")]
    pub admission_limit: u16,
    #[serde(default)]
    waitlist: Vec<admission::Waiting>,
}
#[derive(Clone, Debug)]
pub struct Account {
    pub id: i64,
    pub name: String,
    pub password: String,
    pub sid: Option<SpielerId>,
    pub mode: String,
    pub banned: bool,
}
fn default_bot_period() -> i64 {
    7200
}
pub struct Game {
    pub world: Welt,
    pub runtime: Runtime,
    pub db: Connection,
    pub data_root: PathBuf,
    pub storage_error: Option<String>,
    local_tick_times: Vec<f64>,
    /// Last local time step and its carried game-second remainder: the view reports the tick
    /// age, so clients extrapolate from the real current time instead of jumping back each poll.
    pub(crate) local_tick_at: Option<(std::time::Instant, f64)>,
    /// Local runtime (cockpit singleplayer/LAN): bots answer commands at once, archived
    /// histories leave memory, and pure time steps checkpoint at a bounded rate.
    pub(crate) local: bool,
    /// Pure time steps write the full checkpoint at most this often (wall seconds).
    /// Commands and claims always checkpoint. 0 = every transaction (online server).
    pub(crate) tick_checkpoint_secs: u64,
    /// Last failed local time step (shown in the cockpit instead of disappearing silently).
    pub(crate) local_error: Option<String>,
    /// This local world is open for LAN guests.
    pub(crate) local_host: bool,
    last_checkpoint: std::time::Instant,
    checkpoint_written: bool,
    research_key: Option<(i64, bool)>,
    archive_pruned_day: i64,
    started: std::time::Instant,
    audit_context: Option<Value>,
    email_code_key: String,
    pub(crate) fairplay: std::cell::RefCell<fairplay::Stolperdraht>,
}
pub type Reply = Result<Value, (u16, String)>;
fn err(code: u16, text: &str) -> (u16, String) {
    (code, text.into())
}
fn str_field<'a>(v: &'a Value, k: &str) -> Result<&'a str, (u16, String)> {
    v[k].as_str()
        .ok_or_else(|| err(400, &format!("Feld {k} fehlt")))
}
fn account_from(row: &rusqlite::Row) -> rusqlite::Result<Account> {
    Ok(Account {
        id: row.get(0)?,
        name: row.get(1)?,
        password: row.get(2)?,
        sid: row.get(3)?,
        mode: row.get(4)?,
        banned: row.get(5)?,
    })
}
fn fresh(seed: u64, rules: &str) -> Result<(Welt, Runtime), String> {
    fresh_with(seed, Regelwerk::laden(rules)?, BOT_COUNT, SEATS)
}
fn fresh_with(seed: u64, mut r: Regelwerk, bot_count: u16, seats: u16) -> Result<(Welt, Runtime), String> {
    r.welt.fenster_sekunden = 1;
    let mut world = Welt::neu(r, seed, (bot_count + seats) as usize)?;
    world.ausbau_aktivieren()?;
    // Neue Welten starten in der Mondgeneration (STERNE13); laufende Welten bleiben unverändert.
    world.monde_aktivieren()?;
    // Invasion v1 (Bomber, 48-h-Belagerung, Allianzhilfe, Volksmiliz) nur für neue Welten.
    world.invasion_aktivieren()?;
    // Allianzen bis 50 Mitglieder und Dysonsphäre nur für neue Welten (docs/DYSONSPHAERE.md).
    world.allianz_v2_aktivieren()?;
    world.dyson_aktivieren()?;
    // Technik v1 (Antriebswechsel, Plasmatechnik; regeln/technik.ron) nur für neue Welten.
    world.technik_aktivieren()?;
    for sid in 0..world.spieler.len(){world.heimat_anpassen(sid as SpielerId);}
    for pid in 0..world.planeten.len(){world.raten_neu(pid);}
    let mut bots = BTreeMap::new();
    for sid in 0..bot_count {
        world.spieler[sid as usize].ki = false;
        let mut bot=Bot::neu(Bottyp::ALLE[sid as usize % 4], sid);bot.naechster=0;bots.insert(sid,bot);
    }
    for sid in bot_count..bot_count + seats {
        world.spieler[sid as usize].name = format!("Freier Startplatz {}", sid - bot_count + 1);
        world.startplatz_reservieren(sid)?;
    }
    Ok((
        world,
        Runtime {
            seat_capacity: seats,
            world_id: token(),
            tempo: 1,
            paused: false,
            bots,
            bots_enabled: true,
            bot_period_secs: 900,
            opferschutz: lauf::bots::Opferschutz::neue_welt(),
            revision: 0,
            leases: BTreeMap::new(),
            agent_status: BTreeMap::new(),
            bot_actions: 0,
            bot_rejected: 0,
            bot_trace: BTreeMap::new(),
            public_history: Vec::new(),
            maintenance: String::new(),
            admission_limit: 5,
            waitlist: Vec::new(),
        },
    ))
}
/// Opferschutz aus JSON übernehmen: `true`/`false` schaltet nur ein oder aus, ein Objekt setzt `aktiv`,
/// `pause_stunden` (1 bis 168) und `neulinge_tage` (0 bis 30); fehlende Felder bleiben wie bisher.
pub fn opferschutz_lesen(v: &Value, alt: lauf::bots::Opferschutz) -> Result<lauf::bots::Opferschutz, String> {
    let mut o = alt;
    if let Some(b) = v.as_bool() { o.aktiv = b; return Ok(o); }
    if !v.is_object() { return Err("opferschutz: true, false oder {aktiv, pause_stunden, neulinge_tage}".into()); }
    if let Some(b) = v.get("aktiv") { o.aktiv = b.as_bool().ok_or("opferschutz.aktiv: true oder false")?; }
    if let Some(n) = v.get("pause_stunden") { o.pause_stunden = n.as_u64().filter(|n| (1..=168).contains(n)).ok_or("opferschutz.pause_stunden: 1 bis 168")? as u32; }
    if let Some(n) = v.get("neulinge_tage") { o.neulinge_tage = n.as_u64().filter(|n| *n <= 30).ok_or("opferschutz.neulinge_tage: 0 bis 30")? as u32; }
    Ok(o)
}
impl Game {
    pub fn open(root: &Path, seed: u64) -> Result<Self, String> {
        Self::open_world(root, seed, None)
    }
    /// Lokale Welt wie in der App (Diagnosewerkzeuge, z. B. examples/bot_lage.rs).
    pub fn open_local(root: &Path, seed: u64, bots: u16) -> Result<Self, String> {
        Self::open_world(root, seed, Some(bots))
    }
    fn open_world(root: &Path, seed: u64, local_bots: Option<u16>) -> Result<Self, String> {
        std::fs::create_dir_all(root).map_err(|e| e.to_string())?;
        let db = Connection::open(root.join("spiel.sqlite3")).map_err(|e| e.to_string())?;
        db.busy_timeout(std::time::Duration::from_secs(5))
            .map_err(|e| e.to_string())?;
        db.execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=FULL; PRAGMA foreign_keys=ON;
            CREATE TABLE IF NOT EXISTS checkpoint(id INTEGER PRIMARY KEY CHECK(id=1),version INTEGER NOT NULL,world BLOB NOT NULL,runtime BLOB NOT NULL);
            CREATE TABLE IF NOT EXISTS accounts(id INTEGER PRIMARY KEY,name TEXT NOT NULL COLLATE NOCASE UNIQUE,password TEXT NOT NULL,sid INTEGER UNIQUE,mode TEXT NOT NULL,banned INTEGER NOT NULL DEFAULT 0);
            CREATE TABLE IF NOT EXISTS sessions(hash TEXT PRIMARY KEY,account INTEGER NOT NULL REFERENCES accounts(id),expires INTEGER NOT NULL);
            CREATE TABLE IF NOT EXISTS commands(account INTEGER NOT NULL REFERENCES accounts(id),world_id TEXT NOT NULL,request_id TEXT NOT NULL,body_hash TEXT NOT NULL,result TEXT NOT NULL,created INTEGER NOT NULL,PRIMARY KEY(account,world_id,request_id));
            CREATE TABLE IF NOT EXISTS audit(id INTEGER PRIMARY KEY,time INTEGER NOT NULL,account INTEGER,kind TEXT NOT NULL,result TEXT NOT NULL);
            CREATE TABLE IF NOT EXISTS checkpoint_info(id INTEGER PRIMARY KEY CHECK(id=1),world_id TEXT NOT NULL,simulation_seconds INTEGER NOT NULL,saved_at INTEGER NOT NULL,world_bytes INTEGER NOT NULL,runtime_bytes INTEGER NOT NULL,revision INTEGER NOT NULL);").map_err(|e|e.to_string())?;
        if local_bots.is_some() {
            // Large checkpoints pass through the WAL; keep the file from staying at its peak.
            db.execute_batch("PRAGMA journal_size_limit=67108864;").map_err(|e| e.to_string())?;
        }
        let saved: Option<(u32, Vec<u8>, Vec<u8>)> = db
            .query_row(
                "SELECT version,world,runtime FROM checkpoint WHERE id=1",
                [],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .optional()
            .map_err(|e| e.to_string())?;
        let (world, runtime) = if let Some((version, w, r)) = saved {
            if version != API_VERSION {
                return Err("Unbekannte Server-Datenversion; Originaldaten bleiben erhalten".into());
            }
            (
                Welt::aus_bytes_fuer_spiel(&w)?,
                serde_json::from_slice(&r).map_err(|e| e.to_string())?,
            )
        } else {
            if let Some(bots) = local_bots {
                let mut r = Regelwerk::laden(include_str!("../../../regeln/online-v1.ron"))?;
                let (sectors, systems) = local::dimensions(bots)?;
                r.welt.sektoren = sectors;
                r.welt.systeme_je_sektor = systems;
                r.welt.start_abstand = 1;
                r.hash = digest(&format!("{}:local-v2:{sectors}:{systems}:{bots}", r.hash));
                let seats = local::seat_capacity(bots)?;
                let (mut w, mut rt) = fresh_with(seed, r, bots, seats)?;
                // HTTP actions are immediate; the event queue still processes every due event.
                std::sync::Arc::make_mut(&mut w.regeln).welt.fenster_sekunden = 30;
                rt.admission_limit = 32.min(seats);
                rt.tempo = 30;
                (w, rt)
            } else { fresh(seed, include_str!("../../../regeln/online-v1.ron"))? }
        };
        communication::setup(&db)?;
        workspace::setup(&db)?;
        akten::setup(&db)?;
        agenten::setup(&db)?;
        let email_code_key=identity::setup(&db,root)?;
        authenticator::setup(&db)?;
        harness::setup(&db)?;
        uebergabe::setup(&db)?;
        spielerart::setup(&db)?;
        let fairplay = std::cell::RefCell::new(fairplay::setup(&db, root)?);
        mail::MailConfig::initialize(root)?;
        let mut g = Self {
            world,
            runtime,
            db,
            data_root: root.into(),
            storage_error: None,
            local_tick_times: Vec::new(),
            local_tick_at: None,
            local: local_bots.is_some(),
            tick_checkpoint_secs: if local_bots.is_some() { LOCAL_TICK_CHECKPOINT_SECS } else { 0 },
            local_error: None,
            local_host: false,
            last_checkpoint: std::time::Instant::now(),
            checkpoint_written: false,
            research_key: None,
            archive_pruned_day: -1,
            started: std::time::Instant::now(),
            audit_context: None,
            email_code_key,
            fairplay,
        };
        // No downtime catch-up and no model command survives a process restart.
        g.runtime.leases.clear();
        // 20.10.2026: Spielerart aller Plätze erfassen; Freigaben, die der Neustart gekappt hat, im Kontoprotokoll beenden.
        g.spielerart_start().map_err(|e| e.1)?;
        g.world.industrie_aktivieren();
        g.archive_tail_after_checkpoint_remove()?;
        // Explicit content upgrade retains account, world identity, orders and every
        // legacy ship/defense count. The versioned V11 snapshot stores the new layout.
        // Keep the saved generation and rule identity when resuming.
        g.save().map_err(|e| e.1)?;
        Ok(g)
    }
    /// Crash recovery. Pure time steps may commit their archive rows without a new checkpoint
    /// (`tick_checkpoint_secs`). After a crash the world resumes from the last checkpoint and
    /// re-simulates those seconds deterministically; their earlier rows must not stay twice,
    /// and the communication primary keys would otherwise block every later save.
    fn archive_tail_after_checkpoint_remove(&self) -> Result<(), String> {
        let wid = &self.runtime.world_id;
        let t = self.world.zeit;
        self.db
            .execute(
                "DELETE FROM game_actions WHERE world_id=?1 AND simulation_seconds>?2",
                params![wid, t],
            )
            .and_then(|_| {
                self.db.execute(
                    "DELETE FROM game_domain_events WHERE world_id=?1 AND simulation_seconds>?2",
                    params![wid, t],
                )
            })
            .and_then(|_| {
                self.db.execute(
                    "DELETE FROM communication_events WHERE world_id=?1 AND event_id>?2",
                    params![wid, self.world.kommunikation.ereignisse_gesamt()],
                )
            })
            .and_then(|_| {
                self.db.execute(
                    "DELETE FROM communication_messages WHERE world_id=?1 AND message_id>?2",
                    params![wid, self.world.nachrichten.len() as u64],
                )
            })
            .map(|_| ())
            .map_err(|e| format!("Archiv nach Absturz nicht bereinigt: {e}"))
    }
    fn save(&mut self) -> Result<(), (u16, String)> {
        self.save_archive()?;
        self.snapshot_players()?;
        let w = self.world.zu_bytes();
        let r = serde_json::to_vec(&self.runtime)
            .map_err(|_| err(500, "Checkpoint nicht serialisierbar"))?;
        // checkpoint_info is the readable twin of the binary checkpoint for tools (Python
        // reports, admins): game time, size and revision without decoding bincode.
        let result=self.db.execute("INSERT INTO checkpoint VALUES(1,?1,?2,?3) ON CONFLICT(id) DO UPDATE SET version=excluded.version,world=excluded.world,runtime=excluded.runtime",params![API_VERSION,w,r])
            .and_then(|_|self.db.execute("INSERT INTO checkpoint_info VALUES(1,?1,?2,?3,?4,?5,?6) ON CONFLICT(id) DO UPDATE SET world_id=excluded.world_id,simulation_seconds=excluded.simulation_seconds,saved_at=excluded.saved_at,world_bytes=excluded.world_bytes,runtime_bytes=excluded.runtime_bytes,revision=excluded.revision",
                params![self.runtime.world_id,self.world.zeit,now(),w.len() as i64,r.len() as i64,self.runtime.revision as i64]));
        match result {
            Ok(_) => {self.checkpoint_written=true;Ok(())},
            Err(_) => {
                self.storage_error = Some("Speicherung fehlgeschlagen; Welt angehalten".into());
                self.runtime.paused = true;
                Err(err(503, "Speicherung fehlgeschlagen; Welt angehalten"))
            }
        }
    }
    /// Archive part of a save: communication, domain events and (local) history retention.
    /// Runs in every transaction, also when a pure time step skips the full checkpoint.
    fn save_archive(&mut self) -> Result<(), (u16, String)> {
        self.save_communication()?;
        self.world.fachereignisse.clear();
        if self.local {
            // Everything up to here is archived in this transaction. Memory keeps the current
            // and the previous game day of events (bot initiative checks need today), battle
            // reports and daily values of the last 30 days (the view shows recent reports).
            let tag = self.world.zeit - self.world.zeit.rem_euclid(TAG);
            let archiviert = self.world.kommunikation.ereignisse_gesamt();
            self.world.verlauf_auslagern(tag - TAG, archiviert);
            self.world.berichte_auslagern(self.world.zeit - LOCAL_REPORT_DAYS * TAG);
            if self.archive_pruned_day != self.world.zeit / TAG {
                let (wid, grenze) = (&self.runtime.world_id, self.world.zeit - LOCAL_ARCHIVE_DAYS * TAG);
                self.db
                    .execute("DELETE FROM game_actions WHERE world_id=?1 AND simulation_seconds<?2", params![wid, grenze])
                    .and_then(|_| self.db.execute("DELETE FROM game_domain_events WHERE world_id=?1 AND simulation_seconds<?2", params![wid, grenze]))
                    .map_err(|_| err(503, "Altes Spielprotokoll konnte nicht gekürzt werden"))?;
                self.archive_pruned_day = self.world.zeit / TAG;
            }
        }
        Ok(())
    }
    fn checkpoint_due(&self) -> bool {
        self.tick_checkpoint_secs == 0
            || self.last_checkpoint.elapsed().as_secs() >= self.tick_checkpoint_secs
    }
    /// Writes a complete checkpoint now (end of a local session, before a backup).
    pub fn checkpoint_now(&mut self) -> Reply {
        self.transaction(|_| Ok(json!({"ok":true})))
    }
    fn transaction<F>(&mut self, f: F) -> Reply
    where
        F: FnOnce(&mut Self) -> Reply,
    {
        self.transaction_with(true, f)
    }
    /// `checkpoint=false` for transactions that only advance time or change transient
    /// runtime data (leases, sessions, agent reports): they write their archive rows, but the
    /// full checkpoint only when `checkpoint_due()`. A crash then loses at most those seconds,
    /// which `archive_tail_after_checkpoint_remove` reconciles on the next start.
    fn transaction_with<F>(&mut self, checkpoint: bool, f: F) -> Reply
    where
        F: FnOnce(&mut Self) -> Reply,
    {
        if self.storage_error.is_some() {
            return Err(err(503, "Speicherung gestört; zuerst Datenbank prüfen"));
        }
        let w = self.world.clone();
        let r = self.runtime.clone();
        // Sicherheitsnetz (Karl 27.10.2026, App-Softlock nach schnellem Bauen): Hängt noch eine Transaktion offen, würde
        // jedes BEGIN scheitern und kein Befehl käme mehr durch – bis zum Neustart. Offene Reste zurückrollen.
        if !self.db.is_autocommit() {
            eprintln!("Offene Spieltransaktion vor neuem Befehl zurückgerollt");
            let _ = self.db.execute_batch("ROLLBACK");
        }
        self.db
            .execute_batch("BEGIN IMMEDIATE")
            .map_err(|_| err(503, "Datenbank belegt"))?;
        self.checkpoint_written = false;
        // A failed event must restore both the world and its SQL transaction. Otherwise
        // one panicking completion kills the only game worker and all later HTTP calls.
        // Der Schutz umfasst auch die Nachphase (Ereignisse, Spielerart, Speichern): eine Panik dort ließ die Transaktion
        // offen, danach scheiterte jedes BEGIN IMMEDIATE („Datenbank belegt“) – der lokale Server nahm keinen Befehl mehr an.
        let result=std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| -> Reply {
            let v = f(self)?;
            if let Some(context)=&self.audit_context {
                self.db.execute("INSERT INTO audit(time,account,kind,result) VALUES(?1,NULL,'admin',?2)",params![now(),json!({"context":context,"revision":self.runtime.revision+1,"world_id":self.runtime.world_id}).to_string()]).map_err(|_|err(503,"Admin-Audit konnte nicht gespeichert werden"))?;
            }
            self.runtime.revision+=1; let logs=self.world.log_abholen();self.record_player_events(logs)?;
            if self.runtime.world_id==r.world_id && self.world.zeit>w.zeit {
                self.db.execute("INSERT INTO simulation_clock(world_id,sim_from,sim_to,wall_end,tempo) VALUES(?1,?2,?3,?4,?5)",
                    params![self.runtime.world_id,w.zeit,self.world.zeit,now(),r.tempo.max(1)]).map_err(|_|err(503,"Weltuhr nicht speicherbar"))?;
            }
            // 20.10.2026: Wechsel der Spielerart und Agentenfreigaben im Kontoprotokoll nachführen (gleiche Transaktion).
            self.spielerart_abgleichen()?;
            if checkpoint || self.checkpoint_due() {self.save()?;} else {self.save_archive()?;}
            Ok(v)
        })).unwrap_or_else(|_| {
            eprintln!("Spieltransaktion zurückgerollt: unerwarteter Verarbeitungsfehler");
            Err(err(503,"Verarbeitung wurde sicher zurückgesetzt; bitte erneut versuchen"))
        });
        if result.is_ok() && self.db.execute_batch("COMMIT").is_ok() {
            if self.checkpoint_written {
                self.last_checkpoint = std::time::Instant::now();
            }
            self.checkpoint_written = false;
            return result;
        }
        self.checkpoint_written = false;
        let _ = self.db.execute_batch("ROLLBACK");
        self.world = w;
        self.runtime = r;
        if self.storage_error.is_some() {
            self.runtime.paused = true;
        }
        if result.is_ok() {
            self.storage_error = Some("Commit fehlgeschlagen".into());
            self.runtime.paused = true;
            return Err(err(503, "Speicherung fehlgeschlagen"));
        }
        result
    }
    pub fn advance(&mut self, seconds: u32) -> Reply {
        if self.runtime.paused || self.world.beendet() {
            return Ok(json!({"paused":true}));
        }
        self.transaction_with(self.local, |g| {
            g.advance_steps(seconds.min(3600))
        })
    }
    /// Local round: script bots answer letters, requests and invitations at once, inside the
    /// transaction of the triggering command (one checkpoint, no half-answered state after a
    /// crash). Same policy for browser, harness and local agent commands.
    fn bots_react(&mut self) {
        if !self.runtime.bots_enabled {
            return;
        }
        let scripts: std::collections::BTreeSet<SpielerId> =
            self.runtime.bots.keys().copied().collect();
        let bots: Vec<(SpielerId, Bottyp)> =
            self.runtime.bots.iter().map(|(sid, b)| (*sid, b.typ)).collect();
        for (sid, typ) in bots {
            if !self.world.spieler_aktiv(sid) {
                continue;
            }
            let (a, r) = {
                let s = &self.world.spieler[sid as usize].statistik;
                (s.aktionen, s.abgelehnt)
            };
            lauf::bots::reagieren_mit(&mut self.world, sid, typ, &scripts);
            let s = &self.world.spieler[sid as usize].statistik;
            self.runtime.bot_actions += (s.aktionen - a) as u64;
            self.runtime.bot_rejected += (s.abgelehnt - r) as u64;
        }
    }
    // Shared by transactional live ticks and explicit offline replay tests.
    fn advance_steps(&mut self, seconds: u32) -> Reply {
            let target = self.world.zeit + seconds.min(3600) as i64;
            let script_players=self.runtime.bots.keys().copied().collect();
            while self.world.zeit < target {
                if !self.world.schritt_bis(target) {
                    break;
                }
                let logs=self.world.log_abholen();self.record_player_events(logs)?;
                if self.runtime.bots_enabled {
                    let threatened:std::collections::BTreeSet<_>=self.world.kolonisation.besetzungen.iter()
                        .filter(|(id,_)|self.world.ausbau.boden.miliz.get(id).is_none_or(|m|m.personen.gesamt()==0))
                        .map(|(_,occupation)|occupation.verteidiger).collect();
                    let sids:Vec<_>=self.runtime.bots.iter().filter(|(sid,b)|b.naechster<=self.world.zeit||threatened.contains(sid)).map(|(sid,_)|*sid).collect();
                    let schutz=self.runtime.opferschutz;
                    // Mitspieler (aktive Reiche ohne Skriptbot) genießen die Fairnessregeln der Bots (lauf::bots, Mitspieler).
                    let menschen=self.mitspieler();
                    lauf::bots::mit_mitspielern(&menschen, || -> Result<(), (u16, String)> {
                    for sid in sids {
                        if !self.world.spieler_aktiv(sid) {continue;}
                        let bot=self.runtime.bots.get_mut(&sid).unwrap();
                        bot.opferschutz=schutz;
                        let due = bot.naechster <= self.world.zeit;
                        let prev = self.world.spieler[sid as usize].statistik.clone();
                        lauf::bots::zug(&mut self.world, sid, bot);
                        lauf::bots::reagieren_mit(&mut self.world,sid,bot.typ,&script_players);
                        if due {
                            bot.naechster = self.world.zeit + self.runtime.bot_period_secs;
                        }
                        let next = &self.world.spieler[sid as usize].statistik;
                        self.runtime.bot_actions += (next.aktionen - prev.aktionen) as u64;
                        self.runtime.bot_rejected += (next.abgelehnt - prev.abgelehnt) as u64;
                        if due {self.record_bot_turn(sid)?;}
                    }
                    Ok(())
                    })?;
                }
                let logs=self.world.log_abholen();self.record_player_events(logs)?;
                if self.world.zeit % 60 == 0 {
                    // AI players were already serviced by the core communication tick.
                    let ids=self.runtime.bots.keys().chain(self.runtime.leases.keys()).copied().filter(|sid|!self.world.spieler[*sid as usize].ki).collect::<std::collections::BTreeSet<_>>();
                    for sid in ids {for id in self.world.offene_antworten(sid) {self.world.empfang_bei_zeitmangel(sid,id);}}
                }
                if self.world.zeit % STUNDE == 0 {self.record_public_history();}
            }
            self.runtime.leases.retain(|sid, l| l.until > now() && self.world.spieler_aktiv(*sid));
            Ok(json!({"sekunden":self.world.zeit}))
    }
    /// Aktive Reiche, die kein Skriptbot steuert (Menschen und ihre Agenten): für die Fairnessregeln der Bots.
    pub fn mitspieler(&self) -> std::collections::BTreeSet<SpielerId> {
        self.world.spieler.iter().map(|s| s.id).filter(|sid| self.world.spieler_aktiv(*sid) && !self.runtime.bots.contains_key(sid)).collect()
    }
    pub fn account_by_name(&self, name: &str) -> Option<Account> {
        self.db
            .query_row(
                "SELECT id,name,password,sid,mode,banned FROM accounts WHERE name=?1",
                [name],
                account_from,
            )
            .optional()
            .ok()
            .flatten()
    }
    pub fn authenticate(&self, raw: &str) -> Result<Account, (u16, String)> {
        if raw.len() != 64 {
            return Err(err(401, "Anmeldung erforderlich"));
        }
        let a=self.db.query_row("SELECT a.id,a.name,a.password,a.sid,a.mode,a.banned FROM accounts a JOIN sessions s ON s.account=a.id WHERE s.hash=?1 AND s.expires>?2",params![digest(raw),now()],account_from).optional().map_err(|_|err(503,"Datenbank nicht lesbar"))?.ok_or_else(||err(401,"Sitzung abgelaufen"))?;
        if a.banned {
            return Err(err(403, "Konto gesperrt"));
        }
        Ok(a)
    }
    fn session(&mut self, a: &Account) -> Reply {
        self.db.execute("UPDATE accounts SET last_login_at=?1,login_count=login_count+1 WHERE id=?2",params![now(),a.id]).map_err(|_|err(503,"Anmeldezeit nicht gespeichert"))?;
        self.identity_event(Some(a.id),"login",json!({}))?;
        let raw = token();
        let expires = now() + 7 * 86400;
        self.db
            .execute("DELETE FROM sessions WHERE expires<=?1", [now()])
            .map_err(|_| err(503, "Datenbankfehler"))?;
        self.db
            .execute(
                "INSERT INTO sessions VALUES(?1,?2,?3)",
                params![digest(&raw), a.id, expires],
            )
            .map_err(|_| err(503, "Sitzung nicht gespeichert"))?;
        Ok(
            json!({"token":raw,"name":a.name,"mode":a.mode,"spieler":a.sid,"expires":expires,"world_id":self.runtime.world_id,"api_version":API_VERSION}),
        )
    }
    pub fn register(&mut self, v: &Value, hashed: String) -> Reply {
        // Internal helper for game tests; HTTP registration uses register_email.
        let name = self.validated_account_name(v)?;
        self.transaction(|g| {
            g.db.execute(
                "INSERT INTO accounts(name,password,mode,created_at) VALUES(?1,?2,'zuschauer',?3)",
                params![name, hashed,now()],
            )
            .map_err(|_| err(409, "Name bereits vergeben"))?;
            let a = g
                .account_by_name(name)
                .ok_or_else(|| err(503, "Konto konnte nicht gelesen werden"))?;
            g.identity_event(Some(a.id),"registered",json!({"method":"internal"}))?;
            g.session(&a)
        })
    }
    pub fn login_verified(&mut self, id: i64) -> Reply {
        let a = self
            .db
            .query_row(
                "SELECT id,name,password,sid,mode,banned FROM accounts WHERE id=?1",
                [id],
                account_from,
            )
            .map_err(|_| err(401, "Anmeldung fehlgeschlagen"))?;
        if a.banned {
            return Err(err(403, "Konto gesperrt"));
        }
        // Sessions live in their own table; the world and its checkpoint are unchanged.
        self.transaction_with(false, |g| g.session(&a))
    }
    pub fn lobby(&self) -> Value {
        json!({"api_version":API_VERSION,"world_id":self.runtime.world_id,"revision":self.runtime.revision,"sekunden":self.world.zeit,
          "paused":self.runtime.paused,"tempo":self.runtime.tempo,"beendet":self.world.beendet(),"bots":self.runtime.bots.len(),"freie_plaetze":self.world.aufklaerung.inaktive_spieler.len(),"plaetze":self.runtime.seat_capacity,"freigegebene_plaetze":self.runtime.admission_limit,"freie_zugaenge":self.available_admissions(),"wartende":self.runtime.waitlist.len(),
            "rangliste":self.world.rangliste(),"ausgeschiedene":self.world.ausgeschiedene(),"aktive_reiche":self.world.spieler.iter().filter(|sp|self.world.spieler_aktiv(sp.id)).count(),"ausscheiden_regeln":self.world.ausscheiden.regeln,"regeln":self.world.regeln.version,"maintenance":self.runtime.maintenance,
            "epoche_tage":self.world.regeln.welt.epoche_tage,"verlauf":self.runtime.public_history.iter().rev().take(48).cloned().collect::<Vec<_>>(),"fairplay":self.fairplay_oeffentlich()})
    }
    fn sid(a: &Account) -> Result<SpielerId, (u16, String)> {
        a.sid
            .ok_or_else(|| err(403, "Zuschauer haben keinen Spielplatz"))
    }
    pub fn view(&self, a: &Account, role: Rolle) -> Reply {
        let sid = Self::sid(a)?;
        let mut v = self.world.sicht(sid, role);
        v["world_id"] = json!(self.runtime.world_id);
        v["revision"] = json!(self.runtime.revision);
        v["paused"] = json!(self.runtime.paused);
        v["beendet"] = json!(self.world.beendet());
        // Local worlds step every 5 s: the age of the last step lets the clock run on without jumping back.
        if let Some((at, rest)) = self.local_tick_at.filter(|_| !self.runtime.paused && self.runtime.tempo > 0) {
            v["tick_alter_ms"] = json!(((at.elapsed().as_secs_f64() + rest / self.runtime.tempo as f64) * 1000.0).round() as i64);
        }
        v["mode"] = json!(a.mode);
        // Schonzeit und Raubpause gegenüber den Skriptbots (Fairnessregeln, lauf::bots::schutz_sicht).
        v["bot_schutz"] = lauf::bots::schutz_sicht(&self.world, &self.runtime.opferschutz, sid, !self.runtime.bots.contains_key(&sid));
        Self::inbox_ids(&mut v);
        self.inbox_times(a.id, &mut v)?;
        v["postfach_gelesen"] = self.inbox_reads(a.id)?;
        v["gegner_notizen"] = self.gegner_notizen(a.id)?;
        // Nur der Browserfeed trägt unsichtbare Fair-Play-Köder; die Harness-API nie.
        if role == Rolle::Alle { v["oberflaeche"] = self.oberflaeche(Some(a.id), "reich"); }
        v["agent"] = json!(self
            .runtime
            .leases
            .get(&sid)
            .filter(|l| l.until > now())
            .map(|l| json!({"roles":l.roles,"until":l.until})));
        Ok(v)
    }
    /// Postfach-Arten mit Pfad in der Sicht. Verschachtelte Mondberichte gehören dazu (vorher nie im Postfach).
    const POSTFACH: [(&'static str, &'static str); 9] = [("systemberichte",""),("berichte",""),("flottenberichte",""),("kampfberichte",""),("ereignisse",""),
        ("chronik",""),("bodenberichte",""),("spionageberichte","monde"),("phalanxberichte","monde")];
    fn inbox_ids(v:&mut Value) {
        for (kind, eltern) in Self::POSTFACH {
            let rows = if eltern.is_empty() { v[kind].as_array_mut() } else { v[eltern][kind].as_array_mut() };
            if let Some(rows)=rows {
                for row in rows {
                    if !row.is_object(){continue;}
                    // Berichte tragen eine feste Kennung aus Art, Ort und Zeitpunkt (bzw. Kampfnummer): Live-Felder wie
                    // `aufklaerung` oder neue Berichtsfelder ändern sie nicht mehr – vorher galt ein gelesener
                    // Spionagebericht jede Spielstunde wieder als neu. Meldungen (ereignisse, chronik) hashen weiter
                    // ihren Inhalt, damit ihr Lesestatus erhalten bleibt.
                    let r = &*row;
                    let s = |k: &str| r[k].to_string();
                    let schluessel = match kind {
                        "systemberichte" => format!("system:{}:{}",s("ziel"),s("zeit")),
                        "kampfberichte" => format!("kampf:{}", s("nummer")),
                        "berichte" => format!("bericht:{}:{}:{}", r["art"].as_str().unwrap_or("planet"), s("ziel"), s("zeit")),
                        "flottenberichte" => format!("flotte:{}:{}", s("flotte"), s("zeit")),
                        "bodenberichte" => format!("boden:{}:{}:{}", s("planet"), s("zeit"), s("angreifer_name")),
                        "spionageberichte" => format!("mondspionage:{}:{}", s("ziel"), s("zeit")),
                        "phalanxberichte" => format!("phalanx:{}", s("nr")),
                        _ => {
                            let mut stable=row.clone();
                            if let Some(m)=stable.as_object_mut() {
                                for key in ["alter_sekunden","alter_stunden","bericht_alter_stunden","_postfach_id"] {m.remove(key);}
                            }
                            format!("{kind}:{stable}")
                        }
                    };
                    row["_postfach_id"]=json!(digest(&schluessel));
                }
            }
        }
    }
    fn inbox_times(&self, account:i64, v:&mut Value) -> Result<(),(u16,String)> {
        // Einmal zugeordnete Echtzeiten bleiben bei Tempowechsel und Neuladen stabil.
        let jetzt=now();let sim=self.world.zeit;let tempo=self.runtime.tempo.max(1) as i64;
        let mut stmt=self.db.prepare("SELECT report_id,received_at FROM report_times WHERE world_id=?1 AND account=?2")
            .map_err(|_|err(503,"Berichtszeiten nicht lesbar"))?;
        let mut zeiten:std::collections::BTreeMap<String,i64>=stmt.query_map(params![self.runtime.world_id,account],|r|Ok((r.get(0)?,r.get(1)?)))
            .map_err(|_|err(503,"Berichtszeiten nicht lesbar"))?.collect::<Result<_,_>>().map_err(|_|err(503,"Berichtszeiten nicht lesbar"))?;
        drop(stmt);
        for (kind,parent) in Self::POSTFACH {
            let rows=if parent.is_empty(){v[kind].as_array_mut()}else{v[parent][kind].as_array_mut()};
            if let Some(rows)=rows { for row in rows {
                let Some(id)=row["_postfach_id"].as_str().map(str::to_owned) else{continue};
                if let Some(at)=zeiten.get(&id) {row["echtzeit"]=json!(at);continue;}
                let zeit=row["zeit"].as_i64().or_else(||row["zeit"].as_str().and_then(|s|{
                    let z:Vec<i64>=s.split(|c:char|!c.is_ascii_digit()).filter(|s|!s.is_empty()).filter_map(|s|s.parse().ok()).collect();
                    (z.len()>=3).then(||(z[0]-1)*86400+z[1]*3600+z[2]*60+z.get(3).copied().unwrap_or(0))
                })).unwrap_or(sim);
                let ankunft=self.db.query_row("SELECT wall_end-(sim_to-?2)/tempo FROM simulation_clock WHERE world_id=?1 AND sim_from<=?2 AND sim_to>=?2 ORDER BY sim_to LIMIT 1",
                    params![self.runtime.world_id,zeit],|r|r.get::<_,i64>(0)).optional().map_err(|_|err(503,"Weltuhr nicht lesbar"))?
                    .unwrap_or_else(||jetzt.saturating_sub((sim-zeit).max(0)/tempo));
                self.db.execute("INSERT OR IGNORE INTO report_times VALUES(?1,?2,?3,?4)",
                    params![self.runtime.world_id,account,id,ankunft]).map_err(|_|err(503,"Berichtszeiten nicht speicherbar"))?;
                zeiten.insert(id,ankunft);row["echtzeit"]=json!(ankunft);
            }}
        }
        Ok(())
    }
    fn inbox_reads(&self, account:i64) -> Result<Value,(u16,String)> {
        let mut stmt=self.db.prepare("SELECT report_id FROM inbox_reads WHERE world_id=?1 AND account=?2 ORDER BY read_at DESC LIMIT 2000")
            .map_err(|_|err(503,"Postfachstatus nicht lesbar"))?;
        let ids=stmt.query_map(params![self.runtime.world_id,account],|r|r.get::<_,String>(0))
            .map_err(|_|err(503,"Postfachstatus nicht lesbar"))?.collect::<Result<Vec<_>,_>>()
            .map_err(|_|err(503,"Postfachstatus nicht lesbar"))?;
        Ok(json!(ids))
    }
    /// Reading post is independent of game pause, military roles and agent leases.
    pub fn inbox_read(&mut self,a:&Account,v:&Value)->Reply {
        self.world_matches(v)?;let sid=Self::sid(a)?;
        if let Some(id)=v["brief"].as_u64() {
            return self.transaction(|g| {
                g.world.brief_lesen(sid,id).map_err(|e|err(403,&e))?;
                Ok(json!({"brief":id,"gelesen":true}))
            });
        }
        let id=v["bericht"].as_str().filter(|s|s.len()==64&&s.bytes().all(|b|b.is_ascii_hexdigit()))
            .ok_or_else(||err(400,"Gültige Berichtskennung fehlt"))?;
        let mut own=self.world.sicht(sid,Rolle::Alle);Self::inbox_ids(&mut own);
        let allowed=Self::POSTFACH.iter()
            .any(|(kind,eltern)|(if eltern.is_empty(){&own[*kind]}else{&own[*eltern][*kind]}).as_array().is_some_and(|rows|rows.iter().any(|r|r["_postfach_id"]==id)));
        if !allowed {return Err(err(403,"Bericht gehört nicht zu deinem Postfach"));}
        self.db.execute("INSERT INTO inbox_reads VALUES(?1,?2,?3,?4) ON CONFLICT(world_id,account,report_id) DO UPDATE SET read_at=excluded.read_at",
            params![self.runtime.world_id,a.id,id,now()]).map_err(|_|err(503,"Lesestatus nicht gespeichert"))?;
        self.db.execute("DELETE FROM inbox_reads WHERE world_id=?1 AND account=?2 AND report_id NOT IN(SELECT report_id FROM inbox_reads WHERE world_id=?1 AND account=?2 ORDER BY read_at DESC LIMIT 2000)",
            params![self.runtime.world_id,a.id]).map_err(|_|err(503,"Postfachstatus nicht gespeichert"))?;
        Ok(json!({"bericht":id,"gelesen":true}))
    }
    pub fn tool(&self, a: &Account, v: &Value) -> Reply {
        self.tool_als(a, Rolle::Alle, v)
    }
    /// Lesende Abfrage in einer Regierungsrolle (Harness/MCP): Vorschauen prüfen wie der echte Befehl dieser Rolle.
    pub fn tool_als(&self, a: &Account, rolle: Rolle, v: &Value) -> Reply {
        let sid = Self::sid(a)?;
        let mut result=self.world.werkzeug_als(sid, rolle, v).map_err(|e| err(400, &e))?;
        if matches!(v["typ"].as_str(),Some("regelkontext"|"wirtschaftsdiagnose"|"kostenplan"|"ablaufplan"|"missionsplan"|"marktplan")) {
            result["world_id"]=json!(self.runtime.world_id);
        }
        Ok(result)
    }
    fn world_matches(&self, v: &Value) -> Result<(), (u16, String)> {
        if v["world_id"].as_str() != Some(&self.runtime.world_id) {
            return Err(err(409, "Welt wurde ersetzt; Ansicht neu laden"));
        }
        Ok(())
    }
    pub fn lease(&mut self, a: &Account, v: &Value) -> Reply {
        let sid = Self::sid(a)?;
        self.world_matches(v)?;
        // Leases never survive a restart (cleared on open), so they need no extra checkpoint.
        self.transaction_with(false, |g| {
            let action = str_field(v, "action")?;
            if action == "stop" {
                if let Some(raw) = v["lease"].as_str() {
                    if g.runtime
                        .leases
                        .get(&sid)
                        .is_some_and(|l| l.hash != digest(raw))
                    {
                        return Err(err(409, "Diese Agentfreigabe wurde schon ersetzt"));
                    }
                }
                g.runtime.leases.remove(&sid);
                return Ok(json!({"stopped":true}));
            }
            if !g.world.spieler_aktiv(sid) {return Err(err(409,"Reich besiegt; bis zur nächsten Epoche nur zuschauen"));}
            if action == "heartbeat" {
                harness::maschine_erlaubt(a)?;
                let l = g
                    .runtime
                    .leases
                    .get_mut(&sid)
                    .filter(|l| {
                        l.account == a.id
                            && l.until > now()
                            && Some(l.hash.as_str()) == v["lease"].as_str().map(digest).as_deref()
                    })
                    .ok_or_else(|| err(409, "Agentfreigabe abgelaufen"))?;
                l.until = now() + 90;
                return Ok(json!({"until":l.until}));
            }
            if action != "start" {
                return Err(err(400, "Unbekannter Agentbefehl"));
            }
            if a.mode == "mensch" || a.mode == "zuschauer" {
                return Err(err(403, "Zuerst Agent- oder Mischmodus wählen"));
            }
            if g.runtime.paused || g.world.beendet() {
                return Err(err(409, "Welt angehalten oder beendet"));
            }
            if g.runtime.leases.get(&sid).is_some_and(|l| l.until > now()) {
                return Err(err(409, "Ein Agent steuert dieses Reich bereits"));
            }
            let roles: Vec<Rolle> = serde_json::from_value(v["roles"].clone())
                .map_err(|_| err(400, "Rollenliste fehlt"))?;
            if roles.is_empty() || roles.len() > 4 || roles.contains(&Rolle::Alle) {
                return Err(err(400, "Eine bis vier Regierungsrollen wählen"));
            }
            let raw = token();
            let until = now() + 90;
            g.runtime.leases.insert(
                sid,
                Lease {
                    account: a.id,
                    hash: digest(&raw),
                    roles,
                    until,
                    token: None,
                },
            );
            Ok(json!({"lease":raw,"until":until,"world_id":g.runtime.world_id}))
        })
    }
    pub fn command(&mut self, a: &Account, v: &Value) -> Reply {
        let sid = Self::sid(a)?;
        if v.get("lease").is_some() { harness::maschine_erlaubt(a)?; }
        self.world_matches(v)?;
        let request = str_field(v, "request_id")?;
        if request.is_empty() || request.len() > 80 {
            return Err(err(400, "Ungültige Befehlskennung"));
        }
        let body_hash = digest(&v.to_string());
        let existing:Option<(String,String)>=self.db.query_row("SELECT body_hash,result FROM commands WHERE account=?1 AND world_id=?2 AND request_id=?3",params![a.id,self.runtime.world_id,request],|r|Ok((r.get(0)?,r.get(1)?))).optional().map_err(|_|err(503,"Datenbankfehler"))?;
        if let Some((h, result)) = existing {
            if h != body_hash {
                return Err(err(
                    409,
                    "Befehlskennung schon für anderen Inhalt verwendet",
                ));
            }
            return serde_json::from_str(&result)
                .map_err(|_| err(503, "Gespeicherte Antwort defekt"));
        }
        if self.runtime.paused || self.world.beendet() {
            return Err(err(409, "Welt angehalten oder beendet"));
        }
        if !self.world.spieler_aktiv(sid) {return Err(err(409,"Reich besiegt; bis zur nächsten Epoche nur zuschauen"));}
        let actions = v["aktionen"]
            .as_array()
            .filter(|a| a.len() <= 8 && (!a.is_empty() || v.get("lease").is_some()))
            .ok_or_else(|| err(400, "Höchstens acht Aktionen erlaubt"))?;
        // Gleiche Vertrauensgrenze für Browser und Harness: keine fremden Felder,
        // keine unsichtbaren Zeichen in Texten, die andere Spieler oder Modelle lesen.
        for (i, action) in actions.iter().enumerate() {
            eingabe::aktion_pruefen(action, i + 1).map_err(|e| err(400, &e))?;
        }
        let role = if v.get("lease").is_some() {
            let l = self
                .runtime
                .leases
                .get(&sid)
                .filter(|l| {
                    l.account == a.id
                        && l.until > now()
                        && Some(l.hash.as_str()) == v["lease"].as_str().map(digest).as_deref()
                })
                .ok_or_else(|| err(409, "Agent gestoppt oder Freigabe abgelaufen"))?;
            let role = Rolle::aus_name(str_field(v, "rolle")?)
                .filter(|r| l.roles.contains(r))
                .ok_or_else(|| err(403, "Rolle nicht für Agent freigegeben"))?;
            role
        } else {
            if let Some(l) = self.runtime.leases.get(&sid).filter(|l| l.until > now()) {
                for action in actions {
                    let action: Aktion = serde_json::from_value(action.clone())
                        .map_err(|_| err(400, "Aktion nicht lesbar"))?;
                    if action.zustaendig().iter().all(|r| l.roles.contains(r)) {
                        return Err(err(
                            409,
                            "Diese Rolle steuert gerade ein Agent; zuerst stoppen",
                        ));
                    }
                }
            }
            Rolle::Alle
        };
        self.transaction(|g| {
            g.check_team_turn(a,v)?;
            let result:Vec<Value>=actions.iter().map(|action| {let (ok,text)=g.world.handeln(sid,role,action);json!({"ok":ok,"text":text})}).collect();
            if role!=Rolle::Alle {
                let cleaned=v["notiz"].as_str().map(|n|eingabe::bereinigen(n,true));let notes=cleaned.as_deref();
                let wake=v["wecker_stunden"].as_f64().filter(|n|n.is_finite()).map(|n|(n.clamp(0.0,720.0)*3600.0) as i64);
                let hints=result.iter().filter(|v|v["ok"]==false).filter_map(|v|v["text"].as_str().map(str::to_string)).collect::<Vec<_>>();
                g.world.aufruf_ende(sid,role,notes,wake,&hints);
            }
            g.finish_team_turn(a,v,&result)?;
            if g.local {g.bots_react();}
            let response=json!({"ergebnisse":result,"world_id":g.runtime.world_id,"revision":g.runtime.revision+1});
            g.db.execute("INSERT INTO commands VALUES(?1,?2,?3,?4,?5,?6)",params![a.id,g.runtime.world_id,request,body_hash,response.to_string(),now()]).map_err(|_|err(503,"Befehl konnte nicht gespeichert werden"))?;
            g.db.execute("INSERT INTO audit(time,account,kind,result) VALUES(?1,?2,'spiel',?3)",params![now(),a.id,response.to_string()]).map_err(|_|err(503,"Protokoll nicht gespeichert"))?;
            Ok(response)
        })
    }
    pub fn agent_report(&mut self, a: &Account, v: &Value) -> Reply {
        let sid = Self::sid(a)?;
        harness::maschine_erlaubt(a)?;
        self.world_matches(v)?;
        let l = self
            .runtime
            .leases
            .get(&sid)
            .filter(|l| l.until > now() && l.hash == digest(v["lease"].as_str().unwrap_or("")))
            .ok_or_else(|| err(409, "Agentfreigabe fehlt"))?;
        let _ = l;
        self.transaction_with(false, |g| {
            let clean=json!({"at":now(),"status":v["status"].as_str().unwrap_or("unbekannt").chars().take(80).collect::<String>(),"role":v["role"].as_str().unwrap_or(""),
              "calls":v["calls"].as_u64().unwrap_or(0),"errors":v["errors"].as_u64().unwrap_or(0),"tokens":v["tokens"].as_u64().unwrap_or(0),"latency_ms":v["latency_ms"].as_u64().unwrap_or(0),"accepted":v["accepted"].as_u64().unwrap_or(0),"rejected":v["rejected"].as_u64().unwrap_or(0),
              "provider":v["provider"].as_str().unwrap_or("").chars().take(32).collect::<String>(),"model":v["model"].as_str().unwrap_or("").chars().take(100).collect::<String>(),
              "decision":v["decision"].as_str().unwrap_or("").chars().take(1600).collect::<String>(),"cost":v["cost"].as_f64().filter(|c|c.is_finite()&&*c>=0.0)});
            g.runtime.agent_status.insert(sid,clean);Ok(json!({"ok":true}))
        })
    }
    pub fn logout(&mut self, a: &Account, raw: &str) -> Reply {
        self.transaction(|g| {
            g.identity_event(Some(a.id),"logout",json!({}))?;
            g.db.execute("DELETE FROM sessions WHERE hash=?1", [digest(raw)])
                .map_err(|_| err(503, "Abmeldung fehlgeschlagen"))?;
            if let Some(sid) = a.sid {
                g.runtime.leases.remove(&sid);
            }
            Ok(json!({"ok":true}))
        })
    }
    pub fn admin_status(&self) -> Value {
        let mut accounts=self.account_registry();
        for a in &mut accounts {a["reich_status"]=a["spieler"].as_u64().map(|sid|self.world.reich_status(sid as u16)).unwrap_or(Value::Null);}
        // 20.10.2026: Spielerart je Konto (nur Spielleitung).
        for a in &mut accounts {a["art"]=json!(self.konto_art_nach_id(a["id"].as_i64().unwrap_or(0)));}
        let bots=self.runtime.bots.iter().map(|(sid,b)|{let s=&self.world.spieler[*sid as usize];json!({"spieler":sid,"name":s.name,"typ":b.typ,"naechster":b.naechster,"aktionen":s.statistik.aktionen,"abgelehnt":s.statistik.abgelehnt,"stufe":s.stufe,"punkte":s.punkte.gesamt(),"reich_status":self.world.reich_status(*sid)})}).collect::<Vec<_>>();
        let db_bytes = ["spiel.sqlite3", "spiel.sqlite3-wal", "spiel.sqlite3-shm"]
            .iter()
            .map(|p| {
                std::fs::metadata(self.data_root.join(p))
                    .map(|m| m.len())
                    .unwrap_or(0)
            })
            .sum::<u64>();
        let invariant_errors = self
            .world
            .planeten
            .iter()
            .filter(|p| p.bestand.iter().any(|n| *n < 0) || p.bevoelkerung < 0)
            .map(|p| format!("Negativer Bestand/Bevölkerung auf {}", p.koord))
            .collect::<Vec<_>>();
        json!({"lobby":self.lobby(),"monitor":self.monitoring_status(),"storage_error":self.storage_error,"world_hash":self.world.hash(),"uptime_s":self.started.elapsed().as_secs(),"db_bytes":db_bytes,"invariant_errors":invariant_errors,"bots_enabled":self.runtime.bots_enabled,"bot_period_secs":self.runtime.bot_period_secs,"opferschutz":self.runtime.opferschutz,"epoche_tage":self.world.regeln.welt.epoche_tage,"bots":bots,"accounts":accounts,"warteliste":self.private_waitlist(),
            "agents":self.runtime.agent_status.keys().chain(self.runtime.leases.keys()).copied().collect::<std::collections::BTreeSet<_>>().iter().map(|sid|json!({"spieler":sid,"active":self.runtime.leases.get(sid).is_some_and(|l|l.until>now()),"roles":self.runtime.leases.get(sid).map(|l|&l.roles),"stats":self.runtime.agent_status.get(sid).cloned().unwrap_or_else(||json!({"status":"Wartet auf erste Modellantwort","calls":0,"errors":0,"tokens":0,"latency_ms":0}))})).collect::<Vec<_>>(),
            "flotten":self.world.flotten.len(),"ereignisse":self.world.ereignisse.len(),"fairplay":self.fairplay_admin(),"harness":self.harness_admin(),"bot_actions":self.runtime.bot_actions,"bot_rejected":self.runtime.bot_rejected,"registration":{"enabled":mail::MailConfig::load(&self.data_root).is_ok_and(|c|c.is_some()),"config_file":"mail-private.json","code_digits":6},"database":{"root":self.data_root.to_string_lossy(),"file":"spiel.sqlite3","players_view":"registered_players","states_table":"player_states","events_table":"account_events","snapshot_interval_seconds":60}})
    }
    pub fn backup(&self) -> Result<PathBuf, (u16, String)> {
        let dir = self.data_root.join("backups");
        std::fs::create_dir_all(&dir).map_err(|_| err(503, "Backupverzeichnis fehlt"))?;
        let path = dir.join(format!("spiel-{}-{}.sqlite3", now(), &token()[..8]));
        self.db
            .backup(rusqlite::DatabaseName::Main, &path, None)
            .map_err(|_| err(503, "Backup fehlgeschlagen"))?;
        write_private(&path.with_extension("auth-key.txt"),self.email_code_key.as_bytes()).map_err(|_|err(503,"Privater Authenticator-Backupschlüssel nicht gespeichert"))?;
        Ok(path)
    }
    pub fn admin(&mut self, v: &Value) -> Reply {
        let fields = [
            "action",
            "spieler",
            "id",
            "banned",
            "tempo",
            "paused",
            "bots_enabled",
            "bot_period_secs",
            "opferschutz",
            "versorgung_stunden",
            "stillstand_stunden",
            "epoche_tage",
            "credits",
            "bestand",
            "seed",
            "typ",
            "maintenance",
            "admission_limit",
        ];
        let context = Value::Object(
            fields
                .iter()
                .filter_map(|k| v.get(*k).map(|val| ((*k).into(), val.clone())))
                .collect(),
        );
        self.audit_context = Some(context.clone());
        let result = self.admin_inner(v);
        if result.is_ok() && matches!(v["action"].as_str(), Some("backup" | "inspect" | "map" | "events" | "audit" | "rules_profile" | "rules_validate" | "reset_preview" | "player_details")) {
            if self
                .db
                .execute(
                    "INSERT INTO audit(time,account,kind,result) VALUES(?1,NULL,'admin',?2)",
                    params![
                        now(),
                        json!({"context":context,"world_id":self.runtime.world_id}).to_string()
                    ],
                )
                .is_err()
            {
                self.audit_context = None;
                return Err(err(503, "Admin-Audit konnte nicht gespeichert werden"));
            }
        }
        self.audit_context = None;
        result
    }
    fn admin_inner(&mut self, v: &Value) -> Reply {
        self.world_matches(v)?;
        match str_field(v,"action")? {
            "test_account"=>self.admin_test_account(v),
            "player_details"=>self.player_details(v),
            "map"|"events"|"audit"|"rules_profile"|"rules_validate"|"reset_preview"=>self.admin_read(v),
            "backup"=>Ok(json!({"backup":self.backup()?.file_name().unwrap().to_string_lossy()})),
            "settings"=>self.transaction(|g| {
                for field in ["versorgung_stunden","stillstand_stunden"] {
                    if let Some(value)=v.get(field) {
                        let n=value.as_i64().filter(|n|(1..=720).contains(n)).ok_or_else(||err(400,"Rettungsfristen zwischen 1 und 720 Spielstunden"))?;
                        if field=="versorgung_stunden" {g.world.ausscheiden.regeln.versorgung_stunden=n;}else{g.world.ausscheiden.regeln.stillstand_stunden=n;}
                    }
                }
                if let Some(n)=v["tempo"].as_u64(){if !(1..=3600).contains(&n){return Err(err(400,"Tempo zwischen 1 und 3600"));}g.runtime.tempo=n as u32;}
                if let Some(value)=v.get("admission_limit"){let n=value.as_u64().filter(|n|*n<=SEATS as u64).ok_or_else(||err(400,"Freigabe zwischen 0 und 20"))?;g.runtime.admission_limit=n as u16;}
                if let Some(p)=v["paused"].as_bool(){g.runtime.paused=p;}
                if let Some(p)=v["bots_enabled"].as_bool(){g.runtime.bots_enabled=p;}
                if let Some(m)=v["maintenance"].as_str(){g.runtime.maintenance=m.chars().take(400).collect();}
                if let Some(n)=v["bot_period_secs"].as_i64(){if !(900..=86400).contains(&n){return Err(err(400,"Botabstand zwischen 900 und 86400 Spielsekunden"));}g.runtime.bot_period_secs=n;}
                if let Some(o)=v.get("opferschutz"){g.runtime.opferschutz=opferschutz_lesen(o,g.runtime.opferschutz).map_err(|e|err(400,&e))?;}
                if let Some(n)=v["epoche_tage"].as_i64(){if n<=g.world.zeit/TAG || n>10000{return Err(err(400,"Epochenende muss nach der aktuellen Spielzeit liegen, maximal 10000 Tage"));}
                    let mut r=(*g.world.regeln).clone();r.welt.epoche_tage=n;g.world.regeln=std::sync::Arc::new(r);}
                g.admit_waiting()?;
                Ok(g.lobby())}),
            "inspect"=>{
                let sid=v["spieler"].as_u64().filter(|n|*n<(BOT_COUNT+SEATS) as u64).ok_or_else(||err(400,"Spielernummer fehlt"))? as u16;
                Ok(self.world.sicht(sid,Rolle::Alle))
            },
            "account"=>self.transaction(|g| {
                let id=v["id"].as_i64().ok_or_else(||err(400,"Kontonummer fehlt"))?;
                let banned=v["banned"].as_bool().ok_or_else(||err(400,"Sperrstatus fehlt"))?;
                if g.db.execute("UPDATE accounts SET banned=?1 WHERE id=?2",params![banned,id]).map_err(|_|err(503,"Konto nicht gespeichert"))?!=1{return Err(err(404,"Konto fehlt"));}
                if banned {g.runtime.waitlist.retain(|w|w.account!=id);g.db.execute("DELETE FROM sessions WHERE account=?1",[id]).map_err(|_|err(503,"Sitzungen nicht gesperrt"))?;g.db.execute("UPDATE harness_tokens SET revoked=1 WHERE account=?1",[id]).map_err(|_|err(503,"Harness-Schlüssel nicht widerrufen"))?;g.runtime.leases.retain(|_,l|l.account!=id);}
                Ok(json!({"ok":true}))}),
            "waitlist_remove"=>self.transaction(|g| {let id=v["id"].as_i64().ok_or_else(||err(400,"Kontonummer fehlt"))?;g.runtime.waitlist.retain(|w|w.account!=id);g.admit_waiting()?;Ok(json!({"ok":true}))}),
            "bot"=>self.transaction(|g| {
                let sid=v["spieler"].as_u64().filter(|n|*n<BOT_COUNT as u64).ok_or_else(||err(400,"Botnummer ungültig"))? as u16;
                let typ=Bottyp::aus_name(str_field(v,"typ")?).ok_or_else(||err(400,"Unbekannter Bottyp"))?;
                let bot=g.runtime.bots.get_mut(&sid).unwrap();bot.typ=typ;bot.naechster=g.world.zeit;Ok(json!({"ok":true}))}),
            "fairplay_aufheben"=>self.fairplay_aufheben(v),
            "stop_agent"=>self.transaction(|g| {let sid=v["spieler"].as_u64().filter(|n|*n<(BOT_COUNT+SEATS) as u64).ok_or_else(||err(400,"Spielernummer fehlt"))? as u16;g.runtime.leases.remove(&sid);Ok(json!({"ok":true}))}),
            "player_edit"=>{
                self.backup()?;
                self.transaction(|g| {
                    let sid=v["spieler"].as_u64().filter(|n|*n<(BOT_COUNT+SEATS) as u64).ok_or_else(||err(400,"Spielernummer fehlt"))? as u16;
                    if !g.world.spieler_aktiv(sid){return Err(err(400,"Freier Platz kann nicht bearbeitet werden"));}
                    let pid=g.world.spieler[sid as usize].heimat as usize;
                    if let Some(n)=v["credits"].as_i64(){if !(0..=1_000_000_000).contains(&n){return Err(err(400,"Credits außerhalb Grenze"));}g.world.spieler[sid as usize].credits=n*M;}
                    if let Some(m)=v["bestand"].as_object(){g.world.abrechnen(pid);for (k,n) in m {let gut=Gut::aus_name(k).ok_or_else(||err(400,"Unbekanntes Gut"))?;let n=n.as_i64().filter(|n|(0..=1_000_000_000).contains(n)).ok_or_else(||err(400,"Ungültiger Bestand"))?;g.world.planeten[pid].bestand[gut.idx()]=n*M;}}
                    g.world.raten_neu(pid);g.world.punkte_neu();Ok(json!({"ok":true}))
                })
            },
            "reset"=>{
                if v["confirm"].as_str()!=Some(&format!("RESET {}",self.runtime.world_id)){return Err(err(400,"Resetbestätigung passt nicht zur aktuellen Welt"));}
                let seed=v["seed"].as_u64().unwrap_or_else(||OsRng.next_u64());
                let rules=self.validated_profile(v)?;
                let (mut world,mut runtime)=fresh(seed,&rules).map_err(|e|err(400,&e))?;
                runtime.admission_limit=self.runtime.admission_limit;
                world.ausscheiden.regeln=self.world.ausscheiden.regeln.clone();
                let backup=self.backup()?;
                self.transaction(move |g| {g.world=world;g.runtime=runtime;
                    g.db.execute_batch("UPDATE accounts SET sid=NULL,mode='zuschauer'; DELETE FROM sessions; DELETE FROM commands;").map_err(|_|err(503,"Reset nicht gespeichert"))?;
                    Ok(json!({"world_id":g.runtime.world_id,"backup":backup.file_name().unwrap().to_string_lossy()}))})
            },
            _=>Err(err(400,"Unbekannter Verwaltungsbefehl"))
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn berichtszeiten_beruecksichtigen_tempowechsel_und_bleiben_stabil() {
        let mut g=game();g.world.zeit=7200;g.runtime.tempo=300;
        g.db.execute("INSERT INTO simulation_clock(world_id,sim_from,sim_to,wall_end,tempo) VALUES(?1,0,3600,100000,60)",[&g.runtime.world_id]).unwrap();
        g.db.execute("INSERT INTO simulation_clock(world_id,sim_from,sim_to,wall_end,tempo) VALUES(?1,3600,7200,100100,300)",[&g.runtime.world_id]).unwrap();
        let mut v=json!({"berichte":[{"zeit":1800,"ziel":"1:1:1"},{"zeit":5400,"ziel":"1:1:2"}]});
        Game::inbox_ids(&mut v);g.inbox_times(1,&mut v).unwrap();
        assert_eq!(v["berichte"][0]["echtzeit"],99970);assert_eq!(v["berichte"][1]["echtzeit"],100094);
        let vorher=v.clone();g.runtime.tempo=1;g.inbox_times(1,&mut v).unwrap();assert_eq!(v,vorher);
    }
    use super::*;
    fn game() -> Game {
        let p = std::env::temp_dir().join(format!("sternenepoche-test-{}", token()));
        Game::open(&p, 83).unwrap()
    }
    /// Karl 27.10.2026: App-Softlock („keine neuen Befehle mehr, erst nach Neustart“). Eine Panik in der Transaktion
    /// rollt Welt und SQL zurück, und eine offen gebliebene Transaktion blockiert den nächsten Befehl nicht.
    #[test]
    fn panik_oder_offene_transaktion_blockiert_keine_befehle() {
        let mut g = game();
        g.runtime.bots_enabled = false;
        let zeit = g.world.zeit;
        let e = g.transaction(|g| -> Reply { g.world.zeit += 999; panic!("Testpanik im Befehl") }).unwrap_err();
        assert_eq!(e.0, 503);
        assert!(g.db.is_autocommit() && g.world.zeit == zeit, "zurückgerollt");
        let rev = g.runtime.revision;
        g.transaction(|_| Ok(json!({}))).unwrap();
        assert_eq!(g.runtime.revision, rev + 1, "nächster Befehl geht durch");
        // Hängengebliebene Transaktion (wie früher nach einer Panik in der Nachphase): wird vor dem Befehl zurückgerollt.
        g.db.execute_batch("BEGIN IMMEDIATE").unwrap();
        g.transaction(|_| Ok(json!({}))).unwrap();
        assert!(g.db.is_autocommit());
    }
    fn user(g: &mut Game, name: &str) -> (Account, String) {
        let v = g
            .register(
                &json!({"name":name}),
                password_hash("lange-test-passphrase").unwrap(),
            )
            .unwrap();
        (
            g.authenticate(v["token"].as_str().unwrap()).unwrap(),
            v["token"].as_str().unwrap().into(),
        )
    }
    #[test]
    fn planer_private_core_and_harness_results_agree_without_world_changes() {
        let mut g=game();let (account,raw)=user(&mut g,"Planerprobe");
        g.claim(&account,&json!({"mode":"mensch","volk":"syntheten"})).unwrap();
        let a=g.authenticate(&raw).unwrap();let sid=Game::sid(&a).unwrap();
        let own=g.world.planeten[g.world.spieler[sid as usize].heimat as usize].koord;
        let foreign=g.world.planeten[g.world.spieler[0].heimat as usize].koord;
        let before=g.world.hash();
        for q in [json!({"typ":"regelkontext"}),json!({"typ":"wirtschaftsdiagnose","planet":own.to_string()}),
            json!({"typ":"kostenplan","planet":own.to_string(),"positionen":[{"art":"einheit","einheit":"handelsfrachter","anzahl":2}]})] {
            let result=g.tool(&a,&q).unwrap();assert_eq!(result["world_id"],g.runtime.world_id);
            let s=harness::Schluessel{id:0,label:"Test".into(),rollen:vec![],expires:0};
            assert_eq!(result,g.harness_werkzeug(&a,&s,&json!({"abfrage":q})).unwrap()["ergebnis"]);
            let mut expected=g.world.werkzeug(sid,&q).unwrap();expected["world_id"]=json!(g.runtime.world_id);
            assert_eq!(result,expected);
        }
        assert!(g.tool(&a,&json!({"typ":"wirtschaftsdiagnose","planet":foreign.to_string()})).is_err());
        assert_eq!(before,g.world.hash());
    }
    #[test]
    fn seats_freeze_atomic_claim_and_restart() {
        let mut g = game();
        g.runtime.bots_enabled = false;
        assert_eq!(g.lobby()["freie_plaetze"], 20);
        let old = g.world.planeten[g.world.spieler[30].heimat as usize].clone();
        let credits = g.world.spieler[30].credits;
        for _ in 0..24 {
            g.advance(3600).unwrap();
        }
        let p = &g.world.planeten[g.world.spieler[30].heimat as usize];
        assert_eq!(old.bestand, p.bestand);
        assert_eq!(old.bevoelkerung, p.bevoelkerung);
        assert_eq!(credits, g.world.spieler[30].credits);
        g.runtime.admission_limit=20;
        for i in 0..20 {
            let (a, _) = user(&mut g, &format!("Spieler{i}"));
            g.claim(&a, &json!({"mode":"mensch","volk":"veyari","anfaengerschutz":true}))
                .unwrap();
        }
        let (a, _) = user(&mut g, "Einundzwanzig");
        assert_eq!(g.claim(&a, &json!({"mode":"mensch","volk":"krath"})).unwrap()["warteliste"]["position"],1);
        assert_eq!(
            g.world.spieler[30].schutz_bis,
            86400 + g.world.regeln.diplomatie.anfaengerschutz_tage * TAG
        );
        let hash = g.world.hash();
        let root = g.data_root.clone();
        drop(g);
        let g = Game::open(&root, 1).unwrap();
        assert_eq!(hash, g.world.hash());
        assert_eq!(g.lobby()["freie_plaetze"], 0);
    }
    #[test]
    fn spectator_duplicate_private_lease_stop_reset() {
        let mut g = game();
        let (watch, raw) = user(&mut g, "Zuschauer");
        assert_eq!(g.view(&watch, Rolle::Alle).unwrap_err().0, 403);
        assert!(g.lobby().get("planeten").is_none());
        assert!(g.admin_status().to_string().find(&watch.password).is_none());
        g.claim(&watch, &json!({"mode":"gemischt","volk":"krath"}))
            .unwrap();
        let a = g.authenticate(&raw).unwrap();
        let start = g.world.planeten[g.world.spieler[a.sid.unwrap() as usize].heimat as usize]
            .koord
            .to_string();
        let cmd = json!({"world_id":g.runtime.world_id,"request_id":"befehl-1","aktionen":[{"typ":"bauen","planet":start,"gebaeude":"erzmine"}]});
        let result = g.command(&a, &cmd).unwrap();
        let count = g.world.spieler[30].statistik.aktionen;
        assert_eq!(result, g.command(&a, &cmd).unwrap());
        assert_eq!(count, g.world.spieler[30].statistik.aktionen);
        let mut different = cmd.clone();
        different["aktionen"][0]["gebaeude"] = json!("farm");
        assert_eq!(g.command(&a, &different).unwrap_err().0, 409);
        let lease = g
            .lease(
                &a,
                &json!({"world_id":g.runtime.world_id,"action":"start","roles":["verwalter"]}),
            )
            .unwrap();
        g.agent_report(&a,&json!({"world_id":g.runtime.world_id,"lease":lease["lease"],"status":"aktiv","calls":1})).unwrap();
        let mut remembered = g.runtime.bots[&0].clone();
        remembered.gespaeht.insert(Koord::neu(1, 2, 3), 42);
        g.runtime.bots.insert(0, remembered);
        let runtime_bytes = serde_json::to_vec(&g.runtime).unwrap();
        let restored: Runtime = serde_json::from_slice(&runtime_bytes).unwrap();
        assert_eq!(restored.agent_status[&30]["calls"], 1);
        assert_eq!(restored.bots[&0].gespaeht[&Koord::neu(1, 2, 3)], 42);
        let late = json!({"world_id":g.runtime.world_id,"request_id":"late","lease":lease["lease"],"rolle":"verwalter","aktionen":[{"typ":"bauen","planet":start,"gebaeude":"farm"}]});
        g.lease(&a, &json!({"world_id":g.runtime.world_id,"action":"stop"}))
            .unwrap();
        assert_eq!(g.command(&a, &late).unwrap_err().0, 409);
        let wid = g.runtime.world_id.clone();
        g.admin(&json!({"world_id":wid,"action":"reset","confirm":format!("RESET {wid}")}))
            .unwrap();
        assert_eq!(g.command(&a, &cmd).unwrap_err().0, 409);
        assert_eq!(g.authenticate(&raw).unwrap_err().0, 401);
        assert_eq!(g.lobby()["freie_plaetze"], 20);
    }
    /// Strukturelle Absicherung gegen SQL-Injektion: Keine SQL-Anweisung im Server wird
    /// aus Text zusammengesetzt. Einzige Ausnahme sind fest verdrahtete Spaltennamen
    /// beim Schema-Upgrade in identity.rs. Neue Ausnahmen lassen diesen Test scheitern.
    #[test]
    fn sql_nur_mit_gebundenen_parametern() {
        let quellen = [("lib.rs", include_str!("lib.rs")), ("main.rs", include_str!("main.rs")), ("identity.rs", include_str!("identity.rs")),
            ("admission.rs", include_str!("admission.rs")), ("authenticator.rs", include_str!("authenticator.rs")), ("communication.rs", include_str!("communication.rs")),
            ("monitoring.rs", include_str!("monitoring.rs")), ("workspace.rs", include_str!("workspace.rs")), ("fairplay.rs", include_str!("fairplay.rs")),
            ("harness.rs", include_str!("harness.rs")), ("mcp.rs", include_str!("mcp.rs")), ("agenten.rs", include_str!("agenten.rs")), ("uebergabe.rs", include_str!("uebergabe.rs")), ("mail.rs", include_str!("mail.rs")), ("spielerart.rs", include_str!("spielerart.rs"))];
        let mut funde = Vec::new();
        for (datei, text) in quellen {
            // Nur Produktivcode; Testmodule dürfen Muster als Text enthalten.
            let code = text.split("#[cfg(test)]").next().unwrap_or(text);
            for aufruf in ["execute(&format!", "execute_batch(&format!", "prepare(&format!", "query_row(&format!", "query_map(&format!", "execute(format!", "prepare(format!"] {
                for (i, _) in code.match_indices(aufruf) {
                    let zeile = code[..i].lines().count();
                    let inhalt = code.lines().nth(zeile.saturating_sub(1)).unwrap_or("");
                    // Erlaubt: Schema-Upgrade mit fest verdrahteter Spaltenliste.
                    if !inhalt.contains("ALTER TABLE accounts ADD COLUMN {name} {kind}") { funde.push(format!("{datei}:{zeile}")); }
                }
            }
        }
        assert!(funde.is_empty(), "SQL aus Text zusammengesetzt: {funde:?}");
    }
    #[test]
    fn rollback_storage_and_passwords() {
        let mut g = game();
        let hash = password_hash("richtiges Passwort").unwrap();
        assert!(password_matches("richtiges Passwort", &hash));
        assert!(!password_matches("falsch", &hash));
        let old = g.world.hash();
        let rev = g.runtime.revision;
        let wid = g.runtime.world_id.clone();
        assert!(g
            .admin(&json!({"world_id":wid,"action":"settings","tempo":123,"paused":true}))
            .is_ok());
        assert!(g.admin(&json!({"world_id":wid,"action":"player_edit","spieler":0,"credits":500,"bestand":{"erz":100,"xxx":5}})).is_err());
        assert_eq!(old, g.world.hash());
        assert_eq!(rev + 1, g.runtime.revision);
        let audit_count: i64 =
            g.db.query_row("SELECT count(*) FROM audit WHERE kind='admin'", [], |r| {
                r.get(0)
            })
            .unwrap();
        assert_eq!(audit_count, 1);
        g.admin(&json!({"world_id":wid,"action":"inspect","spieler":0}))
            .unwrap();
        let audit_count: i64 =
            g.db.query_row("SELECT count(*) FROM audit WHERE kind='admin'", [], |r| {
                r.get(0)
            })
            .unwrap();
        assert_eq!(audit_count, 2);
    }
    #[test]
    fn opferschutz_der_raeuberbots_ist_einstellbar_und_abschaltbar() {
        // Karl 20.10.2026: „muss zum Serverstart eingestellt werden können und auch ausgeschaltet werden können“.
        let mut g = game();
        // Karl 23.10.2026: neue Welten standardmäßig aus, beim Start oder im Betrieb einschaltbar.
        assert_eq!(g.runtime.opferschutz, lauf::bots::Opferschutz::neue_welt());
        assert!(!g.runtime.opferschutz.aktiv);
        let wid = g.runtime.world_id.clone();
        g.admin(&json!({"world_id":wid,"action":"settings","opferschutz":true})).unwrap();
        assert!(g.runtime.opferschutz.aktiv);
        g.admin(&json!({"world_id":wid,"action":"settings","opferschutz":false})).unwrap();
        assert!(!g.runtime.opferschutz.aktiv);
        g.admin(&json!({"world_id":wid,"action":"settings","opferschutz":{"aktiv":true,"pause_stunden":48,"neulinge_tage":7}})).unwrap();
        assert_eq!(g.runtime.opferschutz, lauf::bots::Opferschutz { aktiv: true, pause_stunden: 48, neulinge_tage: 7 });
        assert!(g.admin(&json!({"world_id":wid,"action":"settings","opferschutz":{"pause_stunden":0}})).is_err());
        assert_eq!(g.admin_status()["opferschutz"]["pause_stunden"], 48);
        // Die Bots übernehmen den Stand mit ihrem nächsten Zug.
        g.advance(3600).unwrap();
        assert!(g.runtime.bots.values().all(|b| b.opferschutz.pause_stunden == 48 && b.opferschutz.neulinge_tage == 7));
        // Ältere Laufzeiten ohne das Feld laden mit eingeschaltetem Schutz.
        let mut alt = serde_json::to_value(&g.runtime).unwrap();
        alt.as_object_mut().unwrap().remove("opferschutz");
        let r: Runtime = serde_json::from_value(alt).unwrap();
        assert_eq!(r.opferschutz, lauf::bots::Opferschutz::default());
        assert!(r.opferschutz.aktiv);
        assert_eq!(opferschutz_lesen(&json!({"aktiv":false}), r.opferschutz).unwrap().aktiv, false);
    }
    #[test]
    fn thirty_bots_one_game_day_and_backup_resume() {
        let mut g = game();
        for _ in 0..24 {
            g.advance(3600).unwrap();
        }
        assert_eq!(g.lobby()["freie_plaetze"], 20);
        assert!(g.world.spieler[..30]
            .iter()
            .all(|s| s.statistik.aktionen > 0));
        assert!(g.admin_status()["invariant_errors"]
            .as_array()
            .unwrap()
            .is_empty());
        let root = g.data_root.join("restore-test");
        std::fs::create_dir(&root).unwrap();
        std::fs::copy(g.backup().unwrap(), root.join("spiel.sqlite3")).unwrap();
        let mut restored = Game::open(&root, 0).unwrap();
        assert_eq!(g.world.hash(), restored.world.hash());
        g.advance(3600).unwrap();
        restored.advance(3600).unwrap();
        assert_eq!(g.world.hash(), restored.world.hash());
        assert_eq!(g.runtime.bot_actions, restored.runtime.bot_actions);
    }
    #[test]
    fn monitoring_privacy_profile_preview_and_legacy_runtime() {
        let mut g=game();g.advance(3600).unwrap();
        let status=g.admin_status();assert_eq!(status["monitor"]["bots"].as_array().unwrap().len(),30);
        assert!(status["monitor"]["bots"].as_array().unwrap().iter().any(|b|!b["verlauf"].as_array().unwrap().is_empty()));
        let history=g.lobby()["verlauf"].as_array().unwrap().clone();assert_eq!(history.len(),1);
        assert_eq!(history[0].as_object().unwrap().keys().cloned().collect::<std::collections::BTreeSet<_>>(),["kolonien","punkte","reiche","sekunden","stufen"].into_iter().map(str::to_owned).collect());
        let wid=g.runtime.world_id.clone();let hash=g.world.hash();
        let profile=g.admin(&json!({"action":"rules_profile","world_id":wid})).unwrap()["text"].as_str().unwrap().to_owned();
        let preview=g.admin(&json!({"action":"reset_preview","world_id":wid,"rules":profile})).unwrap();
        assert!(g.admin(&json!({"action":"reset_preview","world_id":wid,"rules":" ".repeat(60*1024+1)})).unwrap_err().1.contains("60 KiB"));
        assert_eq!(hash,g.world.hash());assert_eq!(wid,g.runtime.world_id);
        for invalid in [profile.replace("sektoren: 2","sektoren: 0"),profile.replace("treibstoff_teiler: 35000","treibstoff_teiler: 0"),profile.replace("reichtum_min: 0.7","reichtum_min: NaN")] {
            if invalid!=profile {assert!(g.admin(&json!({"action":"reset_preview","world_id":wid,"rules":invalid})).is_err());assert_eq!(hash,g.world.hash());}
        }
        assert!(g.admin(&json!({"action":"reset","world_id":wid,"confirm":format!("RESET {wid}"),"rules":"invalid"})).is_err());
        assert!(!g.data_root.join("backups").exists());assert_eq!(hash,g.world.hash());
        let mut old=serde_json::to_value(&g.runtime).unwrap();for k in ["bot_trace","public_history","maintenance","admission_limit","waitlist"]{old.as_object_mut().unwrap().remove(k);}
        let restored:Runtime=serde_json::from_value(old).unwrap();assert!(restored.bot_trace.is_empty());assert!(restored.public_history.is_empty());
        g.admin(&json!({"action":"reset","world_id":wid,"confirm":format!("RESET {wid}"),"rules":profile})).unwrap();
        assert_eq!(g.world.regeln.hash,preview["profile_hash"].as_str().unwrap());assert_eq!(g.lobby()["freie_plaetze"],20);
    }
    #[test]
    fn thirty_bots_thirty_days_health_and_bounded_history() {
        let mut g=game();
        for _ in 0..(30*24){g.advance(3600).unwrap();}
        assert_eq!(g.lobby()["freie_plaetze"],20);
        assert_eq!(g.runtime.public_history.len(),168);
        assert!(g.runtime.bot_trace.values().all(|t|t.len()<=20));
        assert!(g.admin_status()["invariant_errors"].as_array().unwrap().is_empty());
        assert!(g.world.spieler[..30].iter().all(|s|s.statistik.aktionen>30));
        let summary=json!({"spieltage":30,"aktionen":g.runtime.bot_actions,"abgelehnt":g.runtime.bot_rejected,
            "stufen":g.world.spieler[..30].iter().map(|s|s.stufe).collect::<Vec<_>>(),"kolonien":g.world.spieler[..30].iter().map(|s|s.planeten.len()).collect::<Vec<_>>()});
        println!("30-Tage-Botprüfung: {summary}");
        assert!(g.runtime.bot_trace.values().flatten().all(|v|v["text"].as_str().is_none_or(|text|!text.contains("benötigt einen eigenen Geheimdienst")&&!text.contains("Freier Startplatz"))));
    }
}
