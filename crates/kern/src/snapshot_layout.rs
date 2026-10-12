//! Legacy bincode arrays had 28 buildings and 17 technologies. Keep their exact
//! bytes and hashes; STERNEP6 explicitly selects the extended layout.
use serde::{
    de::{Error, SeqAccess, Visitor},
    ser::SerializeTuple,
    Deserialize, Deserializer, Serializer,
};
use std::{cell::Cell, fmt};
thread_local! { static LEGACY: Cell<bool> = const { Cell::new(true) }; }

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
pub(crate) mod research {
    use super::*;
    pub fn serialize<S: Serializer>(a: &[u8; crate::FORSCHUNGEN], s: S) -> Result<S::Ok, S::Error> {
        super::serialize(a, s, 17)
    }
    pub fn deserialize<'de, D: Deserializer<'de>>(
        d: D,
    ) -> Result<[u8; crate::FORSCHUNGEN], D::Error> {
        super::deserialize(d, 17)
    }
}
