// Offline/LAN presentation. The shared game, discovery and bot policies live in Rust.
window.addEventListener('DOMContentLoaded', () => {
  if (!window.STERNENEPOCHE?.local) return;
  document.documentElement.classList.add('local-play');
  document.querySelector('header .brand span').textContent = 'Lokale Welt';
  const footer = document.querySelector('footer');
  if (footer) for (const n of footer.childNodes) if (n.nodeType === 3 && n.textContent.includes('20 mögliche Spielerplätze')) n.textContent = ' · Lokale Welt · Mensch, Agent oder Gemischt · Spielerplätze nach Weltgröße und Hardware';
  // Beitritt zur eigenen Welt: keine Teilnehmerplätze, keine Warteliste, kein Betreiber (Karl 19.10.2026 am Handy).
  const claim = document.getElementById('claim-panel');
  if (claim) {
    const erster = claim.querySelector(':scope > p'); if (erster) erster.textContent = 'Wähle dein Volk und wie du spielst. Danach beginnt dein Reich sofort.';
    const warte = document.getElementById('waitlist-leave')?.nextElementSibling; if (warte?.classList.contains('muted')) warte.hidden = true;
  }
  document.getElementById('authenticator-panel').hidden = true;
  document.getElementById('registration-flow').hidden = true;
  document.getElementById('offline-entry').hidden = true;
  const login = document.getElementById('login');
  login.elements.otp.closest('label').hidden = true;
  login.elements.password.closest('label').firstChild.textContent = 'Masterpasswort des Hosts ';
  document.getElementById('account-panel').querySelector('p').textContent = 'Mit Name und Masterpasswort der lokalen Sitzung beitreten. Keine E-Mail erforderlich.';
  const hint = document.querySelector('#account-panel .muted');
  if (hint) hint.textContent = 'Dein Name behält seinen Spielplatz in dieser privaten Runde.';
  fetch('/api/local/info').then(r => r.ok ? r.json() : null).then(info => {
    if (info?.host !== false) return;
    login.elements.password.required = false;
    login.elements.password.value = '';
    login.elements.password.closest('label').hidden = true;
    document.getElementById('account-panel').querySelector('p').textContent = 'Singleplayer auf diesem PC: Dein Name genügt. Kein Passwort und keine E-Mail erforderlich.';
  }).catch(() => {});
  const panel = document.createElement('details');
  panel.className = 'panel';
  const summary = document.createElement('summary'); summary.textContent = 'Lokaler Bot-Funk'; panel.append(summary);
  for (const [id, name] of [['oekonom','Ökonom'],['haendler','Händler'],['igel','Verteidiger'],['raeuber','Räuber']]) {
    const row = document.createElement('p'), label = document.createElement('strong'), audio = document.createElement('audio');
    label.textContent = name + ' '; audio.controls = true; audio.preload = 'none'; audio.src = 'audio/bots/' + id + '.ogg'; audio.setAttribute('aria-label', 'Funkspruch ' + name);
    row.append(label, audio); panel.append(row);
  }
  document.getElementById('briefkasten').prepend(panel);
});

// Account-private navigation lives beside the world, so a new loopback port does
// not lose the selected tab, colony or galaxy level when the cockpit restarts.
window.LocalResume=(()=>{
  let owner='',ready=false,restoring=false,last='',scrollTimer;
  function state(){
    return {tab:document.body.dataset.bereich||document.querySelector('[data-tab][aria-selected="true"]')?.dataset.tab||'reich',
      planet:typeof selectedPlanet==='string'?selectedPlanet:'',
      galaxy:window.GalaxyUI?.snapshot()||{stage:'galaxy',sektor:1,system:1},
      pins:window.PlanetNavigation?.snapshot()||[],scroll:Math.max(0,Math.round(window.scrollY)),display:{size:DisplayPreferences.size,ambience:DisplayPreferences.ambience}};
  }
  function save(){
    if(!window.STERNENEPOCHE?.local||!ready||restoring||!session||!world)return;
    const s=state(),text=JSON.stringify(s);if(last===text)return;last=text;
    const key=owner;
    // Kein keepalive: das Android-WebView lehnt keepalive-Anfragen mit eigenen Kopfzeilen ab („Failed to fetch“ bei
    // jedem Seitenwechsel, 19.10.2026). Das Merken der Ansicht ist Komfort – ein Fehlschlag wird still wiederholt, nie gemeldet.
    fetch(base+'/api/local/viewprefs',{method:'POST',headers:{'Content-Type':'application/json',Authorization:'Bearer '+session.token,'X-Sternenepoche-Oberflaeche':'browser'},body:JSON.stringify({world_id:world.world_id,state:s})})
      .then(async r=>{if(!r.ok)throw Error((await r.json().catch(()=>({}))).error||'Spielansicht nicht gespeichert');})
      .catch(e=>{if(owner===key){last='';console.warn('Spielansicht nicht gemerkt:',e.message);}});
  }
  async function restore(){
    if(!window.STERNENEPOCHE?.local||!session||!view||!world)return;
    const key=world.world_id+':'+session.name;if(owner===key)return;
    owner=key;ready=false;restoring=true;
    try{
      const p=await api('/api/local/viewprefs');
      if(p.world_id!==world.world_id)throw Error('Spielansicht gehört zu einer anderen Welt');
      const s=p.state;
      if(s){
        if(s.pins)PlanetNavigation.restore(s.pins);
        if(s.display&&['normal','large','largest'].includes(s.display.size)){DisplayPreferences.size=s.display.size;DisplayPreferences.ambience=s.display.ambience;DisplayPreferences.save();}
        if(view.planeten.some(p=>p.koord===s.planet)){selectedPlanet=s.planet;renderView();}
        navigateGame(s.tab);
        // Die 3D-Galaxie ist optional: scheitert ihr Wiederherstellen, bleibt die Seite trotzdem stehen.
        if(s.tab==='karte')try{await window.GalaxyUI?.restore(s.galaxy);}catch(_){}
        requestAnimationFrame(()=>window.scrollTo(0,s.scroll||0));
      }
      ready=true;
    }catch(e){
      // Nicht erneut versuchen: app.js ruft restore() nach jeder Aktualisierung auf – ein Rücksetzen von owner
      // zwang sonst bei jedem Tipp zurück auf die gemerkte Seite (APK 0.4.0, Karl 18.10.).
      ready=true;console.warn('Spielansicht nicht wiederhergestellt',e);
    }
    finally{restoring=false;}
  }
  document.addEventListener('game-navigation',save);
  document.addEventListener('local-galaxy-view',save);
  document.addEventListener('click',e=>{if(e.target.closest('[data-planet-pick],[data-space-own]'))save();});
  window.addEventListener('scroll',()=>{clearTimeout(scrollTimer);scrollTimer=setTimeout(save,200);},{passive:true});
  // Navigation is saved immediately; closing an inactive second window must not overwrite it.
  return {restore,save};
})();
