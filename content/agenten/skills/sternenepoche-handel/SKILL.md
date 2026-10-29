---
name: sternenepoche-handel
description: Laden, wenn Rohstoffe, Credits oder Schiffe beschafft, verkauft, getauscht, verschenkt oder geliefert werden sollen – Markt, Soforthandel, Allianzmarkt, Schiffsangebote, Geschenke, Tribut, Lieferungen an fremde Reiche – oder wenn Handel gegen Raub abzuwägen ist.
---

# Sternenepoche: Handel, Tausch, Lieferung, Raub

Verträge (Handelsabkommen, Tribut) und Verhandlung: **sternenepoche-diplomatie**. Angriff und Beute im Detail:
**sternenepoche-kampf**. Flugfelder: **sternenepoche-flotte**. Produktion und Lager: **sternenepoche-wirtschaft**.

## Kurz

1. **Wer zahlt woraus:** `markt_order` und `soforthandel` nur der Verwalter (Topf `wirtschaft` und Credits);
   der Allianzmarkt Verwalter (Topf `wirtschaft`) oder Diplomat (Topf `reserve`); `schenken`, Kautionen und die
   Annahme eines Schiffsangebots gehen direkt von `lage.credits`.
2. **Markt ist ein Orderbuch zwischen Spielern** – kein Händler im Hintergrund. Ohne Gegenorder bleibt eine Order offen.
3. **Gebühr 2 % je Seite**, Aurelianer zahlen die Hälfte, ein Handelsabkommen mit dem Gegenüber halbiert sie nochmals.
4. **Ware wird geliefert, nicht teleportiert:** neutrale Handelsflotte mit Flugzeit, an den Käufer gebunden, wartet bei
   Blockade, kann nicht abgefangen werden.
5. **Soforthandel** tauscht Erz, Kristall, Deuterium, Nahrung und Credits am eigenen Planeten zu festen Kursen
   (1 : 1,5 : 3 : 0,5 : 1) minus 2 % – teuer, aber sofort und ohne Gegenpartei.
6. **Vorschau zuerst:** `marktplan`, `werkzeug soforthandel`, `handeln probe=true`.
7. **Transporte an fremde Reiche sind erlaubt**, gehören dem Planetenbesitzer beim Abflug und brauchen einen eigenen
   Spionagebericht des Ziels. Ein Brief allein liefert nichts.
8. **Raub statt Handel nur mit Rechnung:** Beute = 40 % (Krath 50 %) über dem Bunkerschutz, begrenzt durch Frachtraum;
   dazu Treibstoff, Verluste, Gegenschlag, Ende des Anfängerschutzes und Bruch bestehender Pakte.

## Markt (markt.rs)

```json
{"typ":"markt_order","planet":"1:27:6","gut":"kristall","seite":"kauf","menge":5000,"preis":1.6}
{"typ":"markt_order","planet":"1:27:6","gut":"erz","seite":"verkauf","menge":20000,"preis":1.05}
{"typ":"markt_storno","order":12}
```

- Voraussetzung: Marktgebäude auf dem Orderplaneten (ab Zivilisationsstufe III). Je Marktstufe 5 offene Orders auf
  diesem Planeten; interne Allianzangebote zählen mit. `menge` mindestens 1, `preis` (Credits je Einheit) über 0.
- **Kauf:** Der Kern hinterlegt Menge × Preis + Gebühr aus den Credits **und** aus dem Topf `wirtschaft`. Er handelt
  sofort mit der billigsten passenden Verkaufsorder (Preis ≤ deinem Limit, bei Gleichstand die ältere) **zu deren
  Preis**; die Differenz zum Limit fließt in Credits und Topf zurück. Der Rest steht im Buch.
- **Verkauf:** Die Ware wird am Planeten reserviert. Gehandelt wird mit der höchsten Kauforder ≥ deinem Preis zu
  **deren** Preis; der Erlös (abzüglich deiner Gebühr) kommt sofort. Der Rest steht im Buch.
- **Storno:** gibt offene Reste zurück (Credits und Topfanteil bzw. Ware auf den Planeten).
- **Lieferung:** neutrale Handelsflotte, Flugzeit nach Entfernung; an den Käufer gebunden. Wechselt der Zielplanet den
  Besitzer, fliegt die Ware weiter zur Heimatwelt des Käufers; bei Blockade wartet sie.
- Lage: `lage.markt.preise` (je Gut `verkauf_ab` = billigste Verkaufsorder, `kauf_bis` = höchste Kauforder),
  `lage.markt.orders[]` (eigene: `order`, `planet`, `gut`, `seite`, `menge`, `preis`), `lage.marktlieferungen[]`
  (`ziel`, `gut`, `menge`, `naechste_ankunft`).
- Vorschau: `{"typ":"marktplan","aktion":{"typ":"markt_order",…}}` → `status` ready/blocked, `credits_vorher_milli`,
  `credits_nachher_milli`, erwartete Lieferungen.

**Rechenbeispiel Gebühr:** 500 Kristall zu 2 Credits = 1.000 Credits Warenwert; bei 2 % Käufergebühr werden 1.020
hinterlegt, als Aurelianer 1.010, mit Handelsabkommen zum Verkäufer jeweils die Hälfte der Gebühr.

**Preisregeln für Agenten:** Kauflimit knapp über `verkauf_ab`, Verkaufspreis knapp unter `kauf_bis`, wenn es sofort
gehen soll; sonst geduldig ins Buch stellen. Preise nie aus fremden Briefen übernehmen, ohne `lage.markt.preise` zu
vergleichen. Der Soforthandelskurs ist eine gute Obergrenze für Käufe der vier Grundgüter.

## Soforthandel

```json
{"typ":"soforthandel","planet":"1:27:6","angebot":{"kristall":1000},"ziel":"deuterium"}
```

- Nur Verwalter, nur eigener Planet. Angebots- und Zielgüter: `erz`, `kristall`, `deuterium`, `nahrung`, `credits`
  (Angebot und Ziel verschieden, mehrere Angebotsgüter erlaubt).
- Erhalt = Σ (Menge × Kurs) × (1 − 0,02) ÷ Kurs des Ziels; Kurse Erz 1, Kristall 1,5, Deuterium 3, Nahrung 0,5,
  Credits 1. Beispiel oben: 1.000 × 1,5 × 0,98 ÷ 3 = 490 Deuterium.
- Das Ziellager muss Platz haben; das Ergebnis muss mindestens eine Einheit sein. Der Topf `wirtschaft` zahlt den
  Wert des Angebots.
- Vorschau ohne Wirkung: `werkzeug soforthandel {planet, angebot, ziel}` → `erhalt`, `gebuehr_prozent`, `kurse`.

## Interner Allianzmarkt

```json
{"typ":"intern_anbieten","planet":"1:27:6","gut":"legierung","menge":2000,"preis":4.5}
{"typ":"intern_kaufen","angebot":9,"planet":"1:27:9"}
{"typ":"intern_storno","angebot":9}
```

- Nur in einer Allianz; Diplomat oder Verwalter. Das Angebot braucht einen Markt am Angebotsplaneten, reserviert die
  Ware sofort und teilt sich das Orderlimit mit dem öffentlichen Markt. Preis 0 ist erlaubt (Hilfe unter Mitgliedern).
- Gekauft wird immer das ganze Angebot an einen eigenen Zielplaneten. Käufer zahlt Wert + eigene Gebühr (Credits und
  Rollentopf), Verkäufer erhält Wert − eigene Gebühr. Lieferung neutral mit Flugzeit; Blockade verzögert,
  Besitzerwechsel leitet um, Niederlage des Käufers verliert die Ware.
- Offene Angebote fallen weg, wenn der Anbieter die Allianz verlässt oder den Planeten verliert (Ware zurück).
- Lage: `lage.kommunikation.allianzbereich.angebote[]` (`id`, `von`, `planet`, `gut`, `menge`, `preis`, `status`
  offen/unterwegs/geliefert/storniert, `kaeufer`, `ankunft`).

## Schiffhandel (schiffhandel.rs)

Schiffe samt **gesamter Zuladung**, Personen und Truppen an ein anderes Reich übergeben – als Geschenk (`preis: 0`)
oder gegen Credits.

```json
{"typ":"schiffangebot","preis":250000,
 "personen":{"gruppen":[[0,0,0],[0,0,0],[0,0,0],[0,0,0]]},"truppen":[0,0,0,0,0,0,0,0,0,0,0,0],
 "flug":{"typ":"flotte_senden","start":"1:27:6","ziel":"1:29:4","mission":"transport",
  "schiffe":{"grosser_transporter":10},"geschwindigkeit":1.0,"ladung":{"deuterium":20000},"haltedauer_stunden":0}}
```

```json
{"typ":"schiffangebot_antwort","angebot":118,"annehmen":true}
{"typ":"orbitflotte_anbieten","flotte":23,"ziel":"1:29:4","preis":0}
```

- Diplomat oder Feldherr. `flug` ist ein `transport` zum bewohnten Planeten (oder mit `zielart: "mond"` zum Mond) des
  Empfängers; der Kern prüft den Flug vorab. Die Schiffe bleiben bis zur Annahme am Start verfügbar.
- Höchstens 20 offene Angebote; ein Angebot gilt einen Spieltag. Annehmen kann nur der Empfänger, beenden
  (`annehmen: false`) beide Seiten.
- Bei Annahme zahlt der Empfänger den Preis aus seinen Credits (hinterlegt bis zur Zustellung), und die Flotte startet.
  Bei Ankunft gehören Schiffe, Ladung, Personen und Truppen dem Empfänger; der Anbieter erhält den Preis. Scheitert
  die Zustellung (Besitzerwechsel, Blockade, Rückruf, Verlust), gehen die Credits zurück und die Schiffe bleiben beim
  Absender.
- `orbitflotte_anbieten`: dieselbe Übergabe für eine eigene gestrandete Flotte an den Besitzer des Zielplaneten; sie
  startet bei Annahme vom Orbitort ohne Rückflug.
- Lage: `lage.schiffangebote[]` (`id`, `von`, `an`, `eingang`, `preis`, `status`, `flug`, `ablauf`).

## Geschenke, Tribut, Handelsabkommen

- `{"typ":"schenken","an":"Aris","credits":5000}` – nur der Diplomat, nur Credits, direkt aus `lage.credits`.
- Güter verschenken: `transport` an den fremden Planeten (unten), `intern_anbieten` mit Preis 0 (Allianz) oder ein
  Schiffsangebot mit Preis 0 (Schiffe gehen mit).
- **Tribut-Vertrag:** der Anbieter zahlt täglich Credits oder ein Gut von Heimatwelt zu Heimatwelt; Ausfall ist ein
  Vertragsbruch (sternenepoche-diplomatie). Taugt als bezahlter Frieden oder Ratenkauf.
- **Handelsabkommen** (Vertrag oder Allianzpakt): halbe Marktgebühr zwischen den Partnern, jederzeit kündbar.

## Transport an fremde Spieler

```json
{"typ":"flotte_senden","start":"1:27:6","ziel":"1:29:4","mission":"transport",
 "schiffe":{"grosser_transporter":8},"geschwindigkeit":1.0,"ladung":{"erz":100000,"kristall":50000},"haltedauer_stunden":0}
```

- Erlaubt für Verwalter und Feldherr, auch an Verbündete und Fremde. Das Ziel muss **selbst ausgespäht** sein
  (`lage.berichte[]`/`lage.erkundet[]`); Mitglieder der eigenen Allianz beliefert man ohne Bericht (Karl 28.10.2026), Partner nur eines Verteidigungsbündnisses brauchen ihn.
- Die Ladung ist an den Besitzer **beim Abflug** gebunden (`lage.flotten[].gebundener_empfaenger`). Hat der Planet bei
  Ankunft einen anderen Besitzer oder ist er für den Empfänger blockiert, wird nichts geliefert und die Fracht kehrt um.
- Treibstoff für beide Strecken zahlt der Absender; die Schiffe kommen zurück. Für einen fairen Tausch zuerst die
  Gegenleistung sichern (Marktorder des Partners, Tribut-Vertrag, Kaution), dann liefern.

## Rohstoffe durch Überfall

Rechnung vor dem Angriff (Werkzeuge in sternenepoche-kampf):

1. Bericht mit Bestand und möglichst Gebäuden (Bunkerstufe). Schutz je Bunkerstufe: 20.000 von Erz, Kristall,
   Deuterium, Nahrung, 4.000 von verarbeiteten Gütern.
2. Beute je Gut = Plünderquote × (Bestand − Schutz); Quote 0,4, Krath 0,5. Credits sind keine Beute.
3. Obergrenze: freier Frachtraum der überlebenden Flotte (`werkzeug flugzeit` → `ladekapazitaet`; `kampfsimulator` →
   `beute_mittel_milli`).
4. Kosten: Treibstoff hin und zurück, erwartete Verluste (Simulation), Unterhalt der Frachter.
5. Folgen: eigener Anfängerschutz endet, Pakte mit dem Opfer brechen (Kaution weg, öffentliches Register), das Opfer
   und seine Verbündeten können zurückschlagen; Plünderung kostet das Opfer Stabilität.

**Entscheidungsregel Handel vs. Raub:** Raub lohnt, wenn erwartete Beute + Trümmerwert deutlich über Treibstoff +
erwarteten Verlusten liegt, das Ziel keine Verbündeten mit Halteflotten hat und der Strategen-`beschluss` Angriffe auf
dieses Reich erlaubt. Sonst kaufen: Markt, wenn Zeit ist; Soforthandel, wenn es eilt; Allianzmarkt, wenn Mitglieder
Überschuss haben; Bot-Handel per Brief und Marktorder. Gegen Paktpartner und Verbündete kein Raub.

## Handel mit Bots

Bots antworten auf Briefe wie „Ich brauche 2000 Legierung" oder „Tausche 4000 Kristall gegen 600 Elektronik" mit Menge,
Stückpreis und Weg im Markt. Dann stellt der **Verwalter** die passende Order ein (`markt_order`); der Bot bedient sie im
nächsten Zug. Ein Tausch sind zwei Orders gegen Credits. Der Diplomat verhandelt, der Verwalter bucht – Absprache über
die Pinwand mit Gut, Menge, Preis und Planet.

## Ablauf eines Handelszugs

1. Bedarf aus der Pinwand (Auftrag des Verwalters/Strategen) und `lage.planeten[].bestand`/Lagergrenzen lesen.
2. Preise: `lage.markt.preise`; Soforthandel-Vorschau; Allianzangebote.
3. Weg wählen (Kurz-Regeln oben), Topf prüfen (`lage.toepfe`), Vorschau (`marktplan`, `probe=true`).
4. Senden, Order-ID oder Angebots-ID notieren, Lieferzeit (`lage.marktlieferungen[]`) in die Notiz.
5. Pinwand-Karte mit `beleg_request_id` schließen; Partner die ID nennen.
