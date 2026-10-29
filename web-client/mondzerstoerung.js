/* Mondzerstörung durch einen Weltenkrümmer (Karl, 18.10.2026).
 * Der Mond ist immer der echte Mond: Oberfläche aus seiner Mondkarte (PlanetKugel.karte('moons', typ)), daher
 * austauschbar. Er ist vorab in viele Bruchstücke zerlegt (Voronoi-Keile mit zerklüfteten Innenflächen), die zusammen
 * die Kugel bilden. Ablauf (gesteuert über setze()):
 *   Strahl trifft → Krater mit Wall sinkt ein, Risse laufen durch den Kraterboden, darunter wandert Laserlicht wie
 *   Sonnenlicht in einem Pool (Kaustik) → glühende Bruchlinien über die ganze Kugel → Lichtblitz in Strahlfarbe mit
 *   Strahlenkranz → Plasmahülle quillt auf, verwirbelte Druckwelle, Funkenschauer, Staubwolke, die Brocken fliegen.
 * Wird im Film des 3D-Betrachters (weltenkruemmer.js) und in der Sonnensystemansicht (galaxy.js) genutzt. */
(function(){
 'use strict';
 if(typeof THREE==='undefined')return;
 const zufallVon=s=>()=>{s=(s*1664525+1013904223)>>>0;return s/4294967296;};
 const kartenUrl=typ=>window.PlanetKugel?.karte?.('moons',typ)||(typ?new URL(`/content/assets/moons/moons.${typ}.map.png`,location.href).href:'');
 const texturCache=new Map();
 function textur(typ){
  const url=kartenUrl(typ);if(!url)return null;if(texturCache.has(url))return texturCache.get(url);
  const t=new THREE.TextureLoader().load(url);t.colorSpace=THREE.SRGBColorSpace;t.anisotropy=8;texturCache.set(url,t);return t;
 }
 const leinwand=(b,h,malen)=>{const c=document.createElement('canvas');c.width=b;c.height=h;malen(c.getContext('2d'),b,h);const t=new THREE.CanvasTexture(c);t.colorSpace=THREE.SRGBColorSpace;return t;};
 const glanzTextur=()=>leinwand(128,128,(g)=>{const r=g.createRadialGradient(64,64,0,64,64,64);r.addColorStop(0,'rgba(255,255,255,1)');r.addColorStop(.22,'rgba(255,255,255,.55)');r.addColorStop(1,'rgba(255,255,255,0)');g.fillStyle=r;g.fillRect(0,0,128,128);});
 // Strahlenkranz des Blitzes: viele dünne, unterschiedlich lange Strahlen
 const strahlenTextur=saat=>leinwand(512,512,(g)=>{const z=zufallVon(saat);g.translate(256,256);g.globalCompositeOperation='lighter';
  for(let i=0;i<46;i++){const w=z()*Math.PI*2,l=90+z()*165,b=1.2+z()*3.2,gr=g.createLinearGradient(0,0,Math.cos(w)*l,Math.sin(w)*l);gr.addColorStop(0,'rgba(255,255,255,.9)');gr.addColorStop(1,'rgba(255,255,255,0)');
   g.strokeStyle=gr;g.lineWidth=b;g.beginPath();g.moveTo(0,0);g.lineTo(Math.cos(w)*l,Math.sin(w)*l);g.stroke();}});
 // breiter Lichtstreif (anamorph)
 const streifTextur=()=>leinwand(512,64,(g)=>{const gr=g.createLinearGradient(0,0,512,0);gr.addColorStop(0,'rgba(255,255,255,0)');gr.addColorStop(.5,'rgba(255,255,255,1)');gr.addColorStop(1,'rgba(255,255,255,0)');
  g.fillStyle=gr;g.fillRect(0,0,512,64);g.globalCompositeOperation='destination-in';const v=g.createLinearGradient(0,0,0,64);v.addColorStop(0,'rgba(0,0,0,0)');v.addColorStop(.5,'rgba(0,0,0,1)');v.addColorStop(1,'rgba(0,0,0,0)');g.fillStyle=v;g.fillRect(0,0,512,64);});
 // weicher, unregelmäßiger Rauchballen
 const rauchTextur=saat=>leinwand(256,256,(g)=>{const z=zufallVon(saat);for(let i=0;i<46;i++){const x=128+(z()-.5)*120,y=128+(z()-.5)*120,r=26+z()*64,gr=g.createRadialGradient(x,y,0,x,y,r);
  gr.addColorStop(0,'rgba(255,255,255,.16)');gr.addColorStop(1,'rgba(255,255,255,0)');g.fillStyle=gr;g.fillRect(0,0,256,256);}});

 const RAUSCHEN=`
float hz(vec3 p){return fract(sin(dot(p,vec3(12.9898,78.233,37.719)))*43758.5453);}
float rn(vec3 p){vec3 i=floor(p),f=fract(p);f=f*f*(3.-2.*f);return mix(mix(mix(hz(i),hz(i+vec3(1,0,0)),f.x),mix(hz(i+vec3(0,1,0)),hz(i+vec3(1,1,0)),f.x),f.y),mix(mix(hz(i+vec3(0,0,1)),hz(i+vec3(1,0,1)),f.x),mix(hz(i+vec3(0,1,1)),hz(i+vec3(1,1,1)),f.x),f.y),f.z);}
float fbm(vec3 p){float s=0.,a=.5;for(int i=0;i<5;i++){s+=a*rn(p);p=p*2.03+vec3(1.7,9.2,3.1);a*=.5;}return s;}`;

 /** Zerlegte Mondkugel (Einheitsradius) – Geometrie je Bruchstück, Mittelpunkt im Ursprung des Stücks. */
 function zerlegen(anzahl,feinheit,saat){
  const zufall=zufallVon(saat),kugel=new THREE.IcosahedronGeometry(1,feinheit).toNonIndexed(),pos=kugel.attributes.position,nTri=pos.count/3;
  // Keime ungleichmäßig: in Haufen dichter (kleine Splitter) und dazwischen große Schollen
  const keime=[];for(let i=0;i<anzahl;i++){const u=zufall()*2-1,w=zufall()*Math.PI*2,s=Math.sqrt(1-u*u);const v=new THREE.Vector3(s*Math.cos(w),u,s*Math.sin(w));keime.push(v);
   if(zufall()<.35&&keime.length<anzahl){const n=v.clone().add(new THREE.Vector3(zufall()-.5,zufall()-.5,zufall()-.5).multiplyScalar(.25)).normalize();keime.push(n);i++;}}
  const a=new THREE.Vector3(),b=new THREE.Vector3(),c=new THREE.Vector3(),m=new THREE.Vector3(),stueckVon=new Int32Array(nTri);
  for(let t=0;t<nTri;t++){a.fromBufferAttribute(pos,3*t);b.fromBufferAttribute(pos,3*t+1);c.fromBufferAttribute(pos,3*t+2);m.copy(a).add(b).add(c).normalize();
   // leicht verrauschte Zuordnung -> unregelmäßige, gezackte Bruchkanten statt gerader Voronoi-Linien
   const st=Math.sin(m.x*23.1+m.y*17.3)*.035+Math.sin(m.z*31.7-m.x*13.9)*.02;
   let best=0,bd=-9;for(let k=0;k<keime.length;k++){const d=m.dot(keime[k])+st*((k%3)-1);if(d>bd){bd=d;best=k;}}stueckVon[t]=best;}
  const schl=v=>`${Math.round(v.x*1e4)},${Math.round(v.y*1e4)},${Math.round(v.z*1e4)}`,innenPunkt=new Map();
  const kanten=Array.from({length:keime.length},()=>new Map());
  for(let t=0;t<nTri;t++)for(let e=0;e<3;e++){a.fromBufferAttribute(pos,3*t+e);b.fromBufferAttribute(pos,3*t+(e+1)%3);const k=[schl(a),schl(b)].sort().join('|'),mp=kanten[stueckVon[t]],x=mp.get(k);if(x)x.n++;else mp.set(k,{n:1,a:a.clone(),b:b.clone()});}
  const stuecke=[];
  for(let k=0;k<keime.length;k++){
   const p=[],orig=[],innen=[],rand=[],randPunkte=new Set();let aussenAnzahl=0;
   for(const e of kanten[k].values())if(e.n===1){randPunkte.add(schl(e.a));randPunkte.add(schl(e.b));}
   const mitte=keime[k].clone().multiplyScalar(.15+zufall()*.2);
   for(let t=0;t<nTri;t++){if(stueckVon[t]!==k)continue;for(let e=0;e<3;e++){a.fromBufferAttribute(pos,3*t+e);p.push(a.x,a.y,a.z);orig.push(a.x,a.y,a.z);innen.push(0);rand.push(randPunkte.has(schl(a))?1:0);aussenAnzahl++;}}
   if(!aussenAnzahl)continue;
   // Bruchflächen: Randpunkt -> zwei verrauschte Tiefenstufen (gemeinsam für beide Nachbarstücke) -> Keilmitte
   const tief=(v,stufe)=>{const h=schl(v)+'/'+stufe;if(innenPunkt.has(h))return innenPunkt.get(h);let x=0;for(let i=0;i<h.length;i++)x=(x*31+h.charCodeAt(i))>>>0;const z=zufallVon(x);
    const w=v.clone().multiplyScalar(stufe===1?.8+z()*.08:.52+z()*.14).add(new THREE.Vector3(z()-.5,z()-.5,z()-.5).multiplyScalar(stufe===1?.05:.11));innenPunkt.set(h,w);return w;};
   const drei=(a,b,c,ia,ib,ic)=>{for(const [v,i] of [[a,ia],[b,ib],[c,ic]]){p.push(v.x,v.y,v.z);orig.push(v.x,v.y,v.z);innen.push(i);rand.push(1);}};
   for(const e of kanten[k].values()){if(e.n!==1)continue;const a1=tief(e.a,1),b1=tief(e.b,1),a2=tief(e.a,2),b2=tief(e.b,2);
    drei(e.b,e.a,a1,.3,.3,.55);drei(e.b,a1,b1,.3,.55,.55);drei(b1,a1,a2,.55,.55,.8);drei(b1,a2,b2,.55,.8,.8);drei(b2,a2,mitte,.8,.8,1);}
   const g=new THREE.BufferGeometry();g.setAttribute('position',new THREE.Float32BufferAttribute(p,3));g.setAttribute('orig',new THREE.Float32BufferAttribute(orig,3));
   g.setAttribute('innen',new THREE.Float32BufferAttribute(innen,1));g.setAttribute('bruchrand',new THREE.Float32BufferAttribute(rand,1));
   const schwer=keime[k].clone().multiplyScalar(.62);g.translate(-schwer.x,-schwer.y,-schwer.z);g.computeVertexNormals();
   // Außenhaut mit glatten Kugelnormalen (keine sichtbaren Facetten), Bruchflächen bleiben kantig
   const nrm=g.attributes.normal;for(let i=0;i<aussenAnzahl;i++){const o=3*i;const l=Math.hypot(orig[o],orig[o+1],orig[o+2]);nrm.setXYZ(i,orig[o]/l,orig[o+1]/l,orig[o+2]/l);}
   const groesse=Math.cbrt(aussenAnzahl/(nTri*3/keime.length));
   stuecke.push({g,schwer,richtung:keime[k].clone(),tempo:(.55+zufall()*.9)/Math.max(.6,groesse),dreh:new THREE.Vector3(zufall()-.5,zufall()-.5,zufall()-.5).normalize(),drehTempo:(.4+zufall()*1.6)/Math.max(.6,groesse)});
  }
  kugel.dispose();return stuecke;
 }

 function material(karte,farbe){
  const m=new THREE.MeshStandardMaterial({map:karte,color:karte?'#ffffff':'#8a8580',roughness:.95,metalness:0,side:THREE.DoubleSide});
  const u={uTreffer:{value:new THREE.Vector3(0,0,-1)},uSchmelz:{value:0},uRiss:{value:0},uHitze:{value:0},uFarbe:{value:new THREE.Color(farbe)},uZeit:{value:0},uKrater:{value:.17}};
  m.userData.u=u;
  m.onBeforeCompile=sh=>{Object.assign(sh.uniforms,u);
   sh.vertexShader=sh.vertexShader.replace('#include <common>',`#include <common>
attribute vec3 orig;attribute float innen;attribute float bruchrand;varying vec3 vOrig;varying float vInnen;varying float vRand;varying vec3 vRadSicht;varying float vNeig;
uniform vec3 uTreffer;uniform float uSchmelz;uniform float uKrater;
// Kraterprofil über dem Winkel zum Einschlag: Schüssel mit aufgeworfenem Wall
float kraterHoehe(float a,float rc){float x=a/max(rc,1e-4);return (x<1.?-.075*(1.-x*x):0.)+.022*exp(-pow((x-1.05)/.22,2.));}`)
    .replace('#include <uv_vertex>',`#include <uv_vertex>
vOrig=orig;vInnen=innen;vRand=bruchrand;
#ifdef USE_MAP
vMapUv=vec2(atan(orig.z,orig.x)/6.2831853+.5,asin(clamp(orig.y,-1.,1.))/3.1415927+.5);
#endif`)
    .replace('#include <begin_vertex>',`#include <begin_vertex>
vec3 dK=normalize(orig);float aK=acos(clamp(dot(dK,uTreffer),-1.,1.)),rcK=uKrater*smoothstep(0.,1.,uSchmelz);
float hK=kraterHoehe(aK,rcK)*(1.-step(.01,innen))*step(.001,uSchmelz);transformed+=dK*hK;
vec3 radK=normalize(dK-uTreffer*dot(dK,uTreffer)+1e-5);vRadSicht=normalize(normalMatrix*radK);
vNeig=(kraterHoehe(aK+.004,rcK)-kraterHoehe(aK-.004,rcK))/.008*(1.-step(.01,innen))*step(.001,uSchmelz);`);
   sh.fragmentShader=sh.fragmentShader.replace('#include <common>',`#include <common>
varying vec3 vOrig;varying float vInnen;varying float vRand;varying vec3 vRadSicht;varying float vNeig;
uniform vec3 uTreffer;uniform float uSchmelz;uniform float uRiss;uniform float uHitze;uniform vec3 uFarbe;uniform float uZeit;uniform float uKrater;
${RAUSCHEN}
// Kaustik wie Sonnenlicht im Pool: zwei langsam gegeneinander driftende Rauschfelder, hell wo sie sich treffen
float kaustik(vec3 p,float t){float a=fbm(p+vec3(t*.31,-t*.17,t*.23)),b=fbm(p*1.37+vec3(-t*.21,t*.27,-t*.13)+4.1);
 float c=pow(1.-min(1.,abs(a-b)*3.2),7.);float a2=fbm(p*2.1+vec3(t*.4,t*.12,-t*.3)+9.),b2=fbm(p*2.6-vec3(t*.18,t*.33,t*.21)+2.);return c+.55*pow(1.-min(1.,abs(a2-b2)*3.6),9.);}`)
    .replace('#include <normal_fragment_maps>',`#include <normal_fragment_maps>
normal=normalize(normal-vRadSicht*clamp(vNeig,-3.,3.)*.9);
if(vInnen>.01){vec3 q=vOrig*26.;normal=normalize(normal+(vec3(fbm(q),fbm(q+5.2),fbm(q+9.7))-.5)*1.6);}`)
    .replace('#include <emissivemap_fragment>',`#include <emissivemap_fragment>
{vec3 d=normalize(vOrig);float w=dot(d,uTreffer),ang=acos(clamp(w,-1.,1.)),n=rn(d*9.+uZeit*.6)*.6+rn(d*23.)*.4,aussen=1.-step(.01,vInnen),an=step(.001,uSchmelz);
 float rc=uKrater*smoothstep(0.,1.,uSchmelz),x=ang/max(rc,1e-4);
 // Kraterboden: Schmelze, dunkler verbrannter Hof
 float boden=1.-smoothstep(.55,1.,x),hof=smoothstep(2.6,1.,x)*(1.-boden);
 vec3 lava=mix(vec3(.85,.16,.03),vec3(1.,.6,.22),n*n);
 totalEmissiveRadiance+=mix(lava,uFarbe,.35)*boden*(.35+.55*n)*an*aussen*1.2;
 diffuseColor.rgb*=1.-hof*.7*an;
 // Risse durch den Kraterboden und weit hinaus: verästelte Grate im Winkel um die Einschlagachse
 vec3 t1=normalize(cross(uTreffer,abs(uTreffer.y)<.9?vec3(0,1,0):vec3(1,0,0))),t2=cross(uTreffer,t1);float th=atan(dot(d,t2),dot(d,t1));
 float grat=1.-abs(fbm(vec3(cos(th)*2.4,sin(th)*2.4,ang*7.))*2.-1.),grat2=1.-abs(fbm(vec3(d*11.)+3.)*2.-1.);
 float reichweite=rc*(1.4+uSchmelz*1.4+uRiss*2.2),rissKrater=smoothstep(.9,.985,grat)*smoothstep(reichweite,reichweite*.55,ang)+smoothstep(.95,.995,grat2)*smoothstep(reichweite*1.2,reichweite*.6,ang)*.7;
 totalEmissiveRadiance+=mix(vec3(1.,.42,.1),uFarbe,.5)*rissKrater*an*aussen*(1.2+n)*1.6;
 // Pool-Licht: Laserlicht unter der Oberfläche, wellig und unruhig, um den Krater herum (später über die ganze Kugel)
 float pool=smoothstep(reichweite*1.35,rc*.3,ang)*an;
 float ka=kaustik(d*7.,uZeit);totalEmissiveRadiance+=uFarbe*(ka*1.1+.08)*pool*aussen*(.45+.55*uSchmelz);
 diffuseColor.rgb=mix(diffuseColor.rgb,diffuseColor.rgb*.6+uFarbe*.25*ka,pool*.5*aussen);
 // glühende Bruchlinien vor dem Bruch
 float riss=smoothstep(.85,1.,vRand)*uRiss*(.4+.6*n)*aussen;totalEmissiveRadiance+=mix(vec3(1.,.42,.1),uFarbe,.5)*riss*1.05;
 // Innenflächen: dunkles Gestein, glühend direkt nach dem Bruch
 float innenJa=step(.01,vInnen),ader=smoothstep(.55,.8,fbm(vOrig*9.+vec3(3.)))*.8+smoothstep(.62,.9,fbm(vOrig*21.))*.6;
 totalEmissiveRadiance+=mix(vec3(1.,.22,.03),uFarbe,.15)*innenJa*(uHitze*1.4+uRiss*.4)*ader*(.6+.4*n);
 diffuseColor.rgb=mix(diffuseColor.rgb,vec3(.035,.03,.028)*(.6+.9*fbm(vOrig*17.)),innenJa);}`);
  };
  return m;
 }

 // Plasmahülle: aufquellende, turbulente Lichthaut mit weichem Rand (Fresnel), additiv
 function plasmaMaterial(farbe){
  return new THREE.ShaderMaterial({transparent:true,depthWrite:false,blending:THREE.AdditiveBlending,side:THREE.DoubleSide,
   uniforms:{uZeit:{value:0},uAlter:{value:0},uStaerke:{value:0},uFarbe:{value:new THREE.Color(farbe)}},
   vertexShader:`varying vec3 vN,vV,vP;void main(){vP=position;vec4 mv=modelViewMatrix*vec4(position,1.);vN=normalize(normalMatrix*normal);vV=normalize(-mv.xyz);gl_Position=projectionMatrix*mv;}`,
   fragmentShader:`uniform float uZeit,uAlter,uStaerke;uniform vec3 uFarbe;varying vec3 vN,vV,vP;${RAUSCHEN}
void main(){float f=pow(1.-abs(dot(vN,vV)),1.6);float n=fbm(vP*2.6+vec3(0.,0.,uZeit*.5)-uAlter*.8);float m=smoothstep(.35,.85,n);
 vec3 c=mix(vec3(1.,.96,.88),uFarbe,smoothstep(.0,.9,uAlter));c=mix(c,vec3(1.,.45,.12),smoothstep(.8,2.4,uAlter)*.6);
 float a=(f*.9+.25)*m*uStaerke;gl_FragColor=vec4(c*a*(1.1+n),a);}`});
 }
 // Druckwelle: verwirbelte, weiche Gasscheibe statt harter Ringlinie
 function welleMaterial(farbe){
  return new THREE.ShaderMaterial({transparent:true,depthWrite:false,blending:THREE.AdditiveBlending,side:THREE.DoubleSide,
   uniforms:{uZeit:{value:0},uStaerke:{value:0},uFarbe:{value:new THREE.Color(farbe)}},
   vertexShader:`varying vec2 vUv;void main(){vUv=uv;gl_Position=projectionMatrix*modelViewMatrix*vec4(position,1.);}`,
   fragmentShader:`uniform float uZeit,uStaerke;uniform vec3 uFarbe;varying vec2 vUv;${RAUSCHEN}
void main(){vec2 p=vUv*2.-1.;float r=length(p),w=atan(p.y,p.x);float verz=(fbm(vec3(cos(w)*2.,sin(w)*2.,uZeit*.4))-.5)*.22;
 float band=exp(-pow((r-.74-verz)/.11,2.)),dunst=smoothstep(.95,.2,r)*.18;float wolke=fbm(vec3(p*5.,uZeit*.3));
 float a=(band*(.35+.9*wolke)+dunst*wolke)*uStaerke*smoothstep(1.,.92,r);vec3 c=mix(uFarbe,vec3(1.,.9,.8),band*.35);gl_FragColor=vec4(c*a,a);}`});
 }

 /**
  * Neuer zerstörbarer Mond.
  * opt: {typ (Mondtyp der Karte), radius, farbe (Strahlfarbe), stuecke, feinheit, saat, funken}
  * Rückgabe: {objekt, radius, treffer(richtungLokal), setze({schmelz, riss, bruch, zeit, ring?}), entsorgen()}
  *   schmelz 0..1 Kraterfortschritt, riss 0..1 Rissglühen vor dem Bruch, bruch Sekunden seit dem Blitz (<0 = heil)
  */
 function erzeugen(opt={}){
  const radius=opt.radius||6,farbe=new THREE.Color(opt.farbe||'#ff3b22'),wurzel=new THREE.Group(),saat=opt.saat||7;
  const mat=material(textur(opt.typ),farbe),stuecke=zerlegen(opt.stuecke||90,opt.feinheit??40,saat);
  const teile=stuecke.map(s=>{const me=new THREE.Mesh(s.g,mat);me.position.copy(s.schwer);wurzel.add(me);return {me,...s};});
  wurzel.scale.setScalar(radius);
  const glanz=glanzTextur(),strahlen=strahlenTextur(saat),streif=streifTextur(),rauch=rauchTextur(saat+3);
  const sprite=(karte,f,blend=THREE.AdditiveBlending)=>{const sp=new THREE.Sprite(new THREE.SpriteMaterial({map:karte,color:f,transparent:true,depthWrite:false,blending:blend,opacity:0}));wurzel.add(sp);return sp;};
  const weiss=new THREE.Color('#fff4e6');
  const einschlag=sprite(glanz,new THREE.Color('#ffd9a0')),kernGlut=sprite(glanz,new THREE.Color('#ffb060'));
  const blitz=sprite(glanz,weiss),kranz=sprite(strahlen,weiss),kranz2=sprite(strahlen,farbe),streifen=sprite(streif,farbe.clone().lerp(weiss,.4));
  const plasma=new THREE.Mesh(new THREE.SphereGeometry(1,64,40),plasmaMaterial(farbe));plasma.visible=false;wurzel.add(plasma);
  const welleGeo=new THREE.PlaneGeometry(2,2),wellen=[0,1].map(i=>{const w=new THREE.Mesh(welleGeo,welleMaterial(i?new THREE.Color('#ffb070'):farbe));w.visible=false;wurzel.add(w);return w;});
  // Funkenschauer: Lichtstriche, die schnell auseinanderfliegen und verglühen
  const NF=opt.funken??900,fz=zufallVon(saat+51),fr=[],fpos=new Float32Array(NF*6),ffarb=new Float32Array(NF*6);
  for(let i=0;i<NF;i++){const u=fz()*2-1,w=fz()*Math.PI*2,s=Math.sqrt(1-u*u);fr.push({d:new THREE.Vector3(s*Math.cos(w),u,s*Math.sin(w)),v:.8+fz()*3.2,h:.4+fz()*.6});}
  const fg=new THREE.BufferGeometry();fg.setAttribute('position',new THREE.BufferAttribute(fpos,3));fg.setAttribute('color',new THREE.BufferAttribute(ffarb,3));
  const funken=new THREE.LineSegments(fg,new THREE.LineBasicMaterial({vertexColors:true,transparent:true,depthWrite:false,blending:THREE.AdditiveBlending,opacity:0}));funken.visible=false;wurzel.add(funken);
  // Staubwolke: dunkle, langsam aufquellende Ballen, vom Laserlicht angestrahlt
  const sz=zufallVon(saat+77),staub=Array.from({length:16},()=>{const u=sz()*2-1,w=sz()*Math.PI*2,s=Math.sqrt(1-u*u);const sp=sprite(rauch,new THREE.Color('#2e2620'),THREE.NormalBlending);sp.material.rotation=sz()*6.28;
   return {sp,d:new THREE.Vector3(s*Math.cos(w),u,s*Math.sin(w)),v:.5+sz()*1.2,g:1.2+sz()*1.6};});
  const staubGlut=staub.slice(0,8).map(s=>({...s,sp:sprite(rauch,farbe)}));
  // Glutfunken an den Brocken
  const N=900,tp=new Float32Array(N*3),tv=[],zufall=zufallVon(saat+99);for(let i=0;i<N;i++){const u=zufall()*2-1,w=zufall()*Math.PI*2,s=Math.sqrt(1-u*u);tv.push(new THREE.Vector3(s*Math.cos(w),u,s*Math.sin(w)).multiplyScalar(.4+zufall()*1.8));}
  const tg=new THREE.BufferGeometry();tg.setAttribute('position',new THREE.BufferAttribute(tp,3));
  const glut=new THREE.Points(tg,new THREE.PointsMaterial({map:glanz,color:new THREE.Color('#ffb070'),size:.035,transparent:true,depthWrite:false,blending:THREE.AdditiveBlending,opacity:0}));wurzel.add(glut);
  const treffer=new THREE.Vector3(0,0,-1),q=new THREE.Quaternion(),z=new THREE.Vector3(0,0,1);
  // Optional (galaxy.js): Nach dem Bruch ziehen die Brocken in die Mondbahn, verteilen sich entlang der Bahn (nach ihrer
  // Flugrichtung) zum Trümmerring und gehen darin auf. ring: {mitte (Planetenmitte), achse (Normale der Bahnebene),
  // radius (Ringradius)} in Mond-Einheiten relativ zum Mond. Zeitplan: einziehen ab 1 s, verteilen 1,2–10,2 s, aufgehen 6–13 s.
  const glatt=x=>{x=Math.max(0,Math.min(1,x));return x*x*(3-2*x);},rz=zufallVon(saat+131),e0=new THREE.Vector3(),e1=new THREE.Vector3(),ringZiel=new THREE.Vector3();
  for(const t of teile){t.ringR=(rz()-.5)*.4;t.ringH=rz()-.5;}
  let ringOrdnung=false;
  function zumRing(ring,bruch){
   const R=ring.mitte.length();if(R<1e-6)return;
   e0.copy(ring.mitte).multiplyScalar(-1/R);e1.crossVectors(ring.achse,e0).normalize();
   if(!ringOrdnung){ringOrdnung=true;[...teile].sort((a,b)=>a.richtung.dot(e1)-b.richtung.dot(e1)).forEach((t,i,s)=>{t.ringWinkel=((i+.5)/s.length*2-1)*Math.PI*.97;});}
   const k=glatt((bruch-1)/3.5),spr=glatt((bruch-1.2)/9),sk=1-glatt((bruch-6)/7),a=R+(ring.radius-R)*glatt((bruch-1)/5);
   for(const t of teile){
    const phi=t.ringWinkel*spr,rr=a*(1+t.ringR);
    ringZiel.copy(ring.mitte).addScaledVector(e0,Math.cos(phi)*rr).addScaledVector(e1,Math.sin(phi)*rr).addScaledVector(ring.achse,t.ringH*ring.radius*.3);
    t.me.position.lerp(ringZiel,k);t.me.scale.setScalar(Math.max(.001,sk));t.me.visible=sk>.002;
   }
  }
  return {objekt:wurzel,radius,
   treffer(richtung){treffer.copy(richtung).normalize();mat.userData.u.uTreffer.value.copy(treffer);},
   setze({schmelz=0,riss=0,bruch=-1,zeit=0,ring=null}={}){
    const u=mat.userData.u;u.uSchmelz.value=schmelz;u.uRiss.value=riss;u.uZeit.value=zeit;
    einschlag.position.copy(treffer).multiplyScalar(.99);einschlag.material.opacity=Math.min(.55,schmelz*1.1)*(bruch<0?1:Math.max(0,1-bruch*2));einschlag.scale.setScalar(.08+schmelz*.12+Math.sin(zeit*40)*.012);
    const aus=[blitz,kranz,kranz2,streifen];
    if(bruch<0){for(const t of teile){t.me.position.copy(t.schwer);t.me.quaternion.identity();t.me.scale.setScalar(1);t.me.visible=true;}u.uHitze.value=0;for(const s of aus)s.material.opacity=0;plasma.visible=false;for(const w of wellen)w.visible=false;funken.visible=false;
     for(const s of staub)s.sp.material.opacity=0;for(const s of staubGlut)s.sp.material.opacity=0;glut.material.opacity=0;kernGlut.material.opacity=riss*.5;kernGlut.scale.setScalar(1.5);return;}
    // Blitz mit Strahlenkranz und Lichtstreif
    const f=Math.max(0,1-bruch*.7),f2=Math.max(0,1-bruch*.35);
    blitz.material.opacity=Math.min(1,f*1.4);blitz.scale.setScalar(2.2+bruch*6);
    kranz.material.opacity=f*.9;kranz.scale.setScalar(5+bruch*7);kranz.material.rotation=bruch*.15;
    kranz2.material.opacity=f2*.6;kranz2.scale.setScalar(7+bruch*9);kranz2.material.rotation=-bruch*.08+.4;
    streifen.material.opacity=f*.85;streifen.scale.set(16+bruch*10,.6+bruch*.2,1);
    // Plasmahülle quillt schnell auf und bremst, Farbe kühlt von weiß über Strahlfarbe nach orange
    plasma.visible=bruch<7;plasma.scale.setScalar(1.05+3.4*(1-Math.exp(-bruch*1.1)));Object.assign(plasma.material.uniforms,{});plasma.material.uniforms.uZeit.value=zeit;plasma.material.uniforms.uAlter.value=bruch;plasma.material.uniforms.uStaerke.value=Math.exp(-bruch*.55)*Math.min(1,bruch*6);
    // zwei Druckwellen: senkrecht zum Einschlag und schräg dazu
    wellen.forEach((w,i)=>{w.visible=bruch<9;w.scale.setScalar(1.2+(i?5.5:7.5)*(1-Math.exp(-bruch*(i?.7:.9))));q.setFromUnitVectors(z,treffer);w.quaternion.copy(q);if(i)w.rotateX(1.05);
     w.material.uniforms.uZeit.value=zeit;w.material.uniforms.uStaerke.value=Math.exp(-bruch*(i?.45:.38))*Math.min(1,bruch*4)*(i?.7:1);});
    // Brocken: fliegen gebremst auseinander und taumeln; kleine schneller
    const weg=1-Math.exp(-bruch*.5);
    for(const t of teile){t.me.position.copy(t.schwer).addScaledVector(t.richtung,weg*t.tempo*2.4+bruch*.035*t.tempo);q.setFromAxisAngle(t.dreh,bruch*t.drehTempo*.5);t.me.quaternion.copy(q);}
    if(ring)zumRing(ring,bruch);
    u.uHitze.value=Math.max(0,1-bruch*.3);u.uRiss.value=Math.max(0,riss*(1-bruch*.6));
    kernGlut.material.opacity=Math.max(0,.95-bruch*.22);kernGlut.scale.setScalar(1.3+bruch*.5);
    // Funken als Lichtstriche
    funken.visible=bruch<8;funken.material.opacity=Math.max(0,1-bruch*.14);
    for(let i=0;i<NF;i++){const s=fr[i],dist=.9+s.v*(1-Math.exp(-bruch*1.3))*2.2,len=Math.min(.6,.05+s.v*.22*Math.exp(-bruch*.9)),hell=s.h*Math.exp(-bruch*(.35+s.v*.08));
     fpos.set([s.d.x*dist,s.d.y*dist,s.d.z*dist,s.d.x*(dist-len),s.d.y*(dist-len),s.d.z*(dist-len)],i*6);ffarb.set([hell,hell*.7,hell*.4,0,0,0],i*6);}
    fg.attributes.position.needsUpdate=true;fg.attributes.color.needsUpdate=true;
    // Staubwolke
    const wolke=Math.min(1,bruch*.8)*Math.max(0,1-(bruch-4)/9);
    for(const s of staub){const r=.6+s.v*(1-Math.exp(-bruch*.35))*2.6;s.sp.position.copy(s.d).multiplyScalar(r);s.sp.scale.setScalar(s.g*(.55+bruch*.16));s.sp.material.opacity=wolke*.28;}
    for(const s of staubGlut){s.sp.position.copy(s.d).multiplyScalar(.6+s.v*(1-Math.exp(-bruch*.35))*2.6);s.sp.scale.setScalar(s.g*(.6+bruch*.17));s.sp.material.opacity=wolke*.16*Math.exp(-bruch*.3);}
    const tpos=tg.attributes.position;for(let i=0;i<N;i++){const v=tv[i];tpos.setXYZ(i,v.x*(.3+weg*3.4),v.y*(.3+weg*3.4),v.z*(.3+weg*3.4));}tpos.needsUpdate=true;glut.material.opacity=Math.min(1,bruch*3)*Math.max(0,1-bruch*.08);
   },
   entsorgen(){for(const t of teile)t.g.dispose();mat.dispose();tg.dispose();fg.dispose();welleGeo.dispose();plasma.geometry.dispose();for(const t of [glanz,strahlen,streif,rauch])t.dispose();}};
 }
 window.Mondzerstoerung={erzeugen,kartenUrl};
})();
