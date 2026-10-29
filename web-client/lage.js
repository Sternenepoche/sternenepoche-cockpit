// Lagebericht oben in der Übersicht (18.10.2026): Aufstieg mit echtem Knopf (Aktion stufenaufstieg, Haltezeit 0 in
// Ablauf-Version 1), kostenlose Grundausbildung des gewählten Planeten und laufende Invasionen (Bombardierung,
// Belagerung, Miliz, Bodenhilfe). Nur Klassen und <progress> – die Spiel-CSP (style-src 'self') sperrt style-Attribute.
(function(){
 'use strict';
 const el=id=>document.getElementById(id);
 const h=s=>typeof esc==='function'?esc(s):String(s??'');
 const zahl=n=>typeof fmt==='function'?fmt(Math.round(n)):String(Math.round(n));
 const dauer=s=>{s=Math.max(0,Math.round(s||0));if(typeof GameTime!=='undefined'&&GameTime.duration)return GameTime.duration(s);return [Math.floor(s/3600),Math.floor(s/60)%60,s%60].map(n=>String(n).padStart(2,'0')).join(':');};
 const name=k=>typeof named==='function'?named(k):k;
 const badge=(k,o)=>window.Auftraege?Auftraege.badge(k,o):'';
 const hatView=()=>typeof view!=='undefined'&&view;

 // Jede offene Bedingung führt mit einem Tipp zum passenden Bau (Karl 19.10.2026: „über die UI ist fast keine Funktion
 // echt verfügbar“). Ziel: [Kategorie:Schlüssel] für data-bs-springe (öffnet Seite und Detailfeld) oder ein Bereich.
 function bedingungZiel(b){
  const c=typeof rules!=='undefined'?rules?.catalog:null,t=String(b.text||'');
  const stufe=t.match(/^([a-z_]+) Stufe (\d+)/);
  if(stufe){const k=stufe[1];if(c?.forschung?.[k])return {springe:'research:'+k,text:'Erforschen',titel:`${name(k)} erforschen`};return {springe:'buildings:'+k,text:'Ausbauen',titel:`${name(k)} ausbauen`};}
  const heim=(view.planeten||[]).find(p=>p.heimat)||(view.planeten||[])[0];
  if(/Einwohner/.test(t)){const voll=heim&&heim.wohnraum&&heim.bevoelkerung>=heim.wohnraum*0.9;return {springe:'buildings:wohnblock',text:'Wohnblock',titel:voll?'Wohnraum voll – Wohnblock ausbauen':'Wohnblock ausbauen: mehr Wohnraum = schnelleres Wachstum'};}
  if(/Energie im Plus/.test(t)){const strom=heim?.energie&&heim.energie.erzeugung<heim.energie.verbrauch;return strom?{springe:'buildings:solarkraftwerk',text:'Solarkraftwerk',titel:'Strommangel – Solarkraftwerk ausbauen'}:{springe:'buildings:farm',text:'Farm',titel:'Nahrung fehlt – Farm ausbauen'};}
  if(/Stabilität/.test(t))return {tab:'kolonie',text:'Verbessern',titel:'Stabilität verbessern: Steuern, Konsumgüter, Wohnraum'};
  if(/Konsumgüter/.test(t))return {springe:'buildings:konsumgueterwerk',text:'Ausbauen',titel:'Konsumgüterwerk ausbauen'};
  if(/Kolonien/.test(t))return {tab:'kolonien',text:'Kolonisieren',titel:'Kolonie gründen'};
  return null;
 }
 // „erzmine Stufe 5 (jetzt 1)“ → „Erzmine Stufe 5 (jetzt 1)“ – der Kern liefert Schlüssel, die Oberfläche Namen.
 const bedingungText=t=>String(t||'').replace(/^([a-z_]+)(?= Stufe )/,k=>name(k));
 function aufstieg(){
  const n=view.naechste_stufe;if(!n)return '<section class="lage-karte"><h3>Zivilisationsstufe</h3><p>Die höchste Stufe ist erreicht.</p></section>';
  const offen=(n.bedingungen||[]).filter(b=>!b.erfuellt),halte=Number(n.haltezeit_stunden)||0,seit=Number(n.erfuellt_seit_stunden)||0;
  const heim=(view.planeten||[]).find(p=>p.heimat)||(view.planeten||[])[0];
  const fehlt=Object.entries(n.kosten||{}).filter(([g,m])=>(heim?.bestand?.[g]||0)<m);
  const bereit=!offen.length&&seit>=halte,zahlbar=!fehlt.length;
  const grund=offen.length?`Noch offen: ${offen.length} Bedingung${offen.length===1?'':'en'}`:seit<halte?`Bedingungen seit ${seit} von ${halte} Spielstunden erfüllt`:!zahlbar?'Rohstoffe auf der Heimatwelt fehlen: '+fehlt.map(([g,m])=>`${name(g)} ${zahl(m-(heim?.bestand?.[g]||0))}`).join(', '):'';
  return `<section class="lage-karte lage-aufstieg${bereit&&zahlbar?' bereit':''}"><h3>Nächste Zivilisationsstufe: ${h(n.name)}</h3>
   <ul class="lage-bedingungen">${(n.bedingungen||[]).map(b=>{const z=b.erfuellt?null:bedingungZiel(b);return `<li class="${b.erfuellt?'ok':'offen'}"><span>${h(bedingungText(b.text))}</span>${z?`<button type="button" class="lage-schritt" title="${h(z.titel||z.text)}" aria-label="${h(z.titel||z.text)}" ${z.springe?`data-bs-springe="${h(z.springe)}"`:`data-tab="${h(z.tab)}"`}>${h(z.text)} ›</button>`:''}</li>`;}).join('')}</ul>
   ${(n.hinweise||[]).length?`<div class="lage-hinweise"><b>Was als Nächstes hilft</b><ul>${n.hinweise.map(t=>`<li>${h(typeof kernLesbar==='function'?kernLesbar(String(t)):t)}</li>`).join('')}</ul></div>`:''}
   <p class="lage-kosten">Kosten auf der Heimatwelt: ${Object.entries(n.kosten||{}).map(([g,m])=>`<span class="${(heim?.bestand?.[g]||0)<m?'fehlt':''}">${h(name(g))} ${zahl(m)}</span>`).join(' · ')||'keine'}</p>
   <div class="lage-knoepfe"><button type="button" class="lage-aufstieg-knopf" data-promote ${bereit&&zahlbar?'':'disabled'} title="${h(grund)}">Aufstieg zu ${h(n.name)}</button>${grund?`<small>${h(grund)}</small>`:'<small>Alles erfüllt – Aufstieg sofort möglich.</small>'}</div></section>`;
 }
 function grundausbildung(){
  const p=typeof currentPlanet==='function'?currentPlanet():null,g=p?.grundausbildung;if(!g)return '';
  const fehlen=(g.fehlen_fachkraefte||0)+(g.fehlen_forscher||0);
  // Ruht die Akademie, den konkreten Grund aus der Werksbilanz nennen (statt nur „Besetzung, Energie, … oder Regler“).
  const werk=(p.industrie?.werke||[]).find?.(w=>w.gebaeude==='akademie'),gruende=[];
  if(werk&&Number(g.je_tag)===0&&(p.gebaeude?.akademie||0)>0){
   const E={arbeiter:'Arbeiter',fachkraefte:'Fachkräfte',energie:'Energie',eingang:'Eingangsgüter'};
   for(const e of werk.engpaesse||[])gruende.push(e==='arbeiter'?`Arbeiter fehlen (${zahl(werk.arbeiter_zugewiesen)} von ${zahl(werk.arbeiter_bedarf)})`:e==='fachkraefte'?`Fachkräfte fehlen (${zahl(werk.fachkraefte_zugewiesen)} von ${zahl(werk.fachkraefte_bedarf)})`:(E[e]||e)+' fehlt');
   if((werk.integritaet_promille??1000)<1000)gruende.push(`Integrität ${Math.round((werk.integritaet_promille||0)/10)} % – zuerst reparieren`);
   if(p.energie&&p.energie.erzeugung<p.energie.verbrauch)gruende.push(`Energiedefizit (${zahl(p.energie.erzeugung)} erzeugt, ${zahl(p.energie.verbrauch)} Bedarf)`);
   if(p.industrie?.regler?.akademie===0)gruende.push('Leistungsregler auf 0 %');
  }
  return `<section class="lage-karte lage-ausbildung${fehlen?' warn':''}"><h3>Grundausbildung · [${h(p.koord)}]</h3><p>${h(g.hinweis)}</p>${gruende.length?`<p class="lage-grund"><b>Grund:</b> ${gruende.map(h).join(' · ')}</p>`:''}
   <div class="lage-zahlen"><span><b>${zahl(g.fachkraefte)}</b> / ${zahl(g.ziel_fachkraefte)} Fachkräfte</span><span><b>${zahl(g.forscher)}</b> / ${zahl(g.ziel_forscher)} Forscher</span><span><b>${(Number(g.je_tag)||0).toLocaleString('de-DE',{maximumFractionDigits:1})}</b> je Spieltag</span></div>
   <div class="lage-knoepfe"><button type="button" class="secondary" data-open-academy>Akademie öffnen</button></div></section>`;
 }
 // Invasion v2 (Karl 24.10.2026: „Wenn man selber angegriffen wird, soll man das in der Planetenübersicht gleich sehen, dass es
 // eine Invasion gibt, und wenn man Miliz erforscht hat, kann man … mit einem Button gleich ‚Miliz rufen‘“): rote Kopfzeile,
 // die Verteidigungslinien als Reihe mit Status und Truppen, Countdown zur nächsten Welle (Echtzeit, data-arrival-echt) und die
 // nächsten Schritte. Angreifer und Helfer sehen keine Verteidigerzahlen (der Kern liefert sie ihnen nicht).
 function invasionV2(i){
  const w=i.wellen,k=h(i.planet),p=(view.planeten||[]).find(x=>x.koord===i.planet),def=i.rolle==='verteidiger'||i.rolle==='verbuendeter';
  const name=p?(window.Planetenname?.name?Planetenname.name(p):(p.name||'')):'';
  const titel={verteidiger:'Invasion!',verbuendeter:'Invasion beim Verbündeten',angreifer:'Deine Invasion',angriffshelfer:'Invasion deines Verbündeten'}[i.rolle]||'Invasion';
  const uhr=t=>'Tag '+(Math.floor(t/86400)+1)+' · '+[Math.floor(t/3600)%24,Math.floor(t/60)%60].map(n=>String(n).padStart(2,'0')).join(':');
  const kopf=`<div class="iv2-kopf"><h3>${badge(w?'kampfkolonisieren':'bombardieren',{size:18})} ${h(titel)} · <span class="koord">[${k}]</span></h3><small>${name?h(name)+' · ':''}${i.rolle==='angreifer'||i.rolle==='angriffshelfer'?'Besitzer '+h(i.besitzer||''):'Angreifer '+h(w?.angreifer_name||i.bombardement?.angreifer_name||'')}</small></div>`;
  const knopf=[],M=i.rolle==='verteidiger'&&window.Bodenkampf?Bodenkampf.miliz(p):null;
  if(!w){
   // Vor der Landung: Bombardierung (wie bisher), Nachschub schon möglich.
   const b=i.bombardement;
   if(i.rolle==='verteidiger')knopf.push(`<button type="button" data-ff-ziel-plan="${k}" data-ff-auftrag-plan="stationieren">Nachschub landen<small>Bodentruppen auf den Planeten verlegen</small></button>`);
   if(i.rolle==='angreifer')knopf.push(`<button type="button" class="iv2-haupt" data-ff-ziel-plan="${k}" data-ff-auftrag-plan="kampfkolonisieren">Landung vorbereiten<small>Kolonieschiff und Bodentruppen</small></button>`);
   return `<section class="lage-karte lage-invasion iv2 iv2-${h(i.rolle)}">${kopf}<div class="iv2-inhalt">${b?`<div class="lage-phase"><b>Bombardierung</b> · ${h({laeuft:'läuft',unterbrochen:'unterbrochen',ohne_bomber:'ohne Bomber',abgeschlossen:'abgeschlossen'}[b.status]||b.status)}<progress max="100" value="${Number(b.fortschritt_prozent)||0}"></progress><small>Gebäude-Integrität ${(Number(b.integritaet_hoechste_prozent)||0).toLocaleString('de-DE')} % → Ziel ${b.ziel_prozent} % · ${b.runden_bis_ziel>0?`${b.runden_bis_ziel} Runden · noch ${dauer(b.restzeit_sekunden)}`:'Ziel erreicht – Landung möglich'}</small></div>`:''}<p class="iv2-zeile">Die Verteidigungslinien stehen ab der Landung der Bodentruppen.</p>${knopf.length?`<div class="iv2-knoepfe">${knopf.join('')}</div>`:''}</div></section>`;
  }
  const n=w.linien_anzahl,t=Number(w.naechste_welle)||0,ziel=Number(w.linie)>=n?'den Rückhalt':'Linie '+zahl(w.linie);
  const tippbar=l=>i.rolle==='verteidiger'&&!w.verwuestet&&!l.rueckhalt&&l.status!=='gebrochen';
  const linien=`<ol class="iv2-linien" data-css="--iv2-n:${n}" aria-label="Verteidigungslinien">${(w.linien||[]).map(l=>{const inhalt=`<small>${l.rueckhalt?'Rückhalt':'Linie '+zahl(l.linie)}</small><b>${l.status==='gebrochen'?'gebrochen':def&&l.gesamt!=null?zahl(l.gesamt):l.status==='aktuell'?'Front':'–'}</b>${l.status==='aktuell'&&!w.verwuestet?'<small>nächste Welle</small>':''}`;
   return `<li class="${h(l.status)}${l.rueckhalt?' rueckhalt':''}">${tippbar(l)?`<button type="button" class="iv2-linie-knopf" data-bk-oeffnen="${k}" data-bk-zu-linie="${Number(l.linie)-1}" aria-label="Linie ${zahl(l.linie)}: ${def&&l.gesamt!=null?zahl(l.gesamt):'?'} Truppen – verteilen">${inhalt}</button>`:inhalt}</li>`;}).join('')}</ol>`;
  const welle=w.verwuestet?`<p class="iv2-verwuestet"><b>Verwüstet</b> – ${i.rolle==='verteidiger'?'du hast die Kontrolle verloren. Lande neue Truppen, dann steht der Rückhalt wieder.':'ein Kolonieschiff (Kampfkolonisieren oder Kolonisieren) kann landen und übernehmen.'}</p>`
   :`<p class="iv2-welle">Nächste Welle in <b data-arrival-echt="${t}"></b><small>${h(uhr(t))} · trifft ${h(ziel)} · ${zahl(w.wellen)} ${Number(w.wellen)===1?'Welle':'Wellen'} geschlagen</small></p>`;
  const a=(w.angreifer_truppen||[]).reduce((s,x)=>s+(Number(x)||0),0),v=(w.linien||[]).reduce((s,l)=>s+(Number(l.gesamt)||0),0);
  const kraefte=`<p class="iv2-zeile">Angreifer am Boden <b>${zahl(a)}</b>${def?` · deine Verteidiger <b>${zahl(v)}</b>`:''}</p>`;
  if(i.rolle==='verteidiger'){
   knopf.push(`<button type="button" class="iv2-haupt" data-bk-oeffnen="${k}">${w.verwuestet?'Bodenkampf ansehen<small>Linien, Wellen, Berichte</small>':'Linien verteilen<small>jede Linie antippen, Truppen zuteilen</small>'}</button>`);
   if(M?.rufbar)knopf.push(`<button type="button" class="iv2-miliz" data-bk-miliz-rufen="${k}">Miliz rufen<small>${zahl(M.m.verfuegbar)} Milizionäre · ${zahl(M.kosten)} Legierung${M.ausbildung&&window.Bauseite?.dauer?' · '+h(Bauseite.dauer(M.ausbildung/Math.max(1,world?.tempo||1))):''}</small></button>`);
   else if(M?.m?.ausgerufen)knopf.push(`<button type="button" data-bk-oeffnen="${k}">Miliz ${M.bereit?'einsatzbereit':'in Ausbildung'}<small>${h(M.zustand)}</small><progress max="${Math.max(1,M.aufgebot)}" value="${Math.min(M.staerke,M.aufgebot)}"></progress></button>`);
   knopf.push(`<button type="button" data-bk-ausbilden="${k}">Truppen ausbilden<small>${w.verwuestet?'neue Truppen besetzen den Rückhalt wieder':`während der Invasion ${Math.round(100-(Number(w.ausbildung_promille)||1000)/10)} % langsamer · stehen im Rückhalt`}</small></button>`);
   knopf.push(`<button type="button" data-ff-ziel-plan="${k}" data-ff-auftrag-plan="stationieren">Nachschub landen<small>Bodentruppen von einem anderen Planeten</small></button>`);
   if(M&&!M.rufbar&&!M.m?.ausgerufen&&M.grund)knopf.push(`<p class="iv2-hinweis">Miliz: ${h(M.grund)}${M.schritt?` <button type="button" class="bk-schritt" data-bk-springe="${h(M.schritt.springe)}">${h(M.schritt.text)} ›</button>`:''}</p>`);
  }else if(i.rolle==='verbuendeter'){
   knopf.push(`<button type="button" class="iv2-haupt" data-bk-oeffnen="${k}">Bodenkampf ansehen<small>Linien, Wellen, Berichte</small></button>`,`<button type="button" data-ff-ziel-plan="${k}" data-ff-auftrag-plan="verstaerken">Bodentruppen schicken<small>verstärken den Rückhalt</small></button>`,`<button type="button" data-ff-ziel-plan="${k}" data-ff-auftrag-plan="halten">Flotte halten lassen<small>im Orbit mitverteidigen</small></button>`);
  }else{
   knopf.push(`<button type="button" class="iv2-haupt" data-bk-oeffnen="${k}">Bodenkampf ansehen<small>Linien, Wellen, Berichte</small></button>`);
   // Eigenes Kolonieschiff wartet schon im Orbit auf einen Platz (Einsatzprobe 25.10.2026: die Karte bot trotzdem „Kolonieschiff
   // schicken“ an): Wartezustand mit Ausweg zeigen – Forschung für den nächsten Platz oder eine Kolonie aufgeben.
   const wartet=w.verwuestet&&(view.flotten||[]).some(f=>f.ziel===k&&f.zustand==='im_orbit'&&['kolonisieren','kampfkolonisieren'].includes(f.mission)&&((f.schiffe?.kolonieschiff||0)+(f.schiffe?.grosses_kolonieschiff||0))>0);
   const ko=view.kolonien||{};
   if(wartet)knopf.push(`<p class="iv2-hinweis">Dein Kolonieschiff wartet im Orbit und übernimmt, sobald ein Kolonieplatz frei ist (${zahl(ko.anzahl||0)} von ${zahl(ko.erlaubt||0)} belegt).</p>`,
    `<button type="button" data-go="forschen">${ko.naechste_stufe?`${h(named(ko.forschung||'terraforming'))} ${zahl(ko.naechste_stufe)} erforschen`:'Forschung öffnen'}<small>bringt den nächsten Kolonieplatz</small></button>`,
    `<button type="button" class="secondary" data-go="kolonien">Kolonien verwalten<small>eine Kolonie aufgeben macht den Platz sofort frei</small></button>`);
   else if(w.verwuestet)knopf.push(`<button type="button" data-ff-ziel-plan="${k}" data-ff-auftrag-plan="kampfkolonisieren">Kolonieschiff schicken<small>landet mit freiem Kolonieplatz und übernimmt</small></button>`);
   else knopf.push(`<button type="button" data-ff-ziel-plan="${k}" data-ff-auftrag-plan="${i.rolle==='angriffshelfer'?'verstaerken':'kampfkolonisieren'}">Truppen nachschicken<small>kämpfen ab der nächsten Welle mit</small></button>`);
  }
  return `<section class="lage-karte lage-invasion iv2 iv2-${h(i.rolle)}">${kopf}<div class="iv2-inhalt">${welle}${linien}${kraefte}<div class="iv2-knoepfe">${knopf.join('')}</div></div></section>`;
 }
 function invasion(i){
  if(i.wellen||(view.planeten||[]).find(x=>x.koord===i.planet)?.invasion?.regeln==='invasion-v2')return invasionV2(i);
  const b=i.bombardement,bel=i.belagerung,m=i.miliz||{},rolle={verteidiger:'Dein Planet wird angegriffen',angreifer:'Deine Invasion',verbuendeter:'Verbündeter in Not'}[i.rolle]||i.rolle;
  const knopf=[];
  if(i.rolle==='verteidiger'&&m.verfuegbar>0)knopf.push(`<button type="button" class="lage-gefahr" data-bk-miliz-rufen="${h(i.planet)}">Miliz ausrufen (${zahl(m.verfuegbar)} Bewohner · ${zahl(m.kosten?.gesamt_legierung||0)} Legierung)</button>`);
  if(i.rolle!=='angreifer')knopf.push(`<button type="button" class="secondary" data-ff-ziel-plan="${h(i.planet)}" data-ff-auftrag-plan="${!('invasionen' in view)?'halten':i.rolle==='verteidiger'?'stationieren':'verstaerken'}">${badge(i.rolle==='verteidiger'?'stationieren':'verstaerken',{size:16})} ${i.rolle==='verteidiger'?'Eigene Bodentruppen landen':'Bodentruppen schicken'}</button><button type="button" class="secondary" data-ff-ziel-plan="${h(i.planet)}" data-ff-auftrag-plan="halten">${badge('halten',{size:16})} Flotte halten lassen</button>`);
  if(i.rolle==='angreifer'&&!bel)knopf.push(`<button type="button" class="secondary" data-ff-ziel-plan="${h(i.planet)}" data-ff-auftrag-plan="kampfkolonisieren">${badge('kampfkolonisieren',{size:16})} Landung vorbereiten</button>`);
  return `<section class="lage-karte lage-invasion lage-${h(i.rolle)}"><h3>${badge(bel?'kampfkolonisieren':'bombardieren',{size:20})} ${h(rolle)} · [${h(i.planet)}]</h3>
   ${b?`<div class="lage-phase"><b>Bombardierung</b> · ${h({laeuft:'läuft',unterbrochen:'unterbrochen',ohne_bomber:'ohne Bomber',abgeschlossen:'abgeschlossen'}[b.status]||b.status)} · ${h(b.angreifer_name||'')}
     <progress max="100" value="${Number(b.fortschritt_prozent)||0}"></progress><small>Gebäude-Integrität ${(Number(b.integritaet_hoechste_prozent)||0).toLocaleString('de-DE')} % → Ziel ${b.ziel_prozent} % · ${b.runden_bis_ziel>0?`${b.runden_bis_ziel} Runden · noch ${dauer(b.restzeit_sekunden)}`:'Ziel erreicht – Landung möglich'} · ${zahl(b.bombenlast_je_runde)} Bombenlast je 15 min</small></div>`:''}
   ${bel?`<div class="lage-phase"><b>${i.regeln==='invasion-v2'?'Invasion':'Belagerung'}</b> · noch ${dauer(bel.restzeit_sekunden)}
     <progress max="${bel.dauer_sekunden||172800}" value="${(bel.dauer_sekunden||172800)-(bel.restzeit_sekunden||0)}"></progress><small>${bel.verteidiger_gesamt!=null?`Bodenverteidiger ${zahl(bel.verteidiger_gesamt)}`:'Bodenverteidiger unbekannt'}${bel.bodenverteidigung_gebrochen?' – gebrochen':''}${bel.verteidiger?` · verbündete Hilfe ${(bel.verteidiger.verbuendete||[]).length}`:''} · nächste Bodenrunde in ${dauer((bel.naechste_bodenrunde||0)-view.sekunden)}</small></div>`:''}
   ${i.rolle!=='angreifer'&&m.regeln?`<div class="lage-phase"><b>Volksmiliz</b> · ${m.kapituliert?'kapituliert':m.ausgerufen?`ausgerufen, Stärke ${zahl(m.staerke)} · gefallen ${zahl(m.gefallen||0)} von ${zahl(m.kapitulation_bei_gefallenen||0)} bis zur Kapitulation`:m.freigeschaltet?(m.belagert?'bereit':'erst während einer Belagerung'):h(m.freischaltung||'nicht freigeschaltet')}</div>`:''}
   ${knopf.length?`<div class="lage-knoepfe">${knopf.join('')}</div>`:''}</section>`;
 }
 function zeichnen(){
  if(!hatView())return;const reich=el('reich');if(!reich)return;
  let host=el('lagebericht');if(!host){host=document.createElement('div');host.id='lagebericht';host.className='lagebericht';// Hinter den Kopfblock der Übersicht, nicht in dessen Flex-Zeile (sonst steht der Bericht neben der Überschrift
  // und schiebt die Seite auf dem Handy über 375 px hinaus).
  const h2=reich.querySelector('h2');let anker=h2;while(anker&&anker.parentElement&&anker.parentElement!==reich)anker=anker.parentElement;if(anker&&anker.parentElement===reich)anker.after(host);else reich.prepend(host);}
  // Eigene Planeten unter Invasion ganz oben (Karl 24.10.2026: „soll man das in der Planetenübersicht gleich sehen“) – vor dem
  // Planetenkopf der Übersicht (Handy: CSS-order 0 in bodenkampf.css), alle übrigen Lagekarten wie bisher darunter.
  const liste=view.invasionen||[],oben=liste.filter(i=>i.rolle==='verteidiger');
  let alarm=el('invasion-lage');
  if(oben.length&&!alarm){alarm=document.createElement('div');alarm.id='invasion-lage';alarm.className='lagebericht';host.before(alarm);}
  if(alarm){const html=oben.map(invasion).join('');if(alarm.dataset.stand!==html){alarm.dataset.stand=html;setStableHTML(alarm,html);}alarm.hidden=!oben.length;}
  const inv=liste.filter(i=>i.rolle!=='verteidiger').map(invasion).join('');
  const html=`${inv}${aufstieg()}${grundausbildung()}`;
  if(host.dataset.stand!==html){host.dataset.stand=html;if(typeof setStableHTML==='function')setStableHTML(host,html);else host.innerHTML=html;}
 }
 // Miliz ausrufen: Bestätigungsblatt mit den Folgen (bodenkampf.js, data-bk-miliz-rufen) statt confirm() (DESIGNREGELN §9).
 let stand=null;setInterval(()=>{if(!hatView())return;const r=view.revision+':'+view.sekunden+':'+(typeof selectedPlanet!=='undefined'?selectedPlanet:'');if(r!==stand){stand=r;try{zeichnen();}catch(err){console.error('Lagebericht',err);}}},1000);
 document.addEventListener('game-navigation',()=>setTimeout(zeichnen,30));
 window.Lagebericht={zeichnen};
})();
