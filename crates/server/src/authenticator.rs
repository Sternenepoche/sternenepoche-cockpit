//! Local RFC 6238 TOTP, encrypted seeds and single-use recovery codes.
use super::*;
use chacha20poly1305::{aead::{Aead,Payload},ChaCha20Poly1305,KeyInit,Nonce};
use totp_rs::{Algorithm,TOTP};

pub(crate) fn setup(db:&Connection)->Result<(),String> {
    let cols:Vec<String>=db.prepare("PRAGMA table_info(accounts)").and_then(|mut s|s.query_map([],|r|r.get(1))?.collect()).map_err(|e|e.to_string())?;
    for (name,kind) in [("totp_secret","TEXT"),("totp_pending","TEXT"),("totp_pending_until","INTEGER"),("totp_pending_attempts","INTEGER NOT NULL DEFAULT 0"),("totp_last_step","INTEGER NOT NULL DEFAULT -1")] {
        if !cols.iter().any(|c|c==name){db.execute_batch(&format!("ALTER TABLE accounts ADD COLUMN {name} {kind}")).map_err(|e|e.to_string())?;}
    }
    db.execute_batch("CREATE TABLE IF NOT EXISTS recovery_codes(account INTEGER NOT NULL REFERENCES accounts(id),hash TEXT NOT NULL,used_at INTEGER,PRIMARY KEY(account,hash));").map_err(|e|e.to_string())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn user(g:&mut Game,name:&str)->(Account,String){
        let session=g.register(&json!({"name":name}),password_hash("test-authenticator-passwort").unwrap()).unwrap();
        (g.authenticate(session["token"].as_str().unwrap()).unwrap(),session["token"].as_str().unwrap().into())
    }
    #[test]
    fn rfc6238_and_encrypted_setup_recovery_replay_restart_and_backup(){
        let standard=TOTP::new(Algorithm::SHA1,8,0,30,b"12345678901234567890".to_vec(),None,"RFC6238".into()).unwrap();
        assert_eq!(standard.generate(59),"94287082");assert_eq!(standard.generate(1111111109),"07081804");
        let root=std::env::temp_dir().join(format!("authenticator-test-{}",token()));
        let mut g=Game::open(&root,83).unwrap();let (a,session)=user(&mut g,"AuthTester");
        let setup=g.authenticator_start(&a).unwrap();
        assert!(setup["qr_image"].as_str().unwrap().starts_with("data:image/png;base64,iVBOR"));
        let generator=TOTP::from_url(setup["otpauth_uri"].as_str().unwrap()).unwrap();
        let code=generator.generate(now() as u64);
        let enabled=g.authenticator_enable(&a,&json!({"code":code}),&session).unwrap();
        let codes=enabled["recovery_codes"].as_array().unwrap();assert_eq!(codes.len(),8);
        assert!(!g.check_second_factor(a.id,&code).unwrap(),"a code used for setup cannot be replayed for login");
        assert!(!g.check_second_factor(a.id,"").unwrap());
        assert!(g.check_second_factor(a.id,codes[0].as_str().unwrap()).unwrap());
        assert!(!g.check_second_factor(a.id,codes[0].as_str().unwrap()).unwrap());
        let stored:String=g.db.query_row("SELECT totp_secret FROM accounts WHERE id=?1",[a.id],|r|r.get(0)).unwrap();
        assert!(!stored.contains(setup["manual_key"].as_str().unwrap()));
        assert!(g.decrypt_totp(a.id+1,&stored).is_err(),"ciphertext is bound to its account");
        let admin=g.admin_status().to_string();assert!(!admin.contains(setup["manual_key"].as_str().unwrap()));assert!(!admin.contains(&stored));
        let backup=g.backup().unwrap();assert_eq!(std::fs::read_to_string(backup.with_extension("auth-key.txt")).unwrap(),g.email_code_key);
        drop(g);let mut g=Game::open(&root,1).unwrap();assert!(g.authenticator_enabled(a.id));
        assert!(!g.check_second_factor(a.id,codes[0].as_str().unwrap()).unwrap());
        g.authenticator_disable(&a,&json!({"code":codes[1]}),&session).unwrap();assert!(!g.authenticator_enabled(a.id));
        assert!(g.check_second_factor(a.id,"").unwrap());
    }
    #[test]
    fn pending_setup_expires_and_locks_after_five_attempts(){
        let root=std::env::temp_dir().join(format!("authenticator-attempts-{}",token()));let mut g=Game::open(&root,83).unwrap();let (a,session)=user(&mut g,"SetupTester");
        let setup=g.authenticator_start(&a).unwrap();let generator=TOTP::from_url(setup["otpauth_uri"].as_str().unwrap()).unwrap();
        for _ in 0..5 {assert!(g.authenticator_enable(&a,&json!({"code":"invalid"}),&session).is_err());}
        assert!(g.authenticator_enable(&a,&json!({"code":generator.generate(now() as u64)}),&session).is_err());
        g.authenticator_start(&a).unwrap();g.db.execute("UPDATE accounts SET totp_pending_until=0 WHERE id=?1",[a.id]).unwrap();
        assert!(g.authenticator_enable(&a,&json!({"code":"000000"}),&session).is_err());
        assert!(!g.authenticator_enabled(a.id));
    }
}
fn unhex(s:&str)->Result<Vec<u8>,(u16,String)> {
    if s.len()%2!=0 || !s.is_ascii(){return Err(err(503,"Authentifikatorschlüssel ungültig"));}
    (0..s.len()).step_by(2).map(|i|u8::from_str_radix(&s[i..i+2],16).map_err(|_|err(503,"Authentifikatorschlüssel ungültig"))).collect()
}
fn totp(secret:Vec<u8>,name:&str)->Result<TOTP,(u16,String)> {
    // SHA-1, six digits and 30 seconds are interoperable Authenticator defaults.
    TOTP::new(Algorithm::SHA1,6,1,30,secret,Some("Sternenepoche".into()),name.into()).map_err(|_|err(503,"Authenticator konnte nicht vorbereitet werden"))
}
fn matching_step(totp:&TOTP,code:&str,at:i64)->Option<i64> {
    if code.len()!=6 || !code.bytes().all(|b|b.is_ascii_digit()){return None;}
    [0,-1,1].into_iter().map(|offset|at/30+offset).filter(|step|*step>=0).find(|step|totp.check(code,(*step*30) as u64) && totp.generate((*step*30) as u64)==code)
}
impl Game {
    fn encrypt_totp(&self,id:i64,secret:&[u8])->Result<String,(u16,String)> {
        let key=Sha256::digest(format!("totp:{}",self.email_code_key).as_bytes());
        let cipher=ChaCha20Poly1305::new_from_slice(&key).map_err(|_|err(503,"Authentifikatorschlüssel ungültig"))?;
        let mut nonce=[0u8;12];OsRng.fill_bytes(&mut nonce);
        let aad=id.to_string();
        let encrypted=cipher.encrypt(Nonce::from_slice(&nonce),Payload{msg:secret,aad:aad.as_bytes()}).map_err(|_|err(503,"Authenticator nicht gespeichert"))?;
        Ok(format!("{}{}",hex(&nonce),hex(&encrypted)))
    }
    fn decrypt_totp(&self,id:i64,data:&str)->Result<Vec<u8>,(u16,String)> {
        if data.len()<56{return Err(err(503,"Authentifikatorschlüssel ungültig"));}
        let nonce=unhex(&data[..24])?;let encrypted=unhex(&data[24..])?;
        let key=Sha256::digest(format!("totp:{}",self.email_code_key).as_bytes());
        let cipher=ChaCha20Poly1305::new_from_slice(&key).map_err(|_|err(503,"Authentifikatorschlüssel ungültig"))?;
        let aad=id.to_string();
        cipher.decrypt(Nonce::from_slice(&nonce),Payload{msg:&encrypted,aad:aad.as_bytes()}).map_err(|_|err(503,"Privater Authenticator-Schlüssel passt nicht zur Datenbank"))
    }
    pub fn authenticator_enabled(&self,id:i64)->bool {
        self.db.query_row("SELECT totp_secret IS NOT NULL FROM accounts WHERE id=?1",[id],|r|r.get(0)).unwrap_or(false)
    }
    pub fn login_permitted(&self,id:i64)->bool {
        self.db.query_row("SELECT count(*)<10 FROM account_events WHERE account=?1 AND kind='login_failed' AND time>?2",params![id,now()-60],|r|r.get(0)).unwrap_or(false)
    }
    // Checked only after the password succeeds; a replay or a consumed backup fails.
    pub fn check_second_factor(&mut self,id:i64,code:&str)->Result<bool,(u16,String)> {
        let (secret,last):(Option<String>,i64)=self.db.query_row("SELECT totp_secret,totp_last_step FROM accounts WHERE id=?1",[id],|r|Ok((r.get(0)?,r.get(1)?))).map_err(|_|err(503,"Authenticator nicht lesbar"))?;
        let Some(secret)=secret else{return Ok(true);};
        let account=self.db.query_row("SELECT name FROM accounts WHERE id=?1",[id],|r|r.get::<_,String>(0)).map_err(|_|err(503,"Konto fehlt"))?;
        let generator=totp(self.decrypt_totp(id,&secret)?,&account)?;
        if let Some(step)=matching_step(&generator,code,now()).filter(|step|*step>last) {
            let changed=self.db.execute("UPDATE accounts SET totp_last_step=?1 WHERE id=?2 AND totp_last_step<?1",params![step,id]).map_err(|_|err(503,"Authenticator-Prüfung nicht gespeichert"))?;
            return Ok(changed==1);
        }
        let clean=code.trim().replace('-',"").to_ascii_uppercase();
        if clean.len()==24 && clean.bytes().all(|b|b.is_ascii_hexdigit()) {
            let changed=self.db.execute("UPDATE recovery_codes SET used_at=?1 WHERE account=?2 AND hash=?3 AND used_at IS NULL",params![now(),id,digest(&clean)]).map_err(|_|err(503,"Wiederherstellungscode nicht gespeichert"))?;
            if changed==1{self.identity_event(Some(id),"recovery_code_used",json!({}))?;return Ok(true);}
        }
        Ok(false)
    }
    pub fn authenticator_start(&mut self,a:&Account)->Reply {
        if self.authenticator_enabled(a.id){return Err(err(409,"Authenticator bereits verbunden"));}
        let mut secret=[0u8;20];OsRng.fill_bytes(&mut secret);
        let generator=totp(secret.to_vec(),&a.name)?;
        let encrypted=self.encrypt_totp(a.id,&secret)?;
        let qr=generator.get_qr_base64().map_err(|_|err(503,"QR-Code konnte nicht erstellt werden"))?;
        self.transaction(|g| {
            g.db.execute("UPDATE accounts SET totp_pending=?1,totp_pending_until=?2,totp_pending_attempts=0 WHERE id=?3",params![encrypted,now()+600,a.id]).map_err(|_|err(503,"Einrichtung nicht gespeichert"))?;
            g.identity_event(Some(a.id),"authenticator_setup_started",json!({}))?;
            Ok(json!({"qr_image":format!("data:image/png;base64,{qr}"),"otpauth_uri":generator.get_url(),"manual_key":generator.get_secret_base32(),"expires_in":600}))
        })
    }
    pub fn authenticator_enable(&mut self,a:&Account,v:&Value,session:&str)->Reply {
        let code=str_field(v,"code")?;
        let (secret,until,attempts):(Option<String>,Option<i64>,i64)=self.db.query_row("SELECT totp_pending,totp_pending_until,totp_pending_attempts FROM accounts WHERE id=?1",[a.id],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?))).map_err(|_|err(503,"Einrichtung nicht lesbar"))?;
        let secret=secret.filter(|_|until.is_some_and(|n|n>now())&&attempts<5).ok_or_else(||err(400,"Einrichtung abgelaufen; neuen QR-Code anfordern"))?;
        self.db.execute("UPDATE accounts SET totp_pending_attempts=totp_pending_attempts+1 WHERE id=?1",[a.id]).map_err(|_|err(503,"Codeversuch nicht gespeichert"))?;
        let generator=totp(self.decrypt_totp(a.id,&secret)?,&a.name)?;
        let step=matching_step(&generator,code,now()).ok_or_else(||err(400,"Code passt nicht. Prüfe den neuen Eintrag und die automatische Uhrzeit am Handy."))?;
        let recovery:Vec<String>=(0..8).map(|_|token()[..24].to_ascii_uppercase()).collect();
        self.transaction(|g| {
            g.db.execute("UPDATE accounts SET totp_secret=?1,totp_last_step=?2,totp_pending=NULL,totp_pending_until=NULL WHERE id=?3",params![secret,step,a.id]).map_err(|_|err(503,"Authenticator nicht aktiviert"))?;
            g.db.execute("DELETE FROM recovery_codes WHERE account=?1",[a.id]).map_err(|_|err(503,"Alte Wiederherstellungscodes nicht gelöscht"))?;
            for code in &recovery {g.db.execute("INSERT INTO recovery_codes(account,hash) VALUES(?1,?2)",params![a.id,digest(code)]).map_err(|_|err(503,"Wiederherstellungscodes nicht gespeichert"))?;}
            g.db.execute("DELETE FROM sessions WHERE account=?1 AND hash<>?2",params![a.id,digest(session)]).map_err(|_|err(503,"Alte Sitzungen nicht widerrufen"))?;
            if let Some(sid)=a.sid{g.runtime.leases.remove(&sid);}
            g.identity_event(Some(a.id),"authenticator_enabled",json!({}))?;
            Ok(json!({"enabled":true,"recovery_codes":recovery}))
        })
    }
    pub fn authenticator_disable(&mut self,a:&Account,v:&Value,session:&str)->Reply {
        if !self.authenticator_enabled(a.id){return Err(err(409,"Kein Authenticator verbunden"));}
        if !self.check_second_factor(a.id,str_field(v,"code")?)?{return Err(err(401,"Authenticator- oder Wiederherstellungscode ungültig"));}
        self.transaction(|g| {
            g.db.execute("UPDATE accounts SET totp_secret=NULL,totp_pending=NULL,totp_pending_until=NULL,totp_last_step=-1 WHERE id=?1",[a.id]).map_err(|_|err(503,"Authenticator nicht entfernt"))?;
            g.db.execute("DELETE FROM recovery_codes WHERE account=?1",[a.id]).map_err(|_|err(503,"Wiederherstellungscodes nicht entfernt"))?;
            g.db.execute("DELETE FROM sessions WHERE account=?1 AND hash<>?2",params![a.id,digest(session)]).map_err(|_|err(503,"Alte Sitzungen nicht widerrufen"))?;
            g.identity_event(Some(a.id),"authenticator_disabled",json!({}))?;
            Ok(json!({"enabled":false}))
        })
    }
}
