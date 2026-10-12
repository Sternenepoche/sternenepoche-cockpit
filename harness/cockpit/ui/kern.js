/* Kern der Oberfläche: Zustand, Leitung zum Rust-Kern, sicherer DOM-Bau, Abgleich, Bausteine.
   Sicherheit: Alle Daten werden als Text gesetzt (textContent), nie als HTML. Texte anderer
   Spieler (Briefe, RAG-Antworten) können hier also nie Code werden. */
"use strict";

const S = {
  katalog: null, auswahl: null, zustand: null, daten: null,
  ereignisse: [], detail: {},
  ui: { seite: "bruecke", rolle: "verwalter", stufe: "identitaet", codex: { offen: false, art: "schiffe", wahl: null, suche: "" },
        wissen: { quelle: "gedaechtnis", frage: "", antwort: 0, einstellen: false }, labor: { reiter: "probe", verlauf: null },
        plan: { reiter: "aufgaben" }, werkstatt: { reiter: "bau" }, entwurf: {}, einrichtung: null, filter: "" },
};

// Deep-Links wie #seite=rat&rolle=diplomat&stufe=kontext oder #seite=labor&reiter=arena oder
// #codex=schiffe&wahl=fregatte oder #einrichtung=3: für Lesezeichen, Handbuch und Bildschirmfotos.
(() => {
  const p = new URLSearchParams(location.hash.slice(1));
  if (!p.toString()) return;
  for (const k of ["seite", "rolle", "stufe"]) if (p.get(k)) S.ui[k] = p.get(k);
  const bereich = S.ui[S.ui.seite];
  if (p.get("reiter") && bereich && typeof bereich === "object") bereich.reiter = p.get("reiter");
  if (p.get("codex")) Object.assign(S.ui.codex, { offen: true, art: p.get("codex"), wahl: p.get("wahl") });
  if (p.get("quelle")) S.ui.wissen.quelle = p.get("quelle");
  const e = parseInt(p.get("einrichtung") || "", 10);
  if (e >= 1 && e <= 5) S.ui.einrichtung = e; else { S.ui.einrichtung = null; S.ui.einrichtungZu = true; }
})();

// ---------------------------------------------------------------- Leitung
const Leitung = (() => {
  const t = new URLSearchParams(location.search).get("t");
  if (window.ipc && window.ipc.postMessage) {
    return { senden: (m) => window.ipc.postMessage(JSON.stringify(m)), entwicklung: false };
  }
  // Entwicklungsansicht im normalen Browser: Abfrage alle 400 ms.
  let seq = 0;
  const holen = async () => {
    try {
      const r = await fetch(`/api/zustand?t=${encodeURIComponent(t)}&seit=${seq}`);
      const v = await r.json();
      seq = v.seq;
      for (const [art, d] of v.nachrichten) window.__cockpit(art, d);
    } catch (e) { /* Cockpit beendet */ }
    setTimeout(holen, 400);
  };
  setTimeout(holen, 50);
  return { senden: (m) => fetch(`/api/befehl?t=${encodeURIComponent(t)}`, { method: "POST", body: JSON.stringify(m) }), entwicklung: true };
})();
function sende(t, daten) { Leitung.senden(Object.assign({ t }, daten || {})); }
function harness(befehl) { sende("harness", { befehl }); }
function setze(pfad, wert) {
  sende("setze", { pfad, wert });
  // Sofort lokal zeigen, bis der Kern bestätigt.
  if (S.auswahl) { const teile = pfad.split("/").slice(1); let o = S.auswahl; for (let i = 0; i < teile.length - 1; i++) { o = o?.[teile[i]]; } if (o) o[teile[teile.length - 1]] = wert; planen(); }
}

window.__cockpit = (art, d) => {
  if (art === "ereignisse") { S.ereignisse = d.concat(S.ereignisse).slice(0, 400); }
  else if (art === "detail") { S.detail[d.art + d.i] = d.eintrag; }
  else if (art === "steuern") { steuern(d); }
  else { S[art] = d; }
  if (art === "zustand" && d.lokal?.connected && (S.ui.lokal?.go || d.lokal.auto_resume && !S.ui.autoResumeGezeigt)) {
    if(S.ui.lokal)S.ui.lokal.go = false;
    S.ui.autoResumeGezeigt=true; S.ui.einrichtung=null; S.ui.einrichtungZu=true; S.ui.seite = "spiel";
  }
  planen();
};
let geplant = false;
function planen() { if (!geplant) { geplant = true; requestAnimationFrame(() => { geplant = false; zeichnen(); }); } }

// ---------------------------------------------------------------- DOM-Bau
const SVGNS = "http://www.w3.org/2000/svg";
const SVG_TAGS = new Set(["svg", "path", "circle", "rect", "line", "polyline", "polygon", "g", "ellipse", "defs", "linearGradient", "stop", "text"]);
const EREIGNISSE = ["onclick", "oninput", "onchange", "onkeydown", "onpointerdown", "ondblclick", "onblur", "onfocus", "onmouseenter", "ontoggle"];
function h(tag, a, ...kinder) {
  const [name, ...klassen] = tag.split(".");
  const el = SVG_TAGS.has(name) ? document.createElementNS(SVGNS, name) : document.createElement(name || "div");
  if (klassen.length) el.setAttribute("class", klassen.join(" "));
  if (a) for (const [k, v] of Object.entries(a)) {
    if (v === undefined || v === null || v === false) continue;
    if (k.startsWith("on")) el[k.toLowerCase()] = v;
    else if (k === "stil") { for (const [s, w] of Object.entries(v)) { if (s.startsWith("--")) el.style.setProperty(s, w); else el.style[s] = w; } }
    else if (k === "klasse") el.setAttribute("class", ((el.getAttribute("class") || "") + " " + v).trim());
    else if (k === "value" || k === "checked") el[k] = v;
    else if (k === "text") el.textContent = v;
    else el.setAttribute(k, v === true ? "" : String(v));
  }
  for (const k of kinder.flat(Infinity)) {
    if (k === null || k === undefined || k === false) continue;
    el.appendChild(k instanceof Node ? k : document.createTextNode(String(k)));
  }
  return el;
}
/** Neuen Baum in den alten einarbeiten: Fokus, Eingaben und 3D-Bühnen bleiben erhalten. */
function morph(alt, neu) {
  if (alt.nodeType !== neu.nodeType || alt.nodeName !== neu.nodeName || (alt.dataset && alt.dataset.k) !== (neu.dataset && neu.dataset.k)) { alt.replaceWith(neu); return neu; }
  if (alt.nodeType === 3) { if (alt.nodeValue !== neu.nodeValue) alt.nodeValue = neu.nodeValue; return alt; }
  if (alt.nodeType !== 1) return alt;
  if (alt.dataset.fest) return alt;
  for (const at of [...alt.attributes]) if (at.name !== "style" && !neu.hasAttribute(at.name)) alt.removeAttribute(at.name);
  for (const at of [...neu.attributes]) if (at.name !== "style" && alt.getAttribute(at.name) !== at.value) alt.setAttribute(at.name, at.value);
  if (alt.style.cssText !== neu.style.cssText) alt.style.cssText = neu.style.cssText;
  for (const e of EREIGNISSE) if (alt[e] !== neu[e]) alt[e] = neu[e];
  if (alt.tagName === "INPUT" || alt.tagName === "TEXTAREA" || alt.tagName === "SELECT") {
    if (alt.tagName === "SELECT") { morphKinder(alt, neu); }
    if (document.activeElement !== alt) { if (alt.type === "checkbox") alt.checked = neu.checked; else if (alt.value !== neu.value) alt.value = neu.value; }
    return alt;
  }
  morphKinder(alt, neu);
  return alt;
}
function morphKinder(alt, neu) {
  const a = [...alt.childNodes], n = [...neu.childNodes];
  for (let i = 0; i < n.length; i++) { if (i < a.length) morph(a[i], n[i]); else alt.appendChild(n[i]); }
  for (let i = n.length; i < a.length; i++) a[i].remove();
}

// ---------------------------------------------------------------- Symbole (eigene Linienzeichnungen)
const PFADE = {
  bruecke: "M12 3l2.6 5.6L20 9.5l-4 4 1 5.8L12 16.6 7 19.3l1-5.8-4-4 5.4-.9z",
  rat: "M8 11a3 3 0 1 0 0-6 3 3 0 0 0 0 6zm8 0a3 3 0 1 0 0-6 3 3 0 0 0 0 6zM2 20c0-3.3 2.7-6 6-6s6 2.7 6 6M12.5 14.4c1-.3 2.2-.4 3.5-.4 3.3 0 6 2.7 6 6",
  strategie: "M4 20h16M6 20V9l6-5 6 5v11M10 20v-5h4v5M9 10h6",
  plan: "M9 6h11M9 12h11M9 18h11M4 6l1 1 2-2M4 12l1 1 2-2M4 18l1 1 2-2",
  wissen: "M4 6c0-1.7 3.6-3 8-3s8 1.3 8 3-3.6 3-8 3-8-1.3-8-3zm0 0v12c0 1.7 3.6 3 8 3s8-1.3 8-3V6M4 12c0 1.7 3.6 3 8 3s8-1.3 8-3",
  labor: "M9 3h6M10 3v6l-5.5 9.5A2 2 0 0 0 6.2 21h11.6a2 2 0 0 0 1.7-2.5L14 9V3M7.5 15h9",
  codex: "M4 4.5A2.5 2.5 0 0 1 6.5 2H20v17H6.5A2.5 2.5 0 0 0 4 21.5zm0 0v17M8 7h8M8 11h6",
  spiel: "M12 21a9 9 0 1 0 0-18 9 9 0 0 0 0 18zM3.5 9h17M3.5 15h17M12 3c2.5 2.6 3.8 5.6 3.8 9s-1.3 6.4-3.8 9c-2.5-2.6-3.8-5.6-3.8-9S9.5 5.6 12 3z",
  werkstatt: "M14.7 6.3a4 4 0 0 0-5.4 5.4L3 18l3 3 6.3-6.3a4 4 0 0 0 5.4-5.4l-2.6 2.6-2.4-.6-.6-2.4z",
  los: "M7 4l12 8-12 8z", stopp: "M6 6h12v12H6z", pause: "M7 5h3v14H7zM14 5h3v14h-3z",
  haken: "M5 12.5l4.5 4.5L19 7", kreuz: "M6 6l12 12M18 6L6 18", pfeil: "M5 12h14M13 6l6 6-6 6", extern: "M14 4h6v6M20 4l-9 9M18 14v5a1 1 0 0 1-1 1H5a1 1 0 0 1-1-1V7a1 1 0 0 1 1-1h5",
  identitaet: "M12 12a4 4 0 1 0 0-8 4 4 0 0 0 0 8zM5 21a7 7 0 0 1 14 0",
  ausloeser: "M13 2L4 14h7l-1 8 9-12h-7z",
  wahrnehmung: "M2 12s3.6-7 10-7 10 7 10 7-3.6 7-10 7S2 12 2 12zm10 3a3 3 0 1 0 0-6 3 3 0 0 0 0 6z",
  gedaechtnis: "M9 4a3 3 0 0 0-3 3 3 3 0 0 0-2 5 3 3 0 0 0 2 5 3 3 0 0 0 3 3h1V4zm6 0a3 3 0 0 1 3 3 3 3 0 0 1 2 5 3 3 0 0 1-2 5 3 3 0 0 1-3 3h-1V4z",
  kontext: "M4 5h16M4 10h16M4 15h10M4 20h7",
  modell: "M12 2l8.7 5v10L12 22l-8.7-5V7zM12 22V12m8.7-5L12 12 3.3 7",
  werkzeuge: "M4 20l6-6m-2-6l8 8m-1-9l3-3 3 3-3 3M7 10L4 7l3-3 3 3",
  wachposten: "M12 3l8 3v6c0 4.5-3.4 8.3-8 9-4.6-.7-8-4.5-8-9V6z",
  probe: "M9 11l2 2 4-4M4 4h16v16H4z",
  lernen: "M4 19V5l8-2 8 2v14l-8 2zM12 3v18",
  stern: "M12 3l2.4 5 5.6.8-4 3.9 1 5.5-5-2.7-5 2.7 1-5.5-4-3.9 5.6-.8z",
  suche: "M11 18a7 7 0 1 0 0-14 7 7 0 0 0 0 14zm9 3l-4.3-4.3",
  dreid: "M12 2l9 5v10l-9 5-9-5V7zM3 7l9 5 9-5M12 12v10",
  ordner: "M3 6a2 2 0 0 1 2-2h4l2 2h8a2 2 0 0 1 2 2v9a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z",
  kopie: "M8 8h11v11H8zM5 16V5h11",
  plus: "M12 5v14M5 12h14",
  netz: "M5 7a2 2 0 1 0 0-4 2 2 0 0 0 0 4zm14 0a2 2 0 1 0 0-4 2 2 0 0 0 0 4zM12 21a2 2 0 1 0 0-4 2 2 0 0 0 0 4zM6.5 6.5l4.5 9.5M17.5 6.5L13 16M7 5h10",
  stecker: "M9 2v5M15 2v5M6 7h12v4a6 6 0 0 1-12 0zM12 17v5",
  blitz: "M13 2L4 14h7l-1 8 9-12h-7z",
};
function ikon(name, gross) { return h("svg", { viewBox: "0 0 24 24", fill: "none", stroke: "currentColor", "stroke-width": "1.7", "stroke-linecap": "round", "stroke-linejoin": "round", klasse: gross ? "" : "ikon16" }, h("path", { d: PFADE[name] || PFADE.stern })); }

// ---------------------------------------------------------------- Hilfen
const ROLLEN = ["stratege", "verwalter", "diplomat", "feldherr"];
const ROLLENFARBE = { stratege: "var(--stratege)", verwalter: "var(--verwalter)", diplomat: "var(--diplomat)", feldherr: "var(--feldherr)" };
const ROLLENTITEL = { stratege: "Stratege", verwalter: "Verwalter", diplomat: "Diplomat", feldherr: "Feldherr" };
const UMLAUTE = { zerstoerer: "Zerstörer", ueberwachungstechnik: "Überwachungstechnik", konsumgueterwerk: "Konsumgüterwerk", giesserei: "Gießerei", grosser_transporter: "Großer Transporter", gausskanone: "Gaußkanone",
  leichter_jaeger: "Leichter Jäger", aufklaerung: "Aufklärung", lasergeschuetz: "Lasergeschütz", ionengeschuetz: "Ionengeschütz", marktgebuehr: "Marktgebühr", pluenderquote: "Plünderquote", pluenderung: "Plünderung",
  gedaechtnis: "Gedächtnis", gedaechtnis_suche: "Gedächtnissuche", truemmerfeld: "Trümmerfeld", aufklaerungsregeln: "Aufklärungsregeln", ausloeser: "Auslöser", bauschleife_leer: "Bauschleife leer", stufenaufstieg: "Stufenaufstieg",
  zurueckrufen: "zurückrufen", gruenden: "gründen", kuendigen: "kündigen", schleife_leeren: "Schleife leeren", flotte_zurueckrufen: "Flotte zurückrufen", vertrag_kuendigen: "Vertrag kündigen", allianz_gruenden: "Allianz gründen",
  flotte_ausspaehen: "Flotte ausspähen", abreissen: "Abreißen", prioritaeten: "Prioritäten", konsumgut: "Konsumgüter", ohne_nahrung: "Ohne Nahrung" };
function name(k) {
  if (!k) return "";
  if (UMLAUTE[k]) return UMLAUTE[k];
  return String(k).split(/[_:]/).map((w, i) => (i > 0 && ["je", "und", "der", "die", "das"].includes(w)) ? w : w.charAt(0).toUpperCase() + w.slice(1)).join(" ");
}
function zahl(x, stellen) {
  if (x === null || x === undefined || x === "") return "–";
  const n = Number(x); if (!isFinite(n)) return String(x);
  if (Math.abs(n) >= 1e6) return (n / 1e6).toLocaleString("de-DE", { maximumFractionDigits: 1 }) + " Mio.";
  return n.toLocaleString("de-DE", { maximumFractionDigits: stellen ?? (Math.abs(n) < 10 && n % 1 ? 1 : 0) });
}
function jetzt() { return Math.floor(Date.now() / 1000); }
function vor(t) {
  if (!t) return "";
  const s = jetzt() - t;
  if (s < 5) return "gerade eben"; if (s < 60) return `vor ${s} s`; if (s < 3600) return `vor ${Math.floor(s / 60)} min`;
  if (s < 86400) return `vor ${Math.floor(s / 3600)} h`; return `vor ${Math.floor(s / 86400)} d`;
}
function in_(t) {
  const s = t - jetzt();
  if (s <= 0) return "jetzt"; if (s < 60) return `in ${s} s`; if (s < 3600) return `in ${Math.floor(s / 60)}:${String(s % 60).padStart(2, "0")} min`;
  return `in ${Math.floor(s / 3600)} h ${Math.floor((s % 3600) / 60)} min`;
}
function volk() { return (S.zustand?.lage?.volk || S.zustand?.anmeldung?.volk || "veyari"); }
function bild(pfad) { return "bilder/" + pfad + ".webp"; }
function bildVolk(art, key) { return bild(`${art}.${key}.${volk()}`); }
/** Volk der Ratsfigur einer Rolle: frei wählbar (gemischter Rat möglich), sonst das Volk des Reichs. */
function gestalt(rolle) { return S.auswahl?.rollen?.[rolle]?.gestalt || volk(); }
/** Vorschlag für Cloud-Modelle im Einrichtungsassistenten (günstig, schnell, liefert sauberes JSON). */
const OPENROUTER_VORSCHLAG = "anthropic/claude-haiku-5.5";
const VOELKER = ["veyari", "aurelianer", "krath", "syntheten"];
function portraet(rolle) { return bild(`role_portraits.${rolle}.${gestalt(rolle)}`); }
function gutBild(g) { return bild(`resources.${g}`); }
function planetBild(zone) { return bild("planets." + ({ glut: "glut_wueste", leben: "leben_kontinent", frost: "frost_eis", nebel: "nebel_xeno" }[zone] || "leben_ozean")); }
function rolleW(r) { return S.auswahl?.rollen?.[r] || {}; }
function rolleA(r) { return S.zustand?.rollen?.[r] || {}; }
function laeuft() { return !!S.zustand?.laeuft; }

// ---------------------------------------------------------------- Bausteine
function karte(titel, unter, ...inhalt) {
  return h("section.karte", null, titel ? h("h2", null, titel) : null, unter ? h("p.unter", null, unter) : null, ...inhalt);
}
function chip(text, art, bildpfad) { return h("span.chip" + (art ? "." + art : ""), null, bildpfad ? h("img", { src: bildpfad, alt: "" }) : null, text); }
function knopf(text, onclick, art, ikonname, aus) {
  return h("button.k" + (art ? "." + art.split(" ").join(".") : ""), { onclick, disabled: !!aus, type: "button" }, ikonname ? ikon(ikonname) : null, text);
}
function segment(werte, aktuell, onwahl, gold) {
  return h("div.segment" + (gold ? ".gold" : ""), null, werte.map(([w, t, hilfe]) => h("button" + (w === aktuell ? ".an" : ""), { type: "button", title: hilfe || "", onclick: () => onwahl(w) }, t)));
}
function schalter(an, text, onwechsel, hilfe) {
  return h("label.schalter" + (an ? ".an" : ""), { title: hilfe || "", onclick: (e) => { e.preventDefault(); onwechsel(!an); } }, h("span.bahn"), text);
}
function regler(wert, min, max, schritt, anzeige, onwahl) {
  return h("div.regler", null, h("input", { type: "range", min, max, step: schritt, value: wert, oninput: (e) => { e.target.nextSibling.textContent = anzeige(Number(e.target.value)); }, onchange: (e) => onwahl(Number(e.target.value)) }), h("b", null, anzeige(Number(wert))));
}
/** Textfeld, das in die Einstellungen schreibt (verzögert, ohne Fokusverlust). */
const verzoegert = {};
function textfeld(pfad, wert, mehrzeilig, hinweis, extra) {
  const senden = (v) => { clearTimeout(verzoegert[pfad]); verzoegert[pfad] = setTimeout(() => setze(pfad, v), 500); };
  const a = Object.assign({ value: wert ?? "", placeholder: hinweis || "", oninput: (e) => senden(e.target.value), onblur: (e) => { clearTimeout(verzoegert[pfad]); if (e.target.value !== (wert ?? "")) setze(pfad, e.target.value); } }, extra || {});
  return mehrzeilig ? h("textarea.feld" + (extra?.code ? ".code" : ""), a) : h("input.feld", Object.assign({ type: "text" }, a));
}
function balken(anteil, art) { return h("div.balken" + (art ? "." + art : ""), null, h("i", { stil: { width: Math.max(0, Math.min(100, anteil * 100)) + "%" } })); }
function leer(text) { return h("div.leer", null, text); }
function sparkline(werte, farbe, hoehe) {
  const w = 300, hh = hoehe || 60;
  if (!werte || werte.length < 2) return h("svg.diagramm", { viewBox: `0 0 ${w} ${hh}`, stil: { height: hh + "px" } });
  const min = Math.min(...werte), max = Math.max(...werte), sp = (max - min) || 1;
  const pkt = werte.map((v, i) => `${(i / (werte.length - 1)) * w},${hh - 4 - ((v - min) / sp) * (hh - 8)}`);
  return h("svg.diagramm", { viewBox: `0 0 ${w} ${hh}`, preserveAspectRatio: "none", stil: { height: hh + "px" } },
    h("polyline", { points: `0,${hh} ${pkt.join(" ")} ${w},${hh}`, fill: farbe, "fill-opacity": "0.12", stroke: "none" }),
    h("polyline", { points: pkt.join(" "), fill: "none", stroke: farbe, "stroke-width": "2", "vector-effect": "non-scaling-stroke" }));
}
function dialog(inhalt, breit) { return h("div.dialog-grund", null, h("div.dialog" + (breit ? ".breit" : ""), null, inhalt)); }
function nav(seite, extra) { Object.assign(S.ui, { seite }, extra || {}); planen(); }
function codexOeffnen(art, wahl) { S.ui.codex = Object.assign(S.ui.codex, { offen: true, art, wahl }); planen(); }

/** Steuerbefehle von außen (Abnahme): Seite und Auswahl setzen. */
function steuern(d) {
  S.ui.einrichtung = null; S.ui.einrichtungZu = true; S.ui.credits = false;
  if (d.seite === "codex") { codexOeffnen(d.art || "schiffe", d.wahl || null); return; }
  S.ui.codex.offen = false;
  const bereich = S.ui[d.seite];
  if (d.reiter && bereich && typeof bereich === "object") bereich.reiter = d.reiter;
  // Abnahme-Fotos der Ratsstimmen: Vorstellung bzw. Zuruf nach dem Seitenwechsel auslösen. Was vom vorigen Foto
  // noch spricht, verstummt zuerst – sonst wartet der Zuruf in der Schlange und das Funkfenster fehlt im Foto.
  if ((d.vorstellen || d.zuruf) && window.Stimmen?.stopp) Stimmen.stopp(true);
  if (d.vorstellen && window.Stimmen?.vorstellen) setTimeout(() => Stimmen.vorstellen(d.vorstellen), 600);
  if (Array.isArray(d.zuruf) && window.Stimmen?.zuruf) setTimeout(() => Stimmen.zuruf(d.zuruf[0], d.zuruf[1]), 300);
  if (d.rolle) S.ui.rolle = d.rolle;
  if (d.stufe) S.ui.stufe = d.stufe;
  if (d.quelle) S.ui.wissen.quelle = d.quelle;
  S.ui.seite = d.seite || S.ui.seite;
}
