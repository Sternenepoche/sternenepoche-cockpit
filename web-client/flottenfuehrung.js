// Flottenführung (Karl, 18.10.2026, nach der Videoanalyse laeufe/flottenfuehrung-video-2026-10-18):
// Der Flottenversand läuft in drei Schritten – ① Ziel & Auftrag ② Flotte ③ Zeit & Verband – mit einer festen
// Leiste unten (Auftragssymbol, Ziel, Zurück/Weiter/Starten). Dazu Bedarfsrechner (Frachter, Recycler, Sonden,
// Simulation), Verbandsbeitritt beim Versand mit Verzögerungsvorschau je Tempo, Ankunft an eine andere Flotte
// koppeln (Recycler Sekunden nach dem Einschlag), Zusammenarbeit (Verbände, Halten) und eine Zieltabelle aus
// allen Spionageberichten. Das bestehende Formular #fleet bleibt die einzige Quelle der Eingaben: Felder werden
// nur umgehängt, nicht ersetzt, damit Vorschau, Vorlagen und Senden aus mobile035.js/app.js unverändert greifen.
(function(){
 'use strict';
 const A=()=>window.Auftraege;
 const el=id=>document.getElementById(id);
 const PLUENDER_GUETER=['erz','kristall','deuterium','nahrung','legierung','elektronik','konsumgut','xenokristall'];
 const ROH=['erz','kristall','deuterium','nahrung'];
 const FRACHTER=['grosser_transporter','kleiner_transporter','schwerlastfrachter','schnellfrachter','handelsfrachter'];
 const BUNKER_JE_STUFE=20000,BUNKER_VERARBEITET=0.2,VERBAND_MAX=16;
 let startLaeuft=false,startUhr=0,schritt=1,gebaut=false,zielInfo=null,zielSchluessel='',verbandWahl='',kopplung={an:'',versatz:5},sim=null,simSchluessel='',tempoTabelle=null,tempoSchluessel='',tempoLaeuft=false,sortierung={feld:'beute',ab:true};
 const galaxie=new Map(),ladeFaktor={wert:null,schluessel:''};

 // ---------- Hilfen ----------
 const h=s=>typeof esc==='function'?esc(s):String(s??'');
 const zahl=n=>typeof fmt==='function'?fmt(Math.round(n)):String(Math.round(n));
 const dauer=s=>{s=Math.max(0,Math.round(s));if(typeof GameTime!=='undefined'&&GameTime.duration)return GameTime.duration(s);return [Math.floor(s/3600),Math.floor(s/60)%60,s%60].map(n=>String(n).padStart(2,'0')).join(':');};
 const uhr=t=>'Tag '+(Math.floor(t/86400)+1)+' · '+[Math.floor(t/3600)%24,Math.floor(t/60)%60,Math.floor(t)%60].map(n=>String(n).padStart(2,'0')).join(':');
 const name=k=>typeof named==='function'?named(k):k;
 const katalog=k=>(typeof rules!=='undefined'&&rules?.catalog?.einheiten?.[k]?.regel)||{};
 const formular=()=>el('fleet');
 // Start vom Mond: Schiffsbestand des Mondes (sonst „fehlt am Startplaneten: Weltenkrümmer“, obwohl sie dort stehen)
 const startPlanet=()=>{const f=formular()?.elements,p=view?.planeten?.find(x=>x.koord===f?.start.value);if(p&&f?.startart?.value==='mond'){const m=(view.monde?.eigene||[]).find(x=>x.koord===p.koord);if(m)return {...p,schiffe:m.schiffe||{},vomMond:true};}return p;};
 const auftrag=()=>formular()?.elements.mission.value||'';
 const ziel=()=>(formular()?.elements.ziel.value||'').trim();
 const zielGueltig=()=>/^\d+:\d+:\d+$/.test(ziel());
 function schiffe(){const out={};el('ship-inputs')?.querySelectorAll('input[data-map]').forEach(i=>{const n=Number(i.value)||0;if(n>0)out[i.dataset.map]=n;});return out;}
 function fracht(){let s=0;el('cargo-inputs')?.querySelectorAll('input[data-map]').forEach(i=>s+=Number(i.value)||0);return s;}
 function setzeSchiff(k,n){const i=el('ship-inputs')?.querySelector(`input[data-map="${k}"]`);if(!i)return false;i.value=Math.max(0,Math.round(n));i.dispatchEvent(new Event('input',{bubbles:true}));i.dispatchEvent(new Event('change',{bubbles:true}));return true;}
 function wert(kosten){return (kosten?.erz||0)+(kosten?.kristall||0)+(kosten?.deuterium||0);}
 function flottenWert(map){return Object.entries(map||{}).reduce((s,[k,n])=>s+wert(katalog(k).kosten)*(Number(n)||0),0);}
 function zielBericht(k){return (view?.berichte||[]).filter(b=>b.ziel===k).sort((a,b)=>b.zeit-a.zeit)[0]||null;}
 function eigeneQuote(){return Number(rules?.catalog?.pluenderung?.quote_je_volk?.[view?.volk])||(view?.volk==='krath'?0.5:0.4);}
 function beute(b){
  if(!b?.bestand)return null;const stufe=Number(b.gebaeude?.bunker)||0,voll=(Number(rules?.catalog?.pluenderung?.bunker_je_stufe)||BUNKER_JE_STUFE)*stufe,q=eigeneQuote();let summe=0;const je={};
  for(const g of PLUENDER_GUETER){const schutz=ROH.includes(g)?voll:voll*(Number(rules?.catalog?.pluenderung?.bunker_verarbeitet_anteil)||BUNKER_VERARBEITET);const n=Math.max(0,(Number(b.bestand[g])||0)-schutz)*q;je[g]=n;summe+=n;}
  return {summe,je,bunker:stufe,quote:q};
 }
 function sichtbar(node,ja){if(node)node.hidden=!ja;}

 // ---------- Zielerkennung über die Galaxie-Abfrage ----------
 // Wissensstand des Spielers: ein neuer Spionagebericht oder eine Erkundung macht Zwischenspeicher sofort ungültig – sonst
 // meldete der Assistent nach eingetroffenem Bericht bis zu 20 s lang „Ziel nicht aufgeklärt“ und sperrte den Angriff
 // (Spielprobe 20.10.2026, schnelle Flüge im eigenen System).
 const wissen=()=>[(view?.berichte||[]).length,Math.max(0,...(view?.berichte||[]).map(b=>Number(b.zeit)||0)),(view?.erkundet||[]).length,(view?.erkundet||[]).filter(e=>e.bekannt).length].join(':');
 async function systemDaten(sektor,system){
  const k=sektor+':'+system,alt=galaxie.get(k),w=wissen();if(alt&&Date.now()-alt.zeit<20000&&alt.wissen===w)return alt.daten;
  const d=await api('/api/tool',{typ:'galaxie',sektor,von:system,bis:system});galaxie.set(k,{zeit:Date.now(),wissen:w,daten:d});return d;
 }
 async function zielErkennen(){
  const k=ziel();if(!zielGueltig()){zielInfo=null;zielSchluessel='';return;}
  if(k===zielSchluessel&&zielInfo&&zielInfo.wissen===wissen())return;zielSchluessel=k;
  const [s,sy,pos]=k.split(':').map(Number);
  const info={koord:k,art:'unbekannt',spieler:null,system_bekannt:false,ausgespaeht:false,alter:null,truemmer:null,mond:null,guertel:pos===0,heimat:false,wissen:wissen()};
  try{
   const d=await systemDaten(s,sy),sys=d.systeme?.[0];info.system_bekannt=!!sys?.bekannt;
   const p=(sys?.plaetze||[]).find(x=>x.koord===k);
   if(pos===0)info.art='guertel';
   else if(p){
    info.spieler=p.spieler||null;info.truemmer=p.truemmer||null;info.mond=p.mond||null;info.heimat=!!p.heimat;info.klima=p.habitabilitaet||null;info.bekannt=p.bekannt;
    const bez=p.beziehung?.karte_status||p.status;
    if(p.status==='eigen')info.art='eigen';
    else if(['verbuendet','allianz','buendnis','verbündet'].includes(bez))info.art='verbuendet';
    else if(p.bewohnt===false&&p.besiedlung_bekannt)info.art=p.truemmer?'truemmer':'leer';
    else if(p.status==='unbekannt'&&!p.spieler)info.art=sys?.bekannt?'leer':'unerkundet';
    else info.art='fremd';
    if(p.alter_stunden!=null){info.ausgespaeht=true;info.alter=p.alter_stunden;}
   }
  }catch{info.art='unbekannt';}
  if(view?.planeten?.some(p=>p.koord===k))info.art='eigen';
  const b=zielBericht(k);if(b){info.ausgespaeht=true;info.alter=b.alter_stunden;}
  const tf=(view?.truemmer||[]).find(t=>t.koord===k);if(tf)info.truemmer={erz:tf.erz,kristall:tf.kristall};
  if(ziel()===k)zielInfo=info;
 }

 // ---------- Aufbau: Formular in drei Schritte gliedern ----------
 function label(name){return formular()?.elements[name]?.closest('label')||null;}
 function auswahlDetails(innerId){return el(innerId)?.closest('details.fleet-selection')||el(innerId)?.closest('fieldset');}
 function bauen(){
  const f=formular();if(gebaut||!f||!el('ship-inputs')||!window.Auftraege)return;
  if(!f.elements.start||!f.elements.mission)return;
  gebaut=true;f.classList.add('ff-aktiv');
  const kopf=document.createElement('nav');kopf.id='ff-schritte';kopf.className='ff-schritte';kopf.setAttribute('aria-label','Schritte des Flottenversands');
  kopf.innerHTML=[['1','Ziel & Auftrag','Ziel'],['2','Flotte','Flotte'],['3','Zeit & Verband','Zeit']].map(([n,t,k])=>`<button type="button" data-ff-schritt="${n}"><span>${n}</span><em class="ff-lang">${t}</em><em class="ff-kurz">${k}</em></button>`).join('');
  const s=[1,2,3].map(n=>{const x=document.createElement('section');x.className='ff-schritt';x.dataset.schritt=n;return x;});
  const neu=(id,cls='')=>{const d=document.createElement('div');d.id=id;if(cls)d.className=cls;return d;};
  f.prepend(kopf);kopf.after(...s);
  // Schritt 1
  s[0].append(neu('ff-zielwahl','ff-block'));
  for(const n of ['start','ziel','startart','zielart'])if(label(n))s[0].querySelector('#ff-zielwahl').append(label(n));
  s[0].append(neu('ff-schnellziele','ff-chips'),neu('ff-zielkarte','ff-block'),neu('ff-auftraege','ff-auftragsraster'));
  if(label('mission')){label('mission').classList.add('ff-versteckt');s[0].append(label('mission'));}
  // Schritt 2
  const schiffWahl=auswahlDetails('ship-inputs'),frachtWahl=auswahlDetails('cargo-inputs');
  s[1].append(neu('ff-bedarf','ff-block ff-bedarf'));
  if(schiffWahl){schiffWahl.open=true;s[1].append(schiffWahl);}
  for(const node of [frachtWahl,label('siedler_volk'),el('ground-load'),f.querySelector('.return-fuel-check')])if(node)s[1].append(node);
  // Schritt 3
  s[2].append(neu('ff-tempo','ff-block'));
  if(label('geschwindigkeit')){label('geschwindigkeit').classList.add('ff-versteckt');s[2].append(label('geschwindigkeit'));}
  s[2].append(neu('ff-verband','ff-block'),neu('ff-kopplung','ff-block'));
  for(const node of [f.querySelector('.flight-timing'),label('haltedauer_stunden'),el('fleet-live-summary'),el('flight-quote'),el('flight-preview')?.closest('.row')])if(node)s[2].append(node);
  const leiste=document.createElement('div');leiste.id='ff-leiste';leiste.className='ff-leiste';f.append(leiste);
  el('fleet-planner')?.querySelector('summary')?.insertAdjacentHTML('beforeend','<span class="ff-summary-hinweis"> · Schritt für Schritt</span>');
  zeigeSchritt(1);
 }
 function zeigeSchritt(n){
  schritt=Math.min(3,Math.max(1,n));const f=formular();if(!f)return;
  f.querySelectorAll('.ff-schritt').forEach(x=>x.hidden=Number(x.dataset.schritt)!==schritt);
  f.querySelectorAll('[data-ff-schritt]').forEach(b=>{const k=Number(b.dataset.ffSchritt);b.classList.toggle('aktiv',k===schritt);b.classList.toggle('erledigt',k<schritt);b.setAttribute('aria-current',k===schritt?'step':'false');});
  alles();
 }
 // Kernregel (flotte.rs, Aufklärungsregeln): fremde Ziele – auch Verbündete – brauchen eigenes Planetenwissen (Scan,
 // Spionagebericht oder Erkundung), außer bei Erkunden/Spionage/Saven/Abbau/Recyceln und Halten bei Verbündeten.
 // Vorher meldete erst der Server den Fehler, der Startknopf blieb aktiv (Galaxie-Sitzung 19.10.2026).
 const AUFKLAERUNG_FREI=['system_erkunden','spionage','flotten_spionage','saven','abbau','recyceln'];
 function aufklaerungFehlt(){
  const k=ziel(),m=auftrag();if(!view?.aufklaerungsregeln||!zielGueltig()||!m||AUFKLAERUNG_FREI.includes(m))return '';
  // Mondflüge prüft der Kern eigenständig (monde.rs mond_flotte_senden): dort genügt ein kartiertes Sonnensystem.
  const mondziel=formular()?.elements.zielart?.value==='mond'||m==='mond_zerstoeren';
  if(mondziel)return zielInfo&&zielInfo.koord===k&&zielInfo.system_bekannt===false&&!view.planeten?.some(p=>p.koord===k)?`System von [${k}] ist nicht kartiert: zuerst eine Spionagesonde mit Auftrag „System erkunden“ hinschicken.`:'';
  // Eigener Bericht oder eigene Erkundung des Ziels zählt sofort (Kern: planetenwissen kennt Berichte und Erkundungen)
  if(view.planeten?.some(p=>p.koord===k)||(view.berichte||[]).some(b=>b.ziel===k)||(view.erkundet||[]).some(e=>e.koord===k&&e.bekannt))return '';
  if(!zielInfo||zielInfo.koord!==k||zielInfo.bekannt!==false)return '';
  if(m==='halten'&&zielInfo.art==='verbuendet')return '';
  return `Ziel [${k}] ist nicht aufgeklärt: zuerst eine Spionagesonde mit Auftrag „Spionage“ hinschicken – das gilt auch für Verbündete.`;
 }
 function schrittFehler(n){
  if(n>=1){if(!zielGueltig())return 'Ziel als Koordinate angeben, z. B. 1:58:6';if(!auftrag())return 'Auftrag wählen';const af=aufklaerungFehlt();if(af)return af;}
  if(n>=2&&!Object.keys(schiffe()).length)return 'Mindestens ein Schiff auswählen';
  // Verkauf braucht einen Preis – ohne ist es ein Geschenk (wie einsatz.js; Funktionsprüfung 25.10.2026).
  if(n>=3&&auftrag()==='schiffhandel'&&!(Number($('fl-preis')?.value)>0))return 'Kaufpreis eingeben oder „Schiffe verschenken“ wählen';
  if(n>=3){const p=window.Mobile035?.letztePruefung?.();if(!p)return 'Flug wird geprüft …';if(!p.bereit)return p.grund||'Startbedingungen nicht erfüllt';}
  return '';
 }

 // ---------- Schritt 1: Ziel, Schnellziele, Zielkarte, Auftragsraster ----------
 function verlauf(){try{const raw=JSON.parse(localStorage.getItem('sternenepoche-flights:'+world.world_id+':'+view.name)||'{}');return [...(raw.pins||[]),...(raw.recent||[])];}catch{return [];}}
 function schnellziele(){
  const host=el('ff-schnellziele');if(!host||!view)return;
  const chips=[],gesehen=new Set(),chip=(k,text,m,titel)=>{if(!k||gesehen.has(k+m))return;gesehen.add(k+m);chips.push(`<button type="button" class="ff-chip${ziel()===k?' aktiv':''}" data-ff-ziel="${h(k)}" data-ff-auftrag="${h(m||'')}" title="${h(titel||'')}">${m?A().badge(m,{size:16}):''}<span>${h(text)}</span><small>[${h(k)}]</small></button>`);};
  for(const v of (view.verbaende||[]).filter(v=>v.beitretbar))chip(v.ziel,'Verband '+v.spieler,'verband','Offener Verband · '+v.flotten+'/'+VERBAND_MAX+' Flotten');
  for(const a of verlauf().slice(0,5))chip(a.ziel,name(a.mission),a.mission,'Zuletzt geflogen');
  for(const b of [...(view.berichte||[])].sort((a,b)=>(beute(b)?.summe||0)-(beute(a)?.summe||0)).slice(0,4))chip(b.ziel,b.besitzer||'Ausgespäht','angriff','Ausgespäht vor '+b.alter_stunden+' h');
  for(const t of (view.truemmer||[]).slice(0,3))chip(t.koord,'Trümmer '+zahl(t.erz+t.kristall),'recyceln','Trümmerfeld');
  const start=formular()?.elements.start.value;
  for(const p of view.planeten||[])if(p.koord!==start)chip(p.koord,p.name||'Eigener Planet','stationieren','Eigener Planet');
  host.innerHTML=chips.length?`<div class="ff-chips-titel">Schnellziele</div>${chips.join('')}`:'';
 }
 const ART_TEXT={eigen:'Eigener Planet',verbuendet:'Verbündeter Planet',fremd:'Fremder Planet',leer:'Freier Platz',truemmer:'Trümmerfeld',guertel:'Asteroidengürtel · Position 0',unerkundet:'Unerkundetes System',unbekannt:'Ziel wird geprüft …'};
 function zielkarte(){
  const host=el('ff-zielkarte');if(!host)return;
  if(!zielGueltig()){host.innerHTML='<p class="ff-leise">Ziel eingeben oder ein Schnellziel antippen. Die Karte zeigt dann, was dort ist und welche Aufträge passen.</p>';return;}
  const z=zielInfo;if(!z||z.koord!==ziel()){host.innerHTML='<p class="ff-leise">Ziel wird geprüft …</p>';return;}
  const b=zielBericht(z.koord),bt=beute(b),fe=['fremd','unerkundet'].includes(z.art);
  const kette=fe?`<ol class="ff-kette">${[[z.system_bekannt,'System erkundet','system_erkunden'],[z.ausgespaeht,z.ausgespaeht?`Ausgespäht vor ${z.alter??'?'} h`:'Planet ausgespäht','spionage'],[z.system_bekannt&&z.ausgespaeht,'Angriff freigegeben','angriff']].map(([ok,t,m],i)=>`<li class="${ok?'ok':''}">${A().badge(m,{size:18})}<span>${h(t)}</span>${!ok&&i<2&&(i===0||z.system_bekannt)?`<button type="button" class="ff-mini" data-ff-aufklaeren="${m}">Jetzt ${i===0?'erkunden':'ausspähen'}</button>`:''}</li>`).join('')}</ol><p class="ff-leise">Feindliche Aufträge brauchen eigene Aufklärung: erst das System erkunden, dann den Planeten ausspähen.</p>`:'';
  const kl=z.klima&&['leer','unerkundet'].includes(z.art)?z.klima:null;
  const klima=kl?`<div class="ff-klima"><span class="ff-chips-titel">${A().badge('kolonisieren',{size:16})} Klima · ${h(kl.zone||'')} · Sonne ${Math.round((kl.solar_promille||0)/10)} %</span><div class="ff-klima-voelker">${(kl.herkunft||[]).map(v=>`<span class="${v.volk===view.volk?'eigen':''}${v.bewohnbar===false?' unbewohnbar':''}"><b>${h(name(v.volk))}</b>${v.wachstum_je_tag_promille!=null?`${(v.wachstum_je_tag_promille/10).toLocaleString('de-DE',{maximumFractionDigits:1})} % je Tag`:`Wachstum ${Math.round((v.wachstum_promille||0)/10)} %`}${v.terraforming_noetig?` · Terraforming ${v.terraforming_noetig}`:''}</span>`).join('')}</div></div>`:'';
  host.innerHTML=`<div class="ff-zielkopf"><b class="koord">${h(window.Planetenname?Planetenname.text(z.koord):'['+z.koord+']')}</b><span class="ff-zielart ff-art-${h(z.art)}">${h(ART_TEXT[z.art]||z.art)}</span>${z.spieler?`<span>${h(z.spieler)}${z.heimat?' · Heimatwelt':''}</span>`:''}${z.mond?'<span>☾ Mond</span>':''}</div>${z.truemmer?`<p>${A().badge('recyceln',{size:16})} Trümmerfeld: ${zahl(z.truemmer.erz)} Erz · ${zahl(z.truemmer.kristall)} Kristall</p>`:''}${klima}${bt?`<p>${A().badge('angriff',{size:16})} Mögliche Beute ≈ <b>${zahl(bt.summe)}</b> (Plünderquote ${Math.round(bt.quote*100)} %, Bunker ${bt.bunker}) · Flotte ${zahl(flottenWert(b.schiffe))} · Abwehr ${zahl(flottenWert(b.verteidigung))}</p>`:''}${kette}`;
 }
 // Ein eigener Angriff oder ein Verband fliegt dieses Ziel an: dort entsteht beim Kampf ein Trümmerfeld.
 function kampfAnflug(k){return (view?.flotten||[]).some(f=>f.zustand==='hinflug'&&f.ziel===k&&A().feindlich(f.mission))||(view?.verbaende||[]).some(v=>v.ziel===k);}
 function auftragsStatus(k){
  const a=A().info(k),z=zielInfo,p=startPlanet();
  const vorhanden=!a.braucht||a.braucht.some(u=>(p?.schiffe?.[u]||0)>0);
  let passt=true,grund='';
  if(z&&z.koord===ziel()){
   const art=z.art==='unerkundet'?'fremd':z.art;
   passt=a.ziel.includes(art)||(k==='recyceln'&&(!!z.truemmer||kampfAnflug(z.koord)))||(k==='mond_zerstoeren'&&!!z.mond)||(['angriff','spionage'].includes(k)&&!!z.mond&&formular()?.elements.zielart?.value==='mond')||z.art==='unbekannt';
   if(!passt)grund='passt nicht zu: '+(ART_TEXT[z.art]||z.art);
   if(passt&&a.aufklaerung&&art==='fremd'&&!(z.system_bekannt&&z.ausgespaeht))grund='erst aufklären (Schritt-Kette oben)';
  }
  if(a.keineHeimat&&z?.heimat&&z.koord===ziel()){passt=false;grund='Heimatwelten sind vor Übernahme geschützt';}
  if(a.ab&&view?.stufe<a.ab){passt=false;grund='ab Zivilisationsstufe '+a.ab;}
  if(passt&&!vorhanden)grund='fehlt am Startplaneten: '+a.braucht.map(name).join(' oder ');
  return {passt,vorhanden,grund};
 }
 function auftragsraster(){
  const host=el('ff-auftraege'),f=formular();if(!host||!f)return;
  const offen=(view?.verbaende||[]).filter(v=>v.beitretbar&&v.ziel===ziel());
  if(view&&'invasionen' in view&&![...f.elements.mission.options].some(o=>o.value==='verstaerken')){const o=document.createElement('option');o.value='verstaerken';o.textContent='Bodenhilfe (Verbündete)';f.elements.mission.append(o);}
  const keys=[...f.elements.mission.options].map(o=>o.value);
  if(offen.length&&keys.includes('angriff'))keys.splice(keys.indexOf('angriff')+1,0,'verband');
  const aktiv=verbandWahl&&auftrag()==='angriff'?'verband':auftrag();
  const karten=keys.map(k=>{const a=A().info(k),st=k==='verband'?{passt:true,vorhanden:true,grund:''}:auftragsStatus(k);
   return {k,a,st,html:`<button type="button" class="ff-auftrag auftrag-${h(a.gruppe)}${aktiv===k?' aktiv':''}${st.passt?'':' unpassend'}${st.grund?' hinweis':''}" data-ff-mission="${h(k)}" ${st.passt?'':'aria-disabled="true"'}>${A().badge(k,{size:30})}<b>${h(a.name)}</b><small>${h(st.grund||(k==='verband'?offen.length+' offen · '+offen.map(v=>v.spieler).join(', '):a.zweck))}</small>${st.passt&&!st.grund?'<i class="ff-haken" aria-label="passt">✓</i>':''}</button>`};});
  karten.sort((x,y)=>(y.st.passt-x.st.passt)||((!y.st.grund)-(!x.st.grund)));
  host.innerHTML=`<div class="ff-chips-titel">Auftrag ${zielInfo&&zielInfo.koord===ziel()?'· passende zuerst':''}</div>${karten.filter(x=>x.st.passt).map(x=>x.html).join('')}${karten.some(x=>!x.st.passt)?`<details class="ff-weitere-auftraege"><summary>${karten.filter(x=>!x.st.passt).length} hier nicht passende Aufträge</summary><div>${karten.filter(x=>!x.st.passt).map(x=>x.html).join('')}</div></details>`:''}`;
 }

 // ---------- Schritt 2: Schiffsraster ----------
 // Karl 24.10.2026: „nur die Schiffe … die ich wirklich gebaut habe … und spionagesonde ganz oben … besser sortiert und
 // übersichtlich“. Typen ohne Bestand verbirgt renderShipSelection (mobile035.js); hier: Aufklärung, Kampf, Transport,
 // Sonstige in fester Reihe, leere Gruppen ohne Kopf. Kampfschiffe in Katalogreihe (leicht → schwer).
 const GRUPPEN=[['ff-gruppe-sonde','Aufklärung',['spionagesonde']],['ff-gruppe-kampf','Kampfschiffe',null],
  ['ff-gruppe-fracht','Transport &amp; Fracht',['kleiner_transporter','grosser_transporter','schnellfrachter','schwerlastfrachter','handelsfrachter','truppentransporter']],
  ['ff-gruppe-zivil','Kolonie, Bergung &amp; Abbau',['kolonieschiff','grosses_kolonieschiff','recycler','bergungsschiff','bergbauschiff','terraformer','pathfinder']]];
 const gruppeVon=k=>{const g=GRUPPEN.findIndex(x=>x[2]?.includes(k));return g<0?1:g;};
 function schiffRaster(){
  const host=el('ship-inputs');if(!host)return;const gew=schiffe(),gewaehlt=GRUPPEN.map(()=>0),sichtbar=GRUPPEN.map(()=>0);
  host.querySelectorAll('.fleet-pick-row').forEach((r,i)=>{const k=r.dataset.fleetPick,g=gruppeVon(k),pos=GRUPPEN[g][2]?.indexOf(k)??-1;r.style.order=String((g+1)*1000+(pos>=0?pos:100+i));r.classList.toggle('gewaehlt',(gew[k]||0)>0);if(!r.hidden){sichtbar[g]++;gewaehlt[g]+=gew[k]||0;}});
  GRUPPEN.forEach(([cls,text],g)=>{let kopf=host.querySelector('.'+cls);if(!kopf){kopf=document.createElement('div');kopf.className='ff-gruppe '+cls;host.prepend(kopf);}kopf.style.order=String((g+1)*1000-1);if(kopf.hidden!==!sichtbar[g])kopf.hidden=!sichtbar[g];const html=`<span>${text}</span><small>${zahl(gewaehlt[g])} gewählt</small>`;if(kopf.innerHTML!==html)kopf.innerHTML=html;});
  let leer=host.querySelector('.ff-schiffe-leer');if(!leer){leer=document.createElement('p');leer.className='ff-schiffe-leer';leer.textContent='Am Startplaneten liegen keine Schiffe. Baue sie in der Werft oder wähle einen anderen Start.';host.prepend(leer);}
  const keine=!sichtbar.some(Boolean);if(leer.hidden!==!keine)leer.hidden=!keine;
 }

 // ---------- Schritt 2: Bedarfsrechner ----------
 async function kapazitaetFaktor(){
  const p=startPlanet();if(!p)return 1;const key=world.world_id+view.name+p.koord;if(ladeFaktor.schluessel===key&&ladeFaktor.wert)return ladeFaktor.wert;
  try{const q=await api('/api/tool',{typ:'flugzeit',start:p.koord,ziel:p.koord.replace(/:\d+$/,':'+(Number(p.koord.split(':')[2])%12+1)),mission:'transport',geschwindigkeit:1,haltedauer_stunden:0,schiffe:{kleiner_transporter:1},ladung:{}});const roh=katalog('kleiner_transporter').ladung||1;ladeFaktor.wert=(q.ladekapazitaet||roh)/roh;}catch{ladeFaktor.wert=1;}
  ladeFaktor.schluessel=key;return ladeFaktor.wert;
 }
 function frachtraum(map,faktor){return Object.entries(map).reduce((s,[k,n])=>s+(katalog(k).ladung||0)*n,0)*faktor;}
 function vorschlag(menge,typen,faktor){
  const p=startPlanet(),out=[];if(menge<=0)return out;
  for(const k of typen){const cap=(katalog(k).ladung||0)*faktor,da=(p?.schiffe?.[k]||0)-(schiffe()[k]||0);if(cap<=0)continue;const n=Math.ceil(menge/cap);out.push({k,n,da,genug:da>=n});}
  const vorhanden=out.filter(v=>v.da>0);return (vorhanden.length?vorhanden:out).sort((a,b)=>(b.genug-a.genug)||a.n-b.n).slice(0,3);
 }
 function vorschlagsKnopf(v,zusatz=''){return `<button type="button" class="ff-mini${v.genug?'':' knapp'}" data-ff-plus="${h(v.k)}" data-ff-anzahl="${v.n}">+${zahl(v.n)} ${h(name(v.k))}${v.genug?'':' (nur '+zahl(Math.max(0,v.da))+' da)'}${zusatz}</button>`;}
 async function bedarf(){
  const host=el('ff-bedarf');if(!host||schritt!==2)return;
  const m=auftrag(),a=A().info(verbandWahl?'verband':m),gew=schiffe(),f=await kapazitaetFaktor(),raum=frachtraum(gew,f),zeilen=[];
  if(['angriff','blockade','invasion'].includes(m)){
   const bt=beute(zielBericht(ziel()));
   if(bt){const fehlt=bt.summe-(raum-fracht());zeilen.push(`<div class="ff-bedarfzeile">${A().badge('transport',{size:18})}<span>Beute ≈ <b>${zahl(bt.summe)}</b> · Frachtraum ${zahl(raum)}${fehlt>0?` · <b class="ff-warn">fehlt ${zahl(fehlt)}</b>`:' · <b class="ff-gut">reicht</b>'}</span><div class="ff-bedarf-aktionen">${fehlt>0?vorschlag(fehlt,FRACHTER,f).map(v=>vorschlagsKnopf(v)).join(''):''}</div></div>`);}
   else zeilen.push(`<div class="ff-bedarfzeile">${A().badge('spionage',{size:18})}<span>Kein Spionagebericht für [${h(ziel())}] – Beute und Abwehr unbekannt.</span><div class="ff-bedarf-aktionen"><button type="button" class="ff-mini" data-ff-aufklaeren="spionage">Ausspähen</button></div></div>`);
   const key=JSON.stringify([ziel(),gew]),s=sim&&simSchluessel===key?sim:null;
   zeilen.push(`<div class="ff-bedarfzeile">${A().badge('angriff',{size:18})}<span>${s?`Siegchance <b class="${s.siegchance_prozent>=90?'ff-gut':s.siegchance_prozent>=50?'ff-warn':'ff-schlecht'}">${Math.round(s.siegchance_prozent)} %</b> · eigene Verluste ${zahl(s.eigene_verluste_wert)} · Gegner ${zahl(s.verluste_gegner_wert)} · Beute ⌀ ${zahl((s.beute_mittel_milli||[]).reduce((x,y)=>x+y,0)/1000)}`:'Kampf vorab durchrechnen (64 Läufe mit deinem letzten Bericht)'}</span><div class="ff-bedarf-aktionen"><button type="button" class="ff-mini" data-ff-simulieren ${Object.keys(gew).length&&zielBericht(ziel())?'':'disabled'}>${s?'Neu simulieren':'Simulieren'}</button></div></div>`);
   if(s){const tf=s.truemmer_mittel||0;
    // Mondchance wie im Kern (monde.rs chance_prozent): 1 % je truemmer_je_prozent Trümmer, höchstens chance_max_prozent,
    // nur wenn der Zielplanet noch keinen Mond hat.
    const mr=view?.monde?.regeln||{},je=mr.truemmer_je_prozent||100000,mx=mr.chance_max_prozent??20,hatMond=!!zielInfo?.mond&&zielInfo.koord===ziel();
    const mondChance=hatMond?0:Math.min(mx,Math.floor(tf/je));
    zeilen.push(`<div class="ff-bedarfzeile">${A().badge('mond_zerstoeren',{size:18,symbol:'expedition',title:'Mondentstehung'})}<span>${hatMond?'Am Ziel kreist schon ein Mond – keine neue Mondentstehung.':`Mondchance nach diesem Kampf ≈ <b>${mondChance} %</b> (1 % je ${zahl(je)} Trümmer, höchstens ${mx} %)${mondChance<mx&&tf>0?` · für ${mx} % braucht es ≈ ${zahl(je*mx)} Trümmer`:''}`}</span></div>`);
    if(tf>0){const rec=vorschlag(tf,['recycler','bergungsschiff'],f);zeilen.push(`<div class="ff-bedarfzeile">${A().badge('recyceln',{size:18})}<span>Erwartetes Trümmerfeld ≈ <b>${zahl(tf)}</b> → Recycler als eigene Flotte hinterher (Schritt 3: Ankunft koppeln)</span><div class="ff-bedarf-aktionen">${rec.map(v=>`<span class="ff-tag">${zahl(v.n)} ${h(name(v.k))}</span>`).join('')}<button type="button" class="ff-mini" data-ff-recycler-plan>Recycler-Flug vormerken</button></div></div>`);}}
  }
  if(m==='recyceln'){const t=zielInfo?.truemmer;const menge=t?(t.erz||0)+(t.kristall||0):0;const rec=vorschlag(menge-raum,['recycler','bergungsschiff'],f);zeilen.push(`<div class="ff-bedarfzeile">${A().badge('recyceln',{size:18})}<span>${t?`Trümmerfeld <b>${zahl(menge)}</b> · Laderaum ${zahl(raum)}${menge>raum?` · <b class="ff-warn">fehlt ${zahl(menge-raum)}</b>`:' · <b class="ff-gut">reicht</b>'}`:'Noch kein Trümmerfeld bekannt – Recycler dürfen trotzdem starten und sammeln, was bei Ankunft liegt (z. B. nach einem Angriff).'}</span><div class="ff-bedarf-aktionen">${menge>raum?rec.map(v=>vorschlagsKnopf(v)).join(''):''}</div></div>`);}
  if(['transport','stationieren','saven','kolonisieren'].includes(m)){const fr=fracht();const rest=fr-raum;zeilen.push(`<div class="ff-bedarfzeile">${A().badge(m,{size:18})}<span>Fracht <b>${zahl(fr)}</b> · Laderaum ${zahl(raum)}${rest>0?` · <b class="ff-warn">fehlt ${zahl(rest)}</b>`:''}</span><div class="ff-bedarf-aktionen">${rest>0?vorschlag(rest,FRACHTER,f).map(v=>vorschlagsKnopf(v)).join(''):''}</div></div>`);}
  if(a.braucht){const p=startPlanet(),hat=a.braucht.filter(u=>(p?.schiffe?.[u]||0)>0);const gewaehlt=a.braucht.some(u=>gew[u]>0);zeilen.push(`<div class="ff-bedarfzeile">${A().badge(m,{size:18})}<span>${h(a.name)} braucht ${a.braucht.map(name).join(' oder ')}${a.nur?' – und nur diese':''}.</span><div class="ff-bedarf-aktionen">${!gewaehlt&&hat.length?`<button type="button" class="ff-mini" data-ff-plus="${h(hat[0])}" data-ff-anzahl="${m==='spionage'?Math.min(8,p.schiffe[hat[0]]):1}">+${m==='spionage'?Math.min(8,p.schiffe[hat[0]]):1} ${h(name(hat[0]))}</button>`:''}${hat.length?'':'<b class="ff-schlecht">Keine am Startplaneten</b>'}</div></div>`);}
  const eigeneBelagerung=['stationieren','transport'].includes(m)&&(view?.invasionen||[]).some(i=>i.planet===ziel()&&i.rolle==='verteidiger'&&i.belagerung);
  if(['invasion','kampfkolonisieren','verstaerken'].includes(m)||eigeneBelagerung){const t=(window.Mobile035?.groundDraft?.()||[]).reduce((x,y)=>x+(Number(y)||0),0);zeilen.push(`<div class="ff-bedarfzeile">${A().badge('truppen',{size:18})}<span>${m==='kampfkolonisieren'?'Für die Landung nach der Bombardierung braucht die Flotte Kolonieschiff UND Bodentruppen; ohne Truppen keine Landung.':m==='verstaerken'?'Bodenhilfe schickt nur Bodentruppen; reine Truppenkonvois landen ohne Gefecht und verteidigen bis zum Ende der Belagerung mit.':eigeneBelagerung?'Dein Planet wird belagert: Eigene Bodentruppen landen trotz Blockade, die Schiffe kehren danach um.':'Bodentruppen zuladen.'} Geladen: <b class="${t?'ff-gut':'ff-warn'}">${zahl(t)}</b> · Truppentransporter tragen je 100 Plätze.</span><div class="ff-bedarf-aktionen"><button type="button" class="ff-mini" data-ff-truppen>Bodentruppen mitführen</button></div></div>`);}
  if(['halten','saven','abbau'].includes(m))zeilen.push(`<div class="ff-bedarfzeile">${A().badge(m,{size:18})}<span>Aufenthaltsdauer stellst du in Schritt 3 ein.</span></div>`);
  host.innerHTML=`<div class="ff-chips-titel">Bedarf für ${A().badge(verbandWahl?'verband':m,{size:16,text:true})}</div>${zeilen.join('')||'<p class="ff-leise">Für diesen Auftrag gibt es keinen besonderen Bedarf.</p>'}`;
 }
 async function simulieren(){
  const gew=schiffe(),key=JSON.stringify([ziel(),gew]);
  const s=await api('/api/tool',{typ:'kampfsimulator',ziel:ziel(),schiffe:gew,bergung_promille:1000});
  const laeufe=s.einzellaeufe||[];s.truemmer_mittel=laeufe.length?laeufe.reduce((n,l)=>n+(l.bergung_rohmaterial_milli||[0,0]).reduce((x,y)=>x+y,0),0)/laeufe.length/1000:0;
  sim=s;simSchluessel=key;bedarf();
 }

 // ---------- Schritt 3: Tempo je Stufe, Verband, Ankunft koppeln ----------
 function flug(){try{return fleetData();}catch{return null;}}
 async function tempoBerechnen(){
  const a=flug();if(!a||!Object.keys(a.schiffe).length||!zielGueltig())return;
  const key=JSON.stringify([a.start,a.ziel,a.mission,a.schiffe,a.zielart,a.startart]);if(key===tempoSchluessel||tempoLaeuft)return;tempoLaeuft=true;
  try{
   const reihe=await Promise.all(Array.from({length:10},(_,i)=>(i+1)/10).map(async g=>{try{const q=await api('/api/tool',{typ:'flugkoordination',flug:{...a,geschwindigkeit:g},ankunft:null,rueckgeschwindigkeit:1});return {g,hin:q.hinflug,ankunft:q.ankunft,treibstoff:q.treibstoff_hin+q.treibstoff_rueck};}catch{return {g,hin:null};}}));
   tempoTabelle={jetzt:view.sekunden,reihe};tempoSchluessel=key;
  }finally{tempoLaeuft=false;}
  tempo();verband();kopplungZeichnen();
 }
 function gewaehlterVerband(){return (view?.verbaende||[]).find(v=>String(v.fuehrung)===String(verbandWahl));}
 function verbandWirkung(v,ankunft){
  if(!v||ankunft==null)return null;const d=ankunft-v.ankunft_sekunden;
  if(d<=0)return {art:'gut',text:`wartet ${dauer(-d)} auf den Verband`};
  if(ankunft<=v.spaeteste_ankunft_sekunden)return {art:'warn',text:`verzögert den Verband um ${dauer(d)}`};
  return {art:'schlecht',text:'zu langsam – über 6 h Verzögerung'};
 }
 function tempo(){
  const host=el('ff-tempo'),f=formular();if(!host||!f)return;
  const g=Number(f.elements.geschwindigkeit.value)||1,v=auftrag()==='angriff'?gewaehlterVerband():null,ab=view.sekunden-(tempoTabelle?.jetzt||view.sekunden);
  const reihe=tempoTabelle&&tempoSchluessel?tempoTabelle.reihe:[];
  host.innerHTML=`<div class="ff-chips-titel">Hinflug · eigenes Tempo${v?' · Farbe = Wirkung auf den Verband':''}</div><div class="ff-tempo">${Array.from({length:10},(_,i)=>(10-i)/10).map(t=>{const r=reihe.find(x=>Math.abs(x.g-t)<1e-6),an=r?.ankunft!=null?r.ankunft+ab:null,w=v?verbandWirkung(v,an):null;return `<button type="button" class="ff-tempo-knopf${Math.abs(g-t)<1e-6?' aktiv':''}${w?' ff-'+w.art:''}" data-ff-tempo="${t}" title="${w?h(w.text):''}"><b>${Math.round(t*100)} %</b>${an!=null?`<small>${h(uhr(an).replace(/^Tag \d+ · /,''))}</small><small>${zahl(r.treibstoff)} D</small>`:'<small>…</small>'}</button>`;}).join('')}</div>`;
 }
 function verband(){
  const host=el('ff-verband');if(!host)return;
  if(auftrag()!=='angriff'){host.innerHTML='';host.hidden=true;return;}host.hidden=false;
  const offen=(view.verbaende||[]).filter(v=>v.ziel===ziel()&&v.beitretbar),g=Number(formular().elements.geschwindigkeit.value)||1;
  const r=tempoTabelle?.reihe?.find(x=>Math.abs(x.g-g)<1e-6),an=r?.ankunft!=null?r.ankunft+(view.sekunden-tempoTabelle.jetzt):null;
  const terminflug=formular().elements.termin_modus?.value==='arrival';
  const zeile=v=>{const w=verbandWirkung(v,an),voll=v.flotten>=VERBAND_MAX;return `<label class="ff-verbandzeile${String(verbandWahl)===String(v.fuehrung)?' aktiv':''}"><input type="radio" name="ff_verband" value="${v.fuehrung}" ${String(verbandWahl)===String(v.fuehrung)?'checked':''} ${voll?'disabled':''}>${A().badge('verband',{size:22})}<span><b>Verband ${h(v.spieler)}</b> · ${v.flotten}/${VERBAND_MAX} Flotten · ${h((v.teilnehmer||[]).join(', '))}<br><small>Ankunft ${h(uhr(v.ankunft_sekunden))} · Fenster bis ${h(uhr(v.spaeteste_ankunft_sekunden))} (noch ${dauer(v.spaeteste_ankunft_sekunden-v.ankunft_sekunden)} Verzögerung möglich)</small>${w?`<br><b class="ff-${w.art}">Deine Flotte ${h(w.text)}${w.art!=='schlecht'?' · gemeinsame Ankunft '+h(uhr(Math.max(an,v.ankunft_sekunden))):''}</b>`:''}</span></label>`;};
  host.innerHTML=`<div class="ff-chips-titel">${A().badge('verband',{size:16})} Zusammen angreifen</div>
   <label class="ff-verbandzeile${!verbandWahl?' aktiv':''}"><input type="radio" name="ff_verband" value="" ${!verbandWahl?'checked':''}>${A().badge('angriff',{size:22})}<span><b>Allein angreifen</b><br><small>Eigene Ankunft ${an!=null?h(uhr(an)):'…'}</small></span></label>
   ${offen.map(zeile).join('')}
   <label class="ff-verbandzeile${verbandWahl==='neu'?' aktiv':''}"><input type="radio" name="ff_verband" value="neu" ${verbandWahl==='neu'?'checked':''}>${A().badge('verband',{size:22})}<span><b>Neuen Verband eröffnen</b><br><small>Nach dem Start können Allianzmitglieder mit Flotten zum selben Ziel beitreten (höchstens ${VERBAND_MAX} Flotten, keine Flotte mehr als 6 h verzögert).</small></span></label>
   ${verbandWahl&&terminflug?'<p class="ff-schlecht">Für Verbandsbeitritt und -eröffnung muss die Flotte jetzt starten – „Abflug sofort“ wählen.</p>':''}
   <p class="ff-leise">Tipp aus der Praxis: stärkste Schiffe in eine eigene Flotte, Frachter in eine zweite – gleiche Ankunft, aber verschiedene Rückkehrzeiten erschweren das Abfangen. Nur mit Partnern fliegen, denen du vertraust: ein Rückruf löst deren Flotte aus dem Verband.</p>`;
 }
 function kopplungsAnker(){
  const out=[],z=ziel(),sys=z.split(':').slice(0,2).join(':');
  for(const f of view.flotten||[])if(f.zustand==='hinflug'&&f.ziel&&f.ziel.split(':').slice(0,2).join(':')===sys)out.push({id:'f'+f.flotte,an:f.ankunft_sekunden,text:`Flotte ${f.flotte} · ${name(f.mission)} → [${f.ziel}]`,mission:f.mission});
  for(const v of view.verbaende||[])if(v.ziel.split(':').slice(0,2).join(':')===sys)out.push({id:'v'+v.fuehrung,an:v.ankunft_sekunden,text:`Verband ${v.spieler} → [${v.ziel}] · ${v.flotten} Flotten`,mission:'verband'});
  return out;
 }
 function kopplungZeichnen(){
  const host=el('ff-kopplung');if(!host)return;const anker=kopplungsAnker();
  if(!anker.length||!['recyceln','angriff','transport','spionage','halten'].includes(auftrag())){host.innerHTML='';host.hidden=true;return;}host.hidden=false;
  const wahl=anker.find(x=>x.id===kopplung.an),g=Number(formular().elements.geschwindigkeit.value)||1,r=tempoTabelle?.reihe?.find(x=>Math.abs(x.g-g)<1e-6);
  let hinweis='';if(wahl&&r?.hin!=null){const soll=wahl.an+Number(kopplung.versatz||0),abflug=soll-r.hin;hinweis=abflug<view.sekunden?`<b class="ff-schlecht">Mit ${Math.round(g*100)} % zu langsam: Abflug wäre vor ${dauer(view.sekunden-abflug)} gewesen. Schneller fliegen.</b>`:`<b class="ff-gut">Ankunft ${h(uhr(soll))} · Abflug automatisch in ${dauer(abflug-view.sekunden)}</b>`;}
  host.innerHTML=`<div class="ff-chips-titel">${A().badge(auftrag(),{size:16})} Ankunft an andere Flotte koppeln</div><div class="ff-kopplung"><select data-ff-kopplung-anker><option value="">Nicht koppeln · sofort starten</option>${anker.map(x=>`<option value="${x.id}" ${x.id===kopplung.an?'selected':''}>${h(x.text)} · ${h(uhr(x.an))}</option>`).join('')}</select><label>+ <input type="number" min="-3600" max="3600" step="1" value="${Number(kopplung.versatz)||0}" data-ff-kopplung-versatz> s</label></div><p class="ff-leise">Recycler einige Sekunden nach dem Einschlag, Halteflotte kurz davor. Nutzt den Terminflug: der Kern startet die Flotte selbst zur richtigen Zeit.</p>${hinweis}`;
 }
 function kopplungAnwenden(){
  const f=formular(),wahl=kopplungsAnker().find(x=>x.id===kopplung.an);if(!f?.elements.termin_modus)return;
  if(!wahl){if(f.elements.termin_modus.value==='arrival'&&f.dataset.ffGekoppelt){f.elements.termin_modus.value='now';delete f.dataset.ffGekoppelt;f.elements.termin_modus.dispatchEvent(new Event('change',{bubbles:true}));}return;}
  const t=wahl.an+Number(kopplung.versatz||0);f.elements.termin_modus.value='arrival';f.dataset.ffGekoppelt='1';
  if(f.elements.ankunft_tag)f.elements.ankunft_tag.value=Math.floor(t/86400)+1;
  if(f.elements.ankunft_uhrzeit)f.elements.ankunft_uhrzeit.value=[Math.floor(t/3600)%24,Math.floor(t/60)%60,t%60].map(n=>String(Math.floor(n)).padStart(2,'0')).join(':');
  f.elements.termin_modus.dispatchEvent(new Event('change',{bubbles:true}));
 }

 // ---------- Leiste unten ----------
 function leiste(){
  const host=el('ff-leiste');if(!host)return;const m=verbandWahl&&auftrag()==='angriff'?'verband':auftrag(),fehler=schrittFehler(schritt),anzahl=Object.values(schiffe()).reduce((a,b)=>a+b,0);
  host.innerHTML=`<div class="ff-leiste-info">${m?A().badge(m,{size:28}):''}<span><b>${h(A().info(m).name||'Auftrag wählen')}</b><small>${zielGueltig()?'['+h(ziel())+']':'kein Ziel'} · ${zahl(anzahl)} ${anzahl===1?'Schiff':'Schiffe'}${verbandWahl==='neu'?' · eröffnet Verband':gewaehlterVerband()?' · tritt Verband bei':''}</small></span></div><div class="ff-leiste-knoepfe">${schritt>1?'<button type="button" class="secondary" data-ff-zurueck>Zurück</button>':''}${schritt<3?`<button type="button" data-ff-weiter ${fehler?'aria-disabled="true"':''} title="${h(fehler)}">Weiter</button>`:`<button type="button" class="ff-start" data-ff-starten ${schrittFehler(3)||startLaeuft?'aria-disabled="true"':''}>${startLaeuft?'Wird gesendet …':['uebergeben','schiffhandel'].includes(m)?'Angebot senden':'Flotte starten'}</button>`}</div>${(()=>{const g=schritt===3?schrittFehler(3):aufklaerungFehlt();return g&&!startLaeuft?`<p class="ff-leiste-grund">${h(g)}</p>`:'';})()}`;
 }

 // ---------- Zusammenarbeit & Zieltabelle (Flottenseite) ----------
 function zusammenarbeit(){
  const content=el('flotten-content');if(!content||!view)return;let host=el('ff-zusammenarbeit');
  if(!host){host=document.createElement('details');host.id='ff-zusammenarbeit';host.className='ff-panel';const planner=el('fleet-planner');(planner||content.firstChild)?.before(host);}
  const offen=host.open,eigeneFrei=(view.flotten||[]).filter(f=>f.mission==='angriff'&&f.zustand==='hinflug'&&f.verband==null);
  const vb=(view.verbaende||[]).map(v=>{const passende=eigeneFrei.filter(f=>f.ziel===v.ziel);const meine=(view.flotten||[]).filter(f=>f.verband===v.fuehrung);return `<div class="ff-kartenzeile">${A().badge('verband',{size:26})}<div><b>Verband ${h(v.spieler)} → <span class="koord">[${h(v.ziel)}]</span></b><small>${v.flotten}/${VERBAND_MAX} Flotten · ${h((v.teilnehmer||[]).join(', '))} · ${zahl(v.schiffe)} Schiffe · Ankunft <span data-arrival="${v.ankunft_sekunden}"></span> (${h(uhr(v.ankunft_sekunden))})${meine.length?' · du bist dabei':''}</small></div><div class="ff-knoepfe">${v.beitretbar&&passende.length?passende.map(f=>`<button type="button" class="ff-mini" data-ff-beitreten="${f.flotte}" data-ff-fuehrung="${v.fuehrung}">Flotte ${f.flotte} beitreten</button>`).join(''):''}${v.beitretbar?`<button type="button" class="ff-mini" data-ff-ziel-plan="${h(v.ziel)}" data-ff-verband-plan="${v.fuehrung}">Neue Flotte dazu</button>`:'<small>nur Allianz kann beitreten</small>'}</div></div>`;}).join('');
  const frei=eigeneFrei.filter(f=>!(view.verbaende||[]).some(v=>v.fuehrung===f.flotte)).map(f=>`<div class="ff-kartenzeile">${A().badge('angriff',{size:26})}<div><b>Flotte ${f.flotte} → <span class="koord">[${h(f.ziel)}]</span></b><small>Ankunft <span data-arrival="${f.ankunft_sekunden}"></span> · noch kein Verband</small></div><div class="ff-knoepfe"><button type="button" class="ff-mini" data-open-group="${f.flotte}">Als Verband öffnen</button></div></div>`).join('');
  const halten=(view.verbuendete_anfluege||[]).map(x=>`<div class="ff-kartenzeile">${A().badge(x.mission||'halten',{size:26})}<div><b>${h(x.spieler||x.von||'Verbündeter')} → <span class="koord">[${h(x.ziel||'')}]</span></b><small>${h(name(x.mission||'halten'))}${x.ankunft_sekunden?' · Ankunft '+h(uhr(x.ankunft_sekunden)):''}</small></div></div>`).join('');
  // Verbündete Flotten, die schon über eigenen Planeten stehen und mitverteidigen (Kern: verbuendete_im_orbit).
  const imOrbit=(view.verbuendete_im_orbit||[]).map(x=>`<div class="ff-kartenzeile">${A().badge(x.gestrandet?'halten':(x.mission||'halten'),{size:26})}<div><b>${h(x.von)} über <span class="koord">[${h(x.ziel)}]</span></b><small>${x.gestrandet?'gestrandet':h(name(x.mission||'halten'))} · verteidigt mit · ${zahl(x.anzahl)} ${x.anzahl===1?'Schiff':'Schiffe'}${x.truppen?' · '+zahl(x.truppen)+' Bodentruppen':''}${x.bis_sekunden?' · bis '+h(uhr(x.bis_sekunden)):''}</small></div></div>`).join('');
  host.innerHTML=`<summary>${A().badge('verband',{size:20})} Zusammenarbeit · ${(view.verbaende||[]).length} ${(view.verbaende||[]).length===1?'Verband':'Verbände'}${eigeneFrei.length?' · '+eigeneFrei.length+' freie Angriffsflotte'+(eigeneFrei.length===1?'':'n'):''}</summary><div class="ff-panel-inhalt">${vb||'<p class="ff-leise">Kein offener Verband. Eine eigene Angriffsflotte im Hinflug kann als Verband geöffnet werden; Allianzmitglieder treten dann mit Flotten zum selben Ziel bei.</p>'}${frei}${halten?`<h4>Verbündete Anflüge</h4>${halten}`:''}${imOrbit?`<h4>Verbündete im Orbit</h4>${imOrbit}`:''}<details class="ff-legende-box"><summary>Zeichen der Aufträge</summary>${A().legende()}</details></div>`;
  host.open=offen;
 }
 function zieltabelle(){
  const content=el('flotten-content');if(!content||!view)return;let host=el('ff-zieltabelle');
  if(!host){host=document.createElement('details');host.id='ff-zieltabelle';host.className='ff-panel';const planner=el('fleet-planner');(planner||content.firstChild)?.before(host);}
  const offen=host.open,zeilen=(view.berichte||[]).map(b=>({b,beute:beute(b)?.summe||0,flotte:flottenWert(b.schiffe),abwehr:flottenWert(b.verteidigung),alter:b.alter_stunden||0}));
  const {feld,ab}=sortierung;zeilen.sort((x,y)=>(ab?-1:1)*((x[feld]??0)-(y[feld]??0)));
  const kopf=[['ziel','Ziel'],['alter','Alter'],['spieler','Spieler'],['beute','Beute ≈'],['flotte','Flotte'],['abwehr','Abwehr']].map(([k,t])=>`<th>${['ziel','spieler'].includes(k)?h(t):`<button type="button" data-ff-sort="${k}" class="${feld===k?'aktiv':''}">${h(t)}${feld===k?(ab?' ▼':' ▲'):''}</button>`}</th>`).join('');
  const summe=zeilen.reduce((s,z)=>s+z.beute,0);
  host.innerHTML=`<summary>${A().badge('spionage',{size:20})} Zielauswahl · ${zeilen.length} Spionageberichte${summe?' · Beute ≈ '+zahl(summe):''}</summary><div class="ff-panel-inhalt">${zeilen.length?`<table class="ff-ziele"><thead><tr>${kopf}<th>Aktionen</th></tr></thead><tbody>${zeilen.map(z=>`<tr><td class="koord" data-label="Ziel">[${h(z.b.ziel)}]</td><td data-label="Alter">${z.alter} h</td><td data-label="Spieler">${h(z.b.besitzer||'?')}</td><td data-label="Beute ≈"><b>${zahl(z.beute)}</b></td><td data-label="Flotte" class="${z.flotte?'ff-warn':''}">${zahl(z.flotte)}</td><td data-label="Abwehr">${zahl(z.abwehr)}</td><td class="ff-aktionen"><button type="button" class="ff-icon auftrag-${A().info('angriff').gruppe}" data-ff-ziel-plan="${h(z.b.ziel)}" data-ff-auftrag-plan="angriff" title="Angriff planen">${A().svg('angriff',18)}</button><button type="button" class="ff-icon auftrag-${A().info('spionage').gruppe}" data-ff-ziel-plan="${h(z.b.ziel)}" data-ff-auftrag-plan="spionage" title="Erneut ausspähen">${A().svg('spionage',18)}</button><button type="button" class="ff-icon" data-ff-galaxie="${h(z.b.ziel)}" title="Im System zeigen">${A().svg('system_erkunden',18)}</button></td></tr>`).join('')}</tbody><tfoot><tr><td colspan="7">Wert = Erz + Kristall + Deuterium der Einheiten · Beute nach Bunker und Plünderquote ${Math.round(eigeneQuote()*100)} %</td></tr></tfoot></table>`:'<p class="ff-leise">Noch keine Spionageberichte. Ziele über die Galaxie erkunden und ausspähen – sie erscheinen hier zum Vergleichen.</p>'}</div>`;
  host.open=offen;
 }

 // ---------- Zeichnen ----------
 let zeichnenGeplant=false;
 function alles(){
  if(typeof view==='undefined'||!view)return;bauen();if(!gebaut)return;
  schnellziele();zielkarte();auftragsraster();leiste();tempo();verband();kopplungZeichnen();if(schritt===2){schiffRaster();bedarf();}
  zusammenarbeit();zieltabelle();
  if(schritt===3)tempoBerechnen();
 }
 function bald(){if(zeichnenGeplant)return;zeichnenGeplant=true;setTimeout(()=>{zeichnenGeplant=false;try{alles();}catch(err){console.error('Flottenführung',err);}},30);}
 async function zielGeaendert(){await zielErkennen();bald();}

 // ---------- Planen von außen (Berichte, Galaxie, Zusammenarbeit) ----------
 // Jeder Aufruf von pickFleet (Berichte, Galaxie, 3D-Karte, Android-Langdruck) führt in den passenden Schritt:
 // fehlt Aufklärung für einen feindlichen Auftrag → ① mit Aufklärungskette, sonst → ② Flotte.
 function nachPick(k,m){
  verbandWahl='';kopplung.an='';tempoSchluessel='';zielSchluessel='';sim=null;
  bauen();zielGeaendert().then(()=>{if(!gebaut)return;const st=m&&m!=='verband'?auftragsStatus(m):{passt:true};zeigeSchritt(!st.passt||zielInfo&&A().info(m).aufklaerung&&!(zielInfo.system_bekannt&&zielInfo.ausgespaeht)?1:2);});
 }
 function pickFleetUmhuellen(){
  const original=window.pickFleet;if(typeof original!=='function'||original.ffUmhuellt)return;
  // Jeder neue Flug beginnt mit Planet → Planet: Ziel-/Startart des vorigen Mondflugs (Weltenkrümmer, „Flug zum Mond“) blieben
  // sonst stehen – „Mission saven ist gegen Monde nicht möglich“ bzw. Start vom Mond (Spielprobe 20.10.2026). Mondziele setzen
  // zielart/startart nach pickFleet selbst (galaxie-tabelle gtMondziel, ausHangar).
  const huelle=function(k,m){const f=formular()?.elements;if(f?.zielart)f.zielart.value='planet';if(f?.startart)f.startart.value='planet';
   const r=original.apply(this,arguments);try{nachPick(k,m);}catch(err){console.error('Flottenführung',err);}return r;};
  huelle.ffUmhuellt=true;window.pickFleet=huelle;
 }
 function planen(k,m,vb,anker){
  // Einsatzblatt (einsatz.js) übernimmt Verbandswahl und Ankunftskopplung direkt – über pickFleet gingen sie verloren.
  if(window.Einsatz?.oeffnen){Einsatz.oeffnen({ziel:k,mission:m==='verband'?'angriff':m,verband:vb!=null?String(vb):'',kopplung:anker?{anker,versatz:5}:null});return;}
  pickFleetUmhuellen();if(typeof window.pickFleet==='function')window.pickFleet(k,m==='verband'?'angriff':m);
  if(vb!=null){verbandWahl=String(vb);bald();}
  el('fleet-planner')?.scrollIntoView?.({block:'start'});
 }

 // ---------- Ereignisse ----------
 // Nach erfolgreichem Start (mobile035.js rememberFlight): bestätigen, Schiffe/Truppen leeren, zurück auf Schritt 1 –
 // vorher blieb der Assistent auf Schritt 3 mit aktivem Startknopf stehen (Doppelversand, M5).
 // nur die Leiste neu: alles() könnte über Eingabeereignisse eine neue Flugprüfung auslösen (Schleife)
 document.addEventListener('stern:flugpruefung',()=>{if(gebaut)try{leiste();}catch(err){console.error('Flottenführung',err);}});
 document.addEventListener('stern:angebot-gesendet',()=>{clearTimeout(startUhr);startLaeuft=false;zeigeSchritt(1);});
 document.addEventListener('stern:flotte-gestartet',e=>{
  const a=e.detail||{},n=Object.values(a.schiffe||{}).reduce((x,y)=>x+(Number(y)||0),0);
  clearTimeout(startUhr);startLaeuft=false;
  el('ship-inputs')?.querySelectorAll('input[data-map]').forEach(i=>{i.value=0;});
  el('ground-load')?.querySelectorAll('[data-ground-load]').forEach(i=>{i.value=0;});
  el('ship-inputs')?.querySelector('input[data-map]')?.dispatchEvent(new Event('input',{bubbles:true}));
  zeigeSchritt(1);
  show(`Flotte gestartet: ${A().info(a.mission).name||'Auftrag'} nach [${a.ziel||'?'}]${n?' · '+zahl(n)+' Schiffe':''}. Sie steht jetzt unter „Unterwegs“.`);
  // Die Rückmeldung steht oben im Flottenkommando; ohne Sprung bleibt sie am Handy ~1.600 px über dem Daumen unsichtbar
  setTimeout(()=>[...document.querySelectorAll('.fleet-inline-feedback')].find(x=>x.offsetParent)?.scrollIntoView({block:'center',behavior:'smooth'}),60);
 });
 document.addEventListener('click',guardSafe(async e=>{
  const b=e.target.closest('[data-ff-schritt],[data-ff-weiter],[data-ff-zurueck],[data-ff-starten],[data-ff-mission],[data-ff-ziel],[data-ff-tempo],[data-ff-plus],[data-ff-simulieren],[data-ff-aufklaeren],[data-ff-beitreten],[data-ff-ziel-plan],[data-ff-galaxie],[data-ff-sort],[data-ff-recycler-plan],[data-ff-truppen]');if(!b)return;
  if(b.dataset.ffSchritt){const n=Number(b.dataset.ffSchritt),fehl=schrittFehler(n-1);if(n>schritt&&fehl){show(fehl,true);return;}zeigeSchritt(n);return;}
  if(b.hasAttribute('data-ff-weiter')){const fehl=schrittFehler(schritt);if(fehl){show(fehl,true);return;}zeigeSchritt(schritt+1);return;}
  if(b.hasAttribute('data-ff-zurueck')){zeigeSchritt(schritt-1);return;}
  // Doppelversand verhindern (Galaxie-Sitzung 19.10.2026): bis zur Bestätigung oder höchstens 8 s gesperrt
  if(b.hasAttribute('data-ff-starten')){if(startLaeuft)return;const fehl=schrittFehler(3);if(fehl){show(fehl,true);return;}
   startLaeuft=true;leiste();clearTimeout(startUhr);startUhr=setTimeout(()=>{startLaeuft=false;bald();},8000);formular().requestSubmit();return;}
  if(b.dataset.ffMission){const k=b.dataset.ffMission;if(b.getAttribute('aria-disabled')==='true'){show(b.querySelector('small')?.textContent||'Auftrag passt nicht zum Ziel',true);return;}
   const f=formular();if(k==='verband'){const v=(view.verbaende||[]).find(v=>v.beitretbar&&v.ziel===ziel());f.elements.mission.value='angriff';verbandWahl=v?String(v.fuehrung):'';}else{f.elements.mission.value=k;if(k!=='angriff')verbandWahl='';}
   f.elements.mission.dispatchEvent(new Event('change',{bubbles:true}));sim=null;bald();return;}
  if(b.dataset.ffZiel){const f=formular();f.elements.ziel.value=b.dataset.ffZiel;f.elements.ziel.dispatchEvent(new Event('input',{bubbles:true}));const m=b.dataset.ffAuftrag;
   if(m){if(m==='verband'){f.elements.mission.value='angriff';const v=(view.verbaende||[]).find(v=>v.ziel===b.dataset.ffZiel&&v.beitretbar);verbandWahl=v?String(v.fuehrung):'';}else{f.elements.mission.value=m;verbandWahl='';}f.elements.mission.dispatchEvent(new Event('change',{bubbles:true}));}
   await zielGeaendert();return;}
  if(b.dataset.ffTempo){const f=formular();f.elements.geschwindigkeit.value=b.dataset.ffTempo;f.elements.geschwindigkeit.dispatchEvent(new Event('change',{bubbles:true}));if(kopplung.an)kopplungAnwenden();bald();return;}
  if(b.dataset.ffPlus){const k=b.dataset.ffPlus,n=Number(b.dataset.ffAnzahl)||1,p=startPlanet(),aktuell=schiffe()[k]||0;setzeSchiff(k,Math.min(p?.schiffe?.[k]||0,aktuell+n));if(window.Mobile035?.renderShipSelection&&p)Mobile035.renderShipSelection(p);bald();return;}
  if(b.hasAttribute('data-ff-simulieren')){b.disabled=true;b.textContent='Rechnet …';await simulieren();return;}
  if(b.dataset.ffAufklaeren){planen(ziel(),b.dataset.ffAufklaeren);return;}
  if(b.dataset.ffBeitreten){await command([{typ:'verband_beitreten',flotte:Number(b.dataset.ffBeitreten),fuehrung:Number(b.dataset.ffFuehrung)}]);return;}
  if(b.dataset.ffZielPlan){planen(b.dataset.ffZielPlan,b.dataset.ffAuftragPlan||'angriff',b.dataset.ffVerbandPlan);return;}
  if(b.dataset.ffGalaxie){const [s,sy]=b.dataset.ffGalaxie.split(':').map(Number);navigateGame('karte');if(window.GalaxieTabelle?.gehe)GalaxieTabelle.gehe(s,sy);return;}
  if(b.dataset.ffSort){const k=b.dataset.ffSort;sortierung=sortierung.feld===k?{feld:k,ab:!sortierung.ab}:{feld:k,ab:true};zieltabelle();return;}
  if(b.hasAttribute('data-ff-truppen')){const d=el('ground-load');if(d){d.open=true;d.scrollIntoView({block:'center'});}return;}
  if(b.hasAttribute('data-ff-recycler-plan')){const ankerZiel=ziel();show('Erst den Angriff starten – danach öffnet sich der Recycler-Flug mit gekoppelter Ankunft.');formular().dataset.ffRecyclerNach=ankerZiel;return;}
 }));
 function guardSafe(fn){return typeof guard==='function'?guard(fn):async e=>{try{await fn(e);}catch(err){console.error(err);}};}
 document.addEventListener('change',e=>{
  const t=e.target;
  if(t.name==='ff_verband'){verbandWahl=t.value;bald();return;}
  if(t.matches?.('[data-ff-kopplung-anker]')){kopplung.an=t.value;kopplungAnwenden();bald();return;}
  if(t.matches?.('[data-ff-kopplung-versatz]')){kopplung.versatz=Number(t.value)||0;kopplungAnwenden();bald();return;}
  if(t.closest?.('#fleet')){if(['start','mission','zielart','startart'].includes(t.name)){tempoSchluessel='';if(t.name!=='mission')zielSchluessel='';}bald();if(t.name==='ziel'||t.name==='zielart')zielGeaendert();}
 });
 document.addEventListener('input',e=>{const t=e.target;if(t.closest?.('#fleet')){if(t.name==='ziel'){zielSchluessel='';clearTimeout(t._ff);t._ff=setTimeout(zielGeaendert,250);}if(t.matches?.('#ship-inputs input'))tempoSchluessel='';bald();}});
 // Verband: nach dem Start beitreten bzw. eröffnen – Senden selbst übernimmt dieselbe Logik wie app.js.
 // Erst prüfen, dann sperren: guard() sperrt den Absendeknopf, bevor die Funktion läuft. Als Erfassungs-Listener für
 // ALLE Formulare sperrte er so jeden Absendeknopf – Markt, Briefe, Verträge, Allianz, Fertigung blieben stumm, weil ihr
 // eigener Handler den Knopf schon „beschäftigt“ sah (Karl 19.10.2026: „über die UI ist fast keine Funktion verfügbar“).
 document.addEventListener('submit',e=>{
  if(e.target.id!=='fleet')return;
  const f=e.target,wahl=auftrag()==='angriff'?verbandWahl:'',recyclerNach=f.dataset.ffRecyclerNach;
  if(!wahl&&!recyclerNach)return;
  e.preventDefault();e.stopImmediatePropagation();
  void guardSafe(async()=>verbandSenden(e,f,wahl,recyclerNach))(e);
 },true);
 async function verbandSenden(e,f,wahl,recyclerNach){
  if(wahl&&f.elements.termin_modus?.value==='arrival')throw Error('Verband: Flotte muss sofort starten („Abflug sofort“).');
  const flight=fleetData(),truppen=window.Mobile035?.groundDraft?.()||[0,0,0],plan=Mobile035.coordinatedFlight(truppen.some(n=>n>0)?{typ:'boden_senden',flug:flight,truppen}:flight);
  const r=await command([plan]);if(!r.ergebnisse.every(x=>x.ok))return;
  window.Mobile035?.rememberFlight?.({...flight,truppen,personen:window.Flottenladung?.personen(),rueckgeschwindigkeit:plan.rueckgeschwindigkeit});
  const id=Number((r.ergebnisse.map(x=>x.text).join(' ').match(/Flotte (\d+) gestartet/)||[])[1]);
  if(wahl&&id){if(wahl==='neu')await command([{typ:'verband_oeffnen',flotte:id}]);else await command([{typ:'verband_beitreten',flotte:id,fuehrung:Number(wahl)}]);}
  delete f.dataset.ffRecyclerNach;verbandWahl='';
  if(recyclerNach&&id){if(window.Einsatz?.oeffnen){if(typeof refresh==='function')await refresh();planen(recyclerNach,'recyceln',null,'f'+id);}else{planen(recyclerNach,'recyceln');kopplung={an:'f'+id,versatz:5};setTimeout(()=>{kopplungAnwenden();zeigeSchritt(2);},400);}}
 }
 document.addEventListener('game-navigation',()=>{pickFleetUmhuellen();bald();});
 document.addEventListener('DOMContentLoaded',pickFleetUmhuellen);
 document.addEventListener('DOMContentLoaded',()=>{const o=new MutationObserver(()=>{if(!gebaut&&el('ship-inputs')?.children.length)bald();});o.observe(document.body,{childList:true,subtree:true});});
 // Nach jeder Datenaktualisierung neu zeichnen (refresh() setzt view neu).
 let letzteRevision=null;setInterval(()=>{if(typeof view==='undefined'||!view)return;const r=view.revision+':'+view.sekunden;if(r!==letzteRevision){letzteRevision=r;bald();}},1000);
 // Aus dem Flottenhangar (progression.js): Start (Planet/Mond) und Schiffe stehen fest; mit eigenem Ziel direkt zu
 // ③ Zeit, sonst ① Ziel & Auftrag – Schritt ② ist dann schon ausgefüllt.
 function ausHangar({start,startart='planet',schiffe={},ziel='',zielart='planet',mission='',direktFlotte=false}){
  if(typeof navigateGame==='function')navigateGame('flotten');const planner=el('fleet-planner');if(planner)planner.open=true;
  bauen();const f=formular();if(!f)return;
  if(f.elements.startart)f.elements.startart.value=startart;
  f.elements.start.value=start;if(typeof renderFleetStock==='function')renderFleetStock();
  f.elements.ziel.value=ziel;if(f.elements.zielart)f.elements.zielart.value=zielart;
  if(mission)f.elements.mission.value=mission;
  if(el('return-fuel')){el('return-fuel').checked=true;if(typeof syncReturnFuel==='function')syncReturnFuel();}
  el('ship-inputs')?.querySelectorAll('input[data-map]').forEach(i=>{i.value=0;});
  el('cargo-inputs')?.querySelectorAll('input').forEach(i=>{i.value=0;});
  document.querySelectorAll('[data-ground-load]').forEach(i=>{i.value=0;});
  window.Flottenladung?.vorlage();
  for(const [k,n] of Object.entries(schiffe))setzeSchiff(k,n);
  f.dispatchEvent(new Event('change',{bubbles:true}));
  verbandWahl='';kopplung.an='';tempoSchluessel='';zielSchluessel='';sim=null;
  zielGeaendert().then(()=>{zeigeSchritt(direktFlotte?2:ziel&&mission&&!schrittFehler(2)?3:1);el('fleet-planner')?.scrollIntoView?.({block:'start'});});
 }
 window.Flottenfuehrung={planen,zeigeSchritt,beute,flottenWert,neu:bald,ausHangar};
})();
