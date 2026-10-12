/* Werkstatt: Bausteine, Bau, Modelle, Konto, Profile, Schnittstellen – und der Einrichtungsassistent. */
"use strict";

function baustein(titel, text, an, onclick, bildpfad) {
  return h("div.kachel" + (an ? ".an" : ""), { onclick }, an ? h("span.haken", null, ikon("haken")) : null,
    h("div.zeile", null, bildpfad ? h("img", { src: bildpfad, alt: "", stil: { width: "34px", height: "34px", borderRadius: "9px" } }) : null, h("b", null, titel)), h("p", null, text));
}

function bausteine() {
  const a = S.auswahl;
  return karte("Bausteine", "Was dein Harness nutzt – je Kategorie zwei Optionen. Nach einer Änderung einmal „Let's go“: Der beiliegende Harness kann alle Bausteine, selbst übersetzt wird nur auf Wunsch.",
    h("div.raster.r3", null,
      h("div.spalte", null, h("div.klein.gold", null, "Gedächtnis"), baustein("SQLite", "eingebaut, klein, robust", a.datenbank === "Sqlite", () => sende("baustein", { art: "datenbank", wert: "sqlite" })),
        baustein("DuckDB", "Analysen über die Inventur-Zeitreihe, offizieller Download mit Prüfsumme", a.datenbank === "DuckDb", () => sende("baustein", { art: "datenbank", wert: "duckdb" }))),
      h("div.spalte", null, h("div.klein.gold", null, "Eigene Skripte"), baustein("WASM-Sandbox", "Plugins prüfen jeden Zug – mit Treibstoff- und Speichergrenze", a.erweiterung === "WasmSandbox", () => sende("baustein", { art: "erweiterung", wert: "wasm" })),
        baustein("Ohne Plugins", "nur Modelle und Wachposten", a.erweiterung === "Ohne", () => sende("baustein", { art: "erweiterung", wert: "ohne" }))),
      h("div.spalte", null, h("div.klein.gold", null, "Schnittstelle"), baustein("Lokale API + MCP", "andere Programme und MCP-Anwendungen dürfen mitsteuern (127.0.0.1, Zufallsschlüssel)", a.schnittstelle === "LokaleApiMcp", () => sende("baustein", { art: "schnittstelle", wert: "lokal" })),
        baustein("Geschlossen", "nur das Cockpit steuert", a.schnittstelle === "Ohne", () => sende("baustein", { art: "schnittstelle", wert: "ohne" })))),
    h("div.zeile", { stil: { marginTop: "12px" } }, S.zustand.gebaut ? chip("gebaut mit: " + a.abgeleitet.features.join(", "), "gruen") : chip("noch nicht gebaut · wird gebaut mit: " + a.abgeleitet.features.join(", "), "bernstein")));
}

function bauKarte() {
  const z = S.zustand, b = z.bau;
  const symbol = { vorhanden: ["✔", "gruen"], laden: ["⬇", "cyan"], manuell: ["⚠", "bernstein"], fehlt: ["✖", "rot"] };
  return karte("Abhängigkeiten und Bau", "Erst prüfen, was gebraucht wird; mit „Let's go“ wird alles geladen (mit Prüfsumme), der Harness eingerichtet – vorgebaut oder selbst übersetzt – und lokal geprüft.",
    h("div.zeile", null, knopf("Abhängigkeiten prüfen", () => sende("abh_pruefen"), "", "suche", b.laeuft), z.abh_laedt ? h("span.lade") : null,
      knopf("Let's go", () => sende("lets_go"), "gold", "los", z.abh_blockiert || b.laeuft), schalter(z.modellprobe, "jedes Modell einmal kurz antworten lassen", (v) => sende("modellprobe", { wert: v })),
      z.vorgebaut ? schalter(!!S.auswahl.selbst_bauen, "selbst übersetzen (nur für eigene Änderungen am Harness-Code)", (v) => { setze("/selbst_bauen", v); sende("abh_pruefen"); }) : null,
      z.abh_blockiert && !z.abh_laedt && !b.laeuft ? h("span.bernstein.klein", null, (z.abh || []).length ? "Erst die rot markierten Punkte lösen – dann ist „Let's go“ frei." : "Erst „Abhängigkeiten prüfen“ – dann ist „Let's go“ frei.") : null),
    z.abh.length ? h("div.rahmen", { stil: { marginTop: "12px", maxHeight: "300px" } }, h("table.tab", null, z.abh.map((d) => h("tr", null, h("td." + symbol[d.stand][1], null, symbol[d.stand][0]), h("td", null, h("b", null, d.name)),
      h("td.matt.klein", null, d.detail), h("td.matt.klein", null, d.groesse), h("td", null, d.hilfe ? knopf("Anleitung", () => sende("regeln_lesen"), "leise klein") : null))))) : null,
    b.schritte.some((s) => s.stand !== "wartet") ? h("div.spalte", { stil: { marginTop: "14px" } }, b.schritte.map((s) => h("div.zeile", null,
      h("span." + ({ fertig: "gruen", fehler: "rot", laeuft: "cyan", uebersprungen: "matt" }[s.stand] || "matt"), { stil: { width: "18px" } }, { fertig: "✔", fehler: "✖", laeuft: "◆", uebersprungen: "–" }[s.stand] || "·"),
      h("b", { stil: { width: "210px" } }, s.name), s.stand === "laeuft" ? h("div", { stil: { width: "160px" } }, balken(s.fortschritt, "gold")) : null, h("span.klein.matt", null, s.text)))) : null,
    b.pruefungen.length ? h("div.liste", { stil: { marginTop: "12px" } }, b.pruefungen.map((p) => h("div.zeile.klein", null, h("span." + (p.ok ? "gruen" : "rot"), null, p.ok ? "✔" : "✖"), h("b", null, p.pruefung), h("span.matt", null, p.text)))) : null,
    b.ende ? h("p." + (b.ende.ok ? "gruen" : "rot"), null, b.ende.ok ? "Gebaut und geprüft – mit ▶ LOS tritt der Rat zusammen." : b.ende.text) : null,
    b.log.length ? h("details", null, h("summary.gold.klein", null, "Bauprotokoll"), h("pre.code", null, b.log.join("\n"))) : null);
}

function modelleKarte() {
  const z = S.zustand, g = z.gpu || {}, a = S.auswahl;
  return h("div.raster.r2", null,
    karte("Grafikkarte und lokale Modelle", g.name ? `${g.name} · ${zahl(g.vram_mb / 1024, 1)} GB VRAM, davon ${zahl((g.belegt_mb || 0) / 1024, 1)} GB belegt` : "Keine NVIDIA-Grafikkarte erkannt.",
      h("div.formular", null, h("label", null, "Ollama-Adresse"), h("div.zeile.eng", null, h("div", { stil: { flex: "1" } }, textfeld("/ollama_url", a.ollama_url, false, "http://127.0.0.1:11434")), knopf("Neu laden", () => sende("ollama_laden"), "klein")),
        h("label", null, "Kontextfenster"), regler(a.ollama_kontext, 4096, 65536, 1024, (x) => `${zahl(x)} Tokens`, (x) => setze("/ollama_kontext", x))),
      (z.ollama || []).length ? h("div.rahmen", { stil: { marginTop: "10px", maxHeight: "260px" } }, h("table.tab", null, h("tr", null, h("th", null, "Modell"), h("th", null, "Größe"), h("th", null, "VRAM")),
        z.ollama.map((m) => { const p = vramPasst(m.groesse); return h("tr", null, h("td.mono", null, m.name), h("td", null, gb(m.groesse)), h("td", null, p ? chip(p, p === "passt" ? "gruen" : p === "knapp" ? "bernstein" : "rot") : "–")); }))) : leer(z.ollama_fehler || "Keine Modelle.")),
    h("div.spalte", null,
      karte("Eigener Modellserver", "Jeder OpenAI-kompatible Server: LM Studio, vLLM, llama.cpp, text-generation-webui – auch dein selbst trainiertes Modell. Im Rat unter Modell „Eigener Server“ wählen.",
        h("div.formular", null, h("label", null, "Adresse"), textfeld("/eigen_url", a.eigen_url, false, "http://127.0.0.1:1234/v1"),
          h("label", null, "Schlüssel"), h("div.zeile.eng", null, h("input.feld", { type: "password", placeholder: z.eigen_da ? "hinterlegt – neu eingeben zum Ersetzen" : "optional", value: S.ui.eigenSchl || "", oninput: (e) => S.ui.eigenSchl = e.target.value, stil: { flex: "1" } }),
            knopf("In den Tresor", () => { sende("schluessel_eigen", { wert: S.ui.eigenSchl || "" }); S.ui.eigenSchl = ""; }, "klein")))),
      karte("OpenRouter", "Cloud-Modelle mit eigenem Schlüssel und Budget. Der Server ruft nie Modelle für dich auf.",
        h("div.formular", null, h("label", null, "Budget"), regler(a.openrouter_budget, 0.1, 50, 0.1, (x) => `${x.toFixed(1)} $`, (x) => setze("/openrouter_budget", x)),
          h("label", null, "Schlüssel"), h("div.zeile.eng", null, h("input.feld", { type: "password", placeholder: z.openrouter_da ? "hinterlegt – neu eingeben zum Ersetzen" : "sk-or-…", value: S.ui.orSchl || "", oninput: (e) => S.ui.orSchl = e.target.value, stil: { flex: "1" } }),
            knopf("In den Tresor", () => { sende("schluessel_openrouter", { wert: S.ui.orSchl || "" }); S.ui.orSchl = ""; }, "klein"))))));
}

function kontoKarte() {
  const z = S.zustand, info = z.server.info, s = z.konto?.schluessel;
  return h("div.raster.r2", null,
    karte("Spielserver", "Der Harness spricht ausschließlich mit der Harness-API dieses Servers. Browserseiten werden nie ausgelesen.",
      h("div.zeile.eng", null, h("input.feld", { value: S.ui.server ?? S.auswahl.server, oninput: (e) => S.ui.server = e.target.value, stil: { flex: "1" } }), knopf("Prüfen", () => sende("server", { wert: S.ui.server ?? S.auswahl.server }), "klein")),
      h("div", { stil: { marginTop: "10px" } }, info ? h("div.zeile", null, chip(`Harness-API v${info.version}`, "gruen"), chip(`Regeln ${info.regeln?.version}`), chip(`Welt ${(info.welt?.world_id || "").slice(0, 8)}`)) : h("span.rot", null, z.server.fehler || "prüfe …")),
      knopf("Regeln für Harnesses und Fair Play lesen", () => sende("regeln_lesen"), "leise klein", "extern")),
    karte("Harness-Schlüssel", "Persönlich, an Konto, Welt und Rollen gebunden, läuft ab, jederzeit widerrufbar. Liegt im Anmeldeinformationsspeicher von Windows.",
      s ? h("div.zeile", null, chip(s.label || "Schlüssel", "gold"), chip("Rollen: " + (s.rollen || []).map((r) => ROLLENTITEL[r] || r).join(", ")), chip("gültig bis " + new Date((s.gueltig_bis || 0) * 1000).toLocaleDateString("de-DE")))
        : h("p.matt", null, z.schluessel_da ? "Schlüssel hinterlegt." : "Noch kein Schlüssel – ▶ LOS meldet an und erstellt einen."),
      h("div.zeile", { stil: { marginTop: "10px" } }, knopf("Schlüssel entfernen (neu anmelden)", () => sende("schluessel_vergessen"), "leise klein", "kreuz"))));
}

function profileKarte() {
  const z = S.zustand;
  return karte("Profile", "Die ganze Harness-Einstellung (Rat, Seelen, Autonomie, Daueraufträge, Wissensquellen) unter einem Namen speichern, laden oder weitergeben – Schlüssel sind nie enthalten.",
    h("div.zeile", null, h("input.feld", { placeholder: "z. B. Aufbau, Krieg, Handel", value: S.ui.profil || "", oninput: (e) => S.ui.profil = e.target.value, stil: { maxWidth: "260px" } }),
      knopf("Speichern", () => sende("profil_speichern", { name: S.ui.profil || "" }), "gold klein", "haken"), knopf("Ordner", () => sende("oeffnen", { was: "profile" }), "leise klein", "ordner")),
    (z.profile || []).length ? h("div.kacheln", { stil: { marginTop: "12px" } }, z.profile.map((p) => h("div.kachel", null, h("b", null, p),
      h("div.zeile", { stil: { marginTop: "8px" } }, knopf("Laden", () => sende("profil_laden", { name: p }), "klein"), knopf("", () => sende("profil_loeschen", { name: p }), "leise klein", "kreuz"))))) : leer("Noch keine Profile."));
}

function schnittstellenKarte() {
  const z = S.zustand, k = S.katalog, mcp = JSON.stringify(k.mcp_eintrag, null, 2);
  return h("div.raster.r2", null,
    karte("Lokale API", "Für eigene Programme, Skripte und Dashboards auf diesem Rechner. Alles läuft durch Wachposten, Probe und Serverregeln.",
      h("div.zeile", null, chip(z.lokale_api || "http://127.0.0.1:8895 (nach dem Start)", z.lokale_api ? "gruen" : null), chip("Schlüssel: lokale-api.json im Harness-Ordner")),
      h("pre.code", { stil: { marginTop: "10px" } }, "GET  /status  /ereignisse?nach=  /lage?rolle=  /inventur  /analyse  /wissen\nPOST /befehl  /zug  /wissen  /mcp\nBefehle: routine, probezug, kontext, arena, wissen, export,\n         training_export, kennzahlen, hinweis, aufgabe_neu …")),
    karte("MCP", "Für Claude Desktop, Claude Code und jede MCP-Anwendung: 13 Werkzeuge (Lage, Zug, Inventur, Analyse, Routine, Wissen, Kontext …).",
      h("pre.code", null, mcp), h("div.zeile", { stil: { marginTop: "8px" } }, knopf("Kopieren", () => navigator.clipboard?.writeText(mcp), "klein", "kopie")),
      h("p.tipp", null, `Direkt am Server: ${S.auswahl.server}/api/harness/v1/mcp (Authorization: Bearer sep_…)`)));
}

// ---------------------------------------------------------------- Skills, Cronjobs, Werkzeuge (für den ganzen Rat)
function skillsKarte() {
  const kat = S.katalog, eigene = S.auswahl.abgeleitet?.eigene_skills || [];
  const alle = kat.skills.map((s) => ({ ...s, eigen: false })).concat(eigene.map((e) => ({ name: e.name, titel: name(e.name), text: e.text, eigen: true })));
  const zelle = (r, s) => { const w = rolleW(r), an = (w.skills || []).includes(s.name);
    return h("td", { stil: { textAlign: "center" } }, h("span.chip" + (an ? ".gold" : ""), { stil: { cursor: "pointer", justifyContent: "center", minWidth: "34px" }, title: `${ROLLENTITEL[r]}: ${an ? "abwählen" : "auswählen"}`,
      onclick: () => setze(`/rollen/${r}/skills`, an ? w.skills.filter((x) => x !== s.name) : (w.skills || []).concat(s.name)) }, an ? ikon("haken") : "·")); };
  return h("div.raster", { stil: { gridTemplateColumns: "minmax(0, 1.4fr) minmax(0, 1fr)" } },
    karte("Skills im Rat", "Wissensmodule, die in den Systemprompt der Rolle kommen. Jede Zeile ein Skill, jede Spalte eine Rolle – ein Klick schaltet um. Was sie an Kontext kosten, zeigt der Rat unter „Kontext“.",
      h("div.rahmen", { stil: { maxHeight: "none", border: "0" } }, h("table.tab", null,
        h("tr", null, h("th", null, "Skill"), ROLLEN.map((r) => h("th", { stil: { textAlign: "center" } }, h("img.ava", { src: portraet(r), alt: ROLLENTITEL[r], title: ROLLENTITEL[r], stil: { "--rolle": ROLLENFARBE[r], width: "26px", height: "26px" } })))),
        alle.map((s) => h("tr", null, h("td", null, h("b", null, s.titel), s.eigen ? h("span.chip.cyan", { stil: { marginLeft: "6px" } }, "eigen") : null,
          h("div.winzig.matt", null, (s.text || "").split("\n").find((z) => z.trim() && !z.startsWith("#"))?.slice(0, 120) || "")), ROLLEN.map((r) => zelle(r, s))))))),
    h("div.spalte", null,
      karte("Eigenen Skill anlegen", "Ein Skill ist eine Markdown-Datei im Ordner „skills“ des Harness: Regeln, Abläufe, Faustformeln. Der Dateiname ist der Skillname (a–z, 0–9, - und _).",
        h("div.zeile", null, h("input.feld", { placeholder: "z. B. handelsrouten", value: S.ui.neuerSkill || "", oninput: (e) => S.ui.neuerSkill = e.target.value, stil: { maxWidth: "220px" } }),
          knopf("Anlegen", () => { sende("skill_neu", { name: S.ui.neuerSkill || "" }); S.ui.neuerSkill = ""; }, "gold klein", "plus"), knopf("Skill-Ordner öffnen", () => sende("oeffnen", { was: "skills" }), "leise klein", "ordner")),
        h("p.tipp", null, "Tipp: Ein guter Skill sagt, wann er gilt, was zu tun ist und woran man Erfolg erkennt – kurz, damit er wenig Kontext kostet.")),
      karte("Eigene Skills", eigene.length ? `${eigene.length} im Ordner „skills“` : "Noch keine eigenen Skills.",
        eigene.length ? h("div.liste", null, eigene.map((e) => h("div.eintrag", null, h("div.text", null, h("b.mono", null, e.name),
          h("pre.code", { stil: { maxHeight: "120px", overflow: "auto", margin: "6px 0 0" } }, e.text.slice(0, 1200)))))) : null)));
}

function cronjobsKarte() {
  const a = S.auswahl, kat = S.katalog, z = S.zustand;
  return h("div.spalte", null,
    karte("Cronjobs", "Routinen ohne Modell, die nach Zeitplan laufen: Inventur aller Planeten, Analyse mit Befunden, Auto-Ausbau. Sie kosten keine Tokens; Bauaufträge gehen wie jeder Zug durch Wachposten, Probe und Server.",
      h("div.rahmen", { stil: { maxHeight: "none", border: "0" } }, h("table.tab", null,
        h("tr", null, h("th", null, "Routine"), h("th", null, "Intervall"), h("th", null, "Letzter Lauf"), h("th", null, "Meldet an"), h("th", null, "")),
        a.routinen.map((rt, i) => { const st = z.routinen?.[rt.name], info = kat.routinen.find((x) => x.name === rt.art) || {};
          return h("tr", null, h("td", null, schalter(rt.aktiv, h("span", null, h("b", null, info.titel || rt.art), h("span.winzig.matt", null, " " + rt.name)), (v) => setze(`/routinen/${i}/aktiv`, v))),
            h("td", null, `alle ${rt.alle_minuten} min`),
            h("td.klein" + (st && st.ok === false ? ".rot" : ""), null, st?.laeuft ? "läuft …" : st?.kurz ? `${st.ok ? "✔" : "✖"} ${st.kurz.slice(0, 80)}` : "–"),
            h("td.klein", null, rt.melden_an ? ROLLENTITEL[rt.melden_an] : "–"),
            h("td", null, knopf("Jetzt", () => harness({ befehl: "routine", name: rt.name }), "klein", "los", !laeuft()))); })))),
    h("div.raster.r3", null, a.routinen.map((rt, i) => routineKarte(rt, i)),
      h("div.kachel", { stil: { display: "flex", flexDirection: "column", gap: "8px", justifyContent: "center", alignItems: "flex-start" } },
        h("b", null, "Neuer Cronjob"), h("div.zeile.eng", null, kat.routinen.map((r) => knopf(r.titel, () => sende("routine_neu", { art: r.name }), "klein", "plus"))))));
}

function werkzeugeKarte() {
  const kat = S.katalog, a = S.auswahl, quellen = a.wissen || [];
  return h("div.spalte", null,
    karte("Werkzeuge im Rat", "Lesende Werkzeuge, mit denen eine Rolle nachschlägt, bevor sie entscheidet (Kosten, Flugzeit, Kampfsimulator, Analyse, Inventur, Wissen …). Gesperrte Werkzeuge verschwinden aus ihrem Antwortschema.",
      h("div.rahmen", { stil: { maxHeight: "none", border: "0" } }, h("table.tab", null,
        h("tr", null, h("th", null, "Werkzeug"), ROLLEN.map((r) => h("th", { stil: { textAlign: "center" } }, h("img.ava", { src: portraet(r), alt: ROLLENTITEL[r], title: ROLLENTITEL[r], stil: { "--rolle": ROLLENFARBE[r], width: "26px", height: "26px" } })))),
        kat.werkzeuge.map((t) => h("tr", null, h("td", null, h("b", null, name(t.name)), h("div.winzig.matt", null, t.text)),
          ROLLEN.map((r) => { const w = rolleW(r), an = (w.werkzeuge || []).includes(t.name);
            return h("td", { stil: { textAlign: "center" } }, h("span.chip" + (an ? ".gold" : ""), { stil: { cursor: "pointer", justifyContent: "center", minWidth: "34px" },
              onclick: () => setze(`/rollen/${r}/werkzeuge`, an ? w.werkzeuge.filter((x) => x !== t.name) : (w.werkzeuge || []).concat(t.name)) }, an ? ikon("haken") : "·")); })))))),
    h("div.raster.r3", null,
      karte("Wissensquellen als Werkzeug", `${quellen.length} Quelle(n): eigenes Gedächtnis per SQL, eigenes RAG, MCP-Server.`,
        quellen.length ? h("div.liste", null, quellen.map((q) => h("div.eintrag", null, h("div.text", null, h("b", null, q.name), h("span.winzig.matt", null, `${q.art === "gedaechtnis" ? "SQL" : q.art === "http" ? "RAG" : "MCP"} · ${(q.rollen || []).map((r) => ROLLENTITEL[r]).join(", ") || "keine Rolle"}`))))) : leer("Noch keine Quellen."),
        h("div.zeile", { stil: { marginTop: "8px" } }, knopf("Wissen einrichten", () => nav("wissen"), "klein", "wissen"))),
      karte("Plugins (Hooks)", a.erweiterung === "WasmSandbox" ? "WASM-Sandbox ist eingebaut: Plugins prüfen und ändern jeden Zug, bevor er gespielt wird." : "Ohne Plugins gebaut – unter „Bau“ die WASM-Sandbox wählen.",
        h("p.klein", null, "Beispiel: der Energiewächter streicht Bauaufträge, die den Strom ins Minus drücken würden. Eigene Plugins sind WebAssembly-Module ohne Importe, mit Treibstoff- und Speichergrenze."),
        h("div.zeile", null, knopf("Harness-Ordner", () => sende("oeffnen", { was: "arbeitsordner" }), "leise klein", "ordner"))),
      karte("Fremde Werkzeuge (MCP)", "Andere Programme – Claude Desktop, Claude Code, eigene Skripte – steuern den Rat über die lokale API und MCP mit.",
        h("div.zeile", null, chip(S.zustand.lokale_api || "lokale API nach dem Start", S.zustand.lokale_api ? "gruen" : null)),
        h("div.zeile", { stil: { marginTop: "8px" } }, knopf("Schnittstellen", () => { S.ui.werkstatt.reiter = "schnittstellen"; planen(); }, "klein", "pfeil")))));
}

function seiteWerkstatt() {
  const r = S.ui.werkstatt.reiter;
  return h("div.seite", null,
    aenderungsleiste(),
    h("div.seitenkopf", null, h("div", null, h("h1", null, "Werkstatt"), h("p", null, "Hier entsteht dein Harness: Bausteine wählen, Abhängigkeiten prüfen, „Let's go“ – geladen wird nur mit Prüfsumme, eingerichtet und geprüft wird auf deinem Rechner.")),
      h("div.rechts", null, knopf("Einrichtungsassistent", () => { S.ui.einrichtung = 1; planen(); }, "klein", "stern"), knopf("Harness-Ordner", () => sende("oeffnen", { was: "arbeitsordner" }), "leise klein", "ordner"))),
    h("div", { stil: { marginBottom: "16px" } }, segment([["bau", "Bau"], ["modelle", "Modelle"], ["skills", "Skills"], ["cronjobs", "Cronjobs"], ["werkzeuge", "Werkzeuge"], ["konto", "Konto"], ["profile", "Profile"], ["schnittstellen", "Schnittstellen"]], r, (v) => { S.ui.werkstatt.reiter = v; planen(); }, true)),
    r === "modelle" ? modelleKarte() : r === "skills" ? skillsKarte() : r === "cronjobs" ? cronjobsKarte() : r === "werkzeuge" ? werkzeugeKarte() : r === "konto" ? h("div.spalte",null,testpilotenKarte(),kontoKarte()) : r === "profile" ? profileKarte() : r === "schnittstellen" ? schnittstellenKarte() : h("div.spalte", null, bausteine(), bauKarte()));
}

// ---------------------------------------------------------------- Einrichtungsassistent
const EINRICHTUNG = ["Willkommen", "Spielstil", "Modelle", "Bauen", "Los"];
/** Was im Assistenten wirklich erledigt ist – kein Haken für bloßes Weiterklicken. */
function einrichtungStand() {
  const z = S.zustand, aktiv = ROLLEN.filter((r) => rolleW(r).aktiv);
  const installiert = (z.ollama || []).map((m) => m.name);
  const modellDa = (r) => {
    const w = rolleW(r);
    if (!(w.modell || "").trim()) return false;
    if (w.anbieter === "OpenRouter") return !!z.openrouter_da;
    if (w.anbieter === "Eigen") return !!(S.auswahl.eigen_url || "").trim();
    return installiert.some((m) => m === w.modell || m === w.modell + ":latest");
  };
  return [!!z.server?.info, !!S.auswahl.spielstil, aktiv.length > 0 && aktiv.every(modellDa), !!z.gebaut, !!z.laeuft];
}

/** Empfehlung aus den installierten Modellen (wie im Kern): größtes Chat-Modell, das in den Grafikspeicher passt. */
function modellEmpfehlung() {
  const z = S.zustand, grenze = z.gpu?.vram_mb ? z.gpu.vram_mb * 1e6 * 0.85 : 9e9;
  const chat = (n) => !/embed|-vl|vision|ocr|coder|summary|rerank|whisper|tts|cpu/i.test(n);
  return (z.ollama || []).filter((m) => m.groesse > 5e8 && m.groesse <= grenze && chat(m.name)).sort((a, b) => b.groesse - a.groesse)[0]?.name || null;
}

function einrichtungDialog() {
  const z = S.zustand;
  if (S.ui.einrichtung === null && !z.gebaut && !z.laeuft && !z.lokal?.connected && S.ui.seite !== "lokal" && !S.ui.einrichtungZu) S.ui.einrichtung = 1;
  const n = S.ui.einrichtung;
  if (!n) return null;
  const stand = einrichtungStand();
  const weiter = (x) => {
    S.ui.einrichtung = x;
    // Schritt „Bauen“: Abhängigkeiten sofort prüfen, damit „Let's go“ gleich frei ist.
    if (x === 4 && !z.gebaut && !(z.abh || []).length && !z.abh_laedt) sende("abh_pruefen");
    if (x === 3) sende("ollama_laden");
    planen();
  };
  const vorneR = Rund.vorne();
  const koerper = [null,
    () => h("div.spalte.willkommen", null,
      h("div.willkommen-kopf", null, h("h2", null, "Willkommen, Kommandant – dein Rat"),
        knopf("Hier auch Singleplayer offline spielen", () => { S.ui.einrichtung = null; S.ui.einrichtungZu = true; nav("lokal"); }, "gold klein", "spiel")),
      h("p.matt.offline-hinweis", null, "Offline ohne Konto oder Sprachmodell spielen. Unter „Lokale Sitzung“ kannst du auch im WLAN/LAN gemeinsam spielen."),
      h("p.rat-einleitung", null, "Vier Agenten bilden deinen Rat: Stratege, Verwalter, Diplomat und Feldherr – jeder mit eigenem Modell, eigener Seele und eigenen Rechten. Entdecke ihre Häuser und lass sie sich in ihrer eigenen Stimme vorstellen."),
      h("div.zeile.eng.ratsrollen", { "aria-label": "Ratsrolle wählen" }, ROLLEN.map((r) => h("button.k.klein" + (r === vorneR ? ".gold" : ".leise"), { type: "button", "aria-pressed": String(r === vorneR), onclick: () => { Rund.zeigen(r); Hausbuehne.vorstellen(r, gestalt(r)); } }, h("img.ava", { src: portraet(r), alt: "", stil: { width: "22px", height: "22px", "--rolle": ROLLENFARBE[r] } }), ROLLENTITEL[r]))),
      haeuserVorstellung(vorneR),
      h("div.formular", null, h("label", null, "Spielserver"), h("input.feld", { value: S.ui.server ?? S.auswahl.server, oninput: (e) => S.ui.server = e.target.value })),
      h("div.zeile", null, z.server.info ? chip("Server erreichbar · Regeln " + z.server.info.regeln?.version, "gruen") : chip(z.server.fehler || "prüfe Server …", z.server.fehler ? "rot" : null),
        knopf("Prüfen", () => sende("server", { wert: S.ui.server ?? S.auswahl.server }), "leise klein"))),
    () => h("div.spalte", null, h("h2", null, "Wie soll dein Reich spielen?"), h("p.matt", null, "Der Spielstil besetzt den Rat stimmig. Alles ist später fein einstellbar."),
      h("div.kacheln", { stil: { gridTemplateColumns: "repeat(3, minmax(0, 1fr))" } }, S.katalog.spielstile.map((s) => h("div.kachel.bild" + (S.auswahl.spielstil === s.name ? ".an" : ""), { onclick: () => sende("spielstil", { name: s.name }) },
        h("div.grund", { stil: { backgroundImage: `url(${bild(s.bild)})` } }), S.auswahl.spielstil === s.name ? h("span.haken", null, ikon("haken")) : null, h("b", null, s.titel), h("p", null, s.kurz))))),
    () => {
      const installiert = z.ollama || [], empf = modellEmpfehlung();
      const da = (m) => installiert.find((x) => x.name === m || x.name === m + ":latest");
      const fehlen = ROLLEN.filter((r) => rolleW(r).aktiv && rolleW(r).anbieter === "Ollama" && !da(rolleW(r).modell));
      const aktiv = ROLLEN.filter((r) => rolleW(r).aktiv), cloud = aktiv.length > 0 && aktiv.every((r) => rolleW(r).anbieter === "OpenRouter");
      const orModell = S.ui.orModell ?? (cloud ? rolleW(aktiv[0]).modell : "") ?? "";
      const allesCloud = () => { const m = (S.ui.orModell || "").trim() || OPENROUTER_VORSCHLAG;
        aktiv.forEach((r) => { sende("anbieter", { rolle: r, wert: "openrouter" }); setze(`/rollen/${r}/modell`, m); }); S.ui.orModell = m; };
      const allesLokal = () => aktiv.forEach((r) => { sende("anbieter", { rolle: r, wert: "ollama" }); if (empf && !da(rolleW(r).modell)) setze(`/rollen/${r}/modell`, empf); });
      const weg = h("div.zeile", { stil: { alignItems: "center" } }, h("span.winzig.matt", null, "Wo rechnen die Modelle?"),
        segment([["lokal", "Auf diesem PC (Ollama)"], ["cloud", "In der Cloud (OpenRouter)"]], cloud ? "cloud" : "lokal", (v) => (v === "cloud" ? allesCloud() : allesLokal()), true));
      if (cloud) return h("div.spalte", null, h("h2", null, "Welche Modelle?"), weg,
        h("p.matt", null, "Ohne eigene Grafikkarte: Der Rat nutzt Cloud-Modelle über OpenRouter mit deinem Schlüssel und deinem Budget. Der Spielserver ruft nie Modelle für dich auf."),
        h("div.formular", null,
          h("label", null, "OpenRouter-Schlüssel"), h("div.zeile.eng", null, h("input.feld", { type: "password", placeholder: z.openrouter_da ? "hinterlegt – neu eingeben zum Ersetzen" : "sk-or-…", value: S.ui.orSchl || "", oninput: (e) => S.ui.orSchl = e.target.value, stil: { flex: "1" } }),
            knopf("In den Tresor", () => { sende("schluessel_openrouter", { wert: S.ui.orSchl || "" }); S.ui.orSchl = ""; }, "klein")),
          h("label", null, "Modell für alle Rollen"), h("div.zeile.eng", null, h("input.feld", { value: orModell, placeholder: OPENROUTER_VORSCHLAG, oninput: (e) => S.ui.orModell = e.target.value, stil: { flex: "1" } }),
            knopf("Übernehmen", allesCloud, "klein")),
          h("label", null, "Budget"), regler(S.auswahl.openrouter_budget, 0.1, 50, 0.1, (x) => `${x.toFixed(1)} $`, (x) => setze("/openrouter_budget", x))),
        h("div.zeile", null, z.openrouter_da ? chip("Schlüssel im Tresor", "gruen") : chip("Schlüssel fehlt", "bernstein"),
          aktiv.map((r) => chip(`${ROLLENTITEL[r]}: ${rolleW(r).modell || "–"}`, (rolleW(r).modell || "").trim() ? null : "bernstein"))),
        h("p.tipp", null, "Jede Rolle kann später ein eigenes Modell bekommen (Rat → Modell). Den Schlüssel gibt es auf openrouter.ai unter „Keys“."));
      return h("div.spalte", null, h("h2", null, "Welche Modelle?"), weg,
        h("p.matt", null, (z.gpu?.name ? `${z.gpu.name} mit ${zahl(z.gpu.vram_mb / 1024, 1)} GB Grafikspeicher. ` : "") +
          (z.ollama_fehler ? `Ollama: ${z.ollama_fehler}` : `Dein Ollama (${S.auswahl.ollama_url}) hat ${installiert.length} ${installiert.length === 1 ? "Modell" : "Modelle"}.`)),
        fehlen.length && empf ? h("div.eintrag.warnung", null, h("div.text", null, h("b", null, `${fehlen.map((r) => ROLLENTITEL[r]).join(", ")}: Modell nicht installiert`),
          h("span.winzig.matt", null, `Empfehlung aus deinen Modellen: ${empf} – passt in den Grafikspeicher.`)),
          knopf(`${empf} übernehmen`, () => fehlen.forEach((r) => setze(`/rollen/${r}/modell`, empf)), "gold klein", "haken")) : null,
        h("div.liste", null, ROLLEN.map((r) => { const w = rolleW(r), d = da(w.modell);
          return h("div.eintrag", null, avatar(r, 34), h("div.text", null, h("b", null, ROLLENTITEL[r]),
            h("span.winzig" + (w.aktiv && !d && w.anbieter === "Ollama" ? ".bernstein" : ".matt"), null, !w.aktiv ? "nicht im Rat" : w.anbieter !== "Ollama" ? `über ${w.anbieter}` : d ? `installiert · ${gb(d.groesse)}${vramPasst(d.groesse) ? " · " + vramPasst(d.groesse) : ""}` : "nicht installiert – „Let's go“ würde es herunterladen")),
            schalter(w.aktiv, "", (v) => setze(`/rollen/${r}/aktiv`, v)),
            h("select.feld", { stil: { maxWidth: "300px" }, onchange: (e) => setze(`/rollen/${r}/modell`, e.target.value) },
              (d ? [] : [[w.modell, `${w.modell} (nicht installiert)`]]).concat(installiert.map((m) => [m.name, `${m.name} · ${gb(m.groesse)}${vramPasst(m.groesse) ? " · " + vramPasst(m.groesse) : ""}`]))
                .map(([v, tt]) => h("option", { value: v, selected: w.modell === v || w.modell + ":latest" === v }, tt)))); })),
        h("p.tipp", null, installiert.length ? "Eigene Server (LM Studio, vLLM) und Feinheiten später in Werkstatt → Modelle und im Rat." : "Kein Ollama auf diesem PC? Oben „In der Cloud (OpenRouter)“ wählen – oder Ollama von ollama.com installieren und „Weiter“ erneut prüfen."));
    },
    () => h("div.spalte", null, h("h2", null, "Bauen"), h("p.matt", null, z.vorgebaut?.passt
        ? "Der Harness liegt fertig übersetzt bei: Er wird mit Prüfsumme übernommen, eingerichtet und prüft sich danach selbst – ohne Rust und ohne Buildwerkzeuge."
        : "Der Harness wird auf deinem Rechner aus Quelltext gebaut – mit genau den gewählten Bausteinen. Danach prüft er sich selbst."),
      bauKarte()),
    () => h("div.spalte", null, h("h2", null, "Bereit"),
      h("p", null, z.gebaut ? "Dein Harness ist gebaut und geprüft. Mit LOS meldest du dich einmal mit deinem Spielkonto an, der Rat bekommt seinen Schlüssel, tritt zusammen – und es geht direkt im Spiel weiter." : "Noch nicht gebaut – zurück zu Schritt 4 „Bauen“."),
      h("div.zeile", null, knopf("LOS – Rat einberufen", () => { S.ui.einrichtung = null; S.ui.einrichtungZu = true; sende("los"); nav("spiel"); }, "gold gross", "los", !z.gebaut),
        z.gebaut ? null : knopf("Zu „Bauen“", () => weiter(4), "leise", "werkstatt"))),
  ];
  // „Weiter“ nur, wenn der Schritt wirklich erledigt ist; Bauen lässt sich nicht überspringen.
  const sperre = n === 4 && !stand[3] ? "Erst bauen – „Let's go“ starten" : n === 3 && !stand[2] ? "Erst für jede Rolle im Rat ein Modell wählen – installiert (Ollama) oder über OpenRouter mit Schlüssel" : null;
  return dialog(h("div.assistent", null,
    h("div.schritte", null, EINRICHTUNG.map((t, i) => h("div.schritt" + (i + 1 === n ? ".an" : stand[i] ? ".fertig" : ""), { onclick: () => weiter(i + 1), stil: { cursor: "pointer" } }, h("i", null, stand[i] ? "✔" : String(i + 1)), t)),
      h("div", { stil: { flex: "1" } }), knopf("Später", () => { S.ui.einrichtung = null; S.ui.einrichtungZu = true; planen(); }, "leise klein")),
    h("div.koerper", null, koerper[n](), h("div.knopfreihe", null, sperre ? h("span.bernstein.klein", { stil: { marginRight: "auto" } }, sperre) : null,
      n > 1 ? knopf("Zurück", () => weiter(n - 1), "leise") : null,
      n < EINRICHTUNG.length ? knopf("Weiter", () => { if (n === 1 && S.ui.server && S.ui.server !== S.auswahl.server) sende("server", { wert: S.ui.server }); weiter(n + 1); }, "gold", "pfeil", !!sperre) : null))), true);
}
