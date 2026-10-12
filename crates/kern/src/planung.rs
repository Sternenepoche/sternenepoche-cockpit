//! Read-only planning with the player's chosen expedition and build order.
use crate::{typen::*, welt::Welt};
use serde_json::{json, Value};

fn goods(values: &[i64; GUETER]) -> Value {
    json!(Gut::ALLE
        .iter()
        .filter(|g| values[g.idx()] != 0)
        .map(|g| (g.name(), values[g.idx()] as f64 / M as f64))
        .collect::<std::collections::BTreeMap<_, _>>())
}
impl Welt {
    /// Constant-population, constant-research construction estimate; no enemy futures are simulated.
    pub fn kolonieplan(&self, sid: SpielerId, query: &Value) -> Result<Value, String> {
        if !self.kolonisationsregeln_v2() {
            return Err("Kolonieplanung benötigt Regelversion 4".into());
        }
        let target: Koord = query["ziel"].as_str().ok_or("ziel fehlt")?.parse()?;
        let survey = self.spieler[sid as usize]
            .erkundet
            .get(&target)
            .ok_or("Ziel zuerst mit eigener Spionagesonde erkunden")?;
        let builds = query["aufbau"]
            .as_array()
            .ok_or("aufbau muss eine geordnete Liste von Gebäuden sein")?;
        if builds.len() > 32 {
            return Err("Höchstens 32 Aufbauschritte".into());
        }
        let mut w = self.clone();
        let fid = w.naechste_flotte;
        let action = json!({"typ":"flotte_senden","mission":"kolonisieren","start":query["start"],
            "ziel":query["ziel"],"schiffe":query["schiffe"],"ladung":query["ladung"],
            "geschwindigkeit":query.get("geschwindigkeit").cloned().unwrap_or(json!(1.0))});
        let (ok, reason) = w.handeln(sid, Rolle::Verwalter, &action);
        if !ok {
            return Ok(
                json!({"start_moeglich":false,"grund":reason,"mindestfracht":goods(&self.koloniefracht(sid)),"keine_ausfuehrung":true}),
            );
        }
        let f = w.flotten[&fid].clone();
        w.zeit = f.ankunft;
        w.flotte_ankunft(fid);
        let pid = *w
            .belegung
            .get(&target)
            .ok_or("Hypothetische Gründung nicht möglich")? as usize;
        if w.planeten[pid].besitzer != sid {
            return Err("Ziel inzwischen belegt".into());
        }
        let arrival = w.zeit;
        let mut stages = vec![];
        let mut shortages = vec![];
        for entry in builds {
            let name = entry.as_str().ok_or("aufbau enthält keinen Gebäudenamen")?;
            let g =
                Gebaeude::aus_name(name).ok_or_else(|| format!("Unbekanntes Gebäude {name}"))?;
            if let Err(reason) = w.bauen(sid, Rolle::Verwalter, target, g) {
                stages.push(json!({"gebaeude":name,"begonnen":false,"grund":reason}));
                break;
            }
            let Some(end) = w.planeten[pid].bauschleife.first().and_then(|a| a.fertig) else {
                stages.push(json!({"gebaeude":name,"begonnen":false,"grund":"wartet auf Bauvoraussetzungen"}));
                break;
            };
            // Record deficits before clamping stocks to zero. We never label a starved plan self-sufficient.
            let stock = w.bestand_jetzt(pid);
            for good in Gut::ALLE {
                let rate = w.planeten[pid].rate[good.idx()];
                if rate < 0
                    && (stock[good.idx()] as i128 * STUNDE as i128)
                        < -(rate as i128) * (end - w.zeit) as i128
                {
                    shortages.push(json!({"gut":good.name(),"vor_schritt":name,
                        "leer_ab":w.zeit + (stock[good.idx()] as i128 * STUNDE as i128 / -rate as i128) as i64}));
                }
            }
            w.zeit = end;
            w.bau_fertig(pid);
            stages.push(json!({"gebaeude":name,"begonnen":true,"fertig":end,
                "stufe":w.planeten[pid].gebaeude[g.idx()],"bestand":goods(&w.bestand_jetzt(pid))}));
        }
        let p = &w.planeten[pid];
        let complete = stages.len() == builds.len() && stages.iter().all(|v| v["begonnen"] == true);
        let foodless = w.regeln.volk(w.spieler[sid as usize].volk).ohne_nahrung;
        let basic_supply = complete
            && (foodless
                || (p.rate[Gut::Nahrung.idx()] >= 0
                    && !shortages.iter().any(|s| s["gut"] == Gut::Nahrung.name())))
            && p.energie_erzeugung >= p.energie_verbrauch;
        Ok(
            json!({"start_moeglich":true,"keine_ausfuehrung":true,"ziel":target.to_string(),
            "sondenbericht_zeit":survey.zeit,"ankunft":arrival,"aufbau":stages,"aufbau_vollstaendig":complete,
            "versorgungsabrisse":shortages,"rate_nach_aufbau":goods(&p.rate),
            "energie":{"erzeugung":p.energie_erzeugung as f64/M as f64,"verbrauch":p.energie_verbrauch as f64/M as f64},
            "grundversorgung_im_modell_gedeckt":basic_supply,
            "versorgung_im_modell_gedeckt":basic_supply && shortages.is_empty() && p.rate.iter().all(|rate| *rate>=0),
            "annahmen":"Nur eigener gewählter Plan, heutige Forschung, konstante Siedlerzahl/Stabilität; keine Feindaktionen, kein Wachstum, keine externen Lieferungen. Laufende Kredit-/Flottenunterhaltskosten nicht hochgerechnet. Keine Erfolgszusage."}),
        )
    }
}
