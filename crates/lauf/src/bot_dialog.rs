//! Bounded script diplomacy: every promise reports a real command result.
//! No inferred target ownership, gifts, free ships, or simulated deliveries.
use super::*;

fn ausfuehren(w: &mut Welt, sid: SpielerId, a: Value) -> String {
    let (ok, message) = w.handeln(sid, Rolle::Alle, &a);
    format!("{} {message}", if ok { "Ja." } else { "Nein." })
}

fn worte(text: &str) -> Vec<String> {
    text.to_lowercase().split_whitespace().map(|s|s.trim_matches(|c:char|!c.is_alphanumeric()&&c!=','&&c!='.'&&c!=':').to_string()).collect()
}

pub(super) fn antwort(w:&mut Welt,sid:SpielerId,von:SpielerId,text:&str,typ:Bottyp)->String {
    let words=worte(text); let has=|s:&str|words.iter().any(|x|x==s);
    let pid=w.spieler[sid as usize].heimat as usize;
    let k=w.planeten[pid].koord;
    // Absage auf ein Angebot (Antwortleiste schickt „Absage: Nichtangriffspakt“): nie mit einem neuen Angebot antworten –
    // vorher las der Bot nur „nichtangriffspakt“ und bot sofort wieder an (Emulator 19.10.2026).
    if words.iter().any(|x|matches!(x.trim_end_matches(|c|c==':'||c==','||c=='.'),"absage"|"abgelehnt"|"ablehnen")) {
        return "Verstanden, deine Absage ist angekommen. Ich biete dir vorerst nichts Neues an; laufende Flüge und Geschäfte bleiben bestehen.".into();
    }
    // Explicit refusal/cancellation must never be interpreted as a trade command.
    if has("nicht")||has("nein")||has("stopp")||has("abbrechen") {
        return "Verstanden. Aus dieser Nachricht starte ich keinen Auftrag. Bereits laufende Flüge und abgeschlossene Geschäfte bleiben bestehen.".into();
    }
    // Fragen (Wie handeln wir? Was kannst du? Wie funktioniert eine Allianz?) und Handel per Brief (Angebot, Gegenvorschlag,
    // Tausch, Einverstanden) vor den Einzelbefehlen: beides verspricht nur, was der Bot tut (siehe `fragen`, `handel`).
    // Der alte Befehl „Kaufe/Verkaufe <Menge> <Ware> für <Preis>“ ohne ich/mir/dir und „Kaufe Angebot N“ bleiben unten.
    if let Some(r)=fragen::antwort(w,sid,typ,text) {return r;}
    if !handel::alter_befehl(&words)&&!(has("kaufe")&&has("angebot")) {
        if let Some(r)=handel::antwort(w,sid,von,text,typ) {return r;}
    }
    if has("status")||has("strategie")||has("kolonien") {
        let sp=&w.spieler[sid as usize];
        let role=match typ {Bottyp::Haendler=>"Handel, Raumhäfen und Lager",Bottyp::Igel=>"Bunker, Garnisonen und Verteidigung",Bottyp::Raeuber=>"Werften, Treibstoff und Angriffsflotten",_=>"Verarbeitung, Versorgung und Forschung"};
        let planets=sp.planeten.iter().map(|id|{let p=&w.planeten[*id as usize];let next=waehle_bau(w,sid,*id as usize,typ).map(|g|g.name()).unwrap_or("Versorgung abwarten");format!("{}: {} Gebäudestufen, nächstes Bauziel {}",p.koord,p.gebaeude.iter().map(|n|*n as u32).sum::<u32>(),next)}).collect::<Vec<_>>().join("; ");
        return format!("Mein Schwerpunkt: {role}. Zivilisation {}, {} Kolonien, {}/{} Flotten unterwegs. {planets}. Meine Ausbauziele bleiben an verfügbare Rohstoffe, Energie und Personal gebunden.",sp.stufe,w.kolonien(sid),w.flotten.values().filter(|f|f.besitzer==sid).count(),w.flottenplaetze(sid));
    }
    if has("hilfe")||has("entsatz") {
        if let Some(id)=w.kommunikation.hilfe.iter().find(|h|h.von==von&&h.status=="offen"&&Some(h.allianz)==w.spieler[sid as usize].allianz).map(|h|h.id) {
            return hilferuf(w,sid,id,typ);
        }
        return "Nein, noch kein ausführbarer Hilferuf. Erstelle in unserer Allianz einen Hilferuf für deinen Planeten. Ich prüfe dann verfügbare Kampfschiffe, Treibstoff und Flugplätze und nenne die tatsächliche Ankunft.".into();
    }
    if has("frieden")||has("nichtangriffspakt")||has("pakt") {
        if typ==Bottyp::Raeuber{return "Nein. Als Räuber schließe ich keinen Nichtangriffspakt; ein bezahltes Handelsgeschäft ist möglich.".into();}
        if let Some(v)=w.vertraege.iter().find(|v|v.art==Vertragsart::Nichtangriffspakt&&((v.a==sid&&v.b==von)||(v.a==von&&v.b==sid))&&matches!(v.status,Vertragsstatus::Aktiv|Vertragsstatus::Angeboten)).cloned(){
            if v.status==Vertragsstatus::Aktiv{return "Ja. Unser Nichtangriffspakt ist bereits aktiv.".into();}
            if v.b==sid{return ausfuehren(w,sid,json!({"typ":"vertrag_annehmen","vertrag":v.id}));}
            return "Ja. Mein Vertragsangebot liegt bereits in deiner Diplomatie; deine Annahme fehlt noch.".into();
        }
        return ausfuehren(w,sid,json!({"typ":"vertrag_anbieten","partner":w.spieler[von as usize].name,"art":"nichtangriffspakt","kaution":100}));
    }
    if has("kaufe") && has("angebot") {
        let id=words.windows(2).find(|a|a[0]=="angebot").and_then(|a|a[1].parse::<u64>().ok());
        let Some(o)=id.and_then(|id|w.kommunikation.angebote.iter().find(|o|o.id==id&&o.von==von&&o.status=="offen"&&Some(o.allianz)==w.spieler[sid as usize].allianz)).cloned() else{return "Nein. Nenne ein offenes eigenes Allianzangebot: Kaufe Angebot 12. Fremde oder erledigte Angebote kaufe ich nicht auf Zuruf.".into();};
        // Höchstens zum Marktwert (Kaufgrenze im Briefhandel, `handel::band`).
        let fair=milli(handel::band(w,o.gut,typ,false).grenze).max(1);
        let value=o.menge as i128*o.preis as i128/M as i128;
        if o.preis>fair||value>w.spieler[sid as usize].credits as i128/4 {return "Nein. Preis oder Gesamtsumme überschreiten mein Handelsbudget; mindestens drei Viertel meiner Credits bleiben als Reserve.".into();}
        return ausfuehren(w,sid,json!({"typ":"intern_kaufen","angebot":o.id,"planet":k}));
    }
    // An explicit verb + quantity + good + unit price makes the direction unambiguous.
    // e.g. "Verkaufe 100 Erz für 1" asks the bot to reserve its own goods for sale.
    if has("kaufe")||has("verkaufe") {
        let buy=has("kaufe");
        if buy&&has("verkaufe"){return "Nein. Bitte genau eine Richtung je Nachricht: Kaufe oder Verkaufe, Menge, Ware und Preis je Einheit.".into();}
        let verb=if buy{"kaufe"}else{"verkaufe"};
        let parsed=words.windows(3).find(|a|a[0]==verb).and_then(|a|Some((a[1].parse::<i64>().ok()?,Gut::ALLE.into_iter().find(|g|g.name()==a[2])?)));
        let price=words.windows(2).find(|a|a[0]=="für"||a[0]=="fuer"||a[0]=="zu").and_then(|a|a[1].replace(',',".").parse::<f64>().ok());
        let (Some((amount,good)),Some(price))=(parsed,price) else{return "Nein, die Angaben reichen noch nicht. Schreibe zum Beispiel: Verkaufe 100 Erz für 1. Das bedeutet: Ich biete 100 Erz zu 1 Credit je Einheit an. Oder: Kaufe Angebot 12 aus unserem Allianzmarkt.".into();};
        // Preise am Marktwert (Soforthandel-Kurs oder Wertungsgewicht): kauft der Bot, nie darüber, verkauft er, nie
        // darunter – dieselben Grenzen wie im Briefhandel (`handel::band`).
        let wert=marktwert(w,good);
        let grenze=handel::band(w,good,typ,!buy).grenze;
        if !(1..=10_000).contains(&amount)||!price.is_finite()||price<=0.0||price>wert*3.0||price<wert/10.0 {
            return "Nein. Ich handle 1 bis 10.000 Einheiten zu einem fairen Stückpreis; dieser Auftrag liegt außerhalb meiner Grenzen.".into();
        }
        if buy && (amount as f64*price>ganz(w.spieler[sid as usize].credits) as f64/4.0 || price>grenze+1e-9) {return "Nein. Dieser Kauf überschreitet mein Preislimit oder mein freies Handelsbudget.".into();}
        if !buy && price<grenze-1e-9 {return format!("Nein. Unter {} Credits je Einheit gebe ich {} nicht ab.",handel::p(grenze),handel::ware(good));}
        if !buy && amount*M>verfuegbar(w,sid,pid,typ)[good.idx()]/2 {return "Nein. Nach meinen Bau- und Versorgungsrücklagen reicht der freie Warenbestand für diese Menge nicht.".into();}
        if w.orders.iter().any(|o|o.spieler==sid&&o.gut==good) || w.kommunikation.angebote.iter().any(|o|o.von==sid&&o.gut==good&&o.status=="offen") {return "Nein. Für diese Ware habe ich bereits ein offenes Angebot. Bitte nutze zuerst dieses.".into();}
        let internal=!buy && w.spieler[sid as usize].allianz.is_some() && w.spieler[sid as usize].allianz==w.spieler[von as usize].allianz;
        let action=if internal {json!({"typ":"intern_anbieten","planet":k,"gut":good.name(),"menge":amount,"preis":price})} else {json!({"typ":"markt_order","planet":k,"gut":good.name(),"seite":if buy{"kauf"}else{"verkauf"},"menge":amount,"preis":price})};
        let result=ausfuehren(w,sid,action);
        if result.starts_with("Nein."){return result;}
        return format!("{result} {}",if internal {"Die Ware ist im Allianzmarkt reserviert; erst dein Kauf löst Bezahlung und Lieferung aus."}else{"Im öffentlichen Markt gilt die Preisreihenfolge. Offene Restmengen sind noch nicht geliefert."});
    }
    if has("allianz")||has("bündnis") {
        if typ==Bottyp::Raeuber{return "Nein. Ich bleibe unabhängig.".into();}
        // Leitet der Bot eine Allianz mit Platz, lädt er den Fragenden ein (Annahme über die Antwortleiste).
        if let Some(aid)=w.spieler[sid as usize].allianz {
            let al=w.allianzen.iter().find(|a|a.id==aid).cloned();
            if let Some(al)=al.filter(|a|w.allianz_leitung(sid,aid)&&a.mitglieder.len()+a.eingeladen.len()<w.bot_allianz_max()) {
                if w.spieler[von as usize].allianz.is_some(){return format!("Nein. Du bist schon in einer Allianz; verlasse sie zuerst, dann lade ich dich in '{}' ein.",al.name);}
                if super::allianz_bot::beziehung(w,sid,von)<0 {return "Nein. Nach unseren bisherigen Kämpfen oder einem abgelehnten Pakt lade ich dich nicht ein.".into();}
                let text=format!("Einladung in die Allianz '{}' ({} Mitglieder) auf deine Anfrage. Annehmen über die Antwortleiste.",al.name,al.mitglieder.len());
                let r=ausfuehren(w,sid,json!({"typ":"diplomatie_anfrage","partner":w.spieler[von as usize].name,"art":"allianz","text":text}));
                if r.starts_with("Ja."){return format!("Ja. Ich habe dich in die Allianz '{}' eingeladen; nimm die Einladung über die Antwortleiste an.",al.name);}
                return r;
            }
            return "Nein, ich wechsle meine bestehende Allianz nicht auf Zuruf. Wir können Handel oder einen Nichtangriffspakt vereinbaren.".into();
        }
        if let Some(name)=w.allianzen.iter().find(|a|a.eingeladen.contains(&sid)&&a.mitglieder.contains(&von)).map(|a|a.name.clone()) {
            return ausfuehren(w,sid,json!({"typ":"allianz_beitreten","allianz":name}));
        }
        return "Ja, ich bin grundsätzlich bereit. Lade mich über die Allianzverwaltung ein; erst die Einladung erlaubt mir den Beitritt.".into();
    }
    format!("{} Für verbindliche Aufträge kannst du schreiben: Status; Nichtangriffspakt; Ich brauche 2000 Erz; Ich verkaufe dir 500 Legierung; Tausche 3000 Kristall gegen 2000 Erz; Verkaufe 100 Erz für 1; Kaufe Angebot 12; Hilfe. Ich bestätige ausgeführte Aufträge oder nenne den Ablehnungsgrund.",antwort_text(text,typ))
}

/// Sofort nach jedem Befehl: hat ein Gesprächspartner die zum Angebot passende Order eingestellt, bedient der Bot sie
/// (Aufruf aus `reagieren_mit`).
pub(super) fn markt_bedienen(w:&mut Welt,sid:SpielerId,typ:Bottyp,scripts:&std::collections::BTreeSet<SpielerId>) {
    handel::markt_bedienen(w,sid,typ,scripts);
}

pub(super) fn hilferuf(w:&mut Welt,sid:SpielerId,id:u64,typ:Bottyp)->String {
    if typ==Bottyp::Raeuber||typ==Bottyp::Wehrlos{return "Nein. Meine Strategie sieht keine Entsatzflotten für Verbündete vor.".into();}
    let Some(h)=w.kommunikation.hilfe.iter().find(|h|h.id==id&&h.status=="offen"&&Some(h.allianz)==w.spieler[sid as usize].allianz&&h.von!=sid).cloned() else{return "Nein. Kein zugänglicher offener Hilferuf.".into();};
    if let Some(f)=w.flotten.values().find(|f|f.besitzer==sid&&f.ziel==h.planet&&f.mission==Mission::Halten){return format!("Ja. Meine Entsatzflotte {} ist bereits beauftragt, Ankunft bei Spielsekunde {}.",f.id,f.ankunft);}
    let p=w.spieler[sid as usize].heimat as usize;
    let mut ships=serde_json::Map::new();
    for e in Einheit::ALLE[..SCHIFFE].iter().copied().filter(|e|!matches!(e,Einheit::Spionagesonde|Einheit::Kolonieschiff|Einheit::GrossesKolonieschiff)&&w.regeln.einh(*e).angriff>0&&!matches!(e,Einheit::KleinerTransporter|Einheit::GrosserTransporter|Einheit::Truppentransporter|Einheit::Recycler|Einheit::Bergbauschiff|Einheit::Handelsfrachter|Einheit::Schwerlastfrachter|Einheit::Schnellfrachter|Einheit::Pathfinder|Einheit::Bergungsschiff)) {
        let n=(w.planeten[p].einheiten[e.idx()]/2).min(20);if n>0{ships.insert(e.name().into(),json!(n));}
    }
    if ships.is_empty(){return "Nein. Mir fehlen freie Kampfschiffe; ich gebe keine Hilfezusage ohne Flotte.".into();}
    if w.aufklaerungsregeln()&&w.planetenwissen(sid,h.planet)["bekannt"]!=true {
        if w.flotten.values().any(|f|f.besitzer==sid&&f.zustand==Flottenzustand::Hinflug&&matches!(f.mission,Mission::Spionage|Mission::SystemErkunden)&&f.ziel.sektor==h.planet.sektor&&f.ziel.system==h.planet.system){return "Noch keine Entsatz-Zusage: Meine Sonde klärt das Ziel bereits auf.".into();}
        if w.planeten[p].einheiten[Einheit::Spionagesonde.idx()]==0 {
            if sonden_geplant(w,sid,p)==0&&bezahlbar(&w.bestand_jetzt(p),&w.regeln.kosten_einheit(Einheit::Spionagesonde,w.spieler[sid as usize].volk)) {
                let result=ausfuehren(w,sid,json!({"typ":"fertigen","planet":w.planeten[p].koord,"einheit":"spionagesonde","anzahl":1}));
                return format!("Noch keine Entsatz-Zusage: Das Ziel braucht Aufklärung. Sondenauftrag: {result}");
            }
            return "Nein. Das Ziel braucht Aufklärung; mir fehlt eine verfügbare Sonde.".into();
        }
        let mission=if w.system_erfasst(sid,h.planet){"spionage"}else{"system_erkunden"};
        let Some(auftrag)=sondenauftrag(w,sid,p,h.planet,mission,1) else{return "Nein. Für die Aufklärung fehlen ein freier Flottenplatz oder Treibstoff für Hin- und Rückflug.".into();};
        let result=ausfuehren(w,sid,auftrag);
        return format!("Noch keine Entsatz-Zusage. Aufklärung: {result}");
    }
    // Ohne freien Flottenplatz ehrlich absagen, statt den Flug vom Kern ablehnen zu lassen (bot_lage 20.10.2026: 179-mal).
    if w.flotten.values().filter(|f|f.besitzer==sid).count()>=w.flottenplaetze(sid) {
        return "Nein. Alle meine Flottenplätze sind gerade belegt; sobald eine Flotte zurück ist, prüfe ich den Hilferuf erneut.".into();
    }
    let fid=w.naechste_flotte;
    let result=ausfuehren(w,sid,json!({"typ":"flotte_senden","start":w.planeten[p].koord,"ziel":h.planet,"mission":"halten","haltedauer_stunden":6,"schiffe":ships}));
    if let Some(f)=w.flotten.get(&fid){let arrival=f.ankunft;tu(w,sid,json!({"typ":"allianz_hilfe_status","hilfe":id,"erledigt":false}));return format!("Ja. Entsatzflotte {fid} ist gestartet; Ankunft bei Spielsekunde {arrival}, danach sechs Spielstunden Verteidigung. Die Hälfte meiner Schiffe bleibt zurück.");}
    result
}

/// Handel im Bot-Zug (Gesprächspartner zuerst, dann ungefragte Orders aller Reiche) und Hilfe für Verbündete.
pub(super) fn helfen(w:&mut Welt,sid:SpielerId,typ:Bottyp) {
    handel::aktiv_bedienen(w,sid,typ);
    if typ==Bottyp::Raeuber||typ==Bottyp::Wehrlos{return;}
    let pending=w.kommunikation.hilfe.iter().filter(|h|h.von!=sid&&h.status=="offen"&&Some(h.allianz)==w.spieler[sid as usize].allianz&&!h.helfer.contains(&sid)).map(|h|h.id).take(1).collect::<Vec<_>>();
    for id in pending{hilferuf(w,sid,id,typ);}
}

// =====================================================================================================================
// Handelsaushandlung per Brief (Karl 20.10.2026: „Handel aktiv betreiben … wenn ich als Mensch mit den Bots handeln
// möchte, muss das besser durchdacht sein, dass auf eine Aushandlung dieser Bot auch was schickt und das echt
// funktioniert, über die UI bestätigt“).
//
// Ablauf (jede Zusage ist ein echter Kernbefehl, nichts wird vorab geliefert):
//  1. Der Gegenüber schreibt frei, was er braucht, anbietet oder tauscht („Ich brauche 2000 Erz“, „Ich verkaufe dir
//     500 Legierung“, „Tausche 3000 Kristall gegen 2000 Erz“, „Ich biete 0,15 je Erz“, „Einverstanden“).
//  2. Der Bot antwortet mit einem konkreten Angebot nach Bedarf und Überschuss (`frei_verkauf`, `bedarf_kauf`): Menge,
//     Preis je Einheit, Summe, bei Gegengeboten Gegenvorschlag und letzte Grenze. Das Angebot steht maschinenlesbar in der
//     Zeile „Mein Angebot: ich verkaufe 2000 Erz zu 1,15 Credits je Einheit.“ – der Brief selbst ist der gesamte
//     Gesprächszustand (kein neues Feld in Welt oder Regeln, übersteht Speichern und Neustart).
//  3. Der Gegenüber stellt im Markt die passende Order ein (Handel & Markt). Sobald sie im Buch steht, stellt der Bot die
//     Gegenorder ein: Antwort „Einverstanden“, der Hook `markt_bedienen` nach jedem Befehl, spätestens der nächste
//     Bot-Zug. Der Kern gleicht ab, bucht Credits, liefert die Ware. Ein Brief bestätigt den Abschluss und führt den Rest
//     des Angebots weiter („Angebot erledigt: Erz verkauft.“ beendet es) – so wird nie doppelt geliefert.
//  4. Ungefragte Orders aller Reiche (Menschen, Agenten, Bots gleich) bedient der Bot im eigenen Zug, begrenzt und zum
//     Eröffnungspreis (`aktiv_bedienen`); ein Gesprächspartner hat 5 Spielminuten Vorrang.
// Grenzen: eigene Rücklagen (`ruecklage`), halber freier Bestand je Angebot, ein Viertel der Credits, faire Preisspanne
// um den Marktwert je Einheit (Soforthandel-Kurs oder Wertungsgewicht, wie die Marktorders von `markt_handeln`).
// =====================================================================================================================
pub(crate) mod handel {
    use super::*;

    /// Gültigkeit eines Angebots (Spielsekunden): ein Spieltag.
    const GUELTIG: i64 = TAG;
    /// So lange hat ein Gesprächspartner mit offenem Angebot Vorrang vor ungefragten Käufern und Verkäufern.
    const VORRANG: i64 = 300;
    const MAX_LOS: i64 = 10_000;
    const MIN_LOS: i64 = 10;
    /// Höchstens so viele ungefragte Abschlüsse je Bot-Zug.
    const MAX_AKTIV: usize = 2;
    /// Ungefragt höchstens so viele Einheiten je Gegenüber, Ware und Spieltag.
    const MAX_TAG: i64 = 5_000;
    /// Brieflänge: das Spiel erlaubt 1200 Zeichen, die Anrede („Guten Tag, X. Ich bin Y.“) kommt noch davor.
    const BRIEF_MAX: usize = 1_000;

    // ------------------------------------------------------------------------------------------------------------
    // Lesen: Zahlen, Wörter, Absicht
    // ------------------------------------------------------------------------------------------------------------

    #[derive(Clone, Copy, Debug, PartialEq)]
    pub(crate) struct Zahl {
        /// Lesart mit Dezimalpunkt oder -komma („2.000“ = 2).
        pub dez: f64,
        /// Lesart mit Tausenderpunkt („2.000“ = 2000).
        pub tau: f64,
    }

    #[derive(Clone, Debug)]
    pub(crate) struct Tok {
        pub w: String,
        pub zahl: Option<Zahl>,
    }

    pub(crate) fn norm(text: &str) -> String {
        text.to_lowercase().replace('ä', "ae").replace('ö', "oe").replace('ü', "ue").replace('ß', "ss")
    }

    fn zahl_lesen(s: &str) -> Option<Zahl> {
        let (s, faktor) = match s.strip_suffix('k') {
            Some(r) if r.chars().next().is_some_and(|c| c.is_ascii_digit()) => (r, 1000.0),
            _ => (s, 1.0),
        };
        let erste = s.chars().next()?;
        let letzte = s.chars().last()?;
        if !erste.is_ascii_digit() || !letzte.is_ascii_digit() || !s.chars().all(|c| c.is_ascii_digit() || c == ',' || c == '.') {
            return None;
        }
        let punkte = s.matches('.').count();
        let kommas = s.matches(',').count();
        let (dez, tau): (f64, f64) = if punkte > 0 && kommas > 0 {
            let deutsch = s.rfind(',')? > s.rfind('.')?;
            let t = if deutsch { s.replace('.', "").replace(',', ".") } else { s.replace(',', "") };
            let v: f64 = t.parse().ok()?;
            (v, v)
        } else if kommas > 1 {
            let v: f64 = s.replace(',', "").parse().ok()?;
            (v, v)
        } else if kommas == 1 {
            let v: f64 = s.replace(',', ".").parse().ok()?;
            (v, v)
        } else if punkte > 1 {
            let v: f64 = s.replace('.', "").parse().ok()?;
            (v, v)
        } else if punkte == 1 {
            let (a, b) = s.split_once('.')?;
            let dez: f64 = s.parse().ok()?;
            if b.len() == 3 && a.len() <= 3 && a != "0" { (dez, format!("{a}{b}").parse().ok()?) } else { (dez, dez) }
        } else {
            let v: f64 = s.parse().ok()?;
            (v, v)
        };
        if !dez.is_finite() || !tau.is_finite() {
            return None;
        }
        Some(Zahl { dez: dez * faktor, tau: tau * faktor })
    }

    /// Kleinbuchstaben, ohne Satzzeichen (nur Komma und Punkt zwischen Ziffern bleiben), Umlaute aufgelöst.
    pub(crate) fn tokens(text: &str) -> Vec<Tok> {
        let cs: Vec<char> = norm(text).chars().collect();
        let mut s = String::with_capacity(cs.len());
        for (i, &c) in cs.iter().enumerate() {
            if c.is_alphanumeric() {
                s.push(c);
            } else if c == '€' {
                s.push_str(" euro ");
            } else if (c == ',' || c == '.') && i > 0 && cs[i - 1].is_ascii_digit() && cs.get(i + 1).is_some_and(|d| d.is_ascii_digit()) {
                s.push(c);
            } else {
                s.push(' ');
            }
        }
        let mut out: Vec<Tok> = Vec::new();
        for roh in s.split_whitespace() {
            if matches!(roh, "tausend" | "tsd") {
                if let Some(Tok { zahl: Some(z), .. }) = out.last_mut() {
                    z.dez *= 1000.0;
                    z.tau *= 1000.0;
                    continue;
                }
            }
            out.push(Tok { w: roh.to_string(), zahl: zahl_lesen(roh) });
        }
        out
    }

    fn wort_at(t: &[Tok], i: usize) -> &str {
        t.get(i).map(|x| x.w.as_str()).unwrap_or("")
    }

    pub(crate) fn gut_aus_wort(w: &str) -> Option<Gut> {
        const ALIAS: &[(&str, Gut)] = &[
            ("deut", Gut::Deuterium),
            ("xeno", Gut::Xenokristall),
            ("konsum", Gut::Konsumgut),
            ("konsumgueter", Gut::Konsumgut),
            ("konsumguetern", Gut::Konsumgut),
            ("nahrungsmittel", Gut::Nahrung),
            ("lebensmittel", Gut::Nahrung),
        ];
        if let Some((_, g)) = ALIAS.iter().find(|(a, _)| *a == w) {
            return Some(*g);
        }
        Gut::ALLE.into_iter().find(|g| {
            let n = g.name();
            w.starts_with(n) && w.len() <= n.len() + 3
        })
    }

    /// Wie das Gut im Brief heißt (so liest es `gut_aus_wort` wieder ein).
    pub(crate) fn ware(g: Gut) -> &'static str {
        match g {
            Gut::Erz => "Erz",
            Gut::Kristall => "Kristall",
            Gut::Deuterium => "Deuterium",
            Gut::Nahrung => "Nahrung",
            Gut::Legierung => "Legierung",
            Gut::Elektronik => "Elektronik",
            Gut::Konsumgut => "Konsumgüter",
            Gut::Xenokristall => "Xenokristall",
            Gut::Antriebskern => "Antriebskerne",
            Gut::Habitatmodul => "Habitatmodule",
        }
    }

    const FUELL: &[&str] = &["einheiten", "einheit", "stueck", "stk", "x", "mal", "tonnen"];
    const CREDIT: &[&str] = &["credits", "credit", "cr", "euro", "taler", "muenzen"];
    const PREIS_VOR: &[&str] = &["zu", "fuer", "a", "à", "bei", "um", "ab", "bis", "mit", "preis", "kurs", "je", "pro"];
    const GESAMT: &[&str] = &["insgesamt", "gesamt", "zusammen", "komplett", "total", "gesamtpreis"];
    const ZUSTIMMUNG: &[&str] = &[
        "einverstanden", "deal", "abgemacht", "akzeptiert", "akzeptiere", "okay", "ok", "gemacht", "passt", "topp", "gebongt",
        "angenommen", "annehmen", "einig", "handschlag", "genehmigt",
    ];
    const TAUSCH: &[&str] = &["tausch", "tausche", "tauschen", "tauscht", "tauschgeschaeft", "tauschhandel", "tauschangebot"];
    const HANDELSWORT: &[&str] = &[
        "handel", "handeln", "handelt", "geschaeft", "geschaefte", "handelsangebot", "handelspartner", "ware", "waren", "rohstoffe",
        "rohstoff", "handelsgut", "handelsguete",
    ];

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub(crate) enum Hinweis {
        Offen,
        Stueck,
        Gesamt,
    }

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub(crate) enum Art {
        /// Der Gegenüber will kaufen, der Bot verkauft.
        Kauf,
        /// Der Gegenüber will verkaufen, der Bot kauft.
        Verkauf,
        Tausch,
        Zustimmung,
        /// Allgemeine Handelsfrage oder Preisfrage ohne Ware.
        Uebersicht,
        /// Ware genannt, aber nicht, ob gekauft oder verkauft werden soll.
        Unklar,
    }

    #[derive(Clone, Debug)]
    pub(crate) struct Absicht {
        pub art: Art,
        pub gut: Option<Gut>,
        pub menge: Option<i64>,
        pub preis: Option<(Zahl, Hinweis)>,
        /// Tausch: was der Gegenüber bekommen möchte.
        pub gegen: Option<(Gut, Option<i64>)>,
        pub preisfrage: bool,
        /// Ausdrückliche Zustimmung („Einverstanden“, „Deal“) statt eines bloßen „Ja“ oder „Ok“.
        pub stark: bool,
    }

    fn menge_aus(z: Zahl) -> Option<i64> {
        let m = z.tau;
        (m >= 1.0 && (m - m.round()).abs() < 1e-9 && m <= 1e9).then(|| m.round() as i64)
    }

    /// Liest eine Handelsabsicht aus dem Brief (Betreff und Text). `None`, wenn der Brief kein Handelsbrief ist.
    pub(crate) fn lies(t: &[Tok]) -> Option<Absicht> {
        let n = t.len();
        let wort = |i: usize| wort_at(t, i);
        let hat = |ws: &[&str]| t.iter().any(|x| ws.contains(&x.w.as_str()));
        // Waren mit der Menge, die direkt davor steht.
        let mut posten: Vec<(usize, Gut, Option<i64>)> = Vec::new();
        let mut belegt = vec![false; n];
        for i in 0..n {
            if let Some(g) = gut_aus_wort(&t[i].w) {
                let mut j = i;
                while j > 0 && FUELL.contains(&wort(j - 1)) {
                    j -= 1;
                }
                let mut menge = None;
                if j > 0 {
                    if let Some(m) = t[j - 1].zahl.and_then(menge_aus) {
                        menge = Some(m);
                        belegt[j - 1] = true;
                    }
                }
                posten.push((i, g, menge));
            }
        }
        // Übrige Zahlen: Preis (nach „zu“, „für“, vor „Credits“ oder „je“) oder lose.
        let mut preis: Option<(Zahl, Hinweis)> = None;
        let mut lose: Vec<Zahl> = Vec::new();
        for i in 0..n {
            if belegt[i] {
                continue;
            }
            let Some(z) = t[i].zahl else { continue };
            let nach = wort(i + 1);
            let vor = if i > 0 { wort(i - 1) } else { "" };
            let gesamt = (i.saturating_sub(3)..(i + 4).min(n)).any(|k| GESAMT.contains(&wort(k)));
            let stueck = matches!(nach, "je" | "pro" | "stueck" | "einheit") || matches!(wort(i + 2), "einheit" | "stueck");
            let hinweis = if gesamt { Hinweis::Gesamt } else if stueck { Hinweis::Stueck } else { Hinweis::Offen };
            if CREDIT.contains(&nach) || matches!(nach, "je" | "pro") || PREIS_VOR.contains(&vor) {
                if preis.is_none() {
                    preis = Some((z, hinweis));
                }
            } else {
                lose.push(z);
            }
        }
        if preis.is_none() && !lose.is_empty() {
            // „Erz 2000“: eine lose Zahl ist die Menge der ersten Ware ohne Menge.
            if let Some(p) = posten.iter_mut().find(|p| p.2.is_none()) {
                if let Some(m) = lose.iter().find_map(|z| menge_aus(*z)) {
                    p.2 = Some(m);
                }
            }
        }
        let ich_wort = |w: &str| matches!(w, "ich" | "wir");
        let du_wort = |w: &str| matches!(w, "du" | "ihr");
        let ich = t.iter().any(|x| ich_wort(&x.w));
        let dativ_dir = hat(&["dir", "euch"]);
        let subjekt_vor = |i: usize| -> Option<bool> {
            (0..i).rev().find_map(|k| {
                let x = t[k].w.as_str();
                if ich_wort(x) {
                    Some(true)
                } else if du_wort(x) {
                    Some(false)
                } else {
                    None
                }
            })
        };
        let fuer_danach = |i: usize| t.iter().skip(i).take(5).any(|x| x.w == "fuer");
        let mut kauft = 0i32; // Der Gegenüber kauft, der Bot verkauft.
        let mut verkauft = 0i32; // Der Gegenüber verkauft, der Bot kauft.
        for i in 0..n {
            let subj = subjekt_vor(i);
            match t[i].w.as_str() {
                "brauche" | "brauchen" | "benoetige" | "benoetigen" | "suche" | "suchen" | "fehlt" | "fehlen" | "vermisse" | "bedarf" => kauft += 2,
                "brauchst" | "benoetigst" | "suchst" | "braucht" | "benoetigt" => {
                    if subj == Some(true) {
                        kauft += 2
                    } else {
                        verkauft += 2
                    }
                }
                "kaufe" | "kauf" => {
                    if ich {
                        kauft += 3
                    } else {
                        verkauft += 3
                    }
                }
                "kaufen" => match subj {
                    Some(true) => kauft += 3,
                    Some(false) => verkauft += 3,
                    None => kauft += 1,
                },
                "kaufst" | "kauft" | "abkaufen" | "abnehmen" | "ankaufen" | "ankauf" => verkauft += 3,
                "verkaufe" => {
                    if ich || dativ_dir {
                        verkauft += 3
                    } else {
                        kauft += 3
                    }
                }
                "verkaufen" => match subj {
                    Some(true) => verkauft += 3,
                    Some(false) => kauft += 3,
                    None => verkauft += 1,
                },
                "verkaufst" => kauft += 3,
                "verkauft" => {
                    if subj == Some(true) {
                        verkauft += 3
                    } else {
                        kauft += 3
                    }
                }
                "hast" | "habt" | "haettest" | "haettet" => kauft += 2,
                "nehme" | "nehmen" => {
                    if ich {
                        kauft += 2
                    }
                }
                "nimmst" | "nehmt" => {
                    if fuer_danach(i) {
                        kauft += 3
                    } else {
                        verkauft += 3
                    }
                }
                "zahlst" | "zahlt" | "bietest" | "bietet" => verkauft += 3,
                "willst" | "verlangst" => {
                    if fuer_danach(i) {
                        kauft += 3
                    }
                }
                "gibst" if fuer_danach(i) => verkauft += 3,
                "gib" | "gibst" | "gebt" | "gebe" | "geben" | "liefere" | "lieferst" | "liefert" | "liefern" | "besorge" | "besorgst" | "abgeben"
                | "ueberlasse" | "ueberlassen" => match subj {
                    Some(true) => verkauft += 2,
                    Some(false) => kauft += 2,
                    None => {
                        if dativ_dir {
                            verkauft += 2
                        } else {
                            kauft += 2
                        }
                    }
                },
                "kostet" | "kosten" | "teuer" => kauft += 2,
                "preis" | "preise" | "kurs" | "kurse" | "konditionen" => kauft += 1,
                "moechte" | "will" | "wuerde" | "haette" | "wuensche" | "interessiert" | "interesse" => kauft += 1,
                "biete" | "bieten" => {
                    // Erste Zahl dahinter: Menge mit Ware (der Gegenüber verkauft) oder Preis (der Gegenüber kauft).
                    match (i + 1..n).find(|&k| t[k].zahl.is_some()) {
                        Some(k) => {
                            let mut j = k + 1;
                            while FUELL.contains(&wort(j)) {
                                j += 1;
                            }
                            if gut_aus_wort(wort(j)).is_some() {
                                verkauft += 3
                            } else {
                                kauft += 3
                            }
                        }
                        None => {
                            if posten.iter().any(|p| p.0 > i) {
                                verkauft += 2
                            } else {
                                kauft += 1
                            }
                        }
                    }
                }
                "loswerden" | "abzugeben" | "uebrig" | "ueberschuss" => {
                    if ich {
                        verkauft += 2
                    }
                }
                _ => {}
            }
        }
        if t.windows(2).any(|p| p[0].w == "zu" && p[1].w == "viel") && ich {
            verkauft += 2;
        }
        let tausch_wort = hat(TAUSCH);
        let tausch = tausch_wort || (posten.len() >= 2 && hat(&["gegen"]) && posten[0].1 != posten[1].1);
        let zustimmung = hat(ZUSTIMMUNG) || t.windows(2).any(|p| p[0].w == "in" && p[1].w == "ordnung") || (n <= 5 && hat(&["ja"]));
        let preisfrage = hat(&["preis", "preise", "kostet", "kosten", "kurs", "kurse", "konditionen", "teuer"]);
        let handelswort = hat(HANDELSWORT);
        let inhalt = !posten.is_empty() || preis.is_some() || !lose.is_empty();
        let stark = hat(&["einverstanden", "deal", "abgemacht", "akzeptiert", "akzeptiere", "handschlag", "gebongt", "topp"]);
        let mut a = Absicht { art: Art::Unklar, gut: posten.first().map(|p| p.1), menge: posten.first().and_then(|p| p.2), preis, gegen: None, preisfrage, stark };
        if !inhalt {
            a.art = if zustimmung && !tausch_wort && !preisfrage {
                Art::Zustimmung
            } else if preisfrage || handelswort || kauft > 0 || verkauft > 0 || tausch_wort {
                Art::Uebersicht
            } else {
                return None;
            };
            return Some(a);
        }
        if tausch {
            // Links vom „gegen“ gibt der Gegenüber ab, rechts davon bekommt er.
            let gegen_pos = t.iter().position(|x| matches!(x.w.as_str(), "gegen" | "fuer" | "mit" | "zu"));
            let (links, rechts): (Vec<_>, Vec<_>) = match gegen_pos {
                Some(g) => (posten.iter().filter(|p| p.0 < g).cloned().collect(), posten.iter().filter(|p| p.0 > g).cloned().collect()),
                None => (posten.iter().take(1).cloned().collect(), posten.iter().skip(1).take(1).cloned().collect()),
            };
            if let (Some(l), Some(r)) = (links.first(), rechts.first()) {
                a.art = Art::Tausch;
                a.gut = Some(l.1);
                a.menge = l.2;
                a.gegen = Some((r.1, r.2));
                return Some(a);
            }
        }
        // „Ich möchte Erz handeln“: ein Wunschwort ohne Richtung, Menge und Preis ist keine Kaufabsicht – der Bot fragt
        // nach der Richtung und nennt seine Preise (Servertest 20.10.2026: vorher „Von Erz habe ich nichts übrig“).
        if hat(&["handeln", "handel", "handelst", "handelt"]) && kauft <= 1 && verkauft == 0 && a.menge.is_none() && a.preis.is_none() {
            a.art = Art::Unklar;
            return Some(a);
        }
        a.art = match kauft.cmp(&verkauft) {
            std::cmp::Ordering::Greater => Art::Kauf,
            std::cmp::Ordering::Less => Art::Verkauf,
            std::cmp::Ordering::Equal => {
                if zustimmung {
                    Art::Zustimmung
                } else {
                    Art::Unklar
                }
            }
        };
        if a.art == Art::Unklar && posten.is_empty() && preis.is_none() {
            return None;
        }
        Some(a)
    }

    /// Der alte Befehl „Kaufe/Verkaufe <Menge> <Ware> für <Preis>“ (ohne ich, mir, dir …) bleibt beim bisherigen Pfad:
    /// der Bot stellt dann selbst eine Marktorder ein.
    pub(crate) fn alter_befehl(words: &[String]) -> bool {
        let has = |s: &str| words.iter().any(|x| x == s);
        if !(has("kaufe") || has("verkaufe")) {
            return false;
        }
        if ["ich", "wir", "mir", "uns", "dir", "euch", "du", "ihr", "mich", "dich"].iter().any(|p| has(p)) {
            return false;
        }
        let verb = if has("kaufe") { "kaufe" } else { "verkaufe" };
        let parsed = words.windows(3).find(|a| a[0] == verb).is_some_and(|a| a[1].parse::<i64>().is_ok() && Gut::ALLE.into_iter().any(|g| g.name() == a[2]));
        let price = words.windows(2).find(|a| a[0] == "für" || a[0] == "fuer" || a[0] == "zu").is_some_and(|a| a[1].replace(',', ".").parse::<f64>().is_ok());
        parsed && price
    }

    /// Kommt der Brief aus dem privaten Postfach (nicht aus dem Allianzchat)? Dort verhandelt der Bot nicht.
    fn ist_privat(w: &Welt, sid: SpielerId, von: SpielerId, text: &str) -> bool {
        for n in w.nachrichten.iter().rev().take(4_000) {
            if n.von == von && n.an.contains(&sid) && !n.text.is_empty() && text.ends_with(n.text.as_str()) {
                return !n.allianz;
            }
        }
        true
    }

    // ------------------------------------------------------------------------------------------------------------
    // Preise
    // ------------------------------------------------------------------------------------------------------------

    #[derive(Clone, Copy, Debug)]
    pub(crate) struct Band {
        /// Eröffnungspreis: was der Bot zuerst nennt.
        pub offen: f64,
        /// Letzte Grenze: weiter geht der Bot nicht (Verkauf: nicht darunter, Kauf: nicht darüber).
        pub grenze: f64,
    }

    /// Vielfache des Marktwerts je Einheit (`marktwert`: Kurs des Soforthandels, sonst Wertungsgewicht). Die Grenzen
    /// liegen am Marktwert wie die Marktorders von `markt_handeln` und `haendler`: kein Umweg über den Soforthandel oder
    /// einen zweiten Bot bringt Gewinn (20.10.2026: vorher 5–12 % des Gewichts). Die Eröffnung liegt davon entfernt, bei
    /// Gütern ohne Soforthandel weiter: so bleibt Spielraum zum Verhandeln.
    pub(crate) fn band(w: &Welt, g: Gut, typ: Bottyp, bot_verkauft: bool) -> Band {
        let gw = marktwert(w, g);
        let fest = sofortkurs(w, g).is_some();
        let (offen, grenze) = match (bot_verkauft, typ, fest) {
            (true, Bottyp::Haendler, true) => (1.10, 1.0),
            (true, Bottyp::Raeuber, true) => (1.20, 1.03),
            (true, _, true) => (1.15, 1.0),
            (false, Bottyp::Haendler, true) => (0.90, 1.0),
            (false, Bottyp::Raeuber, true) => (0.85, 0.97),
            (false, _, true) => (0.88, 1.0),
            (true, Bottyp::Haendler, false) => (1.30, 1.0),
            (true, Bottyp::Raeuber, false) => (1.50, 1.10),
            (true, _, false) => (1.40, 1.0),
            (false, Bottyp::Haendler, false) => (0.70, 1.0),
            (false, Bottyp::Raeuber, false) => (0.60, 0.90),
            (false, _, false) => (0.65, 1.0),
        };
        let r = |x: f64| -> f64 {
            let v = x * gw * 1000.0;
            (if bot_verkauft { (v - 1e-6).ceil() } else { (v + 1e-6).floor() } / 1000.0).max(0.001)
        };
        Band { offen: r(offen), grenze: r(grenze) }
    }

    /// Nächste Zugeständnisstufe: der Bot geht die Hälfte des Weges zu seiner Grenze.
    fn stufe(q0: f64, grenze: f64, verkauf: bool) -> f64 {
        let q = (q0 + grenze) / 2.0;
        let q = if verkauf { (q * 1000.0 - 1e-6).ceil() / 1000.0 } else { (q * 1000.0 + 1e-6).floor() / 1000.0 };
        if (q - grenze).abs() < 0.05 * grenze { grenze } else { q }
    }

    #[derive(Clone, Copy, Debug, PartialEq)]
    pub(crate) enum Entscheid {
        /// Erstes Angebot ohne Gegengebot.
        Angebot(f64),
        /// Das Gegengebot passt (Preis, zu dem die Order bedient wird).
        Annahme(f64),
        /// Gegenvorschlag (näher am Gegenüber, noch nicht an der Grenze).
        Gegen(f64),
        /// Das Gegengebot liegt jenseits der letzten Grenze; Wert: das aktuelle Angebot.
        Grenze(f64),
    }

    /// `verkauf`: der Bot verkauft (das Gebot des Gegenübers ist sein Kaufpreis); sonst kauft der Bot (der Gegenüber
    /// nennt, was er verlangt). `q0` ist das bisherige Angebot des Bots.
    pub(crate) fn entscheide(verkauf: bool, band: Band, gebot: Option<f64>, q0: f64) -> Entscheid {
        let Some(b) = gebot else { return Entscheid::Angebot(q0) };
        let e = 1e-9;
        if verkauf {
            if b >= q0 - e {
                return Entscheid::Annahme(q0);
            }
            let q1 = stufe(q0, band.grenze, true);
            if b >= q1 - e {
                Entscheid::Annahme(b)
            } else if b >= band.grenze - e {
                Entscheid::Gegen(q1)
            } else {
                Entscheid::Grenze(q1)
            }
        } else {
            if b <= q0 + e {
                return Entscheid::Annahme(q0);
            }
            let q1 = stufe(q0, band.grenze, false);
            if b <= q1 + e {
                Entscheid::Annahme(b)
            } else if b <= band.grenze + e {
                Entscheid::Gegen(q1)
            } else {
                Entscheid::Grenze(q1)
            }
        }
    }

    /// „300 Credits für 2000 Erz“ ist ein Gesamtpreis, „1 Credit für 100 Erz“ ein Stückpreis: bei offener Angabe
    /// entscheidet, welche Lesart nahe am Marktwert liegt.
    pub(crate) fn preis_aufloesen(w: &Welt, gut: Gut, menge: Option<i64>, z: Zahl, h: Hinweis) -> Option<f64> {
        let richt = marktwert(w, gut).max(0.001);
        let plausibel = |p: f64| p >= richt / 100.0 && p <= richt * 10.0;
        let einzel = [z.dez, z.tau];
        let gesamt = |x: f64| menge.filter(|m| *m > 0).map(|m| x / m as f64);
        let p = match h {
            Hinweis::Stueck => Some(z.dez),
            Hinweis::Gesamt => gesamt(z.tau).or(Some(z.dez)),
            Hinweis::Offen => einzel
                .into_iter()
                .find(|p| plausibel(*p))
                .or_else(|| einzel.into_iter().filter_map(gesamt).find(|p| plausibel(*p)))
                .or(Some(z.dez)),
        };
        p.filter(|p| p.is_finite() && *p > 0.0)
    }

    // ------------------------------------------------------------------------------------------------------------
    // Zahlen im Brief
    // ------------------------------------------------------------------------------------------------------------

    /// Preis je Einheit mit mindestens zwei Nachkommastellen, Komma als Trenner.
    pub(crate) fn p(x: f64) -> String {
        let mut s = format!("{:.3}", x);
        while s.ends_with('0') && s.split('.').nth(1).is_some_and(|d| d.len() > 2) {
            s.pop();
        }
        s.replace('.', ",")
    }

    pub(crate) fn c(x: f64) -> String {
        if (x - x.round()).abs() < 0.005 { format!("{}", x.round() as i64) } else { format!("{:.2}", x).replace('.', ",") }
    }

    fn prozent(x: f64) -> String {
        let s = format!("{:.1}", x * 100.0);
        s.trim_end_matches('0').trim_end_matches('.').replace('.', ",")
    }

    // ------------------------------------------------------------------------------------------------------------
    // Gesprächszustand: der Brief selbst
    // ------------------------------------------------------------------------------------------------------------

    /// Ein Teil eines Angebots, aus Sicht des Bots.
    #[derive(Clone, Copy, Debug, PartialEq)]
    pub(crate) struct Teil {
        pub gut: Gut,
        pub menge: i64,
        pub preis: f64,
        pub bot_verkauft: bool,
    }

    fn zeile(t: &Teil) -> String {
        format!("ich {} {} {} zu {} Credits je Einheit", if t.bot_verkauft { "verkaufe" } else { "kaufe" }, t.menge, ware(t.gut), p(t.preis))
    }

    pub(crate) fn angebotszeile(teile: &[Teil]) -> String {
        format!("Mein Angebot: {}.", teile.iter().map(zeile).collect::<Vec<_>>().join(" und "))
    }

    fn erledigt_zeile(t: &Teil) -> String {
        format!("Angebot erledigt: {} {}.", ware(t.gut), if t.bot_verkauft { "verkauft" } else { "gekauft" })
    }

    enum Marke {
        Gilt(Teil),
        Erledigt(Gut, Option<bool>),
    }

    fn marken(text: &str) -> Vec<Marke> {
        let t = tokens(text);
        let n = t.len();
        let wort = |i: usize| wort_at(&t, i);
        let mut out = Vec::new();
        let mut i = 0;
        while i < n {
            if wort(i) == "angebot" {
                if wort(i + 1) == "erledigt" {
                    if let Some(g) = gut_aus_wort(wort(i + 2)) {
                        let seite = match wort(i + 3) {
                            "verkauft" => Some(true),
                            "gekauft" => Some(false),
                            _ => None,
                        };
                        out.push(Marke::Erledigt(g, seite));
                        i += 3;
                        continue;
                    }
                } else if wort(i + 1) == "ich" {
                    let mut j = i + 1;
                    loop {
                        if wort(j) != "ich" {
                            break;
                        }
                        let verk = match wort(j + 1) {
                            "verkaufe" => true,
                            "kaufe" => false,
                            _ => break,
                        };
                        let (Some(m), Some(g), "zu", Some(pr)) = (t.get(j + 2).and_then(|x| x.zahl), gut_aus_wort(wort(j + 3)), wort(j + 4), t.get(j + 5).and_then(|x| x.zahl)) else { break };
                        out.push(Marke::Gilt(Teil { gut: g, menge: m.tau.round() as i64, preis: pr.dez, bot_verkauft: verk }));
                        j += 6;
                        while matches!(wort(j), "credits" | "je" | "einheit") {
                            j += 1;
                        }
                        if wort(j) == "und" {
                            j += 1;
                        } else {
                            break;
                        }
                    }
                    i = j;
                    continue;
                }
            }
            i += 1;
        }
        out
    }

    /// Die offenen Angebote des Bots an einen Gegenüber, zusammengesetzt aus seinen Briefen der letzten 24 Spielstunden.
    #[derive(Clone, Debug, Default)]
    pub(crate) struct Gespraech {
        /// Teil, Brief-ID des Angebots, Zeitpunkt.
        pub teile: Vec<(Teil, u64, SimZeit)>,
    }

    impl Gespraech {
        pub(crate) fn teil(&self, g: Gut, bot_verkauft: bool) -> Option<&(Teil, u64, SimZeit)> {
            self.teile.iter().find(|(t, ..)| t.gut == g && t.bot_verkauft == bot_verkauft)
        }
    }

    /// Alle offenen Angebote des Bots, je Gegenüber – ein Durchgang über die Briefe der letzten 24 Spielstunden.
    pub(crate) fn gespraeche(w: &Welt, sid: SpielerId) -> BTreeMap<SpielerId, Gespraech> {
        let seit = w.zeit - GUELTIG;
        let mut je: BTreeMap<SpielerId, Vec<(u64, Vec<Marke>)>> = BTreeMap::new();
        for i in (0..w.nachrichten.len()).rev() {
            let n = &w.nachrichten[i];
            if n.zeit < seit {
                break;
            }
            if n.von != sid || n.allianz || n.an.len() != 1 || !n.text.contains("ngebot") {
                continue;
            }
            let m = marken(&n.text);
            if !m.is_empty() {
                je.entry(n.an[0]).or_default().push((i as u64 + 1, m));
            }
        }
        je.into_iter()
            .map(|(mit, mut briefe)| {
                briefe.reverse();
                let mut live: Vec<(Teil, u64, SimZeit)> = Vec::new();
                for (id, ms) in briefe {
                    let zeit = w.nachrichten[id as usize - 1].zeit;
                    for m in ms {
                        match m {
                            Marke::Gilt(t) => {
                                live.retain(|(x, ..)| !(x.gut == t.gut && x.bot_verkauft == t.bot_verkauft));
                                live.push((t, id, zeit));
                            }
                            Marke::Erledigt(g, s) => live.retain(|(x, ..)| !(x.gut == g && s.is_none_or(|v| v == x.bot_verkauft))),
                        }
                    }
                }
                (mit, Gespraech { teile: live })
            })
            .filter(|(_, g)| !g.teile.is_empty())
            .collect()
    }

    #[cfg(test)]
    pub(crate) fn gespraech(w: &Welt, sid: SpielerId, mit: SpielerId) -> Gespraech {
        gespraeche(w, sid).remove(&mit).unwrap_or_default()
    }

    /// Menge, die der Bot anderen Gegenübern schon zugesagt hat (Verkauf: Ware, Kauf: Lagerplatz und Credits).
    fn zugesagt(alle: &BTreeMap<SpielerId, Gespraech>, ohne: SpielerId, gut: Gut, bot_verkauft: bool) -> i64 {
        alle.iter()
            .filter(|(m, _)| **m != ohne)
            .flat_map(|(_, g)| g.teile.iter())
            .filter(|(t, ..)| t.gut == gut && t.bot_verkauft == bot_verkauft)
            .map(|(t, ..)| t.menge)
            .sum()
    }

    // ------------------------------------------------------------------------------------------------------------
    // Fähigkeiten des Bots: Bedarf, Überschuss, Markt, Budget
    // ------------------------------------------------------------------------------------------------------------

    fn heimat(w: &Welt, sid: SpielerId) -> usize {
        w.spieler[sid as usize].heimat as usize
    }

    /// Kosten der nächsten Stufe und deren Forschung – derselbe Bedarf wie in `markt_handeln`.
    fn bedarf_stufe(w: &Welt, sid: SpielerId) -> [i64; GUETER] {
        let sp = &w.spieler[sid as usize];
        let r = &w.regeln;
        let mut bedarf = r.stufen.get(sp.stufe as usize - 1).map(|s| preis_array(&s.kosten)).unwrap_or([0; GUETER]);
        if let Some(s) = r.stufen.get(sp.stufe as usize - 1) {
            for (f, n) in &s.forschung {
                if sp.forschung[f.idx()] < *n {
                    let k = r.kosten_forschung(*f, sp.forschung[f.idx()] + 1);
                    for g in 0..GUETER {
                        bedarf[g] += k[g];
                    }
                }
            }
        }
        if sp.stufe >= 5 {
            bedarf[Gut::Xenokristall.idx()] = bedarf[Gut::Xenokristall.idx()].max(5000 * M);
        }
        bedarf
    }

    /// Hat der Bot einen Markt und darf er handeln?
    pub(crate) fn handelsfaehig(w: &Welt, sid: SpielerId) -> bool {
        w.spieler_aktiv(sid) && w.planeten[heimat(w, sid)].gebaeude[Gebaeude::Markt.idx()] > 0
    }

    fn orderplaetze(w: &Welt, sid: SpielerId) -> usize {
        let p = &w.planeten[heimat(w, sid)];
        let grenze = p.gebaeude[Gebaeude::Markt.idx()] as usize * w.regeln.markt.orders_je_marktstufe;
        let offen = w.orders.iter().filter(|o| o.spieler == sid && o.planet == p.id).count()
            + w.kommunikation.angebote.iter().filter(|o| o.von == sid && o.planet == p.id && o.status == "offen").count();
        grenze.saturating_sub(offen)
    }

    /// Schafft einen Orderplatz, indem die kleinste eigene Marktorder zurückgenommen wird (der Markt-Zug stellt sie später
    /// wieder ein). Ohne Platz kann keine Gegenorder eingestellt werden.
    fn platz_schaffen(w: &mut Welt, sid: SpielerId) -> bool {
        if orderplaetze(w, sid) > 0 {
            return true;
        }
        let plan = w.planeten[heimat(w, sid)].id;
        let Some(id) = w.orders.iter().filter(|o| o.spieler == sid && o.planet == plan).min_by_key(|o| (o.menge as i128 * o.preis as i128, o.id)).map(|o| o.id) else {
            return false;
        };
        tu(w, sid, json!({"typ": "markt_storno", "order": id})) && orderplaetze(w, sid) > 0
    }

    /// Einheiten, die der Bot abgeben kann: über Rücklage und Mindestbestand (wie `markt_handeln`).
    pub(crate) fn frei_verkauf(w: &Welt, sid: SpielerId, typ: Bottyp, g: Gut) -> i64 {
        let pid = heimat(w, sid);
        let i = g.idx();
        let hat = w.bestand_jetzt(pid)[i];
        let bedarf = bedarf_stufe(w, sid)[i];
        let behalten = if g == Gut::Xenokristall { bedarf + 2000 * M } else { bedarf.max(w.lagergrenze(pid)[i] / 3) };
        let reserve = ruecklage(w, sid, pid, typ)[i];
        ganz((hat - behalten.max(reserve)).max(0))
    }

    /// Einheiten, die der Bot gebrauchen kann: Fehlbedarf für die nächste Stufe oder ein Vorrat im Lager, nie mehr als
    /// ins Lager passt. Wer genug hat, kauft nicht – und verkauft nie, was er hier noch sucht (kein Hin und Her).
    pub(crate) fn bedarf_kauf(w: &Welt, sid: SpielerId, typ: Bottyp, g: Gut) -> i64 {
        let pid = heimat(w, sid);
        let i = g.idx();
        let bedarf = bedarf_stufe(w, sid)[i];
        let reich: i64 = w.spieler[sid as usize].planeten.iter().map(|q| w.bestand_jetzt(*q as usize)[i]).sum();
        let hat = w.bestand_jetzt(pid)[i];
        let lager = w.lagergrenze(pid)[i];
        let vorrat = (if typ == Bottyp::Haendler { lager / 3 } else { lager / 5 }) - hat;
        let platz = (lager - hat).max(0) * 9 / 10;
        ganz((bedarf - reich).max(vorrat).max(0).min(platz))
    }

    /// Lagerplatz für einen Tausch (nur der Platz zählt, nicht der Bedarf).
    fn platz_kauf(w: &Welt, sid: SpielerId, g: Gut) -> i64 {
        let pid = heimat(w, sid);
        ganz((w.lagergrenze(pid)[g.idx()] - w.bestand_jetzt(pid)[g.idx()]).max(0) * 9 / 10)
    }

    /// So viele Einheiten zu `preis`, ohne mehr als ein Viertel der Credits auszugeben.
    fn budget_einheiten(w: &Welt, sid: SpielerId, preis: f64) -> i64 {
        let credits = ganz(w.spieler[sid as usize].credits) as f64;
        ((credits / 4.0) / preis.max(0.001)).floor() as i64
    }

    fn darf_schreiben(w: &Welt, sid: SpielerId) -> bool {
        w.spieler[sid as usize].nachrichten_heute + 2 < w.regeln.agenten.nachrichten_je_tag
    }

    fn feindschaft(w: &Welt, sid: SpielerId, mit: SpielerId) -> bool {
        super::super::allianz_bot::beziehung(w, sid, mit) <= -100
    }

    fn liefer_minuten(w: &Welt, von: Koord, nach: Koord) -> i64 {
        (w.flugdauer(w.entfernung(von, nach), w.regeln.markt.liefertempo, 1000) / 60).max(1)
    }

    // ------------------------------------------------------------------------------------------------------------
    // Bedienen: die Gegenorder des Bots
    // ------------------------------------------------------------------------------------------------------------

    pub(crate) struct Abschluss {
        pub teil: Teil,
        pub menge: i64,
        pub wert: f64,
        /// Planet des Gegenübers, auf dem die Order stand (Ziel oder Herkunft der Ware).
        pub ort: Koord,
    }

    #[derive(Default)]
    pub(crate) struct Bedient {
        pub abschluesse: Vec<Abschluss>,
        pub verfehlt: Vec<String>,
    }

    /// Stellt zu `o` die Gegenorder des Bots ein: der Kern gleicht ab, bucht und liefert. Liefert (Einheiten, Credits).
    /// Eine Restorder bleibt nie im Buch.
    fn gegenorder(w: &mut Welt, sid: SpielerId, o: &Order, max_menge: i64) -> Option<(i64, f64)> {
        let bot_verkauft = o.seite == Marktseite::Kauf;
        let m = (o.menge / M).min(max_menge);
        if m < 1 || !platz_schaffen(w, sid) {
            return None;
        }
        let koord = w.planeten[heimat(w, sid)].koord;
        let (vorher, naechste) = (w.handel.len(), w.naechste_order);
        let a = json!({"typ": "markt_order", "planet": koord, "gut": o.gut.name(), "seite": if bot_verkauft { "verkauf" } else { "kauf" },
            "menge": m, "preis": o.preis as f64 / 1000.0});
        let (ok, _) = w.handeln(sid, Rolle::Alle, &a);
        let rest: Vec<u32> = w.orders.iter().filter(|x| x.spieler == sid && x.id >= naechste).map(|x| x.id).collect();
        for id in rest {
            tu(w, sid, json!({"typ": "markt_storno", "order": id}));
        }
        if !ok {
            return None;
        }
        let (mut menge, mut wert) = (0i64, 0f64);
        for h in &w.handel[vorher..] {
            let passt = if bot_verkauft { h.verkaeufer == sid && h.kaeufer == o.spieler } else { h.kaeufer == sid && h.verkaeufer == o.spieler };
            if h.gut == o.gut && passt {
                menge += h.menge;
                wert += (h.menge as f64 / M as f64) * (h.preis as f64 / M as f64);
            }
        }
        (menge > 0).then_some((menge / M, wert))
    }

    /// Bedient die Orders von `mit`, die zu den offenen Angeboten passen. Ein Teil gilt höchstens einmal: der Rest steht
    /// danach im Brief, den der Aufrufer sendet.
    fn bedienen_teile(w: &mut Welt, sid: SpielerId, typ: Bottyp, mit: SpielerId, g: &Gespraech) -> Bedient {
        let mut res = Bedient::default();
        if !handelsfaehig(w, sid) || feindschaft(w, sid, mit) {
            return res;
        }
        for (t, ..) in &g.teile {
            let seite = if t.bot_verkauft { Marktseite::Kauf } else { Marktseite::Verkauf };
            let grenze = milli(t.preis);
            let mut orders: Vec<Order> = w.orders.iter().filter(|o| o.spieler == mit && o.gut == t.gut && o.seite == seite).cloned().collect();
            if t.bot_verkauft {
                orders.sort_by_key(|o| (-o.preis, o.id));
            } else {
                orders.sort_by_key(|o| (o.preis, o.id));
            }
            let mut rest = t.menge;
            for o in orders {
                if rest < 1 {
                    break;
                }
                let preis = o.preis as f64 / 1000.0;
                let passt = if t.bot_verkauft { o.preis >= grenze } else { o.preis <= grenze };
                if !passt {
                    res.verfehlt.push(if t.bot_verkauft {
                        format!("Deine Kauforder über {} {} liegt mit {} Credits unter meinem Angebot von {}. Nimm sie zurück und stelle sie zu {} neu ein.", o.menge / M, ware(t.gut), p(preis), p(t.preis), p(t.preis))
                    } else {
                        format!("Deine Verkaufsorder über {} {} verlangt {} Credits und liegt über meinem Angebot von {}. Nimm sie zurück und stelle sie zu {} neu ein.", o.menge / M, ware(t.gut), p(preis), p(t.preis), p(t.preis))
                    });
                    continue;
                }
                let kap = if t.bot_verkauft { frei_verkauf(w, sid, typ, t.gut) } else { platz_kauf(w, sid, t.gut).min(budget_einheiten(w, sid, preis)) };
                if kap < 1 {
                    res.verfehlt.push(format!("{} kann ich gerade nicht mehr {}.", ware(t.gut), if t.bot_verkauft { "abgeben" } else { "annehmen" }));
                    continue;
                }
                let ort = w.planeten[o.planet as usize].koord;
                if let Some((m, wert)) = gegenorder(w, sid, &o, rest.min(kap)) {
                    rest -= m;
                    res.abschluesse.push(Abschluss { teil: *t, menge: m, wert, ort });
                }
            }
        }
        res
    }

    fn abschluss_text(w: &Welt, sid: SpielerId, a: &Abschluss) -> String {
        let schnitt = if a.menge > 0 { a.wert / a.menge as f64 } else { 0.0 };
        if a.teil.bot_verkauft {
            let min = liefer_minuten(w, w.planeten[heimat(w, sid)].koord, a.ort);
            format!("{} {} zu {} Credits je Einheit an dich verkauft (zusammen {} Credits); die Ware fliegt nach {} und ist in etwa {} Spielminuten dort.",
                a.menge, ware(a.teil.gut), p(schnitt), c(a.wert), a.ort, min)
        } else {
            format!("{} {} zu {} Credits je Einheit von dir gekauft (zusammen {} Credits, abzüglich der Marktgebühr auf deiner Seite); die Credits sind bei dir gebucht.",
                a.menge, ware(a.teil.gut), p(schnitt), c(a.wert))
        }
    }

    /// Zustandszeilen nach einem Abschluss: Rest des Angebots oder „erledigt“.
    fn zustand_nach(g: &Gespraech, b: &Bedient) -> String {
        let mut bleibt: Vec<Teil> = Vec::new();
        let mut fertig: Vec<String> = Vec::new();
        for (t, ..) in &g.teile {
            let geliefert: i64 = b.abschluesse.iter().filter(|a| a.teil.gut == t.gut && a.teil.bot_verkauft == t.bot_verkauft).map(|a| a.menge).sum();
            if geliefert == 0 {
                continue;
            }
            let rest = t.menge - geliefert;
            if rest >= MIN_LOS.min(t.menge) {
                bleibt.push(Teil { menge: rest, ..*t });
            } else {
                fertig.push(erledigt_zeile(t));
            }
        }
        let mut teile: Vec<String> = Vec::new();
        if !bleibt.is_empty() {
            teile.push(angebotszeile(&bleibt));
        }
        teile.extend(fertig);
        teile.join(" ")
    }

    // ------------------------------------------------------------------------------------------------------------
    // Briefe
    // ------------------------------------------------------------------------------------------------------------

    /// Pflichtsätze (Antwort, Angebotszeile) und Zusatzsätze, die nur so weit mitkommen, wie der Brief Platz hat.
    fn bauen(pflicht: Vec<String>, optional: Vec<String>) -> String {
        let mut s = pflicht.into_iter().filter(|x| !x.is_empty()).collect::<Vec<_>>().join(" ");
        for o in optional.into_iter().filter(|x| !x.is_empty()) {
            if s.chars().count() + 1 + o.chars().count() <= BRIEF_MAX {
                s.push(' ');
                s.push_str(&o);
            }
        }
        s
    }

    fn anleitung(w: &Welt, von: SpielerId, t: &Teil, kurz: bool) -> Vec<String> {
        let summe = t.menge as f64 * t.preis;
        let gebuehr = w.gebuehr(von, None);
        if kurz {
            // Nachfolgerunde im selben Gespräch: der Weg ist bekannt, nur die Werte ändern sich.
            return vec![if t.bot_verkauft {
                format!("Kauforder wie beschrieben: Gut {}, Menge {}, Preis je Einheit {} (hinterlegt werden {} Credits plus Gebühr). Ich bediene sie, sobald sie im Markt steht; setze genau {}, bei einem höheren Limit zahlst du dein Limit.", ware(t.gut), t.menge, p(t.preis), c(summe), p(t.preis))
            } else {
                format!("Verkaufsorder wie beschrieben: Gut {}, Menge {}, Preis je Einheit {} (du bekommst {} Credits abzüglich Gebühr). Ich bediene sie, sobald sie im Markt steht; fordere nicht mehr als {}.", ware(t.gut), t.menge, p(t.preis), c(summe), p(t.preis))
            }];
        }
        if t.bot_verkauft {
            vec![
                format!("So geht es: Öffne unter Handel den Bereich »Handel & Markt« und trage im Markt-Formular ein: Gut {}, Order »Kaufen«, Menge {}, Preis je Einheit {}. Dann »Order einstellen«.", ware(t.gut), t.menge, p(t.preis)),
                "Ich bediene die Order, sobald sie im Markt steht (meist sofort, spätestens in meinem nächsten Zug). Die Ware fliegt zu dem Planeten, von dem du die Order einstellst; vorher liefere ich nichts.".to_string(),
                format!("Hinterlegt werden {} Credits plus Gebühr (bis {} %). Bei einem höheren Limit zahlst du dein Limit, setze also genau {}.", c(summe), prozent(gebuehr), p(t.preis)),
            ]
        } else {
            vec![
                format!("So geht es: Öffne unter Handel den Bereich »Handel & Markt« und trage im Markt-Formular ein: Gut {}, Order »Verkaufen«, Menge {}, Preis je Einheit {}. Dann »Order einstellen«. Die Ware muss auf dem Planeten liegen, von dem du die Order einstellst.", ware(t.gut), t.menge, p(t.preis)),
                "Ich bediene die Order, sobald sie im Markt steht (meist sofort, spätestens in meinem nächsten Zug), und zahle sofort in Credits.".to_string(),
                format!("Du bekommst {} Credits abzüglich Gebühr (bis {} %). Fordere nicht mehr als {}: das ist mein Höchstpreis für dieses Angebot.", c(summe), prozent(gebuehr), p(t.preis)),
            ]
        }
    }

    fn gueltigkeit() -> String {
        "Das Angebot gilt einen Spieltag.".to_string()
    }

    fn ohne_markt() -> String {
        "Ich habe auf meiner Heimatwelt noch keinen Markt gebaut und kann deshalb gerade keine Marktgeschäfte abschließen. Sobald er steht, schreibe mir, was du brauchst oder anbietest (zum Beispiel »Ich brauche 2000 Erz«): Ich antworte mit Menge, Preis je Einheit und dem Weg über »Handel & Markt«.".to_string()
    }

    /// Was der Bot gerade abgeben und gebrauchen könnte (für Übersichten und Alternativen).
    fn aussichten(w: &Welt, sid: SpielerId, typ: Bottyp, alle: &BTreeMap<SpielerId, Gespraech>, von: SpielerId) -> (Vec<String>, Vec<String>) {
        let mut gibt = Vec::new();
        let mut sucht = Vec::new();
        for g in Gut::ALLE {
            let f = (frei_verkauf(w, sid, typ, g) / 2 - zugesagt(alle, von, g, true)).max(0);
            if f >= 100 {
                gibt.push(format!("{} bis {} zu {}", ware(g), f.min(MAX_LOS), p(band(w, g, typ, true).offen)));
            }
            let b = (bedarf_kauf(w, sid, typ, g) - zugesagt(alle, von, g, false)).max(0);
            if b >= 100 {
                sucht.push(format!("{} bis {} zu {}", ware(g), b.min(MAX_LOS), p(band(w, g, typ, false).offen)));
            }
        }
        gibt.truncate(4);
        sucht.truncate(4);
        (gibt, sucht)
    }

    fn alternativen(w: &Welt, sid: SpielerId, typ: Bottyp, alle: &BTreeMap<SpielerId, Gespraech>, von: SpielerId, bot_verkauft: bool) -> String {
        let (gibt, sucht) = aussichten(w, sid, typ, alle, von);
        let l = if bot_verkauft { gibt } else { sucht };
        if l.is_empty() {
            if bot_verkauft { "Abgeben kann ich derzeit nichts.".into() } else { "Gebrauchen kann ich derzeit nichts.".into() }
        } else if bot_verkauft {
            format!("Abgeben könnte ich gerade (Stückzahl, Preis je Einheit): {}.", l.join(", "))
        } else {
            format!("Gebrauchen könnte ich gerade (Stückzahl, Preis je Einheit): {}.", l.join(", "))
        }
    }

    fn uebersicht(w: &Welt, sid: SpielerId, typ: Bottyp, alle: &BTreeMap<SpielerId, Gespraech>, von: SpielerId, preise: bool) -> String {
        let (gibt, sucht) = aussichten(w, sid, typ, alle, von);
        let mut s = Vec::new();
        if gibt.is_empty() && sucht.is_empty() {
            s.push("Im Moment habe ich weder Überschuss, den ich abgeben könnte, noch Bedarf, den ich am Markt decken will.".to_string());
        } else {
            if !gibt.is_empty() {
                s.push(format!("Abgeben könnte ich gerade (Stückzahl, Preis je Einheit in Credits): {}.", gibt.join(", ")));
            }
            if !sucht.is_empty() {
                s.push(format!("Gebrauchen könnte ich gerade (Stückzahl, Preis je Einheit in Credits): {}.", sucht.join(", ")));
            }
        }
        if preise {
            s.push("Das sind meine Eröffnungspreise; über den Preis lässt sich reden.".to_string());
            let mut markt = Vec::new();
            for g in Gut::ALLE {
                let (verk, kauf) = w.marktpreise(g);
                if verk.is_some() || kauf.is_some() {
                    markt.push(format!("{}: Angebot ab {}, Nachfrage bis {}", ware(g),
                        verk.map(|x| p(x as f64 / 1000.0)).unwrap_or_else(|| "–".into()), kauf.map(|x| p(x as f64 / 1000.0)).unwrap_or_else(|| "–".into())));
                }
            }
            if !markt.is_empty() {
                markt.truncate(5);
                s.push(format!("Im Markt liegt gerade: {}.", markt.join("; ")));
            }
        }
        s.push("Schreibe mir zum Beispiel »Ich brauche 2000 Erz«, »Ich verkaufe dir 500 Legierung« oder »Tausche 3000 Kristall gegen 2000 Erz«, dann mache ich dir ein konkretes Angebot.".to_string());
        s.join(" ")
    }

    fn keine_ware(w: &Welt, sid: SpielerId, typ: Bottyp, alle: &BTreeMap<SpielerId, Gespraech>, von: SpielerId, gut: Gut, bot_verkauft: bool) -> String {
        let kopf = if bot_verkauft {
            format!("Von {} habe ich gerade nichts übrig, was ich abgeben könnte; meine Bestände brauche ich selbst.", ware(gut))
        } else {
            format!("{} brauche ich gerade nicht; mein Lager ist dafür gut gefüllt oder ohne Bedarf.", ware(gut))
        };
        format!("{kopf} {}", alternativen(w, sid, typ, alle, von, bot_verkauft))
    }

    // ------------------------------------------------------------------------------------------------------------
    // Antworten auf Handelsbriefe
    // ------------------------------------------------------------------------------------------------------------

    /// Einstieg aus `antwort`: `None`, wenn der Brief kein Handelsbrief ist.
    pub(crate) fn antwort(w: &mut Welt, sid: SpielerId, von: SpielerId, text: &str, typ: Bottyp) -> Option<String> {
        let t = tokens(text);
        let a = lies(&t)?;
        if !ist_privat(w, sid, von, text) {
            return None;
        }
        if !handelsfaehig(w, sid) {
            return Some(ohne_markt());
        }
        if feindschaft(w, sid, von) {
            return Some("Nach unseren bisherigen Kämpfen handle ich nicht mit dir.".into());
        }
        let alle = gespraeche(w, sid);
        let g = alle.get(&von).cloned().unwrap_or_default();
        // Ein bloßes „Ja“ oder „Ok“ ohne offenes Angebot ist kein Handelsbrief.
        if a.art == Art::Zustimmung && g.teile.is_empty() && !a.stark {
            return None;
        }
        let s = match a.art {
            Art::Uebersicht => uebersicht(w, sid, typ, &alle, von, a.preisfrage),
            Art::Zustimmung => zustimmung(w, sid, von, typ, &g),
            Art::Tausch => tausch(w, sid, von, typ, &a, &alle),
            Art::Kauf => einzelhandel(w, sid, von, typ, &a, &alle, true),
            Art::Verkauf => einzelhandel(w, sid, von, typ, &a, &alle, false),
            Art::Unklar => unklar(w, sid, typ, &a, &alle, von),
        };
        Some(s)
    }

    fn unklar(w: &Welt, sid: SpielerId, typ: Bottyp, a: &Absicht, alle: &BTreeMap<SpielerId, Gespraech>, von: SpielerId) -> String {
        let leer = Gespraech::default();
        let g = alle.get(&von).unwrap_or(&leer);
        // Nur ein Preis oder eine Menge ohne Richtung: gibt es genau ein offenes Angebot, bezieht sich der Brief darauf.
        if g.teile.len() == 1 && a.gut.is_none_or(|x| x == g.teile[0].0.gut) {
            let t = g.teile[0].0;
            return format!("Zu welchem Geschäft meinst du das? Mein offenes Angebot lautet: {} {} {} zu {}. Nenne mir Menge und Preis, die du dir vorstellst, oder schreibe »Einverstanden«.",
                if t.bot_verkauft { "ich verkaufe" } else { "ich kaufe" }, t.menge, ware(t.gut), p(t.preis));
        }
        match a.gut {
            Some(gut) => format!("Möchtest du {} kaufen oder verkaufen? Schreibe zum Beispiel »Ich brauche 1000 {}« (dann verkaufe ich dir) oder »Ich verkaufe dir 1000 {}« (dann kaufe ich von dir). {}",
                ware(gut), ware(gut), ware(gut), uebersicht(w, sid, typ, alle, von, false).split(" Schreibe mir").next().unwrap_or("")),
            None => uebersicht(w, sid, typ, alle, von, false),
        }
    }

    /// Ein Teil des Gesprächs: der Bot verkauft (`bot_verkauft`) oder kauft eine Ware, mit Gegengebot und Spielraum.
    fn einzelhandel(w: &mut Welt, sid: SpielerId, von: SpielerId, typ: Bottyp, a: &Absicht, alle: &BTreeMap<SpielerId, Gespraech>, bot_verkauft: bool) -> String {
        let leer = Gespraech::default();
        let g = alle.get(&von).unwrap_or(&leer);
        let kontext: Vec<Teil> = g.teile.iter().map(|x| x.0).filter(|t| t.bot_verkauft == bot_verkauft).collect();
        let Some(gut) = a.gut.or_else(|| (kontext.len() == 1).then(|| kontext[0].gut)) else {
            return format!("Um welche Ware geht es? Ich handle mit Erz, Kristall, Deuterium, Legierung, Elektronik und weiteren Gütern. {}", alternativen(w, sid, typ, alle, von, bot_verkauft));
        };
        let vorher = g.teil(gut, bot_verkauft).map(|x| x.0);
        let bd = band(w, gut, typ, bot_verkauft);
        let q0 = vorher.map(|t| t.preis).unwrap_or(bd.offen);
        // Was anderen Gegenübern schon zugesagt ist, geht ab: kein Bestand wird zweimal versprochen.
        let zu = zugesagt(alle, von, gut, bot_verkauft);
        let kap = if bot_verkauft { frei_verkauf(w, sid, typ, gut) / 2 } else { bedarf_kauf(w, sid, typ, gut).min(budget_einheiten(w, sid, q0.max(bd.grenze))) };
        let kap = (kap - zu).max(0);
        if kap < MIN_LOS {
            return keine_ware(w, sid, typ, alle, von, gut, bot_verkauft);
        }
        let wunsch = a.menge.or(vorher.map(|t| t.menge));
        let menge = wunsch.unwrap_or(kap.min(1_000)).min(kap).min(MAX_LOS);
        let gekuerzt = wunsch.is_some_and(|x| x > menge);
        let gebot = a.preis.and_then(|(z, h)| preis_aufloesen(w, gut, Some(menge), z, h));
        let e = entscheide(bot_verkauft, bd, gebot, q0);
        let (Entscheid::Angebot(preis) | Entscheid::Annahme(preis) | Entscheid::Gegen(preis) | Entscheid::Grenze(preis)) = e;
        let teil = Teil { gut, menge, preis, bot_verkauft };
        let summe = menge as f64 * preis;
        let w_ = ware(gut);
        let kopf = match (e, bot_verkauft) {
            (Entscheid::Angebot(_), true) => format!("Ja, {w_} kann ich dir abgeben: {menge} Stück zu {} Credits je Einheit, zusammen {} Credits.", p(preis), c(summe)),
            (Entscheid::Angebot(_), false) => format!("Ja, {w_} kann ich gebrauchen: {menge} Stück zu {} Credits je Einheit, zusammen {} Credits.", p(preis), c(summe)),
            (Entscheid::Annahme(_), true) => format!("Einverstanden: {} Credits je Einheit passen mir für {menge} {w_}, zusammen {} Credits.", p(preis), c(summe)),
            (Entscheid::Annahme(_), false) => format!("Einverstanden: {} Credits je Einheit passen mir für {menge} {w_}, zusammen {} Credits.", p(preis), c(summe)),
            (Entscheid::Gegen(_), true) => format!("Da liegen wir noch auseinander. Mein Gegenvorschlag für {menge} {w_}: {} Credits je Einheit, zusammen {} Credits.", p(preis), c(summe)),
            (Entscheid::Gegen(_), false) => format!("Da liegen wir noch auseinander. Mein Gegenvorschlag für {menge} {w_}: {} Credits je Einheit, zusammen {} Credits.", p(preis), c(summe)),
            (Entscheid::Grenze(_), true) => format!("Unter {} Credits je Einheit gebe ich {w_} nicht ab; das ist meine letzte Grenze. Mein Angebot für {menge} {w_} lautet {} Credits je Einheit, zusammen {} Credits.", p(bd.grenze), p(preis), c(summe)),
            (Entscheid::Grenze(_), false) => format!("Mehr als {} Credits je Einheit zahle ich für {w_} nicht; das ist meine letzte Grenze. Mein Angebot für {menge} {w_} lautet {} Credits je Einheit, zusammen {} Credits.", p(bd.grenze), p(preis), c(summe)),
        };
        let grenze_hinweis = match e {
            Entscheid::Annahme(pr) if bot_verkauft && gebot.is_some_and(|b| b > pr + 1e-9) => format!("Mehr als {} musst du nicht bieten.", p(pr)),
            Entscheid::Annahme(pr) if !bot_verkauft && gebot.is_some_and(|b| b < pr - 1e-9) => format!("Ich zahle dir {}.", p(pr)),
            _ => String::new(),
        };
        let mengen_hinweis = if gekuerzt {
            if bot_verkauft { format!("Mehr als {menge} kann ich dir nicht abgeben.") } else { format!("Mehr als {menge} brauche ich nicht.") }
        } else {
            String::new()
        };
        let mut optional = anleitung(w, von, &teil, vorher.is_some());
        optional.push(gueltigkeit());
        bauen(vec![kopf, mengen_hinweis, grenze_hinweis, angebotszeile(&[teil])], optional)
    }

    /// Tausch: zwei Orders des Gegenübers, zwei Gegenorders des Bots. Preise in der Mitte der Spanne, Mengen nach Wert
    /// ausgeglichen; jede Order wird einzeln gegen Credits bedient – nichts wird ohne Gegenleistung geliefert.
    fn tausch(w: &mut Welt, sid: SpielerId, von: SpielerId, typ: Bottyp, a: &Absicht, alle: &BTreeMap<SpielerId, Gespraech>) -> String {
        let (Some(gib), Some((nehm, nehm_menge))) = (a.gut, a.gegen) else {
            return "Was möchtest du gegen was tauschen? Schreibe zum Beispiel »Tausche 3000 Kristall gegen 2000 Erz«.".into();
        };
        if gib == nehm {
            return "Ein Tausch braucht zwei verschiedene Waren, zum Beispiel »Tausche 3000 Kristall gegen 2000 Erz«.".into();
        }
        let bv = band(w, nehm, typ, true);
        let bk = band(w, gib, typ, false);
        let p_nehm = ((bv.offen + bv.grenze) / 2.0 * 1000.0 - 1e-6).ceil() / 1000.0;
        let p_gib = ((bk.offen + bk.grenze) / 2.0 * 1000.0 + 1e-6).floor() / 1000.0;
        if p_nehm <= 0.0 || p_gib <= 0.0 {
            return "Diesen Tausch kann ich nicht bewerten.".into();
        }
        let kap_nehm = (frei_verkauf(w, sid, typ, nehm) / 2 - zugesagt(alle, von, nehm, true)).max(0);
        let kap_gib = (platz_kauf(w, sid, gib).min(budget_einheiten(w, sid, p_gib)) - zugesagt(alle, von, gib, false)).max(0);
        if kap_nehm < MIN_LOS {
            return format!("{} kann ich dir nicht abgeben. {}", ware(nehm), alternativen(w, sid, typ, alle, von, true));
        }
        if kap_gib < MIN_LOS {
            return format!("{} kann ich im Tausch nicht annehmen: mir fehlen Lagerplatz oder freie Credits. {}", ware(gib), alternativen(w, sid, typ, alle, von, false));
        }
        let (mut m_gib, mut m_nehm) = match (a.menge, nehm_menge) {
            (Some(x), Some(y)) => (x, y),
            (Some(x), None) => (x, (x as f64 * p_gib / p_nehm).floor() as i64),
            (None, Some(y)) => ((y as f64 * p_nehm / p_gib).ceil() as i64, y),
            (None, None) => return format!("Wie viel möchtest du tauschen? Schreibe zum Beispiel »Tausche 3000 {} gegen 2000 {}«.", ware(gib), ware(nehm)),
        };
        let mut notiz = Vec::new();
        if m_nehm > kap_nehm {
            let faktor = kap_nehm as f64 / m_nehm as f64;
            m_nehm = kap_nehm;
            m_gib = ((m_gib as f64) * faktor).floor() as i64;
            notiz.push(format!("Mehr als {m_nehm} {} kann ich dir nicht abgeben.", ware(nehm)));
        }
        if m_gib > kap_gib {
            let faktor = kap_gib as f64 / m_gib as f64;
            m_gib = kap_gib;
            m_nehm = ((m_nehm as f64) * faktor).floor() as i64;
            notiz.push(format!("Mehr als {m_gib} {} kann ich nicht annehmen.", ware(gib)));
        }
        m_gib = m_gib.min(MAX_LOS);
        m_nehm = m_nehm.min(MAX_LOS);
        if m_gib < 1 || m_nehm < 1 {
            return "Für diesen Tausch reicht meine Kapazität nicht.".into();
        }
        // Ausgleich nach Wert, wenn die Seiten mehr als 20 % auseinanderliegen.
        let (v_gib, v_nehm) = (m_gib as f64 * p_gib, m_nehm as f64 * p_nehm);
        let mut ausgeglichen = false;
        if v_gib > v_nehm * 1.2 {
            let bal = ((v_gib / p_nehm).floor() as i64).min(kap_nehm).min(MAX_LOS);
            if bal > m_nehm {
                notiz.push(format!("Dein Tausch wäre zu deinen Ungunsten: für {m_gib} {} gebe ich dir lieber {bal} {}.", ware(gib), ware(nehm)));
                m_nehm = bal;
                ausgeglichen = true;
            }
            // Reicht mein Vorrat dafür nicht, genügt dir weniger: nur so viel nehme ich an, wie die Ware wert ist.
            let (v_gib, v_nehm) = (m_gib as f64 * p_gib, m_nehm as f64 * p_nehm);
            if v_gib > v_nehm * 1.2 {
                let weniger = ((v_nehm / p_gib).ceil() as i64).max(1);
                if weniger < m_gib {
                    notiz.push(format!("Für {m_nehm} {} genügen {weniger} {}.", ware(nehm), ware(gib)));
                    m_gib = weniger;
                    ausgeglichen = true;
                }
            }
        } else if v_nehm > v_gib * 1.2 {
            let bal = ((v_nehm / p_gib).ceil() as i64).min(kap_gib).min(MAX_LOS);
            if bal > m_gib {
                notiz.push(format!("Dein Tausch ist für mich zu einseitig: für {m_nehm} {} bräuchte ich {bal} {}.", ware(nehm), ware(gib)));
                m_gib = bal;
                ausgeglichen = true;
            }
        }
        let (v_gib, v_nehm) = (m_gib as f64 * p_gib, m_nehm as f64 * p_nehm);
        let saldo = v_gib - v_nehm;
        let t_gib = Teil { gut: gib, menge: m_gib, preis: p_gib, bot_verkauft: false };
        let t_nehm = Teil { gut: nehm, menge: m_nehm, preis: p_nehm, bot_verkauft: true };
        let kopf = format!("{}: du gibst {m_gib} {} und bekommst {m_nehm} {}. Das sind zwei Orders von dir; ich nehme {} zu {} Credits je Einheit ({} Credits) und gebe {} zu {} Credits je Einheit ({} Credits).",
            if ausgeglichen || !notiz.is_empty() { "Gegenvorschlag für den Tausch" } else { "Tausch möglich" }, ware(gib), ware(nehm), ware(gib), p(p_gib), c(v_gib), ware(nehm), p(p_nehm), c(v_nehm));
        let ausgleich = if saldo.abs() < 0.5 {
            "Beide Seiten gleichen sich aus.".to_string()
        } else if saldo > 0.0 {
            format!("Zu deinen Gunsten bleiben {} Credits.", c(saldo))
        } else {
            format!("Du zahlst {} Credits dazu.", c(-saldo))
        };
        let gebuehr = w.gebuehr(von, None);
        let schritte = vec![
            format!("So geht es: Öffne unter Handel den Bereich »Handel & Markt«. Erstens: Order »Verkaufen«, Gut {}, Menge {m_gib}, Preis je Einheit {}. Zweitens: Order »Kaufen«, Gut {}, Menge {m_nehm}, Preis je Einheit {}. Beide Orders stellst du mit »Order einstellen« ein.", ware(gib), p(p_gib), ware(nehm), p(p_nehm)),
            format!("Ich bediene jede Order, sobald sie im Markt steht, und zahle oder liefere nur gegen Credits. Stelle zuerst die Verkaufsorder ein: mit dem Erlös bezahlst du die Kauforder (Gebühr bis {} %). Die Ware fliegt zu dem Planeten, von dem du bestellst.", prozent(gebuehr)),
            gueltigkeit(),
        ];
        let mut pflicht = vec![kopf, ausgleich];
        pflicht.extend(notiz);
        pflicht.push(angebotszeile(&[t_nehm, t_gib]));
        bauen(pflicht, schritte)
    }

    /// „Einverstanden“: bedient die passenden Orders, sonst sagt der Bot, was er noch erwartet.
    fn zustimmung(w: &mut Welt, sid: SpielerId, von: SpielerId, typ: Bottyp, g: &Gespraech) -> String {
        if g.teile.is_empty() {
            return "Dazu liegt von mir gerade kein offenes Angebot vor (Angebote gelten einen Spieltag). Nenne mir Ware und Menge, zum Beispiel »Ich brauche 2000 Erz«, dann mache ich dir ein neues Angebot.".into();
        }
        // Nur bedienen, wenn der Abschlussbrief sicher rausgeht: sonst fehlte der Zustand und ein Teil würde doppelt geliefert.
        let b = if darf_schreiben(w, sid) { bedienen_teile(w, sid, typ, von, g) } else { Bedient::default() };
        antwort_nach_bedienen(w, sid, g, &b, "")
    }

    fn antwort_nach_bedienen(w: &Welt, sid: SpielerId, g: &Gespraech, b: &Bedient, einleitung: &str) -> String {
        let mut pflicht: Vec<String> = vec![einleitung.to_string()];
        let mut optional: Vec<String> = Vec::new();
        if !b.abschluesse.is_empty() {
            pflicht.push(format!("Abgeschlossen: {}", b.abschluesse.iter().map(|a| abschluss_text(w, sid, a)).collect::<Vec<_>>().join(" ")));
            pflicht.push(zustand_nach(g, b));
        }
        // Teile, die noch auf eine Order warten.
        for (t, ..) in &g.teile {
            let bedient: i64 = b.abschluesse.iter().filter(|a| a.teil.gut == t.gut && a.teil.bot_verkauft == t.bot_verkauft).map(|a| a.menge).sum();
            if bedient == 0 && b.verfehlt.is_empty() {
                if t.bot_verkauft {
                    optional.push(format!("Auf deine Kauforder über {} {} zu {} warte ich noch; sobald sie im Markt steht, bediene ich sie.", t.menge, ware(t.gut), p(t.preis)));
                } else {
                    optional.push(format!("Auf deine Verkaufsorder über {} {} zu {} warte ich noch; sobald sie im Markt steht, bediene ich sie.", t.menge, ware(t.gut), p(t.preis)));
                }
            }
        }
        optional.extend(b.verfehlt.iter().cloned());
        if b.abschluesse.is_empty() {
            let teile: Vec<Teil> = g.teile.iter().map(|x| x.0).collect();
            pflicht.push("Gut, dann steht mein Angebot.".to_string());
            pflicht.push(angebotszeile(&teile));
            if !darf_schreiben(w, sid) {
                optional.push("Heute komme ich nicht mehr zum Schreiben; die Orders bediene ich trotzdem im nächsten Zug.".to_string());
            }
        }
        bauen(pflicht, optional)
    }

    // ------------------------------------------------------------------------------------------------------------
    // Hooks: nach jedem Befehl und im eigenen Zug
    // ------------------------------------------------------------------------------------------------------------

    /// Gesprächspartner mit passender Order sofort bedienen und den Abschluss per Brief bestätigen. Aufruf aus
    /// `reagieren_mit` (nach jedem Befehl eines lokalen Spiels und nach jedem Bot-Zug).
    pub(crate) fn markt_bedienen(w: &mut Welt, sid: SpielerId, typ: Bottyp, scripts: &std::collections::BTreeSet<SpielerId>) {
        if !handelsfaehig(w, sid) || !darf_schreiben(w, sid) {
            return;
        }
        // Schneller Ausstieg: ohne Order eines anderen Reichs gibt es nichts zu bedienen.
        let kandidaten: std::collections::BTreeSet<SpielerId> =
            w.orders.iter().map(|o| o.spieler).filter(|s| *s != sid && !scripts.contains(s) && w.spieler_aktiv(*s)).collect();
        if kandidaten.is_empty() {
            return;
        }
        let alle = gespraeche(w, sid);
        for mit in kandidaten {
            let Some(g) = alle.get(&mit) else { continue };
            if !darf_schreiben(w, sid) {
                continue;
            }
            // Unbeantwortete Post des Gegenübers beantwortet zuerst `reagieren_mit` (dort steht „Einverstanden“ o. ä.): ein
            // Brief ohne Bezug würde sie als beantwortet markieren. Der Abschlussbrief hängt am letzten Brief des Gegenübers.
            if offene_post(w, sid, mit) {
                continue;
            }
            let Some(bezug) = letzter_brief_von(w, mit, sid) else { continue };
            let b = bedienen_teile(w, sid, typ, mit, g);
            if b.abschluesse.is_empty() {
                continue;
            }
            let text = antwort_nach_bedienen(w, sid, g, &b, "Deine Order ist bei mir angekommen und bedient.");
            let an = w.spieler[mit as usize].name.clone();
            if !tu(w, sid, json!({"typ": "brief_senden", "kanal": "privat", "an": an, "betreff": "Handel abgeschlossen", "text": text, "antwort_auf": bezug})) {
                // Der Zustand muss in einen Brief: notfalls nur die Zustandszeilen.
                let kurz = format!("Abgeschlossen. {}", zustand_nach(g, &b));
                tu(w, sid, json!({"typ": "brief_senden", "kanal": "privat", "an": an, "betreff": "Handel abgeschlossen", "text": kurz, "antwort_auf": bezug}));
            }
        }
    }

    /// Hat `von` dem Bot einen Brief geschickt, der noch auf Antwort wartet?
    fn offene_post(w: &Welt, sid: SpielerId, von: SpielerId) -> bool {
        w.dialogantworten(sid).into_iter().any(|id| w.nachrichten[id as usize - 1].von == von)
    }

    /// Der letzte private Brief von `von` an `an` (Bezug für Antworten: das Spiel erlaubt nur Antworten auf Briefe des
    /// Empfängers).
    fn letzter_brief_von(w: &Welt, von: SpielerId, an: SpielerId) -> Option<u64> {
        w.nachrichten.iter().enumerate().rev().take(4_000).find(|(_, n)| n.von == von && n.an.contains(&an) && !n.allianz).map(|(i, _)| i as u64 + 1)
    }

    fn heute_gehandelt(w: &Welt, sid: SpielerId, mit: SpielerId, gut: Gut) -> i64 {
        w.handel
            .iter()
            .rev()
            .take_while(|h| h.zeit >= w.zeit - TAG)
            .filter(|h| h.gut == gut && ((h.kaeufer == sid && h.verkaeufer == mit) || (h.kaeufer == mit && h.verkaeufer == sid)))
            .map(|h| h.menge / M)
            .sum()
    }

    /// Im eigenen Zug: erst Gesprächspartner, dann ungefragte Orders aller Reiche (gleich behandelt), die zu Überschuss
    /// oder Bedarf passen – zum Eröffnungspreis, begrenzt je Zug, Gegenüber und Tag.
    pub(crate) fn aktiv_bedienen(w: &mut Welt, sid: SpielerId, typ: Bottyp) {
        if !handelsfaehig(w, sid) || w.spieler[sid as usize].stufe < 3 {
            return;
        }
        markt_bedienen(w, sid, typ, &std::collections::BTreeSet::new());
        let alle = gespraeche(w, sid);
        let kandidaten: Vec<Order> = w.orders.iter().filter(|o| o.spieler != sid && w.zeit - o.zeit >= VORRANG && w.spieler_aktiv(o.spieler)).cloned().collect();
        let mut gemacht = 0;
        for o in kandidaten {
            if gemacht >= MAX_AKTIV {
                break;
            }
            if !w.orders.iter().any(|x| x.id == o.id && x.menge == o.menge) || feindschaft(w, sid, o.spieler) {
                continue;
            }
            // Wer mit diesem Bot gerade verhandelt, wird nach seinem Angebot bedient, nicht nach dem Eröffnungspreis.
            if alle.contains_key(&o.spieler) {
                continue;
            }
            let bot_verkauft = o.seite == Marktseite::Kauf;
            let bd = band(w, o.gut, typ, bot_verkauft);
            let preis = o.preis as f64 / 1000.0;
            let kap = if bot_verkauft {
                if preis < bd.offen - 1e-9 {
                    continue;
                }
                frei_verkauf(w, sid, typ, o.gut) / 4
            } else {
                if preis > bd.offen + 1e-9 {
                    continue;
                }
                bedarf_kauf(w, sid, typ, o.gut).min(budget_einheiten(w, sid, preis))
            };
            let heute = heute_gehandelt(w, sid, o.spieler, o.gut);
            let kap = kap.min(MAX_LOS).min(MAX_TAG - heute);
            if kap < MIN_LOS.min(o.menge / M) || kap < 1 {
                continue;
            }
            if gegenorder(w, sid, &o, kap).is_some() {
                gemacht += 1;
            }
        }
    }
}

// =====================================================================================================================
// Antwort-Set auf typische deutsche Fragen (Karl 20.10.2026). Jede Antwort verspricht nur, was der Bot wirklich tut:
// die Zusagen entsprechen `reagieren_mit`, `diplomatie`, `allianz_bot`, `kolonisieren`, `bot_monde` und `bot_dyson`.
// Nur echte Fragen (Fragewort oder Fragezeichen, keine Ware, keine Zahl) werden hier beantwortet; einzelne
// Befehlswörter wie „Status“, „Hilfe“ oder „Nichtangriffspakt“ bleiben Aufträge in `antwort`.
// =====================================================================================================================
pub(crate) mod fragen {
    use super::handel::{gut_aus_wort, tokens};
    use super::*;

    pub(crate) fn antwort(w: &Welt, sid: SpielerId, typ: Bottyp, text: &str) -> Option<String> {
        let t = tokens(text);
        let hat = |ws: &[&str]| t.iter().any(|x| ws.contains(&x.w.as_str()));
        // Eine Frage hat ein Fragewort, oder ein Fragezeichen mit Anrede („Greifst du an?“). „Frieden und Pakt?“ ist ein Wunsch:
        // der Bot bietet den Pakt an, statt ihn zu erklären (Servertest Spielerart 20.10.2026).
        let fragewort = hat(&["wie", "was", "warum", "wozu", "wieso", "weshalb", "wer", "wo", "wann", "welche", "welcher", "welches", "erklaere", "erklaer", "erklaeren", "bedeutet", "funktioniert", "gibt"]);
        let frage = fragewort || (text.contains('?') && hat(&["du", "dich", "dir", "ihr", "euch"]));
        if !frage {
            return None;
        }
        // Ware oder Zahl: das ist ein Handelsbrief, keine allgemeine Frage.
        if t.iter().any(|x| gut_aus_wort(&x.w).is_some() || x.zahl.is_some()) {
            return None;
        }
        if hat(&["status", "strategie", "kolonien", "absage", "abgelehnt", "ablehnen", "nicht", "nein"]) {
            return None;
        }
        // Vorschläge und Bitten bleiben Aufträge („Wie wäre es mit einem Pakt?“, „Ich möchte eine Allianz“).
        if hat(&["waere", "waer", "waeren", "vorschlag", "vorschlagen", "schlage", "biete", "bieten", "anbieten", "moechte", "moechten", "will", "wollen", "bitte", "einladen", "lade"]) {
            return None;
        }
        let r = typ == Bottyp::Raeuber;
        if hat(&["allianz", "allianzen", "buendnis", "buendnisse", "verbuendete"]) {
            return Some(allianz(w, sid, r));
        }
        if hat(&["pakt", "nichtangriffspakt", "nichtangriffspakte", "nichtangriff", "vertrag", "vertraege"]) {
            return Some(pakt(r));
        }
        if hat(&["krieg", "frieden", "angriff", "angreifen", "angreifst", "greifst", "greifen", "kampf", "kaempfen", "kaempfst", "feind", "feinde", "aggressiv"]) {
            return Some(krieg(typ));
        }
        if hat(&["hilfe", "hilferuf", "entsatz", "helfen", "helfst", "beistand"]) {
            return Some(hilfe(r));
        }
        if hat(&["kolonie", "kolonisieren", "kolonisierst", "besiedeln", "besiedelst", "siedeln"]) {
            return Some(format!(
                "Ich besiedle freie Welten, sobald ich ein erkundetes freies Ziel und ein Kolonieschiff mit Fracht habe. Derzeit habe ich {} Kolonien neben meiner Heimatwelt; »Status« nennt dir die Einzelheiten.",
                w.kolonien(sid)
            ));
        }
        if hat(&["mond", "monde", "monden", "mondbasis", "sprungtor"]) {
            return Some(mond(w, sid, r));
        }
        if hat(&["dysonsphaere", "dyson", "dysonkugel", "dysonschwarm"]) {
            return Some(dyson(w, r));
        }
        if (hat(&["was"]) && hat(&["kannst", "machst", "tust", "leistest", "bietest", "moeglich", "moeglichkeiten"]))
            || (hat(&["wer"]) && hat(&["bist"]))
            || hat(&["faehigkeiten", "funktionen", "befehle", "kommandos", "moeglichkeiten"])
        {
            return Some(faehigkeiten(typ));
        }
        if hat(&["handel", "handeln", "handelt", "handelst", "markt", "kaufen", "verkaufen", "tauschen", "tausch", "geschaefte"]) {
            return Some(handel(w, sid));
        }
        None
    }

    fn faehigkeiten(typ: Bottyp) -> String {
        let basis = "Ich kann dir auf Anfrage meinen Status nennen (»Status«), mit dir handeln (»Ich brauche 2000 Erz«, »Ich verkaufe dir 500 Legierung«, »Tausche 3000 Kristall gegen 2000 Erz«), Preise nennen (»Preise«) und Fragen zu Allianz, Pakt, Krieg und Frieden, Kolonien, Monden und Dysonsphäre beantworten.";
        let rest = if typ == Bottyp::Raeuber {
            "Pakte und Allianzen schließe ich nicht, und Hilfe für andere gibt es von mir nicht; handeln kann ich gegen sofortige Bezahlung."
        } else {
            "Dazu kommen ein Nichtangriffspakt (»Nichtangriffspakt«), in einer gemeinsamen Allianz der Kauf von Angeboten (»Kaufe Angebot 12«) und, wenn ich Kampfschiffe frei habe, Entsatz bei einem Hilferuf."
        };
        format!("{basis} {rest} Jede Zusage führe ich mit den normalen Spielbefehlen aus; was ich nicht kann, sage ich dir.")
    }

    fn allianz(w: &Welt, sid: SpielerId, raeuber: bool) -> String {
        if raeuber {
            return "Allianzen gehe ich nicht ein; ich bleibe unabhängig. Einzelne Geschäfte sind trotzdem möglich.".into();
        }
        let eigene = w.spieler[sid as usize].allianz.and_then(|aid| w.allianzen.iter().find(|a| a.id == aid)).map(|a| format!(" Derzeit bin ich in der Allianz '{}'.", a.name)).unwrap_or_default();
        format!("In einer Allianz teilen wir Chat, einen internen Markt und Hilferufe. Im internen Markt kaufe ich Angebote auf Zuruf (»Kaufe Angebot 12«), und auf einen Hilferuf schicke ich Entsatz, wenn ich Kampfschiffe frei habe. Bin ich in keiner Allianz, nehme ich eine Einladung an; leite ich selbst eine mit freiem Platz, lade ich dich auf Anfrage ein (schreibe »Allianz«).{eigene}")
    }

    fn pakt(raeuber: bool) -> String {
        if raeuber {
            return "Pakte schließe ich nicht. Wer Ruhe will, sichert Lager und Verteidigung; ein bezahltes Handelsgeschäft ist möglich.".into();
        }
        "Ein Nichtangriffspakt ist ein Vertrag unter Diplomatie: Solange er gilt, greifen wir uns nicht an. Schreibe mir »Nichtangriffspakt«, dann biete ich ihn dir an (mit 100 Credits Kaution); ein Angebot von dir nehme ich an.".into()
    }

    fn krieg(typ: Bottyp) -> String {
        match typ {
            Bottyp::Raeuber => "Ich greife Reiche an, bei denen ein Angriff nach meiner Rechnung lohnt; Pakte schließe ich nicht. Handeln kann ich trotzdem, aber nur gegen sofortige Bezahlung.".into(),
            Bottyp::Igel => "Ich greife nicht an. Ich sichere mein Reich mit Bunkern und Verteidigung und helfe Verbündeten, wenn ich Schiffe frei habe. Frieden sichern wir mit einem Nichtangriffspakt: schreibe mir »Nichtangriffspakt«.".into(),
            _ => "Ich greife nicht an, sondern baue meine Wirtschaft aus und verteidige mich. Frieden sichern wir mit einem Nichtangriffspakt: schreibe mir »Nichtangriffspakt«.".into(),
        }
    }

    fn hilfe(raeuber: bool) -> String {
        if raeuber {
            return "Hilfe gibt es von mir nicht; ich schicke keine Entsatzflotten.".into();
        }
        "Hilfe läuft über eine gemeinsame Allianz: Erstelle dort einen Hilferuf für deinen bedrohten Planeten. Ich prüfe dann freie Kampfschiffe, Treibstoff und Flugplätze und nenne dir die tatsächliche Ankunft; ohne freie Schiffe sage ich Nein. Schreibst du mir danach »Hilfe«, prüfe ich deinen offenen Hilferuf sofort.".into()
    }

    fn mond(w: &Welt, sid: SpielerId, raeuber: bool) -> String {
        if !w.monde.aktiv {
            return "In dieser Welt gibt es keine Monde.".into();
        }
        let n = w.monde.monde.values().filter(|m| m.besitzer == sid).count();
        let zusatz = if raeuber { "; als Räuber scanne ich außerdem mit der Tiefenphalanx" } else { "" };
        format!("Ich besitze derzeit {n} Monde. Vorhandene Monde baue ich aus (Mondbasis, Förderung, Werftdock, Hyperraumtor-Generator), versorge sie über Solarsonden mit Energie und verlege Flotten durch das Hyperraumtor{zusatz}.")
    }

    fn dyson(w: &Welt, raeuber: bool) -> String {
        if !w.ausbau.dyson.aktiv {
            return "In dieser Welt gibt es keine Dysonsphäre.".into();
        }
        if raeuber {
            return "An einer Dysonsphäre beteilige ich mich nicht; ich gehöre keiner Allianz an.".into();
        }
        "Eine Dysonsphäre bauen Allianzen gemeinsam in einem System. Gehört mir dort ein Planet, trete ich einem offenen Projekt meiner Allianz bei und zahle aus meinem Überschuss ein; leite ich die Allianz, schlage ich ein Projekt vor und starte den Bau, sobald die Bedingungen erfüllt sind.".into()
    }

    fn handel(w: &Welt, sid: SpielerId) -> String {
        let markt = if super::handel::handelsfaehig(w, sid) { "" } else { " Mein Markt steht noch nicht; bis dahin kann ich keine Marktgeschäfte abschließen." };
        format!("So handeln wir: Schreibe mir, was du brauchst oder anbietest, zum Beispiel »Ich brauche 2000 Erz«. Ich antworte mit einem Angebot (Menge, Preis je Einheit, Summe) und gehe beim Preis ein Stück entgegen, aber nicht unter meine Grenze. Dann stellst du unter Handel im Bereich »Handel & Markt« die passende Order ein; ich bediene sie, sobald sie im Markt steht. Bezahlt wird in Credits, die Ware fliegt zu deinem Planeten, vorab liefere ich nichts. Auf beiden Seiten fällt die Marktgebühr an.{markt}")
    }
}
