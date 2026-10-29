'use strict';
// Dysonsphäre (docs/DYSONSPHAERE.md): Allianz-Reiter „Großprojekt“, Galaxie-Zeile über Platz 1, Blitz an angeschlossenen
// Planeten und die animierte Sonnenansicht (three.js wie galaxy.js). Daten: view.dyson (Kern dyson_sicht), Galaxie-Werkzeug
// systeme[].dyson. Befehle: dyson_vorschlagen, dyson_beitreten, dyson_austreten, dyson_einzahlen, dyson_bau_starten,
// dyson_abbrechen, dyson_anschluss_waehlen, dyson_anschluss_loesen, dyson_auszahlen – alle über command().
// Server-CSP style-src 'self': keine style-Attribute in HTML-Strings, nur Klassen; Fortschritt als <progress>.
const DysonUI=(()=>{
 const PHASEN=['Gerüst','Äquatorgürtel','Halbschale','Polkappen','vollendet'];
 const PHASE_NAME={sammeln:'Einzahlung läuft',bau:'im Bau',vollendet:'vollendet',verwaist:'verwaist',abgebrochen:'abgebrochen'};
 const ROH=['erz','kristall','deuterium'];
 const d=()=>view?.dyson?.aktiv?view.dyson:null;
 const projekt=()=>d()?.projekte?.find(p=>p.phase!=='abgebrochen')||null;
 const tempo=()=>Math.max(1,world?.tempo||1);
 const dauer=s=>window.Bauseite?Bauseite.dauer(s/tempo()):GameTime.duration(s);
 const phaseVon=f=>PHASEN[Math.min(4,Math.floor(f*4+1e-9))];
 const zahl=x=>fmt(Math.floor(Number(x)||0));

 // ---------------------------------------------------------------- Sonnenansicht (three.js)
 // Eine Szene für die ganze Seite; die Leinwand wandert zwischen Allianzseite und Galaxie-Dialog.
 const Szene=(()=>{
  let r=null,host=null,stand={fortschritt:0,anschluesse:0},laeuft=false,sichtbar=true,frame=0;
  const reduced=matchMedia('(prefers-reduced-motion: reduce)');
  function bauen(){
   if(r||typeof THREE==='undefined')return r;
   const klein=matchMedia('(max-width: 767px)').matches||(navigator.hardwareConcurrency||8)<=4;
   const N=klein?700:1800,NS=klein?5000:18000,FPS=klein?30:60,R=10;
   let renderer;try{renderer=new THREE.WebGLRenderer({antialias:!klein,powerPreference:'low-power',alpha:false});}catch{return null;}
   renderer.setPixelRatio(Math.min(devicePixelRatio,klein?1.25:1.75));renderer.setClearColor(0x030813);renderer.outputColorSpace=THREE.SRGBColorSpace;
   renderer.toneMapping=THREE.ACESFilmicToneMapping;renderer.toneMappingExposure=1.1;renderer.domElement.className='dy-leinwand';
   const scene=new THREE.Scene(),camera=new THREE.PerspectiveCamera(45,1,.1,2000),welt=new THREE.Group();scene.add(welt);
   scene.add(new THREE.AmbientLight(0x8fa6c8,.55));const key=new THREE.DirectionalLight(0xbfd6ff,1.1);key.position.set(-30,24,40);scene.add(key);
   const c=document.createElement('canvas');c.width=c.height=128;const x=c.getContext('2d'),g=x.createRadialGradient(64,64,1,64,64,64);
   g.addColorStop(0,'#fff');g.addColorStop(.15,'rgba(255,240,200,.9)');g.addColorStop(.45,'rgba(255,190,90,.22)');g.addColorStop(1,'rgba(255,160,60,0)');x.fillStyle=g;x.fillRect(0,0,128,128);
   const glut=new THREE.CanvasTexture(c);glut.colorSpace=THREE.SRGBColorSpace;
   const leuchte=(farbe,groesse,deckung,tief=true)=>{const s=new THREE.Sprite(new THREE.SpriteMaterial({map:glut,color:farbe,transparent:true,opacity:deckung,depthWrite:false,depthTest:tief,blending:THREE.AdditiveBlending}));s.scale.setScalar(groesse);return s;};
   const sonne=new THREE.Mesh(new THREE.SphereGeometry(3.2,32,24),new THREE.MeshBasicMaterial({color:0xfff1c4}));welt.add(sonne);
   const korona=leuchte(0xffc860,16,.85);welt.add(korona);const halo=leuchte(0xffb040,46,.18,false);welt.add(halo);
   const geruest=new THREE.LineSegments(new THREE.EdgesGeometry(new THREE.IcosahedronGeometry(R*.985,3)),new THREE.LineBasicMaterial({color:0x9aa7b8,transparent:true,opacity:.55}));welt.add(geruest);
   // Sechseckpaneele auf einer Fibonacci-Kugel, Füllung Äquator → Pole in Breitenbändern (ein Draw-Call).
   const punkte=[],gold=Math.PI*(3-Math.sqrt(5));
   for(let i=0;i<N;i++){const y=1-(i+.5)/N*2,rr=Math.sqrt(1-y*y),a=i*gold;punkte.push(new THREE.Vector3(Math.cos(a)*rr,y,Math.sin(a)*rr));}
   punkte.sort((p,q)=>Math.floor(Math.abs(p.y)*14)-Math.floor(Math.abs(q.y)*14)||Math.atan2(p.z,p.x)-Math.atan2(q.z,q.x));
   const kante=Math.sqrt(4*Math.PI*R*R/N/2.598)*.93,hex=new THREE.CylinderGeometry(kante,kante,.12,6);hex.rotateX(Math.PI/2);
   const mat=new THREE.MeshStandardMaterial({color:0xd9a441,metalness:.85,roughness:.32,emissive:0x5a3606,emissiveIntensity:.35});
   const paneele=new THREE.InstancedMesh(hex,mat,N);paneele.instanceMatrix.setUsage(THREE.DynamicDrawUsage);welt.add(paneele);
   const dummy=new THREE.Object3D();
   const kraene=[];for(let i=0;i<(klein?4:8);i++){const gr=new THREE.Group();gr.add(new THREE.Line(new THREE.BufferGeometry().setFromPoints([new THREE.Vector3(),new THREE.Vector3(0,0,1.6)]),new THREE.LineBasicMaterial({color:0xc8d4e4})));
    const k=leuchte(0xfff0b0,.9,.9);k.position.z=1.6;gr.add(k);gr.userData={phase:i/(klein?4:8)*Math.PI*2,nord:i%2===0};welt.add(gr);kraene.push(gr);}
   // Kollektorschwarm: Bahnen im Vertex-Shader aus der Zeit, keine CPU-Arbeit je Bild.
   const BAHNEN=[[0,0],[.42,.6],[-.38,2.1],[.75,3.4],[-.7,4.6],[1.15,1.2]],pos=new Float32Array(NS*3),attr=new Float32Array(NS*4);
   let zz=12345;const zufall=()=>((zz=Math.imul(zz^zz>>>15,2246822507)+1013904223>>>0)/4294967296);
   for(let i=0;i<NS;i++){const b=BAHNEN[i%BAHNEN.length];attr.set([R*(1.2+(i%BAHNEN.length)*.07)+zufall()*R*.12,zufall()*Math.PI*2,(.05+zufall()*.04)*(i%7?1:-1)*(1.6-(i%BAHNEN.length)*.08),b[0]+(zufall()-.5)*.06],i*4);pos[i*3+2]=b[1];}
   const sg=new THREE.BufferGeometry();sg.setAttribute('position',new THREE.BufferAttribute(pos,3));sg.setAttribute('bahn',new THREE.BufferAttribute(attr,4));
   const smat=new THREE.ShaderMaterial({transparent:true,depthWrite:false,blending:THREE.AdditiveBlending,
    uniforms:{zeit:{value:0},anteil:{value:1},px:{value:renderer.getPixelRatio()},farbe:{value:new THREE.Color(0xffd37a)}},
    vertexShader:'attribute vec4 bahn;uniform float zeit,anteil,px;varying float sicht;void main(){float r=bahn.x,w=bahn.y+zeit*bahn.z,i=bahn.w,k=position.z;vec3 p=vec3(cos(w)*r,0.,sin(w)*r);p=vec3(p.x,p.z*sin(i),p.z*cos(i));p=vec3(p.x*cos(k)-p.z*sin(k),p.y,p.x*sin(k)+p.z*cos(k));sicht=step(fract(bahn.y*7.31),anteil);vec4 mv=modelViewMatrix*vec4(p,1.);gl_Position=projectionMatrix*mv;gl_PointSize=sicht*px*clamp(70./-mv.z,1.,3.);}',
    fragmentShader:'uniform vec3 farbe;varying float sicht;void main(){if(sicht<.5)discard;vec2 c=gl_PointCoord-.5;float a=smoothstep(.5,.1,length(c));gl_FragColor=vec4(farbe,a*.9);}'});
   welt.add(new THREE.Points(sg,smat));
   // Angeschlossene Planeten mit Spiegelsatellit und Strahl (nur vollendet).
   const ansch=new THREE.Group();welt.add(ansch);const planeten=[],pf=[0x5f8fd6,0xc96a3c,0x7fb889,0xb9c6d6];
   for(let i=0;i<4;i++){const p=new THREE.Mesh(new THREE.SphereGeometry(.9,20,14),new THREE.MeshStandardMaterial({color:pf[i],roughness:.8}));const bahn=26+i*7;p.userData={bahn,w:i*1.7,v:.05/(1+i*.4)};
    const strahl=new THREE.Line(new THREE.BufferGeometry().setFromPoints([new THREE.Vector3(),new THREE.Vector3()]),new THREE.LineBasicMaterial({color:0xffd27a,transparent:true,opacity:.7,blending:THREE.AdditiveBlending}));
    const spiegel=leuchte(0xffffff,1.4,1),ring=new THREE.LineLoop(new THREE.BufferGeometry().setFromPoints(Array.from({length:97},(_,j)=>new THREE.Vector3(Math.cos(j/96*Math.PI*2)*bahn,0,Math.sin(j/96*Math.PI*2)*bahn))),new THREE.LineBasicMaterial({color:0x4a6a8f,transparent:true,opacity:.35}));
    const gr=new THREE.Group();gr.add(p,strahl,spiegel,ring);ansch.add(gr);planeten.push({gr,p,strahl,spiegel});}
   let gesetzt=-1,zeit=0,letzte=performance.now();
   function setzen(){
    const f=Math.max(0,Math.min(1,stand.fortschritt)),n=Math.round(f*N);
    if(n!==gesetzt){gesetzt=n;for(let i=0;i<N;i++){dummy.position.copy(punkte[i]).multiplyScalar(R);dummy.lookAt(0,0,0);dummy.scale.setScalar(i<n?1:0);dummy.updateMatrix();paneele.setMatrixAt(i,dummy.matrix);}paneele.instanceMatrix.needsUpdate=true;}
    geruest.material.opacity=.55*(1-f)+.05;smat.uniforms.anteil.value=.15+.85*f;mat.emissiveIntensity=f>=1?.75:.35;korona.material.opacity=.85*(1-f*.7);
    planeten.forEach((q,i)=>{q.gr.visible=f>=1&&i<stand.anschluesse;});
    const front=n<N?Math.abs(punkte[Math.max(0,n-1)].y):1;
    kraene.forEach(k=>{k.visible=f>0&&f<1;k.userData.front=front;});
    kraeneSetzen();
   }
   function kraeneSetzen(){kraene.forEach(k=>{const y=(k.userData.nord?1:-1)*Math.min(.97,(k.userData.front||0)+.04),rr=Math.sqrt(1-y*y),a=k.userData.phase+zeit*.08;k.position.set(Math.cos(a)*rr*R*1.04,y*R*1.04,Math.sin(a)*rr*R*1.04);k.lookAt(0,0,0);k.rotateY(Math.PI);});}
   function groesse(){if(!host)return;const w=Math.max(1,host.clientWidth),h=Math.max(1,host.clientHeight);renderer.setSize(w,h,false);camera.aspect=w/h;
    // Motiv nie beschnitten: Kugel samt äußerster Schwarmbahn (bzw. Anschlussbahnen) passt vollständig in die Fläche.
    const radius=stand.fortschritt>=1&&stand.anschluesse>0?50:R*1.75,tanH=Math.tan(THREE.MathUtils.degToRad(camera.fov/2));
    const abstand=Math.max(radius/tanH,radius/(tanH*camera.aspect))*1.08;camera.position.set(0,abstand*.3,abstand*.954);camera.lookAt(0,0,0);camera.updateProjectionMatrix();}
   function bild(jetzt){
    if(!laeuft)return;requestAnimationFrame(bild);
    if(!sichtbar||!host||document.visibilityState!=='visible')return;
    // Reduzierte Bewegung: kein Dauerzeichnen; das Bild entsteht nur bei Änderungen (zeigen, Ziehen).
    if(reduced.matches)return;
    if(jetzt-letzte<1000/FPS-1)return;const dt=Math.min(.1,(jetzt-letzte)/1000);letzte=jetzt;
    zeit+=dt;welt.rotation.y+=dt*.05;
    smat.uniforms.zeit.value=zeit;sonne.scale.setScalar(1+Math.sin(zeit*1.3)*.015);halo.material.opacity=.14+.06*stand.fortschritt+Math.sin(zeit*.7)*.02;
    kraeneSetzen();
    planeten.forEach(({p,strahl,spiegel},i)=>{const u=p.userData,w=u.w+zeit*u.v;p.position.set(Math.cos(w)*u.bahn,0,Math.sin(w)*u.bahn);const s=p.position.clone().normalize().multiplyScalar(R*1.6);spiegel.position.copy(s);
     const a=strahl.geometry.attributes.position;a.setXYZ(0,s.x,s.y,s.z);a.setXYZ(1,p.position.x,p.position.y,p.position.z);a.needsUpdate=true;strahl.material.opacity=.45+.3*Math.sin(zeit*3+i);});
    renderer.render(scene,camera);frame++;
   }
   const beob=new ResizeObserver(()=>groesse());
   const sicht=new IntersectionObserver(e=>{sichtbar=e.some(x=>x.isIntersecting);});
   // Ziehen dreht die Ansicht (Maus und Touch).
   let ziehen=null;renderer.domElement.addEventListener('pointerdown',e=>{ziehen={x:e.clientX,y:e.clientY};renderer.domElement.setPointerCapture(e.pointerId);});
   renderer.domElement.addEventListener('pointermove',e=>{if(!ziehen)return;welt.rotation.y+=(e.clientX-ziehen.x)*.005;welt.rotation.x=Math.max(-.8,Math.min(.8,welt.rotation.x+(e.clientY-ziehen.y)*.004));ziehen={x:e.clientX,y:e.clientY};if(reduced.matches)renderer.render(scene,camera);});
   renderer.domElement.addEventListener('pointerup',()=>ziehen=null);
   r={renderer,setzen,groesse,bild,beob,sicht,einmal:()=>renderer.render(scene,camera),stufe:{N,NS,FPS}};return r;
  }
  function zeigen(ziel,neu){
   if(!ziel)return false;stand={...stand,...neu};
   const s=bauen();if(!s){ziel.classList.add('dy-ohne-webgl');return false;}
   if(host!==ziel){if(host){s.beob.unobserve(host);s.sicht.unobserve(host);}host=ziel;host.append(s.renderer.domElement);s.beob.observe(host);s.sicht.observe(host);}
   s.setzen();s.groesse();s.einmal();
   if(!laeuft){laeuft=true;requestAnimationFrame(s.bild);}
   return true;
  }
  function anhalten(){laeuft=false;}
  return {zeigen,anhalten,frames:()=>frame,stufe:()=>r?.stufe||null};
 })();

 // ---------------------------------------------------------------- Allianz-Reiter „Großprojekt“
 function abschnitt(){
  let s=$('grossprojekt');
  if(!d()){if(s&&s.hidden)s.remove();return null;}
  if(!s){
   const nach=$('allianzbereich');if(!nach)return null;
   s=document.createElement('section');s.className='panel';s.id='grossprojekt';s.hidden=true;
   s.innerHTML='<span class="eyebrow">ALLIANZWUNDER</span><h2>Großprojekt – Dysonsphäre</h2><div class="dy-kopf"><figure class="dy-banner"><img id="dy-banner-bild" alt="Dysonsphäre"><figcaption id="dy-banner-text"></figcaption></figure><div class="dy-szene" id="dy-szene" role="img" aria-label="Animierte Sonnenansicht der Dysonsphäre"></div></div><div id="dy-inhalt"></div>';
   nach.after(s);if(window.Rahmen?.synchronisieren)Rahmen.synchronisieren();
  }
  return s;
 }
 function bannerBild(p){return p?.phase==='vollendet'||p?.phase==='verwaist'?art('buildings','dysonsphaere',''):art('buildings','dysonsphaere_bau',view?.volk||'');}
 function fortschritt(p){return p?(p.phase==='vollendet'||p.phase==='verwaist'?1:(p.fortschritt_promille||0)/1000):0;}
 function balken(wert,soll,titel){const v=Math.min(soll,Math.max(0,wert));return `<progress class="dy-balken${v>=soll?' voll':''}" max="${soll}" value="${v}" title="${esc(titel)}">${esc(titel)}</progress>`;}
 function bedingungen(p){
  if(!p.bedingungen?.length)return '';
  return `<section class="dy-block"><h3>Bedingungen für den Baubeginn</h3><ul class="dy-bedingungen">${p.bedingungen.map(b=>`<li class="${b.erfuellt?'ok':'fehlt'}"><span aria-hidden="true">${b.erfuellt?'✔':'✖'}</span> ${esc(b.text)} <b>${zahl(b.ist)}/${zahl(b.soll)}</b></li>`).join('')}</ul></section>`;
 }
 function mitglieder(p){
  const kopf='<tr><th>Mitglied</th><th>Erz</th><th>Kristall</th><th>Deuterium</th><th>Status</th></tr>';
  const zeilen=p.mitglieder.map(m=>`<tr class="${m.eigene?'eigen':''}"><td><b>${esc(m.name)}</b><small>${esc(named(m.volk))} · Stufe ${esc(m.stufe)}</small></td>${ROH.map((g,i)=>`<td>${balken(m.eingezahlt[i],p.anteil,named(g)+': '+zahl(m.eingezahlt[i])+' von '+zahl(p.anteil))}<small>${zahl(m.eingezahlt[i])}</small></td>`).join('')}<td>${m.voll?'<span class="dy-ok">voll ✓</span>':'zahlt ein'}${!m.in_allianz?' · <span class="dy-warn">nicht in der Allianz</span>':''}${!m.planet_im_system?' · <span class="dy-warn">kein Planet im System</span>':''}</td></tr>`).join('');
  return `<section class="dy-block"><h3>Projektkasse · Anteil je ${zahl(p.anteil)} Erz, Kristall und Deuterium</h3><p class="dy-summe">Kasse: ${ROH.map((g,i)=>`${zahl(p.kasse[i])} ${esc(named(g))}`).join(' · ')}</p><div class="dy-tabelle-rahmen"><table class="dy-tabelle"><thead>${kopf}</thead><tbody>${zeilen||'<tr><td colspan="5">Noch niemand beigetreten.</td></tr>'}</tbody></table></div></section>`;
 }
 function einzahlung(p){
  if(!p.mitglied){
   const kann=p.eigene_planeten.length>0&&(p.phase==='sammeln'||p.phase==='bau');
   return `<section class="dy-block dy-detail"><h3>Mitmachen</h3><p>${kann?'Du hast einen Planeten im System und kannst dem Projekt beitreten.':'Beitreten können Allianzmitglieder mit eigenem Planeten im System '+esc(p.system)+'.'}</p>${kann?`<button type="button" class="dy-gruen" data-dyson="dyson_beitreten">Dem Projekt beitreten</button>`:''}</section>`;
  }
  const eigen=p.mitglieder.find(m=>m.eigene),rest=ROH.map((g,i)=>Math.max(0,p.anteil-(eigen?.eingezahlt[i]||0)));
  // Austritt vor Baubeginn auch mit voller Einzahlung (Kern dyson_austreten verlangt nur Phase „sammeln“) – Funktionsprüfung 25.10.2026.
  if(eigen?.voll)return `<section class="dy-block dy-detail"><h3>Dein Anteil</h3><p class="dy-ok">Voll eingezahlt – danke.</p>${p.phase==='sammeln'?`<p class="dy-hinweis"><button type="button" class="bs-link" data-dyson="dyson_austreten">Austreten</button> (Rückzahlung ${Math.round((d().regeln.rueckzahlung_austritt||0)*100)} % als Guthaben)</p>`:''}</section>`;
  const orte=[...p.eigene_planeten.map(k=>`<option value="planet|${esc(k)}">Planet [${esc(k)}]</option>`),...p.eigene_monde.map(k=>`<option value="mond|${esc(k)}">Mond [${esc(k)}]</option>`)].join('');
  return `<section class="dy-block dy-detail"><h3>Einzahlen</h3><form id="dy-einzahlen" class="dy-form"><label>Von<select name="von">${orte}</select></label>${ROH.map((g,i)=>`<label>${esc(named(g))} <small>[Rest ${zahl(rest[i])}]</small><input name="${g}" type="number" min="0" max="${rest[i]}" step="1" value="0" inputmode="numeric"></label>`).join('')}<button class="dy-gruen">Einzahlen</button></form><p class="dy-hinweis">Mindestbuchung ${zahl(p.mindestbuchung)} je Gut (oder genau der Rest). Was in der Kasse liegt, ist nicht plünderbar.${p.phase==='sammeln'?' <button type="button" class="bs-link" data-dyson="dyson_austreten">Austreten</button> (Rückzahlung '+Math.round((d().regeln.rueckzahlung_austritt||0)*100)+' %)':''}</p></section>`;
 }
 function leitung(p){
  if(!p.leitung)return '';
  const teile=[];
  if(p.phase==='sammeln'){const bereit=(p.bedingungen||[]).every(b=>b.erfuellt);teile.push(`<button type="button" class="dy-gruen" data-dyson="dyson_bau_starten" ${bereit?'':'disabled'}>Bau starten (${esc(d().regeln.bau_tage)} Spieltage)</button>${bereit?'':`<small class="dy-hinweis">Fehlt: ${esc((p.bedingungen||[]).filter(b=>!b.erfuellt).map(b=>b.text||b.name||'').filter(Boolean).join(' · '))}</small>`}`);}
  if(p.phase==='vollendet'){
   const optionen=[...new Set(p.mitglieder.map(m=>m.name))];
   teile.push(`<div class="dy-slots">${p.anschluesse.map(a=>a.planet?`<div class="dy-slot"><b>Anschluss ${a.slot}</b> ⚡ [${esc(a.planet)}] · ${esc(a.besitzer)}<small>${a.aktiv?'aktiv':'aktiv in '+dauer(a.aktiv_ab-GameTime.now())}${a.sperre_bis>GameTime.now()?' · gesperrt noch '+dauer(a.sperre_bis-GameTime.now()):''}</small>${a.sperre_bis>GameTime.now()?'':`<button type="button" class="bs-link" data-dyson="dyson_anschluss_loesen" data-slot="${a.slot}">Lösen</button>`}</div>`:`<form class="dy-slot" data-dyson-slot="${a.slot}"><b>Anschluss ${a.slot}</b><input name="planet" placeholder="Planet S:SSS:P im System" pattern="\\d+:\\d+:\\d+" required>${a.sperre_bis>GameTime.now()?`<small>gesperrt noch ${dauer(a.sperre_bis-GameTime.now())}</small>`:'<button class="dy-gruen">Anschließen</button>'}</form>`).join('')}</div><p class="dy-hinweis">Höchstens ${esc(d().regeln.anschluesse_je_spieler)} je Spieler, nur Planeten von Projektmitgliedern im System. Aktiv nach ${esc(d().regeln.anschluss_vorlauf_stunden)} h, danach ${esc(d().regeln.anschluss_sperre_tage)} Tage gesperrt. Mitglieder: ${optionen.map(esc).join(', ')}</p>`);
  }
  if(p.phase==='sammeln'||p.phase==='bau')teile.push(`<button type="button" class="dy-rot" data-dyson="dyson_abbrechen">Projekt abbrechen</button>`);
  return teile.length?`<section class="dy-block"><h3>Allianzleitung</h3>${teile.join('')}</section>`:'';
 }
 function wirkung(){
  const r=d().regeln;
  return `<section class="dy-block"><h3>Wirkung bei Fertigstellung</h3><ul class="dy-wirkung"><li>Solarkraftwerke und Solarsatelliten der Allianz im System: Sonnenfaktor × ${esc(String(r.sonnenfaktor).replace('.',','))}</li><li>Bis zu ${esc(r.anschluesse)} angeschlossene Planeten: ${zahl(r.einspeisung)} Energie je Stunde, nie Energiemangel</li><li>Strahlsynthese je Anschluss: ${zahl(r.strahlsynthese_deuterium)} Deuterium und ${zahl(r.strahlsynthese_xenokristall)} Xenokristall je Stunde; Deuteriumsynthesizer und Xenoextraktor × ${esc(r.strombad)}</li><li>Wirkt, solange die Allianz mindestens ${esc(r.halten_min_plaetze)} der Plätze hält.</li></ul></section>`;
 }
 function status(p){
  const f=fortschritt(p),teile=[`<b>${esc(PHASE_NAME[p.phase]||p.phase)}</b>`];
  if(p.phase==='bau')teile.push(`${Math.floor(f*100)} % · ${esc(phaseVon(f))}`,p.ruht?`<span class="dy-warn">ruht: ${esc(p.ruhe_grund)} · Abbruch in ${dauer(p.ruhe_rest_sekunden||0)}</span>`:`fertig in <b data-arrival="${esc(GameTime.now()+(p.rest_sekunden||0))}">${dauer(p.rest_sekunden||0)}</b>`);
  if(p.phase==='verwaist')teile.push('<span class="dy-warn">keine Wirkung – zu wenige Plätze gehalten</span>');
  teile.push(`Plätze ${zahl(p.plaetze.allianz)}/${zahl(p.plaetze.gesamt)}`);
  return `<p class="dy-status">System <span class="koord">[${esc(p.system)}]</span> · ${teile.join(' · ')}</p>${p.phase==='bau'?balken(Math.round(f*1000),1000,'Baufortschritt '+Math.floor(f*100)+' %'):''}<ol class="dy-phasen">${PHASEN.map((n,i)=>`<li class="${f>=i/4-1e-9&&(i<4||f>=1)?'an':''}">${i*25} % ${esc(n)}</li>`).join('')}</ol>`;
 }
 // Nach einem Abbruch gesperrte Systeme vor dem Absenden zeigen (Kern dyson_sicht.gesperrt; Funktionsprüfung 25.10.2026, dyson#13).
 function gesperrt(){const g=d().gesperrt||[];return g.length?`<p class="dy-hinweis dy-gesperrt">Nach Abbruch gesperrt: ${g.map(x=>`System [${esc(x.system)}] bis ${esc(x.bis_text)}`).join(' · ')}</p>`:'';}
 function ohneProjekt(){
  return `<section class="dy-block dy-detail"><h3>Noch kein Projekt</h3><p>Die Dysonsphäre ist das größte Allianzwerk: mindestens ${esc(d().regeln.mitglieder_min)} Mitglieder auf Stufe ${esc(d().regeln.stufe_min)} besiedeln alle Plätze eines Systems und zahlen je ${zahl(d().anteil)} Erz, Kristall und Deuterium ein. Vorschlagen darf die Allianzleitung.</p><form id="dy-vorschlag" class="dy-form"><label>System (Sektor:System)<input name="system" placeholder="1:24" pattern="\\d+:\\d+" required></label><button class="dy-gruen">Dysonsphäre vorschlagen</button></form><p class="dy-hinweis">Dafür brauchst du eine Allianz mit mindestens einem Planeten im System.</p>${gesperrt()}</section>`;
 }
 function guthaben(){
  const g=d().guthaben||[0,0,0];if(!g.some(x=>x>0))return '';
  const orte=(view.planeten||[]).map(p=>`<option value="${esc(p.koord)}">[${esc(p.koord)}]</option>`).join('');
  return `<section class="dy-block dy-detail"><h3>Rückzahlungsguthaben</h3><p>${ROH.map((x,i)=>`${zahl(g[i])} ${esc(named(x))}`).join(' · ')}</p><form id="dy-auszahlen" class="dy-form"><label>Auf Planet<select name="planet">${orte}</select></label><button class="dy-gruen">Auszahlen (bis zur Lagergrenze)</button></form></section>`;
 }
 function render(){
  const s=abschnitt();if(!s||s.hidden)return;
  const p=projekt();
  const img=$('dy-banner-bild'),src=bannerBild(p);if(img&&img.getAttribute('src')!==src)img.setAttribute('src',src);
  setStableHTML($('dy-banner-text'),p?`Dysonsphäre · System [${esc(p.system)}] · [${esc(p.allianz_tag)}]`:'Dysonsphäre – noch kein Projekt');
  Szene.zeigen($('dy-szene'),{fortschritt:fortschritt(p),anschluesse:p?p.anschluesse.filter(a=>a.planet&&a.aktiv).length:0});
  setStableHTML($('dy-inhalt'),p?status(p)+bedingungen(p)+einzahlung(p)+leitung(p)+mitglieder(p)+wirkung()+guthaben():ohneProjekt()+wirkung()+guthaben());
 }

 // ---------------------------------------------------------------- Galaxie und Planetenliste
 function galaxieZeile(daten){
  const x=daten?.dyson;if(!x)return '';
  // Vor Baubeginn gibt es keine Sphäre (Funktionsprüfung 25.10.2026: „Vollendet“ in Phase sammeln).
  const f=x.phase==='bau'?x.fortschritt_prozent/100:x.phase==='sammeln'?0:1;
  const text=x.phase==='bau'?`im Bau ${x.fortschritt_prozent} % (${esc(phaseVon(f))})${x.ruht?' · ruht':''}${x.eigen&&x.rest_sekunden?' · fertig in '+dauer(x.rest_sekunden):''}`:esc(PHASE_NAME[x.phase]||x.phase);
  return `<tr class="gt-dyson"><td colspan="8"><button type="button" class="gt-dyson-knopf" data-dyson-sonne="${esc(x.system)}" data-fortschritt="${f}" data-anschluesse="${(x.anschluesse||[]).length}" title="Sonnenansicht öffnen"><span class="gt-dyson-symbol" aria-hidden="true">☀</span> Dysonsphäre</button> · [${esc(x.allianz_tag)}] · ${text}</td></tr>`;
 }
 function angeschlossen(koord){const x=d();if(!x)return false;return (x.eigene_anschluesse||[]).includes(koord)||(x.projekte||[]).some(p=>p.anschluesse.some(a=>a.planet===koord));}
 function blitz(koord,klasse='dy-blitz'){return angeschlossen(koord)?`<span class="${klasse}" title="An die Dysonsphäre angeschlossen" aria-label="An die Dysonsphäre angeschlossen">⚡</span>`:'';}
 function dialogOeffnen(system,f,n){
  let dlg=$('dy-dialog');
  if(!dlg){dlg=document.createElement('dialog');dlg.id='dy-dialog';dlg.className='dy-dialog';dlg.innerHTML='<div class="dy-dialog-kopf"><h3 id="dy-dialog-titel">Dysonsphäre</h3><button type="button" class="dy-zu" data-dyson-zu aria-label="Schließen">✕</button></div><div class="dy-szene dy-szene-gross" id="dy-dialog-szene" role="img" aria-label="Animierte Sonnenansicht"></div><p class="dy-hinweis" id="dy-dialog-text"></p>';document.body.append(dlg);}
  setStableHTML($('dy-dialog-titel'),`Dysonsphäre · System [${esc(system)}]`);
  setStableHTML($('dy-dialog-text'),f>=1?'Vollendet: Milliarden Kollektoren auf geneigten Bahnen, Spiegelsatelliten bündeln das Licht auf die Anschlüsse.':`Bau ${Math.floor(f*100)} % – ${esc(phaseVon(f))}. Die Paneele füllen die Kugel vom Äquator zu den Polen.`);
  dlg.showModal();Szene.zeigen($('dy-dialog-szene'),{fortschritt:f,anschluesse:n});
 }
 document.addEventListener('click',guard(async e=>{
  const b=e.target.closest('[data-dyson],[data-dyson-sonne],[data-dyson-zu]');if(!b)return;
  if(b.dataset.dysonZu!=null){$('dy-dialog')?.close();if(!$('grossprojekt')?.hidden)render();return;}
  if(b.dataset.dysonSonne){dialogOeffnen(b.dataset.dysonSonne,Number(b.dataset.fortschritt)||0,Number(b.dataset.anschluesse)||0);return;}
  const p=projekt(),typ=b.dataset.dyson,a={typ};if(p)a.projekt=p.id;
  if(typ==='dyson_abbrechen'){if(!confirm('Dysonprojekt wirklich abbrechen? Verbautes ist verloren.'))return;a.bestaetigt=true;}
  if(typ==='dyson_anschluss_loesen')a.slot=Number(b.dataset.slot);
  await command([a]);
 }));
 // Eigener Wrapper statt guard(): andere Module sperren mit guard() den Absendeknopf, solange ihr eigener
 // Submit-Handler läuft – dann sähe guard() hier einen gesperrten Knopf und täte nichts.
 const absenden=fn=>async e=>{const f=e.target;if(!f.closest?.('#grossprojekt'))return;e.preventDefault();const k=e.submitter;if(k?._dyBusy)return;if(k){k._dyBusy=true;}try{await fn(e);}catch(err){show(err.message,true);}finally{if(k)k._dyBusy=false;}};
 document.addEventListener('submit',absenden(async e=>{
  const f=e.target;
  const p=projekt();
  if(f.id==='dy-vorschlag'){await command([{typ:'dyson_vorschlagen',system:f.elements.system.value.trim()}]);return;}
  if(f.id==='dy-einzahlen'){const [art,von]=f.elements.von.value.split('|'),a={typ:'dyson_einzahlen',projekt:p.id,von,zielart:art};for(const g of ROH){const x=Math.floor(Number(f.elements[g].value)||0);if(x>0)a[g]=x;}await command([a]);return;}
  if(f.id==='dy-auszahlen'){const g=d().guthaben;await command([{typ:'dyson_auszahlen',planet:f.elements.planet.value,erz:g[0],kristall:g[1],deuterium:g[2]}]);return;}
  if(f.dataset.dysonSlot){await command([{typ:'dyson_anschluss_waehlen',projekt:p.id,slot:Number(f.dataset.dysonSlot),planet:f.elements.planet.value.trim()}]);}
 }));
 document.addEventListener('game-navigation',()=>{abschnitt();render();});
 return {render,galaxieZeile,blitz,angeschlossen,Szene,abschnitt};
})();
window.DysonUI=DysonUI;
{
 const ansicht=renderView;renderView=function(){ansicht.apply(this,arguments);DysonUI.abschnitt();DysonUI.render();};
}
