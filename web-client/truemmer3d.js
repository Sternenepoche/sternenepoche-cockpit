// Trümmer-Bausatz im 3D-Sonnensystem (Karl, 19.10.2026: „realistische Trümmerringe um einen Planeten – Steine wie im
// Asteroidengürtel, aber auch Wrackteile von Schiffen; echt gute 3D-Objekte, im Handyspiel AAA mit WOW-Effekt“).
// Modelle: assets/truemmer.bausatz.glb aus tools/content/truemmer_bau.py (Blender): 6 Gesteinsarten in drei Detailstufen,
// 14 Wrackteile mit Lack (je Instanz in Volksfarbe), Stahl, Glutkanten, Fenstern. galaxy.js ruft:
//   felsRing(opt)              → Gruppe für Gürtel, Kuiperring und Trümmerfeld (sonst eigene Ikosaeder als Rückfall)
//   truemmerFeld({gruppe,…})   → Trümmerring um den Planeten: Teilemischung nach Rohstoffen, Staubband, Glitzern
//   schritt(dt,t)              → Shaderzeit
// Handy-Technik: je Teil und Material ein InstancedMesh (wenige Zeichenaufrufe); Taumeln, Glutflackern, Blinklichter
// und Kristallschimmer laufen im Vertex-/Fragment-Shader (keine Matrix-Updates je Bild); Detailstufe nach Größe;
// Antippen über eine unsichtbare Grobhülle je Brocken statt Strahltest gegen zehntausende Dreiecke.
window.Truemmer3D=(()=>{
 const skriptBasis=document.currentScript?.src||document.baseURI;
 const GLB=new URL('assets/truemmer.bausatz.glb',skriptBasis).href;
 const zeitU={value:0};
 let teile=null,ladung=null,umgebung=null,fehlt=false;
 const gruppen=new Set();
 const zufall=s=>()=>(s=(s*16807)%2147483647)/2147483647;

 // ---------------------------------------------------------------- GLB lesen (Knoten → Primitive mit Material)
 async function lesen(){
  const buf=await fetch(GLB).then(r=>{if(!r.ok)throw Error(GLB+' '+r.status);return r.arrayBuffer();});
  const dv=new DataView(buf);if(dv.getUint32(0,true)!==0x46546C67)throw Error('kein GLB');
  let off=12,json=null,bin=null;while(off<buf.byteLength){const len=dv.getUint32(off,true),typ=dv.getUint32(off+4,true);if(typ===0x4E4F534A)json=JSON.parse(new TextDecoder().decode(new Uint8Array(buf,off+8,len)));else if(typ===0x004E4942)bin=buf.slice(off+8,off+8+len);off+=8+len;}
  const acc=i=>{const a=json.accessors[i],v=json.bufferViews[a.bufferView],n={SCALAR:1,VEC2:2,VEC3:3,VEC4:4}[a.type],T={5126:Float32Array,5125:Uint32Array,5123:Uint16Array,5121:Uint8Array}[a.componentType];return new THREE.BufferAttribute(new T(bin,(v.byteOffset||0)+(a.byteOffset||0),a.count*n),n,!!a.normalized);};
  const texCache=new Map();const tex=(i,srgb)=>{if(i==null)return null;if(!texCache.has(i))texCache.set(i,(async()=>{const tx=json.textures[i],im=json.images[tx.source??tx.extensions?.EXT_texture_webp?.source],v=json.bufferViews[im.bufferView];const bmp=await createImageBitmap(new Blob([new Uint8Array(bin,v.byteOffset||0,v.byteLength)],{type:im.mimeType}));const t=new THREE.Texture(bmp);t.flipY=false;t.wrapS=t.wrapT=THREE.RepeatWrapping;t.anisotropy=4;if(srgb)t.colorSpace=THREE.SRGBColorSpace;t.needsUpdate=true;return t;})());return texCache.get(i);};
  const mats=await Promise.all(json.materials.map(async m=>{const pbr=m.pbrMetallicRoughness||{},f=pbr.baseColorFactor||[1,1,1,1],em=m.emissiveFactor||[0,0,0],st=m.extensions?.KHR_materials_emissive_strength?.emissiveStrength??1;
   const [map,orm,normal]=await Promise.all([tex(pbr.baseColorTexture?.index,true),tex(pbr.metallicRoughnessTexture?.index,false),tex(m.normalTexture?.index,false)]);
   return material(m.name,{map,orm,normal,farbe:new THREE.Color(f[0],f[1],f[2]),rau:pbr.roughnessFactor??.6,met:pbr.metallicFactor??0,emissiv:new THREE.Color(em[0],em[1],em[2]),staerke:st});}));
  const aus=new Map();
  for(const n of json.nodes){if(n.mesh==null)continue;const prims=json.meshes[n.mesh].primitives.map(pr=>{const g=new THREE.BufferGeometry();g.setAttribute('position',acc(pr.attributes.POSITION));g.setAttribute('normal',acc(pr.attributes.NORMAL));if(pr.attributes.TEXCOORD_0!=null)g.setAttribute('uv',acc(pr.attributes.TEXCOORD_0));if(pr.attributes.COLOR_0!=null)g.setAttribute('color',acc(pr.attributes.COLOR_0));if(pr.indices!=null)g.setIndex(acc(pr.indices));g.computeBoundingSphere();return {g,mat:mats[pr.material]};});
   aus.set(n.name,prims);}
  return aus;
 }

 // ---------------------------------------------------------------- Materialien
 // Leichte Umgebung für Metallglanz (ohne Renderer-Zugriff: Equirect-Leinwand, three filtert sie selbst vor).
 function umgebungsbild(){if(umgebung)return umgebung;const c=document.createElement('canvas');c.width=256;c.height=128;const x=c.getContext('2d');
  const v=x.createLinearGradient(0,0,0,128);v.addColorStop(0,'#2a3446');v.addColorStop(.42,'#141a26');v.addColorStop(.5,'#2b2620');v.addColorStop(.58,'#0d1018');v.addColorStop(1,'#05070b');x.fillStyle=v;x.fillRect(0,0,256,128);
  for(const [px,py,r,f] of [[64,44,38,'rgba(255,214,160,.55)'],[190,70,52,'rgba(90,130,200,.28)']]){const g=x.createRadialGradient(px,py,0,px,py,r);g.addColorStop(0,f);g.addColorStop(1,'rgba(0,0,0,0)');x.fillStyle=g;x.fillRect(0,0,256,128);}
  umgebung=new THREE.CanvasTexture(c);umgebung.mapping=THREE.EquirectangularReflectionMapping;umgebung.colorSpace=THREE.SRGBColorSpace;return umgebung;}
 // Gemeinsame Shader-Erweiterung: Taumeln je Instanz (Achse+Tempo im Attribut dreh), Leuchten je Instanz.
 const KOPF_V='attribute vec4 dreh;uniform float uZeit;varying float vPhase;vec3 drehQ(vec3 v,vec4 q){return v+2.*cross(q.xyz,cross(q.xyz,v)+q.w*v);}\n';
 function material(name,{map,orm,normal,farbe,rau,met,emissiv,staerke}){
  const leucht=name==='glut'||name==='leuchten'||name==='kristall';
  const m=new THREE.MeshStandardMaterial({name,map,color:map?0xffffff:farbe,roughnessMap:orm,metalnessMap:orm,normalMap:normal,roughness:orm?1:rau,metalness:orm?1:met,vertexColors:true,envMap:umgebungsbild(),envMapIntensity:name==='lack'||name==='stahl'||name==='erz'?.85:.45,
   emissive:emissiv,emissiveIntensity:leucht?staerke*(name==='kristall'?2.6:1):0});
  if(normal)m.normalScale.set(1,-1);
  if(name==='kristall'){m.transparent=false;m.roughness=.15;}
  m.onBeforeCompile=sh=>{sh.uniforms.uZeit=zeitU;
   sh.vertexShader=KOPF_V+sh.vertexShader
    .replace('#include <beginnormal_vertex>','#include <beginnormal_vertex>\nfloat dw=uZeit*dreh.w+float(gl_InstanceID)*2.399;vec4 dq=vec4(normalize(dreh.xyz+vec3(1e-4))*sin(dw*.5),cos(dw*.5));objectNormal=drehQ(objectNormal,dq);vPhase=fract(float(gl_InstanceID)*.6180339);')
    .replace('#include <begin_vertex>','#include <begin_vertex>\ntransformed=drehQ(transformed,dq);');
   let fl='1.';
   if(name==='glut')fl='(.72+.28*sin(uZeit*2.7+vPhase*41.)*sin(uZeit*6.1+vPhase*17.))';
   else if(name==='leuchten')fl='mix(.85+.15*sin(uZeit*9.+vPhase*60.),step(fract(uZeit*.55+vPhase*3.),.09)*3.,step(.72,fract(vPhase*7.31)))';
   else if(name==='kristall')fl='(.62+.38*pow(.5+.5*sin(uZeit*1.1+vPhase*25.),2.))';
   sh.fragmentShader='uniform float uZeit;varying float vPhase;\n'+sh.fragmentShader.replace('#include <emissivemap_fragment>',`#include <emissivemap_fragment>\n${leucht?`totalEmissiveRadiance*=vColor.rgb*${fl};`:''}`);};
  m.customProgramCacheKey=()=>'truemmer3d-'+name;
  return m;}

 function bereit(){if(!ladung)ladung=lesen().then(t=>{teile=t;for(const g of gruppen)fuellen(g);return t;}).catch(e=>{fehlt=true;console.info('Trümmer-Bausatz nicht geladen, Rückfall auf einfache Felsen (ab dem nächsten Aufbau):',e.message);return null;});return ladung;}

 // ---------------------------------------------------------------- Auswahl der Teile
 const ARTEN={
  guertel:{fels:.42,kohle:.2,erz:.17,kristall:.07,splitter:.14},
  kuiper:{eis:.62,kohle:.24,fels:.14},
 };
 const WRACK={held:['rumpf','bug','triebwerk','reaktorring'],mittel:['turm','traeger','container','solar','schuessel','rohre'],klein:['kapsel','platte','splitter','spant','platte','splitter']};
 // Lack je Volk (Rumpffarben aus weltenkruemmer.js, für Wracks dunkler und gesättigter: verrußt, nicht gipsweiß)
 const LACK={aurelianer:'#b39a6a',krath:'#6a2a22',syntheten:'#6f8099',veyari:'#7f9a86',neutral:'#7a8188'};
 const LICHT={aurelianer:'#ffd36a',krath:'#ff5a3a',syntheten:'#7cc8ff',veyari:'#7dff96',neutral:'#ffd9a0'};
 const waehle=(gew,z)=>{let s=0;const sum=Object.values(gew).reduce((a,b)=>a+b,0),x=z()*sum;for(const [k,w] of Object.entries(gew)){s+=w;if(x<=s)return k;}return Object.keys(gew)[0];};

 // Lage: Positionen, Größen, Drehachsen – sofort (für das Antippen), unabhängig vom Laden der Modelle.
 function lage(opt){
  const {n,r0,r1,h,groesse,saat,kind}=opt,z=zufall((saat*7919+13)%2147483647||7),L=[];
  const ring=kind==='debris';
  // opt.bogen (rad, Mitte bei Winkel 0 in x/z): Kampftrümmer als dicke Wrackwolke auf einem Bahnbogen (galaxy.js
  // form 'feld'); ohne bogen voller Ring (Mondtrümmer). Dichte fällt zu den Bogenenden ab.
  const bogen=ring&&opt.bogen>0?Math.min(opt.bogen,Math.PI*2):0,winkel=()=>bogen?(z()-.5)*bogen*(.65+.35*z()):z()*Math.PI*2;
  // Trümmerring: Dichtebänder mit Lücken und zwei dichtere Trümmerströme entlang der Bahn (frische Schlacht)
  const baender=ring?Array.from({length:4},()=>({m:z(),b:.04+z()*.09,w:.4+z()})):null,strom=bogen?[(z()-.5)*bogen*.4,(z()-.5)*bogen*.4]:[z()*6.283,z()*6.283];
  for(let i=0;i<n;i++){
   let r,a=winkel();
   if(ring){const b=baender[Math.floor(z()*baender.length)];const u=Math.min(1,Math.max(0,b.m+(z()+z()+z()-1.5)*b.b));r=r0+(r1-r0)*u;if(z()<.35){const s=strom[i%2];a=s+(z()-.5)*1.1;}}
   else r=r0+(r1-r0)*Math.pow(z(),.8);
   const flach=ring?(bogen?.45:.14):1,y=(z()+z()-1)*h*.5*flach*(1-.5*Math.abs((r-(r0+r1)/2)/(r1-r0+1e-6)));
   const gr=groesse[0]+(groesse[1]-groesse[0])*Math.pow(z(),6);
   L.push({p:new THREE.Vector3(Math.cos(a)*r,y,Math.sin(a)*r),gr,q:new THREE.Quaternion().setFromEuler(new THREE.Euler(z()*6.28,z()*6.28,z()*6.28)),achse:new THREE.Vector3(z()-.5,z()-.5,z()-.5),tempo:0,z:z()});}
  // Detailstufe nach Größe: die größten ~2,5 % voll, die nächsten ~22 % mittel, der Rest leicht
  const sortiert=[...L].sort((a,b)=>b.gr-a.gr),nh=Math.max(1,Math.round(n*(kind==='kuiper'?.01:.02))),nk=Math.round(n*(kind==='kuiper'?.12:.18));
  sortiert.forEach((e,i)=>{e.stufe=i<nh?'held':i<nh+nk?'klein':'mini';e.tempo=(e.stufe==='held'?.04+e.z*.12:e.stufe==='klein'?.12+e.z*.4:.25+e.z*.8)*(e.z<.5?1:-1);});
  if(ring){
   for(const e of L)e.gr*=1.35;
   // Füllung: viele Kleinstbrocken und Splitter (je ~50 Dreiecke) in denselben Bändern – dichter Ring für wenig Last
   for(let i=0,m=Math.round(n*1.6);i<m;i++){const b=baender[Math.floor(z()*baender.length)],u=Math.min(1,Math.max(0,b.m+(z()+z()+z()-1.5)*b.b*1.2)),r=r0+(r1-r0)*u,a=z()<.3?strom[i%2]+(z()-.5)*(bogen?bogen*.35:1.3):winkel();
    L.push({p:new THREE.Vector3(Math.cos(a)*r,(z()+z()-1)*h*(bogen?.2:.06),Math.sin(a)*r),gr:groesse[0]*(.5+z()*1.1),q:new THREE.Quaternion().setFromEuler(new THREE.Euler(z()*6.28,z()*6.28,z()*6.28)),achse:new THREE.Vector3(z()-.5,z()-.5,z()-.5),tempo:(.3+z()*.9)*(z()<.5?1:-1),z:z(),stufe:'mini',fuell:true});}
   // große Wrackstücke: wenige, deutlich größer, nahe der Ringebene
   sortiert.slice(0,Math.min(5,1+Math.floor(n/80))).forEach((e,i)=>{e.gr=groesse[1]*(1.45+z()*.6);e.p.y*=.3;e.held=true;e.tempo=(.015+z()*.03)*(i%2?1:-1);});}
  return L;
 }

 // Unsichtbare Grobhülle je Brocken: Ziel für Antippen/Infofeld (galaxy.js sammelt InstancedMesh-Kinder der Gruppe)
 const huelleGeo=new THREE.IcosahedronGeometry(1.15,0),huelleMat=new THREE.MeshBasicMaterial({visible:false});
 const m4=new THREE.Matrix4(),eins=new THREE.Vector3(1,1,1),sk=new THREE.Vector3();
 function felsRing(opt){
  if(fehlt||!window.THREE||!opt||!['guertel','kuiper','debris'].includes(opt.kind))return null;
  void bereit();
  const g=new THREE.Group(),L=lage(opt),huelle=new THREE.InstancedMesh(huelleGeo,huelleMat,L.length);
  L.forEach((e,i)=>{m4.compose(e.p,e.q,sk.setScalar(e.gr));huelle.setMatrixAt(i,m4);});huelle.userData={kind:opt.kind,key:opt.key};huelle.computeBoundingSphere?.();g.add(huelle);
  const bild=new THREE.Group();bild.name='truemmer3d';g.add(bild);
  g.userData.taumel=[];g.userData.t3={opt,L,bild,inhalt:null};gruppen.add(g);
  if(opt.kind!=='debris')fuellen(g);/* Trümmerring füllt truemmerFeld() mit den Rohstoffangaben */
  return g;}

 // Teilemischung je Brocken festlegen und als Instanzen zeichnen.
 function fuellen(g){
  const t3=g.userData.t3;if(!t3||!teile||t3.gezeichnet)return;if(t3.opt.kind==='debris'&&!t3.inhalt)return;
  t3.gezeichnet=true;const {opt,L,bild}=t3,z=zufall((opt.saat*104729+31)%2147483647||11),eimer=new Map(),c=new THREE.Color();
  const volker=t3.inhalt?.volker||['neutral'];
  for(const e of L){let name,volk=null;
   if(e.fuell)name=`fels_${['splitter','fels','kohle','splitter','erz'][Math.floor(z()*5)]}_mini`;
   else if(opt.kind==='debris'){const istFels=!e.held&&z()<t3.inhalt.fels;
    if(istFels)name=`fels_${waehle(t3.inhalt.arten,z)}_${e.stufe==='held'?'klein':e.stufe}`;
    else{const k=e.held?'held':e.gr>opt.groesse[1]*.62?'mittel':'klein';name='wrack_'+WRACK[k][Math.floor(z()*WRACK[k].length)];volk=volker[Math.floor(z()*volker.length)];}}
   else name=`fels_${waehle(ARTEN[opt.kind],z)}_${e.stufe}`;
   if(!teile.has(name))continue;
   if(!eimer.has(name))eimer.set(name,[]);eimer.get(name).push({e,volk,hitze:z()<(t3.inhalt?.hitze??.7)?.55+z()*.6:.06,ton:.82+z()*.3,lz:z()});}
  // Detailstufen nach Kameraabstand (Karl 22.10.2026: „auf dem Handy super laggy“): Gürtel, Kuiperring und Trümmerfelder
  // hatten im Überblick ~700.000 Dreiecke je Bild, obwohl ein Brocken dort nur wenige Pixel groß ist. Große Brocken (held,
  // ~3.300 Dreiecke) und mittlere (klein, ~300) bekommen eine grobe Zweitfassung aus der nächstkleineren Stufe mit
  // denselben Instanzen; detail() zeigt die feine erst, wenn die Kamera dem Ring nahekommt. Aus der Nähe unverändert.
  const GROB={held:['klein',45],klein:['mini',110]};
  for(const [name,liste] of eimer){
   const st=name.startsWith('fels_')&&/_(held|klein)$/.exec(name)?.[1],grob=st&&name.replace(/_(held|klein)$/,'_'+GROB[st][0]);
   zeichnen(name,liste,st&&teile.has(grob)?{art:'fein',weite:GROB[st][1]}:null);
   if(st&&teile.has(grob))zeichnen(grob,liste,{art:'grob',weite:GROB[st][1]});}
  t3.lodNah=null;
  function zeichnen(name,liste,lod){
   for(const {g:geo,mat} of teile.get(name)){
    const n=liste.length,ig=new THREE.BufferGeometry();for(const [k,a] of Object.entries(geo.attributes))ig.setAttribute(k,a);ig.setIndex(geo.index);ig.boundingSphere=geo.boundingSphere;
    const dreh=new Float32Array(n*4),im=new THREE.InstancedMesh(ig,mat,n);
    liste.forEach(({e,volk,hitze,ton,lz},i)=>{m4.compose(e.p,e.q,sk.setScalar(e.gr));im.setMatrixAt(i,m4);const a=e.achse.lengthSq()>1e-6?e.achse.clone().normalize():eins;dreh.set([a.x,a.y,a.z,e.tempo],i*4);
     if(mat.name==='lack')c.set(LACK[volk]||LACK.neutral).multiplyScalar(.55+lz*.5);/* je Wrack unterschiedlich verbrannt */
     else if(mat.name==='glut')c.setScalar(hitze);
     else if(mat.name==='leuchten')c.set(LICHT[volk]||LICHT.neutral).multiplyScalar(lz<.25?.05:1);/* manche Wracks ohne Strom */
     else if(mat.name==='kristall')c.setScalar(.75+lz*.5);
     else if(opt.farbe&&name.startsWith('fels_')){c.set(opt.farbe[Math.floor(lz*opt.farbe.length)]).lerp(new THREE.Color(1,1,1),.55).multiplyScalar(ton*1.1);}
     else c.setScalar(ton);
     im.setColorAt(i,c);});
    ig.setAttribute('dreh',new THREE.InstancedBufferAttribute(dreh,4));
    im.instanceMatrix.needsUpdate=true;if(im.instanceColor)im.instanceColor.needsUpdate=true;
    // Grenzkugel um alle Instanzen (Taumeln ändert sie nicht wesentlich)
    im.computeBoundingSphere();im.frustumCulled=true;if(lod){im.userData.lod=lod;im.visible=window.__g3OhneLod?lod.art==='fein':lod.art==='grob';}bild.add(im);}}
 }
 // Je Ring: Abstand der Kamera zum Ringband (in Ringebene und Höhe); feine Brocken nur in der Nähe, sonst die grobe Fassung.
 const lokal=new THREE.Vector3();
 function detail(kamera){
  if(!kamera)return;
  for(const g of gruppen){const t3=g.userData.t3;if(!t3.gezeichnet||!g.parent)continue;
   g.worldToLocal(lokal.copy(kamera));const rho=Math.hypot(lokal.x,lokal.z),d=Math.hypot(rho-Math.min(t3.opt.r1,Math.max(t3.opt.r0,rho)),lokal.y)*(window.__g3OhneLod?0:1);/* Testschalter für Vorher-Messungen */
   const key=d<45?2:d<110?1:0;if(key===t3.lodNah)continue;t3.lodNah=key;
   for(const im of t3.bild.children){const l=im.userData.lod;if(!l)continue;const nah=d<l.weite;im.visible=l.art==='fein'?nah:!nah;}}}

 // ---------------------------------------------------------------- Trümmerring um den Planeten
 function truemmerFeld({gruppe,planet,radius,daten,truemmer}){
  const t3=gruppe?.userData?.t3;if(!t3)return;
  const erz=Number(truemmer?.erz)||0,kri=Number(truemmer?.kristall)||0,mondTruemmer=truemmer?.ring??!truemmer?.koord;
  const z=zufall(((daten?.position||1)*48271+7)%2147483647);
  const volker=Object.keys(LACK).filter(v=>v!=='neutral');
  const eigen=typeof view!=='undefined'?view?.volk:null,a=eigen&&LACK[eigen]?eigen:volker[Math.floor(z()*4)],b=volker[Math.floor(z()*4)];
  // Mond zerstört → fast nur Gestein (Erz/Kristall des Mondes); Schlachtfeld → überwiegend Schiffswracks
  t3.inhalt={fels:mondTruemmer?.9:.28,arten:{fels:.35,erz:.2+.6*erz/Math.max(1,erz+kri),kristall:.1+.5*kri/Math.max(1,erz+kri),splitter:.15,kohle:.1},volker:[a,a,b,'neutral'],hitze:mondTruemmer?.25:.75};
  fuellen(gruppe);
  // galaxy.js gibt dem Feld eine dicke Wolke warmer Staubpunkte mit (für die alten Felsen): auf die Ringebene drücken
  // Bei einem Bogenfeld liegen diese Punkte als voller Ring daneben: dann ausblenden (eigenes Glitzern/Staub im Bogen)
  for(const o of gruppe.children)if(o.isPoints&&!o.userData.t3&&t3.opt.bogen>0){o.visible=false;o.userData.t3=true;}
  for(const o of gruppe.children)if(o.isPoints&&!o.userData.t3){const a=o.geometry.attributes.position;for(let i=0;i<a.count;i++)a.setY(i,a.getY(i)*.16);a.needsUpdate=true;o.userData.t3=true;}
  const {opt}=t3;const bogen=opt.bogen>0?Math.min(opt.bogen,Math.PI*2):0;
  gruppe.add(staubband(opt.r0*.92,opt.r1*1.06,radius,opt.saat,mondTruemmer?'#b7a58c':'#a9a39b',mondTruemmer?9:0,bogen));
  gruppe.add(glitzern(Math.round(Math.min(420,80+t3.L.length*.8)),opt.r0,opt.r1,opt.h*(bogen?.35:.12),opt.saat,bogen));
 }
 // Staubband: Dichtebänder, Vorwärtsstreuung (gegen die Sonne hell aufleuchtend), Planetenschatten.
 function staubband(r0,r1,pr,saat,farbe,einblenden=0,bogen=0){
  const geo=new THREE.RingGeometry(r0,r1,192,6);geo.rotateX(-Math.PI/2);
  const mat=new THREE.ShaderMaterial({transparent:true,depthWrite:false,side:THREE.DoubleSide,
   uniforms:{uZeit:zeitU,uSaat:{value:(saat%97)*.37},uR0:{value:r0},uR1:{value:r1},uFarbe:{value:new THREE.Color(farbe)},uSonne:{value:new THREE.Vector3()},uPlanet:{value:new THREE.Vector3()},uPR:{value:pr},uEin:{value:einblenden?0:1},uBogen:{value:bogen}},
   vertexShader:'varying vec3 vW,vL;void main(){vL=position;vec4 w=modelMatrix*vec4(position,1.);vW=w.xyz;gl_Position=projectionMatrix*viewMatrix*w;}',
   fragmentShader:`uniform float uZeit,uSaat,uR0,uR1,uPR,uEin,uBogen;uniform vec3 uFarbe,uSonne,uPlanet;varying vec3 vW,vL;
float h1(float x){return fract(sin(x*127.1+uSaat)*43758.5);}float n1(float x){float i=floor(x),f=fract(x);f=f*f*(3.-2.*f);return mix(h1(i),h1(i+1.),f);}
void main(){float r=length(vL.xz),x=(r-uR0)/(uR1-uR0);float a=atan(vL.z,vL.x);
 float d=smoothstep(0.,.1,x)*smoothstep(1.,.82,x)*(.25+.75*n1(x*14.))*(.55+.45*n1(x*61.+3.))*(.8+.2*n1(a*6.+x*9.));
 d*=smoothstep(.18,.45,n1(x*5.+7.))*.7+.3;
 if(uBogen>0.)d*=smoothstep(uBogen*.5,uBogen*.22,abs(a))*(.85+.3*n1(a*9.+x*4.));
 vec3 V=normalize(cameraPosition-vW),L=normalize(uSonne-vW);float mu=dot(V,-L),g=.62;
 float hg=(1.-g*g)/pow(1.+g*g-2.*g*mu,1.5);float licht=.45+.22*hg;
 vec3 P=uPlanet-vW;float t=dot(P,L),q=dot(P,P)-t*t;licht*=t>0.?mix(.1,1.,smoothstep(uPR*uPR*.8,uPR*uPR*1.15,q)):1.;
 gl_FragColor=vec4(uFarbe*licht*1.4,clamp(d*.5*(.75+.25*hg),0.,.85)*uEin);}`});
  const m=new THREE.Mesh(geo,mat);m.renderOrder=1;m.userData.t3band=true;
  const t0=performance.now();m.onBeforeRender=()=>{if(einblenden)mat.uniforms.uEin.value=Math.min(1,(performance.now()-t0)/1000/einblenden);let o=m;while(o.parent&&!o.parent.isScene)o=o.parent;o.getWorldPosition(mat.uniforms.uSonne.value);m.parent?.getWorldPosition(mat.uniforms.uPlanet.value);};
  return m;}
 // Glitzern: Sonnenlicht blitzt kurz auf taumelndem Metall auf.
 function glitzern(n,r0,r1,h,saat,bogen=0){const z=zufall((saat*16807+5)%2147483647||5),pos=new Float32Array(n*3),ph=new Float32Array(n);
  for(let i=0;i<n;i++){const a=bogen?(z()-.5)*bogen*(.65+.35*z()):z()*6.2832,r=r0+(r1-r0)*z();pos.set([Math.cos(a)*r,(z()-.5)*h,Math.sin(a)*r],i*3);ph[i]=z()*100;}
  const g=new THREE.BufferGeometry();g.setAttribute('position',new THREE.BufferAttribute(pos,3));g.setAttribute('ph',new THREE.BufferAttribute(ph,1));
  const m=new THREE.Points(g,new THREE.ShaderMaterial({transparent:true,depthWrite:false,blending:THREE.AdditiveBlending,uniforms:{uZeit:zeitU,px:{value:Math.min(2,devicePixelRatio||1)}},
   vertexShader:'attribute float ph;uniform float uZeit,px;varying float vK;void main(){vK=pow(max(0.,sin(uZeit*(.7+fract(ph)*1.3)+ph)),28.);vec4 mv=modelViewMatrix*vec4(position,1.);gl_Position=projectionMatrix*mv;gl_PointSize=clamp(px*260./-mv.z,1.,10.)*(.4+vK);}',
   fragmentShader:'varying float vK;void main(){vec2 q=gl_PointCoord-.5;float r=length(q);float k=smoothstep(.5,0.,r)+smoothstep(.07,0.,abs(q.x))*smoothstep(.5,0.,abs(q.y))*.6+smoothstep(.07,0.,abs(q.y))*smoothstep(.5,0.,abs(q.x))*.6;gl_FragColor=vec4(vec3(1.,.95,.85)*k*vK*2.2,k*vK);}'}));
  m.frustumCulled=false;return m;}

 function schritt(dt){zeitU.value+=Math.min(.1,dt||0);
  for(const g of gruppen){let o=g;while(o.parent)o=o.parent;if(!o.isScene){if(!g.userData.t3.weg)g.userData.t3.weg=performance.now();else if(performance.now()-g.userData.t3.weg>4000){gruppen.delete(g);g.userData.t3.bild.traverse(x=>{if(x.isInstancedMesh){x.geometry.dispose();x.dispose?.();}});}}else g.userData.t3.weg=0;}}

 // Laden früh anstoßen, damit das erste Sonnensystem schon die echten Modelle zeigt.
 if(document.readyState==='loading')document.addEventListener('DOMContentLoaded',()=>{if(window.THREE)void bereit();});else if(window.THREE)void bereit();
 return {felsRing,truemmerFeld,schritt,detail,bereit,_teile:()=>teile,_gruppen:()=>[...gruppen].map(g=>({kind:g.userData.t3.opt.kind,n:g.userData.t3.L.length,gezeichnet:!!g.userData.t3.gezeichnet,instanzen:g.userData.t3.bild.children.length}))};
})();
