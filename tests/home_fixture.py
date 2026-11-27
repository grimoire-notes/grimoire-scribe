#!/usr/bin/env python3
"""A populated Library for the home's scenario (tests/scenarios/z_home_ruled.qs, #332).

Plants, in the data root given:
- five installed app packages (Calculator, Chess, Sudoku Puzzles, Weather, Word Search: one name too long for
  one line under a Gothic square), each with a 64 px icon drawn
  here (Word Search takes the apppkg fixture's lettered icon) and the apppkg fixture's module;
- three folders: Archive (the plain folder), School (the school icon) and Travel (a picture from photos/, drawn
  here as a small landscape);
- notebooks with handwriting-like ink on different papers: seven at the top level (so the upright home's two rows
  of four show Page 1 of 2) and some in each folder.

The ink is made of strokes as the editor writes them (docs/FORMAT.md): a cursive-like line of loops for writing,
straight strokes for boxes, ellipses for a mind map's nodes. Deterministic. Usage: home_fixture.py ROOT
"""
import hashlib
import json
import math
import os
import struct
import sys
import zlib

root = sys.argv[-1]
here = os.path.dirname(os.path.abspath(__file__))
W, H = 1860, 2480
MM = 300.0 / 25.4
seed = 0x5EED


def rnd():
    global seed
    seed = (seed * 1664525 + 1013904223) & 0xFFFFFFFF
    return seed >> 8


def frand(lo, hi):
    return lo + (hi - lo) * (rnd() % 10001) / 10000.0


# ---- strokes ----

def stroke(tool, pts, size=(0.3, 0.6)):
    params = [size[0], size[1], 0.8, 1.0, 0.0, 1.0, 1.0, 0.0, 70.0, 0.0, 0.0, 0.5, 0.5]
    body = bytearray()
    for i, (x, y, p) in enumerate(pts):
        body += struct.pack('<ffHhhI', x, y, int(max(0.05, min(1.0, p)) * 65535), 0, 0, i * 6)
    return struct.pack('<B3sI13fI', tool, b'\0\0\0', rnd() & 0xFFFFFF, *params, len(pts)) + bytes(body)


def page(paper, strokes):
    return struct.pack('<4sHHIII', b'QPG1', 1, paper, W, H, len(strokes)) + b''.join(strokes)


def word(x, base, xh, n, tool=2, size=(0.3, 0.6)):
    """One cursive-like word of n letters from x on the baseline: loops of a prolate cycloid, some letters tall,
    some below the line. Returns (stroke, x after it)."""
    a = xh * 0.62
    r = xh * 0.24
    pts = [(x - a * 0.4, base + 2, 0.4)]
    for li in range(n):
        k = rnd() % 20
        if k < 3:
            h = xh * frand(1.9, 2.3)       # an ascender (l, h, k)
        elif k < 5:
            h = -xh * frand(1.2, 1.5)      # a descender (g, y, p)
        else:
            h = xh * frand(0.85, 1.1)
        jitter = frand(-2.0, 2.0)
        for s in range(1, 13):
            t = s / 12.0
            yy = base + jitter - h * (1 - math.cos(2 * math.pi * t)) / 2
            xx = x + a * (li + t) - r * math.sin(2 * math.pi * t) + (base - yy) * 0.22
            pts.append((xx, yy, 0.55 + 0.25 * math.sin(li + t * 3)))
    end = x + a * n
    pts.append((end + a * 0.3, base - xh * 0.2, 0.35))
    return stroke(tool, pts, size), end


def writing(x0, x1, base, xh, ragged=True, tool=2, size=(0.3, 0.6)):
    """A line of words between x0 and x1 on the baseline; the last line of a paragraph stops early."""
    out = []
    x = x0
    stop = x1 if not ragged else x1 - frand(0, (x1 - x0) * 0.45)
    while True:
        n = 2 + rnd() % 7
        if x + n * xh * 0.62 > stop:
            break
        s, x = word(x, base, xh, n, tool, size)
        out.append(s)
        x += xh * frand(0.8, 1.1)
    return out


def paragraph(x0, x1, first_base, pitch, lines, xh, tool=2):
    out = []
    for i in range(lines):
        out += writing(x0, x1, first_base + i * pitch, xh, ragged=(i == lines - 1), tool=tool)
    return out


def line(x0, y0, x1, y1, tool=6, size=(0.35, 0.5), n=None):
    n = n or max(2, int(math.hypot(x1 - x0, y1 - y0) / 12))
    pts = [(x0 + (x1 - x0) * i / (n - 1) + frand(-0.6, 0.6), y0 + (y1 - y0) * i / (n - 1) + frand(-0.6, 0.6), 0.7)
           for i in range(n)]
    return stroke(tool, pts, size)


def box(x, y, w, h, tool=6, size=(0.35, 0.5)):
    pts = []
    corners = [(x, y), (x + w, y), (x + w, y + h), (x, y + h), (x, y + 4)]
    for (ax, ay), (bx, by) in zip(corners, corners[1:]):
        n = max(2, int(math.hypot(bx - ax, by - ay) / 14))
        for i in range(n):
            pts.append((ax + (bx - ax) * i / n + frand(-0.8, 0.8), ay + (by - ay) * i / n + frand(-0.8, 0.8), 0.7))
    return stroke(tool, pts, size)


def ellipse(cx, cy, rx, ry, tool=6, size=(0.35, 0.5), turns=1.08):
    n = int(2 * math.pi * max(rx, ry) / 10)
    a0 = frand(0, 6.28)
    pts = []
    for i in range(int(n * turns) + 1):
        a = a0 + 2 * math.pi * i / n
        k = 1 + 0.03 * math.sin(a * 3)
        pts.append((cx + rx * k * math.cos(a), cy + ry * k * math.sin(a), 0.7))
    return stroke(tool, pts, size)


def check(x, y, s):
    return stroke(2, [(x, y + s * 0.5, 0.5), (x + s * 0.35, y + s * 0.9, 0.8), (x + s * 1.1, y - s * 0.2, 0.6)])


def title(x, y, n_words=2, xh=34):
    out = []
    for _ in range(n_words):
        s, x = word(x, y, xh, 3 + rnd() % 4, 5, (0.5, 0.9))
        out.append(s)
        x += xh * 1.1
    return out, x


# ---- the notebooks' pages ----

RULE_FIRST = 22 * MM
MARGIN_X = 25 * MM


def ruled_page(paper, pitch_mm, paras, margin=True):
    pitch = pitch_mm * MM
    x0 = MARGIN_X + 45 if margin else 120
    ink, _ = title(x0, RULE_FIRST - 26, 2, pitch * 0.42)
    i = 2
    for lines in paras:
        ink += paragraph(x0, W - 120, RULE_FIRST + pitch * i - 5, pitch, lines, pitch * 0.36)
        i += lines + 1
    return ink


def book_notes():
    ink = ruled_page(1, 7.1, [4, 6, 3, 5])
    # an underline under the title and a star by a paragraph
    ink.append(line(MARGIN_X + 45, RULE_FIRST - 10, MARGIN_X + 520, RULE_FIRST - 14, 2, (0.35, 0.6)))
    pts = [(MARGIN_X - 140 + 40 * math.cos(math.pi / 2 + k * 4 * math.pi / 5), RULE_FIRST + 7.1 * MM * 9 - 30
            - 40 * math.sin(math.pi / 2 + k * 4 * math.pi / 5), 0.7) for k in range(6)]
    ink.append(stroke(2, pts))
    return page(1, ink)


def daily_planner():
    g = 5 * MM
    ink, _ = title(2 * g + 20, 4 * g, 2, 40)
    ink += writing(W - 9 * g, W - 2 * g, 4 * g, 26, ragged=False)
    for i in range(9):
        y = 7 * g + i * 2 * g
        ink.append(box(2 * g, y - 0.8 * g, 0.8 * g, 0.8 * g, 2, (0.3, 0.5)))
        if i in (0, 1, 3, 6):
            ink.append(check(2 * g + 8, y - 0.8 * g + 6, 0.6 * g))
        ink += writing(3.4 * g, W - 3 * g, y - 4, 22)
    # a notes box at the foot
    ink.append(box(2 * g, 27 * g, W - 4 * g, 10 * g, 6))
    ink += writing(2 * g + 30, W / 2, 28 * g + 20, 22, ragged=False)
    ink += paragraph(2 * g + 30, W - 3 * g, 30 * g, 1.5 * g, 4, 22)
    return page(10, ink)


def garden_plan():
    g = 5 * MM
    ink, _ = title(2 * g, 3 * g, 2, 40)
    beds = [(2, 6, 12, 6), (16, 6, 12, 6), (2, 15, 26, 5), (2, 23, 8, 12), (13, 23, 15, 12)]
    for (bx, by, bw, bh) in beds:
        ink.append(box(bx * g, by * g, bw * g, bh * g, 6, (0.4, 0.6)))
        cols, rows = max(1, int(bw / 3)), max(1, int(bh / 3))
        for cx in range(cols):
            for cy in range(rows):
                ink.append(ellipse((bx + 1.5 + cx * 3) * g, (by + 1.5 + cy * 3) * g, 0.7 * g, 0.7 * g, 2, (0.3, 0.5)))
        ink += writing(bx * g + 6, (bx + bw) * g, (by + bh) * g + 0.9 * g, 18, ragged=False)
    # a north arrow
    ink.append(line(29.5 * g, 4 * g, 29.5 * g, 1.2 * g, 6))
    ink.append(stroke(6, [(28.8 * g, 2 * g, 0.7), (29.5 * g, 1.2 * g, 0.7), (30.2 * g, 2 * g, 0.7)]))
    return page(11, ink)


def ideas():
    ink, _ = title(140, 220, 1, 40)
    cx, cy = W / 2, 1150
    ink.append(ellipse(cx, cy, 280, 150, 5, (0.5, 0.8)))
    s, _ = word(cx - 150, cy + 12, 38, 6, 5, (0.5, 0.8))
    ink.append(s)
    for k in range(6):
        a = -math.pi / 2 + k * math.pi / 3 + 0.25
        ex, ey = cx + 300 * math.cos(a), cy + 165 * math.sin(a)
        nx, ny = cx + 540 * math.cos(a), cy + 600 * math.sin(a)
        ink.append(line(ex, ey, nx, ny, 6))
        ink.append(ellipse(nx, ny, 170, 85, 6))
        s, _ = word(nx - 110, ny + 10, 26, 4 + k % 3)
        ink.append(s)
        # a few leaves fanned under two of the nodes
        if k in (1, 4):
            for j in range(3):
                lx, ly = nx - 170 + 170 * j, ny + 250
                ink.append(line(nx - 60 + 60 * j, ny + 85, lx, ly - 40, 6))
                ink += writing(lx - 70, lx + 90, ly, 20, ragged=False)
    return page(4, ink)


def meeting_notes():
    pitch = 7.1 * MM
    cue = 55 * MM
    ink, _ = title(cue + 40, RULE_FIRST - 26, 2, 30)
    y = RULE_FIRST + pitch * 2 - 5
    for block in (4, 3, 5, 3):
        ink += writing(40, cue - 40, y, 24, ragged=False)
        for i in range(block):
            ink += writing(cue + 40, W - 100, y + i * pitch, 25, ragged=(i == block - 1))
        y += (block + 1) * pitch
    summ = H - 52 * MM
    ink += paragraph(80, W - 120, summ + pitch * 1.4, pitch, 3, 25)
    return page(12, ink)


def algebra():
    g = 5 * MM
    ink, _ = title(2 * g, 3 * g, 1, 40)
    for i in range(6):
        ink += writing(2 * g, 16 * g, (6 + 2 * i) * g, 24, ragged=False)
    ox, oy = 23 * g, 14 * g
    ink.append(line(17 * g, oy, 30 * g, oy, 6))
    ink.append(line(ox, 21 * g, ox, 6 * g, 6))
    pts = [(ox + t * g, oy - (t * t / 3.0 - 4) * g, 0.7) for t in [x / 4.0 for x in range(-22, 23)]]
    ink.append(stroke(2, [p for p in pts if 6 * g < p[1] < 21 * g]))
    for i in range(7):
        ink += writing(2 * g, W - 2 * g, (24 + 2 * i) * g, 24, ragged=(i == 6))
    return page(11, ink)


def biology():
    ink = ruled_page(0, 5.5, [3])
    cx, cy = W / 2, 1350
    ink.append(ellipse(cx, cy, 520, 380, 5, (0.5, 0.8)))
    ink.append(ellipse(cx - 60, cy + 20, 150, 120, 6))
    ink.append(ellipse(cx - 60, cy + 20, 50, 40, 6))
    for k in range(5):
        a = k * 1.3 + 0.5
        ink.append(ellipse(cx + 300 * math.cos(a), cy + 220 * math.sin(a), 60, 28, 6))
    for k, (lx, ly) in enumerate([(cx + 560, cy - 300), (cx + 600, cy + 120), (cx - 900, cy - 380)]):
        ink.append(line(cx + (250 if lx > cx else -400), cy - 100 + 120 * k, lx - (20 if lx > cx else -260), ly, 6))
        ink += writing(lx - (0 if lx > cx else 0), lx + 260, ly + 10, 20, ragged=False)
    ink += paragraph(MARGIN_X + 45, W - 120, RULE_FIRST + 5.5 * MM * 26 - 5, 5.5 * MM, 6, 20)
    return page(0, ink)


def history_essay():
    return page(2, ruled_page(2, 9.0, [5, 5, 4]))


def plain_writing(paper, pitch_mm, paras, margin=True):
    return page(paper, ruled_page(paper, pitch_mm, paras, margin))


# ---- the notebooks ----

def notebook(ident, name, folder, paper, data, created, modified):
    book = os.path.join(root, 'notebooks', ident)
    os.makedirs(book, exist_ok=True)
    with open(os.path.join(book, 'notebook.ini'), 'w', encoding='utf-8') as f:
        f.write(f'# Grimoire notebook\nname={name}\nfolder={folder}\npaper={paper}\ncreated={created}\n'
                f'modified={modified}\nlast_page=0\nnext_page_id=2\npage_w={W}\npage_h={H}\npages=1\n')
    with open(os.path.join(book, 'p1.qpg'), 'wb') as f:
        f.write(data)


NOTEBOOKS = [
    # id, name, folder, paper, page, modified (created a few weeks before)
    ('book-notes', 'Book notes', '', 1, book_notes, 1795723620),
    ('daily-planner', 'Daily planner', '', 10, daily_planner, 1795772460 - 3600),
    ('garden-plan', 'Garden plan', '', 11, garden_plan, 1795617120),
    ('ideas', 'Ideas', '', 4, ideas, 1795508100),
    ('meeting-notes', 'Meeting notes', '', 12, meeting_notes, 1795197780),
    ('piano-practice', 'Piano practice', '', 6, lambda: plain_writing(6, 7.1, [3, 3], False), 1794484080),
    ('reading-list', 'Reading list', '', 2, lambda: plain_writing(2, 9.0, [8]), 1793377200),
    ('algebra', 'Algebra', 'School', 11, algebra, 1795680000),
    ('biology', 'Biology', 'School', 0, biology, 1795400000),
    ('history-essay', 'History essay', 'School', 2, history_essay, 1794900000),
    ('lisbon', 'Lisbon trip', 'Travel', 4, lambda: plain_writing(4, 7.1, [4, 4], False), 1790931900),
    ('packing', 'Packing list', 'Travel', 10, daily_planner, 1790900000),
    ('journal-2025', 'Journal 2025', 'Archive', 1, lambda: plain_writing(1, 7.1, [6, 6]), 1789392600),
]

for ident, name, folder, paper, make, modified in NOTEBOOKS:
    notebook(ident, name, folder, paper, make(), modified - 21 * 86400, modified)

with open(os.path.join(root, 'folders.ini'), 'w', encoding='utf-8') as f:
    f.write('# Grimoire folders\nArchive\nSchool\nTravel\n')
with open(os.path.join(root, 'folder-looks.ini'), 'w', encoding='utf-8') as f:
    f.write('# Grimoire: folder icons and pictures\nfolder=School\nicon=school\nfolder=Travel\npicture=lake.png\n')


# ---- the Travel folder's picture: a small landscape, gray ----

def png_gray(path, w, h, px):
    raw = b''.join(b'\0' + bytes(px[y * w:(y + 1) * w]) for y in range(h))

    def chunk(tag, data):
        return struct.pack('>I', len(data)) + tag + data + struct.pack('>I', zlib.crc32(tag + data) & 0xFFFFFFFF)
    with open(path, 'wb') as f:
        f.write(b'\x89PNG\r\n\x1a\n' + chunk(b'IHDR', struct.pack('>IIBBBBB', w, h, 8, 0, 0, 0, 0))
                + chunk(b'IDAT', zlib.compress(raw, 9)) + chunk(b'IEND', b''))


def landscape(w=480, h=640):
    px = bytearray(w * h)
    horizon = int(h * 0.64)
    trees = []
    for x in range(-10, w + 20, 19):
        if rnd() % 3:
            trees.append((x + rnd() % 9, horizon + 2 - (rnd() % 14), 26 + rnd() % 44))
    for y in range(h):
        for x in range(w):
            if y < horizon:
                v = 190 + 55 * y / horizon
                d = math.hypot(x - w * 0.68, y - h * 0.17)
                if d < 30:
                    v = 254
                elif d < 52:
                    v = max(v, 254 - (d - 30) * 1.6)
                far = h * 0.36 + 34 * math.sin(x / 47.0) + 16 * math.sin(x / 19.0 + 1.3)
                near = h * 0.50 + 40 * math.sin(x / 83.0 + 2.0) + 12 * math.sin(x / 27.0)
                if y > far:
                    v = 160 - (y - far) * 0.15
                if y > near:
                    v = 105 - (y - near) * 0.25
            else:
                k = (y - horizon) / (h - horizon)
                v = 215 - 60 * k
                # the hills' reflection near the shore, broken by ripples
                if k < 0.3 and math.sin(x / 13.0 + y * 0.45) > 0.1:
                    v -= 60 * (0.3 - k) / 0.3
                if int(y * 0.35) % 5 == 0 and math.sin(x / 31.0 + y) > 0.6:
                    v += 25
            for tx, ty, th in trees:
                if ty - th < y < ty and abs(x - tx) < (y - (ty - th)) * 0.26:
                    v = 30
            px[y * w + x] = max(0, min(255, int(v)))
    return w, h, px


os.makedirs(os.path.join(root, 'photos'), exist_ok=True)
lw, lh, lpx = landscape()
png_gray(os.path.join(root, 'photos', 'lake.png'), lw, lh, lpx)


# ---- the apps ----

def raster(shape, n=64, ss=4):
    """A 64 x 64 icon from shape(x, y) -> ink 0..1 (coordinates 0..64), 4 x 4 samples a pixel, as BGRA rows from the
    top: black ink with its coverage as alpha (the worker composites it over white)."""
    out = []
    for py in range(n):
        row = []
        for px_ in range(n):
            acc = 0.0
            for sy in range(ss):
                for sx in range(ss):
                    acc += shape(px_ + (sx + 0.5) / ss, py + (sy + 0.5) / ss)
            a = acc / (ss * ss)
            a = round(a * 3) / 3.0  # (four levels: crisp at twice the size on the panel)
            row.append(int(a * 255))
        out.append(row)
    return out


def ico(alpha_rows):
    n = len(alpha_rows)
    pixels = bytearray()
    for row in reversed(alpha_rows):
        for a in row:
            pixels += bytes((0, 0, 0, a))
    mask = bytes(n * ((n + 31) // 32 * 4))
    dib = struct.pack('<IiiHHIIiiII', 40, n, 2 * n, 1, 32, 0, len(pixels) + len(mask), 0, 0, 0, 0) + bytes(pixels) + mask
    return struct.pack('<HHH', 0, 1, 1) + struct.pack('<BBBBHHII', n, n, 0, 0, 1, 32, len(dib), 22) + dib


def in_rect(x, y, x0, y0, x1, y1):
    return x0 <= x < x1 and y0 <= y < y1


def in_round(x, y, x0, y0, x1, y1, r):
    if not in_rect(x, y, x0, y0, x1, y1):
        return False
    cx = min(max(x, x0 + r), x1 - r)
    cy = min(max(y, y0 + r), y1 - r)
    return (x - cx) ** 2 + (y - cy) ** 2 <= r * r


def calculator(x, y):
    if not in_round(x, y, 13, 5, 51, 59, 6):
        return 0
    if in_round(x, y, 17, 9, 47, 55, 3):
        if in_rect(x, y, 20, 12, 44, 23):
            return 1  # the display
        for c in range(4):
            for r in range(4):
                kx, ky = 20 + c * 6.3, 27 + r * 6.6
                if in_rect(x, y, kx, ky, kx + 4.6, ky + 4.8):
                    return 1
        return 0
    return 1


def rook(x, y):
    parts = [in_round(x, y, 13, 50, 51, 57, 1.5), in_rect(x, y, 17, 45, 47, 51),
             in_rect(x, y, 22 - (y - 22) * 0.1, 22, 42 + (y - 22) * 0.1, 46), in_rect(x, y, 17, 16, 47, 23)]
    for mx in (17, 28.5, 40):
        parts.append(in_rect(x, y, mx, 8, mx + 7, 17))
    return 1 if any(parts) else 0


def weather(x, y):
    circles = [(30, 42, 9), (42, 37, 11), (53, 44, 7.5)]

    def cloud(x, y, grow):
        if any((x - cx) ** 2 + (y - cy) ** 2 <= (r + grow) ** 2 for cx, cy, r in circles):
            return True
        return in_rect(x, y, 30 - grow, 42 - grow, 53 + grow, 51.5 + grow)
    if cloud(x, y, 0):
        return 0 if cloud(x, y, -3.2) else 1
    if cloud(x, y, 2.5):
        return 0
    d = math.hypot(x - 24, y - 24)
    if d <= 10:
        return 1
    a = math.atan2(y - 24, x - 24)
    k = round(a / (math.pi / 4)) * (math.pi / 4)
    off = abs(math.sin(a - k)) * d
    return 1 if 13.5 <= d <= 20 and off <= 1.6 else 0


DIGITS = {'1': ['010', '110', '010', '010', '111'], '3': ['111', '001', '011', '001', '111'],
          '5': ['111', '100', '111', '001', '111'], '7': ['111', '001', '010', '010', '010'],
          '9': ['111', '101', '111', '001', '111'], '4': ['101', '101', '111', '001', '001']}
SUDOKU = {(0, 0): '5', (1, 1): '3', (2, 0): '7', (0, 2): '9', (2, 2): '1', (1, 0): '4'}


def sudoku(x, y):
    if not in_rect(x, y, 6, 6, 58, 58):
        return 0
    if not in_rect(x, y, 9, 9, 55, 55):
        return 1
    for k in (1, 2):
        if abs(x - (9 + 46 * k / 3)) < 1.3 or abs(y - (9 + 46 * k / 3)) < 1.3:
            return 1
    cell = 46 / 3
    c, r = int((x - 9) // cell), int((y - 9) // cell)
    d = SUDOKU.get((c, r))
    if d:
        gx, gy = x - (9 + c * cell + cell / 2 - 4.5), y - (9 + r * cell + cell / 2 - 7.5)
        if 0 <= gx < 9 and 0 <= gy < 15:
            return 1 if DIGITS[d][int(gy // 3)][int(gx // 3)] == '1' else 0
    return 0


fixture_app = os.path.join(here, 'fixtures/apppkg/packages/apps/com.example.fixture')
with open(os.path.join(fixture_app, 'app.wasm'), 'rb') as f:
    wasm = f.read()
with open(os.path.join(fixture_app, 'icon.ico'), 'rb') as f:
    lettered = f.read()
wasm_sha = hashlib.sha256(wasm).hexdigest()

APPS = [
    ('com.example.calculator', 'Calculator', 'A four-function calculator.', ico(raster(calculator))),
    ('com.example.chess', 'Chess', 'Chess against the Kindle, or a friend.', ico(raster(rook))),
    ('com.example.sudoku', 'Sudoku Puzzles', 'Daily number puzzles.', ico(raster(sudoku))),
    ('com.example.weather', 'Weather', 'The forecast for your saved places.', ico(raster(weather))),
    ('com.example.wordsearch', 'Word Search', 'Find the hidden words.', lettered),
]
index = []
for i, (ident, name, desc, icon) in enumerate(APPS):
    d = os.path.join(root, 'packages', 'apps', ident)
    os.makedirs(d, exist_ok=True)
    with open(os.path.join(d, 'app.wasm'), 'wb') as f:
        f.write(wasm)
    with open(os.path.join(d, 'icon.ico'), 'wb') as f:
        f.write(icon)
    with open(os.path.join(d, 'package.yaml'), 'w', encoding='utf-8') as f:
        f.write(f'name: {name}\nid: {ident}\ntype: app\nversion: 1.0.0\nauthor: Grimoire tests\ndescription: {desc}\n'
                f'license: MIT\nmin_grimoire_version: 2.3.0\nplatforms: [all]\nartifact:\n  path: app.wasm\n'
                f'  sha256: {wasm_sha}\n  size: {len(wasm)}\ncapabilities: [storage:app-local]\nspec:\n  runtime: wasm\n'
                f'  entry: _start\n  icon: icon.ico\n  icon_sha256: {hashlib.sha256(icon).hexdigest()}\n'
                f'  icon_size: {len(icon)}\n  status_bar: false\n')
    index.append({'id': ident, 'name': name, 'version': '1.0.0', 'author': 'Grimoire tests', 'description': desc,
                  'icon': 'icon.ico', 'path': f'packages/apps/{ident}/app.wasm', 'sha': wasm_sha,
                  'installed_at': i + 1, 'source_url': ''})
with open(os.path.join(root, 'packages', 'apps-index.json'), 'w', encoding='utf-8') as f:
    json.dump({'version': 1, 'packages': index}, f, indent=2)
with open(os.path.join(root, 'settings.ini'), 'a', encoding='utf-8') as f:
    f.write('\nmirror_pdf=0\n')
