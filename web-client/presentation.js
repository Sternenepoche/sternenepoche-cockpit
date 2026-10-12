'use strict';
const roleNames=['stratege','verwalter','feldherr','diplomat'];
function modelSchema(schema,view){
 const s=JSON.parse(JSON.stringify(schema));
 const strip=o=>{if(o&&typeof o==='object'){delete o.description;delete o.title;if(o.type==='string'&&!o.enum)o.maxLength=Math.min(o.maxLength||512,512);Object.values(o).forEach(strip);}};strip(s);
 if(s.properties.begruendung)s.properties.begruendung.maxLength=400;
 if(s.properties.notiz)s.properties.notiz.maxLength=600;
 s.properties.aktionen.maxItems=3;s.properties.abfragen.maxItems=2;
 const owned=(view?.planeten||[]).map(p=>p.koord);
 if(owned.length){
  const actions=[];
  for(const action of s.properties.aktionen.items.anyOf){
   const type=action.properties.typ.enum[0];
   if(type==='bauen'){
    for(const p of view.planeten){const legal=(p.baubar||[]).filter(b=>!(b.fehlt||[]).length&&requirementsMet(p,b.braucht)).map(b=>b.gebaeude);if(!legal.length||p.bauschleife.length>=p.bauschleife_plaetze)continue;const a=JSON.parse(JSON.stringify(action));a.properties.planet.enum=[p.koord];a.properties.gebaeude.enum=legal;actions.push(a);}continue;
   }
   if(type==='fertigen'&&action.properties.einheit){
    const legal=(view.einheiten_kosten||[]).filter(e=>view.planeten.some(p=>(p.gebaeude.werft||0)>=e.werft&&requirementsMet(p,e.braucht)&&Object.entries(e.kosten).every(([g,n])=>(p.bestand[g]||0)>=n))).map(e=>e.einheit);if(!legal.length)continue;action.properties.einheit.enum=legal;
   }
   if(type==='fertigen'&&action.properties.bauteil&&!view.planeten.some(p=>(p.gebaeude.orbitalwerft||0)>0))continue;
   if(type==='forschen'){const legal=(view.forschung?.moeglich||[]).filter(f=>f.labor_heimat>=f.labor&&!(f.fehlt||[]).length&&(!(f.braucht||[]).length||(view.forschung.stufen.spionagetechnik||0)>0)&&(!f.infrastruktur||view.planeten.some(p=>(p.gebaeude.geheimdienst||0)>0))).map(f=>f.forschung);if(!legal.length)continue;action.properties.forschung.enum=legal;}
   if(type==='flotte_senden'&&!view.planeten.some(p=>Object.values(p.schiffe).some(n=>n>0)))continue;
   for(const name of ['planet','start']){const field=action.properties[name];if(field)field.enum=[...owned,...(Array.isArray(field.type)&&field.type.includes('null')?[null]:[])];}
   actions.push(action);
  }s.properties.aktionen.items.anyOf=actions;
 }
 return s;
}
function requirementsMet(p,requirements){return (requirements||[]).every(text=>{const m=/^([a-z_]+) (\d+)$/.exec(text);return m&&(p.gebaeude[m[1]]||0)>=Number(m[2]);});}
function modelRules(text){return text.split('\n').filter(line=>!line.includes(' | ')).join('\n')+'\nDie aktuellen Kosten, Voraussetzungen und Bauangebote stehen im privaten Lagebild. Für weitere Details nutze lesende Werkzeuge. Schreibe kurze, sachliche Begründungen. Warten ist möglich; sichere zuerst Versorgung und dann wirtschaftliches Wachstum.';}
function empty(text){return `<p class="empty">${esc(text)}</p>`;}
function badge(text,kind=''){return `<span class="badge ${kind}">${esc(text)}</span>`;}
function readable(value){if(value==null)return 'Unbekannt';if(Array.isArray(value))return value.map(readable).join(' · ');if(typeof value==='object')return Object.entries(value).map(([k,v])=>`${label(k)}: ${readable(v)}`).join(' · ');return typeof value==='boolean'?(value?'Ja':'Nein'):String(value);}
function recordCards(items,title){return items?.length?items.map(item=>`<article class="card"><div class="row"><h3>${esc(item.ziel||item.name||title)}</h3>${badge(item.zeit??'')}</div>${Object.entries(item).filter(([k])=>!['zeit','name','ziel'].includes(k)).map(([k,v])=>`<p><strong>${esc(label(k))}</strong> ${esc(readable(v))}</p>`).join('')}</article>`).join(''):empty('Noch keine Einträge.');}
function renderPublic(){
 $('maintenance').textContent=world.maintenance||'';$('maintenance').hidden=!world.maintenance;
 const h=world.verlauf||[];$('public-history').innerHTML=h.length?table(['Spielzeit','Aktive Reiche','Besiedelte Welten','Gesamtpunkte','Stufen I–V'],h.map(v=>[esc(`Tag ${Math.floor(v.sekunden/86400)+1}, ${Math.floor(v.sekunden%86400/3600)} Uhr`),v.reiche,v.kolonien,fmt(v.punkte),esc(v.stufen.join(' / '))])):empty('Die erste Entwicklungsübersicht entsteht nach einer Spielstunde. Die Rangliste ist bereits live.');
}
function renderGovernment(){
 const n=view.naechste_stufe;
 $('government').innerHTML=`<div class="row"><h3>Regierung</h3>${badge('Steuern '+view.steuersatz+' %')}</div><form id="tax-form"><label>Steuersatz (%)<input name="prozent" type="number" min="0" max="50" value="${view.steuersatz}" required></label><button>Steuersatz ändern</button></form>`+(n?`<h3>Aufstieg: ${esc(n.name)}</h3><ul class="conditions">${n.bedingungen.map(b=>`<li>${badge(b.erfuellt?'Erfüllt':'Offen',b.erfuellt?'good':'wait')} ${esc(b.text)}</li>`).join('')}</ul><p>Haltezeit: ${n.erfuellt_seit_stunden} / ${n.haltezeit_stunden} Spielstunden · Kosten: ${esc(goods(n.kosten))}</p><button data-promote>Stufenaufstieg beantragen</button>`:empty('Die höchste Zivilisationsstufe ist erreicht.'));
 $('tax-form').onsubmit=guard(async e=>{await command([{typ:'steuersatz',prozent:Number(e.target.elements.prozent.value)}]);});
 const roles=view.agent?.roles||[];$('control-state').textContent=view.reich_status?.status==='besiegt'?'Dein Reich ist ausgeschieden. Du kannst seine Ansicht und Berichte weiter lesen.':roles.length?'Agent übernimmt: '+roles.map(label).join(', ')+'. Die übrigen Rollen steuerst du selbst.':'Du steuerst dein Reich. Kein Agent besitzt eine aktive Freigabe.';
}
function renderProducts(){
 const select=$('products'),previous=select.value,p=view.planeten.find(p=>p.koord===$('manufacture').elements.planet.value);
 const entries=view.einheiten_kosten.map(e=>{const needs=[...(e.werft?['werft '+e.werft]:[]),...(e.braucht||[])],ready=p&&requirementsMet(p,needs);return `<option value="${esc(e.einheit)}" ${ready?'':'disabled'}>${esc(label(e.einheit))} · ${esc(goods(e.kosten))}${ready?'':' · benötigt '+esc(needs.join(', '))}</option>`;});
 for(const name of ['antriebskern','habitatmodul'])entries.push(`<option value="${name}" ${(p?.gebaeude.orbitalwerft||0)>0?'':'disabled'}>${label(name)} (Bauteil · Orbitalwerft 1)</option>`);
 select.innerHTML=entries.join('');if([...select.options].some(o=>o.value===previous&&!o.disabled))select.value=previous;
}
function renderReports(){
 const groups=[['flottenberichte','Flottenaufklärung'],['berichte','Planetenspionage'],['kampfberichte','Kampfberichte'],['ereignisse','Aktuelle Ereignisse'],['chronik','Chronik'],['meldungen','Regierungsnotizen'],['erkundet','Erkundete Planeten']];
 $('reports').innerHTML=groups.map(([k,t])=>`<details ${['ereignisse','flottenberichte'].includes(k)?'open':''}><summary>${t} (${view[k]?.length||0})</summary>${recordCards(view[k],t)}</details>`).join('');
}
function renderDiplomacy(){
 const m=view.markt||{};
 $('market-state').innerHTML=table(['Gut','Angebot ab','Nachfrage bis'],Object.entries(m.preise||{}).map(([k,v])=>[esc(label(k)),v.verkauf_ab??'Kein Angebot',v.kauf_bis??'Keine Nachfrage']))+`<h3>Eigene Orders</h3>`+(m.orders?.length?table(['Nr.','Planet','Gut','Seite','Menge','Preis',''],m.orders.map(o=>[o.order,esc(o.planet),esc(label(o.gut)),esc(label(o.seite)),fmt(o.menge),o.preis,`<button data-cancel-order="${esc(o.order)}" class="secondary">Stornieren</button>`])):empty('Keine offenen Marktorders.'));
 $('contracts').innerHTML=(view.vertraege?.length?view.vertraege.map(v=>`<article class="card"><h3>${esc(label(v.art))}</h3><p>${esc(readable(v))}</p><div class="row">${v.status==='angebot an dich'?`<button data-contract="${esc(v.vertrag)}" data-contract-action="vertrag_annehmen">Annehmen</button><button class="secondary" data-contract="${esc(v.vertrag)}" data-contract-action="vertrag_ablehnen">Ablehnen</button>`:v.status==='aktiv'?`<button class="secondary" data-contract="${esc(v.vertrag)}" data-contract-action="vertrag_kuendigen">Kündigen</button>`:''}</div></article>`).join(''):empty('Keine Verträge.'));
 if(typeof renderCommunication==='function')renderCommunication();
}
function navigateGame(id){if(window.GalaxyUI)GalaxyUI.visible(id==='karte');document.querySelectorAll('#game > section').forEach(s=>s.hidden=s.id!==id);document.querySelectorAll('[data-tab]').forEach(t=>t.setAttribute('aria-selected',String(t.dataset.tab===id)));document.dispatchEvent(new Event('game-navigation'));}
function pickFleet(target,mission){navigateGame('flotten');if($('fleet-planner'))$('fleet-planner').open=true;const form=$('fleet');form.elements.ziel.value=target;form.elements.mission.value=mission;for(const id of ['ship-inputs','cargo-inputs'])$(id).querySelectorAll('input').forEach(i=>i.value='0');if(['spionage','system_erkunden'].includes(mission))$('ship-inputs').querySelector('[data-map="spionagesonde"]').value='1';$('fleet').scrollIntoView?.({block:'start'});}
function fuelReserve(q,legs){return Math.ceil((q.treibstoff_je_strecke_milli??q.treibstoff_je_strecke*1000)*legs/1000);}
function flightSummary(q,a){
 const hold=['saven','halten','abbau','blockade'].includes(a.mission)?a.haltedauer_stunden:0,oneway=['stationieren','kolonisieren'].includes(a.mission),legs=oneway?1:2,minutes=(q.dauer_sekunden??q.dauer_min*60)/60,gameMinutes=minutes*legs+hold*60;
 const cargo=Object.values(a.ladung).reduce((sum,n)=>sum+n,0),p=view.planeten.find(p=>p.koord===a.start),fuel=fuelReserve(q,legs);
 $('flight-quote').innerHTML=`<h3>Flugplanung</h3><div class="stats">${stat(fmt(minutes)+' Min.','Hinflug (Spielzeit)')}${stat(fmt(gameMinutes)+' Min.',oneway?'Ankunft nach Start':'Planmäßige Rückkehr nach Start')}${stat(fuel,oneway?'Treibstoffreserve für Hinflug':'Treibstoffreserve für beide Strecken')}${stat(cargo+' / '+q.ladekapazitaet,'Fracht / Laderaum')}</div><p>Bei ${world.tempo}×: ca. ${fmt(gameMinutes/world.tempo)} echte Minuten. ${hold?'Wartezeit: '+hold+' Spielstunden.':''} ${oneway?'Einwegauftrag. Erfolg und tatsächlicher Verbleib werden bei Ankunft geprüft.':'Kampf, Blockaden und Rückruf können den Verlauf ändern.'}</p><p>Auf dem Planeten bleiben: ${esc(goods(Object.fromEntries(Object.entries(p.bestand).map(([k,n])=>[k,Math.max(0,n-(a.ladung[k]||0)-(k==='deuterium'?fuel:0))]))))}</p>`;
}
function selectAllShips(){const p=view.planeten.find(p=>p.koord===$('fleet').elements.start.value);$('ship-inputs').querySelectorAll('input').forEach(i=>i.value=p.schiffe[i.dataset.map]||0);}
async function fillSaveCargo(){
 const a=fleetData();if(a.mission!=='saven')throw Error('Fracht automatisch sichern ist für den Auftrag Saven vorgesehen.');
 const q=await api('/api/tool',{...a,typ:'flugzeit'}),p=view.planeten.find(p=>p.koord===a.start);let capacity=q.ladekapazitaet;
 const available={...p.bestand,deuterium:Math.max(0,p.bestand.deuterium-fuelReserve(q,2))};
 $('cargo-inputs').querySelectorAll('input').forEach(i=>{const n=Math.max(0,Math.min(capacity,Math.floor(available[i.dataset.map]||0)));i.value=n;capacity-=n;});flightSummary(q,fleetData());
}
function compactView(v){
 const c={...v};if(c.kommunikation)c.kommunikation={...c.kommunikation,briefkasten:(c.kommunikation.briefkasten||[]).slice(0,12)};for(const key of ['berichte','flottenberichte','kampfberichte','ereignisse','chronik','meldungen','nachrichten','erkundet'])if(Array.isArray(c[key]))c[key]=c[key].slice(0,5);
 c.planeten=(c.planeten||[]).map(p=>Object.fromEntries(['koord','gebaeude','bestand','rate','lager','energie','bevoelkerung','wohnraum','stabilitaet','schiffe','verteidigung','bauschleife','bauschleife_plaetze','blockade','fertigung','fachkraefte','arbeit','vorrat_reicht_sekunden'].map(k=>[k,p[k]]).concat([['baubar',p.baubar.filter(b=>!(b.fehlt||[]).length&&requirementsMet(p,b.braucht)).slice(0,12)]])));
 c.flotten=(c.flotten||[]).slice(0,12);return c;
}
document.addEventListener('click',async e=>{
 const b=e.target.closest('[data-promote],[data-explore],[data-cancel-order],[data-contract]');if(!b)return;
 try{if(b.hasAttribute('data-promote'))await command([{typ:'stufenaufstieg'}]);
 if(b.dataset.explore)pickFleet(b.dataset.explore,b.dataset.system==='true'?'system_erkunden':'spionage');
 if(b.dataset.cancelOrder)await command([{typ:'markt_storno',order:Number(b.dataset.cancelOrder)}]);
 if(b.dataset.contract)await command([{typ:b.dataset.contractAction,vertrag:Number(b.dataset.contract)}]);}catch(err){show(err.message,true);}
});
