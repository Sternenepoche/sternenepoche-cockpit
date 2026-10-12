/* Rat: jedes Ratsmitglied als Agent mit sichtbarem Harness – zehn Stufen vom Auslöser bis zum Lernen. */
"use strict";

const STUFEN = [
  ["identitaet", "Identität", "Seele, Vorlage, Skills"],
  ["ausloeser", "Auslöser", "Takt, Wecker, Budget"],
  ["wahrnehmung", "Wahrnehmung", "Lage vom Server"],
  ["gedaechtnis", "Gedächtnis", "Notizen, Erinnerungen, Team"],
  ["kontext", "Kontext", "Was ins Modell geht"],
  ["modell", "Modell", "Anbieter, Modell, Ersatz"],
  ["werkzeuge", "Werkzeuge", "Abfragen und Wissen"],
  ["wachposten", "Wachposten", "Autonomie und Rechte"],
  ["probe", "Probe", "Spielkern prüft vorab"],
  ["lernen", "Lernen", "Ergebnis, Notiz, Erinnerung"],
];

function gb(bytes) { return (bytes / 1e9).toFixed(1) + " GB"; }
function vramPasst(bytes) {
  const g = S.zustand.gpu; if (!g?.vram_mb || !bytes) return null;
  const v = g.vram_mb * 1048576;
  return bytes < v * 0.82 ? "passt" : bytes < v ? "knapp" : "zu groß";
}
function kontextInfo(r) {
  const k = S.daten?.kontext?.[r];
  const nctx = S.auswahl.ollama_kontext || 16384;
  return { k, tokens: k?.tokens_geschaetzt, nctx, voll: k ? k.tokens_geschaetzt / nctx : null };
}

function stufeWert(r, s) {
  const w = rolleW(r), a = rolleA(r), aus = S.auswahl;
  switch (s) {
    case "identitaet": return [w.vorlage || "eigene Seele", a.seele_version ? "Seele " + a.seele_version : `${(w.skills || []).length} Skills`];
    case "ausloeser": return [`alle ${w.takt_minuten} min`, `${(w.ausloeser || []).length} Auslöser${w.max_aufrufe_stunde ? " · max " + w.max_aufrufe_stunde + "/h" : ""}`];
    case "wahrnehmung": return [`${zahl(aus.kontext_zeichen)} Zeichen Lage`, "gefiltert vom Spielkern"];
    case "gedaechtnis": return [`${aus.erinnerungen_im_kontext} Erinnerungen`, aus.planung_aktiv ? "plant mit dem Team" : "ohne Planung"];
    case "kontext": { const i = kontextInfo(r); return [i.tokens ? `≈ ${zahl(i.tokens)} Tokens` : "Vorschau fehlt", `Fenster ${zahl(i.nctx)}`, i.voll > 1 ? "fehler" : i.voll > 0.85 ? "warn" : ""]; }
    case "modell": return [w.modell || "–", a.latenz ? `${zahl(a.latenz)} ms zuletzt` : (w.anbieter || "")];
    case "werkzeuge": { const q = (aus.wissen || []).filter((x) => (x.rollen || []).includes(r)).length; return [`${(w.werkzeuge || []).length} Werkzeuge`, q ? `${q} Wissensquellen` : "ohne eigene Quellen"]; }
    case "wachposten": { const au = aus.abgeleitet?.autonomie?.[r] || {}; const n = Object.values(au).filter((x) => x === "autopilot").length; return [`${n} Bereiche Autopilot`, `${Object.values(au).filter((x) => x === "erlaubnis" || x === "gesperrt").length} streng`]; }
    case "probe": return [aus.probe_vor_zug ? "Probe vor jedem Zug" : "ohne Probe", `${aus.korrekturrunden} Korrekturrunde(n)`];
    case "lernen": return [`${zahl(a.angenommen || 0)} ✔ · ${zahl(a.abgelehnt || 0)} ✖`, a.kosten ? `${a.kosten.toFixed(3)} $` : "Notiz und Erinnerung"];
  }
  return ["", ""];
}

function schaltplan(r) {
  const a = rolleA(r);
  const aktiv = laeuft() && a.stufe && (jetzt() - (a.stufe_seit || 0) < 25) && (a.status === "denkt" || a.status === "korrigiert" || a.stufe === "wahrnehmung") ? a.stufe : null;
  return h("div.schaltplan", null, STUFEN.map(([k, t, u]) => {
    const [w1, w2, art] = stufeWert(r, k);
    return h("div.stufe" + (S.ui.stufe === k ? ".gewaehlt" : "") + (aktiv === k ? ".aktiv" : "") + (art ? "." + art : ""), { onclick: () => { S.ui.stufe = k; planen(); }, title: u },
      h("div.ikon", null, ikon(k, true)), h("b", null, t), h("span", null, w1), h("div.wert", null, w2));
  }));
}

// ---------------------------------------------------------------- Stufen-Einstellungen
function stufeIdentitaet(r) {
  const w = rolleW(r), kat = S.katalog, pfad = `/rollen/${r}`;
  const vorlagen = kat.vorlagen.filter((v) => v.rolle === r);
  const eigene = S.auswahl.abgeleitet?.eigene_skills || [];
  const alleSkills = kat.skills.concat(eigene.map((e) => ({ name: e.name, titel: name(e.name) + " (eigen)", text: e.text })));
  return h("div.raster", { stil: { gridTemplateColumns: "minmax(0, 1.25fr) minmax(0, 1fr)" } },
    karte("Seele (soul.md)", "Persönlichkeit, Auftrag, Arbeitsweise und Grenzen als Markdown. Liegt als seelen/" + r + ".md im Harness-Ordner und wird vor jedem Zug frisch gelesen – auch Änderungen mit deinem eigenen Editor wirken sofort.",
      textfeld(pfad + "/seele_md", w.seele_md, true, "# Seele …", { code: true, rows: 18, spellcheck: "false" }),
      h("div.zeile", { stil: { marginTop: "8px" } }, h("span.winzig.matt", null, `${(w.seele_md || "").length} Zeichen`),
        h("span.rechts"), knopf("Im Editor öffnen", () => sende("oeffnen", { was: "seele", rolle: r }), "leise klein", "extern"), knopf("Aus Datei laden", () => sende("seele_neu_laden"), "leise klein"),
        laeuft() ? knopf("Probezug mit dieser Seele", () => { sende("uebernehmen"); setTimeout(() => harness({ befehl: "probezug", rolle: r }), 400); S.ui.stufe = "probe"; planen(); }, "klein", "probe") : null)),
    h("div.spalte", null,
      karte("Vorlagen", "Fertige Ausprägungen dieser Rolle – Seele, Skills, Werkzeuge, Auslöser, Rechte und Takt in einem Klick. Modell und Anbieter bleiben.",
        h("div.kacheln", { stil: { gridTemplateColumns: "1fr 1fr" } }, vorlagen.map((v) => h("div.kachel" + (w.vorlage === v.name ? ".an" : ""), { onclick: () => sende("vorlage", { rolle: r, name: v.name }) },
          w.vorlage === v.name ? h("span.haken", null, ikon("haken")) : null, h("b", null, v.name), h("p", null, v.kurz))))),
      karte("Skills", "Wissensmodule im Systemprompt. Eigene Skills sind Markdown-Dateien im Ordner „skills“.",
        h("div.zeile.eng", null, alleSkills.map((s) => { const an = (w.skills || []).includes(s.name);
          return h("span.chip" + (an ? ".gold" : ""), { title: (s.text || "").slice(0, 600), stil: { cursor: "pointer" }, onclick: () => setze(pfad + "/skills", an ? w.skills.filter((x) => x !== s.name) : (w.skills || []).concat(s.name)) }, an ? ikon("haken") : ikon("plus"), s.titel); })),
        h("div.zeile", { stil: { marginTop: "10px" } }, h("input.feld", { placeholder: "neuer-skill", value: S.ui.neuerSkill || "", oninput: (e) => S.ui.neuerSkill = e.target.value, stil: { maxWidth: "180px" } }),
          knopf("Anlegen", () => { sende("skill_neu", { name: S.ui.neuerSkill || "" }); S.ui.neuerSkill = ""; }, "klein", "plus"), knopf("Skill-Ordner", () => sende("oeffnen", { was: "skills" }), "leise klein", "ordner")))));
}

function stufeAusloeser(r) {
  const w = rolleW(r), pfad = `/rollen/${r}`, kat = S.katalog;
  return h("div.raster.r2", null,
    karte("Takt und Budget", "Wie oft die Rolle von sich aus entscheidet – und wie viele Modellzüge je Stunde höchstens.",
      h("div.formular", null,
        h("label", null, "Im Rat"), schalter(w.aktiv, w.aktiv ? "sitzt im Rat" : "nicht im Rat", (v) => setze(pfad + "/aktiv", v), "Ändert die Rollen des Harness-Schlüssels: danach neu anmelden"),
        h("label", null, "Zug alle"), regler(w.takt_minuten, 2, 240, 1, (x) => `${x} min`, (x) => setze(pfad + "/takt_minuten", x)),
        h("label", null, "Budget"), regler(w.max_aufrufe_stunde || 0, 0, 60, 1, (x) => x ? `${x} je h` : "frei", (x) => setze(pfad + "/max_aufrufe_stunde", x)))),
    karte("Aufwachen bei", "Außer im Takt zieht die Rolle sofort, wenn eines dieser Ereignisse eintritt. Die Wache des Harness prüft die Lage regelmäßig.",
      h("div.spalte", null, kat.ausloeser.map((a) => schalter((w.ausloeser || []).includes(a.name), a.text, (v) => setze(pfad + "/ausloeser", v ? (w.ausloeser || []).concat(a.name) : w.ausloeser.filter((x) => x !== a.name))))),
      h("div.trenner"), h("div.formular", null, h("label", null, "Wache prüft alle"), regler(S.auswahl.wache_sekunden, 20, 600, 10, (x) => `${x} s`, (x) => setze("/wache_sekunden", x)))));
}

function stufeWahrnehmung() {
  const a = S.auswahl;
  return h("div.raster.r2", null,
    karte("Lagebild", "Der Spielkern liefert jeder Rolle nur ihre gefilterte Sicht. Ist die Lage zu groß, fallen selten gebrauchte Teile zuerst weg (Chronik, Register, Berichte, Rangliste, Markt …) – das Modell kann sie über Werkzeuge nachholen.",
      h("div.formular", null, h("label", null, "Höchstens"), regler(a.kontext_zeichen, 6000, 80000, 1000, (x) => `${zahl(x)} Zeichen`, (x) => setze("/kontext_zeichen", x))),
      h("p.tipp", null, "Kleine Modelle: 8 000–14 000 Zeichen. Große Modelle mit 32k-Kontext: 24 000 und mehr.")),
    karte("Schutz vor Prompt-Injection", "Texte anderer Spieler (Briefe, Namen, Angebote) verlassen das Lagebild und stehen nur in einem mit Zufallsmarke abgegrenzten Datenblock. Hinweise der Spielleitung kommen nur über server_hinweise.",
      h("div.zeile", null, chip("Fremdtexte isoliert", "gruen"), chip("Unsichtbare Zeichen abgewiesen", "gruen"), chip("Antworten von RAG/MCP in Quarantäne", "gruen"))));
}

function stufeGedaechtnis() {
  const a = S.auswahl;
  return h("div.raster.r2", null,
    karte("Kurz- und Langzeitgedächtnis", "Notizbuch je Rolle, Erinnerungen des Teams (nach Relevanz in den Prompt: Absender, Angriffsziele) und Teambesprechung.",
      h("div.formular", null,
        h("label", null, "Erinnerungen im Prompt"), regler(a.erinnerungen_im_kontext, 0, 40, 1, (x) => String(x), (x) => setze("/erinnerungen_im_kontext", x)),
        h("label", null, "Notizen behalten"), regler(a.notizen_behalten, 5, 500, 5, (x) => String(x), (x) => setze("/notizen_behalten", x))),
      h("div.zeile", { stil: { marginTop: "8px" } }, knopf("Gedächtnis ansehen", () => nav("plan", { plan: { reiter: "erinnerungen" } }), "klein", "gedaechtnis"), knopf("Per SQL abfragen", () => nav("wissen"), "leise klein", "wissen"))),
    karte("Planung im Team", "Rollen legen Aufgaben an, übernehmen sie und melden sich gegenseitig Bedarf – wie ein Todo-Werkzeug eines Coding-Agenten.",
      h("div.spalte", null, schalter(a.planung_aktiv, "Rollen planen selbst (aufgaben, an_team, merken)", (v) => setze("/planung_aktiv", v)),
        h("div.formular", null, h("label", null, "Offene Aufgaben höchstens"), regler(a.max_offen, 5, 100, 1, String, (x) => setze("/max_offen", x))))));
}

const TEIL_FARBEN = ["#dab57b", "#f3d9a6", "#b9a3ff", "#7fd3ff", "#3fa9e0", "#8fdca5", "#4fd994", "#f4b467", "#ff8c8c", "#63b1ff", "#9fb6cc", "#ffd166", "#c59cff", "#5ee3d0"];
function kontextBudget(r, gross) {
  const i = kontextInfo(r), k = i.k;
  if (!k) return h("div.spalte", null, leer(laeuft() ? "Noch keine Vorschau – „Vorschau aktualisieren“ zeigt, was das Modell sieht." : "Die Kontext-Vorschau braucht den laufenden Rat (▶ LOS)."),
    laeuft() ? knopf("Vorschau aktualisieren", () => harness({ befehl: "kontext", rolle: r }), "klein", "kontext") : null);
  const teile = (k.teile || []).filter((t) => t[1] > 0);
  const summe = teile.reduce((s, t) => s + t[1], 0) || k.zeichen || 1;
  const fenster = i.nctx * 4;
  const breite = Math.max(summe, fenster);
  return h("div", null,
    h("div.budget", { stil: { height: gross ? "34px" : "26px" } }, teile.map((t, j) => h("i", { title: `${t[0]}: ${zahl(t[1])} Zeichen ≈ ${zahl(t[1] / 4)} Tokens`, stil: { width: (t[1] / breite * 100) + "%", background: TEIL_FARBEN[j % TEIL_FARBEN.length] } })),
      h("span.grenze", { title: `Kontextfenster ${zahl(i.nctx)} Tokens`, stil: { left: Math.min(99.5, fenster / breite * 100) + "%" } })),
    h("div.legende", null, teile.map((t, j) => h("span", null, h("u", { stil: { background: TEIL_FARBEN[j % TEIL_FARBEN.length] } }), `${t[0]} ${zahl(t[1] / 4)}`))),
    h("div.zeile", { stil: { marginTop: "8px" } },
      chip(`≈ ${zahl(k.tokens_geschaetzt)} Tokens`, i.voll > 1 ? "rot" : i.voll > 0.85 ? "bernstein" : "gruen"), chip(`Fenster ${zahl(i.nctx)}`), chip(`${k.fremdtexte} Fremdtexte isoliert`, "cyan"),
      k.gekuerzt?.length ? chip("gekürzt: " + k.gekuerzt.map(name).join(", "), "bernstein") : null, k.seele_version ? chip("Seele " + k.seele_version) : null,
      i.voll > 1 ? h("span.rot.klein", null, "Passt nicht ins Kontextfenster – Lage kürzen oder Fenster vergrößern.") : null));
}
function stufeKontext(r) {
  const k = kontextInfo(r).k;
  return h("div.spalte", null,
    karte("Kontextbudget", "Genau das geht in jeden Zug: Seele, Doktrin, Skills, Regeln, Antwortschema, Lage, Gedächtnis und Aufgaben. Die rote Linie ist das Kontextfenster des Modells.",
      kontextBudget(r, true),
      h("div.trenner"),
      h("div.formular", null,
        h("label", null, "Kontextfenster (Ollama)"), regler(S.auswahl.ollama_kontext, 4096, 65536, 1024, (x) => `${zahl(x)} Tokens`, (x) => setze("/ollama_kontext", x)),
        h("label", null, "Lage höchstens"), regler(S.auswahl.kontext_zeichen, 6000, 80000, 1000, (x) => `${zahl(x)} Zeichen`, (x) => setze("/kontext_zeichen", x))),
      h("div.zeile", { stil: { marginTop: "8px" } }, laeuft() ? knopf("Vorschau aktualisieren", () => harness({ befehl: "kontext", rolle: r }), "klein", "kontext") : null)),
    k ? h("div.raster.r2", null,
      karte("Systemprompt", `${zahl((k.system || "").length)} Zeichen`, h("pre.code", null, k.system || "")),
      karte("Lage und Gedächtnis", `${zahl((k.nutzer || "").length)} Zeichen`, h("pre.code", null, k.nutzer || ""))) : null);
}

function stufeModell(r) {
  const w = rolleW(r), pfad = `/rollen/${r}`, z = S.zustand, g = z.gpu || {};
  const anbieter = w.anbieter === "OpenRouter" ? "openrouter" : w.anbieter === "Eigen" ? "openai" : "ollama";
  const installiert = z.ollama || [];
  const empf = S.auswahl.abgeleitet?.klassen?.[r] || "mittel";
  return h("div.raster", { stil: { gridTemplateColumns: "minmax(0, 1.3fr) minmax(0, 1fr)" } },
    karte("Modell dieser Rolle", "Drei Wege: lokal mit Ollama, in der Cloud über OpenRouter oder über deinen eigenen, OpenAI-kompatiblen Server (LM Studio, vLLM, llama.cpp – auch selbst trainierte Modelle).",
      segment([["ollama", "Ollama (lokal)"], ["openrouter", "OpenRouter (Cloud)"], ["openai", "Eigener Server"]], anbieter, (v) => sende("anbieter", { rolle: r, wert: v }), true),
      h("div.formular", { stil: { marginTop: "12px" } },
        h("label", null, "Modell"), textfeld(pfad + "/modell", w.modell, false, anbieter === "openrouter" ? "z. B. anthropic/claude-sonnet-4.5" : "z. B. qwen3:8b"),
        h("label", null, "Ersatzmodell"), h("div.zeile.eng", null, h("select.feld", { stil: { maxWidth: "150px" }, value: w.ersatz_anbieter, onchange: (e) => setze(pfad + "/ersatz_anbieter", e.target.value) },
          [["Ollama", "Ollama"], ["OpenRouter", "OpenRouter"], ["Eigen", "Eigener Server"]].map(([v, t]) => h("option", { value: v, selected: w.ersatz_anbieter === v }, t))),
          h("div", { stil: { flex: "1" } }, textfeld(pfad + "/ersatz_modell", w.ersatz_modell, false, "leer = kein Ersatz"))),
        h("label", null, "Temperatur"), regler(w.temperatur, 0, 1.5, 0.05, (x) => x.toFixed(2), (x) => setze(pfad + "/temperatur", x)),
        h("label", null, "Antwortlänge"), regler(w.max_tokens, 256, 8192, 64, (x) => `${zahl(x)} Tokens`, (x) => setze(pfad + "/max_tokens", x)),
        h("label", null, "Modellklasse"), h("div.zeile.eng", null, S.katalog.modellklassen.map((k) => knopf(name(k.name) + (empf === k.name ? " ★" : ""), () => sende("modellklasse", { rolle: r, klasse: k.name }), empf === k.name ? "klein gold" : "klein")),
          h("span.winzig.matt", null, "★ Empfehlung für " + (w.modell || "–")))),
      anbieter === "openai" ? h("p.tipp", null, `Adresse des eigenen Servers: ${S.auswahl.eigen_url} (Werkstatt → Modelle). Eigenes Modell trainiert? Im Labor gibt es Trainingsdaten aus deinen erfolgreichen Zügen.`) : null),
    karte("Lokal installiert", g.name ? `${g.name} · ${zahl(g.vram_mb / 1024, 1)} GB VRAM (${zahl((g.belegt_mb || 0) / 1024, 1)} GB belegt)` : "Keine NVIDIA-Grafikkarte erkannt – Modelle laufen dann auf der CPU.",
      installiert.length ? h("div.liste", { stil: { maxHeight: "340px", overflow: "auto" } }, installiert.map((m) => {
        const passt = vramPasst(m.groesse), an = w.modell === m.name || w.modell + ":latest" === m.name;
        return h("div.eintrag.wahl" + (an ? ".an" : ""), { onclick: () => { setze(pfad + "/modell", m.name); if (anbieter !== "ollama") sende("anbieter", { rolle: r, wert: "ollama" }); } },
          h("div.text", null, h("b.mono", null, m.name), h("span.winzig.matt", null, gb(m.groesse) + " · Klasse " + name(m.klasse || "mittel"))),
          passt ? chip(passt, passt === "passt" ? "gruen" : passt === "knapp" ? "bernstein" : "rot") : null);
      })) : leer(z.ollama_fehler || "Ollama liefert keine Modelle."),
      h("div.zeile", { stil: { marginTop: "8px" } }, knopf("Neu laden", () => sende("ollama_laden"), "leise klein"))));
}

function stufeWerkzeuge(r) {
  const w = rolleW(r), pfad = `/rollen/${r}`;
  const quellen = S.auswahl.wissen || [];
  return h("div.raster.r2", null,
    karte("Werkzeuge", "Was die Rolle nachschlagen darf, bevor sie entscheidet. Gesperrte Werkzeuge verschwinden aus ihrem Antwortschema.",
      h("div.spalte", null, S.katalog.werkzeuge.map((t) => schalter((w.werkzeuge || []).includes(t.name), h("span", null, h("b", null, name(t.name)), " ", h("span.matt.klein", null, t.text)),
        (v) => setze(pfad + "/werkzeuge", v ? (w.werkzeuge || []).concat(t.name) : w.werkzeuge.filter((x) => x !== t.name)))))),
    karte("Wissensquellen dieser Rolle", "Datenbanken, eigenes RAG, MCP-Server – Antworten kommen als fremde Daten in Quarantäne.",
      quellen.length ? h("div.spalte", null, quellen.map((q, i) => schalter((q.rollen || []).includes(r), h("span", null, h("b", null, q.name), " ", h("span.matt.klein", null, q.art === "gedaechtnis" ? "SQL auf das Gedächtnis" : q.art === "http" ? "eigenes RAG" : "MCP-Server")),
        (v) => { setze(`/wissen/${i}/rollen`, v ? (q.rollen || []).concat(r) : q.rollen.filter((x) => x !== r)); if (v && !(w.werkzeuge || []).includes("wissen")) setze(pfad + "/werkzeuge", (w.werkzeuge || []).concat("wissen")); }))) : leer("Noch keine Quellen."),
      h("div.zeile", { stil: { marginTop: "10px" } }, knopf("Quellen einrichten", () => nav("wissen"), "klein", "wissen"))));
}

function stufeWachposten(r) {
  const au = S.auswahl.abgeleitet?.autonomie?.[r] || {}, re = S.auswahl.abgeleitet?.rechte?.[r] || {}, w = rolleW(r);
  const stufen = [["autopilot", "Autopilot"], ["vorsicht", "Vorsicht"], ["erlaubnis", "Erlaubnis"], ["gesperrt", "Gesperrt"]];
  return h("div.raster", { stil: { gridTemplateColumns: "minmax(0, 1fr) minmax(0, 1.2fr)" } },
    karte("Autonomie je Bereich", "Autopilot: handelt selbst · Vorsicht: Riskantes (Geschenke, Angriffe, Verträge …) fragt dich · Erlaubnis: alles fragt · Gesperrt: nie.",
      h("div.spalte", null, S.katalog.gruppen.filter((g) => au[g.name] && au[g.name] !== "–").map((g) => h("div", null, h("div.zeile", null, h("b", null, g.name), h("span.winzig.matt", null, g.text)),
        segment(stufen, au[g.name], (v) => sende("autonomie", { rolle: r, gruppe: g.name, stufe: v }), true), au[g.name] === "gemischt" ? h("span.winzig.bernstein", null, " gemischt – Details rechts") : null)))),
    karte("Rechte im Einzelnen", "Jede Aktion dieser Rolle. Gold umrandet: von dir festgelegt, sonst Grundregel.",
      h("div.rahmen", { stil: { maxHeight: "460px" } }, h("table.tab", null, h("tr", null, h("th", null, "Aktion"), h("th", null, "Recht")),
        Object.entries(re).map(([k, recht]) => { const eigen = (w.rechte || {})[k] !== undefined;
          return h("tr", null, h("td", null, h("span" + (eigen ? ".gold" : ""), null, k.startsWith("flotte_senden:") ? "Flotte → " + name(k.split(":")[1]) : name(k))),
            h("td", null, segment([["erlauben", "✔"], ["fragen", "?"], ["verbieten", "✖"]], recht, (v) => sende("recht", { rolle: r, kennung: k, recht: v })),
              eigen ? knopf("Grundregel", () => sende("recht", { rolle: r, kennung: k, recht: "standard" }), "leise klein") : null)); })))));
}

function probezugKarte(r) {
  const p = (S.daten?.probezuege || []).find((x) => x.rolle === r);
  if (!p) return leer(laeuft() ? "Noch kein Probezug für diese Rolle." : "Probezüge brauchen den laufenden Rat.");
  return h("div.spalte", null,
    h("div.klein", { stil: { fontStyle: "italic" } }, `„${p.begruendung || ""}“`),
    h("div.liste", null, (p.aktionen || []).map((a, i) => { const ok = p.probe?.[i];
      return h("div.eintrag", null, h("img.bildchen", { src: aktionBild(a), alt: "" }), h("div.text", null, h("b", null, aktionText(a)), ok ? h("span.winzig" + (ok.ok ? ".gruen" : ".rot"), null, (ok.ok ? "✔ " : "✖ ") + (ok.text || "")) : null)); }),
      (p.wartend || []).map((x) => h("div.eintrag.warnung", null, h("div.text", null, h("b", null, aktionText(x.aktion)), h("span.winzig.bernstein", null, "würde dich fragen: " + x.grund)))),
      (p.abgelehnt || []).map((x) => h("div.eintrag.gefahr", null, h("div.text", null, h("b", null, aktionText(x.aktion)), h("span.winzig.rot", null, "Wachposten: " + x.grund))))),
    (p.aktionen || []).length === 0 && !(p.wartend || []).length ? h("div.matt.klein", null, "Keine Aktion vorgeschlagen.") : null,
    h("div.zeile.winzig.matt", null, `${zahl(p.tokens)} Tokens · ${zahl(p.latenz_ms)} ms · ${p.modell} · Seele ${p.seele_version || "–"}`));
}
function stufeProbe(r) {
  const a = S.auswahl;
  return h("div.raster.r2", null,
    karte("Probe im Spielkern", "Vor jedem echten Zug prüft der Server die Aktionen ohne Wirkung. Abgelehnte Aktionen gehen mit Grund an das Modell zurück.",
      h("div.spalte", null, schalter(a.probe_vor_zug, "Probe vor jedem Zug", (v) => setze("/probe_vor_zug", v)),
        h("div.formular", null,
          h("label", null, "Korrekturrunden"), regler(a.korrekturrunden, 0, 3, 1, String, (x) => setze("/korrekturrunden", x)),
          h("label", null, "Aktionen je Zug"), regler(a.max_aktionen_je_zug, 1, 8, 1, String, (x) => setze("/max_aktionen_je_zug", x)),
          h("label", null, "Geschenke ohne Frage"), regler(a.geschenk_limit, 0, 100000, 500, (x) => `${zahl(x)} Cr/Tag`, (x) => setze("/geschenk_limit", x))))),
    karte("Probezug", "Die Rolle entscheidet jetzt – mit aktueller Seele, Skills und Rechten –, der Spielkern prüft, nichts wird gespielt.",
      laeuft() ? h("div.zeile", { stil: { marginBottom: "10px" } }, knopf("Probezug starten", () => harness({ befehl: "probezug", rolle: r }), "gold klein", "probe"), knopf("Im Labor vergleichen", () => nav("labor", { labor: { reiter: "arena" } }), "leise klein")) : null,
      probezugKarte(r)));
}
function stufeLernen(r) {
  const a = rolleA(r), w = rolleW(r);
  const zuege = (S.daten?.zuege || []).filter((z) => z.rolle === r).slice(0, 6);
  return h("div.raster.r2", null,
    karte("Bilanz", null, h("div.werte2", null,
      [["Züge", zahl(a.aufrufe || 0)], ["angenommen", zahl(a.angenommen || 0)], ["abgelehnt", zahl(a.abgelehnt || 0)], ["Tokens zuletzt", zahl(a.tokens || 0)], ["Latenz", `${zahl(a.latenz || 0)} ms`], ["Kosten", `${(a.kosten || 0).toFixed(4)} $`]]
        .map(([t, v]) => h("div.wert2", null, h("small", null, t), h("b", null, v)))),
      a.notiz ? h("div", { stil: { marginTop: "10px" } }, h("div.klein.gold", null, "Notizbuch"), h("div.klein", null, a.notiz)) : null),
    karte("Letzte Züge", null, zuege.length ? h("div.liste", null, zuege.map((z) => h("div.eintrag", null, h("div.text", null,
      h("b", null, `${z.quelle} · ${vor(z.zeit)}`), z.begruendung ? h("div.klein", { stil: { fontStyle: "italic" } }, `„${z.begruendung}“`) : null,
      (z.aktionen || []).map((x, i) => h("div.winzig" + (z.ergebnisse?.[i]?.ok ? ".gruen" : ".rot"), null, (z.ergebnisse?.[i]?.ok ? "✔ " : "✖ ") + aktionText(x))),
      (z.team_und_plan || []).map((x) => h("div.winzig.cyan", null, "☰ " + x.text)))))) : leer("Noch keine Züge.")));
}

function seiteRat() {
  const r = S.ui.rolle, w = rolleW(r), a = rolleA(r);
  const glb = `figuren/${r}.${gestalt(r)}.glb`;
  const detail = { identitaet: stufeIdentitaet, ausloeser: stufeAusloeser, wahrnehmung: stufeWahrnehmung, gedaechtnis: stufeGedaechtnis, kontext: stufeKontext, modell: stufeModell,
    werkzeuge: stufeWerkzeuge, wachposten: stufeWachposten, probe: stufeProbe, lernen: stufeLernen }[S.ui.stufe] || stufeIdentitaet;
  return h("div.seite", null,
    aenderungsleiste(),
    h("div.zeile", { stil: { marginBottom: "14px", gap: "10px" } }, ROLLEN.map((x) => h("div.eintrag.wahl" + (x === r ? ".an" : ""), { stil: { "--rolle": ROLLENFARBE[x], alignItems: "center", padding: "6px 12px 6px 6px" }, onclick: () => { S.ui.rolle = x; planen(); if (window.Stimmen) Stimmen.waehlen(x); } },
      h("img.ava", { src: Dreid.kopfbild(`figuren/${x}.${gestalt(x)}.glb`) || portraet(x), alt: "" }), h("div.text", null, h("b", null, ROLLENTITEL[x]), h("span.winzig.matt", null, rolleW(x).aktiv ? (rolleW(x).vorlage || "eigene Seele") : "nicht im Rat"))))),
    h("div.raster", { stil: { gridTemplateColumns: "340px minmax(0, 1fr)", marginBottom: "14px" } },
      h("div.spalte", null, Dreid.element(glb, portraet(r), 430, null, `figuren/${r}.${gestalt(r)}.webp`, { farbe: ROLLENFARBE[r], rolle: r }),
        window.Stimmen && Stimmen.gespraech ? Stimmen.gespraech(r) : null),
      h("div.spalte", null,
        h("section.karte", { stil: { "--rolle": ROLLENFARBE[r] } },
          h("div.zeile", null, h("span.punkt"), h("div", null, h("div", { stil: { font: "600 24px/1.2 var(--titel)" } }, `${ROLLENTITEL[r]} · ${w.vorlage || "eigene Seele"}`),
            h("div.matt", null, (S.katalog.rollen.find((x) => x.name === r) || {}).aufgabe)),
            h("span.rechts"), schalter(w.aktiv, w.aktiv ? "im Rat" : "nicht im Rat", (v) => setze(`/rollen/${r}/aktiv`, v))),
          h("div.zeile", { stil: { marginTop: "10px" } }, chip(w.modell || "kein Modell", "cyan"), chip(name(w.anbieter === "Eigen" ? "eigener Server" : w.anbieter || "")), a.seele_version ? chip("Seele " + a.seele_version) : null,
            chip(laeuft() ? (a.status || "wartet") : "Rat ruht", a.status === "denkt" ? "cyan" : a.status === "Fehler" ? "rot" : null), a.fehler ? h("span.rot.klein", null, a.fehler) : null),
          a.begruendung ? h("div.klein", { stil: { marginTop: "10px", fontStyle: "italic" } }, `„${a.begruendung}“`) : null,
          laeuft() && w.aktiv ? h("div.zeile", { stil: { marginTop: "12px" } },
            knopf("Jetzt ziehen", () => harness({ befehl: "jetzt", rolle: r }), "klein", "los"), knopf("Probezug", () => { harness({ befehl: "probezug", rolle: r }); S.ui.stufe = "probe"; planen(); }, "klein", "probe"),
            knopf("Kontext-Vorschau", () => { harness({ befehl: "kontext", rolle: r }); S.ui.stufe = "kontext"; planen(); }, "klein", "kontext"),
            knopf(a.aus ? "Fortsetzen" : "Anhalten", () => harness({ befehl: a.aus ? "rolle_an" : "rolle_aus", rolle: r }), "leise klein", a.aus ? "los" : "pause")) : null,
          laeuft() && w.aktiv ? h("div.zeile", { stil: { marginTop: "10px" } }, h("input.feld", { placeholder: "Hinweis für den nächsten Zug, z. B. „Baue zuerst Solarkraftwerke auf 2:15:4“", value: S.ui.hinweis?.[r] || "",
            oninput: (e) => { S.ui.hinweis = S.ui.hinweis || {}; S.ui.hinweis[r] = e.target.value; }, onkeydown: (e) => { if (e.key === "Enter" && e.target.value.trim()) { harness({ befehl: "hinweis", rolle: r, text: e.target.value.trim() }); S.ui.hinweis[r] = ""; planen(); } }, stil: { flex: "1" } }),
            knopf("Senden", () => { const t = (S.ui.hinweis?.[r] || "").trim(); if (t) { harness({ befehl: "hinweis", rolle: r, text: t }); S.ui.hinweis[r] = ""; planen(); } }, "klein")) : null),
        h("section.karte", { stil: { paddingBottom: "6px" } }, h("h2", null, "Harness dieser Rolle", h("span.rechts.matt.klein", null, "Stufe anklicken zum Einstellen · leuchtet, während der Zug dort ist")), schaltplan(r)))),
    detail(r));
}
