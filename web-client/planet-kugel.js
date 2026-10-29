'use strict';
// Drehende Planeten und Monde aus flachen Oberflächenkarten (content/assets/*/…map.png).
// Eine gemeinsame WebGL2-Fläche zeichnet alle Kugeln der Seite und kopiert sie in die
// jeweiligen 2D-Leinwände – so gibt es kein Kontextlimit, egal wie viele Kugeln sichtbar sind.
// Licht: weich (Nachtseite bleibt lesbar) und aus der Richtung des Sterns; es wandert mit dem
// Umlauf des Planeten. Dieselben Lichtzahlen nutzt tools/content/kugel_render.py für Standbilder,
// GIFs und GLB, damit Spiel und Bilder gleich aussehen.
window.PlanetKugel=(()=>{
 const TAG=86400,MOND_DREHTAGE=30;
 // Sekunden je Umdrehung bei Tempo „normal“. Sonnensystem schneller, Planetenansicht ruhig.
 const PERIODE={system:{planet:36,mond:16,mondumlauf:70},planet:{planet:240,mond:110,mondumlauf:300}};
 const STUFEN={aus:0,langsam:.5,normal:1,schnell:2};
 const LICHT={grund:.30,umlauf:.45,hoehe:.35};
 // Je Habitabilitätszone 6 Oberflächen (2 ursprüngliche + 4 neue); passend zu tools/content/planeten_karten.py.
 const TYPEN={
  glut:['glut_basalt','glut_wueste','glut_lavameer','glut_schwefel','glut_salzpfanne','glut_obsidian'],
  leben:['leben_kontinent','leben_ozean','leben_dschungel','leben_archipel','leben_savanne','leben_sumpf'],
  frost:['frost_eis','frost_tundra','frost_gletscher','frost_kristall','frost_methan','frost_taiga'],
 };
 const ATMO={leben:[110,170,230],frost:[190,225,255],glut:[255,150,90],nebel:[200,140,255]};
 const ATMO_TYP={glut_basalt:[255,120,70],glut_lavameer:[255,120,60],frost_methan:[240,170,90],leben_sumpf:[130,200,180],glut_salzpfanne:[255,220,170],asteroid:null};
 const reduced=matchMedia('(prefers-reduced-motion: reduce)');
 const lesen=k=>{try{return localStorage.getItem(k);}catch{return null;}};
 const schreiben=(k,v)=>{try{localStorage.setItem(k,v);}catch{}};
 // FNV-1a über die Koordinate: deterministisch, in allen Ansichten (Tabelle, 3D, Übersicht) gleich.
 // Gleiche Formel in crates/spieler/src/ansicht/kolonie.rs (planet_typ).
 function streuung(text){let h=2166136261;for(const c of String(text)){h^=c.charCodeAt(0);h=Math.imul(h,16777619)>>>0;}return h>>>0;}
 function typ(p){
  const h=streuung(p?.koord||'');
  // Nebelwelt genau dann, wenn der Kern dort natürlichen Xenokristall meldet (xeno_vorkommen: gerade Plätze im
  // Nebelsystem, bekannt sobald das System erfasst ist); fehlt das Feld, gilt dieselbe Kernregel (industrie.rs xeno_planet).
  if(p?.xeno_vorkommen===true)return 'nebel_xeno';
  if(p?.xeno_vorkommen==null&&p?.nebel===true&&Number(p?.position||String(p?.koord||'').split(':')[2])%2===0)return 'nebel_xeno';
  const liste=TYPEN[p?.zone]||TYPEN.leben;return liste[(h>>>3)%liste.length];
 }
 function zoneVon(t){return t.split('_')[0];}
 function atmo(t){return t in ATMO_TYP?ATMO_TYP[t]:ATMO[zoneVon(t)]||null;}
 function karte(kategorie,key,volk=''){const m=window.STERNEN_ART?.maps||{};return m[`${kategorie}|${key}|${volk}`]?.karte||m[`${kategorie}|${key}|`]?.karte||'';}
 function kugelbild(kategorie,key,volk=''){const m=window.STERNEN_ART?.maps||{};return m[`${kategorie}|${key}|${volk}`]?.kugel||m[`${kategorie}|${key}|`]?.kugel||'';}
 function stufe(){const s=lesen('sternenepoche-drehtempo');return s in STUFEN?s:(reduced.matches?'aus':'normal');}
 function tempo(){return STUFEN[stufe()];}
 function setzeStufe(s){if(!(s in STUFEN))return;schreiben('sternenepoche-drehtempo',s);document.dispatchEvent(new CustomEvent('drehtempo',{detail:{stufe:s,tempo:STUFEN[s]}}));wecken();}
 // Monde drehen sich nach ihrer Entstehung immer langsamer und stehen nach 30 Tagen (gebundene Rotation).
 function mondFaktor(alterSekunden){const t=Math.max(0,Number(alterSekunden)||0)/(MOND_DREHTAGE*TAG);return t>=1?0:(1-t)**1.6;}
 // Licht aus Richtung des Sterns: der Umlaufwinkel (wie GalaxyModel.orbit) schwenkt den Terminator.
 function lichtAzimut(position,sekunden,langsamer=1){const pos=Math.max(1,Number(position)||1),winkel=pos*2.3999632297+sekunden*.095/Math.sqrt(pos)/langsamer;return -55+42*Math.sin(winkel);}
 function lichtVektor(azimutGrad){const a=azimutGrad*Math.PI/180,v=[Math.sin(a),LICHT.hoehe,Math.cos(a)],n=Math.hypot(...v);return v.map(x=>x/n);}

 // ---------------------------------------------------------------- WebGL2 ----------
 let gl=null,glCanvas=null,programm=null,ort={},quad=null,kaputt=false;
 const texturen=new Map(),ziele=new Set();let frameId=0,zuletzt=0;
 const VS=`#version 300 es
in vec2 aEcke;uniform vec2 uMitte;uniform vec2 uRadius;out vec2 vPos;
void main(){vPos=aEcke;gl_Position=vec4(uMitte+aEcke*uRadius,0.,1.);}`;
 const FS=`#version 300 es
precision highp float;in vec2 vPos;out vec4 o;
uniform sampler2D uKarte;uniform float uDreh,uKipp,uPixel,uGrund,uUmlauf,uAtmoAn;uniform vec3 uLicht,uAtmo;
void main(){
 float d2=dot(vPos,vPos),r=sqrt(d2),a=clamp((1.-r)*uPixel+.5,0.,1.);if(a<=0.)discard;
 float z=sqrt(max(0.,1.-d2)),c=cos(uKipp),s=sin(uKipp);
 float xr=vPos.x*c+vPos.y*s,yr=-vPos.x*s+vPos.y*c;
 float lat=asin(clamp(yr,-1.,1.)),lon=atan(xr,z)-uDreh;
 float u=fract(lon/6.2831853+.5),u2=fract(lon/6.2831853),v=.5-lat/3.1415927;
 vec2 gx=vec2(dFdx(u),dFdx(v)),gy=vec2(dFdy(u),dFdy(v)),gx2=vec2(dFdx(u2),dFdx(v)),gy2=vec2(dFdy(u2),dFdy(v));
 if(abs(gx2.x)+abs(gy2.x)<abs(gx.x)+abs(gy.x)){gx=gx2;gy=gy2;}
 vec3 f=textureGrad(uKarte,vec2(u,v),gx,gy).rgb;
 vec3 n=vec3(vPos,z);float nl=dot(n,uLicht);
 float lam=pow(clamp((nl+uUmlauf)/(1.+uUmlauf),0.,1.),1.2);
 vec3 rgb=f*(uGrund+(1.-uGrund)*lam);
 if(uAtmoAn>.5){float saum=pow(1.-z,2.6)*(.35+.65*clamp(nl*.5+.6,0.,1.));rgb=rgb*(1.-saum*.55)+uAtmo*saum*.75;}
 o=vec4(rgb*a,a);
}`;
 function start(){
  if(gl||kaputt)return !!gl;
  try{
   glCanvas=document.createElement('canvas');glCanvas.width=glCanvas.height=512;
   gl=glCanvas.getContext('webgl2',{premultipliedAlpha:true,alpha:true,antialias:false,powerPreference:'low-power'});
   if(!gl)throw Error('kein WebGL2');
   const sh=(art,src)=>{const s=gl.createShader(art);gl.shaderSource(s,src);gl.compileShader(s);if(!gl.getShaderParameter(s,gl.COMPILE_STATUS))throw Error(gl.getShaderInfoLog(s));return s;};
   programm=gl.createProgram();gl.attachShader(programm,sh(gl.VERTEX_SHADER,VS));gl.attachShader(programm,sh(gl.FRAGMENT_SHADER,FS));gl.linkProgram(programm);
   if(!gl.getProgramParameter(programm,gl.LINK_STATUS))throw Error(gl.getProgramInfoLog(programm));
   for(const n of ['uMitte','uRadius','uKarte','uDreh','uKipp','uPixel','uGrund','uUmlauf','uAtmoAn','uLicht','uAtmo'])ort[n]=gl.getUniformLocation(programm,n);
   quad=gl.createBuffer();gl.bindBuffer(gl.ARRAY_BUFFER,quad);gl.bufferData(gl.ARRAY_BUFFER,new Float32Array([-1,-1,1,-1,-1,1,1,1]),gl.STATIC_DRAW);
   const ecke=gl.getAttribLocation(programm,'aEcke');gl.enableVertexAttribArray(ecke);gl.vertexAttribPointer(ecke,2,gl.FLOAT,false,0,0);
   gl.enable(gl.BLEND);gl.blendFunc(gl.ONE,gl.ONE_MINUS_SRC_ALPHA);
   glCanvas.addEventListener('webglcontextlost',e=>{e.preventDefault();gl=null;kaputt=true;for(const z of ziele)z.fallback();});
   return true;
  }catch(e){kaputt=true;gl=null;console.warn('PlanetKugel: kein WebGL2, Standbilder bleiben',e);return false;}
 }
 function textur(url){
  if(texturen.has(url))return texturen.get(url);
  const eintrag={bereit:false,tex:null};texturen.set(url,eintrag);
  const bild=new Image();bild.decoding='async';
  bild.onload=()=>{if(!gl)return;const t=gl.createTexture();gl.bindTexture(gl.TEXTURE_2D,t);gl.pixelStorei(gl.UNPACK_FLIP_Y_WEBGL,false);gl.texImage2D(gl.TEXTURE_2D,0,gl.RGBA,gl.RGBA,gl.UNSIGNED_BYTE,bild);gl.generateMipmap(gl.TEXTURE_2D);
   gl.texParameteri(gl.TEXTURE_2D,gl.TEXTURE_WRAP_S,gl.REPEAT);gl.texParameteri(gl.TEXTURE_2D,gl.TEXTURE_WRAP_T,gl.CLAMP_TO_EDGE);gl.texParameteri(gl.TEXTURE_2D,gl.TEXTURE_MIN_FILTER,gl.LINEAR_MIPMAP_LINEAR);gl.texParameteri(gl.TEXTURE_2D,gl.TEXTURE_MAG_FILTER,gl.LINEAR);
   eintrag.tex=t;eintrag.bereit=true;for(const z of ziele)z.neu=true;wecken();};
  bild.onerror=()=>{eintrag.fehler=true;};bild.src=url;return eintrag;
 }
 function koerperZeichnen(k,px,breite,hoehe,licht){
  const t=textur(k.karte);if(!t.bereit)return false;
  gl.bindTexture(gl.TEXTURE_2D,t.tex);gl.uniform1i(ort.uKarte,0);
  gl.uniform2f(ort.uMitte,k.x,k.y);gl.uniform2f(ort.uRadius,k.r*Math.min(breite,hoehe)/breite,k.r*Math.min(breite,hoehe)/hoehe);
  gl.uniform1f(ort.uDreh,k.dreh);gl.uniform1f(ort.uKipp,k.kipp??.21);gl.uniform1f(ort.uPixel,px*k.r);
  gl.uniform1f(ort.uGrund,LICHT.grund);gl.uniform1f(ort.uUmlauf,LICHT.umlauf);gl.uniform3fv(ort.uLicht,licht);
  const a=k.atmo;gl.uniform1f(ort.uAtmoAn,a?1:0);gl.uniform3f(ort.uAtmo,...(a||[0,0,0]));
  gl.drawArrays(gl.TRIANGLE_STRIP,0,4);return true;
 }
 // Ein Ziel = eine 2D-Leinwand mit einem Planeten und optional seinem Mond.
 function zielZeichnen(z,sekunden,delta){
  const rect=z.canvas.getBoundingClientRect(),dpr=Math.min(2,devicePixelRatio||1),/* gemeinsamer Faktor: sonst kippt das Seitenverhältnis (Kugel oval) */sk=Math.min(1,512/Math.max(1,rect.width*dpr,rect.height*dpr)),w=Math.max(16,Math.round(rect.width*dpr*sk)),h=Math.max(16,Math.round(rect.height*dpr*sk));
  if(!rect.width||!rect.height)return;
  if(z.canvas.width!==w||z.canvas.height!==h){z.canvas.width=w;z.canvas.height=h;z.neu=true;}
  const v=tempo(),p=PERIODE[z.ansicht]||PERIODE.planet;
  if(v>0&&delta>0){
   z.planet.dreh+=delta*v*2*Math.PI/(z.planet.alter!=null?p.mond:p.planet)*(z.planet.alter!=null?mondFaktor(z.planet.alter+(sekunden-z.start)):1);
   if(z.mond){z.mond.dreh+=delta*v*2*Math.PI/p.mond*mondFaktor(z.mond.alter+(sekunden-z.start));z.mondWinkel+=delta*v*2*Math.PI/p.mondumlauf;}
  }else if(!z.neu)return;
  const licht=lichtVektor(lichtAzimut(z.position,sekunden,z.ansicht==='planet'?4:1));
  gl.viewport(0,0,w,h);gl.clearColor(0,0,0,0);gl.clear(gl.COLOR_BUFFER_BIT);gl.useProgram(programm);
  const koerper=[z.planet];
  if(z.mond){const a=z.mondWinkel,b=z.mondBahn;z.mond.x=Math.cos(a)*b[0];z.mond.y=Math.sin(a)*b[1]+b[2];if(Math.sin(a)>0)koerper.unshift(z.mond);else koerper.push(z.mond);}
  // Pixelmaß für die Kantenglättung: Radius in Gerätepixeln
  let fertig=true;for(const k of koerper)fertig=koerperZeichnen(k,Math.min(w,h)/2,w,h,licht)&&fertig;
  const ctx=z.ctx;ctx.clearRect(0,0,w,h);ctx.drawImage(glCanvas,0,glCanvas.height-h,w,h,0,0,w,h);
  if(fertig&&z.planet.karte){z.neu=false;if(!z.sichtbar){z.sichtbar=true;z.host.classList.add('kugel-aktiv');}}
  mondKnopfLage(z,rect);
 }
 // Mond antippen = auf dem Mond landen (Karl 25.10.2026: „der Mond soll anklickbar sein, dass ich dann direkt in dem Mond
 // lande und dort was bauen kann“). Ein echter Knopf über der Mondscheibe (Fokus, Vorlesen, :active), der mit dem Umlauf
 // wandert; Tippfläche die Mondscheibe (mindestens 24 px), nie über dem Planeten. Steht der Mond ganz hinter dem
 // Planeten, ist der Knopf weg – sonst landete ein Tipp auf den Planeten auf dem Mond.
 function mondKnopf(z,opts){
  let b=z.host.querySelector(':scope>button.kugel-mond');
  if(!z.mond||typeof opts.mondKlick!=='function'){b?.remove();z.mondKlick=null;return;}
  z.mondKlick=opts.mondKlick;
  if(!b){b=document.createElement('button');b.type='button';b.className='kugel-mond kugel-zusatz';b.addEventListener('click',e=>{e.preventDefault();e.stopPropagation();b.__z?.mondKlick?.();});z.host.append(b);}
  b.__z=z;const text=`${opts.mondName||'Mond'} – auf dem Mond landen und dort bauen`;
  if(b.getAttribute('aria-label')!==text){b.setAttribute('aria-label',text);b.title=text;}
  const r=z.canvas.getBoundingClientRect();if(z.mond.x||z.mond.y)mondKnopfLage(z,r);
 }
 function mondKnopfLage(z,rect){
  const b=z.mondKlick&&z.host.querySelector(':scope>button.kugel-mond');if(!b||!z.mond||!rect.width)return;
  const m=z.mond,pl=z.planet,halb=Math.min(rect.width,rect.height)/2;
  const mx=(m.x+1)/2*rect.width,my=(1-m.y)/2*rect.height,mr=m.r*halb;
  const px=(pl.x+1)/2*rect.width,py=(1-pl.y)/2*rect.height,pr=pl.r*halb;
  const hinten=Math.sin(z.mondWinkel)>0,verdeckt=hinten&&Math.hypot(mx-px,my-py)+mr*.6<pr;
  // Nur die Mondscheibe (+3 px Rand, mindestens 24 px): ein Tipp auf den Planeten darf nie auf dem Mond landen (Karl 25.10.2026).
  const d=Math.max(24,Math.round(mr*2+6));
  b.hidden=verdeckt;
  b.style.left=Math.round(mx)+'px';b.style.top=Math.round(my)+'px';b.style.width=b.style.height=d+'px';
 }
 function schleife(zeit){
  frameId=0;if(!gl||document.hidden)return;
  /* echte Zeit: Drehtempo stimmt auch bei wenigen Bildern/s */const delta=zuletzt?Math.min(1,(zeit-zuletzt)/1000):0;
  zuletzt=zeit;const sekunden=Date.now()/1000;
  let maxW=16,maxH=16;for(const z of ziele){if(!z.host.isConnected){ziele.delete(z);continue;}const sichtbar=imBild(z);if(sichtbar&&!z.imBild)z.neu=true;z.imBild=sichtbar;if(z.imBild){maxW=Math.max(maxW,z.canvas.width);maxH=Math.max(maxH,z.canvas.height);}}
  if(glCanvas.width<maxW||glCanvas.height<maxH){glCanvas.width=Math.max(glCanvas.width,maxW);glCanvas.height=Math.max(glCanvas.height,maxH);for(const z of ziele)z.neu=true;}
  let weiter=false;for(const z of ziele){if(!z.imBild)continue;zielZeichnen(z,sekunden,delta);if(tempo()>0||z.neu)weiter=true;}
  if(weiter)frameId=requestAnimationFrame(schleife);else zuletzt=0;
 }
 function wecken(){if(!frameId&&gl)frameId=requestAnimationFrame(schleife);}
 // Sichtbarkeit direkt aus der Lage der Leinwand (ein Beobachter meldet in gedrosselten Ansichten zu spät).
 function imBild(z){const r=z.canvas.getBoundingClientRect();return r.width>0&&r.bottom>0&&r.right>0&&r.top<innerHeight&&r.left<innerWidth;}
 addEventListener('scroll',()=>wecken(),{passive:true,capture:true});
 document.addEventListener('visibilitychange',()=>{if(!document.hidden){zuletzt=0;wecken();}});

 /** Hängt eine drehende Kugel in `host` (enthält ein Standbild als Rückfall).
  *  opts: {planet: Planetenobjekt (koord, zone, nebel, position), mond: Mondobjekt|null, ansicht:'planet'|'system',
  *         mondKlick: Funktion beim Antippen des Mondes (optional), mondName: Name für Knopf und Vorlesen} */
 // Wiederholbar: Ansichten werden per setStableHTML abgeglichen, das Attribute/Kinder zurücksetzt.
 // Gleicher Schlüssel -> Zustand (Drehwinkel) bleibt, nur Markierungen werden erneuert.
 function einbinden(host,opts={}){
  if(!host||!start())return null;
  const p=opts.planet||{},nurMond=!!opts.nurMond,m=opts.mond,t=nurMond?'':opts.typ||typ(p),url=nurMond?karte('moons',m?.typ||'',m?.volk||''):karte('planets',t);if(!url)return null;
  const mondUrl=!nurMond&&m?.typ?karte('moons',m.typ,m.volk||''):'';
  const schluessel=[t||url,p.koord||m?.koord,mondUrl,opts.ansicht||'planet'].join('|');
  let canvas=host.querySelector(':scope>canvas.planet-kugel');
  if(host.__kugel&&host.__kugel.schluessel===schluessel&&host.__kugel.canvas===canvas&&ziele.has(host.__kugel)){
   const z=host.__kugel;if(m&&z.mond)z.mond.alter=Number(m.alter_sekunden)||0;if(m&&z.planet.alter!=null)z.planet.alter=Number(m.alter_sekunden)||0;host.classList.add('kugel-host');if(z.sichtbar)host.classList.add('kugel-aktiv');z.neu=true;mondKnopf(z,opts);wecken();return z;
  }
  if(host.__kugel)ziele.delete(host.__kugel);
  if(!canvas){canvas=document.createElement('canvas');canvas.className='planet-kugel';canvas.setAttribute('aria-hidden','true');host.append(canvas);}
  host.classList.add('kugel-host');host.classList.remove('kugel-aktiv');
  const position=Number(p.position||String(p.koord||'').split(':')[2])||1;
  const groesse=m?.groesse||m?.durchmesser||0,mondR=Math.max(.09,Math.min(.17,.07+groesse/120000));
  const z={host,canvas,schluessel,ctx:canvas.getContext('2d'),ansicht:opts.ansicht||'planet',position,start:Date.now()/1000,neu:true,imBild:false,sichtbar:false,
   planet:nurMond?{karte:url,x:0,y:0,r:.92,dreh:(streuung(m.koord||p.koord)%360)*Math.PI/180,kipp:.12,atmo:null,alter:Number(m.alter_sekunden)||0}:{karte:url,x:0,y:mondUrl?-.04:0,r:mondUrl?.70:.97,dreh:(streuung(p.koord)%360)*Math.PI/180,kipp:.21,atmo:atmo(t)?.map(x=>x/255)},
   mond:mondUrl?{karte:mondUrl,x:0,y:0,r:mondR,dreh:0,kipp:.12,atmo:null,alter:Number(m.alter_sekunden)||0}:null,
   mondWinkel:(streuung(p.koord+'m')%628)/100,mondBahn:[.84,.2,-.04],
   fallback(){canvas.remove();host.classList.remove('kugel-aktiv');}};
  canvas.__kugel=z;host.__kugel=z;ziele.add(z);mondKnopf(z,opts);wecken();return z;
 }
 // ---------------------------------------------------------------- Klappbilder ------
 // Todesmonde haben zwei Zustände: geschlossene Reiseform (Standard) und aufgeklappt (Angriff).
 // ruheform() liefert für jedes Aufklapp-Bild die geschlossene Fassung; beim Überfahren/Fokus legt
 // sich das aufgeklappte Bild animiert darüber. Die Überlagerung hängt am <body>, damit der
 // DOM-Abgleich (setStableHTML) sie nicht entfernt.
 let klappPaare=null;
 function klappListe(){
  if(klappPaare)return klappPaare;klappPaare=new Map();const img=window.STERNEN_ART?.images||{};
  for(const [k,v] of Object.entries(img)){const [kat,key,volk]=k.split('|');if(!key.endsWith('_geschlossen'))continue;const offen=img[`${kat}|${key.slice(0,-12)}|${volk}`];if(offen)klappPaare.set(offen.url,v.url);}
  return klappPaare;
 }
 function ruheform(url){return klappListe().get(url)||url;}
 function offenform(url){for(const [o,g] of klappListe())if(g===url)return o;return '';}
 let klappAuf=null;
 function aufklappen(bild){
  const offen=offenform(bild.getAttribute('src'));if(!offen||klappAuf?.ziel===bild)return;zuklappen();
  const r=bild.getBoundingClientRect();if(!r.width)return;
  const o=document.createElement('img');o.src=offen;o.alt='';o.className='klapp-offen';o.setAttribute('aria-hidden','true');
  Object.assign(o.style,{left:r.left+'px',top:r.top+'px',width:r.width+'px',height:r.height+'px'});(bild.closest('dialog[open]')||document.body).append(o);
  klappAuf={ziel:bild,o};bild.classList.add('klapp-ziel');
 }
 function zuklappen(){if(!klappAuf)return;const {o,ziel}=klappAuf;klappAuf=null;ziel.classList.remove('klapp-ziel');o.classList.add('zu');setTimeout(()=>o.remove(),260);}
 document.addEventListener('pointerover',e=>{const b=e.target.closest?.('img');if(b&&offenform(b.getAttribute('src')))aufklappen(b);});
 document.addEventListener('pointerout',e=>{if(klappAuf&&e.target===klappAuf.ziel)zuklappen();});
 document.addEventListener('focusin',e=>{const b=e.target.querySelector?.('img')||e.target;if(b?.tagName==='IMG'&&offenform(b.getAttribute('src')))aufklappen(b);});
 document.addEventListener('focusout',e=>{if(klappAuf&&(e.target===klappAuf.ziel||e.target.contains?.(klappAuf.ziel)))zuklappen();});
 // Beim Scrollen mitwandern; verschwindet das Bild aus dem DOM oder aus dem Bild, schließen.
 addEventListener('scroll',()=>{if(!klappAuf)return;const {o,ziel}=klappAuf;if(!ziel.isConnected){zuklappen();return;}const r=ziel.getBoundingClientRect();if(r.bottom<0||r.top>innerHeight){zuklappen();return;}o.style.left=r.left+'px';o.style.top=r.top+'px';},{passive:true,capture:true});

 // Stile: web-client/planet-kugel.css (CSP erlaubt nur style-src 'self', keine eingefügten <style>-Blöcke).
 
 /** Auswahl „Drehung: aus/langsam/normal/schnell“ – einmal gespeichert, gilt für alle Ansichten. */
 function regler(){const l=document.createElement('label');l.className='drehtempo-wahl';l.title='Eigendrehung von Planeten und Monden. Im Sonnensystem drehen sie schneller als in der Planetenansicht.';
  l.innerHTML=`<span>Drehung</span><select data-drehtempo>${Object.keys(STUFEN).map(s=>`<option value="${s}" ${s===stufe()?'selected':''}>${s[0].toUpperCase()+s.slice(1)}</option>`).join('')}</select>`;
  return l;}
 function markup(){return `<label class="drehtempo-wahl" title="Eigendrehung von Planeten und Monden. Im Sonnensystem drehen sie schneller als in der Planetenansicht."><span>Drehung von Planeten und Monden</span><select data-drehtempo>${Object.keys(STUFEN).map(s=>`<option value="${s}" ${s===stufe()?'selected':''}>${s[0].toUpperCase()+s.slice(1)}</option>`).join('')}</select></label>`;}
 document.addEventListener('change',e=>{if(e.target.matches?.('select[data-drehtempo]'))setzeStufe(e.target.value);});
 document.addEventListener('drehtempo',()=>{for(const s of document.querySelectorAll('select[data-drehtempo]'))s.value=stufe();});
 reduced.addEventListener?.('change',()=>{for(const z of ziele)z.neu=true;wecken();});
 return {ruheform,offenform,regler,markup,typ,TYPEN,PERIODE,STUFEN,stufe,tempo,setzeStufe,mondFaktor,lichtAzimut,lichtVektor,karte,kugelbild,einbinden,streuung,atmo,MOND_DREHTAGE};
})();
