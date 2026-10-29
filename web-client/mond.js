'use strict';
const silbenHtml = s => (window.Silben ? Silben.html(s) : s);  // Kachelnamen nach Silben trennen, nie mitten im Wort
// Mondseite nach docs/DESIGNREGELN.md §2 (Banner → Kacheln → Detailfeld → Warteschlangen) für die Mondgeneration
// des Kerns (view.monde.eigene[], docs/MONDKONZEPT.md „Beschlossen“). Befehle: mond_bauen, mond_fertigen, mond_faehre,
// sprungtor_freigabe, hyperraumloch_oeffnen, hyperraumloch_durchreisen, phalanx_scannen – alle über command().
const Mondseite=(()=>{
 const GEBAEUDE={mondbasis:'Mondbasis',tiefenphalanx:'Tiefenphalanx',sprungtor:'Hyperraumtor-Generator',regolithfoerderung:'Regolithförderung',kryolager:'Deuterium-Kühllager',werftdock:'Werftdock',ortungsschleier:'Ortungsschleier'};
 const TYPEN={krater_grau:'Grauer Kratermond',basalt_dunkel:'Dunkler Basaltmond',eis_rissig:'Rissiger Eismond',mineral_blau:'Blauer Mineralmond',organisch_gruen:'Grüner Organikmond',rost_staub:'Rostiger Staubmond',schluucht_tief:'Schluchtenmond',vulkan_glut:'Glutvulkanmond',amethyst_violett:'Amethystmond',karmin_rot:'Karminmond'};
 const name=k=>GEBAEUDE[k]||named(k);
 const typName=t=>TYPEN[t]||named(t);
 let aktiv='',offen=null,offenEinheit=null,menge=1;
 const tempo=()=>Math.max(1,world?.tempo||1);
 const dauer=s=>window.Bauseite?Bauseite.dauer(s/tempo()):GameTime.duration(s);
 const eigene=()=>view?.monde?.aktiv?view.monde.eigene||[]:[];
 const mond=()=>eigene().find(m=>m.koord===aktiv)||eigene().find(m=>m.koord===currentPlanet()?.koord)||eigene()[0];

 // Mondbild: Kartenmotiv aus dem Bildkatalog (moons.<typ>[.<volk>].map), sonst eine kleine farbige Kugel.
 function bild(m,klasse=''){
  const img=window.STERNEN_ART?.images||{},volk=view?.volk||'',t=m?.typ||'krater_grau';
  // Gleiches Motiv wie im Sonnensystem und in der Planetenansicht: Kugel aus der Mondkarte (planet-kugel.js).
  const kugel=window.PlanetKugel?.kugelbild('moons',t,m?.volk||volk),eintrag=kugel?{url:kugel}:img[`moons|${t}_${volk}|${volk}`]||img[`moons|${t}|`];
  const titel=m?`${m.name||'Mond'} · ${typName(t)} · ${fmt(m.durchmesser||m.groesse||0)} km`:'Mond';
  return eintrag?`<img class="mond-bild ${klasse}" src="${esc(eintrag.url)}" alt="${esc(titel)}" title="${esc(titel)}">`:`<span class="mond-kugel mond-${esc(t)} ${klasse}" role="img" aria-label="${esc(titel)}" title="${esc(titel)}"></span>`;
 }
 function gebaeudeBild(k){return art('buildings',k,view.volk);}
 function kosten(k,bestand,n=1){return Object.entries(k||{}).filter(([,v])=>v>0).map(([g,v])=>`<span class="bs-gut${(bestand?.[g]||0)<v*n?' knapp':''}" title="${esc(named(g))}: ${v.toLocaleString('de-DE',{maximumFractionDigits:2})} · auf dem Mond ${fmt(Math.floor(bestand?.[g]||0))}"><img src="${esc(art('resources',g,''))}" alt="${esc(named(g))}"><b>${v.toLocaleString('de-DE',{maximumFractionDigits:2})}</b></span>`).join('')||'<span class="bs-gut">kostenlos</span>';}
 const ausgebaut=m=>m.felder.belegt>=m.felder.verfuegbar;
 // Spielername des Mondgebäudes „sprungtor“ ist Hyperraumtor-Generator (Karl 22.10.2026); Regeltexte tragen noch den alten Namen.
 const torText=t=>String(t??'').replace(/Sprungtoren\b/g,'Hyperraumtor-Generatoren').replace(/Sprungtore\b/g,'Hyperraumtor-Generatoren').replace(/Sprungtors\b/g,'Hyperraumtor-Generators').replace(/Sprungtor\b/g,'Hyperraumtor-Generator');
 // Kern meldet fehlende Voraussetzungen mit Schlüsseln („mondbasis 3“, „hyperraumantrieb 3“): lesbar machen.
 const fehltText=f=>{const x=/^([a-z_]+) (\d+)$/.exec(String(f));if(x)return (GEBAEUDE[x[1]]||named(x[1]))+' '+x[2];return f==='freies Feld'?'kein freies Mondfeld (Mondbasis ausbauen: +3 Felder)':torText(f);};
 // Mondaufträge als Spielaufträge für die gemeinsame Fortschrittsanzeige (game-ui.js jobLine/tickGame): Restzeit in Echtzeit,
 // Balken füllt sich – sichtbar sofort nach dem Tipp (Karl 22.10.: „ich muss den Timer sehen“).
 const mondJob=(m,a)=>({planet:'mond:'+m.koord,gebaeude:a.gebaeude,stufe:a.stufe,fertig_sekunden:a.fertig??null,dauer_sekunden:a.dauer||1,wartet:a.fertig==null});
 // Ein Tipp direkt unter der Kachel baut (Karl 22.10.: „nicht erst draufklicken, öffnen lassen und dann bauen“).
 function sofortBau(m,b){
  const a=m.bauschleife.find(x=>x.gebaeude===b.gebaeude);
  if(a)return `<div class="mond-sofort-lauf" data-css="min-width:0">${jobLine(mondJob(m,a))}</div>`;
  const grund=b.fehlt.length?fehltText(b.fehlt[0]):b.fehlende_rohstoffe.length?'Rohstoffe fehlen auf dem Mond':'',ok=b.moeglich&&!gameReadOnly();
  const kurz=b.fehlt[0]==='freies Feld'?'Kein Feld frei':b.fehlt.length?'Fehlt: '+fehltText(b.fehlt[0]):b.fehlende_rohstoffe.length?'Rohstoffe fehlen':'gesperrt';
  // Die Mondbasis darf der Kern auch auf vollem Mond bauen (Ablauf v3, eine Stufe zur Zeit); das letzte freie Feld für ein
  // anderes Gebäude zu verbrauchen, hält den Mond trotzdem an – darum fragt der Knopf dann nach (data-mond-letztes).
  const letztes=ok&&b.gebaeude!=='mondbasis'&&m.felder.verfuegbar-m.felder.belegt===1&&m.felder.verfuegbar<m.felder.obergrenze;
  return `<button type="button" class="bs-bauen bs-klein mond-sofort" data-mond-bauen="${esc(b.gebaeude)}" ${letztes?'data-mond-letztes':''} ${ok?'':'disabled'} title="${esc(grund||'Stufe '+b.stufe+' bauen')}" data-css="width:100%;min-height:34px;font-size:12px;line-height:1.2;white-space:normal">${ok?(b.stufe>1?'Stufe '+b.stufe+' bauen':'Bauen')+(letztes?' · letztes Feld':''):esc(kurz)}</button>`;
 }

 function kachelGebaeude(m,b){
  const stufe=m.gebaeude[b.gebaeude]||0,gesperrt=b.fehlt.filter(f=>f!=='freies Feld').length>0,teuer=!gesperrt&&b.fehlende_rohstoffe.length>0,baut=m.bauschleife.some(a=>a.gebaeude===b.gebaeude);
  return `<div class="mond-kachel" id="mkw-${esc(b.gebaeude)}" data-css="display:flex;flex-direction:column;gap:4px;min-width:0"><button type="button" id="mk-${esc(b.gebaeude)}" class="bs-kachel${gesperrt?' gesperrt':''}${teuer?' teuer':''}${baut?' baut':''}${offen===b.gebaeude?' gewaehlt':''}" data-mond-geb="${esc(b.gebaeude)}" aria-pressed="${offen===b.gebaeude}" title="${esc(name(b.gebaeude)+' – Stufe '+stufe+' – '+torText(b.wirkung)+(b.fehlt.length?' – Fehlt: '+b.fehlt.map(fehltText).join(' · '):''))}"><span class="bs-bild"><img src="${esc(gebaeudeBild(b.gebaeude))}" alt="" loading="lazy"></span><span class="bs-fuss"><span class="bs-name">${silbenHtml(esc(name(b.gebaeude)))}</span><span class="bs-zahl">${stufe}</span></span></button>${sofortBau(m,b)}</div>`;
 }
 function detailGebaeude(m,b){
  const stufe=m.gebaeude[b.gebaeude]||0,e=window.Bauseite?.text?.(b.gebaeude);
  const start=b.fehlt.length?['gesperrt · '+b.fehlt.map(fehltText).join(' · '),'rot']:b.fehlende_rohstoffe.length?['Rohstoffe auf dem Mond fehlen: '+b.fehlende_rohstoffe.map(named).join(', '),'rot']:m.bauschleife.length?['nach laufendem Auftrag','gelb']:['sofort','gruen'];
  // Benutzen statt nur Ausbauen: Sprungtor/Phalanx führen direkt zu ihrem Abschnitt (Handy-Durchgang 19.10.2026, M9)
  const nutzen=stufe&&{sprungtor:['mond-feld-sprungtor','Hyperraumtor öffnen ↓'],tiefenphalanx:['mond-feld-phalanx','Planet scannen ↓']}[b.gebaeude];
  return `<article class="bs-detail" id="mond-detail"><div class="bs-detail-bild"><img src="${esc(gebaeudeBild(b.gebaeude))}" alt="${esc(name(b.gebaeude))}"><button type="button" class="bs-link" data-mond-techinfo="${esc(b.gebaeude)}" data-reiter="techbaum">Techbaum</button>${nutzen?`<button type="button" class="bs-zweit" data-mond-springe="${nutzen[0]}">${nutzen[1]}</button>`:''}</div><div class="bs-detail-text"><div class="bs-detail-kopf"><h3>${esc(name(b.gebaeude))}</h3><span class="bs-stufe">Stufe ${stufe}</span><button type="button" class="bs-schliessen" data-mond-zu aria-label="Detailfeld schließen">✕</button></div><dl class="bs-fakten"><dt>Bauzeit</dt><dd>${dauer(b.bauzeit_sekunden)}</dd><dt>Baubeginn möglich</dt><dd class="bs-${start[1]}">${esc(start[0])}</dd><dt>Kosten (Mondlager)</dt><dd class="bs-kosten">${kosten(b.kosten,m.bestand)}</dd><dt>Energie</dt><dd>${b.energie?fmt(b.energie)+' je Stufe':'erzeugt Grundenergie'}</dd></dl><div class="bs-aktion"><button type="button" class="bs-bauen" data-mond-bauen="${esc(b.gebaeude)}" ${b.moeglich&&!gameReadOnly()?'':'disabled'}>Ausbauen auf Stufe ${b.stufe}</button></div><p class="bs-satz">${esc(e?.zweck||e?.rolle||torText(b.wirkung))} <button type="button" class="bs-hilfe" data-mond-techinfo="${esc(b.gebaeude)}" data-reiter="techinfo" aria-label="Techinfo">?</button></p></div></article>`;
 }
 function detailEinheit(m,u){
  // Seit 19.10. (monde.rs mond_fertigen, t15b): zuerst Mondlager, Rest vom eigenen Mutterplaneten – verfügbar ist die Summe.
  const mutter=view.planeten.find(p=>p.koord===m.koord),bestand={};for(const g of new Set([...Object.keys(m.bestand||{}),...Object.keys(mutter?.bestand||{})]))bestand[g]=(m.bestand?.[g]||0)+(mutter?.bestand?.[g]||0);
  const n=Math.max(1,menge),max=Math.max(0,Math.min(10000,...Object.entries(u.kosten).filter(([,v])=>v>0).map(([g,v])=>Math.floor((bestand[g]||0)/v))));
  // Besatzung kommt aus der Bevölkerung des Mutterplaneten (Kern: ganze Personen, Weltenkrümmer 5.000 je Stück) –
  // ohne diese Zeile meldete erst der Kern beim Bauen den Fehler.
  const proStueck=rules?.catalog?.einheiten?.[u.einheit]?.regel?.besatzung||0,leute=Math.floor(mutter?.bevoelkerung||0),crew=proStueck*n,crewFehlt=proStueck>0&&crew>leute;
  const maxCrew=proStueck>0?Math.floor(leute/proStueck):Infinity;
  const ok=u.baubar&&n<=max&&n<=maxCrew&&!gameReadOnly();
  // Startzeit nicht nur aus dem Werftdock: bezahlt wird aus dem Mondlager – fehlt dort etwas, sagt das Detail was
  // (vorher „sofort“ neben „[max. 0]“ und gesperrtem Bauen ohne Grund; Galaxie-Durchgang 19.10.2026, Punkt 7).
  const fehlt=Object.entries(u.kosten).filter(([g,v])=>v>0&&(bestand[g]||0)<v*n).map(([g,v])=>named(g)+' '+fmt(Math.ceil(v*n-(bestand[g]||0))));
  const start=!u.baubar?['rot','gesperrt · Werftdock '+u.werftdock+' oder Forschung fehlt']:crewFehlt?['rot','Besatzung fehlt · '+fmt(crew)+' Personen nötig, Mutterplanet hat '+fmt(leute)]:fehlt.length?['rot','Mondlager und Mutterplanet reichen nicht · es fehlen '+fehlt.join(', ')]:['gruen','sofort (Werftdock)'];
  return `<article class="bs-detail" id="mond-detail"><div class="bs-detail-bild"><img src="${esc(Bauseite.bildVon?Bauseite.bildVon(u.schiff?'ships':'defenses',{key:u.einheit}):art('ships',u.einheit))}" alt="${esc(named(u.einheit))}"><button type="button" class="bs-link" data-bs-techinfo="${esc(u.einheit)}" data-bs-kat="${u.schiff?'ships':'defenses'}" data-bs-reiter="techbaum">Techbaum</button>${u.einheit==='weltentruemmer'?`<button type="button" class="bs-link" data-wk-3d="${esc(view.volk)}">3D-Verwandlung ansehen</button>${art('megaships','todesmond_werft',view.volk).includes('todesmond_werft')?`<figure class="mond-werftbild"><img src="${esc(art('megaships','todesmond_werft',view.volk))}" alt="Weltenkrümmer im Bau in der Mond-Orbitalwerft" loading="lazy"><figcaption>Bau in der Mond-Orbitalwerft</figcaption></figure>`:''}`:''}</div><div class="bs-detail-text"><div class="bs-detail-kopf"><h3>${esc(named(u.einheit))}</h3><span class="bs-stufe">Auf dem Mond ${fmt(m.schiffe[u.einheit]||m.verteidigung[u.einheit]||0)}</span><button type="button" class="bs-schliessen" data-mond-zu aria-label="Detailfeld schließen">✕</button></div><dl class="bs-fakten"><dt>Bauzeit je Stück</dt><dd>${dauer(u.bauzeit_sekunden)}</dd><dt>Baubeginn möglich</dt><dd class="bs-${start[0]}">${esc(start[1])}${u.baubar&&fehlt.length?` <button type="button" class="bs-zweit" data-mond-springe="mond-feld-faehre">Mondlager füllen ›</button>`:''}</dd>${proStueck?`<dt>Besatzung je Stück</dt><dd class="bs-${crewFehlt?'rot':'gruen'}">${fmt(proStueck)} Personen · Mutterplanet ${fmt(leute)}</dd>`:''}<dt>Kosten je Stück</dt><dd class="bs-kosten">${kosten(u.kosten,bestand,n)}</dd></dl><div class="bs-aktion bs-formular"><label class="bs-anzahl">Anzahl <input id="mond-anzahl" type="number" min="1" max="${Math.max(1,max)}" value="${n}" inputmode="numeric"></label><button type="button" class="bs-max" data-mond-max="${Math.min(max,maxCrew)}">[max. ${fmt(Math.min(max,maxCrew))}]</button><button type="button" class="bs-bauen" data-mond-fertigen="${esc(u.einheit)}" ${ok?'':'disabled'} title="${esc(ok?'':start[0]==='rot'?start[1]:'Anzahl über dem Maximum')}">Bauen</button></div><p class="bs-satz">${u.einheit==='weltentruemmer'?`Megaschiff zur Mondzerstörung, nur im Werftdock ab Stufe ${fmt((m.werftdock?.einheiten||[]).find(x=>x.einheit==='weltentruemmer')?.werftdock??view.monde?.regeln?.weltentruemmer_werftdock??8)}. `:''}Bezahlt zuerst aus dem Mondlager, den Rest vom Mutterplaneten [${esc(m.koord)}]. <button type="button" class="bs-hilfe" data-bs-techinfo="${esc(u.einheit)}" data-bs-kat="${u.schiff?'ships':'defenses'}" data-bs-reiter="techinfo" aria-label="Techinfo">?</button></p></div></article>`;
 }
 function banner(m){
  const voll=ausgebaut(m);
  return `<div class="bs-banner mond-banner"><span class="kugel-host mond-banner-kugel" id="mond-banner-kugel">${bild(m,'')}<canvas class="planet-kugel" aria-hidden="true"></canvas></span><h2>Mond – ${esc(m.name)} <span class="koord">[${esc(m.koord)}]</span>${window.Planetenname?.mondStift?Planetenname.mondStift(m.koord):''}</h2><p>${esc(typName(m.typ))} · ${fmt(m.durchmesser)} km · Felder ${m.felder.belegt} / ${m.felder.verfuegbar} (Obergrenze ${m.felder.obergrenze})${voll?' · <b class="mond-voll">[Mond ist ausgebaut.]</b>':''}</p></div>`;
 }
 function fakten(m){
  const g=Object.entries(m.bestand).filter(([,v])=>v>0);
  return `<section class="mond-fakten"><div><span>Energie</span><b class="${m.energie.erzeugung<m.energie.verbrauch?'bs-rot':''}">${fmt(m.energie.erzeugung)} / ${fmt(m.energie.verbrauch)}</b><small>${fmt(m.energie.je_solarsonde)} je Solarsatellit · ${fmt(m.verteidigung.solarsonde||0)} Satelliten</small></div><div><span>Unterhalt</span><b class="${m.unterhalt.verzug?'bs-rot':''}">${fmt(m.unterhalt.credits_je_tag)} Credits / Tag</b><small>${m.unterhalt.verzug?'Im Verzug: halbe Wirkung, Tor und Phalanx gesperrt':'bezahlt'}</small></div><div><span>Förderung je Stunde</span><b>${Object.entries(m.foerderung_je_stunde||{}).map(([k,v])=>named(k)+' '+fmt(v)).join(' · ')||'–'}</b><small>Wirkung ${fmt(m.wirkung_promille/10)} %</small></div><div><span>Schiffe auf dem Mond</span><b>${Object.entries(m.schiffe).map(([k,v])=>fmt(v)+'× '+named(k)).join(' · ')||'keine'}</b><small><button type="button" class="bs-link" data-mond-flotte="${esc(m.koord)}">Flotte vom Mond starten</button></small></div></section><details class="mond-lager"><summary>Mondlager · ${g.length} Güter</summary><div class="bs-kosten">${g.map(([k,v])=>`<span class="bs-gut${v>=(m.lager[k]||0)&&m.lager[k]?' knapp':''}" title="${esc(named(k))}: ${fmt(Math.floor(v))} / Lager ${fmt(m.lager[k]||0)}"><img src="${esc(art('resources',k,''))}" alt="${esc(named(k))}"><b>${fmt(Math.floor(v))}</b></span>`).join('')}</div></details>`;
 }
 function faehre(m){
  // Bauteile fahren nicht mit der Fähre (Kern mond_faehre) – nicht anbieten (Funktionsprüfung 25.10.2026).
  const gueter=Object.keys(m.bestand).filter(g=>!['antriebskern','habitatmodul'].includes(g));
  return `<section class="mond-feld" id="mond-feld-faehre"><h3 class="bs-gruppenkopf"><span>Versorgungsfähre</span><small>[höchstens ${fmt(m.faehre_deckel_je_stunde)} je Stunde]</small></h3><p class="bs-leer">Verschiebt stündlich Güter zwischen Mutterplanet [${esc(m.mutterplanet)}] und Mond. Positiv zum Mond, negativ zum Planeten, 0 beendet.</p>${m.faehren.length?`<ul class="mond-liste">${m.faehren.map(f=>`<li>${esc(named(f.gut))}: ${f.menge_je_stunde>0?'+':''}${fmt(f.menge_je_stunde)} je Stunde</li>`).join('')}</ul>`:''}<form class="mond-form" data-mond-faehre="${esc(m.koord)}"><label>Gut<select name="gut">${gueter.map(g=>`<option value="${esc(g)}">${esc(named(g))}</option>`).join('')}</select></label><label>Menge je Stunde<input name="menge" type="number" step="1" value="1000"></label><button type="submit" class="bs-zweit">Fähre einstellen</button></form></section>`;
 }
 // Hyperraumtor-Generator (Schlüssel sprungtor): Bauweg mit Bedingungen und einem Knopf, danach Gegenstelle, Schiffe und
 // „Tor öffnen und durchfliegen“ in EINEM Schritt – das Tor steht nur 300 Spielsekunden offen (bei Tempo 30 zehn
 // Echtsekunden), ein Mensch schafft Öffnen und danach erst Schiffe wählen nicht (Kern: erstdurchreise).
 let torZiel='',torQuelle=0;
 function torQuellen(m){
  const sys=k=>String(k).split(':').slice(0,2).join(':'),hier=sys(m.koord),out=[{art:'mond',koord:m.koord,schiffe:m.schiffe,titel:'Mond '+m.name+' ['+m.koord+']'}];
  for(const p of view.planeten)if(sys(p.koord)===hier)out.push({art:'planet',koord:p.koord,schiffe:p.schiffe,titel:Planetenname.name(p)+' ['+p.koord+']'});
  for(const x of eigene())if(x.koord!==m.koord&&sys(x.koord)===hier)out.push({art:'mond',koord:x.koord,schiffe:x.schiffe,titel:'Mond '+x.name+' ['+x.koord+']'});
  for(const f of view.flotten||[])if(f.zustand==='im_orbit'&&sys(f.ziel)===hier)out.push({art:'flotte',flotte:f.flotte,koord:f.ziel,schiffe:f.schiffe,titel:'Flotte #'+f.flotte+' im Orbit ['+f.ziel+']'});
  return out;
 }
 function torNichtBereit(m,t){
  if(t.bereit)return '';
  if(t.bereit_ab>GameTime.now())return `Lädt nach der letzten Öffnung: bereit in <span data-arrival="${t.bereit_ab}"></span>`;
  if(m.unterhalt.verzug)return 'Unterhalt im Verzug – erst Credits bezahlen, dann ist der Generator wieder bereit.';
  if(m.energie.erzeugung<m.energie.verbrauch)return `Zu wenig Energie: ${fmt(m.energie.erzeugung)} von ${fmt(m.energie.verbrauch)} – Solarsatelliten im Werftdock bauen oder Mondbasis ausbauen.`;
  if(m.blockiert)return 'Mond ist blockiert.';
  return 'Nicht bereit: Energie, Unterhalt oder Blockade prüfen.';
 }
 function sprungtor(m){
  const t=m.sprungtor,b=m.baubar.find(x=>x.gebaeude==='sprungtor'),job=m.bauschleife.find(a=>a.gebaeude==='sprungtor');
  const loecher=(view.monde.hyperraumloecher||[]).filter(l=>(l.von===m.koord||l.nach===m.koord)&&l.schliesst>GameTime.now());
  const wozu='<p class="bs-leer">Der Generator reißt neben diesem Mond ein Hyperraumtor zu einem zweiten Generator auf – auf einem anderen eigenen Mond oder einem freigegebenen eines Allianz- oder Bündnispartners. Flotten reisen ohne Flugzeit hindurch.</p>';
  if(!t.stufe){
   const knopf=job?`<div class="mond-tor-bau">${jobLine(mondJob(m,job))}</div>`:b?`<button type="button" class="bs-bauen" data-mond-bauen="sprungtor" ${b.moeglich&&m.felder.verfuegbar-m.felder.belegt===1&&m.felder.verfuegbar<m.felder.obergrenze?'data-mond-letztes':''} ${b.moeglich&&!gameReadOnly()?'':'disabled'}>Hyperraumtor-Generator bauen</button>`:'';
   const bed=b?(b.fehlt.length?`<ul class="mond-liste">${b.fehlt.map(f=>`<li class="bs-rot">✗ ${esc(fehltText(f))}</li>`).join('')}</ul>`:'<p class="bs-gruen">✓ Alle Bedingungen erfüllt</p>'):'';
   return `<section class="mond-feld" id="mond-feld-sprungtor"><h3 class="bs-gruppenkopf"><span>Hyperraumtor-Generator</span><small>[${job?'im Bau':'nicht gebaut'}]</small></h3>${wozu}${b&&!job?`${bed}<p>Kosten (Mondlager): <span class="bs-kosten">${kosten(b.kosten,m.bestand)}</span> · Bauzeit ${dauer(b.bauzeit_sekunden)}</p>${b.fehlende_rohstoffe.length&&!b.fehlt.length?`<p class="bs-rot">Im Mondlager fehlen: ${esc(b.fehlende_rohstoffe.map(named).join(', '))} – per Transport (Zielart Mond) oder Versorgungsfähre nachliefern.</p>`:''}`:''}${knopf}${loecher.map(lochZeile).join('')}</section>`;
  }
  const status=t.bereit?'<b class="bs-gruen">Bereit</b>':`<b class="${t.bereit_ab>GameTime.now()?'bs-gelb':'bs-rot'}">${torNichtBereit(m,t)}</b>`;
  const freigabe=`<button type="button" class="bs-zweit" data-mond-freigabe="${esc(m.koord)}" data-wert="${!t.freigegeben}">${t.freigegeben?'Für Partner freigegeben – sperren':'Für Allianz/Bündnis freigeben'}</button>`;
  let weg;
  if(!t.bereit)weg='<p class="bs-leer">Sobald der Generator bereit ist, stehen hier Gegenstelle, Schiffe und „Tor öffnen und durchfliegen“.</p>';
  else if(!t.ziele.length){
   const andere=eigene().filter(x=>x.koord!==m.koord);
   weg=`<p class="bs-rot">Noch keine Gegenstelle: Ein Hyperraumtor braucht einen zweiten Generator in Reichweite (${fmt(t.reichweite_systeme)} Systeme).</p>${andere.length?`<p>Auf einem anderen eigenen Mond bauen:</p><div class="mond-wahl">${andere.map(x=>`<button type="button" data-mond-wahl="${esc(x.koord)}">${bild(x,'mond-mini')} ${esc(x.name)} [${esc(x.koord)}]</button>`).join('')}</div>`:'<p class="bs-leer">Du hast keinen zweiten Mond. Monde entstehen nach großen Kämpfen – oder ein Partner gibt dir seinen Generator frei.</p>'}`;
  }else{
   const rw=z=>t.stufe?Math.floor(t.reichweite_systeme/t.stufe)*Math.min(t.stufe,Number(z.stufe)||t.stufe):t.reichweite_systeme;
   const ziel=t.ziele.find(z=>z.koord===torZiel)||t.ziele.find(z=>z.abstand<=rw(z))||t.ziele[0];torZiel=ziel.koord;
   const qs=torQuellen(m);torQuelle=Math.max(0,Math.min(torQuelle,qs.length-1));const q=qs[torQuelle];
   const eigenZiel=eigene().some(x=>x.koord===ziel.koord),deutDa=Math.floor(m.bestand?.deuterium||0),weit=ziel.abstand>rw(ziel),arm=ziel.deuterium!=null&&deutDa<ziel.deuterium;
   const schiffe=Object.entries(q?.schiffe||{}).filter(([k,n])=>n>0&&k!=='weltentruemmer');
   const grund=weit?`Gegenstelle ${fmt(ziel.abstand)} Systeme entfernt, Reichweite ${fmt(rw(ziel))} (schwächeres Tor zählt)`:arm?`Mondlager hat ${fmt(deutDa)} Deuterium, nötig ${fmt(ziel.deuterium)}`:ziel.bereit_ab>GameTime.now()?'Gegenstelle lädt noch':'';
   const aus=grund||gameReadOnly()?'disabled':'';
   weg=`<form class="mond-form mond-tor-form" id="mond-tor-form" data-mond-tor="${esc(m.koord)}" data-css="display:grid;grid-template-columns:minmax(0,1fr);gap:10px;align-items:stretch"><label data-css="display:flex;flex-direction:column;gap:3px;width:100%;font-size:13px">1 · Gegenstelle<select name="tor-ziel" data-css="max-width:none;width:100%;min-height:40px">${t.ziele.map(z=>`<option value="${esc(z.koord)}" ${z.koord===ziel.koord?'selected':''}>${esc(z.name)} [${esc(z.koord)}] · ${fmt(z.abstand)} Systeme${z.deuterium!=null?' · '+fmt(z.deuterium)+' Deuterium':''}</option>`).join('')}</select></label>`
    +`<label data-css="display:flex;flex-direction:column;gap:3px;width:100%;font-size:13px">2 · Schiffe von<select name="tor-quelle" data-css="max-width:none;width:100%;min-height:40px">${qs.map((x,i)=>`<option value="${i}" ${i===torQuelle?'selected':''}>${esc(x.titel)}</option>`).join('')}</select></label>`
    +`<div class="mond-schiffe" data-css="display:grid;grid-template-columns:repeat(auto-fill,minmax(120px,1fr));gap:6px;align-items:end">${schiffe.length?schiffe.map(([k,n])=>`<label>${esc(named(k))} <small>(${fmt(n)})</small><input type="number" min="0" max="${n}" value="0" data-tor-schiff="${esc(k)}" inputmode="numeric"></label>`).join('')+`<button type="button" class="bs-link" data-tor-alle>Alle Schiffe</button>`:'<p class="bs-leer">Hier stehen keine reisefähigen Schiffe – „Nur Tor öffnen“ geht trotzdem.</p>'}</div>`
    +`<label data-css="display:flex;flex-direction:column;gap:3px;width:100%;font-size:13px">3 · Danach<select name="tor-danach" data-css="max-width:none;width:100%;min-height:40px">${eigenZiel?'<option value="landen">Am Zielmond landen</option>':''}<option value="orbit">Im Orbit der Gegenstelle halten</option></select></label>`
    +`<label data-css="display:flex;flex-direction:column;gap:3px;width:100%;font-size:13px">4 · Deuterium mitnehmen (für Weiter- und Rückflug)<input type="number" name="tor-deuterium" min="0" value="0" inputmode="numeric" data-css="min-height:40px"></label>`
    +`${grund?`<p class="bs-rot">${esc(grund)}</p>`:''}<div class="bs-knopfreihe"><button type="submit" class="bs-bauen" data-tor-art="durch" ${aus}>Tor öffnen und durchfliegen</button><button type="submit" class="bs-zweit" data-tor-art="nur" ${aus}>Nur Tor öffnen</button></div><p class="bs-leer">Das Tor steht ${fmt(view.monde?.regeln?.loch_offen_sekunden||300)} Spielsekunden offen (≈ ${dauer(view.monde?.regeln?.loch_offen_sekunden||300)} echt). Jeder, der es sieht, darf hindurch – auch der Feind.</p></form>`;
  }
  return `<section class="mond-feld" id="mond-feld-sprungtor"><h3 class="bs-gruppenkopf"><span>Hyperraumtor-Generator · Stufe ${t.stufe}</span><small>[Reichweite ${t.reichweite_systeme} Systeme · Kapazität ${fmt(t.kapazitaet)} Struktur · Abklingzeit ${t.abklingzeit_minuten} min]</small></h3><p>${status} · ${freigabe}</p>${loecher.map(lochZeile).join('')}${weg}</section>`;
 }
 function lochZeile(l){return `<p class="mond-loch"><b>Hyperraumtor offen</b> [${esc(l.von)}] ↔ [${esc(l.nach)}] · schließt in <b data-arrival="${l.schliesst}"></b> · Kapazität ${fmt(l.kapazitaet_rest)} / ${fmt(l.kapazitaet)} <button type="button" class="bs-zweit" data-loch-reisen="${l.id}">Flotte durchschicken</button></p>`;}
 // ---------- Weltenkrümmer-Einsatz (Karl 19.10.2026: „Funktion, dass mein Weltenkrümmer den Mond vernichten kann …
 // durchdachte schöne UI … erst wenn man einen Weltenkrümmer hat … keine Fakes“). Erscheint nur, wenn auf diesem Mond
 // Weltenkrümmer stehen; Ziele sind die bekannten fremden Monde (view.monde.bekannt, ohne Verbündete). Chance wie im
 // Kern (monde.rs zerstoerung_promille): (100 − √d_eff)·√n %, d_eff = Durchmesser·(1 + zerstoerung_mod des Typs);
 // Verlustrisiko je Weltenkrümmer 0,5·√d_eff %. „Mond vernichten“ belegt den Versandassistenten vor (Mission
 // mond_zerstoeren, Start und Ziel jeweils Mond); entschieden wird beim Start im Kern.
 const TYP_ZERSTOERUNG={basalt_dunkel:0.15,eis_rissig:-0.1}; // regeln/monde.ron zerstoerung_mod (übrige Typen 0)
 function freunde(){const s=new Set(view.allianz?.mitglieder||[]);for(const v of view.vertraege||[])if(/buendnis/.test(v.art||'')&&!/abgelehnt|beendet|angeboten/.test(v.status||''))s.add(v.partner);return s;}
 function systemAbstand(a,b){const x=a.split(':').map(Number),y=b.split(':').map(Number);return x[0]!==y[0]?null:Math.abs(x[1]-y[1]);}
 function zerstoerung(x,n){const d=Math.max(0,(x.durchmesser||0)*(1+(view.monde?.regeln?.zerstoerung_mod?.[x.typ]??TYP_ZERSTOERUNG[x.typ]??0))),w=Math.sqrt(d);return {chance:Math.max(0,Math.min(100,(100-w)*Math.sqrt(n))),verlust:Math.min(100,w/2)};}
 function einsatz(m){
  const n=m.schiffe?.weltentruemmer||0;if(!n)return '';
  const f=freunde(),ziele=(view.monde.bekannt||[]).filter(x=>!x.eigen&&!f.has(x.besitzer)).map(x=>({...x,...zerstoerung(x,n),abstand:systemAbstand(m.koord,x.koord)}))
   .sort((a,b)=>b.chance-a.chance||(a.abstand??999)-(b.abstand??999));
  const wkBild=window.Bauseite?.bildVon?Bauseite.bildVon('ships',{key:'weltentruemmer'}):art('ships','weltentruemmer');
  const zeile=x=>`<article class="me-ziel"><span class="me-mond">${bild(x,'me-mond-bild')}</span><span class="me-text"><strong>${esc(x.name||'Mond')} <small>[${esc(x.koord)}]</small></strong><small>${fmt(x.durchmesser)} km · ${esc(typName(x.typ))} · ${esc(x.besitzer||'?')}${x.abstand!=null?' · '+x.abstand+' System'+(x.abstand===1?'':'e'):' · anderer Sektor'}</small><span class="me-chance"><progress max="100" value="${Math.round(x.chance)}"></progress><b>${fmt(Math.round(x.chance))} % Zerstörung</b><small>Verlustrisiko je Weltenkrümmer ${fmt(Math.round(x.verlust))} %</small></span></span><button type="button" class="me-los" data-mond-vernichten="${esc(x.koord)}">Mond vernichten</button></article>`;
  return `<section class="mond-einsatz" id="mond-einsatz"><header class="me-kopf"><img src="${esc(wkBild)}" alt="Weltenkrümmer"><span><h3>Weltenkrümmer einsatzbereit</h3><small>${fmt(n)} im Werftdock von ${esc(m.name||'')} [${esc(m.koord)}] · Chance steigt mit √Anzahl, sinkt mit √Durchmesser</small></span><b class="me-anzahl">${fmt(n)}×</b></header>${ziele.length?`<div class="me-ziele">${ziele.map(zeile).join('')}</div>`:`<p class="me-leer">Kein fremder Mond bekannt. Monde werden sichtbar, sobald ihr Sonnensystem kartiert ist.</p><button type="button" class="me-aufklaeren" data-mond-aufklaeren>Erst einen fremden Mond aufklären ›</button>`}</section>`;
 }
 function phalanx(m){
  const p=m.tiefenphalanx,berichte=(view.monde.phalanxberichte||[]).filter(b=>b.mond===m.koord).slice(0,5);
  if(!p.stufe)return `<section class="mond-feld"><h3 class="bs-gruppenkopf"><span>Tiefenphalanx</span><small>[nicht gebaut]</small></h3><p class="bs-leer">Scannt Flottenbewegungen eines Planeten in Reichweite. Monde sind nicht scannbar.</p></section>`;
  return `<section class="mond-feld" id="mond-feld-phalanx"><h3 class="bs-gruppenkopf"><span>Tiefenphalanx · Stufe ${p.stufe_wirksam}</span><small>[Reichweite ${p.reichweite_systeme} Systeme im Sektor · ${fmt(p.scan_deuterium)} Deuterium je Scan]</small></h3><form class="mond-form" data-mond-phalanx="${esc(m.koord)}"><label>Zielplanet<input name="ziel" placeholder="${esc(m.koord.split(':').slice(0,2).join(':'))}:4" pattern="[0-9]+:[0-9]+:[0-9]+" required></label><button type="submit" class="bs-zweit" ${gameReadOnly()?'disabled':''}>Scannen</button>${p.bereit_ab>(view.sekunden||0)?`<small>bereit in <b data-arrival="${p.bereit_ab}"></b></small>`:''}</form>${berichte.length?`<ul class="mond-liste">${berichte.map(b=>`<li><details><summary><b>[${esc(b.ziel)}]</b> · ${esc(b.zeit_text||'')} · ${fmt(b.kontakte.length)} Kontakt${b.kontakte.length===1?'':'e'}</summary>${window.Mobile035?.report?Mobile035.report(b,'phalanxberichte'):b.kontakte.map(k=>esc(readable(k))).join(' | ')}</details></li>`).join('')}</ul>`:''}</section>`;
 }
 function schlangen(m){
  const b=m.bauschleife[0],w=m.werftdock.schleife[0],r=m.reparatur[0];
  const kasten=(titel,inhalt,leer)=>`<article class="bs-schlange"><h4>${titel}</h4>${inhalt||`<p class="bs-leer">${leer}</p>`}</article>`;
  return `<section class="bs-schlangen">${kasten('Mondgebäude',b?`<div class="bs-auftrag"><img src="${esc(gebaeudeBild(b.gebaeude))}" alt=""><div><strong>${esc(name(b.gebaeude))}</strong><small>Stufe ${b.stufe}${m.bauschleife.length>1?' · + '+(m.bauschleife.length-1)+' wartend':''}</small>${jobLine(mondJob(m,b))}</div></div>`:'','Keine Mondgebäude im Bau.')}${kasten('Werftdock',w?`<div class="bs-auftrag"><img src="${esc(art('ships',w.einheit))}" alt=""><div><strong>${esc(named(w.einheit))}</strong><small>noch ${fmt(w.rest)} Stück${m.werftdock.naechste>0?` · nächstes in <span data-arrival="${m.werftdock.naechste}"></span>`:''}</small></div></div>`:'','Keine Fertigung im Werftdock.')}${kasten('Reparaturdock',r?`<p class="bs-leer">${Object.entries(r.schiffe).map(([k,v])=>fmt(v)+'× '+named(k)).join(' · ')} · fertig in <span data-arrival="${r.fertig}"></span></p>`:'','Keine Schiffe in Reparatur.')}</section>`;
 }
 function raketenZiel(){const f=$('missile-launch');if(f&&!f.elements.zielart&&view?.monde?.aktiv)f.querySelector('.reference-controls')?.insertAdjacentHTML('beforeend','<label>Zielart<select name="zielart"><option value="planet">Planet</option><option value="mond">Mond</option></select></label>');}
 function render(){
  raketenZiel();
  const host=$('mond-seite');if(!host||$('mond')?.hidden||!view)return;
  const m=mond();
  if(!m){setStableHTML(host,`<div class="bs-banner mond-banner"><h2>Mond</h2><p>${view.monde?.aktiv?'Noch kein eigener Mond. Monde entstehen nach großen Kämpfen an deinen Planeten (1 % je '+fmt(view.monde?.regeln?.truemmer_je_prozent||2000)+' Trümmer, höchstens '+fmt(view.monde?.regeln?.chance_max_prozent||40)+' %).':'In dieser Welt gibt es keine Monde.'}</p></div>`);return;}
  aktiv=m.koord;
  const b=offen&&m.baubar.find(x=>x.gebaeude===offen),u=offenEinheit&&m.werftdock.einheiten.find(x=>x.einheit===offenEinheit);
  const mondwahl=eigene().length>1?`<nav class="mond-wahl" aria-label="Eigene Monde">${eigene().map(x=>`<button type="button" data-mond-wahl="${esc(x.koord)}" aria-pressed="${x.koord===m.koord}">${bild(x,'mond-mini')} ${esc(x.name)} <span class="koord">[${esc(x.koord)}]</span></button>`).join('')}</nav>`:'';
  const geb=`<section class="bs-gruppe"><h3 class="bs-gruppenkopf"><span>Mondgebäude</span><small>${ausgebaut(m)?'[Mond ist ausgebaut.]':`[Felder ${m.felder.belegt} / ${m.felder.verfuegbar}]`}</small></h3><div class="bs-raster">${m.baubar.map(x=>kachelGebaeude(m,x)).join('')}</div></section>`;
  const dock=`<section class="bs-gruppe"><h3 class="bs-gruppenkopf"><span>Werftdock · Stufe ${m.werftdock.stufe}</span><small>[Weltenkrümmer ab Stufe ${fmt((m.werftdock?.einheiten||[]).find(x=>x.einheit==='weltentruemmer')?.werftdock??view.monde?.regeln?.weltentruemmer_werftdock??8)}]</small></h3>${m.werftdock.stufe?[['Schiffe',x=>x.schiff&&x.einheit!=='weltentruemmer','bs-raster quer'],['Megaschiff · Weltenkrümmer',x=>x.einheit==='weltentruemmer','bs-raster'],['Verteidigung & Solarsatelliten',x=>!x.schiff,'bs-raster']].map(([titel,f,cls])=>{const l=m.werftdock.einheiten.filter(x=>(x.baubar||x.einheit==='weltentruemmer')&&f(x));return l.length?`<h4 class="mond-unterkopf">${titel}</h4><div class="${cls}">${l.map(x=>`<button type="button" id="mu-${esc(x.einheit)}" class="bs-kachel${x.schiff&&x.einheit!=='weltentruemmer'?' quer':''}${x.baubar?'':' gesperrt'}${offenEinheit===x.einheit?' gewaehlt':''}" data-mond-einheit="${esc(x.einheit)}" title="${esc(named(x.einheit))}"><span class="bs-bild"><img src="${esc(Bauseite.bildVon?Bauseite.bildVon(x.schiff?'ships':'defenses',{key:x.einheit}):art('ships',x.einheit))}" alt="" loading="lazy"></span><span class="bs-fuss"><span class="bs-name">${silbenHtml(esc(named(x.einheit)))}</span><span class="bs-zahl">${fmt(m.schiffe[x.einheit]||m.verteidigung[x.einheit]||0)}</span></span></button>`).join('')}</div>`:'';}).join(''):'<p class="bs-leer">Werftdock bauen (Mondbasis 2), um auf dem Mond Schiffe, Solarsonden und Weltenkrümmer zu fertigen.</p>'}</section>`;
  setStableHTML(host,`${mondwahl}<div class="bs-kopf">${b||u?`<p class="mond-detailname">☾ ${esc(m.name)} <span class="koord">[${esc(m.koord)}]</span>${window.Planetenname?.mondStift?Planetenname.mondStift(m.koord):''}</p>`:''}${b?detailGebaeude(m,b):u?detailEinheit(m,u):banner(m)}</div>${einsatz(m)}${fakten(m)}${geb}${dock}<div class="mond-felder">${sprungtor(m)}${phalanx(m)}${faehre(m)}</div>${schlangen(m)}`);
  window.PlanetKugel?.einbinden(document.getElementById('mond-banner-kugel'),{nurMond:true,mond:m,ansicht:'planet'});
 }
 function oeffne(koord){if(koord)aktiv=koord;offen=null;offenEinheit=null;navigateGame('mond');window.scrollTo({top:0,behavior:'instant'});render();}

 // Techinfo der Mondgebäude: dieselben vier Reiter wie bei allen Bauseiten.
 let listenbaum=false;
 function techinfo(key,reiter='techinfo'){
  const m=mond(),b=m?.baubar.find(x=>x.gebaeude===key),regel=view.monde.regeln.gebaeude[key];if(!m||!b||!regel)return;
  // Reiter „Techbaum“: der neue Gesamtbaum, auf das Mondgebäude gezoomt (wie an allen Bauseiten, Karl 19.10.)
  if(reiter==='techbaum'&&!listenbaum&&window.Bauseite?.neuerBaum){window.Bauseite.neuerBaum(key,r=>techinfo(key,r)).then(ok=>{if(!ok){listenbaum=true;techinfo(key,'techbaum');listenbaum=false;}});return;}
  let d=$('techinfo');if(!d){document.body.insertAdjacentHTML('beforeend','<dialog id="techinfo" class="techinfo" aria-labelledby="techinfo-titel"></dialog>');d=$('techinfo');}
  const e=window.Bauseite?.text?.(key),stufe=m.gebaeude[key]||0,B=window.Bauseite;
  const l=e?.lore?(typeof e.lore==='string'?e.lore:e.lore[view.volk]||e.lore.allgemein):'',felder=[['zweck','Wozu gebaut',e?.zweck],['staerke','Stärke',e?.staerke],['nische','Nische',e?.nische]].filter(f=>f[2]);
  const req=[{t:'Zivilisationsstufe',ok:view.stufe>=regel.ab_stufe,b:`${view.stufe} / ${regel.ab_stufe}`},...b.fehlt.map(f=>({t:f,ok:false,b:'fehlt'}))];
  const ueber=`${felder.length?`<div class="td-karten">${felder.map(([k,t,v])=>`<article class="td-karte td-${k}"><h4>${t}</h4><p>${esc(v)}</p></article>`).join('')}</div>`:''}<p class="td-lead">${esc(regel.wirkung)}</p>`
   +`<section class="td-block"><h4>Voraussetzungen auf diesem Mond</h4><ul class="td-chips">${req.map(r=>`<li class="${r.ok?'ok':'fehlt'}"><span class="td-chip-text">${esc(r.t)}</span><b>${esc(r.b)}</b></li>`).join('')}</ul><button type="button" class="td-cta" data-mond-ti="techbaum" data-key="${esc(key)}">Ganzen Pfad im Technologiebaum zeigen <span aria-hidden="true">⤢</span></button></section>${B?.logbuch?B.logbuch(l):''}`;
  const rows=[];for(let s=Math.max(1,stufe-5);s<=stufe+5&&(!regel.max_stufe||s<=regel.max_stufe);s++)rows.push(`<tr class="${s===stufe?'aktuell':s===stufe+1?'naechste':''}"><th>${s}</th><td>${stageCosts(Object.fromEntries(Object.entries(regel.kosten).map(([g,v])=>[g,Math.floor(v*regel.faktor**(s-1))])))}</td><td>${fmt(regel.energie*s)}</td></tr>`);
  const technik=`<table class="ti-daten"><caption>Technische Daten</caption><tbody><tr><th>Energie je Stufe</th><td>${regel.energie?fmt(regel.energie):'erzeugt Grundenergie'}</td></tr><tr><th>Höchststufe</th><td>${regel.max_stufe||'ohne Grenze'}</td></tr><tr><th>Kostenfaktor je Stufe</th><td>${fmt(regel.faktor)}</td></tr></tbody></table><table class="ti-stufen"><thead><tr><th>Stufe</th><th>Kosten</th><th>Energie</th></tr></thead><tbody>${rows.join('')}</tbody></table>`;
  const frei=key==='werftdock'?`<p>${view.monde?.regeln?.weltentruemmer_werftdock!=null?`Ab Stufe ${fmt(view.monde.regeln.weltentruemmer_werftdock)}: Weltenkrümmer. `:''}Jede Stufe beschleunigt die Mondfertigung.</p>`:key==='mondbasis'?'<p>Schaltet alle übrigen Mondgebäude frei (+3 Felder je Stufe).</p>':'';
  const tafeln=[['techinfo','Überblick',ueber],['technik','Technik',technik],['anwendungen','Freischaltungen',frei?`<p class="td-lead">${esc(regel.wirkung)}</p>${frei}`:''],['archiv','Archiv',B?.archiv?.(key)||'']];
  if(reiter==='techbaum')tafeln.unshift(['techbaum','Pfad',`<p class="ti-hinweis">Voraussetzungen auf diesem Mond (grün erfüllt, rot fehlt).</p><ul class="ti-baum ti-wurzel"><li class="${b.fehlt.length?'fehlt':'erfuellt'}"><span><img src="${esc(gebaeudeBild(key))}" alt="">${esc(name(key))}</span><b>${b.fehlt.length?esc(b.fehlt.join(' · ')):'Voraussetzungen erfüllt'}</b><ul class="ti-baum"><li class="${view.stufe>=regel.ab_stufe?'erfuellt':'fehlt'}"><span>Zivilisationsstufe</span><b>${view.stufe} / ${regel.ab_stufe}</b></li></ul></li></ul>`]);
  // Dieselbe Infoanzeige wie an allen Bauseiten (bauseite.js anzeige), Mond-Reiter behalten ihre Datenattribute
  if(!B?.anzeige)return;
  B.anzeige(d,{kat:'mond',name:name(key),augenbraue:['Mondgebäude',e?.klasse,e?.rolle].filter(Boolean).join(' · '),status:`Stufe <b>${fmt(stufe)}</b>${regel.max_stufe?` / ${fmt(regel.max_stufe)}`:''}`,sperre:b.fehlt,bild:gebaeudeBild(key),
   werte:[{t:'Ausbau',v:`${fmt(stufe)}${regel.max_stufe?' / '+fmt(regel.max_stufe):''}`,anteil:regel.max_stufe?stufe/regel.max_stufe:null},{t:'Energie',v:regel.energie?fmt(regel.energie*stufe):'Grundenergie',dann:regel.energie&&(!regel.max_stufe||stufe<regel.max_stufe)?fmt(regel.energie*(stufe+1)):null}],
   tafeln,aktiv:reiter,leiste:`Mond-Datenbank · ${m.name||m.koord}`,knopf:id=>`data-mond-ti="${id}" data-key="${esc(key)}"`});
 }

 // Durchreise durch ein sichtbares Hyperraumloch: Herkunft, Schiffe, danach landen oder im Orbit halten.
 function quellen(l){
  const out=[];for(const k of l.mitreisen_moeglich_von||[]){const p=view.planeten.find(x=>x.koord===k),m=eigene().find(x=>x.koord===k);if(p)out.push({art:'planet',koord:k,schiffe:p.schiffe,bestand:p.bestand,titel:'Planet ['+k+']'});if(m)out.push({art:'mond',koord:k,schiffe:m.schiffe,bestand:m.bestand,titel:'Mond '+m.name+' ['+k+']'});}
  for(const f of view.flotten||[])if(f.zustand==='im_orbit'&&[l.von,l.nach].some(k=>k.split(':').slice(0,2).join(':')===String(f.ziel).split(':').slice(0,2).join(':')))out.push({art:'flotte',flotte:f.flotte,koord:f.ziel,schiffe:f.schiffe,ladung:f.ladung,titel:'Flotte #'+f.flotte+' im Orbit ['+f.ziel+']'});
  return out;
 }
 function reisen(id){
  const l=(view.monde?.hyperraumloecher||[]).find(x=>String(x.id)===String(id));if(!l){show('Dieses Hyperraumloch ist nicht mehr sichtbar.',true);return;}
  let d=$('durchreise-dialog');if(!d){document.body.insertAdjacentHTML('beforeend','<dialog id="durchreise-dialog" class="techinfo" aria-labelledby="durchreise-titel"></dialog>');d=$('durchreise-dialog');}
  const q=quellen(l);
  // Treibstoff mitnehmen: Am Austritt gibt es nur, was die Flotte dabeihat (Weiter- und Rückflug).
  const ladungFeld=i=>{const x=q[i];if(!x)return '';if(x.art==='flotte')return `<p class="bs-leer">Die Flotte reist mit ihrer Ladung: ${fmt(x.ladung?.deuterium||0)} Deuterium.</p>`;const vorrat=Math.floor(x.bestand?.deuterium||0);return `<label>Deuterium mitnehmen <small>(Vorrat ${fmt(vorrat)})</small><input type="number" min="0" max="${vorrat}" value="0" name="deuterium" data-reise-deut></label><button type="button" class="bs-link" data-reise-fuellen>Laderaum mit Deuterium füllen</button><p class="bs-leer">Die Durchreise selbst kostet keine Flugzeit, nur die Lochgebühr. Für Weiter- oder Rückflug brauchst du Deuterium an Bord – am Austrittsmond steht kein Vorrat bereit.</p>`;};
  const schiffe=i=>Object.entries(q[i]?.schiffe||{}).filter(([k,n])=>n>0&&k!=='weltentruemmer').map(([k,n])=>`<label>${esc(named(k))} <small>(${fmt(n)})</small><input type="number" min="0" max="${n}" value="0" data-reise-schiff="${esc(k)}"></label>`).join('')||'<p class="bs-leer">Hier stehen keine reisefähigen Schiffe.</p>';
  setStableHTML(d,`<div class="ti-kopf"><h2 id="durchreise-titel">Durchreise · Hyperraumtor ${l.id}</h2><span class="koord">[${esc(l.von)}] ↔ [${esc(l.nach)}]</span><button type="button" class="bs-schliessen" data-reise-zu aria-label="Schließen">✕</button></div><form class="ti-inhalt mond-form" id="durchreise-form" data-loch="${l.id}"><p>Offen noch <b data-arrival="${l.schliesst}"></b> · Restkapazität ${fmt(l.kapazitaet_rest)} Struktur. Die Durchreise kostet kein Deuterium; wer im Orbit bleibt, zahlt den späteren Heimflug gleich mit. Am Austritt kämpft die Torwache gegen alle ohne Bündnis oder Nichtangriffspakt.</p>${q.length?`<label>Herkunft<select name="quelle">${q.map((x,i)=>`<option value="${i}">${esc(x.titel)}</option>`).join('')}</select></label><div class="mond-schiffe" id="durchreise-schiffe">${schiffe(0)}</div><div id="durchreise-ladung">${ladungFeld(0)}</div><label>Danach<select name="danach"><option value="orbit">Im Orbit halten</option><option value="landen">Landen (nur eigener Austrittsmond)</option></select></label><button type="submit" class="bs-bauen" ${gameReadOnly()?'disabled':''}>Durchreisen</button>`:'<p class="bs-leer">Du brauchst einen Planeten, Mond oder eine Orbitflotte im Start- oder Zielsystem.</p>'}</form>`);
  d._quellen=q;d._schiffe=schiffe;d._ladung=ladungFeld;if(!d.open)d.showModal();
 }

 document.addEventListener('click',guard(async e=>{
  const b=e.target.closest('[data-mond],[data-mond-springe],[data-mond-vernichten],[data-mond-aufklaeren],[data-mond-oeffnen],[data-mond-wahl],[data-mond-geb],[data-mond-einheit],[data-mond-zu],[data-mond-max],[data-mond-bauen],[data-mond-fertigen],[data-mond-freigabe],[data-mond-loch],[data-loch-reisen],[data-reise-zu],[data-reise-fuellen],[data-wk-3d],[data-mond-techinfo],[data-mond-ti],[data-mond-flotte]');if(!b)return;
  if(b.dataset.mond!==undefined&&b.matches('[data-mond]')){oeffne(b.dataset.mond);return;}
  if(b.dataset.mondOeffnen){oeffne(b.dataset.mondOeffnen);return;}
  if(b.dataset.mondVernichten){const m=mond();window.Flottenfuehrung?.ausHangar?.({start:m.koord,startart:'mond',schiffe:{weltentruemmer:m.schiffe?.weltentruemmer||0},ziel:b.dataset.mondVernichten,zielart:'mond',mission:'mond_zerstoeren'});return;}
  if(b.hasAttribute('data-mond-aufklaeren')){navigateGame('karte');return;}
  if(b.dataset.mondSpringe){document.getElementById(b.dataset.mondSpringe)?.scrollIntoView({block:'start',behavior:'smooth'});return;}
  if(b.dataset.mondWahl){aktiv=b.dataset.mondWahl;offen=offenEinheit=null;render();return;}
  if(b.dataset.mondGeb){offen=offen===b.dataset.mondGeb?null:b.dataset.mondGeb;offenEinheit=null;render();return;}
  if(b.dataset.mondEinheit){offenEinheit=offenEinheit===b.dataset.mondEinheit?null:b.dataset.mondEinheit;offen=null;menge=1;render();return;}
  if(b.hasAttribute('data-mond-zu')){offen=offenEinheit=null;render();return;}
  if(b.dataset.mondMax!==undefined){menge=Math.max(1,Number(b.dataset.mondMax)||1);render();return;}
  if(b.dataset.mondTechinfo){techinfo(b.dataset.mondTechinfo,b.dataset.reiter||'techinfo');return;}
  if(b.dataset.mondTi){techinfo(b.dataset.key,b.dataset.mondTi);return;}
  if(b.dataset.mondFlotte){pickFleet('','transport');const f=$('fleet');if(f.elements.startart){f.elements.startart.value='mond';f.elements.start.value=b.dataset.mondFlotte;}return;}
  if(b.dataset.wk3d){window.Weltenkruemmer?.oeffnen(b.dataset.wk3d);return;}
  if(b.hasAttribute('data-reise-zu')){$('durchreise-dialog')?.close();return;}
  if(b.hasAttribute('data-reise-fuellen')){const f=$('durchreise-form'),feld=f?.querySelector('[data-reise-deut]');if(!feld)return;
   // Grundladung je Schiff aus dem Regelkatalog; Volks- und Logistikboni rechnet der Kern, daher 90 % als sicherer Vorschlag.
   let kap=0;f.querySelectorAll('[data-reise-schiff]').forEach(i=>{kap+=(Number(i.value)||0)*(rules?.catalog?.einheiten?.[i.dataset.reiseSchiff]?.regel?.ladung||0);});
   feld.value=String(Math.max(0,Math.min(Number(feld.max)||0,Math.floor(kap*.9))));if(!kap)show('Erst Schiffe mit Laderaum auswählen.',true);return;}
  if(b.dataset.lochReisen){reisen(b.dataset.lochReisen);return;}
  const m=mond();if(!m)return;
  if(b.dataset.mondBauen&&b.hasAttribute('data-mond-letztes')&&!confirm(`Das ist das letzte freie Mondfeld. Danach lässt sich auch die Mondbasis nicht mehr ausbauen – der Mond bleibt bei ${m.felder.verfuegbar} Feldern. Erst die Mondbasis bauen (+3 Felder) ist meist besser. Trotzdem ${name(b.dataset.mondBauen)} bauen?`))return;
  if(b.dataset.mondBauen){await command([{typ:'mond_bauen',mond:m.koord,gebaeude:b.dataset.mondBauen}]);return;}
  if(b.dataset.mondFertigen){await command([{typ:'mond_fertigen',mond:m.koord,einheit:b.dataset.mondFertigen,anzahl:Math.max(1,menge)}]);return;}
  if(b.dataset.mondFreigabe){await command([{typ:'sprungtor_freigabe',mond:b.dataset.mondFreigabe,freigegeben:b.dataset.wert==='true'}]);return;}
  if(b.dataset.mondLoch){await command([{typ:'hyperraumloch_oeffnen',mond:b.dataset.mondLoch,gegenstelle:b.dataset.gegenstelle,erstdurchreise:null}]);return;}
 }));
 document.addEventListener('change',e=>{const f=e.target.closest?.('#mond-tor-form');if(!f)return;if(e.target.name==='tor-ziel'){torZiel=e.target.value;render();}if(e.target.name==='tor-quelle'){torQuelle=Number(e.target.value)||0;render();}});
 document.addEventListener('click',e=>{const b=e.target.closest?.('[data-tor-alle]');if(!b)return;b.closest('form')?.querySelectorAll('[data-tor-schiff]').forEach(i=>{i.value=i.max;});});
 document.addEventListener('input',e=>{if(e.target.id==='mond-anzahl'){menge=Math.max(1,Math.floor(Number(e.target.value)||1));render();}});
 document.addEventListener('change',e=>{if(e.target.closest('#durchreise-form')&&e.target.name==='quelle'){const d=$('durchreise-dialog');setStableHTML($('durchreise-schiffe'),d._schiffe(Number(e.target.value)));setStableHTML($('durchreise-ladung'),d._ladung(Number(e.target.value)));}});
 document.addEventListener('submit',guard(async e=>{
  const f=e.target;
  if(f.dataset.mondTor){e.preventDefault();const m=mond();if(!m)return;const art=e.submitter?.dataset.torArt||'durch',ziel=f.elements['tor-ziel'].value;
   let erst=null;
   if(art==='durch'){const q=torQuellen(m)[Number(f.elements['tor-quelle'].value)||0],schiffe={};f.querySelectorAll('[data-tor-schiff]').forEach(i=>{const n=Math.floor(Number(i.value)||0);if(n>0)schiffe[i.dataset.torSchiff]=n;});
    if(!Object.keys(schiffe).length)throw Error('Erst Schiffe wählen – oder „Nur Tor öffnen“.');
    erst={von:q.art==='flotte'?{art:'flotte',flotte:q.flotte}:{art:q.art,koord:q.koord},schiffe,danach:f.elements['tor-danach'].value,ladung:(()=>{const d=Math.floor(Number(f.elements['tor-deuterium']?.value)||0);return d>0&&q.art!=='flotte'?{deuterium:d}:{};})()};}
   await command([{typ:'hyperraumloch_oeffnen',mond:m.koord,gegenstelle:ziel,erstdurchreise:erst}]);return;}
  if(f.dataset.mondFaehre){e.preventDefault();await command([{typ:'mond_faehre',mond:f.dataset.mondFaehre,gut:f.elements.gut.value,menge_je_stunde:Math.trunc(Number(f.elements.menge.value)||0)}]);}
  if(f.dataset.mondPhalanx){e.preventDefault();await command([{typ:'phalanx_scannen',mond:f.dataset.mondPhalanx,ziel:f.elements.ziel.value.trim()}]);}
  if(f.id==='durchreise-form'){e.preventDefault();const d=$('durchreise-dialog'),q=d._quellen[Number(f.elements.quelle.value)],schiffe={};f.querySelectorAll('[data-reise-schiff]').forEach(i=>{const n=Number(i.value)||0;if(n>0)schiffe[i.dataset.reiseSchiff]=n;});if(!Object.keys(schiffe).length)throw Error('Mindestens ein Schiff auswählen.');
   const von=q.art==='flotte'?{art:'flotte',flotte:q.flotte}:{art:q.art,koord:q.koord};const deut=Number(f.elements.deuterium?.value)||0,ladung=q.art!=='flotte'&&deut>0?{deuterium:deut}:{};const r=await command([{typ:'hyperraumloch_durchreisen',loch:Number(f.dataset.loch),von,schiffe,danach:f.elements.danach.value,ladung}]);if(r.ergebnisse.every(x=>x.ok))d.close();}
 }));
 document.addEventListener('game-navigation',()=>render());
 document.addEventListener('DOMContentLoaded',()=>{
  // Flottenformular: Start- und Zielart (Planet oder Mond) für Flüge zu, von und gegen Monde.
  const f=$('fleet');if(!f)return;
  f.elements.ziel.closest('label').insertAdjacentHTML('afterend','<label>Startart <select name="startart"><option value="planet">Planet</option><option value="mond">Mond (gleiche Koordinate)</option></select></label><label>Zielart <select name="zielart"><option value="planet">Planet</option><option value="mond">Mond</option></select></label>');
 });
 return {render,oeffne,bild,reisen,typName,eigene};
})();
// Auch als Fenstereigenschaft, damit andere Module optional darauf zugreifen können.
window.Mondseite=Mondseite;
{
 const ansicht=renderView;renderView=function(){ansicht.apply(this,arguments);Mondseite.render();};
 // Flug- und Raketenbefehle tragen Start-/Zielart „mond“ mit (mond_zerstoeren zielt immer auf den Mond).
 const flug=fleetData;fleetData=function(){const a=flug.apply(this,arguments),f=$('fleet').elements;if(f.zielart?.value==='mond'||a.mission==='mond_zerstoeren')a.zielart='mond';if(f.startart?.value==='mond')a.startart='mond';return a;};
 if(typeof missileDraft==='function'){const rakete=missileDraft;missileDraft=function(){const a=rakete.apply(this,arguments),z=$('missile-launch')?.elements.zielart;if(z?.value==='mond'&&a.flotte==null)a.zielart='mond';return a;};}
}
