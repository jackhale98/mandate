// A 2D canvas stand-in for tests (jsdom has no canvas): every method is a
// no-op that counts its calls, every property can be set and read back.
// Test-only.
import type { CanvasFactory, Ctx, Layer } from "./art";

export interface StubLog {
  /** Calls per method name, over every stub context. */
  calls: Record<string, number>;
  /** Canvases made. */
  canvases: number;
  /** Pixels allocated (width × height of every canvas when made). */
  pixels: number;
}

export const newLog = (): StubLog => ({ calls: {}, canvases: 0, pixels: 0 });

export function stubContext(log: StubLog, canvas: { width: number; height: number }): Ctx {
  const props: Record<string | symbol, unknown> = { canvas, globalAlpha: 1, globalCompositeOperation: "source-over", fillStyle: "#000", imageSmoothingEnabled: true };
  return new Proxy(props, {
    get(target, key) {
      if (key in target) return target[key];
      if (typeof key !== "string") return undefined;
      return (...args: unknown[]) => {
        log.calls[key] = (log.calls[key] ?? 0) + 1;
        for (const a of args) if (typeof a === "number" && !Number.isFinite(a)) throw new Error(`${key} got a non-finite number`);
        if (key.startsWith("create")) return { addColorStop() {} };
        return undefined;
      };
    },
    set(target, key, value) {
      target[key] = value;
      return true;
    },
  }) as unknown as Ctx;
}

export function stubCanvasFactory(log: StubLog): CanvasFactory {
  return (width: number, height: number): Layer => {
    log.canvases++;
    log.pixels += Math.ceil(width) * Math.ceil(height);
    const canvas = { width: Math.ceil(width), height: Math.ceil(height) } as unknown as HTMLCanvasElement;
    const ctx = stubContext(log, canvas);
    (canvas as unknown as { getContext: () => Ctx }).getContext = () => ctx;
    return { canvas, ctx };
  };
}
