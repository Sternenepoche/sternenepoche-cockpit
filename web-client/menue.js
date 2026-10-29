'use strict';
// Menü-Blatt hinter dem Themenknopf unten links (Handy < 768 px). Karl 24.10.2026: „Was auch optisch verbessert werden kann
// mit transparenten Hintergrundbildern und so ist, wenn man unten links auf das Menü klickt – das geht cooler und wirkt
// etwas KI-sloppy.“ Vorher: Überschrift „Was möchtest du tun?“, vier Farbkarten mit Fülltext (Zielnamen als Satz) und eine
// Knopfliste.
// Jetzt: vier Themenkarten mit Spielkunst des eigenen Volks als durchscheinendem Hintergrund (Motiv vollständig, rechts,
// Verlauf davor – DESIGNREGELN §5), echte Live-Angaben aus der Sicht (Bauten mit Restzeit, Forschung, Flotten und feindliche
// Anflüge, Kolonien und bekannte Systeme, ungelesene Post/Berichte und offene Angebote), alle vier Ziele je Thema direkt
// antippbar (zwei Tipps bis überall: Menü → Ziel) und die weiteren Bereiche als Kachelraster mit Symbolen und Zählern.
// Bewegung nur transform/opacity, reduzierte Bewegung wird beachtet; kein Dauerglühen, keine Emojis.
// Verhalten bleibt in rahmen.js: Themenknopf/themaWaehlen (data-thema), data-tab, Zurück schließt zuerst das Blatt, aria.
// CSP: Motive kommen per data-css (stable-dom.js setzt sie per CSSOM), keine style-Attribute im HTML.
window.Menue=(()=>{
 const V=()=>typeof view!=='undefined'?view:null;
 const h=s=>typeof esc==='function'?esc(s):String(s??'');
 const zahl=n=>typeof fmt==='function'?fmt(Math.round(Number(n)||0)):String(Math.round(Number(n)||0));
 const name=k=>typeof named==='function'?named(k):String(k);
 // Motiv je Thema: Kategorie, Schlüssel, je Volk eigene Fassung (art.js). Ausgewählt nach Inhalt: Industriestadt (Aufbau),
 // Flottenverband (Flotte), Orbitalstadt mit Planetenhorizont (Galaxie), Ratssaal (Kommunikation).
 const MOTIVE={aufbau:['progression','industrie',true],flotte:['progression','imperium',true],galaxie:['progression','orbit',true],funk:['backgrounds','diplomacy',false]};
 const SYMBOL={
  auftraege:'M9 4h6v3H9ZM7 5H5v16h14V5h-2M8 11h8M8 15h8M8 19h5',
  kolonie:'M9 11a3 3 0 1 0 0-6 3 3 0 0 0 0 6ZM3 20a6 6 0 0 1 12 0M18 8v6M15 11h6',
  freischaltungen:'M5 3h5v5H5ZM14 3h5v5h-5ZM9.5 16h5v5h-5ZM7.5 8v4h9V8M12 12v4',
  warsim:'M4 4l9 9M13 13l3 3M15 20l5-5M20 4l-9 9M11 13l-3 3M9 20l-5-5',
  'core-planner':'M4 6h16v14H4ZM4 10h16M8 3v4M16 3v4M8 14h3M8 17h6',
  berichte:'M6 3h9l4 4v14H6ZM14 3v5h5M9 12h7M9 16h7',
  grossprojekt:'M12 8a4 4 0 1 0 0 8 4 4 0 0 0 0-8ZM12 2v3M12 19v3M2 12h3M19 12h3M5 5l2 2M17 17l2 2M5 19l2-2M17 7l2-2',
  pinwaende:'M5 4h14v11l-5 5H5ZM14 20v-5h5M8 9h8M8 12h5',
  profil:'M4 7h9M17 7h3M4 12h3M11 12h9M4 17h11M19 17h1M15 5v4M9 10v4M17 15v4',
  regeln:'M12 3a9 9 0 1 0 0 18 9 9 0 0 0 0-18ZM9.6 9.3a2.5 2.5 0 1 1 3.5 2.3c-.7.3-1.1.9-1.1 1.6v.6M12 17h.01',
  cockpit:'M8 2h8a1 1 0 0 1 1 1v18a1 1 0 0 1-1 1H8a1 1 0 0 1-1-1V3a1 1 0 0 1 1-1ZM11 18h2'
 };
 // Lange Wörter trennen, statt sie in schmalen Kacheln abzuschneiden (§5: umbrechen statt „…“).
 const TRENNUNG={'Bauaufträge':'Bau&shy;aufträge','Technologiebaum':'Techno&shy;logie&shy;baum','Kampfsimulator':'Kampf&shy;simu&shy;lator','Ausbauplaner':'Ausbau&shy;planer','Großprojekt':'Groß&shy;projekt','Einstellungen':'Einstel&shy;lungen','Handy-Cockpit':'Handy-&shy;Cockpit'};
 const svg=d=>`<svg class="menue-symbol" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true" focusable="false"><path d="${d}"/></svg>`;
 const uhr=t=>`<b class="blatt-uhr" data-arrival-echt="${Number(t)}"></b>`;
 function motivUrl(id){
  const m=MOTIVE[id],v=V();if(!m||typeof art!=='function')return '';
  const u=art(m[0],m[1],m[2]?(v?.volk||''):'');if(!u)return '';
  try{return new URL(u,document.baseURI).href;}catch(_){return u;}
 }
 // ---------------------------------------------------------------- Live-Angaben (nur Werte aus der Sicht)
 function aufbau(v){
  const z=[],bau=(v.planeten||[]).map(p=>p.bauschleife?.[0]).filter(Boolean);
  const laeuft=bau.filter(b=>Number.isFinite(b.fertig_sekunden)&&!b.pausiert&&!b.manuell_pausiert);
  if(bau.length){const t=laeuft.length?Math.min(...laeuft.map(b=>b.fertig_sekunden)):null;
   z.push({t:`${bau.length===1?'1 Bau':zahl(bau.length)+' Bauten'} · ${t!=null?'nächster fertig in '+uhr(t):'pausiert oder wartet'}`});}
  else z.push({t:'Kein Gebäude im Bau',art:'hinweis'});
  const f=v.forschung?.aktiv;
  if(f)z.push({t:`${h(name(f.forschung))} ${zahl(f.stufe||((v.forschung.stufen?.[f.forschung]||0)+1))}${f.pausiert||f.manuell_pausiert?' · pausiert':Number.isFinite(f.fertig_sekunden)?' · fertig in '+uhr(f.fertig_sekunden):''}`});
  else z.push({t:'Keine Forschung aktiv',art:'hinweis'});
  return z;
 }
 function flotte(v){
  const z=[],ang=(v.angriffe||[]).filter(a=>Number.isFinite(a.ankunft_sekunden)),fl=v.flotten||[],slots=v.flottenplaetze||{};
  if(ang.length)z.push({t:`${ang.length===1?'1 feindlicher Anflug':zahl(ang.length)+' feindliche Anflüge'} · erster in ${uhr(Math.min(...ang.map(a=>a.ankunft_sekunden)))}`,art:'gefahr'});
  z.push({t:fl.length?`${zahl(fl.length)} ${fl.length===1?'Flotte':'Flotten'} unterwegs · Plätze ${zahl(slots.belegt??fl.length)} von ${zahl(slots.gesamt??0)}`:`Keine Flotte unterwegs · ${zahl(Math.max(0,(slots.gesamt??0)-(slots.belegt??0)))} Plätze frei`});
  if(z.length<2){const n=(v.planeten||[]).reduce((s,p)=>s+(p.fertigung||[]).filter(j=>j.schleife==='werft').reduce((a,j)=>a+(Number(j.rest)||0),0),0);
   z.push(n?{t:`Werft: ${zahl(n)} ${n===1?'Einheit':'Einheiten'} in Bau`}:{t:'Werft frei',art:'hinweis'});}
  return z;
 }
 function galaxie(v){
  const z=[],k=v.kolonien||{},hin=(v.flotten||[]).filter(f=>f.mission==='kolonisieren'&&f.zustand!=='rueckflug').length;
  const frei=Math.max(0,(Number(k.erlaubt)||0)-(Number(k.anzahl)||0)-hin);
  // Gleiche Zählung wie der Kolonisieren-Dialog („Kolonien: N von M erlaubt“).
  z.push({t:`Kolonien ${zahl(k.anzahl||0)} von ${zahl(k.erlaubt||0)}${frei?` · ${zahl(frei)} frei`:''}${hin?` · ${zahl(hin)} Kolonieflug unterwegs`:''}`,art:frei&&!hin?'hinweis':''});
  const systeme=new Set((v.atlas||[]).map(a=>String(a.koord).split(':').slice(0,2).join(':'))).size,tf=(v.truemmer||[]).length;
  z.push({t:`${zahl(systeme)} ${systeme===1?'System':'Systeme'} bekannt${tf?` · ${zahl(tf)} ${tf===1?'Trümmerfeld':'Trümmerfelder'}`:''}`});
  return z;
 }
 function postLage(v){
  let briefe=0,berichte=0;
  try{if(typeof communicationData==='function')briefe=(communicationData().briefkasten||[]).filter(n=>n.kanal==='privat'&&!n.gesendet&&!n.gelesen).length;}catch(_){}
  try{if(typeof reportRows==='function'){const gelesen=new Set(v.postfach_gelesen||[]);berichte=reportRows().filter(x=>!gelesen.has(x.id)).length;}}catch(_){}
  return {briefe,berichte};
 }
 function funk(v){
  const z=[],{briefe,berichte}=postLage(v);
  z.push(briefe||berichte?{t:[briefe?`${zahl(briefe)} ${briefe===1?'ungelesener Brief':'ungelesene Briefe'}`:'',berichte?`${zahl(berichte)} ${berichte===1?'neuer Bericht':'neue Berichte'}`:''].filter(Boolean).join(' · '),art:'neu'}:{t:'Keine ungelesene Post'});
  const angebote=(v.vertraege||[]).filter(x=>x.status==='angebot an dich').length+(v.kommunikation?.anfragen||[]).filter(r=>r.status==='offen'&&r.an===v.name).length;
  z.push(angebote?{t:`${zahl(angebote)} ${angebote===1?'Angebot wartet':'Angebote warten'} auf Antwort`,art:'hinweis'}:{t:v.allianz?.name?`Allianz ${h(v.allianz.tag?'['+v.allianz.tag+'] ':'')}${h(v.allianz.name)}`:'Keine Allianz'});
  return z;
 }
 const LIVE={aufbau,flotte,galaxie,funk};
 function live(id){const v=V();if(!v||!LIVE[id])return [];try{return LIVE[id](v).slice(0,2);}catch(_){return [];}}

 // ---------------------------------------------------------------- Blatt
 // ctx aus rahmen.js: {themen, thema, aktiv (Bereich), gruppe(id) → Menügruppe, weitere [[tab,name]], cockpit, symbol(name)}
 function html(ctx){
  const v=V(),p=typeof currentPlanet==='function'?currentPlanet():null;
  const ort=p?`<span class="blatt-ort"><small>Aktiver Planet</small><b>${h(window.Planetenname?Planetenname.name(p):p.koord)} <span class="koord">[${h(p.koord)}]</span></b></span>`:'';
  const karte=t=>{
   const an=t.id===ctx.thema,url=motivUrl(t.id),aktivPunkt=t.punkte.find(([b])=>b===ctx.aktiv)?.[0]||t.punkte.find(([b])=>ctx.gruppe(b)&&ctx.gruppe(b)===ctx.gruppe(ctx.aktiv))?.[0];
   const zeilen=live(t.id);
   return `<section class="blatt-karte" data-k="${h(t.id)}"${an?' data-an="true"':''}${url?` data-css="--motiv:url(&quot;${h(url)}&quot;)"`:''}>
<button type="button" class="blatt-thema" data-thema="${h(t.id)}" aria-pressed="${an}" aria-describedby="blatt-live-${h(t.id)}"><span class="blatt-thema-symbol">${ctx.symbol(t.symbol)}</span><span class="blatt-thema-text"><strong>${h(t.titel)}</strong></span>${an?'<small class="blatt-an-marke">In der Leiste</small>':'<span class="blatt-pfeil" aria-hidden="true">›</span>'}</button>
<div class="blatt-live" id="blatt-live-${h(t.id)}">${zeilen.map(z=>`<span class="blatt-live-zeile${z.art?' '+h(z.art):''}">${z.t}</span>`).join('')}</div>
<div class="blatt-ziele" role="group" aria-label="${h(t.titel)}">${t.punkte.map(([b,n,s])=>`<button type="button" data-tab="${h(b)}" data-blatt-thema="${h(t.id)}" aria-current="${b===aktivPunkt?'page':'false'}"${document.getElementById(b)?'':' disabled'}>${ctx.symbol(s)}<span>${h(n)}</span>${b==='briefkasten'&&ctx.post?`<b class="blatt-zahl">${zahl(ctx.post)}</b>`:''}</button>`).join('')}</div></section>`;
  };
  const {berichte}=v?postLage(v):{berichte:0};
  const kachel=(attr,sym,n,aktiv,zaehler)=>`<button type="button" ${attr}${aktiv?' aria-current="page"':' aria-current="false"'}>${svg(SYMBOL[sym]||SYMBOL.regeln)}<span>${TRENNUNG[n]||h(n)}</span>${zaehler?`<b class="blatt-zahl">${zahl(zaehler)}</b>`:''}</button>`;
  const weitere=[ctx.cockpit?`<button type="button" class="blatt-gold" data-blatt-cockpit>${svg(SYMBOL.cockpit)}<span>${TRENNUNG['Handy-Cockpit']}</span></button>`:'',
   ...ctx.weitere.map(([b,n])=>kachel(`data-tab="${h(b)}"`,b,n,b===ctx.aktiv,b==='berichte'?berichte:0))].join('');
  return `<div class="blatt-innen"><div class="blatt-griff" aria-hidden="true"></div>
<div class="blatt-kopf"><h2 id="themenblatt-titel">Menü</h2>${ort}<button type="button" class="blatt-zu" data-themen-blatt-zu aria-label="Menü schließen">${svg('M6 6l12 12M18 6 6 18')}</button></div>
<div class="blatt-themen">${ctx.themen.map(karte).join('')}</div>
<h3 class="blatt-weitere-titel">Weitere Bereiche</h3><div class="blatt-weitere">${weitere}</div></div>`;
 }
 return {html,live,motivUrl};
})();
