/* Spiel im Cockpit: das Spiel selbst (abgeschotteter Browser, nur der Spielserver) und daneben der Rat. */
"use strict";

function seiteSpiel() {
  const w = S.zustand.wartend || [];
  return h("div.seite.voll", null, h("div.spielrahmen", null,
    h("div.platz", { id: "spielplatz" }, h("div.leer", { stil: { paddingTop: "30vh" } }, "Das Spiel lädt … (nur die Seite deines Spielservers ist erreichbar)")),
    h("aside.seitenleiste", null,
      h("div.zeile", { stil: { marginBottom: "10px" } }, h("b.gold", { stil: { letterSpacing: ".18em", fontSize: "12px" } }, "DER RAT"), h("span.rechts"), knopf("Im Browser", () => sende("browser_extern"), "leise klein", "extern")),
      h("div.liste", null, ROLLEN.map((r) => { const a = rolleA(r), aktiv = rolleW(r).aktiv, denkt = a.status === "denkt";
        return h("div.eintrag.wahl", { stil: { "--rolle": ROLLENFARBE[r] }, onclick: () => nav("rat", { rolle: r }) }, avatar(r, 36), h("div.text", null, h("b", null, ROLLENTITEL[r]),
          h("span.winzig.matt", null, !aktiv ? "nicht im Rat" : denkt ? "denkt …" : a.naechster ? "nächster Zug " + in_(a.naechster) : (laeuft() ? "wartet" : "ruht")),
          a.begruendung ? h("div.winzig", { stil: { fontStyle: "italic", marginTop: "3px" } }, `„${a.begruendung.slice(0, 140)}“`) : null), h("span.licht" + (denkt ? ".denkt" : laeuft() && aktiv ? ".an" : ""))); })),
      w.length ? h("div", { stil: { marginTop: "14px" } }, h("b.bernstein.klein", null, "Bittet um Erlaubnis"), h("div.liste", { stil: { marginTop: "6px" } }, w.map((x) => h("div.eintrag.warnung", null,
        h("div.text", null, h("b.klein", null, aktionText(x.aktion)), h("div.zeile.eng", { stil: { marginTop: "6px" } }, knopf("Ja", () => sende("entscheiden", { id: x.id, ja: true }), "gold klein"), knopf("Nein", () => sende("entscheiden", { id: x.id, ja: false }), "leise klein"))))))) : null,
      h("div", { stil: { marginTop: "14px" } }, h("b.gold.klein", null, "Ereignisse"), h("div", { stil: { marginTop: "6px" } }, ereignisFeed(10))))));
}
