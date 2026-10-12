/* 3D-Bühne für die Ratsfiguren (GLB) mit eigenem, schlankem GLB-Lader:
   ein Netz je Primitive, PBR-Texturen aus dem Binärteil, keine fremden Bibliotheken außer three.js. */
"use strict";

const Dreid = (() => {
  const cache = new Map();   // Pfad -> Promise<THREE.Group>
  const buehnen = new Map(); // Element -> Instanz
  const T = window.THREE;
  const TYP = { 5120: Int8Array, 5121: Uint8Array, 5122: Int16Array, 5123: Uint16Array, 5125: Uint32Array, 5126: Float32Array };
  const KOMP = { SCALAR: 1, VEC2: 2, VEC3: 3, VEC4: 4, MAT4: 16 };

  async function laden(pfad) {
    const daten = await (await fetch(pfad)).arrayBuffer();
    const dv = new DataView(daten);
    if (dv.getUint32(0, true) !== 0x46546c67) throw new Error("kein GLB");
    let pos = 12, json = null, bin = null;
    while (pos < daten.byteLength) {
      const laenge = dv.getUint32(pos, true), art = dv.getUint32(pos + 4, true);
      const teil = daten.slice(pos + 8, pos + 8 + laenge);
      if (art === 0x4e4f534a) json = JSON.parse(new TextDecoder().decode(teil));
      else if (art === 0x004e4942) bin = teil;
      pos += 8 + laenge;
    }
    const zugriff = (i) => {
      const a = json.accessors[i], bv = json.bufferViews[a.bufferView];
      const Art = TYP[a.componentType], k = KOMP[a.type], start = (bv.byteOffset || 0) + (a.byteOffset || 0);
      const schritt = bv.byteStride || 0, groesse = Art.BYTES_PER_ELEMENT * k;
      if (!schritt || schritt === groesse) return { feld: new Art(bin, start, a.count * k), k, norm: !!a.normalized };
      const aus = new Art(a.count * k), quelle = new DataView(bin);
      for (let j = 0; j < a.count; j++) for (let c = 0; c < k; c++) {
        const o = start + j * schritt + c * Art.BYTES_PER_ELEMENT;
        aus[j * k + c] = Art === Float32Array ? quelle.getFloat32(o, true) : Art === Uint16Array ? quelle.getUint16(o, true) : Art === Uint32Array ? quelle.getUint32(o, true) : quelle.getUint8(o);
      }
      return { feld: aus, k, norm: !!a.normalized };
    };
    const texturen = new Map();
    const textur = async (info, farbe) => {
      if (!info) return null;
      const schluessel = info.index + (farbe ? "f" : "");
      if (texturen.has(schluessel)) return texturen.get(schluessel);
      const t = json.textures[info.index], img = json.images[t.source ?? t.extensions?.EXT_texture_webp?.source ?? 0], bv = json.bufferViews[img.bufferView];
      const blob = new Blob([new Uint8Array(bin, bv.byteOffset || 0, bv.byteLength)], { type: img.mimeType || "image/png" });
      const bitmap = await createImageBitmap(blob, { imageOrientation: "none" });
      const tex = new T.Texture(bitmap);
      tex.flipY = false;
      if (farbe) { if ("colorSpace" in tex) tex.colorSpace = T.SRGBColorSpace; else tex.encoding = T.sRGBEncoding; }
      tex.anisotropy = 4;
      tex.needsUpdate = true;
      texturen.set(schluessel, tex);
      return tex;
    };
    const gruppe = new T.Group();
    const material = async (i) => {
      const m = (json.materials || [])[i] || {}, p = m.pbrMetallicRoughness || {};
      const mat = new T.MeshStandardMaterial({ color: 0xffffff, metalness: p.metallicFactor ?? 1, roughness: p.roughnessFactor ?? 1 });
      if (p.baseColorFactor) mat.color.setRGB(p.baseColorFactor[0], p.baseColorFactor[1], p.baseColorFactor[2]);
      mat.map = await textur(p.baseColorTexture, true);
      const mr = await textur(p.metallicRoughnessTexture, false);
      if (mr) { mat.metalnessMap = mr; mat.roughnessMap = mr; }
      mat.normalMap = await textur(m.normalTexture, false);
      mat.aoMap = await textur(m.occlusionTexture, false);
      mat.side = m.doubleSided ? T.DoubleSide : T.FrontSide;
      mat.envMapIntensity = 0.85;
      return mat;
    };
    const knoten = async (ni, eltern) => {
      const n = json.nodes[ni], g = new T.Group();
      if (n.matrix) { g.applyMatrix4(new T.Matrix4().fromArray(n.matrix)); }
      else {
        if (n.translation) g.position.fromArray(n.translation);
        if (n.rotation) g.quaternion.fromArray(n.rotation);
        if (n.scale) g.scale.fromArray(n.scale);
      }
      if (n.mesh !== undefined) for (const prim of json.meshes[n.mesh].primitives) {
        const geo = new T.BufferGeometry();
        for (const [attr, ziel] of [["POSITION", "position"], ["NORMAL", "normal"], ["TEXCOORD_0", "uv"], ["COLOR_0", "color"]]) {
          if (prim.attributes[attr] === undefined) continue;
          const z = zugriff(prim.attributes[attr]);
          geo.setAttribute(ziel, new T.BufferAttribute(z.feld, z.k, z.norm));
        }
        if (prim.attributes.TEXCOORD_0 !== undefined) geo.setAttribute("uv2", geo.getAttribute("uv"));
        if (prim.indices !== undefined) geo.setIndex(new T.BufferAttribute(zugriff(prim.indices).feld, 1));
        if (!geo.getAttribute("normal")) geo.computeVertexNormals();
        const mat = await material(prim.material);
        if (geo.getAttribute("color")) mat.vertexColors = true;
        g.add(new T.Mesh(geo, mat));
      }
      for (const k of n.children || []) await knoten(k, g);
      eltern.add(g);
    };
    const szene = json.scenes ? json.scenes[json.scene || 0] : { nodes: json.nodes.map((_, i) => i) };
    for (const ni of szene.nodes) await knoten(ni, gruppe);
    // Aufrecht stellen, auf den Boden setzen, auf 2 Einheiten Höhe bringen.
    let box = new T.Box3().setFromObject(gruppe), groesse = box.getSize(new T.Vector3());
    if (groesse.z > groesse.y * 1.3) { gruppe.rotation.x = -Math.PI / 2; gruppe.updateMatrixWorld(true); box = new T.Box3().setFromObject(gruppe); groesse = box.getSize(new T.Vector3()); }
    const huelle = new T.Group(); huelle.add(gruppe);
    const s = 2 / Math.max(groesse.y, 1e-6), mitte = box.getCenter(new T.Vector3());
    gruppe.position.sub(new T.Vector3(mitte.x, box.min.y, mitte.z));
    huelle.scale.setScalar(s);
    return huelle;
  }

  /** Erscheinen als flackerndes Hologramm (der einzige Effekt, der auf der Figur liegen darf).
      Eigene Materialkopien, damit andere Bühnen mit demselben Modell unberührt bleiben.
      Liefert eine Funktion(t) → true, sobald die Figur fest steht. */
  function materialisieren(modell, farbe) {
    const mats = [];
    modell.traverse((o) => { if (o.isMesh) { o.material = o.material.clone(); mats.push(o.material); } });
    const f = new T.Color(farbe || "#7fd3ff");
    for (const m of mats) { m.transparent = true; m.opacity = 0; if (m.emissive) { m.emissive = f.clone(); m.emissiveIntensity = 1.6; } }
    let start = null;
    return (t) => {
      if (start === null) start = t;
      const p = (t - start) / 200;
      for (const m of mats) { m.opacity = Math.min(1, p); if (m.emissive) m.emissiveIntensity = Math.max(0, 1.6 * (1 - p)); }
      if (p < 1) return false;
      for (const m of mats) { m.opacity = 1; m.transparent = false; if (m.emissive) { m.emissive.setRGB(0, 0, 0); m.emissiveIntensity = 0; } m.needsUpdate = true; }
      return true;
    };
  }
  function holen(pfad) { if (!cache.has(pfad)) cache.set(pfad, laden(pfad)); return cache.get(pfad); }

  /** Weicher Kontaktschatten unter den Füßen (radialer Verlauf, ohne Schattenberechnung). */
  /** Weiche Studio-Umgebung für Spiegelungen: PBR-Metall (Rüstungen, Helme, Schmuck) zeigt nur seine Umgebung –
      ohne sie bleibt es fast schwarz. Ein dunkler Raum mit warmer Decke und kühlen Seitenflächen, einmal je
      Renderer als PMREM-Textur erzeugt. */
  const studios = new WeakMap();
  function studio(renderer) {
    if (studios.has(renderer)) return studios.get(renderer);
    const pm = new T.PMREMGenerator(renderer), raum = new T.Scene();
    raum.add(new T.Mesh(new T.BoxGeometry(12, 12, 12), new T.MeshBasicMaterial({ color: 0x1b2230, side: T.BackSide })));
    const flaeche = (farbe, staerke, pos, groesse) => {
      const m = new T.Mesh(new T.BoxGeometry(...groesse), new T.MeshBasicMaterial({ color: new T.Color(farbe).multiplyScalar(staerke) }));
      m.position.set(...pos); raum.add(m);
    };
    flaeche(0xfff0d8, 5, [0, 5.6, 0.5], [7, 0.1, 5]);      // warme Decke
    flaeche(0xffffff, 2.2, [-5.6, 2, 1.5], [0.1, 4, 5]);  // weiche Seite links
    flaeche(0xcfe6ff, 2.4, [5.6, 1.6, -1], [0.1, 4, 5]);  // kühle Seite rechts
    flaeche(0xfff6ea, 1.4, [0, 1.8, 5.6], [6, 3.5, 0.1]); // von vorn
    const tex = pm.fromScene(raum, 0.04).texture;
    pm.dispose(); studios.set(renderer, tex);
    return tex;
  }

  function schattenTextur() {
    const c = document.createElement("canvas"); c.width = c.height = 128;
    const g = c.getContext("2d"), v = g.createRadialGradient(64, 64, 0, 64, 64, 64);
    v.addColorStop(0, "rgba(0,0,0,0.75)"); v.addColorStop(0.55, "rgba(0,0,0,0.35)"); v.addColorStop(1, "rgba(0,0,0,0)");
    g.fillStyle = v; g.fillRect(0, 0, 128, 128);
    return new T.CanvasTexture(c);
  }

  /** Drehteller mit Schwung: ziehen dreht (mit Nachlauf), Mausrad und zwei Finger zoomen zum Kopf hin,
      Doppelklick setzt zurück, Ansichtsknöpfe fahren weich an, nach einer Pause dreht er wieder allein. */
  function buehne(el) {
    const farbe = new T.Color(el.dataset.farbe || "#7fd3ff");
    const leinwand = document.createElement("canvas");
    el.appendChild(leinwand);
    const renderer = new T.WebGLRenderer({ canvas: leinwand, antialias: true, alpha: true });
    renderer.setPixelRatio(Math.min(2, window.devicePixelRatio || 1));
    if ("outputColorSpace" in renderer) renderer.outputColorSpace = T.SRGBColorSpace; else renderer.outputEncoding = T.sRGBEncoding;
    renderer.toneMapping = T.ACESFilmicToneMapping; renderer.toneMappingExposure = 1.4;
    renderer.autoClear = false;
    // Zwei Durchgänge: erst Effekte (Boden, Ringe, Funken), dann die Figur – kein Effekt verdeckt je die Figur.
    const szene = new T.Scene(), fx = new T.Scene();
    szene.environment = studio(renderer);
    const kamera = new T.PerspectiveCamera(28, 1, 0.05, 100);
    szene.add(new T.HemisphereLight(0xcfe6ff, 0x20160c, 1.05));
    const schluessel = new T.DirectionalLight(0xffe6c4, 2.6); schluessel.position.set(2.2, 4, 3); szene.add(schluessel);
    const kante = new T.DirectionalLight(farbe, 2.2); kante.position.set(-3, 2.8, -2.6); szene.add(kante);
    const kante2 = new T.DirectionalLight(0x7fd3ff, 0.9); kante2.position.set(3, 1.5, -3); szene.add(kante2);
    const aufhell = new T.DirectionalLight(0xffffff, 0.6); kamera.add(aufhell); aufhell.position.set(0, 0, 1); szene.add(kamera);
    const schatten = new T.Mesh(new T.PlaneGeometry(1.5, 1.5), new T.MeshBasicMaterial({ map: schattenTextur(), transparent: true, depthWrite: false }));
    schatten.rotation.x = -Math.PI / 2; schatten.position.y = 0.002; fx.add(schatten);
    const ring = new T.Mesh(new T.RingGeometry(0.62, 0.66, 128), new T.MeshBasicMaterial({ color: farbe, transparent: true, opacity: 0.6, side: T.DoubleSide, depthWrite: false }));
    ring.rotation.x = -Math.PI / 2; fx.add(ring);
    const ring2 = new T.Mesh(new T.RingGeometry(0.74, 0.75, 128), new T.MeshBasicMaterial({ color: 0x7fd3ff, transparent: true, opacity: 0.25, side: T.DoubleSide, depthWrite: false }));
    ring2.rotation.x = -Math.PI / 2; fx.add(ring2);
    // Schwebende Funken in der Rollenfarbe.
    const n = 90, punkte = new Float32Array(n * 3);
    for (let i = 0; i < n; i++) { const w = Math.random() * Math.PI * 2, r = 0.35 + Math.random() * 0.5; punkte.set([Math.cos(w) * r, Math.random() * 2.2, Math.sin(w) * r], i * 3); }
    const funkenGeo = new T.BufferGeometry(); funkenGeo.setAttribute("position", new T.BufferAttribute(punkte, 3));
    const funken = new T.Points(funkenGeo, new T.PointsMaterial({ color: farbe, size: 0.018, transparent: true, opacity: 0.7, depthWrite: false }));
    fx.add(funken);

    // Porträt (Funkfenster der Stimmen): fest auf den Kopf, ohne Leiste und ohne Selbstdrehen.
    const portraet = el.dataset.portraet === "1";
    const START = portraet ? { winkel: 0.22, hoehe: 0.1, abstand: 1.5 } : { winkel: 0.35, hoehe: 0.12, abstand: 4.6 };
    const inst = { el, renderer, szene, kamera, modell: null, lebt: true, letzte: performance.now(),
      winkel: START.winkel, hoehe: START.hoehe, abstand: START.abstand, zielAbstand: START.abstand,
      vw: 0, vh: 0, ziehen: null, finger: new Map(), ruhe: 0, auto: !portraet, ziel: null };
    if (portraet) { ring.visible = ring2.visible = funken.visible = schatten.visible = false; }
    const benutzt = () => { inst.ruhe = 0; el.classList.add("benutzt"); };
    leinwand.addEventListener("pointerdown", (e) => {
      inst.finger.set(e.pointerId, { x: e.clientX, y: e.clientY });
      leinwand.setPointerCapture(e.pointerId); benutzt(); inst.ziel = null;
      inst.ziehen = { x: e.clientX, y: e.clientY, t: performance.now() }; inst.vw = 0; inst.vh = 0; el.classList.add("zieht");
      inst.klick = { x: e.clientX, y: e.clientY, t: performance.now() };
    });
    leinwand.addEventListener("pointermove", (e) => {
      if (!inst.finger.has(e.pointerId)) return;
      const alt = inst.finger.get(e.pointerId);
      inst.finger.set(e.pointerId, { x: e.clientX, y: e.clientY });
      if (inst.finger.size === 2) {
        // Zwei Finger: Abstand der Finger zoomt.
        const [a, b] = [...inst.finger.values()], d = Math.hypot(a.x - b.x, a.y - b.y);
        if (inst.pinch) inst.zielAbstand = Math.max(2.2, Math.min(9, inst.zielAbstand * inst.pinch / Math.max(d, 1)));
        inst.pinch = d; return;
      }
      const jetzt = performance.now(), dt = Math.max(1, jetzt - inst.ziehen.t) / 1000;
      const dx = e.clientX - alt.x, dy = e.clientY - alt.y;
      inst.winkel -= dx * 0.009; inst.hoehe = Math.max(-0.35, Math.min(1.0, inst.hoehe + dy * 0.005));
      // Geschwindigkeit für den Nachlauf (geglättet).
      inst.vw = inst.vw * 0.6 + (-dx * 0.009 / dt) * 0.4; inst.vh = inst.vh * 0.6 + (dy * 0.005 / dt) * 0.4;
      inst.ziehen.t = jetzt;
    });
    const loslassen = (e) => {
      inst.finger.delete(e.pointerId); if (inst.finger.size < 2) inst.pinch = null;
      if (!inst.finger.size) { inst.ziehen = null; el.classList.remove("zieht"); }
      const k = inst.klick; inst.klick = null;
      if (e.type === "pointerup" && k && Math.hypot(e.clientX - k.x, e.clientY - k.y) < 5 && performance.now() - k.t < 350 && el.dataset.rolle && window.Stimmen && Stimmen.vorstellen) Stimmen.vorstellen(el.dataset.rolle);
    };
    leinwand.addEventListener("pointerup", loslassen); leinwand.addEventListener("pointercancel", loslassen);
    leinwand.addEventListener("wheel", (e) => { e.preventDefault(); benutzt(); inst.zielAbstand = Math.max(2.2, Math.min(9, inst.zielAbstand * Math.exp(e.deltaY * 0.0012))); }, { passive: false });
    leinwand.addEventListener("dblclick", () => { benutzt(); inst.ziel = { ...START, winkel: Math.round((inst.winkel - START.winkel) / (Math.PI * 2)) * Math.PI * 2 + START.winkel }; });

    // Ansichtsknöpfe: vorn, Seite, hinten, Kopf; Drehen an/aus; groß.
    const leiste = document.createElement("div"); leiste.className = "ansichten";
    const knopfDreid = (text, titel, tun) => { const b = document.createElement("button"); b.textContent = text; b.title = titel; b.onclick = (e) => { e.stopPropagation(); benutzt(); tun(b); }; leiste.appendChild(b); return b; };
    const fahre = (winkel, hoehe, abstand) => { const basis = Math.round((inst.winkel - winkel) / (Math.PI * 2)) * Math.PI * 2; inst.ziel = { winkel: basis + winkel, hoehe, abstand }; };
    knopfDreid("Vorn", "Von vorn", () => fahre(0, 0.08, 4.3));
    knopfDreid("Seite", "Von der Seite", () => fahre(Math.PI / 2, 0.08, 4.3));
    knopfDreid("Hinten", "Von hinten", () => fahre(Math.PI, 0.1, 4.3));
    knopfDreid("Kopf", "Nah an den Kopf", () => fahre(0.25, 0.32, 2.4));
    const drehKnopf = knopfDreid("⟳", "Selbst drehen an/aus", (b) => { inst.auto = !inst.auto; b.classList.toggle("aus", !inst.auto); });
    if (!el.classList.contains("gross")) knopfDreid("⤢", "Groß ansehen", () => { S.ui.dreidGross = { glb: el.dataset.glb, farbe: el.dataset.farbe || "", rolle: el.dataset.rolle || "" }; planen(); });
    if (!portraet) el.appendChild(leiste);
    void drehKnopf;

    const pfad = el.dataset.glb;
    if (!cache.has(pfad)) cache.set(pfad, laden(pfad));
    cache.get(pfad).then((g) => { if (!inst.lebt) return; inst.modell = g.clone(); szene.add(inst.modell); inst.holo = portraet ? null : materialisieren(inst.modell, farbe); /* Funkfenster: Gesicht sofort, Zurufe sind kurz */ el.classList.add("geladen"); const e = el.querySelector(".ersatz"); if (e) e.remove(); })
      .catch((e) => { const h = el.querySelector(".hinweis"); if (h) h.textContent = "3D nicht ladbar: " + e.message; });
    const schleife = (t) => {
      if (!inst.lebt) return;
      const dt = Math.min(0.05, (t - inst.letzte) / 1000); inst.letzte = t;
      if (inst.ziel) {
        // Weiche Fahrt zur gewählten Ansicht.
        const k = 1 - Math.pow(0.0015, dt);
        inst.winkel += (inst.ziel.winkel - inst.winkel) * k; inst.hoehe += (inst.ziel.hoehe - inst.hoehe) * k; inst.zielAbstand = inst.ziel.abstand;
        if (Math.abs(inst.ziel.winkel - inst.winkel) < 0.002 && Math.abs(inst.ziel.hoehe - inst.hoehe) < 0.002) inst.ziel = null;
        inst.ruhe = 0;
      } else if (!inst.ziehen) {
        // Nachlauf nach dem Loslassen, dann nach 3 s Ruhe wieder langsam selbst drehen.
        inst.winkel += inst.vw * dt; inst.hoehe = Math.max(-0.35, Math.min(1.0, inst.hoehe + inst.vh * dt));
        const d = Math.pow(0.04, dt); inst.vw *= d; inst.vh *= d;
        inst.ruhe += dt;
        if (inst.auto && inst.ruhe > 3) inst.winkel += dt * 0.28 * Math.min(1, (inst.ruhe - 3) / 1.5);
      }
      inst.abstand += (inst.zielAbstand - inst.abstand) * (1 - Math.pow(0.002, dt));
      const b = el.clientWidth, hh = el.clientHeight;
      if (b && hh && (leinwand.width !== Math.floor(b * renderer.getPixelRatio()) || leinwand.height !== Math.floor(hh * renderer.getPixelRatio()))) { renderer.setSize(b, hh, false); kamera.aspect = b / hh; kamera.updateProjectionMatrix(); }
      // Je näher die Kamera, desto mehr schaut sie zum Kopf.
      // Porträt (Funkfenster): darf noch näher und höher blicken, damit der Kopf das Bild füllt.
      const nah = Math.max(0, Math.min(portraet ? 1.3 : 1, (4.6 - inst.abstand) / 2.4)), blick = 1.0 + nah * 0.62;
      kamera.position.set(Math.sin(inst.winkel) * inst.abstand, blick + inst.hoehe * inst.abstand * 0.42, Math.cos(inst.winkel) * inst.abstand);
      kamera.lookAt(0, blick, 0);
      ring.material.opacity = 0.45 + Math.sin(t / 650) * 0.2 + (inst.sprechen || 0) * 0.35; ring2.rotation.z += dt * 0.15;
      inst.pegel = (inst.pegel || 0) + ((inst.sprechen || 0) - (inst.pegel || 0)) * (1 - Math.pow(0.0005, dt));
      if (inst.modell) {
        // Die Füße bleiben verankert; die Kamera übernimmt die sanfte Drehung.
        inst.modell.position.y = 0;
        inst.modell.rotation.set(0, 0, 0);
      }
      kante.intensity = 2.2 + inst.pegel * 1.6;
      const p = funkenGeo.attributes.position;
      for (let i = 0; i < n; i++) { let y = p.getY(i) + dt * (0.08 + (i % 7) * 0.012); if (y > 2.3) y = 0; p.setY(i, y); }
      p.needsUpdate = true;
      if (inst.holo && inst.holo(t)) inst.holo = null;
      kamera.updateMatrixWorld();
      renderer.clear();
      renderer.render(fx, kamera);
      renderer.clearDepth();
      renderer.render(szene, kamera);
      requestAnimationFrame(schleife);
    };
    requestAnimationFrame(schleife);
    return inst;
  }

  function pflegen() {
    if (!T) return;
    if ((S.zustand?.lokal?.connected || S.ui.seite === "lokal" || S.ui.einrichtung === 1) && S.auswahl?.lokaler_sparmodus) {
      for (const [el, inst] of buehnen) { inst.lebt=false; inst.renderer.dispose(); el.querySelector('canvas')?.remove(); }
      buehnen.clear(); return;
    }
    for (const el of document.querySelectorAll(".buehne[data-glb]")) if (!buehnen.has(el)) buehnen.set(el, buehne(el));
    for (const [el, inst] of buehnen) if (!document.body.contains(el)) { inst.lebt = false; inst.renderer.dispose(); buehnen.delete(el); }
  }
  /** Bühne als Element: mit GLB, sonst Bild als Ersatz. */
  /** `figur`: freigestellte Ganzkörperfigur (WebP) als Zwischenstufe, solange das GLB fehlt. */
  /** `opt.farbe`: Rollenfarbe für Kantenlicht, Ring und Funken; `opt.gross`: große Ansicht ohne „Groß“-Knopf;
      `opt.rolle`: Rolle der Figur (Klick ohne Ziehen → Stimmen.vorstellen(rolle), falls geladen);
      `opt.portraet`: festes Kopf-Porträt ohne Leiste, Ring und Hinweis (Funkfenster der Stimmen). */
  function element(glb, ersatz, hoehe, hinweis, figur, opt) {
    const o = opt || {}, liste = S.katalog?.figuren || [];
    const vorhanden = glb && liste.includes(glb) && !((S.zustand?.lokal?.connected || S.ui.seite === "lokal" || S.ui.einrichtung === 1) && S.auswahl?.lokaler_sparmodus);
    const ganz = !vorhanden && figur && liste.includes(figur);
    const art = (vorhanden ? "" : ganz ? ".ganz" : ".bild") + (o.gross ? ".gross" : "");
    return h("div.buehne" + art + (o.portraet ? ".portraet" : ""), { "data-fest": "1", "data-glb": vorhanden ? glb : null, "data-farbe": o.farbe || null, "data-rolle": o.rolle || null, "data-portraet": o.portraet ? "1" : null,
      "data-k": (vorhanden ? glb + (o.gross ? ":gross" : "") : "bild:" + (ganz ? figur : ersatz)) + ":" + (hoehe || 420) + ":" + (o.portraet ? "kopf" : "ganz") + ":" + (o.farbe || ""), stil: { height: (hoehe || 420) + "px", "--rolle": o.farbe || null } },
      ganz && !o.portraet ? h("div.ring") : null, h("div.ersatz", { stil: { backgroundImage: `url(${ganz ? figur : ersatz})` } }),
      o.portraet ? null : h("div.hinweis", null, hinweis || (vorhanden ? "Ziehen dreht · Mausrad zoomt · Doppelklick zurück" : "")));
  }
  /** Große Ansicht einer Figur über allem (Esc oder Klick daneben schließt). */
  function grossDialog() {
    const g = S.ui.dreidGross;
    if (!g) return null;
    const zu = () => { S.ui.dreidGross = null; planen(); };
    return h("div.dialog-grund", { onclick: (e) => { if (e.target === e.currentTarget) zu(); } },
      h("div.dreid-gross", null, element(g.glb, "", Math.round(window.innerHeight * 0.84), null, null, { farbe: g.farbe, gross: true, rolle: g.rolle }),
        h("button.dreid-zu", { onclick: zu, title: "Schließen (Esc)" }, "✕")));
  }
  document.addEventListener("keydown", (e) => { if (e.key === "Escape" && S.ui.dreidGross) { S.ui.dreidGross = null; planen(); } });
  /** Für andere Bausteine (z. B. Stimmen): Bühne weich auf eine Ansicht fahren, Sprechstärke 0..1 melden. */
  function fahren(el, ziel) {
    const i = buehnen.get(el); if (!i) return;
    // Zielwinkel auf die nächstgelegene Umdrehung legen (nach langem Selbstdrehen nicht viele Runden zurück).
    const w = ziel.winkel === undefined ? i.winkel : ziel.winkel + Math.round((i.winkel - ziel.winkel) / (Math.PI * 2)) * Math.PI * 2;
    i.ziel = { winkel: w, hoehe: ziel.hoehe ?? i.hoehe, abstand: ziel.abstand ?? i.zielAbstand }; i.ruhe = 0;
  }
  /** Kopfbild einer Figur aus ihrem 3D-Modell (Porträt für Leisten und Listen), einmal gerendert mit einem
      gemeinsamen Hintergrund-Renderer. Liefert null, solange es entsteht (danach wird neu gezeichnet). */
  const kopfbilder = new Map();
  let kopfR = null;
  function kopfbild(glb) {
    if (!T || !glb || !(S.katalog?.figuren || []).includes(glb)) return null;
    if (kopfbilder.has(glb)) return kopfbilder.get(glb) || null;
    kopfbilder.set(glb, null);
    if (!cache.has(glb)) cache.set(glb, laden(glb));
    cache.get(glb).then((g) => {
      if (!kopfR) { kopfR = new T.WebGLRenderer({ antialias: true, alpha: true, preserveDrawingBuffer: true }); kopfR.setSize(192, 192, false);
        if ("outputColorSpace" in kopfR) kopfR.outputColorSpace = T.SRGBColorSpace; else kopfR.outputEncoding = T.sRGBEncoding;
        kopfR.toneMapping = T.ACESFilmicToneMapping; kopfR.toneMappingExposure = 1.4; }
      const szene = new T.Scene();
      szene.environment = studio(kopfR);
      szene.add(new T.HemisphereLight(0xcfe6ff, 0x20160c, 0.9));
      const l1 = new T.DirectionalLight(0xffe6c4, 2.0); l1.position.set(1.5, 3, 2.5); szene.add(l1);
      const l2 = new T.DirectionalLight(0x7fd3ff, 1.2); l2.position.set(-2.5, 2.5, -2); szene.add(l2);
      szene.add(g.clone());
      const k = new T.PerspectiveCamera(24, 1, 0.05, 50);
      k.position.set(Math.sin(0.22) * 1.4, 1.87, Math.cos(0.22) * 1.4); k.lookAt(0, 1.79, 0);
      kopfR.clear(); kopfR.render(szene, k);
      kopfbilder.set(glb, kopfR.domElement.toDataURL("image/png"));
      planen();
    }).catch(() => kopfbilder.set(glb, ""));
    return null;
  }
  /** Sprechstärke 0..1; solange gesprochen wird, dreht die Figur nicht von selbst weg. */
  function sprechen(el, staerke) { const i = buehnen.get(el); if (i) { i.sprechen = Math.max(0, Math.min(1, staerke || 0)); i.ruhe = 0; } }
  return { pflegen, element, grossDialog, fahren, sprechen, kopfbild, holen, materialisieren, schattenTextur, studio };
})();
