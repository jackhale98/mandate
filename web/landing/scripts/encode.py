"""Encode the captured game images for the landing page (DEC-083).

Reads the raw PNGs written by capture.mjs (target/landing-capture/) and writes
AVIF and WebP files to web/landing/img/. Terrain maps are cropped to the
country (its land in colour; the grey surroundings and the sea mostly go).
Needs Pillow with AVIF and WebP support. `npm run capture` runs this.

    python3 web/landing/scripts/encode.py
"""

import colorsys
from pathlib import Path

from PIL import Image

ROOT = Path(__file__).resolve().parents[3]
RAW = ROOT / "target" / "landing-capture"
OUT = ROOT / "web" / "landing" / "img"

# (seed, widths). The first map is the front-page picture (LCP on desktop).
MAPS = [("4040", (1200, 720)), ("777", (720,)), ("20260", (720,)), ("1848", (720,))]
# Phone screens, light and dark: (name, width). Shown at up to 240 CSS px in a
# row, the election night at up to 300.
SCREENS = [("street", 480), ("talk", 480), ("room", 480), ("notebook", 480), ("night", 600)]
# Wide pictures, light and dark: the desktop street (up to ~1100 CSS px) and
# the country with a data layer (a tile in the map grid, up to ~560 CSS px).
# The large size is AVIF only; browsers without AVIF get the small WebP.
WIDE = [("desktop", (1600, 960)), ("layer", (1200, 720))]


def country_box(im: Image.Image, aspect: float, pad: float = 0.05) -> tuple[int, int, int, int]:
    """Bounding box of the country's own land (greens, yellows and browns in colour), plus a margin."""
    small = im.convert("RGB").resize((im.width // 4, im.height // 4), Image.Resampling.NEAREST)
    px = small.load()
    w, h = small.size
    mask = Image.new("1", (w, h), 0)
    mp = mask.load()
    for y in range(h):
        for x in range(w):
            r, g, b = px[x, y]
            hue, sat, val = colorsys.rgb_to_hsv(r / 255, g / 255, b / 255)
            if 0.08 <= hue <= 0.45 and sat > 0.3 and val > 0.25:
                mp[x, y] = 1
    left, top, right, bottom = mask.getbbox() or (0, 0, w, h)
    m = pad * max(right - left, bottom - top)
    left, top, right, bottom = left - m, top - m, right + m, bottom + m
    # Widen or heighten to the tiles' shape (the layer capture's, so the map
    # tiles line up), about the country's middle, inside the picture.
    cw, ch = right - left, bottom - top
    cw, ch = max(cw, ch * aspect), max(ch, cw / aspect)
    cw, ch = min(cw, w, h * aspect), min(ch, h, w / aspect)
    cx = min(max((left + right) / 2, cw / 2), w - cw / 2)
    cy = min(max((top + bottom) / 2, ch / 2), h - ch / 2)
    return tuple(round(v * 4) for v in (cx - cw / 2, cy - ch / 2, cx + cw / 2, cy + ch / 2))  # type: ignore[return-value]


def save(im: Image.Image, stem: str, formats: tuple[str, ...] = ("avif", "webp")) -> None:
    im = im.convert("RGB")
    if "avif" in formats:
        im.save(OUT / f"{stem}.avif", quality=56, speed=4)
    if "webp" in formats:
        im.save(OUT / f"{stem}.webp", quality=76, method=6)
    print(stem, im.size, *(f"{(OUT / f'{stem}.{e}').stat().st_size // 1024} KB {e}" for e in formats))


def scaled(im: Image.Image, width: int) -> Image.Image:
    return im.resize((width, round(im.height * width / im.width)), Image.Resampling.LANCZOS)


def main() -> None:
    OUT.mkdir(parents=True, exist_ok=True)
    lw, lh = Image.open(RAW / "layer-dark.png").size
    for seed, widths in MAPS:
        im = Image.open(RAW / f"map-{seed}.png")
        crop = im.crop(country_box(im, lw / lh))
        crop.save(RAW / f"map-{seed}-crop.png")
        for w in widths:
            save(scaled(crop, w), f"map-{seed}-{w}")
    for name, width in SCREENS:
        for scheme in ("dark", "light"):
            save(scaled(Image.open(RAW / f"{name}-{scheme}.png"), width), f"{name}-{scheme}")
    for name, widths in WIDE:
        for scheme in ("dark", "light"):
            im = Image.open(RAW / f"{name}-{scheme}.png")
            for w in widths:
                save(scaled(im, w), f"{name}-{scheme}-{w}", ("avif",) if w == max(widths) else ("avif", "webp"))


if __name__ == "__main__":
    main()
