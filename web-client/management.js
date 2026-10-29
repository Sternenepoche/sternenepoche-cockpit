'use strict';
const Management={filter:'all',info:null};
const catalogTarget={buildings:'gebaeude-content',research:'research',ships:'ship-gallery',defenses:'verteidigung-content'};

function pauseButton(kind,planet,paused,cls='order-pause'){
 return `<button type="button" class="${cls}" data-pause="${kind}" data-pause-planet="${esc(planet)}" data-paused="${!!paused}">${paused?'▶ Fortsetzen':'Ⅱ Pausieren'}</button>`;
}
function tileAction(category,item,n,blocked,p){
 const kind=category==='buildings'?'bau':category==='research'?'forschung':'werft';
 const q=view.forschung.warteschlangen?.find(q=>q.art===kind&&(kind==='forschung'||q.planet===p.koord));
 const index=q?.jobs.findIndex(j=>j.key===item.key)??-1;
 if(index>=0){const j=q.jobs[index];return `<div class="tile-actions">${(j.aktiv||j.manuell_pausiert)?pauseButton(kind,q.planet,j.manuell_pausiert??item.job?.manuell_pausiert,'tile-build'):`<button type="button" class="tile-build" data-order-open="auftraege" data-order-planet="${esc(q.planet)}">${index+1}. Auftrag ansehen</button>`}<button type="button" class="tile-cancel" data-queue-change="${kind}" data-queue-planet="${esc(q.planet)}" data-index="${index}" data-remove aria-label="${esc(named(item.key))} abbrechen">×</button></div>`;}
 const waiting=['buildings','research'].includes(category)&&Object.entries(item.cost).some(([g,n])=>((item.stock||p.bestand)[g]||0)<n);
 const label=waiting?`Stufe ${item.nextLevel} vormerken`:category==='buildings'?`Stufe ${item.nextLevel} ausbauen`:category==='research'?`Stufe ${item.nextLevel} erforschen`:`${n}× bauen`;
 return `<button type="button" class="tile-build" ${item.action} data-category="${category}" data-order-quantity="${n}" ${blocked?'disabled':''}>${label}</button>`;
}
const OrderMotion={identity:'',jobs:new Set(),levels:new Map()};
function animateOrderChanges(){
 const identity=world.world_id+':'+view.name,changed=OrderMotion.identity===identity;
 const jobs=new Set(),levels=new Map();
 for(const p of view.planeten){for(const [k,v] of Object.entries(p.gebaeude))levels.set(p.koord+':'+k,v);for(const [k,v] of Object.entries(p.schiffe))levels.set(p.koord+':'+k,v);}
 for(const [k,v] of Object.entries(view.forschung.stufen))levels.set('research:'+k,v);
 for(const q of view.forschung.warteschlangen||[])for(const j of q.jobs)if(j.aktiv)jobs.add((q.art==='forschung'?'research':q.planet)+':'+j.key);
 if(changed&&!matchMedia('(prefers-reduced-motion: reduce)').matches){for(const tile of document.querySelectorAll('.item-tile')){const b=tile.querySelector('[data-item]');if(!b)continue;const key=(b.dataset.category==='research'?'research':selectedPlanet)+':'+b.dataset.item;const started=jobs.has(key)&&!OrderMotion.jobs.has(key),finished=(levels.get(key)||0)>(OrderMotion.levels.get(key)||0);if(started||finished)tile.classList.add(finished?'order-finished':'order-started');}}
 Object.assign(OrderMotion,{identity,jobs,levels});
}
function fleetPhase(f,now=GameTime.now()){
 const returning=f.zustand==='rueckflug',holding=f.zustand==='im_orbit',d=f.flugdetails||{};
 const end=returning?f.rueckkehr_sekunden:holding?f.orbit_ende_sekunden:f.ankunft_sekunden;
 const duration=d.aktuelle_dauer??(f.ankunft_sekunden-f.abflug_sekunden),start=returning?(d.rueck_beginn??end-duration):holding?f.ankunft_sekunden:f.abflug_sekunden;
 const progress=Math.max(0,Math.min(1,(now-start)/Math.max(1,end-start)));
 return {returning,holding,end,start,duration,position:returning?(d.rueck_position??1)*(1-progress):holding?1:progress,label:returning?'Rückflug':holding?'Im Orbit':'Hinflug',speed:returning?d.rueck_tempo_prozent:d.hin_tempo_prozent,left:f.rueckkehr?.ziel||f.start,right:d.umgeleitet?d.rueck_von:f.ziel};
}
function fleetShipEntries(f){return Object.entries(f.schiffe||{}).filter(([,n])=>n>0).sort((a,b)=>b[1]-a[1]||a[0].localeCompare(b[0]));}
function fleetPlanetImage(coord,f){
 const p=[f.strecke?.start,f.strecke?.ziel,...view.planeten,...(view.atlas||[])].find(p=>p?.koord===coord)||{koord:coord};
 const known=!!p.zone,owner=p.spieler||(view.planeten.some(q=>q.koord===coord)?view.name:p.bekannt?'Unbesiedelt':'Unaufgeklärt');
 return `<span class="route-world ${known?'':'route-world-unknown'}">${known?`<img src="${esc(planetArt(p))}" alt="${esc(named(p.zone))}-Planet">`:'<span class="route-unknown-globe" aria-label="Unaufgeklärter Planet"></span>'}</span><span class="route-world-copy"><strong>${esc(coord)}</strong><small>${esc(owner)}</small></span>`;
}
function fleetRouteTrack(f,phase){
 const lead=fleetShipEntries(f)[0]?.[0]||'spionagesonde',distance=f.strecke?.entfernung??f.flugdetails?.distanz;
 return `<span class="route-track" aria-label="${phase.label}: ${esc(named(lead))}"><span class="route-distance">ENTFERNUNG <strong>${distance!=null?fmt(distance)+' RE':'Nicht gespeichert'}</strong></span><span class="route-endpoint route-origin">${fleetPlanetImage(phase.left,f)}</span><span class="route-space-lane"><svg class="route-orbit" viewBox="0 0 300 100" preserveAspectRatio="none" aria-hidden="true"><path d="M 0 70 Q 150 0 300 70" class="route-orbit-glow"/><path d="M 0 70 Q 150 0 300 70" class="route-orbit-line"/></svg><span class="route-vessel"><span class="route-engine" aria-hidden="true"></span><img class="route-ship" data-route-lead="${esc(lead)}" src="${esc(art('ships',lead))}" alt="${esc(named(lead))}"></span></span><span class="route-endpoint route-destination">${fleetPlanetImage(phase.right,f)}</span><span class="route-progress"><span data-route-progress></span><span data-route-distance-left></span></span></span>`;
}
function fleetRouteDetails(f){
 const phase=fleetPhase(f),d=f.flugdetails||{},k=f.kampfkraft||{},fact=(label,value)=>`<div><dt>${label}</dt><dd>${value}</dd></div>`;
 return `<div class="route-detail"><div class="fleet-mission-heading"><span class="eyebrow">${esc(fleetMissionName(f))} · FLOTTE #${f.flotte}${f.verband?' · VERBAND #'+f.verband:''}</span><h4>${esc(phase.returning?phase.right:phase.left)} → ${esc(phase.returning?phase.left:phase.right)}</h4></div><dl class="fleet-flight-facts">${fact('Auftrag',esc(fleetMissionName(f)))}${fact('Entfernung der Planeten',fmt(f.strecke?.entfernung??d.distanz)+' RE')}${fact('Phase',phase.label)}${fact('Tempo',phase.speed!=null?fmt(phase.speed)+' %':'Nicht gespeichert')}${fact('Dauer dieser Etappe',GameTime.duration(phase.duration)+' Spielzeit')}${fact('Ankunft dieser Etappe',phase.end?'Tag '+(Math.floor(phase.end/86400)+1)+' · '+[Math.floor(phase.end/3600)%24,Math.floor(phase.end/60)%60,Math.floor(phase.end)%60].map(n=>String(n).padStart(2,'0')).join(':'):'Offen')}${fact('Restzeit',`<span data-route-remaining></span>`)}${fact('Deuterium Hinflug',d.treibstoff_hin!=null?fmt(d.treibstoff_hin):'Nicht gespeichert')}${fact('Deuterium Rückflug reserviert',d.treibstoff_rueck!=null?fmt(d.treibstoff_rueck):'Nicht gespeichert')}</dl><h4>Gesamte Kampfkraft</h4><dl class="fleet-flight-facts fleet-strength">${fact('Angriff',fmt(k.angriff||0))}${fact('Schild',fmt(k.schild||0))}${fact('Struktur',fmt(k.struktur||0))}</dl><p class="fleet-detail-note">Summe der aktuellen Einheitenwerte mit Forschung und Volksfaktoren. Zielkonter, Schnellfeuer und gegnerische Abwehr bestimmen den Kampfausgang.</p><h4>Schiffszusammensetzung · ${fmt(fleetShipEntries(f).reduce((n,[,v])=>n+v,0))} Einheiten</h4><div class="fleet-detail-ships">${fleetShipEntries(f).map(([key,n])=>`<button type="button" data-help-unit="${esc(key)}">${imageTag('ships',key)}<span><strong>${fmt(n)} × ${esc(named(key))}</strong><small>Schiffsdaten öffnen ↗</small></span></button>`).join('')}</div><details class="fleet-detail-cargo"><summary>Fracht, Bodentruppen & Passagiere</summary><p>Frachtraum: ${fmt(f.frachtraum?.belegt||0)} / ${fmt(f.frachtraum?.gesamt||0)} · ${fmt(f.frachtraum?.frei||0)} frei</p><p>Fracht: ${esc(goods(f.ladung))||'Leer'}</p><p>Passagiere: ${fmt(f.passagiere||0)} · Siedler: ${fmt(f.siedler||0)}</p><p>Bodentruppen: ${esc(goods(Object.fromEntries(Object.entries(f.bodentruppen||{}).filter(([,n])=>n>0))))||'Keine'}</p>${f.gerettete?`<p>${fmt(f.gerettete)} gerettete Überlebende</p>`:''}${Object.keys(f.fremde_schiffe||{}).length?`<p>Übernommene Schiffe: ${esc(goods(f.fremde_schiffe))}</p>`:''}</details>${f.rueckkehr?.wartend?`<p class="warning">${esc(f.rueckkehr.grund)}</p>`:''}<div class="section-actions">${f.mission==='angriff'&&!f.verband?`<button type="button" data-open-group="${f.flotte}">Verband eröffnen</button>`:''}${f.zustand!=='rueckflug'?`${f.gestrandet?`<button type="button" class="secondary" data-es-gestrandet="${f.flotte}">Weiterfliegen</button><button type="button" class="secondary" data-es-versorgen="${f.flotte}">Versorgen</button><button type="button" class="secondary" data-es-uebergeben="${f.flotte}">Übergeben</button>`:`<button type="button" data-recall="${f.flotte}">Flotte zurückrufen</button>`}`:''}<button type="button" data-go="berichte">Missionsberichte öffnen</button></div></div>`;
}
function fleetMissionName(f){return ({angriff:'Angriff',transport:'Lieferung / Transport',stationieren:'Stationierung',kolonisieren:'Kolonisierung',kampfkolonisieren:'Kampfkolonisierung',invasion:'Invasion',halten:'Verbündeten unterstützen',saven:'Sicherungsflug',recyceln:'Trümmer bergen',abbau:'Rohstoffabbau',spionage:'Spionage',system_erkunden:'Systemerkundung',expedition:'Expedition',bergen:'Rettung'})[f.mission]||named(f.mission);}
const FleetFeedback={target:'hub',messages:new Map()};
function fleetFeedbackMarkup(key){const m=FleetFeedback.messages.get(key);return m?`<div class="fleet-inline-feedback ${m.error?'error':''}" role="${m.error?'alert':'status'}"><span>${esc(m.text)}</span><button type="button" data-clear-fleet-feedback="${esc(key)}" aria-label="Meldung schließen">×</button></div>`:'';}
window.showFleetFeedback=(text,error)=>{if(!$('flotten')||$('flotten').hidden)return false;FleetFeedback.messages.set(FleetFeedback.target,{text,error});renderFleetRoutes();for(const key of ['hub','planner'])setStableHTML($('fleet-feedback-'+key),fleetFeedbackMarkup(key));return true;};
document.addEventListener('click',e=>{if(!$('flotten')?.contains(e.target))return;const route=e.target.closest('[data-fleet-route]');FleetFeedback.target=route?route.dataset.fleetRoute:e.target.closest('#fleet-planner')?'planner':'hub';const clear=e.target.closest('[data-clear-fleet-feedback]');if(clear){FleetFeedback.messages.delete(clear.dataset.clearFleetFeedback);renderFleetRoutes();for(const key of ['hub','planner'])setStableHTML($('fleet-feedback-'+key),fleetFeedbackMarkup(key));}},true);
document.addEventListener('submit',e=>{if(e.target.id==='fleet')FleetFeedback.target='planner';},true);
function fleetCargoPreview(f){
 const cargo=Object.entries(f.ladung||{}).filter(([,n])=>n>0),room=f.frachtraum||{};
 return `<span class="route-cargo-preview"><span class="route-cargo-heading"><b>${cargo.length?'Fracht & Beute':'Frachtraum leer'}</b><small>${fmt(room.belegt||0)} / ${fmt(room.gesamt||0)}</small></span>${cargo.length?`<span class="route-cargo-items">${cargo.map(([key,n])=>`<span><img src="${esc(art('resources',key,''))}" alt=""><span><small>${esc(named(key))}</small><strong>${fmt(n)}</strong></span></span>`).join('')}</span>`:''}</span>`;
}
function installFleetHub(){
 const host=$('flotten-content');$('flotten').querySelector('h2').textContent='Flottenkommando';document.querySelectorAll('[data-tab="flotten"]').forEach(n=>n.textContent='Flottenkommando');
 for(const id of ['kampf','expeditions-panel']){document.querySelectorAll('[data-tab="'+id+'"],#section-picker option[value="'+id+'"]').forEach(n=>n.remove());}
 // Keine Knopfreihe mehr (Karl 19.10.2026: „nicht so langweilig mit 3 Button und 3 Tabellen“): der Flottenhangar
 // (progression.js) ist die Übersicht, „Flug planen ›“ steht in seinem Kopf.
 host.insertAdjacentHTML('afterbegin','<div id="fleet-feedback-hub"></div>');$('fleet-feedback-hub').after($('fleet-hangar'));$('fleet-planner').insertAdjacentHTML('afterbegin','<div id="fleet-feedback-planner"></div>');$('fleet-hangar').after($('fleets'));$('fleets').after($('fleet-planner'));$('fleet-planner').after($('flight-quote'));
 const support=document.createElement('section');support.id='fleet-support';support.innerHTML='<h3>Weitere Aufgaben</h3><details id="fleet-combat"><summary>Kampfplanung & Verbände</summary></details><details id="fleet-expeditions"><summary>Expeditionen, Rettung & Personentransport</summary></details><details id="fleet-salvage"><summary>Abbau & kartierte Trümmerfelder</summary></details>';host.append(support);
 $('fleet-combat').append($('kampf-content'));const exp=$('expeditions-panel');exp.hidden=false;exp.classList.remove('panel');$('fleet-expeditions').append(exp);$('fleet-salvage').append($('mining-opportunities'));
}
document.addEventListener('click',e=>{
 const b=e.target.closest('[data-fleet-section],[data-station-plan],[data-fleet-expand]');if(!b)return;
 if(b.hasAttribute('data-fleet-expand')){document.querySelectorAll('details[data-fleet-route],details[data-fleet-location],details[data-fleet-location] details,details.fleet-delivery').forEach(n=>n.open=b.dataset.fleetExpand==='true');return;}
 if(b.dataset.fleetSection){const n=$(b.dataset.fleetSection);if(n.tagName==='DETAILS')n.open=true;n.scrollIntoView({block:'start'});return;}
 const p=view.planeten.find(p=>p.koord===b.dataset.stationPlan);if(!p)return;
 const form=$('fleet');form.elements.start.value=p.koord;form.elements.ziel.value='';form.elements.mission.value='stationieren';
 for(const id of ['ship-inputs','cargo-inputs'])$(id).querySelectorAll('input').forEach(n=>n.value=id==='ship-inputs'?(p.schiffe[n.dataset.map]||0):0);
 document.querySelectorAll('[data-ground-load]').forEach(n=>n.value=0);if(form.elements.termin_modus)form.elements.termin_modus.value='now';
 $('fleet-planner').open=true;renderFleetStock();form.dispatchEvent(new Event('change',{bubbles:true}));$('fleet-planner').scrollIntoView({block:'start'});form.elements.ziel.focus({preventScroll:true});
});
function fleetMarketDeliveries(){return (view.marktlieferungen||[]).length?`<section class="fleet-market-deliveries"><h3>Handelslieferungen · ${view.marktlieferungen.length}</h3>${view.marktlieferungen.map(d=>`<details class="fleet-delivery" id="market-delivery-${d.id??(d.ziel+':'+d.gut+':'+d.naechste_ankunft)}"><summary><img src="${esc(art('resources',d.gut,''))}" alt=""><span><b>Handel · ${fmt(d.menge)} ${esc(named(d.gut))}</b><small>Lieferung nach ${esc(d.ziel)}</small></span><span data-arrival="${d.naechste_ankunft}"></span></summary><p>${esc(d.hinweis)}</p><p>Die Marktlogistik liefert die gekauften Waren. Sie belegt keine eigenen Schiffe und kann nicht zurückgerufen werden.</p><button type="button" data-go="aktionen">Handel öffnen</button></details>`).join('')}</section>`:'';}
document.addEventListener('toggle',e=>{if(e.target.matches?.('[data-fleet-route]')){const n=e.target.querySelector('[data-route-toggle]');if(n)n.textContent=e.target.open?'Details einklappen ⌃':'Details aufklappen ⌄';}},true);
function renderFleetRoutes(){
 const host=$('fleets'),fleets=view.flotten||[],scan=view.entdeckungen?.systemauftrag,opened=new Set([...document.querySelectorAll('.fleet-route[open]')].map(n=>n.dataset.fleetRoute));
 setStableHTML(host, `<svg class="route-filter-defs" width="0" height="0" aria-hidden="true"><defs><filter id="route-ship-cutout" color-interpolation-filters="sRGB"><feColorMatrix in="SourceGraphic" type="matrix" values="0 0 0 0 1 0 0 0 0 1 0 0 0 0 1 .333 .333 .333 0 0" result="luma"/><feComponentTransfer in="luma" result="mask"><feFuncA type="linear" slope="12" intercept="-.50"/></feComponentTransfer><feComposite in="SourceGraphic" in2="mask" operator="in"/></filter></defs></svg>${scan?`<aside class="scan-status"><strong>Systemerkundung ${esc(scan.ziel.split(':').slice(0,2).join(':'))}</strong><span>${scan.gestartet} / ${scan.gesamt} Scans gestartet · ${scan.aktiv.length} unterwegs</span><small>${esc(scan.grund||'Sonden arbeiten die Ziele automatisch ab.')}</small><button type="button" data-stop-scan>Weitere Starts stoppen</button></aside>`:''}<div class="management-heading"><h3>Alle Flüge im Detail <b>${fleets.length}</b></h3></div>${fleets.length?fleets.map(f=>{const phase=fleetPhase(f);return `<details class="fleet-route" data-fleet-route="${f.flotte}" data-returning="${phase.returning}" data-holding="${phase.holding}" ${opened.has(String(f.flotte))?'open':''}><summary><span class="route-title"><b>${esc(fleetMissionName(f))} · #${f.flotte}</b><small>${esc(phase.left)} ⇄ ${esc(phase.right)}</small><small class="route-compact-distance">${fmt(f.strecke?.entfernung??f.flugdetails?.distanz)} RE · ${esc(named(fleetShipEntries(f)[0]?.[0]||"spionagesonde"))}</small></span>${fleetRouteTrack(f,phase)}${fleetCargoPreview(f)}<span class="route-clock"><b data-route-time></b><small>${phase.label}${phase.speed!=null?' · Tempo '+fmt(phase.speed)+' %':''}</small><small data-route-toggle>${opened.has(String(f.flotte))?"Details einklappen ⌃":"Details aufklappen ⌄"}</small></span></summary>${fleetFeedbackMarkup(String(f.flotte))}${fleetRouteDetails(f)}</details>${f.zustand!=='rueckflug'&&!opened.has(String(f.flotte))?`<div class="route-schnell">${f.gestrandet?`<button type="button" class="secondary" data-es-gestrandet="${f.flotte}">Flotte #${f.flotte} weiterfliegen</button>`:`<button type="button" class="secondary" data-recall="${f.flotte}">Flotte #${f.flotte} zurückrufen</button>`}</div>`:''}`;}).join(''):'<p class="orders-empty">Keine Flotte unterwegs. Plane eine Expedition, einen Transport oder einen Bergungseinsatz.</p>'}${fleetMarketDeliveries()}`);
 tickFleetRoutes(GameTime.now());
}
function tickFleetRoutes(now){
 for(const node of document.querySelectorAll('[data-fleet-route]')){const f=view.flotten.find(f=>String(f.flotte)===node.dataset.fleetRoute);if(!f)continue;const phase=fleetPhase(f,now);node.style.setProperty('--flight',phase.position);node.style.setProperty('--flight-arc',70-140*phase.position*(1-phase.position));const progress=node.querySelector('[data-route-progress]'),distanceLeft=node.querySelector('[data-route-distance-left]'),distance=f.strecke?.entfernung??f.flugdetails?.distanz;const completed=phase.holding?1:Math.max(0,Math.min(1,(now-phase.start)/Math.max(1,phase.end-phase.start)));if(progress){const text=phase.label+' · '+Math.floor(completed*100)+' %';if(progress.textContent!==text)progress.textContent=text;}if(distanceLeft&&distance!=null){const remaining=distance*(phase.returning?phase.position:phase.holding?0:1-phase.position),text=fmt(Math.ceil(remaining))+' RE verbleibend';if(distanceLeft.textContent!==text)distanceLeft.textContent=text;}const seconds=Math.max(0,phase.end-now),real=GameTime.duration(seconds/Math.max(1,world.tempo||1));for(const label of node.querySelectorAll('[data-route-time],[data-route-remaining]')){const txt=phase.end?(label.hasAttribute('data-route-remaining')?GameTime.duration(seconds)+' Spielzeit · '+real+' Echtzeit':real+' Echtzeit'):'Im Orbit';if(label.textContent!==txt)label.textContent=txt;}}
}

document.addEventListener('click',guard(async e=>{
 const b=e.target.closest('[data-pause],[data-demolish],[data-open-fleet]');if(!b)return;
 if(b.hasAttribute('data-open-fleet')){navigateGame('flotten');$('fleet-planner').open=true;$('fleet-planner').scrollIntoView({block:'start'});return;}
 if(b.dataset.pause){await command([{typ:'auftrag_pause',planet:b.dataset.pausePlanet,art:b.dataset.pause,pausiert:b.dataset.paused!=='true'}]);return;}
 if(b.dataset.demolish){const planet=b.dataset.demolishPlanet,p=view.planeten.find(p=>p.koord===planet);if(!p)return;if(confirm(`${named(b.dataset.demolish)} auf ${planet}: Stufe ${p.gebaeude[b.dataset.demolish]} abreißen? Keine Rohstofferstattung. Leistung und Kapazitäten sinken; ein Baufeld wird frei.`)){await command([{typ:'abreissen',planet,gebaeude:b.dataset.demolish}]);if(Management.info)showItemInfo(Management.info.category,Management.info.key);}return;}
}));

function ordersPanel(){
 const planets=view.planeten.filter(p=>Management.filter==='all'||p.koord===Management.filter);
 const groups=[['bau','Gebäude','buildings','gebaeude'],['forschung','Forschung','research','forschen'],['werft','Schiffe & Verteidigung','ships','werft']];
 const queues=view.forschung.warteschlangen||[];
 return `<section class="orders-dashboard" aria-label="Bauaufträge"><div class="management-heading"><div><h3>Bauaufträge</h3><p>5 Plätze je Planet und Kategorie · Forschung einmal für dein gesamtes Reich · Countdown in Echtzeit</p></div><label>Ansicht<select data-orders-planet><option value="all">Alle Planeten</option>${view.planeten.map(p=>`<option value="${esc(p.koord)}" ${Management.filter===p.koord?'selected':''}>${esc(p.koord)}${p.heimat?' · Heimat':''}</option>`).join('')}</select></label></div><div class="orders-columns">${groups.map(([kind,title,category,destination])=>{
  const selected=queues.filter(q=>q.art===kind&&(kind==='forschung'||planets.some(p=>p.koord===q.planet)));
  const count=selected.reduce((n,q)=>n+q.jobs.length,0);
  return `<article class="orders-group"><h4>${title} <b>${count} / ${kind==='forschung'?5:planets.length*5}</b></h4>${selected.map(q=>{
   const p=view.planeten.find(p=>p.koord===q.planet),actual=kind==='bau'?p.bauschleife:kind==='werft'?p.fertigung.filter(j=>j.schleife==='werft'):[...(view.forschung.aktiv?[view.forschung.aktiv]:[]),...view.forschung.schlange.map(f=>({forschung:f,wartet:true}))];
   return `<div class="orders-planet"><button class="orders-location" type="button" data-order-open="${destination}" data-order-planet="${esc(q.planet)}"><span class="orders-location-name">${kind==='forschung'?'Reichsweite Forschung':esc(q.planet)}</span><span class="orders-occupancy">${q.jobs.length} / ${q.max}</span>${uiIcon('arrow')}</button>${q.jobs.length?`<ol>${q.jobs.map((j,i)=>{
    const job={...actual[i],planet:q.planet};
    return `<li><button type="button" class="order-description" data-order-open="${destination}" data-order-planet="${esc(q.planet)}"><strong>${esc(named(j.key))}</strong><small>${j.stufe?'Stufe '+j.stufe:(j.anzahl||1)+' Stück'} · ${j.aktiv?'In Arbeit':esc(j.grund)}</small></button>${jobLine(job)}${i===0&&j.aktiv?pauseButton(kind,q.planet,job.manuell_pausiert):''}${queueArrows(kind,q.planet,q,i)}${job.serie_fertig_sekunden?`<small>Gesamte Serie: <span data-arrival="${job.serie_fertig_sekunden}"></span> Spielzeit</small>`:''}<button type="button" class="order-cancel" data-queue-change="${kind}" data-queue-planet="${esc(q.planet)}" data-index="${i}" data-remove aria-label="${esc(named(j.key))} auf ${esc(q.planet)} abbrechen">✕</button></li>`;
   }).join('')}</ol>`:`<div class="orders-empty"><span>${kind==='forschung'?'Labore bereit für dein nächstes Projekt.':kind==='werft'?'Werft bereit für den nächsten Auftrag.':'Bereit für den nächsten Ausbau.'}</span><button type="button" class="ui-action" data-order-open="${destination}" data-order-planet="${esc(q.planet)}"><span>${kind==='forschung'?'Forschung wählen':kind==='werft'?'Schiffe planen':'Ausbau planen'}</span>${uiIcon('arrow')}</button></div>`}</div>`;
  }).join('')}</article>`;
 }).join('')}</div></section>`;
}

function maxBuildable(item){
 const p=currentPlanet(),r=item.r;
 let cap=Math.min(10000,...Object.entries(item.cost).filter(([,n])=>n>0).map(([g,n])=>Math.floor((p.bestand[g]||0)/n)));
 const needsTech=Object.entries(rules.catalog.einheiten[item.key]?.taktik?.voraussetzungen||{}).some(([k,n])=>(view.forschung.stufen[k]||0)<n);if(needsTech)cap=Math.min(cap,view.entdeckungen?.bauplaene?.[item.key]||0);
 if(r.besatzung)cap=Math.min(cap,Math.floor((p.bevoelkerung-500)/r.besatzung));
 if(r.max_anzahl)cap=Math.min(cap,r.max_anzahl-item.level-p.fertigung.filter(j=>j.produkt===item.key).reduce((n,j)=>n+j.rest,0));
 return Math.max(0,cap);
}

// Stage costs and base values come from the same fixed-point core as construction.
function productionFactor(key){return currentPlanet()?.industrie&&['erzmine','kristallmine','deuteriumsynthesizer','farm','giesserei','elektronikwerk','konsumgueterwerk','xenoextraktor'].includes(key)?2:1;}
function buildingStages(item){return (rules.catalog.ausbaustufen?.[item.key]||[]).map(s=>({...s,leistung:item.key==='akademie'&&currentPlanet()?.industrie?20*s.stufe:s.leistung*productionFactor(item.key)}));}
// Wirkung des nächsten Ausbaus für die Bauknöpfe (Bauseite, Kachel, Technologiebaum): Strombedarf jetzt → danach,
// Stromerzeugung der Kraftwerke, Bilanz des Planeten danach, Ertrag. Grundwerte bei voller Besetzung (wie Stufentabelle).
function ausbauWirkung(item,p){
 const st=buildingStages(item);if(!st.length)return null;const max=st.length-1;if(item.level>=max)return null;
 const j=st[Math.min(item.level,max)],n=st[Math.min(item.level+1,max)],kraftwerk=['solarkraftwerk','fusionskraftwerk'].includes(item.key);
 const mehr=(n.energie||0)-(j.energie||0),erzeugtMehr=kraftwerk?(n.leistung||0)-(j.leistung||0):0,saldo=(p?.energie?.erzeugung||0)-(p?.energie?.verbrauch||0);
 return {mehr,jetzt:j.energie||0,dann:n.energie||0,erzeugtMehr,kraftwerk,saldo,saldoDanach:saldo-mehr+erzeugtMehr,
  label:stageOutputLabel(item.key),leistungJetzt:j.leistung||0,leistungDann:n.leistung||0,stufe:item.level+1};
}
function ausbauWirkungHtml(w,klein=false){
 if(!w)return '';const z=v=>(v>0?'+':v<0?'−':'±')+fmt(Math.abs(Math.round(v))),fehlt=w.saldoDanach<0;
 if(klein){const t=w.kraftwerk?`⚡ ${z(w.erzeugtMehr)}`:w.mehr?`⚡ ${z(-w.mehr)}`:'';return t?`<small class="aw-kachel${fehlt?' minus':''}" title="${w.kraftwerk?'Mehr Stromerzeugung':'Mehr Strombedarf'} mit Stufe ${w.stufe} – wirkt so auf die Strombilanz · danach ${z(w.saldoDanach)}">${t}</small>`:'';}
 const strom=w.kraftwerk?`<span>⚡ Erzeugung <b>${z(w.erzeugtMehr)}</b> <small>${fmt(w.leistungJetzt)} → ${fmt(w.leistungDann)}</small></span>`
  :`<span>⚡ Strombedarf <b>${w.mehr?z(w.mehr):'kein Mehrbedarf'}</b> <small>${fmt(w.jetzt)} → ${fmt(w.dann)}</small></span>`;
 const ertrag=!w.kraftwerk&&(w.leistungJetzt||w.leistungDann)?`<span>${esc(w.label)} <b>${fmt(w.leistungJetzt)} → ${fmt(w.leistungDann)}</b> <small>(${z(w.leistungDann-w.leistungJetzt)})</small></span>`:'';
 const bilanz=`<span class="aw-bilanz">Strombilanz danach <b>${z(w.saldoDanach)}</b>${fehlt?' – zu wenig Strom: erst Kraftwerk ausbauen, sonst arbeiten die Anlagen nur anteilig':''}</span>`;
 return `<div class="aw-wirkung${fehlt?' minus':''}" aria-label="Wirkung von Stufe ${w.stufe}"><small class="aw-titel">Mit Stufe ${w.stufe}</small>${strom}${ertrag}${bilanz}</div>`;
}
function buildingStageCosts(key,level){return rules.catalog.ausbaustufen?.[key]?.[level]?.kosten;}
function stageCosts(cost){return `<span class="stage-costs">${Object.entries(cost||{}).map(([g,n])=>`<span title="${esc(named(g))}">${imageTag('resources',g,'')}<span>${fmt(n)}</span></span>`).join('')}</span>`;}
function stageOutputLabel(key){return ({lager:'Lager / Rohstoff',bunker:'Schutz / Rohstoff',wohnblock:'Wohnraum',solarkraftwerk:'Energie',fusionskraftwerk:'Energie',akademie:'Ausbildungsplätze',labor:'FP / h'})[key]||'Leistung / h';}
function stageTable(item,rows,compare=false){
 const all=buildingStages(item),positive=n=>(n>0?'+':'')+fmt(n),hasOutput=all.some(s=>s.leistung>0);
 return `<div class="stage-scroll ${compare?'stage-comparison':''}" tabindex="0" aria-label="${compare?'Aktuelle und nächste Ausbaustufe':'Alle Ausbaustufen'} – ${esc(named(item.key))}"><table class="stage-table"><thead><tr><th scope="col">Stufe</th>${hasOutput?`<th scope="col">${esc(stageOutputLabel(item.key))}</th><th scope="col">Steigerung</th>`:''}<th scope="col">Strombedarf</th><th scope="col">Baukosten der Stufe</th></tr></thead><tbody>${rows.map(s=>{const now=s.stufe===item.level,next=s.stufe===item.level+1;return `<tr data-stage="${s.stufe}" class="${now?'stage-current':next?'stage-next':''}" ${now?'aria-current="true"':''}><th scope="row"><b>${s.stufe}</b>${now?'<small>Aktuell</small>':next?'<small>Nächste</small>':s.stufe===all.length-1?'<small>Maximum</small>':''}</th>${hasOutput?`<td data-label="${esc(stageOutputLabel(item.key))}">${fmt(s.leistung)}</td><td data-label="Steigerung" class="stage-gain">${s.stufe?positive(s.leistung-all[s.stufe-1].leistung):'—'}</td>`:''}<td data-label="Strombedarf">${fmt(s.energie)}${compare&&next?`<small>${positive(s.energie-all[item.level].energie)} Bedarf</small>`:''}</td><td data-label="Baukosten">${compare&&now?'<span class="stage-owned">Bereits vorhanden</span>':stageCosts(s.kosten)}</td></tr>`;}).join('')}</tbody></table></div>`;
}
function buildingProgression(item){
 const all=buildingStages(item);if(!all.length)return '<p>Ausbaudaten werden vom Spielserver geladen.</p>';
 const max=all.length-1,now=Math.min(item.level,max),next=Math.min(now+1,max);
 return `<div class="building-progression"><div class="stage-heading"><strong>${esc(named(item.key))}</strong><span>${now===max?'Voll ausgebaut':`Stufe ${now} → ${next}`} · Maximum ${max} · Grundwerte</span></div>${stageTable(item,all.slice(now,next+1),true)}<details class="stage-history"><summary>Alle ${max} Ausbaustufen <span>Leistung · Zuwachs · Kosten</span></summary><div class="stage-toolbar"><small>Grundwerte bei voller Besetzung. Planet, Volk, Forschung und Versorgung verändern die Leistung.</small><button type="button" class="ui-action" data-stage-last><span>Stufe ${max}</span>${uiIcon('down')}</button></div>${stageTable(item,all.slice(1))}</details></div>`;
}
document.addEventListener('click',e=>{const b=e.target.closest('[data-stage-last]');if(b){const list=b.closest('.stage-history').querySelector('.stage-scroll');list.scrollTop=list.scrollHeight;}});
function progressionTable(category,item){
 const r=item.r,rows=[];
 if(category==='buildings')return buildingProgression(item);
 if(category==='research'){
  for(let level=Math.max(1,item.level);level<=Math.min(30,Math.max(1,item.level)+5);level++)rows.push([level,esc(goods(Object.fromEntries(Object.entries(r.kosten).map(([g,n])=>[g,Math.floor(n*r.faktor**(level-1))])))),fmt(r.fp*r.faktor**(level-1)),esc(r.wirkung)]);
  return `<div class="table-wrap">${table(['Stufe','Kosten','Forschungspunkte','Wirkung je Stufe'],rows)}</div>`;
 }
 const values=view.einheiten_kosten.find(x=>x.einheit===item.key)?.werte||{};
 for(const [title,key] of [['Angriff','angriff'],['Schild','schild'],['Struktur','struktur'],['Tempo','tempo'],['Laderaum','ladung']])rows.push([title,fmt(r[key]||0),fmt(values[key]??r[key]??0)]);
 rows.push(['Rettungskapselplätze',fmt(values.rettungsplaetze??0),fmt(values.rettungsplaetze??0)]);if(item.key==='bergungsschiff')rows.push(['Gerettete pro Einsatz','500','500']);
 return `<div class="table-wrap">${table(['Eigenschaft','Grundwert','Mit Volk & Forschung'],rows)}</div><p>Schiffe werden in Stückzahlen gefertigt. Ihre Werte steigen reichsweit durch Forschung.</p>`;
}


// Visual relationships use catalog keys and factors; no separate combat balancing in the UI.
function dossierGroup(kind,title,hint,items){
 if(!items.length)return '';
 return `<section class="counter-group counter-${kind} ${items.every(x=>x.image.includes('assets/resources.'))?'counter-resources':''}"><h3>${esc(title)}</h3>${hint?`<p class="counter-hint">${esc(hint)}</p>`:''}<ul class="counter-grid">${items.map(x=>`<li class="counter-unit" data-dossier-key="${esc(x.key||'')}" ${x.factor!==undefined?`data-factor="${x.factor}"`:''}><div class="counter-art"><img src="${esc(x.image)}" alt="" loading="lazy"><div class="counter-caption"><strong>${esc(x.name)}</strong><span class="counter-badge">${esc(x.badge)}</span></div></div><span class="counter-effect" title="${esc(x.effect)}">${esc(x.effect.replace(" % eigener Schaden"," % Schaden").replace(" % erlittener Schaden"," % gegen dich").replace("Erzeugt / Spielstunde","Ertrag / h").replace("Verbraucht / Spielstunde","Bedarf / h").replace(" Schadenbudget"," Schaden"))}</span></li>`).join('')}</ul></section>`;
}
function dossierUnit(key,badge,effect,factor,faction=view.volk){return {key,name:named(key),image:art(rules.catalog.einheiten[key].schiff?'ships':'defenses',key,faction),badge,effect,factor};}
function dossierDamage(entries,incoming=false){return entries.map(([k,n])=>dossierUnit(k,`${fmt(n)}×`,`${n>=1?'+':''}${fmt((n-1)*100)} % ${incoming?'erlittener':'eigener'} Schaden`,n));}
function combatDossier(item){
 const c=rules.catalog,e=c.einheiten[item.key],p=e.zielprofil||{},f=p.schadensfaktoren||{},armed=p.ziele_je_runde>0;
 const incoming=Object.entries(c.einheiten).filter(([,x])=>x.zielprofil?.ziele_je_runde>0&&!(x.zielprofil.nicht_erfassbar||[]).includes(item.key));
 const rapid=(pairs,against=false)=>pairs.map(([k,n])=>dossierUnit(k,`${fmt(n)}× SF`,against?'Gegnerisches Schnellfeuer':'Eigenes Schnellfeuer',n));
 const hits=Object.entries(f).filter(([k])=>!(p.nicht_erfassbar||[]).includes(k));
 let html=`<div class="dossier-visual">${e.taktik?.rolle&&e.taktik.rolle!==item.r.wirkung?`<p class="dossier-role">${esc(e.taktik.rolle)}</p>`:''}`;
 if(armed){
  html+=dossierGroup('benefit','Stark gegen','Schadensfaktor pro Treffer; Schnellfeuer wird separat ausgewiesen.',dossierDamage(hits.filter(([,n])=>n>1)));
  html+=dossierGroup('malus','Weniger wirksam gegen','Reduzierter eigener Trefferschaden.',dossierDamage(hits.filter(([,n])=>n<1)));
  html+=dossierGroup('benefit','Eigenes Schnellfeuer',(p.verteilung||'').includes('springt')?'Zusätzliche Schüsse; nach einem Abschuss springt die Salve auf das nächste Ziel derselben Art (Mehrfachfeuer). Kein Schadensmultiplikator.':'Zusätzliche Schüsse auf dasselbe Ziel; kein Schadensmultiplikator.',rapid(Object.entries(p.schnellfeuer||{})));
 }else html+='<p class="counter-neutral">Kein eigener Angriff. Aufgabe und Schutzwerte sind entscheidend.</p>';
 html+=dossierGroup('danger','Gefährliche Gegner','Diese Einheiten haben einen erhöhten Trefferschaden gegen diesen Typ.',dossierDamage(incoming.map(([k,x])=>[k,x.zielprofil.schadensfaktoren?.[item.key]??1]).filter(([,n])=>n>1),true));
 html+=dossierGroup('danger','Schnellfeuer durch Gegner','Diese Gegner besitzen zusätzliches Schnellfeuer gegen diesen Typ.',rapid(incoming.map(([k,x])=>[k,x.zielprofil.schnellfeuer?.[item.key]||0]).filter(([,n])=>n>0),true));
 const missiles=(c.raketen||[]).filter(r=>r.zielprofil.schadensfaktoren[item.key]!==undefined);
 html+=dossierGroup('danger','Angreifbar durch Raketen','Nur bei zulässiger Mission, Reichweite und erreichbarem Ziel. Raketen umgehen Schilde.',missiles.map(r=>({key:r.key,name:r.name,image:art('missiles',r.key),badge:`${fmt(r.zielprofil.schadensfaktoren[item.key])}×`,effect:`${fmt(r.schaden*r.zielprofil.schadensfaktoren[item.key])} Schadenbudget / Rakete`,factor:r.zielprofil.schadensfaktoren[item.key]})));
 if(armed)html+=`<details class="dossier-targets"><summary>Zielprioritäten und Grenzen · ${p.ziele_je_runde} Ziele je Runde</summary>${['primaer','sekundaer','reserve','nicht_erfassbar'].map((key,i)=>dossierGroup(i===3?'malus':'neutral',['Primärziele','Sekundärziele','Ausweichziele','Nicht erfassbar'][i],['Zuerst anvisiert.','Wenn kein passendes Primärziel verfügbar ist.','Nach Primär- und Sekundärzielen.','Diese Einheit kann diese Typen nicht angreifen.'][i],(p[key]||[]).map(k=>dossierUnit(k,i===3?'Kein Ziel':`Priorität ${i+1}`,i===3?'Kein eigener Angriff möglich':'Zielreihenfolge, kein Schadensbonus')))).join('')}<p>${esc(p.verteilung||'')}</p></details>`;
 return html+'</div>';
}
function buildingDossier(item){
 const p=currentPlanet(),c=rules.catalog,key=item.key,w=p.industrie?.werke?.find(x=>x.gebaeude===key),goods=p.industrie?.gueter||[];
 const resource=(k,n,effect)=>({key:k,name:named(k),image:art('resources',k,''),badge:fmt(n),effect});
 let html='<div class="dossier-visual">';
 if(w){
  html+=`<p class="dossier-role">Betrieb auf ${esc(p.koord)} · Stufe ${w.stufe} · ${p.industrie.regler[key]??100} % Leistung. Mengen pro Spielstunde.</p>`;
  html+='<div class="dossier-resource-flow">'+dossierGroup('benefit','Erzeugt','Aktuell / Spielstunde',goods.flatMap((g,i)=>w.produktion[i]>0?[resource(g,w.produktion[i]/1000,'Erzeugt / Spielstunde')]:[]));
  html+=dossierGroup('malus','Verbraucht','Aktuell / Spielstunde',goods.flatMap((g,i)=>w.verbrauch[i]>0?[resource(g,w.verbrauch[i]/1000,'Verbraucht / Spielstunde')]:[]))+'</div>';
  const load=[{key:'energie',name:'Energie gebunden',image:art('buildings','solarkraftwerk'),badge:fmt(w.energiebedarf/1000),effect:'Aktueller Energiebedarf'},{key:'arbeiter',name:'Arbeitskräfte',image:art('buildings','wohnblock'),badge:`${fmt(w.arbeiter_zugewiesen/1000)} / ${fmt(w.arbeiter_bedarf/1000)}`,effect:'Zugewiesen / benötigt'},{key:'fachkraefte',name:key==='labor'?'Forscher':'Fachkräfte',image:art('buildings','akademie'),badge:`${fmt(w.fachkraefte_zugewiesen/1000)} / ${fmt(w.fachkraefte_bedarf/1000)}`,effect:'Zugewiesen / benötigt'}];
  if(['solarkraftwerk','fusionskraftwerk'].includes(key))html+=dossierGroup('benefit','Energieerzeugung','Aktuell, vor dem Bonus des Versorgungsnetzes.',[{key,name:named(key),image:art('buildings',key),badge:fmt(w.energie_vor_versorgungsnetz/1000),effect:'Erzeugte Energie'}]);
  html+=dossierGroup('neutral','Versorgung & Personal','Gebundene Kapazitäten, kein stündlicher Materialverbrauch.',load);
  if(w.engpaesse?.length)html+=`<p class="warning">${esc(w.engpaesse.join(' · '))}</p>`;
 }
 const output={giesserei:'legierung',elektronikwerk:'elektronik',konsumgueterwerk:'konsumgut'}[key];
 if(output)html+=dossierGroup('neutral',`Rezept für 1 ${named(output)}`,'Kontinuierliche Verarbeitung; bei Rohstoffmangel sinkt der Ausstoß.',Object.entries(c.wirtschaft.rezepte?.[output]||{}).map(([k,n])=>resource(k,n,`Verbrauch je 1 ${named(output)}`)));
 // Non-production effects remain authoritative facts from the shared data sheet.
 const sheet=document.createElement('div');sheet.innerHTML=dataSheet('buildings',item);
 const effects=[...sheet.querySelectorAll('.fact-row')].filter(row=>/^(Lager je|Geschützt je|Gleichzeitige Ausbildungsplätze|Forschungsplätze|Treibstoffersparnis|Baugeschwindigkeit|Bau- und Werftgeschwindigkeit|Raketenplätze|Raketenreichweite|Zusätzliche Baufelder|Zusätzlicher Wohnraum|Lokale Forschungsleistung|Lokale Energieerzeugung|Verwaltete Kolonien|Ertrag dieser Stufe|Zusätzlicher Ertrag)/.test(row.querySelector('dt').textContent)).map(row=>{const dd=row.querySelector('dd').cloneNode(true),note=dd.querySelector('small')?.textContent||`Ausbauziel: Stufe ${item.nextLevel}`;dd.querySelectorAll('small').forEach(n=>n.remove());return {key,name:row.querySelector('dt').textContent,image:art('buildings',key),badge:dd.textContent,effect:note};});
 html+=dossierGroup('benefit','Nutzen des Ausbaus',`Stufe ${item.level} → ${item.nextLevel}. Grundwerte; laufender Betrieb steht oben.`,effects);
 const unlocks=[];
 for(const [k,e] of Object.entries(c.einheiten)){const level=key==='werft'?Math.max(1,e.regel.werft||0):e.regel.braucht?.[key];if(level)unlocks.push({...dossierUnit(k,`Stufe ${level}`,`Gebäudevoraussetzung · Zeitalter ${e.regel.ab_stufe}`),level});}
 for(const [k,r] of Object.entries(c.gebaeude)){const level=r.braucht?.[key];if(level)unlocks.push({key:k,name:named(k),image:art('buildings',k),badge:`Stufe ${level}`,effect:`Gebäudevoraussetzung · Zeitalter ${r.ab_stufe}`,level});}
 if(key==='labor')for(const [k,r] of Object.entries(c.forschung))unlocks.push({key:k,name:named(k),image:art('research',k),badge:`Stufe ${r.labor}`,effect:`Labor auf der Heimatwelt · Zeitalter ${r.ab_stufe}`,level:r.labor});
 if(key==='kaserne')for(const t of c.bodentruppen?.typen||[])unlocks.push({key:t.id,name:t.name,image:art('ground_units',t.bild),badge:`Stufe ${t.kaserne}`,effect:`Ausbildung · Zeitalter ${t.ab_stufe}`,level:t.kaserne});
 if(key==='raketensilo')for(const r of c.raketen||[])unlocks.push({key:r.key,name:r.name,image:art('missiles',r.key),badge:`Stufe ${r.silo}`,effect:`Siloproduktion · Zeitalter ${r.zivilisation}`,level:r.silo});
 if(key==='orbitalwerft')for(const k of ['habitatmodul','antriebskern'])unlocks.push({key:k,name:named(k),image:art('resources',k,''),badge:'Automatisch',effect:'Vorratsziel und Materialbedarf unter Schiffswerft',level:1});
 if(unlocks.length)html+=`<details class="dossier-targets" ${unlocks.length<=4?'open':''}><summary>Ermöglicht · ${unlocks.length} Gebäude, Forschungen oder Produkte</summary>${dossierGroup('neutral','Ausbau & Freischaltungen','Weitere Forschung, Zeitalter und Ressourcen bleiben erforderlich.',unlocks.sort((a,b)=>a.level-b.level))}</details>`;
 if(!w&&!effects.length)html+=dossierGroup('neutral','Aufgabe dieser Anlage','', [{key,name:named(key),image:art('buildings',key),badge:`Stufe ${item.level}`,effect:item.r.wirkung}]);
 
 return html+'</div>';
}
function missileDossier(r,faction=view.volk){
 const c=rules.catalog,entries=Object.entries(r.zielprofil.schadensfaktoren);
 if(!r.schaden)return `<div class="dossier-visual">${dossierGroup('benefit','Automatischer Abfangschutz','Schützt den eigenen Planeten aus einem intakten Silo. Die Abfangraketen werden dabei verbraucht.',c.raketen.filter(x=>x.schaden&&x.planet).map(x=>({key:x.key,name:x.name,image:art('missiles',x.key,faction),badge:`${x.abfangbedarf} nötig`,effect:'Abfangraketen je Angriffsrakete'})))}</div>`;
 return `<div class="dossier-visual">${dossierGroup('benefit','Besonders wirksam gegen','Schadensbudget je nicht abgefangener Rakete; kein voller Schaden pro Einzelziel.',entries.filter(([,n])=>n>1).map(([k,n])=>dossierUnit(k,`${fmt(n)}×`,`${fmt(r.schaden*n)} Schadenbudget`,n,faction)))}${r.planet?dossierGroup('malus','Abfangschutz des Gegners','Bei Planetenschlägen können vorhandene Abfangraketen die Salve reduzieren.',[{key:'abfang',name:'Abfangrakete',image:art('missiles','abfang',faction),badge:`${r.abfangbedarf} nötig`,effect:'Abfangraketen je Angriffsrakete'}]):'<p class="counter-neutral">Flottenschläge werden nicht durch planetare Abfangraketen abgefangen.</p>'}<details class="dossier-targets"><summary>Alle ${entries.length} zulässigen Zielklassen</summary>${dossierGroup('neutral','Zielauswahl','Ein Zieltyp wird vor dem Start festgelegt. Kein automatischer Zielwechsel.',entries.map(([k,n])=>dossierUnit(k,`${fmt(n)}×`,`${fmt(r.schaden*n)} Schadenbudget`,n,faction)))}</details><p class="counter-neutral">${esc(r.zielprofil.verteilung)}</p></div>`;
}
function visualDossier(category,item){return category==='buildings'?buildingDossier(item):['ships','defenses'].includes(category)?combatDossier(item):'';}
function showMissileInfo(key){
 const t=currentPlanet().raketensilo?.typen.find(x=>x.regel.key===key);if(!t)return;
 const r=t.regel,dialog=$('item-info');Management.info={category:'missiles',key};dialog.style.setProperty('--detail-art',`url("${art('missiles',key)}")`);
 setStableHTML(dialog,`<header class="detail-heading"><div><small>Raketendossier · ${esc(currentPlanet().koord)}</small><h2>${esc(r.name)}</h2></div><button type="button" data-close-info aria-label="Datenblatt schließen">${uiIcon('close')}</button></header><img class="dossier-hero" src="${esc(art('missiles',key))}" alt="${esc(r.name)}">${missileReference(r,view.volk)}`);
 if(!dialog.open)dialog.showModal();dialog.scrollTop=0;
}
document.addEventListener('click',e=>{const b=e.target.closest('[data-missile-info]');if(b)showMissileInfo(b.dataset.missileInfo);});

function dossierMetrics(category,item){
 if(!['ships','defenses'].includes(category))return '';
 const q=view.einheiten_kosten.find(x=>x.einheit===item.key),v=q?.werte||item.r;
 const fields=[['Angriff',v.angriff],['Schild',v.schild],['Struktur',v.struktur]];
 if(category==='ships')fields.push(['Geschwindigkeit',v.tempo],['Laderaum',v.ladung]);
 fields.push(['Fertigungsplätze / Stück',q?.fertigungsplaetze_je_stueck||1]);
 return `<dl class="dossier-metrics">${fields.map(([k,n])=>factRow(k,fmt(n??0))).join('')}</dl><p class="counter-hint">Wirksame Werte mit deinem Volk und deiner Forschung.</p>`;
}
function showItemInfo(category,key){
 // „i“ an Schiffen (Flottenversand) und andere data-info-Knöpfe: dasselbe Infodisplay wie das „?“ der Bauseiten (Karl 20.10.:
 // „alles lückenlos“). Akademie und Fusionskraftwerk behalten hier ihre Sonderteile (Ausbildung, Deuteriumbedarf).
 if(window.Bauseite?.techinfo&&!['akademie','fusionskraftwerk'].includes(key)&&Bauseite.techinfo(category,key,'techinfo')!==false)return;
 const item=itemRecords(category).find(x=>x.key===key);if(!item)return;
 const dialog=$('item-info');Management.info={category,key};
 dialog.style.setProperty("--detail-art",`url("${art(category,key)}")`);
 setStableHTML(dialog, `<header class="detail-heading"><div><small>${esc(currentPlanet().koord)} · ${category==='buildings'?'Anlagendossier':'Datenblatt'}</small><h2>${esc(named(key))}</h2><p>${esc(item.r.wirkung)}</p></div><button type="button" class="ui-icon-button" data-close-info aria-label="Datenblatt schließen">${uiIcon('close')}</button></header>${key==='akademie'?academyTraining(currentPlanet()):''}${category!=='research'?`<img class="dossier-hero" src="${esc(art(category,key))}" alt="${esc(named(key))}">${dossierMetrics(category,item)}${visualDossier(category,item)}${category==='buildings'?Mobile035.localProduction(key):''}`:''}${category==='buildings'?`<details class="dossier-targets"><summary>Ausbau auf Stufe ${item.nextLevel} · Kosten und Bauzeit</summary>${purchaseCosts(category,item,currentPlanet().bestand)}<p>${orderDuration(category,item)}</p>${requirementSheet(category,key)}</details>`:''}${['ships','defenses'].includes(category)?`<details class="dossier-targets"><summary>Grundwerte mit aktuellen Werten vergleichen</summary>${progressionTable(category,item)}</details>`:progressionTable(category,item)}${dataSheet(category,item).replace('technical-sheet" open','technical-sheet"')}${category==='buildings'&&item.level>0?`<button type="button" class="demolish" data-demolish="${esc(key)}" data-demolish-planet="${esc(currentPlanet().koord)}">Stufe abreißen · ohne Erstattung</button>`:''}<button type="button" data-tree-target="${esc(key)}" data-tree-kind="${category}" data-tree-level="${item.nextLevel||1}" data-close-info>Voraussetzungen im Forschungsbaum ↗</button>`);
 if(!dialog.open)dialog.showModal();dialog.scrollTop=0;
 loadFusionValues(dialog);
 dialog.querySelectorAll('[data-academy-form]').forEach(refreshAcademyQuote);
}

function workforceRate(value){
 const n=Number(value)||0;
 return (n>0?'+':'')+n.toLocaleString('de-DE',{maximumFractionDigits:1});
}
function workforcePanel(p){return window.Mobile035.workforce(p);}
function renderManagement(){
 renderCommandDeck();
 // Nur ein offenes Auswahlfeld schützt vor dem Neuzeichnen; ein angetippter Knopf behält sonst den Fokus und die Liste blieb
 // nach Pausieren/Fortsetzen veraltet – der nächste Tipp sendete den falschen Wert (Funktionsprüfung 25.10.2026).
 for(const id of ['orders-dashboard-host','auftraege']){const orders=$(id),f=document.activeElement;if(orders&&!(orders.contains(f)&&/^(SELECT|INPUT|TEXTAREA)$/.test(f?.tagName||'')))setStableHTML(orders, ordersPanel());}
 const picker=$('section-picker');if(picker){picker.value=document.querySelector('#tabs [aria-selected="true"]')?.dataset.tab||'reich';for(const option of picker.options){option.hidden=!!document.querySelector('#tabs [data-tab="'+option.value+'"]')?.hidden;}}const bar=$('galaxy-own-worlds');if(bar)setStableHTML(bar, view.planeten.map(p=>`<button type="button" data-galaxy-home="${esc(p.koord)}">${p.heimat?'⌂':'◉'} ${esc(p.koord)} <small>${p.heimat?'Heimat':'Kolonie'}</small></button>`).join(''));
 const people=$('workforce-summary'),p=currentPlanet(),speed=world.tempo||1;
 for(const host of [people,$('workforce-economy')])if(host){const open=host.querySelector('details')?.open,focus=host.contains(document.activeElement)?document.activeElement:null;const selector=focus?.tagName==='SUMMARY'?'summary':focus?.dataset.workforceTarget?`[data-workforce-target="${focus.dataset.workforceTarget}"]`:focus?.hasAttribute('data-research-priority')?'[data-research-priority]':null;setStableHTML(host, workforcePanel(p));if(open)host.querySelector('details').open=true;if(selector)host.querySelector(selector)?.focus({preventScroll:true});}
 const researchPeople=$('research-personnel');if(researchPeople){const r=window.Mobile035.researchers(p);setStableHTML(researchPeople, `<strong>${fmt(r.zugewiesen||0)} / ${fmt(r.plaetze||0)} Forscher · ${esc(p.koord)}</strong><span>${workforceRate((r.fp_je_spielstunde||0)*speed)} FP / Echtzeitstunde · ${fmt(view.planeten.reduce((n,x)=>n+window.Mobile035.researchers(x).fp_je_spielstunde,0)*speed)} im gesamten Reich</span><button type="button" data-go="kolonie">Personal & Versorgung ↗</button>`);}
 window.ColonyConsole?.render();
}

function renderTrade(){
 const host=$('instant-trade');if(!host||!view||host.contains(document.activeElement))return;
 if(host.dataset.ready)return;host.dataset.ready='true';
 setStableHTML(host, `<h3>Soforthandel</h3><p>Du kannst mehrere Güter gemeinsam anbieten; der Ertrag landet sofort auf dem gewählten Planeten. Kurse und Gebühr kommen aus den Regeln der Welt, die Vorschau rechnet der Kern. <span id="trade-kurse"></span></p><form id="instant-trade-form"><label>Planet<select name="planet" class="own-planets">${view.planeten.map(p=>`<option ${p.koord===selectedPlanet?'selected':''}>${esc(p.koord)}</option>`).join('')}</select></label><div class="trade-offer">${['erz','kristall','deuterium','nahrung','credits'].map(g=>`<label>${esc(named(g))} anbieten<input type="number" name="${g}" min="0" max="1000000000" step="0.001" value="0"></label>`).join('')}</div><label>Erhalten<select name="ziel">${['deuterium','erz','kristall','nahrung','credits'].map(g=>`<option value="${g}">${esc(named(g))}</option>`).join('')}</select></label><output id="trade-preview" aria-live="polite">Angebot eingeben</output><button type="submit">Sofort tauschen</button></form><p>Credits erhältst du außerdem automatisch durch Bevölkerungssteuern. Öffentliche Marktorders darunter handeln mit anderen Reichen und benötigen einen Markt.</p>`);
}
// Vorschau vom Kern (Werkzeug soforthandel, entprellt): dieselben Kurse, Gebühr, Bestands- und Mindestprüfung wie der Befehl –
// vorher rechnete der Browser mit festen Kursen und zeigte auch Angebote, die der Kern ablehnte (Funktionsprüfung 25.10.2026).
const TRADE_GUETER=['erz','kristall','deuterium','nahrung','credits'],tradeMenge=v=>Number(v).toLocaleString('de-DE',{maximumFractionDigits:3});
let tradeTimer=0,tradeNr=0;
function tradeAngebot(f){return Object.fromEntries(TRADE_GUETER.map(g=>[g,Number(f.elements[g].value)||0]).filter(([,n])=>n>0));}
async function tradeVorschau(){const f=$('instant-trade-form'),out=$('trade-preview');if(!f||!out)return;const a=tradeAngebot(f),nr=++tradeNr;if(!Object.keys(a).length){out.textContent='';out.classList.remove('bs-rot');return;}
 try{const q=await api('/api/tool',{typ:'soforthandel',planet:f.elements.planet.value,angebot:a,ziel:f.elements.ziel.value});if(nr!==tradeNr)return;out.classList.remove('bs-rot');out.textContent=`Vorschau nach ${fmt(q.gebuehr_prozent)} % Gebühr: ${tradeMenge(q.erhalt)} ${named(q.ziel)}`;
  const k=$('trade-kurse');if(k&&q.kurse)k.textContent='Kurse je Einheit: '+Object.entries(q.kurse).map(([g,v])=>`${named(g)} ${tradeMenge(v)}`).join(' · ')+` · Gebühr ${fmt(q.gebuehr_prozent)} %`;}
 catch(err){if(nr!==tradeNr)return;out.classList.add('bs-rot');out.textContent=typeof kernLesbar==='function'?kernLesbar(err.message||String(err)):(err.message||String(err));}}
document.addEventListener('input',e=>{if(!e.target.closest('#instant-trade-form'))return;clearTimeout(tradeTimer);tradeTimer=setTimeout(tradeVorschau,400);});
document.addEventListener('change',e=>{if(e.target.hasAttribute('data-orders-planet')){Management.filter=e.target.value;const host=e.target.closest('.orders-dashboard').parentElement;setStableHTML(host, ordersPanel());renderManagement();tickGame();}if(e.target.closest('#instant-trade-form'))e.target.dispatchEvent(new Event('input',{bubbles:true}));});
document.addEventListener('submit',guard(async e=>{if(e.target.id!=='instant-trade-form')return;const f=e.target,a={typ:'soforthandel',planet:f.elements.planet.value,angebot:tradeAngebot(f),ziel:f.elements.ziel.value};const q=await api('/api/tool',a);const result=await command([a]);if(result.ergebnisse.every(r=>r.ok)){f.reset();$('trade-preview').textContent=`${tradeMenge(q.erhalt)} ${named(q.ziel)} erhalten`;}}));
document.addEventListener('click',guard(async e=>{
 const b=e.target.closest('[data-info],[data-close-info],[data-qty-preset],[data-order-open],[data-galaxy-home],#back-to-top');if(!b)return;
 if(b.hasAttribute('data-info')){showItemInfo(b.dataset.category,b.dataset.info);return;}
 if(b.hasAttribute('data-close-info')){$('item-info').close();return;}
 if(b.dataset.qtyPreset){const category=b.dataset.category,item=itemRecords(category).find(i=>i.key===uiSelections[category]);const n=b.dataset.qtyPreset==='max'?maxBuildable(item):Number(b.dataset.qtyPreset);const field=$('quantity-'+category);field.value=Math.max(1,n);field.dispatchEvent(new Event('input',{bubbles:true}));return;}
 if(b.dataset.orderOpen){selectedPlanet=b.dataset.orderPlanet;renderView();navigateGame(b.dataset.orderOpen);window.scrollTo({top:0,behavior:'instant'});return;}
 if(b.dataset.galaxyHome){selectedPlanet=b.dataset.galaxyHome;const [s,system]=selectedPlanet.split(':').map(Number);await GalaxyUI.restore({stage:'system',sektor:s,system});return;}
 if(b.id==='back-to-top')window.scrollTo({top:0,behavior:'instant'});
}));
document.addEventListener('game-navigation',()=>{if(!view)return;const category=Object.keys(catalogTarget).find(k=>!$(catalogTarget[k]).closest('section.panel').hidden);if(category)renderCatalog(category,catalogTarget[category]);renderManagement();renderTrade();tickGame();});
document.addEventListener('DOMContentLoaded',()=>{
 const nav=$('tabs');nav.insertAdjacentHTML('afterbegin','<label class="section-picker-label">Bereich wählen<select id="section-picker" aria-label="Spielbereich"></select></label>');
 const fleetArea=$('flotten-content');fleetArea.prepend($('fleets'));
 $('research-tree').hidden=true;$('kolonie').insertAdjacentHTML('afterbegin','<section id="workforce-economy" class="workforce-summary"></section><section id="planet-climate" class="workforce-summary"></section>');
 document.body.insertAdjacentHTML('beforeend','<dialog id="item-info" class="item-info-dialog"></dialog><button id="back-to-top" type="button" aria-label="Zurück nach oben">↑ Nach oben</button>');
 $('planets').insertAdjacentHTML('afterend','<div id="orders-dashboard-host"></div><section id="workforce-summary" class="workforce-summary"></section>');
 $('game').insertAdjacentHTML('beforeend','<section id="auftraege" class="panel" hidden></section>');
 document.querySelector('[data-tab="reich"]').insertAdjacentHTML('afterend','<button type="button" data-tab="auftraege" data-go="auftraege" aria-selected="false">Bauaufträge</button>');
 setStableHTML($('section-picker'), [...nav.querySelectorAll('[data-tab]')].map(b=>`<option value="${b.dataset.tab}">${b.textContent}</option>`).join(''));$('section-picker').addEventListener('change',e=>{navigateGame(e.target.value);window.scrollTo({top:0,behavior:'instant'});});
 $('fleet').elements.mission.insertAdjacentHTML('beforeend','<option value="expedition">Expedition · unbekannten Raum erkunden</option><option value="bergen">Überlebende bergen · Bergungsschiffe</option>');
 $('karte').insertAdjacentHTML('afterbegin','<nav id="galaxy-own-worlds" class="galaxy-own-worlds" aria-label="Eigene Planeten in der Galaxie"></nav>');
 $('aktionen').insertAdjacentHTML('afterbegin','<section id="instant-trade" class="instant-trade"></section>');
 const research=$('forschen');research.insertAdjacentHTML('afterbegin','<div class="research-entry"><h3>Forschungswege</h3><p>Wähle ein Ziel und sieh direkt, welche Gebäude und Technologien dafür nötig sind.</p><button type="button" data-go="freischaltungen">Forschungsbaum öffnen ↗</button><div id="research-path-inline" class="research-tree"></div></div>');
 research.insertAdjacentHTML('afterbegin','<aside id="research-personnel" class="research-personnel"></aside>');
});

document.addEventListener('click',guard(async e=>{
 const b=e.target.closest('[data-workforce-target],[data-research-priority]');if(!b)return;
 const p=view.planeten.find(x=>x.koord===b.dataset.workforcePlanet);if(!p){show('Dieser Planet gehört nicht mehr zu deinem Reich.',true);return;}
 if(b.hasAttribute('data-research-priority')){const first=['farm','solarkraftwerk','fusionskraftwerk','akademie','labor'];await command([{typ:'prioritaeten',planet:p.koord,reihenfolge:[...first,...p.prioritaeten.filter(k=>!first.includes(k))]}]);return;}
 PlanetNavigation.select(p.koord);
 uiSelections.buildings=b.dataset.workforceTarget;
 navigateGame('gebaeude');const tile=document.querySelector(`[data-info="${b.dataset.workforceTarget}"][data-category="buildings"]`)?.closest('[data-catalog-item]');
 tile?.scrollIntoView({block:'center',behavior:'instant'});tile?.querySelector('.tile-build')?.focus({preventScroll:true});
}));

let completionPollAt=0;
async function requestCompletionRefresh(){
 if(refreshBusy||document.hidden||!session||performance.now()<completionPollAt)return;
 completionPollAt=performance.now()+4000;refreshBusy=true;
 try{const next=await api('/api/view');if(next.world_id!==world.world_id)return;view=next;GameTime.sync({...world,...view});window.SternenFairplay?.reich(view);renderView();}
 catch(e){completionPollAt=performance.now()+Math.max(5000,(e.retryAfter||0)*1000);if(e.status!==429){uiConnectionLost();show(e.message,true);}}
 finally{refreshBusy=false;}
}
document.addEventListener('visibilitychange',()=>{if(!document.hidden)void refresh();});

document.addEventListener('click',guard(async e=>{const b=e.target.closest('[data-scan-system],[data-stop-scan]');if(!b)return;const stop=b.hasAttribute('data-stop-scan'),job=view.entdeckungen?.systemauftrag;const available=view.planeten.filter(p=>(p.schiffe.spionagesonde||0)>0&&p.gebaeude.raumhafen>0);const p=available.find(p=>p.koord===selectedPlanet)||available[0];if(!stop&&!p){show('Baue mindestens eine Spionagesonde und einen Raumhafen. Zurückkehrende Sonden können weitere Ziele erkunden.',true);navigateGame('werft');return;}
 // Ein Tipp für das ganze System (Karl 22.10.2026): der Kern schickt bei unkartiertem System erst die Systemsonde und
 // danach von selbst die Planetensonden; fehlende Sonden ersetzen zurückkehrende (verwaltung.rs sammelauftrag_weiter).
 await command([{typ:'system_aufklaeren',start:stop?job.start:p.koord,ziel:stop?job.ziel:b.dataset.scanSystem,abbrechen:stop}]);if(!stop)navigateGame('flotten');}));
