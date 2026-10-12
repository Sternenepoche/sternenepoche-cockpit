// Shared deterministic renderer: one small texture instead of hundreds of bitmaps.
'use strict';
window.createNeuralStar=function(canvas,source,size=1280){
const ctx=canvas.getContext('2d');
if(!ctx)throw new Error('Canvas unavailable');
canvas.width=canvas.height=size;
const SIZE=canvas.width,DURATION=18,SOURCE_SIZE=1254;
const center=[629,620],scale=.72;
const core=[[629,486],[663,563],[685,583],[761,620],[685,654],[662,680],[629,745],[595,680],[573,655],[491,620],[573,584],[595,562]];
const regions=[[[629,620],[0,196],[0,0],[1280,0],[1280,170]],[[629,620],[1280,170],[1280,1105]],[[629,620],[1280,1105],[1280,1280],[0,1280],[0,1084]],[[629,620],[0,1084],[0,196]]];
const travel=[[0,-150],[210,0],[0,150],[-210,0]],panels=[],counts=[4,6,8,8,6,4,3];
const nodes=counts.map((n,l)=>Array.from({length:n},(_,i)=>({x:400+l*80,y:640+(i-(n-1)/2)*43})));
const contour=[[629,35],[806,482],[682,583],[826,501],[1150,620],[826,752],[681,665],[798,761],[629,1182],[449,762],[579,667],[428,752],[98,620],[426,502],[581,581],[451,482],[629,35]].map(([x,y])=>({x:640+(x-center[0])*scale,y:640+(y-center[1])*scale}));
const segments=contour.slice(1).map((p,i)=>({a:contour[i],b:p,length:Math.hypot(p.x-contour[i].x,p.y-contour[i].y)})),perimeter=segments.reduce((sum,s)=>sum+s.length,0);
const clamp=x=>Math.max(0,Math.min(1,x)),smooth=x=>{x=clamp(x);return x*x*x*(x*(x*6-15)+10);},mix=(a,b,t)=>a+(b-a)*t;
function polygon(c,points){c.moveTo(...points[0]);for(const p of points.slice(1))c.lineTo(...p);c.closePath();}
const ready=source.decode().then(()=>{for(const region of regions){const texture=document.createElement('canvas');texture.width=texture.height=1280;const c=texture.getContext('2d');c.beginPath();polygon(c,region);c.clip();c.drawImage(source,0,0,SOURCE_SIZE,SOURCE_SIZE);c.globalCompositeOperation='destination-out';c.beginPath();polygon(c,core);c.fill();panels.push(texture);}});
function halo(x,y,r,color,alpha){const g=ctx.createRadialGradient(x,y,0,x,y,r);g.addColorStop(0,`rgba(${color},${alpha})`);g.addColorStop(.23,`rgba(${color},${alpha*.32})`);g.addColorStop(1,`rgba(${color},0)`);ctx.fillStyle=g;ctx.fillRect(x-r,y-r,r*2,r*2);}
// A stationary backdrop avoids repainting a full field of stars in GIF exports.
const backdrop=document.createElement('canvas');backdrop.width=backdrop.height=SIZE;
let backdropReady=false;
function background(){
 if(backdropReady){ctx.drawImage(backdrop,0,0,1280,1280);return;}
 ctx.fillStyle='#050b14';ctx.fillRect(0,0,1280,1280);halo(640,640,610,'35,83,112',.16);halo(640,640,360,'39,151,172',.07);
 for(let i=0;i<65;i++){const x=(i*331+41)%1280,y=(i*617+53)%1280;ctx.globalAlpha=.13;ctx.fillStyle=i%7?'#b4cbd5':'#d8ba82';ctx.fillRect(x,y,i%13?1:2,i%13?1:2);}ctx.globalAlpha=1;
 backdrop.getContext('2d').drawImage(canvas,0,0);backdropReady=true;
}
function network(t,opacity){
 if(opacity<=0)return;ctx.save();ctx.globalAlpha=opacity;
 // Seven dense adjacent feed-forward layers, with three complete signal passes.
 const cycle=(t-4)/1.85,stage=(cycle-Math.floor(cycle))*7,transmitting=t>=4&&t<9.55;
 halo(640,640,315,'39,145,170',.10);ctx.lineWidth=.75;ctx.strokeStyle='rgba(103,192,205,.13)';ctx.beginPath();
 for(let l=0;l<6;l++)for(const a of nodes[l])for(const b of nodes[l+1]){ctx.moveTo(a.x,a.y);ctx.lineTo(b.x,b.y);}ctx.stroke();
 for(let l=0;l<6;l++){
  const f=stage-l-.3;if(!transmitting||f<0||f>1)continue;const active=1-Math.abs(f-.5)*1.3;
  ctx.strokeStyle=`rgba(115,237,226,${active*.24})`;ctx.lineWidth=1.1;ctx.beginPath();for(const a of nodes[l])for(const b of nodes[l+1]){ctx.moveTo(a.x,a.y);ctx.lineTo(b.x,b.y);}ctx.stroke();
  for(const a of nodes[l])for(const b of nodes[l+1]){const x=mix(a.x,b.x,f),y=mix(a.y,b.y,f);ctx.fillStyle=l>3?'#a9f3bb':'#b9f8ff';ctx.globalAlpha=opacity*.78;ctx.beginPath();ctx.arc(x,y,1.45,0,Math.PI*2);ctx.fill();}ctx.globalAlpha=opacity;
 }
 for(let l=0;l<7;l++){const a=transmitting?Math.max(0,1-Math.abs(stage-l-.3)*1.5):0;for(const n of nodes[l]){if(a>.05)halo(n.x,n.y,22,'96,232,216',a*.3);ctx.fillStyle='rgba(7,24,34,.8)';ctx.strokeStyle=a>.05?`rgba(161,253,224,${.45+a*.55})`:'rgba(120,208,218,.65)';ctx.lineWidth=1.3+a;ctx.beginPath();ctx.arc(n.x,n.y,6.4+a*1.4,0,Math.PI*2);ctx.fill();ctx.stroke();ctx.fillStyle=a>.05?'#dcfff3':'#6daab9';ctx.beginPath();ctx.arc(n.x,n.y,1.6+a*1.7,0,Math.PI*2);ctx.fill();}}
 ctx.restore();
}
function emblem(t,open){
 const beat=t<1.6?Math.pow(Math.sin(t/1.6*Math.PI*2),2)*Math.sin(t/1.6*Math.PI):0;
 ctx.save();ctx.translate(640,640);ctx.scale(1+.024*beat,1+.024*beat);ctx.translate(-640,-640);ctx.shadowColor='rgba(0,2,6,.65)';ctx.shadowBlur=18;ctx.shadowOffsetY=9;
 if(open<.0001)ctx.drawImage(source,640-center[0]*scale,640-center[1]*scale,SOURCE_SIZE*scale,SOURCE_SIZE*scale);
 else{
  for(let i=0;i<4;i++){ctx.save();ctx.translate(640+travel[i][0]*open,640+travel[i][1]*open);ctx.rotate((i%2?1:-1)*.032*open);ctx.scale(1-(i%2?.065:.025)*open,1-(i%2?.025:.065)*open);ctx.drawImage(panels[i],-center[0]*scale,-center[1]*scale,1280*scale,1280*scale);ctx.restore();}
  const seal=1-smooth(open/.65);ctx.save();ctx.globalAlpha=seal;ctx.translate(640,640);ctx.scale(1-open*.6,1-open*.6);ctx.translate(-center[0]*scale,-center[1]*scale);ctx.beginPath();polygon(ctx,core.map(p=>p.map(v=>v*scale)));ctx.clip();ctx.drawImage(source,0,0,SOURCE_SIZE*scale,SOURCE_SIZE*scale);ctx.restore();
 }
 ctx.restore();halo(640,640,88,'105,234,250',(1-open)*(.15+.18*beat));
}
function pointAt(distance){distance=((distance%perimeter)+perimeter)%perimeter;for(const s of segments){if(distance<=s.length)return {x:mix(s.a.x,s.b.x,distance/s.length),y:mix(s.a.y,s.b.y,distance/s.length)};distance-=s.length;}return contour[0];}
function greenPoint(p,intensity=1){halo(p.x,p.y,30,'76,255,158',.45*intensity);ctx.fillStyle=`rgba(106,255,173,${intensity})`;ctx.beginPath();ctx.arc(p.x,p.y,3,0,Math.PI*2);ctx.fill();ctx.fillStyle=`rgba(229,255,237,${intensity})`;ctx.beginPath();ctx.arc(p.x,p.y,1.35,0,Math.PI*2);ctx.fill();}
function edgeSignal(t){
 if(t<12.9||t>17.4)return;
 if(t<=16.1){const progress=clamp((t-13.05)/3.05),distance=progress*perimeter,alpha=smooth((t-12.9)/.15);
  // The trail follows the metal silhouette, rather than orbiting around it.
  for(let j=30;j>0;j--){const a=pointAt(distance-j*3.2),b=pointAt(distance-(j-1)*3.2);ctx.strokeStyle=`rgba(91,255,163,${(1-j/31)*.8*alpha})`;ctx.lineWidth=1.8;ctx.beginPath();ctx.moveTo(a.x,a.y);ctx.lineTo(b.x,b.y);ctx.stroke();}greenPoint(pointAt(distance),alpha);
 }else{const f=smooth((t-16.1)/1.1),start=contour[0],p={x:640+Math.sin(f*Math.PI*2)*65*Math.sin(f*Math.PI),y:mix(start.y,640,f)};greenPoint(p,1-smooth((f-.84)/.16));halo(640,640,135,'76,255,158',Math.sin(f*Math.PI)*.13);}
}
function draw(t){const open=smooth((t-1.6)/1.9)*(1-smooth((t-10.4)/2.15)),opacity=smooth((t-2.9)/1)*(1-smooth((t-10.35)/1.3));ctx.setTransform(SIZE/1280,0,0,SIZE/1280,0,0);ctx.globalAlpha=1;ctx.globalCompositeOperation='source-over';background(t,open);network(t,opacity);emblem(t,open);edgeSignal(t);}
return {ready,draw,duration:DURATION,layers:counts};
};

// One complete visible cycle, then a quiet interval measured from its end.
// Dependencies are injectable so the 60-minute boundary can be tested without waiting an hour.
window.createNeuralStarSchedule=function({draw,duration,now=()=>performance.now(),random=Math.random,
  later=setTimeout,cancel=clearTimeout,request=requestAnimationFrame,cancelFrame=cancelAnimationFrame}){
 const delay=(min,max)=>min+Math.min(1,Math.max(0,random()))*(max-min);
 let due=Math.max(now(),delay(0,30000)),phase='waiting',active=false,elapsed=0,started=0,timer=0,frame=0,lastDraw=-Infinity,disposed=false;
 function clear(){cancel(timer);cancelFrame(frame);timer=frame=0;}
 function plan(){if(active&&!disposed&&phase==='waiting')timer=later(begin,Math.max(0,due-now()));}
 function begin(){timer=0;if(!active||disposed)return;phase='playing';elapsed=0;started=now();lastDraw=-Infinity;draw(0);frame=request(tick);}
 function tick(){
  frame=0;if(!active||disposed)return;
  const time=now(),seconds=(elapsed+time-started)/1000;
  if(seconds>=duration){draw(duration);phase='waiting';elapsed=0;due=time+delay(300000,3600000);plan();return;}
  if(time-lastDraw>=1000/30){draw(seconds);lastDraw=time;}
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
 const schedule=window.createNeuralStarSchedule({duration:18,draw:time=>{
  renderer.draw(time);canvas.hidden=false;poster.hidden=true;
 }});
 async function prepare(){
  requested=true;
  try{
   const texture=new Image();texture.src=canvas.dataset.neuralSource;
   try{await texture.decode();}catch{texture.src=canvas.dataset.neuralFallback;await texture.decode();}
   const prepared=window.createNeuralStar(canvas,texture,Math.min(512,Math.max(192,Math.ceil(mark.clientWidth*(window.devicePixelRatio||1)))));
   await prepared.ready;if(stopped)return;renderer=prepared;
   renderer.draw(0);sync();
  }catch{renderer=null;canvas.hidden=true;poster.hidden=false;}
 }
 function sync(){
  const active=visible&&!document.hidden&&!motion.matches&&!stopped;
  if(active&&!requested)prepare();
  schedule.setActive(active&&!!renderer);
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
