'use strict';

// Icons have a fixed optical centre; labels remain free to wrap in any language.
function uiIcon(name){
 const paths={left:'m14 6-6 6 6 6',right:'m10 6 6 6-6 6',down:'m6 9 6 6 6-6',close:'m6 6 12 12M18 6 6 18',arrow:'M7 17 17 7M7 7h10v10',search:'M21 21l-5-5M18 10a8 8 0 1 1-16 0 8 8 0 0 1 16 0',star:'m12 3 2.8 5.7 6.2.9-4.5 4.4 1.1 6.2-5.6-3-5.6 3 1.1-6.2L3 9.6l6.2-.9Z',check:'m5 12 4 4L19 6',signal:'M5 12h2l3-7 4 14 3-7h2',shield:'M12 3 4 6v6c0 5 8 9 8 9s8-4 8-9V6Z',flask:'M9 3h6M10 3v7l-6 9q-1 2 2 2h12q3 0 2-2l-6-9V3M8 15h8'};
 return `<svg class="ui-icon" data-icon="${name}" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true" focusable="false"><path d="${paths[name]||paths.arrow}"/></svg>`;
}

// Display preferences never change simulation speed, commands or save data.
const DisplayPreferences = {
 key:'sternenepoche-display', size:'normal', ambience:true,
 load(){try{const p=JSON.parse(localStorage.getItem(this.key)||'{}');if(['normal','large','largest'].includes(p.size))this.size=p.size;if(typeof p.ambience==='boolean')this.ambience=p.ambience;}catch{}this.apply();},
 apply(){document.body.dataset.textSize=this.size;document.body.dataset.ambience=String(this.ambience);document.body.dataset.pageHidden=String(document.hidden);},
 save(){this.apply();try{localStorage.setItem(this.key,JSON.stringify({size:this.size,ambience:this.ambience}));}catch{}window.LocalResume?.save();},
 markup(){return `<section class="display-preferences"><h3>Lesbarkeit & Atmosphäre</h3><p>Schriftgröße für dieses Gerät</p><div class="text-size-options" role="group" aria-label="Schriftgröße">${[['normal','Kompakt'],['large','Größer +15 %'],['largest','Sehr groß +30 %']].map(([v,label])=>`<button type="button" data-text-size="${v}" aria-pressed="${this.size===v}">${label}</button>`).join('')}</div><label class="ambience-option"><input type="checkbox" data-ambience ${this.ambience?'checked':''}> Dezente Hintergrundereignisse</label><p class="preference-note">Asteroiden und Schiffe sind reine Dekoration. Bei reduzierter Bewegung bleiben sie aus.</p>${window.PlanetKugel?.markup()||''}</section>`;},
 open(){const d=$('display-settings');setStableHTML(d, `<div class="settings-title"><h2>Darstellung</h2><button type="button" data-close-display aria-label="Darstellung schließen">×</button></div>${this.markup()}`);d.showModal();}
};
DisplayPreferences.load();

// Two favourites, one complete selector. Stored per world/account, never as game commands.
window.PlanetNavigation={
 owner:'',pins:[],stamp:'',
 ensure(){const key=world.world_id+':'+session.name;if(key===this.owner)return;this.owner=key;this.stamp='';try{const saved=JSON.parse(localStorage.getItem('sternenepoche-planets:'+key)||'null');this.pins=Array.isArray(saved)?saved.slice(0,2):view.planeten.slice(0,2).map(p=>p.koord);}catch{this.pins=view.planeten.slice(0,2).map(p=>p.koord);}},
 snapshot(){this.ensure();this.pins=this.pins.map(k=>view.planeten.some(p=>p.koord===k)?k:'');return [this.pins[0]||'',this.pins[1]||''];},
 restore(pins){this.ensure();if(Array.isArray(pins)){this.pins=pins.slice(0,2);this.snapshot();this.stamp='';this.save();}},
 save(){try{localStorage.setItem('sternenepoche-planets:'+this.owner,JSON.stringify(this.pins));}catch{}window.LocalResume?.save();},
 select(koord){if(!view.planeten.some(p=>p.koord===koord))return;window.ColonyConsole?.onPlanetChange();selectedPlanet=koord;$('manufacture').elements.planet.value=koord;$('fleet').elements.start.value=koord;renderView();window.LocalResume?.save();},
 render(){
  const pins=this.snapshot(),p=currentPlanet(),host=$('planet-sidebar');
  const stamp=JSON.stringify([this.owner,p.koord,pins,view.planeten.map(x=>[x.koord,x.heimat,x.zone,x.name]),view.kolonien.erlaubt]);
  if(this.stamp===stamp&&host.querySelector('#planet-selector'))return;this.stamp=stamp;
  const card=k=>{const x=view.planeten.find(p=>p.koord===k);return x?`<button type="button" class="planet-pick" data-planet-pick="${esc(k)}" aria-label="${esc(Planetenname.name(x))} [${esc(k)}] auswählen · ${esc(named(x.zone))}" aria-pressed="${k===p.koord}"><img src="${esc(planetArt(x))}" alt="" loading="lazy"><span><strong>${esc(Planetenname.name(x))}</strong><small>[${esc(k)}] · ${esc(named(x.zone))}</small></span></button>`:`<button type="button" class="planet-pin-empty" data-edit-pins><span aria-hidden="true">＋</span> Planet anheften</button>`;};
  setStableHTML(host, `<div class="planet-navigation-title"><span>PLANETEN <b>${view.planeten.length} / ${view.planetenkapazitaet?.maximal||15}</b></span></div><div class="planet-favourites">${pins.map(card).join('')}</div><div class="planet-selector-row"><button id="planet-selector" type="button" class="planet-list-trigger" data-open-planets aria-haspopup="dialog" aria-expanded="false" aria-controls="planet-picker" aria-label="Alle ${view.planeten.length} Planeten durchsuchen"><span>Alle Planeten<small>${esc(p.koord)}</small></span>${uiIcon('down')}</button><button type="button" class="ui-icon-button" data-edit-pins aria-label="Zwei Planeten-Favoriten bearbeiten" title="Favoriten bearbeiten">${uiIcon('star')}</button></div>`);
 },
 open(){const d=$('planet-pins-dialog'),pins=this.snapshot();setStableHTML(d, `<div class="settings-title"><h2>Planeten anheften</h2><button type="button" data-close-pins aria-label="Favoriten schließen">×</button></div><p>Zwei Planeten direkt erreichbar. Alle weiteren bleiben im Dropdown. Beim Wechsel bleibt dein Spielbereich geöffnet.</p>${[0,1].map(i=>`<label>Favorit ${i+1}<select data-pin-slot="${i}" aria-label="Favorit ${i+1}"><option value="">Nicht belegt</option>${view.planeten.map(p=>`<option value="${esc(p.koord)}"${pins[i]===p.koord?' selected':''}>${esc(p.koord)} · ${p.heimat?'Heimatwelt':esc(named(p.zone))}</option>`).join('')}</select></label>`).join('')}`);d.showModal();}
};

function tileCostMarkup(cost,stock){return `<span class="tile-materials">${Object.entries(cost).filter(([,n])=>n>0).map(([g,n])=>{const missing=stock?Math.max(0,Math.ceil(n-(stock[g]||0))):0;return `<span class="tile-material" title="${esc(named(g))}: ${fmt(n)}${stock?' · vorhanden '+fmt(stock[g]||0):''}"><img src="${esc(art('resources',g))}" alt="${esc(named(g))}" width="14" height="14"><b>${fmt(n)}</b>${missing?`<small class="material-shortfall">fehlen ${fmt(missing)}</small>`:''}</span>`;}).join('')}</span>`;}

function renderCommandDeck(){
 if(!view)return;
 const p=currentPlanet(),work=p.arbeit,special=p.fachkraefte,energy=p.energie.erzeugung-p.energie.verbrauch;
 document.body.dataset.humanControl=String(session?.mode==='mensch'||session?.mode==='human');
 const location=$('event-bar').querySelector('.event-location');
 if(location)setStableHTML(location, `<img src="${esc(planetArt(p))}" alt=""><span><strong>${esc(Planetenname.name(p))}</strong><small>[${esc(p.koord)}] · ${esc(named(p.zone))}</small></span>${Planetenname.stift(p.koord,'kopf-stift')}`);
 const metric=(name,value,detail,low=false)=>`<button type="button" class="planet-metric ${low?'metric-low':''} ${['Bevölkerung','Fachkräfte','Forscher'].includes(name)?'metric-flow':''}" data-go="kolonie" title="${esc(name+': '+value+' · '+detail)}"><span>${name}</span><strong>${value}</strong><small>${detail}</small></button>`;
 $('event-bar').querySelector('.planet-status')?.remove();
 const speed=world.tempo||1,r=window.Mobile035.researchers(p);
 $('event-bar').insertAdjacentHTML('beforeend',`<div class="planet-status" aria-label="Bevölkerung und Versorgung des ausgewählten Planeten">${metric('Bevölkerung',fmt(p.bevoelkerung),fmt(p.wohnraum)+' Wohnplätze')}${metric('Arbeitskräfte',fmt(work.verfuegbar),fmt(work.bedarf)+' benötigt',work.verfuegbar<work.bedarf)}${metric('Fachkräfte',fmt(special.verfuegbar),fmt(special.bedarf)+' benötigt',special.verfuegbar<special.bedarf)}${metric('Forscher',fmt(r.zugewiesen||0)+' / '+fmt(r.plaetze||0),r.plaetze?workforceRate((r.fp_je_spielstunde||0)*speed)+' FP / Echtzeit-h':'Labor bauen',r.zugewiesen<r.plaetze)}${metric('Baufläche',p.felder.belegt+' / '+p.felder.gesamt,'Felder belegt')}${metric('Stabilität',p.stabilitaet+' %',p.blockade?'Blockade aktiv':'Keine Blockade',p.stabilitaet<50)}</div>`);
 // Keep the quick navigation useful even far below the first screen.
 document.querySelectorAll('.overview-tools [data-go="codex"]').forEach(b=>b.classList.add('codex-launch'));
 const back=$('back-to-top');if(back)back.hidden=window.scrollY<450;
}

document.addEventListener('DOMContentLoaded',()=>{
 const nav=$('tabs');nav.classList.add('command-navigation');
 const label=nav.querySelector('.section-picker-label');if(label)label.firstChild.textContent='Bereich';
 nav.insertAdjacentHTML('beforeend','<div class="command-nav-tools"><button type="button" class="codex-launch" data-go="codex" aria-label="Codex öffnen: Schiffe, Gebäude und Technologien"><span aria-hidden="true">◇</span> Codex</button></div>');
 const profile=nav.querySelector('[data-tab="profil"]');if(profile)profile.textContent='Profil & Einstellungen';
 const option=$('section-picker').querySelector('option[value="profil"]');if(option)option.textContent='Profil & Einstellungen';
 document.body.insertAdjacentHTML('beforeend','<dialog id="display-settings" class="item-info-dialog" aria-label="Darstellung und Lesbarkeit"></dialog><dialog id="planet-pins-dialog" class="item-info-dialog" aria-label="Planeten-Favoriten"></dialog><div id="space-ambience" aria-hidden="true"><i class="ambient-asteroid"></i><i class="ambient-ship"></i></div>');
 document.addEventListener('visibilitychange',()=>DisplayPreferences.apply());
 window.addEventListener('scroll',()=>{const b=$('back-to-top');if(b)b.hidden=window.scrollY<450;},{passive:true});
});
document.addEventListener('click',e=>{
 const b=e.target.closest('[data-display-settings],[data-close-display],[data-text-size]');if(!b)return;
 if(b.hasAttribute('data-display-settings')){DisplayPreferences.open();return;}
 if(b.hasAttribute('data-close-display')){$('display-settings').close();return;}
 DisplayPreferences.size=b.dataset.textSize;DisplayPreferences.save();
 document.querySelectorAll('[data-text-size]').forEach(n=>n.setAttribute('aria-pressed',String(n.dataset.textSize===DisplayPreferences.size)));
});
document.addEventListener('change',e=>{if(e.target.hasAttribute('data-ambience')){DisplayPreferences.ambience=e.target.checked;DisplayPreferences.save();document.querySelectorAll('[data-ambience]').forEach(n=>n.checked=DisplayPreferences.ambience);}});
document.addEventListener('click',e=>{if(e.target.closest('[data-edit-pins]'))PlanetNavigation.open();if(e.target.closest('[data-close-pins]'))$('planet-pins-dialog').close();});
document.addEventListener('change',e=>{
 if(e.target.hasAttribute('data-pin-slot')){const slot=Number(e.target.dataset.pinSlot),k=e.target.value,pins=PlanetNavigation.snapshot();if(k&&pins[1-slot]===k)pins[1-slot]=pins[slot];pins[slot]=k;PlanetNavigation.pins=pins;PlanetNavigation.save();PlanetNavigation.render();document.querySelectorAll('[data-pin-slot]').forEach(n=>n.value=pins[Number(n.dataset.pinSlot)]);}
});
