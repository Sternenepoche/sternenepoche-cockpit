//! Einmalige Sitzungsübergabe vom Cockpit an den normalen Browser („Go to Browser“).
//! Der Code entsteht nur aus einer bereits angemeldeten Sitzung (Passwort und ggf.
//! Authenticator wurden dort geprüft), gilt 60 Sekunden, genau einmal, und steht im
//! URL-Fragment: er erreicht weder Serverprotokolle noch Referrer.
use super::*;

pub(crate) fn setup(db: &Connection) -> Result<(), String> {
    db.execute_batch("CREATE TABLE IF NOT EXISTS sitzung_uebergabe(hash TEXT PRIMARY KEY,account INTEGER NOT NULL REFERENCES accounts(id),expires INTEGER NOT NULL,used INTEGER NOT NULL DEFAULT 0);")
        .map_err(|e| e.to_string())
}

impl Game {
    pub fn uebergabe_erstellen(&mut self, a: &Account) -> Reply {
        self.transaction(|g| {
            g.db.execute("DELETE FROM sitzung_uebergabe WHERE expires<?1", [now() - 3600]).map_err(|_| err(503, "Übergabe nicht möglich"))?;
            let offen: i64 = g.db.query_row("SELECT count(*) FROM sitzung_uebergabe WHERE account=?1 AND used=0 AND expires>?2", params![a.id, now()], |r| r.get(0)).map_err(|_| err(503, "Übergabe nicht möglich"))?;
            if offen >= 5 { return Err(err(429, "Zu viele offene Übergaben; eine Minute warten")); }
            let code = token();
            g.db.execute("INSERT INTO sitzung_uebergabe(hash,account,expires) VALUES(?1,?2,?3)", params![digest(&code), a.id, now() + 60]).map_err(|_| err(503, "Übergabe nicht gespeichert"))?;
            g.identity_event(Some(a.id), "uebergabe_erstellt", json!({}))?;
            Ok(json!({"code":code,"gueltig_sekunden":60}))
        })
    }

    pub fn uebergabe_einloesen(&mut self, v: &Value) -> Reply {
        let code = str_field(v, "code")?;
        if code.len() != 64 || !code.bytes().all(|b| b.is_ascii_hexdigit()) { return Err(err(400, "Übergabecode ungültig")); }
        self.transaction(|g| {
            let konto: Option<i64> = g.db.query_row("SELECT account FROM sitzung_uebergabe WHERE hash=?1 AND used=0 AND expires>?2", params![digest(code), now()], |r| r.get(0)).optional().map_err(|_| err(503, "Übergabe nicht lesbar"))?;
            let id = konto.ok_or_else(|| err(401, "Übergabecode abgelaufen oder schon benutzt; im Cockpit erneut „Go to Browser“ wählen"))?;
            g.db.execute("UPDATE sitzung_uebergabe SET used=1 WHERE hash=?1", [digest(code)]).map_err(|_| err(503, "Übergabe nicht gespeichert"))?;
            let a = g.db.query_row("SELECT id,name,password,sid,mode,banned FROM accounts WHERE id=?1", [id], account_from).map_err(|_| err(401, "Konto fehlt"))?;
            if a.banned { return Err(err(403, "Konto gesperrt")); }
            g.identity_event(Some(a.id), "uebergabe_eingeloest", json!({}))?;
            g.session(&a)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn uebergabe_einmalig_und_befristet() {
        let mut g = Game::open(&std::env::temp_dir().join(format!("uebergabe-{}", token())), 83).unwrap();
        let v = g.register(&json!({"name":"Cockpitler"}), "test-hash".into()).unwrap();
        let a = g.authenticate(v["token"].as_str().unwrap()).unwrap();
        let code = g.uebergabe_erstellen(&a).unwrap()["code"].as_str().unwrap().to_string();
        let neu = g.uebergabe_einloesen(&json!({"code":code})).unwrap();
        assert_eq!(g.authenticate(neu["token"].as_str().unwrap()).unwrap().id, a.id);
        assert_eq!(g.uebergabe_einloesen(&json!({"code":code})).unwrap_err().0, 401);
        let alt = g.uebergabe_erstellen(&a).unwrap()["code"].as_str().unwrap().to_string();
        g.db.execute("UPDATE sitzung_uebergabe SET expires=0", []).unwrap();
        assert_eq!(g.uebergabe_einloesen(&json!({"code":alt})).unwrap_err().0, 401);
        assert_eq!(g.uebergabe_einloesen(&json!({"code":"nicht-hex"})).unwrap_err().0, 400);
    }
}
