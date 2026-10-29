//! Public rules only: no world state, accounts, coordinates or observations.
use kern::Regelwerk;
use serde_json::{json, Value};
pub fn catalog(r: &Regelwerk) -> Value {
    let mut buildings=serde_json::to_value(&r.gebaeude).unwrap();
    for g in r.gebaeude.keys(){buildings[g.name()]["wirkung"]=json!(kern::regeltext::gebaeude_beschreibung(r,*g));}
    let stages = r.gebaeude.iter().map(|(g, rule)| {
        let max = if matches!(g,kern::Gebaeude::Orbitalring|kern::Gebaeude::Forschungsarchiv|kern::Gebaeude::Versorgungsnetz) {1} else {r.welt.max_gebaeudestufe};
        let rows = (0..=max).map(|level| {
            let cost = if level == 0 {[0;kern::GUETER]} else {r.kosten_gebaeude(*g,level)};
            let value = |base| r.stufenwert(base,level) as f64/kern::M as f64;
            let output = match g {
                kern::Gebaeude::Akademie => value(r.wirtschaft.fachkraefte_je_akademie),
                kern::Gebaeude::Lager => r.lagergrenze(level)[kern::Gut::Erz.idx()] as f64/kern::M as f64,
                kern::Gebaeude::Bunker => r.bunkerschutz(level)[kern::Gut::Erz.idx()] as f64/kern::M as f64,
                _ => value(rule.ertrag),
            };
            json!({"stufe":level,"kosten":kern::Gut::ALLE.into_iter().filter(|g|cost[g.idx()]!=0).map(|g|(g.name(),kern::ganz(cost[g.idx()]))).collect::<std::collections::BTreeMap<_,_>>(),
                "leistung":output,"energie":value(rule.energie),"arbeiter":value(rule.arbeiter),"fachkraefte":value(rule.fachkraefte)})
        }).collect::<Vec<_>>();
        (g.name(),rows)
    }).collect::<std::collections::BTreeMap<_,_>>();
    // Expedition und Rettung (kern::expedition): Ereignisse je Pathfinder-Schwelle, Risiko, Fristen.
    let ereignisse:Vec<u8>=[0i64,1,2,4,8,16,32].iter().map(|p|kern::expedition::ereignislimit(*p)).collect();
    let risiko:Vec<f64>=[0i64,1,2,3,4,8,16,32].iter().map(|p|kern::expedition::risiko_ppm(*p) as f64/1000.0).collect();
    let rettung:Vec<i64>=[0i64,1,2,3].iter().map(|p|kern::expedition::rettung_limit(1,*p)).collect();
    let expedition=json!({"pathfinder_stufen":[1,2,4,8,16,32],"ereignisse":ereignisse,"risiko_promille":risiko,
        "rettung_je_bergungsschiff":rettung,"bordsonden_je_pathfinder":20,"rettungsfrist_stunden":24});
    json!({"gueter":kern::Gut::ALLE.map(|g|g.name()),"regelabschnitte":kern::regeltext::abschnitte(r).iter().map(|a|json!({"id":a.stichwort,"name":a.titel,"text":a.text})).collect::<Vec<_>>(),"bodentruppen":kern::boden_katalog::katalog_bw(&r.kw().boden),"version":r.version,"agenten":{"nachricht_zeichen":r.agenten.nachricht_zeichen,"nachrichten_je_tag":r.agenten.nachrichten_je_tag},"voelker":r.voelker,"wirtschaft":{
        "energie_je_1000_syntheten":r.wirtschaft.energie_je_1000_syntheten,
        "ertragswachstum":r.wirtschaft.ertragswachstum,"unterhalt_schiffe_je_tag":r.wirtschaft.unterhalt_schiffe_je_tag,
        "rezepte":r.wirtschaft.rezepte,"fusion_deuterium":r.wirtschaft.fusion_deuterium,
        "lager_roh":r.wirtschaft.lager_roh,"lager_verarbeitet":r.wirtschaft.lager_verarbeitet,"lager_selten":r.wirtschaft.lager_selten,"lager_faktor":r.wirtschaft.lager_faktor,
        "bunker_je_stufe":r.wirtschaft.bunker_je_stufe,"bunker_verarbeitet_anteil":r.wirtschaft.bunker_verarbeitet_anteil,"fachkraefte_je_akademie":r.wirtschaft.fachkraefte_je_akademie,
        "fertigung_warteschlange":r.wirtschaft.warteschlange,"fertigung_min_einwohner":500,
        "unruhen_unter":r.stabilitaet.unruhen_unter,
        "forschung_warteschlange":r.wirtschaft.forschung_warteschlange,
        "min_bauzeit_sekunden":r.wirtschaft.min_bauzeit_sekunden},
        "gebaeude":buildings,"ausbaustufen":stages,"forschung":r.forschung,"stufen":r.stufen_daten(),
        "stufen_haltezeit_stunden":r.stufen_haltezeit_stunden,"zusatz":r.zusatz,"raketen":kern::raketen::regeln(r),
        "forschung_voraussetzungen":forschung_voraussetzungen(r),
        "technik":r.technik.as_ref().map(|t|json!({"version":t.version,"antriebswechsel":t.antriebswechsel,"plasma_je_stufe":t.plasma_je_stufe,
            "plasmatechnik_braucht":t.plasmatechnik_braucht,"regeltext":kern::technik::regeltext(r,t)})),
        "einheiten":r.einheiten.iter().map(|(e,rule)| (e.name(),json!({"regel":rule,"schiff":e.ist_schiff(),"taktik":taktik(r,*e),"zielprofil":kern::zielwahl::profil(r,*e),
            "kosten_je_volk":kern::Volk::ALLE.into_iter().map(|v| {
                let cost=r.kosten_einheit(*e,v);
                (v.name(),kern::Gut::ALLE.into_iter().filter(|g|cost[g.idx()]!=0).map(|g|(g.name(),cost[g.idx()] as f64/kern::M as f64)).collect::<std::collections::BTreeMap<_,_>>())
            }).collect::<std::collections::BTreeMap<_,_>>()
        }))).collect::<std::collections::BTreeMap<_,_>>(),
        "bauteile":r.wirtschaft.bauteile,"kampf":r.kampf,"flug":r.flug,
        // Wertungsgewichte für Flotten-/Abwehrwert im Einsatzblatt (gleiche Größe wie Kampfsimulator und Kern Regelwerk::wert).
        "wertung":{"gewichte":r.wertung.gewichte},
        // Beutevorschau im Client: dieselben Quoten und Bunkerwerte wie Kern flotte.rs pluendern / regeln.rs bunkerschutz.
        "pluenderung":{"quote_je_volk":kern::Volk::ALLE.iter().map(|v|(v.name(),r.volk(*v).pluenderquote.unwrap_or(r.kampf.pluenderquote))).collect::<std::collections::BTreeMap<_,_>>(),
            "bunker_je_stufe":r.wirtschaft.bunker_je_stufe,"bunker_verarbeitet_anteil":r.wirtschaft.bunker_verarbeitet_anteil},
        "zonen":r.zonen,"welt":{"sektoren":r.welt.sektoren,"systeme_je_sektor":r.welt.systeme_je_sektor,"plaetze_je_system":r.welt.plaetze_je_system,"max_gebaeudestufe":r.welt.max_gebaeudestufe,"max_forschungsstufe":kern::regeln::MAX_FORSCHUNG,
            // Start einer neuen Heimatwelt und Sonne je Bahnplatz – der Techbaum liest sie hier statt feste Texte (23.10.2026).
            "start_gebaeude":r.welt.start_gebaeude.iter().map(|(g,n)|(g.name(),*n)).collect::<std::collections::BTreeMap<_,_>>(),
            "start_bevoelkerung":r.welt.start_bevoelkerung,"grundwohnraum":r.welt.grundwohnraum},
        "sonne":{"solar_promille":r.kw().sonne.solar_promille,"dyson_schatten":r.kw().sonne.dyson_schatten},
        "expedition":expedition,
        "wachstum":{"stabilitaet_voll_ab":r.stabilitaet.wachstum_voll_ab,"stabilitaet_null_unter":r.stabilitaet.wachstum_null_unter},
        "kolonisation":kern::kolonisation::REGELTEXT_V2,"invasion":if r.kw().boden.wellen>0{kern::invasion2::regeltext_v2(&r.kw().boden)}else{kern::invasion::REGELTEXT_INVASION.to_string()},"invasion_werte":kern::Welt::invasion_katalog_bw(&r.kw().boden),"ausscheiden":kern::ausscheiden::REGELTEXT})
}
fn forschung_voraussetzungen(r:&Regelwerk)->Value {
    let mut v=json!({
        "ueberwachungstechnik":{"forschung":{"spionagetechnik":1},"gebaeude_reich":{"geheimdienst":1}},
        "abschirmtechnik":{"forschung":{"spionagetechnik":1}}});
    if let Some(t)=r.technik.as_ref(){v["plasmatechnik"]=json!({"forschung":t.plasmatechnik_braucht});}
    v
}
fn taktik(r:&Regelwerk,e:kern::Einheit)->Value {
    let Some(t)=r.flottendoktrin.get(&e) else{return Value::Null;};
    let p=kern::zielwahl::profil(r,e);let mut v=serde_json::to_value(t).unwrap();
    v["schadensfaktoren"]=json!(p.schadensfaktoren);v["ziele"]=json!(p.ziele_je_runde);
    if e==kern::Einheit::Spionagesonde {v["rolle"]=json!("Schnelle Aufklärung ohne Besatzung. Überlebende Sonden kehren nach dem Scan zurück und sind nach der Landung wieder verfügbar. Treibstoff für Hin- und Rückflug wird beim Start reserviert.");}
    // Alte Welten tragen den falschen Doktrin-v1-Text („nur … können Bomber erfassen“); neue Welten haben die korrekte Rolle aus
    // regeln/flottenausbau.json (Konter-Balance 24.10.2026) und behalten sie.
    if e==kern::Einheit::Bomber && v["rolle"].as_str().is_some_and(|s| s.contains("erfassen")) {v["rolle"]=json!("Spezialisierte Bodenbekämpfung. Bevorzugte Bomberabwehr erhält Zielboni; andere Gegner treffen mit Grundschaden.");}
    v
}
#[cfg(test)] mod tests {
    use super::*;
    #[test] fn codex_matches_current_ground_missile_and_colony_rules(){
        let r=Regelwerk::laden(include_str!("../../../regeln/online-v1.ron")).unwrap();let mut w=kern::Welt::neu(r,77,2).unwrap();w.ausbau_aktivieren().unwrap();let c=catalog(&w.regeln);
        assert_eq!(c["gueter"].as_array().unwrap().len(),kern::GUETER);
        let ground=c["bodentruppen"]["typen"].as_array().unwrap();assert_eq!(ground.len(),12);
        for(i,t)in ground.iter().enumerate(){assert_eq!(t["angriff"],kern::boden_kampf::schaden(i));for j in 0..kern::boden_katalog::KAMPFZAHL{assert_eq!(t["konter_promille"][j],kern::boden_katalog::faktor(i,j));}}
        for(i,r)in c["raketen"].as_array().unwrap().iter().enumerate(){assert_eq!(r["fertigungsplaetze"],kern::parallel_silo::plaetze(i as u8));}
        let sections=c["regelabschnitte"].as_array().unwrap();let text=sections.iter().map(|s|s["text"].as_str().unwrap()).collect::<Vec<_>>().join("\n");
        for t in ground{assert!(text.contains(t["name"].as_str().unwrap()));}
        for stale in ["Keine Beute, Trümmer", "Eine Produktionslinie arbeitet nacheinander", "jede zweite weitere Stufe eine mehr", "wirklich ausgebildet", "nur auf Planeten in Nebelsystemen", "höhere Stufen bauen schneller"]{assert!(!text.contains(stale),"{stale}");}
        for expected in ["ohne Garnison", "Fertigungsplätze", "Jede Astrophysikstufe", "dauerhaft verlorener Verteidigungsanlagen"]{assert!(text.to_lowercase().contains(&expected.to_lowercase()),"{expected}");}
        assert!(c["gebaeude"]["werft"]["wirkung"].as_str().unwrap().contains("Fertigungsplatz"));
        assert!(c["gebaeude"]["xenoextraktor"]["wirkung"].as_str().unwrap().contains("Synthese"));
        assert_eq!(c["einheiten"]["kolonieschiff"]["regel"]["kosten"],json!({"legierung":1000,"elektronik":500,"habitatmodul":1}));
    }
    #[test] fn handelsfrachter_catalog_and_four_embedded_images_match_active_rules(){
        let mut r=Regelwerk::laden(include_str!("../../../regeln/online-v1.ron")).unwrap();
        assert!(catalog(&r)["einheiten"].get("handelsfrachter").is_none());
        r.flottendoktrin_aktivieren();let c=catalog(&r);let ship=&c["einheiten"]["handelsfrachter"];
        assert_eq!(ship["schiff"],true);assert_eq!(ship["regel"]["ladung"],50000);assert_eq!(ship["regel"]["besatzung"],25);assert_eq!(ship["taktik"]["personenplaetze"],20);
        assert_eq!(c["einheiten"]["grosser_transporter"]["taktik"]["personenplaetze"],10);
        for volk in kern::Volk::ALLE {
            let path=format!("/assets/ships.handelsfrachter.{}.webp",volk.name());
            let(bytes,mime)=crate::web_assets::get(&path).expect("embedded faction artwork");
            assert_eq!(mime,"image/webp");assert!(bytes.starts_with(b"RIFF"));assert_eq!(&bytes[8..12],b"WEBP");
        }
    }
    #[test] fn all_new_ships_and_missiles_have_four_embedded_motives(){
        let r=Regelwerk::laden(include_str!("../../../regeln/online-v1.ron")).unwrap();let mut w=kern::Welt::neu(r,77,2).unwrap();w.ausbau_aktivieren().unwrap();let c=catalog(&w.regeln);
        assert!(c["einheiten"].get("pathfinder").is_some());assert!(c["einheiten"].get("bergungsschiff").is_some());assert_eq!(c["raketen"].as_array().unwrap().len(),5);
        for v in kern::Volk::ALLE{for (cat,keys) in [("ships",vec!["pathfinder","bergungsschiff","handelsfrachter","terraformer"]),("missiles",kern::raketen::ARTEN.to_vec())]{for key in keys{let path=format!("/assets/{cat}.{key}.{}.webp",v.name());let (data,mime)=crate::web_assets::get(&path).expect("Faction motive must be embedded");assert_eq!(mime,"image/webp");assert!(data.len()>1000);assert_eq!(&data[..4],b"RIFF");}}}
    }
    #[test] fn public_catalog_uses_active_profile() {
        let mut r=Regelwerk::laden(include_str!("../../../regeln/online-v1.ron")).unwrap();
        r.voelker.get_mut(&kern::Volk::Krath).unwrap().waffen=1.27;
        let c=catalog(&r);
        assert_eq!(c["voelker"]["krath"]["waffen"],1.27);
        assert_eq!(c["einheiten"][kern::Einheit::Spionagesonde.name()]["schiff"],true);
        assert!(c.get("spieler").is_none());
        assert_eq!(c["stufen"][3]["forschung"]["hyperraumantrieb"],1);
        assert_eq!(c["forschung_voraussetzungen"]["ueberwachungstechnik"]["forschung"]["spionagetechnik"],1);
    }
}
