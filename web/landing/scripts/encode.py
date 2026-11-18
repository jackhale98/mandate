"""Encode the captured game images for the landing page (DEC-083).

Reads the raw PNGs written by capture.mjs (target/landing-capture/) and writes
AVIF and WebP files to web/landing/img/. Maps are cropped to the land (the sea
margin of the map screen is dropped). Needs Pillow with AVIF and WebP support.

    python3 web/landing/scripts/encode.py
    node web/landing/scripts/og.mjs        # then the share image
"""

from pathlib import Path

from PIL import Image

ROOT = Path(__file__).resolve().parents[3]
RAW = ROOT / "target" / "landing-capture"
OUT = ROOT / "web" / "landing" / "img"
SEA = (38, 70, 111)

# (seed, widths). The first map is the front-page picture (LCP on desktop).
MAPS = [("4040", (1200, 720)), ("777", (720,)), ("20260", (720,)), ("1848", (720,))]
SCREENS = ["town", "chamber", "night"]
SCREEN_WIDTH = 600  # shown at up to 300 CSS px


def land_box(im: Image.Image, pad: int = 28) -> tuple[int, int, int, int]:
    """Bounding box of everything that is not open sea, plus a margin."""
    rgb = im.convert("RGB")
    px = rgb.load()
    w, h = rgb.size
    m = Image.new("1", (w, h), 0)
    mp = m.load()
    for y in range(0, h, 2):
        for x in range(0, w, 2):
            r, g, b = px[x, y]
            if abs(r - SEA[0]) + abs(g - SEA[1]) + abs(b - SEA[2]) > 24:
                mp[x, y] = 1
    left, top, right, bottom = m.getbbox() or (0, 0, w, h)
    return (max(0, left - pad), max(0, top - pad), min(w, right + pad), min(h, bottom + pad))


def save(im: Image.Image, stem: str) -> None:
    im = im.convert("RGB")
    im.save(OUT / f"{stem}.avif", quality=58, speed=4)
    im.save(OUT / f"{stem}.webp", quality=80, method=6)
    print(stem, im.size, *(f"{(OUT / f'{stem}.{e}').stat().st_size // 1024} KB {e}" for e in ("avif", "webp")))


def main() -> None:
    OUT.mkdir(parents=True, exist_ok=True)
    for seed, widths in MAPS:
        im = Image.open(RAW / f"map-{seed}.png")
        crop = im.crop(land_box(im))
        crop.save(RAW / f"map-{seed}-crop.png")
        for w in widths:
            h = round(crop.height * w / crop.width)
            save(crop.resize((w, h), Image.Resampling.LANCZOS), f"map-{seed}-{w}")
    for name in SCREENS:
        for scheme in ("dark", "light"):
            im = Image.open(RAW / f"{name}-{scheme}.png")
            h = round(im.height * SCREEN_WIDTH / im.width)
            save(im.resize((SCREEN_WIDTH, h), Image.Resampling.LANCZOS), f"{name}-{scheme}")


if __name__ == "__main__":
    main()
