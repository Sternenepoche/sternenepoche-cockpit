//! WASM-Sandbox für eigene Strategie-Plugins (Wasmi-Interpreter, kein JIT).
//!
//! Vertrag: Das Modul exportiert `memory`, `alloc(len: i32) -> i32` und
//! `bewerten(ptr: i32, len: i32) -> i64` (Rückgabe: Zeiger << 32 | Länge einer JSON-Antwort).
//! Eingabe: {"rolle","lage","aktionen"}; Antwort: {"aktionen"?: [...], "hinweise"?: [...]}.
//! Ein Plugin darf nichts importieren: keine Dateien, kein Netz, keine Uhr, kein Zufall.
//! Jeder Aufruf läuft in einem frischen Speicher mit Treibstoff- und Speichergrenze.
use crate::konfig::PluginKonfig;
use serde_json::Value;
use std::path::Path;
use wasmi::{Config, Engine, Linker, Module, Store, StoreLimits, StoreLimitsBuilder};

impl std::fmt::Debug for Plugin {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { write!(f, "Plugin({})", self.name) }
}

pub struct Plugin {
    pub name: String,
    pub rollen: Vec<String>,
    engine: Engine,
    modul: Module,
    treibstoff: u64,
    speicher: usize,
}

impl Plugin {
    pub fn laden(k: &PluginKonfig, basis: &Path) -> Result<Self, String> {
        let pfad = if Path::new(&k.pfad).is_absolute() { k.pfad.clone().into() } else { basis.join(&k.pfad) };
        let bytes = std::fs::read(&pfad).map_err(|e| format!("Plugin {}: {e}", pfad.display()))?;
        Self::aus_bytes(&k.name, &bytes, k.treibstoff, k.speicher_mb as usize, k.rollen.clone())
    }
    pub fn aus_bytes(name: &str, bytes: &[u8], treibstoff: u64, speicher_mb: usize, rollen: Vec<String>) -> Result<Self, String> {
        if bytes.len() > 8 * 1024 * 1024 { return Err(format!("Plugin {name}: größer als 8 MB")); }
        let mut cfg = Config::default();
        cfg.consume_fuel(true);
        cfg.set_max_stack_height(64 * 1024);
        let engine = Engine::new(&cfg);
        let modul = Module::new(&engine, bytes).map_err(|e| format!("Plugin {name}: kein gültiges WASM-Modul ({e})"))?;
        if let Some(i) = modul.imports().next() {
            return Err(format!("Plugin {name}: importiert {}::{} – Plugins dürfen nichts importieren", i.module(), i.name()));
        }
        Ok(Self { name: name.into(), rollen, engine, modul, treibstoff, speicher: speicher_mb * 1024 * 1024 })
    }

    pub fn bewerten(&self, eingabe: &Value) -> Result<Value, String> {
        let limits = StoreLimitsBuilder::new().memory_size(self.speicher).instances(1).memories(1).tables(1).build();
        let mut store: Store<StoreLimits> = Store::new(&self.engine, limits);
        store.limiter(|l| l);
        store.set_fuel(self.treibstoff).map_err(|e| e.to_string())?;
        let linker = Linker::<StoreLimits>::new(&self.engine);
        let instanz = linker.instantiate_and_start(&mut store, &self.modul).map_err(|e| format!("Start fehlgeschlagen: {e}"))?;
        let speicher = instanz.get_memory(&store, "memory").ok_or("Export memory fehlt")?;
        let alloc = instanz.get_typed_func::<i32, i32>(&store, "alloc").map_err(|_| "Export alloc(i32)->i32 fehlt")?;
        let bewerten = instanz.get_typed_func::<(i32, i32), i64>(&store, "bewerten").map_err(|_| "Export bewerten(i32,i32)->i64 fehlt")?;
        let daten = eingabe.to_string().into_bytes();
        if daten.len() > 1024 * 1024 { return Err("Eingabe zu groß".into()); }
        let zeiger = alloc.call(&mut store, daten.len() as i32).map_err(|e| self.fehler(e))?;
        speicher.write(&mut store, zeiger as u32 as usize, &daten).map_err(|_| "Eingabe passt nicht in den Plugin-Speicher")?;
        let r = bewerten.call(&mut store, (zeiger, daten.len() as i32)).map_err(|e| self.fehler(e))? as u64;
        let (aus, laenge) = ((r >> 32) as usize, (r & 0xffff_ffff) as usize);
        if laenge > 1024 * 1024 { return Err("Antwort größer als 1 MB".into()); }
        let mut puffer = vec![0u8; laenge];
        speicher.read(&store, aus, &mut puffer).map_err(|_| "Antwort liegt außerhalb des Plugin-Speichers")?;
        let v: Value = serde_json::from_slice(&puffer).map_err(|_| "Antwort ist kein JSON")?;
        if !v.is_object() { return Err("Antwort muss ein JSON-Objekt sein".into()); }
        if let Some(a) = v.get("aktionen") {
            if !a.as_array().is_some_and(|a| a.len() <= 8 && a.iter().all(|x| x.is_object() && x["typ"].is_string())) { return Err("aktionen: höchstens acht Objekte mit typ".into()); }
        }
        Ok(v)
    }
    fn fehler(&self, e: wasmi::Error) -> String {
        let t = e.to_string();
        if t.contains("fuel") { format!("Treibstoff von {} aufgebraucht (Endlosschleife?)", self.treibstoff) } else { format!("Plugin-Fehler: {t}") }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    const ANTWORT: &str = r#"{"hinweise":["Energie zuerst"],"aktionen":[{"typ":"bauen","planet":"1:2:3","gebaeude":"solarkraftwerk"}]}"#;
    // Wasmi liest mit dem Standardbaustein „wat“ auch die Textform.
    fn wat(rumpf: &str) -> Vec<u8> {
        (format!(r#"(module (memory (export "memory") 1)
            (data (i32.const 1024) "{}")
            (func (export "alloc") (param i32) (result i32) (i32.const 4096))
            (func (export "bewerten") (param i32 i32) (result i64) {rumpf}))"#, ANTWORT.replace('"', "\\\""))).into_bytes()
    }
    fn gute_antwort() -> String { format!("(i64.or (i64.shl (i64.const 1024) (i64.const 32)) (i64.const {}))", ANTWORT.len()) }
    /// Das Rust-Beispiel aus harness/beispiele/energiewaechter, falls gebaut.
    #[test]
    fn beispiel_energiewaechter() {
        let pfad = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../beispiele/energiewaechter/target/wasm32-unknown-unknown/release/energiewaechter.wasm");
        let Ok(bytes) = std::fs::read(&pfad) else { eprintln!("Beispiel nicht gebaut, übersprungen"); return };
        let p = Plugin::aus_bytes("energiewaechter", &bytes, 50_000_000, 16, vec![]).unwrap();
        let lage = json!({"planeten":[{"koord":"1:2:3","energie":{"erzeugung":10,"verbrauch":25}},{"koord":"1:2:4","energie":{"erzeugung":30,"verbrauch":5}}]});
        let v = p.bewerten(&json!({"rolle":"verwalter","lage":lage,"aktionen":[{"typ":"bauen","planet":"1:2:3","gebaeude":"erzmine"},{"typ":"bauen","planet":"1:2:4","gebaeude":"erzmine"}]})).unwrap();
        let a = v["aktionen"].as_array().unwrap();
        assert!(a.iter().all(|x| !(x["planet"] == "1:2:3" && x["gebaeude"] == "erzmine")), "Neubau auf Defizitplanet zurückgehalten");
        assert!(a.iter().any(|x| x["planet"] == "1:2:3" && x["gebaeude"] == "solarkraftwerk"));
        assert!(a.iter().any(|x| x["planet"] == "1:2:4" && x["gebaeude"] == "erzmine"));
        assert!(v["hinweise"][0].as_str().unwrap().contains("1:2:3"));
    }
    #[test]
    fn plugin_antwortet_und_wird_begrenzt() {
        let energie = Plugin::aus_bytes("energie", &wat(&gute_antwort()), 1_000_000, 4, vec![]).unwrap();
        let v = energie.bewerten(&json!({"rolle":"verwalter","lage":{},"aktionen":[]})).unwrap();
        assert_eq!(v["aktionen"][0]["gebaeude"], "solarkraftwerk");
        let schleife = Plugin::aus_bytes("schleife", &wat("(loop $l (br $l)) (i64.const 0)"), 100_000, 4, vec![]).unwrap();
        assert!(schleife.bewerten(&json!({})).unwrap_err().contains("Treibstoff"));
        // Wächst der Speicher trotz Grenze, bricht das Modul ab; wird es verweigert, antwortet es normal.
        let wachsen = Plugin::aus_bytes("gierig", &wat(&format!("(drop (memory.grow (i32.const 1000))) (if (i32.ne (memory.size) (i32.const 1)) (then (unreachable))) {}", gute_antwort())), 1_000_000, 4, vec![]).unwrap();
        assert!(wachsen.bewerten(&json!({})).is_ok(), "Wachstum über die Speichergrenze wird verweigert");
        let mit_import = br#"(module (import "env" "dateien_lesen" (func)) (memory (export "memory") 1))"#.to_vec();
        assert!(Plugin::aus_bytes("spion", &mit_import, 1000, 4, vec![]).unwrap_err().contains("nichts importieren"));
        let ausserhalb = Plugin::aus_bytes("raus", &wat("(i64.or (i64.shl (i64.const 999999) (i64.const 32)) (i64.const 50))"), 1_000_000, 4, vec![]).unwrap();
        assert!(ausserhalb.bewerten(&json!({})).is_err());
    }
}
