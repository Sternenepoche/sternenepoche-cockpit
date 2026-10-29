'use strict';
// Refresh values in place. A poll must not discard decoded images, focus or local form state.
function setStableHTML(host, markup) {
 if (!host) return;
 const template = document.createElement('template');
 template.innerHTML = String(markup ?? '');
 // data-es-klapp: Klappbereiche des Einsatzblatts nach Kennung paaren – nach Position erbte „Personen“ den Offen-Zustand eines
 // anderen Bereichs und klappte beim ersten Tipp zu (Funktionsprüfung 25.10.2026).
 const key = node => {
  if (node.nodeType !== 1) return '';
  for (const attr of ['id','data-catalog-item','data-fleet-location','data-fleet-route','data-era-heading','data-fleet-pick','data-series-form','data-ground-build','data-production','data-resource','data-supply-planet','data-pick-planet','data-hangar-select','data-flight-cancel','data-map','data-es-klapp']) {
   if (node.hasAttribute(attr)) return node.tagName+':'+attr+':'+node.getAttribute(attr)+':'+(node.dataset.product||'')+':'+(node.dataset.planet||'');
  }
  return '';
 };
 const compatible = (a,b) => a.nodeType===b.nodeType && (a.nodeType!==1 || a.tagName===b.tagName && a.namespaceURI===b.namespaceURI) && key(a)===key(b);
 function patch(a,b) {
  if(a.nodeType!==1){if(a.nodeValue!==b.nodeValue)a.nodeValue=b.nodeValue;return;}
  const focused=a===document.activeElement;
  const input=a.matches('input,textarea,select'), oldValue=input?a.value:null;
  const oldDefault=a.getAttribute('value'), newDefault=b.getAttribute('value');
  const preserveValue=input&&(focused||oldDefault===newDefault);
  const open=a.tagName==='DETAILS'?a.open:null;
  // style gehört dem Skript (Fortschritt, data-css, Positionen der KI-Marken): Vorlagen setzen nie style="…" (CSP).
  // Früher fiel es bei jedem Neuzeichnen weg – Balken sprangen auf 0, Marken rutschten, Karl: „flackert die ganze Zeit“.
  const keepStyle=!b.hasAttribute('style')&&!(a.hasAttribute('data-css')&&!b.hasAttribute('data-css'));
  for(const attr of [...a.attributes])if(!b.hasAttribute(attr.name)&&!(attr.name==='open'&&open!==null)&&!(attr.name==='style'&&keepStyle))a.removeAttribute(attr.name);
  for(const attr of b.attributes)if(!(attr.name==='open'&&open!==null)&&a.getAttribute(attr.name)!==attr.value)a.setAttribute(attr.name,attr.value);
  children(a,b);
  if(open!==null)a.open=open;
  if(input){if(preserveValue&&a.value!==oldValue)a.value=oldValue;else if(!preserveValue&&newDefault!==null&&a.value!==newDefault)a.value=newDefault;}
 }
 function children(parent, desired) {
  const original=[...parent.childNodes], keyed=new Map(original.filter(n=>key(n)).map(n=>[key(n),n]));
  // Fremde Zusätze (KI-Marke aus ai-labels.js) bleiben stehen und werden übersprungen – sonst entfernt jedes Neuzeichnen
  // die Marke und ai-labels.js fügt sie im nächsten Bild wieder ein (gemessen: über 1000 Wechsel in 8 s).
  const skip=n=>{while(n&&fremd(n))n=n.nextSibling;return n;};
  let cursor=skip(parent.firstChild);
  for(const wanted of [...desired.childNodes]){
   let found=key(wanted)?keyed.get(key(wanted)):cursor&&compatible(cursor,wanted)?cursor:null;
   if(found&&!compatible(found,wanted))found=null;
   if(found){if(found!==cursor)parent.insertBefore(found,cursor);patch(found,wanted);cursor=skip(found.nextSibling);}
   else{parent.insertBefore(wanted.cloneNode(true),cursor);}
  }
  // Ein blur-Handler kann den Knoten beim Entfernen schon umgehängt haben (Funktionsprüfung 25.10.2026: NotFoundError im Blatt).
  while(cursor){const next=skip(cursor.nextSibling);if(cursor.parentNode===parent)try{cursor.remove();}catch(_){}cursor=next&&next.parentNode===parent?next:null;}
 }
 // Ebenso Zusätze, die ein Modul selbst in seinen Halter hängt (planet-kugel.js: Mondknopf über der Kugel).
 const fremd=n=>n.nodeType===1&&(n.classList.contains('ai-image-label')||n.classList.contains('kugel-zusatz'));
 children(host,template.content);
 // Zähler und Balken sofort auf den aktuellen Stand setzen, nicht erst im nächsten Takt (sonst kurz leer bzw. 0 %).
 if(typeof tickJobs==='function'&&host.querySelector('[data-job],[data-arrival],[data-arrival-echt]'))tickJobs(host);
 return markup;
}
// Spielserver-CSP (style-src 'self') blockiert style="…" in HTML-Vorlagen still – Fortschrittsbalken, Schiffsbilder, Karten-
// und Besitzerfarben fehlten online (Spielprobe 20.10.2026). Vorlagen tragen darum data-css="…"; dieser Beobachter setzt den
// Text per CSSOM (von der CSP erlaubt) – beim Laden und für jedes eingefügte oder geänderte Element.
(function cssUebertragen(){
 const setze=e=>{const t=e.getAttribute('data-css');if(e.style.cssText!==t)e.style.cssText=t;};
 const anwenden=n=>{if(n.nodeType!==1)return;if(n.hasAttribute('data-css'))setze(n);for(const e of n.querySelectorAll('[data-css]'))setze(e);};
 const beobachter=new MutationObserver(liste=>{for(const m of liste){if(m.type==='attributes')anwenden(m.target);else for(const n of m.addedNodes)anwenden(n);}});
 const los=()=>{anwenden(document.documentElement);beobachter.observe(document.documentElement,{subtree:true,childList:true,attributes:true,attributeFilter:['data-css']});};
 if(document.documentElement)los();else document.addEventListener('DOMContentLoaded',los);
})();

// ---------- Silbentrennung (26.10.2026)
(() => {
// Weiche Trennstellen für lange deutsche Wörter (Karl 25./26.10.2026: nie mitten im Wort umbrechen; Edge am Desktop trennt mit
// hyphens:auto nicht). Erst Wortfugen bekannter Grundwörter (…schiff, …kraftwerk), dann Silben je Teil: Vokalgruppen bleiben
// zusammen, von mehreren Konsonanten wandert ein gültiger Silbenanfang (bl, br, schl …) oder der letzte Konsonant in die nächste
// Silbe; ch, ck, sch, ph, th, qu bleiben ganz. Mindestens zwei Buchstaben vor und nach jeder Trennstelle. Ergebnis mit U+00AD.
const SILBEN_FUGEN = ['synthesizer', 'kolonisieren', 'wissenschaft', 'transporter', 'verbrauch', 'kapazität', 'infanterie', 'generator',
  'zerstörer', 'kraftwerk', 'förderung', 'abkommen', 'spionage', 'schlacht', 'frachter', 'satellit', 'extraktor', 'geschütz', 'schleier',
  'krümmer', 'kreuzer', 'antrieb', 'technik', 'bündnis', 'phalanx', 'zentrum', 'pionier', 'kaution', 'brecher', 'gebäude', 'archiv',
  'punkte', 'gürtel', 'schlucht', 'rakete', 'drohne', 'panzer', 'kräfte', 'werfer', 'schiff', 'stoff', 'lager', 'kunde', 'sonde', 'frost',
  'werft', 'werk', 'welt', 'mond', 'pakt', 'netz', 'dock'];
const SILBEN_VOKAL = /[aeiouäöüy]/i, SILBEN_DOPPEL = ['au', 'äu', 'eu', 'ei', 'ai', 'ey', 'ie', 'aa', 'ee', 'oo'];
const SILBEN_ANFANG = ['schl', 'schm', 'schn', 'schr', 'schw', 'sch', 'bl', 'br', 'dr', 'fl', 'fr', 'gl', 'gr', 'kl', 'kn', 'kr', 'pf',
  'pl', 'pr', 'tr', 'zw', 'ch', 'ck', 'ph', 'th', 'qu'];
function silbenTeil(w) {
  if (w.length < 6) return w;
  const k = w.toLocaleLowerCase('de'), stellen = [];
  let i = 0;
  while (i < k.length && !SILBEN_VOKAL.test(k[i])) i++;
  while (i < k.length) {
    let e = i + 1;                                           // Vokalgruppe [i, e)
    while (e < k.length && SILBEN_VOKAL.test(k[e]) && SILBEN_DOPPEL.includes(k.slice(e - 1, e + 1))) e++;
    let n = e;                                               // Konsonanten [e, n)
    while (n < k.length && !SILBEN_VOKAL.test(k[n])) n++;
    if (n >= k.length) break;                                // kein Vokal mehr: Wortende
    let s;
    if (n === e) s = e;                                      // Vokal trifft Vokal (De·u? nein: Doppellaut oben) → zwischen beide
    else {
      const c = k.slice(e, n), a = SILBEN_ANFANG.find(x => c.endsWith(x) && (c.length > x.length || x !== 'ck'));
      s = n - (a ? a.length : 1);
      if (a === 'ck') s = n - 1;                             // Zu·cker: k beginnt die Silbe
    }
    if (s >= 2 && k.length - s >= 2) stellen.push(s);
    i = n;
  }
  let r = '', z = 0;
  for (const s of stellen) { r += w.slice(z, s) + '­'; z = s; }
  return r + w.slice(z);
}
function silbenWort(w) {
  if (w.length < 10) return w;
  const k = w.toLocaleLowerCase('de');
  const f = SILBEN_FUGEN.find(x => k.endsWith(x) && w.length - x.length >= 3);
  if (f) return silbenWort(w.slice(0, w.length - f.length)) + '­' + silbenTeil(w.slice(w.length - f.length));
  return silbenTeil(w);
}
const silbenTrennen = s => String(s ?? '').replace(/[A-Za-zÄÖÜäöüß]{10,}/g, silbenWort);
window.Silben = {trennen: silbenTrennen, html: s => silbenTrennen(s).replace(/\u00ad/g, '&shy;')};
})();
