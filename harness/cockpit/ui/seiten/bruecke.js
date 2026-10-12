/* Brücke: alles Wichtige auf einen Blick, und alles anklickbar. */
"use strict";

const STUFEN_BILD = ["gruendung", "industrie", "orbit", "sternenflug", "imperium"];
function stufenNummer(s) { const r = { I: 1, II: 2, III: 3, IV: 4, V: 5 }; return typeof s === "number" ? s : (r[String(s).split(" ")[0]] || parseInt(s) || 1); }

/** Eine Spielaktion in Worten statt JSON. */
function aktionText(a) {
  if (!a || typeof a !== "object") return String(a);
  switch (a.typ) {
    case "bauen": return `${name(a.gebaeude)} auf ${a.planet} ausbauen`;
    case "forschen": return `${name(a.forschung)} erforschen`;
    case "fertigen": return `${a.anzahl || 1} × ${name(a.einheit || a.produkt)} fertigen`;
    case "schenken": return `${zahl(a.credits)} Credits an ${a.an} verschenken`;
    case "flotte_senden": return `Flotte zur ${name(a.mission)} nach ${a.ziel}`;
    case "brief_senden": return `Brief an ${a.an}: „${a.betreff || ""}“`;
    case "vertrag_anbieten": return `${name(a.art || a.vertrag)} an ${a.an} anbieten`;
    case "markt_order": return `Marktorder: ${a.seite || ""} ${zahl(a.menge)} ${name(a.gut)}`;
    case "abreissen": return `${name(a.gebaeude)} auf ${a.planet} abreißen`;
    default: return name(a.typ) + " " + Object.entries(a).filter(([k]) => k !== "typ").map(([k, v]) => `${k}=${typeof v === "object" ? JSON.stringify(v) : v}`).join(", ");
  }
}
function aktionBild(a) {
  const t = a?.typ;
  if (t === "bauen" || t === "abreissen") return bild(`buildings.${a.gebaeude}`);
  if (t === "forschen") return bild(`research.${a.forschung}`);
  if (t === "schenken") return gutBild("credits");
  if (t === "flotte_senden") return bild(`missions.${a.mission === "pluenderung" ? "angriff" : a.mission}`);
  if (t && t.startsWith("vertrag")) return bild("contracts.handelsabkommen");
  if (t === "brief_senden") return bild("diplomacy.botschaft");
  return bild("events.vertrag_angebot");
}

function agentKarte(r) {
  const w = rolleW(r), a = rolleA(r), aktiv = w.aktiv, denkt = a.status === "denkt" || a.status === "korrigiert";
  const status = !laeuft() || !aktiv ? (aktiv ? "bereit, Rat ruht" : "nicht im Rat") : denkt ? "denkt nach …" : a.status === "Fehler" ? "Fehler" : a.status === "angehalten" ? "angehalten" : a.naechster ? `nächster Zug ${in_(a.naechster)}` : "erster Zug folgt";
  return h("div.agent" + (aktiv ? "" : ".aus") + (denkt ? ".denkt" : ""), { stil: { "--rolle": ROLLENFARBE[r] }, onclick: () => nav("rat", { rolle: r }), title: "Im Rat einstellen" },
    h("div.bild", { stil: { backgroundImage: `url(${portraet(r)})` } }), h("div.randlicht"),
    h("div.oben", null, h("span.chip", null, h("span.licht" + (denkt ? ".denkt" : laeuft() && aktiv ? ".an" : a.status === "Fehler" ? ".rot" : "")), status),
      w.modell ? h("span.chip.cyan", { title: "Modell" }, w.modell) : null),
    h("div.unten", null,
      h("div.name", null, h("small", null, ROLLENTITEL[r]), w.vorlage || "eigene Seele"),
      h("div.zitat", null, a.begruendung ? `„${a.begruendung}“` : (aktiv ? "Noch keine Entscheidung in dieser Sitzung." : "Diese Rolle sitzt nicht im Rat – im Rat aktivieren.")),
      h("div.werte", null, h("span", null, h("b", null, zahl(a.aufrufe || 0)), " Züge"), h("span", null, h("b.gruen", null, zahl(a.angenommen || 0)), " ✔"),
        h("span", null, h("b.rot", null, zahl(a.abgelehnt || 0)), " ✖"), a.kosten ? h("span", null, h("b", null, a.kosten.toFixed(3)), " $") : null,
        laeuft() && aktiv ? h("span.rechts", null, knopf("jetzt", (e) => { e.stopPropagation(); harness({ befehl: "jetzt", rolle: r }); }, "klein")) : null)));
}

function erlaubnisse() {
  const w = S.zustand.wartend || [];
  if (!w.length) return null;
  return h("section.karte.wichtig", null, h("h2", null, "Der Rat bittet um Erlaubnis", h("span.rechts.matt", null, `${w.length} offen`)),
    h("p.unter", null, "Diese Züge hat der Wachposten nach deinen Autonomie-Regeln zurückgehalten. Nach der Freigabe prüft der Spielkern sie erneut."),
    h("div.liste", null, w.map((x) => h("div.eintrag", null, h("img.bildchen", { src: aktionBild(x.aktion), alt: "" }),
      h("div.text", null, h("b", null, aktionText(x.aktion)), h("span.matt.klein", null, `${ROLLENTITEL[x.rolle] || x.rolle} · ${x.grund} · ${vor(x.zeit)}`)),
      knopf("Ablehnen", () => sende("entscheiden", { id: x.id, ja: false }), "leise klein", "kreuz"), knopf("Freigeben", () => sende("entscheiden", { id: x.id, ja: true }), "gold klein", "haken")))));
}

function reichKarte() {
  const l = S.zustand.lage || {}, k = S.daten?.kennzahlen || [];
  const stufe = stufenNummer(l.stufe);
  const punkte = (Array.isArray(k) ? k : []).map((x) => x.punkte).filter((x) => x !== undefined && x !== null);
  const naechste = l.naechste_stufe;
  return h("section.karte", { stil: { padding: "0", overflow: "hidden" } },
    h("div", { stil: { display: "grid", gridTemplateColumns: "190px 1fr", minHeight: "190px" } },
      h("div", { stil: { backgroundImage: `url(${bild(`progression.${STUFEN_BILD[Math.max(0, Math.min(4, stufe - 1))]}.${volk()}`)})`, backgroundSize: "cover", backgroundPosition: "center" } }),
      h("div", { stil: { padding: "16px 18px" } },
        h("h2", null, "Reich"),
        l.name ? h("div", null, h("div", { stil: { font: "600 22px/1.2 var(--titel)" } }, l.name), h("div.matt", null, `${name(l.volk)} · Zivilisationsstufe ${l.stufe} · Rang ${l.rang ?? "–"}`)) : h("p.matt", null, laeuft() ? "Die erste Lage kommt gleich …" : "Mit ▶ LOS tritt der Rat zusammen und meldet die Lage deines Reichs."),
        h("div.zeile", { stil: { marginTop: "10px" } }, chip(`${zahl(l.punkte)} Punkte`, "gold"), chip(`${zahl(l.credits)} Credits`, null, gutBild("credits")), chip(`${l.planeten?.length ?? 0} Planeten`), chip(`${l.flotten ?? 0} Flotten`),
          l.neue_nachrichten ? chip(`${l.neue_nachrichten} neue Briefe`, "bernstein") : null),
        punkte.length > 1 ? h("div", { stil: { marginTop: "6px" } }, sparkline(punkte, "#dab57b", 54), h("div.winzig.matt", null, "Punkte seit Beginn der Aufzeichnung (Routine Inventur)")) : null,
        naechste?.bedingungen ? h("div", { stil: { marginTop: "8px" } }, h("div.klein.gold", null, `Nächste Stufe: ${naechste.name}`),
          h("div.zeile.eng", null, naechste.bedingungen.map((b) => chip(b.text, b.erfuellt ? "gruen" : null)))) : null,
        l.name ? null : planKurz())));
}

/** Vor der ersten Lage: womit der Rat antreten wird – Spielstil, Doktrin, Besetzung. */
function planKurz() {
  const a = S.auswahl, stil = (S.katalog.spielstile || []).find((s) => s.name === a.spielstil);
  const doktrin = (a.doktrin || "").trim();
  return h("div.spalte", { stil: { marginTop: "14px", gap: "10px" } },
    h("div.eintrag.wahl", { onclick: () => nav("strategie") }, h("div.text", null, h("span.winzig.matt", null, "Spielstil"), h("b", null, stil ? stil.titel : "noch keiner gewählt"),
      h("span.winzig.matt", null, stil ? stil.kurz : "Ein Spielstil besetzt den ganzen Rat stimmig – Vorlagen, Doktrin, Autonomie.")), ikon("pfeil")),
    h("div.eintrag.wahl", { onclick: () => nav("strategie") }, h("div.text", null, h("span.winzig.matt", null, "Doktrin"),
      h("div.klein", { stil: { fontStyle: doktrin ? "italic" : "normal" } }, doktrin ? `„${doktrin.slice(0, 220)}${doktrin.length > 220 ? " …" : ""}“` : "Noch keine Grundsätze – sie stehen in jedem Systemprompt.")), ikon("pfeil")),
    h("div.zeile", null, ROLLEN.map((r) => { const w = rolleW(r);
      return h("div.eintrag.wahl", { stil: { "--rolle": ROLLENFARBE[r], opacity: w.aktiv ? "1" : ".45", padding: "6px 10px 6px 6px", flex: "1" }, onclick: () => nav("rat", { rolle: r }) },
        avatar(r, 30), h("div.text", null, h("b.klein", null, ROLLENTITEL[r]), h("span.winzig.matt", null, w.aktiv ? `${w.vorlage || "eigene Seele"} · ${w.modell || "kein Modell"}` : "nicht im Rat"))); })));
}

function planetenKarte() {
  const inv = S.daten?.inventur?.planeten || [];
  const lageP = S.zustand.lage?.planeten || [];
  const liste = inv.length ? inv : lageP;
  if (!liste.length) return karte("Planeten", "Die Routine „Inventur“ erfasst alle Planeten.", leer(laeuft() ? "Erste Inventur folgt …" : "Noch keine Daten – ▶ LOS."));
  return karte("Planeten", inv.length ? `Inventur ${vor(S.daten.inventur?.zeit)} · Klick öffnet den Codex zur Kolonie` : "aus der letzten Lage",
    h("div.spalte", null, liste.slice(0, 6).map((p) => {
      const saldo = p.energie ? (p.energie.saldo ?? (p.energie.erzeugung - p.energie.verbrauch)) : 0;
      const schleife = p.bauschleife?.belegt ?? (Array.isArray(p.bauschleife) ? p.bauschleife.length : 0);
      const plaetze = p.bauschleife?.plaetze ?? 5;
      return h("div.planetkarte", null, h("div.kugel", { stil: { backgroundImage: `url(${planetBild(p.zone)})` } }),
        h("div.daten", null,
          h("div.zeile", null, h("b.mono", null, p.koord), p.heimat ? chip("Heimat", "gold") : null, chip(`${name(p.zone)}`), h("span.rechts.klein" + (saldo < 0 ? ".rot" : ".gruen"), null, `Strom ${saldo >= 0 ? "+" : ""}${zahl(saldo)}`)),
          ["erz", "kristall", "deuterium", "nahrung"].map((g) => {
            const b = p.bestand?.[g] ?? 0, lg = p.lager?.[g] || 1, r = p.rate?.[g];
            return h("div.ressource", null, h("img", { src: gutBild(g), alt: "" }), h("span.zahl", null, zahl(b)), balken(b / lg, b / lg > 0.9 ? "bernstein" : null), h("span.winzig.matt.zahl", null, r !== undefined ? (r >= 0 ? "+" : "") + zahl(r) + "/h" : ""));
          }),
          h("div.zeile.winzig.matt", null, `Bauschleife ${schleife}/${plaetze}`, p.bauschleife?.aktuell ? `· ${name(p.bauschleife.aktuell.gebaeude)} → ${p.bauschleife.aktuell.stufe} (${p.bauschleife.aktuell.rest_min} min)` : (schleife === 0 ? "· leer" : ""),
            p.bevoelkerung !== undefined ? `· ${zahl(p.bevoelkerung)} Einwohner` : "")));
    })));
}

const BEFUND_BILD = { Energie: "events.energie_mangel", Nahrung: "events.hunger", Lager: "events.lager_voll", Angriff: "events.angriff_warnung", Stabilität: "events.unruhen", Vorrat: "events.hunger",
  Bauschleife: "events.bau_fertig", Forschung: "events.forschung_fertig", Stufenaufstieg: "events.stufe_erreicht", Konsumgüter: "events.unruhen", Blockade: "events.belagerung", Felder: "stats.baufelder", Arbeitskräfte: "stats.arbeitskraefte", Fachkräfte: "stats.fachkraefte", Trend: "stats.punkte" };
function befundZeile(b, kurz) {
  return h("div.befund.s" + b.stufe, null, h("img", { src: bild(BEFUND_BILD[b.thema] || "events.vertrag_angebot"), alt: "" }),
    h("div.text", null, h("div.zeile.eng", null, h("b", null, b.thema), b.planet ? h("span.mono.matt", null, b.planet) : null, h("span.punkt", { stil: { "--rolle": ROLLENFARBE[b.rolle] } }), h("span.winzig.matt", null, ROLLENTITEL[b.rolle] || "")),
      h("div.klein", null, b.text), kurz ? null : h("div.winzig.matt", null, "Vorschlag: " + b.vorschlag)),
    laeuft() && !kurz ? h("div.spalte", { stil: { gap: "4px" } },
      knopf("Auftrag", () => harness({ befehl: "aufgabe_neu", titel: `${b.thema}${b.planet ? " " + b.planet : ""}: ${b.vorschlag}`.slice(0, 120), beschreibung: b.text, rolle: b.rolle, prioritaet: b.stufe === 2 ? 1 : 2 }), "klein", "plus"),
      knopf("Hinweis", () => harness({ befehl: "hinweis", rolle: b.rolle, text: `${b.text} – ${b.vorschlag}` }), "leise klein")) : null);
}
function befundeKarte() {
  const a = S.daten?.analyse;
  const liste = a?.befunde || [];
  return karte("Befunde der Analyse", a?.kurz ? `${a.kurz} · Klick auf „Auftrag“ gibt den Befund an die zuständige Rolle` : "Die Routine „Analyse“ prüft alle Planeten auf Engpässe.",
    liste.length ? h("div.liste", null, liste.slice(0, 6).map((b) => befundZeile(b))) : leer(laeuft() ? "Keine Befunde – alles im grünen Bereich." : "Noch keine Analyse."),
    laeuft() ? h("div.zeile", { stil: { marginTop: "10px" } }, knopf("Jetzt analysieren", () => harness({ befehl: "routine", name: "analyse" }), "klein"), knopf("Inventur", () => harness({ befehl: "routine", name: "inventur" }), "leise klein")) : null);
}

const ART_FARBE = { zug: "gruen", fehler: "rot", abbruch: "rot", warnung: "bernstein", bestaetigung_noetig: "bernstein", routine: "cyan", analyse: "cyan", inventur: "cyan", denkt: "cyan", wissen_antwort: "gold" };
function ereignisFeed(anzahl, filter) {
  const e = S.ereignisse.filter((x) => !filter || (x.art + " " + x.rolle + " " + x.text).toLowerCase().includes(filter.toLowerCase())).slice(0, anzahl || 14);
  if (!e.length) return leer(laeuft() ? "Warte auf Ereignisse …" : "Mit ▶ LOS strömen hier alle Ereignisse des Rats herein.");
  return h("div.feed", null, e.map((x) => h("div.z", null, h("span.matt.zahl", null, vor(x.zeit).replace("vor ", "")), h("span." + (ART_FARBE[x.art] || "matt"), null, name(x.art)),
    h("span", { stil: { color: ROLLENFARBE[x.rolle] || "var(--matt)" } }, ROLLENTITEL[x.rolle] || x.rolle || ""), h("span", null, x.text))));
}

/** Vor dem ersten Start: was noch fehlt, mit dem Knopf, der es löst. */
function startklarKarte() {
  const z = S.zustand, a = S.auswahl, aktiv = ROLLEN.filter((r) => rolleW(r).aktiv);
  const ollama = (z.ollama || []).map((m) => m.name);
  const fehlt = aktiv.filter((r) => { const w = rolleW(r); return w.anbieter === "Ollama" && w.modell && !ollama.some((m) => m === w.modell || m === w.modell + ":latest"); });
  const punkte = [
    [!!z.server?.info, "Spielserver erreichbar", z.server?.info ? a.server : (z.server?.fehler || "noch nicht geprüft"), () => sende("server", { wert: a.server }), "Prüfen"],
    [aktiv.length > 0, `${aktiv.length} von 4 Rollen im Rat`, aktiv.map((r) => ROLLENTITEL[r]).join(", ") || "keine Rolle aktiv", () => nav("rat"), "Rat"],
    [!fehlt.length, "Modelle vorhanden", fehlt.length ? "fehlt: " + fehlt.map((r) => rolleW(r).modell).join(", ") : "alle Rollenmodelle bereit", () => nav("rat", { rolle: fehlt[0] || "verwalter", stufe: "modell" }), "Modelle"],
    [!!z.gebaut, "Harness gebaut", z.gebaut ? "aktuell" : "einmal bauen, dauert 1–3 Minuten", () => nav("werkstatt"), "Werkstatt"],
    [laeuft(), "Rat einberufen", "anmelden mit deinem Spielkonto, dann spielt der Rat", () => sende("los"), "▶ LOS"],
  ];
  const erster = punkte.findIndex(([ok]) => !ok);
  return karte("Startklar?", "Fünf Schritte bis zum ersten Zug deines Rats.",
    h("div.liste", null, punkte.map(([ok, titel, text, tun, knopfText], i) => h("div.eintrag" + (i === erster ? ".warnung" : ""), null,
      h("span." + (ok ? "gruen" : "matt"), { stil: { fontSize: "18px", width: "22px", textAlign: "center" } }, ok ? "✔" : String(i + 1)),
      h("div.text", null, h("b", null, titel), h("span.winzig.matt", null, text)),
      !ok && i === erster ? knopf(knopfText, tun, i === 4 ? "gold klein" : "klein", null, i === 4 && !z.gebaut) : null))));
}

function seiteBruecke() {
  const z = S.zustand;
  return h("div.seite", null,
    aenderungsleiste(),
    z.lokal?.connected ? lokalKapazitaet() : null,
    h("div.seitenkopf", null, h("div", null, h("h1", null, "Brücke"), h("p", null, "Dein Reich und dein Rat auf einen Blick. Jede Karte führt zur passenden Stelle – Agenten zum Rat, Befunde als Auftrag an die zuständige Rolle.")),
      h("div.rechts", null, chip(`${zahl(z.zaehler.zuege)} Züge · ${zahl(z.zaehler.angenommen)} ✔ · ${zahl(z.zaehler.abgelehnt)} ✖`), z.lokale_api ? chip("Lokale API aktiv", "cyan") : null)),
    erlaubnisse(),
    h("div.raster.r-haupt", { stil: { marginBottom: "16px" } }, reichKarte(),
      !laeuft() && !(z.alarme || []).length ? startklarKarte() : karte("Alarme", null, (z.alarme || []).length ? h("div.liste", null, z.alarme.slice(0, 5).map((a) => h("div.eintrag" + (a.stufe === 2 ? ".gefahr" : a.stufe === 1 ? ".warnung" : ""), null,
        h("div.text", null, h("div.klein", null, a.text), h("span.winzig.matt", null, vor(a.zeit)))))) : leer("Keine Alarme."))),
    h("h2.gold", { stil: { font: "600 12.5px var(--titel)", letterSpacing: ".18em", margin: "6px 0 10px" } }, "DER RAT"),
    h("div.rat", { stil: { marginBottom: "16px" } }, ROLLEN.map(agentKarte)),
    h("div.raster.r3", null, planetenKarte(), befundeKarte(), karte("Ereignisse", "Live aus dem Harness", ereignisFeed(16))));
}
