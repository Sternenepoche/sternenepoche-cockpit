// Einsatzplanung (Karl, 24.10.2026): „Jeder Auftrag – Stationieren, Bombardieren, Invasion, Kampfkolonisieren, Ausspähen,
// Blockade, Recyceln, Abbau, Kolonisieren, Mond zerstören, Saven, Bodenhilfe … – braucht eine echt gut durchdachte UI/UX mit
// Voreinstellungen und cleveren Funktionen. Von der Schnellsuche bekannter Systeme und Spieler über Schritt für Schritt,
// grafisch und didaktisch; man sieht immer genau, was für diese Aktion zählt und welche Vorbedingungen es gibt; nicht so viel
// auf einen Blick, gute Verschachtelung.“
// Ein Blatt für alle Aufträge: ① Ziel (Schnellsuche) ② Auftrag (passende zuerst, Grund bei den anderen) ③ Flotte (nur was am
// Startort steht, Vorlagen je Auftrag, Regelkarte „So läuft es ab“ + Vorbedingungen) ④ Ladung/Dauer (nur wenn der Auftrag es
// braucht) ⑤ Start (Tempo mit Ankunft, Zusammenfassung, Kernprüfung, Bestätigung bei Unumkehrbarem).
// Verbindlich prüft immer der Kern; die Regeltexte folgen docs/REGELTEXT.md (Missionen, Spionage, Blockade, Invasion v1).
// Nicht-modal (Android-Overlay im Shadow DOM bleibt bedienbar), CSP: keine style-Attribute, Breiten per JS.
// Einstiege: window.Einsatz.oeffnen({ziel,mission,zielart,start,startart}), jedes pickFleet(ziel,mission) der unterstützten
// Aufträge, Kolonisierung.oeffnen und Einsatzplanung.oeffnen. Verband, Ankunft planen und koppeln stehen im Start-Schritt; das
// klassische Flottenformular bleibt nur für Raketenschlag, Expedition und Flottenspionage (IMMER_AUSSEN).
(function(){
 'use strict';
 const el=id=>document.getElementById(id);
 const h=s=>typeof esc==='function'?esc(s):String(s??'').replace(/[&<>"']/g,c=>({'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;',"'":'&#39;'}[c]));
 const zahl=n=>typeof fmt==='function'?fmt(Math.round(Number(n)||0)):Math.round(Number(n)||0).toLocaleString('de-DE');
 const name=k=>typeof named==='function'?named(k):String(k);
 const V=()=>typeof view!=='undefined'?view:null;
 const R=()=>typeof rules!=='undefined'?rules:null;
 const W=()=>typeof world!=='undefined'?world:null;
 const A=()=>window.Auftraege;
 const eintrag=k=>R()?.catalog?.einheiten?.[k]||{};
 const regel=k=>eintrag(k).regel||{};
 const dauer=s=>typeof GameTime!=='undefined'&&GameTime.duration?GameTime.duration(Math.max(0,Math.round(s))):Math.round(s/60)+' min';
 const KOORD=/^\d{1,2}:\d{1,3}:\d{1,2}$/,M=1000,REST_EINWOHNER=1000,PLAETZE_JE_TRUPPENTRANSPORTER=100;
 const VOELKER=['aurelianer','krath','veyari','syntheten'];
 const QUALI=[{k:'bewohner',n:'Bewohner',e:'Minen, Farmen, Kraftwerke'},{k:'fachkraefte',n:'Fachkräfte',e:'Werften, Industrie, Ausbildung'},{k:'forscher',n:'Forscher',e:'Forschungspunkte'}];
 const GUETER=['erz','kristall','deuterium','nahrung','legierung','elektronik','konsumgut','xenokristall','habitatmodul','antriebskern'];
 const STUFE=['','I','II','III','IV','V','VI'];
 const planetName=k=>window.Planetenname?.text?Planetenname.text(k):'['+k+']';

 // ---------------------------------------------------------------- Schiffsgruppen
 const SEKTION={sonde:['spionagesonde'],kolonie:['kolonieschiff','grosses_kolonieschiff'],bergbau:['bergbauschiff'],bergung:['recycler','bergungsschiff'],
  truppen:['truppentransporter'],terraform:['terraformer'],fracht:['kleiner_transporter','grosser_transporter','schnellfrachter','schwerlastfrachter','handelsfrachter'],
  weltenkruemmer:['weltentruemmer']};
 const ZIVIL=new Set([...Object.values(SEKTION).flat().filter(k=>k!=='weltentruemmer'),'pathfinder','solarsonde']);
 const plaetzeJe=k=>Number(eintrag(k).taktik?.personenplaetze)||0;
 const istBomber=k=>(eintrag(k).taktik?.klasse==='bomber')||['bomber','strategischer_bomber'].includes(k);
 const istKampf=k=>!ZIVIL.has(k)&&(Number(regel(k).angriff)||0)>0;
 function sektion(k){
  if(istBomber(k))return 'bomber';
  for(const [s,l] of Object.entries(SEKTION))if(l.includes(k))return s;
  return istKampf(k)?'kampf':'zivil';
 }
 const SEKTION_TITEL={sonde:'Spionagesonden',kolonie:'Kolonieschiff',bergbau:'Bergbauschiffe',bergung:'Recycler & Bergungsschiffe',truppen:'Truppentransporter',
  terraform:'Terraformer',fracht:'Frachter',weltenkruemmer:'Weltenkrümmer',bomber:'Bomber',kampf:'Kampfschiffe',zivil:'Zivile Schiffe'};

 // ---------------------------------------------------------------- Aufträge: Ablauf, was zählt, Schiffe, Schritte
 // ablauf: Phasen als Zeitleiste; zaehlt: was über Erfolg entscheidet (Kern-Regeln, docs/REGELTEXT.md).
 const P={
  system_erkunden:{sektionen:['sonde'],nur:['spionagesonde'],rueck:true,start:'Sonde starten',
   ablauf:['Eine Sonde fliegt ins Sonnensystem','Kartiert alle Plätze, Gürtel, Trümmer und Monde','Kehrt zurück – Systembericht im Funk'],
   zaehlt:['Nur Spionagesonden','1 Sonde genügt','Pflicht vor Ausspähen, Abbau und Recyceln in diesem System']},
  spionage:{sektionen:['sonde'],nur:['spionagesonde'],rueck:true,start:'Sonden starten',
   ablauf:['Sonden fliegen zum Ziel','Scanstärke aus Sonden und Technikvorsprung – Regel unten','Bericht kommt immer an – das Ziel bemerkt den Versuch'],
   zaehlt:['Scanstärke ab 1: Bestände','ab 2: Schiffe','ab 3: Verteidigung','ab 5: Gebäude','ab 7: Forschung & Vollscan','Scanstärke = Sonden + deine − gegnerische Spionagetechnik','viele Schiffe am Ziel schießen Sonden eher ab']},
  angriff:{sektionen:['kampf','bomber','weltenkruemmer','fracht'],rueck:true,lage:true,start:'Angriff starten',
   ablauf:['Anflug – das Ziel sieht dich etwa 30 min vor Ankunft','Raumkampf, höchstens 6 Runden','Sieg: Beute bis 40 % der ungeschützten Güter (Krath 50 %)','30 % der Wracks werden Trümmerfeld – Mondchance 1 % je 2.000 Trümmer, höchstens 40 %','Rückflug mit Beute'],
   zaehlt:['Kampfkraft gegen Flotte und Abwehr','Laderaum begrenzt die Beute','Bunker schützt Güter','Waffen-, Schild-, Panzertechnik je +10 %']},
  blockade:{sektionen:['kampf','bomber','weltenkruemmer'],rueck:true,lage:true,start:'Blockade starten',
   ablauf:['Orbitkampf gewinnen','Die Flotte bleibt im Orbit','Transporte kehren um, Markt wartet, Start nur noch gegen deine Flotte','Rückruf beendet die Blockade'],
   zaehlt:['ab Zivilisationsstufe IV','Unterhalt läuft weiter','Besitzer und Dritte können dich angreifen']},
  bombardieren:{sektionen:['bomber','kampf'],rueck:true,lage:true,start:'Bombardement starten',
   ablauf:['Orbitkampf gegen Schiffe, Abwehr und Schilde','Erste Bombenrunde sofort, dann alle 15 Spielminuten','Alle Gebäude verlieren gleich viel Integrität – nie unter 30 %','Danach Landung mit Kampfkolonisieren'],
   zaehlt:['Nur Bomber wirken: 100 Bombenlast, Strategischer Bomber 150','Strukturpunkte: 700 je Gebäudestufe','Neue Abwehr unterbricht; keine Reparaturen unter Beschuss']},
  kampfkolonisieren:{sektionen:['kolonie','truppen','bomber','kampf'],rueck:true,lage:true,details:'truppen',detailTitel:'Truppen',bestaetigen:true,start:'Eroberung starten',
   ablauf:['Orbit erobern','Bomben, bis alle Gebäude höchstens 30 % haben','Landung: Kolonieschiff + Bodentruppen','Belagerung 48 Spielstunden, Bodenrunden alle 15 min','Übernahme, wenn kein Verteidiger mehr lebt'],
   zaehlt:['Genau ein Kolonieschiff (klein oder groß) und Truppen in Truppentransportern (100 Plätze je Schiff)','Ursprüngliche Heimatwelten sind geschützt','Ein freier Kolonieplatz bei der Übernahme','Garnison, Miliz und Verbündete verteidigen']},
  invasion:{sektionen:['truppen','kampf'],rueck:true,lage:true,details:'truppen',detailTitel:'Truppen',start:'Invasion starten',
   ablauf:['Orbitkampf','Bodentruppen landen','Übernahme über niedrige Stabilität (ältere Welten)'],zaehlt:['Truppentransporter','Heimatwelten sind geschützt']},
  transport:{sektionen:['fracht','truppen'],rueck:true,details:'fracht',detailTitel:'Ladung',start:'Transport starten',
   ablauf:['Flug zum Ziel','Fracht, Personen und Truppen werden abgeladen','Die Schiffe kehren leer zurück'],
   zaehlt:['Laderaum und Personenplätze','Fremde Ziele: Geschenk (wird protokolliert)','Blockierte Ziele: der Transport kehrt um']},
  stationieren:{sektionen:['kampf','bomber','fracht','truppen','zivil'],alleOffen:true,details:'fracht',detailTitel:'Ladung',start:'Verlegen',
   ablauf:['Flug zu deinem Planeten oder Mond','Schiffe und Ladung bleiben dort'],zaehlt:['Nur eigene Ziele','Einweg – kein Rückflugtreibstoff']},
  halten:{sektionen:['kampf','bomber'],rueck:true,dauer:{min:1,max:()=>R()?.catalog?.flug?.halten_max_stunden||168,std:24,schritte:[6,12,24,48,96,168]},start:'Halten starten',
   ablauf:['Flug zum Verbündeten','Steht im Orbit und kämpft bei Angriffen mit','Kehrt nach der Haltedauer zurück'],zaehlt:['Allianz oder Verteidigungsbündnis','Haltedauer 1–168 Spielstunden']},
  verstaerken:{sektionen:['truppen','kampf'],rueck:true,details:'truppen',detailTitel:'Truppen',dauer:{min:1,max:()=>R()?.catalog?.flug?.halten_max_stunden||168,std:48,schritte:[12,24,48,96,168]},start:'Bodenhilfe starten',
   ablauf:['Der Truppenkonvoi landet ohne Gefecht','Deine Truppen verteidigen mit (wird zu Halten)','Sie bleiben deine Truppen und kehren nach der Belagerung heim'],
   zaehlt:['Truppentransporter: 100 Plätze je Schiff','Nur bei Verbündeten']},
  saven:{sektionen:['kampf','bomber','fracht','bergung','bergbau','truppen','zivil','kolonie'],alleOffen:true,rueck:true,details:'fracht',detailTitel:'Ladung & Wartezeit',dauer:{min:0,max:()=>72,std:0,schritte:[0,1,3,6,12,24,48,72]},start:'Flotte retten',
   ablauf:['Flotte und Fracht verlassen den Planeten','Warten am Ziel 0–72 Spielstunden','Kehren automatisch zurück'],
   zaehlt:['Was fliegt, kann am Planeten nicht zerstört oder geplündert werden','Langsames Tempo = längere Abwesenheit, weniger Treibstoff','Ziel: eigener Planet, freier Platz, Gürtel oder Trümmerfeld']},
  recyceln:{sektionen:['bergung','fracht'],rueck:true,start:'Bergung starten',
   ablauf:['Flug zum Trümmerfeld','Recycler und Bergungsschiffe sammeln Erz und Kristall','Freie Personenplätze retten Überlebende','Rückflug mit Ladung'],
   zaehlt:['Nur der Laderaum von Recyclern und Bergungsschiffen zählt','Personenplätze von Transportern und Bergungsschiffen','Das System muss kartiert sein']},
  abbau:{sektionen:['bergbau','fracht'],rueck:true,details:'dauer',detailTitel:'Abbaudauer',dauer:{min:1,max:()=>R()?.catalog?.flug?.abbau_max_stunden||48,std:12,schritte:[2,6,12,24,48]},start:'Abbauflug starten',
   ablauf:['Flug zum Gürtel','Förderung je Bergbauschiff und Stunde: 150 Erz + 100 Kristall × Reichtum','Automatische Rückkehr mit Ladung'],
   zaehlt:['Bergbauschiffe (ab Stufe II)','Laderaum begrenzt den Ertrag','Das System muss kartiert sein']},
  kolonisieren:{sektionen:['kolonie','kampf','fracht','truppen'],details:'kolonie',detailTitel:'Siedler & Fracht',bestaetigen:true,start:'Kolonieflug starten',
   ablauf:['Kolonieschiff und Begleitschutz fliegen los','Gründung, wenn der Platz bei Ankunft frei ist','Siedler, Fracht und Truppen bleiben auf der neuen Kolonie'],
   zaehlt:['Genau ein Kolonieschiff – klein oder groß','Ein freier Kolonieplatz bei der Gründung','3.000 Siedler (großes Schiff 6.000), 1.000 bleiben zurück','Mindestens ein bewaffnetes Begleitschiff','Klima für dein Volk']},
  mond_zerstoeren:{sektionen:['weltenkruemmer','kampf'],rueck:true,lage:true,bestaetigen:true,start:'Angriff auf den Mond',
   ablauf:['Weltenkrümmer greifen den Mond an','Erst nach vollständigem Sieg im Orbit','Chance (100 − √d) · √n %','Der Mond zerfällt zu einem Trümmerring; Überlebende sind bergbar'],
   zaehlt:['d = Monddurchmesser in km','n = Zahl der Weltenkrümmer','Risiko: alle Weltenkrümmer verloren mit √d / 2 %']},
  // Flottenübergabe (Karl 24.10.: „wurde vergessen“): Schiffe samt aller Güter, Personen und Truppen verschenken oder verkaufen.
  uebergeben:{sektionen:['fracht','kampf','bomber','zivil','truppen','bergung','bergbau','kolonie','terraform','sonde'],alleOffen:true,details:'fracht',detailTitel:'Ladung',start:'Angebot senden',
   ablauf:['Angebot an den Empfänger senden','Er nimmt im Flottenkommando an (bis 1 Spieltag)','Die Schiffe fliegen mit allem an Bord los','Bei Ankunft gehören Schiffe, Ladung und Personen ihm – Credits fließen dann'],
   zaehlt:['Geschenk oder Verkauf gegen Credits','Bis zur Annahme bleiben die Schiffe bei dir','Laderaum und Personenplätze voll nutzbar','Höchstens 20 offene Angebote']},
  terraformen:{sektionen:['terraform'],nur:['terraformer'],rueck:true,bestaetigen:true,start:'Terraforming starten',
   ablauf:['Terraformer fliegen zum Planeten oder freien Platz','Zuerst das Klima: bis zum Optimum für dein Volk','Dann die Kruste: bis zu +40 Felder','Eingesetzte Terraformer werden verbraucht, der Rest fliegt heim'],
   zaehlt:['Höhere Terraforming-Stufe = weniger Schiffe','Lebensfeindliche Bahnen: weniger Zusatzfelder','Eigener Planet oder freier Platz in einem kartierten System']}
 };
 const UNTERSTUETZT=new Set(Object.keys(P));
 // Invasion v2 (neue Welten): Verteidigungslinien, „Verwüstet“, Landung des Kolonieschiffs, Wiederaufbau.
 // Bombardieren trägt in v2 Bodentruppen (Kern: Landung bei 30 % mit Bombardieren ODER Kampfkolonisieren, HANDBUCH Kap. 30) –
 // bis 25.10.2026 bot das Blatt dafür weder Truppentransporter noch einen Truppen-Schritt an.
 const P_V1={kampfkolonisieren:{ablauf:P.kampfkolonisieren.ablauf,zaehlt:P.kampfkolonisieren.zaehlt},bombardieren:{ablauf:P.bombardieren.ablauf,zaehlt:P.bombardieren.zaehlt,sektionen:P.bombardieren.sektionen,details:undefined,detailTitel:undefined}};
 function invasionTexte(){
  const v2=V()?.invasion_v2===true;
  Object.assign(P.kampfkolonisieren,v2?{
   ablauf:['Orbit erobern: Kampf gegen Flotten, Abwehr und Schilde','Bomben senken alle Gebäude auf 30 % – dann landen die Bodentruppen','4 Verteidigungslinien, alle 12 Spielstunden greift eine Welle die aktuelle Linie an','Rückhalt gebrochen → der Planet ist verwüstet','Das Kolonieschiff landet (freier Kolonieplatz nötig) und übernimmt','Gebäude stehen auf 30 %: reparieren kostet 75 % der Stufenkosten'],
   zaehlt:['Genau ein Kolonieschiff (klein oder groß), Truppen in Truppentransportern (100 Plätze je Schiff)','Viele Rohstoffe zum Wiederaufbau mitnehmen – sie landen mit','Verteidiger verteilen Truppen und Miliz auf die Linien, Neues steht im Rückhalt','Verbündete schließen sich mit Bodenhilfe an','Ursprüngliche Heimatwelten sind geschützt']}:P_V1.kampfkolonisieren);
  Object.assign(P.bombardieren,v2?{
   ablauf:['Orbitkampf gegen Schiffe, Abwehr und Schilde','Bombenrunden alle 15 Spielminuten senken alle Gebäude auf 30 %','Mit Bodentruppen an Bord: Landung, dann Angriffswellen gegen die Verteidigungslinien','Für die Übernahme später ein Kolonieschiff schicken'],
   zaehlt:P_V1.bombardieren.zaehlt,sektionen:['bomber','kampf','truppen'],details:'truppen',detailTitel:'Truppen'}:P_V1.bombardieren);
 }
 // Was die Flotte am Ziel tut und wann sie heimfliegt (Kern flotte.rs/flugkoordination.rs) – Karl 24.10.2026: Hin- und
 // Rückflug „durchdacht auch für Stationieren oder Halten“. kurz: nach der Handlung sofort zurück · dauer: wählbare Stunden ·
 // offen: bis Rückruf bzw. Ende der Belagerung · einweg: bleibt (nur Hinflug-Treibstoff) · uebergabe: gehört dann dem Empfänger.
 const AUFENTHALT={stationieren:'einweg',kolonisieren:'einweg',uebergeben:'uebergabe',halten:'dauer',abbau:'dauer',saven:'dauer',verstaerken:'dauer',
  blockade:'offen',bombardieren:'offen',kampfkolonisieren:'offen',invasion:'offen'};
 const aufenthaltArt=(m=z?.mission)=>AUFENTHALT[m]||'kurz';
 const AM_ZIEL={spionage:'Scan, dann sofort zurück',system_erkunden:'Kartierung, dann sofort zurück',angriff:'Kampf, dann sofort zurück – mit Beute',
  recyceln:'Bergung, dann sofort zurück – mit Ladung',transport:'Abladen, dann leer zurück',mond_zerstoeren:'Angriff auf den Mond, dann zurück',
  terraformen:'Die nötigen Terraformer werden verbraucht, übrige fliegen heim',blockade:'Nach gewonnenem Kampf: hält den Orbit, bis du sie zurückrufst',
  bombardieren:'Nach gewonnenem Orbitkampf: bombt, bis du sie zurückrufst',kampfkolonisieren:'Bleibt bis zur Übernahme (48 h Bodenkampf) oder bis zum Rückruf',
  invasion:'Bleibt bis zur Übernahme oder bis zum Rückruf',stationieren:'Schiffe und Ladung bleiben dort',
  kolonisieren:'Das Kolonieschiff wird zur Kolonie, Begleitschiffe, Fracht und Siedler bleiben dort',uebergeben:'Schiffe, Ladung und Personen gehören bei Ankunft dem Empfänger'};
 const HALT_TITEL={halten:'Haltedauer',abbau:'Abbaudauer',saven:'Wartezeit am Ziel',verstaerken:'Bodenhilfe'};
 // Hintergrund je Auftragsgruppe (vorhandene Spielkunst, je Volk wo es sie gibt) und das typische Schiff je Auftrag.
 const GRUPPENBILD={kampf:['progression','imperium',true],boden:['backgrounds','shipyard'],aufklaerung:['backgrounds','command'],fracht:['backgrounds','market'],
  verlegung:['backgrounds','hangar'],schutz:['backgrounds','command'],rohstoffe:['progression','industrie',true],ausbreitung:['progression','orbit',true]};
 const AUFTRAGSSCHIFF={angriff:'kreuzer',blockade:'zerstoerer',bombardieren:'bomber',kampfkolonisieren:'truppentransporter',invasion:'truppentransporter',
  transport:'grosser_transporter',uebergeben:'handelsfrachter',stationieren:'schlachtschiff',halten:'schlachtschiff',verstaerken:'truppentransporter',
  saven:'grosser_transporter',spionage:'spionagesonde',system_erkunden:'spionagesonde',recyceln:'recycler',abbau:'bergbauschiff',kolonisieren:'kolonieschiff',
  mond_zerstoeren:'weltentruemmer',terraformen:'terraformer'};
 const absolut=u=>{try{return new URL(u,document.baseURI).href;}catch(_){return u;}};
 function gruppenBild(m){const g=m?A()?.info(m)?.gruppe:'',d=GRUPPENBILD[g]||['backgrounds','nebula'];if(typeof art!=='function')return '';try{return absolut(art(d[0],d[1],d[2]?V()?.volk:''));}catch(_){return '';}}
 function schiffBild(m){const k=AUFTRAGSSCHIFF[m];if(!k||typeof art!=='function')return '';try{return absolut(art('ships',k,V()?.volk));}catch(_){return '';}}
 // Monde als Start oder Ziel nur bei diesen Aufträgen (Kern, monde.rs: flotte_senden mit zielart/startart mond).
 const MOND_AUFTRAEGE=new Set(['transport','stationieren','halten','spionage','angriff','blockade','mond_zerstoeren']);
 // Aufträge, deren Pflichtschiff am Startort stehen muss: andere Orte erscheinen nicht als Start.
 const PFLICHT={kolonisieren:['kolonieschiff','grosses_kolonieschiff'],kampfkolonisieren:['kolonieschiff','grosses_kolonieschiff'],spionage:['spionagesonde'],system_erkunden:['spionagesonde'],
  abbau:['bergbauschiff'],recyceln:['recycler','bergungsschiff'],terraformen:['terraformer'],mond_zerstoeren:['weltentruemmer'],bombardieren:['bomber','strategischer_bomber'],verstaerken:['truppentransporter']};

 // ---------------------------------------------------------------- Zustand
 let z=null,uhr=0,lauf=0,zeichenUhr=0;
 const neu=o=>({schritt:'ziel',ziel:'',zielart:'planet',mission:'',start:'',startart:'planet',schiffe:{},ladung:{},personen:VOELKER.map(()=>[0,0,0]),truppen:Array(12).fill(0),
  mischung:[0,0,0],mischungArt:'bewohner',volk:V()?.volk,frachtArt:'',tempo:1,ruecktempo:1,stunden:null,tfArt:'voll',suche:'',flug:null,plan:null,bedarf:null,tf:null,sim:null,
  bestaetigt:false,offen:{},hinweis:'',fehler:'',senden:false,vorlage:'',ankunftArt:'sofort',ankunftZeit:null,kopplung:{anker:'',versatz:5},verband:'',verkauf:false,preis:0,info:'',tempiErsatz:null,...o});

 // ---------------------------------------------------------------- Daten aus der Sicht
 const planeten=()=>V()?.planeten||[];
 const eigeneMonde=()=>V()?.monde?.eigene||[];
 // 0.4.8 (Karl 24.10.2026): gestrandete eigene Flotten sind Startort (Treibstoff aus ihrer Ladung) und Ziel (Versorgen,
 // Angriff auf fremde). Von dort: Stationieren, Transport, Halten, Spionage, Angriff, Übergabe – nach einem Angriff von dort
 // nur noch Heimflug oder Übergabe.
 const orbitWelt=()=>V()?.orbitflotten===true;
 const gestrandeteFlotten=()=>(V()?.flotten||[]).filter(f=>f.gestrandet);
 const gestrandeteFlotte=id=>gestrandeteFlotten().find(f=>String(f.flotte)===String(id))||null;
 // Alle bekannten gestrandeten Flotten (eigene und in erfassten Systemen), Kern: view.orbitflotten_bekannt.
 const bekannteOrbitflotten=()=>V()?.orbitflotten_bekannt||[];
 // Invasion v2: verwüsteter Planet (Galaxie/Atlas oder eigene Invasionsliste); landen darf nur die Besatzerseite.
 const verwuestung=k=>{const i=(V()?.invasionen||[]).find(x=>x.planet===k&&x.verwuestet);return {an:!!(i||atlasEintrag(k)?.verwuestet),besatzer:!!i&&['angreifer','angriffshelfer'].includes(i.rolle)};};
 // Startort lesbar: Planet, Mond oder gestrandete Flotte.
 // Monde beim Namen, Planeten wie bisher (Funktionsprüfung 25.10.2026: Mond als Start hieß „☾ Heimatwelt“, Mondziele namenlos).
 const zielTitel=(k,art)=>{if(art!=='mond')return planetName(k);const mo=eigeneMonde().find(x=>x.koord===k)||(V()?.monde?.bekannt||[]).find(x=>x.koord===k);return '☾ '+(mo?.name?`${mo.name} [${k}]`:planetName(k));};
 const startName=()=>{if(z.startart==='flotte'){const f=gestrandeteFlotte(z.start);return f?`Flotte #${f.flotte} bei [${f.ziel}]`:`Flotte #${z.start}`;}if(z.startart==='mond'){const mo=eigeneMonde().find(x=>x.koord===z.start);return '☾ '+(mo?.name?`${mo.name} [${z.start}]`:planetName(z.start));}return planetName(z.start);};
 const ORBIT_AUFTRAEGE=new Set(['stationieren','transport','halten','spionage','angriff','uebergeben']);
 const orbitAuftragErlaubt=(f,m)=>ORBIT_AUFTRAEGE.has(m)&&(!f?.gestrandet?.nur_heim||['stationieren','uebergeben'].includes(m));
 function startOrt(){
  if(!z?.start)return null;
  if(z.startart==='flotte'){const f=gestrandeteFlotte(z.start);return f?{koord:f.ziel,schiffe:f.schiffe||{},bestand:f.ladung||{},vomOrbit:true,flotte:f}:null;}
  if(z.startart==='mond'){const m=eigeneMonde().find(x=>x.koord===z.start);if(m)return {...m,schiffe:m.schiffe||{},bestand:m.bestand||m.lager||{},vomMond:true};}
  return planeten().find(p=>p.koord===z.start)||null;
 }
 const da=k=>Math.max(0,Number(startOrt()?.schiffe?.[k])||0);
 function personenAm(p,volk){const g=p?.personengruppen?.gruppen?.[VOELKER.indexOf(volk)]||[0,0,0];return [0,1,2].map(q=>Math.floor((Number(g[q])||0)/M));}
 const bestand=g=>Math.floor(Number(startOrt()?.bestand?.[g])||0);
 const truppenDaten=()=>startOrt()?.bodentruppen||null;
 const atlas=()=>V()?.atlas||[];
 const systemVon=k=>String(k||'').split(':').slice(0,2).join(':');
 const systemKartiert=k=>atlas().some(a=>systemVon(a.koord)===systemVon(k))||(V()?.systemberichte||[]).some(b=>systemVon(b.system||b.ziel||b.koord)===systemVon(k));
 // Mondziele: Mondspionage liegt in view.monde.spionageberichte (Einsatzprobe 25.10.2026), Planeten in view.berichte.
 const bericht=(k,art)=>((art==='mond'?V()?.monde?.spionageberichte:V()?.berichte)||[]).filter(b=>b.ziel===k).sort((a,b)=>(b.zeit||0)-(a.zeit||0))[0]||null;
 const atlasEintrag=k=>atlas().find(a=>a.koord===k)||null;
 const truemmerAn=k=>(V()?.truemmer||[]).find(t=>t.koord===k)||null;
 const bergbauziel=k=>(V()?.bergbauziele||[]).find(b=>b.koord===k)||null;
 const eigenerPlanet=k=>planeten().find(p=>p.koord===k)||null;
 // Scanstärke = Sonden + eigene − gegnerische Spionagetechnik (Kern flotte.rs spionieren). Den Vorsprung kennt ein früherer
 // Bericht (Scanstärke − Sonden); ohne ihn sagt der Text, dass er hinzukommt (Funktionsprüfung 25.10.2026: „bis Gebäude“
 // stand, der Kern lieferte mit Vorsprung 4 Forschung und Vollscan).
 const SCAN=[[7,'Forschung & Vollscan'],[5,'bis Gebäude'],[3,'bis Verteidigung'],[2,'bis Schiffe'],[1,'Bestände']];
 function scanText(n,k){const b=k?bericht(k):null,vs=b?.scan_metadaten?.scanstaerke!=null&&b?.sonden!=null?Number(b.scan_metadaten.scanstaerke)-Number(b.sonden):null;
  if(vs==null){const e=Number(V()?.forschung?.stufen?.spionagetechnik)||0;return `${n} ${n===1?'Sonde':'Sonden'} + deine Spionagetechnik ${e} − gegnerische (noch unbekannt)`;}const s=n+vs;return `Stärke ${s<1?'unter 1':s}: ${(SCAN.find(([a])=>s>=a)||[0,'kaum etwas'])[1]}`;}
 const mondBesitzer=k=>{const m=(V()?.monde?.bekannt||[]).find(x=>x.koord===k&&!x.eigen)||atlasEintrag(k)?.mond;return m&&!m.eigen?m.besitzer||null:null;};
 // Was am Ziel ist: Merkmale, zu denen Aufträge passen (Auftraege.liste[k].ziel).
 function zielMerkmale(k=z?.ziel,art=z?.zielart){
  const out=new Set();if(!KOORD.test(k||''))return out;
  if(art==='mond'){out.add('mond');const m=eigeneMonde().find(x=>x.koord===k);if(m)out.add('eigen');return out;}
  const pos=Number(k.split(':')[2]);
  if(pos===0||bergbauziel(k))out.add('guertel');
  if(truemmerAn(k))out.add('truemmer');
  if(pos===0)return out;
  if(eigenerPlanet(k)){out.add('eigen');return out;}
  // Wie die Galaxietabelle: Besitzer auch aus der Mondsicht; „frei“ nur, wenn der Kern den Platz als unbewohnt kennt –
  // sonst ist er unbekannt, auch in einem kartierten System (Funktionsprüfung 25.10.2026: fremde Heimat galt als „Freier Platz“,
  // Angriff fehlte, Kolonisieren wurde angeboten; Handbuch Kap. 12: ein unaufgeklärter Planet ist nicht automatisch frei).
  const a=atlasEintrag(k),mb=mondBesitzer(k);
  if(a?.bewohnt||a?.spieler){out.add(a.beziehung?.status==='verbuendet'||a.beziehung?.karte_status==='verbuendet'?'verbuendet':'fremd');}
  else if(bericht(k)?.besitzer||mb)out.add('fremd');
  else if(a&&(a.status==='frei'||a.besiedlung_bekannt&&a.bewohnt===false))out.add('leer');
  else out.add('unbekannt');
  return out;
 }
 function zielBeschreibung(k=z?.ziel,art=z?.zielart){
  const m=zielMerkmale(k,art),a=atlasEintrag(k),b=bericht(k,art),t=truemmerAn(k),pos=Number(String(k).split(':')[2]);
  const auf=z?.ziel===k?z.mission:'';
  const art_=m.has('unbekannt')&&!t?'Unbekannter Platz':t&&(auf==='recyceln'||!auf)?'Trümmerfeld':auf==='abbau'&&(pos===0||bergbauziel(k))?(bergbauziel(k)?.art||'Asteroidengürtel'):art==='mond'?'Mond':m.has('eigen')?'Eigener Planet':m.has('verbuendet')?'Verbündeter Planet':m.has('fremd')?'Fremder Planet':pos===0?'Asteroidengürtel':m.has('leer')?'Freier Platz':'Ziel';
  const spieler=a?.spieler||b?.besitzer||mondBesitzer(k)||'';
  const mond=a?.mond||(art==='mond'?eigeneMonde().find(x=>x.koord===k):null);
  return {merkmale:m,art:art_,spieler,heimat:!!(b?.heimat||a?.heimat),mond,bericht:b,truemmer:t,zone:a?.zone||eigenerPlanet(k)?.zone||'',bekannt:!!(a?.bekannt||b||m.has('eigen')),guertel:bergbauziel(k)};
 }

 // ---------------------------------------------------------------- Vorbedingungen je Auftrag (Kern prüft verbindlich)
 const stufeText=n=>'Zivilisationsstufe '+(STUFE[n]||n);
 function bedingungen(m=z?.mission){
  const a=A()?.info(m)||{},v=V()||{},out=[],ziel=z?.ziel||'',d=zielBeschreibung(ziel,z?.zielart),hat=k=>planeten().some(p=>(p.schiffe?.[k]||0)>0)||eigeneMonde().some(x=>(x.schiffe?.[k]||0)>0);
  const zf=z?.zielflotte?(bekannteOrbitflotten().find(o=>Number(o.flotte)===Number(z.zielflotte)&&o.koord===ziel)||null):null;
  if(z?.zielflotte&&m==='angriff'){out.push({ok:!!zf,text:zf?`Ziel: gestrandete Flotte von ${zf.spieler} (${zahl(zf.schiffe)} Schiffe)`:'Die gestrandete Zielflotte ist nicht mehr da'});
   const sys=systemKartiert(ziel);out.push({ok:sys,text:sys?'System bekannt':'System zuerst kartieren',aktion:sys?null:{text:'1 Sonde: System erkunden',sonde:'system_erkunden'}});return out;}
  if(z?.versorgen&&m==='transport'){const f=gestrandeteFlotte(z.versorgen);out.push({ok:!!f,text:f?`Versorgt die gestrandete Flotte ${f.flotte} (an Bord ${zahl(f.gestrandet.deuterium)} Deuterium)`:'Die Flotte ist nicht mehr gestrandet'});if(f&&Number(f.frachtraum?.gesamt??f.flugdetails?.ladekapazitaet??1)<=0)out.push({ok:false,vorrang:true,text:'Diese Flotte hat keinen Laderaum – sie kann keinen Treibstoff aufnehmen'});return out;}
  const vw=verwuestung(ziel),verwuestet=vw.an&&['kolonisieren','kampfkolonisieren'].includes(m);
  // Bodenhilfe zur Invasion eines Verbündeten (Kern angriffshilfe_erlaubt): fremder Planet, aber view.invasionen führt ihn mit
  // Rolle „angriffshelfer“ – Einsatzprobe 25.10.2026: die Lagekarte führte genau dorthin, das Blatt sperrte „Passt nicht zu“.
  const helferInvasion=m==='verstaerken'&&(v.invasionen||[]).some(i=>i.planet===ziel&&i.rolle==='angriffshelfer');
  if(d.merkmale.has('unbekannt')&&!['system_erkunden','spionage'].includes(m)){const sys=(v.flotten||[]).some(f=>systemVon(f.ziel)===systemVon(ziel)&&f.mission==='system_erkunden'&&f.zustand==='hinflug');
   out.push({ok:false,text:'Unbekannter Platz – eine Sonde erkundet das System, dann zeigt sich, wer dort wohnt'+(sys?' (Sonde unterwegs)':''),aktion:sys?null:{text:'1 Sonde: System erkunden',sonde:'system_erkunden'}});return out;}
  // Recyceln ohne Trümmerfeld (Kern flotte.rs erlaubt es): sinnvoll nur hinter einem eigenen Angriff oder Verband auf genau dieses
  // Ziel – „Ankunft koppeln … Recycler 5 s nach dem Einschlag“ (25.10.2026: das Blatt sperrte genau diesen Ablauf).
  const einschlag=m==='recyceln'&&!d.merkmale.has('truemmer')&&!d.merkmale.has('unbekannt')
   &&((v.flotten||[]).some(f=>f.ziel===ziel&&f.zustand==='hinflug'&&A()?.feindlich?.(f.mission))||(v.verbaende||[]).some(b=>b.ziel===ziel));
  if(einschlag){out.push({ok:true,warnung:true,text:'Noch kein Trümmerfeld – dein Angriff ist unterwegs: unter Start „Ankunft koppeln“ wählen, z. B. 5 s nach dem Einschlag'});}
  else{
  const passt=helferInvasion||(verwuestet?vw.besatzer:d.merkmale.has('unbekannt')||(a.ziel||[]).some(x=>d.merkmale.has(x))||(m==='terraformen'&&(d.merkmale.has('eigen')||d.merkmale.has('leer'))));
  out.push({ok:passt,text:m==='recyceln'&&!passt?`Passt nicht zu: ${d.art} – Recyceln braucht ein Trümmerfeld oder einen eigenen Angriff, der dorthin unterwegs ist`:verwuestet?(vw.besatzer?'Verwüsteter Planet: dein Kolonieschiff landet und übernimmt ihn, sobald ein Kolonieplatz frei ist':'Verwüsteter Planet: landen darf nur die Seite, die ihn besetzt hält'):helferInvasion?'Invasion deines Verbündeten: deine Truppen kämpfen auf der Angreiferseite mit':passt?`Ziel passt: ${d.art}`:`Passt nicht zu: ${d.art}`});}
  if(z?.startart==='flotte'){const f=gestrandeteFlotte(z.start);if(f&&!orbitAuftragErlaubt(f,m))out.push({ok:false,text:f.gestrandet.nur_heim?'Nach einem Angriff von hier: nur noch Heimflug (Stationieren) oder Übergabe':'Von einer gestrandeten Flotte aus: Stationieren, Transport, Halten, Spionage, Angriff, Übergabe'});}
  if(a.ab)out.push({ok:(v.stufe||0)>=a.ab,text:(v.stufe||0)>=a.ab?`${stufeText(a.ab)} erreicht`:`ab ${stufeText(a.ab)} (du: ${STUFE[v.stufe]||v.stufe})`});
  const braucht=a.braucht||(m==='terraformen'?['terraformer']:null);
  if(braucht){const ok=braucht.some(hat);
   // Fehlt das Schiff, sagen wann es baubar ist (Werte der Welt, z. B. Bergbauschiff ab Stufe II).
   const wann=braucht.map(k=>{const r=regel(k);return r.ab_stufe?`${name(k)} ab Stufe ${STUFE[r.ab_stufe]||r.ab_stufe}${r.werft?', Werft '+r.werft:''}`:name(k);}).join(' oder ');
   out.push({ok,text:ok?`${braucht.map(name).join(' oder ')} vorhanden`:`Es fehlt: ${wann}`,aktion:ok?null:{text:'Zur Werft',go:'werft'}});}
  // Aufklärungspflicht des Kerns (flotte.rs): System kartiert für Spionage, Abbau, Recyceln; jedes fremde oder freie Ziel
  // außer Erkunden/Spionage/Saven/Abbau/Recyceln zusätzlich mit eigener Sonde ausgespäht (auch freie Plätze vor dem Kolonisieren).
  // Monde (monde.rs: nur System erfasst) und Terraformen (terraform.rs: nur System erfasst) brauchen keinen Planetenbericht –
  // Einsatzprobe 25.10.2026: „Mond zerstören“ blieb sonst endlos bei „Ziel noch nicht ausgespäht“.
  const mondZiel=z?.zielart==='mond';
  const kenntnisPflicht=!mondZiel&&m!=='terraformen'&&!d.merkmale.has('eigen')&&!['system_erkunden','spionage','flotten_spionage','saven','abbau','recyceln'].includes(m)&&!(['halten','verstaerken'].includes(m)&&d.merkmale.has('verbuendet'));
  const sonde=(v.flotten||[]).find(f=>f.ziel===ziel&&['spionage','system_erkunden'].includes(f.mission)&&f.zustand==='hinflug');
  const unterwegs=sonde?` – Sonde unterwegs, Ankunft in ${dauer(Math.max(0,(sonde.ankunft_sekunden||0)-(v.sekunden||0)))}`:'';
  if(['spionage','abbau','recyceln','terraformen'].includes(m)||a.aufklaerung||kenntnisPflicht||(mondZiel&&!d.merkmale.has('eigen')&&!d.merkmale.has('verbuendet')&&m!=='spionage')){const ok=systemKartiert(ziel);out.push({ok,text:ok?'System kartiert':'System noch nicht kartiert'+unterwegs,aktion:ok||sonde?null:{text:'1 Sonde: System erkunden',sonde:'system_erkunden'}});}
  if((a.aufklaerung&&!mondZiel&&m!=='terraformen')||kenntnisPflicht){const ok=d.bekannt;const frei=d.merkmale.has('leer');out.push({ok,text:ok?(d.bericht?`${frei?'Erkundet':'Ausgespäht'} vor ${zahl(Math.max(0,((v.sekunden||0)-(d.bericht.zeit||0))/3600))} h`:frei?'Platz erkundet':'Ziel bekannt'):(frei?'Freier Platz noch nicht erkundet':'Ziel noch nicht ausgespäht')+unterwegs,aktion:ok||sonde?null:{text:a.feindlich?`5 Sonden schicken · ${scanText(5,ziel)}`:frei?'Platz erkunden (1 Sonde)':'1 Sonde schicken',sonde:'spionage'}});}
  if(a.keineHeimat&&d.heimat)out.push({ok:false,text:'Ursprüngliche Heimatwelten sind vor Übernahme geschützt'});
  if(['kolonisieren','kampfkolonisieren'].includes(m))out.push(kolonieplatzBedingung(m));
  if(['halten','verstaerken'].includes(m)&&!helferInvasion)out.push({ok:d.merkmale.has('verbuendet'),vorrang:true,text:d.merkmale.has('verbuendet')?'Verbündeter Planet':'Nur bei Verbündeten (Allianz oder Verteidigungsbündnis)'});
  // Funktionsprüfung 25.10.2026: Saven zum eigenen Startort war „bereit“, der Kern lehnte still ab (flotte.rs „andere Zielkoordinate“).
  if(z?.versorgen){const gf=gestrandeteFlotte(z.versorgen);if(gf&&Number(gf.frachtraum?.gesamt??gf.flugdetails?.ladekapazitaet??1)<=0)out.push({ok:false,vorrang:true,text:'Diese Flotte hat keinen Laderaum – sie kann keinen Treibstoff aufnehmen'});}
  if(m==='saven'&&z?.startart!=='flotte'&&ziel===z?.start)out.push({ok:false,vorrang:true,text:'Saven braucht ein anderes Ziel als den Startort'});
  {const bl=['transport','stationieren'].includes(m)?planeten().find(p=>p.koord===ziel)?.blockade:null;if(bl)out.push({ok:true,warnung:true,text:`Ziel blockiert durch ${bl.durch}: Transporte kehren mit der Ladung um, bis die Blockade endet`});}
  if(m==='stationieren')out.push({ok:d.merkmale.has('eigen')&&!(ziel===z?.start&&z?.startart===z?.zielart),text:d.merkmale.has('eigen')?'Eigenes Ziel':'Nur eigene Planeten und Monde'});
  if(m==='mond_zerstoeren')out.push({ok:z?.zielart==='mond',text:z?.zielart==='mond'?'Ziel ist ein Mond':'Ziel muss ein Mond sein'});
  const fp=v.flottenplaetze;if(fp)out.push({ok:fp.belegt<fp.gesamt,text:fp.belegt<fp.gesamt?`Flottenplätze: ${fp.gesamt-fp.belegt} frei`:`Alle ${fp.gesamt} Flottenplätze belegt – Computertechnik erhöht die Zahl`});
  return out;
 }
 // Kolonieplätze (Karl 24.10.2026): 2 frei ab Start, je weiterer Platz eine Stufe Terraforming, höchstens 14 Kolonien – ein Platz
 // gilt für Kolonisieren und Kampfkolonisieren. Neue Welten (frei_basis > 0): der Flug darf ohne freien Platz starten, gegründet
 // bzw. übernommen wird nur mit Platz (Kern prüft bei Ankunft/Übernahme) – darum Warnung statt Sperre; an der Höchstgrenze Sperre.
 function kolonieplatzBedingung(m){
  const v=V()||{},k=v.kolonien||{},anzahl=k.anzahl||0,erlaubt=k.erlaubt||0,neu=(k.frei_basis||0)>0,max=k.hoechstens||14;
  const unterwegs=(v.flotten||[]).filter(f=>['kolonisieren','kampfkolonisieren'].includes(f.mission)&&['hinflug','im_orbit'].includes(f.zustand)).length;
  const forschung=name(k.forschung||'terraforming'),frei=erlaubt-anzahl-(neu?0:unterwegs);
  // Gezählt wie Karl (25.10.2026): Planeten mit Heimatwelt, also Kolonien + 1, von höchstens 15.
  if(frei>0)return {ok:true,text:`Kolonieplatz frei: ${zahl(frei)} · ${zahl(anzahl+1)} / ${zahl(max+1)} Planeten mit Heimatwelt, ${zahl(erlaubt+1)} freigeschaltet${unterwegs?` · ${unterwegs} Kolonieflug unterwegs – wer zuerst gründet, bekommt den Platz`:''}`};
  if(neu&&anzahl>=max)return {ok:false,text:`Alle ${zahl(max+1)} Planetenplätze belegt (Heimatwelt mitgezählt) – mehr Kolonien erlaubt diese Welt nicht`};
  if(!neu)return {ok:false,text:`Kein Kolonieplatz frei: alle ${zahl(erlaubt+1)} freigeschalteten Planetenplätze belegt (Heimatwelt mitgezählt)${k.naechste_stufe?` – nächster Platz mit ${forschung} ${k.naechste_stufe}`:''}`,aktion:{text:`${forschung} erforschen`,forschung:k.forschung||'astrophysik'}};
  const folge=m==='kampfkolonisieren'?'ohne sie bei der Übernahme kein Besitzerwechsel, die Flotte hält den Orbit':'ohne sie bei Ankunft keine Gründung, die Flotte kehrt um';
  return {ok:true,warnung:true,text:`Kein freier Kolonieplatz (${zahl(anzahl+1)} von ${zahl(erlaubt+1)} freigeschalteten Planetenplätzen belegt, Heimatwelt mitgezählt): ${forschung} Stufe ${k.naechste_stufe} gibt den nächsten frei – ${folge}`,aktion:{text:`${forschung} erforschen`,forschung:k.forschung||'terraforming'}};
 }
 // Ein Wort je Lage (Karl 24.10.): Sonden auf einen unbewohnten Platz „erkunden“ ihn, bewohnte Planeten werden „ausgespäht“.
 function auftragName(m=z?.mission){if(m==='spionage'&&zielMerkmale().has('leer'))return 'Platz erkunden';return A()?.info(m)?.name||m;}
 function bereitFuer(m){return bedingungen(m).every(b=>b.ok);}

 // ---------------------------------------------------------------- Kennzahlen der gewählten Flotte
 const gewaehlt=()=>Object.entries(z.schiffe).filter(([,n])=>n>0);
 const schiffSumme=()=>gewaehlt().reduce((s,[,n])=>s+n,0);
 // Recyceln birgt nur mit Recyclern und Bergungsschiffen (Kern flotte.rs); Frachter tragen nichts aus dem Feld (Funktionsprüfung 25.10.2026).
 function laderaum(){if(z.mission==='recyceln')return z.flug?.bergungskapazitaet??gewaehlt().filter(([k])=>['recycler','bergungsschiff'].includes(k)).reduce((s,[k,n])=>s+(Number(regel(k).ladung)||0)*n,0);return z.flug?.ladekapazitaet??gewaehlt().reduce((s,[k,n])=>s+(Number(regel(k).ladung)||0)*n,0);}
 // Wirksamer Laderaum je Schiff (Volk, Logistik) aus der Kern-Flugrechnung, sonst Grundwert.
 function ladungJe(k){const basis=gewaehlt().reduce((s,[x,n])=>s+(Number(regel(x).ladung)||0)*n,0),f=z.flug?.ladekapazitaet&&basis?z.flug.ladekapazitaet/basis:null;return f?{wert:(Number(regel(k).ladung)||0)*f,grund:Number(regel(k).ladung)||0}:null;}
 const fracht=()=>Object.values(z.ladung).reduce((s,n)=>s+(Number(n)||0),0);
 const personenGewaehlt=()=>z.personen.flat().reduce((s,n)=>s+(Number(n)||0),0);
 const personenplaetze=()=>Number(z.flug?.personenplaetze)||0;
 function truppenPlaetze(){const d=truppenDaten();return z.truppen.reduce((s,n,i)=>s+(Number(n)||0)*(Number(d?.truppenplaetze?.[i])||1),0);}
 const truppenKapazitaet=()=>(z.schiffe.truppentransporter||0)*PLAETZE_JE_TRUPPENTRANSPORTER;
 // Angriff mit eigener Technik (view.einheiten_kosten[].werte = wirksame Werte) – dieselbe Größe wie kampfkraft.angriff der
 // Flottenansicht (Funktionsprüfung 25.10.2026: Blatt 355.000, Kern 639.000 für dieselbe Flotte).
 const kampfwert=map=>Object.entries(map||{}).reduce((s,[k,n])=>s+(Number(V()?.einheiten_kosten?.find(q=>q.einheit===k)?.werte?.angriff??regel(k).angriff)||0)*(Number(n)||0),0);
 const treibstoff=()=>Math.ceil(Number(z.flug?.treibstoff_je_strecke_milli??(z.flug?.treibstoff_je_strecke||0)*M)/M);
 const tempiDa=()=>Array.isArray(z.flug?.tempi)&&z.flug.tempi.length>0;
 function tempoZeile(g){if(!tempiDa())return null;return z.flug.tempi.find(t=>Math.abs(Number(t.geschwindigkeit)-g)<1e-6)||null;}
 function hinflugSekunden(){const t=tempoZeile(z.tempo);return t?Number(t.dauer_sekunden):z.tempiErsatz?Number(z.tempiErsatz.find(x=>Math.abs(x.geschwindigkeit-z.tempo)<1e-6)?.dauer_sekunden):z.flugTempo===z.tempo?Number(z.flug?.dauer_sekunden):null;}
 const kolonieTyp=()=>P.kolonisieren.sektionen&&['kolonieschiff','grosses_kolonieschiff'].find(k=>(z.schiffe[k]||0)>0)||null;
 const siedlerPlaetze=()=>Number(z.bedarf?.siedler)||(kolonieTyp()==='grosses_kolonieschiff'?6000:3000);
 const ohneGruppen=()=>!startOrt()?.personengruppen;

 // ---------------------------------------------------------------- Vorlagen (ein Tipp, dann feinjustieren)
 // Weltenkrümmer fliegen nur Angriff, Mondzerstörung, Transport oder Stationieren (monde.rs).
 function erlaubt(k){const p=P[z.mission];if(!p)return true;if(k==='weltentruemmer'&&!['angriff','mond_zerstoeren','transport','stationieren'].includes(z.mission))return false;if(p.nur)return p.nur.includes(k);if(z.mission==='abbau')return ['bergbau','fracht','bergung'].includes(sektion(k));if(z.mission==='recyceln')return ['bergung','fracht'].includes(sektion(k));return k!=='solarsonde';}
 function alle(filter){const out={};for(const [k,n] of Object.entries(startOrt()?.schiffe||{}))if(n>0&&erlaubt(k)&&filter(k))out[k]=n;return out;}
 function frachterFuer(menge,typen=SEKTION.fracht,basis={}){
  const out={...basis};let rest=menge;const basisSchiffe=gewaehlt().filter(([k])=>z.mission!=='recyceln'||['recycler','bergungsschiff'].includes(k)),faktor=laderaum()>0&&basisSchiffe.length?laderaum()/Math.max(1,basisSchiffe.reduce((s,[k,n])=>s+(Number(regel(k).ladung)||0)*n,0)):1;
  for(const k of typen.filter(k=>da(k)>0).sort((a,b)=>(regel(b).ladung||0)-(regel(a).ladung||0))){if(rest<=0)break;const je=(Number(regel(k).ladung)||0)*faktor;if(je<=0)continue;const frei=da(k)-(out[k]||0);const n=Math.min(frei,Math.ceil(rest/je));if(n>0){out[k]=(out[k]||0)+n;rest-=n*je;}}
  return out;
 }
 function beute(){const b=bericht(z.ziel,z.zielart);return b&&window.Flottenfuehrung?.beute?Flottenfuehrung.beute(b):null;}
 function vorlagen(){
  const m=z.mission,out=[],p=startOrt();if(!p)return out;
  const add=(id,text,schiffe,zusatz='')=>{const summe=Object.values(schiffe).reduce((s,n)=>s+n,0);if(summe>0)out.push({id,text,schiffe,zusatz});};
  if(m==='spionage'){for(const n of [1,2,3,5,7])if(da('spionagesonde')>=n)add('s'+n,`${n} ${n===1?'Sonde':'Sonden'}`,{spionagesonde:n},scanText(n,z?.ziel));}
  else if(m==='system_erkunden')add('s1','1 Sonde',{spionagesonde:1},'kartiert das System');
  else if(m==='abbau')add('alle','Alle Bergbauschiffe',alle(k=>k==='bergbauschiff'),`${zahl(da('bergbauschiff'))} Stück`);
  else if(m==='recyceln'){const t=truemmerAn(z.ziel),menge=t?(t.erz||0)+(t.kristall||0):0;
   if(menge){const s=frachterFuer(menge,['recycler','bergungsschiff']);add('passend','Passend fürs Trümmerfeld',s,`${zahl(menge)} Erz + Kristall`);}
   add('alle','Alle Recycler',alle(k=>sektion(k)==='bergung'));
   const leute=Number(t?.ueberlebende)||0;if(leute){const basis=out[0]?.schiffe||{};const s={...basis};let rest=leute;for(const k of ['bergungsschiff','handelsfrachter','grosser_transporter'].filter(k=>plaetzeJe(k)>0).sort((a,b)=>plaetzeJe(b)-plaetzeJe(a))){const je=plaetzeJe(k);const n=Math.min(da(k)-(s[k]||0),Math.ceil(rest/je));if(n>0){s[k]=(s[k]||0)+n;rest-=n*je;}if(rest<=0)break;}add('rettung','+ Plätze für Überlebende',s,`${zahl(leute)} warten`);}}
  else if(m==='kolonisieren'){const typ=da('kolonieschiff')?'kolonieschiff':da('grosses_kolonieschiff')?'grosses_kolonieschiff':null;if(typ){const s={[typ]:1};const wert=k=>Object.values(regel(k).kosten||{}).reduce((a,b)=>a+(Number(b)||0),0);const schutz=Object.keys(p.schiffe).filter(k=>p.schiffe[k]>0&&istKampf(k)&&!istBomber(k)&&k!=='weltentruemmer').sort((a,b)=>wert(a)-wert(b))[0];if(schutz)s[schutz]=1;add('kolonie',`${name(typ)} + Begleitschutz`,s,schutz?`1 × ${name(schutz)}`:'');}}
  else if(m==='terraformen'){const q=z.tf;if(q){const k=Number(q.schiffe_klima)||0,f=Number(q.schiffe_felder)||0;if(k)add('klima','Nur Klima anpassen',{terraformer:Math.min(da('terraformer'),k)},`${zahl(k)} Terraformer`);if(k+f){const n=Math.min(da('terraformer'),k+f);add('voll',n<k+f?`Klima + Felder – nur ${zahl(n)} von ${zahl(k+f)}`:'Klima + Felder (komplett)',{terraformer:n},n<k+f?'nicht komplett':`${zahl(k+f)} Terraformer`);}}add('alle','Alle Terraformer',alle(k=>k==='terraformer'),`${zahl(da('terraformer'))} Stück · nur die nötigen werden verbraucht`);}
  else if(m==='mond_zerstoeren')add('alle','Alle Weltenkrümmer',alle(k=>k==='weltentruemmer'),`${zahl(da('weltentruemmer'))} Stück`);
  else if(m==='saven'){add('alles','Alles retten',alle(()=>true),'ganze Flotte');add('kampf','Nur Kampfschiffe',alle(istKampf));}
  else if(m==='uebergeben'){add('frachter','Alle Frachter',alle(k=>sektion(k)==='fracht'),'viel Laderaum');add('kampf','Alle Kampfschiffe',alle(istKampf));add('alles','Ganze Flotte am Startort',alle(()=>true));}
  else if(m==='stationieren'){add('alles','Ganze Flotte',alle(()=>true));add('kampf','Nur Kampfschiffe',alle(istKampf));add('zivil','Nur zivile Schiffe',alle(k=>!istKampf(k)));}
  else if(m==='transport'){const fr=fracht()||0;add('frachter','Alle Frachter',alle(k=>sektion(k)==='fracht'));if(fr)add('passend','Frachter passend zur Ladung',frachterFuer(fr),`${zahl(fr)} Fracht`);}
  else if(m==='bombardieren'){add('bomber','Alle Bomber',alle(istBomber));add('bomber_schutz','Bomber + Kampfflotte',alle(k=>istBomber(k)||istKampf(k)));
   if(V()?.invasion_v2===true&&da('truppentransporter'))add('landung','Bomber + Kampfflotte + Truppentransporter',{...alle(k=>istBomber(k)||istKampf(k)),truppentransporter:da('truppentransporter')},'Landung bei 30 %');}
  else if(m==='kampfkolonisieren'){const typ=da('kolonieschiff')?'kolonieschiff':da('grosses_kolonieschiff')?'grosses_kolonieschiff':null;const s=alle(k=>istKampf(k)||istBomber(k));if(typ)s[typ]=1;if(da('truppentransporter'))s.truppentransporter=da('truppentransporter');add('voll','Kolonieschiff + Truppen + Kampfflotte',s);}
  else if(m==='verstaerken')add('truppen','Alle Truppentransporter',alle(k=>k==='truppentransporter'));
  else{const b=beute();const kampf=alle(k=>istKampf(k)||istBomber(k)||k==='weltentruemmer');add('kampf','Alle Kampfschiffe',kampf,`Angriff ${zahl(kampfwert(kampf))}`);
   if(m==='angriff'&&b?.summe>0){add('beute','Kampfschiffe + Frachter für die Beute',frachterFuer(b.summe,SEKTION.fracht,kampf),`Beute ≈ ${zahl(b.summe)}`);}}
  return out;
 }
 function vorlageAnwenden(id){const v=vorlagen().find(x=>x.id===id);if(!v)return;z.schiffe={...v.schiffe};z.vorlage=id;flugNeu();}
 function standardVorlage(){
  const m=z.mission,liste=vorlagen();if(!liste.length)return;
  const wahl={spionage:da('spionagesonde')>=5?'s5':liste[liste.length-1]?.id,system_erkunden:'s1',abbau:'alle',recyceln:liste.find(v=>v.id==='rettung')?'rettung':liste[0].id,kolonisieren:'kolonie',
   terraformen:z.tfArt==='klima'?'klima':'voll',mond_zerstoeren:'alle',saven:'alles',verstaerken:'truppen'}[m];
  if(wahl&&liste.some(v=>v.id===wahl))vorlageAnwenden(wahl);
 }

 // ---------------------------------------------------------------- Siedler (Kolonisieren)
 function mischungSetzen(art){
  const ab=personenAm(startOrt(),z.volk),pl=siedlerPlaetze();let m=[0,0,0],hinweis='';
  const fuellen=reihe=>{let rest=pl;for(const q of reihe){const n=Math.min(ab[q],rest);m[q]+=n;rest-=n;}return rest;};
  // „Nur Fachkräfte/Forscher“ (Karl 24.10.: Sackgasse „45 von 3.000“): erst die Wunschgruppe, der Rest füllt mit Bewohnern auf.
  if(art==='bewohner')fuellen([0,1,2]);
  else if(art==='fachkraefte'){fuellen([1,0,2]);if(ab[1]<pl)hinweis=`Nur ${zahl(ab[1])} Fachkräfte auf dem Planeten – der Rest fliegt als Bewohner mit.`;}
  else if(art==='forscher'){fuellen([2,0,1]);if(ab[2]<pl)hinweis=`Nur ${zahl(ab[2])} Forscher auf dem Planeten – der Rest fliegt als Bewohner mit.`;}
  else{const summe=ab[0]+ab[1]+ab[2]||1;m=ab.map(n=>Math.floor(n*pl/summe));let rest=pl-m[0]-m[1]-m[2];for(const q of [0,1,2]){const d=Math.min(rest,ab[q]-m[q]);m[q]+=d;rest-=d;}}
  z.mischung=m;z.mischungArt=art;z.siedlerHinweis=hinweis;
 }
 function mischungAendern(q,wert){
  const ab=personenAm(startOrt(),z.volk),pl=siedlerPlaetze(),m=z.mischung.slice();
  m[q]=Math.max(0,Math.min(ab[q],pl,Math.round(wert)));let rest=pl-m[0]-m[1]-m[2];
  for(const o of [0,1,2].filter(x=>x!==q)){if(rest>0){const d=Math.min(rest,ab[o]-m[o]);m[o]+=d;rest-=d;}else if(rest<0){const d=Math.min(-rest,m[o]);m[o]-=d;rest+=d;}}
  z.mischung=m;z.mischungArt='eigen';z.siedlerHinweis='';
 }
 const voelkerAm=p=>VOELKER.filter(v=>personenAm(p,v).reduce((a,b)=>a+b,0)>0);

 // ---------------------------------------------------------------- Fracht
 // Deuterium für den Treibstoff bleibt am Start (hin und – auch bei Transport und Übergabe – zurück).
 // Übergabe ab einer gestrandeten Flotte fliegt einweg (Kern: orbitflotte_anbieten), ohne Reserve.
 const spritFaktor=()=>P[z.mission]?.rueck||(z.mission==='uebergeben'&&z.startart!=='flotte')?2:1;
 const frachtMax=g=>Math.max(0,bestand(g)-(g==='deuterium'?treibstoff()*spritFaktor():0));
 function frachtSetzen(art){
  const lr=laderaum();z.ladung={};z.frachtArt=art;z.gutReihe=null;z.vorbelegt=null;if(art==='leer')return;
  // Transport: Menge zuerst, Frachter danach automatisch (Karl: nicht endlos hin und her zwischen Schiffen und Ladung).
  if(z.mission==='transport'&&(art==='aufbau'||art==='rohstoffe')){
   const ziel=art==='rohstoffe'?{erz:Infinity,kristall:Infinity,deuterium:Infinity}:{erz:22500,kristall:15000,deuterium:5000,nahrung:5000,legierung:1500,elektronik:1000};
   for(const [g,n] of Object.entries(ziel)){const m=Math.min(n,frachtMax(g));if(m>0)z.ladung[g]=Math.floor(m);}
   return;
  }
  const min=g=>Math.ceil(Number(z.bedarf?.fracht?.[g])||0);
  if(z.mission==='kolonisieren')for(const g of GUETER){const n=Math.min(min(g),frachtMax(g));if(n>0)z.ladung[g]=n;}
  if(art==='mindest')return;
  // Verhältnisse: Aufbau (Kolonie) bzw. „wertvollstes zuerst“ für Saven und Transport.
  const anteile=art==='aufbau'||art==='voll'&&z.mission==='kolonisieren'?{erz:0.45,kristall:0.3,deuterium:0.1,nahrung:0.1,legierung:0.03,elektronik:0.02}:null;
  let rest=Math.max(0,lr-fracht());if(art==='aufbau')rest=Math.min(rest,Math.max(20000,lr*0.5));
  if(anteile){for(let r=0;r<3&&rest>0;r++){const offen=Object.keys(anteile).filter(g=>frachtMax(g)>(z.ladung[g]||0));const s=offen.reduce((a,g)=>a+anteile[g],0);if(!s)break;let verteilt=0;for(const g of offen){const n=Math.min(Math.floor(rest*anteile[g]/s),frachtMax(g)-(z.ladung[g]||0));if(n>0){z.ladung[g]=(z.ladung[g]||0)+n;verteilt+=n;}}rest-=verteilt;if(!verteilt)break;}return;}
  const reihe=art==='rohstoffe'?['deuterium','kristall','erz']:art==='bauteile'?['antriebskern','habitatmodul','elektronik','legierung']:['xenokristall','antriebskern','habitatmodul','elektronik','legierung','deuterium','kristall','konsumgut','erz','nahrung'];
  for(const g of reihe){if(rest<=0)break;const n=Math.min(rest,frachtMax(g)-(z.ladung[g]||0));if(n>0){z.ladung[g]=(z.ladung[g]||0)+n;rest-=n;}}
 }
 // Versorgen (0.4.8): genau der Treibstoff, den die gestrandete Flotte für den Heimflug zum Startplaneten braucht, plus 10 %.
 async function heimflugFracht(){
  const gf=gestrandeteFlotte(z.versorgen);if(!gf)return;
  const heim=eigenerPlanet(gf.start)?gf.start:z.start;
  const q=await api('/api/tool',{typ:'flugzeit',flotte:gf.flotte,ziel:heim,geschwindigkeit:1}).catch(()=>null);if(!z)return;
  // Mit Tausendsteln rechnen (Funktionsprüfung 25.10.2026: 1,146 wurde als 1 gezeigt, geladen wurden 2).
  const strecke=Number(q?.treibstoff_je_strecke_milli??(Number(q?.treibstoff_je_strecke)||0)*1000)/1000,mitReserve=Math.ceil(strecke*1.1),bord=Number(gf.gestrandet.deuterium)||0;
  const noetig=Math.max(0,mitReserve-bord);
  z.ladung={};z.frachtArt='heimflug';z.heimflugBedarf=q?{noetig,strecke,mitReserve,bord,heim}:null;
  if(noetig>0)z.ladung.deuterium=Math.min(noetig,Math.floor(frachtMax('deuterium')));
 }
 // Kampfkolonisieren (0.4.8): „Wiederaufbau“ lädt die Reparaturkosten aller ausgespähten Gebäude (Kern: wiederaufbau).
 async function wiederaufbauFracht(){
  let q=null,fehler='';try{q=await api('/api/tool',{typ:'wiederaufbau',ziel:z.ziel});}catch(e){fehler=String(e?.message||e);}if(!z)return;
  if(!q?.kosten){frachtSetzen('aufbau');z.wiederaufbau={fehler:fehler||'Ohne Spionagebericht mit Gebäuden: pauschales Aufbaupaket'};return;}
  z.ladung={};z.frachtArt='aufbau';z.wiederaufbau={kosten:q.kosten,gebaeude:q.gebaeude,zeit:q.bericht_zeit};
  for(const [g,n] of Object.entries(q.kosten)){const m=Math.min(Math.ceil(Number(n)||0),Math.floor(frachtMax(g)));if(m>0)z.ladung[g]=m;}
 }
 function frachterErgaenzen(){const fehlt=fracht()-laderaum();if(fehlt<=0)return;z.schiffe=frachterFuer(fehlt,SEKTION.fracht,z.schiffe);flugNeu();}
 // Personenplätze füllen: Bewohner (bzw. erst Fachkräfte und Forscher), 1.000 Einwohner bleiben je Planet zurück.
 function personenFuellen(art){
  const p=startOrt();z.personen=VOELKER.map(()=>[0,0,0]);if(art==='leer'||!p)return;
  let frei=personenplaetze(),rest=Math.max(0,Math.floor(Number(p.bevoelkerung)||0)-REST_EINWOHNER);
  const reihe=art==='fach'?[2,1,0]:[0,1,2];
  for(const v of [V()?.volk,...VOELKER.filter(x=>x!==V()?.volk)]){const i=VOELKER.indexOf(v),ab=personenAm(p,v);if(i<0)continue;
   for(const q of reihe){const n=Math.min(ab[q],frei,rest);if(n>0){z.personen[i][q]+=n;frei-=n;rest-=n;}}}
  z.plan=null;
 }
 function personenplaetzeErgaenzen(){
  let fehlt=personenGewaehlt()-(personenplaetze()||gewaehlt().reduce((s,[k,n])=>s+plaetzeJe(k)*n,0));if(fehlt<=0)return;
  for(const k of Object.keys(startOrt()?.schiffe||{}).filter(k=>plaetzeJe(k)>0).sort((a,b)=>plaetzeJe(b)-plaetzeJe(a))){const frei=da(k)-(z.schiffe[k]||0);if(frei<=0)continue;const n=Math.min(frei,Math.ceil(fehlt/plaetzeJe(k)));z.schiffe[k]=(z.schiffe[k]||0)+n;fehlt-=n*plaetzeJe(k);if(fehlt<=0)break;}
  flugNeu();
 }
 function truppentransporterErgaenzen(){const n=Math.ceil(truppenPlaetze()/PLAETZE_JE_TRUPPENTRANSPORTER);z.schiffe.truppentransporter=Math.min(da('truppentransporter'),Math.max(n,z.schiffe.truppentransporter||0));flugNeu();}

 // ---------------------------------------------------------------- Schritte
 function schritte(){
  const p=P[z.mission];const s=[['ziel','Ziel'],['auftrag','Auftrag'],['flotte','Flotte']];
  if(p?.details&&!(z.startart==='flotte'&&p.details!=='dauer'))s.push(['details',p.detailTitel||'Ladung']);
  s.push(['start','Start']);return s;
 }
 function schrittIndex(id=z.schritt){return schritte().findIndex(([s])=>s===id);}
 function schrittOk(id){
  if(id==='ziel')return KOORD.test(z.ziel);
  if(id==='auftrag')return !!z.mission&&UNTERSTUETZT.has(z.mission);
  if(id==='flotte')return schiffSumme()>0&&!flottenFehler();
  if(id==='details')return !detailFehler();
  return true;
 }
 function flottenFehler(){
  const m=z.mission,p=P[m]||{};if(!startOrt())return 'Startort wählen';if(!schiffSumme())return m==='transport'?'':'Schiffe wählen';
  if(p.nur&&gewaehlt().some(([k])=>!p.nur.includes(k)))return `${A()?.info(m).name} fliegt nur mit ${p.nur.map(name).join(', ')}`;
  const braucht=A()?.info(m)?.braucht||(m==='terraformen'?['terraformer']:null);
  if(braucht&&!braucht.some(k=>(z.schiffe[k]||0)>0))return `Es fehlt: ${braucht.map(name).join(' oder ')}`;
  if(m==='kolonisieren'&&!gewaehlt().some(([k])=>istKampf(k)))return 'Mindestens ein bewaffnetes Begleitschiff mitnehmen';
  if(m==='kampfkolonisieren'&&!kolonieTyp())return 'Kampfkolonisieren braucht ein Kolonieschiff';
  return '';
 }
 function detailFehler(){
  const m=z.mission;
  const vorrang=bedingungen().find(b=>!b.ok&&b.vorrang);if(vorrang)return vorrang.text;
  if(!schiffSumme())return fracht()>0?'Noch keine Schiffe – „Frachter dazunehmen“ tippen':'Noch keine Schiffe – Ladung wählen, dann „Frachter dazunehmen“ tippen';
  if(m==='kolonisieren'){const pl=siedlerPlaetze(),s=z.mischung.reduce((a,b)=>a+b,0),ab=personenAm(startOrt(),z.volk);
   if(!ohneGruppen()&&s!==pl)return `Siedler: ${zahl(s)} von ${zahl(pl)} gewählt`;
   if(!ohneGruppen()&&[0,1,2].some(q=>z.mischung[q]>ab[q]))return 'Zu wenige Personen dieser Gruppe auf dem Startplaneten';
   if((Number(startOrt()?.bevoelkerung)||0)-pl<REST_EINWOHNER)return `Mindestens ${zahl(REST_EINWOHNER)} Einwohner müssen zurückbleiben`;
   const fehlt=GUETER.filter(g=>(Number(z.bedarf?.fracht?.[g])||0)>(Number(z.ladung[g])||0));if(fehlt.length)return `Mindestfracht fehlt: ${fehlt.map(name).join(', ')}`;}
  if(fracht()>laderaum())return `Laderaum zu klein: ${zahl(fracht())} / ${zahl(laderaum())}`;
  if(personenGewaehlt()>0&&z.flug&&personenGewaehlt()>personenplaetze())return `Personenplätze zu wenig: ${zahl(personenGewaehlt())} / ${zahl(personenplaetze())}`;
  if(truppenPlaetze()>truppenKapazitaet())return `Truppen brauchen ${zahl(truppenPlaetze())} Plätze, Truppentransporter tragen ${zahl(truppenKapazitaet())}`;
  if(['kampfkolonisieren','verstaerken'].includes(m)&&!truppenPlaetze())return m==='kampfkolonisieren'?'Ohne Bodentruppen keine Landung – Truppen wählen':'Bodenhilfe braucht Truppen';
  return '';
 }
 function weiterText(){
  const i=schrittIndex(),s=schritte(),naechster=s[i+1];
  if(!naechster)return z.mission==='spionage'&&zielMerkmale().has('leer')?'Sonde starten':P[z.mission]?.start||'Flotte starten';
  return `Weiter: ${naechster[1]}`;
 }
 function sperrgrund(){
  if(z.schritt==='ziel')return KOORD.test(z.ziel)?'':'Ziel wählen oder Koordinate eingeben';
  if(z.schritt==='auftrag')return !z.mission?'Auftrag wählen':((bedingungen().find(b=>!b.ok&&b.vorrang)||bedingungen().find(b=>!b.ok))?.text||'');
  if(z.schritt==='flotte')return flottenFehler()||(bedingungen().find(b=>!b.ok&&b.vorrang)||bedingungen().find(b=>!b.ok))?.text||'';
  if(z.schritt==='details')return detailFehler();
  if(z.schritt==='start'){const f=flottenFehler()||detailFehler();if(f)return f;
   const fp=fahrplan();
   if(fp&&fp.sprit+(Number(z.ladung.deuterium)||0)>(Number(startOrt()?.bestand?.deuterium)||0))return 'Deuterium reicht nicht für Treibstoff und Ladung';
   if(z.ankunftArt==='uhrzeit'&&fp){if(fp.abflug<fp.jetzt-1)return `Zu früh: die Flotte braucht ${zeitText(fp.hin.dauer)} – frühestens ${uhrzeit(fp.jetzt+fp.hin.dauer)}`;if(fp.abflug>fp.jetzt+7*86400)return 'Abflug höchstens 7 Spieltage voraus';}
   if(z.ankunftArt==='kopplung'){if(!z.kopplung.anker)return 'Flotte zum Koppeln wählen';if(fp&&fp.abflug<fp.jetzt-1){const v=z.kopplung.versatz||0,fr=Math.ceil(fp.jetzt-fp.abflug)+v+30;return fp.hin&&z.tempo<1?`Für +${v} s ist der Abflug schon vorbei – schneller fliegen oder frühestens +${fr} s nach der Ankerflotte`:`Für +${v} s ist der Abflug schon vorbei – frühestens +${fr} s nach der Ankerflotte (Abstand erhöhen)`;}}
   if(z.verband&&z.ankunftArt!=='sofort')return 'Für einen Verband sofort starten';
   if(z.verband&&z.verband!=='neu'){const vb=(V()?.verbaende||[]).find(x=>String(x.fuehrung)===String(z.verband)),fpv=fahrplan();if(vb&&fpv&&vb.spaeteste_ankunft_sekunden!=null&&fpv.ankunft>vb.spaeteste_ankunft_sekunden)return 'Zu langsam für diesen Verband (über 6 h Verzögerung) – schneller fliegen oder allein angreifen';if(vb&&fpv&&vb.ankunft_sekunden!=null&&vb.ankunft_sekunden-fpv.ankunft>6*3600)return 'Zu früh für diesen Verband (über 6 h Wartezeit) – langsamer fliegen oder später starten';}
   if(z.mission==='uebergeben'&&z.verkauf&&!(Number(z.preis)>0))return 'Kaufpreis eingeben oder „Geschenk“ wählen';if(P[z.mission]?.bestaetigen&&!z.bestaetigt)return z.mission==='mond_zerstoeren'?'Bitte bestätigen: Mond zerstören ist endgültig':'Bitte bestätigen, dass die Koordinaten stimmen';if(z.plan?.status==='blocked')return z.plan.grund||'Start nicht möglich';return '';}
  return '';
 }

 // ---------------------------------------------------------------- Darstellung: Bausteine
 const badge=(m,o)=>A()?A().badge(m,o):'';
 function balken(cls,teile,gesamt){return `<div class="es-balken ${cls}" role="img" aria-label="${h(teile.map(t=>t.titel).join(', '))}">${teile.map(t=>`<span class="${t.cls}" data-anteil="${gesamt>0?Math.max(0,Math.min(1,t.wert/gesamt)):0}"></span>`).join('')}</div>`;}
 function stepper(art,k,wert,max,label){return `<div class="es-zaehler"><button type="button" class="es-z-knopf" data-es-minus="${art}:${h(k)}" aria-label="${h(label)} weniger">−</button><input type="number" inputmode="numeric" min="0" max="${max}" value="${wert}" data-es-zahl="${art}:${h(k)}" aria-label="${h(label)}"><button type="button" class="es-z-knopf" data-es-plus="${art}:${h(k)}" aria-label="${h(label)} mehr">+</button><button type="button" class="es-z-max" data-es-max="${art}:${h(k)}">Max</button></div>`;}
 const chip=(attr,wert,text,an,zusatz='')=>`<button type="button" class="es-chip" ${attr}="${h(wert)}" aria-pressed="${an?'true':'false'}"><b>${h(text)}</b>${zusatz?`<small>${h(zusatz)}</small>`:''}</button>`;
 function zielBild(k,art){
  if(art==='mond'){const m=atlasEintrag(k)?.mond||eigeneMonde().find(x=>x.koord===k);const url=m?(window.PlanetKugel?.kugelbild?.('moons',m.typ,m.volk||V()?.volk)||''):'';return url?`<img class="es-zielbild" src="${h(url)}" alt="">`:'<span class="es-zielbild es-zielbild-leer" aria-hidden="true"></span>';}
  const p=eigenerPlanet(k);if(p&&typeof planetArt==='function')return `<img class="es-zielbild" src="${h(planetArt(p))}" alt="">`;
  const a=atlasEintrag(k),zone=a?.zone;if(zone&&typeof planetArt==='function'){try{return `<img class="es-zielbild" src="${h(planetArt({koord:k,zone,felder:a.felder}))}" alt="">`;}catch(_){}}
  const pos=Number(String(k).split(':')[2]);return `<span class="es-zielbild es-zielbild-leer${pos===0?' es-zielbild-guertel':''}" aria-hidden="true"></span>`;
 }
 function kopf(){
  const m=z.mission,a=m?A()?.info(m):null,d=KOORD.test(z.ziel)?zielBeschreibung():null,s=schritte(),i=schrittIndex();
  const zfo=z.zielflotte?bekannteOrbitflotten().find(o=>Number(o.flotte)===Number(z.zielflotte))||null:null;
  const gf=z.startart==='flotte'?gestrandeteFlotte(z.start):null,ab=gf?`Flotte #${gf.flotte} · gestrandet bei [${gf.ziel}]`:'';
  return `<div class="es-kopf${a?' auftrag-'+h(a.gruppe):''}"><div class="es-kopf-zeile">${a?badge(m,{size:30}):'<span class="es-kopf-leer" aria-hidden="true"></span>'}<div class="es-kopf-text"><span class="es-eyebrow">${gf?h(ab):a?h(A().gruppen[a.gruppe]?.name||''):'Neuer Einsatz'}</span><h2 id="es-titel">${a?h(auftragName(m)):gf?'Weiterfliegen':'Einsatz planen'}</h2>${d?`<p>${h(zfo?'Gestrandete Flotte von '+zfo.spieler:z.versorgen?'Versorgt deine Flotte #'+z.versorgen:d.art)}${d.spieler&&!zfo&&!z.versorgen?' · '+h(d.spieler):''} · <span class="koord">${h(z.zielart==='mond'?zielTitel(z.ziel,'mond'):planetName(z.ziel))}</span></p>`:gf?`<p>${zahl(gf.gestrandet.deuterium)} Deuterium an Bord · Ziel wählen</p>`:'<p>Ziel und Auftrag wählen</p>'}</div><button type="button" class="es-x" data-es-schliessen aria-label="Einsatz schließen">✕</button></div>
   <ol class="es-schritte" aria-label="Schritte">${s.map(([id,t],n)=>`<li class="${n<i?'fertig':n===i?'aktiv':''}"><button type="button" data-es-schritt="${id}" ${n>i&&!s.slice(0,n).every(([x])=>schrittOk(x))?'disabled':''} aria-current="${n===i?'step':'false'}"><span>${n+1}</span>${h(t)}</button></li>`).join('')}</ol></div>`;
 }

 // ---------------------------------------------------------------- Schritt ① Ziel
 function verlauf(){try{return JSON.parse(localStorage.getItem(verlaufSchluessel())||'[]');}catch(_){return [];}}
 const verlaufSchluessel=()=>'sternenepoche-einsatz:'+(W()?.world_id||'')+':'+(V()?.name||'');
 function merken(){try{const l=verlauf().filter(x=>!(x.ziel===z.ziel&&x.zielart===z.zielart));l.unshift({ziel:z.ziel,zielart:z.zielart,mission:z.mission,t:Date.now()});localStorage.setItem(verlaufSchluessel(),JSON.stringify(l.slice(0,12)));}catch(_){}}
 function flottenZeile(k,o,titel,unter,art){
  const an=z.ziel===k&&(Number(z.zielflotte)===Number(o.flotte)||Number(z.versorgen)===Number(o.flotte));
  return `<button type="button" class="es-zielzeile es-zielflotte${an?' gewaehlt':''}" data-es-ziel="${h(k)}" data-es-zielart="planet" data-es-zf="${h(o.flotte)}" data-es-zf-art="${art}" aria-pressed="${an}"><span class="es-zielbild es-zielbild-flotte${o.eigen?' eigen':''}" aria-hidden="true"><svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" focusable="false"><path d="M3 8h18l-3 4H6ZM12 4v4M12 12v8M8.5 16.5a3.5 3.5 0 0 0 7 0M10 14.5h4"/></svg></span><span class="es-zielzeile-text"><b>${h(titel)}</b><small>${h(unter)}</small></span><span class="es-pfeil" aria-hidden="true">›</span></button>`;
 }
 function zeile(k,art,titel,unter,extra=''){
  const an=z.ziel===k&&z.zielart===art;
  return `<button type="button" class="es-zielzeile${an?' gewaehlt':''}" data-es-ziel="${h(k)}" data-es-zielart="${art}" aria-pressed="${an}">${zielBild(k,art)}<span class="es-zielzeile-text"><b>${h(titel)}</b><small>${h(unter)}</small></span>${extra}<span class="es-pfeil" aria-hidden="true">›</span></button>`;
 }
 function passtZurMission(k,art){if(!z.mission||!A())return true;const a=A().info(z.mission),m=zielMerkmale(k,art);return (a.ziel||[]).some(x=>m.has(x))||(z.mission==='terraformen'&&(m.has('eigen')||m.has('leer')))||(z.mission==='verstaerken'&&(V()?.invasionen||[]).some(i=>i.planet===k&&i.rolle==='angriffshelfer'));}
 function vorschlaege(q){
  const v=V()||{},gruppen=[],t=String(q||'').trim().toLowerCase(),passt=(k,art)=>passtZurMission(k,art);
  const alter=s=>s!=null?`vor ${zahl(Math.max(0,((v.sekunden||0)-s)/3600))} h`:'';
  if(KOORD.test(t)){const d=zielBeschreibung(t,'planet');gruppen.push(['Koordinate',[zeile(t,'planet',planetName(t),[d.art,d.spieler,d.truemmer?`Trümmerfeld ${zahl(d.truemmer.erz)} Erz · ${zahl(d.truemmer.kristall)} Kristall${d.truemmer.ueberlebende?` · ${zahl(d.truemmer.ueberlebende)} Überlebende`:''}`:''].filter(Boolean).join(' · '))]]);
   if(d.mond)gruppen[0][1].push(zeile(t,'mond',`☾ ${d.mond.name||'Mond'}`,`Mond bei [${t}]${d.mond.besitzer&&!d.mond.eigen?' · '+d.mond.besitzer:''}`));}
  const sys=/^(\d{1,2}):(\d{1,3}):?$/.exec(t);
  if(sys){const s=`${sys[1]}:${sys[2]}`,n=R()?.catalog?.welt?.plaetze_je_system||12,liste=[];
   if((v.bergbauziele||[]).some(b=>b.koord===s+':0'))liste.push(zeile(s+':0','planet','Asteroidengürtel',`[${s}:0] · Abbau`));
   for(let i=1;i<=n;i++){const k=`${s}:${i}`,d=zielBeschreibung(k,'planet');if(!passt(k,'planet')&&z.mission)continue;liste.push(zeile(k,'planet',d.merkmale.has('eigen')?planetName(k):`Platz ${i}${d.spieler?' · '+d.spieler:''}`,`${d.art}${d.truemmer?' · Trümmerfeld':''}${d.zone?' · '+name(d.zone):''}`));}
   gruppen.push([`System ${s}`,liste]);}
  if(t&&!KOORD.test(t)&&!sys){
   const eigene=planeten().filter(p=>(p.name||'').toLowerCase().includes(t)||planetName(p.koord).toLowerCase().includes(t));
   if(eigene.length)gruppen.push(['Eigene Planeten',eigene.map(p=>zeile(p.koord,'planet',planetName(p.koord),p.heimat?'Heimatwelt':'Kolonie'))]);
   const namen=new Set([...atlas().map(a=>a.spieler).filter(Boolean),...(v.berichte||[]).map(b=>b.besitzer).filter(Boolean),...(v.rangliste||[]).map(r=>r[1])]);
   for(const s of [...namen].filter(n=>n.toLowerCase().includes(t)&&n!==v.name).slice(0,6)){
    const orte=new Map();for(const a of atlas().filter(a=>a.spieler===s))orte.set(a.koord,a);for(const b of (v.berichte||[]).filter(b=>b.besitzer===s))if(!orte.has(b.ziel))orte.set(b.ziel,{koord:b.ziel});
    const liste=[...orte.keys()].filter(k=>passt(k,'planet')).map(k=>{const b=bericht(k);return zeile(k,'planet',`${s} · [${k}]`,b?`Bericht ${alter(b.zeit)}${b.heimat?' · Heimatwelt':''}`:'Planet bekannt');});
    for(const a of atlas().filter(a=>a.spieler===s&&a.mond))if(passt(a.koord,'mond'))liste.push(zeile(a.koord,'mond',`☾ ${a.mond.name} · ${s}`,`Mond bei [${a.koord}] · ${zahl(a.mond.durchmesser)} km`));
    gruppen.push([`Spieler ${s}`,liste.length?liste:[`<p class="es-leise">Von ${h(s)} kennst du noch keinen Planeten. Systeme erkunden, dann erscheinen sie hier.</p>`]]);}
  }
  if(!t){
   const zuletzt=verlauf().filter(x=>passt(x.ziel,x.zielart)).slice(0,5);
   if(zuletzt.length)gruppen.push(['Zuletzt',zuletzt.map(x=>{const d=zielBeschreibung(x.ziel,x.zielart);return zeile(x.ziel,x.zielart,zielTitel(x.ziel,x.zielart),[d.art,d.spieler,x.mission?A()?.info(x.mission).name:''].filter(Boolean).join(' · '));})]);
   const eigene=[...planeten().map(p=>[p.koord,'planet']),...eigeneMonde().map(m=>[m.koord,'mond'])].filter(([k,a])=>passt(k,a)&&!(k===z.start&&a===z.startart));
   if(eigene.length)gruppen.push(['Eigene Planeten und Monde',eigene.map(([k,a])=>zeile(k,a,zielTitel(k,a),a==='mond'?'Mond':eigenerPlanet(k)?.heimat?'Heimatwelt':'Kolonie'))]);
   if(orbitWelt()){const gf=z.startart==='flotte'?gestrandeteFlotte(z.start):null,l=[];
    for(const o of bekannteOrbitflotten()){const a={koord:o.koord};
     if(o.eigen){if(gf||String(o.flotte)===String(z.start)||(z.mission&&z.mission!=='transport'))continue;
      l.push(flottenZeile(a.koord,o,`Eigene Flotte #${o.flotte} versorgen`,`gestrandet bei [${a.koord}] · ${zahl(o.schiffe)} Schiffe · Transport bringt Deuterium in die Ladung`,'versorgen'));}
     else{if((z.mission&&z.mission!=='angriff')||(gf&&!orbitAuftragErlaubt(gf,'angriff')))continue;
      l.push(flottenZeile(a.koord,o,`Flotte von ${o.spieler} angreifen`,`gestrandet bei [${a.koord}] · ${zahl(o.schiffe)} Schiffe${o.nur_heim?' · darf nur heimfliegen':''}`,'angriff'));}}
    if(l.length)gruppen.push(['Gestrandete Flotten',l.slice(0,10)]);}
   const tf=(v.truemmer||[]).filter(t=>passt(t.koord,'planet'));
   if(tf.length)gruppen.push(['Trümmerfelder',tf.slice(0,8).map(t=>zeile(t.koord,'planet',`Trümmerfeld [${t.koord}]`,`${zahl(t.erz)} Erz · ${zahl(t.kristall)} Kristall${t.ueberlebende?` · ${zahl(t.ueberlebende)} Überlebende`:''}`))]);
   // Recycler hinter den eigenen Angriff (Ankunft koppeln): Ziele laufender Angriffe ohne Trümmerfeld.
   // Auch ohne gewählten Auftrag (Funktionsprüfung 25.10.2026: beim normalen Einstieg fehlte die Gruppe); ein Tipp setzt Recyceln.
   if(z.mission==='recyceln'||!z.mission){const an=[...new Map((v.flotten||[]).filter(f=>f.zustand==='hinflug'&&A()?.feindlich?.(f.mission)&&!truemmerAn(f.ziel)&&KOORD.test(f.ziel||'')).map(f=>[f.ziel,f])).values()].slice(0,6);
    if(an.length)gruppen.push(['Deine Angriffe unterwegs',an.map(f=>zeile(f.ziel,'planet',`Angriffsziel [${f.ziel}]`,`Einschlag ${uhrzeit(f.ankunft_sekunden)} · Recycler hinterher koppeln`).replace('data-es-ziel=','data-es-recycler="1" data-es-ziel='))]);}
   const gu=(v.bergbauziele||[]).filter(b=>passt(b.koord,'planet'));
   if(gu.length)gruppen.push(['Gürtel und Ringe',gu.slice(0,8).map(b=>zeile(b.koord,'planet',b.art||'Asteroidengürtel',`[${b.koord}]`))]);
   const ber=[...new Map((v.berichte||[]).sort((a,b)=>(b.zeit||0)-(a.zeit||0)).map(b=>[b.ziel,b])).values()].filter(b=>passt(b.ziel,'planet')).slice(0,8);
   // Fremde Planeten aus der Galaxie ohne Spionagebericht (Übergabe, Transport, Halten … brauchen keinen Bericht).
   const gesehen=new Set(ber.map(b=>b.ziel)),fremd=atlas().filter(a=>a.spieler&&a.spieler!==v.name&&!gesehen.has(a.koord)&&passt(a.koord,'planet')).slice(0,8);
   if(fremd.length)gruppen.push(['Bekannte Planeten anderer Spieler',fremd.map(a=>zeile(a.koord,'planet',`${a.spieler} · [${a.koord}]`,[verwuestung(a.koord).an?'verwüstet':'',a.zone?name(a.zone):''].filter(Boolean).join(' · ')||'aus der Galaxie bekannt'))]);
   if(ber.length)gruppen.push(['Ausgespäht',ber.map(b=>{const bt=window.Flottenfuehrung?.beute?Flottenfuehrung.beute(b):null;return zeile(b.ziel,'planet',`${b.besitzer||'?'} · [${b.ziel}]`,`${alter(b.zeit)}${bt?.summe?` · Beute ≈ ${zahl(bt.summe)}`:''}`);})]);
   const sk=String(startOrt()?.koord||planeten().find(p=>p.heimat)?.koord||planeten()[0]?.koord||'').split(':').map(Number),abst=k=>{const q=String(k).split(':').map(Number);return Math.abs((q[0]||0)-(sk[0]||0))*1000+Math.abs((q[1]||0)-(sk[1]||0));};
   const frei=atlas().filter(a=>!a.bewohnt&&!a.spieler&&passt(a.koord,'planet')).sort((a,b)=>abst(a.koord)-abst(b.koord)).slice(0,12);
   if(frei.length&&(['kolonisieren','terraformen','saven'].includes(z.mission)||!z.mission))gruppen.push(['Freie Plätze',frei.map(a=>zeile(a.koord,'planet',`Freier Platz [${a.koord}]`,a.zone?name(a.zone):'erkundet'))]);
  }
  return gruppen;
 }
 function zielHtml(){
  const gruppen=vorschlaege(z.suche),d=KOORD.test(z.ziel)?zielBeschreibung():null;
  return `<section class="es-teil"><label class="es-suche"><span class="es-suche-titel">Ziel suchen</span><input type="search" data-es-suche value="${h(z.suche)}" placeholder="Koordinate 2:28:6, System 2:28 oder Spielername" autocomplete="off" enterkeyhint="search"></label>
   ${z.mission?`<p class="es-leise">Gezeigt werden Ziele, die zu ${badge(z.mission,{size:16})} <b>${h(A()?.info(z.mission).name)}</b> passen.</p>`:''}
   ${d?zielKarte():''}
   ${gruppen.map(([t,l])=>`<div class="es-gruppe"><h3>${h(t)}</h3><div class="es-zielliste">${l.join('')}</div></div>`).join('')||(z.suche?'<p class="es-leise">Nichts gefunden. Koordinaten schreibt man Sektor:System:Platz, z. B. 2:28:6.</p>':'<p class="es-leise">Noch keine bekannten Ziele. Tippe eine Koordinate ein oder erkunde Systeme in der Galaxie.</p>')}</section>`;
 }
 function zielKarte(){
  const d=zielBeschreibung(),k=z.ziel,v=V()||{},b=d.bericht,bt=b&&window.Flottenfuehrung?.beute?Flottenfuehrung.beute(b):null;
  const fakten=[];
  const zfo=z.zielflotte?bekannteOrbitflotten().find(o=>Number(o.flotte)===Number(z.zielflotte)):null;
  if(verwuestung(k).an)fakten.push(['Status',verwuestung(k).besatzer?'Verwüstet – deine Truppen halten den Boden; ein Kolonieschiff (Kolonisieren oder Kampfkolonisieren) übernimmt ihn':'Verwüstet – der Besitzer hat die Kontrolle verloren']);
  if(zfo)fakten.push(['Zielflotte',`gestrandet · ${h(zfo.spieler)} · ${zahl(zfo.schiffe)} Schiffe`]);
  if(z.versorgen){const gf=gestrandeteFlotte(z.versorgen);if(gf)fakten.push(['Versorgt',`deine Flotte #${h(gf.flotte)} · ${zahl(gf.gestrandet.deuterium)} Deuterium an Bord`]);}
  if(d.spieler)fakten.push(['Spieler',`<button type="button" class="es-link" data-es-profil="${h(d.spieler)}">${h(d.spieler)}</button>${d.heimat?' · Heimatwelt':''}`]);
  if(d.zone)fakten.push(['Zone',h(name(d.zone))]);
  if(b&&(d.spieler||!(v.erkundet||[]).some(x=>x.koord===k)))fakten.push(['Bericht',`vor ${zahl(Math.max(0,((v.sekunden||0)-(b.zeit||0))/3600))} h${bt?.summe&&d.merkmale.has('fremd')?` · ${z?.mission==='angriff'&&laderaum()<bt.summe?`Beute ≈ ${zahl(laderaum())} (Laderaum voll; am Ziel ≈ ${zahl(bt.summe)})`:`Beute am Ziel ≈ ${zahl(bt.summe)}`}`:''}`]);
  if(d.truemmer)fakten.push(['Trümmerfeld',`${zahl(d.truemmer.erz)} Erz · ${zahl(d.truemmer.kristall)} Kristall${d.truemmer.ueberlebende?` · ${zahl(d.truemmer.ueberlebende)} Überlebende`:''}`]);
  if(d.mond&&z.zielart!=='mond')fakten.push(['Mond',`${h(d.mond.name||'Mond')}${d.mond.durchmesser?' · '+zahl(d.mond.durchmesser)+' km':''}`]);
  if(z.zielart==='mond'&&d.mond)fakten.push(['Durchmesser',`${zahl(d.mond.durchmesser||d.mond.groesse)} km`]);
  if(d.guertel)fakten.push(['Abbau',h(d.guertel.art||'Asteroidengürtel')]);
  // Alles Bekannte über den Platz (Karl 24.10.: vor dem Absenden Größe, Art, Felder … sehen) – aus der eigenen Aufklärung.
  const e=(v.erkundet||[]).find(x=>x.koord===k),p=eigenerPlanet(k);
  if(e||p){const zone=e?.zone||p?.zone||d.zone,typ=zone&&window.PlanetKugel?.typ?PlanetKugel.typ({...(e||{}),koord:k,zone}):'';
   const typName=typ?String(window.STERNEN_ART?.maps?.[`planets|${typ}|`]?.label||'').replace(/^Planet\s+/,'').replace(/\s*\(.*\)\s*$/,''):'';
   const spanne=v2=>Array.isArray(v2)?(v2[0]===v2[1]?`${zahl(v2[0]/10)} %`:`${zahl(v2[0]/10)}–${zahl(v2[1]/10)} %`):v2!=null?`${zahl(v2/10)} %`:'';
   const felder=p?.felder?.gesamt??e?.felder;const hab=e?.habitabilitaet||p?.habitabilitaet;
   if(typName)fakten.push(['Typ',h(typName)]);
   if(felder)fakten.push(['Größe',`${zahl(felder)} Felder${p?.felder?.belegt!=null?` · ${zahl(p.felder.belegt)} belegt`:''}`]);
   if(e?.ertrag_promille)fakten.push(['Ertrag',`Erz ${h(spanne(e.ertrag_promille.erz))} · Kristall ${h(spanne(e.ertrag_promille.kristall))}`]);
   if(hab?.solar_wirksam_promille!=null)fakten.push(['Sonnenlicht',`${zahl(hab.solar_wirksam_promille/10)} %`]);
   if(e)fakten.push(['Vorkommen',h([e.xeno_vorkommen?'Xenokristall':'',e.rohstoffring?'Rohstoffring':''].filter(Boolean).join(' · ')||'keine besonderen')]);
   if(e?.alter_stunden!=null)fakten.push(['Erkundet',`vor ${zahl(e.alter_stunden)} Spielstunden`]);}
  return `<div class="es-zielkarte">${zielBild(k,z.zielart)}<div><div class="es-zielkarte-kopf"><b class="koord">${h(zielTitel(k,z.zielart))}</b><span class="es-zielart">${h(d.art)}</span></div>
   ${fakten.length?`<dl class="es-fakten">${fakten.map(([t,w])=>`<div><dt>${h(t)}</dt><dd>${w}</dd></div>`).join('')}</dl>`:''}
   <div class="es-zielkarte-knoepfe"><button type="button" class="es-mini" data-es-galaxie="${h(k)}">Im System zeigen</button>${z.schritt!=='ziel'?'<button type="button" class="es-mini" data-es-schritt="ziel">Ziel ändern</button>':''}</div></div></div>`;
 }

 // ---------------------------------------------------------------- Schritt ② Auftrag
 const IMMER_AUSSEN=['verband','rakete','expedition','flotten_spionage'];
 function auftragHtml(){
  const v=V()||{},liste=Object.values(A()?.liste||{}).filter(a=>!a.pseudo&&UNTERSTUETZT.has(a.key));
  if(A()&&!A().liste.terraformen)liste.push({key:'terraformen',name:'Terraformen',zweck:'Klima und Felder eines Planeten formen',gruppe:'ausbreitung',ziel:['eigen','leer']});
  const kolonisationV2=(v.regelkontext?.kolonisation_version||0)>=2;
  const sichtbar=liste.filter(a=>!(a.key==='invasion'&&kolonisationV2)&&!((a.key==='bombardieren'||a.key==='kampfkolonisieren')&&!kolonisationV2));
  const d=zielBeschreibung(),passend=[],andere=[];
  for(const a of sichtbar){const b=bedingungen(a.key);(b[0].ok?passend:andere).push({a,b});}
  passend.sort((x,y)=>Number(y.b.every(c=>c.ok))-Number(x.b.every(c=>c.ok)));
  const karte=({a,b})=>{const ok=b.every(c=>c.ok),grund=b.find(c=>!c.ok&&c.vorrang)||b.find(c=>!c.ok),bild=schiffBild(a.key);return `<button type="button" class="es-auftrag auftrag-${h(a.gruppe)}${z.mission===a.key?' gewaehlt':''}${ok?'':' gesperrt'}" data-es-mission="${h(a.key)}" aria-pressed="${z.mission===a.key}"${bild?` data-css="--es-karte:url(&quot;${h(bild)}&quot;)"`:''}>${badge(a.key,{size:28})}<span class="es-auftrag-text"><b>${h(a.name)}</b><small>${h(a.zweck||'')}</small>${ok?'<em class="es-ok">bereit</em>':`<em class="es-grund">${h(grund?.text||'')}</em>`}</span></button>`;};
  const mitInfo=x=>`<div class="es-auftrag-rahmen">${karte(x)}${P[x.a.key]?infoKnopf('mission:'+x.a.key,'So läuft '+x.a.name+' ab'):''}</div>`;
  const extra=['rakete','expedition'].filter(k=>A()?.liste[k]&&(A().info(k).ziel||[]).some(x=>d.merkmale.has(x)));
  return `${zielKarte()}<section class="es-teil"><h3 class="es-teil-titel">Was soll die Flotte tun?</h3><div class="es-auftraege">${passend.map(mitInfo).join('')||'<p class="es-leise">Für dieses Ziel passt kein Auftrag.</p>'}</div>
   ${extra.length?`<div class="es-aussen">${extra.map(k=>`<button type="button" class="es-mini" data-es-aussen="${h(k)}">${badge(k,{size:16})} ${h(A().info(k).name)} …</button>`).join('')}</div>`:''}
   ${andere.length?`<details class="es-klapp"${z.offen.andere?' open':''} data-es-klapp="andere"><summary>Passen nicht zu diesem Ziel (${andere.length})</summary><div class="es-auftraege es-auftraege-andere">${andere.map(mitInfo).join('')}</div></details>`:''}</section>`;
 }

 // ---------------------------------------------------------------- Regelkarte (So läuft es ab · Das zählt · Vorbedingungen)
 function regelkarte(){
  const m=z.mission,p=P[m],b=bedingungen(m),offen=b.filter(x=>!x.ok||x.warnung),erfuellt=b.filter(x=>x.ok&&!x.warnung);
  if(!p)return '';
  const auf=z.offen.regel??!gesehen(m);
  return `<details class="es-regel auftrag-${h(A()?.info(m).gruppe)}" data-es-klapp="regel"${auf?' open':''}><summary><span>So läuft ${h(A()?.info(m).name||m)} ab</span><small>${p.ablauf.length} Schritte · ${p.zaehlt.length} Regeln</small></summary>
   <ol class="es-ablauf">${p.ablauf.map((t,i)=>`<li><span class="es-ablauf-nr">${i+1}</span><span>${h(t)}</span></li>`).join('')}</ol>
   <div class="es-zaehlt"><h4>Das zählt</h4><ul>${p.zaehlt.map(t=>`<li>${h(t)}</li>`).join('')}</ul></div></details>
   ${offen.some(x=>x.aktion?.sonde)?`<p class="es-leise">Warum Sonden? ${infoKnopf('aufklaerung')}</p>`:''}
   ${offen.length?`<ul class="es-bedingungen offen">${offen.map(x=>`<li><span class="es-bed-zeichen" aria-hidden="true">!</span><span>${h(x.text)}</span>${x.aktion?`<button type="button" class="es-mini" ${x.aktion.sonde?`data-es-sonde="${h(x.aktion.sonde)}"`:x.aktion.mission?`data-es-folge="${h(x.aktion.mission)}"`:x.aktion.forschung?`data-es-forschung="${h(x.aktion.forschung)}"`:`data-es-go="${h(x.aktion.go)}"`}>${h(x.aktion.text)}</button>`:''}</li>`).join('')}</ul>`:''}
   ${erfuellt.length?`<details class="es-klapp es-bedingungen-ok" data-es-klapp="bedingungen"${z.offen.bedingungen?' open':''}><summary>${erfuellt.length} Vorbedingungen erfüllt</summary><ul class="es-bedingungen">${erfuellt.map(x=>`<li class="ok"><span class="es-bed-zeichen" aria-hidden="true">✓</span><span>${h(x.text)}</span></li>`).join('')}</ul></details>`:''}`;
 }
 function gesehen(m){try{return !!localStorage.getItem('sternenepoche-einsatz-regel:'+m);}catch(_){return true;}}
 function alsGesehen(m){try{localStorage.setItem('sternenepoche-einsatz-regel:'+m,'1');}catch(_){}}

 // ---------------------------------------------------------------- Lage am Ziel (feindliche Aufträge)
 function lageHtml(){
  const m=z.mission;if(!P[m]?.lage)return '';
  // Angriff auf eine gestrandete Flotte: nur diese Flotte kämpft (kein Planet, keine Abwehr) – keine Planetensimulation.
  if(z.zielflotte){const o=bekannteOrbitflotten().find(x=>Number(x.flotte)===Number(z.zielflotte));
   return `<section class="es-lage"><h3 class="es-teil-titel">Lage am Ziel</h3><div class="es-lagezahlen">${o?`<div class="es-lagezahl"><b>${zahl(o.schiffe)}</b><span>Schiffe der gestrandeten Flotte von ${h(o.spieler)}</span></div>`:'<p class="es-leise">Die Zielflotte ist nicht mehr gestrandet.</p>'}<p class="es-leise">Gekämpft wird nur gegen gestrandete Flotten dieses Spielers an [${h(z.ziel)}] – Planet und Abwehr bleiben außen vor. Trümmer entstehen dort. Schiffstypen zeigt erst ein Spionagebericht.</p></div></section>`;}
  const b=bericht(z.ziel,z.zielart),v=V()||{},bt=beute(),d=zielBeschreibung();
  // Wert wie Kern Regelwerk::wert (Wertungsgewichte: Kristall 1,5, Deuterium 2, Legierung 5 …), sonst lag die Lage unter dem
  // Gegnerverlust der Simulation direkt darunter (Funktionsprüfung 25.10.2026).
  const gw=rules?.catalog?.wertung?.gewichte||{erz:1,kristall:1,deuterium:1};
  const wert=map=>Object.entries(map||{}).reduce((s,[k,n])=>s+Object.entries(regel(k).kosten||{}).reduce((t,[g,v])=>t+(Number(v)||0)*(gw[g]??0),0)*(Number(n)||0),0);
  const zeilen=[];
  // Wie Kern monde.rs zerstoerung_promille_mit: wirksamer Durchmesser mit Typzuschlag (Basalt +15 %, Eis −10 % …), ganzzahlige
  // Wurzeln, Promille (Funktionsprüfung 25.10.2026: ohne Typ zeigte die Lage die doppelte Chance, Risiko 27,4 statt 27,3 %).
  if(m==='mond_zerstoeren'){const dm=Number(d.mond?.durchmesser||d.mond?.groesse)||0,n=z.schiffe.weltentruemmer||0;
   const mods=V()?.monde?.regeln?.zerstoerung_mod||{},mod=Number(mods[d.mond?.typ])||0,isqrt=x=>Math.floor(Math.sqrt(x)),deff=Math.round(dm*(1+mod));
   const wd=isqrt(deff*1e6),wn=isqrt(n*1e6),zerst=dm&&n?Math.max(0,Math.min(1000,Math.floor((100*1000-wd)*wn/100000))):null,verl=dm?Math.max(0,Math.min(1000,Math.floor(wd/(20*10)))):null;
   const pz=x=>(x/10).toLocaleString('de-DE',{maximumFractionDigits:1})+' %';
   zeilen.push(`<div class="es-lagezahl"><b>${zerst!=null?pz(zerst):'–'}</b><span>Zerstörungschance${n?` mit ${zahl(n)} Weltenkrümmer${n===1?'':'n'}`:''}${mod?` · ${h(name(d.mond.typ))} zählt ${mod>0?'+':''}${Math.round(mod*100)} % Durchmesser`:''}</span></div>`);
   if(verl!=null)zeilen.push(`<div class="es-lagezahl warn"><b>${pz(verl)}</b><span>Risiko: alle Weltenkrümmer verloren</span></div>`);}
  if(b){const alter=Math.max(0,((v.sekunden||0)-(b.zeit||0))/3600);
   zeilen.push(`<div class="es-lagezahl${alter>6?' warn':''}"><b>${zahl(alter)} h</b><span>Alter des Berichts</span></div>`);
   if(b.schiffe)zeilen.push(`<div class="es-lagezahl"><b>${zahl(wert(b.schiffe))}</b><span>Flottenwert am Ziel</span></div>`);
   if(b.verteidigung)zeilen.push(`<div class="es-lagezahl"><b>${zahl(wert(b.verteidigung))}</b><span>Abwehrwert</span></div>`);
   if(bt?.summe&&m==='angriff'){const lr=laderaum();zeilen.push(`<div class="es-lagezahl${lr<bt.summe?' warn':''}"><b>${zahl(Math.min(bt.summe,lr))}</b><span>${lr<bt.summe?`Beute ≈, vom Laderaum begrenzt (am Ziel ≈ ${zahl(bt.summe)})`:`Beute ≈ · Laderaum ${zahl(lr)} reicht`}</span></div>`);}
   if(m==='bombardieren'||m==='kampfkolonisieren'){const g=b.gebaeude;const stufen=g?Object.values(g).reduce((s,n)=>s+(Number(n)||0),0):0;
    const last=Object.entries(z.schiffe).reduce((s,[k,n])=>s+(k==='strategischer_bomber'?150:k==='bomber'?100:istBomber(k)?50:0)*(Number(n)||0),0);
    if(stufen&&last){const promille=Math.ceil(1000*last/(700*stufen)),runden=Math.ceil(700/promille);zeilen.push(`<div class="es-lagezahl"><b>${zahl(runden)} Runden</b><span>bis 30 % · ≈ ${dauer(Math.max(0,runden-1)*15*60)} Bombardement (${zahl(promille/10)} % je Runde)</span></div>`);}
    else if(!stufen)zeilen.push('<p class="es-leise">Für die Bombendauer braucht es einen Bericht mit Gebäuden (ab 5 Sonden).</p>');}
  }else if(m!=='mond_zerstoeren')zeilen.push('<p class="es-leise">Kein Spionagebericht – Flotte und Abwehr am Ziel sind unbekannt.</p>');
  const s=z.sim,simOk=s&&z.simSchluessel===JSON.stringify([z.ziel,z.schiffe]);
  // Trümmerfeld, Mondchance (wie Kern monde.rs: 1 % je truemmer_je_prozent, höchstens chance_max_prozent, nur ohne Mond) und
  // der Recycler-Folgeflug – stand bis 0.4.8 nur im klassischen Flottenformular (Bedarfsrechner).
  const tf=simOk?Number(s.truemmer_mittel)||0:0,mr=V()?.monde?.regeln||{},je=mr.truemmer_je_prozent||100000,mx=mr.chance_max_prozent??20;
  const mondDa=z.zielart==='mond'||!!d.mond,mc=mondDa?0:Math.min(mx,Math.floor(tf/je));
  const folgen=tf>0?`<div class="es-lagezahl"><b>${zahl(tf)}</b><span>Trümmerfeld ≈ · ${mondDa?'am Ziel kreist schon ein Mond':`Mondchance ≈ ${zahl(mc)} % (1 % je ${zahl(je)}, höchstens ${zahl(mx)} %)`} · Recycler hinterher: nach dem Start „Recyceln“ mit „Ankunft koppeln“</span></div>`:'';
  const sim=m==='angriff'||m==='blockade'||m==='bombardieren'||m==='kampfkolonisieren'||m==='mond_zerstoeren'?`<div class="es-sim">${simOk?`<div class="es-lagezahl ${s.siegchance_prozent>=90?'gut':s.siegchance_prozent>=50?'warn':'schlecht'}"><b>${zahl(s.siegchance_prozent)} %</b><span>Siegchance · eigene Verluste ${zahl(s.eigene_verluste_wert)} · Gegner ${zahl(s.verluste_gegner_wert)}</span></div>${folgen}`:''}<button type="button" class="es-mini" data-es-sim ${b&&schiffSumme()?'':'disabled'}>${simOk?'Neu simulieren':'Kampf simulieren (64 Läufe)'}</button>${!b?'<small class="es-leise">Simulation braucht einen Spionagebericht – erst ausspähen.</small>':!schiffSumme()?'<small class="es-leise">Erst Schiffe wählen.</small>':''}</div>`:'';
  return `<section class="es-lage"><h3 class="es-teil-titel">Lage am Ziel</h3><div class="es-lagezahlen">${zeilen.join('')}</div>${sim}</section>`;
 }

 // ---------------------------------------------------------------- Schritt ③ Flotte
 function terraformHtml(){
  if(z.mission!=='terraformen')return '';const q=z.tf;
  if(!q)return `<section class="es-lage"><h3 class="es-teil-titel">Terraforming-Plan</h3><p class="es-leise">${z.tfFehler?h(z.tfFehler):'Plan wird berechnet …'}</p></section>`;
  const klima=Number(q.klima_prozent)||0,felder=Number(q.felder)||0,grenze=Number(q.feldgrenze)||0;
  const stufen=Object.entries(q.naechste_stufen||q.stufen||{}).slice(0,4);
  const reichsvoelker=VOELKER.filter(v=>v===V()?.volk||planeten().some(p=>personenAm(p,v).some(n=>n>0)));
  const wahl=reichsvoelker.length>1?`<div class="es-vorlagen" role="group" aria-label="Zielvolk">${reichsvoelker.map(v=>chip('data-es-tfvolk',v,'Für '+name(v),(z.tfVolk||V()?.volk)===v)).join('')}</div>`:'';
  return `<section class="es-lage es-terraform"><h3 class="es-teil-titel">Terraforming-Plan · ${h(planetName(z.ziel))}</h3>${wahl}
   <div class="es-tf-zeile"><span>Klima für ${h(name(q.volk||V()?.volk))}</span>${balken('es-tf-balken',[{wert:klima,cls:'ist',titel:`${klima} % angepasst`}],100)}<b>${zahl(klima)} %</b></div>
   ${q.grundgroesse?`<p class="es-leise">Grundgröße ${zahl(q.grundgroesse)} Felder → komplett ${zahl(q.felder_hoechst??(q.grundgroesse+grenze))} Felder${q.bewohnbar===false?' · für dieses Volk noch nicht bewohnbar – zuerst das Klima':''}</p>`:q.bewohnbar===false?'<p class="es-leise">Für dieses Volk noch nicht bewohnbar – das Klima kommt zuerst.</p>':''}
   <div class="es-tf-zeile"><span>Zusatzfelder</span>${balken('es-tf-balken',[{wert:felder,cls:'ist',titel:`${felder} von ${grenze}`}],Math.max(1,grenze))}<b>+${zahl(felder)} / ${zahl(grenze)}</b></div>
   <div class="es-lagezahlen"><div class="es-lagezahl"><b>${zahl(q.schiffe_klima)}</b><span>Terraformer fürs Klima</span></div><div class="es-lagezahl"><b>${zahl(q.schiffe_felder)}</b><span>Terraformer für die Felder</span></div><div class="es-lagezahl gut"><b>${zahl(q.schiffe_voll??((Number(q.schiffe_klima)||0)+(Number(q.schiffe_felder)||0)))}</b><span>komplett · du hast ${zahl(da('terraformer'))}</span></div></div>
   ${stufen.length?`<p class="es-leise">Mit höherer Forschung: ${stufen.map(([s,n])=>`Stufe ${h(s)}: ${zahl(n)}`).join(' · ')}</p>`:''}${q.hinweis?`<p class="es-leise">${h([].concat(q.hinweis).join(' '))}</p>`:''}</section>`;
 }
 function startWahlHtml(){
  const m=z.mission,p=P[m]||{},rel=k=>(p.nur||[]).includes(k)||sektion(k)===p.sektionen?.[0];
  const orte=startOrte();
  if(!orte.length)return `<p class="es-leise">Auf keinem deiner Planeten stehen passende Schiffe.</p>`;
  return `<div class="es-start" role="group" aria-label="Startort">${orte.map(o=>{const n=Object.entries(o.schiffe||{}).filter(([k])=>rel(k)).reduce((s,[,x])=>s+x,0),an=o.koord===z.start&&o.art===z.startart;
   return `<button type="button" class="es-chip es-chip-start" data-es-start="${h(o.koord)}" data-es-startart="${o.art}" aria-pressed="${an}"><b>${h(o.art==='flotte'?`☄ Flotte ${o.flotte.flotte} an [${o.flotte.ziel}]`:(o.art==='mond'?'☾ ':'')+(o.art==='mond'?(o.name||'Mond'):planetName(o.koord)))}</b><small>${o.art==='flotte'?h(`gestrandet · ${zahl(o.flotte.gestrandet?.deuterium||0)} Deuterium an Bord${o.flotte.gestrandet?.nur_heim?' · nur Heimflug/Übergabe':''}`):n?(['kolonisieren','kampfkolonisieren'].includes(m)?`${zahl(n)} im Hangar · startet 1`:`${zahl(n)} × ${h(SEKTION_TITEL[p.sektionen?.[0]]||'Schiffe')}`):'Schiffe vorhanden'}</small></button>`;}).join('')}</div>`;
 }
 function startOrte(){
  const m=z.mission,pf=PFLICHT[m];
  const orbit=orbitWelt()&&!z.zielflotte&&!z.versorgen?gestrandeteFlotten().filter(f=>orbitAuftragErlaubt(f,m)).map(f=>({koord:String(f.flotte),art:'flotte',schiffe:f.schiffe||{},flotte:f})):[];
  // Einsatzprobe 25.10.2026: ein übernommener Planet ohne Raumhafen wurde vorgewählt, der Kern lehnte erst beim Start ab.
  const raumhafen=x=>(Number(x.gebaeude?.raumhafen)||0)>=1&&(x.gebaeude_integritaet_prozent?.raumhafen??100)>0;
  return [...planeten().filter(raumhafen).map(x=>({...x,art:'planet'})),...(MOND_AUFTRAEGE.has(m)?eigeneMonde().map(x=>({...x,art:'mond'})):[]),...orbit]
   .filter(o=>!(o.koord===z.ziel&&o.art===z.zielart))
   .filter(o=>pf?pf.some(k=>(o.schiffe?.[k]||0)>0):Object.entries(o.schiffe||{}).some(([k,n])=>n>0&&erlaubt(k)));
 }
 function schiffZeile(k,n){
  const r=regel(k),w=z.schiffe[k]||0,info=[`${zahl(n)} da`];
  if(r.ladung){const lj=ladungJe(k);info.push(lj&&Math.round(lj.wert)!==lj.grund?`Laderaum ${zahl(Math.round(lj.wert))} (Grundwert ${zahl(lj.grund)})`:`Laderaum ${zahl(r.ladung)}`);}if(r.angriff&&istKampf(k))info.push(`Angriff ${zahl(r.angriff)}`);
  const pp=plaetzeJe(k);if(pp)info.push(`${zahl(pp)} Personenplätze`);
  return `<div class="es-schiff${w>0?' gewaehlt':''}" id="es-schiff-${h(k)}"><img src="${h(typeof art==='function'?art('ships',k):'')}" alt="" loading="lazy"><div class="es-schiff-name"><b>${h(name(k))} <button type="button" class="es-info" data-es-schiffinfo="${h(k)}" aria-label="Techinfo ${h(name(k))}">i</button></b><small>${h(info.join(' · '))}</small></div>${stepper('schiff',k,w,n,name(k))}</div>`;
 }
 function flotteHtml(){
  if(z.startart==='flotte'){const o=startOrt();
   return `${regelkarte()}<section class="es-teil"><h3 class="es-teil-titel">Start</h3>${startWahlHtml()}</section>
    <section class="es-teil"><h3 class="es-teil-titel">Diese Flotte fliegt</h3>${flottenReihe()}
     <p class="es-leise">Die ganze gestrandete Flotte fliegt mit ihrer Ladung. Treibstoff kommt aus dem Deuterium an Bord (${zahl(o?.flotte?.gestrandet?.deuterium||0)}); reicht er nicht, zuerst versorgen.</p>
     ${o?.flotte?`<div class="es-knopfreihe"><button type="button" class="es-mini" data-es-versorgen="${h(o.flotte.flotte)}">Diese Flotte versorgen …</button></div>`:''}</section>`;}
  const m=z.mission,p=P[m]||{},o=startOrt(),schiffe=Object.entries(o?.schiffe||{}).filter(([k,n])=>n>0&&erlaubt(k));
  const sek=p.sektionen||['kampf'],haupt=[],weitere=[];
  for(const s of sek)for(const [k,n] of schiffe)if(sektion(k)===s&&!haupt.some(x=>x[0]===k))haupt.push([k,n]);
  for(const [k,n] of schiffe)if(!haupt.some(x=>x[0]===k))weitere.push([k,n]);
  const nachSektion=liste=>{const g={};for(const [k,n] of liste)(g[sektion(k)]||=[]).push([k,n]);return Object.entries(g);};
  const vl=vorlagen(),summe=schiffSumme();
  const kennzahlen=[`<span><b>${zahl(summe)}</b> Schiffe</span>`];
  if(laderaum())kennzahlen.push(`<span>Laderaum <b>${zahl(laderaum())}</b></span>`);
  if(personenplaetze())kennzahlen.push(`<span>Personen <b>${zahl(personenplaetze())}</b></span>`);
  if(truppenKapazitaet())kennzahlen.push(`<span>Truppenplätze <b>${zahl(truppenKapazitaet())}</b></span>`);
  const kw=kampfwert(z.schiffe);if(kw&&P[m]?.lage)kennzahlen.push(`<span>Angriff <b>${zahl(kw)}</b></span>`);
  const hs=hinflugSekunden();if(hs!=null&&summe)kennzahlen.push(`<span>Hinflug <b>${h(dauer(hs))}</b></span>`);
  // Saven ist zeitkritisch (Befund B2: ohne Rückfrage): ein Tipp startet alles mit vollem Laderaum und sofortiger Rückkehr.
  const sofort=m==='saven'&&summe&&!flottenFehler()?`<div class="es-sofort"><button type="button" class="es-weiter es-starten" data-es-sofort>Jetzt retten · ${zahl(summe)} Schiffe, Laderaum voll</button><small>Startet sofort mit 100 % Tempo und kehrt ohne Wartezeit zurück. Feinheiten: „Weiter“.</small></div>`:'';
  return `${sofort}${regelkarte()}${terraformHtml()}${lageHtml()}
   <section class="es-teil"><h3 class="es-teil-titel">Start</h3>${startWahlHtml()}</section>
   ${o?`<section class="es-teil">${vl.length?`<h3 class="es-teil-titel">Vorlagen</h3><div class="es-vorlagen">${vl.map(x=>chip('data-es-vorlage',x.id,x.text,z.vorlage===x.id,x.zusatz)).join('')}</div>`:''}
    <div class="es-kennzahlen" aria-live="polite">${kennzahlen.join('')}</div>
    ${nachSektion(haupt).map(([s,l])=>`<div class="es-schiffgruppe"><h4>${h(SEKTION_TITEL[s]||s)}</h4>${l.map(([k,n])=>schiffZeile(k,n)).join('')}</div>`).join('')||`<p class="es-leise">Am Startort stehen keine passenden Schiffe.</p>`}
    ${weitere.length?`<details class="es-klapp" data-es-klapp="weitere"${z.offen.weitere||p.alleOffen?' open':''}><summary>Weitere Schiffe am Startort (${weitere.length})</summary>${nachSektion(weitere).map(([s,l])=>`<div class="es-schiffgruppe"><h4>${h(SEKTION_TITEL[s]||s)}</h4>${l.map(([k,n])=>schiffZeile(k,n)).join('')}</div>`).join('')}</details>`:''}
   </section>`:''}`;
 }

 // ---------------------------------------------------------------- Schritt ④ Details
 function frachtHtml(){
  const lr=laderaum(),f=fracht(),kolonie=z.mission==='kolonisieren';
  const gueter=GUETER.filter(g=>bestand(g)>0||(Number(z.bedarf?.fracht?.[g])||0)>0);
  // Nicht alles auf einmal (Karl): Grundgüter, Pflichtfracht und schon Geladenes oben, der Rest eingeklappt.
  const wichtig=g=>['erz','kristall','deuterium'].includes(g)||(Number(z.bedarf?.fracht?.[g])||0)>0||(Number(z.ladung[g])||0)>0||(kolonie&&g==='nahrung');
  // Geladenes zuerst, dann Pflichtfracht und Grundgüter – so steht das Wichtige oben.
  const sortiert=gueter.filter(wichtig).sort((a,b)=>Number((Number(z.ladung[b])||0)>0)-Number((Number(z.ladung[a])||0)>0));
  if(!z.gutReihe||sortiert.some(g=>!z.gutReihe.includes(g)))z.gutReihe=sortiert;
  const haupt=z.gutReihe.filter(g=>gueter.includes(g)&&wichtig(g)).concat(sortiert.filter(g=>!z.gutReihe.includes(g))),weitere=gueter.filter(g=>!wichtig(g)&&!haupt.includes(g));
  const vl=kolonie?[['mindest','Mindestfracht'],['aufbau','Aufbaupaket'],['voll','Laderaum füllen'],['leer','Leeren']]:z.mission==='kampfkolonisieren'?[['aufbau','Wiederaufbau'],['voll','Laderaum füllen'],['leer','Leeren']]:z.versorgen?[['heimflug','Treibstoff für den Heimflug'],['rohstoffe','Ganzer Bestand Erz, Kristall, Deuterium'],['leer','Leeren']]:z.mission==='transport'?[['aufbau','Versorgungspaket 50.000'],['rohstoffe','Ganzer Bestand Erz, Kristall, Deuterium'],['voll','Laderaum füllen'],['leer','Leeren']]:[['voll','Laderaum voll · wertvollste zuerst'],['rohstoffe','Rohstoffe'],['bauteile','Bauteile (Antriebskerne, Habitatmodule)'],['leer','Leeren']];
  // Balken am Laderaum (Karl 24.10.: am Planetenbestand gemessen war die Fracht bei Millionen auf Lager unsichtbar).
  return `<section class="es-teil"><h3 class="es-teil-titel">Rohstoffe${kolonie?' – Startkapital der Kolonie':''} ${infoKnopf('laderaum')}</h3>
   <div class="es-kapazitaet"><div><b>Fracht ${zahl(f)}</b> / ${zahl(lr)} Laderaum${f>lr?` · <span class="es-schlecht">${zahl(f-lr)} zu viel</span>`:` · ${zahl(Math.max(0,lr-f))} frei`}</div>
    ${balken('es-kap',[{wert:Math.min(f,lr||f),cls:f>lr?'zuviel':'voll',titel:'Fracht'}],Math.max(lr,f,1))}
    ${f>lr?'<button type="button" class="es-mini" data-es-frachter>Frachter dazunehmen</button>':''}${z.flug?`<small>Treibstoff ${zahl(treibstoff()*spritFaktor())} Deuterium kommt zusätzlich vom Startort – er belegt keinen Laderaum.</small>`:''}</div>
   <div class="es-vorlagen">${vl.map(([k,t])=>chip('data-es-fracht',k,t,z.frachtArt===k)).join('')}</div>
   ${z.mission==='kampfkolonisieren'&&z.frachtArt==='aufbau'&&z.wiederaufbau?`<p class="es-leise">${z.wiederaufbau.kosten?`Reparatur der ${zahl(z.wiederaufbau.gebaeude)} ausgespähten Gebäude (Bericht vor ${zahl(Math.max(0,((V()?.sekunden||0)-(z.wiederaufbau.zeit||0))/3600))} h): ${h(Object.entries(z.wiederaufbau.kosten).filter(([,n])=>n>0).map(([g,n])=>zahl(Math.ceil(n))+' '+name(g)).join(', '))} – geladen, soweit Laderaum und Lager reichen.`:h(z.wiederaufbau.fehler)}</p>`:''}
   ${z.versorgen&&z.frachtArt==='heimflug'&&z.heimflugBedarf?`<p class="es-leise">Heimflug der Flotte nach ${h(planetName(z.heimflugBedarf.heim||z.start))}: ${zahl(Math.round(z.heimflugBedarf.strecke*10)/10)} Deuterium bei vollem Tempo, mit 10 % Reserve ${zahl(z.heimflugBedarf.mitReserve??z.heimflugBedarf.noetig)}${(()=>{const fehlt=Math.max(0,(z.heimflugBedarf.mitReserve??0)-(z.heimflugBedarf.bord??0)-(Number(z.ladung.deuterium)||0));return fehlt>0?` – es fehlen noch ${zahl(fehlt)}`:' – reicht mit dieser Ladung';})()}.${z.heimflugBedarf.noetig>(Number(z.ladung.deuterium)||0)?` Am Start ${h(planetName(z.start))} sind nur ${zahl(frachtMax('deuterium'))} Deuterium frei – im Schritt „Flotte“ einen anderen Startort wählen.`:''}</p>`:''}
   ${gutListe(haupt)}${weitere.length?`<details class="es-klapp" data-es-klapp="gueter"${z.offen.gueter?' open':''}><summary>Weitere Güter (${weitere.length}) <small>${h(weitere.map(name).join(', '))}</small></summary>${gutListe(weitere)}</details>`:''}</section>`;
 }
 function gutListe(liste){
  const lr=laderaum(),kolonie=z.mission==='kolonisieren';
  return `<div class="es-gueter">${liste.map(g=>{const w=Number(z.ladung[g])||0,min=Math.ceil(Number(z.bedarf?.fracht?.[g])||0),max=frachtMax(g);
   const info=kolonie&&min?`mind. ${zahl(min)}${w<min?' · fehlt '+zahl(min-w):' ✓'}`:w&&lr?`${zahl(Math.round(w/Math.max(lr,1)*100))} % Laderaum`:'';
   return `<div class="es-gut${w>0?' gewaehlt':''}${kolonie&&w<min?' zuwenig':''}" id="es-gut-${h(g)}">${typeof imageTag==='function'?imageTag('resources',g,''):'<span></span>'}<div class="es-gut-name"><b>${h(name(g))}</b><small>${zahl(bestand(g))} am Start</small></div><small class="es-gut-info">${h(info)}</small>
    ${stepper('gut',g,w,max,name(g))}${balken('es-gut-balken',[{wert:w,cls:'mit',titel:`${zahl(w)} im Laderaum`}],Math.max(lr,1))}</div>`;}).join('')}</div>`;
 }
 const mondFlug=()=>z?.zielart==='mond'||z?.startart==='mond';
 function personenHtml(){
  const p=startOrt();if(!p?.personengruppen||z.mission==='kolonisieren')return '';
  if(mondFlug())return '<p class="es-leise">Monde haben keine Bevölkerung: Personen und Bodentruppen starten und landen nur auf Planeten.</p>';
  const pp=personenplaetze(),g=personenGewaehlt(),voelker=voelkerAm(p);if(!voelker.length)return '';
  const vorlage=pp?`<div class="es-vorlagen">${chip('data-es-personen','bewohner','Plätze mit Bewohnern füllen',false)}${chip('data-es-personen','fach','Erst Fachkräfte und Forscher',false)}${chip('data-es-personen','leer','Keine Personen',false)}</div>`:'';
  return `<details class="es-klapp es-teil" data-es-klapp="personen"${z.offen.personen||g?' open':''}><summary>Personen mitnehmen <small>${g?`${zahl(g)} gewählt · `:''}${pp?`${zahl(pp)} Personenplätze`:'keine Personenplätze in dieser Flotte'}</small></summary>
   ${vorlage}${voelker.map(v=>{const ab=personenAm(p,v),i=VOELKER.indexOf(v);return `<div class="es-personen"><h4>${h(name(v))}</h4>${QUALI.map((q,j)=>`<div class="es-person es-${q.k}"><div><b>${h(q.n)}</b><small>${zahl(ab[j])} da · ${h(q.e)}</small></div>${stepper('person',`${i}:${j}`,z.personen[i][j],ab[j],q.n)}</div>`).join('')}</div>`;}).join('')}
   ${g>pp?`<div class="es-kapazitaet"><span class="es-schlecht">${zahl(g)} Personen, aber nur ${zahl(pp)} Plätze.</span><button type="button" class="es-mini" data-es-personenplaetze>Schiffe mit Personenplätzen dazunehmen</button></div>`:''}</details>`;
 }
 function truppenHtml(pflicht){
  if(mondFlug()&&!pflicht)return '';
  const d=truppenDaten(),best=d?.bestand||[],liste=best.map((n,i)=>[i,n]).filter(([,n])=>n>0);
  if(!liste.length)return pflicht?`<section class="es-teil"><h3 class="es-teil-titel">Bodentruppen</h3><p class="es-schlecht">Keine Bodentruppen am Startort. In der Kaserne ausbilden.</p><button type="button" class="es-mini" data-es-go="truppen">Zur Kaserne</button></section>`:'';
  const tl=truppenPlaetze(),tk=truppenKapazitaet(),gesamt=z.truppen.reduce((a,b)=>a+(Number(b)||0),0);
  const inhalt=`<div class="es-kapazitaet"><div>Truppenplätze <b>${zahl(tl)}</b> / ${zahl(tk)}${tl>tk?` · <span class="es-schlecht">${zahl(tl-tk)} zu viel</span>`:''}</div>${balken('es-kap',[{wert:Math.min(tl,tk||tl),cls:tl>tk?'zuviel':'voll',titel:'belegt'}],Math.max(tk,tl,1))}
    ${tl>tk&&da('truppentransporter')>(z.schiffe.truppentransporter||0)?'<button type="button" class="es-mini" data-es-truppentransporter>Truppentransporter dazunehmen</button>':''}</div>
   <div class="es-vorlagen">${chip('data-es-truppen','alle','Alle Truppen',false)}${chip('data-es-truppen','passend','Passend für die Transporter',false)}${chip('data-es-truppen','keine','Keine',false)}</div>
   <div class="es-truppen">${liste.map(([i,n])=>`<div class="es-trupp" id="es-trupp-${i}">${typeof imageTag==='function'?imageTag('ground_units',d.bilder?.[i]||d.typen?.[i],V()?.volk||''):''}<div class="es-trupp-name"><b>${h(d.namen?.[i]||d.typen?.[i])}</b><small>${zahl(n)} da · ${zahl(d.truppenplaetze?.[i]||1)} ${(d.truppenplaetze?.[i]||1)===1?'Platz':'Plätze'} je Einheit</small></div>${stepper('trupp',String(i),z.truppen[i]||0,n,d.namen?.[i]||'Truppen')}</div>`).join('')}</div>`;
  if(pflicht)return `<section class="es-teil"><h3 class="es-teil-titel">Bodentruppen ${infoKnopf('truppen')}</h3>${z.mission==='kampfkolonisieren'?'<p class="es-leise">Ohne Bodentruppen keine Landung. Die Truppen kämpfen gegen Garnison und Miliz.</p>':''}${inhalt}</section>`;
  return `<details class="es-klapp es-teil" data-es-klapp="truppen"${z.offen.truppen||gesamt?' open':''}><summary>Bodentruppen mitnehmen <small>${gesamt?`${zahl(gesamt)} gewählt`:z.mission==='kolonisieren'?'werden Garnison der neuen Kolonie':'optional'}</small></summary>${inhalt}</details>`;
 }
 function siedlerHtml(){
  const p=startOrt(),pl=siedlerPlaetze(),gesamt=Math.floor(Number(p?.bevoelkerung)||0);
  if(ohneGruppen())return `<section class="es-teil"><h3 class="es-teil-titel">Siedler</h3><p>${zahl(pl)} Siedler fliegen mit, ${zahl(Math.max(0,gesamt-pl))} bleiben auf [${h(z.start)}].</p></section>`;
  const ab=personenAm(p,z.volk),m=z.mischung,voelker=voelkerAm(p),summe=m[0]+m[1]+m[2];
  const vl=[['bewohner','Nur Bewohner'],['fachkraefte','Nur Fachkräfte'],['forscher','Nur Forscher'],['ausgewogen','Ausgewogen']];
  return `<section class="es-teil"><h3 class="es-teil-titel">Siedler <small>${zahl(summe)} / ${zahl(pl)} Plätze</small></h3>
   ${voelker.length>1?`<div class="es-vorlagen" role="group" aria-label="Herkunft">${voelker.map(v=>chip('data-es-volk',v,name(v),v===z.volk,zahl(personenAm(p,v).reduce((a,b)=>a+b,0)))).join('')}</div>`:''}
   <div class="es-vorlagen" role="group" aria-label="Schnellwahl Siedler">${vl.map(([k,t])=>chip('data-es-mischung',k,t,z.mischungArt===k)).join('')}</div>
   ${z.siedlerHinweis?`<p class="es-hinweis">${h(z.siedlerHinweis)}</p>`:''}
   ${balken('es-bev',QUALI.map((q,i)=>({wert:m[i],cls:'es-'+q.k,titel:`${q.n}: ${zahl(m[i])}`})),Math.max(pl,1))}
   <div class="es-qualis">${QUALI.map((q,i)=>`<div class="es-quali es-${q.k}"><div class="es-quali-kopf"><b>${h(q.n)}</b><span><b>${zahl(m[i])}</b> mit · ${zahl(ab[i]-m[i])} bleiben</span></div><input type="range" min="0" max="${Math.min(ab[i],pl)}" step="10" value="${m[i]}" data-es-regler="${i}" aria-label="${h(q.n)} mitnehmen"><small>${h(q.e)} · ${zahl(ab[i])} auf dem Planeten</small></div>`).join('')}</div>
   <p class="es-leise">${zahl(Math.max(0,gesamt-pl))} Einwohner bleiben auf [${h(z.start)}] (mindestens ${zahl(REST_EINWOHNER)}).</p></section>`;
 }
 function dauerHtml(){
  const p=P[z.mission],d=p?.dauer;if(!d)return '';const max=d.max(),w=z.stunden??d.std;
  let ertrag='';if(z.mission==='abbau'){const n=z.schiffe.bergbauschiff||0,fl=R()?.catalog?.flug||{};const roh=abbauRoh(n,w,fl),lr=laderaum();
   ertrag=`<div class="es-lagezahlen"><div class="es-lagezahl${roh>lr?' warn':''}"><b>${zahl(Math.min(roh,lr||roh))}</b><span>Ertrag ≈ (Erz + Kristall)${roh>lr?` – Laderaum ${zahl(lr)} begrenzt, ${zahl(roh)} wären möglich`:''}</span></div><div class="es-lagezahl"><b>${zahl(n)}</b><span>Bergbauschiffe × ${zahl(w)} h</span></div></div>`;}
  return `<section class="es-teil"><h3 class="es-teil-titel">${h(HALT_TITEL[z.mission]||'Dauer')} <small>${zahl(d.min)}–${zahl(max)} Spielstunden</small></h3>
   <div class="es-vorlagen">${d.schritte.filter(s=>s<=max).map(s=>chip('data-es-stunden',String(s),s===0?'sofort zurück':`${s} h`,w===s)).join('')}</div>
   <label class="es-regler-zeile"><input type="range" min="${d.min}" max="${max}" step="1" value="${w}" data-es-stundenregler aria-label="Stunden"><b>${zahl(w)} h</b></label>${ertrag}</section>`;
 }
 function detailsHtml(){
  const m=z.mission,p=P[m];if(!p?.details)return '';
  if(p.details==='kolonie')return `${siedlerHtml()}${frachtHtml()}${truppenHtml(false)}`;
  if(p.details==='truppen')return m==='kampfkolonisieren'&&V()?.invasion_v2===true?`${truppenHtml(true)}${frachtHtml()}`:truppenHtml(true);
  if(p.details==='dauer')return dauerHtml();
  return `${frachtHtml()}${personenHtml()}${truppenHtml(false)}${p.dauer?dauerHtml():''}`;
 }

 // ---------------------------------------------------------------- Erklärungen (Info-Symbole → Unterseite)
 // Karl 24.10.2026: „selbsterklärend, aber ausreichend Infos über Icons, um sich zu informieren – diese Unterseiten müssen
 // zwingend auch gut sein“. Ein ⓘ öffnet eine Unterseite im Blatt; Zurück schließt sie zuerst. Inhalte folgen dem Kern.
 const INFO={
  tempo:{titel:'Tempo und Treibstoff',teile:[
   ['Wie schnell?','Die Flotte fliegt so schnell wie ihr langsamstes Schiff. 100 % ist dessen Höchsttempo; jede Antriebsforschung macht die passenden Schiffe schneller.'],
   ['Flugzeit','Etwa umgekehrt zum Tempo: 50 % braucht rund doppelt so lange. Kürzer als 20 Spielminuten ist kein Flug.'],
   ['Treibstoff','Wächst mit dem Quadrat des Tempos: 50 % braucht nur etwa ein Viertel. Jede Raumhafen-Stufe spart 4 %, höchstens 40 %.'],
   ['Hin und zurück','Das Rückflugtempo stellst du eigens ein – langsam heim spart Deuterium, schnell heim schützt die Flotte. Beide Strecken werden beim Start bezahlt.'],
   ['Am Ziel','Angriff, Spionage, Recyceln, Transport: sofort zurück. Halten, Abbau, Saven, Bodenhilfe: nach der gewählten Dauer. Blockade, Bombardieren, Kampfkolonisieren: erst nach deinem Rückruf. Stationieren und Kolonisieren: kein Rückflug, nur der Hinflug kostet Treibstoff.']]},
  fahrplan:{titel:'Fahrplan',teile:[
   ['Uhrzeiten','In Spielzeit wie die Ereignisliste: „Tag 5 · 14:32:10“. Sie gelten, sobald die Flotte fliegt.'],
   ['Aufenthalt','Halten, Abbau, Saven und Bodenhilfe bleiben die gewählte Dauer (Bodenhilfe länger, solange eine Belagerung läuft). Blockade und Bombardement halten den Orbit bis zum Rückruf. Ein Angriff kehrt nach dem Kampf sofort um.'],
   ['Rückkehr','Kommt die Flotte früher zurück (Rückruf, Kampf ohne Sieger), fliegt sie mit dem gewählten Rückflugtempo.']]},
  termin:{titel:'Ankunft planen und koppeln',teile:[
   ['Ankunft planen','Du wählst die Ankunft, der Kern startet die Flotte automatisch rechtzeitig. Abflug höchstens 7 Spieltage voraus, höchstens 20 geplante Flüge. Schiffe und Vorräte prüft er beim Abflug erneut.'],
   ['Ankunft koppeln','Die Flotte kommt eine feste Zeit nach einer eigenen Flotte oder einem Verband im selben System an – etwa Recycler 5 s nach dem Einschlag.'],
   ['Wichtig','Verbände brauchen einen sofortigen Start.']]},
  verband:{titel:'Verbandsangriff',teile:[
   ['Gemeinsam','Mehrere Flotten greifen dasselbe Ziel zur selben Zeit an. Die gemeinsame Ankunft ist die späteste.'],
   ['Grenzen','Höchstens 16 Flotten; fremde Teilnehmer nur aus derselben Allianz; keine Flotte wird mehr als 6 Stunden gegenüber ihrer eigenen Ankunft verzögert.'],
   ['Vertrauen','Ruft ein Partner zurück, verlässt seine Flotte den Verband – flieg nur mit Partnern, denen du vertraust.']]},
  laderaum:{titel:'Laderaum',teile:[
   ['Was zählt','Laderaum aller Schiffe × Volksfaktor × (1 + 5 % je Logistik-Stufe). Der Treibstoff belegt keinen Laderaum – er wird vom Startort abgebucht.'],
   ['Recyceln','Beim Recyceln zählt nur der Laderaum von Recyclern und Bergungsschiffen.'],
   ['Personen','Personen brauchen eigene Plätze (Bergungsschiff 200, Handelsfrachter 20, Großer Transporter 10). Freier Laderaum schafft keine Sitze.']]},
  truppen:{titel:'Bodentruppen verladen',teile:[
   ['Plätze','Ein Truppentransporter trägt 100 Plätze. Infanterie braucht 1–2, Panzer 10–15 Plätze je Einheit.'],
   ['Garnison','Bei Kolonisieren und Stationieren werden die Truppen Garnison am Ziel. Bei Kampfkolonisieren kämpfen sie nach der Landung gegen Garnison und Miliz.']]},
  uebergabe:{titel:'Flottenübergabe',teile:[
   ['Ablauf','Du sendest ein Angebot. Der Empfänger nimmt es im Flottenkommando an (bis 1 Spieltag). Dann fliegen die Schiffe mit allem an Bord los und gehören bei Ankunft ihm.'],
   ['Geschenk oder Verkauf','Preis 0 ist ein Geschenk. Beim Verkauf liegen die Credits bis zur Zustellung sicher; bei Rückruf, Verlust oder Besitzerwechsel am Ziel gehen sie zurück.'],
   ['Laden','Alle Güter, auch Antriebskerne und Habitatmodule, dazu Bewohner, Fachkräfte, Forscher und Bodentruppen. Treibstoff zahlt der Startort extra.'],
   ['Grenzen','Höchstens 20 offene Angebote; auch ohne Bündnis möglich.']]},
  aufklaerung:{titel:'Aufklärungspflicht',teile:[
   ['System','Abbau, Recyceln und Ausspähen brauchen ein mit einer Sonde kartiertes System.'],
   ['Ziel','Jedes fremde oder freie Ziel – auch ein freier Platz vor dem Kolonisieren – braucht vorher eine eigene Sonde darauf. Ausnahmen: Erkunden, Ausspähen, Saven, Abbau, Recyceln, Hilfe bei Verbündeten.'],
   ['Ein Tipp','„1 Sonde schicken“ erledigt das direkt im Blatt; die Bedingung wird grün, sobald der Bericht da ist.']]}
 };
 function infoKnopf(key,label){return `<button type="button" class="es-info" data-es-info="${h(key)}" aria-label="Erklärung: ${h(label||INFO[key]?.titel||key)}">i</button>`;}
 function infoHtml(){
  const k=z.info;if(!k)return '';let titel='',teile=[],gruppe=z.mission?A()?.info(z.mission).gruppe:'',symbol='',waehlen='';
  if(k.startsWith('mission:')){const m=k.slice(8),p=P[m],a=A()?.info(m)||{};titel=a.name||m;gruppe=a.gruppe;symbol=badge(m,{size:30});
   const bed=KOORD.test(z.ziel)?bedingungen(m):[];
   teile=[['Wofür',a.zweck||''],['So läuft es ab',`<ol class="es-ablauf">${(p?.ablauf||[]).map((t,i)=>`<li><span class="es-ablauf-nr">${i+1}</span><span>${h(t)}</span></li>`).join('')}</ol>`,true],['Das zählt',`<ul class="es-info-liste">${(p?.zaehlt||[]).map(t=>`<li>${h(t)}</li>`).join('')}</ul>`,true]];
   if(bed.length)teile.push([`Für [${z.ziel}]`,`<ul class="es-bedingungen">${bed.map(x=>`<li class="${x.ok?'ok':''}"><span class="es-bed-zeichen" aria-hidden="true">${x.ok?'✓':'!'}</span><span>${h(x.text)}</span></li>`).join('')}</ul>`,true]);
   if(z.schritt==='auftrag'&&bed[0]?.ok)waehlen=`<button type="button" class="es-weiter es-starten es-us-waehlen" data-es-mission="${h(m)}">${h(a.name||m)} wählen</button>`;}
  else{const e=INFO[k];if(!e)return '';titel=e.titel;teile=e.teile;}
  return `<div class="es-unterseite${gruppe?' auftrag-'+h(gruppe):''}" role="dialog" aria-modal="false" aria-label="${h(titel)}"><div class="es-us-kopf"><button type="button" class="es-us-zurueck" data-es-info-zu aria-label="Zurück">‹</button>${symbol}<h3>${h(titel)}</h3></div><div class="es-us-inhalt">${teile.map(([t,text,html])=>`<section><h4>${h(t)}</h4>${html?text:`<p>${h(text)}</p>`}</section>`).join('')}</div><div class="es-us-fuss">${waehlen}<button type="button" class="es-mini es-us-fertig" data-es-info-zu>${waehlen?'Zurück zur Auswahl':'Verstanden'}</button></div></div>`;
 }
 // ---------------------------------------------------------------- Schritt ⑤ Zeit & Start
 // Karl 24.10.2026: „Hin- und Rückflug als coole Auswahl, dahinter je Stufe wie viel weniger Treibstoff in % und die genaue
 // Zeit; eingestellt sieht man in der Vorschau genau, wann die Schiffe am Ziel ankommen und wann sie wieder zu Hause sind.“
 const TEMPI=[1,0.9,0.8,0.7,0.6,0.5,0.4,0.3,0.2,0.1];
 const zeitText=s=>s==null||!Number.isFinite(Number(s))?'–':window.Bauseite?.dauer?Bauseite.dauer(Number(s)):dauer(Number(s));
 const spielJetzt=()=>typeof GameTime!=='undefined'&&GameTime.now?GameTime.now():(V()?.sekunden||0);
 const uhrzeit=t=>t<0?'vor Spielbeginn':'Tag '+(Math.floor(t/86400)+1)+' · '+[Math.floor(t/3600)%24,Math.floor(t/60)%60,Math.floor(t)%60].map(n=>String(n).padStart(2,'0')).join(':');
 // Tempotabelle: vom Kern (flugzeit.tempi) oder – falls ältere Server – zehn Einzelabfragen, nur im Start-Schritt.
 function tempiListe(){
  const roh=Array.isArray(z.flug?.tempi)&&z.flug.tempi.length?z.flug.tempi:z.tempiErsatz;
  if(!roh)return null;
  return TEMPI.map(g=>{const t=roh.find(x=>Math.abs(Number(x.geschwindigkeit)-g)<1e-6);return t?{g,dauer:Number(t.dauer_sekunden),sprit:Math.ceil(Number(t.treibstoff_je_strecke_milli??(Number(t.treibstoff_je_strecke)||0)*M)/M)}:null;}).filter(Boolean);
 }
 // Uhrzeit ohne Tag, wenn es heute ist (kürzer in der Liste); sonst mit Spieltag.
 const uhrKurz=s=>Math.floor(s/86400)===Math.floor(spielJetzt()/86400)?uhrzeit(s).split(' · ')[1]:uhrzeit(s);
 const uhrMin=s=>uhrKurz(s).replace(/:\d\d$/,'');
 const tempoDaten=g=>tempiListe()?.find(t=>Math.abs(t.g-g)<1e-6)||null;
 // Terraformen, bei dem alle Schiffe verbraucht werden: kein Heimflug; der Kern bucht beim Start beide Strecken und
 // erstattet den Rückflug-Treibstoff bei Ankunft (terraform.rs).
 function keinHeimflug(){if(z.mission!=='terraformen'||!z.tf)return false;const n=Number(z.schiffe.terraformer)||0,noetig=Number(z.tf.schiffe_voll)||0;return noetig>0&&n<=noetig;}
 function aufenthalt(){const p=P[z.mission];if(!p?.dauer)return 0;return (z.stunden??p.dauer.std)*3600;}
 // Fahrplan: Abflug, Ankunft am Ziel, Aufenthalt, Rückkehr – in Spielzeit wie die Ereignisliste.
 function fahrplan(){
  const hin=tempoDaten(z.tempo),rueck=P[z.mission]?.rueck?tempoDaten(z.ruecktempo):null,jetzt=spielJetzt();
  if(!hin)return null;
  let ankunft=jetzt+hin.dauer,abflug=jetzt;
  if(z.ankunftArt==='uhrzeit'&&z.ankunftZeit!=null)ankunft=z.ankunftZeit;
  if(z.ankunftArt==='kopplung'){const a=kopplungsAnker().find(x=>x.id===z.kopplung?.anker);if(a)ankunft=a.an+(z.kopplung.versatz||0);}
  if(z.ankunftArt!=='sofort')abflug=ankunft-hin.dauer;
  const offen=aufenthaltArt()==='offen',ohne=keinHeimflug(),halt=aufenthalt(),rueckStart=offen?null:ankunft+halt,zurueck=rueck&&!offen&&!ohne?rueckStart+rueck.dauer:null;
  const reserve=z.mission==='uebergeben'&&z.startart!=='flotte'?hin.sprit:0;
  return {jetzt,abflug,ankunft,halt,rueckStart,zurueck,offen,ohne,erstattet:ohne&&rueck?rueck.sprit:0,hin,rueck,reserve,sprit:hin.sprit+(rueck?rueck.sprit:0)+reserve};
 }
 function tempoWahl(art){
  const liste=tempiListe(),wahl=art==='hin'?z.tempo:z.ruecktempo,d=tempoDaten(wahl),voll=tempoDaten(1),offen=z.offen.tempowahl===art,f=fahrplan();
  const ersparnis=t=>voll&&voll.sprit>0?Math.round((1-t.sprit/voll.sprit)*100):0;
  // Bei geplanter/gekoppelter Ankunft zeigt jede Hinflugstufe ihren Abflug (Ankunft − Dauer) – Funktionsprüfung 25.10.2026.
  const ziel=t=>art==='hin'?(z.ankunftArt==='sofort'?f&&(f.jetzt+t.dauer):f&&f.ankunft!=null?f.ankunft-t.dauer:null):f&&f.rueckStart!=null?f.rueckStart+t.dauer:null;
  // Karl 24.10.2026: Jede Option zeigt, was sie kostet (Deuterium dieser Flotte); wie viel weniger steht nur als Prozent –
  // nie ein Minus-Betrag (DESIGNREGELN §9 D7: Endwerte statt Differenzen).
  const spar=t=>{const e=ersparnis(t);return `${e>0?`<em class="spart">−${e} %</em>`:'<em>volles Tempo</em>'} · <strong>${zahl(t.sprit)} Deuterium</strong>`;};
  const zeile=t=>{const zt=ziel(t),e=ersparnis(t);return `<button type="button" role="option" class="es-tw-option" aria-selected="${Math.abs(t.g-wahl)<1e-6}" data-es-tempo-${art}="${t.g}"><b>${Math.round(t.g*100)} %</b><span class="es-tw-dauer">${h(zeitText(t.dauer))}</span><small class="es-tw-an">${zt!=null?(art==='hin'?(z.ankunftArt==='sofort'?'an ':(f&&zt<f.jetzt?'Abflug vorbei · ':'ab ')):'zurück ')+h(uhrKurz(zt)):art==='rueck'&&f?.offen?'ab Rückruf':''}</small><span class="es-tw-spar">${spar(t)}</span></button>`;};
  const titel=art==='hin'?'Hinflug':aufenthaltArt()==='offen'?'Heimflug nach Rückruf':z.mission==='terraformen'?'Rückflug der übrigen':'Rückflug';
  // Karl 24.10.2026: „unter 10 % ‚kein Rückflug‘ – dann bleibt die Flotte da stehen, es gelten die Regeln für gestrandet“.
  const keinRueck=art==='rueck'&&orbitWelt()&&z.mission!=='uebergeben';
  const keinZeile=keinRueck?`<button type="button" role="option" class="es-tw-option es-tw-kein" aria-selected="${wahl===0}" data-es-tempo-rueck="0"><b>–</b><span class="es-tw-dauer">kein Rückflug</span><small class="es-tw-an">bleibt am Ziel</small><span class="es-tw-spar"><em class="spart">−100 %</em> · <strong>0 Deuterium</strong> · danach gestrandet</span></button>`:'';
  return `<div class="es-tw${offen?' offen':''}"><button type="button" class="es-tw-knopf" data-es-tw-auf="${art}" aria-expanded="${offen}" aria-haspopup="listbox"><span class="es-tw-titel">${titel}</span><b>${art==='rueck'&&wahl===0?'–':Math.round(wahl*100)+' %'}</b>
   <span class="es-tw-wert">${art==='rueck'&&wahl===0?'kein Rückflug · bleibt am Ziel gestrandet · <strong>0 Deuterium</strong>':d?`${h(zeitText(d.dauer))} · ${spar(d)}`:'Flugzeit wird berechnet …'}</span><i class="es-tw-pfeil" aria-hidden="true"></i></button>
   ${offen?`<div class="es-tw-liste" role="listbox" aria-label="${titel}: Tempo wählen">${liste?liste.map(zeile).join(''):'<p class="es-leise">Flugzeiten werden berechnet …</p>'}${art==='rueck'?keinZeile:''}<p class="es-tw-hinweis">Treibstoff wächst mit dem Quadrat des Tempos: halbes Tempo braucht etwa ein Viertel. Die Flugzeit bleibt mindestens 20 Minuten.</p></div>`:''}</div>`;
 }
 // Aufenthalt als eigene Auswahl (Halten, Abbau, Saven, Bodenhilfe): jede Stufe zeigt, wann der Heimflug beginnt und wann
 // die Flotte zu Hause ist; Feinwert per Regler in derselben Liste.
 function haltWahl(){
  const d=P[z.mission]?.dauer;if(!d)return '';const max=d.max(),w=z.stunden??d.std,offen=z.offen.tempowahl==='halt',f=fahrplan(),rueck=tempoDaten(z.ruecktempo);
  const titel=HALT_TITEL[z.mission]||'Dauer',werte=[...new Set([...d.schritte.filter(s=>s<=max),w])].sort((a,b)=>a-b);
  const text=s=>`${zahl(s)} h`;
  const ab=s=>f?f.ankunft+s*3600:null,heim=s=>ab(s)!=null&&rueck?ab(s)+rueck.dauer:null;
  const extra=s=>{if(z.mission!=='abbau')return z.mission==='verstaerken'?'länger, solange die Belagerung läuft':'';const n=z.schiffe.bergbauschiff||0,fl=R()?.catalog?.flug||{},roh=abbauRoh(n,s,fl),lr=laderaum();return `Ertrag ≈ <strong>${zahl(lr?Math.min(roh,lr):roh)}</strong>${lr&&roh>lr?' (Laderaum voll)':''}`;};
  const zeile=s=>`<button type="button" role="option" class="es-tw-option" aria-selected="${s===w}" data-es-halt="${s}"><b>${h(text(s))}</b><span class="es-tw-dauer">${heim(s)!=null?'zurück '+h(uhrKurz(heim(s))):''}</span><small class="es-tw-an"></small><span class="es-tw-spar">${ab(s)!=null?(s===0?'kehrt sofort um':'am Ziel bis '+h(uhrKurz(ab(s)))):''}${extra(s)?' · '+extra(s):''}</span></button>`;
  return `<div class="es-tw es-tw-halt${offen?' offen':''}"><button type="button" class="es-tw-knopf" data-es-tw-auf="halt" aria-expanded="${offen}" aria-haspopup="listbox"><span class="es-tw-titel">Am Ziel · ${h(titel)}</span><b>${h(text(w))}</b>
   <span class="es-tw-wert">${ab(w)!=null?(w===0?'kehrt sofort um':`bleibt bis ${h(uhrKurz(ab(w)))}`):h(AM_ZIEL[z.mission]||'')}${extra(w)?' · '+extra(w):''}</span><i class="es-tw-pfeil" aria-hidden="true"></i></button>
   ${offen?`<div class="es-tw-liste" role="listbox" aria-label="${h(titel)} wählen">${werte.map(zeile).join('')}<label class="es-regler-zeile es-tw-regler"><input type="range" min="${d.min}" max="${max}" step="1" value="${w}" data-es-stundenregler aria-label="${h(titel)} in Stunden"><b>${zahl(w)} h</b></label><p class="es-tw-hinweis">${zahl(d.min)}–${zahl(max)} Spielstunden${z.mission==='saven'?' – solange die Flotte unterwegs ist, kann sie am Planeten nicht zerstört werden':''}.</p></div>`:''}</div>`;
 }
 function amZielHtml(){
  const art=aufenthaltArt();if(art==='dauer')return haltWahl();
  const titel={kurz:'Am Ziel',offen:'Am Ziel · bis zum Rückruf',einweg:'Am Ziel · Einweg',uebergabe:'Am Ziel · Übergabe'}[art];
  const unter={kurz:'',offen:'Rückruf jederzeit im Flottenkommando; der Heimflug ist schon bezahlt. Verliert sie den Kampf, kehrt sie sofort um.',einweg:'Kein Rückflug – nur der Hinflug kostet Treibstoff.',uebergabe:'Treibstoff: Hinflug plus Reserve nach Transportregel.'}[art];
  let haupt=AM_ZIEL[z.mission]||'',zusatz=unter;
  if(z.mission==='terraformen'&&z.tf){const n=Number(z.schiffe.terraformer)||0,noetig=Number(z.tf.schiffe_voll)||0,weg=Math.min(n,noetig),heim=n-weg;
   haupt=noetig<=0?`Nichts mehr zu formen – alle ${zahl(n)} Terraformer fliegen heim`:heim>0?`${zahl(weg)} Terraformer werden verbraucht, ${zahl(heim)} fliegen heim`:`Alle ${zahl(n)} Terraformer werden verbraucht`;
   zusatz=heim>0||noetig<=0?'':'Kein Rückflug: der mitbezahlte Rückflug-Treibstoff wird bei Ankunft erstattet.';}
  // „Kein Rückflug“ gewählt: nach dem Auftrag bleibt die Flotte im Orbit des Ziels (gestrandet).
  if(P[z.mission]?.rueck&&z.ruecktempo===0&&z.mission!=='terraformen'){
   if(art==='offen')zusatz='Kein Rückflug: nach dem Rückruf bleibt sie im Orbit – gestrandet, weiter nur mit Deuterium aus der Ladung.';
   else{haupt=(AM_ZIEL[z.mission]||'Auftrag').replace(/,? ?(dann )?(sofort |leer )?zurück.*$/,'')+', dann bleibt sie im Orbit – gestrandet';zusatz='Weiter nur mit Deuterium aus der Ladung oder nach einer Versorgung.';}}
  return `<div class="es-zp-ziel es-zp-${art}"><span class="es-tw-titel">${h(titel)}</span><b>${h(haupt)}</b>${zusatz?`<small>${h(zusatz)}</small>`:''}</div>`;
 }
 function zeitplanHtml(){
  const p=P[z.mission]||{},linie='<i class="es-zp-linie" aria-hidden="true"></i>';
  const warn=p.rueck&&z.ruecktempo===0?`<p class="es-plan warn">Kein Rückflug: Nach dem Auftrag bleibt die Flotte im Orbit von [${h(z.ziel)}] – gestrandet, für alle Kenner des Systems sichtbar und angreifbar. Weiter nur mit Deuterium aus ihrer Ladung oder nach einer Versorgung.</p>`:'';
  return `<section class="es-teil"><h3 class="es-teil-titel">Zeitplan ${infoKnopf('tempo')}</h3><div class="es-zp">${tempoWahl('hin')}${linie}${amZielHtml()}${p.rueck&&!keinHeimflug()?linie+tempoWahl('rueck'):''}</div>${warn}</section>`;
 }
 function kopplungsAnker(){
  const v=V()||{},sys=systemVon(z.ziel),out=[];
  for(const f of v.flotten||[])if(f.zustand==='hinflug'&&systemVon(f.ziel)===sys)out.push({id:'f'+f.flotte,an:f.ankunft_sekunden,text:`Flotte ${f.flotte} · ${A()?.info(f.mission).name||f.mission} → [${f.ziel}]`});
  for(const b of v.verbaende||[])if(systemVon(b.ziel)===sys)out.push({id:'v'+b.fuehrung,an:b.ankunft_sekunden,text:`Verband ${b.spieler} → [${b.ziel}] · ${b.flotten} Flotten`});
  return out.filter(a=>Number.isFinite(a.an));
 }
 function ankunftHtml(){
  if(z.mission==='uebergeben'||z.startart==='flotte')return '';
  const anker=kopplungsAnker(),f=fahrplan(),arten=[['sofort','Sofort starten','so früh wie möglich ankommen'],['uhrzeit','Ankunft planen','zu einer Spielzeit, Abflug automatisch'],...(anker.length?[['kopplung','Ankunft koppeln','an eine Flotte im selben System']]:[])];
  const tag=f?Math.floor((z.ankunftZeit??f.ankunft)/86400)+1:1,sek=f?Math.floor((z.ankunftZeit??f.ankunft)%86400):0,uhr=[Math.floor(sek/3600),Math.floor(sek/60)%60,sek%60].map(n=>String(n).padStart(2,'0')).join(':');
  return `<div class="es-teil"><h3 class="es-teil-titel">Ankunft ${infoKnopf('termin')}</h3><div class="es-vorlagen">${arten.map(([k,t,u])=>chip('data-es-ankunft',k,t,z.ankunftArt===k,u)).join('')}</div>
   ${z.ankunftArt==='uhrzeit'?`<div class="es-termin"><label>Spieltag<input type="number" inputmode="numeric" min="1" value="${tag}" data-es-termin-tag></label><label>Uhrzeit<input type="time" step="1" value="${uhr}" data-es-termin-uhr></label><small>Abflug höchstens 7 Spieltage voraus, höchstens 20 geplante Flüge. Schiffe und Vorräte prüft der Kern beim Abflug erneut.</small></div>`:''}
   ${z.ankunftArt==='kopplung'?`<div class="es-kopplung">${anker.map(a=>`<label class="es-wahlzeile${z.kopplung?.anker===a.id?' an':''}"><input type="radio" name="es_kopplung" value="${h(a.id)}" data-es-kopplung ${z.kopplung?.anker===a.id?'checked':''}><span><b>${h(a.text)}</b><small>Ankunft ${h(uhrzeit(a.an))}</small></span></label>`).join('')}
    <div class="es-vorlagen">${[...new Set([0,1,5,10,30,60,z.kopplung?.versatz??5,...(()=>{const fp=fahrplan();return fp&&fp.abflug<fp.jetzt-1?[Math.ceil(fp.jetzt-fp.abflug)+(z.kopplung?.versatz||0)+30]:[];})()])].sort((a,b)=>a-b).map(n=>chip('data-es-versatz',String(n),n?`+${n} s`:'gleichzeitig',(z.kopplung?.versatz??5)===n)).join('')}</div><small class="es-leise">Typisch: Recycler 5 s nach dem Einschlag, Halteflotte kurz davor.</small></div>`:''}</div>`;
 }
 function verbandHtml(){
  if(z.mission!=='angriff'||z.startart==='flotte'||z.zielflotte)return '';
  const v=V()||{},offen=(v.verbaende||[]).filter(b=>b.ziel===z.ziel&&b.beitretbar),f=fahrplan();
  const wirkung=b=>{if(!f)return '';const d=f.ankunft-b.ankunft_sekunden;if(d<=0)return -d>6*3600?'<em class="schlecht">zu früh – über 6 h Wartezeit</em>':`<em class="gut">wartet ${h(zeitText(-d))} auf den Verband</em>`;if(f.ankunft<=b.spaeteste_ankunft_sekunden)return `<em class="warn">verzögert den Verband um ${h(zeitText(d))}</em>`;return '<em class="schlecht">zu langsam – über 6 h Verzögerung</em>';};
  const zeile=(wert,titel,unter,extra='')=>`<label class="es-wahlzeile${String(z.verband||'')===wert?' an':''}"><input type="radio" name="es_verband" value="${h(wert)}" data-es-verband ${String(z.verband||'')===wert?'checked':''}><span><b>${titel}</b><small>${unter}</small>${extra}</span></label>`;
  return `<div class="es-teil"><h3 class="es-teil-titel">Zusammen angreifen ${infoKnopf('verband')}</h3>
   ${zeile('','Allein angreifen',f?`Ankunft ${z.ankunftArt==='sofort'?'≈ ':''}${h(uhrzeit(f.ankunft))}`:'')}
   ${offen.map(b=>zeile(String(b.fuehrung),`Verband ${h(b.spieler)} beitreten`,`${b.flotten}/16 Flotten · ${h((b.teilnehmer||[]).join(', '))} · Ankunft ${h(uhrzeit(b.ankunft_sekunden))}`,wirkung(b))).join('')}
   ${zeile('neu','Neuen Verband eröffnen','Allianzmitglieder können mit Flotten zum selben Ziel beitreten (höchstens 16 Flotten, keine mehr als 6 h verzögert).')}
   ${z.verband&&z.ankunftArt!=='sofort'?'<p class="es-schlecht">Für einen Verband muss die Flotte sofort starten.</p>':''}</div>`;
 }
 function fahrplanHtml(){
  const f=fahrplan();if(!f)return `<p class="es-leise">Fahrplan wird berechnet …</p>`;
  const sp=startOrt(),deut=Math.floor(Number(sp?.bestand?.deuterium)||0),fehlt=f.sprit+(Number(z.ladung.deuterium)||0)>deut;
  const punkt=(cls,titel,zeit,text)=>`<li class="${cls}"><span class="es-fp-punkt" aria-hidden="true"></span><div><b>${titel}</b><span>${zeit}</span>${text?`<small>${text}</small>`:''}</div></li>`;
  const strecke=(text)=>`<li class="es-fp-strecke"><span class="es-fp-linie" aria-hidden="true"></span><div><small>${text}</small></div></li>`;
  const halt=f.halt?`${h(HALT_TITEL[z.mission]||'Aufenthalt')} ${h(zeitText(f.halt))}${z.mission==='verstaerken'?' – länger, solange eine Belagerung läuft':''}`:'';
  const gesamt=(f.zurueck??f.ankunft)-f.abflug,teile=[{w:f.hin.dauer,cls:'hin'},{w:f.halt,cls:'halt'},{w:f.zurueck!=null?f.rueck.dauer:0,cls:'rueck'}].filter(t=>t.w>0);
  if(z.mission==='uebergeben')return `<div class="es-fahrplan"><ol class="es-fp">${punkt('start','Abflug bei Annahme','sobald der Empfänger annimmt (bis 1 Spieltag)',`ab ${h(startName())}`)}${strecke(`Flugzeit ${h(zeitText(f.hin.dauer))} · ${Math.round(z.tempo*100)} %`)}${punkt('ziel','Übergabe am Ziel',`${h(zeitText(f.hin.dauer))} nach der Annahme`,h(planetName(z.ziel)))}</ol>
   <div class="es-sprit${fehlt?' fehlt':''}"><b>Treibstoff ${zahl(f.sprit)} Deuterium</b>${f.reserve?` <span>(${zahl(f.hin.sprit)} Flug + ${zahl(f.reserve)} Reserve nach Transportregel)</span>`:''}<small>${z.startart==='flotte'?`Kommt bei der Annahme aus der Ladung der Flotte (einweg). An Bord: ${zahl(deut)} Deuterium`:`Wird bei der Annahme vom Startort abgebucht und belegt keinen Laderaum. Am Start: ${zahl(deut)} Deuterium`}${fehlt?' – reicht nicht':''}.</small></div></div>`;
  return `<div class="es-fahrplan"><div class="es-fp-balken" role="img" aria-label="Hinflug, Aufenthalt, Rückflug im Verhältnis">${teile.map(t=>`<span class="${t.cls}" data-anteil="${gesamt>0?t.w/gesamt:0}"></span>`).join('')}</div>
   <ol class="es-fp">${punkt('start',z.ankunftArt==='sofort'?'Abflug sofort':'Abflug automatisch',h(uhrzeit(f.abflug)),`ab ${h(startName())}`)}
    ${strecke(`Hinflug ${h(zeitText(f.hin.dauer))} · ${Math.round(z.tempo*100)} % · ${zahl(f.hin.sprit)} Deuterium`)}
    ${punkt('ziel','Ankunft am Ziel',(z.ankunftArt==='sofort'?'≈ ':'')+h(uhrzeit(f.ankunft)),h(zielTitel(z.ziel,z.zielart)))}
    ${halt?strecke(halt):''}
    ${halt&&f.rueck?punkt('weiter','Rückflug startet',h(uhrzeit(f.rueckStart)),''):''}
    ${f.offen?strecke(h(AM_ZIEL[z.mission]||'Bleibt bis zum Rückruf')):''}
    ${f.rueck&&f.zurueck!=null?`${strecke(`Rückflug ${h(zeitText(f.rueck.dauer))} · ${Math.round(z.ruecktempo*100)} % · ${zahl(f.rueck.sprit)} Deuterium`)}${punkt('heim','Zurück am Start',h(uhrzeit(f.zurueck)),`unterwegs insgesamt ${h(zeitText(f.zurueck-f.abflug))}`)}`:''}
    ${f.ohne?punkt('bleibt','Kein Rückflug','alle Terraformer werden verbraucht',`${zahl(f.erstattet)} Deuterium Rückflug-Treibstoff werden bei Ankunft erstattet`):''}
    ${f.rueck&&f.offen?punkt('heim','Heimflug nach dem Rückruf',`${h(zeitText(f.rueck.dauer))} ab Rückruf`,`${Math.round(z.ruecktempo*100)} % · ${zahl(f.rueck.sprit)} Deuterium schon beim Start bezahlt`):''}
    ${!f.rueck?punkt('bleibt','Kein Rückflug',h(z.ruecktempo===0&&P[z.mission]?.rueck?'Die Flotte bleibt im Orbit am Ziel – gestrandet, für alle Kenner des Systems sichtbar und angreifbar':AM_ZIEL[z.mission]||'Die Schiffe bleiben am Ziel'),'nur der Hinflug kostet Treibstoff'):''}
   </ol>
   <div class="es-sprit${fehlt?' fehlt':''}"><b>Treibstoff ${zahl(f.sprit)} Deuterium</b>${f.ohne?` <span>(${zahl(f.hin.sprit)} hin + ${zahl(f.rueck.sprit)} Rückflug, bei Ankunft erstattet – es bleiben ${zahl(f.sprit-f.erstattet)})</span>`:f.rueck?` <span>(${zahl(f.hin.sprit)} hin + ${zahl(f.rueck.sprit)} zurück)</span>`:''}<small>${z.startart==='flotte'?`Kommt aus der Ladung der gestrandeten Flotte. An Bord: ${zahl(deut)} Deuterium${fehlt?' – reicht nicht, zuerst versorgen':''}.`:`Wird beim Start vom Startort abgebucht und belegt keinen Laderaum. Am Start: ${zahl(deut)} Deuterium${fehlt?' – reicht nicht':''}.`}</small></div></div>`;
 }
 function uebergabeHtml(){
  if(z.mission!=='uebergeben')return '';
  return `<div class="es-teil"><h3 class="es-teil-titel">Übergabe ${infoKnopf('uebergabe')}</h3><div class="es-vorlagen">${chip('data-es-uebergabe','geschenk','Geschenk',!z.verkauf,'kostenlos, der Empfänger nimmt an')}${chip('data-es-uebergabe','verkauf','Verkauf',!!z.verkauf,'gegen Credits bei Zustellung')}</div>
   ${z.verkauf?`<label class="es-preis">Kaufpreis in Credits<input type="number" inputmode="decimal" min="0" max="1000000000" step="1" value="${Number(z.preis)||0}" data-es-preis></label>`:''}
   <p class="es-leise">Bis zur Annahme (höchstens 1 Spieltag) bleiben Schiffe und Ladung bei dir. Danach fliegen sie los und gehören bei Ankunft samt Ladung und Personen dem Empfänger.</p></div>`;
 }
 // Die gewählte Flotte als Bildreihe: Motiv, Anzahl, Name – man sieht auf einen Blick, wer fliegt.
 function flottenReihe(){
  const liste=gewaehlt().sort((a,b)=>(Number(regel(b[0]).angriff)||0)-(Number(regel(a[0]).angriff)||0));if(!liste.length)return '';
  return `<div class="es-reihe">${liste.map(([k,n])=>`<div class="es-reihe-schiff"><img src="${h(typeof art==='function'?art('ships',k):'')}" alt="" loading="lazy"><b>${zahl(n)}</b><small>${h(name(k))}</small></div>`).join('')}</div>`;
 }
 function startHtml(){
  // Vorgewählte Kopplung (Recycler hinterher) mitführen: ist +5 s nicht mehr erreichbar (gleich lange Flüge im System, die Zeit
  // seit dem Angriffsstart), rückt der Abstand auf das frühestmögliche Vielfache von 5 s – bis der Spieler selbst wählt.
  if(z.ankunftArt==='kopplung'&&z.kopplungVorgewaehlt&&!z.kopplungManuell){const f=fahrplan();if(f&&f.abflug<f.jetzt+2)z.kopplung.versatz=Math.ceil(((z.kopplung.versatz||0)+f.jetzt+2-f.abflug)/5)*5;}
  const m=z.mission,p=P[m]||{},d=zielBeschreibung();
  const zeile=(t,v,cls='')=>`<div class="${cls}"><dt>${h(t)}</dt><dd>${v}</dd></div>`;
  const schiffe=gewaehlt().map(([k,n])=>`${zahl(n)} × ${h(name(k))}`).join(', ');
  const ladung=Object.entries(z.ladung).filter(([,n])=>n>0).map(([g,n])=>`${zahl(n)} ${h(name(g))}`).join(', ');
  const td=truppenDaten(),truppen=z.truppen.map((n,i)=>n>0?`${zahl(n)} × ${h(td?.namen?.[i]||'')}`:'').filter(Boolean).join(', ');
  const personen=personenGewaehlt();
  const status=!['kolonisieren','abbau','recyceln','transport','stationieren'].includes(m)||z.startart==='flotte'||z.zielflotte||z.versorgen||verwuestung(z.ziel).an?(A()?.feindlich(m)?'<p class="es-plan">Der Kern prüft Ziel und Flotte beim Start. Feindliche Ziele sehen deine Flotte etwa 30 Minuten vor Ankunft.</p>':'<p class="es-plan">Der Kern prüft Ziel und Flotte beim Start.</p>')
   :!z.plan?'<p class="es-plan wartet">Startprüfung läuft …</p>':z.plan.status==='ready'?`<p class="es-plan ok">✓ Start geprüft${z.plan.gruendung_unter_vorbehalt?' · Gründung nur, wenn der Platz bei Ankunft noch frei ist':''}</p>`:z.plan.status==='blocked'?`<p class="es-plan schlecht">✗ ${h(z.plan.grund||'Start nicht möglich')}</p>`:`<p class="es-plan">${h(z.plan.grund||'Der Kern prüft beim Start.')}</p>`;
  return `${zielKarte()}${m==='kolonisieren'?eignungHtml():''}${uebergabeHtml()}
   ${zeitplanHtml()}
   ${ankunftHtml()}${verbandHtml()}
   <section class="es-teil"><h3 class="es-teil-titel">Fahrplan ${infoKnopf('fahrplan')}</h3>${fahrplanHtml()}</section>
   <section class="es-teil es-zusammen"><h3 class="es-teil-titel">Das fliegt los</h3>${flottenReihe()}<dl class="es-fakten">
    ${zeile('Start',h(startName()))}${zeile('Ziel',`<b class="koord">${h(z.zielart==='mond'?zielTitel(z.ziel,'mond'):'['+z.ziel+']')}</b> · ${h(d.art)}${d.spieler?' · '+h(d.spieler):''}`)}
    ${m==='kolonisieren'?zeile('Siedler',ohneGruppen()?`${zahl(siedlerPlaetze())} ${h(name(z.volk))}`:`${zahl(z.mischung[0])} Bewohner · ${zahl(z.mischung[1])} Fachkräfte · ${zahl(z.mischung[2])} Forscher (${h(name(z.volk))})`):''}
    ${ladung?zeile('Fracht',ladung):''}${personen?zeile('Personen',zahl(personen)):''}${truppen?zeile('Bodentruppen',truppen):''}
    ${P[m]?.dauer?zeile(HALT_TITEL[m]||'Dauer',`${zahl(z.stunden??P[m].dauer.std)} h`):''}
    ${m==='uebergeben'?zeile('Übergabe',z.verkauf?`Verkauf für ${zahl(z.preis)} Credits`:'Geschenk'):''}
   </dl></section>
   ${status}
   ${['kolonisieren','kampfkolonisieren'].includes(m)&&kolonieplatzBedingung(m).warnung?`<p class="es-plan warn">${h(kolonieplatzBedingung(m).text)}</p>`:''}
   ${p.bestaetigen?`<label class="es-bestaetigen"><input type="checkbox" data-es-bestaetigt ${z.bestaetigt?'checked':''}><span>${m==='mond_zerstoeren'?`Ich will den Mond <b>${h(zielBeschreibung().mond?.name||'')} [${h(z.ziel)}]</b> zerstören – das ist endgültig, und alle Weltenkrümmer können verloren gehen.`:m==='terraformen'?`Die Terraformer werden auf <b>[${h(z.ziel)}]</b> verbraucht – das Ziel stimmt.`:`Ich habe die Zielkoordinaten <b>[${h(z.ziel)}]</b> geprüft – sie stimmen.`}</span></label>`:''}
   ${z.fehler?`<p class="es-fehler" role="alert">${h(z.fehler)}</p>`:''}`;
 }
 function eignungHtml(){
  const k=z.ziel,h0=z.plan?.habitabilitaet||(V()?.erkundet||[]).find(x=>x.koord===k)?.habitabilitaet,e=(h0?.herkunft||[]).find(x=>x.volk===z.volk);
  if(!e)return '';const opt=Number(h0.optimum_promille)||1000,klima=Math.round((Number(e.wachstum_promille)||0)/opt*100);
  return `<section class="es-lage"><h3 class="es-teil-titel">Klima für ${h(name(z.volk))}</h3><div class="es-lagezahlen"><div class="es-lagezahl ${e.bewohnbar?(klima>=80?'gut':'warn'):'schlecht'}"><b>${e.bewohnbar?zahl(klima)+' %':'unbewohnbar'}</b><span>${e.bewohnbar?(Number.isFinite(e.wachstum_je_tag_promille)?`Wachstum ${zahl(e.wachstum_je_tag_promille/10)} % je Tag`:'bewohnbar'):`Terraforming ${zahl(e.terraforming_noetig)} nötig – oder Terraformer schicken`}</span></div>${h0.zone?`<div class="es-lagezahl"><b>${h(name(h0.zone))}</b><span>Klimazone · Platz ${h(h0.platz??String(k).split(':')[2])}</span></div>`:''}</div></section>`;
 }

 // ---------------------------------------------------------------- Zeichnen
 function inhaltHtml(){
  if(z.schritt==='ziel')return zielHtml();
  if(z.schritt==='auftrag')return auftragHtml();
  if(z.schritt==='flotte')return flotteHtml();
  if(z.schritt==='details')return detailsHtml();
  return startHtml();
 }
 // Fußleiste (Karl 24.10.2026: „die Buttons unten sind so komisch“): links ein ruhiger Zurück-Knopf mit Zeichen und Wort,
 // rechts der Hauptknopf; im Start-Schritt darunter das Ergebnis (Ankunft, Treibstoff). Der Satz darüber nur, wenn etwas fehlt.
 function fussHtml(){
  const i=schrittIndex(),grund=sperrgrund(),letzter=i===schritte().length-1,fehler=!!z.fehler;
  const text=fehler?z.fehler:grund,f=letzter?fahrplan():null;
  const unter=z.senden||!letzter?'':z.mission==='uebergeben'?'fliegt los, sobald der Empfänger annimmt':f?`an ${uhrMin(f.ankunft)}${f.zurueck!=null&&!/Tag/.test(uhrMin(f.ankunft)+uhrMin(f.zurueck))?' · heim '+uhrMin(f.zurueck):''} · ${zahl(f.sprit-(f.erstattet||0))} Deuterium`:'';
  return `<p class="es-fuss-text${fehler?' es-fuss-fehler':''}" aria-live="polite">${h(text)}</p>
   <button type="button" class="es-zurueck" data-es-zurueck><span aria-hidden="true">${i>0?'‹':'✕'}</span><small>${i>0?'Zurück':'Abbrechen'}</small></button>
   <button type="button" class="es-weiter${letzter?' es-starten':''}" data-es-weiter ${grund||z.senden?'aria-disabled="true"':''}${z.senden?' aria-busy="true"':''}><b>${z.senden?'Startet …':h(weiterText())}</b>${unter?`<small>${h(unter)}</small>`:''}</button>`;
 }
 function blatt(){
  let b=el('einsatz-blatt');if(b)return b;
  b=document.createElement('section');b.id='einsatz-blatt';b.className='es-blatt';b.setAttribute('role','dialog');b.setAttribute('aria-modal','false');b.setAttribute('aria-labelledby','es-titel');b.hidden=true;
  b.innerHTML='<div class="es-flaeche"><div class="es-kopfbereich"></div><div class="es-innen"></div><div class="es-fuss"></div><div class="es-infobereich"></div></div>';
  document.body.append(b);b.addEventListener('click',klick);b.addEventListener('input',eingabe);b.addEventListener('change',eingabe);b.addEventListener('toggle',klappe,true);
  b.addEventListener('keydown',e=>{if(e.key==='Escape'){e.preventDefault();zurueckSchritt();}if(e.key==='Enter'&&e.target.matches('[data-es-suche]')){e.preventDefault();const t=e.target.value.trim();if(KOORD.test(t))zielSetzen(t,'planet');}});
  return b;
 }
 function zeichnen(){
  const b=el('einsatz-blatt');if(!b||!z||b.hidden)return;
  const set=(sel,html)=>{const n=b.querySelector(sel);if(typeof setStableHTML==='function')setStableHTML(n,html);else n.innerHTML=html;};
  set('.es-kopfbereich',kopf());set('.es-innen',inhaltHtml());set('.es-fuss',fussHtml());set('.es-infobereich',infoHtml());b.classList.toggle('es-info-offen',!!z.info);
  // Eigene Namen: app.js reagiert dokumentweit auf [data-mission] (springt zur Flottenseite) – nie so am Blatt setzen.
  b.dataset.esStufe=z.schritt;b.dataset.esAuftrag=z.mission||'';
  const bild=gruppenBild(z.mission),wert=bild?`url("${bild}")`:'none';if(b.style.getPropertyValue('--es-bild')!==wert)b.style.setProperty('--es-bild',wert);
  b.querySelector('.es-flaeche').className='es-flaeche'+(z.mission?' auftrag-'+(A()?.info(z.mission)?.gruppe||''):'');
  // Breiten nach dem Zeichnen setzen (CSP: style-src 'self', keine style-Attribute im HTML; setStableHTML entfernt sie sonst).
  b.querySelectorAll('[data-anteil]').forEach(s=>{const w=(Number(s.dataset.anteil)*100).toFixed(2)+'%';if(s.style.width!==w)s.style.width=w;});
 }
 function spaeter(){clearTimeout(zeichenUhr);zeichenUhr=setTimeout(zeichnen,40);}

 // ---------------------------------------------------------------- Kernabfragen (sparsam: Ratenlimit)
 function aktion(){
  const schiffe=Object.fromEntries(gewaehlt()),ladung=Object.fromEntries(Object.entries(z.ladung).filter(([,n])=>n>0));
  const a={typ:'flotte_senden',start:z.start,ziel:z.ziel,mission:z.mission==='uebergeben'?'transport':z.mission,geschwindigkeit:z.tempo,haltedauer_stunden:P[z.mission]?.dauer?(z.stunden??P[z.mission].dauer.std):0,schiffe,ladung};
  if(z.zielart==='mond')a.zielart='mond';if(z.startart==='mond')a.startart='mond';
  if(z.mission==='kolonisieren'){a.siedler_volk=z.volk;if(!ohneGruppen())a.siedler_mischung=z.mischung.slice();}
  if(z.mission==='terraformen'&&z.tfVolk&&z.tfVolk!==V()?.volk)a.siedler_volk=z.tfVolk;
  return a;
 }
 function befehl(){
  const a=aktion(),truppen=z.truppen.map(n=>Number(n)||0),mitTruppen=truppen.some(n=>n>0);
  // 0.4.8: Start von einer gestrandeten Flotte, Angriff auf eine gestrandete Flotte, Versorgen einer gestrandeten Flotte.
  if(z.startart==='flotte'){const fid=Number(z.start);
   if(z.mission==='uebergeben')return {typ:'orbitflotte_anbieten',flotte:fid,ziel:z.ziel,preis:z.verkauf?Math.max(0,Number(z.preis)||0):0};
   return {typ:'flotte_starten',flotte:fid,ziel:z.ziel,mission:z.mission,geschwindigkeit:z.tempo,rueckgeschwindigkeit:P[z.mission]?.rueck?z.ruecktempo:1,haltedauer_stunden:a.haltedauer_stunden||0,...(z.zielflotte?{zielflotte:z.zielflotte}:{})};}
  if(z.zielflotte)return {typ:'orbitflotte_angreifen',start:a.start,zielflotte:z.zielflotte,schiffe:a.schiffe,geschwindigkeit:z.tempo,rueckgeschwindigkeit:z.ruecktempo};
  if(z.versorgen)return {typ:'flotte_versorgen',start:a.start,ziel:a.ziel,versorgungsflotte:z.versorgen,schiffe:a.schiffe,geschwindigkeit:a.geschwindigkeit,ladung:a.ladung};
  const voll=Array.from({length:Math.max(12,truppen.length)},(_,i)=>truppen[i]||0);
  // Kolonieflug: geplante/gekoppelte Ankunft über flotte_planen (Funktionsprüfung 25.10.2026: Ankunft ging verloren); Einweg.
  if(z.mission==='kolonisieren'){const k=mitTruppen?{typ:'boden_senden',flug:a,truppen:voll}:a,fk=z.ankunftArt!=='sofort'?fahrplan():null;return fk?{typ:'flotte_planen',flug:k,ankunft:Math.round(fk.ankunft),rueckgeschwindigkeit:1}:k;}
  const b=!mondFlug()&&(personenGewaehlt()>0||mitTruppen)?{typ:'flotte_beladen',flug:a,personen:{gruppen:z.personen.map(g=>g.map(n=>(Number(n)||0)*M))},truppen:voll}:a;
  const personen={gruppen:z.personen.map(g=>g.map(n=>(Number(n)||0)*M))};
  // Flottenübergabe: Angebot an den Empfänger (Kern schiffhandel.rs) – Preis 0 ist ein Geschenk.
  if(z.mission==='uebergeben')return {typ:'schiffangebot',flug:a,preis:z.verkauf?Math.max(0,Number(z.preis)||0):0,personen,truppen:voll};
  // Eigenes Rückflugtempo und geplante Ankunft (Codex 24.10.: getrennte Flugabschnitte) laufen über flotte_planen.
  const f=z.ankunftArt!=='sofort'?fahrplan():null;
  if(f)return {typ:'flotte_planen',flug:b,ankunft:Math.round(f.ankunft),rueckgeschwindigkeit:P[z.mission]?.rueck?z.ruecktempo:1};
  return P[z.mission]?.rueck&&z.ruecktempo<1?{typ:'flotte_planen',flug:b,rueckgeschwindigkeit:z.ruecktempo}:b;
 }
 // Transport: Laderaum mit Deuterium, Kristall, Erz vorbelegen; kommt der echte Laderaum (Volk, Logistik) später, wird neu
 // verteilt, solange der Spieler diese Güter nicht selbst geändert hat (Funktionsprüfung 25.10.2026: 250.000 statt 450.000).
 function vorbelegen(){for(const g of z.vorbelegt||[])delete z.ladung[g];let rest=Math.max(0,laderaum()-fracht());const neu=[],gueter=['deuterium','kristall','erz'];for(let i=0;i<gueter.length;i++){const g=gueter[i],anteil=Math.floor(rest/(gueter.length-i)),n=Math.min(anteil,Math.floor(frachtMax(g)));if(n>0){z.ladung[g]=n;rest-=n;neu.push(g);}}z.vorbelegt=neu;}
 // Abbauertrag wie Kern flotte.rs abbau_ertrag: je Gut Rate × Reichtum des Systems (‰).
 function abbauRoh(n,stunden,fl){const g=zielBeschreibung().guertel||{},re=Number(g.reich_erz_promille??1000)/1000,rk=Number(g.reich_kristall_promille??1000)/1000;return n*stunden*((fl.abbau_erz_je_stunde||150)*re+(fl.abbau_kristall_je_stunde||100)*rk);}
 function flugNeu(){z.flug=null;z.plan=null;z.sim=null;bald();}
 function bald(){clearTimeout(uhr);uhr=setTimeout(nachladen,380);}
 async function nachladen(){
  if(!z)return;const nr=++lauf,a=aktion();
  try{
   if(Object.keys(a.schiffe).length&&KOORD.test(a.ziel)&&z.start){const q=z.startart==='flotte'?{typ:'flugzeit',flotte:Number(z.start),ziel:a.ziel,geschwindigkeit:a.geschwindigkeit}:{typ:'flugzeit',start:a.start,ziel:a.ziel,schiffe:a.schiffe,geschwindigkeit:a.geschwindigkeit};if(z.startart==='mond')q.startart='mond';const f=await api('/api/tool',q);if(nr!==lauf||!z)return;const vorher=laderaum();z.flug=f;z.flugTempo=a.geschwindigkeit;
    // „Laderaum füllen“ mit dem echten Laderaum (Volk, Logistik) nachziehen, solange der Spieler die Fracht nicht selbst geändert hat.
    if(z.frachtArt==='voll'&&laderaum()!==vorher)frachtSetzen('voll');else if(z.vorbelegt?.length&&laderaum()!==vorher)vorbelegen();}
   if(z.schritt==='start'&&z.flug&&!z.flug.tempi&&Object.keys(a.schiffe).length){const s=JSON.stringify([a.start,a.ziel,a.schiffe]);if(z.tempiSchluessel!==s){z.tempiSchluessel=s;
     const reihe=await Promise.all(TEMPI.map(g=>api('/api/tool',z.startart==='flotte'?{typ:'flugzeit',flotte:Number(z.start),ziel:a.ziel,geschwindigkeit:g}:{typ:'flugzeit',start:a.start,ziel:a.ziel,schiffe:a.schiffe,geschwindigkeit:g}).then(x=>({geschwindigkeit:g,dauer_sekunden:x.dauer_sekunden,treibstoff_je_strecke:x.treibstoff_je_strecke,treibstoff_je_strecke_milli:x.treibstoff_je_strecke_milli})).catch(()=>null)));
     if(nr!==lauf||!z)return;z.tempiErsatz=reihe.filter(Boolean);}}
   if(z.schritt==='start'&&z.startart!=='flotte'&&!z.zielflotte&&!z.versorgen&&!verwuestung(z.ziel).an&&['kolonisieren','abbau','recyceln','transport','stationieren'].includes(z.mission)&&!flottenFehler()&&!detailFehler()){const q=await api('/api/tool',{typ:'missionsplan',aktion:a});if(nr!==lauf||!z)return;z.plan=q;}
  }catch(err){if(nr!==lauf||!z)return;if(z.schritt==='start')z.plan={status:'blocked',grund:err.message};}
  zeichnen();
 }
 async function bedarfLaden(){
  if(z.mission!=='kolonisieren')return;const typ=kolonieTyp()||'kolonieschiff',s=z.volk+':'+typ;if(z.bedarfSchluessel===s)return;
  try{const q=await api('/api/tool',{typ:'koloniebedarf',volk:z.volk,einheit:typ});if(!z)return;z.bedarf=q;z.bedarfSchluessel=s;}catch(_){if(z)z.bedarf=null;}
 }
 async function terraformLaden(){
  if(z.mission!=='terraformen'||!KOORD.test(z.ziel))return;const s=z.ziel;
  const volk=z.tfVolk||null,schl=s+':'+volk;if(z.tfSchluessel===schl&&z.tf)return;
  try{const q=await api('/api/tool',{typ:'terraformbedarf',koord:s,volk});if(!z||z.ziel!==s)return;z.tf=q;z.tfSchluessel=schl;z.tfFehler='';}catch(err){if(z){z.tf=null;z.tfFehler=err.message;}}
 }
 async function simulieren(){
  const s=await api('/api/tool',{typ:'kampfsimulator',ziel:z.ziel,...(z.zielart==='mond'?{zielart:'mond'}:{}),schiffe:Object.fromEntries(gewaehlt()),bergung_promille:1000});
  // Mittleres Trümmerfeld aus den Einzelläufen (wie flottenfuehrung.js simulieren): Bergungsrohstoff je Lauf in Tausendsteln.
  const l=s.einzellaeufe||[];s.truemmer_mittel=l.length?l.reduce((n,x)=>n+(x.bergung_rohmaterial_milli||[0,0]).reduce((a,b)=>a+b,0),0)/l.length/1000:0;
  if(!z)return;z.sim=s;z.simSchluessel=JSON.stringify([z.ziel,z.schiffe]);zeichnen();
 }

 // Ein-Tipp-Aufklärung (Befund H4): eine Sonde vom nächsten Planeten mit Sonden, ohne das Blatt zu verlassen.
 async function sondeSchicken(mission,knopf){
  const ziel=z.ziel,orte=planeten().filter(p=>(p.schiffe?.spionagesonde||0)>0);if(!orte.length){z.fehler='Keine Spionagesonde vorhanden – in der Werft bauen.';zeichnen();return;}
  const start=(orte.find(p=>p.koord===z.start)||orte.find(p=>systemVon(p.koord)===systemVon(ziel))||orte[0]).koord;
  // Für Angriffsziele gleich so viele Sonden, dass Flotte, Abwehr und Gebäude im Bericht stehen (Scanstärke 5).
  const n=mission==='spionage'&&A()?.feindlich(z.mission)?Math.min(5,(planeten().find(p=>p.koord===start)?.schiffe?.spionagesonde)||1):1;
  const a={typ:'flotte_senden',start,ziel,mission,geschwindigkeit:1,haltedauer_stunden:0,schiffe:{spionagesonde:n},ladung:{}};if(z.zielart==='mond'&&mission==='spionage')a.zielart='mond';
  knopf.disabled=true;knopf.setAttribute('aria-busy','true');
  try{const r=await postCommand({world_id:W().world_id,request_id:requestId(),aktionen:[a]});const e=r?.ergebnisse||[];if(!e.every(x=>x.ok))throw Error(e.map(x=>x.text).join(' · ')||'Sonde abgelehnt');if(typeof refresh==='function')await refresh();}
  catch(err){if(z){z.fehler=err.message;}}
  if(z)zeichnen();
 }

 // ---------------------------------------------------------------- Ablauf
 function startWaehlen(koord,art='planet'){
  z.start=koord;z.startart=art;z.schiffe={};z.truppen=Array(12).fill(0);z.personen=VOELKER.map(()=>[0,0,0]);z.flug=null;z.plan=null;z.vorlage='';
  if(art==='flotte'){const f=gestrandeteFlotte(koord);z.schiffe={...(f?.schiffe||{})};z.ladung={};return;}
  if(z.mission==='kolonisieren'){const vs=voelkerAm(startOrt());if(!vs.includes(z.volk))z.volk=vs.includes(V()?.volk)?V().volk:(vs[0]||V()?.volk);}
  standardVorlage();
 }
 // Startort: ausdrücklich gewählt › aktiver Planet mit passenden Schiffen › meiste passende Schiffe (gleiches System zuerst).
 function besterStart(){
  const p=P[z.mission]||{},rel=k=>erlaubt(k)&&((p.nur||[]).includes(k)||sektion(k)===p.sektionen?.[0]);
  const orte=startOrte().map(x=>({x,art:x.art,n:Object.entries(x.schiffe||{}).filter(([k,n])=>n>0&&rel(k)).reduce((s,[,n])=>s+n,0)}));
  const aktiv=typeof selectedPlanet!=='undefined'?selectedPlanet:'';
  const deut=o=>o.art==='flotte'?Number(o.x.flotte?.gestrandet?.deuterium)||0:Number(o.x.bestand?.deuterium)||0;
  const gut=orte.filter(o=>o.n>0&&deut(o)>0),liste=gut.length?gut:orte.filter(o=>o.n>0).length?orte.filter(o=>o.n>0):orte;
  const bester=liste.slice().sort((a,b)=>(z.versorgen?deut(b)-deut(a):0)||(systemVon(b.x.koord)===systemVon(z.ziel))-(systemVon(a.x.koord)===systemVon(z.ziel))||(a.art==='planet'?0:1)-(b.art==='planet'?0:1)||b.n-a.n)[0];
  // Einsatzprobe 25.10.2026: eine frische Kolonie (1 Kreuzer, 0 Deuterium) wurde als aktiver Planet vorgewählt – Angriff und
  // Versorgen scheiterten erst beim Start. Aktiv zählt nur, wenn er mindestens die Hälfte der Schiffe des besten Orts hat.
  const aktivOrt=liste.find(o=>o.x.koord===aktiv&&o.art==='planet'&&o.n>0&&deut(o)>0&&o.n*2>=(bester?.n||0)&&!(z.versorgen&&deut(o)*2<deut(bester)));
  const wahl=orte.find(o=>o.x.koord===z.start&&o.art===z.startart)||aktivOrt||bester;
  return wahl?[wahl.x.koord,wahl.art]:[aktiv||planeten()[0]?.koord||'','planet'];
 }
 async function missionSetzen(m){
  z.mission=m;z.missionInit=m;z.plan=null;z.sim=null;z.bestaetigt=false;z.stunden=P[m]?.dauer?P[m].dauer.std:null;z.fehler='';
  // Vom Orbit gestartet (Einsatz.oeffnen({gestrandet})): Startort bleibt die Flotte, solange der Auftrag dort erlaubt ist.
  if(z.startart==='flotte'&&orbitAuftragErlaubt(gestrandeteFlotte(z.start),m)){startWaehlen(z.start,'flotte');return;}
  const [k,art]=besterStart();z.start=k;z.startart=art;
  if(m==='terraformen'){await terraformLaden();if(!z)return;}
  startWaehlen(z.start,z.startart);
  // Aus dem Hangar mitgebrachte Schiffe (Flotte gesamt › Standort) behalten, soweit der Auftrag sie erlaubt.
  if(z.vorwahl){const s={};for(const [k,n] of Object.entries(z.vorwahl))if(n>0&&erlaubt(k))s[k]=Math.min(n,da(k));if(Object.values(s).some(n=>n>0)){z.schiffe=s;z.vorlage='';}}
  if(m==='kolonisieren'){await bedarfLaden();if(!z)return;mischungSetzen('bewohner');frachtSetzen('mindest');}
  if(m==='saven')frachtSetzen('voll');
 }
 function zielSetzen(k,art='planet'){
  z.ziel=k;z.zielart=art;z.suche='';z.tf=null;z.plan=null;z.sim=null;z.fehler='';
  if(z.mission&&!bedingungen(z.mission)[0].ok){z.mission='';}
  geheZu(z.mission?'flotte':'auftrag');
 }
 async function geheZu(s){
  const vorher=z.schritt;z.schritt=s;z.fehler='';if(vorher!==s)z.gutReihe=null;
  if(s==='flotte'&&z.mission&&z.missionInit!==z.mission){await missionSetzen(z.mission);if(!z)return;}
  // Neues Ziel bei gewähltem Terraformen: Plan des neuen Platzes laden (Funktionsprüfung 25.10.2026: „Plan wird berechnet …“ blieb stehen).
  else if(s==='flotte'&&z.mission==='terraformen'&&!z.tf){await missionSetzen('terraformen');if(!z)return;}
  if(s==='flotte'&&vorher!=='flotte'&&z.mission)setTimeout(()=>alsGesehen(z?.mission),1500);
  if(s==='start'){z.plan=null;
   // Recycler hinterher: ohne Trümmerfeld am Ziel die Ankunft einmal an den eigenen Angriff dorthin koppeln (+5 s) –
   // in der Ankerliste stehen sonst auch Sonden und andere Flüge ins System.
   if(z.mission==='recyceln'&&!z.kopplungVorgewaehlt&&z.ankunftArt==='sofort'&&!truemmerAn(z.ziel)){
    const f=(V()?.flotten||[]).filter(x=>x.ziel===z.ziel&&x.zustand==='hinflug'&&A()?.feindlich?.(x.mission)).sort((a,b)=>(b.ankunft_sekunden||0)-(a.ankunft_sekunden||0))[0];// letzter Einschlag: erst dann ist das Trümmerfeld vollständig
    if(f){z.ankunftArt='kopplung';z.kopplung={anker:'f'+f.flotte,versatz:5};z.kopplungVorgewaehlt=true;}}}
  zeichnen();const innen=el('einsatz-blatt')?.querySelector('.es-innen');if(innen&&vorher!==s)innen.scrollTop=0;
  if(s==='flotte'||s==='details'||s==='start')bald();
 }
 function zurueckSchritt(){
  if(z.info){z.info='';zeichnen();return true;}
  if(z.offen.tempowahl){z.offen.tempowahl='';zeichnen();return true;}
  const i=schrittIndex();if(i>0){const ziel=schritte()[i-1][0];geheZu(ziel);return true;}
  schliessen();return true;
 }
 async function weiter(){
  const grund=sperrgrund();if(grund){z.fehler=grund;zeichnen();return;}
  const i=schrittIndex(),s=schritte();
  // Leerer Transport: Laderaum mit Grundrohstoffen vorbelegen – nie ungefragt Xenokristall, Antriebskerne oder Habitatmodule
  // (Einsatzprobe 25.10.2026: „Laderaum füllen“ lud wertvollstes zuerst, eingetippte Mengen überluden dann die Flotte).
  if(i<s.length-1){if(s[i][0]==='flotte'&&z.mission==='transport'&&!z.versorgen&&!fracht()){z.vorbelegt=[];vorbelegen();z.frachtArt='';}geheZu(s[i+1][0]);return;}
  await starten();
 }
 async function starten(){
  if(z.senden)return;const ro=typeof gameReadOnly==='function'?gameReadOnly():'';if(ro){z.fehler=ro;zeichnen();return;}
  z.senden=true;z.fehler='';zeichnen();
  const m=z.mission,ziel=z.ziel,hs=hinflugSekunden();
  try{
   const r=await postCommand({world_id:W().world_id,request_id:requestId(),aktionen:[befehl()]});
   const e=r?.ergebnisse||[];
   if(e.length&&e.every(x=>x.ok)){
    merken();const a=A()?.info(m),fp=fahrplan(),verband=m==='angriff'?z.verband:'';
    // Verband: nach dem Start beitreten bzw. eröffnen (wie flottenfuehrung.js verbandSenden).
    if(verband){const id=Number((e.map(x=>x.text).join(' ').match(/Flotte (\d+)/)||[])[1]);
     if(id){try{await postCommand({world_id:W().world_id,request_id:requestId(),aktionen:[verband==='neu'?{typ:'verband_oeffnen',flotte:id}:{typ:'verband_beitreten',flotte:id,fuehrung:Number(verband)}]});}catch(err){if(typeof show==='function')show('Flotte gestartet, Verband nicht möglich: '+err.message,true);}}}
    // Eigene Rückmeldung statt Kerntext (Karl 24.10.: „Bodentruppen mit Flotte 2 gestartet“ beim Kolonieflug war irreführend).
    // Ankunft aus dem Kern (Flotte nach dem Neuladen), nicht aus der Vorplanung – sie lag einmal 1 s daneben (Funktionsprüfung 25.10.2026).
    const fid=Number((e.map(x=>x.text).join(' ').match(/Flotte (\d+)/)||[])[1]),sofort=z.ankunftArt==='sofort'||!fp;
    const plan=m==='uebergeben'?`Flottenübergabe an [${ziel}] angeboten – Schiffe bleiben bis zur Annahme bei dir`
     :!sofort?`${a?.name||'Flotte'} → [${ziel}] geplant · Abflug ${uhrzeit(fp.abflug)} · Ankunft ${uhrzeit(fp.ankunft)}${fp.zurueck?` · zurück ${uhrzeit(fp.zurueck)}`:''}`:null;
    // Was mitfliegt, steht in der Bestätigung (Funktionsprüfung 25.10.2026: nur Ziel und Zeiten) – vor schliessen() erfassen.
    const tr=z.truppen.reduce((s,n)=>s+(Number(n)||0),0),mit=[fracht()>0?`${zahl(fracht())} Fracht`:'',personenGewaehlt()>0?`${zahl(personenGewaehlt())} Personen`:'',tr>0?`${zahl(tr)} Bodentruppen`:''].filter(Boolean).join(', ');
    schliessen();if(typeof refresh==='function')await refresh().catch(()=>{});
    const kf=fid?(V()?.flotten||[]).find(f=>Number(f.flotte)===fid):null,an=kf?.ankunft_sekunden??fp?.ankunft,zur=Number(kf?.rueckkehr_sekunden)>0?kf.rueckkehr_sekunden:fp?.zurueck;
    const text=(plan||`${a?.name||'Flotte'} → [${ziel}] gestartet${an!=null?' · Ankunft '+uhrzeit(an)+(zur?' · zurück '+uhrzeit(zur):''):hs!=null?' · Ankunft in '+zeitText(hs):''}`)+(mit?` · an Bord: ${mit}`:'');
    if(typeof show==='function')show(text,false);
   }else{z.senden=false;z.fehler=e.map(x=>x.text).filter(Boolean).join(' · ')||'Start abgelehnt';zeichnen();}
  }catch(err){if(!z)return;z.senden=false;z.fehler=err.message||String(err);zeichnen();}
 }
 function schliessen(){
  const b=el('einsatz-blatt');if(b){b.hidden=true;document.body.classList.remove('einsatz-offen');}
  z=null;clearTimeout(uhr);lauf++;
  if(rueckFokus?.isConnected)try{rueckFokus.focus({preventScroll:true});}catch(_){}
 }
 let rueckFokus=null,geoeffnetUm=0;
 // Seitenwechsel über die Leiste schließt das Blatt (Handy-Abnahme 24.10.2026: sonst deckte es die neue Seite ab). Kurz nach
 // dem Öffnen nicht – manche Aufrufer wechseln erst die Seite und öffnen dann.
 document.addEventListener('game-navigation',()=>{const b=el('einsatz-blatt');if(z&&b&&!b.hidden&&Date.now()-geoeffnetUm>600)schliessen();});
 async function oeffnen(o={}){
  geoeffnetUm=Date.now();
  const v=V();if(!v||!planeten().length)return false;
  const ziel=String(o.ziel||'').trim();
  rueckFokus=document.activeElement;
  z=neu({verkauf:!!o.verkauf,verband:o.verband?String(o.verband):'',...(o.kopplung?.anker?{ankunftArt:'kopplung',kopplung:{anker:String(o.kopplung.anker),versatz:Number(o.kopplung.versatz??5)||0}}:{}),ziel:KOORD.test(ziel)?ziel:'',zielart:o.zielart==='mond'?'mond':'planet',start:o.start||'',startart:o.startart==='mond'?'mond':'planet',suche:KOORD.test(ziel)?'':ziel,vorwahl:o.schiffe&&Object.keys(o.schiffe).length?{...o.schiffe}:null,
   zielflotte:o.zielflotte?Number(o.zielflotte):0,versorgen:o.versorgen?Number(o.versorgen):0});
  if(o.gestrandet){const f=gestrandeteFlotte(o.gestrandet);if(f){z.start=String(f.flotte);z.startart='flotte';z.schiffe={...(f.schiffe||{})};
   // „Übergeben“ ohne Ziel: steht die Flotte über einem fremden Planeten, ist dessen Besitzer der naheliegende Empfänger.
   if(o.mission==='uebergeben'&&!z.ziel){const m=zielBeschreibung(f.ziel,'planet').merkmale;if(m.has('fremd')||m.has('verbuendet')){z.ziel=f.ziel;z.zielart='planet';}}}}
  if(z.versorgen){const f=gestrandeteFlotte(z.versorgen);if(f){z.ziel=f.ziel;z.zielart='planet';}}
  const b=blatt();b.hidden=false;document.body.classList.add('einsatz-offen');
  invasionTexte();
  const m=o.mission&&UNTERSTUETZT.has(o.mission)?o.mission:'';
  if(z.ziel&&m&&bedingungen(m)[0].ok){await missionSetzen(m);if(!z)return true;z.schritt='flotte';}
  else if(z.ziel){z.mission=m;z.schritt='auftrag';}
  else{z.mission=m;z.schritt='ziel';}
  zeichnen();bald();
  setTimeout(()=>{const f=b.querySelector(z?.schritt==='ziel'?'[data-es-suche]':'.es-x');try{f?.focus({preventScroll:true});}catch(_){}},60);
  return true;
 }

 // ---------------------------------------------------------------- Bedienung
 function setzen(art,k,n){
  n=Math.max(0,Math.round(Number(n)||0));
  if(art==='schiff'){n=Math.min(n,da(k));if(['kolonieschiff','grosses_kolonieschiff'].includes(k)){n=Math.min(n,1);if(n>0)for(const x of ['kolonieschiff','grosses_kolonieschiff'])if(x!==k)delete z.schiffe[x];}
   if(n>0)z.schiffe[k]=n;else delete z.schiffe[k];z.vorlage='';flugNeu();
   if(z.mission==='kolonisieren'&&['kolonieschiff','grosses_kolonieschiff'].includes(k)){z.bedarfSchluessel='';bedarfLaden().then(()=>{if(!z)return;mischungSetzen(z.mischungArt==='eigen'?'bewohner':z.mischungArt);frachtSetzen(z.frachtArt||'mindest');zeichnen();});}}
  else if(art==='gut'){n=Math.min(n,frachtMax(k));if(n>0)z.ladung[k]=n;else delete z.ladung[k];z.frachtArt='eigen';z.plan=null;
   if(z.vorbelegt?.length){z.vorbelegt=z.vorbelegt.filter(g=>g!==k);let zuviel=fracht()-laderaum();
    for(const g of [...z.vorbelegt].reverse()){if(zuviel<=0)break;const weg=Math.min(zuviel,Number(z.ladung[g])||0);z.ladung[g]-=weg;zuviel-=weg;if(!(z.ladung[g]>0)){delete z.ladung[g];z.vorbelegt=z.vorbelegt.filter(x=>x!==g);}}}}
  else if(art==='trupp'){const i=Number(k);n=Math.min(n,truppenDaten()?.bestand?.[i]||0);z.truppen[i]=n;z.plan=null;}
  else if(art==='person'){const [i,j]=k.split(':').map(Number);n=Math.min(n,personenAm(startOrt(),VOELKER[i])[j]);z.personen[i][j]=n;z.plan=null;}
 }
 function wertVon(art,k){if(art==='schiff')return z.schiffe[k]||0;if(art==='gut')return Number(z.ladung[k])||0;if(art==='trupp')return z.truppen[Number(k)]||0;const [i,j]=k.split(':').map(Number);return z.personen[i][j]||0;}
 function maxVon(art,k){if(art==='schiff')return ['kolonieschiff','grosses_kolonieschiff'].includes(k)?Math.min(1,da(k)):da(k);if(art==='gut')return frachtMax(k);if(art==='trupp')return truppenDaten()?.bestand?.[Number(k)]||0;const [i,j]=k.split(':').map(Number);return personenAm(startOrt(),VOELKER[i])[j];}
 const schrittweite=(art,k)=>art==='gut'||art==='person'?Math.max(1,Math.pow(10,Math.max(0,String(Math.round(maxVon(art,k))).length-2))):1;
 function truppenVorlage(art){
  const d=truppenDaten(),b=d?.bestand||[];
  if(art==='keine')z.truppen=Array(12).fill(0);
  else if(art==='alle')z.truppen=b.map(n=>Number(n)||0);
  else{let frei=truppenKapazitaet()||Math.max(0,da('truppentransporter'))*PLAETZE_JE_TRUPPENTRANSPORTER;z.truppen=Array(12).fill(0);
   // Erst die Bodenkern-Typen (Infanterie, Panzer, Drohnen), dann der Rest – so lange Plätze frei sind.
   for(const i of b.map((_,i)=>i)){const je=Number(d?.truppenplaetze?.[i])||1;const n=Math.min(b[i]||0,Math.floor(frei/je));z.truppen[i]=n;frei-=n*je;}}
  if(['kampfkolonisieren','verstaerken'].includes(z.mission)||truppenPlaetze()>truppenKapazitaet())truppentransporterErgaenzen();
  z.plan=null;
 }
 async function klick(e){
  const b=e.target.closest('button,summary');if(!b||!z)return;const ds=b.dataset;
  if(b.getAttribute('aria-disabled')==='true'&&!('esWeiter' in ds))return;
  if('esSchliessen' in ds){schliessen();return;}
  if('esZurueck' in ds){zurueckSchritt();return;}
  if('esWeiter' in ds){if(b.getAttribute('aria-disabled')==='true'){z.fehler=sperrgrund();zeichnen();const t=el('einsatz-blatt')?.querySelector('.es-fuss-text');t?.classList.add('es-ruck');setTimeout(()=>t?.classList.remove('es-ruck'),400);return;}await weiter();return;}
  if(ds.esSchritt){const s=ds.esSchritt,i=schritte().findIndex(([x])=>x===s);if(i<=schrittIndex()||schritte().slice(0,i).every(([x])=>schrittOk(x)))geheZu(s);return;}
  if(ds.esZiel){
   // Gestrandete Flotte als Ziel: Angriff auf genau diese Flotte bzw. Versorgen; jede andere Zielzeile hebt das auf.
   z.zielflotte=ds.esZfArt==='angriff'?Number(ds.esZf):0;z.versorgen=ds.esZfArt==='versorgen'?Number(ds.esZf):0;
   if(ds.esZfArt){z.mission='';z.missionInit='';z.ziel=ds.esZiel;z.zielart='planet';z.suche='';await missionSetzen(ds.esZfArt==='angriff'?'angriff':'transport');if(!z)return;
    if(bedingungen(z.mission)[0].ok){geheZu('flotte');return;}zeichnen();return;}
   zielSetzen(ds.esZiel,ds.esZielart||'planet');if(ds.esRecycler&&z&&z.mission!=='recyceln'){await missionSetzen('recyceln');if(!z)return;zeichnen();}return;}
  if(ds.esMission){z.info='';const m=ds.esMission;await missionSetzen(m);if(!z)return;zeichnen();if(bedingungen(m)[0].ok)geheZu('flotte');return;}
  if(ds.esAussen){const k=ds.esAussen,ziel=z.ziel;schliessen();if(k==='expedition'&&typeof navigateGame==='function')navigateGame('expeditions-panel');else if(typeof originalPick==='function')originalPick(ziel,k);return;}
  if(ds.esSonde){await sondeSchicken(ds.esSonde,b);return;}
  if(ds.esFolge){const ziel=z.ziel,zielart=z.zielart;await missionSetzen(ds.esFolge);if(!z)return;z.ziel=ziel;z.zielart=zielart;geheZu('flotte');return;}
  if(ds.esGo){schliessen();if(typeof navigateGame==='function')navigateGame(ds.esGo);return;}
  if(ds.esVersorgen){const id=Number(ds.esVersorgen),f=gestrandeteFlotte(id);if(f){schliessen();oeffnen({ziel:f.ziel,mission:'transport',versorgen:id});}return;}
  if(ds.esForschung){const k=ds.esForschung;schliessen();if(typeof navigateGame==='function')navigateGame('forschen');if(window.Bauseite?.oeffne)Bauseite.oeffne('forschen',k);return;}
  if(ds.esGalaxie){const k=ds.esGalaxie;schliessen();if(window.GalaxieTabelle?.zeigen)GalaxieTabelle.zeigen(k);else if(typeof navigateGame==='function'){navigateGame('galaxie');const [s,sy]=k.split(':');const f=el('galaxy-form')?.elements;if(f?.sektor&&f?.system){f.sektor.value=s;f.system.value=sy;f.requestSubmit?.();}}return;}
  if(ds.esProfil){if(window.Spielerprofil?.oeffnen)Spielerprofil.oeffnen(ds.esProfil);return;}
  if(ds.esStart){startWaehlen(ds.esStart,ds.esStartart||'planet');if(z.mission==='kolonisieren'){z.bedarfSchluessel='';await bedarfLaden();if(!z)return;mischungSetzen('bewohner');frachtSetzen('mindest');}zeichnen();bald();return;}
  if(ds.esVorlage){vorlageAnwenden(ds.esVorlage);zeichnen();return;}
  if(ds.esMinus||ds.esPlus||ds.esMax){const [art,...rest]=(ds.esMinus||ds.esPlus||ds.esMax).split(':'),k=rest.join(':');setzen(art,k,ds.esMax?maxVon(art,k):wertVon(art,k)+(ds.esPlus?1:-1)*schrittweite(art,k));zeichnen();if(art!=='schiff')bald();return;}
  if(ds.esFracht==='aufbau'&&z.mission==='kampfkolonisieren'){await wiederaufbauFracht();if(!z)return;if(fracht()>laderaum()){z.flug=null;frachterErgaenzen();}z.plan=null;zeichnen();bald();return;}
  if(ds.esFracht==='heimflug'){await heimflugFracht();if(!z)return;if(fracht()>laderaum()){z.flug=null;frachterErgaenzen();}z.plan=null;zeichnen();bald();return;}
  if(ds.esFracht){frachtSetzen(ds.esFracht);if(fracht()>laderaum()&&ds.esFracht!=='voll'){const vorher=laderaum();z.flug=null;if(fracht()>vorher)frachterErgaenzen();}z.plan=null;zeichnen();bald();return;}
  if('esFrachter' in ds){frachterErgaenzen();zeichnen();return;}
  if('esPersonenplaetze' in ds){personenplaetzeErgaenzen();zeichnen();return;}
  if(ds.esPersonen){personenFuellen(ds.esPersonen);zeichnen();return;}
  if('esTruppentransporter' in ds){truppentransporterErgaenzen();zeichnen();return;}
  if(ds.esTruppen){truppenVorlage(ds.esTruppen);zeichnen();bald();return;}
  if(ds.esVolk){z.volk=ds.esVolk;z.bedarfSchluessel='';await bedarfLaden();if(!z)return;mischungSetzen(z.mischungArt==='eigen'?'bewohner':z.mischungArt);frachtSetzen(z.frachtArt==='eigen'?'mindest':z.frachtArt||'mindest');zeichnen();return;}
  if(ds.esMischung){mischungSetzen(ds.esMischung);zeichnen();return;}
  if(ds.esStunden!=null&&ds.esStunden!==undefined&&b.matches('[data-es-stunden]')){z.stunden=Number(ds.esStunden);z.plan=null;zeichnen();bald();return;}
  if(ds.esHalt!=null&&b.matches('[data-es-halt]')){z.stunden=Number(ds.esHalt)||0;z.offen.tempowahl='';z.plan=null;zeichnen();bald();return;}
  if(ds.esTwAuf){z.offen.tempowahl=z.offen.tempowahl===ds.esTwAuf?'':ds.esTwAuf;zeichnen();if(z.offen.tempowahl&&!tempiListe())bald();return;}
  if(ds.esTempoHin){z.tempo=Number(ds.esTempoHin)||1;z.offen.tempowahl='';z.plan=null;if(!tempiListe())z.flug=null;zeichnen();bald();return;}
  if(ds.esTempoRueck!=null&&b.matches('[data-es-tempo-rueck]')){const v=Number(ds.esTempoRueck);z.ruecktempo=Number.isFinite(v)?v:1;z.offen.tempowahl='';z.plan=null;zeichnen();bald();return;}
  if(ds.esAnkunft){z.ankunftArt=ds.esAnkunft;if(z.ankunftArt==='uhrzeit'&&z.ankunftZeit==null){const f=fahrplan();if(f)z.ankunftZeit=Math.ceil((f.jetzt+f.hin.dauer)/300)*300;}if(z.ankunftArt==='kopplung'&&!z.kopplung.anker)z.kopplung.anker=kopplungsAnker()[0]?.id||'';zeichnen();return;}
  if(ds.esVersatz!=null&&b.matches('[data-es-versatz]')){z.kopplung.versatz=Number(ds.esVersatz)||0;z.kopplungManuell=true;zeichnen();return;}
  if(ds.esTfvolk){z.tfVolk=ds.esTfvolk;z.tf=null;await terraformLaden();if(!z)return;standardVorlage();zeichnen();return;}
  if(ds.esUebergabe){z.verkauf=ds.esUebergabe==='verkauf';zeichnen();return;}
  if(ds.esInfo){z.info=ds.esInfo;zeichnen();setTimeout(()=>el('einsatz-blatt')?.querySelector('.es-us-zurueck')?.focus({preventScroll:true}),30);return;}
  if('esInfoZu' in ds){z.info='';zeichnen();return;}
  if(ds.esSchiffinfo){if(window.Bauseite?.techinfo)Bauseite.techinfo('ships',ds.esSchiffinfo);return;}
  if('esSofort' in ds){frachtSetzen('voll');z.stunden=0;z.tempo=1;z.ruecktempo=1;const g=flottenFehler()||detailFehler()||(bedingungen().find(b=>!b.ok&&b.vorrang)||bedingungen().find(b=>!b.ok))?.text;if(g){z.fehler=g;zeichnen();return;}await starten();return;}
  if('esSim' in ds){b.disabled=true;b.setAttribute('aria-busy','true');try{await simulieren();}catch(err){if(z){z.fehler=err.message;zeichnen();}}return;}
  if('esFormular' in ds){const a=aktion(),ziel=z.ziel;schliessen();if(window.Flottenfuehrung?.ausHangar)Flottenfuehrung.ausHangar({start:a.start,startart:a.startart||'planet',schiffe:a.schiffe,ziel,zielart:a.zielart||'planet',mission:a.mission,klassisch:true});else if(typeof originalPick==='function')originalPick(ziel,a.mission);return;}
 }
 function klappe(e){const d=e.target;if(!z||!d.matches?.('details[data-es-klapp]'))return;z.offen[d.dataset.esKlapp]=d.open;if(d.dataset.esKlapp==='regel'&&!d.open)alsGesehen(z.mission);}
 let suchUhr=0;
 function eingabe(e){
  const t=e.target;if(!z)return;const ds=t.dataset;
  if(t.matches('[data-es-suche]')){z.suche=t.value;clearTimeout(suchUhr);suchUhr=setTimeout(zeichnen,120);return;}
  if(ds.esZahl){if(e.type==='input'&&t.value==='')return;const [art,...rest]=ds.esZahl.split(':'),k=rest.join(':');setzen(art,k,t.value);if(e.type==='change'){t.value=String(wertVon(art,k));zeichnen();}else spaeter();if(art!=='schiff')bald();return;}
  if(t.matches('[data-es-regler]')){mischungAendern(Number(ds.esRegler),Number(t.value));spaeter();return;}
  if(t.matches('[data-es-stundenregler]')){z.stunden=Number(t.value);z.plan=null;spaeter();if(e.type==='change')bald();return;}
  if(t.matches('[data-es-bestaetigt]')){z.bestaetigt=t.checked;zeichnen();return;}
  if((t.matches('[data-es-termin-tag]')||t.matches('[data-es-termin-uhr]'))&&e.type==='change'){const b=el('einsatz-blatt'),tag=Math.max(1,Number(b.querySelector('[data-es-termin-tag]')?.value)||1),[hh,mm,ss]=String(b.querySelector('[data-es-termin-uhr]')?.value||'0:0:0').split(':').map(n=>Number(n)||0);z.ankunftZeit=(tag-1)*86400+hh*3600+mm*60+ss;zeichnen();return;}
  if(t.matches('[data-es-kopplung]')){z.kopplung.anker=t.value;zeichnen();return;}
  if(t.matches('[data-es-verband]')){z.verband=t.value;zeichnen();return;}
  if(t.matches('[data-es-preis]')){z.preis=Math.max(0,Number(t.value)||0);if(e.type==='change')zeichnen();return;}
 }

 // ---------------------------------------------------------------- Einstiege
 // pickFleet(ziel,mission) aus Galaxie, Berichten, Planetenkarten und Android-Overlay öffnet den Einsatz. Aufrufer, die danach noch
 // Zielart/Startort im Flottenformular setzen (Mond als Ziel oder Start), werden respektiert: erst nach dem Aufruf lesen.
 let originalPick=null;
 function umhuellen(){
  const aktuell=window.pickFleet;if(typeof aktuell!=='function'||aktuell.esUmhuellt)return;
  originalPick=aktuell.koloOriginal||aktuell;
  const huelle=function(k,m){
   const roh=String(m||''),mission=roh==='schiffhandel'?'uebergeben':roh,ziel=String(k||'').trim();
   if(IMMER_AUSSEN.includes(mission)||(mission&&!UNTERSTUETZT.has(mission)))return aktuell.apply(this,arguments);
   const f=el('fleet')?.elements;if(f){try{if(f.ziel)f.ziel.value=ziel;if(f.mission&&mission)f.mission.value=mission;if(f.zielart)f.zielart.value='planet';if(f.startart)f.startart.value='planet';}catch(_){}}
   setTimeout(()=>{const ff=el('fleet')?.elements;const zielart=ff?.zielart?.value==='mond'?'mond':'planet',startart=ff?.startart?.value==='mond'?'mond':'planet',start=startart==='mond'?ff?.start?.value:'';
    oeffnen({ziel,mission,zielart,startart,start,verkauf:roh==='schiffhandel'}).then(ok=>{if(!ok)aktuell.call(window,k,m);});},0);
  };
  huelle.esUmhuellt=true;huelle.koloUmhuellt=true;huelle.ffUmhuellt=aktuell.ffUmhuellt;window.pickFleet=huelle;
 }
 // Auftragskarte „Kolonisieren“ o. ä. im Flottenformular mit gültigem Ziel: gleich den Einsatz öffnen.
 document.addEventListener('click',e=>{
  const b=e.target.closest('[data-ff-mission],[data-ff-auftrag]');if(!b||b.closest('#einsatz-blatt')||b.getAttribute('aria-disabled')==='true')return;
  const m=b.dataset.ffMission||b.dataset.ffAuftrag;if(!UNTERSTUETZT.has(m)||!['kolonisieren','terraformen','abbau','recyceln'].includes(m))return;
  const ziel=(b.dataset.ffZiel||el('fleet')?.elements?.ziel?.value||'').trim();if(!KOORD.test(ziel))return;
  e.preventDefault();e.stopPropagation();oeffnen({ziel,mission:m,start:el('fleet')?.elements?.start?.value});
 },true);
 // Flotte gesamt: „Flug planen“ und „Flotte von hier“ öffnen ebenfalls den Einsatz (ein Weg statt zwei). Verband, Ankunft
 // planen und koppeln bietet der Start-Schritt selbst (ankunftHtml, verbandHtml).
 function hangarUmhuellen(){
  const F=window.Flottenfuehrung;if(!F?.ausHangar||F.ausHangar.esUmhuellt)return;const alt=F.ausHangar;
  const neu_=function(o={}){if(o.klassisch||!planeten().length)return alt.call(this,o);const m=o.direktFlotte&&!o.ziel?'':UNTERSTUETZT.has(o.mission)?o.mission:'';// „Flotte ab hier“: Auftrag wählt der Spieler nach dem Ziel
   oeffnen({ziel:o.ziel||'',zielart:o.zielart,start:o.start,startart:o.startart,schiffe:o.schiffe,mission:m}).then(ok=>{if(!ok)alt.call(this,o);});};
  neu_.esUmhuellt=true;F.ausHangar=neu_;
 }
 document.addEventListener('click',e=>{const b=e.target.closest('.hg-planen[data-fleet-section="fleet-planner"]');if(!b||!window.Einsatz)return;e.preventDefault();e.stopImmediatePropagation();
  oeffnen({});},true);
 // Angriffswarnung → Flotte retten (Ereignisliste): Ausweichziel vorschlagen – freier bekannter Platz im selben System,
 // sonst ein anderer eigener Planet, sonst der Gürtel – und den Save-Einsatz mit „Jetzt retten“ öffnen (zwei Tipps).
 function ausweichziel(k){
  const sys=systemVon(k),frei=atlas().filter(a=>systemVon(a.koord)===sys&&!a.bewohnt&&!a.spieler&&a.koord!==k).sort((a,b)=>Math.abs(a.position-Number(k.split(':')[2]))-Math.abs(b.position-Number(k.split(':')[2])));
  if(frei[0])return frei[0].koord;const andere=planeten().find(p=>p.koord!==k);if(andere)return andere.koord;return sys+':0';
 }
 document.addEventListener('click',e=>{const b=e.target.closest('[data-saven-planet]');if(!b||!window.Einsatz)return;e.preventDefault();e.stopPropagation();
  const k=b.dataset.savenPlanet;oeffnen({ziel:ausweichziel(k),mission:'saven',start:k,startart:'planet'});},true);
 document.addEventListener('DOMContentLoaded',hangarUmhuellen);window.addEventListener('load',hangarUmhuellen);
 // Gestrandete Flotten aus Flottenkarten, Galaxie und Berichten: Weiterfliegen, Versorgen, Übergeben (außerhalb des Blatts).
 document.addEventListener('click',e=>{const b=e.target.closest('[data-es-gestrandet],[data-es-versorgen],[data-es-uebergeben]');if(!b||b.closest('#einsatz-blatt'))return;
  e.preventDefault();const ds=b.dataset;
  if(ds.esGestrandet)oeffnen({gestrandet:Number(ds.esGestrandet)});
  else if(ds.esVersorgen){const f=gestrandeteFlotte(ds.esVersorgen);if(f)oeffnen({ziel:f.ziel,mission:'transport',versorgen:Number(ds.esVersorgen)});}
  else if(ds.esUebergeben)oeffnen({gestrandet:Number(ds.esUebergeben),mission:'uebergeben'});
 });
 document.addEventListener('DOMContentLoaded',()=>{umhuellen();window.Rahmen?.zurueckHaken?.push(()=>{if(!z||el('einsatz-blatt')?.hidden)return false;return zurueckSchritt();});});
 document.addEventListener('game-navigation',umhuellen);
 window.addEventListener('load',umhuellen);
 // Frische Bestände nach jeder Aktualisierung zeigen, ohne eine gerade bediente Eingabe zu stören.
 let letzteRevision=null;setInterval(()=>{if(document.hidden||!z||el('einsatz-blatt')?.hidden)return;const r=String(V()?.revision);
  if(r!==letzteRevision){const erst=letzteRevision===null;letzteRevision=r;const a=document.activeElement;if(!erst&&!(a&&el('einsatz-blatt')?.contains(a)&&['INPUT','SELECT'].includes(a.tagName)))zeichnen();}},1500);
 window.Einsatz={oeffnen,schliessen,offen:()=>!!z,zustand:()=>z&&JSON.parse(JSON.stringify(z)),unterstuetzt:m=>UNTERSTUETZT.has(m),profile:P};
 // Ältere Einstiege leiten auf den Einsatz um (eine Planung statt drei Dialoge – keine Widersprüche zwischen ihnen).
 window.Kolonisierung={oeffnen:(ziel,start)=>oeffnen({ziel,mission:'kolonisieren',start}),umhuellen,zustand:()=>window.Einsatz.zustand()};
 window.Einsatzplanung={...(window.Einsatzplanung||{}),oeffnen:(ziel,mission,start)=>oeffnen({ziel,mission:mission||'abbau',start})};
})();
