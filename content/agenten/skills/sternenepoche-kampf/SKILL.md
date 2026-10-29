---
name: sternenepoche-kampf
description: Laden, wenn der Feldherr aufklärt, angreift, plündert, blockiert, bombardiert, einen Planeten erobern will, Raketen baut oder startet, eine Invasion abwehrt oder Verteidigung plant – mit Kampfsimulation, Schutzregeln und dem echten Eroberungsablauf neuer Welten.
---

# Sternenepoche: Aufklärung, Kampf, Eroberung, Verteidigung

Flugfelder, Missionsbaum, Verband und Saven: **sternenepoche-flotte**. Monde und Weltenkrümmer im Detail:
**sternenepoche-monde-dyson**. Pakte und Hilferufe: **sternenepoche-diplomatie**. Ablehnungen: **sternenepoche-fehler**.

## Kurz

1. **Kein Angriff ohne frischen Bericht.** Kette: `system_erkunden` → `spionage` (genug Sonden) → `kampfsimulator`
   → `handeln probe=true` → echt. Ein Bericht ist eine Momentaufnahme (`lage.berichte[].alter_stunden`).
2. **Beute braucht Laderaum.** Geplündert werden 40 % (Krath 50 %) der Güter **über** dem Bunkerschutz, höchstens so
   viel, wie freier Frachtraum da ist. Ohne Frachter keine Beute.
3. **In neuen Welten gibt es keine Mission `invasion`.** Erobert wird mit `bombardieren` (Bomber) + Bodentruppen
   (`boden_senden`), Wellen gegen vier Verteidigungslinien, dann Kolonieschiff (`kampfkolonisieren` oder `kolonisieren`).
4. **Nur Bomber senken Gebäude-Integrität** – Bomber gibt es erst ab Zivilisationsstufe V. Ohne Bomber keine Landung.
5. **Schutz prüfen:** Anfängerschutz (10 Spieltage, endet mit Stufe III oder dem ersten eigenen Angriff), ursprüngliche
   Heimatwelten sind nie übernehmbar, Verbündete (Allianz, Verteidigungsbündnis) sind nicht angreifbar.
6. **Ein Nichtangriffspakt sperrt nichts** – ein Angriff oder Raketenschlag auf den Partner **bricht** ihn öffentlich
   und kostet die Kaution. Vorher Pinwand-`beschluss` des Strategen einholen.
7. **Verteidigung ist ortsfest**, wird nach normalen Kämpfen zu 70 % wiederhergestellt, aber nie durch Raketenschäden.
   Flotten lassen sich saven, Verteidigung nicht.
8. **Abfangraketen** schützen automatisch ihren Planeten (intaktes Silo); Flottenraketen treffen nur **erkannte**
   Anflüge vor deren Ankunft.
9. **Bei Landung des Gegners sofort:** `invasion_linien`, `miliz_ausrufen`, Nachschub per `boden_senden`, Hilferuf.

## Aufklärung

| Aktion | Wer | Wirkung |
|---|---|---|
| `flotte_senden` mission `system_erkunden` (nur Sonden) | Feldherr | kartiert ein System: Gürtel, Nebel, Bewohnerliste |
| `system_aufklaeren {start, ziel, abbrechen}` | Feldherr | eine Sonde je fremdem Platz plus Systemscan, falls nötig |
| `flotte_senden` mission `spionage` (nur Sonden, System kartiert) | Feldherr | Planetenbericht |
| `flotte_ausspaehen {start, flotte, sonden, geschwindigkeit}` | Feldherr | Details eines erkannten Anflugs aus `lage.angriffe[].flotte`, nur vor dessen Ankunft |
| `phalanx_scannen {mond, ziel}` (Tiefenphalanx, Mond) | Feldherr | Flüge eines Planeten in Reichweite; der Gescannte erfährt nichts |

**Berichtstiefe** (Kern `spionieren`): Scanstärke = Sonden + eigene Spionagetechnik − gegnerische Spionagetechnik.
Ab 2: Schiffe und Orbitlage (Halteflotten, Blockaden), ab 3: Verteidigung, ab 5: Gebäude, ab 7: Forschung und Vollscan
(Bevölkerung, Garnison, Raketen). Der Bestand an Gütern steht immer im Bericht. Abwehrchance der Sonden steigt mit der
Schiffszahl am Ziel und der Sondenzahl und fällt mit jedem Technikvorsprung; überlebende Sonden kehren zurück.

Lage: `lage.berichte[]` (`ziel`, `zeit`, `alter_stunden`, `besitzer`, `bestand`, `schiffe`, `verteidigung`,
`gebaeude`, `forschung`, `orbit`, `sonden`, `abgeschossen`), `lage.systemberichte`, `lage.flottenberichte`,
`lage.erkundet[]` (`status` frei/freund/feind, `heimat`).

**Vorwarnung eigener Planeten:** Grundwarnung 120 Spielminuten vor Ankunft, +30 Minuten je effektiver Sensorstufe
(kleinste Stufe aus Geheimdienst, Sensorphalanx, Überwachungstechnik, Spionagetechnik). Anflüge stehen in
`lage.angriffe[]` (`flotte`, `ziel`, `ankunft_sekunden`, `sichtstufe`, `schiffe`, `schiffstypen`), Anflüge auf
Verbündete in `lage.verbuendete_anfluege[]`.

```json
{"typ":"flotte_senden","start":"1:27:6","ziel":"1:29:4","mission":"spionage","schiffe":{"spionagesonde":5},
 "geschwindigkeit":1.0,"ladung":{},"haltedauer_stunden":0}
```

## Kampfsimulation vor jedem Angriff

- `werkzeug kampfsimulator {ziel, schiffe}` rechnet gegen den **eigenen letzten Bericht** (Schiffe und Verteidigung
  müssen darin stehen). Ohne Forschungsteil schätzt es die gegnerische Technik wie die eigene. Ergebnis u. a.
  `siegchance_prozent`, `unentschieden_prozent`, `siegchance_intervall_95_prozent`, `beute_mittel_milli`,
  `bericht_alter_stunden`, `technik_des_gegners`, `bunker_annahme`. Zusätzlich liest der Kern `zielart: "mond"`
  (Mondbericht) und `bergung_promille: 1000` (erwartetes Trümmerfeld für Recycler); beide Felder stehen nicht im
  Antwortschema, strenge Clients rechnen Mondkämpfe dann mit `kampfszenario` aus dem Mondbericht (`lage.monde`).
- `werkzeug kampfszenario {angreifer, verteidiger}` für freie Annahmen: je Seite `volk`, `technik {waffentechnik,
  schildtechnik, panzerung}`, `einheiten {name: anzahl}` (Verteidiger auch Anlagen). Der Kern erlaubt je Seite auch eine
  Liste von 1–8 Gruppen (Verband gegen Verteidiger plus Halteflotten) – das Schema beschreibt nur die Einzelgruppe.
- Schlacht: höchstens 6 Runden; Gewinner ist, wer am Ende noch Kampfeinheiten hat, sonst unentschieden.
  Primärziele einer Einheit nehmen doppelten, Sekundärziele 1,5-fachen Schaden (Kernwerte `kampf`). Schnellfeuer wirkt
  nur gegen Primär- und Sekundärziele und springt in neuen Welten nach einem Abschuss auf ein weiteres Ziel derselben Art.

**Entscheidungsregel:** Angreifen nur, wenn Siegchance hoch, Bericht jünger als die Flugzeit des Gegners zum Verlegen
und die erwartete Beute (+ Trümmer) die erwarteten Verluste und den Treibstoff übersteigt. Bei unbekannter Orbitlage
(`orbit` fehlt im Bericht) mehr Sonden schicken.

## Angriff, Beute, Trümmer

```json
{"typ":"flotte_senden","start":"1:27:6","ziel":"1:29:4","mission":"angriff",
 "schiffe":{"kreuzer":40,"grosser_transporter":20},"geschwindigkeit":1.0,"ladung":{},"haltedauer_stunden":0}
```

- **Beute** = Plünderquote × (Bestand − Bunkerschutz) je Gut, begrenzt durch den freien Frachtraum der überlebenden
  Schiffe. Bunker schützt je Stufe 20.000 von Erz, Kristall, Deuterium und Nahrung und 20 % davon (4.000) bei
  verarbeiteten Gütern. Credits sind keine Beute. Plünderung senkt die Stabilität des Opfers.
- **Trümmer**: 30 % des Erz- und Kristallwerts zerstörter Schiffe. Bergen mit `recyceln` (Recycler/Bergungsschiffe),
  am besten 5 s nach der Angriffsflotte angekoppelt (`flotte_planen` mit `ankunft`, sternenepoche-flotte).
- **Mond**: Kämpfe an besiedelten Planeten ohne Mond können einen Mond erzeugen (`mondwurf` im Kampfbericht).
- **Entsatz**: Angriff auf einen eigenen oder verbündeten Planeten ist nur gegen eine feindliche Blockade dort erlaubt.
- Wer selbst feindlich startet, verliert sofort den eigenen Anfängerschutz.
- Berichte: `lage.kampfberichte[]` (Verluste, Beute, Trümmer, `rundenprotokoll`, `technik`).

## Blockade und Bombardieren

- `blockade` (ab Stufe IV) hält den feindlichen Orbit bis zum Rückruf: Der Planet darf nur noch die Blockade
  angreifen, Transporte und Marktlieferungen warten, die Stabilität sinkt je Tag der Belagerung.
- `bombardieren` (ab Stufe IV, mindestens ein `bomber` oder `strategischer_bomber`) kämpft zuerst gegen alle
  bewaffneten Schiffe, Verteidigungsanlagen, Planetenschilde, verbündete Halteflotten und beistehende gestrandete
  Flotten. Gewinnt sie, hält sie den Orbit. Erste Bombenrunde sofort, dann alle 15 Spielminuten: Bombenlast 50 je
  Bombenschacht (Bomber 2, Strategischer Bomber 3) gegen 700 Strukturpunkte je Gebäudestufe; alle Gebäude verlieren
  gleich viel Integrität, nie unter 30 %. Die erste Runde storniert die Bauschleife des Planeten.
- Fortschritt: `lage.invasionen[].bombardement` (`status` laeuft/unterbrochen/ohne_bomber/abgeschlossen,
  `runden_bis_ziel`, `restzeit_sekunden`, `bombenlast_je_runde`). Weitere Bombardier- oder Kampfkolonisierflotten
  desselben Angreifers vereinigen sich mit der Blockade (nachgeschickte Bomber zählen sofort).
- Neue bewaffnete Verteidigung unterbricht: die Runde wird zum Kampf, ein verlorener Kampf beendet die Bombardierung.
- Treibstoffnachschub für die Orbitflotte nur mit `flotte_versorgen`.

## Eroberung in neuen Welten (Invasion v2) – Schritt für Schritt

Neue Server-Welten laufen mit Kolonisationsregeln v2 und Invasion v2 (`lage.invasion_v2 = true`). Der Kern lehnt
`flotte_senden` mit `mission: "invasion"` ab („Stabilitäts-Invasion ersetzt: bombardieren und kampfkolonisieren
verwenden"). Der Ablauf:

**0. Voraussetzungen prüfen**
- Eigene Zivilisationsstufe IV für Orbitbesetzung; Bomber gibt es ab Stufe V (Werft 7, `lage.einheiten_kosten[]`).
- Truppentransporter (Kaserne nötig) und Bodentruppen aus der Kaserne: `boden_bauen {planet, art, anzahl}`
  (Feldherr, `lage.planeten[].bodentruppen`), Storno `boden_storno {planet, auftrag}`.
- Ein Kolonieschiff für die Übernahme und **zum Zeitpunkt der Übernahme** ein freier Kolonieplatz
  (`lage.kolonien {anzahl, erlaubt}`). Flüge reservieren keinen Platz.
- Ziel ist keine ursprüngliche Heimatwelt (`lage.erkundet[].heimat`), Besitzer nicht im Anfängerschutz und nicht verbündet.

**1. Aufklären:** System kartieren, mit Scanstärke ≥ 5 spähen (Schiffe, Verteidigung, Gebäude, Orbitlage). Die
Gebäudesumme bestimmt die Bombenrunden; `werkzeug wiederaufbau {ziel}` rechnet die Reparaturkosten aus dem Bericht.

**2. Simulieren:** `kampfsimulator` mit Kampfflotte, Bombern und Transportern.

**3. Fliegen:** Bomber, Geleitschutz und Truppentransporter mit Truppen in **einem** Flug, als `boden_senden` mit
Flugmission `bombardieren` (oder `kampfkolonisieren`, dann mit Kolonieschiff an Bord):

```json
{"typ":"boden_senden","truppen":[200,20,20,0,0,0,0,0,0,0,0,0],
 "flug":{"typ":"flotte_senden","start":"1:27:6","ziel":"1:29:4","mission":"bombardieren",
  "schiffe":{"bomber":15,"schlachtschiff":20,"kreuzer":30,"truppentransporter":4},
  "geschwindigkeit":1.0,"ladung":{},"haltedauer_stunden":0}}
```

(100 gewichtete Truppenplätze je Truppentransporter; Truppenreihenfolge in sternenepoche-flotte.)

**4. Orbit und Bomben:** Kampf bei Ankunft, dann Bombenrunden bis alle Gebäude bei höchstens 30 % stehen.

**5. Landung (automatisch):** sobald alle Gebäude ≤ 30 %, der Orbit unbewaffnet ist (keine bewaffnete Halteflotte des
Verteidigers; reine Truppenkonvois sperren nicht) und die Flotte Bodentruppen trägt. Ohne Truppen keine Landung.
Ab jetzt `lage.invasionen[]` mit `wellen` (`linie`, `naechste_welle`, `linien[]`, `berichte`).

**6. Wellen:** Der Planet hat 4 Verteidigungslinien, die letzte ist der Rückhalt. Die erste Welle kommt 12 Spielstunden
nach der Landung, dann alle 12 Stunden; je Welle höchstens 6 Bodenrunden. Leere Linien fallen kampflos.
Ergebnis je Welle: *abgewehrt* (alle Angreifer tot – Invasion beendet), *gebrochen* (nächste Welle trifft die nächste
Linie) oder *gehalten*. Nachschub: weitere `boden_senden` mit `bombardieren`/`kampfkolonisieren` zum selben Ziel
vereinigen ihre Truppen mit der Besatzung; Verbündete des Angreifers mit Truppen im Orbit (`boden_senden` mit `halten`)
kämpfen auf Angreiferseite mit.

**7. Verwüstet:** Fällt der Rückhalt, verliert der Besitzer die Kontrolle (`verwuestet: true`). Jetzt landet ein
Kolonieschiff der Besatzerseite – eigenes oder eines Verbündeten – mit `kampfkolonisieren` (Kolonieschiff, keine
Siedler nötig) oder `kolonisieren` (Startfracht, Eskorte, Siedler; die Siedler werden Bewohner). Ohne freien
Kolonieplatz wartet es im Orbit und landet, sobald einer frei ist. Landen vorher neue Verteidiger, steht der Rückhalt wieder.

```json
{"typ":"flotte_senden","start":"1:27:6","ziel":"1:29:4","mission":"kampfkolonisieren",
 "schiffe":{"kolonieschiff":1,"kreuzer":10,"grosser_transporter":10},"geschwindigkeit":1.0,
 "ladung":{"erz":120000,"kristall":60000,"legierung":3000},"haltedauer_stunden":0}
```

**8. Übernahme:** Bevölkerung bleibt, die Besatzungstruppen werden Garnison, die Fracht bleibt auf dem Planeten, alle
Gebäude stehen bei 30 %. Der Verwalter repariert (`reparieren {planet, gebaeude}`, 75 % der Kosten der aktuellen Stufe).

**Abbruch** durch: alle Angreifertruppen gefallen; verlorener Orbit (verbündete Halteflotte siegt über die
Besatzungsflotte); neue bewaffnete Verteidigung, die nicht besiegt wird; Rückruf der Besatzungsflotte.

## Raketen (raketen.rs, docs/RAKETENSYSTEM.md)

| Art (`art`) | Ziel | Hinweis |
|---|---|---|
| `abfang` | – | nicht startbar; fängt automatisch Angriffsraketen auf den eigenen Planeten ab (intaktes Silo), je Hyperrakete zwei |
| `interplanetar` | eine Verteidigungsklasse eines selbst aufgeklärten Planeten | Flug 1.200 s + 60 s je System, Reichweite 5 Systeme je Silostufe, nur derselbe Sektor |
| `flottenrakete` | eine Schiffsklasse eines **erkannten** Anflugs auf eigene oder verbündete Planeten | muss vor dessen Ankunft einschlagen; `flotte` = `lage.angriffe[].flotte`, `ziel` = dessen Ziel |
| `bunkerbrecher` | Verteidigung, stark gegen Gauß, Plasma, Planetenschilde | höhere Silo- und Zivilisationsstufe |
| `hyperrakete` | Verteidigung oder erkannte Anflüge | höchste Silo-/Zivilisationsstufe, braucht zwei Abfangraketen |

Exakte Voraussetzungen, Kosten, Plätze und Schaden je Art: `lage.planeten[].raketensilo.typen[].regel` und `sperre`,
`werkzeug kosten {rakete}`. Das Silo hat 10 Plätze je Stufe (fertige Raketen und Aufträge zusammen).

```json
{"typ":"raketen_bauen","planet":"1:27:6","art":"abfang","anzahl":20}
{"typ":"raketen_starten","start":"1:27:6","ziel":"1:29:4","anzahl":10,"zieltyp":"plasmawerfer","art":"interplanetar","flotte":null,"zielart":"planet"}
{"typ":"raketen_storno","planet":"1:27:6","auftrag":5521}
```

- Vorher `werkzeug raketenplan {aktion: <raketen_starten>}` (echte Startprüfung, Einschlagszeit) und
  `werkzeug raketensimulator {art, anzahl, zieltyp, ziel_anzahl, angreifer:{waffentechnik, volk}, verteidiger:{panzerung, volk, abfangraketen, silo_intakt}}`.
- Raketen umgehen Schilde, bringen keine Beute und keine Übernahme; zerstörte Verteidigung wird nicht wiederaufgebaut.
- Ein Start beendet den eigenen Anfängerschutz und bricht Pakte mit dem Ziel wie ein Angriff. Laufende Salven:
  `lage.taktische_raketensalven`, `lage.raketensalven`.

## Verteidigung des eigenen Reichs

1. **Anlagen fertigen** (Feldherr, Topf `militaer`): `{"typ":"fertigen","planet":"1:27:6","einheit":"raketenwerfer","anzahl":50}`.
   Der Verwalter fertigt nur zivile Einheiten. Je Planet ist ein Planetenschild aktiv (`planetenschild`,
   `planetenschild_ii`, `planetenschild_iii`; die größere Stufe ersetzt die kleinere).
2. **Bunker** (Verwalter baut) gegen Plünderung; Überschuss über dem Bunkerschutz lieber saven oder verbauen.
3. **Abfangraketen** gegen Raketenschläge; das Silo muss voll repariert sein.
4. **Saven** der Flotte vor überlegenen Angriffen (sternenepoche-flotte), Rückkehr nach der feindlichen Ankunft.
5. **Halten durch Verbündete:** Partner mit Allianz oder Verteidigungsbündnis schicken `halten`; der Diplomat bittet
   über `allianz_hilfe {planet, text}` (Diplomat oder Feldherr, nur in einer Allianz), schließt mit
   `allianz_hilfe_status {hilfe, erledigt:true}`.
6. **Miliz** (ab Kaserne 1 und Soziologie 2, in neuen Welten jederzeit): `{"typ":"miliz_ausrufen","planet":"1:27:9"}`.
   Kämpfender Anteil der Bewohner: Krath 100 %, Veyari 90 %, Syntheten 80 %, Aurelianer 70 %; Waffenwert 0,3 und
   Rüstwert 5 je Person, 1 Legierung je Person (nicht erstattet), Ausbildung 300 Spielsekunden je 1.000, die Hälfte fehlt
   in Produktion und Forschung, Kapitulation nach 30 % Verlusten. `miliz_entlassen {planet}` nur außerhalb einer
   Invasion. (`miliz_aufrufen` wird in diesen Welten auf `miliz_ausrufen` umgeleitet.)
7. **Linien verteilen** während einer Invasion: `invasion_linien {planet, linien}` – bis zu 3 Zeilen (Linie 1–3) mit je
   13 Zahlen (12 Truppentypen in Katalogreihenfolge, dann Miliz). Nicht Zugeteiltes, neu Ausgebildetes, gelandete eigene
   Truppen und verbündete Bodenhilfe stehen im Rückhalt. Gebrochene Linien sind gesperrt.

```json
{"typ":"invasion_linien","planet":"1:27:9","linien":[[40,0,0,0,0,0,0,0,0,0,0,0,800],[30,5,0,0,0,0,0,0,0,0,0,0,800],[0,0,0,0,0,0,0,0,0,0,0,0,0]]}
```

8. **Nachschub trotz Blockade:** eigene Truppen mit `boden_senden` und Flugmission `transport` oder `stationieren`
   landen im Rückhalt; die Kaserne bildet während einer Invasion mit halbem Tempo aus. Unter Bombardierung und
   Invasion keine Reparaturen.

## Mondzerstörung mit Weltenkrümmern

`flotte_senden` mit `mission: "mond_zerstoeren"` (setzt `zielart: "mond"`), mindestens ein `weltentruemmer`
(gebaut im Werftdock eines Mondes ab Stufe 8, Start meist mit `startart: "mond"`). Nach vollständigem Sieg über die
Mondverteidigung: Zerstörungschance (100 − √Durchmesser) × √Anzahl %, Verlustrisiko aller Weltenkrümmer √Durchmesser ÷ 2 %.
Der Mond wird zum Trümmerring (20 Erz und 10 Kristall je km plus Lager-, Gebäude- und Wrackanteil); die Hälfte der
Besatzung stationierter Schiffe überlebt und wird mit `recyceln` als Passagiere geborgen. Mondbericht vorher mit
Spionage (`zielart: "mond"`) und `kampfsimulator` mit `zielart: "mond"`.

## Schutzregeln

| Regel | Wirkung |
|---|---|
| Anfängerschutz | 10 Spieltage ab Beitritt, endet früher mit Stufe III oder dem ersten eigenen feindlichen Start (`lage.anfaengerschutz_bis`) |
| Ursprüngliche Heimatwelt | nie übernehmbar; angreifen, blockieren und bombardieren bleibt möglich |
| Allianz, Verteidigungsbündnis (auch Allianz-zu-Allianz) | gegenseitig nicht angreifbar („Bündnis zuerst ausdrücklich beenden"); Angriff auf ein Allianzmitglied führt zum Ausschluss |
| Nichtangriffspakt | **keine Sperre**; Angriff oder Raketenstart bricht ihn öffentlich, die geschädigte Seite erhält beide Kautionen |
| Schwachenschutz | in neuen Welten aus (Anteil 0) |
| Bot-Schonzeit | `lage.bot_schutz` beschreibt, wann Serverbots dich angreifen dürfen |

## Konter (Kräfteatlas, HANDBUCH Kap. 25, gemessen mit tools/konter_matrix bei gleichem Ressourcenwert)

- Keine Einheit ist die beste; die Stärke liegt in den Zielen. Grobregel: **Jäger** schlagen Großkampfschiffe,
  **Kreuzer** schlagen Jäger, **Großkampfschiffe** (Schlachtschiffe, Zerstörer, Schlachtkreuzer) schlagen Kreuzer.
- **Belagerer** (Bomber, Strategischer Bomber, Belagerungsschlachtschiff) brechen Verteidigung, sind im Flottenkampf
  schwach und brauchen Geleit; Bomber werden gekontert von Schlachtschiff, Flottenkreuzer, Trägerschlachtschiff,
  Abfang-Schlachtkreuzer.
- **Anlagen** schießen nur auf Schiffe: Raketenwerfer und Lasergeschütz fallen gegen Bomber, Gaußkanone gegen
  Strategischen Bomber und Belagerungsschlachtschiff, Plasmawerfer gegen Strategischen Bomber, Belagerungsschlachtschiff,
  Schweren Jäger. Gegen eine Bomberflotte verteidigt man mit Schiffen, nicht mit mehr Anlagen.
- Spezialisten: Abfangjäger jagen Jäger, Schwere Jäger jagen Bomber, Sturmjäger jagen Zerstörer, Zerstörer jagen
  Schlachtschiffe und Belagerer, Sturmschlachtschiffe jagen Zerstörer.
- Für die konkrete Paarung immer `kampfszenario` rechnen; die Tabelle ersetzt keine Simulation mit echter Technik.

## Typischer Feldherr-Zug bei Bedrohung

1. `agenda?rolle=feldherr`; `lage` mit `teile=angriffe,flotten,flottenplaetze,invasionen,planeten`.
2. Anflug bewerten: `flotte_ausspaehen` bei unklarer `sichtstufe`; Kampfszenario eigene Verteidigung gegen Schätzung.
3. Entscheiden: Verteidigen (Flotte bleibt, Anlagen fertigen, Halten erbitten) oder Saven (Flotte und Überschuss weg).
4. Flottenraketen nur, wenn sie vor der Ankunft einschlagen (`raketenplan`).
5. Bei Landung: Linien, Miliz, Nachschub, Hilferuf; Pinwand-`warnung` an alle Rollen.
