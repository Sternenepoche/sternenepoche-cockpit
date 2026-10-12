//! Wiederaufnahme enthält ausschließlich die lokale Spielauswahl, keine Zugangsdaten.
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Fortsetzung {
    pub aktiv: bool,
    pub name: String,
    pub volk: String,
    pub bots: u16,
    pub host: bool,
    pub port: u16,
}
impl Fortsetzung {
    pub fn laden(root: &Path) -> Result<Option<Self>, String> {
        let p = root.join("lokale-fortsetzung.json");
        if !p.exists() { return Ok(None); }
        let raw = std::fs::read(p).map_err(|e| e.to_string())?;
        if raw.len() > 2048 { return Err("Ungültige lokale Wiederaufnahme".into()); }
        let s: Self = serde_json::from_slice(&raw).map_err(|e| e.to_string())?;
        sternenepoche_server::local::dimensions(s.bots)?;
        if s.name.trim().is_empty() || s.name.chars().count() > 80 || !["veyari","aurelianer","krath","syntheten"].contains(&s.volk.as_str()) || s.port < 1024 {
            return Err("Ungültige lokale Wiederaufnahme".into());
        }
        Ok(Some(s))
    }
    pub fn speichern(&self, root: &Path) -> Result<(), String> {
        std::fs::create_dir_all(root).map_err(|e| e.to_string())?;
        let p = root.join("lokale-fortsetzung.json");
        let tmp = root.join("lokale-fortsetzung.tmp");
        let mut f = std::fs::File::create(&tmp).map_err(|e| e.to_string())?;
        use std::io::Write;
        f.write_all(&serde_json::to_vec(self).map_err(|e| e.to_string())?).and_then(|_| f.sync_all()).map_err(|e| e.to_string())?;
        drop(f);
        std::fs::rename(tmp, p).map_err(|e| format!("Wiederaufnahme nicht gespeichert: {e}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn wiederaufnahme_ueberschreiben_und_verlassen() {
        let p = std::env::temp_dir().join(format!("cockpit-resume-{}",sternenepoche_server::token()));
        let mut s = Fortsetzung { aktiv:true,name:"Entdecker".into(),volk:"veyari".into(),bots:30,host:false,port:8892 };
        s.speichern(&p).unwrap();
        assert_eq!(Fortsetzung::laden(&p).unwrap(),Some(s.clone()));
        s.aktiv=false; s.speichern(&p).unwrap();
        assert!(!Fortsetzung::laden(&p).unwrap().unwrap().aktiv);
        std::fs::write(p.join("lokale-fortsetzung.json"),"{\"token\":\"secret\"}").unwrap();
        assert!(Fortsetzung::laden(&p).is_err());
        std::fs::remove_dir_all(p).unwrap();
    }
}
