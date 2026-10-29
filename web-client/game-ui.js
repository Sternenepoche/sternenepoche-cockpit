'use strict';
// Pure timing helpers. Only server snapshots confirm completion.
const GameTime={
  anchor:{seconds:0,stamp:0,speed:0,connected:false},
  // Kleine Abweichungen zum Server (lokale Welten rücken nur alle 5 s vor) gleitet die Uhr aus, statt zurückzuspringen:
  // sonst sprangen Flugbalken und Restentfernung bei jedem Abruf hin und her (Karl 22.10.2026). Die Uhr läuft dabei
  // mindestens mit halbem Tempo weiter, nie rückwärts; große Sprünge (Pause, Tempowechsel, Neuverbindung) gelten sofort.
  sync(w,stamp=performance.now()){
    const speed=w.paused||w.beendet?0:w.tempo,ziel=w.sekunden+(w.tick_alter_ms||0)/1000*speed,a=this.anchor;
    const abw=a.connected&&speed>0&&a.speed===speed?this.now(stamp)-ziel:null,gleiten=abw!=null&&Math.abs(abw)<=8*speed;
    this.anchor={seconds:ziel,stamp,speed,connected:true,korrektur:gleiten?abw:0,dauer:gleiten?Math.max(1.5,2*Math.abs(abw)/speed):0};
  },
  now(stamp=performance.now()){
    const a=this.anchor;if(!a.connected)return a.seconds;const dt=Math.max(0,(stamp-a.stamp)/1000);
    return a.seconds+Math.min(15,dt)*a.speed+(a.korrektur?a.korrektur*Math.max(0,1-dt/a.dauer):0);
  },
  disconnect(stamp=performance.now()){this.anchor={seconds:this.now(stamp),stamp,speed:this.anchor.speed,connected:false,korrektur:0,dauer:0};},
  job(j,seconds,snapshot){
    if(j.wartet||j.fertig_sekunden==null)return {progress:0,remaining:null};
    const remaining=Math.max(0,j.fertig_sekunden-(j.pausiert?snapshot:seconds));
    if(j.punkte_gesamt){
      const elapsed=Math.max(0,(j.pausiert?snapshot:seconds)-snapshot);
      const tick=j.naechster_tick_sekunden??(Math.floor(snapshot/3600)+1)*3600;
      const start=Math.max(tick-3600,j.begonnen_sekunden??tick-3600);
      const until=Math.max(1,tick-start);
      const before=Math.max(0,Math.min(tick,snapshot)-start)/until;
      const after=Math.max(0,Math.min(tick,snapshot+elapsed)-start)/until;
      const credited=Math.min(j.punkte_rest,Math.max(0,j.rate||0)*(after-before+Math.max(0,snapshot+elapsed-tick)/3600));
      return {progress:Math.min(1,Math.max(0,1-(j.punkte_rest-credited)/j.punkte_gesamt)),remaining};
    }
    return {progress:Math.min(1,Math.max(0,1-remaining/Math.max(1,j.dauer_sekunden))),remaining};
  },
  // Ab einem Tag „N T hh:mm:ss“ statt „395:53:46“ (Belagerung 48 h, Dysonbau 30 Tage, lange Flüge).
  duration(s){if(s==null||!Number.isFinite(s))return 'Wartet';s=Math.max(0,Math.ceil(s));const t=Math.floor(s/86400),h=Math.floor(s%86400/3600),m=Math.floor(s%3600/60);const hms=`${String(h).padStart(2,'0')}:${String(m).padStart(2,'0')}:${String(s%60).padStart(2,'0')}`;return t?`${t} T ${hms}`:hms;}
};
// DOM bindings below. GameTime is also exercised without a browser in UI tests.
document.body.dataset.progressMotion=localStorage.getItem('sternenepoche-progress-motion')||'system';
let selectedPlanet='',chosenRace='aurelianer',uiSelections={},uiJobs=new Map(),uiReady=false,uiWorldKey='',uiViewOwner='',uiLastReportStamp='';
const named=k=>window.STERNEN_ART?.names[k]||label(k);
const artAliases={buildings:{geheimdienst:'sensorphalanx'},research:{ueberwachungstechnik:'spionagetechnik',abschirmtechnik:'schildtechnik'},ships:{bergungsschiff:'recycler'},missions:{expedition:'spionage',bergen:'transport',system_erkunden:'spionage',flotten_spionage:'spionage',saven:'transport'}};
// Klappbilder (Todesmond) zeigen zuerst die geschlossene Reiseform; planet-kugel.js klappt beim Überfahren auf.
function art(category,key,faction=view?.volk||''){const url=artDirekt(category,key,faction);return window.PlanetKugel?.ruheform(url)||url;}
function artDirekt(category,key,faction=view?.volk||''){
 const images=window.STERNEN_ART?.images||{},alias=artAliases[category]?.[key]||key;
 // Katalogaliase (art.js "aliases", aus content/catalog.json): anderes Motiv in anderer Kategorie, z. B. Solarsonde -> satellites.
 const [ac,ak]=String(window.STERNEN_ART?.aliases?.[`${category}|${key}`]||'|').split('|');
 return images[`${category}|${key}|${faction}`]?.url||images[`${category}|${key}|`]?.url||(ac&&(images[`${ac}|${ak}|${faction}`]?.url||images[`${ac}|${ak}|`]?.url))||images[`${category}|${alias}|${faction}`]?.url||images[`${category}|${alias}|`]?.url||images['backgrounds|stars|']?.url||'';
}
function imageTag(category,key,faction,cls=''){return `<img class="${cls}" src="${esc(art(category,key,faction))}" alt="${esc(named(key))}" loading="lazy">`;}
// Oberfläche je Planet: eine Formel für Tabelle, Listen, 3D-System und Übersicht (planet-kugel.js).
// Eigener Mond in der Planetenkugel: Antippen landet auf der Mondseite dieses Mondes (Karl 25.10.2026: „direkt in dem Mond
// lande und dort was bauen kann“). Fremde oder noch nicht freigeschaltete Monde bleiben Bild.
function mondLandung(p){
 const m=window.Mondseite?.eigene?.().find(x=>x.koord===p?.koord);
 return m?{mondKlick:()=>Mondseite.oeffne(m.koord),mondName:m.name||'Mond'}:{};
}
function planetArt(p){const typ=window.PlanetKugel?.typ(p)||'leben_kontinent';return window.PlanetKugel?.kugelbild('planets',typ)||art('planets',typ,'');}
function currentPlanet(){return view?.planeten.find(p=>p.koord===selectedPlanet)||view?.planeten[0];}
function uiWorldSync(){if(world&&!view)GameTime.sync(world);}
function uiConnectionLost(){GameTime.disconnect();tickGame();}
function costHtml(cost,stock){return `<div class="costs">${Object.entries(cost||{}).filter(([,n])=>n>0).map(([g,n])=>`<span class="cost ${stock&&(stock[g]||0)<n?'short':''}" title="${esc(named(g))}${stock?' · vorhanden '+fmt(stock[g]||0):''}">${imageTag('resources',g,'')}${fmt(n)}</span>`).join('')}</div>`;}
function keepJob(job){const key=[job.planet||'',job.gebaeude||job.forschung||job.produkt||'job',job.stufe||0,job.fertig_sekunden??'wait',job.dauer_sekunden||job.punkte_gesamt||0].join(':');uiJobs.set(key,job);return key;}
function artShell(category,key,job,faction){
 const f=faction??view?.volk??'',alias=artAliases[category]?.[key]||key,images=window.STERNEN_ART?.images||{};
 if(!images[`${category}|${alias}|${f}`]&&!images[`${category}|${alias}|`])return `<div class="art-shell art-pending" role="img" aria-label="${esc(named(key))} · Motiv wird ergänzt"><span>✧<small>Volksmotiv wird ergänzt</small></span></div>`;
 const src=esc(art(category,key,faction));if(!job)return `<div class="art-shell"><img class="art-img" src="${src}" alt="${esc(named(key))}" loading="lazy"></div>`;
 const id=keepJob(job);return `<div class="art-shell running" data-job="${esc(id)}" role="progressbar" aria-label="${esc(named(key))} · Fortschritt" aria-valuemin="0" aria-valuemax="100"><img class="art-img" src="${src}" alt="${esc(named(key))}" loading="lazy"><img class="art-color" src="${src}" alt="" loading="lazy"><span class="timer-label">Synchronisiert …</span></div>`;
}
function jobLine(job){const id=keepJob(job);return `<div data-job="${esc(id)}" class="queue-bottom"><span class="job-time"></span><div class="job-meter"><span class="job-fill"></span></div></div>`;}
// Eine Zeitanzeige im Kopf (app.js schrieb bei jedem Abruf „… Spielzeit“, der Takt danach ohne – die Zeile sprang hin und her).
function weltZeit(now){const paused=world?.paused||world?.beendet;return `Tag ${Math.floor(now/86400)+1} · ${GameTime.duration(now%86400)}${paused?' · Pause':''}`;}
const schreibe=(el,t)=>{if(el&&el.textContent!==t)el.textContent=t;};
// Timer, Balken und Countdowns unter root auf den aktuellen Stand setzen. Schreibt nur, was sich ändert (jede Änderung
// kostet am Handy ein Neuzeichnen). Auch direkt nach setStableHTML aufgerufen, damit nichts kurz leer steht.
function tickJobs(root=document,now=GameTime.now()){
 const snapshot=view?.sekunden??GameTime.anchor.seconds,paused=world?.paused||world?.beendet,tempo=Math.max(1,world?.tempo||1);
 root.querySelectorAll('[data-job]').forEach(el=>{
   const j=uiJobs.get(el.dataset.job);if(!j)return;const r=GameTime.job(j,now,snapshot),percent=Math.round(r.progress*100);
   const state=j.wartet?'In Warteschlange':j.fertig_sekunden==null?(j.punkte_gesamt?'Steht: Labor ohne Forscher (Akademie bildet aus)':'Wartet auf Rohstoffe'):r.remaining===0?'Abschluss wird übernommen':j.pausiert?(j.manuell_pausiert?'Pausiert':'Pausiert · '+(j.ruht_grund||'Versorgung / Zustand prüfen')):paused?'Welt pausiert':!GameTime.anchor.connected?'Verbindung unterbrochen':percent+' %';
   const time=r.remaining==null?'—':GameTime.duration(r.remaining/tempo);
   const p=String(Math.round(r.progress*1000)/1000);
   if(el.style.getPropertyValue('--progress')!==p)el.style.setProperty('--progress',p);
   const halt=!!j.pausiert||!!paused;if(el.classList.contains('job-paused')!==halt)el.classList.toggle('job-paused',halt);
   if(el.classList.contains('art-shell')){if(el.getAttribute('aria-valuenow')!==String(percent))el.setAttribute('aria-valuenow',percent);const vt=time+' · '+state;if(el.getAttribute('aria-valuetext')!==vt)el.setAttribute('aria-valuetext',vt);const label=el.querySelector('.timer-label');const html=`${time}<small>${state}</small>`;if(label&&label.innerHTML!==html)label.innerHTML=html;}
   else{const jf=el.querySelector('.job-fill'),tf=`scaleX(${p})`;schreibe(el.querySelector('.job-time'),time+' · '+state);schreibe(el.querySelector('.job-uhr'),time);schreibe(el.querySelector('.job-zustand'),state);if(jf&&jf.style.transform!==tf)jf.style.transform=tf;}
   if(r.remaining===0&&!paused&&GameTime.anchor.connected&&typeof requestCompletionRefresh==='function')requestCompletionRefresh();
 });
 root.querySelectorAll('[data-arrival]').forEach(el=>schreibe(el,GameTime.duration(Math.max(0,Number(el.dataset.arrival)-now))+(paused?' · Pause':'')));
 root.querySelectorAll('[data-arrival-echt]').forEach(el=>schreibe(el,GameTime.duration(Math.max(0,Number(el.dataset.arrivalEcht)-now)/tempo)+(paused?' · Pause':'')));
}
function tickGame(){
 const now=GameTime.now();
 if(world)schreibe($('world-time'),weltZeit(now));
 tickJobs(document,now);
 if(typeof tickFleetRoutes==='function')tickFleetRoutes(now); // management.js lädt nach game-ui.js
}
function factionTraits(name){
 const c=rules?.catalog,v=c?.voelker?.[name];if(!v)return {pros:[],cons:[],special:['Regeln werden geladen …']};
 const pros=[],cons=[],special=[];
 const fields=[['ladung','Laderaum',1],['marktgebuehr','Marktgebühren',-1],['waffen','Waffenstärke',1],['panzerung','Strukturpunkte',1],['steuer','Steuereinnahmen',1],['werftzeit','Fertigungsdauer',-1],['forschung','Forschungsleistung',1],['wachstum','Bevölkerungswachstum',1],['nahrung','Nahrungsproduktion',1],['energie','Energieproduktion',1],['kolonieschiff','Kolonieschiffkosten',-1]];
 for(const[k,title,good]of fields){const delta=Math.round(((v[k]??1)-1)*100);if(delta)(delta*good>0?pros:cons).push(`${delta>0?'+':''}${delta} % ${title}`);}
 if(v.pluenderquote!=null&&v.pluenderquote!==c.kampf.pluenderquote){const delta=v.pluenderquote-c.kampf.pluenderquote;(delta>0?pros:cons).push(`${fmt(v.pluenderquote*100)} % Plünderquote (Standard ${fmt(c.kampf.pluenderquote*100)} %)`);}
 if(v.ohne_nahrung){pros.push('Bevölkerung benötigt keine Nahrung');cons.push(`${fmt(c.wirtschaft.energie_je_1000_syntheten)} Energie / 1.000 Einwohner zusätzlich`);}
 if(name==='veyari')special.push('Flottenaufklärung +1 ab Spionagetechnik 1: Sonden gegen anfliegende Flotten erkennen mehr. Planetenspionage unverändert; Geheimdienst und Sonden bleiben nötig.');
 if(name==='syntheten')special.push('Sensorstärke +1 nach Erforschung der Überwachungstechniken. Geheimdienst muss ausgebaut sein.');
 return {pros,cons,special};
}
function factionCards(target,locked=false){
 const titles={aurelianer:['Handel & Logistik','Wirtschaft durch Austausch und große Frachträume.'],krath:['Flotten & Eroberung','Starke Waffen und schnelle Schiffsfertigung.'],veyari:['Wachstum & Aufklärung','Lebendige Kolonien und geschulte Spionage.'],syntheten:['Technik & Sensoren','Forschungsstärke mit eigener Energieversorgung.']};
 setStableHTML($(target), Object.entries(titles).map(([name,[title,desc]])=>{const t=factionTraits(name);return `<button type="button" class="faction-card" data-race="${name}" aria-pressed="${chosenRace===name}" ${locked?'disabled':''}>${imageTag('portraits',name,name)}<span class="faction-copy"><strong>${named(name)}</strong><small>${title} · ${desc}</small><b class="trait-heading">Vorteile</b>${t.pros.map(x=>`<span class="plus">${esc(x)}</span>`).join('')}<b class="trait-heading">Nachteile</b>${t.cons.map(x=>`<span class="minus">${esc(x)}</span>`).join('')}${t.special.map(x=>`<span class="special">${esc(x)}</span>`).join('')}</span></button>`;}).join(''));
}
function renderShell(){
 if(!rules||!world)return;
 const guideKey=JSON.stringify([rules.catalog,world.ausscheiden_regeln]);if(uiReady!==guideKey){uiReady=guideKey;renderGuide();}
 const key=world.world_id;if(uiWorldKey!==key){uiWorldKey=key;selectedPlanet='';uiSelections={};uiViewOwner='';}
 setStableHTML($('account-strip'), session?.spieler!=null?`<img loading="lazy" decoding="async" src="${esc(art('portraits',view?.volk||chosenRace,view?.volk||chosenRace))}" alt=""><span>${esc(named(view?.volk||chosenRace))}${(view?.volk||chosenRace)==='aurelianer'?' · Mensch':''}</span>`:'Beobachter · Alpha-Universum');
 $('account-strip').title=session?`${session.name} · ${session.spieler==null?'Beobachter':named(session.mode)}`:'Beobachter';
 const claimed=session?.spieler!=null;document.body.dataset.playing=String(claimed);$('claim').elements.volk.disabled=claimed;
 if($('inbox-shortcut'))$('inbox-shortcut').hidden=!claimed;
 if(session?.warteliste&&!document.activeElement?.closest('#claim'))chosenRace=session.warteliste.volk;
 if(claimed)chosenRace=view?.volk||chosenRace;
 $('race-fixed').textContent=claimed?'Dein Volk steht für diese Epoche fest. Die Spielweise und Agentenrollen kannst du jederzeit ändern.':window.STERNENEPOCHE?.local?'Deine Volkswahl gilt für die ganze Partie.':'Deine Volkswahl wird bei der Aufnahme in die Spielwelt verbindlich. Auf der Warteliste kannst du sie noch ändern.';
 // Do not rebuild the selection cards under a pointer or focused button every poll.
 const stamp=JSON.stringify([rules.catalog?.voelker,chosenRace,claimed]);
 for(const id of ['volk-preview','volk-choice'])if($(id).dataset.stamp!==stamp){factionCards(id,id==='volk-choice'&&claimed);$(id).dataset.stamp=stamp;}
 $('resource-bar').hidden=!claimed;$('event-bar').hidden=!claimed;
 if(window.GalaxyUI)GalaxyUI.visible(claimed&&document.querySelector('[data-tab="karte"]')?.getAttribute('aria-selected')==='true');
 if(!claimed){setStableHTML($('planet-sidebar'), '');ResourceDisplay.reset();return;}
 $('claim').elements.volk.value=chosenRace;
}
function queueCard(title,category,job,rest=[],destination='gebaeude'){
 const key=job?.gebaeude||job?.forschung||job?.produkt;
 return `<article class="queue-card ${job?'queue-active':'queue-empty'}"><strong>${title}</strong>${job?artShell(category,key,job):`<p class="empty">Kein laufender Auftrag</p>`}${job?`<p>${esc(named(key))}${job.stufe?' · Stufe '+job.stufe:job.rest?' · noch '+job.rest+' Stück':''}</p>${jobLine(job)}`:''}${rest.length?`<ol class="queue-list">${rest.map(j=>`<li>${esc(named(j.gebaeude||j.forschung||j.produkt||j))}${j.stufe?' '+j.stufe:''} · Wartet</li>`).join('')}</ol>`:''}${queueControls(category)}<button class="secondary" type="button" data-go="${destination}">Öffnen</button></article>`;
}
function renderView(){
 const p=currentPlanet();if(!p)return;
 if(uiViewOwner!==`${world.world_id}:${view.name}`){uiViewOwner=`${world.world_id}:${view.name}`;selectedPlanet=p.koord;uiSelections={};}
 selectedPlanet=p.koord;chosenRace=view.volk;document.body.dataset.reichStatus=view.reich_status?.status||'aktiv';for(const planet of view.planeten)for(const j of [...planet.bauschleife,...planet.fertigung])j.planet=planet.koord;
 progressionSnapshot();
 $('empire-name').textContent=`${view.name} · ${named(view.volk)}`;
 setStableHTML($('empire-summary'), stat(view.stufe,'Zivilisationsstufe','Stufe')+stat(fmt(view.punkte.gesamt),'Imperiumspunkte','Punkte')+stat(`${view.flottenplaetze.belegt} / ${view.flottenplaetze.gesamt}`,'Flottenslots','Flotten')+stat(`${view.planeten.length} / ${view.planetenkapazitaet?.maximal||15}`,`Planeten mit Heimatwelt · ${view.planetenkapazitaet?.erlaubt??1+view.kolonien.erlaubt} von ${view.planetenkapazitaet?.maximal||15} freigeschaltet`,'Planeten'));
 setStableHTML($('warnings'), (window.GroundUI?.alert()||'')+reichNotice()+view.warnungen.map(w=>`<button type="button" class="warning" data-go="${/Bau|Forschung|Fertigung/i.test(w)?'auftraege':/Flotte|Angriff/i.test(w)?'flotten':'kolonie'}">${esc(w)} ↗</button>`).join(''));
 const energy=p.energie.erzeugung-p.energie.verbrauch;
 ResourceDisplay.render(p,view,world);
 $('event-bar').classList.toggle('attack',view.angriffe.length>0||laufendeInvasionen().length>0||view.reich_status?.status==='kritisch');
 setStableHTML($('event-bar'), `<span class="event-location">${esc(p.koord)} · ${p.heimat?'Heimat':'Kolonie'}</span><span class="event-fleets" title="Eigene Flotten unterwegs">Flotten: ${view.flotten.length}</span><span class="event-contacts">${angriffsText()}</span><span class="event-clock">${world.beendet?'Epoche beendet':world.paused?'Welt pausiert':world.tempo+'× Spieltempo'}</span>${view.reich_status?.status==='besiegt'?'<strong class="event-critical">BESIEGT · Nur noch zuschauen</strong>':view.reich_status?.status==='kritisch'?'<strong class="event-critical">⚠ Existenzielle Krise</strong><span class="event-critical">Rettungsfrist: <span data-arrival="'+Math.min(...view.reich_status.rettungsfristen.map(k=>k.frist))+'"></span> Spielzeit</span>':''}${view.anfaengerschutz_bis?'<span class="event-protection" title="Anfängerschutz bis '+esc(view.anfaengerschutz_bis)+'">Schutz bis '+esc(view.anfaengerschutz_bis)+'</span>':''}`);
 PlanetNavigation.render();
 // Preserve choices and typing in the actual fleet, market and manufacturing forms.
 document.querySelectorAll('.own-planets').forEach(s=>{const val=s.value,markup=view.planeten.map(x=>`<option value="${esc(x.koord)}">${esc(window.Planetenname?Planetenname.text(x.koord):x.koord)}</option>`).join('');if(s.innerHTML!==markup){setStableHTML(s, markup);if(view.planeten.some(x=>x.koord===val))s.value=val;}});
 setStableHTML($('planets'), `<div class="hero-planet" data-ai-image><span class="hero-orb kugel-host" id="hero-kugel"><img loading="lazy" decoding="async" src="${esc(planetArt(p))}" alt="${esc(named(p.zone))} · ${esc(p.koord)}"><canvas class="planet-kugel" aria-hidden="true"></canvas></span><div class="hero-info"><span class="eyebrow">${p.heimat?'DEINE HEIMAT ZWISCHEN DEN STERNEN':'KOLONIE DEINES IMPERIUMS'}</span><h3>${window.Planetenname?Planetenname.html(p.koord,{standardname:true})+Planetenname.stift(p.koord):esc(p.koord)}</h3><span>${esc(named(p.zone))} · ${esc(named(view.volk))}</span><dl><dt>Baufelder</dt><dd>${p.felder.belegt} / ${p.felder.gesamt}</dd><dt>Bevölkerung</dt><dd>${fmt(p.bevoelkerung)}</dd><dt>Wohnraum</dt><dd>${fmt(p.wohnraum)}</dd><dt>Stabilität</dt><dd>${p.stabilitaet} %</dd><dt>Energiebilanz</dt><dd>${fmt(energy)}</dd><dt>Arbeitskräfte</dt><dd>${fmt(p.arbeit.verfuegbar)} / ${fmt(p.arbeit.bedarf)}</dd><dt>Fachkräfte</dt><dd>${fmt(p.fachkraefte.verfuegbar)} / ${fmt(p.fachkraefte.bedarf)}</dd><dt>Blockade</dt><dd>${p.blockade?esc(p.blockade.durch):'Keine'}</dd></dl></div></div><div class="section-actions"><button data-go="gebaeude">Gebäude ausbauen</button><button class="secondary" data-go="flotten">Flotten verwalten</button><button class="secondary" data-go="berichte">Neue Berichte</button></div>`);
 // Planetenansicht: echter Planet mit seinem echten Mond, langsame Drehung (Sonnensystem dreht schneller).
 window.PlanetKugel?.einbinden($('hero-kugel'),{planet:p,mond:p.mond,ansicht:'planet',...mondLandung(p)});
 renderCatalog('buildings','gebaeude-content');renderCatalog('research','research');renderCatalog('ships','ship-gallery');renderCatalog('defenses','verteidigung-content');
 renderFleetStock();renderEconomy(p);window.Mobile035?.renderIndustry();renderFleets();renderColonies(p);renderCombat();renderEmpire();renderUnlocks();renderProfile();
 // Existing account, agent, market and diplomacy controls keep their authoritative handlers.
 const govFocus=document.activeElement?.closest('#government');if(!govFocus)renderGovernment();
 renderProducts();renderProduction();
 const reportStamp=JSON.stringify([view.systemberichte,view.ereignisse,view.chronik,view.berichte,view.flottenberichte,view.kampfberichte,view.nachrichten,view.meldungen,view.erkundet,view.bodenberichte,view.monde?.spionageberichte,view.monde?.phalanxberichte]);
 if(uiLastReportStamp!==reportStamp){uiLastReportStamp=reportStamp;renderReports();}
 // Nur ein Eingabefeld schützt vor dem Neuzeichnen – ein angetippter Stornieren-Knopf hielt die stornierte Order sichtbar (25.10.2026).
 {const f=document.activeElement;if(!(f?.closest('#human-diplomacy')&&/^(INPUT|SELECT|TEXTAREA)$/.test(f.tagName)))renderDiplomacy();}
 const activeJobKeys=new Set([...document.querySelectorAll('[data-job]')].map(e=>e.dataset.job));for(const key of uiJobs.keys())if(!activeJobKeys.has(key))uiJobs.delete(key);renderManagement();renderTrade();animateOrderChanges();tickGame();renderShell();applyControlState();window.Mobile035?.renderGround();if($('flight-schedule'))window.Mobile035?.renderFlightSchedule?.();
}
// Laufende Bombardierung/Belagerung eigener oder verbündeter Welten zählt als Angriff (vorher nur anfliegende Flotten:
// „Keine Angriffe erkannt“, während die Kolonie belagert wurde – Konsolidierung M1, 19.10.2026).
function laufendeInvasionen(){return (view?.invasionen||[]).filter(i=>i.rolle==='verteidiger'||i.rolle==='verbuendeter');}
function angriffsText(){const a=view.angriffe.length,inv=laufendeInvasionen();if(inv.length)return '⚠ '+(inv[0].belagerung?(inv[0].regeln==='invasion-v2'?'Invasion':'Belagerung'):'Bombardierung')+' auf '+inv[0].planet+(inv.length>1?' (+'+(inv.length-1)+')':'')+(a?' · '+a+' Anflüge':'');return a?'⚠ '+a+' erkannte Angriffe':'Keine Angriffe erkannt';}
function itemRecords(category,p=currentPlanet()){
 const c=rules.catalog;if(!c)return [];
 if(category==='buildings')return Object.entries(c.gebaeude).map(([key,r])=>{const b=p.baubar.find(b=>b.gebaeude===key),level=p.gebaeude[key]||0;const job=p.bauschleife.find(j=>j.gebaeude===key);const needs=[...(r.ab_stufe>view.stufe?['Zivilisationsstufe '+r.ab_stufe]:[]),...(!b?Object.entries(r.braucht).filter(([k,n])=>(p.gebaeude[k]||0)<n).map(([k,n])=>`${named(k)} ${n}`):b.braucht.filter(x=>!String(x).startsWith('Felder voll')).map(named)),...(key==='xenoextraktor'&&!p.nebel&&!p.industrie?['Nebelsystem']:[])];
   if(!b&&!needs.length)needs.push('Maximalstufe erreicht');if(p.bauschleife.length>=p.bauschleife_plaetze)needs.push('Bauschleife voll');if(p.felder.belegt>=p.felder.gesamt)needs.push('Keine freien Baufelder');
   if(!p.industrie&&p.stabilitaet<c.wirtschaft.unruhen_unter&&!p.bauschleife.length)needs.push('Unruhen: zu geringe Stabilität');
   return {key,r,level,nextLevel:b?.stufe||level+1,job,cost:b?.kosten||buildingStageCosts(key,level+1)||Object.fromEntries(Object.entries(r.kosten).map(([g,n])=>[g,Math.floor(n*r.faktor**level)])),needs,missing:b?.fehlt||[],duration:b?.bauzeit_sekunden??b?.bauzeit_min*60,action:`data-build="${esc(key)}" data-planet="${esc(p.koord)}"`,verb:'Stufe '+(b?.stufe||level+1)+' ausbauen',effect:b?.ertrag?`+${fmt(b.ertrag.plus)} ${b.ertrag.art}`:''};});
 if(category==='research')return Object.entries(c.forschung).map(([key,r])=>{const f=view.forschung.moeglich.find(f=>f.forschung===key),level=view.forschung.stufen[key]||0,home=view.planeten.find(x=>x.heimat)||view.planeten[0];const needs=[];if(level>=c.welt.max_forschungsstufe)needs.push('Maximale Forschungsstufe erreicht');if(r.ab_stufe>view.stufe)needs.push('Zivilisationsstufe '+r.ab_stufe);if((home.gebaeude.labor||0)<r.labor)needs.push('Labor '+r.labor+' auf der Heimatwelt');const tor=c.forschung_voraussetzungen?.[key];for(const[k,n]of Object.entries(tor?.forschung||{}))if((view.forschung.stufen[k]||0)<n)needs.push(named(k)+' '+n);for(const[k,n]of Object.entries(tor?.gebaeude_reich||{}))if(!view.planeten.some(x=>(x.gebaeude[k]||0)>=n))needs.push(named(k)+' '+n);if(view.forschung.schlange.length>=c.wirtschaft.forschung_warteschlange)needs.push('Forschungsschlange voll');return {key,r,level,nextLevel:f?.stufe||level+1,job:view.forschung.aktiv?.forschung===key?view.forschung.aktiv:view.forschung.schlange.includes(key)?{wartet:true}:null,cost:f?.kosten||Object.fromEntries(Object.entries(r.kosten).map(([g,n])=>[g,Math.floor(n*r.faktor**level)])),stock:home.bestand,needs,missing:f?.fehlt||[],duration:f?.dauer_stunden*3600,action:`data-research="${esc(key)}"`,verb:'Stufe '+(f?.stufe||level+1)+' erforschen',effect:fmt(view.forschung.punkte_je_stunde)+' FP / Spielstunde'};});
 return Object.entries(c.einheiten).filter(([,e])=>e.schiff===(category==='ships')).map(([key,e])=>{const r=e.regel,q=view.einheiten_kosten.find(x=>x.einheit===key),needs=[];for(const[k,n]of Object.entries(e.taktik?.voraussetzungen||{}))if((view.forschung.stufen[k]||0)<n&&(view.entdeckungen?.bauplaene?.[key]||0)<1)needs.push(named(k)+' '+n);if(r.ab_stufe>view.stufe)needs.push('Zivilisationsstufe '+r.ab_stufe);if((p.gebaeude.werft||0)<Math.max(r.werft,q?.fertigungsplaetze_je_stueck||1))needs.push('Werft '+Math.max(r.werft,q?.fertigungsplaetze_je_stueck||1));for(const[k,n]of Object.entries(r.braucht))if((p.gebaeude[k]||0)<n)needs.push(named(k)+' '+n);if(r.max_anzahl>0&&(p.verteidigung[key]||0)>=r.max_anzahl)needs.push('Höchstbestand erreicht');
    // Schildfamilie wie Kern wirtschaft.rs fertigen: eine Stufe je Planet, keine zweite in Fertigung (Funktionsprüfung 25.10.2026).
    const schilde=['planetenschild','planetenschild_ii','planetenschild_iii'],si=schilde.indexOf(key);if(si>=0&&e.taktik&&!needs.includes('Höchstbestand erreicht')){if(schilde.slice(si).some(s=>(p.verteidigung[s]||0)>0))needs.push('Gleiche oder höhere Schildstufe aktiv');else if(p.fertigung.some(j=>schilde.includes(j.produkt)))needs.push('Ein Planetenschild ist bereits in Fertigung');}
   const cost=q?.kosten_je_planet?.[p.koord]||q?.kosten||e.kosten_je_volk?.[view.volk]||r.kosten,missing=Object.entries(cost).filter(([g,n])=>(p.bestand[g]||0)<n).map(([g])=>g);return {key,r,level:p.schiffe[key]||p.verteidigung[key]||0,job:p.fertigung.find(j=>j.produkt===key),cost,needs,missing,duration:q?.bauzeit_je_planet_sekunden?.[p.koord],action:`data-make="${esc(key)}"`,verb:'Fertigen',effect:`Grundwerte: Angriff ${fmt(r.angriff)} · Schild ${fmt(r.schild)} · Struktur ${fmt(r.struktur)}${r.ladung?' · Laderaum '+fmt(r.ladung):''}`};});
}
function constructionCard(x,costText,blocked,p){
 const names={deuteriumsynthesizer:'Deuterium\u00adsynthesizer',verwaltungszentrum:'Verwaltungs\u00adzentrum',forschungszentrum:'Forschungs\u00adzentrum',fusionskraftwerk:'Fusions\u00adkraftwerk',solarkraftwerk:'Solar\u00adkraftwerk',xenoraffinerie:'Xeno\u00adraffinerie'};
 const title=names[x.key]||named(x.key);
 const effect=x.effect?x.effect.replace(' je Stunde',' / h').replace('deuterium','Deuterium').replace('kristall','Kristall').replace('erz','Erz'):'';
 return `<article data-css="--construction-art:url(&quot;${esc(art('buildings',x.key))}&quot;)" class="construction-card ${x.needs.length?'locked':''}" data-catalog-item="${esc(x.key)}" data-era="${x.r.ab_stufe||1}" data-affordable="${!x.needs.length&&!x.job&&!gameReadOnly()&&Object.entries(x.cost).every(([g,v])=>(p.bestand[g]||0)>=v)}" data-ready="${!x.needs.length}" data-owned="${x.level>0}" data-running="${!!x.job}" data-search="${esc((named(x.key)+' '+x.r.wirkung).toLocaleLowerCase('de'))}"><button type="button" class="construction-art" data-info="${esc(x.key)}" data-category="buildings" aria-label="${esc(named(x.key))}: Datenblatt öffnen">${artShell('buildings',x.key,x.job)}</button><h3 class="construction-name">${esc(title)}</h3><div class="construction-meta"><span>Stufe <b>${x.level}</b>${x.job?` → ${x.job.stufe||x.level+1}`:''}</span><button type="button" class="tile-info" data-info="${esc(x.key)}" data-category="buildings" aria-label="${esc(named(x.key))}: Werte und Steigerungen">i</button></div><div class="construction-effect">${esc(effect)}</div><div class="construction-costs">${costText}</div>${x.needs.length?`<p class="construction-needs">${esc(x.needs.join(' · '))}</p>`:''}<div class="construction-action">${tileAction('buildings',x,1,blocked,p)}</div>${x.key==='akademie'&&x.level>0?'<button type="button" class="academy-open" data-open-academy>Fachkräfte ausbilden</button>':''}${constructionTree(x)}</article>`;
}
function renderCatalog(category,target,force=false){
 const host=$(target);if(!host||!view)return;
 // Seit den Designregeln (17.10.) zeichnet bauseite.js diese Seiten (Kachelraster + Detailfeld); der alte Katalog ruht.
 if(host.closest('.bauseite-panel')&&document.body.hasAttribute('data-rahmen'))return;
 if(host.closest('section.panel')?.hidden&&host.dataset.ready)return;
 if(!force&&host.contains(document.activeElement)&&document.activeElement.matches('input,select,textarea'))return;
 const p=currentPlanet(),items=window.Mobile035.catalogOrder(itemRecords(category,p),category);if(!items.length)return;
 let key=uiSelections[category];if(!items.some(x=>x.key===key))key=items.find(x=>x.job)?.key||items.find(x=>!x.needs.length)?.key||items[0].key;uiSelections[category]=key;
 const selected=items.find(x=>x.key===key),qty=['ships','defenses'].includes(category),amount=Number(uiSelections[category+'Qty'])||1;
 const selectedCanQueue=category==='buildings'||category==='research';
 const selectedBlocked=!!gameReadOnly()||selected.needs.length>0||!selectedCanQueue&&Object.entries(selected.cost).some(([g,n])=>((selected.stock||p.bestand)[g]||0)<n*amount);
 const cardList=items.flatMap((x,index)=>{
  const n=qty&&x.key===key?amount:1,stock=x.stock||p.bestand;
  const canQueue=category==='buildings'||category==='research';
  const blocked=!!gameReadOnly()||x.needs.length>0||!canQueue&&Object.entries(x.cost).some(([g,v])=>(stock[g]||0)<v*n)||qty&&(n>maxBuildable(x)||!Number.isInteger(n)||n<1);
  const progressText=x.job?(qty?' · '+x.job.rest+' im Auftrag':' → '+(x.job.stufe||x.level+1)):'';
  const queue=view.forschung.warteschlangen?.find(q=>q.art===(category==='buildings'?'bau':category==='research'?'forschung':'werft')&&(category==='research'||q.planet===p.koord)),queued=queue?.jobs.find(j=>j.key===x.key);
  const paid=queued?.aktiv||x.job&&!queued&&!x.job.wartet;
  const pendingCost=category==='buildings'&&queued?buildingStageCosts(x.key,queued.stufe):category==='research'&&queued?Object.fromEntries(Object.entries(x.r.kosten).map(([g,v])=>[g,Math.floor(v*x.r.faktor**(queued.stufe-1))])):null;
  const costText=paid?'Material bereits bezahlt':tileCostMarkup(pendingCost||Object.fromEntries(Object.entries(x.cost).map(([g,v])=>[g,v*n])),stock);
  const card=`<article data-catalog-item="${esc(x.key)}" data-era="${x.r.ab_stufe||1}" data-affordable="${!x.needs.length&&!x.job&&!gameReadOnly()&&Object.entries(x.cost).every(([g,v])=>(stock[g]||0)>=v*n)}" data-ready="${!x.needs.length}" data-owned="${x.level>0}" data-running="${!!x.job}" data-search="${esc((named(x.key)+' '+x.r.wirkung).toLocaleLowerCase('de'))}" data-css="--tile-art:url(&quot;${esc(art(category,x.key))}&quot;)" class="item-tile ${x.needs.length?'locked':''} ${x.key===key?'selected':''}"><header class="catalog-card-heading"><strong>${esc(named(x.key))}</strong><button type="button" class="tile-info" data-info="${esc(x.key)}" data-category="${category}" aria-label="${esc(named(x.key))}: Werte und Steigerungen">i</button></header><div class="tile-image"><button type="button" class="tile-select" ${category==='buildings'?'data-info':'data-item'}="${esc(x.key)}" data-category="${category}" aria-label="${esc(named(x.key))}: ${category==='buildings'?'Datenblatt öffnen':'auswählen'}">${artShell(category,x.key,x.job)}</button></div><div class="tile-copy"><small>${qty?'Bestand':'Stufe'} ${x.level}${esc(progressText)}</small>${x.effect?`<small class="tile-effect">${esc(x.effect)}</small>`:''}<div class="tile-costs">${costText}</div>${x.needs.length?`<small>${esc(x.needs.join(' · '))}</small>`:''}</div>${tileAction(category,x,n,blocked,p)}</article>`;
  return [...(category!=='ships'&&(index===0||items[index-1].r.ab_stufe!==x.r.ab_stufe)?[window.Mobile035.eraHeading(x.r.ab_stufe||1)]:[]),category==='buildings'?constructionCard(x,costText,blocked,p):card];
 }),cards=category==='ships'?Mobile035.shipGallery(items,cardList):cardList.join('');
 const heading=host.closest('.panel')?.querySelector('h2');if(heading&&!heading.querySelector('[data-go="auftraege"]')){heading.classList.add('catalog-title');heading.insertAdjacentHTML('beforeend','<button type="button" class="secondary" data-go="auftraege">Bauaufträge ↗</button>');}
 // Kacheln NUR neu aufbauen, wenn sich der Inhalt wirklich geaendert hat. Sonst wird bei jedem
 // Refresh der komplette DOM inkl. aller <img loading="lazy"> zerstoert und der WebView laedt und
 // dekodiert die Kachelbilder immer wieder neu -> sichtbares Nachladen und Flackern ohne Ende.
 const signature=JSON.stringify([category,key,amount,cards,selected.action,selectedBlocked,selected.level,selected.job?.rest??null,selected.needs,selected.missing,gameReadOnly()]);
 if(host.dataset.signature===signature)return;
 host.dataset.signature=signature;
 const rest=`<article class="item-detail"><div>${artShell(category,key,selected.job)}</div><div><h3>${esc(named(key))} · ${qty?'Serienfertigung':'Planung'}</h3><p>${esc(selected.r.wirkung)}</p>${window.Mobile035?.itemFacts(category,selected)||''}<div class="purchase-summary" data-purchase-category="${category}">${purchaseCosts(category,selected,selected.stock||p.bestand)}${requirementSheet(category,key)}</div><p data-order-duration="${category}">${orderDuration(category,selected)}</p><div class="section-actions">${qty?`<label>Stückzahl<input id="quantity-${category}" type="number" min="1" max="10000" value="${amount}" inputmode="numeric"></label><div class="qty-presets">${['10','50','100','max'].map(n=>`<button type="button" class="secondary" data-qty-preset="${n}" data-category="${category}">${n==='max'?'Max':n+'×'}</button>`).join('')}</div>`:''}<button type="button" ${selected.action} ${qty?'data-series-action':''} ${selectedBlocked?'disabled':''}>${qty?'Serie fertigen':selected.verb}</button><button type="button" class="secondary" data-info="${esc(key)}" data-category="${category}">Werte & Steigerungen</button></div></div></article>${queueControls(category)}`;
 // Reconcile values and actions without replacing the card or its decoded image.
 setStableHTML(host,window.Mobile035.catalogToolbar(category)+(category==='buildings'?queueControls(category):'')+`<div class="gallery">${cards}</div>`+(category==='buildings'||category==='ships'&&selected.needs.length&&!uiSelections.shipLockedDetail?'':rest));
 host.dataset.ready='true';window.Mobile035.catalogFilter(host,category);if(category==='buildings'){initializeConstructionGraphs(host);if($('construction-tree-dialog')?.open)renderConstructionLarge();}
 if(qty)$('quantity-'+category)?.dispatchEvent(new Event('input',{bubbles:true}));
 if(category==='research'&&$('research-path-inline')&&!$('forschen').hidden){Progression.target={kind:'research',key,level:selected.nextLevel||selected.level+1};renderResearchTree('research-path-inline');}
}

function renderProduction(){const p=currentPlanet();setStableHTML($('production'), queueControls('ships')+queueControls('components')+`<h3>Fertigungsaufträge · ${esc(p.koord)}</h3>${Object.entries(p.fertigungsplaetze).map(([k,s])=>`<p><b>${esc(named(k))}: ${s.belegt} / ${s.plaetze} Fertigungsplätze belegt</b> · ${s.frei} frei</p>`).join('')}`+(p.fertigung.length?p.fertigung.map(j=>`<article class="fleet-card"><strong>${esc(named(j.produkt))} · noch ${j.rest} Stück</strong><p class="detail-meta">${esc(named(j.schleife))} · ${j.parallel||0} gleichzeitig · ${j.plaetze_je_stueck} Plätze je Stück · nächste Fertigstellung</p>${jobLine(j)}</article>`).join(''):empty('Beide Fertigungsschleifen sind frei.'))+`<h3>Orbitalbauteile</h3><p class="muted">Habitatmodule sind Kolonieschiff-Bauteile. Ab Zivilisationsstufe III und Werft 3: 1 Recycler + 400 Legierung + 250 Elektronik → 1 Habitatmodul. Das kleine Kolonieschiff braucht keinen Antriebskern. Eine Orbitalwerft fertigt beide Bauteiltypen automatisch bis je 10 Stück Vorrat. Fertige Bauteile sind bezahlte Sonderfertigung und werden auch über der Lagergrenze eingelagert. Material wird beim Start bezahlt; fehlende Energie oder Besetzung pausiert die Fertigung. Unter Versorgung oder hier steuerst du die Orbitalwerft mit 0–120 %.</p>${window.Mobile035.localProduction('orbitalwerft',p)}${bauteileListe(p)}`);}
// Bauteile wie die Werft (Karl 24.10.2026): baubare offen, der Rest eingeklappt mit Grund. Regeln wie wirtschaft.rs fertigen:
// Antriebskern nur mit Orbitalwerft; Habitatmodul mit Orbitalwerft oder ab Stufe III mit Werft 3 (aus einem Recycler).
function bauteilGrund(g,p){if(p.gebaeude.orbitalwerft>0)return '';if(g==='antriebskern')return 'Orbitalwerft nötig';const f=[...((view.stufe||1)<3?['Stufe III']:[]),...((p.gebaeude.werft||0)<3?['Werft 3']:[])];return f.length?f.join(' · ')+' oder Orbitalwerft':'';}
function bauteileListe(p){
 const teile=['habitatmodul','antriebskern'],zu=teile.filter(g=>bauteilGrund(g,p));
 return teile.filter(g=>!zu.includes(g)).map(g=>`<h4>${esc(named(g))}</h4>${window.Mobile035.seriesForm('bauteil',g,p)}`).join('')+(zu.length?`<details class="bs-zugeklappt" id="bauteile-zu" data-bs-einklappen="bauteile"${window.Bauseite?.merkOffen?.('bauteile')?' open':''}><summary><span>Noch nicht baubar (${zu.length})</span><small>Grund je Bauteil</small></summary><ul class="bs-bauteil-gruende">${zu.map(g=>`<li><img src="${esc(art('resources',g,''))}" alt=""><b>${esc(named(g))}</b><span>${esc(bauteilGrund(g,p))}</span></li>`).join('')}</ul></details>`:'');
}
function renderEconomy(p){
 if(document.activeElement?.closest('#priority-form, #industry-controls'))return;
 setStableHTML($('kolonie-content'), `<p>${esc(p.koord)} · Vorräte, Produktion und Versorgung</p><div class="table-wrap">${table(['Rohstoff','Bestand','Lager','Pro Spielstunde','Reichweite / Lager voll'],Object.keys(p.bestand).map(g=>[imageTag('resources',g,'','planet-row-icon')+esc(named(g)),fmt(p.bestand[g]),fmt(p.lager[g]),`${p.rate[g]>=0?'+':''}${fmt(p.rate[g])}`,p.rate[g]<0?GameTime.duration(p.vorrat_reicht_sekunden?.[g]):p.voll_in_stunden[g]!=null?fmt(p.voll_in_stunden[g])+' Spielstunden':'—']))}</div><div class="stats">${stat(p.nahrung_deckung+' %','Nahrungsdeckung')}${stat(p.konsum_deckung+' %','Konsumdeckung')}${stat(p.stabilitaet_ziel+' %','Stabilitätsziel')}</div><p class="muted">Reichweite bei konstanten Raten. Wachstum, Bauten, Lieferungen und Kämpfe ändern diese Prognose.</p><h3>Arbeitsprioritäten</h3><p class="muted">Die zuerst genannten Gebäude werden zuerst mit Arbeitskräften versorgt.</p><form id="priority-form"><label>Gebäude (durch Komma getrennt)<input name="reihenfolge" value="${esc(p.prioritaeten.join(', '))}" required></label><button>Prioritäten speichern</button></form>`);
}
function renderFleets(){
 renderFleetHangar();
 renderMiningOpportunities();
 renderFleetRoutes();
 setStableHTML($('attacks'), '<h3>Erkannte Angriffe</h3>'+laufendeInvasionen().map(i=>`<article class="fleet-card warning"><strong>⚠ ${i.belagerung?(i.regeln==='invasion-v2'?'Invasion':'Belagerung'):'Bombardierung'} auf [${esc(i.planet)}]${i.rolle==='verbuendeter'?' · Verbündeter':''}</strong><p>${esc(i.angreifer||i.gegner||'Feindliche Flotte')} greift deine ${i.rolle==='verbuendeter'?'verbündete Welt':'Welt'} an. Miliz, Bodentruppen und Verteidigung steuerst du in der Lagekarte.</p><button type="button" data-tab="reich">Zur Lagekarte</button></article>`).join('')+(view.angriffe.length?view.angriffe.map(f=>`<article class="fleet-card warning"><strong>⚠ Kontakt #${f.flotte} → ${esc(f.ziel)}</strong><p>Ankunft: ${esc(f.ankunft)} · <span data-arrival="${esc(f.ankunft_sekunden)}"></span> Spielzeit</p><p>Von: <b class="koord">[${esc(f.start||'?')}]</b>${f.start_mond?' ☾ Mond':''} · ${f.von?`<button type="button" class="bs-link" data-akte="${esc(f.von)}">${esc(f.von)} · Akte</button>`:'Absender unbekannt'}${f.start?` <button type="button" class="bs-link" data-gt-ziel="${esc(f.start)}">Auf der Karte</button>`:''}<br>Schiffe: ${f.schiffe!=null?fmt(f.schiffe):f.schiffe_spanne?esc(f.schiffe_spanne.join('–')):'Unbekannt'} · Typen: ${f.schiffstypen?esc(goods(f.schiffstypen)):'Unbekannt'}</p>${f.sonde&&!f.sonde.rechtzeitig?`<button disabled>Flottensonde senden</button><small>Zu spät: eine Sonde aus ${esc(f.sonde.start)} käme erst nach dem Angriff an.</small>`:`<button data-probe="${esc(f.flotte)}">Flottensonde senden</button>`}<button class="secondary" data-save-target="${esc(f.ziel)}">Schiffe & Rohstoffe saven</button></article>`).join(''):empty('Kein Angriff erfasst. Ohne ausgebauten Geheimdienst liegt die Warnschwelle bei zwei Spielstunden; außerhalb der Sensorreichweite ist eine Flotte unbekannt.')));
}
function reichNotice(){
 if(world.beendet)return '<div class="defeat-notice" role="status"><strong>EPOCHE BEENDET</strong><p>Die Abschlussrangliste steht fest. Du kannst dein Reich und seine Berichte weiter ansehen. Neue Befehle sind erst in der nächsten Epoche möglich.</p></div>';
 if(world.paused)return '<div class="warning pause-notice" role="status"><strong>Welt pausiert</strong><p>Spielzeit und Aufträge stehen still.</p></div>';
 const state=view.reich_status;if(!state)return '';
 if(state.status==='besiegt')return `<div class="defeat-notice" role="status"><strong>BESIEGT · Für diese Epoche ausgeschieden</strong><p>${esc(state.grund)}. Dein Konto bleibt zum Zuschauen erhalten. Heimatwelt und Spielerplatz bleiben bis zum Epochenwechsel geschützt und belegt.</p></div>`;
 return (state.rettungsfristen||[]).map(k=>`<div class="warning"><strong>Existenzielle Krise: ${esc(k.grund)}</strong><p>Rettungsfrist: <span data-arrival="${k.frist}"></span> Spielzeit. Versorgung und Wiederaufbau jetzt sichern. Eine Erholung beendet diese Krise.</p></div>`).join('');
}
function renderFleetStock(){
 const p=view.planeten.find(x=>x.koord===$('fleet').elements.start.value);if(!p)return;if(window.Mobile035?.renderShipSelection){Mobile035.renderShipSelection(p);return;}
 $('ship-inputs').querySelectorAll('input').forEach(i=>{let note=i.parentElement.querySelector('.fleet-stock');if(!note){note=document.createElement('small');note.className='fleet-stock';note.id='fleet-stock-'+i.dataset.map;i.before(note);i.setAttribute('aria-describedby',note.id);i.setAttribute('aria-label',named(i.dataset.map));}const n=p.schiffe[i.dataset.map]||0;note.textContent='Vorhanden: '+fmt(n);i.max=String(n);});
}
function planetResources(p){
 const own=view.planeten.find(x=>x.koord===p.koord);
 if(own)return esc(Object.entries(own.faktoren).map(([g,n])=>named(g)+' '+fmt(n/10)+' %').join(' · '));
 if(!p.ertrag_promille)return 'Unbekannt';
 const factors=Object.entries(p.ertrag_promille).map(([g,n])=>named(g)+' '+[...new Set(n.map(v=>fmt(v/10)))].join('–')+' %').join(' · ');
 const stock=Object.entries(p.bestand_spannen||{}).map(([g,n])=>named(g)+' '+[...new Set(n.map(fmt))].join('–')).join(' · ');
 return esc(factors)+`<small class="map-observation">Bericht ${fmt(p.alter_stunden||0)} Spielstunden alt${stock?' · Vorräte: '+esc(stock):''}</small>`;
}
function galaxyReports(data){
 const systems=Array.isArray(data)?data:data.systeme||[];
 return systems.map(s=>`<article class="card"><div class="row"><h3>System ${esc(s.sektor??$('map-form').elements.sektor.value)}:${esc(s.system)}</h3>${badge(s.bekannt?'Kartiert':'Systemscan fehlt',s.bekannt?'good':'wait')}</div><button type="button" data-scan-system="${esc(s.sektor??$('map-form').elements.sektor.value)}:${s.system}:1">Ganzes System erkunden</button><p class="muted">Nebel: ${s.nebel==null?'Unbekannt':s.nebel?'Ja':'Nein'} · Asteroidengürtel: ${s.asteroidenguertel==null?'Unbekannt':s.asteroidenguertel?'Ja':'Nein'}</p><div class="table-wrap">${table(['Planet','Status','Typ','Bewohner','Ressourcenprofil','Aktion'],(s.plaetze||s.planeten||[]).map(p=>[`${p.bekannt&&p.zone?`<img class="planet-row-icon ${p.besiegt?'defeated-planet':''}" src="${esc(planetArt(p))}" alt="${esc(named(p.zone))}">`:''}${esc(p.koord)}`,badge(p.besiegt?'Besiegt'+(p.heimat?' · geschützte Heimat':''):named(p.status||'unbekannt'),p.besiegt?'defeated-name':p.bekannt?'good':'wait'),esc(named(p.zone||'unbekannt')),(p.bekannt||p.besiedlung_bekannt)&&p.spieler?relationHtml(p.spieler,p.beziehung):esc(p.status==='frei'||p.bewohnt===false?'Unbewohnt':'Unbekannt'),planetResources(p),`<button class="secondary" data-explore="${esc(p.koord)}" data-system="${!s.bekannt}">${s.bekannt?'Sonde':'Systemscan'}</button>${p.status==='frei'&&!p.heimat?` <button class="secondary" data-colonize="${esc(p.koord)}">Kolonisieren</button>`:''}`]))}</div><p class="muted">Beobachtungen sind historische Sondenberichte. Karte erneut ansehen, um neue Berichte zu laden.</p></article>`).join('')||empty('Keine Systeme im gewählten Bereich.');
}
function renderGalaxy(data){GalaxyUI.update(data);}
function renderColonies(p){
 const quotes=Object.fromEntries(['colony-quote','repair-quote'].map(id=>[id,$(id)?.innerHTML||'']));
 setStableHTML($('kolonien-content'), `${window.ColonyConsole?.management(p)||''}<div class="stats">${stat(`${view.planetenkapazitaet?.erlaubt??1+view.kolonien.erlaubt} / ${view.planetenkapazitaet?.maximal||15}`,view.kolonien.frei_basis?`Planeten freigeschaltet (Heimatwelt mitgezählt): Heimat + ${view.kolonien.frei_basis} ab Start + 1 je ${named(view.kolonien.forschung||'terraforming')}-Stufe, höchstens ${view.planetenkapazitaet?.maximal||15}`:`Planeten freigeschaltet (Heimatwelt mitgezählt) durch ${named(view.kolonien.forschung||'astrophysik')}`)}${stat(view.kolonien.verwaltungsgrenze,'Verwaltungsgrenze')}</div><p class="muted">${esc(rules.catalog.forschung[view.kolonien.forschung||'astrophysik']?.wirkung||'')} Verwaltungszentren sichern die Stabilität eines großen Reichs. Fremde Kolonien lassen sich durch Kampfkolonisation übernehmen; ursprüngliche Heimatwelten bleiben geschützt.</p><h3>Weg zur nächsten Kolonie</h3><ul class="conditions">${(view.kolonie_weg||[]).map(x=>`<li>${badge(x.erledigt?'Erfüllt':'Offen',x.erledigt?'good':'wait')} ${esc(x.text)}</li>`).join('')}</ul><p class="muted">Erst System und Zielplanet mit eigenen Sonden erkunden. Eine freie Kolonie braucht ein Kolonieschiff, bewaffnete Begleitung und die Startfracht für Minen, Farm, Solar und einen Tag Nahrung. Die feste Schiffsausstattung stellt Solar 1 und bei organischen Siedlern Farm 1 bereit. Weiteren Ausbau selbst planen. Die vollständigen 3.000/6.000 Siedler müssen aus einem geeigneten Volk stammen; gerettete oder erbeutete Bewohner können gewählt werden.</p><div class="section-actions"><button type="button" id="colony-cargo">Startfracht anzeigen</button><button type="button" class="secondary" data-mission="kolonisieren">Freie Kolonie planen</button><button type="button" class="secondary" data-mission="kampfkolonisieren">Kampfkolonisation planen</button></div><div id="colony-quote">${quotes['colony-quote']}</div><h3>Laufende Besetzungen</h3>${(view.kampfkolonisationen||[]).length?view.kampfkolonisationen.map(b=>`<article class="fleet-card"><strong>${esc(b.planet)}</strong><p>Verteidiger-Reaktionen: ${b.abgeschlossene_reaktionsfenster} / 2 · frühestens nach <span data-arrival="${esc(b.fruehestens)}"></span> Spielzeit</p><p>${esc(b.hinweis)}</p></article>`).join(''):empty('Keine eigene oder gegen dich gerichtete Besetzung.')}<h3>Integrität & Reparaturen · ${esc(p.koord)}</h3>${table(['Gebäude','Integrität','Status',''],Object.entries(p.gebaeude_integritaet_prozent||{}).map(([g,hp])=>[esc(named(g)),hp+' %',p.reparaturen.some(r=>r.gebaeude===g)?'Reparatur läuft':hp<100?'Beschädigt':'Intakt',hp<100&&!p.reparaturen.some(r=>r.gebaeude===g)?(()=>{const rp=window.Bauseite?.reparatur?.(p,g);return rp?.grund?`<button class="secondary" data-repair="${esc(g)}" disabled>Reparatur prüfen</button><small class="muted">${esc(rp.grund)}</small>`:`<button class="secondary" data-repair="${esc(g)}">Reparatur prüfen</button>`;})():'']))}<div id="repair-quote">${quotes['repair-quote']}</div><p class="muted">Kampfkolonisation braucht Orbitkontrolle und ein überlebendes Kolonieschiff. Gebäudeintegrität muss auf höchstens 30 % sinken. Mindestens 30 Spielminuten und zwei vollständige Verteidiger-Reaktionen verhindern eine sofortige Übernahme. Die ursprüngliche Heimatwelt bleibt geschützt.</p>`);
}
function renderCombat(){
 const result=$('combat-result')?.innerHTML||'';
 const can=document.activeElement?.closest('#combat-form');if(!can)setStableHTML($('kampf-content'), `<h3>Kampfsimulator</h3><p class="muted">Wählt deine Flotte aus dem Flottenformular und rechnet mit deinem letzten vollständigen Spionagebericht. Fehlende Schiff- oder Verteidigungsdaten werden nicht als Null behandelt.</p><button class="secondary" data-go="flotten">Schiffe & Ziel auswählen</button><button type="button" id="simulate-combat">Gewählte Flotte simulieren</button><div id="combat-result">${result}</div><h3>Verbandsangriffe</h3><p class="muted">Eröffne einen Verband für eine eigene Angriffsflotte. Verbündete können beitreten; die Engine prüft Ziel, Freundschaft und rechtzeitige Ankunft.</p>${recordCards(view.verbaende,'Angriffsverband')}<form id="combat-form"><label>Eigene Flotte<input name="flotte" type="number" min="1" required></label><label>Führungsflotte<input name="fuehrung" type="number" min="1" required></label><button>Verband beitreten</button></form><h3>Kampfberichte</h3>${recordCards(view.kampfberichte,'Kampfbericht')}<h3>Trümmerfelder</h3>${(view.truemmer||[]).length?table(['Ort','Erz','Kristall',''],view.truemmer.map(t=>[esc(t.koord),fmt(t.erz),fmt(t.kristall),`<button class="secondary" data-recycle="${esc(t.koord)}">Recycler planen</button>`])):empty('Noch keine bekannten Trümmerfelder.')}<p class="muted">${rules.catalog.kampf.runden} Kampfrunden · Schild regeneriert je Runde · Rapidfire je Einheit · Verteidigungswiederaufbau ${fmt(rules.catalog.kampf.verteidigung_wiederaufbau*100)} % · Trümmeranteil ${fmt(rules.catalog.kampf.truemmer_anteil*100)} %</p>`);
}
function renderEmpire(){const planets=view.planeten;setStableHTML($('imperium-content'), `<div class="table-wrap">${table(['Imperium',...planets.map(p=>p.koord),'Gesamt'],[
 ['Planet',...planets.map(p=>`<div class="empire-head"><img loading="lazy" decoding="async" src="${esc(planetArt(p))}" alt="${esc(named(p.zone))}"><br>${esc(named(p.zone))}</div>`),''],
 ['Bevölkerung',...planets.map(p=>fmt(p.bevoelkerung)),fmt(planets.reduce((n,p)=>n+p.bevoelkerung,0))],
 ...Object.keys(planets[0].bestand).map(g=>[esc(named(g)),...planets.map(p=>fmt(p.bestand[g])),fmt(planets.reduce((n,p)=>n+p.bestand[g],0))]),
 ...Object.keys(rules.catalog.gebaeude).map(g=>[esc(named(g)),...planets.map(p=>p.gebaeude[g]||'—'),planets.reduce((n,p)=>n+(p.gebaeude[g]||0),0)||'—'])])}</div><h3>Rangliste dieser Welt</h3>${table(['Rang','Reich','Punkte','Stufe'],world.rangliste.map(r=>r.map((v,i)=>i===1&&(world.ausgeschiedene||[]).some(d=>d.name===v)?`<span class="defeated-name">${esc(v)} · Ausgeschieden, Endstand Tag ${Math.floor(((world.ausgeschiedene||[]).find(d=>d.name===v)?.seit||0)/86400)+1}</span>`:esc(v))))}`);}
function renderUnlocks(){
 if(!$('progression-map'))setStableHTML($('freischaltungen-content'), '<div id="progression-map" class="research-tree"></div><details><summary>Alle Voraussetzungen als Liste</summary><div id="unlock-table"></div></details>');
 renderResearchTree('progression-map');
 setStableHTML($('unlock-table'), ['buildings','research','ships','defenses'].map(cat=>`<h3>${{buildings:'Gebäude',research:'Forschung',ships:'Schiffe',defenses:'Verteidigung'}[cat]}</h3><div class="table-wrap">${table(['Inhalt','Zivilisationsstufe','Voraussetzungen','Status'],itemRecords(cat).map(i=>[esc(named(i.key)),i.r.ab_stufe,esc(fullRequirements(cat,i.key).filter(x=>x.unit==='Stufe').map(x=>x.title+' '+x.need).join(' · ')),i.needs.length?esc(i.needs.join(' · ')):badge('Technisch freigeschaltet','good')]))}</div>`).join(''));
}
function renderProfile(){
 if(document.activeElement?.closest('#profile-details .display-preferences'))return;
 const t=factionTraits(view.volk);
 setStableHTML($('profile-details'), DisplayPreferences.markup()+`<div class="profile-card">${imageTag('portraits',view.volk,view.volk)}<div><h3>${esc(view.name)}</h3><p>${esc(named(view.volk))} · Spielweise: ${esc(named(session.mode))}<br>Rang ${view.rang} · ${fmt(view.punkte.gesamt)} Punkte<br>${view.anfaengerschutz_bis?'Anfängerschutz bis '+esc(view.anfaengerschutz_bis):'Kein Anfängerschutz aktiv'}</p></div></div><p class="plus">${esc(t.pros.join(' · '))}</p><p class="minus">${esc(t.cons.join(' · '))}</p><p class="muted">${esc(t.special.join(' '))} Die Volkswahl bleibt für die laufende Epoche fest.</p>`);
}
function renderGuide(){
 const c=rules?.catalog;if(!c||!world)return;
 const sections=[['Dein erster Ausbau','Sichere Energie, Nahrung und Wohnraum, bevor du weitere Produktionsgebäude ausbaust. Die Rohstoffleiste zeigt die aktuell ausgewählte Welt. Rot markierte Kosten sind noch nicht bezahlbar. In der Versorgung siehst du negative Raten und die Reichweite deiner Vorräte.'],['Bau, Forschung & Werft','Die aktive Kachel wird von unten nach oben farbig. Mittig stehen die verbleibende Echtzeit und der Fortschritt. Der Bauknopf pausiert oder setzt fort; das X bricht den Auftrag ab. Ein neuer Bau wartet hinter dem laufenden Auftrag; Kosten werden bei Baubeginn bezahlt. Forschung und Fertigstellungen werden zum exakten Zeitpunkt verarbeitet. Die Werft zeigt das nächste Stück eines Auftrags. Bei Pause frieren Spielzeit und Timer ein.'],['Aufklärung & Angriffe','Ein eigener Systemscan erschließt das Sonnensystem. Jeder Planet braucht anschließend eine eigene Spionagesonde, damit Typ, Bewohner und Ressourcen bekannt werden. Ohne Ausbau erscheint ein erfasster Angriff erst zwei Spielstunden vor Ankunft. Geheimdienst, Überwachungstechniken und Spionagetechnik erweitern Reichweite und Detailtiefe. Die genaue Ankunft ist bekannt, sobald die Sensoren die Flotte erfassen. Eine Flottensonde kann die Zusammenstellung prüfen; Abschirmung kann dies verhindern. Überlebende Sonden kehren nach System-, Planeten- und Flottenaufklärung zurück. Hin- und Rückflugtreibstoff werden am Start reserviert. Nach der Landung sind die Sonden wieder verfügbar.'],['Flotten saven','Wähle den Auftrag Saven, Ziel und Wartezeit. „Alle vorhandenen Schiffe“ und „Save-Fracht automatisch verladen“ sichern deine Auswahl; die Frachtplanung reserviert Treibstoff für beide Strecken. Prüfe Hinflug, Aufenthalt und Rückkehr. Beim Rückflug können eine verlorene Landewelt oder Blockaden die Ankunft verändern. Rückruf und Rückkehr musst du selbst beobachten.'],['Kampfsystem',`Die Engine rechnet jede Einheit einzeln über bis zu ${c.kampf.runden} Runden. Ziele werden pro Schuss gewählt; Schilde regenerieren pro Runde. Treffer unter ${fmt(c.kampf.verpuffen_anteil*100)} % des vollen Schilds verpuffen. Unter ${fmt(c.kampf.explosion_unter*100)} % Reststruktur entsteht eine Explosionschance. Rapidfire kann weitere Schüsse auslösen. Waffen, Schilde, Panzerung und Volksboni beeinflussen den Ausgang. Der Planetenschild ist eine eigene Einheit. Der Simulator nutzt historische Berichte und kennzeichnet geschätzte Technik.`],['Ausscheiden & Epochenwechsel',`Aktuelle Rettungsfristen: ${world.ausscheiden_regeln?.versorgung_stunden??72} Spielstunden bei Unterversorgung und ${world.ausscheiden_regeln?.stillstand_stunden??48} bei wirtschaftlichem Stillstand. ${c.ausscheiden}`],['Kolonisation & Übernahme',c.kolonisation],['Menschen, Agenten & Rollen','Ein Konto spielt selbst (Mensch), lässt Modelle alle vier Rollen führen (Agent) oder teilt sich die Rollen mit ihnen (Gemischt). Die vier Rollen heißen Stratege, Verwalter, Feldherr und Diplomat. Modelle steuern entweder als Team hier im Browser unter Modelle & Team – Ollama-Aufrufe laufen auf deinem PC, OpenRouter-Aufrufe mit deinem Schlüssel im Browser – oder als eigener Harness: das Harness-Cockpit auf deinem PC oder ein eigenes Programm mit Harness-Schlüssel. Je Reich steuert immer nur einer dieser Wege, mit einer zeitlich begrenzten Steuerungsfreigabe. Stoppen gibt dir die Steuerung zurück. Alle Befehle werden vom Server geprüft.'],['Anmeldung & Warteliste','Die Alpha-Welt gibt zunächst fünf der zwanzig möglichen Spielerplätze frei. Ein Konto kann zuschauen und seine Volkswahl vor dem Start vergleichen. Bei voller Freigabe zählt die Reihenfolge der Warteliste. Wer aufgenommen wird, erhält sein Reich auch bei geschlossenem Browser; Serverbots und Wirtschaft laufen auf dem Host weiter.']];
 setStableHTML($('player-guide'), `<div class="guide-grid">${sections.map(([t,p])=>`<article><h3>${t}</h3><p>${esc(p)}</p></article>`).join('')}</div>`+communicationFAQ());
}
// Polls never discard an in-progress selection or input. Commands stay in app.js.
document.addEventListener('input',e=>{if(e.target.id?.startsWith('quantity-')){const category=e.target.id.slice(9);uiSelections[category+'Qty']=e.target.value;const item=itemRecords(category).find(i=>i.key===uiSelections[category]),node=document.querySelector('[data-purchase-category='+category+']');if(item&&node){setStableHTML(node, purchaseCosts(category,item,currentPlanet().bestand)+requirementSheet(category,item.key));const duration=document.querySelector('[data-order-duration='+category+']');if(duration)duration.textContent=orderDuration(category,item);const qty=Number(e.target.value),bad=!Number.isInteger(qty)||qty<1||qty>10000||fullRequirements(category,item.key).some(r=>!r.ok)||Object.entries(item.cost).some(([g,n])=>(currentPlanet().bestand[g]||0)<n*qty);const blocked=bad||!!gameReadOnly()||item.needs.length>0||qty>maxBuildable(item);node.closest('.item-detail').querySelector('[data-make]').disabled=blocked;const tile=node.closest('[id]').querySelector('.item-tile.selected');if(tile&&tile.querySelector('[data-make]')){const button=tile.querySelector('[data-make]');button.dataset.orderQuantity=qty;button.textContent=qty+'× Fertigen';button.disabled=blocked;tile.querySelector('.tile-costs').textContent=goods(Object.fromEntries(Object.entries(item.cost).map(([g,n])=>[g,n*qty])));}}}});
document.addEventListener('change',e=>{if(e.target===$('fleet').elements.start&&view)renderFleetStock();if(e.target.id==='progress-motion'){document.body.dataset.progressMotion=e.target.value;localStorage.setItem('sternenepoche-progress-motion',e.target.value);}if(e.target=== $('claim').elements.volk){chosenRace=e.target.value;renderShell();}});
document.addEventListener('click',guard(async e=>{
 const b=e.target.closest('[data-race],[data-planet-pick],[data-item],[data-go],[data-make],[data-repair],[data-open-group],[data-recycle],[data-mission],[data-save-target],[data-colonize],#colony-cargo,#simulate-combat');if(!b)return;
 if(b.dataset.race){chosenRace=b.dataset.race;$('claim').elements.volk.value=chosenRace;renderShell();return;}
 if(b.dataset.go){navigateGame(b.dataset.go);if(b.dataset.go==='flotten')$('fleet-planner').open=true;return;}
 if(b.dataset.planetPick){PlanetNavigation.select(b.dataset.planetPick);return;}
 if(b.dataset.item){if(b.dataset.category==='ships')uiSelections.shipLockedDetail=true;uiSelections[b.dataset.category]=b.dataset.item;const target={buildings:'gebaeude-content',research:'research',ships:'ship-gallery',defenses:'verteidigung-content'}[b.dataset.category];renderCatalog(b.dataset.category,target);$(target).querySelector('.item-detail').scrollIntoView({block:'start'});tickGame();return;}
 if(b.dataset.make){const category=rules.catalog.einheiten[b.dataset.make].schiff?'ships':'defenses',qty=Number(b.dataset.orderQuantity??$('quantity-'+category).value);if(!Number.isInteger(qty)||qty<1||qty>10000)throw Error('Stückzahl muss zwischen 1 und 10.000 liegen.');await api('/api/tool',{typ:'kosten',planet:currentPlanet().koord,einheit:b.dataset.make});await command([{typ:'fertigen',planet:currentPlanet().koord,einheit:b.dataset.make,anzahl:qty}]);return;}
 if(b.dataset.repair){navigateGame('kolonien');const q=await api('/api/tool',{typ:'kosten',planet:currentPlanet().koord,gebaeude:b.dataset.repair});if(!q.reparatur)throw Error('Keine Reparaturquote verfügbar.');setStableHTML($('repair-quote'), `<article class="card"><h3>Reparieren: ${esc(named(b.dataset.repair))}</h3>${costHtml(q.reparatur.kosten,currentPlanet().bestand)}<p>Dauer ${GameTime.duration(q.reparatur.dauer_sekunden)} Spielzeit</p><button data-confirm-repair="${esc(b.dataset.repair)}">Reparatur starten</button></article>`);return;}
 if(b.dataset.openGroup){await command([{typ:'verband_oeffnen',flotte:Number(b.dataset.openGroup)}]);return;}
 if(b.dataset.recycle){pickFleet(b.dataset.recycle,'recyceln');return;}
 if(b.dataset.colonize){pickFleet(b.dataset.colonize,'kolonisieren');return;}
 if(b.dataset.mission){navigateGame('flotten');$('fleet-planner').open=true;$('fleet').elements.mission.value=b.dataset.mission;return;}
 if(b.dataset.saveTarget){navigateGame('flotten');$('fleet-planner').open=true;$('fleet').elements.start.value=b.dataset.saveTarget;$('fleet').elements.mission.value='saven';selectAllShips();return;}
 if(b.id==='colony-cargo'){const identity=planIdentity(),origin=$('fleet').elements.siedler_volk?.value||null,large=Number($('ship-inputs').querySelector('[data-map="grosses_kolonieschiff"]')?.value)>0;const q=await api('/api/tool',{typ:'koloniebedarf',volk:origin,einheit:large?'grosses_kolonieschiff':'kolonieschiff'});if(identity!==planIdentity()||origin!==($('fleet').elements.siedler_volk?.value||null)||large!==(Number($('ship-inputs').querySelector('[data-map="grosses_kolonieschiff"]')?.value)>0))return;setStableHTML($('colony-quote'), `<article class="card"><h3>Startfracht · ${esc(named(q.einheit))} · ${fmt(q.siedler)} ${esc(named(q.volk))}</h3>${costHtml(q.fracht,currentPlanet().bestand)}<p>${esc(q.hinweis)}</p><p>Herkunft und Schiffsgröße im Flottenformular auswählen.</p></article>`);return;}
 if(b.id==='simulate-combat'){const a=fleetData(),q=await api('/api/tool',{typ:'kampfsimulator',ziel:a.ziel,schiffe:a.schiffe});setStableHTML($('combat-result'), `<article class="card"><h3>Simulation: ${esc(a.ziel)}</h3><div class="stats">${stat(q.siegchance_prozent+' %','Siegchance')}${stat(q.unentschieden_prozent+' %','Unentschieden')}${stat(fmt(q.eigene_verluste_wert),'Eigene Verlustwerte')}${stat(fmt(q.verluste_gegner_wert),'Gegnerische Verlustwerte')}</div><p>${q.laeufe} Läufe · Bericht ${q.bericht_alter_stunden} Spielstunden alt · Gegnertechnik: ${esc(q.technik_des_gegners)}</p><p>Durchschnittliche Beute nach Restkapazität: ${esc(q.beute_mittel_milli?goods(Object.fromEntries(q.kontext.gueter_reihenfolge.map((g,i)=>[g,q.beute_mittel_milli[i]/1000]))):q.beute_status)}</p><p class="muted">Eine Simulation garantiert keinen Ausgang. Seit der Beobachtung können Flotten, Verteidigung und Technik geändert worden sein.</p></article>`);}
}));
document.addEventListener('click',guard(async e=>{const b=e.target.closest('[data-confirm-repair]');if(b)await command([{typ:'reparieren',planet:currentPlanet().koord,gebaeude:b.dataset.confirmRepair}]);}));
document.addEventListener('submit',guard(async e=>{if(e.target.id==='priority-form'){const list=e.target.elements.reihenfolge.value.split(',').map(s=>s.trim()).filter(Boolean);await command([{typ:'prioritaeten',planet:currentPlanet().koord,reihenfolge:list}]);}if(e.target.id==='combat-form')await command([{typ:'verband_beitreten',flotte:Number(e.target.elements.flotte.value),fuehrung:Number(e.target.elements.fuehrung.value)}]);}));
setInterval(tickGame,250);

// Persistent resource instruments: animate only differences between confirmed snapshots.
// Planet/account/world changes establish a fresh baseline and never simulate income.
const ResourceDisplay={
 keys:['erz','kristall','deuterium','nahrung','energie','credits'],
 producers:{erz:['erzmine'],kristall:['kristallmine'],deuterium:['deuteriumsynthesizer'],nahrung:['farm'],energie:['solarkraftwerk','fusionskraftwerk']},
 state:new Map(),identity:'',selected:'',motion:matchMedia('(prefers-reduced-motion: reduce)'),
 quiet(){return this.motion.matches||document.body.dataset.progressMotion==='still'||document.hidden;},
 // Zahlen immer vollständig mit Tausenderpunkten (DESIGNREGELN §5), Vorräte ganzzahlig.
 format(n){return Math.floor(Number(n)||0).toLocaleString('de-DE');},
 reset(){for(const s of this.state.values()){cancelAnimationFrame(s.frame);clearTimeout(s.timer);}this.state.clear();this.identity='';this.selected='';},
 inspect(g,returnFocus=false){
  const previous=this.selected;this.selected=this.state.has(g)?g:'';this.updateInspector();
  if(returnFocus&&previous)this.state.get(previous)?.node.querySelector('.resource-hit').focus();
 },
 updateInspector(){
  const panel=$('resource-inspector');if(!panel)return;
  for(const [g,s] of this.state)s.node.querySelector('.resource-hit').setAttribute('aria-expanded',String(g===this.selected));
  const s=this.state.get(this.selected);panel.hidden=!s;if(!s)return;
  panel.setAttribute('aria-label',named(this.selected)+' – genaue Ressourcenwerte');
  const g=this.selected,p=currentPlanet(),keys=this.producers[g]||[];
  panel.style.setProperty('--detail-art',`url("${art('resources',g)}")`);
  panel.querySelector('.resource-inspector-name').textContent=named(g)+' · '+(g==='credits'?'Imperium':p.koord);
  panel.querySelector('.resource-inspector-rate').textContent=g==='energie'?s.node.querySelector('.resource-rate').title:g==='credits'?'Steuern und Handel finanzieren dein Imperium':`${workforceRate((p.rate?.[g]||0)*(world.tempo||1))} / Echtzeit-h`;
  panel.querySelector('.resource-inspector-detail').textContent=s.cap>0?`${fmt(s.value)} / ${fmt(s.cap)} im Lager`:g==='credits'?`${fmt(s.value)} Credits verfügbar`:s.value<0?'Versorgungsdefizit':'Versorgung gesichert';
  const flow=panel.querySelector('.resource-flow'),industry=p.industrie,idx=industry?.gueter?.indexOf(g)??-1;
  if(flow&&industry&&g!=='credits'){
   const works=industry.werke.filter(w=>w.stufe>0),energy=g==='energie';
   const rows=works.map(w=>({w,produced:energy?w.energie_vor_versorgungsnetz/1000:(w.produktion[idx]||0)/1000,used:energy?w.energiebedarf/1000:(w.verbrauch[idx]||0)/1000})).filter(r=>r.produced||r.used||keys.includes(r.w.gebaeude));
   const gross=rows.reduce((n,r)=>n+r.produced,0),used=rows.reduce((n,r)=>n+r.used,0),net=p.rate[g]||0;
   const reason=energy?(p.energie.erzeugung<p.energie.verbrauch?'Der Bedarf übersteigt die Erzeugung. Verbraucher reduzieren oder Kraftwerke versorgen.':'Die Stromversorgung ist gedeckt.'):s.cap>0&&s.value>=s.cap?'Lager voll: Für weitere Förderung erst Platz schaffen.':gross>0&&net<=0?'Die Förderung fließt vollständig in Verbrauch und Weiterverarbeitung. Im Lager bleibt kein Überschuss.':gross<=0?'Keine Förderung: Gebäudestufe, Leistungsregler, Personal und Energie prüfen.':'Der Überschuss wird dem Lager gutgeschrieben. Bau und andere Aufträge bezahlen daraus ihre Kosten.';
   setStableHTML(flow,`<p class="resource-flow-summary">${energy?`${fmt(p.energie.erzeugung)} erzeugt − ${fmt(p.energie.verbrauch)} gebunden`:`${fmt(gross)} gefördert − ${fmt(used)} in Betrieben verbraucht / Spiel-h`}<br><b>${energy?workforceRate(p.energie.erzeugung-p.energie.verbrauch)+' Energie':workforceRate(net)+' netto / Spiel-h · '+workforceRate(net*(world.tempo||1))+' / Echtzeit-h'}</b></p><p>${reason}</p><ul>${rows.map(({w,produced,used})=>`<li><span>${esc(named(w.gebaeude))} <small>Stufe ${w.stufe} · ${industry.regler[w.gebaeude]??100} % Leistung${w.wirksame_besetzung_promille<1000?' · Besetzung '+fmt(w.wirksame_besetzung_promille/10)+' %':''}</small></span><b>${produced?'+'+fmt(produced):''}${produced&&used?' / ':''}${used?'−'+fmt(used):produced?'':'0'}</b></li>`).join('')}</ul><button type="button" data-go="kolonie">Produktion & Versorgung regeln ↗</button>`);
  }else if(flow)setStableHTML(flow,'');
  const stamp=JSON.stringify([g,p.koord,keys.map(k=>[k,p.gebaeude[k]||0]),rules.catalog.ausbaustufen]);
  if(panel.dataset.stamp!==stamp){
   panel.dataset.stamp=stamp;
   setStableHTML(panel.querySelector('.resource-expansion'), keys.length?keys.map(k=>{const r=rules.catalog.gebaeude[k];return r?buildingProgression({key:k,r,level:p.gebaeude[k]||0}):'';}).join(''):`<p>Verkaufe Überschüsse auf dem Markt oder verwalte die Steuern deiner Kolonien.</p><button type="button" class="ui-action" data-go="aktionen"><span>Zum Markt</span>${uiIcon('arrow')}</button>`);
  }

 },
 settle(){for(const s of this.state.values()){cancelAnimationFrame(s.frame);clearTimeout(s.timer);s.display=s.value;s.node.querySelector('.resource-value').textContent=this.format(s.value);s.node.classList.remove('resource-changed','resource-burst');}},
 effect(g){
  if(g==='energie')return '<svg class="resource-lightning" viewBox="0 0 120 100"><path d="M76 0 48 35 69 35 32 90 45 49 24 49 48 0"/><path class="bolt-branch" d="m93 22-13 25 15-2-25 37 8-29-12 2"/></svg>';
  return '<span class="resource-ring"></span>'+'<i class="resource-particle"></i>'.repeat(6);
 },
 render(p,v,w){
  // Je Gut ein Bild und eine Menge (R3): alle Lagergüter des Planeten, dann Energie als Saldo und Credits.
  const keys=[...['erz','kristall','deuterium','nahrung','legierung','elektronik','konsumgut','xenokristall'].filter(g=>g in (p.bestand||{})),'energie','credits'];if(keys.join()!==this.keys.join()){this.reset();this.keys=keys;}
  const bar=$('resource-bar'),identity=JSON.stringify([w.world_id,session?.spieler,v.name,p.koord]);
  if(this.identity!==identity||!bar.querySelector('.resource-value')){
   this.reset();this.identity=identity;
   bar.setAttribute('aria-label','Ressourcen des ausgewählten Planeten und Imperium-Credits');
   setStableHTML(bar, this.keys.map(g=>`<article class="resource" data-resource="${g}" aria-label="${esc(named(g))}"><span class="resource-art" aria-hidden="true"></span><span class="resource-sheen" aria-hidden="true"></span><div class="resource-icon">${imageTag('resources',g,'')}<span class="resource-effect" aria-hidden="true">${this.effect(g)}</span></div><small class="resource-name">${esc(named(g))}</small><strong class="resource-value"></strong><span class="resource-delta" aria-hidden="true"></span><span class="resource-rate"></span><span class="resource-meter" aria-hidden="true"><i></i></span><span class="resource-detail"></span><span class="resource-warning" aria-hidden="true"></span><button class="resource-hit" type="button" data-resource-inspect="${g}" aria-expanded="false" aria-controls="resource-inspector" aria-label="${esc(named(g))} – Details anzeigen"></button></article>`).join('')+`<div id="resource-inspector" class="resource-inspector" role="region" hidden><header class="resource-inspector-head"><div class="resource-inspector-meta"><strong class="resource-inspector-name"></strong><span class="resource-inspector-rate"></span><span class="resource-inspector-detail"></span></div><button type="button" class="ui-icon-button" data-resource-close aria-label="Rohstoffdetails schließen">${uiIcon('close')}</button></header><div class="resource-flow"></div><div class="resource-expansion"></div></div>`);
   // Bildvariable über CSSOM: die Server-CSP (style-src 'self') sperrt style-Attribute im Markup.
   for(const n of bar.querySelectorAll('[data-resource]'))n.style.setProperty('--resource-art',`url("${art('resources',n.dataset.resource)}")`);
  }
  for(const g of this.keys){
   const node=bar.querySelector(`[data-resource="${g}"]`),value=Number(g==='energie'?p.energie.erzeugung-p.energie.verbrauch:g==='credits'?v.credits:p.bestand[g])||0;
   const levels=Object.fromEntries((this.producers[g]||[]).map(k=>[k,p.gebaeude?.[k]||0]));
   let s=this.state.get(g);const fresh=!s;
   if(fresh){s={node,value,display:value,levels,frame:0,timer:0,nextEffect:performance.now()+this.keys.indexOf(g)*1800};this.state.set(g,s);}
   const delta=value-s.value,completed=Object.keys(levels).some(k=>levels[k]>(s.levels[k]||0));
   const rate=Number(p.rate?.[g])||0,cap=Number(p.lager?.[g])||0,energy=g==='energie';
   const deficit=energy?value<0:g!=='credits'&&rate<0;
   node.classList.toggle('resource-deficit',deficit);node.classList.toggle('resource-full',!energy&&cap>0&&value>=cap);node.classList.toggle('resource-large',Math.abs(value)>=10000);node.classList.toggle('resource-huge',Math.abs(value)>=1e8);node.classList.toggle('resource-giant',Math.abs(value)>=1e9);
   node.dataset.direction=delta<0?'down':'up';s.cap=cap;
   node.querySelector('.resource-warning').textContent=deficit?'!':cap>0&&value>=cap?'↑':'';
   // Volles Lager: Produktion ruht (Verarbeitungswerke stehen, der Rest verfällt) – nicht weiter „+X/h“ zeigen.
   const voll=!energy&&g!=='credits'&&cap>0&&value>=cap&&rate>0;
   const fullRate=energy?`${fmt(p.energie.erzeugung)} erzeugt · ${fmt(p.energie.verbrauch)} Bedarf`:g==='credits'?'Imperiumskasse':voll?`Lager voll – Produktion ruht (${fmt(rate)} / Spielstunde gingen verloren; Lager ausbauen, verbrauchen oder abtransportieren)`:`${rate>=0?'+':''}${fmt(rate)} / Spielstunde`;
   node.querySelector('.resource-rate').title=fullRate;
   node.querySelector('.resource-rate').textContent=energy?`${fmt(p.energie.erzeugung)} / ${fmt(p.energie.verbrauch)}`:g==='credits'?'Imperium':voll?'Lager voll':`${rate>=0?'+':''}${fmt(rate)} / h`;
   node.querySelector('.resource-detail').textContent=energy?(deficit?'⚠ Energiedefizit':'Versorgung gesichert'):g==='credits'?'Reichsweit verfügbar':cap>0?`${fmt(Math.min(100,Math.max(0,value/cap*100)))} % Lager${deficit?' · Verbrauch':value>=cap?' · voll':''}`:'Kein Lagerlimit';
   const fill=energy?(p.energie.verbrauch>0?Math.min(1,p.energie.erzeugung/p.energie.verbrauch):1):cap>0?Math.min(1,Math.max(0,value/cap)):0;
   node.querySelector('.resource-meter').hidden=g==='credits'||(!energy&&!cap);
   const fillR=Math.round(fill*1000)/1000;
   const meter=node.querySelector('.resource-meter i');
   if(meter.dataset.fill!==String(fillR)){meter.dataset.fill=String(fillR);meter.style.transform=`scaleX(${fillR})`;}
   node.title=`${named(g)}: ${fmt(value)}. ${fullRate}. ${energy?(deficit?'Energie fehlt.':'Energie reicht aus.'):cap>0?'Lagerkapazität: '+fmt(cap):'Gilt für dein gesamtes Imperium.'}`;
   node.setAttribute('aria-label',node.title);
   node.querySelector('.resource-hit').setAttribute('aria-label',node.title+' Details anzeigen.');
   s.value=value;s.levels=levels;
   // Polling changes numbers, never flashes cards or replays decorative income effects.
   cancelAnimationFrame(s.frame);clearTimeout(s.timer);s.display=value;
   const label=node.querySelector('.resource-value'),text=this.format(value);
   if(label.textContent!==text)label.textContent=text;
   node.classList.remove('resource-changed','resource-burst');

  }
  this.updateInspector();
 }
};
ResourceDisplay.motion.addEventListener('change',()=>{if(ResourceDisplay.quiet())ResourceDisplay.settle();});
document.addEventListener('visibilitychange',()=>{if(document.hidden)ResourceDisplay.settle();});
document.addEventListener('change',e=>{if(e.target.id==='progress-motion'&&ResourceDisplay.quiet())ResourceDisplay.settle();});
document.addEventListener('click',e=>{
 const trigger=e.target.closest('[data-resource-inspect]');
 if(trigger){const g=trigger.dataset.resourceInspect;ResourceDisplay.inspect(ResourceDisplay.selected===g?'':g);}
 else if(e.target.closest('[data-resource-close]'))ResourceDisplay.inspect('',true);
});
document.addEventListener('keydown',e=>{
 if(e.key==='Escape'&&ResourceDisplay.selected&&!$('resource-inspector').hidden){ResourceDisplay.inspect('',true);e.preventDefault();}
});
