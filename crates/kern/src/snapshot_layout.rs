//! Legacy bincode arrays had 28 buildings and 17 technologies. Keep their exact
//! bytes and hashes; STERNEP6 explicitly selects the extended layout.
use serde::{
    de::{Error, SeqAccess, Visitor},
    ser::SerializeTuple,
    Deserialize, Deserializer, Serializer,
};
use std::{cell::Cell, fmt};
thread_local! { static LEGACY: Cell<bool> = const { Cell::new(true) }; }
// 0: twelve original ships; 1: V10 (27 ships); 2: V11 (28 ships); 3: Generation 12 (30 ships,
// 38 units); 4: Android 0.3.4 branch; 5: Mondgeneration STERNE13 (34 ships, 43 units); 6: Terraforming-Generation
// STERNE14 (35 ships, 44 units: Terraformer an Position 34, nur Welten mit Terraformer im Regelwerk).
thread_local! { static FLEETS: Cell<u8> = const { Cell::new(0) }; }
// Human-readable arrays keep the Generation-12 shape inside explicit legacy contexts (hashes,
// legacy envelopes), the STERNE13 shape (34/43) inside layout 5; everywhere else they carry the full current roster.
thread_local! { static EXPLICIT: Cell<bool> = const { Cell::new(false) }; }
pub(crate) const AKTUELL: u8 = 6;
/// Mondgeneration STERNE13: 34 Schiffe, 43 Einheiten (laufende Welten ohne Terraformer).
pub(crate) const MONDE: u8 = 5;
pub(crate) fn with_fleets<T>(version:u8,f:impl FnOnce()->T)->T {
    struct Restore(u8,bool);impl Drop for Restore {fn drop(&mut self){FLEETS.with(|v|v.set(self.0));EXPLICIT.with(|v|v.set(self.1));}}
    let _r=Restore(FLEETS.with(|v|v.replace(version)),EXPLICIT.with(|v|v.replace(true)));f()
}
fn count(version:u8,ships:bool)->usize {match (version,ships){(4,true)=>28,(4,false)=>36,(0,true)=>12,(0,false)=>18,(1,true)=>27,(1,false)=>35,(2,true)=>28,(2,false)=>36,(3,true)=>30,(3,false)=>38,(5,true)=>34,(5,false)=>43,(_,true)=>crate::SCHIFFE,(_,false)=>crate::EINHEITEN}}
fn index(i:usize,version:u8,ships:bool)->usize {
    // Historical defence entries follow their ships; today they start at SCHIFFE.
    let alt_schiffe=match version {0=>12,1=>27,2|4=>28,3=>30,5=>34,_=>return i};
    if version==4 && i==27 {return 29;}
    if ships || i<alt_schiffe {i} else {i-alt_schiffe+crate::SCHIFFE}
}
fn lesbar_version()->u8 {if !EXPLICIT.with(Cell::get) {return AKTUELL;} match FLEETS.with(Cell::get) {v if v<MONDE=>3,MONDE=>MONDE,_=>AKTUELL}}
fn unit_array_serialize<S:Serializer,const N:usize>(a:&[i64;N],s:S,ships:bool)->Result<S::Ok,S::Error>{
    let version=if s.is_human_readable(){lesbar_version()}else{FLEETS.with(Cell::get)};
    let len=count(version,ships);let mut t=s.serialize_tuple(len)?;
    for i in 0..len {t.serialize_element(&a[index(i,version,ships)])?;}t.end()
}
fn unit_array_deserialize<'de,D:Deserializer<'de>,const N:usize>(d:D,ships:bool)->Result<[i64;N],D::Error>{
    struct V<const N:usize>{len:usize,ships:bool,version:u8}
    impl<'de,const N:usize> Visitor<'de> for V<N>{type Value=[i64;N];fn expecting(&self,f:&mut fmt::Formatter)->fmt::Result{write!(f,"{} Einheiten",self.len)}
      fn visit_seq<A:SeqAccess<'de>>(self,mut seq:A)->Result<Self::Value,A::Error>{let mut a=[0;N];for i in 0..self.len {a[index(i,self.version,self.ships)]=seq.next_element()?.ok_or_else(||A::Error::invalid_length(i,&self))?;}Ok(a)}}
    if d.is_human_readable(){let v=Vec::<i64>::deserialize(d)?;let mut a=[0;N];let version=[0u8,1,2,3,MONDE,AKTUELL].into_iter().find(|version|v.len()==count(*version,ships)).ok_or_else(||D::Error::custom("Ungültige Einheitenliste"))?;for(i,n)in v.iter().enumerate(){a[index(i,version,ships)]=*n;}return Ok(a);}
    let version=FLEETS.with(Cell::get);let len=count(version,ships);d.deserialize_tuple(len,V::<N>{len,ships,version})
}
pub(crate) mod units {
 use super::*;pub fn serialize<S:Serializer>(a:&[i64;crate::EINHEITEN],s:S)->Result<S::Ok,S::Error>{unit_array_serialize(a,s,false)}
 pub fn deserialize<'de,D:Deserializer<'de>>(d:D)->Result<[i64;crate::EINHEITEN],D::Error>{unit_array_deserialize(d,false)}
}
pub(crate) mod ships {
 use super::*;pub fn serialize<S:Serializer>(a:&[i64;crate::SCHIFFE],s:S)->Result<S::Ok,S::Error>{unit_array_serialize(a,s,true)}
 pub fn deserialize<'de,D:Deserializer<'de>>(d:D)->Result<[i64;crate::SCHIFFE],D::Error>{unit_array_deserialize(d,true)}
}
pub(crate) mod optional_ships {
 use super::*;#[derive(serde::Serialize,Deserialize)]struct Ships(#[serde(with="super::ships")] [i64;crate::SCHIFFE]);
 pub fn serialize<S:Serializer>(a:&Option<[i64;crate::SCHIFFE]>,s:S)->Result<S::Ok,S::Error>{serde::Serialize::serialize(&a.map(Ships),s)}
 pub fn deserialize<'de,D:Deserializer<'de>>(d:D)->Result<Option<[i64;crate::SCHIFFE]>,D::Error>{Ok(Option::<Ships>::deserialize(d)?.map(|s|s.0))}
}

pub(crate) fn with_layout<T>(legacy: bool, f: impl FnOnce() -> T) -> T {
    struct Restore(bool);
    impl Drop for Restore {
        fn drop(&mut self) {
            LEGACY.with(|v| v.set(self.0));
        }
    }
    let _restore = Restore(LEGACY.with(|v| v.replace(legacy)));
    f()
}

fn serialize<S: Serializer, const N: usize>(
    a: &[u8; N],
    s: S,
    old: usize,
) -> Result<S::Ok, S::Error> {
    let len = if !s.is_human_readable() && LEGACY.with(Cell::get) {
        old
    } else {
        N
    };
    let mut tuple = s.serialize_tuple(len)?;
    for v in a.iter().take(len) {
        tuple.serialize_element(v)?;
    }
    tuple.end()
}
fn deserialize<'de, D: Deserializer<'de>, const N: usize>(
    d: D,
    old: usize,
) -> Result<[u8; N], D::Error> {
    if d.is_human_readable() {
        let values = Vec::<u8>::deserialize(d)?;
        if values.len() != N && values.len() != old {
            return Err(D::Error::custom("Ungültige Anzahl von Ausbaustufen"));
        }
        let mut result = [0; N];
        result[..values.len()].copy_from_slice(&values);
        return Ok(result);
    }
    struct ArrayVisitor<const N: usize>(usize);
    impl<'de, const N: usize> Visitor<'de> for ArrayVisitor<N> {
        type Value = [u8; N];
        fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
            write!(f, "{} Ausbaustufen", self.0)
        }
        fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
            let mut result = [0; N];
            for (i, value) in result.iter_mut().enumerate().take(self.0) {
                *value = seq
                    .next_element()?
                    .ok_or_else(|| A::Error::invalid_length(i, &self))?;
            }
            Ok(result)
        }
    }
    let len = if LEGACY.with(Cell::get) { old } else { N };
    d.deserialize_tuple(len, ArrayVisitor::<N>(len))
}
pub(crate) mod buildings {
    use super::*;
    pub fn serialize<S: Serializer>(a: &[u8; crate::GEBAEUDE], s: S) -> Result<S::Ok, S::Error> {
        super::serialize(a, s, 28)
    }
    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<[u8; crate::GEBAEUDE], D::Error> {
        super::deserialize(d, 28)
    }
}
// Forschungsfeld im Binärteil: 17 Einträge im Altlayout, 19 ab STERNEP6 (Überwachungs-/Abschirmtechnik), 20 nur in
// Welten mit Technik v1 (Plasmatechnik; `ausbau.technik` in der JSON-Erweiterung bestimmt die Länge beim Laden).
pub(crate) const FORSCHUNGEN_V6: usize = 19;
thread_local! { static FORSCHUNG: Cell<usize> = const { Cell::new(FORSCHUNGEN_V6) }; }
pub(crate) fn with_forschungen<T>(n: usize, f: impl FnOnce() -> T) -> T {
    struct Restore(usize);
    impl Drop for Restore {
        fn drop(&mut self) {
            FORSCHUNG.with(|v| v.set(self.0));
        }
    }
    let _restore = Restore(FORSCHUNG.with(|v| v.replace(n.clamp(17, crate::FORSCHUNGEN))));
    f()
}
pub(crate) mod research {
    use super::*;
    pub fn serialize<S: Serializer>(a: &[u8; crate::FORSCHUNGEN], s: S) -> Result<S::Ok, S::Error> {
        let len = if s.is_human_readable() {
            crate::FORSCHUNGEN
        } else if LEGACY.with(Cell::get) {
            17
        } else {
            FORSCHUNG.with(Cell::get)
        };
        let mut tuple = s.serialize_tuple(len)?;
        for v in a.iter().take(len) {
            tuple.serialize_element(v)?;
        }
        tuple.end()
    }
    pub fn deserialize<'de, D: Deserializer<'de>>(
        d: D,
    ) -> Result<[u8; crate::FORSCHUNGEN], D::Error> {
        if d.is_human_readable() {
            let values = Vec::<u8>::deserialize(d)?;
            if ![17, FORSCHUNGEN_V6, crate::FORSCHUNGEN].contains(&values.len()) {
                return Err(D::Error::custom("Ungültige Anzahl von Forschungsstufen"));
            }
            let mut result = [0; crate::FORSCHUNGEN];
            result[..values.len()].copy_from_slice(&values);
            return Ok(result);
        }
        struct ArrayVisitor(usize);
        impl<'de> Visitor<'de> for ArrayVisitor {
            type Value = [u8; crate::FORSCHUNGEN];
            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                write!(f, "{} Forschungsstufen", self.0)
            }
            fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
                let mut result = [0; crate::FORSCHUNGEN];
                for (i, value) in result.iter_mut().enumerate().take(self.0) {
                    *value = seq.next_element()?.ok_or_else(|| A::Error::invalid_length(i, &self))?;
                }
                Ok(result)
            }
        }
        let len = if LEGACY.with(Cell::get) { 17 } else { FORSCHUNG.with(Cell::get) };
        d.deserialize_tuple(len, ArrayVisitor(len))
    }
}
