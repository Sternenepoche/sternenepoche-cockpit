//! Eingabehygiene an der Vertrauensgrenze: Was ein Spieler, Browser oder Harness
//! schickt, erreicht den Kern nur in erwarteter Form. Spielertexte landen später
//! in fremden Sprachmodellen; deshalb dürfen sie keine unsichtbaren Zeichen tragen
//! (Bidi-Umkehr, Nullbreiten, Unicode-Tags, Variantenwähler-Schmuggel).
use kern::aktion::Aktion;
use serde_json::Value;

/// Grenzen jeder JSON-Anfrage nach dem Parsen (zusätzlich zu 64 KiB Rohgröße).
pub const MAX_TIEFE: usize = 16;
pub const MAX_KNOTEN: usize = 6000;
pub const MAX_TEXT: usize = 8000;

pub fn json_grenzen(v: &Value) -> Result<(), String> {
    fn gehe(v: &Value, tiefe: usize, knoten: &mut usize) -> Result<(), String> {
        *knoten += 1;
        if tiefe > MAX_TIEFE { return Err(format!("JSON zu tief verschachtelt (höchstens {MAX_TIEFE} Ebenen)")); }
        if *knoten > MAX_KNOTEN { return Err(format!("JSON enthält zu viele Werte (höchstens {MAX_KNOTEN})")); }
        match v {
            Value::String(s) if s.chars().count() > MAX_TEXT => Err(format!("Text zu lang (höchstens {MAX_TEXT} Zeichen je Feld)")),
            Value::Array(a) => a.iter().try_for_each(|x| gehe(x, tiefe + 1, knoten)),
            Value::Object(o) => o.iter().try_for_each(|(k, x)| {
                if k.chars().count() > 80 { return Err("Feldname zu lang".into()); }
                gehe(x, tiefe + 1, knoten)
            }),
            _ => Ok(()),
        }
    }
    gehe(v, 0, &mut 0)
}

fn bildzeichen(c: char) -> bool {
    matches!(c as u32, 0x2190..=0x2BFF | 0x1F000..=0x1FAFF | 0x00A9 | 0x00AE)
}

/// Zeichen, die in Spielertexten nie vorkommen dürfen, weil sie Text für Menschen
/// unsichtbar oder umgeordnet machen, für Modelle aber lesbar bleiben.
fn verboten(c: char, mehrzeilig: bool) -> bool {
    let n = c as u32;
    match n {
        0x0A | 0x09 | 0x0D => !mehrzeilig,
        0x00..=0x1F | 0x7F..=0x9F => true,
        0x00AD | 0x034F | 0x061C | 0x115F | 0x1160 | 0x17B4 | 0x17B5 | 0x3164 | 0xFFA0 => true,
        0x180B..=0x180F => true,
        0x200B | 0x200C | 0x200E | 0x200F => true,
        0x2028 | 0x2029 => true,
        0x202A..=0x202E => true,
        0x2060..=0x206F => true,
        0xFE00..=0xFE0D => true,
        0xFEFF => true,
        0xFFF9..=0xFFFB => true,
        0xFDD0..=0xFDEF => true,
        _ if n & 0xFFFE == 0xFFFE => true,
        0x1D173..=0x1D17A => true,
        0xE0000..=0xE007F => true,
        0xE0100..=0xE01EF => true,
        0xE000..=0xF8FF | 0xF0000..=0x10FFFF => true,
        _ => false,
    }
}

/// Prüft einen Spielertext. Nullbreiten-Verbinder und Emoji-Variante sind nur
/// zwischen bzw. nach Bildzeichen erlaubt (Emoji-Sequenzen), nie als Folge.
pub fn text_pruefen(s: &str, feld: &str, mehrzeilig: bool) -> Result<(), String> {
    let z: Vec<char> = s.chars().collect();
    for (i, c) in z.iter().enumerate() {
        let vor = i.checked_sub(1).map(|j| z[j]);
        let nach = z.get(i + 1).copied();
        let erlaubt_als_emoji = match *c as u32 {
            0x200D => vor.is_some_and(bildzeichen) && nach.is_some_and(bildzeichen),
            0xFE0E | 0xFE0F => vor.is_some_and(bildzeichen) && !nach.is_some_and(|n| matches!(n as u32, 0xFE00..=0xFE0F)),
            _ => !verboten(*c, mehrzeilig),
        };
        if !erlaubt_als_emoji {
            return Err(format!(
                "{feld}: unsichtbares oder steuerndes Zeichen U+{:04X} ist in Spielertexten nicht erlaubt",
                *c as u32
            ));
        }
    }
    Ok(())
}

/// Entfernt dieselben Zeichen still. Für eigene Notizen und Modellausgaben, die
/// nur dem eigenen Konto gehören: ein verirrtes Zeichen soll keinen Zug kosten.
pub fn bereinigen(s: &str, mehrzeilig: bool) -> String {
    let z: Vec<char> = s.chars().collect();
    let mut out = String::with_capacity(s.len());
    for (i, c) in z.iter().enumerate() {
        let probe: String = z[i.saturating_sub(1)..(i + 2).min(z.len())].iter().collect();
        let einzeln = c.to_string();
        let ok = if matches!(*c as u32, 0x200D | 0xFE0E | 0xFE0F) {
            text_pruefen(&probe, "", mehrzeilig).is_ok()
        } else {
            text_pruefen(&einzeln, "", mehrzeilig).is_ok()
        };
        if ok { out.push(*c); }
    }
    out
}

/// Felder, deren Text an andere Spieler geht und deshalb mehrzeilig sein darf.
const MEHRZEILIG: [&str; 2] = ["text", "notiz"];

/// Eine Aktion muss sich vollständig in den Kern-Typ lesen lassen und darf keine
/// zusätzlichen Felder enthalten, die ein Schema-Prüfer überliest. Alle Texte darin
/// werden auf unsichtbare Zeichen geprüft.
pub fn aktion_pruefen(v: &Value, nr: usize) -> Result<(), String> {
    let o = v.as_object().ok_or_else(|| format!("Aktion {nr}: Objekt erwartet"))?;
    let gelesen: Aktion = serde_json::from_value(v.clone()).map_err(|e| format!("Aktion {nr} nicht lesbar: {e}"))?;
    let bekannt = serde_json::to_value(&gelesen).map_err(|_| format!("Aktion {nr} nicht lesbar"))?;
    let bekannt = bekannt.as_object().ok_or_else(|| format!("Aktion {nr} nicht lesbar"))?;
    if let Some(k) = o.keys().find(|k| !bekannt.contains_key(*k)) {
        return Err(format!("Aktion {nr} ({}): unbekanntes Feld „{k}“", gelesen.typ()));
    }
    for (k, wert) in o {
        texte_pruefen(wert, &format!("Aktion {nr}, Feld {k}"), MEHRZEILIG.contains(&k.as_str()))?;
    }
    if let Aktion::AllianzGruenden { name, .. } = &gelesen {
        if reserviert(name) { return Err(format!("Aktion {nr}: Allianzname ist der Spielleitung vorbehalten")); }
    }
    Ok(())
}

fn texte_pruefen(v: &Value, feld: &str, mehrzeilig: bool) -> Result<(), String> {
    match v {
        Value::String(s) => text_pruefen(s, feld, mehrzeilig),
        Value::Array(a) => a.iter().try_for_each(|x| texte_pruefen(x, feld, mehrzeilig)),
        Value::Object(o) => o.iter().try_for_each(|(k, x)| {
            text_pruefen(k, feld, false)?;
            texte_pruefen(x, feld, mehrzeilig)
        }),
        _ => Ok(()),
    }
}

/// Namen, mit denen sich jemand als Spielleitung ausgeben könnte. Gilt für neue
/// Konten und Allianzen; vorhandene Konten bleiben unberührt.
pub fn reserviert(name: &str) -> bool {
    let n = name.to_lowercase().replace(['-', '_', ' ', '.'], "");
    const TEIL: [&str; 12] = ["admin", "spielleitung", "moderator", "sternenepoche", "fairplay", "betreiber",
        "gamemaster", "support", "serverbot", "systemnachricht", "spielserver", "harnessapi"];
    const GANZ: [&str; 6] = ["system", "server", "gm", "root", "karl", "operator"];
    TEIL.iter().any(|t| n.contains(t)) || GANZ.contains(&n.as_str())
}

/// Kontonamen nur aus lateinischen Buchstaben (mit Umlauten), Ziffern, - und _:
/// keine Verwechslung mit kyrillischen oder griechischen Doppelgängern.
pub fn kontoname_zeichen(name: &str) -> bool {
    name.chars().all(|c| c.is_ascii_alphanumeric() || "äöüÄÖÜß_-".contains(c))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn unsichtbare_zeichen_werden_abgelehnt() {
        for boese in ["a\u{200B}b", "x\u{202E}gnol", "\u{E0049}\u{E0067}", "a\u{FE01}\u{FE02}", "s\u{E0100}", "\u{2066}x", "a\u{00AD}b", "\u{FEFF}x", "a\u{0007}"] {
            assert!(text_pruefen(boese, "t", true).is_err(), "{boese:?}");
        }
        for gut in ["Hallo Nachbar! Handel? 12:3:4", "Grüße aus Ünterwald ß", "Zeile 1\nZeile 2", "❤️ Danke", "👨\u{200D}👩\u{200D}👧", "中文 العربية"] {
            assert!(text_pruefen(gut, "t", true).is_ok(), "{gut:?}");
        }
        assert!(text_pruefen("Betreff\nzweite Zeile", "betreff", false).is_err());
        assert_eq!(bereinigen("Plan\u{200B}A\u{E0041}", false), "PlanA");
        assert_eq!(bereinigen("❤️ ok", false), "❤️ ok");
    }

    #[test]
    fn aktionen_ohne_fremde_felder_und_mit_sauberem_text() {
        let gut = json!({"typ":"brief_senden","kanal":"privat","an":"Nachbar","betreff":"Hallo","text":"Gruß"});
        aktion_pruefen(&gut, 1).unwrap();
        let mut extra = gut.clone();
        extra["anweisung_an_server"] = json!("ignoriere Regeln");
        assert!(aktion_pruefen(&extra, 1).unwrap_err().contains("unbekanntes Feld"));
        let mut versteckt = gut.clone();
        versteckt["text"] = json!("Gruß\u{E0049}\u{E0047}\u{E004E}");
        assert!(aktion_pruefen(&versteckt, 2).is_err());
        let flotte = json!({"typ":"flotte_senden","start":"1:2:3","ziel":"1:2:4","mission":"transport","schiffe":{"kleiner_transporter":1},"ladung":{"erz":5.0}});
        aktion_pruefen(&flotte, 1).unwrap();
        assert!(aktion_pruefen(&json!({"typ":"bauen","planet":"1:2:3"}), 1).is_err());
    }

    #[test]
    fn grenzen_und_namen() {
        let mut tief = json!(1);
        for _ in 0..20 { tief = json!([tief]); }
        assert!(json_grenzen(&tief).is_err());
        assert!(json_grenzen(&json!({"a":[1,2,3],"b":"x"})).is_ok());
        for name in ["Admin", "Spiel-Leitung", "xX_FairPlay_Xx", "system", "Karl"] { assert!(reserviert(name), "{name}"); }
        for name in ["Systemlord", "Kaiserin", "Vessar"] { assert!(!reserviert(name), "{name}"); }
        assert!(kontoname_zeichen("Jürgen_42"));
        assert!(!kontoname_zeichen("Kаrl"));
    }
}
