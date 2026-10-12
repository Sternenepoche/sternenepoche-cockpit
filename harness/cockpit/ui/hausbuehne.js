/* Die Hausbühne aus der Website im Cockpit: vier Figuren nebeneinander,
   farbige Sockel, Nahaufnahme und Stimme beim Anklicken. Der Sparmodus der
   lokalen Partie beeinflusst diese Vorstellung nicht. GLB-Lader und Stimmen
   kommen aus dem Cockpit; ohne WebGL bleiben die Figurenbilder und Hörkarten. */
"use strict";

const Hausbuehne = (() => {
  const T = window.THREE, PLAETZE = [[-2.55, 0.35], [-0.85, -0.25], [0.85, -0.25], [2.55, 0.35]];
  const ruhig = matchMedia("(prefers-reduced-motion: reduce)").matches;
  let inst = null;

  function zurueck() { S.ui.hausFokus = null; window.Stimmen?.stopp(true); planen(); }
  function vorstellen(rolle, volk) {
    const f = S.ui.hausFokus;
    if (f?.rolle === rolle && f.volk === volk) { zurueck(); return; }
    S.ui.hausFokus = { rolle, volk };
    window.Stimmen?.vorstellen(rolle, volk); planen();
  }

  function element(volk) {
    return h("div.hausbuehne", { "data-fest": "1", "data-k": "hausbuehne:" + volk, "data-volk": volk, "aria-label": `Die vier Ratsmitglieder der ${name(volk)} in 3D` },
      h("div.haus-ersatz", null, ROLLEN.map((r) => h("img", { src: `figuren/${r}.${volk}.webp`, alt: ROLLENTITEL[r] }))),
      h("span.haus-hinweis", null, "Der Rat betritt die Bühne …"),
      h("div.haus-dialog", { "aria-label": "Vorstellung des Ratsmitglieds" }),
      h("span.ki-marke.ecke", { title: "KI-generierte Figuren" }, "KI"));
  }

  function bauen(el) {
    if (!T) { el.querySelector(".haus-hinweis").textContent = "Klick auf eine Karte: Vorstellung in ihrer Stimme"; return null; }
    let renderer;
    try { renderer = new T.WebGLRenderer({ antialias: true, alpha: true }); }
    catch (_) { el.querySelector(".haus-hinweis").textContent = "Klick auf eine Karte: Vorstellung in ihrer Stimme"; return null; }
    renderer.setPixelRatio(Math.min(1.5, window.devicePixelRatio || 1));
    if ("outputColorSpace" in renderer) renderer.outputColorSpace = T.SRGBColorSpace; else renderer.outputEncoding = T.sRGBEncoding;
    renderer.toneMapping = T.ACESFilmicToneMapping; renderer.toneMappingExposure = 1.4;
    el.prepend(renderer.domElement);
    const szene = new T.Scene(), kamera = new T.PerspectiveCamera(30, 1, 0.05, 100);
    const studio = Dreid.studio?.(renderer); if (studio) szene.environment = studio;
    szene.add(new T.HemisphereLight(0xd9eaff, 0x19130c, 1.05));
    const key = new T.DirectionalLight(0xffe6c4, 2.2); key.position.set(1.5, 4, 5); szene.add(key);
    const rim = new T.DirectionalLight(0x7fd3ff, 1.35); rim.position.set(-3, 3, -3); szene.add(rim);
    const scheibe = new T.Mesh(new T.CircleGeometry(4.2, 96), new T.MeshBasicMaterial({ color: 0x081b2d, side: T.DoubleSide }));
    scheibe.rotation.x = -Math.PI / 2; szene.add(scheibe);
    const rand = new T.Mesh(new T.RingGeometry(4.14, 4.2, 128), new T.MeshBasicMaterial({ color: 0x7a929c, side: T.DoubleSide }));
    rand.rotation.x = -Math.PI / 2; rand.position.y = 0.001; szene.add(rand);
    const figuren = ROLLEN.map((rolle, i) => {
      const [x, z] = PLAETZE[i], ring = new T.Mesh(new T.RingGeometry(0.48, 0.52, 64), new T.MeshBasicMaterial({ color: new T.Color(ROLLENFARBE[rolle]), transparent: true, opacity: 0.75, side: T.DoubleSide, depthWrite: false }));
      ring.rotation.x = -Math.PI / 2; ring.position.set(x, 0.004, z); szene.add(ring);
      const ziel = new T.Mesh(new T.CylinderGeometry(0.4, 0.4, 2.1, 12), new T.MeshBasicMaterial({ visible: false }));
      ziel.position.set(x, 1.05, z); szene.add(ziel);
      return { rolle, ring, ziel, obj: null, x, z, phase: i * 1.7, drehung: Math.atan2(-x * 0.35, 6) };
    });
    const pos = new Float32Array(270);
    for (let i = 0; i < pos.length; i += 3) { pos[i] = Math.random() * 9 - 4.5; pos[i + 1] = Math.random() * 3.5; pos[i + 2] = -Math.random() * 3; }
    const funkeng = new T.BufferGeometry(); funkeng.setAttribute("position", new T.BufferAttribute(pos, 3));
    const funkenm = new T.PointsMaterial({ color: 0xb1d0dd, size: 0.014, transparent: true, opacity: 0.6, depthWrite: false });
    szene.add(new T.Points(funkeng, funkenm));
    const z = { el, renderer, szene, figuren, lebt: true, raf: 0, sichtbar: true, fertig: 0, volk: el.dataset.volk };
    const observer = new IntersectionObserver(([e]) => { z.sichtbar = e.isIntersecting; }); observer.observe(el); z.observer = observer;
    const hinweis = el.querySelector(".haus-hinweis");
    const stellen = (f, obj) => {
      if (!z.lebt) return;
      f.obj = obj; obj.position.set(f.x, 0, f.z); obj.rotation.y = f.drehung; szene.add(obj);
      z.fertig += 1; el.dataset.geladen = String(z.fertig);
      hinweis.textContent = z.fertig === 4 ? "Ton an · Klick auf eine Figur: Vorstellung in ihrer Stimme" : `Der Rat betritt die Bühne … (${z.fertig}/4)`;
      if (z.fertig === 4) el.classList.add("bereit");
    };
    const ersatz = (f) => {
      new T.TextureLoader().load(`figuren/${f.rolle}.${z.volk}.webp`, (tex) => {
        if (!z.lebt) { tex.dispose(); return; }
        if ("colorSpace" in tex) tex.colorSpace = T.SRGBColorSpace; else tex.encoding = T.sRGBEncoding;
        const obj = new T.Sprite(new T.SpriteMaterial({ map: tex, transparent: true }));
        obj.scale.set(2 * tex.image.width / tex.image.height, 2, 1); obj.position.y = 1;
        const g = new T.Group(); g.add(obj); g.userData.ersatz = true; stellen(f, g);
      }, undefined, () => { if (z.lebt) hinweis.textContent = "Figur nicht verfügbar · Die Hörkarten bleiben nutzbar"; });
    };
    for (const f of figuren) {
      const pfad = `figuren/${f.rolle}.${z.volk}.glb`;
      if ((S.katalog.figuren || []).includes(pfad)) Dreid.holen(pfad).then((g) => stellen(f, g.clone())).catch(() => ersatz(f));
      else ersatz(f);
    }
    const strahl = new T.Raycaster(), maus = new T.Vector2();
    renderer.domElement.addEventListener("click", (e) => {
      const b = el.getBoundingClientRect(); maus.set((e.clientX - b.left) / b.width * 2 - 1, 1 - (e.clientY - b.top) / b.height * 2);
      strahl.setFromCamera(maus, kamera);
      const t = strahl.intersectObjects(figuren.map((f) => f.ziel))[0];
      const f = t && figuren.find((x) => x.ziel === t.object); if (f) vorstellen(f.rolle, z.volk);
    });
    const blick = { pos: new T.Vector3(0, 1.85, 7.3), ziel: new T.Vector3(0, 1.12, 0) };
    let letzte = performance.now();
    const schleife = (t) => {
      if (!z.lebt) return;
      z.raf = requestAnimationFrame(schleife);
      const dt = Math.min(0.05, (t - letzte) / 1000); letzte = t;
      if (!z.sichtbar || document.hidden) return;
      const b = el.clientWidth, hh = el.clientHeight; if (!b || !hh) return;
      const enge = Math.max(0, Math.min(1, (b / hh - 0.7) / 1.1));
      if (renderer.domElement.width !== Math.floor(b * renderer.getPixelRatio()) || renderer.domElement.height !== Math.floor(hh * renderer.getPixelRatio())) {
        renderer.setSize(b, hh, false); kamera.aspect = b / hh; kamera.fov = b / hh < 1.3 ? 38 : 30; kamera.updateProjectionMatrix();
      }
      const s = window.Stimmen?.spricht(), spricht = s?.volk === z.volk ? figuren.find((f) => f.rolle === s.rolle) : null;
      const fokus = S.ui.hausFokus?.volk === z.volk ? figuren.find((f) => f.rolle === S.ui.hausFokus.rolle) : null;
      const nah = fokus || spricht, xNah = nah ? nah.x * (0.52 + 0.48 * enge) : 0;
      const zielPos = nah ? new T.Vector3(xNah * 0.86, 1.85, nah.z + 2.7) : new T.Vector3(0, 1.85 + (1 - enge) * 0.4, 7.3 + (1 - enge) * 3.2);
      const zielBlick = nah ? new T.Vector3(xNah + (fokus ? 0.35 * kamera.aspect : 0), 1.55, nah.z) : new T.Vector3(0, 1.12, 0);
      blick.pos.lerp(zielPos, ruhig ? 1 : 1 - Math.pow(0.004, dt)); blick.ziel.lerp(zielBlick, ruhig ? 1 : 1 - Math.pow(0.004, dt));
      kamera.position.copy(blick.pos); if (!ruhig && !nah) kamera.position.x += Math.sin(t / 6000) * 0.25;
      kamera.lookAt(blick.ziel); kamera.updateMatrixWorld();
      const pegel = Number(document.documentElement.style.getPropertyValue("--pegel")) || 0;
      for (const f of figuren) {
        const x = f.x * (0.52 + 0.48 * enge); f.ring.position.x = x; f.ziel.position.x = x;
        if (!f.obj) continue;
        f.obj.position.x = x;
        const p = f === spricht ? pegel : 0;
        f.obj.position.y = ruhig ? 0 : 0.006 * Math.sin(t / 1150 + f.phase) + p * 0.012 * Math.sin(t / 85);
        f.obj.rotation.y = f.drehung + (ruhig ? 0 : 0.04 * Math.sin(t / 2700 + f.phase));
        f.obj.rotation.x = ruhig ? 0 : p * (0.03 + 0.025 * Math.sin(t / 140));
        f.ring.material.opacity = f === nah ? 0.95 : nah ? 0.25 : 0.65;
      }
      renderer.render(szene, kamera);
    };
    z.raf = requestAnimationFrame(schleife);
    return z;
  }

  function beenden(z) {
    z.lebt = false; cancelAnimationFrame(z.raf); z.observer.disconnect();
    // Geladene Figuren bleiben im gemeinsamen GLB-Cache; nur Bühnenressourcen gehören dieser Instanz.
    for (const f of z.figuren) {
      if (f.obj?.userData.ersatz) f.obj.traverse((o) => { o.material?.map?.dispose(); o.material?.dispose(); });
    }
    z.szene.traverse((o) => { if (o.isMesh && !z.figuren.some((f) => f.obj && istKind(o, f.obj))) { o.geometry?.dispose(); o.material?.dispose(); } if (o.isPoints) { o.geometry.dispose(); o.material.dispose(); } });
    z.szene.environment?.dispose(); z.renderer.dispose(); z.renderer.domElement.remove();
  }
  function istKind(o, p) { for (let e = o; e; e = e.parent) if (e === p) return true; return false; }
  function pflegen() {
    const el = document.querySelector(".hausbuehne");
    if (inst && inst.el !== el) { beenden(inst); inst = null; }
    if (el && !inst && !el.dataset.versucht) { el.dataset.versucht = "1"; inst = bauen(el); }
    if (el) {
      const f = S.ui.hausFokus?.volk === el.dataset.volk ? S.ui.hausFokus : null;
      const d = f && window.Stimmen?.figur(f.rolle, f.volk), s = window.Stimmen?.spricht();
      el.classList.toggle("fokus", !!f); el.dataset.fokus = f?.rolle || "";
      const panel = el.querySelector(".haus-dialog");
      if (f && d) {
        panel.style.setProperty("--rolle", ROLLENFARBE[f.rolle]);
        morphKinder(panel, h("div", null,
          h("span.haus-dialog-rolle", null, `${ROLLENTITEL[f.rolle]} · ${name(f.volk)}`), h("h3", null, d.name),
          h("p.haus-dialog-text", null, d.vorstellung?.text || ""),
          h("div.zeile.eng", null,
            knopf(s?.rolle === f.rolle && s.volk === f.volk ? "Stopp" : "Anhören", () => window.Stimmen?.vorstellen(f.rolle, f.volk), "gold klein", s?.rolle === f.rolle && s.volk === f.volk ? "stopp" : "los", !window.Stimmen?.einstellungen.an),
            knopf("Zur Gruppe", zurueck, "leise klein"))));
      } else morphKinder(panel, h("div"));
    }
  }
  return { element, pflegen, vorstellen, zurueck };
})();
window.Hausbuehne = Hausbuehne;
