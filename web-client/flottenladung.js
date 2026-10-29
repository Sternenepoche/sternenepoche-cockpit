'use strict';
// Gemeinsame Zuladung und bilaterale Schiffsangebote im bestehenden Flottenassistenten.
window.Flottenladung=(()=>{
 let personenWahl=Array.from({length:4},()=>[0,0,0]),quelle='',kap=null,folge=0,stamp='',senden=false,quoteTimer=0;
 const voelker=['aurelianer','krath','veyari','syntheten'],f=()=>$('fleet')?.elements;
 const transfer=()=>['uebergeben','schiffhandel'].includes(f()?.mission.value);
 function start(){const e=f(),p=view?.planeten?.find(p=>p.koord===e?.start.value);return e?.startart?.value==='mond'?(view.monde?.eigene||[]).find(m=>m.koord===p?.koord):p;}
 function personen(){return {gruppen:personenWahl.map(g=>g.map(n=>n*1000))};}
 function aktion(a,t){const p=personen();return p.gruppen.flat().some(n=>n)||t.some(n=>n)?{typ:'flotte_beladen',flug:a,personen:p,truppen:t}:a;}
 function mark(){stamp='';Mobile035.schedulePreview();Flottenfuehrung.neu();}
 function bauen(){
  const form=$('fleet'),step=form?.querySelector('[data-schritt="2"]');if(!step||$('fl-zuladung'))return;
  for(const [k,t] of [['uebergeben','Schiffe verschenken'],['schiffhandel','Schiffe verkaufen']])if(![...f().mission.options].some(o=>o.value===k)){const o=document.createElement('option');o.value=k;o.textContent=t;f().mission.append(o);}
  const host=document.createElement('section');host.id='fl-zuladung';host.className='fl-zuladung';
  const source=document.createElement('div');source.id='fl-startanzeige';source.className='fl-startanzeige';step.prepend(source);
  host.innerHTML='<h3>Zuladung</h3><div id="fl-kapazitaet" aria-live="polite"></div><nav class="fl-lade-reiter" aria-label="Zuladung wählen"><button type="button" data-fl-panel="fracht" aria-pressed="true">Rohstoffe <b id="fl-fracht-zahl">0</b></button><button type="button" data-fl-panel="personen" aria-pressed="false">Personen <b id="fl-personen-zahl">0</b></button><button type="button" data-fl-panel="boden" aria-pressed="false">Truppen <b id="fl-boden-zahl">0</b></button></nav><div data-fl-inhalt="fracht"><p class="fl-hinweis">Alle verfügbaren Güter am Startort. Treibstoff wird zusätzlich reserviert.</p></div><div data-fl-inhalt="personen" hidden><div id="fl-personen"></div></div><div data-fl-inhalt="boden" hidden></div>';
  const cargo=$('cargo-inputs').closest('details')||$('cargo-inputs').closest('fieldset');if(cargo){cargo.open=true;host.querySelector('[data-fl-inhalt="fracht"]').append(cargo);cargo.querySelector('summary')?.remove();cargo.querySelector('legend')?.remove();cargo.querySelector('fieldset')?.setAttribute('aria-label','Rohstoffe verladen');const save=cargo.querySelector('#save-cargo');if(save)save.textContent='Frachtraum automatisch füllen';}
  const ground=$('ground-load');if(ground){ground.open=true;host.querySelector('[data-fl-inhalt="boden"]').append(ground);ground.querySelector('summary').textContent='Garnison auswählen';}
  step.append(host);
  const handel=document.createElement('div');handel.id='fl-angebot';handel.className='fl-angebot';handel.hidden=true;handel.innerHTML='<h3>Verbindliche Schiffsübergabe</h3><p>Der Empfänger nimmt das Angebot im Flottenkommando an. Dann fliegen die gewählten Schiffe mit der gesamten Zuladung los und gehen bei Zustellung in seinen Besitz über.</p><label id="fl-preis-label">Kaufpreis · Credits<input id="fl-preis" type="number" inputmode="decimal" min="0" max="1000000000" step="1" value="0"></label><p>Gilt auch ohne Bündnis. Credits werden bei Annahme hinterlegt und erst bei Zustellung ausgezahlt. Bei Rückruf, Verlust oder Besitzerwechsel am Ziel erhält der Käufer seine Credits zurück.</p>';form.querySelector('[data-schritt="3"]').prepend(handel);
  const q=$('flight-quote');if(q){const details=document.createElement('details');details.className='fl-flugdetails';details.innerHTML='<summary>Termine & Treibstoff im Detail</summary>';q.before(details);details.append(q);const row=$('flight-preview')?.closest('.row');if(row)details.append(row);}
 }
 async function quote(){
  const nr=++folge;try{const a=fleetData();if(!Object.keys(a.schiffe).length){kap=null;zeichnen();return;}const q=await api('/api/tool',{...a,typ:'flugzeit'});if(nr!==folge)return;kap=q;zeichnen();}catch{if(nr===folge){kap=null;zeichnen();}}
 }
 function zeichnen(){
  if(!view||!f())return;bauen();if(!$('fl-zuladung'))return;
  const key=f().start.value+':'+(f().startart?.value||'planet');if(key!==quelle){quelle=key;personenWahl=voelker.map(()=>[0,0,0]);kap=null;}
  const p=start();if(!p)return;const moon=f().startart?.value==='mond'||f().zielart?.value==='mond',m=f().mission.value,erlaubt=!moon&&['transport','stationieren','uebergeben','schiffhandel'].includes(m);
  setStableHTML($('fl-startanzeige'),`<span><small>STARTORT</small><b>${f().startart?.value==='mond'?'☾ '+esc(p.name||'Mond'):esc(Planetenname.name(p))} [${esc(f().start.value)}]</b></span><button type="button" class="secondary" data-ff-schritt="1">Ändern</button>`);
  if(!erlaubt)personenWahl=voelker.map(()=>[0,0,0]);
  const stock=p.personengruppen?.gruppen||voelker.map(()=>[0,0,0]);const sig=JSON.stringify([quelle,stock.map(g=>g.map(n=>Math.floor(n/1000))),erlaubt]);
  if(sig!==$('fl-personen').dataset.stamp&&!$('fl-personen').contains(document.activeElement)){
   $('fl-personen').dataset.stamp=sig;
   setStableHTML($('fl-personen'),erlaubt?`<p class="fl-hinweis">Eigene Personenplätze, getrennt von Fracht und Truppen. Mindestens 500 Einwohner bleiben am Start.</p>${voelker.map((v,i)=>stock[i]?.some(n=>n>=1000)?`<h4>${esc(named(v))}</h4>${stock[i].map((n,j)=>n>=1000?`<label class="fl-person"><span>${['Bewohner','Fachkräfte','Forscher'][j]}<small>${fmt(Math.floor(n/1000))} vorhanden · <span data-fl-rest="${i}:${j}"></span> bleiben</small></span><input type="number" inputmode="numeric" min="0" max="${Math.floor(n/1000)}" value="${personenWahl[i][j]}" data-fl-person="${i}:${j}" aria-label="${esc(named(v))} ${['Bewohner','Fachkräfte','Forscher'][j]} mitnehmen"><button type="button" data-fl-person-max="${i}:${j}">Max</button></label>`:'').join('')}`:'').join('')}`:'<p class="fl-hinweis">Personen reisen mit Transport oder Stationieren zwischen Planeten. Monde besitzen keine zivile Bevölkerung.</p>');
  }
  const inputs=[...$('cargo-inputs').querySelectorAll('input[data-map]')];let fracht=0;
  inputs.forEach(i=>{const k=i.dataset.map,n=Number(p.bestand?.[k])||0,row=i.closest('label');if(!row)return;row.hidden=n<=0&&!(Number(i.value)>0);i.max=String(Math.floor(n));fracht+=Number(i.value)||0;
   let note=row.querySelector('.fl-stock');if(!note){for(const n of [...row.childNodes])if(n.nodeType===3)n.remove();const title=document.createElement('span');title.className='fl-guetername';title.textContent=named(k);row.prepend(title);row.insertAdjacentHTML('afterbegin',imageTag('resources',k));i.setAttribute('aria-label',named(k)+' mitnehmen');note=document.createElement('small');note.className='fl-stock';row.append(note);const b=document.createElement('button');b.type='button';b.dataset.flCargoMax=k;b.textContent='Max';row.append(b);row.classList.add('fl-cargo');}
   note.textContent=`${fmt(n)} vorhanden → ${fmt(n-(Number(i.value)||0))} bleiben`;
  });
  const people=personenWahl.flat().reduce((a,b)=>a+b,0),troops=Mobile035.groundDraft();
  const ground=$('ground-load');if(ground){const hint=ground.querySelector('p');if(hint)hint.textContent='Truppen haben eigene Plätze in Truppentransportern. Transport liefert sie an den Empfänger, Stationieren verlegt die eigene Garnison. Halten unterstützt Verbündete; Kampfkolonisieren führt sie bei der Invasion mit.';ground.querySelectorAll('[data-ground-available]').forEach(x=>{const i=Number(x.dataset.groundAvailable),n=p.bodentruppen?.bestand?.[i]||0;x.textContent=fmt(n)+' vorhanden → '+fmt(n-(troops[i]||0))+' bleiben';});}
  $('fl-fracht-zahl').textContent=fmt(fracht);$('fl-personen-zahl').textContent=fmt(people);$('fl-boden-zahl').textContent=fmt(troops.reduce((a,b)=>a+b,0));
  document.querySelectorAll('[data-fl-rest]').forEach(x=>{const[i,j]=x.dataset.flRest.split(':').map(Number);x.textContent=fmt(Math.floor((stock[i]?.[j]||0)/1000)-personenWahl[i][j]);});
  const cap=Number(kap?.ladekapazitaet)||0,seats=Number(kap?.personenplaetze)||0;
  setStableHTML($('fl-kapazitaet'),`<div class="fl-meter ${fracht>cap?'fl-ueberladen':''}"><span>Frachtraum <b>${fmt(fracht)} / ${fmt(cap)}</b></span><meter min="0" max="${cap||1}" value="${fracht}"></meter></div><div class="fl-meter ${people>seats?'fl-ueberladen':''}"><span>Personenplätze <b>${fmt(people)} / ${fmt(seats)}</b></span><meter min="0" max="${seats||1}" value="${people}"></meter></div>`);
  $('fl-angebot').hidden=!transfer();$('fl-preis-label').hidden=m!=='schiffhandel';if(m==='uebergeben')$('fl-preis').value='0';
  if(f().siedler_volk)f().siedler_volk.closest('label').hidden=m!=='kolonisieren';
  const fuel=$('return-fuel');if(fuel){fuel.checked=true;fuel.closest('label').hidden=true;}
  if(f().termin_modus){if(transfer())f().termin_modus.value='now';f().termin_modus.closest('label').hidden=transfer();f().ankunft_tag?.closest('.row')?.toggleAttribute('hidden',f().termin_modus.value!=='arrival');}
  if(f().rueckgeschwindigkeit)f().rueckgeschwindigkeit.closest('label').hidden=['stationieren','kolonisieren'].includes(m);
  if(f().haltedauer_stunden)f().haltedauer_stunden.closest('label').hidden=!['halten','verstaerken','saven','abbau','blockade','bombardieren','kampfkolonisieren'].includes(m);
  if(moon){$('ground-load')?.querySelectorAll('[data-ground-load]').forEach(i=>i.value='0');if($('ground-load'))$('ground-load').hidden=true;}else if($('ground-load'))$('ground-load').hidden=false;
  angebote();
 }
 // Frist des Angebots (Kern ablauf = erstellt + 1 Spieltag) – sie fehlte auf der Karte (Funktionsprüfung 25.10.2026).
 const tagUhr=s=>'Tag '+(Math.floor(s/86400)+1)+' · '+[Math.floor(s/3600)%24,Math.floor(s/60)%60].map(n=>String(n).padStart(2,'0')).join(':');
 function angebote(){
  let host=$('fl-schiffangebote');if(!host){host=document.createElement('section');host.id='fl-schiffangebote';$('fleet-planner').before(host);}
  const rows=view.schiffangebote||[];
  setStableHTML(host,rows.length?`<h3>Schiffsangebote · ${rows.filter(a=>a.status==='offen').length} offen</h3>${rows.slice(0,20).map(a=>`<article class="fl-handelkarte"><header><b>${a.preis?fmt(a.preis)+' Credits':'Geschenk'}</b><span>${esc({offen:'Wartet auf Annahme',unterwegs:'Unterwegs',zugestellt:'Zugestellt',abgelehnt:'Beendet',gescheitert:'Rückabwicklung',abgelaufen:'Abgelaufen'}[a.status]||a.status)}${a.status==='offen'&&a.ablauf!=null?` · annehmen bis ${esc(tagUhr(a.ablauf))}`:''}</span></header><p>${esc(a.von)} → ${esc(a.an)} · ${a.flug.zielart==='mond'?'☾ ':''}[${esc(a.flug.ziel)}]</p><p>${esc(goods(a.flug.schiffe))}</p><small>Fracht: ${esc(goods(a.flug.ladung)||'keine')} · ${fmt(a.personen.gruppen.flat().reduce((s,n)=>s+n,0)/1000)} Personen · ${fmt(a.truppen.reduce((s,n)=>s+n,0))} Bodentruppen</small>${a.status==='offen'?`<div class="fl-handelaktionen">${a.eingang?`<button type="button" data-fl-angebot="${a.id}" data-fl-annahme="true">${a.preis?'Kaufen & Lieferung starten':'Geschenk annehmen'}</button>`:''}<button type="button" class="secondary" data-fl-angebot="${a.id}" data-fl-annahme="false">${a.eingang?'Ablehnen':'Angebot zurücknehmen'}</button></div>`:''}</article>`).join('')}`:'');
 }
 document.addEventListener('click',guard(async e=>{
  const b=e.target.closest('[data-fl-panel],[data-fl-cargo-max],[data-fl-person-max],[data-fl-angebot]');if(!b)return;
  if(b.dataset.flPanel){document.querySelectorAll('[data-fl-inhalt]').forEach(x=>x.hidden=x.dataset.flInhalt!==b.dataset.flPanel);document.querySelectorAll('[data-fl-panel]').forEach(x=>x.setAttribute('aria-pressed',String(x===b)));return;}
  if(b.dataset.flCargoMax){const i=$('cargo-inputs').querySelector(`[data-map="${b.dataset.flCargoMax}"]`),used=[...$('cargo-inputs').querySelectorAll('input')].filter(x=>x!==i).reduce((s,x)=>s+(Number(x.value)||0),0);i.value=Math.floor(Math.max(0,Math.min(Number(i.max)||0,(Number(kap?.ladekapazitaet)||0)-used)));i.dispatchEvent(new Event('input',{bubbles:true}));return;}
  if(b.dataset.flPersonMax){const[i,j]=b.dataset.flPersonMax.split(':').map(Number),stock=start()?.personengruppen?.gruppen||[],other=personenWahl.flat().reduce((s,n)=>s+n,0)-personenWahl[i][j],max=Math.floor((stock[i]?.[j]||0)/1000),left=Math.floor(stock.flat().reduce((s,n)=>s+n,0)/1000)-500-other;personenWahl[i][j]=Math.max(0,Math.min(max,(Number(kap?.personenplaetze)||0)-other,left));const input=document.querySelector(`[data-fl-person="${i}:${j}"]`);input.value=personenWahl[i][j];input.dispatchEvent(new Event('input',{bubbles:true}));return;}
  if(b.dataset.flAngebot){await command([{typ:'schiffangebot_antwort',angebot:Number(b.dataset.flAngebot),annehmen:b.dataset.flAnnahme==='true'}]);zeichnen();}
 }));
 document.addEventListener('input',e=>{if(e.target.dataset.flPerson){const[i,j]=e.target.dataset.flPerson.split(':').map(Number);personenWahl[i][j]=Math.max(0,Math.floor(Number(e.target.value)||0));}if(e.target.closest('#fleet')){zeichnen();if(e.target.closest('#ship-inputs')||e.target.name==='ziel'){clearTimeout(quoteTimer);quoteTimer=setTimeout(quote,250);}}});
 document.addEventListener('change',e=>{if(e.target.closest('#fleet')){zeichnen();quote();}});
 document.addEventListener('stern:flotte-gestartet',()=>{personenWahl=voelker.map(()=>[0,0,0]);stamp='';});
 document.addEventListener('game-navigation',()=>{zeichnen();quote();});
 document.addEventListener('DOMContentLoaded',()=>{const original=fleetData;fleetData=function(){const a=original.apply(this,arguments);if(transfer()){a.mission='transport';a.rueckgeschwindigkeit=Number(f().rueckgeschwindigkeit?.value)||1;}return a;};});
 document.addEventListener('submit',e=>{if(e.target.id!=='fleet'||!transfer())return;e.preventDefault();e.stopImmediatePropagation();if(senden)return;senden=true;
  void(async()=>{try{const r=await command([{typ:'schiffangebot',flug:fleetData(),preis:Number($('fl-preis').value)||0,personen:personen(),truppen:Mobile035.groundDraft()}]);if(r.ergebnisse.every(x=>x.ok)){document.dispatchEvent(new Event('stern:angebot-gesendet'));show('Schiffsangebot gesendet. Der Empfänger kann es im Flottenkommando annehmen.');$('fl-schiffangebote')?.scrollIntoView({block:'center'});}}catch(err){show(err.message,true);}finally{senden=false;zeichnen();}})();
 },true);
 setInterval(()=>{if(typeof view!=='undefined'&&view){const k=String(view.revision)+':'+view.sekunden;if(stamp!==k){stamp=k;zeichnen();}}},1000);
 function vorlage(p){quelle=(f()?.start.value||'')+':'+(f()?.startart?.value||'planet');personenWahl=voelker.map((_,i)=>[0,1,2].map(j=>Math.max(0,Math.floor((p?.gruppen?.[i]?.[j]||0)/1000))));if($('fl-personen'))$('fl-personen').dataset.stamp='';zeichnen();}
 return {personen,aktion,zeichnen,vorlage};
})();
