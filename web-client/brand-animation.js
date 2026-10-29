// Shared deterministic renderer: one small texture instead of hundreds of bitmaps.
'use strict';
window.neuralStarDuration=3.2;
// Decode once even when the loader and navigation use the same texture.
const neuralStarTextures=new Map();
window.loadNeuralStarTexture=function(url,fallback){
 const key=url+'|'+(fallback||'');
 if(!neuralStarTextures.has(key))neuralStarTextures.set(key,(async()=>{
  const source=new Image();source.src=url;
  try{await source.decode();}catch(error){if(!fallback)throw error;source.src=fallback;await source.decode();}
  return source;
 })().catch(error=>{neuralStarTextures.delete(key);throw error;}));
 return neuralStarTextures.get(key);
};
window.createNeuralStar=function(canvas,source,size=768){
const ctx=canvas.getContext('2d');
if(!ctx)throw new Error('Canvas unavailable');
canvas.width=canvas.height=size;
const SIZE=canvas.width,DURATION=window.neuralStarDuration,SOURCE_SIZE=1254;
const center=[629,620],scale=.72;
const core=[[629,486],[663,563],[685,583],[761,620],[685,654],[662,680],[629,745],[595,680],[573,655],[491,620],[573,584],[595,562]];
const regions=[[[629,620],[0,196],[0,0],[1280,0],[1280,170]],[[629,620],[1280,170],[1280,1105]],[[629,620],[1280,1105],[1280,1280],[0,1280],[0,1084]],[[629,620],[0,1084],[0,196]]];
const travel=[[0,-150],[210,0],[0,150],[-210,0]],panels=[],counts=[4,6,8,8,6,4,3];
const nodes=counts.map((n,l)=>Array.from({length:n},(_,i)=>({x:400+l*80,y:640+(i-(n-1)/2)*43})));
const clamp=x=>Math.max(0,Math.min(1,x)),smooth=x=>{x=clamp(x);return x*x*x*(x*(x*6-15)+10);},mix=(a,b,t)=>a+(b-a)*t;
function polygon(c,points){c.moveTo(...points[0]);for(const p of points.slice(1))c.lineTo(...p);c.closePath();}
const ready=source.decode().then(()=>{for(const region of regions){
 const texture=document.createElement('canvas');texture.width=texture.height=Math.max(1,Math.ceil(SIZE*scale));
 const c=texture.getContext('2d'),ratio=texture.width/1280;c.setTransform(ratio,0,0,ratio,0,0);
 c.beginPath();polygon(c,region);c.clip();c.drawImage(source,0,0,SOURCE_SIZE,SOURCE_SIZE);c.globalCompositeOperation='destination-out';c.beginPath();polygon(c,core);c.fill();
 // Bake the panel shadow once; moving panels no longer blur large bitmaps every frame.
 const shaded=document.createElement('canvas');shaded.width=shaded.height=texture.width;
 const s=shaded.getContext('2d');s.shadowColor='rgba(0,2,6,.65)';s.shadowBlur=18*ratio;s.shadowOffsetY=9*ratio;s.drawImage(texture,0,0);panels.push(shaded);
}});
const glows=new Map();
function halo(x,y,r,color,alpha){
 if(alpha<=0)return;const key=r+'|'+color;
 if(!glows.has(key)){
  const sprite=document.createElement('canvas');sprite.width=sprite.height=Math.max(2,Math.ceil(r*2*SIZE/1280));
  const c=sprite.getContext('2d'),radius=sprite.width/2,g=c.createRadialGradient(radius,radius,0,radius,radius,radius);
  g.addColorStop(0,`rgba(${color},1)`);g.addColorStop(.23,`rgba(${color},.32)`);g.addColorStop(1,`rgba(${color},0)`);c.fillStyle=g;c.fillRect(0,0,sprite.width,sprite.height);glows.set(key,sprite);
 }
 const opacity=ctx.globalAlpha;ctx.globalAlpha=opacity*alpha;ctx.drawImage(glows.get(key),x-r,y-r,r*2,r*2);ctx.globalAlpha=opacity;
}
// A stationary backdrop avoids repainting a full field of stars in GIF exports.
const backdrop=document.createElement('canvas');backdrop.width=backdrop.height=SIZE;
let backdropReady=false;
function background(){
 if(backdropReady){ctx.drawImage(backdrop,0,0,1280,1280);return;}
 ctx.fillStyle='#050b14';ctx.fillRect(0,0,1280,1280);halo(640,640,610,'35,83,112',.16);halo(640,640,360,'39,151,172',.07);
 for(let i=0;i<65;i++){const x=(i*331+41)%1280,y=(i*617+53)%1280;ctx.globalAlpha=.13;ctx.fillStyle=i%7?'#b4cbd5':'#d8ba82';ctx.fillRect(x,y,i%13?1:2,i%13?1:2);}ctx.globalAlpha=1;
 backdrop.getContext('2d').drawImage(canvas,0,0);backdropReady=true;
}
const wiring=document.createElement('canvas');wiring.width=wiring.height=SIZE;
const wire=wiring.getContext('2d');wire.setTransform(SIZE/1280,0,0,SIZE/1280,0,0);wire.lineWidth=.75;wire.strokeStyle='rgba(103,192,205,.13)';wire.beginPath();
for(let l=0;l<6;l++)for(const a of nodes[l])for(const b of nodes[l+1]){wire.moveTo(a.x,a.y);wire.lineTo(b.x,b.y);}wire.stroke();
function network(t,opacity){
 if(opacity<=0)return;ctx.save();ctx.globalAlpha=opacity;
 // One signal wave traverses all seven layers before the panels fold shut.
 const stage=(t-.85)/1.4*7,transmitting=t>=.85&&t<2.25;
 halo(640,640,315,'39,145,170',.10);ctx.drawImage(wiring,0,0,1280,1280);
 for(let l=0;l<6;l++){
  const f=stage-l-.3;if(!transmitting||f<0||f>1)continue;const active=1-Math.abs(f-.5)*1.3;
  ctx.strokeStyle=`rgba(115,237,226,${active*.24})`;ctx.lineWidth=1.1;ctx.beginPath();for(const a of nodes[l])for(const b of nodes[l+1]){ctx.moveTo(a.x,a.y);ctx.lineTo(b.x,b.y);}ctx.stroke();
  for(const a of nodes[l])for(const b of nodes[l+1]){const x=mix(a.x,b.x,f),y=mix(a.y,b.y,f);ctx.fillStyle=l>3?'#a9f3bb':'#b9f8ff';ctx.globalAlpha=opacity*.78;ctx.beginPath();ctx.arc(x,y,1.45,0,Math.PI*2);ctx.fill();}ctx.globalAlpha=opacity;
 }
 for(let l=0;l<7;l++){const a=transmitting?Math.max(0,1-Math.abs(stage-l-.3)*1.5):0;for(const n of nodes[l]){if(a>.05)halo(n.x,n.y,22,'96,232,216',a*.3);ctx.fillStyle='rgba(7,24,34,.8)';ctx.strokeStyle=a>.05?`rgba(161,253,224,${.45+a*.55})`:'rgba(120,208,218,.65)';ctx.lineWidth=1.3+a;ctx.beginPath();ctx.arc(n.x,n.y,6.4+a*1.4,0,Math.PI*2);ctx.fill();ctx.stroke();ctx.fillStyle=a>.05?'#dcfff3':'#6daab9';ctx.beginPath();ctx.arc(n.x,n.y,1.6+a*1.7,0,Math.PI*2);ctx.fill();}}
 ctx.restore();
}
function emblem(t,open){
 ctx.save();
 if(open<.0001)ctx.drawImage(source,640-center[0]*scale,640-center[1]*scale,SOURCE_SIZE*scale,SOURCE_SIZE*scale);
 else{
  for(let i=0;i<4;i++){ctx.save();ctx.translate(640+travel[i][0]*open,640+travel[i][1]*open);ctx.rotate((i%2?1:-1)*.032*open);ctx.scale(1-(i%2?.065:.025)*open,1-(i%2?.025:.065)*open);ctx.drawImage(panels[i],-center[0]*scale,-center[1]*scale,1280*scale,1280*scale);ctx.restore();}
  const seal=1-smooth(open/.65);ctx.save();ctx.globalAlpha=seal;ctx.translate(640,640);ctx.scale(1-open*.6,1-open*.6);ctx.translate(-center[0]*scale,-center[1]*scale);ctx.beginPath();polygon(ctx,core.map(p=>p.map(v=>v*scale)));ctx.clip();ctx.drawImage(source,0,0,SOURCE_SIZE*scale,SOURCE_SIZE*scale);ctx.restore();
 }
 ctx.restore();halo(640,640,88,'105,234,250',(1-open)*.15);
}
function draw(t){t=clamp(t/DURATION)*DURATION;const open=smooth(t/.75)*(1-smooth((t-2.25)/.75)),opacity=smooth((t-.35)/.4)*(1-smooth((t-2.3)/.5));ctx.setTransform(SIZE/1280,0,0,SIZE/1280,0,0);ctx.globalAlpha=1;ctx.globalCompositeOperation='source-over';background();network(t,opacity);emblem(t,open);}
return {ready,draw,duration:DURATION,layers:counts};
};

// Play once while visible, pause off-screen, then keep the final folded frame.
window.createNeuralStarSchedule=function({draw,duration=window.neuralStarDuration,now=()=>performance.now(),
  later=setTimeout,cancel=clearTimeout,request=requestAnimationFrame,cancelFrame=cancelAnimationFrame}){
 let due=now(),phase='waiting',active=false,elapsed=0,started=0,timer=0,frame=0,disposed=false;
 function clear(){cancel(timer);cancelFrame(frame);timer=frame=0;}
 function plan(){if(active&&!disposed&&phase==='waiting')timer=later(begin,Math.max(0,due-now()));}
 function begin(){timer=0;if(!active||disposed)return;phase='playing';elapsed=0;started=now();draw(0);frame=request(tick);}
 function tick(){
  frame=0;if(!active||disposed)return;
  const time=now(),seconds=(elapsed+time-started)/1000;
  if(seconds>=duration){draw(duration);phase='finished';elapsed=duration*1000;return;}
  draw(seconds);
  frame=request(tick);
 }
 return {
  setActive(value){
   if(disposed||active===value)return;
   if(active&&phase==='playing')elapsed+=now()-started;
   active=value;clear();
   if(active){if(phase==='playing'){started=now();frame=request(tick);}else plan();}
  },
  destroy(){clear();active=false;disposed=true;},
  get state(){return {phase,active,due,elapsed};}
 };
};

(() => {
 const mark=document.querySelector('[data-sidebar-logo]');if(!mark)return;
 const canvas=mark.querySelector('canvas'),poster=mark.querySelector('img');
 const motion=matchMedia('(prefers-reduced-motion: reduce)');
 let renderer,visible=false,requested=false,stopped=false;
 const schedule=window.createNeuralStarSchedule({draw:time=>{
  renderer.draw(time);canvas.hidden=false;poster.hidden=true;
 }});
 async function prepare(){
  requested=true;
  try{
   const texture=await window.loadNeuralStarTexture(canvas.dataset.neuralSource,canvas.dataset.neuralFallback);
   const prepared=window.createNeuralStar(canvas,texture,Math.min(512,Math.max(192,Math.ceil(mark.clientWidth*(window.devicePixelRatio||1)))));
   await prepared.ready;if(stopped)return;renderer=prepared;
   renderer.draw(0);sync();
  }catch{renderer=null;canvas.hidden=true;poster.hidden=false;}
 }
 function sync(){
  const active=visible&&!document.hidden&&!motion.matches&&!stopped;
  if(active&&!requested)prepare();
  schedule.setActive(active&&!!renderer);
  if(active&&renderer&&schedule.state.phase==='finished'){canvas.hidden=false;poster.hidden=true;}
  // A reduced-motion preference also replaces a partially opened emblem with its still image.
  if(motion.matches){canvas.hidden=true;poster.hidden=false;}
 }
 const observer=new IntersectionObserver(entries=>{visible=entries[0].isIntersecting;sync();});
 observer.observe(mark);
 document.addEventListener('visibilitychange',sync);
 motion.addEventListener('change',sync);
 window.addEventListener('pagehide',event=>{schedule.setActive(false);if(!event.persisted){stopped=true;schedule.destroy();observer.disconnect();}});
 window.addEventListener('pageshow',sync);
})();
