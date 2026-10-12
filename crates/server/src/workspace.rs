//! Account-private team memory. Never accepts provider credentials. Decisions and
//! their journal entries are committed in the same transaction as game actions.
use super::*;
use std::collections::BTreeSet;

pub(super) fn setup(db: &Connection) -> Result<(), String> {
    db.execute_batch("CREATE TABLE IF NOT EXISTS team_workspace(account INTEGER PRIMARY KEY REFERENCES accounts(id),revision INTEGER NOT NULL,state TEXT NOT NULL);
        CREATE TABLE IF NOT EXISTS team_journal(id INTEGER PRIMARY KEY,account INTEGER NOT NULL REFERENCES accounts(id),world_id TEXT NOT NULL,entry TEXT NOT NULL);
        CREATE INDEX IF NOT EXISTS team_journal_owner ON team_journal(account,id);").map_err(|e|e.to_string())
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Model { id: String, name: String, provider: String, url: String, model: String }
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Role { model: String, soul: String, instructions: String }
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Note { id: String, title: String, text: String, to: String, status: String }
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Board { id: String, title: String, world_id: String, notes: Vec<Note> }
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct State { models: Vec<Model>, roles: BTreeMap<String,Role>, boards: Vec<Board>, delay: u32, limit: u32, cost_limit: f64 }

fn valid_id(s: &str) -> bool { !s.is_empty() && s.len() <= 80 && s.bytes().all(|b|b.is_ascii_alphanumeric() || b == b'-' || b == b'_') }
fn text_ok(s: &str, max: usize) -> bool { s.chars().count() <= max && !s.contains('\0') }
fn clean_state(v: &Value) -> Reply {
    if v.to_string().len()>56000 { return Err(err(400,"Team und Pinwände dürfen zusammen höchstens 56 KB belegen")); }
    let s: State = serde_json::from_value(v.clone()).map_err(|_|err(400,"Ungültige Teamdaten; Schlüssel gehören ausschließlich in den Browser"))?;
    let mut ids=BTreeSet::new();
    if s.models.len()>4 || !(15..=3600).contains(&s.delay) || !(1..=10000).contains(&s.limit) || !s.cost_limit.is_finite() || s.cost_limit<0.0 {return Err(err(400,"Maximal vier Modelle; ungültiges Aufruf- oder Zeitlimit"));}
    for m in &s.models {
        if !valid_id(&m.id) || !ids.insert(m.id.clone()) || m.name.trim().is_empty() || !text_ok(&m.name,40) || !text_ok(&m.model,160) || !text_ok(&m.url,160) || !["ollama","openrouter"].contains(&m.provider.as_str()) {return Err(err(400,"Modellname, Kennung oder Anbieter ungültig"));}
    }
    if s.roles.len()!=4 {return Err(err(400,"Vier Regierungsrollen erforderlich"));}
    for name in ["stratege","verwalter","feldherr","diplomat"] {
        let r=s.roles.get(name).ok_or_else(||err(400,"Regierungsrolle fehlt"))?;
        if (!r.model.is_empty() && !ids.contains(&r.model)) || !text_ok(&r.soul,2000) || !text_ok(&r.instructions,3000) {return Err(err(400,"Rollentext zu lang oder Modellzuordnung unbekannt"));}
    }
    if s.boards.len()>20 {return Err(err(400,"Höchstens 20 Pinwände erlaubt"));}
    ids.clear();
    for b in &s.boards {
        if !valid_id(&b.id) || !ids.insert(b.id.clone()) || b.title.trim().is_empty() || !text_ok(&b.title,80) || !text_ok(&b.world_id,80) || b.notes.len()>40 {return Err(err(400,"Ungültige Pinwand; höchstens 40 Karten je Wand"));}
        let mut notes=BTreeSet::new();
        for n in &b.notes {
            if !valid_id(&n.id) || !notes.insert(n.id.clone()) || !text_ok(&n.title,80) || !text_ok(&n.text,3000) || !text_ok(&n.to,40) || !["offen","aktiv","erledigt"].contains(&n.status.as_str()) {return Err(err(400,"Ungültige Karte oder Text zu lang"));}
        }
    }
    serde_json::to_value(s).map_err(|_|err(503,"Teamdaten nicht lesbar"))
}

impl Game {
    pub fn workspace(&self,a:&Account)->Reply {
        let row:Option<(u64,String)>=self.db.query_row("SELECT revision,state FROM team_workspace WHERE account=?1",[a.id],|r|Ok((r.get(0)?,r.get(1)?))).optional().map_err(|_|err(503,"Pinwände nicht lesbar"))?;
        let (revision,state)=match row {Some((n,s))=>(n,serde_json::from_str::<Value>(&s).map_err(|_|err(503,"Pinwände beschädigt"))?),None=>(0,Value::Null)};
        let mut stmt=self.db.prepare("SELECT entry FROM team_journal WHERE account=?1 AND world_id=?2 ORDER BY id DESC LIMIT 24").map_err(|_|err(503,"Teamprotokoll nicht lesbar"))?;
        let rows=stmt.query_map(params![a.id,self.runtime.world_id],|r|r.get::<_,String>(0)).map_err(|_|err(503,"Teamprotokoll nicht lesbar"))?;
        let journal=rows.map(|r|r.map_err(|_|err(503,"Teamprotokoll nicht lesbar")).and_then(|s|serde_json::from_str::<Value>(&s).map_err(|_|err(503,"Teamprotokoll beschädigt")))).collect::<Result<Vec<_>,_>>()?;
        Ok(json!({"revision":revision,"state":state,"journal":journal,"world_id":self.runtime.world_id}))
    }
    fn workspace_revision(&self,a:&Account,expected:&Value)->Result<(),(u16,String)> {
        let n:Option<u64>=self.db.query_row("SELECT revision FROM team_workspace WHERE account=?1",[a.id],|r|r.get(0)).optional().map_err(|_|err(503,"Pinwände nicht lesbar"))?;
        if expected.as_u64()!=Some(n.unwrap_or(0)) {return Err(err(409,"Team oder Pinwand wurde inzwischen geändert. Neu laden und Entscheidung erneut planen."));} Ok(())
    }
    pub fn save_workspace(&mut self,a:&Account,v:&Value)->Reply {
        self.world_matches(v)?;
        let state=clean_state(&v["state"])?;
        self.transaction(|g|{
            g.workspace_revision(a,&v["revision"])?;
            g.db.execute("INSERT INTO team_workspace VALUES(?1,1,?2) ON CONFLICT(account) DO UPDATE SET revision=revision+1,state=excluded.state",params![a.id,state.to_string()]).map_err(|_|err(503,"Team nicht gespeichert"))?;
            g.workspace(a)
        })
    }
    // Called inside command's existing transaction, before any game action.
    pub(super) fn check_team_turn(&self,a:&Account,v:&Value)->Result<(),(u16,String)> {
        if let Some(team)=v.get("team") {
            self.workspace_revision(a,&team["revision"])?;
            let state=self.workspace(a)?;
            let role=v["rolle"].as_str().unwrap_or("");
            let assigned=&state["state"]["roles"][role]["model"];
            if assigned.as_str().is_none_or(|s|s.is_empty()) || assigned!=&team["model_id"] || v.get("lease").is_none() {return Err(err(409,"Teamrolle ist nicht mehr diesem Modell zugewiesen"));}
            if !text_ok(team["summary"].as_str().unwrap_or(""),1000) || team["pins"].as_array().is_none_or(|p|p.len()>4) {return Err(err(400,"Ungültige Teamnotizen"));}
            // Validate all edits before actions: a malformed memory must never lose a played turn.
            let mut s=state["state"].clone();
            apply_pins(&mut s,&team["pins"])?;
            clean_state(&s)?;
        } Ok(())
    }
    pub(super) fn finish_team_turn(&self,a:&Account,v:&Value,results:&[Value])->Result<(),(u16,String)> {
        let Some(team)=v.get("team") else{return Ok(())};
        let mut state=self.workspace(a)?["state"].clone();
        let mut pins=team["pins"].clone();
        // A failed game action cannot certify a newly completed goal. Preserve
        // the plan as open and keep the actual rejection in the journal.
        if results.iter().any(|r|r["ok"]==false) {
            for pin in pins.as_array_mut().unwrap() {
                if pin["note"]["status"]=="erledigt" {pin["note"]["status"]=json!("offen");}
            }
        }
        apply_pins(&mut state,&pins)?;
        self.db.execute("UPDATE team_workspace SET revision=revision+1,state=?2 WHERE account=?1",params![a.id,state.to_string()]).map_err(|_|err(503,"Teamnotizen nicht gespeichert"))?;
        let entry=json!({"at":now(),"request_id":v["request_id"],"role":v["rolle"],"model_id":team["model_id"],"summary":team["summary"],"results":results});
        self.db.execute("INSERT INTO team_journal(account,world_id,entry) VALUES(?1,?2,?3)",params![a.id,self.runtime.world_id,entry.to_string()]).map_err(|_|err(503,"Teamprotokoll nicht gespeichert"))?;
        self.db.execute("DELETE FROM team_journal WHERE account=?1 AND id NOT IN (SELECT id FROM team_journal WHERE account=?1 ORDER BY id DESC LIMIT 200)",[a.id]).map_err(|_|err(503,"Teamprotokoll nicht gespeichert"))?;
        Ok(())
    }
}

fn apply_pins(state:&mut Value,pins:&Value)->Result<(),(u16,String)> {
    for p in pins.as_array().ok_or_else(||err(400,"Pinwandänderungen fehlen"))? {
        let board=state["boards"].as_array_mut().and_then(|bs|bs.iter_mut().find(|b|b["id"]==p["board_id"])).ok_or_else(||err(400,"Pinwand nicht gefunden"))?;
        // Agents may add/update cards, never delete an entire board or someone else's text implicitly.
        let note:Note=serde_json::from_value(p["note"].clone()).map_err(|_|err(400,"Ungültige Teamkarte"))?;
        let notes=board["notes"].as_array_mut().ok_or_else(||err(400,"Pinwand beschädigt"))?;
        if let Some(old)=notes.iter_mut().find(|n|n["id"].as_str()==Some(&note.id)) {*old=p["note"].clone();}else{notes.push(p["note"].clone());}
    } Ok(())
}
