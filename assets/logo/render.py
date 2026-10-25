"""Renders the placeholder "island wave" logo (PLAN.md D27) without external tools."""
import struct
import sys
import zlib


def rounded_rect(x, y, x0, y0, x1, y1, r):
    cx = min(max(x, x0 + r), x1 - r)
    cy = min(max(y, y0 + r), y1 - r)
    return (x - cx) ** 2 + (y - cy) ** 2 <= r * r and x0 <= x <= x1 and y0 <= y <= y1


def bars(u, v):
    for i, h in enumerate((0.10, 0.17, 0.24)):
        cx = 0.40 + i * 0.10
        if rounded_rect(u, v, cx - 0.032, 0.5 - h / 2, cx + 0.032, 0.5 + h / 2, 0.032):
            return True
    return False


def app_pixel(u, v):
    if not rounded_rect(u, v, 0.1, 0.1, 0.9, 0.9, 0.18):
        return (0, 0, 0, 0)
    if rounded_rect(u, v, 0.22, 0.32, 0.78, 0.68, 0.18):
        return (24, 24, 27, 255) if bars(u, v) else (250, 249, 247, 255)
    return (24, 24, 27, 255)


def template_pixel(u, v):
    outer = rounded_rect(u, v, 0.04, 0.22, 0.96, 0.78, 0.28)
    inner = rounded_rect(u, v, 0.12, 0.30, 0.88, 0.70, 0.20)
    return (0, 0, 0, 255) if (outer and not inner) or bars(u, v) else (0, 0, 0, 0)


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
