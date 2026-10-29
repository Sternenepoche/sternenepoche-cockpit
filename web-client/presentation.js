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
   if(type==='fertigen'&&action.properties.bauteil&&!view.planeten.some(p=>(p.gebaeude.orbitalwerft||0)>0||view.stufe>=3&&p.gebaeude.werft>=3&&p.schiffe.recycler>0))continue;
   if(type==='forschen'){const legal=(view.forschung?.moeglich||[]).filter(f=>f.labor_heimat>=f.labor&&!(f.fehlt||[]).length&&Object.entries(rules?.catalog?.forschung_voraussetzungen?.[f.forschung]?.forschung||{}).every(([k,n])=>(view.forschung.stufen[k]||0)>=n)&&(!f.infrastruktur||view.planeten.some(p=>(p.gebaeude.geheimdienst||0)>0))).map(f=>f.forschung);if(!legal.length)continue;action.properties.forschung.enum=legal;}
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
function recordCards(items,title){return items?.length?items.map(item=>`<article class="card"><div class="row"><h3>${esc(item.ziel||item.name||title)}</h3>${badge(item.zeit??'')}</div>${Object.entries(item).filter(([k])=>!k.startsWith('_')&&!['zeit','name','ziel'].includes(k)).map(([k,v])=>`<p><strong>${esc(label(k))}</strong> ${esc(readable(v))}</p>`).join('')}</article>`).join(''):empty('Noch keine Einträge.');}
function renderPublic(){
 $('maintenance').textContent=world.maintenance||'';$('maintenance').hidden=!world.maintenance;
 const h=world.verlauf||[];setStableHTML($('public-history'), h.length?table(['Spielzeit','Aktive Reiche','Besiedelte Welten','Gesamtpunkte','Stufen I–V'],h.map(v=>[esc(`Tag ${Math.floor(v.sekunden/86400)+1}, ${Math.floor(v.sekunden%86400/3600)} Uhr`),v.reiche,v.kolonien,fmt(v.punkte),esc(v.stufen.join(' / '))])):empty('Die erste Entwicklungsübersicht entsteht nach einer Spielstunde. Die Rangliste ist bereits live.'));
}
function renderGovernment(){
 const n=view.naechste_stufe;
 setStableHTML($('government'), `<div class="row"><h3>Regierung</h3>${badge('Steuern '+view.steuersatz+' %')}</div><form id="tax-form"><label>Steuersatz (%)<input name="prozent" type="number" min="0" max="50" value="${view.steuersatz}" required></label><button>Steuersatz ändern</button></form>`+(n?`<h3>Aufstieg: ${esc(n.name)}</h3><ul class="conditions">${n.bedingungen.map(b=>`<li>${badge(b.erfuellt?'Erfüllt':'Offen',b.erfuellt?'good':'wait')} ${esc(kernLesbar(String(b.text||"")))}</li>`).join('')}</ul>${(n.hinweise||[]).length?`<p><b>Was als Nächstes hilft</b></p><ul class="conditions">${n.hinweise.map(x=>`<li>${esc(kernLesbar(String(x)))}</li>`).join('')}</ul>`:''}<p>${n.haltezeit_stunden>0?`Haltezeit: ${n.erfuellt_seit_stunden} / ${n.haltezeit_stunden} Spielstunden · `:'Aufstieg sofort möglich, sobald alles erfüllt ist · '}Kosten auf der Heimatwelt: ${esc(goods(n.kosten))}</p>`:empty('Die höchste Zivilisationsstufe ist erreicht.')));
 $('tax-form').onsubmit=guard(async e=>{await command([{typ:'steuersatz',prozent:Number(e.target.elements.prozent.value)}]);});
 const roles=view.agent?.roles||[];$('control-state').textContent=view.reich_status?.status==='besiegt'?'Dein Reich ist ausgeschieden. Du kannst seine Ansicht und Berichte weiter lesen.':roles.length?'Agent übernimmt: '+roles.map(label).join(', ')+'. Die übrigen Rollen steuerst du selbst.':'Du steuerst dein Reich. Kein Agent besitzt eine aktive Freigabe.';
}
function renderProducts(){
 const select=$('products'),previous=select.value,p=view.planeten.find(p=>p.koord===$('manufacture').elements.planet.value);
 const records=new Map([...itemRecords('ships',p),...itemRecords('defenses',p)].map(x=>[x.key,x]));
 const entries=view.einheiten_kosten.slice().sort((a,b)=>(rules.catalog.einheiten[a.einheit]?.regel.ab_stufe||1)-(rules.catalog.einheiten[b.einheit]?.regel.ab_stufe||1)).map(e=>{const needs=[...(e.werft?['werft '+e.werft]:[]),...(e.braucht||[])],ready=p&&!records.get(e.einheit)?.needs.length&&requirementsMet(p,needs);return `<option value="${esc(e.einheit)}" ${ready?'':'disabled'}>${esc(label(e.einheit))} · ${esc(goods(e.kosten_je_planet?.[p?.koord]||e.kosten))}${ready?'':' · benötigt '+esc(needs.join(', '))}</option>`;});
 for(const name of ['antriebskern','habitatmodul']){const orbital=(p?.gebaeude.orbitalwerft||0)>0,early=name==='habitatmodul'&&view.stufe>=3&&p?.gebaeude.werft>=3&&p?.schiffe.recycler>0;entries.push(`<option value="${name}" ${orbital||early?'':'disabled'}>${label(name)} · ${orbital?'Orbitalwerft':name==='habitatmodul'?'Werft 3 + Recycler + 400 Legierung + 250 Elektronik':'Orbitalwerft erforderlich'}</option>`);}
 setStableHTML(select, entries.join(''));if([...select.options].some(o=>o.value===previous&&!o.disabled))select.value=previous;
}
function renderReports(){
 if(window.Berichtsarchiv){Berichtsarchiv.render();return;}
 // Alle Berichtsarten (Karl 22.10.2026: „ohne Lücken“): auch Bodenkampf, Mondspionage und Phalanx; Spionage enthält Erkundungen.
 const groups=[['kampfberichte','Kampfberichte'],['berichte','Spionage & Erkundung'],['flottenberichte','Flottenaufklärung'],['bodenberichte','Bodenkampf'],['spionageberichte','Mondspionage','monde'],['phalanxberichte','Phalanxscans','monde'],['ereignisse','Aktuelle Ereignisse'],['chronik','Chronik'],['meldungen','Regierungsnotizen'],['erkundet','Erkundete Planeten']];
 // Offen sind Gruppen mit Einträgen (vorher stand die leere Flottenaufklärung offen, Spionage und Kampf zu); die Dossiers
 // tragen Name, Planet und Alter selbst – keine Kopfzeile „Koordinate · Sekunden“ mehr darüber (Emulator 20.10.2026).
 const dossier=['flottenberichte','berichte','kampfberichte','bodenberichte','spionageberichte','phalanxberichte'];
 // Neueste zuerst – Flottenberichte kamen nach Flotten-ID (älteste oben).
 const zeitVon=r=>typeof r.zeit==='number'?r.zeit:(typeof reportTime==='function'?reportTime(r):0),sortiert=(k,rows)=>k==='flottenberichte'||k==='bodenberichte'?rows.slice().sort((a,b)=>zeitVon(b)-zeitVon(a)):rows;
 // Kompakt (Karl 20.10.: „alles lückenlos“): Dossiers als Karte mit Motiv, Titel, Kurzinfo und Zeit – antippen klappt das volle
 // Dossier auf; Meldungen als eine Zeile mit Ereignismotiv; erkundete Planeten als Planetenkarten (Mobile035.reportKarte/erkundetKarten).
 const M=window.Mobile035,liste=(k,rows)=>`<ul class="rb-liste">${rows.map(r=>dossier.includes(k)?`<li><details class="rb-eintrag">${M.reportKarte(r,k)}<div class="rb-eintrag-inhalt">${M.report(r,k)}</div></details></li>`:`<li class="rb-eintrag rb-zeile">${M.reportKarte(r,k,true)}</li>`).join('')}</ul>`;
 setStableHTML($('reports'), groups.map(([k,t,eltern])=>{const rows=sortiert(k,(eltern?view[eltern]?.[k]:view[k])||[]);if(eltern&&!rows.length)return '';return `<details class="rb-gruppe" ${rows.length&&(dossier.includes(k)||k==='ereignisse'||(k==='erkundet'&&rows.some(r=>(r.alter_stunden||0)<24)))?'open':''}><summary>${esc(t)}<b>${rows.length}</b></summary>${!rows.length?empty('Noch keine Berichte.'):k==='erkundet'?M.erkundetKarten(rows):liste(k,rows)}</details>`;}).join(''));
}
// Vertragskarte in Spielerworten statt Feldnamen (Nachprüfung 25.10.2026: „Gut: erz · Menge Je Tag: 5“).
function vertragText(v){
 const teile=[`mit ${v.partner}`,v.status,v.kaution?`Kaution ${Number(v.kaution).toLocaleString('de-DE')} Credits`:''];
 const t=v.tribut;if(t)teile.push(`${t.zahler} zahlt ${Number(t.menge_je_tag||0).toLocaleString('de-DE')} ${label(t.gut)} je Tag${t.tage!=null?` · ${t.tage} ${Number(t.tage)===1?'Tag':'Tage'} lang`:t.endet?` · bis ${t.endet}`:''}`);
 return teile.filter(Boolean).join(' · ');
}
function renderDiplomacy(){
 const m=view.markt||{};
 setStableHTML($('market-state'), table(['Gut','Angebot ab','Nachfrage bis'],Object.entries(m.preise||{}).map(([k,v])=>[esc(label(k)),v.verkauf_ab!=null?Number(v.verkauf_ab).toLocaleString('de-DE',{maximumFractionDigits:3}):'Kein Angebot',v.kauf_bis!=null?Number(v.kauf_bis).toLocaleString('de-DE',{maximumFractionDigits:3}):'Keine Nachfrage']))+`<h3>Eigene Orders</h3>`+(m.orders?.length?table(['Nr.','Planet','Gut','Seite','Menge','Preis',''],m.orders.map(o=>[o.order,esc(o.planet),esc(label(o.gut)),esc(label(o.seite)),fmt(o.menge),Number(o.preis).toLocaleString('de-DE',{maximumFractionDigits:3}),`<button data-cancel-order="${esc(o.order)}" class="secondary">Stornieren</button>`])):empty('Keine offenen Marktorders.')));
 setStableHTML($('contracts'), view.vertraege?.length?view.vertraege.map(v=>`<article class="card"><h3>${esc(label(v.art))}</h3><p>${esc(vertragText(v))}</p><div class="row">${v.status==='angebot an dich'?(window.AntwortLeiste?AntwortLeiste('vertrag',v.vertrag,v.partner,label(v.art)):`<button data-contract="${esc(v.vertrag)}" data-contract-action="vertrag_annehmen">Annehmen</button><button class="secondary" data-contract="${esc(v.vertrag)}" data-contract-action="vertrag_ablehnen">Ablehnen</button>`):v.status==='aktiv'?`<button class="secondary" data-contract="${esc(v.vertrag)}" data-contract-action="vertrag_kuendigen">Kündigen</button>`:v.status==='von dir angeboten'?`<button class="secondary" data-contract="${esc(v.vertrag)}" data-contract-action="vertrag_ablehnen">Angebot zurückziehen${Number(v.kaution)>0?` · Kaution ${esc(fmt(v.kaution))} zurück`:''}</button>`:''}</div></article>`).join(''):empty('Keine Verträge.'));
 if(typeof renderCommunication==='function')renderCommunication();
 if(typeof renderPlayerHelp==='function')renderPlayerHelp();
}
function navigateGame(id){const fleetSection={kampf:'fleet-combat','expeditions-panel':'fleet-expeditions'}[id];if(fleetSection&&document.getElementById(fleetSection)){id='flotten';const panel=document.getElementById(fleetSection);panel.open=true;requestAnimationFrame(()=>panel.scrollIntoView({block:'start'}));}if(id==='missile-panel')id='verteidigung';/* Raketen liegen jetzt unter Verteidigung */if(!document.getElementById(id))id='reich';if(window.GalaxyUI)GalaxyUI.visible(id==='karte');document.querySelectorAll('#game > section').forEach(s=>s.hidden=s.id!==id);document.querySelectorAll('[data-tab]').forEach(t=>t.setAttribute('aria-selected',String(t.dataset.tab===id)));document.dispatchEvent(new Event('game-navigation'));}
function pickFleet(target,mission){
 navigateGame('flotten');if($('fleet-planner'))$('fleet-planner').open=true;const form=$('fleet');
 const candidates=view.planeten.filter(p=>['kolonisieren','kampfkolonisieren'].includes(mission)?(p.schiffe.kolonieschiff||p.schiffe.grosses_kolonieschiff):['spionage','system_erkunden'].includes(mission)?p.schiffe.spionagesonde:Object.entries(p.schiffe).some(([k,n])=>n>0&&(rules.catalog.einheiten[k]?.regel.angriff||0)>0));
 const p=candidates.find(p=>p.koord===selectedPlanet)||candidates[0]||currentPlanet();form.elements.start.value=p.koord;renderFleetStock();form.elements.ziel.value=target;form.elements.mission.value=mission;
 if($('return-fuel')){$('return-fuel').checked=true;syncReturnFuel();}for(const id of ['ship-inputs','cargo-inputs'])$(id).querySelectorAll('input').forEach(i=>i.value='0');
 document.querySelectorAll('[data-ground-load]').forEach(input=>input.value=0);
 const choose=(key,n)=>{const input=$('ship-inputs').querySelector(`[data-map="${key}"]`);if(input)input.value=n;};
 if(['spionage','system_erkunden'].includes(mission))choose('spionagesonde',p.schiffe.spionagesonde?1:0);
 if(['kolonisieren','kampfkolonisieren'].includes(mission)){
  const key=p.schiffe.kolonieschiff?'kolonieschiff':'grosses_kolonieschiff';choose(key,p.schiffe[key]?1:0);
  const escort=Object.keys(p.schiffe).find(k=>p.schiffe[k]>0&&!['kolonieschiff','grosses_kolonieschiff'].includes(k)&&(rules.catalog.einheiten[k]?.regel.angriff||0)>0);if(escort)choose(escort,1);
  if(mission==='kampfkolonisieren'){choose('truppentransporter',p.schiffe.truppentransporter?1:0);window.Mobile035?.renderGround();window.Mobile035?.schedulePreview();return;}
  const identity=planIdentity(),origin=form.elements.siedler_volk?.value||view.volk;
  api('/api/tool',{typ:'koloniebedarf',volk:origin,einheit:key}).then(q=>{if(identity!==planIdentity()||form.elements.ziel.value!==target||form.elements.mission.value!==mission)return;for(const input of $('cargo-inputs').querySelectorAll('input'))input.value=q.fracht?.[input.dataset.map]||0;window.Mobile035?.schedulePreview();}).catch(e=>show(e.message,true));
 }
 window.Mobile035?.schedulePreview();form.scrollIntoView?.({block:'start'});
}
function fuelReserve(q,legs){return Math.ceil((q.treibstoff_je_strecke_milli??q.treibstoff_je_strecke*1000)*legs/1000);}
function flightSummary(q,a){
 const hold=['saven','halten','abbau','blockade','expedition'].includes(a.mission)?a.haltedauer_stunden:0,oneway=typeof isOneWayMission==='function'?isOneWayMission(a.mission):['stationieren','kolonisieren'].includes(a.mission),legs=oneway?1:2,minutes=(q.dauer_sekunden??q.dauer_min*60)/60,gameMinutes=minutes*legs+hold*60;
 const cargo=Object.values(a.ladung).reduce((sum,n)=>sum+n,0),p=view.planeten.find(p=>p.koord===a.start),fuel=fuelReserve(q,legs);
 if($('fleet-live-summary'))setStableHTML($('fleet-live-summary'), `<span><b>${fmt(minutes)} Min.</b>Hinflug</span><span><b>${fmt(gameMinutes)} Min.</b>${oneway?'Ankunft':'Rückkehr'}</span><span><b>${fmt(fuel)}</b>Treibstoff</span><small>Spielzeit · ${fmt(minutes/(world.tempo||1))} Minuten Echtzeit hin</small>`);
 setStableHTML($('flight-quote'), `<h3>Flugplanung</h3><p>Das langsamste Schiff bestimmt die Reisezeit. Reduziertes Tempo senkt den Verbrauch; schnellere Schiffe drosseln im Verband. Antriebsforschung spart bis zu 40 % Treibstoff, Raumhäfen reduzieren den Startverbrauch zusätzlich.</p><div class="stats">${stat(fmt(minutes)+' Min.','Hinflug (Spielzeit)')}${stat(fmt(gameMinutes)+' Min.',oneway?'Ankunft nach Start':'Planmäßige Rückkehr nach Start')}${stat(fuel,oneway?'Treibstoffreserve für Hinflug':'Treibstoffreserve für beide Strecken')}${stat(cargo+' / '+q.ladekapazitaet,'Fracht / Laderaum')}</div><p>Bei ${world.tempo}×: ca. ${fmt(gameMinutes/world.tempo)} Minuten Echtzeit. ${hold?'Wartezeit: '+hold+' Spielstunden.':''} ${oneway?'Einwegauftrag. Erfolg und Verbleib werden bei Ankunft geprüft.':'Kampf, Blockaden und Rückruf können den Verlauf ändern.'}</p><p>Auf dem Planeten bleiben: ${esc(goods(Object.fromEntries(Object.entries(p.bestand).map(([k,n])=>[k,Math.max(0,n-(a.ladung[k]||0)-(k==='deuterium'?fuel:0))]))))}</p>`);
}
function selectAllShips(){const p=view.planeten.find(p=>p.koord===$('fleet').elements.start.value);if(!p)return;$('ship-inputs').querySelectorAll('input').forEach(i=>i.value=p.schiffe[i.dataset.map]||0);window.Mobile035?.renderShipSelection(p);window.Mobile035?.schedulePreview();}
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
