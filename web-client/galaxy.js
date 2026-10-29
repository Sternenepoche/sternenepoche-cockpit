'use strict';
const GALAXY_RELATION_COLORS={eigen:'#4cf58a',verbuendet:'#48bd9b',nap:'#f3d365',feindlich:'#ff6470',fremde_allianz:'#ff6470',neutral:'#ffffff',unbekannt:'#cbd4e3'};
const GALAXY_RELATION_NAMES={eigen:'Eigene Welt',verbuendet:'Verbündet / eigene Allianz',nap:'Nichtangriffspakt',feindlich:'Feindlich',fremde_allianz:'Fremde Allianz ohne Beziehung',neutral:'Neutral',unbekannt:'Unbekannt'};
// Coordinates are public. Display layout and orbit motion never reveal world state
// and never enter the server's distance or flight calculations.
const GalaxyModel={
 galaxyPosition(sector,system,sectors,systems){
  const fraction=(system-.5)/systems,r=60+170*Math.sqrt(fraction);
  const angle=(sector-1)/sectors*Math.PI*2+((system*.61803398875)%1-.5)*Math.PI*2/sectors*.92+.9*(1-r/230);
  return {x:Math.cos(angle)*r,y:Math.sin(system*7.19+sector)*(5+8*(1-fraction)),z:Math.sin(angle)*r};
 },
 position(sector,system,count){
  const angle=system*2.3999632297+sector*1.3,r=23+112*Math.sqrt(system/count);
  return {x:Math.cos(angle)*r,y:Math.sin(system*7.19+sector)*13,z:Math.sin(angle)*r};
 },
 orbit(position,seconds){
  const radius=23+position*10.5,angle=position*2.3999632297+seconds*.095/Math.sqrt(position);
  return {x:radius*Math.cos(angle),y:0,z:radius*Math.sin(angle),radius};
 },
 planets(system,sector,count){
  return Array.from({length:count},(_,i)=>{
   const position=i+1,raw=(system.plaetze||system.planeten||[]).find(p=>Number(p.position||p.koord?.split(':')[2])===position);
   const coord=`${sector}:${system.system}:${position}`;
   if(!raw?.bekannt)return {koord:coord,position,bekannt:false,status:'unbekannt',/* öffentlich: Zone nach Sonnenabstand, Sternkarten-Mond */habitabilitaet:raw?.habitabilitaet,nebel:raw?.nebel??system.nebel??null,/* Kern: natürlicher Xenokristall ist bekannt, sobald das System erfasst ist – auch für unerforschte Plätze */xeno_vorkommen:raw?.xeno_vorkommen,...(raw?.mond?{mond:raw.mond}:{}),...(raw?.verwuestet?{verwuestet:true}:{}),...(raw?.orbitflotten?{orbitflotten:raw.orbitflotten}:{}),...(raw?.besiedlung_bekannt?{besiedlung_bekannt:true,bewohnt:raw.bewohnt,spieler:raw.spieler,beziehung:raw.beziehung}:{})};
   return {...raw,koord:coord,position,nebel:raw.nebel??system.nebel??null};
  });
 },
 relation(p){
  if(view?.planeten?.some(own=>own.koord===p.koord))return 'eigen';
  if(!p.spieler)return 'unbekannt';
  const r=relationFor(p.spieler,p.beziehung);
  if(r?.karte_status)return r.karte_status;
  if(['verbuendet','nap','feindlich'].includes(r?.status))return r.status;
  return r?.allianz&&r.status==='neutral'?'fremde_allianz':'neutral';
 },
 inhabitants(places){
  const residents=new Map();
  for(const p of places){
   if(!p.spieler||!(p.bekannt||p.besiedlung_bekannt))continue;
   if(!residents.has(p.spieler))residents.set(p.spieler,{name:p.spieler,relation:this.relation(p),tag:relationFor(p.spieler,p.beziehung)?.allianz_tag||'',alliance:relationFor(p.spieler,p.beziehung)?.allianz||'',coords:[]});
   const resident=residents.get(p.spieler);if(!resident.coords.includes(p.koord))resident.coords.push(p.koord);
  }
  return [...residents.values()];
 },
 summary(places,expected=rules.catalog.welt.plaetze_je_system){
  const inhabitants=this.inhabitants(places),order=['eigen','feindlich','fremde_allianz','nap','verbuendet','neutral'];
  inhabitants.sort((a,b)=>order.indexOf(a.relation)-order.indexOf(b.relation));
  const relation=order.find(r=>inhabitants.some(p=>p.relation===r))||'unbekannt';
  const surveyed=places.filter(p=>p.bekannt).length,settled=places.some(p=>(p.bekannt||p.besiedlung_bekannt)&&(p.spieler||p.bewohnt));
  const state=settled?'occupied':surveyed>=expected?'empty':places.some(p=>p.bekannt||p.besiedlung_bekannt)?'partial':'unknown';
  return {inhabitants,relation,state,surveyed,color:GALAXY_RELATION_COLORS[relation]};
 }
};
// Browser scene. Loaded once; renderer and resources are disposed on world change.
window.GalaxyUI=(()=>{
 let host,canvas,renderer,scene,camera,group,raycaster,glow,observer,frameId=0,active=false,dirty=true,inFrame=false,imBild=true;
 let relationStamp='';
 // Handy (< 768 px): die 3D-Ansicht ist die Galaxie-Seite – Steuerung als Overlay, Planetenfeld unten statt Dialog.
 const handy=matchMedia('(max-width: 767px)');
 // Kein requestAnimationFrame (ruht in verdeckten Fenstern, läuft sonst dauernd): gezeichnet wird per Zeitgeber nur
 // nach Eingaben, geladenen Texturen oder solange im Sonnensystem etwas umläuft – und nie, wenn die Karte nicht im Bild ist.
 // Bildtakt: requestAnimationFrame (glatt, bildschirmsynchron); ruht der (verdecktes Fenster), springt nach 150 ms ein
 // Zeitgeber ein. Gezeichnet wird nur, solange die Karte sichtbar ist und sich etwas bewegt.
 let rafId=0,lief=false,versatz=0,filmModus=false;
 // Bedienung (Ziehen, Zoomen, Kameraflug, Schwung, Zeiger über der Karte) und Weltenkrümmer-Abläufe zeichnen jedes Bild,
 // sonst 30 Bilder/s (aktivBis: bis 1,5 s nach der letzten Eingabe).
 let letztesBild=0,sparModus=false,aktivBis=0;
 function bedient(){aktivBis=performance.now()+1500;}
 function sparen(t){return !filmModus&&t>aktivBis&&!pointers.size&&!fliegt&&Math.abs(schwung.yaw)+Math.abs(schwung.pitch)<=2e-3&&!pendingHover&&!weltenkruemmer.length&&!sys3?.felder?.some(f=>f.wachsen!=null);}
 function planen(){if(frameId||filmModus)return;const lauf=()=>{if(!frameId)return;clearTimeout(frameId);cancelAnimationFrame(rafId);frameId=0;rafId=0;frame(performance.now());};rafId=requestAnimationFrame(lauf);frameId=setTimeout(lauf,150);}
 function wake(){if(frameId||inFrame||!active||!renderer||document.hidden||!imBild)return;planen();}
 function neu(){dirty=true;wake();}
 function ruhe(){clearTimeout(frameId);cancelAnimationFrame(rafId);frameId=0;rafId=0;last=0;}
 let stage='galaxy',sector=1,system=1,selected=0,clock=0,last=0,stamp='',epoch=0,paused=false,motionOverride=false;
 const ownerMarks=new Map(),ownerPalette=['#72cfff','#ffbb70','#c9a0ff','#78e0c5','#f596ca','#e4df79','#9bb8ff','#dfad8b','#91da84','#f29b91','#91b2c8','#e9d0e0'];let selectedOwner='';
 function prepareOwners(places){for(const r of GalaxyModel.inhabitants(places).sort((a,b)=>a.name.localeCompare(b.name,'de'))){if(!ownerMarks.has(r.name)){let n=ownerMarks.size,code='';do{code=String.fromCharCode(65+n%26)+code;n=Math.floor(n/26)-1;}while(n>=0);ownerMarks.set(r.name,{code,color:ownerPalette[ownerMarks.size%ownerPalette.length]});}ownerMarks.get(r.name).relation=r.relation;}}
 const friendly=r=>r==='eigen'||r==='verbuendet';
 const friendlyColor={eigen:'#64f5a0',verbuendet:'#65d9ff'};
 function systemColor(mark){if(mark.state==='unknown')return '#9badc1';if(mark.state==='partial')return '#d2b179';if(mark.state==='empty')return '#b0c7d5';if(mark.inhabitants.some(r=>r.relation==='eigen'))return friendlyColor.eigen;if(mark.inhabitants.some(r=>r.relation==='verbuendet'))return friendlyColor.verbuendet;if(mark.inhabitants.some(r=>r.relation==='feindlich'))return '#ff7e88';if(mark.inhabitants.some(r=>r.relation==='nap'))return '#f3d365';return '#edf3fa';}
 function mapLegend(){
  const legend=document.createElement('section');legend.id='galaxy-map-legend';legend.setAttribute('aria-label','Kartenlegende');$g('galaxy-viewport').after(legend);
  legend.innerHTML=`<strong>Kartenlegende</strong><div class="space-legend-key"><span><i class="map-key-unknown">●</i> Unerforscht</span><span><i class="map-key-partial">●</i> Teilweise erkundet</span><span><i class="map-key-empty">○</i> Erkundet · unbewohnt</span><span><i class="map-key-neutral">●</i> Neutral besiedelt</span><span><i class="map-key-own">◇</i> Eigene Kolonie</span><span><i class="map-key-ally">◯</i> Allianz / verbündet</span><span><i class="map-key-enemy">△</i> Feindlich</span><span><i class="map-key-nap">●</i> Nichtangriffspakt</span><span><i class="map-key-debris">✧</i> Trümmer · direkt bergen</span></div><details><summary>Gemischte Systeme & Erkundungsstand</summary><p>Die Zeichen beziehen sich auf bekannte Bewohner: grüne Raute für dich, blauer Ring für deine Allianz oder Verbündete, rotes Dreieck für Feinde. Sie können am selben System gemeinsam erscheinen. Eine fremde Allianz ohne Abkommen gilt hier als neutral.</p><p>Ein leerer Ring bedeutet: alle Planeten erkundet und keine Besiedlung bekannt. Ein grauer Stern ist unerforscht; ein goldener Stern ist teilweise erkundet, ohne bisher bekannte Bewohner. Besiedlung kann bereits bekannt sein, obwohl einzelne Planeten noch nicht untersucht sind. Die Anzeige beschreibt deinen letzten Kenntnisstand.</p><p>Alle Systeme bleiben eingezeichnet. Beim Herauszoomen werden nur Beschriftungen ausgedünnt. Eigene und verbündete Positionen behalten ihre Zeichen und sind über die Kolonielisten oberhalb der Karte erreichbar. Heranzoomen trennt eng beieinanderliegende Sterne.</p><p>Buchstaben identifizieren einzelne Besitzer; Bahnfarben unterstützen die Zuordnung und können sich wiederholen. Über die Suche und die Seitentasten erreichst du jeden bekannten Spieler. Ein Name hebt seine bekannten Positionen hervor.</p></details>`;
 }
 let beaconGroups=[],sectorArcBatch=null,ownerDirectory=null,friendlyDirectory=null,sectorNumberView=false;
 function hasFriendly(names){return names.some(n=>friendly(ownerMarks.get(n)?.relation));}
 // Two batched screen-sized symbols remain visible independently of text culling.
 function addFriendlyBeacons(){
  for(const relation of ['eigen','verbuendet','feindlich','empty']){
   const entries=labels.filter(l=>relation==='empty'?l.kind==='system'&&l.state==='empty':l.relations?.includes(relation));if(!entries.length)continue;
   const art=document.createElement('canvas');art.width=art.height=64;const ctx=art.getContext('2d');ctx.strokeStyle=friendlyColor[relation]|| (relation==='feindlich'?'#ff7e88':'#b0c7d5');ctx.lineWidth=relation==='empty'?9:5;ctx.shadowColor='#020813';ctx.shadowBlur=5;ctx.beginPath();
   if(relation==='eigen'){ctx.moveTo(32,5);ctx.lineTo(59,32);ctx.lineTo(32,59);ctx.lineTo(5,32);ctx.closePath();}else if(relation==='feindlich'){ctx.moveTo(49,6);ctx.lineTo(59,22);ctx.lineTo(39,22);ctx.closePath();}else ctx.arc(32,32,24,0,Math.PI*2);ctx.stroke();
   const geometry=new THREE.BufferGeometry();geometry.setAttribute('position',new THREE.Float32BufferAttribute(new Float32Array(entries.length*3),3));
   const points=new THREE.Points(geometry,new THREE.PointsMaterial({map:new THREE.CanvasTexture(art),size:relation==='empty'?7:relation==='eigen'?10:relation==='verbuendet'?14:18,sizeAttenuation:false,transparent:true,depthTest:false,depthWrite:false}));points.renderOrder=20;group.add(points);beaconGroups.push({points,entries,relation});
  }
 }
 const focus={x:0,y:0,z:0},pointers=new Map();let gesture=null,pendingHover=null,viewportHeight=430;
 let cache=new Map(),targets=[],labels=[],planets=[],moons=[],loecher=[],weltenkruemmer=[],spin=0,orbits=[],sectorStars=[],yaw=.15,pitch=.72,distance=600,drag=null,hovered='';
 const $g=id=>document.getElementById(id),reduced=matchMedia('(prefers-reduced-motion: reduce)');
 const palette={sun:['#ffb52e','#ffe09a','#fff8dd'],unknown:['#69778b','#8192a8','#a4b4c7'],leben:['#1b4e80','#76a56b','#cad7b4'],glut:['#422d30','#bd613d','#eda758'],frost:['#386081','#b3dce3','#f4f9ff'],nebel:['#443775','#9478ae','#d1b4df']};
 const own=c=>view?.planeten?.some(p=>p.koord===c),profile=()=>rules.catalog.welt;
 function disposeTree(tree){const textures=new Set(),materials=new Set(),geometries=new Set();tree.traverse(o=>{if(o.geometry)geometries.add(o.geometry);for(const m of [].concat(o.material||[])){materials.add(m);if(m.map)textures.add(m.map);}});geometries.forEach(x=>x.dispose());materials.forEach(x=>x.dispose());textures.forEach(x=>{if(x!==glow)x.dispose();});}
 function texture(){const c=document.createElement('canvas');c.width=c.height=128;const x=c.getContext('2d'),g=x.createRadialGradient(64,64,1,64,64,64);g.addColorStop(0,'#fff');g.addColorStop(.12,'rgba(255,255,255,.85)');g.addColorStop(.4,'rgba(255,255,255,.14)');g.addColorStop(1,'rgba(255,255,255,0)');x.fillStyle=g;x.fillRect(0,0,128,128);return new THREE.CanvasTexture(c);}
 function dotTexture(){const c=document.createElement('canvas');c.width=c.height=32;const ctx=c.getContext('2d');ctx.fillStyle='#fff';ctx.beginPath();ctx.arc(16,16,13,0,Math.PI*2);ctx.fill();return new THREE.CanvasTexture(c);}
 function sprite(color,size,opacity=1){const m=new THREE.Sprite(new THREE.SpriteMaterial({map:glow,color,transparent:true,opacity,depthWrite:false,blending:THREE.AdditiveBlending}));m.scale.setScalar(size);return m;}
 function stars(){
  const pos=[],colors=[];let seed=41;const random=()=>{seed=(seed*1664525+1013904223)>>>0;return seed/4294967296;};
  for(let i=0;i<2400;i++){const a=random()*Math.PI*2,r=700+random()*750,y=(random()-.5)*1400;pos.push(Math.cos(a)*r,y,Math.sin(a)*r);const c=new THREE.Color(i%3?'#94b9dd':'#ffd2a0');colors.push(c.r,c.g,c.b);}
  const g=new THREE.BufferGeometry();g.setAttribute('position',new THREE.Float32BufferAttribute(pos,3));g.setAttribute('color',new THREE.Float32BufferAttribute(colors,3));scene.add(new THREE.Points(g,new THREE.PointsMaterial({size:1.4,vertexColors:true,transparent:true,opacity:.6})));
 }
 function cloud(cx,cz,scale,color,seed){
  const pos=[];for(let i=0;i<3200;i++){const a=i*2.3999632297+seed,r=Math.sqrt((i+.5)/3200)*145*scale,arm=Math.sin(a*3+r*.026);pos.push(cx+Math.cos(a+r*.005)*r,Math.sin(i*17.3)*18*scale,cz+Math.sin(a+r*.005)*r+arm*6*scale);}
  const g=new THREE.BufferGeometry();g.setAttribute('position',new THREE.Float32BufferAttribute(pos,3));group.add(new THREE.Points(g,new THREE.PointsMaterial({map:glow,size:8*scale,color,transparent:true,opacity:.32,depthWrite:false,blending:THREE.AdditiveBlending})));
  const haze=sprite(color,350*scale,.5);haze.position.set(cx,-20,cz);group.add(haze);const core=sprite('#c4b8ee',80*scale,.65);core.position.set(cx,0,cz);group.add(core);
 }
 function planetTexture(zone,seed){
  const c=document.createElement('canvas');c.width=512;c.height=256;const x=c.getContext('2d'),image=x.createImageData(c.width,c.height),colors=(palette[zone]||palette.frost).map(v=>new THREE.Color(v).convertLinearToSRGB());
  for(let y=0;y<c.height;y++)for(let px=0;px<c.width;px++){
   const lon=px/c.width*Math.PI*2,lat=(y/c.height-.5)*Math.PI,a=Math.cos(lon)*Math.cos(lat),b=Math.sin(lon)*Math.cos(lat),d=Math.sin(lat);
   const n=Math.sin(a*7+seed)*Math.cos(b*8-seed)+Math.sin(b*17+d*13)*.3+Math.cos(a*28-d*23)*.13;
   let color=n>.2?colors[1]:colors[0];if(zone==='frost'&&Math.abs(d)>.65||zone==='leben'&&Math.abs(d)>.87)color=colors[2];
   const detail=.88+.12*Math.sin(n*20+seed),i=(y*c.width+px)*4;image.data[i]=Math.round(color.r*255*detail);image.data[i+1]=Math.round(color.g*255*detail);image.data[i+2]=Math.round(color.b*255*detail);image.data[i+3]=255;
  }
  x.putImageData(image,0,0);const map=new THREE.CanvasTexture(c);map.colorSpace=THREE.SRGBColorSpace;return map;
 }
 // Echte Oberflächenkarte (dieselbe wie in Übersicht und Tabellen, planet-kugel.js); Rauschtextur nur als Rückfall.
 const textureLoader=new THREE.TextureLoader();
 const karten=new Map();
 function surfaceTexture(category,key,volk,fallback){
  const url=window.PlanetKugel?.karte(category,key,volk||'');if(!url)return fallback();
  if(karten.has(url))return karten.get(url);
  const map=textureLoader.load(url,()=>{neu();});karten.set(url,map);map.colorSpace=THREE.SRGBColorSpace;map.wrapS=THREE.RepeatWrapping;map.anisotropy=4;return map;
 }
 // Hyperraumloch: wirbelnde Scheibe am Mond, an dem das Loch endet (Mondgeneration, Hyperraumtor-Generator).
 function lochTextur(){const c=document.createElement('canvas');c.width=c.height=128;const x=c.getContext('2d');const g=x.createRadialGradient(64,64,4,64,64,62);g.addColorStop(0,'rgba(255,255,255,1)');g.addColorStop(.18,'rgba(190,150,255,.95)');g.addColorStop(.5,'rgba(90,120,255,.45)');g.addColorStop(1,'rgba(40,20,120,0)');x.fillStyle=g;x.fillRect(0,0,128,128);x.strokeStyle='rgba(220,200,255,.85)';x.lineWidth=2.4;for(let arm=0;arm<4;arm++){x.beginPath();for(let t=0;t<1;t+=.02){const a=arm*Math.PI/2+t*Math.PI*2.2,r=6+t*54;x.lineTo(64+Math.cos(a)*r,64+Math.sin(a)*r);}x.stroke();}const map=new THREE.CanvasTexture(c);map.colorSpace=THREE.SRGBColorSpace;return map;}
 function lochMarker(data){
  const l=data.hyperraumloch;if(!l||(l.offen_bis!=null&&l.offen_bis<=GameTime.now()))return;
  const hier=[l.von,l.nach].find(k=>k&&String(k).split(':').slice(0,2).join(':')===sector+':'+system),pos=Number(String(hier||'').split(':')[2]);
  const planet=planets.find(m=>Number(m.userData.data.position)===pos);if(!planet)return;
  const sp=new THREE.Sprite(new THREE.SpriteMaterial({map:lochTextur(),color:'#ffffff',transparent:true,depthWrite:false,blending:THREE.AdditiveBlending}));sp.scale.setScalar(8.5);sp.userData={kind:'loch',key:l.id};group.add(sp);targets.push(sp);
  const moon=moons.find(m=>m.planet===planet),tor=moon?.tor||null;
  // Endet das Tor an einem Hyperraumtor-Generator, öffnet es sich im Ring (Wirbel wie beim Weltenkrümmer, klein); die
  // Scheibe bleibt unsichtbar als Antippziel und Anker der Beschriftung.
  if(tor){sp.material.opacity=0;if(!tor.loch){const h=hyperloch('#7fb8ff');h.objekt.scale.setScalar(tor.R*.86*2/46.8);tor.g.add(h.objekt);tor.loch={h,t0:performance.now()};}}
  loecher.push({sprite:sp,planet,moon,tor,bis:l.offen_bis});
  label('Hyperraumloch nach ['+String(l.ziel||'').split(':').slice(0,2).join(':')+']',sp,l.id,'loch');
  const el=labels[labels.length-1].el;el.querySelector('.space-number').textContent='◎ Loch → ['+String(l.ziel||'').split(':').slice(0,2).join(':')+']';el.dataset.lochReisen=l.id;el.setAttribute('aria-label','Hyperraumloch nach '+l.ziel+' – Durchreise planen');el.title='Hyperraumloch: sofort ins Zielsystem, ohne Flugzeit. Jeder mit Planet, Mond oder Orbitflotte in diesem oder im Zielsystem darf durch, in beide Richtungen.';
 }
 // Hyperraumtor-Generator (Karl 22.10.2026: „das Sprungtor soll Hyperraumtor-Generator heißen und neben dem Mond
 // entstehen“): Ringstation auf der Mondbahn kurz hinter dem Mond, die Öffnung entlang der Bahn, vier Pylonen mit
 // Positionslichtern. Kalt-blau leuchtend, wenn bereit (voll versorgt, abgekühlt), sonst gedimmt bernstein. Kartendaten:
 // mond.sprungtor (eigen oder verbündet), Zustand aus view.monde.eigene[].sprungtor.
 const Z_ACHSE=new THREE.Vector3(0,0,1),torTangente=new THREE.Vector3(),torRoll=new THREE.Quaternion();
 function torBauen(m,p){
  const mr=m.mesh.geometry.parameters.radius,R=Math.max(1.1,mr*.62),g=new THREE.Group();
  const em=(view?.monde?.eigene||[]).find(x=>x.koord===p.koord||x.planet===p.koord),bereit=em?.sprungtor?!!em.sprungtor.bereit:true;
  const metall=new THREE.MeshStandardMaterial({color:'#8f9cad',metalness:.72,roughness:.36,emissive:'#22303f',emissiveIntensity:.5});
  const ring=new THREE.Mesh(new THREE.TorusGeometry(R,R*.12,10,56),metall);ring.userData={kind:'tor',key:p.position,data:p,radius:R};g.add(ring);targets.push(ring);
  const farbe=new THREE.Color(bereit?'#7fd6ff':'#ffb35a').multiplyScalar(bereit?2.4:1.2);
  const innen=new THREE.Mesh(new THREE.TorusGeometry(R*.84,R*.03,6,64),new THREE.MeshBasicMaterial({color:farbe,transparent:true,opacity:.9,blending:THREE.AdditiveBlending,depthWrite:false}));g.add(innen);
  const pylon=new THREE.BoxGeometry(R*.18,R*.18,R*.46);
  for(let i=0;i<4;i++){const w=i*Math.PI/2+Math.PI/4,b=new THREE.Mesh(pylon,metall);b.position.set(Math.cos(w)*R*1.14,Math.sin(w)*R*1.14,0);b.rotation.z=w;g.add(b);const l=sprite(farbe,R*.55,.9);l.position.set(Math.cos(w)*R*1.3,Math.sin(w)*R*1.3,0);g.add(l);}
  const schein=sprite(farbe,R*2.6,bereit?.22:.1);g.add(schein);
  group.add(g);m.tor={g,ring,innen,schein,R,bereit,d:Math.min(.9,(mr*2.4+R*1.6)/m.bahn),loch:null};
  label('Hyperraumtor-Generator',ring,p.position,'tor');const el=labels[labels.length-1].el;el.querySelector('.space-number').textContent='⌾ Hyperraumtor-Generator';
  el.setAttribute('aria-label','Hyperraumtor-Generator am Mond ['+p.koord+'] – Zustand und Bedienung');el.title='Hyperraumtor-Generator: öffnet ein Hyperraumtor zu einem anderen Generator – sofort ins Zielsystem, ohne Flugzeit.';
 }
 function torBewegen(m,a,c,u){const t=m.tor,a2=a-t.d,jetzt=performance.now();
  t.g.position.copy(c).addScaledVector(u.e1,Math.cos(a2)*m.bahn).addScaledVector(u.e2,Math.sin(a2)*m.bahn);
  torTangente.copy(u.e1).multiplyScalar(-Math.sin(a2)).addScaledVector(u.e2,Math.cos(a2)).normalize();
  t.g.quaternion.setFromUnitVectors(Z_ACHSE,torTangente).multiply(torRoll.setFromAxisAngle(Z_ACHSE,jetzt/5200));
  t.innen.material.opacity=(t.bereit?.7:.35)+.25*Math.sin(jetzt/650);}
 // Offenes Tor im Generator: öffnet sich in ~2,7 s (Vorbote, Blitz, Wirbel) und bleibt dann als ruhiger Wirbel stehen.
 function torLoch(t,offen){const l=t.loch;if(!l)return;l.h.objekt.visible=offen;if(!offen)return;const jetzt=performance.now();l.h.setze(Math.min(-1.99,-6+(jetzt-l.t0)/1000*1.45),jetzt/1000);t.schein.material.opacity=.4;}
 function torFeld(p){offenesFeld={kind:'tor',key:p.position};const m=p.mond||{},eigen=own(p.koord)||m.eigen,k=esc(p.koord),em=(view?.monde?.eigene||[]).find(x=>x.koord===p.koord||x.planet===p.koord),t=em?.sprungtor;
  const l=(view?.monde?.hyperraumloecher||[]).find(x=>(x.von===p.koord||x.nach===p.koord)&&(x.schliesst==null||x.schliesst>GameTime.now()));
  feldText('Hyperraumtor-Generator',{eyebrow:'MOND '+(m.name?m.name+' ':'')+'· ['+p.koord+']',symbol:'⌾'},eigen?'Dein Hyperraumtor-Generator':'Hyperraumtor-Generator von '+(m.besitzer||'einem Verbündeten'),
   [t?['Stufe',String(t.stufe)]:null,t?['Zustand',t.bereit?'Bereit – ein Tor kann geöffnet werden':'Nicht bereit – lädt oder Mond nicht voll versorgt']:null,t?['Reichweite',fmt(t.reichweite_systeme)+' Systeme']:null,t?['Kapazität je Tor',fmt(t.kapazitaet)+' Struktur']:null,
    l?['Offenes Tor','nach ['+(l.von===p.koord?l.nach:l.von)+'] · schließt in '+GameTime.duration(Math.max(0,(l.schliesst??GameTime.now())-GameTime.now()))]:null],
   [eigen?knopf(`data-mond-oeffnen="${k}"`,'Generator bedienen',true):'',knopf('data-g3-hinfliegen="tor:'+p.position+'"','Hinfliegen')]);}
 // Weltenkrümmer im Sonnensystem (Karl, 18.10.2026)
 //  * Flug mit Mission mond_zerstoeren ins gezeigte System: Austritt aus dem Hyperraum, Abbremsen, Aufklappen, Laden,
 //    3 s Strahl auf den echten Mond (gleicher Typ/gleiche Größe), Schmelzen, 3 s später Blitz in Strahlfarbe und Zerfall.
 //    Alles ist an die Ankunftszeit gebunden (der Bruch fällt auf die Ankunft) – ein Neuaufbau der Ansicht setzt nahtlos fort.
 //  * Andere Flüge ins System: Anflug in Planetenform, am Ziel Verwandlung.
 //  * An einem eigenen Planeten stationiert (Werft: Vorräte/Reparatur): kreist dort wie ein Mond und klappt in Abständen auf.
 function weltenkruemmerMarker(){
  if(!window.Weltenkruemmer)return;const praefix=sector+':'+system+':',meineEpoche=epoch,jetzt=GameTime.now();
  const anlegen=(art,daten)=>window.Weltenkruemmer.instanz(daten.volk||view.volk,'klein').then(inst=>{if(!inst||meineEpoche!==epoch||!group)return;inst.objekt.scale.setScalar(1.05);/* eigene Punktlichter des Modells aus: jede neue Lichtzahl zwingt alle beleuchteten Materialien zum Neuübersetzen (D3D/ANGLE: Sekunden bis Minuten) – das Nachleuchten trägt den Glanz */inst.objekt.traverse(o=>{if(o.isLight)o.visible=false;});group.add(inst.objekt);{const hit=new THREE.Mesh(new THREE.SphereGeometry(7,10,8),new THREE.MeshBasicMaterial({visible:false}));hit.userData={kind:'wk',key:weltenkruemmer.length};inst.objekt.add(hit);targets.push(hit);}weltenkruemmer.push({inst,art,...daten});neu();});
  for(const f of view?.flotten||[]){
   if(!((f.schiffe?.weltentruemmer||0)>0)||!String(f.ziel||'').startsWith(praefix))continue;
   const an=Number(f.ankunft_sekunden)||jetzt;
   if(f.mission==='mond_zerstoeren'&&(f.zustand!=='rueckflug'||(jetzt-an)/Math.max(1,world?.tempo||1)<25))anlegen('zerstoerung',{f});
   else if(f.zustand!=='rueckflug')anlegen('flug',{f});
  }
  for(const p of view?.planeten||[]){
   if(!String(p.koord||'').startsWith(praefix)||!((p.schiffe?.weltentruemmer||0)>0))continue;
   anlegen('station',{koord:p.koord,phase:(window.PlanetKugel?.streuung?.(p.koord+'wk')||0)%628/100});
  }
  // Fremde Weltenkrümmer: bekannt aus dem letzten Spionagebericht (bzw. den Systemdaten), in der Farbe ihres Volks
  for(const m of planets){const d=m.userData.data;if(!d?.bekannt||view?.planeten?.some(o=>o.koord===d.koord))continue;
   const ber=(view?.berichte||[]).filter(b=>b.ziel===d.koord).sort((a,b)=>(b.zeit||0)-(a.zeit||0))[0];
   if(Number(d.weltenkruemmer||ber?.schiffe?.weltentruemmer||0)>0)anlegen('station',{koord:d.koord,volk:d.volk||ber?.volk,phase:(window.PlanetKugel?.streuung?.(d.koord+'wk')||0)%628/100});}
 }
 // Bahnen (Karl, 19.10.): Abstände aus der Reichweite jedes Planeten (planetWeite), damit sich Nachbarn nie berühren; der
 // Asteroidengürtel hat ein eigenes Band.
 function bahnRadius(pos){return sys3?.bahnen?.[pos]??GalaxyModel.orbit(pos,0).radius;}
 // Karl, 19.10.: Ringe, Trümmerfelder, Monde und stationierte Weltenkrümmer liegen (fast) waagrecht in der Äquatorebene ihres
 // Planeten (Achse leicht geneigt, fest im Raum). Monde laufen außerhalb von Trümmerfeld und Ring, Stationen außerhalb der
 // Mondbahn – nichts schneidet sich. Die Bahnabstände richten sich nach dieser Reichweite. Alles läuft rechtläufig (Drehsinn
 // der Planetenbahnen: −y).
 const RING_INNEN=1.5,RING_AUSSEN=2.25,FELD_INNEN=1.35,FELD_AUSSEN=2.15;
 function mondRadius(m,radius){const g=handy.matches?1.2:1;return Math.max(1.3*g,Math.min(2.6*g*(handy.matches?1.3:1),radius*((handy.matches?.3:.18)+(m.groesse||m.durchmesser||0)/90000)));}
 function mondBahn(radius,mr){return radius*2.3+mr+2;}
 function stationBahn(radius,mb,mr){return (mb?mb+mr:radius*RING_AUSSEN)+7;}
 function hatStation(p){const eig=view?.planeten?.find(o=>o.koord===p.koord);if(eig)return (eig.schiffe?.weltentruemmer||0)>0;const ber=(view?.berichte||[]).filter(b=>b.ziel===p.koord).sort((a,b)=>(b.zeit||0)-(a.zeit||0))[0];return Number(p.weltenkruemmer||ber?.schiffe?.weltentruemmer||0)>0;}
 function planetWeite(p,radius){let w=radius;if(p.rohstoffring)w=Math.max(w,radius*RING_AUSSEN);if((view?.truemmer||[]).some(t=>t.koord===p.koord))w=Math.max(w,radius*FELD_AUSSEN);
  let mb=0,mr=0;if(p.mond?.typ){mr=mondRadius(p.mond,radius);mb=mondBahn(radius,mr);w=Math.max(w,mb+mr+1);}if(hatStation(p))w=Math.max(w,stationBahn(radius,mb,mr)+5);return w;}
 function systemRand(){return sys3?.rand??(GalaxyModel.orbit(profile().plaetze_je_system,0).radius+22);}
 const achsR=new THREE.Vector3(),achsT=new THREE.Vector3(),OBEN=new THREE.Vector3(0,1,0);
 function bahnPunkt(pos,uhr,ziel){const q=GalaxyModel.orbit(pos,uhr),f=bahnRadius(pos)/q.radius;return (ziel||new THREE.Vector3()).set(q.x*f,0,q.z*f);}
 function achsen(p){achsR.set(p.x,0,p.z);if(achsR.lengthSq()<1e-6)achsR.set(1,0,0);achsR.normalize();achsT.set(-achsR.z,0,achsR.x);return {r:achsR,t:achsT};}
 const wkAnker=new THREE.Vector3(),wkZiel=new THREE.Vector3(),wkAussen=new THREE.Vector3(),wkRingM=new THREE.Vector3(),wkRingA=new THREE.Vector3(),wkNase=new THREE.Vector3(),wkDummy=new THREE.Object3D(),wkQa=new THREE.Quaternion(),wkE2=new THREE.Vector3();
 const glatt01=x=>{x=Math.max(0,Math.min(1,x));return x*x*(3-2*x);};
 // Lage des angreifenden Weltenkrümmers je Bild: a = Richtung vom Planeten zur Schussposition (Achse der Mondbahn = Planeten-
 // achse, leicht nach außen versetzt – er schwebt über dem Planeten), W = Schussposition (folgt dem Mond zu 30 %), S = Haltepunkt
 // nach dem Austritt, E = Hyperraumloch (seitlich darüber, entlang der eigenen Bahn – dort liegt nichts), aus = Richtung S→E.
 function wkLage(w,planet,mondEintrag){
  const P=planet.position,M=mondEintrag.mesh.position,u=planet.userData,L=Math.max(34,(mondEintrag.bahn||20)*1.5+6),ax=achsen(P);
  const l=w.lage||(w.lage={E:new THREE.Vector3(),S:new THREE.Vector3(),W:new THREE.Vector3(),aus:new THREE.Vector3(),a:new THREE.Vector3()});
  l.a.copy(u.achse).multiplyScalar(Math.cos(.35)).addScaledVector(ax.r,Math.sin(.35)).normalize();
  l.aus.copy(ax.t).multiplyScalar(.8).addScaledVector(OBEN,.6).normalize();
  l.S.copy(P).addScaledVector(l.a,L+10);
  l.W.copy(M).sub(P).multiplyScalar(.3).add(P).addScaledVector(l.a,L);
  l.E.copy(l.S).addScaledVector(l.aus,58);
  return l;
 }
 // ---- Hyperraumloch (Karl, 19.10.): Vorbote (flimmernde Verzerrung), Öffnen mit Lichtblitz, Wirbel mit dunklem Kern und
 // weißglühendem Rand, Lichtstreifen und Druckwelle beim Austritt, Kollaps mit Blitz. setze(rigT): -12…-6 Vorbote,
 // -6…-4 öffnen, -4…-2 Austritt, -2…0 Kollaps. Glühen und Blitze behalten aus der Ferne eine Mindestgröße.
 function hyperloch(farbe){
  const R=18,g=new THREE.Group(),f=new THREE.Color(farbe||'#ff3b22'),v=new THREE.Vector3();
  const mat=new THREE.ShaderMaterial({uniforms:{uZeit:{value:0},uOffen:{value:0},uVor:{value:0},uFarbe:{value:f}},transparent:true,depthWrite:false,side:THREE.DoubleSide,
   vertexShader:'varying vec2 vUv;void main(){vUv=uv*2.-1.;gl_Position=projectionMatrix*modelViewMatrix*vec4(position,1.);}',
   fragmentShader:GLSL_RAUSCHEN+`uniform float uZeit,uOffen,uVor;uniform vec3 uFarbe;varying vec2 vUv;
    void main(){float r=length(vUv),a=atan(vUv.y,vUv.x),o=max(uOffen,.0001);
     float w=a+uZeit*1.7+2.2/(r+.12),arme=.5+.5*sin(w*3.+fbm3(vec3(vUv*3.,uZeit*.4))*2.5),n=fbm3(vec3(cos(w)*r*3.,sin(w)*r*3.,uZeit*.6));
     float kern=1.-smoothstep(.30*o,.41*o,r),rand=exp(-pow((r-.46*o)/(.03+.025*o),2.))*o;
     float wirbel=smoothstep(1.,.36*o,r)*smoothstep(.28*o,.52*o,r)*(arme*.7+n*.6)*o;
     float vor=uVor*(1.-o)*smoothstep(.95,.2,r)*smoothstep(.05,.35,r)*(.35+.65*arme)*(.6+.4*sin(uZeit*9.+r*20.));
     vec3 c=vec3(.35,.55,1.4)*wirbel*1.5+mix(uFarbe*2.,vec3(2.6,2.7,3.),.6)*rand*3.+vec3(.5,.7,1.5)*vor;
     float al=clamp(kern*.94*o+wirbel*.8+rand+vor*.45,0.,1.);c=mix(c,vec3(0.,.004,.018),kern*o*.96);
     gl_FragColor=vec4(c,al);}`});
  const scheibe=new THREE.Mesh(new THREE.PlaneGeometry(2,2),mat);g.add(scheibe);
  const glut=sprite('#8fb4ff',R*2,0),blitz=sprite('#f4f8ff',R*4,0);g.add(glut,blitz);
  const welle=new THREE.Mesh(new THREE.RingGeometry(.93,1,96),new THREE.MeshBasicMaterial({color:new THREE.Color('#bcd4ff').multiplyScalar(2.5),transparent:true,opacity:0,depthWrite:false,side:THREE.DoubleSide,blending:THREE.AdditiveBlending}));g.add(welle);
  const NS=160,sp=new Float32Array(NS*6),sd=[];let z_=97;const zu=()=>((z_=Math.imul(z_^z_>>>15,2246822507)+1013904223>>>0)/4294967296);
  for(let i=0;i<NS;i++){const a=zu()*6.283,r=Math.sqrt(zu())*R*.42;sd.push({x:Math.cos(a)*r,y:Math.sin(a)*r,v:.5+zu(),l:.4+zu()*.9});}
  const sg=new THREE.BufferGeometry();sg.setAttribute('position',new THREE.BufferAttribute(sp,3));
  const streifen=new THREE.LineSegments(sg,new THREE.LineBasicMaterial({color:new THREE.Color('#cfe0ff').multiplyScalar(2.2),transparent:true,opacity:0,depthWrite:false,blending:THREE.AdditiveBlending}));streifen.frustumCulled=false;g.add(streifen);
  const spitze=(x,m,b)=>Math.max(0,1-Math.abs(x-m)/b);
  return {objekt:g,setze(t,zeit){const sicht=t>-12&&t<.8;g.visible=sicht;if(!sicht)return;
   const vor=glatt01((t+12)/6),offen=t<-6?0:t<-4?glatt01((t+6)/2):t<-2?1:t<0?1-glatt01((t+2)/2):0,u=mat.uniforms;
   u.uZeit.value=zeit;u.uOffen.value=offen;u.uVor.value=vor;scheibe.scale.setScalar(R*(1.05+.25*offen));scheibe.visible=vor>.01||offen>.01;
   const d=camera.position.distanceTo(g.getWorldPosition(v)),min=d*.03;
   glut.material.opacity=Math.max(vor*.3*(1-offen),offen*.75);glut.scale.setScalar(Math.max(R*(1.4+2*offen),min*(1.5+2*offen)));
   const bl=Math.min(1,Math.max(spitze(t,-5.7,.5),spitze(t,-3.95,.45),spitze(t,-.35,.6)*1.2));blitz.material.opacity=bl;blitz.scale.setScalar(Math.max(R*4,min*5)*(.7+.5*bl));
   const tw=(t+4.1)/1.4;welle.visible=tw>0&&tw<1;if(welle.visible){welle.scale.setScalar(R*(.5+3.2*tw));welle.material.opacity=(1-tw)*.85;}
   const te=(t+4.3)/2.3;streifen.visible=te>0&&te<1;
   if(streifen.visible){for(let i=0;i<NS;i++){const q=sd[i],za=te*95*q.v,zb=za+R*q.l*(1.4-te);sp.set([q.x*(1+te*.6),q.y*(1+te*.6),za,q.x*(1+te*.4),q.y*(1+te*.4),zb],i*6);}sg.attributes.position.needsUpdate=true;streifen.material.opacity=Math.sin(te*Math.PI);}
  }};
 }
 function wkFern(farbe){
  const g=new THREE.Group(),c=new THREE.Color(farbe||'#ff3b22'),hell=c.clone().lerp(new THREE.Color('#ffffff'),.35).multiplyScalar(4);
  // Strahl aus der Ferne: offene Röhre mit Mindestbreite nach Kameraabstand (eine Linie wäre nur 1 Pixel)
  const roehre=new THREE.Mesh(new THREE.CylinderGeometry(1,1,1,10,1,true),new THREE.MeshBasicMaterial({color:hell,transparent:true,opacity:0,depthWrite:false,blending:THREE.AdditiveBlending}));roehre.frustumCulled=false;
  const lade=sprite(c.clone().lerp(new THREE.Color('#fff2d0'),.4),2,0),treffer=sprite(c,3,0),blitz=sprite('#fff4e0',10,0),marke=sprite(c.clone().lerp(new THREE.Color('#ffe2b0'),.5),4,0);g.add(roehre,lade,treffer,blitz,marke);
  const mitte=new THREE.Vector3(),richtung=new THREE.Vector3(),Y=new THREE.Vector3(0,1,0);
  // Zielring am Mond (immer obenauf): rastet beim Einschwenken ein, pulsiert beim Laden schneller und zieht sich zusammen
  const ringMat=new THREE.MeshBasicMaterial({color:c.clone().lerp(new THREE.Color('#ffffff'),.35).multiplyScalar(1.15),transparent:true,opacity:0,depthWrite:false,depthTest:false,side:THREE.DoubleSide,blending:THREE.AdditiveBlending});
  const zielring=new THREE.Group();zielring.add(new THREE.Mesh(new THREE.RingGeometry(1,1.035,96),ringMat));
  for(let i=0;i<4;i++){const w=i*Math.PI/2,strich=new THREE.Mesh(new THREE.PlaneGeometry(.03,.2),ringMat);strich.position.set(Math.cos(w)*1.16,Math.sin(w)*1.16,0);strich.rotation.z=w-Math.PI/2;zielring.add(strich);}
  zielring.renderOrder=5;g.add(zielring);
  return {objekt:g,setze(nase,ziel,z){if(!nase){g.visible=false;return;}g.visible=true;
   const dN=camera.position.distanceTo(nase),dZ=camera.position.distanceTo(ziel),fern=glatt01((dZ-70)/220);
   roehre.visible=z.feuer>0;if(roehre.visible){mitte.copy(nase).add(ziel).multiplyScalar(.5);richtung.copy(ziel).sub(nase);const lang=richtung.length(),dick=Math.max(.1,camera.position.distanceTo(mitte)*.0022);roehre.position.copy(mitte);roehre.quaternion.setFromUnitVectors(Y,richtung.normalize());roehre.scale.set(dick,lang,dick);roehre.material.opacity=z.feuer*(.55+.25*Math.sin(z.zeit*47))*(.35+.65*fern);}
   // Leuchtmarke am Weltenkrümmer: aus der Ferne ein heller Punkt, der beim Laden stärker glüht
   if(z.wk){marke.position.copy(z.wk);marke.material.opacity=fern*(.45+.4*z.laden+.3*z.feuer);marke.scale.setScalar(camera.position.distanceTo(z.wk)*(.028+.02*z.laden));}else marke.material.opacity=0;
   lade.position.copy(nase);lade.material.opacity=(z.laden>0?.2+.6*z.laden:z.feuer*.8)*(.35+.65*fern);lade.scale.setScalar(Math.max(1.2+z.laden*2.5,dN*(.006+.014*z.laden+.012*z.feuer)));
   treffer.position.copy(ziel);treffer.material.opacity=z.feuer*.8*(.3+.7*fern);treffer.scale.setScalar(Math.max(3,dZ*.022));
   const b=z.bruch>=0&&z.bruch<1.8?1-z.bruch/1.8:0;blitz.position.copy(ziel);blitz.material.opacity=b*fern;blitz.scale.setScalar(Math.max(14,dZ*.14)*(.6+z.bruch*.8));
   zielring.visible=z.fokus>.01;if(zielring.visible){zielring.position.copy(ziel);zielring.quaternion.copy(camera.quaternion);zielring.rotateZ(z.zeit*(.6+1.6*z.laden));zielring.scale.setScalar(Math.max((z.mondR||1.5)*1.55,dZ*.014)*(1.15-.15*z.laden));ringMat.opacity=z.fokus*(.28+.14*Math.sin(z.zeit*(3+10*z.laden)));}}};
 }
 function weltenkruemmerBewegen(){
  const jetzt=GameTime.now(),echt=performance.now()/1000;
  for(const w of weltenkruemmer){
   if(w.art==='station'){
    const pos=Number(String(w.koord).split(':')[2]),planet=planets.find(m=>Number(m.userData.data.position)===pos);if(!planet)continue;
    // eigene Bahn außerhalb der Mondbahn, 0,5 rad gegen die Äquatorebene geneigt, rechtläufig
    const pu=planet.userData,mond=moons.find(m=>Number(m.planet.userData.data.position)===pos),bahn=stationBahn(pu.radius||planet.geometry.parameters.radius,mond?.bahn,mond?mond.mesh.geometry.parameters.radius:0),wink=w.phase+echt*.035;
    wkE2.copy(pu.e2).multiplyScalar(Math.cos(.5)).addScaledVector(pu.achse,Math.sin(.5));
    w.inst.objekt.position.copy(planet.position).addScaledVector(pu.e1,Math.cos(wink)*bahn).addScaledVector(wkE2,Math.sin(wink)*bahn);
    wkZiel.copy(planet.position).addScaledVector(pu.e1,Math.cos(wink+.35)*bahn).addScaledVector(wkE2,Math.sin(wink+.35)*bahn);w.inst.objekt.lookAt(wkZiel);
    // alle 75 s: aufklappen, 10 s offen halten, zuklappen (ohne Schuss)
    const offen=w.inst.bis('offen')+.8,zyklus=75,u=(echt+w.phase*30)%zyklus,auf=w.inst.bis('aufklappen');
    const t=u<offen?u:u<offen+10?offen-.01:u<offen*2+10-auf?Math.max(0,offen-(u-offen-10)):0;
    w.inst.setze(Math.min(t,offen-.01),echt);continue;
   }
   const pos=Number(String(w.f.ziel).split(':')[2]),planet=planets.find(m=>Number(m.userData.data.position)===pos);if(!planet)continue;
   const mondEintrag=moons.find(m=>Number(m.planet.userData.data.position)===pos),ziel=(mondEintrag?.mesh||planet).position,anker=planet.position;
   const herkunft=anker.clone().setY(0).normalize().multiplyScalar(systemRand()+45).add(new THREE.Vector3(0,22,0));
   const ab=Number(w.f.abflug_sekunden)||jetzt,an=Number(w.f.ankunft_sekunden)||jetzt,richtung=herkunft.clone().sub(anker).setY(0).normalize();
   const warte=wkAnker.copy(anker).add(richtung.clone().multiplyScalar(20)).add(new THREE.Vector3(0,13,0));
   if(w.art==='zerstoerung'&&!(mondEintrag&&window.Mondzerstoerung)){if(!w.blick){w.blick=new THREE.Object3D();group.add(w.blick);}w.blick.position.copy(w.inst.objekt.position).lerp(ziel,.5);}
   if(w.art==='zerstoerung'&&mondEintrag&&window.Mondzerstoerung){
    // Karl, 19.10.: Im Normalflug ist der Weltenkrümmer ultra langsam – er kommt aus einem Hyperraumloch, bremst hart, nimmt
    // den Mond in den Fokus und gleicht sich seiner Bahn an. Er steht dazu auf der Achse der Mondbahn (über dem Planeten,
    // leicht nach außen): von dort ist die ganze Mondbahn ein Kreis um den Planeten – der Mond verschwindet beim Zielen nie hinter
    // dem Planeten, der Strahl kreuzt ihn nie, und der Weltenkrümmer folgt dem Mond nur leicht. Zeitplan (rigT = Sekunden zum
    // Beginn der Entfaltung): -12 Vorbote, -6 Loch öffnet, -4 Austritt, -2 Kollaps, 0 Entfaltung + Einschwenken, Laden 7 s, Schuss.
    const tempoW=Math.max(1,world?.tempo||1),feuer=w.inst.bis('feuer'),laden=w.inst.bis('laden'),vorlauf=feuer+6,rigT=vorlauf-(an-jetzt)/tempoW,bruch=rigT-vorlauf,o=w.inst.objekt;
    const lage=wkLage(w,planet,mondEintrag);w.rigT=rigT;
    if(!w.loch){w.loch=hyperloch(w.inst.farbe);group.add(w.loch.objekt);}
    w.loch.objekt.position.copy(lage.E);w.loch.objekt.lookAt(lage.S);w.loch.setze(rigT,echt);
    if(!w.blick){w.blick=new THREE.Object3D();group.add(w.blick);}
    if(rigT<-4){o.visible=false;w.inst.setze(.2,echt);w.blick.position.copy(lage.E);w.fern?.setze(null);for(const L of sys3?.leihlichter||[])L.intensity=0;continue;}
    o.visible=true;
    if(rigT<-2){// Austritt: aus dem Loch schießen und hart bremsen, anfangs gestreckt
     const u=(rigT+4)/2,e=1-Math.pow(1-u,4);o.position.copy(lage.E).addScaledVector(lage.aus,4).lerp(lage.S,e);o.scale.set(1.05,1.05,1.05*(1+7*Math.pow(1-u,3)));
     wkDummy.position.copy(o.position);wkDummy.lookAt(wkZiel.copy(lage.S).addScaledVector(lage.aus,-30));o.quaternion.copy(wkDummy.quaternion);}
    else{// Einschwenken: ultra langsam vom Haltepunkt in die Schussposition, der Blick schwenkt aus der Flugrichtung auf den Mond
     const u=glatt01((rigT-.5)/Math.max(1,w.inst.bis('offen')-.5)),fokus=glatt01((rigT-.3)/3.2);o.position.lerpVectors(lage.S,lage.W,u);o.scale.setScalar(1.05);
     wkDummy.position.copy(o.position);wkDummy.lookAt(wkZiel.copy(lage.S).addScaledVector(lage.aus,-30));wkQa.copy(wkDummy.quaternion);wkDummy.lookAt(ziel);o.quaternion.slerpQuaternions(wkQa,wkDummy.quaternion,fokus);}
    w.blick.position.copy(o.position).lerp(ziel,.5);
    if(!w.mond){const r=mondEintrag.mesh.geometry.parameters.radius||1.5;w.mond=window.Mondzerstoerung.erzeugen({typ:planet.userData.data.mond?.typ,radius:r,farbe:w.inst.farbe,stuecke:50,feinheit:18,funken:300,saat:pos});group.add(w.mond.objekt);w.mondEintrag=mondEintrag;}
    w.mond.objekt.position.copy(ziel);mondEintrag.mesh.visible=false;
    if(!w.ring&&rigT-(w.inst.bis('feuer')+6)>=1.2){w.ring=true;const pd=planet.userData.data;if(!(view.truemmer||[]).some(t=>t.koord===pd.koord)&&!sys3?.felder.some(f=>f.planet===planet)){const dm=Number(pd.mond?.durchmesser||pd.mond?.groesse)||4000,e=truemmerFeld(planet,pd,planet.userData.radius||8,{erz:dm*30,kristall:dm*14});if(e){e.wachsen=performance.now()/1000;e.mond=mondEintrag.mesh;}}}
    if(!w.gemeldet&&rigT-(w.inst.bis('feuer')+6)>=.4){w.gemeldet=true;const m=planet.userData.data.mond||{};systemMeldung('☾ Mond '+(m.name||'')+' zerstört',`[${planet.userData.data.koord}] · Weltenkrümmer · ${m.besitzer||planet.userData.data.spieler||''}`.replace(/ · $/,''));}
    wkZiel.copy(o.position).sub(ziel).normalize();w.mond.treffer(wkZiel);
    const abstand=o.position.distanceTo(ziel)-w.mond.radius*.97;w.inst.strahl(abstand/1.05-w.inst.nase.z);
    const t=Math.max(0,Math.min(rigT,w.inst.dauer-.01));
    w.inst.setze(rigT<0?.2:bruch>=0?Math.max(t,feuer+4.3):t,echt);
    if(sys3?.leihlichter){if(!w.rigLichter){w.rigLichter=[];o.traverse(x=>{if(x.isLight)w.rigLichter.push(x);});}
     sys3.leihlichter.forEach((L,i)=>{const q=w.rigLichter[i];if(!q){L.intensity=0;return;}q.getWorldPosition(L.position);L.color.copy(q.color);L.intensity=q.intensity;L.distance=q.distance;L.decay=q.decay;});}
    // Nach dem Bruch ziehen die Brocken in die Bahn und verteilen sich zum Trümmerring (Mitte des Trümmerfelds: 1,75 Planetenradien)
    const mr=w.mond.radius;wkRingM.copy(anker).sub(ziel).divideScalar(mr);wkRingA.copy(planet.userData.achse);
    w.mond.setze({schmelz:Math.max(0,Math.min(1,(rigT-feuer)/3)),riss:Math.max(0,Math.min(1,(rigT-feuer-3)/3)),bruch:bruch>=0?bruch:-1,zeit:echt,ring:{mitte:wkRingM,achse:wkRingA,radius:(planet.userData.radius||8)*1.75/mr}});
    if(bruch>14){w.mond.objekt.visible=false;}
    // Aus der Ferne: Strahl als leuchtende Linie, Ladeglühen an der Mündung, Blitz beim Bruch (Mindestgröße nach Kameraabstand)
    if(!w.fern){w.fern=wkFern(w.inst.farbe);group.add(w.fern.objekt);}
    w.fern.setze(o.localToWorld(wkNase.copy(w.inst.nase)),ziel,{laden:rigT>=laden&&rigT<feuer?(rigT-laden)/(feuer-laden):0,feuer:rigT>=feuer&&rigT<feuer+3.1?1:0,fokus:rigT<feuer?glatt01((rigT-1)/2):0,mondR:w.mond.radius,wk:o.position,bruch,zeit:echt});
    continue;
   }
   if(w.f.zustand==='hinflug'&&jetzt<an){const anteil=Math.max(0,Math.min(1,(jetzt-ab)/Math.max(1,an-ab)));w.inst.objekt.position.lerpVectors(herkunft,warte,anteil);w.inst.objekt.lookAt(ziel);w.inst.setze(.2,echt);continue;}
   if(w.start==null)w.start=echt;const t=Math.min(echt-w.start,w.inst.dauer-.01);
   w.inst.objekt.position.copy(warte);w.inst.objekt.lookAt(ziel);w.inst.setze(t,echt);
  }
 }
 function ring(radius,color,opacity=.25){const points=[];for(let i=0;i<=180;i++){const a=i/180*Math.PI*2;points.push(new THREE.Vector3(Math.cos(a)*radius,0,Math.sin(a)*radius));}const m=new THREE.LineLoop(new THREE.BufferGeometry().setFromPoints(points),new THREE.LineBasicMaterial({color,transparent:true,opacity}));group.add(m);return m;}
 function orbitAccent(radius,color){const points=[];for(let i=0;i<=16;i++){const a=(i/16-.5)*.18;points.push(new THREE.Vector3(Math.cos(a)*radius,.2,Math.sin(a)*radius));}const arc=new THREE.Line(new THREE.BufferGeometry().setFromPoints(points),new THREE.LineBasicMaterial({color,transparent:true,opacity:.5}));group.add(arc);return arc;}
 function updateOrbits(){for(const orbit of orbits){const d=orbit.userData,match=selectedOwner?d.owner===selectedOwner:Number(d.position)===selected||hovered==='planet:'+d.position;orbit.material.opacity=match?.5:selectedOwner&&!friendly(ownerMarks.get(d.owner)?.relation)?.025:d.baseOpacity;if(d.accent)d.accent.material.opacity=match?.85:selectedOwner&&!friendly(ownerMarks.get(d.owner)?.relation)?.18:.5;}}
 // Farben kommen als data-Attribut und werden per CSSOM gesetzt (farbenSetzen): style="…" im HTML blockiert die CSP des Spielservers.
 function farbenSetzen(root){if(!root)return;for(const e of root.querySelectorAll('[data-farbe]'))e.style.color=e.dataset.farbe;for(const e of root.querySelectorAll('[data-owner-farbe]'))e.style.setProperty('--owner-color',e.dataset.ownerFarbe);}
 function ownerCodes(names){if(names.length>1)return `<span class="space-shared-count">${names.length} Spieler</span>`;const m=ownerMarks.get(names[0]);return m?`<span data-farbe="${esc(m.color)}">${m.code}</span>`:'';}
 function selectOwner(name){selectedOwner=selectedOwner===name?'':name;for(const l of labels){const match=l.owners?.includes(selectedOwner);l.el.classList.toggle('owner-muted',!!selectedOwner&&!match&&!l.friendly.length);l.el.classList.toggle('owner-selected',!!selectedOwner&&match);const codes=l.el.querySelector('.space-owner-codes');if(codes){codes.innerHTML=ownerCodes(l.owners);l.metrics=null;}if(match)l.el.style.setProperty('--owner-color',ownerMarks.get(selectedOwner).color);}for(const star of sectorStars){const match=!selectedOwner||!star.owners.length||star.owners.includes(selectedOwner)||hasFriendly(star.owners);star.mesh.material.color.copy(star.color).multiplyScalar(match?1:.6);star.halo.material.opacity=match?star.opacity:star.opacity*.6;}updateOrbits();for(const planet of planets){const match=planet.userData.data.spieler===selectedOwner;planet.material.emissive.set(match&&selectedOwner?ownerMarks.get(selectedOwner).color:0x000000);planet.material.emissiveIntensity=.3;}updateArcColors();for(const b of document.querySelectorAll('[data-space-owner]'))b.setAttribute('aria-pressed',String(b.dataset.spaceOwner===selectedOwner));neu();}
 // Invasion v2 und gestrandete Flotten (Karl 24.10.2026: „Status ‚Verwüstet‘ in der Sonnensystemansicht“, gestrandete Flotten
 // „offen sichtbar … eigenes Icon an der Stelle des Trümmerfelds“): kleine Marken an der Planetenbeschriftung (Text per textContent).
 const ANKER='<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M3 8h18l-3 4H6ZM12 4v4M12 12v8M8.5 16.5a3.5 3.5 0 0 0 7 0M10 14.5h4"/></svg>';
 function invasionsMarke(b,p){
  if(!b||!p)return;const eigenInv=(view?.invasionen||[]).find(i=>i.planet===p.koord),verw=!!(p.verwuestet||eigenInv?.verwuestet);
  if(verw){const s=document.createElement('span');s.className='space-verwuestet';s.textContent='Verwüstet';b.append(s);b.title+=' · verwüstet';b.setAttribute('aria-label',b.getAttribute('aria-label')+' · verwüstet');}
  for(const eigen of [false,true]){const l=(p.orbitflotten||[]).filter(o=>!!o.eigen===eigen);if(!l.length)continue;const n=l.reduce((s,o)=>s+(Number(o.schiffe)||0),0);
   const s=document.createElement('span');s.className='space-gestrandet'+(eigen?' eigen':'');s.innerHTML=ANKER;const z=document.createElement('b');z.textContent=fmt(n);s.append(z);b.append(s);
   const text=(eigen?'eigene gestrandete Flotte':'gestrandete Flotte von '+l.map(o=>o.spieler).join(', '))+' · '+fmt(n)+' Schiffe';b.title+=' · '+text;b.setAttribute('aria-label',b.getAttribute('aria-label')+' · '+text);}
 }
 function label(text,obj,key,kind,fromSector=sector,mark){
  const b=document.createElement('button');b.type='button';b.className='space-label '+kind;b.dataset.coordinate=text;
  const number=document.createElement('span');number.className='space-number';number.textContent=kind==='system'||kind==='planet'?String(key):text;b.append(number);
  if(kind==='debris'){number.textContent='✧';const t=document.createElement('span');t.className='space-debris-text';t.textContent=' Trümmer · '+key;number.append(t);b.dataset.debris=key;b.title=text;b.setAttribute('aria-label',text+' bergen');}
  b.title=text;b.setAttribute('aria-label',kind==='debris'?text+' ansehen':kind==='planet'?'Planet '+text:kind==='sector'?'Sektor '+key+' öffnen':'System '+fromSector+':'+key+' öffnen');
  if(stage==='galaxy'&&kind==='system'){b.title+=' · Sektor '+fromSector+' öffnen';b.setAttribute('aria-label','Sektor '+fromSector+' öffnen · System '+fromSector+':'+key);}
  if(mark){b.dataset.mapState=mark.state;b.title+=' · '+({unknown:'Unerforscht',partial:'Teilweise erkundet',empty:'Vollständig erkundet · unbewohnt',occupied:'Besiedlung bekannt'}[mark.state]||'');const safe=mark.inhabitants.filter(r=>friendly(r.relation));b.dataset.friendly=safe.map(r=>r.relation).filter((r,i,a)=>a.indexOf(r)===i).join(' ');if(safe.length){const badge=document.createElement('span');badge.className='space-friendly-code';badge.textContent=(safe.some(r=>r.relation==='eigen')?'◆':'')+(safe.some(r=>r.relation==='verbuendet')?'○':'');b.append(badge);}const owners=mark.inhabitants.map(p=>p.name),marks=owners.map(n=>ownerMarks.get(n)).filter(Boolean);b.dataset.owners=JSON.stringify(owners);if(marks.length){b.style.setProperty('--owner-color',marks[0].color);const ids=document.createElement('span');ids.className='space-owner-codes';ids.innerHTML=ownerCodes(owners);farbenSetzen(ids);b.append(ids);}b.dataset.relation=mark.relation;b.style.setProperty('--relation-color',mark.color);
   if(mark.inhabitants.length){const residents=document.createElement('span');residents.className='space-resident';const first=mark.inhabitants[0];residents.textContent=first.name+(first.tag?' ['+first.tag+']':'')+(mark.inhabitants.length>1?' +'+(mark.inhabitants.length-1):'');b.append(residents);b.title+=' · '+mark.inhabitants.map(p=>p.name+(p.tag?' ['+p.tag+']':'')+' · '+GALAXY_RELATION_NAMES[p.relation]).join('; ');b.setAttribute('aria-label',b.getAttribute('aria-label')+' · '+b.title.split(' · ').slice(1).join(' · '));}
  }
  b.style.left='0px';b.style.top='0px';b.onclick=()=>pick(kind,key,fromSector);$g('galaxy-labels').append(b);labels.push({el:b,obj,kind,key,sektor:fromSector,owners:mark?.inhabitants.map(p=>p.name)||[],state:mark?.state,relations:[...new Set((mark?.inhabitants||[]).map(r=>r.relation))],friendly:[...new Set((mark?.inhabitants||[]).map(r=>r.relation).filter(friendly))]});b.addEventListener('focus',()=>{neu();});b.addEventListener('blur',()=>{neu();});
 }
 function knownPlaces(s,n){
  const prefix=s+':'+n+':',places=(view?.atlas||[]).filter(p=>p.koord.startsWith(prefix)).map(p=>({...p,besiedlung_bekannt:true}));
  for(const p of view.planeten||[])if(p.koord.startsWith(prefix)&&!places.some(q=>q.koord===p.koord))places.push({...p,spieler:view.name,bekannt:true});
  return places;
 }
 // Kugeln mit Detailstufe (Handy-Leistung, Karl 22.10.2026): Planeten, Atmosphären, Wolken, Stadtlichter, Monde und Sonne
 // zeichnen eine gröbere Kugel (~30 % der Dreiecke), solange ihr Radius auf dem Schirm unter 160 px liegt – der Rand weicht
 // dabei weniger als einen halben Pixel ab. Näher heran kommt die volle Kugel zurück.
 let kugelLod=[];const kugelV=new THREE.Vector3();
 function kugelnErfassen(){kugelnFrei();const grob=new Map();group.traverse(o=>{const g=o.geometry;if(!o.isMesh||g?.type!=='SphereGeometry')return;const p=g.parameters;if(p.widthSegments<48)return;
   const k=[p.radius,p.widthSegments,p.heightSegments,p.phiStart,p.phiLength,p.thetaStart,p.thetaLength].join(':');let gg=grob.get(k);
   if(!gg){gg=new THREE.SphereGeometry(p.radius,Math.round(p.widthSegments*.55),Math.max(12,Math.round(p.heightSegments*.55)),p.phiStart,p.phiLength,p.thetaStart,p.thetaLength);grob.set(k,gg);}
   kugelLod.push({o,fein:g,grob:gg,an:null});});}
 function kugelnFrei(){for(const k of kugelLod){k.o.geometry=k.fein;k.grob.dispose();}kugelLod=[];}
 function kugelnWaehlen(h){const f=camera.projectionMatrix.elements[5]*h/2;for(const k of kugelLod){k.o.getWorldPosition(kugelV);const rw=k.fein.parameters.radius*k.o.matrixWorld.getMaxScaleOnAxis(),d=kugelV.distanceTo(camera.position);
   const fein=!!window.__g3OhneLod||d<=rw*1.05||rw*f/d>160;if(k.an!==fein){k.an=fein;k.o.geometry=fein?k.fein:k.grob;}}}
 function resetScene(){nebelHimmelZeigen(false);selectedOwner='';sectorNumberView=false;if(host)host.dataset.numberFocus='false';kugelnFrei();if(group){scene.remove(group);disposeTree(group);}group=new THREE.Group();scene.add(group);targets=[];labels=[];planets=[];moons=[];loecher=[];weltenkruemmer=[];orbits=[];sectorStars=[];beaconGroups=[];sectorArcBatch=null;hovered='';karten.clear();sys3=null;folge=null;galaxieDreh=null;$g('galaxy-labels').replaceChildren();neu();}
 function hitAt(e){const r=canvas.getBoundingClientRect();raycaster.setFromCamera(new THREE.Vector2((e.clientX-r.left)/r.width*2-1,-(e.clientY-r.top)/r.height*2+1),camera);return raycaster.intersectObjects(targets,false).find(h=>{for(let o=h.object;o;o=o.parent)if(!o.visible)return false;return true;});}
 function highlight(hit){const d=hit?.object.userData,next=d?.kind==='system'?'system:'+d.sektor+':'+d.key:d?.kind==='planet'?'planet:'+d.key:'';canvas.style.cursor=hit?'pointer':'grab';if(next===hovered)return;hovered=next;neu();for(const l of labels)if(l.kind==='system'||l.kind==='planet')l.el.classList.toggle('is-highlighted',(l.kind==='system'?'system:'+l.sektor+':'+l.key:'planet:'+l.key)===hovered);updateOrbits();}
 // Kamera: focus/yaw/pitch/distance ist der gezeigte Zustand, zielKam das Ziel. Gesten setzen beides (sofort), Flüge
 // (Doppeltipp, Hinfliegen) nur das Ziel; folge hält ein Objekt (Planet, Mond, Flotte, Weltenkrümmer) in der Mitte.
 const zielKam={x:0,y:0,z:0,yaw:.15,pitch:.72,distance:600},schwung={yaw:0,pitch:0},folgePunkt=new THREE.Vector3();let folge=null,folgeKennung='',fliegt=false,bauten=0;
 function kennungVon(o){if(!o)return '';const d=o.userData||{};if(d.kind==='planet')return 'planet:'+d.key;if(d.kind==='mond')return 'mond:'+d.key;if(d.kind==='tor')return 'tor:'+d.key;const w=weltenkruemmer.find(x=>x.inst.objekt===o||x.blick===o);if(w)return 'wk:'+w.art+':'+(w.f?.flotte??w.koord??'');return '';}
 function loeseKennung(k){const [art,a,b]=k.split(':');if(art==='planet')return planets.find(m=>String(m.userData.data.position)===a)||null;if(art==='mond')return moons.find(m=>String(m.planet.userData.data.position)===a)?.mesh||null;if(art==='tor')return moons.find(m=>String(m.planet.userData.data.position)===a)?.tor?.ring||null;if(art==='wk'){const w=weltenkruemmer.find(w=>w.art===a&&String(w.f?.flotte??w.koord??'')===b);return w?(w.blick||w.inst.objekt):null;}return null;}
 function setzeKamera(){camera.position.set(focus.x+Math.sin(yaw)*Math.cos(pitch)*distance,focus.y+Math.sin(pitch)*distance,focus.z+Math.cos(yaw)*Math.cos(pitch)*distance);camera.lookAt(focus.x,focus.y,focus.z);}
 function cameraUpdate(){Object.assign(zielKam,{x:focus.x,y:focus.y,z:focus.z,yaw,pitch,distance});fliegt=false;bedient();setzeKamera();neu();}
 function kameraSchritt(dt){
  if(folgeKennung&&(!folge||!folge.parent))folge=loeseKennung(folgeKennung);else if(folge&&!folge.parent)folge=null;
  if(folge){folge.getWorldPosition(folgePunkt);zielKam.x=folgePunkt.x;zielKam.y=folgePunkt.y;zielKam.z=folgePunkt.z;}
  const sw=Math.abs(schwung.yaw)+Math.abs(schwung.pitch)>2e-3;if(sw){zielKam.yaw+=schwung.yaw*dt;zielKam.pitch=Math.max(-1.54,Math.min(1.54,zielKam.pitch+schwung.pitch*dt));const d=Math.exp(-dt*3.2);schwung.yaw*=d;schwung.pitch*=d;}else schwung.yaw=schwung.pitch=0;
  const k=1-Math.exp(-dt*(fliegt?3.4:14));focus.x+=(zielKam.x-focus.x)*k;focus.y+=(zielKam.y-focus.y)*k;focus.z+=(zielKam.z-focus.z)*k;yaw+=(zielKam.yaw-yaw)*k;pitch+=(zielKam.pitch-pitch)*k;distance=Math.exp(Math.log(distance)+(Math.log(zielKam.distance)-Math.log(distance))*k);
  const rest=Math.abs(zielKam.x-focus.x)+Math.abs(zielKam.y-focus.y)+Math.abs(zielKam.z-focus.z)+(Math.abs(zielKam.yaw-yaw)+Math.abs(zielKam.pitch-pitch))*60+Math.abs(zielKam.distance-distance)*.5;
  if(rest<.02&&!sw)fliegt=false;setzeKamera();return rest>.02||sw||!!folge;
 }
 // Hinfliegen: Objekt verfolgen (Abstand nach Größe) oder auf einen Punkt der Bahnebene zielen.
 function fliegeZu(obj,abstand,punkt,blick){autoFit=null;schwung.yaw=schwung.pitch=0;folgeKennung='';if(obj){folge=obj;folgeKennung=kennungVon(obj);obj.getWorldPosition(folgePunkt);Object.assign(zielKam,{x:folgePunkt.x,y:folgePunkt.y,z:folgePunkt.z});}else if(punkt){folge=null;Object.assign(zielKam,{x:punkt.x,y:punkt.y,z:punkt.z});}zielKam.distance=Math.max(5,Math.min(2400,abstand));zielKam.yaw=yaw;zielKam.pitch=Math.max(.18,Math.min(1.25,pitch));
  if(blick){let y=blick.yaw;while(y-yaw>Math.PI)y-=2*Math.PI;while(y-yaw<-Math.PI)y+=2*Math.PI;zielKam.yaw=y;zielKam.pitch=blick.pitch;}fliegt=true;neu();}
 // Blick auf die Tagseite: Kamera zwischen Objekt und Sonne, leicht seitlich (Terminator im Bild).
 function sonnenBlick(o){const p=o.getWorldPosition(new THREE.Vector3());if(p.lengthSq()<1)return null;return {yaw:Math.atan2(-p.x,-p.z)+.75,pitch:.3};}
 function zielGroesse(o){const d=o?.userData||{};if(d.radius)return d.radius;if(o?.geometry?.parameters?.radius)return o.geometry.parameters.radius;return 4;}
 function hinfliegen(hit,px,py){const o=hit?.object,d=o?.userData||{};
  if(!o){const p=bodenPunkt(px,py);if(p)fliegeZu(null,distance*.5,p);return;}
  if(d.kind==='planet'||d.kind==='mond'){fliegeZu(o,zielGroesse(o)*(d.kind==='mond'?9:5.5),null,stage==='system'?sonnenBlick(o):null);return;}
  if(d.kind==='tor'){fliegeZu(o,zielGroesse(o)*17,null,stage==='system'?sonnenBlick(o):null);return;}
  if(d.kind==='sonne'){fliegeZu(null,(sys3?.klasse.r||14)*6,new THREE.Vector3());return;}
  if(d.kind==='wk'){fliegeZu(o.parent||o,32);return;}
  if(hit.point)fliegeZu(null,Math.min(distance,60),hit.point);}
 function bodenPunkt(px,py){if(!canvas)return null;const r=canvas.getBoundingClientRect();raycaster.setFromCamera(new THREE.Vector2((px-r.left)/r.width*2-1,-(py-r.top)/r.height*2+1),camera);const z=new THREE.Vector3();return raycaster.ray.intersectPlane(new THREE.Plane(new THREE.Vector3(0,1,0),-focus.y),z)?z:null;}
 // autoFit: Basis der letzten Einpassung; solange der Spieler nicht zoomt/verschiebt, passt eine Größenänderung neu ein.
 let autoFit=null;
 function fitView(base){autoFit=base;folge=null;folgeKennung='';focus.x=focus.y=focus.z=0;const extent=stage==='system'?systemRand()+8:stage==='sector'?145:245;distance=Math.max(base,extent/(Math.tan(THREE.MathUtils.degToRad(camera.fov/2))*Math.min(1,camera.aspect)*(handy.matches?.98:.85)));cameraUpdate();}
 // Nur bei echter Größenänderung: renderView ruft visible() bei jeder Aktualisierung (alle ~3 s); setSize mit gleicher Größe
 // legte trotzdem den Zeichenpuffer neu an und alle Beschriftungen wurden neu vermessen – ein Ruckler alle drei Sekunden.
 let groesseAlt='';
 function resize(){if(!renderer)return;const rect=canvas.getBoundingClientRect();if(!rect.width||!rect.height)return;const k=rect.width+'x'+rect.height;if(k===groesseAlt)return;groesseAlt=k;viewportHeight=rect.height;for(const l of labels){l.metrics=null;l.metricsK=null;}renderer.setSize(rect.width,rect.height,false);const previous=camera.aspect;camera.aspect=rect.width/rect.height;camera.updateProjectionMatrix();if(autoFit!=null&&!sectorNumberView)fitView(autoFit);else{distance*=Math.max(1,1/camera.aspect)/Math.max(1,1/previous);cameraUpdate();}}
 function closeDetail(){const card=$g('galaxy-infocard');if(card?.open)card.close();}
 // ------------------------------------------------------------ Handy-Overlay ----------
 // Pfad als Chips (Galaxie › Sektor › System; Antippen öffnet die Auswahl), Kamera an der Seite, „Tabelle“ oben rechts,
 // Planetenfeld unten. Jede Aktion ist ein vorhandener Spielknopf (dieselben data-…-Haken wie die Systemtabelle).
 let overlayEl=null,zoomen=()=>{},systemDaten=null,offenesFeld=null,gleichesSystem=false;
 const ZONEN={glut:'Glutzone',leben:'Lebenszone',frost:'Frostzone'};
 const REL_FARBE={eigen:'#64f5a0',verbuendet:'#65d9ff',feindlich:'#ff7e88',nap:'#f3d365',neutral:'#edf3fa',fremde_allianz:'#edf3fa'};
 const REL_TEXT={eigen:'Du',verbuendet:'Verbündet',nap:'Nichtangriffspakt',feindlich:'Feindlich',fremde_allianz:'Neutral',neutral:'Neutral',unbekannt:'Unbekannt'};
 const heimat=()=>String(currentPlanet()?.koord||view?.planeten?.[0]?.koord||'1:1:1').split(':').map(Number);
 // Kameraknöpfe (Karl, 19.10.: „durchdacht und praktisch umgesetzt, optisch hochwertig“): jeder Wechsel gleitet (zielKam +
 // fliegt), Zoom läuft bei gedrücktem Knopf weiter, „Ansicht“ wechselt Draufsicht ↔ Schrägsicht (Symbol zeigt das Ziel),
 // „Übersicht“ zeigt das ganze System (Sektor, Galaxie), „Heimat“ fliegt zu deinem aktiven Planeten – auch aus anderen Systemen.
 const IK={plus:'<svg viewBox="0 0 24 24" aria-hidden="true"><path d="M12 5v14M5 12h14"/></svg>',minus:'<svg viewBox="0 0 24 24" aria-hidden="true"><path d="M5 12h14"/></svg>',
  oben:'<svg viewBox="0 0 24 24" aria-hidden="true"><circle cx="12" cy="12" r="7.5"/><circle cx="12" cy="12" r="2.3"/><path d="M12 2.5v2.2M12 19.3v2.2M2.5 12h2.2M19.3 12h2.2"/></svg>',
  schraeg:'<svg viewBox="0 0 24 24" aria-hidden="true"><ellipse cx="12" cy="13.5" rx="9.2" ry="3.7"/><circle cx="12" cy="11.6" r="2.6"/></svg>',
  rahmen:'<svg viewBox="0 0 24 24" aria-hidden="true"><path d="M4 9V4h5M15 4h5v5M20 15v5h-5M9 20H4v-5"/><circle cx="12" cy="12" r="2.3"/></svg>',
  heim:'<svg viewBox="0 0 24 24" aria-hidden="true"><path d="M3.6 11.3 12 4.2l8.4 7.1"/><path d="M6.2 9.8V20h11.6V9.8"/><path d="M10.2 20v-4.8h3.6V20"/></svg>'};
 let zoomTakt=0,zoomGehalten=false;
 function gleiten(z){autoFit=null;schwung.yaw=schwung.pitch=0;Object.assign(zielKam,z);fliegt=true;neu();}
 function zoomSchritt(f){gleiten({distance:Math.max(5,Math.min(2400,zielKam.distance*f))});}
 function ansichtKnopf(){const b=overlayEl?.querySelector('[data-g3="blick"]');if(!b)return;const oben=zielKam.pitch>1.05,t=oben?'Schrägsicht':'Draufsicht';b.innerHTML=oben?IK.schraeg:IK.oben;b.setAttribute('aria-label',t);b.title=t;}
 function ansichtWechseln(){gleiten({pitch:zielKam.pitch>1.05?.62:1.42});ansichtKnopf();}
 function ueberblick(){const base=stage==='system'?330:stage==='sector'?350:580,extent=stage==='system'?systemRand()+8:stage==='sector'?145:245;
  const d=Math.max(base,extent/(Math.tan(THREE.MathUtils.degToRad(camera.fov/2))*Math.min(1,camera.aspect)*(handy.matches?.98:.85)));
  folge=null;folgeKennung='';gleiten({x:0,y:0,z:0,distance:d,pitch:Math.max(.62,Math.min(1.1,zielKam.pitch))});autoFit=base;ansichtKnopf();}
 function heimatFliegen(){const [s,n]=heimat(),pos=Number(String(currentPlanet()?.koord||'').split(':')[2]);
  const hin=()=>{const pl=planets.find(m=>Number(m.userData.data.position)===pos);if(pl)fliegeZu(pl,(pl.userData.radius||8)*7,null,sonnenBlick(pl));ansichtKnopf();};
  if(stage==='system'&&sector===s&&system===n)hin();else Promise.resolve(showSystem(s,n)).then(()=>setTimeout(hin,350));}
 function overlayBauen(){
  if(overlayEl||!host)return;
  overlayEl=document.createElement('div');overlayEl.className='g3-overlay';
  overlayEl.innerHTML='<nav class="g3-pfad" aria-label="Kartenebenen"></nav><button type="button" class="g3-tabelle" data-gt-tabelle aria-label="Sonnensystem als Tabelle zeigen">▤ Tabelle</button><div class="g3-seite" role="toolbar" aria-label="Kamera" aria-orientation="vertical"><div class="g3-gruppe"><button type="button" data-g3="in" aria-label="Heranzoomen (gedrückt halten)" title="Heranzoomen">'+IK.plus+'</button><button type="button" data-g3="out" aria-label="Herauszoomen (gedrückt halten)" title="Herauszoomen">'+IK.minus+'</button></div><div class="g3-gruppe"><button type="button" data-g3="blick" aria-label="Draufsicht" title="Draufsicht">'+IK.oben+'</button><button type="button" data-g3="mitte" aria-label="Ganzes System zeigen" title="Übersicht">'+IK.rahmen+'</button></div><div class="g3-gruppe"><button type="button" data-g3="heimat" aria-label="Zu deinem Planeten" title="Zu deinem Planeten">'+IK.heim+'</button></div></div><div class="g3-liste" hidden></div><section class="g3-panel" hidden aria-label="Planet"></section>';
  host.append(overlayEl);
  overlayEl.addEventListener('pointerdown',e=>{const b=e.target.closest('[data-g3="in"],[data-g3="out"]');if(!b)return;const f=b.dataset.g3==='in'?.8:1.25;zoomSchritt(f);zoomGehalten=true;clearInterval(zoomTakt);zoomTakt=setInterval(()=>zoomSchritt(f),140);try{b.setPointerCapture(e.pointerId);}catch{}});
  for(const ende of ['pointerup','pointercancel','lostpointercapture'])overlayEl.addEventListener(ende,()=>{clearInterval(zoomTakt);zoomTakt=0;});
  overlayEl.addEventListener('click',e=>{
   const b=e.target.closest('button');if(!b)return;const d=b.dataset;
   if(d.g3Ebene==='galaxie'){listeZu();panelZu();if(stage!=='galaxy')showGalaxy();}
   else if(d.g3Ebene==='hoch')hoch();
   else if(d.g3Liste){const liste=overlayEl.querySelector('.g3-liste');if(!liste.hidden&&liste.dataset.art===d.g3Liste)listeZu();else listeOeffnen(d.g3Liste);}
   else if(d.g3Sektor){listeZu();panelZu();void showSector(d.g3Sektor);}
   else if(d.g3System){listeZu();panelZu();void showSystem(sector,d.g3System);}
   else if('g3Zu' in d)panelZu(); // data-g3-zu ohne Wert: dataset liefert "" (falsy) – das ✕ schloss das Feld nie
   else if(d.g3Hinfliegen){const [art,k]=d.g3Hinfliegen.split(':');const o=art==='mond'?moons.find(m=>Number(m.planet.userData.data.position)===Number(k))?.mesh:art==='tor'?moons.find(m=>Number(m.planet.userData.data.position)===Number(k))?.tor?.ring:art==='debris'||art==='planet'?planets.find(m=>Number(m.userData.data.position)===Number(k)):null;if(o)hinfliegen({object:o,point:o.getWorldPosition(new THREE.Vector3())});else if(art==='debris'){}panelZu();}
   else if(d.g3Folgen){const [art,k]=d.g3Folgen.split(':');const w=art==='wk'?weltenkruemmer[Number(k)]:null,o=w?.blick||w?.inst.objekt;if(o)fliegeZu(o,w?.blick?70:34,null,{yaw:yaw,pitch:.42});panelZu();}
   else if(d.g3==='in'||d.g3==='out'){if(zoomGehalten)zoomGehalten=false;else zoomSchritt(d.g3==='in'?.62:1.6);}
   else if(d.g3==='blick')ansichtWechseln();
   else if(d.g3==='mitte')ueberblick();
   else if(d.g3==='heimat'){listeZu();panelZu();heimatFliegen();}
   // Spielknöpfe im Planetenfeld laufen über ihre gewohnten Dokument-Haken weiter; das Feld schließt sich danach.
   else if(b.closest('.g3-aktionen'))setTimeout(panelZu,0);
  });
 }
 function chip(attr,text,aktuell,liste){return `<button type="button" class="g3-chip${aktuell?' g3-aktiv':''}" ${attr}${aktuell?' aria-current="step"':''}${liste?' aria-haspopup="true"':''}>${text}${liste?'<span class="g3-pfeil-ab" aria-hidden="true">▾</span>':''}</button>`;}
 function pfadZeichnen(){
  if(!overlayEl)return;const gt='<span class="g3-trenner" aria-hidden="true">›</span>';
  overlayEl.querySelector('.g3-pfad').innerHTML=(stage!=='galaxy'?'<button type="button" class="g3-chip g3-hoch" data-g3-ebene="hoch" aria-label="Eine Ebene zurück">‹</button>':'')
   +chip('data-g3-ebene="galaxie"','Galaxie',stage==='galaxy')+gt
   +chip('data-g3-liste="sektor" aria-label="Sektor wählen"',stage==='galaxy'?'Sektor':'Sektor '+sector,stage==='sector',true)
   +(stage==='galaxy'?'':gt+chip('data-g3-liste="system" aria-label="Sonnensystem wählen"',stage==='system'?sector+':'+system:'System',stage==='system',true));
 }
 function listeOeffnen(art){
  const liste=overlayEl.querySelector('.g3-liste'),w=profile(),eigeneSek=new Set((view?.planeten||[]).map(p=>Number(p.koord.split(':')[0])));panelZu();
  let kopf,knoepfe;
  if(art==='sektor'){kopf='Sektor wählen';knoepfe=Array.from({length:w.sektoren},(_,i)=>{const s=i+1,an=stage!=='galaxy'&&s===sector;return `<button type="button" data-g3-sektor="${s}"${an?' aria-current="true"':''}>${eigeneSek.has(s)?'<b class="g3-eigen" aria-label="eigene Kolonie">◆</b>':''}Sektor ${s}</button>`;}).join('');}
  else{
   const daten=cache.get(sector)||[],eigeneSys=new Set((view?.planeten||[]).filter(p=>Number(p.koord.split(':')[0])===sector).map(p=>Number(p.koord.split(':')[1])));
   kopf='Sonnensystem in Sektor '+sector;
   knoepfe=Array.from({length:w.systeme_je_sektor},(_,i)=>{const n=i+1,s=daten.find(x=>x.system===n),mark=s?GalaxyModel.summary(s.plaetze||[]):null,an=stage==='system'&&n===system;
    return `<button type="button" data-g3-system="${n}" class="g3-sys g3-zustand-${mark?.state||'unknown'}${mark?' g3-rel-'+mark.relation:''}"${an?' aria-current="true"':''} aria-label="System ${sector}:${n}${eigeneSys.has(n)?' · eigene Kolonie':''}">${eigeneSys.has(n)?'<b class="g3-eigen" aria-hidden="true">◆</b>':''}${n}</button>`;}).join('');
  }
  liste.dataset.art=art;liste.innerHTML=`<header><strong>${kopf}</strong><button type="button" class="g3-x" data-g3-liste="${art}" aria-label="Auswahl schließen">✕</button></header><div class="g3-raster g3-raster-${art}">${knoepfe}</div>`;
  liste.hidden=false;for(const c of overlayEl.querySelectorAll('[data-g3-liste]'))c.setAttribute('aria-expanded',String(c.dataset.g3Liste===art));neu();
  liste.querySelector('[aria-current="true"]')?.scrollIntoView?.({block:'nearest'});
 }
 function listeZu(){const liste=overlayEl?.querySelector('.g3-liste');if(!liste||liste.hidden)return false;liste.hidden=true;for(const c of overlayEl.querySelectorAll('[data-g3-liste]'))c.setAttribute('aria-expanded','false');neu();return true;}
 function panelZu(){offenesFeld=null;const panel=overlayEl?.querySelector('.g3-panel');if(!panel||panel.hidden)return false;panel.hidden=true;selected=0;updateOrbits();labels.forEach(l=>{if(l.kind==='planet')l.el.setAttribute('aria-pressed','false');});neu();return true;}
 // Eine Ebene nach oben: erst Auswahl/Planetenfeld schließen, dann System → Sektor → Galaxie (Pfad „‹“ und Android-Zurück).
 function hoch(){if(listeZu()||panelZu())return true;if(stage==='system'){void showSector(sector);return true;}if(stage==='sector'){showGalaxy();return true;}return false;}
 function knopf(attrs,text,haupt){return `<button type="button" class="${haupt?'g3-haupt':'secondary'}" ${attrs}>${text}</button>`;}
 function panelZeigen(p){
  if(!overlayEl||!p)return;listeZu();selected=p.position;updateOrbits();neu();
  labels.forEach(l=>{if(l.kind==='planet')l.el.setAttribute('aria-pressed',String(Number(l.key)===selected));});
  const sysBekannt=!!systemDaten?.bekannt,eigen=own(p.koord),k=esc(p.koord);
  const zone=p.zone||p.habitabilitaet?.zone||'',typ=zone&&window.PlanetKugel?PlanetKugel.typ({...p,zone}):'',bekannt=!!p.bekannt;
  let spieler=p.bekannt||p.besiedlung_bekannt?p.spieler||'':'';const ausMond=!spieler&&!!p.mond?.besitzer&&!p.mond.eigen;if(ausMond)spieler=p.mond.besitzer;
  if(eigen)spieler=view.name;
  const rel=eigen?'eigen':spieler?GalaxyModel.relation({...p,spieler}):'unbekannt',frei=bekannt&&!p.spieler&&(p.status==='frei'||p.bewohnt===false),fremd=!!spieler&&!eigen;
  const bild=(typ&&PlanetKugel.kugelbild('planets',typ))||(bekannt&&zone?planetArt({...p,zone}):'');
  const typName=typ?String(window.STERNEN_ART?.maps?.[`planets|${typ}|`]?.label||'').replace(/^Planet\s+/,'').replace(/\s*\(.*\)\s*$/,''):'';
  const eigenP=eigen?view.planeten.find(x=>x.koord===p.koord):null,name=(eigenP?Planetenname.name(eigenP):p.name)||(eigen?(eigenP?.heimat?'Heimatwelt':'Kolonie'):fremd?(p.heimat?'Heimatwelt':(view.berichte||[]).some(b=>b.ziel===p.koord)?'Kolonie':'Bewohnt'):frei?'Freier Planet':bekannt?'Unbewohnt':'Unerforscht');
  const zustand=(p.verwuestet||(view.invasionen||[]).find(i=>i.planet===p.koord)?.verwuestet)?(eigen?'Deine Kolonie · verwüstet':'Verwüstet · Besatzung im Orbit'):eigen?(eigenP?.heimat?'Deine Heimatwelt':'Deine Kolonie'):fremd?(ausMond?'Besiedelt laut Mond · Planet nicht erkundet':'Bewohnt · '+(REL_TEXT[rel]||'Neutral')+(p.besiegt?' · besiegt':'')):frei?'Frei · kolonisierbar':bekannt?'Erkundet · unbewohnt':'Noch nicht erkundet';
  const tf=(view.truemmer||[]).find(t=>t.koord===p.koord),m=p.mond;
  const inv=(view.invasionen||[]).find(i=>i.planet===p.koord),verwuestet=!!(p.verwuestet||inv?.verwuestet),besatzer=verwuestet&&!!inv&&['angreifer','angriffshelfer'].includes(inv.rolle),invEigen=!!eigenP&&!!(eigenP.invasion?.wellen||inv?.rolle==='verteidiger'),orbit=p.orbitflotten||[];
  // Alle Infos, die Sicht und Galaxie-Werkzeug hergeben, und jede passende Flottenaktion (vorhandene Spielknöpfe).
  const hab=(p.habitabilitaet?.herkunft||[]).find(h=>h.volk===view.volk),rangE=(world?.rangliste||[]).find(x=>x[1]===spieler),flotten=(view.flotten||[]).filter(f=>f.ziel===p.koord&&f.zustand!=='rueckflug'),angr=(view.angriffe||[]).filter(a=>a.ziel===p.koord),zeilen=[];
  if(hab)zeilen.push(['Eignung für dein Volk',eignungBlock(p),true,true]);
  if(p.felder)zeilen.push(['Felder',fmt(p.felder)]);
  if(p.beziehung?.allianz_tag&&!eigen)zeilen.push(['Allianz','['+p.beziehung.allianz_tag+']'+(p.beziehung.allianz?' '+p.beziehung.allianz:'')]);
  if(rangE&&fremd)zeilen.push(['Rang',rangE[0]+' · '+fmt(Number(rangE[2])||0)+' Punkte']);
  {const st=[p.urlaub?'Urlaubsmodus':'',p.inaktiv_tage>=7?'inaktiv seit '+p.inaktiv_tage+' Tagen':'',p.gesperrt?'gesperrt':'',p.besiegt?'besiegt':'',Number.isFinite(p.aktivitaet_minuten)&&p.aktivitaet_minuten<60?'aktiv vor '+p.aktivitaet_minuten+' min':''].filter(Boolean);if(st.length)zeilen.push(['Status',st.join(' · ')]);}
  if(verwuestet)zeilen.push(['Invasion',`Verwüstet – ${besatzer?'dein Kolonieschiff (Kampfkolonisieren oder Kolonisieren) landet und übernimmt, sobald ein Kolonieplatz frei ist':eigen?'Kontrolle verloren; neue Truppen besetzen den Rückhalt wieder':'die Besatzung hält den Boden'}`]);
  else if(invEigen){const w=eigenP?.invasion?.wellen||inv?.wellen;zeilen.push(['Invasion',w?`Linie ${w.linie} von ${w.linien_anzahl} · nächste Welle in ${GameTime.duration(Math.max(0,(Number(w.naechste_welle)||0)-GameTime.now())/Math.max(1,world?.tempo||1))}`:'Bombardierung läuft']);}
  for(const o of orbit)zeilen.push(['Gestrandet',(o.eigen?'Deine Flotte':o.spieler)+' · '+fmt(o.schiffe)+(Number(o.schiffe)===1?' Schiff':' Schiffe')+(o.nur_heim?' · nur noch Heimflug':'')]);
  {const unterwegs=flotten.filter(f=>f.zustand!=='im_orbit'),orbitF=flotten.filter(f=>f.zustand==='im_orbit');if(unterwegs.length)zeilen.push(['Deine Flotten',unterwegs.length+' unterwegs · nächste in '+GameTime.duration(Math.max(0,Math.min(...unterwegs.map(f=>f.ankunft_sekunden||0))-GameTime.now())/Math.max(1,world?.tempo||1))]);if(orbitF.length)zeilen.push(['Im Orbit',orbitF.length+(orbitF.length===1?' eigene Flotte':' eigene Flotten')]);}
  if(angr.length)zeilen.push(['⚠ Angriff',angr.length+(angr.length>1?' feindliche Flotten':' feindliche Flotte')+' im Anflug']);
  const zusatz=zeilen.map(([a,b,roh,voll])=>`<dt${voll?' class="g3-voll"':''}>${esc(a)}</dt><dd${voll?' class="g3-voll"':''}>${roh?b:esc(b)}</dd>`).join('');
  const scan=(view.berichte||[]).filter(b=>b.ziel===p.koord).sort((a,b)=>(b.zeit||0)-(a.zeit||0))[0];let scanHtml='';
  if(scan){const liste=o=>Object.entries(o||{}).filter(([,n])=>Number(n)>0).sort((a,b)=>b[1]-a[1]),kurz=(o,max=4)=>{const l=liste(o);return l.length?l.slice(0,max).map(([k,n])=>fmt(n)+' '+named(k)).join(' · ')+(l.length>max?' · +'+(l.length-max)+' weitere':''):'keine';};
   const alter=Number.isFinite(scan.alter_stunden)?(scan.alter_stunden<1?'unter 1 h':scan.alter_stunden<48?scan.alter_stunden+' h':Math.round(scan.alter_stunden/24)+' Tagen'):'';
   scanHtml=`<details class="g3-scan"><summary>Letzter Scan${alter?' · vor '+esc(alter):''}${scan.geschaetzt?' · geschätzt':''}<small>Stärke ${esc(scan.scan_metadaten?.scanstaerke??'?')}</small></summary><dl>`
    +`<dt>Flotte</dt><dd>${scan.schiffe?esc(kurz(scan.schiffe)):'nicht erfasst'}</dd><dt>Verteidigung</dt><dd>${scan.verteidigung?esc(kurz(scan.verteidigung)):'nicht erfasst'}</dd>`
    +(scan.bestand?`<dt>Rohstoffe</dt><dd>${esc(kurz({erz:scan.bestand.erz,kristall:scan.bestand.kristall,deuterium:scan.bestand.deuterium},3))}</dd>`:'')
    +(scan.gebaeude?`<dt>Gebäude</dt><dd>${esc(kurz(scan.gebaeude,4))}</dd>`:'')+(scan.vollscan?.bevoelkerung?`<dt>Bevölkerung</dt><dd>${fmt(scan.vollscan.bevoelkerung)}</dd>`:'')+`</dl></details>`;}
  else if(fremd)scanHtml='<p class="g3-hinweis">Noch kein Spionagebericht · eine Sonde zeigt Flotte, Verteidigung und Rohstoffe.</p>';
  const plan=(auftrag,text,haupt)=>knopf(`data-ff-ziel-plan="${k}" data-ff-auftrag-plan="${auftrag}"`,text,haupt);
  const phalanx=(view?.monde?.eigene||[]).find(x=>{const [ms,mn]=String(x.koord).split(':').map(Number);return x.tiefenphalanx?.stufe_wirksam>0&&ms===sector&&Math.abs(mn-system)<=x.tiefenphalanx.reichweite_systeme;});
  const aktionen=[];
  if(invEigen)aktionen.push(knopf(`data-bk-oeffnen="${k}"`,'Bodenkampf · Linien verteilen',true));
  if(besatzer)aktionen.push(plan('kampfkolonisieren','Kolonieschiff landen (Kampfkolonisieren)',true),plan('kolonisieren','Kolonisieren'),knopf(`data-bk-oeffnen="${k}"`,'Bodenkampf ansehen'));
  for(const o of orbit){const f=esc(o.flotte);if(o.eigen)aktionen.push(knopf(`data-gt-orbit="weiter" data-gt-orbit-flotte="${f}" data-gt-orbit-ziel="${k}"`,'Gestrandete Flotte: weiterfliegen'),knopf(`data-gt-orbit="versorgen" data-gt-orbit-flotte="${f}" data-gt-orbit-ziel="${k}"`,'Gestrandete Flotte versorgen'));else aktionen.push(knopf(`data-gt-orbit="angriff" data-gt-orbit-flotte="${f}" data-gt-orbit-ziel="${k}"`,'Gestrandete Flotte von '+esc(o.spieler)+' angreifen'));}
  if(eigen)aktionen.push(knopf(`data-space-own="${k}"`,'Zum Planeten',!invEigen),plan('transport','Transport'),plan('stationieren','Stationieren'));
  else if(fremd){
   aktionen.push(ausMond?knopf(`data-explore="${k}" data-system="false"`,'Erkunden',true):knopf(`data-gt-spion="${k}"`,'Spionage',true));
   if(rel==='verbuendet')aktionen.push(plan('transport','Transport'),plan('halten','Verteidigen (Halten)'));
   else{aktionen.push(plan('angriff','Angriff planen'));if(bekannt&&!p.heimat&&!p.besiegt)aktionen.push(plan('kampfkolonisieren','Invasion planen'));{const rk=window.GalaxieTabelle?.raketenLage?.(k);aktionen.push(rk?.grund?`<button type="button" class="secondary" disabled title="${esc(rk.grund)}">${esc(window.Auftraege?.info?.('rakete')?.name||'Raketenschlag')}</button><small class="g3-grund">${esc(rk.grund)}</small>`:knopf(`data-gt-rakete="${k}"`,window.Auftraege?.info?.('rakete')?.name||'Raketenschlag'));}if(phalanx)aktionen.push(knopf(`data-gt-phalanx="${k}" data-phalanx-mond="${esc(phalanx.koord)}"`,'Phalanx-Scan'));}
   aktionen.push(knopf(`data-mail-to="${esc(spieler)}"`,'Nachricht'),knopf(`data-gt-kontakt="${esc(spieler)}"`,'Kontakt / Vertrag'));
  }
  // Freier Platz: Kolonisieren verlangt eigene Planetenaufklärung (Kern: planetenwissen.bekannt) – ohne sie ist „Erkunden (Sonde)“
  // der erste Schritt, sonst landete der Spieler im Assistenten nur bei „zuerst aufklären“ (Spielprobe 20.10.2026).
  else if(frei)aktionen.push(...(bekannt?[knopf(`data-colonize="${k}"`,'Kolonisieren',true),knopf(`data-explore="${k}" data-system="false"`,'Erkunden')]
   :[knopf(`data-explore="${k}" data-system="false"`,'Erkunden (Sonde)',true),knopf(`data-colonize="${k}"`,'Kolonisieren')]),plan('transport','Transport'));
  else aktionen.push(sysBekannt?knopf(`data-gt-spion="${k}"`,'Erkunden (Sonde)',true):knopf(`data-scan-system="${sector}:${system}:1"`,'System erkunden',true));
  if(m)aktionen.push(m.eigen||eigen?knopf(`data-mond-oeffnen="${k}"`,'Mond öffnen'):knopf(`data-gt-mondziel="${k}"`,'Flug zum Mond'));
  const frischesFeld=!tf&&sys3?.felder.some(f=>f.planet?.userData?.data?.koord===p.koord);
  aktionen.push(knopf('data-g3-hinfliegen="planet:'+p.position+'"','Hinfliegen'));
  const panel=overlayEl.querySelector('.g3-panel');
  panel.innerHTML=`<header>${bild?`<img src="${esc(bild)}" alt="" class="g3-kugel${bekannt||eigen?'':' g3-unerkundet'}">`:'<span class="g3-kugel g3-leer" aria-hidden="true">?</span>'}<div><span class="eyebrow">PLANET ${p.position} · [${k}]</span><h3>${esc(name)}</h3><p>${esc(zustand)}</p></div><button type="button" class="g3-x" data-g3-zu aria-label="Planetenfeld schließen">✕</button></header>
<dl><dt>Besitzer</dt><dd class="g3-rel g3-rel-${rel}">${spieler?(eigen?esc(spieler):`<button type="button" class="g3-spieler" data-akte="${esc(spieler)}" aria-label="Spielerprofil von ${esc(spieler)} öffnen">${esc(spieler)}</button>`)+' · '+esc(REL_TEXT[rel]||''):'—'}</dd><dt>Typ</dt><dd>${esc([typName&&(bekannt||eigen)?typName:'',ZONEN[zone]||''].filter(Boolean).join(' · ')||'Unbekannt')}</dd>${zusatz}${m?`<dt>Mond</dt><dd>${esc((m.name||'Mond')+(window.Mondseite?.typName?' · '+Mondseite.typName(m.typ):'')+(m.durchmesser||m.groesse?' · '+fmt(m.durchmesser||m.groesse)+' km':'')+(m.besitzer&&!eigen?' · '+m.besitzer:''))}${vernichtenChip(p)?`<span class="g3-vernichten-zeile">${vernichtenChip(p)}</span>`:''}</dd>`:''}${tf||frischesFeld?`<dt>Trümmer</dt><dd class="g3-abbau-zeile">${abbauChip(p.koord,tf)}</dd>`:''}</dl>
${scanHtml}<div class="g3-aktionen">${aktionen.join('')}</div>`;
  panel.hidden=false;sichtbarMachen();offenesFeld={kind:'planet',key:p.position};
  freiHalten(planets.find(m=>Number(m.userData.data.position)===p.position));
 }
 // Untere Bildschirmleiste (fest) – Höhe der Karte bis dorthin; Beschriftungen darunter werden ausgeblendet.
 function leisteOben(){let oben=innerHeight;for(const id of ['themenleiste','tabs']){const e=document.getElementById(id);if(!e||getComputedStyle(e).position!=='fixed')continue;const r=e.getBoundingClientRect();if(r.height>0&&r.top>innerHeight/2)oben=Math.min(oben,r.top);}return oben;}
 // Handy: Karte füllt den Platz zwischen Planeten-/Ressourcenkopf und unterer Leiste. Die Kontozeile darüber darf
 // wegscrollen (ziel), damit die Karte ~⅔ der Höhe bekommt; über dem Kopf bleibt die Seite normal scrollbar.
 let scrollZiel=0;
 // Lage der Bedienelemente über der Karte für die Beschriftungen. Je Bild gelesen erzwang das ein Neu-Layout der ganzen
 // Spielseite (am Handy mehrere ms je Bild, sobald das Spiel daneben etwas geändert hatte). Gilt 150 ms; Scrollen,
 // Größenänderung und jedes Loslassen verwerfen sie sofort.
 let uiLage=null;
 addEventListener('scroll',()=>{uiLage=null;},{passive:true,capture:true});addEventListener('resize',()=>{uiLage=null;});addEventListener('pointerup',()=>{uiLage=null;},true);
 function uiLageLesen(){const jetzt=performance.now();if(uiLage&&jetzt-uiLage.t<150)return uiLage;
  const hostRect=host.getBoundingClientRect(),ui=[];
  for(const element of host.querySelectorAll('.space-camera-controls,.space-instructions,#galaxy-loading:not([hidden]),.g3-pfad,.g3-tabelle,.g3-seite,.g3-liste:not([hidden]),.g3-panel:not([hidden])')){const r=element.getBoundingClientRect();if(r.width>0)ui.push({x:r.x-hostRect.x,y:r.y-hostRect.y,w:r.width,h:r.height});}
  return uiLage={t:jetzt,hostRect,ui,leiste:handy.matches?leisteOben():innerHeight};}
 function handyLayout(){
  if(!host)return;const an=handy.matches;host.dataset.handy=String(an);if(!an){host.style.removeProperty('--g3-hoehe');return;}
  const oben=host.getBoundingClientRect().top+scrollY,kopf=document.getElementById('rahmen-planetenwahl'),kopfOben=kopf?kopf.getBoundingClientRect().top+scrollY:0;
  scrollZiel=kopf&&kopf.getBoundingClientRect().height&&kopfOben<oben?Math.max(0,Math.round(kopfOben-6)):0;
  const hoehe=Math.round(Math.max(320,leisteOben()-(oben-scrollZiel)-6))+'px';if(host.style.getPropertyValue('--g3-hoehe')!==hoehe)host.style.setProperty('--g3-hoehe',hoehe);
 }
 function sichtbarMachen(){if(!handy.matches||!host)return;const r=host.getBoundingClientRect(),unten=leisteOben()-4;if(r.bottom>unten)scrollBy({top:Math.min(r.bottom-unten,Math.max(0,r.top)),behavior:'instant'});}
 // Antippen neben einem Planeten oder Stern: nächstes Ziel im Umkreis (Fingerbreite) statt Fehltreffer.
 function naechstes(e){
  if(!handy.matches&&e.pointerType!=='touch')return null;const r=canvas.getBoundingClientRect(),v=new THREE.Vector3();let best=null,bestD=handy.matches?44:30;
  for(const t of targets){if(t.isInstancedMesh)continue;t.getWorldPosition(v);v.project(camera);if(v.z>1||v.z<-1)continue;const d=Math.hypot((v.x+1)*r.width/2-(e.clientX-r.left),(1-v.y)*r.height/2-(e.clientY-r.top));if(d<bestD){bestD=d;best={object:t};}}
  return best;
 }
 function saumTextur(){const c=document.createElement('canvas');c.width=c.height=128;const x=c.getContext('2d'),g=x.createRadialGradient(64,64,0,64,64,64);g.addColorStop(0,'rgba(255,255,255,0)');g.addColorStop(.76,'rgba(255,255,255,0)');g.addColorStop(.81,'rgba(255,255,255,.75)');g.addColorStop(.88,'rgba(255,255,255,.22)');g.addColorStop(1,'rgba(255,255,255,0)');x.fillStyle=g;x.fillRect(0,0,128,128);const t=new THREE.CanvasTexture(c);t.colorSpace=THREE.SRGBColorSpace;return t;}
 function ringTextur(){const c=document.createElement('canvas');c.width=c.height=96;const x=c.getContext('2d');x.strokeStyle='#fff';x.lineWidth=5;x.shadowColor='#fff';x.shadowBlur=6;x.beginPath();x.arc(48,48,38,0,Math.PI*2);x.stroke();const t=new THREE.CanvasTexture(c);t.colorSpace=THREE.SRGBColorSpace;return t;}
 // =================================================================== 3D-Engine (Karl, 19.10.2026) ==========
 // Sonnensystem wie in einem Raumspiel: HDR-Szene mit Nachleuchten, Nebelhimmel (einmal auf der GPU gebacken), Sonne als
 // Shader mit Korona, Planeten mit Atmosphäre, Wolken und Rohstoffringen, instanzierte Asteroiden und Trümmer, Dysonbau,
 // eigene und feindliche Flotten, Weltenkrümmer live. Lagen und Bewegungen sind Darstellung; Daten nur aus Sicht und
 // Galaxie-Werkzeug (Besitz, Monde, Trümmer, Gürtel, Dyson, Flotten).
 const GLSL_RAUSCHEN=`
vec3 m289(vec3 x){return x-floor(x*(1./289.))*289.;}vec4 m289(vec4 x){return x-floor(x*(1./289.))*289.;}
vec4 perm(vec4 x){return m289(((x*34.)+1.)*x);}vec4 tis(vec4 r){return 1.79284291400159-.85373472095314*r;}
float snoise(vec3 v){const vec2 C=vec2(1./6.,1./3.);const vec4 D=vec4(0.,.5,1.,2.);vec3 i=floor(v+dot(v,C.yyy));vec3 x0=v-i+dot(i,C.xxx);
 vec3 g=step(x0.yzx,x0.xyz);vec3 l=1.-g;vec3 i1=min(g.xyz,l.zxy);vec3 i2=max(g.xyz,l.zxy);vec3 x1=x0-i1+C.xxx;vec3 x2=x0-i2+C.yyy;vec3 x3=x0-D.yyy;
 i=m289(i);vec4 p=perm(perm(perm(i.z+vec4(0.,i1.z,i2.z,1.))+i.y+vec4(0.,i1.y,i2.y,1.))+i.x+vec4(0.,i1.x,i2.x,1.));
 float n_=.142857142857;vec3 ns=n_*D.wyz-D.xzx;vec4 j=p-49.*floor(p*ns.z*ns.z);vec4 x_=floor(j*ns.z);vec4 y_=floor(j-7.*x_);
 vec4 x=x_*ns.x+ns.yyyy;vec4 y=y_*ns.x+ns.yyyy;vec4 h=1.-abs(x)-abs(y);vec4 b0=vec4(x.xy,y.xy);vec4 b1=vec4(x.zw,y.zw);
 vec4 s0=floor(b0)*2.+1.;vec4 s1=floor(b1)*2.+1.;vec4 sh=-step(h,vec4(0.));vec4 a0=b0.xzyw+s0.xzyw*sh.xxyy;vec4 a1=b1.xzyw+s1.xzyw*sh.zzww;
 vec3 p0=vec3(a0.xy,h.x);vec3 p1=vec3(a0.zw,h.y);vec3 p2=vec3(a1.xy,h.z);vec3 p3=vec3(a1.zw,h.w);
 vec4 nr=tis(vec4(dot(p0,p0),dot(p1,p1),dot(p2,p2),dot(p3,p3)));p0*=nr.x;p1*=nr.y;p2*=nr.z;p3*=nr.w;
 vec4 m=max(.6-vec4(dot(x0,x0),dot(x1,x1),dot(x2,x2),dot(x3,x3)),0.);m=m*m;return 42.*dot(m*m,vec4(dot(p0,x0),dot(p1,x1),dot(p2,x2),dot(p3,x3)));}
float fbm(vec3 p){float f=0.,a=.5;for(int i=0;i<5;i++){f+=a*snoise(p);p=p*2.03+vec3(1.7,9.2,3.1);a*=.5;}return f;}
float fbm3(vec3 p){float f=0.,a=.5;for(int i=0;i<3;i++){f+=a*snoise(p);p=p*2.07+vec3(4.1,1.3,7.7);a*=.5;}return f;}
`;
 const VS_QUAD='varying vec2 vUv;void main(){vUv=uv;gl_Position=vec4(position.xy,0.,1.);}';
 const zeitU={value:0};// gemeinsame Shaderzeit (Sekunden)
 // ---- Himmel: Nebel in voller Auflösung einmal auf der GPU backen, Sterne als funkelnde Punkte mit Beugungskreuz.
 let himmel=null;
 function himmelBauen(){
  /* Software-Rendering (SwiftShader/llvmpipe, Erkennung vor dem Backen): klein backen, sonst blockiert das Rauschen dort das erste Bild */
  const w=softwareRender?512:handy.matches?2048:4096,h=w/2,rt=new THREE.WebGLRenderTarget(w,h,{depthBuffer:false});rt.texture.generateMipmaps=true;rt.texture.minFilter=THREE.LinearMipmapLinearFilter;
  const mat=new THREE.ShaderMaterial({vertexShader:VS_QUAD,depthTest:false,depthWrite:false,fragmentShader:GLSL_RAUSCHEN+`varying vec2 vUv;
void main(){float lon=(vUv.x-.5)*6.2831853,lat=(vUv.y-.5)*3.1415927;vec3 d=vec3(cos(lat)*cos(lon),sin(lat),cos(lat)*sin(lon));
 vec3 s=vec3(3.1,7.7,1.3);float band=exp(-pow(dot(d,normalize(vec3(.28,1.,.17)))*3.,2.));
 float n1=fbm(d*2.1+s),n2=fbm(d*3.9+s*1.7+5.);float w=fbm(d*1.4+vec3(n1,n2,0.)*.9+s*.3);float v=fbm(d*6.3+w*1.2+s*2.);
 vec3 c=vec3(.001,.0016,.004);
 float neb=smoothstep(.12,.8,w)*.5;
 c+=vec3(.025,.11,.14)*pow(neb,2.2)*.55;
 c+=vec3(.16,.05,.22)*pow(smoothstep(.1,.85,n2),2.4)*.32*(.25+band);
 c+=vec3(.30,.07,.12)*pow(max(0.,fbm(d*3.2+s*2.4)),2.6)*.6*band;
 c+=vec3(.34,.18,.07)*pow(max(0.,w),3.2)*.5;
 c+=vec3(.12,.13,.17)*band*(.06+.06*n1);
 c*=1.-.82*smoothstep(.0,.5,v)*band;
 c+=vec3(.9,.8,1.)*pow(max(0.,snoise(d*90.+s)),18.)*.6;
 gl_FragColor=vec4(pow(c*.86,vec3(1./2.1)),1.);}`});
  const q=new THREE.Mesh(new THREE.PlaneGeometry(2,2),mat),sz=new THREE.Scene();sz.add(q);
  const alt=renderer.getRenderTarget();renderer.setRenderTarget(rt);renderer.render(sz,new THREE.OrthographicCamera(-1,1,1,-1,0,1));renderer.setRenderTarget(alt);mat.dispose();q.geometry.dispose();
  rt.texture.colorSpace=THREE.SRGBColorSpace;
  const gr=new THREE.Group();gr.renderOrder=-10;
  const kugel=new THREE.Mesh(new THREE.SphereGeometry(2600,96,48),new THREE.MeshBasicMaterial({map:rt.texture,side:THREE.BackSide,depthWrite:false,toneMapped:false}));kugel.renderOrder=-10;kugel.frustumCulled=false;gr.add(kugel);
  const N=handy.matches?9000:16000,pos=new Float32Array(N*3),farbe=new Float32Array(N*3),art=new Float32Array(N*2);let s=97;const z=()=>(s=(s*16807)%2147483647)/2147483647;
  const tempFarben=[[.62,.74,1],[.82,.88,1],[1,1,1],[1,.93,.78],[1,.78,.55],[1,.62,.45]],c=new THREE.Color();
  for(let i=0;i<N;i++){const u=z()*2-1,a=z()*Math.PI*2,r=Math.sqrt(1-u*u),R=2350;pos.set([Math.cos(a)*r*R,u*R,Math.sin(a)*r*R],i*3);const f=tempFarben[Math.floor(z()*tempFarben.length)];farbe.set(f,i*3);art.set([.55+Math.pow(z(),16)*7,z()*6.28],i*2);}
  const g=new THREE.BufferGeometry();g.setAttribute('position',new THREE.BufferAttribute(pos,3));g.setAttribute('farbe',new THREE.BufferAttribute(farbe,3));g.setAttribute('art',new THREE.BufferAttribute(art,2));
  const pm=new THREE.ShaderMaterial({uniforms:{zeit:zeitU,px:{value:renderer.getPixelRatio()}},transparent:true,depthWrite:false,blending:THREE.AdditiveBlending,
   vertexShader:'attribute vec3 farbe;attribute vec2 art;uniform float zeit,px;varying vec3 vF;varying float vG;void main(){vF=farbe*(.75+.25*sin(zeit*(1.3+art.y)+art.y*7.));vG=art.x;gl_Position=projectionMatrix*modelViewMatrix*vec4(position,1.);gl_PointSize=art.x*px*1.6;}',
   fragmentShader:'varying vec3 vF;varying float vG;void main(){vec2 c=gl_PointCoord-.5;float r=length(c);float k=smoothstep(.5,.0,r);k=k*k;if(vG>4.6){k+=max(0.,1.-abs(c.x)*22.)*max(0.,1.-abs(c.y)*2.2)*.7+max(0.,1.-abs(c.y)*22.)*max(0.,1.-abs(c.x)*2.2)*.7;}gl_FragColor=vec4(vF*k*.95,k);}'});
  const pts=new THREE.Points(g,pm);pts.frustumCulled=false;pts.renderOrder=-9;gr.add(pts);
  scene.add(gr);himmel=gr;
 }
 // ---- Nebelsystem (Karl 20.10.2026: „im Hintergrund der Sonnensystemansicht zu sehen“): nur in Nebelsystemen des Kerns
 // (jedes vierte System ohne Heimatwelt; gerade Plätze mit natürlichem Xenokristall). Nicht zu verwechseln mit dem
 // Asteroidengürtel (jedes System, Band zwischen Platz 7 und 8) und mit Trümmerfeldern (Wrackwolken an Planeten). Einmal auf
 // der GPU gebacken und global gehalten wie der Sternhimmel (disposeTree räumt die Systemgruppe); je System gedreht und wie der
 // Sternhimmel mit der Kamera mitgeführt – sonst schneidet die Fernebene beim Herauszoomen einen Kreis aus dem Nebel.
 let nebelHimmel=null;
 function nebelHimmelBauen(){
  const w=softwareRender?512:handy.matches?2048:4096,h=w/2,rt=new THREE.WebGLRenderTarget(w,h,{depthBuffer:false});rt.texture.generateMipmaps=true;rt.texture.minFilter=THREE.LinearMipmapLinearFilter;
  const mat=new THREE.ShaderMaterial({vertexShader:VS_QUAD,depthTest:false,depthWrite:false,fragmentShader:GLSL_RAUSCHEN+`varying vec2 vUv;
void main(){float lon=(vUv.x-.5)*6.2831853,lat=(vUv.y-.5)*3.1415927;vec3 d=vec3(cos(lat)*cos(lon),sin(lat),cos(lat)*sin(lon));
 vec3 s=vec3(11.3,4.7,8.9);float q=fbm(d*1.6+s),w=fbm(d*2.4+vec3(q)*1.4+s*.5),f=fbm(d*5.5+w*1.6+s*1.3);
 float dicht=smoothstep(-.15,.75,w)*(.55+.45*smoothstep(-.3,.6,q));float faeden=pow(max(0.,1.-abs(f)*3.2),3.)*dicht;
 vec3 c=mix(vec3(.42,.16,.78),vec3(.10,.62,.66),smoothstep(-.2,.5,q))*dicht*.55+vec3(.78,.22,.52)*faeden*.5+vec3(.9,.8,1.)*faeden*faeden*.25;
 c*=1.-.6*smoothstep(.2,.7,fbm3(d*3.1+s*2.));
 gl_FragColor=vec4(c,1.);}`});
  const q=new THREE.Mesh(new THREE.PlaneGeometry(2,2),mat),sz=new THREE.Scene();sz.add(q);
  const alt=renderer.getRenderTarget();renderer.setRenderTarget(rt);renderer.render(sz,new THREE.OrthographicCamera(-1,1,1,-1,0,1));renderer.setRenderTarget(alt);mat.dispose();q.geometry.dispose();
  rt.texture.colorSpace=THREE.SRGBColorSpace;
  const kugel=new THREE.Mesh(new THREE.SphereGeometry(2450,64,32),new THREE.MeshBasicMaterial({map:rt.texture,side:THREE.BackSide,transparent:true,blending:THREE.AdditiveBlending,depthWrite:false,toneMapped:false,opacity:.72}));
  kugel.renderOrder=-9;kugel.frustumCulled=false;kugel.visible=false;kugel.userData={kind:'nebelhimmel'};scene.add(kugel);nebelHimmel=kugel;
 }
 function nebelHimmelZeigen(an,saat){
  if(!an){if(nebelHimmel)nebelHimmel.visible=false;return;}
  if(!nebelHimmel)nebelHimmelBauen();
  nebelHimmel.rotation.set(((saat*37)%60-30)*Math.PI/180,(saat*53%360)*Math.PI/180,0);nebelHimmel.visible=true;
 }
 // ---- Nachleuchten (Bloom): HDR-Szene → helle Anteile → drei Unschärfestufen → zusammen mit ACES auf den Schirm.
 let bloom=null,softwareRender=false;const bloomGroesse2=new THREE.Vector2();
 function bloomBauen(){
  const gl2=renderer.capabilities.isWebGL2,hdr=gl2&&(renderer.extensions.has('EXT_color_buffer_float')||renderer.extensions.has('EXT_color_buffer_half_float'));
  const mk=uniforms=>({uniforms,vertexShader:VS_QUAD,depthTest:false,depthWrite:false});
  const hell=new THREE.ShaderMaterial({...mk({t:{value:null},schwelle:{value:hdr?.95:.78}}),toneMapped:false,fragmentShader:'uniform sampler2D t;uniform float schwelle;varying vec2 vUv;void main(){vec3 c=texture2D(t,vUv).rgb;if(!(c.r==c.r&&c.g==c.g&&c.b==c.b)||max(c.r,max(c.g,c.b))>6e4)c=vec3(0.);c=min(c,vec3(40.));float l=max(c.r,max(c.g,c.b));gl_FragColor=vec4(c*smoothstep(schwelle,schwelle*1.7+.15,l),1.);}'});
  const blur=new THREE.ShaderMaterial({...mk({t:{value:null},r:{value:new THREE.Vector2()}}),toneMapped:false,fragmentShader:'uniform sampler2D t;uniform vec2 r;varying vec2 vUv;void main(){vec3 s=texture2D(t,vUv).rgb*.227027+(texture2D(t,vUv+r*1.3846154).rgb+texture2D(t,vUv-r*1.3846154).rgb)*.3162162+(texture2D(t,vUv+r*3.2307692).rgb+texture2D(t,vUv-r*3.2307692).rgb)*.0702703;gl_FragColor=vec4(s,1.);}'});
  const misch=new THREE.ShaderMaterial({...mk({tS:{value:null},t1:{value:null},t2:{value:null},t3:{value:null},k:{value:1}}),fragmentShader:'uniform sampler2D tS,t1,t2,t3;uniform float k;varying vec2 vUv;void main(){vec3 s0=texture2D(tS,vUv).rgb;if(!(s0.r==s0.r&&s0.g==s0.g&&s0.b==s0.b))s0=vec3(0.);vec3 b=texture2D(t1,vUv).rgb*.5+texture2D(t2,vUv).rgb*.7+texture2D(t3,vUv).rgb;if(!(b.r==b.r&&b.g==b.g&&b.b==b.b))b=vec3(0.);vec3 c=min(s0,vec3(6e4))+b*k;gl_FragColor=vec4(c,1.);\n#include <tonemapping_fragment>\n#include <colorspace_fragment>\n}'});
  const quad=new THREE.Mesh(new THREE.PlaneGeometry(2,2),hell);quad.frustumCulled=false;const sz=new THREE.Scene();sz.add(quad);
  bloom={an:!softwareRender,hdr,gl2,typ:hdr?THREE.HalfFloatType:THREE.UnsignedByteType,hell,blur,misch,quad,sz,cam:new THREE.OrthographicCamera(-1,1,1,-1,0,1),w:0,h:0,szene:null,stufen:[],zeiten:[]};
 }
 function bloomZiele(w,h){
  if(bloom.w===w&&bloom.h===h)return;bloom.szene?.dispose();for(const s of bloom.stufen){s.a.dispose();s.b.dispose();}
  bloom.szene=new THREE.WebGLRenderTarget(w,h,{type:bloom.typ,samples:bloom.gl2?(bloom.msaa??4):0});bloom.stufen=[2,4,8].map(f=>{const o={type:bloom.typ,depthBuffer:false},x=Math.max(1,Math.round(w/f)),y=Math.max(1,Math.round(h/f));return {a:new THREE.WebGLRenderTarget(x,y,o),b:new THREE.WebGLRenderTarget(x,y,o),x,y};});bloom.w=w;bloom.h=h;
 }
 function durchlauf(mat,ziel){bloom.quad.material=mat;renderer.setRenderTarget(ziel);renderer.render(bloom.sz,bloom.cam);}
 function zeichnen(){
  if(!bloom?.an){renderer.setRenderTarget(null);renderer.render(scene,camera);return;}
  renderer.getDrawingBufferSize(bloomGroesse2);bloomZiele(bloomGroesse2.x,bloomGroesse2.y);
  renderer.setRenderTarget(bloom.szene);renderer.render(scene,camera);
  bloom.hell.uniforms.t.value=bloom.szene.texture;durchlauf(bloom.hell,bloom.stufen[0].a);
  let quelle=bloom.stufen[0].a;for(const s of bloom.stufen){bloom.blur.uniforms.t.value=quelle.texture;bloom.blur.uniforms.r.value.set(1/s.x,0);durchlauf(bloom.blur,s.b);bloom.blur.uniforms.t.value=s.b.texture;bloom.blur.uniforms.r.value.set(0,1/s.y);durchlauf(bloom.blur,s.a);quelle=s.a;}
  const u=bloom.misch.uniforms;u.tS.value=bloom.szene.texture;u.t1.value=bloom.stufen[0].a.texture;u.t2.value=bloom.stufen[1].a.texture;u.t3.value=bloom.stufen[2].a.texture;durchlauf(bloom.misch,null);
 }
 // Qualitätsleiter (Karl 22.10.2026: „das Sonnensystem auf dem Handy ist super laggy“). Der alte Wächter senkte erst unter
 // ~22 Bildern/s (Mittel aus 60 Bildern) und nie wieder zurück – ein Handy mit 25–35 Bildern/s blieb dauerhaft ruckelig.
 // Jetzt: Ziel 60 Bilder/s bei Bedienung, 30 im Ruhezustand (sparen). Liegt der Median von 24 Bildabständen deutlich über
 // dem Ziel, sofort eine Stufe tiefer; nach 20 s ruhigem Lauf eine Stufe höher zur Probe. Scheitert die Probe, bleibt die
 // höhere Stufe 2 min gesperrt (dann 4, 8 … min). Stufe 0: Schärfe 2, Nachleuchten mit 4×MSAA … Stufe 4: Schärfe 1 ohne
 // Nachleuchten (letzter Ausweg und Software-Rendering; das Umschalten übersetzt alle Shader neu, ein spürbarer Halt).
 // Handys beginnen bei Stufe 1 (auf 6 Zoll von Stufe 0 nicht zu unterscheiden).
 // Mit GPU-Zeitmessung (EXT_disjoint_timer_query_webgl2, viele Adreno-/Mali-Geräte) wird nur gesenkt, wenn wirklich die
 // Grafik bremst – liegt es an Skripten der Seite, hilft eine kleinere Stufe nicht und kostet nur Bildqualität.
 const STUFEN=[{px:2,msaa:4,bloom:true},{px:1.75,msaa:2,bloom:true},{px:1.5,msaa:2,bloom:true},{px:1.25,msaa:0,bloom:true},{px:1,msaa:0,bloom:true},{px:1,msaa:0,bloom:false}];
 let pixelStufe=0;const q={z:[],roh:[],warm:20,ruhigSeit:0,sperre:new Map(),fehl:new Map(),probe:-1,probeSeit:0,fest:false,js:[],n:0};
 function stufeSetzen(n){n=Math.max(0,Math.min(STUFEN.length-1,n));pixelStufe=n;const s=STUFEN[n];if(!renderer)return;
  renderer.setPixelRatio(Math.min(devicePixelRatio,s.px));if(bloom){bloom.an=s.bloom&&(!softwareRender||q.fest);bloom.msaa=s.msaa;bloom.w=0;}
  q.z.length=0;q.roh.length=0;q.warm=12;q.ruhigSeit=performance.now();gpuZeit.werte.length=0;neu();}
 const gpuZeit={ext:null,gl:null,offen:[],werte:[]};
 function gpuStart(){const g=gpuZeit;if(!g.ext||g.offen.length>3)return null;const x=g.gl.createQuery();g.gl.beginQuery(g.ext.TIME_ELAPSED_EXT,x);return x;}
 function gpuEnde(x){if(!x)return;const g=gpuZeit;g.gl.endQuery(g.ext.TIME_ELAPSED_EXT);g.offen.push(x);}
 function gpuLesen(){const g=gpuZeit;if(!g.ext||!g.offen.length)return;const gl=g.gl,gestoert=gl.getParameter(g.ext.GPU_DISJOINT_EXT);
  while(g.offen.length){const x=g.offen[0];if(!gl.getQueryParameter(x,gl.QUERY_RESULT_AVAILABLE))break;g.offen.shift();const ns=gl.getQueryParameter(x,gl.QUERY_RESULT);gl.deleteQuery(x);if(!gestoert){g.werte.push(ns/1e6);if(g.werte.length>24)g.werte.shift();}}}
 const median=a=>a.length?[...a].sort((x,y)=>x-y)[a.length>>1]:0;
 function leistung(ms,ziel){if(!bloom||q.fest)return;if(q.warm>0){q.warm--;return;}
  const z=q.z,roh=q.roh,jetzt=performance.now(),letzte=STUFEN.length-1;z.push(ms/ziel);roh.push(ms);if(z.length>24)z.shift();if(roh.length>8)roh.shift();
  // durchgehend sehr langsam (8 Bilder hintereinander über 250 ms): gleich die kleinste Stufe
  if(pixelStufe<letzte&&roh.length===8&&Math.min(...roh)>250){stufeSetzen(letzte);return;}
  if(z.length<24)return;const m=[...z].sort((a,b)=>a-b)[12];
  const zielMs=ziel,gm=gpuZeit.werte.length>=12?median(gpuZeit.werte):null,grafik=gm!=null?gm>zielMs*.7:m*zielMs-median(q.js)>zielMs*.75;
  if(m>1.45&&grafik&&pixelStufe<letzte){
   // Stufe erst kürzlich per Probe erreicht → gescheitert: diese Stufe länger sperren
   if(q.probe===pixelStufe&&jetzt-q.probeSeit<60000){const f=(q.fehl.get(pixelStufe)||0)+1;q.fehl.set(pixelStufe,f);q.sperre.set(pixelStufe,jetzt+120000*2**(f-1));}
   q.probe=-1;stufeSetzen(pixelStufe+1);return;}
  if(m>1.15||gm!=null&&gm>zielMs*.45)q.ruhigSeit=jetzt;
  if(pixelStufe>0&&jetzt-q.ruhigSeit>20000&&(q.sperre.get(pixelStufe-1)||0)<jetzt){q.probe=pixelStufe-1;q.probeSeit=jetzt;stufeSetzen(pixelStufe-1);}
 }
 // ---- Felsen: verformte Ikosaeder (flach schattiert), mehrere Formen für Gürtel, Kuiperring und Trümmer.
 function felsGeometrie(saat,detail=1,platt=1){const g=new THREE.IcosahedronGeometry(1,detail),p=g.attributes.position,v=new THREE.Vector3();let s=saat*977+13;const z=()=>(s=(s*16807)%2147483647)/2147483647;const k=[z()*6,z()*6,z()*6,z()*6];
  for(let i=0;i<p.count;i++){v.fromBufferAttribute(p,i);const n=Math.sin(v.x*2.1+k[0])*Math.sin(v.y*2.7+k[1])*Math.sin(v.z*1.9+k[2])*.34+Math.sin(v.x*5.3+k[1])*Math.sin(v.y*4.7+k[3])*Math.sin(v.z*6.1+k[0])*.12+Math.sin(v.x*11.+k[2])*.04;v.multiplyScalar(1+n);v.y*=(.55+z()*.45)*platt;v.x*=.8+z()*.4;p.setXYZ(i,v.x,v.y,v.z);}
  g.computeVertexNormals();return g;}
 let felsRauheit=null;function felsKarte(){if(felsRauheit)return felsRauheit;const c=document.createElement('canvas');c.width=c.height=256;const x=c.getContext('2d'),d=x.createImageData(256,256);for(let i=0;i<256*256;i++){const px=i%256,py=i>>8,n=128+60*Math.sin(px*.21)*Math.sin(py*.17)+40*Math.sin(px*.67+py*.41)+((px*7919+py*104729)%97)-48;d.data[i*4]=d.data[i*4+1]=d.data[i*4+2]=Math.max(0,Math.min(255,n));d.data[i*4+3]=255;}x.putImageData(d,0,0);felsRauheit=new THREE.CanvasTexture(c);felsRauheit.wrapS=felsRauheit.wrapT=THREE.RepeatWrapping;return felsRauheit;}
 // Ein Feld aus Felsbrocken: n Instanzen in einem Ring (Radius r0..r1, Dicke h); 'taumel' Instanzen drehen sich einzeln.
 function felsRing(opt){const ext=window.Truemmer3D?.felsRing?.(opt);if(ext)return ext;return felsRingEigen(opt);}
 // Rückfall/Standard: verformte Ikosaeder. Truemmer3D (web-client/truemmer3d.js, Sitzung „Realistische Trümmerringe“) darf eine
 // Gruppe mit gleicher Form liefern: InstancedMesh-Kinder mit userData {kind,key} (Antippen) und userData.taumel (Liste) an der Gruppe.
 function felsRingEigen({n,r0,r1,h,groesse,farbe,saat,taumel=0,kind,key,detail=1,metall=0,bogen=0}){
  const gr=new THREE.Group(),formen=[0,1,2,3].map(i=>felsGeometrie(saat+i*31,i===3?Math.min(2,detail+1):detail,kind==='debris'?.45:1));
  const mat=new THREE.MeshStandardMaterial({color:'#ffffff',roughness:metall?.38:.96,metalness:metall,flatShading:true,bumpMap:felsKarte(),bumpScale:.6});
  let s=saat*31+7;const z=()=>(s=(s*16807)%2147483647)/2147483647,dummy=new THREE.Object3D(),c=new THREE.Color(),je=Math.ceil(n/formen.length),dreh=[];
  formen.forEach((geo,fi)=>{const m=new THREE.InstancedMesh(geo,mat,je);m.instanceMatrix.setUsage(THREE.DynamicDrawUsage);m.userData={kind,key};
   for(let i=0;i<je;i++){const a=bogen?(z()-.5)*bogen*(.65+.35*z()):z()*Math.PI*2,r=r0+(r1-r0)*Math.pow(z(),.8),y=(z()-.5)*h*(1-.5*Math.abs((r-(r0+r1)/2)/(r1-r0+1e-6))),gr_=groesse[0]+(groesse[1]-groesse[0])*Math.pow(z(),6);
    dummy.position.set(Math.cos(a)*r,y,Math.sin(a)*r);dummy.rotation.set(z()*6.28,z()*6.28,z()*6.28);dummy.scale.setScalar(gr_);dummy.updateMatrix();m.setMatrixAt(i,dummy.matrix);
    const t=farbe[Math.floor(z()*farbe.length)];c.set(t).multiplyScalar(.75+z()*.45);m.setColorAt(i,c);
    if(dreh.length<taumel&&gr_>groesse[0]*1.4)dreh.push({m,i,pos:dummy.position.clone(),rot:dummy.rotation.clone(),sk:gr_,v:new THREE.Vector3(z()-.5,z()-.5,z()-.5).multiplyScalar(.6)});}
   m.instanceColor.needsUpdate=true;gr.add(m);});
  gr.userData.taumel=dreh;return gr;
 }
 const taumelDummy=new THREE.Object3D();
 function taumeln(gr,dt){const t=gr.userData.taumel;if(!t?.length)return;const benutzt=new Set();for(const d of t){d.rot.x+=d.v.x*dt;d.rot.y+=d.v.y*dt;d.rot.z+=d.v.z*dt;taumelDummy.position.copy(d.pos);taumelDummy.rotation.copy(d.rot);taumelDummy.scale.setScalar(d.sk);taumelDummy.updateMatrix();d.m.setMatrixAt(d.i,taumelDummy.matrix);benutzt.add(d.m);}for(const m of benutzt)m.instanceMatrix.needsUpdate=true;}
 // Staub als weiche Punkte (additiv).
 function staubRing(n,r0,r1,h,farbe,saat,deckung=.5,groesse=1.6,bogen=0){let s=saat;const z=()=>(s=(s*16807)%2147483647)/2147483647,pos=new Float32Array(n*3);for(let i=0;i<n;i++){const a=bogen?(z()-.5)*bogen:z()*6.2832,r=r0+(r1-r0)*z();pos.set([Math.cos(a)*r,(z()-.5)*h,Math.sin(a)*r],i*3);}
  const g=new THREE.BufferGeometry();g.setAttribute('position',new THREE.BufferAttribute(pos,3));return new THREE.Points(g,new THREE.PointsMaterial({map:glow,color:farbe,size:groesse,transparent:true,opacity:deckung,depthWrite:false,blending:THREE.AdditiveBlending}));}
 // ---- Sonne: Granulation und Flecken als Rauschen, Randverdunkelung; Werte > 1 leuchten über das Nachleuchten.
 const KLASSEN=[{n:'G',kern:'#fff1c2',rand:'#ff8f26',licht:'#ffe7bd',hell:2.2,r:9.5},{n:'K',kern:'#ffd29a',rand:'#e2501a',licht:'#ffd0a0',hell:2.1,r:9},{n:'M',kern:'#ffae78',rand:'#b5200c',licht:'#ffb58e',hell:1.9,r:8},{n:'F',kern:'#ffffff',rand:'#ffcf72',licht:'#fff5e8',hell:2.4,r:10},{n:'A',kern:'#f2f6ff',rand:'#8fb8ff',licht:'#e6eeff',hell:2.6,r:10.5}];
 const KLASSEN_NAME={G:'Gelber Zwerg',K:'Oranger Zwerg',M:'Roter Zwerg',F:'Weißgelber Stern',A:'Weißer Stern'};
 function sternklasse(s=sector,n=system){const h=(window.PlanetKugel?.streuung?.(s+':'+n+':stern'))??(s*7919+n*104729);const x=(h%1000)/1000;return x<.4?KLASSEN[0]:x<.65?KLASSEN[1]:x<.8?KLASSEN[2]:x<.92?KLASSEN[3]:KLASSEN[4];}
 function sonneBauen(k){
  const mat=new THREE.ShaderMaterial({uniforms:{zeit:zeitU,a:{value:new THREE.Color(k.kern)},b:{value:new THREE.Color(k.rand)},hell:{value:k.hell}},
   vertexShader:'varying vec3 vN,vP,vV;void main(){vN=normalize(normalMatrix*normal);vP=position;vec4 mv=modelViewMatrix*vec4(position,1.);vV=normalize(-mv.xyz);gl_Position=projectionMatrix*mv;}',
   fragmentShader:GLSL_RAUSCHEN+'uniform float zeit,hell;uniform vec3 a,b;varying vec3 vN,vP,vV;void main(){vec3 p=normalize(vP);float t=zeit*.035;float w=fbm(p*2.6+vec3(t,0.,-t));float g=fbm(p*7.5+w*1.8+vec3(0.,t*2.,0.));float z=snoise(p*34.+g*2.5+t*4.);float fl=smoothstep(.45,.75,fbm3(p*3.3-t*.6));float mu=max(dot(vN,vV),0.);float rand=pow(mu,.5);vec3 c=mix(b,a,clamp(.42+.6*g+.18*z,0.,1.));c*=(1.-.55*fl);c=c*(.5+.8*rand)*hell+a*pow(1.-mu,2.5)*.9*hell;gl_FragColor=vec4(c,1.);}'});
  return new THREE.Mesh(new THREE.SphereGeometry(k.r,128,96),mat);
 }
 // Korona: zur Kamera gedrehte Fläche mit Strahlen und Protuberanzen aus Rauschen (nur außerhalb der Scheibe).
 function koronaBauen(k){const R=k.r/.2;const mat=new THREE.ShaderMaterial({uniforms:{zeit:zeitU,f:{value:new THREE.Color(k.rand).lerp(new THREE.Color(k.kern),.35)}},transparent:true,depthWrite:false,blending:THREE.AdditiveBlending,
  vertexShader:'varying vec2 vUv;void main(){vUv=uv;gl_Position=projectionMatrix*modelViewMatrix*vec4(position,1.);}',
  fragmentShader:GLSL_RAUSCHEN+'uniform float zeit;uniform vec3 f;varying vec2 vUv;void main(){vec2 q=vUv*2.-1.;float r=length(q);if(r<.198)discard;float a=atan(q.y,q.x);float s=fbm3(vec3(cos(a)*3.,sin(a)*3.,r*2.4-zeit*.07))*.5+.5;float pr=pow(max(0.,fbm3(vec3(cos(a)*7.,sin(a)*7.,zeit*.05))),2.)*smoothstep(.36,.2,r);float g=exp(-(r-.2)*9.)*1.1+exp(-(r-.2)*2.6)*.38;float k=g*(.45+1.1*s*smoothstep(.2,.55,r))+pr*2.2;k*=smoothstep(1.,.55,r);gl_FragColor=vec4(f*k*1.9,1.);}'});
  const m=new THREE.Mesh(new THREE.PlaneGeometry(R*2,R*2),mat);m.renderOrder=2;return m;}
 // ---- Atmosphäre: Fresnel-Saum, hell auf der Tagseite, Dämmerung am Terminator.
 function atmosphaere(radius,farbe){return new THREE.Mesh(new THREE.SphereGeometry(radius,64,48),new THREE.ShaderMaterial({uniforms:{f:{value:farbe}},transparent:true,depthWrite:false,blending:THREE.AdditiveBlending,
  vertexShader:'varying vec3 vN,vW;void main(){vN=normalize(mat3(modelMatrix)*normal);vec4 w=modelMatrix*vec4(position,1.);vW=w.xyz;gl_Position=projectionMatrix*viewMatrix*w;}',
  fragmentShader:'uniform vec3 f;varying vec3 vN,vW;void main(){vec3 V=normalize(cameraPosition-vW),N=normalize(vN),L=normalize(-vW);float fr=pow(1.-max(dot(N,V),0.),2.4);float tag=smoothstep(-.3,.5,dot(N,L));float k=fr*(.12+1.25*tag);gl_FragColor=vec4(f*k*1.5,k);}'}));}
 // Stadtlichter: bewohnte Welten leuchten auf der Nachtseite (Häufungen auf „Land“, warm).
 function stadtLichter(radius,saat,farbe){return new THREE.Mesh(new THREE.SphereGeometry(radius,72,54),new THREE.ShaderMaterial({uniforms:{s:{value:saat},f:{value:new THREE.Color(farbe)}},transparent:true,depthWrite:false,blending:THREE.AdditiveBlending,
  vertexShader:'varying vec3 vP,vN,vW;void main(){vP=position;vN=normalize(mat3(modelMatrix)*normal);vec4 w=modelMatrix*vec4(position,1.);vW=w.xyz;gl_Position=projectionMatrix*viewMatrix*w;}',
  fragmentShader:GLSL_RAUSCHEN+'uniform float s;uniform vec3 f;varying vec3 vP,vN,vW;void main(){vec3 p=normalize(vP);float nacht=1.-smoothstep(-.18,.12,dot(normalize(vN),normalize(-vW)));float land=smoothstep(0.,.4,fbm3(p*2.6+s));float c=pow(max(0.,snoise(p*48.+s)),3.)*2.2+pow(max(0.,snoise(p*120.+s*2.)),5.)*1.6;float k=nacht*land*c;gl_FragColor=vec4(f*k*2.2,k);}'}));}
 // Wolkenhülle (Lebens-/Nebelwelten): eigenes Rauschen, dreht langsamer als der Boden, von der Sonne beleuchtet.
 function wolken(radius,saat){return new THREE.Mesh(new THREE.SphereGeometry(radius,64,48),new THREE.ShaderMaterial({uniforms:{zeit:zeitU,s:{value:saat}},transparent:true,depthWrite:false,
  vertexShader:'varying vec3 vP,vN,vW;void main(){vP=position;vN=normalize(mat3(modelMatrix)*normal);vec4 w=modelMatrix*vec4(position,1.);vW=w.xyz;gl_Position=projectionMatrix*viewMatrix*w;}',
  fragmentShader:GLSL_RAUSCHEN+'uniform float zeit,s;varying vec3 vP,vN,vW;void main(){vec3 p=normalize(vP);float n=fbm(p*vec3(1.8,3.6,1.8)+vec3(s,zeit*.01,s*.3));float d=smoothstep(.12,.6,n);float L=dot(normalize(vN),normalize(-vW));float licht=.12+.95*smoothstep(-.15,.6,L);gl_FragColor=vec4(vec3(licht),d*.5);}'}));}
 // Rohstoffring (Sicht: rohstoffring): gebänderter Ring, schräg wie die Planetenachse.
 function rohstoffRing(radius,saat){const g=new THREE.RingGeometry(radius*RING_INNEN,radius*RING_AUSSEN,160,1),p=g.attributes.position,uv=g.attributes.uv,v=new THREE.Vector3();for(let i=0;i<p.count;i++){v.fromBufferAttribute(p,i);uv.setXY(i,(v.length()-radius*1.5)/(radius*.75),.5);}
  const m=new THREE.Mesh(g,new THREE.ShaderMaterial({uniforms:{s:{value:saat}},transparent:true,side:THREE.DoubleSide,depthWrite:false,
   vertexShader:'varying vec2 vUv;varying vec3 vW;void main(){vUv=uv;vec4 w=modelMatrix*vec4(position,1.);vW=w.xyz;gl_Position=projectionMatrix*viewMatrix*w;}',
   fragmentShader:GLSL_RAUSCHEN+'uniform float s;varying vec2 vUv;varying vec3 vW;void main(){float x=vUv.x;float b=.55+.45*snoise(vec3(x*38.,s,0.))+.25*snoise(vec3(x*140.,s*2.,1.));float k=smoothstep(0.,.08,x)*smoothstep(1.,.8,x)*smoothstep(.38,.7,b)*(.55+.45*snoise(vec3(x*9.,s*3.,2.)));gl_FragColor=vec4(vec3(.66,.58,.48),k*.3);}'}));
  return m;}
 // ---- Galaxie: Spiralscheibe (Darstellung) hinter den Systemen.
 let galaxieDreh=null;
 function galaxieScheibe(){
  const N=handy.matches?45000:90000,arme=Math.max(2,Math.min(4,profile().sektoren)),pos=new Float32Array(N*3),col=new Float32Array(N*3),gr=new Float32Array(N);let z=777;const zu=()=>(z=(z*16807)%2147483647)/2147483647;
  const kern=new THREE.Color('#ffe2b0'),arm=new THREE.Color('#a9c8ff'),weiss=new THREE.Color('#ffffff'),rosa=new THREE.Color('#ff6fa6'),c=new THREE.Color();
  for(let i=0;i<N;i++){const r=Math.pow(zu(),1.5)*275,a=(i%arme)/arme*Math.PI*2+Math.log(r+12)*2.35+(zu()-.5)*(.38+34/(r+22)),g=(zu()+zu()+zu()-1.5)/1.5,y=g*(3+20*Math.exp(-r/38));
   pos.set([Math.cos(a)*r,y,Math.sin(a)*r],i*3);const k=Math.min(1,r/90);c.copy(kern).lerp(arm,k).lerp(weiss,zu()*.35);let h=.5+.5*Math.exp(-r/30);if(zu()<.025&&r>60){c.copy(rosa);h=1.3;}col.set([c.r*h,c.g*h,c.b*h],i*3);gr[i]=.5+Math.pow(zu(),5)*2.6;}
  const g=new THREE.BufferGeometry();g.setAttribute('position',new THREE.BufferAttribute(pos,3));g.setAttribute('farbe',new THREE.BufferAttribute(col,3));g.setAttribute('gr',new THREE.BufferAttribute(gr,1));
  const m=new THREE.ShaderMaterial({uniforms:{px:{value:renderer.getPixelRatio()}},transparent:true,depthWrite:false,blending:THREE.AdditiveBlending,
   vertexShader:'attribute vec3 farbe;attribute float gr;uniform float px;varying vec3 vF;void main(){vF=farbe;vec4 mv=modelViewMatrix*vec4(position,1.);gl_Position=projectionMatrix*mv;gl_PointSize=clamp(gr*px*420./-mv.z,1.,9.);}',
   fragmentShader:'varying vec3 vF;void main(){float r=length(gl_PointCoord-.5);float k=smoothstep(.5,0.,r);gl_FragColor=vec4(vF*k*.55,k);}'});
  const scheibe=new THREE.Points(g,m);scheibe.frustumCulled=false;const gruppe=new THREE.Group();gruppe.add(scheibe);
  const k1=sprite('#ffd9a0',150,.55);const k2=sprite('#fff1d6',55,.9);const k3=sprite('#6f8fd8',520,.12);k3.position.y=-6;gruppe.add(k1,k2,k3);group.add(gruppe);galaxieDreh=gruppe;
 }
 // ---- Laufende Teile des gezeigten Sonnensystems.
 let sys3=null;
 function systemSzene(data,ps){q.warm=Math.max(q.warm,20);q.z.length=0;q.roh.length=0;/* Aufbau ruckelt kurz: Wächter neu aufwärmen */
  bauten++;const k=sternklasse(),sz={klasse:k,wolken:[],felder:[],ringe:[],dyson:null,guertel:[],korona:null};sys3=sz;
  nebelHimmelZeigen(data?.nebel===true,sector*100+system);
  // Vollendete Dysonsphäre fängt Licht ab: das System wird sichtbar 20 % dunkler (Spielwirkung im Kern: Sonne 50 % für Nicht-Eigner)
  const schatten=data?.dyson&&['vollendet','verwaist'].includes(data.dyson.phase)?.8:1;sz.schatten=schatten;
  const sonne=sonneBauen(k);sonne.userData={kind:'sonne',key:0};if(sonne.material?.uniforms?.hell)sonne.material.uniforms.hell.value*=schatten;group.add(sonne);targets.push(sonne);
  const korona=koronaBauen(k);korona.material?.uniforms?.f?.value.multiplyScalar(schatten);group.add(korona);sz.korona=korona;group.add(sprite(k.licht,k.r*8,.12*schatten));
  const licht=new THREE.PointLight(k.licht,3.3*schatten,0,0);group.add(licht);
  // Leihlichter für das Aufladen (Karl: die Ladeanimation des 3D-Betrachters war effektvoller): Mündungs- und Reaktorlicht des
  // angreifenden Weltenkrümmers werden je Bild auf diese zwei Lichter übertragen – feste Lichtzahl, kein Neuübersetzen der Shader
  if((view?.flotten||[]).some(f=>f.mission==='mond_zerstoeren'&&(f.schiffe?.weltentruemmer||0)>0&&String(f.ziel||'').startsWith(sector+':'+system+':'))){sz.leihlichter=[0,1].map(()=>{const L=new THREE.PointLight(0xffffff,0,40,1.5);group.add(L);return L;});}
  const gross=handy.matches?1.2:1,ringBild=ringTextur(),saumBild=saumTextur();
  {const n=profile().plaetze_je_system,bahnen={},R=pos=>{const radius=(7.4+pos%3*.8)*gross,p=ps.find(x=>Number(x.position)===pos);return p?planetWeite(p,radius):radius;};let r=k.r*1.75+9,vor=0,band=null;
   for(let pos=1;pos<=n;pos++){const rr=R(pos);if(pos===1)r+=rr;else if(pos===8&&data.asteroidenguertel){const a=r+vor+5;band=[a,a+16];r=band[1]+5+rr;}else r+=vor+rr+6;bahnen[pos]=r;vor=rr;}
   sz.bahnen=bahnen;sz.band=band;sz.rand=r+vor+12;}
  for(const p of ps){const isOwn=own(p.koord),radius=(7.4+p.position%3*.8)*gross,zone=p.zone||p.habitabilitaet?.zone||null,klar=p.bekannt||isOwn,typ=zone&&window.PlanetKugel?PlanetKugel.typ({...p,zone}):'';
   const mesh=new THREE.Mesh(new THREE.SphereGeometry(radius,72,54),new THREE.MeshStandardMaterial({map:zone?surfaceTexture('planets',typ,null,()=>planetTexture(zone,p.position+system)):planetTexture('unknown',p.position+system),color:klar?'#ffffff':'#7f8994',roughness:zone==='leben'?.7:.92,metalness:0}));/* Grundhelligkeit wie planet-kugel.js: die Oberfläche versinkt nachts nie ganz */mesh.material.emissiveMap=mesh.material.map;mesh.material.emissive.set(klar?'#ffffff':'#7f8994');mesh.material.emissiveIntensity=.09;
   // Achse leicht geneigt und fest im Raum (Reihenfolge ZYX: erst Eigendrehung um die eigene Achse, dann Neigung)
   mesh.rotation.order='ZYX';mesh.rotation.z=((window.PlanetKugel?.streuung?.(p.koord+'achse')||p.position*37)%40)/100;
   {const kipp=new THREE.Quaternion().setFromAxisAngle(new THREE.Vector3(0,0,1),mesh.rotation.z);mesh.userData={kind:'planet',key:p.position,data:p,radius,kipp,achse:new THREE.Vector3(0,1,0).applyQuaternion(kipp),e1:new THREE.Vector3(1,0,0).applyQuaternion(kipp),e2:new THREE.Vector3(0,0,1).applyQuaternion(kipp)};}group.add(mesh);targets.push(mesh);planets.push(mesh);
   const besitz=isOwn?'eigen':p.spieler&&(p.bekannt||p.besiedlung_bekannt)?GalaxyModel.relation(p):'';if(besitz){const ringSp=new THREE.Sprite(new THREE.SpriteMaterial({map:ringBild,color:REL_FARBE[besitz]||REL_FARBE.neutral,transparent:true,depthWrite:false,opacity:.95}));ringSp.scale.setScalar(radius*2.75);mesh.add(ringSp);mesh.userData.besitzRing=ringSp;}
   const luft=zone&&window.PlanetKugel?.atmo(typ);if(luft)mesh.add(atmosphaere(radius*1.06,new THREE.Color(luft[0]/255,luft[1]/255,luft[2]/255)));
   if(isOwn||p.spieler&&(p.bekannt||p.besiedlung_bekannt))mesh.add(stadtLichter(radius*1.004,p.position*3.1+system*.7,isOwn?'#ffc27a':besitz==='feindlich'?'#ff9a6a':'#ffd9a0'));
   if(zone==='leben'||typ==='nebel_xeno'||typ==='frost_methan'){const w=wolken(radius*1.018,p.position*1.7+system*.13);mesh.add(w);sz.wolken.push(w);}
   if(p.rohstoffring){const r=rohstoffRing(radius,p.position+system*.1);r.quaternion.copy(mesh.userData.kipp).multiply(new THREE.Quaternion().setFromAxisAngle(new THREE.Vector3(1,0,0),-Math.PI/2));group.add(r);sz.ringe.push({m:r,planet:mesh});}
   const owner=p.spieler&&(p.bekannt||p.besiedlung_bekannt)?ownerMarks.get(p.spieler):null,orbit=ring(bahnRadius(p.position),owner?.color||'#6f93b8',profile().plaetze_je_system>16?.05:.1);orbit.userData={owner:owner?p.spieler:'',position:p.position,baseOpacity:isOwn?.16:GalaxyModel.relation(p)==='verbuendet'?.13:profile().plaetze_je_system>16?.05:.1,accent:orbitAccent(bahnRadius(p.position),owner?.color||'#6f93b8')};orbit.material.opacity=orbit.userData.baseOpacity;orbits.push(orbit);
   label(p.koord,mesh,p.position,'planet',sector,GalaxyModel.summary([p],1));invasionsMarke(labels[labels.length-1]?.el,p);
   const tf=(view.truemmer||[]).find(t=>t.koord===p.koord);if(tf){truemmerFeld(mesh,p,radius,tf);/* gestrandete Flotte auch am Trümmerfeld zeigen (beide sichtbar) */const dl=labels[labels.length-1];if(dl?.kind==='debris')invasionsMarke(dl.el,{koord:p.koord,orbitflotten:p.orbitflotten});}
   if(p.mond?.typ){const m=p.mond,mr=mondRadius(m,radius),moon=new THREE.Mesh(new THREE.SphereGeometry(mr,48,32),new THREE.MeshStandardMaterial({map:surfaceTexture('moons',m.typ,m.volk,()=>planetTexture('unknown',p.position*7)),color:'#ffffff',roughness:.95}));moon.material.emissiveMap=moon.material.map;moon.material.emissive.set('#ffffff');moon.material.emissiveIntensity=.2;
    moon.userData={kind:'mond',key:p.position,data:p};group.add(moon);targets.push(moon);{const saum=new THREE.Sprite(new THREE.SpriteMaterial({map:saumBild,color:'#c4cfdd',transparent:true,depthWrite:false,blending:THREE.AdditiveBlending,opacity:.55}));saum.scale.setScalar(mr*2.5);moon.add(saum);}
    moons.push({mesh:moon,planet:mesh,bahn:mondBahn(radius,mr),phase:(window.PlanetKugel?.streuung(p.koord+'m')||0)%628/100,alter:Number(m.alter_sekunden)||0,dreh:0,seit:performance.now()/1000});if(m.sprungtor)torBauen(moons[moons.length-1],p);}
  }
  // Asteroidengürtel (Sicht) zwischen Platz 7 und 8; jedes System hat außerdem einen fernen Kuiperring aus Eis und Staub.
  const rock=['#8d8274','#7b736a','#a09381','#6e655c','#9a8a76','#5f5953'],eis=['#c9d6e2','#aebfcf','#d9e2ea','#8fa3b5'];
  if(data.asteroidenguertel){const r0=sz.band?sz.band[0]:bahnRadius(7)+12,r1=sz.band?sz.band[1]:bahnRadius(8)-12,n=handy.matches?2600:4200;
   const g=felsRing({n,r0,r1,h:6,groesse:[.18,1.9],farbe:rock,saat:system*7+sector,taumel:handy.matches?220:400,kind:'guertel',key:0});g.add(staubRing(handy.matches?5000:9000,r0-2,r1+2,4,'#c9a97f',system*13+5,.35,1.3));group.add(g);sz.guertel.push({g,v:.012});for(const m of g.children)if(m.isInstancedMesh)targets.push(m);}
  {const ra=sz.rand+14,g=felsRing({n:handy.matches?900:1600,r0:ra,r1:ra+34,h:14,groesse:[.15,1.2],farbe:eis,saat:system*3+sector*11,taumel:60,kind:'kuiper',key:0});g.add(staubRing(handy.matches?4000:7000,ra-4,ra+40,12,'#8fb0d0',system*17+1,.22,1.4));group.add(g);sz.guertel.push({g,v:.004});}
  if(data.dyson)dysonBauen(data.dyson,k);
  lochMarker(data);weltenkruemmerMarker();mondEreignisse();kugelnErfassen();
 }
 // Trümmerfeld (Sicht truemmer): taumelnde Wrackteile und Funken um den Planeten; antippen zeigt Rohstoffe und Bergung.
 function truemmerFeld(mesh,p,radius,tf){
  // „Die Brechung der Trümmerfelder muss stimmen“ (Karl, 19.10.): Mondtrümmer bilden einen schmalen Ring in der
  // Äquatorebene, Kampftrümmer eine dicke Wrackwolke auf einem Bogen der Bahn um den Planeten. Herkunft vom Kern (ring),
  // sonst aus den Mondereignissen; bleibt bei jeder Aktualisierung gleich.
  const ring=istMondring(tf,p.koord),menge=(tf.erz||0)+(tf.kristall||0),n=Math.round(Math.min(handy.matches?260:420,60+Math.sqrt(menge)*1.4)),farbe=['#8a9098','#6d737a','#b5b9be','#a0663a','#c4823f','#4f555c'];
  const g=felsRing(ring?{n,r0:radius*FELD_INNEN,r1:radius*FELD_AUSSEN,h:radius*.3,groesse:[.12,.75],farbe,saat:p.position*17+3,taumel:n,kind:'debris',key:p.position,metall:.75,form:'ring'}
   :{n,r0:radius*1.45,r1:radius*FELD_AUSSEN,h:radius*1.1,bogen:2.4,groesse:[.14,.9],farbe,saat:p.position*17+3,taumel:n,kind:'debris',key:p.position,metall:.75,form:'feld'});
  g.add(staubRing(260,radius*1.3,radius*2.2,ring?radius*.25:radius*.9,'#ffd38a',p.position*29,.55,1.1,ring?0:2.6));group.add(g);const eintrag={g,planet:mesh,dreh:p.position,ring};sys3?.felder.push(eintrag);window.Truemmer3D?.truemmerFeld?.({gruppe:g,planet:mesh,radius,daten:p,truemmer:{...tf,ring}});for(const m of g.children)if(m.isInstancedMesh)targets.push(m);
  const marker=new THREE.Object3D();marker.position.set(radius*2.3,radius*.6,0);mesh.add(marker);label('Trümmer · '+p.koord,marker,p.position,'debris');
  return eintrag;
 }
 // Trümmerring aus einem zerstörten Mond: schließt sich in 9 s als Bogen von der Mondposition aus, im Gleichschritt mit den
 // Brocken aus mondzerstoerung.js (ring); Felsen an der Bogenfront wachsen weich, Staub blendet mit ein.
 const bogenM=new THREE.Matrix4(),bogenS=new THREE.Matrix4(),bogenV=new THREE.Vector3(),bogenQ=new THREE.Quaternion();
 function bogenSchritt(f){
  const u=Math.min(1,(performance.now()/1000-f.wachsen)/9),spr=u*u*(3-2*u),taumel=f.g.userData.taumel||[];
  if(!f.bogen){f.bogen=[];f.punkte=[];
   f.g.traverse(o=>{if(o.isInstancedMesh){const a=new Float32Array(o.count),b=[];for(let i=0;i<o.count;i++){o.getMatrixAt(i,bogenM);b.push(bogenM.clone());bogenV.setFromMatrixPosition(bogenM);a[i]=Math.atan2(bogenV.z,bogenV.x);}f.bogen.push({m:o,a,b});}
    else if(o.isPoints)f.punkte.push({o,d:o.material.opacity});});
   for(const d of taumel)d.sk0=d.sk;}
  let a0=0;if(f.mond){bogenV.copy(f.mond.position).sub(f.planet.position).applyQuaternion(bogenQ.copy(f.g.quaternion).invert());a0=Math.atan2(bogenV.z,bogenV.x);}
  const grenze=Math.PI*1.04*spr,anteil=a=>{let d=Math.abs(a-a0)%(Math.PI*2);if(d>Math.PI)d=Math.PI*2-d;return Math.max(0,Math.min(1,(grenze-d)/.35));};
  if(u<1){for(const x of f.bogen){for(let i=0;i<x.a.length;i++){const s=anteil(x.a[i]);x.m.setMatrixAt(i,s>=1?x.b[i]:bogenM.multiplyMatrices(x.b[i],bogenS.makeScale(s,s,s)));}x.m.instanceMatrix.needsUpdate=true;}
   for(const d of taumel)d.sk=d.sk0*anteil(Math.atan2(d.pos.z,d.pos.x));
   for(const p of f.punkte)p.o.material.opacity=p.d*spr;return;}
  for(const x of f.bogen){for(let i=0;i<x.a.length;i++)x.m.setMatrixAt(i,x.b[i]);x.m.instanceMatrix.needsUpdate=true;}
  for(const d of taumel)d.sk=d.sk0;for(const p of f.punkte)p.o.material.opacity=p.d;f.bogen=f.punkte=null;f.wachsen=null;
 }
 function sz3Feld(g){sys3?.felder.push(g);}
 // Der Kern meldet `ring: true` an Feldern aus einer Mondzerstörung (fehlt sonst). Welten mit Mondgeneration kennen das Feld,
 // dort gilt: ohne `ring` ist es ein Kampffeld. Ältere Server ohne die Angabe: aus den Mondereignissen schließen.
 function istMondring(tf,koord){if(tf&&tf.ring!=null)return !!tf.ring;if(tf&&!tf.koord)return true;/* frisch aus der Zerstörung */
  if(view?.monde?.regeln?.zerstoerung_mod)return false;
  return (view?.ereignisse||[]).some(e=>(e.art==='mond_zerstoert'||e.art==='mond_truemmerring')&&String(e.text||'').includes(koord));}
 // ---- Dysonbau (Galaxie-Werkzeug systeme[].dyson): Gerüst, Paneele vom Äquator zu den Polen, Kräne an der Baufront,
 // Kollektorschwarm; vollendet mit Energiestrahlen zu den angeschlossenen Planeten.
 function dysonBauen(x,k){
  const R=k.r*1.75,N=handy.matches?900:1600,f=x.phase==='bau'?Math.max(0,Math.min(1,(x.fortschritt_prozent||0)/100)):x.phase==='vollendet'||x.phase==='verwaist'?1:0,gr=new THREE.Group();gr.userData={kind:'dyson',key:0};
  const geruest=new THREE.LineSegments(new THREE.EdgesGeometry(new THREE.IcosahedronGeometry(R*.985,4)),new THREE.LineBasicMaterial({color:0x9fb2c8,transparent:true,opacity:.45*(1-f)+.08,blending:THREE.AdditiveBlending}));gr.add(geruest);
  const punkte=[],gold=Math.PI*(3-Math.sqrt(5));for(let i=0;i<N;i++){const y=1-(i+.5)/N*2,rr=Math.sqrt(1-y*y),a=i*gold;punkte.push(new THREE.Vector3(Math.cos(a)*rr,y,Math.sin(a)*rr));}
  punkte.sort((p,q)=>Math.floor(Math.abs(p.y)*16)-Math.floor(Math.abs(q.y)*16)||Math.atan2(p.z,p.x)-Math.atan2(q.z,q.x));
  const kante=Math.sqrt(4*Math.PI*R*R/N/2.598)*.92,hex=new THREE.CylinderGeometry(kante,kante,kante*.12,6);hex.rotateX(Math.PI/2);
  const mat=new THREE.MeshStandardMaterial({color:0xd9a441,metalness:.9,roughness:.28,emissive:0xff9a2a,emissiveIntensity:f>=1?.9:.35});
  const n=Math.round(f*N),paneele=new THREE.InstancedMesh(hex,mat,N),d=new THREE.Object3D();for(let i=0;i<N;i++){d.position.copy(punkte[i]).multiplyScalar(R);d.lookAt(0,0,0);d.scale.setScalar(i<n?1:0);d.updateMatrix();paneele.setMatrixAt(i,d.matrix);}paneele.userData={kind:'dyson',key:0};gr.add(paneele);targets.push(paneele);
  const front=n<N?Math.abs(punkte[Math.max(0,n-1)].y):1,kraene=[];
  if(f>0&&f<1)for(let i=0;i<10;i++){const kr=new THREE.Group();kr.add(new THREE.Line(new THREE.BufferGeometry().setFromPoints([new THREE.Vector3(),new THREE.Vector3(0,0,R*.16)]),new THREE.LineBasicMaterial({color:0xd8e4f2})));const fu=sprite('#ffe9a8',R*.09,1);fu.position.z=R*.16;kr.add(fu);kr.userData={phase:i/10*Math.PI*2,nord:i%2===0,fu};gr.add(kr);kraene.push(kr);}
  const NS=handy.matches?2500:5000,pos=new Float32Array(NS*3),attr=new Float32Array(NS*4);let zz=12345;const zu=()=>((zz=Math.imul(zz^zz>>>15,2246822507)+1013904223>>>0)/4294967296);
  for(let i=0;i<NS;i++){attr.set([R*(1.18+(i%6)*.06)+zu()*R*.1,zu()*6.28,(.05+zu()*.05),(zu()-.5)*1.4],i*4);pos[i*3+2]=zu()*6.28;}
  const sg=new THREE.BufferGeometry();sg.setAttribute('position',new THREE.BufferAttribute(pos,3));sg.setAttribute('bahn',new THREE.BufferAttribute(attr,4));
  const sm=new THREE.ShaderMaterial({transparent:true,depthWrite:false,blending:THREE.AdditiveBlending,uniforms:{zeit:zeitU,anteil:{value:.15+.85*f},px:{value:renderer.getPixelRatio()},farbe:{value:new THREE.Color(0xffd37a)}},
   vertexShader:'attribute vec4 bahn;uniform float zeit,anteil,px;varying float sicht;void main(){float r=bahn.x,w=bahn.y+zeit*bahn.z,i=bahn.w,k=position.z;vec3 p=vec3(cos(w)*r,0.,sin(w)*r);p=vec3(p.x,p.z*sin(i),p.z*cos(i));p=vec3(p.x*cos(k)-p.z*sin(k),p.y,p.x*sin(k)+p.z*cos(k));sicht=step(fract(bahn.y*7.31),anteil);vec4 mv=modelViewMatrix*vec4(p,1.);gl_Position=projectionMatrix*mv;gl_PointSize=sicht*px*clamp(160./-mv.z,1.,3.5);}',
   fragmentShader:'uniform vec3 farbe;varying float sicht;void main(){if(sicht<.5)discard;vec2 c=gl_PointCoord-.5;float a=smoothstep(.5,.1,length(c));gl_FragColor=vec4(farbe*1.6,a);}'});
  gr.add(new THREE.Points(sg,sm));
  const strahlen=[];if(f>=1)for(const ko of x.anschluesse||[]){const pos_=Number(String(ko).split(':')[2]),pl=planets.find(m=>Number(m.userData.data.position)===pos_);if(!pl)continue;const l=new THREE.Line(new THREE.BufferGeometry().setFromPoints([new THREE.Vector3(),new THREE.Vector3()]),new THREE.LineBasicMaterial({color:0xffd27a,transparent:true,opacity:.8,blending:THREE.AdditiveBlending}));gr.add(l);strahlen.push({l,pl});}
  group.add(gr);sys3.dyson={gr,kraene,front,R,f,x,strahlen,mat};
 }
 function dysonSchritt(t){const d=sys3?.dyson;if(!d)return;d.gr.rotation.y=-t*.01;
  for(const kr of d.kraene){const y=(kr.userData.nord?1:-1)*Math.min(.97,d.front+.03),rr=Math.sqrt(1-y*y),a=kr.userData.phase+t*.12;kr.position.set(Math.cos(a)*rr*d.R*1.03,y*d.R*1.03,Math.sin(a)*rr*d.R*1.03);kr.lookAt(0,0,0);kr.rotateY(Math.PI);kr.userData.fu.material.opacity=.4+.6*Math.abs(Math.sin(t*9+kr.userData.phase*3));}
  for(const s of d.strahlen){const a=s.l.geometry.attributes.position,p=s.pl.position;const dir=p.clone().normalize().multiplyScalar(d.R);d.gr.worldToLocal(dir.add(d.gr.position));const ziel=d.gr.worldToLocal(p.clone());a.setXYZ(0,dir.x,dir.y,dir.z);a.setXYZ(1,ziel.x,ziel.y,ziel.z);a.needsUpdate=true;s.l.material.opacity=.45+.4*Math.sin(t*3+p.x);}
  d.mat.emissiveIntensity=(d.f>=1?.9:.35)+.08*Math.sin(t*1.7);}
 // Flotten erscheinen im Sonnensystem nicht (Karl, 19.10.: nur Weltenkrümmer sind groß genug, um gesehen zu werden);
 // eigene Flüge und Angriffe stehen im Planetenfeld.
 // ---- Je Bild im Sonnensystem: Shaderzeit, Korona zur Kamera, Wolken, Gürtel, Trümmer, Dyson, Flotten.
 const feldQ2=new THREE.Quaternion();
 function systemSchritt(dt,t){if(!sys3)return;window.Truemmer3D?.schritt?.(dt,t);window.Truemmer3D?.detail?.(camera.position);sys3.korona?.quaternion.copy(camera.quaternion);for(const m of planets){const r=m.userData.besitzRing;if(r){const d=camera.position.distanceTo(m.position)/(m.userData.radius||8);r.material.opacity=Math.max(0,Math.min(.95,(d-7)/12));}}for(const w of sys3.wolken)w.rotation.y-=dt*.012;
  for(const g of sys3.guertel){g.g.rotation.y-=dt*g.v;taumeln(g.g,dt);}
  for(const x of sys3.ringe)x.m.position.copy(x.planet.position);
  for(const f of sys3.felder){if(f.wachsen!=null)bogenSchritt(f);f.dreh+=dt*.06;f.g.position.copy(f.planet.position);feldQ2.setFromAxisAngle(OBEN,-f.dreh);f.g.quaternion.multiplyQuaternions(f.planet.userData.kipp,feldQ2);taumeln(f.g,dt);}dysonSchritt(t);}
 // ---- Meldungen im Sonnensystem: dezentes Band unter dem Pfad, verschwindet nach 8 s (Mond zerstört u. a.).
 const gezeigteMeldungen=new Set();
 function systemMeldung(titel,unter,schluessel){if(!overlayEl)return;if(schluessel){if(gezeigteMeldungen.has(schluessel))return;gezeigteMeldungen.add(schluessel);}let el=overlayEl.querySelector('.g3-meldung');if(!el){el=document.createElement('div');el.className='g3-meldung';el.setAttribute('role','status');overlayEl.append(el);}
  el.innerHTML=`<strong>${esc(titel)}</strong>${unter?`<small>${esc(unter)}</small>`:''}`;el.classList.remove('weg');el.hidden=false;clearTimeout(el._t);el._t=setTimeout(()=>{el.classList.add('weg');el._t=setTimeout(()=>{el.hidden=true;},600);},8000);}
 // Zerstörte Monde aus den Ereignissen (Kern meldet sie Angreifer und Mondbesitzer, auch abwesend): Marke am Planeten + einmal Meldung.
 function mondEreignisse(){for(const e of view?.ereignisse||[]){if(e.art!=='mond_zerstoert')continue;const k=/Mond (\d+:\d+:\d+)/.exec(e.text||'')?.[1];if(!k||!k.startsWith(sector+':'+system+':'))continue;const pos=Number(k.split(':')[2]),pl=planets.find(m=>Number(m.userData.data.position)===pos);if(!pl)continue;
  const marke=new THREE.Object3D();marke.position.set(0,(pl.userData.radius||8)*1.6,0);pl.add(marke);label('Mond zerstört · '+e.zeit,marke,'mz'+pos,'notiz');const lab=labels[labels.length-1];lab.el.querySelector('.space-number').textContent='☾✕ Mond zerstört · '+e.zeit;lab.el.title=e.text;
  systemMeldung('☾ Mond an ['+k+'] zerstört',e.zeit,e._postfach_id||e.text);}}
 // ---- Info-Felder für Mond, Trümmer, Sonne, Gürtel, Dyson, Flotte, Weltenkrümmer (Planet: panelZeigen).
 function freiHalten(o){if(!o||!camera||!overlayEl||folge||fliegt||!handy.matches)return;/* nur Handy: Feld unten */const panel=overlayEl.querySelector('.g3-panel');if(panel.hidden)return;const c=canvas.getBoundingClientRect(),pr=panel.getBoundingClientRect(),v=o.getWorldPosition(new THREE.Vector3()).project(camera),y=c.top+(1-v.y)*c.height/2,grenze=pr.top-40;if(y<grenze||pr.top<=c.top)return;
  const hoehe=2*distance*Math.tan(THREE.MathUtils.degToRad(camera.fov/2)),up=new THREE.Vector3().setFromMatrixColumn(camera.matrix,1),oben=c.top+60,schub=(y-(oben+Math.max(0,grenze-oben)*.5))/c.height*hoehe;autoFit=null;folge=null;zielKam.x=focus.x-up.x*schub;zielKam.y=focus.y-up.y*schub;zielKam.z=focus.z-up.z*schub;fliegt=true;neu();}
 function hoverText(d){if(!d)return '';if(d.kind==='planet'){const p=d.data||{},k=p.koord||'',sp=own(k)?view.name:(p.bekannt||p.besiedlung_bekannt)?p.spieler:'',z=p.zone||p.habitabilitaet?.zone;return `<b>[${esc(k)}]</b> ${sp?esc(sp)+(p.beziehung?.allianz_tag?' ['+esc(p.beziehung.allianz_tag)+']':''):p.bekannt?'unbewohnt':'unerforscht'}${z?' · '+esc(ZONEN[z]||z):''}${p.mond?' · ☾ '+esc(p.mond.name||'Mond'):''}`;}
  if(d.kind==='mond'){const m=d.data?.mond||{};return `<b>☾ ${esc(m.name||'Mond')}</b> · ${esc(window.Mondseite?.typName?Mondseite.typName(m.typ):m.typ||'')}`;}if(d.kind==='debris'){const t=(view.truemmer||[]).find(x=>x.koord===`${sector}:${system}:${d.key}`);return t?`<b>✧ Trümmer</b> · ${fmt(t.erz||0)} Erz · ${fmt(t.kristall||0)} Kristall`:'';}
  if(d.kind==='sonne')return `<b>☀ ${esc(KLASSEN_NAME[sys3?.klasse.n]||'Stern')}</b> · ${sector}:${system}`;if(d.kind==='guertel')return '<b>☄ Asteroidengürtel</b>';if(d.kind==='dyson')return '<b>◍ Dysonsphäre</b>';if(d.kind==='wk')return '<b>◉ Weltenkrümmer</b>';return '';}
 function hoverZeigen(hit,pos){if(!overlayEl||handy.matches)return;let el=overlayEl.querySelector('.g3-hover');if(!el){el=document.createElement('div');el.className='g3-hover';overlayEl.append(el);}const t=hoverText(hit?.object.userData);if(!t){el.hidden=true;return;}el.innerHTML=t;el.hidden=false;const r=host.getBoundingClientRect();el.style.transform=`translate3d(${Math.round(pos.clientX-r.left+14)}px,${Math.round(pos.clientY-r.top+14)}px,0)`;}
 function feldText(titel,kopf,unter,zeilen,aktionen,bild){
  const panel=overlayEl?.querySelector('.g3-panel');if(!panel)return;listeZu();
  panel.innerHTML=`<header>${bild?`<img src="${esc(bild)}" alt="" class="g3-kugel">`:`<span class="g3-kugel g3-leer" aria-hidden="true">${kopf.symbol||'✦'}</span>`}<div><span class="eyebrow">${esc(kopf.eyebrow)}</span><h3>${esc(titel)}</h3><p>${esc(unter)}</p></div><button type="button" class="g3-x" data-g3-zu aria-label="Feld schließen">✕</button></header><dl>${zeilen.filter(Boolean).map(([a,b,roh])=>`<dt>${esc(a)}</dt><dd>${roh?b:esc(b)}</dd>`).join('')}</dl><div class="g3-aktionen">${aktionen.filter(Boolean).join('')}</div>`;
  panel.hidden=false;sichtbarMachen();neu();
 }
 function mondFeld(p){offenesFeld={kind:'mond',key:p.position};const m=p.mond;if(!m)return;const eigen=own(p.koord)||m.eigen,k=esc(p.koord),em=(view?.monde?.eigene||[]).find(x=>x.koord===p.koord||x.planet===p.koord);
  feldText(m.name||'Mond',{eyebrow:'MOND · ['+p.koord+']',symbol:'☾'},eigen?'Dein Mond':m.besitzer?'Mond von '+m.besitzer:'Mond',
   [['Typ',window.Mondseite?.typName?Mondseite.typName(m.typ):m.typ],['Durchmesser',fmt(m.durchmesser||m.groesse||0)+' km'],['Planet','['+p.koord+'] '+(p.spieler||'')],m.besitzer&&!eigen?['Besitzer',m.besitzer]:null,em?.tiefenphalanx?.stufe_wirksam?['Tiefenphalanx','Stufe '+em.tiefenphalanx.stufe_wirksam]:null,m.bedroht?['Lage','⚠ feindlicher Anflug']:null],
   [vernichtenChip(p),eigen?knopf(`data-mond-oeffnen="${k}"`,'Mond öffnen',true):knopf(`data-gt-mondziel="${k}"`,'Flug zum Mond planen',!eigeneWeltenkruemmer().length),!eigen?knopf(`data-gt-spion="${k}"`,'Planet spionieren'):'',knopf('data-g3-hinfliegen="mond:'+p.position+'"','Hinfliegen')],
   window.Mondseite?.bild?null:null);}
 // ---- Eignung für dein Volk (Karl, 19.10.: „man sieht bei erkundeten Planeten nicht richtig, wie sehr er für mich und mein
 // Volk geeignet ist – Bevölkerungsreproduktion, Solarwert …“): nur Kernwerte aus habitabilitaet (klima_sicht) – Klima in % des
 // Optimums, Vermehrung je Tag gegen das Mögliche, Sonne in % zu Platz 6, Dysonwirkung (dyson_promille vom Kern, sonst der
 // heutige Eigner-Bonus der vollendeten Sphäre).
 const pz1=n=>(Number(n)||0).toLocaleString('de-DE',{maximumFractionDigits:1});
 function eignung(p){
  const h=p?.habitabilitaet,e=(h?.herkunft||[]).find(x=>x.volk===view?.volk);if(!h||!e)return null;
  const opt=Number(h.optimum_promille)||1000,klima=Math.max(0,Math.min(100,Math.round((Number(e.wachstum_promille)||0)/opt*100)));
  const tag=Number.isFinite(e.wachstum_je_tag_promille)?e.wachstum_je_tag_promille/10:null,maxTag=Number.isFinite(h.grundwert_je_tag_promille)?h.grundwert_je_tag_promille*(Number(e.volksfaktor)||1)/10:null;
  const sonne=Number.isFinite(h.solar_promille)?h.solar_promille/10:null,d=systemDaten?.dyson;
  const dyson=Number.isFinite(h.dyson_promille)?h.dyson_promille/1000:d&&d.phase==='vollendet'&&d.eigen?1.6:1,wirksam=Number.isFinite(h.solar_wirksam_promille)?h.solar_wirksam_promille/10:sonne==null?null:sonne*dyson;
  const status=e.bewohnbar?'bewohnbar':e.terraforming_noetig?`Terraforming ${e.terraforming_noetig} nötig (du: ${e.terraforming||0})`:'unbewohnbar';
  return {klima,status,bewohnbar:!!e.bewohnbar,verlust:Number(e.klimaverlust_promille_je_stunde)||0,tag,maxTag,sonne,dyson,wirksam};
 }
 const stufe=v=>v>=80?'gut':v>=50?'mittel':v>0?'schwach':'null';
 function balken(wert,stufeName){const w=Math.max(0,Math.min(100,wert));return `<svg class="g3-eig-balken" viewBox="0 0 100 6" preserveAspectRatio="none" aria-hidden="true"><rect class="bahn" width="100" height="6" rx="3"/><rect class="wert ${stufeName}" width="${w.toFixed(1)}" height="6" rx="3"/></svg>`;}
 function eignungBlock(p){const g=eignung(p);if(!g)return '';
  const tagText=g.tag==null?'–':(g.tag>0?'+':'')+pz1(g.tag)+' %/Tag',tagW=g.tag!=null&&g.maxTag?g.tag/g.maxTag*100:0;
  const dysonText=g.dyson>1?` · Dyson ×${pz1(g.dyson)}`:g.dyson<1?` · Dysonschatten ×${pz1(g.dyson)}`:'';
  return `<span class="g3-eignung">`
   +`<span class="g3-eig"><span class="g3-eig-name">Klima</span>${balken(g.klima,stufe(g.klima))}<b>${g.klima} %</b><small class="${g.bewohnbar?'':'warn'}">${esc(g.status)}${!g.bewohnbar&&g.verlust?` · −${pz1(g.verlust/10)} %/h Bewohner`:''}</small></span>`
   +`<span class="g3-eig"><span class="g3-eig-name">Vermehrung</span>${balken(tagW,stufe(tagW))}<b>${esc(tagText)}</b><small>${g.maxTag?'von höchstens '+pz1(g.maxTag)+' %/Tag für dein Volk':''}</small></span>`
   +(g.sonne==null?'':`<span class="g3-eig"><span class="g3-eig-name">Sonne</span>${balken(g.wirksam/1.2,g.wirksam>=90?'gut':g.wirksam>=60?'mittel':'schwach')}<b>${pz1(g.wirksam)} %</b><small>Platz 6 = 100 %${g.dyson!==1?` · Lage ${pz1(g.sonne)} %${esc(dysonText)}`:''} – Solarkraftwerke liefern entsprechend</small></span>`)
   +`</span>`;}
 function eignungKurz(p){const g=eignung(p);if(!g)return '';return `<span class="gt-eignung gt-eig-${stufe(g.klima)}" title="Für dein Volk: Klima ${g.klima} % (${esc(g.status)}) · Vermehrung ${g.tag==null?'–':Math.round(g.tag)+' %/Tag'} · Sonne ${g.wirksam==null?'–':Math.round(g.wirksam)+' %'} (Platz 6 = 100 %)">Klima ${g.klima} %${g.wirksam==null?'':` · Sonne ${Math.round(g.wirksam)} %`}</span>`;}
 // ---- Mond vernichten (Karl, 19.10.: „es fehlt die Funktion, dass mein Weltenkrümmer den Mond vernichten kann … durchdachte,
 // schöne UI-Lösung … erst wenn man einen Weltenkrümmer hat … keine Fakes mehr“): Die Aktion erscheint nur mit eigenen
 // Weltenkrümmern und nur an fremden, nicht verbündeten Monden. Der Ring zeigt die echte Chance nach der Kernregel
 // (monde.rs zerstoerung_promille): d_eff = d·(1+Typmod), Chance (100−√d_eff)·√n %, Verlustrisiko √d_eff/2 %.
 // Ein Tipp öffnet den Flugassistenten: Start mit den meisten Weltenkrümmern (Planet oder eigener Mond), alle dortigen
 // Weltenkrümmer, Ziel, Zielart Mond, Auftrag mond_zerstoeren.
 const MOND_RISS_SVG='<svg viewBox="0 0 24 24" width="18" height="18" fill="none" stroke="currentColor" stroke-width="1.9" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><circle cx="12" cy="12" r="8.6"/><path d="M12.6 3.5 10.7 8.3l3 2.7-2.3 3.6 1.7 2.4-1.2 3.5"/><path d="m13.7 11 3.6-.9"/><path d="m11.4 14.6-3.2 1.4"/></svg>';
 function eigeneWeltenkruemmer(){const orte=[];for(const p of view?.planeten||[]){const n=p.schiffe?.weltentruemmer||0;if(n>0)orte.push({start:p.koord,startart:'planet',n});}
  for(const m of view?.monde?.eigene||[]){const n=m.schiffe?.weltentruemmer||0;if(n>0)orte.push({start:m.koord,startart:'mond',n});}
  return orte.sort((a,b)=>b.n-a.n);}
 function mondChance(m,n){const mods=view?.monde?.regeln?.zerstoerung_mod,d=(Number(m.durchmesser||m.groesse)||0)*(1+(mods?Number(mods[m.typ])||0:0)),w=Math.sqrt(d);
  return {prozent:Math.max(0,Math.min(100,(100-w)*Math.sqrt(n))),verlust:Math.max(0,Math.min(100,w/2)),genau:!!mods};}
 function chanceRing(pz){const u=97.39;return `<svg class="g3-chance-ring" viewBox="0 0 36 36" aria-hidden="true"><circle class="bahn" cx="18" cy="18" r="15.5"/><circle class="wert" cx="18" cy="18" r="15.5" stroke-dasharray="${(pz/100*u).toFixed(1)} ${u}" transform="rotate(-90 18 18)"/><text x="18" y="21.6">${pz}%</text></svg>`;}
 function vernichtenChip(p,opt={}){
  const m=p?.mond;if(!m||m.eigen||own(p.koord)||(p.spieler&&GalaxyModel.relation(p)==='verbuendet'))return '';
  const o=eigeneWeltenkruemmer()[0];if(!o)return '';
  const c=mondChance(m,o.n),pz=Math.round(c.prozent),ca=c.genau?'':'≈ ',klein=opt.klein?' klein':'';
  return `<button type="button" class="g3-vernichten${klein}" data-g3-vernichten="${esc(p.koord)}" title="Mond ${esc(m.name||'')} [${esc(p.koord)}] vernichten – ${o.n} Weltenkrümmer ab ${o.startart==='mond'?'Mond ':''}[${esc(o.start)}] · Zerstörungschance ${ca}${pz} % · Verlustrisiko ${ca}${Math.round(c.verlust)} %"><span class="g3-vernichten-symbol">${MOND_RISS_SVG}</span><span class="g3-abbau-text"><b>Mond vernichten</b> <small>· ${o.n===1?'1 Weltenkrümmer':o.n+' Weltenkrümmer'}</small></span>${chanceRing(pz)}</button>`;
 }
 function mondVernichten(koord){const o=eigeneWeltenkruemmer()[0];if(!o)return;
  if(window.Flottenfuehrung?.ausHangar)Flottenfuehrung.ausHangar({start:o.start,startart:o.startart,schiffe:{weltentruemmer:o.n},ziel:koord,zielart:'mond',mission:'mond_zerstoeren'});
  else if(typeof window.pickFleet==='function')window.pickFleet(koord,'mond_zerstoeren');}
 document.addEventListener('click',e=>{const b=e.target.closest('[data-g3-vernichten]');if(!b)return;e.preventDefault();mondVernichten(b.dataset.g3Vernichten);});
 // ---- Abbauen (Karl, 19.10.: „das Abbauen muss super einfach gehen – kleine Button mit Aktion ‚Abbauen‘, stilvoll“): Chip mit
 // Menge (und Überlebenden); ein Tipp öffnet die Bergung mit vorbelegten Recyclern/Bergungsschiffen und eingetragenem Ziel.
 // Ohne Recycler und Bergungsschiffe: „Recycler fehlen – bauen ›“ führt in die Werft.
 const ABBAU_SVG='<svg viewBox="0 0 24 24" width="16" height="16" fill="none" stroke="currentColor" stroke-width="2.1" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M13.6 7.6 3.6 17.6a1.7 1.7 0 0 0 2.4 2.4l10-10"/><path d="M9.2 4.6c3.8-1.6 8-.9 11 1.8.4.4 0 1-.5.9-2.9-.8-6-.4-8.6 1.2"/><path d="m12.7 6.7 4.6 4.6"/></svg>';
 const kurz=n=>{n=Number(n)||0;return n>=1e6?(n/1e6).toFixed(n>=1e7?0:1).replace('.',',')+' Mio':n>=1e3?Math.round(n/1e3)+' k':String(Math.round(n));};
 const bergeSchiffe=()=>(view?.planeten||[]).reduce((s,p)=>s+(p.schiffe?.recycler||0)+(p.schiffe?.bergungsschiff||0),0);
 function abbauChip(koord,tf,opt={}){
  const k=esc(koord),klein=opt.klein?' klein':'';
  if(!bergeSchiffe())return `<button type="button" class="g3-abbau fehlt${klein}" data-bs-springe="ships:recycler" title="Ohne Recycler oder Bergungsschiffe kein Abbau – in der Werft bauen"><span class="g3-abbau-symbol">${ABBAU_SVG}</span><span class="g3-abbau-text"><b>Recycler fehlen</b> <small>– bauen ›</small></span></button>`;
  const leute=Number(tf?.ueberlebende)||0,menge=!tf?'Menge wird erfasst':opt.klein?`${kurz(tf.erz)} · ${kurz(tf.kristall)}`:`${kurz(tf.erz)} Erz · ${kurz(tf.kristall)} Kristall`;
  return `<button type="button" class="g3-abbau${klein}" data-g3-abbau="${k}" title="Trümmerfeld [${k}] abbauen${tf?`: ${fmt(tf.erz)} Erz, ${fmt(tf.kristall)} Kristall`:''} – Recycler und Bergungsschiffe vorbelegt"><span class="g3-abbau-symbol">${ABBAU_SVG}</span><span class="g3-abbau-text"><b>Abbauen</b> <small>· ${esc(menge)}</small>${leute?` <small class="g3-abbau-leute">· +${fmt(leute)} Überlebende</small>`:''}</span></button>`;
 }
 document.addEventListener('click',e=>{const b=e.target.closest('[data-g3-abbau]');if(!b)return;e.preventDefault();const k=b.dataset.g3Abbau;openSalvage(k,{frisch:!(view?.truemmer||[]).some(t=>t.koord===k)});});
 function truemmerFeldInfo(koord){const t=(view.truemmer||[]).find(x=>x.koord===koord);
  // Frischer Ring aus einer Mondzerstörung: Der Kern hat das Feld schon angelegt, die Menge kommt mit der nächsten Aktualisierung.
  const frisch=!t&&sys3?.felder.some(f=>f.planet?.userData?.data?.koord===koord);if(!t&&!frisch)return;
  const k=esc(koord),rec=view.planeten.reduce((n,p)=>n+(p.schiffe?.recycler||0),0),berg=view.planeten.reduce((n,p)=>n+(p.schiffe?.bergungsschiff||0),0);
  const ring=istMondring(t,koord),leute=Number(t?.ueberlebende)||0;
  feldText(ring?'Trümmerring':'Trümmerfeld',{eyebrow:'ORBIT · ['+koord+']',symbol:'✧'},ring?'Aus dem zerstörten Mond – Recycler und Bergungsschiffe bauen ihn ab':'Wracks und Trümmer aus Kämpfen im Orbit',[...(t?[['Erz',fmt(t.erz||0)],['Kristall',fmt(t.kristall||0)]]:[['Rohstoffe','werden gerade erfasst']]),leute?['Überlebende',fmt(leute)+' warten auf Bergung (Bergungsschiff)']:null,['Deine Bergungsschiffe',fmt(rec)+' Recycler · '+fmt(berg)+' Bergungsschiffe']],
   [abbauChip(koord,t),knopf('data-g3-hinfliegen="debris:'+koord.split(':')[2]+'"','Hinfliegen')]);}
 function sonneFeld(){const k=sys3?.klasse||sternklasse(),d=systemDaten;
  feldText('Stern '+sector+':'+system,{eyebrow:'SONNE · '+k.n+'-KLASSE',symbol:'☀'},KLASSEN_NAME[k.n]||'Stern',[['Planetenplätze',String(profile().plaetze_je_system)],['Erkundet',d?.bekannt?'ja':'noch nicht'],['Nebelsystem',d?.nebel===true?'ja – Xenokristall auf geraden Plätzen (kleinere Welten), mehr Amethystmonde':d?.nebel===false?'nein':'unbekannt – System erkunden'],d?.asteroidenguertel?['Asteroidengürtel','vorhanden (Position 0)']:null,d?.dyson?['Dysonsphäre',(d.dyson.allianz_tag?'['+d.dyson.allianz_tag+'] ':'')+(d.dyson.phase==='bau'?'im Bau '+(d.dyson.fortschritt_prozent||0)+' %':d.dyson.phase)]:null],
   [!d?.bekannt?knopf(`data-scan-system="${sector}:${system}:1"`,'Ganzes System erkunden',true):'',d?.dyson?knopf(`data-dyson-sonne="${esc(d.dyson.system??system)}" data-fortschritt="${d.dyson.phase==='bau'?(d.dyson.fortschritt_prozent||0)/100:d.dyson.phase==='sammeln'?0:1}" data-anschluesse="${(d.dyson.anschluesse||[]).length}"`,'Dyson-Sonnenansicht'):''] );}
 function guertelFeld(){feldText('Asteroidengürtel',{eyebrow:'SYSTEM '+sector+':'+system,symbol:'☄'},'Erz- und Kristallbrocken zwischen Platz 7 und 8 – jedes System hat einen',[['Abbau','mit Bergbauschiffen (Mission abbau, Position 0)'],['Nicht verwechseln','Trümmerfelder an Planeten birgt man mit Recyclern oder Bergungsschiffen (Mission recyceln)']],[knopf(`data-work-site="${sector}:${system}:0" data-work-mission="abbau"`,'Abbau planen',true)]);}
 function dysonFeld(){const x=systemDaten?.dyson;if(!x)return;const f=x.phase==='bau'?(x.fortschritt_prozent||0):x.phase==='sammeln'?0:100;
  feldText('Dysonsphäre',{eyebrow:'GROSSPROJEKT · ['+(x.allianz_tag||'')+']',symbol:'◍'},x.phase==='bau'?'im Bau':x.phase,[['Fortschritt',f+' %'],['Allianz',x.allianz_tag||'—'],['Angeschlossen',String((x.anschluesse||[]).length)+' Planeten'],x.eigen&&x.rest_sekunden?['Fertig in',GameTime.duration(x.rest_sekunden)]:null,x.ruht?['Lage','ruht']:null],
   [knopf(`data-dyson-sonne="${esc(x.system??system)}" data-fortschritt="${f/100}" data-anschluesse="${(x.anschluesse||[]).length}"`,'Dyson-Sonnenansicht',true),x.eigen?knopf('data-go="grossprojekt"','Großprojekt öffnen'):'']);}
 function wkFeld(i){const w=weltenkruemmer[i];if(!w)return;const f=w.f,ziel=f?.ziel||w.koord;
  feldText('Weltenkrümmer',{eyebrow:w.art==='station'?'STATIONIERT · ['+w.koord+']':'IM EINSATZ',symbol:'◉'},w.art==='zerstoerung'?'Mondzerstörung an ['+ziel+']':w.art==='flug'?'Anflug auf ['+ziel+']':'Kreist über deiner Welt',
   [f?['Ankunft',GameTime.duration(Math.max(0,(f.ankunft_sekunden||0)-GameTime.now()))+' (Spielzeit)']:null,f?['Auftrag',window.Auftraege?.info?.(f.mission)?.name||f.mission]:null],
   [knopf('data-g3-folgen="wk:'+i+'"','Live verfolgen',true),knopf('data-go="flotten"','Flotten')]);}
 function init(){
  host=$g('galaxy-space');canvas=$g('galaxy-canvas');if(!host||renderer)return;
  $g('galaxy-detail-close').onclick=closeDetail;
  $g('galaxy-infocard').onclick=e=>{const card=e.currentTarget;if(e.target.closest('[data-explore],[data-colonize],[data-space-own],[data-planet-mission],[data-work-site]'))closeDetail();else if(e.target===card){const r=card.getBoundingClientRect();if(e.clientX<r.left||e.clientX>r.right||e.clientY<r.top||e.clientY>r.bottom)closeDetail();}};
  $g('galaxy-reset-camera').onclick=()=>{if(!renderer)return;yaw=.15;pitch=.8;fitView(stage==='system'?330:stage==='sector'?350:580);};
  $g('galaxy-motion').onclick=()=>{if(reduced.matches&&!motionOverride){motionOverride=true;paused=false;}else paused=!paused;motionState();neu();};
  if(window.PlanetKugel&&!$g('galaxy-drehtempo')){const regler=PlanetKugel.regler();regler.id='galaxy-drehtempo';regler.hidden=true;$g('galaxy-motion').after(regler);}
  $g('galaxy-reload').onclick=()=>{cache.clear();if(stage==='system')showSystem(sector,system);else if(stage==='sector')showSector(sector);else showGalaxy();};
  try{renderer=new THREE.WebGLRenderer({canvas,antialias:true,alpha:false,powerPreference:'low-power'});}catch(e){host.classList.add('space-fallback');$g('galaxy-reset-camera').disabled=true;$g('galaxy-fallback').hidden=false;$g('galaxy-fallback').textContent='3D ist in diesem Browser nicht verfügbar. Die Systemliste und Aufklärungsberichte bleiben nutzbar.';/* Handy ohne WebGL: gleich die Systemtabelle */if(handy.matches)setTimeout(()=>document.querySelector('#karte [data-gt-tabelle]')?.click(),0);return;}
  // Software-Rendering (SwiftShader/llvmpipe: Testbrowser, Geräte ohne GPU-Freigabe) schafft Nachleuchten bei voller Dichte
  // nicht – ein Bild dauert dort Sekunden, und das spätere Absenken blockierte den Hauptthread 16 s. Echte GPUs unverändert.
  {const gl=renderer.getContext(),dbg=gl.getExtension('WEBGL_debug_renderer_info'),gpu=String(dbg?gl.getParameter(dbg.UNMASKED_RENDERER_WEBGL):gl.getParameter(gl.RENDERER)||'');softwareRender=/swiftshader|llvmpipe|softpipe|basic render|software/i.test(gpu);}
  renderer.setPixelRatio(softwareRender?1:Math.min(devicePixelRatio,2));renderer.setClearColor(0x030813);renderer.outputColorSpace=THREE.SRGBColorSpace;renderer.toneMapping=THREE.ACESFilmicToneMapping;renderer.toneMappingExposure=1.15;
  scene=new THREE.Scene();camera=new THREE.PerspectiveCamera(48,1,.1,4000);raycaster=new THREE.Raycaster();glow=texture();himmelBauen();bloomBauen();
  // Startstufe der Qualitätsleiter; Tests können sie mit window.__g3Qualitaet={stufe,fest} vorgeben (fest: keine Anpassung).
  {const gl=renderer.getContext();gpuZeit.gl=gl;gpuZeit.ext=renderer.capabilities.isWebGL2?gl.getExtension('EXT_disjoint_timer_query_webgl2'):null;}
  {const v=window.__g3Qualitaet;q.fest=!!v?.fest;stufeSetzen(Number.isInteger(v?.stufe)?v.stufe:softwareRender?STUFEN.length-1:handy.matches?1:0);}/* Nachtseiten dunkel wie im All, aber lesbar: schwaches Umgebungslicht + Fülllicht von der Kamera */scene.add(new THREE.AmbientLight(0xc8d4e6,.55));{const fill=new THREE.DirectionalLight(0xb8c8ff,.42);camera.add(fill);scene.add(camera);}observer=new ResizeObserver(resize);observer.observe(host);
  // Außerhalb des Bildes (weggescrollt, andere Seite) wird nicht gezeichnet.
  new IntersectionObserver(e=>{imBild=e[e.length-1].isIntersecting;if(imBild)neu();else ruhe();}).observe(host);
  // Zoom zum Finger/Mauszeiger: der Punkt der Bahnebene unter dem Zeiger bleibt stehen.
  const zoom=(factor,px,py)=>{autoFit=null;const alt=distance;distance=Math.max(5,Math.min(2400,distance*factor));if(px!=null){const p=bodenPunkt(px,py);if(p){const k=1-distance/alt;focus.x+=(p.x-focus.x)*k;focus.z+=(p.z-focus.z)*k;if(Math.abs(k)>.002){folge=null;folgeKennung='';}}}cameraUpdate();};zoomen=f=>zoom(f);
  const pan=(dx,dy)=>{autoFit=null;folge=null;folgeKennung='';const scale=2*distance*Math.tan(THREE.MathUtils.degToRad(camera.fov/2))/Math.max(1,viewportHeight),right=new THREE.Vector3().setFromMatrixColumn(camera.matrix,0),up=new THREE.Vector3().setFromMatrixColumn(camera.matrix,1);focus.x+=(-right.x*dx+up.x*dy)*scale;focus.y+=(-right.y*dx+up.y*dy)*scale;focus.z+=(-right.z*dx+up.z*dy)*scale;cameraUpdate();};
  const measure=()=>{const a=[...pointers.values()];return a.length<2?null:{x:(a[0].x+a[1].x)/2,y:(a[0].y+a[1].y)/2,d:Math.hypot(a[0].x-a[1].x,a[0].y-a[1].y),w:Math.atan2(a[1].y-a[0].y,a[1].x-a[0].x)};};
  canvas.tabIndex=0;canvas.setAttribute('aria-label','Interaktive 3D-Karte: ein Finger dreht, zwei Finger zoomen, verschieben und drehen, Doppeltipp fliegt hin, Tippen zeigt Infos. Pfeiltasten drehen, Plus und Minus zoomen.');
  canvas.addEventListener('contextmenu',e=>e.preventDefault());
  const tipp={t:0,x:0,y:0};let letzteBewegung=0,tippTimer=0;
  host.addEventListener('pointerdown',e=>{if(e.button>2||e.target.closest('.space-camera-controls,.g3-overlay')||(!e.target.closest('.space-label')&&e.target!==canvas))return;const hitLabel=e.target.closest('.space-label');if(hitLabel&&e.button===0&&e.pointerType!=='touch')return;bedient();canvas.setPointerCapture(e.pointerId);schwung.yaw=schwung.pitch=0;pointers.set(e.pointerId,{x:e.clientX,y:e.clientY});if(pointers.size===1)drag={id:e.pointerId,x:e.clientX,y:e.clientY,startX:e.clientX,startY:e.clientY,start:performance.now(),moved:false,pan:e.button!==0||e.shiftKey,label:hitLabel};else if(drag)drag.moved=true;gesture=measure();});
  host.addEventListener('pointermove',e=>{
   if(!pointers.has(e.pointerId)){pendingHover={clientX:e.clientX,clientY:e.clientY};wake();return;}
   const old=pointers.get(e.pointerId);pointers.set(e.pointerId,{x:e.clientX,y:e.clientY});const next=measure();
   if(next&&gesture){if(gesture.d>2&&next.d>2)zoom(gesture.d/next.d,next.x,next.y);let dw=next.w-gesture.w;if(dw>Math.PI)dw-=2*Math.PI;if(dw<-Math.PI)dw+=2*Math.PI;if(Math.abs(dw)<.5){yaw-=dw;cameraUpdate();}const mx=next.x-gesture.x,my=next.y-gesture.y;if(Math.hypot(mx,my)>.6)pan(mx,my);gesture=next;return;}
   if(!drag)return;if(!drag.moved&&Math.hypot(e.clientX-drag.startX,e.clientY-drag.startY)<6)return;drag.moved=true;
   const dx=e.clientX-old.x,dy=e.clientY-old.y,jetzt=performance.now(),dtm=Math.max(8,jetzt-(letzteBewegung||jetzt-16))/1000;letzteBewegung=jetzt;
   if(drag.pan)pan(dx,dy);else{yaw-=dx*.0055;pitch=Math.max(-1.54,Math.min(1.54,pitch+dy*.0055));schwung.yaw=-dx*.0055/dtm*.6;schwung.pitch=dy*.0055/dtm*.6;cameraUpdate();}highlight(null);
  });
  const endPointer=e=>{const click=e.type==='pointerup'&&pointers.size===1&&drag&&!drag.moved;pointers.delete(e.pointerId);gesture=measure();
   if(drag&&drag.moved&&!pointers.size&&performance.now()-letzteBewegung>70)schwung.yaw=schwung.pitch=0;if(drag&&drag.moved&&!pointers.size)neu();
   if(click){const jetzt=performance.now(),doppel=jetzt-tipp.t<340&&Math.hypot(e.clientX-tipp.x,e.clientY-tipp.y)<34;Object.assign(tipp,{t:doppel?0:jetzt,x:e.clientX,y:e.clientY});
    clearTimeout(tippTimer);
    if(drag.label){const lab=drag.label;if(doppel){const l=labels.find(x=>x.el===lab);if(l)hinfliegen({object:l.obj.isMesh?l.obj:l.obj.parent||l.obj,point:l.obj.getWorldPosition(new THREE.Vector3())});lab.click();}else tippTimer=setTimeout(()=>lab.click(),250);}
    else{const hit=hitAt(e)||naechstes(e),px=e.clientX,py=e.clientY,auswahl=()=>{if(hit)pick(hit.object.userData.kind,hit.object.userData.key,hit.object.userData.sektor);};if(doppel){hinfliegen(hit,px,py);if(stage==='system')auswahl();}else tippTimer=setTimeout(auswahl,250);}}
   if(!pointers.size)drag=null;else if(drag)drag.moved=true;};
  canvas.addEventListener('pointerup',endPointer);canvas.addEventListener('pointercancel',endPointer);canvas.addEventListener('lostpointercapture',endPointer);
  canvas.addEventListener('pointerleave',()=>{pendingHover=null;if(!pointers.size)highlight(null);});
  host.addEventListener('wheel',e=>{if(e.target.closest('.g3-overlay'))return;e.preventDefault();zoom(Math.exp(e.deltaY*.0012),e.clientX,e.clientY);},{passive:false});
  canvas.addEventListener('keydown',e=>{if(['+','=','-','ArrowLeft','ArrowRight','ArrowUp','ArrowDown','Home'].includes(e.key))e.preventDefault();else return;if(e.key==='+'||e.key==='=')zoom(.85);else if(e.key==='-')zoom(1.15);else if(e.key==='Home')$g('galaxy-reset-camera').click();else{if(e.key==='ArrowLeft')yaw+=.1;if(e.key==='ArrowRight')yaw-=.1;if(e.key==='ArrowUp')pitch+=.1;if(e.key==='ArrowDown')pitch-=.1;pitch=Math.max(-1.54,Math.min(1.54,pitch));cameraUpdate();}});
  const controls=document.createElement('div');controls.className='space-camera-controls';controls.innerHTML='<button type="button" data-camera="in" aria-label="Karte vergrößern">＋</button><button type="button" data-camera="out" aria-label="Karte verkleinern">−</button><button type="button" data-camera="top">Oben</button><button type="button" data-camera="side">Seite</button><button type="button" data-camera="bottom">Unten</button>';host.append(controls);
  controls.addEventListener('click',e=>{const mode=e.target.dataset.camera;if(mode==='in')zoom(.8);else if(mode==='out')zoom(1.25);else if(mode){pitch=mode==='top'?1.5:mode==='bottom'?-1.5:0;cameraUpdate();}});
  const hint=host.querySelector('.space-instructions');if(hint)hint.textContent='1 Finger: drehen · 2 Finger: zoomen / verschieben · Maus: Rad zoomt, rechte Taste verschiebt';
  canvas.addEventListener('webglcontextlost',e=>{e.preventDefault();paused=true;$g('galaxy-fallback').hidden=false;$g('galaxy-fallback').textContent='3D-Grafik unterbrochen. Systemliste weiter nutzbar; Seite neu laden, um die Grafik wiederherzustellen.';});
  reduced.addEventListener('change',()=>{motionState();neu();});
  overlayBauen();pfadZeichnen();handy.addEventListener('change',()=>{handyLayout();resize();});addEventListener('resize',()=>{if(active)handyLayout();});resize();
 }
 function motionState(){if($g('galaxy-motion')){$g('galaxy-motion').textContent=reduced.matches&&!motionOverride?'Umläufe: aus':paused?'Umläufe: aus':'Umläufe: an';$g('galaxy-motion').title=reduced.matches&&!motionOverride?'Deine Systemeinstellung reduziert Bewegung. Du kannst Umläufe hier ausdrücklich aktivieren.':'';}}
 function levelNavigation(){
  let nav=$g('galaxy-levels');
  if(!nav){nav=document.createElement('nav');nav.id='galaxy-levels';nav.setAttribute('aria-label','Kartenebenen');$g('galaxy-title').closest('.space-header').after(nav);}
  const w=profile(),inGalaxy=stage==='galaxy',inSystem=stage==='system';
  const stamp=[stage,sector,system,w.sektoren,w.systeme_je_sektor].join(':');if(nav.dataset.stamp===stamp)return;nav.dataset.stamp=stamp;
  nav.innerHTML=`<ol class="space-level-path"><li><button type="button" data-level="galaxy" id="galaxy-home" ${inGalaxy?'aria-current="step"':''}>Galaxie</button></li><li><button type="button" data-level="sector" ${inGalaxy?'disabled':!inSystem?'aria-current="step"':''}>${inGalaxy?'Sektor':'Sektor '+sector}</button></li><li><button type="button" data-level="system" ${inSystem?'aria-current="step"':'disabled'}>${inSystem?'System '+sector+':'+system:'Sonnensystem'}</button></li></ol><div class="space-level-selectors"><label>Sektor<select id="galaxy-sector-select" aria-label="Sektor wählen">${inGalaxy?'<option value="">Sektor wählen …</option>':''}${Array.from({length:w.sektoren},(_,i)=>`<option value="${i+1}" ${!inGalaxy&&sector===i+1?'selected':''}>Sektor ${i+1} · ${w.systeme_je_sektor} Systeme</option>`).join('')}</select></label>${!inGalaxy?`<label>Sonnensystem<select id="galaxy-system-select" aria-label="Sonnensystem wählen"><option value="">System wählen …</option>${Array.from({length:w.systeme_je_sektor},(_,i)=>`<option value="${i+1}" ${inSystem&&system===i+1?'selected':''}>${sector}:${i+1}</option>`).join('')}</select></label>`:''}</div>`;
  nav.querySelector('[data-level="galaxy"]').onclick=()=>{if(stage!=='galaxy')showGalaxy();};
  nav.querySelector('[data-level="sector"]').onclick=()=>{if(stage!=='sector')void showSector(sector);};
  $g('galaxy-sector-select').onchange=e=>{if(e.target.value)void showSector(e.target.value);};
  if($g('galaxy-system-select'))$g('galaxy-system-select').onchange=e=>{if(e.target.value)void showSystem(sector,e.target.value);};
 }
 function header(title,sub){const hint=host.querySelector('.space-instructions');if(hint)hint.textContent=stage==='system'?profile().plaetze_je_system+' Planeten · Zum Unterscheiden näher heranzoomen':stage==='galaxy'?profile().sektoren+' Sektoren · Stern öffnet seinen Sektor':'Stern antippen · Ziehen dreht die Ansicht';closeDetail();if(!gleichesSystem){listeZu();panelZu();}$g('galaxy-occupancy')?.remove();$g('galaxy-friendly')?.remove();$g('galaxy-map-legend')?.remove();ownerDirectory=null;friendlyDirectory=null;host.dataset.stage=stage;$g('galaxy-title').textContent=title;$g('galaxy-subtitle').textContent=sub;for(const b of document.querySelectorAll('.space-density-control')){b.hidden=stage!=='sector';b.setAttribute('aria-pressed',String(b.dataset.sectorView==='overview'));}$g('galaxy-motion').hidden=stage!=='system';if($g('galaxy-drehtempo'))$g('galaxy-drehtempo').hidden=stage!=='system';levelNavigation();pfadZeichnen();motionState();}
 function index(items){$g('galaxy-index').innerHTML=items.map(i=>`<button type="button" data-space-kind="${esc(i.kind)}" data-space-key="${esc(i.key)}" class="${i.own?'space-own':''}" aria-label="${esc(i.text)} öffnen">${esc(i.text)}${i.owner?' '+relationHtml(i.owner,i.relation):i.known?' <small>kartiert</small>':''}</button>`).join('');if(window.STERNENEPOCHE?.local&&items.length)document.dispatchEvent(new CustomEvent('local-galaxy-view',{detail:{stage,sektor:sector,system,items}}));}
 function showGalaxy(){
  folgeKennung='';gleichesSystem=false;stage='galaxy';selected=0;epoch++;if(renderer)resetScene();$g('galaxy-loading').hidden=true;header('Die Galaxie',profile().sektoren+' Sektoren · Sektor wählen zum Erkunden');
  const w=profile(),items=[],starPositions=[],starColors=[],known=new Map();for(const p of [...(view?.atlas||[]).map(p=>({...p,besiedlung_bekannt:p.besiedlung_bekannt??!!p.spieler})),...(view?.planeten||[]).map(p=>({...p,spieler:view.name,bekannt:true}))]){const key=p.koord.split(':').slice(0,2).join(':');if(!known.has(key))known.set(key,new Map());known.get(key).set(p.koord,p);}const places=[...known.values()].flatMap(m=>[...m.values()]);prepareOwners(places);friendlyNavigation(places);
  if(renderer)galaxieScheibe();
  for(let s=1;s<=w.sektoren;s++){
   const a=(s-1)/w.sektoren*Math.PI*2+.2,cx=Math.cos(a)*170,cz=Math.sin(a)*170;
   if(renderer){const marker=new THREE.Mesh(new THREE.SphereGeometry(16,12,10),new THREE.MeshBasicMaterial({visible:false}));marker.position.set(cx,0,cz);marker.userData={kind:'sector',key:s};group.add(marker);targets.push(marker);label('SEKTOR '+s,marker,s,'sector');
    for(let n=1;n<=w.systeme_je_sektor;n++){const p=GalaxyModel.galaxyPosition(s,n,w.sektoren,w.systeme_je_sektor),mark=GalaxyModel.summary([...(known.get(s+':'+n)?.values()||[])]),star=new THREE.Mesh(new THREE.SphereGeometry(3,6,4),new THREE.MeshBasicMaterial({visible:false}));star.position.set(p.x,p.y,p.z);star.userData={kind:'system',key:n,sektor:s};group.add(star);targets.push(star);label('System '+s+':'+n,star,n,'system',s,mark);if(mark.state!=='empty'){starPositions.push(p.x,p.y,p.z);const color=new THREE.Color(systemColor(mark));starColors.push(color.r,color.g,color.b);}}

   }
   items.push({kind:'sector',key:s,text:'Sektor '+s,own:view.planeten.some(p=>Number(p.koord.split(':')[0])===s)});
  }
  index(items);$g('galaxy-table').innerHTML='';if(renderer){const geometry=new THREE.BufferGeometry();geometry.setAttribute('position',new THREE.Float32BufferAttribute(starPositions,3));geometry.setAttribute('color',new THREE.Float32BufferAttribute(starColors,3));const points=new THREE.Points(geometry,new THREE.ShaderMaterial({uniforms:{px:{value:renderer.getPixelRatio()},gr:{value:w.sektoren*w.systeme_je_sektor>1000?4.5:6}},transparent:true,depthWrite:false,blending:THREE.AdditiveBlending,vertexColors:true,
   vertexShader:'uniform float px,gr;varying vec3 vC;void main(){vC=color;gl_Position=projectionMatrix*modelViewMatrix*vec4(position,1.);gl_PointSize=gr*px;}',fragmentShader:'varying vec3 vC;void main(){float r=length(gl_PointCoord-.5);float k=smoothstep(.5,.0,r);gl_FragColor=vec4(vC*(k*k*2.6+smoothstep(.18,.0,r)*2.),k);}'}));group.add(points);addFriendlyBeacons();fitView(580);}
 }
 async function fetchSector(s,requestEpoch){
  if(cache.has(s))return cache.get(s);const w=profile(),batches=[];
  for(let from=1;from<=w.systeme_je_sektor;from+=20)batches.push(api('/api/tool',{typ:'galaxie',sektor:s,von:from,bis:Math.min(from+19,w.systeme_je_sektor)}));
  const responses=await Promise.all(batches),data=responses.flatMap(v=>v.systeme||[]).map(v=>({...v,sektor:s}));
  if(requestEpoch!==epoch)return null;cache.set(s,data);return data;
 }
 async function showSector(s){
  folgeKennung='';gleichesSystem=false;sector=Number(s);stage='sector';const request=++epoch;if(renderer)resetScene();index([]);$g('galaxy-table').innerHTML='';header('Sektor '+sector,profile().systeme_je_sektor+' Sonnensysteme · Stern antippen zum Öffnen');$g('galaxy-loading').hidden=false;
  try{const systems=await fetchSector(sector,request);if(!systems||request!==epoch)return;if(renderer)resetScene();prepareOwners(systems.flatMap(s=>s.plaetze||[]));sectorOwners(systems);friendlyNavigation(systems.flatMap(s=>s.plaetze||[]));
   index(systems.map(s=>({kind:'system',key:s.system,text:'System '+sector+':'+s.system,known:s.bekannt,own:(s.plaetze||[]).some(p=>own(p.koord)),mark:GalaxyModel.summary(s.plaetze||[])})));
   if(renderer){const ownerArcs=new Map(),densityScale=Math.min(1,Math.sqrt(60/systems.length));const haze=sprite('#40577b',370,.13);haze.position.y=-18;group.add(haze);for(const s of systems){const p=GalaxyModel.position(sector,s.system,profile().systeme_je_sektor),mark=GalaxyModel.summary(s.plaetze||[]),isOwn=mark.relation==='eigen',color=systemColor(mark);
    const star=new THREE.Mesh(new THREE.SphereGeometry((isOwn?2.6:mark.inhabitants.length?2:1.4)*densityScale,12,10),new THREE.MeshBasicMaterial({color:new THREE.Color(color).multiplyScalar(2.3)}));star.position.set(p.x,p.y,p.z);star.visible=mark.state!=='empty';group.add(star);const hit=new THREE.Mesh(new THREE.SphereGeometry(4,10,8),new THREE.MeshBasicMaterial({visible:false}));hit.position.copy(star.position);hit.userData={kind:'system',key:s.system,sektor:sector};group.add(hit);targets.push(hit);const halo=sprite(color,(isOwn?8:6)*densityScale,mark.state==='empty'?0:mark.inhabitants.length?.24:.12);halo.position.copy(star.position);group.add(halo);sectorStars.push({mesh:star,halo,color:star.material.color.clone(),opacity:halo.material.opacity,state:mark.state,owners:mark.inhabitants.map(r=>r.name)});if(mark.inhabitants.length){mark.inhabitants.forEach((r,i)=>{const points=ownerArcs.get(r.name)||[],count=mark.inhabitants.length;for(let j=0;j<16;j++)for(const end of [j,j+1]){const a=(i+(end/16)*.88)/count*Math.PI*2;points.push(p.x+Math.cos(a)*4.5,p.y+.5,p.z+Math.sin(a)*4.5);}ownerArcs.set(r.name,points);});}label('System '+sector+':'+s.system,star,s.system,'system',sector,mark);}
    const positions=[],colors=[],ranges=[];for(const [name,points] of ownerArcs){const start=positions.length;for(const value of points)positions.push(value);const color=new THREE.Color(ownerMarks.get(name).color);for(let i=0;i<points.length;i+=3)colors.push(color.r,color.g,color.b);ranges.push({name,start,end:positions.length,color});}
    const geometry=new THREE.BufferGeometry();geometry.setAttribute('position',new THREE.Float32BufferAttribute(positions,3));geometry.setAttribute('color',new THREE.Float32BufferAttribute(colors,3));const arcs=new THREE.LineSegments(geometry,new THREE.LineBasicMaterial({vertexColors:true,transparent:true,opacity:.55}));group.add(arcs);sectorArcBatch={arcs,ranges};addFriendlyBeacons();pitch=1.15;fitView(350);
   }
   $g('galaxy-table').innerHTML='<p class="muted">Wähle ein System, um seine Umlaufbahnen und die Aufklärungsberichte zu sehen.</p>';
  }catch(e){if(request===epoch)show(e.message,true);}finally{if(request===epoch)$g('galaxy-loading').hidden=true;}
 }
 async function showSystem(s,n){
  gleichesSystem=Number(s)===sector&&Number(n)===system&&stage==='system';if(!gleichesSystem)folgeKennung='';sector=Number(s);system=Number(n);stage='system';selected=0;const request=++epoch;if(renderer)resetScene();index([]);$g('galaxy-table').innerHTML='';header('System '+sector+':'+system,profile().plaetze_je_system+' Planeten · Planet antippen für Details');$g('galaxy-loading').hidden=false;
  try{const systems=await fetchSector(sector,request);if(!systems||request!==epoch)return;const data=systems.find(s=>s.system===system);if(!data)throw Error('System nicht vorhanden');systemDaten=data;const ps=GalaxyModel.planets(data,sector,profile().plaetze_je_system);if(renderer)resetScene();prepareOwners(ps);
   systemOccupancy(ps);friendlyNavigation(ps);
   index(ps.map(p=>({kind:'planet',key:p.position,text:p.koord,known:p.bekannt,own:own(p.koord),owner:p.spieler,relation:p.beziehung,mark:GalaxyModel.summary([p],1)})));
   if(renderer){const kamera=gleichesSystem&&autoFit==null?{yaw,pitch,distance,f:{...focus}}:null;systemSzene(data,ps);pitch=handy.matches?1.02:.8;fitView(330);if(kamera){yaw=kamera.yaw;pitch=kamera.pitch;distance=kamera.distance;Object.assign(focus,kamera.f);autoFit=null;cameraUpdate();}neu();
    if(gleichesSystem&&offenesFeld){const f=offenesFeld;setTimeout(()=>pick(f.kind,f.key),0);}gleichesSystem=false;}
   $g('galaxy-table').innerHTML=`<div class="system-actions"><button type="button" data-scan-system="${sector}:${system}:1">Ganzes System erkunden</button>${data.asteroidenguertel?`<button type="button" data-work-site="${sector}:${system}:0" data-work-mission="abbau">Asteroidengürtel abbauen</button>`:''}<small>Ein Klick plant alle Planetenscans. Sonden, Treibstoff und freie Flottenplätze bestimmen den Ablauf.</small></div><details ${renderer?'':'open'}><summary>Aufklärungsberichte · System ${sector}:${system}</summary>${galaxyReports([data])}</details>${ps.map(p=>debrisInfo(p.koord)).join('')}`;
  }catch(e){if(request===epoch)show(e.message,true);}finally{if(request===epoch)$g('galaxy-loading').hidden=true;}
 }
 function updateArcColors(){
  if(!sectorArcBatch)return;
  const colors=sectorArcBatch.arcs.geometry.attributes.color;
  for(const r of sectorArcBatch.ranges){const factor=!selectedOwner||selectedOwner===r.name||hasFriendly([r.name])?1:.12;for(let i=r.start;i<r.end;i+=3)colors.setXYZ(i/3,r.color.r*factor,r.color.g*factor,r.color.b*factor);}colors.needsUpdate=true;
 }
 function sectorOwners(systems){directoryPanel(systems.flatMap(s=>s.plaetze||[]),false);}
 function systemOccupancy(places){directoryPanel(places,true);}
 function directoryPanel(places,systemMode){
  const panel=document.createElement('section');panel.id='galaxy-occupancy';$g('galaxy-viewport').after(panel);
  const residents=GalaxyModel.inhabitants(places).sort((a,b)=>(a.relation==='eigen'?0:a.relation==='verbuendet'?1:2)-(b.relation==='eigen'?0:b.relation==='verbuendet'?1:2)||a.name.localeCompare(b.name,'de'));
  ownerDirectory={residents,places:new Map(places.map(p=>[p.koord,p])),systemMode,page:0,query:''};
  panel.innerHTML=`<header><strong>${systemMode?'Besitzer & Umlaufbahnen':'Besitzer im Sektor '+sector}</strong><small>${residents.length} bekannte Spieler</small></header><p class="space-owner-hint">Buchstaben verbinden Besitzer und Planeten. Farben wiederholen sich bei vielen Spielern; Namen antippen hebt ihre Positionen hervor.</p><div class="space-directory-tools"><input type="search" data-owner-search aria-label="Spieler, Kürzel oder Koordinaten suchen" placeholder="Spieler, Kürzel, Koordinaten"><button type="button" class="space-owner-reset" data-space-owner="" aria-pressed="true">Alle zeigen</button></div><div id="space-owner-results"></div><div id="space-owner-pages" class="space-directory-pages"></div>`;
  renderDirectory();
 }
 function renderDirectory(){
  if(!ownerDirectory)return;const d=ownerDirectory,q=d.query.toLocaleLowerCase('de'),all=d.residents.filter(r=>!q||[r.name,r.tag,ownerMarks.get(r.name)?.code,...r.coords].some(v=>v?.toLocaleLowerCase('de').includes(q))),pages=Math.max(1,Math.ceil(all.length/8));d.page=Math.min(d.page,pages-1);
  const subset=all.slice(d.page*8,d.page*8+8);
  $g('space-owner-results').className=d.systemMode?'system-owner-list':'sector-owner-list';
  $g('space-owner-results').innerHTML=subset.map(r=>{const m=ownerMarks.get(r.name),pressed=String(selectedOwner===r.name),key=`<b class="owner-key">${m.code}</b>`,relation=GALAXY_RELATION_NAMES[r.relation];
   if(!d.systemMode)return `<button type="button" class="sector-owner-choice" data-owner-farbe="${esc(m.color)}" data-space-owner="${esc(r.name)}" aria-pressed="${pressed}">${key}<span>${esc(r.name)}<small>${new Set(r.coords.map(c=>c.split(':').slice(0,2).join(':'))).size} Systeme · ${relation}</small></span></button>`;
   return `<div class="space-owner-row" data-owner-farbe="${esc(m.color)}"><div><button type="button" class="space-owner-name" data-space-owner="${esc(r.name)}" aria-pressed="${pressed}">${key}${esc(r.name)}${r.tag?' ['+esc(r.tag)+']':''}</button><small>${relation} · ${r.coords.length} ${r.coords.length===1?'Planet':'Planeten'}</small></div><button type="button" class="space-owner-mail" data-report-contact="${esc(r.name)}" aria-label="Nachricht an ${esc(r.name)}">Nachricht ↗</button><div class="space-owner-planets" aria-label="Planeten dieses Spielers">${r.coords.sort((a,b)=>Number(a.split(':')[2])-Number(b.split(':')[2])).map(c=>{const p=d.places.get(c),n=Number(c.split(':')[2]);return `<button type="button" data-space-kind="planet" data-space-key="${n}" aria-label="Planet ${esc(c)} von ${esc(r.name)} ansehen" title="${esc(c)}">${p?.bekannt&&p.zone?`<img src="${esc(planetArt(p))}" alt="">`:'<span class="space-unknown-orb" aria-hidden="true">?</span>'}<span>${n}</span></button>`;}).join('')}</div></div>`;
  }).join('')||'<p>Keine passenden bekannten Besitzer.</p>';farbenSetzen($g('space-owner-results'));
  $g('space-owner-pages').innerHTML=`<button type="button" data-owner-page="-1" ${d.page===0?'disabled':''} aria-label="Vorherige Besitzer">←</button><span role="status">${all.length?d.page*8+1:0}–${Math.min((d.page+1)*8,all.length)} / ${all.length}</span><button type="button" data-owner-page="1" ${d.page===pages-1?'disabled':''} aria-label="Weitere Besitzer">→</button>`;
 }
 function friendlyNavigation(places){
  mapLegend();
  const unique=new Map(places.filter(p=>(p.bekannt||p.besiedlung_bekannt)&&friendly(GalaxyModel.relation(p))).map(p=>[p.koord,p]));
  const entries=[...unique.values()].sort((a,b)=>a.koord.localeCompare(b.koord,'de',{numeric:true}));
  friendlyDirectory={entries,debris:(view.truemmer||[]).filter(t=>stage==='galaxy'||t.koord.startsWith(sector+':'+(stage==='system'?system+':':''))),relation:'',query:'',page:0};
  const panel=document.createElement('section');panel.id='galaxy-friendly';$g('galaxy-viewport').before(panel);
  panel.innerHTML=`<div class="space-friendly-tabs">${['eigen','verbuendet','debris'].map(r=>`<button type="button" data-friendly-tab="${r}" aria-expanded="false" aria-controls="space-friendly-body"><b>${r==='eigen'?'◆':r==='debris'?'✧':'○'}</b> ${r==='eigen'?'Eigene':r==='debris'?'Trümmer':'Verbündete'} <strong>${r==='debris'?friendlyDirectory.debris.length:entries.filter(p=>GalaxyModel.relation(p)===r).length}</strong></button>`).join('')}</div><div id="space-friendly-body" hidden><input type="search" data-friendly-search aria-label="Eigene und verbündete Kolonien suchen" placeholder="Name oder Koordinaten"><div id="space-friendly-results"></div><div id="space-friendly-pages" class="space-directory-pages"></div></div>`;
 }
 function renderFriendly(){
  const d=friendlyDirectory;if(!d)return;const q=d.query.toLocaleLowerCase('de'),entries=(d.relation==='debris'?d.debris:d.entries).filter(p=>(d.relation==='debris'||GalaxyModel.relation(p)===d.relation)&&(!q||[p.koord,p.spieler,p.name].some(v=>v?.toLocaleLowerCase('de').includes(q)))),pages=Math.max(1,Math.ceil(entries.length/6));d.page=Math.min(d.page,pages-1);
  $g('space-friendly-body').hidden=!d.relation;
  for(const b of document.querySelectorAll('[data-friendly-tab]'))b.setAttribute('aria-expanded',String(b.dataset.friendlyTab===d.relation));
  $g('space-friendly-results').innerHTML=entries.slice(d.page*6,d.page*6+6).map(p=>`<button type="button" ${d.relation==='debris'?'data-map-salvage':'data-friendly-target'}="${esc(p.koord)}"><strong>${esc(p.koord)}</strong><span>${d.relation==='debris'?fmt(p.erz+p.kristall)+' Rohstoffe':esc(p.spieler||view.name)}</span><span aria-hidden="true">↗</span></button>`).join('')||'<p>Keine passenden bekannten Ziele in dieser Ansicht.</p>';
  $g('space-friendly-pages').innerHTML=`<button type="button" data-friendly-page="-1" ${d.page===0?'disabled':''} aria-label="Vorherige Kolonien">←</button><span role="status">${entries.length?d.page*6+1:0}–${Math.min((d.page+1)*6,entries.length)} / ${entries.length}</span><button type="button" data-friendly-page="1" ${d.page===pages-1?'disabled':''} aria-label="Weitere Kolonien">→</button>`;
 }
 document.addEventListener('input',e=>{if(e.target.matches('[data-owner-search]')&&ownerDirectory){ownerDirectory.query=e.target.value;ownerDirectory.page=0;renderDirectory();}if(e.target.matches('[data-friendly-search]')&&friendlyDirectory){friendlyDirectory.query=e.target.value;friendlyDirectory.page=0;renderFriendly();}});
 document.addEventListener('click',e=>{const b=e.target.closest('[data-sector-view]');if(!b||stage!=='sector')return;sectorNumberView=b.dataset.sectorView==='numbers';host.dataset.numberFocus=String(sectorNumberView);focus.x=focus.y=focus.z=0;pitch=1.15;if(sectorNumberView){distance=180;cameraUpdate();}else fitView(350);for(const button of document.querySelectorAll('[data-sector-view]'))button.setAttribute('aria-pressed',String(button===b));});
 document.addEventListener('click',async e=>{
  const b=e.target.closest('[data-owner-page],[data-friendly-tab],[data-friendly-page],[data-friendly-target]');if(!b)return;
  if(b.dataset.ownerPage&&ownerDirectory){ownerDirectory.page+=Number(b.dataset.ownerPage);renderDirectory();}
  else if(b.dataset.friendlyTab&&friendlyDirectory){friendlyDirectory.relation=friendlyDirectory.relation===b.dataset.friendlyTab?'':b.dataset.friendlyTab;friendlyDirectory.page=0;renderFriendly();}
  else if(b.dataset.friendlyPage&&friendlyDirectory){friendlyDirectory.page+=Number(b.dataset.friendlyPage);renderFriendly();}
  else if(b.dataset.friendlyTarget){const [s,n,p]=b.dataset.friendlyTarget.split(':').map(Number);await showSystem(s,n);if(stage==='system'&&sector===s&&system===n){pick('planet',p,s);$g('galaxy-viewport').scrollIntoView({block:'start'});}}
 });
 function debrisInfo(coord){
  const t=(view.truemmer||[]).find(t=>t.koord===coord);if(!t)return '';
  return `<section class="orbital-debris" data-debris-field="${esc(coord)}"><span class="eyebrow">ORBIT · TRÜMMERFELD</span><h4>Bergbare Rohstoffe</h4><div class="debris-resources">${['erz','kristall'].map(k=>`<span>${imageTag('resources',k,'')}<strong>${fmt(t[k])}</strong> ${esc(named(k))}</span>`).join('')}</div><p>Recycler und Bergungsschiffe bergen bis zur freien Bergungskapazität. Andere Flotten können das Feld vorher leeren.</p><button type="button" data-map-salvage="${esc(coord)}">Recycler direkt senden</button></section>`;
 }
 function debrisMarker(mesh,p,radius){
  if(!(view.truemmer||[]).some(t=>t.koord===p.koord))return;
  const field=new THREE.Group();mesh.add(field);
  const fragments=[];for(let i=0;i<48;i++){const a=i*2.39996323,r=radius+3.8+(i%5)*.6;fragments.push(Math.cos(a)*r,Math.sin(i*7)*1.4,Math.sin(a)*r);}
  const geometry=new THREE.BufferGeometry();geometry.setAttribute('position',new THREE.Float32BufferAttribute(fragments,3));field.add(new THREE.Points(geometry,new THREE.PointsMaterial({color:0xc5ac83,size:.75,transparent:true,opacity:.8})));
  const marker=new THREE.Mesh(new THREE.SphereGeometry(3,8,6),new THREE.MeshBasicMaterial({visible:false}));marker.position.set(radius+7,2,0);marker.userData={kind:'debris',key:p.position};mesh.add(marker);targets.push(marker);label('Trümmer · '+p.koord,marker,p.position,'debris');
  const hint=document.createElement('span');hint.className='space-debris-halo';hint.setAttribute('aria-hidden','true');$g('galaxy-labels').append(hint);Object.assign(labels[labels.length-1],{debrisPlanet:mesh,debrisHalo:hint});
 }
 let salvageSequence=0,salvageTimer=0,salvageSending=false;
 function salvageAction(form){const schiffe={};for(const key of ['recycler','bergungsschiff']){const n=Number(form.elements[key].value);if(!Number.isSafeInteger(n)||n<0)throw Error('Bitte eine ganze Schiffsanzahl wählen.');if(n)schiffe[key]=n;}return {typ:'flotte_senden',mission:'recyceln',start:form.elements.start.value,ziel:form.dataset.target,geschwindigkeit:Number(form.elements.tempo.value),haltedauer_stunden:0,schiffe,ladung:{}};}
 function salvageStock(form){const p=view.planeten.find(p=>p.koord===form.elements.start.value),field=(view.truemmer||[]).find(t=>t.koord===form.dataset.target),menge=(field?.erz||0)+(field?.kristall||0),leute=Number(field?.ueberlebende)||0;
  const rec=p?.schiffe?.recycler||0,berg=p?.schiffe?.bergungsschiff||0,nRec=Math.min(rec,Math.max(rec?1:0,Math.ceil(menge/ladung('recycler')))),rest=Math.max(0,menge-nRec*ladung('recycler')),nBerg=Math.min(berg,Math.max(leute>0||!nRec?1:0,Math.ceil(rest/ladung('bergungsschiff'))));
  for(const [key,count,wert] of [['recycler',rec,nRec],['bergungsschiff',berg,nBerg]]){form.elements[key].max=count;form.elements[key].value=wert;form.querySelector(`[data-salvage-stock="${key}"]`).textContent=fmt(count)+' verfügbar';}}
 const ladung=k=>Number(rules?.catalog?.einheiten?.[k]?.regel?.ladung)||20000;
 function openSalvage(coord,opt={}){
  if(salvageSending)return;clearTimeout(salvageTimer);salvageSequence++;
  const field=(view.truemmer||[]).find(t=>t.koord===coord)||(opt.frisch?{koord:coord,erz:null,kristall:null}:null);if(!field){show('An dieser Position ist kein Trümmerfeld mehr bekannt.',true);return;}
  // Startplanet: der aktuelle, wenn er Bergungsschiffe hat, sonst der mit der größten Ladekapazität
  const bases=view.planeten.filter(p=>(p.schiffe?.recycler||0)+(p.schiffe?.bergungsschiff||0)>0),kap=p=>(p.schiffe?.recycler||0)*ladung('recycler')+(p.schiffe?.bergungsschiff||0)*ladung('bergungsschiff'),preferred=bases.find(p=>p.koord===currentPlanet()?.koord)||[...bases].sort((a,b)=>kap(b)-kap(a))[0];
  $g('galaxy-detail').innerHTML=`<span class="eyebrow">ORBIT · BERGUNG</span><h3 id="galaxy-planet-title">Trümmer ${esc(coord)}</h3><div class="debris-resources">${['erz','kristall'].map(k=>`<span>${imageTag('resources',k,'')}<strong>${field[k]==null?'wird erfasst':fmt(field[k])}</strong> ${esc(named(k))}</span>`).join('')}${field.ueberlebende?`<span><strong>${fmt(field.ueberlebende)}</strong> Überlebende</span>`:''}</div>${bases.length?`<form id="map-salvage-form" data-target="${esc(coord)}"><label>Startplanet<select name="start">${bases.map(p=>`<option value="${esc(p.koord)}" ${p===preferred?'selected':''}>${esc(p.koord)} · ${fmt(p.schiffe.recycler||0)} Recycler · ${fmt(p.schiffe.bergungsschiff||0)} Bergungsschiffe</option>`).join('')}</select></label><div class="map-salvage-ships">${['recycler','bergungsschiff'].map(k=>`<label>${imageTag('ships',k,view.volk)}<span>${esc(named(k))}</span><small data-salvage-stock="${k}"></small><input type="number" name="${k}" min="0" step="1" value="0" inputmode="numeric"></label>`).join('')}</div><label>Tempo<select name="tempo">${Array.from({length:10},(_,i)=>10-i).map(n=>`<option value="${n/10}">${n*10} %</option>`).join('')}</select></label><div id="map-salvage-quote" aria-live="polite"></div><button type="submit" disabled>Bergungsflotte starten</button></form>`:'<p>Keine stationierten Recycler oder Bergungsschiffe verfügbar. Baue ein Bergungsschiff oder warte auf eine zurückkehrende Flotte.</p>'}<p class="muted">Die Bergung erfolgt bei Ankunft. Andere Flotten können das Feld vorher leeren. Kraftstoff für Hin- und Rückflug wird beim Start reserviert.</p>`;
  if(!$g('galaxy-infocard').open)$g('galaxy-infocard').showModal();const form=$g('map-salvage-form');if(form){salvageStock(form);void previewSalvage(form);}
 }
 async function previewSalvage(form){
  const sequence=++salvageSequence,identity=planIdentity(),button=form.querySelector('[type=submit]'),host=form.querySelector('#map-salvage-quote');button.disabled=true;delete form.dataset.ready;
  try{const action=salvageAction(form);if(!Object.keys(action.schiffe).length)throw Error('Mindestens einen Recycler oder ein Bergungsschiff auswählen.');
   const [flight,mission]=await Promise.all([api('/api/tool',{...action,typ:'flugzeit'}),api('/api/tool',{typ:'flugkoordination',flug:action,rueckgeschwindigkeit:action.geschwindigkeit})]);
   if(sequence!==salvageSequence||identity!==planIdentity()||!form.isConnected||JSON.stringify(action)!==JSON.stringify(salvageAction(form)))return;
   const reason=gameReadOnly()||(mission.bereit&&mission.zeit_gueltig?'':mission.grund||'Startbedingungen nicht erfüllt.'),field=(view.truemmer||[]).find(t=>t.koord===action.ziel),sum=(field?.erz||0)+(field?.kristall||0),capacity=flight.ladekapazitaet;
   setStableHTML(host,`<div class="map-salvage-facts"><span><small>Hinflug · Spielzeit</small><b>${GameTime.duration(flight.dauer_sekunden??flight.dauer_min*60)}</b></span><span><small>Entfernung</small><b>${fmt(flight.entfernung)} RE</b></span><span><small>Deuterium · Hin & zurück</small><b>${fmt(fuelReserve(flight,2))}</b></span><span><small>Bergungskapazität</small><b>${fmt(capacity)}</b></span></div><p>${reason?esc(reason):`Bis zu ${fmt(Math.min(sum,capacity))} Rohstoffe aus dem bekannten Feld bergbar.`}</p>`);
   if(!reason){form.dataset.ready=JSON.stringify(action);button.disabled=salvageSending;}
  }catch(e){if(sequence===salvageSequence&&form.isConnected)host.textContent=e.message;}
 }
 document.addEventListener('click',e=>{const b=e.target.closest('[data-map-salvage]');if(b)openSalvage(b.dataset.mapSalvage);});
 document.addEventListener('input',e=>{const form=e.target.closest('#map-salvage-form');if(!form||salvageSending)return;if(e.target.name==='start')salvageStock(form);clearTimeout(salvageTimer);salvageSequence++;delete form.dataset.ready;form.querySelector('[type=submit]').disabled=true;salvageTimer=setTimeout(()=>void previewSalvage(form),180);});
 document.addEventListener('submit',async e=>{const form=e.target.closest('#map-salvage-form');if(!form)return;e.preventDefault();if(salvageSending)return;try{const action=salvageAction(form);if(form.dataset.ready!==JSON.stringify(action))throw Error('Bitte die aktuelle Flugprüfung abwarten.');salvageSending=true;form.querySelector('[type=submit]').disabled=true;const result=await command([action]);if(result.ergebnisse?.some(r=>!r.ok))throw Error(result.ergebnisse.filter(r=>!r.ok).map(r=>r.text).join(' · '));window.Mobile035?.rememberFlight(action);closeDetail();}catch(error){show(error.message,true);}finally{salvageSending=false;if(form.isConnected)void previewSalvage(form);}});
 function planetDetail(p){
  if(!p)return;selected=p.position;updateOrbits();neu();
  const known=!!p.bekannt,observed=known||!!p.besiedlung_bekannt,protectedHome=!!p.heimat,owner=observed?(p.spieler??(p.status==='frei'||p.bewohnt===false?'Unbewohnt':'Unbekannt')):'Unbekannt';
  $g('galaxy-detail').innerHTML=`<span class="eyebrow">PLANET ${p.position} · INFOKARTE</span><h3 id="galaxy-planet-title">${window.Planetenname?Planetenname.html(p.koord,{standardname:true})+Planetenname.stift(p.koord):esc(p.koord)}</h3>${known&&p.zone?`<img class="space-planet-art ${p.besiegt?'defeated-planet':''}" src="${esc(planetArt(p))}" alt="${esc(named(p.zone))}">`:'<div class="space-unknown" aria-label="Unbekannter Planet">?</div>'}<strong>${esc(p.besiegt?'Besiegt':named(p.status))}</strong><dl><dt>Planetentyp</dt><dd>${esc(known?named(p.zone||'unbekannt'):'Unbekannt')}</dd><dt>Bewohner</dt><dd>${observed&&p.spieler?relationHtml(owner,p.beziehung):esc(owner)}</dd><dt>Heimatwelt</dt><dd>${protectedHome?'Geschützt':'—'}</dd></dl><p>${known?planetResources(p):'Eine Planetensonde enthüllt Typ, Bewohner und Ressourcen.'}</p>${typeof habitabilityMarkup==='function'?habitabilityMarkup(p.habitabilitaet):''}${debrisInfo(p.koord)}${window.Mobile035?.planetFacts(p)||''}<div class="section-actions"><button type="button" data-planet-mission="angriff" data-target="${esc(p.koord)}" ${own(p.koord)?'disabled':''}>Angriff planen</button>${known&&!own(p.koord)&&p.status!=='frei'&&!protectedHome?`<button type="button" data-planet-mission="kampfkolonisieren" data-target="${esc(p.koord)}">Invasion planen</button>`:''}<button type="button" data-explore="${esc(p.koord)}" data-system="${!(cache.get(sector)||[]).find(s=>s.system===system)?.bekannt}">${(cache.get(sector)||[]).find(s=>s.system===system)?.bekannt?'Sonde planen':'Systemscan planen'}</button>${known&&p.status==='frei'&&!protectedHome?`<button type="button" class="secondary" data-colonize="${esc(p.koord)}">Kolonie planen</button>`:''}${own(p.koord)?`<button type="button" class="secondary" data-space-own="${esc(p.koord)}">Planet verwalten</button>`:''}</div>`;
  labels.forEach(l=>{if(l.kind==='planet')l.el.setAttribute('aria-pressed',String(Number(l.key)===selected));});
  if(!$g('galaxy-infocard').open)$g('galaxy-infocard').showModal();
 }
 function pick(kind,key,fromSector=sector){if(kind==='sector')void showSector(key);else if(kind==='system'){if(stage==='galaxy')void showSector(fromSector);else void showSystem(fromSector,key);}else if(kind==='debris'){const k=`${fromSector}:${system}:${key}`;if(overlayEl)truemmerFeldInfo(k);else openSalvage(k);}else if(kind==='mond'){const p=planets.find(m=>Number(m.userData.data.position)===Number(key))?.userData.data;if(p)mondFeld(p);}else if(kind==='sonne')sonneFeld();else if(kind==='guertel')guertelFeld();else if(kind==='dyson')dysonFeld();else if(kind==='wk')wkFeld(Number(key));else if(kind==='loch'){window.Mondseite?.reisen(key);}else if(kind==='tor'){const p=planets.find(m=>Number(m.userData.data.position)===Number(key))?.userData.data;if(p)torFeld(p);}else if(kind==='planet'){const gezeigt=planets.find(m=>Number(m.userData.data.position)===Number(key))?.userData.data,data=gezeigt?null:(cache.get(sector)||[]).find(s=>s.system===system);const p=gezeigt||(data&&GalaxyModel.planets(data,sector,profile().plaetze_je_system).find(p=>p.position===Number(key)));if(p)(overlayEl?panelZeigen:planetDetail)(p);}}
 function frame(time){
  frameId=0;if(!active||document.hidden||!renderer||!imBild){last=0;letztesBild=0;return;}
  // Ruhezustand: 30 Bilder/s – die Umläufe sind langsam, das Handy bleibt kühl und flüssig; bei Bedienung wieder jedes Bild.
  sparModus=sparen(time);if(sparModus&&lief&&letztesBild&&time-letztesBild<29){planen();return;}
  inFrame=true;let laeuft=false;const js0=performance.now();gpuLesen();
  const delta=last?Math.min(.1,(time-last)/1000):0;if(lief&&letztesBild&&time-letztesBild<250)leistung(time-letztesBild,sparModus?1000/30:1000/60);letztesBild=time;last=time;zeitU.value=performance.now()/1000;
  if(kameraSchritt(delta||1/60))dirty=laeuft=true;
  {const panel=overlayEl?.querySelector('.g3-panel'),cw=canvas.clientWidth,ch=canvas.clientHeight,soll=folge&&handy.matches&&panel&&!panel.hidden?Math.min(panel.offsetHeight,ch*.62)*.5:0;versatz+=(soll-versatz)*Math.min(1,(delta||1/60)*8);if(Math.abs(soll-versatz)>.5)dirty=laeuft=true;if(versatz>.5&&cw&&ch)camera.setViewOffset(cw,ch,0,versatz,cw,ch);else if(camera.view)camera.clearViewOffset();}if(stage==='system'&&sys3&&(!reduced.matches||motionOverride))dirty=laeuft=true;if(stage==='galaxy'&&galaxieDreh&&(!reduced.matches||motionOverride)){galaxieDreh.rotation.y-=(delta||0)*.012;dirty=laeuft=true;}if(pendingHover){const h=hitAt(pendingHover);highlight(h);hoverZeigen(h,pendingHover);pendingHover=null;}
  if(stage==='system'&&!paused&&(!reduced.matches||motionOverride)&&!world?.paused&&!world?.beendet&&GameTime.anchor.connected){clock+=delta;dirty=laeuft=true;}
  // Eigendrehung: eigenes Tempo (Einstellung „Drehung“), im Sonnensystem schneller als in der Planetenansicht.
  if(stage==='system'&&(loecher.length||weltenkruemmer.length))dirty=laeuft=true;
  const tempo=window.PlanetKugel?.tempo()??1;if(stage==='system'&&tempo>0){spin+=delta*tempo;const per=window.PlanetKugel?.PERIODE.system||{mond:16};for(const m of moons)m.dreh+=delta*tempo*2*Math.PI/per.mond*(window.PlanetKugel?.mondFaktor(m.alter+performance.now()/1000-m.seit)??1);dirty=laeuft=true;}
  if(dirty){for(const p of planets){bahnPunkt(p.userData.data.position,clock,p.position);p.rotation.y=-(spin*2*Math.PI/(window.PlanetKugel?.PERIODE.system.planet||36)+p.userData.data.position);}const mondUmlauf=window.PlanetKugel?.PERIODE.system.mondumlauf||70;for(const m of moons){const a=m.phase+spin*2*Math.PI/mondUmlauf,c=m.planet.position,u=m.planet.userData;m.mesh.position.copy(c).addScaledVector(u.e1,Math.cos(a)*m.bahn).addScaledVector(u.e2,Math.sin(a)*m.bahn);m.mesh.rotation.y=-m.dreh;if(m.tor)torBewegen(m,a,c,u);}weltenkruemmerBewegen();for(const l of loecher){const offen=l.bis==null||l.bis>GameTime.now();if(l.tor){l.tor.g.getWorldPosition(l.sprite.position);torLoch(l.tor,offen);}else{const c=(l.moon?.mesh||l.planet).position,r=(l.moon?l.moon.mesh.geometry.parameters.radius:l.planet.geometry.parameters.radius)+4.2;l.sprite.position.set(c.x+r*.7,c.y+r*.55,c.z+r*.4);l.sprite.material.rotation=-performance.now()/900;}l.sprite.visible=offen;const lab=labels.find(x=>x.obj===l.sprite);if(lab&&!offen)lab.el.hidden=true;}for(const orbit of orbits){if(orbit.userData.accent){const q=GalaxyModel.orbit(orbit.userData.position,clock);orbit.userData.accent.rotation.y=-Math.atan2(q.z,q.x);}}
   for(const {points,entries} of beaconGroups){const a=points.geometry.attributes.position;entries.forEach((l,i)=>{const p=l.obj.position;a.setXYZ(i,p.x,p.y,p.z);});a.needsUpdate=true;}const w=canvas.clientWidth,h=canvas.clientHeight,v=new THREE.Vector3();
   const lage=uiLageLesen(),hostRect=lage.hostRect,occupied=[],hart=[];
   // Weltenkrümmer und Hyperraumloch freihalten: Beschriftungen weichen ihnen aus (Karl: alles für sich klar erkennbar)
   for(const wk of weltenkruemmer)for(const [o,gr] of [[wk.inst.objekt,9],[wk.loch?.objekt,20]]){if(!o?.visible)continue;o.getWorldPosition(v);const tiefe=-v.clone().applyMatrix4(camera.matrixWorldInverse).z;if(tiefe<=0)continue;v.project(camera);
    const r=Math.max(18,gr*camera.projectionMatrix.elements[5]*h/(2*tiefe)),b={x:(v.x+1)*w/2-r,y:(1-v.y)*h/2-r,w:2*r,h:2*r};occupied.push(b);hart.push(b);}
   const details=stage!=='sector'||!sectorNumberView&&(distance<300||labels.length<=80&&distance<700);
   if(host.dataset.ownerDetail!==String(details)){host.dataset.ownerDetail=String(details);for(const l of labels)l.metrics=null;}
   if(sectorArcBatch)sectorArcBatch.arcs.visible=details;for(const star of sectorStars){const depth=-star.mesh.position.clone().applyMatrix4(camera.matrixWorldInverse).z;const minRadius=2.2*Math.max(1,depth)/(camera.projectionMatrix.elements[5]*h/2);star.mesh.scale.setScalar(Math.max(1,minRadius/star.mesh.geometry.parameters.radius));}if(himmel)himmel.position.copy(camera.position);if(nebelHimmel?.visible)nebelHimmel.position.copy(camera.position);if(stage==='system'){systemSchritt(delta,zeitU.value);kugelnWaehlen(h);}{const gq=gpuStart();zeichnen();gpuEnde(gq);}if(stage==='sector'){const hint=host.querySelector('.space-instructions');if(hint)hint.textContent=labels.length+' Systeme · '+(sectorNumberView?'Nummernansicht · 2 Finger verschieben':details?'Buchstaben = Besitzer · Stern antippen':'Für Besitzer und Details näher heranzoomen');}
   const labelBudget=stage==='galaxy'?Math.max(8,Math.floor(w*h/6000)):Infinity;let shown=0;
   const sichtOben=handy.matches?Math.max(2,2-hostRect.top):2,sichtUnten=handy.matches?Math.min(h-2,lage.leiste-hostRect.top-2):h-2;
   const ui=lage.ui;for(const b of ui)occupied.push({...b});
   // Planet discs are kept clear, including discs other than the label's own planet.
   for(const planet of planets){planet.getWorldPosition(v);const depth=-v.clone().applyMatrix4(camera.matrixWorldInverse).z;v.project(camera);if(depth>0&&Math.abs(v.x)<1.1&&Math.abs(v.y)<1.1){const radius=planet.geometry.parameters.radius*camera.projectionMatrix.elements[5]*h/(2*depth);occupied.push({x:(v.x+1)*w/2-radius,y:(1-v.y)*h/2-radius,w:radius*2,h:radius*2});}}
   // Measure new labels as one batch; moving the camera needs no per-label layout reads.
   const unmeasured=labels.filter(l=>!l.metrics);
   for(const l of unmeasured)l.el.hidden=false;
   for(const l of unmeasured)l.metrics={w:l.el.offsetWidth,h:l.el.offsetHeight};
   const focused=document.activeElement;
   // Hyperraumloch zuerst: es steht fest an seinem Wirbel, alle anderen weichen ihm aus (Überlappungsprüfung 25.10.2026: am Handy
   // lag „◎ Loch → [2:3]“ über der eigenen Planetenplakette, weil das Loch zuletzt und ohne Platzprüfung gesetzt wurde).
   const priority=l=>l.kind==='loch'?12:l.kind==='debris'?11:l.el===focused?10:l.el.classList.contains('is-highlighted')?9:l.friendly.includes('eigen')?8:l.friendly.includes('verbuendet')?7:selectedOwner&&l.owners.includes(selectedOwner)?6:l.el.dataset.relation&&l.el.dataset.relation!=='unbekannt'?1:0;
   for(const l of labels)l.prio=priority(l);
   const ordered=[...labels].sort((a,b)=>b.prio-a.prio||((Number(a.key)*73)%Math.max(1,labels.length))-((Number(b.key)*73)%Math.max(1,labels.length)));
   for(const l of ordered){
    (l.debrisPlanet||l.obj).getWorldPosition(v);let ox=l.kind==='system'?10:0,oy=l.kind==='system'?-7:0;let debrisRadius=0;if(l.debrisPlanet){const depth=-v.clone().applyMatrix4(camera.matrixWorldInverse).z;debrisRadius=l.debrisPlanet.geometry.parameters.radius*camera.projectionMatrix.elements[5]*h/(2*Math.max(.1,depth))+6;ox=debrisRadius+8;oy=-l.metrics.h/2;}
    if(l.kind==='planet'){const depth=-v.clone().applyMatrix4(camera.matrixWorldInverse).z;ox=l.obj.geometry.parameters.radius*camera.projectionMatrix.elements[5]*h/(2*Math.max(.1,depth))+8;oy=-l.metrics.h/2;}
    v.project(camera);let hide=Math.abs(v.x)>1||Math.abs(v.y)>1||v.z>1||v.z< -1||stage==='galaxy'&&l.kind==='system'&&l.prio<6||l.kind==='tor'&&distance>260;
    if(l.debrisHalo){l.debrisHalo.hidden=Math.abs(v.x)>1||Math.abs(v.y)>1||v.z>1||v.z< -1;l.debrisHalo.style.width=l.debrisHalo.style.height=(debrisRadius*2)+'px';l.debrisHalo.style.transform=`translate3d(${(v.x+1)*w/2-debrisRadius}px,${(1-v.y)*h/2-debrisRadius}px,0)`;}
    let x=(v.x+1)*w/2+ox-(l.kind==='sector'?l.metrics.w/2:0),y=(1-v.y)*h/2+oy-(l.kind==='sector'?l.metrics.h/2:0),box={x,y,...l.metrics};
    const imBild_=b=>!(b.x<2||b.y<sichtOben||b.x+b.w>w-2||b.y+b.h>sichtUnten),frei=b=>imBild_(b)&&!occupied.some(a=>b.x<a.x+a.w+3&&b.x+b.w+3>a.x&&b.y<a.y+a.h+3&&b.y+b.h+3>a.y);
    if(!hide&&l.kind==='loch'){occupied.push({...box});hart.push({...box,loch:true});shown++;}else if(!hide){
     // Planeten und Trümmerfelder: rechts belegt → links, unten, oben versuchen; die eigene Welt und Trümmerfelder
     // (anwählbar, Karl 19.10.) bleiben beschriftet, solange sie keine Bedienelemente verdecken.
     // Kein Platz für die volle Plakette → kompakt (Nummer + Zeichen), dann wieder alle vier Seiten probieren.
     let kompakt=false;
     if((l.kind==='planet'||l.kind==='debris')&&!frei(box)){const cx=(v.x+1)*w/2,cy=(1-v.y)*h/2,r=ox-8,seiten=M=>[{x:cx+r+8,y:cy-M.h/2},{x:cx-r-8-M.w,y:cy-M.h/2},{x:cx-M.w/2,y:cy+r+6},{x:cx-M.w/2,y:cy-r-6-M.h}];let gefunden=false;
      for(const b of seiten(l.metrics).slice(1)){const bb={...b,w:l.metrics.w,h:l.metrics.h};if(frei(bb)){box=bb;x=bb.x;y=bb.y;gefunden=true;break;}}
      if(!gefunden&&l.el.querySelector('.space-resident,.space-owner-codes,.space-debris-text')){if(!l.metricsK){l.el.classList.add('kompakt');l.metricsK={w:l.el.offsetWidth,h:l.el.offsetHeight};l.el.classList.remove('kompakt');}
       for(const b of seiten(l.metricsK)){const bb={...b,w:l.metricsK.w,h:l.metricsK.h};if(frei(bb)){box=bb;x=bb.x;y=bb.y;kompakt=true;break;}}}}
     const wichtig=l.kind==='debris'||l.kind==='planet'&&l.friendly.includes('eigen'),ueber=(b,a)=>b.x<a.x+a.w+3&&b.x+b.w+3>a.x&&b.y<a.y+a.h+3&&b.y+b.h+3>a.y,hartFrei=b=>!ui.some(a=>ueber(b,a))&&!hart.some(a=>ueber(b,a));
     if(wichtig&&!frei(box)){const cx=(v.x+1)*w/2,cy=(1-v.y)*h/2,r=ox-8;let wahl=null;
      for(const [M,k] of [[l.metrics,false],...(l.metricsK?[[l.metricsK,true]]:[])]){for(const b of [{x:cx+r+8,y:cy-M.h/2},{x:cx-r-8-M.w,y:cy-M.h/2},{x:cx-M.w/2,y:cy+r+6},{x:cx-M.w/2,y:cy-r-6-M.h}]){const bb={...b,w:M.w,h:M.h};if(imBild_(bb)&&hartFrei(bb)){wahl=[bb,k];break;}}if(wahl)break;}
      // letzter Rückfall: kompakt unter dem Planeten (der angreifende Weltenkrümmer steht immer darüber), nur Bedienelemente sperren
      if(!wahl&&l.metricsK){const M=l.metricsK,bb={x:cx-M.w/2,y:cy+r+6,w:M.w,h:M.h};if(imBild_(bb)&&!ui.some(a=>ueber(bb,a))&&!hart.some(a=>a.loch&&ueber(bb,a)))wahl=[bb,true];}
      if(wahl){box=wahl[0];x=box.x;y=box.y;kompakt=wahl[1];}}
     if(l.el.classList.contains('kompakt')!==kompakt)l.el.classList.toggle('kompakt',kompakt);
     hide=shown>=labelBudget||!(frei(box)||wichtig&&imBild_(box)&&(hartFrei(box)||kompakt&&!ui.some(a=>ueber(box,a))&&!hart.some(a=>a.loch&&ueber(box,a))));if(!hide){occupied.push({...box,x:box.x-5,y:box.y-5,w:box.w+10,h:box.h+10});shown++;}}
    if(l.el.hidden!==hide)l.el.hidden=hide;
    if(!hide)l.el.style.transform=`translate3d(${x.toFixed(1)}px,${y.toFixed(1)}px,0)`;
   }
   dirty=false;
  }
  inFrame=false;lief=laeuft;q.n++;q.js.push(performance.now()-js0);if(q.js.length>60)q.js.shift();
  // Solange sich etwas bewegt (Umläufe, Sonne, Flotten, Kameraflug): jedes Bild; sonst Ruhe bis zur nächsten Eingabe.
  if(laeuft||dirty)planen();else last=0;
 }
 function visible(value){
  const vorher=active;active=value;if(!value){if(vorher){closeDetail();panelZu();listeZu();}ruhe();return;}
  // Handy: erster Aufruf zeigt das Sonnensystem des aktiven Planeten, der Desktop beginnt bei der Galaxie.
  const key=world?.world_id+':'+view?.name;if(key!==stamp){stamp=key;cache.clear();ownerMarks.clear();epoch++;init();if(handy.matches){const [s,n]=heimat();void showSystem(s,n);}else showGalaxy();}
  if(!host)init();handyLayout();resize();neu();
 }
 function update(data){const systems=Array.isArray(data)?data:data.systeme||[];const s=Number($('map-form').elements.sektor.value);cache.delete(s);if(systems.length===1)void showSystem(s,systems[0].system);else void showSector(s);}
 document.addEventListener('click',e=>{const owner=e.target.closest('[data-space-owner]');if(owner){selectOwner(owner.dataset.spaceOwner);return;}const b=e.target.closest('[data-space-kind],[data-space-own]');if(!b)return;if(b.dataset.spaceOwn){selectedPlanet=b.dataset.spaceOwn;renderView();navigateGame('reich');}else pick(b.dataset.spaceKind,b.dataset.spaceKey);});
 document.addEventListener('visibilitychange',()=>{if(document.hidden)ruhe();else neu();});
 // Ohne laufend wachsende Alter (atlas[].mond.alter_sekunden): sonst baute jede Aktualisierung das System neu auf und
 // schloss Infokarte und Bergungsformular sofort wieder (Emulator 19.10.2026, Welt mit bekanntem Mond).
 function relations(){const next=JSON.stringify([view?.kommunikation?.kontakte||[],view?.atlas||[],view?.planeten?.map(p=>p.koord)||[],view?.truemmer||[]],(k,v)=>k==='alter_sekunden'?undefined:v);if(next===relationStamp)return;relationStamp=next;cache.clear();if(active){const saved={stage,yaw,pitch,distance,focus:{...focus},owner:selectedOwner,auto:autoFit},restore=()=>{if(saved.stage!==stage)return;if(saved.owner)selectOwner(saved.owner);/* unberührte Kamera: neue Einpassung behalten */if(saved.auto!=null)return;yaw=saved.yaw;pitch=saved.pitch;distance=saved.distance;Object.assign(focus,saved.focus);autoFit=null;cameraUpdate();};if(stage==='system')void showSystem(sector,system).then(restore);else if(stage==='sector')void showSector(sector).then(restore);else{showGalaxy();restore();}}}
 return {visible,update,relations,pick,
   /* Android-Zurück (Rahmen.zurueckHaken): Auswahl, Planetenfeld, dann eine Ebene hoch */zurueck:()=>!!(active&&renderer&&host?.isConnected&&!$g('karte')?.hidden&&hoch()),
   /* Diagnose fuer Tests: Kamera */_kamera:()=>({yaw,pitch,distance,autoFit,zeichnet:!!frameId,folge:folgeKennung,bauten,nachleuchten:!!bloom?.an,pixelStufe}),
   /* Diagnose fuer Tests: Planetenkugeln (Textur geladen, Ring) */_planeten:()=>planets.map(p=>{const img=p.material.map?.image;const v=p.getWorldPosition(new THREE.Vector3()).project(camera),c=canvas.getBoundingClientRect();return {pos:p.userData.data.position,x:Math.round(c.left+(v.x+1)*c.width/2),y:Math.round(c.top+(1-v.y)*c.height/2),r:p.geometry.parameters.radius,karte:img?.currentSrc||img?.src||img?.tagName||'',breite:img?.naturalWidth??img?.width??0,ring:p.children.some(c=>c.isSprite&&c.material.blending!==THREE.AdditiveBlending)};}),
   /* Diagnose fuer Tests: Last je Bild (Zeichenaufrufe, Dreiecke, Punkte), Software-Rendering, Bildgröße */_last:()=>({...renderer.info.render,software:softwareRender,px:renderer.getPixelRatio(),w:canvas.width,h:canvas.height,nachleuchten:!!bloom?.an}),
   /* Diagnose fuer Tests: Nebelhimmel (an, Abstand zur Kamera, Kamera zum Systemmittelpunkt) */_nebel:()=>({an:!!nebelHimmel?.visible,abstand:nebelHimmel?Math.round(nebelHimmel.position.distanceTo(camera.position)):null,kamera:Math.round(camera.position.length()),stufe:stage,sonne:(()=>{const v=new THREE.Vector3().project(camera),c=canvas.getBoundingClientRect();return {x:Math.round(c.left+(v.x+1)*c.width/2),y:Math.round(c.top+(1-v.y)*c.height/2)};})()}),
   /* Filmaufnahme (laeufe/galaxie-3d/film.py): eigener Bildtakt, Kamera direkt, ein Bild auf Abruf */
   /* Diagnose fuer Tests: Szene und Renderer (Leistungsmessung) */_szene:()=>({scene,renderer,camera}),
   /* Diagnose fuer Tests: Qualitätsleiter (Stufe, Schärfe, Nachleuchten, Ruhetakt, Median der Skriptzeit je Bild) */_leistung:()=>({stufe:pixelStufe,px:renderer?.getPixelRatio(),nachleuchten:!!bloom?.an,msaa:bloom?.msaa,sparen:sparModus,bilder:q.n,gpu_ms:gpuZeit.werte.length?Math.round(median(gpuZeit.werte)*10)/10:null,gpu_messung:!!gpuZeit.ext,js_ms:q.js.length?Math.round([...q.js].sort((a,b)=>a-b)[q.js.length>>1]*10)/10:null,gesperrt:[...q.sperre].filter(([,t])=>t>performance.now()).map(([s])=>s)}),
   _film:{an:()=>{filmModus=true;ruhe();},aus:()=>{filmModus=false;neu();},
    kamera:k=>{if(k.focus)Object.assign(focus,k.focus);if(k.yaw!=null)yaw=k.yaw;if(k.pitch!=null)pitch=k.pitch;if(k.distance!=null)distance=k.distance;folge=null;folgeKennung='';autoFit=null;schwung.yaw=schwung.pitch=0;Object.assign(zielKam,{x:focus.x,y:focus.y,z:focus.z,yaw,pitch,distance});fliegt=false;setzeKamera();},
    ziel:k=>{const o=loeseKennung(k);if(!o)return null;const v=o.getWorldPosition(new THREE.Vector3());return {x:v.x,y:v.y,z:v.z,r:zielGroesse(o)};},
    wk:()=>weltenkruemmer.map(w=>({art:w.art,id:w.f?.flotte??null,pos:w.inst.objekt.position.toArray(),blick:w.blick?w.blick.position.toArray():null,gemeldet:!!w.gemeldet,ring:!!w.ring,rigT:w.rigT??null,lage:w.lage?{E:w.lage.E.toArray(),S:w.lage.S.toArray(),W:w.lage.W.toArray(),a:w.lage.a.toArray(),aus:w.lage.aus.toArray()}:null})),
    bild:()=>{dirty=true;clearTimeout(frameId);frameId=0;inFrame=false;frame(performance.now());},
    neuaufbau:()=>showSystem(sector,system),labels:an=>{$g('galaxy-labels').hidden=!an;},feld:(art,key)=>{if(art)pick(art,key);else panelZu();}},
   abbauChip:(koord,tf,opt)=>abbauChip(koord,tf,opt),vernichtenChip:(p,opt)=>vernichtenChip(p,opt),eignungKurz:p=>eignungKurz(p),
   einstieg:()=>{if(!active||!handy.matches||!host||$g('karte')?.hidden)return;handyLayout();resize();if(scrollZiel>0)scrollTo({top:scrollZiel,behavior:'instant'});},
   /* Tabelle → 3D: dasselbe System zeigen */zeige:(s,n)=>{if(!host)init();if(stage!=='system'||sector!==Number(s)||system!==Number(n))void showSystem(s,n);},
   snapshot:()=>({stage,sektor:Number(sector),system:Number(system),...(autoFit==null?{camera:{yaw,pitch,distance,focus:{...focus}}}:{})}),
   /* Diagnose fuer Tests: Weltenkruemmer und Zielmonde im System */_weltenkruemmer:()=>({wk:weltenkruemmer.map(w=>({art:w.art,pos:w.inst.objekt.position.toArray(),mond:w.mond?w.mond.objekt.position.toArray():null,mondSichtbar:!!w.mond?.objekt.visible})),planeten:planets.map(p=>p.userData.data.position),monde:moons.map(m=>m.planet.userData.data.position)}),restore:async p=>{if(!host)init();if(!host)return;/* 3D-Atlas noch nicht aufgebaut (Handy, Tabellenansicht) */if(p?.stage==='system')await showSystem(p.sektor,p.system);else if(p?.stage==='sector')await showSector(p.sektor);else showGalaxy();const c=p?.camera;if(c&&[c.yaw,c.pitch,c.distance,c.focus?.x,c.focus?.y,c.focus?.z].every(Number.isFinite)){yaw=c.yaw;pitch=Math.max(-1.54,Math.min(1.54,c.pitch));distance=Math.max(24,Math.min(2400,c.distance));Object.assign(focus,c.focus);autoFit=null;cameraUpdate();}}
 };
})();
// Handy: beim Öffnen der Galaxie die Kontozeile wegscrollen, damit die Karte bis zur unteren Leiste reicht.
document.addEventListener('game-navigation',()=>setTimeout(()=>window.GalaxyUI?.einstieg?.(),60));
// Android-Zurück: in der 3D-Ansicht erst Auswahl/Planetenfeld schließen, dann eine Ebene hoch (System → Sektor → Galaxie).
// Seitenzustand (Auswahl, Zoom): erst nach Blättern, Karten und Dialogen (Rahmen.zurueck).
document.addEventListener('DOMContentLoaded',()=>{const h=()=>window.GalaxyUI?.zurueck?.();h.seite=true;window.Rahmen?.zurueckHaken?.push(h);});
