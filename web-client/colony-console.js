'use strict';

// Navigation and comparison share the same selected planet as every build command.
window.ColonyConsole=(()=>{
 let owner='',mode='planet',query='',onlyProblems=false,contextStamp='';
 const label=p=>(p.name?p.name+' · ':'')+(p.heimat?'Heimatwelt':named(p.zone)+' · Kolonie');
 const energy=p=>p.energie.erzeugung-p.energie.verbrauch;
 function issues(p){
  const result=[];
  if(energy(p)<0)result.push('Strommangel');
  if(window.Mobile035.organic(p)&&p.rate.nahrung<0)result.push('Nahrung sinkt');
  if(p.arbeit.verfuegbar<p.arbeit.bedarf)result.push('Arbeitskräfte fehlen');
  if(p.fachkraefte.verfuegbar<p.fachkraefte.bedarf)result.push('Fachkräfte fehlen');
  if(p.bevoelkerung>=p.wohnraum*.9)result.push('Wohnraum knapp');
  if(p.stabilitaet<40)result.push('Unruhen');
  return result;
 }
 function ensure(){
  const key=world.world_id+':'+view.name;if(key===owner)return;
  owner=key;query='';onlyProblems=false;contextStamp='';
  try{mode=localStorage.getItem('sternenepoche-supply:'+owner)==='all'?'all':'planet';}catch{mode='planet';}
 }
 function setMode(value){mode=value==='all'?'all':'planet';try{localStorage.setItem('sternenepoche-supply:'+owner,mode);}catch{}}
 function onPlanetChange(){setMode('planet');}
 function context(){
  const host=$('colony-context');if(!host)return;
  const p=currentPlanet(),planets=view.planeten,index=planets.findIndex(x=>x.koord===p.koord);
  const stamp=JSON.stringify([p.koord,planets.map(x=>[x.koord,x.heimat,x.zone,x.mond?.name||'',x.mond?.typ||'']),document.querySelector('#tabs [aria-selected="true"]')?.dataset.tab]);
  // Mond des aktiven Planeten mit einem Tipp (Karl 25.10.2026: „in jeder Boxgröße immer der Mond des Planeten anwählbar … sonst
  // zu viele Klicks drumrum, wenn ich auf anderen Planeten bin“); am Desktop trägt die Seitenleiste je Planet ihren Mondknopf.
  const m=p.mond,mondKnopf=m?`<button type="button" class="kontext-mond" data-mond="${esc(p.koord)}" aria-label="Mond ${esc(m.name||'')} von ${esc(p.koord)} öffnen">${window.Mondseite?Mondseite.bild(m,'kontext-mond-bild'):'☾'}<span>Mond</span></button>`:'';
  if(stamp!==contextStamp){
   const focused=host.contains(document.activeElement)?document.activeElement?.getAttribute('data-colony-step')||'picker':null;
   setStableHTML(host, `<button id="context-planet" type="button" class="planet-context-trigger" data-open-planets aria-haspopup="dialog" aria-expanded="false" aria-controls="planet-picker" aria-label="Aktiver Planet ${esc(p.koord)} · Planet wechseln"><span class="planet-portrait"><img src="${esc(planetArt(p))}" alt=""></span><span class="planet-trigger-copy"><span class="planet-trigger-meta"><span>Aktiver Planet</span><b>${index+1} / ${planets.length}</b></span><span class="planet-trigger-name"><strong>${esc(p.koord)}</strong><span>${esc(label(p))}</span></span></span>${uiIcon('down')}</button><div class="colony-stepper"><button type="button" class="ui-icon-button" data-colony-step="-1" aria-label="Vorheriger Planet" ${index===0?'disabled':''}>${uiIcon('left')}</button><button type="button" class="ui-icon-button" data-colony-step="1" aria-label="Nächster Planet" ${index===planets.length-1?'disabled':''}>${uiIcon('right')}</button></div>${mondKnopf}`);
   contextStamp=stamp;
   if(focused)host.querySelector(focused==='picker'?'[data-open-planets]':`[data-colony-step="${focused}"]`)?.focus({preventScroll:true});
  }
  // The resource strip always belongs to this planet, including empire comparisons.
  $('resource-bar').setAttribute('aria-label','Rohstoffe auf '+p.koord+' · Credits reichsweit');
 }
 function localHeading(p){
  const host=$('supply-identity');if(!host)return;
  const warnings=issues(p);
  setStableHTML(host, mode==='all'?`<span class="supply-empire-symbol" aria-hidden="true">✧</span><div><span class="scope-eyebrow">GESAMTES IMPERIUM</span><h2>Versorgung im Vergleich</h2><p>${view.planeten.length} Planeten · Rohstoffleiste oben: ${esc(p.koord)}</p></div>`:`<img src="${esc(planetArt(p))}" alt=""><div><span class="scope-eyebrow">VERSORGUNG DIESES PLANETEN</span><h2>${esc(p.koord)} <small>${esc(label(p))}</small></h2><p>${warnings.length?esc(warnings.join(' · ')):'Versorgung ohne akuten Engpass'}</p></div>`);
  for(const b of document.querySelectorAll('.supply-scope button[data-supply-mode]')){b.setAttribute('aria-pressed',String(b.dataset.supplyMode===mode));if(b.dataset.supplyMode==='all')b.textContent=view.planeten.length===1?'Alle Planeten (1)':`Alle ${view.planeten.length} Planeten`;}
 }
 function sum(fn){return view.planeten.reduce((n,p)=>n+(Number(fn(p))||0),0);}
 function management(p){
  const c=view.planetenkapazitaet||{anzahl:view.planeten.length,erlaubt:1+view.kolonien.erlaubt,maximal:15,reserviert:0};
  // Neue Welten (0.4.7, Karl 24.10.2026): ein Platz je Kolonie – gleich ob Kolonieschiff auf einen freien Planeten oder
  // Kampfkolonisierung; Flüge reservieren nichts, entscheidend ist der freie Platz bei Gründung bzw. Übernahme.
  const neu=(view.kolonien?.frei_basis||0)>0,free=Math.max(0,c.erlaubt-c.anzahl-(neu?0:c.reserviert));
  const unterwegs=(view.flotten||[]).filter(f=>['kolonisieren','kampfkolonisieren'].includes(f.mission)&&['hinflug','im_orbit'].includes(f.zustand)).length;
  const herkunft=neu?`${view.kolonien.frei_basis} frei ab Start + 1 je Stufe ${named(view.kolonien.forschung||'terraforming')}`:named(view.kolonien?.forschung||'astrophysik');
  // Name des gewählten Planeten mit Umbenennen-Stift auch hier (Funktionsprüfung 25.10.2026: nur im Übersichtskopf erreichbar).
  const nameZeile=p&&window.Planetenname?`<p class="colony-name">${esc(Planetenname.text(p.koord))}${Planetenname.stift(p.koord,'kolonien-stift')}</p>`:'';
  return `${nameZeile}<div class="colony-management"><div><strong>${c.anzahl} / ${c.maximal} Planeten</strong><span>Heimatwelt mitgezählt · ${c.erlaubt} erlaubt (${esc(herkunft)}) · ${neu?`${unterwegs} Kolonieflug unterwegs`:`${c.reserviert} reserviert`} · ${free} frei</span>${c.anzahl>=c.maximal?`<small>Höchstens ${c.maximal} Planeten. Für ein neues Ziel zuerst eine Kolonie aufgeben.${neu?'':' Bereits gestartete Missionen reservieren ihren Platz.'}</small>`:neu&&!free?`<small>Kolonie- und Kampfkolonisierungsflüge starten auch jetzt – gegründet bzw. übernommen wird nur, wenn bis dahin ${esc(named(view.kolonien.forschung||'terraforming'))} ${view.kolonien.naechste_stufe||''} erforscht ist.</small>`:''}</div>${p.heimat?'<span class="home-protected">Heimatwelt · geschützt</span>':`<button type="button" class="abandon-colony" data-abandon-planet="${esc(p.koord)}">${esc(p.koord)} aufgeben …</button>`}</div>`;
 }
 function overview(){
  const speed=world.tempo||1,p=currentPlanet(),problems=view.planeten.filter(x=>issues(x).length);
  const totals=$('supply-totals');
  setStableHTML(totals, `<div><span>Bevölkerung gesamt</span><strong>${fmt(sum(x=>x.bevoelkerung))}</strong><small>${fmt(sum(x=>x.wohnraum))} Wohnplätze</small></div><div><span>Freie Fachkräfte</span><strong>${fmt(sum(x=>Math.max(0,x.fachkraefte.verfuegbar-x.fachkraefte.bedarf)))}</strong><small>Jeweils vor Ort verfügbar</small></div><div><span>Forschung im Reich</span><strong>${fmt(sum(x=>window.Mobile035.researchers(x).fp_je_spielstunde)*speed)} <small>FP/h</small></strong><small>${fmt(sum(x=>window.Mobile035.researchers(x).zugewiesen))} / ${fmt(sum(x=>window.Mobile035.researchers(x).plaetze))} Forschungsplätze</small></div><div class="${problems.length?'has-shortage':''}"><span>Planeten mit Engpässen</span><strong>${problems.length} <small>/ ${view.planeten.length}</small></strong><small>Strom, Nahrung, Personal, Wohnraum</small></div>`);
  $('supply-only-problems').nextElementSibling.textContent=`Nur Engpässe (${problems.length})`;
  const search=query.trim().toLocaleLowerCase('de'),filtered=view.planeten.filter(x=>(!onlyProblems||issues(x).length)&&(!search||(x.koord+' '+label(x)).toLocaleLowerCase('de').includes(search)));
  $('supply-result-count').textContent=`${filtered.length} / ${view.planeten.length} Planeten · Zeile öffnet die Versorgung`;
  const rows=$('supply-rows'),focused=rows.querySelector(':focus')?.dataset.supplyPlanet;
  const cell=(title,value,sub='',bad=false)=>`<span class="colony-value ${bad?'has-shortage':''}" data-label="${title}"><strong>${value}</strong>${sub?`<small>${sub}</small>`:''}</span>`;
  setStableHTML(rows, filtered.map(x=>{const warnings=issues(x),r=window.Mobile035.researchers(x),active=x.koord===p.koord;return `<button type="button" class="colony-row" data-supply-planet="${esc(x.koord)}" aria-label="Versorgung von ${esc(x.koord)} öffnen${active?' · aktiver Planet':''}" ${active?'aria-current="true"':''}><span class="colony-row-identity"><img src="${esc(planetArt(x))}" alt=""><span><strong>${esc(x.koord)}${active?'<i>Aktiv</i>':''}</strong><small>${esc(label(x))}</small></span></span>${cell('Bevölkerung',fmt(x.bevoelkerung),fmt(x.wohnraum?x.bevoelkerung/x.wohnraum*100:0)+' % Wohnraum',x.bevoelkerung>=x.wohnraum*.9)}${cell('Fachkräfte',fmt(x.fachkraefte.verfuegbar)+' / '+fmt(x.fachkraefte.bedarf),'vorhanden / benötigt',x.fachkraefte.verfuegbar<x.fachkraefte.bedarf)}${cell('Forschung',fmt((r.fp_je_spielstunde||0)*speed)+' FP/h',fmt(r.zugewiesen||0)+' / '+fmt(r.plaetze||0)+' Forscher')}${cell('Energie',workforceRate(energy(x)),'Erzeugung − Bedarf',energy(x)<0)}${cell('Nahrung',!window.Mobile035.organic(x)?'entfällt':workforceRate(x.rate.nahrung*speed)+'/h',!window.Mobile035.organic(x)?'Syntheten': 'Netto · Echtzeit',window.Mobile035.organic(x)&&x.rate.nahrung<0)}<span class="colony-row-status ${warnings.length?'has-shortage':''}">${warnings.length?esc(warnings.join(' · ')):'Versorgt'}<b aria-hidden="true">↗</b></span></button>`;}).join('')||'<p class="colony-no-results">Keine Planeten für diesen Filter. Suchtext oder Engpassfilter anpassen.</p>');
  if(focused)rows.querySelector(`[data-supply-planet="${focused}"]`)?.focus({preventScroll:true});
 }
 function renderClimate(p){
  const host=$('planet-climate');if(!host)return;
  const c=p.habitabilitaet||{},open=host.querySelector('details')?.open;
  setStableHTML(host, `<div class="climate-strip"><span><b>${fmt((c.solar_promille??1000)/10)} %</b> Sonnenlicht</span><span><b>${p.felder.belegt} / ${p.felder.gesamt}</b> Baufelder</span><span><b>${esc(named(c.zone||p.zone))}</b> Klimazone</span></div><details ${open?'open':''}><summary>Klima & Herkunft auf ${esc(p.koord)}</summary>${habitabilityMarkup(c,p.personengruppen?.gruppen)}<p>Die Eignung hängt vom Volk und Terraforming ab. Auch Syntheten benötigen für die innersten Extremorbits Terraforming. Ihr Wachstum braucht Energie; organische Einwohner benötigen Nahrung.</p><button type="button" data-workforce-target="solarkraftwerk" data-workforce-planet="${esc(p.koord)}">Energie verwalten ↗</button></details>${management(p)}`);
 }
 function render(){
  if(!view?.planeten.length||!$('supply-console'))return;
  ensure();context();localHeading(currentPlanet());
  for(const id of ['workforce-economy','planet-climate','kolonie-content'])$(id).hidden=mode==='all';
  $('supply-imperium').hidden=mode!=='all';
  $('kolonie').dataset.supplyMode=mode;
  renderClimate(currentPlanet());
  if(mode==='all')overview();
 }
 function openPlanet(koord){
  if(!view.planeten.some(x=>x.koord===koord)){show('Dieser Planet gehört nicht mehr zu deinem Reich.',true);return;}
  setMode('planet');PlanetNavigation.select(koord);navigateGame('kolonie');
  $('supply-console').scrollIntoView({block:'start',behavior:'instant'});
 }
 document.addEventListener('DOMContentLoaded',()=>{
  $('tabs').insertAdjacentHTML('beforeend','<div id="colony-context" class="colony-context"></div>');
  $('kolonie').querySelector(':scope>h2').hidden=true;
  $('kolonie').insertAdjacentHTML('afterbegin','<div id="supply-console"><div id="supply-identity"></div><div class="supply-scope" role="group" aria-label="Versorgungsansicht"><button type="button" data-supply-mode="planet" aria-pressed="true">Dieser Planet</button><button type="button" data-supply-mode="all" aria-pressed="false">Alle Planeten</button></div></div><section id="supply-imperium" hidden><div id="supply-totals"></div><div class="supply-filters"><label>Planet finden<input id="supply-search" type="search" placeholder="Koordinaten oder Planetentyp" autocomplete="off"></label><label class="supply-check"><input id="supply-only-problems" type="checkbox"><span>Nur Engpässe</span></label><small id="supply-result-count"></small></div><div class="colony-columns" aria-hidden="true"><span>Planet</span><span>Bevölkerung</span><span>Fachkräfte</span><span>Forschung</span><span>Energie</span><span>Nahrung</span><span>Status</span></div><div id="supply-rows"></div></section>');
  const observer=new ResizeObserver(()=>document.documentElement.style.setProperty('--colony-nav-height',Math.ceil($('tabs').getBoundingClientRect().height)+'px'));
  observer.observe($('tabs'));
  document.body.insertAdjacentHTML('beforeend','<dialog id="planet-abandon-dialog" class="item-info-dialog" aria-labelledby="abandon-title"></dialog>');
 });
 document.addEventListener('change',e=>{if(e.target.id==='supply-only-problems'){onlyProblems=e.target.checked;overview();}});
 document.addEventListener('input',e=>{if(e.target.id==='supply-search'){query=e.target.value;overview();}});
 document.addEventListener('click',e=>{
  const b=e.target.closest('button[data-colony-step],button[data-supply-mode],button[data-supply-planet]');if(!b)return;
  if(b.dataset.colonyStep){const i=view.planeten.findIndex(p=>p.koord===selectedPlanet)+Number(b.dataset.colonyStep);if(view.planeten[i])PlanetNavigation.select(view.planeten[i].koord);}
  if(b.dataset.supplyMode){setMode(b.dataset.supplyMode);render();}
  if(b.dataset.supplyPlanet)openPlanet(b.dataset.supplyPlanet);
 });
 document.addEventListener('click',guard(async e=>{
  const b=e.target.closest('[data-abandon-planet],[data-close-abandon]');if(!b)return;
  const d=$('planet-abandon-dialog');
  if(b.hasAttribute('data-close-abandon')){d.close();return;}
  const planet=b.dataset.abandonPlanet,q=await api('/api/tool',{typ:'planet_aufgeben',planet});
  // Kern (verwaltung.rs planet_aufgeben_vorschau) liefert die Verluste unter q.verluste – Einsatzprobe 25.10.2026: der Dialog
  // zeigte „undefined Gebäudestufen · undefined Aufträge“.
  const vl=q.verluste||{},summe=o=>Object.values(o||{}).reduce((s,n)=>s+(Number(n)||0),0),auftraege=(Number(vl.bauauftraege)||0)+(Number(vl.fertigung)||0);
  const p=view.planeten.find(x=>x.koord===planet);if(!p)return;
  setStableHTML(d, `<div class="settings-title"><h2 id="abandon-title">Kolonie ${esc(planet)} aufgeben</h2><button type="button" data-close-abandon aria-label="Aufgeben schließen">×</button></div>${q.moeglich?`<p>Der Planet wird frei und kann erneut besiedelt werden. Diese Aufgabe lässt sich nicht rückgängig machen.</p><div class="abandon-losses"><span><strong>${fmt(vl.bevoelkerung||0)}</strong> Bevölkerung</span><span><strong>${fmt(summe(vl.gebaeude))}</strong> Gebäudestufen</span><span><strong>${fmt(summe(vl.einheiten))}</strong> Schiffe & Verteidigung</span></div><p><b>Zurückgelassene Vorräte:</b> ${Object.keys(vl.bestand||{}).length?esc(goods(vl.bestand)):'keine'}.</p><p>${fmt(auftraege)} Bau-/Fertigungsaufträge werden beendet. Noch offene Marktangebote werden storniert; verkaufte Ware bleibt bezahlt. Unterwegs befindliche Rückflotten und gekaufte Lieferungen weichen auf die Heimat aus. Reichsweite Forschung bleibt erhalten.</p><p class="abandon-advice">Erst bergen, dann gehen: Rohstoffe im Lager und vorhandene Schiffe vor dem Aufgeben abtransportieren. Gebäudeabriss erstattet nichts.</p>`:''}${q.moeglich?`<form id="abandon-form" data-planet="${esc(planet)}"><label for="abandon-confirm">Zur Bestätigung ${esc(planet)} eingeben<input id="abandon-confirm" name="bestaetigung" autocomplete="off" spellcheck="false" required placeholder="${esc(planet)}"></label><div class="abandon-actions"><button type="button" data-close-abandon>Kolonie behalten</button><button id="abandon-submit" class="abandon-colony" type="submit" disabled>Kolonie endgültig aufgeben</button></div></form>`:`<p class="warning">${esc(q.grund)}</p><button type="button" data-close-abandon>Schließen</button>`}`);
  d.showModal();
 }));
 document.addEventListener('input',e=>{if(e.target.id==='abandon-confirm')$('abandon-submit').disabled=e.target.value.trim()!==e.target.form.dataset.planet;});
 document.addEventListener('submit',guard(async e=>{
  if(e.target.id!=='abandon-form')return;
  const f=e.target,planet=f.dataset.planet,bestaetigung=f.elements.bestaetigung.value.trim();
  if(bestaetigung!==planet||f.dataset.busy)return;
  f.dataset.busy='true';
  $('abandon-submit').disabled=true;
  try{const result=await command([{typ:'planet_aufgeben',planet,bestaetigung}]);if(result.ergebnisse.every(r=>r.ok)){$('planet-abandon-dialog').close();PlanetNavigation.select(currentPlanet().koord);navigateGame('kolonien');}}
  finally{delete f.dataset.busy;if($('abandon-submit'))$('abandon-submit').disabled=false;}
 }));
 return {render,onPlanetChange,openPlanet,management};
})();

// Shared, searchable planet picker. The top layer avoids clipping inside sticky navigation.
window.PlanetPicker=(()=>{
 let trigger=null,opened=false;
 const panel=()=>$('planet-picker');
 function position(){
  if(!opened||!trigger?.isConnected){if(opened)close(false);return;}
  const r=trigger.getBoundingClientRect(),p=panel(),viewport=window.visualViewport;
  const vw=viewport?.width||innerWidth,vh=viewport?.height||innerHeight,ox=viewport?.offsetLeft||0,oy=viewport?.offsetTop||0;
  const width=Math.min(420,vw-24),left=Math.max(ox+12,Math.min(r.right-width,ox+vw-width-12));
  const below=oy+vh-r.bottom-12,above=r.top-oy-12,up=below<230&&above>below;
  p.style.width=width+'px';p.style.left=left+'px';p.style.maxHeight=Math.max(160,Math.min(510,up?above:below))+'px';
  p.style.top=up?'auto':Math.max(oy+12,r.bottom+6)+'px';p.style.bottom=up?Math.max(12,innerHeight-r.top+6)+'px':'auto';
 }
 function render(){
  if(!view)return;
  const q=$('planet-picker-search').value.trim().toLocaleLowerCase(),pins=PlanetNavigation.snapshot();
  const list=view.planeten.filter(p=>(p.koord+' '+Planetenname.name(p)+' '+(p.heimat?'Heimatwelt':named(p.zone)+' Kolonie')+(p.mond?' mond '+(p.mond.name||''):'')).toLocaleLowerCase().includes(q));
  $('planet-picker-count').textContent=`${list.length} / ${view.planeten.length}`;
  // Monde als eigene Zeile unter ihrem Planeten: am Handy gibt es keine Seitenleiste mit Mondbild (rahmen.js), ohne diese
  // Zeile wären Mondseite, Sprungtor, Phalanx und Weltenkrümmer-Werft dort unerreichbar (Konsolidierung 19.10.2026, M7).
  const mondZeile=p=>{const m=p.mond;if(!m)return '';const aktiv=document.body.dataset.bereich==='mond'&&window.Mondseite&&$('mond')?.querySelector('.mond-banner')?.textContent.includes(p.koord);
   return `<button type="button" class="planet-option planet-option-mond" data-pick-mond="${esc(p.koord)}" ${aktiv?'aria-current="true"':''}><span class="planet-portrait">${window.Mondseite?Mondseite.bild(m,'planet-mond-bild'):'☾'}</span><span class="planet-option-copy"><strong>☾ Mond ${esc(m.name||'')}</strong><small>Mond von [${esc(p.koord)}]</small></span><span class="planet-option-mark">${aktiv?`<span>Aktiv</span>${uiIcon('check')}`:''}</span></button>`;};
  setStableHTML($('planet-picker-list'), list.map(p=>`<button type="button" class="planet-option" data-pick-planet="${esc(p.koord)}" ${p.koord===selectedPlanet&&document.body.dataset.bereich!=='mond'?'aria-current="true"':''}><span class="planet-portrait"><img src="${esc(planetArt(p))}" alt="" loading="lazy"></span><span class="planet-option-copy"><strong>${esc(Planetenname.name(p))}</strong><small>[${esc(p.koord)}] · ${esc(named(p.zone))}${p.name?' · '+(p.heimat?'Heimatwelt':'Kolonie'):''}</small></span><span class="planet-option-mark">${p.koord===selectedPlanet?`<span>Aktiv</span>${uiIcon('check')}`:pins.includes(p.koord)?uiIcon('star'):''}</span></button>${mondZeile(p)}`).join('')||'<p class="picker-empty">Kein Planet gefunden. Name, Koordinaten oder Planetentyp eingeben.</p>');
 }
 function close(restore=true){
  if(!opened)return;opened=false;
  const p=panel();if(p.hidePopover)p.hidePopover();else p.hidden=true;p.classList.remove('is-open');
  document.querySelectorAll('[data-open-planets]').forEach(b=>b.setAttribute('aria-expanded','false'));
  if(restore)(trigger?.isConnected?trigger:$('context-planet'))?.focus({preventScroll:true});
  trigger=null;
 }
 function open(anchor){
  if(opened&&trigger===anchor){close();return;}if(opened)close(false);
  trigger=anchor;opened=true;const p=panel();$('planet-picker-search').value='';render();
  p.classList.add('is-open');if(p.showPopover)p.showPopover();else p.hidden=false;
  anchor.setAttribute('aria-expanded','true');position();
  const list=$('planet-picker-list'),active=list.querySelector('[aria-current="true"]');
  if(active)list.scrollTop=Math.max(0,active.offsetTop-list.offsetTop-(list.clientHeight-active.offsetHeight)/2);
  // Am Touchgerät nicht ins Suchfeld: die Bildschirmtastatur verdeckte sonst die halbe Liste (Emulator 19.10.2026)
  if(matchMedia('(pointer: coarse)').matches)(active||list.querySelector('[data-pick-planet]'))?.focus({preventScroll:true});
  else $('planet-picker-search').focus({preventScroll:true});
 }
 document.addEventListener('DOMContentLoaded',()=>{
  document.body.insertAdjacentHTML('beforeend',`<div id="planet-picker" popover="manual" role="dialog" aria-labelledby="planet-picker-title"><div class="picker-heading"><div><span class="scope-eyebrow">Navigation</span><h2 id="planet-picker-title">Planet wechseln</h2></div><span id="planet-picker-count"></span><button type="button" class="ui-icon-button" data-close-picker aria-label="Planetenauswahl schließen">${uiIcon('close')}</button></div><label class="picker-search">${uiIcon('search')}<input id="planet-picker-search" type="search" aria-label="Eigene Planeten suchen" placeholder="Koordinaten oder Planetentyp" autocomplete="off"></label><div id="planet-picker-list" role="group" aria-label="Eigene Planeten"></div><p class="picker-hint">Dein Spielbereich bleibt beim Wechsel geöffnet.</p></div>`);
  if(!panel().showPopover)panel().hidden=true;
 });
 document.addEventListener('click',e=>{
  const b=e.target.closest('[data-open-planets],[data-pick-planet],[data-pick-mond],[data-close-picker]');
  if(b?.hasAttribute('data-open-planets')){open(b);return;}
  if(b?.dataset.pickMond){const k=b.dataset.pickMond;close(false);if(PlanetNavigation.snapshot&&view.planeten.some(p=>p.koord===k))PlanetNavigation.select(k);window.Mondseite?.oeffne(k);return;}
  if(b?.dataset.pickPlanet){const k=b.dataset.pickPlanet;close(false);PlanetNavigation.select(k);$('context-planet')?.focus({preventScroll:true});return;}
  if(b?.hasAttribute('data-close-picker')){close();return;}
  if(opened&&!panel().contains(e.target))close(false);
 });
 document.addEventListener('input',e=>{if(e.target.id==='planet-picker-search')render();});
 document.addEventListener('keydown',e=>{
  if(!opened)return;
  if(e.key==='Escape'){e.preventDefault();close();return;}
  if(!panel().contains(e.target))return;
  const choices=[...panel().querySelectorAll('[data-pick-planet]')],index=choices.indexOf(document.activeElement);
  if(['ArrowDown','ArrowUp','Home','End'].includes(e.key)&&!(e.target.id==='planet-picker-search'&&['Home','End'].includes(e.key))){
   e.preventDefault();const next=e.key==='Home'?0:e.key==='End'?choices.length-1:e.key==='ArrowDown'?Math.min(index+1,choices.length-1):Math.max(0,index-1);choices[next]?.focus();
  }
  if(e.key==='Enter'&&e.target.id==='planet-picker-search'&&choices.length===1){e.preventDefault();choices[0].click();}
 });
 window.addEventListener('resize',position);window.addEventListener('scroll',position,{passive:true});
 window.visualViewport?.addEventListener('resize',position);
 return {close};
})();
