//! Der gebaute Harness läuft als eigener, eingesperrter Prozess. Geheimnisse gehen
//! einmalig über stdin hinein, Steuerbefehle folgen als JSON-Zeilen, Ereignisse kommen
//! als JSON-Zeilen über stdout zurück. Schließt das Cockpit, endet auch der Harness.
use serde_json::{json, Value};
use std::io::{BufRead, BufReader, Write};
use std::path::Path;
use std::process::{Child, ChildStdin, Stdio};
use std::sync::mpsc::{channel, Receiver};

pub struct Laufzeit { kind: Child, eingabe: ChildStdin, pub ereignisse: Receiver<Value> }

impl Laufzeit {
    pub fn starten(ordner: &Path, schluessel: &str, openrouter: &str, wissen: &std::collections::BTreeMap<String, String>) -> Result<Self, String> {
        let exe = crate::bau::harness_exe(ordner);
        let mut kind = crate::bau::kommando(&exe).arg("spielen").arg("--konfig").arg(ordner.join("harness.json")).arg("--geheimnisse-stdin")
            .current_dir(ordner).stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped()).spawn().map_err(|e| format!("{}: {e}", exe.display()))?;
        let mut eingabe = kind.stdin.take().unwrap();
        writeln!(eingabe, "{}", json!({"schluessel":schluessel,"openrouter":openrouter,"openai":crate::tresor::lesen("eigen").unwrap_or_default(),"wissen":wissen})).map_err(|e| e.to_string())?;
        eingabe.flush().map_err(|e| e.to_string())?;
        let (tx, rx) = channel();
        let aus = kind.stdout.take().unwrap();
        let tx2 = tx.clone();
        std::thread::spawn(move || for z in BufReader::new(aus).lines().map_while(Result::ok) {
            let v = serde_json::from_str(&z).unwrap_or_else(|_| json!({"art":"text","text":z}));
            if tx2.send(v).is_err() { break; }
        });
        let fehler = kind.stderr.take().unwrap();
        std::thread::spawn(move || for z in BufReader::new(fehler).lines().map_while(Result::ok) {
            if tx.send(json!({"art":"protokoll","text":z})).is_err() { break; }
        });
        Ok(Self { kind, eingabe, ereignisse: rx })
    }
    pub fn befehl(&mut self, b: Value) -> Result<(), String> {
        writeln!(self.eingabe, "{b}").and_then(|_| self.eingabe.flush()).map_err(|_| "Harness läuft nicht mehr".to_string())
    }
    pub fn laeuft(&mut self) -> bool { matches!(self.kind.try_wait(), Ok(None)) }
    pub fn stoppen(&mut self) {
        let _ = self.befehl(json!({"befehl":"stop"}));
        for _ in 0..40 { if !self.laeuft() { return; } std::thread::sleep(std::time::Duration::from_millis(100)); }
        let _ = self.kind.kill();
    }
}
impl Drop for Laufzeit { fn drop(&mut self) { if self.laeuft() { self.stoppen(); } } }
