/* Presentation only. Rust owns the world, passwords, discovery and bot decisions. */
"use strict";
function lokaleForm() {
  return S.ui.lokal ||= { name: "Entdecker", volk: "veyari", password: "", url: "", go: false };
}
function lokalStart() {
  const f = lokaleForm(); f.go = true;
  sende("lokal_start", { name: f.name, volk: f.volk, password: f.password }); f.password = "";
}
function seiteLokal() {
  const f = lokaleForm(), a = S.auswahl, z = S.zustand.lokal || {}, info = z.info || {};
  const input = (key, type = "text", placeholder = "") => h("input.feld", { type, value: f[key], placeholder,"aria-label":{name:"Spielername",password:"Masterpasswort",url:"Host-Adresse"}[key], autocomplete: key === "password" ? "off" : undefined, oninput: e => f[key] = e.target.value });
  return h("div.seite", null,
    h("div.seitenkopf", null, h("h1", null, z.connected ? "Deine lokale Partie" : "Singleplayer & lokale Sitzung"), h("p.matt", null, z.connected ? "Spielstand, Spielzeit und Mitspieler deiner laufenden Sitzung." : z.busy ? "Deine gespeicherte Partie wird vorbereitet …" : "Erkunde Sternenepoche offline oder spiele mit Freunden im selben WLAN oder LAN.")),
    !z.connected && !z.busy ? testpilotenKarte() : null,
    z.connected || z.busy ? null : karte("Hier auch Singleplayer offline spielen", "Alle Spielmechaniken mit speicherbarem Spielstand: Wirtschaft, Forschung, Flotten, Nachrichten, Allianzen und Handel. Die Bots handeln und antworten auf Deutsch ohne Modell oder Internet.",
      h("div.formular", null, h("label", null, "Spielername"), input("name"), h("label", null, "Volk"),
        h("select.feld", { value: f.volk, onchange: e => f.volk = e.target.value }, ["veyari", "aurelianer", "krath", "syntheten"].map(v => h("option", { value: v, selected: v === f.volk }, name(v)))),
        h("label", null, "Bots (Standard: 30)"), h("input.feld", { type: "number", "aria-label":"Anzahl Bots",min: 0, max: 400, value: a.lokale_bots, onchange: e => setze("/lokale_bots", Number(e.target.value)), disabled: z.connected || z.busy })),
      h("p.matt", null, "Bis 50 Bots: 2 Sektoren × 80 Systeme. Ab 51: 4 × 80. Ab 101: 4 × 120. Ab 201: alle 8 Sektoren mit je 120 Systemen. Weltgröße und Botzahl gelten für einen neuen Spielstand; je Botzahl bleibt ein eigener Spielstand erhalten."),
      h("div.zeile", null, knopf(a.lokaler_host_aktiv ? "Lokale Host-Sitzung starten / fortsetzen" : "Singleplayer starten / fortsetzen", lokalStart, "gold", "los", z.busy || z.connected), z.busy ? h("span", { role: "status" }, "Lokale Sitzung wird vorbereitet oder gespeichert …") : null),
      z.connected ? null : S.ui.lokal_neu
        ? h("div.zeile", null, h("span.bernstein", null, `Neue Partie mit ${a.lokale_bots} Bots${a.lokaler_host_aktiv ? " (Host)" : ""}? Der bisherige Spielstand wird in „archiv“ verschoben, nicht gelöscht.`),
            knopf("Ja, neu beginnen", () => { S.ui.lokal_neu = false; sende("lokal_neu"); }, "rot"), knopf("Abbrechen", () => { S.ui.lokal_neu = false; planen(); }, "leise"))
        : knopf("Neue Partie beginnen …", () => { S.ui.lokal_neu = true; planen(); }, "leise klein", null, z.busy)),
    z.connected || z.busy ? null : karte("Lokale Einstellungen", "Der Host hält die gemeinsame Welt. Andere installierte Clients treten mit Name und dem hier hinterlegten Masterpasswort bei.",
      schalter(a.lokaler_host_aktiv, "Lokalen Host-Server aktivieren", v => setze("/lokaler_host_aktiv", v), "Beim Start für das eigene LAN/WLAN öffnen"),
      schalter(a.lokaler_sparmodus, "Sparmodus für schwache Hardware", v => setze("/lokaler_sparmodus", v), "Ratsfiguren als Bild anzeigen; vollständige Spielmechaniken bleiben erhalten"),
      h("div.formular", null, h("label", null, "Masterpasswort (12–128 Zeichen)"), input("password", "password", z.master_da ? "Hinterlegt · leer lassen zum Wiederverwenden" : "Masterpasswort für deine Mitspieler"),
        h("label", null, "Host-Port"), h("input.feld", { type: "number", min: 1024, max: 65535, value: a.lokaler_port, onchange: e => setze("/lokaler_port", Number(e.target.value)), disabled: z.connected || z.busy })),
      h("p.matt", null, "Das Passwort wird im Windows-Anmeldeinformationsspeicher hinterlegt. Bei aktiver Host-Sitzung kann Windows eine Netzwerkfreigabe für das Cockpit anfordern. WLAN-Gastnetze können die Host-Suche blockieren; dann die LAN-IP des Hosts direkt eintragen.")),
    z.connected ? karte(info.paused ? "Partie pausiert" : "Partie läuft", `${z.own_host ? (info.host ? "Eigener Host im WLAN/LAN" : "Singleplayer") : "Gast bei einem Host"} · ${info.bots ?? "–"} Bots · ${info.sektoren ?? "–"} Sektoren · ${info.tempo ?? 30}× Geschwindigkeit`,
      h("p.matt", null, z.own_host ? "Automatische Speicherung ist aktiv. Beim Schließen wird gespeichert; beim nächsten Start öffnet sich deine Partie mit derselben Spielansicht. Eine Pause bleibt erhalten." : "Der Host speichert die gemeinsame Welt automatisch und steuert ihre Spielzeit."),
      z.gespeichert ? h("p.gruen", {role:"status"}, `Zuletzt manuell gespeichert: ${new Date(z.gespeichert*1000).toLocaleTimeString("de-DE")}`) : null,
      z.warning ? h("p.bernstein", null, z.warning) : null,
      info.fehler ? h("p.rot", { role: "alert" }, `Speicherung gestört: ${info.fehler}. Die Welt ist angehalten, damit nichts verloren geht. Freien Speicherplatz prüfen und die Sitzung neu starten.`) : null,
      z.own_host && (z.lan || []).length ? h("div", null,
        h("p", null, h("b", null, "Für Mitspieler im WLAN/LAN: "), z.lan.map(l => l.url).join(" · ")),
        h("div.zeile", null, z.lan.filter(l => l.qr).slice(0, 2).map(l => h("figure", null, h("img", { src: "data:image/svg+xml;charset=utf-8," + encodeURIComponent(l.qr), alt: `QR-Code für ${l.url}`, width: 168, height: 168 }), h("figcaption.matt.klein", null, l.url)))),
        h("p.matt.klein", null, "Der QR-Code enthält nur die Adresse; er öffnet das Spiel im Browser eines Geräts im selben Netz. Name und Masterpasswort geben Mitspieler selbst ein. Ein Name bleibt an das Gerät gebunden, mit dem er zuerst beigetreten ist.")) : null,
      z.own_host && info.host && Array.isArray(info.teilnehmer) ? h("div.liste", { "aria-label": "Teilnehmer der Runde" },
        info.teilnehmer.filter(t => t.bindung !== "host").map(t => h("div.eintrag", null,
          h("div.text", null, h("b", null, t.name), h("span", null, `${name(t.mode)} · ${{ geraet: "an sein Gerät gebunden", frei: "frei für den nächsten Beitritt", alt: "frühere Anmeldung" }[t.bindung] || t.bindung}${t.spieler === null ? " · wählt noch Volk und Spielweise" : ""}`)),
          t.bindung === "geraet" ? knopf("Name freigeben", () => sende("lokal_steuern", { body: { name_freigeben: t.name } }), "leise klein") : null))) : null,
      h("div.zeile", null, knopf("Spiel öffnen", () => nav("spiel"), "gold", "spiel"),
        z.own_host ? knopf(z.steuert ? "Wird geändert …" : info.paused ? "Fortsetzen" : "Pause", () => sende("lokal_steuern", { body: { paused: !info.paused } }), "", info.paused ? "los" : "pause", z.steuert || z.speichert) : null,
        z.own_host ? knopf(z.speichert ? "Wird gespeichert …" : "Speichern", () => sende("lokal_speichern"), "", "haken", z.speichert || z.busy || z.steuert) : null,
        z.own_host ? h("select.feld", { value: info.tempo || 30, onchange: e => sende("lokal_steuern", { body: { tempo: Number(e.target.value) } }) }, [1, 10, 30, 60].map(t => h("option", { value: t, selected: t === (info.tempo || 30) }, `${t}× Spielzeit`))) : null,
        knopf(z.own_host ? "Speichern & Sitzung verlassen" : "Sitzung verlassen", () => sende("lokal_verlassen"), "leise", "stopp", z.speichert || z.busy || z.steuert),
        z.own_host ? knopf("Speichern & Cockpit schließen", () => sende("beenden"), "leise", "kreuz", z.speichert || z.busy || z.steuert) : null)) : null,
    lokalKapazitaet(),
    z.connected || z.busy ? null : karte("Lokale Sitzung finden", "Suche im selben WLAN/LAN. Dein Name und das Masterpasswort des Hosts genügen zum Beitritt.",
      knopf("Hosts im Netzwerk suchen", () => sende("lokal_suchen"), "", "suche", z.busy),
      h("div.liste", null, (z.hosts || []).map(v => h("div.eintrag", null, h("div.text", null, h("b", null, v.name), h("span", null, `${v.url} · ${v.bots} Bots`)), knopf("Auswählen", () => { f.url = v.url; planen(); }, "leise")))),
      h("div.formular", null, h("label", null, "Host-Adresse"), input("url", "text", "http://192.168.1.20:8892")),
      knopf("Mit Name & Masterpasswort beitreten", () => { f.go = true; sende("lokal_beitreten", { url: f.url, name: f.name, volk: f.volk, password: f.password }); f.password = ""; }, "gold", "los", z.busy || z.connected),
      h("p.matt", null, "Die Zugangsdaten oben gelten auch für den Beitritt. Der gemeinsame Spielstand bleibt beim Host; beim Beenden des Hosts endet die Sitzung für alle.")),
    karte("Bot-Funk", "Vier kurze deutsche Funksprüche, lokal vorproduziert. Wiedergabe nur auf deinen Klick.",
      h("div.raster.r2", null, [["oekonom","Ökonom"], ["haendler","Händler"], ["igel","Verteidiger"], ["raeuber","Räuber"]].map(([t,label]) => h("div", null, h("b", null, label), h("audio", { controls: true, preload: "none", src: `bot-stimmen/${t}.ogg`, "aria-label": `Funkspruch ${label}` }))))));
}

function lokalKapazitaet() {
  const z=S.zustand.lokal||{}, k=z.kapazitaet||{}, run=z.agenten||{}, hw=k.hardware||{}, gpu=k.gpu||{};
  const load=run.zuege?Number(run.last||0):Number(k.last||0);
  const info=z.info||{}, p95=Number(info.tick_p95_ms||0);
  const menschen=Number(k.empfohlene_spielerplaetze||0);
  return karte("System & Spielraum",`${hw.cpu||"Hardware wird ermittelt"} · ${hw.cores||"–"} CPU-Threads · ${zahl(hw.ram_free_mb||0)} MB RAM frei · ${gpu.name||"GPU-Speicher unbekannt"}`,
    h("div.zeile",null,h("b",null,`${Math.round(load*100)} % Agentenbudget ${run.zuege?"aus gemessenen Zugzeiten":"geschätzt"}`),chip(run.gedrosselt?"Takt automatisch verlängert":load>.65?"Takt verlangsamen":"Reserve vorhanden",load>.65?"bernstein":"gruen")),
    h("progress.kapazitaet",{max:100,value:Math.min(100,load*100),"aria-label":"Auslastung des Agentenbudgets"}),
    h("p.matt.klein",null,`${k.aktiv||0} Agenten aktiviert · Empfehlung für diesen Modellmix: etwa ${k.empfohlene_plaetze||0} Plätze bei 40 % Reserve. ${k.fehlend||0} Plätze ohne passendes Modell. Spitzenbedarf der Modellwarteschlange: ${zahl(k.spitzen_speicher_mb||0)} MB. ${k.hinweis||""}`),
    h("p.matt.klein",null,z.connected?`Lokale Welt: ${z.info?.belegt||0} belegte Plätze · ${z.info?.freigabe||0} freigegeben · bis ${z.info?.plaetze||0} konfigurierbar. Menschen und Agenten teilen diese Plätze. Hardware-Schätzungen ersetzen keinen Lasttest.`:`Geplante Welt: bis ${z.potentielle_plaetze||0} zusätzliche Spielerplätze; beim Start höchstens 32 freigegeben. Menschen und Agenten teilen diese Plätze. Hardware-Schätzungen ersetzen keinen Lasttest.`),
    h("p.matt.klein",null,`Vorläufiger Spielraum ohne zusätzliche Modelle: etwa ${menschen} Menschen/Agenten. ${p95?`Gemessener Welt-Tick: ${p95.toFixed(1)} ms (95. Perzentil aus ${info.tick_samples} Messungen, Intervall 5 s). Hochrechnung mit 40 % Zeitreserve und 128 MB RAM-Reserve je Teilnehmer.`:"Noch keine Tickmessung: vorsichtiger Ausgangswert aus CPU-Threads und freiem RAM."} Modellplätze sind zusätzlich durch das Agentenbudget begrenzt. Bei hoher Last Tempo oder Spielerfreigabe reduzieren.`),
    knopf("Hardware & Modelle neu prüfen",()=>sende("lokal_hardware"),"leise klein"),
    z.own_host?h("div.formular",null,h("label",null,"Freigegebene Spielerplätze"),h("input.feld",{type:"number",min:1,max:z.info?.plaetze||z.potentielle_plaetze||1,value:z.info?.freigabe||32,onchange:e=>sende("lokal_steuern",{body:{plaetze:Number(e.target.value)}})})):null,
    S.ui.seite==="lokal"?lokalAgentenPlaetze():knopf("Agentenplätze einstellen",()=>nav("lokal"),"leise klein"));
}
function lokalAgentenPlaetze(){
  const slots=S.auswahl.lokale_agenten||[], z=S.zustand.lokal||{}, run=z.agenten||{};
  const change=(i,key,value)=>setze(`/lokale_agenten/${i}/${key}`,value);
  const field=(i,p,key,label,type="text",min,max)=>[h("label",null,label),h("input.feld",{type,value:p[key],min,max,"aria-label":label,disabled:run.laeuft,list:key==="modell"?"lokale-modelle":undefined,oninput:e=>{if(type!=="number")change(i,key,e.target.value);},onblur:e=>change(i,key,type==="number"?Number(e.target.value):e.target.value),onchange:e=>change(i,key,type==="number"?Number(e.target.value):e.target.value)})];
  const select=(i,p,key,options)=>h("select.feld",{disabled:run.laeuft,onchange:e=>change(i,key,e.target.value)},options.map(([v,t])=>h("option",{value:v,selected:v===p[key]},t)));
  return h("details.agenten",{id:"agenten-plaetze",open:!!S.ui.lokal_agenten_offen,ontoggle:e=>S.ui.lokal_agenten_offen=e.target.open},h("summary",null,"Eigene Agentenspieler · sechs vorbereitete Plätze"),
    h("p.matt",null,"Jeder Platz führt ein eigenes Reich. Sparsam wählt ein kleines installiertes Modell, Mittel ein mittleres, Max das größte passende. Custom erlaubt deinen eigenen Mix. Modellaufrufe laufen einzeln; keine automatischen Downloads. Eigene API verwendet den in der Werkstatt hinterlegten Endpunkt und Schlüssel."),
    h("div.raster.r2",null,slots.map((p,i)=>h("div.agenten-platz",null,
      h("label.zeile",null,h("input",{type:"checkbox",checked:p.aktiv,disabled:run.laeuft,"aria-label":`Platz ${i+1} aktiv`,onchange:e=>change(i,"aktiv",e.target.checked)}),`Platz ${i+1} aktiv`),h("div.formular",null,
        field(i,p,"name","Name"),h("label",null,"Profil"),select(i,p,"profil",[["sparsam","Sparsam"],["mittel","Mittel"],["max","Max"],["custom","Custom"]]),
        h("label",null,"Modellquelle"),select(i,p,"anbieter",[["ollama","Ollama lokal"],["ollama_cpu","Ollama nur CPU"],["eigen","Eigene API"]]),field(i,p,"modell","Custom-Modell / API-Modell"),
        field(i,p,"takt","Takt in Sekunden","number",30,86400),field(i,p,"kontext","Kontext in Tokens","number",2048,32768)),
      h("p.matt.klein",null,(z.kapazitaet?.modelle||[]).find(m=>m.name===p.name)?.modell||"Modell wird nach Aktivierung vorgeschlagen")))),
    h("datalist",{id:"lokale-modelle"},(S.zustand.ollama||[]).map(m=>h("option",{value:m.name}))),
    h("div.zeile",null,knopf("Weiteren Agentenplatz hinzufügen",()=>setze("/lokale_agenten",slots.concat({aktiv:false,name:`Agent-${slots.length+1}`,profil:"sparsam",modell:"",anbieter:"ollama",takt:180,kontext:8192})),"leise klein",null,run.laeuft||slots.length>=(z.info?.plaetze||768)),
      run.laeuft?knopf(run.stoppt?"Wartet auf laufenden Modellzug …":"Agentenspieler anhalten",()=>sende("lokal_agenten_stop"),"rot", "stopp",run.stoppt):knopf("Aktive Agentenspieler starten",()=>sende("lokal_agenten_start"),"gold", "los",!z.own_host||!slots.some(p=>p.aktiv))),
    run.aktuell?h("p.matt",{role:"status"},run.aktuell):null,
    h("div.liste",null,(run.spieler||[]).map(p=>h("div.eintrag",null,h("b",null,p.name),h("span",null,`${p.modell} · ${p.zuege} Züge · ${Number(p.sekunden||0).toFixed(1)} s/Zug`),p.fehler?h("span.rot",null,p.fehler):null))),run.fehler?h("p.rot",null,run.fehler):null);
}

