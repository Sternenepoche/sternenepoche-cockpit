// SVG fallback for local browsers without WebGL. The shared 3D atlas is the default.
'use strict';
window.addEventListener('DOMContentLoaded',()=>{
 if(!window.STERNENEPOCHE?.local)return;
 const host=document.getElementById('galaxy-space');let layer,toggle;
 let motion=!matchMedia('(prefers-reduced-motion: reduce)').matches;
 document.addEventListener('local-galaxy-view',e=>{
  if(!host.classList.contains('space-fallback'))return;
  if(!layer){
   layer=document.createElement('div');layer.id='local-galaxy';host.append(layer);
   host.classList.toggle('local-motion-paused',!motion);
   toggle=document.createElement('button');toggle.type='button';toggle.textContent=motion?'Schiffsverkehr pausieren':'Schiffsverkehr aktivieren';toggle.className='secondary';toggle.onclick=()=>{motion=!motion;host.classList.toggle('local-motion-paused',!motion);host.classList.toggle('local-motion-enabled',motion);toggle.textContent=motion?'Schiffsverkehr pausieren':'Schiffsverkehr fortsetzen';};document.querySelector('.space-toolbar').append(toggle);
  }
  const {stage,sektor,system,items}=e.detail;
  const points=items.map((p,i)=>{
   let x,y;
   if(stage==='galaxy'){const a=i/items.length*Math.PI*2;x=500+Math.cos(a)*280;y=255+Math.sin(a)*155;}
   else if(stage==='sector'){const a=i*2.399963,r=35+Math.sqrt((i+1)/items.length)*370;x=500+Math.cos(a)*r;y=255+Math.sin(a)*r*.53;}
   else{const a=i*2.4,r=55+i*24;x=500+Math.cos(a)*r;y=255+Math.sin(a)*r*.53;}
   return {...p,x,y};
  });
  const esc=window.esc||((v)=>String(v).replaceAll('&','&amp;').replaceAll('<','&lt;').replaceAll('"','&quot;'));
  const stars=Array.from({length:110},(_,i)=>`<circle cx="${(i*613)%1000}" cy="${(i*179)%510}" r="${i%4===0?1.2:.6}" fill="#adc8e6" opacity="${.2+i%5*.1}"/>`).join('');
  layer.innerHTML=`<svg viewBox="0 0 1000 510" role="img" aria-label="${stage==='galaxy'?'Galaxie':stage==='sector'?'Sektor '+sektor:'Sonnensystem '+sektor+':'+system}"><defs><radialGradient id="local-nebula"><stop stop-color="#5a447d" stop-opacity=".45"/><stop offset="1" stop-color="#091124" stop-opacity="0"/></radialGradient><radialGradient id="local-sun"><stop stop-color="#fff6c6"/><stop offset=".5" stop-color="#ffc358"/><stop offset="1" stop-color="#ff981b" stop-opacity="0"/></radialGradient></defs><ellipse cx="500" cy="260" rx="490" ry="250" fill="url(#local-nebula)"/>${stars}${stage==='system'?'<circle cx="500" cy="255" r="55" fill="url(#local-sun)"/>':''}${stage==='system'?points.map(p=>`<ellipse cx="500" cy="255" rx="${55+points.indexOf(p)*24}" ry="${(55+points.indexOf(p)*24)*.53}" fill="none" stroke="#6d7493" stroke-opacity=".24"/>`).join(''):''}${points.map(p=>`<circle cx="${p.x}" cy="${p.y}" r="${stage==='galaxy'?24:stage==='sector'?3:7}" fill="${p.mark?.color||(p.own?GALAXY_RELATION_COLORS.eigen:GALAXY_RELATION_COLORS.unbekannt)}" opacity="${stage==='galaxy'?.5:.9}"/>`).join('')}</svg>`;
  for(const p of points){const b=document.createElement('button');b.type='button';b.className='local-star '+stage+(p.own?' own':'');b.dataset.spaceKind=p.kind;b.dataset.spaceKey=p.key;b.onclick=e=>{e.stopPropagation();window.GalaxyUI.pick(p.kind,p.key);};b.textContent=stage==='galaxy'?p.text:stage==='sector'?String(p.key):String(p.key);b.title=p.text;b.dataset.relation=p.mark?.relation||(p.own?'eigen':'unbekannt');b.style.setProperty('--relation-color',p.mark?.color||(p.own?GALAXY_RELATION_COLORS.eigen:GALAXY_RELATION_COLORS.unbekannt));b.setAttribute('aria-label',p.text+' öffnen');b.style.left=(p.x/10)+'%';b.style.top=(p.y/5.1)+'%';layer.append(b);}
  const ships=document.createElement('div');ships.className='local-ships';ships.setAttribute('aria-hidden','true');
  for(let i=0;i<12;i++){const ship=document.createElement('i');ship.className='local-ship';ship.style.left=((i*37)%90)+'%';ship.style.setProperty('--lane',(8+i*7)+'%');ship.style.setProperty('--duration',(24+i*2)+'s');ship.style.setProperty('--delay',(-((i*13)%31))+'s');ship.innerHTML='<svg viewBox="0 0 80 28"><path d="M2 14 L28 10 L37 2 L52 9 L78 14 L52 19 L37 26 L28 18 Z" fill="#93a8ba" stroke="#d1dfec" stroke-width=".7"/><path d="M6 12 L24 14 L6 16 L0 14 Z" fill="#86deff"/></svg>';ships.append(ship);}layer.append(ships);
  layer.classList.remove('arrive');void layer.offsetWidth;layer.classList.add('arrive');
 });
});
