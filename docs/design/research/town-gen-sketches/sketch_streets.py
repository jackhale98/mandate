#!/usr/bin/env python3
"""THROWAWAY SKETCH SCRIPT: street-layout options for town layout v2.

Mock-ups for the owner to choose how streets should look. This is NOT the
generator and is not meant to become it: the real generator is TypeScript on
the client (web/src/hub/layoutV2/), integer-only and versioned. This script is
hand-tuned, uses floats and Python's `random` with a fixed seed, and only makes
pictures.

One river town geography (river crossing diagonally, a hill to the north-west,
a railway at the foot of the hill, a market square by the crossing, five
districts), drawn three ways:

  1. Today (v2 stage A): each district's land is cut into rectangles by
     straight edge-to-edge streets (an imitation of stage 6 in districts.ts),
     arterials step across the map on a 4-tile grid.
  2. Option A: streets grow from the square along desire lines, side streets
     branch off them; right angles only.
  3. Option B: as A, plus 45-degree streets (radials, a riverside high street,
     hill streets along the contours). Buildings stay axis-aligned.

Usage (Python 3 with Pillow):
    python3 docs/design/research/town-gen-sketches/sketch_streets.py [out_dir]
"""

import math
import os
import random
import sys
from collections import deque

from PIL import Image, ImageDraw, ImageFont

SEED = 4242
W, H = 100, 65
FONT_DIR = "/usr/share/fonts/truetype/dejavu/"


def font(size, bold=False):
    try:
        return ImageFont.truetype(FONT_DIR + ("DejaVuSans-Bold.ttf" if bold else "DejaVuSans.ttf"), size)
    except OSError:
        return ImageFont.load_default()


def rng(*key):
    """A keyed random stream (string seeds are stable across runs)."""
    return random.Random(":".join(str(k) for k in (SEED,) + key))


# ---------------------------------------------------------------- palette

BG = (250, 248, 242)
TEXT = (34, 32, 40)
SKETCH_RED = (176, 40, 40)
WATER = (92, 150, 206)
WATER_EDGE = (58, 100, 150)
CARRIAGE = (136, 138, 148)
CARRIAGE_MAIN = (108, 110, 122)
PAVEMENT = (222, 222, 226)
KERB = (160, 158, 160)
LANE = (166, 142, 110)
PATH = (198, 178, 138)
DIRT = (190, 164, 118)
BRIDGE = (196, 170, 128)
PARAPET = (92, 72, 52)
SQUARE = (234, 222, 196)
SQUARE_LINE = (214, 200, 172)
GREEN = (150, 198, 118)
TREE = (84, 138, 80)
TREE_DARK = (62, 108, 62)
RAIL_BED = (176, 168, 160)
RAIL = (58, 56, 62)
FIELDS = [(214, 214, 150), (198, 212, 142), (226, 208, 144), (190, 202, 152), (230, 222, 170)]
HEDGE = (116, 148, 88)
CONTOUR = (150, 140, 108)

CORE, TERR, IND, SUB, RUR = range(5)
# name, ground, roof, label
DISTRICTS = {
    CORE: ("Old core", (218, 212, 196), (180, 86, 62)),
    TERR: ("Terraces", (198, 216, 166), (146, 78, 86)),
    IND: ("Industry by the water", (194, 190, 182), (104, 112, 134)),
    SUB: ("Suburbs on the hill", (188, 216, 150), (208, 130, 90)),
    RUR: ("Rural fringe", (200, 212, 150), (156, 118, 78)),
}

# road classes
PATHC, LANEC, STREETC, MAINC = 1, 2, 3, 4
N4 = [(1, 0), (-1, 0), (0, 1), (0, -1)]


def shade(c, f):
    return tuple(max(0, min(255, int(round(v * f)))) for v in c)


def mixc(a, b, t):
    return tuple(int(round(a[i] + (b[i] - a[i]) * t)) for i in range(3))


class Noise:
    def __init__(self, key, cell):
        self.key = key
        self.cell = cell

    def lat(self, i, j):
        return rng("noise", self.key, i, j).random()

    def at(self, x, y):
        fx, fy = x / self.cell, y / self.cell
        i, j = math.floor(fx), math.floor(fy)
        tx, ty = fx - i, fy - j
        tx, ty = tx * tx * (3 - 2 * tx), ty * ty * (3 - 2 * ty)
        a = self.lat(i, j) + (self.lat(i + 1, j) - self.lat(i, j)) * tx
        b = self.lat(i, j + 1) + (self.lat(i + 1, j + 1) - self.lat(i, j + 1)) * tx
        return a + (b - a) * ty


def catmull(points, step=0.2):
    out = []
    pts = [points[0]] + points + [points[-1]]
    for k in range(1, len(pts) - 2):
        p0, p1, p2, p3 = pts[k - 1], pts[k], pts[k + 1], pts[k + 2]
        seg = max(1, int(math.dist(p1, p2) / step))
        for s in range(seg):
            t = s / seg
            t2, t3 = t * t, t * t * t
            out.append(tuple(0.5 * ((2 * p1[i]) + (-p0[i] + p2[i]) * t + (2 * p0[i] - 5 * p1[i] + 4 * p2[i] - p3[i]) * t2 + (-p0[i] + 3 * p1[i] - 3 * p2[i] + p3[i]) * t3) for i in range(2)))
    out.append(points[-1])
    return out


def interp(pts, x):
    """y of a polyline (sorted by x) at x."""
    for (x0, y0), (x1, y1) in zip(pts, pts[1:]):
        if x0 <= x <= x1:
            return y0 + (y1 - y0) * (x - x0) / max(1e-9, x1 - x0)
    return pts[0][1] if x < pts[0][0] else pts[-1][1]


# ---------------------------------------------------------------- geography (shared by every option)

RIVER = [(-2, 57), (10, 54), (22, 50), (34, 46), (46, 41), (56, 35), (66, 28), (78, 19), (90, 11), (102, 5)]
RAIL_PTS = [(-1, 32), (12, 28), (24, 23), (34, 18), (60, 18), (66, 12), (72, 6), (78, -1)]
SQ = (38, 26, 47, 32)  # market square x0, y0, x1, y1
OLD_BRIDGE_X = (44, 47)
MILL_BRIDGE_X = (80, 83)
STATION = (53, 20, 58, 22)
PLATFORM = (52, 19, 60, 19)


def rail_y(x):
    return interp(RAIL_PTS[:-1] + [(78, -1)], x) if x <= 78 else -99


class Geo:
    def __init__(self):
        n = Noise("hill", 9)
        self.h = [[max(0.0, 46 - 0.85 * (x + y)) + 3.0 * n.at(x, y) for x in range(W)] for y in range(H)]
        self.water = [[False] * W for _ in range(H)]
        for (px, py) in catmull([(float(a), float(b)) for a, b in RIVER]):
            for y in range(int(py) - 4, int(py) + 5):
                for x in range(int(px) - 4, int(px) + 5):
                    if 0 <= x < W and 0 <= y < H and math.dist((x + 0.5, y + 0.5), (px, py)) <= 2.75:
                        self.water[y][x] = True
        self.rail = [[False] * W for _ in range(H)]
        for (a, b) in zip(RAIL_PTS, RAIL_PTS[1:]):
            n_ = int(math.dist(a, b) * 4) + 1
            for k in range(n_ + 1):
                x = a[0] + (b[0] - a[0]) * k / n_
                y = a[1] + (b[1] - a[1]) * k / n_
                tx, ty = int(math.floor(x + 0.5)), int(math.floor(y + 0.5))
                if 0 <= tx < W and 0 <= ty < H:
                    self.rail[ty][tx] = True
        nb = Noise("districts", 7)
        self.dist = [[-1] * W for _ in range(H)]
        for y in range(H):
            for x in range(W):
                if self.water[y][x]:
                    continue
                j = (nb.at(x, y) - 0.5) * 6
                nw = y < interp(RIVER, x)
                above = y < rail_y(x)
                if nw and above:
                    d = SUB
                elif nw:
                    if math.dist((x, y * 1.15), (44, 29 * 1.15)) < 20 + j:
                        d = CORE
                    elif x < 44:
                        d = RUR
                    else:
                        d = IND
                else:
                    if ((x - 54 + j) / 28) ** 2 + ((y - 52 + j) / 14) ** 2 < 1:
                        d = TERR
                    else:
                        d = RUR
                self.dist[y][x] = d


# ---------------------------------------------------------------- a plan (one option)


class Plan:
    def __init__(self, geo, key):
        self.key = key
        self.g = geo
        self.water = geo.water
        self.rail = geo.rail
        self.dist = geo.dist
        self.road = [[0] * W for _ in range(H)]
        self.dirt = [[False] * W for _ in range(H)]
        self.diag = [[False] * W for _ in range(H)]  # part of a 45-degree street (drawn with 45-degree edges)
        self.square = [[False] * W for _ in range(H)]
        self.green = [[False] * W for _ in range(H)]
        self.bld = [[-1] * W for _ in range(H)]
        self.blocked = [[False] * W for _ in range(H)]  # platform etc.
        self.buildings = []  # (x0, y0, x1, y1, colour, kind)
        self.trees = []
        self.labels = []  # (x, y, text, size, bold)
        self.field = [[-1] * W for _ in range(H)]
        x0, y0, x1, y1 = SQ
        for y in range(y0, y1 + 1):
            for x in range(x0, x1 + 1):
                self.square[y][x] = True
        px0, py0, px1, py1 = PLATFORM
        for x in range(px0, px1 + 1):
            self.blocked[py0][x] = True

    # -- predicates
    @staticmethod
    def inb(x, y):
        return 0 <= x < W and 0 <= y < H

    def bridge_ok(self, x, y):
        return OLD_BRIDGE_X[0] <= x <= OLD_BRIDGE_X[1] and 30 < y < 50 or MILL_BRIDGE_X[0] <= x <= MILL_BRIDGE_X[1] and y < 26

    def roadlike(self, x, y):
        return self.inb(x, y) and (self.road[y][x] > 0 or self.square[y][x])

    def free(self, x, y):
        return self.inb(x, y) and not self.water[y][x] and not self.road[y][x] and not self.rail[y][x] and not self.square[y][x] and not self.green[y][x] and self.bld[y][x] < 0 and not self.blocked[y][x]

    # -- painting
    def put(self, x, y, cls, dirt=False, diag=False):
        if not self.inb(x, y) or self.square[y][x] or self.blocked[y][x] or self.bld[y][x] >= 0:
            return
        if self.water[y][x] and not self.bridge_ok(x, y):
            return
        if diag:
            self.diag[y][x] = True
        if self.road[y][x] < cls:
            self.road[y][x] = cls
            self.dirt[y][x] = dirt

    def rect(self, x0, y0, x1, y1, cls, dirt=False):
        for y in range(min(y0, y1), max(y0, y1) + 1):
            for x in range(min(x0, x1), max(x0, x1) + 1):
                self.put(x, y, cls, dirt)

    def band_tiles(self, x, y, dx, dy, w):
        """Tiles of a street's cross-section at centre (x, y) heading (dx, dy)."""
        if dx == 0:
            return [(x + k - (w - 1) // 2, y) for k in range(w)]
        if dy == 0:
            return [(x, y + k - (w - 1) // 2) for k in range(w)]
        run = {1: 2, 2: 4, 3: 5, 4: 6}.get(w, w + 2)  # about w tiles across once drawn at 45 degrees
        return [(x + k - (run - 1) // 2, y) for k in range(run)]

    def path(self, pts, w, cls, dirt=False):
        """A street along axis-aligned or 45-degree legs, `w` tiles wide."""
        for (ax, ay), (bx, by) in zip(pts, pts[1:]):
            dx = (bx > ax) - (bx < ax)
            dy = (by > ay) - (by < ay)
            n = max(abs(bx - ax), abs(by - ay))
            assert dx == 0 or dy == 0 or abs(bx - ax) == abs(by - ay), (ax, ay, bx, by)
            for k in range(n + 1):
                for (tx, ty) in self.band_tiles(ax + dx * k, ay + dy * k, dx, dy, w):
                    self.put(tx, ty, cls, dirt, diag=dx != 0 and dy != 0)
        # fill the elbows
        for (px, py) in pts[1:-1]:
            o = (w - 1) // 2
            self.rect(px - o, py - o, px - o + w - 1, py - o + w - 1, cls, dirt)

    # -- distance to the nearest road through free land
    def road_distance(self, limit=99):
        INF = 999
        d = [[INF] * W for _ in range(H)]
        par = [[None] * W for _ in range(H)]
        q = deque()
        for y in range(H):
            for x in range(W):
                if (self.road[y][x] and not self.water[y][x]) or self.square[y][x]:
                    d[y][x] = 0
                    q.append((x, y))
        while q:
            x, y = q.popleft()
            if d[y][x] >= limit:
                continue
            for dx, dy in N4:
                nx, ny = x + dx, y + dy
                if self.free(nx, ny) and d[ny][nx] == INF:
                    d[ny][nx] = d[y][x] + 1
                    par[ny][nx] = (x, y)
                    q.append((nx, ny))
        return d, par


# ---------------------------------------------------------------- 1. today: recursive rectangular cuts

TODAY_PATTERN = {
    # district: (width, block, depth, lattice, jog, stubs, jitter, dirt)
    CORE: (4, (9, 13), (7, 9), False, 0.3, 0.0, 2, False),
    TERR: (4, (12, 18), (7, 8), True, 0.0, 0.0, 0, False),
    IND: (4, (16, 24), (9, 12), True, 0.0, 0.0, 1, False),
    SUB: (3, (12, 18), (7, 9), False, 0.6, 0.45, 1, False),
    RUR: (2, (18, 28), (10, 14), False, 0.3, 0.5, 3, True),
}


def coarse_walk(p, a, b, w=4):
    """An arterial as A* on a 4-tile grid draws it: a staircase of whole 4x4 cells."""
    ax, ay, bx, by = a[0] // 4, a[1] // 4, b[0] // 4, b[1] // 4
    x, y = ax, ay
    cells = [(x, y)]
    while (x, y) != (bx, by):
        ex, ey = bx - x, by - y
        # step along the axis that is further behind the straight line
        if ex and (not ey or abs(ex) * abs(by - ay + 0.01) >= abs(ey) * abs(bx - ax + 0.01)):
            x += (ex > 0) - (ex < 0)
        else:
            y += (ey > 0) - (ey < 0)
        cells.append((x, y))
    for (cx, cy) in cells:
        p.rect(cx * 4, cy * 4, cx * 4 + w - 1, cy * 4 + w - 1, MAINC)


def today(geo):
    p = Plan(geo, "today")
    # The square's ring (3 tiles) and the arterials (4 tiles on the 4-tile grid).
    x0, y0, x1, y1 = SQ
    for (a, b, c, d) in ((x0 - 3, y0 - 3, x1 + 3, y0 - 1), (x0 - 3, y1 + 1, x1 + 3, y1 + 3), (x0 - 3, y0, x0 - 1, y1), (x1 + 1, y0, x1 + 3, y1)):
        p.rect(a, b, c, d, MAINC)
    coarse_walk(p, (44, 33), (44, 64))  # south over the Old Bridge
    coarse_walk(p, (44, 20), (44, 12))  # north past the station
    coarse_walk(p, (44, 12), (20, 0))
    coarse_walk(p, (36, 32), (0, 44))  # west along the bank
    coarse_walk(p, (48, 24), (80, 8))  # east to the Mill Bridge
    coarse_walk(p, (80, 8), (80, 24))
    coarse_walk(p, (80, 24), (99, 32))
    coarse_walk(p, (48, 48), (99, 52))  # east along the south bank
    # Stage 6: each district's land cut into blocks, component by component.
    for di in (CORE, TERR, IND, SUB, RUR):
        sw, (bw0, bw1), (bd0, bd1), lattice, jog, stubs, jitter, dirt = TODAY_PATTERN[di]
        r = rng("today", "streets", di)
        cls = STREETC if sw >= 3 else LANEC
        cells = [(x, y) for y in range(H) for x in range(W) if geo.dist[y][x] == di and p.free(x, y)]
        lat_x, lat_y = (cells[len(cells) // 2] if cells else (0, 0))
        pY = (bd0 + bd1) // 2 + sw
        pX = (bw0 + bw1) // 2 + sw

        def snap(v, origin, per, lo, hi):
            s = origin + round((v - origin) / per) * per
            return s if lo <= s <= hi else v

        def comps(cellset):
            seen = set()
            out = []
            for c in sorted(cellset, key=lambda t: (t[1], t[0])):
                if c in seen:
                    continue
                comp = [c]
                seen.add(c)
                i = 0
                while i < len(comp):
                    cx, cy = comp[i]
                    i += 1
                    for dx, dy in N4:
                        n = (cx + dx, cy + dy)
                        if n in cellset and n not in seen:
                            seen.add(n)
                            comp.append(n)
                out.append(comp)
            return out

        def cut(comp_set, axis, at, lo, hi, min_len):
            made = []
            start = None
            for a in range(lo, hi + 2):
                ok = a <= hi and all(((a, at + k) if axis == "h" else (at + k, a)) in comp_set for k in range(sw))
                if ok and start is None:
                    start = a
                if not ok and start is not None:
                    if a - start >= min_len:
                        made.append((start, at, a - 1, at + sw - 1) if axis == "h" else (at, start, at + sw - 1, a - 1))
                    start = None
            return made

        queue = comps(set(cells))
        guard = 0
        while queue and guard < 3000:
            guard += 1
            comp = queue.pop(0)
            if len(comp) < 30:
                continue
            cs = set(comp)
            xs = [c[0] for c in comp]
            ys = [c[1] for c in comp]
            cx0, cx1, cy0, cy1 = min(xs), max(xs), min(ys), max(ys)
            bw, bh = cx1 - cx0 + 1, cy1 - cy0 + 1
            depth = r.randint(bd0, bd1)
            across = r.randint(bw0, bw1)
            made = []
            if bh >= depth + sw + 5:
                y = cy0 + depth + (r.randint(-jitter, jitter) if jitter else 0)
                if lattice:
                    y = snap(y, lat_y, pY, cy0 + bd0, cy1 - sw - 4)
                y = max(cy0 + bd0 - 1, min(y, cy1 - sw - 4))
                if jog and bw >= 18 and r.random() < jog:
                    mid = cx0 + bw // 2 + r.randint(-3, 3)
                    dy = r.choice([-2, -1, 1, 2])
                    left = cut(cs, "h", y, cx0, mid + sw - 1, 6)
                    right = cut(cs, "h", y + dy, mid, cx1, 6)
                    if left and right:
                        made = left + right + [(mid, min(y, y + dy), mid + sw - 1, max(y, y + dy) + sw - 1)]
                if not made:
                    made = cut(cs, "h", y, cx0, cx1, 6)
            if not made and bw >= across + sw + 6:
                x = cx0 + across + (r.randint(-jitter, jitter) if jitter else 0)
                if lattice:
                    x = snap(x, lat_x, pX, cx0 + bw0, cx1 - sw - 5)
                x = max(cx0 + 6, min(x, cx1 - sw - 5))
                if stubs and r.random() < stubs and bh >= 12:
                    from_top = r.random() < 0.5
                    ln = bh * r.randint(50, 70) // 100
                    made = cut(cs, "v", x, cy0 if from_top else cy1 - ln + 1, cy0 + ln - 1 if from_top else cy1, 4)
                    for s in made:
                        for yy in range(s[1], s[3] + 1):
                            for xx in range(s[0], s[2] + 1):
                                if (xx, yy) in cs:
                                    p.put(xx, yy, cls, dirt)
                    continue
                made = cut(cs, "v", x, cy0, cy1, 3)
            if not made:
                continue
            for s in made:
                for yy in range(s[1], s[3] + 1):
                    for xx in range(s[0], s[2] + 1):
                        if (xx, yy) in cs:
                            p.put(xx, yy, cls, dirt)
            rest = set(c for c in comp if p.free(*c))
            queue.extend(comps(rest))
    return p


# ---------------------------------------------------------------- 2/3. grown streets

GROW = {
    # widths to pick from; the deepest a tile may lie from a street; tiles a street runs on past
    # its target before deciding again; chance to turn; chance to stop (a cul-de-sac or a lane's end)
    # when no street is close ahead; a turning head at a dead end; dirt
    CORE: dict(widths=[2, 2, 2, 1], gap=4, extra=(1, 5), turn=0.5, stop=0.25, head=False, dirt=False),
    TERR: dict(widths=[3], gap=6, extra=(6, 40), turn=0.1, stop=0.0, head=False, dirt=False),
    IND: dict(widths=[4], gap=9, extra=(2, 8), turn=0.25, stop=0.6, head=True, dirt=False),
    SUB: dict(widths=[3], gap=7, extra=(2, 7), turn=0.55, stop=0.6, head=True, dirt=False),
    RUR: dict(widths=[2], gap=12, extra=(4, 14), turn=0.4, stop=0.6, head=False, dirt=True),
}

AXES = [(1, 0), (-1, 0), (0, 1), (0, -1)]
CONTOUR_DIRS = [(1, -1), (-1, 1)]


def grow_district(p, di, dirs, iters=90):
    """Side streets branch off the streets already there: each new street heads for
    the land furthest from any street, leaves its street square to it (moved along so it
    does not meet a street joining from the far side: a T, not a crossroads), and runs on
    until it meets another street (a T) or stops (a cul-de-sac, a lane's end), sometimes
    turning once on the way (L-shaped blocks)."""
    prm = GROW[di]
    r = rng(p.key, "grow", di)
    skip = set()
    for _ in range(iters):
        d, par = p.road_distance()
        best = 0
        for y in range(H):
            for x in range(W):
                if p.dist[y][x] == di and d[y][x] < 999 and (x, y) not in skip and d[y][x] > best:
                    best = d[y][x]
        if best <= prm["gap"]:
            break
        cands = [(x, y) for y in range(H) for x in range(W) if p.dist[y][x] == di and (x, y) not in skip and best - 1 <= d[y][x] < 999]
        tx, ty = r.choice(cands)
        # the root: where the shortest way from the deepest tile meets a street
        sx, sy = tx, ty
        while par[sy][sx] is not None and not p.roadlike(*par[sy][sx]):
            sx, sy = par[sy][sx]
        vx, vy = tx - sx, ty - sy
        if vx == 0 and vy == 0:
            vx, vy = r.choice(AXES)
        L = math.hypot(vx, vy)
        order = sorted(dirs, key=lambda e: -(e[0] * vx + e[1] * vy) / (L * math.hypot(*e)))
        w = r.choice(prm["widths"])
        ok = any(lay_street(p, di, tx, ty, dx, dy, w, prm, r) for dx, dy in order[:2])
        if not ok:
            for (ax, ay) in [(tx, ty), (tx + 1, ty), (tx - 1, ty), (tx, ty + 1), (tx, ty - 1)]:
                skip.add((ax, ay))


def lay_street(p, di, tx, ty, dx, dy, w, prm, r):
    cls = {1: PATHC, 2: LANEC, 3: STREETC, 4: MAINC}[w]
    dirt = prm["dirt"]

    def band(x, y, ddx, ddy):
        return p.band_tiles(x, y, ddx, ddy, w)

    def hits_road(ts, own=()):
        return any(p.roadlike(a, b) and (a, b) not in own for a, b in ts)

    def open_land(ts):
        return all(p.free(a, b) and p.dist[b][a] == di for a, b in ts)

    def back(cx, cy):
        """Walk back from the target to the street the new one leaves."""
        x, y = cx, cy
        for _ in range(40):
            ts = band(x, y, dx, dy)
            if hits_road(ts):
                return (x, y)
            if not open_land(ts):
                return None
            x, y = x - dx, y - dy
        return None

    axis = dx == 0 or dy == 0
    perp = (dy, -dx) if axis else (1, 0)
    start = None
    for off in sorted(range(-4, 5), key=lambda o: (abs(o), o)):
        cx, cy = tx + perp[0] * off, ty + perp[1] * off
        if not p.inb(cx, cy):
            continue
        s = back(cx, cy)
        if s is None:
            continue
        if axis:
            # across the street it leaves: does a street join from the far side near here?
            fx, fy = s
            k = 0
            while k < 7 and hits_road(band(fx, fy, dx, dy)):
                fx, fy = fx - dx, fy - dy
                k += 1
            near = []
            for m in range(-(w + 1), w + 2):
                near += [(a + perp[0] * m, b + perp[1] * m) for a, b in band(fx, fy, dx, dy)]
            if k < 7 and any(p.road[b][a] and not p.water[b][a] for a, b in near if p.inb(a, b)) and abs(off) < 4:
                continue
        start = s
        break
    if start is None:
        return False
    sx, sy = start
    laid = []
    own = set()
    x, y = sx + dx, sy + dy
    cdx, cdy = dx, dy
    steps = 0
    joined = False
    passed = False
    turned = False
    remaining = None
    while steps < 70:
        ts = band(x, y, cdx, cdy)
        if not all(p.inb(a, b) for a, b in ts):
            break
        if steps > 0 and hits_road(ts, own):
            joined = True
            break
        if not open_land(ts):
            break
        if steps > 0:
            # never run alongside another street (that would merge into one wide paved strip)
            if cdx == 0 or cdy == 0:
                sp = (cdy, -cdx)
                side = [(a + sp[0] * m, b + sp[1] * m) for a, b in (ts[0], ts[-1]) for m in (-1, 1)]
            else:
                side = [(ts[0][0] - 1, ts[0][1]), (ts[-1][0] + 1, ts[-1][1])]
            if any(p.roadlike(a, b) and (a, b) not in own and (a, b) not in ts for a, b in side):
                break
        laid.append(ts)
        for a, b in ts:
            p.put(a, b, cls, dirt, diag=cdx != 0 and cdy != 0)
            own.add((a, b))
        steps += 1
        if not passed and ((x, y) == (tx, ty) or steps > 45):
            passed = True
            remaining = r.randint(*prm["extra"])
        if passed:
            remaining -= 1
            if remaining <= 0:
                ahead = any(hits_road(band(x + cdx * k, y + cdy * k, cdx, cdy), own) for k in range(1, 6))
                if not ahead:
                    if not turned and r.random() < prm["turn"]:
                        turned = True
                        for b in range(y - (w - 1) // 2, y + w // 2 + 1):
                            for a in range(x - (w - 1) // 2, x + w // 2 + 1):
                                if p.free(a, b) and p.dist[b][a] == di:
                                    p.put(a, b, cls, dirt)
                                    own.add((a, b))
                        opts = [(cdy, -cdx), (-cdy, cdx)] if (cdx == 0 or cdy == 0) else [(cdx, 0), (0, cdy)]
                        r.shuffle(opts)
                        cdx, cdy = opts[0]
                        # the new leg starts at the old leg's edge, so the corner keeps the width
                        o = (w - 1) // 2 if cdx + cdy < 0 else w // 2
                        x, y = x + cdx * (o + 1), y + cdy * (o + 1)
                        remaining = r.randint(*prm["extra"])
                        continue
                    if r.random() < prm["stop"]:
                        break
                remaining = 99
        x, y = x + cdx, y + cdy
    if steps < 3:
        for a, b in own:
            p.road[b][a] = 0
            p.diag[b][a] = False
        return False
    if not joined and prm["head"] and steps >= 4:
        # a turning head at the end of a cul-de-sac
        ex, ey = laid[-1][len(laid[-1]) // 2]
        hw = w // 2 + 1
        for b in range(ey - hw, ey + hw + 1):
            for a in range(ex - hw, ex + hw + 1):
                if p.free(a, b) and p.dist[b][a] == di:
                    p.put(a, b, cls, dirt)
    return True


def junction_greens(p, di, count, size):
    """Small greens in the corners of junctions."""
    r = rng(p.key, "greens", di)
    cands = []
    for y in range(1, H - 1):
        for x in range(1, W - 1):
            if p.dist[y][x] != di or not p.free(x, y):
                continue
            rx = 1 if p.road[y][x - 1] else (-1 if p.road[y][x + 1] else 0)
            ry = 1 if p.road[y - 1][x] else (-1 if p.road[y + 1][x] else 0)
            if rx and ry and p.road[y - ry][x - rx]:
                cands.append((x, y, rx, ry))
    r.shuffle(cands)
    made = []
    for (x, y, rx, ry) in cands:
        if len(made) >= count:
            break
        if any(abs(x - mx) + abs(y - my) < 14 for mx, my in made):
            continue
        sw, sh = size
        tiles = [(x + rx * i, y + ry * j) for i in range(sw) for j in range(sh)]
        if all(p.free(a, b) and p.dist[b][a] == di for a, b in tiles):
            for a, b in tiles:
                p.green[b][a] = True
            ax = sorted(a for a, _ in tiles)
            ay = sorted(b for _, b in tiles)
            p.trees.append((ax[len(ax) // 2] + 0.5, ay[len(ay) // 2] + 0.5, 0.9))
            made.append((x, y))


def option_a(geo):
    p = Plan(geo, "a")
    # Desire lines from the square: to the Old Bridge (the high street), the station and
    # the north road, the west road, the Mill Bridge; long legs, few turns.
    p.path([(45, 33), (45, 56), (51, 56), (51, 64)], 4, MAINC)  # High Street, over the Old Bridge, a jog south
    p.path([(47, 25), (47, 11), (31, 11), (31, 0)], 3, MAINC)  # Station Road, up the hill
    p.path([(37, 30), (19, 30), (19, 41), (0, 41)], 3, MAINC)  # West Road
    p.path([(48, 28), (54, 28), (54, 24), (72, 24), (72, 13), (81, 13), (81, 26), (99, 26)], 3, MAINC)  # Mill Road, past the station, over the Mill Bridge
    p.path([(48, 47), (66, 47), (66, 43), (99, 43)], 3, MAINC)  # south-bank road east
    # Old-core lanes: T-junctions, short legs, 1 to 2 tiles wide.
    p.path([(33, 28), (33, 22), (26, 22)], 2, LANEC)  # Church Lane
    p.path([(38, 25), (38, 20)], 2, LANEC)  # beside the town hall
    p.path([(33, 32), (33, 37), (43, 37)], 2, LANEC)  # Back Lane, to the High Street
    p.path([(39, 33), (39, 36)], 2, LANEC)  # Water Lane, off the square
    p.path([(36, 33), (36, 36)], 1, PATHC)  # a passage
    landmarks(p)  # the town hall, church and works take their sites before the side streets
    for di, dirs in ((CORE, AXES), (TERR, AXES), (IND, AXES), (SUB, AXES), (RUR, AXES)):
        grow_district(p, di, dirs)
    junction_greens(p, CORE, 2, (3, 3))
    junction_greens(p, TERR, 3, (4, 3))
    junction_greens(p, SUB, 4, (4, 4))
    return p


def option_b(geo):
    p = Plan(geo, "b")
    p.path([(45, 33), (45, 50)], 4, MAINC)  # High Street to the Old Bridge
    # Riverside high street slanting along the river to the Mill Bridge.
    p.path([(48, 34), (54, 28), (58, 28), (64, 22), (69, 22), (76, 15), (81, 15), (81, 24), (85, 28), (99, 28)], 4, MAINC)
    # Radials from the square at 45 degrees.
    p.path([(48, 25), (50, 23), (50, 10), (56, 4), (56, 0)], 3, MAINC)  # Station Road, NE, then up
    p.path([(37, 25), (29, 17), (19, 7), (19, 0)], 3, MAINC)  # Hill Road, NW, up the slope
    p.path([(37, 31), (24, 31), (17, 38), (0, 38)], 3, MAINC)  # West Road
    p.path([(47, 50), (56, 59), (99, 59)], 3, MAINC)  # south-east radial from the bridge foot
    p.path([(48, 47), (60, 47), (66, 41), (99, 41)], 3, MAINC)  # south-bank road east
    # Hill streets along the contours (x + y constant runs along the slope).
    p.path([(0, 14), (14, 0)], 3, STREETC)
    p.path([(6, 28), (34, 0)], 3, STREETC)
    p.path([(34, 16), (50, 0)], 3, STREETC)
    # Old-core lanes (axis-aligned between the radials).
    p.path([(33, 33), (33, 37), (43, 37)], 2, LANEC)  # Back Lane
    p.path([(39, 33), (39, 36)], 2, LANEC)  # Water Lane
    p.path([(36, 33), (36, 36)], 1, PATHC)  # a passage
    p.path([(31, 30), (31, 24), (28, 24)], 2, LANEC)  # off the West Road, west of the Hill Road
    landmarks(p)
    for di, dirs in ((CORE, AXES), (TERR, AXES), (IND, AXES), (SUB, AXES + CONTOUR_DIRS), (RUR, AXES)):
        grow_district(p, di, dirs)
    junction_greens(p, CORE, 2, (3, 3))
    junction_greens(p, TERR, 3, (4, 3))
    junction_greens(p, SUB, 4, (4, 4))
    return p


# ---------------------------------------------------------------- landmarks, buildings, fields (shared)

PLOTS = {
    # frontage, depth, setbacks (today, grown), gap chance, density
    CORE: ((3, 5), (3, 5), (0, [0, 0, 0, 1]), 0.04, 1.0),
    TERR: ((3, 4), (3, 3), (0, [0, 0, 1]), 0.04, 1.0),
    IND: ((5, 9), (4, 7), (0, [0, 1, 2]), 0.3, 1.0),
    SUB: ((3, 4), (3, 3), (1, [1, 1, 2]), 0.5, 1.0),
    RUR: ((3, 5), (3, 3), (1, [1, 2, 3]), 1.0, 0.22),
}
CORE_SMALL = ((2, 4), (2, 4))


def find_rect(p, w, h, target, radius=10, di=None):
    tx, ty = target
    best = None
    for y in range(ty - radius, ty + radius + 1):
        for x in range(tx - radius, tx + radius + 1):
            if all(p.free(a, b) and (di is None or p.dist[b][a] == di) for b in range(y, y + h) for a in range(x, x + w)):
                dd = abs(x - tx) + abs(y - ty)
                if best is None or dd < best[0]:
                    best = (dd, x, y)
    return None if best is None else (best[1], best[2])


def add_building(p, x0, y0, x1, y1, colour, kind="house"):
    i = len(p.buildings)
    p.buildings.append((x0, y0, x1, y1, colour, kind))
    for y in range(y0, y1 + 1):
        for x in range(x0, x1 + 1):
            p.bld[y][x] = i


def landmarks(p):
    """Fixed for every option: the station by the line, the town hall on the square, the church
    west of the square, the works by the water."""
    # Station building beside the platform (fixed by the railway).
    x0, y0, x1, y1 = STATION
    for y in range(y0, y1 + 1):
        for x in range(x0, x1 + 1):
            p.road[y][x] = 0
    add_building(p, x0, y0, x1, y1, (128, 98, 150), "station")
    p.labels.append((55.5, 23.8, "Station", 15, False))
    # Town hall facing the square from the north.
    at = find_rect(p, 6, 4, (40, 22), 6)
    if at:
        add_building(p, at[0], at[1], at[0] + 5, at[1] + 3, (214, 186, 104), "hall")
        p.labels.append((at[0] + 3, at[1] - 0.6, "Town hall", 15, False))
    # Church in its churchyard, west of the square.
    at = find_rect(p, 7, 7, (25, 33), 12)
    if at:
        for y in range(at[1], at[1] + 7):
            for x in range(at[0], at[0] + 7):
                p.green[y][x] = True
        for y in range(at[1] + 2, at[1] + 5):
            for x in range(at[0] + 1, at[0] + 6):
                p.green[y][x] = False
        add_building(p, at[0] + 1, at[1] + 2, at[0] + 5, at[1] + 4, (156, 152, 164), "church")
        p.trees += [(at[0] + 0.6, at[1] + 0.6, 0.8), (at[0] + 6.4, at[1] + 0.6, 0.8), (at[0] + 0.6, at[1] + 6.4, 0.8), (at[0] + 6.4, at[1] + 6.4, 0.8)]
        p.labels.append((at[0] + 3.5, at[1] + 8.2, "St Mary's", 15, False))
    # Mill by the water in the industrial district.
    at = find_rect(p, 6, 5, (66, 26), 16, IND) or find_rect(p, 5, 4, (66, 26), 16, IND)
    if at:
        add_building(p, at[0], at[1], at[0] + 5, at[1] + 4, (104, 108, 128), "works")
        p.labels.append((at[0] + 3, at[1] + 6.4, "Works", 15, False))


def fill_buildings(p, grown):
    """Buildings front the streets and the square: each plot starts on the street (or set back
    from it) and runs back from it. Then gaps are filled with smaller plots, and in the old core
    and the works, back buildings go behind the front rows."""
    r = rng(p.key, "buildings")
    setback_run = {}
    for pas in range(2):
        d, _ = p.road_distance(limit=14)
        for y in range(H):
            for x in range(W):
                di = p.dist[y][x]
                if di < 0 or not p.free(x, y) or d[y][x] != 1:
                    continue
                rs = [(a, b) for a, b in N4 if p.roadlike(x + a, y + b) and not p.water[y + b][x + a]]
                if not rs:
                    continue
                if any(p.dirt[y + b][x + a] for a, b in rs) and di != RUR:
                    continue
                (fr0, fr1), (dp0, dp1), (sb_today, sb_grown), gap, dens = PLOTS[di]
                if grown and di == CORE and any(p.road[y + b][x + a] <= LANEC and not p.square[y + b][x + a] for a, b in rs):
                    (fr0, fr1), (dp0, dp1) = CORE_SMALL
                if pas == 1:
                    if di in (SUB, RUR):
                        continue
                    fr0, fr1, dp0 = 2, max(2, fr0), 2
                if r.random() > dens:
                    continue
                # setback: today one per type; grown towns vary it along a street, a few plots at a time
                if grown and pas == 0:
                    key = (di, rs[0])
                    if key not in setback_run or r.random() < 0.3:
                        setback_run[key] = r.choice(sb_grown)
                    sb = setback_run[key]
                else:
                    sb = sb_today if pas == 0 else 0
                if any(p.square[y + b][x + a] for a, b in rs):
                    sb = 0
                awx = -sum(a for a, _ in rs)
                awy = -sum(b for _, b in rs)
                fw = r.randint(fr0, fr1)
                dp = r.randint(dp0, dp1)
                if awx and awy:
                    sx, sy = r.randint(2, 3), r.randint(2, 3)
                    ox, oy, dirx, diry = x + awx * sb, y + awy * sb, awx, awy
                elif awy:
                    sx, sy = fw, dp
                    ox, oy, dirx, diry = x, y + awy * sb, 1, awy
                elif awx:
                    sx, sy = dp, fw
                    ox, oy, dirx, diry = x + awx * sb, y, awx, 1
                else:
                    continue
                for (ax, ay) in [(sx, sy), (sx - 1, sy), (sx, sy - 1), (sx - 1, sy - 1), (2, 2)]:
                    if ax < 2 or ay < 2:
                        continue
                    xs = sorted([ox, ox + dirx * (ax - 1)])
                    ys = sorted([oy, oy + diry * (ay - 1)])
                    tiles = [(a, b) for b in range(ys[0], ys[1] + 1) for a in range(xs[0], xs[1] + 1)]
                    if all(p.free(a, b) and p.dist[b][a] == di and d[b][a] >= 1 + sb for a, b in tiles):
                        roof = shade(DISTRICTS[di][2], r.uniform(0.86, 1.1))
                        add_building(p, xs[0], ys[0], xs[1], ys[1], roof, "shed" if di == IND else "house")
                        if pas == 0 and r.random() < gap:
                            # a gap after it (detached houses, yards)
                            gx, gy = (xs[1] + 1, y) if awy else (x, ys[1] + 1)
                            if p.free(gx, gy):
                                p.blocked[gy][gx] = True
                        break
        # clear the gap markers (only spacers), keeping the platform
        px0, py0, px1, py1 = PLATFORM
        for y in range(H):
            for x in range(W):
                if p.blocked[y][x] and not (py0 == y and px0 <= x <= px1):
                    p.blocked[y][x] = False
    # back buildings: workshops behind the old core's rows, sheds in the works' yards
    for y in range(H):
        for x in range(W):
            di = p.dist[y][x]
            if di not in (CORE, IND) or not p.free(x, y):
                continue
            if not any(p.inb(x + a, y + b) and p.bld[y + b][x + a] >= 0 for a, b in N4):
                continue
            if r.random() > (0.6 if di == CORE else 0.3):
                continue
            sx, sy = (r.randint(2, 3), r.randint(2, 3)) if di == CORE else (r.randint(3, 6), r.randint(3, 5))
            tiles = [(a, b) for b in range(y, y + sy) for a in range(x, x + sx)]
            if all(p.free(a, b) and p.dist[b][a] == di for a, b in tiles) and all(not p.roadlike(a + e, b + f) for a, b in tiles for e, f in N4):
                add_building(p, x, y, x + sx - 1, y + sy - 1, shade(DISTRICTS[di][2], r.uniform(0.8, 0.95)), "back")
    # the old core is built up: small infill buildings in what is left, a few yards stay open
    for y in range(H):
        for x in range(W):
            if p.dist[y][x] != CORE or not p.free(x, y) or r.random() > 0.7:
                continue
            for (sx, sy) in ((3, 2), (2, 3), (2, 2)):
                tiles = [(a, b) for b in range(y, y + sy) for a in range(x, x + sx)]
                if all(p.free(a, b) and p.dist[b][a] == CORE for a, b in tiles):
                    add_building(p, x, y, x + sx - 1, y + sy - 1, shade(DISTRICTS[CORE][2], r.uniform(0.84, 1.05)), "back")
                    break


def fields_and_trees(p):
    r = rng(p.key, "fields")
    pts = [(r.uniform(0, W), r.uniform(0, H)) for _ in range(70)]
    for y in range(H):
        for x in range(W):
            if p.dist[y][x] == RUR and p.free(x, y):
                # squarish fields: nearest centre by a stretched max-norm
                k = min(range(len(pts)), key=lambda n: max(abs(pts[n][0] - x) * 0.8, abs(pts[n][1] - y)))
                p.field[y][x] = k
    # garden trees in the suburbs, trees on greens, along the river
    for y in range(H):
        for x in range(W):
            if not p.free(x, y) and not p.green[y][x]:
                continue
            di = p.dist[y][x]
            h = rng(p.key, "tree", x, y).random()
            if p.green[y][x] and h < 0.12:
                p.trees.append((x + 0.5, y + 0.5, 0.75))
            elif di == SUB and p.field[y][x] < 0 and h < 0.07:
                p.trees.append((x + 0.5, y + 0.5, 0.7))
            elif di == RUR and h < 0.02:
                p.trees.append((x + 0.5, y + 0.5, 0.8))


def finish(p, grown):
    if not p.labels:
        landmarks(p)
    fill_buildings(p, grown)
    fields_and_trees(p)
    return p


# ---------------------------------------------------------------- rendering


CORNERS = {
    # corner: the two sides it touches, and its triangle (unit square, y down)
    "NE": ((0, -1), (1, 0), [(0, 0), (1, 0), (1, 1)]),
    "SE": ((1, 0), (0, 1), [(1, 0), (1, 1), (0, 1)]),
    "SW": ((0, 1), (-1, 0), [(1, 1), (0, 1), (0, 0)]),
    "NW": ((-1, 0), (0, -1), [(0, 1), (0, 0), (1, 0)]),
}


def render_map(p, T):
    img = Image.new("RGB", (W * T, H * T), BG)
    dr = ImageDraw.Draw(img, "RGBA")
    g = p.g
    hmax = max(max(r) for r in g.h) or 1
    # 1. a surface kind and colour per tile
    kind = [[""] * W for _ in range(H)]
    col = [[BG] * W for _ in range(H)]
    for y in range(H):
        for x in range(W):
            c = p.road[y][x]
            if c and p.water[y][x]:
                kind[y][x], col[y][x] = "bridge", BRIDGE
            elif c:
                edge = any(p.inb(x + a, y + b) and not p.road[y + b][x + a] and not p.square[y + b][x + a] for a, b in N4)
                if p.dirt[y][x]:
                    kind[y][x], col[y][x] = "dirt", DIRT
                elif c == PATHC:
                    kind[y][x], col[y][x] = "path", PATH
                elif c == LANEC:
                    kind[y][x], col[y][x] = "lane", LANE
                elif edge:
                    kind[y][x], col[y][x] = "pave", PAVEMENT
                else:
                    kind[y][x], col[y][x] = "carr", CARRIAGE_MAIN if c == MAINC else CARRIAGE
            elif p.water[y][x]:
                kind[y][x], col[y][x] = "water", WATER
            elif p.square[y][x]:
                kind[y][x], col[y][x] = "square", SQUARE
            else:
                di = p.dist[y][x]
                base = DISTRICTS[di][1] if di >= 0 else (206, 216, 160)
                if p.field[y][x] >= 0:
                    base = FIELDS[p.field[y][x] % len(FIELDS)]
                if p.green[y][x]:
                    base = GREEN
                kind[y][x], col[y][x] = "ground", shade(base, 1.0 - 0.1 * g.h[y][x] / hmax)
    for y in range(H):
        for x in range(W):
            dr.rectangle([x * T, y * T, x * T + T - 1, y * T + T - 1], fill=col[y][x])
    # 2. 45-degree edges (as the art's 45-degree autotiles would draw them): on diagonal streets,
    #    and along the river everywhere; buildings keep whole tiles
    cut = {}
    soft = ("ground", "pave", "carr", "lane", "dirt", "path", "water")
    # each boundary is cut on one side only: a tile takes a corner of a lower-ranked neighbour
    rank = {"water": 0, "ground": 1, "path": 2, "dirt": 2, "lane": 2, "pave": 3, "carr": 4}
    for y in range(H):
        for x in range(W):
            kA = kind[y][x]
            if kA not in soft or p.bld[y][x] >= 0:
                continue
            for name, ((ax, ay), (bx, by), tri) in CORNERS.items():
                n1, n2 = (x + ax, y + ay), (x + bx, y + by)
                o1, o2 = (x - ax, y - ay), (x - bx, y - by)
                if not (p.inb(*n1) and p.inb(*n2)):
                    continue
                kB = kind[n1[1]][n1[0]]
                if kB == kA or kB not in soft or kind[n2[1]][n2[0]] != kB or rank[kB] >= rank[kA]:
                    continue
                if p.bld[n1[1]][n1[0]] >= 0 or p.bld[n2[1]][n2[0]] >= 0:
                    continue
                if any(p.inb(*o) and kind[o[1]][o[0]] == kB for o in (o1, o2)):
                    continue
                wet = "water" in (kA, kB)
                if wet and not ("ground" in (kA, kB)):
                    continue
                if not wet and not (p.diag[y][x] or p.diag[n1[1]][n1[0]] or p.diag[n2[1]][n2[0]]):
                    continue
                cut[(x, y)] = (name, kB)
                poly = [(x * T + u * T, y * T + v * T) for u, v in tri]
                dr.polygon(poly, fill=col[n1[1]][n1[0]])
                break

    def hidden(x, y, dx, dy):
        """An edge that a 45-degree cut has replaced (in this tile or the one across it)."""
        for (tx, ty, sx, sy) in ((x, y, dx, dy), (x + dx, y + dy, -dx, -dy)):
            c = cut.get((tx, ty))
            if c and (sx, sy) in CORNERS[c[0]][:2]:
                return True
        return False

    # 3. lines: river banks, kerbs, parapets; then the cut edges' diagonals
    bw = max(1, T // 6)
    for y in range(H):
        for x in range(W):
            k = kind[y][x]
            X, Y = x * T, y * T
            for dx, dy in N4:
                nx, ny = x + dx, y + dy
                if not p.inb(nx, ny) or hidden(x, y, dx, dy):
                    continue
                kn = kind[ny][nx]
                if k == "water" and kn not in ("water", "bridge"):
                    edge_line(dr, X, Y, T, dx, dy, WATER_EDGE, bw)
                elif k == "bridge" and kn not in ("bridge", "pave", "carr", "lane"):
                    edge_line(dr, X, Y, T, dx, dy, PARAPET, max(2, T // 4))
                elif k in ("pave", "carr") and kn in ("ground", "square") and not p.square[ny][nx]:
                    edge_line(dr, X, Y, T, dx, dy, KERB, 1)
                elif k in ("lane", "path", "dirt") and kn == "ground":
                    edge_line(dr, X, Y, T, dx, dy, shade(col[y][x], 0.8), 1)
    for (x, y), (name, kB) in cut.items():
        kA = kind[y][x]
        tri = CORNERS[name][2]
        a, b = tri[0], tri[2]  # the hypotenuse
        pa = (x * T + a[0] * T, y * T + a[1] * T)
        pb = (x * T + b[0] * T, y * T + b[1] * T)
        pair = {kA, kB}
        if "water" in pair:
            dr.line([pa, pb], fill=WATER_EDGE, width=bw)
        elif pair == {"pave", "ground"}:
            dr.line([pa, pb], fill=KERB, width=1)
    # 4. the square, its stalls and its cross
    for y in range(H):
        for x in range(W):
            if p.square[y][x] and T >= 8:
                X, Y = x * T, y * T
                dr.line([X, Y + T - 1, X + T - 1, Y + T - 1], fill=SQUARE_LINE)
                dr.line([X + T - 1, Y, X + T - 1, Y + T - 1], fill=SQUARE_LINE)
    sx0, sy0, sx1, sy1 = SQ
    st = rng(p.key, "stalls")
    for (a, b) in [(sx0 + 1, sy0 + 1), (sx0 + 3, sy0 + 1), (sx0 + 5, sy0 + 1), (sx0 + 1, sy0 + 3), (sx0 + 3, sy0 + 3), (sx0 + 6, sy0 + 4), (sx0 + 8, sy0 + 2)]:
        c = st.choice([(214, 88, 72), (72, 128, 196), (230, 180, 60), (96, 160, 96)])
        dr.rectangle([a * T + T // 5, b * T + T // 5, (a + 1) * T + T // 2, b * T + T - 2], fill=c, outline=shade(c, 0.6))
    cx, cy = (sx0 + sx1 + 1) / 2 * T, (sy0 + sy1 + 1) / 2 * T + T
    dr.ellipse([cx - T * 0.5, cy - T * 0.5, cx + T * 0.5, cy + T * 0.5], fill=(170, 160, 150), outline=(110, 100, 90))
    # 5. rail
    pts = [(a * T + T / 2, b * T + T / 2) for a, b in RAIL_PTS]
    dr.line(pts, fill=RAIL_BED, width=max(3, int(T * 0.9)), joint="curve")
    for (a, b) in zip(pts, pts[1:]):
        n = int(math.dist(a, b) / max(3, T * 0.5))
        ux, uy = (b[0] - a[0]), (b[1] - a[1])
        L = math.hypot(ux, uy) or 1
        nx, ny = -uy / L * T * 0.35, ux / L * T * 0.35
        for k in range(n + 1):
            t = k / max(1, n)
            px, py = a[0] + ux * t, a[1] + uy * t
            dr.line([px - nx, py - ny, px + nx, py + ny], fill=(110, 96, 84), width=max(1, T // 8))
    dr.line(pts, fill=RAIL, width=max(1, T // 5))
    px0, py0, px1, py1 = PLATFORM
    dr.rectangle([px0 * T, py0 * T + T // 4, (px1 + 1) * T - 1, (py1 + 1) * T - 1], fill=(200, 194, 186), outline=(140, 134, 126))
    # 6. buildings, with a short shadow to the south-east
    sh = max(1, T // 5)
    for (x0, y0, x1, y1, c, kd) in p.buildings:
        dr.rectangle([x0 * T + sh, y0 * T + sh, (x1 + 1) * T - 1 + sh, (y1 + 1) * T - 1 + sh], fill=(40, 36, 30, 70))
    for (x0, y0, x1, y1, c, kd) in p.buildings:
        dr.rectangle([x0 * T, y0 * T, (x1 + 1) * T - 1, (y1 + 1) * T - 1], fill=c, outline=shade(c, 0.58))
        if x1 - x0 >= y1 - y0:
            ym = (y0 + y1 + 1) / 2 * T
            dr.line([x0 * T + T * 0.3, ym, (x1 + 1) * T - T * 0.3, ym], fill=shade(c, 0.78), width=max(1, T // 10))
        else:
            xm = (x0 + x1 + 1) / 2 * T
            dr.line([xm, y0 * T + T * 0.3, xm, (y1 + 1) * T - T * 0.3], fill=shade(c, 0.78), width=max(1, T // 10))
        if kd == "church":
            cx, cy = (x0 + x1 + 1) / 2 * T, (y0 + y1 + 1) / 2 * T
            dr.line([cx, cy - T * 0.9, cx, cy + T * 0.9], fill=(60, 56, 70), width=max(2, T // 5))
            dr.line([cx - T * 0.55, cy - T * 0.3, cx + T * 0.55, cy - T * 0.3], fill=(60, 56, 70), width=max(2, T // 5))
    # 7. trees
    for (tx, ty, s) in p.trees:
        rr = s * T * 0.6
        dr.ellipse([tx * T - rr, ty * T - rr, tx * T + rr, ty * T + rr], fill=TREE, outline=TREE_DARK)
    return img


def edge_line(dr, X, Y, T, dx, dy, col, w):
    if dx == 1:
        dr.rectangle([X + T - w, Y, X + T - 1, Y + T - 1], fill=col)
    elif dx == -1:
        dr.rectangle([X, Y, X + w - 1, Y + T - 1], fill=col)
    elif dy == 1:
        dr.rectangle([X, Y + T - w, X + T - 1, Y + T - 1], fill=col)
    else:
        dr.rectangle([X, Y, X + T - 1, Y + w - 1], fill=col)


def text_halo(dr, xy, s, f, fill=TEXT, anchor="mm"):
    dr.text(xy, s, font=f, fill=fill, anchor=anchor, stroke_width=3, stroke_fill=(255, 255, 250))


DISTRICT_LABELS = [
    (47.5, 35.2, "OLD CORE"),
    (60, 53, "TERRACES"),
    (80, 7.0, "INDUSTRY"),
    (21, 12, "SUBURBS (HILL)"),
    (14, 46, "RURAL"),
    (86, 50, "RURAL"),
]
PLACE_LABELS = [
    (42.5, 32.0, "Market square"),
    (40.8, 45.5, "Old Bridge"),
    (88, 15.5, "Mill Bridge"),
    (8, 33.6, "railway"),
]


def label_map(img, p, T, scale=1.0, crop=None):
    dr = ImageDraw.Draw(img)
    fd = font(int(15 * scale), True)
    fp = font(int(13 * scale))
    ox, oy = (crop[0], crop[1]) if crop else (0, 0)

    def inside(x, y):
        return not crop or (crop[0] + 1 <= x <= crop[2] - 1 and crop[1] + 0.6 <= y <= crop[3] - 0.6)

    for (x, y, s) in DISTRICT_LABELS:
        if inside(x, y):
            text_halo(dr, ((x - ox) * T, (y - oy) * T), s, fd, fill=(70, 40, 40))
    for (x, y, s) in PLACE_LABELS:
        if inside(x, y):
            text_halo(dr, ((x - ox) * T, (y - oy) * T), s, fp)
    for (x, y, s, size, bold) in p.labels:
        if inside(x, y):
            text_halo(dr, ((x - ox) * T, (y - oy) * T), s, font(int(13 * scale), bold))


def wrap(dr, s, f, width):
    words = s.split()
    lines, cur = [], ""
    for w_ in words:
        t = (cur + " " + w_).strip()
        if dr.textlength(t, font=f) <= width:
            cur = t
        else:
            lines.append(cur)
            cur = w_
    if cur:
        lines.append(cur)
    return lines


def frame(body, title, caption, scale_T, legend=True, title_size=30, cap_size=19):
    """Title, caption and a 'sketch' stamp above the map; a key and scale bar below."""
    Wd = body.width
    tmp = ImageDraw.Draw(Image.new("RGB", (10, 10)))
    ft, fc, fs = font(title_size, True), font(cap_size), font(max(13, cap_size - 4), True)
    lines = wrap(tmp, caption, fc, Wd - 28)
    head = 16 + title_size + 10 + len(lines) * (cap_size + 6) + 10
    foot = 48
    img = Image.new("RGB", (Wd, head + body.height + foot), BG)
    dr = ImageDraw.Draw(img)
    dr.text((14, 12), title, font=ft, fill=TEXT)
    stamp = "SKETCH, not the generator"
    if dr.textlength(title, font=ft) + dr.textlength(stamp, font=fs) + 60 < Wd:
        dr.text((Wd - 14, 16 + title_size + 2), stamp, font=fs, fill=SKETCH_RED, anchor="rs")
    y = 16 + title_size + 10
    for ln in lines:
        dr.text((14, y), ln, font=fc, fill=(70, 66, 76))
        y += cap_size + 6
    img.paste(body, (0, head))
    dr.rectangle([0, head, Wd - 1, head + body.height - 1], outline=(120, 116, 110))
    fy = head + body.height + 10
    fk = font(max(12, cap_size - 5))
    x = 14
    if legend:
        items = [("high st.", CARRIAGE_MAIN), ("street", CARRIAGE), ("pavement", PAVEMENT), ("lane", LANE), ("track", DIRT), ("green", GREEN), ("water", WATER)]
        for (name, col) in items:
            dr.rectangle([x, fy + 2, x + 18, fy + 20], fill=col, outline=(110, 106, 100))
            dr.text((x + 24, fy + 11), name, font=fk, fill=TEXT, anchor="lm")
            x += 24 + int(dr.textlength(name, font=fk)) + 16
    # scale bar: 10 tiles
    bx1 = Wd - 14
    bx0 = bx1 - 10 * scale_T
    if bx0 > x + 10:
        dr.rectangle([bx0, fy + 8, bx1, fy + 13], fill=TEXT)
        dr.text(((bx0 + bx1) / 2, fy + 28), "10 tiles", font=fk, fill=TEXT, anchor="mm")
    if not legend or bx0 <= x + 10:
        dr.text((14, fy + 4), stamp, font=fs, fill=SKETCH_RED)
    return img


OPTIONS = [
    ("1-today", "Today (v2 stage A)", "Each district is cut into rectangles by straight streets that run edge to edge (3 to 4 tiles wide); arterials step across a 4-tile grid; the square has a ring road. Every district reads as a grid.", today, False),
    ("2-option-a", "Option A: grown streets, right angles only", "Main streets grow from the square along desire lines to the bridges, the station and the neighbouring towns; side streets branch off them. T-junctions and offset crossings, L- and T-shaped blocks, a 4-tile high street, 1 to 2-tile lanes in the old core, small greens at junctions, cul-de-sacs on the hill, varied setbacks. All streets axis-aligned.", option_a, True),
    ("3-option-b", "Option B: A plus 45-degree streets", "As A, plus 45-degree streets: radial roads from the square, a high street slanting along the river to the Mill Bridge, and hill streets following the contours. Buildings stay square to the grid and step along the diagonals (staircase frontage).", option_b, True),
]
CROP = (27, 19, 57, 39)  # x0, y0, x1 (excl), y1 (excl): 30 x 20 tiles of the old core


def main():
    out = sys.argv[1] if len(sys.argv) > 1 else os.path.dirname(os.path.abspath(__file__))
    os.makedirs(out, exist_ok=True)
    geo = Geo()
    maps = []
    for (slug, title, caption, fn, grown) in OPTIONS:
        p = finish(fn(geo), grown)
        # overview: 10 px per tile (2x the 5 px base)
        T = 10
        m = render_map(p, T)
        label_map(m, p, T)
        img = frame(m, title, caption, T)
        path = os.path.join(out, f"streets-{slug}.png")
        img.save(path, optimize=True)
        maps.append(img)
        # close-up of the old core: 20 px per tile (4x)
        T2 = 20
        big = render_map(p, T2)
        x0, y0, x1, y1 = CROP
        crop = big.crop((x0 * T2, y0 * T2, x1 * T2, y1 * T2))
        label_map(crop, p, T2, scale=1.25, crop=CROP)
        img2 = frame(crop, title.split(":")[0] + ": old core close-up", "30 x 20 tiles around the market square, 20 px per tile. " + caption.split(".")[0] + ".", T2, legend=False, title_size=24, cap_size=16)
        img2.save(os.path.join(out, f"streets-closeup-{slug}.png"), optimize=True)
        print("wrote", path)
    # all three, stacked for a phone
    gap = 16
    Wd = max(m.width for m in maps)
    head = 56
    allimg = Image.new("RGB", (Wd, head + sum(m.height for m in maps) + gap * (len(maps) - 1)), (232, 228, 220))
    dr = ImageDraw.Draw(allimg)
    dr.text((14, 14), "Street layout options (sketches, not the generator)", font=font(26, True), fill=TEXT)
    y = head
    for m in maps:
        allimg.paste(m, (0, y))
        y += m.height + gap
    allimg.save(os.path.join(out, "streets-all.png"), optimize=True)
    print("wrote", os.path.join(out, "streets-all.png"))


if __name__ == "__main__":
    main()
