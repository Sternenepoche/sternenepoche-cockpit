/* Rahmen: Kopfleiste, Schiene, Seiten, Codex, Dialoge. */
"use strict";

const SEITEN = [
  ["bruecke", "Brücke", "bruecke", "Lage, Rat, Erlaubnisse, Ereignisse"],
  ["rat", "Rat", "rat", "Die vier Agenten und ihr Harness"],
  ["strategie", "Strategie", "strategie", "Spielstil, Doktrin, Autonomie, Daueraufträge"],
  ["plan", "Plan", "plan", "Aufgaben, Team, Erinnerungen"],
  ["wissen", "Wissen", "wissen", "Datenbanken, RAG, MCP"],
  ["labor", "Labor", "labor", "Probezug, Arena, Kontext, Kennzahlen, Training"],
  ["spiel", "Spiel", "spiel", "Das Spiel im Cockpit"],
  ["lokal", "Lokale Sitzung", "spiel", "Singleplayer offline, Host und LAN/WLAN-Beitritt"],
  ["werkstatt", "Werkstatt", "werkstatt", "Bauen, Modelle, Konto, Schnittstellen"],
];
const SEITENFN = { bruecke: () => seiteBruecke(), rat: () => seiteRat(), strategie: () => seiteStrategie(), plan: () => seitePlan(), wissen: () => seiteWissen(),
  labor: () => seiteLabor(), spiel: () => seiteSpiel(), lokal: () => seiteLokal(), werkstatt: () => seiteWerkstatt() };

function kopf() {
  const z = S.zustand, l = z.lage || {}, inv = S.daten?.inventur || {}, summe = inv.summe || {};
  const gueter = ["erz", "kristall", "deuterium", "nahrung"].map((g) => {
    const b = summe.bestand?.[g], r = summe.rate_je_stunde?.[g];
    if (b === undefined) return null;
    return h("span.gut", { title: `${name(g)}: ${zahl(b)} (${r >= 0 ? "+" : ""}${zahl(r)} je Stunde)` }, h("img", { src: gutBild(g), alt: "" }), h("b", null, zahl(b)), h("i" + (r < 0 ? ".minus" : ""), null, (r >= 0 ? "+" : "") + zahl(r)));
  });
  const strom = summe.energie_saldo;
  const reich = l.name ? h("span.gut.reich", { title: "Dein Reich" }, h("img", { src: bild(`emblems.${volk()}.${volk()}`), alt: "" }),
    h("span", null, h("b", null, l.name), h("small", null, `${name(l.volk)} · Stufe ${l.stufe ?? "–"} · Rang ${l.rang ?? "–"} · ${zahl(l.punkte)} Punkte`))) : null;
  const lokal=z.lokal||{}, info=lokal.info||{}, aktiv=lokal.connected, angemeldet=aktiv||!!z.anmeldung?.angemeldet;
  const status = aktiv ? [info.paused ? "Partie pausiert" : "Partie läuft", info.paused ? "gelb" : "an"] : !z.laeuft ? ["Rat ruht", ""] : z.pausiert ? ["Rat pausiert", "gelb"] : ["Rat tagt", "an"];
  return h("header.kopf", null,
    h("div.marke", null, h("img", { src: "bilder/marke.png", alt: "" }), h("div", null, h("b", null, "STERNENEPOCHE"), h("small", null, "HARNESS-COCKPIT"))),
    h("div.reichsband", null, reich, gueter, strom !== undefined ? h("span.gut", { title: "Energiesaldo aller Planeten" }, h("img", { src: gutBild("energie"), alt: "" }), h("b" + (strom < 0 ? ".rot" : ""), null, (strom >= 0 ? "+" : "") + zahl(strom))) : null,
      l.credits !== undefined ? h("span.gut", null, h("img", { src: gutBild("credits"), alt: "" }), h("b", null, zahl(l.credits))) : null),
    h("div.steuerung", null,
      !angemeldet && !lokal.busy ? knopf("Hier auch Singleplayer\noffline spielen", () => nav("lokal"), "gold offline-einstieg", "spiel") : aktiv ? knopf("Partie", () => nav("lokal"), "leise", "spiel") : null,
      z.quote ? h("span.pille.klein", { title: "Verbleibende Anfragen dieser Minute (Mensch und Harness teilen die Quote)" }, "Quote " + z.quote) : null,
      h("span.pille", null, h("span.licht" + (status[1] ? "." + status[1] : "")), status[0]),
      knopf("Codex", () => codexOeffnen(S.ui.codex.art, S.ui.codex.wahl), "leise", "codex"),
      knopf("Spiel im Browser", () => sende("browser_extern"), "leise", "extern"),
      aktiv && lokal.own_host ? knopf(lokal.steuert ? "Wird geändert …" : info.paused ? "Fortsetzen" : "Pause", () => sende("lokal_steuern",{body:{paused:!info.paused}}), "gold", info.paused ? "los" : "pause", lokal.busy || lokal.steuert) : null,
      aktiv && lokal.own_host ? knopf(lokal.speichert ? "Wird gespeichert …" : "Speichern", () => sende("lokal_speichern"), "", "haken", lokal.speichert || lokal.busy || lokal.steuert) : null,
      z.laeuft ? knopf(z.pausiert ? "Rat fortsetzen" : "Rat pausieren", () => harness({ befehl: z.pausiert ? "weiter" : "pause" }), "leise", z.pausiert ? "los" : "pause") : null,
      z.laeuft ? h("button.k.rot.gross.los", { onclick: () => sende("stopp"), title: "Harness anhalten; die Steuerung geht an dich zurück" }, ikon("stopp", true), "STOPP")
               : angemeldet ? (S.ui.seite === "rat" ? knopf("Modellrat starten", () => sende("los"), "leise", "los") : null) : lokal.busy ? null : h("button.k.gold.gross.los", { onclick: () => sende("los"), title: "Einstellungen übernehmen, anmelden (falls nötig) und den Rat einberufen" }, ikon("los", true), "LOS")));
}

function schiene() {
  const z = S.zustand;
  const marker = { bruecke: z.wartend?.length || 0, werkstatt: z.gebaut ? 0 : "!", plan: (S.daten?.aufgaben || []).filter((a) => a.status === "offen").length || 0 };
  return h("nav.schiene", null,
    SEITEN.map(([k, t, i, hilfe]) => h("div.navk" + (S.ui.seite === k && !S.ui.codex.offen ? ".an" : ""), { title: hilfe, onclick: () => { S.ui.codex.offen = false; nav(k); } },
      ikon(i, true), t, marker[k] ? h("span.marker", null, String(marker[k])) : null)),
    h("div.trenner"),
    h("div.navk" + (S.ui.codex.offen ? ".an" : ""), { title: "Codex: Schiffe, Gebäude, Forschung, Völker, Regeln – mit 3D-Rat", onclick: () => { S.ui.codex.offen = !S.ui.codex.offen; planen(); } }, ikon("codex", true), "Codex"),
    h("div.credits-link", { onclick: () => { S.ui.credits = true; planen(); } }, "Credits"));
}

function testpilotenKarte() {
  const z=S.zustand;
  return karte("Testpiloten auf diesem PC", "Private, vorhandene Spielkonten. Zugangsdaten bleiben auf diesem Rechner und werden weder im Download noch auf der Webseite veröffentlicht.",
    h("div.knopfreihe",null,(z.testpiloten||[]).map(p=>knopf(p.name,()=>{sende("testpilot_anmelden",{name:p.name});S.ui.codex.offen=false;nav("spiel");},"", "los", !p.verfuegbar || z.anmeldung?.laeuft || z.lokal?.connected || z.laeuft))),
    (z.testpiloten||[]).some(p=>p.verfuegbar)?null:h("p.matt.klein",null,"Die private Zugangsliste testpiloten-zugaenge.txt wird nur im lokalen Harness-Ordner gelesen."));
}

const CREDITS = [
  ["Ratsfiguren", [["Ganzkörperfiguren aus den Spielporträts", "Qwen-Image 2.1 (Qwen-Team, Alibaba)"], ["3D-Modelle", "TRELLIS.2 (Microsoft)"], ["Freistellen", "BiRefNet (Zheng Peng u. a.)"], ["Berechnet mit", "ComfyUI, lokal"]]],
  ["Darstellung", [["3D im Cockpit", "three.js"], ["Fenster und Weboberfläche", "eframe/egui, wry mit WebView2"], ["Bilder des Spiels", "Sternenepoche-Grafik, KI-generiert. Dazu gehören auch kleine Rohstoff-, Credits-, Energie- und andere Spielsymbole ohne Einzelmarkierung."]]],
  ["Harness", [["Sprache", "Rust"], ["Gedächtnis", "SQLite (rusqlite), DuckDB"], ["Plugins", "wasmi (WebAssembly)"], ["Netz", "reqwest, tiny_http"], ["Protokoll", "Model Context Protocol (MCP)"]]],
  ["Modelle", [["Lokal", "Ollama"], ["Cloud", "OpenRouter"], ["Eigene Server", "jede OpenAI-kompatible Schnittstelle (LM Studio, vLLM, llama.cpp)"]]],
];
function creditsDialog() {
  if (!S.ui.credits) return null;
  const zu = () => { S.ui.credits = false; planen(); };
  return dialog([h("h2", null, "Credits"), h("p.matt", null, "Sternenepoche Harness-Cockpit – mit Dank an die Werkzeuge und Modelle, die es möglich machen."),
    h("div.spalte", { stil: { gap: "14px", marginTop: "8px" } }, CREDITS.map(([titel, zeilen]) => h("div", null, h("div.klein.gold", { stil: { letterSpacing: ".14em", textTransform: "uppercase", marginBottom: "6px" } }, titel),
      h("div.liste", null, zeilen.map(([was, wer]) => h("div.eintrag", null, h("div.text", null, h("span.winzig.matt", null, was), h("b.klein", null, wer)))))))),
    h("div.knopfreihe", null, knopf("Schließen", zu, "gold"))]);
}

/** Pfad einer Einstellung lesbar: /rollen/verwalter/modell → Verwalter · Modell. */
function feldName(p) {
  const t = p.split("/").filter(Boolean);
  return t.map((x, i) => (t[0] === "rollen" && i === 1 ? ROLLENTITEL[x] || x : name(x.replace(/_/g, " ")))).filter((x, i) => !(t[0] === "rollen" && i === 0)).join(" · ");
}
function aenderungsleiste() {
  const z = S.zustand;
  if (!z.gebaut && !laeuft() && !z.lokal?.connected && !z.lokal?.busy && S.ui.seite !== "werkstatt") {
    return h("div.aenderung", null, ikon("werkstatt"), h("b", null, "Dein Modellrat ist noch nicht gebaut"),
      h("span.matt.klein", null, "Offline spielen kannst du sofort. Den optionalen Modellrat baust du in der Werkstatt."),
      h("span.rechts"), knopf("Zur Werkstatt", () => nav("werkstatt"), "gold klein", "pfeil"));
  }
  if (!z.geaendert) return null;
  const liste = (z.aenderungen || []).map(feldName);
  return h("div.aenderung", null, ikon("stern"), h("b", null, "Geändert"),
    h("span.klein", { title: liste.join("\n") }, liste.slice(0, 4).join(", ") + (liste.length > 4 ? ` und ${liste.length - 4} weitere` : "")),
    h("span.matt.klein", null, laeuft() ? "· wirkt nach Übernehmen sofort im laufenden Rat" : "· gilt nach Übernehmen beim nächsten Start"),
    h("span.rechts"), knopf("Verwerfen", () => sende("verwerfen"), "leise klein"), knopf("Übernehmen", () => sende("uebernehmen"), "gold klein", "haken"));
}

const meldungSeit = new Map();
function meldungsband() {
  const m = S.zustand.meldung;
  if (!m) return null;
  // Hinweise verschwinden nach 8 s von selbst (Fehler bleiben, bis man sie schließt); veraltete sofort.
  if (!meldungSeit.has(m.text)) { meldungSeit.set(m.text, Date.now()); if (!m.fehler) setTimeout(() => { if (S.zustand.meldung?.text === m.text) sende("meldung_weg"); }, 8000); }
  if (laeuft() && /mit ▶ LOS/.test(m.text)) { sende("meldung_weg"); return null; }
  return h("div.band" + (m.fehler ? ".fehler" : ""), null, h("span", null, m.text), knopf("", () => sende("meldung_weg"), "leise klein", "kreuz"));
}

// ---------------------------------------------------------------- Dialoge
function anmeldungDialog() {
  const a = S.zustand.anmeldung, info = S.zustand.server.info;
  if (!a.offen) return null;
  S.ui.anm = S.ui.anm || { name: "", passwort: "", otp: "", regeln: false };
  const f = S.ui.anm;
  const voelker = ["veyari", "aurelianer", "krath", "syntheten"];
  return dialog([
    h("h2", null, "Im Spiel anmelden"),
    h("p.matt", null, `Server ${S.auswahl.server}. Das Passwort wird nur für diese Anmeldung gesendet und nie gespeichert. Danach erhält der Rat einen eigenen Harness-Schlüssel nur für seine Rollen – sicher im Anmeldeinformationsspeicher von Windows.`),
    h("div.formular", null,
      h("label", null, "Name"), h("input.feld", { value: f.name, oninput: (e) => f.name = e.target.value, autocomplete: "username" }),
      h("label", null, "Passwort"), h("input.feld", { type: "password", value: f.passwort, oninput: (e) => f.passwort = e.target.value, autocomplete: "current-password" }),
      h("label", null, "Authenticator-Code"), h("input.feld", { value: f.otp, placeholder: "falls aktiviert", oninput: (e) => f.otp = e.target.value })),
    info ? h("details", { open: true }, h("summary.gold", null, `Regeln für Harnesses und Fair Play (Version ${info.regeln?.version || "?"})`),
      h("ol.klein", null, (info.regeln?.punkte || []).map((p) => h("li", null, p)))) : h("p.bernstein", null, S.zustand.server.fehler || "Regeln werden geladen …"),
    schalter(f.regeln, "Ich habe die Regeln gelesen und halte mich daran.", (v) => { f.regeln = v; planen(); }),
    a.platz_noetig ? h("div", null, h("div.trenner"), h("p", null, a.warteliste ? `Du stehst auf Wartelistenplatz ${a.warteliste}.` : "Dein Konto hat noch kein Reich. Wähle ein Volk:"),
      h("div.kacheln", null, voelker.map((v) => h("div.kachel.bild" + (a.volk === v ? ".an" : ""), { onclick: () => sende("platz", { volk: v }) },
        h("div.grund", { stil: { backgroundImage: `url(${bild(`civilizations.${v}.${v}`)})` } }), h("b", null, name(v)))))) : null,
    a.fehler ? h("p.rot", null, a.fehler) : null,
    h("div.knopfreihe", null, knopf("Abbrechen", () => { f.passwort = ""; sende("anmeldung_schliessen"); }, "leise"),
      knopf(a.laeuft ? "Anmeldung läuft …" : "Anmelden und Rat einberufen", () => { sende("anmelden", { name: f.name, passwort: f.passwort, otp: f.otp, regeln_ok: f.regeln }); f.passwort = ""; }, "gold", "haken", a.laeuft || !f.regeln || !f.name || !f.passwort)),
  ]);
}
function fairplayDialog() {
  if (!S.zustand.fairplay) return null;
  return dialog([h("h2.rot", null, "Fair-Play-Verwarnung"), h("p", null, S.zustand.fairplay), h("p.matt", null, "Bis zur Bestätigung sind Spielzüge gesperrt. Ein weiterer Verstoß sperrt das Konto."),
    h("div.knopfreihe", null, knopf("Verstanden – ich halte mich an die Regeln", () => sende("fairplay_ok"), "gold"))]);
}

// ---------------------------------------------------------------- Zeichnen
let zuletztSpiel = "";
function zeichnen() {
  const wurzel = document.getElementById("wurzel");
  if (!S.katalog || !S.zustand || !S.auswahl) {
    morphKinder(wurzel, h("div", null, h("div", { stil: { gridColumn: "1 / -1", gridRow: "1 / -1", display: "flex", alignItems: "center", justifyContent: "center", flexDirection: "column", gap: "16px" } },
      h("img", { src: "bilder/marke.png", alt: "", stil: { width: "120px", filter: "drop-shadow(0 0 24px rgba(127,211,255,.5))" } }), h("div.gold", null, "Das Cockpit erwacht …"))));
    return;
  }
  const seite = SEITENFN[S.ui.seite] ? SEITENFN[S.ui.seite]() : seiteBruecke();
  const neu = h("div", null, kopf(), schiene(),
    h("main.inhalt", null, meldungsband(), seite),
    S.ui.codex.offen ? h("div.codex-schleier", { onclick: () => { S.ui.codex.offen = false; planen(); } }) : null,
    seiteCodex());
  morphKinder(wurzel, neu);
  const schicht = document.getElementById("schicht");
  morphKinder(schicht, h("div", null, anmeldungDialog(), fairplayDialog(), einrichtungDialog(), creditsDialog(), Dreid.grossDialog()));
  nachZeichnen();
}

/** Nach dem Zeichnen: 3D-Bühnen starten, Spielbereich an Rust melden. */
/** KI-Kennzeichnung: jedes KI-erzeugte Bild (Spielbilder, Porträts, Figuren) und jede 3D-Bühne trägt unten rechts
    ein kleines „KI“. Kleine Symbole und Avatare (unter 56 px) bleiben frei; der Hinweis steht zusätzlich in den Credits. */
function kiMarken() {
  for (const m of document.querySelectorAll(".ki-marke")) m.remove();
  const marke = (wirt, ecke, x, y) => {
    if (getComputedStyle(wirt).position === "static") wirt.style.position = "relative";
    const m = document.createElement("span"); m.className = "ki-marke" + (ecke ? " ecke" : ""); m.textContent = "KI"; m.title = "KI-generiertes Bild";
    if (!ecke) { m.style.left = x + "px"; m.style.top = y + "px"; }
    wirt.appendChild(m);
  };
  for (const img of document.querySelectorAll("img")) {
    if (img.offsetWidth < 56 || img.offsetHeight < 56 || !img.offsetParent || !/bilder\/|figuren\//.test(img.getAttribute("src") || "")) continue;
    if (img.closest(".buehne, .rundbuehne")) continue;  // die Bühne trägt schon ein Schild
    const wirt = img.parentElement;
    if (getComputedStyle(wirt).position === "static") wirt.style.position = "relative";
    marke(wirt, false, img.offsetLeft + img.offsetWidth - 25, img.offsetTop + img.offsetHeight - 19);
  }
  for (const el of document.querySelectorAll(".buehne, .rundbuehne, [style*='background-image']")) {
    if (el.offsetWidth < 56 || el.offsetHeight < 56 || !el.offsetParent) continue;
    if (!el.classList.contains("buehne") && !el.classList.contains("rundbuehne") && !/bilder\/|figuren\//.test(el.getAttribute("style") || "")) continue;
    if (!el.classList.contains("buehne") && !el.classList.contains("rundbuehne") && el.closest(".buehne, .rundbuehne")) continue;
    marke(el, true);
  }
}

function nachZeichnen() {
  Dreid.pflegen();
  kiMarken();
  if (window.Stimmen && Stimmen.beobachten) Stimmen.beobachten();
  if (window.Rund) Rund.pflegen();
  if (window.Hausbuehne) Hausbuehne.pflegen();
  const platz = document.getElementById("spielplatz");
  const verdeckt = S.ui.codex.offen || !!document.querySelector(".dialog-grund");
  let meldung = { sichtbar: false };
  if (platz && !verdeckt) { const r = platz.getBoundingClientRect(); meldung = { sichtbar: true, x: Math.round(r.left), y: Math.round(r.top), b: Math.round(r.width), h: Math.round(r.height) }; }
  const text = JSON.stringify(meldung);
  if (text !== zuletztSpiel) { const neuLaden = !zuletztSpiel.includes('"sichtbar":true') && meldung.sichtbar; zuletztSpiel = text; sende("spielbereich", Object.assign({ neu_laden: neuLaden }, meldung)); }
}
window.addEventListener("resize", () => { zuletztSpiel = ""; planen(); });
setInterval(planen, 1000); // Countdown-Anzeigen
document.addEventListener("keydown", (e) => { if (e.key === "Escape" && S.ui.codex.offen) { S.ui.codex.offen = false; planen(); } });
sende("hallo");
