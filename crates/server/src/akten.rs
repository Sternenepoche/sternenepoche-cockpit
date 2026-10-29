//! Gegnerakte: private Notizen je Konto, Welt und fremdem Reich (Karl 22.10.2026: „kleines Profil mit allen
//! gesammelten Informationen (mit Notizen)“). Die gesammelten Spielinformationen baut der Client aus der Sicht
//! (Berichte, Kampfberichte, Angriffe, Sternkarte); hier liegt nur der Text, den der Spieler selbst schreibt.
use super::*;

pub(super) fn setup(db: &Connection) -> Result<(), String> {
    db.execute_batch("CREATE TABLE IF NOT EXISTS gegner_notizen(account INTEGER NOT NULL REFERENCES accounts(id),world_id TEXT NOT NULL,
        spieler TEXT NOT NULL,text TEXT NOT NULL,geaendert INTEGER NOT NULL,PRIMARY KEY(account,world_id,spieler));").map_err(|e|e.to_string())
}

impl Game {
    /// Notizen dieses Kontos in der laufenden Welt, Reichsname → Text (Teil von `/api/view`).
    pub(super) fn gegner_notizen(&self, account: i64) -> Result<Value, (u16, String)> {
        let mut stmt = self.db.prepare("SELECT spieler,text FROM gegner_notizen WHERE account=?1 AND world_id=?2 ORDER BY spieler LIMIT 500")
            .map_err(|_| err(503, "Notizen nicht lesbar"))?;
        let rows = stmt.query_map(params![account, self.runtime.world_id], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))
            .map_err(|_| err(503, "Notizen nicht lesbar"))?
            .collect::<Result<BTreeMap<_, _>, _>>().map_err(|_| err(503, "Notizen nicht lesbar"))?;
        Ok(json!(rows))
    }
    /// Speichert oder löscht (leerer Text) die Notiz zu einem Reich. Unabhängig von Pause und Regierungsrollen.
    pub fn gegner_notiz(&mut self, a: &Account, v: &Value) -> Reply {
        self.world_matches(v)?;
        Self::sid(a)?;
        let spieler = v["spieler"].as_str().unwrap_or("").trim().to_string();
        let text = v["text"].as_str().unwrap_or("").trim().to_string();
        if spieler.is_empty() || spieler.chars().count() > 60 || spieler.contains('\0') { return Err(err(400, "Reich für die Notiz fehlt")); }
        if text.chars().count() > 4000 || text.contains('\0') { return Err(err(400, "Notiz höchstens 4.000 Zeichen")); }
        if text.is_empty() {
            self.db.execute("DELETE FROM gegner_notizen WHERE account=?1 AND world_id=?2 AND spieler=?3", params![a.id, self.runtime.world_id, spieler])
                .map_err(|_| err(503, "Notiz nicht gelöscht"))?;
        } else {
            let anzahl: i64 = self.db.query_row("SELECT COUNT(*) FROM gegner_notizen WHERE account=?1 AND world_id=?2 AND spieler<>?3",
                params![a.id, self.runtime.world_id, spieler], |r| r.get(0)).map_err(|_| err(503, "Notizen nicht lesbar"))?;
            if anzahl >= 500 { return Err(err(400, "Höchstens 500 Akten mit Notizen")); }
            self.db.execute("INSERT INTO gegner_notizen VALUES(?1,?2,?3,?4,?5) ON CONFLICT(account,world_id,spieler) DO UPDATE SET text=excluded.text,geaendert=excluded.geaendert",
                params![a.id, self.runtime.world_id, spieler, text, now()]).map_err(|_| err(503, "Notiz nicht gespeichert"))?;
        }
        Ok(json!({"spieler": spieler, "text": text, "gegner_notizen": self.gegner_notizen(a.id)?}))
    }
}
