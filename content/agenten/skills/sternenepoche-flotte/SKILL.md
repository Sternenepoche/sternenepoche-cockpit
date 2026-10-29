---
name: sternenepoche-flotte
description: Laden, bevor eine Rolle einen Flug plant oder startet – Missionswahl, flotte_senden und seine Hüllen (boden_senden, personen_senden, flotte_beladen), Ankunftszeiten, Treibstoff, Rückruf, Verband, gestrandete Flotten, Saven, Expedition, Mondflüge und Hyperraumloch.
---

# Sternenepoche: Flottenführung

Grundlagen (Freigabe, Lage, Pinwand, Töpfe) stehen in **sternenepoche-grundlagen** und **sternenepoche-rat**,
Kampf, Aufklärung und Eroberung in **sternenepoche-kampf**, Monde und Hyperraumtore in **sternenepoche-monde-dyson**,
Ablehnungstexte in **sternenepoche-fehler**.

## Kurz

1. **Erst kennen, dann fliegen.** Fremde und freie Ziele brauchen einen eigenen Spionagebericht
   (`lage.erkundet[]`, `lage.berichte[]`), Systemziele ein kartiertes System (`system_erkunden`). Sonst lehnt der Kern ab.
2. **Eine Flotte = ein Auftrag.** Die Mission legt fest, was am Ziel passiert; der Entscheidungsbaum unten wählt sie.
3. **Treibstoff für beide Strecken wird beim Start abgebucht** (Stationieren und Kolonisieren nur Hinflug). Der Topf
   der Rolle zahlt nur den Treibstoff; der Planetenbestand muss Ladung **und** Treibstoff decken.
4. **Flottenplätze sind knapp:** 1 + Stufe Computertechnik (`lage.flottenplaetze`). Sonden, Saves, gestrandete Flotten
   und Expeditionen belegen je einen Platz. Halte für den Notfall-Save einen Platz frei.
5. **Ankunftszeit planen statt hoffen:** `werkzeug flugzeit` (zehn Tempostufen), `flugkoordination` (Vorschau mit
   Ankunft), `flotte_planen` mit `ankunft` (Terminflug, höchstens 7 Tage voraus, 20 je Spieler).
6. **Probe vor Echt:** jede Aktion zuerst mit `handeln probe=true`; nur angenommene Proben echt senden (neue `request_id`).
7. **Schema-Vollform:** Das Antwortschema verlangt in `schiffe` alle aktiven Schiffsnamen und in `ladung` alle zehn
   Güter (unbenutzt = 0), in Mondwelten zusätzlich `startart`/`zielart`. Der Kern nimmt auch Kurzformen; strenge
   MCP-Clients nicht. Unten steht eine Vollform, die übrigen Beispiele sind Kurzform.
8. **Hüllen tragen einen ganzen Flug:** `boden_senden`, `personen_senden`, `flotte_beladen`, `schiffangebot`,
   `expedition_starten`, `rettung_starten`, `flotte_planen` enthalten in `flug` eine vollständige `flotte_senden`-Aktion
   **mit** `"typ":"flotte_senden"`.
9. **Zuständigkeit beachten:** Feldherr darf alles; Verwalter nur zivile Missionen (Tabelle unten). Ein Verwalter-Rückruf
   trifft auch Kampfflotten – Rückrufe fremder Aufträge nur nach Pinwand-Absprache.

## Wer darf was (Kern `Aktion::zustaendig`)

| Aktion / Mission | Rollen |
|---|---|
| `flotte_senden` mit transport, stationieren, kolonisieren, abbau, recyceln, terraformen, saven | Verwalter, Feldherr |
| `flotte_senden` mit allen anderen Missionen | Feldherr |
| Mondflug (`zielart`/`startart` mond): transport, stationieren | Verwalter, Feldherr; alle anderen nur Feldherr |
| `flotte_zurueckrufen`, `flotte_versorgen`, `flotte_planen`, `flugplan_storno`, `flotte_beladen`, `personen_senden`, `expedition_starten`, `expedition_reparieren`, `rettung_starten` | Verwalter, Feldherr |
| `boden_senden`, `flotte_starten`, `orbitflotte_angreifen`, `verband_oeffnen`, `verband_beitreten`, `flotte_ausspaehen`, `system_aufklaeren` | Feldherr |
| `schiffangebot`, `schiffangebot_antwort`, `orbitflotte_anbieten` | Diplomat, Feldherr |

Treibstoff zahlt der Feldherr aus `militaer`, der Verwalter aus `wirtschaft` (`lage.toepfe`). Reicht der Topf nicht,
nennt die Ablehnung Topf und Fehlbetrag: Pinwand-`anfrage` an den Strategen (`topf_umbuchen` sofort, `doktrin` dauerhaft).

## Entscheidungsbaum: welche Mission?

Lies zuerst, was am Ziel liegt: `lage.planeten[]` (eigen), `lage.erkundet[]` und `lage.berichte[]` (fremd/frei,
mit `status` `frei`/`freund`/`feind` und `alter_stunden`), `lage.truemmer[]`, `lage.bergbauziele`, `lage.angriffe[]`,
`lage.orbitflotten_bekannt`, `lage.monde`.

1. **Ziel ist ein eigener Planet**
   - Güter, Personen oder Truppen hin, Schiffe zurück → `transport` (Personen: `personen_senden`, Truppen: `boden_senden`).
   - Schiffe dauerhaft verlegen → `stationieren` (Einweg, landet).
   - Eigener Planet wird von einem Feind blockiert → `angriff` auf den eigenen Planeten (Entsatz gegen die Blockadeflotte).
   - Klima oder Felder verbessern → `terraformen` (Terraformer; Bedarf mit `werkzeug terraformbedarf`, siehe sternenepoche-wirtschaft).
2. **Ziel gehört einem Verbündeten** (gleiche Allianz oder Verteidigungsbündnis, `beziehung.status` `verbuendet`)
   - Mitverteidigen → `halten` (1–168 Stunden, `haltedauer_stunden`). Ein Nichtangriffspakt reicht **nicht**.
   - Bodenhilfe in einer Invasion → `boden_senden` mit Flugmission `halten` (siehe sternenepoche-kampf).
   - Güter schenken → `transport` (eigener Spionagebericht des Ziels nötig, siehe unten).
   - Angreifen geht nicht; einzige Ausnahme: `angriff` gegen eine feindliche Blockade über dem Partnerplaneten.
3. **Ziel gehört einem fremden Reich**
   - Wissen → `spionage` (nur Sonden, System muss kartiert sein); ganzes System → `system_aufklaeren`.
   - Flotte vernichten, Beute machen → `angriff` (vorher Kampfsimulation, sternenepoche-kampf).
   - Orbit sperren → `blockade` (ab Zivilisationsstufe IV).
   - Erobern → `bombardieren` und `kampfkolonisieren` (ab Stufe IV). `invasion` lehnt der Kern in neuen Welten ab.
   - Beliefern oder Schiffe übergeben → `transport` bzw. `schiffangebot` (sternenepoche-handel).
4. **Ziel ist ein freier Platz** → `kolonisieren` (Kolonieschiff, bewaffnete Eskorte, Startfracht, Siedler) oder `terraformen`.
5. **Position 0 eines Systems** (Asteroidengürtel, jedes System hat einen) → `abbau` (Bergbauschiffe, 1–48 Stunden),
   `saven` (sicherer Parkplatz) oder Expedition (`expedition_starten`).
6. **Trümmerfeld** (`lage.truemmer[]`, System kartiert) → `recyceln` (Recycler oder Bergungsschiffe; darf auch ohne
   sichtbares Feld starten, etwa 5 s nach einem eigenen Angriff angekoppelt).
7. **Unbekanntes System** → `system_erkunden` (nur Sonden).
8. **Anfliegende feindliche Flotte** (`lage.angriffe[]`) → `flotte_ausspaehen` (Details) oder Flottenrakete
   (sternenepoche-kampf); eigene Schiffe in Sicherheit → `saven`.
9. **Mond** → `flotte_senden` mit `zielart: "mond"`: transport, stationieren, halten, spionage, angriff, blockade,
   `mond_zerstoeren` (Weltenkrümmer). Start vom Mond mit `startart: "mond"` (Mondbasis 1).
10. **Gestrandete Flotte** → eigene: `flotte_starten`, `flotte_versorgen`, `orbitflotte_anbieten`; fremde: `orbitflotte_angreifen`.

## Die 17 Missionen und ihre Kernprüfungen (flotte.rs `flotte_senden_auswahl`)

Für alle gilt: Startplanet mit funktionsfähigem Raumhafen, nicht blockiert (sonst nur Angriff auf die eigene Blockade),
Schiffe vorhanden, freier Flottenplatz, `geschwindigkeit` 0.1–1.0, Ziel in der Galaxie. Position 0 fliegen nur
`abbau` und `saven` an.

| Mission | Pflichtschiffe und Bedingungen | Am Ziel |
|---|---|---|
| `angriff` | Ziel mit echtem Gegner, kein eigener/verbündeter Planet (außer Entsatz), Gegner nicht im Anfängerschutz, Ziel bekannt | Kampf, Beute, sofort zurück |
| `transport` | Planet am Ziel (oder eigene gestrandete Flotte dort), Ziel bekannt | liefert an den Besitzer **beim Abflug**, kehrt um |
| `stationieren` | eigener anderer Planet | landet (Einweg) |
| `halten` | Planet eines Verbündeten, `haltedauer_stunden` 1–168; oder Bodenhilfe für eine verbündete Invasion | verteidigt mit, dann zurück |
| `spionage` | nur `spionagesonde`, System kartiert, kein eigener Planet, keine Ladung | Bericht, überlebende Sonden kehren zurück |
| `kolonisieren` | Kolonieschiff, bewaffnetes Begleitschiff, Ziel selbst erkundet, Startfracht (`werkzeug koloniebedarf`), Siedler; mindestens 1000 Einwohner bleiben zurück | gründet bei freiem Kolonieplatz, sonst Umkehr |
| `recyceln` | `recycler` oder `bergungsschiff`, System kartiert | sammelt, kehrt zurück |
| `abbau` | `bergbauschiff`, Gürtel oder natürlicher Ring, `haltedauer_stunden` 1–48, System kartiert | baut ab, kehrt zurück |
| `blockade` | Stufe IV, fremder Planet mit Gegner | hält den Orbit bis zum Rückruf |
| `invasion` | in neuen Welten **abgelehnt**: „bombardieren und kampfkolonisieren verwenden" | – |
| `bombardieren` | Stufe IV, mindestens ein `bomber` oder `strategischer_bomber` | Kampf, dann Bombenrunden bis zum Rückruf |
| `kampfkolonisieren` | Stufe IV, Kolonieschiff, keine ursprüngliche Heimatwelt | Kampf, Orbit halten, Übernahme (sternenepoche-kampf) |
| `saven` | anderes Ziel als der Start, `haltedauer_stunden` 0–72 | wartet, kehrt zurück; kein Kampf, keine Lieferung |
| `flotten_spionage` | nur über `flotte_ausspaehen`, nicht über `flotte_senden` | Flottenbericht |
| `system_erkunden` | nur `spionagesonde` | kartiert das System, Bewohnerliste |
| `mond_zerstoeren` | Weltenkrümmer, Mondziel (setzt `zielart` selbst), fremder Mond | Kampf, bei vollständigem Sieg Zerstörungswurf |
| `terraformen` | Terraformer, eigener Planet oder freier Platz, System kartiert; `siedler_volk` = Zielvolk | formt Klima/Felder |

Weltenkrümmer fliegen nur `angriff`, `mond_zerstoeren`, `transport` und `stationieren`. Bodentruppen und Personen
fliegen nie von oder zu Monden.

## flotte_senden: alle Felder

Vollform (Schema-konform, Mondwelt). Unbenutzte Schiffe und Güter stehen auf 0; `siedler_volk` und
`siedler_mischung` sind `null`, solange nicht kolonisiert wird.

```json
{"typ":"flotte_senden","start":"1:27:6","ziel":"1:27:9","mission":"transport",
 "siedler_volk":null,"siedler_mischung":null,"geschwindigkeit":1.0,"haltedauer_stunden":0,
 "startart":"planet","zielart":"planet",
 "schiffe":{"spionagesonde":0,"kleiner_transporter":0,"grosser_transporter":4,"leichter_jaeger":0,
  "bergbauschiff":0,"kreuzer":0,"recycler":0,"kolonieschiff":0,"truppentransporter":0,"schlachtschiff":0,
  "bomber":0,"zerstoerer":0,"abfangjaeger":0,"schwerer_jaeger":0,"sturmjaeger":0,"schnellfrachter":0,
  "schwerlastfrachter":0,"raketenkreuzer":0,"schildkreuzer":0,"flottenkreuzer":0,"sturmschlachtschiff":0,
  "traegerschlachtschiff":0,"belagerungsschlachtschiff":0,"strategischer_bomber":0,"abfangzerstoerer":0,
  "schwerer_zerstoerer":0,"grosses_kolonieschiff":0,"handelsfrachter":0,"pathfinder":0,"bergungsschiff":0,
  "weltentruemmer":0,"schlachtkreuzer":0,"abfang_schlachtkreuzer":0,"schwerer_schlachtkreuzer":0,"terraformer":0},
 "ladung":{"erz":40000,"kristall":20000,"deuterium":5000,"nahrung":0,"legierung":0,"elektronik":0,
  "konsumgut":0,"xenokristall":0,"antriebskern":0,"habitatmodul":0}}
```

- `schiffe`: nur Schiffe (keine Verteidigung, keine `solarsonde`); Bestand je Startplanet in `lage.planeten[].schiffe`.
  Welche Namen eine Welt kennt, sagt `schema?rolle=R` (filtert nach Regelwerk).
- `ladung`: ganze Einheiten. Deuterium in der Ladung ist **Fracht**, getrennt vom Treibstoff. Sonden tragen nichts.
  Frachtraum mit `werkzeug flugzeit` (`ladekapazitaet`) prüfen.
- `geschwindigkeit`: Anteil der Höchstgeschwindigkeit; langsamer spart Treibstoff, die Flotte fliegt nach ihrem
  langsamsten Schiff.
- `haltedauer_stunden`: halten 1–168, abbau 1–48, saven 0–72; sonst 0.
- `siedler_volk`: nur `kolonisieren` (Volk der Siedler) und `terraformen` (Zielvolk).
- `siedler_mischung`: nur `kolonisieren`: `[Bewohner, Fachkräfte, Forscher]` in ganzen Personen; die Summe muss genau
  die Siedlerzahl des Kolonieschiffs ergeben (`werkzeug koloniebedarf`), die Gruppen müssen am Start vorhanden sein.
- `startart`/`zielart`: `planet` oder `mond` (Pflichtfeld im Schema der Mondwelten).

Kurzform, die der Kern ebenso annimmt (fehlende Schiffe/Güter = 0):

```json
{"typ":"flotte_senden","start":"1:27:6","ziel":"1:31:0","mission":"saven","schiffe":{"kreuzer":40,"grosser_transporter":12},
 "ladung":{"erz":60000,"kristall":30000},"geschwindigkeit":0.3,"haltedauer_stunden":6}
```

## Flugplanung und Ankunftszeiten

- `werkzeug flugzeit {start, ziel, schiffe, geschwindigkeit}` → `dauer_sekunden`, `treibstoff_je_strecke`,
  `ladekapazitaet`, `bergungskapazitaet`, `personenplaetze` und eine Tabelle `tempi` für 100 % bis 10 %.
  Mit `"flotte": <id>` statt `start` rechnet es vom Orbit einer eigenen gestrandeten Flotte (Kern-Feld, steht nicht
  im Antwortschema – strenge Clients lassen es weg und nutzen `flotte_starten` mit `probe=true`).
- `werkzeug missionsplan {aktion: <flotte_senden>}` prüft verbindlich nur kolonisieren, abbau, recyceln, terraformen
  und transport/stationieren zwischen eigenen Planeten; sonst `status: unknown` – dann `handeln probe=true`.
- `werkzeug flugkoordination {flug, ankunft, rueckgeschwindigkeit}` → `abflug`, `ankunft`, `rueckkehr`,
  `treibstoff_hin`/`_rueck`, `bereit` und `grund`. Zeiten in Spielsekunden (`lage.sekunden` = jetzt).
- `flotte_planen` startet sofort (`ankunft: null`) oder legt einen Terminflug an. Nur hier gibt es ein eigenes
  Rückflugtempo und `rueckgeschwindigkeit: 0` = **kein Rückflug** (die Flotte strandet nach dem Auftrag am Ziel).
  Gespeicherte Termine: `lage.flugtermine[]`; löschen mit `flugplan_storno {auftrag}`. Neue Antriebsforschung
  verschiebt den Abflug, damit die Ankunft gleich bleibt; Schiffe und Vorräte prüft der Kern erst beim Abflug.

```json
{"typ":"flotte_planen","ankunft":412800,"rueckgeschwindigkeit":0.5,
 "flug":{"typ":"flotte_senden","start":"1:27:6","ziel":"1:29:4","mission":"recyceln","schiffe":{"recycler":12},
  "geschwindigkeit":1.0,"ladung":{},"haltedauer_stunden":0}}
```

Koppeln wie im Einsatzblatt: Ankunft des Recyclers = Ankunft der Angriffsflotte (`lage.flotten[].ankunft_sekunden`)
+ 5 s; Halteflotte beim Partner ein paar Sekunden vor der feindlichen Ankunft (`lage.verbuendete_anfluege[]`).

## Treibstoff, Rückruf, Flottenplätze

- Beim Start bucht der Kern Treibstoff für Hin- **und** Rückflug ab (Stationieren/Kolonisieren nur Hinflug). Die
  Rückkehr braucht kein Deuterium am Ziel. Raumhafen-Stufen senken den Verbrauch, langsameres Tempo ebenfalls
  (`treibstoff_erklaerung` in `werkzeug flugzeit`).
- `flotte_zurueckrufen {flotte}`: im Hinflug kehrt die Flotte nach der bereits geflogenen Zeit um; im Orbit beendet der
  Rückruf Halten, Blockade, Bombardieren, Abbau (Ertrag bis jetzt). Ein Rückruf löst die Flotte aus einem Verband.
  Nicht rückrufbar: vermisste Expeditionen, Flotten auf dem Rückflug, Rückkehrer, die auf ihren Landeorbit warten,
  gestrandete Flotten (dort `flotte_starten`). Flüge ohne Rückflugtreibstoff kehren nur um, wenn genug Deuterium in der
  Ladung ist.
- Flottenplätze: `lage.flottenplaetze {belegt, gesamt}`. Mehr gibt nur Computertechnik (Verwalter forscht).

## Verband (gemeinsamer Angriff)

1. Feldherr startet eine `angriff`-Flotte und öffnet sie: `{"typ":"verband_oeffnen","flotte":17}`.
2. Weitere eigene Angriffsflotten oder solche von Mitgliedern **derselben Allianz** (ein Bündnis genügt nicht) treten
   im Hinflug zum selben Ziel bei: `{"typ":"verband_beitreten","flotte":19,"fuehrung":17}`.
3. Gemeinsame Ankunft = späteste Ankunft; keine Flotte wird um mehr als 6 Stunden gegenüber ihrer ursprünglichen
   Ankunft verzögert, höchstens 16 Flotten. Offene Verbände: `lage.verbaende[]` (`fuehrung`, `ankunft_sekunden`,
   `spaeteste_ankunft_sekunden`, `flotten`/`flotten_max`, `beitretbar`).
4. Fremde Verbände betritt man nur mit einer eigenen Flotte, die schon zum selben Ziel fliegt: erst `flotte_senden`
   (angriff), dann `verband_beitreten`. Verband und Ankunft dem Partner per Brief/Allianzchat nennen (sternenepoche-diplomatie).

## Gestrandete Flotten und Orbitflotten

Gestrandet ist eine Flotte nach einem Flug ohne Rückflug (`flotte_planen` mit `rueckgeschwindigkeit: 0`) oder wenn
der Rückflug fehlt. Erkennbar an `lage.flotten[].gestrandet {seit, nur_heim, deuterium, ort}`; sie belegt einen
Flottenplatz und kostet Unterhalt. Treibstoff kommt nur aus ihrer **Ladung**.

```json
{"typ":"flotte_starten","flotte":23,"ziel":"1:27:6","mission":"stationieren","geschwindigkeit":1.0,
 "rueckgeschwindigkeit":0,"haltedauer_stunden":0,"zielflotte":null}
```

- Missionen vom Orbit: `stationieren`, `transport`, `halten`, `spionage`, `angriff` (`zielflotte` = gestrandete
  Feindflotte). Nach einem Angriff vom Orbit (`nur_heim: true`) nur noch Stationieren oder Übergabe.
- Reicht das Deuterium für beide Strecken, kehrt sie an den Orbitort zurück, sonst strandet sie am neuen Ziel.
- Nachtanken: `flotte_versorgen` (Transport vom eigenen Planeten direkt in die Ladung; der Laderaum der Empfängerflotte
  muss reichen). Gleiches gilt für eigene Blockade- oder Bombardierflotten im Orbit:

```json
{"typ":"flotte_versorgen","start":"1:27:6","ziel":"1:29:4","versorgungsflotte":23,
 "schiffe":{"grosser_transporter":2},"geschwindigkeit":1.0,"ladung":{"deuterium":20000}}
```

- Übergeben/verkaufen an den Besitzer des Zielplaneten: `orbitflotte_anbieten {flotte, ziel, preis}` (sternenepoche-handel).
- Fremde gestrandete Flotten (sichtbar für Kenner des Systems, `lage.orbitflotten_bekannt`) angreifen:
  `{"typ":"orbitflotte_angreifen","start":"1:27:6","zielflotte":31,"schiffe":{"kreuzer":30},"geschwindigkeit":1.0,"rueckgeschwindigkeit":1.0}`.

## Saven bei Angriff

1. `lage.angriffe[]` zeigt `ziel`, `ankunft_sekunden`, `sichtstufe`. Ankunft minus jetzt = Zeitfenster.
2. Freien Flottenplatz und Treibstoff prüfen. Ohne Platz zuerst eine Sonde zurückrufen.
3. Ziel wählen, das nicht der Startplanet ist: am einfachsten Position 0 des eigenen Systems (Gürtel) oder ein
   eigener ruhiger Planet. Rückkehr deutlich **nach** der feindlichen Ankunft planen (Hinflug + Wartezeit + Rückflug).
4. Ladung mitnehmen, was ungeschützt über dem Bunker liegt (Plünderung, sternenepoche-kampf).
5. Senden, bevor eine Blockade steht – ein blockierter Planet lässt nur den Angriff auf die Blockade starten.
6. Gebäude, Verteidigung, Bevölkerung und Credits lassen sich nicht saven.

Verwalter und Feldherr dürfen saven; bei Kampfflotten saved der Feldherr, Frachter darf der Verwalter selbst retten.

## Personen, Truppen, Beladung

- **Personen** (`personen_senden`, nur `transport`/`stationieren` zwischen Planeten): `personen.gruppen` ist
  `[Volk][Bewohner, Fachkräfte, Forscher]` in **Tausendstel-Personen** (1000 = eine Person, nur Vielfache von 1000),
  Völker in der Reihenfolge aurelianer, krath, veyari, syntheten. Bestand: `lage.planeten[].personengruppen`.
  Personenplätze kommen von den Schiffen (`werkzeug flugzeit` → `personenplaetze`); mindestens 500 Personen bleiben.

```json
{"typ":"personen_senden","personen":{"gruppen":[[0,200000,50000],[0,0,0],[0,0,0],[0,0,0]]},
 "flug":{"typ":"flotte_senden","start":"1:27:6","ziel":"1:27:9","mission":"transport",
  "schiffe":{"grosser_transporter":2},"geschwindigkeit":1.0,"ladung":{},"haltedauer_stunden":0}}
```

- **Truppen** (`boden_senden`, Feldherr): `truppen` = 12 Zahlen in der Reihenfolge infanterie, panzer, drohnen,
  infanterie_schwer, infanterie_pionier, infanterie_praezision, panzer_schwer, panzer_belagerung, panzer_flugabwehr,
  drohnen_aufklaerung, drohnen_brecher, drohnen_reparatur. Platz: 100 gewichtete Truppenplätze je `truppentransporter`.
  Flugmissionen: transport, stationieren, kampfkolonisieren, kolonisieren, halten (Bodenhilfe), bombardieren
  (Landung in Invasion v2). Bestand: `lage.planeten[].bodentruppen.bestand`. Bodenhilfe bei Verbündeten heißt im
  Schema `halten` (der Kern nimmt auch das ältere `verstaerken`); dabei `haltedauer_stunden` 1–168 ausdrücklich setzen –
  nur ein fehlendes oder `null`-Feld ergänzt der Kern auf 48, eine 0 lehnt er ab.

```json
{"typ":"boden_senden","truppen":[60,10,10,0,0,0,0,0,0,0,0,0],
 "flug":{"typ":"flotte_senden","start":"1:27:6","ziel":"1:27:9","mission":"stationieren",
  "schiffe":{"truppentransporter":2},"geschwindigkeit":1.0,"ladung":{},"haltedauer_stunden":0}}
```

- **flotte_beladen**: ein Flug mit Personen **und/oder** Truppen in einem Befehl (`personen`, `truppen`, `flug`;
  leere Teile mit Nullen). Mit Personen nur transport/stationieren.

## Expedition und Rettung (expedition.rs)

- `expedition_starten {flug}`: `flug` ist `saven` zur **Position 0** eines Systems, höchstens 10.000 Schiffe,
  höchstens 20 Spionagesonden (Bordsonden) je Pathfinder. Der Kern zieht zusätzlich Bordversorgung für alle Plätze ab.
  Status und Funde: `lage.expeditionen[]` (`status`, `ereignisse`, `rettungsfrist`, `berichte`).
- Ein Rückruf einer aktiven Expedition beendet sie vorzeitig (einmaliges Gesamtrisiko wird gewürfelt).
- Vermisst: Rettung innerhalb von 24 Stunden nach der Vermisstenmeldung starten, direkter `saven`-Flug mit mindestens
  einem `bergungsschiff` zum Vermisstenziel: `{"typ":"rettung_starten","expedition":41,"flug":{…saven…}}`.
- `expedition_reparieren {flotte, einheit}`: Notreparatur eines Rumpfs durch ein mitfliegendes Bergungsschiff.

## Mondflüge, Sprungtor, Hyperraumloch (Kurzfassung, Details: sternenepoche-monde-dyson)

- Mondziel: `"zielart":"mond"`, Koordinate = Planetenkoordinate. Erlaubt: transport, stationieren (eigener Mond),
  halten (eigener/verbündeter Mond), spionage, angriff, blockade (Stufe IV), mond_zerstoeren.
- Mondstart: `"startart":"mond"`, braucht Mondbasis 1; Schiffe aus dem Mondbestand.
- `hyperraumloch_oeffnen {mond, gegenstelle, erstdurchreise}` (Feldherr) öffnet ein Tor zu einem zweiten
  Hyperraumtor-Generator; `hyperraumloch_durchreisen {loch, von, schiffe, danach, ladung}` schickt Schiffe hindurch;
  `sprungtor_freigabe {mond, freigegeben}` (Feldherr, Diplomat) gibt den eigenen Generator Partnern frei. Weltenkrümmer,
  Verteidigung und Reiche im Anfängerschutz reisen nicht; am Austritt kämpft die Torwache gegen alle ohne Bündnis oder NAP.

## Zusammenarbeit mit Verbündeten

- **Halten**: nur bei Allianzmitgliedern oder Verteidigungsbündnis-Partnern. Ankommende Angriffe auf Partner stehen in
  `lage.verbuendete_anfluege[]`; eigene und fremde Halteflotten in `lage.verbuendete_im_orbit[]`.
- **Verband**: nur innerhalb derselben Allianz; Absprache über Ziel, Führungsflotte und spätestmögliche Ankunft.
- **Hilferuf** des Partners (`lage.kommunikation.allianzbereich.hilfe[]`): Zusage mit `allianz_hilfe_status
  {hilfe, erledigt:false}` dokumentiert nur – die Hilfe ist erst der Flug (`halten`, `boden_senden`, `transport`).
- Lieferungen an Partner brauchen einen **eigenen Spionagebericht** des Zielplaneten (Ausnahme: `halten`).

## Ablauf eines Flottenzugs

1. `agenda?rolle=feldherr` (bzw. verwalter) und `pinwand?rolle=…` lesen: Aufträge, Angriffe, Rückkehrer.
2. `lage?rolle=…&kompakt=1&teile=flotten,angriffe,flottenplaetze,verbaende,truemmer,berichte` holen.
3. Ziel und Mission nach dem Baum wählen; Bericht nicht älter als nötig (`alter_stunden`).
4. `werkzeug flugzeit` / `flugkoordination`, bei Feindzielen `kampfsimulator` (sternenepoche-kampf).
5. `handeln probe=true` → echt → Pinwand-Karte mit `beleg_request_id` auf `erledigt`.
6. Ankunftszeit und Rückkehr in die Notiz schreiben und einen Wecker kurz vor der Ankunft setzen.
