// THROWAWAY SPIKE (DEC-050): PixiJS v8 (WebGL) renderer. Retained mode: one
// Sprite per entity, kept between frames; the shared y-sorted list sets each
// sprite's zIndex, and sprites not in the list this frame are hidden (culled).
// The ground is one sprite of the same baked canvas the Canvas 2D path uses.
import { Application, Container, Rectangle, Sprite, Texture } from "pixi.js";
import type { Art } from "./art";
import { viewTransform, type Drawable, type FrameInput, type HubRenderer, type RendererFactory } from "./renderer";

export const createRenderer: RendererFactory = async (canvas: HTMLCanvasElement, art: Art) => {
  const app = new Application();
  await app.init({
    canvas,
    width: 1,
    height: 1,
    autoStart: false,
    sharedTicker: false,
    antialias: false,
    autoDensity: true,
    roundPixels: true,
    backgroundColor: 0x1d2a1d,
    preference: "webgl",
    powerPreference: "low-power",
  });

  const atlasTex = Texture.from(art.atlas);
  atlasTex.source.scaleMode = "nearest";
  const groundTex = Texture.from(art.ground);
  groundTex.source.scaleMode = "nearest";
  const textures = art.frames.map((fr) => new Texture({ source: atlasTex.source, frame: new Rectangle(fr.x, fr.y, fr.w, fr.h) }));

  const world = new Container();
  const decalLayer = new Container();
  const spriteLayer = new Container();
  spriteLayer.sortableChildren = true;
  const glowLayer = new Container();
  world.addChild(new Sprite(groundTex), decalLayer, spriteLayer);
  const tintSprite = new Sprite(Texture.WHITE);
  tintSprite.blendMode = "multiply";
  app.stage.addChild(world, tintSprite, glowLayer); // glows sit above the tint

  // Retained sprites: entity id -> sprite.
  const byId = new Map<number, Sprite>();
  const lastSeen = new Map<number, number>();
  const decalPool: Sprite[] = [];
  const glowPool: Sprite[] = [];
  let frameNo = 0;

  const setTex = (s: Sprite, frame: number) => {
    const t = textures[frame];
    const fr = art.frames[frame];
    if (!t || !fr) return;
    if (s.texture !== t) {
      s.texture = t;
      s.anchor.set(fr.ax / fr.w, fr.ay / fr.h);
    }
  };
  const pooled = (pool: Sprite[], layer: Container, items: Drawable[], count: number, blend: "normal" | "add") => {
    for (let i = 0; i < count; i++) {
      let s = pool[i];
      if (!s) {
        s = new Sprite();
        s.blendMode = blend;
        pool.push(s);
        layer.addChild(s);
      }
      const d = items[i] as Drawable;
      setTex(s, d.frame);
      s.position.set(d.x, d.y);
      s.visible = true;
    }
    for (let i = count; i < pool.length; i++) (pool[i] as Sprite).visible = false;
  };

  let cssW = 1;
  let cssH = 1;
  let dpr = 1;
  let glInfo = "WebGL";
  const r = app.renderer as unknown as { gl?: WebGLRenderingContext };
  if (r.gl) {
    const ext = r.gl.getExtension("WEBGL_debug_renderer_info");
    glInfo = ext ? String(r.gl.getParameter(ext.UNMASKED_RENDERER_WEBGL)) : String(r.gl.getParameter(r.gl.RENDERER));
  }

  const renderer: HubRenderer = {
    name: "pixi",
    info: `PixiJS ${app.renderer.name} (${glInfo})`,
    resize(w, h, d) {
      cssW = w;
      cssH = h;
      dpr = d;
      app.renderer.resize(w, h, d);
      tintSprite.width = w;
      tintSprite.height = h;
    },
    render(f: FrameInput) {
      frameNo++;
      const { s, tx, ty } = viewTransform(f, cssW, cssH, dpr);
      world.scale.set(s / dpr);
      world.position.set(tx / dpr, ty / dpr);
      pooled(decalPool, decalLayer, f.decals, f.decalCount, "normal");
      for (let i = 0; i < f.spriteCount; i++) {
        const d = f.sprites[i] as Drawable;
        let sp = byId.get(d.id);
        if (!sp) {
          sp = new Sprite();
          byId.set(d.id, sp);
          spriteLayer.addChild(sp);
        }
        setTex(sp, d.frame);
        sp.position.set(d.x, d.y);
        if (sp.zIndex !== i) sp.zIndex = i;
        sp.visible = true;
        lastSeen.set(d.id, frameNo);
      }
      for (const [id, sp] of byId) if (sp.visible && lastSeen.get(id) !== frameNo) sp.visible = false;
      const night = f.tint !== null;
      tintSprite.visible = night;
      if (night) tintSprite.tint = f.tint ?? 0xffffff;
      pooled(glowPool, glowLayer, f.glows, night ? f.glowCount : 0, "add");
      glowLayer.scale.copyFrom(world.scale);
      glowLayer.position.copyFrom(world.position);
      app.render();
    },
  };
  return renderer;
};
