'use strict';
// Bodenkampf-Blatt (Invasion v2, Karl 24.10.2026: „Dann soll man die Kampfwellen sehen … man kann die Truppen auf die Wellen
// verteilen frei wie man möchte, aber gut durchdacht, dass man jede Welle anklicken kann und dort seine Truppen zum Verteidigen
// verteilen kann. Man kann auch weiter Bodentruppen bauen (50 % langsamer), diese sammeln sich alle in der letzten Welle als
// Rückhalt! Ist der letzte Punkt durchbrochen und der Rückhalt bezwungen, verliert man die Kontrolle über den Planeten“).
// Ablauf und Zahlen folgen dem Kern (crates/kern/src/invasion2.rs, Sicht `wellen`): N Linien, die letzte ist der Rückhalt
// (immer der Rest); nur Linien ab der aktuellen bis N−1 werden zugeteilt (`invasion_linien`), gebrochene nicht mehr.
// Designsprache DESIGNREGELN §9: nicht-modales Blatt (D1), Kopf · Inhalt · feste Fußleiste (D2), Lage zuerst (D3), Vorlagen vor
// Einzelwerten mit −/Zahl/+/Max (D4), Regeln sichtbar (D5), Zeit mit Uhrzeit (D7), eigene Rückmeldung (D9), ID-Bereich (D10).
// Dazu ein kleines Bestätigungsblatt mit den Folgen statt confirm() (Miliz rufen/entlassen) – auch für lage.js und ground-ui.js.
window.Bodenkampf=(()=>{
 const el=id=>document.getElementById(id);
 const h=s=>typeof esc==='function'?esc(s):String(s??'').replace(/[&<>"']/g,c=>({'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;',"'":'&#39;'}[c]));
 const zahl=n=>typeof fmt==='function'?fmt(Math.round(Number(n)||0)):String(Math.round(Number(n)||0));
 const MILIZ=12;
 // Miliz-Regeln neuer Welten (regeln/kernwerte.ron boden): Forschung und Ausbildungszeit stehen nicht in der Sicht; der Kern
 // prüft die Forschung beim Ausrufen selbst (Fehlertext nennt sie). Karl 24.10.: „Dauer pro 1000 Bevölkerung 5 min“.
 const MILIZ_FORSCHUNG={soziologie:2},MILIZ_SEK_JE_1000=300;
 const tempo=()=>Math.max(1,Number(world?.tempo)||1);
 const jetzt=()=>typeof GameTime!=='undefined'?GameTime.now():(view?.sekunden||0);
 // Dauer wie §5 („5 T 11 h 32 min“), Eingabe in Spielsekunden → Echtzeit.
 const dauer=s=>window.Bauseite?.dauer?Bauseite.dauer((Number(s)||0)/tempo()):GameTime.duration((Number(s)||0)/tempo());
 const uhrzeit=t=>'Tag '+(Math.floor(t/86400)+1)+' · '+[Math.floor(t/3600)%24,Math.floor(t/60)%60].map(n=>String(n).padStart(2,'0')).join(':');
 const summe=a=>(a||[]).reduce((s,n)=>s+(Number(n)||0),0);

 // ---------------------------------------------------------------- Daten
 function typen(){const t=rules?.catalog?.bodentruppen?.typen||[];return [...t.map(x=>({id:x.id,name:x.name||named(x.id),bild:x.bild||x.id,angriff:x.angriff,struktur:x.struktur})),{id:'miliz',name:'Miliz',bild:'miliz'}];}
 const typName=i=>typen()[i]?.name||(i===MILIZ?'Miliz':'Truppen');
 const typBild=i=>typeof art==='function'?art('ground_units',i===MILIZ?'miliz':(typen()[i]?.bild||typen()[i]?.id||'')):'';
 /** Invasion eines Planeten: Eintrag aus view.invasionen (mit Rolle) und – für eigene Planeten – die Planetensicht. */
 function lage(k){
  const inv=(view?.invasionen||[]).find(i=>i.planet===k)||null,p=(view?.planeten||[]).find(x=>x.koord===k)||null;
  const w=inv?.wellen||p?.invasion?.wellen||null,rolle=inv?.rolle||(p?'verteidiger':'');
  const miliz=p?.invasion?.miliz||p?.bodentruppen?.miliz||null;
  return {k,inv,p,w,rolle,miliz,regeln:p?.invasion?.regeln||(w?'invasion-v2':'')};
 }
 const planetName=L=>L.p?(window.Planetenname?.name?Planetenname.name(L.p):(L.p.name||(L.p.heimat?'Heimatwelt':'Kolonie'))):'Planet von '+(L.inv?.besitzer||'?');
 const darfVerteilen=L=>!!L.w&&L.rolle==='verteidiger'&&!L.w.verwuestet&&!gameReadOnlyGrund();
 function gameReadOnlyGrund(){try{return typeof gameReadOnly==='function'?gameReadOnly()||'':'';}catch(_){return '';}}
 const aktIdx=w=>Math.max(0,(Number(w.linie)||1)-1);
 const bearbeitbar=(w,l)=>l>=aktIdx(w)&&l<w.linien_anzahl-1;
 /** Verfügbare Verteidiger je Typ (Garnison + Miliz): Summe aller noch stehenden Linien (gebrochene sind 0, Rückhalt = Rest). */
 function bestand(w){const b=Array(13).fill(0);for(const l of w.linien||[])(l.truppen||[]).forEach((n,i)=>{b[i]+=Number(n)||0;});return b;}
 /** Was der Kern gerade je Linie stehen hat (zugeteilt und vorhanden), Linien 0…N−2. */
 function stand(w){return Array.from({length:Math.max(0,w.linien_anzahl-1)},(_,l)=>l<aktIdx(w)?Array(13).fill(0):(w.linien?.[l]?.truppen||Array(13).fill(0)).map(n=>Number(n)||0));}
 // Entwurf auf die aktuelle Lage begrenzen: gebrochene Linien leer, je Typ nie mehr als vorhanden (Reihenfolge der Linien).
 function begrenzen(w,e){const b=bestand(w),out=e.map((z,l)=>l<aktIdx(w)?Array(13).fill(0):z.slice(0,13).map(n=>Math.max(0,Math.floor(Number(n)||0))));
  for(let i=0;i<13;i++){let frei=b[i];for(let l=0;l<out.length;l++){out[l][i]=Math.min(out[l][i],frei);frei-=out[l][i];}}return out;}
 const rest=(w,e)=>{const b=bestand(w);return b.map((n,i)=>n-e.reduce((s,z)=>s+(z[i]||0),0));};
 const geaendert=(w,e)=>JSON.stringify(begrenzen(w,e))!==JSON.stringify(stand(w));
 const maxFuer=(w,e,L,i)=>bestand(w)[i]-e.reduce((s,z,l)=>s+(l===L?0:(z[i]||0)),0);

 // ---------------------------------------------------------------- Miliz (gemeinsam für Lagekarte, Truppenseite, Blatt)
 /** Lage der Miliz eines eigenen Planeten: Zustand, Sperrgrund mit Ein-Tipp-Lösung, Kosten, Ausbildungszeit. */
 function miliz(p){
  const m=p?.invasion?.miliz||p?.bodentruppen?.miliz;if(!m||m.freigeschaltet===undefined)return null;
  // Forschung und Ausbildungszeit gibt es nur in Welten mit Invasion v2; laufende Welten (v1) rufen sofort, nur in der Belagerung.
  const v2=(p?.invasion?.regeln||'')==='invasion-v2';
  const stufen=view?.forschung?.stufen||{},forschung=Object.entries(m.forschung||(v2?MILIZ_FORSCHUNG:{})).map(([f,n])=>({f,n,ist:Number(stufen[f])||0,ok:(Number(stufen[f])||0)>=n}));
  const kaserne=Number(p.gebaeude?.kaserne)||0,legierung=Math.floor(Number(p.bestand?.legierung)||0),kosten=Number(m.kosten?.gesamt_legierung)||0;
  const aufgebot=Number(m.aufgebot)||0,staerke=Number(m.staerke)||0,bereit=m.ausgerufen&&aufgebot>0&&staerke>=aufgebot;
  const anteil=Number(m.anteil_prozent?.[view?.volk])||100,belagert=!!m.belagert;
  let grund='',schritt=null;
  if(m.kapituliert)grund='Die Miliz hat in dieser Belagerung kapituliert';
  else if(!m.freigeschaltet){grund='Kaserne Stufe 1 auf diesem Planeten fehlt';schritt={springe:'buildings:kaserne',text:'Kaserne bauen'};}
  else if(forschung.some(x=>!x.ok)){const x=forschung.find(x=>!x.ok);grund=`${named(x.f)} Stufe ${x.n} fehlt (erforscht: ${x.ist})`;schritt={springe:'research:'+x.f,text:`${named(x.f)} erforschen`};}
  else if(m.ausgerufen)grund='';
  else if(!(Number(m.verfuegbar)>0))grund=m.jederzeit||belagert?'Keine Bewohner, die kämpfen können':'Erst während einer Belagerung';
  else if(legierung<kosten)grund=`Legierung fehlt: ${zahl(kosten-legierung)} (nötig ${zahl(kosten)}, vorhanden ${zahl(legierung)})`;
  const rufbar=!m.ausgerufen&&!grund&&Number(m.verfuegbar)>0;
  // Kern (0.4.8): Forschung und Ausbildungszeit stehen in der Miliz-Sicht; die Konstanten oben sind nur der Rückfall.
  const sekJe1000=Number(m.ausbildung_sekunden_je_1000??(v2?MILIZ_SEK_JE_1000:0))||0;
  const ausbildung=sekJe1000?Math.ceil((Number(m.verfuegbar)||aufgebot)*sekJe1000/1000):0;
  const restAusbildung=sekJe1000&&m.ausgerufen&&aufgebot>0&&staerke<aufgebot?Math.ceil((aufgebot-staerke)*sekJe1000/1000):0;
  const entlassGrund=m.entlassbar?(belagert?'Während einer Belagerung bleibt die Miliz im Dienst':''):'';
  // Nenner ist das Aufgebot (Kern miliz.aufgebot); Zuwachs durch Bevölkerung steht getrennt (Nachprüfung 25.10.2026:
  // „10.446 von 10.446“ neben „Aufgebot 10.311“). Eine Zeile für Truppenseite, Blatt und Übersicht.
  const zustand=`${(Math.min(staerke,aufgebot||staerke)).toLocaleString('de-DE')} von ${(aufgebot||staerke).toLocaleString('de-DE')} Milizionären${aufgebot&&staerke>aufgebot?` · +${(staerke-aufgebot).toLocaleString('de-DE')} durch Bevölkerung`:''}`;
  return {m,v2,forschung,kaserne,legierung,kosten,aufgebot,staerke,bereit,anteil,belagert,grund,schritt,rufbar,ausbildung,restAusbildung,zustand,entlassbar:!!m.entlassbar&&!entlassGrund,entlassGrund};
 }
 function milizRufen(k){
  const p=(view?.planeten||[]).find(x=>x.koord===k),M=miliz(p);if(!M)return;
  if(!M.rufbar){if(typeof show==='function')show(M.grund||'Miliz gerade nicht möglich',true);return;}
  const L=lage(k),m=M.m;
  bestaetigen({titel:`Miliz auf ${planetName(L)} rufen?`,augenbraue:'VOLKSMILIZ · ['+k+']',knopf:`Miliz rufen · ${zahl(m.verfuegbar)}`,
   folgen:[`<b>${zahl(m.verfuegbar)} Milizionäre</b> – ${M.anteil} % deiner ${h(named(view?.volk||''))}-Bewohner greifen zu den Waffen.`,
    M.v2?`<b>Ausbildung etwa ${h(dauer(M.ausbildung))}</b> in Echtzeit (5 Spielminuten je 1.000, umgerechnet auf das Welttempo); die Stärke wächst bis dahin gleichmäßig.`:'<b>Sofort einsatzbereit</b> – nur einmal je Belagerung.',
    `<b>Kosten ${zahl(M.kosten)} Legierung</b> (${zahl(m.kosten?.legierung_je_bewohner)} je Milizionär) – wird nicht erstattet.`,
    `<b>${zahl(m.arbeitsausfall_prozent)} % der Milizionäre im Dienst</b> fehlen in Produktion und Forschung.`,
    `Waffenwert ${h(String(m.waffenwert).replace('.',','))} · Rüstwert ${zahl(m.ruestwert)} je Milizionär; Kapitulation nach ${zahl(m.kapitulation_prozent)} % Gefallenen.`,
    L.w?'Die Miliz steht im Rückhalt und lässt sich wie Truppen auf die Linien verteilen.':'Sie verteidigt bei einer Invasion; entlassen kannst du sie, solange keine Belagerung läuft.'],
   aktion:()=>[{typ:'miliz_ausrufen',planet:k}]});
 }
 function milizEntlassen(k){
  const p=(view?.planeten||[]).find(x=>x.koord===k),M=miliz(p);if(!M)return;
  if(!M.entlassbar){if(typeof show==='function')show(M.entlassGrund||'Keine Miliz zum Entlassen',true);return;}
  bestaetigen({titel:'Miliz entlassen?',augenbraue:'VOLKSMILIZ · ['+k+']',knopf:'Miliz entlassen',gefahr:true,
   folgen:[`<b>${zahl(M.m.im_dienst||M.staerke)} Milizionäre</b> kehren zur zivilen Arbeit zurück.`,'Die Ausrüstung ist verbraucht: keine Erstattung der Legierung.',
    'Neu rufen kostet wieder Legierung und die volle Ausbildungszeit.'],
   aktion:()=>[{typ:'miliz_entlassen',planet:k}]});
 }

 // ---------------------------------------------------------------- Bestätigungsblatt (statt confirm)
 let frage=null;
 function frageHost(){let b=el('bk-frage');if(b)return b;b=document.createElement('section');b.id='bk-frage';b.className='bk-frage';b.setAttribute('role','alertdialog');b.setAttribute('aria-modal','false');b.setAttribute('aria-labelledby','bk-frage-titel');b.hidden=true;document.body.append(b);return b;}
 function frageZeichnen(){
  const b=frageHost();if(!frage){b.hidden=true;return;}
  setStableHTML(b,`<div class="bk-frage-flaeche${frage.gefahr?' bk-frage-gefahr':''}"><p class="bk-augenbraue">${h(frage.augenbraue||'')}</p><h2 id="bk-frage-titel">${h(frage.titel)}</h2>
   <ul class="bk-folgen">${(frage.folgen||[]).map(f=>`<li>${f}</li>`).join('')}</ul>${frage.fehler?`<p class="bk-fehler" role="alert">${h(frage.fehler)}</p>`:''}
   <div class="bk-frage-knoepfe"><button type="button" class="bk-abbrechen" data-bk-frage-nein>Abbrechen</button><button type="button" class="bk-ja" data-bk-frage-ja${frage.sendet?' aria-busy="true" aria-disabled="true"':''}>${frage.sendet?'Wird ausgeführt …':h(frage.knopf||'Bestätigen')}</button></div></div>`);
  b.hidden=false;
 }
 function bestaetigen(o){frage={...o,fehler:'',sendet:false};frageZeichnen();setTimeout(()=>{try{el('bk-frage')?.querySelector('[data-bk-frage-nein]')?.focus({preventScroll:true});}catch(_){}},30);}
 function frageZu(){frage=null;frageZeichnen();}
 async function frageJa(){
  if(!frage||frage.sendet)return;frage.sendet=true;frage.fehler='';frageZeichnen();
  try{const r=await command(frage.aktion());const f=(r?.ergebnisse||[]).find(x=>!x.ok);if(f){frage.sendet=false;frage.fehler=f.text||'Abgelehnt';frageZeichnen();return;}frageZu();zeichnen();}
  catch(err){if(frage){frage.sendet=false;frage.fehler=err?.message||String(err);frageZeichnen();}}
 }

 // ---------------------------------------------------------------- Blatt
 let z=null;// {k, linie:null|Index, entwurf, meldung, fehler, sendet}
 function blatt(){let b=el('bodenkampf-blatt');if(b)return b;b=document.createElement('section');b.id='bodenkampf-blatt';b.className='bk-blatt';b.setAttribute('role','dialog');b.setAttribute('aria-modal','false');b.setAttribute('aria-labelledby','bk-titel');b.hidden=true;document.body.append(b);return b;}
 const offen=()=>!!z&&!el('bodenkampf-blatt')?.hidden;
 // Ungespeicherte Verteilung je Planet bis zum nächsten Öffnen behalten (Funktionsprüfung 25.10.2026: Esc/Zurück verwarf sie still).
 const entwuerfe=new Map();
 function oeffnen(k,linie=null){
  const L=lage(String(k||''));if(!L.w&&!L.inv){if(typeof show==='function')show('Auf diesem Planeten läuft keine Invasion.',true);return false;}
  const l=Number(linie);
  z={k:L.k,linie:L.w&&Number.isInteger(l)&&linie!==null&&linie!==''&&darfVerteilen(L)&&bearbeitbar(L.w,l)?l:null,entwurf:L.w&&L.w.linien?(entwuerfe.get(L.k)||stand(L.w)):[],meldung:entwuerfe.has(L.k)&&L.w?'Ungespeicherte Verteilung von vorhin – speichern oder ändern':'',fehler:'',sendet:false,rueck:document.activeElement};
  const b=blatt();b.hidden=false;document.body.classList.add('bk-offen');zeichnen();
  const innen=b.querySelector('.bk-innen');if(innen)innen.scrollTop=0;
  setTimeout(()=>{try{b.querySelector('.bk-x')?.focus({preventScroll:true});}catch(_){}},40);
  return true;
 }
 function schliessen(){const b=el('bodenkampf-blatt'),f=z?.rueck;if(z){const w=lage(z.k).w;if(w&&z.entwurf?.length&&geaendert(w,begrenzen(w,z.entwurf)))entwuerfe.set(z.k,z.entwurf);else entwuerfe.delete(z.k);}z=null;if(b)b.hidden=true;document.body.classList.remove('bk-offen');if(f?.isConnected)try{f.focus({preventScroll:true});}catch(_){}}
 function chips(a,max=4){
  const l=(a||[]).map((n,i)=>({i,n:Number(n)||0})).filter(x=>x.n>0).sort((x,y)=>y.n-x.n);if(!l.length)return '<span class="bk-leer">keine Truppen</span>';
  const vor=l.slice(0,max).map(x=>`<span class="bk-chip"><img src="${h(typBild(x.i))}" alt="" loading="lazy"><b>${zahl(x.n)}</b> ${h(typName(x.i))}</span>`).join('');
  return vor+(l.length>max?`<span class="bk-chip bk-chip-mehr">+${l.length-max} weitere</span>`:'');
 }
 // Kurzform für die Linienkarten: eine umbrechende Textzeile statt Bildchips (bleibt bei 360 px kompakt).
 function kurzliste(a,max=4){const l=(a||[]).map((n,i)=>({i,n:Number(n)||0})).filter(x=>x.n>0).sort((x,y)=>y.n-x.n);if(!l.length)return '<span class="bk-leer">keine Truppen</span>';
  return `<span class="bk-linie-liste">${l.slice(0,max).map(x=>`<b>${zahl(x.n)}</b> ${h(typName(x.i))}`).join(' · ')}${l.length>max?` · +${l.length-max} weitere`:''}</span>`;}
 function welleText(w){const t=Number(w.naechste_welle)||0;return {t,uhr:uhrzeit(t)};}
 function linienKarten(L){
  const w=L.w,n=w.linien_anzahl,e=z?begrenzen(w,z.entwurf):stand(w),r=rest(w,e),darf=darfVerteilen(L),wt=welleText(w),def=L.rolle!=='angreifer'&&L.rolle!=='angriffshelfer';
  return `<ol class="bk-linien" aria-label="Verteidigungslinien">${(w.linien||[]).map((li,l)=>{
   const st=li.status,rh=!!li.rueckhalt,kann=darf&&bearbeitbar(w,l),bericht=(w.berichte||[]).find(b=>Number(b.linie)===l+1&&b.ergebnis==='gebrochen');
   const truppen=rh?r:(l<e.length?e[l]:null),gesamt=def?(rh?summe(r)+(Number(w.verbuendete_hilfe)||0):summe(truppen)):null;
   const unter=st==='gebrochen'?`gebrochen${bericht?' in Welle '+zahl(bericht.welle):''}`:st==='aktuell'?(w.verwuestet?'überrannt':`nächste Welle trifft hier · in <b data-arrival-echt="${wt.t}"></b>`):rh?'alles nicht Verteilte, neu Ausgebildete und verbündete Hilfe':'wartet';
   const kopf=`<span class="bk-linie-nr" aria-hidden="true">${rh?'R':l+1}</span><span class="bk-linie-text"><b>${rh?'Rückhalt':'Linie '+(l+1)}</b><small>${unter}</small></span><span class="bk-linie-zahl">${st==='gebrochen'?'–':gesamt==null?'<small>unbekannt</small>':zahl(gesamt)}</span>${kann?'<span class="bk-pfeil" aria-hidden="true">›</span>':''}`;
   const unten=def&&st!=='gebrochen'&&truppen?`<span class="bk-linie-chips">${kurzliste(truppen)}${rh&&Number(w.verbuendete_hilfe)>0?` <span class="bk-linie-liste">+ <b>${zahl(w.verbuendete_hilfe)}</b> verbündete Hilfe</span>`:''}</span>`:'';
   const kl=`bk-linie bk-${h(st)}${rh?' bk-rueckhalt':''}${kann?' bk-tippbar':''}`;
   return kann?`<li class="${kl}"><button type="button" class="bk-linie-knopf" data-bk-linie="${l}" aria-label="Linie ${l+1}: ${zahl(gesamt)} Truppen – verteilen">${kopf}${unten}</button></li>`
    :`<li class="${kl}"><div class="bk-linie-knopf" role="group" aria-label="${rh?'Rückhalt':'Linie '+(l+1)}">${kopf}${unten}</div></li>`;
  }).join('')}</ol>`;
 }
 function regeln(w){
  const ab=Number(w.abstand_sekunden)||43200,runden=Number(w.runden_je_welle)||6,lang=Math.round(100-(Number(w.ausbildung_promille)||1000)/10);
  let auf=true;try{auf=localStorage.getItem('sternenepoche.bodenkampf.regeln')!=='zu';}catch(_){}
  return `<details class="bk-klapp bk-regeln" data-bk-regeln${auf?' open':''}><summary><span>So läuft die Invasion</span><small>${zahl(w.linien_anzahl)} Linien · alle ${h(dauer(ab))} eine Welle</small></summary>
   <ol class="bk-zeitleiste"><li>Alle <b>${h(dauer(ab))}</b> greift eine Welle die aktuelle Linie an – höchstens ${zahl(runden)} Bodenrunden.</li>
    <li>Linie ohne Truppen → <b>gebrochen</b>, die nächste Welle trifft die nächste Linie. Angreifer tot → <b>abgewehrt</b>. Beide übrig → die Linie <b>hält</b>.</li>
    <li>Der <b>Rückhalt</b> ist alles, was du nicht verteilst – dazu neu ausgebildete Truppen (Kaserne ${lang>0?lang+' % langsamer':'während der Invasion'}), die Miliz und verbündete Hilfe.</li>
    <li>Fällt der Rückhalt, ist der Planet <b>verwüstet</b>: du verlierst die Kontrolle, ein Kolonieschiff der Gegenseite kann landen. Landen neue Truppen, steht der Rückhalt wieder.</li></ol>
   <ul class="bk-zaehlt"><li>Verteilen geht bis zum Beginn jeder Welle</li><li>Gebrochene Linien bleiben gebrochen</li><li>Nachschub landet trotz Blockade</li></ul>${w.hinweis?`<p class="bk-leise">${h(w.hinweis)}</p>`:''}</details>`;
 }
 function aktionen(L){
  const out=[],k=h(L.k),M=L.p?miliz(L.p):null,def=L.rolle==='verteidiger'||L.rolle==='verbuendeter';
  if(L.rolle==='verteidiger'&&M){
   if(M.rufbar)out.push(`<button type="button" class="bk-aktion bk-aktion-miliz" data-bk-miliz-rufen="${k}"><b>Miliz rufen</b><small>${zahl(M.m.verfuegbar)} Milizionäre · ${zahl(M.kosten)} Legierung${M.ausbildung?' · '+h(dauer(M.ausbildung)):''}</small></button>`);
   else if(M.m.ausgerufen)out.push(`<div class="bk-aktion bk-aktion-info"><b>Miliz ${M.bereit?'einsatzbereit':'in Ausbildung'}</b><small>${h(M.zustand)}${M.restAusbildung?` · noch etwa ${h(dauer(M.restAusbildung))}`:''}</small><progress max="${Math.max(1,M.aufgebot)}" value="${Math.min(M.staerke,M.aufgebot)}"></progress></div>`);
   else if(M.grund)out.push(`<div class="bk-aktion bk-aktion-info"><b>Miliz nicht möglich</b><small>${h(M.grund)}</small>${M.schritt?`<button type="button" class="bk-schritt" data-bk-springe="${h(M.schritt.springe)}">${h(M.schritt.text)} ›</button>`:''}</div>`);
  }
  if(L.rolle==='verteidiger'){
   out.push(`<button type="button" class="bk-aktion" data-bk-ausbilden="${k}"><b>Truppen ausbilden</b><small>${L.w?.verwuestet?'neue Truppen besetzen den Rückhalt wieder':`während der Invasion ${Math.round(100-(Number(L.w?.ausbildung_promille)||1000)/10)} % langsamer · stehen im Rückhalt`}</small></button>`);
   out.push(`<button type="button" class="bk-aktion" data-bk-nachschub="${k}" data-bk-auftrag="stationieren"><b>Nachschub landen</b><small>Bodentruppen von einem anderen Planeten · landen trotz Blockade</small></button>`);
  }else if(L.rolle==='verbuendeter')out.push(`<button type="button" class="bk-aktion" data-bk-nachschub="${k}" data-bk-auftrag="verstaerken"><b>Bodentruppen schicken</b><small>verstärken den Rückhalt deines Verbündeten</small></button>`);
  else if(L.w?.verwuestet)out.push(`<button type="button" class="bk-aktion" data-bk-nachschub="${k}" data-bk-auftrag="kampfkolonisieren"><b>Kolonieschiff schicken</b><small>verwüstet – mit freiem Kolonieplatz landet es und übernimmt</small></button>`);
  else out.push(`<button type="button" class="bk-aktion" data-bk-nachschub="${k}" data-bk-auftrag="${L.rolle==='angriffshelfer'?'verstaerken':'kampfkolonisieren'}"><b>Truppen nachschicken</b><small>kämpfen ab der nächsten Welle mit</small></button>`);
  return out.length?`<div class="bk-aktionen">${out.join('')}</div>`:'';
 }
 function berichte(w){
  const l=w.berichte||[];if(!l.length)return `<section class="bk-teil"><h3 class="bk-teil-titel">Wellenberichte</h3><p class="bk-leise">Noch keine Welle geschlagen. Die erste kommt ${h(uhrzeit(Number(w.naechste_welle)||0))}.</p></section>`;
  const ERG={gebrochen:'gebrochen',gehalten:'gehalten',abgewehrt:'abgewehrt'};
  const reihe=b=>{const t=(b.typen||[]).map((_,i)=>i).filter(i=>(b.angreifer_vorher?.[i]||0)+(b.verteidiger_vorher?.[i]||0)>0);
   return `<div class="bk-tabelle" role="table" aria-label="Verluste je Truppentyp"><div class="bk-tr bk-th" role="row"><span role="columnheader">Typ</span><span role="columnheader">Angreifer</span><span role="columnheader">Verteidiger</span></div>${t.map(i=>`<div class="bk-tr" role="row"><span role="cell">${h(typName(i))}</span><span role="cell">${zahl(b.angreifer_vorher?.[i])} → ${zahl(b.angreifer_nachher?.[i])}${b.angreifer_verluste?.[i]?`<small>−${zahl(b.angreifer_verluste[i])}</small>`:''}</span><span role="cell">${zahl(b.verteidiger_vorher?.[i])} → ${zahl(b.verteidiger_nachher?.[i])}${b.verteidiger_verluste?.[i]?`<small>−${zahl(b.verteidiger_verluste[i])}</small>`:''}</span></div>`).join('')}</div>`;};
  // Herkunft der Verteidiger (Garnison, Verbündete, Miliz) wie im Berichtemodul – nur im Rückhalt geliefert (Nachprüfung 25.10.2026).
  const herkunft=b=>{const q=(b.verteidiger_quellen||[]).filter(x=>x&&x.quelle);if(!q.length)return '';const n=x=>Array.isArray(x)?summe(x):Number(x)||0;
   return `<p class="bk-leise">Verteidiger: ${q.map(x=>`${h({garnison:'Garnison',verbuendete:'Verbündete',miliz:'Volksmiliz'}[x.quelle]||x.quelle)}${x.name?' '+h(x.name):''} ${zahl(n(x.vorher))}`).join(' · ')}</p>`;};
  const runde=r=>{const v=(a)=>(a||[]).map((n,i)=>n>0?`${zahl(n)} ${typName(i)}`:'').filter(Boolean).join(', ')||'keine';
   const feuer=f=>(f||[]).map(x=>`${zahl(x.anzahl)} ${h(typName(typen().findIndex(t=>t.id===x.einheit)))} → ${h(typName(typen().findIndex(t=>t.id===x.ziel)))}: ${zahl(x.schaden)}`).join(' · ');
   return `<li><b>Runde ${zahl(r.runde)}</b><span>Angreifer verliert: ${h(v(r.angreifer_verluste))}</span><span>Verteidiger verliert: ${h(v(r.verteidiger_verluste))}</span>${(r.angreifer_feuer||[]).length?`<small>Feuer Angreifer: ${feuer(r.angreifer_feuer)}</small>`:''}${(r.verteidiger_feuer||[]).length?`<small>Feuer Verteidiger: ${feuer(r.verteidiger_feuer)}</small>`:''}</li>`;};
  return `<section class="bk-teil"><h3 class="bk-teil-titel">Wellenberichte <small>${zahl(l.length)} · neueste zuerst</small></h3>${l.map(b=>`<details class="bk-bericht bk-erg-${h(b.ergebnis)}"><summary><span class="bk-bericht-kopf"><b>Welle ${zahl(b.welle)} · ${Number(b.linie)===Number(b.linien)?'Rückhalt':'Linie '+zahl(b.linie)}</b><span class="bk-erg">${h(ERG[b.ergebnis]||b.ergebnis)}</span></span><small>${h(uhrzeit(Number(b.zeit)||0))} · Angreifer −${zahl(summe(b.angreifer_verluste))} · Verteidiger −${zahl(summe(b.verteidiger_verluste))}</small></summary>${reihe(b)}${herkunft(b)}${(b.runden||[]).length?`<details class="bk-runden"><summary>Runden (${zahl(b.runden.length)})</summary><ol>${b.runden.map(runde).join('')}</ol></details>`:''}</details>`).join('')}</section>`;
 }
 function uebersicht(L){
  const w=L.w,def=L.rolle!=='angreifer'&&L.rolle!=='angriffshelfer',a=summe(w.angreifer_truppen),e=begrenzen(w,z.entwurf),vg=summe(bestand(w))+(Number(w.verbuendete_hilfe)||0);
  const lageZeile=`<div class="bk-kraefte"><div class="bk-kraft bk-kraft-a"><small>Angreifer am Boden</small><b>${zahl(a)}</b></div><div class="bk-kraft bk-kraft-v"><small>${L.rolle==='verteidiger'?'Deine Verteidiger':'Verteidiger'}</small><b>${def?zahl(vg):'?'}</b>${def?'':'<span class="bk-leise">unbekannt</span>'}</div><div class="bk-kraft-typen"><span class="bk-linie-chips">${chips(w.angreifer_truppen,4)}</span></div></div>`;
  const verw=w.verwuestet?`<p class="bk-verwuestet" role="status"><b>Verwüstet</b> ${L.rolle==='verteidiger'?'– du hast die Kontrolle verloren. Lande neue Truppen: dann steht der Rückhalt wieder.':'– die Verteidigung ist gebrochen. Ein Kolonieschiff (Kampfkolonisieren oder Kolonisieren) kann landen und übernehmen, sobald ein Kolonieplatz frei ist.'}</p>`:'';
  const dirty=darfVerteilen(L)&&geaendert(w,e);
  return `${verw}${lageZeile}<section class="bk-teil"><h3 class="bk-teil-titel">Verteidigungslinien <small>${darfVerteilen(L)?'Linie antippen zum Verteilen':L.rolle==='verteidiger'?'':'nur der Besitzer verteilt'}</small></h3>${linienKarten(L)}${dirty?'<p class="bk-hinweis">Verteilung geändert – noch nicht gespeichert.</p>':''}</section>${aktionen(L)}${regeln(w)}${berichte(w)}`;
 }
 function stepper(i,wert,max,name){return `<div class="bk-zaehler"><button type="button" class="bk-z" data-bk-minus="${i}" aria-label="${h(name)} weniger"${wert<=0?' aria-disabled="true"':''}>−</button><input type="number" inputmode="numeric" min="0" max="${max}" value="${wert}" data-bk-zahl="${i}" aria-label="${h(name)} in dieser Linie"><button type="button" class="bk-z" data-bk-plus="${i}" aria-label="${h(name)} mehr"${wert>=max?' aria-disabled="true"':''}>+</button><button type="button" class="bk-z-max" data-bk-max="${i}">Max</button></div>`;}
 function editor(L){
  const w=L.w,l=z.linie,e=begrenzen(w,z.entwurf),b=bestand(w),r=rest(w,e),T=typen(),wt=welleText(w),aktuell=l===aktIdx(w);
  const zeilen=T.map((t,i)=>({t,i})).filter(({i})=>b[i]>0);
  const vorlagen=[['leer','Leeren','0 in dieser Linie'],['haelfte','Hälfte','½ von allem Freien'],['alles','Alles Freie','Rückhalt bleibt leer']];
  return `<div class="bk-unterkopf"><button type="button" class="bk-unterzurueck" data-bk-zurueck aria-label="Zurück zu den Linien"><span aria-hidden="true">‹</span> Linien</button><h3>Linie ${l+1} verteilen</h3></div>
   <p class="bk-leise">${aktuell?`Diese Linie trifft die nächste Welle in <b data-arrival-echt="${wt.t}"></b> (${h(wt.uhr)}).`:`Diese Linie kämpft, wenn Linie ${l} gebrochen ist.`} Was du hier nicht einteilst, steht im Rückhalt.</p>
   <div class="bk-vorlagen" role="group" aria-label="Vorlagen">${vorlagen.map(([id,t,s])=>`<button type="button" class="bk-vorlage" data-bk-vorlage="${id}"><b>${t}</b><small>${s}</small></button>`).join('')}</div>
   ${zeilen.length?`<ul class="bk-typen">${zeilen.map(({t,i})=>{const max=maxFuer(w,e,l,i),anders=b[i]-max;return `<li class="bk-typ${e[l][i]>0?' gewaehlt':''}"><img src="${h(typBild(i))}" alt="" loading="lazy"><span class="bk-typ-name"><b>${h(t.name)}</b><small>${zahl(b[i])} vorhanden${anders>0?` · ${zahl(anders)} in anderen Linien`:''} · Rückhalt ${zahl(r[i])}${i===MILIZ?' · wächst bis zur vollen Ausbildung':''}</small></span>${stepper(i,e[l][i],max,t.name)}</li>`;}).join('')}</ul>`:'<p class="bk-hinweis">Keine Truppen auf dem Planeten. Bilde Truppen aus, lande Nachschub oder rufe die Miliz.</p>'}
   <p class="bk-rest" aria-live="polite"><b>Rest im Rückhalt: ${zahl(summe(r)+(Number(w.verbuendete_hilfe)||0))}</b>${summe(r)?` <span>${r.map((n,i)=>n>0?`${zahl(n)} ${h(typName(i))}`:'').filter(Boolean).join(' · ')}</span>`:''}${Number(w.verbuendete_hilfe)>0?` <span>+ ${zahl(w.verbuendete_hilfe)} verbündete Hilfe</span>`:''}</p>`;
 }
 function fuss(L){
  const w=L.w,dirty=w&&darfVerteilen(L)&&geaendert(w,begrenzen(w,z.entwurf));
  let haupt='',zeile=z.fehler?`<p class="bk-fuss-text bk-fuss-fehler" role="alert">${h(z.fehler)}</p>`:z.meldung?`<p class="bk-fuss-text bk-fuss-ok" role="status">${h(z.meldung)}</p>`:'<p class="bk-fuss-text"></p>';
  const verteilung=()=>{const e=begrenzen(w,z.entwurf),r=rest(w,e);return e.map((x,l)=>l>=aktIdx(w)?`Linie ${l+1}: ${zahl(summe(x))}`:'').filter(Boolean).concat('Rückhalt '+zahl(summe(r)+(Number(w.verbuendete_hilfe)||0))).join(' · ');};
  const sendet=z.sendet?' aria-busy="true" aria-disabled="true"':'';
  if(z.linie!=null&&w)haupt=`<button type="button" class="bk-weiter" data-bk-speichern${sendet}><b>${z.sendet?'Wird gespeichert …':`Linie ${z.linie+1} speichern`}</b><small>${h(verteilung())}</small></button>`;
  else if(dirty)haupt=`<button type="button" class="bk-weiter" data-bk-speichern${sendet}><b>${z.sendet?'Wird gespeichert …':'Verteilung speichern'}</b><small>${h(verteilung())}</small></button>`;
  else if(w&&darfVerteilen(L))haupt=`<button type="button" class="bk-weiter" data-bk-linie="${aktIdx(w)}"><b>Linie ${aktIdx(w)+1} verteilen</b><small>trifft die nächste Welle · ${h(uhrzeit(Number(w.naechste_welle)||0))}</small></button>`;
  else if(L.rolle==='verteidiger')haupt=`<button type="button" class="bk-weiter" data-bk-nachschub="${h(L.k)}" data-bk-auftrag="stationieren"><b>Nachschub landen</b><small>Bodentruppen von einem anderen Planeten</small></button>`;
  else haupt=`<button type="button" class="bk-weiter" data-bk-nachschub="${h(L.k)}" data-bk-auftrag="${L.rolle==='verbuendeter'||L.rolle==='angriffshelfer'?'verstaerken':'kampfkolonisieren'}"><b>${L.rolle==='verbuendeter'?'Bodentruppen schicken':L.w?.verwuestet?'Kolonieschiff schicken':'Truppen nachschicken'}</b><small>öffnet die Einsatzplanung</small></button>`;
  return `<div class="bk-fuss">${zeile}<button type="button" class="bk-zurueck" data-bk-zurueck><span aria-hidden="true">‹</span><small>${z.linie!=null?'Linien':'Zurück'}</small></button>${haupt}</div>`;
 }
 function zeichnen(){
  const b=el('bodenkampf-blatt');if(!z||!b||b.hidden)return;
  const L=lage(z.k);
  if(!L.w&&!L.inv){setStableHTML(b,`<div class="bk-flaeche"><div class="bk-kopf"><div class="bk-kopf-zeile"><div class="bk-kopf-text"><span class="bk-augenbraue">BODENKAMPF</span><h2 id="bk-titel">Invasion beendet</h2><p><span class="koord">[${h(z.k)}]</span></p></div><button type="button" class="bk-x" data-bk-zu aria-label="Schließen">✕</button></div></div><div class="bk-innen"><p class="bk-leise">Auf diesem Planeten läuft keine Invasion mehr. Das Ergebnis steht in den Nachrichten.</p></div><div class="bk-fuss"><p class="bk-fuss-text"></p><button type="button" class="bk-zurueck" data-bk-zu><span aria-hidden="true">‹</span><small>Zurück</small></button><button type="button" class="bk-weiter" data-tab="briefkasten" data-bk-zu><b>Zu den Nachrichten</b></button></div></div>`);return;}
  const w=L.w;
  if(w){z.entwurf=begrenzen(w,z.entwurf.length===Math.max(0,w.linien_anzahl-1)?z.entwurf:stand(w));if(z.linie!=null&&!(darfVerteilen(L)&&bearbeitbar(w,z.linie)))z.linie=null;}
  const wt=w?welleText(w):null,rolleText={verteidiger:'Dein Planet',verbuendeter:'Verbündeter',angreifer:'Deine Invasion',angriffshelfer:'Invasion deines Verbündeten'}[L.rolle]||'';
  const kopf=`<div class="bk-kopf"><div class="bk-kopf-zeile"><span class="bk-kopf-zeichen" aria-hidden="true">${window.Auftraege?Auftraege.svg('kampfkolonisieren',26):''}</span><div class="bk-kopf-text"><span class="bk-augenbraue">BODENKAMPF · ${h(rolleText.toUpperCase())}</span><h2 id="bk-titel">${h(planetName(L))}</h2><p><span class="koord">[${h(L.k)}]</span>${w?` · ${L.rolle==='angreifer'||L.rolle==='angriffshelfer'?'Verteidiger '+h(w.verteidiger_name||'?'):'Angreifer '+h(w.angreifer_name||'?')} · ${zahl(w.wellen)} ${Number(w.wellen)===1?'Welle':'Wellen'} geschlagen`:''}</p></div><button type="button" class="bk-x" data-bk-zu aria-label="Bodenkampf schließen">✕</button></div>
   ${w&&!w.verwuestet?`<p class="bk-welle"><span>Nächste Welle in</span><b data-arrival-echt="${wt.t}"></b><small>${h(wt.uhr)} · trifft ${Number(w.linie)>=w.linien_anzahl?'den Rückhalt':'Linie '+zahl(w.linie)}</small></p>`:w?.verwuestet?'<p class="bk-welle bk-welle-verwuestet"><b>Verwüstet</b><small>Keine Wellen mehr – der Rückhalt ist gebrochen</small></p>':''}</div>`;
  const inhalt=!w?'<p class="bk-leise">Die Bombardierung läuft noch; die Linien stehen ab der Landung der Bodentruppen.</p>':z.linie!=null?editor(L):uebersicht(L);
  setStableHTML(b,`<div class="bk-flaeche">${kopf}<div class="bk-innen">${inhalt}</div>${fuss(L)}</div>`);
 }
 // Eine Ebene zurück: Zuteilung → Übersicht → Blatt zu. Bestätigung zuerst.
 function zurueck(){if(frage){frageZu();return true;}if(!offen())return false;if(z.linie!=null){z.linie=null;z.fehler='';zeichnen();return true;}schliessen();return true;}
 async function speichern(){
  const L=lage(z.k),w=L.w;if(!w||z.sendet||!darfVerteilen(L))return;
  const e=begrenzen(w,z.entwurf),linien=e.map((x,l)=>l<aktIdx(w)?[]:x);
  z.sendet=true;z.fehler='';z.meldung='';zeichnen();
  try{const r=await command([{typ:'invasion_linien',planet:z.k,linien}]);if(!z)return;const f=(r?.ergebnisse||[]).find(x=>!x.ok);
   z.sendet=false;if(f){z.fehler=f.text||'Abgelehnt';zeichnen();return;}
   const w2=lage(z.k).w||w,l=z.linie;z.entwurf=stand(w2);entwuerfe.delete(z.k);
   z.meldung=l!=null?`Linie ${l+1} gespeichert · ${zahl(summe(z.entwurf[l]||[]))} Truppen · Rückhalt ${zahl(summe(rest(w2,z.entwurf))+(Number(w2.verbuendete_hilfe)||0))}`:'Verteilung gespeichert';
   z.linie=null;zeichnen();}
  catch(err){if(z){z.sendet=false;z.fehler=err?.message||String(err);zeichnen();}}
 }
 function setzen(i,wert){const L=lage(z.k),w=L.w;if(!w||z.linie==null)return;const e=begrenzen(w,z.entwurf),max=maxFuer(w,e,z.linie,i);e[z.linie][i]=Math.max(0,Math.min(max,Math.floor(Number(wert)||0)));z.entwurf=e;z.fehler='';z.meldung='';zeichnen();}
 function vorlage(id){const L=lage(z.k),w=L.w;if(!w||z.linie==null)return;const e=begrenzen(w,z.entwurf);for(let i=0;i<13;i++){const max=maxFuer(w,e,z.linie,i);e[z.linie][i]=id==='leer'?0:id==='alles'?max:Math.floor(max/2);}z.entwurf=e;z.meldung='';z.fehler='';zeichnen();}
 function nachschub(k,auftrag){
  // Einsatzplanung (einsatz.js) mit Ziel und Auftrag; sie liegt über diesem Blatt (z-index), Zurück schließt sie zuerst.
  if(window.Einsatz?.oeffnen){Promise.resolve(Einsatz.oeffnen({ziel:k,mission:auftrag})).then(ok=>{if(ok===false&&window.Flottenfuehrung?.planen)Flottenfuehrung.planen(k,auftrag);});return;}
  if(window.Flottenfuehrung?.planen){schliessen();Flottenfuehrung.planen(k,auftrag);}
 }
 function ausbilden(k){schliessen();if(window.PlanetNavigation?.select&&currentPlanet()?.koord!==k)PlanetNavigation.select(k);navigateGame('truppen');window.scrollTo({top:0,behavior:'instant'});}

 // ---------------------------------------------------------------- Ereignisse
 document.addEventListener('click',e=>{
  const t=e.target;
  const fj=t.closest('[data-bk-frage-ja]');if(fj){e.preventDefault();if(fj.getAttribute('aria-disabled')!=='true')void frageJa();return;}
  if(t.closest('[data-bk-frage-nein]')){e.preventDefault();frageZu();return;}
  const o=t.closest('[data-bk-oeffnen]');if(o){e.preventDefault();oeffnen(o.dataset.bkOeffnen,o.dataset.bkZuLinie??null);return;}
  const mr=t.closest('[data-bk-miliz-rufen]');if(mr){e.preventDefault();milizRufen(mr.dataset.bkMilizRufen);return;}
  const me=t.closest('[data-bk-miliz-entlassen]');if(me){e.preventDefault();milizEntlassen(me.dataset.bkMilizEntlassen);return;}
  const ab=t.closest('[data-bk-ausbilden]');if(ab){e.preventDefault();ausbilden(ab.dataset.bkAusbilden);return;}
  const ns=t.closest('[data-bk-nachschub]');if(ns){e.preventDefault();nachschub(ns.dataset.bkNachschub,ns.dataset.bkAuftrag||'stationieren');return;}
  const sp=t.closest('[data-bk-springe]');if(sp){e.preventDefault();const [kat,key]=sp.dataset.bkSpringe.split(':');if(offen())schliessen();if(window.Bauseite?.seiteVon){const s=Bauseite.seiteVon(kat,key);if(s){navigateGame(s);Bauseite.oeffne(s,key);window.scrollTo({top:0,behavior:'instant'});}}return;}
  const b=el('bodenkampf-blatt');if(!b||b.hidden||!b.contains(t))return;
  if(t.closest('[data-bk-zu]')){if(!t.closest('[data-tab]'))e.preventDefault();schliessen();return;}
  const zr=t.closest('[data-bk-zurueck]');if(zr){e.preventDefault();zurueck();return;}
  const li=t.closest('[data-bk-linie]');if(li){e.preventDefault();z.linie=Number(li.dataset.bkLinie);z.meldung='';z.fehler='';zeichnen();const innen=b.querySelector('.bk-innen');if(innen)innen.scrollTop=0;setTimeout(()=>{try{b.querySelector('.bk-unterzurueck')?.focus({preventScroll:true});}catch(_){}},30);return;}
  const sv=t.closest('[data-bk-speichern]');if(sv){e.preventDefault();if(sv.getAttribute('aria-disabled')!=='true')void speichern();return;}
  const vl=t.closest('[data-bk-vorlage]');if(vl){e.preventDefault();vorlage(vl.dataset.bkVorlage);return;}
  const pl=t.closest('[data-bk-plus],[data-bk-minus],[data-bk-max]');if(pl){e.preventDefault();const L=lage(z.k),w=L.w;if(!w||z.linie==null)return;const i=Number(pl.dataset.bkPlus??pl.dataset.bkMinus??pl.dataset.bkMax),e2=begrenzen(w,z.entwurf),ist=e2[z.linie][i]||0,max=maxFuer(w,e2,z.linie,i);
   const schritt=b=>b<50?1:b<500?5:b<5000?50:500;setzen(i,'bkMax' in pl.dataset?max:'bkPlus' in pl.dataset?ist+schritt(max):ist-schritt(max));return;}
 });
 document.addEventListener('input',e=>{const i=e.target.closest?.('#bodenkampf-blatt [data-bk-zahl]');if(!i||!z||z.linie==null)return;const L=lage(z.k);if(!L.w)return;const e2=begrenzen(L.w,z.entwurf),idx=Number(i.dataset.bkZahl);const soll=Math.max(0,Math.min(maxFuer(L.w,e2,z.linie,idx),Math.floor(Number(i.value)||0)));e2[z.linie][idx]=soll;z.entwurf=e2;z.meldung='';z.fehler='';
  // Über dem Bestand oder negativ: Feld sofort auf den übernommenen Wert setzen (sonst stand 9999 bis zum Verlassen).
  if(i.value!==''&&Number(i.value)!==soll)i.value=String(soll);zeichnen();});
 document.addEventListener('change',e=>{const i=e.target.closest?.('#bodenkampf-blatt [data-bk-zahl]');if(!i||!z||z.linie==null)return;const L=lage(z.k);if(!L.w)return;const soll=begrenzen(L.w,z.entwurf)[z.linie][Number(i.dataset.bkZahl)];if(Number(i.value)!==soll)i.value=String(soll);});
 document.addEventListener('toggle',e=>{if(e.target?.matches?.('#bodenkampf-blatt [data-bk-regeln]'))try{localStorage.setItem('sternenepoche.bodenkampf.regeln',e.target.open?'auf':'zu');}catch(_){}},true);
 document.addEventListener('keydown',e=>{if(e.key!=='Escape')return;if(frage){e.preventDefault();frageZu();return;}if(offen()){e.preventDefault();zurueck();}});
 // Seitenwechsel (Leiste, Planetenwahl) schließt das Blatt – es gehört zum Ausgangsort (D1).
 document.addEventListener('game-navigation',()=>{if(offen())schliessen();if(frage)frageZu();});
 // Neue Sicht: offenes Blatt nachziehen (Countdown läuft über tickJobs/data-arrival-echt ohne Neuzeichnen).
 {const ansicht=renderView;renderView=function(){ansicht.apply(this,arguments);try{if(offen())zeichnen();if(frage)frageZeichnen();}catch(err){console.error('Bodenkampf',err);}};}
 // Android-Zurück: erst die Bestätigung, dann die Zuteilung, dann das Blatt. Liegt die Einsatzplanung darüber, schließt sie zuerst.
 document.addEventListener('DOMContentLoaded',()=>{window.Rahmen?.zurueckHaken?.push(()=>{if(!frage&&!offen())return false;const es=el('einsatz-blatt');if(es&&!es.hidden&&!frage)return false;return zurueck();});});
 return {oeffnen,schliessen,zurueck,bestaetigen,miliz,milizRufen,milizEntlassen,lage,offen,MILIZ_SEK_JE_1000};
})();
