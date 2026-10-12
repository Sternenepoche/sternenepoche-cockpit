//! Energiewächter: Beispiel für ein Strategie-Plugin der Harness-Sandbox.
//!
//! Hält Neubauten zurück, solange ein Planet mehr Energie verbraucht als erzeugt,
//! und schlägt stattdessen ein Kraftwerk vor. Das Plugin sieht nur, was der Harness
//! ihm gibt (Rolle, Kurzlage, Vorschlag), und kann nichts importieren: kein Netz,
//! keine Dateien, keine Uhr. Vertrag: `alloc`, `bewerten`, `memory`.
#![no_std]
extern crate alloc;
use alloc::{string::ToString, vec::Vec};
use serde_json::{json, Value};

#[global_allocator]
static ALLOC: Zuteiler = Zuteiler;

/// Einfacher Stapelzuteiler: jeder Aufruf bekommt ohnehin einen frischen Speicher.
struct Zuteiler;
static mut OBEN: usize = 0;
unsafe impl core::alloc::GlobalAlloc for Zuteiler {
    unsafe fn alloc(&self, l: core::alloc::Layout) -> *mut u8 {
        extern "C" { static __heap_base: u8; }
        let basis = core::ptr::addr_of!(__heap_base) as usize;
        if OBEN < basis { OBEN = basis; }
        let start = (OBEN + l.align() - 1) & !(l.align() - 1);
        let ende = start + l.size();
        let seiten = core::arch::wasm32::memory_size(0) * 65536;
        if ende > seiten && core::arch::wasm32::memory_grow(0, (ende - seiten).div_ceil(65536)) == usize::MAX { return core::ptr::null_mut(); }
        OBEN = ende;
        start as *mut u8
    }
    unsafe fn dealloc(&self, _: *mut u8, _: core::alloc::Layout) {}
}

#[panic_handler]
fn panik(_: &core::panic::PanicInfo) -> ! { core::arch::wasm32::unreachable() }

#[no_mangle]
pub extern "C" fn alloc(laenge: i32) -> i32 {
    let mut v: Vec<u8> = Vec::with_capacity(laenge as usize);
    let p = v.as_mut_ptr();
    core::mem::forget(v);
    p as i32
}

#[no_mangle]
pub extern "C" fn bewerten(zeiger: i32, laenge: i32) -> i64 {
    let eingabe = unsafe { core::slice::from_raw_parts(zeiger as *const u8, laenge as usize) };
    let antwort = entscheiden(serde_json::from_slice(eingabe).unwrap_or(Value::Null));
    let text = antwort.to_string().into_bytes();
    let (p, n) = (text.as_ptr() as i64, text.len() as i64);
    core::mem::forget(text);
    (p << 32) | n
}

fn entscheiden(e: Value) -> Value {
    let defizit: Vec<&str> = e["lage"]["planeten"].as_array().map(|p| p.iter()
        .filter(|p| p["energie"]["verbrauch"].as_f64().unwrap_or(0.0) > p["energie"]["erzeugung"].as_f64().unwrap_or(0.0))
        .filter_map(|p| p["koord"].as_str()).collect()).unwrap_or_default();
    if defizit.is_empty() { return json!({"hinweise":["Energie ausgeglichen"]}); }
    let mut aktionen: Vec<Value> = Vec::new();
    for a in e["aktionen"].as_array().into_iter().flatten() {
        let neubau = a["typ"] == "bauen" && !a["gebaeude"].as_str().is_some_and(|g| g.contains("kraftwerk"));
        let auf_defizit = a["planet"].as_str().is_some_and(|k| defizit.contains(&k));
        if !(neubau && auf_defizit) { aktionen.push(a.clone()); }
    }
    for k in &defizit {
        if !aktionen.iter().any(|a| a["planet"] == *k && a["gebaeude"].as_str().is_some_and(|g| g.contains("kraftwerk"))) {
            aktionen.push(json!({"typ":"bauen","planet":k,"gebaeude":"solarkraftwerk"}));
        }
    }
    aktionen.truncate(8);
    json!({"aktionen":aktionen,"hinweise":[alloc::format!("Energiedefizit auf {}: Kraftwerk zuerst", defizit.join(", "))]})
}
