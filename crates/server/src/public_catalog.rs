//! Public rules only: no world state, accounts, coordinates or observations.
use kern::Regelwerk;
use serde_json::{json, Value};
pub fn catalog(r: &Regelwerk) -> Value {
    json!({"version":r.version,"agenten":{"nachricht_zeichen":r.agenten.nachricht_zeichen,"nachrichten_je_tag":r.agenten.nachrichten_je_tag},"voelker":r.voelker,"wirtschaft":{
        "energie_je_1000_syntheten":r.wirtschaft.energie_je_1000_syntheten,
        "unruhen_unter":r.stabilitaet.unruhen_unter,
        "forschung_warteschlange":r.wirtschaft.forschung_warteschlange,
        "min_bauzeit_sekunden":r.wirtschaft.min_bauzeit_sekunden},
        "gebaeude":r.gebaeude,"forschung":r.forschung,
        "einheiten":r.einheiten.iter().map(|(e,rule)| (e.name(),json!({"regel":rule,"schiff":e.ist_schiff(),
            "kosten_je_volk":kern::Volk::ALLE.into_iter().map(|v| {
                let cost=r.kosten_einheit(*e,v);
                (v.name(),kern::Gut::ALLE.into_iter().filter(|g|cost[g.idx()]!=0).map(|g|(g.name(),cost[g.idx()] as f64/kern::M as f64)).collect::<std::collections::BTreeMap<_,_>>())
            }).collect::<std::collections::BTreeMap<_,_>>()
        }))).collect::<std::collections::BTreeMap<_,_>>(),
        "bauteile":r.wirtschaft.bauteile,"kampf":r.kampf,
        "zonen":r.zonen,"welt":{"sektoren":r.welt.sektoren,"systeme_je_sektor":r.welt.systeme_je_sektor,"plaetze_je_system":r.welt.plaetze_je_system,"max_gebaeudestufe":r.welt.max_gebaeudestufe},
        "kolonisation":kern::kolonisation::REGELTEXT_V2,"ausscheiden":kern::ausscheiden::REGELTEXT})
}
#[cfg(test)] mod tests {
    use super::*;
    #[test] fn public_catalog_uses_active_profile() {
        let mut r=Regelwerk::laden(include_str!("../../../regeln/online-v1.ron")).unwrap();
        r.voelker.get_mut(&kern::Volk::Krath).unwrap().waffen=1.27;
        let c=catalog(&r);
        assert_eq!(c["voelker"]["krath"]["waffen"],1.27);
        assert_eq!(c["einheiten"][kern::Einheit::Spionagesonde.name()]["schiff"],true);
        assert!(c.get("spieler").is_none());
    }
}
