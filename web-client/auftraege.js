// Auftragssystematik (Karl, 18.10.2026): ein Symbol und eine Farbe je Auftragsart, überall gleich –
// Versand, Weiter-Knopf, letzte Ziele, Ereignisliste, Flottenkommando, Galaxie und Berichte.
// Farbe = Gruppe (was der Auftrag bewirkt), Symbol = genauer Auftrag. Für alle vier Völker identisch,
// damit jeder die Zeichen sofort liest; Volksfarben bleiben der Spieleridentität vorbehalten.
(function(){
 const G={
  kampf:{name:'Kampf',farbe:'#ff5a5f'},
  boden:{name:'Bodenkampf',farbe:'#ff9d3c'},
  aufklaerung:{name:'Aufklärung',farbe:'#b48cff'},
  fracht:{name:'Fracht',farbe:'#f2c94c'},
  verlegung:{name:'Verlegung',farbe:'#7bd88f'},
  schutz:{name:'Schutz',farbe:'#4fa8ff'},
  rohstoffe:{name:'Rohstoffe',farbe:'#2fd0c0'},
  ausbreitung:{name:'Ausbreitung',farbe:'#f472b6'}
 };
 // 24×24, Linienzeichnung (stroke=currentColor), lesbar ab 14 px.
 const P={
  angriff:'<circle cx="12" cy="12" r="7"/><path d="M12 2v5M12 17v5M2 12h5M17 12h5"/><circle cx="12" cy="12" r="1.2" fill="currentColor"/>',
  verband:'<path d="M3 5l7 7-7 7M10 5l7 7-7 7M21 4v16"/>',
  blockade:'<circle cx="12" cy="12" r="4"/><path d="M3 8V3h5M16 3h5v5M21 16v5h-5M8 21H3v-5"/>',
  bombardieren:'<circle cx="10.5" cy="14" r="6.5"/><path d="M15 9.5l2.5-2.5M19 3v2M22 6h-2M17.5 4.5l1 1"/>',
  mond_zerstoeren:'<path d="M20 14.5A8.5 8.5 0 1 1 9.5 4a6.5 6.5 0 0 0 10.5 10.5z"/><path d="M8 10l3 3-2 2 3 3"/>',
  invasion:'<path d="M4 15a8 8 0 0 1 16 0v2H4z"/><path d="M12 7v4M2 17h20M7 21h10"/>',
  kampfkolonisieren:'<path d="M5 21V4h11l-2.5 4L16 12H5"/><path d="M14 15l6 6M20 15l-6 6"/>',
  transport:'<path d="M3 8l9-5 9 5v8l-9 5-9-5z"/><path d="M3 8l9 5 9-5M12 13v8"/>',
  stationieren:'<circle cx="12" cy="5" r="2"/><path d="M12 7v14M8 10h8M4 13a8 8 0 0 0 16 0"/>',
  halten:'<path d="M12 3l8 3v6c0 5-3.5 8-8 9-4.5-1-8-4-8-9V6z"/><path d="M12 8.5v6M9 11.5h6"/>',
  saven:'<path d="M20.5 13A8.5 8.5 0 1 1 18 6.3"/><path d="M21 3v5h-5"/><path d="M12 8v4l2.5 2"/>',
  spionage:'<path d="M2 12s4-7 10-7 10 7 10 7-4 7-10 7S2 12 2 12z"/><circle cx="12" cy="12" r="3"/>',
  flotten_spionage:'<path d="M2 11s4-6 10-6 10 6 10 6-4 6-10 6S2 11 2 11z"/><circle cx="12" cy="11" r="2.5"/><path d="M8 21l4-3 4 3"/>',
  system_erkunden:'<circle cx="12" cy="12" r="9"/><circle cx="12" cy="12" r="4.5"/><path d="M12 12l6-6"/>',
  kolonisieren:'<path d="M5 21V4M5 4h12l-3 4 3 4H5"/>',
  recyceln:'<path d="M3 4h18l-7 8v6l-4 2v-8z"/>',
  abbau:'<path d="M6 3h12l4 6-10 12L2 9z"/><path d="M2 9h20M10 3L8 9l4 12 4-12-2-6"/>',
  expedition:'<circle cx="12" cy="12" r="9"/><path d="M16 8l-2 6-6 2 2-6z"/>',
  terraformen:'<circle cx="12" cy="12" r="8.5"/><path d="M4 10.5c2.6-1.6 5.2 1.6 8 0s5.4-1.6 8 0M4.6 15c2.4-1.4 4.8 1.4 7.4 0s5-1.4 7.4 0"/><path d="M12 3.5v-2M12 22.5v-2"/>',
  rakete:'<path d="M12 2c3 2 5 6 5 10l-2 4H9l-2-4c0-4 2-8 5-10z"/><path d="M9 16l-3 4M15 16l3 4"/><circle cx="12" cy="9" r="1.3"/>',
  truppen:'<path d="M4 14a8 8 0 0 1 16 0v3H4z"/><path d="M12 6v4M2 17h20"/>',
  verstaerken:'<path d="M12 3l8 3v6c0 5-3.5 8-8 9-4.5-1-8-4-8-9V6z"/><path d="M7.5 14a4.5 4.5 0 0 1 9 0v1.5h-9z"/><path d="M12 9.5v2"/>',
  rueckflug:'<path d="M20 12H5M11 6l-6 6 6 6"/>'
 };
 const SONDE=['spionagesonde'],KOLONIE=['kolonieschiff','grosses_kolonieschiff'];
 // ziel: wo der Auftrag passt (fremd, verbuendet, eigen, leer, truemmer, guertel, mond).
 const A={
  angriff:{gruppe:'kampf',name:'Angriff',zweck:'Kampf, bei Sieg Beute',ziel:['fremd','mond'],feindlich:true,aufklaerung:true},
  verband:{gruppe:'kampf',name:'Verbandsangriff',zweck:'Mehrere Flotten greifen gemeinsam an',ziel:['fremd','mond'],feindlich:true,aufklaerung:true,pseudo:true},
  blockade:{gruppe:'kampf',name:'Blockade',zweck:'Orbit besetzen, Lieferungen sperren',ziel:['fremd'],feindlich:true,aufklaerung:true,ab:4},
  bombardieren:{gruppe:'kampf',name:'Bombardieren',zweck:'Orbit erobern, Bomber senken alle Gebäude alle 15 min bis 30 %',ziel:['fremd'],feindlich:true,aufklaerung:true,braucht:['bomber','strategischer_bomber'],ab:4},
  mond_zerstoeren:{gruppe:'kampf',name:'Mond zerstören',zweck:'Weltenkrümmer gegen einen Mond',ziel:['mond'],feindlich:true,braucht:['weltentruemmer']},
  invasion:{gruppe:'boden',name:'Invasion',zweck:'Kolonie belagern und erobern',ziel:['fremd'],feindlich:true,aufklaerung:true,braucht:['truppentransporter'],ab:4},
  kampfkolonisieren:{gruppe:'boden',name:'Kampfkolonisieren',zweck:'Orbit erobern, Truppen landen, 48 h Bodenkampf, dann übernimmt das Kolonieschiff',ziel:['fremd'],feindlich:true,aufklaerung:true,braucht:KOLONIE,ab:4,keineHeimat:true},
  transport:{gruppe:'fracht',name:'Transport',zweck:'Güter, Personen und Truppen liefern; Schiffe kehren zurück',ziel:['eigen','verbuendet','fremd'],fracht:true},
  // Flottenübergabe (Karl 24.10.2026): verschenken oder verkaufen in einem Auftrag (Kern: schiffangebot, Preis 0 = Geschenk).
  uebergeben:{gruppe:'fracht',name:'Flottenübergabe',zweck:'Schiffe samt Ladung und Personen verschenken oder verkaufen; der Empfänger nimmt an',ziel:['verbuendet','fremd'],fracht:true},
  schiffhandel:{gruppe:'fracht',name:'Schiffe verkaufen',zweck:'Flottenübergabe gegen Credits',ziel:['verbuendet','fremd'],fracht:true,pseudo:true},
  stationieren:{gruppe:'verlegung',name:'Stationieren',zweck:'Eigener Planet oder Mond: Schiffe und Zuladung bleiben bei dir',ziel:['eigen'],fracht:true,einweg:true},
  halten:{gruppe:'schutz',name:'Halten',zweck:'Verbündeten im Orbit mitverteidigen',ziel:['verbuendet'],dauer:true},
  // Invasion v1: Bodentruppen zu einem verbündeten Planeten (boden_senden mit flug.mission "verstaerken", wird zu halten).
  verstaerken:{gruppe:'schutz',name:'Bodenhilfe',zweck:'Bodentruppen helfen einem Verbündeten am Boden – bei seiner Verteidigung oder seiner Invasion',ziel:['verbuendet'],braucht:['truppentransporter'],truppen:true},
  saven:{gruppe:'schutz',name:'Saven',zweck:'Flotte und Fracht aus der Gefahr bringen',ziel:['eigen','leer','guertel','truemmer'],fracht:true,dauer:true},
  spionage:{gruppe:'aufklaerung',name:'Ausspähen',zweck:'Bericht über Planet, Flotte, Abwehr',ziel:['fremd','verbuendet','mond'],braucht:SONDE,nur:SONDE},
  flotten_spionage:{gruppe:'aufklaerung',name:'Flotte ausspähen',zweck:'Erkannten feindlichen Anflug vor seiner Ankunft aufklären',ziel:['fremd'],braucht:SONDE,nur:SONDE},
  system_erkunden:{gruppe:'aufklaerung',name:'System erkunden',zweck:'Sonnensystem kartieren (Pflicht vor Ausspähen)',ziel:['leer','fremd','verbuendet','truemmer','guertel'],braucht:SONDE,nur:SONDE},
  kolonisieren:{gruppe:'ausbreitung',name:'Kolonisieren',zweck:'Freien Platz besiedeln',ziel:['leer'],braucht:KOLONIE,fracht:true,einweg:true},
  expedition:{gruppe:'ausbreitung',name:'Expedition',zweck:'Reise ins Unbekannte',ziel:['guertel'],pseudo:true},
  // Terraforming (Karl 24.10.2026): Terraformer formen Klima und Kruste eines eigenen Planeten oder freien Platzes; werden verbraucht.
  terraformen:{gruppe:'ausbreitung',name:'Terraformen',zweck:'Klima ans Volk anpassen, bis +40 Felder; Schiffe werden verbraucht',ziel:['eigen','leer'],braucht:['terraformer'],nur:['terraformer'],einweg:true},
  recyceln:{gruppe:'rohstoffe',name:'Recyceln',zweck:'Trümmerfeld einsammeln',ziel:['truemmer'],braucht:['recycler','bergungsschiff']},
  abbau:{gruppe:'rohstoffe',name:'Abbau',zweck:'Asteroidengürtel abbauen',ziel:['guertel'],braucht:['bergbauschiff'],dauer:true},
  rakete:{gruppe:'kampf',name:'Raketenschlag',zweck:'Raketen aus dem Silo',ziel:['fremd'],feindlich:true,pseudo:true}
 };
 for(const [k,a] of Object.entries(A)){a.key=k;a.farbe=G[a.gruppe].farbe;}
 function info(k){return A[k]||{key:k,gruppe:'fracht',name:k,zweck:'',ziel:[],farbe:'#9aa7bd'};}
 function svg(k,size=18){const p=P[k]||P[info(k).key]||P.transport;return `<svg class="auftrag-svg" viewBox="0 0 24 24" width="${size}" height="${size}" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true" focusable="false">${p}</svg>`;}
 const e=s=>String(s??'').replace(/[&<>"']/g,c=>({'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;',"'":'&#39;'}[c]));
 // Kleines rundes Abzeichen mit Symbol in Gruppenfarbe; optional mit Text.
 function badge(k,{text=false,size=18,title=null,symbol=null}={}){const a=info(k);return `<span class="auftrag-badge auftrag-${e(a.gruppe)}${symbol?' auftrag-s-'+e(symbol):''}" title="${e(title||a.name+(a.zweck?' · '+a.zweck:''))}">${svg(symbol||k,Math.round(size*.72))}${text?`<b>${e(a.name)}</b>`:''}</span>`;}
 function legende(){return `<div class="auftrag-legende">${Object.entries(G).map(([g,v])=>`<div><span class="auftrag-punkt auftrag-${e(g)}"></span><b>${e(v.name)}</b> ${Object.values(A).filter(a=>a.gruppe===g).map(a=>badge(a.key,{size:20})).join('')}</div>`).join('')}</div>`;}
 // Farbklasse statt Inline-Stil: die Spiel-CSP (style-src 'self') sperrt style-Attribute.
 const klasse=k=>'auftrag-'+info(k).gruppe;
 window.Auftraege={gruppen:G,liste:A,info,svg,badge,legende,klasse,feindlich:k=>!!info(k).feindlich};
})();
