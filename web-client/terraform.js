// Terraforming eines eigenen Planeten (Karl 24.10.2026): Terraformer-Schiffe ziehen das Klima zum Optimum des eigenen Volkes
// und vergrößern den Planeten um bis zu 40 Felder (lebensfeindliche Bahnen weniger); je Forschungsstufe braucht es weniger
// Schiffe. Die Karte steht auf der Seite „Kolonisation“ unter der Verwaltung des gewählten Planeten: Stand (Klima, Felder),
// noch nötige Schiffe jetzt und mit den nächsten Forschungsstufen, ein Knopf zum Einsatz und einer zur Werft.
// Daten: view.planeten[].terraform (Kern, sicht.rs); weitere Stufen über das Werkzeug terraformbedarf (eine Abfrage je Planet).
(function(){
 'use strict';
 const el=id=>document.getElementById(id);
 const h=s=>typeof esc==='function'?esc(s):String(s??'');
 const zahl=n=>typeof fmt==='function'?fmt(Math.round(Number(n)||0)):String(Math.round(Number(n)||0));
 const name=k=>typeof named==='function'?named(k):k;
 const V=()=>typeof view!=='undefined'?view:null;
 let cache={},laeuft=false;
 function planet(){const v=V();if(!v)return null;const k=typeof selectedPlanet!=='undefined'?selectedPlanet:'';return (v.planeten||[]).find(p=>p.koord===k)||v.planeten?.[0]||null;}
 function host(){
  const s=el('kolonien');if(!s)return null;let d=el('terraform-karte');
  // Hinter den Inhalt des Bereichs, nicht hinein: setStableHTML(#kolonien-content) würde fremde Kinder entfernen.
  if(!d){d=document.createElement('div');d.id='terraform-karte';d.className='tf-karte';s.append(d);d.addEventListener('click',klick);}
  return d;
 }
 const stufe=()=>Number(planet()?.terraform?.stufe??V()?.forschung?.terraforming)||0;
 function balken(wert,max,cls){const a=max>0?Math.max(0,Math.min(1,wert/max)):0;return `<span class="tf-balken ${cls}"><i data-anteil="${a}"></i></span>`;}
 async function vorschau(k){
  if(cache[k]?.zeit>Date.now()-60000||laeuft)return;laeuft=true;
  try{cache[k]={zeit:Date.now(),daten:await api('/api/tool',{typ:'terraformbedarf',koord:k})};}catch(err){cache[k]={zeit:Date.now(),fehler:err.message};}
  finally{laeuft=false;}zeichnen();
 }
 function zeichnen(){
  const d=host(),p=planet();if(!d)return;
  const t=p?.terraform;
  if(!p||!t){if(d.innerHTML)d.innerHTML='';d.hidden=true;return;}
  d.hidden=false;
  const klima=Number(t.klima_prozent)||0,felder=Number(t.felder)||0,grenze=Number(t.feldgrenze)||0,sk=Number(t.schiffe_klima)||0,sf=Number(t.schiffe_felder)||0,voll=Number(t.schiffe_voll??sk+sf)||0;
  const da=Number(p.schiffe?.terraformer)||0,fertig=voll<=0,anderswo=da?null:(V()?.planeten||[]).find(x=>(Number(x.schiffe?.terraformer)||0)>0)||null;
  const v=cache[p.koord]?.daten,stufen=Object.entries(v?.naechste_stufen||{}).filter(([s])=>Number(s)>stufe()).slice(0,3);
  const html=`<div class="tf-kopf"><div><span class="tf-eyebrow">Terraforming</span><h3>${h(window.Planetenname?Planetenname.text(p.koord):'['+p.koord+']')}</h3></div><span class="tf-stufe">Forschung ${zahl(stufe())}</span></div>
   <div class="tf-zeile"><span>Klima für ${h(name(V()?.volk))}</span>${balken(klima,100,'klima')}<b>${zahl(klima)} %</b></div>
   <div class="tf-zeile"><span>Zusatzfelder</span>${balken(felder,Math.max(grenze,1),'felder')}<b>+${zahl(felder)} / ${zahl(grenze)}</b></div>
   ${fertig?'<p class="tf-fertig">Fertig geformt: Klima am Optimum, alle Zusatzfelder erschlossen.</p>':`<div class="tf-zahlen"><div><b>${zahl(sk)}</b><span>Terraformer fürs Klima</span></div><div><b>${zahl(sf)}</b><span>für die Felder</span></div><div class="tf-voll"><b>${zahl(voll)}</b><span>komplett · ${zahl(da)} am Planeten</span></div></div>
   ${stufen.length?`<p class="tf-leise">Mit mehr Forschung: ${stufen.map(([s,n])=>`Stufe ${h(s)} → ${zahl(n)} Schiffe`).join(' · ')}</p>`:''}
   <div class="tf-knoepfe"><button type="button" class="tf-los" data-tf-einsatz="${h(p.koord)}"${da||anderswo?'':' aria-disabled="true"'}>Terraformer schicken</button><button type="button" class="secondary" data-go="werft">Terraformer bauen</button></div>
   ${da?'':anderswo?`<p class="tf-leise">Hier liegen keine Terraformer – sie starten von ${h(anderswo.koord)} (im Einsatzblatt wählbar).</p>`:(()=>{const fl=(V()?.flotten||[]).filter(f=>(Number(f.schiffe?.terraformer)||0)>0),n=fl.reduce((s,f)=>s+(Number(f.schiffe.terraformer)||0),0);return n?`<p class="tf-leise">${zahl(n)} Terraformer unterwegs (${fl.map(f=>'['+h(f.ziel)+']').join(', ')}) – hier keine weiteren.</p>`:'<p class="tf-leise">Noch keine Terraformer: ab Zivilisationsstufe IV mit Terraforming 1 in der Werft bauen.</p>';})()}`}
   <p class="tf-leise">Zuerst wird das Klima angepasst, dann die Kruste erweitert. Eingesetzte Terraformer werden verbraucht, überzählige fliegen heim.</p>`;
  if(typeof setStableHTML==='function')setStableHTML(d,html);else d.innerHTML=html;
  d.querySelectorAll('[data-anteil]').forEach(i=>{const w=(Number(i.dataset.anteil)*100).toFixed(1)+'%';if(i.style.width!==w)i.style.width=w;});
  if(!fertig)vorschau(p.koord);
 }
 function klick(e){
  const b=e.target.closest('[data-tf-einsatz]');if(!b||b.getAttribute('aria-disabled')==='true')return;
  if(window.Einsatz)Einsatz.oeffnen({ziel:b.dataset.tfEinsatz,mission:'terraformen'});
 }
 let letzte='';
 setInterval(()=>{if(document.hidden||el('kolonien')?.hidden)return;const p=planet(),s=JSON.stringify([V()?.revision,p?.koord,p?.terraform]);if(s!==letzte){letzte=s;zeichnen();}},1200);
 document.addEventListener('game-navigation',()=>{letzte='';});
 window.Terraform={zeichnen};
})();
