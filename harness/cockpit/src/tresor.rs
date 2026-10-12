//! Geheimnisse (Harness-Schlüssel, OpenRouter-Schlüssel) liegen im Anmeldeinformations-
//! speicher von Windows, nie in harness.json oder Logdateien. Auf anderen Systemen in
//! einer Datei mit Rechten 0600 im Arbeitsordner.
use std::path::Path;

fn ziel(name: &str) -> String { format!("Sternenepoche/{name}") }

#[cfg(windows)]
mod os {
    use windows_sys::Win32::Security::Credentials::*;
    fn breit(s: &str) -> Vec<u16> { s.encode_utf16().chain(std::iter::once(0)).collect() }
    pub fn schreiben(ziel: &str, wert: &str) -> Result<(), String> {
        let mut name = breit(ziel);
        let mut benutzer = breit("sternenepoche");
        let mut daten = wert.as_bytes().to_vec();
        let cred = CREDENTIALW { Type: CRED_TYPE_GENERIC, TargetName: name.as_mut_ptr(), CredentialBlobSize: daten.len() as u32, CredentialBlob: daten.as_mut_ptr(),
            Persist: CRED_PERSIST_LOCAL_MACHINE, UserName: benutzer.as_mut_ptr(), ..unsafe { std::mem::zeroed() } };
        if unsafe { CredWriteW(&cred, 0) } == 0 { return Err("Anmeldeinformation nicht gespeichert".into()); }
        Ok(())
    }
    pub fn lesen(ziel: &str) -> Option<String> {
        let name = breit(ziel);
        let mut p: *mut CREDENTIALW = std::ptr::null_mut();
        if unsafe { CredReadW(name.as_ptr(), CRED_TYPE_GENERIC, 0, &mut p) } == 0 { return None; }
        let wert = unsafe { let c = &*p; String::from_utf8(std::slice::from_raw_parts(c.CredentialBlob, c.CredentialBlobSize as usize).to_vec()).ok() };
        unsafe { CredFree(p.cast()) };
        wert
    }
    pub fn loeschen(ziel: &str) { let name = breit(ziel); unsafe { CredDeleteW(name.as_ptr(), CRED_TYPE_GENERIC, 0) }; }
}

#[cfg(not(windows))]
mod os {
    fn datei(ziel: &str) -> std::path::PathBuf { crate::katalog::laden().ordner().join(format!(".{}", ziel.replace('/', "-"))) }
    pub fn schreiben(ziel: &str, wert: &str) -> Result<(), String> {
        use std::os::unix::fs::OpenOptionsExt;
        use std::io::Write;
        let mut f = std::fs::OpenOptions::new().write(true).create(true).truncate(true).mode(0o600).open(datei(ziel)).map_err(|e| e.to_string())?;
        f.write_all(wert.as_bytes()).map_err(|e| e.to_string())
    }
    pub fn lesen(ziel: &str) -> Option<String> { std::fs::read_to_string(datei(ziel)).ok() }
    pub fn loeschen(ziel: &str) { let _ = std::fs::remove_file(datei(ziel)); }
}

/// Schlüssel je Server, damit ein Wechsel des Servers keinen falschen Schlüssel nutzt.
pub fn harness_name(server: &str) -> String { format!("harness:{}", server.trim_end_matches('/').replace("https://", "").replace("http://", "")) }
/// Harness-Schlüssel einer lokalen Welt: je Welt, nicht je Adresse. Der Singleplayer startet
/// jedes Mal auf einem freien Zufallsport; ein Name je Adresse hätte bei jedem Start einen
/// neuen Schlüssel verlangt und alte im Speicher liegen lassen.
pub fn harness_lokal(world_id: &str) -> String { format!("harness:lokal:{world_id}") }
/// Gerätegeheimnis dieses Rechners für lokale Runden anderer Hosts (bindet den Spielernamen
/// an dieses Gerät). Entsteht beim ersten Beitritt und bleibt im Anmeldeinformationsspeicher.
pub fn geraet() -> Result<String, String> {
    if let Some(g) = lesen("lokal-geraet").filter(|g| g.len() == 48 && g.bytes().all(|b| b.is_ascii_hexdigit())) { return Ok(g); }
    let g = sternenepoche_server::token()[..48].to_string();
    schreiben("lokal-geraet", &g)?;
    Ok(g)
}
/// Schlüssel einer Wissensquelle (eigenes RAG, MCP-Server).
pub fn wissen_name(quelle: &str) -> String { format!("wissen:{quelle}") }
pub fn schreiben(name: &str, wert: &str) -> Result<(), String> { os::schreiben(&ziel(name), wert) }
pub fn lesen(name: &str) -> Option<String> { os::lesen(&ziel(name)).filter(|s| !s.is_empty()) }
pub fn loeschen(name: &str) { os::loeschen(&ziel(name)) }
#[allow(dead_code)]
pub fn ort(_: &Path) -> &'static str { if cfg!(windows) { "Windows-Anmeldeinformationsspeicher" } else { "Datei mit Rechten 0600 im Arbeitsordner" } }

#[cfg(all(test, windows))]
mod tests {
    #[test]
    fn schreiben_lesen_loeschen() {
        let n = format!("test-{}", std::process::id());
        super::schreiben(&n, "sep_geheim").unwrap();
        assert_eq!(super::lesen(&n).as_deref(), Some("sep_geheim"));
        super::loeschen(&n);
        assert!(super::lesen(&n).is_none());
    }
}
