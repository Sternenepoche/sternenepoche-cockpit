//! Öffnet eine Adresse im normalen Browser des Systems. Nur http(s), keine Programme.
pub fn browser(url: &str) {
    if !(url.starts_with("https://") || url.starts_with("http://127.0.0.1") || url.starts_with("http://localhost")) { return; }
    #[cfg(windows)]
    unsafe {
        use windows_sys::Win32::UI::Shell::ShellExecuteW;
        use windows_sys::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;
        let breit = |s: &str| s.encode_utf16().chain(std::iter::once(0)).collect::<Vec<u16>>();
        let (oeffnen, ziel) = (breit("open"), breit(url));
        ShellExecuteW(std::ptr::null_mut(), oeffnen.as_ptr(), ziel.as_ptr(), std::ptr::null(), std::ptr::null(), SW_SHOWNORMAL);
    }
    #[cfg(target_os = "macos")]
    { let _ = std::process::Command::new("open").arg(url).spawn(); }
    #[cfg(all(unix, not(target_os = "macos")))]
    { let _ = std::process::Command::new("xdg-open").arg(url).spawn(); }
}

/// Öffnet eine eigene Textdatei (soul.md, Skill, Export) mit dem zugeordneten Programm;
/// ohne Zuordnung unter Windows mit dem Editor. Nur .md, .json und .jsonl.
pub fn datei(pfad: &std::path::Path) {
    if !pfad.is_file() || !["md", "json", "jsonl"].contains(&pfad.extension().and_then(|e| e.to_str()).unwrap_or("")) { return; }
    #[cfg(windows)]
    unsafe {
        use windows_sys::Win32::UI::Shell::ShellExecuteW;
        use windows_sys::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;
        let breit = |s: &str| s.encode_utf16().chain(std::iter::once(0)).collect::<Vec<u16>>();
        let (oeffnen, ziel) = (breit("open"), breit(&pfad.display().to_string()));
        let r = ShellExecuteW(std::ptr::null_mut(), oeffnen.as_ptr(), ziel.as_ptr(), std::ptr::null(), std::ptr::null(), SW_SHOWNORMAL);
        if (r as isize) <= 32 {
            let (editor, parameter) = (breit("notepad.exe"), breit(&format!("\"{}\"", pfad.display())));
            ShellExecuteW(std::ptr::null_mut(), oeffnen.as_ptr(), editor.as_ptr(), parameter.as_ptr(), std::ptr::null(), SW_SHOWNORMAL);
        }
    }
    #[cfg(target_os = "macos")]
    { let _ = std::process::Command::new("open").arg(pfad).spawn(); }
    #[cfg(all(unix, not(target_os = "macos")))]
    { let _ = std::process::Command::new("xdg-open").arg(pfad).spawn(); }
}

/// Öffnet einen lokalen Ordner im Dateimanager (nur existierende Verzeichnisse).
pub fn ordner(pfad: &std::path::Path) {
    if !pfad.is_dir() { return; }
    #[cfg(windows)]
    unsafe {
        use windows_sys::Win32::UI::Shell::ShellExecuteW;
        use windows_sys::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;
        let breit = |s: &str| s.encode_utf16().chain(std::iter::once(0)).collect::<Vec<u16>>();
        let (oeffnen, ziel) = (breit("open"), breit(&pfad.display().to_string()));
        ShellExecuteW(std::ptr::null_mut(), oeffnen.as_ptr(), ziel.as_ptr(), std::ptr::null(), std::ptr::null(), SW_SHOWNORMAL);
    }
    #[cfg(target_os = "macos")]
    { let _ = std::process::Command::new("open").arg(pfad).spawn(); }
    #[cfg(all(unix, not(target_os = "macos")))]
    { let _ = std::process::Command::new("xdg-open").arg(pfad).spawn(); }
}
