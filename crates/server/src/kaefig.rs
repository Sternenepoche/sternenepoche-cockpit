//! Prozess-Käfig des Spielservers. Selbst wenn ein Fehler je Codeausführung erlaubte,
//! darf der Prozess keine weiteren Programme starten (keine Shell, kein Nachladen
//! fremder Werkzeuge) und keinen unbegrenzten Speicher belegen.
//! Unter Linux übernimmt das die systemd-Einheit (NoNewPrivileges, TasksMax, MemoryMax,
//! SystemCallFilter); unter Windows ein Job Object, dem sich der Server selbst zuordnet.

#[cfg(windows)]
pub fn einsperren(speicher_mb: usize) -> Result<String, String> {
    use std::mem::size_of;
    use std::ptr::null;
    use windows_sys::Win32::System::JobObjects::*;
    use windows_sys::Win32::System::Threading::GetCurrentProcess;
    unsafe {
        let job = CreateJobObjectW(null(), null());
        if job.is_null() { return Err("Job Object nicht erstellt".into()); }
        let mut limits = JOBOBJECT_EXTENDED_LIMIT_INFORMATION::default();
        limits.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_ACTIVE_PROCESS
            | JOB_OBJECT_LIMIT_PROCESS_MEMORY
            | JOB_OBJECT_LIMIT_DIE_ON_UNHANDLED_EXCEPTION;
        limits.BasicLimitInformation.ActiveProcessLimit = 1;
        limits.ProcessMemoryLimit = speicher_mb * 1024 * 1024;
        if SetInformationJobObject(job, JobObjectExtendedLimitInformation, (&limits as *const JOBOBJECT_EXTENDED_LIMIT_INFORMATION).cast(), size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32) == 0 {
            return Err("Job-Grenzen nicht gesetzt".into());
        }
        if AssignProcessToJobObject(job, GetCurrentProcess()) == 0 {
            return Err("Server konnte sich keinem Job Object zuordnen (bereits in einem unveränderlichen Job?)".into());
        }
        // Der Griff bleibt bewusst offen: der Käfig gilt für die ganze Prozesslaufzeit.
        Ok(format!("Prozess-Käfig aktiv: keine Kindprozesse, höchstens {speicher_mb} MB"))
    }
}

#[cfg(not(windows))]
pub fn einsperren(_speicher_mb: usize) -> Result<String, String> {
    Ok("Prozess-Käfig über die systemd-Einheit (deploy/sternenepoche.service)".into())
}

#[cfg(all(test, windows))]
mod tests {
    #[test]
    fn eingesperrter_prozess_startet_keine_kinder() {
        // Eigener Prozess, damit der Testläufer selbst nicht eingesperrt wird.
        if std::env::var("STERNENEPOCHE_KAEFIG_PROBE").is_ok() {
            super::einsperren(512).unwrap();
            let kind = std::process::Command::new("cmd").args(["/c", "exit", "0"]).status();
            std::process::exit(if kind.is_err() { 0 } else { 1 });
        }
        let exe = std::env::current_exe().unwrap();
        let status = std::process::Command::new(exe)
            .args(["kaefig::tests::eingesperrter_prozess_startet_keine_kinder", "--exact", "--nocapture"])
            .env("STERNENEPOCHE_KAEFIG_PROBE", "1").status().unwrap();
        assert!(status.success(), "Kindprozess im Käfig wurde nicht verhindert");
    }
}
