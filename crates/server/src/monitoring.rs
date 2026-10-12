use super::*;

impl Game {
    pub(super) fn record_bot_turn(&mut self,sid:SpielerId)->Result<(),(u16,String)> {
        let logs=self.world.log_abholen();self.record_player_events(logs.clone())?;
        let trace=self.runtime.bot_trace.entry(sid).or_default();
        for e in logs.into_iter().filter(|e|e.spieler==sid) {
            trace.push(json!({"zeit":e.zeit,"ok":e.ok,"aktion":serde_json::from_str::<Value>(&e.aktion).unwrap_or(Value::Null),"text":e.text}));
        }
        if trace.len()>20 {trace.drain(..trace.len()-20);}
        Ok(())
    }
    pub(super) fn record_public_history(&mut self) {
        let active=self.world.spieler.iter().enumerate().filter(|(id,_)|self.world.spieler_aktiv(*id as u16)).map(|(_,s)|s).collect::<Vec<_>>();
        self.runtime.public_history.push(json!({"sekunden":self.world.zeit,"reiche":active.len(),"kolonien":active.iter().map(|s|s.planeten.len()).sum::<usize>(),
            "punkte":active.iter().map(|s|s.punkte.gesamt()).sum::<i64>(),"stufen":(1..=5).map(|stage|active.iter().filter(|s|s.stufe==stage).count()).collect::<Vec<_>>()}));
        if self.runtime.public_history.len()>168 {self.runtime.public_history.remove(0);}
    }
    pub(super) fn monitoring_status(&self)->Value {
        let bots=self.runtime.bots.iter().map(|(sid,b)| {
            let s=&self.world.spieler[*sid as usize];let pid=s.heimat as usize;
            let trace=self.runtime.bot_trace.get(sid).cloned().unwrap_or_default();
            let failures=trace.iter().rev().take_while(|v|v["ok"]==false).count();let p=&self.world.planeten[pid];
            let problems=[(p.energie_erzeugung<p.energie_verbrauch,"Energie fehlt"),(p.rate[Gut::Nahrung.idx()]<0,"Nahrungsbestand sinkt"),
                (s.credits<0,"Credits negativ"),(failures>=3,"Mehrere Befehle hintereinander abgelehnt")].into_iter().filter(|(yes,_)|*yes).map(|(_,text)|text).collect::<Vec<_>>();
            json!({"spieler":sid,"reich_status":self.world.reich_status(*sid),"probleme":if self.world.ist_besiegt(*sid){vec!["Besiegt; für diese Epoche gestoppt"]}else{problems},"diagnose":if self.world.ist_besiegt(*sid){"Keine weiteren Botaktionen nach dem Ausscheiden".into()}else{lauf::bots::bau_diagnose(&self.world,*sid,pid,b.typ)},"verlauf":trace,
                "letzter_erfolg":trace.iter().rev().find(|v|v["ok"]==true),"kolonien":s.planeten.len(),"forschung":s.forschung.iter().map(|n|*n as u32).sum::<u32>()})
        }).collect::<Vec<_>>();
        let latest=std::fs::read_dir(self.data_root.join("backups")).ok().into_iter().flatten().filter_map(Result::ok).filter_map(|e|e.metadata().ok().and_then(|m|m.modified().ok().map(|t|(t,e.file_name().to_string_lossy().into_owned())))).max_by_key(|(t,_)|*t).map(|(_,name)|name);
        json!({"bots":bots,"letztes_backup":latest,"audit_eintraege":self.db.query_row("SELECT count(*) FROM audit",[],|r|r.get::<_,i64>(0)).unwrap_or(0)})
    }
    pub(super) fn validated_profile(&self,v:&Value)->Result<String,(u16,String)> {
        let text=v["rules"].as_str().unwrap_or(include_str!("../../../regeln/online-v1.ron"));
        if text.len()>48_000 {return Err(err(400,"Regelprofil auf 48 KiB begrenzt"));}
        // Preflight before cache construction/world generation: malformed operator
        // values must return an error instead of poisoning the live world's mutex.
        let raw:Regelwerk=ron::from_str(text).map_err(|e|err(400,&format!("Regelprofil: {e}")))?;
        let w=&raw.welt;let e=&raw.wirtschaft;let f=&raw.flug;
        if !(1..=8).contains(&w.sektoren)||w.systeme_je_sektor==0||!(1..=32).contains(&w.plaetze_je_system)
            ||!(1..=w.plaetze_je_system).contains(&w.heimat_position)||w.nebel_abstand==0||w.guertel_abstand==0||w.start_abstand==0
            ||(w.sektoren as u32*w.systeme_je_sektor as u32/(w.start_abstand as u32))<50
            ||w.max_gebaeudestufe==0||w.start_gebaeude.values().any(|n|*n>w.max_gebaeudestufe)
            ||e.bauzeit_teiler<=0||e.min_bauzeit_sekunden<=0||e.warteschlange==0||e.forschung_warteschlange==0
            ||e.lager_roh<=0||e.lager_verarbeitet<=0||e.lager_selten<=0||f.treibstoff_teiler<=0||f.zeitfaktor<=0
            ||w.reichtum_min<=0.0||w.reichtum_min>w.reichtum_max||w.start_steuersatz>50
            ||raw.zonen.values().any(|z|z.von==0||z.von>z.bis||z.felder_min==0||z.felder_min>z.felder_max)
            ||!(1..=w.plaetze_je_system).all(|position|raw.zonen.values().filter(|z|position>=z.von&&position<=z.bis).count()==1)
            ||raw.voelker.values().any(|v|v.pluenderquote.is_some_and(|q|!q.is_finite()||!(0.0..=1.0).contains(&q))) {
            return Err(err(400,"Ungültige Welt-, Start-, Lager-, Zeit- oder Flugparameter (50 sichere Startsysteme erforderlich)"));
        }
        fn numbers(v:&Value,key:&str)->bool {match v {
            Value::Null=>matches!(key,"antrieb"|"pluenderquote"),
            Value::Number(n)=>n.as_f64().is_some_and(|n|n.is_finite()&&n<=1_000_000_000.0&&(n>=0.0||(key=="hunger_min"&&n>=-1.0))),
            Value::Array(a)=>a.iter().all(|x|numbers(x,key)),Value::Object(o)=>o.iter().all(|(k,x)|numbers(x,k)),_=>true
        }}
        if !numbers(&serde_json::to_value(&raw).map_err(|_|err(400,"Profil enthält ungültige Zahlen"))?,"") {
            return Err(err(400,"Regelzahlen müssen endlich und nicht negativ sein (Hungerfaktor: -1 bis 0)"));
        }
        let r=Regelwerk::laden(text).map_err(|e|err(400,&e))?;
        if !r.gebaeude.contains_key(&Gebaeude::Geheimdienst) || !r.forschung.contains_key(&Forschung::Ueberwachungstechnik) || !r.forschung.contains_key(&Forschung::Abschirmtechnik) {
            return Err(err(400,"Onlineprofil benötigt Geheimdienst, Überwachung und Abschirmtechnik"));
        }
        if !(1..=10000).contains(&r.welt.epoche_tage) {return Err(err(400,"Epoche: 1 bis 10000 Spieltage"));}
        let mut checked=r.clone();checked.welt.fenster_sekunden=1;
        let candidate=Welt::neu(checked,83,(BOT_COUNT+SEATS) as usize).map_err(|e|err(400,&e))?;
        if !candidate.aufklaerungsregeln(){return Err(err(400,"Profil aktiviert keine private Onlineaufklärung"));}
        Ok(text.to_owned())
    }
    pub(super) fn admin_read(&self,v:&Value)->Reply {
        match v["action"].as_str().unwrap_or("") {
            "rules_profile"=>{
                let mut r=(*self.world.regeln).clone();r.hash.clear();r.cache=Default::default();
                let text=ron::ser::to_string_pretty(&r,ron::ser::PrettyConfig::default()).map_err(|_|err(503,"Regelprofil nicht lesbar"))?;
                Ok(json!({"text":text,"version":self.world.regeln.version,"hash":self.world.regeln.hash}))
            },
            "rules_validate"|"reset_preview"=>{
                let text=self.validated_profile(v)?;let r=Regelwerk::laden(&text).map_err(|e|err(400,&e))?;
                Ok(json!({"valid":true,"profile_hash":r.hash,"version":r.version,"epoche_tage":r.welt.epoche_tage,"alte_welt":self.runtime.world_id,
                    "betroffene_konten":self.db.query_row("SELECT count(*) FROM accounts",[],|row|row.get::<_,i64>(0)).unwrap_or(0),
                    "wirkung":"Neue Welt, 20 freie Plätze. Alle Sitzungen und Agentfreigaben widerrufen; Konten bleiben als Zuschauer. Automatisches Backup vor Übernahme."}))
            },
            "events"=>{
                let mut events=self.world.ereignisse.iter().collect::<Vec<_>>();events.sort_by_key(|e|(e.zeit,e.prio,e.seq));
                Ok(json!({"gesamt":events.len(),"ereignisse":events.into_iter().take(100).map(|e|json!({"zeit":e.zeit,"art":format!("{:?}",e.art)})).collect::<Vec<_>>()}))
            },
            "audit"=>{
                let records=self.db.prepare("SELECT time,account,kind,result FROM audit ORDER BY id DESC LIMIT 60").and_then(|mut stmt|stmt.query_map([],|row|Ok(json!({"zeit":row.get::<_,i64>(0)?,"konto":row.get::<_,Option<i64>>(1)?,"art":row.get::<_,String>(2)?,"details":serde_json::from_str::<Value>(&row.get::<_,String>(3)?).unwrap_or(Value::Null)})))?.collect::<Result<Vec<_>,_>>()).map_err(|_|err(503,"Audit nicht lesbar"))?;
                Ok(json!({"eintraege":records}))
            },
            "map"=>{
                let r=&self.world.regeln;let sector=v["sektor"].as_u64().filter(|n|*n>=1&&*n<=r.welt.sektoren as u64).ok_or_else(||err(400,"Sektor ungültig"))? as u8;
                let system=v["system"].as_u64().filter(|n|*n>=1&&*n<=r.welt.systeme_je_sektor as u64).ok_or_else(||err(400,"System ungültig"))? as u8;
                let mut planets=Vec::new();
                for position in 1..=r.welt.plaetze_je_system {
                    let coord=Koord::neu(sector,system,position);
                    let data=if let Some(&pid)=self.world.belegung.get(&coord){let p=&self.world.planeten[pid as usize];json!({"koord":coord.to_string(),"spieler":p.besitzer,"name":self.world.spieler[p.besitzer as usize].name,"aktiv":self.world.spieler_aktiv(p.besitzer),"zone":p.zone,"bevoelkerung":p.bevoelkerung,"bestand":Gut::ALLE.iter().map(|g|(g.name(),ganz(self.world.bestand_jetzt(pid as usize)[g.idx()]))).collect::<BTreeMap<_,_>>()})}else{json!({"koord":coord.to_string(),"status":"frei"})};
                    planets.push(data);
                }
                let fleets=self.world.flotten.values().filter(|f|f.ziel.sektor==sector&&f.ziel.system==system).map(|f|json!({"flotte":f.id,"spieler":f.besitzer,"ziel":f.ziel.to_string(),"mission":f.mission,"ankunft":f.ankunft,"rueckkehr":f.rueckkehr})).collect::<Vec<_>>();
                Ok(json!({"planeten":planets,"flotten":fleets}))
            },
            _=>Err(err(400,"Unbekannte Verwaltungsabfrage"))
        }
    }
}
