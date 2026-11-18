// THROWAWAY SPIKE (DEC-050): procedurally drawn placeholder pixel art.
// Everything is packed into one atlas canvas (one texture for PixiJS, one
// drawImage source for Canvas 2D) plus one baked ground canvas.
import { GRASS, H, PATH, PAVE, PLAZA, ROAD, TILE, W, ZEBRA, idx, mulberry32, type Building, type PropKind, type Town } from "./scene";

export interface Frame {
  x: number;
  y: number;
  w: number;
  h: number;
  ax: number; // anchor (feet / base) inside the frame
  ay: number;
}

export interface Art {
  atlas: HTMLCanvasElement;
  ground: HTMLCanvasElement;
  frames: Frame[];
  buildingFrame: number[]; // by building id
  propFrame: Record<PropKind, number>;
  charBase: number; // + (variant * 4 + dir) * 3 + step
  variants: number; // NPC variants; the player is variant `variants`
  marker: number;
  doorArrow: number;
  glow: number;
}

type Draw = (c: CanvasRenderingContext2D) => void;

const ATLAS = 1024;

function packer() {
  const canvas = document.createElement("canvas");
  canvas.width = ATLAS;
  canvas.height = ATLAS;
  const ctx = canvas.getContext("2d");
  if (!ctx) throw new Error("no 2d context");
  const frames: Frame[] = [];
  let cx = 0;
  let cy = 0;
  let rowH = 0;
  const add = (w: number, h: number, ax: number, ay: number, draw: Draw): number => {
    if (cx + w > ATLAS) {
      cx = 0;
      cy += rowH + 2;
      rowH = 0;
    }
    if (cy + h > ATLAS) throw new Error("atlas full");
    ctx.save();
    ctx.translate(cx, cy);
    ctx.beginPath();
    ctx.rect(0, 0, w, h);
    ctx.clip();
    draw(ctx);
    ctx.restore();
    frames.push({ x: cx, y: cy, w, h, ax, ay });
    cx += w + 2;
    rowH = Math.max(rowH, h);
    return frames.length - 1;
  };
  return { canvas, frames, add };
}

const px = (c: CanvasRenderingContext2D, color: string, x: number, y: number, w = 1, h = 1) => {
  c.fillStyle = color;
  c.fillRect(x, y, w, h);
};

// ---------------------------------------------------------------- ground

function bakeGround(town: Town): HTMLCanvasElement {
  const g = document.createElement("canvas");
  g.width = W * TILE;
  g.height = H * TILE;
  const c = g.getContext("2d");
  if (!c) throw new Error("no 2d context");
  const rng = mulberry32(99);
  for (let y = 0; y < H; y++) {
    for (let x = 0; x < W; x++) {
      const t = town.ground[idx(x, y)];
      const ox = x * TILE;
      const oy = y * TILE;
      if (t === GRASS) {
        px(c, "#5c9b3e", ox, oy, TILE, TILE);
        for (let k = 0; k < 7; k++) px(c, rng() < 0.5 ? "#6db24b" : "#4d8a33", ox + ((rng() * 15) | 0), oy + ((rng() * 15) | 0), 1, 2);
        if (rng() < 0.05) {
          const fx = ox + 4 + ((rng() * 8) | 0);
          const fy = oy + 4 + ((rng() * 8) | 0);
          px(c, rng() < 0.5 ? "#f2e27a" : "#f08aa0", fx, fy, 2, 2);
        }
      } else if (t === ROAD || t === ZEBRA) {
        px(c, "#4b4c55", ox, oy, TILE, TILE);
        for (let k = 0; k < 4; k++) px(c, "#55565f", ox + ((rng() * 15) | 0), oy + ((rng() * 15) | 0));
        if (t === ZEBRA) {
          const vertical = town.ground[idx(Math.min(W - 1, x + 1), y)] === ROAD || town.ground[idx(Math.max(0, x - 1), y)] === ROAD;
          for (let k = 1; k < TILE; k += 4) {
            if (vertical) px(c, "#e9e6dc", ox + 1, oy + k, TILE - 2, 2);
            else px(c, "#e9e6dc", ox + k, oy + 1, 2, TILE - 2);
          }
        }
      } else if (t === PAVE) {
        px(c, "#bdb7aa", ox, oy, TILE, TILE);
        px(c, "#aaa496", ox, oy + 7, TILE, 1);
        px(c, "#aaa496", ox + ((x + y) % 2 ? 7 : 0), oy, 1, 7);
        px(c, "#aaa496", ox + ((x + y) % 2 ? 0 : 7), oy + 8, 1, 8);
      } else if (t === PATH) {
        px(c, "#c7a676", ox, oy, TILE, TILE);
        for (let k = 0; k < 5; k++) px(c, "#b08f60", ox + ((rng() * 14) | 0), oy + ((rng() * 14) | 0), 2, 2);
      } else if (t === PLAZA) {
        px(c, "#d9d0bf", ox, oy, TILE, TILE);
        px(c, "#c4baa6", ox, oy + 15, TILE, 1);
        px(c, "#c4baa6", ox + 15, oy, 1, TILE);
      }
    }
  }
  // Lane markings and curbs.
  for (let y = 0; y < H; y++) {
    for (let x = 0; x < W; x++) {
      const t = town.ground[idx(x, y)];
      const ox = x * TILE;
      const oy = y * TILE;
      if (t === ROAD) {
        const below = y + 1 < H ? town.ground[idx(x, y + 1)] : -1;
        const right = x + 1 < W ? town.ground[idx(x + 1, y)] : -1;
        const hRoad = (y === 15 || y === 30) && below === ROAD;
        const vRoad = (x === 15 || x === 32) && right === ROAD;
        if (hRoad && !vRoad && x % 2 === 0) px(c, "#e8d36a", ox + 2, oy + 15, 12, 2);
        if (vRoad && !hRoad && y % 2 === 0) px(c, "#e8d36a", ox + 15, oy + 2, 2, 12);
      }
      if (t === PAVE) {
        const n = (dx: number, dy: number) => {
          const i = x + dx;
          const j = y + dy;
          return i >= 0 && j >= 0 && i < W && j < H ? town.ground[idx(i, j)] : -1;
        };
        const isRoad = (v: number | undefined) => v === ROAD || v === ZEBRA;
        if (isRoad(n(0, 1))) px(c, "#8a8478", ox, oy + 14, TILE, 2);
        if (isRoad(n(0, -1))) px(c, "#8a8478", ox, oy, TILE, 2);
        if (isRoad(n(1, 0))) px(c, "#8a8478", ox + 14, oy, 2, TILE);
        if (isRoad(n(-1, 0))) px(c, "#8a8478", ox, oy, 2, TILE);
      }
    }
  }
  // Soft building shadows on the ground (east side).
  c.fillStyle = "rgba(20,30,20,0.25)";
  for (const b of town.buildings) c.fillRect((b.x + b.w) * TILE, b.y * TILE + 6, 5, b.h * TILE - 6);
  return g;
}

// ---------------------------------------------------------------- buildings

const WALLS = ["#d8b48a", "#c98f6b", "#e0d6c2", "#9fb4c7", "#d9c27a", "#b9a3c9", "#c7c7c7", "#e3a87a", "#a8c49a"];
const ROOFS = ["#a8433a", "#5d6f8f", "#6b4a3a", "#3f6f5a", "#8c5a2b", "#7a3f6b", "#4a4f5a", "#b5652f", "#365f7a"];

function drawBuilding(c: CanvasRenderingContext2D, b: Building) {
  const w = b.w * TILE;
  const h = (b.h + 1) * TILE;
  const wallH = 30;
  const roofH = h - wallH;
  const council = b.style === 9;
  const wall = council ? "#e4dccb" : (WALLS[b.style] ?? "#ccc");
  const roof = council ? "#4f5766" : (ROOFS[b.style] ?? "#844");
  // Roof
  px(c, roof, 0, 0, w, roofH);
  for (let y = 3; y < roofH - 2; y += 4) px(c, "rgba(0,0,0,0.14)", 1, y, w - 2, 1);
  px(c, "rgba(255,255,255,0.18)", 0, 0, w, 2);
  px(c, "rgba(0,0,0,0.35)", 0, roofH - 3, w, 3);
  if (council) {
    // Pediment, dome cue and a flag line.
    const cx = w / 2;
    c.fillStyle = "#cfc6b3";
    c.beginPath();
    c.moveTo(cx - 36, roofH);
    c.lineTo(cx, roofH - 16);
    c.lineTo(cx + 36, roofH);
    c.fill();
    px(c, "#b9b09c", cx - 36, roofH - 2, 72, 2);
    c.fillStyle = "#7d8aa0";
    c.beginPath();
    c.arc(cx, roofH - 26, 12, Math.PI, 0);
    c.fill();
    px(c, "#5d6a80", cx - 12, roofH - 26, 24, 3);
  } else if (b.w >= 5) {
    // Chimney
    px(c, "#7a5a4a", w - 14, 2, 6, 10);
    px(c, "#5a3a2a", w - 14, 2, 6, 2);
  }
  // Wall
  px(c, wall, 0, roofH, w, wallH);
  px(c, "rgba(0,0,0,0.18)", 0, h - 3, w, 3);
  const doorCol = b.doorX - b.x;
  for (let i = 0; i < b.w; i++) {
    const ox = i * TILE;
    if (i === doorCol) continue;
    if (council) {
      px(c, "#f2ede2", ox + 3, roofH + 2, 4, wallH - 5);
      px(c, "#cfc6b3", ox + 3, roofH + 2, 4, 2);
      px(c, "#2e3a4e", ox + 9, roofH + 8, 5, 12);
    } else {
      px(c, "#3d3a3a", ox + 3, roofH + 6, 10, 10);
      px(c, (i + b.id) % 3 ? "#8fc6e8" : "#f4d77a", ox + 4, roofH + 7, 8, 8);
      px(c, "#3d3a3a", ox + 7, roofH + 7, 1, 8);
      px(c, "rgba(0,0,0,0.25)", ox + 3, roofH + 16, 10, 1);
    }
  }
  // Door
  const dx = doorCol * TILE + 3;
  px(c, "#2b1d14", dx - 1, h - 20, 12, 20);
  px(c, council ? "#7a4b2a" : "#6b3f22", dx, h - 19, 10, 19);
  px(c, "#d8b25a", dx + 7, h - 10, 2, 2);
  if (b.name) {
    // Sign over the door.
    px(c, council ? "#2f3c58" : "#3a2b22", dx - 4, roofH + 1, 18, 5);
    px(c, "#e9d79a", dx - 2, roofH + 3, 14, 1);
  }
}

// ---------------------------------------------------------------- props

const PROPS: Record<PropKind, [number, number, number, number, Draw]> = {
  tree: [
    32,
    42,
    16,
    40,
    (c) => {
      px(c, "rgba(0,0,0,0.25)", 6, 37, 20, 5);
      px(c, "#6b4a2b", 13, 24, 6, 16);
      px(c, "#2f6a2e", 3, 6, 26, 20);
      px(c, "#2f6a2e", 7, 2, 18, 26);
      px(c, "#3f8a3a", 6, 5, 18, 14);
      px(c, "#58a84a", 9, 6, 8, 6);
      px(c, "#25562a", 4, 22, 24, 4);
    },
  ],
  pine: [
    24,
    46,
    12,
    44,
    (c) => {
      px(c, "rgba(0,0,0,0.25)", 4, 41, 16, 5);
      px(c, "#5a3a22", 10, 34, 4, 10);
      for (let k = 0; k < 4; k++) {
        const y = 2 + k * 8;
        const half = 4 + k * 2.5;
        px(c, "#245c3a", 12 - half, y + 6, half * 2, 6);
        px(c, "#2f7347", 12 - half + 2, y + 6, half * 2 - 6, 2);
      }
      px(c, "#245c3a", 10, 0, 4, 8);
    },
  ],
  lamp: [
    10,
    36,
    5,
    34,
    (c) => {
      px(c, "rgba(0,0,0,0.25)", 1, 33, 8, 3);
      px(c, "#2d3038", 4, 6, 2, 28);
      px(c, "#2d3038", 3, 31, 4, 3);
      px(c, "#2d3038", 1, 3, 8, 3);
      px(c, "#ffe9a0", 2, 6, 6, 2);
    },
  ],
  bench: [
    16,
    14,
    8,
    13,
    (c) => {
      px(c, "rgba(0,0,0,0.2)", 1, 11, 14, 3);
      px(c, "#6b4a2b", 1, 2, 14, 3);
      px(c, "#8a6238", 1, 6, 14, 3);
      px(c, "#2d3038", 2, 9, 2, 4);
      px(c, "#2d3038", 12, 9, 2, 4);
    },
  ],
  bin: [
    10,
    14,
    5,
    13,
    (c) => {
      px(c, "rgba(0,0,0,0.2)", 1, 11, 8, 3);
      px(c, "#2f5a3a", 1, 2, 8, 10);
      px(c, "#3f7a4e", 2, 3, 2, 8);
      px(c, "#1f3a26", 0, 1, 10, 2);
    },
  ],
  hydrant: [
    10,
    13,
    5,
    12,
    (c) => {
      px(c, "rgba(0,0,0,0.2)", 1, 10, 8, 3);
      px(c, "#c83a2e", 2, 2, 6, 9);
      px(c, "#e85a4a", 3, 3, 1, 6);
      px(c, "#c83a2e", 0, 5, 10, 2);
      px(c, "#8a2a20", 2, 1, 6, 2);
    },
  ],
  shelter: [
    32,
    32,
    16,
    30,
    (c) => {
      px(c, "rgba(0,0,0,0.22)", 2, 27, 30, 5);
      px(c, "#2d3038", 2, 4, 2, 26);
      px(c, "#2d3038", 28, 4, 2, 26);
      px(c, "rgba(170,210,230,0.55)", 4, 6, 24, 18);
      px(c, "#3a5f8a", 0, 0, 32, 5);
      px(c, "#f0f0f0", 6, 14, 8, 6);
      px(c, "#6b4a2b", 8, 22, 16, 3);
    },
  ],
  flag: [
    14,
    46,
    3,
    44,
    (c) => {
      px(c, "rgba(0,0,0,0.25)", 0, 42, 8, 3);
      px(c, "#d0d0d0", 2, 2, 2, 42);
      px(c, "#2a5aa8", 4, 3, 10, 7);
      px(c, "#f2d23a", 4, 6, 10, 1);
    },
  ],
  planter: [
    14,
    16,
    7,
    15,
    (c) => {
      px(c, "#8a6a4a", 1, 8, 12, 7);
      px(c, "#6a4a32", 1, 8, 12, 2);
      px(c, "#3f8a3a", 2, 2, 10, 7);
      px(c, "#e86a8a", 4, 3, 2, 2);
      px(c, "#f2e27a", 8, 4, 2, 2);
    },
  ],
};

// ---------------------------------------------------------------- characters

const SKIN = ["#f1c8a4", "#d9a47a", "#a86f4a", "#6e4630"];
const HAIR = ["#2a1e18", "#6b3f22", "#d9b45a", "#8a8a8a", "#a83a2a"];
const SHIRT = ["#3a6fb0", "#b03a3a", "#3a9a5a", "#d9a23a", "#7a4ab0", "#e8e8e8", "#3a3a3a", "#d96aa0"];
const PANTS = ["#2a3450", "#4a4a4a", "#6b5a3a", "#2a2a2a"];

interface Look {
  skin: string;
  hair: string;
  shirt: string;
  pants: string;
  tie: boolean;
}

/** 16x24 character frame: dir 0 down, 1 up, 2 left, 3 right; step 0..2. */
function drawChar(c: CanvasRenderingContext2D, look: Look, dir: number, step: number) {
  px(c, "rgba(0,0,0,0.28)", 3, 21, 10, 3);
  const bob = step === 0 ? 0 : 1;
  const top = 2 + bob;
  // Legs (alternate feet).
  const lOff = step === 1 ? -1 : step === 2 ? 1 : 0;
  if (dir === 2 || dir === 3) {
    px(c, look.pants, 6 + lOff, top + 14, 2, 6 - bob);
    px(c, look.pants, 8 - lOff, top + 14, 2, 6 - bob);
    px(c, "#1e1a18", 6 + lOff, 21, 3, 1);
    px(c, "#1e1a18", 8 - lOff, 21, 3, 1);
  } else {
    px(c, look.pants, 5, top + 14, 2, 6 - bob + (step === 1 ? -1 : 0));
    px(c, look.pants, 9, top + 14, 2, 6 - bob + (step === 2 ? -1 : 0));
    px(c, "#1e1a18", 5, 21 - (step === 1 ? 1 : 0), 2, 1);
    px(c, "#1e1a18", 9, 21 - (step === 2 ? 1 : 0), 2, 1);
  }
  // Torso and arms.
  px(c, look.shirt, 4, top + 8, 8, 7);
  const swing = step === 1 ? 1 : step === 2 ? -1 : 0;
  if (dir === 2 || dir === 3) {
    px(c, look.shirt, 7 + swing, top + 9, 2, 6);
    px(c, look.skin, 7 + swing, top + 14, 2, 1);
  } else {
    px(c, look.shirt, 3, top + 9 + swing, 1, 5);
    px(c, look.shirt, 12, top + 9 - swing, 1, 5);
    px(c, look.skin, 3, top + 14 + swing, 1, 1);
    px(c, look.skin, 12, top + 14 - swing, 1, 1);
  }
  if (look.tie && dir === 0) px(c, "#c8282e", 7, top + 8, 2, 5);
  // Head.
  px(c, look.skin, 4, top, 8, 8);
  px(c, look.hair, 4, top - 1, 8, 3);
  if (dir === 1) px(c, look.hair, 4, top, 8, 6);
  else if (dir === 0) {
    px(c, look.hair, 4, top + 2, 1, 2);
    px(c, look.hair, 11, top + 2, 1, 2);
    px(c, "#1e1a18", 6, top + 4, 1, 2);
    px(c, "#1e1a18", 9, top + 4, 1, 2);
  } else {
    const back = dir === 2 ? 9 : 4;
    px(c, look.hair, back, top + 1, 3, 4);
    px(c, "#1e1a18", dir === 2 ? 5 : 10, top + 4, 1, 2);
  }
}

// ---------------------------------------------------------------- build

export function buildArt(town: Town, npcVariants = 16): Art {
  const p = packer();
  const buildingFrame: number[] = [];
  for (const b of town.buildings) {
    const w = b.w * TILE;
    const h = (b.h + 1) * TILE;
    buildingFrame[b.id] = p.add(w, h, 0, h, (c) => drawBuilding(c, b));
  }
  const propFrame = {} as Record<PropKind, number>;
  for (const kind of Object.keys(PROPS) as PropKind[]) {
    const [w, h, ax, ay, draw] = PROPS[kind];
    propFrame[kind] = p.add(w, h, ax, ay, draw);
  }
  const rng = mulberry32(7);
  const pick = <T>(a: readonly T[]): T => a[Math.floor(rng() * a.length)] as T;
  const looks: Look[] = [];
  for (let v = 0; v < npcVariants; v++) looks.push({ skin: pick(SKIN), hair: pick(HAIR), shirt: pick(SHIRT), pants: pick(PANTS), tie: false });
  looks.push({ skin: SKIN[1] ?? "#d9a47a", hair: HAIR[0] ?? "#222", shirt: "#23304a", pants: "#1c2438", tie: true });
  let charBase = -1;
  for (const look of looks) {
    for (let dir = 0; dir < 4; dir++) {
      for (let step = 0; step < 3; step++) {
        const f = p.add(16, 24, 8, 22, (c) => drawChar(c, look, dir, step));
        if (charBase < 0) charBase = f;
      }
    }
  }
  const marker = p.add(14, 8, 7, 4, (c) => {
    px(c, "#ffe066", 3, 0, 8, 1);
    px(c, "#ffe066", 0, 2, 2, 4);
    px(c, "#ffe066", 12, 2, 2, 4);
    px(c, "#ffe066", 3, 7, 8, 1);
  });
  const doorArrow = p.add(10, 9, 5, 9, (c) => {
    px(c, "#1e1a18", 0, 0, 10, 5);
    px(c, "#ffe066", 1, 1, 8, 3);
    px(c, "#1e1a18", 2, 5, 6, 2);
    px(c, "#ffe066", 3, 5, 4, 1);
    px(c, "#1e1a18", 4, 7, 2, 2);
  });
  const glow = p.add(64, 64, 32, 32, (c) => {
    const g = c.createRadialGradient(32, 32, 2, 32, 32, 32);
    g.addColorStop(0, "rgba(255,220,140,0.85)");
    g.addColorStop(1, "rgba(255,200,120,0)");
    c.fillStyle = g;
    c.fillRect(0, 0, 64, 64);
  });
  return {
    atlas: p.canvas,
    ground: bakeGround(town),
    frames: p.frames,
    buildingFrame,
    propFrame,
    charBase,
    variants: npcVariants,
    marker,
    doorArrow,
    glow,
  };
}
