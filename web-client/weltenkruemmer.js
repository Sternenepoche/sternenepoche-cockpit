'use strict';
// Weltenkrümmer (Todesmond, Einheit „weltentruemmer“) als begehbares 3D-Modell mit vollständiger Verwandlung:
// Planetenform → Entriegeln (Nähte glühen) → langsames Aufklappen auf Teleskoparmen → Einrasten mit Lichtblitz →
// sichtbares, immer schnelleres Pulsieren beim Aufladen → etwa 3 s Laserstrahl ins Leere (das Ziel bleibt unsichtbar)
// → Abklingen → Zuklappen. Prozedural, damit sich jedes Teil wirklich bewegt; Formensprache und Farben je Volk wie
// in den Bildern megaships.<volk>.todesmond(_geschlossen). Drehen per Ziehen, Zoomen per Rad/zwei Fingern, feste
// Ansichten vorn/links/hinten/rechts/oben/unten.
window.Weltenkruemmer=(()=>{
 const VOLK={
  aurelianer:{name:'Aurelianer',rumpf:'#e9dfc8',kante:'#c9a24a',innen:'#3a3226',strahl:'#ffd21f',kern:'#ffc400',segmente:6,baender:2,zier:'tuerme'},
  krath:{name:'Krath',rumpf:'#2a2424',kante:'#7a1712',innen:'#140d0c',strahl:'#ff3b22',kern:'#ff5a1f',segmente:8,baender:2,zier:'stacheln',aufhellen:1.9},
  syntheten:{name:'Syntheten',rumpf:'#c7ccd4',kante:'#6f7d92',innen:'#1b2230',strahl:'#5ab8ff',kern:'#7fd0ff',segmente:12,baender:3,zier:'waben'},
  veyari:{name:'Veyari',rumpf:'#eef0ea',kante:'#9aa69c',innen:'#1d2a24',strahl:'#5dff7a',kern:'#7dff96',segmente:6,baender:2,zier:'leitungen'},
 };
 // Phasen in Sekunden – langsam und effektvoll (Karl, 18.10.2026)
 // Ablauf mit echten Modellen: Reiseform → Rumpfsegmente lösen sich, die offene Form erscheint → offen halten → zurück.
 // Laden und Laserschuss folgen als Blender-Animation, sobald die offene Form steht (Karl, 18.10.2026).
 const ABLAUF=[['ruhe',1.5],['aufklappen',5.5],['offen',4.0],['zuklappen',5.0],['ruhe_ende',1.2]];
 const ANSICHT={schraeg:[.75,.32],flanke:[1.3,.24],vorn:[0,0.12],links:[-Math.PI/2,0.05],hinten:[Math.PI,0.12],rechts:[Math.PI/2,0.05],oben:[0,1.5],unten:[0,-1.5]};
 const R=3;
 // Fester Zufall: jedes Laden baut exakt dasselbe Schiff, damit getrennt gerenderte Ansichten zusammenpassen.
 let zufallZustand=1;const zufall=()=>{zufallZustand=(zufallZustand*16807)%2147483647;return (zufallZustand-1)/2147483646;};const zufallStart=t=>{zufallZustand=[...String(t)].reduce((n,c)=>(n*31+c.charCodeAt(0))%2147483647,7)||1;};
 const sanft=t=>t<.5?4*t*t*t:1-(-2*t+2)**3/2;
 const zurueckSchnapp=t=>{const c=1.9,d=c+1;return 1+d*(t-1)**3+c*(t-1)**2;}; // leichtes Überschwingen = Einrasten

 function panelTextur(v){
  const c=document.createElement('canvas');c.width=512;c.height=256;const x=c.getContext('2d');
  x.fillStyle=v.rumpf;x.fillRect(0,0,512,256);let s=VOLK[v.key]?.segmente||7;const rnd=()=>{s=(s*16807)%2147483647;return s/2147483647;};
  for(let i=0;i<260;i++){const w=8+rnd()*46,h=6+rnd()*26,px=rnd()*512,py=rnd()*256,t=rnd();x.fillStyle=t<.45?'rgba(0,0,0,.16)':t<.8?'rgba(255,255,255,.10)':v.kante;x.globalAlpha=t<.8?1:.55;x.fillRect(px,py,w,h);}
  x.globalAlpha=1;x.strokeStyle='rgba(0,0,0,.35)';x.lineWidth=1.2;for(let i=0;i<=16;i++){x.beginPath();x.moveTo(i*32,0);x.lineTo(i*32,256);x.stroke();}for(let j=0;j<=8;j++){x.beginPath();x.moveTo(0,j*32);x.lineTo(512,j*32);x.stroke();}
  if(v.zier==='waben'){x.strokeStyle='rgba(60,140,255,.55)';for(let i=0;i<40;i++){const cx=rnd()*512,cy=rnd()*256,r=10;x.beginPath();for(let k=0;k<7;k++){const a=k*Math.PI/3;x.lineTo(cx+Math.cos(a)*r,cy+Math.sin(a)*r);}x.stroke();}}
  if(v.zier==='leitungen'){x.strokeStyle='rgba(90,255,130,.7)';x.lineWidth=2;for(let i=0;i<14;i++){x.beginPath();let px=rnd()*512,py=rnd()*256;x.moveTo(px,py);for(let k=0;k<6;k++){px+=rnd()*60-20;py+=rnd()*30-15;x.lineTo(px,py);}x.stroke();}}
  if(v.zier==='stacheln'){x.strokeStyle='rgba(255,60,30,.5)';x.lineWidth=1.5;for(let i=0;i<30;i++){const px=rnd()*512,py=rnd()*256;x.beginPath();x.moveTo(px,py);x.lineTo(px+12,py+3);x.stroke();}}
  const t=new THREE.CanvasTexture(c);t.colorSpace=THREE.SRGBColorSpace;t.wrapS=t.wrapT=THREE.RepeatWrapping;t.anisotropy=4;return t;
 }
 function glanzTextur(){const c=document.createElement('canvas');c.width=c.height=128;const x=c.getContext('2d'),g=x.createRadialGradient(64,64,0,64,64,64);g.addColorStop(0,'rgba(255,255,255,1)');g.addColorStop(.2,'rgba(255,255,255,.6)');g.addColorStop(1,'rgba(255,255,255,0)');x.fillStyle=g;x.fillRect(0,0,128,128);return new THREE.CanvasTexture(c);}
 function strahlMaterial(farbe){
  return new THREE.ShaderMaterial({transparent:true,depthWrite:false,blending:THREE.AdditiveBlending,side:THREE.DoubleSide,
   uniforms:{uZeit:{value:0},uStaerke:{value:0},uFarbe:{value:new THREE.Color(farbe)}},
   vertexShader:'varying vec2 vUv;void main(){vUv=uv;gl_Position=projectionMatrix*modelViewMatrix*vec4(position,1.);}',
   fragmentShader:'uniform float uZeit,uStaerke;uniform vec3 uFarbe;varying vec2 vUv;void main(){float quer=abs(vUv.x-.5)*2.;float kern=exp(-quer*quer*9.);float rand=exp(-quer*quer*1.6);float flimmer=.82+.18*sin(vUv.y*180.-uZeit*40.)*sin(vUv.y*37.+uZeit*13.);float aus=1.-smoothstep(.75,1.,vUv.y);vec3 c=mix(uFarbe,vec3(1.),kern*.85)*(kern*1.6+rand*.55)*flimmer;gl_FragColor=vec4(c,(kern+rand*.5)*uStaerke*aus);}'});
 }

 function modell(volkKey,echt=null,offen=null){
  zufallStart(volkKey);
  const v={...VOLK[volkKey]||VOLK.krath,key:volkKey};
  const wurzel=new THREE.Group(),teile={segmente:[],arme:[],zinken:[],ringe:[]};
  const tex=panelTextur(v);
  const rumpf=new THREE.MeshStandardMaterial({map:tex,color:'#ffffff',metalness:.55,roughness:.48});
  // Innenwand der Panzersegmente: beim echten Modell dieselbe Rumpftextur, stark abgedunkelt (sonst flache Leuchtflächen)
  const innen=echt?new THREE.MeshStandardMaterial({map:echt[0].mat.map,color:'#1f1d1a',metalness:0,roughness:1,side:THREE.BackSide,emissive:new THREE.Color(v.kern),emissiveIntensity:0})
   :new THREE.MeshStandardMaterial({color:v.innen,metalness:.4,roughness:.7,side:THREE.BackSide,emissive:new THREE.Color(v.kern),emissiveIntensity:0});
  const kante=new THREE.MeshStandardMaterial({color:v.kante,metalness:.85,roughness:.3,emissive:new THREE.Color(v.kern),emissiveIntensity:0});
  const metall=new THREE.MeshStandardMaterial({color:'#8d939c',metalness:.9,roughness:.35});
  // Kern mit Leuchthof
  const kernMat=new THREE.MeshBasicMaterial({color:new THREE.Color(v.kern),toneMapped:false});
  const kern=new THREE.Mesh(new THREE.SphereGeometry(.95,32,24),kernMat);wurzel.add(kern);
  const glanz=glanzTextur(),hof=new THREE.Sprite(new THREE.SpriteMaterial({map:glanz,color:new THREE.Color(v.kern),transparent:true,depthWrite:false,blending:THREE.AdditiveBlending}));hof.scale.setScalar(4.5);wurzel.add(hof);
  const licht=new THREE.PointLight(new THREE.Color(v.kern),2,30,1.6);wurzel.add(licht);
  // Innenleben: Getriebe- und Reaktorringe, Gerüst
  for(let i=0;i<3;i++){const ring=new THREE.Mesh(new THREE.TorusGeometry(1.45+i*.42,.07+.02*i,10,64),i===1?kante:metall);ring.rotation.x=Math.PI/2+i*.5;ring.rotation.y=i*.7;wurzel.add(ring);teile.ringe.push(ring);}
  const zahnrad=new THREE.Group();for(let i=0;i<36;i++){const z=new THREE.Mesh(new THREE.BoxGeometry(.16,.22,.14),metall);const a=i/36*Math.PI*2;z.position.set(Math.cos(a)*2.55,Math.sin(a)*2.55,0);z.rotation.z=a;zahnrad.add(z);}zahnrad.add(new THREE.Mesh(new THREE.TorusGeometry(2.45,.08,8,72),metall));wurzel.add(zahnrad);teile.zahnrad=zahnrad;
  teile.innen=[...teile.ringe,zahnrad];for(let i=0;i<10;i++){const g=new THREE.Mesh(new THREE.CylinderGeometry(.035,.035,4.6,6),metall);g.rotation.set(zufall()*Math.PI,zufall()*Math.PI,0);wurzel.add(g);teile.innen.push(g);}
  // Rumpfsegmente: Kugelschale in Bänder × Sektoren zerlegt; jedes Segment fährt auf einem Teleskoparm nach außen
  const nB=v.baender,sektoren=nB===3?8:v.segmente;
  // Echtes Modell (Pixal3D-Mehransicht) in dieselben Segmente zerlegt; sonst prozedurale Kugelschale
  const echtSeg=echt?echt.flatMap(t=>zerlegen(t.g,nB,sektoren).map(g=>({g,mat:t.mat}))):null;
  for(let b=0;b<nB;b++)for(let k=0;k<sektoren;k++){
   const th0=b/nB*Math.PI,thL=Math.PI/nB,ph0=k/sektoren*Math.PI*2,phL=Math.PI*2/sektoren,spalt=.012;
   const seg=new THREE.Group();
   const schale=new THREE.Mesh(new THREE.SphereGeometry(R*.9,18,12,ph0+spalt,phL-spalt*2,th0+spalt,thL-spalt*2),innen);
   // Echtes Modell: keine Innenschale (die unrunde Hülle ließe sie durchscheinen); die doppelseitige Hülle zeigt innen ihre Rückseite
   if(echtSeg){for(let e=b*sektoren+k;e<echtSeg.length;e+=nB*sektoren){if(echtSeg[e].g.attributes.position.count)seg.add(new THREE.Mesh(echtSeg[e].g,echtSeg[e].mat));}}
   else seg.add(new THREE.Mesh(new THREE.SphereGeometry(R,24,16,ph0+spalt,phL-spalt*2,th0+spalt,thL-spalt*2),rumpf),schale);
   // Richtung zur Segmentmitte (three: x=-cos(phi)sin(theta), y=cos(theta), z=sin(phi)sin(theta))
   const ph=ph0+phL/2,th=th0+thL/2,d=new THREE.Vector3(-Math.cos(ph)*Math.sin(th),Math.cos(th),Math.sin(ph)*Math.sin(th)).normalize();
   // Kantenleiste (nur prozedural; das echte Modell bringt eigene Kanten mit)
   const leiste=new THREE.Mesh(new THREE.TorusGeometry(R*Math.sin(thL/2)*.95,.05,6,40),kante);leiste.position.copy(d.clone().multiplyScalar(R*Math.cos(thL/2)*1.0));leiste.lookAt(d.clone().multiplyScalar(10));if(!echtSeg)seg.add(leiste);
   // Volkszier
   const anzahl=0;// keine zufällig verstreuten Stacheln/Türme (Karl 18.10.: zu unruhig)
   const stachelMat=new THREE.MeshStandardMaterial({color:'#2b2222',metalness:.7,roughness:.5,emissive:'#3a0805'});for(let i=0;i<anzahl;i++){const h=v.zier==='tuerme'?.9+zufall()*.6:.5+zufall()*.5,kegel=new THREE.Mesh(new THREE.ConeGeometry(v.zier==='tuerme'?.12:.09,h,6),v.zier==='tuerme'?kante:stachelMat);
    const jitter=new THREE.Vector3((zufall()-.5)*.5,(zufall()-.5)*.5,(zufall()-.5)*.5),n=d.clone().add(jitter).normalize();kegel.position.copy(n.clone().multiplyScalar(R+h/2-.05));kegel.quaternion.setFromUnitVectors(new THREE.Vector3(0,1,0),n);seg.add(kegel);}
   if(!echtSeg&&v.zier==='tuerme'&&k%2===0){const vane=new THREE.Mesh(new THREE.BoxGeometry(.04,1.6,.6),new THREE.MeshStandardMaterial({color:'#c9a24a',metalness:.9,roughness:.25,emissive:'#4a3a10'}));vane.position.copy(d.clone().multiplyScalar(R+.3));vane.lookAt(d.clone().multiplyScalar(10));seg.add(vane);}
   // Aufbauten: Paneele, Geschütztürme, Radiatorrippen – damit die Planetenform als Raumschiff lesbar ist
   const aufbauMat=v.zier==='stacheln'?new THREE.MeshStandardMaterial({color:'#3a3333',metalness:.8,roughness:.45}):rumpf;
   for(let i=0;i<0;i++){const jit=new THREE.Vector3((zufall()-.5)*.7,(zufall()-.5)*.7,(zufall()-.5)*.7),n=d.clone().add(jit.multiplyScalar(thL)).normalize();
    const art=i%3,g=art===0?new THREE.BoxGeometry(.5+zufall()*.5,.12,.3+zufall()*.4):art===1?new THREE.CylinderGeometry(.13,.16,.22,8):new THREE.BoxGeometry(.05,.35,.7);
    const t=new THREE.Mesh(g,art===2?metall:aufbauMat);t.position.copy(n.clone().multiplyScalar(R+.05));t.quaternion.setFromUnitVectors(new THREE.Vector3(0,1,0),n);t.rotateY(zufall()*Math.PI);seg.add(t);
    if(art===1){const rohr=new THREE.Mesh(new THREE.CylinderGeometry(.03,.03,.5,6),metall);rohr.position.set(0,.12,.22);rohr.rotation.x=Math.PI/2.4;t.add(rohr);}}
   // Triebwerke auf den hinteren Segmenten
   if(!echtSeg&&d.z<-.55){for(let i=0;i<3;i++){const n=d.clone().add(new THREE.Vector3((i-1)*.22,(zufall()-.5)*.2,0)).normalize();const duese=new THREE.Mesh(new THREE.CylinderGeometry(.22,.32,.5,12,1,true),metall);duese.position.copy(n.clone().multiplyScalar(R+.2));duese.quaternion.setFromUnitVectors(new THREE.Vector3(0,-1,0),n);seg.add(duese);
    const flamme=new THREE.Mesh(new THREE.CircleGeometry(.2,12),new THREE.MeshBasicMaterial({color:new THREE.Color(v.strahl),toneMapped:false}));flamme.position.copy(n.clone().multiplyScalar(R+.42));flamme.lookAt(n.clone().multiplyScalar(20));seg.add(flamme);}}
   // Kipp-Achse: senkrecht zur Richtung, damit sich das Segment beim Öffnen nach außen neigt
   const achse=new THREE.Vector3().crossVectors(d,new THREE.Vector3(0,0,1));if(achse.lengthSq()<1e-4)achse.set(1,0,0);achse.normalize();
   wurzel.add(seg);teile.segmente.push({obj:seg,d,achse,weg:2.6+zufall()*.5+(b===1&&nB===3?.4:0),kipp:.35+zufall()*.25,verzug:(b*sektoren+k)*.035});
   // Teleskoparm: drei Glieder, die sich mit dem Segment ausfahren
   const arm=new THREE.Group();for(let g=0;g<3;g++){const glied=new THREE.Mesh(new THREE.CylinderGeometry(.12-g*.03,.12-g*.03,1,8),g===1?kante:metall);arm.add(glied);}
   const gelenk=new THREE.Mesh(new THREE.SphereGeometry(.2,12,8),kante);arm.add(gelenk);
   wurzel.add(arm);teile.arme.push({obj:arm,d,gelenk});
  }
  // Emitterzinken vorn (+z): fahren aus dem Kern nach vorn und bündeln den Strahl
  const zinkenMat=new THREE.MeshStandardMaterial({color:v.kante,metalness:.85,roughness:.3,emissive:new THREE.Color(v.strahl),emissiveIntensity:0});
  for(let i=0;i<8;i++){const a=i/8*Math.PI*2,z=new THREE.Mesh(new THREE.ConeGeometry(.16,2.4,6),zinkenMat);z.userData.a=a;wurzel.add(z);teile.zinken.push(z);}
  // Strahl
  const strahl=new THREE.Mesh(new THREE.PlaneGeometry(1,1,1,1),strahlMaterial(v.strahl));strahl.visible=false;wurzel.add(strahl);
  const strahl2=strahl.clone();strahl2.material=strahl.material;wurzel.add(strahl2);
  const muendung=new THREE.Sprite(new THREE.SpriteMaterial({map:glanz,color:new THREE.Color(v.strahl),transparent:true,depthWrite:false,blending:THREE.AdditiveBlending}));muendung.scale.setScalar(0);wurzel.add(muendung);
  // Ladepartikel: werden spiralförmig zum Kern gezogen
  const N=500,pos=new Float32Array(N*3),saat=new Float32Array(N);for(let i=0;i<N;i++)saat[i]=zufall();
  const pg=new THREE.BufferGeometry();pg.setAttribute('position',new THREE.BufferAttribute(pos,3));
  const punkte=new THREE.Points(pg,new THREE.PointsMaterial({map:glanz,color:new THREE.Color(v.strahl),size:.22,transparent:true,depthWrite:false,blending:THREE.AdditiveBlending,opacity:0}));wurzel.add(punkte);
  // Offene Form (eigenes Pixal3D-Modell) und durchsichtig schaltbare Materialien für den Übergang
  let offenGruppe=null;const offenMats=[],zuMats=echt?[...new Set(echt.map(t=>t.mat))]:[];
  if(offen){offenGruppe=new THREE.Group();for(const t of offen){const mat=t.mat.clone();mat.transparent=true;mat.opacity=0;offenMats.push(mat);offenGruppe.add(new THREE.Mesh(t.g,mat));}offenGruppe.visible=false;wurzel.add(offenGruppe);}
  for(const mat of zuMats)mat.transparent=true;
  return {wurzel,teile,kern,kernMat,hof,licht,innen,kante,zinkenMat,strahl,strahl2,muendung,punkte,saat,v,echt:!!echt,offen:offenGruppe,offenMats,zuMats};
 }

 // ---------------------------------------------------------------- echtes Modell (Pixal3D-Mehransicht)
 // Minimaler GLB-Leser für die gebackenen Werk-GLBs: ein Netz, POSITION/NORMAL/TEXCOORD_0/Indizes, Farb- und ORM-Textur.
 async function glbLesen(url){
  const buf=await fetch(url).then(r=>{if(!r.ok)throw Error(url+' '+r.status);return r.arrayBuffer();});
  const dv=new DataView(buf);if(dv.getUint32(0,true)!==0x46546C67)throw Error('kein GLB');
  let off=12,json=null,bin=null;while(off<buf.byteLength){const len=dv.getUint32(off,true),typ=dv.getUint32(off+4,true);if(typ===0x4E4F534A)json=JSON.parse(new TextDecoder().decode(new Uint8Array(buf,off+8,len)));else if(typ===0x004E4942)bin=buf.slice(off+8,off+8+len);off+=8+len;}
  const acc=i=>{const a=json.accessors[i],v=json.bufferViews[a.bufferView],n={SCALAR:1,VEC2:2,VEC3:3,VEC4:4}[a.type],T=a.componentType===5126?Float32Array:a.componentType===5125?Uint32Array:a.componentType===5123?Uint16Array:Uint8Array;return new T(bin,(v.byteOffset||0)+(a.byteOffset||0),a.count*n);};
  const bild=async i=>{if(i==null)return null;const im=json.images[json.textures[i].source??json.textures[i].extensions?.EXT_texture_webp?.source],v=json.bufferViews[im.bufferView];const b=new Blob([new Uint8Array(bin,v.byteOffset||0,v.byteLength)],{type:im.mimeType});const bmp=await createImageBitmap(b);/* glTF-UV: Ursprung oben links, Bild nicht spiegeln */const t=new THREE.Texture(bmp);t.flipY=false;t.needsUpdate=true;return t;};
  const teile=[];
  for(const mesh of json.meshes)for(const pr of mesh.primitives){const g=new THREE.BufferGeometry();g.setAttribute('position',new THREE.BufferAttribute(acc(pr.attributes.POSITION),3));if(pr.attributes.NORMAL!=null)g.setAttribute('normal',new THREE.BufferAttribute(acc(pr.attributes.NORMAL),3));if(pr.attributes.TEXCOORD_0!=null)g.setAttribute('uv',new THREE.BufferAttribute(acc(pr.attributes.TEXCOORD_0),2));if(pr.indices!=null)g.setIndex(new THREE.BufferAttribute(acc(pr.indices),1));
   const mat=json.materials?.[pr.material]||{},pbr=mat.pbrMetallicRoughness||{};const farbe=await bild(pbr.baseColorTexture?.index),orm=await bild(pbr.metallicRoughnessTexture?.index);if(farbe)farbe.colorSpace=THREE.SRGBColorSpace;
   teile.push({g,mat:new THREE.MeshStandardMaterial({map:farbe,metalnessMap:orm,roughnessMap:orm,aoMap:orm,metalness:orm?.3:(pbr.metallicFactor??.3),roughness:orm?1:(pbr.roughnessFactor??.6)})});}/* Metall ohne Umgebungsspiegelung wirkt schwarz: Anteil gedämpft */
  return teile;
 }
 // Animiertes Werk-Rig (tools/content/weltenkruemmer_rig.py): ganze Szene mit Knoten, Materialien und der
 // Animation „verwandlung“ (Translation/Rotation/Skalierung je Knoten). Liefert {szene, clip, dauer}.
 async function glbSzene(url){
  const buf=await fetch(url).then(r=>{if(!r.ok)throw Error(url+' '+r.status);return r.arrayBuffer();});
  const dv=new DataView(buf);if(dv.getUint32(0,true)!==0x46546C67)throw Error('kein GLB');
  let off=12,json=null,bin=null;while(off<buf.byteLength){const len=dv.getUint32(off,true),typ=dv.getUint32(off+4,true);if(typ===0x4E4F534A)json=JSON.parse(new TextDecoder().decode(new Uint8Array(buf,off+8,len)));else if(typ===0x004E4942)bin=buf.slice(off+8,off+8+len);off+=8+len;}
  const acc=i=>{const a=json.accessors[i],v=json.bufferViews[a.bufferView],n={SCALAR:1,VEC2:2,VEC3:3,VEC4:4}[a.type],T=a.componentType===5126?Float32Array:a.componentType===5125?Uint32Array:a.componentType===5123?Uint16Array:Uint8Array;
   const stride=v.byteStride,bpe=T.BYTES_PER_ELEMENT;if(stride&&stride!==n*bpe){const out=new T(a.count*n),q=new T(bin,(v.byteOffset||0)+(a.byteOffset||0),(a.count-1)*stride/bpe+n);for(let k=0;k<a.count;k++)for(let j=0;j<n;j++)out[k*n+j]=q[k*stride/bpe+j];return out;}
   return new T(bin,(v.byteOffset||0)+(a.byteOffset||0),a.count*n);};
  const texCache=new Map();const tex=async(i,srgb)=>{if(i==null)return null;if(texCache.has(i))return texCache.get(i);const p=(async()=>{const im=json.images[json.textures[i].source??json.textures[i].extensions?.EXT_texture_webp?.source],v=json.bufferViews[im.bufferView];const bmp=await createImageBitmap(new Blob([new Uint8Array(bin,v.byteOffset||0,v.byteLength)],{type:im.mimeType}));const t=new THREE.Texture(bmp);t.flipY=false;t.anisotropy=4;/* glTF-Standard ist Wiederholen (Kacheltexturen der gebauten Rigs) */t.wrapS=t.wrapT=THREE.RepeatWrapping;if(srgb)t.colorSpace=THREE.SRGBColorSpace;t.needsUpdate=true;return t;})();texCache.set(i,p);return p;};
  const mats=await Promise.all((json.materials||[]).map(async m=>{const pbr=m.pbrMetallicRoughness||{},farbe=await tex(pbr.baseColorTexture?.index,true),orm=await tex(pbr.metallicRoughnessTexture?.index,false),normal=await tex(m.normalTexture?.index,false);const f=pbr.baseColorFactor||[1,1,1,1];
   /* Gebaute Rigs (weltenkruemmer_bau.py): Atlas mit Normalenkarte; Leucht-/Glutmaterialien mit Emission in Volksfarbe */
   const em=m.emissiveFactor||[0,0,0],emStaerke=m.extensions?.KHR_materials_emissive_strength?.emissiveStrength??1;
   const mat=new THREE.MeshStandardMaterial({map:farbe,color:new THREE.Color(f[0],f[1],f[2]),metalnessMap:orm,roughnessMap:orm,normalMap:normal,metalness:orm?(normal?1:.3):Math.min(.6,pbr.metallicFactor??.3),roughness:orm?1:(pbr.roughnessFactor??.6),
    emissive:new THREE.Color(em[0],em[1],em[2]),emissiveIntensity:emStaerke,side:normal?THREE.FrontSide:THREE.DoubleSide});if(normal)mat.normalScale.set(1,-1);mat.name=m.name||'';mat.userData.leuchtBasis=emStaerke;return mat;}));
  const standard=new THREE.MeshStandardMaterial({color:'#888',metalness:.3,roughness:.7});
  /* Netze, die mehrere Knoten teilen (alle Sektorarme/-platten), nur einmal hochladen */
  /* Eckfarben (Verdeckung, Kantenabrieb) brauchen vertexColors – je Material eine Kopie mit Eckfarben */
  const mitFarbe=new Map();const farbig=(g,m)=>{if(!g.attributes.color)return m;if(!mitFarbe.has(m)){const k=m.clone();k.vertexColors=true;k.name=m.name;k.userData={...m.userData};mitFarbe.set(m,k);}return mitFarbe.get(m);};
  const geoCache=new Map();const geometrie=pr=>{if(geoCache.has(pr))return geoCache.get(pr);const g=new THREE.BufferGeometry();geoCache.set(pr,g);return g;};
  const knoten=json.nodes.map((n,i)=>{const o=new THREE.Object3D();o.name='n'+i;if(n.translation)o.position.fromArray(n.translation);if(n.rotation)o.quaternion.fromArray(n.rotation);if(n.scale)o.scale.fromArray(n.scale);
   if(n.mesh!=null)for(const pr of json.meshes[n.mesh].primitives){const neu=!geoCache.has(pr),g=geometrie(pr);if(!neu){o.add(new THREE.Mesh(g,farbig(g,pr.material!=null?mats[pr.material]:standard)));continue;}g.setAttribute('position',new THREE.BufferAttribute(acc(pr.attributes.POSITION),3));if(pr.attributes.NORMAL!=null)g.setAttribute('normal',new THREE.BufferAttribute(acc(pr.attributes.NORMAL),3));if(pr.attributes.TEXCOORD_0!=null)g.setAttribute('uv',new THREE.BufferAttribute(acc(pr.attributes.TEXCOORD_0),2));if(pr.attributes.COLOR_0!=null){const ca=json.accessors[pr.attributes.COLOR_0],n={VEC3:3,VEC4:4}[ca.type];g.setAttribute('color',new THREE.BufferAttribute(acc(pr.attributes.COLOR_0),n,ca.componentType!==5126));}if(pr.indices!=null)g.setIndex(new THREE.BufferAttribute(acc(pr.indices),1));o.add(new THREE.Mesh(g,farbig(g,pr.material!=null?mats[pr.material]:standard)));}
   return o;});
  json.nodes.forEach((n,i)=>{for(const c of n.children||[])knoten[i].add(knoten[c]);});
  const szene=new THREE.Group();for(const i of json.scenes[json.scene||0].nodes)szene.add(knoten[i]);
  // Blender legt je Objekt eine eigene Animation an: alle Kanäle zu einem Clip zusammenführen
  let clip=null;const anims=json.animations||[];
  if(anims.length){const spuren=[];for(const anim of anims)for(const ch of anim.channels){const sm=anim.samplers[ch.sampler],zeiten=acc(sm.input),werte=acc(sm.output),ziel='n'+ch.target.node;
    if(ch.target.path==='translation')spuren.push(new THREE.VectorKeyframeTrack(ziel+'.position',Array.from(zeiten),Array.from(werte)));
    else if(ch.target.path==='rotation')spuren.push(new THREE.QuaternionKeyframeTrack(ziel+'.quaternion',Array.from(zeiten),Array.from(werte)));
    else if(ch.target.path==='scale')spuren.push(new THREE.VectorKeyframeTrack(ziel+'.scale',Array.from(zeiten),Array.from(werte)));}
   clip=new THREE.AnimationClip('verwandlung',-1,spuren);}
  // Bugspitze (Strahlaustritt) = vorderster Punkt des Kerns in Kampfform; Kern heißt im Rig „kern“
  const namen=json.nodes.map(n=>n.name||'');
  return {szene,clip,dauer:clip?clip.duration:0,kernKnoten:knoten[namen.indexOf('kern')]||null,muendungKnoten:namen.includes('muendung')?knoten[namen.indexOf('muendung')]:null,benannt:Object.fromEntries(namen.map((n,i)=>[n,knoten[i]]).filter(([n])=>n))};
 }
 // Zerlegt das Rumpfnetz in Segmente nach der Richtung der Dreiecksmitte (gleiche Bänder/Sektoren wie das Modell).
 function zerlegen(g,nB,sektoren){
  const nicht=g.index?g.toNonIndexed():g;nicht.computeBoundingSphere();const pos=nicht.attributes.position,n=pos.count/3;
  const listen=Array.from({length:nB*sektoren},()=>[]),v=new THREE.Vector3(),w=new THREE.Vector3(),u=new THREE.Vector3();
  for(let t=0;t<n;t++){v.fromBufferAttribute(pos,t*3);w.fromBufferAttribute(pos,t*3+1);u.fromBufferAttribute(pos,t*3+2);v.add(w).add(u).multiplyScalar(1/3).normalize();
   const th=Math.acos(Math.max(-1,Math.min(1,v.y))),ph=(Math.atan2(v.z,-v.x)+Math.PI*2)%(Math.PI*2);const b=Math.min(nB-1,Math.floor(th/Math.PI*nB)),k=Math.min(sektoren-1,Math.floor(ph/(Math.PI*2)*sektoren));listen[b*sektoren+k].push(t);}
  return listen.map(dreiecke=>{const neu=new THREE.BufferGeometry();for(const [name,a] of Object.entries(nicht.attributes)){const gr=a.itemSize,arr=new a.array.constructor(dreiecke.length*3*gr);dreiecke.forEach((t,i)=>{arr.set(a.array.subarray(t*3*gr,(t*3+3)*gr),i*3*gr);});neu.setAttribute(name,new THREE.BufferAttribute(arr,gr));}return neu;});
 }
 const modellCache=new Map();let standbildModus=false,ladeVersprechen=Promise.resolve();
 // Modelle liegen neben diesem Skript (assets/…): im Spiel unter /assets, in Vorschauen unter /web-client/assets.
 // Früher fest „/web-client/“ ohne Basis-Pfad → im Spiel (lokaler Server, App) 404, Weltenkrümmer erschienen nie.
 const skriptBasis=document.currentScript?.src||document.baseURI;
 const assetUrl=rel=>new URL(rel,skriptBasis).href;
 const rigCache=new Map();const rigLaden=(k,stufe='')=>{const u=assetUrl(`assets/weltenkruemmer.${k}.rig${stufe?'.'+stufe:''}.glb`);if(!rigCache.has(u))rigCache.set(u,glbSzene(u).catch(()=>null));return rigCache.get(u);};
 async function echtesModell(volkKey,zustand='zu'){
  const url=`assets/weltenkruemmer.${volkKey}${zustand==='offen'?'.offen':''}.glb`;if(modellCache.has(url))return modellCache.get(url);
  const p=glbLesen(new URL(url,document.baseURI.includes('/web-client/')?document.baseURI:location.origin+'/web-client/').href).then(teile=>{
   // Mittelpunkt in den Ursprung; Maßstab nach dem mittleren Rumpfradius (Median), nicht nach der größten
   // Ausdehnung – Stacheln und Türme ragen sonst so weit, dass Innenteile durch die zu kleine Hülle stechen.
   const box=new THREE.Box3();for(const t of teile){t.g.computeBoundingBox();box.union(t.g.boundingBox);}const mitte=box.getCenter(new THREE.Vector3());
   const abst=[];for(const t of teile){const a=t.g.attributes.position;for(let i=0;i<a.count;i+=7)abst.push(Math.hypot(a.getX(i)-mitte.x,a.getY(i)-mitte.y,a.getZ(i)-mitte.z));}abst.sort((x,y)=>x-y);
   const s=R*1.02/Math.max(1e-6,abst[Math.floor(abst.length*.5)]);
   for(const t of teile){t.g.translate(-mitte.x,-mitte.y,-mitte.z);t.g.scale(s,s,s);}return teile;}).catch(e=>{console.info('Weltenkrümmer: kein Modell, prozedurale Hülle',e.message);return null;});
  modellCache.set(url,p);return p;
 }

 // ---------------------------------------------------------------- Rig-Modus (echtes animiertes Modell)
 // Ablauf: Planetenform → stufenloses Ausfahren aller Teile (Blender-Animation) → offen → Aufladen → ~3 s Strahl
 // ins Leere → Abklingen → Einfahren. Effekte erst, wenn das Schiff vollständig offen ist (Karl, 18.10.2026).
 const rigAblauf=d=>[['ruhe',1.0],['aufklappen',d],['offen',0.8],['laden',7.0],['feuer',3.0],['abklingen',1.2],['zuklappen',d],['ruhe_ende',1.0]];
 function phaseIn(liste,t){let s=0;for(const [name,d] of liste){if(t<s+d)return {name,u:(t-s)/d};s+=d;}return {name:'ende',u:1};}
 function rigBauen(volkKey,rig){
  const v={...VOLK[volkKey]||VOLK.krath,key:volkKey},wurzel=new THREE.Group();rig.szene.scale.setScalar(R);/* Rig ist auf Rumpfradius 1 normiert */wurzel.add(rig.szene);
  /* Dunkle Volksfarben (Krath: Schwarz-Rot) heben, sonst säuft der Rumpf im All ab; Materialien teilen sich die Instanzen, darum nur einmal */
  rig.szene.traverse(o=>{for(const mat of [].concat(o.material||[]))if(mat.color&&!mat.userData.aufgehellt){if(v.aufhellen)mat.color.multiplyScalar(v.aufhellen);if('metalness' in mat&&!mat.normalMap)mat.metalness=Math.min(mat.metalness,.35);mat.userData.aufgehellt=true;}});
  const mixer=new THREE.AnimationMixer(rig.szene),aktion=rig.clip?mixer.clipAction(rig.clip):null;if(aktion){aktion.setLoop(THREE.LoopOnce,1);aktion.clampWhenFinished=true;aktion.timeScale=0;aktion.play();}/* Vorspulen: laufend mit Tempo 0, Zeit wird gesetzt */
  // Bugspitze und Kernmitte in voll ausgefahrener Stellung bestimmen
  const nase=new THREE.Vector3(0,0,R*1.4),kernMitte=new THREE.Vector3();
  if(aktion&&rig.kernKnoten){aktion.time=rig.dauer;mixer.update(0);rig.szene.updateMatrixWorld(true);const b=new THREE.Box3().setFromObject(rig.kernKnoten);b.getCenter(kernMitte);nase.set(kernMitte.x,kernMitte.y,b.max.z);
   if(rig.muendungKnoten){const bm=new THREE.Box3().setFromObject(rig.muendungKnoten),c=bm.getCenter(new THREE.Vector3());nase.set(c.x,c.y,bm.max.z);kernMitte.set(c.x,c.y,0);}aktion.time=0;mixer.update(0);}
  /* Ladeweg (Karl, 18.10.): außen an allen Armen gleichzeitig Lichter stufenweise an, je ein Laserpunkt läuft die Sprossen
     hinab zum ersten Ring (blinkt rot auf), dann stufenweise die Spindel entlang, erst dann bildet die Spitze den Laser. */
  const ladeweg={arme:[],spindel:[],ringZ:R*.55,ringR:R*.40};
  if(aktion&&rig.benannt){aktion.time=rig.dauer;mixer.update(0);rig.szene.updateMatrixWorld(true);const pw=new THREE.Vector3();
   for(const [n,o] of Object.entries(rig.benannt)){if(!/^platte_\d+$/.test(n)||!o)continue;o.getWorldPosition(pw);const d=pw.clone().setZ(0);if(d.lengthSq()<1e-6)continue;d.normalize();
    ladeweg.arme.push({d,punkte:[1.80,1.48,1.18,.90].map(f=>d.clone().multiplyScalar(R*f)).concat([d.clone().multiplyScalar(R*.62),d.clone().multiplyScalar(ladeweg.ringR).setZ(ladeweg.ringZ)])});}
   for(const n of ['spindel_1','spindel_2','muendung']){const o=rig.benannt[n];if(!o)continue;const b=new THREE.Box3().setFromObject(o),c=b.getCenter(new THREE.Vector3());ladeweg.spindel.push({z:b.max.z-(n==='muendung'?R*.05:R*.02),r:Math.max(b.max.x-c.x,b.max.y-c.y)*1.15});}
   aktion.time=0;mixer.update(0);}
  const leucht=new Set();rig.szene.traverse(o=>{for(const mat of [].concat(o.material||[]))if(/^(leuchten|glut)/.test(mat.name||''))leucht.add(mat);});
  const glanz=glanzTextur(),farbe=new THREE.Color(v.strahl),kernFarbe=new THREE.Color(v.kern);
  const sprite=(f,s)=>{const sp=new THREE.Sprite(new THREE.SpriteMaterial({map:glanz,color:f,transparent:true,depthWrite:false,blending:THREE.AdditiveBlending,opacity:0}));sp.scale.setScalar(s);wurzel.add(sp);return sp;};
  const kernGlut=sprite(kernFarbe,0);kernGlut.position.copy(kernMitte);
  const muendung=sprite(farbe,0);muendung.position.copy(nase);
  const ring=new THREE.Mesh(new THREE.RingGeometry(.55,.62,64),new THREE.MeshBasicMaterial({color:farbe,transparent:true,opacity:0,side:THREE.DoubleSide,blending:THREE.AdditiveBlending,depthWrite:false}));ring.position.copy(nase);wurzel.add(ring);
  const licht=new THREE.PointLight(farbe,0,40,1.5);licht.position.copy(nase);wurzel.add(licht);
  const strahlMat=strahlMaterial(v.strahl),strahlen=[0,1].map(i=>{const s=new THREE.Mesh(new THREE.PlaneGeometry(1,1),strahlMat);s.rotation.order='ZYX';s.rotation.set(Math.PI/2,0,i*Math.PI/2);s.visible=false;wurzel.add(s);return s;});
  const N=360,pos=new Float32Array(N*3),saat=Float32Array.from({length:N},(_,i)=>((i*0.61803398875)%1));const pg=new THREE.BufferGeometry();pg.setAttribute('position',new THREE.BufferAttribute(pos,3));
  const funken=new THREE.Points(pg,new THREE.PointsMaterial({map:glanz,color:farbe,size:.16,transparent:true,depthWrite:false,blending:THREE.AdditiveBlending,opacity:0}));wurzel.add(funken);
  /* Ladeimpulse: Lichtkugeln laufen vom runden Kern die Spindel entlang zur Muendung, immer schneller (Karl, 18.10.) */
  const impulse=[];/* ersetzt durch den Ladeweg */
  const ringMat=c=>new THREE.MeshBasicMaterial({color:c,transparent:true,opacity:0,blending:THREE.AdditiveBlending,depthWrite:false});
  const armLichter=ladeweg.arme.map(a=>a.punkte.slice(0,4).map(p=>{const sp=sprite(kernFarbe,0);sp.position.copy(p);return sp;}));
  const laufPunkte=ladeweg.arme.map(()=>sprite(new THREE.Color('#ffffff').lerp(kernFarbe,.35),0));
  const reaktorRing=new THREE.Mesh(new THREE.TorusGeometry(ladeweg.ringR,R*.028,10,96),ringMat(new THREE.Color('#ff2a10').lerp(kernFarbe,.4)));reaktorRing.position.z=ladeweg.ringZ;wurzel.add(reaktorRing);
  const ringBlitz=sprite(new THREE.Color('#ff3a1a').lerp(kernFarbe,.4),0);ringBlitz.position.z=ladeweg.ringZ;
  const spindelRinge=ladeweg.spindel.map(sp=>{const t=new THREE.Mesh(new THREE.TorusGeometry(sp.r,R*.02,8,64),ringMat(kernFarbe));t.position.z=sp.z;wurzel.add(t);return t;});
  const spindelPunkt=sprite(new THREE.Color('#ffffff').lerp(kernFarbe,.3),0);
  const reaktorLicht=new THREE.PointLight(kernFarbe,0,R*5,1.5);reaktorLicht.position.copy(kernMitte);wurzel.add(reaktorLicht);
  return {rig:true,v,wurzel,mixer,aktion,dauer:rig.dauer,ablauf:rigAblauf(rig.dauer),nase,kernMitte,kernGlut,muendung,ring,licht,strahlMat,strahlen,funken,saat,leucht:[...leucht],impulse,reaktorLicht,ladeweg,armLichter,laufPunkte,reaktorRing,ringBlitz,spindelRinge,spindelPunkt};
 }
 function rigAnwenden(m,t,zeit){
  const {name,u}=phaseIn(m.ablauf,t);
  const tc=name==='aufklappen'?u*m.dauer:['offen','laden','feuer','abklingen'].includes(name)?m.dauer:name==='zuklappen'?(1-u)*m.dauer:0;
  if(m.aktion){m.aktion.time=Math.min(m.dauer-1e-4,Math.max(0,tc));m.mixer.update(0);}
  const auf=m.dauer?tc/m.dauer:0;
  const ladung=name==='laden'?u*u:name==='feuer'?1:name==='abklingen'?1-u:0,feuer=name==='feuer'?Math.min(1,u*10)*Math.min(1,(1-u)*8):0;
  // Pulsieren wird beim Laden immer schneller (sichtbares Aufladen)
  const takt=1.5+ladung*10,puls=.5+.5*Math.sin(zeit*takt*Math.PI*2);
  // Ladeweg: Fortschritt je Abschnitt (0..1); während Feuer alles an, beim Abklingen verlöschend
  const lu=name==='laden'?u:['feuer'].includes(name)?1:name==='abklingen'?1:0,aus=name==='abklingen'?1-u:1,an=(lu>0?1:0)*aus;
  const tA=Math.min(1,lu/.42),tB=Math.max(0,Math.min(1,(lu-.42)/.10)),tC=Math.max(0,Math.min(1,(lu-.52)/.32)),tD=Math.max(0,Math.min(1,(lu-.84)/.16));
  const stufenLicht=(schwelle,t)=>t<schwelle?0:(1+2.2*Math.exp(-(t-schwelle)*18))*(.75+.25*puls);
  if(m.armLichter){m.armLichter.forEach(liste=>liste.forEach((sp,k)=>{const h=stufenLicht(k/4,tA)*an;sp.material.opacity=Math.min(1,h+feuer*.4);sp.scale.setScalar(R*(.26+.2*Math.min(1.6,h)));}));
   m.laufPunkte.forEach((sp,i)=>{const pk=m.ladeweg.arme[i].punkte,seg=tA*(pk.length-1),j=Math.min(pk.length-2,Math.floor(seg));sp.position.lerpVectors(pk[j],pk[j+1],seg-j);sp.material.opacity=(tA>0&&tA<1?1:0)*an;sp.scale.setScalar(R*.34);});
   const ringH=tB>0?(1+3*Math.exp(-(lu-.42)*14))*(.7+.3*puls):0;m.reaktorRing.material.opacity=Math.min(1,ringH*.7*an+feuer*.6);m.ringBlitz.material.opacity=Math.min(1,tB>0?3*Math.exp(-(lu-.42)*10):0)*an;m.ringBlitz.scale.setScalar(R*(1.2+tB*2));
   m.spindelRinge.forEach((t,j)=>{const h=stufenLicht(j/Math.max(1,m.spindelRinge.length),tC)*an;t.material.opacity=Math.min(1,h*.75+feuer*.5);});
   const sp0=m.ladeweg.ringZ,sp1=m.nase.z;m.spindelPunkt.position.set(m.nase.x,m.nase.y,sp0+(sp1-sp0)*tC);m.spindelPunkt.material.opacity=(tC>0&&tC<1?1:0)*an;m.spindelPunkt.scale.setScalar(R*.26);}
  m.kernGlut.material.opacity=Math.min(.6,tB*.35+feuer*.3)*an;m.kernGlut.scale.setScalar(R*(.5+tB*.4+puls*tB*.1));
  // Mündung: erst im letzten Abschnitt bildet sich der Laser
  const lade=tD*tD;
  m.muendung.material.opacity=Math.min(1,lade*.85+feuer*.85)*aus;m.muendung.scale.setScalar(.3+lade*1.8+puls*lade*.6+feuer*2.6);
  m.ring.material.opacity=lade*(.35+.5*puls)*(1-feuer*.5);m.ring.scale.setScalar(1+(1-lade)*2.5);m.ring.lookAt(m.nase.clone().add(new THREE.Vector3(0,0,10)));
  m.licht.intensity=lade*6+puls*lade*3+feuer*14+tB*3*an;
  const p=m.funken.geometry.attributes.position;for(let i=0;i<m.saat.length;i++){const s=m.saat[i],ph=(s+zeit*(.35+lade))%1,r=(1-ph)*5+.15,a=s*80+ph*7;p.setXYZ(i,m.nase.x+Math.cos(a)*r,m.nase.y+Math.sin(a)*r,m.nase.z+(1-ph)*1.5-(s-.5));}p.needsUpdate=true;m.funken.material.opacity=lade*(1-feuer*.6);
  const laenge=m.strahlLaenge||420;for(const s of m.strahlen){s.visible=feuer>0;s.scale.set(1.1+feuer*1.4+Math.sin(zeit*55)*.06,laenge,1);s.position.set(m.nase.x,m.nase.y,m.nase.z+laenge/2);}
  m.strahlMat.uniforms.uZeit.value=zeit;m.strahlMat.uniforms.uStaerke.value=feuer;
  // Leuchtschlitze und Glutfugen werden mit der Ladung heller und pulsieren im selben Takt
  if(m.reaktorLicht)m.reaktorLicht.intensity=auf*2.5+ladung*(6+puls*3)+feuer*8;
  {const lw=m.armLichter?(Math.max(0,Math.min(1,((name==='laden'?u:name==='feuer'||name==='abklingen'?1:0)-.42)/.1))*1.2+Math.max(0,((name==='laden'?u:name==='feuer'||name==='abklingen'?1:0)-.84)/.16)*1.4)*(name==='abklingen'?1-u:1):ladung*1.6;
   for(const mat of m.leucht||[])mat.emissiveIntensity=(mat.userData.leuchtBasis||1)*(1+lw*(1+puls*.6)+feuer*2.5);}
  if(m.impulse){const n=m.impulse.length,tempo=.25+ladung*1.6+feuer*2.5,weg=m.nase.clone().sub(m.kernMitte);
   m.impulse.forEach((sp,i)=>{const ph=((zeit*tempo)+i/n)%1;sp.position.copy(m.kernMitte).addScaledVector(weg,ph);sp.material.opacity=Math.min(1,(ladung*.9+feuer)*Math.sin(Math.PI*ph));sp.scale.setScalar(R*(.12+ladung*.2+feuer*.25)*(1-ph*.4));});}
  return {name,u,auf,ladung,feuer};
 }

 // Zustand je Zeitpunkt t (Sekunden seit Start der Sequenz)
 function phaseBei(t){let s=0;for(const [name,d] of ABLAUF){if(t<s+d)return {name,u:(t-s)/d};s+=d;}return {name:'ende',u:1};}
 const DAUER=ABLAUF.reduce((s,[,d])=>s+d,0);
 function anwenden(m,t,zeit,kamera){
  if(m.rig)return rigAnwenden(m,t,zeit);
  const {name,u}=phaseBei(t);
  // Öffnungsgrad 0..1
  let auf=0;if(name==='aufklappen')auf=sanft(u);else if(name==='einrasten')auf=zurueckSchnapp(u)*.04+.96+.04*(1-u);else if(['laden','feuer','abklingen'].includes(name))auf=1;else if(name==='zuklappen')auf=1-sanft(u);
  if(name==='aufklappen')auf=sanft(u);else if(name==='offen')auf=1;else if(name==='zuklappen')auf=1-sanft(u);
  if(m.echt){
   if(!m.offen)auf=0;// ohne offene Form keine Verwandlung zeigen (kein Platzhalter)
   for(const s of m.teile.segmente){s.obj.position.copy(s.d).multiplyScalar(s.weg*1.7*auf);s.obj.quaternion.setFromAxisAngle(s.achse,s.kipp*auf);}
   const zuSicht=1-Math.min(1,Math.max(0,(auf-.25)/.55)),offenSicht=Math.min(1,Math.max(0,(auf-.3)/.55));
   for(const mat of m.zuMats){mat.opacity=zuSicht;mat.depthWrite=zuSicht>.99;}
   for(const s of m.teile.segmente)s.obj.visible=zuSicht>.01;
   if(m.offen){m.offen.visible=offenSicht>.01;m.offen.scale.setScalar(.84+.16*auf);for(const mat of m.offenMats){mat.opacity=offenSicht;mat.depthWrite=offenSicht>.99;}}
   for(const o of [m.kern,m.hof,m.strahl,m.strahl2,m.muendung,m.punkte,...m.teile.zinken,...m.teile.arme.map(a=>a.obj),...(m.teile.innen||[])])o.visible=false;
   m.licht.intensity=0;
   return {name,u,auf,ladung:0,feuer:0};
  }
  const naht=name==='entriegeln'?u:name==='aufklappen'?1-u*.4:name==='einrasten'?1-u*.5+Math.sin(u*Math.PI)*1.2:['laden','feuer'].includes(name)?.55:name==='abklingen'?.55*(1-u):name==='zuklappen'?.3*(1-u):0;
  let ladung=name==='laden'?u:name==='feuer'?1:name==='abklingen'?1-u:0;
  const feuer=name==='feuer'?Math.min(1,u*8)*Math.min(1,(1-u)*10):0;
  // Segmente und Arme
  for(const s of m.teile.segmente){const a=Math.max(0,Math.min(1.08,(auf-s.verzug*(name==='zuklappen'?-1:1))/(1-s.verzug*.6)));const w=s.weg*a;s.obj.position.copy(s.d).multiplyScalar(w);s.obj.quaternion.setFromAxisAngle(s.achse,s.kipp*a);}
  m.teile.arme.forEach((arm,i)=>{const s=m.teile.segmente[i],l=Math.max(.001,s.obj.position.length()+R*.6),kinder=arm.obj.children;arm.obj.quaternion.setFromUnitVectors(new THREE.Vector3(0,1,0),arm.d);arm.obj.position.set(0,0,0);
   for(let g=0;g<3;g++){const glied=kinder[g],laenge=l/3+.2;glied.scale.set(1,laenge,1);glied.position.set(0,1.1+(g+.5)*l/3,0);}
   arm.gelenk.position.set(0,1.1+l,0);arm.obj.visible=auf>.02;});
  // Innenleben nur sichtbar, wenn die Hülle offen ist (unrunde echte Rümpfe ließen es sonst durchscheinen)
  for(const o of m.teile.innen||[])o.visible=auf>.03;
  // Nähte, Kantenleisten und Gelenke glühen
  m.innen.emissiveIntensity=(naht*.32+ladung*.18)*(m.innen.map?.35:1);m.kante.emissiveIntensity=naht*.55+(name==='einrasten'?Math.sin(u*Math.PI)*1.6:0)+ladung*.3;
  // Ringe und Zahnrad drehen sich, schneller beim Laden
  if(!standbildModus){m.teile.ringe.forEach((r,i)=>{r.rotation.z+=(.003+ladung*.05)*(i%2?1:-1);});m.teile.zahnrad.rotation.z+=.002+auf*.004+ladung*.03;}
  // Kern pulsiert immer schneller
  const f=1.2+ladung*9,puls=.5+.5*Math.sin(zeit*f*Math.PI*2);
  const k=.75+ladung*.55+puls*ladung*.25+feuer*.4;m.kern.scale.setScalar(k);m.hof.scale.setScalar(2.4+ladung*3+puls*ladung*1.2+feuer*3.5);m.hof.material.opacity=.22+naht*.18+ladung*.42;
  m.licht.intensity=.8+naht*1.4+ladung*2.6+puls*ladung*1.4+feuer*4.5;
  // Emitterzinken
  m.teile.zinken.forEach(z=>{const r=.7+auf*.9,vor=.6+auf*3.2;z.position.set(Math.cos(z.userData.a)*r,Math.sin(z.userData.a)*r,vor);z.lookAt(0,0,vor+6);z.rotateX(Math.PI/2);z.visible=auf>.05;});
  m.zinkenMat.emissiveIntensity=ladung*1.2+puls*ladung*.8+feuer*2;
  // Ladepartikel
  const p=m.punkte.geometry.attributes.position;for(let i=0;i<m.saat.length;i++){const s=m.saat[i],ph=(s+zeit*(.25+ladung*.9))%1,r=8*(1-ph)+.6,a=s*60+ph*9;p.setXYZ(i,Math.cos(a)*r,Math.sin(a*1.3)*r*.5,Math.sin(a)*r);}p.needsUpdate=true;m.punkte.material.opacity=ladung*.95;
  // Strahl: zwei gekreuzte Flächen entlang +z, zur Kamera ausgerichtet um die Strahlachse
  const laenge=420;for(const [i,s] of [m.strahl,m.strahl2].entries()){s.visible=feuer>0;s.scale.set(1.6+feuer*1.2+Math.sin(zeit*60)*.08,laenge,1);s.position.set(0,0,4+laenge/2);s.rotation.order='ZYX';s.rotation.set(Math.PI/2,0,i*Math.PI/2);/* zwei gekreuzte Flächen (XZ und YZ) */s.material.uniforms.uZeit.value=zeit;s.material.uniforms.uStaerke.value=feuer;}
  m.muendung.position.set(0,0,4.1);m.muendung.scale.setScalar(feuer*6+ladung*2*puls);m.muendung.material.opacity=Math.max(feuer,ladung*.6);
  return {name,u,auf,ladung,feuer};
 }

 // ---------------------------------------------------------------- Betrachter (Dialog)
 let film=null,schleife=false,kameraFahrt=false,d=null,renderer=null,szene=null,kamera=null,m=null,volk='krath',t0=0,laeuft=false,t=0,frame=0,letzt=0,drehung=0,yaw=.7,pitch=.25,abstand=13,ziehen=null,halten=null;
 const NAMEN={offen:'Offen',ruhe:'Planetenform',entriegeln:'Entriegeln',aufklappen:'Aufklappen',einrasten:'Einrasten',laden:'Aufladen',feuer:'Laserstrahl',abklingen:'Abklingen',zuklappen:'Zuklappen',ruhe_ende:'Planetenform',ende:'Planetenform'};
 function aufbauen(){
  zufallStart('kulisse');
  d=document.createElement('dialog');d.className='weltenkruemmer-dialog';d.setAttribute('aria-label','Weltenkrümmer in 3D');
  d.innerHTML=`<div class="wk-kopf"><h2>Weltenkrümmer · <span data-wk-volkname></span></h2><button type="button" data-wk-zu aria-label="Schließen">✕</button></div>
<div class="wk-buehne"><canvas></canvas><div class="wk-phase" aria-live="polite"></div></div>
<div class="wk-leiste"><button type="button" data-wk-start>▶ Verwandlung abspielen</button><button type="button" data-wk-halt>⏸ Anhalten</button>
<label>Volk <select data-wk-volk>${Object.entries(VOLK).map(([k,v])=>`<option value="${k}">${v.name}</option>`).join('')}</select></label>
<span class="wk-ansichten">${Object.keys(ANSICHT).map(a=>`<button type="button" data-wk-ansicht="${a}">${a[0].toUpperCase()+a.slice(1)}</button>`).join('')}</span>
<label>Phase <input type="range" min="0" max="${DAUER.toFixed(1)}" step="0.05" value="0" data-wk-zeit></label></div>
<p class="wk-hinweis">Ziehen dreht, Mausrad oder zwei Finger zoomen. Der Strahl trifft ein Ziel außerhalb des Bildes.</p>
<div class="wk-bilder" aria-label="Ansichten und Werft"></div><div class="wk-gross" hidden><img alt=""><button type="button" data-wk-gross-zu aria-label="Zurück">✕</button></div>`;
  document.body.append(d);
  
  const c=d.querySelector('canvas');
  renderer=new THREE.WebGLRenderer({canvas:c,antialias:true,powerPreference:'low-power'});renderer.setPixelRatio(Math.min(devicePixelRatio,1.75));renderer.outputColorSpace=THREE.SRGBColorSpace;renderer.toneMapping=THREE.ACESFilmicToneMapping;renderer.toneMappingExposure=1.1;
  szene=new THREE.Scene();szene.background=new THREE.Color('#02050b');try{const pm=new THREE.PMREMGenerator(renderer),es=new THREE.Scene(),kiste=new THREE.BoxGeometry(1,1,1);es.background=new THREE.Color('#06080e');for(const [x,y,z,sx,sy,sz,c] of [[0,9,0,14,.4,14,'#9fb0cc'],[-10,2,5,.4,7,9,'#ffe8c8'],[10,-2,-4,.4,5,7,'#4a5f8a'],[0,-9,0,10,.4,10,'#151a24']]){const w=new THREE.Mesh(kiste,new THREE.MeshBasicMaterial({color:c}));w.position.set(x,y,z);w.scale.set(sx,sy,sz);es.add(w);}szene.environment=pm.fromScene(es,.04).texture;pm.dispose();}catch(e){console.info('Weltenkrümmer: keine Umgebungsspiegelung',e.message);}szene.fog=new THREE.Fog('#02050b',60,260);
  kamera=new THREE.PerspectiveCamera(40,1.6,.1,800);
  szene.add(new THREE.AmbientLight('#d8e0ec',.45));szene.add(new THREE.HemisphereLight('#dfe8ff','#3a3020',.6));const sonne=new THREE.DirectionalLight('#fff4e0',3.6);sonne.position.set(-12,9,10);szene.add(sonne);const kont=new THREE.DirectionalLight('#7aa0ff',.6);kont.position.set(10,-4,-8);szene.add(kont);const unten=new THREE.DirectionalLight('#b8c4dc',1.1);unten.position.set(-2,-12,3);szene.add(unten);/* Fülllicht: Unterseite bleibt lesbar */
  const sp=[];for(let i=0;i<1400;i++){const a=zufall()*Math.PI*2,b=Math.acos(2*zufall()-1),r=300+zufall()*200;sp.push(r*Math.sin(b)*Math.cos(a),r*Math.cos(b),r*Math.sin(b)*Math.sin(a));}
  const sg=new THREE.BufferGeometry();sg.setAttribute('position',new THREE.Float32BufferAttribute(sp,3));const sterne=new THREE.Points(sg,new THREE.PointsMaterial({color:'#9fb6d6',size:1,sizeAttenuation:false}));szene.add(sterne);

  // Steuerung
  c.addEventListener('pointerdown',e=>{c.setPointerCapture(e.pointerId);ziehen={x:e.clientX,y:e.clientY,id:e.pointerId};c.style.cursor='grabbing';});
  c.addEventListener('pointermove',e=>{if(!ziehen||e.pointerId!==ziehen.id)return;yaw-=(e.clientX-ziehen.x)*.008;pitch=Math.max(-1.52,Math.min(1.52,pitch+(e.clientY-ziehen.y)*.008));ziehen.x=e.clientX;ziehen.y=e.clientY;});
  const los=()=>{ziehen=null;c.style.cursor='grab';};c.addEventListener('pointerup',los);c.addEventListener('pointercancel',los);
  c.addEventListener('wheel',e=>{e.preventDefault();abstand=Math.max(8,Math.min(60,abstand*Math.exp(e.deltaY*.001)));},{passive:false});
  d.addEventListener('click',e=>{const b=e.target.closest('button');if(!b)return;
   if(b.hasAttribute('data-wk-zu')){d.close();return;}
   if(b.hasAttribute('data-wk-start')){t=0;laeuft=true;t0=performance.now()/1000;return;}
   if(b.hasAttribute('data-wk-halt')){laeuft=!laeuft;t0=performance.now()/1000-t;b.textContent=laeuft?'⏸ Anhalten':'▶ Weiter';return;}
   if(b.dataset.wkAnsicht){[yaw,pitch]=ANSICHT[b.dataset.wkAnsicht];return;}
   if(b.hasAttribute('data-wk-gross-zu')){d.querySelector('.wk-gross').hidden=true;return;}});
  d.addEventListener('click',e=>{const i=e.target.closest('.wk-bilder img');if(!i)return;const g=d.querySelector('.wk-gross');g.querySelector('img').src=i.dataset.gross||i.src;g.hidden=false;});
  d.querySelector('[data-wk-volk]').addEventListener('change',e=>laden(e.target.value));
  d.querySelector('[data-wk-zeit]').addEventListener('input',e=>{laeuft=false;t=Number(e.target.value);});
  d.addEventListener('close',()=>{cancelAnimationFrame(frame);frame=0;});
 }
 const gesamtDauer=()=>m?.rig?m.ablauf.reduce((x,y)=>x+y[1],0):DAUER;
 function laden(k){
  volk=VOLK[k]?k:'krath';if(m){szene.remove(m.wurzel);m.wurzel.traverse(o=>{o.geometry?.dispose();});}
  m=modell(volk);m.wurzel.visible=false;/* erst zeigen, wenn klar ist, ob ein echtes Modell kommt */szene.add(m.wurzel);d.querySelector('[data-wk-volkname]').textContent=VOLK[volk].name;d.querySelector('[data-wk-volk]').value=volk;
  // Sobald das echte Modell geladen ist, ersetzt es die prozedurale Hülle (gleiche Segmente, gleiche Animation).
  /* Handy/Tablet: leichte Fassung (≈3 MB, flüssig); Desktop und Webseite: volle Detailtiefe */const mobil=matchMedia('(max-width: 820px)').matches||(navigator.maxTouchPoints>1&&Math.min(screen.width,screen.height)<900);const fuer=volk;ladeVersprechen=(mobil?rigLaden(fuer,'klein').then(r=>r||rigLaden(fuer)):rigLaden(fuer)).then(rig=>{if(volk!==fuer||!d)return;
   if(rig){szene.remove(m.wurzel);m=rigBauen(fuer,rig);szene.add(m.wurzel);const z=d.querySelector('[data-wk-zeit]');z.max=gesamtDauer().toFixed(1);return;}
   return Promise.all([echtesModell(fuer),echtesModell(fuer,'offen')]).then(([teile,offen])=>{if(!teile||volk!==fuer||!d)return;szene.remove(m.wurzel);m.wurzel.traverse(o=>{if(!teile.some(t=>t.g===o.geometry)&&!(offen||[]).some(t=>t.g===o.geometry))o.geometry?.dispose();});m=modell(fuer,teile,offen);szene.add(m.wurzel);});});ladeVersprechen=ladeVersprechen.finally(()=>{if(m)m.wurzel.visible=true;});
  // Bildleiste: Bau in der Mond-Orbitalwerft und die sechs Ansichten beider Zustände (megaships.<volk>.todesmond_*)
  const img=window.STERNEN_ART?.images||{},url=key=>img[`megaships|${key}|${volk}`]?.url,eintraege=[['todesmond_werft','Bau in der Mondwerft','wk-werft']];
  for(const z of ['geschlossen','offen'])for(const k of Object.keys(ANSICHT))eintraege.push([`todesmond_${z}_${k}`,`${z==='offen'?'Aufgeklappt':'Reiseform'} · ${k}`,'']);
  d.querySelector('.wk-bilder').innerHTML=eintraege.filter(([k])=>url(k)).map(([k,t,c])=>`<figure class="${c}"><img src="${url(k)}" alt="${t}" loading="lazy"><figcaption>${t}</figcaption></figure>`).join('');
 }
 // ---------------------------------------------------------------- Film „Mondzerstörung“ (Karl, 18.10.2026)
 // Ankunft aus dem Hyperraum, Abbremsen, Blick auf den echten Mond (Karte seines Mondtyps), Aufklappen, Aufladen,
 // 3 s Strahl mit Blickwechsel (Totale, dann Einschlag nah), Oberfläche schmilzt, 3 s nach Schussende Lichtblitz in
 // Strahlfarbe, der Mond zerbricht. Gleiche Bausteine wie in der Sonnensystemansicht (mondzerstoerung.js).
 const FILM={abstand:72,mondRadius:10,ankunft:2.4,blick:2.4,nachBruch:9};
 function filmZeiten(){
  const a=m.ablauf,bis=n=>a.slice(0,a.findIndex(x=>x[0]===n)).reduce((s,x)=>s+x[1],0),rig0=FILM.ankunft+FILM.blick;
  const feuer=rig0+bis('feuer'),ende=feuer+3,bruch=ende+3;return {rig0,feuer,ende,bruch,laden:rig0+bis('laden'),offen:rig0+bis('offen'),gesamt:bruch+FILM.nachBruch};
 }
 function filmAufbauen(mondTyp){
  if(!window.Mondzerstoerung||!m?.rig)return null;
  if(film?.mond){szene.remove(film.mond.objekt);film.mond.entsorgen();}
  const mond=window.Mondzerstoerung.erzeugen({typ:mondTyp,radius:FILM.mondRadius,farbe:VOLK[volk].strahl,saat:7});
  mond.objekt.position.set(0,0,FILM.abstand);mond.treffer(new THREE.Vector3(0,0,-1));szene.add(mond.objekt);
  m.strahlLaenge=FILM.abstand-FILM.mondRadius*.97-m.nase.z;szene.fog=null;
  const glanz=new THREE.SpriteMaterial({map:m.kernGlut.material.map,color:new THREE.Color(VOLK[volk].strahl),transparent:true,depthWrite:false,blending:THREE.AdditiveBlending,opacity:0});
  const austritt=new THREE.Sprite(glanz);szene.add(austritt);
  const mondLicht=new THREE.DirectionalLight('#fff1dc',2.4);mondLicht.position.set(45,28,FILM.abstand-55);mondLicht.target=mond.objekt;szene.add(mondLicht);
  return {mond,typ:mondTyp,austritt,mondLicht,t0:performance.now()/1000};
 }
 const weich=x=>x<=0?0:x>=1?1:x*x*(3-2*x);
 function filmSchritt(ft,jetzt,ruhig=false){
  const z0=filmZeiten(),rigT=Math.max(0,ft-z0.rig0),zust=anwenden(m,Math.min(rigT,gesamtDauer()-.01),jetzt,kamera);
  // Hyperraum: aus großer Ferne heranrasen, gestreckt, dann hart abbremsen; Austrittsblitz
  const an=Math.min(1,ft/FILM.ankunft),wz=-900*Math.pow(1-an,3.2);m.wurzel.rotation.y=0;m.wurzel.position.set(0,0,wz);
  m.wurzel.scale.set(1,1,1+18*Math.pow(1-an,4));
  film.austritt.position.set(0,0,wz+R*1.5);film.austritt.material.opacity=ft<FILM.ankunft+.4?Math.max(0,Math.sin(Math.min(1,ft/(FILM.ankunft+.4))*Math.PI))*.9:0;film.austritt.scale.setScalar(R*(4+8*(1-an)));
  // Mond: Schmelzen während des Strahls, Risse in den 3 s danach, dann Bruch
  const schmelz=weich((ft-z0.feuer)/3),riss=weich((ft-z0.ende)/3),bruch=ft>=z0.bruch?ft-z0.bruch:-1;
  film.mond.setze({schmelz,riss,bruch,zeit:jetzt});
  if(bruch>=0)m.strahlen.forEach(s=>{s.visible=false;});
  // Kamera: feste Einstellungen mit Schnitten, weiche Fahrt innerhalb einer Einstellung
  const M=new THREE.Vector3(0,0,FILM.abstand),W=m.wurzel.position,p=new THREE.Vector3(),ziel=new THREE.Vector3();let wackeln=0;
  if(ft<FILM.ankunft){p.set(26,7,FILM.abstand-22);ziel.copy(W).lerp(M,.05);}
  else if(ft<z0.rig0){const u=(ft-FILM.ankunft)/FILM.blick;p.set(8-2*u,2.2,-16+3*u);ziel.copy(M);}
  else if(ft<z0.laden){const w=.5+(ft-z0.rig0)*.22;p.set(Math.sin(w)*18,6-(ft-z0.rig0)*.2,Math.cos(w)*-18+3);ziel.set(0,0,1.5);}
  else if(ft<z0.feuer){const u=(ft-z0.laden)/(z0.feuer-z0.laden);p.set(9-3*u,2.5,17+4*u);ziel.set(0,0,4);wackeln=u*.04;}
  else if(ft<z0.feuer+1.5){p.set(46,9,FILM.abstand*.42);ziel.set(0,0,FILM.abstand*.5);wackeln=.12;}
  else if(ft<z0.ende){const u=(ft-z0.feuer-1.5)/1.5;p.set(12-2*u,6,FILM.abstand-FILM.mondRadius-17+2*u);ziel.set(0,0,FILM.abstand-FILM.mondRadius);wackeln=.18;}
  else if(ft<z0.bruch){const u=(ft-z0.ende)/3;p.set(30-6*u,9,FILM.abstand-40+6*u);ziel.copy(M);wackeln=u*.06;}
  else{const u=Math.min(1,(ft-z0.bruch)/FILM.nachBruch);p.set(52+18*u,16+5*u,FILM.abstand-68-12*u);ziel.copy(M).lerp(W,.3);wackeln=Math.max(0,.6-(ft-z0.bruch)*.4);}
  if(ruhig)wackeln=0;
  if(kamera.aspect<1){/* Hochformat (Handy): Blickpunkt zum Motiv ziehen und weiter zurück, damit nichts am Rand abgeschnitten wird */const motiv=ziel.distanceTo(M)<ziel.distanceTo(W)?M:W;ziel.lerp(motiv,.65);const f=Math.pow(1/kamera.aspect,.9);p.sub(ziel).multiplyScalar(f).add(ziel);}/* Hochformat (Handy): weiter zurück, damit alles im Bild bleibt */
  kamera.position.copy(p).add(new THREE.Vector3((Math.random()-.5)*wackeln,(Math.random()-.5)*wackeln,(Math.random()-.5)*wackeln));kamera.lookAt(ziel);
  return {...zust,ft,gesamt:z0.gesamt};
 }
 function bild(){
  frame=requestAnimationFrame(bild);if(!d.open)return;
  const c=renderer.domElement,w=c.clientWidth,h=c.clientHeight;if(c.width!==Math.round(w*renderer.getPixelRatio())||c.height!==Math.round(h*renderer.getPixelRatio())){renderer.setSize(w,h,false);kamera.aspect=w/Math.max(1,h);kamera.updateProjectionMatrix();}
  const jetzt=performance.now()/1000;if(laeuft){t=jetzt-t0;if(t>gesamtDauer()){if(schleife){t0=jetzt;t=0;}else{t=gesamtDauer();laeuft=false;}}}if(kameraFahrt&&!ziehen)yaw+=.0016;
  if(film&&m?.rig){if(!film.mond){const f=filmAufbauen(film.typ);if(f)film=f;else{renderer.render(szene,kamera);return;}}
   let ft=jetzt-film.t0;const g=filmZeiten().gesamt;if(ft>g){if(schleife){film.t0=jetzt;ft=0;}else ft=g;}
   const fz=filmSchritt(ft,jetzt);d.querySelector('.wk-phase').textContent=ft<FILM.ankunft?'Austritt aus dem Hyperraum':fz.ft>=filmZeiten().bruch?'Mond zerstört':(NAMEN[fz.name]||'');
   renderer.render(szene,kamera);return;}
  const z=anwenden(m,t,jetzt,kamera);
  // Planetenform dreht sich langsam um sich selbst
  const dt=letzt?Math.min(.1,jetzt-letzt):0;letzt=jetzt;if(!standbildModus)drehung+=dt*.12*Math.max(0,1-z.auf*4);m.wurzel.rotation.y=drehung;
  const zittern=d.classList.contains('wk-nurbild')?0:z.feuer*.05,ab=abstand*(1+z.auf*.45);kamera.position.set(Math.sin(yaw)*Math.cos(pitch)*ab+(Math.random()-.5)*zittern,Math.sin(pitch)*ab+(Math.random()-.5)*zittern,Math.cos(yaw)*Math.cos(pitch)*ab);kamera.lookAt(0,0,0);
  d.querySelector('.wk-phase').textContent=m.echt&&!m.offen?'Reiseform · die offene Form wird gerade gebaut':(NAMEN[z.name]||'');const regler=d.querySelector('[data-wk-zeit]');if(laeuft)regler.value=t.toFixed(2);
  renderer.render(szene,kamera);
 }
 function oeffnen(k='krath',sofort=true,opt={}){
  if(typeof THREE==='undefined')return false;
  try{if(!d)aufbauen();}catch(e){console.warn('Weltenkrümmer 3D nicht verfügbar',e);return false;}
  laden(k);t=opt.zeit??0;laeuft=sofort;t0=performance.now()/1000-t;yaw=.7;pitch=.25;abstand=13;if(opt.ansicht)[yaw,pitch]=ANSICHT[opt.ansicht];
  // Nur-Bild-Modus (Standbilder der Ansichten): ganze Fläche, keine Bedienung, Signal sobald das echte Modell steht
  standbildModus=!!opt.nurBild;schleife=!!opt.schleife;if(film?.mond){szene.remove(film.mond.objekt);szene.remove(film.austritt);szene.remove(film.mondLicht);film.mond.entsorgen();}if(m){m.strahlLaenge=0;m.wurzel.position.set(0,0,0);m.wurzel.scale.set(1,1,1);}film=opt.film?{typ:opt.film.mond||'krater_grau',t0:performance.now()/1000}:null;if(!film&&szene&&!szene.fog)szene.fog=new THREE.Fog('#02050b',60,260);kameraFahrt=!!opt.kameraFahrt;if(opt.vitrine)d.classList.add('wk-nurbild','wk-vitrine');drehung=0;if(opt.nurBild){d.classList.add('wk-nurbild');window.__wkFertig=false;echtesModell(k).then(()=>setTimeout(()=>{window.__wkFertig=true;},1200));}
  if(!d.open)d.showModal();if(!frame)frame=requestAnimationFrame(bild);return true;
 }
  // Klick auf ein Todesmond-/Weltenkrümmer-Bild öffnet den 3D-Betrachter des passenden Volks.
 /* Nur große Bilder öffnen den 3D-Dialog – nie Kacheln/Knöpfe, mit denen man auswählt oder baut (Mondabnahme 18.10.) */document.addEventListener('click',e=>{const img=e.target.closest?.('img');if(!img||img.closest('.weltenkruemmer-dialog,button,a,label,[role="button"],[data-mond-einheit],[data-key],[data-map],.bs-kachel'))return;const src=img.getAttribute('src')||'';const hit=src.match(/megaships\.(aurelianer|krath|syntheten|veyari)\.todesmond/);if(!hit||e.defaultPrevented)return;if(oeffnen(hit[1]))e.preventDefault();});
 /** Eine Ansicht direkt rendern (ohne Bildschleife) und als PNG-Daten-URL liefern – für konsistente Standbilder. */
 async function standbildPng(k,{zustand='geschlossen',ansicht='vorn',groesse=1024,zeitpunkt=null,breite=null,hoehe=null,weit=1,blick=0}={}){
  oeffnen(k,false,{nurBild:true,ansicht,zeit:0});await ladeVersprechen;if(!m.rig)await Promise.all([echtesModell(k),echtesModell(k,'offen')]);await new Promise(r=>setTimeout(r,600));
  const liste=m.rig?m.ablauf:ABLAUF,bis=n=>liste.slice(0,liste.findIndex(x=>x[0]===n)).reduce((s,x)=>s+x[1],0),zeit=zeitpunkt!=null?zeitpunkt:zustand==='offen'?bis('offen')+.4:.2;t=zeit;
  cancelAnimationFrame(frame);frame=0;
  const alt=[renderer.domElement.width,renderer.domElement.height];renderer.setPixelRatio(1);const bw=breite||groesse,bh=hoehe||groesse;renderer.setSize(bw,bh,false);kamera.aspect=bw/bh;kamera.updateProjectionMatrix();
  const z=anwenden(m,zeit,zeit,kamera);[yaw,pitch]=ANSICHT[ansicht];const ab=abstand*(1+z.auf*.45)*weit;
  /* blick > 0 schiebt Kamera und Blickpunkt in Schussrichtung (+z): Schiff an den Rand, Strahl ins Bild */
  kamera.position.set(Math.sin(yaw)*Math.cos(pitch)*ab,Math.sin(pitch)*ab,Math.cos(yaw)*Math.cos(pitch)*ab+blick);kamera.lookAt(0,0,blick);
  renderer.render(szene,kamera);const url=renderer.domElement.toDataURL('image/png');
  renderer.setSize(alt[0],alt[1],false);return url;
 }
 /** Ein Bild des Mondzerstörungs-Films (für GIF/Video und Prüfung). */
 async function filmBildPng(k,{mond='krater_grau',zeit=0,breite=960,hoehe=540}={}){
  const bereit=film?.mond&&volk===k&&film.typ===mond&&d?.open;
  if(!bereit){oeffnen(k,false,{nurBild:true,film:{mond}});await ladeVersprechen;await new Promise(r=>setTimeout(r,400));
   if(!film.mond){const f=filmAufbauen(film.typ);if(f)film=f;}
   await new Promise(r=>setTimeout(r,film.mond?600:0));}
  cancelAnimationFrame(frame);frame=0;if(renderer.domElement.width!==breite||renderer.domElement.height!==hoehe){renderer.setPixelRatio(1);renderer.setSize(breite,hoehe,false);kamera.aspect=breite/hoehe;kamera.updateProjectionMatrix();}
  const fz=filmSchritt(zeit,zeit,true);renderer.render(szene,kamera);return {url:renderer.domElement.toDataURL('image/png'),gesamt:fz.gesamt};
 }
 const _probe=t=>{if(!m?.rig)return null;rigAnwenden(m,t,t);const k=m.wurzel.children[0].children;return k.slice(0,3).map(o=>o.name+":"+o.position.toArray().map(x=>x.toFixed(2)).join(","));};
 const _diagnose=()=>({rig:!!m?.rig,dauer:m?.dauer,spuren:m?.aktion?._clip?.tracks?.length||0,knoten:m?.rig?m.wurzel.children[0]?.children?.length:0});
 /** Eigenständige Instanz für andere Szenen (3D-Sonnensystem): {objekt, dauer, ablauf, setze(t, zeitSek)}.
  *  stufe 'klein' = leichte Detailstufe; ohne Rig null. Die Instanz teilt Geometrie/Materialien mit dem Cache. */
 async function instanz(k,stufe='klein'){
  const rig=await rigLaden(k,stufe)||await rigLaden(k);if(!rig)return null;
  const szeneKopie=rig.szene.clone(true),kopie={...rig,szene:szeneKopie,kernKnoten:rig.kernKnoten?szeneKopie.getObjectByName(rig.kernKnoten.name):null,muendungKnoten:rig.muendungKnoten?szeneKopie.getObjectByName(rig.muendungKnoten.name):null,benannt:rig.benannt?Object.fromEntries(Object.entries(rig.benannt).map(([n,o])=>[n,szeneKopie.getObjectByName(o.name)])):null};
  const mi=rigBauen(k,kopie),bis=n=>mi.ablauf.slice(0,mi.ablauf.findIndex(x=>x[0]===n)).reduce((s,x)=>s+x[1],0);
  /* strahl(l): Strahllaenge in Objekt-Einheiten (endet am Ziel); bis(phase): Startzeit einer Phase; nase: Muendung lokal */
  return {objekt:mi.wurzel,dauer:mi.ablauf.reduce((x,y)=>x+y[1],0),ablauf:mi.ablauf,bis,nase:mi.nase.clone(),farbe:VOLK[k]?.strahl||'#ff3b22',strahl:l=>{mi.strahlLaenge=l;},setze:(t,zeit)=>rigAnwenden(mi,t,zeit)};
 }
 return {instanz,filmBildPng,_probe,_glbSzene:glbSzene,_diagnose,oeffnen,standbildPng,VOLK,ABLAUF,DAUER,phaseBei,_modell:modell,_anwenden:anwenden};
})();
