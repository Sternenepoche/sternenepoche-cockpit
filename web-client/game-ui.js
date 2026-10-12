'use strict';
// Pure timing helpers. Only server snapshots confirm completion.
const GameTime={
  anchor:{seconds:0,stamp:0,speed:0,connected:false},
  sync(w,stamp=performance.now()){this.anchor={seconds:w.sekunden,stamp,speed:w.paused||w.beendet?0:w.tempo,connected:true};},
  now(stamp=performance.now()){const a=this.anchor;return a.seconds+(a.connected?Math.min(15,Math.max(0,(stamp-a.stamp)/1000))*a.speed:0);},
  disconnect(stamp=performance.now()){this.anchor.seconds=this.now(stamp);this.anchor.stamp=stamp;this.anchor.connected=false;},
  job(j,seconds,snapshot){
    if(j.wartet||j.fertig_sekunden==null)return {progress:0,remaining:null};
    const remaining=Math.max(0,j.fertig_sekunden-(j.pausiert?snapshot:seconds));
    if(j.punkte_gesamt){
      const start=Math.max(j.begonnen_sekunden??j.naechster_tick_sekunden-3600,j.naechster_tick_sekunden-3600);
      const tickFraction=Math.min(1,Math.max(0,(seconds-start)/Math.max(1,j.naechster_tick_sekunden-start)));
      const credited=Math.min(j.punkte_rest,Math.max(0,j.rate||0)*tickFraction);
      return {progress:Math.min(1,Math.max(0,1-(j.punkte_rest-credited)/j.punkte_gesamt)),remaining};
    }
    return {progress:Math.min(1,Math.max(0,1-remaining/Math.max(1,j.dauer_sekunden))),remaining};
  },
  duration(s){if(s==null||!Number.isFinite(s))return 'Wartet';s=Math.max(0,Math.ceil(s));const h=Math.floor(s/3600),m=Math.floor(s%3600/60);return `${String(h).padStart(2,'0')}:${String(m).padStart(2,'0')}:${String(s%60).padStart(2,'0')}`;}
};
// DOM bindings below. GameTime is also exercised without a browser in UI tests.
document.body.dataset.progressMotion=localStorage.getItem('sternenepoche-progress-motion')||'system';
let selectedPlanet='',chosenRace='aurelianer',uiSelections={},uiJobs=new Map(),uiReady=false,uiWorldKey='',uiViewOwner='',uiLastReportStamp='';
const named=k=>window.STERNEN_ART?.names[k]||label(k);
const artAliases={buildings:{geheimdienst:'sensorphalanx'},research:{ueberwachungstechnik:'spionagetechnik',abschirmtechnik:'schildtechnik'},missions:{system_erkunden:'spionage',flotten_spionage:'spionage',saven:'transport'}};
function art(category,key,faction=view?.volk||''){
 const images=window.STERNEN_ART?.images||{},alias=artAliases[category]?.[key]||key;
 return images[`${category}|${alias}|${faction}`]?.url||images[`${category}|${alias}|`]?.url||images['backgrounds|stars|']?.url||'';
}
function imageTag(category,key,faction,cls=''){return `<img class="${cls}" src="${esc(art(category,key,faction))}" alt="${esc(named(key))}" loading="lazy">`;}
function planetArt(p){return art('planets',p.zone==='glut'?'glut_basalt':p.zone==='frost'?'frost_eis':p.zone==='nebel'?'nebel_xeno':'leben_kontinent','');}
function currentPlanet(){return view?.planeten.find(p=>p.koord===selectedPlanet)||view?.planeten[0];}
function uiWorldSync(){if(world)GameTime.sync(world);}
function uiConnectionLost(){GameTime.disconnect();tickGame();}
function costHtml(cost,stock){return `<div class="costs">${Object.entries(cost||{}).filter(([,n])=>n>0).map(([g,n])=>`<span class="cost ${stock&&(stock[g]||0)<n?'short':''}" title="${esc(named(g))}${stock?' · vorhanden '+fmt(stock[g]||0):''}">${imageTag('resources',g,'')}${fmt(n)}</span>`).join('')}</div>`;}
function keepJob(job){const key=[job.gebaeude||job.forschung||job.produkt||'job',job.stufe||0,job.fertig_sekunden??'wait',job.dauer_sekunden||job.punkte_gesamt||0].join(':');uiJobs.set(key,job);return key;}
function artShell(category,key,job,faction){
 const src=esc(art(category,key,faction));if(!job)return `<div class="art-shell"><img class="art-img" src="${src}" alt="${esc(named(key))}" loading="lazy"></div>`;
 const id=keepJob(job);return `<div class="art-shell running" data-job="${esc(id)}" role="progressbar" aria-label="${esc(named(key))} · Fortschritt" aria-valuemin="0" aria-valuemax="100"><img class="art-img" src="${src}" alt="${esc(named(key))}" loading="lazy"><img class="art-color" src="${src}" alt="" loading="lazy"><span class="clock-hand" aria-hidden="true"></span><span class="timer-label">Synchronisiert …</span></div>`;
}
function jobLine(job){const id=keepJob(job);return `<div data-job="${esc(id)}" class="queue-bottom"><span class="job-time"></span><div class="job-meter"><span class="job-fill"></span></div></div>`;}
function tickGame(){
 const now=GameTime.now(),snapshot=view?.sekunden??GameTime.anchor.seconds,paused=world?.paused||world?.beendet;
 if(world){$('world-time').textContent=`Tag ${Math.floor(now/86400)+1} · ${GameTime.duration(now%86400)}${paused?' · Pause':''}`;}
 document.querySelectorAll('[data-job]').forEach(el=>{
   const j=uiJobs.get(el.dataset.job);if(!j)return;const r=GameTime.job(j,now,snapshot),percent=Math.round(r.progress*100);
   const text=j.wartet?'In Warteschlange':j.fertig_sekunden==null?'Keine Forschungsleistung':r.remaining===0?'100 % · Serverbestätigung …':`${GameTime.duration(r.remaining)} · ${percent} %${j.pausiert?' · Unruhen':paused?' · Pause':!GameTime.anchor.connected?' · Verbindung fehlt':''}`;
   el.style.setProperty('--angle',`${r.progress*360}deg`);el.style.setProperty('--turn',`${r.progress*360+180}deg`);el.style.setProperty('--progress',r.progress);
   if(el.classList.contains('art-shell')){el.setAttribute('aria-valuenow',percent);el.setAttribute('aria-valuetext',text);el.querySelector('.timer-label').textContent=text;}
   else{el.querySelector('.job-time').textContent=text;el.querySelector('.job-fill').style.transform=`scaleX(${r.progress})`;}
 });
 document.querySelectorAll('[data-arrival]').forEach(el=>el.textContent=GameTime.duration(Math.max(0,Number(el.dataset.arrival)-now))+(paused?' · Pause':''));
}
function factionTraits(name){
 const c=rules?.catalog,v=c?.voelker?.[name];if(!v)return {pros:[],cons:[],special:['Regeln werden geladen …']};
 const pros=[],cons=[],special=[];
 const fields=[['ladung','Laderaum',1],['marktgebuehr','Marktgebühren',-1],['waffen','Waffenstärke',1],['panzerung','Strukturpunkte',1],['steuer','Steuereinnahmen',1],['werftzeit','Fertigungsdauer',-1],['forschung','Forschungsleistung',1],['wachstum','Bevölkerungswachstum',1],['nahrung','Nahrungsproduktion',1],['energie','Energieproduktion',1],['kolonieschiff','Kolonieschiffkosten',-1]];
 for(const[k,title,good]of fields){const delta=Math.round(((v[k]??1)-1)*100);if(delta)(delta*good>0?pros:cons).push(`${delta>0?'+':''}${delta} % ${title}`);}
 if(v.pluenderquote!=null&&v.pluenderquote!==c.kampf.pluenderquote){const delta=v.pluenderquote-c.kampf.pluenderquote;(delta>0?pros:cons).push(`${fmt(v.pluenderquote*100)} % Plünderquote (Standard ${fmt(c.kampf.pluenderquote*100)} %)`);}
 if(v.ohne_nahrung){pros.push('Bevölkerung benötigt keine Nahrung');cons.push(`${fmt(c.wirtschaft.energie_je_1000_syntheten)} Energie / 1.000 Einwohner zusätzlich`);}
 if(name==='veyari')special.push('Spionage +1 nach Erforschung von Spionagetechnik. Geheimdienst und Sonden bleiben nötig.');
 if(name==='syntheten')special.push('Sensorstärke +1 nach Erforschung der Überwachungstechniken. Geheimdienst muss ausgebaut sein.');
 return {pros,cons,special};
}
function factionCards(target,locked=false){
 const titles={aurelianer:['Handel & Logistik','Wirtschaft durch Austausch und große Frachträume.'],krath:['Flotten & Eroberung','Starke Waffen und schnelle Schiffsfertigung.'],veyari:['Wachstum & Aufklärung','Lebendige Kolonien und geschulte Spionage.'],syntheten:['Technik & Sensoren','Forschungsstärke mit eigener Energieversorgung.']};
 $(target).innerHTML=Object.entries(titles).map(([name,[title,desc]])=>{const t=factionTraits(name);return `<button type="button" class="faction-card" data-race="${name}" aria-pressed="${chosenRace===name}" ${locked?'disabled':''}>${imageTag('portraits',name,name)}<span class="faction-copy"><strong>${named(name)}</strong><small>${title} · ${desc}</small><b class="trait-heading">Vorteile</b>${t.pros.map(x=>`<span class="plus">${esc(x)}</span>`).join('')}<b class="trait-heading">Nachteile</b>${t.cons.map(x=>`<span class="minus">${esc(x)}</span>`).join('')}${t.special.map(x=>`<span class="special">${esc(x)}</span>`).join('')}</span></button>`;}).join('');
}
function renderShell(){
 if(!rules||!world)return;
 const guideKey=JSON.stringify([rules.catalog,world.ausscheiden_regeln]);if(uiReady!==guideKey){uiReady=guideKey;renderGuide();}
 const key=world.world_id;if(uiWorldKey!==key){uiWorldKey=key;selectedPlanet='';uiSelections={};uiViewOwner='';}
 $('account-strip').textContent=session?`${session.name} · ${session.spieler==null?'Beobachter':named(session.mode)}`:'Beobachter · Alpha-Universum';
 const claimed=session?.spieler!=null;$('claim').elements.volk.disabled=claimed;
 if(session?.warteliste&&!document.activeElement?.closest('#claim'))chosenRace=session.warteliste.volk;
 if(claimed)chosenRace=view?.volk||chosenRace;
 $('race-fixed').textContent=claimed?'Dein Volk steht für diese Epoche fest. Die Spielweise und Agentenrollen kannst du jederzeit ändern.':'Deine Volkswahl wird bei der Aufnahme in die Spielwelt verbindlich. Auf der Warteliste kannst du sie noch ändern.';
 // Do not rebuild the selection cards under a pointer or focused button every poll.
 const stamp=JSON.stringify([rules.catalog?.voelker,chosenRace,claimed]);
 for(const id of ['volk-preview','volk-choice'])if($(id).dataset.stamp!==stamp){factionCards(id,id==='volk-choice'&&claimed);$(id).dataset.stamp=stamp;}
 $('resource-bar').hidden=!claimed;$('event-bar').hidden=!claimed;
 if(window.GalaxyUI)GalaxyUI.visible(claimed&&document.querySelector('[data-tab="karte"]')?.getAttribute('aria-selected')==='true');
 if(!claimed){$('planet-sidebar').innerHTML='';ResourceDisplay.reset();return;}
 $('claim').elements.volk.value=chosenRace;
}
function queueCard(title,category,job,rest=[],destination='gebaeude'){
 const key=job?.gebaeude||job?.forschung||job?.produkt;
 return `<article class="queue-card"><strong>${title}</strong>${job?artShell(category,key,job):`<p class="empty">Kein laufender Auftrag</p>`}${job?`<p>${esc(named(key))}${job.stufe?' · Stufe '+job.stufe:job.rest?' · noch '+job.rest+' Stück':''}</p>${jobLine(job)}`:''}${rest.length?`<ol class="queue-list">${rest.map(j=>`<li>${esc(named(j.gebaeude||j.forschung||j.produkt||j))}${j.stufe?' '+j.stufe:''} · Wartet</li>`).join('')}</ol>`:''}<button class="secondary" type="button" data-go="${destination}">Öffnen</button></article>`;
}
function renderView(){
 const p=currentPlanet();if(!p)return;
 if(uiViewOwner!==`${world.world_id}:${view.name}`){uiViewOwner=`${world.world_id}:${view.name}`;selectedPlanet=p.koord;uiSelections={};}
 selectedPlanet=p.koord;chosenRace=view.volk;document.body.dataset.reichStatus=view.reich_status?.status||'aktiv';GameTime.sync({...world,sekunden:view.sekunden});
 $('empire-name').textContent=`${view.name} · ${named(view.volk)}`;
 $('empire-summary').innerHTML=stat(view.stufe,'Zivilisationsstufe')+stat(fmt(view.punkte.gesamt),'Imperiumspunkte')+stat(`${view.flottenplaetze.belegt} / ${view.flottenplaetze.gesamt}`,'Flottenslots')+stat(`${view.kolonien.anzahl} / ${view.kolonien.erlaubt}`,'Kolonien');
 $('warnings').innerHTML=reichNotice()+view.warnungen.map(w=>`<div class="warning">${esc(w)}</div>`).join('');
 const energy=p.energie.erzeugung-p.energie.verbrauch;
 ResourceDisplay.render(p,view,world);
 $('event-bar').classList.toggle('attack',view.angriffe.length>0||view.reich_status?.status==='kritisch');
 $('event-bar').innerHTML=`<span>${esc(p.koord)} · ${p.heimat?'Heimatplanet':'Kolonie'}</span><span>Eigene Flotten: ${view.flotten.length}</span><span>${view.angriffe.length?'⚠ '+view.angriffe.length+' erkannte Angriffe':'Keine erkannten Angriffe'}</span><span>${world.beendet?'Epoche beendet':world.paused?'Welt pausiert':world.tempo+'× Spieltempo'}</span>${view.reich_status?.status==='besiegt'?'<strong>BESIEGT · Nur noch zuschauen</strong>':view.reich_status?.status==='kritisch'?'<strong>⚠ Existenzielle Krise</strong><span>Rettungsfrist: <span data-arrival="'+Math.min(...view.reich_status.rettungsfristen.map(k=>k.frist))+'"></span> Spielzeit</span>':''}${view.anfaengerschutz_bis?'<span>Anfängerschutz: '+esc(view.anfaengerschutz_bis)+'</span>':''}`;
 $('planet-sidebar').innerHTML='<h3>DEINE PLANETEN</h3>'+view.planeten.map(x=>`<button type="button" class="planet-pick" data-planet-pick="${esc(x.koord)}" aria-pressed="${x.koord===p.koord}"><img src="${esc(planetArt(x))}" alt="${esc(named(x.zone))}" loading="lazy"><strong>${esc(x.koord)}</strong><small>${x.heimat?'Heimatwelt':esc(named(x.zone))}</small></button>`).join('');
 // Preserve choices and typing in the actual fleet, market and manufacturing forms.
 document.querySelectorAll('.own-planets').forEach(s=>{const val=s.value,markup=view.planeten.map(x=>`<option>${esc(x.koord)}</option>`).join('');if(s.innerHTML!==markup){s.innerHTML=markup;if(view.planeten.some(x=>x.koord===val))s.value=val;}});
 $('planets').innerHTML=`<div class="hero-planet" data-ai-image><img class="hero-orb" src="${esc(planetArt(p))}" alt="${esc(named(p.zone))} · ${esc(p.koord)}"><div class="hero-info"><span class="eyebrow">${p.heimat?'DEINE HEIMAT ZWISCHEN DEN STERNEN':'KOLONIE DEINES IMPERIUMS'}</span><h3>${esc(p.koord)}</h3><span>${esc(named(p.zone))} · ${esc(named(view.volk))}</span><dl><dt>Baufelder</dt><dd>${p.felder.belegt} / ${p.felder.gesamt}</dd><dt>Bevölkerung</dt><dd>${fmt(p.bevoelkerung)}</dd><dt>Wohnraum</dt><dd>${fmt(p.wohnraum)}</dd><dt>Stabilität</dt><dd>${p.stabilitaet} %</dd><dt>Energiebilanz</dt><dd>${fmt(energy)}</dd><dt>Arbeitskräfte</dt><dd>${fmt(p.arbeit.verfuegbar)} / ${fmt(p.arbeit.bedarf)}</dd><dt>Fachkräfte</dt><dd>${fmt(p.fachkraefte.verfuegbar)} / ${fmt(p.fachkraefte.bedarf)}</dd><dt>Blockade</dt><dd>${p.blockade?esc(p.blockade.durch):'Keine'}</dd></dl></div></div><div class="queue-grid">${queueCard('BAUSCHLEIFE','buildings',p.bauschleife[0],p.bauschleife.slice(1),'gebaeude')}${queueCard('FORSCHUNG','research',view.forschung.aktiv,view.forschung.schlange,'forschen')}${queueCard('SCHIFFSWERFT','ships',p.fertigung.find(f=>f.schleife==='werft'),p.fertigung.filter(f=>f.schleife==='werft').slice(1),'werft')}</div><div class="section-actions"><button data-go="gebaeude">Gebäude ausbauen</button><button class="secondary" data-go="flotten">Flotten verwalten</button><button class="secondary" data-go="berichte">Neue Berichte</button></div>`;
 renderCatalog('buildings','gebaeude-content');renderCatalog('research','research');renderCatalog('ships','ship-gallery');renderCatalog('defenses','verteidigung-content');
 renderFleetStock();renderEconomy(p);renderFleets();renderColonies(p);renderCombat();renderEmpire();renderUnlocks();renderProfile();
 // Existing account, agent, market and diplomacy controls keep their authoritative handlers.
 const govFocus=document.activeElement?.closest('#government');if(!govFocus)renderGovernment();
 renderProducts();renderProduction();
 const reportStamp=JSON.stringify([view.ereignisse,view.chronik,view.berichte,view.flottenberichte,view.kampfberichte,view.nachrichten]);
 if(uiLastReportStamp!==reportStamp){uiLastReportStamp=reportStamp;renderReports();}
 if(!document.activeElement?.closest('#human-diplomacy'))renderDiplomacy();
 const activeJobKeys=new Set([...document.querySelectorAll('[data-job]')].map(e=>e.dataset.job));for(const key of uiJobs.keys())if(!activeJobKeys.has(key))uiJobs.delete(key);tickGame();renderShell();applyControlState();
}
function itemRecords(category,p=currentPlanet()){
 const c=rules.catalog;if(!c)return [];
 if(category==='buildings')return Object.entries(c.gebaeude).map(([key,r])=>{const b=p.baubar.find(b=>b.gebaeude===key),level=p.gebaeude[key]||0;const job=p.bauschleife.find(j=>j.gebaeude===key);const needs=[...(r.ab_stufe>view.stufe?['Zivilisationsstufe '+r.ab_stufe]:[]),...(!b?Object.entries(r.braucht).filter(([k,n])=>(p.gebaeude[k]||0)<n).map(([k,n])=>`${named(k)} ${n}`):b.braucht.map(named)),...(key==='xenoextraktor'&&!p.nebel?['Nebelsystem']:[])];
   if(!b&&!needs.length)needs.push('Maximalstufe erreicht');if(p.bauschleife.length>=p.bauschleife_plaetze)needs.push('Bauschleife voll');if(p.felder.belegt>=p.felder.gesamt)needs.push('Keine freien Baufelder');
   if(p.stabilitaet<c.wirtschaft.unruhen_unter&&!p.bauschleife.length)needs.push('Unruhen: zu geringe Stabilität');
   return {key,r,level,job,cost:b?.kosten||Object.fromEntries(Object.entries(r.kosten).map(([g,n])=>[g,Math.floor(n*r.faktor**level)])),needs,missing:b?.fehlt||[],duration:b?.bauzeit_sekunden??b?.bauzeit_min*60,action:`data-build="${esc(key)}" data-planet="${esc(p.koord)}"`,verb:'Stufe '+(b?.stufe||level+1)+' ausbauen',effect:b?.ertrag?`+${fmt(b.ertrag.plus)} ${b.ertrag.art}`:''};});
 if(category==='research')return Object.entries(c.forschung).map(([key,r])=>{const f=view.forschung.moeglich.find(f=>f.forschung===key),level=view.forschung.stufen[key]||0,home=view.planeten.find(x=>x.heimat)||view.planeten[0];const needs=[];if(r.ab_stufe>view.stufe)needs.push('Zivilisationsstufe '+r.ab_stufe);if((home.gebaeude.labor||0)<r.labor)needs.push('Labor '+r.labor+' auf der Heimatwelt');if(['ueberwachungstechnik','abschirmtechnik'].includes(key)&&!(view.forschung.stufen.spionagetechnik>0))needs.push('Spionagetechnik 1');if(key==='ueberwachungstechnik'&&!view.planeten.some(x=>x.gebaeude.geheimdienst>0))needs.push('Geheimdienst 1');if(view.forschung.schlange.length>=c.wirtschaft.forschung_warteschlange)needs.push('Forschungsschlange voll');return {key,r,level,job:view.forschung.aktiv?.forschung===key?view.forschung.aktiv:view.forschung.schlange.includes(key)?{wartet:true}:null,cost:f?.kosten||Object.fromEntries(Object.entries(r.kosten).map(([g,n])=>[g,Math.floor(n*r.faktor**level)])),stock:home.bestand,needs,missing:f?.fehlt||[],duration:f?.dauer_stunden*3600,action:`data-research="${esc(key)}"`,verb:'Stufe '+(f?.stufe||level+1)+' erforschen',effect:fmt(view.forschung.punkte_je_stunde)+' FP / Spielstunde'};});
 return Object.entries(c.einheiten).filter(([,e])=>e.schiff===(category==='ships')).map(([key,e])=>{const r=e.regel,q=view.einheiten_kosten.find(x=>x.einheit===key),needs=[];if(r.ab_stufe>view.stufe)needs.push('Zivilisationsstufe '+r.ab_stufe);if((p.gebaeude.werft||0)<r.werft)needs.push('Werft '+r.werft);for(const[k,n]of Object.entries(r.braucht))if((p.gebaeude[k]||0)<n)needs.push(named(k)+' '+n);if(r.max_anzahl>0&&(p.verteidigung[key]||0)>=r.max_anzahl)needs.push('Höchstbestand erreicht');
   const cost=q?.kosten||e.kosten_je_volk?.[view.volk]||r.kosten,missing=Object.entries(cost).filter(([g,n])=>(p.bestand[g]||0)<n).map(([g])=>g);return {key,r,level:p.schiffe[key]||p.verteidigung[key]||0,job:p.fertigung.find(j=>j.produkt===key),cost,needs,missing,duration:q?.bauzeit_je_planet_sekunden?.[p.koord],action:`data-make="${esc(key)}"`,verb:'Fertigen',effect:`Angriff ${fmt(r.angriff)} · Schild ${fmt(r.schild)} · Struktur ${fmt(r.struktur)}${r.ladung?' · Laderaum '+fmt(r.ladung):''}`};});
}
function renderCatalog(category,target){
 if(document.activeElement?.matches('input,select,textarea')&&$(target).contains(document.activeElement))return;
 const p=currentPlanet(),items=itemRecords(category,p);if(!items.length)return;
 let key=uiSelections[category];if(!items.some(i=>i.key===key))key=items.find(i=>i.job)?.key||items.find(i=>!i.needs.length)?.key||items[0].key;uiSelections[category]=key;
 const i=items.find(i=>i.key===key),blocked=view.reich_status?.status==='besiegt'||i.needs.length>0||i.missing.length>0;
 const qty=['ships','defenses'].includes(category),stock=i.stock||p.bestand;
 const detail=`<article class="item-detail">${artShell(category,i.key,i.job)}<div><span class="eyebrow">${esc(p.koord)} · ${qty?'BESTAND '+i.level:'STUFE '+i.level}</span><h3>${esc(named(i.key))}</h3><p>${esc(i.r.wirkung)}</p>${i.effect?`<p class="detail-meta">${esc(i.effect)}</p>`:''}${costHtml(i.cost,stock)}<p class="detail-meta">${i.duration?`Bauzeit: ${GameTime.duration(i.duration)} Spielzeit · ${GameTime.duration(i.duration/(world.tempo||1))} bei ${world.tempo}×`:qty?'Stückkosten. Die Werftquote wird vor dem Fertigen geprüft.':'Dauer erst mit erfüllten Voraussetzungen bestimmbar.'}</p>${i.needs.length?`<p class="requirement-list">Benötigt: ${esc(i.needs.join(' · '))}</p>`:''}${i.missing.length?`<p class="requirement-list">Rohstoffe fehlen: ${esc(i.missing.map(named).join(', '))}</p>`:''}<div class="section-actions">${qty?`<label>Stückzahl<input id="quantity-${category}" type="number" min="1" max="10000" value="${esc(uiSelections[category+'Qty']||1)}"></label>`:''}<button type="button" ${i.action} ${blocked?'disabled':''}>${i.verb}</button>${category==='buildings'&&p.gebaeude_integritaet_prozent?.[i.key]<100?`<button type="button" class="secondary" data-repair="${esc(i.key)}">Reparatur prüfen</button>`:''}</div></div></article>`;
 $(target).innerHTML=detail+`<div class="gallery" aria-label="${category==='buildings'?'Gebäude':category==='research'?'Forschungen':category==='ships'?'Schiffe':'Verteidigungsanlagen'}">${items.map(x=>`<button type="button" class="item-tile ${x.needs.length?'locked':''} ${x.key===key?'selected':''}" data-item="${esc(x.key)}" data-category="${category}" aria-pressed="${x.key===key}" title="${esc(named(x.key)+(x.needs.length?' · '+x.needs.join(', '):x.missing.length?' · Rohstoffe fehlen':' · Verfügbar'))}">${artShell(category,x.key,x.job)}<span class="tile-copy"><strong>${esc(named(x.key))}</strong><small>${qty?'Bestand':'Stufe'} ${x.level}${x.needs.length?' · Gesperrt':x.job?' · In Arbeit':x.missing.length?' · Rohstoffe fehlen':' · Verfügbar'}</small></span></button>`).join('')}</div>`;
 if(category==='research')$(target).insertAdjacentHTML('beforeend',`<h3>Forschungsschlange</h3>${queueCard('AKTIVES PROJEKT','research',view.forschung.aktiv,view.forschung.schlange,'forschen')}<p class="muted">Forschung wird an vollen Spielstunden verrechnet. Restzeit und Fortschritt sind eine Prognose bei unveränderter Laborleistung.</p>`);
}
function renderProduction(){const p=currentPlanet();$('production').innerHTML=`<h3>Fertigungsaufträge · ${esc(p.koord)}</h3>`+(p.fertigung.length?p.fertigung.map(j=>`<article class="fleet-card"><strong>${esc(named(j.produkt))} · noch ${j.rest} Stück</strong><p class="detail-meta">${esc(named(j.schleife))} · Fortschritt des nächsten Stücks</p>${jobLine(j)}</article>`).join(''):empty('Beide Fertigungsschleifen sind frei.'))+`<h3>Orbitalbauteile</h3><p class="muted">Antriebskerne und Habitatmodule werden in der Orbitalwerft gefertigt und für Kolonieschiffe benötigt. Nutze den Fertigungsauftrag oben.</p>`;}
function renderEconomy(p){
 if(document.activeElement?.closest('#priority-form'))return;
 $('kolonie-content').innerHTML=`<p>${esc(p.koord)} · Vorräte, Produktion und Versorgung</p><div class="table-wrap">${table(['Rohstoff','Bestand','Lager','Pro Spielstunde','Reichweite / Lager voll'],Object.keys(p.bestand).map(g=>[imageTag('resources',g,'','planet-row-icon')+esc(named(g)),fmt(p.bestand[g]),fmt(p.lager[g]),`${p.rate[g]>=0?'+':''}${fmt(p.rate[g])}`,p.rate[g]<0?GameTime.duration(p.vorrat_reicht_sekunden?.[g]):p.voll_in_stunden[g]!=null?fmt(p.voll_in_stunden[g])+' Spielstunden':'—']))}</div><div class="stats">${stat(p.nahrung_deckung+' %','Nahrungsdeckung')}${stat(p.konsum_deckung+' %','Konsumdeckung')}${stat(p.stabilitaet_ziel+' %','Stabilitätsziel')}</div><p class="muted">Reichweite bei konstanten Raten. Wachstum, Bauten, Lieferungen und Kämpfe ändern diese Prognose.</p><h3>Arbeitsprioritäten</h3><p class="muted">Die zuerst genannten Gebäude werden zuerst mit Arbeitskräften versorgt.</p><form id="priority-form"><label>Gebäude (durch Komma getrennt)<input name="reihenfolge" value="${esc(p.prioritaeten.join(', '))}" required></label><button>Prioritäten speichern</button></form>`;
}
function renderFleets(){
 $('fleets').innerHTML='<h3>Eigene Flotten</h3>'+(view.flotten.length?view.flotten.map(f=>{const end=f.zustand==='hinflug'?f.ankunft_sekunden:f.rueckkehr_sekunden;return `<article class="fleet-card"><div class="fleet-route"><strong>#${f.flotte} · ${esc(named(f.mission))}</strong><span>${esc(f.start)} → ${esc(f.ziel)}</span></div><p>${esc(named(f.zustand))} · ${esc(f.bis)} ${end!=null?`· <span data-arrival="${esc(end)}"></span> Spielzeit`:''}</p><p class="fleet-data">Schiffe: ${esc(goods(f.schiffe))}<br>Fracht: ${esc(goods(f.ladung))}${f.verband?' · Verband #'+f.verband:''}</p><div class="fleet-actions"><button class="secondary" data-recall="${esc(f.flotte)}">Rückrufen</button>${f.mission==='angriff'&&!f.verband?`<button class="secondary" data-open-group="${esc(f.flotte)}">Verband eröffnen</button>`:''}</div></article>`;}).join(''):empty('Keine eigene Flotte unterwegs.'));
 $('attacks').innerHTML='<h3>Erkannte Angriffe</h3>'+(view.angriffe.length?view.angriffe.map(f=>`<article class="fleet-card warning"><strong>⚠ Kontakt #${f.flotte} → ${esc(f.ziel)}</strong><p>Ankunft: ${esc(f.ankunft)} · <span data-arrival="${esc(f.ankunft_sekunden)}"></span> Spielzeit</p><p>Absender: ${esc(f.von??'Unbekannt')}<br>Schiffe: ${f.schiffe!=null?fmt(f.schiffe):f.schiffe_spanne?esc(f.schiffe_spanne.join('–')):'Unbekannt'} · Typen: ${f.schiffstypen?esc(goods(f.schiffstypen)):'Unbekannt'}</p><button data-probe="${esc(f.flotte)}">Flottensonde senden</button><button class="secondary" data-save-target="${esc(f.ziel)}">Schiffe & Rohstoffe saven</button></article>`).join(''):empty('Kein Angriff erfasst. Ohne ausgebauten Geheimdienst liegt die Warnschwelle bei zwei Spielstunden; außerhalb der Sensorreichweite ist eine Flotte unbekannt.'));
}
function reichNotice(){
 if(world.beendet)return '<div class="defeat-notice" role="status"><strong>EPOCHE BEENDET</strong><p>Die Abschlussrangliste steht fest. Du kannst dein Reich und seine Berichte weiter ansehen. Neue Befehle sind erst in der nächsten Epoche möglich.</p></div>';
 if(world.paused)return '<div class="warning" role="status"><strong>WELT PAUSIERT</strong><p>Spielzeit und Aufträge stehen still. Ansichten, Flugplanung und Berichte bleiben verfügbar.</p></div>';
 const state=view.reich_status;if(!state)return '';
 if(state.status==='besiegt')return `<div class="defeat-notice" role="status"><strong>BESIEGT · Für diese Epoche ausgeschieden</strong><p>${esc(state.grund)}. Dein Konto bleibt zum Zuschauen erhalten. Heimatwelt und Spielerplatz bleiben bis zum Epochenwechsel geschützt und belegt.</p></div>`;
 return (state.rettungsfristen||[]).map(k=>`<div class="warning"><strong>Existenzielle Krise: ${esc(k.grund)}</strong><p>Rettungsfrist: <span data-arrival="${k.frist}"></span> Spielzeit. Versorgung und Wiederaufbau jetzt sichern. Eine Erholung beendet diese Krise.</p></div>`).join('');
}
function renderFleetStock(){
 const p=view.planeten.find(x=>x.koord===$('fleet').elements.start.value);if(!p)return;
 $('ship-inputs').querySelectorAll('input').forEach(i=>{let note=i.parentElement.querySelector('.fleet-stock');if(!note){note=document.createElement('small');note.className='fleet-stock';note.id='fleet-stock-'+i.dataset.map;i.before(note);i.setAttribute('aria-describedby',note.id);i.setAttribute('aria-label',named(i.dataset.map));}const n=p.schiffe[i.dataset.map]||0;note.textContent='Vorhanden: '+fmt(n);i.max=String(n);});
}
function planetResources(p){
 const own=view.planeten.find(x=>x.koord===p.koord);
 if(own)return esc(Object.entries(own.faktoren).map(([g,n])=>named(g)+' '+fmt(n/10)+' %').join(' · '));
 if(!p.ertrag_promille)return 'Unbekannt';
 const factors=Object.entries(p.ertrag_promille).map(([g,n])=>named(g)+' '+n.map(v=>fmt(v/10)).join('–')+' %').join(' · ');
 const stock=Object.entries(p.bestand_spannen||{}).map(([g,n])=>named(g)+' '+n.map(fmt).join('–')).join(' · ');
 return esc(factors)+`<small class="map-observation">Bericht ${fmt(p.alter_stunden||0)} Spielstunden alt${stock?' · Vorräte: '+esc(stock):''}</small>`;
}
function galaxyReports(data){
 const systems=Array.isArray(data)?data:data.systeme||[];
 return systems.map(s=>`<article class="card"><div class="row"><h3>System ${esc(s.sektor??$('map-form').elements.sektor.value)}:${esc(s.system)}</h3>${badge(s.bekannt?'Kartiert':'Systemscan fehlt',s.bekannt?'good':'wait')}</div><p class="muted">Nebel: ${s.nebel==null?'Unbekannt':s.nebel?'Ja':'Nein'} · Asteroidengürtel: ${s.asteroidenguertel==null?'Unbekannt':s.asteroidenguertel?'Ja':'Nein'}</p><div class="table-wrap">${table(['Planet','Status','Typ','Bewohner','Ressourcenprofil','Aktion'],(s.plaetze||s.planeten||[]).map(p=>[`${p.bekannt&&p.zone?`<img class="planet-row-icon ${p.besiegt?'defeated-planet':''}" src="${esc(planetArt(p))}" alt="${esc(named(p.zone))}">`:''}${esc(p.koord)}`,badge(p.besiegt?'Besiegt'+(p.heimat?' · geschützte Heimat':''):named(p.status||'unbekannt'),p.besiegt?'defeated-name':p.bekannt?'good':'wait'),esc(named(p.zone||'unbekannt')),(p.bekannt||p.besiedlung_bekannt)&&p.spieler?relationHtml(p.spieler,p.beziehung):esc(p.status==='frei'||p.bewohnt===false?'Unbewohnt':'Unbekannt'),planetResources(p),`<button class="secondary" data-explore="${esc(p.koord)}" data-system="${!s.bekannt}">${s.bekannt?'Sonde':'Systemscan'}</button>${p.status==='frei'&&!p.heimat?` <button class="secondary" data-colonize="${esc(p.koord)}">Kolonisieren</button>`:''}`]))}</div><p class="muted">Beobachtungen sind historische Sondenberichte. Karte erneut ansehen, um neue Berichte zu laden.</p></article>`).join('')||empty('Keine Systeme im gewählten Bereich.');
}
function renderGalaxy(data){GalaxyUI.update(data);}
function renderColonies(p){
 const quotes=Object.fromEntries(['colony-quote','repair-quote'].map(id=>[id,$(id)?.innerHTML||'']));
 $('kolonien-content').innerHTML=`<div class="stats">${stat(view.kolonien.anzahl,'Kolonien')}${stat(view.kolonien.erlaubt,'Durch Astrophysik erlaubt')}${stat(view.kolonien.verwaltungsgrenze,'Verwaltungsgrenze')}</div><h3>Weg zur nächsten Kolonie</h3><ul class="conditions">${(view.kolonie_weg||[]).map(x=>`<li>${badge(x.erledigt?'Erfüllt':'Offen',x.erledigt?'good':'wait')} ${esc(x.text)}</li>`).join('')}</ul><p class="muted">Erst System und Zielplanet mit eigenen Sonden erkunden. Eine freie Kolonie braucht ein Kolonieschiff, bewaffnete Begleitung und die Startfracht für Minen, Farm, Solar und einen Tag Nahrung. Die Fracht wird nach Ankunft nicht automatisch verbaut.</p><div class="section-actions"><button type="button" id="colony-cargo">Startfracht anzeigen</button><button type="button" class="secondary" data-mission="kolonisieren">Freie Kolonie planen</button><button type="button" class="secondary" data-mission="kampfkolonisieren">Kampfkolonisation planen</button></div><div id="colony-quote">${quotes['colony-quote']}</div><h3>Laufende Besetzungen</h3>${(view.kampfkolonisationen||[]).length?view.kampfkolonisationen.map(b=>`<article class="fleet-card"><strong>${esc(b.planet)}</strong><p>Verteidiger-Reaktionen: ${b.abgeschlossene_reaktionsfenster} / 2 · frühestens nach <span data-arrival="${esc(b.fruehestens)}"></span> Spielzeit</p><p>${esc(b.hinweis)}</p></article>`).join(''):empty('Keine eigene oder gegen dich gerichtete Besetzung.')}<h3>Integrität & Reparaturen · ${esc(p.koord)}</h3>${table(['Gebäude','Integrität','Status',''],Object.entries(p.gebaeude_integritaet_prozent||{}).map(([g,hp])=>[esc(named(g)),hp+' %',p.reparaturen.some(r=>r.gebaeude===g)?'Reparatur läuft':hp<100?'Beschädigt':'Intakt',hp<100&&!p.reparaturen.some(r=>r.gebaeude===g)?`<button class="secondary" data-repair="${esc(g)}">Reparatur prüfen</button>`:'']))}<div id="repair-quote">${quotes['repair-quote']}</div><p class="muted">Kampfkolonisation braucht Orbitkontrolle und ein überlebendes Kolonieschiff. Gebäudeintegrität muss auf höchstens 30 % sinken. Mindestens 30 Spielminuten und zwei vollständige Verteidiger-Reaktionen verhindern eine sofortige Übernahme. Die ursprüngliche Heimatwelt bleibt geschützt.</p>`;
}
function renderCombat(){
 const result=$('combat-result')?.innerHTML||'';
 const can=document.activeElement?.closest('#combat-form');if(!can)$('kampf-content').innerHTML=`<h3>Kampfsimulator</h3><p class="muted">Wählt deine Flotte aus dem Flottenformular und rechnet mit deinem letzten vollständigen Spionagebericht. Fehlende Schiff- oder Verteidigungsdaten werden nicht als Null behandelt.</p><button class="secondary" data-go="flotten">Schiffe & Ziel auswählen</button><button type="button" id="simulate-combat">Gewählte Flotte simulieren</button><div id="combat-result">${result}</div><h3>Verbandsangriffe</h3><p class="muted">Eröffne einen Verband für eine eigene Angriffsflotte. Verbündete können beitreten; die Engine prüft Ziel, Freundschaft und rechtzeitige Ankunft.</p>${recordCards(view.verbaende,'Angriffsverband')}<form id="combat-form"><label>Eigene Flotte<input name="flotte" type="number" min="1" required></label><label>Führungsflotte<input name="fuehrung" type="number" min="1" required></label><button>Verband beitreten</button></form><h3>Kampfberichte</h3>${recordCards(view.kampfberichte,'Kampfbericht')}<h3>Trümmerfelder</h3>${(view.truemmer||[]).length?table(['Ort','Erz','Kristall',''],view.truemmer.map(t=>[esc(t.koord),fmt(t.erz),fmt(t.kristall),`<button class="secondary" data-recycle="${esc(t.koord)}">Recycler planen</button>`])):empty('Noch keine bekannten Trümmerfelder.')}<p class="muted">${rules.catalog.kampf.runden} Kampfrunden · Schild regeneriert je Runde · Rapidfire je Einheit · Verteidigungswiederaufbau ${fmt(rules.catalog.kampf.verteidigung_wiederaufbau*100)} % · Trümmeranteil ${fmt(rules.catalog.kampf.truemmer_anteil*100)} %</p>`;
}
function renderEmpire(){const planets=view.planeten;$('imperium-content').innerHTML=`<div class="table-wrap">${table(['Imperium',...planets.map(p=>p.koord),'Gesamt'],[
 ['Planet',...planets.map(p=>`<div class="empire-head"><img src="${esc(planetArt(p))}" alt="${esc(named(p.zone))}"><br>${esc(named(p.zone))}</div>`),''],
 ['Bevölkerung',...planets.map(p=>fmt(p.bevoelkerung)),fmt(planets.reduce((n,p)=>n+p.bevoelkerung,0))],
 ...Object.keys(planets[0].bestand).map(g=>[esc(named(g)),...planets.map(p=>fmt(p.bestand[g])),fmt(planets.reduce((n,p)=>n+p.bestand[g],0))]),
 ...Object.keys(rules.catalog.gebaeude).map(g=>[esc(named(g)),...planets.map(p=>p.gebaeude[g]||'—'),planets.reduce((n,p)=>n+(p.gebaeude[g]||0),0)||'—'])])}</div><h3>Rangliste dieser Welt</h3>${table(['Rang','Reich','Punkte','Stufe'],world.rangliste.map(r=>r.map((v,i)=>i===1&&(world.ausgeschiedene||[]).some(d=>d.name===v)?`<span class="defeated-name">${esc(v)} · Besiegt</span>`:esc(v))))}`;}
function renderUnlocks(){
 $('freischaltungen-content').innerHTML=`<p>Alle Stufen und Gebäudebedingungen stammen aus dem aktuellen Serverprofil. Gesperrte Motive bleiben sichtbar und nennen den Weg zur Freischaltung.</p>`+['buildings','research','ships','defenses'].map(cat=>`<h3>${{buildings:'Gebäude',research:'Forschung',ships:'Schiffe',defenses:'Verteidigung'}[cat]}</h3><div class="table-wrap">${table(['Inhalt','Zivilisationsstufe','Voraussetzungen','Status'],itemRecords(cat).map(i=>[esc(named(i.key)),i.r.ab_stufe,esc(cat==='research'?'Labor '+i.r.labor:Object.entries(i.r.braucht||{}).map(([k,n])=>named(k)+' '+n).join(' · ')||'Keine weiteren Gebäude'),i.needs.length?esc(i.needs.join(' · ')):badge('Technisch freigeschaltet','good')]))}</div>`).join('');
}
function renderProfile(){if(document.activeElement?.id==='progress-motion')return;const motion=document.body.dataset.progressMotion;const t=factionTraits(view.volk);$('profile-details').innerHTML=`<div class="profile-card">${imageTag('portraits',view.volk,view.volk)}<div><h3>${esc(view.name)}</h3><p>${esc(named(view.volk))} · Spielweise: ${esc(named(session.mode))}<br>Rang ${view.rang} · ${fmt(view.punkte.gesamt)} Punkte<br>${view.anfaengerschutz_bis?'Anfängerschutz bis '+esc(view.anfaengerschutz_bis):'Anfängerschutz abgelaufen'}</p></div></div><p class="plus">${esc(t.pros.join(' · '))}</p><p class="minus">${esc(t.cons.join(' · '))}</p><p class="muted">${esc(t.special.join(' '))} Die Volkswahl bleibt für die laufende Epoche fest. Dein Konto und Imperium bleiben nach dem Abmelden erhalten.</p><label>Fortschrittsdarstellung<select id="progress-motion"><option value="system" ${motion==='system'?'selected':''}>Systemeinstellung beachten</option><option value="clock" ${motion==='clock'?'selected':''}>Uhrzeiger · im Uhrzeigersinn einfärben</option><option value="still" ${motion==='still'?'selected':''}>Ruhige Darstellung · ohne Uhrzeiger</option></select></label>`;}
function renderGuide(){
 const c=rules?.catalog;if(!c||!world)return;
 const sections=[['Dein erster Ausbau','Sichere Energie, Nahrung und Wohnraum, bevor du weitere Produktionsgebäude ausbaust. Die Rohstoffleiste zeigt die aktuell ausgewählte Welt. Rot markierte Kosten sind noch nicht bezahlbar. In der Versorgung siehst du negative Raten und die Reichweite deiner Vorräte.'],['Bau, Forschung & Werft','Die aktive Kachel wird im Uhrzeigersinn farbig. Daneben stehen Restzeit und Prozent. Ein neuer Bau wartet hinter dem laufenden Auftrag; Kosten werden bei Baubeginn bezahlt. Forschung rechnet an vollen Spielstunden ab. Die Werft zeigt das nächste Stück eines Auftrags. Bei Pause frieren Spielzeit und Timer ein.'],['Aufklärung & Angriffe','Ein eigener Systemscan erschließt das Sonnensystem. Jeder Planet braucht anschließend eine eigene Spionagesonde, damit Typ, Bewohner und Ressourcen bekannt werden. Ohne Ausbau erscheint ein erfasster Angriff erst zwei Spielstunden vor Ankunft. Geheimdienst, Überwachungstechniken und Spionagetechnik erweitern Reichweite und Detailtiefe. Die genaue Ankunft ist bekannt, sobald die Sensoren die Flotte erfassen. Eine Flottensonde kann die Zusammenstellung prüfen; Abschirmung kann dies verhindern.'],['Flotten saven','Wähle den Auftrag Saven, Ziel und Wartezeit. „Alle vorhandenen Schiffe“ und „Save-Fracht automatisch verladen“ sichern deine Auswahl; die Frachtplanung reserviert Treibstoff für beide Strecken. Prüfe Hinflug, Aufenthalt und Rückkehr. Beim Rückflug können eine verlorene Landewelt oder Blockaden die Ankunft verändern. Rückruf und Rückkehr musst du selbst beobachten.'],['Kampfsystem',`Die Engine rechnet jede Einheit einzeln über bis zu ${c.kampf.runden} Runden. Ziele werden pro Schuss gewählt; Schilde regenerieren pro Runde. Treffer unter ${fmt(c.kampf.verpuffen_anteil*100)} % des vollen Schilds verpuffen. Unter ${fmt(c.kampf.explosion_unter*100)} % Reststruktur entsteht eine Explosionschance. Rapidfire kann weitere Schüsse auslösen. Waffen, Schilde, Panzerung und Volksboni beeinflussen den Ausgang. Der Planetenschild ist eine eigene Einheit. Der Simulator nutzt historische Berichte und kennzeichnet geschätzte Technik.`],['Ausscheiden & Epochenwechsel',`Aktuelle Rettungsfristen: ${world.ausscheiden_regeln?.versorgung_stunden??72} Spielstunden bei Unterversorgung und ${world.ausscheiden_regeln?.stillstand_stunden??48} bei wirtschaftlichem Stillstand. ${c.ausscheiden}`],['Kolonisation & Übernahme',c.kolonisation],['Menschen, Agenten & Rollen','Ein Konto spielt selbst (Mensch), lässt Modelle alle vier Rollen führen (Agent) oder teilt sich die Rollen mit ihnen (Gemischt). Die vier Rollen heißen Stratege, Verwalter, Feldherr und Diplomat. Modelle steuern entweder als Team hier im Browser unter Modelle & Team – Ollama-Aufrufe laufen auf deinem PC, OpenRouter-Aufrufe mit deinem Schlüssel im Browser – oder als eigener Harness: das Harness-Cockpit auf deinem PC oder ein eigenes Programm mit Harness-Schlüssel. Je Reich steuert immer nur einer dieser Wege, mit einer zeitlich begrenzten Steuerungsfreigabe. Stoppen gibt dir die Steuerung zurück. Alle Befehle werden vom Server geprüft.'],['Anmeldung & Warteliste','Die Alpha-Welt gibt zunächst fünf der zwanzig möglichen Spielerplätze frei. Ein Konto kann zuschauen und seine Volkswahl vor dem Start vergleichen. Bei voller Freigabe zählt die Reihenfolge der Warteliste. Wer aufgenommen wird, erhält sein Reich auch bei geschlossenem Browser; Serverbots und Wirtschaft laufen auf dem Host weiter.']];
 $('player-guide').innerHTML=`<div class="guide-grid">${sections.map(([t,p])=>`<article><h3>${t}</h3><p>${esc(p)}</p></article>`).join('')}</div>`+communicationFAQ();
}
// Polls never discard an in-progress selection or input. Commands stay in app.js.
document.addEventListener('input',e=>{if(e.target.id?.startsWith('quantity-'))uiSelections[e.target.id.slice(9)+'Qty']=e.target.value;});
document.addEventListener('change',e=>{if(e.target===$('fleet').elements.start&&view)renderFleetStock();if(e.target.id==='progress-motion'){document.body.dataset.progressMotion=e.target.value;localStorage.setItem('sternenepoche-progress-motion',e.target.value);}if(e.target=== $('claim').elements.volk){chosenRace=e.target.value;renderShell();}});
document.addEventListener('click',guard(async e=>{
 const b=e.target.closest('[data-race],[data-planet-pick],[data-item],[data-go],[data-make],[data-repair],[data-open-group],[data-recycle],[data-mission],[data-save-target],[data-colonize],#colony-cargo,#simulate-combat');if(!b)return;
 if(b.dataset.race){chosenRace=b.dataset.race;$('claim').elements.volk.value=chosenRace;renderShell();return;}
 if(b.dataset.go){navigateGame(b.dataset.go);if(b.dataset.go==='flotten')$('fleet-planner').open=true;return;}
 if(b.dataset.planetPick){selectedPlanet=b.dataset.planetPick;$('manufacture').elements.planet.value=selectedPlanet;$('fleet').elements.start.value=selectedPlanet;renderView();return;}
 if(b.dataset.item){uiSelections[b.dataset.category]=b.dataset.item;renderCatalog(b.dataset.category,{buildings:'gebaeude-content',research:'research',ships:'ship-gallery',defenses:'verteidigung-content'}[b.dataset.category]);tickGame();return;}
 if(b.dataset.make){const category=rules.catalog.einheiten[b.dataset.make].schiff?'ships':'defenses',qty=Number($('quantity-'+category).value);if(!Number.isInteger(qty)||qty<1||qty>10000)throw Error('Stückzahl muss zwischen 1 und 10.000 liegen.');await api('/api/tool',{typ:'kosten',planet:currentPlanet().koord,einheit:b.dataset.make});await command([{typ:'fertigen',planet:currentPlanet().koord,einheit:b.dataset.make,anzahl:qty}]);return;}
 if(b.dataset.repair){navigateGame('kolonien');const q=await api('/api/tool',{typ:'kosten',planet:currentPlanet().koord,gebaeude:b.dataset.repair});if(!q.reparatur)throw Error('Keine Reparaturquote verfügbar.');$('repair-quote').innerHTML=`<article class="card"><h3>Reparieren: ${esc(named(b.dataset.repair))}</h3>${costHtml(q.reparatur.kosten,currentPlanet().bestand)}<p>Dauer ${GameTime.duration(q.reparatur.dauer_sekunden)} Spielzeit</p><button data-confirm-repair="${esc(b.dataset.repair)}">Reparatur starten</button></article>`;return;}
 if(b.dataset.openGroup){await command([{typ:'verband_oeffnen',flotte:Number(b.dataset.openGroup)}]);return;}
 if(b.dataset.recycle){pickFleet(b.dataset.recycle,'recyceln');return;}
 if(b.dataset.colonize){pickFleet(b.dataset.colonize,'kolonisieren');return;}
 if(b.dataset.mission){navigateGame('flotten');$('fleet-planner').open=true;$('fleet').elements.mission.value=b.dataset.mission;return;}
 if(b.dataset.saveTarget){navigateGame('flotten');$('fleet-planner').open=true;$('fleet').elements.start.value=b.dataset.saveTarget;$('fleet').elements.mission.value='saven';selectAllShips();return;}
 if(b.id==='colony-cargo'){const q=await api('/api/tool',{typ:'regel',stichwort:'kolonisation'});$('colony-quote').innerHTML=`<article class="card"><h3>Mindest-Startfracht für dein Volk</h3>${costHtml(q.kolonie_startfracht,currentPlanet().bestand)}<p>${esc(q.text)}</p></article>`;return;}
 if(b.id==='simulate-combat'){const a=fleetData(),q=await api('/api/tool',{typ:'kampfsimulator',ziel:a.ziel,schiffe:a.schiffe});$('combat-result').innerHTML=`<article class="card"><h3>Simulation: ${esc(a.ziel)}</h3><div class="stats">${stat(q.siegchance_prozent+' %','Siegchance')}${stat(q.unentschieden_prozent+' %','Unentschieden')}${stat(fmt(q.eigene_verluste_wert),'Eigene Verlustwerte')}${stat(fmt(q.verluste_gegner_wert),'Gegnerische Verlustwerte')}</div><p>${q.laeufe} Läufe · Bericht ${q.bericht_alter_stunden} Spielstunden alt · Gegnertechnik: ${esc(q.technik_des_gegners)}</p><p>Beute ohne Ladegrenze: ${esc(goods(q.beute_bei_sieg_ohne_ladegrenze))}</p><p class="muted">Eine Simulation garantiert keinen Ausgang. Seit der Beobachtung können Flotten, Verteidigung und Technik geändert worden sein.</p></article>`;}
}));
document.addEventListener('click',guard(async e=>{const b=e.target.closest('[data-confirm-repair]');if(b)await command([{typ:'reparieren',planet:currentPlanet().koord,gebaeude:b.dataset.confirmRepair}]);}));
document.addEventListener('submit',guard(async e=>{if(e.target.id==='priority-form'){const list=e.target.elements.reihenfolge.value.split(',').map(s=>s.trim()).filter(Boolean);await command([{typ:'prioritaeten',planet:currentPlanet().koord,reihenfolge:list}]);}if(e.target.id==='combat-form')await command([{typ:'verband_beitreten',flotte:Number(e.target.elements.flotte.value),fuehrung:Number(e.target.elements.fuehrung.value)}]);}));
setInterval(tickGame,250);

// Persistent resource instruments: animate only differences between confirmed snapshots.
// Planet/account/world changes establish a fresh baseline and never simulate income.
const ResourceDisplay={
 keys:['erz','kristall','deuterium','nahrung','energie','credits'],
 producers:{erz:['erzmine'],kristall:['kristallmine'],deuterium:['deuteriumsynthesizer'],nahrung:['farm'],energie:['solarkraftwerk','fusionskraftwerk']},
 state:new Map(),identity:'',motion:matchMedia('(prefers-reduced-motion: reduce)'),
 quiet(){return this.motion.matches||document.body.dataset.progressMotion==='still'||document.hidden;},
 format(n){return Math.abs(n)>=1e6?new Intl.NumberFormat('de-DE',{notation:'compact',maximumFractionDigits:1}).format(n):fmt(n);},
 reset(){for(const s of this.state.values()){cancelAnimationFrame(s.frame);clearTimeout(s.timer);}this.state.clear();this.identity='';},
 settle(){for(const s of this.state.values()){cancelAnimationFrame(s.frame);clearTimeout(s.timer);s.display=s.value;s.node.querySelector('.resource-value').textContent=this.format(s.value);s.node.classList.remove('resource-changed','resource-burst');}},
 effect(g){
  if(g==='energie')return '<svg class="resource-lightning" viewBox="0 0 120 100"><path d="M76 0 48 35 69 35 32 90 45 49 24 49 48 0"/><path class="bolt-branch" d="m93 22-13 25 15-2-25 37 8-29-12 2"/></svg>';
  return '<span class="resource-ring"></span>'+'<i class="resource-particle"></i>'.repeat(6);
 },
 render(p,v,w){
  const bar=$('resource-bar'),identity=JSON.stringify([w.world_id,session?.spieler,v.name,p.koord]);
  if(this.identity!==identity||!bar.querySelector('.resource-value')){
   this.reset();this.identity=identity;
   bar.setAttribute('aria-label','Ressourcen des ausgewählten Planeten und Imperium-Credits');
   bar.innerHTML=this.keys.map(g=>`<article class="resource" data-resource="${g}" aria-label="${esc(named(g))}"><div class="resource-icon">${imageTag('resources',g,'')}<span class="resource-effect" aria-hidden="true">${this.effect(g)}</span></div><small class="resource-name">${esc(named(g))}</small><strong class="resource-value"></strong><span class="resource-delta" aria-hidden="true"></span><span class="resource-rate"></span><span class="resource-meter" aria-hidden="true"><i></i></span><span class="resource-detail"></span></article>`).join('');
  }
  for(const g of this.keys){
   const node=bar.querySelector(`[data-resource="${g}"]`),value=Number(g==='energie'?p.energie.erzeugung-p.energie.verbrauch:g==='credits'?v.credits:p.bestand[g])||0;
   const levels=Object.fromEntries((this.producers[g]||[]).map(k=>[k,p.gebaeude?.[k]||0]));
   let s=this.state.get(g);const fresh=!s;
   if(fresh){s={node,value,display:value,levels,frame:0,timer:0};this.state.set(g,s);}
   const delta=value-s.value,completed=Object.keys(levels).some(k=>levels[k]>(s.levels[k]||0));
   const rate=Number(p.rate?.[g])||0,cap=Number(p.lager?.[g])||0,energy=g==='energie';
   const deficit=energy?value<0:g!=='credits'&&rate<0;
   node.classList.toggle('resource-deficit',deficit);node.classList.toggle('resource-full',!energy&&cap>0&&value>=cap);node.classList.toggle('resource-large',Math.abs(value)>=10000);
   node.dataset.direction=delta<0?'down':'up';
   node.querySelector('.resource-rate').textContent=energy?`${fmt(p.energie.erzeugung)} erzeugt · ${fmt(p.energie.verbrauch)} Bedarf`:g==='credits'?'Imperiumskasse':`${rate>=0?'+':''}${fmt(rate)} / Spielstunde`;
   node.querySelector('.resource-detail').textContent=energy?(deficit?'⚠ Energiedefizit':'Versorgung gesichert'):g==='credits'?'Reichsweit verfügbar':cap>0?`${fmt(Math.min(100,Math.max(0,value/cap*100)))} % Lager${deficit?' · Verbrauch':value>=cap?' · voll':''}`:'Kein Lagerlimit';
   const fill=energy?(p.energie.verbrauch>0?Math.min(1,p.energie.erzeugung/p.energie.verbrauch):1):cap>0?Math.min(1,Math.max(0,value/cap)):0;
   node.querySelector('.resource-meter').hidden=g==='credits'||(!energy&&!cap);
   node.querySelector('.resource-meter i').style.transform=`scaleX(${fill})`;
   node.title=`${named(g)}: ${fmt(value)}. ${node.querySelector('.resource-rate').textContent}. ${energy?(deficit?'Energie fehlt.':'Energie reicht aus.'):cap>0?'Lagerkapazität: '+fmt(cap):'Gilt für dein gesamtes Imperium.'}`;
   node.setAttribute('aria-label',node.title);
   s.value=value;s.levels=levels;
   if(fresh||this.quiet()){cancelAnimationFrame(s.frame);clearTimeout(s.timer);s.display=value;node.querySelector('.resource-value').textContent=this.format(value);node.classList.remove('resource-changed','resource-burst');continue;}
   if(!delta&&!completed)continue;
   cancelAnimationFrame(s.frame);clearTimeout(s.timer);
   const from=s.display,start=performance.now();
   const count=stamp=>{const t=Math.min(1,(stamp-start)/650);s.display=from+(value-from)*(1-(1-t)**3);node.querySelector('.resource-value').textContent=this.format(s.display);if(t<1)s.frame=requestAnimationFrame(count);else{s.display=value;node.querySelector('.resource-value').textContent=this.format(value);}};
   s.frame=requestAnimationFrame(count);
   node.querySelector('.resource-delta').textContent=completed?'↑ Ausbau fertig':`${delta>0?'+':''}${fmt(delta)}`;
   node.classList.remove('resource-changed','resource-burst');
   // Restart one bounded, decorative effect; unchanged polling keeps its DOM intact.
   void node.offsetWidth;
   node.classList.add('resource-changed');
   if(completed||(delta>0&&g!=='energie'))node.classList.add('resource-burst');
   s.timer=setTimeout(()=>node.classList.remove('resource-changed','resource-burst'),2200);
  }
 }
};
ResourceDisplay.motion.addEventListener('change',()=>{if(ResourceDisplay.quiet())ResourceDisplay.settle();});
document.addEventListener('visibilitychange',()=>{if(document.hidden)ResourceDisplay.settle();});
document.addEventListener('change',e=>{if(e.target.id==='progress-motion'&&ResourceDisplay.quiet())ResourceDisplay.settle();});
