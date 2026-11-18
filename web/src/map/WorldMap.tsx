// Strategic map (DEC-021): pixel-art raster plus vector overlays, with pan,
// zoom, pinch, and tap-to-select. Canvas 2D for M1; the map spike (M1s) may
// move this to PixiJS.
import { useEffect, useMemo, useRef, useState } from "react";
import type { MapView } from "../engine/protocol/MapView";
import { bakeRaster } from "./render";

interface Props {
  view: MapView;
  selected: number | null;
  onSelect: (municipalityId: number | null) => void;
  label: string;
}

interface Camera {
  x: number; // map cell at the view centre
  y: number;
  scale: number; // screen px per cell
}

const CLASS_RADIUS: Record<string, number> = { metro: 5, city: 4, town: 3, rural: 2 };

export function WorldMap({ view, selected, onSelect, label }: Props) {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const wrapRef = useRef<HTMLDivElement>(null);
  const raster = useMemo(() => bakeRaster(view, selected), [view, selected]);
  const offscreen = useMemo(() => {
    const c = document.createElement("canvas");
    c.width = raster.width;
    c.height = raster.height;
    c.getContext("2d")?.putImageData(raster.image, 0, 0);
    return c;
  }, [raster]);
  const [size, setSize] = useState({ w: 320, h: 320 });
  const capital = view.municipalities.find((m) => m.national_capital);
  // scale 0 = "auto": fit the country, zoomed in on portrait screens.
  const [cam, setCam] = useState<Camera>({ x: view.width / 2, y: view.height / 2, scale: 0 });

  // Fit to container.
  useEffect(() => {
    const el = wrapRef.current;
    if (!el) return;
    const ro = new ResizeObserver(([entry]) => {
      if (!entry) return;
      const { width, height } = entry.contentRect;
      setSize({ w: Math.max(1, width), h: Math.max(1, height) });
    });
    ro.observe(el);
    return () => ro.disconnect();
  }, []);
  const fitScale = Math.min(size.w / view.width, size.h / view.height);
  const portrait = size.h > size.w * 1.2;
  const autoScale = portrait ? fitScale * 1.9 : fitScale;
  const scale = cam.scale || autoScale;
  // Auto mode centres on the capital in portrait, the whole map otherwise.
  const camX = cam.scale ? cam.x : portrait && capital ? capital.x : view.width / 2;
  const camY = cam.scale ? cam.y : portrait && capital ? capital.y : view.height / 2;

  // Draw.
  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;
    const dpr = window.devicePixelRatio || 1;
    canvas.width = Math.round(size.w * dpr);
    canvas.height = Math.round(size.h * dpr);
    const ctx = canvas.getContext("2d");
    if (!ctx) return;
    ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
    ctx.fillStyle = "#26466f";
    ctx.fillRect(0, 0, size.w, size.h);
    const ox = size.w / 2 - camX * scale;
    const oy = size.h / 2 - camY * scale;
    ctx.imageSmoothingEnabled = false;
    ctx.drawImage(offscreen, ox, oy, view.width * scale, view.height * scale);
    const toScreen = (cell: number) => [ox + ((cell % view.width) + 0.5) * scale, oy + (Math.floor(cell / view.width) + 0.5) * scale] as const;

    // Transport.
    for (const l of view.links) {
      if (l.path.length < 2) continue;
      ctx.beginPath();
      l.path.forEach((c, k) => {
        const [x, y] = toScreen(c);
        if (k === 0) ctx.moveTo(x, y);
        else ctx.lineTo(x, y);
      });
      if (l.mode === "rail") {
        ctx.strokeStyle = "rgba(30,30,30,0.85)";
        ctx.lineWidth = Math.max(1, scale * 0.35);
        ctx.setLineDash([Math.max(2, scale), Math.max(2, scale)]);
      } else {
        ctx.strokeStyle = l.mode === "highway" ? "rgba(196,72,48,0.9)" : "rgba(130,92,62,0.8)";
        ctx.lineWidth = Math.max(1, scale * (l.mode === "highway" ? 0.45 : 0.3));
        ctx.setLineDash([]);
      }
      ctx.stroke();
    }
    ctx.setLineDash([]);

    // Settlements and labels.
    const zoom = scale / fitScale;
    ctx.textAlign = "center";
    ctx.font = "600 12px system-ui, sans-serif";
    for (const m of view.municipalities) {
      const x = ox + (m.x + 0.5) * scale;
      const y = oy + (m.y + 0.5) * scale;
      const r = (CLASS_RADIUS[m.class] ?? 2) * Math.min(2, Math.max(0.8, zoom * 0.6));
      ctx.fillStyle = "#141414";
      ctx.fillRect(x - r - 1, y - r - 1, 2 * r + 2, 2 * r + 2);
      ctx.fillStyle = m.national_capital ? "#ffd23f" : m.id === selected ? "#ffb347" : "#fafafa";
      ctx.fillRect(x - r, y - r, 2 * r, 2 * r);
      const show = m.class === "metro" || zoom > 1.6 || (m.class === "city" && zoom > 1.1) || m.id === selected;
      if (show) {
        ctx.lineWidth = 3;
        ctx.strokeStyle = "rgba(10,10,14,0.85)";
        ctx.strokeText(m.name, x, y - r - 4);
        ctx.fillStyle = "#fff";
        ctx.fillText(m.name, x, y - r - 4);
      }
    }
    if (zoom < 1.6) {
      ctx.font = "700 13px Georgia, serif";
      for (const rg of view.regions) {
        const x = ox + rg.label_x * scale;
        const y = oy + rg.label_y * scale + 14;
        ctx.lineWidth = 3;
        ctx.strokeStyle = "rgba(255,255,255,0.7)";
        ctx.strokeText(rg.name.toUpperCase(), x, y);
        ctx.fillStyle = "rgba(40,30,30,0.9)";
        ctx.fillText(rg.name.toUpperCase(), x, y);
      }
    }
  }, [offscreen, camX, camY, scale, fitScale, size, view, selected]);

  // Interaction: pan, wheel zoom, pinch, tap.
  const pointers = useRef(new Map<number, { x: number; y: number }>());
  const gesture = useRef<{ moved: boolean; startDist: number; startScale: number }>({ moved: false, startDist: 0, startScale: 1 });

  const clampCam = (c: Camera): Camera => ({
    scale: Math.min(Math.max(c.scale, fitScale * 0.8), fitScale * 10),
    x: Math.min(Math.max(c.x, 0), view.width),
    y: Math.min(Math.max(c.y, 0), view.height),
  });

  const zoomAt = (factor: number, sx: number, sy: number) => {
    setCam((c) => {
      const s0 = c.scale || autoScale;
      const cx0 = c.scale ? c.x : camX;
      const cy0 = c.scale ? c.y : camY;
      const s1 = Math.min(Math.max(s0 * factor, fitScale * 0.8), fitScale * 10);
      // Keep the cell under the cursor fixed.
      const mx = cx0 + (sx - size.w / 2) / s0;
      const my = cy0 + (sy - size.h / 2) / s0;
      return clampCam({ scale: s1, x: mx - (sx - size.w / 2) / s1, y: my - (sy - size.h / 2) / s1 });
    });
  };

  const local = (e: { clientX: number; clientY: number }) => {
    const r = canvasRef.current?.getBoundingClientRect();
    return { x: e.clientX - (r?.left ?? 0), y: e.clientY - (r?.top ?? 0) };
  };

  const onPointerDown = (e: React.PointerEvent) => {
    (e.target as Element).setPointerCapture(e.pointerId);
    pointers.current.set(e.pointerId, local(e));
    gesture.current.moved = false;
    if (pointers.current.size === 2) {
      const [a, b] = [...pointers.current.values()];
      if (a && b) gesture.current = { moved: true, startDist: Math.hypot(a.x - b.x, a.y - b.y), startScale: scale };
    }
  };
  const onPointerMove = (e: React.PointerEvent) => {
    const prev = pointers.current.get(e.pointerId);
    if (!prev) return;
    const cur = local(e);
    pointers.current.set(e.pointerId, cur);
    if (pointers.current.size === 2) {
      const [a, b] = [...pointers.current.values()];
      if (!a || !b) return;
      const d = Math.hypot(a.x - b.x, a.y - b.y);
      const target = (gesture.current.startScale * d) / Math.max(1, gesture.current.startDist);
      zoomAt(target / scale, (a.x + b.x) / 2, (a.y + b.y) / 2);
      return;
    }
    const dx = cur.x - prev.x;
    const dy = cur.y - prev.y;
    if (Math.abs(dx) + Math.abs(dy) > 2) gesture.current.moved = true;
    setCam((c) => {
      const s0 = c.scale || autoScale;
      const x0 = c.scale ? c.x : camX;
      const y0 = c.scale ? c.y : camY;
      return clampCam({ scale: s0, x: x0 - dx / s0, y: y0 - dy / s0 });
    });
  };
  const onPointerUp = (e: React.PointerEvent) => {
    const wasTap = !gesture.current.moved && pointers.current.size === 1;
    pointers.current.delete(e.pointerId);
    if (!wasTap) return;
    const p = local(e);
    const cx = Math.floor(camX + (p.x - size.w / 2) / scale);
    const cy = Math.floor(camY + (p.y - size.h / 2) / scale);
    if (cx < 0 || cy < 0 || cx >= view.width || cy >= view.height) return onSelect(null);
    const o = raster.owner[cy * view.width + cx] ?? 65535;
    onSelect(o === 65535 ? null : o);
  };

  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;
    const onWheel = (e: WheelEvent) => {
      e.preventDefault();
      const r = canvas.getBoundingClientRect();
      zoomAt(Math.exp(-e.deltaY * 0.0015), e.clientX - r.left, e.clientY - r.top);
    };
    canvas.addEventListener("wheel", onWheel, { passive: false });
    return () => canvas.removeEventListener("wheel", onWheel);
  });

  return (
    <div ref={wrapRef} className="map-wrap">
      <canvas
        ref={canvasRef}
        className="map-canvas"
        role="img"
        aria-label={label}
        style={{ width: size.w, height: size.h }}
        onPointerDown={onPointerDown}
        onPointerMove={onPointerMove}
        onPointerUp={onPointerUp}
        onPointerCancel={(e) => pointers.current.delete(e.pointerId)}
      />
    </div>
  );
}
