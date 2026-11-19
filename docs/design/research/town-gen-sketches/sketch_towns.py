#!/usr/bin/env python3
"""THROWAWAY SKETCH SCRIPT: illustrations for docs/design/research/TOWN_GENERATION.md.

These images illustrate the town layout v2 design. This is NOT the generator
and is not meant to become it: the real generator is TypeScript on the client,
integer-only and versioned (DEC-085), built from content registries. This
script is hand-tuned per scenario, uses floats and Python's `random`, and only
needs to make pictures that show the ideas: towns grown from their geography,
with a centre, arterials, bridges at narrow crossings, districts that read,
landmarks, reserved land, decline, and hidden places.

Each scenario borrows its inputs from a real generated town (seed 4242,
medium world), noted in the image title.

Usage (Python 3 with Pillow):
    python3 docs/design/research/town-gen-sketches/sketch_towns.py [out_dir]
"""

import heapq
import math
import os
import random
import sys

from PIL import Image, ImageDraw, ImageFont

FONT_DIR = "/usr/share/fonts/truetype/dejavu/"


def font(size, bold=False, italic=False):
    name = {(False, False): "DejaVuSans", (True, False): "DejaVuSans-Bold", (False, True): "DejaVuSans-Oblique", (True, True): "DejaVuSans-BoldOblique"}[(bold, italic)]
    try:
        return ImageFont.truetype(FONT_DIR + name + ".ttf", size)
    except OSError:
        return ImageFont.load_default()


# ---------------------------------------------------------------- palette

DISTRICT_STYLE = {
    # type: (ground tint, roof colour, legend name)
    "old_core": ((238, 214, 176), (176, 86, 64), "Old core (organic streets, market square)"),
    "terraces": ((234, 204, 196), (150, 78, 82), "Terraces (tight grid)"),
    "riverside": ((214, 222, 236), (120, 96, 120), "Riverside terraces (flood-prone)"),
    "suburb": ((214, 228, 186), (200, 128, 92), "Suburbs (curving streets, gardens)"),
    "hillside": ((222, 222, 190), (186, 110, 84), "Hillside streets (follow the contours)"),
    "estate": ((222, 232, 214), (140, 146, 164), "Post-war estate"),
    "industrial": ((206, 202, 208), (112, 116, 134), "Industrial edge (big lots, yards)"),
    "docks": ((200, 212, 224), (98, 112, 138), "Docks and harbour"),
    "pit_rows": ((212, 196, 180), (104, 88, 84), "Pit rows (mining district)"),
    "campus": ((226, 214, 238), (128, 98, 160), "University quarter (campus)"),
    "rural": ((226, 228, 180), (160, 120, 80), "Rural fringe (farms, lanes)"),
    "village": ((236, 218, 182), (172, 92, 66), "Village centre"),
}

FIELD_COLOURS = [(214, 214, 150), (196, 210, 140), (226, 206, 140), (188, 200, 150), (230, 222, 168)]
WATER = (86, 146, 204)
SEA = (64, 118, 178)
SEA_DEEP = (52, 96, 156)
LAKE = (92, 150, 200)
ARTERIAL = (250, 238, 204)
ARTERIAL_EDGE = (150, 124, 92)
STREET = (238, 234, 226)
STREET_EDGE = (176, 170, 160)
LANE = (216, 198, 152)
BRIDGE = (190, 164, 126)
BRIDGE_EDGE = (92, 72, 52)
RAIL = (66, 64, 70)
PLAZA = (236, 226, 204)
PARK = (168, 206, 140)
WOOD = (96, 140, 92)
WOOD_DOT = (64, 108, 70)
SAND = (238, 224, 172)
QUAY = (184, 182, 176)
YARD = (196, 194, 190)
SPOIL = (92, 86, 82)
TEXT = (34, 32, 40)
HIDDEN = (150, 60, 160)
RESERVED = (226, 132, 40)

# ---------------------------------------------------------------- helpers


def lerp(a, b, t):
    return a + (b - a) * t


def mixc(c1, c2, t):
    return tuple(int(round(lerp(c1[i], c2[i], t))) for i in range(3))


def shade(c, f):
    return tuple(max(0, min(255, int(round(v * f)))) for v in c)


class Noise:
    """Value noise (sketch quality)."""

    def __init__(self, seed, cell):
        self.seed = seed
        self.cell = cell

    def lat(self, i, j):
        r = random.Random((self.seed * 73856093) ^ (i * 19349663) ^ (j * 83492791))
        return r.random()

    def at(self, x, y):
        fx, fy = x / self.cell, y / self.cell
        i, j = math.floor(fx), math.floor(fy)
        tx, ty = fx - i, fy - j
        tx, ty = tx * tx * (3 - 2 * tx), ty * ty * (3 - 2 * ty)
        a = lerp(self.lat(i, j), self.lat(i + 1, j), tx)
        b = lerp(self.lat(i, j + 1), self.lat(i + 1, j + 1), tx)
        return lerp(a, b, ty)


def catmull(points, step=0.2):
    """Dense smooth polyline through control points."""
    out = []
    pts = [points[0]] + points + [points[-1]]
    for k in range(1, len(pts) - 2):
        p0, p1, p2, p3 = pts[k - 1], pts[k], pts[k + 1], pts[k + 2]
        seg = max(1, int(math.dist(p1, p2) / step))
        for s in range(seg):
            t = s / seg
            t2, t3 = t * t, t * t * t
            x = 0.5 * ((2 * p1[0]) + (-p0[0] + p2[0]) * t + (2 * p0[0] - 5 * p1[0] + 4 * p2[0] - p3[0]) * t2 + (-p0[0] + 3 * p1[0] - 3 * p2[0] + p3[0]) * t3)
            y = 0.5 * ((2 * p1[1]) + (-p0[1] + p2[1]) * t + (2 * p0[1] - 5 * p1[1] + 4 * p2[1] - p3[1]) * t2 + (-p0[1] + 3 * p1[1] - 3 * p2[1] + p3[1]) * t3)
            out.append((x, y))
    out.append(points[-1])
    return out


N8 = [(1, 0), (-1, 0), (0, 1), (0, -1), (1, 1), (1, -1), (-1, 1), (-1, -1)]
N4 = N8[:4]

# ---------------------------------------------------------------- town model


class Town:
    def __init__(self, W, H, seed, contour=10.0):
        self.W, self.H = W, H
        self.rng = random.Random(seed)
        self.seed = seed
        self.h = [[0.0] * W for _ in range(H)]
        self.water = [[0] * W for _ in range(H)]  # 0 land, 1 river, 2 sea, 3 lake, 4 creek
        self.road = [[0] * W for _ in range(H)]  # 0 none, 1 lane, 2 street, 3 arterial
        self.rail = [[False] * W for _ in range(H)]
        self.dist = [[-1] * W for _ in range(H)]
        self.special = [[None] * W for _ in range(H)]
        self.bld = [[None] * W for _ in range(H)]
        self.derelict = [[False] * W for _ in range(H)]
        self.flood = [[False] * W for _ in range(H)]
        self.reserved = [[False] * W for _ in range(H)]
        self.field = [[-1] * W for _ in range(H)]
        self.districts = []  # dicts: name, type, seed, weight, pattern params
        self.labels = []  # (x, y, text, kind)
        self.icons = []  # (kind, x, y)
        self.contour = contour
        self.steps = []  # stair lines (x0,y0,x1,y1) in tiles

    # -- grid
    def inb(self, x, y):
        return 0 <= x < self.W and 0 <= y < self.H

    def land(self, x, y):
        return self.inb(x, y) and self.water[y][x] == 0

    def set_heights(self, fn):
        for y in range(self.H):
            for x in range(self.W):
                self.h[y][x] = fn(x, y)

    # -- water
    def paint_line(self, pts, width, val, carve=0.0):
        r = width / 2.0
        dense = catmull(pts)
        for (px, py) in dense:
            for y in range(int(py - r - 1), int(py + r + 2)):
                for x in range(int(px - r - 1), int(px + r + 2)):
                    if not self.inb(x, y):
                        continue
                    d = math.dist((x + 0.5, y + 0.5), (px, py))
                    if d <= r:
                        if self.water[y][x] in (0, 4) or val in (1, 3):
                            self.water[y][x] = val
                    if carve and d <= r + 6:
                        self.h[y][x] -= carve * max(0.0, 1 - d / (r + 6)) * 0.08
        return dense

    def sea_where(self, fn):
        for y in range(self.H):
            for x in range(self.W):
                if fn(x, y):
                    self.water[y][x] = 2

    def lake_blob(self, cx, cy, rx, ry, seed):
        n = Noise(seed, 6)
        for y in range(self.H):
            for x in range(self.W):
                d = ((x - cx) / rx) ** 2 + ((y - cy) / ry) ** 2
                if d + (n.at(x, y) - 0.5) * 0.7 < 1:
                    self.water[y][x] = 3

    # -- sites
    def site(self, x, y, w, h, kind="site"):
        for j in range(y, y + h):
            for i in range(x, x + w):
                if self.inb(i, j):
                    self.special[j][i] = kind

    # -- routing
    def route(self, a, b, cls, slope_k=0.25, water_cost=16.0, reuse=0.35, bridge_ok=True, rail=False):
        a = (int(round(a[0])), int(round(a[1])))
        b = (int(round(b[0])), int(round(b[1])))
        INF = float("inf")
        g = {a: 0.0}
        came = {}
        pq = [(0.0, a)]
        while pq:
            _, cur = heapq.heappop(pq)
            if cur == b:
                break
            cx, cy = cur
            for dx, dy in N8:
                nx, ny = cx + dx, cy + dy
                if not self.inb(nx, ny):
                    continue
                diag = dx != 0 and dy != 0
                step = 1.414 if diag else 1.0
                wet = self.water[ny][nx] != 0
                cur_wet = self.water[cy][cx] != 0
                if wet or cur_wet:
                    if diag:
                        continue  # bridges are straight: no diagonal steps on or off the water
                    if wet and self.water[ny][nx] in (2, 3):
                        continue  # no bridges over the sea or the lake
                    if wet and not bridge_ok:
                        continue
                sp = self.special[ny][nx]
                if sp in ("site", "plaza") and (nx, ny) != b:
                    step += 30
                dh = abs(self.h[ny][nx] - self.h[cy][cx])
                c = step + slope_k * dh * (4 if rail else 1)
                if wet:
                    c += 1.0 if (self.road[ny][nx] >= 3 or (rail and self.rail[ny][nx])) else water_cost
                if not rail and self.road[ny][nx] >= 2:
                    c *= reuse
                if rail and self.rail[ny][nx]:
                    c *= 0.3
                ng = g[cur] + c
                if ng < g.get((nx, ny), INF):
                    g[(nx, ny)] = ng
                    came[(nx, ny)] = cur
                    hdist = math.dist((nx, ny), b) * 0.9
                    heapq.heappush(pq, (ng + hdist, (nx, ny)))
        path = [b]
        while path[-1] != a and path[-1] in came:
            path.append(came[path[-1]])
        path.reverse()
        return path

    def lay_road(self, path, cls):
        for (x, y) in path:
            cells = [(x, y)]
            if cls >= 3:
                cells += [(x + 1, y), (x, y + 1)]
            for (i, j) in cells:
                if self.inb(i, j) and self.special[j][i] not in ("plaza",):
                    if self.water[j][i] in (2, 3):
                        continue
                    self.road[j][i] = max(self.road[j][i], cls)

    def lay_rail(self, path):
        for (x, y) in path:
            if self.inb(x, y):
                self.rail[y][x] = True
                if self.road[y][x] == 2:
                    self.road[y][x] = 0

    def arterial(self, a, b, **kw):
        p = self.route(a, b, 3, **kw)
        self.lay_road(p, 3)
        return p

    # -- land components (rivers and the sea split the land; creeks do not)
    def components(self):
        comp = [[-1] * self.W for _ in range(self.H)]
        n = 0
        for y in range(self.H):
            for x in range(self.W):
                if comp[y][x] >= 0 or self.water[y][x] not in (0, 4):
                    continue
                q = [(x, y)]
                comp[y][x] = n
                while q:
                    cx, cy = q.pop()
                    for dx, dy in N4:
                        nx, ny = cx + dx, cy + dy
                        if self.inb(nx, ny) and comp[ny][nx] < 0 and self.water[ny][nx] in (0, 4):
                            comp[ny][nx] = n
                            q.append((nx, ny))
                n += 1
        return comp

    # -- districts
    def add_district(self, name, dtype, seed, weight=1.0, penalty=None, cross=12.0, **params):
        self.districts.append(dict(name=name, type=dtype, seed=seed, weight=weight, penalty=penalty, cross=cross, **params))

    def assign_districts(self):
        comp = self.components()
        for d in self.districts:
            sx, sy = d["seed"]
            d["comp"] = comp[sy][sx]
        for y in range(self.H):
            for x in range(self.W):
                if self.water[y][x] not in (0, 4):
                    continue
                best, bi = None, -1
                for i, d in enumerate(self.districts):
                    s = math.dist((x, y), d["seed"]) / d["weight"]
                    if d["penalty"]:
                        s += d["penalty"](x, y)
                    if comp[y][x] != d["comp"]:
                        s += d["cross"]
                    if best is None or s < best:
                        best, bi = s, i
                self.dist[y][x] = bi
        ext = getattr(self, "extent", None)
        rural = [i for i, d in enumerate(self.districts) if d["type"] == "rural"]
        if ext and rural:
            cx, cy, ex, ey = ext
            nz = Noise(self.seed + 99, 9)
            for y in range(self.H):
                for x in range(self.W):
                    if self.dist[y][x] < 0 or self.districts[self.dist[y][x]]["type"] == "rural":
                        continue
                    e = ((x - cx) / ex) ** 2 + ((y - cy) / ey) ** 2
                    if e > 0.8 + 0.45 * nz.at(x, y):
                        self.dist[y][x] = min(rural, key=lambda i: math.dist((x, y), self.districts[i]["seed"]))

    def water_distance(self, kinds=(1,)):
        INF = 10**9
        dd = [[INF] * self.W for _ in range(self.H)]
        q = [(x, y) for y in range(self.H) for x in range(self.W) if self.water[y][x] in kinds]
        for x, y in q:
            dd[y][x] = 0
        head = 0
        while head < len(q):
            x, y = q[head]
            head += 1
            for dx, dy in N4:
                nx, ny = x + dx, y + dy
                if self.inb(nx, ny) and dd[ny][nx] > dd[y][x] + 1:
                    dd[ny][nx] = dd[y][x] + 1
                    q.append((nx, ny))
        return dd

    # -- streets by pattern
    def streets(self):
        for i, d in enumerate(self.districts):
            pat = d.get("pattern")
            r = random.Random(self.seed * 31 + i)
            if pat == "organic":
                self._organic(i, d, r)
            elif pat == "grid":
                self._grid(i, d, d.get("gx", 9), d.get("gy", 6), d.get("ox", 0), d.get("oy", 0), r, jitter=0)
            elif pat == "yards":
                self._grid(i, d, d.get("gx", 16), d.get("gy", 12), d.get("ox", 0), d.get("oy", 0), r, jitter=0)
            elif pat == "curvy":
                self._curvy(i, d, r)
            elif pat == "contour":
                self._contour(i, d, r)
            elif pat == "quay":
                self._quay(i, d, r)
            elif pat == "rural":
                self._rural(i, d, r)

    def _ok(self, i, x, y):
        return self.inb(x, y) and self.dist[y][x] == i and self.water[y][x] == 0 and self.special[y][x] is None and not self.rail[y][x]

    def _put(self, i, x, y, cls=2):
        if self._ok(i, x, y) and self.road[y][x] == 0:
            self.road[y][x] = cls

    def _grid(self, i, d, gx, gy, ox, oy, r, jitter=0):
        for y in range(self.H):
            for x in range(self.W):
                if (x - ox) % gx == 0 or (y - oy) % gy == 0:
                    self._put(i, x, y)

    def _organic(self, i, d, r):
        # Irregular small blocks: jittered lines that bend every few tiles.
        gx, gy = d.get("gx", 7), d.get("gy", 5)
        cols = {}
        for k in range(-2, self.W // gx + 3):
            off = 0
            for y in range(self.H):
                if y % 4 == 0:
                    off = max(-2, min(2, off + r.choice([-1, 0, 0, 1])))
                cols[(k, y)] = k * gx + off
        rows = {}
        for k in range(-2, self.H // gy + 3):
            off = 0
            for x in range(self.W):
                if x % 5 == 0:
                    off = max(-1, min(1, off + r.choice([-1, 0, 0, 1])))
                rows[(k, x)] = k * gy + off
        for (k, y), x in cols.items():
            if r.random() < 0.9:
                self._put(i, x, y)
        for (k, x), y in rows.items():
            self._put(i, x, y)

    def _curvy(self, i, d, r):
        cx, cy = d.get("centre", d["seed"])
        per = d.get("period", 7)
        spokes = [r.uniform(0, 2 * math.pi) for _ in range(d.get("spokes", 5))]
        for y in range(self.H):
            for x in range(self.W):
                if self.dist[y][x] != i:
                    continue
                rr = math.dist((x, y), (cx, cy))
                if abs((rr % per) - per / 2) < 0.55:
                    self._put(i, x, y)
                a = math.atan2(y - cy, x - cx)
                for s in spokes:
                    da = abs((a - s + math.pi) % (2 * math.pi) - math.pi)
                    if da * rr < 0.6:
                        self._put(i, x, y)
        # a few cul-de-sacs: short stubs off the rings
        for _ in range(d.get("culs", 6)):
            a = r.uniform(0, 2 * math.pi)
            ring = r.randint(1, 4)
            rr0 = ring * per + per / 2
            for t in range(0, 4):
                rr = rr0 + t
                x, y = int(round(cx + rr * math.cos(a))), int(round(cy + rr * math.sin(a)))
                self._put(i, x, y)

    def _contour(self, i, d, r):
        dh = d.get("dh", 12.0)
        gx = d.get("gx", 14)
        for y in range(self.H):
            for x in range(self.W):
                if self.dist[y][x] != i:
                    continue
                hh = self.h[y][x]
                if int(hh // dh) != int(self.h[y][min(self.W - 1, x + 1)] // dh) or int(hh // dh) != int(self.h[min(self.H - 1, y + 1)][x] // dh):
                    self._put(i, x, y)
        # steep connecting lanes (often steps) down the slope
        for x0 in range(d.get("ox", 3), self.W, gx):
            for y in range(self.H):
                if self.dist[y][x0] == i:
                    self._put(i, x0, y, 1)

    def _quay(self, i, d, r):
        # distance to the sea, BFS
        INF = 10**9
        dd = [[INF] * self.W for _ in range(self.H)]
        q = []
        for y in range(self.H):
            for x in range(self.W):
                if self.water[y][x] == 2:
                    dd[y][x] = 0
                    q.append((x, y))
        head = 0
        while head < len(q):
            x, y = q[head]
            head += 1
            for dx, dy in N4:
                nx, ny = x + dx, y + dy
                if self.inb(nx, ny) and dd[ny][nx] > dd[y][x] + 1:
                    dd[ny][nx] = dd[y][x] + 1
                    q.append((nx, ny))
        for y in range(self.H):
            for x in range(self.W):
                if self.dist[y][x] != i or self.water[y][x]:
                    continue
                if dd[y][x] <= 2:
                    if self.special[y][x] is None:
                        self.special[y][x] = "quay"
                elif dd[y][x] in (3, 10, 17):
                    self._put(i, x, y)
                elif x % 9 == 0 and dd[y][x] < 18:
                    self._put(i, x, y)

    def _rural(self, i, d, r):
        pts = [(r.uniform(0, self.W), r.uniform(0, self.H)) for _ in range(max(12, self.W * self.H // 160))]
        for y in range(self.H):
            for x in range(self.W):
                if self.dist[y][x] != i or self.water[y][x] or self.special[y][x] is not None:
                    continue
                k = min(range(len(pts)), key=lambda n: (pts[n][0] - x) ** 2 + (pts[n][1] - y) ** 2)
                self.field[y][x] = k

    # -- buildings
    def buildings(self):
        for y in range(self.H):
            for x in range(self.W):
                i = self.dist[y][x]
                if i < 0 or not self.land(x, y) or self.road[y][x] or self.rail[y][x] or self.special[y][x] is not None:
                    continue
                d = self.districts[i]
                dens = d.get("density", 0.6)
                if self.field[y][x] >= 0:
                    dens = d.get("farm_density", 0.0)
                adj = any(self.inb(x + dx, y + dy) and self.road[y + dy][x + dx] >= 2 for dx, dy in N4)
                adj_lane = any(self.inb(x + dx, y + dy) and self.road[y + dy][x + dx] >= 1 for dx, dy in N4)
                if d.get("slope_max") and self._slope(x, y) > d["slope_max"]:
                    continue
                if (adj or (adj_lane and d["type"] in ("hillside", "pit_rows", "rural"))) and self.rng.random() < dens:
                    self.bld[y][x] = self._roof(d)
        # second row for dense districts
        for y in range(self.H):
            for x in range(self.W):
                i = self.dist[y][x]
                if i < 0 or self.bld[y][x] or not self.land(x, y) or self.road[y][x] or self.rail[y][x] or self.special[y][x] is not None:
                    continue
                d = self.districts[i]
                if d.get("rows", 1) < 2:
                    continue
                if any(self.inb(x + dx, y + dy) and self.bld[y + dy][x + dx] for dx, dy in N4) and self.rng.random() < d.get("density", 0.6) * 0.8:
                    self.bld[y][x] = self._roof(d)

    def _roof(self, d):
        base = DISTRICT_STYLE[d["type"]][1]
        return shade(base, self.rng.uniform(0.88, 1.1))

    def _slope(self, x, y):
        s = 0.0
        for dx, dy in N4:
            if self.inb(x + dx, y + dy):
                s = max(s, abs(self.h[y + dy][x + dx] - self.h[y][x]))
        return s

    def landmark(self, x, y, w, h, colour, label=None, icon=None, label_dx=0, label_dy=-1, kind="place"):
        for j in range(y, y + h):
            for i in range(x, x + w):
                if self.inb(i, j):
                    self.bld[j][i] = colour
                    self.special[j][i] = "landmark"
        if icon:
            self.icons.append((icon, x + w / 2, y + h / 2))
        if label:
            self.labels.append((x + w / 2 + label_dx, y + label_dy, label, kind))

    def mark_flood(self, pred):
        for y in range(self.H):
            for x in range(self.W):
                if self.land(x, y) and pred(x, y):
                    self.flood[y][x] = True

    def mark_reserved(self, pred, clear=True):
        for y in range(self.H):
            for x in range(self.W):
                if self.land(x, y) and pred(x, y) and not self.rail[y][x] and self.road[y][x] < 3:
                    self.reserved[y][x] = True
                    if clear:
                        self.bld[y][x] = None
                        if self.road[y][x] == 2:
                            self.road[y][x] = 0

    def mark_derelict(self, pred, share=0.55):
        for y in range(self.H):
            for x in range(self.W):
                if self.bld[y][x] and pred(x, y) and self.special[y][x] != "landmark" and self.rng.random() < share:
                    self.derelict[y][x] = True

    def trees(self, pred, density=0.8, kind="wood"):
        for y in range(self.H):
            for x in range(self.W):
                if self.land(x, y) and not self.road[y][x] and not self.rail[y][x] and not self.bld[y][x] and self.special[y][x] is None and pred(x, y):
                    if self.rng.random() < density:
                        self.special[y][x] = kind

    # ------------------------------------------------------------ render
    def render(self, T, title, subtitle, legend_extra=(), legend=True, notes=()):
        W, H = self.W, self.H
        LEG = 300 if legend else 0
        TOP = 54
        img = Image.new("RGB", (W * T + LEG, H * T + TOP + 8), (250, 248, 242))
        dr = ImageDraw.Draw(img)
        ox, oy = 0, TOP
        hmin = min(min(r) for r in self.h)
        hmax = max(max(r) for r in self.h)
        comp_sea = [[0] * W for _ in range(H)]
        for y in range(H):
            for x in range(W):
                X, Y = ox + x * T, oy + y * T
                w = self.water[y][x]
                if w:
                    if w == 2:
                        # deeper away from the shore
                        near = any(self.land(x + dx, y + dy) for dx in range(-3, 4) for dy in range(-3, 4))
                        c = SEA if near else SEA_DEEP
                    elif w == 3:
                        c = LAKE
                    else:
                        c = WATER
                    dr.rectangle([X, Y, X + T - 1, Y + T - 1], fill=c)
                    continue
                t = (self.h[y][x] - hmin) / max(1e-6, hmax - hmin)
                base = mixc((206, 216, 168), (188, 180, 150), t)
                i = self.dist[y][x]
                if i >= 0:
                    dt = self.districts[i]["type"]
                    base = mixc(base, DISTRICT_STYLE[dt][0], 0.55 if dt != "rural" else 0.25)
                if self.field[y][x] >= 0:
                    base = FIELD_COLOURS[self.field[y][x] % len(FIELD_COLOURS)]
                sp = self.special[y][x]
                if sp == "plaza":
                    base = PLAZA
                elif sp == "park":
                    base = PARK
                elif sp == "wood":
                    base = WOOD
                elif sp == "sand":
                    base = SAND
                elif sp == "quay":
                    base = QUAY
                elif sp == "yard":
                    base = YARD
                elif sp == "spoil":
                    base = SPOIL
                # hillshade from the upper left
                gx = self.h[y][min(W - 1, x + 1)] - self.h[y][max(0, x - 1)]
                gy = self.h[min(H - 1, y + 1)][x] - self.h[max(0, y - 1)][x]
                f = 1.0 - (gx + gy) * 0.012
                base = shade(base, max(0.82, min(1.12, f)))
                dr.rectangle([X, Y, X + T - 1, Y + T - 1], fill=base)
                if sp == "wood" and (x * 7 + y * 3) % 2 == 0:
                    dr.ellipse([X + 1, Y + 1, X + T - 2, Y + T - 2], fill=WOOD_DOT)
                if sp == "park" and (x * 5 + y * 3) % 4 == 0:
                    dr.ellipse([X + 1, Y + 1, X + T - 2, Y + T - 2], fill=(110, 160, 96))
                if self.field[y][x] >= 0 and self.special[y][x] is None and not self.road[y][x]:
                    # hedges between fields
                    for dx, dy in ((1, 0), (0, 1)):
                        nx, ny = x + dx, y + dy
                        if self.inb(nx, ny) and self.field[ny][nx] >= 0 and self.field[ny][nx] != self.field[y][x]:
                            if dx:
                                dr.line([X + T - 1, Y, X + T - 1, Y + T - 1], fill=(120, 150, 90))
                            else:
                                dr.line([X, Y + T - 1, X + T - 1, Y + T - 1], fill=(120, 150, 90))
                    if (y % 2 == 0) and self.field[y][x] % 2 == 0:
                        dr.line([X, Y + T // 2, X + T - 1, Y + T // 2], fill=shade(base, 0.92))
        # contours
        ci = self.contour
        for y in range(H):
            for x in range(W):
                if not self.land(x, y) or self.road[y][x] or self.bld[y][x]:
                    continue
                X, Y = ox + x * T, oy + y * T
                k = math.floor(self.h[y][x] / ci)
                if x + 1 < W and self.land(x + 1, y) and math.floor(self.h[y][x + 1] / ci) != k:
                    dr.line([X + T - 1, Y, X + T - 1, Y + T - 1], fill=(150, 140, 110))
                if y + 1 < H and self.land(x, y + 1) and math.floor(self.h[y + 1][x] / ci) != k:
                    dr.line([X, Y + T - 1, X + T - 1, Y + T - 1], fill=(150, 140, 110))
        # banks
        for y in range(H):
            for x in range(W):
                if self.water[y][x] in (1, 4, 3, 2):
                    X, Y = ox + x * T, oy + y * T
                    for dx, dy in N4:
                        if self.land(x + dx, y + dy) and not self.road[y + dy][x + dx]:
                            if dx == 1:
                                dr.line([X + T - 1, Y, X + T - 1, Y + T - 1], fill=(56, 96, 140))
                            if dx == -1:
                                dr.line([X, Y, X, Y + T - 1], fill=(56, 96, 140))
                            if dy == 1:
                                dr.line([X, Y + T - 1, X + T - 1, Y + T - 1], fill=(56, 96, 140))
                            if dy == -1:
                                dr.line([X, Y, X + T - 1, Y], fill=(56, 96, 140))
        # reserved land: hatch
        for y in range(H):
            for x in range(W):
                if self.reserved[y][x]:
                    X, Y = ox + x * T, oy + y * T
                    for k in range(0, T):
                        if (X + Y + k) % 6 == 0:
                            dr.point((X + k, Y + (T - 1 - k)), fill=RESERVED)
                    for dx, dy in N4:
                        if self.inb(x + dx, y + dy) and not self.reserved[y + dy][x + dx] and (x + y) % 2 == 0:
                            if dx == 1:
                                dr.line([X + T - 1, Y, X + T - 1, Y + T - 1], fill=RESERVED)
                            if dx == -1:
                                dr.line([X, Y, X, Y + T - 1], fill=RESERVED)
                            if dy == 1:
                                dr.line([X, Y + T - 1, X + T - 1, Y + T - 1], fill=RESERVED)
                            if dy == -1:
                                dr.line([X, Y, X + T - 1, Y], fill=RESERVED)
        # roads
        for y in range(H):
            for x in range(W):
                c = self.road[y][x]
                if not c:
                    continue
                X, Y = ox + x * T, oy + y * T
                wet = self.water[y][x] != 0
                if wet:
                    fill, edge = BRIDGE, BRIDGE_EDGE
                elif c == 3:
                    fill, edge = ARTERIAL, ARTERIAL_EDGE
                elif c == 2:
                    fill, edge = STREET, STREET_EDGE
                else:
                    fill, edge = LANE, shade(LANE, 0.75)
                dr.rectangle([X, Y, X + T - 1, Y + T - 1], fill=fill)
                for dx, dy in N4:
                    nx, ny = x + dx, y + dy
                    other = self.road[ny][nx] if self.inb(nx, ny) else 0
                    if other == 0:
                        if dx == 1:
                            dr.line([X + T - 1, Y, X + T - 1, Y + T - 1], fill=edge)
                        if dx == -1:
                            dr.line([X, Y, X, Y + T - 1], fill=edge)
                        if dy == 1:
                            dr.line([X, Y + T - 1, X + T - 1, Y + T - 1], fill=edge)
                        if dy == -1:
                            dr.line([X, Y, X + T - 1, Y], fill=edge)
        # rail
        for y in range(H):
            for x in range(W):
                if self.rail[y][x]:
                    X, Y = ox + x * T, oy + y * T
                    cx, cy = X + T // 2, Y + T // 2
                    for dx, dy in N8:
                        if self.inb(x + dx, y + dy) and self.rail[y + dy][x + dx]:
                            dr.line([cx, cy, cx + dx * T // 2, cy + dy * T // 2], fill=RAIL, width=2)
                    dr.line([cx - 2, cy - 2, cx + 2, cy + 2], fill=(120, 110, 100))
        # buildings
        for y in range(H):
            for x in range(W):
                c = self.bld[y][x]
                if not c:
                    continue
                X, Y = ox + x * T, oy + y * T
                if self.derelict[y][x]:
                    c = (150, 148, 146)
                dr.rectangle([X, Y, X + T - 1, Y + T - 1], fill=c)
                # wall face: the south side darker (three-quarter view hint)
                if not (self.inb(x, y + 1) and self.bld[y + 1][x]):
                    dr.rectangle([X, Y + T - max(2, T // 3), X + T - 1, Y + T - 1], fill=shade(c, 0.72))
                if not (self.inb(x + 1, y) and self.bld[y][x + 1]):
                    dr.line([X + T - 1, Y, X + T - 1, Y + T - 1], fill=shade(c, 0.6))
                if self.derelict[y][x]:
                    dr.line([X + 1, Y + 1, X + T - 2, Y + T - 2], fill=(90, 88, 86))
                    dr.line([X + T - 2, Y + 1, X + 1, Y + T - 2], fill=(90, 88, 86))
        # flood-prone hatch
        for y in range(H):
            for x in range(W):
                if self.flood[y][x]:
                    X, Y = ox + x * T, oy + y * T
                    for k in range(T):
                        for j in range(T):
                            if (k - j + X - Y) % 4 == 0 and (self.bld[y][x] is None or (k + j) % 2 == 0):
                                dr.point((X + k, Y + j), fill=(40, 110, 200))
        # stairs
        for (x0, y0, x1, y1) in self.steps:
            n = int(max(abs(x1 - x0), abs(y1 - y0)) * 2)
            for s in range(n + 1):
                t = s / max(1, n)
                px, py = ox + (lerp(x0, x1, t) + 0.5) * T, oy + (lerp(y0, y1, t) + 0.5) * T
                dr.line([px - 2, py, px + 2, py], fill=(110, 90, 70))
        # icons
        for (kind, x, y) in self.icons:
            self._icon(dr, kind, ox + x * T, oy + y * T, T)
        # labels
        for (x, y, text, kind) in self.labels:
            px, py = ox + x * T, oy + y * T
            if kind == "district":
                f = font(12, bold=True, italic=True)
                fill = (60, 56, 70)
            elif kind == "hidden":
                f = font(10, italic=True)
                fill = HIDDEN
            elif kind == "bridge":
                f = font(10, italic=True)
                fill = (90, 64, 40)
            elif kind == "water":
                f = font(11, italic=True)
                fill = (40, 80, 140)
            elif kind == "edge":
                f = font(10)
                fill = (110, 100, 90)
            else:
                f = font(10)
                fill = TEXT
            bb = dr.textbbox((0, 0), text, font=f)
            tw = bb[2] - bb[0]
            dr.text((px - tw / 2, py - 12), text, font=f, fill=fill, stroke_width=2, stroke_fill=(252, 250, 244))
        # title
        dr.text((10, 6), title, font=font(17, bold=True), fill=TEXT)
        dr.text((10, 30), subtitle, font=font(11), fill=(80, 76, 86))
        if legend:
            sk = "SKETCH: illustration of the design, not the generator"
            f = font(10, italic=True)
            bb = dr.textbbox((0, 0), sk, font=f)
            dr.text((W * T + LEG - (bb[2] - bb[0]) - 10, 8), sk, font=f, fill=(170, 60, 60))
        if legend:
            self._legend(dr, W * T + 14, TOP + 4, legend_extra, notes)
        # scale bar: 10 tiles
        sx, sy = 10, oy + H * T - 14
        dr.rectangle([sx - 3, sy - 4, sx + 10 * T + 70, sy + 9], fill=(250, 248, 242))
        dr.line([sx, sy + 3, sx + 10 * T, sy + 3], fill=TEXT, width=2)
        dr.text((sx + 10 * T + 5, sy - 3), "10 tiles", font=font(9), fill=TEXT)
        return img

    def _icon(self, dr, kind, x, y, T):
        s = max(5, T)
        if kind == "hall":
            dr.polygon([(x - s, y - s * 0.6), (x, y - s * 1.4), (x + s, y - s * 0.6)], fill=(240, 228, 200), outline=(80, 60, 50))
        elif kind == "church":
            dr.rectangle([x - 1, y - s * 2.2, x + 1, y - s * 0.3], fill=(70, 60, 60))
            dr.line([x - 3, y - s * 1.8, x + 3, y - s * 1.8], fill=(70, 60, 60), width=2)
        elif kind == "chimney":
            dr.rectangle([x - 1, y - s * 2.0, x + 2, y], fill=(80, 70, 70))
            for k in range(3):
                dr.ellipse([x + 2 + k * 4, y - s * 2.4 - k * 3, x + 6 + k * 5, y - s * 2.0 - k * 3], fill=(170, 170, 176))
        elif kind == "headframe":
            dr.line([x - s, y + s * 0.6, x, y - s * 1.6], fill=(60, 50, 50), width=2)
            dr.line([x + s, y + s * 0.6, x, y - s * 1.6], fill=(60, 50, 50), width=2)
            dr.ellipse([x - 4, y - s * 1.6 - 4, x + 4, y - s * 1.6 + 4], outline=(60, 50, 50), width=2)
        elif kind == "stalls":
            cols = [(214, 80, 70), (240, 190, 60), (80, 150, 210), (90, 170, 110)]
            k = 0
            for j in range(2):
                for i in range(4):
                    cx, cy = x - s * 1.8 + i * s * 1.2, y - s * 0.5 + j * s * 1.1
                    dr.rectangle([cx, cy, cx + s * 0.8, cy + s * 0.6], fill=cols[k % 4], outline=(90, 70, 60))
                    k += 1
        elif kind == "station":
            dr.rectangle([x - s, y - s * 0.5, x + s, y + s * 0.5], fill=(200, 80, 60), outline=(60, 40, 40))
        elif kind == "crane":
            dr.line([x, y + s, x, y - s * 1.6], fill=(220, 150, 40), width=2)
            dr.line([x - s * 0.4, y - s * 1.6, x + s * 1.6, y - s * 1.6], fill=(220, 150, 40), width=2)
        elif kind == "lighthouse":
            dr.polygon([(x - 3, y + 4), (x + 3, y + 4), (x + 1, y - s * 1.6), (x - 1, y - s * 1.6)], fill=(240, 240, 240), outline=(170, 50, 50))
        elif kind == "hidden":
            r = s * 0.9
            for k in range(0, 360, 40):
                a0 = math.radians(k)
                a1 = math.radians(k + 20)
                dr.line([x + r * math.cos(a0), y + r * math.sin(a0), x + r * math.cos(a1), y + r * math.sin(a1)], fill=HIDDEN, width=2)
            dr.text((x - 3, y - 7), "?", font=font(10, bold=True), fill=HIDDEN)
        elif kind == "wheel":
            dr.ellipse([x - s * 0.7, y - s * 0.7, x + s * 0.7, y + s * 0.7], outline=(80, 60, 40), width=2)
        elif kind == "tree":
            dr.ellipse([x - s, y - s, x + s, y + s], fill=(90, 150, 90))
        elif kind == "anchor":
            dr.ellipse([x - 3, y - 3, x + 3, y + 3], fill=(40, 60, 120))
        elif kind == "dot":
            dr.ellipse([x - 3, y - 3, x + 3, y + 3], fill=(60, 50, 60))
        elif kind == "cross":
            dr.line([x, y - 5, x, y + 5], fill=(80, 60, 50), width=2)
            dr.line([x - 4, y - 1, x + 4, y - 1], fill=(80, 60, 50), width=2)

    def _legend(self, dr, x, y, extra, notes):
        dr.text((x, y), "Districts (data-driven types)", font=font(11, bold=True), fill=TEXT)
        y += 18
        seen = []
        for d in self.districts:
            t = d["type"]
            if t in seen:
                continue
            seen.append(t)
            g, roof, name = DISTRICT_STYLE[t]
            dr.rectangle([x, y + 1, x + 16, y + 12], fill=g, outline=(120, 110, 100))
            dr.rectangle([x + 9, y + 4, x + 14, y + 9], fill=roof)
            dr.text((x + 22, y), name, font=font(10), fill=TEXT)
            y += 16
        y += 6
        dr.text((x, y), "Map key", font=font(11, bold=True), fill=TEXT)
        y += 18
        rows = [
            ("road3", "Arterial (to a neighbouring town)"),
            ("road2", "Street"),
            ("road1", "Lane, steps, footpath"),
            ("bridge", "Bridge (straight, at a narrow crossing)"),
            ("rail", "Railway"),
        ] + list(extra)
        for key, text in rows:
            if key == "road3":
                dr.rectangle([x, y + 2, x + 16, y + 10], fill=ARTERIAL, outline=ARTERIAL_EDGE)
            elif key == "road2":
                dr.rectangle([x, y + 4, x + 16, y + 8], fill=STREET, outline=STREET_EDGE)
            elif key == "road1":
                dr.rectangle([x, y + 5, x + 16, y + 7], fill=LANE)
            elif key == "bridge":
                dr.rectangle([x, y, x + 16, y + 12], fill=WATER)
                dr.rectangle([x + 5, y, x + 11, y + 12], fill=BRIDGE, outline=BRIDGE_EDGE)
            elif key == "rail":
                dr.line([x, y + 6, x + 16, y + 6], fill=RAIL, width=2)
                for k in range(0, 17, 4):
                    dr.line([x + k, y + 3, x + k, y + 9], fill=(120, 110, 100))
            elif key == "flood":
                dr.rectangle([x, y, x + 16, y + 12], fill=(214, 222, 236))
                for k in range(0, 17, 5):
                    dr.line([x + k, y, x + k + 6, y + 12], fill=(40, 110, 200))
            elif key == "reserved":
                dr.rectangle([x, y, x + 16, y + 12], outline=RESERVED)
                for k in range(0, 17, 5):
                    dr.line([x + k, y + 12, x + k + 6, y], fill=RESERVED)
            elif key == "derelict":
                dr.rectangle([x + 3, y + 1, x + 13, y + 11], fill=(150, 148, 146))
                dr.line([x + 3, y + 1, x + 13, y + 11], fill=(90, 88, 86))
                dr.line([x + 13, y + 1, x + 3, y + 11], fill=(90, 88, 86))
            elif key == "hidden":
                self._icon(dr, "hidden", x + 8, y + 6, 6)
            elif key == "wood":
                dr.rectangle([x, y, x + 16, y + 12], fill=WOOD)
                dr.ellipse([x + 3, y + 3, x + 9, y + 9], fill=WOOD_DOT)
            elif key == "contour":
                dr.line([x, y + 4, x + 16, y + 4], fill=(150, 140, 110))
                dr.line([x, y + 9, x + 16, y + 9], fill=(150, 140, 110))
            elif key == "fields":
                dr.rectangle([x, y, x + 8, y + 12], fill=FIELD_COLOURS[0])
                dr.rectangle([x + 8, y, x + 16, y + 12], fill=FIELD_COLOURS[1])
                dr.line([x + 8, y, x + 8, y + 12], fill=(120, 150, 90))
            elif key == "spoil":
                dr.rectangle([x, y, x + 16, y + 12], fill=SPOIL)
            elif key == "quay":
                dr.rectangle([x, y, x + 16, y + 12], fill=QUAY)
            elif key == "steps":
                for k in range(0, 13, 3):
                    dr.line([x + 4, y + k, x + 12, y + k], fill=(110, 90, 70))
            dr.text((x + 22, y), text, font=font(10), fill=TEXT)
            y += 16
        if notes:
            y += 8
            dr.text((x, y), "Inputs (engine view)", font=font(11, bold=True), fill=TEXT)
            y += 18
            for n in notes:
                dr.text((x, y), n, font=font(9), fill=(70, 66, 76))
                y += 13


# ---------------------------------------------------------------- scenarios


def river_town(out):
    """Marwell (seed 4242): pop 60k, river class 2, elevation 17 m, flood 0.60, steppe;
    3 roads and a rail link; manufacturing; minerals 0.45 (a small quarry)."""
    W, H = 132, 90
    t = Town(W, H, seed=11, contour=4.0)
    n1, n2 = Noise(5, 18), Noise(6, 7)
    t.set_heights(lambda x, y: 34 - y * 0.18 + x * 0.04 + n1.at(x, y) * 10 + n2.at(x, y) * 2.5)
    river = [(-2, 64), (18, 66), (36, 58), (50, 56), (62, 57), (76, 50), (92, 48), (108, 41), (122, 38), (134, 33)]
    t.paint_line(river, 4.2, 1, carve=60)
    creek = [(84, -2), (82, 12), (77, 24), (74, 36), (73, 46), (75, 50)]
    t.paint_line(creek, 1.2, 4, carve=20)
    # the square: on the terrace above the floodplain, by the narrowest crossing
    t.site(52, 36, 10, 6, "plaza")
    for x in range(52, 62):
        for y in range(36, 42):
            t.special[y][x] = "plaza"
    # landmarks' sites before the roads
    t.site(53, 31, 8, 4)  # town hall
    t.site(63, 37, 5, 4)  # council chamber
    t.site(37, 20, 4, 4)  # church on the rise
    # arterials: to the west, north, south (across the river) and east
    t.arterial((52, 39), (0, 34))
    t.arterial((57, 36), (60, 0))
    t.arterial((57, 41), (60, 89))
    t.arterial((61, 39), (131, 58))
    t.arterial((98, 37), (100, 60), water_cost=4)
    t.arterial((100, 60), (112, 89))
    # rail: from the west, a station north of the core, on to the east; a spur to the quarry
    rp = t.route((0, 24), (131, 18), 0, rail=True, slope_k=0.5)
    t.lay_rail(rp)
    t.site(46, 26, 6, 3)
    sp = t.route((96, 21), (114, 9), 0, rail=True, slope_k=0.3)
    t.lay_rail(sp)
    t.add_district("Old Town", "old_core", (57, 38), 1.0, pattern="organic", gx=6, gy=5, density=0.85, rows=2)
    t.add_district("Southbank", "riverside", (52, 72), 1.0, pattern="grid", gx=8, gy=5, ox=2, oy=1, density=0.8, rows=2)
    t.add_district("Millside", "industrial", (96, 46), 1.0, pattern="yards", gx=14, gy=11, ox=3, oy=2, density=0.5, penalty=lambda x, y: 0 if y > 25 else 20)
    t.add_district("Northfield", "suburb", (56, 13), 1.15, pattern="curvy", centre=(58, 12), period=7, spokes=5, culs=8, density=0.55)
    t.add_district("Westgate", "suburb", (18, 42), 1.0, pattern="curvy", centre=(20, 44), period=6, spokes=4, culs=6, density=0.55)
    t.add_district("Rural fringe", "rural", (120, 82), 0.42, pattern="rural", farm_density=0.012, penalty=lambda x, y: -18 if (x < 6 or x > 120 or y > 84 or y < 4) else 0)
    t.extent = (60, 42, 58, 40)
    t.assign_districts()
    wd = t.water_distance((1,))
    t.mark_flood(lambda x, y: wd[y][x] <= 7 and t.dist[y][x] >= 0 and t.districts[t.dist[y][x]]["name"] in ("Southbank", "Millside", "Rural fringe") and y > 40)
    t.streets()
    # creek-side park
    for y in range(H):
        for x in range(W):
            if t.land(x, y) and any(t.inb(x + dx, y + dy) and t.water[y + dy][x + dx] == 4 for dx in range(-2, 3) for dy in range(-2, 3)) and y < 46 and t.road[y][x] < 3:
                t.special[y][x] = "park"
                if t.road[y][x] == 2:
                    t.road[y][x] = 1
    t.buildings()
    t.trees(lambda x, y: t.dist[y][x] >= 0 and t.districts[t.dist[y][x]]["type"] == "rural" and Noise(9, 9).at(x, y) > 0.66, 0.9)
    t.landmark(53, 31, 8, 4, (226, 210, 180), "Town hall", "hall", label_dy=-1.2)
    t.landmark(63, 37, 5, 4, (214, 196, 170), "Council chamber", label_dx=4, label_dy=-0.4)
    t.icons.append(("stalls", 57, 39.5))
    t.labels.append((57, 43.3, "Market square (open-air market)", "place"))
    t.landmark(37, 20, 4, 4, (200, 190, 176), "St Ann's (on the rise)", "church", label_dy=-2.6)
    t.landmark(46, 26, 6, 3, (200, 90, 70), "Station", "station", label_dy=-0.6)
    t.landmark(100, 40, 7, 4, (120, 124, 140), "Works (by the water)", "chimney", label_dy=-2.4)
    t.landmark(88, 37, 4, 3, (150, 120, 110), "Union hall", label_dy=-0.3)
    t.landmark(80, 50, 4, 3, (170, 150, 120), "Old mill", "wheel", label_dx=2, label_dy=4.2)
    t.landmark(112, 6, 6, 4, (130, 120, 112), "Quarry", label_dy=-0.4)
    for y in range(9, 14):
        for x in range(117, 123):
            if t.land(x, y) and not t.rail[y][x]:
                t.special[y][x] = "spoil"
    t.landmark(50, 15, 5, 3, (196, 150, 110), "School", label_dy=-0.3)
    t.landmark(22, 38, 5, 3, (196, 150, 110), "Community centre", label_dy=-0.3)
    t.landmark(46, 68, 4, 3, (150, 110, 120), "Clinic", label_dy=-0.3)
    t.labels.append((79, 29, "Creek park", "place"))
    t.labels.append((52, 61.5, "Old Bridge (stone)", "bridge"))
    t.labels.append((107, 49.5, "Mill Bridge (iron)", "bridge"))
    t.labels.append((112, 39.5, "River Mar", "water"))
    t.icons.append(("hidden", 30, 16))
    t.labels.append((30, 13.5, "Lookout (hidden)", "hidden"))
    t.icons.append(("hidden", 24, 62))
    t.labels.append((24, 59.5, "Old boathouse (hidden)", "hidden"))
    for d in t.districts:
        sx, sy = d["seed"]
        if d["type"] != "rural":
            t.labels.append((sx, sy + 3.2, d["name"].upper(), "district"))
    t.labels.append((126, 86, "farms", "edge"))
    img = t.render(
        6,
        "Marwell: a river town (pop 60,400)",
        "Old core on the terrace by the narrowest crossing; riverside terraces on the floodplain; works and mill by the water; suburbs on higher ground.",
        legend_extra=[("flood", "Flood-prone (engine hazard exposure)"), ("contour", "Contours (4 m)"), ("fields", "Fields and hedges"), ("wood", "Woods"), ("spoil", "Spoil (quarry)"), ("hidden", "Hidden place (discovered in play)")],
        notes=[
            "seed 4242, municipality 28 (Marwell)",
            "class town · population 60,424",
            "5 neighbourhoods: core, industrial, 2 suburbs, rural",
            "river class 2 through the town (W to E)",
            "elevation 17 m · flood hazard 0.60",
            "links: 3 roads + rail",
            "leading sector: manufacturing",
            "minerals 0.45: a small quarry, not a pit",
        ],
    )
    img.save(out)


def mining_town(out):
    """Ashwick (seed 4242): pop 103k, 900 m, ruggedness 1.0, forest 0.6, river through the valley,
    a lake to the south-east, minerals 0.59, timber 0.52; regional capital."""
    W, H = 140, 96
    t = Town(W, H, seed=21, contour=14.0)
    n1, n2 = Noise(15, 16), Noise(16, 6)

    def rx(y):
        return 58 + 7 * math.sin(y / 17.0)

    def height(x, y):
        d = x - rx(y)
        side = 2.6 if d > 0 else 1.9
        return 880 + abs(d) ** 1.08 * side + n1.at(x, y) * 28 + n2.at(x, y) * 6 + (y * 0.25)

    t.set_heights(height)
    river = [(rx(y), y) for y in range(-2, 99, 6)]
    t.paint_line(river, 3.4, 1, carve=40)
    t.lake_blob(122, 86, 22, 12, 7)
    # the square on the valley floor, west bank
    for x in range(44, 53):
        for y in range(42, 47):
            t.special[y][x] = "plaza"
    t.site(44, 37, 9, 4)
    t.site(54, 42, 0, 0)
    t.arterial((48, 44), (rx(0) - 4, 0), slope_k=0.15)
    t.arterial((48, 46), (rx(95) + 2, 95), slope_k=0.15)
    t.arterial((52, 44), (139, 50), slope_k=0.12)
    t.arterial((44, 44), (0, 60), slope_k=0.12)
    rp = t.route((rx(95) + 6, 95), (rx(0) + 4, 0), 0, rail=True, slope_k=0.25)
    t.lay_rail(rp)
    t.site(67, 52, 6, 3)
    # the incline to the pit: a spur climbing east
    sp = t.route((66, 55), (112, 30), 0, rail=True, slope_k=0.12)
    t.lay_rail(sp)
    t.add_district("Old Town", "old_core", (48, 46), 1.0, pattern="organic", gx=6, gy=5, density=0.85, rows=2)
    t.add_district("Pit Rows", "pit_rows", (96, 38), 1.0, pattern="contour", dh=30.0, gx=15, ox=5, density=0.8, slope_max=16)
    t.add_district("Hillside", "hillside", (26, 34), 1.0, pattern="contour", dh=26.0, gx=14, ox=2, density=0.7, slope_max=14)
    t.add_district("Lakeview", "suburb", (98, 74), 1.0, pattern="curvy", centre=(104, 78), period=6, spokes=4, culs=5, density=0.5)
    t.add_district("Upper Town", "suburb", (30, 72), 1.0, pattern="curvy", centre=(28, 74), period=7, spokes=4, culs=5, density=0.5)
    t.add_district("Valley works", "industrial", (66, 14), 0.9, pattern="yards", gx=12, gy=10, density=0.45, penalty=lambda x, y: 0 if y < 36 else 60)
    t.add_district("Forest edge", "rural", (8, 6), 0.4, pattern="rural", farm_density=0.01, penalty=lambda x, y: -22 if (x < 8 or x > 130 or y < 4 or y > 90) else 0)
    t.extent = (66, 46, 66, 46)
    t.assign_districts()
    t.streets()
    t.buildings()
    # woods on the upper slopes
    t.trees(lambda x, y: t.h[y][x] > 975 + Noise(3, 5).at(x, y) * 30, 0.95)
    t.landmark(44, 37, 9, 4, (226, 210, 180), "Regional assembly + town hall", "hall", label_dy=-1.3)
    t.icons.append(("stalls", 48.5, 44.5))
    t.labels.append((61, 46.2, "Market square", "place"))
    t.landmark(67, 52, 6, 3, (200, 90, 70), "Station", "station", label_dx=6, label_dy=-0.4)
    t.landmark(110, 26, 6, 5, (110, 96, 92), "Ashwick Colliery", "headframe", label_dy=-3.3)
    for y in range(18, 27):
        for x in range(118, 128):
            if t.land(x, y) and math.dist((x, y), (123, 22)) < 5.2:
                t.special[y][x] = "spoil"
                t.bld[y][x] = None
                t.road[y][x] = 0
    t.labels.append((124, 17.4, "spoil heap", "edge"))
    t.landmark(88, 33, 4, 3, (150, 120, 110), "Union hall", label_dy=-0.3)
    t.landmark(68, 10, 7, 4, (150, 120, 90), "Sawmill (on the river)", "wheel", label_dy=-1.6)
    t.landmark(100, 68, 6, 4, (196, 170, 140), "Lake Hotel", label_dy=-0.3)
    t.landmark(22, 30, 5, 3, (196, 150, 110), "School", label_dy=-0.3)
    t.landmark(36, 52, 4, 4, (200, 190, 176), "Chapel", "church", label_dy=-2.5)
    t.labels.append((rx(78) - 4, 78, "River Ash", "water"))
    t.labels.append((124, 88, "Ash Water", "water"))
    t.icons.append(("hidden", 92, 30))
    t.labels.append((92, 27.5, "Old union archive (hidden)", "hidden"))
    t.icons.append(("hidden", 128, 8))
    t.labels.append((126, 5.5, "Disused incline (hidden)", "hidden"))
    t.steps.append((70, 46, 80, 40))
    for d in t.districts:
        sx, sy = d["seed"]
        if d["type"] != "rural":
            t.labels.append((sx, sy + 3.4, d["name"].upper(), "district"))
    img = t.render(
        6,
        "Ashwick: a mining town in the hills (pop 103,300)",
        "Valley-floor core; terraces follow the contours; the colliery at the edge with its headframe, spoil heap and rail incline; sawmill on the river.",
        legend_extra=[("contour", "Contours (14 m)"), ("wood", "Woods on the upper slopes"), ("spoil", "Spoil heap (grows as the seam is worked)"), ("steps", "Steps"), ("hidden", "Hidden place (discovered in play)")],
        notes=[
            "seed 4242, municipality 18 (Ashwick)",
            "class town · population 103,282",
            "6 neighbourhoods: core, industrial, 3 suburbs, rural",
            "elevation 901 m · ruggedness 1.00",
            "river through the centre; lake to the SE",
            "minerals 0.59 · timber 0.52 · forest 0.60",
            "seismic 0.56 · regional capital",
            "links: 5 roads + rail",
        ],
    )
    img.save(out)


def coastal_town(out):
    """Larkobridge (seed 4242): pop 135k, sea to the west, port, steppe scrub rising east,
    flood 0.26, storm 0.39, minerals 0.57; 8 neighbourhoods incl. waterfront."""
    W, H = 150, 100
    t = Town(W, H, seed=31, contour=12.0)
    n1, n2 = Noise(25, 14), Noise(26, 6)

    def shore(y):
        s = 30 + 6 * math.sin(y / 13.0)
        if 36 < y < 66:  # the harbour bay
            s += 14 * math.sin((y - 36) / 30 * math.pi)
        if y < 22:  # the headland
            s -= 14 * math.sin(max(0, (22 - y)) / 22 * math.pi / 2) * (1 if y > 4 else 0.3)
        return s

    t.set_heights(lambda x, y: max(0, (x - shore(y))) * 2.1 + n1.at(x, y) * 26 + n2.at(x, y) * 5 + (8 if y < 20 else 0))
    t.sea_where(lambda x, y: x < shore(y))
    # beach in the south
    for y in range(70, H):
        for x in range(W):
            if t.land(x, y) and x < shore(y) + 4:
                t.special[y][x] = "sand"
    # piers into the harbour
    for (py, ln) in ((42, 9), (48, 11), (54, 10), (60, 7)):
        x0 = int(shore(py))
        for x in range(x0 - ln, x0 + 1):
            for y in (py, py + 1):
                if t.inb(x, y):
                    t.water[y][x] = 0
                    t.special[y][x] = "quay"
    # the square on the slope above the harbour
    for x in range(58, 66):
        for y in range(45, 50):
            t.special[y][x] = "plaza"
    t.site(58, 40, 8, 4)
    t.arterial((62, 45), (149, 30), slope_k=0.12)
    t.arterial((65, 48), (149, 74), slope_k=0.12)
    t.arterial((58, 47), (int(shore(0)) + 8, 0), slope_k=0.1)
    t.arterial((60, 49), (40, 99), slope_k=0.1)
    rp = t.route((48, 99), (int(shore(0)) + 12, 0), 0, rail=True, slope_k=0.2)
    t.lay_rail(rp)
    sp = t.route((56, 80), (124, 88), 0, rail=True, slope_k=0.2)
    t.lay_rail(sp)
    t.add_district("Old Town", "old_core", (60, 46), 1.0, pattern="organic", gx=6, gy=4, density=0.85, rows=2)
    t.add_district("Harbour", "docks", (48, 52), 1.0, pattern="quay", density=0.7, penalty=lambda x, y: 0 if 32 < y < 70 else 30)
    t.add_district("North Yards", "industrial", (44, 28), 1.0, pattern="yards", gx=12, gy=9, density=0.5)
    t.add_district("Seaview", "suburb", (90, 24), 1.0, pattern="curvy", centre=(92, 22), period=7, spokes=4, culs=6, density=0.5)
    t.add_district("Hilltop", "suburb", (98, 52), 1.0, pattern="curvy", centre=(100, 52), period=6, spokes=5, culs=6, density=0.5)
    t.add_district("Strand", "terraces", (54, 82), 1.0, pattern="grid", gx=9, gy=5, ox=1, oy=2, density=0.75, rows=2)
    t.add_district("Upland farms", "rural", (140, 8), 0.4, pattern="rural", farm_density=0.012, penalty=lambda x, y: -20 if x > 128 else 0)
    t.add_district("Southmoor", "rural", (140, 96), 0.4, pattern="rural", farm_density=0.012, penalty=lambda x, y: -20 if x > 128 else 0)
    t.extent = (62, 50, 74, 52)
    t.assign_districts()
    t.streets()
    t.buildings()
    t.mark_flood(lambda x, y: t.dist[y][x] >= 0 and t.districts[t.dist[y][x]]["name"] in ("Harbour", "Strand") and t.h[y][x] < 11)
    t.landmark(58, 40, 8, 4, (226, 210, 180), "Town hall", "hall", label_dy=-1.2)
    t.icons.append(("stalls", 62, 47.5))
    t.labels.append((62, 51.4, "Fish market", "place"))
    for (cx, cy) in ((44, 47), (44, 53), (46, 59)):
        t.icons.append(("crane", cx, cy))
    t.labels.append((40, 37.2, "Port (quays, cranes)", "place"))
    t.icons.append(("lighthouse", 25, 10))
    t.labels.append((26, 6.5, "Lighthouse", "place"))
    t.landmark(46, 22, 8, 4, (120, 124, 140), "Cannery", "chimney", label_dy=-2.3)
    t.landmark(72, 36, 4, 4, (200, 190, 176), "Harbour church", "church", label_dy=-2.5)
    t.landmark(70, 52, 4, 3, (150, 120, 110), "Union hall", label_dx=3, label_dy=-0.3)
    t.landmark(120, 84, 6, 5, (110, 96, 92), "Pit (edge of town)", "headframe", label_dy=-2.6)
    for y in range(86, 94):
        for x in range(126, 136):
            if t.land(x, y) and math.dist((x, y), (131, 90)) < 4.2:
                t.special[y][x] = "spoil"
                t.bld[y][x] = None
    t.landmark(92, 46, 5, 3, (196, 150, 110), "School", label_dy=-0.3)
    t.labels.append((40, 90, "The Strand (beach promenade)", "place"))
    t.labels.append((14, 60, "Lark Sound", "water"))
    t.steps += [(64, 50, 52, 56), (66, 44, 72, 34)]
    t.icons.append(("hidden", 36, 66))
    t.labels.append((34, 63.5, "Smugglers' steps (hidden)", "hidden"))
    t.icons.append(("hidden", 56, 56))
    t.labels.append((56, 59.8, "Back room at the Anchor (hidden)", "hidden"))
    t.icons.append(("hidden", 112, 12))
    t.labels.append((112, 9.5, "Ridge lookout (hidden)", "hidden"))
    for d in t.districts:
        sx, sy = d["seed"]
        if d["type"] != "rural":
            t.labels.append((sx, sy + 3.4, d["name"].upper(), "district"))
    img = t.render(
        6,
        "Larkobridge: a hilly coastal port (pop 135,200)",
        "Harbour in the bay with quays and piers; the old town climbs the slope with steps; yards by the rail; suburbs up the hill; a pit at the edge.",
        legend_extra=[("quay", "Quays and piers"), ("flood", "Storm-surge prone (engine exposure)"), ("steps", "Steps down to the harbour"), ("contour", "Contours (12 m)"), ("hidden", "Hidden place (discovered in play)")],
        notes=[
            "seed 4242, municipality 15 (Larkobridge)",
            "class town · population 135,179 · port",
            "8 neighbourhoods: core, waterfront, industrial,",
            "  3 suburbs, 2 rural",
            "sea to the west · elevation 134 m",
            "fishing 0.61 · minerals 0.57",
            "flood 0.26 · storm 0.39",
            "links: 2 roads (+ coast rail shown)",
        ],
    )
    img.save(out)


def scaling(out):
    """A rural village (2 neighbourhoods) and a city (13 neighbourhoods), at the same tile scale."""
    T = 4
    # ---- village
    vw, vh = 70, 50
    v = Town(vw, vh, seed=41, contour=5.0)
    n1 = Noise(35, 12)
    v.set_heights(lambda x, y: 60 + n1.at(x, y) * 12 - y * 0.1)
    v.paint_line([(46, -2), (42, 14), (38, 26), (40, 38), (36, 52)], 1.2, 4, carve=10)
    for x in range(26, 32):
        for y in range(22, 27):
            v.special[y][x] = "park"
    v.arterial((29, 22), (30, 0))
    v.arterial((32, 24), (69, 30))
    v.arterial((26, 24), (0, 20))
    v.arterial((29, 27), (22, 49))
    v.add_district("Village", "village", (29, 24), 1.0, pattern="organic", gx=8, gy=6, density=0.6)
    v.add_district("Farms", "rural", (5, 45), 0.3, pattern="rural", farm_density=0.02)
    v.extent = (30, 24, 17, 12)
    v.assign_districts()
    v.streets()
    v.buildings()
    v.landmark(32, 19, 3, 3, (200, 190, 176), None, "church")
    v.labels.append((33.5, 17.0, "Church", "place"))
    v.icons.append(("cross", 29, 24.5))
    v.labels.append((29, 30.6, "Green, market cross", "place"))
    v.landmark(24, 19, 3, 2, (214, 196, 170), None)
    v.labels.append((22, 18.5, "Village hall", "place"))
    v.landmark(40, 31, 3, 2, (170, 150, 120), None, "wheel")
    v.labels.append((44, 35.7, "Mill + ford", "place"))
    v.icons.append(("hidden", 52, 8))
    v.labels.append((52, 5.6, "Old quarry pool (hidden)", "hidden"))
    vimg = v.render(T, "Village (pop 12,000)", "2 neighbourhoods: centre + farms", legend=False)
    # ---- city
    cw, ch = 190, 128
    c = Town(cw, ch, seed=51, contour=6.0)
    m1, m2 = Noise(45, 22), Noise(46, 8)
    c.set_heights(lambda x, y: 50 + m1.at(x, y) * 26 + m2.at(x, y) * 4 + max(0, 60 - y) * 0.2)
    c.paint_line([(-2, 84), (30, 80), (60, 70), (84, 72), (104, 64), (130, 66), (160, 56), (192, 54)], 5.0, 1, carve=40)
    for x in range(88, 98):
        for y in range(52, 58):
            c.special[y][x] = "plaza"
    c.site(88, 47, 10, 4)
    for dst in ((0, 50), (94, 0), (189, 30), (189, 100), (60, 127), (130, 127), (0, 110)):
        c.arterial((93, 55), dst, slope_k=0.1)
    # ring road
    ring = [(int(93 + 46 * math.cos(a / 24 * 2 * math.pi)), int(62 + 34 * math.sin(a / 24 * 2 * math.pi))) for a in range(25)]
    for a, b in zip(ring, ring[1:]):
        c.lay_road(c.route(a, b, 3, slope_k=0.05, water_cost=8), 3)
    rp = c.route((0, 30), (189, 76), 0, rail=True, slope_k=0.3)
    c.lay_rail(rp)
    c.site(70, 42, 7, 3)
    c.add_district("Old Town", "old_core", (93, 55), 1.0, pattern="organic", gx=6, gy=5, density=0.85, rows=2)
    inner = [(70, 52), (112, 50), (80, 86), (112, 82)]
    for k, s in enumerate(inner):
        c.add_district(["Westbury", "Eastholm", "Southbank", "Riverside"][k], ["terraces", "terraces", "riverside", "estate"][k], s, 1.0, pattern="grid", gx=8, gy=5, ox=k, oy=k, density=0.8, rows=2)
    c.add_district("University", "campus", (96, 32), 1.0, pattern="yards", gx=12, gy=8, ox=2, density=0.35)
    c.add_district("Docks and works", "industrial", (150, 62), 1.0, pattern="yards", gx=14, gy=10, density=0.45)
    subs = [(40, 32), (60, 20), (130, 22), (160, 34), (40, 100), (140, 104)]
    for k, s in enumerate(subs):
        c.add_district(["Northgate", "Hilldene", "Oakfield", "Greenway", "Southmead", "Parkside"][k], "suburb", s, 1.0, pattern="curvy", centre=s, period=7, spokes=4, culs=5, density=0.5)
    c.add_district("Fringe", "rural", (186, 124), 0.35, pattern="rural", farm_density=0.008, penalty=lambda x, y: -26 if (x < 10 or x > 178 or y < 6 or y > 120) else 0)
    c.extent = (95, 62, 98, 66)
    c.assign_districts()
    c.streets()
    c.buildings()
    c.mark_flood(lambda x, y: c.h[y][x] < 49 and c.dist[y][x] >= 0 and c.districts[c.dist[y][x]]["type"] in ("riverside", "terraces", "old_core"))
    # reserved estate land (fills as the population grows), decline in the old works
    c.mark_reserved(lambda x, y: (x - 175) ** 2 / 14**2 + (y - 18) ** 2 / 12**2 < 1)
    c.mark_reserved(lambda x, y: (x - 22) ** 2 / 12**2 + (y - 70) ** 2 / 10**2 < 1)
    c.mark_derelict(lambda x, y: c.dist[y][x] >= 0 and c.districts[c.dist[y][x]]["name"] == "Docks and works" and x > 150, 0.6)
    c.landmark(88, 47, 10, 4, (226, 210, 180), "Town hall + council", "hall", label_dy=-1.4)
    c.icons.append(("stalls", 93, 55))
    c.landmark(70, 42, 7, 3, (200, 90, 70), "Central station", "station", label_dy=-0.5)
    c.landmark(92, 26, 9, 5, (150, 120, 180), "University", label_dy=-0.4)
    c.landmark(160, 58, 8, 5, (150, 146, 150), "Closed works", "chimney", label_dy=-2.3)
    c.labels.append((175, 33, "reserved estate", "edge"))
    c.labels.append((22, 83, "reserved estate", "edge"))
    c.labels.append((168, 72, "derelict quarter", "edge"))
    c.icons.append(("hidden", 165, 66))
    c.labels.append((166, 69, "Disused rail yard (hidden)", "hidden"))
    for d in c.districts:
        sx, sy = d["seed"]
        if d["type"] != "rural":
            c.labels.append((sx, sy + 3.6, d["name"].upper(), "district"))
    cimg = c.render(T, "City (pop 420,000)", "13 neighbourhoods; ring road; reserved land; a declining quarter", legend=False)
    # ---- compose
    pad = 16
    Wt = vimg.width + cimg.width + pad * 3
    Ht = max(vimg.height, cimg.height) + 70
    out_img = Image.new("RGB", (Wt, Ht), (250, 248, 242))
    dr = ImageDraw.Draw(out_img)
    dr.text((pad, 8), "Population scaling: the same tile scale, size from population", font=font(17, bold=True), fill=TEXT)
    sk = "SKETCH: illustration of the design, not the generator"
    bb = dr.textbbox((0, 0), sk, font=font(10, italic=True))
    dr.text((Wt - (bb[2] - bb[0]) - pad, 10), sk, font=font(10, italic=True), fill=(170, 60, 60))
    dr.text((pad, 32), "Map size grows with log(population); districts = the engine's neighbourhoods; readability caps hold at every size.", font=font(11), fill=(80, 76, 86))
    out_img.paste(vimg, (pad, 60))
    out_img.paste(cimg, (pad * 2 + vimg.width, 60))
    y = 60 + vimg.height + 16
    rows = [
        "Village: 70 x 50 tiles, 2 districts, 1 crossroads,",
        "a green with a market cross, a ford and a mill.",
        "",
        "City: 190 x 128 tiles, 13 districts, 7 arterials",
        "and a ring road, a campus district, 2 bridges,",
        "reserved estates (hatched) that fill as the town",
        "grows, and a derelict quarter around closed works.",
    ]
    for r in rows:
        dr.text((pad, y), r, font=font(11), fill=TEXT)
        y += 15
    y += 10
    for key, text in (("reserved", "Reserved land (fills with growth)"), ("derelict", "Derelict / vacant (decline)"), ("flood", "Flood-prone"), ("hidden", "Hidden place")):
        x0 = pad
        if key == "reserved":
            dr.rectangle([x0, y, x0 + 16, y + 12], outline=RESERVED)
            for k in range(0, 17, 5):
                dr.line([x0 + k, y + 12, x0 + k + 6, y], fill=RESERVED)
        elif key == "derelict":
            dr.rectangle([x0 + 3, y + 1, x0 + 13, y + 11], fill=(150, 148, 146))
            dr.line([x0 + 3, y + 1, x0 + 13, y + 11], fill=(90, 88, 86))
            dr.line([x0 + 13, y + 1, x0 + 3, y + 11], fill=(90, 88, 86))
        elif key == "flood":
            dr.rectangle([x0, y, x0 + 16, y + 12], fill=(214, 222, 236))
            for k in range(0, 17, 5):
                dr.line([x0 + k, y, x0 + k + 6, y + 12], fill=(40, 110, 200))
        else:
            c._icon(dr, "hidden", x0 + 8, y + 6, 6)
        dr.text((x0 + 22, y), text, font=font(10), fill=TEXT)
        y += 17
    out_img.save(out)


def main():
    out = sys.argv[1] if len(sys.argv) > 1 else os.path.dirname(os.path.abspath(__file__))
    os.makedirs(out, exist_ok=True)
    river_town(os.path.join(out, "1-river-town.png"))
    mining_town(os.path.join(out, "2-mining-town.png"))
    coastal_town(os.path.join(out, "3-coastal-port.png"))
    scaling(os.path.join(out, "4-population-scaling.png"))
    print("wrote sketches to", out)


if __name__ == "__main__":
    main()
