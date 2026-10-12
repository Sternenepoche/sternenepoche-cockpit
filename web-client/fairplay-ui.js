'use strict';
// Fair-Play, Harness-Schlüssel und Sitzungsübergabe. Alle fremden oder serverseitigen
// Texte gehen ausschließlich über textContent in die Seite.
window.SternenFairplay=(()=>{
 const el=(tag,props={},...kinder)=>{const e=document.createElement(tag);for(const[k,v]of Object.entries(props)){if(k==='class')e.className=v;else if(k==='text')e.textContent=v;else if(k.startsWith('data-')||k.startsWith('aria-')||['type','name','value','href','rel','tabindex','min','max','maxlength','for','id','role','title'].includes(k))e.setAttribute(k,v);else e[k]=v;}for(const kind of kinder)if(kind!=null)e.append(kind);return e;};
 const zeit=s=>new Date(Number(s)*1000).toLocaleString('de-DE',{dateStyle:'short',timeStyle:'short'});
 let regeln=null;

 // Unsichtbare Köder: nie sichtbar, nie vorgelesen, nie kopierbar, nie zurückgesendet.
 function koeder(daten,anker){
  const ziel=document.getElementById(anker);if(!ziel)return;ziel.replaceChildren();
  if(!daten||!Array.isArray(daten.eintraege))return;
  const box=el('div',{class:'fp-signal',hidden:true,'aria-hidden':'true'});
  box.append(document.createComment(' interner Datenstand '+String(daten.eintraege[0]?.text||'')+' '));
  const liste=el('ul');for(const e of daten.eintraege)liste.append(el('li',{'data-ort':String(e.ort||''),text:String(e.text||'')}));
  box.append(liste);
  if(typeof daten.link==='string'&&daten.link.startsWith('/api/intern/'))box.append(el('a',{href:daten.link,rel:'nofollow',tabindex:'-1',text:'Vollständige Sternenkarte'}));
  ziel.append(box);
 }
 // Die Köder verlassen sofort die Spieldaten, damit keine andere Funktion sie je weiterreicht.
 function lobby(world){
  if(!world)return;const d=world.oberflaeche;delete world.oberflaeche;koeder(d,'fp-anker-lobby');
  const fp=world.fairplay;const box=document.getElementById('fairplay-meldungen');if(!box||!fp)return;
  regeln=fp.regel||regeln;box.replaceChildren();
  const liste=(fp.meldungen||[]).slice(0,8);
  box.append(el('p',{class:'muted',text:fp.regel||''}),el('a',{href:'harness-regeln.html',text:'Alle Regeln für Harnesses und Fair Play'}));
  if(!liste.length)box.append(el('p',{class:'muted',text:'Keine Fair-Play-Meldungen.'}));
  else box.append(el('ul',{class:'fp-meldungen'},...liste.map(m=>el('li',{class:'fp-'+String(m.art||'')},el('time',{text:zeit(m.zeit)}),' ',el('span',{text:String(m.text||'')})))));
  const neu=liste.find(m=>Number(m.zeit)>Date.now()/1000-86400&&m.art!=='aufgehoben');const band=document.getElementById('fairplay-band');
  if(band){band.hidden=!neu;band.textContent=neu?String(neu.text):'';}
 }
 function reich(view){if(!view)return;const d=view.oberflaeche;delete view.oberflaeche;koeder(d,'fp-anker-reich');}

 // Verwarnung: Spielen erst nach ausdrücklicher Bestätigung.
 function konto(me){
  const dialog=document.getElementById('fairplay-dialog');if(!dialog)return false;
  const fp=me?.fairplay;const offen=!!fp?.bestaetigung_noetig;dialog.hidden=!offen;
  if(offen){dialog.querySelector('[data-fp-text]').textContent=String(fp.text||'');dialog.querySelector('[data-fp-regel]').textContent=String(fp.regel||regeln||'');dialog.querySelector('button').focus();}
  return offen;
 }
 async function bestaetigen(){await api('/api/fairplay/bestaetigen',{});document.getElementById('fairplay-dialog').hidden=true;show('Verwarnung bestätigt. Ein weiterer Verstoß sperrt das Konto.');await refresh();}

 // Harness-Schlüssel: nur mit Browsersitzung und bestätigten Regeln.
 async function schluesselLaden(){
  const box=document.getElementById('harness-liste');if(!box||!session)return;
  let v;try{v=await api('/api/harness/v1/schluessel');}catch(e){box.textContent=e.message;return;}
  box.replaceChildren();
  if(!v.schluessel.length){box.append(el('p',{class:'muted',text:'Noch kein Harness-Schlüssel.'}));return;}
  const tabelle=el('table',{},el('thead',{},el('tr',{},...['Bezeichnung','Rollen','Client','Gültig bis','Zuletzt',''].map(t=>el('th',{text:t})))));
  const body=el('tbody');
  for(const s of v.schluessel){
   const aktiv=!s.widerrufen&&s.aktuelle_welt&&s.gueltig_bis>v.jetzt;
   const knopf=aktiv?el('button',{type:'button',class:'secondary',text:'Widerrufen','data-harness-widerruf':String(Number(s.id))}):el('span',{class:'muted',text:s.widerrufen?'widerrufen':s.aktuelle_welt?'abgelaufen':'frühere Welt'});
   const client=[s.manifest?.client,s.manifest?.version].filter(Boolean).join(' ')||'–';
   body.append(el('tr',{},el('td',{text:String(s.label)}),el('td',{text:(s.rollen||[]).map(label).join(', ')}),el('td',{text:client}),el('td',{text:zeit(s.gueltig_bis)}),el('td',{text:s.zuletzt?zeit(s.zuletzt):'nie'}),el('td',{},knopf)));
  }
  tabelle.append(body);box.append(tabelle);
 }
 async function schluesselErstellen(form){
  const f=form.elements;const rollen=[...form.querySelectorAll('input[name="harness-rolle"]:checked')].map(i=>i.value);
  if(!rollen.length)throw Error('Mindestens eine Rolle wählen');
  if(!f['harness-regeln'].checked)throw Error('Bitte die Regeln lesen und bestätigen');
  const v=await api('/api/harness/v1/schluessel',{label:f['harness-label'].value.trim()||'Mein Harness',rollen,gueltig_tage:Number(f['harness-tage'].value)||14,regeln_akzeptiert:(typeof rules!=='undefined'&&rules?.harness_regeln)||f['harness-regeln'].value});
  const ausgabe=document.getElementById('harness-neu');ausgabe.hidden=false;ausgabe.querySelector('code').textContent=v.schluessel;
  ausgabe.querySelector('[data-harness-hinweis]').textContent=v.hinweis;form.reset();await schluesselLaden();
 }

 // „Go to Browser“ aus dem Cockpit: Einmalcode im Fragment gegen eine eigene Sitzung tauschen.
 async function uebergabe(base){
  const m=/^#uebergabe=([0-9a-f]{64})$/.exec(location.hash||'');if(!m||!base)return false;
  history.replaceState(null,'',location.pathname+location.search);
  try{const s=await api('/api/sitzung/einloesen',{code:m[1]},false);session=s;sessionStorage.setItem('sternenepoche-session',JSON.stringify({server:base,...s}));return true;}
  catch(e){show('Anmeldung aus dem Cockpit fehlgeschlagen: '+e.message+' Bitte normal anmelden.',true);return false;}
 }

 document.addEventListener('DOMContentLoaded',()=>{
  document.getElementById('fairplay-bestaetigen')?.addEventListener('click',guard(bestaetigen));
  const form=document.getElementById('harness-form');
  if(form){form.addEventListener('submit',guard(async()=>schluesselErstellen(form)));document.getElementById('harness-oeffnen')?.addEventListener('toggle',e=>{if(e.target.open)void schluesselLaden();});}
  document.addEventListener('click',guard(async e=>{const b=e.target.closest?.('[data-harness-widerruf]');if(!b)return;await api('/api/harness/v1/schluessel/widerrufen',{id:Number(b.dataset.harnessWiderruf)});show('Schlüssel widerrufen; laufende Harnesses verlieren sofort die Steuerung.');await schluesselLaden();}));
  document.getElementById('harness-kopieren')?.addEventListener('click',async()=>{const t=document.querySelector('#harness-neu code')?.textContent||'';try{await navigator.clipboard.writeText(t);show('Schlüssel kopiert. Er wird nicht erneut angezeigt.');}catch{show('Kopieren nicht möglich; Schlüssel bitte markieren und kopieren.',true);}});
 });
 return {lobby,reich,konto,uebergabe,schluesselLaden};
})();
