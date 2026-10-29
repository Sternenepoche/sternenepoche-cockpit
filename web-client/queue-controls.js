'use strict';
const QueueUI={identity:'',open:new Set()};
document.addEventListener('toggle',e=>{const q=e.target;if(!q.isConnected||!q.classList?.contains('queue-controls'))return;q.open?QueueUI.open.add(q.dataset.queueId):QueueUI.open.delete(q.dataset.queueId);},true);
function queueControls(category,planet=selectedPlanet){
 const kind=category==='buildings'?'bau':category==='research'?'forschung':category==='components'?'bauteile':'werft';
 const q=view.forschung.warteschlangen?.find(q=>q.art===kind&&(kind==='forschung'||q.planet===planet));if(!q)return '';
 const count=q.jobs.length,active=q.jobs[0]?.aktiv;if(!count)return '';
 const identity=world.world_id+':'+view.name;if(QueueUI.identity!==identity){QueueUI.identity=identity;QueueUI.open.clear();}const queueId=kind+':'+q.planet;
 return `<details class="queue-controls" data-queue-kind="${kind}" data-queue-id="${queueId}" ${QueueUI.open.has(queueId)?'open':''}><summary>Aufträge verwalten <b>${count} / ${q.max}</b></summary><p class="queue-explanation">${kind==='bau'?'Ausbauten laufen nacheinander. Vorgemerkte Aufträge starten automatisch, sobald das Lager dieses Planeten und das Budget reichen. Erst beim Start werden die Rohstoffe abgezogen.':kind==='forschung'?'Eine Forschung läuft reichsweit. Die Schlange startet das nächste Projekt automatisch, sobald Ressourcen und Voraussetzungen reichen.':'Freie Fertigungsplätze arbeiten gleichzeitig. Schwere Einheiten belegen mehrere Plätze. Rohstoffe und Besatzung sind bei Bestellung reserviert.'}</p>${count?`<ol>${q.jobs.map((j,i)=>`<li><div><strong>${esc(named(j.key))}${j.stufe?' · '+j.stufe:j.anzahl?' ×'+fmt(j.anzahl):''}</strong><small>${j.aktiv?(j.parallel!=null?j.parallel+' gleichzeitig':'In Arbeit'):esc(j.grund)} · ${queueUnbezahlt(kind,j)?'noch nicht bezahlt':'Abbruch: '+(j.erstattbare_stueck!=null?j.erstattbare_stueck+' von '+j.anzahl+' Stück erstattet':(j.erstattung_prozent??0)+' % Erstattung')}</small>${j.aktiv&&j.grund?`<small class="queue-stalled">${esc(j.grund)}</small>`:''}</div><div class="queue-buttons"><button type="button" data-queue-change="${kind}" data-index="${i}" data-delta="-1" ${i<1||j.aktiv||q.jobs[i-1]?.aktiv?'disabled':''} aria-label="${esc(named(j.key))} nach oben">↑</button><button type="button" data-queue-change="${kind}" data-index="${i}" data-delta="1" ${i===count-1||j.aktiv||q.jobs[i+1]?.aktiv?'disabled':''} aria-label="${esc(named(j.key))} nach unten">↓</button><button type="button" data-queue-change="${kind}" data-index="${i}" data-remove aria-label="${esc(named(j.key))} entfernen">✕</button></div></li>`).join('')}</ol>`:'<p>Keine Aufträge. Wähle einen Eintrag aus dem Katalog, um vorauszuplanen.</p>'}<p class="queue-explanation">Abbruch vor 75 % Fertigstellung: bezahlte Rohstoffe vollständig zurück. Ab 75 % bleibt das laufende Stück bezahlt. Unbegonnene Fertigungsstücke werden erstattet. Verschieben erhält den laufenden Fortschritt.</p></details>`;
}
// Wartende Einträge mit ↑ ↓ ✕ dort, wo die Schlange zu sehen ist (Funktionsprüfung 25.10.2026: auf dem Handy gab es keinen
// Weg zum Umsortieren, obwohl Kern und Handbuch Kap. 24 es vorsehen). Bau und Forschung bezahlen erst beim Start.
const queueUnbezahlt=(kind,j)=>!j.aktiv&&(kind==='bau'||kind==='forschung');
function queueArrows(kind,planet,q,i){
 const j=q?.jobs[i],n=q?.jobs.length||0;if(!j||j.aktiv)return '';const name=esc(named(j.key)),wo=`data-queue-change="${kind}" data-queue-planet="${esc(planet)}" data-index="${i}"`;
 return `<button type="button" class="bs-zweit queue-pfeil" ${wo} data-delta="-1" ${i<1||q.jobs[i-1]?.aktiv?'disabled':''} aria-label="${name} nach vorn">↑</button><button type="button" class="bs-zweit queue-pfeil" ${wo} data-delta="1" ${i>=n-1?'disabled':''} aria-label="${name} nach hinten">↓</button>`;
}
function queueWaiting(kind,planet,q){
 const rows=(q?.jobs||[]).map((j,i)=>[j,i]).filter(([j])=>!j.aktiv);if(!rows.length)return '';
 return `<ol class="queue-wartend">${rows.map(([j,i])=>`<li><span class="queue-wartend-text"><b>${i+1}. ${esc(named(j.key))}${j.stufe?' · Stufe '+j.stufe:j.anzahl?' ×'+fmt(j.anzahl):''}</b><small>${esc(kernLesbar(String(j.grund||'wartet')))}</small></span><span class="queue-wartend-knoepfe">${queueArrows(kind,planet,q,i)}<button type="button" class="bs-zweit queue-pfeil" data-queue-change="${kind}" data-queue-planet="${esc(planet)}" data-index="${i}" data-remove aria-label="${esc(named(j.key))} aus der Schlange nehmen">✕</button></span></li>`).join('')}</ol>`;
}
document.addEventListener('click',guard(async e=>{
 const b=e.target.closest('[data-queue-change]');if(!b)return;
 const kind=b.dataset.queueChange,q=view.forschung.warteschlangen.find(q=>q.art===kind&&(kind==='forschung'||q.planet===(b.dataset.queuePlanet||selectedPlanet)));
 const index=Number(b.dataset.index),remove=b.hasAttribute('data-remove'),j=q?.jobs[index];if(!j)return;
 if(remove&&!confirm(`${named(j.key)}${j.aktiv?' · laufenden Auftrag':' · wartenden Auftrag'} entfernen? ${queueUnbezahlt(kind,j)?'Noch nicht begonnen und nicht bezahlt – es wird nichts abgebucht.':j.erstattbare_stueck!=null?j.erstattbare_stueck+' von '+j.anzahl+' verbliebenen Stück werden erstattet.':j.erstattung_prozent===100?'Bezahlte Rohstoffe werden vollständig erstattet.':'Ab 75 %: keine Erstattung für den laufenden Auftrag bzw. das laufende Stück. Unbegonnene Stücke werden erstattet.'}`))return;
 await command([{typ:'warteschlange_aendern',planet:q.planet,art:kind,index,ziel:remove?null:index+Number(b.dataset.delta),loeschen:remove,kennung:q.kennung}]);
}));
function renderMiningOpportunities(){
 const host=$('mining-opportunities');if(!host)return;
 const sites=[...(view.bergbauziele||[]).map(t=>({...t,mission:'abbau',label:t.art})),...(view.truemmer||[]).map(t=>({...t,mission:'recyceln',label:`Trümmer · ${fmt(t.erz)} Erz · ${fmt(t.kristall)} Kristall`}))];
 setStableHTML(host, `<article class="mission-opportunities"><div class="management-heading"><h3>Expedition & Bergung</h3><button type="button" data-go="expeditions-panel">Expedition planen</button></div><p>Expeditionen fliegen an Position 0 außerhalb der Planetenbahnen. Pathfinder, Bergungsschiffe und Bordsonden erfüllen unterschiedliche Aufgaben. Rohstoffe, Bewohner, Bauteile und beschädigte Schiffsrümpfe können geborgen werden. Gefundene Rümpfe schalten keinen Nachbau frei.</p><details><summary>Kartierte Abbaustellen & Trümmer · ${sites.length}</summary>${sites.map(t=>`<p>${esc(t.koord)} · ${esc(t.label)} <button type="button" data-work-site="${esc(t.koord)}" data-work-mission="${t.mission}">Flug planen</button></p>`).join('')||'<p>Noch keine kartierten Vorkommen. Systeme mit Sonden erkunden.</p>'}</details><p>Bevölkerung, vermisste Expeditionen, Rettungsfristen und Rumpfreparaturen stehen im Expeditionsmenü.</p></article>`);
}

document.addEventListener('click',guard(async e=>{
 const b=e.target.closest('[data-work-site]');if(!b)return;
 const mission=b.dataset.workMission,keys=mission==='recyceln'?['recycler','bergungsschiff']:[mission==='abbau'?'bergbauschiff':mission==='bergen'?'bergungsschiff':'kleiner_transporter'];
 const p=[currentPlanet(),...view.planeten].find(p=>p&&keys.some(k=>p.schiffe[k]>0)),key=keys.find(k=>p?.schiffe[k]>0)||keys[0];
 if(!p){show(`Du benötigst ${named(key)} auf einem eigenen Planeten.`,true);navigateGame('werft');uiSelections.ships=key;renderCatalog('ships','ship-gallery');return;}
 pickFleet(b.dataset.workSite,mission);$('fleet').elements.start.value=p.koord;renderFleetStock();$('ship-inputs').querySelectorAll('input').forEach(i=>i.value=i.dataset.map===key?'1':'0');$('fleet').dispatchEvent(new Event('input',{bubbles:true}));
 if(['abbau','expedition'].includes(mission))$('fleet').elements.haltedauer_stunden.value='4';
}));
