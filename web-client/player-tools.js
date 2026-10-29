'use strict';
// Human-facing reference, inbox and overview. All private facts come from the authenticated view.
const PlayerHelp={codex:{kind:'ships',faction:'',key:'',query:'',level:1},quotes:new Map(),report:null};
// Alle Berichtsarten des Postfachs (Server: inbox_ids). Mondberichte liegen in view.monde (Pfad „monde.…“).
const reportGroups=[['systemberichte','Systemscan'],['kampfberichte','Kampfbericht'],['berichte','Planetenspionage'],['spionageberichte','Mondspionage','monde'],['flottenberichte','Flottenaufklärung'],['bodenberichte','Bodenkampf'],['phalanxberichte','Phalanxscan','monde'],['ereignisse','Meldung'],['chronik','Chronik']];
const reportList=(kind,eltern)=>(eltern?view?.[eltern]?.[kind]:view?.[kind])||[];
// Schmale Spalten: Name und Wert nach Silben trennen statt mitten im Wort (window.Silben aus stable-dom.js)
const factRow=(name,value,note='',trennH=s=>globalThis.Silben?globalThis.Silben.html(s):s)=>`<div class="fact-row"><dt>${trennH(esc(name))}</dt><dd>${trennH(esc(value))}${note?`<small>${esc(note)}</small>`:''}</dd></div>`;
function fullRequirements(category,key,p=currentPlanet()){
 const c=rules.catalog,r=category==='buildings'?c.gebaeude[key]:category==='research'?c.forschung[key]:c.einheiten[key].regel;
 const rows=[],add=(title,have,need,unit='Stufe')=>rows.push({title,have,need,ok:have>=need,unit});
 add('Zivilisationsstufe',view.stufe,r.ab_stufe);
 if(category==='research'){
  const home=view.planeten.find(x=>x.heimat)||p;add('Labor auf der Heimatwelt',home.gebaeude.labor||0,r.labor);
  const prereq=c.forschung_voraussetzungen?.[key];
  for(const[k,n]of Object.entries(prereq?.forschung||{}))add(named(k),view.forschung.stufen[k]||0,n);
  for(const[k,n]of Object.entries(prereq?.gebaeude_reich||{}))add(named(k)+' auf einem eigenen Planeten',Math.max(...view.planeten.map(x=>x.gebaeude[k]||0)),n);
  add('Freie Forschungsplätze',c.wirtschaft.forschung_warteschlange-view.forschung.schlange.length,1,'Platz');
 }else{
  if(category==='ships')add('Werft',p.gebaeude.werft||0,Math.max(1,r.werft||0));
  const gates=Object.entries(c.einheiten[key]?.taktik?.voraussetzungen||{}),blueprints=view.entdeckungen?.bauplaene?.[key]||0,planned=Math.max(1,Number(uiSelections[category+'Qty'])||1);if(gates.some(([k,n])=>(view.forschung.stufen[k]||0)<n)&&blueprints>0)add('Gefundene Baupläne statt Forschung',blueprints,planned,'Stück');else for(const[k,n]of gates)add(named(k),view.forschung.stufen[k]||0,n);
  for(const[k,n]of Object.entries(r.braucht||{}))add(named(k),p.gebaeude[k]||0,n);
  if(category==='buildings'&&key==='xenoextraktor'&&!p.industrie)rows.push({title:'Nebelsystem',have:p.nebel?'Vorhanden':'Fehlt',need:'Erforderlich',ok:!!p.nebel,unit:''});
  if(category==='buildings'){
   add('Freie Baufelder',p.felder.gesamt-p.felder.belegt,1,'Feld');
   add('Freie Bauplätze',p.bauschleife_plaetze-p.bauschleife.length,1,'Platz');
  }
  if(['ships','defenses'].includes(category)){
   const qty=Math.max(1,Number(uiSelections[category+'Qty'])||1),crew=r.besatzung||0;
   if(crew)add('Einwohner · 500 müssen bleiben',p.bevoelkerung,(c.wirtschaft.fertigung_min_einwohner||500)+qty*crew,'Einwohner');
   if(c.wirtschaft.fertigung_warteschlange)add('Freie Auftragsplätze',c.wirtschaft.fertigung_warteschlange-(p.fertigung||[]).filter(j=>j.schleife==='werft').length,1,'Platz');
   if(r.max_anzahl)add('Noch zulässige Einheiten',Math.max(0,r.max_anzahl-(p.verteidigung[key]||p.schiffe[key]||0)-(p.fertigung||[]).filter(j=>j.produkt===key).reduce((a,j)=>a+j.rest,0)),qty,'Stück');
  }
 }
 return rows;
}
function requirementSheet(category,key){return `<div class="requirements"><h4>Voraussetzungen · dein Stand</h4><ul>${fullRequirements(category,key).map(x=>`<li class="${x.ok?'met':'unmet'}"><span aria-label="${x.ok?'Erfüllt':'Fehlt'}">${x.ok?'✓':'✕'}</span><span>${esc(x.title)}</span><b>${esc(typeof x.have==='number'?fmt(x.have):x.have)} / ${esc(typeof x.need==='number'?fmt(x.need):x.need)}${x.unit?' '+esc(x.unit):''}</b></li>`).join('')}</ul></div>`;}
function purchaseCosts(category,item,stock){
 const qty=['ships','defenses'].includes(category)?Math.max(1,Number(uiSelections[category+'Qty'])||1):1;
 const costs=Object.fromEntries(Object.entries(item.cost).map(([k,n])=>[k,n*qty]));
 return `<div class="purchase-costs"><h4>Kosten${qty>1?' für '+fmt(qty)+' Stück':''} · vorhanden / benötigt</h4><ul>${Object.entries(costs).filter(([,n])=>n>0).map(([k,n])=>`<li class="${(stock[k]||0)>=n?'met':'unmet'}"><span>${esc(named(k))}</span><b>${fmt(stock[k]||0)} / ${fmt(n)}</b></li>`).join('')}</ul></div>`;
}
function orderDuration(category,item){
 if(!item.duration)return ['ships','defenses'].includes(category)?'Die Werftzeit wird vom Server ermittelt.':'Dauer erst mit erfüllten Voraussetzungen bestimmbar.';
 const qty=Math.max(1,Number(uiSelections[category+'Qty'])||1),weight=view.einheiten_kosten.find(x=>x.einheit===item.key)?.fertigungsplaetze_je_stueck||1,capacity=Math.floor((currentPlanet().fertigungsplaetze?.werft.plaetze||0)/weight);
 return ['ships','defenses'].includes(category)?`Pro Stück: ${GameTime.duration(item.duration)} · Für ${fmt(qty)} Stück: ${capacity?GameTime.duration(item.duration*Math.ceil(qty/capacity)):'zu wenige Fertigungsplätze'} Spielzeit. Bei freier Werft ${capacity} gleichzeitig. Belegte Plätze, Pausen und Versorgung verlängern die Zeit. Ein Ausbau schafft weitere parallele Plätze; die Stückdauer bleibt gleich.`:`Bauzeit: ${GameTime.duration(item.duration)} Spielzeit · ${GameTime.duration(item.duration/(world.tempo||1))} bei ${world.tempo}×`;
}
function stageValue(base,level){const n=Math.max(0,Math.min(rules.catalog.welt.max_gebaeudestufe,Number(level)||0));return Math.floor(base*n*(rules.catalog.wirtschaft.ertragswachstum||1)**n*1000)/1000;}
// Technik v1: Antriebswechsel je Forschungsstufe (Sicht einheit_daten.antriebswechsel; antrieb/verbrauch sind schon die wirksamen Werte).
function antriebsstufeText(s){return `ab ${named(s.forschung)} ${s.stufe}: ${named(s.antrieb)}, Tempo ${fmt(s.grundtempo)}, Verbrauch ${fmt(s.verbrauch)}${s.erreicht?' ✓':''}`;}
function dataSheet(category,item,faction=view.volk,level=item.nextLevel||item.level+1,quote){
 const r=item.r,c=rules.catalog,p=currentPlanet(),rows=[];
 if(category==='ships'||category==='defenses'){
  const own=view.einheiten_kosten?.find(x=>x.einheit===item.key)?.werte;
  const w=quote?.werte||(faction===view.volk?own:null),basis=w?.basis||r;
  for(const[name,k]of [['Angriff','angriff'],['Schild pro Einheit','schild'],['Struktur pro Einheit','struktur'],['Geschwindigkeit','tempo'],['Laderaum','ladung']])rows.push(factRow(name,fmt(w?.[k]??r[k]??0),w?`Basis: ${fmt(basis[k]??r[k]??0)}`:'Grundwert; Technik- und Volkswerte werden geladen'));
  rows.push(factRow('Treibstoffverbrauch',fmt(w?.verbrauch??r.verbrauch??0),w?.antriebswechsel?.some(s=>s.erreicht)?'Nach Antriebswechsel; Streckenkosten in der Flugplanung':'Grundverbrauch je Schiff; Streckenkosten in der Flugplanung'));
  rows.push(factRow('Besatzung',fmt(r.besatzung||0),'Einwohner je gebauter Einheit'));
  const personenplaetze=w?.personenplaetze??c.einheiten[item.key]?.taktik?.personenplaetze??0;
  if(personenplaetze)rows.push(factRow('Zusätzliche Personenplätze',fmt(personenplaetze),'Unabhängig vom Frachtraum und zusätzlich zur Betriebsbesatzung'));
  const cost=c.einheiten[item.key].kosten_je_volk?.[faction]||r.kosten;
  rows.push(factRow('Unterhalt pro Spieltag',w?fmt(w.unterhalt_je_tag):'Serverwert wird geladen', 'Wertäquivalent nach aktivem Regelprofil'));
  const antrieb=w?.antrieb||r.antrieb;
  if(antrieb)rows.push(factRow('Antrieb',named(antrieb),`Deine Technik: Stufe ${view.forschung.stufen[antrieb]||0}`));
  if(w?.antriebswechsel?.length)rows.push(factRow('Antriebswechsel',w.antriebswechsel.map(antriebsstufeText).join(' · '),'Mit dieser Forschungsstufe fliegt das Schiff mit dem neuen Antrieb, Tempo und Verbrauch'));
  if(r.max_anzahl)rows.push(factRow('Höchstbestand pro Planet',fmt(r.max_anzahl)));
  const targetProfile=c.einheiten[item.key].zielprofil;
  // ×N erklärt (Handbuch Kap. 15): nach jedem Treffer auf diese Klasse Chance (N−1)/N auf einen weiteren Schuss (Funktionsprüfung 25.10.2026).
  rows.push(factRow('Schnellfeuer gegen',Object.entries(targetProfile?.schnellfeuer||{}).map(([k,n])=>`${named(k)} ×${n}`).join(' · ')||'Keines',Object.keys(targetProfile?.schnellfeuer||{}).length?'×N: nach jedem Treffer Chance (N−1) von N auf einen weiteren Schuss':''));
  const tactic=c.einheiten[item.key].taktik;
  if(targetProfile){
   const names=keys=>keys.map(named).join(' · ')||'Keine';
   rows.push(factRow('Primärziele',names(targetProfile.primaer)));
   rows.push(factRow('Sekundärziele',names(targetProfile.sekundaer),'Erst wenn kein lebendes, noch nicht anvisiertes Primärziel verfügbar ist'));
   rows.push(factRow('Ausweichziele',names(targetProfile.reserve),'Erst nach Primär- und Sekundärzielen'));
   rows.push(factRow('Nicht erfassbare Ziele',names(targetProfile.nicht_erfassbar)));
   rows.push(factRow('Gleichzeitige Ziele je Einheit und Runde',targetProfile.ziele_je_runde,targetProfile.ziele_je_runde?targetProfile.verteilung:'Unbewaffnet: greift keine Ziele an'));
  }
  if(tactic){rows.push(factRow('Aufgabe und Grenzen',tactic.rolle));
   rows.push(factRow('Trefferfaktoren',Object.entries(targetProfile?.schadensfaktoren||{}).map(([k,n])=>`${named(k)} ×${fmt(n)}`).join(' · ')||'Grundschaden','Alle anderen Ziele: ×1 Grundschaden, kein Zielbonus oder Abschlag'));
   if(tactic.schildfaktor!==1)rows.push(factRow('Schaden auf Schilde bevorzugter Ziele',`×${fmt(tactic.schildfaktor)}`));
   if(tactic.siedler)rows.push(factRow('Koloniestart',`${fmt(tactic.siedler)} Siedler · ${tactic.kolonie_minen} fertige Minen`,`Siedler werden am gewählten Startplaneten eingeschifft; Grundversorgung und mindestens ${fmt(r.ladung)} Rohstoff-Lagerkapazität enthalten`));
   if(tactic.klasse==='schild')rows.push(factRow('Planetenschild', 'Eine aktive Schildstufe pro Planet','Höhere Stufen ersetzen kleinere Schilde nach Fertigstellung'));
  }
  rows.push(factRow('Schnellfeuer durch Gegner',Object.entries(c.einheiten).filter(([,e])=>e.zielprofil?.schnellfeuer?.[item.key]).map(([k,e])=>`${named(k)} ×${e.zielprofil.schnellfeuer[item.key]}`).join(' · ')||'Keines'));
 }else if(category==='buildings'){
  const now=item.level||0,next=Math.max(1,Number(level)||now+1),b=p.baubar.find(x=>x.gebaeude===item.key);
  const extra=next===b?.stufe?b.strom_plus:stageValue(r.energie||0,next)-stageValue(r.energie||0,next-1);
  rows.push(factRow('Zusätzlicher Strombedarf',`+${fmt(extra)}`,`Ausbau auf Stufe ${next} · bei voller Besetzung`));
  rows.push(factRow('Strombedarf dieser Stufe',fmt(stageValue(r.energie||0,next))));
  rows.push(factRow('Aktuelle Energiebilanz',fmt(p.energie.erzeugung-p.energie.verbrauch),extra>p.energie.erzeugung-p.energie.verbrauch?'Energie zuerst erhöhen; zusätzliche Verbraucher können die Versorgung senken':'Puffer für zusätzliche Verbraucher'));
  rows.push(factRow('Zusätzliche Arbeitskräfte',fmt(next===b?.stufe?b.arbeiter_plus:stageValue(r.arbeiter||0,next)-stageValue(r.arbeiter||0,next-1))));
  rows.push(factRow('Zusätzliche Fachkräfte',fmt(next===b?.stufe?b.fachkraefte_plus:stageValue(r.fachkraefte||0,next)-stageValue(r.fachkraefte||0,next-1))));
  if(r.ertrag)rows.push(factRow('Ertrag dieser Stufe',fmt(stageValue(r.ertrag,next)*productionFactor(item.key)),'Grundwert; Planet, Volk, Technik, Besetzung und Versorgung beeinflussen den Ertrag'));
  if(b?.ertrag&&next===b.stufe)rows.push(factRow('Zusätzlicher Ertrag',`${fmt(b.ertrag.plus)} ${b.ertrag.art}`));
  rows.push(factRow('Freie Baufelder',`${Math.max(0,p.felder.gesamt-p.felder.belegt)} / ${p.felder.gesamt}`));
  rows.push(factRow('Kostenfaktor je Stufe',String(r.faktor)));
  const w=c.wirtschaft,z=c.zusatz||{},output={giesserei:'legierung',elektronikwerk:'elektronik',konsumgueterwerk:'konsumgut'}[item.key];
  if(output)rows.push(factRow('Verbrauch je erzeugter Einheit',goods(w.rezepte?.[output]||{}),'Bei Rohstoffmangel fällt die Produktion geringer aus'));
  if(item.key==='fusionskraftwerk')rows.push(`<div class="fact-row"><dt>Deuteriumbedarf pro Spielstunde</dt><dd data-fusion-stage="${esc(next)}">Serverwert wird geladen</dd></div>`);
  if(item.key==='lager')for(const[k,label]of [['lager_roh','Lager je Rohstoff'],['lager_verarbeitet','Lager je verarbeitetem Gut'],['lager_selten','Lager je seltenem Gut']])rows.push(factRow(label,fmt(Math.floor(w[k]*w.lager_faktor**next)),`Kapazität nach Stufe ${next}`));
  if(item.key==='bunker'){rows.push(factRow('Geschützt je Rohstoff',fmt(w.bunker_je_stufe*next)));rows.push(factRow('Geschützt je verarbeitetem / seltenem Gut',fmt(w.bunker_je_stufe*next*w.bunker_verarbeitet_anteil)));}
  if(item.key==='akademie')rows.push(factRow('Gleichzeitige Ausbildungsplätze',fmt(20*next),'Fachkräfte: 24 Spielstunden; Forscher: 48 Spielstunden. Ausbildung beauftragen, Material und Personen erforderlich. Leistung und Versorgung steuern den Fortschritt.'));
  if(item.key==='labor')rows.push(factRow('Forschungsplätze',fmt(stageValue(r.fachkraefte||0,next)),'Werden automatisch mit ausgebildeten Forschern besetzt. Freie Plätze benötigen weitere Ausbildung; volle Labore benötigen einen Ausbau.'));
  if(item.key==='raumhafen')rows.push(factRow('Treibstoffersparnis',fmt(Math.min(c.flug.raumhafen_ersparnis*next,c.flug.raumhafen_ersparnis_max)*100)+' %'));
  if(item.key==='bauhof')rows.push(factRow('Baugeschwindigkeit',`×${next+1}`,'Bei intaktem Gebäude, vor Nanofabrik und Mindestbauzeit'));
  if(item.key==='nanofabrik')rows.push(factRow('Bau- und Werftgeschwindigkeit',`×${2**next}`,'Bei intaktem Gebäude, vor Mindestbauzeit'));
  if(item.key==='raketensilo'){rows.push(factRow('Raketenplätze',fmt(z.silo_plaetze_je_stufe*next)));rows.push(factRow('Raketenreichweite',fmt(z.raketen_reichweite_je_silo*next)+' Systeme (Standard), '+fmt((rules.catalog.raketen?.find(r=>r.key==='hyperrakete')?.reichweite_je_silo||10)*next)+' (Hyperrakete)')); }
  if(item.key==='orbitalring'){rows.push(factRow('Zusätzliche Baufelder',fmt(z.orbitalring_felder)));rows.push(factRow('Zusätzlicher Wohnraum',fmt(z.orbitalring_wohnraum)));}
  if(item.key==='forschungsarchiv')rows.push(factRow('Lokale Forschungsleistung',`+${fmt(z.archiv_fp_bonus*100)} %`));
  if(item.key==='versorgungsnetz')rows.push(factRow('Lokale Energieerzeugung',`+${fmt(z.versorgungsnetz_energie_bonus*100)} %`));
  if(item.key==='verwaltungszentrum')rows.push(factRow('Verwaltete Kolonien',fmt(Math.floor(next/2)),'Je zwei Stufen eine Kolonie ohne den zusätzlichen Verwaltungsmalus'));
 }else{
  rows.push(factRow('Forschungsort dieser Bestellung',(view.planeten.find(x=>x.heimat)||p).koord,'Laborvoraussetzung und Rohstoffzahlung beziehen sich hier auf die Heimatwelt'));
  rows.push(factRow('Forschungspunkte dieser Stufe',fmt((r.fp||0)*r.faktor**Math.max(0,level-1))));
  rows.push(factRow('Deine Forschungsleistung',`${fmt(view.forschung.punkte_je_stunde)} FP / Spielstunde`));
  rows.push(factRow('Kostenfaktor je Stufe',String(r.faktor)));
  rows.push(factRow('Wirkung',r.wirkung));
  const unlocks=[...Object.entries(c.einheiten).filter(([,e])=>e.taktik?.voraussetzungen?.[item.key]).map(([k,e])=>`${named(k)} ab Stufe ${e.taktik.voraussetzungen[item.key]} · Werft ${e.regel.werft} · Zeitalter ${e.regel.ab_stufe}`),...(c.raketen||[]).filter(m=>m.braucht[item.key]).map(m=>`${m.name} ab Stufe ${m.braucht[item.key]} · Silo ${m.silo} · Zeitalter ${m.zivilisation}`),...Object.entries(c.forschung_voraussetzungen||{}).filter(([,v])=>v.forschung?.[item.key]).map(([k,v])=>`${named(k)} ab Stufe ${v.forschung[item.key]}`)];
  rows.push(factRow('Ermöglicht',unlocks.join(' · ')||'Keine zusätzlichen Einheitentypen; verbessert die beschriebene Fähigkeit.','Weitere Gebäude-, Technik- und Zeitaltervoraussetzungen gelten zusätzlich.'));

 }
 return `<details class="technical-sheet" open><summary>Vollständiges Datenblatt</summary>${category==='ships'||category==='defenses'?'<p class="data-sheet-note">Volk und deine Technik werden in den wirksamen Serverwerten berücksichtigt; Grundwerte stehen zum Vergleich daneben.</p>':''}<dl>${rows.join('')}</dl></details>`;
}
function loadFusionValues(container){
 const identity=JSON.stringify([world.world_id,view.name,currentPlanet().koord]);
 for(const node of container.querySelectorAll('[data-fusion-stage]')){
  const level=Number(node.dataset.fusionStage),planet=currentPlanet().koord;
  const stillCurrent=()=>node.isConnected&&world&&view&&currentPlanet()&&identity===JSON.stringify([world.world_id,view.name,currentPlanet().koord]);
  api('/api/tool',{typ:'kosten',gebaeude:'fusionskraftwerk',stufe:level,planet}).then(q=>{
   if(!stillCurrent())return;
   setStableHTML(node, `${esc(fmt(q.deuteriumbedarf_je_stunde_vollbesetzt))}<small>Bei voller Besetzung auf Stufe ${esc(level)}. Besetzung und verfügbarer Deuteriumbestand begrenzen den Verbrauch. Energietechnik erhöht die Energieausbeute, nicht diesen Grundbedarf.</small>`);
  }).catch(e=>{if(stillCurrent())node.textContent=e.message;});
 }
}
function reportRows(){
 const seen=new Set();return reportGroups.flatMap(([kind,title,eltern])=>reportList(kind,eltern).map(row=>({kind,title:kind==='berichte'&&row.art==='erkundung'?'Erkundungsbericht':title,row,id:row._postfach_id})))
  .filter(x=>x.id&&!seen.has(x.id)&&seen.add(x.id)).sort((a,b)=>reportTime(b.row)-reportTime(a.row));
}
function reportTime(r){if(typeof r.zeit==='number')return r.zeit;if(r.sekunden!=null)return r.sekunden;const m=String(r.zeit||'').match(/Tag\s*(\d+)\D+(\d\d):(\d\d)(?::(\d\d))?/);return m?(+m[1]-1)*86400+(+m[2])*3600+(+m[3])*60+(+m[4]||0):0;}
function reportWhen(r){return typeof r.zeit==='number'?`Tag ${Math.floor(r.zeit/86400)+1} · ${GameTime.duration(r.zeit%86400)}`:r.zeit||'';}
function reportPresentation(x){
 const type=String(x.row.art||x.kind);
 if(x.kind==='systemberichte')return {section:'Aufklärung',title:'Systemscan',art:'missions.spionage.webp',icon:'search',go:'karte',action:'Galaxie öffnen'};
 if(x.kind==='berichte'&&x.row.art==='erkundung')return {section:'Aufklärung',title:'Erkundungsbericht',art:'missions.spionage.webp',icon:'search',go:'kolonien',action:'Kolonisation öffnen'};
 if(x.kind==='bodenberichte')return {section:'Flottenkommando',title:'Bodenkampf',art:'missions.angriff.webp',icon:'shield',go:'truppen',action:'Truppen öffnen'};
 if(x.kind==='phalanxberichte'||x.kind==='spionageberichte')return {section:'Aufklärung',title:x.title,art:'missions.spionage.webp',icon:'search',go:'karte',action:'Galaxie öffnen'};
 if(/erkundung|aufklaerung|spionage/.test(type))return {section:'Aufklärung',title:x.title==='Meldung'?'Meldung der Aufklärung':x.title,art:'missions.spionage.webp',icon:'search',go:'karte',action:'Galaxie öffnen'};
 if(/lieferung|recycling|abbau|unterhalt/.test(type))return {section:'Flottenkommando',title:x.title==='Meldung'?'Meldung der Flotte':x.title,art:'backgrounds.hangar.webp',icon:'signal',go:'flotten',action:'Flotten & Einsätze öffnen'};
 if(/allianz/.test(type))return {section:'Diplomatie',title:'Diplomatisches Signal',art:'backgrounds.diplomacy.webp',icon:'shield',go:'allianzbereich',action:'Allianz & Einladungen ansehen'};
 if(/diplomatie|vertrag/.test(type))return {section:'Diplomatie',title:'Nachricht der Diplomaten',art:'backgrounds.diplomacy.webp',icon:'shield',go:'briefkasten',folder:'anfragen',action:'Diplomatische Anfragen & Verträge'};
 if(/forschung/.test(type))return {section:'Wissenschaft',title:'Bericht aus den Laboren',art:'backgrounds.research.webp',icon:'flask',go:'forschen',action:'Forschung verwalten'};
 if(/kampf|bombard|invasion|besetzung/.test(type))return {section:'Flottenkommando',title:x.title==='Meldung'?'Lagebericht vom Einsatz':x.title,art:'missions.angriff.webp',icon:'shield',go:'flotten',action:'Flotten & Einsätze öffnen'};
 if(/markt|handel/.test(type))return {section:'Handelskontor',title:'Handelsbericht',art:'backgrounds.market.webp',icon:'signal',go:'aktionen',action:'Markt & Lieferungen öffnen'};
 if(/bau|fertigung|abriss|reparatur/.test(type))return {section:'Planetare Verwaltung',title:'Bericht aus der Produktion',art:'backgrounds.shipyard.webp',icon:'signal',go:'auftraege',action:'Bauaufträge öffnen'};
 if(/flotte|expedition|bergung|rettung/.test(type))return {section:'Flottenkommando',title:x.title==='Meldung'?'Signal aus dem All':x.title,art:'backgrounds.hangar.webp',icon:'signal',go:'flotten',action:'Flotten & Einsätze öffnen'};
 if(x.kind==='berichte')return {section:'Aufklärung',title:x.title,art:'missions.spionage.webp',icon:'search',go:'karte',action:'Galaxie öffnen'};
 // Systemkartierung und Erkundung: zur Galaxie statt zur Imperiumsübersicht (dort geht es mit Planetensonden weiter).
 if(/systemaufklaerung|erkund/.test(type))return {section:'Aufklärung',title:x.title==='Meldung'?'Bericht der Aufklärung':x.title,art:'missions.spionage.webp',icon:'search',go:'karte',action:'Galaxie öffnen'};
 return {section:'Imperium',title:x.title==='Meldung'?'Neues aus deinem Reich':x.title,art:'backgrounds.command.webp',icon:'signal',go:'reich',action:'Imperiumsübersicht öffnen'};
}
function reportTitle(x){return `${reportPresentation(x).title}${x.row.ziel?' · '+(window.Planetenname?Planetenname.text(x.row.ziel):x.row.ziel):x.row.ort?' · '+x.row.ort:x.row.planet?' · '+x.row.planet:x.row.flotte?' · Kontakt '+x.row.flotte:''}`;}
function renderReportReader(x){
 const meta=reportPresentation(x),dialog=$('report-reader');
 dialog.style.setProperty('--report-art',`url('assets/${meta.art}')`);
 $('report-reader-title').textContent=reportTitle(x);
 setStableHTML($('report-reader-context'), `<span class="report-emblem">${uiIcon(meta.icon)}</span><div><span>${esc(meta.section)}</span><time>${esc(reportWhen(x.row))}</time></div>`);
 const fields=Object.entries(x.row).filter(([k])=>!k.startsWith('_')&&!['zeit','sekunden','art','text'].includes(k));
 const value=(k,v)=>typeof v==='boolean'?(v?'Ja':'Nein'):typeof v==='object'&&v?readable(v):v??'Nicht aufgeklärt';
 setStableHTML($('report-reader-body'), x.kind==='systemberichte'&&window.Berichtsarchiv?Berichtsarchiv.systembericht(x.row):window.Mobile035.report(x.row,x.kind));
 setStableHTML($('report-reader-actions'), `<button type="button" class="ui-action" data-report-go="${meta.go}" ${meta.folder?`data-report-folder="${meta.folder}"`:''}><span>${meta.action}</span>${uiIcon('arrow')}</button>${['berichte','flottenberichte','spionageberichte'].includes(x.kind)&&x.row.art!=='erkundung'?`<button type="button" class="ui-action secondary" data-sim-report><span>In Kampfsimulator übernehmen</span>${uiIcon('arrow')}</button>`:''}`);
}
function reportPreview(x){
 if(x.kind==='systemberichte')return [x.row.nebel?'Nebelsystem':'Kein Nebel',x.row.asteroidenguertel?'Asteroidengürtel':'Kein Gürtel',`${(x.row.plaetze||[]).filter(p=>p.spieler).length} bekannte besiedelte Plätze`].join(' · ');
 if(x.kind==='berichte'&&x.row.art==='erkundung')return `Freier Platz · ${fmt(x.row.felder)} Felder · Zone ${named(x.row.zone||'')}${(x.row.orbit||[]).length?` · ${fmt(x.row.orbit.length)} Flotten im Orbit`:''}`;
 if(x.kind==='bodenberichte'){const v=s=>(x.row[s+'_verluste']||[]).reduce((n,a)=>n+(Number(a)||0),0);return `Bodenrunde · Verluste Angreifer ${fmt(v('angreifer'))}, Verteidiger ${fmt(v('verteidiger'))}`;}
 if(x.kind==='phalanxberichte')return `${fmt((x.row.kontakte||[]).length)} Flottenbewegungen am Ziel`;
 if(x.kind==='kampfberichte'&&x.row.sieger)return `Ergebnis: ${named(x.row.sieger)}${x.row.ziel_mond?' · am Mond':''}${x.row.mondwurf?.entstanden?' · Mond entstanden':''}`;
 return (x.row.text&&typeof kernLesbar==='function'?kernLesbar(x.row.text):x.row.text)||x.row.sieger&&'Ergebnis: '+named(x.row.sieger)||x.row.besitzer&&'Beobachteter Besitzer: '+x.row.besitzer||'Beobachtung und Details öffnen';}
function renderReportInbox(){
 if(!view||!$('mail-reports-pane'))return;
 const read=new Set(view.postfach_gelesen||[]),query=$('report-search').value.toLocaleLowerCase('de'),filter=$('report-filter').value;
 const rows=reportRows().filter(x=>(!filter||x.kind===filter)&&(!query||`${reportTitle(x)} ${reportPreview(x)}`.toLocaleLowerCase('de').includes(query)));
 setStableHTML($('mail-reports-list'), window.Berichtsarchiv?Berichtsarchiv.inbox(rows):rows.map(x=>`<button type="button" class="report-entry ${read.has(x.id)?'':'unread'}" data-open-report="${esc(x.id)}"><span><strong>${esc(reportTitle(x))}</strong><small>${esc(reportPreview(x))}</small></span><time>${esc(reportWhen(x.row))}${read.has(x.id)?'':' · Neu'}</time></button>`).join('')||'<p class="empty">Keine passenden Berichte.</p>');
 const mails=(communicationData().briefkasten||[]).filter(n=>n.kanal==='privat'&&!n.gesendet&&!n.gelesen).length,unread=reportRows().filter(x=>!read.has(x.id)).length;
 $('inbox-count').textContent=mails+unread||'';$('inbox-shortcut').setAttribute('aria-label',`Briefkasten · ${mails} ungelesene Briefe und ${unread} ungelesene Berichte`);
 $('mail-report-count').textContent=unread||'';
 window.Gegnerakte?.neueBerichte(reportRows(),read);
}
function knownMovements(){
 const own=(view.flotten||[]).map(f=>({...f,kind:'eigen',time:f.zustand==='rueckflug'?f.rueckkehr_sekunden:f.zustand==='im_orbit'?(f.orbit_ende_sekunden||null):f.ankunft_sekunden,who:view.name,detail:f}));
 const allies=(view.verbuendete_anfluege||[]).map(f=>({...f,kind:'verbuendet',time:f.ankunft_sekunden,who:f.von,detail:f}));
 const enemies=(view.angriffe||[]).map(f=>({...f,kind:'feindlich',time:f.ankunft_sekunden,who:f.von||'Unbekannter Absender',detail:f}));
 return [...own,...allies,...enemies].sort((a,b)=>(a.time??Infinity)-(b.time??Infinity)).slice(0,5);
}
function renderOverviewHelp(){
 if(!view||!$('overview-intel'))return;
 const reports=reportRows().filter(x=>x.kind!=='chronik').map(x=>({...x,time:reportTime(x.row)})),letters=(communicationData().briefkasten||[]).filter(n=>n.kanal==='privat'&&!n.gesendet).map(n=>({letter:n,time:reportTime(n)})),messages=[...reports,...letters].sort((a,b)=>b.time-a.time).slice(0,3),fleets=knownMovements();
 const opened=new Set([...$('overview-intel').querySelectorAll('details[open]')].map(e=>e.dataset.movement));
 setStableHTML($('overview-intel'), `<div class="overview-feed"><div class="overview-heading"><h3>Neueste Meldungen</h3><button type="button" class="secondary" data-go="briefkasten">Post & Berichte ↗</button></div>${messages.map(x=>x.letter?`<button type="button" class="overview-message" data-open-letter="${esc(x.letter.id)}"><span>✉ ${esc(x.letter.von)}</span><strong>${esc(x.letter.betreff||'Nachricht')}</strong></button>`:`<button type="button" class="overview-message" data-open-report="${esc(x.id)}"><time>${esc(reportWhen(x.row))}</time><span>${esc(typeof kernLesbar==='function'?kernLesbar(x.row.text||reportTitle(x)):x.row.text||reportTitle(x))}</span></button>`).join('')}${!messages.length?'<p class="overview-empty">Noch keine Meldungen oder persönliche Post.</p>':''}</div><div class="overview-movements"><div class="overview-heading"><h3>Nächste 5 Flotten</h3><small>Eigene Bewegungen & bekannte Anflüge</small></div>${fleets.map(f=>{const key=f.kind+':'+f.flotte;return `<details class="movement ${f.kind}" data-movement="${key}" ${opened.has(key)?'open':''}><summary><span class="movement-mark">${f.kind==='eigen'?'↑':f.kind==='verbuendet'?'↗':'!'}</span><span><b>${esc(f.kind==='eigen'?'Eigene Flotte':f.kind==='verbuendet'?'Verbündete Flotte':'Feindlicher Anflug')} · ${esc(f.ziel)}</b><small>${esc(f.who)}</small></span><time>${f.time!=null?`<span data-arrival="${f.time}"></span>`:'Im Orbit'}</time></summary><div class="movement-detail"><p>${esc(named(f.mission))}${f.start?' · '+esc(f.start)+' → '+esc(f.ziel):''}</p><p>Schiffe: ${esc(typeof f.schiffe==='object'&&f.schiffe?goods(f.schiffe):f.schiffe??(f.schiffe_spanne?.join('–')||'Unbekannt'))}</p>${f.schiffstypen?`<p>${esc(goods(f.schiffstypen))}</p>`:''}${f.kind==='feindlich'?`<p>${f.sondenbericht?'Vorhandener Sondenbericht; historische Beobachtung.':'Zusammensetzung erfordert Flottenaufklärung.'}</p><button type="button" data-probe="${f.flotte}">Flottensonde senden</button>`:f.kind==='eigen'?'<button type="button" class="secondary" data-go="flotten">Flotte verwalten</button>':''}</div></details>`;}).join('')||'<p class="overview-empty">Keine bekannte Flottenbewegung. Unentdeckte Anflüge bleiben verborgen.</p>'}</div>`);
 tickGame();
}
function codexArt(kind,key,faction){const img=window.STERNEN_ART?.images?.[`${kind}|${key}|${faction}`]||window.STERNEN_ART?.images?.[`${kind}|${key}|`];return img?`<img src="${esc(img.url)}" alt="${esc(named(key)+' · '+named(faction))}">`:`<div class="art-shell art-pending" role="img" aria-label="${esc(named(key))} · Motiv wird ergänzt"><span>✧<small>Volksmotiv wird ergänzt</small></span></div>`;}
function missileReference(r,faction){
 const rows=[['Basisschaden',r.schaden||'Abfangschutz'],['Maximale Ziele je Rakete',r.zielprofil.ziele_je_rakete],['Lagerplätze / Stück',r.plaetze],['Fertigungsplätze / Stück',r.fertigungsplaetze],['Grundbauzeit / Stück',GameTime.duration(r.bauzeit_sekunden)],['Raketensilo',r.silo],['Zivilisationsstufe',r.zivilisation],['Forschung',Object.entries(r.braucht).map(([k,n])=>`${named(k)} ${n}`).join(' · ')||'Keine zusätzliche Forschung'],['Einsatz',r.schaden?[r.planet?'Planetare Verteidigung':'',r.flotte?'Erkannte feindliche Anflüge':''].filter(Boolean).join(' · '):'Automatischer Schutz des eigenen Planeten']];
 if(r.schaden)rows.push(['Reichweite',`${r.reichweite_je_silo} Systeme je Silostufe · gleicher Sektor`],['Flugzeit',`${r.flug_min_sekunden} s + ${r.flug_je_system_sekunden} s je Systemabstand`]);
 return `<p>${esc(r.besonderheit)}</p><dl class="dossier-metrics">${rows.map(([k,v])=>factRow(k,typeof v==='number'?fmt(v):v)).join('')}</dl><h4>Grundkosten pro Rakete</h4>${costHtml(r.kosten)}<p class="counter-hint">Kosten und Bauzeit bei 100 % Siloleistung. Jede Silostufe stellt einen Fertigungsplatz bereit; Lagerplätze sind davon getrennt. Überlast erhöht den Materialbedarf. Energie- oder Personalmangel pausiert die Fertigung.</p>${missileDossier(r,faction)}${r.schaden?`<p>Schilde werden umgangen. Waffentechnik und Volksbonus erhöhen den Startschaden. Struktur, Panzertechnik und Volksboni des Ziels bestimmen die Widerstandskraft. Keine Beute und kein Wiederaufbau. ${fmt(rules.catalog.kampf.truemmer_anteil*100)} % der Erz- und Kristallbaukosten vollständig zerstörter Einheiten bilden Trümmer am Ziel.</p>`:''}`;
}
function codexSupplement(kind){
 const c=rules.catalog;
 if(kind==='ground')return [...c.bodentruppen.typen.map(t=>({key:t.id,name:t.name,text:t.rolle,era:t.ab_stufe,level:t.kaserne,image:art('ground_units',t.bild,PlayerHelp.codex.faction)})),{key:'miliz',name:'Miliz',text:'Bevölkerung während einer Invasion mobilisieren',era:0,level:0,image:art('ground_units','miliz',PlayerHelp.codex.faction)}];
 if(kind==='missiles')return c.raketen.map(r=>({key:r.key,name:r.name,text:r.besonderheit,era:r.zivilisation,level:r.silo,image:art('missiles',r.key,PlayerHelp.codex.faction)}));
 if(kind==='rules')return (c.regelabschnitte||[]).map(r=>({key:r.id,name:r.name,text:r.text}));
 return null;
}
function codexSupplementView(entries,controls){
 const s=PlayerHelp.codex,c=rules.catalog;
 entries=entries.filter(e=>`${e.name} ${e.text}`.toLocaleLowerCase('de').includes(s.query.toLocaleLowerCase('de'))).sort((a,b)=>(a.era??0)-(b.era??0)||(a.level??0)-(b.level??0));
 if(!entries.some(e=>e.key===s.key))s.key=entries[0]?.key||'';
 const entry=entries.find(e=>e.key===s.key);
 const content=!entry?'':s.kind==='ground'?GroundUI.reference(s.key,s.faction):s.kind==='missiles'?missileReference(c.raketen.find(r=>r.key===s.key),s.faction):`<p class="codex-rule-text">${esc(entry.text)}</p>`;
 setStableHTML($('codex-content'),`${controls}<div class="codex-reference-layout"><nav class="codex-reference-list" aria-label="Einträge">${entries.map(e=>`<button type="button" data-codex-key="${esc(e.key)}" aria-pressed="${e.key===s.key}">${e.image?`<img src="${esc(e.image)}" alt="" loading="lazy">`:''}<span>${esc(e.name)}${e.era?`<small>Zeitalter ${['','I','II','III','IV','V'][e.era]}</small>`:''}</span></button>`).join('')}</nav>${entry?`<article class="codex-detail codex-reference" data-codex-entry="${esc(s.key)}" data-css="--codex-scene:url(&quot;${esc(art('buildings',s.kind==='ground'?'kaserne':'raketensilo',s.faction))}&quot;)"><h3>${esc(entry.name)}</h3>${entry.image?`<img class="dossier-hero" src="${esc(entry.image)}" alt="${esc(entry.name+' · '+named(s.faction))}">`:''}${content}</article>`:'<p>Keine passenden Einträge.</p>'}</div>`);
}
function renderCodex(){
 if(!rules?.catalog||!view||!$('codex-content'))return;
 const s=PlayerHelp.codex,c=rules.catalog;s.faction=s.faction||view.volk;
 const categories=[['ships','Schiffe'],['defenses','Verteidigung'],['buildings','Gebäude'],['research','Forschung'],['ground','Bodentruppen'],['missiles','Raketen'],['rules','Spielregeln']];
 const controls=`<div class="reference-controls"><nav aria-label="Codex-Themen">${categories.map(([k,t])=>`<button type="button" class="secondary" data-codex-kind="${k}" aria-pressed="${s.kind===k}">${t}</button>`).join('')}</nav>${s.kind!=='rules'?`<label>Volk<select id="codex-faction">${Object.keys(c.voelker).map(k=>`<option value="${k}" ${s.faction===k?'selected':''}>${esc(named(k))}</option>`).join('')}</select></label>`:''}<label>Nachschlagen<input id="codex-search" type="search" value="${esc(s.query)}" placeholder="Name oder Wirkung"></label></div>`;
 const supplement=codexSupplement(s.kind);if(supplement){codexSupplementView(supplement,controls);return;}
 const catalog=s.kind==='buildings'?c.gebaeude:s.kind==='research'?c.forschung:Object.fromEntries(Object.entries(c.einheiten).filter(([,e])=>e.schiff===(s.kind==='ships')).map(([k,e])=>[k,e.regel]));
 const entries=Object.entries(catalog).filter(([k,r])=>`${named(k)} ${r.wirkung}`.toLocaleLowerCase('de').includes(s.query.toLocaleLowerCase('de')));
 if(!entries.some(([k])=>k===s.key))s.key=entries[0]?.[0]||'';
 const r=catalog[s.key],p=currentPlanet(),level=s.kind==='research'?view.forschung.stufen[s.key]||0:p.gebaeude[s.key]||0;
 const cost=r?(s.kind==='ships'||s.kind==='defenses'?c.einheiten[s.key].kosten_je_volk[s.faction]:(s.kind==='buildings'&&buildingStageCosts(s.key,s.level))||Object.fromEntries(Object.entries(r.kosten).map(([k,n])=>[k,Math.floor(n*r.faktor**Math.max(0,s.level-1))]))):{};
 setStableHTML($('codex-content'), `<div class="reference-controls"><nav>${categories.map(([k,t])=>`<button type="button" class="secondary" data-codex-kind="${k}" aria-pressed="${s.kind===k}">${t}</button>`).join('')}</nav><label>Volk<select id="codex-faction">${Object.keys(c.voelker).map(k=>`<option value="${k}" ${s.faction===k?'selected':''}>${esc(named(k))}</option>`).join('')}</select></label><label>Nachschlagen<input id="codex-search" type="search" value="${esc(s.query)}" placeholder="Name oder Wirkung"></label>${['buildings','research'].includes(s.kind)?`<label>Stufe<input id="codex-level" type="number" min="1" max="${s.kind==='research'?c.welt.max_forschungsstufe:c.welt.max_gebaeudestufe}" value="${s.level}"></label>`:''}</div>${r?`<article class="codex-detail">${codexArt(s.kind,s.key,s.faction)}<div><h3>${esc(named(s.key))}</h3><p>${esc(r.wirkung)}</p><p class="muted">${esc(named(s.faction))} · Referenz für alle Völker · Werte bei deinen Technikstufen</p>${costHtml(cost)}${requirementSheet(s.kind,s.key)}<div id="codex-data-sheet">${dataSheet(s.kind,{key:s.key,r,level},s.faction,s.level)}</div></div></article>`:'<p>Keine passenden Einträge.</p>'}<div class="gallery codex-gallery">${entries.map(([k])=>`<button type="button" class="item-tile" data-codex-key="${esc(k)}" aria-pressed="${s.key===k}">${artShell(s.kind,k,null,s.faction)}<span class="tile-copy"><strong>${esc(named(k))}</strong></span></button>`).join('')}</div>`);
 if(r&&['ships','defenses'].includes(s.kind)){
  const key=s.key,fac=s.faction,identity=world.world_id+':'+view.name;api('/api/tool',{typ:'kosten',einheit:key,volk:fac,planet:p.koord}).then(q=>{if(identity===world.world_id+':'+view.name&&PlayerHelp.codex.key===key&&PlayerHelp.codex.faction===fac)setStableHTML($('codex-data-sheet'), dataSheet(s.kind,{key,r,level},fac,s.level,q));}).catch(e=>show(e.message,true));
 }
 loadFusionValues($('codex-content'));
}
function setupWarSim(){
 if(!rules?.catalog||$('war-form'))return;
 const c=rules.catalog,options=Object.keys(c.voelker).map(k=>`<option value="${k}">${esc(named(k))}</option>`).join('');
 const side=(name,title)=>`<fieldset><legend>${title}</legend><div class="input-grid"><label>Volk<select name="${name}_volk"><option value="" disabled selected>Volk wählen</option>${options}</select></label>${['waffentechnik','schildtechnik','panzerung'].map(k=>`<label>${esc(named(k))}<input name="${name}_${k}" type="number" min="0" max="30" value="${name==='angreifer'?Number(view.forschung?.stufen?.[k])||0:''}" placeholder="Stufe eingeben"></label>`).join('')}</div><details open><summary>Einheiten wählen</summary><div class="sim-units">${Object.entries(c.einheiten).filter(([,e])=>name==='verteidiger'||e.schiff).map(([k])=>`<label>${esc(named(k))}<input type="number" min="0" value="0" data-sim-side="${name}" data-sim-unit="${k}" aria-label="${title}: ${esc(named(k))}"></label>`).join('')}</div></details></fieldset>`;
 setStableHTML($('warsim-content'), `<p>Vergleiche Völker, Flotten, Verteidigung und Waffen-, Schild- und Panzerungstechnik nach den Kampfregeln. Fremde Daten trägst du selbst ein oder übernimmst deine eigenen Spionageberichte.</p><form id="war-form"><div class="sim-sides">${side('angreifer','Deine Angriffsflotte')}${side('verteidiger','Gegner & Verteidigung')}</div><div class="section-actions"><button type="button" class="secondary" id="sim-own">Schiffe dieses Planeten & Technik übernehmen</button><button type="submit">Kampf simulieren</button></div><p class="muted">Bis zu 20.000 Einheiten insgesamt. Mehrere Zufallsläufe zeigen Chancen und durchschnittliche Verluste; das Ergebnis ist keine Sieggarantie. Abfangraketen wirken nur gegen Raketen, nicht im Flottenkampf.</p></form><div id="war-result" role="status"></div>${rocketSimForm()}`);
 rocketSimTargets();if($('rocket-sim'))$('rocket-sim').elements.a_volk.value=view.volk;
 $('war-form').elements.angreifer_volk.value=view.volk;
 // Eigene Technik vorbelegt, Gegnertechnik und -volk bewusst eintragen (Funktionsprüfung 25.10.2026: still 0 bzw. erstes Volk).
}
function isOneWayMission(mission){return ['stationieren','kolonisieren'].includes(mission);}
function syncReturnFuel(){const one=isOneWayMission($('fleet').elements.mission.value);$('return-fuel').disabled=one;$('return-fuel-note').textContent=one?'Einwegmission: nur der Hinflug wird berechnet.':'Bei Rückflugmissionen reserviert der Server beide Strecken am Startplaneten.';}
function renderPlayerHelp(){if(!view)return;renderCorePlanner();renderExpeditionPanel();renderMissilePanel();$('inbox-shortcut').hidden=false;renderReportInbox();renderOverviewHelp();setupWarSim();if(!$('codex').hidden&&!document.activeElement?.closest('#codex-content'))renderCodex();}
function installPlayerHelp(){
 document.querySelector('header').insertAdjacentHTML('beforeend','<button id="inbox-shortcut" type="button" class="secondary" data-go="briefkasten" hidden><span aria-hidden="true">✉</span> Briefkasten <b id="inbox-count"></b></button>');
 $('tabs').insertAdjacentHTML('beforeend','<small class="nav-group">NACHSCHLAGEN & PLANEN</small><button type="button" data-tab="codex" aria-selected="false">Codex</button><button type="button" data-tab="warsim" aria-selected="false">Kampfsimulator</button>');
 $('game').insertAdjacentHTML('beforeend','<section class="panel" id="codex" hidden><h2>Codex · die ganze Galaxie</h2><div id="codex-content"></div></section><section class="panel" id="warsim" hidden><h2>Kampfsimulator</h2><div id="warsim-content"></div></section>');
 $('planets').insertAdjacentHTML('beforebegin','<div id="overview-intel" class="overview-intel"></div>');
 $('empire-summary').insertAdjacentHTML('afterend','<nav class="overview-tools" aria-label="Nachschlagen und planen"><button type="button" class="secondary" data-go="codex">Codex öffnen ↗</button><button type="button" class="secondary" data-go="warsim">Kampf simulieren ↗</button></nav>');
 document.querySelector('#briefkasten .comm-tabs').insertAdjacentHTML('beforeend','<button type="button" class="secondary" data-mail-folder="berichte" aria-pressed="false">Berichte & Meldungen <span id="mail-report-count"></span></button>');
 $('mail-requests-pane').insertAdjacentHTML('afterend',`<div id="mail-reports-pane" hidden><div class="reference-controls"><label>Berichtsart<select id="report-filter"><option value="">Alle</option>${reportGroups.map(([k,t])=>`<option value="${k}">${t}</option>`).join('')}</select></label><label>Suchen<input type="search" id="report-search" placeholder="Ziel oder Inhalt"></label></div><div id="mail-reports-list"></div></div>`);
 document.body.insertAdjacentHTML('beforeend','<dialog id="report-reader" class="letter-reader" aria-labelledby="report-reader-title"><header class="report-header"><div id="report-reader-context" class="report-context"></div><button type="button" class="ui-icon-button" data-close-report aria-label="Bericht schließen"><svg class="ui-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.7" aria-hidden="true"><path d="m6 6 12 12M18 6 6 18"/></svg></button><h2 id="report-reader-title"></h2></header><div id="report-reader-body"></div><footer class="section-actions" id="report-reader-actions"></footer></dialog>');
 $('fleet').insertAdjacentHTML('beforeend','<label class="return-fuel-check"><input type="checkbox" id="return-fuel" checked> Rückflugkraftstoff beim Abflug sichern <small id="return-fuel-note">Bei Rückflugmissionen reserviert der Server beide Strecken am Startplaneten.</small></label>');
}
installPlayerHelp();
document.addEventListener('game-navigation',()=>{if(view&&!$('codex').hidden)renderCodex();});
document.addEventListener('change',e=>{
 if(e.target.id==='codex-faction'){PlayerHelp.codex.faction=e.target.value;renderCodex();}
 if(e.target.id==='codex-level'){PlayerHelp.codex.level=Math.max(1,Math.min(e.target.max,Number(e.target.value)||1));renderCodex();}
 if(['report-filter','report-search'].includes(e.target.id))renderReportInbox();
 if(e.target.closest('#fleet'))syncReturnFuel();
});
document.addEventListener('input',e=>{if(e.target.id==='codex-search'){const caret=e.target.selectionStart;PlayerHelp.codex.query=e.target.value;renderCodex();$('codex-search').focus();$('codex-search').setSelectionRange(caret,caret);}if(e.target.id==='report-search')renderReportInbox();});
document.addEventListener('click',guard(async e=>{
 const b=e.target.closest('[data-codex-kind],[data-codex-key],[data-open-report],[data-close-report],[data-report-go],#sim-own,[data-sim-report]');if(!b)return;
 if(b.dataset.codexKind){PlayerHelp.codex.kind=b.dataset.codexKind;PlayerHelp.codex.key='';renderCodex();}
 if(b.dataset.codexKey){PlayerHelp.codex.key=b.dataset.codexKey;renderCodex();requestAnimationFrame(()=>document.querySelector('.codex-detail')?.scrollIntoView({behavior:matchMedia('(prefers-reduced-motion: reduce)').matches?'instant':'smooth',block:'start'}));}
 if(b.dataset.openReport){
  const x=reportRows().find(x=>x.id===b.dataset.openReport);if(!x)return;PlayerHelp.report=x;
  renderReportReader(x);
  if(!$('report-reader').open)$('report-reader').showModal();await api('/api/inbox/read',{world_id:world.world_id,bericht:x.id});await refresh();
 }
 if(b.hasAttribute('data-close-report'))$('report-reader').close();
 if(b.dataset.reportGo){$('report-reader').close();if(b.dataset.reportFolder){mailFolder=b.dataset.reportFolder;renderCommunication();}navigateGame(b.dataset.reportGo);}
 if(b.id==='sim-own'){
  const f=$('war-form');f.elements.angreifer_volk.value=view.volk;for(const k of ['waffentechnik','schildtechnik','panzerung'])f.elements['angreifer_'+k].value=view.forschung.stufen[k]||0;
  f.querySelectorAll('[data-sim-side="angreifer"]').forEach(i=>i.value=currentPlanet().schiffe[i.dataset.simUnit]||0);
 }
 if(b.hasAttribute('data-sim-report')){
  const r=PlayerHelp.report.row,f=$('war-form');
  if(PlayerHelp.report.kind==='berichte'&&(!r.schiffe||!r.verteidigung)||PlayerHelp.report.kind==='flottenberichte'&&!r.schiffstypen)throw Error('Dieser Bericht hat keine vollständige Zusammensetzung. Unbekannte Schiffe oder Verteidigung werden nicht als Null übernommen. Sende eine weitere Sonde oder plane ein eigenes Szenario.');
  f.elements.verteidiger_volk.value=r.volk||'';for(const k of ['waffentechnik','schildtechnik','panzerung'])f.elements['verteidiger_'+k].value=r.forschung?.[k]??'';
  f.querySelectorAll('[data-sim-side="verteidiger"]').forEach(i=>i.value=r.schiffstypen?.[i.dataset.simUnit]||r.schiffe?.[i.dataset.simUnit]||r.verteidigung?.[i.dataset.simUnit]||0);
  $('report-reader').close();navigateGame('warsim');show('Bericht übernommen. Volk und unbekannte Technik des Gegners vor der Simulation prüfen.');
 }
}));
document.addEventListener('input',e=>{if(!e.target.closest?.('#war-form'))return;const r=$('war-result');if(r&&r.textContent.trim()&&!r.querySelector('.sim-veraltet'))r.insertAdjacentHTML('afterbegin','<p class="sim-veraltet warning">Eingaben geändert – neu simulieren.</p>');});
document.addEventListener('submit',guard(async e=>{
 if(e.target.id!=='war-form')return;const f=e.target,party=name=>({volk:f.elements[name+'_volk'].value,technik:Object.fromEntries(['waffentechnik','schildtechnik','panzerung'].map(k=>[k,Number(f.elements[name+'_'+k].value)])),einheiten:Object.fromEntries([...f.querySelectorAll(`[data-sim-side="${name}"]`)].filter(i=>Number(i.value)>0).map(i=>[i.dataset.simUnit,Number(i.value)]))});
 // Eingaben am Auslöser prüfen und Fehler im Ergebnisbereich zeigen, altes Ergebnis nicht stehen lassen (Funktionsprüfung 25.10.2026).
 const A=party('angreifer'),D=party('verteidiger'),summe=x=>Object.values(x.einheiten).reduce((a,b)=>a+b,0),n=summe(A)+summe(D),fehl=t=>setStableHTML($('war-result'),`<p class="warning">${esc(t)}</p>`);
 // Pflichtangaben als Spielmeldung statt Browser-Blase (Nachprüfung 25.10.2026): Gegnervolk und -technik bewusst wählen.
 if(!D.volk)return fehl('Gegnervolk wählen – die Völker kämpfen unterschiedlich.');
 const leer=['waffentechnik','schildtechnik','panzerung'].filter(k=>f.elements['verteidiger_'+k].value.trim()==='');
 if(leer.length)return fehl(`Gegner-Technik eintragen (${leer.map(named).join(', ')}) – aus einem Spionagebericht übernehmen oder Stufe schätzen.`);
 if(!summe(A))return fehl('Keine Angriffsschiffe gewählt – mindestens ein Schiff eintragen.');
 if(n>20000)return fehl(`Insgesamt höchstens 20.000 Einheiten je Simulation – eingetragen sind ${fmt(n)}.`);
 let q;try{q=await api('/api/tool',{typ:'kampfszenario',angreifer:A,verteidiger:D});}catch(err){return fehl(typeof kernLesbar==='function'?kernLesbar(err.message||String(err)):(err.message||String(err)));}
 const iv=q.siegchance_intervall_95_prozent;
 setStableHTML($('war-result'), `<h3>Simulation · ${q.laeufe} Durchläufe</h3><div class="stats">${stat(fmt(q.siegchance_prozent)+' %','Siegchance'+(Array.isArray(iv)?` (95 %: ${fmt(iv[0])}–${fmt(iv[1])} %)`:''))}${stat(fmt(q.unentschieden_prozent)+' %','Unentschieden')}${stat(fmt(q.eigene_verluste_wert),'Eigene Verluste · Wert')}${stat(fmt(q.verluste_gegner_wert),'Gegnerverluste · Wert')}</div><p>Durchschnittlich überlebende Angreifer: ${esc(goods(q.ueberlebende_angreifer)||'Keine')}</p><p>Durchschnittlich überlebende Verteidiger: ${esc(goods(q.ueberlebende_verteidiger)||'Keine')}</p><p class="muted">${esc(q.hinweis)}</p>`);
}));


// Planner drafts are scoped to world, account and planet; saving a draft never sends a command.
const CorePlan={key:'',rows:[],variant:'A',request:0,result:null};
function planIdentity(){return world&&view&&currentPlanet()?JSON.stringify([world.world_id,view.name,currentPlanet().koord]):'';}
function planStorage(){return 'sternenepoche:plan:v1:'+CorePlan.key+':'+CorePlan.variant;}
function loadPlanDraft(){try{const v=JSON.parse(localStorage.getItem(planStorage())||'[]');CorePlan.rows=Array.isArray(v)?v.slice(0,32):[];}catch{CorePlan.rows=[];}CorePlan.result=null;CorePlan.request++;}
function savePlanDraft(){try{localStorage.setItem(planStorage(),JSON.stringify(CorePlan.rows));}catch{show('Entwurf konnte lokal nicht gespeichert werden',true);}CorePlan.result=null;CorePlan.request++;drawPlanRows();}
function planLabel(a){return `${named(a.typ)} · ${named(a.gebaeude||a.forschung||a.einheit||a.bauteil||'')} ${a.anzahl?('× '+a.anzahl):''}`;}
function drawPlanRows(){if(!$('core-plan-rows'))return;setStableHTML($('core-plan-rows'), CorePlan.rows.map((a,i)=>`<article class="fleet-card"><strong>${i+1}. ${esc(planLabel(a))}</strong><div class="section-actions"><button type="button" class="secondary" data-plan-move="${i}" data-direction="-1" ${i===0?'disabled':''} aria-label="Schritt ${i+1} nach oben">↑</button><button type="button" class="secondary" data-plan-move="${i}" data-direction="1" ${i===CorePlan.rows.length-1?'disabled':''} aria-label="Schritt ${i+1} nach unten">↓</button><button type="button" class="secondary" data-plan-delete="${i}">Entfernen</button></div></article>`).join('')||'<p>Noch keine Schritte. Ohne Schritte zeigt die Berechnung die bestehende Wirtschaft.</p>');if(!CorePlan.result)$('core-plan-result').textContent='Entwurf geändert; neu berechnen.';}
function planChoices(){const kind=$('core-plan-kind').value;const options=kind==='bauen'?Object.keys(rules.catalog.gebaeude):kind==='forschen'?Object.keys(rules.catalog.forschung):Object.keys(rules.catalog.einheiten);setStableHTML($('core-plan-item'), options.map(k=>`<option value="${esc(k)}">${esc(named(k))}</option>`).join(''));$('core-plan-quantity').hidden=kind!=='fertigen';}
function renderCorePlanner(){
 if(!$('core-planner'))return;const key=planIdentity();if(!key)return;
 if(CorePlan.key===key&&$('core-plan-add'))return;CorePlan.key=key;loadPlanDraft();
 setStableHTML($('core-planner'), `<h2>Ausbau planen · ${esc(currentPlanet().koord)}</h2><p>Vergleiche zwei Reihenfolgen mit den aktuellen Spielregeln. Bereits bezahlte Aufträge, Versorgung und Wirtschaftsticks sind enthalten. Jeder Schritt wartet auf den Abschluss seiner Warteschlange.</p><div class="reference-controls"><label>Entwurf<select id="core-plan-variant"><option>A</option><option>B</option></select></label><label>Horizont in Spielstunden<input id="core-plan-horizon" type="number" min="1" max="168" value="24"></label></div><form id="core-plan-add"><div class="reference-controls"><label>Vorhaben<select id="core-plan-kind"><option value="bauen">Gebäude ausbauen</option><option value="forschen">Forschen</option><option value="fertigen">Einheiten fertigen</option></select></label><label>Auswahl<select id="core-plan-item"></select></label><label id="core-plan-quantity" hidden>Anzahl<input id="core-plan-count" type="number" min="1" max="10000" value="1"></label><button type="submit">Zum Entwurf hinzufügen</button></div></form><div id="core-plan-rows"></div><div class="section-actions"><button type="button" id="core-plan-calculate">Entwurf berechnen</button><button type="button" class="secondary" id="core-plan-compare">A und B vergleichen</button></div><div id="core-plan-result" role="status"></div>`);
 $('core-plan-variant').value=CorePlan.variant;planChoices();drawPlanRows();
}
function renderPlanResult(r,title='Ergebnis'){
 const labels={completed:'Abgeschlossen',horizon_reached:'Horizont oder Rechenlimit erreicht',blocked:'Blockiert',unknown:'Unbekannter Verlauf'};
 const end=r.endzustand, p=end?.planeten?.find(p=>p.planet===currentPlanet().koord);
 return `<article class="fleet-card"><h3>${esc(title)} · ${esc(labels[r.status]||r.status)}</h3>${r.grund?`<p>${esc(r.grund)}</p>`:''}<ol>${(r.schritte||[]).map(x=>`<li>${esc(planLabel(x.aktion))}: Start ${esc(GameTime.duration(x.begonnen))}${x.fertig_im_fenster!==null?' · fertig '+esc(GameTime.duration(x.fertig_im_fenster)):' · noch offen'}</li>`).join('')}</ol>${p?`<p>Einwohner: ${fmt(p.bevoelkerung_milli/1000)} · Wohnraum: ${fmt(p.wohnraum_milli/1000)} · Credits: ${fmt(end.credits_milli/1000)}</p><div class="table-wrap">${table(['Gut','Bestand am Ende','Rate je Spielstunde'],r.kontext.gueter_reihenfolge.map((g,i)=>[named(g),fmt(p.bestand_milli[i]/1000),fmt(p.rate_milli[i]/1000)]))}</div>`:''}<p class="muted">${esc((r.annahmen||[]).join(' · '))}</p></article>`;
}
async function calculatePlan(compare=false){
 const key=planIdentity(), request=++CorePlan.request, hours=Number($('core-plan-horizon').value), current=CorePlan.variant;
 if(!Number.isInteger(hours)||hours<1||hours>168)throw Error('Horizont: 1 bis 168 ganze Spielstunden');
 $('core-plan-result').textContent='Core berechnet den Verlauf …';
 const variants=compare?['A','B']:[current], results=[];
 for(const variant of variants){let rows=CorePlan.rows;if(variant!==current){try{rows=JSON.parse(localStorage.getItem('sternenepoche:plan:v1:'+key+':'+variant)||'[]');}catch{rows=[];}}
 const r=await api('/api/tool',{typ:'ablaufplan',plan:{schritte:rows,horizont_stunden:hours,rolle:'alle'}});results.push([variant,r]);}
 if(key!==planIdentity()||request!==CorePlan.request)return;
 setStableHTML($('core-plan-result'), results.map(([name,r])=>renderPlanResult(r,'Entwurf '+name)).join(''));
 CorePlan.result=compare?null:results[0][1];
 if(CorePlan.result?.schritte?.[0]?.begonnen===CorePlan.result?.startzeit)$('core-plan-result').insertAdjacentHTML('beforeend','<button type="button" id="core-plan-apply">Ersten Schritt jetzt starten</button>');
}
function expeditionFlight(){const p=currentPlanet(),schiffe={},ladung={};for(const e of ['pathfinder','bergungsschiff','handelsfrachter','spionagesonde']){const n=Number($('exp-'+e).value);if(!Number.isInteger(n)||n<0||n>10000)throw Error('Ganze Schiffszahlen zwischen 0 und 10.000 eingeben');if(n)schiffe[e]=n;}
 for(const g of ['erz','kristall','deuterium','legierung','elektronik']){const n=Number($('exp-cargo-'+g).value);if(!Number.isFinite(n)||n<0)throw Error('Fracht darf nicht negativ sein');if(n)ladung[g]=n;}
 return {typ:'flotte_senden',mission:'saven',start:p.koord,ziel:$('exp-target').value,schiffe,ladung,geschwindigkeit:1,haltedauer_stunden:0};}
function habitabilityMarkup(info,groups){
 if(!info?.herkunft)return '';const unsafe=info.herkunft.filter((e,i)=>!e.bewohnbar&&groups?.[i]?.some(n=>n>0));
 return `<h3>Bewohnbarkeit</h3><p>Sonnenabstand: ${esc(named(info.zone))} · Solarertrag ${fmt(info.solar_promille/10)} % · Terraforming ${info.herkunft[0]?.terraforming??0}</p>${table(['Herkunft','Klima-Wachstumsfaktor','Besiedlung'],info.herkunft.map(e=>[esc(named(e.volk)),e.bewohnbar?fmt(e.wachstum_promille/1000)+'×':'Kein Wachstum',e.bewohnbar?'Bewohnbar':'Ab Terraforming '+e.terraforming_noetig]))}${unsafe.length?`<p class="minus">Unbewohnbare Lebensbedingungen: ${unsafe.map(e=>esc(named(e.volk))+' verlieren '+fmt(e.klimaverlust_promille_je_stunde/10)+' % Bevölkerung je Spielstunde').join('; ')}. Evakuierung oder Terraforming erforderlich.</p>`:''}<p class="muted">Klima multipliziert positives Wachstum. Nahrung bzw. Energie, Stabilität und Wohnraum begrenzen es zusätzlich. Syntheten benötigen auch im Eis Strom; Terraforming verstärkt die Sonne nicht.</p>`;
}
let ExpeditionPanelKey='';
function renderExpeditionPanel(){
 if(!$('expeditions-panel')||!view)return;const key=planIdentity();
 if(ExpeditionPanelKey!==key){ExpeditionPanelKey=key;const p=currentPlanet();
 setStableHTML($('expeditions-panel'), `<h2>Expeditionen und Bevölkerung</h2><p>Pathfinder erweitern die Suche von zwei auf höchstens acht Ereignisse. Bordsonden kehren nach einem Scan zurück, sofern sie nicht verloren gehen. Vermisste Expeditionen haben eine Startfrist von 24 Spielstunden für Rettung; der direkte Anflug kommt hinzu.</p><form id="exp-start"><label>Ziel · Sektor:System:0<input id="exp-target" value="${esc(p.koord.split(':').slice(0,2).join(':')+':0')}" required></label><div class="reference-controls">${['pathfinder','bergungsschiff','handelsfrachter','spionagesonde'].map(e=>`<label>${esc(named(e))} · vorhanden ${p.schiffe[e]||0}<input id="exp-${e}" type="number" min="0" max="10000" value="0"></label>`).join('')}</div><details><summary>Fracht für Funde und Notreparatur</summary><div class="reference-controls">${['erz','kristall','deuterium','legierung','elektronik'].map(g=>`<label>${esc(named(g))}<input id="exp-cargo-${g}" type="number" min="0" value="0"></label>`).join('')}</div></details><button type="submit">Expedition starten</button></form><h3>Ausbildung</h3><p>Ausbildung zu Fachkräften und Forschern läuft in der Akademie – mit Kosten aus dem Kern und Platzprüfung. <button type="button" class="bs-link" data-open-academy>Akademie öffnen ›</button></p><h3>Personentransport</h3><form id="person-send"><p>Verwendet die oben ausgewählte Flotte und Fracht. Personenplätze sind zusätzlich zur Betriebsbesatzung.</p><label>Eigener Zielplanet<select id="person-destination">${view.planeten.filter(t=>t.koord!==p.koord).map(t=>`<option>${esc(t.koord)}</option>`).join('')}</select></label><label>Qualifikation der Reisenden<select id="person-travel-qualification"><option value="0">Bevölkerung</option><option value="1">Fachkräfte</option><option value="2">Forscher</option></select></label><p>Herkunft und Anzahl aus den Feldern bei Ausbildung. Alle landen, auch bei Überbelegung.</p><button type="submit">Personentransport starten</button></form><div id="exp-status"></div>`);}
 const p=currentPlanet(),groups=p.personengruppen?.gruppen;
 setStableHTML($('exp-status'), `<h3>Bevölkerung auf ${esc(p.koord)}</h3>${groups?table(['Herkunft','Ohne Qualifikation','Fachkräfte','Forscher'],['aurelianer','krath','veyari','syntheten'].map((v,i)=>[named(v),...groups[i].map(n=>fmt(n/1000))])):'<p>Neue Partie für diese Mechaniken erforderlich.</p>'}${p.ueberbelegung?`<p class="minus">Überbelegung: ${fmt(p.ueberbelegung)} fehlende Wohnplätze.</p>`:''}${habitabilityMarkup(p.habitabilitaet,groups)}<h3>Laufende Expeditionen</h3>${(view.expeditionen||[]).map(e=>`<article class="fleet-card"><h4>Expedition ${e.flotte} · ${esc(e.status)}</h4><p>${e.ereignisse} Ereignisse${e.rettungsfrist?' · Rettung starten in den nächsten '+esc(GameTime.duration(Math.max(0,e.rettungsfrist-GameTime.now())))+' Spielzeit':''}</p>${(e.berichte||[]).length?`<details class="exp-logbuch" open><summary>Logbuch · ${fmt(e.berichte.length)} Einträge</summary><ol>${e.berichte.slice().reverse().map(t=>`<li>${esc(t)}</li>`).join('')}</ol></details>`:''}${e.status==='vermisst'?`<button type="button" data-rescue="${e.flotte}">Mit ausgewählter Flotte retten</button>`:''}${Object.entries(e.ruempfe||{}).filter(([,n])=>n>0).map(([ship,n])=>`<p>${fmt(n)} beschädigte ${esc(named(ship))} <button type="button" data-repair-exp="${e.flotte}" data-ship="${esc(ship)}">Einen Rumpf reparieren</button></p>`).join('')}</article>`).join('')||'<p>Keine Expedition gestartet.</p>'}`);
}
$('tabs').insertAdjacentHTML('beforeend','<button type="button" data-tab="core-planner" aria-selected="false">Ausbauplaner</button><button type="button" data-tab="expeditions-panel" aria-selected="false">Expeditionen & Personen</button>');
$('game').insertAdjacentHTML('beforeend','<section class="panel" id="core-planner" hidden></section><section class="panel" id="expeditions-panel" hidden></section>');
document.addEventListener('change',e=>{if(e.target.id==='core-plan-kind')planChoices();if(e.target.id==='core-plan-variant'){CorePlan.variant=e.target.value;loadPlanDraft();drawPlanRows();}if(e.target.id==='core-plan-horizon'){CorePlan.request++;CorePlan.result=null;drawPlanRows();}});
document.addEventListener('submit',guard(async e=>{
 if(e.target.id==='core-plan-add'){e.preventDefault();if(CorePlan.rows.length>=32)throw Error('Höchstens 32 Schritte');const typ=$('core-plan-kind').value,item=$('core-plan-item').value,a={typ,planet:currentPlanet().koord};a[typ==='bauen'?'gebaeude':typ==='forschen'?'forschung':'einheit']=item;if(typ==='fertigen'){a.anzahl=Number($('core-plan-count').value);if(!Number.isInteger(a.anzahl)||a.anzahl<1||a.anzahl>10000)throw Error('Anzahl: 1 bis 10.000');}CorePlan.rows.push(a);savePlanDraft();}
 if(e.target.id==='exp-start'){e.preventDefault();await command([{typ:'expedition_starten',flug:expeditionFlight()}]);}
 if(e.target.id==='person-school'){e.preventDefault();await command([{typ:'ausbilden',planet:currentPlanet().koord,volk:$('person-origin').value,qualifikation:Number($('person-qualification').value),anzahl:Number($('person-count').value)}]);}
 if(e.target.id==='person-send'){e.preventDefault();const flug=expeditionFlight();flug.mission='transport';flug.ziel=$('person-destination').value;const gruppen=Array.from({length:4},()=>[0,0,0]);gruppen[['aurelianer','krath','veyari','syntheten'].indexOf($('person-origin').value)][Number($('person-travel-qualification').value)]=Number($('person-count').value)*1000;await command([{typ:'personen_senden',flug,personen:{gruppen}}]);}
}));
document.addEventListener('click',guard(async e=>{
 const b=e.target.closest('[data-plan-delete],[data-plan-move],#core-plan-calculate,#core-plan-compare,#core-plan-apply,[data-rescue],[data-repair-exp]');if(!b)return;
 if(b.dataset.planDelete!==undefined){CorePlan.rows.splice(Number(b.dataset.planDelete),1);savePlanDraft();}
 if(b.dataset.planMove!==undefined){const i=Number(b.dataset.planMove),j=i+Number(b.dataset.direction);if(j>=0&&j<CorePlan.rows.length){[CorePlan.rows[i],CorePlan.rows[j]]=[CorePlan.rows[j],CorePlan.rows[i]];savePlanDraft();}}
 if(b.id==='core-plan-calculate'||b.id==='core-plan-compare')await calculatePlan(b.id==='core-plan-compare');
 if(b.id==='core-plan-apply'){if(!CorePlan.result||CorePlan.key!==planIdentity())throw Error('Plan bitte neu berechnen');b.disabled=true;const result=await command([{typ:'plan_uebernehmen',revision:CorePlan.result.revision,schritte:CorePlan.rows.slice(0,1)}]);if(result.ergebnisse.every(x=>x.ok)){CorePlan.rows.shift();savePlanDraft();}else{CorePlan.result=null;$('core-plan-result').textContent='Zustand geändert oder Schritt nicht ausführbar. Bitte neu berechnen.';}}
 if(b.dataset.rescue){const flug=expeditionFlight(),f=view.flotten.find(f=>String(f.flotte??f.id)===b.dataset.rescue);if(f?.ziel)flug.ziel=f.ziel;await command([{typ:'rettung_starten',expedition:Number(b.dataset.rescue),flug}]);}
 if(b.dataset.repairExp)await command([{typ:'expedition_reparieren',flotte:Number(b.dataset.repairExp),einheit:b.dataset.ship}]);
}));

function marketDraft(){const f=$('market-form').elements;return {typ:'markt_order',planet:f.planet.value,gut:f.gut.value,seite:f.seite.value,menge:Number(f.menge.value),preis:Number(f.preis.value)};}
document.addEventListener('click',guard(async e=>{
 if(!e.target.closest('#market-preview'))return;const identity=planIdentity(),a=marketDraft();
 const q=await api('/api/tool',{typ:'marktplan',aktion:a});if(identity!==planIdentity()||JSON.stringify(a)!==JSON.stringify(marketDraft()))return;
 setStableHTML($('market-quote'), q.status==='ready'?`<p>Credits hinterlegt oder verbraucht: ${fmt(q.credits_reserviert_oder_verbraucht_milli/1000)} · Danach verfügbar: ${fmt(q.credits_nachher_milli/1000)}</p><p>${q.lieferungen.map(l=>`${esc(named(l.gut))}: ${fmt(l.menge_milli/1000)} · Ankunft ${esc(GameTime.duration(l.ankunft))}`).join('<br>')||'Noch keine garantierte Warenlieferung.'}</p><p class="muted">${esc(q.annahmen.join(' · '))}</p>`:`<p>${esc(q.grund)}</p>`);
}));


// Missile data and launch predictions come from the same core as production and impacts.
function missileDraft(){
 const f=$('missile-launch').elements,n=Number(f.anzahl.value),id=f.flotte.value?Number(f.flotte.value):null;
 if(!Number.isInteger(n)||n<1||n>1000000)throw Error('Ganze Raketenanzahl zwischen 1 und 1.000.000 eingeben');
 const target=id===null?f.ziel.value:(view.angriffe||[]).find(a=>a.flotte===id)?.ziel;
 if(!target)throw Error('Dieser Anflug ist nicht mehr sichtbar');
 return {typ:'raketen_starten',start:currentPlanet().koord,ziel:target,anzahl:n,zieltyp:f.zieltyp.value,art:f.art.value,flotte:id};
}
// Neuester Spionagebericht zum Planetenziel (Verteidigung je Klasse) – Raketen treffen nur die gewählte Zielklasse.
function missileBericht(f){
 if(f.flotte.value)return null;const z=String(f.ziel.value||'').trim();
 return (view.berichte||[]).filter(b=>b.ziel===z&&b.verteidigung).sort((a,b)=>(b.zeit||0)-(a.zeit||0))[0]||null;
}
// Startart passend zum Zielmodus: Planetenziel nur Arten für Planeten, Anflug nur Arten gegen Flotten (Kern raketen.rs
// lehnt sonst „nicht für Planetenverteidigung geeignet“ ab); frisch gebaute Raketen erscheinen ohne Neuladen.
function raketenArten(){
 const form=$('missile-launch'),artSel=form?.elements?.art,p=currentPlanet();if(!artSel||!p?.raketensilo||document.activeElement===artSel)return false;
 const flotte=!!form.elements.flotte?.value,arten=p.raketensilo.typen.filter(t=>t.bestand>0&&(flotte?t.regel.flotte:t.regel.planet)),sig=(flotte?'f:':'p:')+arten.map(t=>t.regel.key+t.bestand).join(',');
 if(artSel.dataset.arten===sig)return false;const alt=artSel.value;
 artSel.innerHTML=arten.map(t=>`<option value="${esc(t.regel.key)}">${esc(t.regel.name)} · ${fmt(t.bestand)}</option>`).join('')||`<option value="">${flotte?'Keine Raketen gegen Flotten im Silo':'Keine Raketen gegen Planeten im Silo'}</option>`;
 artSel.dataset.arten=sig;if(arten.some(t=>t.regel.key===alt))artSel.value=alt;return true;
}
// Summe für die eingetragene Menge (Kosten und Lagerplätze), live beim Tippen.
function raketenSumme(p,key,n){
 const t=p?.raketensilo?.typen?.find(t=>t.regel.key===key);if(!t||!(n>=1))return 'Menge ab 1 eingeben.';
 const k=Object.entries(t.kosten_aktuell||t.regel.kosten||{}).filter(([,v])=>v>0).map(([g,v])=>`${fmt(v*n)} ${named(g)}`).join(' · ');
 const frei=Math.max(0,(p.raketensilo.kapazitaet||0)-(p.raketensilo.belegt||0)),platz=n*(t.regel.plaetze||1);
 return `${fmt(n)} Stück: ${k} · ${fmt(platz)} Lagerplätze (frei ${fmt(frei)})${platz>frei?' – Silo zu klein':''}`;
}
document.addEventListener('input',e=>{const f=e.target.closest?.('form[data-missile-build]');if(!f||e.target.name!=='anzahl')return;const out=f.querySelector('[data-missile-summe]');if(out)out.textContent=raketenSumme(currentPlanet(),f.dataset.missileBuild,Number(e.target.value));});
function missileTargetClasses(){
 const form=$('missile-launch');if(!form)return;const f=form.elements,isFleet=!!f.flotte.value,old=f.zieltyp.value;
 f.ziel.disabled=isFleet;
 // Spielprobe 20.10.2026: Vorauswahl „Gaußkanone“ gegen ein Ziel ohne Gaußkanonen – beide Raketen verpufften
 // („0 gausskanone zerstört“). Jetzt: Anzahl laut Bericht je Klasse, stärkste vorhandene Klasse vorgewählt, Warnung bei 0.
 const b=missileBericht(f),v=b?.verteidigung||{},zielKey=(isFleet?'flotte:'+f.flotte.value:f.ziel.value.trim());
 setStableHTML(f.zieltyp, Object.entries(rules.catalog.einheiten||{}).filter(([,e])=>e.schiff===isFleet).map(([key])=>`<option value="${esc(key)}">${esc(named(key))}${b?' · '+fmt(v[key]||0):''}</option>`).join(''));
 const da=k=>[...f.zieltyp.options].some(o=>o.value===k);
 // Nur eine eigene Wahl für genau dieses Ziel bleibt stehen; der Browser-Standard (erste Option) ist keine Wahl
 // (Funktionsprüfung 25.10.2026: vorgewählt waren 4 Gaußkanonen statt 60 Raketenwerfer).
 if(da(old)&&form.dataset.zieltypFuer===zielKey)f.zieltyp.value=old;
 else if(b){const best=Object.entries(v).filter(([k,n])=>n>0&&da(k)).sort((x,y)=>y[1]-x[1])[0];if(best)f.zieltyp.value=best[0];else if(da(old))f.zieltyp.value=old;}
 else if(da(old))f.zieltyp.value=old;
 raketenArten();
 let info=$('missile-ziel-info');
 if(!info){$('missile-quote')?.insertAdjacentHTML('beforebegin','<p id="missile-ziel-info" class="muted"></p>');info=$('missile-ziel-info');}
 if(info){
  const sel=f.zieltyp.value,n=v[sel]||0,alt=b?.alter_stunden!=null?` (vor ${fmt(b.alter_stunden)} h)`:'';
  info.classList.toggle('bs-rot',!!b&&n===0);
  info.dataset.basis=info.textContent=isFleet?'Ziel ist der gewählte Anflug: getroffen werden nur Schiffe der Zielklasse.'
   :!b?'Kein Spionagebericht zu diesem Ziel: Raketen treffen nur die gewählte Zielklasse – vorher ausspähen.'
   :n>0?`Laut Spionagebericht${alt}: ${fmt(n)} ${named(sel)} am Ziel.`
   :`Laut Spionagebericht${alt} hat das Ziel keine ${named(sel)} – diese Raketen würden nichts treffen. Andere Zielklasse wählen.`;
  if(b&&n>0)raketenErwartung();
 }
 $('missile-quote').textContent='Startvorschau neu berechnen.';
}
// Erwartung der Salve mit den Zahlen des Berichts (Kernwerkzeug „raketensimulator“, dieselbe Rechnung wie der Einschlag).
// Spielprobe 20.10.: 2 Interplanetarraketen auf 4 Gaußkanonen zerstörten 0 – die Startvorschau sagte nur „Start möglich“.
let raketenErwartungNr=0,raketenErwartungT=0;
function raketenErwartung(){
 clearTimeout(raketenErwartungT);
 raketenErwartungT=setTimeout(async()=>{
  const form=$('missile-launch'),info=$('missile-ziel-info');if(!form||!info)return;
  // Ohne gewählte Raketenart (leeres Silo) keine Abfrage – sonst HTTP 400 und Konsolenfehler (Einsatzprobe 25.10.2026).
  const f=form.elements,b=missileBericht(f),typ=f.zieltyp.value,n=b?.verteidigung?.[typ]||0;if(!b||!n||!f.art?.value)return;
  const nr=++raketenErwartungNr,basis=info.dataset.basis,abfang=b.vollscan?.raketen?.abfang;
  try{
   const q=await api('/api/tool',{typ:'raketensimulator',art:f.art.value,anzahl:Math.max(1,Math.trunc(Number(f.anzahl.value))||1),zieltyp:typ,ziel_anzahl:n,
    angreifer:{volk:view.volk,waffentechnik:view.forschung?.stufen?.waffentechnik||0},verteidiger:{volk:b.volk,panzerung:b.forschung?.panzerung||0,abfangraketen:abfang||0}});
   if(nr!==raketenErwartungNr||info.dataset.basis!==basis)return;
   const je=q.schaden_je_rakete>0?Math.ceil(q.struktur_je_ziel/q.schaden_je_rakete):0;
   let satz=` Erwartung: ${fmt(q.treffer)} Treffer zerstören ${fmt(q.zerstoert)} von ${fmt(n)}${q.abgefangen?` (${fmt(q.abgefangen)} abgefangen)`:''}.`;
   if(abfang==null)satz+=' Abfangraketen des Ziels unbekannt (nur ein Vollscan zeigt sie).';
   // Panzerung des Ziels nur mit Vollscan bekannt: als Spanne rechnen statt still 0 (Nachprüfung 25.10.2026: 14 erwartet, 9 zerstört).
   if(b.forschung?.panzerung==null){const eigen=Number(view.forschung?.stufen?.panzerung)||0;
    if(eigen>0){const q2=await api('/api/tool',{typ:'raketensimulator',art:f.art.value,anzahl:Math.max(1,Math.trunc(Number(f.anzahl.value))||1),zieltyp:typ,ziel_anzahl:n,
     angreifer:{volk:view.volk,waffentechnik:view.forschung?.stufen?.waffentechnik||0},verteidiger:{volk:b.volk,panzerung:eigen,abfangraketen:abfang||0}}).catch(()=>null);
     if(nr!==raketenErwartungNr||info.dataset.basis!==basis)return;
     satz+=q2?` Panzerung des Ziels unbekannt: ${fmt(q.zerstoert)} ohne Panzerung, ${fmt(q2.zerstoert)} bei Panzerung ${fmt(eigen)} wie deiner.`:' Panzerung des Ziels unbekannt – gerechnet ohne Panzerung.';}
    else satz+=' Panzerung des Ziels unbekannt – gerechnet ohne Panzerung.';}
   if(!q.zerstoert)satz+=je>0?` Zu wenig: eine ${named(typ)} braucht etwa ${fmt(je)} Treffer.`:' Diese Rakete richtet an dieser Zielklasse nichts aus.';
   info.textContent=basis+satz;info.classList.toggle('bs-rot',!q.zerstoert);
  }catch{}
 },500);
}
// Manuelle Zielklasse für dieses Ziel merken (vor dem Neuaufbau der Liste); Zieleingabe aktualisiert die Klassen sofort.
document.addEventListener('change',e=>{const f=e.target.form;if(f?.id==='missile-launch'&&e.target.name==='zieltyp')f.dataset.zieltypFuer=f.elements.flotte.value?'flotte:'+f.elements.flotte.value:f.elements.ziel.value.trim();},true);
document.addEventListener('input',e=>{if(e.target.form?.id==='missile-launch'&&['ziel','anzahl'].includes(e.target.name))missileTargetClasses();});
// Raketenschlag mit derselben Core-Rechnung wie der echte Einschlag (Werkzeug "raketensimulator").
function rocketSimForm(){
 const types=(rules.catalog.raketen||[]).filter(r=>r.planet||r.flotte),volk=Object.keys(rules.catalog.voelker).map(k=>`<option value="${k}">${esc(named(k))}</option>`).join('');
 if(!types.length)return '';
 const own=view.forschung?.stufen||{};
 return `<h3 id="rocket-sim-title">Raketenschlag simulieren</h3><p class="muted">Raketen werden unter Verteidigung im Raketensilo gebaut. Berücksichtigt werden: Abfangraketen des Ziels, Panzerung, Volksboni und Zielboni der Rakete.</p><form id="rocket-sim"><div class="input-grid"><label>Rakete<select name="art">${types.map(t=>`<option value="${t.key}">${esc(t.name)}</option>`).join('')}</select></label><label>Anzahl<input name="anzahl" type="number" min="1" max="100000" value="10"></label><label>Zielklasse<select name="zieltyp"></select></label><label>Ziele vorhanden<input name="ziel_anzahl" type="number" min="0" max="10000000" value="100"></label><label>Dein Volk<select name="a_volk">${volk}</select></label><label>Deine Waffentechnik<input name="a_waffen" type="number" min="0" max="30" value="${Number(own.waffentechnik||0)}"></label><label>Volk des Ziels<select name="v_volk">${volk}</select></label><label>Panzerung des Ziels<input name="v_panzerung" type="number" min="0" max="30" placeholder="unbekannt"></label><label>Abfangraketen des Ziels<input name="v_abfang" type="number" min="0" max="1000000" value="0"></label><label class="check"><input name="v_silo" type="checkbox" checked> Zielsilo intakt</label></div><div class="section-actions"><button type="submit">Raketenschlag simulieren</button></div></form><div id="rocket-sim-result" role="status"></div>`;
}
function rocketSimTargets(){
 const f=$('rocket-sim');if(!f)return;const t=(rules.catalog.raketen||[]).find(r=>r.key===f.elements.art.value);const old=f.elements.zieltyp.value;
 setStableHTML(f.elements.zieltyp, Object.keys(t?.zielprofil?.schadensfaktoren||{}).map(k=>`<option value="${k}">${esc(named(k))}</option>`).join(''));
 if([...f.elements.zieltyp.options].some(o=>o.value===old))f.elements.zieltyp.value=old;
}
document.addEventListener('change',e=>{if(e.target.closest('#rocket-sim')&&e.target.name==='art')rocketSimTargets();});
document.addEventListener('submit',guard(async e=>{
 if(e.target.id!=='rocket-sim')return;e.preventDefault();const f=e.target.elements,n=k=>Number(f[k].value)||0;
 const sim=panzerung=>api('/api/tool',{typ:'raketensimulator',art:f.art.value,anzahl:n('anzahl'),zieltyp:f.zieltyp.value,ziel_anzahl:n('ziel_anzahl'),
  angreifer:{volk:f.a_volk.value,waffentechnik:n('a_waffen')},verteidiger:{volk:f.v_volk.value,panzerung,abfangraketen:n('v_abfang'),silo_intakt:f.v_silo.checked}});
 // Leeres Panzerungsfeld = unbekannt: als Spanne rechnen statt still 0, wie die Starterwartung (Nachprüfung 25.10.2026: 14 erwartet, 9 zerstört).
 const unbekannt=f.v_panzerung.value.trim()==='',q=await sim(n('v_panzerung')),eigen=Number(view.forschung?.stufen?.panzerung)||0;
 const q2=unbekannt&&eigen>0?await sim(eigen).catch(()=>null):null;
 if(unbekannt)q.hinweis=(q2?`Panzerung des Ziels unbekannt: ${fmt(q.zerstoert)} zerstört ohne Panzerung, ${fmt(q2.zerstoert)} bei Panzerung ${fmt(eigen)} wie deiner. `:'Panzerung des Ziels unbekannt – gerechnet ohne Panzerung. ')+(q.hinweis||'');
 setStableHTML($('rocket-sim-result'), `<article class="fleet-card"><strong>${esc(q.name)}: ${fmt(q.zerstoert)} von ${fmt(q.ziel_anzahl)} ${esc(named(q.zieltyp))} zerstört</strong><dl>${factRow('Gestartet',fmt(q.anzahl))}${factRow('Abgefangen',fmt(q.abgefangen),fmt(q.abfangraketen_verbraucht)+' Abfangraketen verbraucht')}${factRow('Treffer',fmt(q.treffer))}${factRow('Schaden je Rakete',fmt(q.schaden_je_rakete),'mit Waffentechnik und Volksbonus beim Start')}${factRow('Struktur je Ziel',fmt(q.struktur_je_ziel),'mit Panzerung und Volksbonus des Ziels')}${factRow('Zielbonus',('×'+fmt(q.faktor)))}${factRow('Höchstens je Rakete',fmt(q.hoechstens_je_rakete))}${factRow('Übrig',fmt(q.verbleibend))}</dl><p class="muted">${esc(q.hinweis)}</p></article>`);
}));
function renderMissilePanel(){
 const host=$('missile-panel'),p=currentPlanet();if(!host||!p?.raketensilo)return;
 if(host.dataset.identity!==planIdentity()){
 host.dataset.identity=planIdentity();setStableHTML(host, `<h2 id="raketen">Raketen · Raketensilo ${esc(p.koord)}</h2><p>Raketen werden hier unter Verteidigung im Raketensilo gebaut und gelagert (nicht in der Werft). Bauaufträge reservieren ihre Plätze sofort. Abfangraketen schützen automatisch diesen Planeten. Alle Flugzeiten sind Spielzeit.</p><div id="missile-stock"></div><h3>Taktischer Raketenstart</h3><form id="missile-launch"><div class="reference-controls"><label>Rakete<select name="art">${p.raketensilo.typen.filter(t=>t.bestand>0&&(t.regel.planet||t.regel.flotte)).map(t=>`<option value="${t.regel.key}">${esc(t.regel.name)}</option>`).join('')}</select></label><label>Zielmodus<select name="flotte"><option value="">Planet</option></select></label><label>Zielplanet<input name="ziel" placeholder="1:27:6" value="${esc(p.koord)}"></label><label>Zielklasse<select name="zieltyp"></select></label><label>Anzahl<input name="anzahl" type="number" min="1" max="1000000" value="1"></label></div><p>Flottenziele sind erkannte feindliche Anflüge auf eigene oder verbündete Planeten. Raketen müssen vor der Flotte ankommen. Rückruf oder Zielwechsel kann den Schuss vereiteln.</p><button type="button" id="missile-preview">Start im Core prüfen</button> <button type="submit">Raketen verbindlich starten</button><p id="missile-quote" aria-live="polite"></p></form><div id="missile-salvos"></div>`);
 missileTargetClasses();
 }
 // Startbare Raketenarten bei jedem Zeichnen nachführen – das Formular entsteht nur bei neuer planIdentity (Einsatzprobe
 // 25.10.2026: frisch gebaute Raketen fehlten in der Auswahl bis zum Neuladen der Seite).
 if(raketenArten())missileTargetClasses();
 const stockHtml=`<p><strong>${fmt(p.raketensilo.belegt)} / ${fmt(p.raketensilo.kapazitaet)} Siloplätze belegt</strong> · Integrität ${fmt(p.raketensilo.integritaet/10)} %</p>${window.Mobile035.catalogToolbar('missiles')}<div class="cards">${p.raketensilo.typen.slice().sort((a,b)=>a.regel.zivilisation-b.regel.zivilisation||a.regel.silo-b.regel.silo).map((t,index,all)=>{const r=t.regel;return `${index===0||all[index-1].regel.zivilisation!==r.zivilisation?window.Mobile035.eraHeading(r.zivilisation):''}<article class="fleet-card" data-catalog-item="${r.key}" data-era="${r.zivilisation}" data-ready="${!t.sperre}" data-owned="${t.bestand>0}" data-running="${p.raketensilo.warteschlange.some(j=>j.art===r.key)}" data-search="${esc(r.name.toLocaleLowerCase('de'))}">${artShell('missiles',r.key)}<h3>${esc(r.name)} · ${fmt(t.bestand)} bereit</h3><button type="button" class="missile-info-button" data-missile-info="${r.key}" aria-label="${esc(r.name)}: Zielbilder und Abfangschutz">Zielbilder & Abfangschutz ↗</button><p>${esc(r.besonderheit)}</p><dl>${factRow('Primärziel',r.zielprofil.primaer)}${factRow('Sekundärziel',r.zielprofil.sekundaer)}${factRow('Maximale Ziele je Rakete',r.zielprofil.ziele_je_rakete,r.zielprofil.verteilung)}${factRow('Zielboni',Object.entries(r.zielprofil.schadensfaktoren).filter(([,n])=>n>1).map(([k,n])=>named(k)+' ×'+fmt(n)).join(' · ')||'Keine','Alle übrigen zulässigen Zielklassen: ×1 Grundschaden')}${factRow('Lagerplätze pro Stück',r.plaetze)}${factRow('Fertigungsplätze pro Stück',t.fertigungsplaetze_je_stueck)}${factRow('Bauzeit bei aktueller Leistung',GameTime.duration(t.dauer_aktuell_sekunden??r.bauzeit_sekunden))}${factRow('Basisschaden',r.schaden?fmt(r.schaden):'Automatische Abwehr')}${factRow('Flugzeit',r.schaden?GameTime.duration(r.flug_min_sekunden)+' + '+r.flug_je_system_sekunden+' s/System':'Automatische Abwehr')}${factRow('Reichweite',r.reichweite_je_silo?r.reichweite_je_silo+' Systeme je Silostufe':'Eigener Planet')}${factRow('Voraussetzungen','Silo '+r.silo+' · Zivilisation '+r.zivilisation+' · '+Object.entries(r.braucht).map(([k,n])=>named(k)+' '+n).join(', '))}</dl>${costHtml(t.kosten_aktuell||r.kosten,p.bestand)}${t.sperre?`<p class="muted">${esc(t.sperre)}</p>`:''}<form data-missile-build="${r.key}"><label>Menge<input name="anzahl" type="number" min="1" max="1000000" value="1" required></label><button type="submit" ${t.sperre?'disabled':''}>Im Silo bauen</button><small class="muted" data-missile-summe="${r.key}">${esc(raketenSumme(p,r.key,1))}</small></form>${window.Mobile035.seriesForm('rakete',r.key,p)}</article>`;}).join('')}</div>${window.Mobile035.localProduction('raketensilo',p)}<h3>Siloproduktion · ${p.raketensilo.fertigung.belegt} / ${p.raketensilo.fertigung.plaetze} Fertigungsplätze belegt</h3>${p.raketensilo.warteschlange.slice().sort((a,b)=>a.fertig-b.fertig).map(j=>`<p>${fmt(j.anzahl)} × ${esc(named(j.art))} · ${(j.lose||[]).reduce((n,b)=>n+b.anzahl,0)} gleichzeitig · nächste Fertigstellung bei Spielzeit ${esc(GameTime.duration(j.fertig))}${j.stornierbar?` <button type="button" data-silo-cancel="${j.auftrag}" data-planet="${esc(p.koord)}">Baucharge abbrechen · ${j.erstattbare_stueck??(j.erstattung_prozent?j.anzahl:0)} von ${j.anzahl} Stück erstatten</button>`:''}</p>`).join('')||'<p>Keine Raketen im Bau.</p>'}`;
 // Nur ein Eingabefeld schützt vor dem Neuzeichnen (ein angetippter Abbrechen-Knopf hielt die stornierte Charge sichtbar).
 const stockNode=$('missile-stock'),fokus=document.activeElement;if(!(fokus?.closest('#missile-stock')&&/^(INPUT|SELECT|TEXTAREA)$/.test(fokus.tagName))&&stockNode._sig!==stockHtml){stockNode._sig=stockHtml;setStableHTML(stockNode, stockHtml);}
 window.Mobile035.catalogFilter(stockNode,'missiles');
 const f=$('missile-launch').elements,selected=f.flotte.value;
 if(document.activeElement!==f.flotte){setStableHTML(f.flotte, '<option value="">Planet</option>'+(view.angriffe||[]).map(a=>`<option value="${a.flotte}">Anflug ${a.flotte} → ${esc(a.ziel)}</option>`).join(''));if([...f.flotte.options].some(o=>o.value===selected))f.flotte.value=selected;else if(selected)missileTargetClasses();}
 setStableHTML($('missile-salvos'), '<h3>Laufende Raketenschläge</h3>'+(view.taktische_raketensalven||[]).map(s=>`<p>${fmt(s.anzahl)} × ${esc(named(s.art))} → ${esc(s.ziel)}${s.flotte!==null?' · Flotte '+s.flotte:''} · Einschlag bei Spielzeit ${esc(GameTime.duration(s.ankunft))}</p>`).join(''));
}
// Raketen gehören zur Planetenverteidigung: das Silo steht als eigener Abschnitt unter "Verteidigung".
$('verteidigung').insertAdjacentHTML('beforeend','<div class="missile-section" id="missile-panel"></div>');
document.addEventListener('change',e=>{if(e.target.closest('#missile-launch'))missileTargetClasses();});
document.addEventListener('submit',guard(async e=>{
 if(e.target.dataset.missileBuild){e.preventDefault();await command([{typ:'raketen_bauen',planet:currentPlanet().koord,art:e.target.dataset.missileBuild,anzahl:Number(e.target.elements.anzahl.value)}]);}
 if(e.target.id==='missile-launch'){e.preventDefault();await command([missileDraft()]);}
}));
async function previewMissiles(){
 const identity=planIdentity(),a=missileDraft();const q=await api('/api/tool',{typ:'raketenplan',aktion:a});
 if(identity!==planIdentity()||JSON.stringify(a)!==JSON.stringify(missileDraft()))return;
 $('missile-quote').textContent=q.status==='ready'?q.bericht+' '+q.hinweis:q.grund;
}
document.addEventListener('click',guard(async e=>{if(e.target.closest('#missile-preview'))await previewMissiles();const cancel=e.target.closest('[data-silo-cancel]');if(cancel)await command([{typ:'raketen_storno',planet:cancel.dataset.planet,auftrag:Number(cancel.dataset.siloCancel)}]);}));

// Academy controls stay beside the building and use the selected planet's core data.
// Warum bezahlte Ausbildung pausiert (Kern verwaltung.rs: Akademie ruht bei Energie- oder Personalmangel); die kostenlose
// Grundausbildung läuft dabei anteilig weiter – beides gleichzeitig zu lesen, war widersprüchlich (Funktionsprüfung 25.10.2026).
function akademieRuht(p){
 const w=p.industrie?.werke?.find(w=>w.gebaeude==='akademie'),z=v=>fmt(Math.round((v||0)/1000));
 const teile=(w?.engpaesse||[]).map(e=>e==='arbeiter'?`Arbeiter ${z(w.arbeiter_zugewiesen)} von ${z(w.arbeiter_bedarf)}`:e==='fachkraefte'?`Fachkräfte ${z(w.fachkraefte_zugewiesen)} von ${z(w.fachkraefte_bedarf)}`:e==='integritaet'?'beschädigt':'');
 if((p.energie?.erzeugung??0)<(p.energie?.verbrauch??0))teile.unshift('Energie fehlt');
 const grund=teile.filter(Boolean).join(', ');
 return `Bezahlte Ausbildung pausiert: Akademie nicht voll besetzt${w?` (${fmt(Math.round(w.wirksame_besetzung_promille/10))} %)`:''}${grund?' – '+grund:''}.${p.akademie?.grundausbildung?.je_tag>0?' Die kostenlose Grundausbildung läuft anteilig weiter.':''}`;
}
function academyTraining(p){
 const a=p.akademie;if(!a)return '';
 const candidates=a.kandidaten.filter(v=>v.frei>0),preferred=candidates.some(v=>v.volk===view.volk)?view.volk:candidates[0]?.volk;
 return `<section class="academy-training"><h3>Ausbildung · ${esc(p.koord)}</h3><p>${fmt(a.frei)} / ${fmt(a.plaetze)} Plätze frei · ${fmt(p.fachkraefte.verfuegbar)} Fachkräfte im Einsatz${(a.grundausbildung?.fachkraefte||0)>p.fachkraefte.verfuegbar?` (von ${fmt(a.grundausbildung.fachkraefte)} ausgebildeten – nicht alle arbeiten gleichzeitig)`:''} / ${fmt(p.fachkraefte.bedarf)} benötigt${p.forscher?` · Forscher im Labor ${fmt(p.forscher.zugewiesen)} / ${fmt(p.forscher.bedarf)} Laborbedarf`:''}</p>${a.grundausbildung?`<div class="lage-grundausbildung"><b>Grundausbildung (kostenlos)</b><p>${esc(a.grundausbildung.hinweis)}</p><p class="muted">Ausgebildet / Ziel der Grundausbildung: Fachkräfte ${fmt(a.grundausbildung.fachkraefte)}${a.grundausbildung.ziel_fachkraefte>0?' / '+fmt(a.grundausbildung.ziel_fachkraefte):' (Bedarf gedeckt)'} · Forscher ${fmt(a.grundausbildung.forscher)}${a.grundausbildung.ziel_forscher>0?' / '+fmt(a.grundausbildung.ziel_forscher):' (Bedarf gedeckt)'} · ${fmt(a.grundausbildung.je_tag)} je Spieltag</p></div>`:''}${p.gebaeude.akademie?`<form data-academy-form data-planet="${esc(p.koord)}"><div class="academy-fields"><label>Herkunft<select name="volk">${candidates.map(v=>`<option value="${esc(v.volk)}" ${v.volk===preferred?'selected':''}>${esc(named(v.volk))} · ${fmt(v.frei)} verfügbar</option>`).join('')}</select></label><label>Ausbilden zu<select name="qualifikation"><option value="1">Fachkraft</option><option value="2" ${a.forscher_erlaubt?'':'disabled'}>Forscher${a.forscher_erlaubt?'':' · Labor fehlt'}</option></select></label><label>Personen<input name="anzahl" type="number" inputmode="numeric" min="1" max="${Math.min(1000,a.frei)}" value="1"></label></div><div data-school-quote></div><button type="submit">Ausbildung beginnen</button></form>`:'<p>Akademie Stufe 1 bauen, um Einwohner auszubilden.</p>'}${a.ruht&&p.gebaeude.akademie>0?`<p class="warning">${esc(akademieRuht(p))}</p>`:''}<p class="academy-note">24 Spielstunden je Fachkraft, 48 je Forscher bei 100 % Leistung. Alle belegten Plätze arbeiten gleichzeitig. Ausgebildete Einwohner wechseln ihre Qualifikation und werden automatisch den Betrieben zugewiesen.</p>${a.auftraege.length?`<ul class="academy-jobs">${a.auftraege.map(j=>`<li><b>${fmt(j.anzahl)} ${j.qualifikation===1?'Fachkräfte':'Forscher'}</b> · ${esc(named(j.volk))}<small>${a.ruht?'Pausiert':GameTime.duration(Math.max(0,j.fertig-GameTime.now()))+' Spielzeit bis Abschluss'}</small></li>`).join('')}</ul>`:''}</section>`;
}
function refreshAcademyQuote(form){
 const p=view.planeten.find(p=>p.koord===form.dataset.planet),a=p?.akademie;if(!a)return;
 const n=Number(form.elements.anzahl.value),q=Number(form.elements.qualifikation.value),origin=a.kandidaten.find(v=>v.volk===form.elements.volk.value),speed=a.tempo;
 const factor=speed>100?(100+(speed-100)*2)/speed:1;
 // Kosten je Person aus dem Kern (Funktionsprüfung 25.10.2026: fest 5 Elektronik je Fachkraft, der Kern bucht 0).
 const je=a.kosten_je_person?.[q===2?'forscher':'fachkraft']||{konsumgut:10,elektronik:q*5};
 const costs=Object.fromEntries(Object.entries(je).filter(([,v])=>v>0).map(([g,v])=>[g,Math.ceil(n*v*1000*factor)/1000]));
 const limit=Math.min(1000,a.frei,origin?.frei||0);form.elements.anzahl.max=limit;
 const invalid=!Number.isInteger(n)||n<1||n>limit,fehlt=Object.entries(costs).filter(([g,v])=>(p.bestand[g]||0)<v);
 const warum=!Number.isInteger(n)||n<1?'Mindestens 1 Person eingeben.':n>a.frei?`Nur ${fmt(a.frei)} Akademieplätze frei.`:n>(origin?.frei||0)?`Nur ${fmt(origin?.frei||0)} ${named(form.elements.volk.value)} ohne Ausbildung verfügbar.`:n>1000?'Höchstens 1.000 je Auftrag.':fehlt.length?'Rohstoffe fehlen: '+fehlt.map(([g,v])=>`${named(g)} ${fmt(Math.ceil(v-(p.bestand[g]||0)))}`).join(', '):'';
 setStableHTML(form.querySelector('[data-school-quote]'),(Object.keys(costs).length?tileCostMarkup(costs,p.bestand):'<small>kostenlos</small>')+`<small>${GameTime.duration(q*86400*100/speed)} Spielzeit${a.ruht?' · startet, sobald die Akademie voll besetzt ist':''}</small>${warum?`<small class="warning">${esc(warum)}</small>`:''}`);
 form.querySelector('[type=submit]').disabled=invalid||!!gameReadOnly()||fehlt.length>0;
}
document.addEventListener('click',guard(async e=>{if(!e.target.closest('[data-open-academy]'))return;showItemInfo('buildings','akademie');$('item-info').querySelector('.academy-training')?.scrollIntoView({block:'start'});}));
document.addEventListener('input',e=>{const f=e.target.closest('[data-academy-form]');if(f)refreshAcademyQuote(f);});
document.addEventListener('submit',guard(async e=>{
 const f=e.target.closest('[data-academy-form]');if(!f)return;e.preventDefault();
 await command([{typ:'ausbilden',planet:f.dataset.planet,volk:f.elements.volk.value,qualifikation:Number(f.elements.qualifikation.value),anzahl:Number(f.elements.anzahl.value)}]);
 showItemInfo('buildings','akademie');
}));
