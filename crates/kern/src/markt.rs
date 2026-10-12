//! Markt: Orderbuch je Gut in Credits. Die Gebühr wird vernichtet, gekaufte Ware
//! bringt eine neutrale Handelsflotte, die nicht abgefangen werden kann.

use crate::typen::*;
use crate::welt::*;

impl Welt {
    /// Gebührensatz eines Spielers, halbiert bei Handelsabkommen mit dem Gegenüber.
    pub fn gebuehr(&self, sid: SpielerId, partner: Option<SpielerId>) -> f64 {
        let mut g = self.regeln.markt.gebuehr
            * self
                .regeln
                .volk(self.spieler[sid as usize].volk)
                .marktgebuehr;
        if let Some(p) = partner {
            if self.vertrag_zwischen(sid, p, Vertragsart::Handelsabkommen) {
                g *= 0.5;
            }
        }
        g
    }

    fn handelswert(menge: i64, preis: i64) -> i64 {
        ((menge as i128 * preis as i128) / M as i128) as i64
    }

    /// Credits, die ein Käufer für eine Order hinterlegt: Wert plus höchste mögliche Gebühr.
    fn hinterlegung(&self, sid: SpielerId, menge: i64, preis: i64) -> i64 {
        let wert = Self::handelswert(menge, preis);
        wert + mal(wert, self.gebuehr(sid, None))
    }

    #[allow(clippy::too_many_arguments)]
    fn handel_ausfuehren(
        &mut self,
        kaeufer: SpielerId,
        kaeufer_planet: PlanetId,
        kaeufer_limit: i64,
        verkaeufer: SpielerId,
        verkaeufer_planet: PlanetId,
        gut: Gut,
        menge: i64,
        preis: i64,
    ) {
        let wert = Self::handelswert(menge, preis);
        let geb_k = mal(wert, self.gebuehr(kaeufer, Some(verkaeufer)));
        let geb_v = mal(wert, self.gebuehr(verkaeufer, Some(kaeufer)));
        // Der Käufer hat zum eigenen Limit hinterlegt und bekommt die Differenz zurück.
        let zurueck = self.hinterlegung(kaeufer, menge, kaeufer_limit) - (wert + geb_k);
        self.spieler[kaeufer as usize].credits += zurueck.max(0);
        self.spieler[verkaeufer as usize].credits += wert - geb_v;
        let d = self.entfernung(
            self.planeten[verkaeufer_planet as usize].koord,
            self.planeten[kaeufer_planet as usize].koord,
        );
        let dauer = self.flugdauer(d, self.regeln.markt.liefertempo, 1000);
        let lieferung = if self.kolonisationsregeln_v2() {
            EreignisArt::MarktlieferungGebunden {
                planet: kaeufer_planet,
                empfaenger: kaeufer,
                gut,
                menge,
            }
        } else {
            EreignisArt::Marktlieferung {
                planet: kaeufer_planet,
                gut,
                menge,
            }
        };
        self.plane(self.zeit + dauer, lieferung);
        self.handel.push(Handel {
            zeit: self.zeit,
            gut,
            menge,
            preis,
            kaeufer,
            verkaeufer,
        });
        let (kn, vn) = (
            self.spieler[kaeufer as usize].name.clone(),
            self.spieler[verkaeufer as usize].name.clone(),
        );
        self.vorfall(
            kaeufer,
            "markt",
            format!(
                "Kauf: {} {gut} von {vn} zu {} Credits je Einheit, Lieferung in {} min",
                ganz(menge),
                preis as f64 / 1000.0,
                dauer / 60
            ),
        );
        self.vorfall(
            verkaeufer,
            "markt",
            format!(
                "Verkauf: {} {gut} an {kn} zu {} Credits je Einheit",
                ganz(menge),
                preis as f64 / 1000.0
            ),
        );
    }

    #[allow(clippy::too_many_arguments)]
    pub fn markt_order(
        &mut self,
        sid: SpielerId,
        rolle: Rolle,
        k: Koord,
        gut: Gut,
        seite: Marktseite,
        menge: i64,
        preis: i64,
    ) -> Result<String, String> {
        let r = self.regeln.clone();
        let pid = self.eigener_planet(sid, k)?;
        let planet_id = self.planeten[pid].id;
        let markt = self.planeten[pid].gebaeude[Gebaeude::Markt.idx()] as usize;
        if markt == 0 {
            return Err(format!("auf {k} steht kein Markt"));
        }
        if menge < M || preis <= 0 {
            return Err("menge muss mindestens 1 sein und preis über 0".into());
        }
        // Cast only after the complete value plus fees fits in the fixed-point
        // account type. Otherwise an overflowing buy order could mint credits.
        let wert = menge as i128 * preis as i128 / M as i128;
        if wert > i64::MAX as i128 / 4 {
            return Err("Orderwert ist für die Festkommarechnung zu groß".into());
        }
        let offen = self
            .orders
            .iter()
            .filter(|o| o.spieler == sid && o.planet == planet_id)
            .count();
        let offen=offen+self.kommunikation.angebote.iter().filter(|o|o.von==sid&&o.planet==planet_id&&o.status=="offen").count();
        if offen >= markt * r.markt.orders_je_marktstufe {
            return Err(format!(
                "auf {k} sind höchstens {} offene Orders erlaubt",
                markt * r.markt.orders_je_marktstufe
            ));
        }
        self.abrechnen(pid);
        let mut rest = menge;
        let mut gehandelt = 0i64;
        match seite {
            Marktseite::Kauf => {
                let hinterlegt = self.hinterlegung(sid, menge, preis);
                if self.spieler[sid as usize].credits < hinterlegt {
                    return Err(format!(
                        "für die Order fehlen Credits: {} nötig einschließlich Gebühr",
                        ganz(hinterlegt)
                    ));
                }
                if let Some(t) = Self::topf_fuer(rolle, Topf::Wirtschaft) {
                    if self.spieler[sid as usize].toepfe[t.idx()] < hinterlegt {
                        return Err(format!(
                            "Topf {t} reicht nicht für die Order ({} Werteinheiten)",
                            ganz(hinterlegt)
                        ));
                    }
                    self.spieler[sid as usize].toepfe[t.idx()] -= hinterlegt;
                }
                self.spieler[sid as usize].credits -= hinterlegt;
                // Günstigste Verkaufsorder zuerst, bei gleichem Preis die ältere.
                while rest > 0 {
                    let bester = self
                        .orders
                        .iter()
                        .enumerate()
                        .filter(|(_, o)| {
                            o.gut == gut
                                && o.seite == Marktseite::Verkauf
                                && o.spieler != sid
                                && o.preis <= preis
                        })
                        .min_by_key(|(_, o)| (o.preis, o.id))
                        .map(|(i, _)| i);
                    let Some(i) = bester else {
                        break;
                    };
                    let o = self.orders[i].clone();
                    let m = rest.min(o.menge);
                    self.handel_ausfuehren(
                        sid, planet_id, preis, o.spieler, o.planet, gut, m, o.preis,
                    );
                    rest -= m;
                    gehandelt += m;
                    self.orders[i].menge -= m;
                    if self.orders[i].menge <= 0 {
                        self.orders.remove(i);
                    }
                }
            }
            Marktseite::Verkauf => {
                if self.planeten[pid].bestand[gut.idx()] < menge {
                    return Err(format!(
                        "auf {k} liegen nur {} {gut}",
                        ganz(self.planeten[pid].bestand[gut.idx()])
                    ));
                }
                self.planeten[pid].bestand[gut.idx()] -= menge;
                // Höchste Kauforder zuerst, bei gleichem Preis die ältere.
                while rest > 0 {
                    let bester = self
                        .orders
                        .iter()
                        .enumerate()
                        .filter(|(_, o)| {
                            o.gut == gut
                                && o.seite == Marktseite::Kauf
                                && o.spieler != sid
                                && o.preis >= preis
                        })
                        .min_by_key(|(_, o)| (-o.preis, o.id))
                        .map(|(i, _)| i);
                    let Some(i) = bester else {
                        break;
                    };
                    let o = self.orders[i].clone();
                    let m = rest.min(o.menge);
                    self.handel_ausfuehren(
                        o.spieler, o.planet, o.preis, sid, planet_id, gut, m, o.preis,
                    );
                    rest -= m;
                    gehandelt += m;
                    self.orders[i].menge -= m;
                    if self.orders[i].menge <= 0 {
                        self.orders.remove(i);
                    }
                }
            }
        }
        let mut text = format!("{} {gut} sofort gehandelt", ganz(gehandelt));
        if rest > 0 {
            let id = self.naechste_order;
            self.naechste_order += 1;
            self.orders.push(Order {
                id,
                spieler: sid,
                planet: planet_id,
                gut,
                seite,
                menge: rest,
                preis,
                zeit: self.zeit,
            });
            text.push_str(&format!(
                ", Order {id} über {} {gut} steht im Buch",
                ganz(rest)
            ));
        }
        Ok(text)
    }

    fn order_aufloesen(&mut self, o: &Order) {
        match o.seite {
            Marktseite::Kauf => {
                let zurueck = self.hinterlegung(o.spieler, o.menge, o.preis);
                self.spieler[o.spieler as usize].credits += zurueck;
            }
            Marktseite::Verkauf => {
                let pid = o.planet as usize;
                self.abrechnen(pid);
                self.planeten[pid].bestand[o.gut.idx()] += o.menge;
            }
        }
    }

    pub fn markt_storno(&mut self, sid: SpielerId, id: u32) -> Result<String, String> {
        let Some(i) = self
            .orders
            .iter()
            .position(|o| o.id == id && o.spieler == sid)
        else {
            return Err(format!(
                "Order {id} gibt es nicht oder sie gehört dir nicht"
            ));
        };
        let o = self.orders.remove(i);
        self.order_aufloesen(&o);
        Ok(format!("Order {id} storniert"))
    }

    /// Löst alle Orders eines Planeten auf: Credits zurück an den Spieler, Ware zurück auf den Planeten.
    pub fn orders_stornieren_planet(&mut self, planet: PlanetId) {
        let betroffen: Vec<Order> = self
            .orders
            .iter()
            .filter(|o| o.planet == planet)
            .cloned()
            .collect();
        self.orders.retain(|o| o.planet != planet);
        for o in betroffen {
            self.order_aufloesen(&o);
        }
    }

    pub fn marktlieferung(&mut self, pid: usize, gut: Gut, menge: i64) {
        if self.planeten[pid].blockade.is_some() {
            // Die neutrale Handelsflotte wartet, bis die Blockade endet.
            let id = self.planeten[pid].id;
            self.plane(
                self.zeit + STUNDE,
                EreignisArt::Marktlieferung {
                    planet: id,
                    gut,
                    menge,
                },
            );
            return;
        }
        self.abrechnen(pid);
        self.planeten[pid].bestand[gut.idx()] += menge;
        let (besitzer, k) = (self.planeten[pid].besitzer, self.planeten[pid].koord);
        self.vorfall(
            besitzer,
            "markt",
            format!("Marktlieferung auf {k}: {} {gut}", ganz(menge)),
        );
    }
    /// The neutral carrier keeps the buyer identity across ownership changes.
    /// It first reaches its old physical destination, then flies to the buyer's home.
    pub fn marktlieferung_gebunden(
        &mut self,
        pid: usize,
        empfaenger: SpielerId,
        gut: Gut,
        menge: i64,
    ) {
        let p = &self.planeten[pid];
        if p.besitzer != empfaenger {
            let home = self.spieler[empfaenger as usize].heimat;
            let von = p.koord;
            let ziel = self.planeten[home as usize].koord;
            let dauer = self.flugdauer(
                self.entfernung(von, ziel),
                self.regeln.markt.liefertempo,
                1000,
            );
            self.plane(
                self.zeit + dauer,
                EreignisArt::MarktlieferungGebunden {
                    planet: home,
                    empfaenger,
                    gut,
                    menge,
                },
            );
            self.fachereignis("market_delivery_redirected",empfaenger,serde_json::json!({"from":von,"target":ziel,"good":gut,"amount":menge,"arrival":self.zeit+dauer}));
            self.vorfall(empfaenger,"markt",format!("Marktlieferung an {von} wegen Besitzerwechsel unterwegs zur Heimat {ziel}; zusätzliche Flugzeit {dauer}s"));
            return;
        }
        if p.blockade.is_some() {
            self.plane(
                self.zeit + STUNDE,
                EreignisArt::MarktlieferungGebunden {
                    planet: p.id,
                    empfaenger,
                    gut,
                    menge,
                },
            );
            return;
        }
        self.abrechnen(pid);
        self.planeten[pid].bestand[gut.idx()] += menge;
        self.fachereignis(
            "market_delivery_completed",
            empfaenger,
            serde_json::json!({"planet":pid,"good":gut,"amount":menge}),
        );
        self.vorfall(
            empfaenger,
            "markt",
            format!(
                "Marktlieferung auf {}: {} {gut}",
                self.planeten[pid].koord,
                ganz(menge)
            ),
        );
    }

    /// Bester Verkaufs- und Kaufpreis je Gut, in Tausendstel Credits.
    pub fn marktpreise(&self, gut: Gut) -> (Option<i64>, Option<i64>) {
        let brief = self
            .orders
            .iter()
            .filter(|o| o.gut == gut && o.seite == Marktseite::Verkauf)
            .map(|o| o.preis)
            .min();
        let geld = self
            .orders
            .iter()
            .filter(|o| o.gut == gut && o.seite == Marktseite::Kauf)
            .map(|o| o.preis)
            .max();
        (brief, geld)
    }
}
