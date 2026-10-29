---
name: sternenepoche-wirtschaft
description: Laden, wenn der Verwalter (oder Stratege) Planeten ausbaut, Energie, Nahrung, Personal, Stabilität, Lager, Bau- und Forschungsschlangen, Produktionsgrade, Transporte, Kolonien, Zivilisationsstufen oder Monde wirtschaftlich steuert – bis 15 Planeten.
---

# Wirtschaft: 15 Planeten und ihre Monde verwalten

## Kurz

- **Engpass zuerst:** Energie < Verbrauch, Versorgung unter 100 %, fehlende Arbeitskräfte, Stabilität unter 30 – das bremst alles andere. Die Fakten stehen in `lage.warnungen`.
- Bauen nur, was in `planeten[].baubar[]` **ohne `braucht`** steht; `fehlt` nennt fehlende Güter, `strom_plus` den Mehrverbrauch der nächsten Stufe.
- Eine Bauschleife hat 5 Plätze; der vorderste Auftrag zahlt erst beim Start. Fehlt ihm etwas, warten auch alle dahinter.
- Werften und Orbitalwerft ruhen bei Strommangel oder Personalmangel (`fertigung[].ruht_grund`); Bauaufträge laufen auch bei Strommangel weiter.
- Je Zug wenige Planeten im Detail (`lage?planet=K`), die anderen über `kompakt`, Agenda und Warnungen.
- Kosten nie schätzen: `werkzeug kosten`, `kostenplan`, `wirtschaftsdiagnose`, `ablaufplan`.
- Werte stammen aus `regeln/online-v1.ron`, `regeln/kernwerte.ron`, `regeln/flottenausbau.json`, `regeln/technik.ron`; wirksam ist, was `werkzeug regelkontext` und `regel` melden.

## 1 Was ein Planet zeigt

`lage.planeten[]` (voll mit `lage?rolle=verwalter&planet=K`):

| Feld | Bedeutung |
|---|---|
| `bestand`, `rate`, `lager`, `voll_in_stunden` | Vorrat, Nettorate je Stunde (Produktion minus Verbrauch, nicht noch einmal abziehen), Lagergrenze, Stunden bis voll |
| `vorrat_reicht_sekunden` | je Gut mit negativer Rate: wie lange der Vorrat bei gleicher Rate reicht |
| `energie{erzeugung,verbrauch,je_solarsonde}` | Strombilanz; `je_solarsonde` = Energie eines Solarsatelliten über diesem Planeten |
| `arbeit{verfuegbar,bedarf}`, `fachkraefte{verfuegbar,bedarf,zaehlt}`, `forscher{zugewiesen,bedarf}` | Personal |
| `bevoelkerung`, `wohnraum`, `ueberbelegung`, `wachstum`, `personengruppen` | Einwohner |
| `stabilitaet`, `stabilitaet_ziel` | Stabilität und ihr Zielwert |
| `nahrung_deckung`, `konsum_deckung` | Versorgung in Prozent |
| `felder{belegt,gesamt}` | Bauplätze; jede Gebäudestufe und jeder Auftrag belegt ein Feld |
| `gebaeude`, `gebaeude_integritaet_prozent`, `reparaturkosten` | Stufen, Schäden |
| `bauschleife[]`, `bauschleife_plaetze` | Aufträge (`rest_min`, `wartet`, `pausiert`) |
| `baubar[]` | je Gebäude die nächste Stufe: `kosten`, `bauzeit_sekunden`, `fehlt`, `braucht`, `ertrag`, `strom_plus`, `arbeiter_plus`, `fachkraefte_plus` |
| `fertigung[]` | Werft (`schleife: werft`) und Orbitalwerft (`orbitalwerft`) mit `produkt`, `rest`, `wartet`, `ruht_grund` |
| `prioritaeten` | Reihenfolge, in der Gebäude Arbeitskräfte bekommen |
| `produktionsserien`, `industrie` | Vorratsziele, Werksbilanz |
| `mond` | Mond an dieser Koordinate (Kartensicht); voll unter `lage.monde.eigene[]` |

Reichsweit: `lage.toepfe`, `lage.credits`, `lage.steuersatz`, `lage.forschung`, `lage.naechste_stufe`, `lage.kolonien`, `lage.kolonie_weg`, `lage.einheiten_kosten`, `lage.markt`.

## 2 Energie

- Erzeugung: Solarkraftwerk (Ertrag × Sonnenfaktor des Platzes, keine eigene Stromlast), Fusionskraftwerk (ab Stufe II, verbraucht Deuterium und Fachkräfte), Solarsatelliten (`solarsonde`, ab Stufe II, Werft 1, kein Feld, kein Personal, wehrlos), auf Monden die Mondbasis.
- Sonnenfaktor je Platz (`kernwerte.ron`, `sonne.solar_promille`): Platz 1 120 %, Platz 6 100 %, Platz 12 80 %. Innere Planeten lohnen Solar, äußere Fusion.
- Verbrauch: Minen, Werke, Labor, Werft, Akademie und mehr (Feld `energie` je Gebäude in `online-v1.ron`); Syntheten brauchen zusätzlich Energie für die Bevölkerung (`energie_je_1000_syntheten` 25 je Stunde).
- Regel: Vor einem Verbraucher prüfen, ob `energie.erzeugung − energie.verbrauch ≥ strom_plus` der geplanten Stufe ist. Sonst zuerst Kraftwerk oder Satelliten.
- Fusion: `werkzeug kosten {"gebaeude":"fusionskraftwerk","stufe":n,"planet":K}` nennt `deuteriumbedarf_je_stunde_vollbesetzt`. Fusion frisst Flottentreibstoff; Deuteriumrate danach prüfen.
- Strommangel: Werften ruhen ganz („Energie fehlt (Deckung N %)“), andere Werke leisten anteilig weniger. Bauaufträge laufen weiter – ein Planet kann sich immer aus dem Mangel herausbauen (Solarkraftwerk zuerst).

## 3 Nahrung, Konsum, Bevölkerung

- 1.000 Einwohner verbrauchen 40 Nahrung je Stunde (`nahrung_je_1000`) und ab Zivilisationsstufe III 1,5 Konsumgüter (`konsum_je_1000`); davor keine Konsumgüter. Syntheten essen nicht (`ohne_nahrung`).
- Nahrung kommt von der Farm (Ertrag × Nahrungsfaktor der Zone und des Volkes), Agrarwissenschaft +8 % je Stufe.
- Konsumgüterwerk (ab Stufe II) verarbeitet je Konsumgut 1 Erz, 1 Kristall, 0,5 Deuterium. Konsumdeckung hebt die Stabilität (`konsum_bonus` 20) und ist Bedingung für Stufe IV (80 %).
- Wachstum je Tag: `wachstum_je_tag` 0,7 × Volksfaktor × Klimafaktor des Platzes (`kernwerte.ron`, `klima.wachstum`) × (1 − Bevölkerung/Wohnraum) × Versorgung × Stabilitätsfaktor (voll ab 50, null unter 30). Vor Stufe III zählt das Wachstum doppelt (Frühphase).
- Wohnraum: Grundwohnraum 2.000 plus Wohnblock (`ertrag` 5.600 je Stufe, wachsend). Ist der Wohnraum voll, bringt Nahrung kein Wachstum. Mindestens 20 % freier Wohnraum gibt den vollen Wohnraumbonus auf die Stabilität.
- Hunger (Deckung < 100 %): Wachstum wird negativ (bis −0,5), Warnung „Versorgung auf K nur zu N Prozent gedeckt“. Sofort Farm bauen oder Nahrung transportieren.
- Klima: `werkzeug habitabilitaet {"planet":K}` zeigt Klimafaktor und eine nötige Terraformingstufe. Auf gesperrten Plätzen wächst das Volk nicht und verliert bis zu 2 % je Stunde.

## 4 Arbeitskräfte, Fachkräfte, Forscher

- 60 % der Bevölkerung arbeiten (`arbeitsquote`). Jedes Gebäude braucht Arbeiter (`arbeiter_plus` in `baubar`), anspruchsvolle zusätzlich Fachkräfte (Fusion, Elektronikwerk, Xenoextraktor, Orbitalwerft, Verwaltungszentrum, Nanofabrik, Sensorphalanx, Geheimdienst), das Labor Forscher.
- Fachkräfte zählen erst ab Zivilisationsstufe III (`fachkraefte.zaehlt`, `ab_stufe`).
- Grundausbildung (`planeten[].grundausbildung`, `akademie`): Akademien bilden kostenlos bis zum Bedarf aus; die Heimatwelt auch ohne Akademie wie Akademie 1; Kolonien brauchen eine eigene Akademie.
- Bezahlte Ausbildung beschleunigt: `{"typ":"ausbilden","planet":K,"volk":"krath","qualifikation":1,"anzahl":20}` (1 = Fachkraft, 2 = Forscher; höchstens 20 je Akademiestufe gleichzeitig; Fachkraft 10 Konsumgüter, Forscher 10 Konsumgüter und 10 Elektronik; vor Stufe III kostenlos; Forscher brauchen ein Labor).
- Fehlt Personal: `prioritaeten {"planet":K,"reihenfolge":["farm","solarkraftwerk","erzmine",…]}` legt fest, wer zuerst Arbeiter bekommt. Automatisierung senkt den Arbeiterbedarf (−4 % je Stufe, höchstens −50 %).
- Werft steht still mit „Besetzung N % – Arbeiter a von b, Fachkräfte c von d“: Personal schaffen oder Produktionsgrad anderer Werke senken.

## 5 Stabilität und Steuern

- Ziel 50 plus Boni (Konsum 20, Wohnraum 10, Soziologie 2 je Stufe) minus Mali (Steuer über 15 % je Punkt 1, Kolonien über der Verwaltungsgrenze je 5, Plünderung 5 für 5 Tage, höchstens 10). Annäherung mit etwa 24 Spielstunden Halbwertszeit.
- Unter 30: Unruhen. Neue Bauaufträge ruhen, laufende werden unterbrochen – außer Verwaltungszentrum, Konsumgüterwerk und Wohnblock.
- `steuersatz {"prozent":n}` gilt reichsweit, 0–50 (Start 10). Steuern bringen Credits (Unterhalt, Verträge, Handel), keine Topfgüter. Über 15 % kostet jeder Punkt Stabilität auf allen Planeten.
- Verwaltungsgrenze (`lage.kolonien.verwaltungsgrenze`): je 2 Stufen Verwaltungszentrum (ab Stufe IV) eine Kolonie ohne Stabilitätsverlust.
- Stufe III verlangt Heimatstabilität 55, Stufe IV 60.

## 6 Lager und Bunker

- Lagergrenze je Gut: Rohstoffe 8.000, Verarbeitetes 1.500, Seltenes 400, jeweils × 1,6 je Lagerstufe (`lager_*`, `lager_faktor`). Was darüber gefördert würde, verfällt und füllt keine Töpfe.
- Verarbeitungswerke ruhen, solange ihr Erzeugnis voll ist (verbrauchen dann auch nichts).
- Warnung „Lager für G auf K voll in unter 6 Stunden“: Lager ausbauen, verbauen, abtransportieren oder verkaufen.
- Bunker (ab Stufe II) schützt je Stufe 20.000 je Rohstoff und 20 % davon bei verarbeiteten Gütern vor Plünderung (`bunkerschutz`).
- Lieferungen, Beute und Marktkäufe kommen auch über die Grenze an.

## 7 Bauketten und Voraussetzungen

Freischaltung nach Zivilisationsstufe (`ab_stufe` in `online-v1.ron`), Gebäudevoraussetzungen (`braucht`):

| Ab Stufe | Gebäude |
|---|---|
| I | Erzmine, Kristallmine, Deuteriumsynthesizer, Farm, Solarkraftwerk, Lager, Wohnblock, Akademie, Labor, Bauhof, Raumhafen, Werft (braucht Raumhafen 1) |
| II | Fusionskraftwerk, Gießerei, Elektronikwerk, Konsumgüterwerk, Bunker, Geheimdienst (braucht Labor 1) |
| III | Markt, Sensorphalanx, Raketensilo, Kaserne (`kernwerte.ron`, `industrie.kaserne_ab_stufe`) |
| IV | Xenoextraktor, Verwaltungszentrum, Nanofabrik (braucht Bauhof 6), Orbitalwerft (braucht Werft 4) |
| V | Orbitalring, Forschungsarchiv, Versorgungsnetz (je Planet einmal) |

Wichtige Ketten:

- **Schiffe:** Raumhafen 1 → Werft n. Jeder Schiffstyp braucht eine Werftstufe (`werft` in `regeln/flottenausbau.json`) und oft Forschung; `lage.einheiten_kosten[]` nennt je Typ `werft`, `braucht`, Kosten und Bauzeit je Planet. Flottenstarts brauchen einen funktionsfähigen Raumhafen.
- **Verarbeitung:** Gießerei (4 Erz → 1 Legierung), Elektronikwerk (2 Kristall + 1 Erz → 1 Elektronik, braucht Fachkräfte), Konsumgüterwerk.
- **Bauteile:** Antriebskern und Habitatmodul fertigt die Orbitalwerft (`fertigen` mit `bauteil`). Ohne Orbitalwerft baut die Werft ab Stufe III und Werft 3 einen vorhandenen Recycler zu einem Habitatmodul um. Jedes neue Reich startet mit 2 Habitatmodulen und 2 Antriebskernen.
- **Forschung:** jedes Projekt braucht ein Labor der genannten Stufe auf dem Forschungsplaneten (Astrophysik 3, Terraforming 3, Hyperraumantrieb 6, Xenomaterialkunde 5, Plasmatechnik 4 laut `technik.ron`). Plasmatechnik braucht Energietechnik 5, Überwachungs- und Abschirmtechnik Spionagetechnik 1, Überwachungstechnik dazu einen eigenen Geheimdienst.
- Bauzeit: (Erz + Kristall + 2 × Legierung) × 3.600 / 2.500 Sekunden, verkürzt durch Bauhof und Nanofabrik (halbiert je Stufe), mindestens 60 s. Genau: `baubar[].bauzeit_sekunden` oder `werkzeug kosten`.
- Abriss (`abreissen`) senkt eine Stufe ohne Erstattung; gesperrt, wenn das Gebäude in der Schleife steht, ein wartender Auftrag es auf dieser Stufe braucht oder Fertigungsplätze belegt sind.
- Beschädigte Gebäude erst `reparieren` (Bauseite zeigt `reparaturkosten`), sonst „Beschädigtes Gebäude vor weiterem Ausbau reparieren“.

## 8 Produktionsgrad

`{"typ":"produktion_setzen","planet":K,"gebaeude":"giesserei","prozent":60}` – 0 bis 120 in 10er-Schritten für Minen, Synthesizer, Farm, Solar- und Fusionskraftwerk, Gießerei, Elektronik-, Konsumgüterwerk, Xenoextraktor, Werft, Orbitalwerft, Kaserne, Raketensilo, Labor, Akademie.

- Über 100 % kostet doppelt: 120 % Ausstoß braucht 140 % Personal und Energie.
- 0 % legt das Werk still (spart Personal und Strom).
- Werft und Orbitalwerft: der Grad verändert die Fertigungszeit laufender Aufträge.
- Einsatz: Fusion drosseln, wenn Deuterium knapp ist; Gießerei drosseln, wenn Erz für Bauten fehlt; Labor hochfahren in Forschungsphasen.

## 9 Warteschlangen

- **Bau:** `bauen` reiht ein (höchstens 5 je Planet einschließlich des laufenden). Gezahlt wird beim Start; vorher zeigt `forschung.warteschlangen[]` den Grund („Wartet: 1200 kristall fehlen · Budget wirtschaft: … fehlen“). `schleife_leeren {"planet":K}` entfernt alle wartenden Aufträge.
- **Forschung:** eine aktive plus 3 wartende (`forschung_warteschlange`). `forschen {"forschung":"astrophysik","planet":K}` – ohne `planet` die Heimatwelt; dort müssen Laborstufe und beim Start die Güter liegen. Forschungspunkte kommen von allen Laboren (`lage.forschung.punkte_je_stunde`).
- **Werft und Bauteile:** `fertigen {"planet":K,"einheit":"kleiner_transporter","anzahl":5}` oder `{"planet":K,"bauteil":"habitatmodul","anzahl":1}` – genau eines von `einheit` und `bauteil`, Anzahl 1–10.000. Bezahlt sofort; Besatzung kommt aus der Bevölkerung (mindestens 500 bleiben). Der Verwalter fertigt nur zivile Einheiten (Transporter, Frachter, Bergbauschiff, Recycler, Kolonieschiffe, Solarsatellit, Terraformer), Kriegsschiffe und Verteidigung der Feldherr.
- **Umordnen, löschen:** `warteschlange_aendern` braucht die aktuelle `kennung` aus `lage.forschung.warteschlangen[]` (Eintrag mit passender `art` und `planet`):

```json
{"typ":"warteschlange_aendern","planet":"1:27:6","art":"bau","index":3,"ziel":1,"loeschen":false,"kennung":"<64 Hexzeichen>"}
```

  `index` und `ziel` zählen ab 0 in `jobs[]`. Der laufende Auftrag bleibt vorn. Löschen erstattet vor 75 % Fertigstellung die bezahlten Güter samt Topf (`stornoregel`, `erstattung_prozent`). Hat sich die Schlange geändert: „Die Warteschlange hat sich inzwischen geändert …“ – Lage neu lesen, neue Kennung.
- **Pause:** `auftrag_pause {"planet":K,"art":"bau|forschung|werft|bauteile","pausiert":true}` hält einen Auftrag an; Fortschritt und bezahlte Güter bleiben.
- **Produktionsserie (Vorratsziel):** `{"typ":"produktionsserie","planet":K,"art":"einheit","produkt":"solarsonde","ziel":30}` füllt den lokalen Bestand automatisch bis zum Ziel nach (bezahlt jedes Los normal, mit dem Topf der Rolle, die die Serie gesetzt hat). `ziel: 0` schaltet ab. Arten `einheit`, `bauteil`; `rakete` und `boden` setzt der Feldherr.

## 10 Zivilisationsstufen

- `lage.naechste_stufe.bedingungen[]` nennt jede Bedingung mit `erfuellt`; `hinweise` erklärt Verstecktes (fehlende Felder, Labor zu klein, Unruhen).
- Sind alle erfüllt, sofort `stufenaufstieg {}` (Stratege oder Verwalter). Kosten zahlt die Heimatwelt, kein Topf. Es gibt keine Haltezeit (`stufen_haltezeit_stunden` 0). Die Agenda meldet „Alle Bedingungen für … erfüllt“.
- Stufen laut `online-v1.ron`:

| Ziel | Bedingungen | Kosten |
|---|---|---|
| II Industrie | 1.500 Einwohner, Erzmine 5, Kristallmine 5, Nahrung und Energie auf allen Planeten im Plus | 2.000 Erz, 1.000 Kristall |
| III Orbit | 4.000 Einwohner, Gießerei 3, Elektronikwerk 2, Energietechnik 3, Heimatstabilität 55 | 10.000 Erz, 6.000 Kristall, 500 Legierung, 200 Elektronik |
| IV Sternenflug | 40.000 Einwohner, Werft 4, Impulsantrieb 3, Astrophysik 1, Heimatstabilität 60, Konsumdeckung 80 % | 40.000 Erz, 25.000 Kristall, 10.000 Deuterium, 2.000 Legierung, 1.000 Elektronik |
| V Imperium | 200.000 Einwohner, Hyperraumantrieb 1, 3 Kolonien | 200.000 Erz, 150.000 Kristall, 50.000 Deuterium, 10.000 Legierung, 5.000 Elektronik, 5.000 Xenokristall |

- Einwohner zählen reichsweit, Gebäudestufen auf irgendeinem eigenen Planeten.
- Achtung: Der Aufstieg auf Stufe III beendet den Anfängerschutz (`diplomatie.anfaengerschutz_bis_stufe` 3). Vorher mit dem Feldherrn abstimmen (Pinwand `warnung`).
- Xenokristall für Stufe V: Xenoextraktor (ab IV) auf einem Planeten im Nebelsystem oder per Synthese; Markt.

## 11 Transporte zwischen eigenen Planeten

```json
{"typ":"flotte_senden","start":"1:27:6","ziel":"1:27:9","mission":"transport",
 "schiffe":{"grosser_transporter":4},"ladung":{"erz":40000,"kristall":20000,"deuterium":5000},"geschwindigkeit":1.0}
```

- Vorher `werkzeug flugzeit` mit denselben Schiffen: `ladekapazitaet`, `treibstoff_je_strecke`, `dauer_sekunden`. Transport kostet Treibstoff für Hin- und Rückflug, `stationieren` nur einfach (die Schiffe bleiben am Ziel).
- Der Startplanet muss Ladung **und** Treibstoff im Bestand haben; der Topf (Verwalter: `wirtschaft`) zahlt nur den Treibstoff. Deuterium in der Ladung ist kein Treibstoff.
- Langsamer fliegen (`geschwindigkeit` 0,1–1,0) spart Treibstoff.
- Flottenplätze (`lage.flottenplaetze`) teilen sich alle Rollen; vor einer Transportwelle mit dem Feldherrn absprechen.
- Personen: `personen_senden` (Transport oder Stationieren zwischen eigenen Planeten, Mindestbestand 500 am Start); Beispiel mit `GET aktionen?typ=personen_senden`.
- Mond: `zielart:"mond"` bzw. Versorgungsfähre, siehe `sternenepoche-monde-dyson`.
- Faustregel für 15 Planeten: Jede Kolonie trägt ihre Grundversorgung (Energie, Nahrung, Lager) selbst. Transportiert werden nur Engpassgüter (Legierung, Elektronik, Xenokristall, Aufbaufracht), nicht im Kreis.

## 12 Kolonien gründen

Plätze: `lage.kolonien` (`anzahl`, `erlaubt`, `frei_basis`, `forschung`, `naechste_stufe`, `hoechstens`). Neue Welten: 2 Kolonien frei ab Start, jede Stufe Terraforming (ab III, Labor 3) eine weitere, höchstens 14 (15 Planeten mit der Heimatwelt; `kernwerte.ron`, `kolonien`).

Der Weg steht in `lage.kolonie_weg[]` (`text`, `erledigt`) – den ersten offenen Schritt angehen:

1. Zivilisationsstufe und freier Kolonieplatz.
2. Raumhafen auf dem Startplaneten.
3. Kolonieschiff: neue Welten Stufe III, Werft 3, Impulsantrieb 2, ein Habitatmodul, keine Orbitalwerft (`kernwerte.ron`, `industrie`); großes Kolonieschiff ab Stufe IV mit Werft 6, Orbitalwerft 2, Astrophysik 3, Logistik 3, Impulsantrieb 4 (`flottenausbau.json`), vier Habitatmodulen und zwei Antriebskernen. Kosten mit `werkzeug kosten {"einheit":"kolonieschiff"}`.
4. Freier Platz, selbst erkundet (Systemsonde, dann Spionage auf den Platz) – sonst „Ziel zuerst erkunden“.
5. Siedler und Fracht: `werkzeug koloniebedarf` nennt `siedler` und `fracht`; organische Siedler brauchen Nahrung für 24 Spielstunden in der Ladung, mindestens 1.000 Einwohner bleiben zurück.
6. Eskorte: mindestens ein bewaffnetes Schiff am selben Startplaneten (Karte an den Feldherrn).
7. Prüfen: `werkzeug kolonieplan {"start":…,"ziel":…,"schiffe":{…},"ladung":{…},"aufbau":["solarkraftwerk","farm","erzmine","lager"]}` simuliert Start, Ankunft und Aufbau; `start_moeglich`, `grund`, `aufbau_vollstaendig`.
8. Start: `flotte_senden` mit `mission:"kolonisieren"`, optional `siedler_volk`.

Nach der Gründung: Die Fracht ist ein Tagesvorrat. Sofort Solarkraftwerk, Farm (organische Völker), Minen, Lager einreihen; Akademie für die Grundausbildung; nicht über die Verwaltungsgrenze hinaus gründen, ohne Verwaltungszentrum (Stabilität). Jede Kolonie kostet 300 Credits je Tag.

Aufgeben (nur Stratege, unumkehrbar): erst `werkzeug planet_aufgeben {"planet":K}`, dann `planet_aufgeben {"planet":K,"bestaetigung":K}`.

## 13 Monde aus Sicht der Wirtschaft

- Mondgebäude (`mond_bauen`, Verwalter) bezahlt der **Mondbestand**, nicht der Planet: Güter vorher per Transport mit `zielart:"mond"` oder per Fähre (`mond_faehre`) hinbringen.
- Fähre: `{"typ":"mond_faehre","mond":"1:27:6","gut":"deuterium","menge_je_stunde":2000}` – positiv zum Mond, negativ zum Planeten, 0 beendet; zusammen höchstens 2.000 je Stunde und Mondbasisstufe; keine Bauteile.
- Solarsatelliten auf dem Mond fertigt der Verwalter (`mond_fertigen`, Einheit `solarsonde`); alles andere im Werftdock der Feldherr.
- Mondunterhalt kostet täglich Credits (150 + 20 je Gebäudestufe). Mehr: `sternenepoche-monde-dyson`.

## 14 Werkzeuge der Wirtschaft

- `wirtschaftsdiagnose {"planet":K}`: Werksbilanz mit Engpässen (Arbeiter, Fachkräfte, Integrität, Eingänge), Energie- und Versorgungsdeckung. Erste Wahl, wenn eine Rate nicht stimmt.
- `kostenplan {"planet":K,"positionen":[{"art":"gebaeude","gebaeude":"erzmine","von":10,"bis":14}]}`: Summe und `fehlmenge` am Planeten.
- `ablaufplan {"plan":{"schritte":[…Aktionen…],"horizont_stunden":48}}`: simuliert die Schrittfolge mit echten Kerntakten; mit `revision` und `plan_uebernehmen` atomar ausführen.
- `kosten {"forschung":"astrophysik"}`, `kosten {"einheit":"grosser_transporter","planet":K}`.
- `soforthandel` (Werkzeug als Vorschau, Aktion als Tausch) gegen Erz, Kristall, Deuterium, Nahrung oder Credits ohne Markt, 2 % Gebühr.
- `regel {"stichwort":"lager"}` für Regeltexte.

## 15 Arbeitsroutine für viele Planeten

Je Zug (Verwalter-Takt 4 Spielstunden, Agenda weckt früher):

1. **Agenda:** Priorität 1 (Warnungen zu Energie, Versorgung, Unruhen, Unterhalt, Blockade; Stufenaufstieg) sofort; `bauschleife_leer` und `forschung_ruht` danach.
2. **Engpass zuerst:** je Warnung den Planeten mit `planet=K` öffnen, Ursache mit `wirtschaftsdiagnose` bestätigen, eine Aktion dagegen.
3. **Rotation:** neben den Warnplaneten 2–3 weitere Planeten im Detail, reihum (im Notizbuch merken, welche zuletzt dran waren). Die übrigen nur über `kompakt`.
4. **Schleifen füllen:** je Planet so viele Aufträge einreihen, dass die Schleife bis zum nächsten Zug nicht leerläuft, aber nur bezahlbare an erster Stelle (`fehlt` leer).
5. **Forschung:** Schlange mit 3 wartenden Projekten gefüllt halten; Ziel aus der Doktrin.
6. **Fertigung:** Transporter, Satelliten, Bauteile für die nächsten Schritte (Kolonie, Stufe).
7. **Bündeln:** bis 8 Aktionen je Zug, zuerst `probe`; danach Karten quittieren, Notizbuch.

Bauprioritäten in dieser Reihenfolge:

1. Energie ≥ Verbrauch auf jedem Planeten.
2. Versorgung ≥ 100 % (Farm; Syntheten Energie).
3. Lager nicht voll laufen lassen; Bunker auf Planeten mit großem Vorrat in Reichweite von Gegnern.
4. Stabilität über 50 (Konsumgüter, Wohnraum, Steuersatz moderat).
5. Minen und Synthesizer nach Bedarf der nächsten Stufe (`naechste_stufe`).
6. Verarbeitung (Gießerei, Elektronik) für Stufe III/IV, Schiffe und Bauteile.
7. Labor, Akademie, Werft, Orbitalwerft, Verwaltungszentrum nach Doktrin.

Was nicht in die eigene Rolle fällt (Kriegsschiffe, Verteidigung, Eskorten, Verträge, Topfgeld), geht als Karte an die zuständige Rolle (`sternenepoche-rat`).
