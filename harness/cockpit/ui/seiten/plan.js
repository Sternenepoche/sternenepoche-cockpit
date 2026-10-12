/* Plan und Gedächtnis: Aufgaben, Teambesprechung, Erinnerungen, Notizbücher. */
"use strict";

function avatar(rolle, groesse) {
  if (!ROLLEN.includes(rolle)) return h("span.ava", { stil: { width: (groesse || 30) + "px", height: (groesse || 30) + "px", display: "inline-flex", alignItems: "center", justifyContent: "center", background: "rgba(218,181,123,.2)", color: "var(--gold2)", fontSize: "12px" }, title: rolle === "besitzer" ? "Du" : name(rolle) }, rolle === "besitzer" ? "Du" : "R");
  return h("img.ava", { src: portraet(rolle), alt: "", title: ROLLENTITEL[rolle], stil: { "--rolle": ROLLENFARBE[rolle], width: (groesse || 30) + "px", height: (groesse || 30) + "px" } });
}

function aufgaben() {
  const liste = S.daten?.aufgaben || [];
  const f = S.ui.neueAufgabe = S.ui.neueAufgabe || { titel: "", text: "", rolle: "verwalter", prio: 2 };
  const spalten = [["offen", "Offen"], ["aktiv", "In Arbeit"], ["blockiert", "Blockiert"], ["erledigt", "Erledigt"]];
  if (!liste.length && !laeuft()) {
    return h("section.karte.leerbild", { stil: { "--bild": `url(${bild("backgrounds.research")})` } },
      h("div.spalte", { stil: { maxWidth: "640px" } }, h("h2", null, "Der Rat plant selbst"),
        h("p", null, "Sobald der Rat tagt, legen sich die Agenten hier Aufgaben an, geben sie an Kollegen weiter und haken sie ab – der Stratege plant die Kolonie, der Verwalter baut die Werft, der Feldherr wartet auf die Sonden. Du siehst jeden Schritt und kannst selbst Aufgaben vergeben."),
        h("div.zeile", null, ROLLEN.map((r) => avatar(r, 40)), h("span.matt.klein", null, "Planung ist eingeschaltet – Grenzen stellst du im Rat unter „Gedächtnis“ ein.")),
        h("div.zeile", { stil: { marginTop: "6px" } }, S.zustand.gebaut ? knopf("▶ LOS – Rat einberufen", () => sende("los"), "gold", null) : knopf("Erst bauen", () => nav("werkstatt"), "gold", "werkstatt"),
          knopf("Wie plant der Rat?", () => nav("rat", { stufe: "gedaechtnis" }), "leise", "rat"))));
  }
  return h("div.spalte", null,
    laeuft() ? h("section.karte", null, h("h2", null, "Neue Aufgabe"),
      h("div.zeile", null, h("input.feld", { placeholder: "z. B. „Kolonie in 2:16 vorbereiten“", value: f.titel, oninput: (e) => f.titel = e.target.value, stil: { flex: "2", minWidth: "260px" } }),
        h("div.zeile.eng", null, ROLLEN.map((r) => h("span", { stil: { cursor: "pointer", opacity: f.rolle === r ? "1" : ".45" }, onclick: () => { f.rolle = r; planen(); } }, avatar(r, 32)))),
        segment([[1, "P1"], [2, "P2"], [3, "P3"]], f.prio, (v) => { f.prio = v; planen(); }),
        knopf("Anlegen", () => { if (f.titel.trim()) { harness({ befehl: "aufgabe_neu", titel: f.titel.trim(), beschreibung: f.text.trim(), rolle: f.rolle, prioritaet: f.prio }); f.titel = ""; f.text = ""; planen(); } }, "gold klein", "plus")),
      h("input.feld", { placeholder: "Beschreibung (optional)", value: f.text, oninput: (e) => f.text = e.target.value, stil: { marginTop: "8px" } })) : null,
    h("div.raster.r4", null, spalten.map(([s, t]) => {
      const teil = liste.filter((a) => a.status === s);
      return h("div.spalte", null, h("div.zeile", null, h("b.gold", null, t), chip(String(teil.length))),
        teil.slice(0, s === "erledigt" ? 15 : 80).map((a) => h("section.karte", { stil: { padding: "12px 14px", borderLeft: `3px solid ${ROLLENFARBE[a.rolle] || "var(--linie)"}` } },
          h("div.zeile.eng", null, avatar(a.rolle, 26), h("b", null, `#${a.id} ${a.titel}`)),
          h("div.winzig.matt", { stil: { margin: "4px 0" } }, `P${a.prioritaet} · von ${a.von === "besitzer" ? "dir" : ROLLENTITEL[a.von] || a.von} · ${vor(a.aktualisiert)}`),
          a.beschreibung ? h("div.klein", null, a.beschreibung) : null, a.notiz ? h("div.klein.cyan", null, "Notiz: " + a.notiz) : null,
          laeuft() ? h("div.zeile.eng", { stil: { marginTop: "8px" } }, spalten.filter(([x]) => x !== s).map(([x, tt]) => knopf(tt, () => harness({ befehl: "aufgabe_aendern", id: a.id, status: x }), "leise klein")),
            knopf("", () => harness({ befehl: "aufgabe_loeschen", id: a.id }), "leise klein", "kreuz")) : null)),
        teil.length ? null : leer("–"));
    })));
}

function team() {
  const t = S.daten?.team || [];
  const f = S.ui.teamNachricht = S.ui.teamNachricht || { an: "stratege", text: "" };
  return h("div.raster.r-haupt", null,
    karte("Teambesprechung", "Nachrichten zwischen den Rollen – Bedarf, Übergaben, Warnungen. Routinen melden hier ihre Ergebnisse.",
      t.length ? h("div.liste", null, t.slice(0, 60).map((m) => h("div.eintrag", null, avatar(m.von, 34),
        h("div.text", null, h("div.zeile.eng", null, h("b", { stil: { color: ROLLENFARBE[m.von] || "var(--gold2)" } }, m.von === "besitzer" ? "Du" : (ROLLENTITEL[m.von] || name(m.von))), h("span.matt", null, "→"), avatar(m.an, 20), h("span.winzig.matt", null, vor(Number(m.zeit)))),
          h("div.klein", null, m.text))))) : leer(laeuft() ? "Noch keine Nachrichten." : "Erscheint, sobald der Rat tagt.")),
    laeuft() ? karte("Nachricht an das Team", "Wie eine Teamnachricht einer Rolle – nur von dir, vertrauenswürdig.",
      h("div.zeile.eng", null, ROLLEN.map((r) => h("span", { stil: { cursor: "pointer", opacity: f.an === r ? "1" : ".45" }, onclick: () => { f.an = r; planen(); } }, avatar(r, 36)))),
      h("textarea.feld", { value: f.text, placeholder: `Nachricht an ${ROLLENTITEL[f.an]}`, oninput: (e) => f.text = e.target.value, stil: { marginTop: "10px" } }),
      h("div.zeile", { stil: { marginTop: "8px" } }, h("span.rechts"), knopf("Senden", () => { if (f.text.trim()) { harness({ befehl: "team_nachricht", rolle: f.an, text: f.text.trim() }); f.text = ""; planen(); } }, "gold klein", "pfeil"))) : null);
}

function erinnerungen() {
  const e = S.daten?.erinnerungen || [];
  const f = S.ui.erinnerung = S.ui.erinnerung || { suche: "", thema: "", text: "" };
  const gefiltert = e.filter((x) => !f.suche || JSON.stringify(x).toLowerCase().includes(f.suche.toLowerCase()));
  return h("div.spalte", null,
    h("div.zeile", null, h("input.feld", { placeholder: "Erinnerungen durchsuchen …", value: f.suche, oninput: (ev) => { f.suche = ev.target.value; planen(); }, stil: { maxWidth: "380px" } }), chip(`${gefiltert.length} von ${e.length}`)),
    h("div.kacheln", { stil: { gridTemplateColumns: "repeat(auto-fill, minmax(280px, 1fr))" } },
      laeuft() ? h("div.kachel", { stil: { cursor: "default" } }, h("b", null, "Dem Team etwas einprägen"), h("input.feld", { placeholder: "Thema, z. B. Spielername", value: f.thema, oninput: (ev) => f.thema = ev.target.value, stil: { marginTop: "8px" } }),
        h("textarea.feld", { placeholder: "Was das Team wissen soll", value: f.text, oninput: (ev) => f.text = ev.target.value, stil: { marginTop: "6px", minHeight: "60px" } }),
        h("div.zeile", { stil: { marginTop: "6px" } }, h("span.rechts"), knopf("Merken", () => { if (f.thema.trim() && f.text.trim()) { harness({ befehl: "erinnerung_neu", thema: f.thema.trim(), text: f.text.trim() }); f.thema = ""; f.text = ""; planen(); } }, "gold klein", "plus"))) : null,
      gefiltert.slice(0, 120).map((x) => h("div.kachel", { stil: { cursor: "default" } }, h("div.zeile.eng", null, avatar(x.von, 24), h("b", null, x.thema), h("span.rechts"),
        laeuft() ? knopf("", () => harness({ befehl: "erinnerung_loeschen", id: x.id }), "leise klein", "kreuz") : null), h("p", null, x.text), h("div.winzig.matt", null, vor(Number(x.zeit)))))));
}

function notizbuecher() {
  const n = S.daten?.notizen || {};
  return h("div.raster.r4", null, ROLLEN.map((r) => h("section.karte", { stil: { borderTop: `3px solid ${ROLLENFARBE[r]}` } }, h("div.zeile", null, avatar(r, 34), h("b", null, ROLLENTITEL[r])),
    (n[r] || []).length ? h("div.liste", { stil: { marginTop: "10px" } }, n[r].map((x) => h("div.eintrag", null, h("div.text", null, h("div.klein", null, x.text), h("span.winzig.matt", null, vor(Number(x.zeit))))))) : leer("Noch keine Notizen."))));
}

function seitePlan() {
  const reiter = S.ui.plan.reiter, g = S.daten?.gedaechtnis || {};
  return h("div.seite", null,
    aenderungsleiste(),
    h("div.seitenkopf", null, h("div", null, h("h1", null, "Plan und Gedächtnis"), h("p", null, "Was der Rat vorhat, worüber er spricht und was er sich merkt – du kannst überall mitschreiben.")),
      h("div.rechts", null, chip(`${g.art === "duckdb" ? "DuckDB" : "SQLite"}`, "cyan"), chip(`${zahl(g.erinnerungen || 0)} Erinnerungen`), chip(`${zahl(g.zuege || 0)} Züge`), chip(`${zahl(g.inventur || 0)} Inventur-Zeilen`))),
    h("div", { stil: { marginBottom: "16px" } }, segment([["aufgaben", "Aufgaben"], ["team", "Teambesprechung"], ["erinnerungen", "Erinnerungen"], ["notizen", "Notizbücher"]], reiter, (v) => { S.ui.plan.reiter = v; planen(); }, true)),
    reiter === "team" ? team() : reiter === "erinnerungen" ? erinnerungen() : reiter === "notizen" ? notizbuecher() : aufgaben());
}
