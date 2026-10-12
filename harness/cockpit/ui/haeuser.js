/* Vorstellung der vier Häuser im Einrichtungsassistenten.
   Herkunft: assets/stimmen/rat.json; Spielprofil: docs/HANDBUCH.md, Kapitel Völker.
   Die Hauswahl besetzt die ausgewählte Ratsrolle, sie gründet kein neues Reich. */
"use strict";

const HAEUSER = {
  veyari: {
    farbe: "#8fdca5", welt: "Veyhara", titel: "Ein Reich, das Wurzeln schlägt",
    kurz: "Wachstum & Kolonien", herkunft: "Die Waldwelt Veyhara",
    text: "Auf Veyhara wachsen Städte, statt gebaut zu werden. Die Veyari denken in Jahreszeiten, Wurzeln und einem Geflecht, das Generationen verbindet. Ihr Weg zu den Sternen beginnt mit vollen Speichern und neuen Gärten auf fernen Welten.",
    staerken: "Schnelles Bevölkerungswachstum, mehr Nahrung und günstige Kolonieschiffe.",
    schwaechen: "Ihre Schiffe haben eine schwächere Struktur.",
  },
  aurelianer: {
    farbe: "#f3d9a6", welt: "Aurelion", titel: "Die Sterne sind eine Handelsroute",
    kurz: "Handel & Wohlstand", herkunft: "Die Handelswelt Aurelion",
    text: "Elfenbein und Messing prägen die Städte Aurelions an den großen Handelsrouten. Für die Aurelianer sind Verträge, Ansehen und ein verlässlicher Handel das Fundament eines Reichs. Wo andere Grenzen sehen, sehen sie den nächsten Handelspartner.",
    staerken: "Mehr Frachtraum und Steuereinnahmen, halbierte Marktgebühren.",
    schwaechen: "Ihre Waffen sind weniger durchschlagskräftig.",
  },
  krath: {
    farbe: "#ff9b7e", welt: "Krath Ur", titel: "Ehre wird nicht verhandelt",
    kurz: "Flotten & Eroberung", herkunft: "Die Basaltwelt Krath Ur",
    text: "Schwarzer Basalt und glühende Spalten formen Krath Ur. Die Krath leben für ihren Clan, ihre Ehre und ihr gegebenes Wort. Ihre Werften arbeiten schnell, ihre Flotten schlagen hart zu – und ein Bündnis zählt so viel wie der Wille, dafür einzustehen.",
    staerken: "Stärkere Waffen, schnelle Werften und eine hohe Plünderquote.",
    schwaechen: "Ihre Forschung kommt langsamer voran.",
  },
  syntheten: {
    farbe: "#9bcaff", welt: "Hexaris", titel: "Wissen ist die nächste Evolutionsstufe",
    kurz: "Forschung & Energie", herkunft: "Die Maschinenwelt Hexaris",
    text: "Hexaris ist eine selbst gebaute Welt aus Sechseck-Modulen. Die Syntheten suchen Wissen, klare Zusammenhänge und effiziente Lösungen. Ihre Zivilisation benötigt keine Nahrung; Energie hält ihre Bevölkerung und ihr Streben nach Erkenntnis am Leben.",
    staerken: "Mehr Forschungsleistung und Energie, kein Nahrungsbedarf der Bevölkerung.",
    schwaechen: "Langsames Bevölkerungswachstum und zusätzlicher Strombedarf je Einwohner.",
  },
};

function haeuserVorstellung(rolle) {
  const v = gestalt(rolle), haus = HAEUSER[v] || HAEUSER.veyari;
  const waehlen = (volk) => { Rund.halten(); Hausbuehne.zurueck(); setze(`/rollen/${rolle}/gestalt`, volk); };
  return h("section.haeuser", { "aria-label": "Vorstellung der vier Häuser", stil: { "--haus": haus.farbe } },
    h("div.haeuser-kopf", null,
      h("h3", null, "Vier Häuser. Vier Wege zu den Sternen."),
      h("p.matt", null, `Lerne die Häuser kennen und wähle die Herkunft für die Rolle „${ROLLENTITEL[rolle]}“.`)),
    h("div.haus-auswahl", { "aria-label": "Haus des Ratsmitglieds wählen" }, VOELKER.map((volk) => {
      const d = HAEUSER[volk];
      return h("button.haus-karte" + (v === volk ? ".an" : ""), {
        type: "button", "data-haus": volk, "aria-pressed": String(v === volk), onclick: () => waehlen(volk),
        stil: { "--haus": d.farbe, backgroundImage: `linear-gradient(180deg, rgba(5, 10, 18, .55), #08111d), url(${bild(`surfaces.leben.${volk}`)})` },
      }, h("img.haus-emblem", { src: bild(`emblems.${volk}.${volk}`), alt: "", width: "38", height: "38" }),
      h("b", null, name(volk)), h("span", null, d.kurz), h("small", null, d.welt));
    })),
    Hausbuehne.element(v),
    h("div.haus-rat", null, ROLLEN.map((r) => {
      const f = window.Stimmen?.figur(r, v), s = window.Stimmen?.spricht();
      const spricht = s?.rolle === r && s.volk === v;
      return h("article.haus-ratskarte" + (spricht ? ".spricht" : ""), { stil: { "--rolle": ROLLENFARBE[r] } },
        h("button.haus-ratskopf", { type: "button", "aria-label": `${f?.name || ROLLENTITEL[r]} stellt sich vor`, onclick: () => Hausbuehne.vorstellen(r, v), disabled: !f?.vorstellung },
          h("img.ava", { src: bild(`role_portraits.${r}.${v}`), alt: "", width: "40", height: "40" }),
          h("span", null, h("small", null, ROLLENTITEL[r]), h("b", null, f?.name || ROLLENTITEL[r])), h("i.haus-play", null, spricht ? "■" : "▶")),
        h("div.haus-zurufe", null, [["klick", "Angewählt"], ["angriff", "Angriff!"], ["erfolg", "Erfolg"]].map(([anlass, titel]) =>
          h("button", { type: "button", disabled: !f || !window.Stimmen?.einstellungen.an, onclick: () => { window.Stimmen?.stopp(); window.Stimmen?.zuruf(anlass, r, { volk: v, abkling: 0 }); } }, titel))));
    })),
    h("div.haus-vorstellung", null,
      h("div.haus-geschichte", { "aria-live": "polite", "aria-atomic": "true" },
        h("span.haus-herkunft", null, haus.herkunft), h("h3", null, haus.titel), h("p", null, haus.text),
        h("dl.haus-profil", null,
          h("div", null, h("dt.gruen", null, "Stärken"), h("dd", null, haus.staerken)),
          h("div", null, h("dt.bernstein", null, "Schwächen"), h("dd", null, haus.schwaechen))))),
    h("div.haus-fuss", null, h("span.matt", null, "Jede Rolle darf aus einem anderen Haus stammen. Das Volk deines Reichs wählst du bei der Anmeldung."),
      knopf(`Ganzer Rat: ${name(v)}`, () => { Rund.halten(); Hausbuehne.zurueck(); ROLLEN.forEach((r) => setze(`/rollen/${r}/gestalt`, v)); }, "leise klein")));
}
