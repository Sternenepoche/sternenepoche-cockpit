'use strict';
const fmt=n=>Number(n||0).toLocaleString('de-DE',{maximumFractionDigits:2});
const label=s=>String(s).replaceAll('_',' ').replace(/\b\w/g,c=>c.toUpperCase());
const readable=v=>v==null?'–':Array.isArray(v)?v.map(readable).join(' · '):typeof v==='object'?Object.entries(v).map(([k,n])=>label(k)+': '+readable(n)).join(' · '):String(v);
const cards=rows=>rows?.length?rows.map(v=>`<article class="card record">${Object.entries(v).map(([k,n])=>`<p><strong>${esc(label(k))}</strong> ${esc(readable(n))}</p>`).join('')}</article>`).join(''):'<p class="empty">Keine Einträge.</p>';
const gameTime=t=>`Tag ${Math.floor(t/86400)+1} · ${new Date(t*1000).toISOString().slice(11,19)}`;
async function read(v){return api('/api/admin/action',{world_id:state.lobby.world_id,...v});}
function renderMonitoring(){
 if(!state)return;
 if(prepared&&prepared.world!==state.lobby.world_id)invalidateProfile();
 const openBots=new Set([...$('bot-diagnostics').querySelectorAll('details[open]')].map(d=>d.dataset.bot));const m=state.monitor;$('bot-health').textContent=`${state.bot_actions} ausgeführte Aktionen · ${state.bot_rejected} abgelehnte Aktionen · ${m.bots.filter(b=>b.probleme.length).length} Bots mit Versorgungshinweisen.`;
 $('bot-diagnostics').innerHTML=m.bots.map(b=>`<details data-bot="${b.spieler}" ${openBots.has(String(b.spieler))?'open':''}><summary>Bot ${b.spieler} · ${b.probleme.length?esc(b.probleme.join(', ')):'Versorgung unauffällig'} · ${b.kolonien} Kolonien</summary><p>${esc(b.diagnose)}</p>${table(['Spielzeit','Ergebnis','Befehl','Rückmeldung'],b.verlauf.slice().reverse().map(t=>[esc(gameTime(t.zeit)),t.ok?'Ausgeführt':'Abgelehnt',esc(readable(t.aktion)),esc(t.text)]))}</details>`).join('');
 $('agents').innerHTML=table(['Spieler','Freigabe / Rollen','Status / Entscheidung','Modell','Aufrufe / Fehler','Befehle OK / abgelehnt','Tokens / Zeit','Gemeldete USD',''],state.agents.map(a=>[a.spieler,esc((a.active?'Aktiv':'Gestoppt')+' · '+readable(a.roles)),esc((a.stats.status||'')+' · '+(a.stats.decision||'')),esc((a.stats.provider||'')+' / '+(a.stats.model||'')),`${a.stats.calls} / ${a.stats.errors}`,`${a.stats.accepted||0} / ${a.stats.rejected||0}`,`${fmt(a.stats.tokens)} / ${a.stats.latency_ms} ms`,fmt(a.stats.cost),`<button data-stop="${a.spieler}" class="secondary">Stoppen</button>`]));
 $('backup-state').textContent='Letztes Backup: '+(m.letztes_backup||'Noch keines')+' · Protokolleinträge: '+m.audit_eintraege;
}
function invalidateProfile(){prepared=null;$('reset-submit').disabled=true;$('reset-preview').textContent='Profil vor dem Reset erneut prüfen.';}
$('admin-map').addEventListener('submit',guard(async e=>{const f=e.target.elements,v=await read({action:'map',sektor:Number(f.sektor.value),system:Number(f.system.value)});$('map-result').innerHTML='<h3>Planeten</h3>'+cards(v.planeten)+'<h3>Flotten</h3>'+cards(v.flotten);}));
$('events-load').onclick=guard(async()=>{const v=await read({action:'events'});$('event-result').innerHTML=`<h3>Ereignisse · ${v.gesamt} insgesamt, maximal 100 angezeigt</h3>`+cards(v.ereignisse);});
$('audit-load').onclick=guard(async()=>{$('audit-result').innerHTML='<h3>Letzte 60 Verwaltungseinträge</h3>'+cards((await read({action:'audit'})).eintraege);});
$('profile-load').onclick=guard(async()=>{$('rule-profile').value=(await read({action:'rules_profile'})).text;invalidateProfile();});
$('rule-profile').oninput=invalidateProfile;
$('profile-validate').onclick=guard(async()=>{invalidateProfile();const text=$('rule-profile').value;if(!text.trim())throw Error('Zuerst Profil laden oder eingeben');const v=await read({action:'reset_preview',rules:text});prepared={text,world:state.lobby.world_id,hash:v.profile_hash};$('reset-preview').innerHTML=cards([v]);$('reset-submit').disabled=false;});
