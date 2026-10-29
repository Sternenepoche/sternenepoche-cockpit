'use strict';
// Spieler- und Systemprofil (Karl 24.10.2026: „dass man den Spieler anklicken will, man dann sein Profil mit
// Heimatplanet sieht, wenn dieser erkundet wurde, oder von dort Spionageversuche oder davon angegriffen wurde“).
// Karten im nicht-modalen Funk-Blatt (berichtszentrale.js), aufrufbar überall:
//   Spielerprofil.oeffnen(name)   – Name aus Berichten, Briefen, Galaxie (auch {spieler:'…'})
//   Systemprofil.oeffnen('S:SSS') – Plätze, Besitzer, Scanalter, Gürtel, Trümmer, eigene Berichte
//   Systemprofil.planet('S:SSS:P')– ein Platz: die eine Planeten-Infokarte der Galaxie (GalaxieTabelle.zeigen)
// Gezeigt wird nur, was die Sicht des Spielers hergibt (Atlas, Berichte, Kämpfe, Briefe, Rangliste, Beziehung).
// Notizen liegen weiter je Konto und Welt auf dem Server (/api/akte/notiz, gegnerakte.js).
window.Spielerprofil=(()=>{
 const F=()=>window.Funk,H=()=>Funk.hilfe;
 const ZEITLEISTE=15;
 const notizStatus=new Map(),geschenk=new Map();
 // Nach einer Änderung gilt „Gespeichert“ nicht mehr (Nachprüfung 25.10.2026).
 document.addEventListener('input',e=>{const t=e.target;if(t?.id!=='sp-notiz')return;const n=t.dataset.spNotizName;if(n&&notizStatus.has(n)){notizStatus.delete(n);const s=document.getElementById('sp-notiz-status');if(s)s.textContent='';}});
 const mehr=new Map();
 function volkVon(name){
  for(const b of view?.berichte||[])if(b.besitzer===name&&b.volk)return b.volk;
  for(const b of view?.monde?.spionageberichte||[])if(b.besitzer===name&&b.volk)return b.volk;
  for(const k of view?.kampfberichte||[]){const kz=Object.entries(k.spielernamen||{}).find(([,n])=>n===name)?.[0];if(kz&&k.spielervoelker?.[kz])return k.spielervoelker[kz];}
  return '';
 }
 function steckbrief(name,x,akte){
  const {h}=H(),r=(view?.rangliste||world?.rangliste||[]).find(e=>e[1]===name),b=typeof relationFor==='function'?relationFor(name,null):null;
  const volk=volkVon(name),heimat=akte.heimat||x?.heimat||H().spielerHeimat(name);
  const feld=(t,w)=>`<div><dt>${t}</dt><dd>${w}</dd></div>`;
  return `<dl class="sp-steckbrief sp-steckbrief-kurz">${feld('Rang',r?fmt(r[0]):'–')}${feld('Punkte',r?fmt(Number(r[2])||0):'–')}${feld('Zeitalter',r&&r[3]!=null?H().ROEMISCH[r[3]]||h(r[3]):'–')}${feld('Volk',volk?h(named(volk)):'unbekannt')}${feld('Allianz',b?.allianz?`${b.allianz_tag?`[${h(b.allianz_tag)}] `:''}${h(b.allianz)}`:'keine')}${feld('Beziehung',`<span class="player-relation${b?.status&&b.status!=='neutral'?' relation-'+h(b.status):''}">${h(b?.bezeichnung||'Neutral')}</span>`)}</dl><div class="sp-heimat">${heimat?`<button type="button" class="bz-k bz-aktion" data-sy-planet="${h(heimat)}"><span>Heimatwelt <b class="koord">[${h(heimat)}]</b></span></button>`:'<p class="bz-leise">Heimatwelt noch nicht erkundet – ein Kampf- oder Spionagebericht zeigt sie.</p>'}</div>`;
 }
 function lage(x){
  if(!x)return '';
  const c=[x.anflug?`<span class="sp-chip rot"><b>${x.anflug}</b> Anflug läuft</span>`:'',x.angriffe?`<span class="sp-chip rot"><b>${x.angriffe}</b> Angriff${x.angriffe>1?'e':''} auf dich${x.niederlagen?` · ${x.niederlagen} verloren`:''}</span>`:'',x.fremdSpionage?`<span class="sp-chip warn"><b>${x.fremdSpionage}×</b> spioniert dich</span>`:'',x.eigeneAngriffe?`<span class="sp-chip"><b>${x.eigeneAngriffe}</b> deine Angriffe</span>`:'',x.eigeneSpionage?`<span class="sp-chip"><b>${x.eigeneSpionage}</b> deine Scans</span>`:'',x.briefe?`<span class="sp-chip"><b>${x.briefe}</b> Brief${x.briefe>1?'e':''}</span>`:''].filter(Boolean);
  return c.length?`<div class="sp-lage">${c.join('')}</div>`:'';
 }
 function scanZeile(b){
  const {summe}=H();
  if(!b)return '<span class="bz-leise">noch kein Scan</span>';
  const beute=window.Flottenfuehrung?.beute?.(b)?.summe;
  return [`<span>${b.alter_stunden!=null?`Scan vor ${b.alter_stunden<1?'unter 1':fmt(b.alter_stunden)} h Spielzeit`:'Scan vorhanden'}</span>`,beute!=null?`<span>Beute ≈ <b>${fmt(Math.round(beute))}</b></span>`:'',`<span>${b.schiffe!=null?`<b>${fmt(summe(b.schiffe))}</b> Schiffe`:'Flotte ?'}</span>`,`<span>${b.verteidigung!=null?`<b>${fmt(summe(b.verteidigung))}</b> Abwehr`:'Abwehr ?'}</span>`].filter(Boolean).join('');
 }
 function planetKarte(p,name){
  const {h}=H(),b=p.bericht;
  return `<li class="sp-planet${p.heimat?' heimat':''}"><div class="sp-planet-kopf"><button type="button" class="bz-link koord" data-sy-planet="${h(p.koord)}">[${h(p.koord)}]</button>${p.heimat?'<span class="sp-marke gold">Heimatwelt</span>':''}${p.angriffVon?'<span class="sp-marke rot">Angriffe von hier</span>':''}${p.mond?'<span class="sp-marke">☾ Mond</span>':''}</div><p class="sp-scan">${scanZeile(b)}</p><div class="bz-aktionen-reihe">${F().spion(p.koord)}${F().plan(p.koord,'angriff','Angriff planen')}${b?F().knopf(`data-bz-oeffnen="b:${h(b._postfach_id)}"`,'Letzter Bericht'):''}</div></li>`;
 }
 function karte(name){
  if(!view||!name)return null;
  const {h}=H(),m=F().modell(),x=m.spieler.get(name),akte=window.Gegnerakte?.sammeln?.(name)||{planeten:[],angriffe:[],heimat:''};
  const eintraege=(x?.eintraege||[]).filter(e=>F().ausblendenSichtbar()||!e.aus),n=mehr.get(name)||ZEITLEISTE,eigen=name===view.name;
  const ziel=akte.heimat||x?.heimat||akte.planeten[0]?.koord||'';
  const aktionen=eigen?'':`<div class="bz-aktionen-reihe">${F().knopf(`data-sp-brief="${h(name)}"`,'Nachricht',{haupt:true})}${ziel?F().spion(ziel)+F().plan(ziel,'angriff','Angriff planen'):''}${F().knopf(`data-bz-filter-spieler="${h(name)}"`,'In der Chronik')}${F().knopf(`data-sp-notiz-zeigen="${h(name)}"`,'Notiz')}${F().knopf(`data-sp-schenken="${h(name)}"`,'Credits schenken')}</div>`;
  const anfluege=akte.angriffe?.length?`<section class="sp-abschnitt"><h3>Anflug läuft · ${akte.angriffe.length}</h3><ul class="sp-liste">${akte.angriffe.map(f=>`<li class="rot"><b>→ <span class="koord">[${h(f.ziel)}]</span></b> von <span class="koord">[${h(f.start||'?')}]</span>${f.start_mond?' ☾':''}<small>Ankunft ${h(f.ankunft||'')} · ${f.schiffe!=null?fmt(f.schiffe)+' Schiffe':f.schiffe_spanne?h(f.schiffe_spanne.join('–'))+' Schiffe':'Stärke unbekannt'}</small></li>`).join('')}</ul></section>`:'';
  const planeten=`<section class="sp-abschnitt"><details open id="sp-planeten"><summary><h3>Bekannte Planeten · ${akte.planeten.length}</h3></summary>${akte.planeten.length?`<ul class="sp-planeten">${akte.planeten.map(p=>planetKarte(p,name)).join('')}</ul>`:'<p class="bz-leise">Noch kein Planet bekannt. Ein Systemscan oder eine Spionage füllt das Profil von selbst.</p>'}</details></section>`;
  const zeit=`<section class="sp-abschnitt"><details open id="sp-zeitleiste"><summary><h3>Zeitleiste · ${eintraege.length}</h3></summary>${eintraege.length?`<ol class="bz-liste sp-zeitleiste">${eintraege.slice(0,n).map(e=>F().zeileHTML(e)).join('')}</ol>${eintraege.length>n?`<button type="button" class="bz-k bz-knopf bz-mehr" data-sp-mehr="${h(name)}">Ältere zeigen · noch ${fmt(eintraege.length-n)}</button>`:''}`:'<p class="bz-leise">Noch keine Berichte oder Briefe mit diesem Reich.</p>'}</details></section>`;
  const notiz=eigen?'':`<section class="sp-abschnitt sp-notiz" id="sp-notiz-bereich"><h3>Notiz</h3><label class="bz-sr" for="sp-notiz">Eigene Notiz zu ${h(name)}</label><textarea id="sp-notiz" data-sp-notiz-name="${h(name)}" maxlength="4000" rows="3" placeholder="Nur du siehst diese Notiz – z. B. Fluchtzeiten, Verbündete, Absprachen.">${h(view.gegner_notizen?.[name]||'')}</textarea><div class="bz-aktionen-reihe">${F().knopf(`data-sp-notiz-speichern="${h(name)}"`,'Notiz speichern',{key:'notiz'})}<span class="sp-notiz-status" id="sp-notiz-status" role="status">${h(notizStatus.get(name)||'')}</span></div></section>`;
  // Credits schenken (Kern `schenken`; Aktionsabdeckung 25.10.2026: Bots konnten es, Spieler nicht). Unumkehrbar → zweiter Tipp
  // zur Bestätigung mit Betrag und Empfänger; der Kern prüft Guthaben und Empfänger.
  const gs=geschenk.get(name)||{},geschenkTeil=eigen?'':`<section class="sp-abschnitt sp-geschenk" id="sp-geschenk-bereich"${gs.offen?'':' hidden'}><h3>Credits schenken</h3>`
   +`<label class="sp-geschenk-feld">Betrag<input id="sp-geschenk-betrag" type="number" min="1" step="1" inputmode="numeric" value="${gs.betrag||''}" placeholder="z. B. 50000"></label>`
   +`<p class="bz-leise">Du hast ${fmt(Math.floor(view.credits||0))} Credits. Ein Geschenk geht sofort an ${h(name)} und lässt sich nicht zurücknehmen.</p>`
   +(gs.bestaetigen?`<p class="sp-geschenk-frage"><b>${fmt(gs.betrag)} Credits an ${h(name)} schenken?</b></p><div class="bz-aktionen-reihe">${F().knopf(`data-sp-schenken-los="${h(name)}"`,'Ja, schenken',{haupt:true})}${F().knopf(`data-sp-schenken-zurueck="${h(name)}"`,'Abbrechen')}</div>`
    :`<div class="bz-aktionen-reihe">${F().knopf(`data-sp-schenken-pruefen="${h(name)}"`,'Weiter',{haupt:true})}</div>`)
   +`<p class="sp-geschenk-status" id="sp-geschenk-status" role="status">${h(gs.status||'')}</p></section>`;
  const r=typeof relationFor==='function'?relationFor(name,null):null;
  return {augenbraue:eigen?'Dein Reich':`Spieler · ${r?.bezeichnung||'Neutral'}`,titel:name,meta:lage(x),aktionen,inhalt:steckbrief(name,x,akte)+anfluege+planeten+zeit+notiz+geschenkTeil,dossier:false};
 }
 function oeffnen(wer){
  const name=typeof wer==='object'&&wer?(wer.spieler||wer.name||wer.besitzer):wer;
  if(!name||!window.Funk)return;mehr.delete(String(name));Funk.blatt.oeffnen('spieler',String(name));
 }
 document.addEventListener('click',async e=>{
  const b=e.target.closest('[data-sp-brief],[data-sp-mehr],[data-sp-notiz-zeigen],[data-sp-notiz-speichern],[data-sp-schenken],[data-sp-schenken-pruefen],[data-sp-schenken-zurueck],[data-sp-schenken-los]');if(!b)return;
  const d=b.dataset;
  if(d.spSchenken){geschenk.set(d.spSchenken,{offen:true});Funk.blatt.zeichnen(false);setTimeout(()=>{const f=$('sp-geschenk-betrag');f?.scrollIntoView({block:'center'});f?.focus({preventScroll:true});},60);return;}
  // „1.500“ meint 1.500 Credits (Tausenderpunkt), kein Bruch – vorher wurde still 1 daraus (Funktionsprüfung 25.10.2026).
  if(d.spSchenkenPruefen){const roh=String($('sp-geschenk-betrag')?.value||'').trim().replace(/\.(?=\d{3}(\D|$))/g,'').replace(',','.'),z=Number(roh),n=Number.isInteger(z)?z:0;
   geschenk.set(d.spSchenkenPruefen,n>0&&n<=(view.credits||0)?{offen:true,betrag:n,bestaetigen:true}:{offen:true,betrag:n||'',status:n>0?`Es fehlen Credits – du hast ${fmt(Math.floor(view.credits||0))}.`:'Betrag über 0 eingeben.'});Funk.blatt.zeichnen(false);return;}
  if(d.spSchenkenZurueck){const g=geschenk.get(d.spSchenkenZurueck)||{};geschenk.set(d.spSchenkenZurueck,{offen:true,betrag:g.betrag});Funk.blatt.zeichnen(false);return;}
  if(d.spSchenkenLos){const wer=d.spSchenkenLos,g=geschenk.get(wer)||{};
   await Funk.mitKnopf(b,'schenken:'+wer,async()=>{const r=await command([{typ:'schenken',an:wer,credits:g.betrag}]);const f=r.ergebnisse?.find(x=>!x.ok);
    geschenk.set(wer,f?{offen:true,betrag:g.betrag,status:f.text}:{offen:true,status:`${fmt(g.betrag)} Credits an ${wer} geschenkt.`});if(typeof refresh==='function')await refresh();Funk.blatt.zeichnen(false);}).catch(err=>{geschenk.set(wer,{offen:true,betrag:g.betrag,status:err.message||String(err)});Funk.blatt.zeichnen(false);});
   return;}
  if(d.spBrief){if(typeof openComposer==='function'){const f=$('message-form');if(f){f.elements.antwort_auf.value='';f.elements.betreff.value='';}openComposer('privat',d.spBrief);}return;}
  if(d.spMehr){mehr.set(d.spMehr,(mehr.get(d.spMehr)||ZEITLEISTE)+ZEITLEISTE);Funk.blatt.zeichnen(false);return;}
  if(d.spNotizZeigen){const t=$('sp-notiz');t?.scrollIntoView({block:'center',behavior:matchMedia('(prefers-reduced-motion: reduce)').matches?'instant':'smooth'});t?.focus({preventScroll:true});return;}
  if(d.spNotizSpeichern){
   const t=$('sp-notiz'),status=$('sp-notiz-status');if(!t)return;
   await Funk.mitKnopf(b,'notiz',async()=>{
    status&&(status.textContent='Speichert …');
    const r=await api('/api/akte/notiz',{world_id:world.world_id,spieler:d.spNotizSpeichern,text:t.value});
    // Rückmeldung im Zustand halten: das Neuzeichnen nach dem Knopf leerte das Statusfeld nach 27 ms (Funktionsprüfung 25.10.2026).
    view.gegner_notizen=r.gegner_notizen||{};notizStatus.set(d.spNotizSpeichern,t.value.trim()?'Gespeichert':'Notiz gelöscht');if(status)status.textContent=notizStatus.get(d.spNotizSpeichern);
   }).catch(()=>{});
  }
 });
 window.Funk?.karte('spieler',karte);
 return {oeffnen,karte};
})();

window.Systemprofil=(()=>{
 const F=()=>window.Funk,H=()=>Funk.hilfe;
 const mehr=new Map();
 const plaetzeJeSystem=()=>Number(rules?.catalog?.welt?.plaetze_je_system)||12;
 // Neuester Systemscan dieses Systems (mit Einzelaufstellung der Plätze, falls gespeichert).
 const letzterScan=sys=>(view?.systemberichte||[]).filter(r=>H().sysVon(r.ziel)===sys).sort((a,b)=>(Number(b.zeit)||0)-(Number(a.zeit)||0))[0]||null;
 function platzWissen(koord){
  const atlas=(view?.atlas||[]).find(a=>a.koord===koord)||null,bericht=(view?.berichte||[]).filter(b=>b.ziel===koord).sort((a,b)=>b.zeit-a.zeit)[0]||null;
  const eigen=(view?.planeten||[]).find(p=>p.koord===koord)||null,scan=letzterScan(H().sysVon(koord))?.plaetze?.find(p=>p.koord===koord)||null;
  const gal=window.SystemIntel?.daten?.get(koord)||null,tf=(view?.truemmer||[]).find(t=>t.koord===koord)||null;
  const spieler=eigen?view.name:bericht?.besitzer||atlas?.spieler||scan?.spieler||gal?.spieler||'';
  const frei=!spieler&&(atlas?.status==='frei'||scan?.bewohnt===false||scan?.status==='frei'||gal?.status==='frei'||bericht?.art==='erkundung');
  const wissen=bericht?.aufklaerung||scan||gal||atlas||{};
  return {koord,atlas,bericht,eigen,scan,gal,tf,spieler,frei,mond:eigen?.mond||atlas?.mond||scan?.mond||gal?.mond||null,zone:eigen?.zone||wissen.zone||atlas?.zone||'',felder:eigen?.felder?.gesamt??wissen.felder??null,alter:bericht?.alter_stunden??wissen.alter_stunden??null,heimat:eigen?.heimat||bericht?.aufklaerung?.heimat||wissen.heimat||false,bekannt:!!(eigen||bericht||wissen.bekannt)};
 }
 function tfText(t){if(!t)return '';const s=(Number(t.erz)||0)+(Number(t.kristall)||0);return `Trümmerfeld ${fmt(s)}${t.ueberlebende?` · ${fmt(t.ueberlebende)} Überlebende`:''}${t.ring?' · Trümmerring':''}`;}
 function zeitleiste(eintraege,schluessel){
  const n=mehr.get(schluessel)||15;
  return eintraege.length?`<ol class="bz-liste sp-zeitleiste">${eintraege.slice(0,n).map(e=>F().zeileHTML(e)).join('')}</ol>${eintraege.length>n?`<button type="button" class="bz-k bz-knopf bz-mehr" data-sy-mehr="${H().h(schluessel)}">Ältere zeigen · noch ${fmt(eintraege.length-n)}</button>`:''}`:'<p class="bz-leise">Noch keine Berichte von hier.</p>';
 }
 function karte(sys){
  if(!view||!sys)return null;
  const {h,alter}=H(),m=F().modell(),x=m.systeme.get(sys),scan=letzterScan(sys),eintrScan=x?.eintraege.find(e=>e.kind==='systemberichte');
  const eigen=(view.planeten||[]).some(p=>H().sysVon(p.koord)===sys),kartiert=eigen||!!scan||(view.atlas||[]).some(a=>H().sysVon(a.koord)===sys&&(a.bekannt||a.besiedlung_bekannt));
  // Plätze als kompaktes Raster (6 je Zeile, Nummer + Zustand als Wort), darunter nur die besetzten Plätze mit Einzelheiten.
  const raster=[],zeilen=[];
  for(let i=1;i<=plaetzeJeSystem();i++){
   const k=`${sys}:${i}`,w=platzWissen(k),rel=w.spieler&&w.spieler!==view.name&&typeof relationFor==='function'?relationFor(w.spieler,null):null;
   const zustand=w.eigen?'eigen':w.spieler?(rel?.status==='verbuendet'?'Partner':'fremd'):w.frei?'frei':'?';
   raster.push(`<li><button type="button" class="bz-k sy-feld${w.eigen?' eigen':w.spieler?' besetzt':w.frei?' frei':' unbekannt'}${w.tf?' tf':''}" data-sy-planet="${h(k)}" aria-label="Platz ${i} in der Galaxie zeigen: ${w.eigen?'eigener Planet':w.spieler?h(w.spieler):w.frei?'frei':'unbekannt'}${w.mond?', Mond':''}${w.tf?', Trümmerfeld':''}"><b>${i}</b><small>${h(zustand)}</small>${w.mond||w.tf?`<i aria-hidden="true">${w.mond?'☾':''}${w.tf?'·TF':''}</i>`:''}</button></li>`);
   if(!w.spieler&&!w.tf)continue;
   const wer=w.eigen?`<span class="sp-eigen">${h(window.Planetenname?Planetenname.name(w.eigen):'eigener Planet')}</span>`:w.spieler?`<button type="button" class="bz-link${rel?.status&&rel.status!=='neutral'?' relation-'+h(rel.status):''}" data-sp-profil="${h(w.spieler)}">${h(w.spieler)}</button>${w.heimat?' <span class="sp-marke gold">Heimat</span>':''}`:'<span class="bz-leise">frei</span>';
   zeilen.push(`<li class="sy-platz${w.eigen?' eigen':w.spieler?' besetzt':''}"><button type="button" class="bz-k sy-nr" data-sy-planet="${h(k)}" aria-label="Platz ${i} [${h(k)}] in der Galaxie zeigen">${i}</button><div class="sy-wer">${wer}<small>${[w.mond?'☾ Mond':'',tfText(w.tf),w.alter!=null&&!w.eigen?`Scan vor ${w.alter<1?'unter 1':fmt(w.alter)} h Spielzeit`:''].filter(Boolean).join(' · ')}</small></div></li>`);
  }
  const g=scan?.asteroidenguertel?`<li class="sy-platz sy-guertel"><span class="sy-nr" aria-hidden="true">☄</span><div class="sy-wer">Asteroidengürtel<small>${scan.bergbau?`Rohstoffumfang ${h(scan.bergbau.groesse||'')} · ${fmt(scan.bergbau.erz_je_stunde)} Erz + ${fmt(scan.bergbau.kristall_je_stunde)} Kristall je Schiff und Spielstunde`:'vorhanden'}</small></div></li>`:'';
  const tfs=(view.truemmer||[]).filter(t=>H().sysVon(t.koord)===sys);
  const aktionen=`<div class="bz-aktionen-reihe">${F().imSystemKnopf(sys+':0')}${F().knopf(`data-system-scan="${h(sys)}:1"`,kartiert&&scan?'Erneut scannen · 1 Sonde':'System erkunden · 1 Sonde',{zeichen:'system_erkunden',haupt:!scan,key:'scan:'+sys})}${tfs.slice(0,1).map(t=>F().plan(t.koord,'recyceln',`Recycler nach [${h(t.koord)}]`)).join('')}${x?F().knopf(`data-bz-filter-system="${h(sys)}"`,'In der Chronik'):''}</div>`;
  const meta=`<p>${scan?`Systemscan ${eintrScan?alter(eintrScan.t):`vor ${fmt(scan.alter_stunden??0)} h`}`:eigen?'Eigenes System':'Noch nicht kartiert'}${scan?.nebel?' · Nebelsystem':''}${scan?.asteroidenguertel?' · Asteroidengürtel':''}</p>`;
  return {augenbraue:'System'+(x?.angriffe?` · ${x.angriffe} Angriff${x.angriffe>1?'e':''} auf dich`:''),titel:`System [${sys}]`,meta,aktionen,
   inhalt:`<section class="sp-abschnitt"><h3>Plätze</h3><ol class="sy-raster">${raster.join('')}</ol>${zeilen.length||g?`<ol class="sy-plaetze">${zeilen.join('')}${g}</ol>`:''}${!scan&&!eigen?'<p class="bz-leise">Ein Systemscan zeigt Besitzer, Monde und Trümmer aller Plätze.</p>':''}</section><section class="sp-abschnitt"><details open><summary><h3>Deine Berichte hier · ${x?.eintraege.length||0}</h3></summary>${zeitleiste(x?.eintraege||[],sys)}</details></section>`,dossier:false};
 }
 function oeffnen(sys){const s=window.Funk?Funk.hilfe.sysVon(sys)||String(sys||''):'';if(!s)return;mehr.delete(s);Funk.blatt.oeffnen('system',s);}
 // Ein Planet: die eine Infokarte der Galaxie (GalaxieTabelle.zeigen) – keine zweite Planetenkarte im Funk-Blatt.
 function planet(koord){if(!koord||!window.Funk)return;Funk.imSystem(String(koord));}
 document.addEventListener('click',e=>{const b=e.target.closest('[data-sy-mehr]');if(!b)return;mehr.set(b.dataset.syMehr,(mehr.get(b.dataset.syMehr)||15)+15);Funk.blatt.zeichnen(false);});
 window.Funk?.karte('system',karte);
 return {oeffnen,planet,karte};
})();
