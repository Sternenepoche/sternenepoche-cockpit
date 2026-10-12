/* Ratsstimmen: Jedes Ratsmitglied hat eine eigene Stimme.
   - Klick auf die Figur: die Kamera fährt zum Kopf, das Ratsmitglied stellt sich vor (Volk, Herkunft, was ihm
     wichtig ist), mit Untertitel; der Kopf nickt und wippt nach dem echten Pegel der Stimme.
   - Zurufe: bei Ereignissen (Angriff, Erfolg, Misserfolg, Erlaubnis, Aufstieg, neue Kolonie, Fehler …) rufen
     die Ratsmitglieder kurze Sätze herein, mit eigener Persönlichkeit, mit Funkfenster und sprechendem Kopf.
   - Gespräch: Fragen an ein Ratsmitglied beantwortet das Modell seiner Rolle in seiner Persönlichkeit; die
     Antwort spricht Voicebox in seiner Stimme (Rust: src/stimmen.rs).
   Die Zeilen sind vorab mit Loki-Stimmen erzeugt (stimmen/zeilen.json + OGG), die Stimme selbst liegt je Figur
   unter assets/stimmen/<rolle>.<volk>/stimme/ (Referenz, Stimmabdruck, Voicebox-Paket). */
"use strict";

const Stimmen = (() => {
  let manifest = null;
  fetch("stimmen/zeilen.json").then((r) => (r.ok ? r.json() : null)).then((m) => { manifest = m; planen(); }).catch(() => {});

  // Einstellungen je Betrachter (Bequemlichkeit; ohne Speicher gelten die Vorgaben).
  const E = Object.assign({ an: true, zurufe: true, laut: 0.9, live: true },
    (() => { try { return JSON.parse(localStorage.getItem("ratsstimmen") || "{}"); } catch (e) { return {}; } })());
  const merken = () => { try { localStorage.setItem("ratsstimmen", JSON.stringify(E)); } catch (e) { /* ohne Speicher */ } };

  const ZUSTAENDIG = { angriff: "feldherr", kolonie: "verwalter", aufstieg: "stratege", rat_tagt: "stratege", rat_ruht: "stratege" };
  const ABKLINGEN = { klick: 0, genervt: 0, rat_tagt: 4, rat_ruht: 4, erfolg: 150, misserfolg: 150, angriff: 60, erlaubnis: 40, aufstieg: 40, kolonie: 40, fehler: 240, leerlauf: 900 };
  const KOPF = { winkel: 0.22, hoehe: 0.3, abstand: 2.35 }, GANZ = { hoehe: 0.12, abstand: 4.6 };

  const figur = (rolle, v) => manifest?.[`${rolle}.${v || gestalt(rolle)}`] || null;
  const buehnen = (rolle) => [...document.querySelectorAll(`.buehne[data-rolle="${rolle}"], .hausbuehne`)];
  const sichtbar = (el) => { const r = el.getBoundingClientRect(); return r.width > 40 && r.bottom > 40 && r.top < window.innerHeight - 40; };

  // ------------------------------------------------------------------ Wiedergabe
  let ctx = null, job = null, rafId = 0, pegelRolle = null, zuletztGesagt = Date.now();
  const schlange = [];
  function kontext() {
    if (!ctx) { try { ctx = new (window.AudioContext || window.webkitAudioContext)(); } catch (e) { ctx = null; } }
    if (ctx && ctx.state === "suspended") ctx.resume().catch(() => {});
    return ctx;
  }
  document.addEventListener("pointerdown", () => kontext(), { capture: true });
  // Bühnen ohne 3D-Modell (Ganzkörperbild oder Porträt): Klick stellt die Figur ebenso vor.
  document.addEventListener("click", (e) => {
    const b = e.target.closest && e.target.closest(".buehne[data-rolle]");
    if (b && !b.dataset.glb && !e.target.closest("button")) vorstellen(b.dataset.rolle);
  });

  /** Spielt einen Auftrag {rolle, art, datei, text, saetze?, name}; löst am Ende auf. */
  function abspielen(j) {
    stopp(true);
    return new Promise((fertig) => {
      const a = new Audio(j.datei);
      const c = kontext();
      try {
        if (!c) throw new Error("ohne Web Audio");
        const q = c.createMediaElementSource(a), an = c.createAnalyser(), g = c.createGain();
        an.fftSize = 1024; g.gain.value = E.laut; q.connect(an); an.connect(g); g.connect(c.destination);
        j.analyser = an; j.gain = g;
      } catch (e) { a.volume = E.laut; }
      j.audio = a; j.fertig = fertig; job = j; zuletztGesagt = Date.now();
      const ende = () => { if (job === j) beenden(); else fertig(); };
      a.onended = ende; a.onerror = ende;
      a.play().catch(ende);
      if (j.art !== "vorstellung" || !buehnen(j.rolle).some(sichtbar)) funk(j);
      cancelAnimationFrame(rafId); rafId = requestAnimationFrame(takt);
      planen();
    });
  }
  function beenden() {
    const j = job; if (!j) return;
    job = null;
    try { j.audio.pause(); } catch (e) { /* schon zu */ }
    pegel(null, 0); untertitel(null); funk(null, j);
    if (j.art !== "zuruf") for (const el of buehnen(j.rolle)) Dreid.fahren(el, GANZ);
    j.fertig && j.fertig();
    planen();
    setTimeout(weiter, 350);
  }
  /** Laufende Wiedergabe beenden (still: ohne Warteschlange fortzusetzen). */
  function stopp(still) { if (!job) return; if (still) { const j = job; job = null; try { j.audio.pause(); } catch (e) { /**/ } pegel(null, 0); untertitel(null); funk(null, j); j.fertig && j.fertig(); } else beenden(); }
  function weiter() {
    if (job) return;
    while (schlange.length && Date.now() - schlange[0].zeit > 25000) schlange.shift(); // veraltete Zurufe verfallen
    const n = schlange.shift();
    if (n) abspielen(n);
  }

  const puffer = new Float32Array(1024);
  function takt() {
    const j = job;
    if (!j) return;
    let p = 0;
    if (j.analyser) {
      j.analyser.getFloatTimeDomainData(puffer);
      let s = 0; for (let i = 0; i < puffer.length; i++) s += puffer[i] * puffer[i];
      // Zeilen sind auf -16 LUFS normiert: Sprache liegt bei RMS 0,05-0,25; leicht gekrümmt, damit der Kopf nicht dauernd am Anschlag nickt.
      p = Math.min(1, Math.pow(Math.sqrt(s / puffer.length) * 3.6, 1.25));
    } else if (!j.audio.paused) p = 0.3 + 0.25 * Math.abs(Math.sin(performance.now() / 95));
    pegel(j.rolle, p);
    if (j.saetze && j.saetze.length) {
      const t = j.audio.currentTime || 0;
      const s = j.saetze.find((x) => t >= x.start - 0.05 && t <= x.ende + 0.35) || null;
      untertitel(j, s ? satzBei(s, t) : "");
    } else untertitel(j, j.text);
    rafId = requestAnimationFrame(takt);
  }
  /** Ein Zeitabschnitt enthält oft mehrere Sätze: den gerade gesprochenen nach Zeichenanteil schätzen. */
  function satzBei(abschnitt, t) {
    const teile = abschnitt.text.match(/[^.!?]+[.!?]+["“”»]*|[^.!?]+$/g) || [abschnitt.text];
    if (teile.length < 2) return abschnitt.text;
    const anteil = (t - abschnitt.start) / Math.max(0.1, abschnitt.ende - abschnitt.start);
    const gesamt = teile.reduce((a, x) => a + x.length, 0);
    let summe = 0;
    for (const x of teile) { summe += x.length; if (summe / gesamt >= anteil) return x.trim(); }
    return teile[teile.length - 1].trim();
  }
  /** Pegel an Bühnen (3D-Kopf nickt) und CSS (--pegel für 2D-Porträts und Funkfenster). */
  function pegel(rolle, p) {
    const w = document.documentElement;
    w.style.setProperty("--pegel", p.toFixed(3));
    if (rolle) w.dataset.spricht = rolle; else delete w.dataset.spricht;
    if (pegelRolle && pegelRolle !== rolle) for (const el of buehnen(pegelRolle)) Dreid.sprechen(el, 0);
    if (rolle) for (const el of buehnen(rolle)) Dreid.sprechen(el, p);
    pegelRolle = rolle;
  }

  // ------------------------------------------------------------------ Untertitel (über der Bühne) und Funkfenster
  let utEl = null;
  function untertitel(j, text) {
    if (!utEl) { utEl = document.createElement("div"); utEl.className = "stimme-untertitel"; document.body.appendChild(utEl); }
    const el = j && j.art !== "zuruf" ? buehnen(j.rolle).filter(sichtbar).sort((a, b) => b.clientHeight - a.clientHeight)[0] : null;
    if (!el || !text || (funkEl && funkEl.dataset.id === String(j.id))) { utEl.classList.remove("an"); return; }
    const r = el.getBoundingClientRect();
    utEl.style.left = r.left + 12 + "px"; utEl.style.width = Math.max(120, r.width - 24) + "px"; utEl.style.top = r.bottom - 14 + "px";
    if (utEl.textContent !== text) utEl.textContent = text;
    utEl.style.setProperty("--rolle", ROLLENFARBE[j.rolle] || "var(--gold)");
    utEl.classList.add("an");
  }
  let funkEl = null, funkZu = 0;
  /** Funkfenster rechts unten: sprechender Kopf (3D, sonst Porträt), Name, Rolle, Satz. */
  function funk(j, alt) {
    if (!funkEl) { funkEl = document.createElement("div"); funkEl.id = "funk"; document.body.appendChild(funkEl); }
    clearTimeout(funkZu);
    if (!j) {
      if (alt && funkEl.dataset.id === String(alt.id)) funkZu = setTimeout(() => { funkEl.classList.remove("an"); setTimeout(() => { if (!funkEl.classList.contains("an")) { morphKinder(funkEl, h("div")); Dreid.pflegen(); } }, 400); }, 1400);
      return;
    }
    j.id = j.id || Math.random().toString(36).slice(2);
    const f = figur(j.rolle, j.volk) || {}, v = j.volk || gestalt(j.rolle);
    const neu = h("div", null, h("div.funk-box", null,
      h("div.funk-kopf", null, Dreid.element(`figuren/${j.rolle}.${v}.glb`, bild(`role_portraits.${j.rolle}.${v}`), 132, null, null, { portraet: true, farbe: ROLLENFARBE[j.rolle], rolle: j.rolle }),
        h("div.funk-linien")),
      h("div.funk-text", null,
        h("div.funk-name", null, h("b", null, f.name || ROLLENTITEL[j.rolle]), h("span", null, `${ROLLENTITEL[j.rolle]} · ${name(v)}`)),
        h("div.funk-satz", null, j.text || ""),
        h("div.funk-pegel", null, Array.from({ length: 14 }, (_, i) => h("i", { stil: { "--i": String(i) } })))),
      h("button.funk-zu", { title: "Verstummen", onclick: () => stopp() }, "✕")));
    funkEl.dataset.id = j.id;
    funkEl.style.setProperty("--rolle", ROLLENFARBE[j.rolle] || "var(--gold)");
    morphKinder(funkEl, neu);
    Dreid.pflegen();
    requestAnimationFrame(() => funkEl.classList.add("an"));
  }

  // ------------------------------------------------------------------ Vorstellung und Zurufe
  const klickZeiten = {};
  /** Klick auf die Figur: Vorstellung (mit Kopf-Ansicht); erneuter Klick hört auf; Dauerklicken nervt sie. */
  function vorstellen(rolle, volk) {
    const jetztMs = Date.now(), k = (klickZeiten[rolle] = (klickZeiten[rolle] || []).filter((t) => jetztMs - t < 2500).concat(jetztMs));
    if (k.length >= 4) { klickZeiten[rolle] = []; stopp(true); zuruf("genervt", rolle, { volk }); return; }
    if (job && job.rolle === rolle && job.volk === (volk || gestalt(rolle)) && job.art === "vorstellung") { beenden(); return; }
    if (!E.an) return;
    const f = figur(rolle, volk);
    for (const el of buehnen(rolle)) Dreid.fahren(el, KOPF);
    if (!f || !f.vorstellung) { hinweis(rolle, "Diese Stimme wird gerade noch erzeugt."); return; }
    schlange.length = 0;
    abspielen({ rolle, volk: volk || gestalt(rolle), art: "vorstellung", datei: f.vorstellung.datei, text: f.vorstellung.text, saetze: f.vorstellung.saetze, name: f.name });
  }
  /** Rolle in der Leiste gewählt: kurze Bestätigung wie beim Anwählen einer Einheit; Dauerklicken nervt. */
  function waehlen(rolle) {
    const jetztMs = Date.now(), k = (klickZeiten["w" + rolle] = (klickZeiten["w" + rolle] || []).filter((t) => jetztMs - t < 3000).concat(jetztMs));
    if (job && job.art !== "zuruf") return;
    if (k.length >= 5) { klickZeiten["w" + rolle] = []; zuruf("genervt", rolle); } else zuruf("klick", rolle);
  }
  const zuletztZeile = {}, abkling = {};
  /** Zuruf zu einem Anlass. `rolle` leer: zuständige oder zufällige aktive Rolle. */
  function zuruf(anlass, rolle, opt) {
    const o = opt || {};
    if (!E.an || (!E.zurufe && anlass !== "klick" && anlass !== "genervt")) return false;
    const aktiv = ROLLEN.filter((r) => rolleW(r).aktiv !== false);
    const r = rolle || ZUSTAENDIG[anlass] || aktiv[Math.floor(Math.random() * Math.max(1, aktiv.length))] || "stratege";
    const f = figur(r, o.volk), liste = f?.zeilen?.[anlass];
    if (!liste || !liste.length) return false;
    const sch = o.schluessel || anlass, jetztS = Date.now() / 1000;
    if (abkling[sch] && jetztS - abkling[sch] < (o.abkling ?? ABKLINGEN[anlass] ?? 30)) return false;
    abkling[sch] = jetztS;
    let i = Math.floor(Math.random() * liste.length);
    if (liste.length > 1 && i === zuletztZeile[r + anlass]) i = (i + 1) % liste.length;
    zuletztZeile[r + anlass] = i;
    const j = { rolle: r, volk: o.volk || gestalt(r), art: "zuruf", anlass, datei: liste[i].datei, text: liste[i].text, name: f.name, zeit: Date.now() };
    const dringend = anlass === "klick" || anlass === "genervt" || anlass === "angriff";
    if (job && job.art === "zuruf" && dringend) { schlange.unshift(j); beenden(); }
    else if (!job) abspielen(j);
    else if (schlange.length < 3) (dringend ? schlange.unshift(j) : schlange.push(j));
    return true;
  }
  function hinweis(rolle, text) { funk({ rolle, volk: gestalt(rolle), art: "hinweis", text, id: "h" + Date.now() }); funk(null, { id: funkEl.dataset.id }); }

  // ------------------------------------------------------------------ Ereignisse beobachten (nach jedem Zeichnen)
  let vorher = null, dienstZeit = 0;
  const gespielt = new Set();
  const zahlOder = (x) => (typeof x === "number" ? x : Number.isFinite(Number(x)) && x !== null && x !== "" ? Number(x) : null);
  function schnappschuss() {
    const z = S.zustand, l = z.lage || {};
    return {
      laeuft: !!z.laeuft,
      wartend: (z.wartend || []).map((w) => String(w.id ?? JSON.stringify(w.aktion || w).slice(0, 80))),
      angriffe: (l.angriffe || []).map((a) => `${a.ziel}@${a.ankunft}`),
      stufe: zahlOder(l.stufe), rang: zahlOder(l.rang),
      planeten: Array.isArray(l.planeten) ? l.planeten.length : null,
      zuege: S.daten ? (S.daten.zuege || []).map((x) => `${x.zeit}:${x.rolle}:${x.quelle}`) : null,
      fehler: ROLLEN.filter((r) => z.rollen?.[r]?.status === "Fehler"),
    };
  }
  function beobachten() {
    if (!S.zustand) return;
    const n = schnappschuss(), v = vorher;
    if (v) {
      if (n.laeuft && !v.laeuft) zuruf("rat_tagt");
      if (!n.laeuft && v.laeuft) zuruf("rat_ruht");
      const neuW = (S.zustand.wartend || []).filter((w, i) => !v.wartend.includes(n.wartend[i]));
      if (neuW.length) zuruf("erlaubnis", ROLLEN.includes(neuW[0].rolle) ? neuW[0].rolle : null);
      if (n.angriffe.some((a) => !v.angriffe.includes(a))) zuruf("angriff", "feldherr");
      if (n.stufe !== null && v.stufe !== null && n.stufe > v.stufe) zuruf("aufstieg", "stratege", { abkling: 5 });
      else if (n.rang !== null && v.rang !== null && n.rang < v.rang) zuruf("aufstieg", "diplomat", { schluessel: "rang", abkling: 900 });
      else if (n.rang !== null && v.rang !== null && n.rang > v.rang + 2) zuruf("misserfolg", "stratege", { schluessel: "rang_ab", abkling: 900 });
      if (n.planeten !== null && v.planeten !== null && n.planeten > v.planeten) zuruf("kolonie", "verwalter");
      if (n.planeten !== null && v.planeten !== null && n.planeten < v.planeten) zuruf("misserfolg", "feldherr", { abkling: 5 });
      if (n.zuege && v.zuege) {
        const neu = (S.daten.zuege || []).filter((x) => !v.zuege.includes(`${x.zeit}:${x.rolle}:${x.quelle}`) && !x.nur_probe);
        for (const zug of neu) {
          const erg = zug.ergebnisse || [], ok = erg.filter((e) => e.ok).length, nein = erg.length - ok + (zug.abgelehnt || []).length;
          const r = ROLLEN.includes(zug.rolle) ? zug.rolle : null;
          if (ok >= 2 && nein === 0 && Math.random() < 0.5 && zuruf("erfolg", r)) break;
          if (nein >= 2 && ok === 0 && zuruf("misserfolg", r)) break;
        }
      }
      for (const r of n.fehler) if (!v.fehler.includes(r)) zuruf("fehler", r);
      if (n.laeuft && Date.now() - zuletztGesagt > 12 * 60e3) zuruf("leerlauf");
    }
    // Züge werden erst verglichen, wenn beide Stände Daten haben: sonst wären beim Start alle alten Züge „neu“.
    vorher = n;
    // Gespräche: fertig vertonte Antworten einmal abspielen.
    for (const g of S.zustand.stimmen?.gespraeche || []) {
      if (g.audio && g.status === "fertig" && !gespielt.has(g.id)) {
        gespielt.add(g.id);
        if (Date.now() / 1000 - (g.zeit || 0) < 600) { for (const el of buehnen(g.rolle)) Dreid.fahren(el, KOPF); schlange.length = 0; abspielen({ rolle: g.rolle, volk: g.volk, art: "gespraech", datei: g.audio, text: g.antwort, name: figur(g.rolle, g.volk)?.name }); }
      } else if (!g.audio && g.status === "fertig") gespielt.add(g.id);
    }
    if (Date.now() - dienstZeit > 45000) { dienstZeit = Date.now(); sende("stimmen_pruefen"); }
  }

  // ------------------------------------------------------------------ Gespräch (Block unter der Figur auf der Rat-Seite)
  function fragen(r) {
    const t = (S.ui.frage?.[r] || "").trim();
    if (!t) return;
    const verlauf = [];
    for (const g of (S.zustand?.stimmen?.gespraeche || []).filter((x) => x.rolle === r && x.volk === gestalt(r) && x.antwort).slice(-4)) {
      verlauf.push({ spieler: true, text: g.frage }, { spieler: false, text: g.antwort });
    }
    sende("gespraech", { rolle: r, volk: gestalt(r), frage: t, verlauf, stimme: E.live });
    S.ui.frage[r] = ""; planen();
  }
  function dienstText(d) {
    if (!d || d.voicebox === undefined) return "Stimm-Dienste werden geprüft …";
    if (d.voicebox && !d.gpu_belegt) return "Live-Stimme: Voicebox bereit";
    if (d.voicebox && d.gpu_belegt) return `Live-Stimme: GPU belegt (${d.gpu_belegt})${d.loki_stimmen ? " – Loki-Stimmen spricht auf der CPU" : " – Antworten als Text"}`;
    return `Live-Stimme: Voicebox läuft nicht${d.loki_stimmen ? " – Loki-Stimmen spricht auf der CPU (langsam)" : " – Antworten als Text"}`;
  }
  function gespraech(r) {
    const f = figur(r), st = S.zustand?.stimmen || {}, d = st.dienst || {};
    S.ui.frage = S.ui.frage || {};
    const liste = (st.gespraeche || []).filter((g) => g.rolle === r && g.volk === gestalt(r)).slice(-4);
    const offen = (st.gespraeche || []).some((g) => g.status === "denkt" || g.status === "vertont");
    const sprichtJetzt = job && job.rolle === r;
    const zustand = (g) => g.status === "denkt" ? `${f?.kurzname || ROLLENTITEL[r]} denkt nach …` : g.status === "vertont" ? (g.hinweis || "Stimme entsteht …") : g.hinweis;
    return h("section.karte.gespraech", { "data-k": "gespraech:" + r, stil: { "--rolle": ROLLENFARBE[r] } },
      h("div.gespraech-kopf", null,
        h("div", null, h("b", null, f ? f.name : ROLLENTITEL[r]), h("small", null, f ? `${ROLLENTITEL[r]} der ${name(gestalt(r))}` : "Stimme wird noch erzeugt")),
        h("span.rechts"),
        knopf(sprichtJetzt ? "Stopp" : "Vorstellung", () => (sprichtJetzt ? stopp() : vorstellen(r)), "klein" + (sprichtJetzt ? "" : " gold"), sprichtJetzt ? "stopp" : "los", !f?.vorstellung && !sprichtJetzt)),
      liste.length ? h("div.gespraech-verlauf", null, liste.map((g) => h("div.paar", { "data-k": "g:" + g.id },
        h("div.frage", null, g.frage),
        h("div.antwort" + (g.status === "fehler" ? ".fehler" : ""), null,
          g.antwort ? h("span", null, g.antwort) : null,
          zustand(g) ? h("small", null, zustand(g)) : null,
          g.audio ? h("button.nochmal", { title: "Noch einmal hören", onclick: () => abspielen({ rolle: g.rolle, volk: g.volk, art: "gespraech", datei: g.audio, text: g.antwort }) }, "▶") : null)))) : null,
      h("div.gespraech-eingabe", null,
        h("input.feld", { placeholder: f ? `${f.kurzname} etwas fragen …` : "Etwas fragen …", value: S.ui.frage[r] || "", maxlength: "600",
          oninput: (e) => { S.ui.frage[r] = e.target.value; }, onkeydown: (e) => { if (e.key === "Enter") fragen(r); } }),
        knopf(offen ? "…" : "Fragen", () => fragen(r), "klein", null, offen)),
      h("div.gespraech-fuss", null,
        schalter(E.zurufe, "Zurufe", (v) => { E.zurufe = v; merken(); planen(); }, "Kurze Sätze der Ratsmitglieder bei Ereignissen, mit eigener Persönlichkeit"),
        schalter(E.live, "Antwort sprechen", (v) => { E.live = v; merken(); planen(); }, "Antworten im Gespräch in der Stimme der Figur (Voicebox)"),
        h("label.laut", { title: "Lautstärke der Ratsstimmen" }, "🔊", h("input", { type: "range", min: "0", max: "1", step: "0.05", value: String(E.laut),
          oninput: (e) => { E.laut = Number(e.target.value); if (job?.gain) job.gain.gain.value = E.laut; else if (job) job.audio.volume = E.laut; }, onchange: merken })),
        h("span.winzig.matt", { title: d.einrichten?.text || "" }, dienstText(d)),
        d.voicebox ? knopf("In Voicebox anlegen", () => sende("stimme_einrichten"), "leise klein", null, d.einrichten?.laeuft) : null,
        knopf("Stimmdateien", () => sende("oeffnen", { was: "stimmen" }), "leise klein", "ordner")),
      d.einrichten?.text ? h("div.winzig" + (d.einrichten.ok === false ? ".rot" : ".matt"), null, d.einrichten.text) : null);
  }

  return { figur, vorstellen, waehlen, zuruf, beobachten, gespraech, stopp, einstellungen: E, spricht: () => (job ? { rolle: job.rolle, volk: job.volk, art: job.art } : null) };
})();
// Für die Haken in dreid.js, rat.js und app.js (prüfen „window.Stimmen“; const allein hängt nicht an window).
window.Stimmen = Stimmen;
