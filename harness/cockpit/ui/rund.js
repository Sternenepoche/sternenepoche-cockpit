/* Kreisbühne: der ganze Rat im Kreis auf einer Plattform – zum Vorstellen im Einrichtungsassistenten.
   Ziehen dreht den Kreis (mit Schwung, rastet auf ein Ratsmitglied ein), Klick holt eine Figur nach vorn,
   ohne Eingriff stellt der Kreis nacheinander jede Figur vor. Effekte liegen nie über einer Figur:
   erst Plattform, Lichtsäulen und Funken, dann die Figuren in einem eigenen Durchgang. */
"use strict";

const Rund = (() => {
  const T = window.THREE;
  const R = 1.3, SCHRITT = Math.PI / 2;
  let inst = null;

  function glbPfad(r) { const p = `figuren/${r}.${gestalt(r)}.glb`; return (S.katalog?.figuren || []).includes(p) ? p : null; }
  function bildPfad(r) { const p = `figuren/${r}.${gestalt(r)}.webp`; return (S.katalog?.figuren || []).includes(p) ? p : portraet(r); }
  function vorne() { return inst ? ROLLEN[inst.vorne] : (S.ui.rund?.vorne || ROLLEN[0]); }

  /** Platz für die Bühne; die Szene selbst baut pflegen() nach dem Zeichnen. */
  function element(hoehe) {
    return h("div.rundbuehne", { "data-fest": "1", "data-k": "rund", stil: { height: (hoehe || 380) + "px" } },
      h("div.rund-titel"), h("div.hinweis", null, "Ziehen dreht den Rat · Klick holt eine Figur nach vorn"));
  }

  function lichtsaeule(farbe) {
    const c = document.createElement("canvas"); c.width = 4; c.height = 128;
    const g = c.getContext("2d"), v = g.createLinearGradient(0, 128, 0, 0);
    v.addColorStop(0, "rgba(255,255,255,0.32)"); v.addColorStop(0.3, "rgba(255,255,255,0.07)"); v.addColorStop(1, "rgba(255,255,255,0)");
    g.fillStyle = v; g.fillRect(0, 0, 4, 128);
    const m = new T.Mesh(new T.CylinderGeometry(0.34, 0.44, 2.4, 48, 1, true), new T.MeshBasicMaterial({ map: new T.CanvasTexture(c), color: farbe, transparent: true, depthWrite: false, side: T.DoubleSide, blending: T.AdditiveBlending }));
    m.position.y = 1.3;
    return m;
  }

  function aufbauen(el) {
    const leinwand = document.createElement("canvas");
    el.insertBefore(leinwand, el.firstChild);
    const renderer = new T.WebGLRenderer({ canvas: leinwand, antialias: true, alpha: true });
    renderer.setPixelRatio(Math.min(2, window.devicePixelRatio || 1));
    if ("outputColorSpace" in renderer) renderer.outputColorSpace = T.SRGBColorSpace; else renderer.outputEncoding = T.sRGBEncoding;
    renderer.toneMapping = T.ACESFilmicToneMapping; renderer.toneMappingExposure = 1.35; renderer.autoClear = false;
    const szene = new T.Scene(), fx = new T.Scene();
    if (Dreid.studio) szene.environment = Dreid.studio(renderer);
    const kamera = new T.PerspectiveCamera(30, 1, 0.05, 100);
    szene.add(new T.HemisphereLight(0xcfe6ff, 0x1c140b, 0.9));
    const schluessel = new T.DirectionalLight(0xffe6c4, 2.4); schluessel.position.set(1.5, 4, 5); szene.add(schluessel);
    const kante = new T.DirectionalLight(0x7fd3ff, 1.6); kante.position.set(-3, 3, -3); szene.add(kante);
    const spot = new T.SpotLight(0xffffff, 26, 9, 0.42, 0.6, 1.6); spot.position.set(0, 4.6, R + 2.2); spot.target.position.set(0, 1, R); szene.add(spot, spot.target);

    // Plattform: dunkle Scheibe, Ringe in Gold und Cyan, für jedes Ratsmitglied ein Sockelring und eine Lichtsäule.
    const scheibe = new T.Mesh(new T.CircleGeometry(R + 0.85, 96), new T.MeshBasicMaterial({ color: 0x08162a, transparent: true, opacity: 0.9, depthWrite: false }));
    scheibe.rotation.x = -Math.PI / 2; fx.add(scheibe);
    const rand = new T.Mesh(new T.RingGeometry(R + 0.8, R + 0.86, 160), new T.MeshBasicMaterial({ color: 0xdab57b, transparent: true, opacity: 0.7, depthWrite: false, side: T.DoubleSide }));
    rand.rotation.x = -Math.PI / 2; rand.position.y = 0.003; fx.add(rand);
    const innen = new T.Mesh(new T.RingGeometry(0.42, 0.45, 96), new T.MeshBasicMaterial({ color: 0x7fd3ff, transparent: true, opacity: 0.35, depthWrite: false, side: T.DoubleSide }));
    innen.rotation.x = -Math.PI / 2; innen.position.y = 0.003; fx.add(innen);
    const kreisFx = new T.Group(), kreis = new T.Group(); fx.add(kreisFx); szene.add(kreis);
    // Hologramm-Scan beim Wechsel: ein leuchtender Ring fährt an der vorderen Figur hoch (der einzige Effekt über einer Figur).
    const scan = new T.Mesh(new T.TorusGeometry(0.42, 0.012, 8, 96), new T.MeshBasicMaterial({ color: 0x9fe6ff, transparent: true, opacity: 0, blending: T.AdditiveBlending, depthWrite: false }));
    scan.rotation.x = Math.PI / 2; scan.position.set(0, 0, R); szene.add(scan);
    let scanStart = -1e9;
    const plaetze = ROLLEN.map((r, i) => {
      const a = i * SCHRITT, farbe = new T.Color(ROLLENFARBE[r]);
      const platz = new T.Group(); platz.position.set(Math.sin(a) * R, 0, Math.cos(a) * R); platz.rotation.y = a; kreis.add(platz);
      const sockel = new T.Group(); sockel.position.copy(platz.position); kreisFx.add(sockel);
      const ring = new T.Mesh(new T.RingGeometry(0.4, 0.45, 96), new T.MeshBasicMaterial({ color: farbe, transparent: true, opacity: 0.5, depthWrite: false, side: T.DoubleSide }));
      ring.rotation.x = -Math.PI / 2; ring.position.y = 0.004; sockel.add(ring);
      const schatten = new T.Mesh(new T.PlaneGeometry(1.1, 1.1), new T.MeshBasicMaterial({ map: Dreid.schattenTextur(), transparent: true, depthWrite: false }));
      schatten.rotation.x = -Math.PI / 2; schatten.position.y = 0.005; sockel.add(schatten);
      const saeule = lichtsaeule(farbe); sockel.add(saeule);
      return { rolle: r, platz, ring, saeule, modell: null, pfad: null, holo: null };
    });
    // Funken über der Plattform.
    const n = 140, pos = new Float32Array(n * 3);
    for (let i = 0; i < n; i++) { const w = Math.random() * Math.PI * 2, rr = Math.random() * (R + 0.7); pos.set([Math.cos(w) * rr, Math.random() * 2.6, Math.sin(w) * rr], i * 3); }
    const funkenGeo = new T.BufferGeometry(); funkenGeo.setAttribute("position", new T.BufferAttribute(pos, 3));
    fx.add(new T.Points(funkenGeo, new T.PointsMaterial({ color: 0x9fdcff, size: 0.02, transparent: true, opacity: 0.65, depthWrite: false })));

    const start = Math.max(0, ROLLEN.indexOf(S.ui.rund?.vorne || ROLLEN[0]));
    const z = { el, renderer, szene, fx, kamera, plaetze, lebt: true, letzte: performance.now(), winkel: -start * SCHRITT, ziel: -start * SCHRITT,
      v: 0, ziehen: null, ruhe: 0, vorne: start, hoehe: 0.0, auto: true };

    function einsetzen(p) {
      const glb = glbPfad(p.rolle), pfad = glb || bildPfad(p.rolle);
      if (p.pfad === pfad) return;
      p.pfad = pfad;
      const weg = p.modell; p.modell = null;
      if (weg) p.platz.remove(weg);
      if (glb) {
        Dreid.holen(glb).then((g) => { if (!z.lebt || p.pfad !== pfad) return; const m = g.clone(); p.platz.add(m); p.modell = m; p.holo = Dreid.materialisieren(m, ROLLENFARBE[p.rolle]); });
      } else {
        // Noch kein 3D-Modell: die Figur als Bild, das sich zur Kamera dreht.
        new T.TextureLoader().load(pfad, (tex) => {
          if (!z.lebt || p.pfad !== pfad) return;
          if ("colorSpace" in tex) tex.colorSpace = T.SRGBColorSpace; else tex.encoding = T.sRGBEncoding;
          const s = new T.Sprite(new T.SpriteMaterial({ map: tex, transparent: true }));
          const hoch = pfad.endsWith(".webp") ? 2 : 1.1, breit = hoch * tex.image.width / tex.image.height;
          s.scale.set(breit, hoch, 1); s.position.y = pfad.endsWith(".webp") ? 1 : 1.55; s.center.set(0.5, 0.5);
          p.platz.add(s); p.modell = s; p.holo = null;
        });
      }
    }
    plaetze.forEach(einsetzen);
    z.einsetzen = () => plaetze.forEach(einsetzen);

    // Bedienung: ziehen dreht den Kreis, loslassen rastet ein, Klick auf eine Figur holt sie nach vorn.
    const strahl = new T.Raycaster(), maus = new T.Vector2();
    leinwand.addEventListener("pointerdown", (e) => { z.auto = false; leinwand.setPointerCapture(e.pointerId); z.ziehen = { x: e.clientX, y: e.clientY, sx: e.clientX, t: performance.now() }; z.v = 0; z.ruhe = 0; el.classList.add("zieht", "benutzt"); });
    leinwand.addEventListener("pointermove", (e) => {
      if (!z.ziehen) return;
      const jetzt = performance.now(), dt = Math.max(1, jetzt - z.ziehen.t) / 1000, dx = e.clientX - z.ziehen.x;
      z.winkel += dx * 0.008; z.ziel = z.winkel; z.v = z.v * 0.5 + (dx * 0.008 / dt) * 0.5;
      z.hoehe = Math.max(-0.15, Math.min(0.5, z.hoehe + (e.clientY - z.ziehen.y) * 0.003));
      z.ziehen.x = e.clientX; z.ziehen.y = e.clientY; z.ziehen.t = jetzt;
    });
    leinwand.addEventListener("pointerup", (e) => {
      const g = z.ziehen; z.ziehen = null; el.classList.remove("zieht");
      if (g && Math.abs(e.clientX - g.sx) < 5) {
        const r = leinwand.getBoundingClientRect(); maus.set(((e.clientX - r.left) / r.width) * 2 - 1, -((e.clientY - r.top) / r.height) * 2 + 1);
        strahl.setFromCamera(maus, kamera);
        const treffer = strahl.intersectObjects(plaetze.map((p) => p.platz), true)[0];
        if (treffer) {
          const p = plaetze.find((q) => { let o = treffer.object; while (o) { if (o === q.platz) return true; o = o.parent; } return false; });
          // Klick auf die vordere Figur: sie stellt sich vor (Stimmen), sonst kommt die angeklickte nach vorn.
          if (p && ROLLEN.indexOf(p.rolle) === z.vorne && window.Stimmen && Stimmen.vorstellen) Stimmen.vorstellen(p.rolle);
          else if (p) waehlen(ROLLEN.indexOf(p.rolle));
        }
        return;
      }
      // Schwung mitnehmen, dann auf das nächste Ratsmitglied einrasten.
      z.ziel = Math.round((z.winkel + z.v * 0.35) / SCHRITT) * SCHRITT;
    });
    function waehlen(i) {
      const ziel = -i * SCHRITT, k = Math.round((z.winkel - ziel) / (Math.PI * 2));
      z.ziel = ziel + k * Math.PI * 2; z.ruhe = 0;
    }
    z.waehlen = waehlen;

    const titel = el.querySelector(".rund-titel");
    const schleife = (t) => {
      if (!z.lebt) return;
      const dt = Math.min(0.05, (t - z.letzte) / 1000); z.letzte = t;
      if (!z.ziehen) {
        z.ruhe += dt;
        // Solange niemand eingreift, stellt der Kreis alle 5 s das nächste Ratsmitglied vor; nach dem
        // ersten Eingriff bleibt die Wahl stehen, damit die Volkswahl nie die falsche Figur trifft.
        if (z.auto && z.ruhe > 5) { z.ruhe = 0; z.ziel -= SCHRITT; }
        z.winkel += (z.ziel - z.winkel) * (1 - Math.pow(0.004, dt));
      }
      kreis.rotation.y = z.winkel; kreisFx.rotation.y = z.winkel;
      const i = ((Math.round(-z.winkel / SCHRITT) % 4) + 4) % 4;
      if (i !== z.vorne) {
        z.vorne = i; scanStart = t; S.ui.rund = Object.assign(S.ui.rund || {}, { vorne: ROLLEN[i] });
        if (titel) titel.textContent = ""; planen();
      }
      if (titel && !titel.textContent) { const r = ROLLEN[i]; titel.textContent = `${ROLLENTITEL[r]} · ${name(gestalt(r))}`; titel.style.setProperty("--rolle", ROLLENFARBE[r]); }
      for (const p of plaetze) {
        const vorn = p.rolle === ROLLEN[i];
        p.ring.material.opacity += ((vorn ? 0.95 : 0.35) - p.ring.material.opacity) * Math.min(1, dt * 5);
        p.saeule.material.opacity += ((vorn ? 0.55 : 0.1) - p.saeule.material.opacity) * Math.min(1, dt * 5);
        if (p.holo && p.holo(t)) p.holo = null;
        // Ruhiges Leben: Atmen und Wiegen, versetzt je Figur; jede Figur wendet sich halb der Kamera zu.
        if (p.modell && !p.modell.isSprite) {
          const o = ROLLEN.indexOf(p.rolle), blick = -(o * SCHRITT + z.winkel), w = Math.atan2(Math.sin(blick), Math.cos(blick)) * 0.55;
          p.modell.position.y = 0.005 * Math.sin(t / 1100 + o);
          p.modell.rotation.y += ((w + 0.04 * Math.sin(t / 2600 + o * 1.3)) - p.modell.rotation.y) * Math.min(1, dt * 3);
        }
      }
      rand.rotation.z += dt * 0.05; innen.rotation.z -= dt * 0.2;
      const s = (t - scanStart) / 900;
      scan.visible = s >= 0 && s <= 1; if (scan.visible) { scan.position.y = s * 2.05; scan.material.opacity = Math.sin(s * Math.PI) * 0.9; scan.scale.setScalar(1 - s * 0.35); }
      const fp = funkenGeo.attributes.position;
      for (let k = 0; k < n; k++) { let y = fp.getY(k) + dt * (0.07 + (k % 5) * 0.015); if (y > 2.7) y = 0; fp.setY(k, y); }
      fp.needsUpdate = true;
      const b = el.clientWidth, hh = el.clientHeight;
      if (b && hh && (leinwand.width !== Math.floor(b * renderer.getPixelRatio()) || leinwand.height !== Math.floor(hh * renderer.getPixelRatio()))) { renderer.setSize(b, hh, false); kamera.aspect = b / hh; kamera.updateProjectionMatrix(); }
      const abstand = 5.4 / Math.min(1.35, Math.max(0.75, kamera.aspect / 1.6));
      // Sanfte Kamerabewegung: leichtes Schweben und Atmen der Entfernung.
      kamera.position.set(Math.sin(t / 5200) * 0.35, 1.55 + z.hoehe * 3 + Math.sin(t / 3700) * 0.06, R + abstand + Math.sin(t / 4300) * 0.18);
      kamera.lookAt(0, 0.95, 0); kamera.updateMatrixWorld();
      renderer.clear(); renderer.render(fx, kamera); renderer.clearDepth(); renderer.render(szene, kamera);
      requestAnimationFrame(schleife);
    };
    requestAnimationFrame(schleife);
    return z;
  }

  function pflegen() {
    if (!T) return;
    const el = document.querySelector(".rundbuehne");
    if (inst && (!el || inst.el !== el)) { inst.lebt = false; inst.renderer.dispose(); inst = null; }
    if (el && !inst) inst = aufbauen(el);
    if (inst) { inst.einsetzen(); const t = inst.el.querySelector(".rund-titel"); if (t) t.textContent = ""; }
  }
  /** Ratsmitglied nach vorn holen (z. B. aus den Knöpfen unter der Bühne). */
  function zeigen(rolle) { halten(); if (inst) inst.waehlen(ROLLEN.indexOf(rolle)); else { S.ui.rund = Object.assign(S.ui.rund || {}, { vorne: rolle }); planen(); } }
  /** Selbstdrehen beenden (erster Eingriff des Spielers). */
  function halten() { if (inst) inst.auto = false; }
  return { element, pflegen, vorne, zeigen, halten };
})();
window.Rund = Rund;
