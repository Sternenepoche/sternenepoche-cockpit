'use strict';
// Sternenepoche · Technologiebaum (externe Testseite). Daten: daten.js aus tools/techbaum_bauen.py, Kampf: kampf.js.
// Start: TechbaumStart({wurzel, basis, eingebettet, spieler, aktiv, schliessen}).
// wurzel = document (Testseite) oder ShadowRoot (im Spiel, CSP-sicher: keine Inline-Styles, nur data-st → element.style).
window.TechbaumStart = (OPT = {}) => {
const D = window.TECHBAUM, K = D.knoten, BASIS = OPT.basis ?? '../../web-client/', EINGEBETTET = !!OPT.eingebettet;
const W = OPT.wurzel || document;
// Zivilisationsstufen wie im Spiel (römisch).
const ROEM = {1: 'I', 2: 'II', 3: 'III', 4: 'IV', 5: 'V'};
{ const box = document.createElement('div'); box.innerHTML = GERUEST; (W === document ? document.body : W).append(...box.childNodes); }
const $ = id => W.getElementById(id);
// Eigenes Techbaum-Logo (Vierstern + Leiterbahn-Baum, docs/website/logo/techbaum-logo.json), als data:-URL eingebettet:
// jeder Einsatzort (Spiel, APK, Testseite, Website) findet es ohne Pfad; CSP img-src erlaubt data:.
W.querySelector('.marke-logo').src = LOGO;
// Inline-Styles sind im Spiel per CSP verboten: Vorlagen tragen data-st, hier wird es über element.style gesetzt.
function stileSetzen(el) { for (const teil of el.getAttribute('data-st').split(';')) { const i = teil.indexOf(':'); if (i > 0) el.style.setProperty(teil.slice(0, i).trim(), teil.slice(i + 1).trim()); } el.removeAttribute('data-st'); }
function stileUnter(el) { if (el.hasAttribute?.('data-st')) stileSetzen(el); for (const x of el.querySelectorAll?.('[data-st]') || []) stileSetzen(x); }
new MutationObserver(ms => { for (const m of ms) for (const n of m.addedNodes) if (n.nodeType === 1) stileUnter(n); }).observe($('app'), {childList: true, subtree: true});
const esc = s => String(s ?? '').replace(/[&<>"']/g, c => ({'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;',"'":'&#39;'}[c]));
// Lange Namen (Kachel 112 px) nach Fugen und Silben weich trennen – gleiche Regeln wie window.Silben im Spiel.
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
function trennen(name) { return silbenTrennen(name); }
window.silbenTrennen = window.silbenTrennen || silbenTrennen;  // auch für kampf.js
const escT = s => esc(trennen(s)).replace(/­/g, '&shy;');
const fmt = (n, st = 1) => Number(n || 0).toLocaleString('de-DE', {maximumFractionDigits: st});
const kurz = n => n >= 1e9 ? fmt(n / 1e9, 2) + ' Mrd.' : n >= 1e6 ? fmt(n / 1e6, 2) + ' Mio.' : fmt(n, n < 10 ? 2 : 0);
const mal = f => '×' + fmt(f, 2);
const prozent = f => (f > 1 ? '+' : '−') + fmt(Math.abs(f - 1) * 100, 0) + ' %';
const speicher = {lies(k){try{return localStorage.getItem('techbaum.' + k);}catch{return null;}}, schreib(k, v){try{localStorage.setItem('techbaum.' + k, v);}catch{}}};

/* ---------- Grunddaten ---------- */
const BEREICH = Object.fromEntries(D.bereiche.map((b, i) => [b.id, {...b, i}]));
const TYP = {gebaeude:'Gebäude', forschung:'Forschung', schiff:'Schiff', megaschiff:'Megaschiff', verteidigung:'Verteidigung', schild:'Planetenschild', satellit:'Satellit',
  boden:'Bodentruppe', rakete:'Rakete', mond:'Mondanlage', wunder:'Allianzwunder', gut:'Gut', bauteil:'Bauteil', kennzahl:'Kennzahl', rolle:'Regierungsrolle',
  mission:'Mission', diplomatie:'Diplomatie', vertrag:'Vertrag', zone:'Zone', planet:'Planetentyp', himmel:'Himmelskörper', mondtyp:'Mond'};
const EINHEIT = new Set(['schiff', 'megaschiff', 'verteidigung', 'schild', 'satellit']);
const MIT_STUFEN = new Set(['gebaeude', 'forschung', 'mond']);
const WIRT = new Set(['labor', 'werft', 'kaserne', 'raketensilo', 'werftdock']);
const HART = new Set(['lokal', 'heimat', 'reich', 'mond', 'gut', 'einheit', 'bedingung']);
const ORT = {lokal: '', heimat: 'auf der Heimatwelt', reich: 'irgendwo im Reich', mond: 'auf einem eigenen Mond', gut: 'Güterbedarf', einheit: 'Einheit in der Flotte',
  bedingung: 'Bedingung', versorgung: 'Versorgung', nutzt: 'hängt zusammen'};
const STUFE = D.stufen;
const name = id => K[id]?.name || D.namen?.[id] || (id.startsWith('stufe_') ? STUFE[+id.slice(6) - 1]?.name : id);
const bildVon = (b, v) => b ? BASIS + (b[v] || b._ || Object.values(b)[0] || '') : '';
const bild = id => K[id] ? bildVon(K[id].bild, zustand.volk) : id.startsWith('stufe_') ? bildVon(STUFE[+id.slice(6) - 1].bild, zustand.volk) : '';
// Kacheln (84 px) und Stufenköpfe zeigen Vorschaubilder aus assets/klein/ (192 px, tools/techbaum_kleinbilder.py) statt der
// 480-px-Motive: am Handy dekodierte der erste Flug in einen Bereich sonst ~200 MB (Ruckeln, Android beendete andere Apps).
// Fehlt eine kleine Fassung (ältere Server, Website), lädt der Fehlerhaken unten das große Bild.
const bildKlein = id => bild(id).replace(/assets\/(?!klein\/)([^/?]+\.webp)/, 'assets/klein/$1');
W.addEventListener('error', e => { const i = e.target; if (i?.tagName === 'IMG' && i.src.includes('/assets/klein/')) i.src = i.src.replace('/assets/klein/', '/assets/'); }, true);
const res = g => D.ressourcen[g] ? BASIS + D.ressourcen[g] : '';
const gutName = g => D.namen?.[g] || g;
const farbe = id => K[id] ? BEREICH[K[id].bereich].farbe : 'var(--gold)';

/* ---------- Zustand ---------- */
const hash = new URLSearchParams(EINGEBETTET ? '' : location.hash.slice(1));
const zustand = {
  volk: D.voelker[OPT.spieler?.()?.volk] ? OPT.spieler().volk : D.voelker[hash.get('volk')] ? hash.get('volk') : D.voelker[speicher.lies('volk')] ? speicher.lies('volk') : 'aurelianer',
  zu: new Set(), gewaehlt: null, plan: null, meine: 0, volksmodus: false, allelinien: false,
  cam: {x: 0, y: 0, s: 0.4},
};
const vorausVon = (n, v = zustand.volk) => [...(n.voraus || []), ...(n.voraus_volk?.[v] || [])];
let nach = {};
function baueNach() { nach = {}; for (const n of Object.values(K)) for (const v of vorausVon(n)) if (HART.has(v.art)) (nach[v.id] ||= []).push({id: n.id, n: v.n, art: v.art, oder: v.oder}); }
const verstaerktVon = {};
for (const n of Object.values(K)) for (const v of n.verstaerkt || []) (verstaerktVon[v.id] ||= []).push({id: n.id, text: v.text});
const Kampf = window.TechbaumKampf({D, K, esc, fmt, kurz, bild: id => bild(id), name});

/* ---------- Volkswirkungen je Eintrag (wie im Kern angewendet) ---------- */
function volkEffekte(n, v = zustand.volk) {
  const r = D.voelker[v].regel, aus = [];
  const add = (feld, text, kleinerBesser = false) => {
    const f = r[feld]; if (f == null || Math.abs(f - 1) < 1e-9) return;
    aus.push({feld, text, f, gut: kleinerBesser ? f < 1 : f > 1});
  };
  const w = n.werte || {};
  if (EINHEIT.has(n.typ)) {
    if (w.angriff > 0) add('waffen', 'Waffenschaden');
    if (w.struktur > 0) add('panzerung', 'Panzerung (Struktur)');
    if (n.typ === 'schiff' && w.ladung > 0) add('ladung', 'Ladekapazität');
    add('werftzeit', n.typ === 'megaschiff' ? 'Bauzeit im Werftdock' : 'Werftbauzeit', true);
    if (n.einheitklasse === 'kolonie') add('kolonieschiff', 'Baukosten', true);
    if (n.typ === 'satellit') add('energie', 'Energieertrag');
    if (n.typ === 'schiff' && w.ladung > 0 && w.angriff > 0 && r.pluenderquote != null)
      aus.push({feld: 'pluenderquote', text: `Plünderquote ${fmt(r.pluenderquote * 100, 0)} % statt ${fmt(D.pluenderquote * 100, 0)} %`, f: r.pluenderquote / D.pluenderquote, gut: r.pluenderquote > D.pluenderquote});
  }
  if (n.typ === 'rakete' && w.schaden > 0) add('waffen', 'Raketenschaden');
  if (n.typ === 'forschung' || n.id === 'gut_forschungspunkte') add('forschung', 'Forschungstempo');
  if (n.id === 'labor' || n.id === 'forschungsarchiv') add('forschung', 'Forschungstempo des Reichs');
  if (n.id === 'farm' || n.id === 'gut_nahrung') { add('nahrung', 'Nahrungsertrag'); if (r.ohne_nahrung) aus.push({feld: 'ohne_nahrung', text: `Syntheten essen nicht – sie brauchen ${fmt(D.energie_je_1000_syntheten)} Strom je 1000 Einwohner und Stunde`, f: 1, gut: true}); }
  if (n.id === 'agrarwissenschaft') add('nahrung', 'Nahrung (wirkt mit Agrarbonus zusammen)');
  if (['solarkraftwerk', 'fusionskraftwerk', 'versorgungsnetz', 'energietechnik', 'gut_energie'].includes(n.id)) add('energie', 'Energieerzeugung');
  if (n.id === 'markt' || n.id === 'vertrag_handelsabkommen') add('marktgebuehr', 'Marktgebühr', true);
  if (['wohnblock', 'orbitalring', 'kz_bevoelkerung'].includes(n.id)) add('wachstum', 'Bevölkerungswachstum');
  if (['wohnblock', 'verwaltungszentrum', 'gut_credits', 'kz_bevoelkerung'].includes(n.id)) add('steuer', 'Steuern je Einwohner');
  if (n.id === 'werft' || n.id === 'orbitalwerft' || n.id === 'werftdock') add('werftzeit', 'Bauzeit aller Einheiten', true);
  if (n.id === 'waffentechnik') add('waffen', 'Waffenschaden (Grundfaktor)');
  if (n.id === 'panzerung') add('panzerung', 'Struktur (Grundfaktor)');
  if (n.id === 'logistik') add('ladung', 'Ladekapazität (Grundfaktor)');
  if (n.id === 'mission_kolonisieren') add('kolonieschiff', 'Kosten der Kolonieschiffe', true);
  if (n.id === 'mission_angriff' && r.pluenderquote != null)
    aus.push({feld: 'pluenderquote', text: `Plünderquote ${fmt(r.pluenderquote * 100, 0)} % statt ${fmt(D.pluenderquote * 100, 0)} %`, f: r.pluenderquote / D.pluenderquote, gut: r.pluenderquote > D.pluenderquote});
  if (n.typ === 'zone') {
    const H = D.voelker[v].habitat, [von, bis] = n.bahnen;
    if (H.heimat >= von && H.heimat <= bis) aus.push({feld: 'heimat', text: `Heimatzone (Heimatbahn ${H.heimat})`, f: 1, gut: true});
    if (n.voraus_volk?.[v]) aus.push({feld: 'terraforming', text: n.voraus_volk[v][0].text ? 'Terraforming nötig ' + n.voraus_volk[v][0].text : 'Terraforming nötig', f: 1, gut: false});
  }
  if (n.bedingungen_volk?.[v] && !['zone', 'kennzahl'].includes(n.typ) && n.id !== 'farm')
    aus.push({feld: 'besonders', text: n.bedingungen_volk[v].join(' '), f: 1, gut: true});
  return aus;
}
const effektZahl = e => ['ohne_nahrung', 'pluenderquote', 'heimat', 'terraforming', 'besonders'].includes(e.feld) ? (e.gut ? '+' : '−') : prozent(e.f);

/* ---------- Layout ---------- */
const CW = 116, CH = 138, LINKS = 196, OBEN = 104, PAD_T = 24, PAD_B = 14, MIN_BAND = 172, BAHN_GAP = 12, PAD_B_X = 14, B_GAP = 22, ZU_W = 66;
let LAY = null;
function gate(n) { return Math.max(0, ...n.voraus.filter(v => WIRT.has(v.id) || v.id === 'raumhafen').map(v => typeof v.n === 'number' ? v.n : 0)); }
function layout() {
  const ordnung = {}; D.bereiche.forEach(b => b.bahnen.forEach(l => l.knoten.forEach((k, i) => ordnung[k] = i)));
  const zellen = {}, spalten = {};
  for (const b of D.bereiche) for (const l of b.bahnen) {
    const proStufe = {};
    for (const k of l.knoten) (proStufe[K[k].stufe] ||= []).push(k);
    for (const s in proStufe) proStufe[s].sort((a, c) => gate(K[a]) - gate(K[c]) || ordnung[a] - ordnung[c]);
    zellen[l.id] = proStufe;
    spalten[l.id] = l.spalten || Math.max(1, Math.min(3, Math.ceil(Object.values(proStufe).reduce((m, a) => Math.max(m, a.length), 1) / 2)));
  }
  const baender = []; let y = OBEN;
  for (const st of STUFE) {
    let reihen = 1;
    for (const b of D.bereiche) { if (zustand.zu.has(b.id)) continue;
      for (const l of b.bahnen) reihen = Math.max(reihen, Math.ceil((zellen[l.id][st.nr]?.length || 0) / spalten[l.id])); }
    const h = Math.max(MIN_BAND, PAD_T + reihen * CH + PAD_B);
    baender.push({nr: st.nr, y, h}); y += h;
  }
  const pos = {}, bereiche = []; let x = LINKS;
  for (const b of D.bereiche) {
    const zu = zustand.zu.has(b.id); let bx = x + PAD_B_X; const bahnen = [];
    if (zu) {
      for (const l of b.bahnen) for (const s in zellen[l.id]) { const band = baender[s - 1];
        for (const k of zellen[l.id][s]) pos[k] = {x: x + ZU_W / 2 - 48, y: band.y + band.h / 2 - 42, zu: true}; }
      bereiche.push({...b, x, w: ZU_W, zu, bahnen}); x += ZU_W + B_GAP; continue;
    }
    for (const l of b.bahnen) {
      const c = spalten[l.id];
      for (const s in zellen[l.id]) { const band = baender[s - 1];
        zellen[l.id][s].forEach((k, i) => { pos[k] = {x: bx + (i % c) * CW + (CW - 96) / 2, y: band.y + PAD_T + Math.floor(i / c) * CH, zu: false}; }); }
      bahnen.push({...l, x: bx - x, w: c * CW}); bx += c * CW + BAHN_GAP;
    }
    const w = bx - x - BAHN_GAP + PAD_B_X;
    bereiche.push({...b, x, w, zu, bahnen}); x += w + B_GAP;
  }
  LAY = {pos, baender, bereiche, w: x + 40, h: y + 40, zellen};
}

/* ---------- Aufbau DOM ---------- */
const welt = $('welt'), buehne = $('buehne'), ebene = $('knoten');
const knotenEl = {};
function baueKnoten() {
  ebene.innerHTML = '';
  for (const n of Object.values(K)) {
    const b = BEREICH[n.bereich], el = document.createElement('button');
    el.type = 'button'; el.className = 'knoten ' + n.typ; el.dataset.id = n.id; el.style.setProperty('--bf', b.farbe);
    const host = n.voraus.find(v => WIRT.has(v.id));
    const tor = host ? ({labor:'L', werft:'W', kaserne:'K', raketensilo:'S', werftdock:'D'}[host.id] + host.n) : '';
    el.innerHTML = `<span class="kachel"><img alt="" loading="lazy" decoding="async"></span>${tor ? `<span class="tor" title="${esc(name(host.id))} ${host.n}">${tor}</span>` : ''}<span class="vb" hidden></span><span class="schloss" aria-hidden="true">🔒</span><span class="lvl" aria-hidden="true"></span><span class="name">${escT(n.name)}</span><span class="info"></span>`;
    el.setAttribute('aria-label', `${n.name}, ${TYP[n.typ]}, ab Zivilisationsstufe ${ROEM[n.stufe] || n.stufe}`);
    ebene.append(el); knotenEl[n.id] = el;
  }
}
function infoZeile(n) {
  const k = Object.entries(n.kosten || {}).filter(([, v]) => v).slice(0, 2).map(([g, v]) => `${gutName(g).slice(0, 4)}. ${kurz(v)}`).join(' · ');
  return k || TYP[n.typ];
}
function aktualisiereBilder() {
  for (const n of Object.values(K)) { const el = knotenEl[n.id], img = el.querySelector('img'), u = bildKlein(n.id); if (img.getAttribute('src') !== u) img.src = u;
    el.querySelector('.info').textContent = infoZeile(n);
    const eff = volkEffekte(n), vb = el.querySelector('.vb');
    el.classList.toggle('volk-betroffen', eff.length > 0);
    if (eff.length) { const gut = eff.every(e => e.gut), schlecht = eff.every(e => !e.gut);
      vb.hidden = false; vb.className = 'vb ' + (gut ? 'plus' : schlecht ? 'minus' : 'misch'); vb.textContent = gut ? '+' : schlecht ? '−' : '±';
      vb.title = eff.map(e => `${e.text} ${effektZahl(e)}`).join('\n'); }
    else vb.hidden = true;
  }
}
function setzePositionen() {
  welt.style.width = LAY.w + 'px'; welt.style.height = LAY.h + 'px';
  for (const [id, p] of Object.entries(LAY.pos)) { const el = knotenEl[id]; el.style.left = p.x + 'px'; el.style.top = p.y + 'px'; el.classList.toggle('zu', p.zu); el.tabIndex = p.zu ? -1 : 0; }
  $('baender').innerHTML = LAY.baender.map((b, i) => { const st = STUFE[i];
    return `<div class="band${zustand.meine && st.nr > zustand.meine ? ' gesperrt' : ''}" data-st="top:${b.y}px;height:${b.h}px;width:${LAY.w}px"><button type="button" class="band-kopf" data-stufe="${st.nr}"><img alt="" src="${esc(bildKlein('stufe_' + st.nr))}"><strong>${esc(st.name)}</strong><small>${st.nr === 1 ? 'Start jeder Kolonie' : fmt(st.einwohner) + ' Einwohner' + (st.kolonien ? ` · ${st.kolonien} Kolonien` : '')}</small></button></div>`; }).join('');
  $('spalten').innerHTML = LAY.bereiche.map(b => {
    const quelle = D.bereiche.find(x => x.id === b.id);
    const zaehler = b.zu ? LAY.baender.map((band, i) => { const n = quelle.bahnen.reduce((s, l) => s + (LAY.zellen[l.id][i + 1]?.length || 0), 0);
      return n ? `<span class="zaehler" data-st="top:${band.y + band.h / 2 - 17}px">${n}</span>` : ''; }).join('') : '';
    const gesamt = quelle.bahnen.reduce((s, l) => s + l.knoten.length, 0);
    return `<section class="bereich${b.zu ? ' zu' : ''}" data-bereich="${b.id}" data-st="--bf:${b.farbe};left:${b.x}px;width:${b.w}px;height:${LAY.h - 20}px">
      <div class="bereich-kopf" data-kopf="${b.id}" title="${b.zu ? 'Aufklappen' : 'Hineinzoomen'}"><div data-st="min-width:0"><h2>${esc(b.titel)}</h2><p>${gesamt} Einträge</p></div>
      <div class="kn"><button type="button" data-info="${b.id}" title="Übersicht und Regeln" aria-label="Übersicht ${esc(b.titel)}">ⓘ</button><button type="button" data-zoom="${b.id}" title="In diesen Baum zoomen" aria-label="In ${esc(b.titel)} zoomen">⤢</button><button type="button" data-klapp="${b.id}" title="${b.zu ? 'Aufklappen' : 'Zuklappen'}" aria-label="${esc(b.titel)} ${b.zu ? 'aufklappen' : 'zuklappen'}">${b.zu ? '▸' : '▾'}</button></div>
      ${b.bahnen.map(l => `<span class="bahn-titel" data-st="left:${l.x}px;width:${l.w - 6}px">${esc(l.titel)}</span>`).join('')}</div>${zaehler}</section>`; }).join('');
  $('leiste-bereiche').innerHTML = D.bereiche.map(b => `<button type="button" data-zoom="${b.id}" data-st="--bf:${b.farbe}">${esc(b.titel)}</button>`).join('');
  zeichneLinien(); stufenStatus(); minikarteGrund = null; kopfX = kopfY = null; kamera();
}

/* ---------- Linien ---------- */
const NS = 'http://www.w3.org/2000/svg';
function anker(id) { const p = LAY.pos[id]; return p ? {x: p.x + 48, y: p.y + 42, zu: p.zu} : null; }
function pfad(a, b) {
  if (b.y - a.y > 60) { const y1 = a.y + (a.zu ? 17 : 46), y2 = b.y - (b.zu ? 17 : 46), d = Math.max(30, (y2 - y1) * .5);
    return `M${a.x},${y1} C${a.x},${y1 + d} ${b.x},${y2 - d} ${b.x},${y2}`; }
  if (a.y - b.y > 60) { const y1 = a.y - 46, y2 = b.y + 46, d = Math.max(30, (y1 - y2) * .5);
    return `M${a.x},${y1} C${a.x},${y1 - d} ${b.x},${y2 + d} ${b.x},${y2}`; }
  const r = b.x >= a.x ? 1 : -1, x1 = a.x + r * (a.zu ? 17 : 44), x2 = b.x - r * (b.zu ? 17 : 44), d = Math.max(26, Math.abs(x2 - x1) * .45);
  return `M${x1},${a.y} C${x1 + r * d},${a.y} ${x2 - r * d},${b.y} ${x2},${b.y}`;
}
let KANTEN = [];
const kanten = () => Object.values(K).flatMap(n => vorausVon(n).map(v => ({von: v.id, zu: n.id, n: v.n, art: v.art, oder: v.oder, wirt: WIRT.has(v.id)})));
function linie(g, k, klasse, zahl) {
  const a = anker(k.von), b = anker(k.zu); if (!a || !b || k.von === k.zu) return null;
  if (a.zu && b.zu && LAY.pos[k.von].x === LAY.pos[k.zu].x) return null;
  const p = document.createElementNS(NS, 'path'); p.setAttribute('d', pfad(a, b)); p.setAttribute('class', 'linie ' + klasse); g.append(p);
  if (zahl) { const t = document.createElementNS(NS, 'text'), L = p.getTotalLength(), m = p.getPointAtLength(L * .5);
    t.setAttribute('x', m.x); t.setAttribute('y', m.y + 4); t.setAttribute('text-anchor', 'middle'); t.setAttribute('class', 'linie-zahl'); t.textContent = zahl; g.append(t); }
  return p;
}
const kantenZahl = k => typeof k.n === 'number' && k.n > 1 ? kurz(k.n) : '';
function zeichneLinien() {
  KANTEN = kanten();
  const svg = $('linien'); svg.setAttribute('width', LAY.w); svg.setAttribute('height', LAY.h);
  const ruhe = $('linien-ruhe'); ruhe.innerHTML = '';
  for (const k of KANTEN) { if (!HART.has(k.art)) continue; const quer = K[k.von]?.bereich !== K[k.zu]?.bereich;
    if (!zustand.allelinien && (k.wirt || quer || k.art === 'gut')) continue;
    linie(ruhe, k, (k.wirt ? 'wirt' : quer ? 'quer' : k.art === 'gut' ? 'gut' : '') + (k.oder ? ' oder' : '')); }
  zeichneAktiv();
}
let VOR = new Set(), NACH = new Set(), PLAN = null;
// Ketten folgen nur festen Bedingungen; „eine davon“-Alternativen erscheinen als direkte Linie, nicht als Kette.
function vorfahren(id, s = new Set()) { const n = K[id]; if (!n) return s; for (const v of vorausVon(n)) if (HART.has(v.art) && !v.oder && !s.has(v.id)) { s.add(v.id); vorfahren(v.id, s); } return s; }
function nachfahren(id, s = new Set()) { for (const v of nach[id] || []) if (!v.oder && !s.has(v.id)) { s.add(v.id); nachfahren(v.id, s); } return s; }
function zeichneAktiv() {
  const g = $('linien-aktiv'); g.innerHTML = '';
  if (PLAN) for (const k of KANTEN) if (HART.has(k.art) && PLAN.bedarf.has(k.zu) && PLAN.bedarf.has(k.von)) linie(g, k, 'plan', kantenZahl(k));
  const s = zustand.gewaehlt; if (!s || !K[s]) return;
  for (const k of KANTEN) {
    if (!HART.has(k.art)) { if (k.zu === s) linie(g, k, 'weich ' + k.art, ''); continue; }
    if (k.oder) { if (k.zu === s) linie(g, k, 'vor oder', kantenZahl(k)); else if (k.von === s) linie(g, k, 'nach oder', ''); continue; }
    if ((k.zu === s || VOR.has(k.zu)) && VOR.has(k.von)) linie(g, k, 'vor' + (k.oder ? ' oder' : ''), kantenZahl(k));
    else if ((k.von === s || NACH.has(k.von)) && NACH.has(k.zu)) linie(g, k, 'nach', '');
  }
  for (const v of K[s].verstaerkt || []) linie(g, {von: s, zu: v.id}, 'boost', '');
  for (const v of verstaerktVon[s] || []) linie(g, {von: v.id, zu: s}, 'boost', '');
}
function impulse() {
  const g = $('impulse'), pfade = [...$('linien-aktiv').querySelectorAll('path.plan')]; if (!pfade.length) return;
  const start = performance.now(), punkte = pfade.map((p, i) => { const c = document.createElementNS(NS, 'circle'); c.setAttribute('r', 6); c.setAttribute('class', 'impuls'); g.append(c); return {p, c, L: p.getTotalLength(), v: (i % 6) * 60}; });
  const schritt = t => { let fertig = true;
    for (const q of punkte) { const f = Math.min(1, Math.max(0, (t - start - q.v) / 900)); if (f < 1) fertig = false; const e = 1 - Math.pow(1 - f, 3), m = q.p.getPointAtLength(q.L * e);
      q.c.setAttribute('cx', m.x); q.c.setAttribute('cy', m.y); q.c.setAttribute('opacity', f >= 1 ? 0 : 1); }
    if (!fertig) requestAnimationFrame(schritt); else g.innerHTML = ''; };
  requestAnimationFrame(schritt);
}

/* ---------- Kamera ---------- */
const MIN_S = 0.04, MAX_S = 2.4;
// Handy (< 768 px): Kopf eine Zeile, unten das Dock mit Bereichen; Blätter (Dossier, Weg) kommen von unten.
// LESBAR: kleinster Maßstab, bei dem Namen auf dem Handy mindestens 12 px groß sind (Name 13 px × 0,95).
const MOBIL_BIS = 768, LESBAR = 0.95, BLATT_HALB = .46, BLATT_GANZ = .78;
const istMobil = () => buehne.getBoundingClientRect().width < MOBIL_BIS;
// Während einer Geste ändern sich Bühne, Kopf und Dock nicht: Maße einmal je Geste lesen statt bei jedem Fingerzug
// (jede Messung erzwang am Handy eine Stilberechnung pro Bild).
let sichtFest = null;
function sicht() {
  if (sichtFest) return sichtFest;
  const r = buehne.getBoundingClientRect(), mobil = r.width < MOBIL_BIS;
  const dossierOffen = !$('dossier').hidden, planOffen = !$('plan').hidden;
  if (mobil) {
    const kopf = W.querySelector('.kopf').getBoundingClientRect().bottom - r.top, dock = $('dock')?.getBoundingClientRect();
    const unten = dossierOffen ? r.height * ($('dossier').classList.contains('halb') ? BLATT_HALB : BLATT_GANZ) : planOffen ? r.height * BLATT_HALB : dock?.height ? r.bottom - dock.top + 4 : 70;
    return {w: r.width, h: r.height, oben: kopf + 4, unten, links: 6, rechts: 6, mobil};
  }
  return {w: r.width, h: r.height, oben: r.width < 1100 ? 150 : 82, unten: 140,
    links: planOffen ? 350 : 16, rechts: dossierOffen ? 490 : 16, mobil};
}
let kameraRaf = 0;
const kameraBald = () => { if (!kameraRaf) kameraRaf = requestAnimationFrame(() => { kameraRaf = 0; kamera(); }); };
// Flackern am Handy (Karl 22.10.2026): Die Welt ist eine riesige Ebene. Jede Layout-Änderung darin (klebende Köpfe per
// left/top) zwang bei jedem Bild zum Neuzeichnen und Neurastern der ganzen Ebene – das Android-WebView kam nicht nach und
// zeigte leere Kacheln. Jetzt: erst messen, dann schreiben (kein erzwungenes Layout), klebende Köpfe nur per transform auf
// eigenen Ebenen und nur bei echter Änderung. Die Weltebene (will-change: transform) behält ihren Rastermaßstab; ändert
// ein Zoom ihn deutlich, wird sie nach der Geste genau einmal im neuen Maßstab gerastert (reine Verschiebungen nie).
let kopfX = null, kopfY = null, ruheT = 0, rasterS = null;
function scharf() {
  const s = zustand.cam.s; if (rasterS === null) { rasterS = s; return; }
  if (Math.abs(s - rasterS) / rasterS < 0.15) return; rasterS = s;
  welt.style.willChange = 'auto'; requestAnimationFrame(() => requestAnimationFrame(() => { welt.style.willChange = ''; }));
}
function bewegung() { clearTimeout(ruheT); ruheT = setTimeout(scharf, 320); }
function kamera() {
  const c = zustand.cam, v = sicht(), bx = Math.max(18, (-c.x + 8) / c.s), ky = Math.max(0, (v.oben - c.y) / c.s);
  const t = `translate(${c.x}px,${c.y}px) scale(${c.s})`;
  if (welt.style.transform !== t) { welt.style.transform = t; bewegung(); }
  welt.classList.toggle('z-makro', c.s < 0.42); welt.classList.toggle('z-detail', c.s > 1.15);
  const rx = Math.round((bx - 18) * 10) / 10, ry = Math.round(ky * 10) / 10;
  if (rx !== kopfX) { kopfX = rx; for (const k of W.querySelectorAll('.band-kopf')) { k.style.transform = rx ? `translateX(${rx}px)` : ''; k.classList.toggle('klebt', bx > 19); } }
  if (ry !== kopfY) { kopfY = ry; for (const k of W.querySelectorAll('.bereich-kopf')) { k.style.transform = ry ? `translateY(${ry}px)` : ''; k.classList.toggle('klebt', ky > 0); } }
  if (v.mobil) dockAktiv((v.w / 2 - c.x) / c.s);
  else zeichneMinikarte();
}
// Dock-Chip des Bereichs in der Bildmitte hervorheben und in Sicht rollen.
let dockBereich = null;
function dockAktiv(wx) {
  const b = LAY?.bereiche.find(x => wx >= x.x && wx < x.x + x.w + B_GAP); if (!b || b.id === dockBereich) return; dockBereich = b.id;
  const leiste = $('dock-chips'); if (!leiste) return;
  for (const x of leiste.children) x.classList.toggle('aktiv', x.dataset.zoom === b.id);
  const el = leiste.querySelector(`[data-zoom="${b.id}"]`); if (el) leiste.scrollTo({left: el.offsetLeft - leiste.offsetLeft - leiste.clientWidth / 2 + el.offsetWidth / 2, behavior: 'smooth'});
}
function begrenze() {
  const c = zustand.cam, v = sicht(), w = LAY.w * c.s, h = LAY.h * c.s;
  c.x = Math.min(v.w - 120, Math.max(c.x, 120 - w)); c.y = Math.min(v.h - 120, Math.max(c.y, 120 - h));
}
let flug = null;
function fliege(ziel, ms = 650) {
  cancelAnimationFrame(flug); schwungStopp(); sichtFest = null; const a = {...zustand.cam}, t0 = performance.now();
  if (matchMedia('(prefers-reduced-motion: reduce)').matches) ms = 1;
  const schritt = t => { const f = Math.min(1, (t - t0) / ms), e = 1 - Math.pow(1 - f, 4);
    zustand.cam = {x: a.x + (ziel.x - a.x) * e, y: a.y + (ziel.y - a.y) * e, s: a.s + (ziel.s - a.s) * e}; kamera();
    if (f < 1) flug = requestAnimationFrame(schritt); };
  flug = requestAnimationFrame(schritt);
}
function passend(r, maxS = 1.3) {
  const v = sicht(), bw = v.w - v.links - v.rechts, bh = v.h - v.oben - v.unten;
  const s = Math.max(MIN_S, Math.min(maxS, bw / r.w, bh / r.h));
  return {s, x: v.links + (bw - r.w * s) / 2 - r.x * s, y: v.oben + (bh - r.h * s) / 2 - r.y * s};
}
const gesamt = () => fliege(passend({x: 0, y: 0, w: LAY.w, h: LAY.h}));
function zumBereich(id) {
  if (zustand.zu.has(id)) { zustand.zu.delete(id); neuLayout(); }
  const b = LAY.bereiche.find(x => x.id === id), v = sicht(), bw = v.w - v.links - v.rechts, bh = v.h - v.oben - v.unten;
  if (v.mobil) {
    // Handy: lesbar an den linken Rand des Bereichs, oben die Stufe des eigenen Stands (sonst Stufe I)
    const s = Math.max(LESBAR, Math.min(1.1, zustand.cam.s)), band = LAY.baender[Math.max(0, Math.min(LAY.baender.length, S?.stufe || 1) - 1)];
    fliege({s, x: v.links - b.x * s, y: v.oben - Math.max(0, band.y - 80) * s});
  } else {
    const s = Math.max(MIN_S, Math.min(0.85, bw / (b.w + 40))), hoehe = LAY.h * s;
    fliege({s, x: b.w * s > bw ? v.links - b.x * s + 4 : v.links + (bw - b.w * s) / 2 - b.x * s, y: hoehe < bh ? v.oben + (bh - hoehe) / 2 : v.oben + 6});
  }
  W.querySelectorAll('.leiste-bereiche button,.dock-chips button').forEach(x => x.classList.toggle('aktiv', x.dataset.zoom === id));
  zeige(`${b.titel}: ${D.bereiche.find(x => x.id === id).bahnen.reduce((s, l) => s + l.knoten.length, 0)} Einträge`);
}
// Handy-Start: lesbar beim eigenen Stand – die jetzt möglichen (sonst erreichten) Einträge bis zur eigenen Stufe,
// im ersten Bereich, in dem es welche gibt. Ohne Spielstand (Testseite): Stufe I des ersten Bereichs.
function standAnsicht() {
  const v = sicht(), s = LESBAR;
  let ids = [];
  if (S) for (const k of ['moeglich', 'erreicht']) {
    ids = Object.keys(K).filter(id => LAY.pos[id] && !LAY.pos[id].zu && K[id].stufe <= S.stufe && knotenEl[id].classList.contains('st-' + k));
    if (ids.length) break;
  }
  let b;
  if (ids.length) { const bid = ids.map(id => K[id].bereich).sort((a, c) => BEREICH[a].i - BEREICH[c].i)[0]; b = LAY.bereiche.find(x => x.id === bid); ids = ids.filter(id => K[id].bereich === bid); }
  else b = LAY.bereiche.find(x => !x.zu) || LAY.bereiche[0];
  const ps = ids.map(id => LAY.pos[id]), bw = v.w - v.links - v.rechts;
  const stufe = ps.length ? Math.min(...ids.map(id => K[id].stufe)) : 1, band = LAY.baender[stufe - 1];
  const x0 = ps.length && (Math.max(...ps.map(p => p.x)) + 112 - b.x) * s > bw ? Math.min(...ps.map(p => p.x)) - 14 : b.x;
  return {s, x: v.links - x0 * s, y: v.oben - Math.max(0, band.y - 80) * s, bereich: b.id};
}
function zumStand() { const z = standAnsicht(); fliege(z); dockBereich = null; zeige(S ? `Dein Stand: ${STUFE[S.stufe - 1]?.name || ''} · ${BEREICH[z.bereich].titel}` : BEREICH[z.bereich].titel); }
function zuBand(nr) {
  const v = sicht(), c = zustand.cam, s = Math.max(LESBAR, c.s), band = LAY.baender[nr - 1]; if (!band) return;
  fliege({s, x: v.w / 2 - (v.w / 2 - c.x) / c.s * s, y: v.oben - Math.max(0, band.y - 80) * s});
}
function zuKnoten(id, s) {
  const p = LAY.pos[id], v = sicht(), c = zustand.cam, ns = s ?? Math.max(c.s, v.mobil ? LESBAR : 0.85);
  const bw = v.w - v.links - v.rechts, bh = v.h - v.oben - v.unten;
  fliege({s: ns, x: v.links + bw / 2 - (p.x + 48) * ns, y: v.oben + bh / 2 - (p.y + 60) * ns});
}
function zoomUm(f, sx, sy) {
  const c = zustand.cam, ns = Math.max(MIN_S, Math.min(MAX_S, c.s * f));
  c.x = sx - (sx - c.x) * ns / c.s; c.y = sy - (sy - c.y) * ns / c.s; c.s = ns; begrenze(); kameraBald();
}

// Schwung nach dem Wischen (Karl 22.10.2026: „ruckelt und zuckelt, keine flüssigen Bewegungen“): die Karte gleitet mit der
// Geschwindigkeit der letzten ~80 ms weiter und bremst sanft ab – wie native Listen am Handy.
let schwung = 0, spur = [];
function schwungStopp() { cancelAnimationFrame(schwung); schwung = 0; }
function schwungStart() {
  const jetzt = performance.now(), neu = spur.filter(p => jetzt - p.t < 90); spur = [];
  if (neu.length < 2 || matchMedia('(prefers-reduced-motion: reduce)').matches) return;
  const a = neu[0], b = neu[neu.length - 1], dt = Math.max(8, b.t - a.t);
  let vx = (b.x - a.x) / dt, vy = (b.y - a.y) / dt;
  if (Math.hypot(vx, vy) < 0.15) return;
  let t0 = jetzt; sichtFest = sicht();
  const schritt = t => {
    const d = Math.min(40, t - t0); t0 = t; const r = Math.pow(0.9945, d);
    const c = zustand.cam, x0 = c.x, y0 = c.y; c.x += vx * d; c.y += vy * d; begrenze();
    if (c.x === x0) vx = 0; if (c.y === y0) vy = 0; // am Rand anhalten
    vx *= r; vy *= r; kamera();
    if (Math.hypot(vx, vy) > 0.02) schwung = requestAnimationFrame(schritt); else { schwung = 0; sichtFest = null; bewegung(); }
  };
  schwung = requestAnimationFrame(schritt);
}

// Zeiger: Ziehen, Kneifen, Rad
const zeiger = new Map(); let zug = null, bewegt = 0, angefasst = false;
buehne.addEventListener('pointerdown', () => { angefasst = true; }, {capture: true});
buehne.addEventListener('pointerdown', e => {
  if (e.target.closest('.kn,.band-kopf,.leiste-bereiche')) return;
  zeiger.set(e.pointerId, {x: e.clientX, y: e.clientY});
  cancelAnimationFrame(flug); schwungStopp(); spur = []; sichtFest = null; sichtFest = sicht(); if (zeiger.size === 1) bewegt = 0;
  if (zeiger.size === 1) zug = {x: e.clientX, y: e.clientY, cx: zustand.cam.x, cy: zustand.cam.y};
  else if (zeiger.size === 2) { const [a, b] = [...zeiger.values()]; zug = {kneif: Math.hypot(a.x - b.x, a.y - b.y), s: zustand.cam.s}; bewegt = 99;
    for (const id of zeiger.keys()) try { buehne.setPointerCapture(id); } catch {} }
});
buehne.addEventListener('pointermove', e => {
  if (!zeiger.has(e.pointerId)) return; zeiger.set(e.pointerId, {x: e.clientX, y: e.clientY});
  if (zeiger.size === 2 && zug?.kneif) { const [a, b] = [...zeiger.values()], d = Math.hypot(a.x - b.x, a.y - b.y), r = buehne.getBoundingClientRect();
    zoomUm(zug.s * d / zug.kneif / zustand.cam.s, (a.x + b.x) / 2 - r.left, (a.y + b.y) / 2 - r.top); return; }
  if (zug && zeiger.size === 1 && !zug.kneif) { const dx = e.clientX - zug.x, dy = e.clientY - zug.y; bewegt = Math.max(bewegt, Math.hypot(dx, dy));
    if (bewegt > 5) { // erst beim echten Ziehen fangen, damit normale Klicks ihr Ziel behalten
      if (!buehne.hasPointerCapture?.(e.pointerId)) try { buehne.setPointerCapture(e.pointerId); } catch {}
      if (!buehne.classList.contains('zieht')) buehne.classList.add('zieht');
      spur.push({x: e.clientX, y: e.clientY, t: e.timeStamp || performance.now()}); if (spur.length > 8) spur.shift();
      zustand.cam.x = zug.cx + dx; zustand.cam.y = zug.cy + dy; begrenze(); kameraBald(); } }
});
const loslassen = e => {
  const warZug = zeiger.size === 1 && zug && !zug.kneif && bewegt > 5 && e.type === 'pointerup';
  zeiger.delete(e.pointerId); buehne.classList.remove('zieht');
  if (zeiger.size === 0) { zug = null; if (warZug) schwungStart(); else { spur = []; sichtFest = null; } }
  else if (zeiger.size === 1) { const [p] = [...zeiger.values()]; zug = {x: p.x, y: p.y, cx: zustand.cam.x, cy: zustand.cam.y}; }
};
buehne.addEventListener('pointerup', loslassen); buehne.addEventListener('pointercancel', loslassen);
// Fokus auf einen Eintrag darf die Bühne nicht selbst scrollen – die Kamera bewegt die Welt.
buehne.addEventListener('scroll', () => { if (buehne.scrollTop || buehne.scrollLeft) { buehne.scrollTop = 0; buehne.scrollLeft = 0; } });
buehne.addEventListener('focusin', e => { const k = e.target.closest?.('.knoten'); if (!k || e.target.matches(':hover')) return;
  const r = k.getBoundingClientRect(), v = sicht(); if (r.left < v.links || r.right > v.w - v.rechts || r.top < v.oben || r.bottom > v.h - v.unten) zuKnoten(k.dataset.id, zustand.cam.s); });
buehne.addEventListener('wheel', e => { e.preventDefault(); const r = buehne.getBoundingClientRect();
  if (e.ctrlKey || Math.abs(e.deltaY) >= Math.abs(e.deltaX)) zoomUm(Math.exp(-e.deltaY * (e.ctrlKey ? .01 : .0015)), e.clientX - r.left, e.clientY - r.top);
  else { zustand.cam.x -= e.deltaX; begrenze(); kamera(); } }, {passive: false});
buehne.addEventListener('click', e => {
  if (bewegt > 5 && e.detail !== 0) return; // war ein Ziehen
  const z = e.target.closest('[data-zoom]'), kl = e.target.closest('[data-klapp]'), st = e.target.closest('[data-stufe]'), kopf = e.target.closest('[data-kopf]'), k = e.target.closest('.knoten'), inf = e.target.closest('[data-info]');
  if (inf) { zeigeBereich(inf.dataset.info); return; }
  if (kl) { klappe(kl.dataset.klapp); return; }
  if (z) { zumBereich(z.dataset.zoom); return; }
  if (st) { zeigeStufe(+st.dataset.stufe); return; }
  if (kopf) { zustand.zu.has(kopf.dataset.kopf) ? klappe(kopf.dataset.kopf) : zumBereich(kopf.dataset.kopf); return; }
  if (k) { if (zustand.zielwahl) zielGewaehlt(k.dataset.id); else waehle(k.dataset.id, {fliegen: true}); return; }
  if (e.target.closest('.zaehler')) { const b = e.target.closest('[data-bereich]'); if (b) zumBereich(b.dataset.bereich); return; }
  if (zustand.gewaehlt || !$('dossier').hidden) schliesseDossier();
});

function klappe(id) {
  zustand.zu.has(id) ? zustand.zu.delete(id) : zustand.zu.add(id);
  neuLayout(); zeige(`${BEREICH[id].titel} ${zustand.zu.has(id) ? 'zugeklappt' : 'aufgeklappt'}`);
}
function neuLayout() { layout(); setzePositionen(); markiere(); speicher.schreib('zu', [...zustand.zu].join(',')); }

/* ---------- Auswahl & Hervorhebung ---------- */
/* ---------- Spielerstand (nur im Spiel: Volk, Stufe, Forschung, Gebäude aus view) ---------- */
let S = null;
function hatStufe(id) { const k = K[id]; if (!S || !k) return null; if (k.typ === 'forschung') return S.forschung[id] || 0; if (k.typ === 'gebaeude') return S.gebaeude[id] || 0; return null; }
function erfuellt(x) {
  if (!S) return null;
  if (x.id.startsWith('stufe_')) return S.stufe >= +x.id.slice(6);
  const k = K[x.id], n = typeof x.n === 'number' && x.n > 0 ? x.n : 1; if (!k) return null;
  if (k.typ === 'forschung') return (S.forschung[x.id] || 0) >= n;
  if (k.typ === 'gebaeude') return ((x.art === 'heimat' ? S.heimat : S.gebaeude)[x.id] || 0) >= n;
  if (x.id === 'kz_bevoelkerung') return S.bevoelkerung >= n;
  if (x.id === 'kz_stabilitaet' && x.art === 'bedingung') return S.stabilitaet >= n;
  if (x.id === 'mission_kolonisieren' && n > 1) return S.kolonien >= n;
  return null;
}
function status(n) {
  if (!S || !n) return null;
  const lv = hatStufe(n.id); if (lv > 0) return {k: 'erreicht', lv};
  const fehlt = [], oder = {};
  if (S.stufe < n.stufe) fehlt.push({id: 'stufe_' + n.stufe, n: n.stufe, art: 'bedingung'});
  for (const x of vorausVon(n)) { if (!HART.has(x.art)) continue; const e = erfuellt(x); if (e === null) continue;
    if (x.oder) oder[x.oder] = oder[x.oder] || e; else if (!e) fehlt.push(x); }
  const offen = Object.entries(oder).filter(([, v]) => !v).map(([g]) => g);
  return fehlt.length || offen.length ? {k: 'gesperrt', fehlt, oder: offen} : {k: 'moeglich'};
}
function standAktualisieren() {
  S = OPT.spieler?.() || null;
  $('app').classList.toggle('mit-stand', !!S);
  welt.classList.toggle('stand-an', !!S && zustand.standAn !== false);
  for (const [id, el] of Object.entries(knotenEl)) {
    const st = status(K[id]);
    for (const k of ['erreicht', 'moeglich', 'gesperrt']) el.classList.toggle('st-' + k, st?.k === k);
    el.querySelector('.lvl').textContent = st?.k === 'erreicht' && MIT_STUFEN.has(K[id].typ) ? st.lv : '';
  }
  for (const el of W.querySelectorAll('.band')) el.classList.toggle('st-erreicht', !!S && S.stufe >= +el.querySelector('[data-stufe]').dataset.stufe);
  if (S && $('meinstand')) $('meinstand').querySelector('small').textContent = `${STUFE[S.stufe - 1]?.name || ''}`;
  if (PLAN) { const offen = !$('plan').hidden; PLAN = berechnePlan(PLAN.ziel); zeigePlan(); $('plan').hidden = !offen; markiere(); }
  if (zustand.gewaehlt) oeffneDossier(dossierKnoten(K[zustand.gewaehlt]), {behalten: true});
}
function standBox(n) {
  const st = status(n); if (!st) return '';
  const titel = st.k === 'erreicht' ? (MIT_STUFEN.has(n.typ) ? `Dein Stand: Stufe ${st.lv}` : 'Vorhanden')
    : st.k === 'moeglich' ? (EINHEIT.has(n.typ) || ['boden', 'rakete'].includes(n.typ) ? 'Jetzt baubar – alle Bedingungen erfüllt' : 'Jetzt möglich – alle Bedingungen erfüllt') : 'Noch gesperrt';
  const fehlt = st.k === 'gesperrt' ? [...st.fehlt.map(x => esc(name(x.id)) + (!x.id.startsWith('stufe_') && bedWert(x) ? ' ' + esc(bedWert(x)) : '')), ...st.oder.map(() => 'eine der Alternativen')] : [];
  const naechste = st.k === 'erreicht' && MIT_STUFEN.has(n.typ) && n.faktor && n.faktor !== 1 && !OPT.spiel?.info?.(n.id, n.typ) ? `<small>Nächste Stufe ${st.lv + 1} kostet:</small>${guterListe(kostenStufe(n, st.lv + 1))}` : '';
  return `<div class="d-stand st-${st.k}"><b>${titel}</b>${fehlt.length ? `<small>Es fehlt: ${fehlt.join(', ')}</small>` : ''}${naechste}</div>`;
}
// Im Spiel: echter Auftrag aus dem Dossier über OPT.spiel (techbaum-start.js → dieselben Handler wie die Kacheln der Bauseiten).
function aktionBlock(n) {
  const i = OPT.spiel?.info?.(n.id, n.typ); if (!i) return '';
  const imSpiel = `<button type="button" class="d-zweit" data-spiel-oeffnen="${esc(n.id)}">${esc(i.seiteText)}</button>`;
  if (i.auftrag) return `<div class="d-aktion laeuft"><b>${esc(i.auftrag)}</b><small>${esc(i.ort)}</small>${imSpiel}</div>`;
  if (i.gesperrt.length) {
    const st = status(n), ids = st?.k === 'gesperrt' ? st.fehlt.map(x => x.id).filter(id => K[id] || id.startsWith('stufe_')) : [];
    return `<div class="d-aktion gesperrt"><b>Gesperrt: braucht ${esc(i.gesperrt.join(' · '))}</b>${ids.length ? `<small>Antippen springt zur Bedingung:</small><div class="chips">${ids.map(id => chip(id)).join('')}</div>` : ''}${imSpiel}</div>`;
  }
  const aus = !!i.lesen || !i.bezahlbar && !i.vormerken;
  const menge = i.menge ? `<label class="d-menge"><span>Anzahl</span><input id="d-menge" type="number" min="1" max="${Math.max(1, i.max || 1)}" value="1" inputmode="numeric"></label>` : '';
  const hinweis = i.lesen ? i.lesen : !i.bezahlbar ? (i.vormerken ? 'Rohstoffe fehlen noch: der Auftrag wird vorgemerkt und startet automatisch.' : 'Rohstoffe reichen noch nicht.') : i.hinweis || '';
  return `<div class="d-aktion bereit"><div class="d-aktion-zeile">${menge}<button type="button" class="d-bauen" data-spiel-bauen="${esc(n.id)}"${aus ? ' disabled' : ''}>${esc(i.wort)}</button></div>
    ${guterListe(i.kosten)}${i.wirkung || ''}<small>${esc(i.ort)}${hinweis ? ' · ' + esc(hinweis) : ''}</small>${imSpiel}</div>`;
}
// Kräfteatlas: gemessene Konter dieses Eintrags (im Spiel OPT.atlas aus techbaum-start.js, auf der Website die Atlas-Seite).
function atlasBlock(n) {
  if (!OPT.atlas?.kennt?.(n.id)) return '';
  const was = n.id === 'weltentruemmer' ? 'Gegen gleichen Wert gemessen' : n.typ === 'rakete' ? 'Was eine Rakete je Ziel zerstört' : 'Konter und Gefechte';
  return `<button type="button" class="d-zweit d-atlas" data-atlas="${esc(n.id)}"><b>Kräfteatlas</b><small>${was} – mit der Kampfengine gemessen</small></button>`;
}
async function spielBauen(knopf) {
  const id = knopf.dataset.spielBauen, menge = Math.max(1, Math.floor(+($('d-menge')?.value || 1)));
  knopf.disabled = true; knopf.classList.add('laeuft'); knopf.textContent = 'Wird beauftragt …';
  let r; try { r = await OPT.spiel.ausfuehren(id, K[id].typ, menge); } catch (e) { r = {ok: false, text: e.message}; }
  zeige(r.text || (r.ok ? 'Auftrag erteilt' : 'Nicht möglich'), {fehler: !r.ok, ms: 4500});
  standAktualisieren();
}
function markiere() {
  const s = zustand.gewaehlt && K[zustand.gewaehlt] ? zustand.gewaehlt : null;
  VOR = s ? vorfahren(s) : new Set(); NACH = s ? nachfahren(s) : new Set();
  welt.classList.toggle('fokus', !!s);
  const boost = new Set(s ? [...(K[s].verstaerkt || []).map(v => v.id), ...(verstaerktVon[s] || []).map(v => v.id)] : []);
  const alternativ = new Set(s ? [...vorausVon(K[s]).filter(v => v.oder).map(v => v.id), ...(nach[s] || []).filter(v => v.oder).map(v => v.id)] : []);
  for (const [id, el] of Object.entries(knotenEl)) {
    el.classList.toggle('gewaehlt', id === s); el.classList.toggle('vor', VOR.has(id)); el.classList.toggle('nach', NACH.has(id) && !VOR.has(id));
    el.classList.toggle('boost', (boost.has(id) || alternativ.has(id)) && !VOR.has(id) && !NACH.has(id));
    el.classList.toggle('geplant', !!PLAN?.bedarf.has(id));
    el.classList.toggle('gesperrt', !!zustand.meine && K[id].stufe > zustand.meine);
  }
  zeichneAktiv();
}
function stufenStatus() { for (const el of W.querySelectorAll('.band')) el.classList.toggle('gesperrt', !!zustand.meine && +el.querySelector('[data-stufe]').dataset.stufe > zustand.meine); }
function waehle(id, {fliegen = false} = {}) {
  const n = K[id]; if (!n) return;
  if (zustand.zu.has(n.bereich)) { zustand.zu.delete(n.bereich); neuLayout(); }
  zustand.gewaehlt = id; markiere(); oeffneDossier(dossierKnoten(n));
  schreibHash();
  if (fliegen) zuKnoten(id);
}
function schliesseDossier() { zustand.gewaehlt = null; $('dossier').hidden = true; markiere(); schreibHash(); kamera(); }
function schreibHash() { if (EINGEBETTET) return; const h = new URLSearchParams({volk: zustand.volk}); if (zustand.gewaehlt) h.set('id', zustand.gewaehlt); history.replaceState(null, '', '#' + h); }

/* ---------- Kostenrechnung ---------- */
const STUFEN_MAX = 10;
function kostenStufe(n, l) {
  if (n.typ === 'gebaeude' && n.ausbau?.[l - 1]) return n.ausbau[l - 1].kosten;
  const p = Math.pow(n.faktor || 1, l - 1), aus = {};
  for (const [g, v] of Object.entries(n.kosten || {})) aus[g] = Math.floor(v * p);
  return aus;
}
function einheitKosten(n, v = zustand.volk) {
  const f = n.einheitklasse === 'kolonie' ? D.voelker[v].regel.kolonieschiff : 1, aus = {};
  for (const [g, x] of Object.entries(n.kosten || {})) aus[g] = ['erz', 'kristall', 'deuterium', 'legierung', 'elektronik'].includes(g) ? Math.floor(x * f) : x;
  return aus;
}
const summe = (a, b) => { for (const [g, v] of Object.entries(b || {})) a[g] = (a[g] || 0) + v; return a; };
function bisStufe(n, bis, von = 1) { const s = {}; let fp = 0; for (let l = von; l <= bis; l++) { summe(s, kostenStufe(n, l)); if (n.fp) fp += n.fp * Math.pow(n.faktor, l - 1); } return {kosten: s, fp}; }

/* ---------- Dossier-Bausteine ---------- */
const wertNamen = {angriff:'Angriff', schild:'Schild', struktur:'Struktur', waffenwert:'Waffenwert/Bewohner', ruestwert:'Rüstwert/Bewohner', tempo:'Tempo', ladung:'Fracht', verbrauch:'Treibstoff', besatzung:'Besatzung', schaden:'Schaden',
  transportplaetze:'Transportplätze', fertigungsplaetze:'Fertigungsplätze', personal:'Personal', dauer_sekunden:'Ausbildung', bauzeit_sekunden:'Bauzeit', plaetze:'Siloplätze',
  reichweite_je_silo:'Reichweite/Silo', abfangbedarf:'Abfangraketen nötig', arbeiter:'Arbeiter', fachkraefte:'Fachkräfte', energie:'Energie', ertrag:'Ertrag', max_stufe:'Höchststufe',
  mitglieder_min:'Mitglieder', bau_tage:'Bauzeit (Tage)', anschluesse:'Anschlüsse', sonnenfaktor:'Solarfaktor', strombad:'Strombad'};
const volkFeldFuerWert = {angriff: 'waffen', schaden: 'waffen', struktur: 'panzerung', ladung: 'ladung'};
const MAXIMA = {};
for (const n of Object.values(K)) for (const [k, v] of Object.entries(n.werte || {})) if (typeof v === 'number') { const g = (EINHEIT.has(n.typ) ? 'einheit' : n.typ) + ':' + k; MAXIMA[g] = Math.max(MAXIMA[g] || 0, v); }
const zeit = s => s >= 3600 ? fmt(s / 3600) + ' h' : s >= 60 ? fmt(s / 60) + ' min' : fmt(s) + ' s';

function abschnitt(titel, innen, zusatz = '', offen = true) {
  if (!innen) return '';
  return `<details class="d-abschnitt"${offen ? ' open' : ''}><summary><h3>${titel}</h3>${zusatz ? `<small>${zusatz}</small>` : ''}</summary><div class="innen">${innen}</div></details>`;
}
function guterListe(k, basis) {
  const e = Object.entries(k || {}).filter(([, v]) => v); if (!e.length) return '';
  return `<div class="kosten">${e.map(([g, v]) => `<div class="gut">${res(g) ? `<img alt="" src="${esc(res(g))}">` : ''}<span>${esc(gutName(g))}</span><b>${kurz(v)}</b>${basis && basis[g] !== v ? `<del>${kurz(basis[g])}</del>` : ''}</div>`).join('')}</div>`;
}
function chip(id, n, fa) {
  const st = id.startsWith('stufe_');
  return `<button type="button" class="chip${st ? ' stufe' : ''}" data-geh="${esc(id)}" data-st="--cf:${fa || farbe(id)}">${st ? '' : `<img alt="" src="${esc(bild(id))}" loading="lazy">`}<span>${esc(name(id))}</span>${n ? `<b>${n}</b>` : ''}</button>`;
}
function werteBlock(n) {
  const r = D.voelker[zustand.volk].regel, w = n.werte || {}, gruppe = EINHEIT.has(n.typ) ? 'einheit' : n.typ;
  const zeilen = Object.entries(w).filter(([, v]) => v != null && v !== 0 && typeof v === 'number').map(([k, v]) => {
    const feld = volkFeldFuerWert[k], f = feld && (EINHEIT.has(n.typ) || n.typ === 'rakete') ? r[feld] : 1;
    // Kleine Bruchwerte (Miliz: Angriff 0,3 je Bewohner) nicht auf 0 abrunden.
    const wert = Number.isInteger(v) || v >= 10 ? Math.floor(v * f + 1e-9) : Math.round(v * f * 100) / 100, max = MAXIMA[gruppe + ':' + k] || v;
    const breite = x => Math.max(3, Math.log1p(x) / Math.log1p(max * (f > 1 ? f : 1)) * 100);
    const anzeige = /sekunden/.test(k) ? zeit(v) : k === 'sonnenfaktor' || k === 'strombad' ? mal(v) : kurz(wert);
    return `<div class="wert"><span>${wertNamen[k] || k}</span><span class="balken">${f !== 1 ? `<i class="basis" data-st="width:${breite(v)}%"></i>` : ''}<i data-st="width:${breite(wert)}%"></i></span><b>${anzeige}${f !== 1 ? `<em class="${f > 1 ? 'plus' : 'minus'}">${prozent(f)}</em>` : ''}</b></div>`; });
  if (n.wert && (EINHEIT.has(n.typ) || n.typ === 'boden' || n.typ === 'rakete')) zeilen.push(`<div class="wert"><span>Punktwert</span><span class="balken"></span><b>${kurz(n.wert)}</b></div>`);
  return zeilen.length ? `<div class="werte">${zeilen.join('')}</div>` : '';
}
function stufenTabelle(n) {
  if (!MIT_STUFEN.has(n.typ) || !n.faktor || n.faktor === 1) return '';
  const gueter = Object.keys(n.kosten || {}).filter(g => n.kosten[g]);
  const max = n.typ === 'mond' && n.werte?.max_stufe ? n.werte.max_stufe : STUFEN_MAX;
  const rf = D.voelker[zustand.volk].regel.forschung;
  const zeilen = []; let sum = {};
  for (let l = 1; l <= max; l++) { const k = kostenStufe(n, l), a = n.ausbau?.[l - 1]; sum = summe({...sum}, k);
    zeilen.push(`<tr><td>${l}</td>${gueter.map(g => `<td>${kurz(k[g] || 0)}</td>`).join('')}${n.fp ? `<td>${kurz(n.fp * Math.pow(n.faktor, l - 1))}</td>` : ''}${a ? `<td>${fmt(a.energie, 0)}</td><td>${fmt(a.arbeiter, 0)}</td>` : ''}<td class="summe">${kurz(Object.values(sum).reduce((x, y) => x + y, 0))}</td></tr>`); }
  return `<div class="tabelle-roll"><table class="tabelle"><thead><tr><th>Stufe</th>${gueter.map(g => `<th>${esc(gutName(g))}</th>`).join('')}${n.fp ? '<th>FP</th>' : ''}${n.ausbau?.length ? '<th>Energie</th><th>Arbeiter</th>' : ''}<th>Σ Güter</th></tr></thead><tbody>${zeilen.join('')}</tbody></table></div>${n.fp && rf !== 1 ? `<p class="vp-hinweis">${esc(D.voelker[zustand.volk].name)} forschen ${rf > 1 ? 'schneller' : 'langsamer'}: Forschungspunkte entstehen ${mal(rf)} so schnell.</p>` : ''}`;
}
function absaetze(t) { const s = String(t).split(/(?<=[.!?“])\s+(?=[A-ZÄÖÜ„])/), aus = []; for (let i = 0; i < s.length; i += 3) aus.push(`<p>${esc(s.slice(i, i + 3).join(' '))}</p>`); return aus.join(''); }
function archiv(n) {
  const a = n.archiv || {}, v = zustand.volk, reihe = [v, ...Object.keys(D.voelker).filter(x => x !== v)].filter(x => a[x]);
  const teile = [];
  if (a.technik) teile.push(`<details class="akte"><summary>Technisches Dossier</summary>${absaetze(a.technik)}</details>`);
  if (a.geschichte) teile.push(`<details class="akte"><summary>Chronik</summary>${absaetze(a.geschichte)}</details>`);
  for (const x of reihe) teile.push(`<details class="akte" data-st="--af:${D.voelker[x].farbe}"${x === v ? ' open' : ''}><summary>${esc(D.voelker[x].name)}-Akte${x === v ? ' <small>gewähltes Volk</small>' : ''}</summary>${absaetze(a[x])}</details>`);
  return teile.join('');
}
function regeln(ids) {
  return (ids || []).filter(id => D.regelabschnitte[id]).map(id => `<details class="akte regel"><summary>${esc(D.regelabschnitte[id].name)}</summary><div class="regeltext">${D.regelabschnitte[id].text.split('\n').map(z => z.includes(' | ') ? `<p class="mono">${esc(z)}</p>` : absaetze(z)).join('')}</div></details>`).join('');
}
function bedWert(x) {
  const k = K[x.id], n = x.n;
  if (typeof n !== 'number' || n <= 0) return '';
  if (!k) return String(n);
  if (MIT_STUFEN.has(k.typ)) return 'Stufe ' + n;
  if (x.id === 'kz_stabilitaet' || x.id === 'gut_konsumgut' && x.art === 'bedingung') return (x.id === 'gut_konsumgut' ? n + ' %' : '≥ ' + fmt(n));
  if (k.typ === 'kennzahl' || k.typ === 'gut' || k.typ === 'bauteil') return n === 1 && x.art !== 'gut' ? '' : kurz(n);
  if (EINHEIT.has(k.typ) || k.typ === 'boden' || k.typ === 'mission' || k.typ === 'diplomatie') return n > 1 ? kurz(n) + '×' : '';
  return n > 1 ? kurz(n) : '';
}
function bedZeile(x, volk) {
  const e = erfuellt(x), mark = e === null ? '' : e ? ' ok' : ' fehlt';
  return `<button type="button" class="bed${volk ? ' volk' : ''}${mark}"${e === null ? '' : ` aria-label="${esc(name(x.id))}: ${e ? 'erfüllt' : 'fehlt noch'}"`} data-geh="${esc(x.id)}" data-st="--cf:${farbe(x.id)}"><img alt="" src="${esc(bild(x.id))}" loading="lazy"><span><b>${esc(name(x.id))}</b><small>${esc(x.text || ORT[x.art] || TYP[K[x.id]?.typ] || '')}</small></span><em>${esc(bedWert(x))}</em></button>`;
}
function bedingungen(n, stufeNr) {
  const v = zustand.volk, V = D.voelker[v], basis = n.voraus || [], volkV = n.voraus_volk?.[v] || [];
  const gruppen = {}, einzeln = {hart: [], gut: [], einheit: [], bedingung: [], weich: []};
  for (const x of basis) {
    if (x.oder) (gruppen[x.oder] ||= []).push(x);
    else if (['lokal', 'heimat', 'reich', 'mond'].includes(x.art)) einzeln.hart.push(x);
    else if (einzeln[x.art]) einzeln[x.art].push(x);
    else einzeln.weich.push(x);
  }
  const teile = [];
  if (stufeNr > 1) teile.push(`<div class="bed-gruppe"><h4>Zivilisationsstufe</h4><div class="beds">${bedZeile({id: 'stufe_' + stufeNr, art: 'bedingung', text: 'mindestens erreicht'})}</div></div>`);
  const block = (titel, liste) => liste.length ? `<div class="bed-gruppe"><h4>${titel}</h4><div class="beds">${liste.map(x => bedZeile(x)).join('')}</div></div>` : '';
  teile.push(block('Gebäude & Forschung', einzeln.hart), block('Einheiten', einzeln.einheit), block('Güter', einzeln.gut), block('Weitere Bedingungen', einzeln.bedingung));
  for (const [g, liste] of Object.entries(gruppen)) teile.push(`<div class="bed-gruppe oder"><h4>Eine davon genügt</h4><div class="beds">${liste.map(x => bedZeile(x)).join('')}</div></div>`);
  const volkText = n.bedingungen_volk?.[v] || [];
  if (volkV.length || volkText.length) teile.push(`<div class="bed-gruppe volk" data-st="--vf:${V.farbe}"><h4>Nur für ${esc(V.name)}</h4>${volkV.length ? `<div class="beds">${volkV.map(x => bedZeile(x, true)).join('')}</div>` : ''}${volkText.length ? `<ul class="bed-text">${volkText.map(t => `<li>${esc(t)}</li>`).join('')}</ul>` : ''}</div>`);
  if (n.bedingungen?.length) teile.push(`<div class="bed-gruppe"><h4>Regeln</h4><ul class="bed-text">${n.bedingungen.map(t => `<li>${esc(t)}</li>`).join('')}</ul></div>`);
  if (einzeln.weich.length) teile.push(`<details class="vt-mehr"><summary>Versorgung & Zusammenhänge (${einzeln.weich.length})</summary><div class="beds">${einzeln.weich.map(x => bedZeile(x)).join('')}</div></details>`);
  const html = teile.filter(Boolean).join('');
  return html || '<p class="vp-hinweis">Keine Voraussetzungen – von Beginn an verfügbar.</p>';
}
// Weltenkrümmer im Spiel: aktuelle 3D-Verwandlung und Mondzerstörungsfilm aus web-client/weltenkruemmer.js.
function wkKnoepfe(n) {
  if (!window.Weltenkruemmer?.oeffnen || !(['weltentruemmer', 'mission_mond_zerstoeren', 'mond'].includes(n.id) || n.typ === 'mondtyp')) return '';
  const mond = n.typ === 'mondtyp' && n.id !== 'mond' ? n.id.slice(8) : 'krater_grau';
  return `<div class="knopfzeile wk"><button type="button" data-wk="3d">3D-Verwandlung ansehen</button><button type="button" data-wk="film" data-mond="${esc(mond)}">Film: ${n.typ === 'mondtyp' && n.id !== 'mond' ? esc(n.name) + ' zerstören' : 'Mond zerstören'}</button></div>`;
}
function bildwahl(n) {
  const v = zustand.volk, opt = [['Motiv', bild(n.id)]];
  if (n.blaupause && Object.keys(n.blaupause).length) opt.push(['Blaupause', bildVon(n.blaupause, v)]);
  if (n.schnitt && Object.keys(n.schnitt).length) opt.push(['Schnitt', bildVon(n.schnitt, v)]);
  if (n.explosion && Object.keys(n.explosion).length) opt.push(['Explosion', bildVon(n.explosion, v)]);
  for (const [k, b] of Object.entries(n.varianten || {})) if (Object.keys(b).length) opt.push([k.replace(/_/g, ' '), bildVon(b, v)]);
  return opt.length > 1 ? `<div class="bildwahl" role="tablist">${opt.map(([t, u], i) => `<button type="button" role="tab" aria-selected="${i === 0}" data-bild="${esc(u)}">${esc(t)}</button>`).join('')}</div>` : '';
}
function habitatTabelle(n) {
  if (!n.habitat) return '';
  const [von, bis] = n.bahnen, plaetze = []; for (let p = von; p <= bis; p++) plaetze.push(p);
  const farbeF = f => f >= 1 ? 'vt-sg' : f >= .9 ? 'vt-g' : f >= .75 ? 'vt-n' : f >= .6 ? 'vt-s' : 'vt-ss';
  return `<div class="tabelle-roll"><table class="tabelle hab"><thead><tr><th>Volk</th>${plaetze.map(p => `<th>Bahn ${p}</th>`).join('')}</tr></thead><tbody>${Object.entries(n.habitat).map(([v, reihe]) =>
    `<tr class="${v === zustand.volk ? 'aktiv' : ''}"><td data-st="color:${D.voelker[v].farbe}">${esc(D.voelker[v].name)}${D.voelker[v].habitat.heimat >= von && D.voelker[v].habitat.heimat <= bis ? ' ⌂' : ''}</td>${reihe.map(b => `<td class="${farbeF(b.faktor)}">${fmt(b.faktor, 2)}${b.terraforming ? `<small>TF ${b.terraforming}</small>` : ''}</td>`).join('')}</tr>`).join('')}</tbody></table></div>
    <p class="vp-hinweis">Klimafaktor des Bevölkerungswachstums je Bahn (1,0 = Heimatklima). TF = nötige Terraforming-Stufe zum Besiedeln. ⌂ = Heimatbahn liegt in dieser Zone.</p>`;
}
function dossierKnoten(n) {
  const b = BEREICH[n.bereich], v = zustand.volk, V = D.voelker[v], eff = volkEffekte(n);
  const eigenesBild = !!n.bild?.[v] && n.bild[v] !== n.bild._;
  const flaeche = ['gebaeude', 'mond', 'wunder', 'forschung', 'zone', 'diplomatie', 'vertrag', 'rolle', 'himmel'].includes(n.typ);
  const kosten = EINHEIT.has(n.typ) ? einheitKosten(n) : n.kosten;
  const frei = (nach[n.id] || []).map(x => chip(x.id, typeof x.n === 'number' && x.n > 1 ? (K[n.id] && MIT_STUFEN.has(K[n.id].typ) ? '≥' + x.n : kurz(x.n)) : '')).join('');
  const verst = (n.verstaerkt || []).map(x => `<button type="button" class="kchip" data-geh="${esc(x.id)}"><img alt="" src="${esc(bild(x.id))}" loading="lazy"><span><b>${esc(name(x.id))}</b><small>${esc(x.text)}</small></span></button>`).join('');
  const durch = (verstaerktVon[n.id] || []).map(x => `<button type="button" class="kchip" data-geh="${esc(x.id)}"><img alt="" src="${esc(bild(x.id))}" loading="lazy"><span><b>${esc(name(x.id))}</b><small>${esc(x.text)}</small></span></button>`).join('');
  const t = [['Wozu', n.zweck], ['Stärke', n.staerke], ['Grenzen', n.nische]].filter(([, x]) => x);
  const lore = n.lore || {};
  const volkText = eff.length
    ? `<ul class="vp-liste">${eff.map(e => `<li class="${e.gut ? 'plus' : 'minus'}"><span>${esc(e.text)}</span><b>${effektZahl(e)}</b></li>`).join('')}</ul>`
    : `<p class="vp-hinweis">Keine Regeländerung bei den ${esc(V.name)}${eigenesBild ? ' – eigenes Volksmotiv' : ''}.</p>`;
  const kampf = Kampf.abschnitte(n, v).map(([titel, html, zusatz], i) => abschnitt(titel, html, zusatz || '', i < 4)).join('');
  const aw = (n.antriebswechsel || []).length ? `<ol class="sf-liste">${n.antriebswechsel.map(a => `<li data-geh="${esc(a.forschung)}"><img alt="" src="${esc(bild(a.forschung))}"><span><b>${esc(name(a.forschung))} ${a.stufe}</b><small>wechselt auf ${esc(name(a.antrieb))}</small></span><em>${kurz(a.tempo)}</em></li>`).join('')}</ol><p class="vp-hinweis">Ab der genannten Stufe gelten neuer Antrieb, Grundtempo und Verbrauch; Startantrieb: ${esc(name(n.antrieb))}.</p>` : '';
  const anteile = n.anteile ? `<div class="tabelle-roll"><table class="tabelle"><thead><tr><th>Epoche</th><th>Anteil je Gut</th></tr></thead><tbody>${n.anteile.map(z => `<tr><td>${z.tage} Tage</td><td>${kurz(z.anteil)}</td></tr>`).join('')}</tbody></table></div>` : '';
  const gewichte = n.gewichte ? `<div class="kp-zahlen">${Object.entries(n.gewichte).map(([z, w]) => `<div><b>${w}</b><span>${esc(name('zone_' + z))}</span></div>`).join('')}</div><p class="vp-hinweis">Relative Häufigkeit nach Zone des Planeten.</p>` : '';
  const held = `<div class="d-held${flaeche ? ' flaeche' : ''}${['planet', 'mondtyp'].includes(n.typ) ? ' kugel' : ''}"><img id="d-bild" alt="${esc(n.name)}" src="${esc(bild(n.id))}">${eigenesBild ? `<span class="d-volk">${V.emblem ? `<img alt="" src="${esc(BASIS + V.emblem)}">` : ''}${esc(V.name)}-Motiv</span>` : ''}</div>`;
  const titel = `<div class="d-titel"><small>${esc(b.titel)} · ${esc(n.klasse || TYP[n.typ])}</small><h2>${escT(n.name)}</h2></div>`;
  const werte = abschnitt('Werte', werteBlock(n), EINHEIT.has(n.typ) ? 'Grundwerte mit Volksbonus' : '') + kampf;
  const bed = abschnitt('Bedingungen', bedingungen(n, n.stufe), 'alles, was erfüllt sein muss'), volk = abschnitt('Volk · ' + esc(V.name), volkText);
  const kost = abschnitt(EINHEIT.has(n.typ) || ['boden', 'rakete', 'bauteil'].includes(n.typ) ? 'Baukosten' : n.typ === 'wunder' ? 'Anteil je Mitglied (365 Tage)' : 'Kosten Stufe 1', guterListe(kosten, n.kosten), n.typ === 'boden' ? 'je Einheit' : '');
  const wirk = n.wirkung && n.wirkung !== n.rolle ? abschnitt('Wirkung', `<p data-st="margin:0">${esc(n.wirkung)}</p>`) : '';
  // Handy: Bild klein neben dem Titel, darunter Stand und Bauen/Erforschen, dann Kosten, Wirkung, Bedingungen (im halben Blatt sichtbar)
  const mobil = istMobil();
  return {farbe: b.farbe, html: (mobil ? `
    <div class="d-oben">${held}${titel}</div>
    ${bildwahl(n)}
    ${wkKnoepfe(n)}
    ${standBox(n)}
    ${aktionBlock(n)}
    ${atlasBlock(n)}
    ${n.rolle ? `<p class="d-rolle">${esc(n.rolle)}</p>` : ''}
    ${kost}${wirk}${bed}${werte}${volk}` : `
    ${held}
    ${bildwahl(n)}
    ${wkKnoepfe(n)}
    ${titel}
    ${standBox(n)}
    ${aktionBlock(n)}
    ${atlasBlock(n)}
    ${n.rolle ? `<p class="d-rolle">${esc(n.rolle)}</p>` : ''}
    ${werte}${bed}${volk}${kost}${wirk}`) + `
    ${abschnitt('Klima je Volk', habitatTabelle(n), 'Bevölkerungswachstum')}
    ${abschnitt('Vorkommen', gewichte)}
    ${abschnitt('Anteil nach Epochenlänge', anteile)}
    ${abschnitt('Antriebswechsel durch Forschung', aw)}
    ${abschnitt('Verstärkt', verst ? `<div class="kchips">${verst}</div>` : '', '', false)}
    ${abschnitt('Wird verstärkt durch', durch ? `<div class="kchips">${durch}</div>` : '', '', false)}
    ${abschnitt(['gut', 'bauteil', 'kennzahl'].includes(n.typ) ? 'Gebraucht von' : 'Schaltet frei / wird gebraucht von', frei ? `<div class="chips">${frei}</div>` : '', `${(nach[n.id] || []).length} Einträge`, false)}
    ${abschnitt('Ausbau', stufenTabelle(n), n.typ === 'forschung' ? 'Kosten und Forschungspunkte je Stufe' : 'Kosten je Stufe', false)}
    ${abschnitt('Einsatz', t.length ? `<dl class="texte">${t.map(([k, x]) => `<dt>${k}</dt><dd>${esc(x)}</dd>`).join('')}</dl>` : '')}
    ${abschnitt('Lore', (lore.allgemein ? `<p class="lore">${esc(lore.allgemein)}</p>` : '') + (lore[v] ? `<p class="lore" data-st="border-color:${V.farbe}">${esc(lore[v])}</p>` : ''), '', false)}
    ${abschnitt('Archiv', archiv(n), '', false)}
    ${abschnitt('Regeltexte', regeln(n.regeln), 'Volltext aus dem Regelwerk', false)}
    <button type="button" class="halten${PLAN?.ziel === n.id ? ' fertig' : ''}" id="halten" data-ziel="${esc(n.id)}"><i></i><strong>${PLAN?.ziel === n.id ? 'WEG GEPLANT ✓' : mobil ? 'WEG HIERHER PLANEN' : 'WEG PLANEN'}</strong><small>${PLAN?.ziel === n.id ? (mobil ? 'antippen zum Aufheben' : 'erneut halten zum Aufheben') : mobil ? 'antippen – alle Schritte, Bedingungen und Gesamtkosten' : 'halten (1,2 s) – alle Schritte, Bedingungen und Gesamtkosten'}</small></button>
    <div class="knopfzeile"><button type="button" data-pfad="${esc(n.id)}">Ganzen Pfad zeigen</button><button type="button" data-zoom="${n.bereich}">Baum „${esc(b.titel)}“</button></div>`};
}
function zeigeStufe(nr) {
  const st = STUFE[nr - 1], frei = Object.values(K).filter(n => n.stufe === nr);
  zustand.gewaehlt = null; markiere();
  const bed = nr > 1 ? bedingungen({voraus: st.voraus, bedingungen: st.bedingungen}, nr - 1 > 1 ? nr - 1 : 0) : `<ul class="bed-text">${st.bedingungen.map(t => `<li>${esc(t)}</li>`).join('')}</ul>`;
  oeffneDossier({farbe: 'var(--gold)', html: `
    <div class="${istMobil() ? 'd-oben' : 'd-stufe-kopf'}"><div class="d-held flaeche"><img alt="${esc(st.name)}" src="${esc(bild('stufe_' + nr))}"></div>
    <div class="d-titel"><small>Zivilisationsstufe</small><h2>${esc(st.name)}</h2></div></div>
    <p class="d-rolle">${nr === 1 ? 'Jedes Reich beginnt hier.' : `Aufstieg schaltet ${frei.length} Einträge frei.`}</p>
    ${abschnitt('Bedingungen', bed, 'alles, was erfüllt sein muss')}
    ${abschnitt('Aufstiegskosten', guterListe(st.kosten))}
    ${abschnitt('Schaltet frei', `<div class="chips">${frei.map(n => chip(n.id)).join('')}</div>`, `${frei.length} Einträge`)}
    ${abschnitt('Regeltexte', regeln(['stufen']), '', false)}
    ${nr > 1 ? `<button type="button" class="halten${PLAN?.ziel === 'stufe_' + nr ? ' fertig' : ''}" id="halten" data-ziel="stufe_${nr}"><i></i><strong>${PLAN?.ziel === 'stufe_' + nr ? 'WEG GEPLANT ✓' : 'AUFSTIEG PLANEN'}</strong><small>${istMobil() ? 'antippen' : 'halten (1,2 s)'}</small></button>` : ''}`});
  const band = LAY.baender[nr - 1]; if (istMobil()) zuBand(nr); else fliege(passend({x: 0, y: band.y - 10, w: LAY.w, h: band.h + 20}, 0.9));
}
function zeigeBereich(id) {
  const b = BEREICH[id];
  zustand.gewaehlt = null; markiere();
  const bahnen = b.bahnen.map(l => `<div class="bed-gruppe"><h4>${esc(l.titel)} · ${l.knoten.length}</h4><div class="chips">${l.knoten.map(k => chip(k)).join('')}</div></div>`).join('');
  const matrix = ['flotte', 'verteidigung', 'raumfahrt', 'mond'].includes(id) ? kontermatrix(Object.values(K).filter(n => EINHEIT.has(n.typ) && (n.werte?.struktur || 0) > 0))
    : id === 'boden' ? kontermatrix(Object.values(K).filter(n => n.typ === 'boden' && n.id !== 'garnison')) : '';
  oeffneDossier({farbe: b.farbe, html: `
    <div class="d-titel" data-st="margin-top:12px"><small>Bereich</small><h2>${esc(b.titel)}</h2></div>
    ${abschnitt('Inhalt', bahnen, `${b.bahnen.reduce((s, l) => s + l.knoten.length, 0)} Einträge`)}
    ${abschnitt('Kontermatrix', matrix, 'Zeile greift an, Spalte wird getroffen', false)}
    ${abschnitt('Regeltexte', regeln(b.regeln), 'Volltext aus dem Regelwerk', false)}
    <div class="knopfzeile"><button type="button" data-zoom="${id}">In den Baum zoomen</button></div>`});
}
function kontermatrix(einheiten) {
  einheiten.sort((a, c) => BEREICH[a.bereich].i - BEREICH[c.bereich].i || a.stufe - c.stufe || a.name.localeCompare(c.name));
  const vA = D.voelker[zustand.volk].regel;
  const kopf = einheiten.map(B => `<th><button type="button" data-geh="${esc(B.id)}" title="${esc(B.name)}"><img alt="${esc(B.name)}" src="${esc(bild(B.id))}" loading="lazy"></button></th>`).join('');
  const zeilen = einheiten.map(A => `<tr><th><button type="button" data-geh="${esc(A.id)}" title="${esc(A.name)}"><img alt="" src="${esc(bild(A.id))}" loading="lazy"><span>${esc(A.name)}</span></button></th>${einheiten.map(B => {
    if (A === B) return '<td class="km-selbst"></td>';
    const g = Kampf.gefecht(A, B, A.typ === 'boden' ? null : vA, zustand.volk);
    return `<td class="vt-${g.urteil.k}" title="${esc(A.name)} gegen ${esc(B.name)}: ${g.urteil.t} – ${esc(B.name)} verliert ${Math.round(g.verlustB * 100)} %, ${esc(A.name)} verliert ${Math.round(g.verlustA * 100)} %"></td>`; }).join('')}</tr>`).join('');
  return `<div class="km-roll"><table class="kmatrix-t"><thead><tr><th></th>${kopf}</tr></thead><tbody>${zeilen}</tbody></table></div>
    <ul class="gf-legende">${Kampf.URTEILE.map(s => `<li class="vt-${s.k}"><i></i><b>${s.t}</b><small>${s.k === 'x' ? 'keine Seite richtet viel aus' : 'Zeile ' + s.lang}</small></li>`).join('')}</ul>
    <p class="vp-hinweis">Jedes Feld ist ein Gefecht bei gleichem Rohstoffwert: die <b>Zeile</b> gegen die <b>Spalte</b>. Grün = die Zeile gewinnt, Rot = die Spalte gewinnt. Volk ${esc(D.voelker[zustand.volk].name)} auf Seite der Zeile. Tippe ein Bild für das Dossier.</p>`;
}
function zeigeRegeln() {
  zustand.gewaehlt = null; markiere();
  oeffneDossier({farbe: 'var(--cyan)', html: `
    <div class="d-titel" data-st="margin-top:12px"><small>Regelwerk des Kerns</small><h2>Alle Regeln</h2></div>
    <label class="suche regelsuche"><span class="sr">Regeln durchsuchen</span><input id="regelsuche" type="search" placeholder="Regeln durchsuchen …"></label>
    <div id="regelliste">${regeln(Object.keys(D.regelabschnitte))}</div>`});
}
function oeffneDossier({farbe: f, html}, {behalten = false} = {}) {
  const d = $('dossier'), inhalt = $('dossier-inhalt'), y = inhalt.scrollTop; behalten &&= !d.hidden;
  d.style.setProperty('--bf', f); inhalt.innerHTML = html; inhalt.scrollTop = behalten ? y : 0;
  if (istMobil() && !$('plan').hidden) $('plan').hidden = true; // auf dem Handy immer nur ein Blatt
  const war = d.hidden; d.hidden = false; if (!behalten) d.classList.toggle('halb', istMobil()); if (war) kamera();
}
$('dossier-zu').addEventListener('click', schliesseDossier);
$('dossier-griff').addEventListener('click', () => $('dossier').classList.toggle('halb'));
$('dossier-inhalt').addEventListener('scroll', e => { if (e.target.scrollTop > 40) $('dossier').classList.remove('halb'); }, {passive: true});
$('dossier').addEventListener('click', e => {
  const sb = e.target.closest('[data-spiel-bauen]'); if (sb) { if (!sb.disabled) spielBauen(sb); return; }
  const so = e.target.closest('[data-spiel-oeffnen]'); if (so) { OPT.spiel?.oeffnen?.(so.dataset.spielOeffnen, K[so.dataset.spielOeffnen]?.typ); return; }
  const at = e.target.closest('[data-atlas]'); if (at) { OPT.atlas?.zeige?.(at.dataset.atlas); return; }
  const ha = e.target.closest('#halten'); if (ha) { if (istMobil()) planUmschalten(ha.dataset.ziel); return; }
  const wk = e.target.closest('[data-wk]'); if (wk) { window.Weltenkruemmer?.oeffnen(zustand.volk, true, wk.dataset.wk === 'film' ? {film: {mond: wk.dataset.mond}} : {}); return; }
  const bw = e.target.closest('[data-bild]'); if (bw) { $('d-bild').src = bw.dataset.bild; bw.parentElement.querySelectorAll('button').forEach(x => x.setAttribute('aria-selected', x === bw)); return; }
  const f = e.target.closest('[data-vtf]'); if (f) { const wert = f.dataset.vtf, box = f.closest('details');
    f.parentElement.querySelectorAll('button').forEach(x => x.setAttribute('aria-selected', x === f));
    const alle = f.closest('.gf-alle') || box; alle.querySelectorAll('.gf').forEach(z => { z.hidden = wert !== 'alle' && z.dataset.gruppe !== wert; });
    alle.querySelectorAll('.gf-gruppe').forEach(g => { const n = [...g.querySelectorAll('.gf')].filter(z => !z.hidden).length; g.hidden = !n; g.querySelector('summary small').textContent = n; }); return; }
  const g = e.target.closest('[data-geh]'); if (g) { const id = g.dataset.geh; id.startsWith('stufe_') ? zeigeStufe(+id.slice(6)) : waehle(id, {fliegen: true}); return; }
  const z = e.target.closest('[data-zoom]'); if (z) { zumBereich(z.dataset.zoom); return; }
  const p = e.target.closest('[data-pfad]'); if (p) { const ids = [p.dataset.pfad, ...vorfahren(p.dataset.pfad), ...nachfahren(p.dataset.pfad)]; passeAn(ids); }
});
$('dossier').addEventListener('input', e => {
  if (e.target.id !== 'regelsuche') return; const q = e.target.value.trim().toLocaleLowerCase('de');
  for (const d of W.querySelectorAll('#regelliste details')) { const treffer = !q || d.textContent.toLocaleLowerCase('de').includes(q); d.hidden = !treffer; d.open = !!q && treffer; }
});
function passeAn(ids) {
  const ps = ids.map(i => LAY.pos[i]).filter(Boolean); if (!ps.length) return;
  const x0 = Math.min(...ps.map(p => p.x)) - 30, y0 = Math.min(...ps.map(p => p.y)) - 40, x1 = Math.max(...ps.map(p => p.x)) + 126, y1 = Math.max(...ps.map(p => p.y)) + 150;
  fliege(passend({x: x0, y: y0, w: x1 - x0, h: y1 - y0}, 1.1));
}

/* ---------- Halten zum Bestätigen ---------- */
let haltTimer = null;
function haltStart(e) { const h = e.target.closest('#halten'); if (!h || h.classList.contains('laeuft') || istMobil() && e.type === 'pointerdown') return; e.preventDefault();
  h.classList.add('laeuft'); haltTimer = setTimeout(() => { h.classList.remove('laeuft'); navigator.vibrate?.(25); planUmschalten(h.dataset.ziel); }, 1200); }
function haltEnde() { clearTimeout(haltTimer); W.querySelector('#halten.laeuft')?.classList.remove('laeuft'); }
$('dossier').addEventListener('pointerdown', haltStart);
$('dossier').addEventListener('pointerup', haltEnde); $('dossier').addEventListener('pointerleave', haltEnde); $('dossier').addEventListener('pointercancel', haltEnde);
$('dossier').addEventListener('keydown', e => { if ((e.key === 'Enter' || e.key === ' ') && e.target.id === 'halten' && !e.repeat) haltStart(e); });
$('dossier').addEventListener('keyup', e => { if (e.key === 'Enter' || e.key === ' ') haltEnde(); });

/* ---------- Weg planen: harte Bedingungen, bei „eine davon“ die erste Möglichkeit ---------- */
function berechnePlan(ziel) {
  const bedarf = new Map(), stufen = new Set(), erledigt = new Set(), v = zustand.volk;
  const folge = n => { const gewaehlt = new Set(); return vorausVon(n, v).filter(x => HART.has(x.art) && (!x.oder || (!gewaehlt.has(x.oder) && gewaehlt.add(x.oder)))); };
  const stufe = s => { for (let t = 2; t <= s; t++) if (!stufen.has(t) && !(S && S.stufe >= t)) { stufen.add(t); for (const x of folge(STUFE[t - 1])) braucht(x.id, x.n); } };
  const braucht = (id, n) => { n = typeof n === 'number' && n > 0 ? n : 1; if ((bedarf.get(id) ?? 0) >= n || erledigt.has(id + ':' + n)) return;
    if (S && erfuellt({id, n, art: 'lokal'}) === true) { erledigt.add(id + ':' + n); return; }
    bedarf.set(id, n); const k = K[id]; if (!k) return;
    stufe(k.stufe); for (const x of folge(k)) braucht(x.id, x.n); };
  if (ziel.startsWith('stufe_')) stufe(+ziel.slice(6)); else braucht(ziel, 1);
  const gesamt = {}; let fp = 0;
  const schritte = [...bedarf].map(([id, n]) => { const k = K[id];
    const von = MIT_STUFEN.has(k.typ) && S ? (hatStufe(id) || 0) + 1 : 1;
    const r = MIT_STUFEN.has(k.typ) ? bisStufe(k, n, von) : EINHEIT.has(k.typ) ? {kosten: einheitKosten(k), fp: 0} : ['boden', 'rakete'].includes(k.typ) ? {kosten: k.kosten, fp: 0} : {kosten: {}, fp: 0};
    summe(gesamt, r.kosten); fp += r.fp; return {id, n, von, stufe: k.stufe, kosten: r.kosten, fp: r.fp}; });
  for (const s of stufen) summe(gesamt, STUFE[s - 1].kosten);
  return {ziel, bedarf, stufen, schritte, gesamt, fp, erledigt: erledigt.size};
}
function planUmschalten(ziel) {
  if (PLAN?.ziel === ziel) { PLAN = null; $('plan').hidden = true; $('planknopf').classList.remove('an'); $('dock-weg')?.classList.remove('an'); markiere(); zeige('Planung aufgehoben'); refreshDossier(); return; }
  PLAN = berechnePlan(ziel); speicher.schreib('plan', ziel);
  if (istMobil()) { // Handy: Dossier zu, Wegblatt halb hoch, Karte darüber zeigt den ganzen Weg
    zustand.gewaehlt = null; $('dossier').hidden = true; schreibHash(); zeigePlan(); markiere();
    passeAn([...PLAN.bedarf.keys(), ...(ziel.startsWith('stufe_') ? [] : [ziel])]);
  } else { zeigePlan(); markiere(); refreshDossier(); }
  requestAnimationFrame(() => impulse()); zeige(`Weg geplant: ${PLAN.schritte.length} Schritte, ${PLAN.stufen.size} Aufstiege`);
}
// Weg auf dem Handy: „Weg“ im Dock ohne Plan → Ziel wählen (Suche oder Eintrag antippen).
function zielwahl(an) {
  zustand.zielwahl = an; $('dock-weg')?.classList.toggle('an', an || !!PLAN);
  suche.placeholder = an ? 'Ziel für den Weg …' : 'Suchen …';
  if (an) { zeige('Weg planen: Ziel suchen oder Eintrag auf der Karte antippen', {ms: 4000}); suche.focus(); }
}
function zielGewaehlt(id) { zielwahl(false); suche.blur(); planUmschalten(id); }
function refreshDossier() { if (zustand.gewaehlt) oeffneDossier(dossierKnoten(K[zustand.gewaehlt])); else { const h = W.querySelector('#halten'); if (h?.dataset.ziel?.startsWith('stufe_')) zeigeStufeNurDossier(+h.dataset.ziel.slice(6)); } }
function zeigeStufeNurDossier(nr) { const c = {...zustand.cam}; zeigeStufe(nr); cancelAnimationFrame(flug); zustand.cam = c; kamera(); }
function zeigePlan() {
  const p = PLAN, el = $('plan'); if (!p) { el.hidden = true; return; }
  const gruppen = {}; for (const s of p.schritte) (gruppen[s.stufe] ||= []).push(s);
  const ord = id => BEREICH[K[id].bereich].i * 1000 + (K[id].typ === 'gebaeude' ? 0 : 1);
  const angabe = s => { const k = K[s.id]; return MIT_STUFEN.has(k.typ) ? (s.von > 1 ? `Stufe ${s.von}→${s.n}` : 'Stufe ' + s.n) : EINHEIT.has(k.typ) || ['boden', 'rakete'].includes(k.typ) ? '1×' : s.n > 1 ? (s.id === 'kz_stabilitaet' ? '≥ ' : '') + kurz(s.n) : '✓'; };
  const html = STUFE.map(st => { const g = (gruppen[st.nr] || []).sort((a, b) => ord(a.id) - ord(b.id)), auf = p.stufen.has(st.nr);
    if (!g.length && !auf) return '';
    return `<div class="plan-stufe">${esc(st.name)}</div><ul class="plan-liste">${auf ? `<li><button type="button" data-geh="stufe_${st.nr}"><img alt="" src="${esc(bildKlein('stufe_' + st.nr))}"><span>Aufstieg: ${esc(st.name)}${st.einwohner ? `<br><small data-st="color:var(--leise)">${fmt(st.einwohner)} Einwohner${st.kolonien ? `, ${st.kolonien} Kolonien` : ''}</small>` : ''}</span><b>↑</b></button></li>` : ''}${g.map(s => `<li><button type="button" data-geh="${s.id}"><img alt="" src="${esc(bild(s.id))}" loading="lazy"><span>${esc(name(s.id))}<br><small data-st="color:var(--leise)">${esc(TYP[K[s.id].typ])}</small></span><b>${angabe(s)}</b></button></li>`).join('')}</ul>`; }).join('');
  const mobil = istMobil();
  el.innerHTML = `<div class="plan-kopf"><span>GEPLANTER WEG</span><button type="button" class="x" id="plan-zu" aria-label="Plan schließen">✕</button></div><div class="plan-inhalt">
    <div class="plan-ziel"><img alt="" src="${esc(bild(p.ziel))}"><div><small>Ziel</small><strong>${esc(name(p.ziel))}</strong><small>${p.schritte.length} Schritte · ${p.stufen.size} Aufstiege · ${esc(D.voelker[zustand.volk].name)}</small></div></div>
    ${mobil ? html : ''}${abschnitt('Gesamtkosten', guterListe(p.gesamt) + (p.fp ? `<p class="vp-hinweis">Forschungspunkte: ${kurz(p.fp)}${D.voelker[zustand.volk].regel.forschung !== 1 ? ` (Tempo ${mal(D.voelker[zustand.volk].regel.forschung)})` : ''}</p>` : ''), 'alle Stufen kumuliert', !mobil)}
    ${S ? `<p class="vp-hinweis">Ab deinem Stand gerechnet: ${p.erledigt} Bedingungen sind schon erfüllt und fehlen hier; Ausbaustufen zählen ab deiner aktuellen Stufe.</p>` : ''}
    <p class="vp-hinweis">Bei „eine davon genügt“ nimmt der Plan die erste Möglichkeit; Versorgung (Strom, Personal) ist nicht eingerechnet.</p>
    ${mobil ? '' : html}<div class="knopfzeile"><button type="button" id="plan-zeigen">Auf der Karte zeigen</button><button type="button" id="plan-weg">Plan aufheben</button></div></div>`;
  el.hidden = false; $('planknopf').classList.add('an'); $('dock-weg')?.classList.add('an');
}
$('plan').addEventListener('click', e => {
  const g = e.target.closest('[data-geh]'); if (g) { const id = g.dataset.geh; id.startsWith('stufe_') ? zeigeStufe(+id.slice(6)) : waehle(id, {fliegen: true}); if (sicht().mobil) $('plan').hidden = true; return; }
  if (e.target.closest('#plan-zu')) { $('plan').hidden = true; kamera(); }
  if (e.target.closest('#plan-weg')) planUmschalten(PLAN.ziel);
  if (e.target.closest('#plan-zeigen')) { passeAn([...PLAN.bedarf.keys()]); impulse(); }
});
$('planknopf').addEventListener('click', () => { if (!PLAN) { zeige('Noch kein Weg geplant – Eintrag öffnen und „Weg planen“ halten.'); return; } $('plan').hidden = !$('plan').hidden; if (!$('plan').hidden) { passeAn([...PLAN.bedarf.keys()]); impulse(); } });
$('regelknopf').addEventListener('click', zeigeRegeln);

/* ---------- Völker ---------- */
function baueVoelker() {
  $('voelker').innerHTML = Object.entries(D.voelker).map(([k, v]) => `<button type="button" class="volk-knopf" data-volk="${k}" data-st="--vf:${v.farbe}" aria-pressed="${k === zustand.volk}">${v.emblem ? `<img alt="" src="${esc(BASIS + v.emblem)}">` : ''}<span>${esc(v.name)}</span></button>`).join('');
}
$('voelker').addEventListener('click', e => { const b = e.target.closest('[data-volk]'); if (b) setzeVolk(b.dataset.volk); });
const VOLK_FELDER = [['ladung', 'Ladekapazität'], ['marktgebuehr', 'Marktgebühr', true], ['waffen', 'Waffen'], ['panzerung', 'Panzerung'], ['steuer', 'Steuereinnahmen'], ['werftzeit', 'Werftbauzeit', true],
  ['forschung', 'Forschungstempo'], ['wachstum', 'Bevölkerungswachstum'], ['nahrung', 'Nahrung'], ['energie', 'Energie'], ['kolonieschiff', 'Kosten Kolonieschiff', true]];
function setzeVolk(v) {
  zustand.volk = v; speicher.schreib('volk', v); const V = D.voelker[v];
  $('app').style.setProperty('--volk', V.farbe);
  W.querySelectorAll('.volk-knopf').forEach(b => b.setAttribute('aria-pressed', b.dataset.volk === v));
  const r = V.regel, zeilen = VOLK_FELDER.filter(([f]) => Math.abs((r[f] ?? 1) - 1) > 1e-9).map(([f, t, kl]) => { const gut = kl ? r[f] < 1 : r[f] > 1; return `<li class="${gut ? 'plus' : 'minus'}"><span>${t}</span><b>${prozent(r[f])}</b></li>`; });
  if (r.pluenderquote != null) zeilen.push(`<li class="${r.pluenderquote > D.pluenderquote ? 'plus' : 'minus'}"><span>Plünderquote</span><b>${fmt(r.pluenderquote * 100, 0)} %</b></li>`);
  if (r.ohne_nahrung) zeilen.push(`<li class="plus"><span>Braucht keine Nahrung (dafür Strom)</span><b>✓</b></li>`);
  const H = V.habitat, hab = H ? `<div class="hab-streifen" aria-label="Klimafaktor je Bahn">${H.bahnen.map(b => `<span class="${b.faktor >= 1 ? 'vt-sg' : b.faktor >= .9 ? 'vt-g' : b.faktor >= .75 ? 'vt-n' : b.faktor >= .6 ? 'vt-s' : 'vt-ss'}${b.platz === H.heimat ? ' heim' : ''}" title="Bahn ${b.platz}: ×${fmt(b.faktor, 2)}${b.terraforming ? ', Terraforming ' + b.terraforming : ''}"><b>${b.platz}</b>${b.terraforming ? `<small>TF${b.terraforming}</small>` : ''}</span>`).join('')}</div><p class="vp-hinweis">Klima je Bahn 1 (innen, Glut) bis 12 (außen, Frost); Heimatbahn ${H.heimat}, Volksfaktor Wachstum ${fmt(H.volksfaktor, 2)}.</p>` : '';
  baueNach();
  const betroffen = Object.values(K).filter(n => volkEffekte(n).length).length;
  const pv = $('volksprofil'), zu = pv.classList.contains('zu');
  pv.innerHTML = `<button type="button" class="vp-kopf" id="vp-kopf" aria-expanded="${!zu}"><img alt="" src="${esc(BASIS + (V.portrait || V.emblem || ''))}"><div><strong>${esc(V.name.toUpperCase())}</strong><small>${betroffen} Einträge verändert</small></div><span class="pfeil">▾</span></button>
    <div class="vp-rest">${V.zivilisation ? `<img class="vp-stadt" alt="Hauptstadt der ${esc(V.name)}" src="${esc(BASIS + V.zivilisation)}">` : ''}<ul class="vp-liste">${zeilen.join('') || '<li class="leer"><span>Keine Abweichungen</span><b>–</b></li>'}</ul>${hab}<p class="vp-hinweis">Bäume sind für alle Völker gleich; das Volk ändert Werte, Kosten, Tempo, Klima und Motive. „Volksänderungen“ blendet alles andere aus.</p></div>`;
  aktualisiereBilder(); for (const k of W.querySelectorAll('.band-kopf img')) k.src = bildKlein('stufe_' + k.parentElement.dataset.stufe);
  if (LAY) zeichneLinien();
  if (PLAN) { const offen = !$('plan').hidden; PLAN = berechnePlan(PLAN.ziel); zeigePlan(); $('plan').hidden = !offen; }
  if (zustand.gewaehlt) oeffneDossier(dossierKnoten(K[zustand.gewaehlt]));
  W.querySelector('.kulisse-bild').style.setProperty('--hg', `url("${new URL(BASIS + (D.hintergrund.command || D.hintergrund.stars), document.baseURI).href}")`); // absolut: CSS-Variablen lösen relativ zur CSS-Datei auf
  schreibHash(); zeichneMinikarteNeu(); markiere();
}
$('volksprofil').addEventListener('click', e => { if (e.target.closest('#vp-kopf')) { const p = $('volksprofil'); p.classList.toggle('zu'); $('vp-kopf').setAttribute('aria-expanded', !p.classList.contains('zu')); speicher.schreib('vpzu', p.classList.contains('zu') ? '1' : '0'); } });

/* ---------- Werkzeuge oben ---------- */
$('meinestufe').innerHTML += STUFE.map(s => `<option value="${s.nr}">${esc(s.name)}</option>`).join('');
$('meinestufe').options[0].textContent = 'Alle Stufen';
$('meinestufe').addEventListener('change', e => { zustand.meine = +e.target.value; markiere(); stufenStatus(); if (zustand.meine && istMobil()) zuBand(zustand.meine); zeige(zustand.meine ? `Gesperrt ist alles über ${STUFE[zustand.meine - 1].name}` : 'Alle Stufen offen'); });
$('volksmodus').dataset.kurz = 'Volk'; $('allelinien').dataset.kurz = 'Linien';
$('volksmodus').addEventListener('click', e => { zustand.volksmodus = !zustand.volksmodus; e.currentTarget.setAttribute('aria-pressed', zustand.volksmodus); welt.classList.toggle('volksfokus', zustand.volksmodus);
  zeige(zustand.volksmodus ? `Hervorgehoben: was die ${D.voelker[zustand.volk].name} verändern` : 'Alle Einträge sichtbar'); });
$('allelinien').addEventListener('click', e => { zustand.allelinien = !zustand.allelinien; e.currentTarget.setAttribute('aria-pressed', zustand.allelinien); zeichneLinien(); });
$('ganz').addEventListener('click', gesamt);
$('rein').addEventListener('click', () => { const v = sicht(); zoomUm(1.35, v.w / 2, v.h / 2); });
$('raus').addEventListener('click', () => { const v = sicht(); zoomUm(1 / 1.35, v.w / 2, v.h / 2); });
$('allezu').addEventListener('click', () => { const alle = zustand.zu.size < D.bereiche.length; zustand.zu = alle ? new Set(D.bereiche.map(b => b.id)) : new Set(); neuLayout(); gesamt(); zeige(alle ? 'Alle Bäume zugeklappt – Bereich antippen zum Öffnen' : 'Alle Bäume aufgeklappt'); });

// Suche
const suche = $('suche'), treffer = $('suchtreffer');
const norm = s => String(s || '').toLocaleLowerCase('de').normalize('NFD').replace(/[̀-ͯ]/g, '');
suche.addEventListener('input', () => {
  const q = norm(suche.value.trim()); if (!q) { treffer.hidden = true; return; }
  const liste = Object.values(K).map(n => ({n, p: norm(n.name).startsWith(q) ? 0 : norm(n.name).includes(q) ? 1 : norm((n.klasse || '') + ' ' + (n.rolle || '') + ' ' + TYP[n.typ]).includes(q) ? 2 : 9})).filter(x => x.p < 9).sort((a, b) => a.p - b.p).slice(0, 14);
  treffer.innerHTML = liste.length ? liste.map(({n}) => `<li><button type="button" data-geh="${n.id}"><img alt="" src="${esc(bild(n.id))}" loading="lazy"><span>${esc(n.name)}<small>${esc(BEREICH[n.bereich].titel)} · ${esc(TYP[n.typ])} · Stufe ${ROEM[n.stufe] || n.stufe}</small></span></button></li>`).join('') : '<li data-st="padding:8px;color:var(--leise)">Nichts gefunden</li>';
  treffer.hidden = false;
});
suche.addEventListener('keydown', e => { if (e.key === 'Enter') treffer.querySelector('[data-geh]')?.click(); if (e.key === 'Escape') { suche.value = ''; treffer.hidden = true; } if (e.key === 'ArrowDown') { e.preventDefault(); treffer.querySelector('button')?.focus(); } });
treffer.addEventListener('click', e => { const b = e.target.closest('[data-geh]'); if (!b) return; treffer.hidden = true; suche.value = ''; suche.blur();
  if (zustand.zielwahl) { zielGewaehlt(b.dataset.geh); return; }
  waehle(b.dataset.geh, {fliegen: true});
  const el = knotenEl[b.dataset.geh]; el.classList.remove('treffer'); void el.offsetWidth; el.classList.add('treffer'); });
document.addEventListener('pointerdown', e => { if (!e.composedPath().some(x => x.classList?.contains('suche'))) treffer.hidden = true; });

// Tastatur
document.addEventListener('keydown', e => {
  if (EINGEBETTET && !OPT.aktiv?.()) return;
  if (e.composedPath()[0]?.matches?.('input,select,textarea')) return;
  const v = sicht();
  if (e.key === 'Escape') { if (!zurueck()) OPT.schliessen?.(); }
  else if (e.key === '0') gesamt();
  else if (e.key === '+' || e.key === '=') zoomUm(1.25, v.w / 2, v.h / 2);
  else if (e.key === '-') zoomUm(.8, v.w / 2, v.h / 2);
  else if (e.key === '/') { e.preventDefault(); suche.focus(); }
  else if (e.key.startsWith('Arrow') && (W.activeElement || document.activeElement) === buehne) { const d = 80; zustand.cam.x += e.key === 'ArrowLeft' ? d : e.key === 'ArrowRight' ? -d : 0; zustand.cam.y += e.key === 'ArrowUp' ? d : e.key === 'ArrowDown' ? -d : 0; begrenze(); kamera(); e.preventDefault(); }
});

/* ---------- Minikarte ---------- */
const mk = $('minikarte'), mkc = mk.getContext('2d'); let minikarteGrund = null;
function zeichneMinikarteNeu() { minikarteGrund = null; zeichneMinikarte(); }
function mkMass() { const dpr = devicePixelRatio || 1, W = 220, H = 140; if (mk.width !== W * dpr) { mk.width = W * dpr; mk.height = H * dpr; } const s = Math.min(W / LAY.w, H / LAY.h); return {dpr, W, H, s, ox: (W - LAY.w * s) / 2, oy: (H - LAY.h * s) / 2}; }
function zeichneMinikarte() {
  if (!LAY || mk.offsetParent === null) return; const m = mkMass();
  mkc.setTransform(m.dpr, 0, 0, m.dpr, 0, 0); mkc.clearRect(0, 0, m.W, m.H);
  if (!minikarteGrund) { minikarteGrund = document.createElement('canvas'); minikarteGrund.width = m.W * m.dpr; minikarteGrund.height = m.H * m.dpr; const g = minikarteGrund.getContext('2d'); g.setTransform(m.dpr, 0, 0, m.dpr, 0, 0);
    LAY.baender.forEach((b, i) => { g.fillStyle = i % 2 ? 'rgba(95,225,255,.05)' : 'rgba(95,225,255,.02)'; g.fillRect(m.ox, m.oy + b.y * m.s, LAY.w * m.s, b.h * m.s); });
    for (const b of LAY.bereiche) { g.fillStyle = b.farbe; g.globalAlpha = .5; g.fillRect(m.ox + b.x * m.s, m.oy, b.w * m.s, 2); g.globalAlpha = 1; }
    for (const [id, p] of Object.entries(LAY.pos)) { if (p.zu) continue; g.fillStyle = BEREICH[K[id].bereich].farbe; g.globalAlpha = .85; g.fillRect(m.ox + (p.x + 36) * m.s, m.oy + (p.y + 30) * m.s, Math.max(2, 24 * m.s), Math.max(2, 24 * m.s)); }
    g.globalAlpha = 1; }
  mkc.drawImage(minikarteGrund, 0, 0, m.W, m.H);
  if (zustand.gewaehlt) { const p = LAY.pos[zustand.gewaehlt]; mkc.fillStyle = '#fff'; mkc.beginPath(); mkc.arc(m.ox + (p.x + 48) * m.s, m.oy + (p.y + 42) * m.s, 3, 0, 7); mkc.fill(); }
  const c = zustand.cam, v = buehne.getBoundingClientRect();
  mkc.strokeStyle = '#8ff0ff'; mkc.lineWidth = 1.2; mkc.strokeRect(m.ox + (-c.x / c.s) * m.s, m.oy + (-c.y / c.s) * m.s, v.width / c.s * m.s, v.height / c.s * m.s);
}
let mkZieht = false;
const mkGehe = e => { const r = mk.getBoundingClientRect(), m = mkMass(), wx = (e.clientX - r.left - m.ox) / m.s, wy = (e.clientY - r.top - m.oy) / m.s, v = sicht(), c = zustand.cam;
  c.x = v.w / 2 - wx * c.s; c.y = v.h / 2 - wy * c.s; begrenze(); kamera(); };
mk.addEventListener('pointerdown', e => { mkZieht = true; mk.setPointerCapture(e.pointerId); cancelAnimationFrame(flug); mkGehe(e); });
mk.addEventListener('pointermove', e => { if (mkZieht) mkGehe(e); });
mk.addEventListener('pointerup', () => mkZieht = false);

/* ---------- Hinweis ---------- */
let hinweisT = null;
function zeige(t, {fehler = false, ms = 2200} = {}) { const h = $('hinweis'); h.textContent = t; h.classList.toggle('fehler', fehler); h.classList.add('zeigen'); clearTimeout(hinweisT); hinweisT = setTimeout(() => h.classList.remove('zeigen'), ms); }

/* ---------- Mobil: Blatt nach unten wischen ---------- */
(() => { const g = $('dossier-griff'); let y0 = null;
  g.addEventListener('pointerdown', e => { y0 = e.clientY; g.setPointerCapture(e.pointerId); });
  g.addEventListener('pointerup', e => { if (y0 == null) return; const dy = e.clientY - y0; y0 = null;
    if (dy > 80) { if ($('dossier').classList.contains('halb')) schliesseDossier(); else $('dossier').classList.add('halb'); } else if (dy < -60) $('dossier').classList.remove('halb'); });
})();

/* ---------- Start ---------- */
if (EINGEBETTET) {
  $('app').classList.add('eingebettet');
  W.querySelector('.marke small').textContent = 'Technologiebaum · Gesamtübersicht';
  const zu = document.createElement('button'); zu.type = 'button'; zu.className = 'schalter zurueck'; zu.textContent = '✕ Zum Spiel'; zu.dataset.kurz = '✕'; zu.setAttribute('aria-label', 'Technologiebaum schließen, zurück zum Spiel');
  zu.addEventListener('click', () => OPT.schliessen?.());
  const stand = document.createElement('button'); stand.type = 'button'; stand.className = 'schalter'; stand.id = 'meinstand'; stand.setAttribute('aria-pressed', 'true');
  stand.title = 'Deinen Stand zeigen: erreicht (Gold), jetzt möglich (Cyan), gesperrt (grau)';
  stand.innerHTML = '<span class="lampe"></span>Mein Stand <small></small>'; stand.dataset.kurz = 'Stand';
  stand.addEventListener('click', () => { zustand.standAn = !(zustand.standAn !== false); stand.setAttribute('aria-pressed', zustand.standAn); welt.classList.toggle('stand-an', !!S && zustand.standAn); });
  const werkzeuge = W.querySelector('.kopf-werkzeuge'); werkzeuge.prepend(stand); werkzeuge.prepend(zu);
}
if (OPT.atlas) {
  const at = document.createElement('button'); at.type = 'button'; at.className = 'schalter atlas-knopf'; at.dataset.kurz = '⚔'; at.textContent = 'Kräfteatlas';
  at.title = 'Kräfteatlas: wer schlägt wen – gemessen mit der Kampfengine bei gleichem Wert'; at.setAttribute('aria-label', at.title);
  at.addEventListener('click', () => OPT.atlas.oeffnen ? OPT.atlas.oeffnen(null) : OPT.atlas.zeige?.(null));
  W.querySelector('.kopf-werkzeuge').append(at);
}
/* ---------- Handy: Menü (Volk, Anzeige, Regeln) und Dock (Stand, Weg, Bereiche) ---------- */
const menueKnopf = document.createElement('button'); menueKnopf.type = 'button'; menueKnopf.className = 'schalter menue-knopf'; menueKnopf.id = 'menueknopf';
menueKnopf.dataset.kurz = '☰'; menueKnopf.setAttribute('aria-label', 'Menü: Volk, Anzeige, Regeln'); menueKnopf.setAttribute('aria-expanded', 'false'); menueKnopf.setAttribute('aria-controls', 'menue');
W.querySelector('.kopf-werkzeuge').append(menueKnopf);
const menue = document.createElement('div'); menue.className = 'menue glas'; menue.id = 'menue'; menue.hidden = true; menue.setAttribute('role', 'dialog'); menue.setAttribute('aria-label', 'Menü');
const dock = document.createElement('nav'); dock.className = 'dock glas'; dock.id = 'dock'; dock.setAttribute('aria-label', 'Stand, Weg und Bereiche');
dock.innerHTML = `<button type="button" class="dock-knopf" id="dock-stand"><b>◉</b><span>${EINGEBETTET ? 'Stand' : 'Start'}</span></button><button type="button" class="dock-knopf" id="dock-weg"><b>◎</b><span>Weg</span></button>`
  + `<div class="dock-chips" id="dock-chips">${D.bereiche.map(b => `<button type="button" data-zoom="${b.id}" data-st="--bf:${b.farbe}">${esc(b.titel)}</button>`).join('')}</div>`;
$('app').append(menue, dock);
const schaltZeile = (m, text, an) => `<button type="button" class="menue-zeile" data-m="${m}" aria-pressed="${!!an}"><span class="lampe"></span>${text}</button>`;
function baueMenue() {
  menue.innerHTML = `<div class="menue-kopf"><span>ANSICHT</span><button type="button" class="x" data-m="zu" aria-label="Menü schließen">✕</button></div>
    <h4>Volk</h4><div class="menue-voelker">${Object.entries(D.voelker).map(([k, v]) => `<button type="button" class="volk-knopf" data-mvolk="${k}" data-st="--vf:${v.farbe}" aria-pressed="${k === zustand.volk}">${v.emblem ? `<img alt="" src="${esc(BASIS + v.emblem)}">` : ''}<span>${esc(v.name)}</span></button>`).join('')}</div>
    <h4>Anzeige</h4><div class="menue-liste">${S ? schaltZeile('stand', 'Mein Stand (Gold erreicht, Cyan möglich)', zustand.standAn !== false) : ''}${schaltZeile('volksmodus', 'Nur Volksänderungen hervorheben', zustand.volksmodus)}${schaltZeile('allelinien', 'Alle Linien zeigen', zustand.allelinien)}</div>
    <h4>Gehe zu</h4><div class="menue-liste"><button type="button" class="menue-zeile" data-m="profil">Volksprofil ${esc(D.voelker[zustand.volk].name)}</button><button type="button" class="menue-zeile" data-m="ganz">Gesamtübersicht aller Bäume</button><button type="button" class="menue-zeile" data-m="klappen">Alle Bereiche zu-/aufklappen</button><button type="button" class="menue-zeile" data-m="regeln">Alle Regeltexte</button></div>`;
}
function menueZeigen(an) { if (an) baueMenue(); menue.hidden = !an; menueKnopf.setAttribute('aria-expanded', an); }
menueKnopf.addEventListener('click', () => menueZeigen(menue.hidden));
menue.addEventListener('click', e => {
  const v = e.target.closest('[data-mvolk]'); if (v) { setzeVolk(v.dataset.mvolk); baueMenue(); return; }
  const m = e.target.closest('[data-m]')?.dataset.m; if (!m) return;
  if (m === 'stand') $('meinstand')?.click(); else if (m === 'volksmodus') $('volksmodus').click(); else if (m === 'allelinien') $('allelinien').click();
  if (['stand', 'volksmodus', 'allelinien'].includes(m)) { baueMenue(); return; }
  menueZeigen(false);
  if (m === 'profil') { const pv = $('volksprofil'); pv.classList.remove('zu'); pv.classList.add('mobil-offen'); }
  else if (m === 'ganz') gesamt(); else if (m === 'klappen') $('allezu').click(); else if (m === 'regeln') zeigeRegeln();
});
$('volksprofil').addEventListener('click', e => { if (e.target.closest('#vp-kopf') && istMobil()) $('volksprofil').classList.remove('mobil-offen'); });
dock.addEventListener('click', e => {
  const z = e.target.closest('[data-zoom]'); if (z) { zielwahl(false); zumBereich(z.dataset.zoom); return; }
  if (e.target.closest('#dock-stand')) { zielwahl(false); zumStand(); return; }
  if (e.target.closest('#dock-weg')) {
    if (zustand.zielwahl) { zielwahl(false); return; }
    if (!PLAN) { zielwahl(true); return; }
    const pl = $('plan'); if (pl.hidden) { $('dossier').hidden = true; zustand.gewaehlt = null; markiere(); pl.hidden = false; passeAn([...PLAN.bedarf.keys()]); impulse(); } else { pl.hidden = true; kamera(); }
  }
});
document.addEventListener('pointerdown', e => { if (!menue.hidden && !e.composedPath().some(x => x === menue || x === menueKnopf)) menueZeigen(false); });
const gespeichertZu = speicher.lies('zu'); if (gespeichertZu) for (const z of gespeichertZu.split(',')) if (BEREICH[z]) zustand.zu.add(z);
if (speicher.lies('vpzu') !== '0') $('volksprofil').classList.add('zu');
baueNach(); baueVoelker(); baueKnoten(); layout(); setzePositionen();
setzeVolk(zustand.volk);
standAktualisieren();
const startPlan = speicher.lies('plan'); if (startPlan && (K[startPlan] || /^stufe_[2-5]$/.test(startPlan))) { PLAN = berechnePlan(startPlan); zeigePlan(); $('plan').hidden = true; markiere(); }
// Start: Desktop zeigt alle Bäume, das Handy beginnt lesbar beim eigenen Stand.
const startKamera = () => { const {s, x, y} = istMobil() ? standAnsicht() : passend({x: 0, y: 0, w: LAY.w, h: LAY.h}); zustand.cam = {s, x, y}; dockBereich = null; kamera(); };
startKamera();
if (hash.get('id') && K[hash.get('id')]) setTimeout(() => waehle(hash.get('id'), {fliegen: true}), 300);
else zeige(istMobil() ? 'Wischen verschiebt · zwei Finger zoomen · Eintrag antippen' : 'Ziehen zum Verschieben · Rad/zwei Finger zum Zoomen · Eintrag antippen für das Dossier', {ms: 3500});
addEventListener('resize', () => { minikarteGrund = null; if (!angefasst) startKamera(); else kamera(); });
// Zurück (Android, Browser, Esc): erst Menü, Suche, Zielwahl, Volksprofil, Dossier, Weg – dann darf der Baum selbst schließen.
function zurueck() {
  if (!menue.hidden) { menueZeigen(false); return true; }
  if (!treffer.hidden) { treffer.hidden = true; suche.value = ''; suche.blur(); return true; }
  if (zustand.zielwahl) { zielwahl(false); return true; }
  if ($('volksprofil').classList.contains('mobil-offen')) { $('volksprofil').classList.remove('mobil-offen'); return true; }
  if (!$('dossier').hidden) { schliesseDossier(); return true; }
  if (!$('plan').hidden) { $('plan').hidden = true; kamera(); return true; }
  return false;
}
// Bei jedem Öffnen im Spiel: Stand neu lesen; wer noch nichts verschoben hat, landet wieder beim eigenen Stand.
function geoeffnet() { standAktualisieren(); if (!angefasst) startKamera(); }
// Objektansicht im Spiel (Karl 19.10.: „immer den neuen Baum nutzen und für das Objekt passend zoomen, um alles zu
// erkennen – auch auf dem Handy“): Eintrag wählen (Dossier, ganzer Pfad hervorgehoben) und die Kamera auf ihn, seine
// direkten Voraussetzungen und das direkt Freigeschaltete einpassen. Nie kleiner als lesbar: passt der Kreis nicht, nur
// Eintrag + Voraussetzungen, sonst der Eintrag in der Mitte (Pfadlinien führen weiter).
function zeigeObjekt(id) {
  const n = K[id]; if (!n) return false;
  if (zustand.zu.has(n.bereich)) { zustand.zu.delete(n.bereich); neuLayout(); }
  angefasst = true; zustand.gewaehlt = id; markiere(); oeffneDossier(dossierKnoten(n)); schreibHash();
  const da = i => LAY.pos[i] && !LAY.pos[i].zu, vor = vorausVon(n).filter(v => HART.has(v.art)).map(v => v.id).filter(da), folge = (nach[id] || []).map(v => v.id).filter(da);
  const v = sicht(), min = v.mobil ? .62 : .5;
  const rahmen = ids => { const ps = ids.map(i => LAY.pos[i]), x = Math.min(...ps.map(p => p.x)) - 30, y = Math.min(...ps.map(p => p.y)) - 40;
    return passend({x, y, w: Math.max(...ps.map(p => p.x)) + 126 - x, h: Math.max(...ps.map(p => p.y)) + 150 - y}, 1.1); };
  let ziel = [[id, ...vor, ...folge], [id, ...vor]].map(rahmen).find(z => z.s >= min);
  if (!ziel) { const p = LAY.pos[id], bw = v.w - v.links - v.rechts, bh = v.h - v.oben - v.unten; ziel = {s: min, x: v.links + bw / 2 - (p.x + 48) * min, y: v.oben + bh / 2 - (p.y + 60) * min}; }
  fliege(ziel, 750);
  zeige(`${n.name || id}: ${vor.length ? vor.length + ' direkte Voraussetzung' + (vor.length > 1 ? 'en' : '') : 'keine Voraussetzung'} · ${folge.length ? 'schaltet ' + folge.length + ' frei' : 'schaltet nichts frei'}`, {ms: 3200});
  return true;
}
window.__techbaum = {zeigeObjekt, zustand, LAY: () => LAY, waehle, zumBereich, zumStand, planUmschalten, berechnePlan, zeigeBereich, zeigeRegeln, Kampf, standAktualisieren, zurueck, geoeffnet, sicht};
return window.__techbaum;
};
const LOGO = 'data:image/webp;base64,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';
const GERUEST = '<div id="app" class="app">\n  <div class="kulisse" aria-hidden="true"><div class="kulisse-bild"></div><div class="kulisse-raster"></div><div class="kulisse-vignette"></div></div>\n\n  <header class="kopf glas">\n    <div class="marke">\n      <img class="marke-logo" alt="Sternenepoche" decoding="async">\n      <div><strong>STERNENEPOCHE</strong><small>Technologiebaum · Gesamtübersicht</small></div>\n    </div>\n    <nav class="voelker" id="voelker" aria-label="Volk wählen"></nav>\n    <div class="kopf-werkzeuge">\n      <label class="suche"><span class="sr">Suchen</span><input id="suche" type="search" placeholder="Suchen …" autocomplete="off"><ul id="suchtreffer" class="suchtreffer glas" hidden></ul></label>\n      <label class="stufenwahl" title="Was ist bei deiner Stufe schon offen?"><span>Meine Stufe</span><select id="meinestufe"><option value="0">alle zeigen</option></select></label>\n      <button type="button" class="schalter" id="volksmodus" aria-pressed="false" title="Nur Einträge hervorheben, die das gewählte Volk verändert"><span class="lampe"></span>Volksänderungen</button>\n      <button type="button" class="schalter" id="allelinien" aria-pressed="false" title="Auch Gebäude-Voraussetzungen (Labor, Werft, Kaserne …) dauerhaft zeigen"><span class="lampe"></span>Alle Linien</button>\n    </div>\n  </header>\n\n  <section class="volksprofil glas" id="volksprofil" aria-label="Volksprofil"></section>\n\n  <main class="buehne" id="buehne" tabindex="0" aria-label="Technologiekarte: ziehen zum Verschieben, Mausrad oder zwei Finger zum Zoomen">\n    <div class="welt" id="welt">\n      <svg class="linien" id="linien" aria-hidden="true"><defs>\n        <linearGradient id="g-pfad" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="#ffd68a"/><stop offset="1" stop-color="#ff9f43"/></linearGradient>\n        <linearGradient id="g-frei" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="#8ff0ff"/><stop offset="1" stop-color="#3fb6ff"/></linearGradient>\n        <filter id="glimmen" x="-50%" y="-50%" width="200%" height="200%"><feGaussianBlur stdDeviation="3" result="b"/><feMerge><feMergeNode in="b"/><feMergeNode in="SourceGraphic"/></feMerge></filter>\n      </defs><g id="linien-ruhe"></g><g id="linien-aktiv"></g><g id="impulse"></g></svg>\n      <div class="baender" id="baender"></div>\n      <div class="spalten" id="spalten"></div>\n      <div class="knoten-ebene" id="knoten"></div>\n    </div>\n    <div class="leiste-stufen" id="leiste-stufen" aria-hidden="true"></div>\n    <div class="leiste-bereiche" id="leiste-bereiche"></div>\n  </main>\n\n  <aside class="dossier glas" id="dossier" aria-label="Dossier" hidden>\n    <div class="dossier-griff" id="dossier-griff" aria-hidden="true"></div>\n    <div class="dossier-kopf"><span>DOSSIER</span><button type="button" class="x" id="dossier-zu" aria-label="Dossier schließen">✕</button></div>\n    <div class="dossier-inhalt" id="dossier-inhalt"></div>\n  </aside>\n\n  <aside class="plan glas" id="plan" aria-label="Geplanter Weg" hidden></aside>\n\n  <div class="minikarte glas" id="minikarte-rahmen"><canvas id="minikarte" width="220" height="140" aria-label="Übersichtskarte – tippen, um dorthin zu springen"></canvas></div>\n\n  <nav class="daumen glas" aria-label="Schnellsteuerung">\n    <button type="button" id="ganz" title="Gesamtübersicht (Taste 0)"><b>⌂</b><span>Übersicht</span></button>\n    <button type="button" id="rein" title="Hineinzoomen (+)"><b>＋</b><span>Näher</span></button>\n    <button type="button" id="raus" title="Herauszoomen (−)"><b>－</b><span>Weiter</span></button>\n    <button type="button" id="allezu" title="Alle Bereiche zu- oder aufklappen"><b>⇕</b><span>Klappen</span></button>\n    <button type="button" id="planknopf" title="Geplanten Weg zeigen"><b>◎</b><span>Weg</span></button>\n    <button type="button" id="regelknopf" title="Alle Regeltexte"><b>§</b><span>Regeln</span></button>\n  </nav>\n  <p class="hinweis" id="hinweis" role="status" aria-live="polite"></p>\n</div>';

