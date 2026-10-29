//! Interne Spielerart (20.10.2026, Karl): Der Server weiß jederzeit, ob ein Platz von einem Skriptbot,
//! einem reinen Agenten, einem Menschen oder einem Menschen mit Agenten gespielt wird. Die Art ist
//! ausschließlich eine Sache des Servers und der Spielleitung: Kein Spielerantwortpfad (Lobby, Rangliste,
//! Planetensicht, Briefe, Diplomatie, Markt) liest sie, und Handel, Pakte und Allianzen laufen über alle
//! Arten hinweg durch dieselben Kernregeln. Das Kernflag `Spieler.ki` taugt dafür nicht (im Server überall
//! aus) und wird hier nie benutzt.
//!
//! Woher die Art kommt:
//! * Bot: der Platz steht in `runtime.bots` (Skriptregister).
//! * Sonst die Spielweise des Kontos, das den Platz hält (`accounts.mode`): „mensch“ = Mensch (der Server sperrt
//!   dort jede Maschine, `harness::maschine_erlaubt`), „agent“ = reiner Agent, „gemischt“ = Mensch mit Agent.
//! * Ohne Konto ist der Platz frei; ein Konto ohne Platz ist Zuschauer.
//!
//! Was nachverfolgt wird (Tabelle `spielerart_stand`, Ereignisse im Kontoprotokoll `account_events`):
//! * Jeder Wechsel der Art mit Zeit und Ereignisname (`agent_dazugenommen`, `agent_entfernt`,
//!   `mensch_uebernimmt`, `agent_uebernimmt`, `platz_besetzt`).
//! * Jede Agentenfreigabe (Beginn, Ende, Kanal Harness oder Browser-Modellteam, erste Nutzung überhaupt).
//! * Zähler der letzten Spielstunde je Platz aus dem Aktionsprotokoll `game_actions` (Rolle „alle“ = direkter
//!   Browser-/Sitzungsbefehl, Rolle einer Regierungsrolle = Agentenfreigabe), ohne neue Kernfelder.
use super::*;

/// Fenster der Stundenzähler in Spielzeit.
const FENSTER: i64 = STUNDE;
/// So viele jüngste Protokollzeilen liest eine Abfrage höchstens; reicht das Fenster weiter zurück, meldet die
/// Auswertung `fenster_vollstaendig:false` statt eines Fehlers.
const MAX_ZEILEN: i64 = 40_000;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Spielerart {
    /// Skriptbot des Servers.
    Bot,
    /// Reiner Agentenspieler (Spielweise „agent“): Maschinen spielen, kein Mensch am Browser.
    Agent,
    /// Mensch ohne Agenten (Spielweise „mensch“): der Server sperrt Freigaben und Harness-Schlüssel.
    Mensch,
    /// Mensch, der Agenten dazunehmen darf oder dazugenommen hat (Spielweise „gemischt“).
    MenschMitAgent,
    /// Konto ohne Spielplatz.
    Zuschauer,
    /// Startplatz ohne Konto.
    Frei,
}

impl Spielerart {
    pub const ALLE: [Spielerart; 6] = [
        Spielerart::Bot,
        Spielerart::Agent,
        Spielerart::Mensch,
        Spielerart::MenschMitAgent,
        Spielerart::Zuschauer,
        Spielerart::Frei,
    ];
    pub fn name(self) -> &'static str {
        match self {
            Spielerart::Bot => "bot",
            Spielerart::Agent => "agent",
            Spielerart::Mensch => "mensch",
            Spielerart::MenschMitAgent => "mensch_mit_agent",
            Spielerart::Zuschauer => "zuschauer",
            Spielerart::Frei => "frei",
        }
    }
    pub fn aus_name(s: &str) -> Option<Self> {
        Self::ALLE.into_iter().find(|a| a.name() == s)
    }
    /// Art aus der Spielweise eines Kontos mit Platz. Ein unbekannter Modus verhält sich im Server wie
    /// „gemischt“ (gesperrt werden Maschinen nur bei „mensch“ und „zuschauer“), also wird er auch so geführt.
    pub fn aus_modus(modus: &str) -> Self {
        match modus {
            "mensch" => Spielerart::Mensch,
            "agent" => Spielerart::Agent,
            "zuschauer" => Spielerart::Zuschauer,
            _ => Spielerart::MenschMitAgent,
        }
    }
    /// Spielt diese Art ganz oder teilweise über Maschinen (Skript oder Agent)?
    pub fn maschine_beteiligt(self) -> bool {
        matches!(self, Spielerart::Bot | Spielerart::Agent | Spielerart::MenschMitAgent)
    }
    /// Ist ein Mensch am Browser beteiligt?
    pub fn mensch_beteiligt(self) -> bool {
        matches!(self, Spielerart::Mensch | Spielerart::MenschMitAgent)
    }
}

/// Richtwerte für die Auffälligkeitsanzeige je Art und Spielstunde: (Aktionen, Angriffe, Briefe, Markt).
/// Sie greifen nie ins Spiel ein, sie färben nur die Anzeige der Spielleitung. Bots: gemessen an 30 Bots über
/// 20 Spieltage (20.10.2026), jeweils größter Wert eines Bots in einer Spielstunde: 39 Aktionen, 0 abgelehnt,
/// 1 Angriff, 2 Briefe, 8 Markt-Aufträge; die Richtwerte liegen beim Drei- bis Zwölffachen. Mensch und Agent: weit
/// unterhalb der Quote (20 Züge à 8 Aktionen je Minute), aber über dem, was ein Mensch oder ein Modell
/// dauerhaft entscheidet (nicht gemessen, nach oben offen gehalten).
fn richtwerte(art: Spielerart) -> (u32, u32, u32, u32) {
    match art {
        Spielerart::Bot => (120, 12, 12, 40),
        Spielerart::Agent | Spielerart::MenschMitAgent => (1200, 60, 60, 120),
        _ => (600, 30, 30, 60),
    }
}
/// Ablehnquote ab dieser Zahl Aktionen im Fenster: mindestens die Hälfte abgelehnt gilt als auffällig.
const ABLEHN_MIN_AKTIONEN: u32 = 20;

/// Zähler eines Platzes im Fenster, gelesen aus `game_actions`.
#[derive(Default, Clone, Debug)]
pub(crate) struct Zaehler {
    aktionen: u32,
    abgelehnt: u32,
    angriffe: u32,
    briefe: u32,
    markt: u32,
    diplomatie: u32,
    /// Aktionen mit Rolle „alle“ von einem Konto: direkter Befehl aus Browser oder Sitzung.
    browser: u32,
    /// Aktionen mit einer Regierungsrolle von einem Konto: Agentenfreigabe (Harness oder Modellteam).
    agent: u32,
    /// Aktionen eines Skriptbots.
    skript: u32,
}

#[derive(Clone, Debug)]
struct Stand {
    konto: i64,
    art: String,
    seit: i64,
    seit_spielzeit: i64,
    wechsel: i64,
    agent_aktiv: bool,
    agent_seit: Option<i64>,
    agent_erstmals: Option<i64>,
    agent_zuletzt: Option<i64>,
    agent_sitzungen: i64,
}

#[derive(Clone, Debug)]
struct Platz {
    konto: i64,
    name: String,
    modus: String,
    gesperrt: bool,
}

pub(crate) fn setup(db: &Connection) -> Result<(), String> {
    db.execute_batch(
        "CREATE TABLE IF NOT EXISTS spielerart_stand(world_id TEXT NOT NULL,sid INTEGER NOT NULL,account INTEGER NOT NULL,art TEXT NOT NULL,seit INTEGER NOT NULL,seit_spielzeit INTEGER NOT NULL,wechsel INTEGER NOT NULL DEFAULT 0,agent_aktiv INTEGER NOT NULL DEFAULT 0,agent_seit INTEGER,agent_erstmals INTEGER,agent_zuletzt INTEGER,agent_sitzungen INTEGER NOT NULL DEFAULT 0,PRIMARY KEY(world_id,sid));",
    )
    .map_err(|e| e.to_string())
}

/// Ereignisname eines Artwechsels, so wie ihn die Spielleitung im Kontoprotokoll liest.
fn wechsel_name(von: Spielerart, nach: Spielerart) -> &'static str {
    use Spielerart::*;
    match (von, nach) {
        (Frei | Zuschauer, _) => "platz_besetzt",
        (Mensch, MenschMitAgent) => "agent_dazugenommen",
        (MenschMitAgent, Mensch) => "agent_entfernt",
        (Agent, Mensch) | (Agent, MenschMitAgent) => "mensch_uebernimmt",
        (Mensch, Agent) | (MenschMitAgent, Agent) => "agent_uebernimmt",
        _ => "art_geaendert",
    }
}

impl Game {
    /// Art eines Spielplatzes. Bots zuerst (Skriptregister), dann die Spielweise des Kontos, sonst frei.
    pub fn spieler_art(&self, sid: SpielerId) -> Spielerart {
        if self.runtime.bots.contains_key(&sid) {
            return Spielerart::Bot;
        }
        if sid as usize >= self.world.spieler.len() {
            return Spielerart::Frei;
        }
        let modus: Option<String> = self.db.query_row("SELECT mode FROM accounts WHERE sid=?1", [sid], |r| r.get(0)).optional().ok().flatten();
        modus.map_or(Spielerart::Frei, |m| Spielerart::aus_modus(&m))
    }

    /// Art eines Kontos: mit Platz die des Platzes, ohne Platz Zuschauer.
    pub fn konto_art(&self, a: &Account) -> Spielerart {
        match a.sid {
            Some(_) => Spielerart::aus_modus(&a.mode),
            None => Spielerart::Zuschauer,
        }
    }

    /// Wie `konto_art`, aber aus der Kontonummer (Spielleitungsansichten); unbekanntes Konto gilt als frei.
    pub(crate) fn konto_art_nach_id(&self, id: i64) -> Spielerart {
        let row: Option<(Option<u16>, String)> = self.db.query_row("SELECT sid,mode FROM accounts WHERE id=?1", [id], |r| Ok((r.get(0)?, r.get(1)?))).optional().ok().flatten();
        match row {
            Some((Some(_), modus)) => Spielerart::aus_modus(&modus),
            Some((None, _)) => Spielerart::Zuschauer,
            None => Spielerart::Frei,
        }
    }

    fn plaetze_mit_konto(&self) -> Result<BTreeMap<SpielerId, Platz>, (u16, String)> {
        let fehler = |_: rusqlite::Error| err(503, "Konten nicht lesbar");
        let mut stmt = self.db.prepare_cached("SELECT sid,id,name,mode,banned FROM accounts WHERE sid IS NOT NULL").map_err(fehler)?;
        let rows = stmt
            .query_map([], |r| Ok((r.get::<_, u16>(0)?, Platz { konto: r.get(1)?, name: r.get(2)?, modus: r.get(3)?, gesperrt: r.get(4)? })))
            .map_err(fehler)?;
        rows.collect::<Result<BTreeMap<_, _>, _>>().map_err(fehler)
    }

    fn spielerart_staende(&self) -> Result<BTreeMap<SpielerId, Stand>, (u16, String)> {
        let fehler = |_: rusqlite::Error| err(503, "Spielerart nicht lesbar");
        let mut stmt = self
            .db
            .prepare_cached("SELECT sid,account,art,seit,seit_spielzeit,wechsel,agent_aktiv,agent_seit,agent_erstmals,agent_zuletzt,agent_sitzungen FROM spielerart_stand WHERE world_id=?1")
            .map_err(fehler)?;
        let rows = stmt
            .query_map([&self.runtime.world_id], |r| {
                Ok((
                    r.get::<_, u16>(0)?,
                    Stand {
                        konto: r.get(1)?,
                        art: r.get(2)?,
                        seit: r.get(3)?,
                        seit_spielzeit: r.get(4)?,
                        wechsel: r.get(5)?,
                        agent_aktiv: r.get(6)?,
                        agent_seit: r.get(7)?,
                        agent_erstmals: r.get(8)?,
                        agent_zuletzt: r.get(9)?,
                        agent_sitzungen: r.get(10)?,
                    },
                ))
            })
            .map_err(fehler)?;
        rows.collect::<Result<BTreeMap<_, _>, _>>().map_err(fehler)
    }

    /// Beim Öffnen einer Welt: erfasst alle Plätze mit Konto und beendet Freigaben, die ein Neustart gekappt hat.
    pub(crate) fn spielerart_start(&self) -> Result<(), (u16, String)> {
        self.spielerart_abgleich(true)
    }

    /// Läuft am Ende jeder Spieltransaktion (siehe `transaction_with`): vergleicht die aktuelle Art und den
    /// Freigabezustand jedes Platzes mit dem gespeicherten Stand, schreibt Wechsel und Freigabe-Ereignisse mit Zeit
    /// ins Kontoprotokoll und führt den Stand nach. Billig: zwei kleine Abfragen über die belegten Plätze.
    pub(crate) fn spielerart_abgleichen(&self) -> Result<(), (u16, String)> {
        self.spielerart_abgleich(false)
    }

    fn spielerart_abgleich(&self, start: bool) -> Result<(), (u16, String)> {
        let fehler = |_: rusqlite::Error| err(503, "Spielerart nicht gespeichert");
        let plaetze = self.plaetze_mit_konto()?;
        let staende = self.spielerart_staende()?;
        if plaetze.is_empty() && staende.is_empty() {
            return Ok(());
        }
        let (jetzt, spielzeit, wid) = (now(), self.world.zeit, &self.runtime.world_id);
        // Erster belegter Platz dieser Welt (neue Welt nach einem Reset): Stände früherer Welten räumen.
        if staende.is_empty() {
            self.db.execute("DELETE FROM spielerart_stand WHERE world_id<>?1", [wid]).map_err(fehler)?;
        }
        for (sid, p) in &plaetze {
            let art = Spielerart::aus_modus(&p.modus);
            let lease = self.runtime.leases.get(sid).filter(|l| l.until > jetzt);
            let aktiv = lease.is_some();
            let kanal = lease.map(|l| if l.token.is_some() { "harness" } else { "browser" });
            let rollen = lease.map(|l| l.roles.clone());
            let Some(alt) = staende.get(sid).filter(|s| s.konto == p.konto) else {
                // Neuer Platzinhaber, oder die Welt wird zum ersten Mal erfasst (ältere Spielstände).
                let ereignis = if start { "erfasst" } else { "platz_besetzt" };
                self.identity_event(Some(p.konto), "spielerart_wechsel", json!({"sid":sid,"von":Spielerart::Frei,"nach":art,"ereignis":ereignis,"modus":p.modus,"spielzeit":spielzeit}))?;
                if aktiv {
                    self.identity_event(Some(p.konto), "agent_freigabe_gestartet", json!({"sid":sid,"art":art,"kanal":kanal,"rollen":rollen,"erstmals":true,"spielzeit":spielzeit}))?;
                }
                self.db
                    .execute(
                        "INSERT INTO spielerart_stand(world_id,sid,account,art,seit,seit_spielzeit,wechsel,agent_aktiv,agent_seit,agent_erstmals,agent_zuletzt,agent_sitzungen) VALUES(?1,?2,?3,?4,?5,?6,0,?7,?8,?8,NULL,?9) \
                         ON CONFLICT(world_id,sid) DO UPDATE SET account=excluded.account,art=excluded.art,seit=excluded.seit,seit_spielzeit=excluded.seit_spielzeit,wechsel=0,agent_aktiv=excluded.agent_aktiv,agent_seit=excluded.agent_seit,agent_erstmals=excluded.agent_erstmals,agent_zuletzt=NULL,agent_sitzungen=excluded.agent_sitzungen",
                        params![wid, sid, p.konto, art.name(), jetzt, spielzeit, aktiv, aktiv.then_some(jetzt), aktiv as i64],
                    )
                    .map_err(fehler)?;
                continue;
            };
            let von = Spielerart::aus_name(&alt.art).unwrap_or(Spielerart::Frei);
            if von != art {
                self.identity_event(
                    Some(p.konto),
                    "spielerart_wechsel",
                    json!({"sid":sid,"von":von,"nach":art,"ereignis":wechsel_name(von, art),"modus":p.modus,"vorher_seit":alt.seit,"spielzeit":spielzeit}),
                )?;
                self.db
                    .execute("UPDATE spielerart_stand SET art=?3,seit=?4,seit_spielzeit=?5,wechsel=wechsel+1 WHERE world_id=?1 AND sid=?2", params![wid, sid, art.name(), jetzt, spielzeit])
                    .map_err(fehler)?;
            }
            if alt.agent_aktiv != aktiv {
                if aktiv {
                    let token = lease.and_then(|l| l.token);
                    self.identity_event(Some(p.konto), "agent_freigabe_gestartet", json!({"sid":sid,"art":art,"kanal":kanal,"rollen":rollen,"schluessel":token,"erstmals":alt.agent_erstmals.is_none(),"spielzeit":spielzeit}))?;
                    self.db
                        .execute(
                            "UPDATE spielerart_stand SET agent_aktiv=1,agent_seit=?3,agent_erstmals=coalesce(agent_erstmals,?3),agent_sitzungen=agent_sitzungen+1 WHERE world_id=?1 AND sid=?2",
                            params![wid, sid, jetzt],
                        )
                        .map_err(fehler)?;
                } else {
                    let grund = if start { "neustart" } else { "beendet" };
                    self.identity_event(Some(p.konto), "agent_freigabe_beendet", json!({"sid":sid,"art":art,"grund":grund,"dauer_s":alt.agent_seit.map(|s| (jetzt - s).max(0)),"spielzeit":spielzeit}))?;
                    self.db.execute("UPDATE spielerart_stand SET agent_aktiv=0,agent_zuletzt=?3 WHERE world_id=?1 AND sid=?2", params![wid, sid, jetzt]).map_err(fehler)?;
                }
            }
        }
        Ok(())
    }

    /// Zähler der letzten Spielstunde je Platz aus dem Aktionsprotokoll. Der zweite Wert sagt, ob das Fenster
    /// vollständig gelesen wurde (sonst deckt die Auswertung nur die jüngsten `MAX_ZEILEN` Aktionen ab).
    fn fenster_zaehler(&self) -> Result<(BTreeMap<SpielerId, Zaehler>, bool), (u16, String)> {
        let fehler = |_: rusqlite::Error| err(503, "Aktionsprotokoll nicht lesbar");
        let start = self.world.zeit - FENSTER;
        let mut stmt = self
            .db
            .prepare_cached(
                "SELECT simulation_seconds,actor,role,accepted,CASE WHEN json_valid(action_json) THEN json_extract(action_json,'$.typ') END,CASE WHEN json_valid(action_json) THEN json_extract(action_json,'$.mission') END \
                 FROM game_actions WHERE world_id=?1 ORDER BY id DESC LIMIT ?2",
            )
            .map_err(fehler)?;
        let mut rows = stmt.query(params![self.runtime.world_id, MAX_ZEILEN]).map_err(fehler)?;
        let mut aus: BTreeMap<SpielerId, Zaehler> = BTreeMap::new();
        let (mut gelesen, mut ende_erreicht) = (0i64, false);
        while let Some(r) = rows.next().map_err(fehler)? {
            gelesen += 1;
            let zeit: i64 = r.get(0).map_err(fehler)?;
            if zeit < start {
                ende_erreicht = true;
                break;
            }
            let actor: u16 = r.get(1).map_err(fehler)?;
            let rolle: String = r.get(2).map_err(fehler)?;
            let angenommen: bool = r.get(3).map_err(fehler)?;
            let typ: Option<String> = r.get(4).map_err(fehler)?;
            let mission: Option<String> = r.get(5).map_err(fehler)?;
            let typ = typ.as_deref().unwrap_or("");
            // Das Ende eines Agentenaufrufs ist Buchführung der Rolle, keine Spielaktion.
            if typ == "aufruf_ende" {
                continue;
            }
            let z = aus.entry(actor).or_default();
            z.aktionen += 1;
            if !angenommen {
                z.abgelehnt += 1;
            }
            if self.runtime.bots.contains_key(&actor) {
                z.skript += 1;
            } else if rolle.eq_ignore_ascii_case("alle") {
                z.browser += 1;
            } else {
                z.agent += 1;
            }
            let feindlich = typ == "flotte_senden" && mission.as_deref().and_then(Mission::aus_name).is_some_and(|m| m.feindlich());
            if feindlich || typ == "raketen_starten" {
                z.angriffe += 1;
            }
            match typ {
                "brief_senden" | "nachricht" => z.briefe += 1,
                "markt_order" | "markt_storno" | "intern_anbieten" | "intern_kaufen" | "intern_storno" | "soforthandel" | "schenken" => z.markt += 1,
                t if t.starts_with("vertrag_") || t.starts_with("diplomatie_") || t.starts_with("allianz_") => z.diplomatie += 1,
                _ => {}
            }
        }
        let vollstaendig = ende_erreicht || gelesen < MAX_ZEILEN;
        Ok((aus, vollstaendig))
    }

    /// Fair-Play-Zustand je Konto (Spielleitung).
    fn fairplay_staende(&self) -> BTreeMap<i64, (String, i64)> {
        self.db
            .prepare_cached("SELECT account,status,verstoesse FROM fairplay_status")
            .and_then(|mut s| s.query_map([], |r| Ok((r.get::<_, i64>(0)?, (r.get::<_, String>(1)?, r.get::<_, i64>(2)?)))).and_then(|rows| rows.collect::<Result<BTreeMap<_, _>, _>>()))
            .unwrap_or_default()
    }

    /// Aktive Harness-Schlüssel und letzte Nutzung je Konto in dieser Welt.
    fn schluessel_staende(&self) -> BTreeMap<i64, (i64, Option<i64>)> {
        self.db
            .prepare_cached("SELECT account,sum(CASE WHEN revoked=0 AND expires>?2 THEN 1 ELSE 0 END),max(last_used) FROM harness_tokens WHERE world_id=?1 GROUP BY account")
            .and_then(|mut s| {
                s.query_map(params![self.runtime.world_id, now()], |r| Ok((r.get::<_, i64>(0)?, (r.get::<_, i64>(1)?, r.get::<_, Option<i64>>(2)?)))).and_then(|rows| rows.collect::<Result<BTreeMap<_, _>, _>>())
            })
            .unwrap_or_default()
    }

    /// Alles, was die Einzelansicht eines Platzes braucht, einmal geladen.
    fn spielerart_kontext(&self) -> Result<Kontext, (u16, String)> {
        let (fenster, voll) = self.fenster_zaehler()?;
        Ok(Kontext { plaetze: self.plaetze_mit_konto()?, staende: self.spielerart_staende()?, fenster, voll, schluessel: self.schluessel_staende(), fairplay: self.fairplay_staende() })
    }

    fn art_im_kontext(&self, sid: SpielerId, k: &Kontext) -> Spielerart {
        if self.runtime.bots.contains_key(&sid) {
            Spielerart::Bot
        } else if let Some(p) = k.plaetze.get(&sid) {
            Spielerart::aus_modus(&p.modus)
        } else {
            Spielerart::Frei
        }
    }

    fn spieler_eintrag(&self, sid: SpielerId, k: &Kontext) -> Value {
        let art = self.art_im_kontext(sid, k);
        let s = &self.world.spieler[sid as usize];
        let st = &s.statistik;
        let platz = k.plaetze.get(&sid);
        let stand = k.staende.get(&sid).filter(|x| Some(x.konto) == platz.map(|p| p.konto));
        let z = k.fenster.get(&sid).cloned().unwrap_or_default();
        let fairplay = platz.and_then(|p| k.fairplay.get(&p.konto));
        let jetzt = now();
        let lease = self.runtime.leases.get(&sid).filter(|l| l.until > jetzt);
        let agent = platz.map(|p| {
            let (aktive, zuletzt_genutzt) = k.schluessel.get(&p.konto).copied().unwrap_or((0, None));
            let bericht = self.runtime.agent_status.get(&sid);
            json!({"erlaubt":art.maschine_beteiligt(),"freigabe_aktiv":lease.is_some(),"rollen":lease.map(|l|&l.roles),"kanal":lease.map(|l|if l.token.is_some(){"harness"}else{"browser"}),
                "freigabe_bis":lease.map(|l|l.until),"seit":stand.and_then(|x|x.agent_seit),"erstmals":stand.and_then(|x|x.agent_erstmals),"zuletzt_beendet":stand.and_then(|x|x.agent_zuletzt),"sitzungen":stand.map(|x|x.agent_sitzungen).unwrap_or(0),
                "nutzung":if lease.is_some(){"aktiv"}else if stand.is_some_and(|x|x.agent_erstmals.is_some()){"frueher"}else{"nie"},
                "schluessel_aktiv":aktive,"schluessel_zuletzt":zuletzt_genutzt,
                "bericht":bericht.map(|b|json!({"status":b["status"],"calls":b["calls"],"errors":b["errors"],"tokens":b["tokens"],"accepted":b["accepted"],"rejected":b["rejected"],"at":b["at"],"provider":b["provider"],"model":b["model"]}))})
        });
        // Widerspruch zwischen Art und beobachtetem Kanal: dort liegt die Ursache, wenn „der Falsche“ durchdreht.
        let abweichung = match art {
            Spielerart::Agent if z.browser > 0 => Some("Agentenspieler, aber direkte Befehle aus Browser oder Sitzung (Mensch greift ein)"),
            Spielerart::Mensch if z.agent > 0 => Some("Menschenspieler, aber Befehle über eine Agentenfreigabe"),
            _ => None,
        };
        let (r_akt, r_ang, r_brief, r_markt) = richtwerte(art);
        let mut auffaellig = Vec::<String>::new();
        if art.maschine_beteiligt() || art.mensch_beteiligt() {
            if z.aktionen > r_akt {
                auffaellig.push(format!("{} Aktionen in der letzten Spielstunde (Richtwert {r_akt})", z.aktionen));
            }
            if z.aktionen >= ABLEHN_MIN_AKTIONEN && z.abgelehnt * 2 >= z.aktionen {
                auffaellig.push(format!("{} von {} Aktionen abgelehnt", z.abgelehnt, z.aktionen));
            }
            if z.angriffe > r_ang {
                auffaellig.push(format!("{} Angriffe in der letzten Spielstunde (Richtwert {r_ang})", z.angriffe));
            }
            if z.briefe > r_brief {
                auffaellig.push(format!("{} Briefe in der letzten Spielstunde (Richtwert {r_brief})", z.briefe));
            }
            if z.markt > r_markt {
                auffaellig.push(format!("{} Markt-Aufträge in der letzten Spielstunde (Richtwert {r_markt})", z.markt));
            }
            if let Some((status, n)) = fairplay {
                auffaellig.push(format!("Fair-Play: {status} ({n} Verstöße)"));
            }
            if let Some(a) = abweichung {
                auffaellig.push(a.to_string());
            }
        }
        json!({"spieler":sid,"name":s.name,"art":art,"aktiv":self.world.spieler_aktiv(sid),
            "erklaert":if art==Spielerart::Bot{Value::from("skript")}else{platz.map(|p|json!(p.modus)).unwrap_or(Value::Null)},
            "bottyp":self.runtime.bots.get(&sid).map(|b|b.typ),
            "konto":platz.map(|p|p.konto),"konto_name":platz.map(|p|&p.name),"gesperrt":platz.map(|p|p.gesperrt),
            "seit":stand.map(|x|x.seit),"seit_spielzeit":if art==Spielerart::Bot{Some(0)}else{stand.map(|x|x.seit_spielzeit)},"wechsel":stand.map(|x|x.wechsel).unwrap_or(0),
            "agent":agent,
            "stunde":{"aktionen":z.aktionen,"abgelehnt":z.abgelehnt,"angriffe":z.angriffe,"briefe":z.briefe,"markt":z.markt,"diplomatie":z.diplomatie,"kanal":{"browser":z.browser,"agent":z.agent,"skript":z.skript}},
            "gesamt":{"aktionen":st.aktionen,"abgelehnt":st.abgelehnt,"angriffe":st.angriffe,"vertragsbrueche":st.vertragsbrueche},
            "fairplay":fairplay.map(|(status,n)|json!({"status":status,"verstoesse":n})),
            "abweichung":abweichung,"auffaellig":auffaellig})
    }

    /// Verlauf der Artwechsel und Agentenfreigaben eines Kontos, jüngste zuerst (Kontoprotokoll).
    pub(crate) fn spielerart_verlauf(&self, konto: i64) -> Result<Vec<Value>, (u16, String)> {
        let fehler = |_: rusqlite::Error| err(503, "Kontoprotokoll nicht lesbar");
        let mut stmt = self
            .db
            .prepare_cached("SELECT time,kind,details FROM account_events WHERE account=?1 AND kind IN('spielerart_wechsel','agent_freigabe_gestartet','agent_freigabe_beendet') ORDER BY id DESC LIMIT 100")
            .map_err(fehler)?;
        let rows = stmt
            .query_map([konto], |r| Ok(json!({"time":r.get::<_, i64>(0)?,"kind":r.get::<_, String>(1)?,"details":serde_json::from_str::<Value>(&r.get::<_, String>(2)?).unwrap_or(Value::Null)})))
            .map_err(fehler)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(fehler)
    }

    /// Einzelansicht für die Spielleitung (Konto-Details).
    pub(crate) fn spielerart_eintrag(&self, sid: SpielerId) -> Value {
        if sid as usize >= self.world.spieler.len() {
            return Value::Null;
        }
        match self.spielerart_kontext() {
            Ok(k) => self.spieler_eintrag(sid, &k),
            Err((_, e)) => json!({"fehler":e}),
        }
    }

    /// Spielleitungsansicht im Monitoring: je Platz die Art mit Beginn und Kennzahlen, dazu Summen je Art.
    pub(crate) fn spielerarten_status(&self) -> Value {
        let k = match self.spielerart_kontext() {
            Ok(k) => k,
            Err((_, e)) => return json!({"fehler":e}),
        };
        #[derive(Default)]
        struct Metrik {
            summe: u64,
            max: u32,
            max_spieler: Option<SpielerId>,
        }
        impl Metrik {
            fn zaehlen(&mut self, sid: SpielerId, n: u32) {
                self.summe += n as u64;
                if n > self.max {
                    self.max = n;
                    self.max_spieler = Some(sid);
                }
            }
            fn json(&self) -> Value {
                json!({"summe":self.summe,"max":self.max,"max_spieler":self.max_spieler})
            }
        }
        #[derive(Default)]
        struct Summe {
            anzahl: u32,
            aktiv: u32,
            auffaellig: u32,
            aktionen: Metrik,
            abgelehnt: Metrik,
            angriffe: Metrik,
            briefe: Metrik,
            markt: Metrik,
            gesamt: (u64, u64, u64),
            agent_aktiv: u32,
        }
        let mut je_art: BTreeMap<Spielerart, Summe> = BTreeMap::new();
        let mut spieler = Vec::new();
        let mut auffaellige = Vec::new();
        let mut frei = 0u32;
        for sid in 0..self.world.spieler.len() as SpielerId {
            let art = self.art_im_kontext(sid, &k);
            if art == Spielerart::Frei {
                frei += 1;
                continue;
            }
            let e = self.spieler_eintrag(sid, &k);
            let z = k.fenster.get(&sid).cloned().unwrap_or_default();
            let st = &self.world.spieler[sid as usize].statistik;
            let sum = je_art.entry(art).or_default();
            sum.anzahl += 1;
            sum.aktiv += self.world.spieler_aktiv(sid) as u32;
            sum.aktionen.zaehlen(sid, z.aktionen);
            sum.abgelehnt.zaehlen(sid, z.abgelehnt);
            sum.angriffe.zaehlen(sid, z.angriffe);
            sum.briefe.zaehlen(sid, z.briefe);
            sum.markt.zaehlen(sid, z.markt);
            sum.gesamt.0 += st.aktionen as u64;
            sum.gesamt.1 += st.abgelehnt as u64;
            sum.gesamt.2 += st.angriffe as u64;
            sum.agent_aktiv += e["agent"]["freigabe_aktiv"].as_bool().unwrap_or(false) as u32;
            if e["auffaellig"].as_array().is_some_and(|a| !a.is_empty()) {
                sum.auffaellig += 1;
                auffaellige.push(json!({"spieler":sid,"name":e["name"],"art":art,"gruende":e["auffaellig"]}));
            }
            spieler.push(e);
        }
        let mut arten = serde_json::Map::new();
        for art in [Spielerart::Bot, Spielerart::Agent, Spielerart::Mensch, Spielerart::MenschMitAgent] {
            let s = je_art.remove(&art).unwrap_or_default();
            arten.insert(
                art.name().into(),
                json!({"anzahl":s.anzahl,"aktiv":s.aktiv,"agent_freigaben_aktiv":s.agent_aktiv,"auffaellig":s.auffaellig,
                    "stunde":{"aktionen":s.aktionen.json(),"abgelehnt":s.abgelehnt.json(),"angriffe":s.angriffe.json(),"briefe":s.briefe.json(),"markt":s.markt.json()},
                    "gesamt":{"aktionen":s.gesamt.0,"abgelehnt":s.gesamt.1,"angriffe":s.gesamt.2},
                    "richtwerte":{"aktionen":richtwerte(art).0,"angriffe":richtwerte(art).1,"briefe":richtwerte(art).2,"markt":richtwerte(art).3}}),
            );
        }
        let ohne_platz: i64 = self.db.query_row("SELECT count(*) FROM accounts WHERE sid IS NULL", [], |r| r.get(0)).unwrap_or(0);
        arten.insert("zuschauer".into(), json!({"konten":ohne_platz,"warteliste":self.runtime.waitlist.len()}));
        arten.insert("frei".into(), json!({"plaetze":frei}));
        json!({"fenster_stunden":FENSTER / STUNDE,"fenster_vollstaendig":k.voll,"arten":arten,"auffaellig":auffaellige,"spieler":spieler,
            "hinweis":"Art, Beginn und Zähler nur für die Spielleitung; keine Spielerantwort enthält sie. Richtwerte färben nur die Anzeige."})
    }
}

/// Einmal geladene Daten für die Platzansichten.
struct Kontext {
    plaetze: BTreeMap<SpielerId, Platz>,
    staende: BTreeMap<SpielerId, Stand>,
    fenster: BTreeMap<SpielerId, Zaehler>,
    voll: bool,
    schluessel: BTreeMap<i64, (i64, Option<i64>)>,
    fairplay: BTreeMap<i64, (String, i64)>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use kern::welt::Vertragsstatus;

    /// Online-ähnliche Welt ohne laufende Bots (genaue Zähler).
    fn spiel() -> Game {
        let mut g = Game::open(&std::env::temp_dir().join(format!("spielerart-{}", token())), 83).unwrap();
        g.runtime.bots_enabled = false;
        g
    }
    /// Lokale Welt: Skriptbots antworten innerhalb des Befehls der Spieler (wie in der App).
    fn lokal() -> Game {
        Game::open_local(&std::env::temp_dir().join(format!("spielerart-{}", token())), 712, 30).unwrap()
    }
    fn konto(g: &mut Game, name: &str, mode: &str) -> Account {
        let v = g.register(&json!({"name":name}), "test-hash".into()).unwrap();
        let a = g.authenticate(v["token"].as_str().unwrap()).unwrap();
        g.claim(&a, &json!({"mode":mode,"volk":"krath"})).unwrap();
        g.account_by_name(name).unwrap()
    }
    fn frisch(g: &Game, a: &Account) -> Account {
        g.account_by_name(&a.name).unwrap()
    }
    fn waehle(g: &mut Game, a: &Account, mode: &str) -> Account {
        g.claim(a, &json!({"mode":mode,"volk":"krath"})).unwrap();
        frisch(g, a)
    }
    fn heimat(g: &Game, sid: SpielerId) -> (usize, String) {
        let pid = g.world.spieler[sid as usize].heimat as usize;
        (pid, g.world.planeten[pid].koord.to_string())
    }
    /// Direkter Befehl wie aus dem Browser (Rolle „alle“).
    fn befehl(g: &mut Game, a: &Account, id: &str, aktionen: Value) -> Value {
        let wid = g.runtime.world_id.clone();
        g.command(a, &json!({"world_id":wid,"request_id":id,"aktionen":aktionen})).unwrap()
    }
    /// Freigabe im Browser (Modellteam); liefert das Freigabegeheimnis.
    fn freigabe(g: &mut Game, a: &Account, rollen: Value) -> String {
        let wid = g.runtime.world_id.clone();
        g.lease(a, &json!({"world_id":wid,"action":"start","roles":rollen})).unwrap()["lease"].as_str().unwrap().to_string()
    }
    fn freigabe_stop(g: &mut Game, a: &Account) {
        let wid = g.runtime.world_id.clone();
        g.lease(a, &json!({"world_id":wid,"action":"stop"})).unwrap();
    }
    /// Befehl über die Agentenfreigabe einer Regierungsrolle.
    fn agent_befehl(g: &mut Game, a: &Account, lease: &str, rolle: &str, id: &str, aktionen: Value) -> Value {
        let wid = g.runtime.world_id.clone();
        g.command(a, &json!({"world_id":wid,"request_id":id,"lease":lease,"rolle":rolle,"aktionen":aktionen})).unwrap()
    }
    /// Protokollierte Ereignisse der Art, älteste zuerst: (Name, Details).
    fn verlauf(g: &Game, a: &Account) -> Vec<(String, Value)> {
        let mut v: Vec<(String, Value)> = g
            .spielerart_verlauf(a.id)
            .unwrap()
            .into_iter()
            .map(|e| {
                let name = if e["kind"] == "spielerart_wechsel" { e["details"]["ereignis"].as_str().unwrap().to_string() } else { e["kind"].as_str().unwrap().to_string() };
                (name, e["details"].clone())
            })
            .collect();
        v.reverse();
        v
    }
    fn namen(v: &[(String, Value)]) -> Vec<&str> {
        v.iter().map(|(n, _)| n.as_str()).collect()
    }
    fn eintrag(g: &Game, sid: SpielerId) -> Value {
        g.spielerart_eintrag(sid)
    }

    #[test]
    fn art_folgt_register_und_spielweise_und_nie_dem_kernflag() {
        let mut g = spiel();
        // Das Kernflag ist im Server für alle aus; es kann Bots nicht von Menschen trennen.
        assert!(g.world.spieler.iter().all(|s| !s.ki));
        assert_eq!(g.spieler_art(0), Spielerart::Bot);
        assert_eq!(g.spieler_art(29), Spielerart::Bot);
        assert_eq!(g.spieler_art(30), Spielerart::Frei);
        assert_eq!(g.spieler_art(9999), Spielerart::Frei);
        let m = konto(&mut g, "Tivar", "mensch");
        let a = konto(&mut g, "Quenta", "agent");
        let gm = konto(&mut g, "Rilke", "gemischt");
        let z = {
            let v = g.register(&json!({"name":"Zaungast"}), "test-hash".into()).unwrap();
            g.authenticate(v["token"].as_str().unwrap()).unwrap()
        };
        assert_eq!(g.spieler_art(m.sid.unwrap()), Spielerart::Mensch);
        assert_eq!(g.spieler_art(a.sid.unwrap()), Spielerart::Agent);
        assert_eq!(g.spieler_art(gm.sid.unwrap()), Spielerart::MenschMitAgent);
        assert_eq!((g.konto_art(&m), g.konto_art(&a), g.konto_art(&gm), g.konto_art(&z)), (Spielerart::Mensch, Spielerart::Agent, Spielerart::MenschMitAgent, Spielerart::Zuschauer));
        // Ein unbekannter Modus verhält sich im Server wie „gemischt“ und wird auch so geführt.
        assert_eq!(Spielerart::aus_modus("ki"), Spielerart::MenschMitAgent);
        for art in Spielerart::ALLE {
            assert_eq!(Spielerart::aus_name(art.name()), Some(art));
        }
        let st = g.admin_status();
        let arten = &st["monitor"]["spielerarten"]["arten"];
        assert_eq!(arten["bot"]["anzahl"], 30);
        assert_eq!((arten["mensch"]["anzahl"].as_u64(), arten["agent"]["anzahl"].as_u64(), arten["mensch_mit_agent"]["anzahl"].as_u64()), (Some(1), Some(1), Some(1)));
        assert_eq!(arten["zuschauer"]["konten"], 1);
        assert_eq!(arten["frei"]["plaetze"], 17);
        // Jedes Konto der Spielleitung trägt seine Art.
        let konten = st["accounts"].as_array().unwrap();
        let art_von = |name: &str| konten.iter().find(|k| k["name"] == name).unwrap()["art"].clone();
        assert_eq!((art_von("Tivar"), art_von("Quenta"), art_von("Rilke"), art_von("Zaungast")), (json!("mensch"), json!("agent"), json!("mensch_mit_agent"), json!("zuschauer")));
        // Je Platz: Art, Spielweise, Konto, Beginn.
        let plaetze = st["monitor"]["spielerarten"]["spieler"].as_array().unwrap();
        assert_eq!(plaetze.len(), 33, "30 Bots und drei besetzte Plätze; freie Startplätze stehen nur als Zahl");
        let bot = plaetze.iter().find(|p| p["spieler"] == 0).unwrap();
        assert_eq!((&bot["art"], &bot["erklaert"], &bot["konto"]), (&json!("bot"), &json!("skript"), &Value::Null));
        assert!(bot["bottyp"].is_string());
        let mensch = plaetze.iter().find(|p| p["spieler"] == m.sid.unwrap()).unwrap();
        assert_eq!((&mensch["art"], &mensch["erklaert"], &mensch["konto_name"]), (&json!("mensch"), &json!("mensch"), &json!("Tivar")));
        assert!(mensch["seit"].as_i64().unwrap() >= now() - 60);
        assert_eq!(mensch["agent"]["nutzung"], "nie");
        // Die Einzelansicht der Spielleitung (Konto-Details) trägt Art und Verlauf.
        let d = g.player_details(&json!({"id":gm.id})).unwrap();
        assert_eq!(d["spielerart"]["art"], "mensch_mit_agent");
        assert_eq!(d["spielerart_verlauf"][0]["details"]["ereignis"], "platz_besetzt");
        assert!(g.player_details(&json!({"id":z.id})).unwrap()["spielerart"].is_null());
    }

    #[test]
    fn uebergaenge_zwischen_mensch_agent_und_gemischt_werden_protokolliert() {
        let start = now();
        let mut g = spiel();
        let mut a = konto(&mut g, "Umsteiger", "mensch");
        let sid = a.sid.unwrap();
        // 1. Mensch -> Mensch mit Agent: der Spieler nimmt später Agenten dazu.
        a = waehle(&mut g, &a, "gemischt");
        assert_eq!(g.spieler_art(sid), Spielerart::MenschMitAgent);
        // Dieselbe Wahl noch einmal ist kein Wechsel.
        a = waehle(&mut g, &a, "gemischt");
        // 2. Der Agent beginnt zu spielen: Harness-Schlüssel, Freigabe.
        let k = g.harness_schluessel_erstellen(&a, &json!({"label":"Test","rollen":["verwalter"],"regeln_akzeptiert":harness::REGELN_VERSION})).unwrap();
        let (ha, hs) = g.harness_authenticate(k["schluessel"].as_str().unwrap()).unwrap();
        g.harness_freigabe(&ha, &hs, &json!({"aktion":"start"})).unwrap();
        let e = eintrag(&g, sid);
        assert_eq!((&e["agent"]["freigabe_aktiv"], &e["agent"]["kanal"], &e["agent"]["nutzung"]), (&json!(true), &json!("harness"), &json!("aktiv")));
        assert_eq!(e["agent"]["schluessel_aktiv"], 1);
        g.harness_freigabe(&ha, &hs, &json!({"aktion":"stop"})).unwrap();
        // 3. Dasselbe über das Modellteam im Browser.
        freigabe(&mut g, &a, json!(["diplomat"]));
        freigabe_stop(&mut g, &a);
        // 4. Mensch mit Agent -> reiner Agent -> Mensch mit Agent -> Mensch.
        a = waehle(&mut g, &a, "agent");
        a = waehle(&mut g, &a, "gemischt");
        a = waehle(&mut g, &a, "mensch");
        assert_eq!(g.spieler_art(sid), Spielerart::Mensch);
        let v = verlauf(&g, &a);
        assert_eq!(
            namen(&v),
            ["platz_besetzt", "agent_dazugenommen", "agent_freigabe_gestartet", "agent_freigabe_beendet", "agent_freigabe_gestartet", "agent_freigabe_beendet", "agent_uebernimmt", "mensch_uebernimmt", "agent_entfernt"]
        );
        // Beschreibung der Ereignisse: Art vorher und nachher, Kanal, erste Nutzung, Zeit.
        assert_eq!((&v[0].1["von"], &v[0].1["nach"]), (&json!("frei"), &json!("mensch")));
        assert_eq!((&v[1].1["von"], &v[1].1["nach"], &v[1].1["modus"]), (&json!("mensch"), &json!("mensch_mit_agent"), &json!("gemischt")));
        assert_eq!((&v[2].1["kanal"], &v[2].1["erstmals"], &v[2].1["art"]), (&json!("harness"), &json!(true), &json!("mensch_mit_agent")));
        assert_eq!(v[2].1["schluessel"], k["id"]);
        assert_eq!(v[3].1["grund"], "beendet");
        assert_eq!((&v[4].1["kanal"], &v[4].1["erstmals"]), (&json!("browser"), &json!(false)));
        assert_eq!((&v[6].1["von"], &v[6].1["nach"]), (&json!("mensch_mit_agent"), &json!("agent")));
        assert_eq!((&v[7].1["von"], &v[7].1["nach"]), (&json!("agent"), &json!("mensch_mit_agent")));
        assert_eq!((&v[8].1["von"], &v[8].1["nach"]), (&json!("mensch_mit_agent"), &json!("mensch")));
        // Ereigniszeit steht im Kontoprotokoll.
        let zeiten: Vec<i64> = g.spielerart_verlauf(a.id).unwrap().iter().map(|e| e["time"].as_i64().unwrap()).collect();
        assert!(zeiten.len() == 9 && zeiten.iter().all(|t| *t >= start && *t <= now()));
        // Stand: vier Wechsel, zwei Freigabe-Sitzungen, Agentennutzung „früher“, Beginn der Art ist der letzte Wechsel.
        let e = eintrag(&g, sid);
        assert_eq!((&e["wechsel"], &e["agent"]["sitzungen"], &e["agent"]["nutzung"], &e["agent"]["freigabe_aktiv"]), (&json!(4), &json!(2), &json!("frueher"), &json!(false)));
        assert!(e["agent"]["erstmals"].is_i64() && e["agent"]["zuletzt_beendet"].is_i64());
        assert_eq!(e["art"], "mensch");
        // „Mensch“ sperrt Maschinen weiter: Schlüssel widerrufen, keine neue Freigabe.
        assert_eq!(g.harness_authenticate(k["schluessel"].as_str().unwrap()).unwrap_err().0, 401);
        let wid = g.runtime.world_id.clone();
        assert_eq!(g.lease(&a, &json!({"world_id":wid,"action":"start","roles":["verwalter"]})).unwrap_err().0, 403);
    }

    #[test]
    fn freigabe_ohne_stopp_wird_beim_ablauf_beendet_und_ein_rollback_hinterlaesst_nichts() {
        let mut g = spiel();
        let a = konto(&mut g, "Ablaufer", "gemischt");
        let sid = a.sid.unwrap();
        freigabe(&mut g, &a, json!(["verwalter"]));
        assert_eq!(eintrag(&g, sid)["agent"]["freigabe_aktiv"], true);
        // Ohne Puls läuft die Freigabe ab (90 s); der nächste Zeitschritt führt das Protokoll nach.
        g.runtime.leases.get_mut(&sid).unwrap().until = now() - 1;
        g.advance(60).unwrap();
        let v = verlauf(&g, &a);
        assert_eq!(namen(&v), ["platz_besetzt", "agent_freigabe_gestartet", "agent_freigabe_beendet"]);
        assert_eq!(v[2].1["grund"], "beendet");
        assert!(v[2].1["dauer_s"].as_i64().unwrap() >= 0);
        // Scheitert die Speicherung der Transaktion, verschwinden Wechsel und Ereignis mit ihr.
        let vorher = g.spielerart_verlauf(a.id).unwrap().len();
        g.db.execute_batch("CREATE TRIGGER fail_checkpoint BEFORE UPDATE ON checkpoint BEGIN SELECT RAISE(FAIL,'test'); END;").unwrap();
        assert!(g.claim(&a, &json!({"mode":"mensch","volk":"krath"})).is_err());
        assert_eq!(g.account_by_name("Ablaufer").unwrap().mode, "gemischt");
        assert_eq!(g.spielerart_verlauf(a.id).unwrap().len(), vorher);
        assert_eq!(eintrag(&g, sid)["art"], "mensch_mit_agent");
        assert_eq!(eintrag(&g, sid)["wechsel"], 0);
    }

    #[test]
    fn neustart_beendet_freigaben_im_protokoll_und_behaelt_den_beginn() {
        let mut g = spiel();
        let a = konto(&mut g, "Neustarter", "gemischt");
        let sid = a.sid.unwrap();
        freigabe(&mut g, &a, json!(["verwalter"]));
        let seit = eintrag(&g, sid)["seit"].clone();
        let root = g.data_root.clone();
        drop(g);
        let g = Game::open(&root, 1).unwrap();
        let v = verlauf(&g, &a);
        assert_eq!(namen(&v), ["platz_besetzt", "agent_freigabe_gestartet", "agent_freigabe_beendet"], "Beginn bleibt, kein zweites „platz_besetzt“");
        assert_eq!(v[2].1["grund"], "neustart");
        let e = eintrag(&g, sid);
        assert_eq!((&e["seit"], &e["agent"]["freigabe_aktiv"], &e["agent"]["nutzung"]), (&seit, &json!(false), &json!("frueher")));
        // Spielstand aus der Zeit vor der Spielerart: Plätze werden beim Öffnen erfasst, nicht neu „besetzt“.
        g.db.execute("DELETE FROM spielerart_stand", []).unwrap();
        g.db.execute("DELETE FROM account_events WHERE kind IN('spielerart_wechsel','agent_freigabe_gestartet','agent_freigabe_beendet')", []).unwrap();
        drop(g);
        let g = Game::open(&root, 1).unwrap();
        let v = verlauf(&g, &a);
        assert_eq!(namen(&v), ["erfasst"]);
        assert_eq!(v[0].1["nach"], "mensch_mit_agent");
        assert_eq!(g.spieler_art(sid), Spielerart::MenschMitAgent);
    }

    #[test]
    fn reset_beginnt_die_art_neu_und_raeumt_alte_staende() {
        let mut g = spiel();
        let a = konto(&mut g, "Resetter", "gemischt");
        let alte_welt = g.runtime.world_id.clone();
        g.admin(&json!({"world_id":alte_welt,"action":"reset","confirm":format!("RESET {alte_welt}")})).unwrap();
        assert_eq!(g.spieler_art(a.sid.unwrap()), Spielerart::Frei);
        let a = frisch(&g, &a);
        assert_eq!(g.konto_art(&a), Spielerart::Zuschauer, "nach dem Reset sind alle Konten Zuschauer");
        let a = waehle(&mut g, &a, "agent");
        assert_eq!(g.spieler_art(a.sid.unwrap()), Spielerart::Agent);
        let v = verlauf(&g, &a);
        assert_eq!(namen(&v), ["platz_besetzt", "platz_besetzt"], "je Welt einmal besetzt");
        assert_eq!((&v[1].1["von"], &v[1].1["nach"]), (&json!("frei"), &json!("agent")));
        let welten: i64 = g.db.query_row("SELECT count(DISTINCT world_id) FROM spielerart_stand", [], |r| r.get(0)).unwrap();
        assert_eq!(welten, 1, "nur die Stände der laufenden Welt bleiben");
        assert_eq!(eintrag(&g, a.sid.unwrap())["wechsel"], 0);
    }

    #[test]
    fn kennzahlen_je_spieler_und_art_zeigen_kanal_und_auffaelligkeit() {
        let mut g = spiel();
        let m = konto(&mut g, "Zaehlmensch", "mensch");
        let a = konto(&mut g, "Zaehlagent", "agent");
        let gm = konto(&mut g, "Zaehlgemischt", "gemischt");
        let (_, mk) = heimat(&g, m.sid.unwrap());
        let (_, ak) = heimat(&g, a.sid.unwrap());
        let bot_name = g.world.spieler[0].name.clone();
        // Mensch: 27 aussichtslose Rückrufe (abgelehnt), ein Angriff, ein Marktauftrag, drei Briefe.
        let mut gesamt = 0;
        for i in 0..4 {
            let n = if i < 3 { 8 } else { 3 };
            let aktionen: Vec<Value> = (0..n).map(|_| json!({"typ":"flotte_zurueckrufen","flotte":999_999})).collect();
            let r = befehl(&mut g, &m, &format!("rueck-{i}"), json!(aktionen));
            assert!(r["ergebnisse"].as_array().unwrap().iter().all(|e| e["ok"] == false));
            gesamt += n;
        }
        befehl(&mut g, &m, "angriff", json!([{"typ":"flotte_senden","start":mk,"ziel":"1:1:1","mission":"angriff","schiffe":{"spionagesonde":1}}]));
        befehl(&mut g, &m, "markt", json!([{"typ":"markt_order","planet":mk,"gut":"erz","seite":"kauf","menge":10,"preis":1.0}]));
        for i in 0..3 {
            befehl(&mut g, &m, &format!("brief-{i}"), json!([{"typ":"brief_senden","kanal":"privat","an":bot_name,"betreff":"Hallo","text":format!("Guten Tag {i}")}]));
        }
        let e = eintrag(&g, m.sid.unwrap());
        assert_eq!(e["stunde"]["aktionen"], gesamt + 5);
        assert_eq!((&e["stunde"]["angriffe"], &e["stunde"]["briefe"], &e["stunde"]["markt"]), (&json!(1), &json!(3), &json!(1)));
        assert_eq!((&e["stunde"]["kanal"]["browser"], &e["stunde"]["kanal"]["agent"], &e["stunde"]["kanal"]["skript"]), (&json!(gesamt + 5), &json!(0), &json!(0)));
        assert!(e["stunde"]["abgelehnt"].as_u64().unwrap() >= gesamt as u64);
        assert_eq!(e["gesamt"]["aktionen"].as_u64().unwrap(), (gesamt + 5) as u64, "stimmt mit der Kernstatistik überein");
        assert!(e["abweichung"].is_null());
        assert!(e["auffaellig"].as_array().unwrap().iter().any(|t| t.as_str().unwrap().contains("abgelehnt")), "{e}");
        // Agent: fünf Meldungen über die Freigabe, eine direkte Browserbefehl-Abweichung.
        let lease = freigabe(&mut g, &a, json!(["verwalter"]));
        for i in 0..5 {
            agent_befehl(&mut g, &a, &lease, "verwalter", &format!("agent-{i}"), json!([{"typ":"meldung","text":format!("Lage {i}")}]));
        }
        let e = eintrag(&g, a.sid.unwrap());
        assert_eq!((&e["stunde"]["kanal"]["agent"], &e["stunde"]["kanal"]["browser"]), (&json!(5), &json!(0)));
        assert!(e["abweichung"].is_null());
        assert_eq!(e["agent"]["rollen"], json!(["verwalter"]));
        befehl(&mut g, &a, "mensch-greift-ein", json!([{"typ":"meldung","text":"Ich bin es selbst"}]));
        let e = eintrag(&g, a.sid.unwrap());
        assert_eq!(e["stunde"]["kanal"]["browser"], 1);
        assert!(e["abweichung"].as_str().unwrap().contains("Agentenspieler"));
        assert!(e["auffaellig"].as_array().unwrap().iter().any(|t| t.as_str().unwrap().contains("Agentenspieler")));
        // Mensch mit Agent: beide Kanäle getrennt gezählt.
        let lease2 = freigabe(&mut g, &gm, json!(["verwalter"]));
        agent_befehl(&mut g, &gm, &lease2, "verwalter", "g1", json!([{"typ":"meldung","text":"vom Agenten"}]));
        befehl(&mut g, &gm, "g2", json!([{"typ":"meldung","text":"vom Menschen"},{"typ":"meldung","text":"noch einmal"}]));
        let e = eintrag(&g, gm.sid.unwrap());
        assert_eq!((&e["stunde"]["kanal"]["agent"], &e["stunde"]["kanal"]["browser"]), (&json!(1), &json!(2)));
        assert!(e["abweichung"].is_null(), "beide Kanäle sind bei Mensch mit Agent erlaubt");
        // Summen je Art und Liste der Auffälligen in der Spielleitungsansicht.
        let s = g.admin_status();
        let arten = &s["monitor"]["spielerarten"]["arten"];
        assert_eq!(arten["mensch"]["auffaellig"], 1);
        assert_eq!(arten["mensch"]["stunde"]["aktionen"]["max_spieler"], m.sid.unwrap());
        assert_eq!(arten["agent"]["auffaellig"], 1);
        assert_eq!(arten["mensch_mit_agent"]["auffaellig"], 0);
        assert_eq!(arten["mensch_mit_agent"]["agent_freigaben_aktiv"], 1);
        assert_eq!(arten["bot"]["auffaellig"], 0);
        let auffaellig = s["monitor"]["spielerarten"]["auffaellig"].as_array().unwrap();
        assert!(auffaellig.iter().any(|p| p["art"] == "mensch" && p["name"] == "Zaehlmensch"));
        assert!(auffaellig.iter().any(|p| p["art"] == "agent" && p["name"] == "Zaehlagent"));
        assert!(!auffaellig.iter().any(|p| p["art"] == "bot"));
        // Das Fenster gleitet: nach zwei Spielstunden sind die Stundenzähler leer, die Gesamtzähler bleiben.
        g.advance(3600).unwrap();
        g.advance(3600).unwrap();
        let e = eintrag(&g, m.sid.unwrap());
        assert_eq!((&e["stunde"]["aktionen"], &e["gesamt"]["aktionen"]), (&json!(0), &json!(gesamt + 5)));
        assert!(e["auffaellig"].as_array().unwrap().is_empty());
        let _ = ak;
    }

    #[test]
    fn bots_laufen_in_der_bilanz_mit_und_bleiben_unter_den_richtwerten() {
        let mut g = Game::open(&std::env::temp_dir().join(format!("spielerart-{}", token())), 83).unwrap();
        let (mut gesehen, mut bot_stunden) = (0u64, 0u32);
        for _ in 0..48 {
            g.advance(3600).unwrap();
            let s = g.spielerarten_status();
            gesehen += s["arten"]["bot"]["stunde"]["aktionen"]["summe"].as_u64().unwrap();
            assert!(s["auffaellig"].as_array().unwrap().is_empty(), "Richtwert der Bots zu knapp: {}", s["auffaellig"]);
            assert_eq!(s["fenster_vollstaendig"], true);
            // Bots zählen unter „skript“, nie unter Browser oder Agent.
            for p in s["spieler"].as_array().unwrap().iter().filter(|p| p["stunde"]["aktionen"].as_u64().unwrap() > 0) {
                assert_eq!(p["art"], "bot");
                assert_eq!(p["stunde"]["kanal"]["skript"], p["stunde"]["aktionen"]);
                assert!(p["stunde"]["kanal"]["browser"] == 0 && p["stunde"]["kanal"]["agent"] == 0);
                bot_stunden += 1;
            }
        }
        assert!(gesehen > 0 && bot_stunden > 0, "die Bots handeln, und die Zähler sehen es");
        let s = g.spielerarten_status();
        assert_eq!(s["arten"]["bot"]["anzahl"], 30);
        assert!(s["arten"]["bot"]["gesamt"]["aktionen"].as_u64().unwrap() > 0);
    }

    #[test]
    fn fairplay_befund_nennt_die_art_und_alte_datenbanken_werden_nachgeruestet() {
        let mut g = spiel();
        let m = konto(&mut g, "Kratzer", "mensch");
        let a = konto(&mut g, "Kratzbot", "agent");
        let k = g.koeder(None, "lobby").unwrap();
        let t = g.fairplay_pruefen(&json!({"notiz":k.code}), "").unwrap();
        // Ein Menschenkonto liest die Oberfläche aus, ein Agentenkonto tut es über die Harness-API.
        assert_eq!(g.fairplay_verstoss(&m, &t, "/api/command").0, 403);
        assert_eq!(g.fairplay_verstoss(&a, &t, "/api/harness/v1/handeln").0, 403);
        let admin = g.fairplay_admin();
        let v = admin["vorfaelle"].as_array().unwrap();
        assert_eq!((&v[0]["name"], &v[0]["art"], &v[0]["zugang"]), (&json!("Kratzbot"), &json!("agent"), &json!("harness")));
        assert_eq!((&v[1]["name"], &v[1]["art"], &v[1]["zugang"]), (&json!("Kratzer"), &json!("mensch"), &json!("browser")));
        assert_eq!(v[0]["art_jetzt"], "agent");
        // Wechselt die Art später, bleibt der Befund bei der Art zum Zeitpunkt des Vorfalls.
        waehle(&mut g, &m, "gemischt");
        let v = g.fairplay_admin()["vorfaelle"].clone();
        assert_eq!((&v[1]["art"], &v[1]["art_jetzt"]), (&json!("mensch"), &json!("mensch_mit_agent")));
        let status = g.fairplay_admin()["status"].clone();
        assert!(status.as_array().unwrap().iter().any(|s| s["name"] == "Kratzbot" && s["art"] == "agent"));
        // Auch das Kontoprotokoll trägt die Art; die Plätze der Verwarnten sind in der Spielleitungsansicht markiert.
        let ereignis = g.db.query_row("SELECT details FROM account_events WHERE account=?1 AND kind='fairplay_verstoss'", [a.id], |r| r.get::<_, String>(0)).unwrap();
        let ereignis: Value = serde_json::from_str(&ereignis).unwrap();
        assert_eq!((&ereignis["art"], &ereignis["zugang"]), (&json!("agent"), &json!("harness")));
        let e = eintrag(&g, a.sid.unwrap());
        assert_eq!(e["fairplay"]["status"], "verwarnt");
        assert!(e["auffaellig"].as_array().unwrap().iter().any(|t| t.as_str().unwrap().contains("Fair-Play")));
        // Öffentliche Meldungen und die eigene Kontoansicht nennen keine Art.
        let pub_text = g.fairplay_oeffentlich().to_string() + &g.fairplay_konto(a.id).to_string() + &g.me(&frisch(&g, &a)).to_string();
        for verboten in ["\"art\":\"agent\"", "\"art\":\"mensch\"", "\"art\":\"bot\"", "mensch_mit_agent", "spielerart", "zugang"] {
            assert!(!pub_text.contains(verboten), "{verboten} in {pub_text}");
        }
        // Eine Datenbank aus der Zeit vor der Art: Spalte wird nachgerüstet, Altfälle heißen „unbekannt“.
        let root = g.data_root.clone();
        g.db.execute_batch("ALTER TABLE fairplay_vorfaelle DROP COLUMN art").unwrap();
        g.db.execute("INSERT INTO fairplay_vorfaelle(zeit,account,quelle_konto,flaeche,kanal,fundstelle,wert,folge) VALUES(1,?1,NULL,'lobby','/api/command','/notiz','X','verwarnung')", [m.id]).unwrap();
        drop(g);
        let mut g = Game::open(&root, 1).unwrap();
        let v = g.fairplay_admin()["vorfaelle"].clone();
        assert_eq!(v.as_array().unwrap().iter().filter(|x| x["art"] == "unbekannt").count(), 3, "alle drei Altfälle, auch der ohne Spalte angelegte");
        assert!(v.as_array().unwrap().iter().any(|x| x["art"] == "unbekannt" && x["zeit"] == 1));
        // Neue Vorfälle tragen die Art wieder.
        let t = g.fairplay_pruefen(&json!({"notiz":k.code}), "").unwrap();
        g.fairplay_verstoss(&a, &t, "/api/harness/v1/handeln");
        assert_eq!(g.fairplay_admin()["vorfaelle"][0]["art"], "agent");
    }

    /// Das Zusammenspiel aller Arten in einer lokalen Welt (Bots antworten sofort). Liefert die Spieler zurück.
    struct Szene {
        g: Game,
        mensch: Account,
        agent: Account,
        gemischt: Account,
        lease: String,
        haendler: SpielerId,
        oekonom: SpielerId,
    }
    fn zusammenspiel() -> Szene {
        let mut g = lokal();
        let m = konto(&mut g, "Tivar", "mensch");
        let a = konto(&mut g, "Quenta", "agent");
        let gm = konto(&mut g, "Rilke", "gemischt");
        let haendler = g.runtime.bots.iter().find(|(_, b)| b.typ == Bottyp::Haendler).map(|(s, _)| *s).unwrap();
        let oekonom = g.runtime.bots.iter().find(|(_, b)| b.typ == Bottyp::Oekonom).map(|(s, _)| *s).unwrap();
        for sid in [m.sid.unwrap(), a.sid.unwrap(), gm.sid.unwrap(), haendler, oekonom] {
            let (p, _) = heimat(&g, sid);
            g.world.planeten[p].gebaeude[Gebaeude::Markt.idx()] = 2;
            g.world.planeten[p].bestand[Gut::Erz.idx()] = 100_000 * M;
            g.world.planeten[p].bestand[Gut::Kristall.idx()] = 100_000 * M;
            g.world.spieler[sid as usize].credits = 1_000_000 * M;
            g.world.raten_neu(p);
        }
        let lease = freigabe(&mut g, &a, json!(["diplomat", "verwalter"]));
        Szene { g, mensch: m, agent: a, gemischt: gm, lease, haendler, oekonom }
    }
    fn alle_ok(r: &Value) {
        assert!(r["ergebnisse"].as_array().unwrap().iter().all(|e| e["ok"] == true), "{r}");
    }
    fn vertrag(g: &Game, a: SpielerId, b: SpielerId) -> Option<(u32, Vertragsstatus)> {
        g.world.vertraege.iter().find(|v| v.art == Vertragsart::Nichtangriffspakt && ((v.a == a && v.b == b) || (v.a == b && v.b == a))).map(|v| (v.id, v.status))
    }

    #[test]
    fn handel_und_kooperation_laufen_ueber_alle_arten() {
        let Szene { mut g, mensch: m, agent: a, gemischt: gm, lease, haendler, oekonom } = zusammenspiel();
        let (ms, as_, gs) = (m.sid.unwrap(), a.sid.unwrap(), gm.sid.unwrap());
        let (haendler_name, oekonom_name) = (g.world.spieler[haendler as usize].name.clone(), g.world.spieler[oekonom as usize].name.clone());
        let (_, mk) = heimat(&g, ms);
        let (_, ak) = heimat(&g, as_);
        // Arten stimmen und ändern sich durch das Zusammenspiel nicht.
        let arten = |g: &Game| (g.spieler_art(ms), g.spieler_art(as_), g.spieler_art(gs), g.spieler_art(haendler), g.spieler_art(oekonom));
        let erwartet = (Spielerart::Mensch, Spielerart::Agent, Spielerart::MenschMitAgent, Spielerart::Bot, Spielerart::Bot);
        assert_eq!(arten(&g), erwartet);

        // Mensch <-> Bot: Brief, der Bot bietet einen Pakt an, der Mensch nimmt an.
        alle_ok(&befehl(&mut g, &m, "m1", json!([{"typ":"brief_senden","kanal":"privat","an":haendler_name,"betreff":"Frieden","text":"Frieden und Pakt?"}])));
        let (id, status) = vertrag(&g, haendler, ms).expect("Bot antwortet mit einem Paktangebot");
        assert_eq!(status, Vertragsstatus::Angeboten);
        alle_ok(&befehl(&mut g, &m, "m2", json!([{"typ":"vertrag_annehmen","vertrag":id}])));
        assert_eq!(vertrag(&g, haendler, ms).unwrap().1, Vertragsstatus::Aktiv);
        // Mensch <-> Bot: Markt. Der Bot stellt auf Zuruf Erz ein, der Mensch kauft.
        alle_ok(&befehl(&mut g, &m, "m3", json!([{"typ":"brief_senden","kanal":"privat","an":haendler_name,"betreff":"Handel","text":"Verkaufe 100 erz für 1"}])));
        assert!(g.world.orders.iter().any(|o| o.spieler == haendler && o.gut == Gut::Erz));
        alle_ok(&befehl(&mut g, &m, "m4", json!([{"typ":"markt_order","planet":mk,"gut":"erz","seite":"kauf","menge":100,"preis":1.0}])));
        assert!(g.world.handel.iter().any(|h| h.kaeufer == ms && h.verkaeufer == haendler && h.gut == Gut::Erz));

        // Agent <-> Bot: Brief über die Freigabe, Markt in Gegenrichtung, Pakt mit einem zweiten Bot.
        alle_ok(&agent_befehl(&mut g, &a, &lease, "diplomat", "a1", json!([{"typ":"brief_senden","kanal":"privat","an":haendler_name,"betreff":"Handel","text":"Kaufe 100 kristall für 1"}])));
        assert!(g.world.orders.iter().any(|o| o.spieler == haendler && o.gut == Gut::Kristall));
        alle_ok(&agent_befehl(&mut g, &a, &lease, "verwalter", "a2", json!([{"typ":"markt_order","planet":ak,"gut":"kristall","seite":"verkauf","menge":100,"preis":1.0}])));
        assert!(g.world.handel.iter().any(|h| h.kaeufer == haendler && h.verkaeufer == as_ && h.gut == Gut::Kristall));
        alle_ok(&agent_befehl(&mut g, &a, &lease, "diplomat", "a3", json!([{"typ":"vertrag_anbieten","partner":oekonom_name,"art":"nichtangriffspakt","kaution":100}])));
        assert_eq!(vertrag(&g, as_, oekonom).unwrap().1, Vertragsstatus::Aktiv, "der Bot nimmt das Angebot eines Agenten sofort an");

        // Mensch <-> Agent: Brief hin und zurück, Pakt.
        alle_ok(&befehl(&mut g, &m, "m5", json!([{"typ":"brief_senden","kanal":"privat","an":"Quenta","betreff":"Hallo","text":"Wollen wir uns absichern?"}])));
        let ansicht = g.view(&a, Rolle::Diplomat).unwrap();
        assert!(ansicht["kommunikation"]["briefkasten"].as_array().unwrap().iter().any(|b| b["von"] == "Tivar" && b["text"] == "Wollen wir uns absichern?"));
        alle_ok(&agent_befehl(&mut g, &a, &lease, "diplomat", "a4", json!([{"typ":"brief_senden","kanal":"privat","an":"Tivar","betreff":"Re: Hallo","text":"Gern, ich biete einen Pakt an."},{"typ":"vertrag_anbieten","partner":"Tivar","art":"nichtangriffspakt","kaution":100}])));
        let ansicht = g.view(&m, Rolle::Alle).unwrap();
        assert!(ansicht["kommunikation"]["briefkasten"].as_array().unwrap().iter().any(|b| b["von"] == "Quenta"));
        let (id, status) = vertrag(&g, as_, ms).unwrap();
        assert_eq!(status, Vertragsstatus::Angeboten);
        alle_ok(&befehl(&mut g, &m, "m6", json!([{"typ":"vertrag_annehmen","vertrag":id}])));
        assert_eq!(vertrag(&g, as_, ms).unwrap().1, Vertragsstatus::Aktiv);

        // Mensch mit Agent <-> Mensch und Bot.
        alle_ok(&befehl(&mut g, &gm, "g1", json!([{"typ":"vertrag_anbieten","partner":"Tivar","art":"nichtangriffspakt","kaution":100},{"typ":"vertrag_anbieten","partner":oekonom_name,"art":"nichtangriffspakt","kaution":100}])));
        let (id, _) = vertrag(&g, gs, ms).unwrap();
        alle_ok(&befehl(&mut g, &m, "m7", json!([{"typ":"vertrag_annehmen","vertrag":id}])));
        assert_eq!(vertrag(&g, gs, ms).unwrap().1, Vertragsstatus::Aktiv);
        assert_eq!(vertrag(&g, gs, oekonom).unwrap().1, Vertragsstatus::Aktiv);

        // Allianz über alle Arten: der Mensch gründet, lädt Agent, Mensch mit Agent und einen Bot ein, alle treten bei.
        alle_ok(&befehl(&mut g, &m, "al1", json!([{"typ":"allianz_gruenden","name":"Vierfach","tag":"VIF"}])));
        alle_ok(&befehl(&mut g, &m, "al2", json!([{"typ":"allianz_einladen","spieler":"Quenta"},{"typ":"allianz_einladen","spieler":"Rilke"},{"typ":"allianz_einladen","spieler":oekonom_name}])));
        alle_ok(&agent_befehl(&mut g, &a, &lease, "diplomat", "a5", json!([{"typ":"allianz_beitreten","allianz":"Vierfach"}])));
        alle_ok(&befehl(&mut g, &gm, "g2", json!([{"typ":"allianz_beitreten","allianz":"Vierfach"}])));
        let allianz = g.world.spieler[ms as usize].allianz.expect("Gründer ist in seiner Allianz");
        for sid in [as_, gs, oekonom] {
            assert_eq!(g.world.spieler[sid as usize].allianz, Some(allianz), "Spieler {sid} ({}) ist in der Allianz", g.spieler_art(sid).name());
        }
        // Und die Arten sind danach noch dieselben.
        assert_eq!(arten(&g), erwartet);
        // Bei der Spielleitung stehen alle Kooperationen unter der jeweils richtigen Art (Briefe, Markt, Diplomatie).
        let e = eintrag(&g, ms);
        assert!(e["stunde"]["briefe"].as_u64().unwrap() >= 3 && e["stunde"]["markt"].as_u64().unwrap() >= 1 && e["stunde"]["diplomatie"].as_u64().unwrap() >= 3, "{e}");
        let e = eintrag(&g, as_);
        assert!(e["stunde"]["kanal"]["agent"].as_u64().unwrap() >= 6 && e["stunde"]["kanal"]["browser"] == 0, "{e}");
        let e = eintrag(&g, haendler);
        assert!(e["stunde"]["kanal"]["skript"].as_u64().unwrap() >= 4 && e["art"] == "bot", "{e}");
    }

    /// Alle Schlüssel und Zeichenketten eines JSON-Baums.
    fn durchsuchen(v: &Value, f: &mut dyn FnMut(&str)) {
        match v {
            Value::String(s) => f(s),
            Value::Array(a) => a.iter().for_each(|x| durchsuchen(x, f)),
            Value::Object(o) => o.iter().for_each(|(k, x)| {
                f(k);
                durchsuchen(x, f)
            }),
            _ => {}
        }
    }
    fn schluessel(v: &Value) -> Vec<String> {
        let mut k: Vec<String> = v.as_object().map(|o| o.keys().cloned().collect()).unwrap_or_default();
        k.sort();
        k
    }

    #[test]
    fn spielersicht_verraet_die_art_nicht() {
        let Szene { mut g, mensch: m, agent: a, gemischt: gm, lease, haendler, oekonom } = zusammenspiel();
        let (ms, as_, gs) = (m.sid.unwrap(), a.sid.unwrap(), gm.sid.unwrap());
        let (hn, on) = (g.world.spieler[haendler as usize].name.clone(), g.world.spieler[oekonom as usize].name.clone());
        // Von jeder Art geht Post an jede andere: Briefe, Pakte, Markt.
        for (i, an) in [hn.as_str(), "Quenta", "Rilke"].into_iter().enumerate() {
            befehl(&mut g, &m, &format!("v-m-{i}"), json!([{"typ":"brief_senden","kanal":"privat","an":an,"betreff":"Frieden","text":"Frieden und Pakt?"}]));
        }
        agent_befehl(&mut g, &a, &lease, "diplomat", "v-a-1", json!([{"typ":"brief_senden","kanal":"privat","an":"Tivar","betreff":"Hallo","text":"Guten Tag"},{"typ":"brief_senden","kanal":"privat","an":on,"betreff":"Frieden","text":"Frieden und Pakt?"}]));
        befehl(&mut g, &gm, "v-g-1", json!([{"typ":"brief_senden","kanal":"privat","an":"Tivar","betreff":"Hallo","text":"Guten Tag"},{"typ":"brief_senden","kanal":"privat","an":hn,"betreff":"Handel","text":"Verkaufe 100 erz für 1"}]));
        // Alles, was die drei Spieler sehen können.
        let k = g.harness_schluessel_erstellen(&a, &json!({"label":"Sicht","rollen":["diplomat"],"regeln_akzeptiert":harness::REGELN_VERSION})).unwrap();
        let (ha, hs) = g.harness_authenticate(k["schluessel"].as_str().unwrap()).unwrap();
        let mut ausgaben: Vec<(String, Value)> = vec![("lobby".into(), g.lobby())];
        for (name, acc) in [("mensch", &m), ("agent", &a), ("gemischt", &gm)] {
            ausgaben.push((format!("me-{name}"), g.me(acc)));
            ausgaben.push((format!("view-{name}"), g.view(acc, Rolle::Alle).unwrap()));
            ausgaben.push((format!("tool-{name}"), g.tool(acc, &json!({"typ":"regelkontext"})).unwrap()));
        }
        for rolle in [Rolle::Stratege, Rolle::Verwalter, Rolle::Feldherr, Rolle::Diplomat] {
            ausgaben.push((format!("agentensicht-{rolle}"), g.view(&a, rolle).unwrap()));
        }
        ausgaben.push(("harness-lage".into(), g.harness_lage(&ha, &hs, Some("diplomat")).unwrap()));
        ausgaben.push(("harness-ich".into(), g.harness_ich(&ha, &hs)));
        ausgaben.push(("harness-info".into(), g.harness_info()));
        ausgaben.push(("fairplay".into(), g.fairplay_oeffentlich()));
        assert!(ausgaben.iter().all(|(_, v)| !v.is_null()));

        // 1. Kein Wort der Spielerart in irgendeiner Spielerantwort (Schlüssel wie Texte).
        let verboten = ["spielerart", "mensch_mit_agent", "bottyp", "erklaert", "art_jetzt", "quelle_art", "agent_freigabe", "spielerarten"];
        for (wo, v) in &ausgaben {
            durchsuchen(v, &mut |s| {
                for w in verboten {
                    assert!(!s.contains(w), "{wo}: „{w}“ in „{s}“");
                }
            });
            assert!(!v.to_string().contains("\"art\":\"bot\"") && !v.to_string().contains("\"art\":\"agent\""), "{wo}");
        }
        // 2. Eigene Kontodaten ja, Fremdes nein: `mode`/`agent` stehen nur in der eigenen Sicht und in jeder gleich.
        let sichten: Vec<&Value> = ausgaben.iter().filter(|(n, _)| ["view-mensch", "view-agent", "view-gemischt"].contains(&n.as_str())).map(|(_, v)| v).collect();
        assert_eq!(schluessel(sichten[0]), schluessel(sichten[1]));
        assert_eq!(schluessel(sichten[1]), schluessel(sichten[2]));
        // 3. Jeder Eintrag über einen anderen Spieler hat dieselbe Form, ob Bot, Agent, Mensch oder Mensch mit Agent.
        let mut formen: BTreeMap<String, std::collections::BTreeSet<Vec<String>>> = BTreeMap::new();
        for v in &sichten {
            for kontakt in v["kommunikation"]["kontakte"].as_array().unwrap() {
                formen.entry("kontakt".into()).or_default().insert(schluessel(kontakt));
            }
            for brief in v["kommunikation"]["briefkasten"].as_array().unwrap() {
                // Brief eines Bots, eines Agenten, eines Menschen und eines Menschen mit Agent: eine Form.
                let von = brief["von"].as_str().unwrap();
                let art = if von == hn || von == on { "bot" } else if von == "Quenta" { "agent" } else if von == "Rilke" { "gemischt" } else { "mensch" };
                formen.entry("brief".into()).or_default().insert(schluessel(brief));
                formen.entry(format!("brief-von-{art}")).or_default().insert(schluessel(&brief["von_beziehung"]));
            }
            for eintrag in v["rangliste"].as_array().unwrap() {
                formen.entry("rangliste".into()).or_default().insert(vec![format!("{}", eintrag.as_array().unwrap().len())]);
            }
            for vt in v["vertraege"].as_array().unwrap() {
                formen.entry("vertrag".into()).or_default().insert(schluessel(vt));
            }
        }
        for (was, f) in &formen {
            assert_eq!(f.len(), 1, "{was} hat je nach Absender verschiedene Formen: {f:?}");
        }
        for was in ["brief-von-bot", "brief-von-agent", "brief-von-mensch", "brief-von-gemischt"] {
            assert!(formen.contains_key(was), "kein Brief von {was} im Postfach, der Vergleich wäre leer");
        }
        // 4. Der Wechsel der Art ändert nichts, was andere von diesem Platz sehen.
        let vorher = (g.view(&m, Rolle::Alle).unwrap()["kommunikation"]["kontakte"].clone(), g.lobby());
        let gm = waehle(&mut g, &gm, "mensch");
        let gm = waehle(&mut g, &gm, "agent");
        waehle(&mut g, &gm, "gemischt");
        let nachher = (g.view(&m, Rolle::Alle).unwrap()["kommunikation"]["kontakte"].clone(), g.lobby());
        assert_eq!(vorher.0, nachher.0, "Kontakte und Beziehungen bleiben gleich");
        let ohne_zaehler = |mut l: Value| {
            l.as_object_mut().unwrap().remove("revision");
            l
        };
        assert_eq!(ohne_zaehler(vorher.1), ohne_zaehler(nachher.1), "Lobby und Rangliste bleiben gleich");
        let _ = (ms, as_, gs);
    }
}
