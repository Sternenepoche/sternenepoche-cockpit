/* Codex: das Nachschlagewerk – Spielbilder, Werte, Kosten je Volk, Stufenrechner, 3D-Rat, Auftrag an den Rat. */
"use strict";

const CODEX_ARTEN = [["schiffe", "Schiffe"], ["verteidigung", "Verteidigung"], ["gebaeude", "Gebäude"], ["forschung", "Forschung"], ["voelker", "Völker"], ["allianzen", "Allianzen"], ["rat", "Rat in 3D"], ["regeln", "Regeln"]];
const ROEMISCH = ["", "I", "II", "III", "IV", "V"];

function codexEintraege(art) {
  const c = S.katalog?.regeln?.catalog;
  if (!c) return [];
  const v = volk();
  switch (art) {
    case "allianzen": return (S.zustand?.codex_allianzen?.allianzen || []).map(a=>({k:String(a.id),titel:`[${a.tag}] ${a.name}`,unter:`${a.anzahl} Mitglieder · ${(a.voelker||[]).map(name).join(", ")}`,bild:bild("diplomacy.allianz"),daten:a}));
    case "schiffe": case "verteidigung":
      return Object.entries(c.einheiten || {}).filter(([, e]) => !!e.schiff === (art === "schiffe")).map(([k, e]) => ({ k, titel: name(k), unter: `Stufe ${ROEMISCH[e.regel.ab_stufe] || "?"} · Angriff ${zahl(e.regel.angriff)} · Schild ${zahl(e.regel.schild)}`,
        bild: bild(`${art === "schiffe" ? "ships" : "defenses"}.${k}.${v}`), daten: e }));
    case "gebaeude": return Object.entries(c.gebaeude || {}).map(([k, g]) => ({ k, titel: name(k), unter: `Stufe ${ROEMISCH[g.ab_stufe] || "?"} · ${g.wirkung}`, bild: bild(`buildings.${k}.${v}`), daten: g }));
    case "forschung": return Object.entries(c.forschung || {}).map(([k, g]) => ({ k, titel: name(k), unter: `Labor ${g.labor} · ${g.wirkung}`, bild: bild(`research.${k}`), daten: g }));
    case "voelker": return Object.entries(c.voelker || {}).map(([k, g]) => ({ k, titel: name(k), unter: Object.entries(g).filter(([, x]) => typeof x === "number" && Math.abs(x - 1) > 1e-6).map(([m, x]) => `${name(m)} ×${x}`).join(" · "), bild: bild(`portraits.${k}.${k}`), daten: g }));
    case "rat": return ["veyari", "aurelianer", "krath", "syntheten"].flatMap((vv) => ROLLEN.map((r) => ({ k: `${r}.${vv}`, titel: `${ROLLENTITEL[r]} der ${name(vv)}`, unter: (S.katalog.rollen.find((x) => x.name === r) || {}).aufgabe || "", bild: bild(`role_portraits.${r}.${vv}`), daten: { r, v: vv } })));
    case "regeln": return ["kolonisation", "ausscheiden", "kampf", "zonen", "welt", "wirtschaft", "agenten"].filter((k) => c[k] !== undefined).map((k) => ({ k, titel: name(k), unter: "Regelwerk", bild: bild(({kolonisation:`ships.kolonieschiff.${v}`,ausscheiden:"backgrounds.ranking",kampf:"backgrounds.hangar",zonen:"backgrounds.nebula",welt:"backgrounds.stars",wirtschaft:"backgrounds.market",agenten:`role_portraits.stratege.${v}`})[k]), daten: c[k] }));
  }
  return [];
}

function stufenkosten(basis, faktor, stufe) { const p = Math.pow(faktor || 1, Math.max(0, stufe - 1)); return Object.fromEntries(Object.entries(basis || {}).map(([g, x]) => [g, Math.floor(x * p)])); }
function kostenZeile(kosten) { return h("div.zeile.eng", null, Object.entries(kosten || {}).filter(([, x]) => x > 0).map(([g, x]) => h("span.chip", { title: name(g) }, h("img", { src: gutBild(g), alt: "" }), zahl(x), h("span.matt", null, " " + name(g))))); }

function auftragAnRat(titel, text, standardRolle) {
  S.ui.codexRolle = S.ui.codexRolle || standardRolle;
  if (!laeuft()) return h("p.tipp", null, "Mit ▶ LOS kannst du von hier aus Aufträge und Hinweise an den Rat geben.");
  return h("section.karte.hell", null, h("h2", null, "Auftrag an den Rat"),
    h("div.zeile", null, ROLLEN.map((r) => h("span", { stil: { cursor: "pointer", opacity: S.ui.codexRolle === r ? "1" : ".4" }, onclick: () => { S.ui.codexRolle = r; planen(); } }, avatar(r, 34))),
      h("span.rechts"), knopf("Als Aufgabe", () => harness({ befehl: "aufgabe_neu", titel: titel.slice(0, 120), beschreibung: text.slice(0, 800), rolle: S.ui.codexRolle, prioritaet: 2 }), "gold klein", "plus"),
      knopf("Als Hinweis", () => harness({ befehl: "hinweis", rolle: S.ui.codexRolle, text: (titel + ". " + text).slice(0, 1000) }), "klein")));
}

function codexDetail(art, e) {
  if (!e) return leer("Eintrag links wählen.");
  const d = e.daten, c = S.katalog.regeln.catalog;
  if (art === "allianzen") return h("div.spalte",null,h("h2.gold",null,e.titel),h("p",null,e.unter),h("p",null,`Leitung: ${d.leitung}`),h("p",null,`Beziehung: ${d.beziehung?.bezeichnung||"Neutral"}`),h("p.matt",null,"Allianzen stehen allen Völkern offen. Der Beitritt braucht eine Einladung und deine Annahme. Planetenzuordnungen siehst du im Atlas erst nach Aufklärung."));
  if (art === "rat") {
    const glb = `figuren/${d.r}.${d.v}.glb`;
    return h("div.spalte", null, h("div", { stil: { font: "600 26px var(--titel)", color: "var(--gold2)" } }, e.titel),
      h("div.raster.r2", null, Dreid.element(glb, bild(`role_portraits.${d.r}.${d.v}`), 520, null, `figuren/${d.r}.${d.v}.webp`, { farbe: ROLLENFARBE[d.r], rolle: d.r }),
        h("div.spalte", null, h("img", { src: bild(`role_portraits.${d.r}.${d.v}`), alt: "", stil: { width: "100%", borderRadius: "16px", border: "1px solid var(--linie)" } }),
          h("div.spalte", { stil: { gap: "4px" } }, h("b", null, `${ROLLENTITEL[d.r]} · ${name(d.v)}`), h("span.matt.klein", null, (S.katalog.rollen.find((x) => x.name === d.r) || {}).aufgabe || ""),
            h("span.matt.klein", null, `Im Rat: ${rolleW(d.r).aktiv ? (rolleW(d.r).vorlage || "eigene Seele") + " · " + (rolleW(d.r).modell || "kein Modell") : "nicht aktiv"}`)),
          knopf("Diesen Agenten im Rat öffnen", () => { S.ui.codex.offen = false; nav("rat", { rolle: d.r }); }, "klein", "rat"))));
  }
  if (art === "regeln") return h("div.spalte", null, h("h2.gold", null, e.titel), typeof d === "string" ? h("p", null, d) : h("table.tab", null, Object.entries(d).map(([k, x]) => h("tr", null, h("td.matt", null, name(k)), h("td", null, typeof x === "object" ? JSON.stringify(x) : String(x))))));
  if (art === "voelker") return h("div.spalte", null, h("div.held", null, h("div.portraet", null, h("img", { src: e.bild, alt: "" })),
    h("div", null, h("div", { stil: { font: "600 28px var(--titel)", color: "var(--gold2)" } }, e.titel), h("img", { src: bild(`emblems.${e.k}.${e.k}`), alt: "", stil: { width: "64px", margin: "8px 0" } }),
      h("div.werte2", null, Object.entries(d).map(([m, x]) => h("div.wert2", null, h("small", null, name(m)), h("b" + (typeof x === "number" && x !== 1 ? ".cyan" : ""), null, typeof x === "number" ? "×" + x : x === null ? "–" : String(x))))))),
    h("div.zeile", null, ROLLEN.map((r) => h("img", { src: bild(`role_portraits.${r}.${e.k}`), alt: "", title: ROLLENTITEL[r], stil: { width: "120px", borderRadius: "12px" } }))));
  if (art === "schiffe" || art === "verteidigung") {
    const g = d.regel, kosten = d.kosten_je_volk?.[volk()] || g.kosten;
    const verwundbar = Object.entries(c.einheiten).filter(([, x]) => x.regel.schnellfeuer?.[e.k]).map(([n, x]) => `${name(n)} ×${x.regel.schnellfeuer[e.k]}`);
    return h("div.spalte", null, h("div.held", null, h("div.portraet", null, h("img", { src: e.bild, alt: "" })),
      h("div.spalte", null, h("div", { stil: { font: "600 28px var(--titel)", color: "var(--gold2)" } }, e.titel), h("div.matt", { stil: { fontStyle: "italic" } }, g.wirkung),
        h("div.zeile.eng", null, chip(`ab Stufe ${ROEMISCH[g.ab_stufe]}`, "bernstein"), g.werft ? chip(`Werft ${g.werft}`, "bernstein") : null, Object.entries(g.braucht || {}).map(([b, s]) => chip(`${name(b)} ${s}`, "bernstein")), g.antrieb ? chip(name(g.antrieb), "cyan") : null),
        h("div.werte2", null, [["Angriff", g.angriff], ["Schild", g.schild], ["Struktur", g.struktur], ["Tempo", g.tempo], ["Ladung", g.ladung], ["Verbrauch", g.verbrauch], ["Besatzung", g.besatzung]].map(([t, x]) => h("div.wert2", null, h("small", null, t), h("b", null, zahl(x))))),
        h("div", null, h("div.klein.gold", null, `Kosten (${name(volk())})`), kostenZeile(kosten)),
        Object.keys(g.schnellfeuer || {}).length ? h("div.klein", null, h("span.bernstein", null, "Schnellfeuer: "), Object.entries(g.schnellfeuer).map(([z, n]) => `${name(z)} ×${n}`).join(", ")) : null,
        verwundbar.length ? h("div.klein.matt", null, "Verwundbar durch: " + verwundbar.join(", ")) : null)),
      h("div.zeile", null, ["veyari", "aurelianer", "krath", "syntheten"].map((vv) => h("img", { src: bild(`${art === "schiffe" ? "ships" : "defenses"}.${e.k}.${vv}`), alt: "", title: name(vv), stil: { width: "110px", borderRadius: "12px", border: vv === volk() ? "1px solid var(--gold)" : "1px solid var(--linie)" } }))),
      auftragAnRat(`${e.titel} ${art === "schiffe" ? "fertigen" : "bauen"}`, `Kosten je Stück: ${Object.entries(kosten).map(([gg, x]) => `${zahl(x)} ${name(gg)}`).join(", ")}. Voraussetzung: Stufe ${ROEMISCH[g.ab_stufe]}, Werft ${g.werft}.`, "feldherr"));
  }
  const stufe = S.ui.codexStufe || 1, max = c.welt?.max_gebaeudestufe || 40;
  if (art === "gebaeude") {
    const k = stufenkosten(d.kosten, d.faktor, stufe);
    const planeten = S.daten?.inventur?.planeten || [];
    return h("div.spalte", null, h("div.held", null, h("div.portraet", null, h("img", { src: e.bild, alt: "" })),
      h("div.spalte", null, h("div", { stil: { font: "600 28px var(--titel)", color: "var(--gold2)" } }, e.titel), h("div.matt", { stil: { fontStyle: "italic" } }, d.wirkung),
        h("div.zeile.eng", null, chip(`ab Stufe ${ROEMISCH[d.ab_stufe]}`, "bernstein"), Object.entries(d.braucht || {}).map(([b, s]) => chip(`${name(b)} ${s}`, "bernstein"))),
        h("div.werte2", null, [["Strom (Grund)", d.energie], ["Arbeiter", d.arbeiter], ["Fachkräfte", d.fachkraefte], ["Ertrag (Grund)", d.ertrag], ["Kostenfaktor", d.faktor]].map(([t, x]) => h("div.wert2", null, h("small", null, t), h("b", null, zahl(x, 2))))),
        h("div", null, h("div.klein.gold", null, `Kosten für Stufe ${stufe}`), regler(stufe, 1, max, 1, (x) => `Stufe ${x}`, (x) => { S.ui.codexStufe = x; planen(); }), kostenZeile(k)))),
      planeten.length ? karte("Auf deinen Planeten", null, h("div.liste", null, planeten.map((p) => { const jetzt_ = p.gebaeude?.[e.k] || 0, baubar = (p.baubar_jetzt || []).some((b) => b.startsWith(e.k + " "));
        return h("div.eintrag", null, h("img.bildchen", { src: planetBild(p.zone), alt: "" }), h("div.text", null, h("b.mono", null, p.koord), h("span.winzig.matt", null, `Stufe ${jetzt_}`)), baubar ? chip("jetzt baubar", "gruen") : null); }))) : null,
      auftragAnRat(`${e.titel} ausbauen`, `Wirkung: ${d.wirkung}. Kosten Stufe ${stufe}: ${Object.entries(k).filter(([, x]) => x > 0).map(([g, x]) => `${zahl(x)} ${name(g)}`).join(", ")}.`, "verwalter"));
  }
  if (art === "forschung") {
    const k = stufenkosten(d.kosten, d.faktor, stufe), stand = S.zustand.lage?.forschung?.stufen?.[e.k];
    return h("div.spalte", null, h("div.held", null, h("div.portraet", null, h("img", { src: e.bild, alt: "" })),
      h("div.spalte", null, h("div", { stil: { font: "600 28px var(--titel)", color: "var(--gold2)" } }, e.titel), h("div.matt", { stil: { fontStyle: "italic" } }, d.wirkung),
        h("div.zeile.eng", null, chip(`ab Stufe ${ROEMISCH[d.ab_stufe]}`, "bernstein"), chip(`Labor ${d.labor}`, "bernstein"), stand !== undefined ? chip(`dein Stand: Stufe ${stand}`, "gruen") : null),
        h("div", null, h("div.klein.gold", null, `Kosten für Stufe ${stufe}`), regler(stufe, 1, 30, 1, (x) => `Stufe ${x}`, (x) => { S.ui.codexStufe = x; planen(); }), kostenZeile(k),
          h("div.winzig.matt", null, `${zahl(Math.floor((d.fp || 0) * Math.pow(d.faktor || 1, stufe - 1)))} Forschungspunkte`)))),
      auftragAnRat(`${e.titel} erforschen`, `Wirkung: ${d.wirkung}. Braucht Labor ${d.labor}.`, "verwalter"));
  }
  return leer("–");
}

function seiteCodex() {
  const cx = S.ui.codex;
  const scope=S.zustand?.lokal?.url+":"+S.zustand?.anmeldung?.angemeldet;
  if(cx.offen&&cx.art==="allianzen"&&S.zustand?.anmeldung?.angemeldet&&cx.allianzScope!==scope){cx.allianzScope=scope;sende("codex_allianzen");}
  const alle = codexEintraege(cx.art);
  const suche = (cx.suche || "").toLowerCase();
  const liste = alle.filter((e) => !suche || (e.titel + " " + e.unter + " " + JSON.stringify(e.daten)).toLowerCase().includes(suche));
  const wahl = liste.find((e) => e.k === cx.wahl) || liste[0];
  return h("aside.codex" + (cx.offen ? ".offen" : ""), null,
    h("div.links", null,
      h("div.kopfzeile", null, h("div.zeile", null, h("b.gold", { stil: { letterSpacing: ".2em", fontSize: "13px" } }, "CODEX"), h("span.rechts"), knopf("", () => { cx.offen = false; planen(); }, "leise klein", "kreuz")),
        h("div.segment.gold", null, CODEX_ARTEN.map(([k, t]) => h("button" + (cx.art === k ? ".an" : ""), { type: "button", onclick: () => { cx.art = k; cx.wahl = null; planen(); } }, t))),
        h("input.feld", { placeholder: cx.art==="allianzen"?"Allianz-Kürzel oder Name …":"Suchen …", value: cx.suche, oninput: (e) => { cx.suche = e.target.value; planen(); } }),
        cx.art==="allianzen"?knopf("Verzeichnis aktualisieren",()=>sende("codex_allianzen"),"leise klein",null,!S.zustand?.anmeldung?.angemeldet):null),
      h("div.liste", null, S.katalog?.regeln ? liste.map((e) => h("div.eintrag.wahl" + (wahl && wahl.k === e.k ? ".an" : ""), { onclick: () => { cx.wahl = e.k; planen(); } },
        h("img.bildchen", { src: e.bild, alt: "", loading: "lazy" }), h("div.text", null, h("b", null, e.titel), h("span.winzig.matt", null, e.unter)))) : leer(S.katalog?.regeln_fehler || "Regelkatalog wird vom Server geladen …"))),
    h("div.detail", null, cx.offen && S.katalog?.regeln ? codexDetail(cx.art, wahl) : null));
}
