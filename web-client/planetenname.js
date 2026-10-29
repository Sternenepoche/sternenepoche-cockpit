'use strict';

// Planetennamen (Kern-Aktion `planet_benennen`): ein Name je eigenem Planeten, überall gleich angezeigt.
// Umbenennen über den Stift-Knopf (`data-planet-umbenennen`) dort, wo der Planet steht; leerer Name = Standardname.
// Namen von Verbündeten kommen aus `view.planetennamen` bzw. dem Galaxie-Wissen (`name`); Fremde sehen keine.
window.Planetenname=(()=>{
 const MAX=24;
 const planeten=()=>(typeof view!=='undefined'&&view?.planeten)||[];
 const eigen=k=>planeten().find(p=>p.koord===k)||null;
 function standard(p){
  if(!p)return '';
  if(p.heimat)return 'Heimatwelt';
  const i=planeten().filter(x=>!x.heimat).findIndex(x=>x.koord===p.koord);
  return i>=0?'Kolonie '+(i+1):'Kolonie';
 }
 // Anzeigename eines eigenen Planeten (Objekt aus view.planeten).
 const name=p=>p?.name||standard(p);
 // Selbst gewählter Name zu einer Koordinate (eigen oder verbündet), sonst ''.
 function zuKoord(k){
  const p=eigen(k);if(p)return p.name||'';
  return (typeof view!=='undefined'&&view?.planetennamen?.[k])||'';
 }
 // „Name [1:2:3]“ für eigene/verbündete benannte Planeten, sonst „[1:2:3]“; eigene ohne Namen mit Standardname.
 function text(k,{standardname=true}={}){
  const p=eigen(k),n=p?(standardname?name(p):p.name||''):zuKoord(k);
  return n?`${n} [${k}]`:`[${k}]`;
 }
 // HTML: Name (falls vorhanden) vor der Koordinate, für Tabellen und Leisten.
 function html(k,{standardname=false}={}){
  const p=eigen(k),n=p?(standardname?name(p):p.name||''):zuKoord(k);
  return `${n?`<span class="pn-name">${esc(n)}</span> `:''}<span class="koord">[${esc(k)}]</span>`;
 }
 // Monde (Kern `mond_umbenennen`, Aktionsabdeckung 25.10.2026: Bots konnten Monde umbenennen, Spieler nicht): Name Pflicht,
 // höchstens 40 Zeichen; derselbe Dialog, Stift auf der Mondseite.
 const MOND_MAX=40,mondEigen=k=>((typeof view!=='undefined'&&view?.monde?.eigene)||[]).find(m=>m.koord===k)||null;
 const mondStift=k=>mondEigen(k)?`<button type="button" class="pn-stift" data-mond-umbenennen="${esc(k)}" aria-label="Mond bei [${esc(k)}] umbenennen"><svg viewBox="0 0 24 24" aria-hidden="true"><path d="M4 20h4L19 9l-4-4L4 16v4ZM14 6l4 4"/></svg></button>`:'';
 const stift=(k,klasse='')=>eigen(k)?`<button type="button" class="pn-stift ${klasse}" data-planet-umbenennen="${esc(k)}" title="Planet umbenennen" aria-label="Planet [${esc(k)}] umbenennen"><svg viewBox="0 0 24 24" aria-hidden="true" focusable="false"><path d="M4 20h4L19 9l-4-4L4 16v4ZM14 6l4 4"/></svg></button>`:'';
 // Gleiche Regeln wie im Kern (`planetenname_pruefen`); der Server bleibt maßgeblich.
 function pruefen(roh){
  const n=String(roh??'').split(/\s+/u).filter(Boolean).join(' ');
  if([...n].length>MAX)return {fehler:`Höchstens ${MAX} Zeichen`};
  const bad=[...n].find(c=>!/[\p{Alphabetic}\p{N} \-'.’]/u.test(c));
  if(bad)return {fehler:`Zeichen „${bad}“ ist nicht erlaubt – Buchstaben (auch Umlaute), Ziffern, Leerzeichen, - ' .`};
  if(n&&!/[\p{Alphabetic}\p{N}]/u.test(n))return {fehler:'Mindestens ein Buchstabe oder eine Ziffer'};
  return {name:n};
 }
 function dialog(){
  let d=document.getElementById('planetenname-dialog');if(d)return d;
  d=document.createElement('dialog');d.id='planetenname-dialog';d.className='pn-dialog';d.setAttribute('aria-labelledby','pn-titel');
  d.innerHTML=`<form method="dialog" id="pn-form" novalidate><div class="settings-title"><h2 id="pn-titel">Planet umbenennen</h2><button type="button" data-pn-zu aria-label="Umbenennen schließen">×</button></div>
   <label class="pn-feld"><span>Name</span><input id="pn-eingabe" name="name" maxlength="${MAX}" autocomplete="off" spellcheck="false" enterkeyhint="done"></label>
   <p class="pn-info"><span id="pn-zaehler"></span><span id="pn-standard"></span></p><p class="pn-fehler" id="pn-fehler" role="alert" hidden></p>
   <div class="pn-knoepfe"><button type="button" class="pn-zurueck" data-pn-standard>Standardname</button><span></span><button type="button" data-pn-zu>Abbrechen</button><button type="submit" class="pn-speichern" id="pn-speichern">Speichern</button></div></form>`;
  document.body.append(d);
  return d;
 }
 function pruefenMond(roh){const n=String(roh??'').trim().replace(/[\u0000-\u001f]/g,'');if(!n)return {fehler:'Ein Mond braucht einen Namen'};if([...n].length>MOND_MAX)return {fehler:`Höchstens ${MOND_MAX} Zeichen`};return {name:n};}
 function aktualisieren(){
  const d=dialog(),mond=d.dataset.art==='mond',r=(mond?pruefenMond:pruefen)(d.querySelector('#pn-eingabe').value),f=d.querySelector('#pn-fehler');
  d.querySelector('#pn-zaehler').textContent=`${[...(r.name??d.querySelector('#pn-eingabe').value.trim())].length} / ${mond?MOND_MAX:MAX}`;
  f.hidden=!r.fehler;f.textContent=r.fehler||'';
  d.querySelector('#pn-speichern').disabled=!!r.fehler;
  return r;
 }
 function oeffnenMond(k){
  const m=mondEigen(k);if(!m)return;
  const d=dialog(),input=d.querySelector('#pn-eingabe');
  d.dataset.koord=k;d.dataset.art='mond';delete d.dataset.busy;
  d.querySelector('#pn-titel').textContent=`Mond bei [${k}] umbenennen`;d.querySelector('#pn-standard').textContent='';
  d.querySelector('[data-pn-standard]').hidden=true;input.maxLength=MOND_MAX;input.value=m.name||'';input.placeholder='Name des Mondes';
  aktualisieren();if(!d.open)d.showModal();input.focus();input.select();
 }
 function oeffnen(k){
  const p=eigen(k);if(!p)return;
  const d=dialog(),input=d.querySelector('#pn-eingabe');
  d.dataset.koord=k;d.dataset.art='planet';delete d.dataset.busy;
  d.querySelector('[data-pn-standard]').hidden=false;input.maxLength=MAX;
  d.querySelector('#pn-titel').textContent=`Planet [${k}] umbenennen`;
  d.querySelector('#pn-standard').textContent=`Leer lassen = Standardname „${standard(p)}“`;
  input.value=p.name||'';input.placeholder=standard(p);
  aktualisieren();
  if(!d.open)d.showModal();
  input.focus();input.select();
 }
 async function speichern(roh){
  const d=dialog(),k=d.dataset.koord,mond=d.dataset.art==='mond',r=(mond?pruefenMond:pruefen)(roh);
  if(r.fehler||d.dataset.busy||!k)return;
  d.dataset.busy='true';d.querySelector('#pn-speichern').disabled=true;
  const f=d.querySelector('#pn-fehler');
  try{
   const result=await command([mond?{typ:'mond_umbenennen',mond:k,name:r.name}:{typ:'planet_benennen',planet:k,name:r.name}]);
   const fehl=result.ergebnisse.find(x=>!x.ok);
   if(fehl){f.hidden=false;f.textContent=fehl.text;}else d.close();
  }catch(err){f.hidden=false;f.textContent=err.message||String(err);}
  finally{delete d.dataset.busy;if(d.open)aktualisieren();}
 }
 document.addEventListener('click',e=>{
  const b=e.target.closest?.('[data-planet-umbenennen]');
  if(b){e.preventDefault();e.stopPropagation();oeffnen(b.dataset.planetUmbenennen);return;}
  const mb=e.target.closest?.('[data-mond-umbenennen]');
  if(mb){e.preventDefault();e.stopPropagation();oeffnenMond(mb.dataset.mondUmbenennen);return;}
  if(e.target.closest?.('[data-pn-zu]'))dialog().close();
  else if(e.target.closest?.('[data-pn-standard]'))speichern('');
 },true);
 document.addEventListener('input',e=>{if(e.target.id==='pn-eingabe')aktualisieren();});
 document.addEventListener('submit',e=>{if(e.target.id!=='pn-form')return;e.preventDefault();speichern(dialog().querySelector('#pn-eingabe').value);});
 return {MAX,standard,name,zuKoord,text,html,stift,mondStift,pruefen,oeffnen,oeffnenMond,eigen};
})();
