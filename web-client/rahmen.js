'use strict';
// Fester Dreispalten-Rahmen nach docs/DESIGNREGELN.md §1: Hauptmenü links (eine Ebene), Ressourcen- und
// Ereignisleiste oben, Planetenliste rechts. Seltene Bereiche sind Reiter einer Seite – höchstens eine Ebene tief.
// Alle Befehle laufen weiter über die vorhandenen Handler (data-build, pickFleet, command …).
const Rahmen=(()=>{
 // Menüpunkt → Bereiche (der erste ist das Ziel des Menüpunkts, die übrigen erscheinen als Reiter).
 const MENUE=[
  ['uebersicht','Übersicht',[['reich','Übersicht'],['auftraege','Bauaufträge']],'uebersicht'],
  ['versorgung','Versorgung',[['versorgung','Gebäude'],['kolonie','Produktion & Personal']],'versorgung'],
  ['anlagen','Anlagen',[['gebaeude','Anlagen']],'anlagen'],
  ['forschung','Forschung',[['forschen','Forschung'],['freischaltungen','Technologiebaum']],'forschung'],
  ['werft','Schiffswerft',[['werft','Schiffswerft']],'werft'],
  ['verteidigung','Verteidigung',[['verteidigung','Verteidigung & Raketen']],'verteidigung'],
  ['truppen','Truppen',[['truppen','Truppen']],'truppen'],
  ['flotte','Flotte',[['flotten','Flottenkommando'],['warsim','Kampfsimulator']],'flotte'],
  ['galaxie','Galaxie',[['karte','Galaxie'],['kolonien','Kolonisation']],'galaxie'],
  ['imperium','Imperium',[['imperium','Imperiumsvergleich'],['core-planner','Ausbauplaner'],['codex','Codex']],'imperium'],
  ['allianz','Allianz',[['allianzbereich','Allianz'],['grossprojekt','Großprojekt'],['allianzpost','Allianzpost · Führung']],'allianz'],
  // 0.4.7: Briefe und alle Berichte stehen in einer Chronik (berichtszentrale.js); „berichte“ führt dorthin.
  ['nachrichten','Nachrichten',[['briefkasten','Nachrichten']],'nachrichten'],
  ['handel','Handel',[['aktionen','Handel & Markt']],'handel'],
  ['rat','Rat',[['agent','Modelle & Team'],['pinwaende','Pinwände']],'rat'],
  // Nur über die Kopfzeile erreichbar (Einstellungen, Hilfe); eigener Reiterverbund ohne Menüpunkt.
  ['konto','Konto',[['profil','Einstellungen'],['regeln','Hilfe']],null],
  // Mondseite: über das Mondbild in der Planetenliste und in der Galaxie (ein Klick), kein eigener Menüpunkt.
  ['mond','Mond',[['mond','Mond']],null]
 ];
 // Bereiche, die in anderen Seiten aufgehen (alte Ziele bleiben gültig).
 const UMLEITUNG={'missile-panel':'verteidigung'};
 // Auf dem Handy (< 768 px) zeigt die untere Leiste fünf Punkte und „Mehr“.
 const MOBIL=['reich','versorgung','flotten','karte','briefkasten'];
 const SYMBOLE={
  uebersicht:'M3 11 12 4l9 7M5 10v10h14V10',
  versorgung:'M4 20V9l4 3V9l4 3V9l4 3V5h4v15Z',
  anlagen:'M3 21V10l6-4v4l6-4v4l6-4v15ZM7 17h2m4 0h2',
  forschung:'M9 3h6M10 3v7l-6 9q-1 2 2 2h12q3 0 2-2l-6-9V3M8 15h8',
  werft:'M3 17h18l-3 4H6ZM6 17V9l6-5 6 5v8M12 4v13',
  verteidigung:'M12 3 4 6v6c0 5 8 9 8 9s8-4 8-9V6Z',
  truppen:'M12 4a3 3 0 1 1 0 6 3 3 0 0 1 0-6ZM5 21v-3a7 7 0 0 1 14 0v3',
  flotte:'M2 12h6l3-6 3 12 3-6h5',
  galaxie:'M12 12m-3 0a3 3 0 1 0 6 0 3 3 0 1 0-6 0M3 12c4-6 14-6 18 0M3 12c4 6 14 6 18 0',
  imperium:'M4 20h16M6 20V10m4 10V10m4 10V10m4 10V10M3 10l9-6 9 6',
  allianz:'M8 11a3 3 0 1 0 0-6 3 3 0 0 0 0 6Zm8 0a3 3 0 1 0 0-6 3 3 0 0 0 0 6ZM2 20a6 6 0 0 1 12 0m-2 0a6 6 0 0 1 10 0',
  nachrichten:'M3 6h18v12H3ZM3 6l9 7 9-7',
  handel:'M4 7h13l-3-3M20 17H7l3 3',
  rat:'M12 3a4 4 0 0 1 4 4v2a4 4 0 0 1-8 0V7a4 4 0 0 1 4-4ZM4 21a8 8 0 0 1 16 0',
  kolonien:'M12 12m-4 0a4 4 0 1 0 8 0 4 4 0 1 0-8 0M2 15c4 3 17-1 20-7',
  codex:'M5 4h11a3 3 0 0 1 3 3v13H8a3 3 0 0 1-3-3ZM5 17a3 3 0 0 1 3-3h11',
  themen:'M4 4h7v7H4ZM13 4h7v7h-7ZM4 13h7v7H4ZM13 13h7v7h-7Z',
  mehr:'M5 12h.01M12 12h.01M19 12h.01'
 };
 // Handy (< 768 px): Die untere Leiste zeigt ein Thema mit vier Zielen; der Themenknopf links wechselt das Thema
 // (Karl 19.10.2026: „in 4 themen um bauen, flotte und kommunikation … modern und optisch cool, aber platz sparen“).
 // Jedes Ziel ist ein Bereich (data-tab); alle übrigen Bereiche stehen im Themenblatt – höchstens zwei Tipps bis überall.
 const THEMEN=[
  {id:'aufbau',name:'Aufbau',titel:'Aufbau & Forschung',symbol:'versorgung',punkte:[['reich','Übersicht','uebersicht'],['versorgung','Versorgung','versorgung'],['gebaeude','Anlagen','anlagen'],['forschen','Forschung','forschung']]},
  {id:'flotte',name:'Flotte',titel:'Flotte & Verteidigung',symbol:'flotte',punkte:[['werft','Werft','werft'],['flotten','Flotte','flotte'],['verteidigung','Abwehr','verteidigung'],['truppen','Truppen','truppen']]},
  {id:'galaxie',name:'Galaxie',titel:'Galaxie & Imperium',symbol:'galaxie',punkte:[['karte','Karte','galaxie'],['kolonien','Kolonien','kolonien'],['imperium','Imperium','imperium'],['codex','Codex','codex']]},
  {id:'funk',name:'Funk',titel:'Kommunikation',symbol:'nachrichten',punkte:[['briefkasten','Post','nachrichten'],['allianzbereich','Allianz','allianz'],['aktionen','Handel','handel'],['agent','Rat','rat']]}
 ];
 const GRUPPE_THEMA={uebersicht:'aufbau',versorgung:'aufbau',anlagen:'aufbau',forschung:'aufbau',werft:'flotte',verteidigung:'flotte',truppen:'flotte',flotte:'flotte',galaxie:'galaxie',imperium:'galaxie',mond:'galaxie',allianz:'funk',nachrichten:'funk',handel:'funk',rat:'funk'};
 // Weitere Bereiche im Themenblatt (Reiter, die sonst nur über ihre Seite erreichbar sind).
 const WEITERE=[['auftraege','Bauaufträge'],['kolonie','Produktion & Personal'],['freischaltungen','Technologiebaum'],['warsim','Kampfsimulator'],['core-planner','Ausbauplaner'],['grossprojekt','Großprojekt'],['pinwaende','Notizen'],['profil','Einstellungen'],['regeln','Hilfe']];
 const THEMA_SPEICHER='sternenepoche.thema';
 let letzterBereich=null;
 let thema=(()=>{try{const t=localStorage.getItem(THEMA_SPEICHER);return THEMEN.some(x=>x.id===t)?t:'aufbau';}catch(_){return 'aufbau';}})();
 const themaVon=id=>{const g=gruppeVonId(id);return g?GRUPPE_THEMA[g[0]]||null:null;};
 function gruppeVonId(id){return MENUE.find(([, ,bereiche])=>bereiche.some(([b])=>b===id));}
 function themaSetzen(id){thema=id;try{localStorage.setItem(THEMA_SPEICHER,id);}catch(_){}themenleiste();}
 const symbol=name=>`<svg class="menue-symbol" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true" focusable="false"><path d="${SYMBOLE[name]||SYMBOLE.mehr}"/></svg>`;
 const gruppeVon=id=>MENUE.find(([, ,bereiche])=>bereiche.some(([b])=>b===id));
 let eingerichtet=false;

 function einrichten(){
  if(eingerichtet)return;eingerichtet=true;
  const nav=$('tabs'),quelle=nav.querySelector('.reiter-quelle');
  // Alle Bereichsknöpfe ohne eigenen Menüpunkt (auch die später von anderen Modulen eingefügten) wandern in die Reiterquelle.
  for(const b of [...nav.querySelectorAll('[data-tab]')])if(!b.dataset.menue&&!quelle.contains(b))quelle.append(b);
  for(const b of nav.querySelectorAll('[data-menue]')){const [,name]=MENUE.find(([k])=>k===b.dataset.menue)||[];b.innerHTML=`<span class="menue-name">${esc(name||b.textContent)}</span>${symbol(b.dataset.menue)}`;b.title=name||'';if(MOBIL.includes(b.dataset.tab))b.dataset.mobil='true';}
  nav.insertAdjacentHTML('beforeend',`<button type="button" class="menue-mehr" data-menue-mehr aria-expanded="false"><span class="menue-name">Mehr</span>${symbol('mehr')}</button>`);
  // Ereignisleiste und Reiterzeile stehen über dem Inhalt; das bisherige Ereignisband bleibt als Datenquelle unsichtbar.
  $('resource-bar').insertAdjacentHTML('afterend','<div id="ereignisleiste" role="region" aria-label="Ereignisse"></div><nav id="seiten-reiter" aria-label="Reiter dieser Seite" hidden></nav>');
  // Auf dem Handy wird die Planetenliste zu einem Auswahlknopf oben (der vorhandene Planetenwähler).
  const kontext=$('colony-context');if(kontext){$('resource-bar').insertAdjacentHTML('beforebegin','<div id="rahmen-planetenwahl"></div>');$('rahmen-planetenwahl').append(kontext);}
  if(!document.querySelector('[data-tab][aria-selected="true"]'))nav.querySelector('[data-tab="reich"]').setAttribute('aria-selected','true');
  synchronisieren();
 }

 function aktiverBereich(){return [...document.querySelectorAll('#game > section')].find(s=>!s.hidden)?.id||'reich';}
 function synchronisieren(){
  const nav=$('tabs'),quelle=nav?.querySelector('.reiter-quelle');
  if(eingerichtet&&quelle)for(const b of [...nav.querySelectorAll('[data-tab]')])if(!b.dataset.menue&&!quelle.contains(b))quelle.append(b);
  const id=aktiverBereich(),gruppe=gruppeVon(id);
  for(const b of document.querySelectorAll('#tabs [data-menue]')){const aktiv=gruppe?.[0]===b.dataset.menue;if(b.getAttribute('aria-selected')!==String(aktiv))b.setAttribute('aria-selected',String(aktiv));if(aktiv){if(b.getAttribute('aria-current')!=='page')b.setAttribute('aria-current','page');}else if(b.hasAttribute('aria-current'))b.removeAttribute('aria-current');}
  const reiter=$('seiten-reiter');if(!reiter)return;
  const sichtbar=(gruppe?.[2]||[]).filter(([b])=>$(b)&&!(b==='allianzpost'&&$('alliance-post-nav')?.hidden));
  reiter.hidden=sichtbar.length<2;
  setStableHTML(reiter,sichtbar.map(([b,name])=>`<button type="button" data-tab="${b}" aria-selected="${b===id}">${esc(name)}</button>`).join(''));
  document.body.dataset.bereich=id;
  for(const b of document.querySelectorAll('#kopfzeile [data-tab]')){const v=String(b.dataset.tab===id);if(b.getAttribute('aria-selected')!==v)b.setAttribute('aria-selected',v);}
  // Die Themenleiste folgt der Navigation: wer über einen Link in einen Bereich eines anderen Themas springt, sieht dessen Ziele.
  // nur beim Seitenwechsel – sonst setzte jede Aktualisierung ein gewischtes Thema zurück
  if(id!==letzterBereich){letzterBereich=id;const t=themaVon(id);if(t&&t!==thema){thema=t;try{localStorage.setItem(THEMA_SPEICHER,t);}catch(_){}}}
  themenleiste();
 }

 function postZahl(){const n=parseInt(($('inbox-shortcut')?.textContent||'').replace(/\D+/g,''),10);return Number.isFinite(n)&&n>0?n:0;}
 function themenleiste(){
  if(!eingerichtet)return;
  let leiste=$('themenleiste');
  if(!leiste){
   document.body.insertAdjacentHTML('beforeend',`<nav id="themenleiste" aria-label="Hauptmenü"></nav><div id="themenblatt" role="dialog" aria-modal="true" aria-label="Menü" hidden></div>`);
   leiste=$('themenleiste');
   if(window.ResizeObserver)new ResizeObserver(()=>{const h=leiste.offsetHeight;if(h)document.documentElement.style.setProperty('--themenleiste-h',h+'px');}).observe(leiste);
   // Wischen auf der Leiste wechselt das Thema (links = nächstes, rechts = voriges).
   let x0=null,y0=0;
   leiste.addEventListener('touchstart',e=>{x0=e.touches[0].clientX;y0=e.touches[0].clientY;},{passive:true});
   leiste.addEventListener('touchend',e=>{if(x0==null)return;const dx=e.changedTouches[0].clientX-x0,dy=e.changedTouches[0].clientY-y0;x0=null;if(Math.abs(dx)<50||Math.abs(dy)>Math.abs(dx))return;const i=THEMEN.findIndex(x=>x.id===thema);const n=THEMEN[(i+(dx<0?1:THEMEN.length-1))%THEMEN.length];leiste.dataset.richtung=dx<0?'links':'rechts';themaSetzen(n.id);});
  }
  const id=aktiverBereich(),t=THEMEN.find(x=>x.id===thema)||THEMEN[0],gruppe=gruppeVonId(id)?.[0];
  // Aktiv: genau dieser Bereich, sonst der Punkt aus derselben Menügruppe (Bauaufträge → Übersicht, Technologiebaum → Forschung).
  const aktivPunkt=t.punkte.find(([b])=>b===id)?.[0]||t.punkte.find(([b])=>gruppeVonId(b)?.[0]===gruppe)?.[0];
  const post=postZahl();
  if(leiste.dataset.thema&&leiste.dataset.thema!==t.id){leiste.dataset.wechsel='1';clearTimeout(themenleiste.t);themenleiste.t=setTimeout(()=>{delete leiste.dataset.wechsel;delete leiste.dataset.richtung;},400);}
  leiste.dataset.thema=t.id;
  setStableHTML(leiste,`<button type="button" class="thema-knopf" data-themen-blatt aria-haspopup="dialog" aria-expanded="${!$('themenblatt').hidden}" aria-label="Menü öffnen – Thema ${esc(t.titel)}">${symbol('themen')}<span class="thema-name">${esc(t.name)}</span><span class="thema-punkte" aria-hidden="true">${THEMEN.map(x=>`<i class="${x.id===t.id?'an':''}"></i>`).join('')}</span></button><div class="thema-ziele">${t.punkte.map(([b,name,sym])=>`<button type="button" data-tab="${b}" aria-current="${b===aktivPunkt?'page':'false'}"${$(b)?'':' disabled'}>${symbol(sym)}<span>${esc(name)}</span>${b==='briefkasten'&&post?`<b class="thema-zahl">${post}</b>`:''}</button>`).join('')}</div>`);
  // Offenes Menü-Blatt mit der neuen Sicht nachziehen (Live-Angaben), ohne Knöpfe unter dem Finger zu ersetzen.
  if(!$('themenblatt').hidden&&window.Menue)themenblatt(true,true);
 }
 // Menü-Blatt (Themenknopf unten links). Gestaltung und Live-Angaben in menue.js (Karl 24.10.2026: „geht cooler … wirkt
 // etwas KI-sloppy“); hier bleiben Öffnen/Schließen, Fokus und die Daten des Rahmens (Themen, weitere Bereiche).
 function themenblatt(offen,nurAktualisieren){
  const blatt=$('themenblatt');if(!blatt)return;
  if(offen){
   const id=aktiverBereich();
   const cockpit=!!document.querySelector('#tabs [data-menue="cockpit"]');
   const ctx={themen:THEMEN,thema,aktiv:id,gruppe:b=>gruppeVonId(b)?.[0]||null,weitere:WEITERE.filter(([b])=>$(b)),cockpit,symbol,post:postZahl()};
   if(window.Menue)setStableHTML(blatt,Menue.html(ctx));
   else blatt.innerHTML=`<div class="blatt-innen"><div class="blatt-griff" aria-hidden="true"></div><h2>Menü</h2><div class="blatt-themen">${THEMEN.map(x=>`<button type="button" class="blatt-thema" data-thema="${x.id}" aria-pressed="${x.id===thema}">${symbol(x.symbol)}<strong>${esc(x.titel)}</strong></button>`).join('')}</div><h3>Weitere Bereiche</h3><div class="blatt-weitere">${cockpit?'<button type="button" class="blatt-gold" data-blatt-cockpit>Handy-Cockpit</button>':''}${ctx.weitere.map(([b,name])=>`<button type="button" data-tab="${b}" aria-current="${b===id?'page':'false'}">${esc(name)}</button>`).join('')}</div><button type="button" class="blatt-zu" data-themen-blatt-zu>Schließen</button></div>`;
   if(nurAktualisieren)return;
  }
  const war=!blatt.hidden;
  blatt.hidden=!offen;document.body.classList.toggle('themenblatt-offen',offen);
  if(offen)blatt.setAttribute('aria-labelledby','themenblatt-titel');
  const knopf=$('themenleiste')?.querySelector('[data-themen-blatt]');knopf?.setAttribute('aria-expanded',String(offen));
  // Fokus: beim Öffnen auf die Überschrift (Bildschirmleser liest „Menü“), beim Schließen zurück auf den Themenknopf.
  if(offen&&!war){blatt.querySelector('.blatt-innen')?.scrollTo?.(0,0);setTimeout(()=>{const t=$('themenblatt-titel');if(t){t.tabIndex=-1;try{t.focus({preventScroll:true});}catch(_){}}},40);}
  if(!offen&&war&&blatt.contains(document.activeElement))try{knopf?.focus({preventScroll:true});}catch(_){}
 }

 // Kopfzeile R1: Spieler · Rang · Notizen · Kontakte · Suche · Einstellungen · Hilfe (die Zeit steht daneben).
 function kopfzeile(){
  const host=$('kopfzeile');if(!host)return;const spielt=session?.spieler!=null&&!!view;host.hidden=!spielt;if(!spielt)return;
  setStableHTML(host,`<span class="kopf-spieler" title="Dein Reich">${esc(view.name)}</span><span class="kopf-rang" title="Platz in der Rangliste">Rang ${esc(view.rang??'–')}</span><button type="button" data-tab="pinwaende">Notizen</button><button type="button" data-tab="briefkasten">Kontakte</button><button type="button" data-tab="codex">Suche</button><button type="button" data-tab="profil">Einstellungen</button><button type="button" data-tab="regeln">Hilfe</button>`);
 }

 // Ereignisleiste R4: nächstes Ereignis mit Countdown, aufklappbar zur vollständigen Liste.
 function ereignisse(){
  const liste=[];
  for(const f of view.flotten||[]){
   const ph=typeof fleetPhase==='function'?fleetPhase(f):{end:f.ankunft_sekunden,returning:false,holding:false,left:f.start,right:f.ziel};
   if(!ph.end)continue;
   const anzahl=Object.values(f.schiffe||{}).reduce((n,x)=>n+(Number(x)||0),0);
   const vb=f.verband!=null&&!ph.returning?(view.verbaende||[]).find(v=>v.fuehrung===f.verband):null;
   liste.push({zeit:ph.end,mission:vb?'verband':f.mission,name:typeof fleetMissionName==='function'?fleetMissionName(f):named(f.mission),start:ph.returning?ph.right:ph.left,ziel:ph.returning?ph.left:ph.right,anzahl,richtung:ph.returning?'Rückflug':ph.holding?'Im Orbit':'Hinflug',feind:false,flotte:f.flotte,rueck:ph.returning,verband:vb});
  }
  // Verbände von Allianzpartnern, an denen man (noch) nicht teilnimmt, als eigene Zeile.
  for(const v of view.verbaende||[])if(!(view.flotten||[]).some(f=>f.verband===v.fuehrung))liste.push({zeit:v.ankunft_sekunden,mission:'verband',name:'Verband '+v.spieler,start:(v.teilnehmer||[]).join(', '),ziel:v.ziel,anzahl:v.schiffe??null,richtung:'Hinflug',feind:false,verband:v,fremd:true});
  for(const a of view.angriffe||[])liste.push({zeit:a.ankunft_sekunden,mission:'angriff',name:'Feindlicher Anflug'+(a.von?' · '+a.von:''),start:a.start?(a.start+(a.start_mond?' ☾':'')):(a.von??'Unbekannt'),ziel:a.ziel,anzahl:a.schiffe??a.sondenbericht?.schiffe??null,sonde:a.sonde||null,richtung:'Ankunft',feind:true,flotte:a.flotte});
  for(const t of view.flugtermine||[]){const a=t.flug?.flug||t.flug||{};liste.push({zeit:t.abflug,mission:a.mission,name:'Geplanter Start · '+named(a.mission),start:a.start,ziel:a.ziel,anzahl:Object.values(a.schiffe||{}).reduce((n,x)=>n+(Number(x)||0),0),richtung:'Abflug',feind:false});}
  return liste.filter(e=>Number.isFinite(e.zeit)).sort((a,b)=>a.zeit-b.zeit);
 }
 const uhrzeit=t=>'Tag '+(Math.floor(t/86400)+1)+' · '+[Math.floor(t/3600)%24,Math.floor(t/60)%60,Math.floor(t)%60].map(n=>String(n).padStart(2,'0')).join(':');
 function ereignisleiste(){
  const host=$('ereignisleiste');if(!host||!view)return;
  const liste=ereignisse(),e=liste[0],offen=host.querySelector('details')?.open,status=view.reich_status?.status;
  const inv=typeof laufendeInvasionen==='function'?laufendeInvasionen():[];const feind=liste.some(x=>x.feind)||status==='kritisch'||inv.length>0;host.classList.toggle('angriff',feind);
  const symbol=x=>window.Auftraege?Auftraege.badge(x.mission,{size:20,symbol:x.rueck?'rueckflug':null,title:x.rueck?'Rückflug · '+Auftraege.info(x.mission).name:null}):`<img src="${esc(art('missions',x.mission))}" alt="" class="ereignis-symbol">`;
  const zeile=x=>`<tr class="${x.feind?'feindlich':''}${x.rueck?' rueck':''}${window.Auftraege&&!x.feind?' '+Auftraege.klasse(x.mission):''}" data-gruppe="${esc(window.Auftraege?Auftraege.info(x.mission).gruppe:'')}"><td><span data-arrival-echt="${x.zeit}"></span></td><td>${esc(uhrzeit(x.zeit))}</td><td>${symbol(x)} ${esc(x.name)}${x.verband?`<span class="ereignis-verband">${x.verband.flotten}/16 Flotten · ${(x.verband.teilnehmer||[]).length} Spieler</span>`:''}${x.feind&&x.flotte!=null?(x.sonde&&!x.sonde.rechtzeitig?`<small class="ereignis-zuspaet">Sonde käme zu spät</small>`:`<button type="button" class="ereignis-retten" data-probe="${esc(x.flotte)}">Ausspähen</button>`):''}${x.feind&&window.Einsatz&&view.planeten?.some(p=>p.koord===x.ziel)?`<button type="button" class="ereignis-retten" data-saven-planet="${esc(x.ziel)}">Flotte retten</button>`:''}</td><td class="koord">${ort(x.start)}</td><td>${x.anzahl==null?'?':fmt(x.anzahl)}</td><td>${esc(x.richtung)}</td><td class="koord">${ort(x.ziel)}</td></tr>`;
  // Invasionswarnung springt zum Bodenkampf-Blatt (Funktionsprüfung 25.10.2026: Tipp klappte nur die Liste auf).
  const hinweise=[...inv.map(i=>`<button type="button" class="ereignis-kritisch ereignis-sprung" data-bk-oeffnen="${esc(i.planet)}">⚠ ${i.belagerung?(i.regeln==='invasion-v2'?'Invasion':'Belagerung'):'Bombardierung'} auf ${ort(i.planet)} ›</button>`),status==='besiegt'?'<strong class="ereignis-kritisch">BESIEGT · nur noch zuschauen</strong>':'',status==='kritisch'?'<strong class="ereignis-kritisch">⚠ Existenzielle Krise</strong>':'',view.anfaengerschutz_bis?`<span class="ereignis-schutz">Anfängerschutz bis ${esc(view.anfaengerschutz_bis)}</span>`:'',world?.paused?'<span>Welt pausiert</span>':'',world?.beendet?'<span>Epoche beendet</span>':''].filter(Boolean).join('');
  const kopf=e?`<span class="ereignis-naechstes">Nächstes: <b data-arrival-echt="${e.zeit}"></b> · ${esc(e.name)} · ${ort(e.start)} → ${ort(e.ziel)}</span>`:inv.length?'':'<span class="ereignis-naechstes">Keine Flottenbewegungen</span>';
  setStableHTML(host,`<details ${offen?'open':''}><summary>${kopf}${hinweise}<span class="ereignis-zahl">${liste.length?liste.length+' Ereignis'+(liste.length===1?'':'se'):''}</span></summary>${liste.length?`<table class="ereignis-liste"><thead><tr><th>Zeit</th><th>Uhrzeit</th><th>Mission</th><th>Start</th><th>Anzahl</th><th>Richtung</th><th>Ziel</th></tr></thead><tbody>${liste.map(zeile).join('')}</tbody></table>`:'<p class="ereignis-leer">Keine eigenen oder erkannten fremden Flotten unterwegs.</p>'}<p class="ereignis-fuss"><button type="button" data-tab="flotten">Zum Flottenkommando</button></p></details>`);
 }

 // Planetenliste R5: Bild, Name, [S:SSS:P]; aktiver Planet hervorgehoben; Mond als kleines Bild am Planeten
 // (Monde kennt der Kern noch nicht – der Platz ist vorbereitet); Werkzeug, solange gebaut wird.
 function planetName(p,i){return window.Planetenname?Planetenname.name(p):p.name||(p.heimat?'Heimatwelt':'Kolonie '+i);}
 // Ort in der Ereignisleiste: eigene/verbündete Planeten mit Namen, sonst [Koordinate]; Freitext (Spielername, Teilnehmer) bleibt.
 function ort(s){const m=/^(\d+:\d+:\d+)(.*)$/.exec(String(s??''));return m&&window.Planetenname?Planetenname.html(m[1],{standardname:true})+esc(m[2]):`<span class="koord">[${esc(s)}]</span>`;}
 function planetenliste(){
  const host=$('planet-sidebar');if(!host||!view)return;
  const aktiv=currentPlanet(),kap=view.planetenkapazitaet||{};
  const zeile=(p,i)=>{const mond=p.mond;return `<li class="planet-zeile ${p.koord===aktiv.koord?'aktiv':''}"><button type="button" class="planet-wahl" data-planet-pick="${esc(p.koord)}" aria-pressed="${p.koord===aktiv.koord}" aria-label="${esc(planetName(p,i))} [${esc(p.koord)}] auswählen"><span class="planet-bild"><img src="${esc(planetArt(p))}" alt="" loading="lazy">${p.bauschleife?.length?'<span class="planet-baut" title="Bau läuft" aria-label="Bau läuft">⚒</span>':''}${window.DysonUI?DysonUI.blitz(p.koord,'planet-blitz'):''}</span><span class="planet-text"><strong>${esc(planetName(p,i))}</strong><small class="koord">[${esc(p.koord)}]</small></span></button>${mond?`<button type="button" class="planet-mond" data-mond="${esc(p.koord)}" title="Mond ${esc(mond.name||'')} öffnen" aria-label="Mond ${esc(mond.name||'')} von ${esc(p.koord)} öffnen">${window.Mondseite?Mondseite.bild(mond,'planet-mond-bild'):'☾'}</button>`:'<span class="planet-mond-platz" data-mond-platz="'+esc(p.koord)+'" aria-hidden="true"></span>'}</li>`;};
  let n=0;
  // Planeten wie Karl zählt (25.10.2026: „man kann gesamt 15 Planeten haben, der Heimatplanet wird mitgezählt, zum Start hat man
  // gleich 3/15 verfügbar, aber nur 1/15 aktuell“): Planeten mit Heimatwelt / Höchstzahl, dazu freigeschaltete und freie Plätze
  // und welche Forschungsstufe den nächsten Platz bringt (Karl 24.10.: „2 Kolonisationsstufen frei – das muss man auch sehen“).
  const kol=view.kolonien||{},anzahl=kap.anzahl??view.planeten.length,erlaubt=kap.erlaubt??1+(kol.erlaubt||0),maximal=kap.maximal||15;
  const frei=Math.max(0,erlaubt-anzahl-((kol.frei_basis||0)>0?0:kap.reserviert||0)),naechst=kol.naechste_stufe?`${typeof named==='function'?named(kol.forschung||'terraforming'):kol.forschung} ${kol.naechste_stufe}`:'';
  const kopf=`<div class="planeten-kopf"><span>Planeten</span><b>${anzahl} / ${maximal}</b></div><p class="planeten-kopf-hinweis">${erlaubt} von ${maximal} freigeschaltet · ${anzahl>=maximal?'alle belegt':frei?`${frei} ${frei===1?'Platz':'Plätze'} frei`:naechst?`nächster Platz: ${esc(naechst)}`:'alle Plätze belegt'}</p>`;
  setStableHTML(host,`${kopf}<ol class="planeten-liste">${view.planeten.map(p=>zeile(p,p.heimat?0:++n)).join('')}</ol>`);
 }

 // Rohstoffleiste lebendig (Karl 19.10.2026): jede Ware in ihrer Farbe, Zugewinn steigt als „+N“ grün auf, Abgang
 // (Bau, Verbrauch, Strom eines neuen Verbrauchers) als „−N“ rot; Zusatzwaren einklappbar, auch dort mit Anzeige.
 const ROH_HAUPT=['erz','kristall','deuterium','nahrung','energie','credits'];
 const ROH_SPEICHER='sternenepoche.rohstoffe-offen';
 const roh={planet:null,werte:{},offen:false,zuletzt:{},sammel:{}};
 try{roh.offen=localStorage.getItem(ROH_SPEICHER)==='1';}catch(_){}
 function rohWert(g,p){return g==='energie'?(p.energie?.erzeugung||0)-(p.energie?.verbrauch||0):g==='credits'?(view.credits||0):(p.bestand?.[g]||0);}
 function rohFlug(node,d,g){
  const el=document.createElement('span');el.className='r-flug '+(d>0?'r-plus':'r-minus');el.setAttribute('aria-hidden','true');
  el.textContent=(d>0?'+':'−')+(typeof fmt==='function'?fmt(Math.abs(Math.round(d))):Math.abs(Math.round(d)))+(g==='energie'?' ⚡':'');
  node.append(el);el.addEventListener('animationend',()=>el.remove(),{once:true});setTimeout(()=>el.remove(),3000);
  node.classList.remove('r-puls','r-puls-minus');void node.offsetWidth;node.classList.add(d>0?'r-puls':'r-puls-minus');
 }
 function rohstoffe(){
  const bar=$('resource-bar'),p=currentPlanet?.();if(!bar||!p)return;
  if(roh.planet!==p.koord){roh.planet=p.koord;roh.werte={};roh.sammel={};}
  const knoten=[...bar.querySelectorAll('[data-resource]')],extras=knoten.filter(n=>!ROH_HAUPT.includes(n.dataset.resource));
  bar.dataset.rohstoffe='lebendig';bar.dataset.offen=String(roh.offen);
  let knopf=bar.querySelector('#rohstoff-mehr');
  if(extras.length&&!knopf){bar.insertAdjacentHTML('beforeend','<button type="button" id="rohstoff-mehr" class="rohstoff-mehr" aria-expanded="false"></button>');knopf=bar.querySelector('#rohstoff-mehr');}
  if(knopf){knopf.hidden=!extras.length;knopf.setAttribute('aria-expanded',String(roh.offen));const t=roh.offen?'▴ Weniger Rohstoffe':`▾ ${extras.length} weitere Rohstoffe`;if(knopf.textContent!==t)knopf.textContent=t;}
  // Laufende Förderung zählt still hoch. Aufstieg und Leuchtring nur für Ereignisse: Ausgabe (rot) und Zugewinn über
  // die Förderung hinaus (Beute, Lieferung, Markt). Vorher pulsierte bei Tempo 30 jede Kachel alle 1,6–3 s – Karl:
  // „der Kopf flackert, alle paar Sekunden kurz eine andere Anzeige“ (22.10.2026).
  const spielZeit=typeof GameTime!=='undefined'?GameTime.now():0,dt=Math.max(0,spielZeit-(roh.zeit??spielZeit));roh.zeit=spielZeit;let extraBewegt=0;
  for(const n of knoten){
   const g=n.dataset.resource,w=rohWert(g,p),extra=String(!ROH_HAUPT.includes(g));if(n.dataset.extra!==extra)n.dataset.extra=extra;
   const rate=g==='energie'?w:g==='credits'?0:Number(p.rate?.[g])||0,trend=rate>0?'plus':rate<0?'minus':'null';if(n.dataset.trend!==trend)n.dataset.trend=trend;
   if(!(g in roh.werte)){roh.werte[g]=w;continue;}
   const d=w-roh.werte[g];roh.werte[g]=w;if(!d)continue;
   const erwartet=g==='energie'?0:rate*dt/3600,ereignis=d-erwartet;
   // Abgang (Bau, Kauf, Versand): rot. Zugewinn nur, was die Förderung übersteigt (mit Spielraum für Rundung und Abrufabstand).
   const s=d<=-1&&g!=='energie'?d:ereignis>Math.max(50,Math.abs(erwartet)*2)?ereignis:g==='energie'&&Math.abs(d)>=1?d:0;
   if(!s)continue;
   if(n.dataset.extra==='true'&&!roh.offen){extraBewegt=extraBewegt||Math.sign(s);continue;}
   rohFlug(n,s,g);
  }
  if(knopf&&extraBewegt){knopf.classList.remove('r-puls','r-puls-minus');void knopf.offsetWidth;knopf.classList.add(extraBewegt>0?'r-puls':'r-puls-minus');}
 }
 document.addEventListener('click',e=>{if(!e.target.closest('#rohstoff-mehr'))return;e.preventDefault();e.stopPropagation();roh.offen=!roh.offen;try{localStorage.setItem(ROH_SPEICHER,roh.offen?'1':'0');}catch(_){}rohstoffe();},true);

 // Übersicht wie Karls Vorbild (OGame, 19.10.2026: „wo ist die aufgeräumte übersichtliche UI, in der man trotzdem alles
 // findet?“): oben der Planet groß mit seinen Kerndaten, darunter die drei Kästen Gebäude · Forschung · Werft mit dem,
 // was gerade läuft, oder „Keine … im Bau (Zur …)“. Am Handy treten die bisherigen Doppelungen zurück (rahmen.css).
 const ROEMISCH=['','I','II','III','IV','V'];
 // Hinweis-Chip statt Textkasten: Symbol, wenige Worte, ein Schritt; die Erklärung liegt im title.
 const CHIP_SYMBOL={labor:'M9 3h6M10 3v7l-6 9q-1 2 2 2h12q3 0 2-2l-6-9V3M8 15h8',werft:'M3 17h18l-3 4H6ZM6 17V9l6-5 6 5v8M12 4v13',kaserne:'M12 4a3 3 0 1 1 0 6 3 3 0 0 1 0-6ZM5 21v-3a7 7 0 0 1 14 0v3',markt:'M4 7h13l-3-3M20 17H7l3 3',rakete:'M12 2c3 3 4 7 4 11l-4 3-4-3c0-4 1-8 4-11ZM8 13l-3 4h4M16 13l3 4h-4M12 16v5'};
 function hinweisChip(symbol,kurz,lang,schritte){return `<div class="hinweis-chip" role="note" title="${esc(lang)}"><svg viewBox="0 0 24 24" aria-hidden="true"><path d="${CHIP_SYMBOL[symbol]||CHIP_SYMBOL.labor}"/></svg><span>${esc(kurz)}</span>${schritte}</div>`;}
 window.HinweisChip=hinweisChip;
 // Versorgung auf einen Blick (Karl 19.10.2026: „klar sehen, ob Wohnblocks, Konsumgüter, Nahrung, Akademien oder Labore
 // fehlen, ohne raten zu müssen“ – „keine Textbox mehr, Icons und Symbole, freundlich, einladend“): sechs Symbole mit
 // Ringanzeige (grün/gelb/rot), kurzer Wert, kleine Unterschrift; Antippen führt zum passenden Bau. Die Hauptbremse des
 // Wachstums (Kernprognose `wachstum.haupt_bremse`) pulsiert sanft.
 const VS_SYMBOL={
  wohnen:'M3 11 12 4l9 7M5 10v10h14V10M10 20v-6h4v6',
  nahrung:'M12 21V7M12 7c-2-1-3-2.6-3-4.5 2 0 3 1.2 3 3 0-1.8 1-3 3-3 0 1.9-1 3.5-3 4.5ZM12 12c-2-1-4-1-5-3 2-1 4 0 5 1.3M12 12c2-1 4-1 5-3-2-1-4 0-5 1.3M12 16.5c-2-1-4-1-5-3 2-1 4 0 5 1.3M12 16.5c2-1 4-1 5-3-2-1-4 0-5 1.3',
  strom:'M13 2 4 14h7l-1 8 9-12h-7Z',
  konsum:'M5 8h14l-1 12H6ZM9 8V6a3 3 0 0 1 6 0v2',
  fach:'M2 9l10-5 10 5-10 5ZM6 11v5c3 2 9 2 12 0v-5M22 9v6',
  forschung:'M9 3h6M10 3v7l-6 9q-1 2 2 2h12q3 0 2-2l-6-9V3M8 15h8'
 };
 function versorgungsLeiste(p){
  const g=k=>p.gebaeude?.[k]||0,rate=k=>Number(p.rate?.[k])||0,fp=Number(view.forschung?.punkte_je_stunde)||0;
  const kurzZahl=n=>n>=1e6?(n/1e6).toLocaleString('de-DE',{maximumFractionDigits:1})+' Mio.':n>=1e4?Math.round(n/1000).toLocaleString('de-DE')+' Tsd.':fmt(n);
  const ohneNahrung=view.volk==='syntheten',wa=p.wachstum||{},bremse=wa.haupt_bremse||'';
  const wohnAnteil=p.wohnraum?p.bevoelkerung/p.wohnraum:1,saldo=(p.energie?.erzeugung||0)-(p.energie?.verbrauch||0),bedarf=p.energie?.verbrauch||0;
  const konsumBaubar=(p.baubar||[]).some(b=>b.gebaeude==='konsumgueterwerk')||g('konsumgueterwerk')>0;
  const fk=p.fachkraefte||{},ga=p.grundausbildung||{};
  const punkte=[
   {k:'wohnen',name:'Wohnen',wert:(wohnAnteil>0&&wohnAnteil<.01?'<1':Math.round(wohnAnteil*100))+' %',anteil:1-Math.min(1,wohnAnteil),stufe:wohnAnteil<.8?'gut':wohnAnteil<.95?'knapp':'fehlt',ziel:'buildings:wohnblock',titel:`Wohnraum ${fmt(p.bevoelkerung)} / ${fmt(p.wohnraum)} – Wohnblock ausbauen für mehr Wachstum`,bremse:bremse==='wohnraum'},
   // Nahrung im Ring mit Einheit „/h“ je Spielstunde wie die Rohstoffleiste (Nachprüfung 25.10.2026); Echtzeit steht im Titel.
   ohneNahrung?null:{k:'nahrung',name:'Nahrung',wert:(rate('nahrung')>=0?'+':'−')+kurzZahl(Math.abs(rate('nahrung')))+'/h',anteil:Math.min(1,(p.nahrung_deckung||0)/100),stufe:(p.nahrung_deckung||0)<100?'fehlt':rate('nahrung')<0?'knapp':'gut',ziel:'buildings:farm',titel:`Nahrung ${p.nahrung_deckung??0} % gedeckt, ${rate('nahrung')>=0?'+':''}${fmt(rate('nahrung'))} je Spielstunde${(world?.tempo||1)>1?` (${fmt(rate('nahrung')*world.tempo)} je Stunde Echtzeit)`:''} – Farm ausbauen`,bremse:bremse==='versorgung'},
   {k:'strom',name:'Strom',wert:(saldo>=0?'+':'−')+fmt(Math.abs(saldo)),anteil:bedarf?Math.min(1,(p.energie?.erzeugung||0)/bedarf):1,stufe:saldo<0?'fehlt':bedarf&&saldo<bedarf*.08?'knapp':'gut',ziel:'buildings:'+(g('fusionskraftwerk')>0||view.stufe>=2&&(p.baubar||[]).some(b=>b.gebaeude==='fusionskraftwerk')?'fusionskraftwerk':'solarkraftwerk'),titel:`Energie ${fmt(p.energie?.erzeugung||0)} erzeugt, ${fmt(bedarf)} Bedarf – Kraftwerk ausbauen`,bremse:ohneNahrung&&bremse==='versorgung'},
   // Vor Zivilisationsstufe III verbraucht die Bevölkerung keine Konsumgüter (Kern: fruehphase, wie fk.zaehlt) – Ring ruht.
   fk.zaehlt===false?{k:'konsum',name:'Konsum',wert:'ab III',anteil:1,stufe:'ruht',ziel:'buildings:konsumgueterwerk',titel:'Vor Zivilisationsstufe III verbraucht die Bevölkerung keine Konsumgüter. Ab Stufe III brauchen alle 1.000 Einwohner Konsumgüter je Stunde – Konsumgüterwerke vorher ausbauen.',bremse:false}:
   {k:'konsum',name:'Konsum',wert:konsumBaubar?(p.konsum_deckung??0)+' %':'ab II',anteil:Math.min(1,(p.konsum_deckung||0)/100),stufe:!konsumBaubar?'ruht':(p.konsum_deckung||0)>=80?'gut':(p.konsum_deckung||0)>=40?'knapp':'fehlt',ziel:'buildings:konsumgueterwerk',titel:konsumBaubar?`Konsumgüter ${p.konsum_deckung??0} % gedeckt – stärken Stabilität und Wachstum; Konsumgüterwerk ausbauen`:'Konsumgüter gibt es ab Zivilisationsstufe II (Konsumgüterwerk)',bremse:bremse==='stabilitaet'&&konsumBaubar},
   // Vor Zivilisationsstufe III zählt Personal nicht (Kern: planer::wirtschaft::PERSONAL_AB_STUFE) – dann nie „fehlt“.
   {k:'fach',name:'Fachkräfte',wert:fk.zaehlt===false?`ab ${['','I','II','III','IV','V'][fk.ab_stufe]||'III'}`:fk.bedarf?`${fmt(fk.verfuegbar||0)}/${fmt(fk.bedarf)}`:fmt(fk.verfuegbar||0),anteil:fk.zaehlt===false?1:fk.bedarf?Math.min(1,(fk.verfuegbar||0)/fk.bedarf):1,stufe:fk.zaehlt===false||((fk.verfuegbar||0)>=(fk.bedarf||0)&&!(ga.fehlen_fachkraefte>0))?'gut':'fehlt',ziel:g('akademie')?'tab:kolonie':'buildings:akademie',titel:`Fachkräfte ${fmt(fk.verfuegbar||0)} von ${fmt(fk.bedarf||0)} benötigt – ${g('akademie')?'in der Akademie ausbilden':'Akademie bauen'}`},
   {k:'forschung',name:'Forschung',wert:fp>0?fmt(fp*Math.max(1,world?.tempo||1))+' FP':'0 FP',anteil:fp>0?1:0,stufe:!g('labor')&&!(view.planeten||[]).some(x=>x.gebaeude?.labor>0)?'fehlt':fp>0?'gut':'knapp',ziel:(view.planeten||[]).some(x=>x.gebaeude?.labor>0)?(fp>0?'research:energietechnik':'tab:kolonie'):'buildings:labor',titel:fp>0?`${fmt(fp*Math.max(1,world?.tempo||1))} Forschungspunkte je Echtzeitstunde – Labor ausbauen für mehr`:'Ohne Labor und Forscher wird nicht geforscht – Labor bauen, Forscher ausbilden'}
  ].filter(Boolean);
  const U=2*Math.PI*16;
  const knopf=x=>{const ziel=x.ziel.startsWith('tab:')?`data-tab="${x.ziel.slice(4)}"`:`data-bs-springe="${x.ziel}"`,voll=Math.max(.04,Math.min(1,x.anteil));
   return `<button type="button" class="vs vs-${x.stufe}${x.bremse?' vs-bremse':''}" ${ziel} title="${esc(x.titel)}" aria-label="${esc(x.titel)}"><svg class="vs-ring" viewBox="0 0 40 40" aria-hidden="true"><circle class="vs-spur" cx="20" cy="20" r="16"/><circle class="vs-fuell" cx="20" cy="20" r="16" stroke-dasharray="${(voll*U).toFixed(1)} ${U.toFixed(1)}" transform="rotate(-90 20 20)"/><g transform="translate(10 10) scale(.8333)"><path d="${VS_SYMBOL[x.k]}"/></g></svg><b>${esc(x.wert)}</b><small>${esc(x.name)}</small></button>`;};
  return `<div class="uk-versorgung" role="group" aria-label="Versorgung auf einen Blick">${punkte.map(knopf).join('')}</div>`;
 }
 // Wachstum als Pfeil am Bevölkerungswert: ↗ +48/Tag (grün), → 0 (gelb), ↘ −12 (rot)
 function wachstumsPfeil(p){
  const t=Number(p.wachstum?.je_tag);if(!Number.isFinite(t))return '';
  const k=t>0?'plus':t<0?'minus':'null',z=t>0?'↗':t<0?'↘':'→';
  return `<span class="uk-wachstum uk-w-${k}" title="Bevölkerung je Spieltag">${z} ${t>0?'+':t<0?'−':''}${fmt(Math.abs(t))}/Tag</span>`;
 }
 window.RahmenVersorgung=versorgungsLeiste;
 // Offene Vertragsangebote an dich mit Annehmen/Ablehnen – vorher nur in einem zugeklappten Abschnitt tief im
 // Briefkasten („Verträge mit Kaution verwalten“). Die Knöpfe nutzen den vorhandenen data-contract-Handler.
 // Antwortleiste: Annehmen ✓ / Ablehnen ✕ und eine schmale Zeile für eine persönliche Antwort, die als Brief mitgeht.
 function antwortLeiste(art,id,partner,thema){
  const d=`data-antwort="${art}" data-id="${esc(id)}" data-partner="${esc(partner||'')}" data-thema="${esc(thema||'')}"`;
  return `<div class="antwortleiste"><input class="antwort-text" type="text" maxlength="400" enterkeyhint="send" placeholder="Antwort an ${esc(partner||'…')} (optional)" aria-label="Antwort an ${esc(partner||'')} (optional)"><button type="button" class="antwort-ja" ${d} data-annehmen="true" aria-label="Annehmen">✓<span>Annehmen</span></button><button type="button" class="antwort-nein" ${d} data-annehmen="false" aria-label="Ablehnen">✕<span>Ablehnen</span></button></div>`;
 }
 window.AntwortLeiste=antwortLeiste;
 // Offene Vertragsangebote und diplomatische Anfragen an dich – vorher tief im Briefkasten versteckt.
 function angeboteKarte(){
  const nameVon=k=>typeof label==='function'?label(k):named(k);
  const anfragen=(view.kommunikation?.anfragen||[]).filter(r=>r.status==='offen'&&r.an===view.name);
  // Ein Vorgang, eine Karte: Verträge mit zugehöriger Anfrage (Kern verknüpft sie über „vertrag“) nur als Anfrage mit Text zeigen
  // (Funktionsprüfung 25.10.2026: derselbe Nichtangriffspakt stand zweimal da, nach Annahme blieb die zweite Karte).
  const verbunden=new Set(anfragen.map(r=>r.vertrag).filter(x=>x!=null).map(String));
  const vertraege=(view.vertraege||[]).filter(v=>v.status==='angebot an dich'&&!verbunden.has(String(v.vertrag)));
  if(!vertraege.length&&!anfragen.length)return '';
  const zeileV=v=>`<li><div class="angebot-kopf"><strong>${esc(nameVon(v.art))}</strong><small>von ${esc(v.partner||v.von||'unbekannt')}${Number(v.kaution)>0?` · Kaution ${esc(fmt(v.kaution))} Credits`:''}</small></div>${antwortLeiste('vertrag',v.vertrag,v.partner||v.von,nameVon(v.art))}</li>`;
  const zeileA=r=>`<li><div class="angebot-kopf"><strong>${esc(nameVon(r.art))}</strong><small>von ${esc(r.von)}${r.text?` · „${esc(String(r.text).slice(0,90))}“`:''}</small></div>${antwortLeiste('anfrage',r.id,r.von,nameVon(r.art))}</li>`;
  return `<section class="uk-angebote" aria-label="Angebote und Anfragen"><h3>Angebote an dich <b>${vertraege.length+anfragen.length}</b></h3><ul>${vertraege.map(zeileV).join('')}${anfragen.map(zeileA).join('')}</ul></section>`;
 }
 window.RahmenAngebote=angeboteKarte;
 // Schutz vor den Skriptbots (Server: view.bot_schutz, lauf::bots::schutz_sicht) – Karl 22.10.2026: „als Mensch über die
 // UI wehrlos, unfair“. Antippen zeigt den vollständigen Satz des Servers.
 function botSchutzChip(){
  // Opferschutz aus: kein Chip (Karl 23.10.2026: „stört und interessiert niemand, ich brauche den Platz“).
  const s=view?.bot_schutz;if(!s||s.aktiv===false||!s.mitspieler)return '';
  const tag=t=>Math.floor(t/86400)+1;
  const [klasse,text]=s.geschuetzt?(s.raubpause_bis_sekunden&&!s.grund?['schutz','🛡 Raubpause bis Tag '+tag(s.raubpause_bis_sekunden)]:['schutz','🛡 Schonzeit vor Bots']):['offen','⚔ Bots dürfen angreifen'];
  return `<button type="button" class="uk-schutz uk-schutz-${klasse}" data-bot-schutz title="${esc(s.text||'')}">${esc(text)}<small>${s.geschuetzt&&s.grund?esc(s.grund):s.geschuetzt?'':'fair: halbe Beute, Anflug ≥ 4 Spielstunden'}</small></button>`;
 }
 document.addEventListener('click',e=>{if(!e.target.closest('[data-bot-schutz]'))return;const t=view?.bot_schutz?.text;if(t&&typeof show==='function')show(t,view.bot_schutz.aktiv===false);});
 function uebersichtKopf(){
  const reich=$('reich');if(!reich||!window.Bauseite?.schlangen)return;
  const p=currentPlanet?.();if(!p)return;
  let host=$('uebersicht-kopf');
  if(!host){reich.querySelector('.row')?.insertAdjacentHTML('afterend','<section id="uebersicht-kopf" aria-label="Übersicht"></section>');host=$('uebersicht-kopf');if(!host)return;}
  const energie=(p.energie?.erzeugung||0)-(p.energie?.verbrauch||0),wohn=Math.max(1,p.wohnraum||1),anteil=Math.min(100,Math.round((p.bevoelkerung||0)/wohn*100));
  const fakt=(name,wert,klasse='')=>`<div class="uk-fakt ${klasse}"><dt>${esc(name)}</dt><dd>${wert}</dd></div>`;
  const html=`<div class="uk-held"><div class="uk-bild kugel-host" id="uk-kugel"><img src="${esc(planetArt(p))}" alt="" loading="lazy" decoding="async"><canvas class="planet-kugel" aria-hidden="true"></canvas></div>
   <div class="uk-titel"><small>Übersicht · ${p.heimat?'Heimatwelt':'Kolonie'}</small><h2><span class="uk-planetenname">${esc(planetName(p,0))} <span class="koord">[${esc(p.koord)}]</span></span>${window.Planetenname?Planetenname.stift(p.koord,'uk-stift'):''}</h2><span>${esc(named(p.zone))} · ${esc(named(view.volk))} · Stufe ${ROEMISCH[view.stufe]||view.stufe}</span></div>
   <dl class="uk-fakten">${fakt('Bevölkerung',`${fmt(p.bevoelkerung)} <small>/ ${fmt(p.wohnraum)}</small>${wachstumsPfeil(p)}<i class="uk-balken"><b data-anteil="${Math.round(anteil/5)*5}"></b></i>`,'uk-breit')}
    ${fakt('Energie',`${energie>=0?'+':'−'}${fmt(Math.abs(energie))}`,energie>=0?'uk-plus':'uk-minus')}${fakt('Baufelder',`${p.felder.belegt} / ${p.felder.gesamt}`)}
    ${fakt('Stabilität',`${p.stabilitaet} %`,p.stabilitaet<40?'uk-minus':'')}${fakt('Punkte',`${fmt(view.punkte?.gesamt||0)} <small>#${esc(view.rang??'–')}</small>`)}</dl>${versorgungsLeiste(p)}${botSchutzChip()}</div>
   ${Bauseite.schlangen(p).replace('Keine Gebäude im Bau.','Frei.').replace('Keine Forschung läuft.','Frei.').replace('Keine Schiffe oder Verteidigung im Bau.','Frei.').replace('<h4>Werft / Verteidigung</h4>','<h4>Werft</h4>').replace('(Zur Versorgung)','Versorgung ›').replace('(Zur Forschung)','Forschung ›').replace('(Zur Schiffswerft)','Werft ›')}${typeof flottenTabelle==='function'?flottenTabelle():''}`;
  setStableHTML(host,html);
  window.PlanetKugel?.einbinden($('uk-kugel'),{planet:p,mond:p.mond,ansicht:'planet',...(typeof mondLandung==='function'?mondLandung(p):{})});
  // Angebote in eigenem Halter: nur neu zeichnen, wenn sich die Angebote ändern und niemand gerade eine Antwort tippt
  let a=$('uk-angebote-halter');
  if(!a){host.insertAdjacentHTML('afterend','<div id="uk-angebote-halter"></div>');a=$('uk-angebote-halter');}
  const tippt=[...a.querySelectorAll('.antwort-text')].some(i=>i.value||i===document.activeElement);
  if(!tippt)setStableHTML(a,angeboteKarte());
 }

 // Öffentlicher Markt ohne Marktgebäude: vorher sagen, statt dass „Order einstellen“ erst nach dem Absenden scheitert.
 function marktHinweis(){
  const form=$('market-form'),p=currentPlanet?.();if(!form||!p)return;
  let h=$('markt-hinweis');const fehlt=!(p.gebaeude?.markt>0);
  if(!fehlt){if(h)h.remove();return;}
  if(!h){form.insertAdjacentHTML('beforebegin','<div id="markt-hinweis"></div>');h=$('markt-hinweis');}
  setStableHTML(h,hinweisChip('markt','Marktgebäude fehlt – Soforthandel geht','Orders auf dem öffentlichen Markt brauchen das Gebäude Markt auf diesem Planeten (ab Zivilisationsstufe III). Bis dahin tauschst du sofort über den Soforthandel oben.','<button type="button" class="bs-schritt" data-bs-springe="buildings:markt">Markt ›</button>'));
 }

 // Allianzseite verweist auf das Großprojekt (Konsolidierung M10): vorher nur über Themenblatt → Weitere erreichbar.
 function grossprojektKarte(){
  const sec=$('allianzbereich');if(!sec)return;
  let k=$('grossprojekt-karte');const dy=view?.dyson;
  if(!dy?.aktiv){if(k)k.remove();return;}
  if(!k){sec.insertAdjacentHTML('afterbegin','<button type="button" id="grossprojekt-karte" class="grossprojekt-karte" data-tab="grossprojekt"></button>');k=$('grossprojekt-karte');}
  const pr=(dy.projekte||[]).find(x=>x.phase!=='abgebrochen'),f=pr?.phase==='bau'?Number(pr?.fortschritt_promille)/1000:NaN;
  const zustand=pr?`${{sammeln:'Einzahlung läuft',bau:'im Bau',vollendet:'vollendet',verwaist:'verwaist'}[pr.phase]||pr.phase}${Number.isFinite(f)?` · ${Math.round(f*100)} %`:''}`:'noch kein Projekt – jetzt vorschlagen';
  setStableHTML(k,`<span class="gk-titel">Großprojekt Dysonsphäre</span><span class="gk-stand">${esc(zustand)}</span><span class="gk-pfeil" aria-hidden="true">›</span>`);
 }

 // Raketenstart ohne Raketensilo: vorher sagen und Start sperren (Galaxie-Sitzung Nr. 9 – das Formular erschien,
 // „Start im Core prüfen“ scheiterte erst mit „Nicht genügend fertige Raketen“).
 function raketenHinweis(){
  const form=$('missile-launch'),p=currentPlanet?.();if(!form||!p)return;
  let h=$('raketen-hinweis');const ohne=!(p.raketensilo?.kapazitaet>0);
  for(const b of form.querySelectorAll('button'))if(ohne){b.disabled=true;b.title='Kein Raketensilo auf diesem Planeten';b.dataset.siloGesperrt='1';}else if(b.dataset.siloGesperrt){b.disabled=false;b.removeAttribute('title');delete b.dataset.siloGesperrt;}
  if(!ohne){if(h)h.remove();return;}
  if(!h){form.insertAdjacentHTML('beforebegin','<div id="raketen-hinweis"></div>');h=$('raketen-hinweis');}
  setStableHTML(h,hinweisChip('rakete','Raketensilo fehlt','Auf diesem Planeten steht kein Raketensilo. Raketen werden im Silo gebaut, gelagert und von dort gestartet – Abfangraketen schützen dann auch diesen Planeten.','<button type="button" class="bs-schritt" data-bs-springe="buildings:raketensilo">Silo bauen ›</button>'));
 }

 function versorgungAufKolonie(){
  const sec=$('kolonie'),p=currentPlanet?.();if(!sec||!p)return;
  let h=$('kolonie-versorgung');
  if(!h){sec.insertAdjacentHTML('afterbegin','<div id="kolonie-versorgung"></div>');h=$('kolonie-versorgung');}
  setStableHTML(h,versorgungsLeiste(p));
 }

 function render(){
  if(!view||session?.spieler==null)return;
  versorgungAufKolonie();
  grossprojektKarte();raketenHinweis();
  einrichten();kopfzeile();ereignisleiste();planetenliste();synchronisieren();rohstoffe();uebersichtKopf();marktHinweis();
 }

 function navigieren(id){return UMLEITUNG[id]||id;}
 // Thema im Blatt gewählt: Leiste wechselt; liegt die aktuelle Seite nicht im Thema, öffnet sich dessen erstes Ziel.
 // Zurück-Taste (Android): erst offene Ebenen (Themenblatt, dann angemeldete Haken anderer Module, z. B. Dossier oder
 // 3D-Ansicht – ein Haken gibt true zurück, wenn er etwas geschlossen hat), dann die vorige Seite, zuletzt die Übersicht.
 // Erst auf der Übersicht ohne Verlauf gibt sie false zurück; dann entscheidet die App (Startseite).
 const verlauf=[],zurueckHaken=[];let ohneVerlauf=false;
 function merken(alt,neu){if(!ohneVerlauf&&alt&&alt!==neu){verlauf.push(alt);if(verlauf.length>30)verlauf.shift();}}
 function zurueck(){
  if($('themenblatt')&&!$('themenblatt').hidden){themenblatt(false);return true;}
  // Ebenen vor Seitenzustand: Ein modaler Dialog sperrt alles andere und schließt zuerst; dann Blätter und Karten (Haken,
  // zuletzt angemeldet zuerst), dann nicht-modale Dialoge; erst danach Haken, die nur die Seite ändern (h.seite, z. B. der
  // Zoom der Galaxiekarte) – sonst zoomte Zurück die Karte, während ein Bericht oder das Einsatzblatt darüber offen war.
  const dialoge=[...document.querySelectorAll('dialog[open]')],modal=dialoge.filter(d=>{try{return d.matches(':modal');}catch(_){return false;}}).pop();
  if(modal){modal.close();return true;}
  const haken=[...zurueckHaken].reverse();
  for(const h of haken.filter(h=>!h.seite)){try{if(h())return true;}catch(_){}}
  const dialog=dialoge.pop();if(dialog){dialog.close();return true;}
  for(const h of haken.filter(h=>h.seite)){try{if(h())return true;}catch(_){}}
  const detailZu=document.querySelector('#game>section:not([hidden]) [data-bs-schliessen]');if(detailZu){detailZu.click();return true;}
  let ziel;while((ziel=verlauf.pop())&&(ziel===aktiverBereich()||!$(ziel)||ziel==='reich'));verlauf.length=0;
  const gehe=id=>{ohneVerlauf=true;try{navigateGame(id);}finally{ohneVerlauf=false;}};
  if(ziel){gehe(ziel);window.scrollTo({top:0,behavior:'instant'});return true;}
  if(aktiverBereich()!=='reich'){gehe('reich');window.scrollTo({top:0,behavior:'instant'});return true;}
  return false;
 }
 function themaWaehlen(id){const t=THEMEN.find(x=>x.id===id);if(!t)return;themaSetzen(id);themenblatt(false);if(themaVon(aktiverBereich())!==id){navigateGame(t.punkte[0][0]);window.scrollTo({top:0,behavior:'instant'});}}
 return {render,synchronisieren,einrichten,navigieren,MENUE,THEMEN,themenblatt,themaWaehlen,themaSetzen,zurueck,zurueckHaken,merken,aktiverBereich};
})();
// Auch als Fenstereigenschaft, damit andere Module optional darauf zugreifen können.
window.Rahmen=Rahmen;

// Navigation: bisherige Funktion bleibt zuständig, der Rahmen gleicht Menü, Reiter und Kopfzeile an.
{
 const bisher=navigateGame;
 navigateGame=function(id){const alt=Rahmen.aktiverBereich();bisher(Rahmen.navigieren(id));Rahmen.merken(alt,Rahmen.aktiverBereich());Rahmen.synchronisieren();document.getElementById('tabs')?.classList.remove('mehr-offen');};
 const ansicht=renderView;
 renderView=function(){ansicht.apply(this,arguments);Rahmen.render();};
 // Die Planetenliste rechts ersetzt Favoriten und Klappliste; Auswahl weiter über PlanetNavigation.select.
 PlanetNavigation.render=function(){Rahmen.render();};
}
document.addEventListener('click',e=>{
 if(e.target.closest('#message.toast')){document.getElementById('message').hidden=true;return;}
 // Themenleiste (Handy): Themenknopf öffnet das Blatt, Themenkarte wechselt das Thema, Cockpit öffnet das Handy-Cockpit.
 if(e.target.closest('[data-themen-blatt]')){Rahmen.themenblatt(document.getElementById('themenblatt')?.hidden!==false);return;}
 if(e.target.closest('[data-themen-blatt-zu]')||e.target.id==='themenblatt'){Rahmen.themenblatt(false);return;}
 const th=e.target.closest('#themenblatt [data-thema]');if(th){Rahmen.themaWaehlen(th.dataset.thema);return;}
 // Ziel direkt aus einer Themenkarte: das gewählte Thema wird zur Leiste (ausdrückliche Wahl im Menü), dann Seitenwechsel unten.
 const zielThema=e.target.closest('#themenblatt [data-blatt-thema]');if(zielThema)Rahmen.themaSetzen?.(zielThema.dataset.blattThema);
 if(e.target.closest('[data-blatt-cockpit]')){Rahmen.themenblatt(false);document.querySelector('#tabs [data-menue="cockpit"]')?.click();return;}
 // Reiter, Kopfzeile und Ereignisleiste: Bereichsknöpfe außerhalb des Hauptmenüs (das Hauptmenü bedient app.js).
 const b=e.target.closest('[data-tab]');
 if(b&&!b.closest('#tabs')){e.preventDefault();if(b.closest('#themenblatt'))Rahmen.themenblatt(false);navigateGame(b.dataset.tab);window.scrollTo({top:0,behavior:'instant'});return;}
 const mehr=e.target.closest('[data-menue-mehr]');
 if(mehr){const nav=$('tabs'),offen=!nav.classList.contains('mehr-offen');nav.classList.toggle('mehr-offen',offen);mehr.setAttribute('aria-expanded',String(offen));return;}
 if(e.target.closest('#tabs [data-tab]'))window.scrollTo({top:0,behavior:'instant'});
});
document.addEventListener('click',async e=>{
 const b=e.target.closest('[data-antwort]');if(!b)return;e.preventDefault();e.stopPropagation();
 const leiste=b.closest('.antwortleiste'),text=leiste?.querySelector('.antwort-text')?.value.trim()||'',ja=b.dataset.annehmen==='true',id=Number(b.dataset.id);
 const aktionen=[b.dataset.antwort==='vertrag'?{typ:ja?'vertrag_annehmen':'vertrag_ablehnen',vertrag:id}:{typ:'diplomatie_entscheiden',anfrage:id,annehmen:ja}];
 if(text&&b.dataset.partner)aktionen.push({typ:'brief_senden',kanal:'privat',an:b.dataset.partner,betreff:(ja?'Zusage: ':'Absage: ')+(b.dataset.thema||'Diplomatie'),text});
 for(const k of leiste?.querySelectorAll('button')||[])k.disabled=true;
 try{await command(aktionen);}catch(err){show(err.message,true);for(const k of leiste?.querySelectorAll('button')||[])k.disabled=false;return;}
 // Beantwortet: Zeile sofort weg – sonst hielte der getippte Text die Karte (Schutz beim Tippen) samt Zähler fest.
 const zeile=leiste?.closest('li'),karte=zeile?.closest('.uk-angebote');zeile?.remove();
 if(karte&&!karte.querySelector('li'))karte.remove();else{const z=karte?.querySelector('h3 b');if(z)z.textContent=String(karte.querySelectorAll('li').length);}
},true);
document.addEventListener('keydown',e=>{if(e.key!=='Enter'||!e.target.matches?.('.antwort-text'))return;e.preventDefault();e.target.closest('.antwortleiste')?.querySelector('.antwort-ja')?.click();});
document.addEventListener('DOMContentLoaded',()=>{Rahmen.einrichten();});
document.addEventListener('game-navigation',()=>Rahmen.synchronisieren());
// Sprungknopf in der Ereignisleiste: Zusammenfassung nicht auf-/zuklappen, der Bodenkampf-Handler öffnet das Blatt.
document.addEventListener('click',e=>{if(e.target.closest?.('summary .ereignis-sprung'))e.preventDefault();},true);
