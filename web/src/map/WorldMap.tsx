// Strategic map (DEC-021): pixel-art raster plus vector overlays, with pan,
// zoom, pinch, and tap-to-select. Canvas 2D for M1; the map spike (M1s) may
// move this to PixiJS.
import { useEffect, useMemo, useRef, useState } from "react";
import type { MapView } from "../engine/protocol/MapView";
import { bakeRaster, type Overlay } from "./render";

interface Props {
  view: MapView;
  selected: number | null;
  onSelect: (municipalityId: number | null) => void;
  label: string;
  overlay: Overlay;
}

interface Camera {
  x: number; // map cell at the view centre
  y: number;
  scale: number; // screen px per cell
}

const CLASS_RADIUS: Record<string, number> = { metro: 5, city: 4, town: 3, rural: 2 };

export function WorldMap({ view, selected, onSelect, label, overlay }: Props) {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const wrapRef = useRef<HTMLDivElement>(null);
  const raster = useMemo(() => bakeRaster(view, selected, overlay), [view, selected, overlay]);
  // Terrain plus the transport network, baked once into a high-resolution
  // image (DEC-049 pixel art). Gestures only scale this image, so zooming never
  // re-strokes thousands of road segments per frame.
  const offscreen = useMemo(() => {
    const k = Math.max(1, Math.min(6, Math.floor(2048 / Math.max(raster.width, raster.height))));
    const base = document.createElement("canvas");
    base.width = raster.width;
    base.height = raster.height;
    base.getContext("2d")?.putImageData(raster.image, 0, 0);
    const c = document.createElement("canvas");
    c.width = raster.width * k;
    c.height = raster.height * k;
    const ctx = c.getContext("2d");
    if (!ctx) return c;
    ctx.imageSmoothingEnabled = false;
    ctx.drawImage(base, 0, 0, c.width, c.height);
    const at = (cell: number) => [((cell % view.width) + 0.5) * k, (Math.floor(cell / view.width) + 0.5) * k] as const;
    for (const l of view.links) {
      if (l.path.length < 2) continue;
      ctx.beginPath();
      l.path.forEach((cell, i) => {
        const [x, y] = at(cell);
        if (i === 0) ctx.moveTo(x, y);
        else ctx.lineTo(x, y);
      });
      if (l.mode === "rail") {
        ctx.strokeStyle = "rgba(30,30,30,0.85)";
        ctx.lineWidth = Math.max(1, k * 0.35);
        ctx.setLineDash([k, k]);
      } else {
        ctx.strokeStyle = l.mode === "highway" ? "rgba(196,72,48,0.9)" : "rgba(130,92,62,0.8)";
        ctx.lineWidth = Math.max(1, k * (l.mode === "highway" ? 0.45 : 0.3));
        ctx.setLineDash([]);
      }
      ctx.stroke();
    }
    return c;
  }, [raster, view]);
  const [size, setSize] = useState({ w: 320, h: 320 });
  const capital = view.municipalities.find((m) => m.national_capital);

  // The camera lives outside React state: gestures mutate it and request one
  // redraw per animation frame, so panning and zooming never re-render the tree.
  const fitScale = Math.min(size.w / view.width, size.h / view.height);
  const portrait = size.h > size.w * 1.2;
  const cam = useRef<Camera | null>(null);
  const frame = useRef(0);
  const camera = (): Camera => {
    if (!cam.current) {
      // Auto: fit the country; on portrait screens zoom in on the capital.
      cam.current = {
        scale: portrait ? fitScale * 1.9 : fitScale,
        x: portrait && capital ? capital.x : view.width / 2,
        y: portrait && capital ? capital.y : view.height / 2,
      };
    }
    return cam.current;
  };

  // Latest drawing inputs, read by the frame callback.
  const scene = useRef({ offscreen, size, view, selected, fitScale });
  // Effects run in order: this sync runs before the redraw effects below.
  useEffect(() => {
    scene.current = { offscreen, size, view, selected, fitScale };
  });

  const draw = () => {
    frame.current = 0;
    const canvas = canvasRef.current;
    const ctx = canvas?.getContext("2d");
    if (!canvas || !ctx) return;
    const { offscreen: img, size: sz, view: v, selected: sel, fitScale: fit } = scene.current;
    const { x: camX, y: camY, scale } = camera();
    const dpr = window.devicePixelRatio || 1;
    ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
    ctx.fillStyle = "#26466f";
    ctx.fillRect(0, 0, sz.w, sz.h);
    const ox = sz.w / 2 - camX * scale;
    const oy = sz.h / 2 - camY * scale;
    ctx.imageSmoothingEnabled = false;
    ctx.drawImage(img, ox, oy, v.width * scale, v.height * scale);

    // Settlements and labels.
    const zoom = scale / fit;
    ctx.textAlign = "center";
    ctx.font = "600 12px system-ui, sans-serif";
    for (const m of v.municipalities) {
      const x = ox + (m.x + 0.5) * scale;
      const y = oy + (m.y + 0.5) * scale;
      const r = (CLASS_RADIUS[m.class] ?? 2) * Math.min(2, Math.max(0.8, zoom * 0.6));
      ctx.fillStyle = "#141414";
      ctx.fillRect(x - r - 1, y - r - 1, 2 * r + 2, 2 * r + 2);
      ctx.fillStyle = m.national_capital ? "#ffd23f" : m.id === sel ? "#ffb347" : "#fafafa";
      ctx.fillRect(x - r, y - r, 2 * r, 2 * r);
      const show = m.class === "metro" || zoom > 1.6 || (m.class === "city" && zoom > 1.1) || m.id === sel;
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
      for (const rg of v.regions) {
        const x = ox + rg.label_x * scale;
        const y = oy + rg.label_y * scale + 14;
        ctx.lineWidth = 3;
        ctx.strokeStyle = "rgba(255,255,255,0.7)";
        ctx.strokeText(rg.name.toUpperCase(), x, y);
        ctx.fillStyle = "rgba(40,30,30,0.9)";
        ctx.fillText(rg.name.toUpperCase(), x, y);
      }
    }
  };
  const requestDraw = () => {
    if (!frame.current) frame.current = requestAnimationFrame(draw);
  };

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

  // Resize the backing store only when the container size changes (resizing
  // a canvas reallocates it, which is what made gestures stutter).
  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;
    const dpr = window.devicePixelRatio || 1;
    canvas.width = Math.round(size.w * dpr);
    canvas.height = Math.round(size.h * dpr);
    cam.current = null; // re-fit for the new size
    requestDraw();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [size]);

  // Redraw when the map content or selection changes.
  useEffect(() => {
    requestDraw();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [offscreen, view, selected]);

  useEffect(() => () => cancelAnimationFrame(frame.current), []);

  // Interaction: pan, wheel zoom, pinch, tap.
  const pointers = useRef(new Map<number, { x: number; y: number }>());
  const gesture = useRef<{ moved: boolean; startDist: number; startScale: number }>({ moved: false, startDist: 0, startScale: 1 });

  const clampCam = (c: Camera): Camera => ({
    scale: Math.min(Math.max(c.scale, fitScale * 0.8), fitScale * 10),
    x: Math.min(Math.max(c.x, 0), view.width),
    y: Math.min(Math.max(c.y, 0), view.height),
  });

  /** Zoom to an absolute scale, keeping the cell under (sx, sy) fixed. */
  const zoomTo = (target: number, sx: number, sy: number) => {
    const c = camera();
    const s1 = Math.min(Math.max(target, fitScale * 0.8), fitScale * 10);
    const mx = c.x + (sx - size.w / 2) / c.scale;
    const my = c.y + (sy - size.h / 2) / c.scale;
    cam.current = clampCam({ scale: s1, x: mx - (sx - size.w / 2) / s1, y: my - (sy - size.h / 2) / s1 });
    requestDraw();
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
      if (a && b) gesture.current = { moved: true, startDist: Math.hypot(a.x - b.x, a.y - b.y), startScale: camera().scale };
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
      zoomTo((gesture.current.startScale * d) / Math.max(1, gesture.current.startDist), (a.x + b.x) / 2, (a.y + b.y) / 2);
      return;
    }
    const dx = cur.x - prev.x;
    const dy = cur.y - prev.y;
    if (Math.abs(dx) + Math.abs(dy) > 2) gesture.current.moved = true;
    const c = camera();
    cam.current = clampCam({ scale: c.scale, x: c.x - dx / c.scale, y: c.y - dy / c.scale });
    requestDraw();
  };
  const onPointerUp = (e: React.PointerEvent) => {
    const wasTap = !gesture.current.moved && pointers.current.size === 1;
    pointers.current.delete(e.pointerId);
    if (!wasTap) return;
    const p = local(e);
    const c = camera();
    const cx = Math.floor(c.x + (p.x - size.w / 2) / c.scale);
    const cy = Math.floor(c.y + (p.y - size.h / 2) / c.scale);
    if (cx < 0 || cy < 0 || cx >= view.width || cy >= view.height) return onSelect(null);
    const o = raster.owner[cy * view.width + cx] ?? 65535;
    onSelect(o === 65535 ? null : o);
  };

  // Wheel zoom (non-passive so the page does not scroll). The handler reads
  // the latest zoom function through a ref, so it is attached once.
  const zoomRef = useRef(zoomTo);
  useEffect(() => {
    zoomRef.current = zoomTo;
  });
  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;
    const onWheel = (e: WheelEvent) => {
      e.preventDefault();
      const r = canvas.getBoundingClientRect();
      zoomRef.current(camera().scale * Math.exp(-e.deltaY * 0.0015), e.clientX - r.left, e.clientY - r.top);
    };
    canvas.addEventListener("wheel", onWheel, { passive: false });
    return () => canvas.removeEventListener("wheel", onWheel);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

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
