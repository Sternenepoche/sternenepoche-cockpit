use super::*;

fn fixture()->Welt {
    let r=Regelwerk::laden(include_str!("../../../regeln/online-v1.ron")).unwrap();
    let mut w=Welt::neu(r,917,4).unwrap();w.ausbau_aktivieren().unwrap();
    for sid in 0..4 {w.spieler[sid].ki=false;w.spieler[sid].credits=100_000*M;let p=w.spieler[sid].heimat as usize;
        w.spieler[sid].stufe=3;w.spieler[sid].forschung.fill(3);w.planeten[p].gebaeude[Gebaeude::Markt.idx()]=2;
        w.planeten[p].gebaeude[Gebaeude::Raumhafen.idx()]=3;w.planeten[p].bestand.fill(20_000*M);w.raten_neu(p);
    }w
}
/// Regeln laufender Welten: Terraforming bringt +6 Felder je Stufe und braucht Labor 6 (neue Welten seit 0.4.7: keine
/// Felder aus der Forschung, Labor 3). Die Tests der engen Heimat prüfen das Verhalten dort, wo es noch gilt.
fn terraforming_mit_feldern(w:&mut Welt){
    let r=std::sync::Arc::make_mut(&mut w.regeln);r.wirtschaft.terraforming_felder=6;
    let f=r.forschung.get_mut(&Forschung::Terraforming).unwrap();f.labor=6;f.ab_stufe=4;
}
fn act(w:&mut Welt,sid:u16,a:Value){let(ok,msg)=w.handeln(sid,Rolle::Alle,&a);assert!(ok,"{msg}");}
fn alliance(w:&mut Welt){act(w,0,json!({"typ":"allianz_gruenden","name":"Testbund"}));let name=w.spieler[1].name.clone();act(w,0,json!({"typ":"allianz_einladen","spieler":name}));act(w,1,json!({"typ":"allianz_beitreten","allianz":"Testbund"}));}
fn letter(w:&mut Welt,text:&str,reply:Option<u64>)->u64 {let name=w.spieler[1].name.clone();act(w,0,json!({"typ":"brief_senden","kanal":"privat","an":name,"betreff":"Abstimmung","text":text,"antwort_auf":reply}));w.nachrichten.len() as u64}
fn answer(w:&mut Welt)->String {reagieren_mit(w,1,Bottyp::Haendler,&[1].into_iter().collect());w.nachrichten.iter().enumerate().rev().find(|(i,n)|n.von==1&&w.kommunikation.briefe.get(&(*i as u64+1)).is_some_and(|b|b.kanal=="privat")).unwrap().1.text.clone()}

#[test]
fn human_followups_execute_once_and_survive_save() {
    let mut w=fixture();alliance(&mut w);
    letter(&mut w,"Status",None);let s=answer(&mut w);assert!(s.contains("Gebäudestufen"));let reply=w.nachrichten.len() as u64;
    letter(&mut w,"Verkaufe 100 erz für 1",Some(reply));let s=answer(&mut w);assert!(s.contains("Ja."),"{s}");
    assert_eq!(w.kommunikation.angebote.len(),1);let offer=w.kommunikation.angebote[0].clone();assert_eq!(offer.menge,100*M);assert_eq!(offer.von,1);
    w=Welt::aus_bytes(&w.zu_bytes()).unwrap();
    let count=w.nachrichten.len();answer(&mut w);assert_eq!(w.nachrichten.len(),count);assert_eq!(w.kommunikation.angebote.len(),1);
    // A repeated request cannot reserve the same warehouse goods twice.
    letter(&mut w,"Verkaufe 100 erz für 1",None);assert!(answer(&mut w).contains("bereits ein offenes Angebot"));assert_eq!(w.kommunikation.angebote.len(),1);
    let p=w.spieler[0].heimat as usize;let k=w.planeten[p].koord;act(&mut w,0,json!({"typ":"intern_kaufen","angebot":offer.id,"planet":k}));
    let arrival=w.kommunikation.angebote[0].ankunft.unwrap();while w.zeit<arrival{w.schritt();}
    assert_eq!(w.kommunikation.angebote[0].status,"geliefert");assert!(w.handel.iter().any(|h|h.kaeufer==0&&h.verkaeufer==1&&h.menge==100*M));
}

#[test]
fn purchase_letter_pays_real_credits_and_rejects_unaffordable_price() {
    let mut w=fixture();alliance(&mut w);let p=w.spieler[0].heimat as usize;let k=w.planeten[p].koord;
    act(&mut w,0,json!({"typ":"intern_anbieten","planet":k,"gut":"erz","menge":100,"preis":1}));
    let before=w.spieler[1].credits;letter(&mut w,"Kaufe Angebot 1",None);let s=answer(&mut w);assert!(s.contains("Ja."),"{s}");assert!(w.spieler[1].credits<before);assert_eq!(w.kommunikation.angebote[0].status,"unterwegs");
    act(&mut w,0,json!({"typ":"intern_anbieten","planet":k,"gut":"erz","menge":100,"preis":100}));
    let before=w.spieler[1].credits;letter(&mut w,"Kaufe Angebot 2",None);assert!(answer(&mut w).contains("Nein."));assert_eq!(w.spieler[1].credits,before);assert_eq!(w.kommunikation.angebote[1].status,"offen");
    letter(&mut w,"Kaufe Angebot 99999",None);assert!(answer(&mut w).contains("Nein."));
}

#[test]
fn treaty_and_help_replies_match_actual_state() {
    let mut w=fixture();letter(&mut w,"Nichtangriffspakt",None);let s=answer(&mut w);assert!(s.contains("Ja."),"{s}");assert!(w.vertraege.iter().any(|v|v.a==1&&v.b==0));
    alliance(&mut w);let p=w.spieler[0].heimat as usize;let target=w.planeten[p].koord;
    act(&mut w,0,json!({"typ":"allianz_hilfe","planet":target,"text":"Entsatz benötigt"}));
    letter(&mut w,"Hilfe",None);let s=answer(&mut w);assert!(s.contains("Nein."),"{s}");assert!(w.kommunikation.hilfe[0].helfer.is_empty());
    let hp=w.spieler[1].heimat as usize;w.planeten[hp].einheiten[Einheit::LeichterJaeger.idx()]=10;
    w.planeten[hp].einheiten[Einheit::Spionagesonde.idx()]=3;
    for _ in 0..3 {dialog::hilferuf(&mut w,1,1,Bottyp::Haendler);if w.planetenwissen(1,target)["bekannt"]==true{break;}let done=w.flotten.values().filter(|f|f.besitzer==1).map(|f|f.ankunft).max().unwrap();while w.zeit<=done{w.schritt();}}
    letter(&mut w,"Hilfe",None);let s=answer(&mut w);assert!(s.contains("Entsatzflotte"),"{s}");assert!(w.kommunikation.hilfe[0].helfer.contains(&1));
    let f=w.flotten.values().find(|f|f.besitzer==1&&f.mission==Mission::Halten).unwrap();assert_eq!(f.schiffe[Einheit::LeichterJaeger.idx()],5);assert_eq!(f.ziel,target);assert!(s.contains(&f.ankunft.to_string()));
    assert_eq!(w.planeten[hp].einheiten[Einheit::LeichterJaeger.idx()],5);
}

/// Antwortleiste: „Absage: Nichtangriffspakt“ mit Text darf kein neues Angebot auslösen (Emulator 19.10.2026: der Bot las
/// nur das Wort „nichtangriffspakt“ und bot sofort wieder an).
#[test]
fn absage_auf_angebot_bietet_nicht_sofort_neu_an() {
    let mut w=fixture();let name=w.spieler[1].name.clone();
    act(&mut w,0,json!({"typ":"brief_senden","kanal":"privat","an":name,"betreff":"Absage: Nichtangriffspakt","text":"Spaeter vielleicht"}));
    let s=answer(&mut w);assert!(s.contains("Absage"),"{s}");
    assert!(!w.vertraege.iter().any(|v|v.a==1&&v.b==0),"nach einer Absage kein neues Angebot");
}

#[test]
fn refusal_never_creates_trade_and_bot_reply_does_not_loop() {
    let mut w=fixture();letter(&mut w,"Verkaufe nicht 100 erz für 1",None);assert!(answer(&mut w).contains("keinen Auftrag"));assert!(w.orders.is_empty());
    let n=w.nachrichten.len();reagieren_mit(&mut w,0,Bottyp::Oekonom,&[0,1].into_iter().collect());assert_eq!(w.nachrichten.len(),n);
    kommunizieren(&mut w,0,Bottyp::Oekonom);assert_eq!(w.nachrichten.len(),n,"host economic turns must not reclassify bot letters as human requests");
}

#[test]
fn public_trade_and_alliance_chat_use_the_same_real_actions() {
    let mut w=fixture();let p=w.spieler[0].heimat as usize;let k=w.planeten[p].koord;
    act(&mut w,0,json!({"typ":"markt_order","planet":k,"gut":"erz","seite":"verkauf","menge":100,"preis":1}));
    let credits=w.spieler[1].credits;letter(&mut w,"Kaufe 100 erz für 1",None);let s=answer(&mut w);assert!(s.contains("Ja."),"{s}");
    assert!(w.spieler[1].credits<credits);assert!(w.handel.iter().any(|h|h.kaeufer==1&&h.verkaeufer==0&&h.menge==100*M));
    alliance(&mut w);act(&mut w,0,json!({"typ":"brief_senden","kanal":"allianz","an":"","betreff":"Handel","text":"Verkaufe 100 kristall für 1,5","antwort_auf":null}));
    reagieren_mit(&mut w,1,Bottyp::Haendler,&[1].into_iter().collect());assert!(w.kommunikation.angebote.iter().any(|o|o.von==1&&o.gut==Gut::Kristall&&o.menge==100*M));
    let n=w.kommunikation.angebote.len();reagieren_mit(&mut w,1,Bottyp::Haendler,&[1].into_iter().collect());assert_eq!(w.kommunikation.angebote.len(),n);
}

#[test]
fn all_colony_strategies_include_real_processing_and_distinct_priorities() {
    let w=fixture();let r=&w.regeln;
    for t in Bottyp::ALLE {let z=ziele(r,4,t,false,false,true);for g in [Gebaeude::Giesserei,Gebaeude::Elektronikwerk,Gebaeude::Akademie,Gebaeude::Raumhafen]{assert!(z.iter().any(|(b,n)|*b==g&&*n>0),"{t:?} missing {g:?}");}}
    assert!(ziele(r,4,Bottyp::Haendler,false,false,true).contains(&(Gebaeude::Markt,2)));
    assert!(ziele(r,4,Bottyp::Igel,false,false,true).contains(&(Gebaeude::Bunker,3)));
    assert!(ziele(r,4,Bottyp::Raeuber,false,false,true).contains(&(Gebaeude::Werft,4)));
    assert!(ziele(r,4,Bottyp::Oekonom,false,false,true).contains(&(Gebaeude::Elektronikwerk,4)));
}

#[test]
fn bot_releases_power_and_workers_then_restores_output_when_capacity_exists() {
    let mut w=fixture();let p=w.spieler[0].heimat as usize;w.planeten[p].gebaeude[Gebaeude::Erzmine.idx()]=15;w.planeten[p].gebaeude[Gebaeude::Kristallmine.idx()]=15;w.planeten[p].gebaeude[Gebaeude::Deuteriumsynthesizer.idx()]=10;
    // Solar 4 (bis 20.10.2026 Solar 5; seit Energie +10 % deckte 5 den Bedarf schon fast).
    w.planeten[p].gebaeude[Gebaeude::Solarkraftwerk.idx()]=4;w.raten_neu(p);
    let before=w.industrie_berechnen(&w.planeten[p]);assert!(before.energie_deckung_promille<1000);
    last_ausgleichen(&mut w,0);let after=w.industrie_berechnen(&w.planeten[p]);assert!(after.energie_verbrauch<before.energie_verbrauch);assert!(after.arbeit_bedarf<before.arbeit_bedarf);
    let low=w.produktionsstufen(p);assert!(low[Gebaeude::Erzmine.idx()]<100||low[Gebaeude::Kristallmine.idx()]<100);
    w.planeten[p].gebaeude[Gebaeude::Solarkraftwerk.idx()]=25;w.planeten[p].bevoelkerung=100_000*M;w.raten_neu(p);
    // Worker capacity follows actual population groups, not just the legacy counter.
    let v=w.spieler[0].volk;w.ausbau.personen.get_mut(&(p as PlanetId)).unwrap().gruppen[v.idx()][0]=100_000*M;w.raten_neu(p);
    last_ausgleichen(&mut w,0);assert!(w.produktionsstufen(p).iter().zip(low).any(|(a,b)|*a>b));
}

#[test]
fn returning_scouts_count_towards_replacement_orders_and_are_reusable() {
    let mut w=fixture();let p=w.spieler[0].heimat as usize;let home=w.planeten[p].koord;
    let target=(1..=12).map(|position|Koord::neu(home.sektor,home.system,position)).find(|k|!w.belegung.contains_key(k)).unwrap();
    w.planeten[p].einheiten[Einheit::Spionagesonde.idx()]=4;
    let a=sondenauftrag(&w,0,p,target,"spionage",4).unwrap();act(&mut w,0,a);
    assert_eq!(sonden_geplant(&w,0,p),4);
    assert_eq!(w.planeten[p].einheiten[Einheit::Spionagesonde.idx()],0);
    let id=*w.flotten.keys().next().unwrap();let arrival=w.flotten[&id].ankunft;
    while w.zeit<arrival&&w.schritt_bis(arrival){}
    assert_eq!(w.flotten[&id].zustand,Flottenzustand::Rueckflug);
    assert_eq!(sonden_geplant(&w,0,p),4);
    let landing=w.flotten[&id].rueckkehr;
    while w.zeit<landing&&w.schritt_bis(landing){}
    assert_eq!(w.planeten[p].einheiten[Einheit::Spionagesonde.idx()],4);
    assert_eq!(sonden_geplant(&w,0,p),4);
    assert!(sondenauftrag(&w,0,p,target,"spionage",2).is_some());
    assert_eq!(w.spieler[0].statistik.abgelehnt,0);
}

#[test]
fn scout_planning_slows_down_for_round_trip_budget_and_waits_if_unfunded() {
    let mut w=fixture();let p=w.spieler[0].heimat as usize;let home=w.planeten[p].koord;
    let target=Koord::neu(home.sektor,if home.system==1{60}else{1},6);
    w.planeten[p].einheiten[Einheit::Spionagesonde.idx()]=2;
    let mut ships=[0;SCHIFFE];ships[Einheit::Spionagesonde.idx()]=2;
    let slow=w.flugplan(0,p,target,&ships,100).unwrap();let fast=w.flugplan(0,p,target,&ships,1000).unwrap();
    assert!(fast.treibstoff>slow.treibstoff);
    w.planeten[p].bestand[Gut::Deuterium.idx()]=slow.treibstoff*2;
    let a=sondenauftrag(&w,0,p,target,"system_erkunden",2).unwrap();
    assert!(a["geschwindigkeit"].as_f64().unwrap()<1.0);act(&mut w,0,a);
    assert_eq!(w.spieler[0].statistik.abgelehnt,0);
    w.planeten[p].einheiten[Einheit::Spionagesonde.idx()]=2;
    w.planeten[p].bestand[Gut::Deuterium.idx()]=0;
    assert!(sondenauftrag(&w,0,p,target,"system_erkunden",2).is_none());
}

#[test]
fn cramped_bot_home_prioritizes_laboratory_then_terraforming() {
    let mut w=fixture();let p=w.spieler[0].heimat as usize;terraforming_mit_feldern(&mut w);
    w.spieler[0].stufe=4;w.spieler[0].forschung[Forschung::Terraforming.idx()]=0;
    w.planeten[p].gebaeude[Gebaeude::Labor.idx()]=4;
    w.planeten[p].gebaeude[Gebaeude::Erzmine.idx()]=12;
    w.planeten[p].gebaeude[Gebaeude::Lager.idx()]=15;
    w.planeten[p].bestand.fill(1_000_000*M);w.raten_neu(p);
    w.planeten[p].felder=w.felder_belegt(p)+5;
    assert_eq!(waehle_bau(&w,0,p,Bottyp::Oekonom),Some(Gebaeude::Labor));
    w.planeten[p].gebaeude[Gebaeude::Labor.idx()]=6;
    forschen(&mut w,0,Bottyp::Oekonom);
    assert_eq!(w.spieler[0].forschung_aktiv.as_ref().unwrap().forschung,Forschung::Terraforming);
}

#[test]
fn saturated_bot_home_recovers_lab_access_from_surplus_mine_without_touching_life_support() {
    let mut w=fixture();let p=w.spieler[0].heimat as usize;terraforming_mit_feldern(&mut w);
    w.spieler[0].stufe=4;w.spieler[0].forschung[Forschung::Terraforming.idx()]=0;
    w.planeten[p].gebaeude[Gebaeude::Labor.idx()]=4;
    w.planeten[p].gebaeude[Gebaeude::Erzmine.idx()]=12;
    w.planeten[p].gebaeude[Gebaeude::Lager.idx()]=15;
    w.planeten[p].bestand.fill(1_000_000*M);w.raten_neu(p);
    w.planeten[p].felder=w.felder_belegt(p);
    let before=w.planeten[p].gebaeude;
    bauen(&mut w,0,p,Bottyp::Oekonom);
    assert_eq!(w.planeten[p].gebaeude[Gebaeude::Erzmine.idx()],11);
    for g in [Gebaeude::Farm,Gebaeude::Solarkraftwerk,Gebaeude::Wohnblock,Gebaeude::Giesserei]{assert_eq!(w.planeten[p].gebaeude[g.idx()],before[g.idx()]);}
    assert!(w.planeten[p].bauschleife.iter().any(|a|a.gebaeude==Gebaeude::Labor));
    assert_eq!(w.spieler[0].statistik.abgelehnt,0);
}

/// 20.10.2026 (bot_lage: 30 Tage ohne einen Kampf): ein Räuber mit schweren Jägern greift ein verteidigtes, reiches
/// Ziel an, sobald sein Bericht die Verteidigung zeigt; Neulinge bleiben verschont, und an einem Ort wird höchstens
/// einmal je Tag gekämpft.
#[test]
fn raeuber_kaempft_gegen_verteidigung_verschont_neulinge_und_pausiert() {
    let mut w=fixture();let p=w.spieler[0].heimat as usize;let ziel=w.spieler[1].heimat as usize;let ort=w.planeten[ziel].koord;
    w.zeit=12*TAG;w.spieler[1].schutz_bis=0;
    w.planeten[p].gebaeude[Gebaeude::Werft.idx()]=4;
    for (e,n) in [(Einheit::SchwererJaeger,80),(Einheit::KleinerTransporter,20)] {w.planeten[p].einheiten[e.idx()]=n;}
    w.planeten[p].bestand.fill(100_000*M);
    w.planeten[ziel].einheiten[Einheit::Lasergeschuetz.idx()]=20;
    w.planeten[ziel].bestand.fill(200_000*M);
    let bericht=|w:&Welt|Spionagebericht{zeit:w.zeit,ziel:ort,besitzer:1,bestand:w.planeten[ziel].bestand.to_vec(),
        schiffe:Some(w.planeten[ziel].einheiten[..SCHIFFE].to_vec()),verteidigung:Some(w.planeten[ziel].einheiten[SCHIFFE..].to_vec()),gebaeude:None,forschung:None};
    let angriff=|w:&Welt|w.flotten.values().any(|f|f.besitzer==0&&f.ziel==ort&&f.mission==Mission::Angriff);
    let mut bot=Bot::neu(Bottyp::Raeuber,0);bot.opferschutz.aktiv=true; // Schonzeit seit 23.10.2026 standardmäßig aus
    // Neuling: Stufe II und erst seit einem Tag dabei.
    w.spieler[1].stufe=2;w.planeten[ziel].gegruendet=w.zeit-TAG;
    let b=bericht(&w);w.spieler[0].berichte.push(b);
    raeuber(&mut w,0,&mut bot);
    assert!(!angriff(&w),"kein Angriff auf einen Neuling");
    // Opferschutz beim Serverstart abgeschaltet: dann auch der Neuling.
    bot.opferschutz.aktiv=false;
    raeuber(&mut w,0,&mut bot);
    assert!(angriff(&w),"ohne Opferschutz greift der Räuber auch Neulinge an: {:?}",diagnose_zaehler());
    bot.opferschutz.aktiv=true;
    w.planeten[ziel].gegruendet=0;
    raeuber(&mut w,0,&mut bot);
    assert!(angriff(&w),"verteidigtes, reiches Ziel nach Bericht mit Verteidigung: {:?}",diagnose_zaehler());
    assert_eq!(w.spieler[0].statistik.abgelehnt,0);
    let ende=w.zeit+10*TAG;
    while w.kampfberichte.is_empty()&&w.zeit<ende {w.schritt();}
    let k=w.kampfberichte.last().expect("Kampf am Ziel").clone();
    assert_eq!((k.angreifer,k.ort,k.sieger),(0,ort,Sieger::Angreifer));
    assert!(k.truemmer[0]+k.truemmer[1]>0,"der Kampf gegen Verteidigung hinterlässt Trümmer");
    while w.flotten.values().any(|f|f.besitzer==0&&f.mission==Mission::Angriff) {w.schritt();}
    // Neuer Bericht, aber am Ort wurde vor weniger als einem Tag gekämpft: kein zweiter Angriff.
    w.planeten[ziel].bestand.fill(200_000*M);
    let b=bericht(&w);w.spieler[0].berichte.push(b);
    let pause=bot.opferschutz.pause_stunden as i64*STUNDE;
    if w.zeit-k.zeit<pause {raeuber(&mut w,0,&mut bot);assert!(!angriff(&w),"höchstens ein Angriff je Ort und Tag");}
    while w.zeit<k.zeit+pause {w.schritt();}
    w.planeten[ziel].bestand.fill(200_000*M);
    let b=bericht(&w);w.spieler[0].berichte.push(b);
    raeuber(&mut w,0,&mut bot);
    assert!(angriff(&w),"nach einem Tag wieder");
}

/// 20.10.2026: eine volle Welt tauscht eine Minenstufe bei vollem Lager gegen genau das Gebäude, das ihr fehlt –
/// auch wenn die Förderrate bei vollem Lager 0 ist (zwei Ökonomen blieben mit 32 % Konsumgütern auf Stufe III).
#[test]
fn volle_welt_tauscht_mine_gegen_konsumgueterwerk_und_wohnblock() {
    for wohnen in [false,true] {
        let mut w=fixture();let p=w.spieler[0].heimat as usize;
        for (g,n) in [(Gebaeude::Labor,6),(Gebaeude::Erzmine,15),(Gebaeude::Lager,10),(Gebaeude::Wohnblock,10),(Gebaeude::Konsumgueterwerk,4),
            (Gebaeude::Solarkraftwerk,25),(Gebaeude::Farm,15)] {w.planeten[p].gebaeude[g.idx()]=n;}
        w.planeten[p].bestand.fill(1_000_000*M);w.raten_neu(p);
        w.spieler[0].forschung[Forschung::Terraforming.idx()]=0;w.planeten[p].felder=w.felder_belegt(p);
        w.planeten[p].rate[Gut::Erz.idx()]=0;w.planeten[p].rate[Gut::Nahrung.idx()]=1000*M;
        w.planeten[p].energie_erzeugung=w.planeten[p].energie_verbrauch+1000*M;
        if wohnen {w.planeten[p].konsum_deckung=1000;w.planeten[p].bevoelkerung=mal(w.planeten[p].wohnraum,0.6);}
        else {w.planeten[p].konsum_deckung=320;w.planeten[p].bevoelkerung=mal(w.planeten[p].wohnraum,0.3);}
        bauen(&mut w,0,p,Bottyp::Oekonom);
        let soll=if wohnen {Gebaeude::Wohnblock} else {Gebaeude::Konsumgueterwerk};
        assert_eq!(w.planeten[p].gebaeude[Gebaeude::Erzmine.idx()],14,"wohnen={wohnen}");
        assert!(w.planeten[p].bauschleife.first().is_some_and(|a|a.gebaeude==soll),"wohnen={wohnen}: {:?}",w.planeten[p].bauschleife);
        assert_eq!(w.spieler[0].statistik.abgelehnt,0);
    }
}

#[test]
fn synthetic_xeno_waits_for_industry_surplus_and_restarts_with_reserves() {
    let mut w=fixture();let p=w.spieler[0].heimat as usize;
    w.planeten[p].gebaeude[Gebaeude::Xenoextraktor.idx()]=1;
    assert!(!w.xeno_planet(w.planeten[p].koord));
    w.planeten[p].bestand[Gut::Elektronik.idx()]=1000*M;
    last_ausgleichen(&mut w,0);
    assert_eq!(w.produktionsstufen(p)[Gebaeude::Xenoextraktor.idx()],0);
    w.planeten[p].bestand[Gut::Elektronik.idx()]=15000*M;
    w.planeten[p].bestand[Gut::Legierung.idx()]=25000*M;
    w.planeten[p].bestand[Gut::Xenokristall.idx()]=0;
    last_ausgleichen(&mut w,0);
    assert_eq!(w.produktionsstufen(p)[Gebaeude::Xenoextraktor.idx()],100);
    w.planeten[p].bestand[Gut::Xenokristall.idx()]=6000*M;
    last_ausgleichen(&mut w,0);
    assert_eq!(w.produktionsstufen(p)[Gebaeude::Xenoextraktor.idx()],0);
}

// ---------------------------------------------------------------------------------------------------------------------
// Handel per Brief (Karl 20.10.2026): Brief → Angebot → Order des Gegenübers → Gegenorder des Bots → Abschluss im Kern.
// Spieler 0 ist der Gegenüber (Mensch, Agent oder Bot sind für den Brief gleich), Spieler 1 der Händler-Bot.
// ---------------------------------------------------------------------------------------------------------------------
mod handel_tests {
    use super::*;
    use dialog::handel;

    /// Der Bot hat Überschuss an Legierung, Elektronik und Deuterium und braucht Kristall und Erz (Vorrat fehlt).
    fn handelswelt() -> Welt {
        let mut w = fixture();
        let p = w.spieler[1].heimat as usize;
        w.planeten[p].bestand[Gut::Kristall.idx()] = 0;
        w.planeten[p].bestand[Gut::Erz.idx()] = 0;
        w.raten_neu(p);
        w
    }
    fn ort(w: &Welt, sid: usize) -> Koord {
        w.planeten[w.spieler[sid].heimat as usize].koord
    }
    fn order(w: &mut Welt, sid: u16, gut: Gut, seite: &str, menge: i64, preis: f64) {
        let k = ort(w, sid as usize);
        act(w, sid, json!({"typ": "markt_order", "planet": k, "gut": gut.name(), "seite": seite, "menge": menge, "preis": preis}));
    }
    fn hook(w: &mut Welt) {
        reagieren_mit(w, 1, Bottyp::Haendler, &[1].into_iter().collect());
    }
    /// Preisspanne des Händler-Bots (Spieler 1) für `g`; Briefe nennen Preise mit `handel::p`.
    fn spanne(w: &Welt, g: Gut, bot_verkauft: bool) -> handel::Band {
        handel::band(w, g, Bottyp::Haendler, bot_verkauft)
    }
    fn teile(w: &Welt) -> Vec<handel::Teil> {
        handel::gespraech(w, 1, 0).teile.iter().map(|x| x.0).collect()
    }
    fn bestand(w: &Welt, sid: usize, g: Gut) -> i64 {
        w.bestand_jetzt(w.spieler[sid].heimat as usize)[g.idx()]
    }
    /// Alle Briefe des Bots an Spieler 0, älteste zuerst.
    fn briefe_des_bots(w: &Welt) -> Vec<String> {
        w.nachrichten.iter().filter(|n| n.von == 1 && n.an.contains(&0) && !n.allianz).map(|n| n.text.clone()).collect()
    }
    /// Bis zur Lieferung der neutralen Handelsflotte vorspulen.
    fn liefern(w: &mut Welt) {
        let ende = w.zeit + TAG;
        while w.zeit < ende && w.schritt_bis(ende) {}
    }
    /// Nichts im Brief verrät, dass der Absender ein Skript ist.
    fn menschlich(s: &str) {
        let l = s.to_lowercase();
        for w in ["skript", "script", " bot ", "bot-", "ki-", "künstliche", "automatisch", "algorithmus", "programm"] {
            assert!(!l.contains(w), "verrät die Art des Absenders ({w}): {s}");
        }
        assert!(s.chars().count() <= 1_120, "Brief zu lang ({}): {s}", s.chars().count());
    }

    #[test]
    fn brief_angebot_order_bedient_buchung_und_lieferung() {
        let mut w = handelswelt();
        letter(&mut w, "Ich brauche 2000 Legierung", None);
        let s = answer(&mut w);
        menschlich(&s);
        let b = spanne(&w, Gut::Legierung, true);
        assert!(s.contains(&format!("Mein Angebot: ich verkaufe 2000 Legierung zu {} Credits je Einheit.", handel::p(b.offen))), "{s}");
        assert!(s.contains(&format!("{} Credits", handel::c(2000.0 * b.offen))), "Gesamtsumme fehlt: {s}");
        assert!(s.contains("Handel & Markt") && s.contains("Order einstellen") && s.contains("Kaufen"), "UI-Weg fehlt: {s}");
        assert!(s.contains("liefere ich nichts"), "{s}");
        let t = teile(&w);
        assert_eq!(t.len(), 1);
        assert!(t[0].bot_verkauft && t[0].menge == 2000 && t[0].gut == Gut::Legierung);
        assert!(w.handel.is_empty() && w.orders.is_empty(), "ohne Order des Gegenübers geschieht nichts");
        let (c0, c1, b1, b0) = (w.spieler[0].credits, w.spieler[1].credits, bestand(&w, 1, Gut::Legierung), bestand(&w, 0, Gut::Legierung));
        // Der Zustand steht im Brief: er übersteht Speichern und Laden.
        w = Welt::aus_bytes(&w.zu_bytes()).unwrap();
        assert_eq!(teile(&w), t);
        // Der Gegenüber stellt die genannte Order ein; der Bot bedient sie beim nächsten Befehl.
        order(&mut w, 0, Gut::Legierung, "kauf", 2000, t[0].preis);
        assert_eq!(w.orders.len(), 1);
        hook(&mut w);
        assert!(w.orders.is_empty(), "die Order muss bedient sein: {:?}", w.orders);
        let h = w.handel.last().unwrap().clone();
        assert_eq!((h.kaeufer, h.verkaeufer, h.gut, h.menge, h.preis), (0, 1, Gut::Legierung, 2000 * M, milli(b.offen)));
        // Den Wert plus Gebühr zahlt der Käufer, der Verkäufer bekommt ihn minus seine Gebühr (Marktgebühr 2 % mal Volksfaktor).
        let wert = 2000 * milli(b.offen);
        let (g0, g1) = (w.gebuehr(0, Some(1)), w.gebuehr(1, Some(0)));
        assert_eq!(c0 - w.spieler[0].credits, wert + mal(wert, g0));
        assert_eq!(w.spieler[1].credits - c1, wert - mal(wert, g1));
        assert_eq!(b1 - bestand(&w, 1, Gut::Legierung), 2000 * M, "die Ware verlässt den Bot erst durch die Order");
        assert_eq!(bestand(&w, 0, Gut::Legierung), b0, "geliefert wird erst, wenn die Handelsflotte ankommt");
        liefern(&mut w);
        assert_eq!(bestand(&w, 0, Gut::Legierung) - b0, 2000 * M, "echte Lieferung beim Käufer");
        // Bestätigungsbrief mit erledigtem Angebot.
        let letzter = briefe_des_bots(&w).pop().unwrap();
        menschlich(&letzter);
        assert!(letzter.contains("Abgeschlossen") && letzter.contains("2000 Legierung"), "{letzter}");
        assert!(letzter.contains("Angebot erledigt: Legierung verkauft."), "{letzter}");
        assert!(teile(&w).is_empty());
        // Keine Doppellieferung: weder „Einverstanden“ noch weitere Bot-Züge liefern noch einmal.
        let n = w.handel.len();
        letter(&mut w, "Einverstanden", None);
        let s = answer(&mut w);
        assert!(s.contains("kein offenes Angebot"), "{s}");
        hook(&mut w);
        handel::aktiv_bedienen(&mut w, 1, Bottyp::Haendler);
        assert_eq!(w.handel.len(), n);
    }

    #[test]
    fn einverstanden_bedient_die_wartende_order_und_nennt_sonst_den_weg() {
        let mut w = handelswelt();
        letter(&mut w, "Verkaufst du mir 1000 Legierung?", None);
        answer(&mut w);
        let t = teile(&w)[0];
        // Order noch nicht da: der Bot nennt, was er erwartet, und liefert nichts.
        letter(&mut w, "Deal", None);
        let s = answer(&mut w);
        assert!(s.contains("warte ich noch") && s.contains("Mein Angebot: ich verkaufe 1000 Legierung"), "{s}");
        assert!(w.handel.is_empty());
        // Order, aber unter dem Angebot: sie wird nicht bedient, der Bot sagt, warum.
        let b = spanne(&w, Gut::Legierung, true);
        let unter = ((b.grenze + b.offen) / 2.0 * 100.0).round() / 100.0;
        order(&mut w, 0, Gut::Legierung, "kauf", 1000, unter);
        hook(&mut w);
        assert!(w.handel.is_empty(), "unter dem Angebot wird nicht bedient");
        letter(&mut w, "Einverstanden", None);
        let s = answer(&mut w);
        assert!(s.contains(&format!("liegt mit {} Credits unter meinem Angebot", handel::p(unter))), "{s}");
        // Order zum Angebot: „Einverstanden“ genügt, der Bot bedient auch ohne den Hook.
        let id = w.orders[0].id;
        act(&mut w, 0, json!({"typ": "markt_storno", "order": id}));
        order(&mut w, 0, Gut::Legierung, "kauf", 1000, t.preis);
        letter(&mut w, "Einverstanden", None);
        let s = answer(&mut w);
        assert!(s.contains("Abgeschlossen") && s.contains("1000 Legierung") && s.contains("Angebot erledigt: Legierung verkauft."), "{s}");
        assert_eq!(w.handel.len(), 1);
    }

    #[test]
    fn gegenvorschlag_annahme_und_letzte_grenze() {
        let mut w = handelswelt();
        letter(&mut w, "Verkaufst du mir 1000 Legierung?", None);
        let s = answer(&mut w);
        let b = spanne(&w, Gut::Legierung, true);
        assert!(s.contains(&format!("zu {} Credits", handel::p(b.offen))), "{s}");
        // Zu niedrig: Gegenvorschlag auf halbem Weg zur Grenze.
        let q1 = ((b.offen + b.grenze) / 2.0 * 1000.0 - 1e-6).ceil() / 1000.0;
        let bieten = |x: f64| format!("Ich biete {} je Legierung", handel::p(x));
        letter(&mut w, &bieten(b.grenze + (q1 - b.grenze) * 0.4), None);
        let s = answer(&mut w);
        let gegen = format!("Mein Angebot: ich verkaufe 1000 Legierung zu {} Credits", handel::p(q1));
        assert!(s.contains("Gegenvorschlag") && s.contains(&gegen), "{s}");
        // Auf den Gegenvorschlag eingehen: die Annahme gilt zum Gegenvorschlag.
        letter(&mut w, &bieten(q1 + 0.05), None);
        let s = answer(&mut w);
        assert!(s.contains("Einverstanden") && s.contains(&gegen), "{s}");
        // Weit unter der Grenze: letzte Grenze, das Angebot bleibt.
        letter(&mut w, &bieten(b.grenze * 0.4), None);
        let s = answer(&mut w);
        assert!(s.contains("letzte Grenze") && s.contains(&handel::p(b.grenze)), "{s}");
        assert!(teile(&w)[0].preis > b.grenze, "das Angebot bleibt über der Grenze");
        // Ein sehr hohes Gebot kostet nicht mehr als das Angebot.
        letter(&mut w, &format!("Ich biete {} Credits je Legierung", handel::p(b.offen * 3.0)), None);
        let s = answer(&mut w);
        assert!(s.contains("Mehr als") && s.contains("musst du nicht bieten"), "{s}");
        for b in briefe_des_bots(&w) {
            menschlich(&b);
        }
        assert!(w.handel.is_empty() && w.orders.is_empty());
    }

    #[test]
    fn unfaire_preise_werden_abgelehnt_und_ungefragt_nur_zum_eroeffnungspreis_bedient() {
        let mut w = handelswelt();
        // Ohne Gespräch und weit unter dem Eröffnungspreis: auch im Bot-Zug kein Abschluss.
        let b = spanne(&w, Gut::Legierung, true);
        order(&mut w, 0, Gut::Legierung, "kauf", 1000, b.grenze * 0.4);
        w.zeit += 600;
        handel::aktiv_bedienen(&mut w, 1, Bottyp::Haendler);
        hook(&mut w);
        assert!(w.handel.is_empty(), "40 % der Grenze bleiben unbedient");
        // Zum Eröffnungspreis wird die Order erst nach der Vorrangzeit bedient, begrenzt auf ein Viertel des Überschusses.
        let id = w.orders[0].id;
        act(&mut w, 0, json!({"typ": "markt_storno", "order": id}));
        order(&mut w, 0, Gut::Legierung, "kauf", 1000, b.offen);
        handel::aktiv_bedienen(&mut w, 1, Bottyp::Haendler);
        assert!(w.handel.is_empty(), "Gesprächspartner haben Vorrang: erst nach fünf Spielminuten");
        w.zeit += 301;
        handel::aktiv_bedienen(&mut w, 1, Bottyp::Haendler);
        assert_eq!(w.handel.len(), 1);
        assert_eq!((w.handel[0].kaeufer, w.handel[0].verkaeufer, w.handel[0].menge), (0, 1, 1000 * M));
        // Ein Gebot weit unter der Grenze erzeugt auch im Gespräch nie einen Abschluss.
        letter(&mut w, "Ich brauche 1000 Legierung", None);
        answer(&mut w);
        let n = w.handel.len();
        letter(&mut w, "Ich biete 0,01 je Legierung", None);
        let s = answer(&mut w);
        assert!(s.contains("letzte Grenze"), "{s}");
        order(&mut w, 0, Gut::Legierung, "kauf", 1000, 0.01);
        hook(&mut w);
        assert_eq!(w.handel.len(), n);
    }

    #[test]
    fn bot_kauft_was_er_braucht_und_zahlt_nur_innerhalb_seiner_grenzen() {
        let mut w = handelswelt();
        letter(&mut w, "Ich verkaufe dir 500 Kristall", None);
        let s = answer(&mut w);
        menschlich(&s);
        let b = spanne(&w, Gut::Kristall, false);
        assert!(s.contains(&format!("Mein Angebot: ich kaufe 500 Kristall zu {} Credits je Einheit.", handel::p(b.offen))), "{s}");
        assert!(s.contains("Verkaufen") && s.contains("Order einstellen") && s.contains("zahle sofort"), "{s}");
        let t = teile(&w)[0];
        assert!(!t.bot_verkauft);
        // Fordert der Gegenüber zu viel, geht der Bot bis zur Grenze, nie darüber.
        letter(&mut w, &format!("Ich verkaufe dir 500 Kristall für {}", handel::p(b.grenze * 2.0)), None);
        let s = answer(&mut w);
        assert!(s.contains("letzte Grenze") && s.contains(&handel::p(b.grenze)), "{s}");
        let (c0, c1, k0, k1) = (w.spieler[0].credits, w.spieler[1].credits, bestand(&w, 0, Gut::Kristall), bestand(&w, 1, Gut::Kristall));
        // Verkaufsorder zu hoch: nicht bedient. Zum Angebot: bedient.
        order(&mut w, 0, Gut::Kristall, "verkauf", 500, b.grenze * 2.0);
        hook(&mut w);
        assert!(w.handel.is_empty());
        let id = w.orders[0].id;
        act(&mut w, 0, json!({"typ": "markt_storno", "order": id}));
        let angebot = teile(&w)[0];
        order(&mut w, 0, Gut::Kristall, "verkauf", 500, angebot.preis);
        hook(&mut w);
        let h = w.handel.last().unwrap().clone();
        assert_eq!((h.kaeufer, h.verkaeufer, h.gut, h.menge), (1, 0, Gut::Kristall, 500 * M));
        let wert = 500 * milli(angebot.preis);
        assert_eq!(w.spieler[1].credits, c1 - wert - mal(wert, w.gebuehr(1, Some(0))), "der Bot zahlt Wert plus Gebühr");
        assert_eq!(w.spieler[0].credits, c0 + wert - mal(wert, w.gebuehr(0, Some(1))), "der Verkäufer bekommt Wert minus Gebühr");
        assert_eq!(bestand(&w, 0, Gut::Kristall), k0 - 500 * M);
        liefern(&mut w);
        assert!(bestand(&w, 1, Gut::Kristall) >= k1 + 500 * M, "Lieferung beim Bot");
        // Was der Bot nicht braucht, kauft er nicht.
        letter(&mut w, "Ich verkaufe dir 500 Legierung", None);
        let s = answer(&mut w);
        assert!(s.contains("Legierung brauche ich gerade nicht"), "{s}");
        assert!(w.orders.is_empty());
    }

    #[test]
    fn tausch_sind_zwei_orders_beide_gegen_credits() {
        let mut w = handelswelt();
        letter(&mut w, "Tausche 8000 Kristall gegen 2000 Legierung", None);
        let s = answer(&mut w);
        menschlich(&s);
        assert!(s.contains("Tausch möglich") && s.contains("zwei Orders"), "{s}");
        let t = teile(&w);
        assert_eq!(t.len(), 2, "{s}");
        let (nehm, gib) = (t.iter().find(|x| x.bot_verkauft).unwrap(), t.iter().find(|x| !x.bot_verkauft).unwrap());
        assert_eq!((gib.gut, gib.menge, nehm.gut, nehm.menge), (Gut::Kristall, 8000, Gut::Legierung, 2000));
        let (l0, k1, c0) = (bestand(&w, 0, Gut::Legierung), bestand(&w, 1, Gut::Kristall), w.spieler[0].credits);
        // Ohne die zweite Order wird die erste trotzdem nur gegen Credits bedient: nichts wird ohne Gegenleistung geliefert.
        order(&mut w, 0, Gut::Kristall, "verkauf", 8000, gib.preis);
        hook(&mut w);
        assert_eq!(w.handel.len(), 1);
        assert!(w.spieler[0].credits > c0, "der Erlös der Verkaufsorder steht sofort bereit");
        order(&mut w, 0, Gut::Legierung, "kauf", 2000, nehm.preis);
        hook(&mut w);
        assert_eq!(w.handel.len(), 2);
        assert!(w.orders.is_empty() && teile(&w).is_empty());
        let verkauf = 8000 * milli(gib.preis);
        let kauf = 2000 * milli(nehm.preis);
        let g = w.gebuehr(0, Some(1));
        assert_eq!(w.spieler[0].credits, c0 + verkauf - mal(verkauf, g) - kauf - mal(kauf, g));
        liefern(&mut w);
        assert_eq!(bestand(&w, 0, Gut::Legierung) - l0, 2000 * M);
        assert!(bestand(&w, 1, Gut::Kristall) >= k1 + 8000 * M);
    }

    #[test]
    fn tausch_ungleicher_werte_bekommt_einen_ausgeglichenen_gegenvorschlag() {
        let mut w = handelswelt();
        letter(&mut w, "Tausche 3000 Kristall gegen 2000 Legierung", None);
        let s = answer(&mut w);
        assert!(s.contains("zu deinen Ungunsten") || s.contains("zu einseitig"), "{s}");
        let t = teile(&w);
        let (nehm, gib) = (t.iter().find(|x| x.bot_verkauft).unwrap(), t.iter().find(|x| !x.bot_verkauft).unwrap());
        let (v_gib, v_nehm) = (gib.menge as f64 * gib.preis, nehm.menge as f64 * nehm.preis);
        assert!((v_gib - v_nehm).abs() <= 0.21 * v_gib.max(v_nehm), "ausgeglichen: {v_gib} gegen {v_nehm}");
    }

    #[test]
    fn teilweise_orders_zaehlen_gegen_das_angebot_und_nichts_wird_doppelt_geliefert() {
        let mut w = handelswelt();
        letter(&mut w, "Ich brauche 2000 Legierung", None);
        answer(&mut w);
        let preis = teile(&w)[0].preis;
        order(&mut w, 0, Gut::Legierung, "kauf", 800, preis);
        hook(&mut w);
        assert_eq!(w.handel.iter().map(|h| h.menge).sum::<i64>(), 800 * M);
        let t = teile(&w);
        assert_eq!(t.len(), 1);
        assert_eq!(t[0].menge, 1200, "der Rest steht im Brief");
        // Größere Order als der Rest: nur der Rest wird bedient.
        order(&mut w, 0, Gut::Legierung, "kauf", 5000, preis);
        hook(&mut w);
        assert_eq!(w.handel.iter().map(|h| h.menge).sum::<i64>(), 2000 * M, "genau die angebotene Menge");
        assert!(teile(&w).is_empty());
        assert_eq!(w.orders.len(), 1, "die Restorder des Gegenübers bleibt im Buch, die des Bots nie");
        assert!(w.orders.iter().all(|o| o.spieler == 0));
        // Weitere Hooks und Bot-Züge liefern nichts mehr.
        let n = w.handel.len();
        for _ in 0..3 {
            hook(&mut w);
            w.zeit += 4000;
            handel::aktiv_bedienen(&mut w, 1, Bottyp::Haendler);
        }
        // Ungefragt darf der Bot die Restorder zum Eröffnungspreis nach Vorrangzeit bedienen – aber höchstens begrenzt.
        let neu: i64 = w.handel[n..].iter().map(|h| h.menge / M).sum();
        assert!(neu <= 5_000, "Tagesgrenze je Gegenüber und Ware: {neu}");
    }

    #[test]
    fn preisangaben_gesamt_oder_stueck_und_zahlenformate() {
        let w = handelswelt();
        let lies = |s: &str| {
            let t = handel::tokens(s);
            handel::lies(&t)
        };
        // 300 Credits für 2000 Erz: Gesamtpreis → 0,15 je Einheit.
        let a = lies("Ich brauche 2000 Erz für 300 Credits").unwrap();
        let (z, h) = a.preis.unwrap();
        assert_eq!(a.menge, Some(2000));
        assert!((handel::preis_aufloesen(&w, Gut::Erz, a.menge, z, h).unwrap() - 0.15).abs() < 1e-9);
        // 0,15 je Erz: Stückpreis.
        let a = lies("Ich biete 0,15 je Erz").unwrap();
        let (z, h) = a.preis.unwrap();
        assert!((handel::preis_aufloesen(&w, Gut::Erz, None, z, h).unwrap() - 0.15).abs() < 1e-9);
        // Alter Befehl bleibt: 100 Erz für 1 ist ein Stückpreis von einem Credit.
        let a = lies("Kaufe 100 erz für 1").unwrap();
        let (z, h) = a.preis.unwrap();
        assert!((handel::preis_aufloesen(&w, Gut::Erz, Some(100), z, h).unwrap() - 1.0).abs() < 1e-9);
        // Deutsche Tausenderpunkte und Kommazahlen.
        let a = lies("Ich brauche 2.000 Kristall").unwrap();
        assert_eq!(a.menge, Some(2000));
        let a = lies("Ich brauche 12.500,5 Kristall").unwrap();
        assert_eq!(a.menge, None, "keine ganze Menge");
        let a = lies("brauche 5k Deuterium").unwrap();
        assert_eq!((a.gut, a.menge), (Some(Gut::Deuterium), Some(5000)));
    }

    #[test]
    fn absichten_aus_freien_deutschen_saetzen() {
        use handel::Art::*;
        let lies = |s: &str| {
            let t = handel::tokens(s);
            handel::lies(&t).map(|a| (a.art, a.gut, a.menge, a.gegen.map(|g| g.0)))
        };
        let faelle: &[(&str, Option<(handel::Art, Option<Gut>, Option<i64>, Option<Gut>)>)] = &[
            ("Ich brauche 2000 Erz", Some((Kauf, Some(Gut::Erz), Some(2000), None))),
            ("Ich benötige 500 Kristalle", Some((Kauf, Some(Gut::Kristall), Some(500), None))),
            ("Verkaufst du mir Kristall?", Some((Kauf, Some(Gut::Kristall), None, None))),
            ("Kannst du mir 1500 Deuterium verkaufen?", Some((Kauf, Some(Gut::Deuterium), Some(1500), None))),
            ("Hast du Legierung übrig?", Some((Kauf, Some(Gut::Legierung), None, None))),
            ("Was willst du für 1000 Deuterium?", Some((Kauf, Some(Gut::Deuterium), Some(1000), None))),
            ("Was kostet 500 Elektronik?", Some((Kauf, Some(Gut::Elektronik), Some(500), None))),
            ("Ich möchte 800 Nahrung kaufen", Some((Kauf, Some(Gut::Nahrung), Some(800), None))),
            ("Ich kaufe 2000 Erz für 0,15", Some((Kauf, Some(Gut::Erz), Some(2000), None))),
            ("Liefere mir 3000 Erz", Some((Kauf, Some(Gut::Erz), Some(3000), None))),
            ("Ich biete 0,15 je Erz", Some((Kauf, Some(Gut::Erz), None, None))),
            ("Ich biete 300 Credits für 2000 Erz", Some((Kauf, Some(Gut::Erz), Some(2000), None))),
            ("Ich verkaufe dir 500 Legierung", Some((Verkauf, Some(Gut::Legierung), Some(500), None))),
            ("Ich verkaufe 500 Legierung für 0,3 je Einheit", Some((Verkauf, Some(Gut::Legierung), Some(500), None))),
            ("Kaufst du Kristall?", Some((Verkauf, Some(Gut::Kristall), None, None))),
            ("Kannst du 2000 Erz von mir kaufen?", Some((Verkauf, Some(Gut::Erz), Some(2000), None))),
            ("Nimmst du 1000 Erz ab?", Some((Verkauf, Some(Gut::Erz), Some(1000), None))),
            ("Was zahlst du für 1000 Erz?", Some((Verkauf, Some(Gut::Erz), Some(1000), None))),
            ("Was bietest du für 4000 Kristall?", Some((Verkauf, Some(Gut::Kristall), Some(4000), None))),
            ("Ich biete dir 700 Elektronik an", Some((Verkauf, Some(Gut::Elektronik), Some(700), None))),
            ("Brauchst du Deuterium?", Some((Verkauf, Some(Gut::Deuterium), None, None))),
            ("Ich habe 3000 Erz zu verkaufen", Some((Verkauf, Some(Gut::Erz), Some(3000), None))),
            ("Wir verkaufen euch 2500 Nahrung", Some((Verkauf, Some(Gut::Nahrung), Some(2500), None))),
            ("Tausche 3000 Kristall gegen 2000 Erz", Some((Tausch, Some(Gut::Kristall), Some(3000), Some(Gut::Erz)))),
            ("Ich tausche Kristall gegen Erz", Some((Tausch, Some(Gut::Kristall), None, Some(Gut::Erz)))),
            ("Ich möchte 2000 Erz gegen Deuterium tauschen", Some((Tausch, Some(Gut::Erz), Some(2000), Some(Gut::Deuterium)))),
            ("Einverstanden", Some((Zustimmung, None, None, None))),
            ("Deal", Some((Zustimmung, None, None, None))),
            ("Ja, abgemacht!", Some((Zustimmung, None, None, None))),
            ("Wollen wir handeln?", Some((Uebersicht, None, None, None))),
            ("Welche Preise hast du?", Some((Uebersicht, None, None, None))),
            ("Hallo, wie geht es dir?", None),
            ("Status", None),
            ("Nichtangriffspakt", None),
        ];
        for (text, soll) in faelle {
            assert_eq!(&lies(text), soll, "{text}");
        }
    }

    #[test]
    fn alter_befehl_bleibt_beim_marktauftrag_des_bots_und_ich_kaufe_ist_kein_befehl() {
        let mut w = handelswelt();
        // Legacy: „Verkaufe 100 erz für 1“ stellt eine Order des Bots ein (live bestätigt am 20.10.2026) – unter dem
        // Marktwert nicht (sonst kauft man beim Bot billig und verkauft im Soforthandel teuer).
        letter(&mut w, "Verkaufe 100 legierung für 1", None);
        let s = answer(&mut w);
        assert!(s.contains("Nein.") && w.orders.is_empty(), "{s}");
        let offen = spanne(&w, Gut::Legierung, true).offen;
        letter(&mut w, &format!("Verkaufe 100 legierung für {}", handel::p(offen)), None);
        let s = answer(&mut w);
        assert!(s.contains("Ja."), "{s}");
        assert_eq!(w.orders.len(), 1, "der Bot stellt selbst eine Verkaufsorder ein: {s}");
        assert_eq!(w.orders[0].spieler, 1);
        assert!(teile(&w).is_empty(), "der alte Befehl ist kein Gespräch");
    }

    #[test]
    fn ich_kaufe_stellt_keine_order_des_bots_ein() {
        let mut w = handelswelt();
        let b = spanne(&w, Gut::Legierung, true);
        letter(&mut w, &format!("Ich kaufe 1000 Legierung für {}", handel::p(b.offen + 1.0)), None);
        let s = answer(&mut w);
        menschlich(&s);
        assert!(w.orders.is_empty(), "der Bot stellt von sich aus nichts ein: {s}");
        assert!(s.contains(&format!("Mein Angebot: ich verkaufe 1000 Legierung zu {} Credits", handel::p(b.offen))), "{s}");
        assert!(s.contains(&format!("Mehr als {} musst du nicht bieten", handel::p(b.offen))), "{s}");
    }

    #[test]
    fn bot_ohne_markt_sagt_es_ehrlich() {
        let mut w = handelswelt();
        let p = w.spieler[1].heimat as usize;
        w.planeten[p].gebaeude[Gebaeude::Markt.idx()] = 0;
        letter(&mut w, "Ich brauche 2000 Legierung", None);
        let s = answer(&mut w);
        assert!(s.contains("noch keinen Markt gebaut"), "{s}");
        assert!(teile(&w).is_empty());
    }

    #[test]
    fn keine_ware_ohne_ueberschuss_und_alternativen() {
        let mut w = handelswelt();
        letter(&mut w, "Ich brauche 2000 Erz", None);
        let s = answer(&mut w);
        assert!(s.contains("Von Erz habe ich gerade nichts übrig") && s.contains("Abgeben könnte ich gerade"), "{s}");
        assert!(teile(&w).is_empty());
        letter(&mut w, "Wollen wir handeln?", None);
        let s = answer(&mut w);
        assert!(s.contains("Gebrauchen könnte ich gerade") && s.contains("Kristall"), "{s}");
    }

    #[test]
    fn menge_wird_auf_das_begrenzt_was_der_bot_wirklich_abgeben_kann() {
        let mut w = handelswelt();
        let p = w.spieler[1].heimat as usize;
        w.planeten[p].bestand[Gut::Legierung.idx()] = 6_000 * M;
        letter(&mut w, "Ich brauche 9000 Legierung", None);
        let s = answer(&mut w);
        let t = teile(&w)[0];
        assert!(t.menge < 9000 && t.menge >= 10, "{s}");
        assert!(s.contains(&format!("Mehr als {} kann ich dir nicht abgeben", t.menge)), "{s}");
        // Der Bot liefert nie unter seine Rücklage.
        let frei = handel::frei_verkauf(&w, 1, Bottyp::Haendler, Gut::Legierung);
        assert!(t.menge <= frei / 2 + 1, "{} > {}", t.menge, frei / 2);
    }

    /// Offene Handelsfragen bekommen Richtung und Preise, ein knapper Wunsch wird zum Auftrag (20.10.2026: „Ich möchte Erz
    /// handeln“ galt als Kauf, „Frieden und Pakt?“ wurde nur erklärt statt angeboten).
    #[test]
    fn offene_handelsfrage_fragt_nach_der_richtung_und_ein_kurzer_wunsch_bleibt_auftrag() {
        for (text, typ) in [("Wie handeln wir Erz?", Bottyp::Haendler), ("Ich möchte Erz handeln. Wie geht das?", Bottyp::Oekonom)] {
            let mut w = handelswelt();
            let name = w.spieler[1].name.clone();
            act(&mut w, 0, json!({"typ": "brief_senden", "kanal": "privat", "an": name, "betreff": "Handel", "text": text}));
            reagieren_mit(&mut w, 1, typ, &[1].into_iter().collect());
            let s = briefe_des_bots(&w).pop().unwrap();
            menschlich(&s);
            assert!(s.contains("Erz kaufen oder verkaufen") && s.contains("Preis je Einheit"), "{text}: {s}");
            assert!(w.orders.is_empty() && teile(&w).is_empty(), "eine Frage löst nichts aus: {text}");
        }
        let mut w = handelswelt();
        let name = w.spieler[1].name.clone();
        act(&mut w, 0, json!({"typ": "brief_senden", "kanal": "privat", "an": name, "betreff": "Frieden", "text": "Frieden und Pakt?"}));
        reagieren_mit(&mut w, 1, Bottyp::Haendler, &[1].into_iter().collect());
        assert!(w.vertraege.iter().any(|v| v.art == Vertragsart::Nichtangriffspakt && v.a == 1 && v.b == 0), "{:?}", briefe_des_bots(&w).pop());
    }

    #[test]
    fn fragen_werden_ehrlich_und_ohne_nebenwirkung_beantwortet() {
        let faelle: &[(&str, Bottyp, &str)] = &[
            ("Was kannst du eigentlich?", Bottyp::Haendler, "Ich kann dir auf Anfrage meinen Status nennen"),
            ("Was kannst du?", Bottyp::Raeuber, "Pakte und Allianzen schließe ich nicht"),
            ("Wie handeln wir miteinander?", Bottyp::Oekonom, "bediene sie, sobald sie im Markt steht"),
            ("Wie funktioniert der Markt?", Bottyp::Igel, "Handel & Markt"),
            ("Wie funktioniert eine Allianz?", Bottyp::Haendler, "internen Markt"),
            ("Was bringt eine Allianz?", Bottyp::Raeuber, "ich bleibe unabhängig"),
            ("Was ist ein Nichtangriffspakt?", Bottyp::Oekonom, "greifen wir uns nicht an"),
            ("Wie funktioniert ein Pakt?", Bottyp::Raeuber, "Pakte schließe ich nicht"),
            ("Greifst du an?", Bottyp::Raeuber, "Ich greife Reiche an"),
            ("Greifst du an?", Bottyp::Igel, "Ich greife nicht an"),
            ("Wie bekomme ich Hilfe?", Bottyp::Haendler, "Hilferuf"),
            ("Kolonisierst du auch?", Bottyp::Oekonom, "Kolonien neben meiner Heimatwelt"),
            ("Was machst du mit Monden?", Bottyp::Haendler, "Monde"),
            ("Baust du eine Dysonsphäre?", Bottyp::Oekonom, "Dysonsphäre"),
            ("Welche Preise nimmst du?", Bottyp::Haendler, "Eröffnungspreise"),
        ];
        for (frage, typ, erwartet) in faelle {
            let mut w = handelswelt();
            let name = w.spieler[1].name.clone();
            act(&mut w, 0, json!({"typ": "brief_senden", "kanal": "privat", "an": name, "betreff": "Frage", "text": frage}));
            reagieren_mit(&mut w, 1, *typ, &[1].into_iter().collect());
            let s = briefe_des_bots(&w).pop().unwrap();
            menschlich(&s);
            assert!(s.contains(erwartet), "{frage} ({typ:?}): {s}");
            assert!(w.orders.is_empty() && w.vertraege.is_empty() && w.handel.is_empty(), "Fragen lösen keine Aufträge aus: {frage}: {s}");
        }
    }

    #[test]
    fn einzelne_befehlswoerter_bleiben_auftraege() {
        // Status, Nichtangriffspakt und Hilfe verhalten sich wie vorher (nur Fragen werden erklärt).
        let mut w = handelswelt();
        letter(&mut w, "Nichtangriffspakt", None);
        let s = answer(&mut w);
        assert!(s.contains("Ja.") && w.vertraege.iter().any(|v| v.a == 1 && v.b == 0), "{s}");
        letter(&mut w, "Status", None);
        assert!(answer(&mut w).contains("Gebäudestufen"));
        letter(&mut w, "Hilfe", None);
        assert!(answer(&mut w).contains("Nein, noch kein ausführbarer Hilferuf"));
        // Ein bloßes „Ja“ ohne offenes Angebot ist kein Handelsbrief; „Einverstanden“ ohne Angebot bekommt eine klare Antwort.
        letter(&mut w, "Ja", None);
        assert!(!answer(&mut w).contains("kein offenes Angebot"));
        letter(&mut w, "Einverstanden", None);
        assert!(answer(&mut w).contains("kein offenes Angebot"));
    }

    #[test]
    fn bots_antworten_nie_auf_bots_und_handeln_nicht_im_kreis() {
        // Zwei Bots, ein Gegenüber: keine Briefschleife, kein Hin und Her am Markt.
        let mut w = handelswelt();
        let scripts: std::collections::BTreeSet<u16> = [1, 2].into_iter().collect();
        let name2 = w.spieler[2].name.clone();
        act(&mut w, 1, json!({"typ": "brief_senden", "kanal": "privat", "an": name2, "betreff": "Handel", "text": "Ich brauche 2000 Legierung"}));
        let n = w.nachrichten.len();
        for _ in 0..5 {
            reagieren_mit(&mut w, 2, Bottyp::Oekonom, &scripts);
            reagieren_mit(&mut w, 1, Bottyp::Haendler, &scripts);
        }
        assert_eq!(w.nachrichten.len(), n, "Bots antworten nicht auf Briefe anderer Bots");
        // Markt: alle vier Reiche handeln im Bot-Takt über viele Runden; kein Reich kauft und verkauft dieselbe Ware.
        let mut w = fixture();
        for sid in 0..4usize {
            let p = w.spieler[sid].heimat as usize;
            w.planeten[p].bestand.fill(0);
            w.planeten[p].bestand[Gut::Legierung.idx()] = if sid % 2 == 0 { 15_000 * M } else { 0 };
            w.planeten[p].bestand[Gut::Elektronik.idx()] = if sid % 2 == 1 { 15_000 * M } else { 0 };
            w.raten_neu(p);
        }
        let typen = [Bottyp::Haendler, Bottyp::Oekonom, Bottyp::Igel, Bottyp::Raeuber];
        let briefe_vorher = w.nachrichten.len();
        for runde in 0..60 {
            w.zeit += 900;
            for sid in 0..4u16 {
                markt_handeln(&mut w, sid, typen[sid as usize]);
                handel::aktiv_bedienen(&mut w, sid, typen[sid as usize]);
            }
            assert!(w.orders.len() <= 4 * 10, "Runde {runde}: das Buch wächst nicht ohne Grenze: {}", w.orders.len());
        }
        for g in Gut::ALLE {
            for sid in 0..4u16 {
                let kauft = w.handel.iter().any(|h| h.gut == g && h.kaeufer == sid);
                let verkauft = w.handel.iter().any(|h| h.gut == g && h.verkaeufer == sid);
                assert!(!(kauft && verkauft), "Reich {sid} kauft und verkauft {g}: kein Hin und Her");
            }
        }
        assert!(w.handel.len() < 200, "Handel bleibt begrenzt: {}", w.handel.len());
        assert_eq!(w.nachrichten.len(), briefe_vorher, "Bots schreiben sich am Markt keine Briefe");
    }

    #[test]
    fn ungefragte_orders_aller_reiche_werden_gleich_und_begrenzt_bedient() {
        // Mensch (0), Agent (2) und Bot (3) stellen dieselbe Kauforder zum Eröffnungspreis ein: der Bot bedient sie gleich,
        // aber höchstens zwei je Zug, und erst nach der Vorrangzeit.
        let mut w = handelswelt();
        let b = spanne(&w, Gut::Legierung, true);
        for sid in [0u16, 2, 3] {
            order(&mut w, sid, Gut::Legierung, "kauf", 500, b.offen);
        }
        handel::aktiv_bedienen(&mut w, 1, Bottyp::Haendler);
        assert!(w.handel.is_empty(), "Vorrangzeit für Gesprächspartner");
        w.zeit += 301;
        handel::aktiv_bedienen(&mut w, 1, Bottyp::Haendler);
        assert_eq!(w.handel.len(), 2, "höchstens zwei ungefragte Abschlüsse je Zug");
        handel::aktiv_bedienen(&mut w, 1, Bottyp::Haendler);
        assert_eq!(w.handel.len(), 3);
        let kaeufer: std::collections::BTreeSet<u16> = w.handel.iter().map(|h| h.kaeufer).collect();
        assert_eq!(kaeufer, [0u16, 2, 3].into_iter().collect(), "Mensch, Agent und Bot werden gleich behandelt");
        assert!(w.handel.iter().all(|h| h.verkaeufer == 1 && h.menge == 500 * M));
        assert!(w.nachrichten.is_empty(), "ungefragter Handel schreibt keine Briefe");
    }

    #[test]
    fn ungefragte_verkaufsorder_wird_bei_bedarf_zum_eroeffnungspreis_gekauft() {
        // Der Bot braucht Kristall: eine Verkaufsorder unter seinem Eröffnungspreis kauft er nach der Vorrangzeit, begrenzt;
        // eine darüber nicht – so viel zahlt er ungefragt nicht.
        let mut w = handelswelt();
        let b = spanne(&w, Gut::Kristall, false);
        let (teuer, billig) = (((b.offen + b.grenze) / 2.0 * 100.0).round() / 100.0, ((b.offen - 0.05) * 100.0).round() / 100.0);
        order(&mut w, 0, Gut::Kristall, "verkauf", 800, teuer);
        order(&mut w, 2, Gut::Kristall, "verkauf", 800, billig);
        let (k0, c1) = (bestand(&w, 0, Gut::Kristall), w.spieler[1].credits);
        w.zeit += 301;
        handel::aktiv_bedienen(&mut w, 1, Bottyp::Haendler);
        assert_eq!(w.handel.len(), 1);
        let h = &w.handel[0];
        assert_eq!((h.kaeufer, h.verkaeufer, h.gut, h.menge, h.preis), (1, 2, Gut::Kristall, 800 * M, milli(billig)));
        assert!(w.spieler[1].credits < c1, "der Bot bezahlt");
        assert_eq!(bestand(&w, 0, Gut::Kristall), k0, "die teurere Order des Menschen bleibt im Buch");
        assert!(w.orders.iter().any(|o| o.spieler == 0 && o.gut == Gut::Kristall), "unbedient");
    }

    #[test]
    fn zusagen_an_andere_gegenueber_werden_nicht_doppelt_versprochen() {
        let mut w = handelswelt();
        let p = w.spieler[1].heimat as usize;
        w.planeten[p].bestand[Gut::Legierung.idx()] = 6_000 * M;
        // Zweiter Gegenüber: Spieler 2 schreibt dem Bot ebenfalls.
        let name = w.spieler[1].name.clone();
        letter(&mut w, "Ich brauche 9000 Legierung", None);
        answer(&mut w);
        let erste = teile(&w)[0].menge;
        act(&mut w, 2, json!({"typ": "brief_senden", "kanal": "privat", "an": name, "betreff": "Handel", "text": "Ich brauche 9000 Legierung"}));
        hook(&mut w);
        let zweite = handel::gespraech(&w, 1, 2).teile.first().map(|x| x.0.menge).unwrap_or(0);
        let frei = handel::frei_verkauf(&w, 1, Bottyp::Haendler, Gut::Legierung);
        assert!(erste + zweite <= frei / 2 + 1, "{erste} + {zweite} > {}", frei / 2);
    }

    #[test]
    fn ohne_moeglichen_abschlussbrief_wird_nicht_bedient_und_danach_nur_einmal() {
        // Ist das Tageslimit der Bot-Briefe erschöpft, bliebe ein Abschluss ohne Zustandsbrief: dann wird nicht bedient,
        // sonst könnte derselbe Teil des Angebots später noch einmal geliefert werden.
        let mut w = handelswelt();
        letter(&mut w, "Ich brauche 1000 Legierung", None);
        answer(&mut w);
        let preis = teile(&w)[0].preis;
        order(&mut w, 0, Gut::Legierung, "kauf", 1000, preis);
        w.spieler[1].nachrichten_heute = w.regeln.agenten.nachrichten_je_tag - 1;
        hook(&mut w);
        assert!(w.handel.is_empty(), "ohne Briefplatz kein Abschluss");
        w.spieler[1].nachrichten_heute = 0;
        hook(&mut w);
        assert_eq!(w.handel.len(), 1, "am nächsten Tag wird bedient");
        hook(&mut w);
        handel::aktiv_bedienen(&mut w, 1, Bottyp::Haendler);
        assert_eq!(w.handel.iter().map(|h| h.menge).sum::<i64>(), 1000 * M, "genau einmal");
    }

    #[test]
    fn im_allianzchat_verhandelt_der_bot_nicht_oeffentlich() {
        // Angebote gehören ins private Postfach: der Allianzchat bekommt keinen Handelsbrief mit Angebotszeile.
        let mut w = handelswelt();
        alliance(&mut w);
        act(&mut w, 0, json!({"typ": "brief_senden", "kanal": "allianz", "an": "", "betreff": "Handel", "text": "Ich brauche 2000 Legierung", "antwort_auf": null}));
        reagieren_mit(&mut w, 1, Bottyp::Haendler, &[1].into_iter().collect());
        assert!(teile(&w).is_empty());
        assert!(!w.nachrichten.iter().any(|n| n.von == 1 && n.text.contains("Mein Angebot")));
    }

    #[test]
    fn briefe_bleiben_innerhalb_der_zeichengrenze_und_menschlich() {
        let spiel = |typ: Bottyp, texte: &[&str]| {
            let mut w = handelswelt();
            let name = w.spieler[1].name.clone();
            for t in texte {
                act(&mut w, 0, json!({"typ": "brief_senden", "kanal": "privat", "an": name, "betreff": "Handel", "text": t}));
                reagieren_mit(&mut w, 1, typ, &[1].into_iter().collect());
            }
            briefe_des_bots(&w)
        };
        for typ in [Bottyp::Haendler, Bottyp::Oekonom, Bottyp::Igel, Bottyp::Raeuber] {
            let alle = spiel(typ, &["Ich brauche 9000 Legierung", "Ich biete 0,2 je Legierung", "Ich verkaufe dir 5000 Kristall", "Tausche 8000 Kristall gegen 2000 Elektronik",
                "Was kannst du?", "Wollen wir handeln?", "Welche Preise hast du?", "Einverstanden", "Wie funktioniert eine Allianz?"]);
            assert!(alle.len() >= 9, "{typ:?}: jede Frage bekommt eine Antwort");
            for b in alle {
                menschlich(&format!("Guten Tag, Aster. Ich bin Mira. {b}"));
            }
        }
    }
}

/// 22.10.2026 (Karl: „die Bots können Schiffe bauen und mich angreifen, ich bin als Mensch über die UI wehrlos“):
/// Mitspieler haben Schonzeit nach Stärke (ohne Werft kein Ziel), Bots haben als Ziel Vorrang, und gegen einen
/// Mitspieler fliegt nur die nötige Flotte mit höchstens halber Beute und langem Anflug; wer selbst angreift, verliert
/// die Schonzeit.
#[test]
fn mitspieler_schonzeit_vorrang_und_faire_raubzuege() {
    let aufbau=|w:&mut Welt|{
        let p=w.spieler[0].heimat as usize;w.zeit=12*TAG;
        w.planeten[p].gebaeude[Gebaeude::Werft.idx()]=4;
        for (e,n) in [(Einheit::SchwererJaeger,80),(Einheit::KleinerTransporter,1000)] {w.planeten[p].einheiten[e.idx()]=n;}
        w.planeten[p].bestand.fill(100_000*M);
        for z in [1usize,2] {let ziel=w.spieler[z].heimat as usize;w.spieler[z].schutz_bis=0;w.planeten[ziel].gegruendet=0;
            w.planeten[ziel].gebaeude[Gebaeude::Werft.idx()]=1;w.planeten[ziel].einheiten[Einheit::Lasergeschuetz.idx()]=10;
            w.planeten[ziel].bestand.fill(if z==1 {300_000*M} else {200_000*M});}
    };
    let bericht=|w:&Welt,z:u16|{let ziel=w.spieler[z as usize].heimat as usize;Spionagebericht{zeit:w.zeit,ziel:w.planeten[ziel].koord,besitzer:z,
        bestand:w.planeten[ziel].bestand.to_vec(),schiffe:Some(w.planeten[ziel].einheiten[..SCHIFFE].to_vec()),
        verteidigung:Some(w.planeten[ziel].einheiten[SCHIFFE..].to_vec()),gebaeude:None,forschung:None}};
    let angriff=|w:&Welt|w.flotten.values().find(|f|f.besitzer==0&&f.mission==Mission::Angriff).cloned();
    let menschen:std::collections::BTreeSet<SpielerId>=[1].into_iter().collect();
    let ort1=|w:&Welt|w.planeten[w.spieler[1].heimat as usize].koord;

    // 1) Vorrang: Mitspieler 1 ist reicher, trotzdem trifft es den Bot 2.
    let mut w=fixture();aufbau(&mut w);
    for z in [1,2] {let b=bericht(&w,z);w.spieler[0].berichte.push(b);}
    let mut bot=Bot::neu(Bottyp::Raeuber,0);bot.opferschutz.aktiv=true; // Schonzeit seit 23.10.2026 standardmäßig aus
    mit_mitspielern(&menschen,||raeuber(&mut w,0,&mut bot));
    let f=angriff(&w).expect("Angriff auf den Bot");
    assert_eq!(f.ziel,w.planeten[w.spieler[2].heimat as usize].koord,"Bots haben als Ziel Vorrang");

    // 2) Schonzeit: Stufe III, aber ohne Werft – weder Angriff noch Spähflug.
    let mut w=fixture();aufbau(&mut w);let ziel=w.spieler[1].heimat as usize;
    w.planeten[ziel].gebaeude[Gebaeude::Werft.idx()]=0;
    let b=bericht(&w,1);w.spieler[0].berichte.push(b);
    let mut bot=Bot::neu(Bottyp::Raeuber,0);bot.opferschutz.aktiv=true; // Schonzeit seit 23.10.2026 standardmäßig aus
    mit_mitspielern(&menschen,||raeuber(&mut w,0,&mut bot));
    assert!(angriff(&w).is_none(),"Mitspieler ohne Werft ist kein Ziel");
    assert!(!w.flotten.values().any(|f|f.besitzer==0&&f.ziel==ort1(&w)),"und wird nicht ausgespäht");
    assert_eq!(mitspieler_schonzeit(&w,&bot.opferschutz,1),Some("noch ohne Werft"));
    // Gegenprobe Bot gegen Bot: dasselbe Ziel ohne Werft, aber kein Mitspieler – der Räuber greift an wie bisher.
    raeuber(&mut w,0,&mut bot);
    assert!(angriff(&w).is_some(),"Bot gegen Bot unverändert: {:?}",diagnose_zaehler());

    // 3) Fairer Raub: Mitspieler mit Werft. Vergleich mit demselben Ziel als Bot.
    let start=|mensch:bool|{let mut w=fixture();aufbau(&mut w);let b=bericht(&w,1);w.spieler[0].berichte.push(b);
        let mut bot=Bot::neu(Bottyp::Raeuber,0);bot.opferschutz.aktiv=true; // Schonzeit seit 23.10.2026 standardmäßig aus
        let leer=std::collections::BTreeSet::new();
        mit_mitspielern(if mensch {&menschen} else {&leer},||raeuber(&mut w,0,&mut bot));(angriff(&w).expect("Angriff"),w)};
    let (als_bot,_)=start(false);
    let (als_mensch,w)=start(true);
    let jaeger=|f:&Flotte|f.schiffe[Einheit::SchwererJaeger.idx()];
    let tr=|f:&Flotte|f.schiffe[Einheit::KleinerTransporter.idx()];
    assert_eq!(jaeger(&als_bot),80);
    assert!(jaeger(&als_mensch)<80,"nur die nötige Flotte: {}",jaeger(&als_mensch));
    assert!(tr(&als_mensch)*2<=tr(&als_bot)+1,"höchstens halbe Beute: {} gegen {}",tr(&als_mensch),tr(&als_bot));
    let p=w.spieler[0].heimat as usize;let mut fl=[0i64;SCHIFFE];fl.copy_from_slice(&als_mensch.schiffe);
    let langsam=w.flugplan(0,p,als_mensch.ziel,&fl,100).unwrap().dauer;
    assert!(als_mensch.flugdauer>=ANFLUG_MIN.min(langsam),"Anflug {} s, Ziel mindestens {} s",als_mensch.flugdauer,ANFLUG_MIN.min(langsam));
    assert!(als_mensch.flugdauer>=als_bot.flugdauer);

    // 4) Raubpause doppelt so lang, und wer selbst angreift, verliert die Schonzeit.
    let mut w=fixture();aufbau(&mut w);let ort=ort1(&w);
    w.kampfberichte.push(kampfbericht_fuer_test(&w,2,1,ort,w.zeit-30*STUNDE));
    let b=bericht(&w,1);w.spieler[0].berichte.push(b);
    let mut bot=Bot::neu(Bottyp::Raeuber,0);bot.opferschutz.aktiv=true; // Schonzeit seit 23.10.2026 standardmäßig aus
    assert!(raub_erlaubt(&w,&bot.opferschutz,0,1,ort),"als Bot nach 30 h wieder Ziel");
    mit_mitspielern(&menschen,||{assert!(!raub_erlaubt(&w,&bot.opferschutz,0,1,ort),"als Mitspieler 48 h Pause");});
    let mut w=fixture();aufbau(&mut w);let ziel=w.spieler[1].heimat as usize;w.spieler[1].stufe=2;
    mit_mitspielern(&menschen,||{
        assert_eq!(mitspieler_schonzeit(&w,&bot.opferschutz,1),Some("unter Zivilisationsstufe III"));
        let k=kampfbericht_fuer_test(&w,1,2,w.planeten[w.spieler[2].heimat as usize].koord,w.zeit-STUNDE);w.kampfberichte.push(k);
        assert_eq!(mitspieler_schonzeit(&w,&bot.opferschutz,1),None,"wer angreift, verliert die Schonzeit");
        let s=schutz_sicht(&w,&bot.opferschutz,1,true);assert_eq!(s["geschuetzt"],false);assert!(s["text"].as_str().unwrap().contains("selbst angegriffen"));
    });
    let _=ziel;bot.opferschutz.aktiv=false;
    mit_mitspielern(&menschen,||assert_eq!(mitspieler_schonzeit(&w,&bot.opferschutz,1),None,"ohne Opferschutz keine Regeln"));
}

fn kampfbericht_fuer_test(w:&Welt,angreifer:SpielerId,verteidiger:SpielerId,ort:Koord,zeit:SimZeit)->Kampfbericht {
    Kampfbericht{nr:w.kampfberichte.len() as u64+1,zeit,ort,mission:Mission::Angriff,angreifer,angreifer_gruppen:Vec::new(),verteidiger:vec![verteidiger],
        runden:1,sieger:Sieger::Angreifer,ang_vorher:[0;EINHEITEN],ang_nachher:[0;EINHEITEN],vert_vorher:[0;EINHEITEN],vert_nachher:[0;EINHEITEN],beute:[0;GUETER],truemmer:[0;2]}
}

/// 0.4.7 (Karl 24.10.2026): neue Welten haben zwei Kolonieplätze ab Start und je Stufe Terraforming einen weiteren. Bots
/// kolonisieren dort ohne Astrophysik (das Kolonieschiff braucht seit 25.10.2026 nur Impulsantrieb 2) und forschen für die dritte Kolonie
/// der Stufe V Terraforming statt Astrophysik; laufende Welten (Standard-Kernwerte) bleiben bei Astrophysik.
#[test]
fn bots_kolonisieren_neue_welten_ueber_freie_plaetze_und_terraforming() {
    let mut w=kern::quellen::neue_welt(&kern::quellen::RegelQuellen::eingebaut(),917,4).unwrap();
    assert_eq!(w.kolonie_forschung(),Forschung::Terraforming);
    let p=w.spieler[0].heimat as usize;let ks=w.planeten[p].koord;
    {let sp=&mut w.spieler[0];sp.ki=false;sp.stufe=3;sp.credits=1_000_000*M;
        sp.forschung[Forschung::Impulsantrieb.idx()]=2;}
    for (g,n) in [(Gebaeude::Werft,3),(Gebaeude::Raumhafen,2),(Gebaeude::Labor,3),(Gebaeude::Lager,12)] {w.planeten[p].gebaeude[g.idx()]=n;}
    for g in [Gut::Erz,Gut::Kristall,Gut::Deuterium,Gut::Nahrung,Gut::Legierung,Gut::Elektronik] {w.planeten[p].bestand[g.idx()]=200_000*M;}
    w.planeten[p].bevoelkerung=30_000*M;w.personen_abgleichen(p);
    w.raten_neu(p);
    assert_eq!((w.kolonien(0),w.kolonien_erlaubt(0)),(0,2),"zwei Plätze ohne Forschung");
    let mut gespaeht=BTreeMap::new();
    kolonisieren(&mut w,0,&mut gespaeht);
    assert!(w.planeten[p].fertigung[0].iter().any(|f|f.produkt==Produkt::Einheit(Einheit::Kolonieschiff)),
        "der Bot bestellt das Kolonieschiff ohne Astrophysik ({ks}): {:?}",diagnose_zaehler());
    // Stufe IV: die Stufe V verlangt drei Kolonien → Pflichtforschung Terraforming 1 (zwei Plätze sind frei).
    w.spieler[0].stufe=4;w.spieler[0].forschung[Forschung::Hyperraumantrieb.idx()]=1;
    assert_eq!(pflichtforschung(&w,0),Some(Forschung::Terraforming));
    w.spieler[0].forschung[Forschung::Terraforming.idx()]=1;
    assert_eq!(w.kolonien_erlaubt(0),3);
    assert_eq!(pflichtforschung(&w,0),None,"Terraforming 1 reicht für drei Kolonien");
    // Laufende Welten (Standard-Kernwerte): weiter Astrophysik, je Stufe eine Kolonie.
    let mut alt=fixture();alt.spieler[0].stufe=4;alt.spieler[0].forschung[Forschung::Astrophysik.idx()]=2;
    alt.planeten[alt.spieler[0].heimat as usize].gebaeude[Gebaeude::Labor.idx()]=6;
    assert_eq!(alt.kolonie_forschung(),Forschung::Astrophysik);
    assert_eq!(pflichtforschung(&alt,0),Some(Forschung::Astrophysik));
}
