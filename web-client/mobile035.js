'use strict';
// Rechenweg der Produktion (planer/wirtschaft.rs Spur): Kennungen des Kerns lesbar statt roh – „wirksame_besetzung_inkl_integritaet“
// lief als ein Wort in die Nachbarspalte (Überlappungsprüfung 25.10.2026).
const FAKTOREN={deuteriumvorrat:'Deuteriumvorrat',dyson_einspeisung:'Dyson-Einspeisung',dyson_sonnenfaktor:'Dyson-Sonnenfaktor',dyson_strombad:'Dyson-Strombad',
 eigener_volkswert:'Eigener Volkswert',energiedeckung:'Energiedeckung',energietechnik_und_volk:'Energietechnik und Volk',fusion:'Fusion',
 gemeinsame_eingangsdeckung:'Gemeinsame Eingangsdeckung',panzerung_und_volk:'Panzerung und Volk',planet:'Planet',planet_solar:'Planet (Solar)',
 plasmatechnik:'Plasmatechnik',produktionsausbau:'Produktionsausbau',produktionsleistung:'Produktionsleistung',produktionsregler:'Produktionsregler',
 raumhafen_mit_ersparnisobergrenze:'Raumhafen (mit Ersparnisobergrenze)',schildtechnik:'Schildtechnik',solar:'Solar',solarsonden:'Solarsonden',
 stabilitaet:'Stabilität',synthese_statt_vorkommen:'Synthese statt Vorkommen',syntheseeingangsdeckung:'Syntheseeingangsdeckung',
 volk_und_logistik:'Volk und Logistik',volk_werftzeit:'Volk (Werftzeit)',vor_zivilisationsstufe_iii:'Vor Zivilisationsstufe III',
 waffentechnik_und_volk:'Waffentechnik und Volk',wirksame_besetzung_inkl_integritaet:'Wirksame Besetzung inkl. Integrität',xenomaterialkunde:'Xenomaterialkunde'};
const OPERATION={faktor:'Faktor',promille:'Promille'};
function faktorName(k){k=String(k??'');return FAKTOREN[k]||(typeof named==='function'&&named(k)!==k?named(k):k.replace(/_/g,' ').replace(/^./,c=>c.toUpperCase()));}

// Numbers originate in the same core calculation that settles the planet's stock.
window.Mobile035=(()=>{
 const catalogFilters={};
 const buildingUses={erzmine:'erz',kristallmine:'kristall',deuteriumsynthesizer:'deuterium',farm:'nahrung',solarkraftwerk:'energie',fusionskraftwerk:'energie',versorgungsnetz:'energie',giesserei:'legierung',elektronikwerk:'elektronik',konsumgueterwerk:'konsumgut',xenoextraktor:'xenokristall',wohnblock:'bevoelkerung',akademie:'bevoelkerung',labor:'forschung',forschungsarchiv:'forschung',werft:'fertigung',orbitalwerft:'fertigung',kaserne:'fertigung',raketensilo:'fertigung',lager:'logistik',bunker:'logistik',raumhafen:'logistik',markt:'logistik',bauhof:'ausbau',nanofabrik:'ausbau',orbitalring:'ausbau',verwaltungszentrum:'verwaltung',geheimdienst:'aufklaerung',sensorphalanx:'aufklaerung'};
 const filterDefault=category=>({text:'',status:category==='buildings'?'current':'all',era:'',use:''});
 function buildingToolbar(f){
  const purposes=[['','Alle Aufgaben'],['erz','Erz'],['kristall','Kristall'],['deuterium','Deuterium'],['nahrung','Nahrung'],['energie','Energie'],['legierung','Legierung'],['elektronik','Elektronik'],['konsumgut','Konsumgüter'],['xenokristall','Xenokristalle'],['bevoelkerung','Bevölkerung & Ausbildung'],['forschung','Forschung'],['fertigung','Schiffe, Truppen & Raketen'],['logistik','Lager & Handel'],['ausbau','Bau & Ausbau'],['verwaltung','Verwaltung'],['aufklaerung','Aufklärung']];
  return `<div class="catalog-filters building-filters" data-catalog-filter="buildings"><div class="building-filter-heading"><strong>Gebäude finden</strong><button type="button" data-filter-reset>Zurücksetzen</button></div><label class="building-search"><span class="visually-hidden">Gebäudename suchen</span><input type="search" data-filter-text value="${esc(f.text)}" placeholder="Gebäude suchen …" aria-label="Gebäude suchen"></label><div class="building-status-pills" aria-label="Gebäude filtern">${[['current','Dein Zeitalter'],['affordable','Jetzt baubar'],['owned','Vorhanden'],['running','Im Bau'],['later','Später'],['all','Alle']].map(([v,t])=>`<button type="button" data-filter-pick="${v}" aria-pressed="${f.status===v}">${t} <span data-filter-total="${v}"></span></button>`).join('')}</div><div class="building-filter-selects"><label>Rohstoff / Aufgabe<select data-filter-use>${purposes.map(([k,n])=>`<option value="${k}" ${f.use===k?'selected':''}>${n}</option>`).join('')}</select></label><label>Zeitalter<select data-filter-era><option value="">Alle</option>${[1,2,3,4,5].map(n=>`<option value="${n}" ${String(n)===f.era?'selected':''}>Zeitalter ${eraName(n)}</option>`).join('')}</select></label></div><div class="building-filter-result"><output data-filter-count aria-live="polite"></output><span>Planet ${esc(currentPlanet().koord)}</span></div></div>`;
 }

 const eraName=n=>['','I','II','III','IV','V'][n]||String(n);
 const shipClasses=['sonde','jaeger','kreuzer','schlachter','bomber','zerstoerer','transporter','truppen','kolonie','bergbau','bergung','expedition','sonstige'];
 const shipClassNames={sonde:'Aufklärung',jaeger:'Jäger',kreuzer:'Kreuzer',schlachter:'Schlachtschiffe',bomber:'Bomber',zerstoerer:'Zerstörer',transporter:'Transport & Handel',truppen:'Truppentransport',kolonie:'Kolonisation',bergbau:'Bergbau',bergung:'Recycling & Bergung',expedition:'Expedition',sonstige:'Spezialschiffe'};
 function shipClass(key){const c=rules.catalog.einheiten[key]?.taktik?.klasse;return shipClasses.includes(c)?c:({pathfinder:'expedition',bergungsschiff:'bergung'}[key]||'sonstige');}
 function catalogOrder(items,category){return items.slice().sort((a,b)=>(category==='ships'?shipClasses.indexOf(shipClass(a.key))-shipClasses.indexOf(shipClass(b.key)):0)||(a.r.ab_stufe||1)-(b.r.ab_stufe||1)||Number(!!a.needs.length)-Number(!!b.needs.length)||named(a.key).localeCompare(named(b.key),'de'));}
 function shipGallery(items,cards){
  const group=locked=>shipClasses.map(k=>{const found=items.map((x,i)=>({x,i})).filter(({x})=>shipClass(x.key)===k&&!!x.needs.length===locked);return found.length?`<section class="ship-class-group" data-ship-class="${k}"><h3 class="ship-class-heading">${shipClassNames[k]}</h3>${found.map(({i})=>cards[i]).join('')}</section>`:'';}).join('');
  const ready=items.filter(x=>!x.needs.length).length,locked=items.length-ready;
  return `${ready?group(false):'<p class="shipyard-empty">Noch keine Schiffsklasse freigeschaltet. Unter „Voraussetzungen fehlen“ siehst du den nächsten Ausbauschritt.</p>'}${locked?`<details class="shipyard-locked" id="shipyard-locked"><summary>Voraussetzungen fehlen <span data-ship-locked-count>${locked}</span><small>Zeitalter, Werft und Forschung ansehen</small></summary><div class="gallery">${group(true)}</div></details>`:''}`;
 }
 function eraHeading(n){return `<h3 class="catalog-era" data-era-heading="${n}" data-era="${n}">Zivilisationszeitalter ${eraName(n)} <small>${n>view.stufe?'Spätere Freischaltungen':'Erreicht'}</small></h3>`;}
 function catalogToolbar(category){
  const f=catalogFilters[category]||filterDefault(category);
  if(category==='buildings')return buildingToolbar(f);
  return `<div class="catalog-filters" data-catalog-filter="${category}"><label>Suchen<input type="search" data-filter-text value="${esc(f.text)}" placeholder="Name eingeben" aria-label="Katalog durchsuchen"></label><label>Anzeige<select data-filter-status>${[['all','Alle Einträge'],['ready','Voraussetzungen erfüllt'],['owned','Bereits vorhanden'],['running','In Produktion'],['later','Späteres Zeitalter']].map(([v,t])=>`<option value="${v}" ${v===f.status?'selected':''}>${t}</option>`).join('')}</select></label><label>Zeitalter<select data-filter-era><option value="">Alle Zeitalter</option>${[1,2,3,4,5].map(n=>`<option value="${n}" ${String(n)===f.era?'selected':''}>${eraName(n)}</option>`).join('')}</select></label><output data-filter-count aria-live="polite"></output></div>`;
 }
 function catalogFilter(host,category){
  const f=catalogFilters[category]||filterDefault(category);let count=0;
  const cards=[...host.querySelectorAll('[data-catalog-item]')];
  const statusMatch=(d,status)=>status==='all'||status==='current'&&Number(d.era)<=view.stufe||status==='affordable'&&d.affordable==='true'||status==='ready'&&d.ready==='true'||status==='owned'&&d.owned==='true'||status==='running'&&d.running==='true'||status==='later'&&Number(d.era)>view.stufe;
  for(const c of cards){const d=c.dataset,match=(!f.text||(d.search||'').includes(f.text.toLocaleLowerCase('de')))&&(!f.era||d.era===f.era)&&(!f.use||buildingUses[d.catalogItem]===f.use)&&statusMatch(d,f.status);c.hidden=!match;if(match)count++;}
  for(const n of host.querySelectorAll('[data-filter-total]'))n.textContent=cards.filter(c=>statusMatch(c.dataset,n.dataset.filterTotal)).length;
  for(const b of host.querySelectorAll('[data-filter-pick]'))b.setAttribute('aria-pressed',String(f.status===b.dataset.filterPick));

  for(const h of host.querySelectorAll('[data-era-heading]'))h.hidden=!cards.some(c=>!c.hidden&&c.dataset.era===h.dataset.eraHeading);
  for(const g of host.querySelectorAll('[data-ship-class]'))g.hidden=![...g.querySelectorAll('[data-catalog-item]')].some(c=>!c.hidden);
  const locked=host.querySelector('.shipyard-locked');if(locked){const n=[...locked.querySelectorAll('[data-catalog-item]')].filter(c=>!c.hidden).length;locked.hidden=!n;locked.querySelector('[data-ship-locked-count]').textContent=n;}
  const out=host.querySelector('[data-filter-count]');if(out)out.textContent=`${count} / ${cards.length} Einträge${count?'':' · Filter ändern, um weitere Einträge zu sehen'}`;
 }
 function catalogFilterEvent(e){const bar=e.target.closest('[data-catalog-filter]');if(!bar)return;const category=bar.dataset.catalogFilter,previous=catalogFilters[category]||filterDefault(category);catalogFilters[category]={text:bar.querySelector('[data-filter-text]').value,status:bar.querySelector('[data-filter-status]')?.value||previous.status,era:bar.querySelector('[data-filter-era]').value,use:bar.querySelector('[data-filter-use]')?.value||''};if(category==='buildings'&&e.target.matches('[data-filter-era]')&&e.target.value)catalogFilters[category].status='all';catalogFilter(bar.parentElement,category);}
 document.addEventListener('input',catalogFilterEvent);document.addEventListener('change',catalogFilterEvent);
 document.addEventListener('click',e=>{const b=e.target.closest('[data-filter-pick],[data-filter-reset]');if(!b)return;const bar=b.closest('[data-catalog-filter]'),category=bar.dataset.catalogFilter;
  if(b.hasAttribute('data-filter-reset')){catalogFilters[category]=filterDefault(category);bar.querySelector('[data-filter-text]').value='';bar.querySelector('[data-filter-era]').value='';if(bar.querySelector('[data-filter-use]'))bar.querySelector('[data-filter-use]').value='';}
  else{catalogFilters[category]={...(catalogFilters[category]||filterDefault(category)),status:b.dataset.filterPick,era:''};bar.querySelector('[data-filter-era]').value='';}
  catalogFilter(bar.parentElement,category);
 });
 document.addEventListener('DOMContentLoaded',()=>{
  const form=$('fleet');form.insertAdjacentHTML('afterbegin','<output id="fleet-live-summary" class="fleet-live-summary" aria-live="polite">Schiffe auswählen: Reisezeit und Treibstoff werden laufend berechnet.</output>');
  for(const field of [...form.querySelectorAll(':scope>fieldset')]){const details=document.createElement('details');details.className='fleet-selection';details.open=!!field.querySelector('#ship-inputs');const summary=document.createElement('summary');summary.textContent=field.querySelector('legend').textContent+' · Auswahl ein-/ausklappen';details.append(summary);field.before(details);details.append(field);}
 });
 function localProduction(key,p=currentPlanet()){
  const n=p?.industrie?.regler?.[key],w=p?.industrie?.werke?.find(w=>w.gebaeude===key);if(n===undefined||!w?.stufe)return '';
  return `<aside class="local-production"><strong>${esc(named(key))} · ${esc(p.koord)}</strong><label>Leistung <output>${n} %</output><input type="range" min="0" max="120" step="10" value="${n}" data-production="${key}" data-production-planet="${esc(p.koord)}" aria-label="${esc(named(key))}: Leistung"></label><small>${fmt(w.energiebedarf/1000)} Energie gebunden · ${fmt(w.arbeiter_zugewiesen/1000)} Arbeitskräfte · ${fmt(w.fachkraefte_zugewiesen/1000)} ${key==='labor'?'Forscher':'Fachkräfte'}. 0 % pausiert und gibt den Produktionsbedarf frei. Überlast: 110 % Leistung / 120 % Bedarf, 120 % / 140 %.</small></aside>`;
 }
 function seriesForm(art,key,p=currentPlanet()){
  const s=p.produktionsserien?.[art+':'+key],defaultOrbital=art==='bauteil'&&p.gebaeude.orbitalwerft>0;
  return `<details class="production-series"><summary>Produktionsschleife · ${s?.ziel>0?s.ziel+' Stück Vorratsziel':defaultOrbital&&!s?'automatisch bis 10 Stück':'aus'}</summary><p>Füllt den lokalen Bestand nach Verbrauch oder Abflug wieder auf. Nutzt freie Fertigungsplätze gleichzeitig; vorhandene Aufträge zuerst. Material${art==='boden'?' und Rekruten':art==='einheit'?' und Besatzung':''} werden bezahlt, sobald die Schleife einen Auftrag erteilt – dann für das ganze Los, nicht schon beim Setzen des Ziels. ${art==='rakete'?'Belegte und reservierte Siloplätze begrenzen jeden Auftrag. ':''}${art==='bauteil'&&key==='habitatmodul'&&!defaultOrbital?'Jedes Modul verbraucht zusätzlich einen vorhandenen Recycler. ':''}Bei fehlenden Voraussetzungen wartet die Schleife. Ausschalten beendet nur künftige Wiederholungen; bezahlte Aufträge separat abbrechen.</p><form data-series-form="${art}" data-product="${key}" data-planet="${esc(p.koord)}"><label>Vorratsziel · 0 = aus<input name="ziel" type="number" min="0" max="10000" step="1" value="${s?.ziel??(defaultOrbital?10:0)}" required></label><button type="submit">Vorratsziel speichern</button></form><p role="status">${esc(s?.status||(defaultOrbital?'Automatische Bauteilversorgung aktiv':'Keine Wiederholung eingerichtet'))}</p></details>`;
 }
 document.addEventListener('submit',guard(async e=>{const f=e.target;if(!f.dataset.seriesForm)return;await command([{typ:'produktionsserie',planet:f.dataset.planet,art:f.dataset.seriesForm,produkt:f.dataset.product,ziel:Number(f.elements.ziel.value)}]);}));
 const groups=[['Übersicht',['reich','kolonie','imperium','auftraege']],['Bauen',['gebaeude','werft','verteidigung','core-planner']],['Forschung',['forschen','freischaltungen','codex']],['Flottenmanagement',['flotten']],['Galaxie',['karte','kolonien']],['Kommunikation',['briefkasten','allianzbereich','allianzpost','berichte','aktionen','pinwaende']],['Profil & Einstellungen',['profil','agent','regeln']]];
 function navigation(){
  const nav=$('tabs'),buttons=[...nav.querySelectorAll('[data-tab]')];
  nav.querySelectorAll('.nav-group').forEach(n=>n.remove());
  const container=document.createElement('div');container.id='main-navigation';nav.append(container);
  groups.forEach(([name,ids],i)=>{
   const box=document.createElement('div');box.className='navigation-group';
   setStableHTML(box, `<button type="button" data-nav-group="${i}" aria-expanded="false" aria-controls="nav-sub-${i}"><span class="navigation-label">${name}</span>${uiIcon('down')}</button><div class="navigation-children" id="nav-sub-${i}" hidden></div>`);
   for(const b of buttons.filter(b=>ids.includes(b.dataset.tab)))box.lastChild.append(b);
   container.append(box);
  });
  // Preserve future/optional areas instead of silently losing their controls.
  for(const b of buttons.filter(b=>!b.closest('#main-navigation')))container.lastChild.lastChild.append(b);
  nav.classList.add('grouped-navigation');syncNavigation();
 }
 function syncNavigation(){
  const id=document.querySelector('#tabs [data-tab][aria-selected="true"]')?.dataset.tab||'reich';
  const index=groups.findIndex(([,ids])=>ids.includes(id));
  for(const b of document.querySelectorAll('[data-nav-group]')){
   const active=Number(b.dataset.navGroup)===index;b.setAttribute('aria-expanded',String(active));b.classList.toggle('active',active);b.nextElementSibling.hidden=!active;
  }
 }
 function researchers(p){const w=p.industrie?.werke?.find(w=>w.gebaeude==='labor');return {zugewiesen:(w?.fachkraefte_zugewiesen||0)/1000,plaetze:(w?.fachkraefte_bedarf||0)/1000,fp_je_spielstunde:(p.industrie?.forschung_milli||0)/1000,gesamt:(p.personengruppen?.gruppen||[]).reduce((n,g)=>n+(g[2]||0),0)/1000};}
 function organic(p){return (p.personengruppen?.gruppen||[]).slice(0,3).some(g=>g.some(n=>n>0));}
 function workforce(p){
  const r=researchers(p),groups=p.personengruppen?.gruppen,training=(view.ausbildungen||[]).filter(a=>a.planet===p.id||a.koord===p.koord);
  return `<div class="management-heading"><h3>Personal · ${esc(p.koord)}</h3><small>Vorhanden / benötigt</small></div><div class="population-chain"><article><span>Bevölkerung</span><strong>${fmt(p.bevoelkerung)} / ${fmt(p.wohnraum)}</strong><small>Wohnraum · Wachstum abhängig von Herkunft, Klima, Stabilität und Versorgung</small></article><article><span>Arbeitskräfte</span><strong>${fmt(p.arbeit.verfuegbar)} / ${fmt(p.arbeit.bedarf)}</strong><small>Prioritäten bestimmen die Zuweisung</small></article><article><span>Fachkräfte</span><strong>${fmt(p.fachkraefte.verfuegbar)} / ${fmt(p.fachkraefte.bedarf)}</strong><small>Eigene Qualifikation; Forscher werden gesondert gezählt</small></article><article><span>Forscher im Labor</span><strong>${fmt(r.zugewiesen)} / ${fmt(r.plaetze)}</strong><small>${fmt(r.gesamt)} ausgebildet · ${fmt(r.fp_je_spielstunde)} FP / Spielstunde · ${fmt(r.fp_je_spielstunde*(world.tempo||1))} FP / Echtzeitstunde</small></article></div>${groups?`<details><summary>Herkunft und Qualifikation</summary>${table(['Herkunft','Einwohner','Fachkräfte','Forscher'],['aurelianer','krath','veyari','syntheten'].map((v,i)=>[esc(named(v)),...groups[i].map(n=>fmt(n/1000))]))}</details>`:''}<p>Ausbildung wird von dir beauftragt: Fachkräfte benötigen 24, Forscher 48 Spielstunden. Sie wechseln ihre Qualifikation; es entstehen keine zusätzlichen Einwohner. Akademieplätze, Material und für Forscher ein Labor sind erforderlich.</p><button type="button" data-open-academy>Fachkräfte & Forscher ausbilden</button>`;
 }
 // force: nach einem Reglerbefehl immer aus dem Kernstand zeichnen, auch mit Fokus im Feld (Nachprüfung 25.10.2026:
 // Kopf blieb auf 0 %, der zweite Pfeil sendete den alten Wert).
 function renderIndustry(force){
  const p=currentPlanet(),data=p?.industrie,host=$('kolonie-content');if(!host||!data)return;
  let panel=$('industry-controls');if(!force&&panel?.contains(document.activeElement))return;
  if(!panel){panel=document.createElement('section');panel.id='industry-controls';host.prepend(panel);}
  const goodsMap=a=>Object.fromEntries((data.gueter||[]).map((g,i)=>[g,(a?.[i]||0)/1000]));
  // Engpässe lesbar mit Zahlen; ab Stufe III läuft ein Werk auch knapp besetzt mit Mindestbesetzung (Kern planer/wirtschaft.rs
  // PERSONAL_SANFT_BODEN_PROMILLE) – sonst stand „0 Arbeitskräfte, 75 % Besetzung, Ausstoß 71“ unerklärt da (Funktionsprüfung 25.10.2026).
  const z=v=>fmt(Math.round((v||0)/100)/10);
  const engpassText=w=>w.engpaesse.map(e=>e==='arbeiter'?`Arbeiter fehlen (${z(w.arbeiter_zugewiesen)} von ${z(w.arbeiter_bedarf)})`:e==='fachkraefte'?`${w.gebaeude==='labor'?'Forscher':'Fachkräfte'} fehlen (${z(w.fachkraefte_zugewiesen)} von ${z(w.fachkraefte_bedarf)})`:e==='integritaet'?`beschädigt (${fmt(w.integritaet_promille/10)} %)`:e==='syntheserohstoffe'?'Syntheserohstoffe fehlen':named(e)).join(' · ')+((w.engpaesse.includes('arbeiter')||w.engpaesse.includes('fachkraefte'))&&w.besetzung_vor_integritaet_promille>0?` – läuft mit ${fmt(w.besetzung_vor_integritaet_promille/10)} % Besetzung (ab Stufe III mindestens 75 %)`:'');
  const card=w=>{
   const key=w.gebaeude,percent=data.regler[key],output=goodsMap(w.produktion),input=goodsMap(w.verbrauch),trace=w.ausstoss;
   return `<article class="production-unit"><header><h4>${esc(named(key))} <small>Stufe ${w.stufe}</small></h4><b>${percent??100} %</b></header>${percent!==undefined?`<label>Förderung / Leistung <output>${percent} %</output><input type="range" min="0" max="120" step="10" value="${percent}" data-production="${esc(key)}" data-production-planet="${esc(p.koord)}" aria-label="${esc(named(key))} auf ${esc(p.koord)}: Produktionsleistung"></label><p class="production-demand">${percent} % Sollausstoß · ${percent>100?100+(percent-100)*2:percent} % Grundbedarf</p>`:''}<dl><dt>Erzeugt pro Spielstunde</dt><dd>${esc(goods(output))}</dd><dt>Verbraucht pro Spielstunde</dt><dd>${esc(goods(input))}</dd>${['solarkraftwerk','fusionskraftwerk'].includes(key)?`<dt>Energieerzeugung vor Versorgungsnetz</dt><dd>${fmt(w.energie_vor_versorgungsnetz/1000)}</dd>`:''}<dt>Energiebedarf</dt><dd>${fmt(w.energiebedarf/1000)}</dd><dt>Arbeitskräfte</dt><dd>${fmt(w.arbeiter_zugewiesen/1000)} / ${fmt(w.arbeiter_bedarf/1000)}</dd><dt>Fachkräfte${key==='labor'?' (Forscher)':''}</dt><dd>${fmt(w.fachkraefte_zugewiesen/1000)} / ${fmt(w.fachkraefte_bedarf/1000)}</dd><dt>Besetzung / Zustand / Rohstoffdeckung</dt><dd>${fmt(w.wirksame_besetzung_promille/10)} % / ${fmt(w.integritaet_promille/10)} % / ${fmt(w.eingangsdeckung_promille/10)} %</dd></dl>${key==='orbitalwerft'?'<p>Automatisch Habitatmodule und Antriebskerne, bis je 10 Stück bereitliegen. Aktueller Stückfortschritt und bezahlte Materialkosten unter Schiffswerft.</p>':''}${key==='xenoextraktor'?`<p>${data.xeno_natuerlich?'Natürliches Vorkommen':'Synthese: 65 % des natürlichen Ausstoßes; je Xenokristall 12 Deuterium, 20 Kristall, 8 Legierung und 4 Elektronik; vierfacher Energiebedarf.'}</p>`:''}${percent===0?'<p class="muted">Pausiert: keine Produktion und kein Bedarf. Regler hochziehen, um wieder anzulaufen.</p>':''}${w.engpaesse?.length?`<p class="warning">${esc(engpassText(w))}</p>`:''}${trace?`<details><summary>Faktoren & Berechnung</summary><p>Grundwert: ${fmt(trace.grundwert/1000)} → ${fmt(trace.wert/1000)} ${esc(trace.einheit)}</p>${table(['Faktor','Operation','Parameter','Ergebnis'],trace.schritte.map(s=>[esc(faktorName(s.quelle)),esc(OPERATION[s.operation]||s.operation),fmt(s.parameter),fmt(s.nachher/1000)]))}</details>`:''}</article>`;
  };
  setStableHTML(panel, `<h3>Produktionssteuerung · ${esc(p.koord)}</h3><p>Alle Mengen gelten pro Spielstunde (${GameTime.duration(3600/(world.tempo||1))} Echtzeit). Verarbeitung läuft kontinuierlich: Ein-/Ausgangsmengen unten gehören zur gleichen Zeitspanne. Die Nettobilanz enthält zusätzlich Bevölkerung und Unterhalt. Bei Engpässen sinkt der Ausstoß.</p><div class="production-units">${data.werke.filter(w=>w.stufe>0&&data.regler[w.gebaeude]!==undefined).map(card).join('')}</div>${(()=>{const ohne=data.werke.filter(w=>w.stufe>0&&data.regler[w.gebaeude]===undefined);return ohne.length?`<details class="production-ohne"><summary>Gebäude ohne Leistungsregler · ${ohne.length}</summary><div class="production-units">${ohne.map(card).join('')}</div></details>`:'';})()}`);
 }
 function itemFacts(category,item){
  if(category==='research'){
   const gates=Object.entries(rules.catalog.einheiten||{}).filter(([,r])=>r.taktik?.voraussetzungen?.[item.key]);
   const missiles=(rules.catalog.raketen||[]).filter(r=>r.braucht?.[item.key]);
   return `<div class="inline-facts"><strong>Freischaltungen durch ${esc(named(item.key))}</strong>${gates.length?`<ul>${gates.map(([k,r])=>`<li>${esc(named(k))} ab Stufe ${r.taktik.voraussetzungen[item.key]}${Object.entries(r.taktik.voraussetzungen).filter(([f])=>f!==item.key).map(([f,n])=>' · '+esc(named(f))+' '+n).join('')} · Werft ${r.regel.werft}</li>`).join('')}</ul>`:'<p>Verbessert bestehende Fähigkeiten. Wirkung und Voraussetzungen stehen im Datenblatt.</p>'}${missiles.length?`<ul>${missiles.map(r=>`<li>${esc(r.name)} ab ${esc(named(item.key))} ${r.braucht[item.key]} · Silo ${r.silo} · Zivilisation ${r.zivilisation}</li>`).join('')}</ul>`:''}${dataSheet(category,item)}${localProduction('labor')}</div>`;
  }
  return `<div class="inline-facts">${visualDossier(category,item)}${item.key==='kaserne'?`<h4>Bodentruppen freischalten</h4><ul>${(rules.catalog.bodentruppen?.typen||[]).map(t=>`<li>${esc(t.name)} · Kaserne ${t.kaserne} · Zeitalter ${eraName(t.ab_stufe)}</li>`).join('')}</ul>`:''}${dataSheet(category,item)}${['ships','defenses'].includes(category)?`<p><b>${view.einheiten_kosten.find(x=>x.einheit===item.key)?.fertigungsplaetze_je_stueck||1} Fertigungsplätze je Einheit</b> · ${currentPlanet().fertigungsplaetze.werft.frei} von ${currentPlanet().fertigungsplaetze.werft.plaetze} Werftplätzen frei. Jeder Platz arbeitet gleichzeitig; schwere Einheiten belegen mehrere Plätze bis zur Fertigstellung.</p>`:''}${localProduction(category==='buildings'?item.key:'werft')}${['ships','defenses'].includes(category)?seriesForm('einheit',item.key):''}</div>`;
 }
 let historyOwner='',history={recent:[],pins:[]},previewTimer=0,previewSequence=0;
 function loadHistory(){const key=world.world_id+':'+view.name;if(historyOwner===key)return;historyOwner=key;try{const raw=JSON.parse(localStorage.getItem('sternenepoche-flights:'+key)||'{}');history={recent:(raw.recent||[]).slice(0,5),pins:(raw.pins||[]).slice(0,2)};}catch{history={recent:[],pins:[]};}}
 function saveHistory(){try{localStorage.setItem('sternenepoche-flights:'+historyOwner,JSON.stringify(history));}catch{}}
 // app.js ruft das nur nach erfolgreichem Start; das Ereignis lässt den Flottenassistenten zurücksetzen und bestätigen.
 function rememberFlight(a){loadHistory();history.recent=[JSON.parse(JSON.stringify(a)),...history.recent.filter(x=>JSON.stringify(x)!==JSON.stringify(a))].slice(0,5);saveHistory();renderHistory();document.dispatchEvent(new CustomEvent('stern:flotte-gestartet',{detail:a}));}
 function renderHistory(){if(!view)return;renderFlightSchedule();loadHistory();let host=$('flight-history');if(!host){host=document.createElement('div');host.id='flight-history';$('fleet').before(host);}const wasOpen=host.querySelector('details')?.open;setStableHTML(host, `<details ${wasOpen?'open':''}><summary>Flugvorlagen · ${history.recent.length} letzte Flüge · ${history.pins.length} / 2 angeheftet</summary>${['pins','recent'].map(k=>`<h4>${k==='pins'?'Angeheftet':'Zuletzt gestartet'}</h4>${history[k].map((a,i)=>`<article class="flight-template"><button type="button" data-flight-template="${k}:${i}"><b>${esc(named(a.mission))} · ${esc(a.start)} → ${esc(a.ziel)}</b><small>${esc(goods(a.schiffe))}</small></button><button type="button" data-flight-pin="${k}:${i}" ${k==='recent'&&history.pins.length===2?'disabled':''}>${k==='pins'?'Lösen':'Anheften'}</button></article>`).join('')||'<p>Noch keine Vorlagen.</p>'}`).join('')}<p>Eine Vorlage füllt die Planung. Der Start benötigt deine gesonderte Aktion und wird vom Core erneut geprüft.</p></details>`);}
 function coordinatedFlight(flug){
  if(window.Flottenladung){const base=flug.typ==='boden_senden'?flug.flug:flug;const truppen=flug.typ==='boden_senden'?flug.truppen:Array(12).fill(0);if(base.typ==='flotte_senden')flug=Flottenladung.aktion(base,truppen);}
  const f=$('fleet').elements,mode=f.termin_modus?.value||'now';let ankunft=null;
  if(mode==='arrival'){const day=Number(f.ankunft_tag.value),parts=f.ankunft_uhrzeit.value.split(':').map(Number);if(day<1||parts.length<2)throw Error('Ankunft als Spieltag und Uhrzeit wählen.');ankunft=(day-1)*86400+parts[0]*3600+parts[1]*60+(parts[2]||0);}
  return {typ:'flotte_planen',flug,ankunft,rueckgeschwindigkeit:Number(f.rueckgeschwindigkeit?.value||1)};
 }
 function clockStamp(t){return 'Tag '+(Math.floor(t/86400)+1)+' · '+[Math.floor(t/3600)%24,Math.floor(t/60)%60,Math.floor(t)%60].map(n=>String(n).padStart(2,'0')).join(':');}
 function coordinationMarkup(q){return `<div class="stats">${stat(clockStamp(q.abflug),'Abflug · Spielzeit')}${stat(clockStamp(q.ankunft),'Ankunft · Spielzeit')}${stat(GameTime.duration(q.hinflug),'Hinflug')}${stat(q.rueckkehr===null?'Einweg':GameTime.duration(q.rueckflug),'Rückflug')}${stat(fmt(q.treibstoff_hin),'Deuterium Hinflug')}${stat(fmt(q.treibstoff_rueck),'Deuterium Rückflug')}</div><p>${q.rueckkehr!==null?'Rückkehr '+clockStamp(q.rueckkehr)+' · '+esc(q.rueckkehr_annahme):'Die Mission benötigt keinen regulären Rückflug.'}</p><p class="${q.bereit&&q.zeit_gueltig?'good':'warning'}">${!q.zeit_gueltig?'Ankunft zu früh oder Abflug mehr als sieben Spieltage entfernt.':q.bereit?'Aktuelle Startbedingungen erfüllt.':esc(q.grund)}</p>`;}
 function renderFlightSchedule(){if(!view)return;let host=$('flight-schedule');if(!host){host=document.createElement('div');host.id='flight-schedule';$('fleets').before(host);}const rows=[...(view.flugtermine||[])].sort((a,b)=>a.ankunft-b.ankunft||a.id-b.id);setStableHTML(host, rows.length?`<h3>Geplante Ankünfte</h3>${rows.map((r,i)=>{const a=r.flug.flug||r.flug;return `<article class="production-unit"><b>${i+1}. ${esc(a.ziel)} · ${esc(window.Auftraege?.info?.(a.mission)?.name||named(a.mission))}</b><p>Abflug ${clockStamp(r.abflug)}<br>Ankunft ${clockStamp(r.ankunft)}${window.Auftraege?.info?.(a.mission)?.einweg?'':Number(r.ruecktempo)===0?'<br>kein Rückflug – bleibt am Ziel':`<br>Rückflug mit ${Math.round((Number(r.ruecktempo)||1)*100)} % Tempo`}</p><button type="button" data-flight-cancel="${r.id}">Plan entfernen</button></article>`;}).join('')}`:'');}
 document.addEventListener('click',guard(async e=>{const b=e.target.closest('[data-flight-cancel]');if(b){await command([{typ:'flugplan_storno',auftrag:Number(b.dataset.flightCancel)}]);renderFlightSchedule();}}));
 document.addEventListener('DOMContentLoaded',()=>{const f=$('fleet');const speed=f.elements.geschwindigkeit;setStableHTML(speed, Array.from({length:10},(_,i)=>10-i).map(n=>`<option value="${n/10}">${n*10} %</option>`).join(''));const host=document.createElement('details');host.className='flight-timing';host.open=true;setStableHTML(host, `<summary>Ankunft koordinieren &amp; Treibstoff sparen</summary><p>Alle Zeiten sind Spielzeit. Geringeres Tempo spart Deuterium auf der jeweiligen Strecke.</p><label>Rückflug · eigenes Tempo<select name="rueckgeschwindigkeit">${speed.innerHTML}</select></label><label>Startplanung<select name="termin_modus"><option value="now">Sofort starten</option><option value="arrival">Zu festgelegter Spielzeit eintreffen</option></select></label><div class="row"><label>Ankunft am Spieltag<input name="ankunft_tag" type="number" min="1" value="1"></label><label>Uhrzeit HH:MM:SS<input name="ankunft_uhrzeit" type="time" step="1" value="12:00:00"></label></div><p>Geplante Flüge werden auf dem Spielserver gespeichert, auch wenn dieses Menü geschlossen ist. Bis zum Abflug bleiben Schiffe und Vorräte verfügbar: fehlen sie dann, startet der Flug nicht und du erhältst einen Bericht. Bei pausierter oder beendeter lokaler Welt wartet auch der Abflug.</p>`);speed.closest('label').after(host);});
 async function preview(){
  const sequence=++previewSequence;let a;
  try{a=fleetData();}catch(e){$('flight-quote').textContent=e.message;return;}
  const owner=planIdentity(),host=$('flight-quote');host.hidden=false;
  if(!Object.values(a.schiffe).some(n=>n>0)){host.textContent='Schiffe auswählen, um Reisezeit, Fracht und Treibstoff zu berechnen.';if($('fleet-live-summary'))$('fleet-live-summary').textContent=host.textContent;return;}
  host.setAttribute('aria-busy','true');
  try{
   const troops=groundDraft();const request=coordinatedFlight(troops.some(n=>n>0)?{typ:'boden_senden',flug:a,truppen:troops}:a);const q=await api('/api/tool',{...request,typ:'flugkoordination'});
   if(sequence!==previewSequence||owner!==planIdentity()||JSON.stringify(a)!==JSON.stringify(fleetData()))return;
   merkePruefung(a,troops,q.bereit&&q.zeit_gueltig,q.grund||'Termin prüfen');
   setStableHTML(host, coordinationMarkup(q));if($('fleet-live-summary'))setStableHTML($('fleet-live-summary'), `<span><b>${GameTime.duration(q.hinflug)}</b>Hinflug</span><span><b>${q.rueckkehr===null?'Einweg':GameTime.duration(q.rueckflug)}</b>Rückflug</span><span><b>${fmt(q.treibstoff_hin+q.treibstoff_rueck)}</b>Deuterium</span><small>Ankunft ${clockStamp(q.ankunft)} · ${q.bereit&&q.zeit_gueltig?'Startbedingungen erfüllt':esc(q.grund||'Termin prüfen')}</small>`);
  }catch(e){if(sequence===previewSequence){host.textContent=e.message;merkePruefung(a,groundDraft(),false,e.message);}}finally{if(sequence===previewSequence)host.removeAttribute('aria-busy');}
 }
 // Ergebnis der Flugprüfung für den Startknopf (flottenfuehrung.js): gilt nur für genau diesen Flug. Vorher blieb
 // „Flotte starten“ aktiv, obwohl die Prüfung schon „nicht bereit“ mit Grund meldete (Galaxie-Durchgang, Punkt 8).
 let pruefung=null;
 const flugSchluessel=(a,t)=>JSON.stringify([a,t,window.Flottenladung?.personen(),...['rueckgeschwindigkeit','termin_modus','ankunft_tag','ankunft_uhrzeit'].map(k=>$('fleet').elements[k]?.value)]);
 function merkePruefung(a,truppen,bereit,grund){pruefung={schluessel:flugSchluessel(a,truppen),bereit:!!bereit,grund:String(grund||'')};document.dispatchEvent(new Event('stern:flugpruefung'));}
 function letztePruefung(){if(!pruefung)return null;try{return pruefung.schluessel===flugSchluessel(fleetData(),groundDraft())?pruefung:null;}catch{return null;}}
 function schedulePreview(){clearTimeout(previewTimer);previewSequence++;previewTimer=setTimeout(preview,220);}
 // Das Hauptmenü hat seit den Designregeln (17.10.) genau eine Ebene; rahmen.js baut es, navigation() bleibt ungenutzt.
 document.addEventListener('DOMContentLoaded',()=>{$('flight-quote').setAttribute('aria-live','polite');$('flight-preview').onclick=()=>preview();});
 document.addEventListener('game-navigation',()=>{syncNavigation();renderHistory();});
 // Leistungsregler: senden, sobald der angezeigte Wert vom Kernwert (value-Attribut) abweicht – bei change, beim Loslassen und
 // spätestens 600 ms nach der letzten Eingabe (Funktionsprüfung 25.10.2026: ein Tipp zeigte 120 %, es kam kein change, nichts
 // wurde gesendet). Nach Pfeiltasten bleibt der Fokus auf dem Regler, damit weitere Tasten wirken.
 const reglerTimer=new WeakMap();
 async function reglerSenden(input){
  clearTimeout(reglerTimer.get(input));if(!input?.dataset.production||input.disabled||!input.isConnected)return;const soll=Number(input.value);if(soll===Number(input.getAttribute('value')))return;
  const key=input.dataset.production,planet=input.dataset.productionPlanet,tastatur=input.dataset.tastatur==='1';input.disabled=true;
  try{await command([{typ:'produktion_setzen',planet,gebaeude:key,prozent:soll}]);}catch(err){show(err.message||String(err),true);}
  finally{input.disabled=false;if(document.activeElement===input)input.blur();renderIndustry(true);
   // Anzeige = Kern: nach Annahme steht der neue Wert im value-Attribut, nach Ablehnung springt der Regler zurück.
   const neu=document.querySelector(`[data-production="${CSS.escape(key)}"][data-production-planet="${CSS.escape(planet)}"]`);
   if(neu){const kern=neu.getAttribute('value');if(neu.value!==kern){neu.value=kern;if(neu.previousElementSibling)neu.previousElementSibling.value=kern+' %';}if(tastatur){neu.dataset.tastatur='1';neu.focus({preventScroll:true});}}}
 }
 document.addEventListener('input',e=>{if(e.target.dataset.production){const n=Number(e.target.value);e.target.previousElementSibling.value=n+' %';const demand=e.target.closest('.production-unit')?.querySelector('.production-demand');if(demand)demand.textContent=`${n} % Sollausstoß · ${n>100?100+(n-100)*2:n} % Grundbedarf`;clearTimeout(reglerTimer.get(e.target));reglerTimer.set(e.target,setTimeout(()=>reglerSenden(e.target),600));return;}if(e.target.closest('#fleet'))schedulePreview();});
 document.addEventListener('change',e=>{const input=e.target;if(input.dataset.production)reglerSenden(input);else if(input.closest('#fleet'))schedulePreview();});
 for(const art of ['pointerup','touchend'])document.addEventListener(art,e=>{const input=e.target.closest?.('[data-production]');if(input){input.dataset.tastatur='';setTimeout(()=>reglerSenden(input),0);}},true);
 document.addEventListener('keydown',e=>{if(e.target.dataset?.production)e.target.dataset.tastatur='1';},true);
 document.addEventListener('click',e=>{
  const b=e.target.closest('[data-nav-group],[data-flight-template],[data-flight-pin]');if(!b)return;
  if(b.dataset.navGroup){const index=Number(b.dataset.navGroup);navigateGame(groups[index][1].find(id=>$(id)&&!document.querySelector(`[data-tab="${id}"]`)?.hidden)||'reich');return;}
  loadHistory();const [kind,index]=(b.dataset.flightTemplate||b.dataset.flightPin).split(':'),a=history[kind]?.[Number(index)];if(!a)return;
  if(b.dataset.flightPin){if(kind==='pins')history.pins.splice(Number(index),1);else if(history.pins.length<2)history.pins.push(JSON.parse(JSON.stringify(a)));saveHistory();renderHistory();return;}
  if(!view.planeten.some(p=>p.koord===a.start)){show('Der Startplanet dieser Vorlage ist nicht mehr verfügbar.',true);return;}
  pickFleet(a.ziel,a.mission);const f=$('fleet');f.elements.start.value=a.start;if(f.elements.startart)f.elements.startart.value=a.startart||'planet';if(f.elements.zielart)f.elements.zielart.value=a.zielart||'planet';if(f.elements.termin_modus)f.elements.termin_modus.value='now';renderFleetStock();
  for(const k of ['geschwindigkeit','rueckgeschwindigkeit','haltedauer_stunden','siedler_volk'])if(f.elements[k]&&a[k]!=null)f.elements[k].value=a[k];
  for(const [id,k]of [['ship-inputs','schiffe'],['cargo-inputs','ladung']])$(id).querySelectorAll('input').forEach(input=>input.value=a[k]?.[input.dataset.map]||0);
  document.querySelectorAll('[data-ground-load]').forEach((input,i)=>input.value=a.truppen?.[i]||0);window.Flottenladung?.vorlage(a.personen);renderGround();
  schedulePreview();
 });
 function structured(value){
  if(value===null||value===undefined)return '<span class="muted">Nicht aufgeklärt</span>';
  if(value.typen&&Array.isArray(value.bestand)&&window.GroundUI)return GroundUI.intelligence(value);
  if(typeof value!=='object')return esc(typeof value==='boolean'?(value?'Ja':'Nein'):typeof value==='number'?fmt(value):value);
  if(Array.isArray(value))return value.length?`<div class="report-array">${value.map(v=>`<div>${structured(v)}</div>`).join('')}</div>`:'<span>Keine Einträge</span>';
  const entries=Object.entries(value).filter(([k])=>!k.startsWith('_'));
  return `<dl class="report-facts">${entries.map(([k,v])=>`<div><dt>${esc(named(k))}</dt><dd>${structured(v)}</dd></div>`).join('')}</dl>`;
 }
 function reportPicture(category,key,faction=''){
  // Volk unbekannt (z. B. Flottenkontakt): dasselbe Motiv in einer Volksfassung statt eines leeren Platzhalters
  const images=window.STERNEN_ART?.images||{},url=images[`${category}|${key}|${faction}`]?.url||images[`${category}|${key}|`]?.url||Object.entries(images).find(([k])=>k.startsWith(`${category}|${key}|`))?.[1]?.url;
  return url?`<img src="${esc(url)}" alt="${esc(named(key))}" loading="lazy">`:'<span class="report-art-unknown" aria-label="Volksmotiv unbekannt">◇</span>';
 }
 function reportResources(values){
  if(values==null)return '<p class="report-unknown">Nicht aufgeklärt</p>';
  return `<div class="report-resources">${Object.entries(values).sort(([a],[b])=>['erz','kristall','deuterium','nahrung','legierung','elektronik','konsumgut','xenokristall','antriebskern','habitatmodul'].indexOf(a)-['erz','kristall','deuterium','nahrung','legierung','elektronik','konsumgut','xenokristall','antriebskern','habitatmodul'].indexOf(b)).map(([k,n])=>`<div class="report-resource">${reportPicture('resources',k)}<span><strong>${fmt(n)}</strong><small>${esc(named(k))}</small></span></div>`).join('')||'<p>Keine Rohstoffe</p>'}</div>`;
 }
 function reportInventory(values,category,faction,levels=false){
  if(values==null)return '<p class="report-unknown">Nicht aufgeklärt · Dieser Scan enthüllt keine Bestände.</p>';
  const items=Object.entries(values).filter(([,n])=>Number(n)>0);
  return items.length?`<div class="report-inventory">${items.map(([k,n])=>`<article class="report-stock">${reportPicture(category,k,faction)}<span><strong>${levels?'Stufe ':''}${fmt(n)}</strong><small>${esc(named(k))}</small></span></article>`).join('')}</div>`:'<p class="report-empty">Keine vorhanden · beim Scan bestätigt</p>';
 }
 function reportFullScan(v,faction){
  if(!v)return '<p class="report-unknown">Vollscan nicht aufgeklärt · Ab Scanstärke 7 verfügbar.</p>';
  const fact=(label,value)=>`<div><dt>${label}</dt><dd>${value}</dd></div>`;
  const people=(v.personengruppen?.gruppen||[]).map((group,i)=>group.some(n=>n>0)?`<div class="report-population-row"><strong>${esc(named(['aurelianer','krath','veyari','syntheten'][i]))}</strong><dl>${group.map((n,j)=>fact(['Ungelernt','Fachkräfte','Forscher'][j],fmt(n/1000))).join('')}</dl></div>`:'').join('');
  const ground=v.bodentruppen,stock=ground?Object.fromEntries(ground.bestand.map((n,i)=>[ground.bilder[i],n])):null;
  if(stock&&ground.miliz?.aktiv)stock.miliz=ground.miliz.aktiv;
  return `<dl class="scan-key-facts">${fact('Bevölkerung',fmt(v.bevoelkerung))}${fact('Wohnplätze',fmt(v.wohnraum))}${fact('Baufelder',fmt(v.felder?.belegt)+' / '+fmt(v.felder?.gesamt))}${fact('Stabilität',fmt(v.stabilitaet)+' %')}${fact('Energieerzeugung',fmt(v.energie?.erzeugung))}${fact('Energieverbrauch',fmt(v.energie?.verbrauch))}</dl><h4>Bevölkerung nach Herkunft & Qualifikation</h4>${people||'<p>Nicht erfasst</p>'}<h4>Rohstoffbilanz pro Spielstunde</h4>${reportResources(v.rate)}<p class="report-note">Nettoänderung zum Scanzeitpunkt, einschließlich laufender Versorgung und Verarbeitung.</p><h4>Bodentruppen & Miliz</h4>${reportInventory(stock,'ground_units',faction)}${ground?`<details class="report-method"><summary>Ausbildungsaufträge & Verteidigungsstatus</summary>${GroundUI.intelligence(ground)}</details>`:''}<h4>Raketenvorrat</h4>${reportInventory(v.raketenbestand||Object.fromEntries(Object.entries(v.raketen||{}).filter(([k])=>['abfang','interplanetar'].includes(k))),'missiles',faction)}<p class="report-note">Siloplätze einschließlich Bau: ${fmt(v.raketen?.belegt_mit_bau)} / ${fmt(v.raketen?.kapazitaet)} · Garnison: ${fmt(v.garnison)}</p><details class="report-method"><summary>Gebäudezustand & Klima</summary><h4>Verbleibende Gebäudeintegrität</h4><dl class="scan-key-facts">${Object.entries(v.gebaeude_integritaet_prozent||{}).map(([k,n])=>fact(esc(named(k)),fmt(n)+' %')).join('')}</dl>${v.habitabilitaet?habitabilityMarkup(v.habitabilitaet):''}</details>`;
 }
 function reportAlt(row,kind,nurRunden=false){
  if(kind==='kampfberichte'){
   const names=row.spielernamen||{},factions=row.spielervoelker||{},name=id=>names[id]||'Spieler '+id,rounds=row.rundenprotokoll||[];
   const category=k=>rules.catalog.einheiten[k]?.schiff?'ships':'defenses';
   const unit=(key,id)=>`<div class="report-combatant">${reportPicture(category(key),key,factions[id]||'')}<strong>${esc(named(key))}</strong><small>${esc(name(id))}</small></div>`;
   const metric=(n,label)=>`<span><strong>${fmt(n)}</strong><small>${label}</small></span>`;
   const counts=groups=>(groups||[]).map(g=>`<section class="round-stock"><h5>${esc(name(g.spieler))} · Gruppe ${g.gruppe+1}</h5><div class="round-counts">${Object.entries(g.einheiten).map(([k,n])=>`<span>${fmt(n)} × ${esc(named(k))}</span>`).join('')||'Keine Überlebenden'}</div></section>`).join('');
   // Schnellfeuer je Paar sichtbar (Karl 25.10.2026: neues Balancing „gewissenhaft umsetzen“) – ohne den Hinweis wirkten
   // „60 Schüsse, 60 zerstört“ von 20 Bombern wie ein Fehler. Mehrfachfeuer erkennt der Client am Verteilungstext der Welt.
   const sfZeile=s=>{const p=rules.catalog.einheiten[s.einheit]?.zielprofil,n=p?.schnellfeuer?.[s.ziel]||0;if(n<=1)return '';
    return `<p class="report-sf">Schnellfeuer ×${fmt(n)} gegen ${esc(named(s.ziel))}${(p.verteilung||'').includes('springt')?' · nach jedem Abschuss springt die Salve auf das nächste Ziel derselben Art (Mehrfachfeuer)':''}</p>`;};
   const shots=(rows,title)=>`<h5 class="report-fire-title">${title}</h5><div class="report-fire-list">${rows.map(s=>`<article class="report-fire"><div class="report-duel">${unit(s.einheit,s.schuetze)}<span class="report-direction" aria-label="feuert auf">→</span>${unit(s.ziel,s.zielspieler)}</div><div class="report-hit-metrics">${metric(s.schuesse,'Schüsse')}${metric(s.zerstoert,'Zerstört')}${metric(s.explosionen,'davon Explosionen')}</div>${sfZeile(s)}<details class="report-shot-detail"><summary>Schadensbilanz · G${s.schuetzengruppe+1} → G${s.zielgruppe+1}</summary><dl><div><dt>Schaden angeboten</dt><dd>${fmt(s.schaden_angeboten)}</dd></div><div><dt>Schildschaden</dt><dd>${fmt(s.schildschaden)}</dd></div><div><dt>Strukturverlust</dt><dd>${fmt(s.strukturschaden)}</dd></div></dl></details></article>`).join('')||'<p>Kein Feuer abgegeben</p>'}</div>`;
   const destroyedRows=r=>['angreifer','verteidiger'].map(side=>{
    const after=r.nachher[side]||[];
    const losses=(r.vorher[side]||[]).flatMap(g=>Object.entries(g.einheiten).map(([key,n])=>({key,n,remain:after.find(a=>a.gruppe===g.gruppe&&a.spieler===g.spieler)?.einheiten?.[key]||0,id:g.spieler,group:g.gruppe}))).filter(x=>x.n>x.remain);
    return `<section class="round-loss-side"><h5>${side==='angreifer'?'Verluste der Angreifer':'Verluste der Verteidiger'}</h5>${losses.length?losses.map(x=>`<article class="round-loss">${reportPicture(category(x.key),x.key,factions[x.id]||'')}<div><strong>${fmt(x.n-x.remain)} × ${esc(named(x.key))} zerstört</strong><small>${esc(name(x.id))} · Gruppe ${x.group+1}</small><span>${fmt(x.n)} vor der Runde → ${fmt(x.remain)} überlebt</span></div></article>`).join(''):'<p class="report-kept">Keine Einheiten zerstört</p>'}</section>`;
   }).join('');
   const side=(side,label)=>{
    const ids=(rounds[0]?.vorher?.[side]||[]).map(g=>g.spieler),race=[...new Set(ids.map(id=>factions[id]).filter(Boolean))],faction=race.length===1?race[0]:'';
    return `<section class="report-force" data-side="${side}"><header><span class="eyebrow">${label}</span><h4>${(row[side]||[]).map(n=>`<button type="button" class="report-player-link" data-report-contact="${esc(n)}">${esc(n)} ↗</button>`).join(' · ')}</h4></header><div class="report-force-units">${Object.entries(row[side+'_vorher']||{}).filter(([,n])=>n>0).map(([k,n])=>{
     const remaining=row[side+'_nachher']?.[k]||0,permanent=row['verluste_'+side]?.[k]||0,end=rounds.length?(rounds.at(-1).nachher[side]||[]).reduce((sum,g)=>sum+(g.einheiten[k]||0),0):remaining,restored=Math.max(0,remaining-end),destroyed=n-end;
     return `<article class="report-balance" data-report-unit="${esc(k)}"><div class="report-balance-art">${reportPicture(category(k),k,faction)}<span>${esc(named(k))}</span></div><div><span>${fmt(n)} <i>→</i> ${fmt(remaining)}</span><small>Start → einsatzbereit</small>${destroyed?`<dl class="report-loss-breakdown"><div><dt>Im Gefecht zerstört</dt><dd>${fmt(destroyed)}</dd></div>${restored?`<div class="report-restored"><dt>Wiederaufgebaut</dt><dd>${fmt(restored)}</dd></div>`:''}<div class="report-loss"><dt>Dauerhaft verloren</dt><dd>${fmt(permanent)}</dd></div></dl>`:'<small class="report-kept">Keine Verluste</small>'}</div></article>`;
    }).join('')||'<p>Keine Einheiten</p>'}</div></section>`;
   };
   const winner=row.sieger==='angreifer'?'Angreifer siegen':row.sieger==='verteidiger'?'Verteidiger siegen':'Unentschieden';
   const voll=`<div class="combat-dossier"><div class="report-outcome"><span class="eyebrow">GEFECHT ABGESCHLOSSEN</span><h3>${winner}</h3><span>${fmt(row.runden)} Runden · ${esc(named(row.mission))} · ${esc(row.ort)}</span></div><div class="report-forces">${side('angreifer','Angreifer')}${side('verteidiger','Verteidiger')}</div><section class="report-material"><h4>Geborgene Beute</h4>${reportResources(row.beute)}<h4>Entstandenes Trümmerfeld</h4>${reportResources(row.truemmer)}<p class="report-note">Erz und Kristall aus verlorenen Schiffen und dauerhaft zerstörten Anlagen. Wiederaufgebaute Verteidigung bleibt erhalten.</p>${Object.values(row.truemmer||{}).some(n=>n>0)?`<button type="button" data-report-salvage="${esc(row.ort)}">Trümmerbergung planen</button>`:''}</section><div class="report-round-heading"><h4>Rundenprotokoll</h4><span>${rounds.length} dokumentiert</span></div><details class="report-method"><summary>So wird der Kampf ausgewertet</summary><p>Beide Seiten feuern gleichzeitig. Schilde erneuern sich pro Runde. Strukturverlust enthält auch explodierte Reststruktur; angebotener Schaden enthält absorbierte und wirkungslose Treffer. Feuer gleicher Schützen- und Zieltypen wird je Teilnehmergruppe zusammengefasst. Die Schlussbilanz enthält wiederhergestellte Verteidigung.</p></details>${rounds.length?rounds.map(r=>`<details class="combat-round" ${r.runde===1?'open':''}><summary><strong>Runde ${r.runde}</strong><span>${fmt(r.angreifer_feuer.reduce((n,s)=>n+s.zerstoert,0))} Verteidiger · ${fmt(r.verteidiger_feuer.reduce((n,s)=>n+s.zerstoert,0))} Angreifer zerstört</span></summary><div class="round-losses">${destroyedRows(r)}</div><details class="round-roster"><summary>Aufstellung zu Rundenbeginn</summary>${counts(r.vorher.angreifer)}${counts(r.vorher.verteidiger)}</details>${shots(r.angreifer_feuer,'Angreifer feuern')}${shots(r.verteidiger_feuer,'Verteidiger feuern')}<details class="round-roster"><summary>Überlebende am Rundenende</summary>${counts(r.nachher.angreifer)}${counts(r.nachher.verteidiger)}</details></details>`).join(''):'<p>Für diesen Kampf ist kein Rundenprotokoll gespeichert.</p>'}</div>`;
   // Nur das Rundenprotokoll (für den neuen Kampfbericht): alles nach der Überschrift „Rundenprotokoll“
   if(nurRunden){const t=document.createElement('template');t.innerHTML=voll;const h=t.content.querySelector('.report-round-heading');let out='';for(let n=h?.nextElementSibling;n;n=n.nextElementSibling)out+=n.outerHTML;return out;}
   return voll;
  }
  // Reine Meldungen (Vertragsangebot, Produktion, Ereignis) ohne Aufklärungsdaten: eigene Ansicht statt der
  // Aufklärungsvorlage „Flottenaufklärung · Unbekannter Kontakt“ (Karl 19.10.2026 am Handy).
  const scanFelder=['bestand','schiffe','verteidigung','gebaeude','forschung','vollscan','aufklaerung','schiffstypen','ladung'];
  if(row.text&&!scanFelder.some(k=>Object.hasOwn(row,k))){
   const lesbar=t=>typeof kernLesbar==='function'?kernLesbar(t):t,art=String(row.art||'');
   const augenbraue=/vertrag|diplom|allianz|pakt/.test(art)?'DIPLOMATIE':/bau|produktion|forschung|werft/.test(art)?'PRODUKTION':/angriff|kampf|flotte/.test(art)?'FLOTTE':'MELDUNG';
   return `<div class="meldung-dossier"><span class="eyebrow">${augenbraue}</span><h3>${esc(lesbar(row.text))}</h3>${row.zeit!=null?`<p class="report-note">${esc(typeof reportWhen==='function'?reportWhen(row):'')}</p>`:''}${augenbraue==='DIPLOMATIE'?(window.RahmenAngebote?.()||'<p>Kein offenes Angebot mehr – Verträge findest du unter „Diplomatische Anfragen & Verträge“.</p>'):''}</div>`;
  }
  const groups=[['bestand','Rohstoffvorräte','resources','market'],['schiffe','Flotte','ships','hangar'],['verteidigung','Verteidigungsanlagen','defenses','command'],['gebaeude','Gebäudestufen','buildings','command'],['forschung','Forschungsstand','research','research'],['vollscan','Vollscan · Infrastruktur & Bevölkerung',null,'command'],['aufklaerung','Kartierte Planetendaten · letzter Kenntnisstand',null,'command'],['schiffstypen','Schiffstypen',null,'hangar'],['ladung','Ladung','resources','market']];
  const used=new Set([...groups.map(([k])=>k),'text','volk','historisch','zeit','ziel','besitzer','alter_stunden','geschaetzt','scan_metadaten']);
  const sections=groups.filter(([k])=>Object.hasOwn(row,k)).map(([k,title,category,motif])=>{
   const content=k==='vollscan'?reportFullScan(row[k],row.volk||''):category==='resources'?reportResources(row[k]):category?reportInventory(row[k],category,row.volk||'',['gebaeude','forschung'].includes(k)):structured(row[k]);
   return `<details class="report-section" data-css="--section-art:url('assets/backgrounds.${motif}.webp')" ${['bestand','schiffe','verteidigung','vollscan'].includes(k)?'open':''}><summary><span>${title}</span><small>${row[k]==null?'Nicht aufgeklärt':'Erfasst'}</small></summary>${content}</details>`;
  }).join('');
  return `<div class="intelligence-dossier"><div class="report-scan-overview"><span class="eyebrow">${kind==='berichte'?'PLANETENAUFKLÄRUNG':'FLOTTENAUFKLÄRUNG'}</span><h3>${row.besitzer?`<button type="button" class="report-player-link" data-report-contact="${esc(row.besitzer)}" aria-label="${esc(row.besitzer)}: Nachricht verfassen">${esc(row.besitzer)} ↗</button>`:esc(row.ziel||'Unbekannter Kontakt')}</h3><div class="report-scan-meta"><span>${esc(row.ziel||'')}</span>${row.volk?`<span>${esc(named(row.volk))}</span>`:''}<span>Zivilisation ${row.scan_metadaten?.zivilisationsstufe?eraName(row.scan_metadaten.zivilisationsstufe):'nicht erfasst'}</span><span>${row.geschaetzt?'Geschätzte Werte':'Scanaufnahme'}</span><span>${fmt(row.alter_stunden||0)} Spielstunden alt</span></div>${row.ziel?`<div class="report-target-actions"><button type="button" data-report-mission="angriff" data-target="${esc(row.ziel)}">Angriff planen</button><button type="button" data-report-mission="spionage" data-target="${esc(row.ziel)}">Erneut ausspähen</button><button type="button" data-report-map="${esc(row.ziel)}">Im System zeigen</button></div>`:''}</div><p class="report-note">Bestände gelten zum Scanzeitpunkt. Nicht aufgeklärte Bereiche sind unbekannt, nicht leer.</p>${row.scan_metadaten?`<details class="report-method"><summary>Aufklärungstiefe · Stärke ${fmt(row.scan_metadaten.scanstaerke)}</summary><p>Scanstärke = Sondenzahl + eigene Spionagetechnik − gegnerische Spionagetechnik. Flotte ab 2, Verteidigung ab 3, Gebäude ab 5, Forschung und Vollscan ab 7. Mehr Sonden oder höhere Forschung erschließen zusätzliche Daten.</p></details>`:''}${sections}${structured(Object.fromEntries(Object.entries(row).filter(([k])=>!used.has(k))))}${row.text?`<p>${esc(row.text)}</p>`:''}</div>`;
 }
 // ---- Berichte (Karl 19.10.: „Spionage- und Kampfberichte hochwertig, umfassende Informationsdichte, super modern und
 // übersichtlich“; „klassischer Spionagebericht nur zum Ausklappen, sonst Kacheln mit unseren Assets – so auch die anderen
 // Berichte, alles intuitiv erklärt“). Ein Stil mit dem Infodisplay (dossier.css): Kopf mit Lage und Kennzahlen,
 // aufklappbare Teile mit Kachelrastern aus den Spielmotiven, Erklärungen eingeklappt, Text zum Kopieren, Aktionsleiste.
 // Nur Kernwerte; Unbekanntes bleibt unbekannt (gesperrte Kachel mit Grund), nie als Null.
 const RB_IKON={
  rohstoffe:'<svg viewBox="0 0 24 24" aria-hidden="true"><path d="M12 3 4 8l8 13 8-13-8-5z"/><path d="M4 8h16M9 8l3 13 3-13"/></svg>',
  flotte:'<svg viewBox="0 0 24 24" aria-hidden="true"><path d="M3 13c4-1 7-4 9-9 1 4 4 7 9 9-5 1-8 3-9 7-1-4-4-6-9-7z"/></svg>',
  verteidigung:'<svg viewBox="0 0 24 24" aria-hidden="true"><path d="M12 3 5 6v6c0 4 3 7 7 9 4-2 7-5 7-9V6l-7-3z"/></svg>',
  gebaeude:'<svg viewBox="0 0 24 24" aria-hidden="true"><path d="M4 21V9l5-3v15M9 21V4l6 3v14M15 21v-9l5 2v7M3 21h18"/></svg>',
  forschung:'<svg viewBox="0 0 24 24" aria-hidden="true"><path d="M9 3h6M10 3v6L5 18a2 2 0 0 0 2 3h10a2 2 0 0 0 2-3l-5-9V3"/><path d="M7.5 14h9"/></svg>',
  planet:'<svg viewBox="0 0 24 24" aria-hidden="true"><circle cx="12" cy="12" r="6"/><path d="M3 15c3 2 15-1 18-6"/></svg>',
  verlauf:'<svg viewBox="0 0 24 24" aria-hidden="true"><path d="M3 20h18M5 16l4-5 4 3 6-8"/></svg>',
  beute:'<svg viewBox="0 0 24 24" aria-hidden="true"><path d="M4 9h16l-2 11H6L4 9zM8 9V6a4 4 0 0 1 8 0v3"/></svg>',
  einheiten:'<svg viewBox="0 0 24 24" aria-hidden="true"><path d="M4 18h16M6 18V9l6-4 6 4v9M10 18v-5h4v5"/></svg>',
  text:'<svg viewBox="0 0 24 24" aria-hidden="true"><path d="M5 4h14M5 9h14M5 14h9M5 19h11"/></svg>',
  kontakt:'<svg viewBox="0 0 24 24" aria-hidden="true"><circle cx="12" cy="12" r="8"/><path d="M12 4v4M12 16v4M4 12h4M16 12h4"/></svg>'};
 const rbSumme=o=>Object.values(o||{}).reduce((s,n)=>s+(Number(n)||0),0);
 const rbListe=o=>Object.entries(o||{}).filter(([,n])=>Number(n)>0).sort((a,b)=>b[1]-a[1]);
 const rbKurz=n=>{n=Number(n)||0;const a=Math.abs(n);return a>=1e9?fmt(Math.round(n/1e8)/10)+' Mrd.':a>=1e6?fmt(Math.round(n/1e5)/10)+' Mio.':fmt(Math.round(n));};
 function rbMess(anteil,stufe){const a=Math.max(0,Math.min(1,Number(anteil)||0));return `<svg class="td-mess" viewBox="0 0 100 4" preserveAspectRatio="none" aria-hidden="true"><rect class="bahn" width="100" height="4" rx="2"/><rect class="wert ${stufe||(a>=.75?'hoch':a>=.4?'mittel':'tief')}" width="${(a*100).toFixed(1)}" height="4" rx="2"/></svg>`;}
 function rbTeil(id,titel,rechts,inhalt,offen=true){return `<details class="rb-teil rb-${id}"${offen?' open':''}><summary>${RB_IKON[id]||''}<h4>${esc(titel)}</h4><span>${rechts||''}</span></summary><div class="rb-inhalt">${inhalt}</div></details>`;}
 function rbKacheln(eintraege,kat,volk,stufe=false){return `<ul class="rb-kacheln">${eintraege.map(([k,n])=>`<li><span class="rb-bild">${reportPicture(kat,k,volk)}</span><b>${stufe?'<small>Stufe</small> ':''}${fmt(n)}</b><span class="rb-name">${esc(named(k))}</span></li>`).join('')}</ul>`;}
 function rbGesperrt(ab,staerke,was){const mehr=Number.isFinite(staerke)?ab-staerke:null;return `<p class="rb-gesperrt"><b>${esc(was)} nicht aufgeklärt</b><span>Sichtbar ab Scanstärke ${ab}${mehr>0?` · dieser Scan hatte ${staerke}: ${mehr} Sonde${mehr>1?'n':''} mehr oder bessere Spionagetechnik`:''}.</span></p>`;}
 function rbRohstoffe(b,mitVorzeichen=false){
  if(b==null)return '<p class="rb-gesperrt"><b>Rohstoffe nicht aufgeklärt</b></p>';
  const haupt=['erz','kristall','deuterium','nahrung'],neben=Object.keys(b).filter(k=>!haupt.includes(k)&&Number(b[k])),zahl=n=>(mitVorzeichen&&n>0?'+':'')+fmt(n);
  return `<ul class="rb-kacheln rb-gross">${haupt.filter(k=>k in b).map(k=>`<li class="${mitVorzeichen&&b[k]<0?'minus':''}"><span class="rb-bild">${reportPicture('resources',k)}</span><b>${zahl(b[k]||0)}</b><span class="rb-name">${esc(named(k))}</span></li>`).join('')}</ul>${neben.length?`<ul class="td-chips rb-neben">${neben.map(k=>`<li class="${mitVorzeichen&&b[k]<0?'schlecht':''}">${reportPicture('resources',k)}<span>${esc(named(k))}</span><b>${zahl(b[k])}</b></li>`).join('')}</ul>`:''}`;
 }
 function rbText(zeilen,titel='Klassischer Bericht (Text)'){const t=zeilen.filter(Boolean).join('\n');return `<details class="rb-teil rb-text rb-klassisch"><summary>${RB_IKON.text}<h4>${esc(titel)}</h4><span>zum Kopieren</span></summary><div class="rb-inhalt"><pre>${esc(t)}</pre><button type="button" class="rb-kopieren" data-rb-kopieren>Text kopieren</button></div></details>`;}
 const rbPaare=o=>rbListe(o).map(([k,n])=>`${named(k)} ${fmt(n)}`).join(' · ')||'keine';
 function rbAktionen(row,kind){
  if(!row.ziel)return '';const imLeser=typeof PlayerHelp!=='undefined'&&PlayerHelp.report?.row===row;
  return `<div class="rb-aktionen"><button type="button" class="rb-angriff" data-report-mission="angriff" data-target="${esc(row.ziel)}">Angreifen</button><button type="button" class="rb-spion" data-report-mission="spionage" data-target="${esc(row.ziel)}">Erneut spionieren</button>${imLeser&&row.schiffe&&row.verteidigung?'<button type="button" class="rb-sim" data-sim-report>Simulieren</button>':''}<button type="button" class="rb-karte" data-report-map="${esc(row.ziel)}">Im System</button></div>`;
 }
 // Spionage: Lage, Tiefe des Scans, Abschussrisiko eines neuen Vollscans, dann Rohstoffe → Flotte → Verteidigung → Gebäude → Forschung → Planet
 function rbSpionage(row,kind){
  const m=row.scan_metadaten||{},st=Number(m.scanstaerke),sw={schiffe:2,verteidigung:3,gebaeude:5,forschung:7,vollscan:7,...(m.schwellen||{})},volk=row.volk||'',c=rules.catalog;
  const stufen=[['Rohstoffe',0],['Flotte',sw.schiffe],['Verteidigung',sw.verteidigung],['Gebäude',sw.gebaeude],['Forschung · Vollscan',Math.max(sw.forschung,sw.vollscan)]];
  const tiefe=Number.isFinite(st)?`<ol class="rb-tiefe" aria-label="Aufklärungstiefe">${stufen.map(([t,ab])=>`<li class="${st>=ab?'ja':'nein'}"><span>${esc(t)}</span><small>${ab?'ab '+ab:'immer'}</small></li>`).join('')}</ol>`:'';
  // Abschussrisiko eines neuen Vollscans nach der Kernregel (flotte.rs): Chance ‰ = min(1000, Schiffe × Sonden × 2),
  // je Stufe Spionagetechnik-Vorsprung halbiert, je Stufe Rückstand verdoppelt – nur wenn Flotte und Forschung bekannt sind.
  const eigen=view.forschung?.stufen?.spionagetechnik||0,fremd=row.forschung?row.forschung.spionagetechnik||0:null,schiffe=row.schiffe?rbSumme(row.schiffe):null;
  const chips=[];
  if(Number.isFinite(st))chips.push(`<li class="${st>=Math.max(sw.forschung,sw.vollscan)?'ok':'mittel'}"><span class="td-chip-text">Scanstärke</span><b>${fmt(st)}</b></li>`);
  // Dieser Scan: Sondenzahl, Abwehrchance und ob die Sonden abgeschossen wurden (Kern seit 22.10.2026).
  if(row.sonden!=null)chips.push(`<li><span class="td-chip-text">Sonden</span><b>${fmt(row.sonden)}</b></li>`);
  if(row.abgeschossen!=null)chips.push(`<li class="${row.abgeschossen?'schlecht':'gut'}" title="Abwehrchance dieses Scans ${fmt(Math.round((row.abwehrchance_promille||0)/10))} %"><span class="td-chip-text">${row.abgeschossen?'Sonden abgeschossen':'Sonden zurück'}</span><b>${fmt(Math.round((row.abwehrchance_promille||0)/10))} % Abwehr</b></li>`);
  if(fremd!=null&&schiffe!=null){const diff=eigen-fremd,n=Math.max(1,Math.max(sw.forschung,sw.vollscan)-diff);let ch=Math.min(1000,schiffe*n*2);ch=diff>0?ch>>Math.min(diff,10):Math.min(1000,ch<<Math.min(-diff,10));
   chips.push(`<li class="${ch>=500?'schlecht':ch>=150?'mittel':'gut'}" title="Neuer Vollscan braucht ${n} Sonde${n>1?'n':''} (deine Spionagetechnik ${eigen}, seine ${fremd}); Abschusschance nach ${fmt(schiffe)} Schiffen in diesem Bericht"><span class="td-chip-text">Neuer Vollscan · ${n} Sonde${n>1?'n':''}</span><b>Abschuss ${fmt(Math.round(ch/10))} %</b></li>`);}
  else if(Number.isFinite(st))chips.push(`<li><span class="td-chip-text">Abschussrisiko</span><b>unbekannt</b></li>`);
  if(row.geschaetzt)chips.push('<li class="mittel"><span class="td-chip-text">Werte geschätzt</span></li>');
  const kopf=`<div class="rb-kopf rb-spionage-kopf">${volk?`<span class="rb-wappen">${reportPicture('emblems',volk,volk)}</span>`:''}<p class="rb-augenbraue">${kind==='spionageberichte'||row.mond?'Mondspionage':kind==='berichte'?'Spionagebericht':'Aufklärung'}</p>`
   +`<h3>${row.besitzer?`<button type="button" class="report-player-link" data-report-contact="${esc(row.besitzer)}" aria-label="${esc(row.besitzer)}: Nachricht verfassen">${esc(row.besitzer)} ↗</button>`:esc(row.ziel||'Unbekannter Kontakt')}</h3>`
   +`<p class="rb-meta"><span>${row.mond?'☾ Mond':'Planet'} [${esc(row.ziel||'?')}]</span>${volk?`<span>${esc(named(volk))}</span>`:''}${m.zivilisationsstufe?`<span>Zivilisation ${esc(eraName(m.zivilisationsstufe))}</span>`:''}<span>${fmt(row.alter_stunden||0)} Spielstunden alt</span></p>`
   +`${chips.length?`<ul class="td-chips rb-lage">${chips.join('')}</ul>`:''}${tiefe}</div>`;
  const feuer=row.schiffe?rbListe(row.schiffe).reduce((s,[k,n])=>s+(c.einheiten[k]?.regel?.angriff||0)*n,0):0;
  const einheiten=(o,kat,ab,was,leer)=>o==null?rbGesperrt(ab,st,was):rbListe(o).length?rbKacheln(rbListe(o),kat,volk):`<p class="rb-leer">${esc(leer)} – beim Scan bestätigt.</p>`;
  const KAMPF=['waffentechnik','schildtechnik','panzerung','raumschiffpanzerung','spionagetechnik'];
  // Mondberichte nennen Mondgebäude (eigene Bilder unter „moon_buildings“, sonst Gebäudebilder).
  const forschung=row.forschung==null?rbGesperrt(sw.forschung,st,'Forschung'):(()=>{const l=rbListe(row.forschung).sort((a,b)=>(KAMPF.includes(b[0])-KAMPF.includes(a[0]))||b[1]-a[1]);return l.length?`<ul class="rb-zeilen">${l.map(([k,n])=>`<li class="${KAMPF.includes(k)?'kampf':''}"><span class="rb-bild">${reportPicture('research',k)}</span><span class="rb-name">${esc(named(k))}</span><b>${fmt(n)}</b></li>`).join('')}</ul><p class="rb-hinweis">Kampfrelevante Forschung zuerst (Waffen, Schilde, Panzerung, Spionage).</p>`:'<p class="rb-leer">Keine Forschung – beim Scan bestätigt.</p>';})();
  const v=row.vollscan,planet=v?(()=>{const fakt=(t,wert,anteil,stufe,sub)=>`<div class="td-wert"><dt>${t}</dt><dd><b>${wert}</b>${sub?`<small>${sub}</small>`:''}${anteil==null?'':rbMess(anteil,stufe)}</dd></div>`;
   const e=v.energie||{},ok=(e.erzeugung||0)>=(e.verbrauch||0);
   return `<dl class="td-werte rb-werte">${fakt('Bevölkerung',fmt(v.bevoelkerung),v.wohnraum?v.bevoelkerung/v.wohnraum:null,null,`von ${fmt(v.wohnraum)} Wohnplätzen`)}${fakt('Baufelder',`${fmt(v.felder?.belegt)} / ${fmt(v.felder?.gesamt)}`,v.felder?.gesamt?v.felder.belegt/v.felder.gesamt:null)}${fakt('Stabilität',fmt(v.stabilitaet)+' %',(v.stabilitaet||0)/100,(v.stabilitaet||0)>=60?'hoch':(v.stabilitaet||0)>=30?'mittel':'tief')}${fakt('Energie',`${fmt(e.erzeugung)} / ${fmt(e.verbrauch)}`,e.erzeugung?Math.min(1,(e.verbrauch||0)/e.erzeugung):1,ok?'hoch':'tief',ok?'Erzeugung / Verbrauch':'Mangel: Verbrauch über Erzeugung')}${fakt('Garnison',fmt(v.garnison||0))}${fakt('Raketensilo',`${fmt(v.raketen?.belegt_mit_bau||0)} / ${fmt(v.raketen?.kapazitaet||0)}`)}</dl>`
    +`<h5 class="rb-zwischen">Rohstoffbilanz je Spielstunde</h5>${rbRohstoffe(v.rate,true)}<details class="rb-mehr"><summary>Alle Vollscan-Daten · Bevölkerung, Truppen, Raketen, Gebäudezustand, Klima</summary>${reportFullScan(v,volk)}</details>`;})():null;
  const z=[`${row.mond?'Mondspionage · Mond':'Spionagebericht · Planet'} [${row.ziel||'?'}] · ${row.besitzer||'unbekannt'}${volk?' ('+named(volk)+')':''} · ${typeof reportWhen==='function'?reportWhen(row):''}`,Number.isFinite(st)?`Scanstärke ${st}${row.geschaetzt?' · geschätzte Werte':''}`:'',
   row.sonden!=null?`Sonden: ${row.sonden}${row.abgeschossen?' – abgeschossen':' – zurück'} (Abwehrchance ${Math.round((row.abwehrchance_promille||0)/10)} %)`:'',
   Array.isArray(row.orbit)?`Im Orbit: ${row.orbit.length?row.orbit.map(o=>`${o.spieler} (${named(o.mission)}, ${rbPaare(o.schiffe)})`).join('; '):'keine Flotten'}`:'',
   `Rohstoffe: ${rbPaare(row.bestand)}`,`Flotte: ${row.schiffe==null?'nicht aufgeklärt':rbPaare(row.schiffe)}`,`Verteidigung: ${row.verteidigung==null?'nicht aufgeklärt':rbPaare(row.verteidigung)}`,`Gebäude: ${row.gebaeude==null?'nicht aufgeklärt':rbPaare(row.gebaeude)}`,`Forschung: ${row.forschung==null?'nicht aufgeklärt':rbPaare(row.forschung)}`,
   v?`Bevölkerung ${fmt(v.bevoelkerung)} / ${fmt(v.wohnraum)} · Baufelder ${fmt(v.felder?.belegt)}/${fmt(v.felder?.gesamt)} · Stabilität ${fmt(v.stabilitaet)} % · Energie ${fmt(v.energie?.erzeugung)}/${fmt(v.energie?.verbrauch)}`:''];
  return `<div class="rb rb-spionage">${kopf}`
   +rbTeil('rohstoffe','Rohstoffe',row.bestand?'Σ '+rbKurz(rbSumme(row.bestand)):'',rbRohstoffe(row.bestand))
   +rbTeil('flotte','Flotte',row.schiffe?`${fmt(rbSumme(row.schiffe))} Schiffe${feuer?` · Feuerkraft ${rbKurz(feuer)}`:''}`:'nicht aufgeklärt',einheiten(row.schiffe,'ships',sw.schiffe,'Flotte','Keine Schiffe stationiert'))
   +rbTeil('verteidigung','Verteidigung',row.verteidigung?`${fmt(rbSumme(row.verteidigung))} Anlagen`:'nicht aufgeklärt',einheiten(row.verteidigung,'defenses',sw.verteidigung,'Verteidigung','Keine Verteidigungsanlagen'))
   +(Array.isArray(row.orbit)?rbTeil('kontakt','Flotten im Orbit',row.orbit.length?`${fmt(row.orbit.length)} Flotte${row.orbit.length>1?'n':''} · kämpfen mit`:'keine',rbOrbit(row.orbit)):row.scan_metadaten&&Number.isFinite(st)&&st<sw.schiffe?rbTeil('kontakt','Flotten im Orbit','nicht aufgeklärt',rbGesperrt(sw.schiffe,st,'Flotten im Orbit'),false):'')
   +rbTeil('gebaeude','Gebäude',row.gebaeude?`${rbListe(row.gebaeude).length} Gebäude`:'nicht aufgeklärt',row.gebaeude==null?rbGesperrt(sw.gebaeude,st,'Gebäude'):rbKacheln(rbListe(row.gebaeude),'buildings',volk,true),row.gebaeude!=null)
   +rbTeil('forschung','Forschung',row.forschung?(rbListe(row.forschung).length?`${rbListe(row.forschung).length} Gebiete`:'nichts erforscht'):'nicht aufgeklärt',forschung,row.forschung!=null)
   +(planet?rbTeil('planet','Planetenzustand','Vollscan',planet):Number.isFinite(st)?rbTeil('planet','Planetenzustand','nicht aufgeklärt',rbGesperrt(Math.max(sw.forschung,sw.vollscan),st,'Vollscan (Bevölkerung, Energie, Truppen, Raketen)'),false):'')
   +(row.aufklaerung?`<details class="rb-mehr"><summary>Kartierte Planetendaten · letzter Kenntnisstand</summary>${structured(row.aufklaerung)}</details>`:'')
   +`<details class="rb-mehr"><summary>So entsteht der Bericht</summary><p>Scanstärke = Sondenzahl + eigene Spionagetechnik − Spionagetechnik des Ziels. Flotte ab ${sw.schiffe}, Verteidigung ab ${sw.verteidigung}, Gebäude ab ${sw.gebaeude}, Forschung und Vollscan ab ${Math.max(sw.forschung,sw.vollscan)}. Bestände gelten zum Scanzeitpunkt; nicht Aufgeklärtes ist unbekannt, nicht leer. Abschusschance je Scan: Schiffe am Ziel × Sonden × 2 ‰, je Stufe Technikvorsprung halbiert, je Stufe Rückstand verdoppelt.</p></details>`
   +rbText(z)+rbAktionen(row,kind)+'</div>';
 }
 // Flotten im Orbit eines Ziels (Halteflotten, Blockaden, Belagerer): Sie kämpfen bei einem Angriff mit.
 function rbOrbit(liste){
  if(!liste?.length)return '<p class="rb-leer">Keine Flotten im Orbit – beim Scan bestätigt.</p>';
  const art={eigen:'eigene Flotte',verbuendet:'verbündet',fremd:'fremd'};
  return `<ul class="rb-zeilen rb-orbit">${liste.map(o=>`<li class="rb-orbit-${esc(o.beziehung||'fremd')}"><span class="rb-name">${esc(o.spieler)} · ${esc(art[o.beziehung]||'fremd')} · ${esc(named(o.mission))}</span><b>${fmt(o.anzahl??rbSumme(o.schiffe))} Schiffe</b><small>${esc(rbPaare(o.schiffe))}</small></li>`).join('')}</ul><p class="rb-hinweis">Halteflotten und Blockaden kämpfen bei einem Angriff mit bzw. werden zuerst bekämpft – im Kampfsimulator mit einrechnen.</p>`;
 }
 // Erkundung eines freien Platzes (Spionage ohne Besitzer): Felder, Zone, Rohstoffreichtum, Eignung, Orbit
 function rbErkundung(row){
  const a=row.aufklaerung||{},pz=v=>v==null?'–':`${fmt(Math.round(v/10))} %`,eignung=window.GalaxyUI?.eignungKurz?GalaxyUI.eignungKurz({...a,koord:row.ziel}):'';
  const chips=[`<li class="ok"><span class="td-chip-text">Baufelder</span><b>${fmt(row.felder)}</b></li>`,`<li><span class="td-chip-text">Zone</span><b>${esc(named(row.zone||a.zone||''))}</b></li>`,
   `<li><span class="td-chip-text">Erzreichtum</span><b>${pz(row.reich_erz)}</b></li>`,`<li><span class="td-chip-text">Kristallreichtum</span><b>${pz(row.reich_kristall)}</b></li>`,
   row.nebel?'<li class="mittel"><span class="td-chip-text">Im Nebel</span></li>':'',row.sonden!=null?`<li><span class="td-chip-text">Sonden</span><b>${fmt(row.sonden)}</b></li>`:''].filter(Boolean);
  const kopf=`<div class="rb-kopf rb-spionage-kopf"><p class="rb-augenbraue">Erkundungsbericht</p><h3>Freier Platz [${esc(row.ziel)}]</h3>`
   +`<p class="rb-meta"><span>niemand besiedelt diesen Platz</span><span>${fmt(row.alter_stunden||0)} Spielstunden alt</span></p><ul class="td-chips rb-lage">${chips.join('')}</ul>${eignung?`<p class="rb-eignung">${eignung}</p>`:''}</div>`;
  const z=[`Erkundungsbericht · freier Platz [${row.ziel}] · ${typeof reportWhen==='function'?reportWhen(row):''}`,`${row.felder} Felder · Zone ${named(row.zone||'')} · Erz ${pz(row.reich_erz)} · Kristall ${pz(row.reich_kristall)}${row.nebel?' · im Nebel':''}`,
   `Im Orbit: ${(row.orbit||[]).length?(row.orbit||[]).map(o=>`${o.spieler} (${named(o.mission)}, ${rbPaare(o.schiffe)})`).join('; '):'keine Flotten'}`];
  return `<div class="rb rb-spionage rb-erkundung">${kopf}`
   +rbTeil('kontakt','Flotten im Orbit',(row.orbit||[]).length?`${fmt(row.orbit.length)} Flotte${row.orbit.length>1?'n':''}`:'keine',rbOrbit(row.orbit))
   +(Object.keys(a).length?`<details class="rb-mehr"><summary>Kartierte Platzdaten · Klima, Trümmer, Mond, Vorkommen</summary>${structured(a)}</details>`:'')
   +`<details class="rb-mehr"><summary>So entsteht der Bericht</summary><p>Eine Sonde auf einen freien Platz liefert Felder, Zone und Rohstoffreichtum des Systems sowie die Flotten im Orbit. Damit ist der Platz erkundet und kann kolonisiert werden. Rohstoffreichtum 100 % = Normalwert.</p></details>`
   +rbText(z)
   +`<div class="rb-aktionen"><button type="button" data-report-mission="kolonisieren" data-target="${esc(row.ziel)}">Kolonisieren</button><button type="button" class="rb-spion" data-report-mission="spionage" data-target="${esc(row.ziel)}">Erneut erkunden</button><button type="button" class="rb-karte" data-report-map="${esc(row.ziel)}">Im System</button></div></div>`;
 }
 // Bodenkampf: eine Runde der Invasion (Kern: bodenberichte); Tabelle aus GroundUI, Herkunft der Verteidiger
 function rbBoden(row){
  const seite={angriff:'du greifst an',verteidigung:'du verteidigst',hilfe:'deine Hilfstruppen kämpfen mit'}[row.seite]||'';
  const quellen=(row.verteidiger_quellen||[]).map(q=>`<li><span class="rb-name">${esc({garnison:'Garnison',verbuendete:'Verbündete',miliz:'Volksmiliz'}[q.quelle]||q.quelle)}${q.name?' · '+esc(q.name):''}${q.quelle==='verbuendete'&&q.flotte!=null?' · Flotte '+esc(q.flotte):''}</span><b>${fmt(rbSumme(Array.isArray(q.vorher)?Object.fromEntries(q.vorher.map((n,i)=>[i,n])):{x:q.vorher}))} → ${fmt(rbSumme(Array.isArray(q.nachher)?Object.fromEntries(q.nachher.map((n,i)=>[i,n])):{x:q.nachher}))}</b></li>`).join('');
  const verl=s=>(row[s+'_verluste']||[]).reduce((n,x)=>n+(Number(x)||0),0);
  // Invasion v2: Welle, Linie und Ergebnis im Kopf (Einsatzprobe 25.10.2026: der Bericht nannte nichts davon).
  const v2=row.regeln==='invasion-v2'&&row.linie,linieText=v2?(row.linie===row.linien?'Rückhalt':`Linie ${row.linie} von ${row.linien}`):'';
  const ergebnis=v2?({gebrochen:row.linie===row.linien?'Rückhalt gebrochen – verwüstet':'Linie gebrochen',gehalten:'Linie hält stand',abgewehrt:'Invasion abgewehrt'}[row.ergebnis]||row.ergebnis||''):'';
  const welleText=v2?`Welle ${row.welle} · ${linieText}${ergebnis?' · '+ergebnis:''}${row.miliz_kapituliert?' · Miliz kapituliert':''}`:'';
  const kopf=`<div class="rb-kopf rb-kampf-kopf"><p class="rb-augenbraue">Bodenkampf${seite?' · '+esc(seite):''}${welleText?' · '+esc(welleText):''}</p><h3>[${esc(row.planet)}]</h3><p class="rb-meta"><span>Angreifer ${esc(row.angreifer_name||'unbekannt')}</span><span>Verteidiger ${esc(row.verteidiger_name||'unbekannt')}</span><span>${esc(typeof reportWhen==='function'?reportWhen(row):'')}</span></p>`
   +`<ul class="td-chips rb-lage"><li class="${verl('angreifer')?'schlecht':'gut'}"><span class="td-chip-text">Angreifer verliert</span><b>${fmt(verl('angreifer'))}</b></li><li class="${verl('verteidiger')?'schlecht':'gut'}"><span class="td-chip-text">Verteidiger verliert</span><b>${fmt(verl('verteidiger'))}</b></li></ul></div>`;
  return `<div class="rb rb-boden">${kopf}`+(window.GroundUI?.report?GroundUI.report(row):'')
   +(quellen?rbTeil('verteidigung','Herkunft der Verteidiger','',`<ul class="rb-zeilen rb-herkunft">${quellen}</ul>`):'')
   +rbText([`Bodenkampf · [${row.planet}] · ${typeof reportWhen==='function'?reportWhen(row):''}${welleText?' · '+welleText:''}`,`Angreifer ${row.angreifer_name||'?'} verliert ${verl('angreifer')} · Verteidiger ${row.verteidiger_name||'?'} verliert ${verl('verteidiger')}`])+'</div>';
 }
 // Phalanxscan: Flottenbewegungen am Zielplaneten zum Scanzeitpunkt
 function rbPhalanx(row){
  const k=row.kontakte||[],r={anflug:'Anflug',orbit:'im Orbit',abflug:'Abflug',rueckkehr:'Rückkehr'};
  const kopf=`<div class="rb-kopf"><p class="rb-augenbraue">Phalanxscan #${esc(row.nr??'?')} vom Mond [${esc(row.mond||'?')}]</p><h3>Ziel [${esc(row.ziel)}]</h3><p class="rb-meta"><span>${esc(row.zeit_text||'')}</span><span>${fmt(k.length)} Kontakt${k.length===1?'':'e'}</span></p></div>`;
  const zeilen=k.map(c=>`<li><span class="rb-name">${esc(r[c.richtung]||c.richtung)} · ${c.besitzer?esc(c.besitzer):'unbekannt'} · [${esc(c.herkunft)}] → [${esc(c.ziel)}]</span><b>${c.schiffe!=null?fmt(c.schiffe)+' Schiffe':c.schiffe_spanne?esc(Array.isArray(c.schiffe_spanne)?c.schiffe_spanne.join('–'):c.schiffe_spanne)+' Schiffe':'Stärke unbekannt'}</b><small>${esc(c.ankunft_text||'')}${c.mission?' · '+esc(named(c.mission)):''}${c.schiffstypen?' · '+esc(rbPaare(c.schiffstypen)):''}</small></li>`).join('');
  return `<div class="rb rb-phalanx">${kopf}`+rbTeil('kontakt','Flottenbewegungen',`${fmt(k.length)}`,zeilen?`<ul class="rb-zeilen">${zeilen}</ul>`:'<p class="rb-leer">Keine Flotten unterwegs – beim Scan bestätigt.</p>')
   +rbText([`Phalanxscan #${row.nr} · Ziel [${row.ziel}] · ${row.zeit_text||''}`,...k.map(c=>`${r[c.richtung]||c.richtung}: ${c.besitzer||'unbekannt'} [${c.herkunft}] → [${c.ziel}] · ${c.ankunft_text||''}${c.schiffe!=null?' · '+c.schiffe+' Schiffe':''}`)])+'</div>';
 }
 // Beteiligte im Kampf je Flotte (Verband) und je verteidigender Partei (Planet, Mond, Halteflotte)
 function rbBeteiligte(row){
  const a=row.angreifer_flotten||[],v=row.verteidiger_parteien||[];if(a.length<2&&v.length<2)return '';
  const zeile=(t,vorher,nachher,extra)=>`<li><span class="rb-name">${t}</span><b>${fmt(rbSumme(vorher))} → ${fmt(rbSumme(nachher))}</b><small>${extra}</small></li>`;
  const art={planet:'Planet',mond:'Mond',flotte:'Halteflotte'};
  return rbTeil('einheiten','Beteiligte einzeln',`${fmt(a.length)} : ${fmt(v.length)}`,
   `<h5 class="rb-zwischen">Angreifende Flotten</h5><ul class="rb-zeilen">${a.map(f=>zeile(`${esc(f.spieler)} · Flotte #${esc(f.flotte)}`,f.vorher,f.nachher,`verloren: ${esc(rbPaare(f.verluste))}`)).join('')}</ul>`
   +`<h5 class="rb-zwischen">Verteidiger</h5><ul class="rb-zeilen">${v.map(p=>zeile(`${esc(p.spieler||'?')} · ${esc(art[p.art]||p.art)}${p.flotte!=null?' #'+esc(p.flotte):''}`,p.vorher,p.nachher,`Verlustwert ${fmt(p.verlust_wert||0)}`)).join('')}</ul>`,false);
 }
 // Mondentstehung nach dem Kampf (Kern: monde.wuerfe je Kampfnummer)
 function rbMondwurf(row){
  const m=row.mondwurf;if(!m)return '';
  return rbTeil('planet','Mondentstehung',m.entstanden?'Mond entstanden':'kein Mond',`<p>${m.entstanden?`<b>Ein Mond ist entstanden:</b> ${esc(named(m.typ))}, ${fmt(m.durchmesser)} km.`:'Kein Mond entstanden.'} Mondchance ${fmt(m.chance_prozent)} % aus ${fmt(m.truemmer)} Trümmern, Wurf ${fmt(m.wurf)}.</p>`,!!m.entstanden);
 }
 // Flottenaufklärung: Kontakt, Kurs, Zusammensetzung, Ladung
 function rbFlotte(row){
  const typen=row.schiffstypen,ladung=row.ladung;
  const kopf=`<div class="rb-kopf"><p class="rb-augenbraue">Flottenaufklärung · Kontakt #${esc(row.flotte??'?')}</p><h3>${row.von?`<button type="button" class="report-player-link" data-report-contact="${esc(row.von)}">${esc(row.von)} ↗</button>`:'Unbekannter Absender'}</h3>`
   +`<p class="rb-meta">${row.ziel_beim_scan?`<span>Kurs auf [${esc(row.ziel_beim_scan)}]</span>`:''}${row.ankunft_sekunden_beim_scan!=null?`<span>Ankunft beim Scan: ${esc(typeof GameTime!=='undefined'?GameTime.duration(Math.max(0,row.ankunft_sekunden_beim_scan-(row.zeit||0))):'')}</span>`:''}${row.alter_sekunden!=null?`<span>vor ${esc(typeof GameTime!=='undefined'?GameTime.duration(row.alter_sekunden):fmt(row.alter_sekunden)+' s')}</span>`:''}</p>`
   +`<ul class="td-chips rb-lage"><li class="${typen?'ok':'mittel'}"><span class="td-chip-text">Schiffe</span><b>${row.schiffe!=null?fmt(row.schiffe):'unbekannt'}</b></li>${row.abgeschirmt?'<li class="schlecht"><span class="td-chip-text">Abgeschirmt</span><b>Sensoren gestört</b></li>':''}</ul></div>`;
  return `<div class="rb rb-flottenbericht">${kopf}`
   +rbTeil('flotte','Zusammensetzung',typen?`${fmt(rbSumme(typen))} Schiffe`:'nicht aufgeklärt',typen?(rbListe(typen).length?rbKacheln(rbListe(typen),'ships',''):'<p class="rb-leer">Keine Schiffe.</p>'):'<p class="rb-gesperrt"><b>Zusammensetzung nicht aufgeklärt</b><span>Mehr Sonden oder bessere Spionagetechnik zeigen die Schiffstypen.</span></p>')
   +(ladung?rbTeil('beute','Ladung','Σ '+rbKurz(rbSumme(ladung)),rbRohstoffe(ladung)):'')
   +rbText([`Flottenaufklärung · Kontakt #${row.flotte??'?'} · ${row.von||'unbekannt'}`,row.ziel_beim_scan?`Kurs auf [${row.ziel_beim_scan}]`:'',`Schiffe: ${typen?rbPaare(typen):row.schiffe!=null?fmt(row.schiffe)+' (Typen unbekannt)':'unbekannt'}`,ladung?`Ladung: ${rbPaare(ladung)}`:''])+'</div>';
 }
 // Kampf: Ergebnis aus deiner Sicht, Fronten im Vergleich, Kräfteverlauf, Einheiten, Beute und Trümmer; Runden eingeklappt
 // Startplaneten der Angreifer (Kern: Kampfbericht.herkunft) – Karl 22.10.2026: „ich muss die Koordinaten des Planeten
 // sehen, von dem er mich angegriffen hat“. Ältere Berichte haben das Feld nicht.
 function rbHerkunft(row){
  const h=row.herkunft||[];if(!h.length)return '';
  const orte=[...new Map(h.map(x=>[x.start+(x.mond?'m':''),x])).values()];
  // Heimatwelt je Angreifer (Kern seit 22.10.2026) – Karl: „von dem Angreifer der Heimatplanet“.
  const heimaten=[...new Map(h.filter(x=>x.heimat).map(x=>[x.spieler,x])).values()];
  return `<p class="rb-herkunft">Gestartet von ${orte.map(x=>`<b class="koord">[${esc(x.start)}]</b>${x.mond?' ☾ Mond':''}${h.some(y=>y.spieler!==x.spieler)?' · '+esc(x.spieler):''} <button type="button" class="bs-link" data-gt-ziel="${esc(x.start)}">Auf der Karte</button>`).join(' · ')}</p>`
   +(heimaten.length?`<p class="rb-herkunft rb-heimat">Heimatwelt ${heimaten.map(x=>`${heimaten.length>1?esc(x.spieler)+' ':''}<b class="koord">[${esc(x.heimat)}]</b> <button type="button" class="bs-link" data-gt-ziel="${esc(x.heimat)}">Auf der Karte</button> <button type="button" class="bs-link" data-akte="${esc(x.spieler)}">Profil</button>`).join(' · ')}</p>`:'');
 }
 // Technik und Schaden (Karl 22.10.2026: „genauer Schaden, die Effekte von Forschung und Schilden“). Technik: Kern zum
 // Kampfzeitpunkt (Stufe + Wirkung inkl. Volksbonus); Schaden summiert aus dem Rundenprotokoll: abgefeuert, von Schilden
 // abgefangen, Strukturschaden, verpufft (Schuss zu schwach für den Schild oder Überschuss an zerstörten Zielen).
 function rbTechnik(row){
  const t=row.technik||[],rounds=row.rundenprotokoll||[];if(!t.length&&!rounds.length)return '';
  const seiteVon=n=>(row.angreifer||[]).includes(n)?'angreifer':'verteidiger',pz=v=>`${v>0?'+':''}${fmt(v)} %`;
  const feuer=(liste)=>{const z={schuesse:0,schaden:0,schild:0,struktur:0,zerstoert:0};for(const f of liste||[]){z.schuesse+=f.schuesse||0;z.schaden+=f.schaden_angeboten||0;z.schild+=f.schildschaden||0;z.struktur+=f.strukturschaden||0;z.zerstoert+=f.zerstoert||0;}z.verpufft=Math.max(0,z.schaden-z.schild-z.struktur);return z;};
  const gesamt=s=>feuer(rounds.flatMap(r=>r[s+'_feuer']||[])),a=gesamt('angreifer'),v=gesamt('verteidiger');
  const technik=t.length?`<div class="rb-tabellenrahmen"><table class="rb-techniktabelle"><thead><tr><th>Reich</th><th>Waffen</th><th>Schilde</th><th>Panzer</th></tr></thead><tbody>${t.map(x=>`<tr class="rb-${seiteVon(x.spieler)}"><th>${esc(x.spieler)}<small>${seiteVon(x.spieler)==='angreifer'?'Angreifer':'Verteidiger'}</small></th><td>St. ${fmt(x.waffentechnik)}<small>${pz(x.angriff_prozent)}</small></td><td>St. ${fmt(x.schildtechnik)}<small>${pz(x.schild_prozent)}</small></td><td>St. ${fmt(x.panzerung)}<small>${pz(x.struktur_prozent)}</small></td></tr>`).join('')}</tbody></table></div>`:'<p class="rb-leer">Technikstand nicht aufgezeichnet (älterer Bericht).</p>';
  const zeilen=[['schuesse','Schüsse'],['schaden','Schaden abgefeuert'],['schild','von Schilden abgefangen'],['struktur','Struktur&shy;schaden'],['verpufft','verpufft / Überschuss'],['zerstoert','Einheiten zerstört']];
  const schaden=rounds.length?`<table class="rb-schaden"><thead><tr><th>Gesamt</th><th>Angreifer</th><th>Verteidiger</th></tr></thead><tbody>${zeilen.map(([k,l])=>`<tr><th>${l}</th><td>${fmt(a[k])}</td><td>${fmt(v[k])}</td></tr>`).join('')}</tbody></table>`
   +`<table class="rb-schaden rb-runden"><thead><tr><th>Runde</th><th>Angreifer: Schaden · davon Schild</th><th>Verteidiger: Schaden · davon Schild</th></tr></thead><tbody>${rounds.map((r,i)=>{const x=feuer(r.angreifer_feuer),y=feuer(r.verteidiger_feuer);return `<tr><th>R${r.runde??i+1}</th><td>${fmt(x.schaden)} · ${fmt(x.schild)}<small>${fmt(x.zerstoert)} zerstört</small></td><td>${fmt(y.schaden)} · ${fmt(y.schild)}<small>${fmt(y.zerstoert)} zerstört</small></td></tr>`;}).join('')}</tbody></table>`:'';
  const quote=(z)=>z.schaden?Math.round(z.schild/z.schaden*100):0;
  // Schnellfeuer/Mehrfachfeuer in Worten, sobald es in diesem Kampf wirkte (neues Balancing 0.4.8).
  const sfPaar=rounds.flatMap(r=>[...(r.angreifer_feuer||[]),...(r.verteidiger_feuer||[])]).find(f=>(rules.catalog.einheiten[f.einheit]?.zielprofil?.schnellfeuer?.[f.ziel]||0)>1);
  const sfSatz=sfPaar?` Schnellfeuer wirkte (etwa ${esc(named(sfPaar.einheit))} gegen ${esc(named(sfPaar.ziel))}): zusätzliche Schüsse je Runde${(rules.catalog.einheiten[sfPaar.einheit]?.zielprofil?.verteilung||'').includes('springt')?'; nach jedem Abschuss springt die Salve auf das nächste Ziel derselben Art (Mehrfachfeuer)':''}.`:'';
  return rbTeil('technik','Technik & Schaden',rounds.length?`${rbKurz(a.schaden)} / ${rbKurz(v.schaden)}`:'',technik+schaden
   +(rounds.length?`<p class="rb-hinweis">Schilde fingen ${quote(v)} % des Angreiferfeuers und ${quote(a)} % des Verteidigerfeuers ab. Prozentwerte: Wirkung der Forschung inkl. Volksbonus – Waffentechnik erhöht den Schaden je Schuss, Schildtechnik die Schildstärke, Panzerung die Struktur.${sfSatz}</p>`:''));
 }
 function rbKampf(row){
  const names=row.spielernamen||{},factions=row.spielervoelker||{},rounds=row.rundenprotokoll||[],c=rules.catalog;
  const {meins,ergebnis,titel}=rbErgebnis(row);
  const kategorie=k=>c.einheiten[k]?.schiff?'ships':'defenses';
  const volkSeite=s=>{const ids=(rounds[0]?.vorher?.[s]||[]).map(g=>g.spieler),r=[...new Set(ids.map(id=>factions[id]).filter(Boolean))];return r.length===1?r[0]:'';};
  // Werftdock am Kampfort: Reparaturauftrag dieses Kampfs (nur für den Mondbesitzer sichtbar, view.monde.eigene[].reparatur)
  const reparatur=(view?.monde?.eigene||[]).find(m=>m.koord===row.ort)?.reparatur?.find(l=>Number(l.kampf)===Number(row.nummer))||null;
  // Reparatur: eigener Werftdock-Auftrag (mit Restzeit) oder – für alle Beteiligten – die Kernangabe `repariert`.
  const repDaten=reparatur?.schiffe||(rbSumme(row.repariert)?row.repariert:null),repSeite=reparatur?(meins||'verteidiger'):repDaten?'verteidiger':null,repN=(s,k)=>s===repSeite?Number(repDaten?.[k]||0):0,repSumme=s=>s===repSeite?rbSumme(repDaten):0;
  // Wiederaufgebaute Verteidigung: Kernangabe (genau) statt Rückrechnung aus dem Rundenprotokoll.
  const wieder=(s,k)=>s==='verteidiger'&&row.wiederaufgebaut?(k==null?rbSumme(row.wiederaufgebaut):Number(row.wiederaufgebaut[k]||0)):null;
  const repFertig=reparatur?.fertig!=null&&typeof GameTime!=='undefined'?GameTime.duration(Math.max(0,reparatur.fertig-GameTime.now())):'';
  const zaehle=(s,wann)=>wann==='vorher'?rbSumme(row[s+'_vorher']):rounds.length?(rounds.at(-1).nachher[s]||[]).reduce((n,g)=>n+rbSumme(g.einheiten),0):rbSumme(row[s+'_nachher']);
  const front=(s,label)=>{const start=zaehle(s,'vorher'),ende=zaehle(s,'nachher'),bereit=rbSumme(row[s+'_nachher']),verloren=rbSumme(row['verluste_'+s]),volk=volkSeite(s),anteil=start?Math.max(0,bereit)/start:0;
   return `<div class="rb-front rb-${s}${meins===s?' rb-eigen':''}">${volk?`<span class="rb-wappen">${reportPicture('emblems',volk,volk)}</span>`:''}<p class="rb-augenbraue">${label}${meins===s?' · du':''}</p><h4>${(row[s]||[]).map(n=>`<button type="button" class="report-player-link" data-report-contact="${esc(n)}">${esc(n)}</button>`).join(', ')||'–'}</h4>${s==='angreifer'?rbHerkunft(row):''}`
    +`<dl><div><dt>Start</dt><dd>${fmt(start)}</dd></div><div><dt>Im Gefecht zerstört</dt><dd class="${start-ende?'minus':''}">${fmt(start-ende)}</dd></div>${(wieder(s)??(bereit-ende))>0?`<div><dt>Wiederaufgebaut (${fmt(row.wiederaufbau_prozent??70)} %)</dt><dd class="plus">${fmt(wieder(s)??(bereit-ende))}</dd></div>`:''}${repSumme(s)?`<div><dt>Im Werftdock in Reparatur</dt><dd class="plus">${fmt(repSumme(s))}</dd></div>`:''}<div><dt>Dauerhaft verloren</dt><dd class="${verloren-repSumme(s)?'minus':''}">${fmt(Math.max(0,verloren-repSumme(s)))}</dd></div><div class="rb-bereit"><dt>Einsatzbereit</dt><dd>${fmt(bereit)}</dd></div></dl>${rbMess(anteil,anteil>=.6?'hoch':anteil>=.3?'mittel':'tief')}</div>`;};
  const kopf=`<div class="rb-kopf rb-kampf-kopf rb-${ergebnis}"><p class="rb-augenbraue">Kampfbericht · ${esc(named(row.mission))} · ${fmt(row.runden)} Runde${row.runden===1?'':'n'}</p><h3>${titel}</h3><p class="rb-meta"><span>Ort [${esc(row.ort)}]${row.ziel_mond?' ☾ Mond':''}</span><span>${esc(String(row.zeit||''))}</span>${meins&&ergebnis!=='neutral'?`<span>${row.sieger==='angreifer'?'Angreifer siegen':row.sieger==='verteidiger'?'Verteidiger siegen':'Kein Sieger'}</span>`:''}</p></div>`;
  // Kräfteverlauf: verbliebene Einheiten je Runde in % des Starts, beide Seiten
  const verlauf=(()=>{if(!rounds.length)return '';const pkt=s=>{const start=zaehle(s,'vorher')||1;return [1,...rounds.map(r=>(r.nachher[s]||[]).reduce((n,g)=>n+rbSumme(g.einheiten),0)/start)];};
   const a=pkt('angreifer'),v=pkt('verteidiger'),n=a.length-1,x=i=>(20+i*(260/Math.max(1,n))).toFixed(1),y=w=>(8+(1-Math.min(1,w))*84).toFixed(1),linie=w=>w.map((q,i)=>`${i?'L':'M'}${x(i)} ${y(q)}`).join(' ');
   return `<svg class="rb-verlauf" viewBox="0 0 300 112" role="img" aria-label="Kräfteverlauf: Angreifer ${Math.round(a.at(-1)*100)} %, Verteidiger ${Math.round(v.at(-1)*100)} % am Ende"><path class="raster" d="M14 8H286M14 50H286M14 92H286"/>${a.map((_,i)=>`<text x="${x(i)}" y="108">${i?'R'+i:'Start'}</text>`).join('')}<path class="angreifer" d="${linie(a)}"/><path class="verteidiger" d="${linie(v)}"/>${a.map((q,i)=>`<circle class="angreifer" cx="${x(i)}" cy="${y(q)}" r="2.6"/>`).join('')}${v.map((q,i)=>`<circle class="verteidiger" cx="${x(i)}" cy="${y(q)}" r="2.6"/>`).join('')}</svg><p class="rb-legende"><span class="angreifer">Angreifer ${Math.round(a.at(-1)*100)} %</span><span class="verteidiger">Verteidiger ${Math.round(v.at(-1)*100)} %</span><small>verbliebene Einheiten in % des Starts</small></p>`;})();
  // Einheiten je Seite: Start → einsatzbereit, Verlustbalken, Wiederaufbau
  const einheiten=s=>{const volk=volkSeite(s),l=Object.entries(row[s+'_vorher']||{}).filter(([,n])=>n>0);if(!l.length)return '<p class="rb-leer">Keine Einheiten.</p>';
   return `<ul class="rb-kacheln rb-verluste">${l.map(([k,n])=>{const bereit=row[s+'_nachher']?.[k]||0,dauerhaft=row['verluste_'+s]?.[k]||0,ende=rounds.length?(rounds.at(-1).nachher[s]||[]).reduce((m,g)=>m+(g.einheiten[k]||0),0):bereit,neu=wieder(s,k)??Math.max(0,bereit-ende);
    return `<li class="${dauerhaft?'verlust':''}"><span class="rb-bild">${reportPicture(kategorie(k),k,volk)}</span><b>${fmt(n)} → ${fmt(bereit)}</b><span class="rb-name">${esc(named(k))}</span>${rbMess(n?bereit/n:0,bereit>=n?'hoch':bereit>0?'mittel':'tief')}<small>${dauerhaft-repN(s,k)>0?`−${fmt(dauerhaft-repN(s,k))} verloren`:dauerhaft?'':'keine Verluste'}${repN(s,k)?`${dauerhaft-repN(s,k)>0?' · ':''}${fmt(repN(s,k))} in Reparatur`:''}${neu?` · ${fmt(neu)} wiederaufgebaut`:''}</small></li>`;}).join('')}</ul>`;};
  const material=Object.values(row.truemmer||{}).some(n=>n>0);
  const z=[`Kampfbericht · [${row.ort}] · ${named(row.mission)} · ${row.zeit||''}`,`Ergebnis: ${row.sieger==='angreifer'?'Angreifer siegen':row.sieger==='verteidiger'?'Verteidiger siegen':'Unentschieden'} nach ${row.runden} Runden`,
   `Angreifer (${(row.angreifer||[]).join(', ')}): ${rbPaare(row.angreifer_vorher)} → ${rbPaare(row.angreifer_nachher)} · verloren: ${rbPaare(row.verluste_angreifer)}`,
   `Verteidiger (${(row.verteidiger||[]).join(', ')}): ${rbPaare(row.verteidiger_vorher)} → ${rbPaare(row.verteidiger_nachher)} · verloren: ${rbPaare(row.verluste_verteidiger)}`,
   repDaten?`In Reparatur (Werftdock): ${rbPaare(repDaten)}`:'',row.wiederaufgebaut&&rbSumme(row.wiederaufgebaut)?`Wiederaufgebaut: ${rbPaare(row.wiederaufgebaut)}`:'',`Beute: ${rbPaare(row.beute)}`,`Trümmerfeld: ${rbPaare(row.truemmer)}`,
   row.mondwurf?`Mondchance ${row.mondwurf.chance_prozent} %: ${row.mondwurf.entstanden?'Mond entstanden ('+named(row.mondwurf.typ)+', '+row.mondwurf.durchmesser+' km)':'kein Mond'}`:''];
  return `<div class="rb rb-kampf">${kopf}<div class="rb-fronten">${front('angreifer','Angreifer')}<span class="rb-gegen" aria-hidden="true">VS</span>${front('verteidiger','Verteidiger')}</div>`
   +(verlauf?rbTeil('verlauf','Kräfteverlauf',rounds.length===1?'1 Runde':`${fmt(rounds.length)} Runden`,verlauf):'')
   +rbTechnik(row)
   +rbTeil('einheiten','Angreifer · Einheiten',`${fmt(zaehle('angreifer','vorher'))} → ${fmt(rbSumme(row.angreifer_nachher))}`,einheiten('angreifer'))
   +rbTeil('verteidigung','Verteidiger · Einheiten',`${fmt(zaehle('verteidiger','vorher'))} → ${fmt(rbSumme(row.verteidiger_nachher))}`,einheiten('verteidiger'))
   +rbTeil('beute','Beute & Trümmer',`${rbKurz(rbSumme(row.beute))} / ${rbKurz(rbSumme(row.truemmer))}`,`<h5 class="rb-zwischen">Geborgene Beute</h5>${rbSumme(row.beute)?rbRohstoffe(row.beute):'<p class="rb-leer">Keine Beute.</p>'}<h5 class="rb-zwischen">Entstandenes Trümmerfeld</h5>${material?rbRohstoffe(row.truemmer):'<p class="rb-leer">Kein Trümmerfeld.</p>'}<p class="rb-hinweis">Erz und Kristall (${fmt(row.truemmer_prozent??30)} %) aus verlorenen Schiffen und dauerhaft zerstörten Anlagen; wiederaufgebaute Verteidigung und Schiffe, die ein Werftdock am Kampfort repariert, hinterlassen keine Trümmer.</p>${reparatur?`<p class="rb-hinweis rb-reparatur">Dein Werftdock repariert ${rbPaare(reparatur.schiffe)}${repFertig?` – fertig in ${esc(repFertig)} Spielzeit`:''}.</p>`:''}${row.ueberlebende?`<p class="ep-rettung">${fmt(row.ueberlebende)} Überlebende in Rettungskapseln. Schicke Transporter oder Bergungsschiffe mit freien Personenplätzen.</p>`:''}${material||row.ueberlebende?`<button type="button" class="rb-bergen" data-report-salvage="${esc(row.ort)}">Trümmer bergen</button>`:''}`)
   +rbMondwurf(row)+rbBeteiligte(row)
   +rbTeil('text','Rundenprotokoll',`${fmt(rounds.length)} dokumentiert`,reportAlt(row,'kampfberichte',true),false)
   +rbText(z)+'</div>';
 }
 // Ergebnis eines Kampfes aus Sicht des Lesers (Sieg/Niederlage), sonst neutral
 function rbErgebnis(row){const ich=view?.name,meins=(row.angreifer||[]).includes(ich)?'angreifer':(row.verteidiger||[]).includes(ich)?'verteidiger':null;
  const ergebnis=row.sieger==='unentschieden'||!row.sieger?'remis':meins?row.sieger===meins?'sieg':'niederlage':'neutral';
  return {meins,ergebnis,titel:{sieg:'Sieg',niederlage:'Niederlage',remis:'Unentschieden',neutral:row.sieger==='angreifer'?'Angreifer siegen':'Verteidiger siegen'}[ergebnis]};}
 // Kompakte Karte für Berichtslisten (Seite „Berichte“): Motiv, Titel, Kurzinfo, Zeit – als <summary> zum Aufklappen des Dossiers
 // oder flach (Meldungen). Nur Werte aus dem Bericht.
 function reportKarte(row,kind,flach=false){
  const wann=esc(typeof reportWhen==='function'?reportWhen(row):String(row.zeit??'')),motiv=x=>x&&!x.includes('report-art-unknown')?x:'';
  let bild='',titel='',unter='',klasse='';
  if(kind==='kampfberichte'){const e=rbErgebnis(row);klasse='rb-'+e.ergebnis;bild=reportPicture('events',e.ergebnis==='niederlage'?'kampf_niederlage':e.ergebnis==='remis'?'kampf_unentschieden':'kampf_sieg');
   titel=`${esc(e.titel)} · [${esc(row.ort)}]`;unter=[esc(named(row.mission)),`${fmt(row.runden)} Runde${row.runden===1?'':'n'}`,`Verluste ${fmt(rbSumme(row.verluste_angreifer))} : ${fmt(rbSumme(row.verluste_verteidiger))}`,rbSumme(row.truemmer)?`Trümmer ${rbKurz(rbSumme(row.truemmer))}`:''].filter(Boolean).join(' · ');}
  else if(kind==='berichte'&&row.art==='erkundung'){bild=reportPicture('missions','spionage');
   titel=`Erkundung · freier Platz [${esc(row.ziel)}]`;unter=[`${fmt(row.felder)} Felder`,esc(named(row.zone||'')),(row.orbit||[]).length?`${fmt(row.orbit.length)} Flotten im Orbit`:''].filter(Boolean).join(' · ');}
  else if(kind==='bodenberichte'){bild=reportPicture('events','eroberung');const v=s=>(row[s+'_verluste']||[]).reduce((n,x)=>n+(Number(x)||0),0);
   titel=`Bodenkampf · [${esc(row.planet)}]`;unter=[esc(row.angreifer_name||''),`Verluste ${fmt(v('angreifer'))} : ${fmt(v('verteidiger'))}`].filter(Boolean).join(' · ');}
  else if(kind==='phalanxberichte'){bild=reportPicture('missions','flotten_spionage');titel=`Phalanxscan · [${esc(row.ziel)}]`;unter=`${fmt((row.kontakte||[]).length)} Kontakte`;}
  else if(kind==='berichte'||kind==='spionageberichte'){const st=row.scan_metadaten?.scanstaerke;bild=row.volk?reportPicture('emblems',row.volk,row.volk):reportPicture('missions','spionage');
   titel=`${row.mond?'Mondspionage':'Spionage'} · ${esc(row.besitzer||'Unbekannt')}`;unter=[`[${esc(row.ziel)}]`,st!=null?`Scanstärke ${fmt(st)}`:'',row.schiffe?`${fmt(rbSumme(row.schiffe))} Schiffe`:'',row.verteidigung?`${fmt(rbSumme(row.verteidigung))} Anlagen`:'',row.bestand?`Rohstoffe ${rbKurz(rbSumme(row.bestand))}`:''].filter(Boolean).join(' · ');}
  else if(kind==='flottenberichte'){const erst=rbListe(row.schiffstypen)[0];bild=erst?reportPicture('ships',erst[0],''):reportPicture('missions','flotten_spionage');
   titel=`Flottenkontakt #${esc(row.flotte??'?')} · ${esc(row.von||'Unbekannt')}`;unter=[row.schiffe!=null?`${fmt(row.schiffe)} Schiffe`:'Stärke unbekannt',row.ziel_beim_scan?`Kurs [${esc(row.ziel_beim_scan)}]`:''].filter(Boolean).join(' · ');}
  else{const text=String(row.text||''),such=(String(row.art||'')+' '+text).toLowerCase(),m=(RB_EREIGNIS.find(([r])=>r.test(such))||[])[1];bild=m?reportPicture('events',m):'';
   titel=esc(typeof kernLesbar==='function'?kernLesbar(text):text);unter=row.von?esc(named(row.von)):'';}
  const innen=`<span class="rb-karte-bild">${motiv(bild)}</span><span class="rb-karte-text"><b>${titel}</b>${unter?`<small>${unter}</small>`:''}</span><time>${wann}</time>`;
  return flach?`<div class="rb-karte ${klasse}">${innen}</div>`:`<summary class="rb-karte ${klasse}">${innen}</summary>`;
 }
 // Erkundete Planeten als Karten: Zone, Baufelder, Ertrag, Eignung für dein Volk, Kolonisieren / Im System
 function erkundetKarten(rows){
  const pz=v=>Array.isArray(v)?(v[0]===v[1]?`${fmt(Math.round(v[0]/10))} %`:`${fmt(Math.round(v[0]/10))}–${fmt(Math.round(v[1]/10))} %`):'–';
  return `<ul class="rb-planeten">${rows.slice().sort((a,b)=>(Number(a.alter_stunden)||0)-(Number(b.alter_stunden)||0)||String(a.koord).localeCompare(String(b.koord),'de',{numeric:true})).map(r=>{const zone=r.zone||r.habitabilitaet?.zone||'',frei=r.frei||r.status==='frei';
   return `<li class="rb-planet rb-zone-${esc(zone)}"><div class="rb-planet-kopf"><span class="rb-kugel" aria-hidden="true"></span><b>[${esc(r.koord)}]</b><small>${frei?'frei':esc(named(r.status||'besiedelt'))}</small><span>${esc(zone?named(zone):'Zone unbekannt')}</span></div>`
    +`<dl><div><dt>Felder</dt><dd>${fmt(r.felder)}</dd></div><div><dt>Erz</dt><dd>${pz(r.ertrag_promille?.erz)}</dd></div><div><dt>Kristall</dt><dd>${pz(r.ertrag_promille?.kristall)}</dd></div></dl>`
    +`${window.GalaxyUI?.eignungKurz?GalaxyUI.eignungKurz(r):''}${r.xeno_vorkommen||r.rohstoffring?`<p class="rb-planet-extra">${[r.xeno_vorkommen?'Xenokristall-Vorkommen':'',r.rohstoffring?'Rohstoffring':''].filter(Boolean).join(' · ')}</p>`:''}`
    +`<div class="rb-planet-aktionen">${frei?`<button type="button" data-report-mission="kolonisieren" data-target="${esc(r.koord)}">Kolonisieren</button>`:''}<button type="button" data-report-map="${esc(r.koord)}">Im System</button></div><small class="rb-planet-alter">erkundet vor ${fmt(r.alter_stunden||0)} Spielstunden</small></li>`;}).join('')}</ul>`;
 }
 // Meldungen ohne Aufklärungsdaten: Ereignisbild, Text, Zeit, Weiterführendes
 const RB_EREIGNIS=[[/vertrag.*bruch/,'vertrag_bruch'],[/vertrag.*ende/,'vertrag_ende'],[/vertrag|diplom|pakt|angebot/,'vertrag_angebot'],[/forschung/,'forschung_fertig'],[/fertigung|werft|produktion/,'fertigung_fertig'],[/bau|abriss|reparatur/,'bau_fertig'],[/spionage/,'spionage_bericht'],[/plünder|pluender/,'pluenderung'],[/eroberung|invasion|besetzung/,'eroberung'],[/belager|blockade/,'belagerung'],[/kampf|angriff/,'kampf_sieg'],[/handel|markt|liefer/,'handel_geliefert'],[/kolonie|siedl/,'kolonie_gegruendet'],[/hunger|nahrung/,'hunger'],[/energie/,'energie_mangel'],[/lager/,'lager_voll'],[/unruh|stabil/,'unruhen'],[/desert/,'desertion'],[/stufe|aufstieg|epoche/,'stufe_erreicht'],[/hyperraum|loch/,'hyperraumloch'],[/schutz/,'schutz_ende'],[/rueckkehr|rückkehr/,'flotte_rueckkehr'],[/flotte|expedition|bergung|rettung|erkundung/,'flotte_ankunft']];
 function rbMeldung(row){
  const lesbar=t=>typeof kernLesbar==='function'?kernLesbar(t):t,art=String(row.art||''),text=String(row.text||''),such=(art+' '+text).toLowerCase();
  const augenbraue=/vertrag|diplom|allianz|pakt/.test(art)?'Diplomatie':/bau|produktion|forschung|werft|fertigung/.test(art)?'Produktion':/angriff|kampf|flotte|spionage/.test(art)?'Flotte':'Meldung';
  const motiv=(RB_EREIGNIS.find(([r])=>r.test(such))||[])[1],bild=motiv?reportPicture('events',motiv):'';
  return `<div class="rb rb-meldung"><div class="rb-kopf rb-meldung-kopf">${bild&&!bild.includes('report-art-unknown')?`<span class="rb-ereignis">${bild}</span>`:''}<p class="rb-augenbraue">${augenbraue}${row.von?' · '+esc(named(row.von)):''}</p><h3>${esc(lesbar(text))}</h3>${row.zeit!=null?`<p class="rb-meta"><span>${esc(typeof reportWhen==='function'?reportWhen(row):String(row.zeit))}</span></p>`:''}</div>${augenbraue==='Diplomatie'?(window.RahmenAngebote?.()||'<p class="rb-hinweis">Kein offenes Angebot mehr – Verträge findest du unter „Diplomatische Anfragen & Verträge“.</p>'):''}</div>`;
 }
 function report(row,kind){
  if(kind==='systemberichte'&&window.Berichtsarchiv)return Berichtsarchiv.systembericht(row);
  try{
   if(kind==='kampfberichte')return rbKampf(row);
   if(kind==='berichte'&&row.art==='erkundung')return rbErkundung(row);
   if(kind==='bodenberichte')return rbBoden(row);
   if(kind==='phalanxberichte')return rbPhalanx(row);
   if(kind==='spionageberichte')return rbSpionage(row,kind);
   if(kind==='flottenberichte'&&('schiffstypen' in row||'von' in row))return rbFlotte(row);
   const scanFelder=['bestand','schiffe','verteidigung','gebaeude','forschung','vollscan','aufklaerung','schiffstypen','ladung'];
   if(row.text&&!scanFelder.some(k=>Object.hasOwn(row,k)))return rbMeldung(row);
   if(kind==='berichte'||Object.hasOwn(row,'scan_metadaten')||Object.hasOwn(row,'bestand'))return rbSpionage(row,kind);
  }catch(e){console.warn('Bericht',e);}
  return reportAlt(row,kind);
 }
 document.addEventListener('click',e=>{const b=e.target.closest('[data-rb-kopieren]');if(!b)return;const t=b.closest('.rb-klassisch')?.querySelector('pre')?.textContent||'';
  const fertig=ok=>{b.textContent=ok?'Kopiert ✓':'Kopieren nicht möglich';setTimeout(()=>{b.textContent='Text kopieren';},1800);};
  if(navigator.clipboard?.writeText)navigator.clipboard.writeText(t).then(()=>fertig(true),()=>fertig(false));else fertig(false);});
 document.addEventListener('click',e=>{const b=e.target.closest('[data-report-contact],[data-report-mission],[data-report-map]');if(!b)return;$('report-reader')?.close();$('galaxy-infocard')?.close();if(b.dataset.reportContact){if(window.Gegnerakte&&b.dataset.reportContact!==view?.name)Gegnerakte.oeffnen(b.dataset.reportContact);else{navigateGame('briefkasten');openComposer('privat',b.dataset.reportContact);}}else if(b.dataset.reportMission)pickFleet(b.dataset.target,b.dataset.reportMission);else{const [s,n,p]=b.dataset.reportMap.split(':').map(Number);navigateGame('karte');GalaxyUI.restore({stage:'system',sektor:s,system:n}).then(()=>GalaxyUI.pick('planet',p));}});
 document.addEventListener('click',e=>{const b=e.target.closest('[data-report-salvage]');if(!b)return;$('report-reader').close();const proxy=document.createElement('button');proxy.dataset.workSite=b.dataset.reportSalvage;proxy.dataset.workMission='recyceln';document.body.append(proxy);proxy.click();proxy.remove();});
 function planetFacts(p){
  const own=view.planeten.find(x=>x.koord===p.koord),latest=view.berichte?.find(x=>x.ziel===p.koord);
  if(own)return `<details open><summary>Eigener Planet · aktueller Stand</summary>${structured(Object.fromEntries(['bevoelkerung','wohnraum','felder','bestand','rate','energie','gebaeude','schiffe','verteidigung','raketen','garnison','bodentruppen'].map(k=>[k,own[k]])))}</details>`;
  return `<details open><summary>Verfügbare Planetendaten</summary>${structured(Object.fromEntries(Object.entries(p).filter(([k])=>!['habitabilitaet','beziehung','spieler','koord','position'].includes(k))))}</details>${latest?`<details><summary>Letzter Spionagebericht · ${fmt(latest.alter_stunden)} Spielstunden alt</summary>${report(latest,'berichte')}</details>`:'<p>Noch kein Spionagebericht. Sonde senden, um weitere Daten zu erhalten.</p>'}`;
 }
 function renderShipSelection(p){
  if(!p)return;const host=$('ship-inputs');
  // Start vom Mond: Bestand und Höchstwerte vom Mond, nicht vom Mutterplaneten – sonst blockierte die Browser-
  // Formularprüfung den Start still (max = Planetenbestand, z. B. 0 Weltenkrümmer; Konsolidierung 19.10.2026).
  const fe=$('fleet')?.elements;if(fe?.startart?.value==='mond'){const m=(view?.monde?.eigene||[]).find(x=>x.koord===fe.start.value);if(m)p={...p,schiffe:m.schiffe||{}};}
  for(const input of host.querySelectorAll('input[data-map]')){
   const key=input.dataset.map;let row=input.closest('[data-fleet-pick]');
   if(!row){const old=input.parentElement;row=document.createElement('article');row.className='fleet-pick-row';row.dataset.fleetPick=key;input.id='fleet-quantity-'+key;
    setStableHTML(row, `<img src="${esc(art('ships',key))}" alt="" loading="lazy"><div class="fleet-pick-name"><label for="${input.id}">${esc(named(key))}</label><small data-fleet-available></small></div><button type="button" class="ship-info" data-info="${esc(key)}" data-category="ships" aria-label="${esc(named(key))}: Schiffsdaten">i</button><div class="fleet-pick-quantity"><span>Menge</span><button type="button" data-ship-max="${esc(key)}">Max</button></div>`);
    old.before(row);row.querySelector('.fleet-pick-quantity button').before(input);old.remove();input.setAttribute('aria-label',named(key)+' auswählen');input.inputMode='numeric';
   }
   const available=p.schiffe[key]||0;input.max=String(available);row.querySelector('[data-fleet-available]').textContent=fmt(available)+' verfügbar';const leer=available===0&&!(Number(input.value)>0)&&!$('fleet-show-empty')?.checked;if(row.hidden!==leer)row.hidden=leer;
  }
  const counts=[...host.querySelectorAll('input')].map(i=>Number(i.value)||0),total=Object.values(p.schiffe).reduce((n,v)=>n+v,0),out=$('fleet-selection-count');if(out)out.textContent=counts.reduce((n,v)=>n+v,0)+' / '+total+' Schiffe ausgewählt · Start '+p.koord;
 }
 document.addEventListener('DOMContentLoaded',()=>{
  const planner=$('fleet-planner'),form=$('fleet'),ships=$('ship-inputs').closest('.fleet-selection');planner.open=false;
  $('all-ships').textContent='Alle am Startplaneten';const tools=document.createElement('div');tools.className='fleet-selection-tools';$('all-ships').before(tools);tools.append($('all-ships'));tools.insertAdjacentHTML('beforeend','<button type="button" id="clear-ships">Auswahl leeren</button><output id="fleet-selection-count" aria-live="polite"></output><label><input type="checkbox" id="fleet-show-empty">Auch nicht verfügbare Schiffstypen anzeigen</label>');
  form.elements.mission.closest('label').after(ships);ships.querySelector('summary').textContent='Schiffe gemeinsam auswählen';
  installFleetHub();
  form.elements.mission.value='saven';
  $('clear-ships').onclick=()=>{document.querySelectorAll('#ship-inputs input').forEach(i=>i.value=0);renderFleetStock();schedulePreview();};
  $('fleet-show-empty').onchange=()=>renderFleetStock();
 });
 document.addEventListener('click',e=>{const b=e.target.closest('[data-ship-max]');if(!b)return;const input=$('ship-inputs').querySelector('[data-map="'+b.dataset.shipMax+'"]');input.value=input.max;renderFleetStock();schedulePreview();});
 document.addEventListener('input',e=>{if(e.target.matches('#ship-inputs input[data-map]'))renderFleetStock();});
 function groundDraft(){return [...document.querySelectorAll('[data-ground-load]')].map(i=>Number(i.value)||0);}
 function renderGround(){
  const p=currentPlanet(),d=p?.bodentruppen;if(!d)return;
  let host=$('ground-forces');if(!host){host=document.createElement('section');host.id='ground-forces';($('truppen-content')||$('verteidigung')).append(host);}
  // Nur ein Eingabefeld schützt vor dem Neuzeichnen (ein angetippter Abbrechen-Knopf hielt den gelöschten Auftrag sichtbar).
  const fokus=document.activeElement;if(!(host.contains(fokus)&&/^(INPUT|SELECT|TEXTAREA)$/.test(fokus.tagName))){
   const stamp=JSON.stringify([p.koord,d,p.gebaeude.kaserne,p.industrie.regler.kaserne,view.forschung?.stufen?.soziologie,!!p.invasion?.wellen,Math.floor(p.bestand?.legierung||0)>=(Number(d.miliz?.kosten?.gesamt_legierung)||0)]);
   if(host.dataset.stamp!==stamp){host.dataset.stamp=stamp;setStableHTML(host, `<h3>Oberflächenverteidigung · ${esc(p.koord)}</h3>${GroundUI.militia(d,p)}<p>Garnison: Infanterie, Panzer und Drohnen bleiben auf diesem Planeten, bis du sie verlegst. Während einer Kampfkolonisation verteidigen sie automatisch die Oberfläche; neue Einheiten und eingetroffene Verstärkungen nehmen an der nächsten Bodenrunde teil.</p><p>${esc(d.konter)}</p>${document.body.hasAttribute('data-rahmen')?'':`${catalogToolbar('ground')}<div class="production-units">${GroundUI.cards(d,p)}</div>`}<p>Kaserne: ${p.industrie.regler.kaserne??100} % · ${d.ruht?'Produktion ruht: Leistung, Energie, Personal und Gebäude prüfen':'Produktionsbereit'}. Energie und Tempo unter Versorgung regeln.</p>${localProduction('kaserne',p)}<h4>Kasernenaufträge · ${d.fertigungsplaetze} Fertigungsplätze, ${(d.auftraege||[]).reduce((sum,a)=>sum+(a.lose||[]).reduce((n,b)=>n+b.anzahl*d.fertigungsplaetze_je_stueck[a.art],0),0)} belegt</h4>${(d.auftraege||[]).map((a,i)=>`<p>${fmt(a.rest)} ${esc(d.namen?.[a.art]||named(d.typen[a.art]))} · ${(a.lose||[]).reduce((n,b)=>n+b.anzahl,0)} gleichzeitig in Ausbildung · nächste Fertigstellung ${fmt((1-a.arbeit/a.gesamt)*100)} % <button type="button" data-ground-cancel="${a.id}" data-planet="${esc(p.koord)}">Abbrechen · ${a.rest-(a.lose||[]).filter(b=>b.arbeit*4<=b.gesamt).reduce((n,b)=>n+b.anzahl,0)} von ${a.rest} Stück erstatten</button></p>`).join('')||'<p>Keine Aufträge.</p>'}<details><summary>Bodenkampfberichte (${view.bodenberichte?.length||0})</summary>${(view.bodenberichte||[]).map(GroundUI.report).join('')||'<p>Noch kein Bodenkampf.</p>'}</details>`);}
  }
  catalogFilter(host,'ground');
  let load=$('ground-load');if(!load){load=document.createElement('details');load.id='ground-load';setStableHTML(load, `<summary>Bodentruppen mitführen</summary><p>100 Truppenplätze je Truppentransporter. Erlaubt: eigener Transport, Stationieren oder Kampfkolonisation (Invasion). Bodenrunden erfolgen alle 15 Spielminuten während der Besetzung. Garnison und Verstärkungen können die Übernahme verhindern.</p><p class="ground-fill"><button type="button" class="ff-mini" data-ground-fill>Laderaum füllen</button> <button type="button" class="ff-mini secondary" data-ground-clear>Leeren</button></p>${d.typen.map((t,i)=>`<label>${esc(d.namen?.[i]||named(t))}<small data-ground-available="${i}"></small><input data-ground-load="${i}" type="number" min="0" max="100000" value="0"><button type="button" class="ff-mini" data-ground-max="${i}">Max</button></label>`).join('')}<output id="ground-load-status"></output>`);$('fleet').append(load);}
  const source=view.planeten.find(x=>x.koord===$('fleet').elements.start.value)||p,units=source.bodentruppen?.bestand||[0,0,0],chosen=groundDraft(),ships=Number($('ship-inputs').querySelector('[data-map="truppentransporter"]')?.value)||0;
  load._grenzen={plaetze:d.truppenplaetze,units,ships};
  d.typen.forEach((t,i)=>{const input=load.querySelector(`[data-ground-load="${i}"]`);if(input){input.max=units[i]||0;input.closest('label').hidden=!(units[i]>0);if(!(units[i]>0))input.value='0';}const stock=load.querySelector(`[data-ground-available="${i}"]`);if(stock)stock.textContent=`${fmt(units[i]||0)} am Startplaneten`;});
  $('ground-load-status').textContent=`${chosen.reduce((n,v)=>n+v,0)} Bodeneinheiten · ${chosen.reduce((n,v,i)=>n+v*d.truppenplaetze[i],0)} / ${ships*100} Truppenplätze. Miliz bleibt auf ihrer Heimatkolonie.`;
 }
 document.addEventListener('submit',guard(async e=>{const form=e.target;if(form.dataset.groundBuild)await command([{typ:'boden_bauen',planet:form.dataset.planet,art:form.dataset.groundBuild,anzahl:Number(form.elements.anzahl.value)}]);}));
 document.addEventListener('click',guard(async e=>{const b=e.target.closest('[data-ground-cancel]');if(b)await command([{typ:'boden_storno',planet:b.dataset.planet,auftrag:Number(b.dataset.groundCancel)}]);}));
 // Am Handy waren 14 Zahlenfelder einzeln zu tippen (Konsolidierung 19.10.2026, M3): „Max“ je Truppenart füllt bis zum
 // Bestand bzw. freien Laderaum, „Laderaum füllen“ belädt alle Truppentransporter (spätere Zeitalter zuerst).
 document.addEventListener('click',e=>{const b=e.target.closest('[data-ground-max],[data-ground-fill],[data-ground-clear]');if(!b)return;e.preventDefault();
  /* Grenzen frisch: die Transporterzahl ändert sich ohne input-Ereignis */renderGround();const load=$('ground-load'),g=load?._grenzen;if(!g)return;const inputs=[...load.querySelectorAll('[data-ground-load]')],platz=i=>Math.max(1,g.plaetze?.[i]||1);
  const frei=()=>g.ships?g.ships*100-inputs.reduce((n,x,i)=>n+(Number(x.value)||0)*platz(i),0):Infinity;
  const fuelle=i=>{const rest=frei()+(Number(inputs[i].value)||0)*platz(i);inputs[i].value=Math.max(0,Math.floor(Math.min(g.units[i]||0,rest/platz(i))));};
  if(b.hasAttribute('data-ground-clear'))inputs.forEach(x=>x.value=0);
  else if(b.dataset.groundMax!=null)fuelle(Number(b.dataset.groundMax));
  else for(let i=inputs.length-1;i>=0;i--)if((g.units[i]||0)>0&&frei()>=platz(i))fuelle(i);
  inputs[0]?.dispatchEvent(new Event('input',{bubbles:true}));});
 document.addEventListener('input',e=>{if(e.target.closest('#fleet'))renderGround();});
 document.addEventListener('change',e=>{if(e.target.closest('#fleet'))renderGround();});
 document.addEventListener('change',e=>{if(e.target.name==='startart'&&e.target.closest('#fleet')&&typeof renderFleetStock==='function')renderFleetStock();});
 document.addEventListener('click',e=>{const b=e.target.closest('[data-planet-mission]');if(b&&!b.disabled)pickFleet(b.dataset.target,b.dataset.planetMission);});
 return {letztePruefung,renderFlightSchedule,renderShipSelection,coordinatedFlight,researchers,organic,workforce,renderIndustry,itemFacts,rememberFlight,schedulePreview,renderHistory,planetFacts,report,reportKarte,erkundetKarten,renderGround,groundDraft,catalogOrder,shipGallery,eraHeading,catalogToolbar,catalogFilter,localProduction,seriesForm};
})();
