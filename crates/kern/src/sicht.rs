//! Was ein Spieler sieht: das Lagebild als Daten und die lesenden Werkzeuge.
//!
//! Die Informationsgrenze liegt hier in der Engine: ein Lagebild enthält nur die eigene
//! Lage, öffentliche Daten und das, was der Spieler selbst herausgefunden hat.

use crate::kampf::{kampf, Gruppe};
use crate::regeltext;
use crate::typen::*;
use crate::welt::*;
use serde_json::{json, Map, Value};

pub(crate) fn gueter(a: &[i64], alle: bool) -> Value {
    let mut m = Map::new();
    for g in Gut::ALLE {
        if alle || a[g.idx()] != 0 {
            m.insert(g.name().to_string(), json!(ganz(a[g.idx()])));
        }
    }
    Value::Object(m)
}

pub(crate) fn einheiten(a: &[i64], ab: usize) -> Value {
    let mut m = Map::new();
    for (i, n) in a.iter().enumerate() {
        if *n > 0 {
            m.insert(Einheit::ALLE[ab + i].name().to_string(), json!(n));
        }
    }
    Value::Object(m)
}

fn stufen<T: Copy + std::fmt::Display>(alle: &[T], werte: &[u8]) -> Value {
    let mut m = Map::new();
    for (i, n) in werte.iter().enumerate() {
        if *n > 0 {
            m.insert(alle[i].to_string(), json!(n));
        }
    }
    Value::Object(m)
}

impl Welt {
    /// Bestand zum jetzigen Zeitpunkt, ohne den Zustand zu ändern.
    pub fn bestand_jetzt(&self, pid: usize) -> [i64; GUETER] {
        let p = &self.planeten[pid];
        let grenze = self.lagergrenze(pid);
        let dt = (self.zeit - p.stand).max(0);
        let mut b = p.bestand;
        for g in 0..GUETER {
            let delta = (p.rate[g] as i128 * dt as i128 / STUNDE as i128) as i64;
            if delta >= 0 {
                if b[g] < grenze[g] {
                    b[g] = (b[g] + delta).min(grenze[g]);
                }
            } else {
                b[g] = (b[g] + delta).max(0);
            }
        }
        b
    }

    fn planet_sicht(&self, pid: usize) -> Value {
        let p = &self.planeten[pid];
        let r = &self.regeln;
        let jetzt = self.zeit;
        let bestand = self.bestand_jetzt(pid);
        let grenze = self.lagergrenze(pid);
        let mut voll_in = Map::new();
        for g in Gut::ALLE {
            let i = g.idx();
            if p.rate[i] > 0 && bestand[i] < grenze[i] {
                voll_in.insert(
                    g.name().to_string(),
                    json!((grenze[i] - bestand[i]) / p.rate[i]),
                );
            } else if p.rate[i] > 0 {
                voll_in.insert(g.name().to_string(), json!(0));
            }
        }
        let schleife: Vec<Value> = p
            .bauschleife
            .iter()
            .map(|a| match a.fertig {
                Some(t) => json!({"gebaeude": a.gebaeude.name(), "stufe": a.stufe, "rest_min": (t - jetzt).max(0) / 60,
                    "fertig_sekunden":t,"dauer_sekunden":if a.dauer>0{a.dauer}else{self.bauzeit(pid,&r.kosten_gebaeude(a.gebaeude,a.stufe))},
                    "pausiert":p.stabilitaet<milli(r.stabilitaet.unruhen_unter)}),
                None => json!({"gebaeude": a.gebaeude.name(), "stufe": a.stufe, "wartet": true}),
            })
            .collect();
        let mut fertigung = Vec::new();
        for (i, name) in ["werft", "orbitalwerft"].iter().enumerate() {
            for (j, f) in p.fertigung[i].iter().enumerate() {
                let was = match f.produkt {
                    Produkt::Einheit(e) => e.name(),
                    Produkt::Bauteil(g) => g.name(),
                };
                let naechstes = if j == 0 {
                    (p.fertigung_naechste[i] - jetzt).max(0) / 60
                } else {
                    f.dauer / 60
                };
                fertigung.push(json!({"schleife": name, "produkt": was, "rest": f.rest, "naechstes_in_min": naechstes,
                    "wartet":j>0,"fertig_sekunden":if j==0{Some(p.fertigung_naechste[i])}else{None},"dauer_sekunden":f.dauer}));
            }
        }
        // Nächste Stufe jedes Gebäudes, das hier je gebaut werden kann, mit Kosten, Bauzeit und dem, was noch
        // fehlt. Dieselben Prüfungen wie beim Bauen; spart dem Verwalter eine Kostenabfrage je Gebäude.
        let stufe_sp = self.spieler[p.besitzer as usize].stufe;
        let nebel = self.system(p.koord).map(|s| s.nebel).unwrap_or(false);
        let mut baubar = Vec::new();
        for g in Gebaeude::ALLE {
            if !r.gebaeude.contains_key(&g) {continue;}
            let gr = r.geb(g);
            let gross = matches!(
                g,
                Gebaeude::Orbitalring | Gebaeude::Forschungsarchiv | Gebaeude::Versorgungsnetz
            );
            let naechste = p.gebaeude[g.idx()]
                + p.bauschleife.iter().filter(|a| a.gebaeude == g).count() as u8
                + 1;
            if gr.ab_stufe > stufe_sp
                || (g == Gebaeude::Xenoextraktor && !nebel)
                || (gross && naechste > 1)
                || naechste > r.welt.max_gebaeudestufe
            {
                continue;
            }
            let k = r.kosten_gebaeude(g, naechste);
            let fehlt: Vec<&str> = Gut::ALLE
                .iter()
                .filter(|x| bestand[x.idx()] < k[x.idx()])
                .map(|x| x.name())
                .collect();
            let mut braucht: Vec<String> = gr
                .braucht
                .iter()
                .filter(|(b, st)| p.gebaeude[b.idx()] < **st)
                .map(|(b, st)| format!("{b} {st}"))
                .collect();
            if self.integritaet(pid, g) < 1000 {
                braucht.push("Zuerst reparieren".into());
            }
            // Was die Stufe bringt und zusätzlich braucht, mit den Faktoren dieses Planeten: ohne das fragte der
            // Verwalter fast jeden Bau einzeln ab, und jede Abfrage kostete einen weiteren Modellaufruf.
            let plus =
                |grund: f64| r.stufenwert(grund, naechste) - r.stufenwert(grund, naechste - 1);
            let roh = |f: usize, art: &'static str| {
                Some((anteil(plus(gr.ertrag), p.faktor[f], 1000), art))
            };
            let ertrag = match g {
                Gebaeude::Erzmine => roh(F_ERZ, "erz je Stunde"),
                Gebaeude::Kristallmine => roh(F_KRISTALL, "kristall je Stunde"),
                Gebaeude::Deuteriumsynthesizer => roh(F_DEUTERIUM, "deuterium je Stunde"),
                Gebaeude::Farm => roh(F_NAHRUNG, "nahrung je Stunde"),
                Gebaeude::Solarkraftwerk => roh(F_SOLAR, "Strom"),
                Gebaeude::Fusionskraftwerk => Some((plus(gr.ertrag), "Strom")),
                Gebaeude::Giesserei => Some((plus(gr.ertrag), "legierung je Stunde")),
                Gebaeude::Elektronikwerk => Some((plus(gr.ertrag), "elektronik je Stunde")),
                Gebaeude::Konsumgueterwerk => Some((plus(gr.ertrag), "konsumgut je Stunde")),
                Gebaeude::Xenoextraktor => Some((plus(gr.ertrag), "xenokristall je Stunde")),
                Gebaeude::Wohnblock => Some((plus(gr.ertrag), "Wohnraum")),
                Gebaeude::Labor => Some((plus(gr.ertrag), "Forschungspunkte je Stunde")),
                Gebaeude::Akademie => {
                    Some((plus(r.wirtschaft.fachkraefte_je_akademie), "Fachkräfte"))
                }
                Gebaeude::Lager => Some((
                    r.lagergrenze(naechste)[Gut::Erz.idx()]
                        - r.lagergrenze(naechste - 1)[Gut::Erz.idx()],
                    "Lagerplatz je Rohstoff",
                )),
                Gebaeude::Bunker => Some((
                    r.bunkerschutz(naechste)[Gut::Erz.idx()]
                        - r.bunkerschutz(naechste - 1)[Gut::Erz.idx()],
                    "geschützt je Rohstoff",
                )),
                _ => None,
            }
            .filter(|(m, _)| *m > 0)
            .map(|(m, art)| json!({"plus": ganz(m), "art": art}));
            baubar.push(json!({"gebaeude": g.name(), "stufe": naechste, "kosten": gueter(&k, false), "bauzeit_min": self.bauzeit(pid, &k) / 60, "bauzeit_sekunden":self.bauzeit(pid,&k),
                "fehlt": fehlt, "braucht": braucht, "ertrag": ertrag, "strom_plus": ganz(plus(gr.energie)),
                "arbeiter_plus": ganz(plus(gr.arbeiter)), "fachkraefte_plus": ganz(plus(gr.fachkraefte)), "kostenfaktor": gr.faktor}));
        }
        let blockade = p.blockade.and_then(|fid| self.flotten.get(&fid)).map(|f| {
            json!({"durch": self.spieler[f.besitzer as usize].name, "art": f.mission.name(), "flotte": f.id})
        });
        let mut view = json!({
            "koord": p.koord.to_string(),
            "heimat": p.heimat,
            "zone": p.zone.name(),
            "nebel": self.system(p.koord).map(|s| s.nebel).unwrap_or(false),
            "felder": {"belegt": self.felder_belegt(pid), "gesamt": self.felder_gesamt(pid)},
            "faktoren": {"erz": p.faktor[F_ERZ], "kristall": p.faktor[F_KRISTALL], "deuterium": p.faktor[F_DEUTERIUM],
                         "nahrung": p.faktor[F_NAHRUNG], "solar": p.faktor[F_SOLAR]},
            "bevoelkerung": ganz(p.bevoelkerung),
            "wohnraum": ganz(p.wohnraum),
            "stabilitaet": p.stabilitaet / M,
            "stabilitaet_ziel": p.stab_ziel / M,
            "bestand": gueter(&bestand, true),
            "rate": gueter(&p.rate, true),
            "lager": gueter(&grenze, true),
            "voll_in_stunden": voll_in,
            "bunkerschutz": ganz(r.bunkerschutz(p.gebaeude[Gebaeude::Bunker.idx()])[0]),
            "energie": {"erzeugung": ganz(p.energie_erzeugung), "verbrauch": ganz(p.energie_verbrauch)},
            "arbeit": {"verfuegbar": ganz(p.arbeit_verfuegbar), "bedarf": ganz(p.arbeit_bedarf)},
            "fachkraefte": {"verfuegbar": ganz(p.fk_verfuegbar), "bedarf": ganz(p.fk_bedarf)},
            "nahrung_deckung": p.nahrung_deckung / 10,
            "konsum_deckung": p.konsum_deckung / 10,
            "gebaeude": stufen(&Gebaeude::ALLE, &p.gebaeude),
            "gebaeude_integritaet_prozent": Gebaeude::ALLE.iter().filter(|g|p.gebaeude[g.idx()]>0).map(|g|(g.name(),self.integritaet(pid,*g)/10)).collect::<std::collections::BTreeMap<_,_>>(),
            "kampfkolonisation": self.kolonisation.besetzungen.get(&p.id),
            "reparaturen": self.kolonisation.reparaturen.iter().filter(|((id,_),_)|*id==p.id).map(|((_,g),r)|json!({"gebaeude":g.name(),"fertig":r.fertig})).collect::<Vec<_>>(),
            "bauschleife": schleife,
            "bauschleife_plaetze": r.wirtschaft.warteschlange,
            // Reihenfolge, in der gebaute Gebäude Arbeitskräfte bekommen (eigene Vorgabe, dann Versorgung, dann der Rest).
            "prioritaeten": Self::reihenfolge(p).into_iter().filter(|g| p.gebaeude[g.idx()] > 0).map(|g| g.name()).collect::<Vec<_>>(),
            "baubar": baubar,
            "fertigung": fertigung,
            "schiffe": einheiten(&p.einheiten[..SCHIFFE], 0),
            "verteidigung": einheiten(&p.einheiten[SCHIFFE..], SCHIFFE),
            "raketen": {"abfang":p.raketen[0],"interplanetar":p.raketen[1],"belegt_mit_bau":self.raketen_belegt(pid),"kapazitaet":p.gebaeude[Gebaeude::Raketensilo.idx()] as i64*r.zusatz.silo_plaetze_je_stufe},
            "raketenbau": self.ereignisse.iter().filter_map(|e| match e.art { EreignisArt::RaketenFertig{planet,besitzer,art,anzahl} if planet==p.id && besitzer==p.besitzer=>Some(json!({"art":if art==0{"abfang"}else{"interplanetar"},"anzahl":anzahl,"fertig":zeittext(e.zeit)})),_=>None }).collect::<Vec<_>>(),
            "garnison": self.garnison(pid),
            "blockade": blockade,
        });
        if self.kolonisationsregeln_v2() {
            view["vorrat_reicht_sekunden"] = json!(Gut::ALLE
                .iter()
                .filter(|g| p.rate[g.idx()] < 0)
                .map(|g| (
                    g.name(),
                    (bestand[g.idx()].max(0) as i128 * STUNDE as i128 / -(p.rate[g.idx()] as i128))
                        as i64
                ))
                .collect::<std::collections::BTreeMap<_, _>>());
            view["versorgungsprognose"] = json!("Konstante aktuelle Raten; Wachstum, neue Bauten, Lieferungen und Angriffe ändern die Reichweite.");
        }
        if !self.kolonisation.aktiv {
            for key in [
                "gebaeude_integritaet_prozent",
                "kampfkolonisation",
                "reparaturen",
            ] {
                view.as_object_mut().unwrap().remove(key);
            }
        }
        view
    }

    /// Schritte zur nächsten Kolonie, in der Reihenfolge, in der man sie geht: Stufe, Astrophysik, Raumhafen,
    /// Orbitalwerft, Bauteile, Kolonieschiff, ein erkundeter freier Platz, der Start. Leer, wenn keine Kolonie mehr
    /// erlaubt sein kann. Menschen und Modelle sehen so, was als Nächstes fehlt, statt nur „3 Kolonien (jetzt 0)“.
    fn kolonie_weg(&self, sid: SpielerId) -> Value {
        let r = &self.regeln;
        let sp = &self.spieler[sid as usize];
        let anzahl = self.kolonien(sid);
        let ab = r.einh(Einheit::Kolonieschiff).ab_stufe;
        // Auch junge Reiche sollen ihren vollständigen Weg zur Kolonie planen können.
        if anzahl >= r.wirtschaft.kolonien_max as usize {
            return json!([]);
        }
        let planeten: Vec<usize> = sp.planeten.iter().map(|&p| p as usize).collect();
        let hat = |g: Gebaeude| {
            planeten
                .iter()
                .any(|&p| self.planeten[p].gebaeude[g.idx()] > 0)
        };
        let schiff = Einheit::Kolonieschiff;
        let kosten = r.kosten_einheit(schiff, sp.volk);
        let bestand = |g: Gut| {
            planeten
                .iter()
                .map(|&p| self.bestand_jetzt(p)[g.idx()])
                .max()
                .unwrap_or(0)
        };
        let gebaut = planeten
            .iter()
            .map(|&p| self.planeten[p].einheiten[schiff.idx()])
            .sum::<i64>()
            > 0;
        let unterwegs = self
            .flotten
            .values()
            .any(|f| f.besitzer == sid && f.mission == Mission::Kolonisieren);
        let (kern, habitat) = (
            kosten[Gut::Antriebskern.idx()],
            kosten[Gut::Habitatmodul.idx()],
        );
        let stufe = r
            .stufen
            .get((ab as usize).saturating_sub(2))
            .map(|s| s.name.clone())
            .unwrap_or_default();
        let erlaubt = self.kolonien_erlaubt(sid);
        let schritte = [
            (format!("Zivilisationsstufe {stufe}"), sp.stufe >= ab),
            (
                format!("Astrophysik erlaubt eine weitere Kolonie (jetzt {anzahl} von {erlaubt})"),
                erlaubt > anzahl,
            ),
            (
                "Raumhafen auf dem Startplaneten".to_string(),
                hat(Gebaeude::Raumhafen),
            ),
            (
                "Orbitalwerft (braucht Werft 4)".to_string(),
                hat(Gebaeude::Orbitalwerft),
            ),
            (
                format!(
                    "Bauteile aus der Orbitalwerft: {} Antriebskern, {} Habitatmodul",
                    ganz(kern),
                    ganz(habitat)
                ),
                gebaut
                    || unterwegs
                    || (bestand(Gut::Antriebskern) >= kern
                        && bestand(Gut::Habitatmodul) >= habitat),
            ),
            (
                "Kolonieschiff gefertigt (Werft)".to_string(),
                gebaut || unterwegs,
            ),
            (
                "Freier Platz erkundet (Mission spionage mit einer Sonde auf den Platz)"
                    .to_string(),
                sp.erkundet.keys().any(|k| self.planetenwissen(sid,*k)["status"]=="frei"),
            ),
            (
                "Flotte mit Kolonieschiff und Mission kolonisieren gestartet".to_string(),
                unterwegs,
            ),
        ];
        let mut out = schritte
            .into_iter()
            .map(|(text, erledigt)| json!({"text": text, "erledigt": erledigt}))
            .collect::<Vec<_>>();
        if self.kolonisationsregeln_v2() {
            let cargo = self.koloniefracht(sid);
            let ready = planeten.iter().any(|&pid| {
                let p = &self.planeten[pid];
                p.einheiten[Einheit::Kolonieschiff.idx()] > 0
                    && Einheit::ALLE[..SCHIFFE].iter().any(|e| {
                        *e != Einheit::Spionagesonde
                            && *e != Einheit::Kolonieschiff
                            && p.einheiten[e.idx()] > 0
                            && r.einh(*e).angriff > 0
                    })
                    && self
                        .bestand_jetzt(pid)
                        .iter()
                        .zip(cargo)
                        .all(|(have, need)| *have >= need)
            });
            out.insert(out.len()-1,json!({"text":"Kolonieschiff, bewaffnete Eskorte und Mindestfracht am selben Startplaneten; Treibstoff/Laderaum zusätzlich prüfen","erledigt":ready||unterwegs,"mindestfracht":gueter(&cargo,false)}));
            out.push(json!({"text":"Mit kolonieplan den selbst gewählten Aufbau prüfen: Mindestfracht allein garantiert keine Selbstversorgung","erledigt":false,"hinweis":true}));
        }
        json!(out)
    }

    fn bericht_sicht(&self, sid: SpielerId, b: &Spionagebericht) -> Value {
        json!({
            "ziel": b.ziel.to_string(),
            "besitzer": self.spieler[b.besitzer as usize].name,
            "alter_stunden": (self.zeit - b.zeit) / STUNDE,
            "bestand": Gut::ALLE.iter().enumerate().map(|(i,g)| {
                let span=self.scans.get(&(sid,b.ziel)).and_then(|s|s.bestand.get(i)).copied()
                    .unwrap_or_else(||crate::sternkarte::intervall(ganz(b.bestand[i]),10000));
                (g.name(),(span[0]+span[1])/2)
            }).collect::<std::collections::BTreeMap<_,_>>(),
            "geschaetzt": true,
            "aufklaerung": self.planetenwissen(sid,b.ziel),
            "schiffe": b.schiffe.as_ref().map(|s| einheiten(s, 0)),
            "verteidigung": b.verteidigung.as_ref().map(|s| einheiten(s, SCHIFFE)),
            "gebaeude": b.gebaeude.as_ref().map(|s| stufen(&Gebaeude::ALLE, s)),
            "forschung": b.forschung.as_ref().map(|s| stufen(&Forschung::ALLE, s)),
        })
    }

    /// Lagebild einer Rolle als Daten. Der Orchestrator macht daraus Text.
    pub fn sicht(&self, sid: SpielerId, rolle: Rolle) -> Value {
        let r = &self.regeln;
        let sp = &self.spieler[sid as usize];
        let jetzt = self.zeit;
        let seit = if rolle == Rolle::Alle {
            0
        } else {
            sp.weck[rolle.idx()].letzter
        };
        let name = |id: SpielerId| self.spieler[id as usize].name.clone();

        let planeten: Vec<Value> = sp
            .planeten
            .iter()
            .map(|pid| self.planet_sicht(*pid as usize))
            .collect();

        // Warnungen als reine Fakten.
        let mut warnungen: Vec<String> = Vec::new();
        for pid in &sp.planeten {
            let p = &self.planeten[*pid as usize];
            let k = p.koord;
            if p.energie_erzeugung < p.energie_verbrauch {
                warnungen.push(format!(
                    "Energie fehlt auf {k}: Erzeugung {}, Verbrauch {}",
                    ganz(p.energie_erzeugung),
                    ganz(p.energie_verbrauch)
                ));
            }
            if p.nahrung_deckung < 1000 {
                warnungen.push(format!(
                    "Versorgung auf {k} nur zu {} Prozent gedeckt",
                    p.nahrung_deckung / 10
                ));
            }
            if p.arbeit_bedarf > p.arbeit_verfuegbar {
                warnungen.push(format!(
                    "Arbeitskräfte fehlen auf {k}: Bedarf {}, verfügbar {}",
                    ganz(p.arbeit_bedarf),
                    ganz(p.arbeit_verfuegbar)
                ));
            }
            if p.stabilitaet < milli(r.stabilitaet.unruhen_unter) {
                warnungen.push(format!("Unruhen auf {k}: Stabilität {}", p.stabilitaet / M));
            }
            let bestand = self.bestand_jetzt(*pid as usize);
            let grenze = self.lagergrenze(*pid as usize);
            for g in Gut::ALLE {
                let i = g.idx();
                if p.rate[i] > 0 && (grenze[i] - bestand[i]) / p.rate[i] < 6 {
                    warnungen.push(format!("Lager für {g} auf {k} voll in unter 6 Stunden"));
                }
            }
            if p.bauschleife
                .first()
                .map(|a| a.fertig.is_none())
                .unwrap_or(false)
            {
                let a = &p.bauschleife[0];
                warnungen.push(format!(
                    "Bauauftrag {} Stufe {} auf {k} wartet auf Güter oder Topf",
                    a.gebaeude, a.stufe
                ));
            }
            if p.blockade.is_some() {
                warnungen.push(format!("{k} wird blockiert"));
            }
        }
        if sp.schuld_seit.is_some() {
            warnungen.push("Unterhalt ist unbezahlt".to_string());
        }
        let mut angriffe: Vec<Value> = Vec::new();
        for fid in self.sichtbare_angriffe(sid) {
            let info = self.angriffswissen(sid, fid).expect("Sichtbarer Angriff");
            let text = format!(
                "Feindliche Flotte erreicht {} um {}; Schiffszahl: {}",
                info["ziel"].as_str().unwrap_or(""), info["ankunft"].as_str().unwrap_or(""),
                info["schiffe"].as_i64().map(|n|n.to_string()).unwrap_or_else(||"unbekannt".into())
            );
            warnungen.push(text);
            angriffe.push(info);
        }

        let flotten: Vec<Value> = self
            .flotten
            .values()
            .filter(|f| f.besitzer == sid)
            .map(|f| {
                let (zustand, wann) = match f.zustand {
                    Flottenzustand::Hinflug => ("hinflug", f.ankunft),
                    Flottenzustand::ImOrbit => ("im_orbit", f.orbit_ende),
                    Flottenzustand::Rueckflug => ("rueckflug", f.rueckkehr),
                };
                let mut view=json!({"flotte": f.id, "mission": f.mission.name(), "start": f.start_koord.to_string(), "ziel": f.ziel.to_string(),
                    "zustand": zustand, "abflug_sekunden":f.abflug,"ankunft_sekunden":f.ankunft,"rueckkehr_sekunden":f.rueckkehr,
                    "fortschritt": if self.kolonisation.rueckwege.contains_key(&f.id) {Value::Null} else {json!(match f.zustand {
                        Flottenzustand::Hinflug => ((jetzt-f.abflug) as f64 / f.flugdauer.max(1) as f64).clamp(0.,1.),
                        Flottenzustand::ImOrbit => 1.,
                        Flottenzustand::Rueckflug => ((f.rueckkehr-jetzt) as f64 / f.flugdauer.max(1) as f64).clamp(0.,1.),
                    })}, "bis": if wann > 0 { json!(zeittext(wann)) } else { Value::Null },
                    "schiffe": einheiten(&f.schiffe, 0), "ladung": gueter(&f.ladung, false),"verband":f.verband,"verbandsfuehrung":f.verband==Some(f.id)});
                if self.kolonisationsregeln_v2() {
                    if let Some(route)=self.kolonisation.rueckwege.get(&f.id) {
                        view["rueckkehr"]=json!({"ziel":self.planeten[route.ziel as usize].koord.to_string(),
                            "wartend":route.wartend,"grund":if route.wartend{Some("Landung durch feindliche Blockade gesperrt; Flotte belegt weiter einen Platz und kostet Unterhalt")}else{None}});
                    }
                    if let Some(target)=self.kolonisation.transportziele.get(&f.id) {
                        view["gebundener_empfaenger"]=match target {
                            crate::kolonisation::Transportziel::Planet{planet,empfaenger}=>json!({"art":"planet","planet":self.planeten[*planet as usize].koord.to_string(),"spieler":empfaenger}),
                            crate::kolonisation::Transportziel::Flotte{flotte,empfaenger}=>json!({"art":"flotte","flotte":flotte,"spieler":empfaenger}),
                        };
                    }
                }
                view
            })
            .collect();

        // Only observed ownership, never undiscovered live positions.
        let nachbarn: Vec<Value> = sp.berichte.iter().map(|b| {
            let mut v = self.planetenwissen(sid,b.ziel);
            v["spieler"]=json!(name(b.besitzer)); v
        }).collect();

        let rangliste: Vec<Value> = self
            .rangliste()
            .into_iter()
            .map(|(rang, n, punkte, stufe)| json!([rang, n, punkte, stufe]))
            .collect();

        let vertraege: Vec<Value> = self
            .vertraege
            .iter()
            .filter(|v| (v.a == sid || v.b == sid) && !matches!(v.status, Vertragsstatus::Beendet | Vertragsstatus::Abgelehnt | Vertragsstatus::Gebrochen))
            .map(|v| {
                let partner = if v.a == sid { v.b } else { v.a };
                let status = match v.status {
                    Vertragsstatus::Angeboten if v.b == sid => "angebot an dich".to_string(),
                    Vertragsstatus::Angeboten => "von dir angeboten".to_string(),
                    Vertragsstatus::Gekuendigt { ende } => format!("gekündigt, endet {}", zeittext(ende)),
                    _ => "aktiv".to_string(),
                };
                let mut o = json!({"vertrag": v.id, "art": v.art.name(), "partner": name(partner), "status": status, "kaution": ganz(v.kaution)});
                if v.art == Vertragsart::Tribut {
                    o["tribut"] = json!({"zahler": name(v.a), "gut": v.tribut_gut.map(|g| g.name()).unwrap_or("credits"),
                        "menge_je_tag": ganz(v.tribut_menge)});
                }
                o
            })
            .collect();
        let register: Vec<Value> = self
            .register
            .iter()
            .rev()
            .take(20)
            .map(|e| json!({"zeit": zeittext(e.zeit), "art": e.art, "a": e.a, "b": e.b, "vorgang": e.vorgang}))
            .collect();
        let allianz = sp.allianz.and_then(|aid| self.allianzen.iter().find(|a| a.id == aid)).map(|a| {
            json!({"name": a.name, "mitglieder": a.mitglieder.iter().map(|m| name(*m)).collect::<Vec<_>>()})
        });
        let einladungen: Vec<String> = self
            .allianzen
            .iter()
            .filter(|a| a.eingeladen.contains(&sid))
            .map(|a| a.name.clone())
            .collect();

        let grenze = jetzt - r.agenten.chronik_tage * TAG;
        let nachrichten: Vec<Value> = self.briefkasten(sid).into_iter().filter(|n|n["sekunden"].as_i64().unwrap_or(0)>=grenze).take(30).map(|mut n|{n["neu"]=json!(n["gelesen"]==false && n["gesendet"]==false);n}).collect();

        let ereignisse: Vec<Value> = sp
            .vorfaelle
            .iter()
            .filter(|v| v.zeit >= seit)
            .rev()
            .take(40)
            .map(|v| json!({"zeit": zeittext(v.zeit), "art": v.art, "text": v.text}))
            .collect();
        let wichtig = [
            "kampf",
            "beute",
            "stufe",
            "vertrag",
            "eroberung",
            "kolonie",
            "blockade",
            "unterhalt",
            "allianz",
        ];
        let chronik: Vec<Value> = sp
            .vorfaelle
            .iter()
            .filter(|v| v.zeit < seit && wichtig.contains(&v.art.as_str()))
            .rev()
            .take(25)
            .map(|v| json!({"zeit": zeittext(v.zeit), "text": v.text}))
            .collect();

        let naechste_stufe = r.stufen.get(sp.stufe as usize - 1).map(|regel| {
            json!({
                "name": regel.name,
                "bedingungen": self.stufen_bedingungen(sid).into_iter().map(|(t, ok)| json!({"text": t, "erfuellt": ok})).collect::<Vec<_>>(),
                "erfuellt_seit_stunden": sp.stufen_zaehler / STUNDE,
                "haltezeit_stunden": r.stufen_haltezeit_stunden,
                "kosten": regel.kosten.iter().map(|(g, m)| (g.name().to_string(), json!(m))).collect::<Map<String, Value>>(),
            })
        });

        let fp = self.fp_rate(sid);
        // Nächste Stufe jeder Forschung, die die Zivilisationsstufe schon erlaubt; geforscht wird auf der Heimatwelt,
        // wenn kein anderer Planet genannt ist.
        let heim = sp.heimat as usize;
        let bestand_heim = self.bestand_jetzt(heim);
        let labor_heim = self.planeten[heim].gebaeude[Gebaeude::Labor.idx()];
        let forschung_moeglich: Vec<Value> = Forschung::ALLE
            .iter()
            .filter(|f|r.forschung.contains_key(f))
            .filter(|f| r.forsch(**f).ab_stufe <= sp.stufe)
            .filter_map(|f| {
                let geplant = sp.forschung_aktiv.as_ref().filter(|a| a.forschung == *f).map(|_| 1).unwrap_or(0)
                    + sp.forschung_schlange.iter().filter(|(x, _, _)| x == f).count() as u8;
                let stufe = sp.forschung[f.idx()] + geplant + 1;
                if stufe > crate::regeln::MAX_FORSCHUNG {
                    return None;
                }
                let k = r.kosten_forschung(*f, stufe);
                let punkte = r.fp_forschung(*f, stufe);
                Some(json!({"forschung": f.name(), "stufe": stufe, "kosten": gueter(&k, false),
                    "dauer_stunden": if fp > 0 { json!((punkte + fp - 1) / fp) } else { Value::Null },
                    "labor": r.forsch(*f).labor, "labor_heimat": labor_heim,
                    "braucht": if matches!(*f,Forschung::Ueberwachungstechnik|Forschung::Abschirmtechnik){vec!["Spionagetechnik Stufe 1"]}else{vec![]},
                    "infrastruktur":if *f==Forschung::Ueberwachungstechnik{Some("Eigenen Geheimdienst")}else{None},
                    "fehlt": Gut::ALLE.iter().filter(|x| bestand_heim[x.idx()] < k[x.idx()]).map(|x| x.name()).collect::<Vec<_>>()}))
            })
            .collect();
        // Stückkosten der Einheiten, die die Zivilisationsstufe schon erlaubt, mit Werftstufe und weiteren Gebäuden.
        let einheiten_kosten: Vec<Value> = Einheit::ALLE
            .iter()
            .filter(|e| r.einh(**e).ab_stufe <= sp.stufe)
            .map(|e| {
                let er = r.einh(*e);
                json!({"einheit": e.name(), "schiff": e.ist_schiff(), "kosten": gueter(&r.kosten_einheit(*e, sp.volk), false),
                    "bauzeit_je_planet_sekunden":sp.planeten.iter().map(|pid| (self.planeten[*pid as usize].koord.to_string(),self.fertigungszeit(*pid as usize,&r.kosten_einheit(*e,sp.volk),0))).collect::<std::collections::BTreeMap<_,_>>(),
                    "werft": er.werft, "braucht": er.braucht.iter().map(|(b, st)| format!("{b} {st}")).collect::<Vec<_>>()})
            })
            .collect();
        let forschung = json!({
            "stufen": stufen(&Forschung::ALLE, &sp.forschung),
            "aktiv": sp.forschung_aktiv.as_ref().map(|a| json!({"forschung": a.forschung.name(), "stufe": a.stufe,
                "rest_stunden": if fp > 0 { json!((a.fp_rest + fp - 1) / fp) } else { Value::Null },
                "punkte_rest":a.fp_rest,"punkte_gesamt":r.fp_forschung(a.forschung,a.stufe),"rate":fp,
                "begonnen_sekunden":a.begonnen,
                "naechster_tick_sekunden":(self.zeit/STUNDE+1)*STUNDE,
                "fertig_sekunden":if fp>0{Some((self.zeit/STUNDE+(a.fp_rest+fp-1)/fp)*STUNDE)}else{None}})),
            "schlange": sp.forschung_schlange.iter().map(|(f, _, _)| f.name()).collect::<Vec<_>>(),
            "punkte_je_stunde": ganz(fp),
            "moeglich": forschung_moeglich,
        });

        let mut markt = Map::new();
        for g in Gut::ALLE {
            let (brief, geld) = self.marktpreise(g);
            if brief.is_some() || geld.is_some() {
                markt.insert(g.name().to_string(), json!({"verkauf_ab": brief.map(|p| p as f64 / 1000.0), "kauf_bis": geld.map(|p| p as f64 / 1000.0)}));
            }
        }
        let orders: Vec<Value> = self
            .orders
            .iter()
            .filter(|o| o.spieler == sid)
            .map(|o| json!({"order": o.id, "planet": self.planeten[o.planet as usize].koord.to_string(), "gut": o.gut.name(), "seite": o.seite.name(),
                "menge": ganz(o.menge), "preis": o.preis as f64 / 1000.0}))
            .collect();

        let truemmer: Vec<Value> = self
            .truemmer
            .iter()
            .filter(|(k, _)| {
                if self.aufklaerungsregeln() {
                    return sp.planeten.iter().any(|pid|self.planeten[*pid as usize].koord == **k);
                }
                sp.planeten
                    .iter()
                    .any(|e| self.planeten[*e as usize].koord.sektor == k.sektor)
            })
            .take(20)
            .map(
                |(k, t)| json!({"koord": k.to_string(), "erz": ganz(t[0]), "kristall": ganz(t[1])}),
            )
            .collect();

        let toepfe: Map<String, Value> = Topf::ALLE
            .iter()
            .map(|t| {
                (
                    t.name().to_string(),
                    json!({"guthaben": ganz(sp.toepfe[t.idx()]), "anteil": sp.anteile[t.idx()]}),
                )
            })
            .collect();

        let mut view = json!({
            "zeit": zeittext(jetzt),
            "sekunden": jetzt,
            "tag": jetzt / TAG + 1,
            "epoche_tage": r.welt.epoche_tage,
            "name": sp.name,
            "reich_status": self.reich_status(sid),
            "ausgeschiedene": self.ausgeschiedene(),
            "volk": sp.volk.name(),
            "rolle": rolle.name(),
            "stufe": sp.stufe,
            "rang": sp.rang,
            "spielerzahl": self.spieler.len(),
            "punkte": {"gesamt": sp.punkte.gesamt(), "wirtschaft": sp.punkte.wirtschaft, "forschung": sp.punkte.forschung,
                       "militaer": sp.punkte.militaer, "zivilisation": sp.punkte.zivilisation},
            "credits": ganz(sp.credits),
            "steuersatz": sp.steuersatz,
            "anfaengerschutz_bis": if jetzt < sp.schutz_bis { json!(zeittext(sp.schutz_bis)) } else { Value::Null },
            "toepfe": toepfe,
            "naechste_stufe": naechste_stufe,
            "forschung": forschung,
            "einheiten_kosten": einheiten_kosten,
            "planeten": planeten,
            "flotten": flotten,
            "flottenplaetze": {"belegt": self.flotten.values().filter(|f| f.besitzer == sid).count(), "gesamt": self.flottenplaetze(sid)},
            "kolonien": {"anzahl": self.kolonien(sid), "erlaubt": self.kolonien_erlaubt(sid), "verwaltungsgrenze": self.verwaltungsgrenze(sid)},
            "kolonie_weg": self.kolonie_weg(sid),
            "warnungen": warnungen,
            "angriffe": angriffe,
            "raketensalven":self.ereignisse.iter().filter_map(|e| match e.art {EreignisArt::RaketenAnkunft{von,ziel,anzahl,zieltyp} if von==sid || self.belegung.get(&ziel).is_some_and(|p|self.planeten[*p as usize].besitzer==sid)=>Some(json!({"von":name(von),"ziel":ziel.to_string(),"anzahl":anzahl,"zieltyp":zieltyp.name(),"ankunft":zeittext(e.zeit)})),_=>None}).collect::<Vec<_>>(),
            "verbaende":self.flotten.values().filter(|f|f.verband==Some(f.id) && (f.besitzer==sid || self.verbuendet(sid,f.besitzer))).map(|f|json!({"fuehrung":f.id,"spieler":name(f.besitzer),"ziel":f.ziel.to_string(),"ankunft":zeittext(f.ankunft)})).collect::<Vec<_>>(),
            // Lesbar wie ein Kampfbericht im Spiel: Namen statt Spielernummern, Verluste je Einheit.
            "kampfberichte": self.kampfberichte.iter().rev()
                .filter(|b| b.angreifer == sid || b.angreifer_gruppen.iter().any(|g| g.spieler == sid) || b.verteidiger.contains(&sid))
                .take(20)
                .map(|b| {
                    let mut angreifer = vec![name(b.angreifer)];
                    for g in &b.angreifer_gruppen {
                        let n = name(g.spieler);
                        if !angreifer.contains(&n) {
                            angreifer.push(n);
                        }
                    }
                    let verlust = |vorher: &[i64; EINHEITEN], nachher: &[i64; EINHEITEN]| {
                        einheiten(&vorher.iter().zip(nachher).map(|(a, n)| a - n).collect::<Vec<_>>(), 0)
                    };
                    json!({"zeit": zeittext(b.zeit), "ort": b.ort.to_string(), "mission": b.mission.name(),
                        "seite": if b.verteidiger.contains(&sid) { "verteidigung" } else { "angriff" },
                        "angreifer": angreifer, "verteidiger": b.verteidiger.iter().map(|v| name(*v)).collect::<Vec<_>>(),
                        "sieger": match b.sieger { Sieger::Angreifer => "angreifer", Sieger::Verteidiger => "verteidiger", Sieger::Unentschieden => "unentschieden" },
                        "runden": b.runden,
                        "verluste_angreifer": verlust(&b.ang_vorher, &b.ang_nachher),
                        "verluste_verteidiger": verlust(&b.vert_vorher, &b.vert_nachher),
                        "beute": gueter(&b.beute, false),
                        "truemmer": {"erz": ganz(b.truemmer[0]), "kristall": ganz(b.truemmer[1])}})
                })
                .collect::<Vec<_>>(),
            "berichte": sp.berichte.iter().rev().map(|b| self.bericht_sicht(sid,b)).collect::<Vec<_>>(),
            "flottenberichte": self.eigene_flottenberichte(sid),
            "aufklaerungsregeln": self.aufklaerungsregeln(),
            "erkundet": sp.erkundet.iter().map(|(k, _)| {
                let mut v=self.planetenwissen(sid,*k);
                v["frei"]=json!(v["status"]=="frei");
                v
            }).collect::<Vec<_>>(),
            "nachbarn": nachbarn,
            "rangliste": rangliste,
            "vertraege": vertraege,
            "allianz": allianz,
            "einladungen": einladungen,
            "register": register,
            "nachrichten": nachrichten,
            "kommunikation": self.kommunikation_sicht(sid),
            "atlas": self.atlas_sicht(sid),
            "ereignisse": ereignisse,
            "chronik": chronik,
            "markt": {"preise": markt, "orders": orders},
            "truemmer": truemmer,
            "doktrin": sp.doktrin,
            "notiz": if rolle == Rolle::Alle { "" } else { sp.notizen[rolle.idx()].as_str() },
            "meldungen": sp.meldungen.iter().rev().take(10).map(|m| json!({"zeit": zeittext(m.zeit), "von": m.von.name(), "text": m.text})).collect::<Vec<_>>(),
        });
        if self.kolonisationsregeln_v2() {
            let mut deliveries=self.ereignisse.iter().filter_map(|e|match e.art {
                EreignisArt::MarktlieferungGebunden{planet,empfaenger,gut,menge} if empfaenger==sid =>
                    Some(json!({"ziel":self.planeten[planet as usize].koord.to_string(),"gut":gut.name(),
                        "menge":menge as f64/M as f64,"naechste_ankunft":e.zeit,
                        "hinweis":"An Käufer gebunden; Besitzerwechsel oder Blockade kann die Zustellung verzögern."})),
                _=>None,
            }).collect::<Vec<_>>();
            deliveries.sort_by_key(|d| {
                (
                    d["naechste_ankunft"].as_i64().unwrap_or(0),
                    d["ziel"].as_str().unwrap_or("").to_string(),
                    d["gut"].as_str().unwrap_or("").to_string(),
                )
            });
            view["marktlieferungen"] = json!(deliveries);
            view["kampfkolonisationen"] = json!(self.kolonisation.besetzungen.iter().filter_map(|(pid,b)| {
                let f = self.flotten.get(&b.flotte)?;
                if b.verteidiger != sid && f.besitzer != sid { return None; }
                Some(json!({"planet":self.planeten[*pid as usize].koord.to_string(),"flotte":b.flotte,
                    "angreifer":f.besitzer,"verteidiger":b.verteidiger,"seit":b.seit,"fruehestens":b.fruehestens,
                    "abgeschlossene_reaktionsfenster":b.reaktionsfenster,"mindestens_noch_fenster":2u8.saturating_sub(b.reaktionsfenster),
                    "hinweis":"Übernahme erst nach Abschluss der letzten Reaktion und bei fortbestehender Orbitkontrolle."}))
            }).collect::<Vec<_>>());
        }
        view
    }

    /// Lesende Werkzeuge der Agenten.
    pub fn werkzeug(&self, sid: SpielerId, abfrage: &Value) -> Result<Value, String> {
        let r = &self.regeln;
        let sp = &self.spieler[sid as usize];
        let text = |feld: &str| abfrage.get(feld).and_then(|v| v.as_str());
        let koord = |feld: &str| -> Result<Koord, String> {
            text(feld)
                .ok_or_else(|| format!("Feld {feld} fehlt"))?
                .parse::<Koord>()
        };
        let schiffe = || -> Result<[i64; SCHIFFE], String> {
            let mut s = [0i64; SCHIFFE];
            let Some(m) = abfrage.get("schiffe").and_then(|v| v.as_object()) else {
                return Err("Feld schiffe fehlt".into());
            };
            for (n, z) in m {
                let e = Einheit::aus_name(n)
                    .filter(|e| e.ist_schiff())
                    .ok_or_else(|| format!("{n} ist kein Schiff"))?;
                s[e.idx()] = z.as_i64().unwrap_or(0).max(0);
            }
            Ok(s)
        };
        match text("typ").unwrap_or("") {
            "briefarchiv" => {
                let before=abfrage["vor_id"].as_u64().unwrap_or(u64::MAX);
                let limit=abfrage["limit"].as_u64().unwrap_or(50).clamp(1,100) as usize;
                let channel=abfrage["kanal"].as_str().unwrap_or("alle");
                let letters=self.briefkasten_seite(sid,before,limit,channel);
                Ok(json!({"briefe":letters,"naechste_vor_id":letters.last().and_then(|n|n["id"].as_u64())}))
            },
            "kolonieplan" => self.kolonieplan(sid, abfrage),
            "kosten" => {
                let pid = match text("planet") {
                    Some(k) => self.eigener_planet(sid, k.parse()?)?,
                    None => sp.heimat as usize,
                };
                if let Some(a) = text("rakete") {
                    let art=match a {"abfang"=>0,"interplanetar"=>1,_=>return Err("Raketenart muss abfang oder interplanetar sein".into())};
                    let anzahl=abfrage.get("anzahl").and_then(Value::as_i64).unwrap_or(1);
                    let kosten=self.raketen_kosten(art,anzahl)?;
                    Ok(json!({"rakete":a,"anzahl":anzahl,"kosten":gueter(&kosten,false),"bauzeit_sekunden":r.zusatz.raketen_bauzeit_sekunden*anzahl,"kapazitaet":self.planeten[pid].gebaeude[Gebaeude::Raketensilo.idx()] as i64*r.zusatz.silo_plaetze_je_stufe,"belegt":self.raketen_belegt(pid)}))
                } else if let Some(g) = text("gebaeude") {
                    let g = Gebaeude::aus_name(g).ok_or_else(|| format!("Gebäude {g} gibt es nicht"))?;
                    let jetzt = self.planeten[pid].gebaeude[g.idx()];
                    let stufe = abfrage.get("stufe").and_then(|v| v.as_u64()).map(|s| s as u8).unwrap_or(jetzt + 1).clamp(1, r.welt.max_gebaeudestufe);
                    let k = r.kosten_gebaeude(g, stufe);
                    let gr = r.geb(g);
                    let mut quote = json!({"gebaeude": g.name(), "stufe": stufe, "kosten": gueter(&k, false), "bauzeit_min": self.bauzeit(pid, &k) / 60,
                        "ertrag": ganz(r.stufenwert(gr.ertrag, stufe)), "arbeiter": ganz(r.stufenwert(gr.arbeiter, stufe)),
                        "fachkraefte": ganz(r.stufenwert(gr.fachkraefte, stufe)), "strom": ganz(r.stufenwert(gr.energie, stufe)),
                        "ab_zivilisationsstufe": gr.ab_stufe});
                    if self.kolonisationsregeln_v2() && self.integritaet(pid,g)<1000 {
                        let cost = self.reparaturkosten(pid,g)?;
                        quote["reparatur"] = json!({"integritaet_prozent":self.integritaet(pid,g)/10,
                            "kosten":gueter(&cost,false),"dauer_sekunden":self.bauzeit(pid,&cost).max(15*MINUTE),
                            "hinweis":"Eigene Baustelle; kein paralleler Gebäudeausbau auf diesem Planeten."});
                    }
                    Ok(quote)
                } else if let Some(f) = text("forschung") {
                    let f = Forschung::aus_name(f).ok_or_else(|| format!("Forschung {f} gibt es nicht"))?;
                    let stufe = abfrage.get("stufe").and_then(|v| v.as_u64()).map(|s| s as u8).unwrap_or(sp.forschung[f.idx()] + 1).clamp(1, crate::regeln::MAX_FORSCHUNG);
                    let fp = r.fp_forschung(f, stufe);
                    let rate = self.fp_rate(sid);
                    Ok(json!({"forschung": f.name(), "stufe": stufe, "kosten": gueter(&r.kosten_forschung(f, stufe), false),
                        "punkte": ganz(fp), "dauer_stunden": if rate > 0 { json!((fp + rate - 1) / rate) } else { Value::Null },
                        "labor": r.forsch(f).labor, "ab_zivilisationsstufe": r.forsch(f).ab_stufe}))
                } else if let Some(e) = text("einheit") {
                    let e = Einheit::aus_name(e).ok_or_else(|| format!("Einheit {e} gibt es nicht"))?;
                    let k = r.kosten_einheit(e, sp.volk);
                    let er = r.einh(e);
                    Ok(json!({"einheit": e.name(), "kosten": gueter(&k, false), "bauzeit_min": (self.fertigungszeit(pid, &k, 0) + 59) / 60,
                        "besatzung": er.besatzung, "unterhalt_je_tag": ganz(mal(r.wert(&k), r.wirtschaft.unterhalt_schiffe_je_tag)),
                        "ab_zivilisationsstufe": er.ab_stufe, "werft": er.werft}))
                } else {
                    Err("kosten braucht gebaeude, forschung oder einheit".into())
                }
            }
            "flugzeit" => {
                let start = koord("start")?;
                let ziel = koord("ziel")?;
                let pid = self.eigener_planet(sid, start)?;
                let s = schiffe()?;
                let sigma = milli(abfrage.get("geschwindigkeit").and_then(|v| v.as_f64()).unwrap_or(1.0)).clamp(100, 1000);
                let plan = self.flugplan(sid, pid, ziel, &s, sigma)?;
                Ok(json!({"entfernung": plan.entfernung, "dauer_min": plan.dauer / 60, "dauer_sekunden":plan.dauer, "treibstoff_je_strecke": ganz(plan.treibstoff), "treibstoff_je_strecke_milli":plan.treibstoff,
                    "ladekapazitaet": ganz(plan.kapazitaet), "tempo": plan.tempo}))
            }
            "kampfsimulator" => {
                let ziel = koord("ziel")?;
                let s = schiffe()?;
                let Some(b) = sp.berichte.iter().find(|b| b.ziel == ziel) else {
                    return Err(format!("für {ziel} liegt kein Spionagebericht vor"));
                };
                let (Some(bs), Some(bv)) = (&b.schiffe, &b.verteidigung) else {
                    return Err("der Bericht zeigt Schiffe oder Verteidigung nicht: mehr Sonden oder bessere Spionagetechnik".into());
                };
                let mut ang = [0i64; EINHEITEN];
                ang[..SCHIFFE].copy_from_slice(&s);
                let mut vert = [0i64; EINHEITEN];
                vert[..SCHIFFE].copy_from_slice(bs);
                vert[SCHIFFE..].copy_from_slice(bv);
                let a = Gruppe::neu(r, sp, ang);
                // Technik des Verteidigers aus dem Bericht, sonst wie die eigene geschätzt.
                let tech = |f: Forschung| {
                    let stufe = b.forschung.as_ref().map(|t| t[f.idx()]).unwrap_or(sp.forschung[f.idx()]);
                    1.0 + r.kampf.tech_je_stufe * stufe as f64
                };
                let volk_v = self.spieler[b.besitzer as usize].volk;
                let v = Gruppe::mit_werten(r, b.besitzer, vert, tech(Forschung::Waffentechnik)*r.volk(volk_v).waffen, tech(Forschung::Schildtechnik), tech(Forschung::Panzerung)*r.volk(volk_v).panzerung);
                let laeufe = r.kampf.simulator_laeufe.max(1);
                let (mut siege, mut patt, mut verlust_a, mut verlust_v) = (0u32, 0u32, 0i64, 0i64);
                let wert = |e: usize, n: i64, volk:Volk| n * r.wert(&r.kosten_einheit(Einheit::ALLE[e], volk));
                for i in 0..laeufe {
                    let mut rng = strom(self.startwert, KANAL_SIMULATOR, (self.zeit as u64) << 20 | (sid as u64) << 8 | i as u64);
                    let erg = kampf(r, std::slice::from_ref(&a), std::slice::from_ref(&v), &mut rng);
                    match erg.sieger {
                        Sieger::Angreifer => siege += 1,
                        Sieger::Unentschieden => patt += 1,
                        Sieger::Verteidiger => {}
                    }
                    for e in 0..EINHEITEN {
                        verlust_a += wert(e, ang[e] - erg.angreifer[0][e], sp.volk);
                        verlust_v += wert(e, vert[e] - erg.verteidiger[0][e], volk_v);
                    }
                }
                let quote = r.volk(sp.volk).pluenderquote.unwrap_or(r.kampf.pluenderquote);
                let schutz = b.gebaeude.as_ref().map(|g| r.bunkerschutz(g[Gebaeude::Bunker.idx()]));
                let mut beute = [0i64; GUETER];
                for g in 0..8 {
                    beute[g] = mal((b.bestand[g] - schutz.map(|s| s[g]).unwrap_or(0)).max(0), quote);
                }
                Ok(json!({"laeufe": laeufe, "siegchance_prozent": siege * 100 / laeufe, "unentschieden_prozent": patt * 100 / laeufe,
                    "eigene_verluste_wert": ganz(verlust_a / laeufe as i64), "verluste_gegner_wert": ganz(verlust_v / laeufe as i64),
                    "beute_bei_sieg_ohne_ladegrenze": gueter(&beute, false), "bericht_alter_stunden": (self.zeit - b.zeit) / STUNDE,
                    "technik_des_gegners": if b.forschung.is_some() { "aus dem Bericht" } else { "geschätzt wie die eigene" }}))
            }
            "regel" => {
                let query=text("stichwort").unwrap_or("");
                let colony=self.kolonisation.aktiv && ["kolon","erober","invasion","repar","bombard"].iter().any(|q|query.to_lowercase().contains(q));
                if self.kolonisationsregeln_v2() {
                    let base=if colony {String::new()}else{regeltext::nachschlagen(r,query)};
                    Ok(json!({"text":format!("{base}\nMaßgebliche neue Kolonisations-/Flottenregeln (ersetzen alte Invasion): {}",crate::kolonisation::REGELTEXT_V2),"kolonie_startfracht":gueter(&self.koloniefracht(sid),true)}))
                } else if colony {Ok(json!({"text":crate::kolonisation::REGELTEXT,"kolonie_startfracht":gueter(&self.koloniefracht(sid),true)}))}
                else {Ok(json!({"text":regeltext::nachschlagen(r,query)}))}
            },
            "galaxie" => {
                let sektor = abfrage.get("sektor").and_then(Value::as_u64).unwrap_or(1);
                let von = abfrage.get("von").and_then(Value::as_u64).unwrap_or(1);
                let bis = abfrage.get("bis").and_then(Value::as_u64).unwrap_or(von.saturating_add(19)).min(von.saturating_add(19)).min(r.welt.systeme_je_sektor as u64);
                if sektor == 0 || sektor > r.welt.sektoren as u64 || von == 0 || von > r.welt.systeme_je_sektor as u64 || bis < von {
                    return Err("Koordinaten außerhalb der Galaxie".into());
                }
                let mut systeme = Vec::new();
                for n in von..=bis {
                    let sys = self.system(Koord::neu(sektor as u8, n as u8, 1)).unwrap();
                    let plaetze: Vec<Value> = (1..=r.welt.plaetze_je_system).map(|p|self.planetenwissen(sid,Koord::neu(sektor as u8,n as u8,p))).collect();
                    let belegt: Vec<Value> = plaetze.iter().filter(|p| !p["spieler"].is_null()).cloned().collect();
                    let known = !self.aufklaerungsregeln() || self.system_erfasst(sid,Koord::neu(sektor as u8,n as u8,1));
                    systeme.push(json!({"system":n,"bekannt":known,
                        "nebel":if known {json!(sys.nebel)} else {Value::Null},
                        "asteroidenguertel":if known {json!(sys.guertel)} else {Value::Null},"belegt":belegt,"plaetze":plaetze}));
                }
                Ok(json!({"sektor": sektor, "systeme": systeme}))
            }
            anderes => Err(format!("Werkzeug '{anderes}' gibt es nicht: kosten, flugzeit, kampfsimulator, regel, galaxie")),
        }
    }
}
