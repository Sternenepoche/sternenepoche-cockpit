// Actual readiness drives the screen. No invented percentage or forced animation cycle.
(() => {
  'use strict';
  const screen=document.getElementById('sternen-loader');if(!screen)return;
  const status=screen.querySelector('[data-loader-status]'),pause=screen.querySelector('[data-loader-pause]');
  const image=screen.querySelector('img'),canvas=screen.querySelector('canvas');
  const reduced=matchMedia('(prefers-reduced-motion: reduce)');
  let still=reduced.matches,started=performance.now(),finishTimer,fadeTimer,deadline,finished=false;
  try{still=still||localStorage.getItem('sternenepoche-loading-motion')==='paused';}catch{}
  let restore=[];
  let renderer,requested=false,frame=0,playedAt=0,elapsed=0,lastTick=0;
  function stopMotion(){
    cancelAnimationFrame(frame);frame=0;
    if(playedAt){elapsed+=performance.now()-playedAt;playedAt=0;}
  }
  function tick(now){
    if(now-lastTick>=1000/30){renderer.draw(((elapsed+now-playedAt)/1000)%renderer.duration);lastTick=now;}
    frame=requestAnimationFrame(tick);
  }
  async function prepareMotion(){
    requested=true;
    try{
      const texture=new Image();texture.src=canvas.dataset.neuralSource;
      try{await texture.decode();}catch{texture.src=canvas.dataset.neuralFallback;await texture.decode();}
      const size=Math.min(1280,Math.max(512,Math.ceil(canvas.parentElement.clientWidth*(window.devicePixelRatio||1))));
      renderer=window.createNeuralStar(canvas,texture,size);await renderer.ready;
      renderer.draw(0);updateMotion();
    }catch{ /* The small JPEG remains usable if Canvas or either texture fails. */ }
  }
  function updateMotion(){
    const running=!still&&!document.hidden&&!finished&&!screen.hidden;
    screen.classList.toggle('is-still',still);
    if(running&&renderer){
      canvas.hidden=false;image.hidden=true;
      if(!frame){playedAt=performance.now();frame=requestAnimationFrame(tick);}
    }else stopMotion();
    if(running&&!requested&&window.createNeuralStar)prepareMotion();
    pause.textContent=still?'Animation starten':'Animation pausieren';pause.setAttribute('aria-pressed',String(still));
  }
  pause.addEventListener('click',()=>{still=!still;try{localStorage.setItem('sternenepoche-loading-motion',still?'paused':'running');}catch{}updateMotion();});
  reduced.addEventListener('change',e=>{still=e.matches;updateMotion();});
  document.addEventListener('visibilitychange',updateMotion);
  function release(){restore.forEach(([node,inert])=>{node.inert=inert;node.removeAttribute('aria-busy');});restore=[];}
  function hide(){
    if(finished)return;finished=true;clearTimeout(deadline);clearTimeout(finishTimer);
    stopMotion();
    release();screen.classList.add('is-leaving');
    if(screen.contains(document.activeElement)){const main=document.querySelector('main');if(main){main.tabIndex=-1;main.focus({preventScroll:true});}}
    fadeTimer=setTimeout(()=>{screen.hidden=true;screen.classList.remove('is-leaving');},reduced.matches?0:400);
  }
  function ready(){clearTimeout(finishTimer);finishTimer=setTimeout(hide,0);}
  function start(message){
    clearTimeout(finishTimer);clearTimeout(fadeTimer);clearTimeout(deadline);started=performance.now();finished=false;
    stopMotion();
    screen.hidden=false;screen.classList.remove('is-leaving');if(message)status.textContent=message;
    elapsed=0;playedAt=0;if(renderer)renderer.draw(0);
    if(!restore.length)restore=[...document.querySelectorAll('main,header,footer')].map(node=>{const previous=node.inert;node.inert=true;node.setAttribute('aria-busy','true');return [node,previous];});
    updateMotion();deadline=setTimeout(hide,18000);
  }
  window.SternenLoading={start,ready,status:message=>{status.textContent=message;}};
  screen.querySelector('[data-loader-skip]').addEventListener('click',hide);
  window.addEventListener('pageshow',e=>{if(e.persisted)hide();});
  start();
  if(screen.dataset.kind!=='game'){
    const contentReady=()=>{
      const critical=[...document.querySelectorAll('.hero-art img')].map(img=>img.decode().catch(()=>{}));
      Promise.allSettled([document.fonts?.ready,...critical]).then(ready);
    };
    if(document.readyState==='loading')document.addEventListener('DOMContentLoaded',contentReady,{once:true});else contentReady();
  }
})();
