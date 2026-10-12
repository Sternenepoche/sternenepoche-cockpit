//! Prozess-Käfig des Harness: Er startet nie andere Programme und schreibt nur in
//! seinen eigenen Ordner. Windows: Job Object ohne Kindprozesse und mit Speichergrenze.
//! Linux: Landlock beschränkt Schreibzugriffe auf den Harness-Ordner, no_new_privs.
use std::path::Path;

#[cfg(windows)]
pub fn einsperren(_basis: &Path, speicher_mb: usize) -> Result<String, String> {
    use std::mem::size_of;
    use std::ptr::null;
    use windows_sys::Win32::System::JobObjects::*;
    use windows_sys::Win32::System::Threading::GetCurrentProcess;
    unsafe {
        let job = CreateJobObjectW(null(), null());
        if job.is_null() { return Err("Job Object nicht erstellt".into()); }
        let mut l = JOBOBJECT_EXTENDED_LIMIT_INFORMATION::default();
        l.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_ACTIVE_PROCESS | JOB_OBJECT_LIMIT_PROCESS_MEMORY | JOB_OBJECT_LIMIT_DIE_ON_UNHANDLED_EXCEPTION;
        l.BasicLimitInformation.ActiveProcessLimit = 1;
        l.ProcessMemoryLimit = speicher_mb * 1024 * 1024;
        if SetInformationJobObject(job, JobObjectExtendedLimitInformation, (&l as *const JOBOBJECT_EXTENDED_LIMIT_INFORMATION).cast(), size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32) == 0 {
            return Err("Job-Grenzen nicht gesetzt".into());
        }
        if AssignProcessToJobObject(job, GetCurrentProcess()) == 0 { return Err("Job Object nicht zugeordnet".into()); }
        Ok(format!("Käfig aktiv: keine Kindprozesse, höchstens {speicher_mb} MB"))
    }
}

#[cfg(target_os = "linux")]
pub fn einsperren(basis: &Path, _speicher_mb: usize) -> Result<String, String> {
    use landlock::{Access, AccessFs, PathBeneath, PathFd, Ruleset, RulesetAttr, RulesetCreatedAttr, RulesetStatus, ABI};
    let abi = ABI::V2;
    let status = Ruleset::default()
        .handle_access(AccessFs::from_all(abi)).map_err(|e| e.to_string())?
        .create().map_err(|e| e.to_string())?
        .add_rule(PathBeneath::new(PathFd::new("/").map_err(|e| e.to_string())?, AccessFs::from_read(abi))).map_err(|e| e.to_string())?
        .add_rule(PathBeneath::new(PathFd::new(basis).map_err(|e| e.to_string())?, AccessFs::from_all(abi))).map_err(|e| e.to_string())?
        .restrict_self().map_err(|e| e.to_string())?;
    Ok(match status.ruleset {
        RulesetStatus::FullyEnforced => "Käfig aktiv: Schreiben nur im Harness-Ordner (Landlock)".into(),
        RulesetStatus::PartiallyEnforced => "Käfig teilweise aktiv (älterer Kernel)".into(),
        RulesetStatus::NotEnforced => "Landlock vom Kernel nicht unterstützt; Harness läuft ohne Käfig".into(),
    })
}

#[cfg(not(any(windows, target_os = "linux")))]
pub fn einsperren(_basis: &Path, _speicher_mb: usize) -> Result<String, String> {
    Ok("Auf diesem System ohne Prozess-Käfig".into())
}
