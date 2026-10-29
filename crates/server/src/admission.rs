//! Admission is part of the same durable checkpoint/transaction as the world.
use super::*;

#[derive(Clone, Serialize, Deserialize)]
pub(crate) struct Waiting {
    pub account: i64,
    pub mode: String,
    pub volk: Volk,
    pub joined: i64,
    #[serde(default)] pub anfaengerschutz: bool,
}
pub(crate) fn legacy_admission_limit() -> u16 { SEATS }

impl Game {
    pub fn me(&self, a: &Account) -> Value {
        let waiting = self.runtime.waitlist.iter().enumerate().find(|(_, w)| w.account == a.id)
            .map(|(i, w)| json!({"position":i+1,"mode":w.mode,"volk":w.volk,"seit":w.joined,"anfaengerschutz":w.anfaengerschutz}));
        json!({"name":a.name,"mode":a.mode,"spieler":a.sid,"world_id":self.runtime.world_id,"warteliste":waiting,"reich_status":a.sid.map(|sid|self.world.reich_status(sid)),"authenticator_enabled":self.authenticator_enabled(a.id),"fairplay":self.fairplay_konto(a.id)})
    }
    pub(crate) fn available_admissions(&self) -> usize {
        if self.world.beendet() { return 0; }
        let inactive = self.world.aufklaerung.inaktive_spieler.len();
        (self.runtime.admission_limit as usize).saturating_sub((self.runtime.seat_capacity as usize).saturating_sub(inactive)).min(inactive)
    }
    pub(crate) fn private_waitlist(&self) -> Vec<Value> {
        self.runtime.waitlist.iter().enumerate().map(|(i, w)| {
            let name = self.db.query_row("SELECT name FROM accounts WHERE id=?1", [w.account], |r| r.get::<_, String>(0)).ok();
            json!({"id":w.account,"name":name,"position":i+1,"mode":w.mode,"volk":w.volk,"seit":w.joined,"anfaengerschutz":w.anfaengerschutz})
        }).collect()
    }
    // Called only inside a transaction. Queue order cannot be bypassed by a new claim.
    pub(crate) fn admit_waiting(&mut self) -> Result<(), (u16, String)> {
        while !self.world.beendet() && self.available_admissions() > 0 && !self.runtime.waitlist.is_empty() {
            let w = self.runtime.waitlist.remove(0);
            let a = self.db.query_row("SELECT id,name,password,sid,mode,banned FROM accounts WHERE id=?1", [w.account], account_from)
                .optional().map_err(|_| err(503,"Warteliste nicht lesbar"))?;
            let Some(a) = a.filter(|a| !a.banned && a.sid.is_none()) else { continue; };
            let sid = *self.world.aufklaerung.inaktive_spieler.iter().next().ok_or_else(|| err(409,"Kein Startplatz frei"))?;
            self.world.startplatz_aktivieren(sid,a.name,w.volk).map_err(|e| err(409,&e))?;
            self.world.spieler[sid as usize].schutz_bis=if w.anfaengerschutz{self.world.zeit+self.world.regeln.diplomatie.anfaengerschutz_tage*TAG}else{self.world.zeit};
            self.db.execute("UPDATE accounts SET sid=?1,mode=?2 WHERE id=?3",params![sid,w.mode,a.id])
                .map_err(|_| err(503,"Platz nicht gespeichert"))?;
        }
        Ok(())
    }
    pub fn leave_waitlist(&mut self, a: &Account, v: &Value) -> Reply {
        self.world_matches(v)?;
        self.transaction(|g| {
            g.runtime.waitlist.retain(|w| w.account != a.id);
            g.admit_waiting()?;
            Ok(g.me(a))
        })
    }
    pub fn claim(&mut self, a: &Account, v: &Value) -> Reply {
        if v.get("world_id").is_some() { self.world_matches(v)?; }
        let mode = str_field(v,"mode")?;
        if !["mensch","agent","gemischt"].contains(&mode) { return Err(err(400,"Modus mensch, agent oder gemischt wählen")); }
        let volk = Volk::aus_name(str_field(v,"volk")?).ok_or_else(|| err(400,"Unbekanntes Volk"))?;
        self.transaction(|g| {
            let current = g.account_by_name(&a.name).ok_or_else(|| err(401,"Konto fehlt"))?;
            if current.banned { return Err(err(403,"Konto gesperrt")); }
            if let Some(sid) = current.sid {
                g.db.execute("UPDATE accounts SET mode=?1 WHERE id=?2",params![mode,current.id]).map_err(|_| err(503,"Spielweise nicht gespeichert"))?;
                if mode == "mensch" {
                    // Mensch heißt: keine Maschine handelt. Freigabe weg, Harness-Schlüssel widerrufen (neue erst nach Wechsel zu Agent/Gemischt).
                    g.runtime.leases.remove(&sid);
                    let n = g.db.execute("UPDATE harness_tokens SET revoked=1 WHERE account=?1 AND revoked=0",params![current.id]).map_err(|_| err(503,"Widerruf nicht gespeichert"))?;
                    if n > 0 { g.identity_event(Some(current.id),"harness_widerruf",json!({"grund":"spielweise_mensch","anzahl":n}))?; }
                }
            } else {
                if g.world.beendet() { return Err(err(409,"Epoche beendet")); }
                if let Some(w) = g.runtime.waitlist.iter_mut().find(|w| w.account == current.id) {
                    w.mode = mode.into(); w.volk = volk; w.anfaengerschutz=v["anfaengerschutz"].as_bool().unwrap_or(false);
                } else {
                    if g.runtime.waitlist.len() >= 1000 { return Err(err(409,"Warteliste voll; später erneut versuchen")); }
                    g.runtime.waitlist.push(Waiting { account:current.id,mode:mode.into(),volk,joined:now(),anfaengerschutz:v["anfaengerschutz"].as_bool().unwrap_or(false) });
                }
                g.admit_waiting()?;
            }
            let current = g.account_by_name(&a.name).ok_or_else(|| err(503,"Konto nicht lesbar"))?;
            Ok(g.me(&current))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn user(g: &mut Game,name: &str) -> Account {
        let raw=g.register(&json!({"name":name}),"test-hash".into()).unwrap();
        g.authenticate(raw["token"].as_str().unwrap()).unwrap()
    }
    #[test]
    fn protection_requires_explicit_start_choice_and_cannot_be_reenabled(){
        let root=std::env::temp_dir().join(format!("protection-{}",token()));let mut g=Game::open(&root,83).unwrap();
        let a=user(&mut g,"Unprotected");g.claim(&a,&json!({"mode":"mensch","volk":"aurelianer"})).unwrap();let sid=g.account_by_name(&a.name).unwrap().sid.unwrap();assert_eq!(g.world.spieler[sid as usize].schutz_bis,g.world.zeit);
        g.claim(&a,&json!({"mode":"mensch","volk":"aurelianer","anfaengerschutz":true})).unwrap();assert_eq!(g.world.spieler[sid as usize].schutz_bis,g.world.zeit);
        let b=user(&mut g,"Protected");g.claim(&b,&json!({"mode":"mensch","volk":"aurelianer","anfaengerschutz":true})).unwrap();let sid=g.account_by_name(&b.name).unwrap().sid.unwrap();assert!(g.world.spieler[sid as usize].schutz_bis>g.world.zeit);
        drop(g);let g=Game::open(&root,83).unwrap();assert!(g.world.spieler[sid as usize].schutz_bis>g.world.zeit);
    }
    #[test]
    fn three_seats_fifo_duplicate_cancel_restart_and_reset() {
        let root=std::env::temp_dir().join(format!("admission-{}",token()));
        let mut g=Game::open(&root,83).unwrap();
        g.runtime.admission_limit=3;
        let users:Vec<_>=(0..7).map(|i|user(&mut g,&format!("Wartender{i}"))).collect();
        let claim=json!({"mode":"mensch","volk":"veyari"});
        for a in &users {g.claim(a,&claim).unwrap();}
        assert_eq!(g.lobby()["freigegebene_plaetze"],3);
        assert_eq!(g.lobby()["freie_zugaenge"],0);
        assert_eq!(g.lobby()["freie_plaetze"],17);
        assert_eq!(g.me(&users[3])["warteliste"]["position"],1);
        assert_eq!(g.runtime.waitlist.len(),4);
        g.claim(&users[4],&json!({"mode":"agent","volk":"krath"})).unwrap();
        assert_eq!(g.me(&users[4])["warteliste"]["position"],2);
        let hash=g.world.hash();let wid=g.runtime.world_id.clone();
        g.leave_waitlist(&users[5],&json!({"world_id":wid})).unwrap();
        assert_eq!(g.runtime.waitlist.len(),3);
        assert_eq!(hash,g.world.hash());
        assert!(g.lobby().get("warteliste").is_none());
        assert!(g.view(&users[3],Rolle::Alle).is_err());
        assert_eq!(g.world.rangliste().len(),33);
        drop(g);let mut g=Game::open(&root,1).unwrap();
        assert_eq!(g.runtime.admission_limit,3);
        assert_eq!(g.me(&users[4])["warteliste"]["position"],2);
        g.admin(&json!({"world_id":wid,"action":"settings","admission_limit":4})).unwrap();
        assert_eq!(g.account_by_name(&users[3].name).unwrap().sid,Some(33));
        assert!(g.account_by_name(&users[4].name).unwrap().sid.is_none());
        g.claim(&users[5],&claim).unwrap(); // rejoin goes to the end
        assert_eq!(g.me(&users[5])["warteliste"]["position"],3);
        g.admin(&json!({"world_id":wid,"action":"account","id":users[4].id,"banned":true})).unwrap();
        assert_eq!(g.me(&users[6])["warteliste"]["position"],1);
        g.admin(&json!({"world_id":wid,"action":"settings","admission_limit":5})).unwrap();
        assert_eq!(g.account_by_name(&users[6].name).unwrap().sid,Some(34));
        assert_eq!(g.me(&users[5])["warteliste"]["position"],1);
        assert!(g.admin(&json!({"world_id":wid,"action":"settings","admission_limit":21})).is_err());
        assert_eq!(g.runtime.admission_limit,5);
        g.admin(&json!({"world_id":wid,"action":"reset","confirm":format!("RESET {wid}")})).unwrap();
        assert_eq!(g.runtime.admission_limit,5);
        assert!(g.runtime.waitlist.is_empty());
        assert_eq!(g.lobby()["freie_zugaenge"],5);
        assert!(g.leave_waitlist(&users[5],&json!({"world_id":wid})).is_err());
    }
    #[test]
    fn closed_admission_admin_removal_and_lower_limit_keep_players() {
        let root=std::env::temp_dir().join(format!("admission-{}",token()));let mut g=Game::open(&root,83).unwrap();
        let wid=g.runtime.world_id.clone();let a=user(&mut g,"Geschlossen");
        g.admin(&json!({"world_id":wid,"action":"settings","admission_limit":0})).unwrap();
        g.claim(&a,&json!({"mode":"gemischt","volk":"syntheten"})).unwrap();
        assert_eq!(g.world.rangliste().len(),30);
        g.admin(&json!({"world_id":wid,"action":"waitlist_remove","id":a.id})).unwrap();
        assert!(g.runtime.waitlist.is_empty());
        g.admin(&json!({"world_id":wid,"action":"settings","admission_limit":1})).unwrap();
        g.claim(&a,&json!({"mode":"gemischt","volk":"syntheten"})).unwrap();
        g.admin(&json!({"world_id":wid,"action":"settings","admission_limit":0})).unwrap();
        assert_eq!(g.account_by_name(&a.name).unwrap().sid,Some(30));
        assert_eq!(g.lobby()["freie_zugaenge"],0);
    }
}
