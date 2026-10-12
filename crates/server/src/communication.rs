//! Research records are private host data. No HTTP endpoint exposes this archive.
use super::*;
pub(crate) fn setup(db: &Connection) -> Result<(), String> {
    db.execute_batch("CREATE TABLE IF NOT EXISTS communication_events(
        world_id TEXT NOT NULL,event_id INTEGER NOT NULL,simulation_seconds INTEGER NOT NULL,
        actor INTEGER NOT NULL,kind TEXT NOT NULL,payload_json TEXT NOT NULL,
        PRIMARY KEY(world_id,event_id));
      CREATE INDEX IF NOT EXISTS communication_actor ON communication_events(world_id,actor,kind,event_id);
      CREATE TABLE IF NOT EXISTS game_actions(
        id INTEGER PRIMARY KEY,world_id TEXT NOT NULL,simulation_seconds INTEGER NOT NULL,
        actor INTEGER NOT NULL,role TEXT NOT NULL,accepted INTEGER NOT NULL,action_json TEXT NOT NULL,result TEXT NOT NULL);
      CREATE INDEX IF NOT EXISTS game_actions_actor ON game_actions(world_id,actor,id);
      CREATE TABLE IF NOT EXISTS game_domain_events(
        id INTEGER PRIMARY KEY,world_id TEXT NOT NULL,simulation_seconds INTEGER NOT NULL,
        actor INTEGER NOT NULL,kind TEXT NOT NULL,payload_json TEXT NOT NULL);
      CREATE TABLE IF NOT EXISTS communication_objects(
        world_id TEXT NOT NULL,kind TEXT NOT NULL,object_id INTEGER NOT NULL,state_json TEXT NOT NULL,
        PRIMARY KEY(world_id,kind,object_id));
      CREATE TABLE IF NOT EXISTS communication_messages(
        world_id TEXT NOT NULL,message_id INTEGER NOT NULL,sender INTEGER NOT NULL,
        simulation_seconds INTEGER NOT NULL,channel TEXT NOT NULL,subject TEXT NOT NULL,body TEXT NOT NULL,
        recipients_json TEXT NOT NULL,reply_to INTEGER,automatic INTEGER NOT NULL,
        PRIMARY KEY(world_id,message_id));
      CREATE INDEX IF NOT EXISTS communication_thread ON communication_messages(world_id,reply_to,message_id);
      CREATE TABLE IF NOT EXISTS research_epochs(
        world_id TEXT PRIMARY KEY,seed TEXT NOT NULL,rules_hash TEXT NOT NULL,simulation_seconds INTEGER NOT NULL,
        finished INTEGER NOT NULL,summary_json TEXT NOT NULL);") .map_err(|e|e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn command_idempotency_private_views_atomic_archive_and_restart() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../laeufe/kommunikation-tests")
            .join(token());
        let mut g = Game::open(&root, 712).unwrap();
        g.runtime.bots_enabled = false;
        let auth = g
            .register(
                &json!({"name":"MailTester"}),
                password_hash("test-password-private").unwrap(),
            )
            .unwrap();
        let a = g.authenticate(auth["token"].as_str().unwrap()).unwrap();
        g.claim(&a, &json!({"mode":"mensch","volk":"aurelianer"}))
            .unwrap();
        let a = g.account_by_name("MailTester").unwrap();
        let target = g.world.spieler[0].name.clone();
        let cmd = json!({"world_id":g.runtime.world_id,"request_id":"mail-idempotent-1","aktionen":[{"typ":"brief_senden","kanal":"privat","an":target,"betreff":"Persistence","text":"Secret body","antwort_auf":null}]});
        let first = g.command(&a, &cmd).unwrap();
        assert_eq!(first["ergebnisse"][0]["ok"], true);
        g.command(&a, &cmd).unwrap();
        assert_eq!(
            g.db.query_row("SELECT count(*) FROM communication_messages", [], |r| r
                .get::<_, i64>(0))
                .unwrap(),
            1
        );
        assert_eq!(
            g.db.query_row("SELECT count(*) FROM game_actions", [], |r| r
                .get::<_, i64>(0))
                .unwrap(),
            1
        );
        assert!(!g
            .world
            .sicht(1, Rolle::Alle)
            .to_string()
            .contains("Secret body"));
        let hash = g.world.hash();
        let wid = g.runtime.world_id.clone();
        drop(g);
        let mut g = Game::open(&root, 712).unwrap();
        assert_eq!(hash, g.world.hash());
        assert_eq!(wid, g.runtime.world_id);
        // A failed archive write rolls back both world and all table changes.
        g.db.execute_batch("CREATE TRIGGER fail_communication BEFORE INSERT ON communication_events BEGIN SELECT RAISE(FAIL,'test'); END;").unwrap();
        let mut next = cmd.clone();
        next["request_id"] = json!("mail-rollback-2");
        assert!(g.command(&a, &next).is_err());
        assert_eq!(g.world.hash(), hash);
        assert_eq!(
            g.db.query_row("SELECT count(*) FROM communication_messages", [], |r| r
                .get::<_, i64>(0))
                .unwrap(),
            1
        );
    }
}
impl Game {
    pub(crate) fn save_communication(&mut self) -> Result<(), (u16, String)> {
        let error =
            |_: rusqlite::Error| err(503, "Kommunikationsarchiv konnte nicht gespeichert werden");
        let wid = &self.runtime.world_id;
        // Engine domain events are a drainable delivery buffer, not another full
        // in-memory history. Communications already have their own atomic archive.
        let mut domain=self.db.prepare_cached("INSERT INTO game_domain_events(world_id,simulation_seconds,actor,kind,payload_json) VALUES(?1,?2,?3,?4,?5)").map_err(error)?;
        for e in self.world.fachereignisse.iter().filter(|e|e.get("id").is_none()) {
            domain.execute(params![wid,e["time"].as_i64(),e["player_id"].as_u64(),e["event_type"].as_str(),e["payload"].to_string()]).map_err(error)?;
        }
        let last: u64 = self
            .db
            .query_row(
                "SELECT coalesce(max(event_id),0) FROM communication_events WHERE world_id=?1",
                [wid],
                |r| r.get(0),
            )
            .map_err(error)?;
        // Ids are continuous across retention (`ereignisse_ausgelagert`); never a vector index.
        let events = self
            .world
            .kommunikation
            .ereignisse_nach(last)
            .map_err(|e| err(503, &e))?;
        for e in events.iter() {
            let p = &e["payload"];
            self.db
                .execute(
                    "INSERT INTO communication_events VALUES(?1,?2,?3,?4,?5,?6)",
                    params![
                        wid,
                        e["id"].as_u64(),
                        e["time"].as_i64(),
                        e["player_id"].as_u64(),
                        e["event_type"].as_str(),
                        p.to_string()
                    ],
                )
                .map_err(error)?;
            if e["event_type"] == "communication.sent" {
                let id = p["message_id"].as_u64().unwrap();
                let b = &self.world.kommunikation.briefe[&id];
                let n = &self.world.nachrichten[id as usize - 1];
                self.db
                    .execute(
                        "INSERT INTO communication_messages VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)",
                        params![
                            wid,
                            id,
                            n.von,
                            n.zeit,
                            b.kanal,
                            b.betreff,
                            n.text,
                            serde_json::to_string(&n.an).unwrap(),
                            b.antwort_auf,
                            b.automatisch
                        ],
                    )
                    .map_err(error)?;
            }
        }
        if !events.is_empty() {
            let put = |kind: &str, id: u64, value: Value| -> Result<(), (u16, String)> {
                self.db.execute("INSERT INTO communication_objects VALUES(?1,?2,?3,?4) ON CONFLICT(world_id,kind,object_id) DO UPDATE SET state_json=excluded.state_json",params![wid,kind,id,value.to_string()]).map_err(error)?;
                Ok(())
            };
            // Only objects affected by this transaction are rewritten.
            let mut affected = std::collections::BTreeSet::new();
            for e in events.iter() {
                for (key, kind) in [
                    ("message_id", "letter"),
                    ("request_id", "request"),
                    ("offer_id", "offer"),
                    ("help_id", "help"),
                    ("alliance", "alliance"),
                ] {
                    if let Some(id) = e["payload"][key].as_u64() {
                        affected.insert((kind, id));
                    }
                }
            }
            for (kind, id) in affected {
                let c = &self.world.kommunikation;
                let v = match kind {
                    "letter" => c.briefe.get(&id).map(|o| json!(o)),
                    "request" => c.anfragen.iter().find(|o| o.id == id).map(|o| json!(o)),
                    "offer" => c.angebote.iter().find(|o| o.id == id).map(|o| json!(o)),
                    "help" => c.hilfe.iter().find(|o| o.id == id).map(|o| json!(o)),
                    "alliance" => self
                        .world
                        .allianzen
                        .iter()
                        .find(|o| o.id as u64 == id)
                        .map(|o| json!(o)),
                    _ => None,
                };
                if let Some(v) = v {
                    put(kind, id, v)?;
                }
            }
        }
        // The research summary hashes the whole world (a full copy and serialization). Writing
        // it once per game hour and at the epoch end keeps clicks and ticks independent of
        // the world size; the checkpoint itself is unaffected.
        let key = (self.world.zeit / STUNDE, self.world.beendet());
        if self.research_key != Some(key) {
            self.db.execute("INSERT INTO research_epochs VALUES(?1,?2,?3,?4,?5,?6) ON CONFLICT(world_id) DO UPDATE SET simulation_seconds=excluded.simulation_seconds,finished=excluded.finished,summary_json=excluded.summary_json",params![wid,self.world.startwert.to_string(),self.world.regeln.hash,self.world.zeit,self.world.beendet(),json!({"ranking":self.world.rangliste(),"communications":self.world.kommunikation.ereignisse_gesamt(),"world_hash":self.world.hash(),"players":self.world.spieler.iter().map(|s|json!({"id":s.id,"name":s.name,"stats":s.statistik})).collect::<Vec<_>>(),"inference":"script_bots_or_client_models; see agent reports"}).to_string()]).map_err(error)?;
            self.research_key = Some(key);
        }
        Ok(())
    }
}
