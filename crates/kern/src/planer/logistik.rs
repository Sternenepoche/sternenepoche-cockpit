//! Read-only quotes execute the validated command on a disposable copy.
use crate::{aktion::Aktion, typen::*, welt::EreignisArt, Welt};
use serde_json::{json, Value};
impl Welt {
    pub fn missionsplan(&self, sid: SpielerId, action: &Value) -> Result<Value, String> {
        self.missionsplan_als(sid, Rolle::Alle, action)
    }
    /// Vorschau in der Rolle des Anfragenden: Ein Agent als Verwalter bekommt dieselbe Ablehnung
    /// (Zuständigkeit, Topf), die ihm der echte Befehl gäbe – vorher prüfte die Vorschau immer als „alle“.
    pub fn missionsplan_als(&self, sid: SpielerId, rolle: Rolle, action: &Value) -> Result<Value, String> {
        let a: Aktion = serde_json::from_value(action.clone()).map_err(|e| e.to_string())?;
        let (start, ziel, mission) = match a {
            Aktion::FlotteSenden {
                start,
                ziel,
                mission,
                ..
            } => (start, ziel, mission),
            _ => return Err("missionsplan benötigt flotte_senden".into()),
        };
        let pid = self.eigener_planet(sid, start)?;
        // A hostile/unknown destination is not validated against hidden actual defenses.
        if !matches!(mission, Mission::Kolonisieren | Mission::Abbau | Mission::Recyceln | Mission::Terraformen) && (!matches!(mission, Mission::Transport | Mission::Stationieren)
            || self.eigener_planet(sid, ziel).is_err())
        {
            return Ok(
                json!({"status":"unknown","grund":"Exakte Missionszusage derzeit nur für Transport und Stationieren zwischen eigenen Planeten; Außenmissionen benötigen bekannte Zielbedingungen"}),
            );
        }
        let mut w = self.clone();
        let fid = w.naechste_flotte;
        let (ok, reason) = w.handeln(sid, rolle, action);
        if !ok {
            return Ok(json!({"status":"blocked","grund":reason,"habitabilitaet":self.klima_sicht(sid,ziel)}));
        }
        let f = w
            .flotten
            .get(&fid)
            .ok_or("Flugauftrag erzeugte keine Flotte")?;
        let before = self.bestand_jetzt(pid);
        let after = w.bestand_jetzt(pid);
        let delta: Vec<_> = before.iter().zip(after).map(|(a, b)| a - b).collect();
        // Terraformen (0.4.7): Stand und Bedarf am Ziel für das Zielvolk dieses Flugs, mit der Forschung von heute.
        let terraform = if mission == Mission::Terraformen {
            let volk = w.ausbau.terraform_fluege.get(&fid).copied().unwrap_or(self.spieler[sid as usize].volk);
            self.terraformbedarf_werkzeug(sid, &json!({"koord": ziel.to_string(), "volk": volk})).ok()
        } else { None };
        let mut plan =
            json!({"status":"ready","kontext":self.planer_regelkontext(),"revision":self.plan_revision(sid)?,
            "abflug":f.abflug,"ankunft":f.ankunft,"rueckkehr":if matches!(mission,Mission::Stationieren|Mission::Kolonisieren){None}else{Some(f.ankunft+f.flugdauer)},
            "habitabilitaet":self.klima_sicht(sid,ziel),"siedler":w.ausbau.siedler.get(&fid),
            "gruendung_unter_vorbehalt":mission==Mission::Kolonisieren,
            "reserviert_milli":delta,"ladung_milli":f.ladung,"hinflug_sekunden":f.flugdauer,
            "annahmen":["Tatsächlicher Startbefehl auf Kopie geprüft; keine Ausführung","Ziel muss bei Ankunft weiterhin verfügbar sein; künftige Fremdaktionen sind unbekannt","Rückflugtreibstoff nach aktiver Missionsregel bereits reserviert"]});
        if let Some(t) = terraform { plan["terraform"] = t; }
        // Kolonieplatz (0.4.7): neue Welten starten auch ohne freien Platz – die Oberfläche warnt mit diesem Stand.
        if matches!(mission, Mission::Kolonisieren | Mission::Kampfkolonisieren) {
            let (anzahl, erlaubt) = (self.kolonien(sid), self.kolonien_erlaubt(sid));
            plan["kolonieplatz"] = json!({"frei": anzahl < erlaubt, "anzahl": anzahl, "erlaubt": erlaubt, "hoechstens": self.kolonien_grenze(),
                "text": if anzahl < erlaubt { Value::Null } else { json!(self.kolonien_voll_text(sid)) }});
        }
        Ok(plan)
    }
    pub fn marktplan(&self, sid: SpielerId, action: &Value) -> Result<Value, String> {
        self.marktplan_als(sid, Rolle::Alle, action)
    }
    pub fn marktplan_als(&self, sid: SpielerId, rolle: Rolle, action: &Value) -> Result<Value, String> {
        let a: Aktion = serde_json::from_value(action.clone()).map_err(|e| e.to_string())?;
        let planet = match a {
            Aktion::MarktOrder { planet, .. } => planet,
            _ => return Err("marktplan benötigt markt_order".into()),
        };
        self.eigener_planet(sid, planet)?;
        let mut w = self.clone();
        let n = w.handel.len();
        let seq = w.seq;
        let credits = self
            .spieler
            .get(sid as usize)
            .ok_or("Spieler fehlt")?
            .credits;
        let (ok, reason) = w.handeln(sid, rolle, action);
        if !ok {
            return Ok(json!({"status":"blocked","grund":reason}));
        }
        let deliveries:Vec<_>=w.ereignisse.iter().filter(|e|e.seq>=seq).filter_map(|e|match e.art{
            EreignisArt::MarktlieferungGebunden{empfaenger,gut,menge,planet} if empfaenger==sid=>Some(json!({"planet":w.planeten[planet as usize].koord.to_string(),"gut":gut,"menge_milli":menge,"ankunft":e.zeit})),
            EreignisArt::Marktlieferung{gut,menge,planet} if w.planeten[planet as usize].besitzer==sid=>Some(json!({"planet":w.planeten[planet as usize].koord.to_string(),"gut":gut,"menge_milli":menge,"ankunft":e.zeit})),_=>None}).collect();
        Ok(
            json!({"status":"ready","kontext":self.planer_regelkontext(),"grund":reason,
            "markt_revision":super::kontext::fingerprint(&(self.plan_revision(sid)?,&self.orders,&self.vertraege.iter().filter(|v|v.a==sid||v.b==sid).collect::<Vec<_>>())),
            "credits_vorher_milli":credits,"credits_nachher_milli":w.spieler[sid as usize].credits,
            "credits_reserviert_oder_verbraucht_milli":credits-w.spieler[sid as usize].credits,
            "teilfuellungen":w.handel[n..],"lieferungen":deliveries,
            "offene_restorders":w.orders.iter().filter(|o|o.spieler==sid&&o.id>=self.naechste_order).collect::<Vec<_>>(),
            "annahmen":["Gebühren und Hinterlegung durch den echten Marktcore","Sichtbares Orderbuch bleibt bis zum Befehl gleich","Offener Rest hat keine garantierte Füllung oder Lieferzeit"]}),
        )
    }
}
