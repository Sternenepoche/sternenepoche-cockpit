'use strict';
// Funk – Nachrichten- und Berichtszentrale (Karl 24.10.2026: „… primär Nachrichten, Spionageberichte, Kampfberichte und so
// weiter, dass ich das auf dem Smartphone besser verwalten, ordnen und strukturieren kann. Berichte nach Personen oder
// Sonnensystemen gefiltert …“, „nicht so viel auf einen Blick, gute Verschachtelung“, „keine Widersprüche mehr“).
// Eine Zentrale statt zweier Listen (vorher Briefkasten-Ordner „Berichte & Meldungen“ und Seite „Berichte“ nebeneinander):
// Briefe, Berichte und Meldungen als Chronik nach Tagen, zusammengehörige Ereignisse im 20-Minuten-Fenster gebündelt
// (Codex 24.10.: Systemscan und Planetensonden eines Systems, Kampf mit Begleitmeldungen). Umschalter nach Spieler/System.
// Detail, Spieler- und Systemprofil liegen in EINEM nicht-modalen Blatt (ein modales <dialog> macht die Seite inert und
// verdeckt das Android-Overlay); Zurück schließt erst das Blatt, dann den Filter, dann die Seite. Ein Planet öffnet die eine
// Infokarte der Galaxie (GalaxieTabelle.zeigen), Flottenaufträge die Einsatzplanung (Einsatz.oeffnen bzw. pickFleet).
// Lesestatus: ein geöffneter Bericht geht sofort an den Server – nur mit echter _postfach_id aus reportRows()
// (Fair-Play-Stolperdraht); ohne anschließendes refresh() (vorher 3 Abrufe je Bericht, Quote „meta“ 30/min).
// „Alle gelesen“ wirkt sofort auf diesem Gerät und gleicht im Hintergrund mit höchstens 10 Abrufen je Minute ab.
// Markieren, Ausblenden und das Archiv älterer Berichte (der Server hält z. B. nur die letzten 20 Kampfberichte und
// 150 Meldungen) liegen nur auf diesem Gerät (localStorage; ohne ihn läuft alles weiter, nur ohne Merker/Archiv).
window.Funk=(()=>{
 const h=s=>esc(s);
 const summe=o=>Object.values(o||{}).reduce((n,v)=>n+(Number(v)||0),0);
 const sysVon=k=>{const m=/^(\d+):(\d+)/.exec(String(k||''));return m?`${m[1]}:${m[2]}`:'';};
 const koordIn=t=>(/\b\d+:\d+:\d+\b/.exec(String(t||''))||[''])[0];
 const ich=()=>view?.name||'';
 const jetzt=()=>Date.now()/1000;
 const tempo=()=>Math.max(1,Number(world?.tempo)||1);
 // Briefe und Anfragen tragen nur Spielzeit; Berichte liefert der Server mit fester Echtzeit (`echtzeit`).
 const echtAusSim=s=>Number.isFinite(Number(s))&&s!==null?jetzt()-Math.max(0,(Number(view?.sekunden)||0)-Number(s))/tempo():jetzt();
 const zwei=n=>String(n).padStart(2,'0');
 const simText=s=>Number.isFinite(Number(s))&&s!==null?`Tag ${Math.floor(s/86400)+1} · ${zwei(Math.floor(s%86400/3600))}:${zwei(Math.floor(s%3600/60))}`:'';
 const UHR=new Intl.DateTimeFormat('de-DE',{hour:'2-digit',minute:'2-digit'});
 const DATUM=new Intl.DateTimeFormat('de-DE',{weekday:'short',day:'2-digit',month:'2-digit'});
 const tagNr=t=>{const d=new Date(t*1000);return d.getFullYear()*10000+(d.getMonth()+1)*100+d.getDate();};
 const tagName=t=>{const n=tagNr(t);return n===tagNr(jetzt())?'Heute':n===tagNr(jetzt()-86400)?'Gestern':DATUM.format(new Date(t*1000));};
 const uhr=t=>UHR.format(new Date(t*1000));
 function dauer(s){s=Math.max(0,Math.round(s));const T=Math.floor(s/86400),H=Math.floor(s%86400/3600),M=Math.floor(s%3600/60);return T?`${T} T ${H} h`:H?`${H} h ${M} min`:`${Math.max(1,M)} min`;}
 const alter=t=>jetzt()-t<60?'gerade eben':'vor '+dauer(jetzt()-t);
 const ortText=k=>k?(window.Planetenname?Planetenname.text(k):`[${k}]`):'';
 // Koordinaten im Text hellblau (§5), sonst reiner Text.
 const mitKoord=t=>h(t).replace(/\[(\d+:\d+(?::\d+)?)\]/g,'<span class="koord">[$1]</span>');
 const kurz=id=>String(id).replace(/[^a-z0-9]/gi,'').slice(0,24);
 const hash=s=>{let x=5381;for(const c of String(s))x=(x*33^c.charCodeAt(0))>>>0;return x.toString(36);};
 const ROEMISCH=['','I','II','III','IV','V','VI'];

 // ---------------------------------------------------------------- Ablage auf diesem Gerät
 const ARCHIV_MAX=1200,ARCHIV_REICH=300;
 const Speicher=(()=>{
  let key='',d=null,uhrzeit=0;
  function laden(){
   const k=`sternenepoche.funk.${world?.world_id||'-'}.${ich()}`;
   if(k===key&&d)return d;
   let roh={};try{roh=JSON.parse(localStorage.getItem(k)||'{}')||{};}catch(_){roh={};}
   key=k;d={gelesen:new Set(roh.gelesen||[]),markiert:new Set(roh.markiert||[]),aus:new Set(roh.aus||[]),archiv:new Map((roh.archiv||[]).map(a=>[a.id,a])),version:0};
   return d;
  }
  function speichern(){
   if(!d)return;d.version++;clearTimeout(uhrzeit);const k=key,daten=d;
   uhrzeit=setTimeout(()=>{
    const alle=[...daten.archiv.values()].sort((a,b)=>b.t-a.t);let reich=0;
    const archiv=alle.filter(a=>a.a!=='reich'||++reich<=ARCHIV_REICH).slice(0,ARCHIV_MAX);daten.archiv=new Map(archiv.map(a=>[a.id,a]));
    try{localStorage.setItem(k,JSON.stringify({v:1,gelesen:[...daten.gelesen].slice(-4000),markiert:[...daten.markiert].slice(-1000),aus:[...daten.aus].slice(-2000),archiv}));}catch(_){}
   },600);
  }
  return {laden,speichern};
 })();

 // ---------------------------------------------------------------- Lesestatus zum Server (nur echte Kennungen)
 let serverBerichte=new Set(),serverBriefe=new Set(),abgeglichen=null;
 const Sync=(()=>{
  const warte=new Map();let takt=0,pauseBis=0;const ABSTAND=6000;
  const zeilen=()=>typeof reportRows==='function'?reportRows():[];
  function gueltig(j){
   if(j.bericht)return !serverBerichte.has(j.bericht)&&zeilen().some(x=>x.id===j.bericht);
   return !serverBriefe.has(j.brief)&&(communicationData().briefkasten||[]).some(n=>n.id===j.brief&&!n.gesendet);
  }
  async function senden(j){
   await api('/api/inbox/read',j.bericht?{world_id:world.world_id,bericht:j.bericht}:{world_id:world.world_id,brief:j.brief});
   if(j.bericht)serverBerichte.add(j.bericht);else serverBriefe.add(j.brief);
  }
  function fehler(e,j){
   if(e.status===429){pauseBis=Date.now()+1000*Math.max(10,e.retryAfter||30);warte.set(j.schluessel,j);}
   else if(!e.status){pauseBis=Date.now()+20000;warte.set(j.schluessel,j);}
   // 400/403/409: Kennung nicht (mehr) im eigenen Postfach oder Welt ersetzt – nie wiederholen.
  }
  function planen(){if(takt||!warte.size)return;takt=setTimeout(lauf,Math.max(ABSTAND,pauseBis-Date.now()));}
  async function lauf(){
   takt=0;if(!warte.size)return;
   if(document.hidden||!world||!view||Date.now()<pauseBis){planen();return;}
   const [k,j]=warte.entries().next().value;warte.delete(k);
   if(gueltig(j))try{await senden(j);}catch(e){fehler(e,j);}
   planen();
  }
  function vormerken(j){if(!warte.has(j.schluessel)&&gueltig(j)){warte.set(j.schluessel,j);planen();}}
  async function sofort(j){
   if(Date.now()<pauseBis){vormerken(j);return;}
   warte.delete(j.schluessel);if(!gueltig(j))return;
   try{await senden(j);}catch(e){fehler(e,j);planen();}
  }
  return {vormerken,sofort,offen:()=>warte.size};
 })();
 // Lokal gelesene Einträge in die Sicht übernehmen, damit Kopf-, Leisten- und Übersichtszähler dasselbe sagen.
 function lesestandAbgleichen(){
  if(!view)return;
  if(abgeglichen!==view){
   abgeglichen=view;serverBerichte=new Set(view.postfach_gelesen||[]);view.postfach_gelesen=[...(view.postfach_gelesen||[])];
   serverBriefe=new Set((communicationData().briefkasten||[]).filter(n=>n.gelesen).map(n=>n.id));
  }
  const L=Speicher.laden(),liste=view.postfach_gelesen,da=new Set(liste);
  for(const x of typeof reportRows==='function'?reportRows():[]){
   // Chronik = ältere Vorfälle jenseits der 150 neuesten: gelten als gelesen (nur Anzeige, nichts an den Server).
   if(x.kind==='chronik'){if(!da.has(x.id)){liste.push(x.id);da.add(x.id);}continue;}
   if(!L.gelesen.has('b:'+x.id))continue;
   if(!da.has(x.id)){liste.push(x.id);da.add(x.id);}
   if(!serverBerichte.has(x.id))Sync.vormerken({schluessel:'b:'+x.id,bericht:x.id});
  }
  for(const n of communicationData().briefkasten||[])if(!n.gesendet&&L.gelesen.has('brief:'+n.id)){n.gelesen=true;if(!serverBriefe.has(n.id))Sync.vormerken({schluessel:'brief:'+n.id,brief:n.id});}
 }
 function zaehlerAuffrischen(){try{if(typeof renderReportInbox==='function')renderReportInbox();window.Rahmen?.synchronisieren?.();}catch(_){}}

 // ---------------------------------------------------------------- Einträge aus der Sicht
 const MELDUNG_ART={kampf:'kampf',beute:'kampf',bombardement:'kampf',blockade:'kampf',eroberung:'kampf',raketen:'kampf',miliz:'kampf',kampfkolonisation:'kampf',bodentruppen:'kampf',besiegt:'kampf',existenzkrise:'kampf',weltentruemmer_verloren:'kampf',mond:'kampf',reparaturdock:'kampf',reparatur:'kampf',spionage:'spionage',flottenaufklaerung:'spionage',systemaufklaerung:'scans',vertrag:'briefe',allianz:'briefe',abgelehnt:'briefe',geschenk:'briefe'};
 const MELDUNG_ZEICHEN={kampf:'angriff',beute:'angriff',eroberung:'angriff',bombardement:'bombardieren',blockade:'blockade',raketen:'rakete',miliz:'invasion',kampfkolonisation:'kampfkolonisieren',bodentruppen:'truppen',spionage:'spionage',flottenaufklaerung:'flotten_spionage',systemaufklaerung:'system_erkunden',recycling:'recyceln',abbau:'abbau',lieferung:'transport',markt:'transport',schiffhandel:'transport',geschenk:'transport',expedition:'expedition',kolonie:'kolonisieren',ueberlebende_geborgen:'recyceln',ueberlebende_warten:'recyceln'};
 const MELDUNG_NAME={kampf:'Kampf',beute:'Beute',bombardement:'Bombardierung',blockade:'Blockade',eroberung:'Eroberung',raketen:'Raketen',miliz:'Miliz',kampfkolonisation:'Kampfkolonisierung',bodentruppen:'Bodentruppen',besiegt:'Ausgeschieden',existenzkrise:'Existenzkrise',mond:'Mond',reparaturdock:'Werftdock',reparatur:'Reparatur',spionage:'Spionage',flottenaufklaerung:'Flottenaufklärung',systemaufklaerung:'Systemaufklärung',vertrag:'Vertrag',allianz:'Allianz',abgelehnt:'Diplomatie',flotte:'Flotte',markt:'Markt',lieferung:'Lieferung',schiffhandel:'Schiffhandel',geschenk:'Geschenk',expedition:'Expedition',kolonie:'Kolonie',recycling:'Recycling',abbau:'Abbau',fertigung:'Fertigung',stufe:'Zeitalter',mondbau:'Mondbau',flugplan:'Flugplan',unterhalt:'Unterhalt'};
 const artName=a=>MELDUNG_NAME[a]||(a?named(a):'Reich');
 // Kerntext lesbar: Schlüssel → Namen, große Zahlen mit Tausenderpunkten, Koordinaten in Klammern (§5).
 function meldungText(t){
  let s=String((typeof kernLesbar==='function'?kernLesbar(t):t)||'');
  s=s.replace(/(^|[\s(])erz\b/g,'$1Erz').replace(/(^|[^\d.,:])(\d{4,})(?![\d:])/g,(m,v,n)=>v+Number(n).toLocaleString('de-DE'));
  return s.replace(/(^|[^[\d:])(\d+:\d+:\d+)(?![\d:\]])/g,'$1[$2]');
 }
 const RANG={kampfberichte:9,bodenberichte:8,berichte:7,systemberichte:6,spionageberichte:5,flottenberichte:4,phalanxberichte:3};
 function kern(e,teile){e.kern=teile.filter(Boolean);}
 function ausBericht(x){
  const r=x.row,k=x.kind,sim=typeof reportTime==='function'?reportTime(r):Number(r.zeit)||0;
  const e={id:'b:'+x.id,pid:x.id,quelle:'bericht',kind:k,row:r,t:Number(r.echtzeit)||echtAusSim(sim),sim,spieler:[],koord:'',ton:'',zeichen:'',feind:false,buendel:''};
  if(k==='kampfberichte'){
   const vert=r.seite==='verteidigung',meins=vert?'verteidiger':'angreifer',remis=r.sieger==='unentschieden'||!r.sieger,sieg=!remis&&r.sieger===meins;
   const gegner=((vert?r.angreifer:r.verteidiger)||[]).filter(n=>n&&n!==ich());
   const eigen=summe(vert?r.verluste_verteidiger:r.verluste_angreifer),fremd=summe(vert?r.verluste_angreifer:r.verluste_verteidiger),beute=summe(r.beute),tf=summe(r.truemmer);
   const ort=ortText(r.ort)+(r.ziel_mond?' ☾':'');
   Object.assign(e,{art:'kampf',zeichen:window.Auftraege&&Auftraege.liste[r.mission]?r.mission:'angriff',feind:vert,ton:remis?'':sieg?'gut':'schlecht',koord:r.ort,spieler:gegner,buendel:'kampf:'+r.ort,truemmer:tf,
    titel:vert?(sieg?`Angriff abgewehrt · ${ort}`:`Angriff auf ${ort} · ${remis?'Unentschieden':'Niederlage'}`):`${remis?'Unentschieden':sieg?'Sieg':'Niederlage'} bei ${ort}`,
    unter:vert?`von ${gegner.join(', ')||'unbekannt'}`:`gegen ${gegner.join(', ')||'unbesetzt'}`,
    herkunft:[...new Set((r.herkunft||[]).filter(y=>y.spieler!==ich()).map(y=>y.start))],
    heimat:Object.fromEntries((r.herkunft||[]).filter(y=>y.spieler!==ich()&&y.heimat).map(y=>[y.spieler,y.heimat]))});
   kern(e,[beute&&!vert?`Beute ${fmt(beute)}`:beute?`geplündert ${fmt(beute)}`:'',`Verluste du ${fmt(eigen)} · Gegner ${fmt(fremd)}`,tf?`TF ${fmt(tf)}`:'',r.ueberlebende?`${fmt(r.ueberlebende)} Überlebende`:'',r.mondwurf?.entstanden?'Mond entstanden':'']);
  }else if(k==='bodenberichte'){
   const meins=r.seite==='angriff'?'angreifer':'verteidiger',er=meins==='angreifer'?'verteidiger':'angreifer',v=s=>(r[s+'_verluste']||[]).reduce((n,a)=>n+(Number(a)||0),0);
   const gegner=[r[er+'_name']].filter(n=>n&&n!==ich());
   Object.assign(e,{art:'kampf',zeichen:'invasion',feind:r.seite!=='angriff',ton:v(meins)>v(er)?'schlecht':'gut',koord:r.planet,spieler:gegner,buendel:'boden:'+r.planet,
    titel:`Bodenkampf ${ortText(r.planet)}`,unter:`${r.angreifer_name||'?'} gegen ${r.verteidiger_name||'?'}`});
   kern(e,[`Verluste du ${fmt(v(meins))} · Gegner ${fmt(v(er))}`]);
  }else if(k==='berichte'&&r.art==='erkundung'){
   Object.assign(e,{art:'scans',zeichen:'spionage',koord:r.ziel,buendel:'scan:'+sysVon(r.ziel),titel:`Erkundung ${ortText(r.ziel)} · freier Platz`,unter:`Zone ${named(r.zone||'')}`});
   kern(e,[`${fmt(r.felder)} Felder`,r.nebel?'Nebel':'',(r.orbit||[]).length?`${fmt(r.orbit.length)} Flotten im Orbit`:'']);
  }else if(k==='berichte'||k==='spionageberichte'){
   const b=k==='berichte'?window.Flottenfuehrung?.beute?.(r)?.summe:null,mond=k==='spionageberichte';
   Object.assign(e,{art:'spionage',zeichen:'spionage',koord:r.ziel,spieler:[r.besitzer].filter(n=>n&&n!==ich()),buendel:'scan:'+sysVon(r.ziel),ton:r.abgeschossen?'warn':'',
    titel:`${mond?'Mondspionage':'Spionage'} ${ortText(r.ziel)}`,unter:[r.besitzer||'unbesiedelt',r.aufklaerung?.heimat?'Heimatwelt':'',r.volk&&r.besitzer?named(r.volk):''].filter(Boolean).join(' · '),
    heimat:r.besitzer&&r.aufklaerung?.heimat?{[r.besitzer]:r.ziel}:null});
   kern(e,[b!=null?`mögliche Beute ≈ ${fmt(Math.round(b))}`:'',r.schiffe!=null?`${fmt(summe(r.schiffe))} Schiffe`:'Flotte ?',r.verteidigung!=null?`${fmt(summe(r.verteidigung))} Abwehr`:'Abwehr ?',r.abgeschossen?'Sonden abgeschossen':'']);
  }else if(k==='systemberichte'){
   const sys=sysVon(r.ziel),bes=(r.plaetze||[]).filter(p=>p.spieler);
   Object.assign(e,{art:'scans',zeichen:'system_erkunden',koord:r.ziel,system:sys,buendel:'scan:'+sys,spieler:[...new Set(bes.map(p=>p.spieler))].filter(n=>n!==ich()),titel:`Systemscan [${sys}]`,unter:bes.length?`${fmt(bes.length)} besiedelt`:'unbesiedelt'});
   kern(e,[r.asteroidenguertel?'Asteroidengürtel':'',r.nebel?'Nebel':'',(r.plaetze||[]).some(p=>p.truemmer)?'Trümmerfeld':'',(r.plaetze||[]).some(p=>p.mond)?'Monde':'']);
  }else if(k==='phalanxberichte'){
   const n=(r.kontakte||[]).length;
   Object.assign(e,{art:'scans',zeichen:'flotten_spionage',koord:r.ziel,buendel:'phalanx:'+r.ziel,spieler:[...new Set((r.kontakte||[]).map(c=>c.spieler||c.von||c.besitzer).filter(n=>n&&n!==ich()))],titel:`Phalanxscan ${ortText(r.ziel)}`,unter:r.mond?`vom Mond [${r.mond}]`:'Sensorphalanx'});
   kern(e,[`${fmt(n)} Flottenbewegung${n===1?'':'en'}`]);
  }else if(k==='flottenberichte'){
   Object.assign(e,{art:'spionage',zeichen:'flotten_spionage',feind:true,koord:r.ziel_beim_scan||'',spieler:[r.von].filter(n=>n&&n!==ich()),buendel:'flotte:'+r.flotte,
    titel:`Flottenaufklärung · Anflug auf ${ortText(r.ziel_beim_scan)||'?'}`,unter:r.von?`Flotte von ${r.von}`:'Absender verdeckt'});
   kern(e,[r.schiffe!=null?`${fmt(r.schiffe)} Schiffe`:r.abgeschirmt?'abgeschirmt':'Stärke unbekannt',r.ladung?'Ladung bekannt':'']);
  }else{
   const text=meldungText(r.text),roh=String(r.text||'');
   const art=r.art||(/Kampf|Beute|erobert|Blockade|Bombard/i.test(roh)?'kampf':/Vertrag|Allianz|Pakt/i.test(roh)?'vertrag':'');
   const fremd=/Spionage durch (.+?) (?:auf|am Mond) (\d+:\d+:\d+) bemerkt; Sonde gestartet von (\d+:\d+:\d+)/.exec(roh);
   if(fremd){
    Object.assign(e,{art:'spionage',zeichen:'spionage',feind:true,ton:'warn',koord:fremd[2],spieler:[fremd[1]],herkunft:[fremd[3]],buendel:'fremd:'+fremd[1],
     titel:`Spionage durch ${fremd[1]}`,unter:`bei ${ortText(fremd[2])} · Sonde aus [${fremd[3]}]`});
    kern(e,[/abgeschossen/.test(roh)?'Sonde abgeschossen':'nicht abgefangen']);
   }else{
    const sys=(/System (\d+:\d+)/.exec(roh)||[])[1]||'',ko=koordIn(roh),wer=nameIn(roh),diplo=['vertrag','allianz','abgelehnt'].includes(art);
    const begleit=['spionage','systemaufklaerung','flottenaufklaerung','kampf','beute'].includes(art)||diplo&&!!wer;
    Object.assign(e,{art:MELDUNG_ART[art]||'reich',zeichen:MELDUNG_ZEICHEN[art]||'',meldung:art||'meldung',koord:ko,system:sysVon(ko)||sys,begleit,feind:/deine Flotte .* untersucht/.test(roh),spieler:wer?[wer]:[],
     buendel:diplo?(wer?'diplo:'+wer:'meld:'+art):begleit?(art==='kampf'||art==='beute'?(ko?'kampf:'+ko:'meld:'+art):art==='flottenaufklaerung'?'flotte:'+((/Anflug (\d+)/.exec(roh)||[])[1]||ko):'scan:'+(sysVon(ko)||sys)):'meld:'+(art||k),
     titel:text,unter:k==='chronik'?`Chronik · ${artName(art)}`:`Meldung · ${artName(art)}`});
    kern(e,[]);
   }
  }
  return e;
 }
 // Bekannter Spielername in einem Meldungstext (längster Treffer, ganze Wörter) – für Profil-Zeitleiste und Bündel.
 let namenStempel='',namenListe=[];
 function nameIn(text){
  const k=communicationData().kontakte||[];if(k.length+':'+(k[0]?.spieler||'')!==namenStempel){namenStempel=k.length+':'+(k[0]?.spieler||'');namenListe=k.map(c=>c.spieler).filter(n=>n&&n.length>2).sort((a,b)=>b.length-a.length);}
  for(const n of namenListe){if(n===ich())continue;const i=text.indexOf(n);if(i<0)continue;const vor=text[i-1]||' ',nach=text[i+n.length]||' ';if(!/[\p{L}\p{N}]/u.test(vor)&&!/[\p{L}\p{N}]/u.test(nach))return n;}
  return '';
 }
 // Betreff aus Kernschlüsseln („nichtangriffspakt“) lesbar machen.
 const betreff=b=>/^[a-z_]+$/.test(b||'')?label(b):(b||'Nachricht');
 function ausBrief(n){
  const aus=!!n.gesendet,partner=aus?(n.an||[]):[n.von],auto=/^[a-z_]+$/.test(n.betreff||'');
  const wer=aus?(n.an||[]).map(x=>typeof relationHtml==='function'?relationHtml(x,n.an_beziehungen?.find(r=>r.spieler===x)):h(x)).join(', '):(typeof relationHtml==='function'?relationHtml(n.von,n.von_beziehung):h(n.von));
  return {id:'brief:'+n.id,quelle:'brief',kind:'brief',row:n,brief:n.id,art:'briefe',richtung:aus?'aus':'ein',t:echtAusSim(n.sekunden),sim:Number(n.sekunden),
   titel:betreff(n.betreff),unter:(aus?'an ':'von ')+partner.join(', '),unterHtml:`${aus?'an':'von'} ${wer}${n.automatisch?' · automatisch':''}`,
   vorschau:String(n.text||'').replace(/\s+/g,' ').slice(0,140),spieler:partner.filter(x=>x&&x!==ich()),koord:'',ton:'',kern:[],zeichen:'',
   // Begleitbrief einer diplomatischen Anfrage (Betreff = Vertragsart) gehört zur Anfrage.
   buendel:auto?'diplo:'+(aus?(n.an||[])[0]:n.von):'',begleit:auto};
 }
 function ausAnfrage(a){
  const anMich=a.an===ich(),offen=a.status==='offen'&&anMich,partner=anMich?a.von:a.an;
  return {id:'anfrage:'+a.id,quelle:'anfrage',kind:'anfrage',row:a,art:'briefe',richtung:anMich?'ein':'aus',t:echtAusSim(a.zeit),sim:Number(a.zeit),offen,
   titel:`${label(a.art)} · ${anMich?'von':'an'} ${partner}`,unter:`Diplomatie · ${offen?'wartet auf deine Antwort':named(a.status)}`,vorschau:String(a.text||'').slice(0,140),
   spieler:[partner].filter(Boolean),koord:'',ton:offen?'warn':'',kern:[],buendel:'diplo:'+partner,zeichen:''};
 }
 const kompakt=e=>({id:e.id,k:e.kind,t:Math.round(e.t),s:e.sim,a:e.art,m:e.zeichen,ti:e.titel,u:e.unter,ke:e.kern,to:e.ton,sp:e.spieler,ko:e.koord,f:e.feind?1:0,bu:e.buendel,he:e.herkunft,hw:e.heimat,me:e.meldung});
 const ausArchiv=c=>({id:c.id,quelle:'archiv',kind:c.k,row:null,art:c.a,zeichen:c.m||'',t:c.t,sim:c.s,titel:c.ti,unter:c.u,kern:c.ke||[],ton:c.to||'',spieler:c.sp||[],koord:c.ko||'',feind:!!c.f,buendel:c.bu||'',herkunft:c.he||[],heimat:c.hw||null,meldung:c.me});

 let cache={stempel:'',eintraege:[],spieler:new Map(),systeme:new Map()};
 function modell(){
  if(!view)return cache;
  const L=Speicher.laden();
  const stempel=[world?.world_id,ich(),view.revision,view.sekunden,(view.postfach_gelesen||[]).length,typeof mailArchive!=='undefined'?mailArchive.length:0,L.version,z.zeigeAus].join('|');
  if(stempel===cache.stempel)return cache;
  const liste=[],da=new Set();
  for(const x of typeof reportRows==='function'?reportRows():[]){const e=ausBericht(x);if(e&&!da.has(e.id)){liste.push(e);da.add(e.id);}}
  let neu=false;for(const e of liste)if(!L.archiv.has(e.id)){L.archiv.set(e.id,kompakt(e));neu=true;}
  const briefe=new Set();
  for(const n of [...(communicationData().briefkasten||[]),...(typeof mailArchive!=='undefined'?mailArchive:[])]){if(n.kanal!=='privat'||briefe.has(n.id))continue;briefe.add(n.id);liste.push(ausBrief(n));}
  for(const a of communicationData().anfragen||[])liste.push(ausAnfrage(a));
  // Archiv ergänzt nur, was der Server nicht mehr liefert. Meldungen wandern nach 150 neueren Vorfällen von „ereignisse“ in
  // die „chronik“ (andere Kennung) – gleiche Spielzeit und gleicher Text gelten als derselbe Eintrag.
  const sig=e=>(e.kind==='ereignisse'||e.kind==='chronik'?'m':e.kind)+'|'+e.sim+'|'+e.titel,live=new Set(liste.map(sig));
  for(const c of L.archiv.values()){if(da.has(c.id))continue;const e=ausArchiv(c),k=sig(e);if(live.has(k))continue;live.add(k);liste.push(e);}
  const gelesen=new Set(view.postfach_gelesen||[]);
  for(const e of liste){
   e.system=e.system||sysVon(e.koord);
   e.gelesen=e.quelle==='archiv'||e.kind==='chronik'||L.gelesen.has(e.id)||(e.quelle==='bericht'?gelesen.has(e.pid):e.quelle==='brief'?!!(e.row.gelesen||e.row.gesendet):true);
   e.markiert=L.markiert.has(e.id);e.aus=L.aus.has(e.id);
   e.suche=[e.titel,e.unter,(e.kern||[]).join(' '),(e.spieler||[]).join(' '),e.koord,e.koord?`[${e.koord}]`:'',e.system?`[${e.system}]`:'',e.vorschau||''].join(' ').toLocaleLowerCase('de');
  }
  liste.sort((a,b)=>b.t-a.t||(RANG[b.kind]||0)-(RANG[a.kind]||0));
  if(neu)Speicher.speichern();
  cache={stempel,eintraege:liste,...indexBauen(liste)};
  return cache;
 }
 function indexBauen(liste){
  const sp=new Map(),sy=new Map();
  const P=n=>{let x=sp.get(n);if(!x)sp.set(n,x={name:n,eintraege:[],letzter:0,angriffe:0,niederlagen:0,eigeneAngriffe:0,fremdSpionage:0,eigeneSpionage:0,briefe:0,ungelesen:0,anflug:0,heimat:''});return x;};
  const S=s=>{let x=sy.get(s);if(!x)sy.set(s,x={system:s,eintraege:[],letzter:0,spieler:new Set(),scan:0,ungelesen:0,angriffe:0});return x;};
  for(const e of liste){
   if(e.aus&&!z.zeigeAus)continue;
   for(const n of new Set(e.spieler||[])){
    if(!n||n===ich())continue;const x=P(n);x.eintraege.push(e);x.letzter=Math.max(x.letzter,e.t);if(!e.gelesen)x.ungelesen++;
    if(e.kind==='kampfberichte'||e.kind==='bodenberichte'){if(e.feind){x.angriffe++;if(e.ton==='schlecht')x.niederlagen++;}else x.eigeneAngriffe++;}
    else if(e.art==='spionage'&&e.kind!=='flottenberichte'){if(e.feind)x.fremdSpionage++;else x.eigeneSpionage++;}
    else if(e.kind==='brief')x.briefe++;
    if(e.heimat?.[n]&&!x.heimat)x.heimat=e.heimat[n];
   }
   if(e.system){const s=S(e.system);s.eintraege.push(e);s.letzter=Math.max(s.letzter,e.t);for(const n of e.spieler||[])if(n!==ich())s.spieler.add(n);if(e.kind==='systemberichte')s.scan=Math.max(s.scan,e.t);if(!e.gelesen)s.ungelesen++;if(e.feind&&e.art==='kampf')s.angriffe++;}
  }
  for(const a of view?.angriffe||[])if(a.von&&a.von!==ich()){const x=P(a.von);x.anflug++;x.letzter=Math.max(x.letzter,jetzt());}
  return {spieler:sp,systeme:sy};
 }
 const drohung=x=>x.anflug*5+x.angriffe*2+x.fremdSpionage;
 const finde=id=>modell().eintraege.find(e=>e.id===id)||null;

 // ---------------------------------------------------------------- Ansichtszustand und Filter
 const ARTEN=[['alle','Alle'],['kampf','Kampf'],['spionage','Spionage'],['scans','Scans'],['briefe','Briefe'],['reich','Reich']];
 const MENGE=40;
 const z={ansicht:'chronik',art:'alle',unter:'alle',spieler:'',system:'',text:'',ungelesen:false,markiert:false,zeigeAus:false,menge:MENGE,sortierung:'zuletzt'};
 const filterAktiv=()=>z.art!=='alle'||!!z.spieler||!!z.system||!!z.text||z.ungelesen||z.markiert||z.ansicht!=='chronik';
 function filterLoeschen(){Object.assign(z,{ansicht:'chronik',art:'alle',unter:'alle',spieler:'',system:'',text:'',ungelesen:false,markiert:false,menge:MENGE});const s=$('mail-search');if(s)s.value='';ordnerSetzen();}
 function passt(e){
  if(e.aus&&!z.zeigeAus)return false;
  if(z.art!=='alle'&&e.art!==z.art)return false;
  if(z.art==='briefe'&&z.unter==='eingang'&&!(e.kind!=='anfrage'&&e.richtung==='ein'))return false;
  if(z.art==='briefe'&&z.unter==='gesendet'&&!(e.kind==='brief'&&e.richtung==='aus'))return false;
  if(z.spieler&&!(e.spieler||[]).includes(z.spieler))return false;
  if(z.system&&e.system!==z.system)return false;
  if(z.ungelesen&&e.gelesen)return false;
  if(z.markiert&&!e.markiert)return false;
  if(z.text&&!e.suche.includes(z.text.trim().toLocaleLowerCase('de')))return false;
  return true;
 }
 // Gleiche Art im selben Ort innerhalb von 20 Minuten Echtzeit = ein Bündel (Systemscan + Planetensonden, Kampf + Meldungen).
 function buendeln(liste){
  const aus=[],offen=new Map();
  for(const e of liste){
   const b=e.buendel&&offen.get(e.buendel);
   // Fenster ab dem neuesten Eintrag des Bündels (nicht gleitend) – sonst wüchse ein stündlich ausgespähtes System zu einem Riesenbündel.
   if(b&&b.t-e.t<=1200){b.glieder.push(e);b.letzter=e.t;continue;}
   const neu={typ:'einzel',t:e.t,glieder:[e],key:e.buendel,letzter:e.t};aus.push(neu);if(e.buendel)offen.set(e.buendel,neu);
  }
  for(const b of aus){
   if(b.glieder.length<2)continue;
   // Ein Bericht mit seinen Begleitmeldungen bleibt EINE Zeile (ein Tipp öffnet den Bericht, die Meldungen stehen darin).
   const haupt=b.glieder.filter(e=>!e.begleit);
   if(haupt.length===1){b.mit=b.glieder.filter(e=>e!==haupt[0]);b.glieder=haupt;continue;}
   b.typ='buendel';b.kopf=[...b.glieder].sort((x,y)=>(RANG[y.kind]||0)-(RANG[x.kind]||0)||y.t-x.t)[0];
  }
  return aus;
 }
 // Begleitmeldungen eines Berichts: gleiche Bündelkennung im 20-Minuten-Fenster.
 const platzGenau=e=>Number(String(e.koord||'').split(':')[2])>0;
 const begleitVon=(e,liste)=>e.buendel&&!e.begleit?liste.filter(x=>x.begleit&&x.buendel===e.buendel&&Math.abs(x.t-e.t)<=1200&&(!x.aus||z.zeigeAus)&&(!platzGenau(e)||!x.koord||x.koord===e.koord)):[];
 function ordnerSetzen(){if(typeof mailFolder==='undefined')return;mailFolder=z.art==='briefe'?(['eingang','gesendet','anfragen'].includes(z.unter)?z.unter:'alle'):'alle';letzterOrdner=mailFolder;}
 let letzterOrdner='alle';
 // Ordnerknöpfe des Briefkastens (auch per Skript, z. B. „Kontakt“ in der Galaxietabelle) wählen die passende Sicht.
 function ordner(f){
  if(f===letzterOrdner)return;letzterOrdner=f;
  if(['eingang','gesendet','anfragen'].includes(f))Object.assign(z,{ansicht:'chronik',art:'briefe',unter:f,menge:MENGE});
  else if(f==='berichte')Object.assign(z,{ansicht:'chronik',art:'alle',menge:MENGE});
 }

 // ---------------------------------------------------------------- Zeichnen der Zentrale
 const SVG_BRIEF='<svg viewBox="0 0 24 24" width="18" height="18" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true" focusable="false"><path d="M3 6h18v12H3z"/><path d="m3 6 9 7 9-7"/></svg>';
 const SVG_DIPLO='<svg viewBox="0 0 24 24" width="18" height="18" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true" focusable="false"><path d="M12 3 4 6v6c0 5 8 9 8 9s8-4 8-9V6Z"/><path d="m9 12 2 2 4-4"/></svg>';
 const SVG_MELDUNG='<svg viewBox="0 0 24 24" width="18" height="18" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" aria-hidden="true" focusable="false"><circle cx="12" cy="12" r="8"/><path d="M12 8v5M12 16h.01"/></svg>';
 function zeichen(e,groesse=28){
  if(e.kind==='brief')return `<span class="bz-zeichen bz-zeichen-brief${e.richtung==='aus'?' aus':''}">${SVG_BRIEF}</span>`;
  if(e.kind==='anfrage')return `<span class="bz-zeichen bz-zeichen-diplo">${SVG_DIPLO}</span>`;
  if(e.zeichen&&window.Auftraege)return `<span class="bz-zeichen">${Auftraege.badge(e.zeichen,{size:groesse})}</span>`;
  return `<span class="bz-zeichen bz-zeichen-meldung">${SVG_MELDUNG}</span>`;
 }
 function unterzeile(e){
  const teile=[e.unterHtml||mitKoord(e.unter||''),...(e.kern||[]).map(k=>`<b>${h(k)}</b>`)];
  return teile.filter(Boolean).join('<i aria-hidden="true"> · </i>');
 }
 function zeileHTML(e,mit=null){
  const attr=e.kind==='brief'?`data-open-letter="${h(e.brief)}"`:`data-bz-oeffnen="${h(e.id)}"`,dazu=Array.isArray(mit)?mit:[];
  const gelesen=e.gelesen&&dazu.every(x=>x.gelesen);
  const nurMeldungen=dazu.every(x=>x.kind==='ereignisse'||x.kind==='chronik');
  const extra=dazu.length?`<i aria-hidden="true"> · </i><span>+${dazu.length} ${nurMeldungen?(dazu.length>1?'Meldungen':'Meldung'):'weitere'}</span>`:'';
  return `<li class="bz-zeile${gelesen?'':' neu'}${e.ton?' ton-'+e.ton:''}${e.feind?' feind':''}${e.markiert?' markiert':''}${e.aus?' versteckt':''}" id="bzr-${kurz(e.id)}"><button type="button" class="bz-k bz-zeile-knopf" ${attr}>${zeichen(e)}<span class="bz-zeile-text"><span class="bz-zeile-titel">${mitKoord(e.titel)}</span><span class="bz-zeile-unter">${unterzeile(e)}${extra}</span>${e.vorschau&&(e.kind==='brief'||e.kind==='anfrage')?`<span class="bz-vorschau">${h(e.vorschau)}</span>`:''}</span><span class="bz-zeile-zeit"><time>${uhr(e.t)}</time>${gelesen?'':'<i class="bz-punkt" aria-hidden="true"></i><span class="bz-sr">ungelesen</span>'}${e.markiert?'<span class="bz-merk">Markiert</span>':''}${e.quelle==='archiv'?'<span class="bz-archiv-marke">Archiv</span>':''}</span></button></li>`;
 }
 const zeilen=liste=>liste.map(e=>zeileHTML(e)).join('');
 function buendelTitel(b){
  const k=b.kopf,n=b.glieder.length,art=b.key.split(':')[0];
  if(art==='scan'){const c=t=>b.glieder.filter(t).length,sc=c(e=>e.kind==='systemberichte'),pb=b.glieder.filter(e=>e.kind==='berichte'||e.kind==='spionageberichte'),pd=new Set(pb.map(e=>e.koord)).size,me=c(e=>e.begleit);
   return {titel:`Aufklärung System [${b.key.slice(5)}]`,unter:[sc?`${sc} Systemscan${sc>1?'s':''}`:'',pd?`${pd} Planet${pd>1?'en':''}${pb.length>pd?` (${pb.length} Berichte)`:''}`:'',me?`${me} Meldung${me>1?'en':''}`:''].filter(Boolean).join(' · '),zeichen:sc?'system_erkunden':'spionage'};}
  if(art==='boden')return {titel:k.titel,unter:`${n} Bodenrunden · ${k.unter}`,zeichen:'invasion'};
  if(art==='fremd')return {titel:k.titel,unter:k.unter,zeichen:'spionage'};
  if(art==='meld')return {titel:`${artName(b.key.slice(5))} · ${n} Meldungen`,unter:k.titel,zeichen:k.zeichen};
  return {titel:k.titel,unter:k.unter,zeichen:k.zeichen};
 }
 function buendelHTML(b){
  const k=b.kopf,ti=buendelTitel(b),n=b.glieder.length,neu=b.glieder.filter(e=>!e.gelesen).length,id='bzb-'+hash(b.key+'|'+b.glieder.at(-1).id);
  const kernZeigen=/^(kampf|boden|fremd|flotte):/.test(b.key)&&k.kern?.length;
  return `<li class="bz-zeile bz-buendel${neu?' neu':''}${k.ton?' ton-'+k.ton:''}${k.feind?' feind':''}" id="${id}"><details><summary class="bz-zeile-knopf" aria-label="${h(ti.titel)}, ${n} Einträge${neu?`, ${neu} ungelesen`:''} – aufklappen">${zeichen({...k,zeichen:ti.zeichen})}<span class="bz-zeile-text"><span class="bz-zeile-titel">${mitKoord(ti.titel)}<span class="bz-anzahl" aria-hidden="true">${n}</span></span><span class="bz-zeile-unter">${mitKoord(ti.unter)}${kernZeigen?k.kern.map(x=>`<i aria-hidden="true"> · </i><b>${h(x)}</b>`).join(''):''}${neu?`<i aria-hidden="true"> · </i><b class="bz-neu-text">${neu} neu</b>`:''}</span></span><span class="bz-zeile-zeit"><time>${uhr(b.t)}</time>${neu?'<i class="bz-punkt" aria-hidden="true"></i>':''}<span class="bz-klappe" aria-hidden="true"></span></span></summary><ol class="bz-glieder">${zeilen(b.glieder)}</ol></details></li>`;
 }
 function chronikHTML(m){
  const liste=m.eintraege.filter(passt),posten=buendeln(liste);
  if(!posten.length){
   if(filterAktiv())return `<div class="bz-leer"><p>Keine Einträge für diese Auswahl.</p><button type="button" class="bz-k bz-knopf" data-bz-filter-weg="alle">Filter löschen</button></div>`;
   return `<div class="bz-leer"><p>Noch keine Berichte oder Briefe. Erkunde ein System – Scans, Spionage, Kämpfe und Post erscheinen hier in zeitlicher Reihenfolge.</p><button type="button" class="bz-k bz-knopf" data-tab="karte">Zur Galaxie</button></div>`;
  }
  let html='',tag=null;
  for(const p of posten.slice(0,z.menge)){
   const n=tagNr(p.t);
   if(n!==tag){if(tag!==null)html+='</ol></section>';tag=n;const d=new Date(p.t*1000);html+=`<section class="bz-tag" id="bzt-${n}"><h3 class="bz-tag-kopf"><span>${h(tagName(p.t))}</span><small>${zwei(d.getDate())}.${zwei(d.getMonth()+1)}.${d.getFullYear()}</small></h3><ol class="bz-liste">`;}
   html+=p.typ==='buendel'?buendelHTML(p):zeileHTML(p.glieder[0],p.mit);
  }
  if(tag!==null)html+='</ol></section>';
  if(posten.length>z.menge)html+=`<button type="button" class="bz-k bz-knopf bz-mehr" data-bz-mehr>Ältere zeigen · noch ${fmt(posten.length-z.menge)}</button>`;
  return html;
 }
 function personZeile(x){
  const r=typeof relationFor==='function'?relationFor(x.name,null):null,heimat=x.heimat||spielerHeimat(x.name);
  const drohungen=[x.anflug?`<b class="bz-rot">Anflug läuft · ${x.anflug}</b>`:'',x.angriffe?`<span class="bz-rot">${x.angriffe} Angriff${x.angriffe>1?'e':''} auf dich</span>`:'',x.fremdSpionage?`<span class="bz-warn">${x.fremdSpionage}× spioniert</span>`:'',x.eigeneAngriffe?`<span>${x.eigeneAngriffe} eigene Angriff${x.eigeneAngriffe>1?'e':''}</span>`:'',x.eigeneSpionage?`<span>${x.eigeneSpionage} Scans</span>`:'',x.briefe?`<span>${x.briefe} Brief${x.briefe>1?'e':''}</span>`:''].filter(Boolean);
  return `<li class="bz-person${drohung(x)?' bedroht':''}" id="bzp-${kurz(hash(x.name))}"><button type="button" class="bz-k bz-person-knopf" data-sp-profil="${h(x.name)}"><span class="bz-avatar${r?.status?' rel-'+h(r.status):''}" aria-hidden="true">${h([...x.name][0]||'?')}</span><span class="bz-zeile-text"><span class="bz-zeile-titel">${typeof relationHtml==='function'?relationHtml(x.name,r):h(x.name)}${x.ungelesen?`<span class="bz-neu-zahl">${x.ungelesen} neu</span>`:''}</span><span class="bz-zeile-unter">${heimat?`Heimat <span class="koord">[${h(heimat)}]</span><i aria-hidden="true"> · </i>`:''}${fmt(x.eintraege.length)} Einträge · ${x.letzter?alter(x.letzter):'–'}</span>${drohungen.length?`<span class="bz-drohung">${drohungen.join('')}</span>`:''}</span></button><button type="button" class="bz-k bz-mini" data-bz-filter-spieler="${h(x.name)}" aria-label="Chronik mit ${h(x.name)}">Chronik</button></li>`;
 }
 function personenHTML(m){
  const q=z.text.trim().toLocaleLowerCase('de');
  const liste=[...m.spieler.values()].filter(x=>!q||x.name.toLocaleLowerCase('de').includes(q)).sort((a,b)=>z.sortierung==='drohung'?drohung(b)-drohung(a)||b.letzter-a.letzter:b.letzter-a.letzter);
  if(!liste.length)return `<div class="bz-leer"><p>${q?'Kein Spieler mit diesem Namen in deinen Berichten.':'Noch kein Kontakt zu anderen Reichen. Spionage, Kämpfe und Briefe füllen diese Liste.'}</p></div>`;
  return `<div class="bz-sortierung" role="group" aria-label="Sortieren"><button type="button" class="bz-k bz-seg" data-bz-sort="zuletzt" aria-pressed="${z.sortierung!=='drohung'}">Zuletzt</button><button type="button" class="bz-k bz-seg" data-bz-sort="drohung" aria-pressed="${z.sortierung==='drohung'}">Bedrohung</button></div><ol class="bz-liste bz-personen">${liste.slice(0,z.menge).map(personZeile).join('')}</ol>${liste.length>z.menge?`<button type="button" class="bz-k bz-knopf bz-mehr" data-bz-mehr>Weitere zeigen · noch ${fmt(liste.length-z.menge)}</button>`:''}`;
 }
 function systemZeile(s){
  const tf=(view?.truemmer||[]).filter(t=>sysVon(t.koord)===s.system),namen=[...s.spieler];
  return `<li class="bz-person bz-system" id="bzs-${kurz(s.system)}"><button type="button" class="bz-k bz-person-knopf" data-sy-profil="${h(s.system)}"><span class="bz-avatar bz-avatar-system" aria-hidden="true">${h(s.system.split(':')[1]||'')}</span><span class="bz-zeile-text"><span class="bz-zeile-titel"><span class="koord">[${h(s.system)}]</span>${s.ungelesen?`<span class="bz-neu-zahl">${s.ungelesen} neu</span>`:''}</span><span class="bz-zeile-unter">${namen.length?h(namen.slice(0,4).join(', '))+(namen.length>4?` +${namen.length-4}`:'')+'<i aria-hidden="true"> · </i>':''}${fmt(s.eintraege.length)} Einträge · ${alter(s.letzter)}</span><span class="bz-drohung">${s.scan?`<span>Scan ${alter(s.scan)}</span>`:'<span>nicht kartiert</span>'}${tf.length?`<span class="bz-tf">Trümmerfeld${tf.length>1?' ×'+tf.length:''}</span>`:''}${s.angriffe?`<span class="bz-rot">${s.angriffe} Angriff${s.angriffe>1?'e':''} auf dich</span>`:''}</span></span></button><button type="button" class="bz-k bz-mini" data-bz-filter-system="${h(s.system)}" aria-label="Chronik für System ${h(s.system)}">Chronik</button></li>`;
 }
 function systemeHTML(m){
  const q=z.text.trim().replace(/[[\]\s]/g,'');
  const liste=[...m.systeme.values()].filter(s=>!q||s.system.startsWith(q)||`${s.system}`.includes(q)||[...s.spieler].some(n=>n.toLocaleLowerCase('de').includes(q.toLocaleLowerCase('de')))).sort((a,b)=>b.letzter-a.letzter);
  if(!liste.length)return `<div class="bz-leer"><p>${q?'Kein System passt zur Suche.':'Noch keine Berichte aus fremden Systemen.'}</p><button type="button" class="bz-k bz-knopf" data-tab="karte">Zur Galaxie</button></div>`;
  return `<ol class="bz-liste bz-personen">${liste.slice(0,z.menge).map(systemZeile).join('')}</ol>${liste.length>z.menge?`<button type="button" class="bz-k bz-knopf bz-mehr" data-bz-mehr>Weitere zeigen · noch ${fmt(liste.length-z.menge)}</button>`:''}`;
 }
 function spielerHeimat(name){
  for(const n of view?.nachbarn||[])if(n.spieler===name&&n.heimat)return n.koord;
  for(const b of view?.berichte||[])if(b.besitzer===name&&b.aufklaerung?.heimat)return b.ziel;
  for(const k of view?.kampfberichte||[])for(const x of k.herkunft||[])if(x.spieler===name&&x.heimat)return x.heimat;
  return '';
 }
 // Beim Tippen in der Chronik: passende Spieler und ein getipptes System als Filter anbieten.
 function vorschlaegeHTML(m){
  const q=z.text.trim().toLocaleLowerCase('de');
  if(z.ansicht!=='chronik'||q.length<2)return '';
  const k=/^\[?(\d+):(\d+)(?::(\d+))?\]?$/.exec(q),treffer=[];
  if(k)treffer.push(`<button type="button" class="bz-k bz-chip" data-bz-filter-system="${h(k[1]+':'+k[2])}">System <span class="koord">[${h(k[1]+':'+k[2])}]</span> filtern</button>`);
  for(const x of [...m.spieler.values()].filter(x=>x.name.toLocaleLowerCase('de').includes(q)).sort((a,b)=>b.letzter-a.letzter).slice(0,3))treffer.push(`<button type="button" class="bz-k bz-chip" data-bz-filter-spieler="${h(x.name)}">Spieler ${h(x.name)} filtern</button>`);
  return treffer.length?`<div class="bz-vorschlag-liste">${treffer.join('')}</div>`:'';
 }
 const REITER_SVG={alle:'<path d="M4 6h16M4 12h16M4 18h10"/>',briefe:'<path d="M3 6h18v12H3z"/><path d="m3 6 9 7 9-7"/>',reich:'<path d="M4 20h16M6 20V10m4 10V10m4 10V10m4 10V10M3 10l9-6 9 6"/>',
  spieler:'<circle cx="12" cy="8" r="3.5"/><path d="M5 20a7 7 0 0 1 14 0"/>',systeme:'<circle cx="12" cy="12" r="2.5"/><circle cx="12" cy="12" r="6.5"/><circle cx="12" cy="12" r="10" stroke-dasharray="2 3"/>'};
 const REITER_AUFTRAG={kampf:'angriff',spionage:'spionage',scans:'system_erkunden'};
 function reiterZeichen(k){
  if(REITER_AUFTRAG[k]&&window.Auftraege)return Auftraege.svg(REITER_AUFTRAG[k],20);
  return `<svg class="auftrag-svg" viewBox="0 0 24 24" width="20" height="20" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true" focusable="false">${REITER_SVG[k]||REITER_SVG.alle}</svg>`;
 }
 function kopfHTML(m){
  const ung=a=>m.eintraege.filter(e=>!e.gelesen&&!e.aus&&(a==='alle'||e.art===a)).length;
  const sicht=m.eintraege.filter(passt),neu=sicht.filter(e=>!e.gelesen).length,aus=m.eintraege.filter(e=>e.aus).length;
  const weg=(art,filter,inhalt)=>filter?`<button type="button" class="bz-k bz-chip aktiv" data-bz-filter-weg="${art}" aria-label="Filter ${art==='system'?'System':art==='text'?'Suche':'Spieler'} ${h(filter)} entfernen">${inhalt} <span aria-hidden="true">×</span></button>`:'';
  const reiter=(k,t,attr,an,n)=>`<button type="button" class="bz-k bz-reiter bz-reiter-${k}" ${attr} aria-pressed="${an}"${n?` aria-label="${t}, ${n} ungelesen"`:''}><span class="bz-reiter-zeichen">${reiterZeichen(k)}</span><span class="bz-reiter-name">${t}</span>${n?`<b class="bz-zahl" aria-hidden="true">${n>99?'99+':n}</b>`:''}</button>`;
  return {
   // Acht gleich große Reiter: sechs Arten der Chronik und die Blickwinkel „nach Spieler“ und „nach System“.
   arten:ARTEN.map(([k,t])=>reiter(k,t,`data-bz-art="${k}"`,z.ansicht==='chronik'&&z.art===k,ung(k))).join('')+reiter('spieler','Spieler','data-bz-ansicht="spieler"',z.ansicht==='spieler',0)+reiter('systeme','Systeme','data-bz-ansicht="systeme"',z.ansicht==='systeme',0),
   wahl:z.ansicht!=='chronik'?'':weg('spieler',z.spieler,`Spieler: ${h(z.spieler)}`)+weg('system',z.system,`System <span class="koord">[${h(z.system)}]</span>`),
   filter:z.ansicht!=='chronik'?'':[
    `<button type="button" class="bz-k bz-chip" data-bz-ungelesen aria-pressed="${z.ungelesen}">Ungelesen</button>`,
    `<button type="button" class="bz-k bz-chip" data-bz-markiert aria-pressed="${z.markiert}">Markiert</button>`,
    neu?`<button type="button" class="bz-k bz-chip bz-gelesen-alle" data-bz-alle-gelesen>Alle ${fmt(neu)} gelesen</button>`:''
   ].join(''),
   zaehler:z.ansicht==='chronik'?(!filterAktiv()&&!aus?'':`${filterAktiv()?`${fmt(sicht.length)} von ${fmt(m.eintraege.filter(e=>!e.aus||z.zeigeAus).length)} Einträgen`:`${fmt(sicht.length)} Einträge`}${aus?` · <button type="button" class="bz-link" data-bz-zeige-aus aria-pressed="${z.zeigeAus}">${z.zeigeAus?'Ausgeblendete verbergen':`${fmt(aus)} ausgeblendet`}</button>`:''}`)
    :z.ansicht==='spieler'?`${fmt(m.spieler.size)} Spieler aus deinen Berichten und Briefen · Tippen öffnet das Profil`:`${fmt(m.systeme.size)} Systeme mit Berichten · Tippen öffnet das System`,
   ungelesen:ung('alle')
  };
 }

 // ---------------------------------------------------------------- Aufbau der Seite (einmal) und Zeichnen
 let montiert=false,schmutzig=true;
 const seiteSichtbar=()=>!!$('briefkasten')&&!$('briefkasten').hidden;
 function montieren(){
  if(montiert)return;const seite=$('briefkasten');if(!seite||!$('messages'))return;montiert=true;
  seite.classList.add('bz-zentrale');
  const kopf=seite.querySelector('.comm-heading');
  if(kopf){const eb=kopf.querySelector('.eyebrow'),h2=kopf.querySelector('h2'),p=kopf.querySelector('p');if(eb){eb.id='bz-seitenzeile';eb.textContent='Funk';}if(h2)h2.textContent='Nachrichten';if(p)p.textContent='Briefe, Kampf-, Spionage- und Scanberichte in einer Chronik – nach Spieler und System geordnet.';const b=kopf.querySelector('[data-compose="privat"]');if(b){b.classList.add('bz-k','bz-schreiben');b.innerHTML='<span aria-hidden="true">＋</span> Brief';b.setAttribute('aria-label','Brief schreiben');}}
  const leiste=document.createElement('div');leiste.id='bz-kopf';leiste.className='bz-kopf';
  leiste.innerHTML=`<div class="bz-arten" id="bz-arten" role="group" aria-label="Art und Blickwinkel"></div><div class="bz-unter" id="bz-unter"></div><div class="bz-suchreihe" id="bz-suchreihe"><div class="bz-suche" id="bz-suche"></div></div><div class="bz-vorschlaege" id="bz-vorschlaege" hidden></div><div class="bz-wahl" id="bz-wahl" role="group" aria-label="Aktive Filter"></div><div class="bz-filter" id="bz-filter" role="group" aria-label="Filter"></div><p class="bz-zaehler" id="bz-zaehler" role="status"></p>`;
  if(kopf)kopf.after(leiste);else seite.prepend(leiste);
  // „Brief schreiben“ neben die Suche: eine Zeile weniger am Handy (der Seitentitel steht dort in der Leiste „Post“).
  const schreiben=kopf?.querySelector('[data-compose="privat"]');if(schreiben)$('bz-suchreihe').append(schreiben);
  // Ordner des alten Briefkastens werden zur Unterwahl „Briefe“; „Berichte & Meldungen“ ist jetzt die Zentrale selbst.
  const tabs=seite.querySelector('.comm-tabs');
  if(tabs){
   $('bz-unter').append(tabs);tabs.setAttribute('aria-label','Briefe');
   tabs.insertAdjacentHTML('afterbegin','<button type="button" class="secondary" data-bz-unter="alle" aria-pressed="true">Alle</button>');
   const text=(f,t)=>{const b=tabs.querySelector(`[data-mail-folder="${f}"]`);if(b&&b.firstChild?.nodeType===3)b.firstChild.nodeValue=t+' ';};
   text('eingang','Eingang');text('anfragen','Diplomatie');
   const alt=tabs.querySelector('[data-mail-folder="berichte"]');if(alt)alt.hidden=true;
  }
  const such=seite.querySelector('label.mail-search');
  if(such){$('bz-suche').append(such);const i=such.querySelector('input');if(i){i.placeholder='Spieler, [S:SSS] oder Text';i.setAttribute('autocapitalize','none');i.setAttribute('spellcheck','false');i.setAttribute('autocomplete','off');}const sr=such.querySelector('.sr-only');if(sr)sr.textContent='Nachrichten und Berichte durchsuchen';}
  seite.querySelector('#mail-list-pane .comm-section-heading')?.setAttribute('hidden','');
  const alt=$('mail-reports-pane');if(alt)alt.hidden=true;
  $('messages').classList.add('bz-liste-host');
  Blatt.knoten();
 }
 // Läuft innerhalb von renderView: ein Fehler hier darf nie die Abfrage abbrechen (sonst „Verbindung unterbrochen“).
 function render(){try{zeichneSeite();}catch(err){console.warn('Funk-Zentrale',err);}}
 function zeichneSeite(){
  if(!view)return;
  montieren();lesestandAbgleichen();
  if(Blatt.offen())Blatt.zeichnen(false);
  if(!montiert)return;
  const seite=$('briefkasten');
  // Bereichsteile des Briefkastens folgen der Wahl (auch wenn die Seite gerade verdeckt ist – sonst blitzen sie beim Öffnen auf).
  const briefe=z.ansicht==='chronik'&&z.art==='briefe',anfragen=briefe&&z.unter==='anfragen';
  seite.classList.toggle('bz-briefe',briefe);
  $('mail-list-pane').hidden=anfragen;$('mail-requests-pane').hidden=!anfragen;
  if($('mail-reports-pane'))$('mail-reports-pane').hidden=true;
  const ally=seite.querySelector('.ally-picker');if(ally)ally.hidden=!briefe;
  if($('older-letters'))$('older-letters').hidden=!briefe||anfragen;
  $('bz-unter').hidden=!briefe;$('bz-filter').hidden=z.ansicht!=='chronik'||anfragen;$('bz-suchreihe').hidden=anfragen;
  for(const b of seite.querySelectorAll('#bz-unter [data-mail-folder],#bz-unter [data-bz-unter]'))b.setAttribute('aria-pressed',String((b.dataset.mailFolder||b.dataset.bzUnter)===z.unter));
  if(!seiteSichtbar()){schmutzig=true;return;}
  schmutzig=false;
  const m=modell(),k=kopfHTML(m);
  setStableHTML($('bz-arten'),k.arten);setStableHTML($('bz-wahl'),k.wahl);$('bz-wahl').hidden=!k.wahl;setStableHTML($('bz-filter'),k.filter);setStableHTML($('bz-zaehler'),k.zaehler);$('bz-zaehler').hidden=!k.zaehler||anfragen;
  const zeile=$('bz-seitenzeile');if(zeile){const t=k.ungelesen?`Funk · ${fmt(k.ungelesen)} ungelesen`:'Funk';if(zeile.textContent!==t)zeile.textContent=t;}
  const v=vorschlaegeHTML(m);$('bz-vorschlaege').hidden=!v;setStableHTML($('bz-vorschlaege'),v);
  const s=$('mail-search');if(s){if(document.activeElement!==s&&s.value!==z.text)s.value=z.text;const ph=z.ansicht==='spieler'?'Spieler suchen':z.ansicht==='systeme'?'System [S:SSS] oder Spieler':'Spieler, [S:SSS] oder Text';if(s.placeholder!==ph)s.placeholder=ph;}
  if(anfragen)return;
  setStableHTML($('messages'),z.ansicht==='spieler'?personenHTML(m):z.ansicht==='systeme'?systemeHTML(m):chronikHTML(m));
 }
 function neuZeichnen(){z.menge=MENGE;render();}

 // ---------------------------------------------------------------- Blatt (Detail, Profile) – nicht modal, ein Stapel
 const KARTEN={};
 const Blatt=(()=>{
  let el=null;const stapel=[];
  function knoten(){
   if(el)return el;
   el=document.createElement('div');el.id='bz-blatt';el.className='bz-blatt';el.hidden=true;
   el.innerHTML=`<div class="bz-schleier" data-bz-blatt-zu aria-hidden="true"></div><section class="bz-blatt-flaeche" role="dialog" aria-modal="false" aria-labelledby="bz-blatt-titel"><div class="bz-blatt-kopf"><button type="button" class="bz-k bz-rund" data-bz-blatt-zurueck aria-label="Zurück zur vorigen Karte" hidden><span aria-hidden="true">‹</span></button><div class="bz-blatt-titel"><p class="bz-augenbraue" id="bz-blatt-augenbraue"></p><h2 id="bz-blatt-titel" tabindex="-1"></h2></div><button type="button" class="bz-k bz-rund" data-bz-blatt-zu aria-label="Schließen"><span aria-hidden="true">×</span></button></div><div class="bz-blatt-rumpf" id="bz-blatt-rumpf"><div class="bz-blatt-meta" id="bz-blatt-meta"></div><div class="bz-blatt-aktionen" id="bz-blatt-aktionen"></div><p class="bz-blatt-fehler" id="bz-blatt-fehler" role="alert" hidden></p><div class="bz-karte" id="bz-karte"></div><div class="bz-blatt-fuss" id="bz-blatt-fuss"></div></div></section>`;
   document.body.append(el);
   // Berichtsdossiers (Mobile035.report) sind in dossier.css auf #reports gestaltet – der Container zieht ins Blatt um.
   let rep=$('reports');if(!rep){rep=document.createElement('div');rep.id='reports';}
   rep.classList.add('bz-dossier');$('bz-karte').after(rep);rep.replaceChildren();rep.hidden=true;
   return el;
  }
  const offen=()=>!!el&&!el.hidden;
  const rumpf=()=>$('bz-blatt-rumpf');
  function oeffnen(typ,schluessel){
   knoten();if(!offen())stapel.length=0;
   const top=stapel.at(-1);if(top&&top.typ===typ&&top.schluessel===schluessel){zeichnen(false);return;}
   if(top)top.scroll=rumpf().scrollTop;
   stapel.push({typ,schluessel,scroll:0});
   el.hidden=false;document.body.classList.add('bz-blatt-offen');$('bericht-sofort')?.setAttribute('hidden','');
   zeichnen(true);rumpf().scrollTop=0;
   $('bz-blatt-titel')?.focus({preventScroll:true});
  }
  function zurueck(){if(stapel.length>1){stapel.pop();zeichnen(true);const s=stapel.at(-1).scroll||0;setTimeout(()=>{rumpf().scrollTop=s;},0);return true;}schliessen();return true;}
  function schliessen(){stapel.length=0;if(el)el.hidden=true;document.body.classList.remove('bz-blatt-offen');}
  function zeichnen(neu){
   const top=stapel.at(-1);if(!top||!el)return;
   let k=null;try{k=KARTEN[top.typ]?.(top.schluessel)||null;}catch(e){console.warn('Funk-Karte',e);}
   if(!k){if(stapel.length>1){stapel.pop();zeichnen(true);}else schliessen();return;}
   $('bz-blatt-augenbraue').textContent=k.augenbraue||'';$('bz-blatt-titel').textContent=k.titel||'';
   el.dataset.typ=top.typ;el.dataset.ton=k.ton||'';el.dataset.art=k.art||'';el.querySelector('[data-bz-blatt-zurueck]').hidden=stapel.length<2;
   setStableHTML($('bz-blatt-fuss'),k.fuss||'');
   setStableHTML($('bz-blatt-meta'),k.meta||'');setStableHTML($('bz-blatt-aktionen'),k.aktionen||'');
   const rep=$('reports'),karte=$('bz-karte'),ziel=k.dossier?rep:karte,leer=k.dossier?karte:rep;
   leer.hidden=true;if(neu){leer.replaceChildren();ziel.replaceChildren();}ziel.hidden=false;setStableHTML(ziel,k.inhalt||'');
   if(neu)fehler('');
  }
  return {knoten,oeffnen,zurueck,schliessen,zeichnen,offen,oben:()=>stapel.at(-1)||null};
 })();
 function fehler(text){const f=$('bz-blatt-fehler');if(!f)return;f.textContent=text||'';f.hidden=!text;}

 // Folgehandlungen (R10): Versand immer über die Einsatzplanung (planen), Ein-Tipp-Aufträge (Ausspähen, Systemsonde) direkt.
 const busy=new Set();
 const knopf=(attr,text,{zeichen:z2='',haupt=false,key=''}={})=>`<button type="button" class="bz-k bz-aktion${haupt?' haupt':''}" ${attr}${busy.has(key||attr)?' disabled aria-busy="true"':''}>${z2&&window.Auftraege?Auftraege.badge(z2,{size:18}):''}<span>${text}</span></button>`;
 const plan=(k,auftrag,text,haupt,zielart='planet')=>knopf(`data-bz-plan="${h(k)}" data-bz-auftrag="${h(auftrag)}" data-bz-zielart="${zielart}"`,text,{zeichen:auftrag,haupt});
 const spion=(k,text='Ausspähen')=>{let n=1;try{n=Math.max(1,Math.min(1000,Number(localStorage.getItem('sternenepoche.spionage.sonden'))||1));}catch(_){}return knopf(`data-bz-spion="${h(k)}"`,`${text} · ${n} Sonde${n>1?'n':''}`,{zeichen:'spionage',key:'spion:'+k});};
 const profilKnopf=n=>knopf(`data-sp-profil="${h(n)}"`,`Profil ${h(n)}`);
 const imSystemKnopf=k=>knopf(`data-bz-karte="${h(k)}"`,'Im System zeigen');
 const eigenerPlanet=k=>(view?.planeten||[]).some(p=>p.koord===k);
 function aktionen(e){
  const a=[],k=e.koord,r=e.row||{},fremdPlanet=k&&!eigenerPlanet(k)&&Number(k.split(':')[2])>0;
  if(e.kind==='kampfberichte'){
   if(e.feind){for(const s of (e.herkunft||[]).slice(0,1)){a.push(spion(s,'Herkunft ausspähen'));a.push(plan(s,'angriff','Gegenangriff planen'));}}
   else if(fremdPlanet){a.push(plan(k,'angriff','Erneut angreifen',true));a.push(spion(k));}
   if(e.truemmer>0||r.ueberlebende)a.push(plan(k,'recyceln','Recycler schicken',!e.feind&&!fremdPlanet));
  }else if(e.kind==='berichte'&&r.art!=='erkundung'||e.kind==='spionageberichte'){
   if(fremdPlanet){a.push(plan(k,'angriff','Angriff planen',true,e.kind==='spionageberichte'?'mond':'planet'));a.push(spion(k,'Erneut ausspähen'));}
   if(e.kind==='berichte'&&r.schiffe&&r.verteidigung)a.push(knopf('data-bz-sim','Simulieren'));
  }else if(e.kind==='berichte'&&r.art==='erkundung'){a.push(plan(k,'kolonisieren','Kolonisieren',true));a.push(spion(k,'Erneut erkunden'));}
  else if(e.kind==='systemberichte'){a.push(knopf(`data-sy-profil="${h(e.system)}"`,'Systemprofil',{haupt:true}));a.push(knopf(`data-system-scan="${h(e.system)}:1"`,'Erneut scannen · 1 Sonde',{zeichen:'system_erkunden',key:'scan:'+e.system}));}
  else if(e.kind==='flottenberichte'&&k)a.push(plan(k,'saven','Flotte saven'));
  else if(e.art==='spionage'&&e.feind&&e.herkunft?.length){a.push(spion(e.herkunft[0],'Herkunft ausspähen'));a.push(plan(e.herkunft[0],'angriff','Angriff planen'));}
  else if(e.kind==='bodenberichte'&&fremdPlanet)a.push(plan(k,'angriff','Angriff planen'));
  for(const n of (e.spieler||[]).slice(0,2))a.push(profilKnopf(n));
  if(k&&Number(k.split(':')[2])>0)a.push(imSystemKnopf(k));else if(e.system)a.push(imSystemKnopf(e.system+':0'));
  return `<div class="bz-aktionen-reihe">${a.join('')}</div>`;
 }
 // Verwalten (nur auf diesem Gerät) und Begleitmeldungen stehen am Ende des Blatts – oben bleiben die Folgehandlungen.
 function fussHTML(e){
  const dazu=begleitVon(e,modell().eintraege);
  return `${dazu.length?`<section class="bz-begleit"><h3>Dazu gehören · ${dazu.length}</h3><ol class="bz-liste">${zeilen(dazu)}</ol></section>`:''}<div class="bz-aktionen-reihe bz-verwalten">${knopf(`data-bz-merken="${h(e.id)}"`,e.markiert?'Markierung lösen':'Markieren',{key:'m'})}${knopf(`data-bz-ausblenden="${h(e.id)}"`,e.aus?'Wieder einblenden':'Ausblenden',{key:'a'})}</div><p class="bz-leise bz-geraet">Markierungen und Ausblenden gelten auf diesem Gerät.</p>`;
 }
 // Zeit und Beteiligte als Text – Profil und Ort sind darunter eigene Knöpfe (keine doppelten kleinen Verweise).
 function metaHTML(e){
  const wer=(e.spieler||[]).join(', '),wo=e.koord&&Number(String(e.koord).split(':')[2])>0?ortText(e.koord):e.system?`[${e.system}]`:'';
  return `<p><time>${h(tagName(e.t))}, ${uhr(e.t)}</time> · ${alter(e.t)}${e.sim!=null&&Number.isFinite(e.sim)?` · Spielzeit ${h(simText(e.sim))}`:''}</p>${wer||wo?`<p>${wer?`<span>${e.feind?'Gegner':'Gegenüber'}: <b>${h(wer)}</b></span>`:''}${wo?`<span>Ort: ${mitKoord(wo)}</span>`:''}</p>`:''}`;
 }
 const AUGENBRAUE={kampfberichte:'Kampfbericht',bodenberichte:'Bodenkampf',berichte:'Spionagebericht',spionageberichte:'Mondspionage',systemberichte:'Systemscan',flottenberichte:'Flottenaufklärung',phalanxberichte:'Phalanxscan',ereignisse:'Meldung',chronik:'Chronik'};
 KARTEN.bericht=id=>{
  const e=finde(id);if(!e)return null;
  if(e.kind==='brief')return KARTEN.brief(e.brief);
  if(e.kind==='anfrage')return KARTEN.anfrage(e.row.id);
  const augen=(e.kind==='berichte'&&e.row?.art==='erkundung'?'Erkundungsbericht':AUGENBRAUE[e.kind]||'Bericht')+(e.feind?' · gegen dich':'')+(e.quelle==='archiv'?' · Archiv':'');
  let inhalt='';
  if(e.row){try{inhalt=window.Mobile035?.report?Mobile035.report(e.row,e.kind):'';}catch(err){console.warn('Bericht',err);}}
  else inhalt=`<div class="rb bz-archiv"><p class="rb-augenbraue">Zusammenfassung auf diesem Gerät</p><p>${mitKoord(e.unter||'')}</p>${(e.kern||[]).length?`<ul class="bz-kernliste">${e.kern.map(x=>`<li>${h(x)}</li>`).join('')}</ul>`:''}<p class="rb-hinweis">Der ausführliche Bericht liegt nicht mehr im Funkspeicher des Servers (er hält zum Beispiel nur die letzten 20 Kampfberichte und 150 Meldungen). Diese Zusammenfassung hat dein Gerät aufbewahrt.</p></div>`;
  return {augenbraue:augen,titel:e.titel,meta:metaHTML(e),aktionen:aktionen(e),inhalt,dossier:!!e.row,fuss:fussHTML(e),ton:e.ton,art:e.kind};
 };
 KARTEN.brief=id=>{
  const n=[...(communicationData().briefkasten||[]),...(typeof mailArchive!=='undefined'?mailArchive:[])].find(x=>String(x.id)===String(id));if(!n)return null;
  const e=finde('brief:'+n.id)||ausBrief(n),partner=n.gesendet?(n.an||[]):[n.von];
  return {augenbraue:`Brief · ${n.gesendet?'gesendet':'empfangen'}${n.automatisch?' · automatisch':''}`,titel:betreff(n.betreff),
   meta:`<p><time>${h(tagName(e.t))}, ${uhr(e.t)}</time> · ${alter(e.t)}${n.zeit?` · Spielzeit ${h(n.zeit)}`:''}</p><p class="bz-teilnehmer">${typeof relationHtml==='function'?relationHtml(n.von,n.von_beziehung):h(n.von)} → ${(n.an||[]).map(x=>typeof relationHtml==='function'?relationHtml(x,n.an_beziehungen?.find(r=>r.spieler===x)):h(x)).join(', ')}${n.antwort_auf?` · Antwort auf Brief ${h(n.antwort_auf)}`:''}</p>`,
   aktionen:`<div class="bz-aktionen-reihe">${n.gesendet?'':knopf(`data-bz-antwort="${h(n.id)}"`,'Antworten',{haupt:true})}${partner.filter(x=>x&&x!==ich()).slice(0,2).map(profilKnopf).join('')}</div>`,
   inhalt:`<div class="bz-brieftext">${h(n.text||'')}</div>`,dossier:false,fuss:fussHTML(e)};
 };
 KARTEN.anfrage=id=>{
  const a=(communicationData().anfragen||[]).find(x=>String(x.id)===String(id));if(!a)return null;
  const e=finde('anfrage:'+a.id)||ausAnfrage(a),partner=a.an===ich()?a.von:a.an,offen=a.status==='offen'&&a.an===ich();
  return {augenbraue:`Diplomatie · ${offen?'wartet auf dich':named(a.status)}`,titel:label(a.art),meta:`<p><time>${h(tagName(e.t))}, ${uhr(e.t)}</time> · ${alter(e.t)}</p><p>${h(a.von)} → ${h(a.an)}</p>`,
   aktionen:`<div class="bz-aktionen-reihe">${profilKnopf(partner)}</div>${offen&&window.AntwortLeiste?AntwortLeiste('anfrage',a.id,a.von,label(a.art)):''}`,
   inhalt:`<div class="bz-brieftext">${h(a.text||'')}</div><p class="bz-leise">Verträge, Kündigungsfristen und neue Anfragen unter Briefe → Diplomatie.</p>`,dossier:false,fuss:fussHTML(e)};
 };

 // ---------------------------------------------------------------- Handlungen
 function oeffnenEintrag(e){
  if(!e)return;
  if(typeof PlayerHelp!=='undefined'&&e.row&&e.quelle==='bericht')PlayerHelp.report={kind:e.kind,row:e.row,id:e.pid,title:AUGENBRAUE[e.kind]||'Bericht'};
  Blatt.oeffnen(e.kind==='brief'?'brief':e.kind==='anfrage'?'anfrage':'bericht',e.kind==='brief'?String(e.brief):e.kind==='anfrage'?String(e.row.id):e.id);
  alsGelesen(e,true);
  for(const x of begleitVon(e,modell().eintraege))if(!x.gelesen)alsGelesen(x,false);
 }
 function alsGelesen(e,sofort){
  if(!e||e.gelesen)return;
  const L=Speicher.laden();L.gelesen.add(e.id);Speicher.speichern();e.gelesen=true;
  if(e.quelle==='bericht'){if(!view.postfach_gelesen.includes(e.pid))view.postfach_gelesen.push(e.pid);if(sofort)Sync.sofort({schluessel:e.id,bericht:e.pid});}
  else if(e.quelle==='brief'){e.row.gelesen=true;if(sofort)Sync.sofort({schluessel:e.id,brief:e.brief});}
  cache.stempel='';zaehlerAuffrischen();render();
 }
 function alleGelesen(){
  const m=modell(),liste=m.eintraege.filter(passt).filter(e=>!e.gelesen);if(!liste.length)return;
  const L=Speicher.laden();for(const e of liste){L.gelesen.add(e.id);e.gelesen=true;}Speicher.speichern();
  cache.stempel='';lesestandAbgleichen();zaehlerAuffrischen();render();
 }
 function umschalten(menge,id){const L=Speicher.laden();L[menge].has(id)?L[menge].delete(id):L[menge].add(id);if(!L.aus.size)z.zeigeAus=false;Speicher.speichern();cache.stempel='';}
 async function mitKnopf(b,key,fn){
  if(busy.has(key))return;busy.add(key);b.disabled=true;b.setAttribute('aria-busy','true');fehler('');
  try{await fn();}catch(err){const t=err?.message||String(err);if(b.closest('#bz-blatt'))fehler(t);else show(t,true);}
  finally{busy.delete(key);if(b.isConnected){b.disabled=false;b.removeAttribute('aria-busy');}if(Blatt.offen())Blatt.zeichnen(false);}
 }
 async function spionieren(ziel){
  let n=1;try{n=Math.max(1,Math.min(1000,Number(localStorage.getItem('sternenepoche.spionage.sonden'))||1));}catch(_){}
  const taugt=p=>(p?.schiffe?.spionagesonde||0)>=n&&(p?.gebaeude?.raumhafen||0)>0;
  const eigen=typeof currentPlanet==='function'?currentPlanet():null,start=taugt(eigen)?eigen:(view.planeten||[]).find(taugt);
  if(!start)throw Error(`${n} ${n===1?'Spionagesonde':'Spionagesonden'} und ein Raumhafen am Startplaneten werden benötigt.`);
  const r=await command([{typ:'flotte_senden',start:start.koord,ziel,mission:'spionage',geschwindigkeit:1,haltedauer_stunden:0,schiffe:{spionagesonde:n},ladung:{}}]);
  if(r.ergebnisse?.some(x=>!x.ok))throw Error(r.ergebnisse.filter(x=>!x.ok).map(x=>x.text).join(' · '));
 }
 // Ort zeigen: die Galaxie hat die eine Planeten-Infokarte (GalaxieTabelle.zeigen öffnet System und Karte).
 function imSystem(k){
  const [s,n,p]=String(k).split(':').map(Number);if(!s||!n)return;
  Blatt.schliessen();
  if(window.GalaxieTabelle?.zeigen){GalaxieTabelle.zeigen(p>0?`${s}:${n}:${p}`:`${s}:${n}:0`);return;}
  navigateGame('karte');window.GalaxieTabelle?.gehe?.(s,n);
 }
 // Flottenaufträge immer über die Einsatzplanung (Einsatz.oeffnen), sonst über pickFleet – nie eigene Versandlogik.
 function planen(ziel,mission,zielart){
  if(window.Einsatz?.oeffnen){Einsatz.oeffnen({ziel,mission,zielart:zielart==='mond'?'mond':'planet'});return;}
  window.pickFleet?.(ziel,mission);
 }
 function zeigeZentrale(vorgabe){
  Object.assign(z,{ansicht:'chronik',art:'alle',unter:'alle',spieler:'',system:'',text:'',ungelesen:false,markiert:false,menge:MENGE},vorgabe||{});
  ordnerSetzen();const s=$('mail-search');if(s)s.value=z.text;
  Blatt.schliessen();
  if(!seiteSichtbar())navigateGame('briefkasten');
  render();window.scrollTo({top:0,behavior:'instant'});
 }
 // Klicks: Erfassungsphase für „Bericht öffnen“ überall im Spiel (Übersicht, Sofortleiste) – ein Detail, kein zweiter Leser.
 document.addEventListener('click',e=>{
  const b=e.target.closest('[data-open-report]');if(!b||b.closest('#report-reader'))return;
  const x=finde('b:'+b.dataset.openReport);if(!x)return;
  e.preventDefault();e.stopPropagation();oeffnenEintrag(x);
 },true);
 document.addEventListener('click',e=>{
  const b=e.target.closest('[data-bz-ansicht],[data-bz-art],[data-bz-unter],[data-bz-filter-spieler],[data-bz-filter-system],[data-bz-filter-weg],[data-bz-ungelesen],[data-bz-markiert],[data-bz-alle-gelesen],[data-bz-mehr],[data-bz-oeffnen],[data-bz-zeige-aus],[data-bz-sort],[data-bz-blatt-zu],[data-bz-blatt-zurueck],[data-bz-merken],[data-bz-ausblenden],[data-bz-plan],[data-bz-spion],[data-bz-karte],[data-bz-antwort],[data-bz-sim],[data-sp-profil],[data-sy-profil],[data-sy-planet],[data-intel-planet],[data-intel-player],[data-intel-focus]');
  if(!b)return;const d=b.dataset;
  if(d.bzAnsicht){z.ansicht=z.ansicht===d.bzAnsicht?'chronik':d.bzAnsicht;z.text='';const s=$('mail-search');if(s)s.value='';neuZeichnen();return;}
  if(d.bzArt){if(z.ansicht!=='chronik'){z.ansicht='chronik';z.art=d.bzArt;}else z.art=z.art===d.bzArt&&d.bzArt!=='alle'?'alle':d.bzArt;if(z.art!=='briefe')z.unter='alle';ordnerSetzen();neuZeichnen();return;}
  if(d.bzUnter){z.unter=d.bzUnter;ordnerSetzen();neuZeichnen();return;}
  if(d.bzFilterSpieler){zeigeZentrale({spieler:d.bzFilterSpieler,art:z.art==='briefe'?'alle':z.art});return;}
  if(d.bzFilterSystem){zeigeZentrale({system:d.bzFilterSystem,art:z.art==='briefe'?'alle':z.art});return;}
  if(d.bzFilterWeg){const w=d.bzFilterWeg;if(w==='alle')filterLoeschen();else if(w==='text'){z.text='';const s=$('mail-search');if(s)s.value='';}else z[w]=w==='spieler'||w==='system'?'':false;neuZeichnen();return;}
  if('bzUngelesen' in d){z.ungelesen=!z.ungelesen;neuZeichnen();return;}
  if('bzMarkiert' in d){z.markiert=!z.markiert;neuZeichnen();return;}
  if('bzAlleGelesen' in d){alleGelesen();return;}
  if('bzMehr' in d){z.menge+=MENGE;render();return;}
  if('bzZeigeAus' in d){z.zeigeAus=!z.zeigeAus;cache.stempel='';render();return;}
  if(d.bzSort){z.sortierung=d.bzSort;render();return;}
  if(d.bzOeffnen){oeffnenEintrag(finde(d.bzOeffnen));return;}
  if('bzBlattZu' in d){Blatt.schliessen();return;}
  if('bzBlattZurueck' in d){Blatt.zurueck();return;}
  if(d.bzMerken){umschalten('markiert',d.bzMerken);render();return;}
  if(d.bzAusblenden){const war=Speicher.laden().aus.has(d.bzAusblenden);umschalten('aus',d.bzAusblenden);if(!war&&!z.zeigeAus)Blatt.zurueck();render();return;}
  if(d.bzPlan){planen(d.bzPlan,d.bzAuftrag||'angriff',d.bzZielart);return;}
  if(d.bzSpion){void mitKnopf(b,'spion:'+d.bzSpion,()=>spionieren(d.bzSpion));return;}
  if(d.bzKarte){imSystem(d.bzKarte);return;}
  if(d.bzAntwort){antworten(d.bzAntwort);return;}
  if('bzSim' in d){const t=Blatt.oben();const x=t&&finde(t.schluessel);if(x?.row&&typeof PlayerHelp!=='undefined')PlayerHelp.report={kind:x.kind,row:x.row,id:x.pid};const p=document.createElement('button');p.type='button';p.hidden=true;p.dataset.simReport='';document.body.append(p);p.click();p.remove();return;}
  if(d.spProfil){window.Spielerprofil?.oeffnen(d.spProfil);return;}
  if(d.syProfil){window.Systemprofil?.oeffnen(d.syProfil);return;}
  if(d.syPlanet){imSystem(d.syPlanet);return;}
  if(d.intelPlanet){imSystem(d.intelPlanet);return;}
  if(d.intelPlayer){window.Spielerprofil?.oeffnen(d.intelPlayer);return;}
  if(d.intelFocus){window.Systemprofil?.oeffnen(sysVon(d.intelFocus)||d.intelFocus);return;}
 });
 function antworten(id){
  const n=[...(communicationData().briefkasten||[]),...(typeof mailArchive!=='undefined'?mailArchive:[])].find(x=>String(x.id)===String(id));if(!n)return;
  const f=$('message-form');if(!f||typeof openComposer!=='function')return;
  f.elements.an.value=n.von;f.elements.betreff.value=('Re: '+(n.betreff||'')).slice(0,120);f.elements.antwort_auf.value=n.id;
  openComposer('privat');f.elements.text.focus();
 }
 document.addEventListener('input',e=>{if(e.target.id==='mail-search'){z.text=e.target.value;z.menge=MENGE;render();}});
 document.addEventListener('keydown',e=>{
  if(e.target.id==='mail-search'&&e.key==='Enter'){const c=$('bz-vorschlaege')?.querySelectorAll('[data-bz-filter-spieler],[data-bz-filter-system]');if(c?.length===1){e.preventDefault();c[0].click();}}
  if(e.key==='Escape'&&Blatt.offen()&&!document.querySelector('dialog[open]')){e.preventDefault();Blatt.zurueck();}
 });
 // Seitenwechsel schließt das Blatt; die Zentrale zeichnet erst, wenn sie sichtbar wird (verdeckt nur „schmutzig“).
 let seite='';
 document.addEventListener('game-navigation',()=>{const neu=window.Rahmen?.aktiverBereich?.()||'';if(neu!==seite){seite=neu;Blatt.schliessen();}if(seiteSichtbar()&&(schmutzig||montiert))render();});
 // Android-Zurück: erst ein offenes Dialogfenster (macht der Rahmen), dann das Blatt, dann Auswahlliste, dann Filter.
 if(window.Rahmen?.zurueckHaken)Rahmen.zurueckHaken.push(()=>{
  try{if(document.querySelector('dialog[open]:modal'))return false;}catch(_){if(document.querySelector('dialog[open]'))return false;}
  if(Blatt.offen())return Blatt.zurueck();
  if(seiteSichtbar()&&filterAktiv()){filterLoeschen();neuZeichnen();return true;}
  return false;
 });
 // Eine Seite statt zweier: „Berichte“ (Themenblatt, alte Links) führt in dieselbe Zentrale; die Reiter
 // „Briefkasten | Berichte“ entfallen (Laufzeit-Haken, rahmen.js bleibt unverändert).
 if(window.Rahmen?.navigieren){
  const vorher=Rahmen.navigieren;
  Rahmen.navigieren=id=>{if(id==='berichte'){Object.assign(z,{ansicht:'chronik',art:'alle',unter:'alle',spieler:'',system:'',text:'',menge:MENGE});ordnerSetzen();return 'briefkasten';}return vorher(id);};
  const gruppe=Rahmen.MENUE?.find(g=>g[0]==='nachrichten');if(gruppe)gruppe[2]=[['briefkasten','Nachrichten']];
 }
 function karte(typ,fn){KARTEN[typ]=fn;}
 return {render,modell,finde,ordner,karte,blatt:Blatt,zeileHTML,zeilen,begleitVon,oeffnenEintrag,zeigeZentrale,alsGelesen,spionieren,mitKnopf,knopf,plan,planen,spion,profilKnopf,imSystemKnopf,imSystem,
  hilfe:{h,summe,sysVon,ortText,mitKoord,alter,uhr,tagName,dauer,simText,kurz,hash,ROEMISCH,drohung,spielerHeimat,echtAusSim,jetzt},zustand:z,ausblendenSichtbar:()=>z.zeigeAus};
})();

// ---------------------------------------------------------------- Bisherige Schnittstellen (andere Module, Prüfskripte)
window.Berichtsarchiv=(()=>{
 const h=s=>esc(s);
 // presentation.renderReports → eine Zentrale (die Seite „Berichte“ führt in dieselbe Chronik).
 function render(){Funk.render();}
 // Der alte Ordner „Berichte & Meldungen“ im Briefkasten ist in der Zentrale aufgegangen.
 function inbox(){return '';}
 function zeige(k,art='system'){
  if(art==='planet'){Funk.imSystem(k);return;}
  if(art==='spieler'){window.Spielerprofil?.oeffnen(k);return;}
  window.Systemprofil?.oeffnen(Funk.hilfe.sysVon(k)||k);
 }
 // Inhalt eines Systemscans (Mobile035.report delegiert hierher).
 function systembericht(row){
  const sys=Funk.hilfe.sysVon(row.ziel),pl=row.plaetze||[];
  // Besiedelte Plätze, Monde und Trümmer einzeln; leere Plätze als eine Zeile (zwölf gleiche Zeilen „unbewohnt“ waren zu lang).
  const wichtig=p=>!!(p.spieler||p.truemmer||p.mond||p.status==='eigen'),frei=pl.filter(p=>!wichtig(p)).map(p=>p.position??String(p.koord).split(':')[2]);
  const platz=p=>{const tf=p.truemmer,besitzer=p.spieler;return `<li class="bz-platz${besitzer?'':' frei'}"><button type="button" class="bz-k bz-platz-knopf" data-sy-planet="${h(p.koord)}"><b class="koord">[${h(p.koord)}]</b><span>${besitzer?h(besitzer):p.bewohnt===false||p.status==='frei'?'unbewohnt':'unbekannt'}${p.status==='eigen'?' · eigen':''}</span><small>${[p.mond?'☾ Mond':'',tf?`Trümmerfeld ${fmt((tf.erz||0)+(tf.kristall||0))}${tf.ueberlebende?' · '+fmt(tf.ueberlebende)+' Überlebende':''}`:''].filter(Boolean).join(' · ')}</small></button></li>`;};
  return `<div class="rb bz-systembericht"><p class="rb-augenbraue">Systemscan · eine Sonde</p><h3>System <span class="koord">[${h(sys)}]</span></h3><dl class="bz-fakten"><div><dt>Nebel</dt><dd>${row.nebel?'Ja':'Nein'}</dd></div><div><dt>Asteroidengürtel</dt><dd>${row.asteroidenguertel?(row.bergbau?.groesse?h(row.bergbau.groesse)+' · Rohstoffumfang':'vorhanden'):'keiner'}</dd></div><div><dt>Besiedelt</dt><dd>${fmt(pl.filter(p=>p.spieler).length)} von ${fmt(pl.length||12)}</dd></div></dl><p class="rb-hinweis">Die Systemsonde erkennt Besiedlung, Monde und Trümmer. Felder, Klima und Bestände eines Planeten zeigt erst ein Planetenscan.</p>${pl.length?`${pl.some(wichtig)?`<ul class="bz-plaetze">${pl.filter(wichtig).map(platz).join('')}</ul>`:''}${frei.length?`<p class="bz-frei">${frei.length===pl.length?'Alle Plätze':'Ohne Besiedlung'}: Platz ${frei.join(', ')}</p>`:''}`:'<p class="rb-leer">Für diesen früheren Scan ist keine Einzelaufstellung gespeichert.</p>'}</div>`;
 }
 return {render,inbox,zeige,systembericht};
})();

// Galaxietabelle „Planet ⓘ“ (data-gt-planet-info) und Systemsonde – jetzt als Karte im Blatt statt modalem Dialog.
window.SystemIntel=(()=>{
 const daten=new Map();
 function planet(p){if(!p?.koord)return;daten.set(p.koord,p);Funk.imSystem(p.koord);}
 async function scan(ziel){
  const bases=view.planeten.filter(p=>p.schiffe?.spionagesonde>0&&p.gebaeude?.raumhafen>0),p=bases.find(p=>p.koord===currentPlanet()?.koord)||bases[0];
  if(!p)throw Error('Eine Spionagesonde und ein Raumhafen am Startplaneten werden benötigt.');
  const result=await command([{typ:'flotte_senden',start:p.koord,ziel,mission:'system_erkunden',geschwindigkeit:1,haltedauer_stunden:0,schiffe:{spionagesonde:1},ladung:{}}]);
  if(result.ergebnisse?.some(r=>!r.ok))throw Error(result.ergebnisse.filter(r=>!r.ok).map(r=>r.text).join(' · '));
 }
 document.addEventListener('click',e=>{
  const b=e.target.closest('[data-system-scan]');if(!b)return;
  void Funk.mitKnopf(b,'scan:'+Funk.hilfe.sysVon(b.dataset.systemScan),()=>scan(b.dataset.systemScan));
 });
 return {planet,scan,daten};
})();
