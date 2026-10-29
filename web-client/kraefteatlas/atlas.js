'use strict';
// Kräfteatlas als Modul – erzeugt von tools/konter_matrix/uebersicht.py aus uebersicht_vorlage.html. Nicht von Hand ändern.
// Start: KraefteatlasStart({wurzel, basis, eingebettet, vollbild, start, schliessen}) → {zurueck, zeige, reiter}.
window.KraefteatlasStart = (OPT = {}) => {
const wurzel = OPT.wurzel || document, basis = OPT.basis || '';
const MARKUP = "\n<div id=\"kulisse\" aria-hidden=\"true\"><div class=\"nebel\"></div><div class=\"schleier\"></div><div class=\"s s1\"></div><div class=\"s s2\"></div><div class=\"s s3\"></div><div class=\"sonne\"></div>\n<div class=\"kugel mond\"><div class=\"flaeche\" id=\"k-mond\"></div></div><div class=\"kugel planet\"><div class=\"flaeche\" id=\"k-planet\"></div></div><div class=\"schnuppe\"></div><div class=\"schnuppe b\"></div></div>\n<div class=\"wrap\">\n<div class=\"ko kopf\">\n<div class=\"leiste\"><span class=\"led\">Tech-Datenbank · Kräfteatlas · Stand 0.4.8</span>\n<span class=\"wahl\"><span class=\"lbl\">Dein Volk</span><select id=\"volk\" aria-label=\"Dein Volk: Motive, Werte und Lore\"><option value=\"aurelianer\">Aurelianer</option><option value=\"krath\">Krath</option><option value=\"syntheten\" selected>Syntheten</option><option value=\"veyari\">Veyari</option></select></span></div>\n<div class=\"ko-inhalt\">\n<div class=\"buehne\" id=\"kopf-buehne\"><div class=\"nebel\"></div><div class=\"horizont\"><div class=\"flaeche\" id=\"k-horizont\"></div></div><div class=\"flotte\" id=\"kopf-bilder\"></div><span class=\"abtasten\"></span>\n<div class=\"titel\"><p class=\"augenbraue\">Konter-Analyse aller Kampfeinheiten · echte Kampfengine</p><h1>Wer schlägt wen – und warum</h1>\n<span class=\"pille\" id=\"kopf-meta\"></span></div></div>\n<p class=\"lead\">Alle 20 Kampfschiffe, 5 Verteidigungsanlagen, 4 Angriffsraketen, 10 Bodentruppen und der Weltenkrümmer mit den Werten, die im Spiel gelten –\njede Einheit gegen jede, mit genau gleich viel Ressourcen auf beiden Seiten. <b>Alles lässt sich antippen:</b> Kachel, Zelle, Linie oder Chip öffnet das Gefecht\nim Detail, mit Verlauf Runde für Runde, Feuer beider Seiten und der Zielmechanik dahinter.</p>\n<div class=\"kpis\" id=\"kpis\"></div>\n</div></div>\n\n<nav class=\"reiter\" role=\"tablist\" aria-label=\"Bereiche\" id=\"reiter\">\n<button role=\"tab\" data-ziel=\"ueberblick\">Überblick</button><button role=\"tab\" data-ziel=\"voelker\">Volk gegen Volk</button><button role=\"tab\" data-ziel=\"flotte\">Flotte &amp; Anlagen</button>\n<button role=\"tab\" data-ziel=\"verteidigung\">Verteidigung &amp; Überfälle</button><button role=\"tab\" data-ziel=\"raketen\">Raketen</button>\n<button role=\"tab\" data-ziel=\"boden\">Bodentruppen</button><button role=\"tab\" data-ziel=\"wk\">Weltenkrümmer</button><button role=\"tab\" data-ziel=\"methode\">Methode</button>\n<span class=\"zeiger\" id=\"zeiger\"></span>\n</nav>\n\n<main>\n<section data-tab=\"ueberblick\">\n<div class=\"ko\"><div class=\"leiste\"><span class=\"led\">Lesehilfe – zuerst lesen</span></div><div class=\"ko-inhalt\">\n<h2>Was bedeuten ×2, /3 und „klarer Konter“?</h2>\n<p class=\"lead\">Jedes Gefecht im Atlas läuft gleich ab: <b>Beide Seiten setzen genau gleich viel Ressourcen ein</b> (je <span class=\"budget\"></span> Werteinheiten), beide mit\nTechnik 6, höchstens 6 Runden. Danach wird verglichen: <b>Wie viel Wert hat die Einheit zerstört – und wie viel hat sie dafür selbst verloren?</b></p>\n<div class=\"lese\" id=\"lese\"></div>\n<div class=\"stufen\">\n<div class=\"karte gruen\"><h4>Klarer Konter</h4><p>Ab ×2 – und der Verlierer büßt mindestens ein Viertel seiner Flotte ein. Darauf ist Verlass: Diese Einheit baust du gezielt gegen diesen Gegner.</p></div>\n<div class=\"karte\"><h4>Knapper Vorteil</h4><p>Ab ×1,4 – spürbar, aber nicht sicher. Mehr Stückzahl, bessere Technik oder eine gemischte Flotte des Gegners können ihn kippen.</p></div>\n<div class=\"karte violett\"><h4>Offen</h4><p>Zwischen /1,4 und ×1,4 – keiner hat einen Vorteil, der Rest entscheidet.</p></div>\n</div>\n<p class=\"leise\">Für dich heißt das: <b class=\"gut\">Grün</b> zeigt, wofür du eine Einheit baust. <b class=\"schlecht\">Rot</b> zeigt, welche Gegner du mit ihr meidest – oder gegen die du ihren Konter mitbringst.</p>\n</div></div>\n\n<div class=\"ko\"><div class=\"leiste\"><span class=\"led\">Konter-Netz</span></div><div class=\"ko-inhalt\">\n<h2>Wer wen schlägt – Einheit für Einheit</h2>\n<p class=\"lead\">Kein starres „Klasse schlägt Klasse“: Jede Einheit hat eigene Opfer und eigene Konter, oft quer durch die Klassen. Tippe im Rad eine Einheit an.\n<b class=\"gut\">Grüne Linien</b> führen zu Einheiten, die sie schlägt, <b class=\"schlecht\">rote</b> zu denen, die sie schlagen – je dicker, desto deutlicher. Eine Linie öffnet das Gefecht.</p>\n<div class=\"leistezeile\"><span class=\"wahl\"><span class=\"lbl\">Einheit</span><select id=\"rad-wahl\" aria-label=\"Einheit wählen\"></select></span>\n<span class=\"wahl\" id=\"rad-stufe\"><span class=\"lbl\">Zeigen</span><button data-w=\"hart\">nur klare Konter</button><button data-w=\"weich\">auch knappe Vorteile</button></span></div>\n<p class=\"hilfe\" id=\"rad-hilfe\"></p>\n<div class=\"rad\"><div id=\"rad-svg\"></div><div id=\"rad-info\"></div></div>\n</div></div>\n\n<div class=\"ko\"><div class=\"leiste\"><span class=\"led\">Klassentendenz</span></div><div class=\"ko-inhalt\">\n<h2>Klasse gegen Klasse im Mittel</h2>\n<p class=\"lead\">Mittel aller Einzelgefechte zwischen zwei Klassen – Zeile gegen Spalte. Die Mittel sind bewusst mäßig: Die klaren Konter sitzen bei einzelnen Spezialisten,\ndie die Sätze darunter nennen. Tippe eine Zelle für alle Einzelgefechte.</p>\n<div id=\"klassen\"></div><p class=\"leg\" id=\"leg-klassen\"></p>\n<div class=\"karten raster\" id=\"tendenz\" data-st=\"margin-top:14px\"></div>\n</div></div>\n\n<div class=\"ko\"><div class=\"leiste\"><span class=\"led\">Belagerung</span></div><div class=\"ko-inhalt\">\n<h2>Jeder Belagerer bricht andere Anlagen</h2>\n<p class=\"lead\">Bei gleichem Wert gegen jede Waffenanlage einzeln. Kein Belagerer knackt alles – wer eine gemischte Verteidigung brechen will, braucht die passende Mischung.</p>\n<div id=\"belagerung\"></div>\n<div class=\"karten raster\" id=\"belagerung-text\" data-st=\"margin-top:14px\"></div>\n</div></div>\n\n<div class=\"ko\"><div class=\"leiste\"><span class=\"led\">Kampfregeln</span></div><div class=\"ko-inhalt\">\n<h2>Wie ein Gefecht abläuft</h2>\n<div class=\"karten raster\" id=\"regeln\"></div>\n</div></div>\n</section>\n\n<section data-tab=\"voelker\">\n<div class=\"ko\"><div class=\"leiste\"><span class=\"led\">Volk gegen Volk</span></div><div class=\"ko-inhalt\">\n<h2>Dein Volk gegen das Gegnervolk</h2>\n<p class=\"lead\">Links wählst du dein Volk, rechts das Gegnervolk. Jede Paarung ist mit der echten Kampfengine gerechnet – mit den Kampffaktoren der Völker\n(Waffen, Panzerung) wie im Spiel, je <span class=\"budget\"></span> Werteinheiten, Technik 6. Die Motive zeigen die Schiffe beider Völker. Tippe eine Zelle für das Gefecht im Detail.</p>\n<div class=\"vk-kopf\">\n<div class=\"vk-seite\"><h3>Dein Volk</h3><div class=\"vk-wahl\" id=\"vk-wahl-l\"></div><div class=\"vk-merkmale\" id=\"vk-merk-l\"></div></div>\n<div class=\"vk-buehne\" id=\"vk-buehne\"></div>\n<div class=\"vk-seite\"><h3>Gegnervolk</h3><div class=\"vk-wahl\" id=\"vk-wahl-r\"></div><div class=\"vk-merkmale\" id=\"vk-merk-r\"></div></div>\n</div>\n<div class=\"karten raster\" id=\"vk-effekt\" data-st=\"margin-top:16px\"></div>\n</div></div>\n<div class=\"ko\"><div class=\"leiste\"><span class=\"led\">Klassenmatrix · Volk gegen Volk</span></div><div class=\"ko-inhalt\">\n<h2 id=\"vk-klassen-titel\"></h2>\n<p class=\"lead\">Mittel aller Einzelgefechte: Zeilen sind die Klassen deines Volks, Spalten die des Gegnervolks. Darunter klein: wie sich das Mittel durch die Völker gegenüber dem Gleichstand\nohne Volksboni verschiebt (▲ besser für dich, ▼ schlechter). Tippe eine Zelle für alle Einzelgefechte.</p>\n<div id=\"vk-klassen\"></div><p class=\"leg\" id=\"leg-vk-klassen\"></p>\n</div></div>\n<div class=\"ko\"><div class=\"leiste\"><span class=\"led\">Einheitenmatrix · Volk gegen Volk</span></div><div class=\"ko-inhalt\">\n<h2>Jede Einheit gegen jede</h2>\n<div class=\"leistezeile\"><span class=\"wahl\" id=\"vk-zahl\"><span class=\"lbl\">Zahl</span><button data-w=\"tausch\">Tausch mit Volksboni</button><button data-w=\"delta\">Volkseffekt</button></span></div>\n<div class=\"hm-scroll\" id=\"vk-matrix\"></div><p class=\"leg\" id=\"leg-vk-matrix\"></p>\n</div></div>\n<div class=\"ko\"><div class=\"leiste\"><span class=\"led\">Raketensilo · Volk gegen Volk</span></div><div class=\"ko-inhalt\">\n<h2>Raketen deines Volks gegen das Gegnervolk</h2>\n<p class=\"lead\">Raketenschaden wächst mit dem Waffenfaktor deines Volks, die Struktur der Ziele mit dem Panzerungsfaktor des Gegnervolks. Zahl = zerstörter Gegnerwert je eigenem Raketenwert\n(1 = so viel, wie die Rakete kostet), dahinter der Unterschied zum Gleichstand.</p>\n<div id=\"vk-raketen\"></div>\n</div></div>\n</section>\n\n<section data-tab=\"flotte\">\n<div class=\"ko\"><div class=\"leiste\"><span class=\"led\">Schiffswerft · Verteidigung</span></div><div class=\"ko-inhalt\">\n<h2>Alle Kampfeinheiten</h2>\n<p class=\"lead\">Tippe eine Einheit für ihr Dossier: Werte, Ziele, wen sie schlägt, wer sie schlägt, und alle Gefechte einzeln. Unter jedem Motiv: <b class=\"gut\">▲</b> wie viele Einheiten sie klar schlägt, <b class=\"schlecht\">▼</b> von wie vielen sie klar geschlagen wird.</p>\n<div id=\"kacheln-flotte\"></div>\n</div></div>\n<div class=\"ko\"><div class=\"leiste\"><span class=\"led\">Gefechtsmatrix</span></div><div class=\"ko-inhalt\">\n<h2>Jede gegen jede</h2>\n<p class=\"lead\">Jede Zeile ist eine Einheit, jede Spalte ihr Gegner – je <span class=\"budget\"></span> Werteinheiten auf beiden Seiten, Technik 6, ohne Volksboni (mit Völkern: Reiter „Volk gegen Volk“). Die Zahl sagt, wie die Zeile tauscht.\nTippe eine Zelle für das Gefecht, ein Motiv für das Dossier. Bei Schiffen feuern beide Seiten gleichzeitig; bei Anlagen greift immer das Schiff an.</p>\n<div class=\"leistezeile\">\n<span class=\"wahl\" id=\"mx-kennzahl\"><span class=\"lbl\">Zahl</span><button data-w=\"tausch\">Tausch</button><button data-w=\"sieg\">Siegquote</button><button data-w=\"verlust\">eigener Verlust</button><button data-w=\"gegner\">Gegnerverlust</button></span>\n<span class=\"wahl\" id=\"mx-ordnung\"><span class=\"lbl\">Reihenfolge</span><button data-w=\"klasse\">nach Klasse</button><button data-w=\"staerke\">nach Stärke</button></span>\n<span class=\"wahl\" id=\"mx-hart\"><span class=\"lbl\">Zeigen</span><button data-w=\"alle\">alle</button><button data-w=\"hart\">nur klare Konter</button></span>\n<span class=\"wahl\"><span class=\"lbl\">Hervorheben</span><select id=\"mx-fokus\" aria-label=\"Einheit hervorheben\"></select></span></div>\n<div class=\"hm-scroll\" id=\"matrix\"></div>\n<p class=\"leg\" id=\"leg-matrix\"></p>\n</div></div>\n<div class=\"ko\"><div class=\"leiste\"><span class=\"led\">Duell-Bilanz</span></div><div class=\"ko-inhalt\">\n<h2>Wen schlägt jedes Schiff, wer schlägt es?</h2>\n<p class=\"lead\">Jedes Schiff tritt einzeln gegen jedes andere Kampfschiff an, beide Seiten mit dem gleichen Ressourcenwert. <b>Ein Kästchen ist ein Gegner.</b>\nGrüne Kästchen rechts der goldenen Mittellinie sind gewonnene Duelle, rote links davon verlorene, graue in der Mitte gehen offen aus.\nJe weiter die Reihe nach rechts reicht, desto mehr Gegner schlägt das Schiff; je weiter nach links, desto mehr Konter hat es.\nRechts daneben die fünf Waffenanlagen: Gewöhnliche Schiffe zielen nicht auf Anlagen und verlieren gegen gleich teure Verteidigung, das ist gewollt –\nAnlagen bricht man mit den Belagerern. Die drei Belagerer zählen links nicht als Gegner, weil sie im Flottenkampf absichtlich schwach sind.\nJedes Kästchen öffnet das Gefecht im Detail, der Name das Dossier.</p>\n<div class=\"bz-legende\" id=\"bz-legende\"></div>\n<div class=\"leistezeile\"><span class=\"wahl\" id=\"bz-ordnung\"><span class=\"lbl\">Reihenfolge</span><button data-w=\"klasse\">nach Klasse</button><button data-w=\"bilanz\">nach Bilanz</button></span></div>\n<div class=\"bilanz\" id=\"profil-schiffe\"></div>\n</div></div>\n<div class=\"ko\"><div class=\"leiste\"><span class=\"led\">Dossiers</span></div><div class=\"ko-inhalt\">\n<h2>Kampfschiffe im Einzelnen</h2>\n<div id=\"karten-schiffe\"></div>\n</div></div>\n</section>\n\n<section data-tab=\"verteidigung\">\n<div class=\"ko\"><div class=\"leiste\"><span class=\"led\">Planetenschutz</span></div><div class=\"ko-inhalt\">\n<h2>Verteidigungsanlagen</h2>\n<p class=\"lead\">Anlagen zielen nur auf Schiffe. Gegen gewöhnliche Schiffe halten sie bei gleichem Wert deutlich stand, denn Schiffe treffen Anlagen nur als Reserve mit Grundschaden.\nBrechen lassen sie sich von den passenden Belagerern – und von großer Übermacht.</p>\n<div id=\"kacheln-vert\"></div>\n<h3 class=\"block-h\">Anlage gegen Schiffsklasse <small>Ø Tausch, Zelle antippen für die Gefechte</small></h3>\n<div id=\"vert-klassen\"></div>\n<h3 class=\"block-h\">Belagerer gegen Anlagen <small>jeder Belagerer bricht andere Anlagen</small></h3>\n<div id=\"belagerung2\"></div>\n</div></div>\n<div class=\"ko\"><div class=\"leiste\"><span class=\"led\">Abschreckung</span></div><div class=\"ko-inhalt\">\n<h2>Was kostet ein Überfall den Räuber?</h2>\n<p class=\"lead\">Gemischte Raubflotte wie in den Bot-Welten (Wertanteile Schwerer Jäger 55 %, Kreuzer 30 %, Leichter Jäger 15 %) gegen eine Verteidigung im Wert von\n<span id=\"vwert\"></span>, mit dem gleichen, dreifachen und zehnfachen Wert. Große Zahl = <b>Abschreckung</b>: zerstörter Räuberwert ÷ Wert der Verteidigung – ab 1 verliert der Räuber\nmehr, als die Verteidigung wert ist. Darunter: wie oft der Räuber gewinnt. Tippe eine Kachel für Aufstellung, Verlauf und „wer schießt auf wen“.</p>\n<div class=\"ugrid\" id=\"ueberfall\"></div>\n<p class=\"leg\"><span><span class=\"sw\" data-st=\"background:var(--gold)\"></span>Abschreckung, heller Strich = 1</span><span><span class=\"sw\" data-st=\"background:var(--sa)\"></span>Anteil der Läufe, die der Räuber gewinnt</span></p>\n<div class=\"karte gold\" data-st=\"margin-top:12px\"><h4>Was das für dich heißt</h4><p id=\"ueberfall-text\"></p></div>\n<h3 class=\"block-h\">Bot-Welten <small>30 Bots, 30 Spieltage, frische Welt mit diesen Regeln</small></h3>\n<div class=\"tbl-wrap\"><table class=\"tbl\" id=\"bots\"></table></div>\n<p class=\"leise\">Abschreckung hier = Räuberverlust ÷ Kampfwert der Verteidiger über alle Überfälle auf Planeten mit Anlagen. Bots greifen nur an, wenn sie zu mindestens 75 % siegen\nund die Beute den doppelten Verlust übersteigt – darum liegt der Wert unter dem der gezielten Überfälle oben.</p>\n</div></div>\n</section>\n\n<section data-tab=\"raketen\">\n<div class=\"ko\"><div class=\"leiste\"><span class=\"led\">Raketensilo</span></div><div class=\"ko-inhalt\">\n<h2>Angriffsraketen</h2>\n<p class=\"lead\">Raketen kämpfen nicht in Runden: Sie treffen eine vor dem Start gewählte Zielklasse, umgehen Schilde und zerstören Schaden × Faktor ÷ Struktur Stück, höchstens so viele\nZiele je Rakete wie angegeben. Waffen- und Panzerungstechnik wirken auf beiden Seiten gleich, darum gelten die Zahlen für jede gleiche Technikstufe. Balken = zerstörter Gegnerwert\nje eigenem Raketenwert, der helle Strich ist 1 (so viel, wie die Rakete kostet). Tippe eine Zeile für die Rechnung.</p>\n<div id=\"raketen\"></div>\n</div></div>\n</section>\n\n<section data-tab=\"boden\">\n<div class=\"ko\"><div class=\"leiste\"><span class=\"led\">Kaserne</span></div><div class=\"ko-inhalt\">\n<h2>Bodentruppen</h2>\n<p class=\"lead\">Im Bodenkampf feuern beide Seiten gleichzeitig; jeder Typ schießt geschlossen auf den vorhandenen Gegnertyp mit dem höchsten Konterfaktor. Eine Welle dauert höchstens\n<span id=\"b-runden\"></span> Runden, Verwundete heilen bis zur nächsten Welle. Je Seite <span id=\"b-budget\"></span> Werteinheiten, gerechnet mit den Kernfunktionen des Spiels – exakt, ohne Zufall.</p>\n<div id=\"kacheln-boden\"></div>\n<div class=\"leistezeile\"><span class=\"wahl\" id=\"b-modus\"><span class=\"lbl\">Ergebnis</span><button data-w=\"ende\">bis Kampfende</button><button data-w=\"welle\">eine Welle</button></span></div>\n<div class=\"hm-scroll\" id=\"b-matrix\"></div>\n<p class=\"leg\" id=\"leg-boden\"></p>\n<h3 class=\"block-h\">Duell-Bilanz <small id=\"b-bilanz-modus\">bis Kampfende</small></h3>\n<p class=\"leg\" data-st=\"margin:0 0 8px\">Ein Kästchen ist ein gegnerischer Truppentyp mit gleichem Wert: grün rechts der Mittellinie gewonnen, rot links verloren, grau offen. Der Umschalter über der Matrix wechselt zwischen Kampfende und einer Welle.</p>\n<div class=\"bilanz\" id=\"profil-boden\"></div>\n<h3 class=\"block-h\">Unterstützer <small>ohne eigene Zeile in der Matrix</small></h3>\n<div class=\"karten\" id=\"b-hilfe\"></div>\n</div></div>\n</section>\n\n<section data-tab=\"wk\">\n<div class=\"ko\"><div class=\"leiste\"><span class=\"led\">Mondwerft · Megaschiff</span></div><div class=\"ko-inhalt\">\n<div id=\"wk-info\"></div>\n<h3 class=\"block-h\">Ein Weltenkrümmer gegen den gleichen Wert <small>Zeile antippen für das Gefecht</small></h3>\n<div class=\"tbl-wrap\"><table class=\"tbl\" id=\"wk\"></table></div>\n</div></div>\n</section>\n\n<section data-tab=\"methode\">\n<div class=\"ko\"><div class=\"leiste\"><span class=\"led\">Messung</span></div><div class=\"ko-inhalt\">\n<h2>Methode</h2>\n<div class=\"karten raster\" id=\"methode\"></div>\n</div></div>\n</section>\n</main>\n</div>\n<div class=\"schleier\" id=\"schleier\"></div>\n<aside class=\"blatt\" id=\"detail\" role=\"dialog\" aria-modal=\"false\" aria-labelledby=\"d-leiste\" aria-hidden=\"true\">\n<div class=\"bk\"><button class=\"rund\" id=\"d-zurueck\" aria-label=\"Zurück\" hidden>←</button><div class=\"leiste\"><span class=\"led\" id=\"d-leiste\"></span></div><button class=\"rund\" id=\"d-zu\" aria-label=\"Schließen\">✕</button></div>\n<div class=\"bi\" id=\"d-inhalt\"></div>\n</aside>\n<div id=\"tip\" role=\"tooltip\"></div>\n";
// Inline-Styles sind im Spiel per CSP verboten: Vorlagen tragen data-st, hier wird es über element.style gesetzt.
const stileSetzen = el => { for (const teil of el.getAttribute('data-st').split(';')) { const i = teil.indexOf(':'); if (i > 0) el.style.setProperty(teil.slice(0, i).trim(), teil.slice(i + 1).trim()); } el.removeAttribute('data-st'); };
const stileUnter = el => { if (el.hasAttribute && el.hasAttribute('data-st')) stileSetzen(el); if (el.querySelectorAll) for (const x of el.querySelectorAll('[data-st]')) stileSetzen(x); };
const seite = document.createElement('div'); seite.className = 'atlas-seite' + (OPT.vollbild ? ' vollbild' : '') + (OPT.schliessen ? ' mit-zu' : '');
new MutationObserver(ms => { for (const m of ms) for (const n of m.addedNodes) if (n.nodeType === 1) stileUnter(n); }).observe(seite, {childList: true, subtree: true});
seite.innerHTML = MARKUP; stileUnter(seite);
if (OPT.schliessen) { const zu = document.createElement('button'); zu.type = 'button'; zu.className = 'atlas-zu'; zu.textContent = '✕ Schließen'; zu.setAttribute('aria-label', 'Kräfteatlas schließen'); zu.addEventListener('click', () => OPT.schliessen()); const leiste = document.createElement('div'); leiste.className = 'atlas-zu-leiste'; seite.prepend(leiste); seite.prepend(zu); }
wurzel.append(seite);
const roh = window.KRAEFTEATLAS_DATEN, pfad = u => !u || u.startsWith('data:') ? u : basis + u;
const bilder = {volk: Object.fromEntries(Object.entries(roh.bilder.volk).map(([v, m]) => [v, Object.fromEntries(Object.entries(m).map(([k, u]) => [k, pfad(u)]))])),
  grund: Object.fromEntries(Object.entries(roh.bilder.grund).map(([k, u]) => [k, pfad(u)]))};
const UMG = {wurzel, seite, scroller: OPT.vollbild ? seite : window, eingebettet: OPT.eingebettet !== false, start: OPT.start, volk: OPT.volk, schliessen: OPT.schliessen, daten: roh.daten, bilder};

"use strict";
/* Umgebung: eigenständige Seite (document) oder Modul im Spiel/auf der Website (UMG aus kraefteatlas/atlas.js:
   wurzel = ShadowRoot, seite = Bildlauf-Element, daten/bilder aus kraefteatlas/daten.js). */
const U0 = typeof UMG !== "undefined" ? UMG : { wurzel: document, seite: document.body, scroller: window, eingebettet: false };
const W = U0.wurzel, SEITE = U0.seite, SCROLLER = U0.scroller;
const D = U0.daten || JSON.parse(document.getElementById("daten").textContent);
const BILD = U0.bilder || JSON.parse(document.getElementById("bilder").textContent);
const M = D.meta, E = D.e, R = D.reihe, B = D.boden;
const VERT = new Set(D.gruppen.find(g => g.key === "verteidigung").e);
const SCHIFFE = R.filter(e => !VERT.has(e));
const BEL = new Set(D.belagerer);
const $ = (s, el = W) => el.querySelector(s);
const esc = s => String(s).replace(/[&<>"]/g, c => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;" }[c]));
const fmt = (x, s = 0) => Number(x).toLocaleString("de-DE", { minimumFractionDigits: s, maximumFractionDigits: s });
const pct = (x, s = 0) => fmt(x * 100, s) + " %";
const fz = x => fmt(x, 2).replace(/,?0+$/, "");
const N = e => D.namen[e] || e;
const GUT = { erz: "Erz", kristall: "Kristall", deuterium: "Deuterium", nahrung: "Nahrung", legierung: "Legierung", elektronik: "Elektronik", konsumgut: "Konsumgut", xenokristall: "Xenokristall", antriebskern: "Antriebskerne", habitatmodul: "Habitatmodule" };
const kosten = k => Object.entries(k).map(([g, n]) => fmt(n) + " " + (GUT[g] || g)).join(" · ");
let volk = "syntheten";
try { volk = localStorage.getItem("kraefteatlas.volk") || volk; } catch (e) { }
if (U0.volk && ["aurelianer", "krath", "syntheten", "veyari"].includes(U0.volk)) volk = U0.volk; // im Spiel: das eigene Volk
const bild = e => (BILD.volk[volk] || BILD.volk.syntheten)[e] || "";
/* Bühnenhintergrund: dasselbe Motiv ultra-transparent (im Duell je Seite das eigene) */
const hinter = (u, seite = "") => u ? `<span class="hinter ${seite}" data-st="background-image:url(${u})"></span>` : "";
const img = (e, alt = "") => `<img src="${bild(e)}" alt="${esc(alt || N(e))}">`;
/* ---------- Völker: Kampffaktoren, Motive je Volk, Gefechte je Volkspaarung (alles aus Messung und Katalog) */
const VK = D.voelker, VOELKER = ["aurelianer", "krath", "syntheten", "veyari"];
let gvolk = "krath";
try { gvolk = localStorage.getItem("kraefteatlas.gegner") || (volk === "krath" ? "aurelianer" : "krath"); } catch (e) { }
const VN = v => VK.voelker[v].name, VFARBE = v => VK.voelker[v].farbe;
const VF = v => ({ w: VK.voelker[v].regel.waffen, p: VK.voelker[v].regel.panzerung });
const bildV = (e, v) => (BILD.volk[v] || BILD.volk.syntheten)[e] || "";
const imgV = (e, v, alt = "") => `<img src="${bildV(e, v)}" alt="${esc(alt || N(e))}">`;
/* Kampfwerte mit Technik 6 und Volksfaktoren, gerundet wie mal() im Kern */
const kwV = (e, v) => { const x = E[e], f = VF(v); return [Math.round(x.basis[0] * M.technik * f.w), Math.round(x.basis[1] * M.technik), Math.round(x.basis[2] * M.technik * f.p)]; };
function paarV(a, b, L, R) {
  let x = VK.kampf[`${a}|${b}|${L}|${R}`], flip = false;
  if (!x) { x = VK.kampf[`${b}|${a}|${R}|${L}`]; flip = true; }
  if (!x) return null;
  const [na, nd, sa, re, sd, la, ld, dauer, ra, rd, fe] = x;
  const o = flip ? { n: [nd, na], sieg: [sd, re, sa], verlust: [ld, la], rest: [rd, ra], feuer: fe.map(r => [[r[2], r[3]], [r[0], r[1]]]) }
    : { n: [na, nd], sieg: [sa, re, sd], verlust: [la, ld], rest: [ra, rd], feuer: fe.map(r => [[r[0], r[1]], [r[2], r[3]]]) };
  return Object.assign(o, { a, b, L, R, dauer, angreifer: flip ? b : a, t: tausch(o.verlust[1], o.verlust[0]) });
}
const TV = (a, b, L, R) => { if (VERT.has(a) && VERT.has(b)) return null; const p = paarV(a, b, L, R); return p ? p.t : null; };
const MERKMAL = [["waffen", "Waffen", 1, true], ["panzerung", "Panzerung", 1, true], ["ladung", "Ladekapazität", 1], ["marktgebuehr", "Marktgebühr", -1], ["steuer", "Steuereinnahmen", 1],
  ["werftzeit", "Werftbauzeit", -1], ["forschung", "Forschung", 1], ["wachstum", "Bevölkerungswachstum", 1], ["nahrung", "Nahrung", 1], ["energie", "Energie", 1], ["kolonieschiff", "Kosten des Kolonieschiffs", -1]];
function merkmale(v) {
  const r = VK.voelker[v].regel, l = [];
  for (const [k, n, gut, kampf] of MERKMAL) if (Math.abs(r[k] - 1) > 1e-9) l.push(`<span class="merk ${kampf ? "kampf " : ""}${(r[k] - 1) * gut > 0 ? "gut" : "schlecht"}">${n} <b>×${fz(r[k])}</b></span>`);
  if (r.pluenderquote !== null && r.pluenderquote !== undefined) l.push(`<span class="merk gut">Plünderquote <b>${pct(r.pluenderquote)}</b> statt ${pct(D.voelker.pluenderquote || 0.4)}</span>`);
  if (r.ohne_nahrung) l.push(`<span class="merk gut">braucht <b>keine Nahrung</b></span>`);
  if (VK.voelker[v].miliz) l.push(`<span class="merk">Miliz: <b>${pct(VK.voelker[v].miliz / 1000)}</b> der Bewohner kämpfen</span>`);
  if (!l.some(x => x.includes("kampf"))) l.unshift(`<span class="merk kampf">Kampf <b>ohne Volksbonus</b></span>`);
  return l.join("");
}
/* Steckbrief aus dem Techbaum: Flug, Laderaum, Kosten, Voraussetzungen, Bedingungen, Lore je Volk, Archiv */
/* Spielzeit lesbar: 17595 s → „4 h 53 min“ (Spielsekunden; Echtzeit = Spielzeit ÷ Welttempo) */
function dauer(s) {
  s = Math.round(s);
  if (s < 60) return fmt(s) + " s";
  if (s < 3600) { const mi = Math.floor(s / 60), se = s % 60; return mi + " min" + (se ? " " + se + " s" : ""); }
  if (s < 86400) { const h = Math.floor(s / 3600), mi = Math.round((s % 3600) / 60); return h + " h" + (mi ? " " + mi + " min" : ""); }
  const d = Math.floor(s / 86400), h = Math.round((s % 86400) / 3600); return d + " d" + (h ? " " + h + " h" : "");
}
const GEB = { werft: "Werft", orbitalwerft: "Orbitalwerft", nanofabrik: "Nanofabrik", kaserne: "Kaserne", raketensilo: "Raketensilo", raumhafen: "Raumhafen", forschungslabor: "Forschungslabor" };
const wort = x => GEB[x] || D.namen[x] || (x.charAt(0).toUpperCase() + x.slice(1));
const wertKarte = (n, b, k) => `<div class="wert"><dt>${n}</dt><dd><b>${b}</b>${k ? `<small>${k}</small>` : ""}</dd></div>`;
const gueterChips = k => `<div class="vk-merkmale">${Object.entries(k).filter(([, n]) => n).map(([g, n]) => `<span class="merk"><b>${fmt(n)}</b>&nbsp;${esc(GUT[g] || g)}</span>`).join("")}</div>`;
function steckbrief(e, v) {
  const s = D.steckbrief[e] || {}, sbR = D.sb.regeln;
  const sb = (D.sb.einheit[v] || {})[e], bo = D.sb.boden[e], ra = D.sb.raketen[e.replace(/^rakete_/, "")];
  const w = s.werte || {}, teile = [];
  // ---------- Bauen
  if (sb) {
    const wmin = Math.max(1, sb.werft, e === "weltentruemmer" ? 0 : sb.werftplaetze);
    const bauort = [`Werft ab Stufe <b>${fmt(wmin)}</b>${wmin > Math.max(1, sb.werft) ? ` (${fmt(sb.werftplaetze)} Werftplätze je Stück)` : ""}`, ...sb.braucht.map(([g, n]) => `${esc(wort(g))} <b>${fmt(n)}</b>`), `Zivilisationsstufe <b>${["", "I", "II", "III", "IV", "V", "VI"][sb.ab_stufe] || sb.ab_stufe}</b>`];
    const forschung = sb.forschung.map(([f, n]) => `${esc(wort(f))} <b>${fmt(n)}</b>`);
    const werften = [...new Set(sb.bauzeit.map(b => b.werft))], nanos = [...new Set(sb.bauzeit.map(b => b.nanofabrik))];
    const zeit = (wf, nf) => (sb.bauzeit.find(b => b.werft === wf && b.nanofabrik === nf) || {}).sekunden;
    const wz = D.sb.werftzeit[v];
    const tabelle = sb.bauzeit.length ? `<div class="tbl-wrap"><table class="tbl"><thead><tr><th>Werft</th>${nanos.map(n => `<th>Nanofabrik ${n}</th>`).join("")}<th>gleichzeitig</th></tr></thead><tbody>${werften.map(wf =>
        `<tr><th>Stufe ${wf}</th>${nanos.map(n => `<td class="z" data-tip="${esc(`${E[e] ? E[e].name : N(e)}: Werft ${wf}, Nanofabrik ${n} – ${dauer(zeit(wf, n))} je Stück, ${Math.floor(wf / sb.werftplaetze)} gleichzeitig`)}">${dauer(zeit(wf, n))}</td>`).join("")}<td class="z"><b>${fmt(Math.floor(wf / sb.werftplaetze))}</b><small>${fmt(wf)} Plätze ÷ ${fmt(sb.werftplaetze)}</small></td></tr>`).join("")}</tbody></table></div>` : "";
    teile.push(`<h3 class="block-h">Bauen <small>Kosten und Zeit je Stück · ${esc(VN(v))}</small></h3>
      <div class="karte gold"><h4>Kosten je Stück · Wert ${fmt(sb.wert)}</h4>${gueterChips(sb.kosten)}<p class="leise" data-st="margin:6px 0 0">Der Wert gewichtet die Güter wie im Kampfvergleich (Erz 1, Kristall 1,5, Deuterium 2, Legierung 5, Elektronik 8, Xenokristall 20).</p></div>
      <dl class="werte">${wertKarte("Bauort", bauort.join(" · "), "")}${forschung.length ? wertKarte("Forschung", forschung.join(" · "), "muss erforscht sein") : ""}
        ${e !== "weltentruemmer" ? wertKarte("Werftplätze je Stück", fmt(sb.werftplaetze), `Werftstufe = Plätze; ${fmt(sb.werftplaetze)} je Stück`) : ""}
        ${sb.max_anzahl ? wertKarte("Höchstbestand", fmt(sb.max_anzahl), "je Planet") : ""}
        ${wz !== 1 ? wertKarte("Werftzeit des Volkes", "×" + fz(wz), `${esc(VN(v))} baut ${wz < 1 ? "schneller" : "langsamer"} – in den Bauzeiten eingerechnet`) : ""}</dl>
      ${tabelle ? `<p class="reihe" data-st="color:var(--cyan2)">Bauzeit je Stück (Spielzeit, volle Produktionsleistung)</p>${tabelle}
        <p class="leise" data-st="margin:4px 0 0">Jede Werftstufe beschleunigt, jede Nanofabrikstufe halbiert die Zeit annähernd; mindestens ${fmt(sbR.mindest_fertigungszeit)} s je Stück. Gleichzeitig = Werftstufe ÷ Werftplätze je Stück. Echtzeit = Spielzeit ÷ Welttempo.</p>` : ""}`);
    if (e === "weltentruemmer") teile.push(`<p class="leise">Gebaut wird er im Werftdock eines eigenen Mondes (siehe Voraussetzungen); die Bauzeit richtet sich nach der Werftdockstufe.</p>`);
    // ---------- Betrieb
    const betrieb = [];
    if (sb.unterhalt_tag) betrieb.push(wertKarte("Unterhalt je Spieltag", fmt(sb.unterhalt_tag) + " Credits", `${pct(sbR.unterhalt_schiffe_je_tag, 1)} des Werts; läuft auch unterwegs und im Orbit`));
    else if (VERT.has(e)) betrieb.push(wertKarte("Unterhalt", "keiner", "Verteidigungsanlagen kosten keinen Unterhalt"));
    if (sb.besatzung) betrieb.push(wertKarte("Besatzung", fmt(sb.besatzung), "Einwohner, beim Bau vom Planeten abgezogen"));
    if (sb.ladung) betrieb.push(wertKarte("Laderaum", fmt(sb.ladung), `${esc(VN(v))}${VK.voelker[v].regel.ladung !== 1 ? " ×" + fz(VK.voelker[v].regel.ladung) : ""} · mit Logistik 5: ${fmt(sb.ladung_logistik5)} (+${pct(sbR.logistik_bonus)} je Stufe)`));
    if (sb.personenplaetze) betrieb.push(wertKarte("Personenplätze", fmt(sb.personenplaetze), "Siedler oder Fachkräfte an Bord"));
    if (betrieb.length) teile.push(`<h3 class="block-h">Betrieb</h3><dl class="werte">${betrieb.join("")}</dl>`);
    // ---------- Flug
    if (sb.flug) {
      const st = sb.flug, a0 = st[0];
      teile.push(`<h3 class="block-h">Flug <small>ein Schiff, volle Geschwindigkeit, ohne Raumhafen</small></h3>
        <dl class="werte">${wertKarte("Antrieb", esc(wort(a0.antrieb || "")), `Grundtempo ${fmt(a0.tempo)} · +${pct(VK.antrieb_bonus[a0.antrieb] || 0)} je Antriebsstufe`)}${wertKarte("Treibstoffwert", fmt(a0.verbrauch), "Faktor in der Treibstoffformel")}</dl>
        <div class="tbl-wrap"><table class="tbl"><thead><tr><th>Strecke</th><th class="z">Deuterium<small>je Flug</small></th>${st.map(x => `<th class="z">Stufe ${x.stufe}<small>Tempo ${fmt(x.tempo)}</small></th>`).join("")}</tr></thead><tbody>${a0.wege.map((g, i) =>
          `<tr><th>${esc(g.strecke)}<small>Entfernung ${fmt(g.entfernung)}</small></th><td class="z"><b>${fmt(g.deuterium, g.deuterium < 10 ? 1 : 0)}</b></td>${st.map(x => `<td class="z" data-tip="${esc(`${g.strecke} mit ${wort(x.antrieb || "")} ${x.stufe}: ${dauer(x.wege[i].sekunden)}`)}">${dauer(x.wege[i].sekunden)}</td>`).join("")}</tr>`).join("")}</tbody></table></div>
        <p class="leise" data-st="margin:4px 0 0">Flugzeit je ${esc(wort(a0.antrieb || ""))}-Stufe (Spielzeit); Mindestflugzeit ${dauer(sbR.flug_min_sekunden)}. Der Raumhafen spart ${pct(sbR.raumhafen_ersparnis)} Treibstoff je Stufe (höchstens ${pct(sbR.raumhafen_ersparnis_max)}); langsamer fliegen spart quadratisch. Hin- und Rückflug zählen getrennt.</p>`);
    }
    // ---------- Kampffolgen
    const folgen = [];
    if (sb.truemmer.erz || sb.truemmer.kristall) folgen.push(wertKarte("Trümmer je zerstörtem Stück", `${fmt(sb.truemmer.erz)} Erz · ${fmt(sb.truemmer.kristall)} Kristall`, `${pct(sbR.truemmer_anteil)} von Erz und Kristall; mit Recyclern bergen`));
    if (VERT.has(e)) folgen.push(wertKarte("Wiederaufbau", pct(sbR.verteidigung_wiederaufbau), "der zerstörten Anlagen stehen nach dem Kampf wieder"));
    if (sb.ladung && !VERT.has(e)) folgen.push(wertKarte("Beute", `bis ${fmt(sb.ladung)} je Schiff`, `nach einem Sieg höchstens ${pct(VK.voelker[v].regel.pluenderquote ?? sbR.pluenderquote)} des ungeschützten Lagers (${esc(VN(v))}), begrenzt durch den Laderaum`));
    if (folgen.length) teile.push(`<h3 class="block-h">Im Kampf</h3><dl class="werte">${folgen.join("")}</dl>`);
  } else if (bo) {
    teile.push(`<h3 class="block-h">Ausbilden <small>Kosten und Zeit je Einheit</small></h3>
      <div class="karte gold"><h4>Kosten je Einheit</h4>${gueterChips(bo.kosten)}</div>
      <dl class="werte">${wertKarte("Kaserne", "ab Stufe " + fmt(bo.kaserne), `Zivilisationsstufe ${["", "I", "II", "III", "IV", "V", "VI"][bo.ab_stufe] || bo.ab_stufe}`)}
        ${wertKarte("Ausbildung je Einheit", dauer(bo.dauer_sekunden), "Spielzeit bei voller Produktionsleistung")}
        ${wertKarte("Kasernenplätze je Einheit", fmt(bo.fertigungsplaetze), "Kasernenstufe = Plätze; gleichzeitig = Stufe ÷ Plätze")}
        ${wertKarte("Personal", fmt(bo.personal), "Rekrut(en) aus der Bevölkerung, mindestens 500 Einwohner bleiben")}
        ${wertKarte("Transportplätze", fmt(bo.transportplaetze), "im Truppentransporter (100 Plätze)")}</dl>`);
  } else if (ra) {
    teile.push(`<h3 class="block-h">Bauen <small>Kosten und Zeit je Rakete</small></h3>
      <div class="karte gold"><h4>Kosten je Rakete</h4>${gueterChips(ra.kosten)}</div>
      <dl class="werte">${wertKarte("Raketensilo", "ab Stufe " + fmt(ra.silo), `Zivilisationsstufe ${["", "I", "II", "III", "IV", "V", "VI"][ra.zivilisation] || ra.zivilisation}`)}
        ${wertKarte("Bauzeit je Rakete", dauer(ra.bauzeit_sekunden), "Spielzeit bei voller Produktionsleistung")}
        ${wertKarte("Siloplätze", fmt(ra.plaetze), "je Rakete im Lager des Silos")}
        ${ra.reichweite_je_silo ? wertKarte("Reichweite", fmt(ra.reichweite_je_silo) + " Systeme je Silostufe", "") : ""}
        ${ra.flug_min_sekunden || ra.flug_je_system_sekunden ? wertKarte("Flugzeit", `${dauer(ra.flug_min_sekunden)} + ${dauer(ra.flug_je_system_sekunden)} je System`, "") : ""}
        ${ra.schaden ? wertKarte("Schaden", fmt(ra.schaden), "Grundwert, Waffentechnik erhöht ihn") : ""}
        ${ra.abfangbedarf ? wertKarte("Abfangbedarf", fmt(ra.abfangbedarf), "Abfangraketen je anfliegender Rakete") : ""}</dl>`);
  }
  const vor = (s.voraus || []).concat((s.voraus_volk || {})[v] || []).filter(x => x.art !== "gut").map(x => x.text || `${x.name} ${typeof x.n === "number" ? fmt(x.n) : ""}`.trim());
  const bed = (s.bedingungen || []).concat((s.bedingungen_volk || {})[v] || []);
  const wechsel = (s.antriebswechsel || []).map(a => `ab ${esc(a.forschung_name || a.forschung)} ${a.stufe}: Tempo ${fmt(a.tempo)}, Treibstoff ${fmt(a.verbrauch)}`);
  const lore = s.lore || {}, ar = s.archiv || {};
  const absatz = x => x ? `<p>${esc(x)}</p>` : "";
  return `<h3 class="block-h">Steckbrief <small>${esc(s.klasse || "")}</small></h3>
    <div class="karten" data-st="margin-top:8px">${s.zweck ? `<div class="karte"><h4>Zweck</h4><p>${esc(s.zweck)}</p></div>` : ""}${s.staerke ? `<div class="karte gruen"><h4>Stärke</h4><p>${esc(s.staerke)}</p></div>` : ""}${s.nische ? `<div class="karte violett"><h4>Nische</h4><p>${esc(s.nische)}</p></div>` : ""}</div>
    ${teile.join("")}
    ${vor.length ? `<p class="reihe" data-st="color:var(--cyan2)">Alle Voraussetzungen</p><div class="vk-merkmale">${vor.map(x => `<span class="merk">${esc(x)}</span>`).join("")}</div>` : ""}
    ${wechsel.length ? `<p class="reihe" data-st="color:var(--cyan2)">Antriebswechsel</p><div class="vk-merkmale">${wechsel.map(x => `<span class="merk">${x}</span>`).join("")}</div>` : ""}
    ${bed.length ? `<div class="karte" data-st="margin-top:10px"><h4>Bedingungen</h4><p>${bed.map(esc).join("<br>")}</p></div>` : ""}
    ${lore.allgemein || lore[v] ? `<details class="logbuch"><summary>Logbuch · ${esc(VN(v))}</summary><div class="text">${absatz(lore[v])}${absatz(lore.allgemein)}</div></details>` : ""}
    ${ar.technik || ar.geschichte || ar[v] ? `<details class="logbuch"><summary>Archiv · Technik, Chronik, Akte ${esc(VN(v))}</summary><div class="text">${ar.technik ? `<h5>Technisches Dossier</h5>${absatz(ar.technik)}` : ""}${ar.geschichte ? `<h5>Chronik</h5>${absatz(ar.geschichte)}` : ""}${ar[v] ? `<h5>Akte ${esc(VN(v))}</h5>${absatz(ar[v])}` : ""}
      ${ar.verweise && ar.verweise.length ? `<h5>Querverweise</h5><div class="vk-merkmale">${ar.verweise.map(([id, n]) => E[id] ? `<button class="merk" data-open="einheit:${id}" data-st="cursor:pointer">${esc(n)}</button>` : `<span class="merk">${esc(n)}</span>`).join("")}</div>` : ""}</div></details>` : ""}`;
}
/* Tauschverhältnis kurz: 3 = dreimal so viel zerstört wie verloren, /3 = dreimal so viel verloren wie zerstört, 20+ = 20-fach oder mehr. */
function rf(r) {
  if (r >= 19.5) return "20+"; if (r >= 9.5) return fmt(r); if (r >= 1.05) return fmt(r, 1);
  if (r <= 1 / 19.5) return "/20+"; if (r <= 1 / 9.5) return "/" + fmt(1 / r); if (r <= 0.95) return "/" + fmt(1 / r, 1); return "=";
}
const tf = t => rf(Math.exp(t));
const tx = t => { const s = tf(t); return s[0] === "/" || s === "=" ? s : "×" + s; };
/* mit Wort, wo die Zahl allein steht: „= ausgeglichen“ */
const txw = t => tf(t) === "=" ? "= ausgeglichen" : tx(t);
const kl = t => t >= 0.14 ? "gut" : t <= -0.14 ? "schlecht" : "";
const tausch = (ld, la) => Math.max(-3, Math.min(3, Math.log((ld + 0.03) / (la + 0.03))));
const POS = [111, 211, 154], NEG = [232, 105, 122], MID = [18, 35, 58];
function farbe(t, max = 2.3) { const s = Math.min(1, Math.abs(t) / max), p = t > 0 ? POS : NEG; return "rgb(" + p.map((v, i) => Math.round(MID[i] + (v - MID[i]) * s)).join(",") + ")"; }
const dunkel = (t, max = 2.3) => Math.abs(t) / max > 0.55;
const RANG = ["Primärziel", "Sekundärziel", "Reserveziel"];
const wie = f => f >= 19.5 ? "mindestens 20-mal" : fmt(f, f >= 9.5 ? 0 : 1) + "-mal";
const mio = w => w >= 50000 ? fmt(w / 1e6, 1) + " Mio." : fmt(w);
const kanal = n => `${n} Zielkan${n > 1 ? "äle" : "al"}`;
/* Messleiste: Wert im Verhältnis zum größten der Kampfeinheiten, Wurzelmaß wie im Spiel gut lesbar */
const MAX = [0, 1, 2].map(i => Math.max(...R.map(e => E[e].basis[i])));
const mess = (v, max, cls) => { const w = Math.max(3, Math.sqrt(Math.max(0, v) / max) * 100); return `<span class="mess"><i class="${cls || (w > 66 ? "hoch" : w > 33 ? "mittel" : "")}" data-st="width:${w}%"></i></span>`; };

/* ---------- Paar-Sicht: Messung immer aus Sicht der Zeile (a) ---------- */
function paar(a, b) {
  let p = D.paare[a + "|" + b];
  if (p) return Object.assign({ a, b, angreifer: a, verteidiger: b }, p);
  p = D.paare[b + "|" + a];
  if (!p) return null;
  return { a, b, angreifer: b, verteidiger: a, art: p.art, n: [p.n[1], p.n[0]], sieg: [p.sieg[2], p.sieg[1], p.sieg[0]], verlust: [p.verlust[1], p.verlust[0]],
    t: -p.t, aktiv: p.aktiv, dauer: p.dauer, rest: [p.rest[1], p.rest[0]], feuer: p.feuer.map(r => [r[1], r[0]]),
    laeufe: p.laeufe.map(l => [l[1], l[0], l[2] === "a" ? "d" : l[2] === "d" ? "a" : "-", l[3]]) };
}
const T = (a, b) => { if (VERT.has(a) && VERT.has(b)) return null; const p = paar(a, b); return p ? p.t : null; };
const klar = p => Math.abs(p.t) >= M.hart && Math.max(...p.verlust) >= 0.25;

/* ---------- Reiter, Blatt, Hinweise ---------- */
function reiter(name, push = true) {
  W.querySelectorAll("#reiter button").forEach(b => b.setAttribute("aria-selected", b.dataset.ziel === name));
  W.querySelectorAll("section[data-tab]").forEach(s => s.classList.toggle("an", s.dataset.tab === name));
  const b = $(`#reiter button[data-ziel="${name}"]`), z = $("#zeiger");
  if (b) { z.style.left = b.offsetLeft + 8 + "px"; z.style.width = b.offsetWidth - 16 + "px"; }
  if (push && !U0.eingebettet) try { history.replaceState(null, "", "#" + name); } catch (e) { }
}
const stapel = [];
function oeffne(art, ...args) { stapel.push([art, args]); zeichne(); }
function zeichne() {
  const d = $("#detail");
  if (!stapel.length) { d.classList.remove("offen"); d.setAttribute("aria-hidden", "true"); SEITE.classList.remove("blatt-offen"); return; }
  const [art, args] = stapel[stapel.length - 1];
  const r = ANSICHT[art](...args);
  $("#d-leiste").textContent = r.leiste;
  $("#d-inhalt").innerHTML = r.html;
  $("#d-zurueck").hidden = stapel.length < 2;
  d.classList.add("offen"); d.setAttribute("aria-hidden", "false"); SEITE.classList.add("blatt-offen");
  $("#d-inhalt").scrollTop = 0;
  $("#d-zu").focus({ preventScroll: true });
}
const zu = () => { stapel.length = 0; zeichne(); };
$("#d-zu").onclick = zu;
$("#schleier").onclick = zu;
$("#d-zurueck").onclick = () => { stapel.pop(); zeichne(); };
W.addEventListener("keydown", e => {
  // Escape gehört dem Atlas: nicht an den Technologiebaum darunter weiterreichen (der schlösse sonst gleich mit)
  if (e.key === "Escape" && (stapel.length || U0.schliessen)) { e.stopPropagation(); if (stapel.length) stapel.length > 1 ? (stapel.pop(), zeichne()) : zu(); else U0.schliessen(); }
  if ((e.key === "Enter" || e.key === " ") && e.target.matches && e.target.matches("[data-open]:not(button),[data-rad]:not(button)")) { e.preventDefault(); e.target.dispatchEvent(new MouseEvent("click", { bubbles: true })); }
});
W.addEventListener("click", e => {
  const rn = e.target.closest("[data-rad]");
  if (rn) { rad.e = rn.dataset.rad; radZeichnen(); return; }
  const t = e.target.closest("[data-open]");
  if (t) { e.preventDefault(); tip.style.opacity = 0; const [art, ...args] = t.dataset.open.split(":"); oeffne(art, ...args); return; }
  const r = e.target.closest("[data-reiter]");
  if (r) { zu(); reiter(r.dataset.reiter); SCROLLER.scrollTo({ top: 0 }); }
});
W.querySelectorAll("#reiter button").forEach(b => b.onclick = () => reiter(b.dataset.ziel));
const tip = $("#tip");
W.addEventListener("pointermove", e => {
  const el = e.target.closest("[data-tip]");
  if (!el || e.pointerType === "touch") { tip.style.opacity = 0; return; }
  tip.textContent = el.dataset.tip; tip.style.opacity = 1;
  const w = tip.offsetWidth, h = tip.offsetHeight; let l = e.clientX + 14, o = e.clientY + 14;
  if (l + w > innerWidth - 8) l = e.clientX - w - 14; if (o + h > innerHeight - 8) o = e.clientY - h - 14;
  tip.style.left = Math.max(8, l) + "px"; tip.style.top = Math.max(8, o) + "px";
});
function segment(id, schluessel, ziel, neu) {
  const el = $(id);
  const set = () => el.querySelectorAll("button").forEach(b => b.setAttribute("aria-pressed", b.dataset.w === ziel[schluessel]));
  el.querySelectorAll("button").forEach(b => b.onclick = () => { ziel[schluessel] = b.dataset.w; set(); neu(); });
  set();
}

/* ---------- Bausteine ---------- */
const chips = (l, art, open, text) => `<ul class="chips">${l.map(o => `<li class="${art}${o.knapp ? " knapp" : ""}"><button data-open="${open(o)}" data-tip="${esc(o.tip || "")}">${img(o.e)}<span>${esc(N(o.e))}</span>${o.wert ? ` <b>${o.wert}</b>` : ""}${o.knapp ? " <small>knapp</small>" : ""}</button></li>`).join("") || `<li class="leise">${text || "–"}</li>`}</ul>`;
const marke = e => `<span class="marke">${esc(E[e] ? E[e].kurz : B.typen[e].kurz)}</span>`;
function buehne(e, augenbraue, titel, pillen = "", extra = "") {
  return `<div class="buehne">${hinter(bild(e))}<span class="strahl"></span><span class="sockel"><i></i></span>${img(e)}${extra}<span class="abtasten"></span>
    <div class="titel"><p class="augenbraue">${augenbraue}</p><h2>${esc(titel)}</h2>${pillen}</div></div>`.replace("<img ", '<img class="motiv" ');
}
function linien(serien, xl, opt = {}) {
  const W = 560, H = opt.h || 220, l = 46, r = 14, o = 24, u = 30, n = xl.length;
  const x = i => l + (W - l - r) * (n > 1 ? i / (n - 1) : 0.5), ymax = opt.ymax || 1, y = v => o + (H - o - u) * (1 - v / ymax);
  let s = `<svg viewBox="0 0 ${W} ${H}" width="100%" role="img" aria-label="${esc(opt.label || "Verlauf")}">`;
  for (let k = 0; k <= 4; k++) { const v = ymax * k / 4; s += `<line x1="${l}" x2="${W - r}" y1="${y(v)}" y2="${y(v)}" stroke="#97abc026"/><text x="${l - 7}" y="${y(v) + 4}" text-anchor="end" font-size="11" fill="#97abc0">${pct(v)}</text>`; }
  xl.forEach((t, i) => { s += `<text x="${x(i)}" y="${H - 10}" text-anchor="middle" font-size="11" fill="#97abc0">${esc(t)}</text>`; });
  // Namen am Linienende: mindestens 15 Einheiten auseinander und über der Achsenbeschriftung – enden beide Linien gleich
  // (Duell zweier gleicher Truppen bis 0 %), lagen die Namen sonst übereinander und auf „W7 W8“ (Prüfung 26.10.2026).
  const oben = o + 10, unten = H - u - 6, ly = serien.map(se => Math.min(unten, Math.max(oben, y(se.werte[se.werte.length - 1]) - 9)));
  const folge = ly.map((_, i) => i).sort((a, b) => ly[a] - ly[b]);
  for (let j = 1; j < folge.length; j++) ly[folge[j]] = Math.max(ly[folge[j]], ly[folge[j - 1]] + 15);
  const ueber = ly[folge[folge.length - 1]] - unten;
  if (ueber > 0) folge.forEach(i => { ly[i] = Math.max(oben, ly[i] - ueber); });
  serien.forEach((se, k) => {
    s += `<polyline fill="none" stroke="${se.farbe}" stroke-width="2.4" stroke-linejoin="round" points="${se.werte.map((v, i) => x(i) + "," + y(v)).join(" ")}"/>`;
    se.werte.forEach((v, i) => { s += `<circle cx="${x(i)}" cy="${y(v)}" r="5" fill="${se.farbe}" stroke="#060d18" stroke-width="2" data-tip="${esc(se.tips ? se.tips[i] : se.name + ": " + pct(v))}"/>`; });
    s += `<text x="${W - r}" y="${ly[k]}" text-anchor="end" font-size="12" font-weight="700" fill="${se.farbe}">${esc(se.name)}</text>`;
  });
  return s + "</svg>";
}
function streuung(laeufe, ax, ay, opt = {}) {
  const W = 300, H = 276, l = 52, r = 14, o = 16, u = 40, mx = opt.mx || 1, my = opt.my || 1, f = opt.fmt || pct;
  const x = v => l + (W - l - r) * Math.min(1, v / mx), y = v => o + (H - o - u) * (1 - Math.min(1, v / my));
  let s = `<svg viewBox="0 0 ${W} ${H}" width="100%" role="img" aria-label="Streuung der Läufe">`;
  for (let k = 0; k <= 4; k++) {
    s += `<line x1="${x(mx * k / 4)}" x2="${x(mx * k / 4)}" y1="${o}" y2="${H - u}" stroke="#97abc01f"/><line x1="${l}" x2="${W - r}" y1="${y(my * k / 4)}" y2="${y(my * k / 4)}" stroke="#97abc01f"/>`;
    s += `<text x="${x(mx * k / 4)}" y="${H - u + 14}" text-anchor="middle" font-size="10.5" fill="#97abc0">${f(mx * k / 4)}</text><text x="${l - 12}" y="${y(my * k / 4) + 4}" text-anchor="end" font-size="10.5" fill="#97abc0">${f(my * k / 4)}</text>`;
  }
  if (opt.diagonale !== false) s += `<line x1="${x(0)}" y1="${y(0)}" x2="${x(Math.min(mx, my))}" y2="${y(Math.min(mx, my))}" stroke="#e2bf7d99" stroke-dasharray="4 4"/>`;
  s += `<text x="${(l + W - r) / 2}" y="${H - 6}" text-anchor="middle" font-size="11" fill="#6fd6ff">${esc(ax)}</text>`;
  s += `<text transform="translate(12 ${(o + H - u) / 2}) rotate(-90)" text-anchor="middle" font-size="11" fill="#e2bf7d">${esc(ay)}</text>`;
  const g = new Map();
  for (const lf of laeufe) { const k = [Math.round(lf[0] * 50) / 50, Math.round(lf[1] * 50) / 50, lf[2]].join("|"); g.set(k, (g.get(k) || 0) + 1); }
  for (const [k, n] of g) {
    const [a, b, w] = k.split("|"), c = w === "a" ? "#6fd6ff" : w === "d" ? "#e2bf7d" : "#97abc0";
    s += `<circle cx="${x(+a)}" cy="${y(+b)}" r="${Math.min(9, 3.5 + Math.sqrt(n) * 1.1)}" fill="${c}" fill-opacity=".8" stroke="#060d18" data-tip="${n} Lauf${n > 1 ? "e" : ""}: ${esc(ax)} ${f(+a)}, ${esc(ay)} ${f(+b)} · ${esc(opt.sieger[w])}"/>`;
  }
  return s + "</svg>";
}
function stapelbalken(sieg, na, nb) {
  const t = (v, cls, txt) => v > 0.001 ? `<span class="${cls}" data-st="flex:${v}" data-tip="${esc(txt + ": " + pct(v))}">${v >= 0.12 ? pct(v) : ""}</span>` : "";
  return `<div class="stapel">${t(sieg[0], "sa", na + " siegt")}${t(sieg[1], "so", "offen nach 6 Runden")}${t(sieg[2], "sb", nb + " siegt")}</div>
  <p class="leg" data-st="margin:0"><span><span class="sw" data-st="background:var(--sa)"></span>${esc(na)} siegt ${pct(sieg[0])}</span><span><span class="sw" data-st="background:#4b5d70"></span>offen nach 6 Runden ${pct(sieg[1])}</span><span><span class="sw" data-st="background:var(--sb)"></span>${esc(nb)} siegt ${pct(sieg[2])}</span></p>`;
}
/* Punktachse, logarithmisch (±ln 16), mit „verliert ← · → gewinnt“ */
const LG = Math.log(16);
const posX = (t, g = LG) => 50 + 50 * Math.max(-1, Math.min(1, t / g));
const TICKS = [["/16", -LG], ["/4", -Math.log(4)], ["/2", -Math.log(2)], ["×1", 0], ["×2", Math.log(2)], ["×4", Math.log(4)], ["×16", LG]];
const T20 = [["/20", -3], ["/4", -Math.log(4)], ["×1", 0], ["×4", Math.log(4)], ["×20", 3]];
const tickzeile = (tk, g, links, rechts) => `<div class="ticks"><span>${links}</span><div><span class="ende l" data-st="left:0">◀ verliert</span><span class="ende r" data-st="left:100%">gewinnt ▶</span>${tk.map(([t, v]) => `<span data-st="left:${posX(v, g)}%">${t}</span>`).join("")}</div><span>${rechts}</span></div>`;
const gitter = (tk, g) => tk.map(([, v]) => `<i class="${v === 0 ? "eins" : ""}" data-st="left:${posX(v, g)}%"></i>`).join("");
function profilzeile(e, name, werte, open, tk = TICKS, g = LG, zusatz = "") {
  const marken = werte.filter(w => w.t !== null && w.t !== undefined).map(w => `<span class="m ${w.d ? "d" : ""}" data-st="left:${posX(w.t, g)}%" data-tip="${esc(w.tip)}"></span>`).join("");
  const v = werte.filter(w => w.t !== null && w.t !== undefined).map((w, i) => `${i ? "<small>" : ""}<span class="mk ${w.d ? "d" : ""}"></span><b class="${kl(w.t)}">${tx(w.t)}</b>${i ? "</small>" : ""}`).join("");
  return `<div class="pr" data-open="${open}" tabindex="0"><span class="n">${img(e)}<span>${esc(name)}${zusatz}</span></span><span class="achse">${gitter(tk, g)}${marken}</span><span class="v">${v}</span></div>`;
}
function balkenzeile(e, t, open, tip) {
  const w = Math.min(50, Math.abs(t) / 3 * 50), left = t >= 0 ? 50 : 50 - w;
  return `<div class="pr" data-open="${open}" tabindex="0" data-tip="${esc(tip)}"><span class="n">${img(e)}<span>${esc(N(e))}</span></span><span class="achse">${gitter(T20, 3)}<span class="bar" data-st="left:${left}%;width:${Math.max(w, .8)}%;background:${t >= 0 ? "var(--gut)" : "var(--rot)"}"></span></span><span class="v"><b class="${kl(t)}">${tx(t)}</b></span></div>`;
}

/* ---------- Duell-Bilanz ----------
   Stufe je Duell: 2 klar gewonnen (Tausch ≥ ×2 und der Verlierer verliert ≥ 25 %), 1 knapp gewonnen (≥ ×1,4), 0 offen, −1/−2 spiegelbildlich. */
const KNAPP = Math.log(1.4);
const stufe = (t, verlust) => { const a = Math.abs(t), s = t >= 0 ? 1 : -1; return a >= M.hart && Math.max(...verlust) >= 0.25 ? 2 * s : a >= KNAPP ? s : 0; };
const STUFE_WORT = { 2: "klar gewonnen", 1: "knapp gewonnen", 0: "offen", "-1": "knapp verloren", "-2": "klar verloren" };
const kcls = k => k === 2 ? "k2" : k === 1 ? "k1" : k === 0 ? "k0" : k === -1 ? "km1" : "km2";
const bzLegende = () => [-2, -1, 0, 1, 2].map(k => `<span><i class="${kcls(k)}"></i>${STUFE_WORT[k]}</span>`).join("") + '<span><span class="mlinie"></span>Mittellinie: so viele Siege wie Niederlagen</span>';
/* zeilen: [{e, name, zusatz, open, g: [{k, t, z, kurz, tip, open}], anl: [...] | null, mittel}] – alle Reihen teilen eine Skala, damit die Mittellinien fluchten */
function bilanz(zeilen, ordnung, kopf) {
  const zaehl = r => { const n = [-2, -1, 0, 1, 2].map(k => r.g.filter(x => x.k === k).length); return { n, l: n[0] + n[1] + n[2] / 2, r: n[3] + n[4] + n[2] / 2, punkte: r.g.reduce((s, x) => s + x.k, 0) }; };
  zeilen.forEach(r => r.z = zaehl(r));
  const L = Math.max(...zeilen.map(r => r.z.l)), Rr = Math.max(...zeilen.map(r => r.z.r)), ges = L + Rr, mitte = L / ges * 100;
  const liste = ordnung === "bilanz" ? [...zeilen].sort((a, b) => b.z.punkte - a.z.punkte || b.mittel - a.mittel) : zeilen;
  const breite = 100 / ges;
  const zelle = (x, text) => `<i class="${kcls(x.k)}" data-open="${x.open}" tabindex="0" data-tip="${esc(x.tip)}"${x.kk ? ` data-k="${esc(x.kk)}"` : ""}>${text ? esc(x.kurz || "") : ""}</i>`;
  const ohneA = !zeilen.some(r => r.anl);
  let h = `<div class="bz kopf${ohneA ? " ohne-a" : ""}"><span>${kopf.links}</span><span class="bahn"><b>◀ verliert gegen</b><b>schlägt ▶</b></span>${ohneA ? "" : `<span class="anl">${kopf.anl.map(d => `<span data-tip="${esc(N(d))}">${img(d)}</span>`).join("")}</span>`}<span data-st="text-align:right">Siege : Niederlagen</span></div>`;
  for (const r of liste) {
    const g = [...r.g].sort((a, b) => a.k - b.k || a.t - b.t);
    const start = L - r.z.l;
    const bloecke = g.map((x, i) => zelle(x).replace("<i ", `<i data-st="left:calc(${(start + i) * breite}% + 1px);width:calc(${breite}% - 2px)" `)).join("");
    const [kl2, kl1, k0, kg1, kg2] = r.z.n, sieg = kg1 + kg2, nied = kl2 + kl1;
    h += `<div class="bz${ohneA ? " ohne-a" : ""}"><button class="n" data-open="${r.open}" data-tip="${esc(r.name)} – Dossier öffnen">${img(r.e)}<span>${esc(r.name)}${r.zusatz || ""}</span></button>
      <span class="bahn" data-st="--mitte:${mitte}%" data-tip="${esc(`${r.name}: ${kg2} klar und ${kg1} knapp gewonnen, ${k0} offen, ${kl1} knapp und ${kl2} klar verloren · im Mittel ${tx(r.mittel)}`)}">${bloecke}</span>
      ${r.anl ? `<span class="anl">${r.anl.map(x => zelle(x, true)).join("")}</span>` : ""}
      <span class="r"><span class="stand"><b class="gut">${sieg}</b><span>:</span><b class="schlecht">${nied}</b></span><small>${k0} offen · Ø ${tx(r.mittel)}</small></span></div>`;
  }
  return h;
}

/* ---------- Mechanik: wie X auf Y schießt (Zielwahl der Engine je Paar) ---------- */
function mechanik(x, y, nx, fx, fy) {
  const ex = E[x], ey = E[y], zw = ex.zw[y] || [null, 1, 1];
  const angr = fx ? Math.round(ex.basis[0] * M.technik * fx.w) : ex.kampf[0];
  const [rang, f, sf] = zw, bevorzugt = rang !== null && rang < 2;
  const dmg = Math.round(angr * f), sfk = bevorzugt ? ex.schildfaktor : 1;
  const S = ey.kampf[1], St = fy ? Math.round(ey.basis[2] * M.technik * fy.p) : ey.kampf[2];
  if (angr <= 0) return { li: [`${esc(ex.name)} hat keine Waffen.`], kurz: "unbewaffnet" };
  if (rang === null) return { li: [`${esc(ey.name)} ist für ${esc(ex.name)} <b>nicht erfassbar</b> – kein Schuss.`], kurz: "nicht erfassbar" };
  const li = [];
  li.push(`<b>${RANG[rang]}</b>${rang === 2 ? " – nur wenn kein Primär- oder Sekundärziel lebt; hier ist es das einzige Ziel, also Grundschaden ohne Schnellfeuer" : ""} · Trefferfaktor <b>×${fz(f)}</b>${bevorzugt && sf > 1 ? ` · Schnellfeuer <b>${sf}</b>` : ""}${bevorzugt && sfk !== 1 ? ` · Schildbrecher ×${fz(sfk)}` : ""}`);
  li.push(`Ein Treffer: ${fmt(angr)} Angriff × ${fz(f)} = <b>${fmt(dmg)}</b> Schaden. Ziel: Schild ${fmt(S)} (lädt jede Runde neu) und Struktur ${fmt(St)}.`);
  const verpufft = dmg * sfk < S * M.verpuffen;
  let k = null;
  if (verpufft) li.push(`<b>Der Treffer verpufft</b>: Er liegt unter ${pct(M.verpuffen)} des Schildwerts (${fmt(S * M.verpuffen)}) – ${esc(ex.name)} kann ${esc(ey.name)} nicht verletzen.`);
  else {
    k = Math.max(1, Math.ceil((S / sfk + St) / dmg));
    const kx = Math.max(1, Math.ceil((S / sfk + (1 - M.explosion) * St) / dmg));
    li.push(k === 1 ? `<b>Ein Treffer zerstört eine Einheit</b>; überschüssiger Schaden verfällt${bevorzugt && sf > 1 && M.mehrfachfeuer ? ", das Schnellfeuer springt aber weiter" : ""}.`
      : `Für einen Abschuss in einer Runde braucht es <b>${k} Treffer</b>; ab ${kx} Treffer${kx > 1 ? "n" : ""} kann die Einheit explodieren (unter ${pct(M.explosion)} Struktur). Strukturschaden bleibt über die Runden, der Schild lädt nach.`);
  }
  const schuss = bevorzugt ? sf : 1;
  li.push(`Je Runde: ${nx ? fmt(nx) + " × " : ""}${kanal(ex.kanaele)} (je Kanal ein anderes Ziel)${schuss > 1 ? ` × Ø ${schuss} Schüsse` : ""}${nx ? ` = Ø <b>${fmt(nx * ex.kanaele * schuss)}</b> Treffer, solange genug Ziele leben` : ""}.` +
    (schuss > 1 ? ` Schnellfeuer ${sf}: nach jedem Schuss mit ${pct((sf - 1) / sf)} ein weiterer${M.mehrfachfeuer ? "; fällt das Ziel, springt die Salve auf das nächste derselben Art (Mehrfachfeuer)" : "; endet mit dem Abschuss"}.` : ""));
  const kurz = verpufft ? "Treffer verpufft" : (bevorzugt ? RANG[rang] : "Reserveziel") + (schuss > 1 ? ` mit Schnellfeuer ${sf}` : "") + (k ? ` (${k === 1 ? "1 Treffer" : k + " Treffer"} je Abschuss)` : "");
  return { li, kurz };
}
function wortTausch(name, t, v) {
  const r = Math.exp(Math.abs(t));
  if (t >= 0 && v[0] < 0.005) return `${esc(name)} verliert fast nichts und zerstört ${pct(v[1])} des Gegners`;
  if (t < 0 && v[1] < 0.005) return `${esc(name)} richtet fast nichts aus und verliert ${pct(v[0])}`;
  if (rf(r) === "=") return `beide Seiten verlieren etwa gleich viel Wert`;
  return t >= 0 ? `${esc(name)} zerstört ${wie(r)} so viel Wert, wie es verliert` : `${esc(name)} verliert ${wie(r)} so viel Wert, wie es zerstört`;
}
/* Werte einer Einheit mit Messleisten */
function wertleiste(e, technik) {
  const x = E[e], w = technik ? x.kampf : x.basis;
  return `<dl class="werte">${["Angriff", "Schild", "Struktur"].map((n, i) => `<div class="wert"><dt>${n}</dt><dd><b>${fmt(w[i])}</b>${mess(x.basis[i], MAX[i])}</dd></div>`).join("")}
    <div class="wert"><dt>Wert</dt><dd><b>${fmt(x.wert)}</b><small>ab Stufe ${x.stufe}</small></dd></div><div class="wert"><dt>Ziele je Runde</dt><dd><b>${x.kanaele}</b><small>${x.schildfaktor !== 1 ? "Schildbrecher ×" + fz(x.schildfaktor) : "Zielkanäle"}</small></dd></div></dl>`;
}
const zielchips = (e, l, std) => chips(l.map(z => ({ e: z, wert: [(Math.abs((E[e].zw[z] || [0, std])[1] - std) > 1e-9 ? "×" + fz(E[e].zw[z][1]) : ""), E[e].sf[z] ? "SF " + E[e].sf[z] : ""].filter(Boolean).join(" · "),
  tip: `${N(z)}: Trefferfaktor ×${fz((E[e].zw[z] || [0, std])[1])}${E[e].sf[z] ? ", Schnellfeuer " + E[e].sf[z] : ""}` })), "", o => T(e, o.e) !== null ? `paar:${e}:${o.e}` : `einheit:${e}`);

/* ---------- Ansichten im Blatt ---------- */
const ANSICHT = {
  paar(a, b) {
    const p = paar(a, b), ea = E[a], eb = E[b];
    if (!p) return { leiste: "Kein Gefecht", html: "<p>Anlagen kämpfen nie gegeneinander.</p>" };
    const wa = p.n[0] * ea.wert, wb = p.n[1] * eb.wert, gleich = a === b;
    const art = p.art === "verteidigung" ? `Angriff auf Verteidigung · ${N(p.angreifer)} greift an` : p.art === "weltenkruemmer" ? "Weltenkrümmer gegen den gleichen Wert" : gleich ? "Spiegelgefecht" : "Flottenkampf · beide feuern gleichzeitig";
    const runden = ["Start", ...p.aktiv.map((_, i) => "R" + (i + 1))];
    const ra = p.rest[0].map(v => v / p.n[0]), rb = p.rest[1].map(v => v / p.n[1]);
    const tips = (r, n, name) => r.map((v, i) => `${name} ${i === 0 ? "zu Beginn" : "nach Runde " + i}: Ø ${fmt(v * n, 1)} von ${fmt(n)} (${pct(v)})${i > 0 ? " · in " + pct(p.aktiv[i - 1]) + " der Läufe wird diese Runde gekämpft" : ""}`);
    const ma = mechanik(a, b, p.n[0]), mb = mechanik(b, a, p.n[1]);
    const urteil = gleich ? "Spiegelgefecht" : klar(p) ? (p.t > 0 ? `${ea.name} ist klarer Konter` : `${eb.name} ist klarer Konter`) : Math.abs(p.t) >= Math.log(1.4) ? `knapper Vorteil für ${p.t > 0 ? ea.name : eb.name}` : "offen";
    const seite = (e, n, w, cls) => `<div class="seite ${cls}">${img(e)}<span class="name">${esc(E[e].name)}</span><span class="unter">${fmt(n)} Stück · Wert ${mio(w)}</span></div>`;
    const runde = s => p.feuer.map((r, i) => r[s][0] ? `<tr><th>R${i + 1}</th><td class="z">${fmt(r[s][0], 1)}<small>Schüsse</small></td><td class="z">${fmt(r[s][2])} · ${fmt(r[s][3])}<small>Schild · Struktur</small></td><td class="z"><b>${fmt(r[s][4], 1)}</b><small>zerstört${r[s][5] >= 0.05 ? ", davon " + fmt(r[s][5], 1) + " Explosion" : ""}</small></td></tr>` : "").join("") || `<tr><td colspan="4" class="leise">kein Schuss</td></tr>`;
    const tabelle = (s, e) => `<div class="tbl-wrap"><table class="tbl"><thead><tr><th colspan="4"><span class="einh">${img(e)}${esc(E[e].name)} feuert</span></th></tr></thead><tbody>${runde(s)}</tbody></table></div>`;
    let fazit = gleich ? `Beide Seiten verlieren im Mittel ${pct(p.verlust[0])}.` : `${esc(ea.name)} trifft ${esc(eb.name)} als ${ma.kurz}; ${esc(eb.name)} trifft ${esc(ea.name)} als ${mb.kurz}.`;
    fazit += ` Im Mittel endet das Gefecht nach ${fmt(p.dauer, 1)} Runden.`;
    const html = `<div class="buehne duell">${hinter(bild(a), "l")}${hinter(bild(b), "r")}
        ${seite(a, p.n[0], wa, "a")}<span class="gegen">GEGEN</span>${seite(b, p.n[1], wb, "b")}</div>
      <p class="augenbraue" data-st="margin-top:12px">${esc(art)} · je ${mio(M.budget)} Werteinheiten · ${M.laeufe} Läufe</p>
      <div class="ergebnis"><span class="gross" data-st="color:${gleich ? "var(--text)" : p.t >= 0.14 ? "var(--gut2)" : p.t <= -0.14 ? "var(--rot2)" : "var(--text)"}">${gleich ? "=" : tx(p.t)}</span><span>${gleich ? "gleiche Einheit auf beiden Seiten" : wortTausch(ea.name, p.t, p.verlust)} – <b>${esc(urteil)}</b></span></div>
      ${stapelbalken(p.sieg, ea.name, eb.name)}
      <dl class="werte"><div class="wert"><dt>${esc(ea.name)} verliert</dt><dd><b>${pct(p.verlust[0])}</b>${mess(p.verlust[0], 1, "rot")}<small>${mio(p.verlust[0] * wa)}</small></dd></div>
        <div class="wert"><dt>${esc(eb.name)} verliert</dt><dd><b>${pct(p.verlust[1])}</b>${mess(p.verlust[1], 1, "rot")}<small>${mio(p.verlust[1] * wb)}</small></dd></div></dl>
      <div class="karte gold" data-st="margin-top:10px"><h4>Kurz gesagt</h4><p>${fazit}</p></div>
      <h3 class="block-h">Verlauf <small>Bestand nach jeder Runde, Mittel aller Läufe</small></h3>
      ${linien(gleich ? [{ name: ea.name, farbe: "#6fd6ff", werte: ra, tips: tips(ra, p.n[0], ea.name) }] : [{ name: ea.name, farbe: "#6fd6ff", werte: ra, tips: tips(ra, p.n[0], ea.name) }, { name: eb.name, farbe: "#e2bf7d", werte: rb, tips: tips(rb, p.n[1], eb.name) }], runden, { label: "Bestand je Runde" })}
      <h3 class="block-h">Wie ${esc(ea.name)} auf ${esc(eb.name)} schießt</h3><div class="karte"><p>${ma.li.join("<br>")}</p></div>
      ${gleich ? "" : `<h3 class="block-h">Wie ${esc(eb.name)} auf ${esc(ea.name)} schießt</h3><div class="karte gold"><p>${mb.li.join("<br>")}</p></div>`}
      <h3 class="block-h">Feuer je Runde <small>aus dem Kampfprotokoll, Mittel aller Läufe</small></h3>
      ${tabelle(0, a)}${gleich ? "" : tabelle(1, b)}
      <p class="leise">„Schild · Struktur“ sind tatsächlich abgezogene Punkte; Schaden über die Reststruktur hinaus und verpuffte Treffer zählen nicht. Wer in einer Runde fällt, schießt in ihr noch.</p>
      <h3 class="block-h">Streuung <small>jeder der ${M.laeufe} Läufe</small></h3>
      <div class="streu">${streuung(p.laeufe, "Verlust " + ea.name, "Verlust " + eb.name, { sieger: { a: ea.name + " siegt", d: eb.name + " siegt", "-": "offen nach 6 Runden" } })}
      <p class="leise">Jeder Punkt ein Lauf, größer = mehrere gleiche. Über der gestrichelten Linie verliert ${esc(eb.name)} anteilig mehr – bei gleichem Einsatz tauscht dann ${esc(ea.name)} besser.
      Farbe: Cyan = ${esc(ea.name)} siegt, Gold = ${esc(eb.name)} siegt, grau = offen nach 6 Runden.</p></div>
      <div class="knoepfe">${gleich ? "" : `<button class="cta klein" data-open="paar:${b}:${a}">Aus Sicht von ${esc(eb.name)}</button>`}<button class="cta klein" data-open="einheit:${a}">Dossier ${esc(ea.name)}</button>${gleich ? "" : `<button class="cta klein" data-open="einheit:${b}">Dossier ${esc(eb.name)}</button>`}</div>`;
    return { leiste: `Gefecht · ${ea.name} gegen ${eb.name}`, html };
  },
  paarV(a, b, L, R) {
    const p = paarV(a, b, L, R), p0 = paar(a, b), ea = E[a], eb = E[b];
    if (!p) return { leiste: "Kein Gefecht", html: "<p>Anlagen kämpfen nie gegeneinander.</p>" };
    const wa = p.n[0] * ea.wert, wb = p.n[1] * eb.wert, fa = VF(L), fb = VF(R), d = p.t - p0.t;
    const ma = mechanik(a, b, p.n[0], fa, fb), mb = mechanik(b, a, p.n[1], fb, fa);
    const seite = (e, v, n, w, cls) => `<div class="seite ${cls}">${imgV(e, v)}<span class="name">${esc(E[e].name)}</span><span class="unter" data-st="color:${VFARBE(v)}">${esc(VN(v))}</span><span class="unter">${fmt(n)} Stück · Wert ${mio(w)}</span></div>`;
    const urteil = klar(p) ? `${p.t > 0 ? ea.name : eb.name} (${VN(p.t > 0 ? L : R)}) ist klarer Konter` : Math.abs(p.t) >= Math.log(1.4) ? `knapper Vorteil für ${p.t > 0 ? ea.name : eb.name}` : "offen";
    const ursachen = [fa.w !== 1 ? `${VN(L)}-Waffen ×${fz(fa.w)} verändern jeden Treffer von ${ea.name}` : "", fb.p !== 1 ? `${VN(R)}-Panzerung ×${fz(fb.p)} verändert die Struktur von ${eb.name}` : "",
      fb.w !== 1 ? `${VN(R)}-Waffen ×${fz(fb.w)} verändern jeden Treffer von ${eb.name}` : "", fa.p !== 1 ? `${VN(L)}-Panzerung ×${fz(fa.p)} verändert die Struktur von ${ea.name}` : ""].filter(Boolean);
    const runden = ["Start", ...p.rest[0].slice(1).map((_, i) => "R" + (i + 1))];
    const ra = p.rest[0].map(x => x / p.n[0]), rb = p.rest[1].map(x => x / p.n[1]);
    const zeilen = p.feuer.map((r, i) => r[0][0] || r[1][0] ? `<tr><th>R${i + 1}</th><td class="z">${fmt(r[0][0], 1)}<small>Schüsse ${esc(ea.name)}</small></td><td class="z"><b>${fmt(r[0][1], 1)}</b><small>zerstört</small></td><td class="z">${fmt(r[1][0], 1)}<small>Schüsse ${esc(eb.name)}</small></td><td class="z"><b>${fmt(r[1][1], 1)}</b><small>zerstört</small></td></tr>` : "").join("");
    const html = `<div class="buehne duell">${hinter(bildV(a, L), "l")}${hinter(bildV(b, R), "r")}
        ${seite(a, L, p.n[0], wa, "a")}<span class="gegen">GEGEN</span>${seite(b, R, p.n[1], wb, "b")}</div>
      <p class="augenbraue" data-st="margin-top:12px">Volk gegen Volk · ${esc(VN(L))} gegen ${esc(VN(R))} · je ${mio(M.budget)} Werteinheiten · ${M.laeufe} Läufe</p>
      <div class="ergebnis"><span class="gross" data-st="color:${p.t >= 0.14 ? "var(--gut2)" : p.t <= -0.14 ? "var(--rot2)" : "var(--text)"}">${tx(p.t)}</span><span>${wortTausch(ea.name, p.t, p.verlust)} – <b>${esc(urteil)}</b></span></div>
      ${stapelbalken(p.sieg, ea.name, eb.name)}
      <dl class="werte"><div class="wert"><dt>${esc(ea.name)} verliert</dt><dd><b>${pct(p.verlust[0])}</b>${mess(p.verlust[0], 1, "rot")}<small>${mio(p.verlust[0] * wa)}</small></dd></div>
        <div class="wert"><dt>${esc(eb.name)} verliert</dt><dd><b>${pct(p.verlust[1])}</b>${mess(p.verlust[1], 1, "rot")}<small>${mio(p.verlust[1] * wb)}</small></dd></div>
        <div class="wert"><dt>Ohne Volksboni</dt><dd><b class="${kl(p0.t)}">${tx(p0.t)}</b><small>Gleichstand der Völker</small></dd></div>
        <div class="wert"><dt>Volkseffekt</dt><dd><b class="${kl(d)}">${Math.abs(d) < 0.05 ? "kaum" : (d > 0 ? "▲ " : "▼ ") + fmt(Math.exp(Math.abs(d)), 2) + "×"}</b><small>${d > 0.05 ? "besser für " + esc(VN(L)) : d < -0.05 ? "besser für " + esc(VN(R)) : "Völker gleichen sich aus"}</small></dd></div></dl>
      <div class="karte gold"><h4>Was die Völker hier ändern</h4><p>${ursachen.length ? ursachen.map(esc).join(". ") + "." : "Beide Völker kämpfen ohne Kampfbonus – das Ergebnis entspricht dem Gleichstand."}</p></div>
      <h3 class="block-h">Kampfwerte mit Technik 6 und Volk</h3>
      <div class="tbl-wrap"><table class="tbl"><thead><tr><th></th><th class="z">Angriff</th><th class="z">Schild</th><th class="z">Struktur</th></tr></thead><tbody>
        <tr><th><span class="einh">${imgV(a, L)}${esc(ea.name)} · ${esc(VN(L))}</span></th>${kwV(a, L).map(x => `<td class="z">${fmt(x)}</td>`).join("")}</tr>
        <tr><th><span class="einh">${imgV(b, R)}${esc(eb.name)} · ${esc(VN(R))}</span></th>${kwV(b, R).map(x => `<td class="z">${fmt(x)}</td>`).join("")}</tr></tbody></table></div>
      <h3 class="block-h">Verlauf <small>Bestand nach jeder Runde, Mittel aller Läufe</small></h3>
      ${linien([{ name: ea.name, farbe: "#6fd6ff", werte: ra }, { name: eb.name, farbe: "#e2bf7d", werte: rb }], runden, { label: "Bestand je Runde" })}
      <h3 class="block-h">Wie ${esc(ea.name)} auf ${esc(eb.name)} schießt</h3><div class="karte"><p>${ma.li.join("<br>")}</p></div>
      <h3 class="block-h">Wie ${esc(eb.name)} auf ${esc(ea.name)} schießt</h3><div class="karte gold"><p>${mb.li.join("<br>")}</p></div>
      <h3 class="block-h">Feuer je Runde <small>aus dem Kampfprotokoll, Mittel aller Läufe</small></h3>
      <div class="tbl-wrap"><table class="tbl"><tbody>${zeilen}</tbody></table></div>
      <div class="knoepfe">${a === b && L === R ? "" : `<button class="cta klein" data-open="paarV:${b}:${a}:${R}:${L}">Aus Sicht von ${esc(eb.name)} (${esc(VN(R))})</button>`}<button class="cta klein" data-open="paar:${a}:${b}">Ohne Volksboni ansehen</button><button class="cta klein" data-open="einheit:${a}">Dossier ${esc(ea.name)}</button></div>`;
    return { leiste: `Volk gegen Volk · ${ea.name} gegen ${eb.name}`, html };
  },
  klassenV(g1, g2, L, R) {
    const a = D.gruppen.find(g => g.key === g1), b = D.gruppen.find(g => g.key === g2);
    let s = `<h2 data-st="margin:6px 0 4px;font:700 20px var(--schrift)">${esc(a.name)} (${esc(VN(L))}) gegen ${esc(b.name)} (${esc(VN(R))})</h2><p class="leise">Jede Zelle ein Gefecht aus Sicht der Zeile; klein darunter der Volkseffekt gegenüber dem Gleichstand.</p>
      <div class="kgrid" data-st="grid-template-columns:auto repeat(${b.e.length},minmax(auto,1fr))"><span></span>${b.e.map(z => `<span class="h oben">${imgV(z, R)}${esc(E[z].name)}</span>`).join("")}`;
    for (const x of a.e) {
      s += `<span class="h">${imgV(x, L)}${esc(E[x].name)}</span>`;
      for (const z of b.e) { const tv = TV(x, z, L, R); if (tv === null) { s += "<span></span>"; continue; } const dd = tv - T(x, z);
        s += `<button class="k" data-st="background:${farbe(tv)};${dunkel(tv) ? "color:#04121d" : ""}" data-open="paarV:${x}:${z}:${L}:${R}" data-tip="${esc(N(x) + " (" + VN(L) + ") gegen " + N(z) + " (" + VN(R) + "): " + tx(tv) + " · ohne Volksboni " + tx(T(x, z)))}">${x === z && L === R ? "=" : tf(tv)}<small>${Math.abs(dd) < 0.05 ? "·" : (dd > 0 ? "▲" : "▼") + fmt(Math.exp(Math.abs(dd)), 2)}</small></button>`; }
    }
    return { leiste: `Volk gegen Volk · ${a.name} gegen ${b.name}`, html: s + "</div>" };
  },
  einheit(e) {
    const x = E[e];
    const gegner = (VERT.has(e) ? SCHIFFE : R).filter(z => z !== e);
    const zeilen = gegner.map(z => ({ z, t: T(e, z) })).filter(o => o.t !== null).sort((p, q) => q.t - p.t);
    const hatGs = x.gs !== undefined && x.gs !== null, hatGa = x.ga !== undefined && x.ga !== null;
    const pillen = `<span class="pille">Wert <b>${fmt(x.wert)}</b></span><span class="pille">ab Stufe <b>${x.stufe}</b></span>${x.belagerer ? '<span class="pille"><b>Belagerer</b></span>' : ""}`;
    const sieg = (x.schlaegt || []).map(([z, t]) => ({ e: z, wert: tx(t), tip: `${x.name} gegen ${N(z)}: ${tx(t)}` }));
    const nied = (x.konter || []).map(([z, t]) => ({ e: z, wert: tx(t), tip: `${N(z)} gegen ${x.name}: ${tx(t)}` }));
    const html = buehne(e, `${esc(x.klasse)}${x.belagerer ? " · Belagerer" : ""}`, x.name, pillen) + `
      <dl class="werte"><div class="wert"><dt>Angriff</dt><dd><b>${fmt(x.basis[0])}</b>${mess(x.basis[0], MAX[0])}<small>Technik 6 · ${esc(VN(volk))}: ${fmt(kwV(e, volk)[0])}${VF(volk).w !== 1 ? " (×" + fz(VF(volk).w) + ")" : ""}</small></dd></div>
        <div class="wert"><dt>Schild</dt><dd><b>${fmt(x.basis[1])}</b>${mess(x.basis[1], MAX[1])}<small>Technik 6: ${fmt(kwV(e, volk)[1])}</small></dd></div>
        <div class="wert"><dt>Struktur</dt><dd><b>${fmt(x.basis[2])}</b>${mess(x.basis[2], MAX[2])}<small>Technik 6 · ${esc(VN(volk))}: ${fmt(kwV(e, volk)[2])}${VF(volk).p !== 1 ? " (×" + fz(VF(volk).p) + ")" : ""}</small></dd></div>
        <div class="wert"><dt>Ziele je Runde</dt><dd><b>${x.kanaele}</b><small>${x.schildfaktor !== 1 ? "Schildbrecher ×" + fz(x.schildfaktor) : "Zielkanäle"}</small></dd></div></dl>
      <div class="karte"><h4>Rolle</h4><p>${esc(x.rolle)}</p></div>
      ${hatGs ? `<dl class="werte"><div class="wert"><dt>Ø gegen ${VERT.has(e) ? "alle Kampfschiffe" : "Schiffe"}</dt><dd><b class="${kl(x.gs)}">${txw(x.gs)}</b></dd></div>${hatGa ? `<div class="wert"><dt>Ø gegen Anlagen</dt><dd><b class="${kl(x.ga)}">${txw(x.ga)}</b></dd></div>` : ""}</dl>` : ""}
      ${x.schlaegt ? `<h3 class="block-h">Konter</h3><p class="reihe gut">Schlägt klar</p>${chips(sieg, "gut", o => `paar:${e}:${o.e}`)}<p class="reihe schlecht">Verwundbar durch</p>${chips(nied, "schlecht", o => `paar:${o.e}:${e}`)}` : ""}
      <h3 class="block-h">Zielwahl <small>Primär ×${fz(M.pf)}, Sekundär ×${fz(M.sf)}; Schnellfeuer nur hier</small></h3>
      <p class="reihe" data-st="color:var(--cyan2)">Primärziele</p>${zielchips(e, x.primaer, M.pf)}
      <p class="reihe" data-st="color:var(--cyan2)">Sekundärziele</p>${zielchips(e, x.sekundaer, M.sf)}
      <p class="leise">${VERT.has(e) ? "Andere Schiffe nur als Reserve mit Grundschaden; Anlagen sind nicht erfassbar." : "Alles andere nur als Reserve mit Grundschaden, ohne Schnellfeuer – wenn kein Primär- oder Sekundärziel mehr lebt."}</p>
      ${steckbrief(e, volk)}
      <h3 class="block-h">Alle Gefechte <small>vom besten zum schlechtesten Tausch, ohne Volksboni</small></h3>
      <div class="profil liste">${tickzeile(T20, 3, "Gegner", "Tausch")}${zeilen.map(o => balkenzeile(o.z, o.t, `paar:${e}:${o.z}`, `${x.name} gegen ${N(o.z)}: ${tx(o.t)}`)).join("")}</div>`;
    return { leiste: `Tech-Datenbank · ${VN(volk)} · ${x.name}`, html };
  },
  klassen(g1, g2) {
    const a = D.gruppen.find(g => g.key === g1), b = D.gruppen.find(g => g.key === g2);
    let s = `<h2 data-st="margin:6px 0 4px;font:700 20px var(--schrift)">${esc(a.name)} gegen ${esc(b.name)}</h2><p class="leise">Jede Zelle ein Gefecht, aus Sicht der Zeile. Antippen für das Gefecht im Detail.</p>
      <div class="kgrid" data-st="grid-template-columns:auto repeat(${b.e.length},minmax(auto,1fr))"><span></span>${b.e.map(z => `<button class="h oben" data-open="einheit:${z}">${img(z)}${esc(E[z].name)}</button>`).join("")}`;
    for (const x of a.e) {
      s += `<button class="h" data-open="einheit:${x}">${img(x)}${esc(E[x].name)}</button>`;
      for (const z of b.e) { const t = T(x, z); s += t === null ? "<span></span>" : x === z ? `<button class="k" data-st="background:#12233a" data-open="paar:${x}:${z}">=</button>` : `<button class="k" data-st="background:${farbe(t)};${dunkel(t) ? "color:#04121d" : ""}" data-open="paar:${x}:${z}" data-tip="${esc(N(x) + " gegen " + N(z) + ": " + tx(t))}">${tf(t)}</button>`; }
    }
    return { leiste: `Klassen · ${a.name} gegen ${b.name}`, html: s + "</div>" };
  },
  ueberfall(i) {
    const u = D.ueberfall[+i], vn = u.v === "igel" ? "Igel-Mischung" : N(u.v);
    const wert = m => Object.entries(m).reduce((s, [k, n]) => s + n * E[k].wert, 0);
    const ra = u.rest_a.map(m => wert(m) / u.wa), rd = u.rest_d.map(m => wert(m) / u.wd);
    const ende = (seite, k) => (seite === "a" ? u.rest_a : u.rest_d)[u.rest_a.length - 1][k] || 0;
    const kacheln = (m, s) => `<div class="kacheln">${Object.entries(m).map(([k, n]) => { const r = ende(s, k), v = 1 - r / n;
      return `<button class="kachel" data-open="einheit:${k}"><span class="bild">${img(k)}</span><span class="name zahl">${fmt(n)} → ${fmt(r, r < 10 ? 1 : 0)}</span><span class="zeile">${esc(N(k))}</span>${mess(1 - v, 1, v > 0.5 ? "rot" : "hoch")}<span class="zeile">${v < 0.005 ? "keine Verluste" : "−" + pct(v)}</span></button>`; }).join("")}</div>`;
    const ziele = (von, auf) => `<div class="tbl-wrap"><table class="tbl"><thead><tr><th>schießt auf →</th>${auf.map(z => `<th class="z" title="${esc(N(z))}">${esc(E[z].kurz)}</th>`).join("")}</tr></thead><tbody>` +
      von.map(x => `<tr><th><span class="einh">${img(x)}${esc(N(x))}</span></th>${auf.map(z => { const w = E[x].zw[z] || [null]; const ges = u.feuer.flat().filter(f => f.einheit === x && f.ziel === z); const sch = ges.reduce((s, f) => s + f.schuesse, 0), ab = ges.reduce((s, f) => s + f.zerstoert, 0);
        return `<td class="z" data-tip="${esc(N(x) + " → " + N(z) + ": " + (w[0] === null ? "nicht erfassbar" : RANG[w[0]] + (w[0] < 2 && w[2] > 1 ? ", Schnellfeuer " + w[2] : "")) + " · Ø " + fmt(sch, 1) + " Schüsse, " + fmt(ab, 1) + " Abschüsse")}"><b>${sch ? fmt(ab, 1) : "0"}</b><small>${w[0] === null ? "nicht erfassbar" : ["Primär", "Sekundär", "Reserve"][w[0]]} · ${fmt(sch)} Schüsse</small></td>`; }).join("")}</tr>`).join("") + "</tbody></table></div>";
    const ta = Object.keys(u.ang), tv = Object.keys(u.ver);
    const html = `<div class="buehne" data-st="min-height:190px">
        <div class="reihenbild" data-st="position:absolute;inset:10px 10px 74px;display:flex;justify-content:center;align-items:center">${tv.map(k => img(k)).join("")}</div>
        <div class="titel"><p class="augenbraue">Überfall · ${u.m}-fache Übermacht</p><h2>${esc(vn)}</h2><span class="pille">Verteidigung <b>${mio(u.wd)}</b></span><span class="pille">Raubflotte <b>${mio(u.wa)}</b></span></div></div>
      <div class="ergebnis"><span class="gross" data-st="color:${u.abschreckung >= 1 ? "var(--gold2)" : "var(--text)"}">${fmt(u.abschreckung, 2)}</span><span>Abschreckung: Der Räuber verliert Ø ${mio(u.abschreckung * u.wd)}, das ${fmt(u.abschreckung, 2)}-Fache des Verteidigungswerts. Die Verteidigung verliert ${pct(u.verlust_d)}; zerstörte Anlagen werden zu 70 % wieder aufgebaut.</span></div>
      ${stapelbalken(u.sieg, "Räuber", vn)}
      <h3 class="block-h">Raubflotte · Einheiten <small>Start → Ende, Mittel aller Läufe</small></h3>${kacheln(u.ang, "a")}
      <h3 class="block-h">${esc(vn)} · Einheiten</h3>${kacheln(u.ver, "d")}
      <h3 class="block-h">Verlauf <small>verbleibender Wert je Seite</small></h3>
      ${linien([{ name: "Raubflotte", farbe: "#6fd6ff", werte: ra }, { name: vn, farbe: "#e2bf7d", werte: rd }], ["Start", ...u.aktiv.map((_, k) => "R" + (k + 1))], { label: "Wert je Runde" })}
      <h3 class="block-h">Wer schießt auf wen <small>Abschüsse fett, darunter Zielrang und Schüsse</small></h3>
      ${ziele(ta, tv)}${ziele(tv, ta)}
      <h3 class="block-h">Streuung <small>jeder der ${M.laeufe} Läufe</small></h3>
      <div class="streu">${streuung(u.laeufe, "Räuberverlust ÷ Verteidigungswert", "Verteidigung verliert", { mx: Math.max(1, Math.ceil(Math.max(...u.laeufe.map(l => l[0])))), my: 1, diagonale: false, fmt: v => fmt(v, v > 0 && v < 1 ? 2 : 1), sieger: { a: "Räuber siegt", d: "Verteidigung hält", "-": "offen nach 6 Runden" } })}
      <p class="leise">Jeder Punkt ein Lauf. Cyan = Räuber siegt, Gold = Verteidigung hält, grau = offen nach 6 Runden.</p></div>`;
    return { leiste: `Überfall · ${vn} gegen ${u.m}-fache Raubflotte`, html };
  },
  rakete(key, ziel) {
    const r = D.raketen.find(x => x.key === key), z = r.ziele.find(x => x.e === ziel), ez = E[ziel];
    const zerst = z.stueck * ez.wert, top = Math.max(1.05, r.ziele[0].q * 1.05);
    const html = buehne("rakete_" + key, `Rakete · Silostufe ${r.silo} · gegen ${[r.planet ? "Verteidigung" : "", r.flotte ? "Flotten" : ""].filter(Boolean).join(" und ")}`, r.name,
      `<span class="pille">Schaden <b>${fmt(r.schaden)}</b></span><span class="pille">Wert <b>${fmt(r.wert)}</b></span><span class="pille">bis <b>${r.ziele_je}</b> Ziele</span>`) + `
      <div class="karte"><h4>Besonderheit</h4><p>${esc(r.text)}</p></div><div class="karte violett"><h4>Kosten je Rakete</h4><p>${kosten(r.kosten)} – Wert ${fmt(r.wert)}</p></div>
      <h3 class="block-h">Rechnung gegen ${esc(ez.name)}</h3>
      <div class="ablauf"><div class="karte"><h4>1 · Stück je Rakete</h4><p>Schaden ${fmt(r.schaden)} × Faktor ${fz(z.f)} ÷ Struktur ${fmt(ez.basis[2])} = <b>${fmt(z.roh, 2)}</b>${z.gedeckelt ? `, gedeckelt auf <b>${r.ziele_je}</b> Ziele je Rakete` : ""}. Schilde zählen nicht.</p></div>
      <div class="karte"><h4>2 · Abrechnung</h4><p>Je Salve ganzzahlig (⌊Treffer × Schaden × Faktor ÷ Struktur⌋): Eine einzelne Rakete zerstört ${fmt(Math.floor(z.stueck))} Stück, viele zusammen im Mittel ${fmt(z.stueck, 2)} je Rakete.</p></div>
      <div class="karte gold"><h4>3 · Lohnt es sich?</h4><p>${fmt(z.stueck, 2)} × ${fmt(ez.wert)} = <b>${fmt(zerst)}</b> zerstört je Rakete; je eigenem Wert ${fmt(zerst)} ÷ ${fmt(r.wert)} = <b>${fmt(z.q, 2)}</b>${z.q >= 1 ? " – mehr, als sie kostet" : " – weniger, als sie kostet; Raketen sind Erstschlag ohne Flottenverlust"}.</p></div>
      <div class="karte rot"><h4>Abfang</h4><p>${r.abfang} Abfangrakete${r.abfang > 1 ? "n" : ""} aus einem intakten Silo heben eine ${esc(r.name)} auf: ${fmt(r.abfang * D.abfang.wert)} gegen ${fmt(r.wert)} – der Verteidiger zahlt ${pct(r.abfang * D.abfang.wert / r.wert)} des Angriffs. ${esc(D.abfang.text)}</p></div></div>
      <h3 class="block-h">Alle Ziele <small>zerstörter Wert je Raketenwert</small></h3>
      <div class="profil">${r.ziele.map(x => `<div class="pr" data-open="rakete:${key}:${x.e}" tabindex="0"><span class="n">${img(x.e)}<span>${esc(N(x.e))}</span></span><span class="achse"><i class="eins" data-st="left:${100 / top}%"></i><span class="bar" data-st="left:0;width:${Math.max(.8, x.q / top * 100)}%;background:${x.q >= 1 ? "var(--gut)" : "#6c7f93"}"></span></span><span class="v"><b>${fmt(x.q, 2)}</b></span></div>`).join("")}</div>
      ${steckbrief(key, volk)}
      <p class="leise">Voraussetzung: ${Object.entries(r.braucht).map(([k, n]) => esc(k) + " " + n).join(", ")} · Flug mindestens ${r.flug[0] >= 60 ? fmt(r.flug[0] / 60) + " min" : fmt(r.flug[0]) + " s"}, je System ${fmt(r.flug[1])} s · Reichweite ${r.reichweite} Systeme je Silostufe</p>`;
    return { leiste: `Raketensilo · ${r.name} gegen ${ez.name}`, html };
  },
  boden(a, d) {
    const p = B.paare[a + "|" + d], ta = B.typen[a], td = B.typen[d], gleich = a === d;
    const ka = ta.faktor[d] / 1000, kd = td.faktor[a] / 1000;
    const tw = tausch(p.welle[1], p.welle[0]), te = tausch(p.ende[1], p.ende[0]);
    const sum = l => l.reduce((s, f) => s + f.schaden, 0);
    const runden = p.runden.map((r, i) => `<tr><th>R${i + 1}</th><td class="z">${fmt(sum(r.feuer[0]))}<small>Schaden ${esc(ta.kurz)}</small></td><td class="z"><b>${fmt(r.verlust[1])}</b><small>${esc(td.kurz)} fallen</small></td><td class="z">${fmt(sum(r.feuer[1]))}<small>Schaden ${esc(td.kurz)}</small></td><td class="z"><b>${fmt(r.verlust[0])}</b><small>${esc(ta.kurz)} fallen</small></td><td class="z">${fmt(r.rest[0])} / ${fmt(r.rest[1])}<small>Rest</small></td></tr>`).join("");
    const wl = p.verlauf.map(v => [v[0] / p.n[0], v[1] / p.n[1]]);
    const r1 = p.runden[0] || { feuer: [[], []] };
    const erkl = (x, y, nx, k, f) => `${esc(x.name)}: ${fmt(nx)} × Angriff ${fmt(x.angriff)} × Konterfaktor <b>×${fz(k)}</b> gegen ${esc(y.name)} = <b>${fmt(f)}</b> Schaden in Runde 1 → ${fmt(Math.floor(f / y.struktur))} ${esc(y.name)} fallen (Struktur ${fmt(y.struktur)}), der Rest bleibt als Verwundung.`;
    const seite = (e, t, n, cls) => `<div class="seite ${cls}">${img(e)}<span class="name">${esc(t.name)}</span><span class="unter">${fmt(n)} Stück · Angriff ${fmt(t.angriff)} · Struktur ${fmt(t.struktur)}</span></div>`;
    const html = `<div class="buehne duell">${hinter(bild(a), "l")}${hinter(bild(d), "r")}${seite(a, ta, p.n[0], "a")}<span class="gegen">GEGEN</span>${seite(d, td, p.n[1], "b")}</div>
      <p class="augenbraue" data-st="margin-top:12px">Bodenkampf · je ${mio(B.budget)} Werteinheiten · exakt, ohne Zufall</p>
      <dl class="werte"><div class="wert"><dt>Eine Welle (${B.runden_je_welle} Runden)</dt><dd><b class="${kl(tw)}">${gleich ? "=" : tx(tw)}</b></dd></div><div class="wert"><dt>Bis Kampfende (${p.wellen} Welle${p.wellen > 1 ? "n" : ""})</dt><dd><b class="${kl(te)}">${gleich ? "=" : tx(te)}</b></dd></div>
        <div class="wert"><dt>${esc(ta.name)} verliert</dt><dd><b>${pct(p.ende[0])}</b>${mess(p.ende[0], 1, "rot")}</dd></div><div class="wert"><dt>${esc(td.name)} verliert</dt><dd><b>${pct(p.ende[1])}</b>${mess(p.ende[1], 1, "rot")}</dd></div></dl>
      <h3 class="block-h">Warum</h3><div class="ablauf">${r1.feuer[0].length ? `<div class="karte"><p>${erkl(ta, td, p.n[0], ka, sum(r1.feuer[0]))}</p></div>` : ""}${gleich || !r1.feuer[1].length ? "" : `<div class="karte gold"><p>${erkl(td, ta, p.n[1], kd, sum(r1.feuer[1]))}</p></div>`}
      <div class="karte violett"><p>Beide Seiten feuern gleichzeitig; Verluste = (Schaden + Verwundung) ÷ Struktur, abgerundet. Zwischen den Wellen heilen Verwundete.</p></div></div>
      <h3 class="block-h">Erste Welle Runde für Runde</h3>
      <div class="tbl-wrap"><table class="tbl"><tbody>${runden}</tbody></table></div>
      ${p.wellen > 1 ? `<h3 class="block-h">Wellen bis Kampfende</h3>${linien([{ name: ta.name, farbe: "#6fd6ff", werte: [1, ...wl.map(v => v[0])] }, { name: td.name, farbe: "#e2bf7d", werte: [1, ...wl.map(v => v[1])] }], ["Start", ...wl.map((_, i) => "W" + (i + 1))], { label: "Bestand je Welle" })}` : ""}
      <div class="knoepfe">${gleich ? "" : `<button class="cta klein" data-open="boden:${d}:${a}">Aus Sicht von ${esc(td.name)}</button>`}<button class="cta klein" data-open="truppe:${a}">Dossier ${esc(ta.name)}</button>${gleich ? "" : `<button class="cta klein" data-open="truppe:${d}">Dossier ${esc(td.name)}</button>`}</div>`;
    return { leiste: `Bodenkampf · ${ta.name} gegen ${td.name}`, html };
  },
  truppe(a) {
    const t = B.typen[a], m = bmodus === "ende" ? t.mittel : t.mittel_welle;
    const zeilen = B.ids.filter(d => d !== a).map(d => ({ d, t: tausch(B.paare[a + "|" + d][bmodus][1], B.paare[a + "|" + d][bmodus][0]) })).sort((p, q) => q.t - p.t);
    const konter = Object.entries(t.faktor).filter(([z]) => B.typen[z] && B.typen[z].kampf).sort((p, q) => q[1] - p[1]);
    const html = buehne(a, `Bodentruppe · Kaserne ${t.kaserne} · ab Stufe ${t.stufe}`, t.name, `<span class="pille">Wert <b>${fmt(t.wert)}</b></span><span class="pille">Transportplätze <b>${t.plaetze}</b></span>`) + `
      <dl class="werte"><div class="wert"><dt>Angriff</dt><dd><b>${fmt(t.angriff)}</b></dd></div><div class="wert"><dt>Struktur</dt><dd><b>${fmt(t.struktur)}</b></dd></div><div class="wert"><dt>Ø Tausch (${bmodus === "ende" ? "bis Kampfende" : "eine Welle"})</dt><dd><b class="${kl(m)}">${txw(m)}</b></dd></div></dl>
      <div class="karte"><h4>Rolle</h4><p>${esc(t.rolle)}</p></div>
      <p class="reihe gut">Schlägt klar</p>${chips(t.schlaegt.map(([z, v]) => ({ e: z, wert: tx(v) })), "gut", o => `boden:${a}:${o.e}`)}
      <p class="reihe schlecht">Verwundbar durch</p>${chips(t.konter.map(([z, v]) => ({ e: z, wert: tx(v) })), "schlecht", o => `boden:${o.e}:${a}`)}
      <h3 class="block-h">Konterfaktoren <small>Schaden gegen …</small></h3>${chips(konter.map(([z, k]) => ({ e: z, wert: "×" + fz(k / 1000) })), "", o => `boden:${a}:${o.e}`)}
      ${steckbrief(a, volk)}
      <h3 class="block-h">Alle Gefechte <small>${bmodus === "ende" ? "bis Kampfende" : "eine Welle"}</small></h3><div class="profil liste">${tickzeile(T20, 3, "Gegner", "Tausch")}${zeilen.map(o => balkenzeile(o.d, o.t, `boden:${a}:${o.d}`, `${t.name} gegen ${N(o.d)}: ${tx(o.t)}`)).join("")}</div>`;
    return { leiste: `Kaserne · ${t.name}`, html };
  },
};

/* ---------- Lesebeispiele ---------- */
function lese() {
  const z = [["=", 1, 1, "ausgeglichen – beide verlieren gleich viel"], ["×1,4", 1, 1.4, "knapper Vorteil"], ["×2", 1, 2, "klarer Konter"],
    ["×5", 0.4, 2, "klarer Konter: zerstört fünfmal so viel, wie es verliert"], ["20+", 0.08, 2, "verliert fast nichts – erdrückend"], ["/3", 1.8, 0.6, "wird klar geschlagen: verliert dreimal so viel, wie es zerstört"]];
  $("#lese").innerHTML = z.map(([l, w, d, s]) => `<div class="lz"><b data-st="color:${d > w * 1.3 ? "var(--gut2)" : d < w / 1.3 ? "var(--rot2)" : "var(--text)"}">${l}</b>
    <span class="bb"><span><i data-st="width:${w * 70}px;background:var(--rot)"></i>verliert ${fmt(w, 1)} Mio.</span><span><i data-st="width:${d * 70}px;background:var(--gut)"></i>zerstört ${fmt(d, 1)} Mio.</span></span><small>${s}</small></div>`).join("");
}

/* ---------- Kopf ---------- */
function kopf() {
  $("#kopf-bilder").innerHTML = [["schlachtschiff", "f1"], ["kreuzer", "f2"], ["zerstoerer", "f3"], ["leichter_jaeger", "f4"], ["bomber", "f5"]].map(([e, k]) => img(e).replace("<img ", `<img class="${k}" `)).join("");
  for (const [id, b] of [["k-planet", BILD.grund.planet], ["k-mond", BILD.grund.mond], ["k-horizont", BILD.grund.planet]]) $("#" + id).style.backgroundImage = `url(${b})`;
  $("#kopf-meta").innerHTML = `Regelhash <b>${M.hash}</b> · je Paar <b>${M.laeufe}</b> Läufe · Technik <b>6</b> · ${M.mehrfachfeuer ? "Mehrfachfeuer an" : "ohne Mehrfachfeuer"}`;
  const gs = SCHIFFE.filter(e => !BEL.has(e)).map(e => Math.exp(E[e].gs));
  const hk = Math.min(...SCHIFFE.map(e => E[e].konter.length));
  const igel = D.ueberfall.findIndex(u => u.v === "igel" && u.m === 3), ig = D.ueberfall[igel];
  const mg = M.maengel.length + M.boden_maengel.length;
  $("#kpis").innerHTML = [
    [`${mg}`, mg ? "Konter-Mängel – Einzelheiten unter Methode" : "Konter-Mängel: jede Einheit hat klare Konter und klare Opfer, jedes Primärziel wird geschlagen (Flotte und Boden)", 'data-reiter="methode"'],
    [`×${fmt(Math.min(...gs), 2)} – ×${fmt(Math.max(...gs), 2)}`, "Spanne der Kampfschiffe: Ø Tausch gegen alle anderen (ohne Belagerer) – niemand dominiert", 'data-reiter="flotte"'],
    [`≥ ${hk}`, "klare Konter hat jedes Kampfschiff mindestens; jede Anlage hat ihren Belagerer", 'data-reiter="flotte"'],
    [`${fmt(ig.abschreckung, 2)}`, `Abschreckung: Eine dreifache Raubflotte siegt gegen einen Igel nur in ${pct(ig.sieg[0])} der Läufe und verliert mehr als dessen Wert`, `data-open="ueberfall:${igel}"`],
  ].map(([b, s, a]) => `<button class="kpi" ${a}><b>${b}</b><span>${s}</span></button>`).join("");
  W.querySelectorAll(".budget").forEach(x => x.textContent = fmt(M.budget));
}

/* ---------- Überblick ---------- */
function ueberblick() {
  const sel = $("#rad-wahl");
  sel.innerHTML = D.gruppen.map(g => `<optgroup label="${esc(g.name)}">${g.e.map(e => `<option value="${e}">${esc(E[e].name)}</option>`).join("")}</optgroup>`).join("");
  sel.onchange = () => { rad.e = sel.value; radZeichnen(); };
  segment("#rad-stufe", "stufe", rad, radZeichnen);
  radZeichnen();
  const G = D.gruppen;
  let h = `<div class="kgrid" data-st="grid-template-columns:auto repeat(${G.length},minmax(auto,1fr))"><span class="h oben" data-st="align-items:flex-start;text-align:left">Zeile ↓ gegen Spalte →</span>${G.map(g => `<span class="h oben">${img(g.e[0])}${esc(g.name)}</span>`).join("")}`;
  for (const a of G) {
    h += `<span class="h">${img(a.e[0])}${esc(a.name)}</span>`;
    for (const b of G) {
      const v = []; for (const x of a.e) for (const z of b.e) { if (x === z) continue; const t = T(x, z); if (t !== null) v.push(t); }
      if (!v.length) { h += "<span></span>"; continue; }
      const t = v.reduce((s, x) => s + x, 0) / v.length;
      h += `<button class="k" data-st="background:${farbe(t, 2)};${dunkel(t, 2) ? "color:#04121d" : ""}" data-open="klassen:${a.key}:${b.key}" data-tip="${esc(a.name + " gegen " + b.name + ": Ø " + tx(t) + " aus " + v.length + " Gefechten")}">${tf(t)}</button>`;
    }
  }
  $("#klassen").innerHTML = h + "</div>";
  $("#leg-klassen").innerHTML = `<span><span class="sw" data-st="background:var(--gut)"></span>Zeile gewinnt – 2,7 = zerstört 2,7-mal so viel, wie sie verliert</span><span><span class="sw" data-st="background:var(--rot)"></span>Zeile verliert – /2,7 = verliert 2,7-mal so viel, wie sie zerstört</span><span><span class="sw" data-st="background:var(--mid);border:1px solid var(--linie)"></span>ausgeglichen</span>`;
  const satz = [];
  for (const a of G) for (const b of G) {
    if (a === b || a.key === "verteidigung" || b.key === "verteidigung") continue;
    const v = []; for (const x of a.e) for (const z of b.e) v.push([x, z, T(x, z), paar(x, z)]);
    const t = v.reduce((s, x) => s + x[2], 0) / v.length;
    if (t < Math.log(1.8)) continue;
    const w = v.filter(x => x[2] >= M.hart && x[3].verlust[1] >= 0.25).length / v.length, l = v.filter(x => x[2] <= -M.hart && x[3].verlust[0] >= 0.25).length / v.length;
    const traeger = a.e.filter(x => b.e.reduce((s, z) => s + T(x, z), 0) / b.e.length >= M.hart), schwach = a.e.filter(x => b.e.reduce((s, z) => s + T(x, z), 0) / b.e.length <= -M.hart);
    satz.push([t, `<div class="karte gruen"><h4>${esc(a.name)} gegen ${esc(b.name)} · Ø ${tx(t)}</h4><p>${pct(w)} der Paarungen klar gewonnen, ${pct(l)} klar verloren${traeger.length ? ". Getragen von " + traeger.map(x => esc(E[x].name)).join(", ") : ""}${schwach.length ? ". Ausnahme: " + schwach.map(x => esc(E[x].name)).join(", ") + " verliert hier" : ""}.</p>
      <button class="cta klein" data-open="klassen:${a.key}:${b.key}">Alle ${v.length} Gefechte ansehen</button></div>`]);
  }
  const vg = G.find(g => g.key === "verteidigung"), normal = SCHIFFE.filter(e => !BEL.has(e));
  const tv = vg.e.reduce((s, d) => s + normal.reduce((q, z) => q + T(d, z), 0), 0) / (vg.e.length * normal.length);
  const tb = vg.e.reduce((s, d) => s + D.belagerer.reduce((q, z) => q + T(d, z), 0), 0) / (vg.e.length * D.belagerer.length);
  $("#tendenz").innerHTML = satz.sort((p, q) => q[0] - p[0]).map(x => x[1]).join("") +
    `<div class="karte gold"><h4>Verteidigung · Ø ${tx(tv)} gegen gewöhnliche Schiffe, Ø ${tx(tb)} gegen Belagerer</h4><p>Schiffe ohne Belagerungsauftrag treffen Anlagen nur als Reserve. Gegen die drei Belagerer ist es im Mittel offen, im Einzelnen aber eindeutig – siehe nächster Abschnitt.</p></div>`;
  belagerung($("#belagerung"), $("#belagerung-text"));
  $("#regeln").innerHTML = [
    ["Zielwahl", `Je Runde wählt jeder Zielkanal einer Einheit zuerst ein lebendes, noch nicht anvisiertes <b>Primärziel</b>, dann ein <b>Sekundärziel</b>, dann irgendein anderes Ziel (Reserve). Innerhalb einer Stufe ist die Wahl zufällig.`, ""],
    ["Trefferfaktoren", `Treffer auf Primärziele zählen <b>×${fz(M.pf)}</b>, auf Sekundärziele <b>×${fz(M.sf)}</b>; eigene, höhere Faktoren stehen im Dossier. Reserveziele bekommen Grundschaden.`, "gold"],
    ["Schnellfeuer", `Nach jedem Schuss feuert die Einheit mit Wahrscheinlichkeit (r−1)/r erneut – im Mittel r Schüsse, höchstens 64 je Kanal, nur gegen Primär- und Sekundärziele.`, ""],
    ["Mehrfachfeuer", M.mehrfachfeuer ? `Fällt das Ziel, springt die Salve auf das nächste Ziel derselben Art. So vernichtet eine Salve viele schwache Ziele in einer Runde – darum knacken Bomber Werferfelder und Kreuzer Jägerschwärme.` : `Schnellfeuer endet mit dem Abschuss des Ziels.`, "gold"],
    ["Schild und Struktur", `Schilde fangen je Runde zuerst ab und laden voll nach; ein Treffer unter ${pct(M.verpuffen)} des Schildwerts verpufft. Strukturschaden bleibt; unter ${pct(M.explosion)} Struktur kann eine Einheit explodieren.`, "violett"],
    ["Ablauf", `Beide Seiten feuern gleichzeitig – wer in einer Runde fällt, schießt in ihr noch. Höchstens ${M.runden} Runden; wer danach steht, behält seine Einheiten. Zerstörte Anlagen werden zu 70 % wieder aufgebaut; 30 % der Erz- und Kristallkosten zerstörter Schiffe und verlorener Anlagen werden Trümmerfeld.`, "violett"],
  ].map(([h, p, c]) => `<div class="karte ${c}"><h4>${h}</h4><p>${p}</p></div>`).join("");
}
function belagerung(el, txt) {
  const vert = [...VERT];
  let h = `<div class="kgrid" data-st="grid-template-columns:auto repeat(${vert.length},minmax(auto,1fr))"><span class="h oben" data-st="align-items:flex-start;text-align:left">Belagerer ↓ gegen Anlage →</span>${vert.map(d => `<button class="h oben" data-open="einheit:${d}">${img(d)}${esc(E[d].name)}</button>`).join("")}`;
  for (const b of D.belagerer) {
    h += `<button class="h" data-open="einheit:${b}">${img(b)}${esc(E[b].name)}</button>`;
    for (const d of vert) { const t = T(b, d), p = paar(b, d); h += `<button class="k" data-st="background:${farbe(t)};${dunkel(t) ? "color:#04121d" : ""}" data-open="paar:${b}:${d}" data-tip="${esc(E[b].name + " gegen " + E[d].name + ": " + tx(t) + " · Verlust " + pct(p.verlust[0]) + " / " + pct(p.verlust[1]))}">${tf(t)}</button>`; }
  }
  el.innerHTML = h + "</div>";
  if (txt) txt.innerHTML = D.belagerer.map(b => {
    const gut = vert.filter(d => T(b, d) >= M.hart), schlecht = vert.filter(d => T(b, d) <= -M.hart);
    return `<div class="karte"><h4>${esc(E[b].name)}</h4><p class="reihe gut" data-st="margin-top:4px">Bricht</p>${chips(gut.map(d => ({ e: d, wert: tx(T(b, d)) })), "gut", o => `paar:${b}:${o.e}`, "keine Anlage allein")}
      ${schlecht.length ? `<p class="reihe schlecht">Fällt gegen</p>${chips(schlecht.map(d => ({ e: d, wert: tx(T(b, d)) })), "schlecht", o => `paar:${b}:${o.e}`)}` : ""}</div>`;
  }).join("");
}
/* Konter-Rad: alle 25 Einheiten als Motive im Kreis, Linien zu den klaren (oder knappen) Ergebnissen der gewählten Einheit */
const rad = { e: "leichter_jaeger", stufe: "hart" };
function radZeichnen() {
  const W = 720, H = 720, CX = 360, CY = 360, RN = 236, RG = 284, RL = 301;
  const klein = matchMedia("(max-width:640px)").matches, rk = klein ? 22 : 21, rs = klein ? 29 : 29;
  const plaetze = []; D.gruppen.forEach(g => { g.e.forEach(e => plaetze.push(e)); plaetze.push(null); });
  const n = plaetze.length, step = 2 * Math.PI / n, off = -Math.PI / 2 - step * (D.gruppen[0].e.length - 1) / 2;
  const pos = {}; plaetze.forEach((e, i) => { if (e) { const w = off + i * step; pos[e] = [CX + RN * Math.cos(w), CY + RN * Math.sin(w), w]; } });
  const s0 = rad.e, weich = rad.stufe === "weich", grenze = weich ? Math.log(1.4) : M.hart, mind = weich ? 0.15 : 0.25;
  const sieg = [], nied = [];
  for (const z of R) {
    if (z === s0) continue; const p = paar(s0, z); if (!p || (VERT.has(s0) && VERT.has(z))) continue;
    if (p.t >= grenze && p.verlust[1] >= mind) sieg.push([z, p.t, klar(p), p]); else if (p.t <= -grenze && p.verlust[0] >= mind) nied.push([z, -p.t, klar(p), p]);
  }
  sieg.sort((a, b) => b[1] - a[1]); nied.sort((a, b) => b[1] - a[1]);
  const kurve = (a, b) => { const [x1, y1] = pos[a], [x2, y2] = pos[b]; const cx = CX + ((x1 + x2) / 2 - CX) * 0.2, cy = CY + ((y1 + y2) / 2 - CY) * 0.2; return `M${x1.toFixed(1)} ${y1.toFixed(1)} Q${cx.toFixed(1)} ${cy.toFixed(1)} ${x2.toFixed(1)} ${y2.toFixed(1)}`; };
  let s = `<svg viewBox="0 0 ${W} ${H}" width="100%" role="img" aria-label="Konter-Rad für ${esc(E[s0].name)}"><defs><radialGradient id="knotengrund" cx="50%" cy="60%" r="60%"><stop offset="0" stop-color="#1b3550"/><stop offset="1" stop-color="#050b14"/></radialGradient></defs>`;
  D.gruppen.forEach(g => {
    const w1 = pos[g.e[0]][2] - step * 0.42, w2 = pos[g.e[g.e.length - 1]][2] + step * 0.42, wm = (w1 + w2) / 2;
    const p1 = [CX + RG * Math.cos(w1), CY + RG * Math.sin(w1)], p2 = [CX + RG * Math.cos(w2), CY + RG * Math.sin(w2)];
    s += `<path d="M${p1[0].toFixed(1)} ${p1[1].toFixed(1)} A${RG} ${RG} 0 0 1 ${p2[0].toFixed(1)} ${p2[1].toFixed(1)}" fill="none" stroke="#6fd6ff66" stroke-width="2" stroke-linecap="round"/>`;
    // Klassenname als Bogenschrift: oben im Uhrzeigersinn, unten gegenläufig, damit er immer aufrecht steht
    const unten = Math.sin(wm) > 0.05, r2 = unten ? RL + 9 : RL, q1 = [CX + r2 * Math.cos(w1 - step), CY + r2 * Math.sin(w1 - step)], q2 = [CX + r2 * Math.cos(w2 + step), CY + r2 * Math.sin(w2 + step)];
    const bogen = unten ? `M${q2[0].toFixed(1)} ${q2[1].toFixed(1)} A${r2} ${r2} 0 0 0 ${q1[0].toFixed(1)} ${q1[1].toFixed(1)}` : `M${q1[0].toFixed(1)} ${q1[1].toFixed(1)} A${r2} ${r2} 0 0 1 ${q2[0].toFixed(1)} ${q2[1].toFixed(1)}`;
    s += `<path id="kb-${g.key}" d="${bogen}" fill="none"/><text font-size="12" font-weight="600" fill="#a9c4dc" letter-spacing="1.6"><textPath href="#kb-${g.key}" startOffset="50%" text-anchor="middle">${esc(g.name.toUpperCase())}</textPath></text>`;
  });
  const linie = ([z, t, k], farbe, von, nach) => { const d = kurve(s0, z), b = 1.6 + 3.4 * Math.min(1, t / 3);
    return `<path d="${d}" fill="none" stroke="${farbe}" stroke-width="${b.toFixed(1)}" stroke-opacity="${k ? .9 : .7}" stroke-linecap="round"${k ? "" : ' stroke-dasharray="5 5"'}/><path d="${d}" fill="none" stroke="transparent" stroke-width="12" data-st="cursor:pointer" data-open="paar:${von}:${nach}" data-tip="${esc(N(von) + " schlägt " + N(nach) + (k ? " klar" : " knapp") + ": zerstört " + wie(Math.exp(t)) + " so viel Wert, wie es verliert")}"/>`; };
  s += nied.map(x => linie(x, "#e8697a", x[0], s0)).join("") + sieg.map(x => linie(x, "#6fd39a", s0, x[0])).join("");
  const sS = new Set(sieg.map(x => x[0])), sN = new Set(nied.map(x => x[0]));
  for (const e of R) {
    const [x, y] = pos[e], ist = e === s0, an = ist || sS.has(e) || sN.has(e), r = ist ? rs : rk;
    const rand = ist ? "#ffe2a3" : sS.has(e) ? "#6fd39a" : sN.has(e) ? "#e8697a" : "#a9c1db55";
    s += `<g class="kn" data-rad="${e}" opacity="${an ? 1 : .45}" tabindex="0" data-tip="${esc(E[e].name + (ist ? " (gewählt)" : sS.has(e) ? " – wird geschlagen" : sN.has(e) ? " – schlägt " + E[s0].name : ""))}">
      <circle cx="${x.toFixed(1)}" cy="${y.toFixed(1)}" r="${r}" fill="url(#knotengrund)"/><image href="${bild(e)}" x="${(x - r * 0.71).toFixed(1)}" y="${(y - r * 0.71).toFixed(1)}" width="${(r * 1.42).toFixed(1)}" height="${(r * 1.42).toFixed(1)}" preserveAspectRatio="xMidYMid meet"/>
      <circle class="ring" cx="${x.toFixed(1)}" cy="${y.toFixed(1)}" r="${r}" fill="none" stroke="${rand}" stroke-width="${ist ? 3 : an ? 2.4 : 1.2}"/>
      <text x="${(CX + (RN + r + 12) * Math.cos(pos[e][2])).toFixed(1)}" y="${(CY + (RN + r + 12) * Math.sin(pos[e][2]) + 4).toFixed(1)}" text-anchor="middle" font-size="${klein ? 13 : 11}" font-weight="700" fill="${ist ? "#ffe2a3" : "#dbe6f1"}">${E[e].kurz}</text></g>`;
  }
  s += `<rect x="${CX - 130}" y="${CY - 32}" width="260" height="60" rx="14" fill="#060d18eb" stroke="#e2bf7d73"/><text x="${CX}" y="${CY - 6}" text-anchor="middle" font-size="18" font-weight="700" fill="#fff">${esc(E[s0].name)}</text><text x="${CX}" y="${CY + 16}" text-anchor="middle" font-size="12.5" fill="#97abc0"><tspan fill="#8fe0b0">▲ schlägt ${sieg.length}</tspan>   <tspan fill="#ff9aa6">▼ verwundbar durch ${nied.length}</tspan></text></svg>`;
  $("#rad-svg").innerHTML = s;
  const x = E[s0];
  const bsp = (o, gut) => { if (!o) return ""; const [z, , , p] = o, verl = p.verlust[0] * p.n[0] * x.wert, zer = p.verlust[1] * p.n[1] * E[z].wert, f = Math.exp(Math.abs(p.t));
    return gut ? `Gegen <b>${esc(N(z))}</b> verliert ${esc(x.name)} ${mio(verl)} und zerstört ${mio(zer)} – <b class="gut">${tx(p.t)}</b>, also ${wie(f)} so viel, wie es verliert.`
      : `Gegen <b>${esc(N(z))}</b> verliert ${esc(x.name)} ${mio(verl)} und zerstört ${zer < 0.005 * M.budget ? "fast nichts" : mio(zer)} – <b class="schlecht">${tx(p.t)}</b>, also ${wie(f)} so viel verloren wie zerstört.`; };
  $("#rad-hilfe").innerHTML = `<b>Gezeigt: ${weich ? "klare Konter (durchgezogen) und knappe Vorteile ab ×1,4 (gestrichelt)" : "nur klare Konter – ab ×2, und der Verlierer büßt mindestens ein Viertel ein"}.</b> Beispiel mit den echten Zahlen, je ${mio(M.budget)} Einsatz: ${bsp(sieg[0], true)} ${bsp(nied[0], false)}`;
  const ch = (l, art, rev) => chips(l.map(([z, t, k]) => ({ e: z, wert: "×" + rf(Math.exp(t)), knapp: !k, tip: rev ? `${N(z)} zerstört ${wie(Math.exp(t))} so viel Wert, wie es gegen ${x.name} verliert` : `${x.name} zerstört ${wie(Math.exp(t))} so viel Wert, wie es gegen ${N(z)} verliert` })), art, o => rev ? `paar:${o.e}:${s0}` : `paar:${s0}:${o.e}`);
  $("#rad-info").innerHTML = buehne(s0, `${esc(x.klasse)}${x.belagerer ? " · Belagerer" : ""}`, x.name, `<span class="pille">Wert <b>${fmt(x.wert)}</b></span><span class="pille">ab Stufe <b>${x.stufe}</b></span>`).replace('class="buehne"', 'class="buehne info-buehne"') +
    `<div class="karte" data-st="margin-top:10px"><p>${esc(x.rolle)}</p></div>
    <p class="reihe gut">Schlägt ${weich ? "klar oder knapp" : "klar"}</p>${ch(sieg, "gut", false)}<p class="reihe schlecht">Verwundbar durch</p>${ch(nied, "schlecht", true)}
    <button class="cta" data-open="einheit:${s0}">Dossier mit allen Gefechten ↗</button>`;
  $("#rad-wahl").value = s0;
}

/* ---------- Flotte & Anlagen ---------- */
const mx = { kennzahl: "tausch", ordnung: "klasse", hart: "alle", fokus: "" };
function zell(p) {
  if (mx.kennzahl === "tausch") return { c: farbe(p.t), dk: dunkel(p.t), txt: tf(p.t) };
  if (mx.kennzahl === "sieg") { const b = (p.sieg[0] - p.sieg[2]) * 2.3; return { c: farbe(b), dk: dunkel(b), txt: pct(p.sieg[0]) }; }
  if (mx.kennzahl === "verlust") { const b = -p.verlust[0] * 2.3; return { c: farbe(b), dk: dunkel(b), txt: pct(p.verlust[0]) }; }
  const b = p.verlust[1] * 2.3; return { c: farbe(b), dk: dunkel(b), txt: pct(p.verlust[1]) };
}
const nachStaerke = l => l.slice().sort((a, b) => E[b].gs - E[a].gs);
const reihenfolge = () => mx.ordnung === "klasse" ? R : nachStaerke(SCHIFFE).concat(nachStaerke([...VERT]));
function matrix() {
  const o = reihenfolge();
  const starts = new Set(mx.ordnung === "klasse" ? D.gruppen.slice(1).map(g => g.e[0]) : [nachStaerke([...VERT])[0]]);
  let s = `<table class="hm-t"><thead><tr><th class="ecke"><b>Zeile ↓</b>Einheit<b data-st="margin-top:4px">Spalte →</b>Gegner</th>${o.map(z => `<th class="kopf" data-open="einheit:${z}" data-c="${z}" data-tip="${esc(N(z))} – Dossier öffnen"><span class="kz">${img(z)}${E[z].kurz}</span></th>`).join("")}</tr></thead><tbody>`;
  for (const a of o) {
    s += `<tr${starts.has(a) ? ' class="gr"' : ""}><th class="kopf" data-open="einheit:${a}" data-r="${a}" data-tip="${esc(N(a))} – Dossier öffnen"><span class="kz">${E[a].kurz}${img(a)}</span></th>`;
    for (const b of o) {
      const p = (VERT.has(a) && VERT.has(b)) ? null : paar(a, b);
      if (!p) { s += '<td class="leer"></td>'; continue; }
      const z = zell(p), matt = mx.hart === "hart" && !klar(p) && a !== b;
      const tip = `${N(a)} gegen ${N(b)}: ${tx(p.t)} · ${wortTausch(N(a), p.t, p.verlust)} · Zeile siegt ${pct(p.sieg[0])}`;
      s += `<td class="${a === b ? "diag" : ""}${z.dk && a !== b ? " dk" : ""}${matt ? " matt" : ""}" data-st="${a === b ? "" : "background:" + z.c}" data-a="${a}" data-b="${b}" data-open="paar:${a}:${b}" tabindex="0" data-tip="${esc(a === b ? N(a) + " gegen sich selbst" : tip)}">${a === b ? "" : z.txt}</td>`;
    }
    s += "</tr>";
  }
  $("#matrix").innerHTML = s + "</tbody></table>";
  fokus();
  $("#leg-matrix").innerHTML = {
    tausch: `<span><span class="sw" data-st="background:var(--gut)"></span>Zeile gewinnt – 3 = zerstört dreimal so viel Wert, wie sie verliert</span><span><span class="sw" data-st="background:var(--rot)"></span>Zeile verliert – /3 = verliert dreimal so viel, wie sie zerstört</span><span><span class="sw" data-st="background:var(--mid);border:1px solid var(--linie)"></span>ausgeglichen (=)</span><span>20+ = 20-fach oder mehr</span>`,
    sieg: `<span>Zahl = Anteil der Läufe, die die Zeile gewinnt; Farbe = Siege der Zeile minus Siege der Spalte (dunkel: meist offen nach 6 Runden)</span>`,
    verlust: `<span><span class="sw" data-st="background:var(--rot)"></span>Zahl = Anteil, den die Zeile verliert</span>`,
    gegner: `<span><span class="sw" data-st="background:var(--gut)"></span>Zahl = Anteil, den die Spalte verliert</span>`,
  }[mx.kennzahl] + `<span>Schraffur = Spiegelgefecht · leer = Anlagen kämpfen nie gegeneinander</span>`;
}
function fokus() {
  const f = mx.fokus;
  W.querySelectorAll("#matrix td[data-a]").forEach(td => td.classList.toggle("hl", !!f && (td.dataset.a === f || td.dataset.b === f)));
  W.querySelectorAll("#matrix th[data-r],#matrix th[data-c]").forEach(th => th.classList.toggle("hl", !!f && (th.dataset.r === f || th.dataset.c === f)));
}
function kachel(e, zeile) {
  const x = E[e];
  return `<button class="kachel${x.belagerer ? " bel" : ""}" data-open="einheit:${e}"><span class="bild">${img(e)}<span class="marke">${x.kurz}</span></span><span class="name">${esc(x.name)}</span><span class="zeile">${zeile}</span></button>`;
}
function kachelgruppen(gruppen, zeile) {
  return gruppen.map(g => `<div class="gruppe-h"><span>${esc(g.name)}</span><small>${esc(g.text)}</small></div><div class="kacheln">${g.e.map(e => kachel(e, zeile(e))).join("")}</div>`).join("");
}
function karte(e) {
  const x = E[e], hatGa = x.ga !== null && x.ga !== undefined;
  const sieg = x.schlaegt.slice(0, 6).map(([z, t]) => ({ e: z, wert: tx(t), tip: `${x.name} gegen ${N(z)}: ${tx(t)}` }));
  const nied = x.konter.slice(0, 6).map(([z, t]) => ({ e: z, wert: tx(t), tip: `${N(z)} gegen ${x.name}: ${tx(t)}` }));
  return `<div class="ko" data-st="margin:0"><div class="ko-inhalt" data-st="padding:14px 14px 16px">${buehne(e, `${esc(x.klasse)}${x.belagerer ? " · Belagerer" : ""}`, x.name,
    `<span class="pille">Wert <b>${fmt(x.wert)}</b></span><span class="pille">Ø Schiffe <b class="${kl(x.gs)}">${txw(x.gs)}</b></span>${hatGa ? `<span class="pille">Ø Anlagen <b class="${kl(x.ga)}">${txw(x.ga)}</b></span>` : ""}`).replace('class="buehne"', 'class="buehne info-buehne"')}
    <dl class="werte">${["Angriff", "Schild", "Struktur"].map((n, i) => `<div class="wert"><dt>${n}</dt><dd><b>${fmt(x.basis[i])}</b>${mess(x.basis[i], MAX[i])}</dd></div>`).join("")}</dl>
    <p class="leise" data-st="font-size:13px">${esc(x.rolle)}</p>
    <p class="reihe gut">Schlägt klar</p>${chips(sieg, "gut", o => `paar:${e}:${o.e}`)}${x.schlaegt.length > 6 ? `<p class="leise">+ ${x.schlaegt.length - 6} weitere im Dossier</p>` : ""}
    <p class="reihe schlecht">Verwundbar durch</p>${chips(nied, "schlecht", o => `paar:${o.e}:${e}`)}${x.konter.length > 6 ? `<p class="leise">+ ${x.konter.length - 6} weitere im Dossier</p>` : ""}
    <button class="cta" data-open="einheit:${e}">Dossier mit allen Gefechten ↗</button></div></div>`;
}
function flotte() {
  $("#kacheln-flotte").innerHTML = kachelgruppen(D.gruppen, e => `<b class="gut">▲ ${E[e].schlaegt.length}</b> schlägt klar · <b class="schlecht">▼ ${E[e].konter.length}</b> verwundbar`);
  segment("#mx-kennzahl", "kennzahl", mx, matrix); segment("#mx-ordnung", "ordnung", mx, matrix); segment("#mx-hart", "hart", mx, matrix);
  const sel = $("#mx-fokus");
  sel.innerHTML = '<option value="">keine</option>' + R.map(e => `<option value="${e}">${esc(E[e].name)}</option>`).join("");
  sel.onchange = () => { mx.fokus = sel.value; fokus(); };
  $("#matrix").addEventListener("pointerover", e => {
    const td = e.target.closest("td[data-a]"); if (!td || mx.fokus) return;
    W.querySelectorAll("#matrix .hl").forEach(x => x.classList.remove("hl"));
    W.querySelectorAll(`#matrix td[data-a="${td.dataset.a}"],#matrix td[data-b="${td.dataset.b}"],#matrix th[data-r="${td.dataset.a}"],#matrix th[data-c="${td.dataset.b}"]`).forEach(x => x.classList.add("hl"));
  });
  $("#matrix").addEventListener("pointerleave", () => { if (!mx.fokus) W.querySelectorAll("#matrix .hl").forEach(x => x.classList.remove("hl")); });
  matrix();
  $("#bz-legende").innerHTML = bzLegende();
  segment("#bz-ordnung", "ordnung", bz, bilanzSchiffe);
  bilanzSchiffe();
  $("#karten-schiffe").innerHTML = D.gruppen.filter(g => g.key !== "verteidigung").map(g => `<div class="gruppe-h"><span>${esc(g.name)}</span><small>${esc(g.text)}</small></div><div class="kacheln" data-st="grid-template-columns:repeat(auto-fill,minmax(330px,1fr));gap:14px">${g.e.map(karte).join("")}</div>`).join("");
}

const bz = { ordnung: "klasse" };
try { const o = localStorage.getItem("kraefteatlas.bilanz"); if (o === "bilanz" || o === "klasse") bz.ordnung = o; } catch (e) { }
function bilanzSchiffe() {
  try { localStorage.setItem("kraefteatlas.bilanz", bz.ordnung); } catch (e) { }
  const duell = (e, z, anl) => { const p = paar(e, z), k = stufe(p.t, p.verlust);
    return { k, t: p.t, kurz: anl ? tf(p.t) : E[z].kurz, kk: E[z].kurz, open: `paar:${e}:${z}`, tip: `${E[e].name} gegen ${E[z].name}: ${STUFE_WORT[k]} · ${wortTausch(E[e].name, p.t, p.verlust)} (${tx(p.t)})` }; };
  const zeilen = SCHIFFE.map(e => ({ e, name: E[e].name, open: `einheit:${e}`, mittel: E[e].gs, zusatz: E[e].belagerer ? '<span class="marke">Belagerer</span>' : "",
    g: SCHIFFE.filter(z => z !== e && !E[z].belagerer).map(z => duell(e, z)), anl: [...VERT].map(d => duell(e, d, true)) }));
  $("#profil-schiffe").innerHTML = bilanz(zeilen, bz.ordnung, { links: "Schiff", anl: [...VERT] });
}

/* ---------- Verteidigung ---------- */
function verteidigung() {
  $("#kacheln-vert").innerHTML = `<div class="kacheln" data-st="grid-template-columns:repeat(auto-fill,minmax(330px,1fr));gap:14px">${[...VERT].map(karte).join("")}</div>`;
  const G = D.gruppen.filter(g => g.key !== "verteidigung");
  let h = `<div class="kgrid" data-st="grid-template-columns:auto repeat(${G.length},minmax(auto,1fr))"><span class="h oben" data-st="align-items:flex-start;text-align:left">Anlage ↓ gegen Klasse →</span>${G.map(g => `<span class="h oben">${img(g.e[0])}${esc(g.name)}</span>`).join("")}`;
  for (const d of VERT) {
    h += `<button class="h" data-open="einheit:${d}">${img(d)}${esc(E[d].name)}</button>`;
    for (const g of G) { const v = g.e.map(s => T(d, s)), t = v.reduce((a, b) => a + b, 0) / v.length; h += `<button class="k" data-st="background:${farbe(t, 2.5)};${dunkel(t, 2.5) ? "color:#04121d" : ""}" data-open="klassen:verteidigung:${g.key}" data-tip="${esc(E[d].name + " gegen " + g.name + ": Ø " + tx(t))}">${tf(t)}</button>`; }
  }
  $("#vert-klassen").innerHTML = h + "</div>";
  belagerung($("#belagerung2"), null);
  $("#vwert").textContent = fmt(D.ueberfall[0].wd) + " Werteinheiten";
  const mxw = Math.max(1.5, ...D.ueberfall.map(x => x.abschreckung)) * 1.05;
  let u = `<span></span>${[1, 3, 10].map(m => `<span class="h oben">${m === 1 ? "gleich stark" : m === 3 ? "dreifache Übermacht" : "zehnfache Übermacht"}</span>`).join("")}`;
  for (const v of ["igel", ...VERT]) {
    u += `<span class="h">${v === "igel" ? `${img("lasergeschuetz")}<span>Igel-Mischung<small>Werfer 20 %, Laser 40 %, Ionen 25 %, Gauß 15 %</small></span>` : `${img(v)}<span>${esc(N(v))}<small>allein</small></span>`}</span>`;
    for (const m of [1, 3, 10]) {
      const i = D.ueberfall.findIndex(x => x.v === v && x.m === m), x = D.ueberfall[i];
      u += `<button class="uk${x.abschreckung >= 1 ? " stark" : ""}" data-open="ueberfall:${i}" data-tip="${esc((v === "igel" ? "Igel" : N(v)) + " gegen " + m + "-fache Raubflotte: Räuber siegt in " + pct(x.sieg[0]) + " der Läufe und verliert das " + fmt(x.abschreckung, 2) + "-Fache des Verteidigungswerts")}">
        <span class="z1" data-st="color:${x.abschreckung >= 1 ? "var(--gold2)" : "var(--text)"}">${fmt(x.abschreckung, 2)}</span><span class="mess"><i class="mittel" data-st="width:${Math.min(100, x.abschreckung / mxw * 100)}%"></i><em data-st="left:${100 / mxw}%"></em></span>
        <span class="z2">Räuber siegt ${pct(x.sieg[0])}</span><span class="mess"><i class="hoch" data-st="width:${Math.max(1, x.sieg[0] * 100)}%"></i></span></button>`;
    }
  }
  $("#ueberfall").innerHTML = u;
  $("#ueberfall-text").innerHTML = `Einzeln stehende schwere Anlagen schrecken leichte Raubflotten kaum ab: Gaußkanone und Plasmawerfer zielen auf Großkampfschiffe und Bomber,
    Jäger und Kreuzer treffen sie nur als Reserve – und Schwere Jäger jagen Plasmawerfer gezielt. Wer sich schützen will, mischt: Werfer und Laser gegen die Masse,
    Ionengeschütz und Gaußkanone gegen die Linie, Plasmawerfer gegen Bomber. Gegen die zehnfache Übermacht fällt jede Verteidigung schon in den ersten Runden.`;
  $("#bots").innerHTML = `<thead><tr><th>Lauf</th><th>Reiche in Stufe II/III/IV/V</th><th class="z">mit Kolonie</th><th class="z">Monde</th><th class="z">Kämpfe</th><th class="z">gegen Anlagen</th><th class="z">Räuber siegt</th><th class="z">Übermacht</th><th class="z">Abschreckung</th></tr></thead><tbody>` +
    D.bots.map(b => `<tr><th>Startwert ${b.seed}</th><td>${b.stufen || "–"}</td><td class="z">${b.kolonie ?? "–"}</td><td class="z">${b.monde ?? "–"}</td><td class="z">${fmt(b.kaempfe)}</td><td class="z">${fmt(b.anlagen)}</td><td class="z">${pct(b.sieg_anlagen / Math.max(1, b.anlagen))}</td><td class="z">×${fmt(b.uebermacht, 1)}</td><td class="z"><b>${fmt(b.absch, 2)}</b></td></tr>`).join("") + "</tbody>";
}

/* ---------- Raketen ---------- */
function raketen() {
  $("#raketen").innerHTML = `<div class="kacheln" data-st="grid-template-columns:repeat(auto-fill,minmax(360px,1fr));gap:14px">` + D.raketen.map(r => {
    const top = Math.max(1.05, r.ziele[0].q * 1.05), ziel = [r.planet ? "Verteidigung" : "", r.flotte ? "Flotten" : ""].filter(Boolean).join(" und ");
    return `<div class="ko" data-st="margin:0"><div class="ko-inhalt" data-st="padding:14px 14px 16px">${buehne("rakete_" + r.key, `Silostufe ${r.silo} · gegen ${ziel}`, r.name, `<span class="pille">Schaden <b>${fmt(r.schaden)}</b></span><span class="pille">Wert <b>${fmt(r.wert)}</b></span><span class="pille">bis <b>${r.ziele_je}</b> Ziele</span>`).replace('class="buehne"', 'class="buehne info-buehne"')}
      <p class="leise" data-st="font-size:13px">${esc(r.text)}</p>
      <div class="karte rot"><h4>Abfang</h4><p>${r.abfang} Abfangrakete${r.abfang > 1 ? "n" : ""} (${fmt(r.abfang * D.abfang.wert)}) heben sie auf – der Verteidiger zahlt ${pct(r.abfang * D.abfang.wert / r.wert)} des Angriffs.</p></div>
      <h3 class="block-h">Zerstörter Wert je Raketenwert</h3>
      <div class="profil liste">${r.ziele.map(x => `<div class="pr" data-open="rakete:${r.key}:${x.e}" tabindex="0" data-tip="${esc(N(x.e) + ": " + fmt(x.stueck, 2) + " Stück je Rakete" + (x.f !== 1 ? " (Faktor ×" + fz(x.f) + ")" : "") + (x.gedeckelt ? ", gedeckelt" : "") + " · zerstört das " + fmt(x.q, 2) + "-Fache ihres Werts")}"><span class="n">${img(x.e)}<span>${esc(N(x.e))}</span></span><span class="achse"><i class="eins" data-st="left:${100 / top}%"></i><span class="bar" data-st="left:0;width:${Math.max(.8, x.q / top * 100)}%;background:${x.q >= 1 ? "var(--gut)" : "#6c7f93"}"></span></span><span class="v"><b>${fmt(x.q, 2)}</b></span></div>`).join("")}</div></div></div>`;
  }).join("") + "</div>";
}

/* ---------- Boden ---------- */
let bmodus = "ende";
function boden() {
  $("#b-runden").textContent = B.runden_je_welle; $("#b-budget").textContent = fmt(B.budget);
  const ids = B.ids, tb = (a, d) => tausch(B.paare[a + "|" + d][bmodus][1], B.paare[a + "|" + d][bmodus][0]);
  $("#kacheln-boden").innerHTML = `<div class="kacheln">${ids.map(a => { const t = B.typen[a], m = bmodus === "ende" ? t.mittel : t.mittel_welle;
    return `<button class="kachel" data-open="truppe:${a}"><span class="bild">${img(a)}<span class="marke">${t.kurz}</span></span><span class="name">${esc(t.name)}</span><span class="zeile">Ø Tausch <b class="${kl(m)}">${txw(m)}</b></span></button>`; }).join("")}</div>`;
  let s = `<table class="hm-t"><thead><tr><th class="ecke"><b>Zeile ↓</b>Truppe<b data-st="margin-top:4px">Spalte →</b>Gegner</th>${ids.map(z => `<th class="kopf" data-open="truppe:${z}" data-tip="${esc(N(z))} – Dossier öffnen"><span class="kz">${img(z)}${B.typen[z].kurz}</span></th>`).join("")}</tr></thead><tbody>`;
  for (const a of ids) {
    s += `<tr><th class="kopf" data-open="truppe:${a}" data-tip="${esc(N(a))} – Dossier öffnen"><span class="kz">${B.typen[a].kurz}${img(a)}</span></th>`;
    for (const d of ids) {
      const t = tb(a, d), p = B.paare[a + "|" + d];
      s += a === d ? `<td class="diag" data-open="boden:${a}:${d}" tabindex="0" data-tip="${esc(N(a))} gegen sich selbst"></td>` : `<td class="${dunkel(t) ? "dk" : ""}" data-st="background:${farbe(t)}" data-open="boden:${a}:${d}" tabindex="0" data-tip="${esc(N(a) + " gegen " + N(d) + ": " + tx(t) + " · Verlust " + pct(p[bmodus][0]) + " / " + pct(p[bmodus][1]))}">${tf(t)}</td>`;
    }
    s += "</tr>";
  }
  $("#b-matrix").innerHTML = s + "</tbody></table>";
  $("#leg-boden").innerHTML = `<span><span class="sw" data-st="background:var(--gut)"></span>Zeile gewinnt – 3 = zerstört dreimal so viel Wert, wie sie verliert</span><span><span class="sw" data-st="background:var(--rot)"></span>Zeile verliert – /3 = verliert dreimal so viel, wie sie zerstört</span>`;
  $("#b-bilanz-modus").textContent = bmodus === "ende" ? "bis Kampfende" : "eine Welle";
  $("#profil-boden").innerHTML = bilanz(ids.map(a => { const t = B.typen[a];
    return { e: a, name: t.name, open: `truppe:${a}`, mittel: bmodus === "ende" ? t.mittel : t.mittel_welle, anl: null,
      g: ids.filter(d => d !== a).map(d => { const v = B.paare[a + "|" + d][bmodus], x = tb(a, d), k = stufe(x, v);
        return { k, t: x, kurz: B.typen[d].kurz, open: `boden:${a}:${d}`, tip: `${t.name} gegen ${B.typen[d].name}: ${STUFE_WORT[k]} · ${wortTausch(t.name, x, v)} (${tx(x)})` }; }) }; }), "klasse", { links: "Truppe" });
  $("#b-hilfe").innerHTML = Object.entries(B.typen).filter(([, t]) => !t.kampf).map(([k, t]) => `<div class="karte violett"><h4>${esc(t.name)} · Angriff ${fmt(t.angriff)} · Struktur ${fmt(t.struktur)}</h4><p data-st="display:flex;gap:12px;align-items:center"><img src="${bild(k)}" alt="" data-st="width:76px;height:46px;object-fit:contain;flex:none">${esc(t.rolle)}</p></div>`).join("");
}

/* ---------- Weltenkrümmer ---------- */
function wk() {
  const w = E.weltentruemmer, b = D.wk_bau, st = ["I", "II", "III", "IV", "V"][b.stufe - 1];
  $("#wk-info").innerHTML = buehne("weltentruemmer", "Megaschiff · Mondwerft", w.name, `<span class="pille">Wert <b>${mio(w.wert)}</b></span><span class="pille">ab Stufe <b>${st}</b></span><span class="pille">Werftdock <b>${b.dock}</b></span>`) + `
    <dl class="werte"><div class="wert"><dt>Angriff</dt><dd><b>${fmt(w.basis[0])}</b></dd></div><div class="wert"><dt>Schild</dt><dd><b>${fmt(w.basis[1])}</b></dd></div><div class="wert"><dt>Struktur</dt><dd><b>${fmt(w.basis[2])}</b></dd></div><div class="wert"><dt>Ziele je Runde</dt><dd><b>${w.kanaele}</b></dd></div></dl>
    <div class="karten"><div class="karte gold"><h4>Jeder Spieler kann ihn allein bauen</h4><p>Stufe ${st}, ein Mond mit Werftdock ${b.dock}, ${Object.entries(b.forschung).map(([k, n]) => esc(k) + " " + n).join(", ")}. Kosten: ${kosten(b.kosten)}.</p></div>
    <div class="karte"><h4>Rolle</h4><p>${esc(b.wirkung)}. Er steht bewusst außerhalb des Gleichgewichts – wer ihm begegnet, braucht seine Konter in großer Zahl.</p></div></div>
    <button class="cta" data-open="einheit:weltentruemmer">Dossier Weltenkrümmer ↗</button>`;
  const rows = R.map(z => ({ z, p: paar("weltentruemmer", z) })).filter(o => o.p);
  $("#wk").innerHTML = `<thead><tr><th>Gegner</th><th class="z">Anzahl</th><th class="z">Krümmer verliert</th><th class="z">Gegner verliert</th><th>Ausgang</th></tr></thead><tbody>` +
    rows.map(({ z, p }) => {
      const zw = w.zw[z] || [null, 1, 1], aus = p.sieg[0] >= 0.5 ? '<b class="gut">Weltenkrümmer siegt</b>' : p.sieg[2] >= 0.5 ? '<b class="schlecht">Gegner siegt</b>' : "offen nach 6 Runden";
      return `<tr class="klick" data-open="paar:weltentruemmer:${z}" tabindex="0"><th><span class="einh">${img(z)}${esc(N(z))}</span></th><td class="z">${fmt(p.n[1])}</td><td class="z">${pct(p.verlust[0])}</td><td class="z">${pct(p.verlust[1])}</td><td>${aus}<small>${zw[0] === null ? "nicht erfassbar" : RANG[zw[0]]}${zw[0] !== null && zw[0] < 2 && zw[2] > 1 ? ", Schnellfeuer " + zw[2] : ""}</small></td></tr>`;
    }).join("") + "</tbody>";
}

/* ---------- Methode ---------- */
function methode() {
  const mg = M.maengel.concat(M.boden_maengel);
  $("#methode").innerHTML = [
    ["Engine", `Regeln einer frischen Welt über <code>kern::quellen::neue_welt</code> – dieselbe Kette wie Server, App und ConfigWelt –, Kampf mit <code>kern::kampf</code> samt Rundenprotokoll (gleicher Zufallsstrom und Ausgang wie ohne Protokoll), Bodenkampf mit <code>kern::boden_kampf</code>.`, ""],
    ["Aufstellung", `Gleicher Ressourcenwert je Seite (${fmt(M.budget)} Werteinheiten, Boden ${fmt(B.budget)}; Wertung Erz 1, Kristall 1,5, Deuterium 2, Legierung 5, Elektronik 8, Xenokristall 20), reine Einheitentypen, Technik 6 beidseitig (×${fz(M.technik)} auf Angriff, Schild und Struktur), Volk ohne Kampfboni, ${M.laeufe} Zufallsläufe je Paar; höchstens 12.000 Einheiten je Seite.`, ""],
    ["Bewertung", `Tausch = ln((Gegnerverlust + 0,03) ÷ (eigener Verlust + 0,03)), begrenzt auf ±3 (Faktor 20). Klarer Konter: Tausch ≥ ×2 und der Verlierer büßt mindestens 25 % ein; knapper Vorteil: ab ×1,4. Mängel: Einheit ohne klaren Konter, ohne Opfer, Primärziel nicht geschlagen. Aktuell: ${mg.length}${mg.length ? " – " + mg.map(esc).join("; ") : ""}.`, "gold"],
    ["Verlauf und Mechanik", `Verlauf, Feuer und Streuung sind Mittelwerte aus dem Protokoll aller Läufe; ein früher beendeter Lauf behält seinen Endbestand. Die Erklärungen nutzen die Zielwahl der Engine je Paar (Rang, Faktor, Schnellfeuer, Schildfaktor).`, ""],
    ["Bodenkampf", `Wellen zu ${B.runden_je_welle} Runden wie im Spiel, Verwundungen beginnen jede Welle bei null; „bis Kampfende“ = weitere Wellen, bis eine Seite fällt.`, "violett"],
    ["Grenzen", `Nicht abgebildet: gemischte Flotten (außer den Überfällen), Technik- und Volksunterschiede, Planetenschilde als Puffer, Treibstoff und Tempo. Darum zusätzlich Überfälle mit Mischflotten und Bot-Welten. Die Motive zeigen das gewählte Volk; die Werte gelten für alle Völker ohne Volksbonus.`, "rot"],
    ["Neu erzeugen", `Nach jeder Werteänderung: <code>konter_matrix.exe</code> mit <code>KONTER_LAEUFE=32</code> nach <code>laeufe/konter-matrix/atlas/</code>, dann <code>python -X utf8 tools/konter_matrix/uebersicht.py</code>. Das Skript bricht ab, wenn Messung, Endmessung und Regeldateien nicht zusammenpassen.`, ""],
  ].map(([h, p, c]) => `<div class="karte ${c}"><h4>${h}</h4><p>${p}</p></div>`).join("");
}

let vkzahl = "tausch";
function voelkerZeichnen() {
  const wahl = (id, wert, setze) => { $(id).innerHTML = VOELKER.map(v => `<button class="vk-knopf" data-st="--vf:${VFARBE(v)}" aria-pressed="${v === wert}" data-v="${v}"><img src="${BILD.grund["wappen_" + v]}" alt="">${esc(VN(v))}</button>`).join("");
    $(id).querySelectorAll("button").forEach(b => b.onclick = () => setze(b.dataset.v)); };
  wahl("#vk-wahl-l", volk, v => { volk = v; $("#volk").value = v; try { localStorage.setItem("kraefteatlas.volk", v); } catch (e) { } allesZeichnen(); });
  wahl("#vk-wahl-r", gvolk, v => { gvolk = v; try { localStorage.setItem("kraefteatlas.gegner", v); } catch (e) { } voelkerZeichnen(); });
  $("#vk-merk-l").innerHTML = merkmale(volk); $("#vk-merk-r").innerHTML = merkmale(gvolk);
  const L = volk, Rv = gvolk;
  $("#vk-buehne").innerHTML = `<div class="nebel" data-st="background-image:url(${BILD.grund.nebula})"></div><img class="bild l" src="${BILD.grund["bildnis_" + L]}" alt="${esc(VN(L))}"><img class="bild r" src="${BILD.grund["bildnis_" + Rv]}" alt="${esc(VN(Rv))}"><span class="gegen">GEGEN</span>
    <div class="namen"><span><small>Dein Volk</small><b data-st="color:${VFARBE(L)}">${esc(VN(L))}</b></span><span data-st="text-align:right"><small>Gegnervolk</small><b data-st="color:${VFARBE(Rv)}">${esc(VN(Rv))}</b></span></div>`;
  // Volkseffekt über alle Paarungen
  const paare = [];
  for (const a of R) for (const b of R) { if (VERT.has(a) && VERT.has(b)) continue; const p = paarV(a, b, L, Rv), p0 = paar(a, b); if (p && p0) paare.push([a, b, p, p0, p.t - p0.t]); }
  const kat = p => klar(p) ? (p.t > 0 ? 2 : -2) : Math.abs(p.t) >= Math.log(1.4) ? (p.t > 0 ? 1 : -1) : 0;
  const kipp = paare.filter(x => kat(x[2]) !== kat(x[3]) && x[0] !== x[1]).sort((x, y) => Math.abs(y[4]) - Math.abs(x[4]));
  const schiff = paare.filter(x => !VERT.has(x[0]) && !VERT.has(x[1]) && x[0] !== x[1]);
  const mitte = schiff.reduce((s, x) => s + x[4], 0) / Math.max(1, schiff.length);
  const katn = ["klar verloren", "knapp verloren", "offen", "knapp gewonnen", "klar gewonnen"];
  const fL = VF(L), fR = VF(Rv);
  $("#vk-effekt").innerHTML = `<div class="karte gold"><h4>Im Flottenkampf</h4><p>Dein Schaden: ${esc(VN(L))}-Waffen ×${fz(fL.w)} gegen ${esc(VN(Rv))}-Panzerung ×${fz(fR.p)} → <b class="${kl(Math.log(fL.w / fR.p))}">×${fz(fL.w / fR.p)}</b> gegenüber dem Gleichstand.
      Sein Schaden: ${esc(VN(Rv))}-Waffen ×${fz(fR.w)} gegen ${esc(VN(L))}-Panzerung ×${fz(fL.p)} → <b class="${kl(-Math.log(fR.w / fL.p))}">×${fz(fR.w / fL.p)}</b>.<br>Weil Verluste die Feuerkraft jede Runde weiter senken, wirkt ein kleiner Unterschied im Tausch deutlich stärker.<br>${Math.abs(mitte) < 0.02 ? `Die Völker gleichen sich aus – Schiff gegen Schiff tauscht ${esc(VN(L))} im Mittel wie ohne Volksboni.` :
      `Schiff gegen Schiff tauscht ${esc(VN(L))} im Mittel <b class="${kl(mitte)}">${fmt(Math.exp(Math.abs(mitte)), 2)}-mal ${mitte > 0 ? "besser" : "schlechter"}</b> als ohne Volksboni.`}</p></div>
    <div class="karte"><h4>Kipppunkte · ${kipp.length} Paarungen ändern ihre Einstufung</h4><p>${kipp.slice(0, 6).map(([a, b, p, p0]) => `<button class="merk" data-st="cursor:pointer;margin:0 4px 4px 0" data-open="paarV:${a}:${b}:${L}:${Rv}">${esc(N(a))} gegen ${esc(N(b))}: ${katn[kat(p0) + 2]} → <b class="${kl(p.t)}">${katn[kat(p) + 2]}</b></button>`).join("") || "Keine Paarung wechselt die Einstufung."}</p></div>`;
  // Klassenmatrix
  const G = D.gruppen;
  $("#vk-klassen-titel").textContent = `${VN(L)} gegen ${VN(Rv)}`;
  let h = `<div class="kgrid" data-st="grid-template-columns:auto repeat(${G.length},minmax(auto,1fr))"><span class="h oben" data-st="align-items:flex-start;text-align:left;color:${VFARBE(L)}">${esc(VN(L))} ↓<br><span data-st="color:${VFARBE(Rv)}">${esc(VN(Rv))} →</span></span>${G.map(g => `<span class="h oben">${imgV(g.e[0], Rv)}${esc(g.name)}</span>`).join("")}`;
  for (const a of G) {
    h += `<span class="h">${imgV(a.e[0], L)}${esc(a.name)}</span>`;
    for (const b of G) {
      const v = [], v0 = []; for (const x of a.e) for (const z of b.e) { if (x === z && L === Rv) continue; const tv = TV(x, z, L, Rv); if (tv !== null) { v.push(tv); v0.push(T(x, z)); } }
      if (!v.length) { h += "<span></span>"; continue; }
      const tm = v.reduce((s, x) => s + x, 0) / v.length, t0 = v0.reduce((s, x) => s + x, 0) / v0.length, dd = tm - t0;
      h += `<button class="k" data-st="background:${farbe(tm, 2)};${dunkel(tm, 2) ? "color:#04121d" : ""}" data-open="klassenV:${a.key}:${b.key}:${L}:${Rv}" data-tip="${esc(a.name + " (" + VN(L) + ") gegen " + b.name + " (" + VN(Rv) + "): Ø " + tx(tm) + " · ohne Volksboni Ø " + tx(t0))}">${tf(tm)}<small>${Math.abs(dd) < 0.03 ? "·" : (dd > 0 ? "▲" : "▼") + fmt(Math.exp(Math.abs(dd)), 2)}</small></button>`;
    }
  }
  $("#vk-klassen").innerHTML = h + "</div>";
  $("#leg-vk-klassen").innerHTML = `<span><span class="sw" data-st="background:var(--gut)"></span>${esc(VN(L))} gewinnt</span><span><span class="sw" data-st="background:var(--rot)"></span>${esc(VN(L))} verliert</span><span>▲1,15 = Volksboni verbessern das Mittel um das 1,15-Fache, ▼ = verschlechtern, · = kaum Unterschied</span>`;
  // Einheitenmatrix
  let s = `<table class="hm-t"><thead><tr><th class="ecke"><b data-st="color:${VFARBE(L)}">Zeile ↓</b>${esc(VN(L))}<b data-st="margin-top:4px;color:${VFARBE(Rv)}">Spalte →</b>${esc(VN(Rv))}</th>${R.map(z => `<th class="kopf" data-open="einheit:${z}" data-tip="${esc(N(z))} (${esc(VN(Rv))})"><span class="kz">${imgV(z, Rv)}${E[z].kurz}</span></th>`).join("")}</tr></thead><tbody>`;
  const starts = new Set(D.gruppen.slice(1).map(g => g.e[0]));
  for (const a of R) {
    s += `<tr${starts.has(a) ? ' class="gr"' : ""}><th class="kopf" data-open="einheit:${a}" data-tip="${esc(N(a))} (${esc(VN(L))})"><span class="kz">${E[a].kurz}${imgV(a, L)}</span></th>`;
    for (const b of R) {
      if (VERT.has(a) && VERT.has(b)) { s += '<td class="leer"></td>'; continue; }
      const p = paarV(a, b, L, Rv), t0 = T(a, b), dd = p.t - t0, wert = vkzahl === "delta" ? dd * 4 : p.t;
      const txt = vkzahl === "delta" ? (Math.abs(dd) < 0.05 ? "·" : (dd > 0 ? "▲" : "▼") + fmt(Math.exp(Math.abs(dd)), 2)) : tf(p.t);
      s += `<td class="${dunkel(wert) ? "dk" : ""}" data-st="background:${farbe(wert)}" data-open="paarV:${a}:${b}:${L}:${Rv}" tabindex="0" data-tip="${esc(N(a) + " (" + VN(L) + ") gegen " + N(b) + " (" + VN(Rv) + "): " + tx(p.t) + " · ohne Volksboni " + tx(t0))}">${txt}</td>`;
    }
    s += "</tr>";
  }
  $("#vk-matrix").innerHTML = s + "</tbody></table>";
  $("#leg-vk-matrix").innerHTML = vkzahl === "delta" ? `<span>▲1,15 = die Volksboni verbessern den Tausch der Zeile um das 1,15-Fache, ▼ = verschlechtern ihn, · = kaum Unterschied (unter 5 %)</span>`
    : `<span><span class="sw" data-st="background:var(--gut)"></span>Zeile gewinnt</span><span><span class="sw" data-st="background:var(--rot)"></span>Zeile verliert</span><span>3 = zerstört dreimal so viel Wert, wie sie verliert · /3 = umgekehrt</span>`;
  // Raketen
  const fl = VF(L).w, fp = VF(Rv).p;
  $("#vk-raketen").innerHTML = `<div class="tbl-wrap"><table class="tbl"><thead><tr><th>Rakete</th><th>Ziele (zerstörter Wert je Raketenwert)</th></tr></thead><tbody>` + D.raketen.map(r => {
    const z = r.ziele.map(x => { const st = Math.min(r.schaden * fl * x.f / (E[x.e].basis[2] * fp), r.ziele_je), q = st * E[x.e].wert / r.wert; return [x.e, q, x.q]; }).sort((p, q) => q[1] - p[1]).slice(0, 6);
    return `<tr><th><span class="einh">${imgV("rakete_" + r.key, L)}${esc(r.name)}</span><small>Schaden ${fmt(Math.round(r.schaden * fl))}${fl !== 1 ? " (×" + fz(fl) + ")" : ""}</small></th><td>${chips(z.map(([e, q, q0]) => ({ e, wert: fmt(q, 2) + (Math.abs(q / q0 - 1) > 0.005 ? ` (${q > q0 ? "+" : "−"}${pct(Math.abs(q / q0 - 1))})` : ""), tip: `${N(e)}: ${fmt(q, 2)} je Raketenwert, ohne Volksboni ${fmt(q0, 2)}` })), "", o => `rakete:${r.key}:${o.e}`)}</td></tr>`;
  }).join("") + "</tbody></table></div>";
}
function allesZeichnen() { kopf(); ueberblick(); voelkerZeichnen(); flotte(); verteidigung(); raketen(); boden(); wk(); methode(); }
lese();
allesZeichnen();
segment("#b-modus", "m", { get m() { return bmodus; }, set m(v) { bmodus = v; } }, boden);
segment("#vk-zahl", "z", { get z() { return vkzahl; }, set z(v) { vkzahl = v; } }, voelkerZeichnen);
$("#volk").value = volk;
$("#volk").onchange = () => { volk = $("#volk").value; try { localStorage.setItem("kraefteatlas.volk", volk); } catch (e) { } allesZeichnen(); if (stapel.length) zeichne(); };
/* Direkt auf einen Eintrag springen: aus dem Technologiebaum (Modul-API zeige) oder über die Adresse #e=<id> (Website) */
const kennt = id => id === "weltentruemmer" || !!(E[id] && R.includes(id)) || B.ids.includes(id) || !!B.typen[id] || D.raketen.some(r => r.key === id);
function zeigeEintrag(id) {
  if (!kennt(id)) return false;
  const r = D.raketen.find(x => x.key === id);
  reiter(id === "weltentruemmer" ? "wk" : r ? "raketen" : B.typen[id] ? "boden" : VERT.has(id) ? "verteidigung" : "flotte", false);
  stapel.length = 0;
  if (r) oeffne("rakete", id, r.ziele[0].e); else if (B.ids.includes(id)) oeffne("truppe", id); else if (E[id] && R.includes(id)) oeffne("einheit", id); else zeichne();
  return true;
}
const vonHash = () => { const h0 = U0.eingebettet ? "" : decodeURIComponent(location.hash.slice(1)); if (h0.startsWith("e=") && zeigeEintrag(h0.slice(2))) return; const h = U0.eingebettet ? (U0.start || "") : h0; reiter(h && W.querySelector(`section[data-tab="${h}"]`) ? h : "ueberblick", false); };
if (!U0.eingebettet) { window.addEventListener("hashchange", vonHash); vonHash(); } else reiter(U0.start || "ueberblick", false);
window.addEventListener("resize", () => { const b = W.querySelector('#reiter button[aria-selected="true"]'); if (b) reiter(b.dataset.ziel, false); });

return {
  zurueck() { if (!stapel.length) return false; stapel.length > 1 ? (stapel.pop(), zeichne()) : zu(); return true; },
  zeige: zeigeEintrag, kennt,
  reiter(name) { zu(); reiter(name, false); },
  volk(v) { if (v && v !== volk && W.querySelector('#volk')) { W.querySelector('#volk').value = v; W.querySelector('#volk').dispatchEvent(new Event('change')); } },
};
};
