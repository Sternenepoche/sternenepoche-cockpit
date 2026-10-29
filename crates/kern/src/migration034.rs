//! The published Android 0.3.4 branch used 11/12 for workforce/discovery,
//! whereas the merged branch used those labels for fleet/content layouts.
use crate::{Welt,typen::*};
use serde_json::Value;
fn split(b:&[u8])->Result<(&[u8],Value),String>{
 let n=u64::from_le_bytes(b.get(8..16).ok_or("Unvollständiger alter Spielstand")?.try_into().unwrap()) as usize;
 let end=16usize.checked_add(n).filter(|n|*n<=b.len()).ok_or("Ungültige alte Snapshotlänge")?;
 Ok((&b[16..end],serde_json::from_slice(&b[end..]).map_err(|e|format!("Alter Spielstand: {e}"))?))
}
pub fn laden(b:&[u8])->Result<Option<Welt>,String>{
 if !b.starts_with(b"STERNE12"){return Ok(None);}
 let(mut base,discovery)=split(b)?;if !discovery.is_object()||discovery.get("aktiv").is_none(){return Ok(None);}
 // Unknown live extension jobs are rejected rather than silently discarded.
 for key in ["erforscht","aufklaerung","kapseln","fracht","wissen","bauplaene","fremd_planet","fremd_flotte"]{
  if discovery[key].as_object().is_some_and(|m|!m.is_empty()){return Err(format!("0.3.4-Spielstand enthält aktive Altdaten ({key}); automatische Übernahme würde sie verändern. Original bleibt erhalten."));}
 }
 let workforce=if base.starts_with(b"STERNE11"){let(next,v)=split(base)?;base=next;v}else{Value::Null};
 if !base.starts_with(b"STERNE10"){return Err("Unbekanntes Flottenlayout im Android-0.3.4-Spielstand".into());}
 let(inner,doctrine)=split(base)?;
 let mut w=crate::snapshot_layout::with_fleets(4,||Welt::aus_bytes(inner))?;
 std::sync::Arc::make_mut(&mut w.regeln).flottendoktrin=serde_json::from_value(doctrine).map_err(|e|format!("Alte Flottenprofile: {e}"))?;
 // Enum discriminant 35 was the rescue ship in that branch.
 let r=std::sync::Arc::make_mut(&mut w.regeln);
 if let Some(rule)=r.einheiten.remove(&Einheit::Handelsfrachter){r.einheiten.insert(Einheit::Bergungsschiff,rule);}
 for p in &mut w.planeten {for queue in &mut p.fertigung {for a in queue {if a.produkt==crate::welt::Produkt::Einheit(Einheit::Handelsfrachter){a.produkt=crate::welt::Produkt::Einheit(Einheit::Bergungsschiff);}}}}
 w.ausbau_aktivieren()?;
 if let Some(states)=workforce.as_object(){for(key,state)in states{let id:usize=key.parse().map_err(|_|"Ungültiger alter Personalplanet")?;let p=w.planeten.get(id).ok_or("Alter Personalplanet fehlt")?;let total=p.bevoelkerung;let f=state["fachkraefte"].as_i64().unwrap_or(0).max(0).min(total);let research=state["forscher"].as_i64().unwrap_or(0).max(0).min(total-f);let volk=w.spieler[p.besitzer as usize].volk;let mut people=crate::ausbau::Personen::default();people.gruppen[volk.idx()]=[total-f-research,f,research];w.ausbau.personen.insert(id as u32,people);}}
 if let Some(p)=discovery["pausen"].as_array(){for key in p{if let Some(k)=key.as_str(){w.ausbau.pausen.insert(k.into());}}}
 for p in 0..w.planeten.len(){w.raten_neu(p);}w.personen_validieren()?;Ok(Some(w))
}
