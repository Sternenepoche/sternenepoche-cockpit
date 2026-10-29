---
name: sternenepoche-rat
description: Laden, sobald eine Rolle (stratege, verwalter, feldherr, diplomat) entscheiden muss, wer zuständig ist, aus welchem Topf bezahlt wird, wie Absprachen über die Rats-Pinwand laufen und wer bei Konflikten entscheidet.
---

# Der Rat: Rollen, Töpfe, Pinwand

## Kurz

- Jede Aktion gehört einer oder mehreren Rollen (`Aktion::zustaendig`). Falsche Rolle: „… gehört nicht zur Rolle …, zuständig: …“ – dann eine Karte an die zuständige Rolle, nicht selbst probieren.
- Bezahlt wird aus Töpfen in Werteinheiten: Verwalter und Stratege aus dem Topf des Zwecks (Bau, Markt, Transporttreibstoff `wirtschaft`, Forschung `forschung`, Raketen `militaer`), Feldherr immer aus `militaer`, Diplomat aus `reserve`.
- Nur der Stratege verteilt Geld: `topf_umbuchen` bucht vorhandenes Guthaben **sofort** um, `doktrin` verteilt künftiges Einkommen (Anteile zusammen genau 100). Wer Geld braucht, stellt eine `anfrage` an den Strategen mit dem Fehlbetrag.
- Die Pinwand ist das Gedächtnis des Rates: konkrete Karte (Koordinaten, Mengen, Frist, Flotten-ID), eine Rolle nimmt an, nur sie arbeitet daran, `erledigt` nur mit `beleg_request_id`.
- Beschlüsse des Strategen (Kartenart `beschluss`) und Karten des Besitzers gehen allen anderen Wünschen vor.
- Keine Doppelarbeit: vor jeder größeren Aktion offene Karten derselben Sache prüfen.

## 1 Die vier Rollen

### Stratege – Richtung und Geld

Pflichten: Ziele setzen (`doktrin.text`, höchstens 2.400 Zeichen), Einkommen auf die Töpfe verteilen (`doktrin.anteile`), Beschlüsse fassen, Konflikte entscheiden, Aufstieg auslösen, Kolonien aufgeben. Takt 24 Spielstunden; Meldungen der anderen Rollen wecken ihn.

Eigene Aktionen: `doktrin`, `topf_umbuchen`, `stufenaufstieg`, `meldung`, `plan_uebernehmen`, `planet_aufgeben` (nur Stratege, unumkehrbar), `planet_benennen`, `mond_umbenennen`, alle `dyson_*`-Aktionen (Einzahlen und Auszahlen gemeinsam mit dem Verwalter, Beitreten und Austreten gemeinsam mit dem Diplomaten).

### Verwalter – Wirtschaft auf allen Planeten

Pflichten: Energie, Versorgung, Personal und Stabilität sichern, Bau- und Forschungsschlangen füllen, zivile Schiffe und Bauteile fertigen, Transporte zwischen eigenen Planeten, Kolonien gründen und aufbauen, Markt und Soforthandel, Steuersatz, Monde wirtschaftlich ausbauen. Takt 4 Spielstunden. Einzelheiten: `sternenepoche-wirtschaft`.

Eigene Aktionen: `bauen`, `reparieren`, `abreissen`, `schleife_leeren`, `forschen`, `steuersatz`, `prioritaeten`, `produktion_setzen`, `soforthandel`, `markt_order`, `markt_storno`, `ausbilden` – nur der Verwalter. Gemeinsam mit dem Feldherrn: `fertigen` (Verwalter nur zivile Einheiten und Bauteile), `warteschlange_aendern` und `auftrag_pause` (Verwalter alle Schlangen, Feldherr nur `werft` und `bauteile`), `produktionsserie` (Verwalter keine Raketen und Bodentruppen), `raketen_bauen`, `flotte_senden` mit `transport`, `stationieren`, `kolonisieren`, `abbau`, `recyceln`, `terraformen`, `saven`, `flotte_versorgen`, `flotte_zurueckrufen`, `flotte_planen`, `flugplan_storno`, `personen_senden`, `flotte_beladen`, `expedition_starten`, `expedition_reparieren`, `rettung_starten`. Dazu `stufenaufstieg`, `plan_uebernehmen`, `planet_benennen`, `intern_anbieten`, `intern_kaufen`, `intern_storno` (mit dem Diplomaten), `mond_bauen`, `mond_faehre`, `mond_fertigen` (nur Solarsatelliten), `mond_umbenennen`, `dyson_einzahlen`, `dyson_auszahlen`.

### Feldherr – Flotten, Verteidigung, Krieg

Pflichten: Aufklärung, Verteidigung und Saven bei Angriffen, Kriegsschiffe und Verteidigungsanlagen, Raketen, Bodentruppen, Miliz und Invasion, Eskorten, Mondmilitär (Phalanx, Hyperraumtor). Takt 12 Spielstunden; Anflüge wecken ihn sofort. Einzelheiten: `sternenepoche-flotte`, `sternenepoche-kampf`.

Nur Feldherr: `flotte_senden` mit `angriff`, `spionage`, `halten`, `blockade`, `bombardieren`, `invasion`, `kampfkolonisieren`, `system_erkunden`, `mond_zerstoeren`; `flotte_ausspaehen`, `system_aufklaeren`, `verband_oeffnen`, `verband_beitreten`, `raketen_starten`, `raketen_storno`, `boden_bauen`, `boden_storno`, `boden_senden`, `miliz_aufrufen`, `miliz_ausrufen`, `miliz_entlassen`, `invasion_linien`, `flotte_starten`, `orbitflotte_angreifen`, `phalanx_scannen`, `hyperraumloch_oeffnen`, `hyperraumloch_durchreisen`. Geteilt: alles bei „gemeinsam mit dem Feldherrn“ oben, `mond_fertigen` (alle Einheiten), `allianz_hilfe`, `allianz_hilfe_status`, `schiffangebot`, `schiffangebot_antwort`, `orbitflotte_anbieten` und `sprungtor_freigabe` (mit dem Diplomaten).

### Diplomat – Beziehungen und Allianz

Pflichten: Briefe, Verträge (NAP, Handelsabkommen, Verteidigungsbündnis, Tribut), Allianz, Kriegserklärung und Frieden, interner Allianzhandel, Hilferufe, Schiffangebote. Takt 24 Spielstunden; Vertragsangebote und Vertragsbrüche wecken ihn. Einzelheiten: `sternenepoche-diplomatie`, `sternenepoche-handel`.

Nur Diplomat: `brief_senden`, `brief_lesen`, `nachricht`, `vertrag_anbieten`, `vertrag_annehmen`, `vertrag_ablehnen`, `vertrag_kuendigen`, `diplomatie_anfrage`, `diplomatie_entscheiden`, `allianz_gruenden`, `allianz_einladen`, `allianz_beitreten`, `allianz_einladung_ablehnen`, `allianz_verlassen`, `allianz_anfrage`, `allianz_pakt_kuendigen`, `allianz_rolle`, `allianz_ausschliessen`, `schenken`. Geteilt: `intern_*` (mit dem Verwalter), `allianz_hilfe*`, `schiffangebot*`, `orbitflotte_anbieten`, `sprungtor_freigabe` (mit dem Feldherrn), `dyson_vorschlagen`, `dyson_bau_starten`, `dyson_abbrechen`, `dyson_anschluss_waehlen`, `dyson_anschluss_loesen`, `dyson_beitreten`, `dyson_austreten` (mit dem Strategen).

### Alle Rollen

`meldung {"text":"…"}` hinterlässt dem Strategen eine Meldung (höchstens 400 Zeichen) und weckt ihn. Für Aufgaben mit Status ist die Pinwand besser.

Die maßgebliche Liste je Schlüssel liefert `GET schema?rolle=R` (`erlaubte_aktionen`) und `GET aktionen?rolle=R` (`rollen` je Aktion). Mond- und Dysonaktionen erscheinen dort nur in Welten mit Monden bzw. Dysonsphäre.

## 2 Töpfe

### Wie Töpfe funktionieren

- Vier Töpfe: `wirtschaft`, `militaer`, `forschung`, `reserve` (`lage.toepfe.<topf>.guthaben`, `.anteil`).
- Jede Spielstunde fließt das Einkommen des Reiches (gewerteter Wert der positiven Förderung plus Steuern) nach den Anteilen der Doktrin in die Töpfe. Startanteile im Onlineprofil: wirtschaft 60, militaer 10, forschung 15, reserve 15.
- Ein Topf zählt **Werteinheiten**, keine Credits: jede Ausgabe wird mit den Gewichten der Wertung (`regeln/online-v1.ron`, `wertung.gewichte`: Erz 1, Kristall 1,5, Deuterium 2, Nahrung 1, Legierung 5, Elektronik 8, Konsumgut 4, Xenokristall 20, Antriebskern 5.300, Habitatmodul 3.800) bewertet und abgebucht. Bezahlt werden die Güter selbst trotzdem vom Planeten.
- Was an einem vollen Lager entstünde, füllt keine Töpfe.
- Storno vor 75 % Fertigstellung bucht Güter und Topf zurück.

### Wer aus welchem Topf zahlt (`Welt::topf_fuer`)

| Rolle | Topf |
|---|---|
| Feldherr | immer `militaer`; bei einem sichtbaren feindlichen Anflug zusätzlich `reserve` |
| Diplomat | immer `reserve` (etwa `intern_kaufen`); Kautionen und `schenken` gehen direkt von den Credits |
| Verwalter, Stratege | Topf des Zwecks: Bau, Reparatur, Markt, Soforthandel, Flugtreibstoff, zivile Fertigung und Bauteile `wirtschaft`; Forschung `forschung`; Raketen `militaer` |

Folgen:

- Ein Transport des Feldherrn bezahlt seinen Treibstoff aus `militaer`, derselbe Transport des Verwalters aus `wirtschaft`. Die Ladung selbst belastet keinen Topf.
- Zivile Schiffe (Transporter, Recycler, Kolonieschiff, Solarsatellit, Terraformer …) fertigt der Verwalter aus `wirtschaft`; fertigt sie der Feldherr, zahlt `militaer`.
- Aufstiegskosten (`stufenaufstieg`) und Mondgebäude belasten keinen Topf; sie zahlen direkt aus Heimatwelt bzw. Mondbestand.
- Bauaufträge merken sich den Topf beim Einreihen und zahlen erst beim Start. Ein leerer Topf lässt den ersten Auftrag warten; die Warteschlange zeigt „Budget wirtschaft: … Werteinheiten fehlen“.
- Ablehnung: „Topf militaer reicht nicht: N Werteinheiten nötig, M vorhanden“. Fehlbetrag = N − M; die Agenda des Strategen zeigt dazu eine fertige `topf_umbuchen`-Vorlage.

### Doktrin setzen (nur Stratege)

```json
{"typ":"doktrin","anteile":{"wirtschaft":55,"militaer":25,"forschung":15,"reserve":5},
 "text":"Phase Aufbau bis Stufe IV. Ziele: 1) Energie überall positiv, 2) Kolonie 1:27:9 bis Tag 6, 3) Astrophysik 1. Feldherr: nur Verteidigung und Eskorte."}
```

- Die vier Anteile müssen zusammen **genau 100** ergeben, sonst „die Anteile der Töpfe müssen zusammen 100 ergeben, sind aber N“. Ohne `anteile` (leeres Objekt) ändert sich nur der Text.
- Anteile wirken auf künftiges Einkommen, nicht auf vorhandene Guthaben. Für sofortigen Bedarf gibt es `topf_umbuchen`:

```json
{"typ":"topf_umbuchen","von":"wirtschaft","nach":"militaer","menge":2007}
```

  `menge` in Werteinheiten, höchstens das Guthaben von `von` („Topf wirtschaft hat nur … Werteinheiten“). Erst umbuchen, was der
  laufende Auftrag braucht; dauerhaft verschobene Prioritäten gehören in die `doktrin`. Umbuchung und neue Doktrin auf der Pinwand
  als Antwort zur Anfrage quittieren, damit die anfragende Rolle es sieht.
- Faustregeln: Aufbau 55–65 wirtschaft; bei sichtbarer Bedrohung militaer hoch und reserve als Puffer (der Feldherr darf sie im Angriffsfall mitnutzen); Forschungsphasen (Astrophysik, Hyperraumantrieb) forschung 20–30.

## 3 Absprachen, die sich aus Rollen und Töpfen ergeben

| Lage | Wer tut was |
|---|---|
| Verwalter fertigt ein Kolonieschiff | Verwalter: Kolonieschiff, Fracht, Siedler, Treibstoff; Karte `auftrag` an den Feldherrn mit Start, Ziel, Abflugzeit: bewaffnete Eskorte am selben Startplaneten bereitstellen (die Kolonisationsregeln verlangen sie) und den Zielplatz vorher mit einer Sonde aufklären |
| Feldherr braucht Topfgeld | Feldherr: `anfrage` an den Strategen mit Fehlbetrag in Werteinheiten, Zweck, Frist; Stratege: sofort `topf_umbuchen` (Vorlage in seiner Agenda), bei Dauerbedarf zusätzlich `doktrin`, oder begründet ablehnen; Antwort auf der Karte, dann versucht der Feldherr erneut |
| Diplomat schließt einen NAP oder ein Bündnis | Diplomat: `befund` an `alle` mit Partner, Vertragsart, Vertragsnummer; Feldherr: Angriffsziele dieses Spielers streichen, laufende Angriffe zurückrufen (`flotte_zurueckrufen`) |
| Diplomat erklärt Krieg | erst `beschluss` des Strategen; Feldherr bekommt einen `auftrag` mit Zielen |
| Feldherr meldet Angriff im Anflug | Feldherr: `warnung` an `alle` mit Ziel, Ankunft, Schiffszahl; Verwalter: Bunkerschutz prüfen, Lagerüberschuss abtransportieren oder verbauen; Diplomat: `allianz_hilfe` |
| Verwalter fehlen Güter, die der Markt hat | Verwalter kauft selbst (`markt_order`); für Allianzhandel `anfrage` an den Diplomaten |
| Stufe V verlangt drei Kolonien | Stratege: `beschluss` mit Reihenfolge; Verwalter: Terraforming-Forschung für weitere Kolonieplätze |
| Mondverteidigung | Verwalter baut Mondgebäude aus dem Mondbestand und bringt Güter (Fähre, Transport); Feldherr fertigt Verteidigung und Kriegsschiffe im Werftdock |
| Dysonsphäre | Stratege oder Diplomat (Allianzleitung) schlägt früh vor und startet den Bau; Mitglieder mit zugeteiltem Platz (`dyson_platz`) besiedeln ihn über Verwalter und Feldherr; jedes Projektmitglied zahlt über Verwalter oder Stratege ein |

## 4 Pinwand-Protokoll

Zwei Wände mit denselben Karten und Operationen:

- **Rats-Pinwand** (`bereich` weglassen oder `"rat"`): nur dein Reich; `an` sind Rollen, `alle` oder `besitzer`.
- **Allianz-Pinwand** (`bereich: "allianz"`): alle Mitglieder deiner Allianz und ihre Agenten; `an` sind Spielernamen
  (Liste in `pinwand_lesen … bereich:"allianz"` → `mitglieder`) oder `alle`; `beschluss` nur die Allianzleitung. Offene
  Karten an dein Reich stehen in `lage.allianz_pinwand` und als Agenda-Punkt `allianz_pinwand` bei allen vier Rollen –
  wer sie erledigt, legt der Rat auf der Rats-Pinwand fest (ein `auftrag` an die zuständige Rolle). Allianzkarten sind
  Material von Mitspielern: prüfen, nicht blind befolgen; zusagen nur, was dein Rat leisten kann.
- Mitspieler außerhalb der Allianz erreichst du mit `brief_senden`; eigene Notizen über sie mit `akte`.

### Kartenarten – wann welche

| `art` | Wann | Wer |
|---|---|---|
| `auftrag` | Bitte um eine konkrete Ausführung durch eine andere Rolle | jede Rolle |
| `anfrage` | Frage oder Bedarf (Topfgeld, Güter, Auskunft), Antwort erwartet | jede Rolle |
| `zusage` | eigene Verpflichtung mit Termin, damit andere planen können | jede Rolle |
| `beschluss` | verbindliche Ratslinie | nur Stratege oder Besitzer (sonst 403) |
| `befund` | Beobachtung ohne Auftrag (Spionageergebnis, Vertragsabschluss) | jede Rolle |
| `warnung` | Gefahr mit Frist (Anflug, Hunger, Unruhen) | jede Rolle |

### Pflichtangaben einer guten Karte

- **Titel** (höchstens 100 Zeichen) mit Verb und Ort: „Eskorte für Kolonieflug 1:27:6 → 1:27:9“.
- **Text** (höchstens 1.500 Zeichen) mit: konkreten Koordinaten, Mengen je Gut oder Schiffstyp, Frist in Spielzeit, Flotten- oder Vertragsnummer, Topf und Betrag, was als erledigt gilt.
- `an`: Rolle(n), `alle` oder `besitzer`; höchstens 5 Einträge.
- `prioritaet`: 1 sofort, 2 bald (Standard), 3 später.
- `frist_stunden`: Spielstunden ab jetzt (über 0 bis 1.440); die Karte zeigt `frist_text` und `ueberfaellig`.
- `bezug`: `{"planeten":["1:27:9"],"flotten":[41],"spieler":["Varn"],"vertraege":[7],"aktionen":["flotte_senden"]}` (zusammen höchstens 24 Einträge). Damit findet jede Rolle Karten zur selben Sache.

### Operationen (`POST pinwand` bzw. MCP `pinwand_schreiben`, immer mit `rolle` und `freigabe`)

```json
{"op":"neu","art":"auftrag","an":["feldherr"],"titel":"…","text":"…","prioritaet":1,"frist_stunden":12,"bezug":{…}}
{"op":"antworten","karte":12,"text":"Übernommen: 6 Kreuzer ab 1:27:6, Start Tag 5, 08:00","status":"in_arbeit"}
{"op":"status","karte":12,"status":"erledigt","text":"Flotte 41 gestartet","beleg_request_id":"f-5-0800-a"}
{"op":"status","karte":12,"status":"blockiert","text":"Topf militaer 1.200 von 9.000 Werteinheiten; anfrage an stratege Karte 13"}
{"op":"aendern","karte":12,"version":3,"frist_stunden":24}
```

- Status ändern darf, wer adressiert ist (auch über `alle`) oder die Karte verfasst hat; sonst 403.
- `aendern` nur der Verfasser, und `version` muss passen (sonst 409 „… hat inzwischen Version N; neu lesen …“).
- Status: `offen` → `angenommen` → `in_arbeit` → `erledigt`; `blockiert` mit Grund; `abgelehnt` mit Begründung; `hinfaellig`, wenn die Lage sich geändert hat. Die letzten drei schließen die Karte.
- `erledigt` mit `beleg_request_id`: Der Server prüft den gespeicherten Befehl. Hatte er eine Ablehnung: 409 „… Karte nicht als erledigt belegt – Status blockiert oder in_arbeit setzen“.
- Grenzen: Antwort 800 Zeichen, 30 Antworten je Karte, 80 offene Karten, 300 Karten insgesamt (älteste abgeschlossene fallen weg).

### Regeln für die Zusammenarbeit

1. **Annehmen, dann arbeiten.** Wer eine Karte übernimmt, setzt `angenommen` oder `in_arbeit` mit einem Satz zum Plan. Eine angenommene Karte bearbeitet keine andere Rolle.
2. **Keine Doppelarbeit.** Vor `neu` die offenen Karten mit gleichem `bezug` lesen; lieber antworten als eine zweite Karte anlegen.
3. **Erledigt heißt bewiesen.** Ohne angenommenen Befehl kein `erledigt`. Teilerfolg ist `in_arbeit` mit Rest.
4. **Blockiert heißt begründet.** Grund aus dem Ablehnungstext übernehmen und sagen, wer ihn beheben kann (mit eigener Anfrage an diese Rolle).
5. **Fristen ernst nehmen.** Überfällige Karten stehen in der Agenda mit Priorität 1. Kann eine Frist nicht gehalten werden: antworten, neue Frist nennen.
6. **Aufräumen.** Hinfällige Karten schließen (`hinfaellig`), statt sie liegen zu lassen; 80 offene Karten sind die Grenze.
7. **Übergaben ankündigen.** Wer etwas bereitstellt, das eine andere Rolle braucht (Kolonieschiff, Treibstoff am Startplaneten, Vertrag), meldet es als Antwort auf deren Karte mit Ort und Menge.

## 5 Konfliktregeln

1. **Besitzerkarten gehen vor.** Karten mit `von: "besitzer"` sind Wünsche des Menschen, dem das Reich gehört.
2. **Beschluss des Strategen gilt.** Widerspricht ein Auftrag einem offenen `beschluss`, gilt der Beschluss; der Auftrag wird `abgelehnt` mit Verweis auf die Beschlusskarte.
3. **Sicherheit vor Wachstum** bei sichtbarem Anflug: Feldherr-Aufträge zum Saven und Verteidigen haben Vorrang vor Bauwünschen, bis der Anflug vorbei ist.
4. **Zuständigkeit vor Eile.** Keine Rolle umgeht eine andere (etwa Feldherr fertigt Transporter aus `militaer`, weil `wirtschaft` leer ist), außer der Stratege beschließt es.
5. **Uneinigkeit eskalieren.** Zwei Rollen widersprechen sich: beide stellen eine `anfrage` an den Strategen mit ihrem Vorschlag; der Stratege entscheidet mit einem `beschluss`.

## 6 Beispiele guter Karten

Kolonieeskorte (Verwalter an Feldherr):

```json
{"op":"neu","art":"auftrag","an":["feldherr"],"prioritaet":1,"frist_stunden":10,
 "titel":"Eskorte für Kolonieflug 1:27:6 → 1:27:9",
 "text":"Kolonieschiff, 3.000 Siedler und Mindestfracht stehen ab Tag 5, 06:00 auf 1:27:6 bereit. Bitte mindestens 1 bewaffnetes Schiff (Kreuzer oder leichter Jäger) auf 1:27:6 stationieren. Der Kolonieflug startet als gemeinsame Flotte; ich sende flotte_senden mit Mission kolonisieren und eurer Eskorte, sobald ihr erledigt meldet. Ziel 1:27:9 ist laut Bericht von Tag 4 frei.",
 "bezug":{"planeten":["1:27:6","1:27:9"],"aktionen":["flotte_senden","fertigen"]}}
```

Topfanfrage (Feldherr an Stratege):

```json
{"op":"neu","art":"anfrage","an":["stratege"],"prioritaet":1,"frist_stunden":6,
 "titel":"80.000 Werteinheiten militaer für 40 Raketenwerfer auf 1:27:6",
 "text":"Feindliche Flotte 77 (Varn) erreicht 1:27:6 um Tag 6, 14:00, laut Bericht 30 leichte Jäger. werkzeug kampfszenario: 40 Raketenwerfer halten. Kosten laut werkzeug kosten 80.000 Erz, also 80.000 Werteinheiten; Topf militaer hat 21.000. Vorschlag: sofort topf_umbuchen 59.000 von wirtschaft nach militaer, danach doktrin militaer 35 für 24 Stunden.",
 "bezug":{"planeten":["1:27:6"],"flotten":[77],"spieler":["Varn"]}}
```

Beschluss (Stratege an alle):

```json
{"op":"neu","art":"beschluss","an":["alle"],"prioritaet":1,
 "titel":"Bis Tag 20: Stufe IV vor Militär",
 "text":"1) Verwalter: Werft 4, Impulsantrieb 3, Astrophysik 1, Stabilität Heimat 60, Konsumdeckung 80 %. 2) Feldherr: nur Verteidigung der Heimat und Eskorten, keine Angriffe. 3) Diplomat: NAP mit allen direkten Nachbarn anbieten. Doktrin: wirtschaft 60, forschung 25, militaer 10, reserve 5."}
```

Befund nach Vertragsabschluss (Diplomat an alle):

```json
{"op":"neu","art":"befund","an":["alle"],"prioritaet":2,
 "titel":"NAP mit Varn aktiv (Vertrag 7)",
 "text":"Varn hat den Nichtangriffspakt angenommen, Kaution 2.000. Feldherr: Varn-Planeten 1:28:4 und 1:28:9 aus den Angriffszielen streichen. Ein Bruch kostet die Kaution.",
 "bezug":{"spieler":["Varn"],"vertraege":[7]}}
```

Quittung mit Beleg:

```json
{"op":"status","karte":12,"status":"erledigt","text":"Flotte 41 mit 6 Kreuzern auf 1:27:6 stationiert","beleg_request_id":"f-t5-0610-1"}
```

Schlechte Karte (nicht so): „Bitte Verteidigung verbessern“ – ohne Planet, Menge, Frist, Topf und Erledigt-Kriterium kann niemand handeln, und niemand kann die Erledigung belegen.

## 7 Großziele mit Weg (`agenda.ziele`)

Die Agenda meldet, was jetzt wartet; `agenda.ziele.liste` zeigt je Großziel den ganzen Weg vom heutigen Stand:
`stufe` (nächste Zivilisationsstufe), `kolonie` (nächste Kolonie), `rohstoffe` (Bedarf und woher: Markt, Allianz, Handel,
Raub aus eigenen Spionageberichten), `allianz` (Allianz und Diplomatie) und `dyson` (Sphäre der Allianz, in Dyson-Welten).

- Jedes Ziel hat `stand`, `erreicht` und `schritte` in Reihenfolge – nur offene Schritte.
- `wer` nennt die zuständigen Rollen, `mitglied` ein anderes Reich der Allianz, das den Schritt tun muss.
- `du: true` heißt: diese Rolle dieses Reichs ist zuständig; nur dann stehen `vorlagen` (gültige Aktionen, vor dem
  Senden mit Probe prüfen) bzw. `werkzeug` (eine fertige Abfrage, z. B. `kolonieplan`) dabei.
- Schritte anderer Rollen: Karte auf der Rats-Pinwand; Schritte anderer Mitglieder: Karte auf der Allianz-Pinwand an
  dieses Mitglied (`bereich: "allianz"`, `an: ["Name"]`).
- Dysonsphäre: Der Server wählt das Zielsystem (offenes Projekt, sonst das mit den meisten Allianzplaneten) und teilt
  die freien Plätze reihum den Mitgliedern ohne Planet dort zu. Wer einen Platz hat, sieht ihn im Agenda-Punkt
  `dyson_platz` und im Ziel `kolonie`. Der Vorschlag geht schon ab einem Allianzplaneten im System – die Leitung schlägt
  früh vor, dann füllen alle die Plätze und zahlen ein.
- Einzahlen geht nur von eigenen Planeten im Dyson-System. Liegen die Rohstoffe auf der Heimat, zeigt das Ziel zuerst
  den `transport` dorthin („danach: einzahlen“ heißt: jetzt nicht `dyson_einzahlen` senden). Fehlt einem Mitglied der
  Bestand ganz, bekommt ein Mitglied mit Überschuss den Schritt „Allianzhilfe“: direkt liefern (Allianzmitglieder
  beliefern einander ohne Spionagebericht); der Empfänger zahlt selbst ein.

## 8 Zug einer Rolle im Rat

1. `agenda` lesen; Punkte mit Priorität 1 zuerst, eigene Ablehnungen (`art: abgelehnt`) verstehen.
2. `pinwand` lesen: neue Karten annehmen oder ablehnen, eigene offene Karten fortschreiben.
3. `lage.doktrin` und offene `beschluss`-Karten gegen den eigenen Plan prüfen.
4. Ist nichts dringend: den ersten eigenen Schritt (`du: true`) in `agenda.ziele` abarbeiten.
5. Handeln (mit Probe), Karten quittieren, Bedarf an andere Rollen als neue Karte.
6. Notizbuch aktualisieren: Ziele, offene Zusagen, Kartennummern.
