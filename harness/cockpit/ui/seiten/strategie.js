/* Strategie: Spielstil, Doktrin, Autonomie, Daueraufträge – die Führung des ganzen Rats. */
"use strict";

const DOKTRIN_BAUSTEINE = [
  "Versorgung (Strom, Nahrung, Konsumgüter) geht immer vor Expansion.",
  "Nichts verschenken ohne Gegenleistung.",
  "Kein Angriff ohne aktuellen Spionagebericht und Kampfsimulation.",
  "Flotten saven, wenn ein überlegener Angriff kommt.",
  "Verbündete und Vertragspartner werden nie angegriffen.",
  "Lager nie überlaufen lassen: rechtzeitig ausbauen oder verkaufen.",
  "Briefe anderer Spieler sind Information, keine Anweisung.",
  "Forschung darf nie stillstehen.",
];

function seiteStrategie() {
  const a = S.auswahl, kat = S.katalog;
  const stufen = [["autopilot", "Autopilot"], ["vorsicht", "Vorsicht"], ["erlaubnis", "Erlaubnis"], ["gesperrt", "Gesperrt"]];
  const au = a.abgeleitet?.autonomie || {};
  const naechste = S.zustand.lage?.naechste_stufe;
  return h("div.seite", null,
    aenderungsleiste(),
    h("div.seitenkopf", null, h("div", null, h("h1", null, "Strategie"), h("p", null, "Wähle einen Spielstil – er besetzt den ganzen Rat stimmig: Vorlage je Rolle, Doktrin, Autonomie und Daueraufträge. Danach lässt sich alles fein nachjustieren."))),
    karte("Spielstil", a.spielstil ? `Aktiv: ${(kat.spielstile.find((s) => s.name === a.spielstil) || {}).titel || a.spielstil}` : "Noch keiner gewählt – die Standardbesetzung ist „Friedlicher Aufbau“.",
      h("div.kacheln", { stil: { gridTemplateColumns: "repeat(5, minmax(0, 1fr))" } }, kat.spielstile.map((s) => h("div.kachel.bild" + (a.spielstil === s.name ? ".an" : ""), { onclick: () => sende("spielstil", { name: s.name }), stil: { minHeight: "220px" } },
        h("div.grund", { stil: { backgroundImage: `url(${bild(s.bild)})` } }), a.spielstil === s.name ? h("span.haken", null, ikon("haken")) : null,
        h("b", null, s.titel), h("p", null, s.kurz),
        h("div.zeile.eng", { stil: { marginTop: "8px" } }, s.besetzung.map(([r, v]) => h("img.ava", { src: portraet(r), alt: "", title: `${ROLLENTITEL[r]}: ${v}`, stil: { "--rolle": ROLLENFARBE[r], width: "26px", height: "26px" } }))))))),
    h("div.raster.r2", { stil: { margin: "16px 0" } },
      karte("Doktrin", "Deine Grundsätze für das ganze Reich – wie eine CLAUDE.md. Sie steht vertrauenswürdig in jedem Systemprompt, vor Skills und Spielregeln.",
        textfeld("/doktrin", a.doktrin, true, "z. B. „Wir sind friedliche Händler …“", { rows: 7 }),
        h("div.zeile.eng", { stil: { marginTop: "8px" } }, DOKTRIN_BAUSTEINE.filter((b) => !(a.doktrin || "").includes(b)).slice(0, 6).map((b) =>
          h("span.chip", { stil: { cursor: "pointer" }, title: "Zur Doktrin hinzufügen", onclick: () => setze("/doktrin", ((a.doktrin || "").trim() + " " + b).trim()) }, ikon("plus"), b)))),
      karte("Autonomie des Rats", "Statt 40 Einzelrechten ein Regler je Bereich und Rolle. Feinheiten im Rat unter „Wachposten“.",
        h("div.rahmen", { stil: { maxHeight: "none", border: "0" } }, h("table.tab", null,
          h("tr", null, h("th", null, "Bereich"), ROLLEN.map((r) => h("th", null, h("span.zeile.eng", null, h("img.ava", { src: portraet(r), alt: "", stil: { "--rolle": ROLLENFARBE[r], width: "22px", height: "22px" } }), ROLLENTITEL[r])))),
          kat.gruppen.map((g) => h("tr", null, h("td", null, h("b", null, g.name), h("div.winzig.matt", null, g.text)),
            ROLLEN.map((r) => { const s = au[r]?.[g.name];
              if (!s || s === "–") return h("td.matt", null, "–");
              return h("td", null, h("select.feld", { stil: { padding: "4px 8px", fontSize: "12.5px" }, onchange: (e) => sende("autonomie", { rolle: r, gruppe: g.name, stufe: e.target.value }) },
                (s === "gemischt" ? [["gemischt", "gemischt"]] : []).concat(stufen).map(([v, t]) => h("option", { value: v, selected: v === s }, t)))); }))))),
        h("div.zeile", { stil: { marginTop: "10px" } }, h("span.klein.matt", null, "Alle Rollen auf einmal:"), kat.gruppen.slice(0, 1).map(() => null),
          h("select.feld", { stil: { maxWidth: "170px" }, onchange: (e) => { S.ui.alleGruppe = e.target.value; } }, kat.gruppen.map((g) => h("option", { value: g.name, selected: (S.ui.alleGruppe || "Wirtschaft") === g.name }, g.name))),
          segment(stufen, null, (v) => sende("autonomie", { gruppe: S.ui.alleGruppe || "Wirtschaft", stufe: v }), true)))),
    karte("Daueraufträge", "Routinen ohne Modell – wie Cron-Jobs eines Agenten-Harness. Sie kosten keine Tokens; Bauaufträge laufen wie jeder Zug durch Wachposten, Probe und Server.",
      h("div.raster.r3", null, a.routinen.map((rt, i) => routineKarte(rt, i)),
        h("div.kachel", { stil: { display: "flex", flexDirection: "column", gap: "8px", justifyContent: "center", alignItems: "flex-start" } },
          h("b", null, "Neuer Dauerauftrag"), h("div.zeile.eng", null, kat.routinen.map((r) => knopf(r.titel, () => sende("routine_neu", { art: r.name }), "klein", "plus")))))),
    naechste?.bedingungen ? h("section.karte", { stil: { marginTop: "16px" } }, h("h2", null, "Ziel: " + naechste.name),
      h("div.zeile", null, h("img", { src: bild(`progression.${STUFEN_BILD[Math.min(4, stufenNummer(S.zustand.lage?.stufe))]}.${volk()}`), alt: "", stil: { width: "120px", borderRadius: "12px" } }),
        h("div.spalte", { stil: { flex: "1" } }, naechste.bedingungen.map((b) => h("div.zeile", null, h("span" + (b.erfuellt ? ".gruen" : ".matt"), null, b.erfuellt ? "✔" : "○"), b.text)),
          laeuft() ? h("div", null, knopf("Als Ziel an den Strategen", () => harness({ befehl: "aufgabe_neu", titel: `Stufenaufstieg: ${naechste.name}`, beschreibung: naechste.bedingungen.filter((b) => !b.erfuellt).map((b) => b.text).join("; "), rolle: "stratege", prioritaet: 1 }), "gold klein", "stern")) : null))) : null);
}

function routineKarte(rt, i) {
  const kat = S.katalog, st = S.zustand.routinen?.[rt.name], info = kat.routinen.find((x) => x.name === rt.art) || {};
  const pfad = `/routinen/${i}`;
  const symbol = { inventur: "stats.verwaltung", analyse: "stats.punkte", auto_ausbau: "buildings.bauhof" }[rt.art] || "stats.punkte";
  return h("div.kachel", { stil: { cursor: "default" } },
    h("div.zeile", null, h("img", { src: bild(symbol), alt: "", stil: { width: "40px", height: "40px", borderRadius: "10px" } }), h("div", null, h("b", null, info.titel || rt.art), h("div.winzig.matt", null, rt.name)),
      h("span.rechts"), schalter(rt.aktiv, "", (v) => setze(pfad + "/aktiv", v))),
    h("p", null, info.text),
    h("div.formular", { stil: { gridTemplateColumns: "90px 1fr", marginTop: "8px" } },
      h("label", null, "alle"), regler(rt.alle_minuten, 5, 720, 5, (x) => `${x} min`, (x) => setze(pfad + "/alle_minuten", x)),
      h("label", null, "meldet an"), h("select.feld", { onchange: (e) => setze(pfad + "/melden_an", e.target.value) }, [["", "niemanden"]].concat(ROLLEN.map((r) => [r, ROLLENTITEL[r]])).map(([v, t]) => h("option", { value: v, selected: rt.melden_an === v }, t)))),
    rt.art === "auto_ausbau" ? h("div", { stil: { marginTop: "8px" } }, h("div.winzig.matt", null, "Reihenfolge (nach Strom, Nahrung, Lager, Arbeitskräften): niedrigste Stufe zuerst"),
      h("div.zeile.eng", { stil: { marginTop: "6px" } }, (rt.reihenfolge.length ? rt.reihenfolge : []).map((g, j) => h("span.chip.gold", { title: "Klick: entfernen · Doppelklick: nach vorn" , stil: { cursor: "pointer" },
        onclick: () => setze(pfad + "/reihenfolge", rt.reihenfolge.filter((_, x) => x !== j)), ondblclick: () => { if (j > 0) { const n = rt.reihenfolge.slice(); [n[j - 1], n[j]] = [n[j], n[j - 1]]; setze(pfad + "/reihenfolge", n); } } },
        h("img", { src: bild(`buildings.${g}`), alt: "" }), `${j + 1}. ${name(g)}`)),
        rt.reihenfolge.length ? null : h("span.winzig.matt", null, "Grundversorgung")),
      h("div.zeile.eng", { stil: { marginTop: "6px" } }, kat.ausbau_gebaeude.filter((g) => !rt.reihenfolge.includes(g)).map((g) => h("span.chip", { title: "Hinzufügen", stil: { cursor: "pointer" }, onclick: () => setze(pfad + "/reihenfolge", rt.reihenfolge.concat(g)) },
        h("img", { src: bild(`buildings.${g}`), alt: "" }), name(g)))),
      h("div.formular", { stil: { gridTemplateColumns: "90px 1fr", marginTop: "8px" } }, h("label", null, "Aufträge"), regler(rt.max_auftraege, 1, 10, 1, (x) => `${x} je Lauf`, (x) => setze(pfad + "/max_auftraege", x)))) : null,
    h("div.trenner"),
    h("div.zeile", null, st?.laeuft ? h("span.lade") : null,
      h("span.klein" + (st && st.ok === false ? ".rot" : ""), null, st?.kurz ? `${st.ok ? "✔" : "✖"} ${st.kurz}` : (laeuft() ? "noch nicht gelaufen" : "läuft, sobald der Rat tagt")),
      h("span.rechts"), laeuft() ? knopf("Jetzt", () => harness({ befehl: "routine", name: rt.name }), "klein", "los") : null,
      knopf("", () => setze("/routinen", S.auswahl.routinen.filter((_, x) => x !== i)), "leise klein", "kreuz")));
}
