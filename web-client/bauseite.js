'use strict';
// Bauseiten nach docs/DESIGNREGELN.md §2: Bannerbild → Kachelraster → Detailfeld (ersetzt das Banner) →
// drei Warteschlangenkästen. Techinfo-Fenster mit vier Reitern nach §3, dazu das Archiv (LORE-BIBEL §8). Befehle laufen über die vorhandenen
// Handler: data-build / data-research (app.js), data-make (game-ui.js), data-ground-build (mobile035.js),
// data-queue-change (queue-controls.js), data-pause / data-demolish (management.js).
const Bauseite=(()=>{
 const VERSORGUNG=['erzmine','kristallmine','deuteriumsynthesizer','farm','xenoextraktor','solarkraftwerk','fusionskraftwerk','versorgungsnetz','giesserei','elektronikwerk','konsumgueterwerk','lager','bunker','wohnblock'];
 const KAMPFKLASSEN=['jaeger','kreuzer','schlachter','schlachtkreuzer','bomber','zerstoerer'];
 const SEITEN={
  versorgung:{kat:'buildings',host:'versorgung-bauseite',titel:'Versorgung',bild:['buildings','deuteriumsynthesizer'],satz:'Rohstoffe fördern, Energie erzeugen, Güter veredeln und lagern.',
   gruppen:[['Rohstoffgebäude',['erzmine','kristallmine','deuteriumsynthesizer','farm','xenoextraktor']],['Energie',['solarkraftwerk','fusionskraftwerk','versorgungsnetz']],['Verarbeitung',['giesserei','elektronikwerk','konsumgueterwerk']],['Lager & Wohnen',['lager','bunker','wohnblock']]]},
  gebaeude:{kat:'buildings',host:'anlagen-bauseite',titel:'Anlagen',bild:['buildings','raumhafen'],satz:'Werften, Labore, Verwaltung und Aufklärung für dein Imperium.',
   gruppen:[['Fertigung & Forschung',['werft','orbitalwerft','kaserne','raketensilo','labor','forschungsarchiv','akademie']],['Ausbau & Verwaltung',['bauhof','nanofabrik','orbitalring','verwaltungszentrum']],['Handel & Aufklärung',['raumhafen','markt','geheimdienst','sensorphalanx']]],rest:'Ausbau & Verwaltung'},
  forschen:{kat:'research',host:'forschung-bauseite',titel:'Forschung',bild:['backgrounds','research'],satz:'Eine Forschung läuft reichsweit; Labore auf der Heimatwelt bestimmen, was möglich ist.',
   gruppen:[['Grundlagenforschung',['energietechnik','computertechnik','werkstoffkunde','automatisierung','logistik','soziologie','agrarwissenschaft','xenomaterialkunde']],['Wirtschaft',['plasmatechnik']],['Antriebe & Raumfahrt',['verbrennungsantrieb','impulsantrieb','hyperraumantrieb','astrophysik','terraforming']],['Kampfforschung',['waffentechnik','schildtechnik','panzerung']],['Aufklärung',['spionagetechnik','ueberwachungstechnik','abschirmtechnik']]],rest:'Grundlagenforschung'},
  werft:{kat:'ships',host:'werft-bauseite',titel:'Schiffswerft',bild:['backgrounds','shipyard'],satz:'Jede Werftstufe schafft einen Fertigungsplatz; schwere Schiffe belegen mehrere.'},
  verteidigung:{kat:'defenses',host:'verteidigung-bauseite',titel:'Verteidigung',bild:['defenses','planetenschild'],satz:'Geschütze und Schilde schützen diesen Planeten. Raketen stehen darunter im Raketensilo.'},
  truppen:{kat:'ground',host:'truppen-bauseite',titel:'Truppen',bild:['buildings','kaserne'],satz:'Bodentruppen verteidigen die Oberfläche und übernehmen Planeten bei einer Invasion.'}
 };
 const SEITE_JE_KAT={research:'forschen',ships:'werft',defenses:'verteidigung',ground:'truppen'};
 const BILDKAT={buildings:'buildings',research:'research',ships:'ships',defenses:'defenses',ground:'ground_units',missiles:'missiles',bauteile:'resources'};
 const offen={},menge={};
 let texte={},geladen=0,ladeVersuch=0;
 // Werft (Karl 24.10.2026: „bei Werft will ich nur baubare Schiffe sehen, den Rest eingeklappt wegen Übersicht!“): oben nur,
 // was auf diesem Planeten jetzt gebaut werden kann (Forschung, Zivilisationsstufe, Werft, Gebäude erfüllt; fehlende Rohstoffe
 // sagt die Kachel), darunter EIN eingeklappter Bereich mit dem Rest und dem Grund je Schiff. Offen/zu merkt localStorage.
 const EINKLAPPEN={versorgung:'sternenepoche.versorgung.gesperrt-offen',gebaeude:'sternenepoche.gebaeude.gesperrt-offen',truppen:'sternenepoche.truppen.gesperrt-offen',verteidigung:'sternenepoche.verteidigung.gesperrt-offen',werft:'sternenepoche.werft.gesperrt-offen',bauteile:'sternenepoche.bauteile.gesperrt-offen'};
 const merkOffen=seite=>{try{return localStorage.getItem(EINKLAPPEN[seite])==='1';}catch{return false;}};
 const grund=x=>x.needs.map(n=>String(n).replace(/^Zivilisationsstufe (\d)$/,(_,z)=>'Stufe '+(['','I','II','III','IV','V'][z]||z))).join(' · ');

 // Beschreibungskatalog (§6). Fehlt Datei oder Eintrag, bleiben die Felder einfach weg.
 async function ladeTexte(erzwingen=false){
  if(!erzwingen&&Date.now()-ladeVersuch<120000)return;ladeVersuch=Date.now();
  try{const r=await fetch((typeof base==='string'&&base?base:'')+'/beschreibungen.json',{signal:AbortSignal.timeout(8000)});if(!r.ok)return;const j=await r.json();
   if(j&&typeof j.eintraege==='object'&&!Array.isArray(j.eintraege)){texte=j.eintraege;geladen=Date.now();render();}}catch{}
 }
 const text=key=>texte[key]&&typeof texte[key]==='object'?texte[key]:null;
 const lore=e=>{const l=e?.lore;if(!l)return '';if(typeof l==='string')return l;return l[view?.volk]||l.allgemein||'';};
 const ersterSatz=t=>{const s=String(t||'').trim();const m=/^(.+?[.!?])(\s|$)/.exec(s);return m?m[1]:s;};

 // Zeiten wie „5 T 11 h 32 min“ (§5), Eingabe in Echtzeit-Sekunden.
 function dauer(s){if(s==null||!Number.isFinite(s))return '–';s=Math.max(0,Math.round(s));if(s<60)return s+' s';const t=Math.floor(s/86400),h=Math.floor(s%86400/3600),m=Math.floor(s%3600/60);if(!t&&!h&&m<10)return `${m} min ${s%60} s`;return [t?t+' T':'',h?h+' h':'',m?m+' min':''].filter(Boolean).join(' ')||'0 min';}
 const tempo=()=>Math.max(1,world?.tempo||1);
 const heimat=()=>view.planeten.find(x=>x.heimat)||view.planeten[0];
 function planetName(p){if(p.name)return p.name;if(p.heimat)return 'Heimatwelt';return 'Kolonie '+(view.planeten.filter(x=>!x.heimat).findIndex(x=>x.koord===p.koord)+1);}

 function truppen(p=currentPlanet()){
  const d=p?.bodentruppen;if(!d)return [];const gueter=p.industrie?.gueter||[];
  return d.typen.map((key,i)=>{const era=d.ab_stufe[i],needs=[];if(era>view.stufe)needs.push('Zivilisationsstufe '+era);if((p.gebaeude.kaserne||0)<d.kaserne[i])needs.push('Kaserne '+d.kaserne[i]);
   const cost=Object.fromEntries(gueter.map((g,n)=>[g,(d.kosten_aktuell_milli?.[i]?.[n]||0)/1000]).filter(([,n])=>n>0));const regler=p.industrie?.regler?.kaserne;
   return {key,i,name:d.namen?.[i]||named(key),bild:d.bilder?.[i]||key,r:{ab_stufe:era,wirkung:d.rollen?.[i]||''},level:d.bestand[i]||0,cost,needs,missing:[],stock:p.bestand,plaetze:Number(d.fertigungsplaetze_je_stueck?.[i])||1,personal:Number(d.personal?.[i])||0,duration:regler?d.grunddauer_sekunden[i]*100/regler*invasionsFaktor(p):null,job:(d.auftraege||[]).find(a=>a.art===i)||null};});
 }
 // Invasion v2 (Karl 24.10.2026: „Man kann auch weiter Bodentruppen bauen (50 % langsamer), diese sammeln sich alle in der
 // letzten Welle als Rückhalt“): Kaserne arbeitet mit ausbildung_promille (Kern boden.rs), solange die Invasion läuft.
 function invasionsFaktor(p){const w=p?.invasion?.wellen;return w?1000/Math.max(1,Number(w.ausbildung_promille)||1000):1;}
 function invasionHinweis(p,seite='truppen'){
  const w=p?.invasion?.wellen;if(!w)return '';const langsamer=Math.round(100-(Number(w.ausbildung_promille)||1000)/10),verw=w.verwuestet||p.invasion.verwuestet;
  // Auf den Gebäudeseiten erklärt der Hinweis die Reparatursperre (Funktionsprüfung 25.10.2026: dort stand nur „Belagert“).
  const satz=seite==='truppen'?(verw?'Neue Truppen besetzen den Rückhalt wieder.':`Ausbildung ${langsamer>0?langsamer+' % langsamer':'läuft weiter'} – neue Truppen stehen im Rückhalt.`):(verw?'Der Planet ist besetzt: keine Reparaturen, solange die Verwüstung andauert.':'Unter Belagerung sind keine Reparaturen möglich.');
  return `<p class="bs-invasion" role="status"><b>${verw?'Verwüstet':'Invasion läuft'}</b><span>${satz}</span><button type="button" data-bk-oeffnen="${esc(p.koord)}">Linien verteilen ›</button></p>`;
 }
 // Reparieren (Karl 24.10.2026: „…auf jedem Gebäude ein ‚Reparieren‘, wo sonst ‚Bauen‘ steht, mit den Kosten für Reparatur!“).
 // Kern-Sicht: reparaturkosten {gebaeude:{gut:menge}} nur für beschädigte Gebäude, reparaturen [{gebaeude,fertig}] laufend.
 // Kern kolonisation.rs reparieren: nicht unter Bombardierung/Belagerung, eine planetare Baustelle für Bau und Reparatur.
 function reparatur(p,key){
  const kosten=p?.reparaturkosten?.[key];if(!kosten)return null;
  const lauf=(p.reparaturen||[]).find(r=>r.gebaeude===key)||null,hp=Math.round(Number(p.gebaeude_integritaet_prozent?.[key]??100));
  const andere=(p.reparaturen||[]).find(r=>r.gebaeude!==key),bau=(p.bauschleife||[])[0];
  const besetzt=!!(p.invasion?.belagerung||p.invasion?.bombardement||p.kampfkolonisation||p.invasion?.verwuestet);
  const fehlt=Object.entries(kosten).filter(([g,v])=>(p.bestand?.[g]||0)<v);
  const verw=!!p.invasion?.verwuestet;
  const grund=lauf?'':besetzt?(verw?'Verwüstet: Solange der Planet besetzt ist, sind keine Reparaturen möglich':'Unter Bombardierung oder Belagerung sind keine Reparaturen möglich'):andere?`Baustelle belegt: ${named(andere.gebaeude)} wird repariert`:bau?`Baustelle belegt: ${named(bau.gebaeude)} wird gebaut`:fehlt.length?'Rohstoffe fehlen: '+fehlt.map(([g,v])=>`${named(g)} ${fmt(Math.ceil(v-(p.bestand?.[g]||0)))}`).join(', '):'';
  const kurz=lauf?'':besetzt?(verw?'Verwüstet – keine Reparatur':'Belagert – keine Reparatur'):andere||bau?'Baustelle belegt':fehlt.length?'Rohstoffe fehlen':'';
  return {kosten,lauf,hp,grund,kurz,besetzt,v2:p.invasion?.regeln==='invasion-v2'};
 }
 const repDauer=new Map();
 function repDauerLaden(p,key,hp){
  const id=`${p.koord}:${key}:${hp}`;if(repDauer.has(id)||typeof api!=='function')return repDauer.get(id);
  repDauer.set(id,null);api('/api/tool',{typ:'kosten',planet:p.koord,gebaeude:key}).then(q=>{const s=Number(q?.reparatur?.dauer_sekunden);if(Number.isFinite(s)){repDauer.set(id,s);render();}}).catch(()=>{});
  return null;
 }
 function eintraege(seite,p=currentPlanet()){
  const s=SEITEN[seite];if(!p||!rules?.catalog)return [];
  if(s.kat==='ground')return truppen(p);
  let items=itemRecords(s.kat,p);
  // Weltenkrümmer entstehen nur im Werftdock eines Mondes (Mondseite).
  if(s.kat==='ships')items=items.filter(x=>x.key!=='weltentruemmer');
  if(s.kat==='buildings')items=items.map(x=>p.reparaturkosten?.[x.key]?{...x,needs:x.needs.filter(n=>!/reparier/i.test(String(n))&&n!=='Maximalstufe erreicht')}:x);
  if(seite==='versorgung')items=items.filter(x=>VERSORGUNG.includes(x.key));
  if(seite==='gebaeude')items=items.filter(x=>!VERSORGUNG.includes(x.key));
  return items;
 }
 function gruppen(seite,items){
  const s=SEITEN[seite],c=rules.catalog;
  if(s.kat==='ships'){const kampf=items.filter(x=>KAMPFKLASSEN.includes(c.einheiten[x.key]?.taktik?.klasse));return [['Kampfschiffe',kampf],['Zivile Schiffe',items.filter(x=>!kampf.includes(x))]];}
  if(s.kat==='defenses'){const schild=items.filter(x=>c.einheiten[x.key]?.taktik?.klasse==='schild'),sonden=items.filter(x=>x.key==='solarsonde');return [['Geschütze',items.filter(x=>!schild.includes(x)&&!sonden.includes(x))],['Planetenschilde',schild],['Solarsatelliten · '+(()=>{const je=currentPlanet()?.energie?.je_solarsonde;return je>0?`je Satellit ${fmt(je)} Energie auf dieser Bahn`:'Energie aus der Umlaufbahn';})(),sonden]];}
  if(s.kat==='ground'){const eras=[...new Set(items.map(x=>x.r.ab_stufe))].sort((a,b)=>a-b);return eras.map(n=>['Zeitalter '+['','I','II','III','IV','V'][n],items.filter(x=>x.r.ab_stufe===n)]);}
  const liste=s.gruppen.map(([name,keys])=>[name,keys.map(k=>items.find(x=>x.key===k)).filter(Boolean)]);
  const bekannt=new Set(s.gruppen.flatMap(([,k])=>k)),rest=items.filter(x=>!bekannt.has(x.key));
  if(rest.length){const ziel=liste.find(([n])=>n===s.rest)||liste[liste.length-1];ziel[1].push(...rest);}
  return liste;
 }
 function gruppenHinweis(seite,index,p){
  if(index)return '';const s=SEITEN[seite];
  if(s.kat==='buildings')return `[Baufelder ${p.felder.belegt} / ${p.felder.gesamt} belegt${p.felder.belegt>p.felder.gesamt?' – mehr als deine Felder: gebaut mit Terraforming oder Orbitalring des Vorbesitzers; Neubau erst, wenn durch Abriss wieder ein Feld frei ist':''}]`;
  if(s.kat==='research')return `[${fmt((view.forschung.punkte_je_stunde||0)*tempo())} FP je Echtzeitstunde]`;
  if(s.kat==='ships'||s.kat==='defenses'){const w=p.fertigungsplaetze?.werft;return w?`[Werftplätze ${w.frei} von ${w.plaetze} frei]`:'';}
  if(s.kat==='ground')return `[Kaserne Stufe ${p.gebaeude.kaserne||0}]`;
  return '';
 }
 // Bild je Eintrag über art(); Sondermotive (Solarsonde, Weltenkrümmer) lösen die Katalogaliase in art.js auf.
 // Weiche Trennstellen in langen Namen vor bekannten Wortteilen (Deuterium­synthesizer, Verwaltungs­zentrum):
 // wirkt in jedem Browser, auch ohne Silbentrennungs-Wörterbuch (Handy-Abnahme 18.10.: Brüche wie „Verwaltungszentr|um“).
 const FUGEN=/(synthesizer|zentrum|technik|antrieb|infanterie|kraftwerk|schlachtschiff|schlachtkreuzer|kreuzer|zerstörer|transporter|frachter|kolonieschiff|forschung|extraktor|anlage|panzer|drohne|geschütz|kanone|kunde|schaft|archiv|fabrik|satellit|sonde|werfer|phalanx|ring|werft|schild|netz|werk|block|mine|lager|pionier)$/i;
 // Silbentrennung aus stable-dom.js (Fugen und Silben; „Automatisierung“, „Überwachungs·technik“ brachen sonst mitten im Wort).
 const trennbar=s=>window.Silben?Silben.html(String(s)):String(s).replace(/[A-Za-zÄÖÜäöüß]{13,}/g,w=>{const m=w.match(FUGEN);return m&&w.length-m[0].length>=4?w.slice(0,w.length-m[0].length)+'&shy;'+w.slice(w.length-m[0].length):w;});
 const bildVon=(kat,x)=>art(BILDKAT[kat],kat==='ground'?x.bild:x.key);
 const istBezahlbar=(x,n,p)=>Object.entries(x.cost||{}).every(([g,v])=>((x.stock||p.bestand)[g]||0)>=v*n);
 function laeuft(kat,x){if(kat==='research')return view.forschung.aktiv?.forschung===x.key;if(kat==='ground')return !!x.job;return !!x.job&&!x.job.wartet;}
 // Eintrag in der Kern-Warteschlange (Bau je Planet, Forschung reichsweit) mit Platz, Stufe und Kern-Grund. Eingereihte
 // Einträge sind nicht „gesperrt“, auch wenn die Schlange voll ist (Funktionsprüfung 25.10.2026: sie rutschten zugeklappt weg).
 function inSchlange(kat,x,p){
  if(kat!=='buildings'&&kat!=='research')return null;const art=kat==='buildings'?'bau':'forschung';
  const q=view.forschung.warteschlangen?.find(q=>q.art===art&&(art==='forschung'||q.planet===p.koord)),i=q?.jobs.findIndex(j=>j.key===x.key)??-1;
  return i>=0?{art,q,i,j:q.jobs[i]}:null;
 }
 // Reparatur und Bau teilen sich eine planetare Baustelle (Kern kolonisation.rs, warteschlangen.rs „Wartet auf laufende Reparatur“).
 function laufendeReparatur(p){const r=(p.reparaturen||[]).slice().sort((a,b)=>a.fertig-b.fertig)[0];return r?{...r,rest:Math.max(0,Number(r.fertig)-GameTime.now())}:null;}

 function kachel(seite,x,p){
  const kat=SEITEN[seite].kat,e=text(x.key),gesperrt=x.needs.length>0&&!inSchlange(kat,x,p),teuer=!gesperrt&&!istBezahlbar(x,1,p),baut=laeuft(kat,x),gewaehlt=offen[seite]===x.key,rep=kat==='buildings'?reparatur(p,x.key):null;
  // Baufortschritt im Bild (Karl 22.10.2026): unten grau, darüber das farbige Motiv, das von unten nach oben aufgedeckt
  // wird (--progress aus tickJobs, data-job am Bild); bei Fertigstellung (Stufe/Bestand gestiegen) kurzes Aufleuchten.
  const bauJob=baut?kachelJob(kat,x):null,stufeKey=p.koord+':'+kat+':'+x.key,vorher=stufeVorher.get(stufeKey),fertig=vorher!=null&&x.level>vorher;
  stufeVorher.set(stufeKey,x.level);
  const zahl=['ships','defenses','ground'].includes(kat)?fmt(x.level):x.level;
  const titel=[x.name||named(x.key),(['ships','defenses','ground'].includes(kat)?'Bestand ':'Stufe ')+zahl,rep?`beschädigt, Integrität ${rep.hp} %`:'',e?.rolle||ersterSatz(x.r?.wirkung),gesperrt?'Fehlt: '+x.needs.join(' · '):''].filter(Boolean).join(' – ');
  // Motiv vollständig (object-fit: contain), Name und Zahl in einer eigenen Fußzeile unter dem Bild – nie darüber.
  // Darunter direkt Bauen und der laufende Timer (Karl 22.10.2026: „gleich auf dem Bild in der Vorschau bauen, nicht erst
  // öffnen“ und „ich muss den Timer sehen, damit ich alles visuell verfolgen kann“). Bild antippen öffnet weiter das Detailfeld.
  return `<div id="bs-k-${seite}-${esc(x.key)}" class="bs-kachel${kat==='ships'?' quer':''}${gesperrt?' gesperrt':''}${teuer?' teuer':''}${baut?' baut':kachelJob(kat,x)?' wartet':''}${fertig?' fertig':''}${gewaehlt?' gewaehlt':''}${rep?' beschaedigt':''}"><button type="button" class="bs-kachel-auf" data-bs-seite="${seite}" data-bs-key="${esc(x.key)}" aria-pressed="${gewaehlt}" title="${esc(titel)}"><span class="bs-bild"${bauJob?` data-job="${esc(keepJob(bauJob))}"`:''}>${rep?`<span class="bs-integritaet" aria-label="Integrität ${rep.hp} %">${rep.hp} %</span>`:''}<img src="${esc(bildVon(kat,x))}" alt="" loading="lazy" decoding="async">${bauJob?`<img class="bs-farbe" src="${esc(bildVon(kat,x))}" alt="" decoding="async"><span class="bs-uhr" aria-live="off"><b class="job-uhr"></b><small class="job-zustand"></small>${bauJob.parallel>1?`<small class="bs-parallel">×${fmt(bauJob.parallel)} gleichzeitig</small>`:''}</span>`:!baut&&kachelJob(kat,x)?'<span class="bs-uhr"><small>wartet</small></span>':''}</span><span class="bs-fuss"><span class="bs-name">${trennbar(esc(x.name||named(x.key)))}</span><span class="bs-zahl">${zahl}${x.job&&kat!=='ground'&&x.job.stufe?' → '+x.job.stufe:''}</span></span></button>${kachelUnten(seite,x,p)}</div>`;
 }
 // Laufender Auftrag dieses Eintrags (Timer und Balken laufen über tickJobs in game-ui.js).
 const stufeVorher=new Map();
 function kachelJob(kat,x){
  if(kat==='research'){const f=view.forschung.aktiv;return f?.forschung===x.key?f:null;}
  if(kat==='ground')return x.job&&x.job.fertig_sekunden!=null?x.job:null;
  return x.job||null;
 }
 function kachelUnten(seite,x,p){
  const kat=SEITEN[seite].kat,lesen=!!gameReadOnly(),gesperrt=x.needs.length>0,job=kachelJob(kat,x),stueck=['ships','defenses','ground'].includes(kat);
  // Bauschleife mit Bauziel: Ziel und warum sie gerade wartet (Karl 22.10.2026: „die Bauschleife mit Bauziel funktioniert
  // nicht“ – sie wartete still auf Kristall, nichts war zu sehen).
  const serie=stueck?p.produktionsserien?.[(kat==='ground'?'boden':'einheit')+':'+x.key]:null;
  const serieZeile=serie?.ziel>0?`<small class="bs-kachel-serie${/^Wartet/i.test(serie.status||'')?' wartet':''}" title="${esc(serie.status||'')}">Ziel ${fmt(serie.ziel)} · ${esc(String(serie.status||'läuft').replace(/^Wartet:\s*/i,'wartet: ').replace(/ auf \d+:\d+:\d+/,''))}</small>`:'';
  const zeile=(job?`<div class="bs-kachel-job">${kat==='ships'||kat==='defenses'?`<small>noch ${fmt(job.rest||1)} Stück</small>`:''}${laeuft(kat,x)||kat==='buildings'||kat==='research'?'':jobLine(job)}</div>`:'')+serieZeile;
  let knopf='';
  if(kat==='buildings'||kat==='research'){
   const art=kat==='buildings'?'bau':'forschung',q=view.forschung.warteschlangen?.find(q=>q.art===art&&(art==='forschung'||q.planet===p.koord)),i=q?.jobs.findIndex(j=>j.key===x.key)??-1;
   const rep=kat==='buildings'?reparatur(p,x.key):null;
   if(i>=0){const j=q.jobs[i];knopf=j.aktiv?pauseButton(art,q.planet,j.manuell_pausiert??x.job?.manuell_pausiert,'bs-kachel-bauen bs-kachel-pause'):`<span class="bs-kachel-platz">Platz ${i+1}${j.stufe?' · Stufe '+j.stufe:''}<small>${esc(kernLesbar(String(j.grund||'wartet')))}</small></span>`;}
   else if(rep)knopf=rep.lauf?`<span class="bs-rep-lauf" role="status">Reparatur · noch <b data-arrival-echt="${esc(rep.lauf.fertig)}"></b></span>`
    :`${rep.kurz?`<small class="bs-kachel-fehlt">${esc(rep.kurz)}</small>`:''}<button type="button" class="bs-kachel-bauen bs-rep" data-bs-reparieren="${esc(x.key)}" ${rep.grund||lesen?'disabled':''} title="${esc(rep.grund||`Reparieren: Integrität ${rep.hp} % → 100 %`)}">Reparieren · ${rep.hp} %</button>`;
   else{const ziel=x.nextLevel||x.level+1,rl=kat==='buildings'?laufendeReparatur(p):null,frei=istBezahlbar(x,1,p)&&!rl;knopf=`<button type="button" class="bs-kachel-bauen${frei?'':' vormerken'}" ${x.action} ${gesperrt||lesen?'disabled':''} title="${esc(gesperrt?'Gesperrt: '+x.needs.join(' · '):rl?`Baustelle belegt: ${named(rl.gebaeude)} wird repariert – wird vorgemerkt und startet danach`:frei?'Sofort beginnen':'Rohstoffe fehlen noch – wird vorgemerkt und startet automatisch')}">${gesperrt?'Gesperrt':kat==='research'?(frei?'Forschen':'Vormerken'):(frei?(x.level?'Stufe '+ziel:'Bauen'):'Vormerken')}</button>`;}
  }else if(kat==='ground'){
   const ok=!gesperrt&&!lesen&&istBezahlbar(x,1,p);
   knopf=`<form class="bs-kachel-form" data-ground-build="${esc(x.key)}" data-planet="${esc(p.koord)}"><input type="hidden" name="anzahl" value="1"><button type="submit" class="bs-kachel-bauen" ${ok?'':'disabled'}>${gesperrt?'Gesperrt':'+1 ausbilden'}</button></form>`;
  }else{
   const ok=!gesperrt&&!lesen&&maxAnzahl(kat,x,p)>=1;
   // Eingeklappte Seiten (Werft): gesperrte Kacheln nennen statt „Gesperrt“ kurz den Grund, baubare sagen, was gerade fehlt.
   if(gesperrt&&EINKLAPPEN[seite])knopf=`<span class="bs-kachel-grund" title="${esc('Fehlt: '+x.needs.join(' · '))}">${esc(grund(x))}</span>`;
   else knopf=(!gesperrt&&!lesen&&!ok&&EINKLAPPEN[seite]?`<small class="bs-kachel-fehlt">${istBezahlbar(x,1,p)?'Besatzung fehlt':'Rohstoffe fehlen'}</small>`:'')+`<button type="button" class="bs-kachel-bauen" ${x.action} data-order-quantity="1" ${ok?'':'disabled'} title="${esc(gesperrt?'Gesperrt: '+x.needs.join(' · '):ok?'Ein Stück bauen – weitere reihen sich an':'Rohstoffe, Besatzung oder Werftplatz fehlen')}">${gesperrt?'Gesperrt':job?'+1 bauen':'Bauen'}</button>`;
  }
  const strom=kat==='buildings'&&!job&&!reparatur(p,x.key)&&typeof ausbauWirkungHtml==='function'?ausbauWirkungHtml(ausbauWirkung(x,p),true):'';
  // Kosten der nächsten Stufe bzw. je Stück auf jeder Kachel, Fehlendes rot (Karl 22.10.2026: „es darf doch nicht fehlen
  // wie Erz, Kristall oder was benötigt wird“); daneben die Wirkung auf die Strombilanz.
  const lager=x.stock||(kat==='research'?heimat()?.bestand:p.bestand)||{},kurz=n=>n>=1e6?fmt(Math.round(n/1e5)/10)+' Mio.':fmt(n);
  const repK=kat==='buildings'?reparatur(p,x.key):null,liste=repK?(repK.lauf?{}:repK.kosten):x.cost;
  const preise=job&&!repK?'':Object.entries(liste||{}).filter(([,v])=>v>0).map(([g,v])=>`<span class="bs-gut${(lager[g]||0)<v?' knapp':''}" title="${esc(named(g))}: ${fmt(v)}${repK?' Reparatur':stueck?' je Stück':''} · vorhanden ${fmt(Math.floor(lager[g]||0))}"><img src="${esc(art('resources',g,''))}" alt="${esc(named(g))}"><b>${kurz(v)}</b></span>`).join('');
  return `<div class="bs-kachel-unten">${zeile}${preise||strom?`<div class="bs-kachel-kosten">${preise}${strom}</div>`:''}${knopf}</div>`;
 }

 function maxAnzahl(kat,x,p){
  if(kat==='ships'||kat==='defenses')return maxBuildable(x);
  if(kat==='ground')return Math.max(0,Math.min(1000,...Object.entries(x.cost).map(([g,n])=>Math.floor(((x.stock||p.bestand)[g]||0)/n))));
  return 1;
 }
 function baubeginn(kat,x,n,p){
  if(x.needs.length)return ['gesperrt · '+x.needs.join(' · '),'rot'];
  // Besatzung kommt aus der Bevölkerung, 500 müssen bleiben (Kern) – sonst stünde „sofort“ neben gesperrtem Knopf.
  const crew=(kat==='ships'||kat==='defenses')?(x.r?.besatzung||0)*n:0,frei=Math.max(0,Math.floor(p.bevoelkerung||0)-500);
  if(crew>frei)return [`Besatzung fehlt · ${fmt(crew)} Personen nötig, ${fmt(frei)} frei`,'rot'];
  const quelle=kat==='research'?heimat():p,stock=x.stock||quelle.bestand;let warte=0,ohne='';
  for(const[g,v]of Object.entries(x.cost||{})){const fehlt=v*n-(stock[g]||0);if(fehlt<=0)continue;const rate=Number(quelle.rate?.[g])||0;if(rate<=0)ohne=named(g);else warte=Math.max(warte,fehlt/rate*3600);}
  if(ohne)return [`${ohne} fehlt – keine laufende Förderung`,'rot'];
  // Eine laufende Reparatur belegt die Baustelle bis zu ihrem Ende (Kern: „Wartet auf laufende Reparatur“).
  const rl=kat==='buildings'?laufendeReparatur(p):null;if(rl)warte=Math.max(warte,rl.rest);
  const belegt=kat==='buildings'?p.bauschleife.length>0:kat==='research'?!!view.forschung.aktiv:false;
  const nach=rl?` · nach der Reparatur von ${named(rl.gebaeude)}`:belegt?' · nach laufendem Auftrag':'';
  if(warte>0)return ['in '+dauer(warte/tempo())+nach,'gelb'];
  if(belegt)return ['nach laufendem Auftrag · wird vorgemerkt','gelb'];
  return ['sofort','gruen'];
 }
 function kosten(kat,x,n,p,zusatz=['ships','defenses','ground'].includes(kat)?' je Stück':''){
  const stock=x.stock||(kat==='research'?heimat().bestand:p.bestand);
  const teile=Object.entries(x.cost||{}).filter(([,v])=>v>0).map(([g,v])=>{const knapp=(stock[g]||0)<v*n;return `<span class="bs-gut${knapp?' knapp':''}" title="${esc(named(g))}: ${fmt(v)}${zusatz} · vorhanden ${fmt(Math.floor(stock[g]||0))}"><img src="${esc(art('resources',g,''))}" alt="${esc(named(g))}"><b>${fmt(v)}</b></span>`;});
  return teile.join('')||'<span class="bs-gut">kostenlos</span>';
 }
 function aktion(seite,x,n,p,ungueltig=false){
  const kat=SEITEN[seite].kat,lesen=!!gameReadOnly(),gesperrt=x.needs.length>0;
  if(kat==='buildings'||kat==='research'){
   const art=kat==='buildings'?'bau':'forschung',q=view.forschung.warteschlangen?.find(q=>q.art===art&&(art==='forschung'||q.planet===p.koord)),index=q?.jobs.findIndex(j=>j.key===x.key)??-1;
   const rep=kat==='buildings'&&index<0?reparatur(p,x.key):null;
   if(rep){
    const wirkung=rep.v2?'75 % der Baukosten der aktuellen Stufe, anteilig zum Schaden':'der beschädigte Anteil der Baukosten';
    return `<p class="bs-rep-info"><b>Beschädigt · Integrität ${rep.hp} %</b> – Produktion, Wohnraum und Boni sind vermindert. Reparatur: ${wirkung}; danach arbeitet es wieder voll.</p>${rep.lauf?`<p class="bs-imauftrag">In Arbeit · Reparatur auf 100 %</p>`:`${rep.grund?`<p class="bs-gesperrt">${esc(rep.grund)}</p>`:''}<button type="button" class="bs-bauen bs-rep" data-bs-reparieren="${esc(x.key)}" ${rep.grund||lesen?'disabled':''}>Reparieren · ${rep.hp} % → 100 %</button>`}`;
   }
   // Eingereiht: Platz, Kern-Grund, Umsortieren und die nächste Stufe gleich dahinter vormerken (Kern zählt geplante Stufen mit;
   // Funktionsprüfung 25.10.2026: bei laufender Forschung gab es keinen Weg zu Stufe N+1).
   if(index>=0){const j=q.jobs[index],ziel=x.nextLevel||x.level+1,mehr=x.needs;
    return `<p class="bs-imauftrag">${j.aktiv?'In Arbeit':'In der Warteschlange · Platz '+(index+1)}${j.stufe?' · Stufe '+j.stufe:''}${!j.aktiv&&j.grund?`<small>${esc(kernLesbar(String(j.grund)))}</small>`:''}</p><div class="bs-knopfreihe">${j.aktiv?pauseButton(art,q.planet,j.manuell_pausiert??x.job?.manuell_pausiert,'bs-zweit'):queueArrows(art,q.planet,q,index)}<button type="button" class="bs-zweit" data-queue-change="${art}" data-queue-planet="${esc(q.planet)}" data-index="${index}" data-remove>Abbrechen</button></div>${mehr.length?`<p class="bs-gesperrt">Stufe ${ziel} vormerken: ${esc(mehr.map(t=>kernLesbar(String(t))).join(' · '))}</p>`:`<button type="button" class="bs-bauen bs-zweitstufe" ${x.action} ${lesen?'disabled':''}>Stufe ${ziel} vormerken</button>`}`;}
   const ziel=x.nextLevel||x.level+1,wort=kat==='buildings'?(x.level?`Ausbauen auf Stufe ${ziel}`:'Bauen'):(x.level?`Forschen · Stufe ${ziel}`:'Forschen');
   return `${gesperrt?`<p class="bs-gesperrt">Gesperrt: ${esc(x.needs.map(t=>typeof kernLesbar==='function'?kernLesbar(String(t)):t).join(' · '))}</p>`:''}<button type="button" class="bs-bauen" ${x.action} ${gesperrt||lesen?'disabled':''}>${wort}</button>${kat==='buildings'&&typeof ausbauWirkungHtml==='function'?ausbauWirkungHtml(ausbauWirkung(x,p)):''}${!gesperrt&&!istBezahlbar(x,1,p)?'<small class="bs-vormerken">Rohstoffe fehlen noch: der Auftrag wird vorgemerkt und startet automatisch.</small>':''}`;
  }
  // Gesperrt: Grund nennen statt Anzahlfeld mit „[max. 7]“ (Konsolidierung M13, 19.10.2026)
  if(gesperrt)return `<p class="bs-gesperrt">Gesperrt: ${esc(x.needs.map(t=>typeof kernLesbar==='function'?kernLesbar(String(t)):t).join(' · '))}</p><button type="button" class="bs-bauen" disabled>${kat==='ground'?'Ausbilden':'Bauen'}</button>`;
  const max=maxAnzahl(kat,x,p),ok=!ungueltig&&!gesperrt&&!lesen&&n>=1&&n<=max&&Number.isInteger(n)&&istBezahlbar(x,n,p);
  const feld=`<label class="bs-anzahl">Anzahl <input id="bs-anzahl-${seite}" ${kat==='ground'?'name="anzahl"':''} type="number" min="1" max="${Math.max(1,max)}" value="${ungueltig?esc(String(menge[seite]??'')):n}" inputmode="numeric"></label><button type="button" class="bs-max" data-bs-max="${seite}" data-bs-maxwert="${max}">[max. ${fmt(max)}]</button>`;
  const warum=!ok&&!ungueltig&&!lesen?(n>max&&istBezahlbar(x,n,p)?`Höchstens ${fmt(max)} Stück${kat==='ground'?' je Auftrag':''}`:!istBezahlbar(x,n,p)?`Rohstoffe fehlen für ${fmt(n)} Stück`:n>max?`Höchstens ${fmt(max)} Stück`:''):'';
  const warumZeile=warum?`<small class="bs-vormerken">${esc(warum)}</small>`:'';
  if(kat==='ground')return `<form class="bs-formular" data-ground-build="${esc(x.key)}" data-planet="${esc(p.koord)}">${feld}<button type="submit" class="bs-bauen" ${ok?'':'disabled'}>Ausbilden</button></form>${ungueltig?'<small class="bs-vormerken">Mindestens 1 ganzes Stück eingeben.</small>':warumZeile}`;
  return `<div class="bs-formular">${feld}<button type="button" class="bs-bauen" ${x.action} data-order-quantity="${n}" ${ok?'':'disabled'}>Bauen</button></div>${ungueltig?'<small class="bs-vormerken">Mindestens 1 ganzes Stück eingeben.</small>':warumZeile}`;
 }
 // Technik v1: wirksamer Antrieb und nächster Antriebswechsel des Schiffs im Detailfeld.
 function antriebsfakt(kat,x){
  if(kat!=='ships')return '';const w=view.einheiten_kosten?.find(q=>q.einheit===x.key)?.werte;if(!w?.antriebswechsel?.length)return '';
  const naechste=w.antriebswechsel.find(s=>!s.erreicht);
  return `<dt>Antrieb</dt><dd>${esc(named(w.antrieb))}${naechste?` · ${esc(antriebsstufeText(naechste))}`:' · alle Antriebswechsel erreicht'}</dd>`;
 }
 function detail(seite,x,p){
  const s=SEITEN[seite],kat=s.kat,e=text(x.key),roh=menge[seite],zahl=Number(roh),ungueltig=['ships','defenses','ground'].includes(kat)&&roh!=null&&!(String(roh)!==''&&Number.isInteger(zahl)&&zahl>=1),n=['ships','defenses','ground'].includes(kat)&&!ungueltig?Math.max(1,Math.floor(zahl)||1):1;
  const rep=kat==='buildings'?reparatur(p,x.key):null,repZeit=rep&&!rep.lauf?repDauerLaden(p,x.key,rep.hp):null;
  let [start,klasse]=baubeginn(kat,x,n,p);const stueck=['ships','defenses','ground'].includes(kat);
  if(rep){[start,klasse]=rep.lauf?['läuft · bezahlt','gelb']:rep.grund?[rep.grund,'rot']:['sofort','gruen'];}
  if(stueck&&!rep&&!x.needs.length){const mx=maxAnzahl(kat,x,p);if(ungueltig)[start,klasse]=['Menge eingeben','rot'];else if(n>mx&&istBezahlbar(x,n,p))[start,klasse]=[`höchstens ${fmt(mx)} Stück${kat==='ground'?' je Auftrag':''}`,'rot'];}
  const zeit=rep?(rep.lauf?`noch <b data-arrival-echt="${esc(rep.lauf.fertig)}"></b>`:repZeit!=null?dauer(repZeit/tempo()):'wird berechnet …'):x.duration?dauer(x.duration/tempo()):'–',zeitTitel=rep?(repZeit!=null?GameTime.duration(repZeit)+' Spielzeit (mindestens 15 Spielminuten)':''):x.duration?GameTime.duration(x.duration)+' Spielzeit':'';
  const invLangsam=kat==='ground'&&invasionsFaktor(p)>1?` <small class="bs-vormerken">Invasion: ${Math.round(100-(Number(p.invasion.wellen.ausbildung_promille)||1000)/10)} % langsamer</small>`:'';
  const gesamt=stueck&&n>1&&x.duration?` · für ${fmt(n)} Stück ${dauer(x.duration*(kat==='ground'?Math.ceil(n/Math.max(1,Math.floor((p.gebaeude?.kaserne||0)/(x.plaetze||1)))):Math.ceil(n/Math.max(1,Math.floor((p.fertigungsplaetze?.werft?.plaetze||1)/(view.einheiten_kosten.find(q=>q.einheit===x.key)?.fertigungsplaetze_je_stueck||1)))))/tempo())}`:'';
  const wp=p.fertigungsplaetze?.werft,jeStueck=view.einheiten_kosten.find(q=>q.einheit===x.key)?.fertigungsplaetze_je_stueck||1;
  const werftZeile=(kat==='ships'||kat==='defenses')&&wp?`<dt>Werftplätze</dt><dd>Werft Stufe ${fmt(p.gebaeude?.werft||0)} = ${fmt(wp.plaetze)} Plätze · ${fmt(jeStueck)} je Stück → ${fmt(Math.floor(wp.plaetze/jeStueck))} gleichzeitig${wp.belegt?` · ${fmt(wp.belegt)} belegt`:''}</dd>`:'';
  const satz=e?.zweck||e?.rolle||ersterSatz(x.r?.wirkung);
  // Kosten und Zeiten gelten der nächsten freien Stufe; bei eingereihten Einträgen ist das nicht die laufende (Funktionsprüfung 25.10.2026).
  const stufeZu=!rep&&(kat==='buildings'||kat==='research')?' · Stufe '+(x.nextLevel||x.level+1):'';
  const extras=[kat==='buildings'&&x.key==='akademie'&&x.level>0?'<button type="button" class="bs-zweit" data-open-academy>Fachkräfte & Forscher ausbilden</button>':'',kat==='buildings'&&x.level>0?(inSchlange(kat,x,p)?`<button type="button" class="bs-zweit bs-abriss" disabled>Stufe ${x.level} abreißen</button><small class="bs-abriss-grund">In der Bauschleife – erst abbrechen oder fertigstellen, dann abreißen</small>`:`<button type="button" class="bs-zweit bs-abriss" data-demolish="${esc(x.key)}" data-demolish-planet="${esc(p.koord)}">Stufe ${x.level} abreißen</button>`):'',kat==='research'?`<button type="button" class="bs-zweit" data-bs-techinfo="${esc(x.key)}" data-bs-kat="research" data-bs-reiter="techbaum">Technologiebaum</button>`:''].filter(Boolean).join('');
  const schleife=(kat==='ships'||kat==='defenses')?Mobile035.seriesForm('einheit',x.key,p):kat==='ground'?Mobile035.seriesForm('boden',x.key,p):'';
  return `<article class="bs-detail${kat==='ships'?' quer':''}" id="bs-detail-${seite}" aria-label="${esc(x.name||named(x.key))}"><div class="bs-detail-bild"><img src="${esc(bildVon(kat,x))}" alt="${esc(x.name||named(x.key))}"><button type="button" class="bs-link" data-bs-techinfo="${esc(x.key)}" data-bs-kat="${kat}" data-bs-reiter="techbaum">Techbaum</button></div><div class="bs-detail-text"><div class="bs-detail-kopf"><h3>${esc(x.name||named(x.key))}</h3><span class="bs-stufe">${stueck?'Anzahl '+fmt(x.level):'Stufe '+x.level}</span><button type="button" class="bs-schliessen" data-bs-schliessen="${seite}" aria-label="Detailfeld schließen">✕</button></div><dl class="bs-fakten"><dt>${rep?'Reparaturzeit':kat==='ground'?'Dauer je Ausbildungsrunde':stueck?'Bauzeit je Stück':(kat==='research'?'Forschungszeit':'Bauzeit')+stufeZu}</dt><dd title="${esc(zeitTitel)}">${zeit}${rep?'':gesamt}${invLangsam}</dd>${werftZeile}<dt>${rep?(klasse==='rot'?'Reparatur':'Reparatur möglich'):(klasse==='rot'?'Baubeginn':'Baubeginn möglich')+stufeZu}</dt><dd class="bs-${klasse}">${esc(start)}</dd><dt>${rep?'Reparaturkosten':'Kosten'+(stueck?' je Stück':stufeZu)}</dt><dd class="bs-kosten">${rep?kosten(kat,{...x,cost:rep.kosten},1,p,' Reparatur'):kosten(kat,x,n,p)}</dd>${stueck&&!rep&&n>1?`<dt>Gesamt · ${fmt(n)} Stück</dt><dd class="bs-kosten">${kosten(kat,{...x,cost:Object.fromEntries(Object.entries(x.cost||{}).map(([g,v])=>[g,v*n]))},1,p,' gesamt')}${kat==='ground'&&x.personal?`<span class="bs-gut" title="Rekruten aus der Bevölkerung"><b>${fmt(n*x.personal)} Rekruten</b></span>`:''}</dd>`:''}${antriebsfakt(kat,x)}</dl><div class="bs-aktion">${aktion(seite,x,n,p,ungueltig)}</div><p class="bs-satz">${esc(satz)} <button type="button" class="bs-hilfe" data-bs-techinfo="${esc(x.key)}" data-bs-kat="${kat}" data-bs-reiter="techinfo" aria-label="Techinfo zu ${esc(x.name||named(x.key))}">?</button></p>${extras?`<div class="bs-knopfreihe">${extras}</div>`:''}${schleife}</div></article>`;
 }
 function banner(seite,p){const s=SEITEN[seite];return `<div class="bs-banner"><img class="bs-banner-bild" src="${esc(art(s.bild[0],s.bild[1],s.bild[0]==='backgrounds'?'':view.volk))}" alt=""><h2>${esc(s.titel)} – ${esc(planetName(p))} <span class="koord">[${esc(p.koord)}]</span></h2><p>${esc(s.satz)}</p></div>`;}

 // Drei gleiche Warteschlangenkästen: Gebäude · Forschung · Werft/Verteidigung (§2 Punkt 4).
 function kasten(titel,laufend,leer,ziel,zielName){
  return `<article class="bs-schlange"><h4>${titel}</h4>${laufend||`<p class="bs-leer">${leer} <button type="button" class="bs-link" data-tab="${ziel}">(${zielName})</button></p>`}</article>`;
 }
 function auftrag(kat,key,name,zusatz,job,knoepfe,weitere){return `<div class="bs-auftrag"><img src="${esc(art(BILDKAT[kat],key))}" alt=""><div><strong>${esc(name)}</strong><small>${esc(zusatz)}</small>${job?jobLine(job):''}<div class="bs-knopfreihe">${knoepfe}</div>${weitere||''}</div></div>`;}
 function schlangen(p){
  const qs=view.forschung.warteschlangen||[],qb=qs.find(q=>q.art==='bau'&&q.planet===p.koord),qf=qs.find(q=>q.art==='forschung'),qw=qs.find(q=>q.art==='werft'&&q.planet===p.koord);
  const b=p.bauschleife[0];
  const bau=b?auftrag('buildings',b.gebaeude,named(b.gebaeude),'Stufe '+(b.stufe||''),b,`${pauseButton('bau',p.koord,b.manuell_pausiert,'bs-zweit')}<button type="button" class="bs-zweit" data-queue-change="bau" data-queue-planet="${esc(p.koord)}" data-index="0" data-remove>Abbrechen</button>`,queueWaiting('bau',p.koord,qb)):qb?.jobs.length?queueWaiting('bau',p.koord,qb):'';
  const f=view.forschung.aktiv;
  const forschung=f?auftrag('research',f.forschung,named(f.forschung),'Stufe '+(f.stufe||(view.forschung.stufen[f.forschung]||0)+1),f,`${pauseButton('forschung',qf?.planet||heimat().koord,f.manuell_pausiert,'bs-zweit')}<button type="button" class="bs-zweit" data-queue-change="forschung" data-queue-planet="${esc(qf?.planet||'')}" data-index="0" data-remove>Abbrechen</button>`,queueWaiting('forschung',qf?.planet||heimat().koord,qf)):qf?.jobs.length?queueWaiting('forschung',qf.planet,qf):'';
  const w=p.fertigung.find(j=>j.schleife==='werft'),boden=(p.bodentruppen?.auftraege||[])[0];
  let werft='';
  if(w){const kat=rules.catalog.bauteile?.[w.produkt]?'bauteile':rules.catalog.einheiten[w.produkt]?.schiff?'ships':'defenses';werft=auftrag(kat,w.produkt,named(w.produkt),'noch '+fmt(w.rest)+' Stück',w,`${pauseButton('werft',p.koord,w.manuell_pausiert,'bs-zweit')}<button type="button" class="bs-zweit" data-queue-change="werft" data-queue-planet="${esc(p.koord)}" data-index="0" data-remove>Abbrechen</button>`,queueWaiting('werft',p.koord,qw));}
  else if(boden){const d=p.bodentruppen;werft=auftrag('ground',d.bilder?.[boden.art]||d.typen[boden.art],d.namen?.[boden.art]||named(d.typen[boden.art]),'noch '+fmt(boden.rest)+' Stück',boden.fertig_sekunden!=null?boden:null,`<button type="button" class="bs-zweit" data-ground-cancel="${esc(boden.id)}" data-planet="${esc(p.koord)}">Abbrechen · ${fmt(boden.rest-(boden.lose||[]).filter(b=>b.arbeit*4<=b.gesamt).reduce((s,b)=>s+b.anzahl,0))} von ${fmt(boden.rest)} erstatten</button>`,'');}
  return `<section class="bs-schlangen" aria-label="Warteschlangen">${kasten('Gebäude',bau,'Keine Gebäude im Bau.','versorgung','Zur Versorgung')}${kasten('Forschung',forschung,'Keine Forschung läuft.','forschen','Zur Forschung')}${kasten('Werft / Verteidigung',werft,'Keine Schiffe oder Verteidigung im Bau.','werft','Zur Schiffswerft')}</section>`;
 }

 // Wenn auf einer Seite noch nichts gehen kann, sagt ein Hinweis warum und führt mit einem Tipp zum fehlenden Bau
 // (Karl 19.10.2026: am Handy waren Forschung, Werft und Truppen nur graue Kacheln ohne Erklärung).
 // Wenn auf einer Seite noch nichts gehen kann: kleiner Hinweis-Chip mit Schritt (Karl 19.10.2026: keine Textkästen)
 function startHinweis(seite,p){
  const heim=view.planeten.find(x=>x.heimat)||p,g=k=>(p.gebaeude?.[k]||0),schritt=(ziel,text)=>`<button type="button" class="bs-schritt" data-bs-springe="${ziel}">${esc(text)} ›</button>`;
  const chip=(sym,kurz,lang,knopf)=>window.HinweisChip?HinweisChip(sym,kurz,lang,knopf):'';
  if(seite==='forschen'&&!(view.forschung?.punkte_je_stunde>0)){
   if(!(heim.gebaeude?.labor>0))return chip('labor','Ohne Labor keine Forschung','Das Labor auf der Heimatwelt erzeugt die Forschungspunkte – seine Stufe bestimmt auch, welche Forschungen möglich sind.',schritt('buildings:labor','Labor bauen'));
   return chip('labor','Labor ohne Forscher','Bilde Forscher aus (Produktion & Personal) oder baue eine Akademie.','<button type="button" class="bs-schritt" data-tab="kolonie">Ausbilden ›</button>');
  }
  if((seite==='werft'||seite==='verteidigung')&&!g('werft'))return chip('werft','Werft fehlt',(seite==='werft'?'Schiffe':'Geschütze und Schilde')+' entstehen in der Werft. Sie braucht zuerst einen Raumhafen.',g('raumhafen')?schritt('buildings:werft','Werft bauen'):schritt('buildings:raumhafen','Raumhafen bauen'));
  // Stufe I: alle Geschütze erst ab Stufe II – sagen warum und dass Neulinge bis dahin nicht angegriffen werden (lauf/bots.rs ist_neuling).
  const schutz=view.bot_schutz;
  if(seite==='verteidigung'&&(view.stufe||1)<2)return chip('werft','Verteidigung ab Stufe II','Geschütze und Schilde schaltet Zivilisationsstufe II frei. '+(schutz?.text||'Der Weg zur nächsten Stufe steht in der Übersicht.'),'<button type="button" class="bs-schritt" data-tab="reich">Zum Aufstieg ›</button>');
  if((seite==='verteidigung'||seite==='werft')&&schutz?.mitspieler&&!schutz.geschuetzt)return chip('werft','Ab jetzt können Bots angreifen','Vorwarnung mindestens 4 Spielstunden: Rohstoffe und Schiffe saven oder Geschütze bauen. '+(schutz.text||''),'<button type="button" class="bs-schritt" data-tab="flotten">Zur Flotte ›</button>');
  if(seite==='truppen'&&!g('kaserne'))return chip('kaserne','Kaserne fehlt','Bodentruppen bildet die Kaserne aus. Sie ist ab einer höheren Zivilisationsstufe baubar – der Aufstieg steht in der Übersicht.',schritt('buildings:kaserne','Kaserne'));
  return '';
 }
 function render(nur){
  if(!view||!rules?.catalog||session?.spieler==null)return;
  if(!geladen)void ladeTexte();
  const p=currentPlanet();if(!p)return;
  for(const seite of Object.keys(SEITEN)){
   if(nur&&nur!==seite)continue;
   const s=SEITEN[seite],host=$(s.host),panel=host?.closest('section');if(!host||panel?.hidden)continue;
   const items=eintraege(seite,p);if(!items.length&&s.kat!=='ground'){setStableHTML(host,banner(seite,p)+'<p class="bs-leer">Daten werden geladen …</p>');continue;}
   const x=items.find(i=>i.key===offen[seite]);if(!x)offen[seite]=null;
   if(['ships','defenses'].includes(s.kat))uiSelections[s.kat+'Qty']=Math.max(1,Number(menge[seite])||1);
   const sichtbar=i=>!i.needs.length||(['ships','defenses','ground'].includes(s.kat)&&(i.level>0||i.job))||!!inSchlange(s.kat,i,p);
   const zu=EINKLAPPEN[seite]?items.filter(i=>!sichtbar(i)):[],oben=zu.length?items.filter(sichtbar):items;
   const raster=gruppen(seite,oben).filter(([,l])=>l.length).map(([name,liste],i)=>`<section class="bs-gruppe"><h3 class="bs-gruppenkopf"><span>${esc(name)}</span><small>${esc(gruppenHinweis(seite,i,p))}</small></h3><div class="bs-raster${s.kat==='ships'?' quer':''}">${liste.map(it=>kachel(seite,it,p)).join('')}</div></section>`).join('')||`<p class="bs-leer">${zu.length?'Hier ist noch kein Eintrag freigeschaltet. Die Voraussetzungen stehen darunter.':'Noch keine Einträge verfügbar.'}</p>`;
   // Nächstliegende zuerst: wenigste fehlende Voraussetzungen oben (stabile Sortierung behält sonst die Katalogreihe).
   const rest=zu.length?`<details class="bs-zugeklappt" id="bs-zu-${seite}" data-bs-einklappen="${seite}"${merkOffen(seite)?' open':''}><summary><span>Noch nicht baubar (${fmt(zu.length)})</span><small>Fehlende Voraussetzungen</small></summary><div class="bs-raster${s.kat==='ships'?' quer':''}">${[...zu].sort((a,b)=>a.needs.length-b.needs.length).map(it=>kachel(seite,it,p)).join('')}</div></details>`:'';
   setStableHTML(host,`<div class="bs-kopf">${x?detail(seite,x,p):banner(seite,p)}</div>${seite==='truppen'||s.kat==='buildings'?invasionHinweis(p,seite):''}${startHinweis(seite,p)}${raster}${seite==='werft'?bauteilKarte(p):''}${rest}${schlangen(p)}`);
  }
 }
 // Habitatmodul in der Werft (Karl 24.10.2026: „das Recycler-Umbauen funktioniert nicht – es gibt keine Funktion in der Werft
 // dafür“; „die Habitatmodule sollten von Tag 1 bereitliegen“). Kern wirtschaft.rs fertigen: ohne Orbitalwerft ab Stufe III und
 // Werft 3 je Modul ein vorhandener Recycler + Bauteilkosten, mit Orbitalwerft dort ohne Recycler. Neue Welten haben 2 Module ab
 // Start im Lager (online-v1.ron start_bestand); das kleine Kolonieschiff braucht je eines.
 function bauteilKarte(p){
  const c=rules.catalog,kosten=c.bauteile?.habitatmodul;if(!kosten||!c.einheiten?.kolonieschiff)return '';
  const lager=Math.floor(p.bestand?.habitatmodul||0),jeSchiff=Number(c.einheiten.kolonieschiff.regel?.kosten?.habitatmodul??c.einheiten.kolonieschiff.kosten?.habitatmodul)||1;
  const orbital=(p.gebaeude?.orbitalwerft||0)>0,recycler=Math.floor(p.schiffe?.recycler||0),lesen=!!gameReadOnly();
  const fehlt=orbital?[]:[...((view.stufe||1)<3?['Zivilisationsstufe III']:[]),...((p.gebaeude?.werft||0)<3?['Werft 3']:[])];
  const reicht=Object.entries(kosten).filter(([,v])=>v>0).map(([g,v])=>Math.floor((p.bestand?.[g]||0)/v));
  const max=Math.max(0,Math.min(orbital?10000:recycler,...reicht));
  const job=p.fertigung?.find(j=>j.produkt==='habitatmodul');
  const chips=Object.entries(kosten).filter(([,v])=>v>0).map(([g,v])=>`<span class="bs-gut${(p.bestand?.[g]||0)<v?' knapp':''}" title="${esc(named(g))}: ${fmt(v)} je Stück · vorhanden ${fmt(Math.floor(p.bestand?.[g]||0))}"><img src="${esc(art('resources',g,''))}" alt="${esc(named(g))}"><b>${fmt(v)}</b></span>`).join('')
   +(orbital?'':`<span class="bs-gut${recycler<1?' knapp':''}" title="Recycler: 1 je Stück · vorhanden ${fmt(recycler)}"><img src="${esc(art('ships','recycler'))}" alt="Recycler"><b>1</b></span>`);
  const grundText=fehlt.length?`Umbau ab ${fehlt.join(' und ')}`:!orbital&&recycler<1?'Kein Recycler auf diesem Planeten – zuerst einen Recycler bauen':max<1?'Rohstoffe fehlen für den Umbau':'';
  const knoepfe=grundText?`<p class="bs-bauteil-grund">${esc(grundText)}</p>${!fehlt.length&&!orbital&&recycler<1&&c.einheiten.recycler?'<button type="button" class="bs-zweit" data-bs-seite="werft" data-bs-key="recycler">Recycler ansehen</button>':''}`
   :`<div class="bs-knopfreihe"><button type="button" class="bs-kachel-bauen bs-bauteil-los" data-bs-bauteil="habitatmodul" data-bs-anzahl="1" ${lesen?'disabled':''}>${orbital?'1 Habitatmodul fertigen':'1 Recycler umbauen'}</button>${max>1?`<label class="bs-anzahl">Anzahl <input id="bs-bauteil-anzahl" type="number" min="1" max="${max}" value="${Math.min(max,Math.max(1,Number(menge.bauteil)||max))}" inputmode="numeric"></label><button type="button" class="bs-zweit" data-bs-bauteil="habitatmodul" data-bs-anzahl-feld="bs-bauteil-anzahl" data-bs-maxwert="${max}" ${lesen?'disabled':''}>${orbital?'Anzahl fertigen':'Anzahl umbauen'} (max. ${fmt(max)})</button>`:''}</div>`;
  return `<section class="bs-gruppe bs-bauteile" aria-label="Bauteile"><h3 class="bs-gruppenkopf"><span>Bauteile für Kolonieschiffe</span><small>[Habitatmodule im Lager: ${fmt(lager)}]</small></h3>
   <article class="bs-bauteil"><img class="bs-bauteil-bild" src="${esc(art('resources','habitatmodul',''))}" alt=""><div class="bs-bauteil-text">
    <div class="bs-bauteil-kopf"><b>Habitatmodul</b><span class="bs-stufe">Lager ${fmt(lager)}</span></div>
    <p class="bs-bauteil-satz">Je kleines Kolonieschiff ${fmt(jeSchiff)}. ${lager>=jeSchiff?`Reicht für ${fmt(Math.floor(lager/jeSchiff))} Kolonieschiff${Math.floor(lager/jeSchiff)===1?'':'e'}.`:'Für das nächste Kolonieschiff fehlt eins.'} ${orbital?'Die Orbitalwerft fertigt weitere ohne Recycler.':'Weitere entstehen, indem die Werft einen Recycler umbaut.'}</p>
    ${job?`<div class="bs-kachel-job"><small>${esc(orbital&&job.schleife!=='werft'?'Orbitalwerft':'Werft')}: noch ${fmt(job.rest||1)} Stück</small>${typeof jobLine==='function'?jobLine(job):''}</div>`:''}
    <div class="bs-kachel-kosten">${chips}</div>${knoepfe}</div></article></section>`;
 }
 function oeffne(seite,key){offen[seite]=key;menge[seite]=menge[seite]||1;render(seite);requestAnimationFrame(()=>{const d=$('bs-detail-'+seite);if(d&&d.getBoundingClientRect().top<0)d.scrollIntoView({block:'start'});});}
 function seiteVon(kat,key){return kat==='buildings'?(VERSORGUNG.includes(key)?'versorgung':'gebaeude'):SEITE_JE_KAT[kat];}

 // ---- Techinfo-Fenster mit vier Reitern (§3) ----
 function datensatz(kat,key){if(kat==='ships'||kat==='defenses')return itemRecords(kat,currentPlanet()).find(x=>x.key===key)||null;const seite=seiteVon(kat,key);if(!seite)return null;return eintraege(seite).find(x=>x.key===key)||null;}
 function voraussetzungen(kat,key){
  const c=rules.catalog,p=currentPlanet(),out=[],add=(k,schluessel,need,have)=>{if(need>0)out.push({kat:k,key:schluessel,need,have});};
  if(kat==='buildings')for(const[k,n]of Object.entries(c.gebaeude[key]?.braucht||{}))add('buildings',k,n,p.gebaeude[k]||0);
  if(kat==='research'){const r=c.forschung[key],v=c.forschung_voraussetzungen?.[key];if(r)add('buildings','labor',r.labor,heimat().gebaeude.labor||0);for(const[k,n]of Object.entries(v?.forschung||{}))add('research',k,n,view.forschung.stufen[k]||0);for(const[k,n]of Object.entries(v?.gebaeude_reich||{}))add('buildings',k,n,Math.max(0,...view.planeten.map(x=>x.gebaeude[k]||0)));}
  if(kat==='ships'||kat==='defenses'){const e=c.einheiten[key];if(e){add('buildings','werft',Math.max(kat==='ships'?1:0,e.regel.werft||0),p.gebaeude.werft||0);for(const[k,n]of Object.entries(e.taktik?.voraussetzungen||{}))add('research',k,n,view.forschung.stufen[k]||0);for(const[k,n]of Object.entries(e.regel.braucht||{}))add('buildings',k,n,p.gebaeude[k]||0);}}
  if(kat==='ground'){const d=p.bodentruppen,i=d?.typen.indexOf(key)??-1;if(i>=0)add('buildings','kaserne',d.kaserne[i],p.gebaeude.kaserne||0);}
  return out;
 }
 function stufeVon(kat,key){const c=rules.catalog;if(kat==='buildings')return c.gebaeude[key]?.ab_stufe;if(kat==='research')return c.forschung[key]?.ab_stufe;if(kat==='ground'){const d=currentPlanet().bodentruppen;return d?.ab_stufe[d.typen.indexOf(key)];}return c.einheiten[key]?.regel.ab_stufe;}
 function baum(kat,key,tiefe,gesehen,nurPunkte=false){
  const rows=voraussetzungen(kat,key);if(!rows.length)return '';
  const punkte=rows.map(x=>{const id=x.kat+':'+x.key,ok=x.have>=x.need,unter=tiefe<5&&!gesehen.has(id)?baum(x.kat,x.key,tiefe+1,new Set([...gesehen,id])):'';return `<li class="${ok?'erfuellt':'fehlt'}"><button type="button" data-bs-springe="${id}"><img src="${esc(art(BILDKAT[x.kat],x.key))}" alt="">${esc(named(x.key))}</button><b>${fmt(x.have)} / ${fmt(x.need)}</b>${unter}</li>`;}).join('');
  return nurPunkte?punkte:`<ul class="ti-baum">${punkte}</ul>`;
 }
 function anwendungen(kat,key){
  const c=rules.catalog,out=[];
  if(kat==='buildings'){
   for(const[k,r]of Object.entries(c.gebaeude))if(r.braucht?.[key])out.push(['buildings',k,r.braucht[key]]);
   for(const[k,e]of Object.entries(c.einheiten)){const lvl=key==='werft'?(e.schiff?Math.max(1,e.regel.werft||0):e.regel.werft||0):e.regel.braucht?.[key];if(lvl)out.push([e.schiff?'ships':'defenses',k,lvl]);}
   if(key==='labor')for(const[k,r]of Object.entries(c.forschung))out.push(['research',k,r.labor]);
   for(const[k,v]of Object.entries(c.forschung_voraussetzungen||{}))if(v.gebaeude_reich?.[key])out.push(['research',k,v.gebaeude_reich[key]]);
   if(key==='kaserne'){const d=currentPlanet().bodentruppen;(d?.typen||[]).forEach((t,i)=>out.push(['ground',t,d.kaserne[i],d.namen?.[i],d.bilder?.[i]]));}
   if(key==='raketensilo')(c.raketen||[]).forEach(r=>out.push(['missiles',r.key,r.silo,r.name]));
  }
  if(kat==='research'){
   for(const[k,e]of Object.entries(c.einheiten))if(e.taktik?.voraussetzungen?.[key])out.push([e.schiff?'ships':'defenses',k,e.taktik.voraussetzungen[key]]);
   for(const[k,v]of Object.entries(c.forschung_voraussetzungen||{}))if(v.forschung?.[key])out.push(['research',k,v.forschung[key]]);
   (c.raketen||[]).forEach(r=>{if(r.braucht?.[key])out.push(['missiles',r.key,r.braucht[key],r.name]);});
  }
  return out.sort((a,b)=>a[2]-b[2]);
 }
 function technischeDaten(kat,x){
  const zeilen=[];const z=(n,v)=>zeilen.push(`<tr><th>${esc(n)}</th><td>${v}</td></tr>`);
  if(kat==='ships'||kat==='defenses'){const r=x.r,w=view.einheiten_kosten.find(q=>q.einheit===x.key)?.werte||{};const v=k=>fmt(w[k]??r[k]??0)+(w[k]!=null&&w[k]!==r[k]?` <small>(Basis ${fmt(r[k]||0)})</small>`:'');
   z('Struktur',v('struktur'));z('Schild',v('schild'));z('Angriff',v('angriff'));if(kat==='ships'){z('Geschwindigkeit',v('tempo'));z('Ladekapazität',v('ladung'));z('Treibstoffverbrauch',fmt(r.verbrauch||0));}z('Besatzung',fmt(r.besatzung||0));if(r.max_anzahl)z('Höchstbestand je Planet',fmt(r.max_anzahl));}
  else if(kat==='buildings'){const st=buildingStages(x),jetzt=st[Math.min(x.level,st.length-1)],next=st[Math.min(x.level+1,st.length-1)];if(jetzt){z(stageOutputLabel(x.key)+' (jetzt → nächste Stufe)',`${fmt(jetzt.leistung)} → ${fmt(next.leistung)}`);z('Energiebedarf',`${fmt(jetzt.energie)} → ${fmt(next.energie)}`);z('Arbeitskräfte',`${fmt(jetzt.arbeiter)} → ${fmt(next.arbeiter)}`);if(next.fachkraefte)z('Fachkräfte',`${fmt(jetzt.fachkraefte)} → ${fmt(next.fachkraefte)}`);z('Höchste Stufe',fmt(st.length-1));}}
  else if(kat==='research'){const r=x.r;z('Forschungspunkte nächste Stufe',fmt(Math.round(r.fp*r.faktor**Math.max(0,x.level))));z('Labor auf der Heimatwelt',fmt(r.labor));z('Wirkung',esc(r.wirkung));}
  else if(kat==='ground'){const d=currentPlanet().bodentruppen,i=x.i;z('Angriff',fmt(d.angriff[i]));z('Struktur',fmt(d.struktur[i]));z('Transportplätze',fmt(d.truppenplaetze[i]));z('Rekruten je Einheit',fmt(d.personal[i]));z('Fertigungsplätze je Einheit',fmt(d.fertigungsplaetze_je_stueck[i]));z('Ausbildungszeit je Stück',x.duration?dauer(x.duration/tempo()):'Kaserne pausiert');}
  return zeilen.length?`<table class="ti-daten"><caption>Technische Daten</caption><tbody>${zeilen.join('')}</tbody></table>`:'';
 }
 function schnellfeuer(kat,key){
  if(!['ships','defenses'].includes(kat))return '';
  const c=rules.catalog,e=c.einheiten[key],gegen=Object.entries(e.zielprofil?.schnellfeuer||{}).filter(([,n])=>n>0);
  const von=Object.entries(c.einheiten).map(([k,x])=>[k,x.zielprofil?.schnellfeuer?.[key]||0]).filter(([,n])=>n>0);
  const li=(cls,wort,[k,n])=>`<li class="${cls}"><button type="button" data-bs-springe="${c.einheiten[k]?.schiff?'ships':'defenses'}:${esc(k)}">${wort} ${esc(named(k))}</button><b>${fmt(n)}</b></li>`;
  return `<div class="ti-sf"><h4>Schnellfeuer</h4>${gegen.length||von.length?`<ul>${von.map(x=>li('sf-von','Schnellfeuer von',x)).join('')}${gegen.map(x=>li('sf-gegen','Schnellfeuer gegen',x)).join('')}</ul>`:'<p>Kein Schnellfeuer von oder gegen diese Einheit.</p>'}</div>`;
 }
 // Technische Zeichnungen je Volk (Katalogkategorie "blueprints", tools/content/blaupausen_katalog.py): eine große
 // Zeichnung, Umschalter Blaupause / Explosion / Schnitt; Motiv nie beschnitten (object-fit: contain in rahmen.css).
 const ZEICHNUNG=[['blaupause','Blaupause','Blaupause'],['explosion','Explosionszeichnung','Explosion'],['schnitt','Schnittzeichnung','Schnitt']];
 function zeichnungen(kat,x){
  if(kat!=='ships'&&kat!=='ground')return '';
  const bilder=window.STERNEN_ART?.images||{},volk=view?.volk||'';
  let [k,schluessel]=kat==='ground'?['ground_units',x.bild]:['ships',x.key];
  const alias=window.STERNEN_ART?.aliases?.[`${k}|${schluessel}`];if(alias)[k,schluessel]=alias.split('|');
  const da=ZEICHNUNG.map(([s,t,kurz])=>[s,t,bilder[`blueprints|${s}.${k}.${schluessel}|${volk}`],kurz]).filter(z=>z[2]);
  if(!da.length)return '';
  const wahl=da.find(z=>z[0]===ti.zeichnung)||da[0],name=x.name||named(x.key);
  return `<section class="ti-zeichnungen"><h4>Technische Zeichnungen</h4><div class="ti-zeichnung-wahl" role="group" aria-label="Zeichnung wählen">${da.map(([s,t,,kurz])=>`<button type="button" data-ti-zeichnung="${s}" aria-pressed="${s===wahl[0]}" title="${t}">${kurz}</button>`).join('')}</div><figure class="ti-zeichnung"><img src="${esc(wahl[2].url)}" alt="${esc(wahl[1]+': '+name)}" decoding="async"><figcaption>${esc(wahl[1])} · ${esc(name)}${VOLKNAME[volk]?' · Bauart '+VOLKNAME[volk]:''}</figcaption></figure></section>`;
 }
 function technik(kat,x){
  if(kat==='buildings'){const st=buildingStages(x);if(!st.length)return '<p>Ausbaudaten werden vom Server geladen.</p>';const von=Math.max(1,x.level-5),bis=Math.min(st.length-1,x.level+5),hat=st.some(s=>s.leistung>0);
   return `<table class="ti-stufen"><thead><tr><th>Stufe</th><th>Kosten</th>${hat?`<th>${esc(stageOutputLabel(x.key))}</th><th>Zuwachs</th>`:''}<th>Energie</th></tr></thead><tbody>${st.slice(von,bis+1).map(s=>`<tr class="${s.stufe===x.level?'aktuell':s.stufe===x.level+1?'naechste':''}"><th>${s.stufe}</th><td>${stageCosts(s.kosten)}</td>${hat?`<td>${fmt(s.leistung)}</td><td>${s.stufe?(s.leistung-st[s.stufe-1].leistung>=0?'+':'')+fmt(s.leistung-st[s.stufe-1].leistung):'–'}</td>`:''}<td>${fmt(s.energie)}</td></tr>`).join('')}</tbody></table>`;}
  if(kat==='research'){const r=x.r,von=Math.max(1,x.level-5),bis=Math.min(rules.catalog.welt.max_forschungsstufe||30,x.level+5),rows=[];for(let l=von;l<=bis;l++)rows.push(`<tr class="${l===x.level?'aktuell':l===x.level+1?'naechste':''}"><th>${l}</th><td>${stageCosts(Object.fromEntries(Object.entries(r.kosten).map(([g,n])=>[g,Math.floor(n*r.faktor**(l-1))])))}</td><td>${fmt(Math.round(r.fp*r.faktor**(l-1)))}</td></tr>`);return `<table class="ti-stufen"><thead><tr><th>Stufe</th><th>Kosten</th><th>Forschungspunkte</th></tr></thead><tbody>${rows.join('')}</tbody></table>`;}
  if(kat==='ships'||kat==='defenses')return zeichnungen(kat,x)+progressionTable(kat,x)+`<details class="ti-mehr"><summary>Vollständiges Datenblatt</summary>${dataSheet(kat,x)}</details>`;
  if(kat==='ground')return zeichnungen(kat,x)+`<p>Bodentruppen haben keine Ausbaustufen. Kosten je Stück: </p><div class="bs-kosten">${kosten('ground',x,1,currentPlanet())}</div>`;
  return '';
 }
 // Archiv (docs/LORE-BIBEL.md §8): technisches Dossier, Chronik, vier Volksakten (eigenes Volk zuerst offen), Verweise.
 const VOLKNAME={aurelianer:'Aurelianer',krath:'Krath',syntheten:'Syntheten',veyari:'Veyari'};
 const ARCHIVKAT=['buildings','research','ships','defenses','ground'];
 function archiv(key){
  const a=text(key)?.archiv;if(!a||typeof a!=='object')return '<p class="ti-hinweis">Zu diesem Eintrag gibt es keinen Archiveintrag.</p>';
  const eigen=VOLKNAME[view?.volk]?view.volk:'aurelianer',reihe=[eigen,...Object.keys(VOLKNAME).filter(v=>v!==eigen)];
  const akte=(v,i)=>a[v]?`<details class="ti-akte" data-volk="${v}"${i===0?' open':''}><summary>${VOLKNAME[v]}-Akte${v===eigen?' <small>dein Volk</small>':''}</summary>${absaetze(a[v])}</details>`:'';
  const verweis=k=>{const kat=ARCHIVKAT.find(c=>datensatz(c,k)),nm=text(k)?.name||named(k);return `<li>${kat?`<button type="button" data-ti-archiv="${kat}:${esc(k)}">${esc(nm)}</button>`:`<span>${esc(nm)}</span>`}</li>`;};
  const v=Array.isArray(a.verweise)?a.verweise:[];
  // Lange Felder in Absätze zu je etwa drei Sätzen teilen (Lesbarkeit im Fenster)
  const absaetze=t=>{const s=String(t).split(/(?<=[.!?“])\s+(?=[A-ZÄÖÜ„])/),n=s.length>4?3:s.length,out=[];for(let i=0;i<s.length;i+=n)out.push(`<p>${esc(s.slice(i,i+n).join(' '))}</p>`);return out.join('');};
  const kapitel=(titel,t)=>{const ps=absaetze(t).split('</p>').filter(Boolean).map(p=>p+'</p>');return `<section><h4>${titel}</h4>${ps[0]||''}${ps.length>1?`<details class="ti-weiter"><summary>Weiterlesen · ${ps.length-1} ${ps.length>2?'Absätze':'Absatz'}</summary>${ps.slice(1).join('')}</details>`:''}</section>`;};
  return `<div class="ti-archiv">${a.technik?kapitel('Technisches Dossier',a.technik):''}${a.geschichte?kapitel('Chronik',a.geschichte):''}<section><h4>Volksakten</h4>${reihe.map(akte).join('')}</section>${v.length?`<section><h4>Siehe auch</h4><ul class="ti-verweise">${v.map(verweis).join('')}</ul></section>`:''}</div>`;
 }
 let ti={kat:'',key:'',reiter:'techinfo'};
 // Reiter „Techbaum“: immer der neue Gesamtbaum, auf dieses Objekt gezoomt (Karl 19.10.); Schließen führt zurück zum
 // Objekt auf dem vorigen Reiter. Nur wenn der Baum nicht lädt, bleibt die Liste als Ersatz.
 function neuerBaum(key,zurueck){
  const d=$('techinfo'),vorher=d?.open?d.querySelector('[role=tab][aria-selected="true"]'):null,reiter=vorher?.dataset.tiReiter||vorher?.dataset.mondTi;
  if(!window.TechbaumSpiel?.zeigeObjekt)return Promise.resolve(false);
  if(d?.open)d.close();
  return window.TechbaumSpiel.zeigeObjekt(key,vorher?()=>zurueck(reiter&&reiter!=='techbaum'?reiter:'techinfo'):null);
 }
 function techinfo(kat,key,reiter=ti.reiter){
  const x=datensatz(kat,key);if(!x)return false;
  if(reiter==='techbaum'&&!ti.listenbaum){neuerBaum(key,r=>techinfo(kat,key,r)).then(ok=>{if(!ok){ti.listenbaum=true;techinfo(kat,key,'techbaum');}});return;}
  ti={kat,key,reiter,zeichnung:ti.zeichnung};void ladeTexte();
  let d=$('techinfo');if(!d){document.body.insertAdjacentHTML('beforeend','<dialog id="techinfo" class="techinfo" aria-labelledby="techinfo-titel"></dialog>');d=$('techinfo');}
  // Infodisplay statt Reiterkasten (Karl 19.10.): Bühne mit Motiv, Kennwerte, Tafeln zum Wischen; „Techbaum ⤢“ öffnet den
  // Gesamtbaum. Die Liste der Voraussetzungen bleibt nur als Ersatz, wenn der Gesamtbaum nicht lädt (ti.listenbaum).
  const e=text(key),name=x.name||named(key),ab=stufeVon(kat,key);
  const tafeln=[['techinfo','Überblick',ueberblick(kat,key,x,e)],['technik','Technik',technikTafel(kat,x)],['anwendungen','Freischaltungen',freischaltungen(kat,key,x)],['archiv','Archiv',archiv(key)]];
  if(reiter==='techbaum'){const zivil=ab?`<li class="${view.stufe>=ab?'erfuellt':'fehlt'}"><span>Zivilisationsstufe</span><b>${view.stufe} / ${ab}</b></li>`:'';tafeln.unshift(['techbaum','Pfad',`<p class="ti-hinweis">Grün erfüllt, rot fehlt. Ein Klick springt zur Kachel.</p><ul class="ti-baum ti-wurzel"><li class="${x.needs.length?'fehlt':'erfuellt'}"><span><img src="${esc(bildVon(kat,x))}" alt="">${esc(name)}</span>${x.needs.length?`<b>${esc(x.needs.join(' · '))}</b>`:'<b>Voraussetzungen erfüllt</b>'}<ul class="ti-baum">${zivil}${baum(kat,key,1,new Set([kat+':'+key]),true)}</ul></li></ul>`]);}
  anzeige(d,{kat,name,augenbraue:[KATNAME[kat],e?.klasse,e?.rolle].filter(Boolean).join(' · '),
   status:['ships','defenses','ground'].includes(kat)?`Bestand <b>${fmt(x.level)}</b>`:`Stufe <b>${fmt(x.level)}</b>`,
   sperre:x.needs||[],bild:bildVon(kat,x),werte:kennwerte(kat,x),tafeln,aktiv:reiter,leiste:`Tech-Datenbank · ${VOLKNAME[view?.volk]||'Reich'}`});
  return true;
 }
 // ---- Infodisplay (Karl 19.10.: „die Infobox ist noch aus 2000 … wie ein digitales Display in der Schiffsinnenansicht,
 // Infos zum Wischen, modern und nativ, nicht hakelig … alles da, aber eingeklappt oder als clevere Elemente, nicht
 // oberflächlich“). Bühne: Motiv auf Holo-Grund, Klasse, Name, Bestand/Stufe, Sperre; Kennwerte als Messleisten (Rang in der
 // eigenen Klasse). Tafeln liegen nebeneinander in einer CSS-Scroll-Snap-Bahn (natives Wischen mit Schwung, kein
 // Gestencode); Reiter und Zeiger folgen der Bahn. Gemeinsam für Bauseiten und Mond (mond.js) – Stile in dossier.css.
 const KATNAME={buildings:'Gebäude',research:'Forschung',ships:'Raumschiff',defenses:'Verteidigung',ground:'Bodentruppe'};
 function messleiste(anteil){const a=Math.max(0,Math.min(1,Number(anteil)||0));return `<svg class="td-mess" viewBox="0 0 100 4" preserveAspectRatio="none" aria-hidden="true"><rect class="bahn" width="100" height="4" rx="2"/><rect class="wert ${a>=.75?'hoch':a>=.4?'mittel':'tief'}" width="${(a*100).toFixed(1)}" height="4" rx="2"/></svg>`;}
 function logbuch(l){if(!l)return '';const satz=String(l).split(/(?<=[.!?…])\s+/)[0],anfang=satz.length>140?satz.slice(0,138).replace(/\s+\S*$/,'')+' …':satz;
  return `<details class="td-logbuch"><summary><span>Logbuch</span><q>${esc(anfang)}</q></summary><blockquote>${esc(l)}</blockquote></details>`;}
 function anzeige(d,o){
  const tafeln=o.tafeln.filter(t=>t[2]),i=Math.max(0,tafeln.findIndex(t=>t[0]===o.aktiv)),knopf=o.knopf||(id=>`data-ti-reiter="${id}"`);
  const reiter=tafeln.map(([id,t],n)=>`<button type="button" role="tab" ${knopf(id)} data-tafel-ziel="${n}" id="td-r-${id}" aria-controls="td-t-${id}" aria-selected="${n===i}">${esc(t)}</button>`).join('');
  const baumKnopf=tafeln.some(t=>t[0]==='techbaum')?'':`<button type="button" ${knopf('techbaum')} class="td-baum" title="Gesamter Technologiebaum, auf dieses Objekt gezoomt"><span aria-hidden="true">⤢</span> Techbaum</button>`;
  const werte=(o.werte||[]).map(w=>`<div class="td-wert"${w.titel?` title="${esc(w.titel)}"`:''}><dt>${esc(w.t)}</dt><dd><b>${esc(w.v)}</b>${w.dann!=null?`<small>→ ${esc(w.dann)}</small>`:''}${w.anteil==null?'':messleiste(w.anteil)}</dd></div>`).join('');
  const objekt=o.kat+':'+o.name,neuesObjekt=d.dataset.tdObjekt!==objekt;d.dataset.tdObjekt=objekt;
  setStableHTML(d,`<div class="td td-${esc(o.kat)}"><div class="td-kopf"><div class="td-leiste"><span>${esc(o.leiste||'Tech-Datenbank')}</span><button type="button" class="td-zu" data-ti-schliessen aria-label="Schließen">✕</button></div>`
   +`<div class="td-buehne"><img src="${esc(o.bild)}" alt="${esc(o.name)}" decoding="async">${baumKnopf}<div class="td-titel"><p class="td-augenbraue">${esc(o.augenbraue||'')}</p><h2 id="techinfo-titel">${esc(o.name)}</h2><p class="td-status">${o.status||''}</p></div></div>`
   +`${o.sperre?.length?`<p class="td-sperre"><b>Gesperrt</b><span>${esc(o.sperre.join(' · '))}</span></p>`:''}${werte?`<dl class="td-werte">${werte}</dl>`:''}</div>`
   +`<nav class="td-reiter" role="tablist" aria-label="Tafeln">${reiter}<span class="td-zeiger" aria-hidden="true"></span></nav>`
   +`<div class="td-bahn">${tafeln.map(([id,t,html])=>`<section class="td-tafel" role="tabpanel" id="td-t-${id}" aria-labelledby="td-r-${id}" data-tafel="${id}">${html}</section>`).join('')}</div></div>`);
  d.classList.add('td-dialog');
  // Neues Objekt: alle Tafeln oben, Bühne offen (setStableHTML behält sonst die Lage der vorigen Tafeln)
  if(neuesObjekt||!d.open){d.querySelectorAll('.td-tafel').forEach(t=>{t.scrollTop=0;t._vor=0;});d.classList.remove('td-klein');}
  if(!d.open)d.showModal();
  const b=d.querySelector('.td-bahn');if(b)b.scrollLeft=i*b.clientWidth;tafelLage(d);
  // Bahn: Reiter/Zeiger folgen; Tafel nach unten gescrollt: Bühne klappt zur schmalen Titelleiste ein (oben wieder auf)
  // Einklappen nur, wenn die Tafel danach noch deutlich länger als die Ansicht ist (sonst Flackern durch Begrenzen der
  // Scrolllage); Aufklappen beim Zurückscrollen bis oben, beim Wechsel auf eine kurze Tafel oder per Tipp auf die Leiste.
  if(!d.dataset.tdLage){d.dataset.tdLage='1';
   d.addEventListener('scroll',ev=>{const t=ev.target,klein=d.classList.contains('td-klein');
    if(t.classList?.contains('td-bahn')){tafelLage(d);const a=d.querySelector('.td-tafel.aktiv');if(klein&&a&&a.scrollHeight-a.clientHeight<60&&a.scrollTop<4)d.classList.remove('td-klein');return;}
    if(!t.classList?.contains('td-tafel'))return;
    if(!klein&&t.scrollTop>40&&t.scrollHeight-t.clientHeight>320)d.classList.add('td-klein');else if(klein&&t.scrollTop<4&&(t._vor??0)>t.scrollTop)d.classList.remove('td-klein');t._vor=t.scrollTop;},{capture:true,passive:true});
   d.addEventListener('click',ev=>{if(d.classList.contains('td-klein')&&ev.target.closest('.td-buehne')&&!ev.target.closest('button')){d.classList.remove('td-klein');const a=d.querySelector('.td-tafel.aktiv');if(a){a.scrollTop=0;a._vor=0;}}});}
 }
 // Reiter, Zeiger und aktive Tafel aus der Lage der Bahn (auch mitten im Wischen: der Zeiger gleitet mit)
 function tafelLage(d){
  const b=d.querySelector('.td-bahn'),z=d.querySelector('.td-zeiger'),tabs=[...d.querySelectorAll('.td-reiter [data-tafel-ziel]')];if(!b||!tabs.length)return;
  const f=Math.max(0,Math.min(tabs.length-1,b.scrollLeft/Math.max(1,b.clientWidth))),a=tabs[Math.floor(f)],c=tabs[Math.ceil(f)],t=f-Math.floor(f),i=Math.round(f);
  if(z){z.style.transform=`translateX(${(a.offsetLeft+(c.offsetLeft-a.offsetLeft)*t).toFixed(1)}px)`;z.style.width=`${(a.offsetWidth+(c.offsetWidth-a.offsetWidth)*t).toFixed(1)}px`;}
  tabs.forEach((r,n)=>{if(r.getAttribute('aria-selected')!==String(n===i))r.setAttribute('aria-selected',String(n===i));});
  [...b.children].forEach((s,n)=>s.classList.toggle('aktiv',n===i));
  const id=b.children[i]?.dataset.tafel;if(id&&id!=='techbaum'&&d.querySelector('[data-ti-reiter]'))ti.reiter=id;
  const nav=d.querySelector('.td-reiter'),r=tabs[i];if(nav&&r&&(r.offsetLeft<nav.scrollLeft||r.offsetLeft+r.offsetWidth>nav.scrollLeft+nav.clientWidth))nav.scrollTo({left:r.offsetLeft-nav.clientWidth/2+r.offsetWidth/2,behavior:'smooth'});
 }
 // Reiter antippen gleitet zur Tafel (statt neu zu zeichnen); Pfeiltasten wechseln; Drehen hält die Tafel
 document.addEventListener('click',ev=>{const t=ev.target.closest('.td-reiter [data-tafel-ziel]');if(!t)return;ev.stopImmediatePropagation();ev.preventDefault();const b=t.closest('dialog')?.querySelector('.td-bahn');if(b)b.scrollTo({left:Number(t.dataset.tafelZiel)*b.clientWidth,behavior:matchMedia('(prefers-reduced-motion: reduce)').matches?'auto':'smooth'});},true);
 // Esc schließt das Detailfeld wie die Zurück-Taste (Rahmen.zurueck), solange kein Dialog offen ist und nicht getippt wird.
 document.addEventListener('keydown',ev=>{if(ev.key!=='Escape'||ev.defaultPrevented||document.querySelector('dialog[open]')||/^(INPUT|TEXTAREA|SELECT)$/.test(ev.target.tagName||''))return;const zu=document.querySelector('#game>section:not([hidden]) [data-bs-schliessen]');if(zu){zu.click();ev.preventDefault();}});
 document.addEventListener('keydown',ev=>{const t=ev.target.closest?.('.td-reiter [role=tab]');if(!t||!['ArrowLeft','ArrowRight'].includes(ev.key))return;const l=[...t.parentElement.querySelectorAll('[role=tab]')],n=l[(l.indexOf(t)+(ev.key==='ArrowRight'?1:l.length-1))%l.length];ev.preventDefault();n.focus();if(n.dataset.tafelZiel)n.click();});
 addEventListener('resize',()=>{const d=$('techinfo'),b=d?.open&&d.querySelector('.td-bahn');if(!b)return;const i=[...d.querySelectorAll('.td-reiter [data-tafel-ziel]')].findIndex(r=>r.getAttribute('aria-selected')==='true');if(i>=0)b.scrollLeft=i*b.clientWidth;tafelLage(d);});
 // Kennwerte der Bühne: Einheiten mit Rang in ihrer Klasse (Weltenkrümmer zählt nicht zum Vergleich), Gebäude und Forschung
 // mit Ausbaustand und dem Schritt zur nächsten Stufe, Truppen im Vergleich der Truppentypen.
 function kennwerte(kat,x){
  const c=rules.catalog,out=[],anteil=(liste,v)=>liste.length?liste.filter(n=>n<=v).length/liste.length:null;
  if(kat==='ships'||kat==='defenses'){
   const w=view.einheiten_kosten?.find(q=>q.einheit===x.key)?.werte||{},r=x.r||{},wert=k=>Number(w[k]??r[k]??0);
   const klasse=Object.entries(c.einheiten).filter(([k,u])=>!!u.schiff===(kat==='ships')&&k!=='weltentruemmer').map(([,u])=>u.regel||{});
   for(const [k,t] of kat==='ships'?[['angriff','Angriff'],['schild','Schild'],['struktur','Struktur'],['tempo','Tempo']]:[['angriff','Angriff'],['schild','Schild'],['struktur','Struktur']]){
    // Rang auf gleicher Grundlage: Grundwert gegen Grundwerte (wirksam gegen Grund verschob den Rang, Funktionsprüfung 25.10.2026).
    const v=wert(k),g0=Number(r[k]??v),a=anteil(klasse.map(g=>Number(g[k]||0)),g0);
    out.push({t,v:fmt(v),anteil:a,titel:`${t} ${fmt(v)}${g0!==v?` (Grundwert ${fmt(g0)})`:''} – Grundwert mindestens so hoch wie bei ${Math.round((a||0)*100)} % der ${kat==='ships'?'Schiffe':'Anlagen'}`});}
  }
  if(kat==='buildings'){const st=buildingStages(x);if(st.length){const max=Math.max(1,st.length-1),j=st[Math.min(x.level,max)],n=st[Math.min(x.level+1,max)];
   out.push({t:'Ausbau',v:`${fmt(x.level)} / ${fmt(max)}`,anteil:x.level/max});
   const schritt=x.level<max;
   if(j.leistung||n.leistung)out.push({t:stageOutputLabel(x.key),v:fmt(j.leistung),dann:schritt?fmt(n.leistung):null,titel:'Jetzt, darunter mit der nächsten Stufe'});
   out.push({t:'Energie',v:fmt(j.energie),dann:schritt?fmt(n.energie):null,titel:'Bedarf jetzt, darunter mit der nächsten Stufe'});
   if(j.arbeiter||n.arbeiter)out.push({t:'Arbeiter',v:fmt(j.arbeiter),dann:schritt?fmt(n.arbeiter):null,titel:'Arbeitskräfte jetzt, darunter mit der nächsten Stufe'});}}
  if(kat==='research'){const r=x.r||{},max=c.welt?.max_forschungsstufe||30,labor=heimat()?.gebaeude?.labor||0;
   out.push({t:'Forschungsstand',v:`${fmt(x.level)} / ${fmt(max)}`,anteil:x.level/max});
   if(r.fp)out.push({t:'Nächste Stufe',v:fmt(Math.round(r.fp*r.faktor**Math.max(0,x.level)))+' FP'});
   if(r.labor)out.push({t:'Labor (Heimatwelt)',v:`${fmt(labor)} / ${fmt(r.labor)}`,anteil:Math.min(1,labor/r.labor)});}
  if(kat==='ground'){const d=currentPlanet()?.bodentruppen,i=x.i;if(d&&i>=0){
   out.push({t:'Angriff',v:fmt(d.angriff[i]),anteil:anteil(d.angriff,d.angriff[i]),titel:'Rang unter den Truppentypen'});
   out.push({t:'Struktur',v:fmt(d.struktur[i]),anteil:anteil(d.struktur,d.struktur[i]),titel:'Rang unter den Truppentypen'});
   out.push({t:'Transportplätze',v:fmt(d.truppenplaetze[i])});out.push({t:'Rekruten je Einheit',v:fmt(d.personal[i])});}}
  return out;
 }
 // Überblick: Zweck/Stärke/Nische als Karten, Schnellfeuer als Bildchips, Voraussetzungen mit Stand, Logbuch eingeklappt
 function ueberblick(kat,key,x,e){
  const felder=[['zweck','Wozu gebaut',e?.zweck],['staerke','Stärke (Grundwerte)',e?.staerke],['nische','Nische',e?.nische]].filter(f=>f[2]);
  const karten=felder.length?`<div class="td-karten">${felder.map(([k,t,v])=>`<article class="td-karte td-${k}"><h4>${t}</h4><p>${esc(v)}</p></article>`).join('')}</div>`:'';
  const ab=stufeVon(kat,key),req=[...(ab?[{zivil:true,need:ab,have:view.stufe}]:[]),...voraussetzungen(kat,key)];
  const chips=req.map(r=>`<li class="${r.have>=r.need?'ok':'fehlt'}">${r.zivil?'<span class="td-chip-text">Zivilisationsstufe</span>':`<button type="button" data-bs-springe="${r.kat}:${esc(r.key)}"><img src="${esc(art(BILDKAT[r.kat],r.key))}" alt="">${esc(named(r.key))}</button>`}<b>${fmt(r.have)} / ${fmt(r.need)}</b></li>`).join('');
  const offen=req.filter(r=>r.have<r.need).length;
  return `${karten}${!felder.length&&x.r?.wirkung?`<p class="td-lead">${esc(x.r.wirkung)}</p>`:''}${sfChips(kat,key)}`
   +`<section class="td-block"><h4>Voraussetzungen <small>${req.length?offen?offen+' offen':'alle erfüllt':''}</small></h4>${req.length?`<ul class="td-chips">${chips}</ul>`:'<p class="td-leise">Keine Voraussetzungen.</p>'}`
   +`<button type="button" class="td-cta" data-ti-reiter="techbaum">Ganzen Pfad im Technologiebaum zeigen <span aria-hidden="true">⤢</span></button></section>${logbuch(lore(e))}`;
 }
 function sfChips(kat,key){
  if(!['ships','defenses'].includes(kat))return '';
  const c=rules.catalog,e=c.einheiten[key];if(!e)return '';
  const gegen=Object.entries(e.zielprofil?.schnellfeuer||{}).filter(([,n])=>n>0).sort((a,b)=>b[1]-a[1]);
  const von=Object.entries(c.einheiten).map(([k,u])=>[k,u.zielprofil?.schnellfeuer?.[key]||0]).filter(([,n])=>n>0).sort((a,b)=>b[1]-a[1]);
  const chip=(cls,[k,n])=>{const kk=c.einheiten[k]?.schiff?'ships':'defenses';return `<li class="${cls}"><button type="button" data-bs-springe="${kk}:${esc(k)}"><img src="${esc(art(BILDKAT[kk],k))}" alt="">${esc(named(k))}</button><b>×${fmt(n)}</b></li>`;};
  // Mehrfachfeuer in Worten (Karl 25.10.2026, neues Balancing): stand vorher nur im eingeklappten Datenblatt (Technik).
  const springt=(e.zielprofil?.verteilung||'').includes('springt');
  const satz=gegen.length?`<p class="td-leise">Je Runde zusätzliche Schüsse gegen diese Ziele${springt?'; nach jedem Abschuss springt die Salve auf das nächste Ziel derselben Art (Mehrfachfeuer)':', bis das Ziel fällt'}.</p>`:'';
  return `<section class="td-block td-kampf"><h4>Schnellfeuer</h4>${gegen.length?`<p class="td-reihe gut">Stark gegen</p><ul class="td-chips">${gegen.map(x=>chip('gut',x)).join('')}</ul>${satz}`:''}${von.length?`<p class="td-reihe schlecht">Verwundbar durch</p><ul class="td-chips">${von.map(x=>chip('schlecht',x)).join('')}</ul>`:''}${gegen.length||von.length?'':'<p class="td-leise">Kein Schnellfeuer von oder gegen diese Einheit.</p>'}</section>`;
 }
 // Technik: Zeichnungen zuerst (Schiffe, Truppen), dann Datenblatt, Stufen- bzw. Fortschrittstabelle, volles Datenblatt eingeklappt
 function technikTafel(kat,x){const z=zeichnungen(kat,x),rest=technik(kat,x);return z+technischeDaten(kat,x)+(z&&rest.startsWith(z)?rest.slice(z.length):rest);}
 // Freischaltungen als Bildkacheln mit Stufe; bereits erreichte sind markiert
 function freischaltungen(kat,key,x){
  const a=anwendungen(kat,key);if(!a.length)return '';
  const lead=x.r?.wirkung?`<p class="td-lead">${esc(x.r.wirkung)}</p>`:'',frei=a.filter(z=>x.level>=z[2]).length;
  if(!a.length)return lead+'<p class="td-leise">Schaltet nichts Weiteres frei; verbessert die genannten Werte.</p>';
  return `${lead}<p class="td-leise">${frei} von ${a.length} freigeschaltet · nächste ab Stufe ${fmt(a.find(z=>x.level<z[2])?.[2]??x.level)}</p><ul class="td-kacheln">${a.map(([k,s,lvl,nm,bild])=>`<li class="${x.level>=lvl?'frei':''}"><button type="button" ${k==='missiles'?'data-tab="verteidigung"':`data-bs-springe="${k}:${esc(s)}"`}><img src="${esc(art(BILDKAT[k],bild||s))}" alt="" loading="lazy"><span>${esc(nm||named(s))}</span><b>${x.level>=lvl?'✓ ':''}Stufe ${fmt(lvl)}</b></button></li>`).join('')}</ul>`;
 }

 // Sofortige Rückmeldung am Kachelknopf, bis der Server antwortet und die Kachel mit Timer neu zeichnet.
 document.addEventListener('click',e=>{const b=e.target.closest('.bs-kachel-bauen');if(!b||b.disabled||b.dataset.pause)return;b.classList.add('sendet');b.textContent='Wird gestartet …';},true);
 document.addEventListener('click',e=>{
  const k=e.target.closest('[data-bs-key]');
  if(k){const seite=k.dataset.bsSeite;if(offen[seite]===k.dataset.bsKey)offen[seite]=null;else{offen[seite]=k.dataset.bsKey;menge[seite]=1;}render(seite);if(offen[seite]){const d=$('bs-detail-'+seite);if(d&&d.getBoundingClientRect().top<0)d.scrollIntoView({block:'start'});}return;}
  const rp=e.target.closest('[data-bs-reparieren]');if(rp){if(rp.disabled||rp.getAttribute('aria-busy')==='true')return;const p=currentPlanet();
   rp.setAttribute('aria-busy','true');rp.disabled=true;Promise.resolve(command([{typ:'reparieren',planet:p.koord,gebaeude:rp.dataset.bsReparieren}])).catch(err=>{if(typeof show==='function')show(err.message||String(err),true);}).finally(()=>render());return;}
  const bt=e.target.closest('[data-bs-bauteil]');if(bt){if(bt.disabled||bt.getAttribute('aria-busy')==='true')return;const feld=bt.dataset.bsAnzahlFeld?$(bt.dataset.bsAnzahlFeld):null,n=feld?Number(feld.value):Math.max(1,Number(bt.dataset.bsAnzahl)||1),p=currentPlanet();
   if(feld&&!(Number.isInteger(n)&&n>=1&&n<=Number(bt.dataset.bsMaxwert))){show(`Anzahl 1 bis ${fmt(Number(bt.dataset.bsMaxwert))} eingeben.`,true);return;}if(feld)menge.bauteil=n;
   bt.setAttribute('aria-busy','true');bt.disabled=true;Promise.resolve(command([{typ:'fertigen',planet:p.koord,bauteil:bt.dataset.bsBauteil,anzahl:n}])).catch(err=>{if(typeof show==='function')show(err.message||String(err),true);}).finally(()=>render('werft'));return;}
  const zu=e.target.closest('[data-bs-schliessen]');if(zu){offen[zu.dataset.bsSchliessen]=null;render(zu.dataset.bsSchliessen);return;}
  const mx=e.target.closest('[data-bs-max]');if(mx){menge[mx.dataset.bsMax]=Math.max(1,Number(mx.dataset.bsMaxwert)||1);render(mx.dataset.bsMax);return;}
  const t=e.target.closest('[data-bs-techinfo]');if(t){techinfo(t.dataset.bsKat,t.dataset.bsTechinfo,t.dataset.bsReiter||'techinfo');return;}
  const r=e.target.closest('[data-ti-reiter]');if(r){techinfo(ti.kat,ti.key,r.dataset.tiReiter);return;}
  const zw=e.target.closest('[data-ti-zeichnung]');if(zw){ti.zeichnung=zw.dataset.tiZeichnung;techinfo(ti.kat,ti.key,'technik');return;}
  const av=e.target.closest('[data-ti-archiv]');if(av){const [kat,key]=av.dataset.tiArchiv.split(':');techinfo(kat,key,'archiv');$('techinfo')?.querySelector('.ti-inhalt')?.scrollTo({top:0});return;}
  if(e.target.closest('[data-ti-schliessen]')){$('techinfo')?.close();return;}
  const s=e.target.closest('[data-bs-springe]');if(s){const [kat,key]=s.dataset.bsSpringe.split(':');const seite=seiteVon(kat,key);if(!seite)return;$('techinfo')?.close();navigateGame(seite);oeffne(seite,key);window.scrollTo({top:0,behavior:'instant'});return;}
  if(e.target.closest('#techinfo [data-tab]'))$('techinfo')?.close();
 });
 document.addEventListener('toggle',e=>{const d=e.target,seite=d?.dataset?.bsEinklappen;if(!EINKLAPPEN[seite])return;try{localStorage.setItem(EINKLAPPEN[seite],d.open?'1':'0');}catch{}},true);
 // Feld und Befehl aus derselben Eingabe: leer oder 0 bestellt nicht still 1 Stück, sondern sperrt mit Grund (Funktionsprüfung 25.10.2026).
 document.addEventListener('input',e=>{const m=/^bs-anzahl-(.+)$/.exec(e.target.id||'');if(!m)return;menge[m[1]]=String(e.target.value).trim();render(m[1]);});
 document.addEventListener('game-navigation',()=>render());
 // Knoten im Technologiebaum (progression.js) öffnen das Detailfeld der Bauseite statt des alten Katalogs.
 document.addEventListener('click',e=>{const b=e.target.closest('[data-tree-open]');if(!b||b.dataset.treeKind==='civilization')return;const seite=seiteVon(b.dataset.treeKind,b.dataset.treeOpen);if(!seite)return;e.stopImmediatePropagation();e.preventDefault();navigateGame(seite);oeffne(seite,b.dataset.treeOpen);window.scrollTo({top:0,behavior:'instant'});},true);
 // Sprünge aus Personal- und Versorgungsansichten öffnen das passende Detailfeld.
 document.addEventListener('click',e=>{const b=e.target.closest('[data-workforce-target]');if(b)setTimeout(()=>{const key=b.dataset.workforceTarget,seite=seiteVon('buildings',key);navigateGame(seite);oeffne(seite,key);},0);});
 return {merkOffen,render,oeffne,techinfo,neuerBaum,anzeige,logbuch,messleiste,dauer,seiteVon,ladeTexte,text,bildVon,archiv,schlangen,gruppen,reparatur};
})();
// Auch als Fenstereigenschaft, damit andere Module optional darauf zugreifen können.
window.Bauseite=Bauseite;
{const ansicht=renderView;renderView=function(){ansicht.apply(this,arguments);Bauseite.render();};}
