//! Was ein Spieler sieht: das Lagebild als Daten und die lesenden Werkzeuge.
//!
//! Die Informationsgrenze liegt hier in der Engine: ein Lagebild enthält nur die eigene
//! Lage, öffentliche Daten und das, was der Spieler selbst herausgefunden hat.


use crate::regeltext;
use crate::typen::*;
use crate::welt::*;
use serde_json::{json, Map, Value};

pub(crate) fn gueter_exakt(a:&[i64;GUETER])->Value{json!(Gut::ALLE.iter().filter(|g|a[g.idx()]!=0).map(|g|(g.name(),a[g.idx()] as f64/M as f64)).collect::<std::collections::BTreeMap<_,_>>())}

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
    for (a, n) in alle.iter().zip(werte) {
        if *n > 0 {
            m.insert(a.to_string(), json!(n));
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
        if self.ausbau.industrie_version>0 {
            let rest=self.ausbau.bestandsrest.get(&p.id).copied().unwrap_or([0;GUETER]);
            crate::planer::bestand::fortschreiben_mit_rest(&p.bestand,&p.rate,&grenze,dt,&rest).0.bestand
        }else{crate::planer::bestand::fortschreiben(&p.bestand, &p.rate, &grenze, dt).bestand}
    }

    pub(crate) fn planet_sicht(&self, pid: usize) -> Value {
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
                    "pausiert":self.bau_ruht_unruhen(pid)||self.auftrag_pausiert("bau",p.id),"manuell_pausiert":self.auftrag_pausiert("bau",p.id)}),
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
                let batches=self.ausbau.parallel.get(&crate::parallel::key(pid,i)).map(|lots|lots.iter().filter(|b|b.auftrag==j).cloned().collect::<Vec<_>>()).unwrap_or_default();
                let end=if self.ausbau.industrie_version>0{batches.iter().map(|b|b.ende).min()}else if j==0{Some(p.fertigung_naechste[i])}else{None};
                let naechstes = if j == 0 {
                    (p.fertigung_naechste[i] - jetzt).max(0) / 60
                } else {
                    f.dauer / 60
                };
                // Warum die Werft ruht (Einsatzprobe 25.10.2026: die Karte sagte nur „Versorgung / Zustand prüfen“).
                let ruht_grund = {
                    let g = if i == 0 { Gebaeude::Werft } else { Gebaeude::Orbitalwerft };
                    let pid = p.id as usize;
                    if self.ausbau.industrie_version == 0 || !self.werk_ruht(pid, g) { None } else {
                        let (e, b) = self.werk_kern(pid);
                        Some(if e < 1000 { format!("Energie fehlt (Deckung {} %)", e / 10) }
                            else if b[g.idx()] < 1000 {
                                // Engpass aus der Werksbilanz benennen (Funktionsprüfung 25.10.2026: „Arbeiter fehlen“ stand auch,
                                // wenn Fachkräfte knapp waren oder das Werk beschädigt war).
                                let pers = |v: i64| (v + 500) / 1000;
                                let teile: Vec<String> = self.werksbilanz(pid, g).map(|w| w.engpaesse.iter().map(|e| match e.as_str() {
                                    "arbeiter" => format!("Arbeiter {} von {}", pers(w.arbeiter_zugewiesen), pers(w.arbeiter_bedarf)),
                                    "fachkraefte" => format!("Fachkräfte {} von {}", pers(w.fachkraefte_zugewiesen), pers(w.fachkraefte_bedarf)),
                                    "integritaet" => format!("beschädigt, Integrität {} %", w.integritaet_promille / 10),
                                    x => x.to_string(),
                                }).collect()).unwrap_or_default();
                                if teile.is_empty() { format!("Besetzung {} %, Werke laufen erst voll besetzt", b[g.idx()] / 10) }
                                else { format!("Besetzung {} % – {}", b[g.idx()] / 10, teile.join(", ")) }
                            }
                            else { "Gebäude beschädigt oder stillgelegt".to_string() })
                    }
                };
                fertigung.push(json!({"schleife": name, "produkt": was, "rest": f.rest, "naechstes_in_min": naechstes,
                    "wartet":end.is_none(),"fertig_sekunden":end,"parallel":batches.iter().map(|b|b.anzahl).sum::<i64>(),"lose":batches,"plaetze_je_stueck":crate::parallel::platzbedarf(f.produkt),"dauer_sekunden":f.dauer,
                    "pausiert":self.auftrag_pausiert(if i==0{"werft"}else{"bauteile"},p.id),"manuell_pausiert":self.ausbau.pausen.contains(&format!("{}:{}",if i==0{"werft"}else{"bauteile"},p.id)),
                    "ruht_grund":ruht_grund}));
            }
        }
        // Nächste Stufe jedes Gebäudes, das hier je gebaut werden kann, mit Kosten, Bauzeit und dem, was noch
        // fehlt. Dieselben Prüfungen wie beim Bauen; spart dem Verwalter eine Kostenabfrage je Gebäude.
        let stufe_sp = self.spieler[p.besitzer as usize].stufe;
        let nebel = self.system(p.koord).map(|s| s.nebel).unwrap_or(false);
        let mut baubar = Vec::new();
        // Jeder Bau belegt ein Feld (bauen lehnt ab, sobald alle belegt sind): sonst würden Spieler und KI-Agenten
        // unmögliche Bauten angeboten bekommen.
        let felder_voll = self.felder_belegt(pid) >= self.felder_gesamt(pid);
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
                || (g == Gebaeude::Xenoextraktor && !nebel && self.ausbau.version==0)
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
            if felder_voll {
                braucht.push(format!("Felder voll ({}/{})", self.felder_belegt(pid), self.felder_gesamt(pid)));
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
            .map(|(m, art)| {let factor=if self.ausbau.industrie_version>0&&matches!(g,Gebaeude::Erzmine|Gebaeude::Kristallmine|Gebaeude::Deuteriumsynthesizer|Gebaeude::Farm|Gebaeude::Giesserei|Gebaeude::Elektronikwerk|Gebaeude::Konsumgueterwerk|Gebaeude::Xenoextraktor){2}else{1};if g==Gebaeude::Akademie&&self.ausbau.version>0{json!({"plus":20,"art":"gleichzeitige Ausbildungsplätze"})}else{json!({"plus":ganz(m*factor),"art":art})}});
            baubar.push(json!({"gebaeude": g.name(), "stufe": naechste, "kosten": gueter(&k, false), "bauzeit_min": self.bauzeit(pid, &k) / 60, "bauzeit_sekunden":self.bauzeit(pid,&k),
                "fehlt": fehlt, "braucht": braucht, "ertrag": ertrag, "strom_plus": ganz(plus(gr.energie)),
                "arbeiter_plus": ganz(plus(gr.arbeiter)), "fachkraefte_plus": ganz(plus(gr.fachkraefte)), "kostenfaktor": gr.faktor}));
        }
        let blockade = p.blockade.and_then(|fid| self.flotten.get(&fid)).map(|f| {
            json!({"durch": self.spieler[f.besitzer as usize].name, "art": f.mission.name(), "flotte": f.id})
        });
        let labor_bilanz = if self.ausbau.industrie_version > 0 && p.gebaeude[Gebaeude::Labor.idx()] > 0 { self.werksbilanz(pid, Gebaeude::Labor) } else { None };
        let mut view = json!({
            "koord": p.koord.to_string(),
            "heimat": p.heimat,
            "zone": if self.ausbau.version>0 {crate::habitabilitaet::zone(p.koord.position,r.welt.plaetze_je_system).name()}else{p.zone.name()},
            "nebel": self.system(p.koord).map(|s| s.nebel).unwrap_or(false),
            "felder": {"belegt": self.felder_belegt(pid), "gesamt": self.felder_gesamt(pid)},
            "faktoren": {"erz": p.faktor[F_ERZ], "kristall": p.faktor[F_KRISTALL], "deuterium": if self.ausbau.version>0 {crate::habitabilitaet::faktoren(r,p)[F_DEUTERIUM]} else {p.faktor[F_DEUTERIUM]},
                         "nahrung": if self.ausbau.version>0 {crate::habitabilitaet::faktoren(r,p)[F_NAHRUNG]} else {p.faktor[F_NAHRUNG]}, "solar": if self.ausbau.version>0 {crate::habitabilitaet::solar_r(r,p.koord)} else {p.faktor[F_SOLAR]}},
            "bevoelkerung": ganz(p.bevoelkerung),
            "personengruppen":self.ausbau.personen.get(&p.id),
            "fertigungsplaetze":{"werft":self.parallel_sicht(pid,0),"orbitalwerft":self.parallel_sicht(pid,1)},
            "industrie":if self.ausbau.industrie_version>0{self.industrie_sicht(pid)}else{Value::Null},
            "bodentruppen":if self.ausbau.industrie_version>0{self.boden_sicht(pid)}else{Value::Null},
            "produktionsserien":self.produktionsserien_sicht(pid),
            "habitabilitaet":self.klima_sicht(p.besitzer,p.koord),
            "ueberbelegung":ganz((p.bevoelkerung-p.wohnraum).max(0)),
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
            // fk_bedarf (Spielstand) enthält den Forscherbedarf des Labors; die Sicht trennt ihn ab (Funktionsprüfung 25.10.2026:
            // „14 / 137 Fachkräfte“ waren 137 Forscherplätze, die Grundausbildung sagte zugleich „Bedarf gedeckt“).
            "forscher": labor_bilanz.as_ref().map(|w| json!({"zugewiesen": ganz(w.fachkraefte_zugewiesen), "bedarf": ganz(w.fachkraefte_bedarf)})),
            "fachkraefte": {"verfuegbar": ganz(p.fk_verfuegbar), "bedarf": ganz(p.fk_bedarf - labor_bilanz.as_ref().map_or(0, |w| w.fachkraefte_bedarf)),
                "zaehlt": self.fruehphase_faktor(p.besitzer) == 1, "ab_stufe": crate::planer::wirtschaft::PERSONAL_AB_STUFE},
            "akademie": self.akademie_sicht(pid),
            "grundausbildung": self.grundausbildung_sicht(pid),
            "wachstum": if self.ausbau.version>0 { self.wachstum_prognose(pid) } else { serde_json::Value::Null },
            "nahrung_deckung": p.nahrung_deckung / 10,
            "konsum_deckung": p.konsum_deckung / 10,
            "gebaeude": stufen(&Gebaeude::ALLE, &p.gebaeude),
            "gebaeude_integritaet_prozent": Gebaeude::ALLE.iter().filter(|g|p.gebaeude[g.idx()]>0).map(|g|(g.name(),self.integritaet(pid,*g)/10)).collect::<std::collections::BTreeMap<_,_>>(),
            // Beschädigte Gebäude: Kosten der Reparatur (Bauseite zeigt dort „Reparieren“ statt „Bauen“).
            "reparaturkosten": Gebaeude::ALLE.iter().filter(|g|p.gebaeude[g.idx()]>0&&self.integritaet(pid,**g)<1000)
                // aufgerundet: der Kern bucht in Tausendsteln – abgerundet war der Knopf aktiv, die Abbuchung lehnte „fehlen 1“ ab
                .filter_map(|g|self.reparaturkosten(pid,*g).ok().map(|k|(g.name(),json!(Gut::ALLE.iter().filter(|x|k[x.idx()]>0).map(|x|(x.name(),(k[x.idx()]+M-1)/M)).collect::<std::collections::BTreeMap<_,_>>()))))
                .collect::<std::collections::BTreeMap<_,_>>(),
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
            "raketensilo":self.silo_sicht(pid),
            "raketen": {"abfang":p.raketen[0],"interplanetar":p.raketen[1],"belegt_mit_bau":self.raketen_belegt(pid),"kapazitaet":p.gebaeude[Gebaeude::Raketensilo.idx()] as i64*r.zusatz.silo_plaetze_je_stufe},
            "raketenbau": self.ereignisse.iter().filter_map(|e| match e.art { EreignisArt::RaketenFertig{planet,besitzer,art,anzahl} if planet==p.id && besitzer==p.besitzer=>Some(json!({"art":crate::raketen::regeln(&self.regeln)[art as usize].key,"anzahl":anzahl,"fertig":zeittext(e.zeit)})),_=>None }).collect::<Vec<_>>(),
            "garnison": self.garnison(pid),
            "blockade": blockade,
        });
        if self.invasion_v1() { view["invasion"] = self.invasion_planet_sicht(pid); }
        // Terraformer-Schiffe (0.4.7): Stand und noch nötige Schiffe bei der aktuellen Forschungsstufe.
        if self.terraformer_welt() { view["terraform"] = self.terraform_planet_sicht(pid); }
        // Selbst gewählter Name (`planet_benennen`); fehlt = Standardname, die Sicht bleibt dann unverändert.
        if let Some(n) = self.planetenname(pid) { view["name"] = json!(n); }
        // Energie eines Solarsatelliten über diesem Planeten (Sonnenfaktor der Bahn, Energietechnik, Volk, Dyson-Sphäre):
        // vorher nur am Mond angezeigt, die Bauseite konnte den Nutzen über Planeten nicht nennen.
        if let Some(m) = r.monde.as_ref() {
            let sp = &self.spieler[p.besitzer as usize];
            let je = crate::monde::solarsonde_energie_an(r, m, p.koord, sp.forschung[Forschung::Energietechnik.idx()], r.volk(sp.volk).energie, 0.0);
            let je = match self.dyson_wirkung(p) { Some(d) => mal(je, d.sonnenfaktor), None => je };
            view["energie"]["je_solarsonde"] = json!(je as f64 / M as f64);
        }
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
        let werft = r.einh(schiff).werft.max(1);
        let stufe = r
            .stufen
            .get((ab as usize).saturating_sub(2))
            .map(|s| s.name.clone())
            .unwrap_or_default();
        let erlaubt = self.kolonien_erlaubt(sid);
        // Neue Welten (0.4.7): freie Kolonieplätze ab Start, ab dann je Stufe der Kolonieforschung (Terraforming) einer.
        let platz_text = if r.kw().kolonien.ist_standard() {
            format!("Astrophysik erlaubt eine weitere Kolonie (jetzt {anzahl} von {erlaubt})")
        } else {
            let frei = self.kolonien_frei();
            match self.kolonie_stufe_fuer(anzahl + 1) {
                Some(n) if n > 0 => format!("{} Stufe {n} erlaubt die {}. Kolonie ({frei} Plätze frei ab Start; jetzt {anzahl} von {erlaubt})",
                    crate::regeltext::forschung_name(self.kolonie_forschung()), anzahl + 1),
                _ => format!("Freier Kolonieplatz ({frei} frei ab Start; jetzt {anzahl} von {erlaubt})"),
            }
        };
        let schritte = [
            (format!("Zivilisationsstufe {stufe}"), sp.stufe >= ab),
            (
                platz_text,
                erlaubt > anzahl,
            ),
            (
                "Raumhafen auf dem Startplaneten".to_string(),
                hat(Gebaeude::Raumhafen),
            ),
            (
                // Neue Welten: das Kolonieschiff entsteht in der Werft (Karl 24.10.2026: die Habitatmodule liegen ab Tag 1 im
                // Lager – kein Umbau-Schritt, solange welche da sind).
                if kern>0 {"Orbitalwerft".to_string()}else{format!("Werft {werft} (baut das Kolonieschiff)")},
                if kern>0 {hat(Gebaeude::Orbitalwerft)}else{planeten.iter().any(|&p|self.planeten[p].gebaeude[Gebaeude::Werft.idx()]>=werft)},
            ),
            (
                if kern>0 {format!(
                    "Benötigte Bauteile: {} Antriebskern, {} Habitatmodul",
                    ganz(kern),
                    ganz(habitat)
                )} else if bestand(Gut::Habitatmodul) >= habitat {
                    format!("Habitatmodul im Lager: {} (je Kolonieschiff {})", crate::schiffhandel::credits_text(ganz(bestand(Gut::Habitatmodul))), ganz(habitat))
                } else {
                    format!("Habitatmodul fehlt (je Kolonieschiff {}): in der Werft einen Recycler umbauen (ab Stufe III, Werft 3)", ganz(habitat))
                },
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
                p.einheiten[Einheit::Kolonieschiff.idx()]+p.einheiten[Einheit::GrossesKolonieschiff.idx()] > 0
                    && Einheit::ALLE[..SCHIFFE].iter().any(|e| {
                        *e != Einheit::Spionagesonde
                            && *e != Einheit::Kolonieschiff && *e != Einheit::GrossesKolonieschiff
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
            out.push(json!({"text":"Aufbau der neuen Kolonie vorab mit dem Kolonieplan prüfen: Mindestfracht allein garantiert keine Selbstversorgung","erledigt":false,"hinweis":true}));
        }
        json!(out)
    }

    pub(crate) fn bericht_sicht(&self, sid: SpielerId, b: &Spionagebericht) -> Value {
        json!({
            "ziel": b.ziel.to_string(),
            "zeit": b.zeit,
            "besitzer": self.spieler[b.besitzer as usize].name,
            "volk": self.ausbau.scan_details.get(&format!("{sid}:{}",b.ziel)).filter(|v|v["zeit"]==b.zeit).and_then(|v|v["metadaten"]["volk"].as_str()).unwrap_or(self.spieler[b.besitzer as usize].volk.name()),
            "scan_metadaten":self.ausbau.scan_details.get(&format!("{sid}:{}",b.ziel)).filter(|v|v["zeit"]==b.zeit).and_then(|v|v.get("metadaten")),
            "alter_stunden": (self.zeit - b.zeit) / STUNDE,
            "bestand": Gut::ALLE.iter().enumerate().map(|(i,g)| {
                if self.ausbau.version>0 {return (g.name(),ganz(b.bestand[i]));}
                let span=self.scans.get(&(sid,b.ziel)).filter(|s|s.zeit==b.zeit).and_then(|s|s.bestand.get(i)).copied()
                    .unwrap_or_else(||crate::sternkarte::intervall(ganz(b.bestand[i]),10000));
                (g.name(),(span[0]+span[1])/2)
            }).collect::<std::collections::BTreeMap<_,_>>(),
            "geschaetzt": self.ausbau.version==0,
            "vollscan":self.ausbau.scan_details.get(&format!("{sid}:{}",b.ziel)).filter(|v|v["zeit"]==b.zeit&&v.get("bevoelkerung").is_some()).map(|v|v.as_object().unwrap().iter().filter(|(k,_)|k.as_str()!="metadaten").map(|(k,v)|(k.clone(),v.clone())).collect::<serde_json::Map<_,_>>()),
            "historisch":true,
            "sonden": self.ausbau.scan_details.get(&format!("{sid}:{}",b.ziel)).filter(|v|v["zeit"]==b.zeit).and_then(|v|v.get("sonden")).cloned(),
            "abwehrchance_promille": self.ausbau.scan_details.get(&format!("{sid}:{}",b.ziel)).filter(|v|v["zeit"]==b.zeit).and_then(|v|v.get("abwehrchance_promille")).cloned(),
            "abgeschossen": self.ausbau.scan_details.get(&format!("{sid}:{}",b.ziel)).filter(|v|v["zeit"]==b.zeit).and_then(|v|v.get("abgeschossen")).cloned(),
            "orbit": self.ausbau.scan_details.get(&format!("{sid}:{}",b.ziel)).filter(|v|v["zeit"]==b.zeit).and_then(|v|v.get("orbit")).cloned(),
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
            .map(|pid| {
                let mut v = self.planet_sicht(*pid as usize);
                // Mondgeneration: Mondplatz in der Planetenliste (Felder wie in der Galaxie).
                if let Some(m) = self.monde_regeln().and_then(|_| self.mond_kartensicht(sid, self.planeten[*pid as usize].koord)) { v["mond"] = m; }
                v
            })
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
                "Feindliche Flotte von {} erreicht {} um {}; Schiffszahl: {}",
                info["start"].as_str().unwrap_or("?"), info["ziel"].as_str().unwrap_or(""), info["ankunft"].as_str().unwrap_or(""),
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
                    "zustand": zustand, "abflug_sekunden":f.abflug,"ankunft_sekunden":f.ankunft,"rueckkehr_sekunden":f.rueckkehr,"orbit_ende_sekunden":f.orbit_ende,"rueckflug_dauer_sekunden":self.rueckdauer(f.id,f.flugdauer),"haltedauer_sekunden":f.haltedauer,"kein_rueckflug":self.ausbau.rueckflugplaene.get(&f.id).is_some_and(|r|r.tempo==0.0),
                    "fortschritt": if self.kolonisation.rueckwege.contains_key(&f.id) {Value::Null} else {json!(match f.zustand {
                        Flottenzustand::Hinflug => ((jetzt-f.abflug) as f64 / f.flugdauer.max(1) as f64).clamp(0.,1.),
                        Flottenzustand::ImOrbit => 1.,
                        Flottenzustand::Rueckflug => ((f.rueckkehr-jetzt) as f64 / f.flugdauer.max(1) as f64).clamp(0.,1.),
                    })}, "bis": if wann > 0 { json!(zeittext(wann)) } else { Value::Null },
                    "bodentruppen":crate::boden::NAMEN.iter().enumerate().map(|(i,n)|(*n,self.ausbau.boden.flotten.get(&f.id).map(|t|t[i]).unwrap_or(0))).collect::<std::collections::BTreeMap<_,_>>(),"schiffe": einheiten(&f.schiffe, 0), "ladung": gueter(&f.ladung, false),"verband":f.verband,"verbandsfuehrung":f.verband==Some(f.id)});
                let mut track=self.ausbau.fluganzeigen.get(&f.id).cloned().unwrap_or(json!({}));
                // Saved one-way probes also return; their original fuel payment stays unchanged.
                if f.mission==Mission::Spionage {track["einweg"]=json!(false);}
                let retour=self.ausbau.rueckflugplaene.get(&f.id);
                if track.get("rueck_tempo_prozent").is_none(){track["rueck_tempo_prozent"]=retour.map(|p|json!(p.tempo*100.0)).unwrap_or_else(||track["hin_tempo_prozent"].clone());}
                track["treibstoff_rueck"]=retour.map(|p|json!(p.treibstoff as f64/M as f64)).unwrap_or_else(||if track["einweg"]==true||self.ausbau.einwegsonden.contains(&f.id){json!(0)}else{track["treibstoff_hin"].clone()});
                track["aktuelle_dauer"]=json!(match f.zustand{Flottenzustand::Hinflug=>f.ankunft-f.abflug,Flottenzustand::ImOrbit=>(f.orbit_ende-f.ankunft).max(0),Flottenzustand::Rueckflug=>track["rueck_dauer"].as_i64().unwrap_or(self.rueckdauer(f.id,f.flugdauer))});
                let route_start=self.kolonisation.rueckwege.get(&f.id).map(|r|self.planeten[r.ziel as usize].koord).unwrap_or(f.start_koord);
                let route_target=if track["umgeleitet"]==true {track["rueck_von"].as_str().and_then(|s|s.parse::<Koord>().ok()).unwrap_or(f.ziel)}else{f.ziel};
                let endpoint=|k:Koord|{let p=self.planetenwissen(sid,k);json!({"koord":k.to_string(),"bekannt":p["bekannt"],"spieler":p["spieler"],"zone":p["zone"],"status":p["status"],"nebel":p["xeno_vorkommen"]})};
                view["strecke"]=json!({"entfernung":self.entfernung(route_start,route_target),"start":endpoint(route_start),"ziel":endpoint(route_target)});
                view["flugdetails"]=track;
                if let Some(o)=self.ausbau.orbitflotten.get(&f.id) {
                    view["gestrandet"]=json!({"seit":o.seit,"nur_heim":o.nur_heim,"deuterium":ganz(f.ladung[Gut::Deuterium.idx()]),"ort":f.ziel.to_string()});
                }
                if let Some(rz)=self.ausbau.rueckziele.get(&f.id) {
                    view["rueckziel"]=match rz { crate::orbitflotten::Rueckziel::Stranden=>json!({"art":"stranden"}), crate::orbitflotten::Rueckziel::Orbit{ort,nur_heim}=>json!({"art":"orbit","ort":ort.to_string(),"nur_heim":nur_heim}) };
                }
                if let Some(z)=self.ausbau.zielflotten.get(&f.id) { view["zielflotte"]=json!(z); }
                view["passagiere"]=json!(self.ausbau.passagiere.get(&f.id).map(|p|ganz(p.gesamt())).unwrap_or(0));
                view["siedler"]=json!(ganz(f.siedler));
                let capacity=mal((0..SCHIFFE).map(|i|f.schiffe[i]*r.ladung_milli(Einheit::ALLE[i],sp.volk)).sum(),r.volk(sp.volk).ladung*(1.0+r.flug.logistik_bonus*sp.forschung[Forschung::Logistik.idx()] as f64));
                view["frachtraum"]=json!({"gesamt":ganz(capacity),"belegt":ganz(f.ladung.iter().sum()),"frei":ganz((capacity-f.ladung.iter().sum::<i64>()).max(0))});
                let mut units=[0;EINHEITEN];units[..SCHIFFE].copy_from_slice(&f.schiffe);let group=crate::kampf::Gruppe::neu(&self.regeln,sp,units);
                view["kampfkraft"]=json!({"angriff":(0..SCHIFFE).map(|i|f.schiffe[i]*group.angriff[i]).sum::<i64>(),"schild":(0..SCHIFFE).map(|i|f.schiffe[i]*group.schild[i]).sum::<i64>(),"struktur":(0..SCHIFFE).map(|i|f.schiffe[i]*group.struktur[i]).sum::<i64>()});
                if self.kolonisationsregeln_v2() {
                    if let Some(route)=self.kolonisation.rueckwege.get(&f.id) {
                        view["rueckkehr"]=json!({"ziel":self.planeten[route.ziel as usize].koord.to_string(),
                            "wartend":route.wartend,"grund":if route.wartend{Some("Landung durch feindliche Blockade gesperrt; Flotte belegt weiter einen Platz und kostet Unterhalt")}else{None}});
                    }
                    if let Some(target)=self.kolonisation.transportziele.get(&f.id) {
                        view["gebundener_empfaenger"]=match target {
                            crate::kolonisation::Transportziel::Planet{planet,empfaenger}=>json!({"art":"planet","planet":self.planeten[*planet as usize].koord.to_string(),"spieler":self.spieler[*empfaenger as usize].name}),
                            crate::kolonisation::Transportziel::Flotte{flotte,empfaenger}=>json!({"art":"flotte","flotte":flotte,"spieler":self.spieler[*empfaenger as usize].name}),
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
                        "menge_je_tag": ganz(v.tribut_menge),
                        // Laufzeit: im Angebot die vereinbarten Tage, danach das Ende (Nachprüfung 25.10.2026: Dauer ging verloren).
                        "tage": if matches!(v.status, Vertragsstatus::Angeboten | Vertragsstatus::Abgelehnt) { json!(v.tribut_bis / TAG) } else { Value::Null },
                        "endet": if matches!(v.status, Vertragsstatus::Angeboten | Vertragsstatus::Abgelehnt) { Value::Null } else { json!(zeittext(v.tribut_bis)) }});
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
            // Nur ausführbare Beitritte: wer schon in einer Allianz ist, sieht keine Einladungen (Funktionsprüfung 25.10.2026).
            .filter(|a| a.eingeladen.contains(&sid) && sp.allianz.is_none())
            .map(|a| a.name.clone())
            .collect();

        let grenze = jetzt - r.agenten.chronik_tage * TAG;
        let nachrichten: Vec<Value> = self.briefkasten(sid).into_iter().filter(|n|n["sekunden"].as_i64().unwrap_or(0)>=grenze).take(30).map(|mut n|{n["neu"]=json!(n["gelesen"]==false && n["gesendet"]==false);n}).collect();

        // Menschen (Rolle::Alle) sehen mehr Meldungen als KI-Rollen, deren Lagebild klein bleiben muss; 40 Einträge
        // verdrängten in aktiven Reichen Erkundungs- und Kampfmeldungen binnen Minuten (Berichte „ohne Lücken“).
        let fenster = if rolle == Rolle::Alle { 150 } else { 40 };
        let ereignisse: Vec<Value> = sp
            .vorfaelle
            .iter()
            .filter(|v| v.zeit >= seit)
            .rev()
            .take(fenster)
            .map(|v| json!({"zeit": zeittext(v.zeit), "art": v.art, "text": v.text}))
            .collect();
        // Chronik: wichtige Meldungen vor dem gezeigten Fenster. Für Rolle::Alle ist `seit` 0 – vorher blieb die
        // Chronik dadurch im Spiel immer leer.
        // KI-Rollen: Was seit dem letzten Aufruf über das Fenster hinausging, fiel vorher aus Ereignissen UND Chronik
        // (Chronik endete bei `seit`). Jetzt beginnt die Chronik unterhalb des gezeigten Fensters, wichtige Meldungen
        // bleiben so sichtbar; `ereignisse_ausgelassen` nennt die Zahl der übrigen.
        let seit_letztem = sp.vorfaelle.iter().filter(|v| v.zeit >= seit).count();
        let chronik_bis = if rolle == Rolle::Alle {
            sp.vorfaelle.iter().rev().nth(fenster).map_or(SimZeit::MIN, |v| v.zeit + 1)
        } else if seit_letztem > fenster {
            sp.vorfaelle.iter().rev().nth(fenster).map_or(seit, |v| v.zeit + 1)
        } else {
            seit
        };
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
            .filter(|v| v.zeit < chronik_bis && wichtig.contains(&v.art.as_str()))
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
                "hinweise": self.stufen_hinweise(sid),
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
                // Unerfüllte Voraussetzungen wie beim Befehl `forschen` (Nachprüfung 25.10.2026: Geheimdienst fehlte in der Sicht).
                let mut voraus = Vec::<String>::new();
                if matches!(*f, Forschung::Ueberwachungstechnik | Forschung::Abschirmtechnik) && sp.forschung[Forschung::Spionagetechnik.idx()] == 0 { voraus.push("Spionagetechnik Stufe 1".into()); }
                if *f == Forschung::Ueberwachungstechnik && !sp.planeten.iter().any(|p| self.planeten[*p as usize].gebaeude[Gebaeude::Geheimdienst.idx()] > 0) { voraus.push("Eigenen Geheimdienst".into()); }
                if *f == Forschung::Plasmatechnik { voraus.extend(self.plasmatechnik_fehlt(sid)); }
                Some(json!({"forschung": f.name(), "stufe": stufe, "kosten": gueter(&k, false),
                    "dauer_stunden": if fp > 0 { json!((punkte + fp - 1) / fp) } else { Value::Null },
                    "labor": r.forsch(*f).labor, "labor_heimat": labor_heim,
                    "braucht": if matches!(*f,Forschung::Ueberwachungstechnik|Forschung::Abschirmtechnik){vec!["Spionagetechnik Stufe 1".to_string()]}else if *f==Forschung::Plasmatechnik{self.regeln.technik.as_ref().map(|t|t.plasmatechnik_braucht.iter().map(|(f,n)|format!("{f} Stufe {n}")).collect()).unwrap_or_default()}else{vec![]},
                    "infrastruktur":if *f==Forschung::Ueberwachungstechnik{Some("Eigenen Geheimdienst")}else{None},
                    "voraussetzungen_fehlen": voraus,
                    "fehlt": Gut::ALLE.iter().filter(|x| bestand_heim[x.idx()] < k[x.idx()]).map(|x| x.name()).collect::<Vec<_>>()}))
            })
            .collect();
        // Stückkosten der Einheiten, die die Zivilisationsstufe schon erlaubt, mit Werftstufe und weiteren Gebäuden.
        let einheiten_kosten: Vec<Value> = Einheit::ALLE
            .iter()
            .filter(|e| r.einheiten.contains_key(e) && r.einh(**e).ab_stufe <= sp.stufe)
            .map(|e| {
                let er = r.einh(*e);
                json!({"einheit": e.name(), "schiff": e.ist_schiff(), "nur_mond": *e == Einheit::Weltentruemmer, "stationaer": *e == Einheit::Solarsonde, "kosten": gueter(&r.kosten_einheit(*e, sp.volk), false),
                    "fertigungsplaetze_je_stueck":crate::parallel::platzbedarf(Produkt::Einheit(*e)),"bauzeit_je_planet_sekunden":sp.planeten.iter().map(|pid| (self.planeten[*pid as usize].koord.to_string(),self.fertigungszeit(*pid as usize,&r.kosten_einheit(*e,sp.volk),0))).collect::<std::collections::BTreeMap<_,_>>(),
                    "kosten_je_planet":sp.planeten.iter().map(|pid|(self.planeten[*pid as usize].koord.to_string(),Gut::ALLE.iter().map(|g|(g.name(),self.produktionskosten(*pid as usize,Gebaeude::Werft,r.kosten_einheit(*e,sp.volk))[g.idx()] as f64/M as f64)).collect::<std::collections::BTreeMap<_,_>>())).collect::<std::collections::BTreeMap<_,_>>(),
                    "werte":self.einheit_daten(sid,*e,sp.volk),"werft": er.werft, "braucht": er.braucht.iter().map(|(b, st)| format!("{b} {st}")).collect::<Vec<_>>()})
            })
            .collect();
        let forschung = json!({
            "stufen": stufen(self.forschungen(), &sp.forschung),
            "aktiv": sp.forschung_aktiv.as_ref().map(|a| json!({"forschung": a.forschung.name(), "stufe": a.stufe,
                "rest_stunden": if fp > 0 { json!((a.fp_rest + fp - 1) / fp) } else { Value::Null },
                "punkte_rest":a.fp_rest,"punkte_gesamt":r.fp_forschung(a.forschung,a.stufe),"rate":fp,
                "begonnen_sekunden":a.begonnen,
                "naechster_tick_sekunden":(self.zeit/STUNDE+1)*STUNDE,
                "fertig_sekunden":if fp>0{Some((self.zeit/STUNDE+(a.fp_rest+fp-1)/fp)*STUNDE)}else{None},
                "pausiert":self.auftrag_pausiert("forschung",sid as u32),"manuell_pausiert":self.auftrag_pausiert("forschung",sid as u32)})),
            "schlange": sp.forschung_schlange.iter().map(|(f, _, _)| f.name()).collect::<Vec<_>>(),
            "warteschlangen":self.warteschlangen_sicht(sid),
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

        let mut truemmer: Vec<Value> = self
            .truemmer
            .iter()
            .filter(|(k, _)| {
                if self.aufklaerungsregeln() {
                    return self.system_erfasst(sid,**k);
                }
                sp.planeten
                    .iter()
                    .any(|e| self.planeten[*e as usize].koord.sektor == k.sektor)
            })
            .map(
                |(k, t)| { let mut v = json!({"koord": k.to_string(), "erz": ganz(t[0]), "kristall": ganz(t[1])}); self.truemmer_zusatz(*k, &mut v); v },
            )
            .collect();

        // Personen bleiben rettbar, wenn eine frühere Flotte bereits alle Rohstoffe eingesammelt hat.
        for (k,people) in &self.monde.ueberlebende {
            if people.gesamt()>0 && self.system_erfasst(sid,*k) && !truemmer.iter().any(|r|r["koord"]==k.to_string()) {
                let mut v=json!({"koord":k.to_string(),"erz":0,"kristall":0});self.truemmer_zusatz(*k,&mut v);truemmer.push(v);
            }
        }

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
            "flugtermine":self.ausbau.flugtermine.values().filter(|a|a.spieler==sid).collect::<Vec<_>>(),
            "anfaengerschutz_bis": if jetzt < sp.schutz_bis { json!(zeittext(sp.schutz_bis)) } else { Value::Null },
            "toepfe": toepfe,
            "naechste_stufe": naechste_stufe,
            "forschung": forschung,
            "bergbauziele":self.bergbauziele(sid),
            "einheiten_kosten": einheiten_kosten,
            "planeten": planeten,
            "flotten": flotten,
            "regelkontext":self.planer_regelkontext(),
            "expeditionen":self.ausbau.expeditionen.iter().filter(|(_,e)|e.besitzer==sid).map(|(id,e)|json!({"flotte":id,"status":e.status,"ereignisse":e.ereignisse,"naechstes":e.naechstes,"rettungsfrist":e.frist,"ruempfe":e.ruempfe,"berichte":e.berichte,"personen":self.ausbau.passagiere.get(id)})).collect::<Vec<_>>(),
            "ausbildungen":self.ausbau.ausbildungen.iter().filter(|a|self.planeten[a.planet as usize].besitzer==sid).collect::<Vec<_>>(),
            "verbuendete_anfluege": self.verbuendete_anfluege(sid),
            "verbuendete_im_orbit": self.verbuendete_im_orbit(sid),
            "flottenplaetze": {"belegt": self.flotten.values().filter(|f| f.besitzer == sid).count(), "gesamt": self.flottenplaetze(sid)},
            "kolonien": {"anzahl": self.kolonien(sid), "erlaubt": self.kolonien_erlaubt(sid), "verwaltungsgrenze": self.verwaltungsgrenze(sid),
                // 0.4.7: freie Plätze ab Start, Forschung für weitere Plätze und deren nötige Stufe für die nächste Kolonie
                // (null, wenn die Höchstzahl erreicht ist).
                "frei_basis": self.kolonien_frei(), "forschung": self.kolonie_forschung().name(),
                "naechste_stufe": self.kolonie_stufe_fuer(self.kolonien(sid) + 1), "hoechstens": self.kolonien_grenze()},
            "planetenkapazitaet": self.planetenkapazitaet(sid),
            "kolonie_weg": self.kolonie_weg(sid),
            "warnungen": warnungen,
            "angriffe": angriffe,
            "taktische_raketensalven":self.salven_sicht(sid),
            "raketensalven":self.ereignisse.iter().filter_map(|e| match e.art {EreignisArt::RaketenAnkunft{von,ziel,anzahl,zieltyp} if von==sid || self.belegung.get(&ziel).is_some_and(|p|self.planeten[*p as usize].besitzer==sid)=>Some(json!({"von":name(von),"ziel":ziel.to_string(),"anzahl":anzahl,"zieltyp":zieltyp.name(),"ankunft":zeittext(e.zeit)})),_=>None}).chain(self.ausbau.raketensystem.salven.values().filter(|s|s.von==sid||s.gegner==sid).map(|s|json!({"von":name(s.von),"ziel":s.ziel.to_string(),"anzahl":s.anzahl,"zieltyp":s.zieltyp.name(),"ankunft":zeittext(s.ankunft)}))).collect::<Vec<_>>(),
            "verbaende":self.flotten.values().filter(|f|f.verband==Some(f.id) && (f.besitzer==sid || self.verbuendet(sid,f.besitzer))).map(|f|{
                // Für den Versand-Assistenten: exakte Ankunft, Teilnehmer und das Fenster, in dem ein Beitritt den
                // Verband noch verzögern darf (keine Flotte mehr als 6 h gegenüber ihrer ursprünglichen Ankunft).
                let teile:Vec<_>=self.flotten.values().filter(|x|x.verband==Some(f.id)&&x.zustand==Flottenzustand::Hinflug).collect();
                let spaeteste=teile.iter().map(|x|x.abflug+x.flugdauer).min().unwrap_or(f.ankunft)+6*STUNDE;
                let mut spieler:Vec<String>=teile.iter().map(|x|name(x.besitzer)).collect();spieler.sort();spieler.dedup();
                let allianz=self.spieler[sid as usize].allianz;
                json!({"fuehrung":f.id,"spieler":name(f.besitzer),"ziel":f.ziel.to_string(),"ankunft":zeittext(f.ankunft),
                    "ankunft_sekunden":f.ankunft,"spaeteste_ankunft_sekunden":spaeteste,"flotten":teile.len(),"flotten_max":16,
                    "teilnehmer":spieler,"schiffe":teile.iter().map(|x|x.schiffe.iter().sum::<i64>()).sum::<i64>(),
                    "beitretbar":f.besitzer==sid||(allianz.is_some()&&allianz==self.spieler[f.besitzer as usize].allianz)})
            }).collect::<Vec<_>>(),
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
                    // Berichtseigene Kürzel statt Spieler-IDs (ID-Leck: Bots tragen die IDs 0–29).
                    let mut beteiligte: Vec<SpielerId> = Vec::new();
                    for id in b.angreifer_gruppen.iter().map(|g| g.spieler).chain(std::iter::once(b.angreifer)).chain(b.verteidiger.iter().copied()) {
                        if !beteiligte.contains(&id) { beteiligte.push(id); }
                    }
                    let kuerzel = |id: SpielerId| format!("t{}", beteiligte.iter().position(|x| *x == id).map_or(0, |i| i + 1));
                    let protokoll = self.ausbau.kampfrunden.get(&b.nr).map(|runden| {
                        let mut r = serde_json::Value::Array(runden.clone());
                        spieler_kuerzel(&mut r, &kuerzel);
                        r
                    });
                    json!({"zeit": zeittext(b.zeit), "ort": b.ort.to_string(), "mission": b.mission.name(),
                        "seite": if b.verteidiger.contains(&sid) { "verteidigung" } else { "angriff" },
                        "angreifer": angreifer, "verteidiger": b.verteidiger.iter().map(|v| name(*v)).collect::<Vec<_>>(),
                        "sieger": match b.sieger { Sieger::Angreifer => "angreifer", Sieger::Verteidiger => "verteidiger", Sieger::Unentschieden => "unentschieden" },
                        "runden": b.runden,"nummer":b.nr,
                        // Startplanet je angreifender Flotte plus Heimatwelt des Angreifers (Karl 22.10.2026).
                        "herkunft": self.ausbau.kampf_herkunft.get(&b.nr).map(|h| h.iter().map(|(s,k,m)| json!({"spieler":name(*s),"start":k.to_string(),"mond":m,
                            "heimat":self.planeten[self.spieler[*s as usize].heimat as usize].koord.to_string()})).collect::<Vec<_>>()).unwrap_or_default(),
                        // Kampftechnik beider Seiten zum Kampfzeitpunkt: Stufe und Wirkung in Prozent (inkl. Volksbonus).
                        "technik": self.ausbau.kampf_technik.get(&b.nr).map(|t| t.iter().map(|(s,[w,sc,p])| {
                            let volk = self.regeln.volk(self.spieler[*s as usize].volk);
                            let prozent = |stufe: u8, faktor: f64| (((1.0 + self.regeln.kampf.tech_je_stufe * stufe as f64) * faktor - 1.0) * 100.0).round();
                            json!({"spieler":name(*s),"kuerzel":kuerzel(*s),"waffentechnik":w,"schildtechnik":sc,"panzerung":p,
                                "angriff_prozent":prozent(*w, volk.waffen),"schild_prozent":prozent(*sc, 1.0),"struktur_prozent":prozent(*p, volk.panzerung)})
                        }).collect::<Vec<_>>()).unwrap_or_default(),
                        "spielervoelker":beteiligte.iter().map(|id|(kuerzel(*id),self.spieler[*id as usize].volk.name())).collect::<std::collections::BTreeMap<_,_>>(),
                        "rundenprotokoll":protokoll,
                        "spielernamen":beteiligte.iter().map(|id|(kuerzel(*id),name(*id))).collect::<std::collections::BTreeMap<_,_>>(),
                        "teilnehmer":angreifer.iter().chain(b.verteidiger.iter().map(|v|name(*v)).collect::<Vec<_>>().iter()).cloned().collect::<Vec<_>>(),
                        "angreifer_vorher":einheiten(&b.ang_vorher,0),"angreifer_nachher":einheiten(&b.ang_nachher,0),
                        "verteidiger_vorher":einheiten(&b.vert_vorher,0),"verteidiger_nachher":einheiten(&b.vert_nachher,0),
                        "verluste_angreifer": verlust(&b.ang_vorher, &b.ang_nachher),
                        "verluste_verteidiger": verlust(&b.vert_vorher, &b.vert_nachher),
                        "beute": gueter(&b.beute, false),
                        "truemmer": {"erz": ganz(b.truemmer[0]), "kristall": ganz(b.truemmer[1])},
                        // Je angreifender Flotte (Verband) Verluste einzeln.
                        "angreifer_flotten": b.angreifer_gruppen.iter().map(|g| json!({"flotte": g.flotte, "spieler": name(g.spieler),
                            "vorher": einheiten(&g.vorher, 0), "nachher": einheiten(&g.nachher, 0), "verluste": verlust(&g.vorher, &g.nachher)})).collect::<Vec<_>>(),
                        // Berichtszusatz: Ziel Mond, je verteidigender Partei, Wiederaufbau, Werftdock (fehlt in alten Kämpfen).
                        "ziel_mond": self.ausbau.kampf_zusatz.get(&b.nr).and_then(|z| z.get("ziel_mond")).cloned(),
                        "wiederaufgebaut": self.ausbau.kampf_zusatz.get(&b.nr).and_then(|z| z.get("wiederaufgebaut")).cloned(),
                        "ueberlebende": self.ausbau.kampf_zusatz.get(&b.nr).and_then(|z| z.get("ueberlebende")).cloned(),
                        "wiederaufbau_prozent": self.ausbau.kampf_zusatz.get(&b.nr).and_then(|z| z.get("wiederaufbau_prozent")).cloned()
                            .unwrap_or(json!((self.regeln.kampf.verteidigung_wiederaufbau * 100.0).round())),
                        "repariert": self.ausbau.kampf_zusatz.get(&b.nr).and_then(|z| z.get("repariert")).cloned(),
                        "truemmer_prozent": self.ausbau.kampf_zusatz.get(&b.nr).and_then(|z| z.get("truemmer_prozent")).cloned()
                            .unwrap_or(json!((self.regeln.kampf.truemmer_anteil * 100.0).round())),
                        "verteidiger_parteien": self.ausbau.kampf_zusatz.get(&b.nr).and_then(|z| z["parteien"].as_array()).map(|ps| ps.iter().map(|p| {
                            let mut q = p.clone();
                            q["spieler"] = json!(p["spieler"].as_u64().map(|s| name(s as SpielerId)));
                            q
                        }).collect::<Vec<_>>()),
                        "mondwurf": self.monde.wuerfe.get(&b.nr).map(|m| json!({"chance_prozent": m.chance_prozent, "wurf": m.wurf,
                            "entstanden": m.mond.is_some(), "typ": m.typ.name(), "durchmesser": m.durchmesser, "truemmer": ganz(m.truemmer)}))})
                })
                .collect::<Vec<_>>(),
            // Auch Verbündete, deren Hilfstruppen mitkämpfen, sehen die Bodenrunde; „seite“ sagt, wofür sie stand.
            "bodenberichte":self.ausbau.boden.berichte.iter().rev()
                .filter(|b|b["angreifer"]==sid||b["verteidiger"]==sid||b["verteidiger_quellen"].as_array().is_some_and(|q|q.iter().any(|e|e["spieler"]==sid)))
                .take(60).map(|b|{let mut v=bodenbericht_ohne_nummern(b);v["seite"]=json!(if b["angreifer"]==sid{"angriff"}else if b["verteidiger"]==sid{"verteidigung"}else{"hilfe"});v}).collect::<Vec<_>>(),
            // Spionageberichte und Erkundungsberichte freier Plätze, neueste zuerst.
            "berichte": self.berichte_sicht(sid),
            "systemberichte": self.systemberichte_sicht(sid),
            "schiffangebote": self.schiffangebote_sicht(sid),
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
            "ereignisse_ausgelassen": seit_letztem.saturating_sub(fenster),
            "meldungen": sp.meldungen.iter().rev().take(10).map(|m| json!({"zeit": zeittext(m.zeit), "von": m.von.name(), "text": m.text})).collect::<Vec<_>>(),
        });
        if self.kolonisationsregeln_v2() {
            let mut deliveries=self.ereignisse.iter().filter_map(|e|match e.art {
                EreignisArt::MarktlieferungGebunden{planet,empfaenger,gut,menge} if empfaenger==sid =>
                    Some(json!({"id":e.seq,"ziel":self.planeten[planet as usize].koord.to_string(),"gut":gut.name(),
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
                    "angreifer":self.spieler[f.besitzer as usize].name,"verteidiger":self.spieler[b.verteidiger as usize].name,"seit":b.seit,"fruehestens":b.fruehestens,
                    "abgeschlossene_reaktionsfenster":b.reaktionsfenster,"mindestens_noch_fenster":2u8.saturating_sub(b.reaktionsfenster),
                    "hinweis":"Übernahme erst nach Abschluss der letzten Reaktion und bei fortbestehender Orbitkontrolle."}))
            }).collect::<Vec<_>>());
        }
        if self.invasion_v1() { view["invasionen"] = self.invasion_sicht(sid); }
        view["orbitflotten"] = json!(self.orbitflotten_aktiv());
        if self.orbitflotten_aktiv() { view["orbitflotten_bekannt"] = self.orbitflotten_bekannt(sid); }
        view["invasion_v2"] = json!(self.invasion_v2());
        if self.monde.aktiv { view["monde"] = self.monde_sicht(sid); }
        if self.ausbau.dyson.aktiv { view["dyson"] = self.dyson_sicht(sid); }
        // Selbst gewählte Namen eigener und verbündeter Planeten (Koordinate → Name) für Flotten, Ereignisse und Berichte;
        // fehlt, solange niemand benannt hat (Sicht bleibt dann unverändert).
        let namen: Map<String, Value> = self.ausbau.planetennamen.keys()
            .filter_map(|id| { let k = self.planeten.get(*id as usize)?.koord; self.planetenname_fuer(sid, k).map(|n| (k.to_string(), json!(n))) }).collect();
        if !namen.is_empty() { view["planetennamen"] = Value::Object(namen); }
        view
    }

    /// Lesende Werkzeuge der Agenten.
    pub fn werkzeug(&self, sid: SpielerId, abfrage: &Value) -> Result<Value, String> {
        self.werkzeug_als(sid, Rolle::Alle, abfrage)
    }

    /// Lesende Abfrage in einer Regierungsrolle: Vorschauen (missionsplan, marktplan, flugkoordination, ablaufplan ohne
    /// eigene Rolle) prüfen mit genau den Rechten und Töpfen, mit denen der spätere Befehl dieser Rolle läuft.
    pub fn werkzeug_als(&self, sid: SpielerId, rolle: Rolle, abfrage: &Value) -> Result<Value, String> {
        let r = &self.regeln;
        let sp = self.spieler.get(sid as usize).ok_or("Spieler fehlt")?;
        let text = |feld: &str| abfrage.get(feld).and_then(|v| v.as_str());
        let zielstufe = |aktuell: u8, maximum: u8| -> Result<u8, String> {
            let n = match abfrage.get("stufe") {
                None | Some(Value::Null) => u64::from(aktuell) + 1,
                Some(v) => v.as_u64().ok_or_else(|| format!("stufe: ganze Zahl von 1 bis {maximum} erwartet"))?,
            };
            if n == 0 || n > u64::from(maximum) {
                return Err(format!("stufe: ganze Zahl von 1 bis {maximum} erwartet"));
            }
            Ok(n as u8)
        };
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
                    .filter(|e| e.ist_schiff() && r.einheiten.contains_key(e))
                    .ok_or_else(|| format!("{n} ist kein Schiff"))?;
                s[e.idx()] = z.as_i64().filter(|n| *n >= 0)
                    .ok_or_else(|| format!("{n}: nicht negative ganze Schiffszahl erwartet"))?;
            }
            Ok(s)
        };
        match text("typ").unwrap_or("") {
            "koloniebedarf" => {
                let volk=abfrage.get("volk").filter(|v|!v.is_null()).map(|v|serde_json::from_value::<Volk>(v.clone())).transpose().map_err(|e|e.to_string())?.unwrap_or(self.spieler[sid as usize].volk);
                let typ=abfrage.get("einheit").filter(|v|!v.is_null()).map(|v|serde_json::from_value::<Einheit>(v.clone())).transpose().map_err(|e|e.to_string())?.unwrap_or(Einheit::Kolonieschiff);
                let cargo=self.koloniebedarf(sid,typ,volk)?;
                Ok(json!({"volk":volk,"einheit":typ,"siedler":self.regeln.flottendoktrin.get(&typ).map(|p|p.siedler).unwrap_or(self.regeln.wirtschaft.siedler),"fracht":gueter(&cargo,true),"hinweis":"Vollständige Siedlergruppe aus genau einer Herkunft; mindestens 1000 Einwohner bleiben zurück. Solar und bei organischen Siedlern Farm starten auf Stufe 1. Weiteren Ausbau selbst planen. Nahrung reicht rechnerisch 24 Stunden ohne Farmproduktion."}))
            },
            "raketenplan" => {let a=abfrage.get("aktion").ok_or("Aktion fehlt")?;if a["typ"]!="raketen_starten"{return Err("Nur Raketenstart kann vorgeschaut werden".into());}self.raketenplan(sid,a)},
            "terraformbedarf" => self.terraformbedarf_werkzeug(sid, abfrage),
            "habitabilitaet" => {let k=koord("planet")?;if self.platz(k).is_none(){return Err("Ungültiger Planetenplatz".into());}Ok(self.klima_sicht(sid,k))},
            "missionsplan" => self.missionsplan_als(sid,rolle,&abfrage["aktion"]),
            "marktplan" => self.marktplan_als(sid,rolle,&abfrage["aktion"]),
            "ablaufplan" => {let mut q:crate::planer::ablauf::PlanAnfrage=serde_json::from_value(abfrage["plan"].clone()).map_err(|e|format!("Plan: {e}"))?;
                // Ohne eigene Planrolle gilt die Rolle des Anfragenden (Harness/MCP); eine andere Regierungsrolle darf sie nicht vorschauen.
                if q.rolle==Rolle::Alle {q.rolle=rolle;} else if rolle!=Rolle::Alle && q.rolle!=rolle {return Err(format!("ablaufplan: Rolle {} statt {rolle} angefragt",q.rolle));}
                self.ablaufplan(sid,&q)},
            "regelkontext" => Ok(serde_json::to_value(self.planer_regelkontext()).unwrap()),
            "wirtschaftsdiagnose" => self.wirtschaftsdiagnose(sid, koord("planet")?),
            "kostenplan" => {
                let positionen=serde_json::from_value::<Vec<crate::planer::kosten::KostenAnfrage>>(abfrage["positionen"].clone())
                    .map_err(|e|format!("Ungültige Kostenposition: {e}"))?;
                self.kostenplan(sid,koord("planet")?,&positionen)
            },
            "briefarchiv" => {
                let before=abfrage["vor_id"].as_u64().unwrap_or(u64::MAX);
                let limit=abfrage["limit"].as_u64().unwrap_or(50).clamp(1,100) as usize;
                let channel=abfrage["kanal"].as_str().unwrap_or("alle");
                let letters=self.briefkasten_seite(sid,before,limit,channel);
                Ok(json!({"briefe":letters,"naechste_vor_id":letters.last().and_then(|n|n["id"].as_u64())}))
            },
            "kolonieplan" => self.kolonieplan(sid, abfrage),
            "planet_aufgeben" => {
                let k = koord("planet")?;
                self.planet_aufgeben_vorschau(sid, k)
            }
            "soforthandel" => {
                let k = koord("planet")?;
                let angebot: std::collections::BTreeMap<String, f64> = serde_json::from_value(abfrage["angebot"].clone()).map_err(|e| format!("Angebot: {e}"))?;
                self.soforthandel_angebot(sid, k, &angebot, abfrage["ziel"].as_str().ok_or("Ziel fehlt")?)
            }
            "raketensimulator" => crate::raketen::simulieren(&self.regeln, abfrage),
            "wiederaufbau" => {
                // 0.4.8 (Karl 24.10.2026: „…dass man viele Rohstoffe zum Wiederaufbau mitführen sollte“): Reparaturkosten aller
                // Gebäude aus dem letzten eigenen Spionagebericht, voll heruntergebombt – Grundlage der Vorlage „Wiederaufbau“.
                let ziel = koord("ziel")?;
                let b = sp.berichte.iter().filter(|b| b.ziel == ziel).max_by_key(|b| b.zeit)
                    .ok_or("Für dieses Ziel gibt es noch keinen Spionagebericht")?;
                let g = b.gebaeude.as_ref().ok_or("Der Spionagebericht zeigt keine Gebäude – mit mehr Sonden ausspähen")?;
                let anteil = self.regeln.kw().boden.reparatur_stufe_promille.max(0) as i128;
                let schaden = (1000 - self.regeln.kw().boden.integritaet_grenze_promille).clamp(0, 1000) as i128;
                let mut summe = [0i64; GUETER];
                let mut gebaeude = 0i64;
                for (i, geb) in Gebaeude::ALLE.iter().enumerate() {
                    let stufe = g.get(i).copied().unwrap_or(0);
                    if stufe == 0 || !r.gebaeude.contains_key(geb) { continue; }
                    gebaeude += 1;
                    if anteil > 0 {
                        for (j, w) in r.kosten_gebaeude(*geb, stufe).iter().enumerate() { summe[j] += ((*w as i128 * anteil + 999) / 1000) as i64; }
                    } else {
                        for s in 1..=stufe { for (j, w) in r.kosten_gebaeude(*geb, s).iter().enumerate() { summe[j] += ((*w as i128 * schaden + 999) / 1000) as i64; } }
                    }
                }
                Ok(json!({"ziel":ziel.to_string(),"bericht_zeit":b.zeit,"gebaeude":gebaeude,"kosten":gueter(&summe,false),
                    "anteil_promille":anteil,"hinweis":"Reparatur aller ausgespähten Gebäude bei 30 % Integrität; die Fracht landet mit dem Kolonieschiff."}))
            }
            "kosten" => {
                let pid = match text("planet") {
                    Some(k) => self.eigener_planet(sid, k.parse()?)?,
                    None => sp.heimat as usize,
                };
                if let Some(a) = text("rakete") {
                    let art=crate::raketen::art(a)?;
                    let anzahl=abfrage.get("anzahl").and_then(Value::as_i64).unwrap_or(1);
                    self.raketen_kosten(art,anzahl)?;
                    let kosten=self.produktionskosten(pid,Gebaeude::Raketensilo,self.raketen_kosten(art,1)?).map(|c|c*anzahl);
                    Ok(json!({"rakete":a,"anzahl":anzahl,"kosten":if self.ausbau.industrie_version>0{gueter_exakt(&kosten)}else{gueter(&kosten,false)},"bauzeit_sekunden":if self.ausbau.version>0{(crate::raketen::regeln(&self.regeln)[art as usize].bauzeit_sekunden*anzahl*100+self.werk_tempo(pid,Gebaeude::Raketensilo)-1)/self.werk_tempo(pid,Gebaeude::Raketensilo)}else{r.zusatz.raketen_bauzeit_sekunden*anzahl},"kapazitaet":self.planeten[pid].gebaeude[Gebaeude::Raketensilo.idx()] as i64*r.zusatz.silo_plaetze_je_stufe,"belegt":self.raketen_belegt(pid)}))
                } else if let Some(g) = text("gebaeude") {
                    let g = Gebaeude::aus_name(g).filter(|g| r.gebaeude.contains_key(g)).ok_or_else(|| format!("Gebäude {g} gibt es in diesem Regelprofil nicht"))?;
                    let jetzt = self.planeten[pid].gebaeude[g.idx()];
                    let stufe = zielstufe(jetzt, r.welt.max_gebaeudestufe)?;
                    let k = r.kosten_gebaeude(g, stufe);
                    let gr = r.geb(g);
                    let mut quote = json!({"gebaeude": g.name(), "stufe": stufe, "kosten": gueter(&k, false), "bauzeit_min": self.bauzeit(pid, &k) / 60,
                        "ertrag": ganz(r.stufenwert(gr.ertrag, stufe)), "arbeiter": ganz(r.stufenwert(gr.arbeiter, stufe)),
                        "fachkraefte": ganz(r.stufenwert(gr.fachkraefte, stufe)), "strom": ganz(r.stufenwert(gr.energie, stufe)),
                        "ab_zivilisationsstufe": gr.ab_stufe,
                        "zeitberechnung":self.zeitberechnung(pid,&k,None)});
                    if g == Gebaeude::Fusionskraftwerk {
                        quote["deuteriumbedarf_je_stunde_vollbesetzt"] = json!(r.stufenwert(r.wirtschaft.fusion_deuterium, stufe) as f64 / M as f64);
                    }
                    if self.kolonisationsregeln_v2() && self.integritaet(pid,g)<1000 {
                        let cost = self.reparaturkosten(pid,g)?;
                        quote["reparatur"] = json!({"integritaet_prozent":self.integritaet(pid,g)/10,
                            "kosten":gueter(&cost,false),"dauer_sekunden":self.bauzeit(pid,&cost).max(15*MINUTE),
                            "hinweis":"Eigene Baustelle; kein paralleler Gebäudeausbau auf diesem Planeten."});
                    }
                    Ok(quote)
                } else if let Some(f) = text("forschung") {
                    let f = Forschung::aus_name(f).filter(|f| r.forschung.contains_key(f)).ok_or_else(|| format!("Forschung {f} gibt es in diesem Regelprofil nicht"))?;
                    let stufe = zielstufe(sp.forschung[f.idx()], crate::regeln::MAX_FORSCHUNG)?;
                    let fp = r.fp_forschung(f, stufe);
                    let rate = self.fp_rate(sid);
                    Ok(json!({"forschung": f.name(), "stufe": stufe, "kosten": gueter(&r.kosten_forschung(f, stufe), false),
                        "punkte": ganz(fp), "dauer_stunden": if rate > 0 { json!((fp + rate - 1) / rate) } else { Value::Null },
                        "labor": r.forsch(f).labor, "ab_zivilisationsstufe": r.forsch(f).ab_stufe}))
                } else if let Some(e) = text("einheit") {
                    let e = Einheit::aus_name(e).filter(|e| r.einheiten.contains_key(e)).ok_or_else(|| format!("Einheit {e} gibt es in diesem Regelprofil nicht"))?;
                    let volk=text("volk").map(|v|Volk::aus_name(v).ok_or_else(||format!("Unbekanntes Volk: {v}"))).transpose()?.unwrap_or(sp.volk);
                    let k = r.kosten_einheit(e, volk);
                    let er = r.einh(e);
                    Ok(json!({"einheit": e.name(), "kosten": if self.ausbau.industrie_version>0{gueter_exakt(&self.produktionskosten(pid,Gebaeude::Werft,k))}else{gueter(&k,false)}, "bauzeit_min": (self.fertigungszeit(pid, &k, 0) + 59) / 60,"bauzeit_sekunden":self.fertigungszeit(pid,&k,0),
                        "besatzung": er.besatzung, "unterhalt_je_tag": self.einheit_daten(sid,e,volk)["unterhalt_je_tag"],
                        "werte":self.einheit_daten(sid,e,volk),"ab_zivilisationsstufe": er.ab_stufe, "werft": er.werft,"braucht":er.braucht,
                        "zeitberechnung":self.zeitberechnung(pid,&k,Some((0,sp.volk)))}))
                } else {
                    Err("kosten braucht gebaeude, forschung oder einheit".into())
                }
            }
            "flugkoordination"=>self.flugkoordination_quote_als(sid,rolle,abfrage),
            "flugzeit" => {
                // 0.4.8: `flotte` = eigene gestrandete Flotte als Startort (ihre Schiffe, Treibstoff aus der Ladung).
                let orbit = abfrage.get("flotte").and_then(|v| v.as_u64()).map(|x| x as FlottenId)
                    .filter(|f| self.gestrandet(*f) && self.flotten.get(f).is_some_and(|x| x.besitzer == sid));
                let ziel = koord("ziel")?;
                let (start, raumhafen, s) = if let Some(f) = orbit { (self.flotten[&f].ziel, 0u8, self.flotten[&f].schiffe) } else {
                    let start = koord("start")?;
                    let pid = self.eigener_planet(sid, start)?;
                    (start, self.planeten[pid].gebaeude[Gebaeude::Raumhafen.idx()], schiffe()?)
                };
                let sigma = milli(abfrage.get("geschwindigkeit").and_then(|v| v.as_f64()).unwrap_or(1.0)).clamp(100, 1000);
                let plan = self.flugplan_von(sid, start, raumhafen, ziel, &s, sigma)?;
                let roh: i64 = Einheit::ALLE.iter().take(SCHIFFE).map(|e|s[e.idx()]*r.ladung_milli(*e,sp.volk)).sum();
                let berg: i64 = [Einheit::Recycler,Einheit::Bergungsschiff].iter().map(|e|s[e.idx()]*r.ladung_milli(*e,sp.volk)).sum();
                let bergung = if roh>0 {ganz(((plan.kapazitaet as i128*berg as i128)/roh as i128) as i64)}else{0};
                // Tempotabelle 100 % bis 10 % mit demselben Flugplan (gleiche Schiffe, Start und Ziel): die Oberfläche
                // braucht so eine Abfrage je Flotte statt zehn (Ratenlimit).
                let tempi = (1..=10).rev().map(|z| {
                    let p = self.flugplan_von(sid, start, raumhafen, ziel, &s, z * 100)?;
                    Ok(json!({"geschwindigkeit": z as f64 / 10.0, "dauer_sekunden": p.dauer, "treibstoff_je_strecke": ganz(p.treibstoff), "treibstoff_je_strecke_milli": p.treibstoff}))
                }).collect::<Result<Vec<_>, String>>()?;
                Ok(json!({"entfernung": plan.entfernung, "dauer_min": plan.dauer / 60, "dauer_sekunden":plan.dauer, "treibstoff_je_strecke": ganz(plan.treibstoff), "treibstoff_je_strecke_milli":plan.treibstoff,
                    "ladekapazitaet": ganz(plan.kapazitaet), "bergungskapazitaet":bergung, "personenplaetze":plan.personenplaetze,"personenbeforderung_verfuegbar":false, "tempo": plan.tempo,
                    "treibstoff_erklaerung":plan.treibstoff_erklaerung,"kapazitaet_erklaerung":plan.kapazitaet_erklaerung,"tempi":tempi,
                    "vom_orbit":orbit.is_some(),"deuterium_an_bord":orbit.map(|f| ganz(self.flotten[&f].ladung[Gut::Deuterium.idx()]))}))
            }
            "kampfszenario" => self.kampfszenario(abfrage),
            "kampfsimulator" => {
                let ziel = koord("ziel")?;
                let s = schiffe()?;
                // 0.4.8 (Einsatzprobe 25.10.2026): Mondziele rechnen mit dem Mondspionagebericht – vorher zeigte das Blatt für einen
                // Mond die Planetenwerte (100 % Siegchance), der echte Mondkampf kostete die halbe Flotte.
                if text("zielart") == Some("mond") {
                    let m = self.monde.spionageberichte.iter().filter(|b| b.spieler == sid && b.koord == ziel).max_by_key(|b| b.zeit)
                        .ok_or_else(|| format!("für den Mond bei {ziel} liegt kein Mondspionagebericht vor"))?;
                    let (Some(ms), Some(mv)) = (&m.schiffe, &m.verteidigung) else {
                        return Err("der Mondbericht zeigt Schiffe oder Verteidigung nicht: mehr Sonden schicken".into());
                    };
                    let mut ang = [0i64; EINHEITEN];
                    ang[..SCHIFFE].copy_from_slice(&s);
                    let mut vert = [0i64; EINHEITEN];
                    for (k, n) in ms.iter().chain(mv.iter()) { if let Some(e) = Einheit::aus_name(k) { vert[e.idx()] += *n; } }
                    crate::spielhilfe::simulator_grenzen(&ang, &vert)?;
                    let volk_v = self.spieler.get(m.besitzer as usize).ok_or("Berichtsbesitzer fehlt")?.volk;
                    let mut tech = sp.forschung.clone();
                    if let Some(f) = &m.forschung { for (k, n) in f { if let Some(x) = Forschung::aus_name(k) { if let Some(t) = tech.get_mut(x.idx()) { *t = *n; } } } }
                    let party=|volk:Volk,counts:&[i64;EINHEITEN],tech:&[u8]|json!({"volk":volk.name(),
                        "einheiten":Einheit::ALLE.iter().filter(|e|counts[e.idx()]>0).map(|e|(e.name(),counts[e.idx()])).collect::<std::collections::BTreeMap<_,_>>(),
                        "technik":{"waffentechnik":tech[Forschung::Waffentechnik.idx()],"schildtechnik":tech[Forschung::Schildtechnik.idx()],"panzerung":tech[Forschung::Panzerung.idx()],"logistik":tech[Forschung::Logistik.idx()]}});
                    let mut bestand = m.bestand.clone();
                    bestand.resize(GUETER, 0);
                    let mut query = json!({"angreifer":party(sp.volk,&ang,&sp.forschung),"verteidiger":party(volk_v,&vert,&tech),"bestand_milli":bestand,"bunker":0});
                    for key in ["szenarioseed","vorladung_milli","bergung_promille"] {if let Some(v)=abfrage.get(key){query[key]=v.clone();}}
                    let mut result = self.kampfszenario(&query)?;
                    result["modus"] = json!("mondspionagebericht");
                    result["bericht_alter_stunden"] = json!((self.zeit - m.zeit) / STUNDE);
                    result["technik_des_gegners"] = json!(if m.forschung.is_some() {"aus dem Mondbericht"} else {"geschätzt wie die eigene"});
                    return Ok(result);
                }
                let Some(b) = sp.berichte.iter().find(|b| b.ziel == ziel) else {
                    return Err(format!("für {ziel} liegt kein Spionagebericht vor"));
                };
                let (Some(bs), Some(bv)) = (&b.schiffe, &b.verteidigung) else {
                    return Err("der Bericht zeigt Schiffe oder Verteidigung nicht: mehr Sonden oder bessere Spionagetechnik".into());
                };
                // Berichte ohne Terraformer-Spalte (34 Schiffe) bleiben lesbar.
                if !(SCHIFFE-1..=SCHIFFE).contains(&bs.len()) || bv.len()!=EINHEITEN-SCHIFFE {return Err("Berichtsformat passt nicht zum aktiven Katalog".into());}
                let mut ang = [0i64; EINHEITEN];
                ang[..SCHIFFE].copy_from_slice(&s);
                let mut vert = [0i64; EINHEITEN];
                vert[..bs.len()].copy_from_slice(bs);
                vert[SCHIFFE..].copy_from_slice(bv);
                crate::spielhilfe::simulator_grenzen(&ang, &vert)?;
                let volk_v = self.spieler.get(b.besitzer as usize).ok_or("Berichtsbesitzer fehlt")?.volk;
                let party=|volk:Volk,counts:&[i64;EINHEITEN],tech:&[u8]|json!({"volk":volk.name(),
                    "einheiten":Einheit::ALLE.iter().filter(|e|counts[e.idx()]>0).map(|e|(e.name(),counts[e.idx()])).collect::<std::collections::BTreeMap<_,_>>(),
                    "technik":{"waffentechnik":tech[Forschung::Waffentechnik.idx()],"schildtechnik":tech[Forschung::Schildtechnik.idx()],"panzerung":tech[Forschung::Panzerung.idx()],"logistik":tech[Forschung::Logistik.idx()]}});
                let defender_tech=b.forschung.as_deref().unwrap_or(&sp.forschung);
                if !(defender_tech.len()==FORSCHUNGEN || defender_tech.len()==self.forschungszahl()) || b.bestand.len()!=GUETER {return Err("Berichtsformat passt nicht zum aktiven Katalog".into());}
                let mut query=json!({"angreifer":party(sp.volk,&ang,&sp.forschung),"verteidiger":party(volk_v,&vert,defender_tech),"bestand_milli":b.bestand,
                    "bunker":b.gebaeude.as_ref().map(|g|g[Gebaeude::Bunker.idx()]).unwrap_or(0)});
                for key in ["szenarioseed","vorladung_milli","bergung_promille"] {if let Some(v)=abfrage.get(key){query[key]=v.clone();}}
                let mut result=self.kampfszenario(&query)?;
                result["modus"]=json!("spionagebericht");
                result["bericht_alter_stunden"]=json!((self.zeit-b.zeit)/STUNDE);
                result["technik_des_gegners"]=json!(if b.forschung.is_some(){"aus dem Bericht"}else{"geschätzt wie die eigene"});
                result["bunker_annahme"]=json!(if b.gebaeude.is_some(){"aus dem Bericht"}else{"unbekannt; Szenario ohne Bunkerschutz"});
                Ok(result)
            }

            "regel" => {
                let query=text("stichwort").unwrap_or("");
                let colony=self.kolonisation.aktiv && ["kolon","erober","invasion","repar","bombard"].iter().any(|q|query.to_lowercase().contains(q));
                if self.kolonisationsregeln_v2() {
                    let base=if colony {String::new()}else{regeltext::nachschlagen(r,query)};
                    let invasion={let t=self.regeltext_invasion();if t.is_empty(){t}else{format!("\n{t}")}};
                    Ok(json!({"text":format!("{base}\nKolonisations- und Flottenregeln: {}{invasion}",crate::kolonisation::REGELTEXT_V2),"kolonie_startfracht":gueter(&self.koloniefracht(sid),true)}))
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
                    let mut eintrag = json!({"system":n,"bekannt":known,
                        "nebel":if known {json!(sys.nebel)} else {Value::Null},
                        "asteroidenguertel":if known {json!(sys.guertel)} else {Value::Null},"belegt":belegt,"plaetze":plaetze});
                    if known {
                        eintrag["bergbau"]=self.guertelumfang(Koord::neu(sektor as u8,n as u8,0));
                    }
                    if let Some(l) = self.loch_im_system(sid, Koord::neu(sektor as u8, n as u8, 1)) { eintrag["hyperraumloch"] = l; }
                    if let Some(d) = self.dyson_im_system(sid, sektor as u8, n as u8) { eintrag["dyson"] = d; }
                    systeme.push(eintrag);
                }
                let mut antwort = json!({"sektor": sektor, "systeme": systeme});
                if self.monde.aktiv {
                    antwort["hyperraumloecher"] = json!(self.loecher_sicht(sid).into_iter().filter(|l| [&l["von"], &l["nach"]].iter().any(|k| k.as_str().and_then(|k| k.parse::<Koord>().ok()).is_some_and(|k| k.sektor as u64 == sektor && (von..=bis).contains(&(k.system as u64))))).collect::<Vec<_>>());
                }
                Ok(antwort)
            }
            anderes => Err(format!("Werkzeug '{anderes}' gibt es nicht: kosten, kostenplan, wirtschaftsdiagnose, regelkontext, flugzeit, kampfsimulator, kampfszenario, kolonieplan, briefarchiv, regel, galaxie")),
        }
    }
}

/// Ersetzt im Rundenprotokoll jede numerische `spieler`-Angabe durch das berichtseigene Kürzel (ID-Leck, 20.10.2026).
/// Auch Schütze und Zielspieler der Feuerzeilen (25.10.2026: der Bericht zeigte „Spieler 30“ statt des Namens).
fn spieler_kuerzel(v: &mut serde_json::Value, kuerzel: &dyn Fn(SpielerId) -> String) {
    match v {
        serde_json::Value::Object(o) => {
            for (k, w) in o.iter_mut() {
                if matches!(k.as_str(), "spieler" | "schuetze" | "zielspieler") {
                    if let Some(id) = w.as_u64() { *w = serde_json::Value::String(kuerzel(id as SpielerId)); continue; }
                }
                spieler_kuerzel(w, kuerzel);
            }
        }
        serde_json::Value::Array(a) => a.iter_mut().for_each(|w| spieler_kuerzel(w, kuerzel)),
        _ => {}
    }
}

/// Bodenbericht für die Spielersicht: ohne die internen Spielernummern von Angreifer, Verteidiger und Verstärkungen (Bots
/// belegen online 0–29, die Nummer verriet die Spielerart; Karl 20.10.2026). Namen stehen schon im Bericht.
fn bodenbericht_ohne_nummern(b: &Value) -> Value {
    let mut b = b.clone();
    if let Some(o) = b.as_object_mut() {
        o.remove("angreifer");
        o.remove("verteidiger");
        let verteidiger = o.get("verteidiger_name").cloned().unwrap_or(Value::Null);
        if let Some(q) = o.get_mut("verteidiger_quellen").and_then(|q| q.as_array_mut()) {
            for e in q.iter_mut().filter_map(|e| e.as_object_mut()) {
                e.remove("spieler");
                e.entry("name").or_insert_with(|| verteidiger.clone());
            }
        }
    }
    b
}
