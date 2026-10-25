"""Renders the lyre logo (PLAN.md D89) without external tools."""
import re
import struct
import sys
import zlib

# SVG paths on a 32-unit grid with their stroke widths; Logo.vue draws the same paths.
STROKES = (
    ("M 7.5 3.5 C 9.5 4 10.5 5.5 10.5 7.5 C 10.5 12 7.5 13.5 7.5 18.5 C 7.5 23.5 10.5 27.5 16 27.5 "
     "C 21.5 27.5 24.5 23.5 24.5 18.5 C 24.5 13.5 21.5 12 21.5 7.5 C 21.5 5.5 22.5 4 24.5 3.5", 2.4),
    ("M 7.5 9 L 24.5 9 M 11.5 22.5 L 20.5 22.5", 2.4),
    ("M 13 9 L 13 22 M 16 9 L 16 22 M 19 9 L 19 22", 1.3),
)


def segments():
    out = []
    for d, width in STROKES:
        toks = re.findall(r"[MLC]|[-\d.]+", d)
        i = 0
        while i < len(toks):
            n = 6 if toks[i] == "C" else 2
            v = [float(t) for t in toks[i + 1:i + 1 + n]]
            if toks[i] == "M":
                pen = v
            elif toks[i] == "L":
                out.append((*pen, *v, width / 2))
                pen = v
            else:
                p = (pen, v[0:2], v[2:4], v[4:6])
                for k in range(1, 17):
                    t = k / 16
                    q = [(1 - t) ** 3 * p[0][j] + 3 * (1 - t) ** 2 * t * p[1][j] + 3 * (1 - t) * t * t * p[2][j]
                         + t ** 3 * p[3][j] for j in (0, 1)]
                    out.append((*pen, *q, width / 2))
                    pen = q
            i += 1 + n
    return out


def bucket(segs):
    return [[[s for s in segs
              if min(s[0], s[2]) - s[4] <= x + 1 and max(s[0], s[2]) + s[4] >= x
              and min(s[1], s[3]) - s[4] <= y + 1 and max(s[1], s[3]) + s[4] >= y]
             for x in range(32)] for y in range(32)]


CELLS = bucket(segments())


def lyre(x, y):
    if not (0 <= x < 32 and 0 <= y < 32):
        return False
    for ax, ay, bx, by, r in CELLS[int(y)][int(x)]:
        dx, dy = bx - ax, by - ay
        t = min(max(((x - ax) * dx + (y - ay) * dy) / (dx * dx + dy * dy), 0), 1)
        if (x - ax - t * dx) ** 2 + (y - ay - t * dy) ** 2 <= r * r:
            return True
    return False


def rounded_rect(x, y, x0, y0, x1, y1, r):
    cx = min(max(x, x0 + r), x1 - r)
    cy = min(max(y, y0 + r), y1 - r)
    return (x - cx) ** 2 + (y - cy) ** 2 <= r * r and x0 <= x <= x1 and y0 <= y <= y1


def app_pixel(u, v):
    if not rounded_rect(u, v, 0.1, 0.1, 0.9, 0.9, 0.18):
        return (0, 0, 0, 0)
    if lyre(16 + (u - 0.5) * 53, 16 + (v - 0.5) * 53):
        return (250, 249, 247, 255)
    return (24, 24, 27, 255)


def template_pixel(u, v):
    return (0, 0, 0, 255) if lyre(16 + (u - 0.5) * 30, 15.5 + (v - 0.5) * 30) else (0, 0, 0, 0)


def render(size, pixel, ss=4):
    rows = []
    for y in range(size):
        row = bytearray([0])
        for x in range(size):
            acc = [0, 0, 0, 0]
            for sy in range(ss):
                for sx in range(ss):
                    p = pixel((x + (sx + 0.5) / ss) / size, (y + (sy + 0.5) / ss) / size)
                    for k in range(3):
                        acc[k] += p[k] * p[3]
                    acc[3] += p[3]
            a = acc[3] / (ss * ss)
            rgb = [int(acc[k] / acc[3]) if acc[3] else 0 for k in range(3)]
            row += bytes(rgb + [int(a)])
        rows.append(bytes(row))
    raw = b"".join(rows)

    def chunk(kind, data):
        return struct.pack(">I", len(data)) + kind + data + struct.pack(">I", zlib.crc32(kind + data))

    return (b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", struct.pack(">IIBBBBB", size, size, 8, 6, 0, 0, 0))
            + chunk(b"IDAT", zlib.compress(raw, 9)) + chunk(b"IEND", b""))


if __name__ == "__main__":
    out = sys.argv[1]
    open(f"{out}/app-icon.png", "wb").write(render(1024, app_pixel, ss=2))
    open(f"{out}/tray.png", "wb").write(render(18, template_pixel, ss=8))
    open(f"{out}/tray@2x.png", "wb").write(render(36, template_pixel, ss=8))
