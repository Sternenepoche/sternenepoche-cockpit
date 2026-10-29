---
name: sternenepoche-monde-dyson
description: Laden, wenn ein Reich Monde hat oder angreift (Mondgebäude, Werftdock, Weltenkrümmer, Tiefenphalanx, Hyperraumtor, Ortungsschleier, Fähre, Mondflüge, Mondzerstörung) oder wenn die Allianz eine Dysonsphäre plant, finanziert, baut oder nutzt.
---

# Monde und Dysonsphäre

## Kurz

- Monde gibt es nur in Welten mit Mondgeneration (`lage.monde.aktiv`); die Dysonsphäre nur, wenn `lage.dyson.aktiv`. Sonst lehnt der Kern ab („Monde sind in dieser Partie nicht aktiv“, „Die Dysonsphäre gibt es in dieser Welt nicht“).
- Ein Mond hat **dieselbe Koordinate** wie sein Planet. Mondaktionen nennen ihn mit `mond: "S:S:P"`, Flüge mit `zielart: "mond"` bzw. `startart: "mond"`.
- Mondgebäude zahlt der **Mondbestand**. Güter erst per Transport (`zielart: mond`) oder Fähre hinbringen.
- Hyperraumtor und Tiefenphalanx brauchen volle Energie und bezahlten Mondunterhalt.
- Dysonsphäre: Allianzleitung schlägt vor und startet; jedes Projektmitglied zahlt seinen Anteil von eigenen Planeten oder Monden im System ein; Bau erst, wenn alle 12 Plätze der Allianz gehören und mindestens 10 Mitglieder voll eingezahlt haben.
- Quellen: `crates/kern/src/monde.rs`, `regeln/monde.ron`, `crates/kern/src/dyson.rs`, `regeln/dyson.ron`; Regeltext mit `werkzeug regel {"stichwort":"mond"}` bzw. `"dyson"`.

## Teil A: Monde

### A1 Entstehung und Eigentum

- Ein Mond entsteht nur nach einem **Kampf an einem besiedelten Planeten ohne Mond**. Chance 1 % je 2.000 Trümmer (Erz plus Kristall) **dieses Kampfes**, höchstens 40 % (`truemmer_je_prozent` 2.000, `chance_max_prozent` 40). Vorhandene Trümmer zählen nicht; Kämpfe am Mond, Raketen, Bodenkämpfe und Expeditionen erzeugen nie einen Mond. Ergebnis im Kampfbericht (`kampfberichte[].mondwurf`) und in `lage.monde.mondwuerfe`.
- Der Mond gehört dem **Planetenbesitzer**, nicht dem Sieger. Er geht bei Eroberung mit und wird beim Aufgeben des Planeten gelöscht. `planet_aufgeben` ist gesperrt, solange Schiffe auf dem Mond stehen oder vom Mond gestartete Flotten unterwegs sind.
- Durchmesser ⌊√(x · 10⁶ + 30 · T)⌋ km mit x 10–20 und T Trümmern bis 2.000.000. Felderobergrenze ⌊d²/10⁶⌋ (Typ `schluucht_tief` +25 %); nutzbar sind 1 Grundfeld plus 3 je Mondbasisstufe bis zur Obergrenze. Die Mondbasis lässt sich auch auf vollem Mond eine Stufe weiter bauen.
- Ein Mond belegt keinen Kolonieplatz und hat keine Bevölkerung, keine Forschung, keinen Markt.
- Zehn Typen (`typ`: `krater_grau`, `basalt_dunkel`, `eis_rissig`, `mineral_blau`, `organisch_gruen`, `rost_staub`, `schluucht_tief`, `vulkan_glut`, `amethyst_violett`, `karmin_rot`) mit je einem Vorteil und Nachteil; Werte unter `lage.monde.eigene[].typ_regel` und in `regeln/monde.ron`.

### A2 Mondsicht

| Pfad | Inhalt |
|---|---|
| `planeten[].mond` | Kartensicht: `id`, `koord`, `typ`, `durchmesser`, `besitzer`, `eigen`, `bedroht`, `sprungtor` |
| `monde.eigene[]` | voll: `felder{belegt,verfuegbar,obergrenze}`, `gebaeude`, `bauschleife`, `baubar[]` (`stufe`, `kosten`, `fehlt`, `fehlende_rohstoffe`, `moeglich`), `bestand`, `lager`, `foerderung_je_stunde`, `energie{erzeugung,verbrauch,faktor_promille,je_solarsonde}`, `schiffe`, `verteidigung`, `werftdock{stufe,schleife,einheiten[]}`, `sprungtor{stufe,bereit,bereit_ab,freigegeben,reichweite_systeme,kapazitaet,ziele[]}`, `tiefenphalanx{stufe,stufe_wirksam,reichweite_systeme,bereit_ab,scan_deuterium}`, `faehren`, `faehre_deckel_je_stunde`, `unterhalt{credits_je_tag,verzug}`, `blockiert` |
| `monde.bekannt[]` | fremde Monde in erfassten Systemen |
| `monde.anfluege_auf_monde[]` | feindliche Anflüge auf eigene Monde (`flotte`, `mond`, `ankunft`) |
| `monde.hyperraumloecher[]` | sichtbare offene Löcher |
| `monde.phalanxberichte[]`, `monde.spionageberichte[]` | eigene Scans und Mondspionage |
| `monde.regeln` | Gebäuderegeln, Lochdauer, Weltenkrümmer-Werftdock |

### A3 Mondgebäude (`mond_bauen`, Verwalter)

```json
{"typ":"mond_bauen","mond":"1:27:6","gebaeude":"mondbasis"}
```

Werte aus `regeln/monde.ron` (Kosten der Stufe n = Grundkosten × Faktor^(n−1)):

| Gebäude | ab Stufe | braucht | Grundkosten | Energie | Wirkung |
|---|---|---|---|---|---|
| `mondbasis` | IV | – | 10.000 Erz, 20.000 Kristall, 10.000 Deuterium (Faktor 1,45) | erzeugt Grundenergie | +3 Felder je Stufe, Raumhafen und Bauhof des Mondes, Lager, Fähre |
| `regolithfoerderung` | IV | Mondbasis 1 | 4.000 Erz, 2.000 Kristall, 100 Legierung | 15 | fördert Erz und Kristall ohne Arbeiter |
| `kryolager` | IV | Mondbasis 1 | 5.000 Erz, 3.000 Kristall, 200 Legierung | 5 | Deuteriumlager, ein Viertel plünderungssicher |
| `tiefenphalanx` | IV | Mondbasis 1, Spionagetechnik 2, Überwachungstechnik 1 | 10.000 Erz, 20.000 Kristall, 5.000 Deuterium, 500 Elektronik | 40 | scannt Flüge eines Planeten |
| `werftdock` | IV | Mondbasis 2 | 20.000 Erz, 10.000 Kristall, 1.000 Legierung, 300 Elektronik | 30 | Mondwerft und Reparaturdock; ab Stufe 8 Weltenkrümmer |
| `ortungsschleier` | IV | Mondbasis 2, Abschirmtechnik 1 | 8.000 Kristall, 4.000 Deuterium, 400 Elektronik | 25 | erschwert Spionage, verbirgt Lochöffnungen; höchstens Stufe 5 |
| `sprungtor` | V | Mondbasis 3, Hyperraumantrieb 3 | 500.000 Erz, 1.000.000 Kristall, 500.000 Deuterium, 20.000 Legierung, 10.000 Elektronik, 3.000 Xenokristall (Faktor 1,6) | 150 | Hyperraumtor-Generator; höchstens Stufe 5 |

- `baubar[].moeglich` sagt, ob der nächste Bau jetzt geht; `fehlt` nennt Stufe, Gebäude, Forschung, Feld; `fehlende_rohstoffe` die Güter im **Mondbestand**.
- Mondbauschleife: so lang wie die Planetenschleife (5). Ablehnungen: „Mondbestand reicht nicht: G fehlt (Mond K; Nachschub per Transport oder Fähre)“, „Keine freien Felder auf dem Mond (b/v); Mondbasis ausbauen …“, „Mondbauschleife ist voll“.
- Energie kommt aus der Mondbasis und Solarsatelliten; fehlt sie, sinkt die Wirkung anteilig.
- Unterhalt: 150 + 20 Credits je Gebäudestufe und Tag aus der Reichskasse. Im Verzug halbe Wirkung, Tor und Phalanx gesperrt.

### A4 Lager, Förderung, Fähre

- Mondlager: halbe Planetenkapazität, mit der Mondbasisstufe als Lagerstufe; Kryolager 20.000 × 1,6^(n−1) Deuterium, davon 25 % plünderungssicher. Sonst schützt auf dem Mond nichts.
- Regolithförderung: 15 Erz und 10 Kristall je Stunde als Grundertrag, plus Typspuren.
- Fähre (`mond_faehre`, Verwalter): Daueranweisung je Gut, `menge_je_stunde` positiv zum Mond, negativ zum Planeten, 0 beendet. Alle Güter einer Fähre zusammen höchstens 2.000 je Stunde und Mondbasisstufe; keine Bauteile; Richtung Planet nur bis zu dessen Lagergrenze; ruht bei Blockade; kein Flug, unsichtbar für die Phalanx.

### A5 Werftdock, Weltenkrümmer, Reparaturdock (`mond_fertigen`)

```json
{"typ":"mond_fertigen","mond":"1:27:6","einheit":"weltentruemmer","anzahl":1}
```

- Das Werftdock zählt als Werftstufe: jede Einheit braucht Werftdock ≥ ihre Werftstufe, der **Weltenkrümmer** (`weltentruemmer`) Werftdock 8 (`weltentruemmer_werftdock`). Planetenschilde gibt es nur auf Planeten; eine Planetenwerft lehnt Weltenkrümmer ab („… entstehen nur im Werftdock … eines Mondes: mond_fertigen“).
- Bezahlt wird erst aus dem Mondbestand, der Rest vom eigenen Mutterplaneten. Die Besatzung kommt vom Mutterplaneten; dort müssen mindestens 500 Einwohner bleiben.
- Rollen: Verwalter nur `solarsonde`, alles andere der Feldherr („… bestellt der Feldherr, nicht der Verwalter“).
- Weltenkrümmer (Werte in `regeln/flottenausbau.json`): Zivilisationsstufe V, Hyperraumantrieb 5, Waffentechnik 8, Kosten unter anderem 20 Antriebskerne – `werkzeug kosten {"einheit":"weltentruemmer"}`. Er fliegt gegen Monde nur `angriff`, `mond_zerstoeren`, `transport`, `stationieren` und passt durch kein Hyperraumloch.
- Reparaturdock: Nach einem Kampf am Mond oder seinem Planeten, in dem du verteidigt hast, setzt es 3 % je Stufe (höchstens 30 %) der verlorenen Schiffe instand (10 % Kosten, 10 % Bauzeit); `monde.eigene[].reparatur`.

### A6 Flüge zu und von Monden

`flotte_senden` mit `zielart` und/oder `startart`:

```json
{"typ":"flotte_senden","start":"1:27:6","ziel":"1:27:6","zielart":"mond","mission":"transport",
 "schiffe":{"grosser_transporter":2},"ladung":{"deuterium":40000},"geschwindigkeit":1.0}
```

| Mission gegen einen Mond | Bedingung | Rolle |
|---|---|---|
| `transport` | beliebiger Mond | Verwalter, Feldherr |
| `stationieren` | nur eigener Mond, nicht derselbe wie der Start | Verwalter, Feldherr |
| `halten` | eigener oder verbündeter Mond, 1 bis 168 Stunden | Feldherr |
| `spionage` | fremder Mond, nur Sonden, System kartiert | Feldherr |
| `angriff`, `blockade`, `mond_zerstoeren` | fremder Mond ohne Bündnis; Blockade ab Stufe IV; `mond_zerstoeren` mit mindestens einem Weltenkrümmer (setzt `zielart` selbst auf `mond`) | Feldherr |

- Start vom Mond (`startart:"mond"`) braucht Mondbasis 1; Schiffe, Ladung und Treibstoff kommen aus dem Mond. Vom Mond starten nicht `kolonisieren`, `kampfkolonisieren`, `invasion`, `bombardieren`, `mond_zerstoeren`, `terraformen` („Mission … startet nicht von einem Mond“).
- Flug Planet ↔ eigener Mond dauert mindestens die Mindestflugzeit (20 Spielminuten). Treibstoff vom Planeten zahlt der Rollentopf, vom Mond der Mondbestand.
- Monde haben keine Bevölkerung: `personen_senden` zu oder von Monden lehnt der Kern ab.
- Raketen auf Monde: `raketen_starten` mit `zielart:"mond"` (Feldherr) treffen nur Anlagen und Solarsatelliten.
- Kampfvorschau gegen einen Mond: `werkzeug kampfsimulator {"ziel":K,"zielart":"mond","schiffe":{…}}` braucht einen eigenen Mondspionagebericht.

### A7 Tiefenphalanx (`phalanx_scannen`, Feldherr)

```json
{"typ":"phalanx_scannen","mond":"1:27:6","ziel":"1:29:4"}
```

- Scannt einen **Planeten** im eigenen Sektor; Reichweite ⌊(Stufe² − 1)/2⌋ Systeme (`monde.eigene[].tiefenphalanx.reichweite_systeme`), 5.000 Deuterium aus dem Mondbestand je Scan, danach 300 s Abklingzeit. Monde sind nicht scannbar.
- Sie zeigt Flüge zum und vom Ziel und Flotten in seinem Orbit; unsichtbar bleiben Flüge zwischen zwei Monden und Hyperraumdurchreisen, ebenso Fähren und Raketen. Der Gescannte erfährt nichts.
- Detailtiefe: kleinere Stufe aus Phalanx und Überwachungstechnik minus Abschirmtechnik der Flotte.
- Ablehnungen: „Tiefenphalanx braucht volle Energie und bezahlten Unterhalt“, „Tiefenphalanx lädt noch bis …“, „Ziel außerhalb der Reichweite …“, „Scan kostet N Deuterium aus dem Mondbestand“.

### A8 Hyperraumtor-Generator und Hyperraumloch (Feldherr)

Öffnen:

```json
{"typ":"hyperraumloch_oeffnen","mond":"1:20:6","gegenstelle":"1:45:9",
 "erstdurchreise":{"schiffe":{"schlachtkreuzer":40},"danach":"landen"}}
```

- Gegenstelle ist ein anderes Tor: eigenes oder das eines Allianz- oder Bündnispartners, der es mit `sprungtor_freigabe {"mond":K,"freigegeben":true}` (Feldherr oder Diplomat) freigegeben hat.
- Beide Tore: volle Energie, Unterhalt bezahlt, nicht in Abklingzeit, nicht blockiert, kein Feind im Orbit, kein offenes Loch.
- Reichweite 15 Systeme je Stufe des **schwächeren** Tors; Sektorwechsel ab Torstufe 4 (zählt 30 Systeme Aufschlag). Ablehnungen nennen den Abstand: „Gegenstelle N Systeme entfernt; Reichweite M“. Kosten 20.000 + 2.000 Deuterium je System aus dem Mondbestand des Öffners (`sprungtor.ziele[].deuterium` nennt sie vorab).
- Das Loch ist 300 Spielsekunden offen, nicht vorzeitig schließbar, **öffentlich**: Jeder, der es sieht (eigene Planeten, Monde oder Orbitflotten im Start- oder Zielsystem, Partner, ferne Phalanx über dem Ortungsschleier), darf in beide Richtungen mitreisen. Danach Abklingzeit für beide Tore (120 min, −10 % je Stufe, mindestens 60 min).

Durchreisen:

```json
{"typ":"hyperraumloch_durchreisen","loch":3,"von":{"art":"mond","koord":"1:20:6"},
 "schiffe":{"kreuzer":20},"danach":"orbit","ladung":{"deuterium":30000}}
```

- `loch`: `lage.monde.hyperraumloecher[].id`; `von.art` `planet`, `mond` (mit `koord`) oder `flotte` (mit `flotte`, nur Flotten im Orbit); `mitreisen_moeglich_von` nennt passende Orte.
- Kapazität 2.000.000 Struktur je Torstufe, zuerst der Öffner, dann nach Befehlsreihenfolge. Jede Flotte und jeder Herkunftsort einmal je Loch.
- `danach: "landen"` nur am eigenen Austrittsmond; `orbit` bleibt an der Koordinate (am fremden Mond höchstens 72 Stunden, dann Heimflug). Wer im Orbit bleibt, zahlt den späteren Rückflugtreibstoff sofort – Deuterium in `ladung` mitnehmen.
- Torwache: Am Austritt bei einem Mond ohne Allianz, Bündnis oder NAP folgt sofort ein Kampf.
- Nie durch ein Loch: Weltenkrümmer, Solarsatelliten, Verteidigung, Expeditionen, Reiche unter Anfängerschutz.

### A9 Ortungsschleier

Jede Stufe hebt die Schwellen einer Spionage auf diesen Mond um eins und verbirgt die Lochöffnung vor fernen Phalanxen, deren Stufe nicht höher ist. Höchstens Stufe 5. Gegenstück beim Angreifer: mehr Sonden, höhere Spionagetechnik.

### A10 Mondzerstörung

- Flotte mit mindestens einem Weltenkrümmer, Mission `mond_zerstoeren`, gegen einen fremden Mond. Erst normaler Kampf gegen alles auf dem Mond; **nur bei vollständigem Sieg** mit überlebendem Weltenkrümmer folgt der Versuch.
- Chance (100 − √d) · √n %, d wirksamer Durchmesser (Basaltmond 15 % größer, Eismond 10 % kleiner), n überlebende Weltenkrümmer. Verlustchance √d/2 %: dann sind **alle** Weltenkrümmer der Flotte verloren (`kernwerte.ron`, `mond`).
- Folgen: Mond, Gebäude, Bestände verschwinden; Trümmerring (20 Erz und 10 Kristall je km, dazu Lager- und Gebäudeanteil); die Hälfte der Besatzung stationierter Schiffe überlebt und kann mit `recyceln` (Recycler, Bergungsschiffe) geborgen werden; der Planet verliert 72 Stunden lang 5 Stabilität; Fähren enden, offene Löcher schließen, Flüge zum Mond kehren um oder fliegen zum Planeten.
- Abwägung mit `werkzeug kampfsimulator` (Mondbericht) und dem Durchmesser aus `monde.bekannt[]`: kleine Monde sind leichte Beute, große teuer.

### A11 Mondroutine

- Verwalter: Mondbasis und Regolith zuerst, Kryolager für Treibstoffdepot, Satelliten für Energie, Fähre für Nachschub; `baubar[].fehlende_rohstoffe` per Fähre oder Transport auffüllen.
- Feldherr: Verteidigung im Werftdock, Flotte bei Angriff vom Planeten auf den Mond retten (Mondflüge sind für fremde Phalanx sichtbar, Mond-zu-Mond-Flüge nicht), Phalanx vor eigenen Angriffen, Tor nur mit starker Torwache öffnen.
- Unterhalt im Blick: Credits für alle Monde täglich (`unterhalt.credits_je_tag`).

## Teil B: Dysonsphäre (Allianz-Großprojekt)

### B1 Ablauf

1. **Vorschlagen** – Allianzleitung (die obersten vier Ränge; `lage.kommunikation.allianz.leitung` bzw. `dyson.projekte[].leitung`), Rollen Diplomat oder Stratege:
   `{"typ":"dyson_vorschlagen","system":"1:27"}` – die Allianz braucht dort mindestens einen Planeten; je System eine Sphäre, je Allianz ein offenes Projekt; nach einem Abbruch ist das System gesperrt (`dyson.gesperrt[]`).
2. **Beitreten** – jedes Allianzmitglied mit eigenem Planeten im System (Stratege oder Diplomat): `{"typ":"dyson_beitreten"}` (ohne `projekt` das Projekt der eigenen Allianz). Nur ein offenes Projekt je Spieler.
3. **Einzahlen** – Verwalter oder Stratege: `{"typ":"dyson_einzahlen","von":"1:27:4","erz":2000000,"kristall":2000000,"deuterium":1000000}` (`zielart:"mond"`, wenn vom Mond). Nur von eigenen Planeten oder Monden **im System**, je genanntem Gut mindestens die Mindestbuchung (`dyson.mindestbuchung`) oder genau der Rest, keine Überzahlung, nicht für andere. Kein Topf; die Güter gehen direkt vom Bestand.
4. **Bau starten** – Allianzleitung: `{"typ":"dyson_bau_starten"}`. Der Kern prüft alle Bedingungen gleichzeitig und nennt die fehlenden mit Ist/Soll.
5. **Anschlüsse wählen** – nach Vollendung, Allianzleitung: `{"typ":"dyson_anschluss_waehlen","slot":1,"planet":"1:27:4"}`; lösen mit `dyson_anschluss_loesen`.
6. **Auszahlen** – Guthaben aus Austritt oder Abbruch: `{"typ":"dyson_auszahlen","planet":K,"erz":…,"kristall":…,"deuterium":…}` (Verwalter oder Stratege, jeder eigene Planet, bis zur Lagergrenze).

Austreten nur vor Baubeginn (`dyson_austreten`, 90 % zurück). Abbrechen: `{"typ":"dyson_abbrechen","bestaetigt":true}` (Allianzleitung; vor Bau 100 %, im Bau 75 % des nicht verbauten Teils zurück, danach 30 Tage Sperre).

### B2 Bedingungen (`regeln/dyson.ron`)

`dyson.projekte[].bedingungen[]` (`kennung`, `text`, `ist`, `soll`, `erfuellt`):

- `plaetze`: alle 12 Plätze des Systems von Allianzmitgliedern besiedelt.
- `mitglieder`: mindestens 10 Projektmitglieder mit Planet im System.
- `eingezahlt`: alle Projektmitglieder voll eingezahlt (mindestens 10).
- `stufe`: alle auf Zivilisationsstufe V.
- `mitgliedschaft`: alle seit mindestens 7 Spieltagen in der Allianz.

Anteil je Gut und Mitglied: `dyson.anteil` (aus der Epochenlänge abgeleitet; bei 365 Tagen 5 Millionen, voll 1 Milliarde ab 730 Tagen, unter 190 Tagen keine Sphäre). Die Projektkasse steht in `dyson.projekte[].kasse`, der eigene Stand in `mitglieder[].eingezahlt`.

### B3 Bau und Wirkung

- Bauzeit 15 Spieltage. Fällt eine Bedingung weg (Platz verloren, weniger als 10 voll eingezahlte Mitglieder mit Planet im System), **ruht** der Bau (`ruht`, `ruhe_grund`, `ruhe_rest_sekunden`); nach insgesamt 14 Ruhetagen Abbruch.
- Wirkung erst nach Vollendung: Solarkraftwerke und Solarsatelliten der Eigner-Allianz im System mit Sonnenfaktor × 1,6; fremde Planeten im System bekommen nur noch einen Teil der Sonne (`kernwerte.ron`, `sonne.dyson_schatten` 0,5).
- Bis zu 4 Anschlüsse (einer je Spieler), aktiv nach 72 Stunden, danach 7 Tage gesperrt: feste Energieeinspeisung, volle Energiedeckung, Strahlsynthese von Deuterium und Xenokristall (absolute Mengen × Anteil/1 Mrd., `dyson.regeln`), Deuteriumsynthesizer und Xenoextraktor × 3, Xenosynthese ohne Abschlag. Erträge enden an der Lagergrenze und sind plünderbar.
- Die Sphäre ist kein Kampfziel. Sie wirkt, solange die Eigner-Allianz mindestens 6 Plätze hält; sonst verwaist sie und geht an eine Allianz, die 7 Tage alle Plätze mit mindestens 10 Mitgliedern hält.
- Galaxie: `werkzeug galaxie` zeigt laufende Sphären unter `systeme[].dyson`.

### B4 Unter Angriff fertigbauen (zehn Reiche, eine Allianz)

Die Agenda führt jede Rolle; Vorlagen sind gültige Aktionen:

1. Leitung (Stratege): Punkt `dyson_vorschlagen`, sobald die Allianz alle Plätze eines Systems hält. Danach ein
   `beschluss` auf der **Allianz-Pinwand** (`bereich:"allianz"`, `an:["alle"]`) mit System, Frist und Pflichten.
2. Jedes Reich: `dyson_beitreten` (Diplomat/Stratege), `dyson_einzahlen` (Verwalter; die Vorlage nennt den zahlbaren
   Teil je Systemplanet – fehlt Bestand, erst Rohstoffe per Transport hinbringen). Zusage auf der Allianz-Pinwand.
3. Leitung: `dyson_sammeln` nennt fehlende Bedingungen und säumige Mitglieder → `auftrag` an genau diese Spieler;
   `dyson_bau_starten`, sobald alles erfüllt ist.
4. Verteidigen: Ein Angriff auf einen Planeten im Dyson-System erscheint mit Hinweis „DYSON-SYSTEM“. Sofort
   `allianz_hilfe` (Vorlage). Jeder Feldherr der Allianz sieht den Hilferuf als Priorität 1: Kampfflotte mit
   `flotte_senden` `mission:"halten"` (`haltedauer_stunden`) zum bedrohten Planeten, vorher `werkzeug kampfsimulator`;
   Zusage mit `allianz_hilfe_status`. Halte-Flotten kämpfen an der Seite des Besitzers und unterbrechen auch eine
   laufende Bombardierung (Entsatz).
5. Platz verloren: Punkt `dyson_bau_ruht` bei allen Rollen nennt Grund, Restfrist (14 Ruhetage bis zum Abbruch) und
   jeden Platz außerhalb der Allianz. Die Leitung gibt einen `auftrag` an ein Mitglied mit Eroberungsflotte; zurück
   mit `boden_senden` (`flug.mission:"bombardieren"`, Bomber + Eskorte + Truppentransporter mit Truppen) und
   `kampfkolonisieren` (Kolonieschiff), freie Plätze mit `kolonisieren` und bewaffneter Eskorte. Der neue Besitzer muss
   nur Allianzmitglied sein. Sobald alle Plätze wieder in Allianzhand sind, läuft der Bau weiter.
6. Fortschritt: `dyson.projekte[].fortschritt_promille`, Ereignisse bei 25/50/75 % und bei Vollendung; danach
   `dyson_anschluss` (Leitung).

### B5 Rollen im Projekt

| Aufgabe | Rolle |
|---|---|
| Vorschlag, Baustart, Abbruch, Anschlüsse | Diplomat oder Stratege, nur Allianzleitung |
| Beitritt, Austritt | Stratege oder Diplomat |
| Einzahlen, Auszahlen | Verwalter oder Stratege |
| System voll besiedeln (alle 12 Plätze) | Verwalter (Kolonien) und Feldherr (Eskorte, Kampfkolonisierung) |
| Bedingungen der Mitglieder (Stufe V, 7 Tage Allianz) | Diplomat über den Allianzkanal |

Absprache: Der Stratege fasst einen `beschluss` mit System und Zielzeitpunkt; der Verwalter plant die Einzahlungen nach Förderrate und Lagergrenze (Anteil in Teilzahlungen über mehrere Tage, Lager vorher ausbauen); der Diplomat verfolgt die Bedingungen aller Mitglieder (`bedingungen[]`) und meldet Lücken als `befund`.
