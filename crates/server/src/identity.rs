//! Durable private identities, one-time email grants and human-readable snapshots.
use super::*;
use rand::Rng;

pub(crate) fn setup(db:&Connection,root:&Path)->Result<String,String> {
    let columns:Vec<String>=db.prepare("PRAGMA table_info(accounts)").and_then(|mut s|s.query_map([],|r|r.get(1))?.collect()).map_err(|e|e.to_string())?;
    for (name,kind) in [("email","TEXT"),("email_verified_at","INTEGER"),("created_at","INTEGER NOT NULL DEFAULT 0"),("last_login_at","INTEGER"),("login_count","INTEGER NOT NULL DEFAULT 0"),("is_test","INTEGER NOT NULL DEFAULT 0")] {
        if !columns.iter().any(|c|c==name) { db.execute_batch(&format!("ALTER TABLE accounts ADD COLUMN {name} {kind}")).map_err(|e|e.to_string())?; }
    }
    db.execute_batch("CREATE UNIQUE INDEX IF NOT EXISTS account_email ON accounts(email) WHERE email IS NOT NULL;
      CREATE TABLE IF NOT EXISTS email_challenges(id TEXT PRIMARY KEY,email TEXT NOT NULL,code_hash TEXT NOT NULL,created INTEGER NOT NULL,expires INTEGER NOT NULL,attempts INTEGER NOT NULL DEFAULT 0,delivered INTEGER NOT NULL DEFAULT 0,verified_at INTEGER,grant_hash TEXT,grant_expires INTEGER,consumed INTEGER NOT NULL DEFAULT 0);
      CREATE INDEX IF NOT EXISTS email_requests ON email_challenges(email,created);
      CREATE TABLE IF NOT EXISTS account_events(id INTEGER PRIMARY KEY,time INTEGER NOT NULL,account INTEGER REFERENCES accounts(id),kind TEXT NOT NULL,details TEXT NOT NULL);
      CREATE INDEX IF NOT EXISTS account_event_lookup ON account_events(account,id);
      CREATE TABLE IF NOT EXISTS player_states(account INTEGER PRIMARY KEY REFERENCES accounts(id),world_id TEXT NOT NULL,simulation_seconds INTEGER NOT NULL,updated_at INTEGER NOT NULL,summary_json TEXT NOT NULL,state_json TEXT NOT NULL);
      CREATE VIEW IF NOT EXISTS registered_players AS SELECT a.id,a.name,a.email,a.email_verified_at,a.is_test,a.created_at,a.last_login_at,a.login_count,a.sid,a.mode,a.banned,s.world_id,s.simulation_seconds,s.updated_at,s.summary_json FROM accounts a LEFT JOIN player_states s ON s.account=a.id;").map_err(|e|e.to_string())?;
    db.execute("DELETE FROM email_challenges WHERE created<?1",[now()-86400]).map_err(|e|e.to_string())?;
    let path=root.join("email-code-key.txt");
    if !path.exists() {
        if columns.iter().any(|c|c=="totp_secret") && db.query_row("SELECT count(*) FROM accounts WHERE totp_secret IS NOT NULL",[],|r|r.get::<_,i64>(0)).map_err(|e|e.to_string())?>0 {
            return Err("Authenticator-Schlüssel fehlt. Datenbank und zugehörigen auth-key.txt-Backupschlüssel gemeinsam wiederherstellen.".into());
        }
        write_private(&path,token().as_bytes()).map_err(|e|e.to_string())?;
    }
    let key=std::fs::read_to_string(path).map_err(|e|e.to_string())?;
    if key.len()!=64 { return Err("Privater E-Mail-Schlüssel ungültig; Original behalten".into()); }
    Ok(key)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn email_codes_grants_are_single_use_and_failed_attempts_persist(){
        let root=std::env::temp_dir().join(format!("email-test-{}",token()));let mut g=Game::open(&root,83).unwrap();
        assert!(mail::MailConfig::load(&root).unwrap().is_none());
        let c=g.begin_email("Tester@Example.org").unwrap();assert_eq!(c.code.len(),6);
        assert!(g.begin_email("tester@example.org").is_err());
        assert!(g.verify_email(&json!({"challenge_id":c.id,"code":c.code})).is_err(),"unsent code must fail");
        g.email_delivery(&c.id,true).unwrap();
        let wrong=if c.code=="000000"{"111111"}else{"000000"};
        for _ in 0..5 {assert!(g.verify_email(&json!({"challenge_id":c.id,"code":wrong})).is_err());}
        assert!(g.verify_email(&json!({"challenge_id":c.id,"code":c.code})).is_err());
        let c=g.begin_email("Fresh@Example.org").unwrap();g.email_delivery(&c.id,true).unwrap();
        let grant=g.verify_email(&json!({"challenge_id":c.id,"code":c.code})).unwrap();
        assert!(g.verify_email(&json!({"challenge_id":c.id,"code":c.code})).is_err());
        let request=json!({"registration_token":grant["registration_token"],"name":"EmailTester","mode":"mensch","volk":"veyari","world_id":g.runtime.world_id});
        let hash=password_hash("email-test-passwort").unwrap();let session=g.register_email(&request,hash.clone()).unwrap();
        assert_eq!(session["spieler"],30);assert!(g.register_email(&request,hash).is_err());
        let a=g.account_by_name("EmailTester").unwrap();let details=g.player_details(&json!({"id":a.id})).unwrap();
        assert_eq!(details["account"]["email"],"fresh@example.org");assert!(details["account"]["email_verified_at"].is_i64());
        assert_eq!(details["account"]["login_count"],1);assert!(details["events"].as_array().unwrap().len()>=2);
        assert!(!g.admin_status().to_string().contains(&g.email_code_key));
        drop(g);let g=Game::open(&root,2).unwrap();assert_eq!(g.account_by_name("EmailTester").unwrap().sid,Some(30));
        assert_eq!(g.db.query_row("SELECT count(*) FROM player_states",[],|r|r.get::<_,i64>(0)).unwrap(),1);
    }
    #[test]
    fn expiry_and_delivery_failure_never_grant_access(){
        let root=std::env::temp_dir().join(format!("email-expiry-{}",token()));let mut g=Game::open(&root,83).unwrap();
        let c=g.begin_email("expiry@example.org").unwrap();g.email_delivery(&c.id,true).unwrap();
        g.db.execute("UPDATE email_challenges SET expires=0 WHERE id=?1",[&c.id]).unwrap();
        assert!(g.verify_email(&json!({"challenge_id":c.id,"code":c.code})).is_err());
        let c=g.begin_email("failed@example.org").unwrap();g.email_delivery(&c.id,false).unwrap();
        assert!(g.verify_email(&json!({"challenge_id":c.id,"code":c.code})).is_err());
        assert!(g.register_email(&json!({"name":"BypassTester","registration_token":token(),"mode":"mensch","volk":"krath"}),"test-hash".into()).is_err());
    }
}

pub struct EmailChallenge {pub id:String,pub email:String,pub code:String}
impl Game {
    pub(crate) fn record_player_events(&self,logs:Vec<kern::welt::Logeintrag>)->Result<(),(u16,String)> {
        for event in logs {
            self.db.execute("INSERT INTO game_actions(world_id,simulation_seconds,actor,role,accepted,action_json,result) VALUES(?1,?2,?3,?4,?5,?6,?7)",params![self.runtime.world_id,event.zeit,event.spieler,event.rolle.to_string(),event.ok,event.aktion,event.text]).map_err(|_|err(503,"Spielaktionsarchiv konnte nicht gespeichert werden"))?;
            let account=self.db.query_row("SELECT id FROM accounts WHERE sid=?1",[event.spieler],|r|r.get::<_,i64>(0)).optional().map_err(|_|err(503,"Spielerkonto nicht lesbar"))?;
            if let Some(id)=account{self.identity_event(Some(id),"game_event",serde_json::to_value(event).map_err(|_|err(503,"Spielprotokoll ungültig"))?)?;}
        }
        Ok(())
    }
    pub(crate) fn identity_event(&self,account:Option<i64>,kind:&str,details:Value)->Result<(),(u16,String)> {
        self.db.execute("INSERT INTO account_events(time,account,kind,details) VALUES(?1,?2,?3,?4)",params![now(),account,kind,details.to_string()]).map_err(|_|err(503,"Kontoprotokoll nicht gespeichert"))?;
        Ok(())
    }
    pub fn failed_login(&self,name:&str)->Result<(),(u16,String)> {
        let id=self.account_by_name(name).map(|a|a.id);
        self.identity_event(id,"login_failed",json!({"name":name.chars().take(24).collect::<String>()}))
    }
    pub fn begin_email(&mut self,email:&str)->Result<EmailChallenge,(u16,String)> {
        let email=email.trim().to_ascii_lowercase();
        if email.len()>254 || !email.is_ascii() || email.parse::<lettre::Address>().is_err() {
            return Err(err(400,"Gültige E-Mail-Adresse eingeben"));
        }
        let (count,last):(i64,i64)=self.db.query_row("SELECT count(*),coalesce(max(created),0) FROM email_challenges WHERE email=?1 AND created>?2",params![email,now()-3600],|r|Ok((r.get(0)?,r.get(1)?))).map_err(|_|err(503,"Anmeldung nicht verfügbar"))?;
        if count>=5 || last>now()-60 {return Err(err(429,"Bitte warten: frühestens nach 60 Sekunden erneut senden, höchstens fünf Codes pro Stunde"));}
        let total:i64=self.db.query_row("SELECT count(*) FROM email_challenges WHERE created>?1",[now()-3600],|r|r.get(0)).map_err(|_|err(503,"Anmeldung nicht verfügbar"))?;
        if total>=500 {return Err(err(429,"Zu viele Registrierungen; später erneut versuchen"));}
        let id=token();let code=format!("{:06}",OsRng.gen_range(0..1_000_000u32));
        let hash=digest(&format!("{}:{id}:{code}",self.email_code_key));
        self.db.execute("DELETE FROM email_challenges WHERE created<?1",[now()-86400]).map_err(|_|err(503,"Anmeldung nicht verfügbar"))?;
        self.db.execute("INSERT INTO email_challenges(id,email,code_hash,created,expires) VALUES(?1,?2,?3,?4,?5)",params![id,email,hash,now(),now()+600]).map_err(|_|err(503,"Code nicht gespeichert"))?;
        Ok(EmailChallenge{id,email,code})
    }
    pub fn email_delivery(&mut self,id:&str,ok:bool)->Reply {
        self.transaction(|g| {
            g.db.execute("UPDATE email_challenges SET delivered=?1,expires=CASE WHEN ?1 THEN expires ELSE 0 END WHERE id=?2",params![ok,id]).map_err(|_|err(503,"Versandstatus nicht gespeichert"))?;
            if ok {g.db.execute("UPDATE email_challenges SET expires=0 WHERE email=(SELECT email FROM email_challenges WHERE id=?1) AND id<>?1 AND consumed=0",[id]).map_err(|_|err(503,"Alter Code nicht widerrufen"))?;}
            g.identity_event(None,if ok{"email_sent"}else{"email_failed"},json!({}))?;
            Ok(json!({"challenge_id":id,"expires_in":600,"resend_after":60}))
        })
    }
    pub fn verify_email(&mut self,v:&Value)->Reply {
        let id=str_field(v,"challenge_id")?;let code=str_field(v,"code")?;
        if id.len()!=64 || code.len()!=6 || !code.bytes().all(|b|b.is_ascii_digit()) {return Err(err(400,"Sechsstelligen Code eingeben"));}
        let result=self.transaction(|g| {
            let row:Option<(String,i64,i64,bool,bool)>=g.db.query_row("SELECT code_hash,expires,attempts,delivered,verified_at IS NOT NULL FROM email_challenges WHERE id=?1",[id],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?))).optional().map_err(|_|err(503,"Code nicht lesbar"))?;
            let Some((hash,expires,attempts,delivered,verified))=row else{return Ok(json!({"invalid":true}));};
            if expires<=now() || attempts>=5 || !delivered || verified {return Ok(json!({"invalid":true}));}
            // A wrong attempt must commit, rather than roll back with an HTTP error.
            g.db.execute("UPDATE email_challenges SET attempts=attempts+1 WHERE id=?1",[id]).map_err(|_|err(503,"Versuch nicht gespeichert"))?;
            let expected=digest(&format!("{}:{id}:{code}",g.email_code_key));
            if hash.len()!=expected.len() || !hash.bytes().zip(expected.bytes()).fold(true,|same,(a,b)|same & (a==b)) {return Ok(json!({"invalid":true}));}
            let grant=token();
            g.db.execute("UPDATE email_challenges SET verified_at=?1,code_hash='',grant_hash=?2,grant_expires=?3 WHERE id=?4",params![now(),digest(&grant),now()+600,id]).map_err(|_|err(503,"Bestätigung nicht gespeichert"))?;
            Ok(json!({"registration_token":grant,"expires_in":600}))
        })?;
        if result["invalid"]==true {Err(err(400,"Code ungültig, abgelaufen oder zu oft versucht. Fordere gegebenenfalls einen neuen Code an."))}else{Ok(result)}
    }
    pub fn register_email(&mut self,v:&Value,hashed:String)->Reply {
        let grant=str_field(v,"registration_token")?;
        if grant.len()!=64 {return Err(err(403,"Zuerst E-Mail-Adresse bestätigen"));}
        let name=self.validated_account_name(v)?.to_owned();
        let row:Option<(String,String)>=self.db.query_row("SELECT id,email FROM email_challenges WHERE grant_hash=?1 AND grant_expires>?2 AND verified_at IS NOT NULL AND consumed=0",params![digest(grant),now()],|r|Ok((r.get(0)?,r.get(1)?))).optional().map_err(|_|err(503,"Bestätigung nicht lesbar"))?;
        let (id,email)=row.ok_or_else(||err(403,"E-Mail-Bestätigung fehlt oder ist abgelaufen"))?;
        let mode=str_field(v,"mode")?.to_owned();
        if !matches!(mode.as_str(),"mensch"|"agent"|"gemischt") {return Err(err(400,"Spielweise wählen"));}
        let volk=Volk::aus_name(str_field(v,"volk")?).ok_or_else(||err(400,"Volk wählen"))?;
        if v.get("world_id").is_some(){self.world_matches(v)?;}
        if self.world.beendet(){return Err(err(409,"Diese Epoche ist beendet"));}
        self.transaction(|g| {
            g.db.execute("INSERT INTO accounts(name,password,mode,email,email_verified_at,created_at) VALUES(?1,?2,'zuschauer',?3,?4,?4)",params![name,hashed,email,now()]).map_err(|_|err(409,"Name oder E-Mail bereits einem Konto zugeordnet"))?;
            let a=g.account_by_name(&name).ok_or_else(||err(503,"Konto nicht lesbar"))?;
            g.db.execute("UPDATE email_challenges SET consumed=1,grant_hash=NULL WHERE id=?1",[id]).map_err(|_|err(503,"Bestätigung nicht verbraucht"))?;
            if g.runtime.waitlist.len()>=1000 {return Err(err(409,"Warteliste voll"));}
            g.runtime.waitlist.push(admission::Waiting{account:a.id,mode,volk,joined:now(),anfaengerschutz:v["anfaengerschutz"].as_bool().unwrap_or(false)});g.admit_waiting()?;
            g.identity_event(Some(a.id),"registered",json!({"method":"email_otp"}))?;
            let a=g.account_by_name(&name).ok_or_else(||err(503,"Konto nicht lesbar"))?;g.session(&a)
        })
    }
    pub(crate) fn validated_account_name<'a>(&self,v:&'a Value)->Result<&'a str,(u16,String)> {
        let name=str_field(v,"name")?.trim();
        if !(3..=24).contains(&name.chars().count()) || !eingabe::kontoname_zeichen(name) {return Err(err(400,"Name: 3 bis 24 lateinische Buchstaben (auch Umlaute), Ziffern, - oder _"));}
        if eingabe::reserviert(name) {return Err(err(400,"Dieser Name ist der Spielleitung vorbehalten"));}
        if self.world.spieler.iter().any(|s|s.name.eq_ignore_ascii_case(name)) || self.account_by_name(name).is_some() || self.fairplay_name_belegt(name) {return Err(err(409,"Name bereits vergeben"));}
        Ok(name)
    }
    pub(crate) fn admin_test_account(&mut self,v:&Value)->Reply {
        let name=str_field(v,"name")?.to_owned();
        let existing=self.account_by_name(&name);
        let hashed=if v.get("password").is_some(){Some(password_hash(str_field(v,"password")?).map_err(|e|err(400,&e))?)}else{None};
        if existing.is_none(){self.validated_account_name(v)?;if hashed.is_none(){return Err(err(400,"Passwort für neues Testkonto erforderlich"));}}
        let count:i64=self.db.query_row("SELECT count(*) FROM accounts WHERE is_test=1",[],|r|r.get(0)).map_err(|_|err(503,"Testkonten nicht lesbar"))?;
        if count>=5 && !existing.as_ref().is_some_and(|a|self.db.query_row("SELECT is_test FROM accounts WHERE id=?1",[a.id],|r|r.get::<_,bool>(0)).unwrap_or(false)){return Err(err(409,"Fünf Testkonten sind bereits angelegt"));}
        self.transaction(|g| {
            if let Some(a)=existing {
                if let Some(hash)=hashed {
                    g.db.execute("UPDATE accounts SET password=?1 WHERE id=?2",params![hash,a.id]).map_err(|_|err(503,"Passwort nicht gespeichert"))?;
                    g.db.execute("DELETE FROM sessions WHERE account=?1",[a.id]).map_err(|_|err(503,"Sitzungen nicht widerrufen"))?;
                    g.db.execute("UPDATE harness_tokens SET revoked=1 WHERE account=?1",[a.id]).map_err(|_|err(503,"Harness-Schlüssel nicht widerrufen"))?;
                    if let Some(sid)=a.sid{g.runtime.leases.remove(&sid);}
                    g.identity_event(Some(a.id),"test_password_reset",json!({}))?;
                }
                g.db.execute("UPDATE accounts SET is_test=1 WHERE id=?1",[a.id]).map_err(|_|err(503,"Testkonto nicht gespeichert"))?;
            }else{
                g.db.execute("INSERT INTO accounts(name,password,mode,created_at,is_test) VALUES(?1,?2,'zuschauer',?3,1)",params![name,hashed,now()]).map_err(|_|err(409,"Name bereits vergeben"))?;
                let a=g.account_by_name(&name).ok_or_else(||err(503,"Konto nicht lesbar"))?;
                g.identity_event(Some(a.id),"test_account_created",json!({}))?;
            }
            let a=g.account_by_name(&name).ok_or_else(||err(503,"Konto nicht lesbar"))?;
            Ok(json!({"id":a.id,"name":a.name,"spieler":a.sid,"is_test":true}))
        })
    }
    pub(crate) fn account_registry(&self)->Vec<Value> {
        self.db.prepare("SELECT id,name,email,email_verified_at,is_test,created_at,last_login_at,login_count,sid,mode,banned,totp_secret IS NOT NULL FROM accounts ORDER BY id").and_then(|mut s|s.query_map([],|r|Ok(json!({"id":r.get::<_,i64>(0)?,"name":r.get::<_,String>(1)?,"email":r.get::<_,Option<String>>(2)?,"email_verified_at":r.get::<_,Option<i64>>(3)?,"is_test":r.get::<_,bool>(4)?,"created_at":r.get::<_,i64>(5)?,"last_login_at":r.get::<_,Option<i64>>(6)?,"login_count":r.get::<_,i64>(7)?,"spieler":r.get::<_,Option<u16>>(8)?,"mode":r.get::<_,String>(9)?,"banned":r.get::<_,bool>(10)?,"authenticator_enabled":r.get::<_,bool>(11)?})))?.collect()).unwrap_or_default()
    }
    pub(crate) fn player_details(&self,v:&Value)->Reply {
        let id=v["id"].as_i64().ok_or_else(||err(400,"Kontonummer fehlt"))?;
        let account=self.account_registry().into_iter().find(|a|a["id"]==id).ok_or_else(||err(404,"Konto fehlt"))?;
        let view=account["spieler"].as_u64().map(|sid|self.world.sicht(sid as u16,Rolle::Alle));
        let events=self.db.prepare("SELECT time,kind,details FROM account_events WHERE account=?1 ORDER BY id DESC LIMIT 100").and_then(|mut s|s.query_map([id],|r|Ok(json!({"time":r.get::<_,i64>(0)?,"kind":r.get::<_,String>(1)?,"details":serde_json::from_str::<Value>(&r.get::<_,String>(2)?).unwrap_or(Value::Null)})))?.collect::<Result<Vec<_>,_>>()).map_err(|_|err(503,"Kontoprotokoll nicht lesbar"))?;
        let commands=self.db.prepare("SELECT created,result FROM commands WHERE account=?1 ORDER BY created DESC LIMIT 50").and_then(|mut s|s.query_map([id],|r|Ok(json!({"time":r.get::<_,i64>(0)?,"result":serde_json::from_str::<Value>(&r.get::<_,String>(1)?).unwrap_or(Value::Null)})))?.collect::<Result<Vec<_>,_>>()).map_err(|_|err(503,"Befehle nicht lesbar"))?;
        // 20.10.2026: Spielerart des Platzes mit Zählern, dazu der Verlauf der Artwechsel und Agentenfreigaben (die
        // 100 jüngsten Kontoereignisse sind meist Spielaktionen und würden ihn verdrängen).
        let spielerart=account["spieler"].as_u64().map(|sid|self.spielerart_eintrag(sid as u16));
        let spielerart_verlauf=self.spielerart_verlauf(id)?;
        Ok(json!({"account":account,"state":view,"events":events,"commands":commands,"spielerart":spielerart,"spielerart_verlauf":spielerart_verlauf,"world_id":self.runtime.world_id,"simulation_seconds":self.world.zeit}))
    }
    pub(crate) fn snapshot_players(&self)->Result<(),(u16,String)> {
        for a in self.account_registry() {
            let id=a["id"].as_i64().unwrap();
            let due:bool=self.db.query_row("SELECT NOT EXISTS(SELECT 1 FROM player_states WHERE account=?1 AND world_id=?2 AND updated_at>?3 AND json_extract(summary_json,'$.spieler') IS ?4)",params![id,self.runtime.world_id,now()-60,a["spieler"].as_u64()],|r|r.get(0)).map_err(|_|err(503,"Spielerstand nicht lesbar"))?;
            if !due {continue;}
            let sid=a["spieler"].as_u64();
            let view=sid.map(|sid|self.world.sicht(sid as u16,Rolle::Alle)).unwrap_or(Value::Null);
            let summary=json!({"spieler":sid,"volk":view["volk"],"stufe":view["stufe"],"credits":view["credits"],"punkte":view["punkte"],"kolonien":view["planeten"].as_array().map(Vec::len),"reich_status":view["reich_status"]});
            self.db.execute("INSERT INTO player_states VALUES(?1,?2,?3,?4,?5,?6) ON CONFLICT(account) DO UPDATE SET world_id=excluded.world_id,simulation_seconds=excluded.simulation_seconds,updated_at=excluded.updated_at,summary_json=excluded.summary_json,state_json=excluded.state_json",params![id,self.runtime.world_id,self.world.zeit,now(),summary.to_string(),view.to_string()]).map_err(|_|err(503,"Spielerstand nicht gespeichert"))?;
        }
        Ok(())
    }
}
