// A large pixel portrait of someone (the conversation away from the street,
// DEC-093): their street sprite's bust (./portraitArt), upscaled whole art
// pixels at a time in a framed window. Decorative: the conversation names
// them in text (DEC-051).
import { useEffect, useRef, useState } from "react";
import "./portrait.css";

export function Portrait({ variant, player = false, className = "" }: { variant: number; player?: boolean; className?: string }) {
  const canvas = useRef<HTMLCanvasElement>(null);
  const [drawn, setDrawn] = useState(false);
  useEffect(() => {
    const c = canvas.current;
    // Unit tests have no canvas to draw on.
    if (!c || import.meta.env.MODE === "test") return;
    const ctx = c.getContext("2d");
    if (!ctx) return;
    let live = true;
    setDrawn(false);
    void import("./portraitArt")
      .then(({ bust }) => bust(variant, player))
      .then((b) => {
        if (!live) return;
        c.width = b.w;
        c.height = b.h;
        const img = ctx.createImageData(b.w, b.h);
        img.data.set(b.rgba);
        ctx.putImageData(img, 0, 0);
        setDrawn(true);
      })
      .catch(() => {
        // No pack (the worker failed): the frame stays empty; the conversation goes on.
      });
    return () => {
      live = false;
    };
  }, [variant, player]);
  return (
    <div className={`portrait ${className}`} aria-hidden="true" data-testid="talk-portrait" data-drawn={drawn ? "1" : "0"}>
      <canvas ref={canvas} className="portrait-canvas" width={16} height={16} />
    </div>
  );
}
