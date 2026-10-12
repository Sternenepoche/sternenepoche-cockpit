/* Labor: messen, vergleichen, feinjustieren – und aus guten Zügen eigene Modelle trainieren. */
"use strict";

function laborProbe() {
  const p = S.daten?.probezuege || [];
  return h("div.spalte", null,
    h("div.zeile", null, laeuft() ? knopf("Probezug für alle Rollen im Rat", () => ROLLEN.filter((r) => rolleW(r).aktiv).forEach((r) => harness({ befehl: "probezug", rolle: r })), "gold", "probe") : h("span.matt", null, "Probezüge brauchen den laufenden Rat (▶ LOS)."),
      h("span.matt.klein", null, "Jede Rolle entscheidet mit ihrer aktuellen Seele; der Spielkern prüft, nichts wird gespielt.")),
    h("div.raster.r4", null, ROLLEN.map((r) => h("section.karte", { stil: { borderTop: `3px solid ${ROLLENFARBE[r]}` } }, h("div.zeile", null, avatar(r, 34), h("b", null, ROLLENTITEL[r]), h("span.rechts"),
      laeuft() && rolleW(r).aktiv ? knopf("Probe", () => harness({ befehl: "probezug", rolle: r }), "klein") : null), h("div", { stil: { marginTop: "10px" } }, probezugKarte(r))))));
}

function laborArena() {
  const f = S.ui.arena = S.ui.arena || { rolle: "verwalter", a: { modell: "", vorlage: "" }, b: { modell: "", vorlage: "" }, c: { modell: "", vorlage: "" }, dritte: false };
  f.c = f.c || { modell: "", vorlage: "" };
  const w = rolleW(f.rolle), vorlagen = S.katalog.vorlagen.filter((v) => v.rolle === f.rolle);
  const ergebnis = S.daten?.arena, laeuftA = S.zustand.arena_laeuft;
  const modelle = (S.zustand.ollama || []).map((m) => m.name);
  const variante = (k, t) => h("section.karte.hell", null, h("h2", null, t),
    h("div.formular", { stil: { gridTemplateColumns: "90px 1fr" } },
      h("label", null, "Modell"), h("select.feld", { onchange: (e) => { f[k].modell = e.target.value; planen(); } }, [["", `wie eingestellt (${w.modell || "–"})`]].concat(modelle.map((m) => [m, m])).map(([v, tt]) => h("option", { value: v, selected: f[k].modell === v }, tt))),
      h("label", null, "Seele"), h("select.feld", { onchange: (e) => { f[k].vorlage = e.target.value; planen(); } }, [["", "aktuelle soul.md"]].concat(vorlagen.map((v) => [v.name, "Vorlage " + v.name])).map(([v, tt]) => h("option", { value: v, selected: f[k].vorlage === v }, tt)))));
  const start = () => {
    const v = (k, t) => { const x = { name: t + (f[k].modell ? " · " + f[k].modell : "") + (f[k].vorlage ? " · " + f[k].vorlage : ""), anbieter: f[k].modell ? "ollama" : null, modell: f[k].modell || null };
      if (f[k].vorlage) x.seele = (S.katalog.vorlagen.find((y) => y.name === f[k].vorlage) || {}).seele; return x; };
    harness({ befehl: "arena", rolle: f.rolle, varianten: [v("a", "A"), v("b", "B")].concat(f.dritte ? [v("c", "C")] : []) });
  };
  return h("div.spalte", null,
    karte("Arena", "Zwei oder drei Varianten derselben Rolle entscheiden über dieselbe Lage – anderes Modell, andere Seele. Der Spielkern prüft jede, gespielt wird nichts. So findest du heraus, was wirklich besser spielt.",
      h("div.zeile", null, ROLLEN.map((r) => h("span", { stil: { cursor: "pointer", opacity: f.rolle === r ? "1" : ".4" }, onclick: () => { f.rolle = r; planen(); } }, avatar(r, 38))), h("b", { stil: { marginLeft: "6px" } }, ROLLENTITEL[f.rolle])),
      h("div.raster." + (f.dritte ? "r3" : "r2"), { stil: { marginTop: "12px" } }, variante("a", "Variante A"), variante("b", "Variante B"), f.dritte ? variante("c", "Variante C") : null),
      h("div.zeile", { stil: { marginTop: "8px" } }, schalter(f.dritte, "dritte Variante", (x) => { f.dritte = x; planen(); })),
      h("div.zeile", { stil: { marginTop: "12px" } }, knopf(laeuftA ? "Arena läuft …" : "Vergleichen", start, "gold", "los", !laeuft() || laeuftA), laeuftA ? h("span.lade") : null,
        laeuft() ? null : h("span.matt.klein", null, "Braucht den laufenden Rat."))),
    ergebnis?.varianten ? h("div.raster." + (ergebnis.varianten.length > 2 ? "r3" : "r2"), null, ergebnis.varianten.map((v) => h("section.karte", null, h("h2", null, v.name || "Variante"),
      v.fehler ? h("p.rot", null, v.fehler) : h("div.spalte", null,
        h("div.zeile", null, chip(`${zahl(v.tokens)} Tokens`), chip(`${zahl(v.latenz_ms)} ms`), chip(`${v.gueltig}/${v.aktionen?.length || 0} gültig`, v.gueltig === (v.aktionen?.length || 0) && v.gueltig ? "gruen" : "bernstein"), chip("Seele " + (v.seele_version || "–"))),
        h("div.klein", { stil: { fontStyle: "italic" } }, `„${v.begruendung || ""}“`),
        h("div.liste", null, (v.aktionen || []).map((a, i) => h("div.eintrag", null, h("img.bildchen", { src: aktionBild(a), alt: "" }), h("div.text", null, h("b", null, aktionText(a)),
          v.probe?.[i] ? h("span.winzig" + (v.probe[i].ok ? ".gruen" : ".rot"), null, (v.probe[i].ok ? "✔ " : "✖ ") + (v.probe[i].text || "")) : null)))),
        (v.wartend || []).length ? h("div.winzig.bernstein", null, `${v.wartend.length} Aktion(en) würden dich fragen`) : null)))) : null);
}

function laborKontext() {
  return h("div.raster.r2", null, ROLLEN.filter((r) => rolleW(r).aktiv).map((r) => h("section.karte", null, h("div.zeile", null, avatar(r, 30), h("b", null, ROLLENTITEL[r]), h("span.rechts"),
    laeuft() ? knopf("Vorschau", () => harness({ befehl: "kontext", rolle: r }), "klein", "kontext") : null, knopf("Im Rat einstellen", () => nav("rat", { rolle: r, stufe: "kontext" }), "leise klein")),
    h("div", { stil: { marginTop: "10px" } }, kontextBudget(r)))));
}

function laborKennzahlen() {
  const k = Array.isArray(S.daten?.kennzahlen) ? S.daten.kennzahlen : [];
  const reihe = (feld) => k.map((x) => x[feld]).filter((x) => x !== undefined && x !== null);
  const r = S.zustand.rollen || {};
  return h("div.spalte", null,
    h("div.zeile", null, laeuft() ? knopf("Aktualisieren", () => harness({ befehl: "kennzahlen" }), "klein") : null, h("span.matt.klein", null, `${k.length} Messpunkte aus der Routine Inventur`)),
    h("div.raster.r3", null, [["punkte", "Punkte", "#dab57b"], ["credits", "Credits", "#f3d9a6"], ["bevoelkerung", "Einwohner", "#8fdca5"], ["erz", "Erz", "#9fb6cc"], ["kristall", "Kristall", "#7fd3ff"], ["energie_saldo", "Energiesaldo", "#f4b467"]].map(([f, t, c]) => {
      const w = reihe(f);
      return karte(t, w.length ? `zuletzt ${zahl(w[w.length - 1])}` : "noch keine Daten", sparkline(w, c, 90));
    })),
    karte("Rollen im Vergleich", "Seit Start dieser Sitzung", h("table.tab", null, h("tr", null, ["Rolle", "Modell", "Züge", "angenommen", "abgelehnt", "Quote", "Tokens zuletzt", "Latenz", "Kosten"].map((x) => h("th", null, x))),
      ROLLEN.map((x) => { const a = r[x] || {}, s = (a.angenommen || 0) + (a.abgelehnt || 0);
        return h("tr", null, h("td", null, h("span.zeile.eng", null, avatar(x, 22), ROLLENTITEL[x])), h("td.mono", null, a.modell || rolleW(x).modell || "–"), h("td", null, zahl(a.aufrufe || 0)), h("td.gruen", null, zahl(a.angenommen || 0)), h("td.rot", null, zahl(a.abgelehnt || 0)),
          h("td", null, s ? Math.round((a.angenommen || 0) / s * 100) + " %" : "–"), h("td", null, zahl(a.tokens || 0)), h("td", null, `${zahl(a.latenz || 0)} ms`), h("td", null, `${(a.kosten || 0).toFixed(4)} $`)); }))));
}

function laborVerlauf() {
  const v = S.daten?.verlauf || [], wahl = S.ui.labor.verlauf;
  const d = wahl !== null && wahl !== undefined ? S.detail["verlauf" + wahl] : null;
  return h("div.raster", { stil: { gridTemplateColumns: "minmax(320px, 0.9fr) minmax(0, 1.6fr)" } },
    karte("Modellaufrufe", "Jeder Aufruf mit Modell, Tokens und Latenz", v.length ? h("div.liste", { stil: { maxHeight: "70vh", overflow: "auto" } }, v.map((e) => h("div.eintrag.wahl" + (wahl === e.i ? ".an" : ""),
      { onclick: () => { S.ui.labor.verlauf = e.i; sende("verlauf_detail", { i: e.i }); planen(); } }, avatar(e.rolle, 28),
      h("div.text", null, h("b", null, `${ROLLENTITEL[e.rolle] || e.rolle} · ${e.modell}`), h("span.winzig.matt", null, `${zahl(e.tokens)} Tokens · ${zahl(e.latenz_ms)} ms · ${vor(e.zeit)}`))))) : leer("Noch keine Modellaufrufe.")),
    d ? h("div.spalte", null,
      karte("Antwort des Modells", `${d.modell} · ${zahl(d.tokens)} Tokens · ${zahl(d.latenz_ms)} ms`, h("pre.code", null, (() => { try { return JSON.stringify(JSON.parse(d.antwort), null, 2); } catch (e) { return d.antwort; } })())),
      karte("Lage und Kontext (Prompt)", null, h("pre.code", null, d.nutzer || "")),
      karte("Systemprompt", null, h("pre.code", null, d.system || ""))) : karte("Detail", null, leer("Einen Aufruf links wählen.")));
}

function laborTraining() {
  return h("div.raster.r2", null,
    karte("Trainingsdaten aus deinen Zügen", "Jeder Modellzug wird mit Ergebnis gespeichert (Tabelle training). Exportiere nur die Züge, die der Server angenommen hat, als Chat-JSONL – direkt nutzbar für Fine-Tuning (z. B. Unsloth, Axolotl, LLaMA-Factory).",
      h("div.zeile", null, knopf("Nur erfolgreiche Züge exportieren", () => harness({ befehl: "training_export", nur_erfolgreich: true }), "gold", "lernen", !laeuft()),
        knopf("Alles exportieren", () => harness({ befehl: "training_export", nur_erfolgreich: false }), "", null, !laeuft()), knopf("Ordner öffnen", () => sende("oeffnen", { was: "training" }), "leise", "ordner")),
      S.zustand.export ? h("p.tipp", null, "Zuletzt: " + S.zustand.export) : null,
      h("div.trenner"),
      h("div.klein", null, "Format je Zeile:"), h("pre.code", null, '{"messages":[{"role":"system","content":"…"},\n  {"role":"user","content":"…"},\n  {"role":"assistant","content":"{…}"}],\n "meta":{"rolle":"verwalter","angenommen":2,"seele":"1fcc02f7"}}'),
      knopf("Verlauf als JSONL (alle Aufrufe und Züge)", () => harness({ befehl: "export" }), "leise klein", "kopie", !laeuft())),
    karte("Eigenes Modell zurück in den Rat", "Der Kreislauf: spielen → aufzeichnen → auswählen → trainieren → einsetzen → in der Arena vergleichen.",
      h("ol.klein", null,
        h("li", null, "Trainieren mit deinem Werkzeug (LoRA auf Qwen, Llama, Mistral …)."),
        h("li", null, "Als GGUF in Ollama laden: ", h("span.mono", null, "ollama create mein-verwalter -f Modelfile"), " – erscheint sofort im Rat unter Modell."),
        h("li", null, "Oder über einen OpenAI-kompatiblen Server (LM Studio, vLLM, llama.cpp) unter Werkstatt → Modelle anbinden."),
        h("li", null, "In der Arena gegen das Grundmodell antreten lassen, dann im Rat einsetzen.")),
      knopf("Zur Arena", () => { S.ui.labor.reiter = "arena"; planen(); }, "klein", "pfeil")));
}

function seiteLabor() {
  const r = S.ui.labor.reiter;
  return h("div.seite", null,
    h("div.seitenkopf", null, h("div", null, h("h1", null, "Labor"), h("p", null, "Messen, vergleichen, feinjustieren. Hier entsteht ein Harness, der wirklich gut spielt – und aus guten Zügen ein eigenes Modell."))),
    h("div", { stil: { marginBottom: "16px" } }, segment([["probe", "Probezug"], ["arena", "Arena"], ["kontext", "Kontext"], ["kennzahlen", "Kennzahlen"], ["verlauf", "Modellaufrufe"], ["training", "Training"]], r, (v) => { S.ui.labor.reiter = v; planen(); }, true)),
    { probe: laborProbe, arena: laborArena, kontext: laborKontext, kennzahlen: laborKennzahlen, verlauf: laborVerlauf, training: laborTraining }[r]());
}
