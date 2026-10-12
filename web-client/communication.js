'use strict';
// Private correspondence is a mailbox. Only the alliance room is a chat.
let communicationIdentity='',allianceArchiveScope='';
async function communicationCommand(actions){const r=await command(actions);if(r.ergebnisse?.some(e=>!e.ok))throw Error(r.ergebnisse.filter(e=>!e.ok).map(e=>e.text).join(' · '));return r;}
let mailFolder='eingang',openedLetter=null,mailArchive=[];
function communicationData(){return view?.kommunikation||{};}
function resetCommunication(){communicationIdentity='';allianceArchiveScope='';mailArchive=[];openedLetter=null;document.querySelectorAll('.letter-composer,.letter-reader').forEach(d=>d.close());document.querySelectorAll('.composer-form').forEach(f=>f.reset());}
const RELATION_COLORS={verbuendet:'#63d89a',nap:'#f1cf65',handelsabkommen:'#c296f5',feindlich:'#ef6b73',ausgeschieden:'#9aa0a8'};
function relationFor(name,fallback){return communicationData().kontakte?.find(c=>c.spieler===name)||fallback;}
function relationHtml(name,fallback){const r=relationFor(name,fallback);const cls=r&&Object.hasOwn(RELATION_COLORS,r.status)?' relation-'+r.status:'';return `<span class="player-relation${cls}">${esc(name)}${r?.allianz?' <small title="'+esc(r.allianz)+'">['+esc(r.allianz_tag||r.allianz)+']</small>':''}${r&&r.status!=='neutral'?' <small>· '+esc(r.bezeichnung)+'</small>':''}</span>`;}
function mailRows(channel){const seen=new Set();return [...(communicationData().briefkasten||[]),...mailArchive].filter(n=>n.kanal===channel&&!seen.has(n.id)&&seen.add(n.id)).sort((a,b)=>b.id-a.id);}
function letterList(rows,target){
 $(target).innerHTML=rows.length?'<div class="mail-letters">'+rows.map(n=>`<button class="mail-entry ${n.gelesen?'':'unread'}" data-open-letter="${esc(n.id)}"><span class="mail-avatar" aria-hidden="true">${esc((n.gesendet?n.an[0]:n.von)?.slice(0,1)||'✉')}</span><span class="mail-copy"><span class="mail-person">${n.gesendet?n.an.map(name=>relationHtml(name,n.an_beziehungen?.find(r=>r.spieler===name))).join(', '):relationHtml(n.von,n.von_beziehung)}</span><strong>${esc(n.betreff||'Nachricht')}</strong><span class="mail-preview">${esc(n.text.slice(0,160))}</span></span><span class="mail-date">${esc(n.zeit)}${n.gelesen?'':'<span class="unread-dot">Neu</span>'}</span></button>`).join('')+'</div>':`<div class="mail-empty"><span aria-hidden="true">✉</span><h3>Noch keine Briefe hier</h3><p>Wähle oben einen Verbündeten oder schreibe einen neuen Brief.</p></div>`;
}
function renderCommunication(){
 if(!view)return;const key=world.world_id+':'+view.name;if(communicationIdentity!==key){communicationIdentity=key;mailArchive=[];openedLetter=null;$('letter-reader').close();for(const d of document.querySelectorAll('.letter-composer')){d.close();d.querySelector('form').reset();}}const c=communicationData(),a=c.allianzbereich;
 const scope=JSON.stringify([key,a?.id,a?.leitung]);if(scope!==allianceArchiveScope){mailArchive=mailArchive.filter(n=>n.kanal==='privat');allianceArchiveScope=scope;}
 $('mail-folder-state').textContent=mailFolder==='eingang'?'Posteingang':'Gesendet';
 $('mail-list-pane').hidden=mailFolder==='anfragen';$('mail-requests-pane').hidden=mailFolder!=='anfragen';
 document.querySelectorAll('[data-mail-folder]').forEach(b=>b.setAttribute('aria-pressed',String(b.dataset.mailFolder===mailFolder)));
 const term=$('mail-search').value.toLocaleLowerCase('de');
 letterList(mailRows('privat').filter(n=>(mailFolder==='gesendet'?n.gesendet:!n.gesendet)&&(!term||[n.von,n.an.join(' '),n.betreff,n.text].join(' ').toLocaleLowerCase('de').includes(term))),'messages');
 const contacts=c.kontakte||[],allies=contacts.filter(r=>r.status==='verbuendet'&&r.spieler!==view.name);
 const contactStamp=JSON.stringify(contacts);if($('mail-allies').dataset.stamp!==contactStamp){$('mail-allies').dataset.stamp=contactStamp;$('mail-allies').innerHTML=allies.length?allies.map(r=>`<button type="button" class="ally-contact secondary" data-mail-to="${esc(r.spieler)}"><span class="contact-dot" aria-hidden="true"></span><span><strong>${esc(r.spieler)}</strong><small>${esc(r.allianz||'Verteidigungsbündnis')}</small></span><span aria-hidden="true">↗</span></button>`).join(''):'<p class="muted">Noch keine Verbündeten. Du kannst jeden Spieler mit bekanntem Namen anschreiben.</p>';$('known-players').innerHTML=contacts.filter(r=>r.spieler!==view.name).map(r=>`<option value="${esc(r.spieler)}">${esc(r.allianz||r.bezeichnung)}</option>`).join('');}
 $('known-alliances').innerHTML=(c.allianzen||[]).map(r=>`<option value="${esc(r.name)}"></option>`).join('');
 const unread=mailRows('privat').filter(n=>!n.gesendet&&!n.gelesen).length;$('mail-unread').textContent=unread||'';
 const requests=c.anfragen||[];
 $('diplomatic-requests').innerHTML=requests.length?requests.map(r=>`<article class="card"><div class="row"><h3>${esc(label(r.art))}</h3>${badge(r.status)}</div><p>${esc(r.von)} → ${esc(r.an)}</p><p>${esc(r.text)}</p>${r.status==='offen'&&r.an===view.name?`<button data-request="${esc(r.id)}" data-accept="true">Annehmen</button> <button class="secondary" data-request="${esc(r.id)}" data-accept="false">Ablehnen</button>`:''}</article>`).join(''):empty('Keine diplomatischen Anfragen.');
 $('alliance-state').innerHTML=a?`<h3>[${esc(a.tag)}] ${esc(a.name)}</h3><p>${a.mitglieder.length} Mitglieder · Dein Amt: ${esc(a.mitglieder.find(m=>m.name===view.name)?.rolle||'Mitglied')}</p>`:empty('Du gehörst keiner Allianz an. Gründe eine Allianz oder nimm eine Einladung an.');
 renderAllianceCodex();
 $('alliance-invitations').innerHTML=(view.einladungen||[]).map(n=>`<p>Einladung: ${esc(n)} <button data-join-alliance="${esc(n)}">Beitreten</button></p>`).join('');
 $('alliance-private').hidden=!a;$('alliance-public').hidden=!!a;
 $('alliance-leadership').hidden=!a?.gruender;
 const invite=$('alliance-invite');invite.hidden=!a?.leitung;
 $('alliance-post-nav').hidden=!a?.leitung;
 $('alliance-post-content').hidden=!a?.leitung;
 $('alliance-post-denied').hidden=!!a?.leitung;
 if(!a){$('alliance-chat-log').innerHTML='';$('alliance-members').innerHTML='';$('alliance-trades').innerHTML='';$('alliance-help-list').innerHTML='';$('alliance-letter-list').innerHTML='';$('alliance-agreements').innerHTML='';}
 else {
  $('alliance-members').innerHTML=table(['Rang','Mitglied','Amt','Welten'],a.mitglieder.map(m=>[m.rang,esc(m.name),esc(m.rolle),esc(m.planeten.join(' · '))]));
  $('alliance-chat-log').innerHTML=mailRows('allianz').reverse().map(n=>`<article class="chat-line"><strong>${esc(n.von)}</strong><time>${esc(n.zeit)}</time><p>${esc(n.text)}</p></article>`).join('')||empty('Der Allianzchat ist noch leer.');
  const offers=a.angebote||[];
  $('alliance-economy-summary').innerHTML=`<div class="stats">${stat(offers.filter(o=>o.status==='offen').length,'Offene Angebote')}${stat(offers.filter(o=>o.status==='unterwegs').length,'Unterwegs')}${stat(offers.filter(o=>o.status==='geliefert').length,'Geliefert')}</div><div class="trade-flow" aria-label="Handelsablauf"><span>Ware reserviert</span><b>→</b><span>Kauf & Zahlung</span><b>→</b><span>Neutrale Lieferung</span></div>`;
  $('alliance-trades').innerHTML=offers.length?table(['Anbieter / Start','Ware','Menge','Preis / Einheit','Status',''],offers.map(o=>[esc(o.von+' · '+o.planet),esc(label(o.gut)),fmt(o.menge),fmt(o.preis),esc(o.status)+(o.status==='unterwegs'&&o.ankunft?'<br><small>In '+esc(GameTime.duration(o.ankunft-GameTime.now()))+' Spielzeit</small>':''),o.status==='offen'?(o.von===view.name?`<button data-cancel-internal="${esc(o.id)}">Stornieren</button>`:`<button data-buy-internal="${esc(o.id)}">Angebot kaufen</button>`):esc(o.kaeufer||'')])):empty('Noch keine internen Handelsangebote.');
  $('alliance-help-list').innerHTML=(a.hilfe||[]).map(h=>`<article class="card"><h3>Hilferuf ${h.id} · ${esc(h.planet)}</h3><p>${esc(h.von)} · ${esc(h.status)}</p><p>${esc(h.text)}</p><p>Zugesagt: ${esc(h.helfer.join(', ')||'Noch niemand')}</p>${h.status==='offen'?`<button data-help="${esc(h.id)}" data-close="${esc(h.von===view.name)}">${h.von===view.name?'Hilferuf schließen':'Hilfe zusagen'}</button>`:''}</article>`).join('')||empty('Keine Hilferufe.');
  if(a.leitung){letterList(mailRows('diplomatie'),'alliance-letter-list');$('alliance-agreements').innerHTML=(c.allianzanfragen||[]).map(r=>`<article class="card"><h3>${esc(label(r.art))} · #${r.id}</h3><p>${esc(r.text)} · ${esc(r.status)}</p>${r.status==='offen'&&r.empfangsberechtigt?`<button data-request="${esc(r.id)}" data-accept="true">Annehmen</button><button data-request="${esc(r.id)}" data-accept="false">Ablehnen</button>`:r.status==='angenommen'&&['nichtangriffspakt','verteidigungsbuendnis','handelsabkommen'].includes(r.art)?`<button data-cancel-pact="${esc(r.id)}">Pakt kündigen</button>`:''}</article>`).join('');}else $('alliance-letter-list').innerHTML='';
 }
 // Never leave cached restricted content visible after membership or role changes.
 if(openedLetter&&!mailRows(openedLetter.kanal).some(n=>n.id===openedLetter.id)){openedLetter=null;$('letter-reader').close();}
 if(openedLetter)$('letter-participants').innerHTML=relationHtml(openedLetter.von,openedLetter.von_beziehung)+' → '+openedLetter.an.map(name=>relationHtml(name,openedLetter.an_beziehungen?.find(r=>r.spieler===name))).join(', ');
 if(!a?.leitung)$('diplomatie-compose-dialog').close();
 for(const select of document.querySelectorAll('.communication-planets')){const old=select.value;select.innerHTML=(view.planeten||[]).map(p=>`<option>${esc(p.koord)}</option>`).join('');if([...select.options].some(o=>o.value===old))select.value=old;}
 const g=$('internal-good');if(!g.options.length)g.innerHTML=Object.keys(view.planeten?.[0]?.bestand||{}).map(k=>`<option value="${esc(k)}">${esc(label(k))}</option>`).join('');
 window.GalaxyUI?.relations?.();
 document.querySelectorAll('.communication-text').forEach(updateLetterCount);
}
function showLetter(id){
 const n=[...(communicationData().briefkasten||[]),...mailArchive].find(n=>n.id===id);if(!n)return;
 openedLetter=n;const reader=$('letter-reader');if(!reader.open)reader.showModal();
 reader.querySelector('h3').textContent=n.betreff||'Nachricht';$('letter-metadata').textContent=`${n.von} → ${n.an.join(', ')} · ${n.zeit}${n.antwort_auf?' · Antwort auf Brief '+n.antwort_auf:''}${n.automatisch?' · Automatische Empfangs-/Systemantwort':''}`;
 $('letter-participants').innerHTML=relationHtml(n.von,n.von_beziehung)+' → '+n.an.map(name=>relationHtml(name,n.an_beziehungen?.find(r=>r.spieler===name))).join(', ');$('letter-body').textContent=n.text;$('reply-letter').hidden=n.gesendet;
}
function openComposer(channel='privat',recipient){
 const d=$(channel+'-compose-dialog'),form=d.querySelector('form');
 if(recipient){form.elements.an.value=recipient;form.elements.antwort_auf.value='';}
 form.querySelector('.composer-status').textContent='';
 if(!d.open)d.showModal();updateLetterCount(form.elements.text);
 (form.elements.an.value?form.elements.betreff:form.elements.an).focus();
}
function updateLetterCount(field){
 const max=rules?.catalog?.agenten?.nachricht_zeichen||1200;
 // Keep pasted text intact; validate instead of silently truncating at maxlength.
 const reserved=field.form?.id==='alliance-help-form'?(field.form.elements.planet.value+': ').length:0;
 const limit=max-reserved,count=Array.from(field.value).length,over=count>limit;
 field.removeAttribute('maxlength');field.setCustomValidity(over?`${count-limit} Zeichen zu viel. Bitte kürze den Text; dein Entwurf bleibt erhalten.`:'');
 let output=field.form?.querySelector('.character-count');
 if(!output&&field.form){output=document.createElement('span');output.className='character-count';field.parentElement.append(output);}
 if(output){output.textContent=`${count.toLocaleString('de-DE')} / ${limit.toLocaleString('de-DE')} Zeichen${over?' · bitte kürzen':''}`;output.classList.toggle('over-limit',over);}
}
async function sendLetterForm(form,action){
 const button=form.querySelector('[type="submit"]'),status=form.querySelector('.composer-status');
 updateLetterCount(form.elements.text);if(!form.reportValidity())return;
 button.disabled=true;status.textContent='Wird gesendet …';
 try{await communicationCommand([action]);form.elements.text.value='';form.elements.antwort_auf.value='';updateLetterCount(form.elements.text);status.textContent='Brief gesendet.';form.closest('dialog').close();}
 catch(error){status.textContent=error.message||'Senden fehlgeschlagen. Dein Entwurf bleibt erhalten.';}
 finally{button.disabled=false;}
}
function renderAllianceCodex(){
 const term=$('alliance-codex-search').value.trim().toLocaleLowerCase('de');
 const entries=(communicationData().allianzen||[]).filter(a=>!term||[a.tag,a.name,...(a.voelker||[])].join(' ').toLocaleLowerCase('de').includes(term));
 $('alliance-codex-list').innerHTML=entries.map(a=>`<article class="card"><strong>[${esc(a.tag)}] ${esc(a.name)}</strong><p>${esc(a.anzahl)} Mitglieder · ${(a.voelker||[]).map(v=>esc(named(v))).join(', ')} · Leitung: ${esc(a.leitung)}</p></article>`).join('')||empty('Keine Allianz gefunden.');
}
document.addEventListener('input',e=>{if(e.target.matches('.communication-text'))updateLetterCount(e.target);if(e.target.id==='mail-search')renderCommunication();if(e.target.id==='alliance-codex-search')renderAllianceCodex();});
document.addEventListener('click',e=>{
 const b=e.target.closest('[data-compose],[data-close-compose],[data-mail-to],[data-alliance-pane]');if(!b)return;
 if(b.dataset.compose)openComposer(b.dataset.compose);
 if(b.dataset.closeCompose)$(b.dataset.closeCompose+'-compose-dialog').close();
 if(b.dataset.mailTo)openComposer('privat',b.dataset.mailTo);
 if(b.dataset.alliancePane){document.querySelectorAll('[data-alliance-room]').forEach(p=>p.hidden=p.dataset.allianceRoom!==b.dataset.alliancePane);document.querySelectorAll('[data-alliance-pane]').forEach(t=>t.setAttribute('aria-pressed',String(t===b)));}
});
document.addEventListener('click',guard(async e=>{
 const b=e.target.closest('[data-mail-folder],[data-open-letter],[data-request],[data-join-alliance],[data-buy-internal],[data-cancel-internal],[data-help],[data-cancel-pact],[data-older-channel],#reply-letter,#close-letter,#older-letters');if(!b)return;
 if(b.dataset.mailFolder){mailFolder=b.dataset.mailFolder;renderCommunication();}
 if(b.dataset.openLetter){const id=Number(b.dataset.openLetter);showLetter(id);await communicationCommand([{typ:'brief_lesen',brief:id}]);}
 if(b.id==='close-letter'){$('letter-reader').close();openedLetter=null;}
 if(b.id==='reply-letter'&&openedLetter){const n=openedLetter,f=$(n.kanal==='diplomatie'?'alliance-post-form':'message-form');f.elements.an.value=n.kanal==='diplomatie'?n.zielallianz:n.von;f.elements.betreff.value=('Re: '+n.betreff).slice(0,120);f.elements.antwort_auf.value=n.id;$('letter-reader').close();navigateGame(n.kanal==='diplomatie'?'allianzpost':'briefkasten');openComposer(n.kanal);f.elements.text.focus();}
 if(b.dataset.cancelPact)await communicationCommand([{typ:'allianz_pakt_kuendigen',anfrage:Number(b.dataset.cancelPact)}]);
 if(b.dataset.request)await communicationCommand([{typ:'diplomatie_entscheiden',anfrage:Number(b.dataset.request),annehmen:b.dataset.accept==='true'}]);
 if(b.dataset.joinAlliance)await communicationCommand([{typ:'allianz_beitreten',allianz:b.dataset.joinAlliance}]);
 if(b.dataset.buyInternal)await communicationCommand([{typ:'intern_kaufen',angebot:Number(b.dataset.buyInternal),planet:$('internal-target').value}]);
 if(b.dataset.cancelInternal)await communicationCommand([{typ:'intern_storno',angebot:Number(b.dataset.cancelInternal)}]);
 if(b.dataset.help)await communicationCommand([{typ:'allianz_hilfe_status',hilfe:Number(b.dataset.help),erledigt:b.dataset.close==='true'}]);
 if(b.id==='older-letters'||b.dataset.olderChannel){const channel=b.dataset.olderChannel||'privat',rows=mailRows(channel),last=rows.at(-1)?.id||Number.MAX_SAFE_INTEGER;const result=await api('/api/tool',{typ:'briefarchiv',vor_id:last,limit:50,kanal:channel});mailArchive.push(...result.briefe);renderCommunication();}
}));
document.addEventListener('submit',guard(async e=>{
 const f=e.target.elements;let action;
 switch(e.target.id){
  case 'alliance-request-form':action={typ:'allianz_anfrage',allianz:f.allianz.value,art:f.art.value,text:f.text.value};break;
  case 'request-form':action={typ:'diplomatie_anfrage',partner:f.partner.value,art:f.art.value,text:f.text.value};break;
  case 'alliance-chat-form':action={typ:'brief_senden',kanal:'allianz',an:'',betreff:'Allianzchat',text:f.text.value,antwort_auf:null};break;
  case 'alliance-post-form':await sendLetterForm(e.target,{typ:'brief_senden',kanal:'diplomatie',an:f.an.value,betreff:f.betreff.value,text:f.text.value,antwort_auf:Number(f.antwort_auf.value)||null});return;
  case 'alliance-create':action={typ:'allianz_gruenden',name:f.name.value,tag:f.tag.value.trim()||null};break;
  case 'alliance-invite':action={typ:'allianz_einladen',spieler:f.spieler.value};break;
  case 'alliance-rank':action={typ:'allianz_rolle',spieler:f.spieler.value,rang:Number(f.rang.value)};break;
  case 'alliance-expel':action={typ:'allianz_ausschliessen',spieler:f.spieler.value};break;
  case 'internal-offer':action={typ:'intern_anbieten',planet:f.planet.value,gut:f.gut.value,menge:Number(f.menge.value),preis:Number(f.preis.value)};break;
  case 'alliance-help-form':action={typ:'allianz_hilfe',planet:f.planet.value,text:f.text.value};break;
 }
 if(action){await communicationCommand([action]);if(f.text)f.text.value='';if(f.antwort_auf)f.antwort_auf.value='';}
}));
document.addEventListener('click',guard(async e=>{if(e.target.closest('#leave-alliance'))await communicationCommand([{typ:'allianz_verlassen'}]);}));

function communicationFAQ(){
 const items=[
  ['Wie schreibe ich einem Spieler privat?','Wähle oben einen Verbündeten oder öffne Brief schreiben. Das große Schreibfenster hat Empfänger, Betreff und viel Platz für Absätze. Schließen und Escape behalten den Entwurf im Tab. Zu langer Text wird nicht abgeschnitten; der Zeichenzähler bittet vor dem Senden ums Kürzen. Eingang und Gesendet sind getrennt; öffne einen Brief zum Lesen und benutze Antworten. Ältere Briefe kannst du nachladen. Das ist persönliche Post. Nur im Allianzbereich gibt es einen gemeinsamen Chat.','schreiben'],
  ['Wie sende ich NAP, Allianzangebot oder Kriegserklärung?','Benutze Diplomatische Anfragen im Briefkasten. NAP, Verteidigungsbündnis, Handelsabkommen und Frieden benötigen Annahme. Die Kriegserklärung gilt sofort. Eine Allianzeinladung benötigt Leitungsrechte und die Annahme des Empfängers. Ein normaler Brieftext schließt keinen Spielvertrag. Schutzverträge haben grundsätzlich 48 Spielstunden Kündigungsfrist; Handelsabkommen enden sofort. Laufende Flotten stoppen dadurch nicht automatisch.','anfragen'],
  ['Wie gründe ich eine Allianz und verteile Rollen?','Öffne Allianz, wähle einen freien Namen und gründe sie. Als Leitung lädst du Spieler ein; sie müssen beitreten. Die Liste zeigt Mitglieder, Rollen und ihre Planeten. Die ersten vier Positionen heißen Leitung, Stellvertretung, Diplomatie und Quartiermeister. Nur die Leitung kann Positionen tauschen und Mitglieder ausschließen. Grundsätzlich sind acht Mitglieder möglich.','gruenden'],
  ['Wer sieht den Allianzchat?','Nur Mitglieder deiner Allianz lesen und schreiben hier. Nach Austritt entfällt der Zugriff serverseitig. Später beigetretene Mitglieder sehen keine früher an andere Mitglieder adressierten Beiträge. Persönliche Briefe bleiben im eigenen Briefkasten.','chat'],
  ['Wie schreiben Allianzen miteinander?','Allianzpost · Führung ist ein eigener Briefkasten. Adressiere eine Allianz mit ihrem Namen. Nur die obersten vier Mitglieder der beiden beteiligten Allianzen dürfen die Post lesen und beantworten. Hier bietet ihr auch gemeinsame Pakte und Frieden an oder erklärt Krieg. Angenommene Pakte gelten für die aktuellen Mitglieder.','allianzpost'],
  ['Wie funktioniert die interne Wirtschaft?','Im Allianzbereich stellst du Ware, Verkaufsplanet, Menge und Preis ein. Die Ware wird reserviert. Käufer wählen ihren Zielplaneten und zahlen einschließlich Gebühren; der Verkäufer erhält den Nettoerlös. Marktfreischaltung und gemeinsame Orderlimits gelten. Die Übersicht zeigt Angebot, Kauf, unterwegs und geliefert. Blockaden verzögern die Ankunft. Offene Angebote können storniert werden.','wirtschaft'],
  ['Wie bitte ich bei einem Angriff um Hilfe?','Erstelle einen Hilferuf mit eigenem Planeten und Beschreibung. Mitglieder können Hilfe zusagen. Eine Zusage startet keine Flotte: Entsatz oder Transport muss separat befohlen werden. Der Ersteller schließt den Hilferuf ab. So bleibt sichtbar, was zugesagt wurde und was noch zu tun ist.','hilfe'],
  ['Antworten Bots und Modelle auch?','Ja. Bots und freigegebene Modelle erhalten dieselben berechtigten Ansichten und Aktionen. Sie können sich vorstellen, Gespräche beginnen, ablehnen und selbst Diplomatie betreiben. Bleibt eine erste Antwort aus, folgt nach einer Spielstunde eine ausdrücklich automatische Empfangsantwort. Sie ersetzt keine Modellentscheidung und erzeugt keine Antwortschleife. Bei Pause vergeht keine Spielzeit.',''],
  ['Was bedeuten die Farben?','Verbündet: grün. NAP: gelb. Handelsabkommen: violett. Feindlich: rot. Ausgeschieden: grau. Neutral: keine Färbung. Allianzname und Beziehung stehen im Postfach und an bereits bekannten Spielern in der Galaxie. Bei mehreren Beziehungen gilt diese Priorität: ausgeschieden, feindlich, verbündet, NAP, Handel, neutral. Unbekannte Besitzer werden dadurch nicht aufgedeckt.',''],
  ['Wie hilft Kommunikation der nächsten Epoche?','Die Datenbasis erfasst Nachrichten, Vertragsentscheidungen, Mitgliedschaften, Handel, Lieferungen und Hilferufe. Spezialisten werden zusätzlich zum Gesamtsieger nach Wirtschaft, Forschung, Diplomatie, Logistik und Zuverlässigkeit ausgewählt. Nachrichtenmenge und Hilfsversprechen zählen nicht als Erfolg. Auditierte echte Modellläufe können Trainingssubsets und wählbare Harness-Varianten liefern. Bewertung bleibt auf getrennten Weltseeds; die Vorbereitung startet noch kein Modelltraining.','']
 ];
 return '<h3>Post, Allianzen und Diplomatie</h3><p>Aufnahmen aus einer vorbereiteten lokalen Beispielwelt mit der echten Spiel-API.</p><div class="communication-faq">'+items.map(([title,body,pic])=>`<details><summary>${esc(title)}</summary><p>${esc(body)}</p>${pic?`<a href="guide/${pic}.webp" target="_blank" rel="noopener"><img loading="lazy" src="guide/${pic}.webp" alt="Spielansicht: ${esc(title)}" width="1440" height="1080"></a>`:''}</details>`).join('')+'</div>';
}
