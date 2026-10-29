'use strict';
// Gesamt-Technologiebaum im Spiel. Knopf in „Forschung“ und „Technologiebaum“ öffnet ein Vollbild-Overlay.
// Läuft im Shadow DOM (eigene Gestaltung, keine Kollision mit dem Spiel-CSS) und hält die CSP ein:
// keine Inline-Styles/-Skripte, Dateien werden erst beim ersten Öffnen nachgeladen (techbaum/*.js, ~1,5 MB).
(() => {
  let host = null, api = null, laden = null;
  const skript = src => new Promise((ok, fehler) => {
    if (document.querySelector(`script[data-techbaum="${src}"]`)) return ok();
    const s = document.createElement('script'); s.src = src; s.dataset.techbaum = src; s.onload = ok; s.onerror = () => fehler(new Error(src));
    document.head.append(s);
  });
  // Spielerstand aus dem Spiel (view ist eine lexikalische Globale aus app.js, nicht window.view).
  function spieler() {
    try {
      if (typeof view === 'undefined' || !view || !view.volk) return null;
      const planeten = view.planeten || [], heimat = planeten.find(p => p.heimat) || planeten[0] || {}, gebaeude = {};
      for (const p of planeten) for (const [k, n] of Object.entries(p.gebaeude || {})) gebaeude[k] = Math.max(gebaeude[k] || 0, n || 0);
      return {volk: view.volk, stufe: view.stufe || 1, forschung: {...(view.forschung?.stufen || {})}, gebaeude, heimat: {...(heimat.gebaeude || {})},
        bevoelkerung: planeten.reduce((s, p) => s + (p.bevoelkerung || 0), 0), kolonien: view.kolonien?.anzahl ?? Math.max(0, planeten.length - 1),
        stabilitaet: heimat.stabilitaet ?? 0, konsum: heimat.konsum_deckung ?? 0, name: view.name || ''};
    } catch { return null; }
  }
  // Aufträge aus dem Dossier. Der Baum läuft im Shadow DOM: Klicks dort erreichen die Spiel-Handler auf document
  // (data-build/data-research in app.js, data-make in game-ui.js, data-ground-build in mobile035.js) nur als Klick
  // auf den Wirt. Darum klickt der Baum einen unsichtbaren Befehlsknopf im Light DOM – dieselben Handler wie die
  // Kacheln der Bauseiten, mit denselben Prüfungen, Meldungen und dem anschließenden refresh().
  // view und rules sind lexikalische Globale (let) aus app.js; kein eval (CSP ohne unsafe-eval).
  const spielView = () => typeof view === 'undefined' ? null : view, spielRegeln = () => typeof rules === 'undefined' ? null : rules;
  function katVon(id, typ) {
    const c = spielRegeln()?.catalog; if (!c) return null;
    if (typ === 'gebaeude' && c.gebaeude?.[id]) return 'buildings';
    if (typ === 'forschung' && c.forschung?.[id]) return 'research';
    if (typ === 'boden') return 'ground';
    const e = c.einheiten?.[id]; if (e && id !== 'weltentruemmer' && ['schiff', 'verteidigung', 'schild', 'satellit'].includes(typ)) return e.schiff ? 'ships' : 'defenses';
    return null;
  }
  const SEITE_TEXT = {versorgung: 'Versorgung', gebaeude: 'Anlagen', forschen: 'Forschung', werft: 'Schiffswerft', verteidigung: 'Verteidigung', truppen: 'Truppen'};
  function datensatz(id, typ) {
    const v = spielView(), kat = katVon(id, typ); if (!v || !kat || typeof currentPlanet !== 'function') return null;
    const p = currentPlanet(); if (!p) return null;
    if (kat !== 'ground') { const x = typeof itemRecords === 'function' ? itemRecords(kat, p).find(r => r.key === id) : null; return x ? {kat, p, x} : null; }
    const d = p.bodentruppen, i = d?.typen?.indexOf(id) ?? -1; if (i < 0) return null;
    const needs = [], gueter = p.industrie?.gueter || [];
    if (d.ab_stufe[i] > v.stufe) needs.push('Zivilisationsstufe ' + d.ab_stufe[i]);
    if ((p.gebaeude.kaserne || 0) < d.kaserne[i]) needs.push('Kaserne ' + d.kaserne[i]);
    const cost = Object.fromEntries(gueter.map((g, n) => [g, (d.kosten_aktuell_milli?.[i]?.[n] || 0) / 1000]).filter(([, n]) => n > 0));
    return {kat, p, x: {key: id, level: d.bestand[i] || 0, cost, needs, stock: p.bestand, job: (d.auftraege || []).find(a => a.art === i) || null}};
  }
  function info(id, typ) {
    try {
      const r = datensatz(id, typ); if (!r) return null;
      const {kat, p, x} = r, v = spielView(), heim = v.planeten.find(q => q.heimat) || v.planeten[0];
      const ort = kat === 'research' ? 'Reichsweit, Labore der Heimatwelt' : `${window.Planetenname ? Planetenname.name(p) : p.name || (p.heimat ? 'Heimatwelt' : 'Kolonie')} [${p.koord}]`;
      const seite = window.Bauseite?.seiteVon?.(kat, id), seiteText = seite ? `Im Spiel: ${SEITE_TEXT[seite] || 'öffnen'}` : 'Im Spiel öffnen';
      let auftrag = null;
      if (kat === 'buildings' || kat === 'research') {
        const art = kat === 'buildings' ? 'bau' : 'forschung', q = v.forschung?.warteschlangen?.find(q => q.art === art && (art === 'forschung' || q.planet === p.koord));
        const i = q?.jobs?.findIndex(j => j.key === id) ?? -1;
        if (i >= 0) { const j = q.jobs[i]; auftrag = (j.aktiv ? 'In Arbeit' : 'In der Warteschlange · Platz ' + (i + 1)) + (j.stufe ? ' · Stufe ' + j.stufe : ''); }
      } else if (x.job) auftrag = kat === 'ground' ? 'In Ausbildung' : 'In Fertigung' + (x.job.rest ? ` · noch ${x.job.rest} Stück` : '');
      const lager = x.stock || (kat === 'research' ? heim : p).bestand || {};
      const bezahlbar = Object.entries(x.cost || {}).every(([g, n]) => (lager[g] || 0) >= n);
      const ziel = x.nextLevel || x.level + 1, menge = !['buildings', 'research'].includes(kat);
      const wort = kat === 'buildings' ? (x.level ? `Ausbauen auf Stufe ${ziel}` : 'Bauen') : kat === 'research' ? (x.level ? `Forschen · Stufe ${ziel}` : 'Forschen') : kat === 'ground' ? 'Ausbilden' : 'Fertigen';
      const max = kat === 'ships' || kat === 'defenses' ? (typeof maxBuildable === 'function' ? maxBuildable(x) : 1) : kat === 'ground' ? Math.max(0, Math.min(1000, ...Object.entries(x.cost).map(([g, n]) => Math.floor((lager[g] || 0) / n)))) : 1;
      return {kat, wort, gesperrt: x.needs || [], lesen: typeof gameReadOnly === 'function' ? gameReadOnly() || '' : '', auftrag, kosten: x.cost, bezahlbar, vormerken: !menge, menge, max, ort, seiteText,
        hinweis: menge ? `Bestand ${x.level}` : x.level ? `jetzt Stufe ${x.level}` : '',
        wirkung: kat === 'buildings' && typeof ausbauWirkungHtml === 'function' ? ausbauWirkungHtml(ausbauWirkung(x, p)) : ''};
    } catch (e) { console.warn('Techbaum: Spielstand', e); return null; }
  }
  async function ausfuehren(id, typ, n = 1) {
    const r = datensatz(id, typ); if (!r) return {ok: false, text: 'Im Spiel nicht verfügbar'};
    const {kat, p} = r; let el, knopf;
    if (kat === 'ground') {
      el = document.createElement('form'); el.dataset.groundBuild = id; el.dataset.planet = p.koord;
      el.addEventListener('submit', e => e.preventDefault()); // nie echt absenden, falls der Handler fehlt
      const zahl = document.createElement('input'); zahl.name = 'anzahl'; zahl.type = 'number'; zahl.value = String(n);
      knopf = document.createElement('button'); knopf.type = 'submit'; el.append(zahl, knopf);
    } else {
      el = knopf = document.createElement('button'); el.type = 'button';
      if (kat === 'buildings') { el.dataset.build = id; el.dataset.planet = p.koord; }
      else if (kat === 'research') el.dataset.research = id;
      else { el.dataset.make = id; el.dataset.orderQuantity = String(n); }
    }
    el.hidden = true; el.className = 'techbaum-befehl'; document.body.append(el);
    // Meldung des Spiels mitlesen (show() aus app.js ist eine Fensterfunktion), danach unverändert weiterreichen.
    const alt = window.show; let meldung = null;
    if (typeof alt === 'function') window.show = function (s, fehler) { meldung = {text: String(s), fehler: !!fehler}; return alt.apply(this, arguments); };
    try {
      if (kat === 'ground') el.requestSubmit(knopf); else el.click();
      // data-build/-research/-ground-build tragen _busy bis nach refresh(); data-make nicht – dort auf Meldung und neuen Auftrag warten.
      const warte = () => new Promise(r => setTimeout(r, 80)), busy = () => el._busy || knopf._busy, mitFlag = !['ships', 'defenses'].includes(kat);
      const bis = Date.now() + 20000; await warte();
      while (!meldung && Date.now() < bis && (!mitFlag || busy())) await warte();
      const bis2 = Date.now() + 5000;
      while (Date.now() < bis2 && (busy() || !mitFlag && meldung && !meldung.fehler && !info(id, typ)?.auftrag)) await warte();
    } finally { if (window.show !== alt && alt) window.show = alt; el.remove(); }
    const danach = info(id, typ);
    const ok = meldung ? !meldung.fehler : !!danach?.auftrag;
    const lesbar = x => typeof meldungLesbar === 'function' ? meldungLesbar(x) : x;
    return {ok, text: meldung ? lesbar(meldung.text) : ok ? 'Auftrag erteilt' : 'Kein Auftrag entstanden'};
  }
  function imSpielOeffnen(id, typ) {
    const kat = katVon(id, typ), seite = kat && window.Bauseite?.seiteVon?.(kat, id); if (!seite || typeof navigateGame !== 'function') return;
    rueckweg = null; schliessen(); navigateGame(seite); window.Bauseite.oeffne?.(seite, id); window.scrollTo({top: 0, behavior: 'instant'});
  }
  const spiel = {info, ausfuehren, oeffnen: imSpielOeffnen};

  // Kräfteatlas (tools/konter_matrix/uebersicht.py → kraefteatlas/): wer schlägt wen, gemessen mit der Kampfengine bei gleichem
  // Ressourcenwert – Matrix, Duell-Bilanz, jedes Gefecht im Detail. Eigenes Vollbild über dem Spiel oder über dem Baum, Shadow DOM,
  // CSP-sicher (data-st → element.style); Dateien erst beim ersten Öffnen (~2,7 MB), der kleine Index mit dem Baum.
  const Atlas = (() => {
    let ahost = null, aapi = null, aladen = null, index = null, spielVerdeckt = false;
    const istOffen = () => !!ahost && ahost.style.display !== 'none';
    const indexLaden = () => index || (index = skript('kraefteatlas/index.js').then(() => window.KRAEFTEATLAS_INDEX || null, () => (index = null)));
    const kennt = id => !!window.KRAEFTEATLAS_INDEX?.ids?.includes(id);
    function verbergen() {
      if (!ahost) return; ahost.style.display = 'none';
      if (!offen()) { document.documentElement.style.overflow = ''; if (spielVerdeckt) delete document.body.dataset.techbaumOffen; }
      spielVerdeckt = false;
    }
    function schliessen() { if (!istOffen()) return; verbergen(); if (history.state?.kraefteatlas) history.back(); }
    async function oeffnen(id, knopf) {
      if (!aladen) aladen = (async () => { await skript('kraefteatlas/index.js'); await skript('kraefteatlas/daten.js'); await skript('kraefteatlas/atlas.js'); })();
      if (knopf) { knopf.disabled = true; knopf.dataset.text = knopf.textContent; knopf.textContent = 'Kräfteatlas wird geladen …'; }
      try { await aladen; }
      catch (e) { aladen = null; if (knopf) knopf.textContent = 'Laden fehlgeschlagen – erneut versuchen'; return false; }
      finally { if (knopf) { knopf.disabled = false; if (knopf.textContent.startsWith('Kräfteatlas wird')) knopf.textContent = knopf.dataset.text; } }
      if (!ahost) {
        ahost = document.createElement('div'); ahost.id = 'kraefteatlas-overlay';
        ahost.setAttribute('role', 'dialog'); ahost.setAttribute('aria-modal', 'true'); ahost.setAttribute('aria-label', 'Kräfteatlas: wer schlägt wen');
        Object.assign(ahost.style, {position: 'fixed', inset: '0', zIndex: '2147483100', display: 'none'});
        document.body.append(ahost);
        const wurzel = ahost.attachShadow({mode: 'open'}), css = document.createElement('link');
        css.rel = 'stylesheet'; css.href = 'kraefteatlas/atlas.css';
        const geladen = new Promise(r => { css.onload = r; css.onerror = r; });
        wurzel.append(css); ahost.style.display = ''; await geladen;
        aapi = window.KraefteatlasStart({wurzel, basis: '', eingebettet: true, vollbild: true, volk: spielView()?.volk, schliessen});
      }
      // Das Spiel unter dem Vollbild nicht weiter malen (rahmen.css, wie beim Baum).
      if (!document.body.dataset.techbaumOffen) { document.body.dataset.techbaumOffen = '1'; spielVerdeckt = true; }
      ahost.style.display = ''; document.documentElement.style.overflow = 'hidden'; androidZurueck();
      if (!history.state?.kraefteatlas) history.pushState({...(history.state || {}), kraefteatlas: true}, '');
      if (id) aapi?.zeige?.(id);
      ahost.shadowRoot.querySelector('.atlas-zu')?.focus({preventScroll: true});
      return true;
    }
    function zurueck() { if (!istOffen()) return false; if (!aapi?.zurueck?.()) schliessen(); return true; }
    addEventListener('popstate', () => { if (!istOffen()) return; if (aapi?.zurueck?.()) history.pushState({...(history.state || {}), kraefteatlas: true}, ''); else verbergen(); });
    return {oeffnen, schliessen, zurueck, offen: istOffen, kennt, indexLaden, zeige: id => oeffnen(id)};
  })();

  const offen = () => !!host && host.style.display !== 'none';
  // Rückweg: kam der Baum aus einer Objektansicht (Techinfo-Dialog), öffnet das Schließen wieder dieses Objekt.
  let rueckweg = null;
  function verbergen() { if (!host) return; host.style.display = 'none'; document.documentElement.style.overflow = ''; delete document.body.dataset.techbaumOffen; const r = rueckweg; rueckweg = null; if (r) setTimeout(r, 0); }
  function schliessen() { if (!offen()) return; verbergen(); if (history.state?.techbaum) history.back(); }
  async function oeffnen(knopf) {
    if (!laden) laden = (async () => { await skript('techbaum/daten.js'); await skript('techbaum/kampf.js'); await skript('techbaum/techbaum.js'); await Atlas.indexLaden(); })();
    if (knopf) { knopf.disabled = true; knopf.dataset.text = knopf.textContent; knopf.textContent = 'Technologiebaum wird geladen …'; }
    try { await laden; }
    catch (e) { laden = null; if (knopf) knopf.textContent = 'Laden fehlgeschlagen – erneut versuchen'; return; }
    finally { if (knopf) { knopf.disabled = false; if (knopf.textContent.startsWith('Technologiebaum wird')) knopf.textContent = knopf.dataset.text; } }
    if (!host) {
      host = document.createElement('div'); host.id = 'techbaum-overlay';
      host.setAttribute('role', 'dialog'); host.setAttribute('aria-modal', 'true'); host.setAttribute('aria-label', 'Gesamt-Technologiebaum');
      Object.assign(host.style, {position: 'fixed', inset: '0', zIndex: '2147483000', display: 'none'});
      document.body.append(host);
      const wurzel = host.attachShadow({mode: 'open'}), css = document.createElement('link');
      css.rel = 'stylesheet'; css.href = 'techbaum/techbaum.css';
      const geladen = new Promise(r => { css.onload = r; css.onerror = r; });
      wurzel.append(css); host.style.display = ''; await geladen;
      api = window.TechbaumStart({wurzel, basis: '', eingebettet: true, spieler, aktiv: () => offen() && !Atlas.offen(), schliessen, spiel, atlas: Atlas});
    }
    // Das Spiel unter dem Vollbild-Baum nicht mehr malen (rahmen.css): es zeichnete weiter ~800 Änderungen je 10 s.
    host.style.display = ''; document.documentElement.style.overflow = 'hidden'; document.body.dataset.techbaumOffen = '1'; androidZurueck();
    if (!history.state?.techbaum) history.pushState({...(history.state || {}), techbaum: true}, '');
    api?.geoeffnet ? api.geoeffnet() : api?.standAktualisieren?.();
  }
  // Zurück (Browser und Android-App): erst Dossier/Plan im Techbaum schließen, dann das Overlay.
  function zurueck() { if (Atlas.zurueck()) return true; if (!offen()) return false; if (!api?.zurueck?.()) schliessen(); return true; }
  addEventListener('popstate', () => { if (!offen() || Atlas.offen()) return; if (api?.zurueck?.()) history.pushState({...(history.state || {}), techbaum: true}, ''); else verbergen(); });
  // Android-Zurück läuft über Rahmen.zurueck() (rahmen.js): Haken meldet true nur, wenn er wirklich etwas geschlossen hat.
  // Ohne Rahmen (ältere Seiten) bleibt der Umweg über SternMobil.zurueck.
  let hakenDrin = false;
  function androidZurueck() {
    if (window.Rahmen?.zurueckHaken) { if (!hakenDrin) { window.Rahmen.zurueckHaken.push(() => zurueck()); hakenDrin = true; } return; }
    const m = window.SternMobil; if (!m || m.__techbaum) return;
    const alt = m.zurueck; m.zurueck = function () { return zurueck() || (alt ? alt.apply(this, arguments) : false); }; m.__techbaum = true;
  }
  function knopf() {
    const b = document.createElement('button');
    b.type = 'button'; b.className = 'secondary techbaum-knopf'; b.textContent = 'Gesamter Technologiebaum ⤢';
    b.title = 'Alle Bäume auf einer Karte: Gebäude, Forschung, Schiffe, Verteidigung, Truppen, Raketen, Mond, Missionen, Diplomatie – mit deinem Stand';
    b.addEventListener('click', () => oeffnen(b));
    return b;
  }
  function atlasKnopf() {
    const b = document.createElement('button');
    b.type = 'button'; b.className = 'secondary techbaum-knopf kraefteatlas-knopf'; b.textContent = 'Kräfteatlas: wer schlägt wen ⤢';
    b.title = 'Gemessen mit der Kampfengine: jede Kampfeinheit gegen jede bei gleichem Wert – Konter, Duell-Bilanz, Völker, Raketen, Bodentruppen';
    b.addEventListener('click', () => Atlas.oeffnen(null, b));
    return b;
  }
  // Seite „Technologiebaum“ (Karl 20.10.: „immer den neuen nutzen“): statt des alten Wegplaners (progression.js, per Klasse
  // tb-einstieg in dossier.css ausgeblendet) ein großer Einstieg in den Gesamtbaum. Die Seite bleibt eine normale Seite –
  // kein automatisches Öffnen des Overlays (es läge über allen weiteren Menüklicks).
  function einstieg() {
    const karte = document.createElement('div'); karte.className = 'tb-einstieg-karte';
    const auge = document.createElement('p'); auge.className = 'tb-augenbraue'; auge.textContent = 'Gesamter Technologiebaum';
    const titel = document.createElement('h3'); titel.textContent = 'Alle Bäume auf einer Karte – mit deinem Stand';
    const text = document.createElement('p');
    text.textContent = 'Gebäude, Forschung, Schiffe, Verteidigung, Truppen, Raketen, Mond und Diplomatie. Eintrag antippen zeigt Bedingungen, Kosten und Bauen; „Weg“ plant den Pfad zu jedem Ziel, auch zur nächsten Zivilisationsstufe.';
    const b = knopf(); b.textContent = 'Technologiebaum öffnen ⤢';
    karte.append(auge, titel, text, b, atlasKnopf());
    return karte;
  }
  function einhaengen() {
    for (const id of ['forschen', 'freischaltungen']) {
      const sektion = document.getElementById(id), h2 = sektion?.querySelector(':scope > h2');
      if (!h2 || sektion.querySelector(':scope > .techbaum-knopf, :scope > .tb-einstieg-karte')) continue;
      if (id === 'freischaltungen') { sektion.classList.add('tb-einstieg'); h2.insertAdjacentElement('afterend', einstieg()); }
      else { h2.insertAdjacentElement('afterend', atlasKnopf()); h2.insertAdjacentElement('afterend', knopf()); }
    }
  }
  // „Weg zu …“-Verweise (progression.js, data-tree-target) öffnen den neuen Baum auf dem Ziel statt des alten Wegplaners;
  // Schließen führt zurück auf die Seite, von der man kam. Zivilisationsstufen haben keinen Eintrag: Baum beim eigenen Stand.
  document.addEventListener('click', e => {
    const b = e.target.closest?.('[data-tree-target]'); if (!b) return;
    e.stopImmediatePropagation(); e.preventDefault();
    if (b.dataset.treeKind === 'civilization') { oeffnen().then(() => { if (api?.zustand?.gewaehlt) api.zurueck?.(); api?.zumStand?.(); }); return; }
    zeigeObjekt(b.dataset.treeTarget).then(ok => { if (!ok) oeffnen(); });
  }, true);
  if (document.readyState === 'loading') document.addEventListener('DOMContentLoaded', einhaengen); else einhaengen();
  // Objektansicht (Reiter „Techbaum“ an Gebäuden, Forschung, Schiffen, Verteidigung, Truppen, Raketen, Mond): immer der
  // neue Gesamtbaum, auf das Objekt gezoomt (Karl 19.10.). false, wenn der Baum nicht lädt oder den Eintrag nicht kennt.
  async function zeigeObjekt(id, rueck) {
    await oeffnen();
    if (!offen() || !api?.zeigeObjekt?.(id)) { rueckweg = null; schliessen(); return false; }
    rueckweg = typeof rueck === 'function' ? rueck : null; return true;
  }
  window.TechbaumSpiel = {oeffnen, schliessen, zurueck, info, zeigeObjekt};
  window.KraefteatlasSpiel = {oeffnen: Atlas.oeffnen, schliessen: Atlas.schliessen, zurueck: Atlas.zurueck};
})();
