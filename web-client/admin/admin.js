'use strict';
const $=id=>document.getElementById(id),esc=s=>String(s??'').replace(/[&<>"']/g,c=>({'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;',"'":'&#39;'}[c]));
let key='',state=null,busy=false,timer,prepared=null;
function message(s,error=false){$('message').textContent=s;$('message').hidden=false;$('message').classList.toggle('error',error);clearTimeout(timer);timer=setTimeout(()=>$('message').hidden=true,8000);}
async function api(path,body){const r=await fetch(path,{method:body===undefined?'GET':'POST',headers:{'X-Sternenepoche-Admin':key,...(body===undefined?{}:{'Content-Type':'application/json'})},body:body===undefined?undefined:JSON.stringify(body),signal:AbortSignal.timeout(20000)});const v=await r.json();if(!r.ok)throw Error(v.error);return v;}
const table=(head,rows)=>`<table><thead><tr>${head.map(h=>`<th>${esc(h)}</th>`).join('')}</tr></thead><tbody>${rows.map(r=>`<tr>${r.map(v=>`<td>${v}</td>`).join('')}</tr>`).join('')}</tbody></table>`;
async function refreshBase(){if(busy)return;busy=true;try{state=await api('/api/admin/status');const w=state.lobby;$('status').textContent=state.storage_error|| (w.paused?'Welt pausiert':'Server läuft');$('summary').innerHTML=[['Tag',Math.floor(w.sekunden/86400)+1],['Jetzt frei',w.freie_zugaenge??w.freie_plaetze],['Freigabe',(w.freigegebene_plaetze??20)+'/20'],['Wartende',w.wartende??0],['Flotten',state.flotten],['Ereignisse',state.ereignisse],['Tempo',w.tempo+'×'],['Uptime',Math.floor(state.uptime_s/60)+' Min.'],['Datenbank',(state.db_bytes/1048576).toFixed(1)+' MiB'],['Invarianten',state.invariant_errors.length?'FEHLER':'OK']].map(([k,v])=>`<div class="stat"><strong>${esc(v)}</strong><span>${esc(k)}</span></div>`).join('');
 if(!document.activeElement?.closest('#settings')){$('admission-limit').value=w.freigegebene_plaetze??20;$('tempo').value=w.tempo;$('epoch-days').value=state.epoche_tage;$('bot-period').value=state.bot_period_secs;$('paused').checked=w.paused;$('bots-enabled').checked=state.bots_enabled;$('versorgung-hours').value=w.ausscheiden_regeln?.versorgung_stunden??72;$('stillstand-hours').value=w.ausscheiden_regeln?.stillstand_stunden??48;$('maintenance').value=w.maintenance||'';}
 $('world-hash').textContent='Welt '+w.world_id+' · Zustand '+state.world_hash;
 renderAccountRegistry();
 $('waitlist').innerHTML=(state.warteliste||[]).length?table(['Position','Konto','Spielweise','Volk','Angemeldet',''],state.warteliste.map(w=>[w.position,esc(w.name),esc(w.mode),esc(w.volk),esc(new Date(w.seit*1000).toLocaleString('de-DE')),`<button data-waitlist-remove="${w.id}" class="secondary">Von Warteliste entfernen</button>`])):'Niemand wartet derzeit.';
 $('bots').innerHTML=table(['Nr.','Name','Strategie','Reichstatus','Aktionen','Abgelehnt','Nächster Zug (Spielsek.)','Stufe / Punkte'],state.bots.map(b=>[b.spieler,esc(b.name),esc(b.typ),esc(b.reich_status?.status||'aktiv'),b.aktionen,b.abgelehnt,b.naechster,`${b.stufe} / ${b.punkte}`]));
 $('bot-health').textContent=`${state.bot_actions} ausgeführte Aktionen · ${state.bot_rejected} abgelehnte Aktionen. Bei hohen Ablehnungszahlen Voraussetzungen und Strategie prüfen.`;
 $('agents').innerHTML=table(['Spieler','Freigabe','Status','Aufrufe','Fehler','Tokens','Antwortzeit',''],state.agents.map(a=>[a.spieler,a.active?'Aktiv':'Gestoppt',esc(a.stats.status),a.stats.calls,a.stats.errors,a.stats.tokens,a.stats.latency_ms+' ms',`<button data-stop="${a.spieler}" class="secondary">Stoppen</button>`]));
 $('confirm').placeholder='RESET '+w.world_id;
 }catch(e){$('status').textContent='Verbindung unterbrochen';message(e.message,true);}finally{busy=false;}}
async function action(v){const result=await api('/api/admin/action',{world_id:state.lobby.world_id,...v});message(result.backup?'Backup: '+result.backup:'Änderung gespeichert');await refresh();return result;}
async function refresh(){await refreshBase();if(typeof renderMonitoring==='function')renderMonitoring();}
function guard(fn){return async e=>{if(e?.type==='submit')e.preventDefault();const b=e?.type==='submit'?e.submitter:e?.currentTarget;if(b?.disabled)return;if(b)b.disabled=true;try{await fn(e);}catch(e){message(e.message,true);}finally{if(b)b.disabled=b.id==='reset-submit'&&!prepared;}};}
$('settings').addEventListener('submit',guard(async()=>{await action({action:'settings',admission_limit:Number($('admission-limit').value),tempo:Number($('tempo').value),epoche_tage:Number($('epoch-days').value),bot_period_secs:Number($('bot-period').value),paused:$('paused').checked,bots_enabled:$('bots-enabled').checked,versorgung_stunden:Number($('versorgung-hours').value),stillstand_stunden:Number($('stillstand-hours').value),maintenance:$('maintenance').value});}));
$('bot-edit').addEventListener('submit',guard(async e=>{const f=e.target.elements;await action({action:'bot',spieler:Number(f.spieler.value),typ:f.typ.value});}));
$('player-edit').addEventListener('submit',guard(async e=>{const f=e.target.elements;await action({action:'player_edit',spieler:Number(f.spieler.value),...(f.credits.value===''?{}:{credits:Number(f.credits.value)}),...(f.menge.value===''?{}:{bestand:{[f.gut.value]:Number(f.menge.value)}})});}));
$('reset').addEventListener('submit',guard(async()=>{if(!prepared||prepared.text!==$('rule-profile').value||prepared.world!==state.lobby.world_id)throw Error('Regelprofil und aktuelle Welt zuerst prüfen');await action({action:'reset',rules:prepared.text,confirm:$('confirm').value,...($('seed').value===''?{}:{seed:Number($('seed').value)})});$('confirm').value='';invalidateProfile();}));
$('inspect').addEventListener('submit',guard(async e=>{const v=await read({action:'inspect',spieler:Number(e.target.elements.spieler.value)});$('inspection').innerHTML=cards([{name:v.name,volk:v.volk,stufe:v.stufe,credits:v.credits,reich_status:v.reich_status,forschung:v.forschung,flotten:v.flotten},...v.planeten]);}));
$('backup').onclick=guard(async()=>{await action({action:'backup'});});$('refresh').onclick=refresh;
document.addEventListener('click',guard(async e=>{const b=e.target.closest('[data-account],[data-stop],[data-waitlist-remove]');if(!b)return;if(b.dataset.waitlistRemove)await action({action:'waitlist_remove',id:Number(b.dataset.waitlistRemove)});else if(b.dataset.account)await action({action:'account',id:Number(b.dataset.account),banned:b.dataset.banned==='true'});else await action({action:'stop_agent',spieler:Number(b.dataset.stop)});}));
(async()=>{try{const b=await api('/api/admin/bootstrap');key=b.key;$('game-link').href=b.game_url;await refresh();setInterval(refresh,10000);}catch(e){message(e.message,true);}})();

let selectedAccount=null;
const dateText=n=>n?new Date(n*1000).toLocaleString('de-DE'):'—';
function renderAccountRegistry(){
 const query=$('account-filter').value.trim().toLocaleLowerCase('de');
 const accounts=state.accounts.filter(a=>[a.name,a.email].some(s=>String(s||'').toLocaleLowerCase('de').includes(query)));
 $('account-database').textContent='Private SQLite-Datenbank: '+(state.database?.root||'data/online')+'/'+(state.database?.file||'spiel.sqlite3')+' · SQL-Ansicht registered_players · Spielerstände player_states · Protokolle account_events. Der Weltcheckpoint wird laufend gespeichert; lesbare Stände mindestens alle 60 Sekunden.';
 $('account-registration').textContent=state.registration?.enabled?'E-Mail-Registrierung aktiv · sechsstelliger Einmalcode.':'E-Mail-Registrierung ausgeschaltet · Testkonten funktionieren ohne E-Mail. Authenticator kann jedes Konto selbst verbinden.';
 $('accounts').innerHTML=table(['Konto / Name','E-Mail / Zugang','Authenticator','Spieler / Modus','Letzte Anmeldung','Anmeldungen','Status',''],accounts.map(a=>[`${a.id} · ${esc(a.name)}`,a.is_test?'Testkonto · ohne E-Mail':esc(a.email||'Bestandskonto ohne E-Mail'),a.authenticator_enabled?'Verbunden':'Freiwillig',`${a.spieler??'Zuschauer'} · ${esc(a.mode)}`,esc(dateText(a.last_login_at)),a.login_count??0,a.banned?'Gesperrt':esc(a.reich_status?.status||'Aktiv'),`<button data-player-details="${a.id}" class="secondary">Details & Logs</button> <button data-account="${a.id}" data-banned="${!a.banned}" class="secondary">${a.banned?'Entsperren':'Sperren'}</button>`]));
}
$('account-filter').addEventListener('input',()=>{if(state)renderAccountRegistry();});
document.addEventListener('click',guard(async e=>{
 const button=e.target.closest('[data-player-details]');if(!button)return;
 selectedAccount=await api('/api/admin/action',{world_id:state.lobby.world_id,action:'player_details',id:Number(button.dataset.playerDetails)});
 $('account-detail').hidden=false;$('account-detail-title').textContent=selectedAccount.account.name+' · Konto '+selectedAccount.account.id;
 $('account-detail-meta').textContent='Angelegt: '+dateText(selectedAccount.account.created_at)+' · Letzte Anmeldung: '+dateText(selectedAccount.account.last_login_at)+' · E-Mail bestätigt: '+dateText(selectedAccount.account.email_verified_at)+' · Spielzeit: '+selectedAccount.simulation_seconds+' Sekunden';
 $('account-events').innerHTML=table(['Zeit','Ereignis','Details'],selectedAccount.events.map(e=>[esc(dateText(e.time)),esc(e.kind),esc(JSON.stringify(e.details))]));
 $('account-state').textContent=JSON.stringify(selectedAccount.state,null,2);
}));
$('account-export').onclick=()=>{
 if(!selectedAccount)return;const url=URL.createObjectURL(new Blob([JSON.stringify(selectedAccount,null,2)],{type:'application/json'}));
 const a=document.createElement('a');a.href=url;a.download='Sternenepoche-Konto-'+selectedAccount.account.id+'.json';a.click();setTimeout(()=>URL.revokeObjectURL(url),1000);
};
