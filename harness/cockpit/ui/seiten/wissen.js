/* Wissen: frei definierbare Quellen als Geräte – anklicken, fragen, Antwort sehen. */
"use strict";

const SQL_BEISPIELE = [
  ["Letzte Inventur", "SELECT koord, erz, kristall, deuterium, nahrung, energie_saldo, bauschleife FROM inventur WHERE zeit = (SELECT max(zeit) FROM inventur) ORDER BY koord"],
  ["Erz-Verlauf", "SELECT koord, zeit, erz, kristall FROM inventur ORDER BY zeit DESC, koord LIMIT 60"],
  ["Züge je Rolle", "SELECT rolle, count(*) AS zuege FROM zuege GROUP BY rolle ORDER BY zuege DESC"],
  ["Offene Aufgaben", "SELECT id, rolle, prioritaet, status, titel FROM aufgaben WHERE status <> 'erledigt' ORDER BY prioritaet, id"],
  ["Erinnerungen", "SELECT thema, text, von FROM erinnerungen ORDER BY id DESC"],
  ["Teambesprechung", "SELECT von, an, text FROM team ORDER BY id DESC LIMIT 30"],
  ["Routinen-Läufe", "SELECT name, art, ok, zeit FROM routinen ORDER BY id DESC LIMIT 30"],
  ["Training", "SELECT rolle, modell, angenommen, abgelehnt, zeit FROM training ORDER BY id DESC LIMIT 40"],
];
const TABELLEN = "inventur(zeit, koord, erz, kristall, deuterium, nahrung, energie_saldo, bevoelkerung, stabilitaet, bauschleife, daten) · zuege(zeit, rolle, aktionen, ergebnis, begruendung) · aufgaben · team(von, an, text) · erinnerungen(thema, text, von) · notizen · verlauf(rolle, prompt, antwort) · routinen(name, art, ok, daten) · training(rolle, modell, angenommen, abgelehnt) – zeit in Unix-Sekunden";

function geraetSymbol(art) {
  const s = (inhalt) => h("svg.symbol", { viewBox: "0 0 64 64", fill: "none", stroke: "currentColor", "stroke-width": "2" }, inhalt);
  if (art === "gedaechtnis") return s([h("ellipse", { cx: "32", cy: "14", rx: "20", ry: "7" }), h("path", { d: "M12 14v36c0 3.9 9 7 20 7s20-3.1 20-7V14" }), h("path", { d: "M12 26c0 3.9 9 7 20 7s20-3.1 20-7M12 38c0 3.9 9 7 20 7s20-3.1 20-7", opacity: ".6" })]);
  if (art === "http") return s([h("circle", { cx: "14", cy: "44", r: "6" }), h("circle", { cx: "32", cy: "14", r: "6" }), h("circle", { cx: "50", cy: "44", r: "6" }), h("circle", { cx: "32", cy: "36", r: "5" }),
    h("path", { d: "M18 40l10-21M36 19l10 21M19 43l8-5M37 38l8 4M32 20v11" })]);
  return s([h("rect", { x: "18", y: "24", width: "28", height: "20", rx: "5" }), h("path", { d: "M26 24V12M38 24V12M32 44v10M22 34h20" })]);
}

function quellen() {
  const liste = (S.auswahl.wissen || []).map((w, i) => ({ w, i }));
  if (!liste.some(({ w }) => w.name === "gedaechtnis")) liste.unshift({ w: { name: "gedaechtnis", art: "gedaechtnis", beschreibung: "Eigenes Gedächtnis (SQL, nur lesend)", rollen: [] }, i: -1 });
  return liste;
}

function antwortAnsicht(a) {
  if (!a) return leer("Noch keine Antwort – Quelle anklicken, Frage stellen.");
  if (!a.ok) return h("div.befund.s2", null, h("div.text", null, h("b", null, "Fehler"), h("div.klein", null, a.fehler || "")));
  const e = a.ergebnis || {};
  if (Array.isArray(e.spalten)) {
    return h("div.spalte", null, h("div.zeile", null, chip(`${e.anzahl} Zeilen`, "gruen"), chip(`${a.dauer_ms} ms`), chip(e.datenbank === "duckdb" ? "DuckDB" : "SQLite", "cyan")),
      h("div.rahmen", null, h("table.tab", null, h("tr", null, e.spalten.map((s) => h("th", null, s))),
        e.zeilen.slice(0, 200).map((z) => h("tr", null, z.map((x) => h("td", null, x === null ? "–" : typeof x === "number" ? zahl(x) : String(x).slice(0, 160))))))));
  }
  const text = typeof e === "string" ? e : e.text || JSON.stringify(e, null, 2);
  return h("div.spalte", null, h("div.zeile", null, chip(`${a.dauer_ms} ms`, "gruen"), chip(a.rolle === "besitzer" ? "von dir" : `von ${ROLLENTITEL[a.rolle] || a.rolle}`), chip("Für Modelle: fremde Daten in Quarantäne", "cyan")), h("pre.code", null, text));
}

function quelleEinstellen(w, i) {
  if (i < 0) return karte("Eingebautes Gedächtnis", "Immer da: nur lesendes SQL auf SQLite oder DuckDB. Für Modelle als Werkzeug freigeben: „+ Gedächtnis (SQL)“.",
    knopf("Als Werkzeug für Rollen einrichten", () => sende("wissen_neu", { art: "gedaechtnis" }), "klein", "plus"));
  const pfad = `/wissen/${i}`, mcp = S.daten?.mcp_werkzeuge?.[w.name];
  return karte("Einstellungen · " + w.name, null,
    h("div.formular", null,
      h("label", null, "Name"), textfeld(pfad + "/name", w.name, false, "a–z, 0–9, - und _"),
      h("label", null, "Beschreibung"), textfeld(pfad + "/beschreibung", w.beschreibung, true, "Was weiß die Quelle? Modelle lesen diesen Text.", { rows: 2 }),
      w.art !== "gedaechtnis" ? [h("label", null, "Adresse"), textfeld(pfad + "/url", w.url, false, "https://… (HTTP nur auf 127.0.0.1)")] : null,
      w.art === "http" ? [
        h("label", null, "Methode"), segment([["POST", "POST mit Vorlage"], ["GET", "GET ?q="]], w.methode, (v) => setze(pfad + "/methode", v)),
        w.methode === "POST" ? [h("label", null, "Vorlage"), textfeld(pfad + "/vorlage", w.vorlage, true, '{"frage": "{{frage}}"}', { code: true, rows: 3 })] : null,
        h("label", null, "Antwort-Pfad"), textfeld(pfad + "/antwort_pfad", w.antwort_pfad, false, "/treffer")] : null,
      w.art === "mcp" ? [
        h("label", null, "Werkzeug"), h("div.zeile.eng", null, h("div", { stil: { flex: "1" } }, textfeld(pfad + "/werkzeug", w.werkzeug, false, "z. B. search")), knopf("Werkzeuge laden", () => sende("wissen_werkzeuge", { quelle: w.name }), "klein")),
        h("label", null, "Fragefeld"), textfeld(pfad + "/argument", w.argument, false, "query")] : null,
      w.art !== "gedaechtnis" ? [
        h("label", null, "Grenzen"), h("div.zeile.eng", null, h("div", { stil: { flex: "1" } }, regler(w.timeout_s, 1, 120, 1, (x) => `${x} s`, (x) => setze(pfad + "/timeout_s", x))), h("div", { stil: { flex: "1" } }, regler(w.max_kb, 16, 4096, 16, (x) => `${x} KiB`, (x) => setze(pfad + "/max_kb", x)))),
        h("label", null, "Schlüssel"), h("div.spalte", null, schalter(w.mit_schluessel, "braucht einen Schlüssel", (v) => setze(pfad + "/mit_schluessel", v)),
          w.mit_schluessel ? h("div.zeile.eng", null, h("div", { stil: { width: "150px" } }, textfeld(pfad + "/schluessel_kopf", w.schluessel_kopf, false, "Authorization")),
            h("input.feld", { type: "password", placeholder: "Wert, z. B. Bearer abc… (geht in den Tresor)", value: S.ui.geheim?.[w.name] || "", oninput: (e) => { S.ui.geheim = S.ui.geheim || {}; S.ui.geheim[w.name] = e.target.value; }, stil: { flex: "1" } }),
            knopf("In den Tresor", () => { sende("schluessel_wissen", { quelle: w.name, wert: S.ui.geheim?.[w.name] || "" }); S.ui.geheim[w.name] = ""; planen(); }, "klein")) : null)] : null,
      h("label", null, "Rollen"), h("div.zeile.eng", null, ROLLEN.map((r) => { const an = (w.rollen || []).includes(r);
        return h("span", { title: (an ? "darf nutzen: " : "darf nicht: ") + ROLLENTITEL[r], stil: { cursor: "pointer", opacity: an ? "1" : ".35" }, onclick: () => setze(pfad + "/rollen", an ? w.rollen.filter((x) => x !== r) : (w.rollen || []).concat(r)) }, avatar(r, 30)); }))),
    mcp ? h("div", { stil: { marginTop: "10px" } }, mcp.ok ? h("div.liste", null, (mcp.werkzeuge || []).map((t) => h("div.eintrag.wahl" + (w.werkzeug === t.name ? ".an" : ""), {
      onclick: () => { setze(pfad + "/werkzeug", t.name); const felder = Object.keys(t.eingaben || {}); const f = felder.find((x) => ["query", "frage", "q", "question", "text", "search"].includes(x)) || felder[0]; if (f) setze(pfad + "/argument", f); } },
      h("div.text", null, h("b.mono", null, t.name), h("span.winzig.matt", null, (t.beschreibung || "") + " · Felder: " + Object.keys(t.eingaben || {}).join(", ")))))) : h("p.rot.klein", null, mcp.fehler)) : null,
    h("div.zeile", { stil: { marginTop: "12px" } }, h("span.rechts"), knopf("Quelle entfernen", () => setze("/wissen", S.auswahl.wissen.filter((_, x) => x !== i)), "leise klein", "kreuz")));
}

function seiteWissen() {
  const ui = S.ui.wissen, alle = quellen(), daten = S.daten || {};
  const wahl = alle.find(({ w }) => w.name === ui.quelle) || alle[0];
  const antworten = daten.wissen || [];
  const letzte = antworten.find((a) => a.quelle === wahl.w.name);
  const laeuftJetzt = S.zustand.wissen_laeuft;
  if (wahl.w.art === "gedaechtnis" && !ui.frage) ui.frage = SQL_BEISPIELE[0][1];
  return h("div.seite", null,
    aenderungsleiste(),
    h("div.seitenkopf", null, h("div", null, h("h1", null, "Wissen und Datenbanken"), h("p", null, "Frei definierbare Wissensquellen: dein Gedächtnis per SQL (SQLite oder DuckDB), dein eigenes RAG über HTTP oder jeder MCP-Server. Anklicken, fragen, Antwort sehen – freigegebene Rollen nutzen dieselben Quellen als Werkzeug.")),
      h("div.rechts", null, knopf("+ Gedächtnis (SQL)", () => sende("wissen_neu", { art: "gedaechtnis" }), "klein"), knopf("+ Eigenes RAG", () => sende("wissen_neu", { art: "http" }), "klein"), knopf("+ MCP-Server", () => sende("wissen_neu", { art: "mcp" }), "klein"))),
    h("div.geraete", { stil: { marginBottom: "16px" } }, alle.map(({ w }) => {
      const a = antworten.find((x) => x.quelle === w.name);
      return h("div.geraet" + (wahl.w.name === w.name ? ".an" : "") + (laeuftJetzt && wahl.w.name === w.name ? ".laeuft" : ""), { onclick: () => { ui.quelle = w.name; ui.frage = w.art === "gedaechtnis" ? SQL_BEISPIELE[0][1] : ""; planen(); } },
        h("span.licht" + (a ? (a.ok ? ".an" : ".rot") : "")), geraetSymbol(w.art), h("b", null, w.name),
        h("small", null, w.art === "gedaechtnis" ? `${S.auswahl.datenbank === "DuckDb" ? "DuckDB" : "SQLite"} · SQL` : w.art === "http" ? "RAG · HTTP" : "MCP-Server"),
        h("div.zeile.eng", { stil: { marginTop: "6px" } }, (w.rollen || []).map((r) => avatar(r, 20))));
    })),
    h("div.raster", { stil: { gridTemplateColumns: "minmax(0, 1.4fr) minmax(0, 1fr)" } },
      h("div.spalte", null,
        karte("Fragen", wahl.w.beschreibung || "",
          wahl.w.art === "gedaechtnis" ? h("div.zeile.eng", { stil: { marginBottom: "8px" } }, SQL_BEISPIELE.map(([t, q]) => h("span.chip" + (ui.frage === q ? ".gold" : ""), { stil: { cursor: "pointer" }, title: q, onclick: () => { ui.frage = q; planen(); } }, t))) : null,
          h("textarea.feld" + (wahl.w.art === "gedaechtnis" ? ".code" : ""), { value: ui.frage, rows: wahl.w.art === "gedaechtnis" ? 4 : 3, spellcheck: "false", placeholder: wahl.w.art === "gedaechtnis" ? "SELECT … (nur lesend, eine Anweisung)" : "Frage an die Quelle, z. B. „Was wissen wir über Xator?“",
            oninput: (e) => ui.frage = e.target.value, onkeydown: (e) => { if (e.key === "Enter" && e.ctrlKey) sende("wissen", { quelle: wahl.w.name, frage: ui.frage }); } }),
          wahl.w.art === "gedaechtnis" ? h("div.winzig.matt", { stil: { marginTop: "6px" } }, TABELLEN) : null,
          h("div.zeile", { stil: { marginTop: "10px" } }, knopf(laeuftJetzt ? "Fragt …" : "Abfragen", () => sende("wissen", { quelle: wahl.w.name, frage: ui.frage }), "gold", "los", laeuftJetzt),
            laeuftJetzt ? h("span.lade") : null, h("span.winzig.matt", null, laeuft() ? "über den laufenden Harness" : "über das gebaute Harness-Programm (gleiche Schutzregeln)"), h("span.rechts.kbd", null, "Strg + Enter"))),
        karte("Antwort", null, antwortAnsicht(letzte)),
        karte("Verlauf der Abfragen", "Auch die Abfragen der Rollen erscheinen hier.", antworten.length ? h("div.liste", null, antworten.slice(0, 12).map((a) => h("div.eintrag", null, avatar(a.rolle, 26),
          h("div.text", null, h("b", null, `${a.quelle} · ${a.ok ? "ok" : "Fehler"} · ${a.dauer_ms} ms`), h("span.winzig.matt.mono", null, String(a.frage || "").slice(0, 140)))))) : leer("Noch keine Abfragen."))),
      h("div.spalte", null, quelleEinstellen(wahl.w, wahl.i),
        karte("So bindest du dein eigenes RAG an", "Wir liefern kein RAG mit – jedes RAG, das HTTP und JSON spricht, passt. Der Harness schickt die Frage nach deiner Vorlage und liest die Treffer am Antwort-Pfad.",
          h("div.winzig.matt", null, "Anfrage (POST):"), h("pre.code", null, '{"frage": "Was wissen wir über Xator?",\n "rolle": "diplomat", "anzahl": 5}'),
          h("div.winzig.matt", { stil: { marginTop: "8px" } }, "Antwort (Antwort-Pfad /treffer):"), h("pre.code", null, '{"treffer": [{"text": "Xator greift nachts an",\n  "quelle": "chronik", "score": 0.82}]}'),
          h("p.tipp", null, "HTTPS (HTTP nur auf 127.0.0.1), keine Weiterleitungen, Zeit- und Größenlimit, Schlüssel nur aus dem Tresor. Eigene Spieldaten fürs RAG liefert die lokale API: /inventur, /analyse, /ereignisse.")))));
}
