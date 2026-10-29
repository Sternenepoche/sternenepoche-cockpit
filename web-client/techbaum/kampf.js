'use strict';
// Einheitennamen in schmalen Balkenzeilen weich trennen (silbenTrennen aus techbaum.js, sonst unverändert).
const trennK = s => { const e = String(s ?? '').replace(/[&<>"']/g, c => ({'&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;'}[c])); return window.silbenTrennen ? window.silbenTrennen(e).replace(/\u00ad/g, '&shy;') : e; };
// Kampfauswertung für die Techbaum-Testseite. Rechnet nach den Regeln des Kerns (crates/kern/src/kampf.rs, zielwahl.rs):
// je Zielkanal voller Grundschaden × Zielfaktor (Primär ×2, Sekundär ×1,5 oder eigener Faktor > 1, Reserve ×1);
// Schildschaden × Schildfaktor nur auf Primär-/Sekundärziele; Schuss unter 1 % des Schildwerts verpufft;
// Schnellfeuer r nur auf Primär-/Sekundärziele: Folgeschuss mit (r−1)/r, höchstens 64 Schüsse je Kanal; mit Mehrfachfeuer
// (neue Welten, D.kampfregeln.mehrfachfeuer) springt die Salve nach einem Abschuss auf das nächste Ziel derselben Art.
// Bodenkampf: Angriff × Konterfaktor, kein Schild, ein Schuss je Runde, alle feuern gleichzeitig.
// Darstellung als Gefechtsprognose: beide Seiten setzen gleich viel Güterwert ein, Runde für Runde bis zur Entscheidung.
window.TechbaumKampf = ctx => {
const {D, K, esc, fmt, kurz, bild, name} = ctx;
const VERPUFF = D.kampfregeln?.verpuffen_anteil ?? 0.01;
const RUNDEN_FLOTTE = D.kampfregeln?.runden || 6;
const MEHRFACH = !!D.kampfregeln?.mehrfachfeuer;
const BODEN_MIN = (D.bodenkampf?.rundenabstand_sekunden || 900) / 60, RUNDEN_BODEN = 192; // 48 Stunden Belagerung
const FLOTTE = new Set(['schiff', 'megaschiff', 'verteidigung', 'schild', 'satellit']);
const ZIVIL = new Set(['transporter', 'kolonie', 'erkundung', 'sonde', 'bergbau', 'bergung', 'truppen', 'terraforming', 'satellit']);
const URTEILE = [
  {k: 'sg', t: 'klar überlegen', lang: 'gewinnt deutlich'},
  {k: 'g', t: 'überlegen', lang: 'gewinnt'},
  {k: 'n', t: 'ausgeglichen', lang: 'etwa gleich stark'},
  {k: 's', t: 'unterlegen', lang: 'verliert'},
  {k: 'ss', t: 'klar unterlegen', lang: 'verliert deutlich'},
  {k: 'x', t: 'kaum Wirkung', lang: 'beide richten fast nichts aus'}];
const URTEIL = Object.fromEntries(URTEILE.map(u => [u.k, u]));
const RANG = ['Primärziel', 'Sekundärziel', 'Reserveziel'];
const pct = v => v >= 0.995 ? '100 %' : v <= 0 ? '0 %' : v < 0.01 ? '< 1 %' : fmt(v * 100, 0) + ' %';
const stueck = n => n >= 100 ? fmt(n, 0) : n >= 10 ? fmt(n, 0) : n >= 1 ? fmt(n, 1) : fmt(n, 2);
const avatar = n => `<img alt="" src="${esc(bild(n.id))}" loading="lazy">`;

/* ---------- Treffer nach Kernregeln ---------- */
function rang(A, B) {
  const k = A.kampf || {};
  if (!(A.werte?.angriff > 0)) return null;
  if (k.nicht_erfassbar?.includes(B.id)) return null;
  if (k.primaer?.includes(B.id)) return 0;
  if (k.sekundaer?.includes(B.id)) return 1;
  return 2;
}
function treffer(A, B, vA, vB) {
  if (A.typ === 'boden') {
    const p = (A.konter?.[B.id] ?? 1000) / 1000, d = (A.werte?.angriff || 0) * p, H = B.werte?.struktur || 1;
    return {boden: true, f: p, d, H, kan: 1, r: 1, E: 1, sf: 1, S: 0, rang: d > 0 ? 0 : null};
  }
  const rg = rang(A, B); if (rg == null) return {rang: null, d: 0, H: B.werte?.struktur || 1, S: B.werte?.schild || 0};
  const k = A.kampf || {}, bev = rg < 2;
  const f = bev ? (k.schadensfaktoren?.[B.id] ?? (rg === 0 ? 2 : 1.5)) : 1;
  const r = bev ? Math.min(64, k.schnellfeuer?.[B.id] || 1) : 1, E = r > 1 ? r * (1 - Math.pow(1 - 1 / r, 64)) : 1;
  const sf = bev ? (k.schildfaktor || 1) : 1;
  const d = Math.floor(A.werte.angriff * (vA?.waffen ?? 1) + 1e-9) * f;
  const S = B.werte?.schild || 0, H = Math.floor((B.werte?.struktur || 1) * (vB?.panzerung ?? 1) + 1e-9);
  const verpufft = S > 0 && d * sf < S * VERPUFF;
  return {rang: rg, f, r, E, sf, d, S, H, verpufft, kan: k.ziele_je_runde || A.ziele || 1};
}
const wertVon = n => n.wert || 1;
// Anteil der aktuell lebenden Ziele, der in einer Runde fällt, wenn je Ziel q Zielkanäle eintreffen.
// Der Schild erneuert sich je Runde; q < 1: nur ein Teil der Ziele wird getroffen, jedes verliert seinen Schild einmal.
function anteil(t, q) {
  if (!t || t.rang == null || t.verpufft || !(t.d > 0) || !(q > 0)) return 0;
  const schild = (t.S || 0) / (t.sf || 1);
  // Mehrfachfeuer: reicht die Salve (Ø E Schüsse) für einen Abschuss mit k Schüssen, fallen je Zielkanal E / k Ziele.
  if (MEHRFACH && t.r > 1) { const k = Math.max(1, Math.ceil((schild + t.H) / t.d)); if (t.E >= k) return Math.min(1, q * t.E / k); }
  const struktur = q < 1 ? q * Math.max(0, t.E * t.d - schild) : Math.max(0, q * t.E * t.d - schild);
  return Math.min(1, q, struktur / t.H);
}
/* ---------- Gefechtsprognose bei gleichem Einsatz ---------- */
function gefecht(A, B, vA, volk) {
  const ab = treffer(A, B, vA, null), ba = treffer(B, A, null, vA);
  const wA = wertVon(A), wB = wertVon(B), boden = A.typ === 'boden';
  const q0ab = (ab.kan || 1) * wB / wA, q0ba = (ba.kan || 1) * wA / wB;
  const max = boden ? RUNDEN_BODEN : RUNDEN_FLOTTE;
  let a = 1, b = 1, endA = null, endB = null; const verlauf = [{a, b}];
  // Echte Kampfengine (D.gefechte, tools/konter_matrix/src/bin/techbaum_gefechte.rs): Verluste und Runden je Volk; die
  // Näherung unten bleibt nur der Rückfall für Paare ohne Messung.
  const echt = !boden && D.gefechte?.[volk]?.[`${A.id}|${B.id}`];
  if (echt) {
    a = 1 - echt[0] / 100; b = 1 - echt[1] / 100;
    const runden = Math.max(1, echt[2]);
    if (echt[1] >= 100) endB = runden;
    if (echt[0] >= 100) endA = runden;
    verlauf.push({a, b});
    const vB = 1 - b, vA_ = 1 - a;
    let u;
    if (vA_ < 0.02 && vB < 0.02) u = URTEIL.x;
    else { const v = (vB + 0.005) / (vA_ + 0.005); u = v >= 3 ? URTEIL.sg : v >= 1.5 ? URTEIL.g : v > 2 / 3 ? URTEIL.n : v > 1 / 3 ? URTEIL.s : URTEIL.ss; }
    const nA = wA >= wB ? 10 : 10 * wB / wA, nB = wA >= wB ? 10 * wA / wB : 10;
    return {A, B, ab, ba, q0ab, q0ba, a, b, verlustA: vA_, verlustB: vB, endA, endB, runden, verlauf, urteil: u, nA, nB, boden, max, gemessen: true};
  }
  for (let r = 1; r <= max; r++) {
    const kb = b * anteil(ab, q0ab * a / b), ka = a * anteil(ba, q0ba * b / a);
    b = Math.max(0, b - kb); a = Math.max(0, a - ka); verlauf.push({a, b});
    if (b <= 0.005 && endB == null) endB = r;
    if (a <= 0.005 && endA == null) endA = r;
    if (a <= 0.005 || b <= 0.005 || (ka < 1e-9 && kb < 1e-9)) break;
  }
  const vB = 1 - b, vA_ = 1 - a;
  let u;
  if (vA_ < 0.02 && vB < 0.02) u = URTEIL.x;
  else { const v = (vB + 0.005) / (vA_ + 0.005); u = v >= 3 ? URTEIL.sg : v >= 1.5 ? URTEIL.g : v > 2 / 3 ? URTEIL.n : v > 1 / 3 ? URTEIL.s : URTEIL.ss; }
  const nA = wA >= wB ? 10 : 10 * wB / wA, nB = wA >= wB ? 10 * wA / wB : 10;
  return {A, B, ab, ba, q0ab, q0ba, a, b, verlustA: vA_, verlustB: vB, endA, endB, runden: verlauf.length - 1, verlauf, urteil: u, nA, nB, boden, max};
}
function gegnerVon(A) {
  if (A.typ === 'boden') return Object.values(K).filter(n => n.typ === 'boden' && n.id !== 'garnison' && n.id !== A.id);
  return Object.values(K).filter(n => FLOTTE.has(n.typ) && n.id !== A.id && (n.werte?.struktur || 0) > 0);
}
function grund(t) {
  if (!t || t.rang == null) return 'kann nicht angreifen';
  if (t.boden) return t.f === 1 ? 'Grundschaden ×1' : `Konterfaktor ×${fmt(t.f, 2)}`;
  const g = [t.rang < 2 ? `${RANG[t.rang]} ×${fmt(t.f, 2)}` : 'Grundschaden ×1'];
  if (t.r > 1) g.push(`Schnellfeuer ${t.r}`);
  if (t.sf > 1) g.push(`Schildbrecher ×${fmt(t.sf, 2)}`);
  if (t.kan > 1) g.push(`${t.kan} Ziele je Runde`);
  if (t.verpufft) g.push('verpufft am Schild');
  return g.join(' · ');
}
const rundeText = (x, r) => x.boden ? `Runde ${r} (≈ ${fmt(r * BODEN_MIN / 60, 1)} h)` : `Runde ${r}`;

/* ---------- Zeilen ---------- */
function gfZeile(x) {
  const {A, B, urteil: u} = x;
  const einsatz = `${stueck(x.nA)} ${esc(A.name)} gegen ${stueck(x.nB)} ${esc(B.name)}`;
  const ausgang = x.endB && !x.endA ? `${esc(B.name)} vernichtet in ${rundeText(x, x.endB)}` : x.endA && !x.endB ? `${esc(A.name)} vernichtet in ${rundeText(x, x.endA)}`
    : x.endA && x.endB ? `beide vernichtet in ${rundeText(x, x.endA)}` : x.boden ? `nach ${fmt(x.runden)} Bodenrunden offen` : `nach ${x.runden} Runden kein Sieger – Angreifer kehrt heim`;
  const gruppe = B.typ === 'schiff' || B.typ === 'megaschiff' ? 'schiffe' : B.typ === 'boden' ? 'boden' : 'verteidigung';
  return `<li class="gf vt-${u.k}" data-gruppe="${gruppe}">
    <button type="button" class="gf-kopf" data-geh="${esc(B.id)}">${avatar(B)}<span class="gf-name"><b>${esc(B.name)}</b><small>${einsatz}</small></span><span class="gf-urteil">${u.t}</span></button>
    <div class="gf-zeilen">
      <div class="gf-balken gegner"><span class="gf-l">${trennK(B.name)} verliert</span><span class="gf-bar"><i data-st="width:${Math.max(x.verlustB > 0 ? 2 : 0, x.verlustB * 100)}%"></i></span><b>${pct(x.verlustB)}</b></div>
      <div class="gf-balken eigen"><span class="gf-l">${trennK(A.name)} verliert</span><span class="gf-bar"><i data-st="width:${Math.max(x.verlustA > 0 ? 2 : 0, x.verlustA * 100)}%"></i></span><b>${pct(x.verlustA)}</b></div>
      <p class="gf-satz">${ausgang}. <span>${esc(A.name)}: ${esc(grund(x.ab))}.</span> <span>${esc(B.name)}: ${esc(grund(x.ba))}.</span></p>
    </div>
    <details class="gf-detail"><summary>Alle Zahlen dieses Gefechts</summary>${detail(x)}</details></li>`;
}
function detail(x) {
  const {A, B, ab, ba} = x;
  const z = (t, q) => t.rang == null ? null : t;
  const zeile = (titel, f) => `<tr><th>${titel}</th><td>${f(ab, x.q0ab, B, A)}</td><td>${f(ba, x.q0ba, A, B)}</td></tr>`;
  const kann = (t, f) => t.rang == null ? '–' : f();
  const tab = `<div class="tabelle-roll"><table class="tabelle gf-tab"><thead><tr><th></th><th>${esc(A.name)} → ${esc(B.name)}</th><th>${esc(B.name)} → ${esc(A.name)}</th></tr></thead><tbody>
    ${zeile('Zielwahl', t => t.rang == null ? 'kann nicht angreifen' : t.boden ? 'Bodenkampf' : RANG[t.rang])}
    ${zeile('Faktor', t => kann(t, () => '×' + fmt(t.f, 2)))}
    ${zeile('Schaden je Treffer', t => kann(t, () => kurz(t.d)))}
    ${x.boden ? '' : zeile('Schnellfeuer', t => kann(t, () => t.r > 1 ? `${t.r} (Ø ${fmt(t.E, 1)} Schüsse)` : '–'))}
    ${x.boden ? '' : zeile('Ziele je Runde', t => kann(t, () => String(t.kan)))}
    ${x.boden ? '' : zeile('Schildbrecher', t => kann(t, () => t.sf > 1 ? '×' + fmt(t.sf, 2) : '–'))}
    ${x.boden ? '' : zeile('Schild des Ziels', t => kurz(t.S || 0))}
    ${zeile('Struktur des Ziels', t => kurz(t.H || 0))}
    ${x.boden ? '' : zeile('Verpufft am Schild', t => kann(t, () => t.verpufft ? 'ja (unter 1 %)' : 'nein'))}
    ${zeile('Treffer je Ziel, Runde 1', (t, q) => kann(t, () => fmt(q * (t.E || 1), 2)))}
    ${zeile('Gegner fällt in Runde 1', (t, q) => kann(t, () => pct(anteil(t, q))))}
    <tr><th>Einsatz (gleicher Wert)</th><td>${stueck(x.nA)} Stück · ${kurz(x.nA * wertVon(A))} Wert</td><td>${stueck(x.nB)} Stück · ${kurz(x.nB * wertVon(B))} Wert</td></tr>
    <tr><th>Übrig am Ende</th><td>${stueck(x.nA * x.a)} von ${stueck(x.nA)}</td><td>${stueck(x.nB * x.b)} von ${stueck(x.nB)}</td></tr>
  </tbody></table></div>`;
  const schritte = x.verlauf.length > 1 ? `<div class="gf-verlauf" aria-label="Verlauf je Runde">${x.verlauf.slice(1, 13).map((p, i) => `<div><span class="va" data-st="height:${p.a * 100}%"></span><span class="vb" data-st="height:${p.b * 100}%"></span><small>${i + 1}</small></div>`).join('')}</div>
    <p class="vt-hinweis"><span class="lg va"></span>${esc(A.name)} übrig <span class="lg vb"></span>${esc(B.name)} übrig – je Runde${x.verlauf.length > 13 ? ' (erste 12 Runden)' : ''}.</p>` : '';
  return tab + schritte;
}

/* ---------- Abschnitte ---------- */
function abschnitte(A, volk) {
  const vA = D.voelker[volk]?.regel, aus = [];
  if (FLOTTE.has(A.typ) || A.typ === 'boden') {
    const reihenfolge = {sg: 0, g: 1, n: 2, s: 3, ss: 4, x: 5};
    const liste = gegnerVon(A).map(B => gefecht(A, B, A.typ === 'boden' ? null : vA, volk))
      .sort((p, q) => reihenfolge[p.urteil.k] - reihenfolge[q.urteil.k] || (q.verlustB - q.verlustA) - (p.verlustB - p.verlustA) || p.B.name.localeCompare(q.B.name));
    // Kampfeinheiten vor zivilen Schiffen: „stark gegen Bergungsschiff“ sagt nichts über Konter (Karl 25.10.2026: schlüssig).
    const zivil = x => (x.B.typ === 'schiff' && ZIVIL.has(x.B.einheitklasse)) || (x.B.werte?.angriff || 0) <= 0 ? 1 : 0;
    const stark = liste.filter(x => ['sg', 'g'].includes(x.urteil.k)).sort((p, q) => zivil(p) - zivil(q) || q.verlustB - p.verlustB || p.verlustA - q.verlustA).slice(0, 6);
    const gefahr = liste.filter(x => ['ss', 's'].includes(x.urteil.k)).sort((p, q) => zivil(p) - zivil(q) || q.verlustA - p.verlustA || p.verlustB - q.verlustB).slice(0, 6);
    const lesen = `<p class="gf-lesen">Jede Zeile ist ein Gefecht, in dem beide Seiten <b>gleich viel Rohstoffwert</b> einsetzen (bis ${A.typ === 'boden' ? '48 Stunden Belagerung' : RUNDEN_FLOTTE + ' Kampfrunden'}). <span class="g">Grün</span> = ${esc(A.name)} gewinnt, <span class="r">Rot</span> = der Gegner gewinnt. Die Balken zeigen, wie viel jede Seite verliert.</p>`;
    if (A.typ !== 'boden') aus.push(['Kampfprofil', profil(A, vA), 'wen sie zuerst angreift']);
    else aus.push(['Bodenkampf', bodenProfil(A), `Runde alle ${fmt(BODEN_MIN)} Spielminuten`]);
    aus.push([`${esc(A.name)} ist stark gegen`, lesen + (stark.length ? `<ol class="gf-liste">${stark.map(gfZeile).join('')}</ol>` : `<p class="vt-hinweis">Gegen keine Einheit überlegen – ${A.werte?.angriff > 0 ? 'eine Unterstützungs- oder Masseneinheit.' : 'unbewaffnet.'}</p>`)]);
    aus.push([`Gefährlich für ${esc(A.name)}`, lesen + (gefahr.length ? `<ol class="gf-liste">${gefahr.map(gfZeile).join('')}</ol>` : '<p class="vt-hinweis">Keine Einheit ist bei gleichem Einsatz überlegen.</p>')]);
    aus.push(['Alle Gegner im Überblick', uebersicht(A, liste), `${liste.length} Gefechte`]);
    const sf = schnellfeuer(A); if (sf) aus.push(['Schnellfeuer', sf, MEHRFACH ? 'mehrere Schüsse; nach einem Abschuss springt die Salve auf das nächste Ziel derselben Art' : 'mehrere Schüsse auf dasselbe Ziel']);
    const ein = schnellfeuerEin(A); if (ein) aus.push(['Schnellfeuer gegen diese Einheit', ein]);
    aus.push(['So wird gerechnet', methode(A)]);
  }
  if (A.typ === 'rakete') aus.push(['Raketenziele', raketen(A, vA), 'Wirkung je Rakete']);
  return aus;
}
function uebersicht(A, liste) {
  const z = URTEILE.map(u => ({...u, n: liste.filter(x => x.urteil.k === u.k).length}));
  const band = `<div class="vt-band" role="img" aria-label="${z.map(s => `${s.n} ${s.t}`).join(', ')}">${z.filter(s => s.n).map(s => `<span class="vt-${s.k}" data-st="flex:${s.n}"><b>${s.n}</b></span>`).join('')}</div>
    <ul class="gf-legende">${z.map(s => `<li class="vt-${s.k}"><i></i><b>${s.n}× ${s.t}</b><small>${s.k === 'x' ? 'keine Seite richtet viel aus' : esc(A.name) + ' ' + s.lang}</small></li>`).join('')}</ul>
    <div class="vt-streifen" role="list" aria-label="Gegner von stark nach schwach">${liste.map(x => `<button type="button" role="listitem" class="vt-av vt-${x.urteil.k}" data-geh="${esc(x.B.id)}" title="${esc(x.B.name)}: ${x.urteil.t}">${avatar(x.B)}</button>`).join('')}</div>`;
  const filter = A.typ === 'boden' ? '' : `<div class="vt-filter" role="tablist"><button type="button" role="tab" data-vtf="alle" aria-selected="true">Alle</button><button type="button" role="tab" data-vtf="schiffe" aria-selected="false">Schiffe</button><button type="button" role="tab" data-vtf="verteidigung" aria-selected="false">Verteidigung</button></div>`;
  const gruppen = z.filter(s => s.n).map(s => `<details class="gf-gruppe vt-${s.k}"><summary><i></i>${s.t}<small>${s.n}</small></summary><ol class="gf-liste">${liste.filter(x => x.urteil.k === s.k).map(gfZeile).join('')}</ol></details>`).join('');
  return band + `<div class="gf-alle">${filter}${gruppen}</div>`;
}
function profil(A, vA) {
  const k = A.kampf || {};
  if (!(A.werte?.angriff > 0)) return '<p class="vt-hinweis">Unbewaffnet – kann nicht angreifen und nimmt nur Schaden.</p>';
  const chips = (ids, rg) => (ids || []).map(id => K[id] ? `<button type="button" class="pchip" data-geh="${esc(id)}">${avatar(K[id])}<span><b>${esc(K[id].name)}</b><small>${rg === 0 ? 'wird zuerst angegriffen' : 'danach'}${k.schnellfeuer?.[id] > 1 ? ` · Schnellfeuer ${k.schnellfeuer[id]}` : ''}</small></span><em>×${fmt(k.schadensfaktoren?.[id] ?? (rg === 0 ? 2 : 1.5), 2)}</em></button>` : '').join('');
  const reserve = (k.reserve || []).filter(id => K[id]);
  return `<div class="kp-zahlen"><div><b>${k.ziele_je_runde || A.ziele || 1}</b><span>verschiedene Ziele je Runde</span></div><div><b>${kurz(Math.floor(A.werte.angriff * (vA?.waffen ?? 1) + 1e-9))}</b><span>Schaden je Treffer${vA && vA.waffen !== 1 ? ' (mit Volk)' : ''}</span></div>
    <div><b>×${fmt(k.schildfaktor || 1, 2)}</b><span>Schaden auf Schilde${(k.schildfaktor || 1) > 1 ? ' – Schildbrecher' : ''}</span></div><div><b>${Object.keys(k.schnellfeuer || {}).length}</b><span>Ziele mit Schnellfeuer</span></div></div>
    <p class="vt-hinweis">Der Faktor rechts (×2, ×1,5) ist der Schadensbonus gegen dieses Ziel.</p>
    ${k.primaer?.length ? `<h4 class="kp-h">Primärziele – werden zuerst angegriffen</h4><div class="pchips">${chips(k.primaer, 0)}</div>` : ''}
    ${k.sekundaer?.length ? `<h4 class="kp-h">Sekundärziele – wenn keine Primärziele übrig sind</h4><div class="pchips">${chips(k.sekundaer, 1)}</div>` : ''}
    <details class="vt-mehr"><summary>Reserveziele: ${reserve.length} weitere mit Grundschaden ×1</summary><div class="mini-av">${reserve.map(id => `<button type="button" data-geh="${esc(id)}" title="${esc(K[id].name)}">${avatar(K[id])}</button>`).join('')}</div></details>
    ${(k.nicht_erfassbar || []).length ? `<p class="vt-hinweis">Nicht erfassbar: ${k.nicht_erfassbar.map(id => esc(name(id))).join(', ')}</p>` : ''}
    ${k.verteilung ? `<details class="vt-mehr"><summary>Zielregel im Wortlaut</summary><p class="vt-hinweis">${esc(k.verteilung)} Der Kern wählt in jedem Kampf nach dieser Rangfolge (kampf.rs, feuern_doktrin).</p></details>` : ''}`;
}
function bodenProfil(A) {
  const reihe = Object.entries(A.konter || {}).map(([id, p]) => ({id, p})).sort((a, b) => b.p - a.p);
  const stufeP = p => p >= 2000 ? 'sg' : p > 1000 ? 'g' : p === 1000 ? 'n' : p >= 500 ? 's' : 'ss';
  return `<div class="kp-zahlen"><div><b>${fmt(A.werte.angriff, 2)}</b><span>Schaden je Schuss</span></div><div><b>${fmt(A.werte.struktur, 2)}</b><span>Struktur</span></div>
    <div><b>${A.werte.transportplaetze ?? '–'}</b><span>Transportplätze</span></div><div><b>${A.werte.angriff && A.werte.transportplaetze ? fmt(A.werte.angriff / A.werte.transportplaetze, 1) : '–'}</b><span>Schaden je Platz</span></div></div>
    <h4 class="kp-h">Schadensfaktor dieser Einheit gegen jeden Bodentyp</h4>
    <div class="kmatrix">${reihe.map(x => `<button type="button" class="km vt-${stufeP(x.p)}" data-geh="${esc(x.id)}">${K[x.id] ? avatar(K[x.id]) : ''}<span>${esc(name(x.id))}</span><b>×${fmt(x.p / 1000, 2)}</b></button>`).join('')}</div>
    <p class="vt-hinweis">×2 = doppelter Schaden, ×½ = halber Schaden. Bodenrunden mit gleichzeitigem Feuer; gilt für alle Völker gleich.</p>`;
}
function schnellfeuer(A) {
  const sf = Object.entries(A.kampf?.schnellfeuer || {}).filter(([id, r]) => K[id] && r > 1).sort((a, b) => b[1] - a[1]);
  const inaktiv = Object.entries(A.schnellfeuer || {}).filter(([id, r]) => r > 1 && !(A.kampf?.schnellfeuer || {})[id]);
  if (!sf.length && !inaktiv.length) return '';
  const max = Math.max(2, ...sf.map(([, r]) => r));
  return `<p class="vt-hinweis">Schnellfeuer r: Nach jedem Treffer schießt die Einheit mit Wahrscheinlichkeit (r−1)/r noch einmal auf dasselbe Ziel – im Mittel r Schüsse, höchstens 64. Gilt nur gegen Primär- und Sekundärziele.</p>
    <ol class="sf-liste">${sf.map(([id, r]) => { const E = r * (1 - Math.pow(1 - 1 / r, 64));
    return `<li data-geh="${esc(id)}">${avatar(K[id])}<span><b>${esc(K[id].name)}</b><small>Folgeschuss ${fmt((r - 1) / r * 100, 1)} % · Ø ${fmt(E, 1)} Schüsse je Treffer</small></span><span class="sf-balken"><i data-st="width:${Math.max(4, Math.log(r) / Math.log(max) * 100)}%"></i></span><em>${r}</em></li>`; }).join('')}</ol>
    ${inaktiv.length ? `<p class="vt-hinweis warn">Im Datenblatt, aber im Kampf ohne Wirkung (kein Primär- oder Sekundärziel): ${inaktiv.map(([id, r]) => `${esc(name(id))} ${r}`).join(', ')}.</p>` : ''}`;
}
function schnellfeuerEin(A) {
  const q = Object.values(K).filter(B => (B.kampf?.schnellfeuer?.[A.id] || 0) > 1).map(B => [B, B.kampf.schnellfeuer[A.id]]).sort((a, b) => b[1] - a[1]);
  return q.length ? `<p class="vt-hinweis">Diese Gegner schießen mehrfach auf ${esc(A.name)}:</p><div class="kchips">${q.map(([B, r]) => `<button type="button" class="kchip gegen" data-geh="${esc(B.id)}">${avatar(B)}<span><b>${esc(B.name)}</b><small>Folgeschuss ${fmt((r - 1) / r * 100, 1)} % · Ø ${fmt(r * (1 - Math.pow(1 - 1 / r, 64)), 1)} Schüsse</small></span><em>${r}</em></button>`).join('')}</div>` : '';
}
function raketen(A, vA) {
  const z = A.raketenziel || {}, f = z.schadensfaktoren || {}, je = z.ziele_je_rakete || 1, umgeht = /umgeht Schilde/i.test(A.wirkung || '');
  const ziele = Object.keys(f).filter(id => K[id]);
  const kopf = `<div class="kp-zahlen"><div><b>${kurz(Math.floor((A.werte.schaden || 0) * (vA?.waffen ?? 1) + 1e-9))}</b><span>Schaden je Rakete</span></div><div><b>${je}</b><span>Ziele je Rakete höchstens</span></div><div><b>${A.werte.abfangbedarf || '–'}</b><span>Abfangraketen nötig</span></div><div><b>${umgeht ? 'ja' : 'nein'}</b><span>umgeht Schilde</span></div></div>
    <p class="vt-hinweis">Primär: ${esc(z.primaer || '–')}. ${esc(z.sekundaer || '')}</p>`;
  if (!ziele.length) return kopf + (z.verteilung ? `<p class="vt-hinweis">${esc(z.verteilung)}</p>` : '');
  const sch = Math.floor((A.werte.schaden || 0) * (vA?.waffen ?? 1) + 1e-9);
  const liste = ziele.map(id => { const B = K[id], H = (B.werte.struktur || 1) + (umgeht ? 0 : B.werte.schild || 0), dmg = sch * f[id];
    const n = Math.min(je, dmg / H), X = n * B.wert / A.wert; const k = X >= 2 ? 'sg' : X >= 1.25 ? 'g' : X >= 0.8 ? 'n' : X >= 0.5 ? 's' : 'ss';
    return {B, n, X, f: f[id], k}; }).sort((a, b) => b.X - a.X);
  return kopf + `<ol class="gf-liste">${liste.map(x => `<li class="gf vt-${x.k}"><button type="button" class="gf-kopf" data-geh="${esc(x.B.id)}">${avatar(x.B)}<span class="gf-name"><b>${esc(x.B.name)}</b><small>Faktor ×${fmt(x.f, 2)} · eine Rakete zerstört ≈ ${fmt(x.n, 2)} Stück (höchstens ${je})</small></span><span class="gf-urteil">${x.X >= 1 ? 'lohnt sich' : 'teurer als Schaden'}</span></button>
      <div class="gf-zeilen"><div class="gf-balken gegner"><span class="gf-l">Zerstörter Wert je Raketenwert</span><span class="gf-bar"><i data-st="width:${Math.min(100, x.X / 4 * 100)}%"></i></span><b>${fmt(x.X, 2)}×</b></div></div></li>`).join('')}</ol>
    <p class="vt-hinweis">„2×“ heißt: Die Rakete zerstört doppelt so viel Rohstoffwert, wie sie kostet. ${z.verteilung ? esc(z.verteilung) : ''}</p>`;
}
function methode(A) {
  const gew = Object.entries(D.gewichte || {}).map(([g, w]) => `${esc(name(g))} ${fmt(w, 1)}`).join(', ');
  if (A.typ === 'boden') return `<p class="vt-hinweis">Beide Seiten setzen gleich viel Rohstoffwert ein (Gewichte der Punktwertung: ${gew}). Je Bodenrunde schießt jede Einheit einmal: Schaden = Angriff × Konterfaktor, kein Schild, höchstens ein Abschuss je Schuss; alle feuern gleichzeitig. Die Rechnung läuft Runde für Runde, bis eine Seite fällt oder 48 Stunden Belagerung (192 Runden) vorbei sind. Urteil aus dem Verhältnis der Verluste: ab 3× klar überlegen, ab 1,5× überlegen, unter ⅓ klar unterlegen; richten beide Seiten unter 2 % aus, heißt es „kaum Wirkung“.</p>`;
  return `<p class="vt-hinweis">Beide Seiten setzen gleich viel Rohstoffwert ein (Gewichte der Punktwertung: ${gew}). Jede Einheit greift je Runde so viele verschiedene Ziele an, wie ihr Profil erlaubt, mit vollem Schaden × Zielfaktor (Primärziel ×2, Sekundärziel ×1,5 oder eigener Wert, sonst ×1) und Volksbonus des gewählten Volkes; Gegner mit Grundwerten.</p>
    <p class="vt-hinweis">Der Schild fängt je Runde zuerst ab und erneuert sich; Schildbrecher wirken stärker auf Schilde. Ein Treffer unter ${fmt(VERPUFF * 100)} % des Schildwerts verpufft. Schnellfeuer r bringt im Mittel r Schüsse${MEHRFACH ? '; nach einem Abschuss springt die Salve auf das nächste Ziel derselben Art (Mehrfachfeuer)' : '. Ein Zielkanal zerstört höchstens eine Einheit'}. ${D.gefechte ? `Die Ergebnisse stammen aus der Kampfengine des Spiels: ${RUNDEN_FLOTTE} Runden, Mittel aus 12 Zufallsgefechten je Paar und Volk, ohne Forschung.` : `Die Rechnung läuft Runde für Runde bis zur Vernichtung oder zum Kampfende nach ${RUNDEN_FLOTTE} Runden.`}</p>
    <p class="vt-hinweis">Urteil aus dem Verhältnis der Verluste: ab 3× klar überlegen, ab 1,5× überlegen, zwischen ⅔ und 1,5 ausgeglichen, unter ⅓ klar unterlegen; verlieren beide unter 2 %, heißt es „kaum Wirkung“. Nicht eingerechnet: ${D.gefechte ? '' : `Explosionsregel unter ${fmt((D.kampfregeln?.explosion_unter ?? .7) * 100)} % Struktur, `}Forschungsstufen, gemischte Flotten, Verteidigungs-Wiederaufbau.</p>`;
}
return {abschnitte, gefecht, treffer, URTEILE};
};
