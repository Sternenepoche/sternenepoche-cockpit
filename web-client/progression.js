'use strict';
// Dependency diagrams use the active public rules, never a decorative or invented unlock chain.
const Progression={identity:'',previous:null,before:{},finished:[],target:null,graphs:new Map(),timers:new Set()};
const ConstructionTech={open:new Set(),levels:new Map(),zoom:new Map(),initialized:new WeakMap()};
function constructionUnlocks(key){
 const c=rules.catalog,out=[];
 for(const kind of ['buildings','research','ships','defenses']){
  const entries=kind==='buildings'?Object.keys(c.gebaeude):kind==='research'?Object.keys(c.forschung):Object.keys(c.einheiten).filter(k=>c.einheiten[k].schiff===(kind==='ships'));
  for(const k of entries){const deps=TechnologyRules.dependencies(c,kind,k),gate=deps.find(d=>d.kind==='buildings'&&d.key===key);if(!gate)continue;
   const missing=deps.filter(d=>progressionHave(d.kind,d.key,d.scope)<d.level).map(d=>(d.kind==='civilization'?'Zeitalter':named(d.key))+' '+d.level+(d.scope==='home'?' auf der Heimatwelt':d.scope==='realm'?' im Reich':''));
   out.push({kind,key:k,name:named(k),level:gate.level,image:art(kind,k),missing});
  }
 }
 if(key==='kaserne')for(const t of c.bodentruppen?.typen||[]){const missing=[];if(currentPlanet().gebaeude.kaserne<t.kaserne||!currentPlanet().gebaeude.kaserne)missing.push('Kaserne '+t.kaserne);if(view.stufe<t.ab_stufe)missing.push('Zeitalter '+t.ab_stufe);out.push({kind:'ground',key:t.id,name:t.name,level:t.kaserne,image:art('ground_units',t.bild),missing});}
 if(key==='raketensilo')for(const r of c.raketen||[]){const missing=[];if((currentPlanet().gebaeude.raketensilo||0)<r.silo)missing.push('Silo '+r.silo);if(view.stufe<r.zivilisation)missing.push('Zeitalter '+r.zivilisation);for(const[k,n]of Object.entries(r.braucht))if((view.forschung.stufen[k]||0)<n)missing.push(named(k)+' '+n);out.push({kind:'missiles',key:r.key,name:r.name,level:r.silo,image:art('missiles',r.key),missing});}
 return out.sort((a,b)=>a.level-b.level||a.name.localeCompare(b.name,'de'));
}
function constructionDiagram(item,id,large=false){
 const links=constructionUnlocks(item.key),levels=[...new Set([item.level||1,item.nextLevel,...links.map(x=>x.level)])].sort((a,b)=>a-b);
 let level=ConstructionTech.levels.get(id);if(!levels.includes(level))level=levels.find(n=>n>item.level&&links.some(x=>x.level===n))||item.nextLevel;
 const outcomes=links.filter(x=>x.level===level),sources=new Map(),edges=[];
 const depsFor=x=>x.kind==='ground'?(()=>{const t=rules.catalog.bodentruppen.typen.find(t=>t.id===x.key);return [{kind:'buildings',key:'kaserne',level:t.kaserne},{kind:'civilization',key:String(t.ab_stufe),level:t.ab_stufe}];})():x.kind==='missiles'?(()=>{const r=rules.catalog.raketen.find(r=>r.key===x.key);return [{kind:'buildings',key:'raketensilo',level:r.silo},{kind:'civilization',key:String(r.zivilisation),level:r.zivilisation},...Object.entries(r.braucht).map(([key,level])=>({kind:'research',key,level}))];})():TechnologyRules.dependencies(rules.catalog,x.kind,x.key);
 const add=(d,to)=>{if(d.kind==='buildings'&&d.key===item.key){edges.push({from:'focus',to,need:d.level,met:item.level>=d.level});return;}const k=[d.kind,d.key,d.scope||'local'].join(':');const old=sources.get(k);if(!old||old.level<d.level)sources.set(k,{...d,id:k});edges.push({from:k,to,need:d.level,met:progressionHave(d.kind,d.key,d.scope)>=d.level});};
 for(const d of TechnologyRules.dependencies(rules.catalog,'buildings',item.key))add(d,'focus');
 outcomes.forEach((x,i)=>depsFor(x).forEach(d=>add(d,'out-'+i)));
 const nodeW=large?208:144,nodeH=large?100:74,step=large?116:88,height=Math.max(210,Math.max(sources.size,outcomes.length,1)*step+50),width=outcomes.length?(large?840:552):sources.size?(large?520:352):(large?240:180);
 const rootX=sources.size?(large?312:204):16,outX=sources.size?(large?608:392):(large?312:204);
 const nodes=[...sources.values()].map((d,i)=>({...d,x:16,y:30+i*step,name:d.kind==='civilization'?'Zeitalter '+d.level:named(d.key),image:d.kind==='civilization'?art('portraits',view.volk,view.volk):art(d.kind,d.key),have:progressionHave(d.kind,d.key,d.scope)}));
 const root={id:'focus',kind:'buildings',key:item.key,level,name:named(item.key),image:art('buildings',item.key),have:item.level,x:rootX,y:Math.max(30,(height-nodeH)/2)};nodes.push(root);
 outcomes.forEach((x,i)=>nodes.push({...x,id:'out-'+i,gate:level,x:outX,y:Math.max(30,(height-outcomes.length*step)/2)+i*step,have:1,level:1,outcome:true}));
 const positions=new Map(nodes.map(n=>[n.id,n])),scale=ConstructionTech.zoom.get(id+(large?':large':''))||1,prefix='ct-'+id.replaceAll(':','-')+(large?'-large':'');
 const paths=edges.map((e,i)=>{const a=positions.get(e.from),b=positions.get(e.to),x=a.x+nodeW,y=a.y+nodeH/2,tx=b.x,ty=b.y+nodeH/2,mid=(x+tx)/2;return `<path id="${prefix}-edge-${i}" data-tech-from="${esc(e.from)}" data-tech-to="${esc(e.to)}" class="${e.met?'met':'missing'}" d="M${x},${y} C${mid},${y} ${mid},${ty} ${tx-5},${ty}" marker-end="url(#${prefix}-${e.met?'yes':'no'})"><title>${esc(a.name)} ${e.need} → ${esc(b.name)}</title></path>`;}).join('');
 return `<div class="tech-map-controls"><label><span class="tech-step-label">Etappe</span><select data-tech-level aria-label="Gebäudestufe und Freischaltungen">${levels.map(n=>`<option value="${n}" ${n===level?'selected':''}>Stufe ${n}${n===item.level?' · aktuell':''}${links.some(x=>x.level===n)?' · '+links.filter(x=>x.level===n).length+' Freischaltungen':''}</option>`).join('')}</select></label><div><button type="button" data-tech-fit aria-label="Gesamten Tech-Tree einpassen">⊡</button><button type="button" data-tech-zoom="-" aria-label="Tech-Tree verkleinern">−</button><output>${Math.round(scale*100)} %</output><button type="button" data-tech-zoom="+" aria-label="Tech-Tree vergrößern">+</button></div></div><p class="tech-map-legend"><span>✓ erfüllt</span><span>◇ fehlt</span> · Pfeile zeigen Voraussetzungen · Wischen zum Verschieben</p><div class="tech-map-viewport" data-tech-view="${level}" data-tech-large="${large}" tabindex="0" aria-label="Tech-Tree: verschieben, zoomen und Knoten öffnen"><div class="tech-map-space" data-css="width:${width*scale}px;height:${height*scale}px"><div class="tech-map-canvas" data-css="width:${width}px;height:${height}px;transform:scale(${scale})"><svg class="tech-map-edges" width="${width}" height="${height}" aria-label="Verbindungen zwischen Voraussetzungen und Freischaltungen"><defs><marker id="${prefix}-yes" markerWidth="6" markerHeight="6" refX="5" refY="3" orient="auto"><path d="M0,0 L6,3 L0,6" fill="#8fd6b4"/></marker><marker id="${prefix}-no" markerWidth="6" markerHeight="6" refX="5" refY="3" orient="auto"><path d="M0,0 L6,3 L0,6" fill="#bfa46d"/></marker></defs>${paths}</svg>${nodes.map(n=>{const met=n.outcome?!n.missing.length:n.have>=n.level;return `<button type="button" id="${prefix}-node-${n.id.replaceAll(':','-')}" class="tech-map-node ${large?'large-node':''} ${met?'met':'missing'} ${n.id==='focus'?'focus':''}" data-css="left:${n.x}px;top:${n.y}px;width:${nodeW}px;height:${nodeH}px" data-construction-open="${esc(n.key)}" data-tech-kind="${n.kind}" data-tech-gate="${n.gate||n.level}" data-tech-node="${esc(n.id)}" title="${esc(n.outcome?(n.missing.length?'Fehlt: '+n.missing.join(' · '):'Freigeschaltet'):n.name+' '+n.have+' / '+n.level)}"><img src="${esc(n.image)}" alt="" loading="lazy"><span><strong>${esc(n.name)}</strong><small>${n.outcome?(met?'Freigeschaltet':'Voraussetzungen fehlen'):`${n.have} → ${n.level}${n.scope==='home'?' · Heimat':n.scope==='realm'?' · Reich':''}`}</small></span><b>${met?'✓':'◇'}</b></button>`;}).join('')}</div></div></div>${outcomes.length?'':`<p class="mini-tech-note">Stufe ${level} verbessert diese Anlage. Auf dieser Etappe wird kein weiterer Typ freigeschaltet.</p>`}`;
}
function constructionTree(item){
 const key=item.key,id=currentPlanet().koord+':'+key,open=ConstructionTech.open.has(id);
 if(!open)return `<details class="construction-tech" data-construction-tech="${esc(key)}" data-tech-id="${esc(id)}"><summary>Tech-Tree <span aria-hidden="true">＋</span></summary></details>`;
 const next=view.naechste_stufe;
 const stageGate=rules.catalog.stufen?.[view.stufe-1]?.gebaeude?.[key];
 return `<details class="construction-tech" data-construction-tech="${esc(key)}" data-tech-id="${esc(id)}" open><summary>Tech-Tree <span aria-hidden="true">−</span></summary><div class="construction-tech-body"><div class="mini-tech-origin"><strong>${esc(named(key))} · Stufe ${item.level}</strong><button type="button" data-tech-enlarge="${esc(key)}">⛶ Groß öffnen</button></div>${constructionDiagram(item,id)}<div class="mini-tech-ascension">${next?`<strong>Nächstes Zeitalter · ${esc(next.name)}</strong>${stageGate?`<p>Aufstiegsziel: ${esc(named(key))} ${stageGate} auf einem eigenen Planeten · ${progressionHave('buildings',key,'realm')>=stageGate?'erfüllt':'noch offen'}</p>`:'<p>Für dieses Gebäude ist keine feste Ausbaustufe vorgeschrieben.</p>'}<div class="mini-tech-progress"><span data-css="width:${100*next.bedingungen.filter(b=>b.erfuellt).length/Math.max(1,next.bedingungen.length)}%"></span></div><details class="mini-tech-more"><summary>${next.bedingungen.filter(b=>b.erfuellt).length} / ${next.bedingungen.length} Aufstiegsbedingungen erfüllt</summary><ul>${next.bedingungen.map(b=>`<li class="${b.erfuellt?'ready':'pending'}"><span>${b.erfuellt?'✓':'◇'}</span>${esc(kernLesbar(String(b.text||"")))}</li>`).join('')}</ul><p>${next.haltezeit_stunden?`${next.erfuellt_seit_stunden} / ${next.haltezeit_stunden} Spielstunden halten`:'Aufstieg sofort möglich, sobald alles erfüllt ist'} · Kosten auf der Heimatwelt</p>${costHtml(next.kosten,view.planeten.find(p=>p.heimat)?.bestand)}</details>`:'<strong>Höchstes Zivilisationszeitalter erreicht</strong>'}</div></div></details>`;
}
document.addEventListener('toggle',e=>{const d=e.target;if(!d.matches?.('[data-construction-tech]'))return;const had=ConstructionTech.open.has(d.dataset.techId);if(had===d.open)return;if(d.open)ConstructionTech.open.add(d.dataset.techId);else ConstructionTech.open.delete(d.dataset.techId);renderCatalog('buildings','gebaeude-content');},true);
document.addEventListener('change',e=>{if(!e.target.matches('[data-tech-level]'))return;const d=e.target.closest('[data-tech-id]');ConstructionTech.levels.set(d.dataset.techId,Number(e.target.value));if(d.id==='construction-tree-dialog')renderConstructionLarge();else renderCatalog('buildings','gebaeude-content',true);});
document.addEventListener('click',e=>{
 const help=e.target.closest('[data-tech-help]');if(help){const note=$('construction-tree-dialog').querySelector('.tech-large-note');note.hidden=!note.hidden;help.setAttribute('aria-expanded',String(!note.hidden));return;}
 const enlarge=e.target.closest('[data-tech-enlarge]');if(enlarge){showConstructionLarge(enlarge.dataset.techEnlarge);return;}if(e.target.closest('[data-tech-close]')){$('construction-tree-dialog').close();return;}
 const fit=e.target.closest('[data-tech-fit]');if(fit){const d=fit.closest('[data-tech-id]'),v=d.querySelector('.tech-map-viewport'),canvas=d.querySelector('.tech-map-canvas');ConstructionTech.zoom.set(d.dataset.techId+(d.id==='construction-tree-dialog'?':large':''),Math.max(.25,Math.min(1,Math.min(v.clientWidth/canvas.offsetWidth,v.clientHeight/canvas.offsetHeight))));if(d.id==='construction-tree-dialog')renderConstructionLarge();else renderCatalog('buildings','gebaeude-content');v.scrollLeft=0;v.scrollTop=0;return;}
 const zoom=e.target.closest('[data-tech-zoom]');if(zoom){const d=zoom.closest('[data-tech-id]'),v=d.querySelector('.tech-map-viewport'),zoomId=d.dataset.techId+(d.id==='construction-tree-dialog'?':large':''),old=ConstructionTech.zoom.get(zoomId)||1,next=Math.max(.25,Math.min(1.5,Math.round((old+(zoom.dataset.techZoom==='+'?.1:-.1))*10)/10)),cx=(v.scrollLeft+v.clientWidth/2)/old,cy=(v.scrollTop+v.clientHeight/2)/old;ConstructionTech.zoom.set(zoomId,next);if(d.id==='construction-tree-dialog')renderConstructionLarge();else renderCatalog('buildings','gebaeude-content');v.scrollLeft=cx*next-v.clientWidth/2;v.scrollTop=cy*next-v.clientHeight/2;return;}
 const b=e.target.closest('[data-construction-open]');if(!b)return;const kind=b.dataset.techKind,key=b.dataset.constructionOpen;if(['ground','missiles'].includes(kind)){Object.assign(PlayerHelp.codex,{kind,key,query:'',faction:view.volk});navigateGame('codex');renderCodex();}else if(kind==='civilization'){if(Number(key)<2){navigateGame('profil');return;}Progression.target={kind,key,level:Number(key)};navigateGame('freischaltungen');renderResearchTree('progression-map');}else showItemInfo(kind,key);
});
let constructionDrag=null,constructionDragUntil=0;
function initializeConstructionGraphs(host){
 for(const v of host.querySelectorAll('.tech-map-viewport')){if(ConstructionTech.initialized.get(v)===v.dataset.techView)continue;ConstructionTech.initialized.set(v,v.dataset.techView);requestAnimationFrame(()=>{if(!v.isConnected)return;const n=v.querySelector('[data-tech-node="focus"]'),scale=ConstructionTech.zoom.get(v.closest('[data-tech-id]').dataset.techId+(v.dataset.techLarge==='true'?':large':''))||1;v.scrollLeft=Math.max(0,n.offsetLeft*scale-12);v.scrollTop=Math.max(0,(n.offsetTop+n.offsetHeight/2)*scale-v.clientHeight/2);});}
}
function showConstructionLarge(key){
 let d=$('construction-tree-dialog');if(!d){d=document.createElement('dialog');d.id='construction-tree-dialog';d.setAttribute('aria-labelledby','construction-tree-title');document.body.append(d);}
 d.dataset.building=key;d.dataset.techId=currentPlanet().koord+':'+key;renderConstructionLarge();if(!d.open)d.showModal();initializeConstructionGraphs(d);
}
function renderConstructionLarge(){
 const d=$('construction-tree-dialog');if(!d)return;const item=itemRecords('buildings').find(x=>x.key===d.dataset.building);if(!item)return;const helpOpen=d.querySelector('.tech-large-note')?.hidden===false;
 setStableHTML(d,`<header class="tech-large-heading"><div><h2 id="construction-tree-title">${esc(named(item.key))}<small>Tech-Tree</small></h2></div><button type="button" data-tech-help aria-label="Bedienhinweise anzeigen" aria-expanded="${helpOpen}">i</button><button type="button" data-tech-close aria-label="Großansicht schließen">×</button></header>${constructionDiagram(item,d.dataset.techId,true)}<p class="tech-large-note" ${helpOpen?'':'hidden'}>Knoten antippen: Datenblatt öffnen. Wischen oder ziehen: verschieben. − / +: zoomen. ⊡: gesamten Baum einpassen. Grün: erfüllt, Gold: noch offen.</p>`);
 initializeConstructionGraphs(d);
}
document.addEventListener('pointerdown',e=>{const el=e.target.closest('.tech-map-viewport');if(!el||e.pointerType!=='mouse'||e.button!==0)return;constructionDrag={el,id:e.pointerId,x:e.clientX,y:e.clientY,left:el.scrollLeft,top:el.scrollTop,moved:false};});
document.addEventListener('pointermove',e=>{const d=constructionDrag;if(!d||d.id!==e.pointerId)return;if(Math.hypot(e.clientX-d.x,e.clientY-d.y)>5)d.moved=true;if(!d.moved)return;d.el.setPointerCapture(e.pointerId);d.el.scrollLeft=d.left+d.x-e.clientX;d.el.scrollTop=d.top+d.y-e.clientY;e.preventDefault();});
document.addEventListener('pointerup',e=>{if(constructionDrag?.id===e.pointerId){if(constructionDrag.moved)constructionDragUntil=performance.now()+300;constructionDrag=null;}});
document.addEventListener('pointercancel',()=>constructionDrag=null);
document.addEventListener('click',e=>{if(performance.now()<constructionDragUntil&&e.target.closest('.tech-map-viewport')){e.preventDefault();e.stopImmediatePropagation();}},true);
function progressionSnapshot(){
 const identity=world.world_id+':'+view.name;
 if(Progression.identity!==identity){Progression.identity=identity;Progression.previous=null;Progression.target=null;Progression.graphs.clear();for(const t of Progression.timers)clearTimeout(t);Progression.timers.clear();for(const el of document.querySelectorAll('.research-tree'))delete el.dataset.stamp;}
 Progression.before=Progression.previous||{};Progression.finished=Progression.previous?Object.keys(view.forschung.stufen).filter(k=>view.forschung.stufen[k]>(Progression.previous[k]||0)):[];
 Progression.previous={...view.forschung.stufen};document.body.dataset.volk=view.volk;
}
function progressionRule(kind,key){const c=rules.catalog;return kind==='research'?c.forschung[key]:kind==='buildings'?c.gebaeude[key]:kind==='ships'||kind==='defenses'?c.einheiten[key]?.regel:kind==='civilization'?c.stufen?.[Number(key)-2]:null;}
function progressionHave(kind,key,scope='local'){
 if(kind==='research')return view.forschung.stufen[key]||0;
 if(kind==='buildings')return scope==='realm'?Math.max(...view.planeten.map(p=>p.gebaeude[key]||0)):(scope==='home'?view.planeten.find(p=>p.heimat):currentPlanet()).gebaeude[key]||0;
 if(kind==='civilization')return view.stufe;
 return currentPlanet().schiffe[key]||currentPlanet().verteidigung[key]||0;
}
function progressionDependencies(kind,key,level,scope='local'){
 return TechnologyRules.dependencies(rules.catalog,kind,key,scope);
}
function civilizationFacts(key){
 const r=progressionRule('civilization',key);if(!r)return [];
 const home=view.planeten.find(p=>p.heimat),all=view.planeten,items=[`${fmt(r.einwohner)} Einwohner im Reich (jetzt ${fmt(all.reduce((s,p)=>s+p.bevoelkerung,0))})`];
 if(r.kolonien)items.push(`${r.kolonien} Kolonien (jetzt ${view.kolonien.anzahl})`);
 if(r.versorgung_plus)items.push('Nahrung und Energie im Plus');
 if(r.stabilitaet)items.push(`Heimatstabilität ≥ ${r.stabilitaet} % (jetzt ${home.stabilitaet} %)`);
 if(r.konsum_deckung)items.push(`Konsumdeckung ≥ ${Math.round(r.konsum_deckung*100)} % (jetzt ${home.konsum_deckung} %)`);
 items.push(rules.catalog.stufen_haltezeit_stunden>0?`${rules.catalog.stufen_haltezeit_stunden} Spielstunden halten und Aufstieg bezahlen`:'Aufstieg sofort möglich, sobald alle Bedingungen erfüllt und die Rohstoffe auf der Heimatwelt vorhanden sind');return items;
}
function progressionGraph(target,successors=false){
 const nodes=new Map(),edges=[],visit=(t,stack=new Set())=>{
  const id=[t.kind,t.key,t.level,t.scope||'local'].join(':');if(nodes.has(id))return id;
  const node={...t,id,deps:[],depth:0,have:progressionHave(t.kind,t.key,t.scope),r:progressionRule(t.kind,t.key)};nodes.set(id,node);
  if(stack.has(id)||!node.r)return id;const next=new Set(stack);next.add(id);
  // Already completed gates remain visible; their historical prerequisites need not fill the screen.
  if(node.have<t.level||id===targetId)for(const d of progressionDependencies(t.kind,t.key,t.level,t.scope)){const parent=visit(d,next);node.deps.push(parent);edges.push({from:parent,to:id});}
  node.depth=node.deps.length?Math.max(...node.deps.map(d=>nodes.get(d).depth))+1:0;
  node.ready=node.deps.every(d=>nodes.get(d).have>=nodes.get(d).level);
  if(t.kind==='civilization'&&node.have<t.level){const home=view.planeten.find(p=>p.heimat),r=node.r;
   node.ready=node.ready&&view.planeten.reduce((s,p)=>s+p.bevoelkerung,0)>=r.einwohner&&view.kolonien.anzahl>=r.kolonien&&home.stabilitaet>=r.stabilitaet&&home.konsum_deckung>=r.konsum_deckung*100;
   if(r.versorgung_plus)node.ready=node.ready&&view.planeten.every(p=>(rules.catalog.voelker[view.volk].ohne_nahrung||p.rate.nahrung>=0)&&p.energie.erzeugung>=p.energie.verbrauch);
  }
  return id;
 };
 const targetId=[target.kind,target.key,target.level,target.scope||'local'].join(':');visit(target);
 if(successors&&target.kind==='research')for(const next of progressionFollowers(target.key).filter(f=>f.kind==='research'&&!f.effect))visit(next);
 // One building card per location, with the required milestones on its outgoing paths.
 // A Labor 1 and Labor 6 are stages of the same building, not two independent facilities.
 const groups=new Map(),aliases=new Map();
 for(const n of nodes.values()){
  let scope=n.scope||'local';if(n.kind==='buildings'&&(scope==='home'&&currentPlanet().heimat||scope==='realm'&&view.planeten.length===1))scope='local';
  const id=n.kind==='buildings'?[n.kind,n.key,scope].join(':'):n.id,g=groups.get(id);
  aliases.set(n.id,id);
  if(!g)groups.set(id,{...n,id,scope,levels:[n.level]});else{g.levels=[...new Set([...g.levels,n.level])].sort((a,b)=>a-b);if(n.level>g.level){g.level=n.level;g.ready=n.ready;}g.depth=Math.max(g.depth,n.depth);}
 }
 const folded=[];for(const edge of edges){const from=aliases.get(edge.from),to=aliases.get(edge.to),requires=nodes.get(edge.from).level;if(from!==to&&!folded.some(e=>e.from===from&&e.to===to&&e.requires===requires))folded.push({from,to,requires});}
 return {nodes:[...groups.values()],edges:folded,targetId:aliases.get(targetId)};
}
function progressionFollowers(key){
 const c=rules.catalog,out=[];
 for(const[k,e]of Object.entries(c.einheiten))if(e.taktik?.voraussetzungen?.[key])out.push({kind:e.schiff?'ships':'defenses',key:k,level:1,relation:`Freischaltung ab ${named(key)} ${e.taktik.voraussetzungen[key]}`});
 for(const[k,g]of Object.entries(c.forschung_voraussetzungen||{}))if(c.forschung[k]&&g.forschung?.[key])out.push({kind:'research',key:k,level:1,relation:`Freischaltung ab ${named(key)} ${g.forschung[key]}`});
 (c.stufen||[]).forEach((s,i)=>{if(s.forschung?.[key])out.push({kind:'civilization',key:String(i+2),level:i+2,relation:`Aufstiegsbedingung: ${named(key)} ${s.forschung[key]}`});});
 const affected=key==='energietechnik'?['solarkraftwerk','fusionskraftwerk']:key==='werkstoffkunde'?['giesserei']:key==='agrarwissenschaft'?['farm']:key==='xenomaterialkunde'?['xenoextraktor']:key==='plasmatechnik'?['erzmine','kristallmine','deuteriumsynthesizer']:key==='automatisierung'?Object.keys(c.gebaeude).filter(k=>c.gebaeude[k].arbeiter>0):[];
 affected.forEach(k=>out.push({kind:'buildings',key:k,level:1,relation:'Verbessert den Betrieb · keine Freischaltbedingung',effect:true}));
 for(const[k,e]of Object.entries(c.einheiten))if(e.regel.antrieb===key||['waffentechnik','schildtechnik','panzerung'].includes(key)||e.schiff&&['logistik','computertechnik','abschirmtechnik'].includes(key)||k==='spionagesonde'&&key==='spionagetechnik')out.push({kind:e.schiff?'ships':'defenses',key:k,level:1,relation:e.regel.antrieb===key?'Erhöht das Tempo · keine Freischaltbedingung':'Verbessert diese Einheit · keine Freischaltbedingung',effect:true});
 if(key==='astrophysik'&&!c.einheiten.kolonieschiff?.taktik?.voraussetzungen?.astrophysik)out.push({kind:'ships',key:'kolonieschiff',level:1,relation:'Ermöglicht Kolonien · Schiff braucht zusätzlich Werft und Zivilisationsstufe',effect:true});
 if(['spionagetechnik','ueberwachungstechnik'].includes(key))for(const k of ['geheimdienst','sensorphalanx'])out.push({kind:'buildings',key:k,level:1,relation:'Gemeinsame Wirkung auf Aufklärung und Vorwarnung',effect:true});
 return out;
}
function progressionNode(n,targetId){
 const done=n.have>=n.level,job=n.kind==='research'&&view.forschung.aktiv?.forschung===n.key?view.forschung.aktiv:n.kind==='buildings'?(n.scope==='home'?view.planeten.find(p=>p.heimat):currentPlanet()).bauschleife.find(j=>j.gebaeude===n.key):null;
 const title=n.kind==='civilization'?n.r?.name||'Zivilisationsstufe '+n.key:named(n.key),img=n.kind==='civilization'?art('portraits',view.volk,view.volk):art(n.kind,n.key);
 const status=done?'Erfüllt':job?'In Arbeit':n.ready?'Voraussetzungen erfüllt':'Voraussetzungen fehlen';
 return `<article class="tree-node ${done?'done':n.ready?'ready':'locked'} ${n.id===targetId?'tree-target':''}" data-node-id="${esc(n.id)}" data-node-key="${esc(n.key)}" data-node-kind="${n.kind}"><button type="button" data-tree-open="${esc(n.key)}" data-tree-kind="${n.kind}" aria-label="${esc(title+' · '+status)}"><img src="${esc(img)}" alt=""><span><strong>${esc(title)}</strong><small>${n.kind==='civilization'?'Aufstieg':n.kind==='research'?'Reichsweite Technik':n.scope==='home'?'Heimatwelt':n.scope==='realm'?'Im Reich':'Dein Planet'} · ${n.kind==='civilization'?n.have+' / '+n.level:'Stufe '+n.have+' / '+n.level}</small>${n.levels?.length>1?`<small>Etappen: ${n.levels.join(' · ')}</small>`:''}<b>${done?'✓ ':job?'◷ ':n.ready?'● ':'◇ '}${status}</b></span></button>${job?`<div class="tree-job">${artShell(n.kind,n.key,job)}${jobLine(job)}</div>`:''}${n.kind==='civilization'?`<p class="civilization-core">${fmt(n.r.einwohner)} Einwohner${n.r.kolonien?' · '+n.r.kolonien+' Kolonien':''}</p><details><summary>Alle Aufstiegsbedingungen</summary><ul>${civilizationFacts(n.key).map(t=>`<li>${esc(t)}</li>`).join('')}</ul>${costHtml(n.r.kosten)}</details>`:''}</article>`;
}
function renderResearchTree(host='research-tree',selected=uiSelections.research){
 const el=$(host);if(!el||!view||!rules.catalog.stufen)return;
 const stage=k=>view.forschung.aktiv?.forschung===k?view.forschung.aktiv.stufe:Math.max(1,view.forschung.stufen[k]||0);
 const target=host==='progression-map'?(Progression.target||{kind:'research',key:selected||'energietechnik',level:stage(selected||'energietechnik')}):{kind:'research',key:selected||'energietechnik',level:stage(selected||'energietechnik')};
 const graph=progressionGraph(target,true),job=view.forschung.aktiv,stamp=JSON.stringify([Progression.identity,view.volk,target,currentPlanet().koord,graph.nodes.map(n=>[n.id,n.have,n.ready]),job?.forschung,job?.stufe,job?.fertig_sekunden,view.stufe]);
 if(el.dataset.stamp===stamp)return;const scrollLeft=el.querySelector('.tree-scroll')?.scrollLeft||0,sameTarget=el.dataset.target===graph.targetId;el.dataset.target=graph.targetId;el.dataset.stamp=stamp;Progression.graphs.set(host,graph);
 const followers=target.kind==='research'?progressionFollowers(target.key):[],tiers=Array.from({length:Math.max(...graph.nodes.map(n=>n.depth))+1},(_,depth)=>graph.nodes.filter(n=>n.depth===depth));
 const options=[...Object.keys(rules.catalog.forschung).map(k=>['research',k]),...Object.entries(rules.catalog.einheiten).map(([k,e])=>[e.schiff?'ships':'defenses',k]),...Object.keys(rules.catalog.gebaeude).map(k=>['buildings',k]),...rules.catalog.stufen.map((_,i)=>['civilization',String(i+2)])];
 const openResearch=graph.nodes.filter(n=>n.kind==='research'&&n.have<n.level),openBuildings=graph.nodes.filter(n=>n.kind==='buildings'&&n.have<n.level);
 const followHtml=followers.map(f=>{const ready=f.effect||progressionGraph(f).nodes.find(n=>n.id===[f.kind,f.key,f.level,'local'].join(':')).ready;return `<button type="button" class="tree-follow ${f.effect?'effect':'unlock'} ${ready?'ready':'locked'}" data-tree-target="${esc(f.key)}" data-tree-kind="${f.kind}" data-tree-level="${f.level}"><img src="${esc(f.kind==='civilization'?art('portraits',view.volk,view.volk):art(f.kind,f.key))}" alt=""><span><strong>${esc(f.kind==='civilization'?progressionRule(f.kind,f.key).name:named(f.key))}</strong><small>${esc(f.relation)}${f.effect?'':ready?' · Bedingungen erfüllt':' · Noch gesperrt'}</small></span></button>`;}).join('');
 setStableHTML(el, `<div class="tree-heading"><div><span class="eyebrow">DEIN WEG DURCH DIE TECHNIK</span><h3>${target.kind==='civilization'?esc(progressionRule(target.kind,target.key).name):esc(named(target.key))}</h3></div>${host==='progression-map'?`<label>Ziel planen<select data-tree-goal>${options.map(([kind,key])=>`<option value="${kind}:${key}" ${target.kind===kind&&target.key===key?'selected':''}>${({ships:'Schiff',defenses:'Verteidigung',research:'Forschung',buildings:'Gebäude',civilization:'Zivilisation'})[kind]} · ${esc(kind==='civilization'?progressionRule(kind,key).name:named(key))}</option>`).join('')}</select></label>`:'<button type="button" class="secondary" data-go="freischaltungen">Andere Ziele planen ↗</button>'}</div><p class="tree-explanation">Durchgezogene Pfade zeigen Voraussetzungen. ${openResearch.length?`${openResearch.length} Forschungsziele und `:''}${openBuildings.length} Gebäudeziele sind auf diesem Weg noch offen. Rohstoffe und freie Bauplätze prüfst du im Datenblatt.</p><div class="tree-legend"><span>✓ Erfüllt</span><span>● Voraussetzungen erfüllt</span><span>◇ Noch gesperrt</span><span>◷ Aktiver Timer</span></div><div class="tree-scroll" tabindex="0" aria-label="Voraussetzungsbaum, horizontal scrollbar"><div class="tree-canvas"><svg class="tree-wires" aria-hidden="true"></svg>${tiers.map((nodes,depth)=>`<div class="tree-tier" data-tier="${depth}">${nodes.map(n=>progressionNode(n,graph.targetId)).join('')}</div>`).join('')}</div></div>${followers.length?`<h4>Damit erreichst oder verbesserst du</h4><div class="tree-followers">${followHtml}</div>`:''}<p class="tree-status" role="status" aria-live="polite"></p>`);
 const finished=[...Progression.finished],before={...Progression.before};
 requestAnimationFrame(()=>{if(el.dataset.stamp!==stamp)return;drawTreeWires(host);const scroll=el.querySelector('.tree-scroll'),targetNode=[...el.querySelectorAll('[data-node-id]')].find(n=>n.dataset.nodeId===graph.targetId);if(scroll&&targetNode)scroll.scrollLeft=sameTarget?scrollLeft:Math.max(0,targetNode.getBoundingClientRect().left-el.querySelector('.tree-canvas').getBoundingClientRect().left-(scroll.clientWidth-targetNode.offsetWidth)/2);if(finished.length&&!el.closest('[hidden]'))animateResearchCompletion(host,finished,before);});
}
function drawTreeWires(host){
 const el=$(host),graph=Progression.graphs.get(host);if(!el||!graph||el.closest('[hidden]'))return;
 const canvas=el.querySelector('.tree-canvas'),svg=el.querySelector('.tree-wires'),box=canvas.getBoundingClientRect();
 svg.setAttribute('width',canvas.scrollWidth);svg.setAttribute('height',canvas.offsetHeight);svg.replaceChildren();
 const locate=id=>[...el.querySelectorAll('[data-node-id]')].find(n=>n.dataset.nodeId===id);
 for(const edge of graph.edges){const from=locate(edge.from),to=locate(edge.to);if(!from||!to)continue;const a=from.getBoundingClientRect(),b=to.getBoundingClientRect(),x1=a.right-box.left,y1=a.top+a.height/2-box.top,x2=b.left-box.left,y2=b.top+b.height/2-box.top,m=(x1+x2)/2;
  const path=document.createElementNS('http://www.w3.org/2000/svg','path');path.setAttribute('d',`M${x1},${y1} C${m},${y1} ${m},${y2} ${x2},${y2}`);path.dataset.from=edge.from;path.dataset.to=edge.to;path.classList.add('tree-path');const source=graph.nodes.find(n=>n.id===edge.from);if(source.have>=edge.requires)path.classList.add('active');svg.append(path);
  if(source.levels.length>1){const text=document.createElementNS('http://www.w3.org/2000/svg','text');text.setAttribute('x',x2-38);text.setAttribute('y',y2-6);text.classList.add('tree-edge-label');text.textContent='St. '+edge.requires;svg.append(text);}
 }
}
function progressionLater(fn,ms){const timer=setTimeout(()=>{Progression.timers.delete(timer);fn();},ms);Progression.timers.add(timer);}
function animateAlongPath(el,path,duration,reverse=false){
 const length=path.getTotalLength(),frames=Array.from({length:81},(_,i)=>{const p=path.getPointAtLength(length*(reverse?1-i/80:i/80));return {transform:`translate(${p.x}px,${p.y}px)`,offset:i/80};});
 return el.animate(frames,{duration,easing:view.volk==='krath'?'steps(40,end)':view.volk==='syntheten'?'steps(80,end)':'linear',fill:'forwards'});
}
function completionOrbit(node){
 const orbit=document.createElement('span');orbit.className='completion-orbit';orbit.setAttribute('aria-hidden','true');node.append(orbit);
 const w=orbit.clientWidth,h=orbit.clientHeight,r=view.volk==='syntheten'?3:14,path=document.createElementNS('http://www.w3.org/2000/svg','path');
 path.setAttribute('d',`M${w/2},0 H${w-r} Q${w},0 ${w},${r} V${h-r} Q${w},${h} ${w-r},${h} H${r} Q0,${h} 0,${h-r} V${r} Q0,0 ${r},0 Z`);
 const ball=document.createElement('i');ball.className='completion-ball';orbit.append(ball);animateAlongPath(ball,path,1050);
 if(view.volk==='veyari'){const companion=ball.cloneNode();companion.classList.add('companion');orbit.append(companion);animateAlongPath(companion,path,1050,true);}return orbit;
}
function animateResearchCompletion(host,keys,before=Progression.before){
 const el=$(host);if(!el||el.closest('[hidden]'))return;
 const completion=Progression.identity+':'+keys.map(k=>k+':'+view.forschung.stufen[k]).join(',');if(el.dataset.completion===completion)return;el.dataset.completion=completion;
 const nodes=[...el.querySelectorAll('[data-node-kind="research"]')].filter(n=>keys.includes(n.dataset.nodeKey)&&Progression.graphs.get(host)?.nodes.find(x=>x.id===n.dataset.nodeId)?.level<=view.forschung.stufen[n.dataset.nodeKey]);if(!nodes.length)return;
 const status=el.querySelector('.tree-status');status.textContent=keys.map(k=>`${named(k)} Stufe ${view.forschung.stufen[k]} abgeschlossen`).join(' · ');
 if(matchMedia('(prefers-reduced-motion: reduce)').matches||document.body.dataset.progressMotion==='still')return;
 for(const node of nodes){node.classList.add('research-complete');const orbit=completionOrbit(node);
  progressionLater(()=>{if(!node.isConnected)return;const required=Progression.graphs.get(host)?.nodes.find(n=>n.id===node.dataset.nodeId)?.level;for(const path of el.querySelectorAll('.tree-path'))if(path.dataset.from===node.dataset.nodeId&&required<=view.forschung.stufen[node.dataset.nodeKey]&&(before[node.dataset.nodeKey]||0)<required){
   const packet=document.createElementNS('http://www.w3.org/2000/svg','circle');packet.setAttribute('r',view.volk==='syntheten'?'3':'4');packet.classList.add('tree-packet');path.parentElement.append(packet);animateAlongPath(packet,path,650);
   progressionLater(()=>{packet.remove();const to=[...el.querySelectorAll('[data-node-id]')].find(n=>n.dataset.nodeId===path.dataset.to);if(to&&to.classList.contains('ready')){to.classList.add('newly-ready');progressionLater(()=>to.classList.remove('newly-ready'),900);}},650);
  }
  for(const follower of el.querySelectorAll('.tree-follow.unlock.ready')){const kind=follower.dataset.treeKind,key=follower.dataset.treeTarget,level=kind==='research'?rules.catalog.forschung_voraussetzungen?.[key]?.forschung?.[node.dataset.nodeKey]:['ships','defenses'].includes(kind)?rules.catalog.einheiten[key]?.taktik?.voraussetzungen?.[node.dataset.nodeKey]:progressionRule(kind,key)?.forschung?.[node.dataset.nodeKey];if(level&&(before[node.dataset.nodeKey]||0)<level&&view.forschung.stufen[node.dataset.nodeKey]>=level){follower.classList.add('newly-ready');progressionLater(()=>follower.classList.remove('newly-ready'),900);}}
  },1050);progressionLater(()=>{orbit.remove();node.classList.remove('research-complete');},2400);
 }
}
// Flottenhangar (Karl 19.10.2026: „alle Schiffe auf einen Blick … für jeden Planeten einzeln … von dort aus gleich
// Schiffe bewegen und Flüge planen … modern, nicht 3 Knöpfe und 3 Tabellen“). Oben die Gesamtflotte als Bildleiste
// (antippen filtert die Standorte), darunter je Planet und Mond eine Karte: Schiffe als Bild-Chips, antippen wählt sie,
// Menge per Schieber, Ziel per Schnellwahl (eigene Planeten/Monde, letzte Ziele) und „Senden ›“ in den
// Versandassistenten (flottenfuehrung.js, Flottenfuehrung.ausHangar). Flüge des Standorts als kurze Zeitleiste.
// CSP: keine style-Attribute; Fortschritt als <progress>.
const Hangar={wahl:{ort:'',art:'planet',schiffe:{}},filter:''};
// Weiche Trennstellen wie bauseite.js (trennbar), damit „Spionage­sonde“ statt „Spionageson|de“ bricht
const HG_FUGEN=/(synthesizer|zentrum|technik|antrieb|kreuzer|zerstörer|transporter|frachter|satellit|sonde|jäger|bomber|schiff)$/i;
const hgName=k=>esc(named(k)).replace(/[A-Za-zÄÖÜäöüß]{11,}/g,w=>{const m=w.match(HG_FUGEN);return m&&w.length-m[0].length>=4?w.slice(0,w.length-m[0].length)+'&shy;'+w.slice(w.length-m[0].length):w;});
function hangarOrte(){
 const orte=view.planeten.map(p=>({koord:p.koord,art:'planet',name:Planetenname.name(p)+' · '+named(p.zone),schiffe:p.schiffe||{},bild:`<img class="hg-kugel" src="${esc(planetArt(p))}" alt="">`}));
 const monde=view?.monde?.aktiv?view.monde.eigene||[]:[];
 for(const m of monde){const i=orte.findIndex(o=>o.koord===m.koord&&o.art==='planet');orte.splice(i<0?orte.length:i+1,0,{koord:m.koord,art:'mond',name:'Mond '+(m.name||''),schiffe:m.schiffe||{},bild:window.Mondseite?Mondseite.bild(m,'hg-kugel'):'<span class="hg-kugel">☾</span>'});}
 return orte;
}
const HANGAR_ZUSTAND={hinflug:'Hinflug',rueckflug:'Rückflug',im_orbit:'Im Orbit'};
// Geplante Rückkehr schon im Hinflug: Ankunft + Haltedauer + Rückflugdauer aus dem Kern (Funktionsprüfung 25.10.2026: die
// Rückkehrzeit des Start-Schritts war nach dem Start nirgends mehr zu sehen); Einwegflüge und „kein Rückflug“ ohne.
function hangarRueckkehr(f){if(f.zustand!=='hinflug'||f.kein_rueckflug||f.rueckflug_dauer_sekunden==null||window.Auftraege?.info?.(f.mission)?.einweg)return '';const t=f.ankunft_sekunden+(Number(f.haltedauer_sekunden)||0)+Number(f.rueckflug_dauer_sekunden);return ` · zurück Tag ${Math.floor(t/86400)+1} · ${[Math.floor(t/3600)%24,Math.floor(t/60)%60,Math.floor(t)%60].map(n=>String(n).padStart(2,'0')).join(':')}`;}
function hangarFlugEnde(f){return f.zustand==='hinflug'?f.ankunft_sekunden:f.zustand==='im_orbit'?f.orbit_ende_sekunden:f.rueckkehr_sekunden;}
function hangarZiele(o){
 const ziele=[],gesehen=new Set([o.art+o.koord]),add=(k,art,text)=>{if(!k||gesehen.has(art+k))return;gesehen.add(art+k);ziele.push({k,art,text});};
 for(const x of hangarOrte())add(x.koord,x.art,x.art==='mond'?'☾ '+x.koord:x.koord);
 try{const raw=JSON.parse(localStorage.getItem('sternenepoche-flights:'+world.world_id+':'+view.name)||'{}');for(const a of [...(raw.pins||[]),...(raw.recent||[])].slice(0,4))add(a.ziel,'planet',a.ziel+' · '+named(a.mission));}catch{}
 return ziele.slice(0,8);
}
// Flottenbestand auf einen Blick: nur Zahlen, Schiffstyp × Standort (Karl 22.10.2026: „nicht alle Bilder, nur die Zahlen
// als Tabelle, übersichtlich, um immer gleich zu sehen, was ich habe“). Zeile antippen zeigt, wo dieser Typ steht.
function hangarTabelle(orte,fl,typen){
 const spalten=orte.filter(o=>Object.values(o.schiffe).some(n=>n>0)),unterwegs={},orbit={};
 for(const f of fl)for(const [k,n] of Object.entries(f.schiffe||{}))if(n>0){const z=f.zustand==='im_orbit'?orbit:unterwegs;z[k]=(z[k]||0)+n;}
 // Gestrandete/haltende Flotten stehen im Orbit, sie fliegen nicht (Funktionsprüfung 25.10.2026: zählten als „unterwegs“).
 // Nachprüfung 25.10.2026: eine Spalte „unterwegs oder im Orbit“ mischte Flüge und gestrandete Flotten – jetzt getrennt.
 const zahl=n=>n?fmt(n):'<span class="hg-null">·</span>',mitFlug=Object.keys(unterwegs).length>0,mitOrbit=Object.keys(orbit).length>0,summeVon=o=>Object.values(o).reduce((a,b)=>a+b,0);
 const kopf=`<tr><th scope="col">Schiff</th>${spalten.map(o=>`<th scope="col"><button type="button" class="hg-start" data-hg-start="${esc(o.koord)}" data-hg-startart="${o.art}" title="Flotte ab ${esc(o.name)} planen" aria-label="Flotte ab ${o.art==='mond'?'Mond':'Planet'} [${esc(o.koord)}] versenden">${o.art==='mond'?'☾ ':''}${esc(o.koord)} <span aria-hidden="true">↗</span></button></th>`).join('')}${mitFlug?'<th scope="col">Unterwegs</th>':''}${mitOrbit?'<th scope="col">Im Orbit</th>':''}<th scope="col">Gesamt</th></tr>`;
 const zeilen=typen.map(([k,n])=>`<tr class="${Hangar.filter===k?'hg-an':''}"><th scope="row"><button type="button" class="bs-link" data-hg-filter="${esc(k)}" aria-pressed="${Hangar.filter===k}">${esc(named(k))}</button></th>${spalten.map(o=>`<td>${zahl(o.schiffe[k]||0)}</td>`).join('')}${mitFlug?`<td>${zahl(unterwegs[k]||0)}</td>`:''}${mitOrbit?`<td>${zahl(orbit[k]||0)}</td>`:''}<td class="hg-tab-summe">${fmt(n)}</td></tr>`).join('');
 const summe=typen.reduce((a,[,n])=>a+n,0);
 const fuss=`<tr><th scope="row">Summe</th>${spalten.map(o=>`<td>${fmt(Object.values(o.schiffe).reduce((a,b)=>a+(b>0?b:0),0))}</td>`).join('')}${mitFlug?`<td>${fmt(summeVon(unterwegs))}</td>`:''}${mitOrbit?`<td>${fmt(summeVon(orbit))}</td>`:''}<td class="hg-tab-summe">${fmt(summe)}</td></tr>`;
 // Einklappbar (Karl 22.10.2026); der Zustand gilt für Übersicht und Flottenkommando und bleibt gemerkt.
 let offen=true;try{offen=localStorage.getItem('sternenepoche.flottentabelle')!=='zu';}catch(_){}
 return `<details class="hg-tabelle-klapp" data-flottentabelle ${offen?'open':''}><summary>Flottenbestand · ${fmt(summe)} ${summe===1?'Schiff':'Schiffe'}${mitFlug?' · '+fmt(summeVon(unterwegs))+' unterwegs':''}${mitOrbit?' · '+fmt(summeVon(orbit))+' im Orbit':''}</summary><div class="hg-tabelle-rahmen"><table class="hg-tabelle"><thead>${kopf}</thead><tbody>${zeilen}</tbody><tfoot>${fuss}</tfoot></table></div></details>`;
}
document.addEventListener('toggle',e=>{const d=e.target;if(!d.matches?.('[data-flottentabelle]'))return;try{localStorage.setItem('sternenepoche.flottentabelle',d.open?'auf':'zu');}catch(_){}for(const x of document.querySelectorAll('[data-flottentabelle]'))if(x!==d&&x.open!==d.open)x.open=d.open;},true);
document.addEventListener('click',e=>{const b=e.target.closest('[data-hg-start]');if(!b)return;Flottenfuehrung.ausHangar({start:b.dataset.hgStart,startart:b.dataset.hgStartart,mission:'transport',direktFlotte:true});});
// Dieselbe Tabelle in der Übersicht; Zeile antippen öffnet das Flottenkommando mit diesem Typ.
function flottenTabelle(){
 if(!view)return '';const orte=hangarOrte(),fl=view.flotten||[],gesamt={};
 for(const o of orte)for(const [k,n] of Object.entries(o.schiffe))if(n>0)gesamt[k]=(gesamt[k]||0)+n;
 for(const f of fl)for(const [k,n] of Object.entries(f.schiffe||{}))if(n>0)gesamt[k]=(gesamt[k]||0)+n;
 const typen=Object.entries(gesamt).sort((a,b)=>b[1]-a[1]);
 return `<section class="uk-flotte" aria-label="Flottenbestand">${typen.length?hangarTabelle(orte,fl,typen).replaceAll('data-hg-filter=','data-go="flotten" data-hg-filter='):'<p class="hg-leer">Noch keine Schiffe. <button type="button" class="bs-link" data-tab="werft">In der Werft bauen ›</button></p>'}${verbuendeteImOrbit()}${bestandTabellen()}</section>`;
}
// Bestand je Planet wie der Flottenbestand (Karl 25.10.2026: „wie für Flotten auch für die Bodentruppen und Raketen für jeden
// Planeten sehen, wie viele Bestand sind (einklappbar) … Verteidigungsanlagen und Solarsatelliten auch!“). Zeilen = Typen,
// Spalten = Planeten (und Monde) mit Bestand; der Klappzustand gilt je Tabelle überall und bleibt gemerkt (anfangs zu).
function bestandOffen(id){try{return localStorage.getItem('sternenepoche.bestand.'+id)==='auf';}catch(_){return false;}}
document.addEventListener('toggle',e=>{const d=e.target,id=d?.dataset?.bestandtabelle;if(!id)return;try{localStorage.setItem('sternenepoche.bestand.'+id,d.open?'auf':'zu');}catch(_){}for(const x of document.querySelectorAll(`[data-bestandtabelle="${id}"]`))if(x!==d&&x.open!==d.open)x.open=d.open;},true);
function bestandTabelle(id,titel,zaehl,orte,zeilen,{go,unterwegs,extra,leer}={}){
 const wert=(z,o)=>Number(z[2](o))||0,spalten=orte.filter(o=>zeilen.some(z=>wert(z,o)>0));
 const unt=k=>Number(unterwegs?.[k])||0,summe=z=>spalten.reduce((a,o)=>a+wert(z,o),0)+unt(z[0]),aktiv=zeilen.filter(z=>summe(z)>0);
 const gesamt=aktiv.reduce((a,z)=>a+summe(z),0),knopf=go?`<button type="button" class="bs-link" data-go="${go}">${esc(leer||'Bauen')} ›</button>`:'';
 if(!aktiv.length)return `<details class="hg-tabelle-klapp hg-bestand-leer" data-bestandtabelle="${id}" ${bestandOffen(id)?'open':''}><summary>${esc(titel)} · keine</summary><p class="hg-leer">Auf keinem Planeten vorhanden. ${knopf}</p></details>`;
 const zahl=n=>n?fmt(n):'<span class="hg-null">·</span>',mitFlug=aktiv.some(z=>unt(z[0])>0);
 const kopf=`<tr><th scope="col">${esc(titel)}</th>${spalten.map(o=>`<th scope="col">${o.art==='mond'?'☾ ':''}${esc(o.koord)}</th>`).join('')}${mitFlug?'<th scope="col">Unterwegs</th>':''}<th scope="col">Gesamt</th></tr>`;
 const zeilenHtml=aktiv.map(z=>`<tr><th scope="row">${go?`<button type="button" class="bs-link" data-go="${go}">${esc(z[1])}</button>`:esc(z[1])}</th>${spalten.map(o=>`<td>${zahl(wert(z,o))}</td>`).join('')}${mitFlug?`<td>${zahl(unt(z[0]))}</td>`:''}<td class="hg-tab-summe">${fmt(summe(z))}</td></tr>`).join('');
 const fuss=`<tr><th scope="row">Summe</th>${spalten.map(o=>`<td>${fmt(aktiv.reduce((a,z)=>a+wert(z,o),0))}</td>`).join('')}${mitFlug?`<td>${fmt(aktiv.reduce((a,z)=>a+unt(z[0]),0))}</td>`:''}<td class="hg-tab-summe">${fmt(gesamt)}</td></tr>`
  +(extra?extra.map(([name,f,ges])=>`<tr class="hg-zusatz"><th scope="row">${esc(name)}</th>${spalten.map(o=>`<td>${f(o)}</td>`).join('')}${mitFlug?'<td></td>':''}<td>${ges??''}</td></tr>`).join(''):'');
 return `<details class="hg-tabelle-klapp" data-bestandtabelle="${id}" ${bestandOffen(id)?'open':''}><summary>${esc(titel)} · ${fmt(gesamt)} ${esc(zaehl(gesamt))}${mitFlug?' · '+fmt(aktiv.reduce((a,z)=>a+unt(z[0]),0))+' unterwegs':''}</summary><div class="hg-tabelle-rahmen"><table class="hg-tabelle"><thead>${kopf}</thead><tbody>${zeilenHtml}</tbody><tfoot>${fuss}</tfoot></table></div></details>`;
}
// Übersicht: wer gerade über eigenen Planeten mitverteidigt (Halten, Bodenhilfe, gestrandete Helfer).
function verbuendeteImOrbit(){
 const l=view?.verbuendete_im_orbit||[];if(!l.length)return '';
 return `<p class="hg-verbuendete-kurz"><b>Verbündete im Orbit:</b> ${l.map(x=>`${esc(x.von)} über [${esc(x.ziel)}] · ${fmt(x.anzahl)} ${x.anzahl===1?'Schiff':'Schiffe'}${x.truppen?' · '+fmt(x.truppen)+' Bodentruppen':''}`).join(' · ')} <button type="button" class="bs-link" data-go="flotten">Flottenkommando ›</button></p>`;
}
function bestandTabellen(){
 if(!view?.planeten)return '';
 const planeten=view.planeten.map(p=>({koord:p.koord,art:'planet',p})),monde=view?.monde?.aktiv?view.monde.eigene||[]:[];
 const orte=[];for(const o of planeten){orte.push(o);for(const m of monde)if(m.koord===o.koord)orte.push({koord:m.koord,art:'mond',m});}
 const vert=o=>(o.art==='mond'?o.m.verteidigung:o.p.verteidigung)||{};
 const abwehr=[...new Set(orte.flatMap(o=>Object.keys(vert(o))))].filter(k=>k!=='solarsonde').map(k=>[k,named(k),o=>vert(o)[k]]);
 const anlagen=bestandTabelle('verteidigung','Verteidigung',n=>n===1?'Anlage':'Anlagen',orte,abwehr,{go:'verteidigung'});
 const je=o=>Number((o.art==='planet'?o.p:o.m).energie?.je_solarsonde)||0,satelliten=o=>Number(vert(o).solarsonde)||0;
 const energie=orte.reduce((a,o)=>a+satelliten(o)*je(o),0);
 const solar=bestandTabelle('solarsatelliten','Solarsatelliten',n=>n===1?'Satellit':'Satelliten',orte,[['solarsonde',named('solarsonde'),satelliten]],
  {go:'verteidigung',extra:[['Energie',o=>je(o)&&satelliten(o)?'+'+fmt(Math.round(satelliten(o)*je(o))):'<span class="hg-null">·</span>',energie?'+'+fmt(Math.round(energie)):'']]});
 // Bodentruppen: Garnison je Planet, dazu Truppen an Bord eigener Flotten (wie „unterwegs“ beim Flottenbestand).
 const bt=view.planeten.find(p=>p.bodentruppen?.typen)?.bodentruppen,unterwegs={};
 for(const f of view.flotten||[])for(const [k,n] of Object.entries(f.bodentruppen||{}))if(n>0)unterwegs[k]=(unterwegs[k]||0)+n;
 const truppen=bt?bestandTabelle('bodentruppen','Bodentruppen',n=>n===1?'Einheit':'Einheiten',planeten,bt.typen.map((k,i)=>[k,bt.namen?.[i]||named(k),o=>o.p.bodentruppen?.bestand?.[i]]),{go:'truppen',unterwegs,leer:'Ausbilden'}):'';
 // Alle Raketenarten des Silos (Abfang-, Interplanetar-, Flottenraketen …) und belegte Plätze – Einsatzprobe 25.10.2026: zwei
 // Flottenraketen im Silo ergaben „Raketen · keine“, die Silo-Zeile zeigte nur die Kapazität. Werte „belegt / gesamt“, kurze
 // Beschriftung (Probe 3: die lange machte am Handy die erste Spalte so breit, dass „Gesamt“ angeschnitten war).
 const rak=o=>o.p.raketen||{},silo=o=>o.p.raketensilo||{},typen=planeten.find(o=>silo(o).typen?.length)?.p.raketensilo.typen||[];
 const raketenZeilen=typen.length?typen.map((t,i)=>[t.regel?.key||'rakete'+i,t.regel?.name||named(t.regel?.key||''),o=>silo(o).typen?.[i]?.bestand])
  :[['abfang','Abfangraketen',o=>rak(o).abfang],['interplanetar','Interplanetarraketen',o=>rak(o).interplanetar]];
 const kap=o=>Number(silo(o).kapazitaet??rak(o).kapazitaet)||0,belegt=o=>Number(silo(o).belegt)||0;
 const raketen=bestandTabelle('raketen','Raketen',n=>n===1?'Rakete':'Raketen',planeten,raketenZeilen,
  {go:'verteidigung',extra:[['Silo-Plätze',o=>kap(o)?`${fmt(belegt(o))} / ${fmt(kap(o))}`:'<span class="hg-null">·</span>',
   `${fmt(planeten.reduce((a,o)=>a+belegt(o),0))} / ${fmt(planeten.reduce((a,o)=>a+kap(o),0))}`]]});
 return `<div class="hg-bestaende">${anlagen}${solar}${truppen}${raketen}</div>`;
}
function renderFleetHangar(){
 const host=$('fleet-hangar');if(!host||!view)return;
 const orte=hangarOrte(),fl=view.flotten||[],orbit=fl.filter(f=>f.zustand==='im_orbit'),unterwegs=fl.filter(f=>f.zustand!=='im_orbit');
 const gesamt={};for(const o of orte)for(const [k,n] of Object.entries(o.schiffe))if(n>0)gesamt[k]=(gesamt[k]||0)+n;
 for(const f of fl)for(const [k,n] of Object.entries(f.schiffe||{}))if(n>0)gesamt[k]=(gesamt[k]||0)+n;
 const summe=Object.values(gesamt).reduce((a,b)=>a+b,0),typen=Object.entries(gesamt).sort((a,b)=>b[1]-a[1]);
 if(Hangar.filter&&!gesamt[Hangar.filter])Hangar.filter='';
 const w=Hangar.wahl,gewaehlt=Object.values(w.schiffe).reduce((a,b)=>a+b,0);
 const chip=(k,n,attr,an)=>`<button type="button" class="hg-chip" ${attr} aria-pressed="${!!an}" title="${esc(named(k))}"><img src="${esc(art('ships',k))}" alt=""><b>${fmt(n)}</b><small>${hgName(k)}</small></button>`;
 const karte=o=>{
  const eigene=Object.entries(o.schiffe).filter(([,n])=>n>0).sort((a,b)=>b[1]-a[1]),aktiv=w.ort===o.koord&&w.art===o.art,anzahl=eigene.reduce((a,[,n])=>a+n,0);
  const sa=f=>f.flugdetails?.startart||'planet',za=f=>f.flugdetails?.zielart||'planet';
  const fluege=fl.filter(f=>(f.start===o.koord&&sa(f)===o.art)||(f.ziel===o.koord&&za(f)===o.art));
  const wahlBlock=aktiv&&gewaehlt?`<div class="hg-wahl">${Object.entries(w.schiffe).filter(([,n])=>n>0).map(([k,n])=>`<label class="hg-menge"><img src="${esc(art('ships',k))}" alt=""><span>${hgName(k)}</span><input type="range" min="0" max="${o.schiffe[k]||0}" value="${n}" data-hg-menge="${esc(k)}" aria-label="Anzahl ${esc(named(k))}"><output>${fmt(n)}</output></label>`).join('')}<p class="hg-wohin">Wohin mit ${fmt(gewaehlt)} Schiffen?</p><div class="hg-ziele">${hangarZiele(o).map(z=>`<button type="button" class="hg-ziel" data-hg-ziel="${esc(z.k)}" data-hg-zielart="${z.art}">${esc(z.text)}</button>`).join('')}<button type="button" class="hg-ziel hg-andere" data-hg-ziel="">Anderes Ziel …</button></div></div>`:'';
  return `<article class="hg-ort${o.art==='mond'?' hg-mond':''}${aktiv&&gewaehlt?' hg-aktiv':''}" data-hg-ort="${esc(o.koord)}" data-hg-art="${o.art}"><header class="hg-ort-kopf">${o.bild}<span class="hg-ort-name"><strong>${esc(o.name)}</strong><small>[${esc(o.koord)}]${fluege.length?' · '+fluege.length+' Flug'+(fluege.length===1?'':'e'):''}</small></span><b class="hg-summe">${fmt(anzahl)}<small>Schiffe</small></b></header>`
   +(eigene.length?`<div class="hg-schiffe">${eigene.map(([k,n])=>chip(k,n,`data-hg-schiff="${esc(k)}"`,aktiv&&w.schiffe[k]>0)).join('')}</div>`:'<p class="hg-leer">Keine Schiffe stationiert.</p>')
   +wahlBlock
   +(fluege.length?`<ul class="hg-fluege">${fluege.map(f=>{const e=hangarFlugEnde(f),zurueck=f.zustand==='rueckflug',abHier=f.start===o.koord&&sa(f)===o.art,anderer=abHier?f.ziel+(za(f)==='mond'?' ☾':''):f.start+(sa(f)==='mond'?' ☾':'');return `<li><img src="${esc(art('ships',Object.entries(f.schiffe||{}).sort((a,b)=>b[1]-a[1])[0]?.[0]||'kleiner_transporter'))}" alt=""><span class="hg-flug-text"><b>${esc(named(f.mission))} #${f.flotte}</b><small>${abHier&&!zurueck?'→':'←'} [${esc(anderer)}] · ${f.gestrandet?`gestrandet · ${fmt(f.gestrandet.deuterium||0)} Deuterium an Bord`:esc(HANGAR_ZUSTAND[f.zustand]||named(f.zustand))}${hangarRueckkehr(f)}</small></span>${f.fortschritt!=null?`<progress max="100" value="${Math.round(f.fortschritt*100)}"></progress>`:'<span></span>'}<span class="hg-zeit" title="Echtzeit">${e?`<span data-arrival-echt="${e}"></span>`:''}</span>${f.gestrandet?`<button type="button" class="hg-rueck" data-es-gestrandet="${f.flotte}" aria-label="Gestrandete Flotte #${f.flotte} weiterfliegen">➚</button>`:!zurueck?`<button type="button" class="hg-rueck" data-recall="${f.flotte}" aria-label="Flotte #${f.flotte} zurückrufen" title="Zurückrufen">↩</button>`:''}</li>`;}).join('')}</ul>`:'')
   // Verbündete (Halten, Bodenhilfe) und gestrandete Helfer über diesem Planeten – verteidigen mit (Einsatzprobe 25.10.2026).
   +(()=>{const fremd=o.art==='planet'?(view.verbuendete_im_orbit||[]).filter(x=>x.ziel===o.koord):[];return fremd.length?`<ul class="hg-fluege hg-verbuendete">${fremd.map(x=>`<li><img src="${esc(art('ships',Object.entries(x.schiffe||{}).sort((a,b)=>b[1]-a[1])[0]?.[0]||'kreuzer'))}" alt=""><span class="hg-flug-text"><b>${esc(x.von)} · ${esc(x.gestrandet?'gestrandet':named(x.mission))}</b><small>verteidigt mit · ${fmt(x.anzahl)} ${x.anzahl===1?'Schiff':'Schiffe'}${x.truppen?' · '+fmt(x.truppen)+' Bodentruppen':''}</small></span><span></span><span class="hg-zeit">${x.bis_sekunden?`<span data-arrival-echt="${x.bis_sekunden}"></span>`:''}</span><span></span></li>`).join('')}</ul>`:'';})()
   +`<footer class="hg-ort-fuss">${eigene.length?`<button type="button" class="secondary" data-hg-alle>${aktiv&&gewaehlt?'Auswahl leeren':'Alle wählen'}</button>`:''}${aktiv&&gewaehlt?`<button type="button" class="hg-senden" data-hg-senden>${fmt(gewaehlt)} Schiffe senden ›</button>`:''}</footer></article>`;
 };
 const sichtbar=orte.filter(o=>!Hangar.filter||(o.schiffe[Hangar.filter]||0)>0||fl.some(f=>(f.schiffe?.[Hangar.filter]||0)>0&&(f.start===o.koord||f.ziel===o.koord)));
 setStableHTML(host, `<div class="hg-gesamt"><div class="hg-gesamt-kopf"><span><b>Flotte gesamt</b><small>${fmt(summe)} Schiffe · ${unterwegs.length} ${unterwegs.length===1?'Flug':'Flüge'} unterwegs · ${orbit.length} im Orbit${view.flugtermine?.length?' · '+view.flugtermine.length+' geplant':''}</small></span><button type="button" class="hg-planen" data-fleet-section="fleet-planner">Flug planen ›</button></div>${typen.length?hangarTabelle(orte,fl,typen):'<p class="hg-leer">Noch keine Schiffe. <button type="button" class="bs-link" data-hg-werft>In der Werft bauen ›</button></p>'}${Hangar.filter?`<p class="hg-filter-hinweis">Nur Standorte mit ${esc(named(Hangar.filter))} · <button type="button" class="bs-link" data-hg-filter="${esc(Hangar.filter)}">alle zeigen</button></p>`:''}${bestandTabellen()}</div><div class="hg-orte">${sichtbar.map(karte).join('')}</div>`);
}
document.addEventListener('click',e=>{
 if(e.target.closest('[data-hg-werft]')){navigateGame('werft');return;}
 const b=e.target.closest('[data-hg-filter],[data-hg-schiff],[data-hg-alle],[data-hg-ziel],[data-hg-senden]');if(!b||!view)return;
 const karte=b.closest('[data-hg-ort]'),ort=karte?.dataset.hgOrt,art=karte?.dataset.hgArt,o=ort&&hangarOrte().find(x=>x.koord===ort&&x.art===art),w=Hangar.wahl;
 if(b.dataset.hgFilter!=null){Hangar.filter=Hangar.filter===b.dataset.hgFilter?'':b.dataset.hgFilter;renderFleetHangar();return;}
 if(!o)return;
 if(w.ort!==o.koord||w.art!==o.art){w.ort=o.koord;w.art=o.art;w.schiffe={};}
 if(b.dataset.hgSchiff){const k=b.dataset.hgSchiff;w.schiffe[k]=w.schiffe[k]>0?0:(o.schiffe[k]||0);renderFleetHangar();return;}
 if(b.hasAttribute('data-hg-alle')){const leer=!Object.values(w.schiffe).some(n=>n>0);w.schiffe=leer?Object.fromEntries(Object.entries(o.schiffe).filter(([,n])=>n>0)):{};renderFleetHangar();return;}
 const schiffe=Object.fromEntries(Object.entries(w.schiffe).filter(([,n])=>n>0));if(!Object.keys(schiffe).length)return;
 const ziel=b.dataset.hgZiel||'',zielart=b.dataset.hgZielart||'planet',eigen=ziel&&hangarOrte().some(x=>x.koord===ziel&&x.art===zielart);
 window.Flottenfuehrung?.ausHangar?.({start:o.koord,startart:o.art,schiffe,ziel,zielart,mission:eigen?'stationieren':''});
 Hangar.wahl={ort:'',art:'planet',schiffe:{}};renderFleetHangar();
});
document.addEventListener('input',e=>{const r=e.target.closest?.('[data-hg-menge]');if(!r)return;Hangar.wahl.schiffe[r.dataset.hgMenge]=Number(r.value)||0;const out=r.parentElement.querySelector('output');if(out)out.textContent=fmt(Number(r.value)||0);});
document.addEventListener('change',e=>{if(e.target.closest?.('[data-hg-menge]'))renderFleetHangar();});
function fleetVisualCard(f){
 const entries=Object.entries(f.schiffe).filter(([,n])=>n>0),lead=entries.sort((a,b)=>b[1]-a[1])[0]?.[0]||'kleiner_transporter',end=f.zustand==='hinflug'?f.ankunft_sekunden:f.zustand==='im_orbit'?f.orbit_ende_sekunden:f.rueckkehr_sekunden;
 return `<article class="fleet-card fleet-visual" data-css="--fleet-image:url('${esc(art('ships',lead))}')"><div class="fleet-route"><strong>#${f.flotte} · ${esc(named(f.mission))}</strong><span>${esc(named(f.zustand))}</span></div><div class="fleet-course"><span>${esc(f.start)}</span><span aria-hidden="true">${f.zustand==='rueckflug'?'←':'→'}</span><span>${esc(f.ziel)}</span></div>${f.fortschritt!=null?`<div class="fleet-travel" aria-label="Flugfortschritt ${Math.round(f.fortschritt*100)} %"><span data-css="width:${Math.round(f.fortschritt*100)}%"></span></div>`:''}<p class="fleet-countdown">${f.gestrandet?`Gestrandet an [${esc(f.gestrandet.ort)}] · ${fmt(f.gestrandet.deuterium||0)} Deuterium an Bord${f.gestrandet.nur_heim?' · nur Heimflug oder Übergabe':''}`:esc(f.bis||'Im Orbit')}${end?` · <span data-arrival="${end}"></span> Spielzeit`:''}</p><div class="fleet-composition">${entries.map(([k,n])=>`<button type="button" data-help-unit="${esc(k)}" aria-label="${fmt(n)} ${esc(named(k))}, Datenblatt öffnen"><img src="${esc(art('ships',k))}" alt=""><span>${fmt(n)} <small>${esc(named(k))}</small></span></button>`).join('')}</div><details class="fleet-cargo"><summary>Fracht${f.verband?' · Verband #'+f.verband:''}</summary><p>${esc(goods(f.ladung)||'Keine Fracht')}</p></details><div class="fleet-actions">${f.gestrandet?`<button type="button" class="secondary" data-es-gestrandet="${f.flotte}">Weiterfliegen</button><button type="button" class="secondary" data-es-versorgen="${f.flotte}">Versorgen</button><button type="button" class="secondary" data-es-uebergeben="${f.flotte}">Übergeben</button>`:`<button type="button" class="secondary" data-recall="${f.flotte}">Rückrufen</button>`}${f.mission==='angriff'&&!f.verband?`<button type="button" class="secondary" data-open-group="${f.flotte}">Verband eröffnen</button>`:''}</div></article>`;
}
function installProgression(){
 $('research').insertAdjacentHTML('beforebegin','<div id="research-tree" class="research-tree"></div>');
 $('fleet-planner').insertAdjacentHTML('beforebegin','<div id="fleet-hangar"></div><div id="mining-opportunities"></div>');
}
installProgression();
document.addEventListener('game-navigation',()=>{for(const host of Progression.graphs.keys())requestAnimationFrame(()=>drawTreeWires(host));});
let treeResize;window.addEventListener('resize',()=>{clearTimeout(treeResize);treeResize=setTimeout(()=>{for(const host of Progression.graphs.keys())drawTreeWires(host);},100);});
document.addEventListener('change',e=>{if(e.target.hasAttribute('data-tree-goal')){const[kind,key]=e.target.value.split(':');Progression.target={kind,key,level:kind==='civilization'?Number(key):kind==='research'?Math.min(rules.catalog.welt.max_forschungsstufe,(view.forschung.stufen[key]||0)+1):kind==='buildings'?Math.min(rules.catalog.welt.max_gebaeudestufe,progressionHave(kind,key)+1):1};renderResearchTree('progression-map');}});
document.addEventListener('click',e=>{
 const b=e.target.closest('[data-tree-open],[data-tree-target],[data-help-unit],[data-hangar-select]');if(!b)return;
 if(b.dataset.treeTarget){Progression.target={kind:b.dataset.treeKind,key:b.dataset.treeTarget,level:Number(b.dataset.treeLevel)};navigateGame('freischaltungen');renderResearchTree('progression-map');return;}
 if(b.dataset.treeOpen){const kind=b.dataset.treeKind,key=b.dataset.treeOpen;if(kind==='civilization'){navigateGame('profil');return;}uiSelections[kind]=key;const target={research:'research',buildings:'gebaeude-content',ships:'ship-gallery',defenses:'verteidigung-content'}[kind];navigateGame({research:'forschen',buildings:'gebaeude',ships:'werft',defenses:'verteidigung'}[kind]);renderCatalog(kind,target);if(kind==='buildings'){showItemInfo(kind,key);}else{$(target).querySelector('.item-detail')?.scrollIntoView({block:'start'});}return;}
 if(b.dataset.helpUnit){PlayerHelp.codex.kind='ships';PlayerHelp.codex.key=b.dataset.helpUnit;PlayerHelp.codex.faction=view.volk;navigateGame('codex');renderCodex();return;}
 if(b.dataset.hangarSelect){const form=$('fleet'),p=view.planeten.find(p=>p.koord===b.dataset.hangarPlanet);form.elements.start.value=p.koord;form.querySelectorAll('#ship-inputs input').forEach(i=>i.value=i.dataset.map===b.dataset.hangarSelect?p.schiffe[i.dataset.map]||0:0);$('fleet-planner').open=true;renderFleetStock();$('fleet-planner').scrollIntoView({block:'start'});window.Mobile035?.schedulePreview();}
});
