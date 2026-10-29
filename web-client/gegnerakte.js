'use strict';
// Gegnerakte und Sofortmeldung (Karl 22.10.2026): „nicht nur wer mich angegriffen hat, sondern der genaue Planet mit
// Koordinaten … und von dem Angreifer der Heimatplanet und ein kleines Profil mit allen gesammelten Informationen
// (mit Notizen), das sich automatisch füllt … und man auch die Infos vom letzten Scan sieht“.
// Seit 24.10.2026 ist die Akte das Spielerprofil im Funk-Blatt (spielerprofil.js, Karl: „dass man den Spieler anklicken
// will, man dann sein Profil mit Heimatplanet sieht …“). Hier bleiben: das Sammeln der Planeten eines Reichs aus der
// Sicht (Sternkarte, Spionage-, Kampf- und Flottenberichte, Angriffe) und die Sofortleiste für neue Berichte mit direkter
// Reaktion (Spionage → Angriff planen / erneut ausspähen, Kampf → Trümmer bergen / Herkunft ausspähen).
// Notizen liegen je Konto und Welt auf dem Server (/api/akte/notiz, view.gegner_notizen).
window.Gegnerakte=(()=>{
 const summe=o=>Object.values(o||{}).reduce((n,v)=>n+(Number(v)||0),0);
 const koordWert=k=>String(k).split(':').map(Number).reduce((n,v)=>n*100+(v||0),0);

 // Alle bekannten Planeten eines Reichs – nur aus der eigenen Sicht.
 function sammeln(name){
  const planeten=new Map(),merk=(k,info)=>{if(!k)return;const p=planeten.get(k)||{koord:k};for(const[a,b]of Object.entries(info))if(b!=null&&b!==false)p[a]=b;planeten.set(k,p);};
  for(const a of view?.atlas||[])if(a.spieler===name)merk(a.koord,{zone:a.zone,mond:a.mond});
  for(const n of view?.nachbarn||[])if(n.spieler===name)merk(n.koord,{heimat:n.heimat===true,zone:n.zone});
  for(const b of view?.berichte||[])if(b.besitzer===name){const alt=planeten.get(b.ziel)?.bericht;merk(b.ziel,{bericht:!alt||b.zeit>alt.zeit?b:alt,heimat:b.aufklaerung?.heimat===true});}
  const kaempfe=(view?.kampfberichte||[]).filter(k=>(k.teilnehmer||[]).includes(name));
  for(const k of kaempfe)for(const x of k.herkunft||[])if(x.spieler===name){merk(x.start,{angriffVon:true});if(x.heimat)merk(x.heimat,{heimat:true});}
  const angriffe=(view?.angriffe||[]).filter(f=>f.von===name);
  for(const f of angriffe)merk(f.start,{angriffVon:true});
  const kontakte=(view?.flottenberichte||[]).filter(f=>f.von===name);
  const liste=[...planeten.values()].sort((a,b)=>(b.heimat?1:0)-(a.heimat?1:0)||koordWert(a.koord)-koordWert(b.koord));
  const rang=(view?.rangliste||world?.rangliste||[]).find(r=>r[1]===name);
  const beziehung=typeof relationFor==='function'?relationFor(name,null):null;
  return {name,planeten:liste,heimat:liste.find(p=>p.heimat)?.koord||'',kaempfe,angriffe,kontakte,rang,beziehung,notiz:view?.gegner_notizen?.[name]||''};
 }
 // Ältere Aufrufe (Galaxietabelle, Kampfbericht „Akte“) öffnen das Profil.
 function oeffnen(name){if(!view||!name)return;$('report-reader')?.open&&$('report-reader').close();$('galaxy-infocard')?.close?.();window.Spielerprofil?.oeffnen(name);}

 // ---------------------------------------------------------------- Sofortmeldung neuer Berichte
 let gesehen=null,leisteTimer=0;
 const h=s=>esc(s);
 function reaktion(x){
  const e=window.Funk?.finde('b:'+x.id);if(!e)return {e:null,knoepfe:''};
  const k=[],F=window.Funk,fremd=e.koord&&!(view.planeten||[]).some(p=>p.koord===e.koord);
  if(e.kind==='kampfberichte'){
   if(e.feind&&e.herkunft?.[0])k.push(F.spion(e.herkunft[0],'Herkunft ausspähen'));
   if(e.truemmer>0)k.push(F.plan(e.koord,'recyceln','Trümmer bergen'));
  }else if((e.kind==='berichte'&&e.row?.art!=='erkundung'||e.kind==='spionageberichte')&&fremd){k.push(F.plan(e.koord,'angriff','Angriff planen'));k.push(F.spion(e.koord,'Erneut'));}
  else if(e.art==='spionage'&&e.feind&&e.herkunft?.[0])k.push(F.spion(e.herkunft[0],'Herkunft ausspähen'));
  return {e,knoepfe:k.slice(0,2).join('')};
 }
 function ergebnis(x){
  const e=window.Funk?.finde('b:'+x.id);
  if(!e)return null;
  return {klasse:e.ton==='gut'?'ga-gut':e.ton==='schlecht'||e.ton==='warn'?'ga-schlecht':'',titel:e.titel,text:[e.unter,...(e.kern||[])].filter(Boolean).join(' · '),gegner:(e.spieler||[])[0]};
 }
 function leiste(){let l=document.getElementById('bericht-sofort');if(!l){l=document.createElement('div');l.id='bericht-sofort';l.className='ga-sofort';l.setAttribute('role','status');l.hidden=true;document.body.append(l);}return l;}
 function neueBerichte(rows,read){
  const wichtig=rows.filter(x=>['kampfberichte','berichte','flottenberichte','bodenberichte','spionageberichte','phalanxberichte','ereignisse'].includes(x.kind)&&(x.kind!=='ereignisse'||/Spionage durch .+ bemerkt/.test(x.row?.text||'')));
  if(!gesehen){gesehen=new Set(wichtig.map(x=>x.id));return;}  // beim Laden nichts nachmelden
  const neu=wichtig.filter(x=>!gesehen.has(x.id)&&!read.has(x.id));wichtig.forEach(x=>gesehen.add(x.id));
  if(!neu.length||document.body.classList.contains('bz-blatt-offen'))return;
  const x=neu.find(x=>x.kind==='kampfberichte'&&x.row?.seite==='verteidigung')||neu.find(x=>x.kind==='kampfberichte')||neu[0],e=ergebnis(x);if(!e)return;
  const {knoepfe}=reaktion(x);
  const l=leiste();l.className='ga-sofort '+e.klasse;
  l.innerHTML=`<div class="ga-sofort-text"><b>${h(e.titel)}</b><span>${h(e.text)}</span>${neu.length>1?`<button type="button" class="ga-weitere" data-bz-sofort-alle data-sofort-zu>+${neu.length-1} weitere neue Berichte</button>`:''}</div><div class="ga-knoepfe">${knoepfe}<button type="button" data-bz-oeffnen="b:${h(x.id)}" data-sofort-zu>Bericht</button>${e.gegner&&e.gegner!==view.name?`<button type="button" data-sp-profil="${h(e.gegner)}" data-sofort-zu>Profil</button>`:''}<button type="button" class="ga-zu" data-sofort-zu aria-label="Meldung schließen">×</button></div>`;
  l.hidden=false;clearTimeout(leisteTimer);leisteTimer=setTimeout(()=>{l.hidden=true;},20000);
 }

 document.addEventListener('click',e=>{
  const b=e.target.closest('[data-akte],[data-sofort-zu],[data-bz-sofort-alle]');if(!b)return;
  if(b.hasAttribute('data-sofort-zu')){const l=document.getElementById('bericht-sofort');if(l)l.hidden=true;}
  if(b.hasAttribute('data-bz-sofort-alle')){window.Funk?.zeigeZentrale({ungelesen:true});return;}
  if(b.dataset.akte)oeffnen(b.dataset.akte);
 });
 return {oeffnen,sammeln,neueBerichte};
})();
