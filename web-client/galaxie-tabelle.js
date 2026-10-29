'use strict';
// Galaxieansicht nach docs/DESIGNREGELN.md §4: ein System = eine Tabelle mit einer Zeile je Platz (Desktop).
// Handy (< 768 px, Befund H9 vom 24.10.2026): dieselben Plätze als zweizeilige Einträge ohne seitliches Wischen – Planet
// mit Platznummer, Name/Spieler, darunter Zone, Klima, Mond und Trümmer als Marken; je Zeile EINE Hauptaktion
// (Ausspähen, Kolonisieren, Systemscan) und „⋯“ für die übrigen. Vorher war die Tabelle ~890 px breit bei 337 px Anzeige,
// der Kopf wanderte beim Wischen mit und die Gürtelzeile war abgeschnitten. Der Kopf (Systemkonsole) steht jetzt
// außerhalb jedes Scrollbereichs; auf dem Desktop scrollt nur die Tabelle selbst, falls sie einmal breiter wird.
// Daten aus dem lesenden Werkzeug „galaxie“ (Sondenberichte, nie mehr als der Server freigibt).
// Aktionen kommen aus EINER Liste (aktionen()) für Hauptaktion, „⋯“, Infokarte und Desktop-Symbolspalte. Spionage und
// Systemscan bleiben Ein-Tipp-Aufträge; jeder Flottenauftrag öffnet die Einsatzplanung (window.Einsatz.oeffnen), ohne sie
// den bisherigen Weg (Einsatzplanung für Abbau/Bergung, Flottenführung/pickFleet).
// Infokarte (Tipp aufs Planetensymbol oder die Zeile): nicht-modales Blatt mit Zurück-Haken. Zone, Klima, Größe und
// Besitzer aus denselben Quellen wie Tabelle und Kolonisieren-Dialog – vorher stand „Klimazone unbekannt“ über
// „Sonnenabstand: Frostzone“, und die Herkunftstabelle lief rechts aus dem Bild.
const GalaxieTabelle=(()=>{
 let letzteScans=null;
 let sektor=0,system=0,daten=null,laedt=false,fehler='',ladeNr=0,lochTimer=0;
 let offen='';      // Handy: Platz mit ausgeklappten weiteren Aktionen („⋯“)
 let karteKoord=''; // Platz der offenen Infokarte
 let karteFokus=null;
 let zeigenNach='';  // Platz, dessen Infokarte nach dem Laden aufgehen soll (zeigen())
 const filter=new Set(),spionBusy=new Set();
 let sondenAnzahl=1;try{sondenAnzahl=Math.max(1,Math.min(1000,Number(localStorage.getItem('sternenepoche.spionage.sonden'))||1));}catch{}
 const HANDY=typeof matchMedia==='function'?matchMedia('(max-width: 767px)'):{matches:false};
 const handy=()=>!!HANDY.matches;
 const A=()=>window.Auftraege;
 const motiv=(k,n=20)=>A()&&k?A().svg(k,n):'';
 const missionKlasse=k=>A()&&k?A().klasse(k):'';
 const auftragName=k=>A()?A().info(k).name:named(k);
 const zahl=n=>fmt(Math.round(Number(n)||0));
 const ZONEN={glut:'Glutzone',leben:'Lebenszone',frost:'Frostzone'};
 const ZONE_KURZ={glut:'Glut',leben:'Leben',frost:'Frost'};
 const VOELKER=['aurelianer','krath','veyari','syntheten'];
 const welt=()=>rules?.catalog?.welt||{sektoren:1,systeme_je_sektor:1,plaetze_je_system:15};
 const klemmen=(n,max)=>Math.min(max,Math.max(1,Math.floor(Number(n)||1)));
 function startwerte(){if(sektor)return;const k=String(currentPlanet()?.koord||'1:1:1').split(':').map(Number);sektor=k[0]||1;system=k[1]||1;}

 // Zone eines Platzes – EINE Quelle für Tabelle, Liste und Infokarte. Der Kern leitet sie aus der Bahn ab
 // (habitabilitaet.rs bahn/zone) und liefert sie für jeden Platz in habitabilitaet.zone; ältere Welten in zone.
 function zoneAusPlatz(pos,n){const b=Math.min(11,Math.floor(Math.max(0,pos-1)*11/Math.max(1,n-1)));return b<=2?'glut':b<=7?'leben':'frost';}
 function zoneVon(p){return p?.habitabilitaet?.zone||p?.zone||zoneAusPlatz(Number(p?.position)||Number(String(p?.koord||'').split(':')[2])||1,welt().plaetze_je_system||12);}
 // Klima in % des Optimums und Wachstum je Tag – dieselbe Rechnung wie der Kolonisieren-Dialog (kolonisierung.js eignungText).
 function klima(p,volk=view?.volk){
  const h=p?.habitabilitaet,e=(h?.herkunft||[]).find(x=>x.volk===volk);if(!h||!e)return null;
  const opt=Number(h.optimum_promille)||1000;
  return {volk,bewohnbar:!!e.bewohnbar,prozent:Math.max(0,Math.round((Number(e.wachstum_promille)||0)/opt*100)),tag:Number.isFinite(e.wachstum_je_tag_promille)?e.wachstum_je_tag_promille/10:null,
   noetig:Number(e.terraforming_noetig)||0,eigen:Number(e.terraforming)||0,verlust:Number(e.klimaverlust_promille_je_stunde)||0};
 }
 // Sonnenlicht wie im Kolonisieren-Dialog: wirksamer Wert (mit Dyson), sonst der Lagewert; Platz 6 = 100 %.
 function sonne(p){const h=p?.habitabilitaet;if(!h)return null;const w=Number.isFinite(h.solar_wirksam_promille)?h.solar_wirksam_promille:h.solar_promille;return Number.isFinite(w)?w/10:null;}
 const stufe=v=>v>=80?'gut':v>=50?'mittel':v>0?'schwach':'null';

 async function laden(){
  startwerte();const nr=++ladeNr;laedt=true;fehler='';render();
  try{const r=await api('/api/tool',{typ:'galaxie',sektor,von:system,bis:system});if(nr!==ladeNr)return;daten=(r.systeme||[]).find(s=>Number(s.system)===system)||{system,plaetze:[],belegt:[]};daten.sektor=sektor;}
  catch(e){if(nr===ladeNr){fehler=e.message;daten=null;}}
  finally{if(nr===ladeNr){laedt=false;render();if(zeigenNach){const k=zeigenNach;zeigenNach='';if(plaetze().some(x=>x.koord===k))karteAuf(k);}}}
 }
 function plaetze(){
  const n=welt().plaetze_je_system||15,s=daten||{plaetze:[],belegt:[]};
  return Array.from({length:n},(_,i)=>{const pos=i+1,koord=`${sektor}:${system}:${pos}`;
   const basis=(s.plaetze||[]).find(p=>Number(p.position)===pos)||{},belegt=(s.belegt||[]).find(p=>Number(p.position)===pos)||{};
   return {koord,position:pos,...basis,...belegt,nebel:basis.nebel??s.nebel??null};});
 }
 // Rangliste der Lobby: [Rang, Reich, Punkte, Stufe]
 function rang(name){const r=(world?.rangliste||[]).find(x=>x[1]===name);return r?{rang:r[0],punkte:Number(r[2])||0}:null;}
 function status(p){
  const s=[];if(!p.spieler)return s;
  if(p.besiegt)s.push(['b','besiegt']);
  if(p.urlaub)s.push(['u','Urlaubsmodus']);
  if(p.gesperrt)s.push(['g','gesperrt']);
  if(p.inaktiv_tage>=28)s.push(['I','inaktiv seit 28 Tagen']);else if(p.inaktiv_tage>=7)s.push(['i','inaktiv seit 7 Tagen']);
  const eigen=rang(view.name),fremd=rang(p.spieler);
  if(eigen&&fremd&&p.spieler!==view.name){if(fremd.punkte*5<eigen.punkte)s.push(['n','deutlich schwächer (Neuling)']);else if(fremd.punkte>eigen.punkte*5)s.push(['s','deutlich stärker']);}
  return s;
 }
 // Mondbild über die Mondseite (Kartenmotiv moons.<typ>.map oder farbige Kugel).
 function mondBild(m,titel){return window.Mondseite?Mondseite.bild(m||{typ:'krater_grau',name:titel},'gt-mond'):`<span class="gt-mond-symbol" title="${esc(titel)}">☾</span>`;}
 const mondTyp=m=>window.Mondseite?.typName?Mondseite.typName(m.typ):m.typ;
 // Tiefenphalanx: eigener Mond mit wirksamer Phalanx im selben Sektor und in Reichweite.
 function phalanxMond(k){const [s,n]=String(k).split(':').map(Number);return (view?.monde?.eigene||[]).find(m=>{const [ms,mn]=m.koord.split(':').map(Number);return m.tiefenphalanx?.stufe_wirksam>0&&ms===s&&Math.abs(mn-n)<=m.tiefenphalanx.reichweite_systeme;});}
 function beziehung(p){return typeof GalaxyModel!=='undefined'?GalaxyModel.relation(p):(view.planeten.some(x=>x.koord===p.koord)?'eigen':'neutral');}
 const beziehungText=p=>(typeof relationFor==='function'?relationFor(p.spieler,p.beziehung)?.bezeichnung:p.beziehung?.bezeichnung)||'Neutral';

 // Alles, was Zeile, Infokarte und Aktionen über einen Platz wissen müssen – einmal abgeleitet.
 function lage(p0){
  let p=p0;
  // Sternkarte (System erfasst) zeigt Mond und Mondbesitzer (MONDKONZEPT §6.1), der Planet selbst braucht eine Sonde:
  // der Besitzer erscheint dann aus der Mondsicht, der Planet bleibt „nicht erkundet“.
  if(!p.spieler&&p.mond?.besitzer&&!p.mond.eigen)p={...p,spieler:p.mond.besitzer,ausMond:true};
  const eigen=view.planeten.find(x=>x.koord===p.koord)||null,bekannt=!!(p.bekannt||eigen),besiedlung=!!(bekannt||p.besiedlung_bekannt);
  // Frei: Kern meldet „frei“ (Planetensonde) oder Systemscan/Allianzsicht „unbewohnt“. Kolonisieren verlangt die eigene
  // Planetensonde (flotte.rs: planetenwissen.bekannt), darum erst ausspähen.
  const frei=!eigen&&!p.spieler&&(p.status==='frei'||besiedlung&&p.bewohnt===false);
  const fremd=!!p.spieler&&!eigen;
  // Invasion v2 (Karl 24.10.2026: „Status ‚Verwüstet‘ in der Sonnensystemansicht“): Kern planetenwissen.verwuestet für Kenner
  // des Systems; Besatzerseite = eigene Invasion dort (view.invasionen, Rolle angreifer/angriffshelfer).
  const inv=(view.invasionen||[]).find(i=>i.planet===p.koord)||null,verwuestet=!!(p.verwuestet||inv?.verwuestet);
  return {p,eigen,bekannt,besiedlung,frei,kolonisierbar:frei&&!!p.bekannt,fremd,rel:eigen?'eigen':beziehung(p),zone:zoneVon(p),
   tf:(view.truemmer||[]).find(t=>t.koord===p.koord)||null,st:status(p),rang:p.spieler?rang(p.spieler):null,aktiv:!!eigen&&eigen.koord===currentPlanet()?.koord,
   inv,verwuestet,besatzer:verwuestet&&!!inv&&['angreifer','angriffshelfer'].includes(inv.rolle),invasionEigen:!!eigen&&!!(eigen.invasion?.wellen||inv?.rolle==='verteidiger'),
   orbit:Array.isArray(p.orbitflotten)?p.orbitflotten:[]};
 }
 // „Heimatwelt“ nur, wenn der Kern es sagt – allein aus Systemscan oder Allianzsicht ist es unbekannt (vorher stand dort
 // „Kolonie“, auch bei der Heimatwelt des Gegenspielers).
 const artFremd=p=>p.heimat===true?'Heimatwelt':p.heimat===false?'Kolonie':'bewohnt';
 function titelText(L){const {p,eigen,frei,fremd,bekannt}=L;return eigen?Planetenname.name(eigen):fremd?(p.name||(p.heimat===true?'Heimatwelt':p.heimat===false?'Kolonie':'Planet')+' von '+p.spieler):frei?'Freier Platz':bekannt?'Unbewohnt':'Unerforschter Platz';}
 function zustandText(L){const {p,eigen,frei,kolonisierbar,fremd,bekannt,besiedlung}=L;
  if(L.verwuestet)return eigen?'Deine Kolonie · verwüstet – Kontrolle verloren':`Verwüstet · ${L.besatzer?'deine Besatzung hält den Boden':'Besatzung im Orbit'}`;
  return eigen?(L.invasionEigen?'Deine Kolonie · Invasion läuft':eigen.heimat?'Deine Heimatwelt':'Deine Kolonie'):fremd?(p.ausMond?'Besitzer laut Mond · Planet nicht erkundet':'Bewohnt · '+beziehungText(p)+(p.besiegt?' · besiegt':'')):kolonisierbar?'Frei · kolonisierbar':frei?'Frei · noch nicht erkundet':bekannt?'Erkundet':besiedlung?'Besiedlung bekannt':'Noch nicht erkundet';}

 // ---------------------------------------------------------------- Aktionen (eine Liste für alle Ansichten)
 // Karl 24.10.: „Spionage nur wenn der Planet bewohnt ist, sonst soll da Erkunden stehen! denke bitte besser mit!“ – je Lage
 // genau ein Wort und eine Hauptaktion, in Tabelle, Liste, „⋯“ und Infokarte gleich. Sinnlose Aktionen fehlen (Nachricht an
 // sich selbst, Angriff auf Verbündete); was hier und jetzt nicht geht, bleibt sichtbar, gedämpft und nennt den Grund.
 // Raketen: Kern raketen.rs – derselbe Sektor, Abstand ≤ Reichweite je Silostufe × Silostufe des Startplaneten.
 function raketenLage(koord){
  const [s,n]=String(koord).split(':').map(Number),silos=(view.planeten||[]).filter(p=>(p.gebaeude?.raketensilo||0)>0);
  if(!silos.length)return {grund:'Kein Raketensilo – unter Verteidigung bauen'};
  // Nur Arten, die im Silo liegen, bestimmen die Reichweite (Funktionsprüfung 25.10.2026: gesperrte Hyperrakete zählte mit).
  const typ=p=>(p.raketensilo?.typen||[]).filter(t=>t.regel?.planet&&Number(t.bestand)>0);
  const bestueckt=silos.filter(p=>typ(p).length);if(!bestueckt.length)return {grund:'Keine Angriffsraketen im Silo – unter Verteidigung bauen'};
  const start=bestueckt.find(p=>{const [ps,pn]=p.koord.split(':').map(Number),r=Math.max(0,...typ(p).map(t=>Number(t.regel.reichweite_je_silo)||0));return ps===s&&Math.abs(pn-n)<=r*(p.gebaeude.raketensilo||0);});
  return start?{start:start.koord}:{grund:'Außer Reichweite der Raketen in deinen Silos'};
 }
 // Freie Kolonieplätze wie im Kolonisieren-Dialog (view.kolonien), laufende Kolonieflüge abgezogen.
 function kolonieLage(){const k=view.kolonien||{},hin=(view.flotten||[]).filter(f=>['kolonisieren','kampfkolonisieren'].includes(f.mission)&&f.zustand!=='rueckflug').length;
  // Neue Welten: Flüge reservieren keinen Platz, wer zuerst gründet, bekommt ihn (Kern kolonieplatz_erst_bei_gründung, Handbuch
  // Kap. 16) – wie Einsatzblatt und Kolonieseite (Funktionsprüfung 25.10.2026: die Tabelle sperrte mit „kein Kolonieplatz frei“).
  const neu=(Number(k.frei_basis)||0)>0;
  return {frei:(Number(k.erlaubt)||0)-(Number(k.anzahl)||0)-(neu?0:hin),text:`${fmt(k.anzahl||0)} von ${fmt(k.erlaubt||0)} Kolonien${hin?`, ${fmt(hin)} Kolonieflug unterwegs`:''}`};}
 function aktionen(L){
  const {p,eigen,kolonisierbar,frei,fremd,rel,tf}=L,k=esc(p.koord),out=[];
  const ein=(m,text,o={})=>({id:o.id||m,m,text:text||auftragName(m),attr:`data-gt-einsatz="${m}" data-gt-einsatz-ziel="${k}"${o.mond?' data-gt-einsatz-art="mond"':''}`,...o});
  const n=sondenAnzahl;
  // Bewohnt: Ausspähen mit der gemerkten Sondenzahl. Unbewohnt: Erkunden mit EINER Sonde (Kern: Mission spionage; mehr
  // Sonden zeigen auf einem leeren Platz nichts zusätzlich).
  const ausspaehen={id:'spion',m:'spionage',text:'Ausspähen',attr:`data-gt-spion="${k}"`,titel:`${n} ${n===1?'Sonde':'Sonden'} sofort nach [${p.koord}]: Flotte, Abwehr, Rohstoffe`};
  const erkunden=(text='Erkunden')=>({id:'erkunden',m:'system_erkunden',text,attr:`data-gt-spion="${k}" data-gt-sonden="1"`,titel:`1 Sonde sofort nach [${p.koord}]: Größe, Klima und Rohstoffe des Platzes`});
  if(eigen){
   if(L.invasionEigen)out.push({id:'bodenkampf',z:'bodenkampf',text:'Bodenkampf',attr:`data-bk-oeffnen="${k}"`,haupt:true,titel:'Invasion: Linien verteilen, Wellen und Berichte'});
   if(!L.aktiv){
    out.push({id:'waehlen',z:'waehlen',text:'Auswählen',attr:`data-planet-pick="${k}"`,haupt:!L.invasionEigen,titel:'Als aktiven Planeten wählen'});
    const von=currentPlanet()?.koord||'';
    out.push(ein('transport',null,{titel:`Güter von ${Planetenname.text(von)} hierher`}),ein('stationieren',null,{titel:`Schiffe von ${Planetenname.text(von)} hierher verlegen`}));
   }
   if(p.mond)out.push({id:'mond',z:'mond',text:'Mond öffnen',attr:`data-mond-oeffnen="${k}"`,titel:'Mondseite öffnen'});
   out.push({id:'umbenennen',z:'umbenennen',text:'Umbenennen',attr:`data-planet-umbenennen="${k}"`,titel:'Planet umbenennen'});
  }else if(fremd&&L.besatzer){
   // Verwüstet (Kern invasion2.rs verwuestet_landen): ein Kolonieschiff mit Kampfkolonisieren oder Kolonisieren der
   // Besatzerseite landet und übernimmt, sobald ein Kolonieplatz frei ist.
   const kl=kolonieLage();
   out.push(ein('kampfkolonisieren',null,{haupt:true,titel:`Verwüstet – Kolonieschiff landet und übernimmt (${kl.text})`}),ein('kolonisieren',null,{titel:`Verwüstet – gleiche Regeln wie Kampfkolonisieren (${kl.text})`}),
    {id:'bodenkampf',z:'bodenkampf',text:'Bodenkampf',attr:`data-bk-oeffnen="${k}"`,titel:'Linien, Wellen und Berichte dieser Invasion'},ausspaehen);
   out.push({id:'nachricht',z:'nachricht',text:'Nachricht',attr:`data-mail-to="${esc(p.spieler)}"`,titel:`Nachricht an ${p.spieler}`});
  }else if(fremd){
   if(rel==='verbuendet')out.push(ein('transport',null,{haupt:true}),ein('halten',null,{titel:'Im Orbit mitverteidigen'}),ausspaehen);
   else{
    out.push({...ausspaehen,haupt:true},ein('angriff','Angriff planen'));
    // Wie das 3D-Feld: Invasion auf bekannte fremde Kolonien (nicht Heimatwelt, nicht besiegt) – Funktionsprüfung 25.10.2026.
    if(p.bekannt&&!p.heimat&&!p.besiegt)out.push(ein('kampfkolonisieren','Invasion planen'));
    const rk=raketenLage(p.koord);
    out.push({id:'rakete',m:'rakete',text:auftragName('rakete'),attr:rk.start?`data-gt-rakete="${k}" data-gt-rakete-start="${esc(rk.start)}"`:'',grund:rk.grund||'',titel:rk.grund||`Raketenangriff aus dem Silo auf [${rk.start}]`});
    const ph=phalanxMond(p.koord);if(ph)out.push({id:'phalanx',z:'phalanx',text:'Phalanx-Scan',attr:`data-gt-phalanx="${k}" data-phalanx-mond="${esc(ph.koord)}"`,titel:'Tiefenphalanx: Flottenbewegungen an diesem Platz scannen'});
   }
   out.push({id:'nachricht',z:'nachricht',text:'Nachricht',attr:`data-mail-to="${esc(p.spieler)}"`,titel:`Nachricht an ${p.spieler}`},{id:'kontakt',z:'kontakt',text:'Kontakt / Vertrag',attr:`data-gt-kontakt="${esc(p.spieler)}"`,titel:`Diplomatische Anfrage an ${p.spieler}`});
  }else if(kolonisierbar){
   const kl=kolonieLage(),alter=p.alter_stunden!=null?`erkundet vor ${fmt(p.alter_stunden)} Spielstunden`:'erkundet';
   if(kl.frei>0)out.push(ein('kolonisieren',null,{haupt:true,titel:`Kolonieplatz frei (${kl.text})`}),erkunden('Erneut erkunden'));
   else out.push({...erkunden(),haupt:true,titel:`${alter} – erneut erkunden`},{...ein('kolonisieren'),attr:'',grund:`Kein Kolonieplatz frei (${kl.text})`});
  }else if(frei)out.push({...erkunden(),haupt:true},{...ein('kolonisieren'),attr:'',grund:'Erst erkunden – Kolonisieren braucht eine eigene Planetensonde'});
  else if(!daten?.bekannt)out.push({id:'scan',m:'system_erkunden',text:'Systemscan',attr:`data-system-scan="${sektor}:${system}:1"`,haupt:true,titel:'Eine Sonde kartiert Besiedlung, Monde und Trümmer des ganzen Systems'});
  else out.push({...erkunden(),haupt:true});
  if(p.mond&&!(p.mond.eigen||eigen))out.push({id:'mondflug',z:'mond',text:'Mond: Aufträge',attr:`data-gt-mondziel="${k}"`,titel:'Aufträge gegen den Mond planen (Einsatzplanung, Zielart Mond)'});
  if(tf)out.push(ein('recyceln',null,{titel:`Trümmerfeld recyceln: ${tfMenge(tf)}`}));
  // Gestrandete Flotten (Karl 24.10.2026: „für alle Spieler, die das System erkundet haben, offen sichtbar … von jedem als Ziel
  // auswählbar“): fremde angreifen, eigene weiterfliegen oder versorgen (Einsatzplanung).
  for(const o of L.orbit){
   const n=`${fmt(o.schiffe)} ${Number(o.schiffe)===1?'Schiff':'Schiffe'}`;
   if(o.eigen)out.push({id:'orbit-weiter-'+o.flotte,z:'gestrandet',eigenOrbit:true,text:'Weiterfliegen',attr:`data-gt-orbit="weiter" data-gt-orbit-flotte="${esc(o.flotte)}" data-gt-orbit-ziel="${k}"`,titel:`Eigene gestrandete Flotte (${n})${o.nur_heim?' – nur noch zu einem eigenen Planeten oder Übergabe':''}`},
    {id:'orbit-versorgen-'+o.flotte,m:'transport',text:'Versorgen',attr:`data-gt-orbit="versorgen" data-gt-orbit-flotte="${esc(o.flotte)}" data-gt-orbit-ziel="${k}"`,titel:`Deuterium und Güter zur gestrandeten Flotte (${n}) bringen`});
   else out.push({id:'orbit-angriff-'+o.flotte,z:'gestrandet',text:`Gestrandete Flotte von ${o.spieler} angreifen`,attr:`data-gt-orbit="angriff" data-gt-orbit-flotte="${esc(o.flotte)}" data-gt-orbit-ziel="${k}"`,titel:`${n} im Orbit von [${p.koord}]`});
  }
  if(window.Einsatz?.oeffnen)out.push({id:'alle',z:'alle',text:'Alle Aufträge',attr:`data-gt-einsatz="" data-gt-einsatz-ziel="${k}"`,titel:'Einsatzplanung mit diesem Ziel: alle Aufträge mit ihren Vorbedingungen'});
  return out;
 }
 // ---------------------------------------------------------------- Aktionszeichen (ein Satz für alle Ansichten)
 // Karl 24.10.: „die Aktionsicons sind noch sehr mies umgesetzt, alle farbig!“ – jede Aktion derselbe Knopf: abgerundetes
 // Quadrat, 1 px Rand und leicht getönte Fläche in ihrer Farbe, Linienzeichnung 24×24 mit Strich 2 in derselben Farbe.
 // Aufträge zeichnet und färbt Auftraege (R8); die übrigen Aktionen haben eigene Zeichnungen und feste Töne.
 const ZEICHEN={
  nachricht:'M3 6h18v12H3ZM3 7l9 6 9-6',
  kontakt:'M9 11a4 4 0 1 0 0-8 4 4 0 0 0 0 8ZM2 21a7 7 0 0 1 14 0M19 8v6M16 11h6',
  phalanx:'M4 20h16M12 20v-6M12 14a2 2 0 1 0 0-4 2 2 0 0 0 0 4ZM8.5 8.5a5 5 0 0 1 7 0M5.5 5.5a9 9 0 0 1 13 0',
  mond:'M20 14.5A8 8 0 1 1 9.5 4a6.5 6.5 0 0 0 10.5 10.5Z',
  waehlen:'M12 3a9 9 0 1 0 0 18 9 9 0 0 0 0-18ZM8 12l3 3 5-6',
  umbenennen:'M4 20h4L19 9l-4-4L4 16v4ZM14 6l4 4',
  info:'M12 3a9 9 0 1 0 0 18 9 9 0 0 0 0-18ZM12 11v6M12 7.5v.5',
  alle:'M9 6h11M9 12h11M9 18h11M4.5 6h.5M4.5 12h.5M4.5 18h.5',
  gestrandet:'M3 8h18l-3 4H6ZM12 4v4M12 12v8M8.5 16.5a3.5 3.5 0 0 0 7 0M10 14.5h4',
  bodenkampf:'M5 19 15 9M15 9l2-5 3 3-5 2M19 19 9 9M9 9 7 4 4 7l5 2M4 20l3-3M20 20l-3-3'
 };
 const FARBE={nachricht:'kontakt',kontakt:'kontakt',phalanx:'phalanx',waehlen:'eigen',gestrandet:'feind',bodenkampf:'feind'};
 const zeichen=(x,n)=>x.m&&A()?A().svg(x.m,n):`<svg class="auftrag-svg" viewBox="0 0 24 24" width="${n}" height="${n}" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true" focusable="false"><path d="${ZEICHEN[x.z]||ZEICHEN.info}"/></svg>`;
 const farbe=x=>x.m?missionKlasse(x.m):'gt-farbe-'+(x.eigenOrbit?'eigen':FARBE[x.z]||'neutral');
 // Zeichen der gestrandeten Flotte(n) eines Platzes mit Schiffszahl; eigene grün, fremde rot.
 const orbitText=L=>L.orbit.map(o=>`${o.eigen?'eigene':o.spieler} · ${fmt(o.schiffe)} ${Number(o.schiffe)===1?'Schiff':'Schiffe'}`).join('; ');
 function orbitZeichen(L,n=18){
  if(!L.orbit.length)return '';const eigen=L.orbit.filter(o=>o.eigen),fremd=L.orbit.filter(o=>!o.eigen),summe=l=>l.reduce((s,o)=>s+(Number(o.schiffe)||0),0);
  const z=(cls,l)=>l.length?`<span class="gt-orbit ${cls}"><svg viewBox="0 0 24 24" width="${n}" height="${n}" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true" focusable="false"><path d="${ZEICHEN.gestrandet}"/></svg><b>${esc(kurzMenge(summe(l)))}</b></span>`:'';
  return z('gt-orbit-fremd',fremd)+z('gt-orbit-eigen',eigen);
 }
 const TRENNEN={Raketenschlag:'Raketen&shy;schlag',Kolonisieren:'Koloni&shy;sieren',Stationieren:'Statio&shy;nieren',Kampfkolonisieren:'Kampf&shy;koloni&shy;sieren',Bombardieren:'Bombar&shy;dieren',Systemscan:'System&shy;scan'};
 const beschriftung=t=>esc(t).replace(/[A-Za-zÄÖÜäöüß]{10,}/g,w=>TRENNEN[w]||w);
 // Gesperrt: keine Auftragsattribute, sondern der Grund (Tipp zeigt ihn); gedämpft, gestrichelter Rand.
 const ziel=x=>x.grund?`aria-disabled="true" data-gt-grund="${esc(x.grund)}"`:x.attr;
 const titelAttr=x=>x.titel||x.grund?` title="${esc(x.grund?x.text+' – '+x.grund:x.titel)}"`:'';
 const vorlesen=x=>esc(x.text+(x.grund?' – nicht möglich: '+x.grund:x.titel?' – '+x.titel:''));
 // Hauptaktion am Handy: Zeichen über der Beschriftung (wie die untere Leiste).
 const knopfHaupt=x=>`<button type="button" class="gt-akt gt-hauptaktion ${farbe(x)}" ${ziel(x)}${titelAttr(x)} aria-label="${vorlesen(x)}">${zeichen(x,18)}<span>${beschriftung(x.text)}</span></button>`;
 // Beschriftete Aktion („⋯“ und Infokarte); gesperrt mit sichtbarem Grund.
 const knopfLeiste=(x,haupt)=>`<button type="button" class="gt-akt gt-aktionsknopf${haupt?' gt-primaer':''} ${farbe(x)}" ${ziel(x)}${titelAttr(x)} aria-label="${vorlesen(x)}">${zeichen(x,18)}<span>${beschriftung(x.text)}${x.grund?`<small>${esc(x.grund)}</small>`:''}</span></button>`;
 // Desktop-Aktionsspalte (§4): gleich große Zeichenknöpfe, Hauptaktion zuerst und kräftiger; Mond und TF haben eigene Spalten.
 const NICHT_IN_SPALTE=new Set(['mond','mondflug','recyceln','umbenennen']);
 const symbolSpalte=liste=>liste.filter(a=>!NICHT_IN_SPALTE.has(a.id)).map(a=>`<button type="button" class="gt-akt gt-akt-symbol${a.haupt?' gt-primaer':''}${a.id==='angriff'?' gt-angriff':''} ${farbe(a)}" ${ziel(a)}${titelAttr(a)} aria-label="${vorlesen(a)}">${zeichen(a,18)}</button>`).join('');

 // ---------------------------------------------------------------- Bausteine einer Zeile
 function bildHtml(L){const {p,bekannt,zone}=L;return bekannt?`<img src="${esc(planetArt({...p,zone}))}" alt="${esc(ZONEN[zone]||named(zone))}" class="${p.besiegt?'gt-besiegt':''}">`:'<span class="gt-unbekannt" aria-hidden="true"></span>';}
 const spielerKnopf=(name,klasse='gt-name')=>`<button type="button" class="${klasse}" data-gt-spieler="${esc(name)}" title="Spielerprofil von ${esc(name)} öffnen">${esc(name)}</button>`;
 // Status als kurzes Wort statt Kürzel – Android zeigt keine Tooltips (Nachprüfung 25.10.2026: „(s)“ blieb unerklärt).
 const ST_WORT={b:'besiegt',u:'Urlaub',g:'gesperrt',I:'inaktiv 28 T',i:'inaktiv 7 T',n:'Neuling',s:'viel stärker'};
 const statusHtml=st=>st.length?` <span class="gt-status">(${st.map(([k,t])=>`<abbr title="${esc(t)}" class="gt-st-${k==='I'?'ii':k}">${esc(ST_WORT[k]||k)}</abbr>`).join(', ')})</span>`:'';
 function eignungChip(p){const g=klima(p),s=sonne(p);if(!g)return '';
  const t=`Für dein Volk: Klima ${g.prozent} % (${g.bewohnbar?'bewohnbar':'Terraforming '+g.noetig+' nötig'})${g.tag!=null?' · Wachstum '+zahl(g.tag)+' % je Tag':''}${s!=null?' · Sonne '+zahl(s)+' % (Platz 6 = 100 %)':''}`;
  return `<span class="gt-eignung gt-eig-${g.bewohnbar?stufe(g.prozent):'null'}" title="${esc(t)}">Klima ${g.bewohnbar?g.prozent+' %':'unbewohnbar'}${s!=null?` · Sonne ${zahl(s)} %`:''}</span>`;}
 const aktivitaet=p=>Number.isFinite(p.aktivitaet_minuten)&&p.aktivitaet_minuten<60?p.aktivitaet_minuten:null;
 const tfMenge=tf=>`${fmt(tf.erz)} Erz · ${fmt(tf.kristall)} Kristall${Number(tf.ueberlebende)>0?` · ${fmt(tf.ueberlebende)} Überlebende`:''}`;

 // Desktop-Zeile (§4: Platz · Planet · Name · Mond · TF · Spieler · Allianz · Aktion)
 function zeile(L,liste){
  const {p,eigen,rel,zone,tf,st,rang:r}=L,k=esc(p.koord),dim=filter.size&&![...filter].every(f=>passt(p,f));
  const akt=aktivitaet(p);
  const name=eigen?`<button type="button" class="gt-eigen" data-planet-pick="${k}" title="Zu diesem Planeten wechseln">${esc(Planetenname.name(eigen))}</button>${Planetenname.stift(p.koord,'gt-stift')}`
   :L.fremd?`<span>${p.name?`<b class="pn-name">${esc(p.name)}</b> · `:''}${esc(p.ausMond?'Planet nicht erkundet':artFremd(p))}${p.besiegt?' · besiegt':''}</span>`
   :`<span class="${L.frei?'gt-frei':'gt-unerforscht'}">${esc(titelText(L))}</span><small class="gt-name-unter">${esc(unterText(L))}</small>`;
  const tfZeile=(tf?`<span class="gt-tf-text">Trümmerfeld: ${esc(tfMenge(tf))}</span>`:'')+(L.orbit.length?`<span class="gt-orbit-text">Gestrandete Flotte: ${esc(orbitText(L))}</span>`:'')
   +(L.verwuestet?`<span class="gt-verwuestet-marke" title="Rückhalt gebrochen – ${L.besatzer?'dein Kolonieschiff kann landen':'die Besatzung hält den Boden'}">Verwüstet</span>`:L.invasionEigen?'<span class="gt-verwuestet-marke gt-invasion-marke">Invasion</span>':'');
  // Mond vernichten (nur mit eigenen Weltenkrümmern, fremder Mond): Chip aus galaxy.js mit Chance-Ring
  const vernichten=window.GalaxyUI?.vernichtenChip?GalaxyUI.vernichtenChip(p,{klein:true}):'';
  const mond=p.mond?`<button type="button" class="gt-mondknopf" ${p.mond.eigen||eigen?`data-mond-oeffnen="${k}"`:`data-gt-mondziel="${k}"`} title="${esc((p.mond.name||'Mond')+' · '+mondTyp(p.mond)+' · '+fmt(p.mond.durchmesser||p.mond.groesse||0)+' km'+(p.mond.besitzer?' · '+p.mond.besitzer:'')+(p.mond.eigen||eigen?' – Mondseite öffnen':' – Flug zum Mond planen'))}">${mondBild(p.mond,p.mond.name||'Mond')}</button>${p.mond.bedroht?'<span class="gt-warn" title="Feindlicher Anflug auf diesen Mond">⚠</span>':''}`:'';
  const orbitKnopf=L.orbit.length?`<button type="button" class="gt-orbitknopf" ${L.orbit.length===1&&!L.orbit[0].eigen?`data-gt-orbit="angriff" data-gt-orbit-flotte="${esc(L.orbit[0].flotte)}" data-gt-orbit-ziel="${k}"`:`data-gt-planet-info="${k}"`} title="Gestrandete Flotte: ${esc(orbitText(L))}" aria-label="Gestrandete Flotte: ${esc(orbitText(L))}${L.orbit.length===1&&!L.orbit[0].eigen?' – angreifen':' – Infokarte'}">${orbitZeichen(L,16)}</button>`:'';
  const trumm=orbitKnopf+(tf?`<button type="button" class="gt-tf" data-gt-einsatz="recyceln" data-gt-einsatz-ziel="${k}" title="Trümmerfeld: ${esc(tfMenge(tf))} – recyceln" aria-label="Trümmerfeld recyceln: ${esc(tfMenge(tf))}">✦</button>`:'');
  const spieler=p.spieler?`<span class="gt-spieler gt-rel-${esc(rel)}">${spielerKnopf(p.spieler)}${statusHtml(st)}${r?`<small>Rang ${esc(r.rang)}</small>`:''}${p.ausMond?'<small title="Aus der Mondsicht der Sternkarte">laut Mond</small>':''}</span>`:'';
  const allianz=p.beziehung?.allianz_tag?`<abbr title="${esc(p.beziehung.allianz||'')} · ${esc(p.beziehung.bezeichnung||'')}">${esc(p.beziehung.allianz_tag)}</abbr>`:'';
  const symbole=symbolSpalte(liste);
  return `<tr class="gt-zone-${esc(zone)} gt-bez-${esc(rel)}${L.verwuestet?' gt-verwuestet':''}${dim?' gt-gedimmt':''}${L.aktiv?' gt-aktiv':''}${karteKoord===p.koord&&karteOffen()?' gt-gewaehlt':''}" data-koord="${k}"><td class="gt-platz" title="${esc(ZONEN[zone]||'')}">${p.position}</td><td class="gt-planet"><button type="button" class="gt-planet-info" data-gt-planet-info="${k}" aria-label="Platz ${p.position} [${k}]: Infokarte öffnen">${bildHtml(L)}</button>${akt!=null?`<span class="gt-aktiv-plakette" title="Aktivität vor ${akt} min">${akt<15?'*':akt}</span>`:''}${window.DysonUI?DysonUI.blitz(p.koord):''}</td><td class="gt-planetname">${name}${eignungChip(p)}${tfZeile}${vernichten}</td><td class="gt-mondzelle">${mond}</td><td class="gt-tfzelle">${trumm}</td><td class="gt-spielerzelle">${spieler}</td><td class="gt-allianz">${allianz}</td><td class="gt-aktionzelle">${symbole?`<span class="gt-aktion">${symbole}</span>`:''}</td></tr>`;
 }
 // Art des Platzes unter dem Namen – dasselbe Wort in Liste, Tabelle und Infokarte.
 function unterText(L){const {p,eigen,rang:r}=L;
  if(eigen)return `${eigen.name?(eigen.heimat?'Heimatwelt':'Kolonie')+' · ':''}eigene Welt`;
  if(L.fremd)return `${p.name?p.name+' · ':''}${p.ausMond?'laut Mond · Planet nicht erkundet':artFremd(p)}${p.besiegt?' · besiegt':''}${r?' · Rang '+r.rang:''}`;
  if(L.kolonisierbar){const kl=kolonieLage();return [Number(p.felder)>0?fmt(p.felder)+' Baufelder':'',p.alter_stunden!=null?'erkundet vor '+fmt(p.alter_stunden)+' h':'erkundet',kl.frei>0?'':'kein Kolonieplatz frei'].filter(Boolean).join(' · ');}
  // Ein System mit eigenem Planeten oder Systembericht ist erfasst (Kern system_erfasst) – unbekannt ist dann nur der Platz
  // (Funktionsprüfung 25.10.2026: Kopf „Kartiert“, Zeilen „System nicht kartiert“).
  return L.frei?'noch nicht erkundet':L.besiedlung?'unbewohnt':systemErfasst(p.koord)?'Bewohner unbekannt – System erkunden':'System nicht kartiert – Systemscan';}
 function systemErfasst(k){const s=String(k).split(':').slice(0,2).join(':'),gleich=x=>String(x||'').split(':').slice(0,2).join(':')===s;return (view.planeten||[]).some(x=>gleich(x.koord))||(view.systemberichte||[]).some(b=>gleich(b.system||b.ziel||b.koord));}
 // Handy-Eintrag (Karl 24.10.: „wo ist der Platz für Mond und Trümmerfeld?“): wie die Tabelle feste Spalten, in jeder
 // Zeile an derselben Stelle – Platz/Planet · Name · Mond · TF · Aktion (Hauptaktion über „⋯“). Leere Zellen zeigen einen
 // ruhigen Strich. Darunter Zone/Klima-Marken über Name, Mond und TF; „⋯“ klappt die übrigen Aktionen auf.
 // Spaltenmaße stehen einmal in galaxie-kompakt.css (--gt-spalten) und gelten für Kopfzeile und Einträge.
 const SPALTENKOPF='<div class="gt-liste-spalten" aria-hidden="true"><span>Platz</span><span>Name</span><span>Mond</span><span>TF</span><span>Aktion</span></div>';
 // Kurze Menge für die TF-Spalte (voller Wert in Infokarte, title und Vorlesetext).
 function kurzMenge(n){n=Math.max(0,Number(n)||0);const z=(v,e)=>(v<10?fmt(Math.round(v*10)/10):fmt(Math.round(v)))+' '+e;
  return n>=1e9?z(n/1e9,'Mrd.'):n>=1e6?z(n/1e6,'Mio.'):n>=1e4?z(n/1e3,'Tsd.'):fmt(Math.round(n));}
 function mondZelle(L){
  const {p,eigen}=L,m=p.mond,k=esc(p.koord);
  if(!m)return `<span class="gt-zelle-leer gt-mondspalte" title="${L.besiedlung?'kein Mond':'Mond unbekannt'}">–</span>`;
  const eigenMond=m.eigen||!!eigen,info=`${m.name||'Mond'} · ${mondTyp(m)} · ${fmt(m.durchmesser||m.groesse||0)} km${m.besitzer&&!eigenMond?' · '+m.besitzer:''}`;
  return `<button type="button" class="gt-mondspalte gt-zellknopf" ${eigenMond?`data-mond-oeffnen="${k}"`:`data-gt-mondziel="${k}"`} title="${esc(info)}" aria-label="Mond ${esc(info)} – ${eigenMond?'Mondseite öffnen':'Aufträge gegen den Mond planen'}${m.bedroht?' – feindlicher Anflug':''}">${mondBild(m,m.name||'Mond')}${m.bedroht?'<b class="gt-zell-warn" aria-hidden="true">⚠</b>':''}</button>`;
 }
 function tfZelle(L){
  const {p,tf}=L,k=esc(p.koord);
  // Gestrandete Flotte: eigenes Zeichen an der Stelle des Trümmerfelds, beide sichtbar. Eine fremde Flotte allein: Tipp greift
  // an (wie ✦ recycelt); mehrere Möglichkeiten (eigene Flotte, Trümmer daneben): Tipp öffnet die Infokarte mit allen Aktionen.
  if(L.orbit.length){
   const allein=!tf&&L.orbit.length===1&&!L.orbit[0].eigen,ziel=allein?`data-gt-orbit="angriff" data-gt-orbit-flotte="${esc(L.orbit[0].flotte)}" data-gt-orbit-ziel="${k}"`:`data-gt-planet-info="${k}"`;
   const leute=Number(tf?.ueberlebende)||0,summe=(Number(tf?.erz)||0)+(Number(tf?.kristall)||0);
   return `<button type="button" class="gt-tfspalte gt-zellknopf gt-orbitzelle" ${ziel} title="Gestrandete Flotte: ${esc(orbitText(L))}${tf?' · Trümmerfeld: '+esc(tfMenge(tf)):''}" aria-label="Gestrandete Flotte: ${esc(orbitText(L))}${tf?' · Trümmerfeld '+esc(tfMenge(tf)):''} – ${allein?'angreifen':'Infokarte'}"><span class="gt-tf-oben">${orbitZeichen(L,15)}</span>${tf?`<span class="gt-tf-menge"><span class="gt-tf-symbol" aria-hidden="true">✦</span>${esc(kurzMenge(summe))}${leute?` <b class="gt-tf-leute" aria-hidden="true">${esc(kurzMenge(leute))}</b>`:''}</span>`:''}</button>`;
  }
  if(!tf)return '<span class="gt-zelle-leer gt-tfspalte" title="kein Trümmerfeld bekannt">–</span>';
  const leute=Number(tf.ueberlebende)||0,summe=(Number(tf.erz)||0)+(Number(tf.kristall)||0);
  return `<button type="button" class="gt-tfspalte gt-zellknopf" data-gt-einsatz="recyceln" data-gt-einsatz-ziel="${k}" title="Trümmerfeld: ${esc(tfMenge(tf))} – recyceln" aria-label="Trümmerfeld ${esc(tfMenge(tf))} – recyceln"><span class="gt-tf-oben"><span class="gt-tf-symbol" aria-hidden="true">✦</span>${leute?`<b class="gt-tf-leute" aria-hidden="true">${esc(kurzMenge(leute))}</b>`:''}</span><span class="gt-tf-menge">${esc(kurzMenge(summe))}</span></button>`;
 }
 function eintrag(L,liste){
  const {p,eigen,rel,zone,st,rang:r}=L,k=esc(p.koord),pos=p.position,dim=filter.size&&![...filter].every(f=>passt(p,f));
  const haupt=liste.find(x=>x.haupt),rest=liste.filter(x=>x!==haupt&&!['recyceln','mond','mondflug'].includes(x.id)),auf=offen===p.koord,akt=aktivitaet(p);
  // Spielername als echtes 44-px-Ziel (Kastenhöhe, keine überstehende Tippfläche); darunter die Art des Platzes.
  // Der Stift (Umbenennen) steht in „⋯“ und in der Infokarte – in der schmalen Namensspalte verdeckte er die Unterzeile
  // („eigene Well“ auf Karls Foto).
  const titel=eigen?`<b class="gt-titel-name gt-rel-eigen">${esc(Planetenname.name(eigen))}</b>`
   :L.fremd?`<span class="gt-titel-name gt-spieler gt-rel-${esc(rel)}">${spielerKnopf(p.spieler)}${statusHtml(st)}</span>`
   :`<b class="gt-titel-name ${L.frei?'gt-frei':'gt-unerforscht'}">${esc(titelText(L))}</b>`;
  const unter=unterText(L);
  const marken=[eignungChip(p),
   p.beziehung?.allianz_tag&&!eigen?`<span class="gt-marke">[${esc(p.beziehung.allianz_tag)}]</span>`:'',
   p.mond?.bedroht?'<span class="gt-marke gt-marke-warn">⚠ Anflug auf den Mond</span>':'',
   L.verwuestet?`<span class="gt-marke gt-marke-verwuestet">Verwüstet${L.besatzer?' · Kolonieschiff kann landen':''}</span>`:L.invasionEigen?'<span class="gt-marke gt-marke-verwuestet">Invasion läuft</span>':'',
   ...L.orbit.map(o=>`<span class="gt-marke ${o.eigen?'gt-marke-orbit-eigen':'gt-marke-orbit-fremd'}">Gestrandet: ${esc(o.eigen?'eigene Flotte':o.spieler)} · ${esc(fmt(o.schiffe))}</span>`),
   akt!=null?`<span class="gt-marke gt-marke-aktiv">aktiv vor ${akt<15?'unter 15':akt} min</span>`:'',
   window.DysonUI?DysonUI.blitz(p.koord):''].filter(Boolean).join('');
  const vernichten=auf&&window.GalaxyUI?.vernichtenChip?GalaxyUI.vernichtenChip(p,{klein:true}):'';
  const weitere=auf?`<div class="gt-weitere" id="gt-weitere-${pos}" role="group" aria-label="Weitere Aktionen für Platz ${pos}">${rest.map(x=>knopfLeiste(x)).join('')}${vernichten}${knopfLeiste({id:'info',z:'info',text:'Infokarte',attr:`data-gt-planet-info="${k}"`})}</div>`:'';
  return `<li class="gt-eintrag gt-zone-${esc(zone)} gt-bez-${esc(rel)}${L.verwuestet?' gt-verwuestet':''}${dim?' gt-gedimmt':''}${L.aktiv?' gt-aktiv':''}${auf?' gt-offen':''}${karteKoord===p.koord&&karteOffen()?' gt-gewaehlt':''}" data-koord="${k}">
<button type="button" class="gt-planet-info" data-gt-planet-info="${k}" aria-label="Platz ${pos}, ${esc(ZONEN[zone]||'')}: Infokarte öffnen">${bildHtml(L)}<b class="gt-platznr">${pos}</b><small class="gt-zone-wort">${esc(ZONE_KURZ[zone]||'')}</small></button>
<div class="gt-titel">${titel}<button type="button" class="gt-unterzeile" data-gt-planet-info="${k}" tabindex="-1">${esc(unter)}</button></div>
${mondZelle(L)}${tfZelle(L)}
<div class="gt-haupt">${haupt?knopfHaupt(haupt):L.aktiv?'<span class="gt-aktiv-marke">aktiv</span>':''}</div>
<button type="button" class="gt-mehr" data-gt-mehr="${k}" aria-expanded="${auf}" aria-controls="gt-weitere-${pos}" aria-label="Weitere Aktionen für Platz ${pos}">⋯</button>
<div class="gt-marken">${marken}</div>${weitere}</li>`;
 }

 function kopfHtml(w,besiedelt){
  const p=currentPlanet(),sonden=p.schiffe.spionagesonde||0,recycler=p.schiffe.recycler||0,slots=view.flottenplaetze||{},kartiert=!!daten?.bekannt;
  const nav=(art,titel,wert,max)=>`<label class="gt-navfeld"><span>${titel}</span><span class="gt-navinput"><button type="button" data-gt-schritt="${art}:-1" aria-label="Vorherig${art==='sektor'?'er Sektor':'es System'}">‹</button><input id="gt-${art}" type="number" inputmode="numeric" enterkeyhint="go" min="1" max="${max}" value="${wert}" aria-label="${titel}"><button type="button" data-gt-schritt="${art}:1" aria-label="Nächst${art==='sektor'?'er Sektor':'es System'}">›</button></span></label>`;
  const reload='<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.7" aria-hidden="true"><path d="M20 7v5h-5M20 12a8 8 0 1 0-2 6M20 7v5"/></svg>';
  const archiv='<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.7" aria-hidden="true"><rect x="4" y="4" width="16" height="17" rx="2"/><path d="M8 9h8M8 13h8M8 17h5M8 2v4M16 2v4"/></svg>';
  // Systemprofil (Berichte-Agent) als Zusatz: der Systemtitel wird antippbar, sobald es angeboten wird.
  const titel=`<small>SONNENSYSTEM</small><b>${sektor}<i>:</i>${system}</b>`;
  return `<section class="gt-console" aria-label="Systemnavigation und Direktaktionen">
   <div class="gt-console-top">${window.Systemprofil?.oeffnen?`<button type="button" class="gt-systemtitel" data-gt-systemprofil="${sektor}:${system}" title="Systemprofil öffnen">${titel}</button>`:`<span class="gt-systemtitel">${titel}</span>`}<span class="gt-sensorstatus${kartiert?' bereit':''}">${kartiert?(daten.nebel?'Nebel':'Kartiert'):'Unerkundet'}<small>${besiedelt} besiedelt</small></span><button type="button" class="gt-iconbutton gt-refresh" data-gt-refresh title="Systemdaten aktualisieren" aria-label="Systemdaten aktualisieren" ${laedt?'disabled':''}>${reload}</button><button type="button" class="gt-iconbutton" data-gt-atlas title="Zur 3D-Ansicht" aria-label="3D-Ansicht öffnen">3D</button></div>
   <div class="gt-navigation">${nav('sektor','Sektor',sektor,w.sektoren)}${nav('system','System',system,w.systeme_je_sektor)}<button type="button" class="gt-anzeigen" data-gt-los title="System anzeigen" aria-label="System anzeigen">→</button></div>
   <div class="gt-direkt"><button type="button" class="gt-mission ${missionKlasse('system_erkunden')}" data-system-scan="${sektor}:${system}:1" title="Genau eine Sonde kartiert Besiedlung, Monde und Trümmer" aria-label="Systemscan mit einer Sonde starten">${motiv('system_erkunden',24)}<span><b>Systemscan</b><small>1 Sonde</small></span></button>
   ${daten?.asteroidenguertel?`<button type="button" class="gt-mission ${missionKlasse('abbau')}" data-gt-einsatz="abbau" data-gt-einsatz-ziel="${sektor}:${system}:0" title="Asteroidengürtel abbauen (Einsatzplanung)">${motiv('abbau',24)}<span><b>${esc(auftragName('abbau'))}</b><small>Asteroiden</small></span></button>`:`<button type="button" class="gt-mission ${missionKlasse('expedition')}" data-go="expeditions-panel">${motiv('expedition',24)}<span><b>Expedition</b><small>Planen</small></span></button>`}
   <button type="button" class="gt-mission gt-archiv" data-intel-focus="${sektor}:${system}" title="Gesammelte Systemberichte und Scan-Historie">${archiv}<span><b>Archiv</b><small>Aufklärung</small></span></button></div>
   <div class="gt-bestandszeile"><label class="gt-sondenvorgabe ${missionKlasse('spionage')}">${motiv('spionage',18)}<span>Je Spionage</span><input id="gt-sondenzahl" type="number" inputmode="numeric" min="1" max="1000" value="${sondenAnzahl}" aria-label="Sonden je Planetenspionage (startet bei jedem Tipp auf Ausspähen, wird gespeichert)"></label><span class="gt-mini-bestand" aria-label="${fmt(sonden)} Spionagesonden am aktiven Planeten">${motiv('system_erkunden',15)}<b>${fmt(sonden)}</b></span>${recycler?`<span class="gt-mini-bestand ${missionKlasse('recyceln')}" aria-label="${fmt(recycler)} Recycler am aktiven Planeten">${motiv('recyceln',15)}<b>${fmt(recycler)}</b></span>`:''}<span class="gt-slots">Flotten <b>${slots.belegt??0}/${slots.gesamt??0}</b></span></div>
   <details class="gt-optionen"><summary>Filter & weitere Aktionen <span aria-hidden="true">⌄</span></summary><div class="gt-options-innen"><div class="gt-filter" role="group" aria-label="Systemfilter">${FILTER.map(([k,t])=>`<button type="button" data-gt-filter="${k}" aria-pressed="${filter.has(k)}">${t}</button>`).join('')}</div><button type="button" class="gt-mission ${missionKlasse('spionage')}" data-scan-system="${sektor}:${system}:1">${motiv('spionage',18)} Alle Planeten einzeln scannen</button><p>Einzelaufklärung startet für jeden Planeten eine eigene Sonde. Der Systemscan darüber benötigt insgesamt eine Sonde.</p><button type="button" class="gt-mission ${missionKlasse('expedition')}" data-go="expeditions-panel">${motiv('expedition',18)} Expedition planen</button></div></details>
  </section>`;
 }
 const FILTER=[['E','Erkundet'],['I','Inaktiv'],['N','Neuling'],['A','Allianz'],['V','Urlaub']];
 function passt(p,f){const st=status(p).map(x=>x[0]);const rel=beziehung(p);return f==='E'?!!p.bekannt:f==='I'?st.includes('i')||st.includes('I'):f==='N'?st.includes('n'):f==='A'?['eigen','verbuendet'].includes(rel)||!!p.beziehung?.allianz&&p.beziehung.allianz===view.allianz?.name:f==='V'?st.includes('u'):true;}
 const LEGENDE_AKT={spion:{m:'spionage'},erkunden:{m:'system_erkunden'},angriff:{m:'angriff'},rakete:{m:'rakete'},nachricht:{z:'nachricht'},kontakt:{z:'kontakt'},phalanx:{z:'phalanx'},alle:{z:'alle'}};
 function legende(){
  return `<details class="gt-legende"><summary>ⓘ Legende</summary><dl>
<dt><span class="gt-legende-zone gt-zone-glut"></span><span class="gt-legende-zone gt-zone-leben"></span><span class="gt-legende-zone gt-zone-frost"></span></dt><dd>Zonenfarbe am Platz: Glut · Leben · Frost (aus dem Sonnenabstand, für jeden Platz bekannt)</dd>
<dt><span class="gt-unbekannt"></span></dt><dd>Unerforscht – Systemscan oder Sonde nötig; Planet antippen öffnet die Infokarte</dd>
<dt>${mondBild(null,'Mond')}</dt><dd>Mond (Name, Typ, Durchmesser in der Infokarte; Tipp: eigene Mondseite oder Flug zum fremden Mond) · ⚠ feindlicher Anflug auf den Mond</dd>
<dt>Hyperraumloch</dt><dd>Eigene Zeile über Platz 1: Ziel, Restzeit, Knopf „Durchreisen“ (nur wenn das Loch für dich sichtbar ist)</dd>
<dt><span class="gt-dyson-symbol">☀</span> <span class="dy-blitz">⚡</span></dt><dd>Dysonsphäre: Zeile über Platz 1 mit Allianz, Phase und Fortschritt (Klick: Sonnenansicht); ⚡ an die Sphäre angeschlossener Planet</dd>
<dt><span class="gt-aktiv-plakette">*</span> <span class="gt-aktiv-plakette">42</span></dt><dd>Aktivität: unter 15 Minuten / vor 42 Minuten (erscheint, sobald der Kern Aktivität meldet)</dd>
<dt>Klima 97 % · Sonne 83 %</dt><dd>Für dein Volk: Klima in % des bestmöglichen Wachstums, Sonnenlicht für Solarkraftwerke (Platz 6 = 100 %)</dd>
<dt>✦</dt><dd>Trümmerfeld mit Erz/Kristall (und Überlebenden) – Tipp: Recyceln in der Einsatzplanung</dd>
<dt><abbr>i</abbr> <abbr>I</abbr></dt><dd>inaktiv seit 7 / 28 Tagen</dd>
<dt><abbr>n</abbr> <abbr>s</abbr></dt><dd>deutlich schwächer (Neuling) / deutlich stärker als du (nach Punkten, Faktor 5)</dd>
<dt><abbr>u</abbr> <abbr>g</abbr> <abbr>b</abbr></dt><dd>Urlaubsmodus · gesperrt · besiegt</dd>
<dt><span class="gt-name gt-rel-eigen">Name</span> <span class="gt-name gt-rel-verbuendet">Name</span> <span class="gt-name gt-rel-nap">Name</span> <span class="gt-name gt-rel-feindlich">Name</span></dt><dd>Eigene Welt · verbündet · Nichtangriffspakt · feindlich – Spielername antippen öffnet das Spielerprofil</dd>
<dt>${['spion','erkunden','angriff','rakete','nachricht','kontakt','phalanx','alle'].map(id=>`<span class="gt-akt gt-akt-symbol gt-legende-akt ${farbe(LEGENDE_AKT[id])}">${zeichen(LEGENDE_AKT[id],16)}</span>`).join(' ')}</dt><dd>Ausspähen (bewohnt, gemerkte Sondenzahl, ein Tipp) · Erkunden (freier Platz, 1 Sonde) · Angriff planen · Raketenschlag · Nachricht · Kontakt/Vertrag · Phalanx (nur mit eigener Tiefenphalanx in Reichweite) · Alle Aufträge (Einsatzplanung). Gestrichelt = gerade nicht möglich, Tipp nennt den Grund</dd>
<dt>⋯</dt><dd>Am Handy: weitere Aktionen der Zeile (auch Umbenennen eigener Planeten); die Hauptaktion steht darüber. Mond und TF stehen wie in der Tabelle in jeder Zeile an derselben Stelle, „–“ = keiner bekannt; die Zahl am ✦ ist Erz + Kristall, die kleine Marke die Überlebenden</dd>
<dt>E I N A V</dt><dd>Filter: erkundet · inaktiv · Neuling · Allianz · Urlaub (nicht passende Zeilen werden abgedunkelt)</dd></dl></details>`;
 }
 function sonderzeilen(){
  const guertel=daten?.asteroidenguertel?{art:'guertel',html:`<div class="gt-guertel-inhalt"><span class="gt-sonder-symbol" aria-hidden="true">☄</span><span class="gt-sonder-text"><strong>Asteroidengürtel</strong> <span class="koord">[${sektor}:${system}:0]</span><small>${daten.bergbau?`Rohstoffumfang ${esc(daten.bergbau.groesse||'nicht vermessen')} · je Bergbauschiff und Spielstunde ${fmt(daten.bergbau.erz_je_stunde)} Erz + ${fmt(daten.bergbau.kristall_je_stunde)} Kristall`:'Erz und Kristall abbauen'}</small></span><button type="button" class="gt-aktionsknopf ${missionKlasse('abbau')}" data-gt-einsatz="abbau" data-gt-einsatz-ziel="${sektor}:${system}:0">${motiv('abbau',16)}<span>${esc(auftragName('abbau'))}</span></button></div>`}:null;
  // Ein Hyperraumloch ist nur offen_bis (Spielsekunden) sichtbar; danach verschwindet die Zeile ohne neue Abfrage.
  const hlRoh=daten?.hyperraumloch,hl=hlRoh&&!(hlRoh.offen_bis!=null&&hlRoh.offen_bis<=GameTime.now())?hlRoh:null;
  clearTimeout(lochTimer);if(hl&&hl.offen_bis!=null)lochTimer=setTimeout(render,Math.max(250,(hl.offen_bis-GameTime.now())/Math.max(1,world?.tempo||1)*1000+300));
  const sichtbar=hl&&(view.monde?.hyperraumloecher||[]).some(l=>l.id===hl.id);
  const loch=hl?{art:'loch',html:`<span class="gt-loch-symbol" aria-hidden="true">◎</span> Hyperraumloch nach <span class="koord">[${esc(String(hl.ziel).split(':').slice(0,2).join(':'))}]</span>${hl.offen_bis!=null?' · offen noch <b data-arrival="'+esc(hl.offen_bis)+'"></b>':''}${hl.kapazitaet_rest!=null?' · Restkapazität '+fmt(hl.kapazitaet_rest)+' Struktur':''} ${sichtbar?`<button type="button" class="gt-zweit" data-loch-reisen="${esc(hl.id)}">Durchreisen</button>`:''}`}:null;
  return {guertel,loch};
 }
 function render(){
  const host=$('galaxie-tabelle');if(!host||!view||$('karte').hidden||$('karte').dataset.ansicht==='atlas')return;
  startwerte();const w=welt(),liste=plaetze(),lagen=liste.map(lage),besiedelt=lagen.filter(L=>L.p.spieler||L.eigen).length;
  const kopf=kopfHtml(w,besiedelt),{guertel,loch}=sonderzeilen();
  const merkmale=daten?[daten.nebel?'Nebelsystem':'',daten.asteroidenguertel?'Asteroidengürtel':'',daten.hyperraumloch?`Hyperraumloch nach [${esc(daten.hyperraumloch.ziel)}]`:''].filter(Boolean).join(' · '):'';
  const ueber=`System <span class="koord">[${sektor}:${system}]</span>${merkmale?' · '+merkmale:''}${laedt?' · lädt …':''}${daten&&!daten.bekannt?' · noch nicht kartiert':''}`;
  const dyson=window.DysonUI?DysonUI.galaxieZeile(daten):'';
  let system_;
  if(handy()){
   // Sonderzeilen als eigene Einträge; Dyson-Zeile kommt als Tabellenzeile – Inhalt übernehmen.
   const dy=dyson?`<li class="gt-sonder gt-dyson">${dyson.replace(/^\s*<tr[^>]*>\s*<td[^>]*>/,'').replace(/<\/td>\s*<\/tr>\s*$/,'')}</li>`:'';
   const g=guertel?`<li class="gt-sonder gt-guertel">${guertel.html}</li>`:'',l=loch?`<li class="gt-sonder gt-loch">${loch.html}</li>`:'';
   system_=`<p class="gt-liste-kopf">${ueber}</p>${SPALTENKOPF}<ol class="gt-liste" aria-label="Plätze im System [${sektor}:${system}]">${dy}${l}${lagen.map((L,i)=>eintrag(L,aktionen(L))+(i===6?g:'')).join('')}${lagen.length<7?g:''}</ol>`;
  }else{
   const g=guertel?`<tr class="gt-guertel"><td colspan="8">${guertel.html}</td></tr>`:'',l=loch?`<tr class="gt-loch"><td colspan="8">${loch.html}</td></tr>`:'';
   system_=`<div class="gt-scroll"><table class="gt-tabelle"><caption>${ueber}</caption><thead><tr><th>Platz</th><th>Planet</th><th>Name</th><th>Mond</th><th>TF</th><th>Spieler (Status)</th><th>Allianz</th><th>Aktion</th></tr></thead><tbody>${dyson}${l}${lagen.map((L,i)=>zeile(L,aktionen(L))+(i===6?g:'')).join('')}${lagen.length<7?g:''}</tbody></table></div>`;
  }
  const fuss=`<div class="gt-fuss"><span>${besiedelt} ${besiedelt===1?'Planet':'Planeten'} besiedelt${fehler?` · <span class="gt-fehler">${esc(fehler)}</span>`:''}</span>${legende()}</div>`;
  setStableHTML(host,kopf+system_+fuss);
  if(karteOffen())karteZeichnen();
 }
 function gehe(s,n){const w=welt();sektor=klemmen(s,w.sektoren);system=klemmen(n,w.systeme_je_sektor);offen='';karteZu();void laden();}
 // Ort aus anderen Modulen zeigen (Einsatzplanung „In der Galaxie zeigen“, Berichte): Galaxie als Tabelle, das System,
 // bei einer Planetenkoordinate dazu dessen Infokarte.
 function zeigen(koord){
  const [s,n,pos]=String(koord||'').split(':').map(Number);if(!s||!n)return;
  if(typeof navigateGame==='function')navigateGame('karte');
  const karte=$('karte');if(karte&&karte.dataset.ansicht==='atlas')zeigeAtlas(false);
  zeigenNach=pos>0?`${s}:${n}:${pos}`:'';gehe(s,n);
 }
 // Umschalten 3D ↔ Tabelle zeigt dasselbe Sonnensystem.
 function zeigeAtlas(an){
  const karte=$('karte');karte.dataset.ansicht=an?'atlas':'tabelle';try{localStorage.setItem('sternenepoche.galaxie.ansicht',karte.dataset.ansicht);}catch{}
  if(an){karteZu();if(window.GalaxyUI){GalaxyUI.visible(!karte.hidden);if(sektor&&system)GalaxyUI.zeige?.(sektor,system);GalaxyUI.einstieg?.();}return;}
  const s=window.GalaxyUI?.snapshot?.();if(window.GalaxyUI)GalaxyUI.visible(false);
  if(s?.stage==='system'&&(s.sektor!==sektor||s.system!==system)){gehe(s.sektor,s.system);return;}
  // Immer frisch laden: sonst blieb nach einer Systemsonde „noch nicht kartiert“ stehen (Karl 22.10.2026).
  if(!laedt)void laden();else render();
 }

 // ---------------------------------------------------------------- Infokarte (nicht-modales Blatt)
 const karteHost=()=>{let k=$('gt-planetkarte');if(!k){document.body.insertAdjacentHTML('beforeend','<section id="gt-planetkarte" class="gt-karte" role="dialog" aria-modal="false" aria-labelledby="gt-karte-titel" hidden></section>');k=$('gt-planetkarte');}return k;};
 function karteOffen(){const k=$('gt-planetkarte');return !!k&&!k.hidden&&!!karteKoord;}
 function groesse(L){
  const {p,eigen}=L,basis=Number(p.felder)||0;
  if(eigen){const g=Number(eigen.felder?.gesamt)||0,b=Number(eigen.felder?.belegt)||0;return `${fmt(b)} von ${fmt(g)} Baufeldern belegt${basis&&g>basis?` <small>Grundgröße ${fmt(basis)}, +${fmt(g-basis)} durch Terraforming und Orbitalring</small>`:''}`;}
  if(L.bekannt&&basis)return `${fmt(basis)} Baufelder <small>Grundgröße · nach der Besiedlung erweitern Terraforming und Orbitalring</small>`;
  return '<span class="gt-leise">unbekannt – Planetensonde nötig</span>';
 }
 function letzterBericht(k){
  const b=(view.berichte||[]).filter(x=>x.ziel===k).sort((a,c)=>(c.zeit||0)-(a.zeit||0))[0];if(!b)return '';
  const summe=o=>Object.values(o||{}).reduce((s,n)=>s+(Number(n)||0),0),alter=Number.isFinite(b.alter_stunden)?b.alter_stunden:null;
  return `vor ${alter==null?'?':alter<1?'unter 1':fmt(alter)} Spielstunden · ${b.schiffe?fmt(summe(b.schiffe))+' Schiffe':'Flotte nicht erfasst'} · ${b.verteidigung?fmt(summe(b.verteidigung))+' Abwehr':'Abwehr nicht erfasst'}${b.bestand?` · ${fmt((b.bestand.erz||0)+(b.bestand.kristall||0)+(b.bestand.deuterium||0))} Rohstoffe`:''}`;
 }
 function karteHtml(L){
  const {p,eigen,bekannt,fremd,zone,tf,st,rang:r}=L,k=p.koord,pos=Number(p.position),n=welt().plaetze_je_system||12,sn=sonne(p);
  const typ=bekannt&&window.PlanetKugel?PlanetKugel.typ({...p,zone}):'',typName=typ?String(window.STERNEN_ART?.maps?.[`planets|${typ}|`]?.label||'').replace(/^Planet\s+/,'').replace(/\s*\(.*\)\s*$/,''):'';
  const liste=aktionen(L),haupt=liste.find(x=>x.haupt);
  const zeile=(t,v,cls='')=>`<div class="${cls}"><dt>${esc(t)}</dt><dd>${v}</dd></div>`;
  const f=[];
  f.push(zeile('Lage',`${esc(ZONEN[zone]||'')} · Platz ${pos} von ${n}${sn!=null?` · Sonne ${zahl(sn)} %`:''}`));
  f.push(zeile('Besitzer',eigen?`${esc(view.name)} <small>du</small>`:fremd?`<span class="gt-spieler gt-rel-${esc(L.rel)}">${spielerKnopf(p.spieler)}${statusHtml(st)}</span> <small>${esc(beziehungText(p))}${r?' · Rang '+esc(r.rang)+' · '+fmt(r.punkte)+' Punkte':''}${p.ausMond?' · laut Mond':''}</small>`:L.frei?'niemand – frei':L.besiedlung?'niemand':'<span class="gt-leise">unbekannt – Systemscan nötig</span>'));
  if(p.beziehung?.allianz_tag&&!eigen)f.push(zeile('Allianz',`[${esc(p.beziehung.allianz_tag)}] ${esc(p.beziehung.allianz||'')}`));
  f.push(zeile('Planetentyp',typName?esc(typName):'<span class="gt-leise">unbekannt – Planetensonde nötig</span>'));
  f.push(zeile('Größe',groesse(L)));
  if(bekannt)f.push(zeile('Rohstoffe',typeof planetResources==='function'?planetResources(p):'—'));
  if(p.mond){const m=p.mond,vern=window.GalaxyUI?.vernichtenChip?GalaxyUI.vernichtenChip(p,{klein:true}):'';f.push(zeile('Mond',`${esc((m.name||'Mond')+' · '+mondTyp(m)+(m.durchmesser||m.groesse?' · '+fmt(m.durchmesser||m.groesse)+' km':'')+(m.besitzer&&!m.eigen&&!eigen?' · '+m.besitzer:''))}${m.bedroht?' <b class="gt-warn">⚠ feindlicher Anflug</b>':''}${vern?`<span class="gt-karte-chip">${vern}</span>`:''}`));}
  else f.push(zeile('Mond',L.besiedlung?'keiner':'<span class="gt-leise">unbekannt</span>'));
  f.push(zeile('Trümmerfeld',tf?esc(tfMenge(tf)):'keines bekannt'));
  if(L.orbit.length)f.push(zeile('Gestrandete Flotten',L.orbit.map(o=>`<span class="gt-orbit-zeile ${o.eigen?'gt-orbit-eigen':'gt-orbit-fremd'}">${orbitZeichen({orbit:[o]},14)} ${esc(o.eigen?'deine Flotte':o.spieler)} · ${fmt(o.schiffe)} ${Number(o.schiffe)===1?'Schiff':'Schiffe'}${o.nur_heim?' · nur noch Heimflug':''}</span>`).join('')));
  if(L.verwuestet)f.push(zeile('Invasion',`<b class="gt-warn">Verwüstet</b> – Rückhalt gebrochen, der Besitzer hat die Kontrolle verloren. ${L.besatzer?'Dein Kolonieschiff (Kampfkolonisieren oder Kolonisieren) landet und übernimmt, sobald ein Kolonieplatz frei ist.':eigen?'Lande neue Truppen, dann steht der Rückhalt wieder.':'Ein Kolonieschiff der Besatzerseite kann landen.'}`,'gt-gefahr'));
  else if(L.invasionEigen){const w=eigen?.invasion?.wellen||L.inv?.wellen;f.push(zeile('Invasion',`<b class="gt-warn">Invasion läuft</b>${w?` – Linie ${fmt(w.linie)} von ${fmt(w.linien_anzahl)} · nächste Welle in <b data-arrival-echt="${esc(w.naechste_welle)}"></b>`:''}`,'gt-gefahr'));}
  const bericht=letzterBericht(k);
  f.push(zeile('Aufklärung',eigen?'eigener Planet':p.alter_stunden!=null?`Planetensonde vor ${fmt(p.alter_stunden)} Spielstunden${bericht?' · Spionage '+esc(bericht):''}`:bericht?'Spionage '+esc(bericht):L.besiedlung?'nur Besiedlung bekannt (Systemscan)':'<span class="gt-leise">noch nichts bekannt</span>'));
  const akt=aktivitaet(p),stx=[p.urlaub?'Urlaubsmodus':'',p.inaktiv_tage>=7?'inaktiv seit '+p.inaktiv_tage+' Tagen':'',p.gesperrt?'gesperrt':'',p.besiegt?'besiegt':'',akt!=null?'aktiv vor '+(akt<15?'unter 15':akt)+' min':''].filter(Boolean);
  if(stx.length)f.push(zeile('Status',esc(stx.join(' · '))));
  const flotten=(view.flotten||[]).filter(x=>x.ziel===k&&x.zustand!=='rueckflug'),angr=(view.angriffe||[]).filter(a=>a.ziel===k);
  {const unterwegs=flotten.filter(x=>x.zustand!=='im_orbit'),orbitF=flotten.filter(x=>x.zustand==='im_orbit');
   if(unterwegs.length)f.push(zeile('Deine Flotten',`${unterwegs.length} unterwegs · nächste in <b data-arrival-echt="${Math.min(...unterwegs.map(x=>x.ankunft_sekunden||0))}"></b>`));
   if(orbitF.length)f.push(zeile('Im Orbit',`${orbitF.length} eigene ${orbitF.length===1?'Flotte':'Flotten'}`));}
  if(angr.length)f.push(zeile('Angriff',`<b class="gt-warn">⚠ ${angr.length} feindliche ${angr.length>1?'Flotten':'Flotte'} im Anflug</b>`,'gt-gefahr'));
  // Klima je Herkunft als Liste (bei 360 px vollständig lesbar, keine Tabelle mehr) – eigenes Volk zuerst.
  const h=p.habitabilitaet,reihe=[view.volk,...VOELKER.filter(v=>v!==view.volk)];
  const klimaListe=h?.herkunft?`<section class="gt-karte-klima" aria-label="Klima und Besiedlung"><h3>Klima und Besiedlung</h3><ul>${reihe.map(v=>{const e=klima(p,v);if(!e)return '';
    return `<li class="${e.bewohnbar?'':'gt-unbewohnbar'}${v===view.volk?' gt-eigenes-volk':''}"><b>${esc(named(v))}${v===view.volk?' <small>dein Volk</small>':''}</b><span>${e.bewohnbar?`Klima ${e.prozent} %${e.tag!=null?` · Wachstum ${zahl(e.tag)} % je Tag`:''}${e.noetig?` · braucht Terraforming ${e.noetig} (du: ${e.eigen})`:''}`:`unbewohnbar – Terraforming ${e.noetig} nötig (du: ${e.eigen})${e.verlust?` · −${fmt(e.verlust/10)} % Bewohner je Stunde`:''}`}</span></li>`;}).join('')}</ul><p class="gt-leise">Klima = Anteil am bestmöglichen Wachstum der Herkunft. Versorgung, Stabilität und Wohnraum begrenzen zusätzlich; Sonnenlicht wirkt auf Solarkraftwerke.</p></section>`:'';
  const bild=bekannt?`<img src="${esc(planetArt({...p,zone}))}" alt="" class="${p.besiegt?'gt-besiegt':''}">`:'<span class="gt-unbekannt" aria-hidden="true"></span>';
  return `<div class="gt-karte-innen"><header class="gt-karte-kopf gt-zone-${esc(zone)}"><span class="gt-karte-bild">${bild}</span><div class="gt-karte-titel"><small>PLATZ ${pos} · ${esc((ZONEN[zone]||'').toUpperCase())}</small><h2 id="gt-karte-titel" tabindex="-1">${esc(titelText(L))}</h2><p><span class="koord">[${esc(k)}]</span> · ${esc(zustandText(L))}</p></div><button type="button" class="gt-karte-zu" data-gt-karte-zu aria-label="Infokarte schließen">✕</button></header>
<div class="gt-karte-aktionen" role="group" aria-label="Aktionen für [${esc(k)}]">${haupt?knopfLeiste(haupt,true):''}${liste.filter(x=>x!==haupt).map(x=>knopfLeiste(x)).join('')}</div>
<dl class="gt-karte-fakten">${f.join('')}</dl>${klimaListe}</div>`;
 }
 function karteZeichnen(){
  const host=karteHost(),p=plaetze().find(x=>x.koord===karteKoord);
  if(!p||!view){karteZu();return;}
  const neu=host.dataset.koord!==karteKoord;host.dataset.koord=karteKoord;
  setStableHTML(host,karteHtml(lage(p)));
  if(neu){const innen=host.querySelector('.gt-karte-innen');if(innen)innen.scrollTop=0;}
 }
 function karteAuf(koord,ausloeser){
  if(!koord)return;const war=karteOffen();karteKoord=koord;karteFokus=ausloeser||document.activeElement;
  const host=karteHost();host.hidden=false;document.body.classList.add('gt-karte-offen');karteZeichnen();render();
  if(!war||ausloeser)setTimeout(()=>{try{host.querySelector('#gt-karte-titel')?.focus({preventScroll:true});}catch(_){}},30);
 }
 function karteZu(){
  const host=$('gt-planetkarte'),war=karteOffen();if(host)host.hidden=true;document.body.classList.remove('gt-karte-offen');
  karteKoord='';if(host)delete host.dataset.koord;
  if(war){const f=karteFokus;karteFokus=null;if(f?.isConnected)try{f.focus({preventScroll:true});}catch(_){}render();}
 }

 // ---------------------------------------------------------------- Aufträge
 // Jeder Flottenauftrag über die Einsatzplanung (window.Einsatz.oeffnen) – ohne sie der bisherige Weg.
 function einsatz(ziel,mission,zielart='planet'){
  if(window.Einsatz?.oeffnen){window.Einsatz.oeffnen({ziel,mission:mission||undefined,zielart});return;}
  if((mission==='abbau'||mission==='recyceln')&&window.Einsatzplanung?.oeffnen){Einsatzplanung.oeffnen(ziel,mission);return;}
  if(zielart==='mond'){pickFleet(ziel,mission||'spionage');const f=$('fleet')?.elements;if(f?.zielart)f.zielart.value='mond';return;}
  if(mission&&window.Flottenfuehrung?.planen){Flottenfuehrung.planen(ziel,mission);return;}
  if(typeof window.pickFleet==='function')window.pickFleet(ziel,mission||'transport');
 }
 // Direktspionage: die gemerkte Anzahl, ohne Flottenformular oder zweite Bestätigung.
 async function spionieren(ziel,anzahl){
  if(spionBusy.has(ziel))return;
  const n=Math.max(1,Math.min(1000,Math.floor(Number(anzahl)||Number($('gt-sondenzahl')?.value)||sondenAnzahl)));
  // Start braucht Sonden UND einen Raumhafen (Kern flotte.rs) – der aktive Planet ohne Raumhafen scheiterte erst im Kern.
  const tauglich=p=>(p?.schiffe?.spionagesonde||0)>=n&&(p?.gebaeude?.raumhafen||0)>0,eigen=currentPlanet(),start=tauglich(eigen)?eigen:view.planeten.find(tauglich);
  if(!start)throw Error(view.planeten.some(p=>(p.schiffe.spionagesonde||0)>=n)?`${n} Spionagesonden stehen nur auf Planeten ohne Raumhafen – dort kann nichts starten.`:`${n} Spionagesonden benötigt. Auf keinem Startplaneten stehen genug bereit. Ändere die Anzahl über der Tabelle.`);
  spionBusy.add(ziel);
  try{await command([{typ:'flotte_senden',start:start.koord,ziel,mission:'spionage',geschwindigkeit:1,haltedauer_stunden:0,schiffe:{spionagesonde:n},ladung:{}}]);}
  finally{spionBusy.delete(ziel);}
 }
 document.addEventListener('input',e=>{if(e.target.id==='gt-sondenzahl'&&Number(e.target.value)>=1){sondenAnzahl=Math.max(1,Math.min(1000,Math.floor(Number(e.target.value))));try{localStorage.setItem('sternenepoche.spionage.sonden',String(sondenAnzahl));}catch{}}});
 document.addEventListener('change',e=>{if(e.target.id==='gt-sondenzahl'){e.target.value=String(sondenAnzahl);render();}});
 // Zeile antippen (nicht auf einem Knopf) öffnet die Infokarte – größere Tippfläche als das Planetensymbol (Befund D3/H9).
 document.addEventListener('click',e=>{
  const li=e.target.closest('#galaxie-tabelle .gt-eintrag[data-koord]');if(!li||e.target.closest('button,a,input,label,summary,select,abbr'))return;
  karteAuf(li.dataset.koord,li.querySelector('.gt-planet-info'));
 });
 document.addEventListener('keydown',e=>{if(e.key==='Escape'&&karteOffen()){e.preventDefault();karteZu();}});
 document.addEventListener('click',guard(async e=>{
  const b=e.target.closest('[data-gt-orbit],[data-gt-schritt],[data-gt-los],[data-gt-filter],[data-gt-atlas],[data-gt-spion],[data-gt-kontakt],[data-gt-rakete],[data-gt-tabelle],[data-gt-phalanx],[data-gt-mondziel],[data-gt-refresh],[data-gt-planet-info],[data-gt-einsatz],[data-gt-mehr],[data-gt-spieler],[data-gt-systemprofil],[data-gt-karte-zu],[data-gt-grund]');if(!b)return;
  if(b.dataset.gtGrund){show(b.dataset.gtGrund);return;}
  if(b.dataset.gtOrbit){const art=b.dataset.gtOrbit,fid=Number(b.dataset.gtOrbitFlotte),ziel=b.dataset.gtOrbitZiel;karteZu();offen='';
   if(!window.Einsatz?.oeffnen){show('Die Einsatzplanung ist noch nicht geladen.',true);return;}
   if(art==='angriff')window.Einsatz.oeffnen({ziel,mission:'angriff',zielflotte:fid});
   else if(art==='weiter')window.Einsatz.oeffnen({gestrandet:fid});
   else if(art==='versorgen')window.Einsatz.oeffnen({ziel,mission:'transport',versorgen:fid});
   return;}
  if(b.dataset.gtPlanetInfo){if(karteOffen()&&karteKoord===b.dataset.gtPlanetInfo&&!b.closest('.gt-weitere')){karteZu();return;}karteAuf(b.dataset.gtPlanetInfo,b);return;}
  if(b.hasAttribute('data-gt-karte-zu')){karteZu();return;}
  if(b.dataset.gtMehr){const k=b.dataset.gtMehr;offen=offen===k?'':k;render();setTimeout(()=>{const w=$('galaxie-tabelle')?.querySelector(`.gt-eintrag[data-koord="${CSS.escape(k)}"] .gt-weitere`);if(w&&w.getBoundingClientRect().bottom>innerHeight-(parseFloat(getComputedStyle(document.documentElement).getPropertyValue('--themenleiste-h'))||0))w.scrollIntoView({block:'nearest'});},30);return;}
  if(b.dataset.gtSpieler){const n=b.dataset.gtSpieler;if(window.Spielerprofil?.oeffnen)window.Spielerprofil.oeffnen(n);else if(window.Gegnerakte?.oeffnen)Gegnerakte.oeffnen(n);return;}
  if(b.dataset.gtSystemprofil){window.Systemprofil?.oeffnen?.(b.dataset.gtSystemprofil);return;}
  if(b.hasAttribute('data-gt-einsatz')){const ziel=b.dataset.gtEinsatzZiel;if(!ziel)return;karteZu();offen='';einsatz(ziel,b.dataset.gtEinsatz,b.dataset.gtEinsatzArt==='mond'?'mond':'planet');return;}
  if(b.hasAttribute('data-gt-refresh')){await refresh();await laden();show('Systemdaten aktualisiert.');return;}
  if(b.dataset.gtPhalanx){await command([{typ:'phalanx_scannen',mond:b.dataset.phalanxMond,ziel:b.dataset.gtPhalanx}]);const r=(view.monde?.phalanxberichte||[])[0];if(r&&r.ziel===b.dataset.gtPhalanx)show(`Phalanx [${r.ziel}]: ${r.kontakte.length?r.kontakte.map(k=>readable(k)).join(' | '):'keine Flottenbewegungen'}`);return;}
  // Flug zum fremden Mond (auch aus der 3D-Planetenkarte): Einsatzplanung mit Zielart Mond, sonst Flottenformular.
  if(b.dataset.gtMondziel){karteZu();einsatz(b.dataset.gtMondziel,window.Einsatz?.oeffnen?'':'spionage','mond');return;}
  if(b.dataset.gtSchritt){const [art,d]=b.dataset.gtSchritt.split(':');if(art==='sektor')gehe(sektor+Number(d),system);else{let n=system+Number(d),s=sektor;const w=welt();if(n<1&&s>1){s--;n=w.systeme_je_sektor;}if(n>w.systeme_je_sektor&&s<w.sektoren){s++;n=1;}gehe(s,n);}return;}
  if(b.hasAttribute('data-gt-los')){gehe($('gt-sektor').value,$('gt-system').value);return;}
  if(b.dataset.gtFilter){const f=b.dataset.gtFilter;filter.has(f)?filter.delete(f):filter.add(f);render();return;}
  if(b.hasAttribute('data-gt-atlas')){zeigeAtlas(true);return;}
  if(b.hasAttribute('data-gt-tabelle')){zeigeAtlas(false);return;}
  // Ein-Tipp-Spionage: Knopf sofort gesperrt und „läuft“, bis der Server antwortet (Handy-Regel Rückmeldung).
  if(b.dataset.gtSpion){b.closest('dialog')?.close();b.disabled=true;b.setAttribute('aria-busy','true');try{await spionieren(b.dataset.gtSpion,b.dataset.gtSonden);}finally{b.disabled=false;b.removeAttribute('aria-busy');}return;}
  if(b.dataset.gtKontakt){karteZu();navigateGame('briefkasten');document.querySelector('[data-mail-folder="anfragen"]')?.click();const f=$('request-form');if(f){f.closest('details').open=true;f.elements.partner.value=b.dataset.gtKontakt;f.elements.art.value='nichtangriffspakt';f.scrollIntoView({block:'center'});f.elements.text.focus({preventScroll:true});}return;}
  if(b.dataset.gtRakete){karteZu();const st=b.dataset.gtRaketeStart;if(st&&st!==currentPlanet()?.koord&&window.PlanetNavigation?.select)PlanetNavigation.select(st);navigateGame('verteidigung');const f=$('missile-launch');if(!f){show('Raketen brauchen ein Raketensilo auf diesem Planeten.',true);return;}if(f.elements.flotte)f.elements.flotte.value='';if(f.elements.ziel)f.elements.ziel.value=b.dataset.gtRakete;f.dispatchEvent(new Event('change',{bubbles:true}));f.scrollIntoView({block:'center'});return;}
 }));
 // Nachricht und Planetenwahl aus der Infokarte verlassen den Ort – Karte schließen, damit sie nicht über der Zielseite liegt.
 document.addEventListener('click',e=>{if(e.target.closest('#gt-planetkarte [data-mail-to],#gt-planetkarte [data-mond-oeffnen],#gt-planetkarte [data-g3-vernichten]'))setTimeout(karteZu,0);});
 document.addEventListener('keydown',e=>{if(e.key==='Enter'&&['gt-sektor','gt-system'].includes(e.target.id)){e.preventDefault();gehe($('gt-sektor').value,$('gt-system').value);}});
 document.addEventListener('change',e=>{if(!['gt-sektor','gt-system'].includes(e.target.id))return;const s=klemmen($('gt-sektor')?.value,welt().sektoren),n=klemmen($('gt-system')?.value,welt().systeme_je_sektor);if(s!==sektor||n!==system)gehe(s,n);});
 // Zurück (Android/Rahmen): erst die Infokarte, dann die aufgeklappten Aktionen schließen.
 window.Rahmen?.zurueckHaken?.push(()=>{if(karteOffen()){karteZu();return true;}if(offen&&!$('karte')?.hidden){offen='';render();return true;}return false;});
 // Wechsel Handy ↔ Desktop (Drehen, Fenstergröße): Liste und Tabelle tauschen.
 HANDY.addEventListener?.('change',()=>{offen='';render();});
 document.addEventListener('DOMContentLoaded',()=>{
  // Handy (< 768 px): Galaxie öffnet direkt die 3D-Ansicht, die Tabelle ist ein Umschalter darin. Desktop: Tabelle zuerst.
  const karte=$('karte');karte.dataset.ansicht=matchMedia('(max-width: 767px)').matches?'atlas':'tabelle';try{const gemerkt=localStorage.getItem('sternenepoche.galaxie.ansicht');if(['atlas','tabelle'].includes(gemerkt))karte.dataset.ansicht=gemerkt;}catch{}
  karte.insertAdjacentHTML('afterbegin','<div id="galaxie-tabelle" aria-label="Sonnensystem als Tabelle"></div>');
  karte.querySelector('.space-header')?.insertAdjacentHTML('beforeend','<button type="button" class="gt-zweit" data-gt-tabelle>Zur Systemtabelle</button>');
  // Der 3D-Atlas läuft nur, wenn er ausdrücklich gewählt ist (WebGL kostet Akku und GPU).
  if(window.GalaxyUI){const sichtbar=GalaxyUI.visible;GalaxyUI.visible=v=>sichtbar(!!v&&karte.dataset.ansicht==='atlas');}
 });
 // Erstes Öffnen: System des aktiven Planeten laden; danach nur auf Wunsch (Los!, Pfeile). Wer die Galaxie verlässt,
 // nimmt die Infokarte nicht mit.
 document.addEventListener('game-navigation',()=>{if($('karte')&&!$('karte').hidden&&$('karte').dataset.ansicht!=='atlas'){if(!laedt)void laden();else render();}else{offen='';karteZu();}});
 // Planetenwechsel oder neue Lage: Zähler und eigene Markierungen aktualisieren.
 {const ansicht=renderView;renderView=function(){ansicht.apply(this,arguments);const mark=JSON.stringify([view?.systemberichte,view?.berichte?.map(b=>[b.ziel,b.zeit]),view?.truemmer]);const neu=letzteScans!==null&&mark!==letzteScans;letzteScans=mark;if(neu&&!laedt&&!$('karte')?.hidden&&$('karte')?.dataset.ansicht!=='atlas')void laden();else render();};}
 return {render,gehe,zeigen,laden,spionieren,karte:karteAuf,karteZu,zone:zoneVon,einsatz,raketenLage};
})();
// Auch als Fenstereigenschaft, damit andere Module optional darauf zugreifen können.
window.GalaxieTabelle=GalaxieTabelle;
// „Auf der Karte“ an Angriffswarnungen und Kampfberichten: Systemtabelle des Herkunftsplaneten öffnen.
document.addEventListener('click',e=>{const b=e.target.closest('[data-gt-ziel]');if(!b)return;const [s,n]=String(b.dataset.gtZiel).split(':').map(Number);if(!s||!n)return;e.preventDefault();navigateGame('karte');const k=document.getElementById('karte');if(k&&k.dataset.ansicht==='atlas')k.querySelector('[data-gt-tabelle]')?.click();GalaxieTabelle.gehe(s,n);});
