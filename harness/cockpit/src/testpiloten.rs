//! Private Testzugänge bleiben auf diesem Rechner und gelangen nie ins Download-Paket.
use std::path::Path;
pub const NAMEN: [&str; 5] = ["LiveDemo", "TestPilot2", "TestPilot3", "TestPilot4", "TestPilot5"];
fn zugang(text: &str, name: &str) -> Option<String> {
    if !NAMEN.contains(&name) || text.len() > 16_384 { return None; }
    let mut zeilen=text.lines();
    while let Some(z)=zeilen.next() {
        if z.strip_prefix("Testkonto-Name: ")==Some(name) {
            return zeilen.next()?.strip_prefix("Passwort: ").filter(|p| !p.is_empty() && p.len()<=128).map(str::to_string);
        }
    }
    None
}
pub fn lesen(ordner: &Path, name: &str) -> Result<String,String> {
    let pfad=ordner.join("testpiloten-zugaenge.txt");
    let m=std::fs::metadata(&pfad).map_err(|_|"Private Testzugänge fehlen auf diesem PC. Die lokale Zugangsliste liegt im Harness-Ordner als testpiloten-zugaenge.txt.".to_string())?;
    if m.len()>16_384 {return Err("Lokale Zugangsliste ist zu groß".into());}
    zugang(&std::fs::read_to_string(pfad).map_err(|_|"Lokale Zugangsliste nicht lesbar")?,name).ok_or_else(||"Dieser Testpilot hat hier keinen gespeicherten Zugang".into())
}
#[cfg(test)] mod tests {
    use super::*;
    #[test] fn nur_bekannte_private_zugaenge() {
        let t="Testkonto-Name: TestPilot2\r\nPasswort: lokal-test\r\nTestkonto-Name: Fremder\nPasswort: unerlaubt";
        assert_eq!(zugang(t,"TestPilot2").as_deref(),Some("lokal-test"));
        assert!(zugang(t,"Fremder").is_none()); assert!(zugang(t,"LiveDemo").is_none());
        assert!(zugang(&"x".repeat(16_385),"LiveDemo").is_none());
    }
}
