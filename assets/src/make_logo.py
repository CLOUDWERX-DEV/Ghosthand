"""Draws the Ghosthand mascot, a pixel ghost with a cursor on its chest, as SVG.
Run `python3 make_logo.py` here: it writes ../icon.svg and banner.svg. Render PNGs with rsvg-convert."""
import pathlib

PAL = {
    "G": "#cdf4ff", "g": "#9fe6f7", "d": "#58bfd9",   # ghost: body, shade, deep shade
    "W": "#ffffff", "P": "#1c3dff",                    # eye white, pupil
    "T": "#a45cff", "t": "#7b3ad8", "k": "#4d1f9e",   # tee: body, shade, collar
}
BODY = [
    "......GGGGGGGG......",  # 0
    "....GGGGGGGGGGGG....",  # 1
    "...GGWWGGGGGGGGGg...",  # 2
    "..GGWGGGGGGGGGGGGg..",  # 3
    ".GGGWWWWGGGGWWWWGGg.",  # 4
    ".GGGWWWWGGGGWWWWGGgd",  # 5
    ".GGGWWWWGGGGWWWWGGgd",  # 6
    ".GGGWWPPGGGGWWPPGGgd",  # 7
    ".GGGWWPPGGGGWWPPGGgd",  # 8
    ".GGGGGGGGGGGGGGGGGgd",  # 9
    ".GGGGGGGGGGGGGGGGGgd",  # 10
    ".GGGGGGGGGGGGGGGGGG.",  # 11
    ".GGGGGGGGGGGGGGGGGG.",  # 12
    ".GGGGGGGGGGGGGGGGGG.",  # 13
    ".GGGGGGGGGGGGGGGGGG.",  # 14
    ".GGGGGGGGGGGGGGGGGG.",  # 15
    ".GGGGGGGGGGGGGGGGGG.",  # 16
    ".GGGGGGGGGGGGGGGGGG.",  # 17
    ".GGGGGGGGGGGGGGGGGG.",  # 18
    ".GGGGGGGGGGGGGGGGGgd",  # 21  wavy skirt
    ".GGGG..GGGGGG..GGGG.",  # 22
    ".GG......GGGG....GG.",  # 23
]
CURSOR = [
    "X....",
    "XX...",
    "XXX..",
    "XXXX.",
    "XXXXX",
    "XXX..",
    "X.XX.",
]
CURSOR_AT = (8, 11)  # (col, row) of the cursor's top-left, on the chest
GHOST = {PAL["G"], PAL["g"], PAL["d"]}


def mascot_rects(px, ox=0, oy=0):
    cells = {}
    for y, row in enumerate(BODY):
        row = row.rstrip("d") + "." * (len(row) - len(row.rstrip("d"))) if row.endswith("d") else row  # the body is symmetric: cols 1 to 18
        row = row.replace("g", "G").replace("d", "G")
        ghost = [x for x, ch in enumerate(row) if ch == "G"]
        for x, ch in enumerate(row):
            if ch != ".":
                cells[(x, y)] = PAL[ch]
        if y >= 3 and ghost:  # a little shade down the right side, so it has some volume
            cells[(ghost[-1], y)] = PAL["d"]
            if len(ghost) > 1 and ghost[-2] == ghost[-1] - 1:
                cells[(ghost[-2], y)] = PAL["g"]
    cur = {(CURSOR_AT[0] + x, CURSOR_AT[1] + y) for y, row in enumerate(CURSOR) for x, ch in enumerate(row) if ch == "X"}
    for (x, y) in {(a + dx, b + dy) for a, b in cur for dx, dy in ((1, 0), (-1, 0), (0, 1), (0, -1), (1, 1), (-1, -1), (1, -1), (-1, 1))} - cur:
        if cells.get((x, y)) in GHOST:
            cells[(x, y)] = "#ffffff"  
    for p in cur:
        cells[p] = "#0d1330"
    out = []
    for (x, y), color in sorted(cells.items(), key=lambda kv: (kv[0][1], kv[0][0])):
        out.append(f'<rect x="{ox + x * px}" y="{oy + y * px}" width="{px}" height="{px}" fill="{color}"/>')
    return "".join(out)


DEFS = """<defs>
    <linearGradient id="tile" x1="0" y1="0" x2="1" y2="1"><stop offset="0" stop-color="#0d1330"/><stop offset="1" stop-color="#2a1a5e"/></linearGradient>
    <radialGradient id="aura" cx="0.5" cy="0.45" r="0.55"><stop offset="0" stop-color="#3de0ff" stop-opacity="0.38"/><stop offset="1" stop-color="#3de0ff" stop-opacity="0"/></radialGradient>
    <filter id="glow" x="-30%" y="-30%" width="160%" height="160%"><feGaussianBlur stdDeviation="7"/></filter>
    <pattern id="scan" width="4" height="4" patternUnits="userSpaceOnUse"><rect width="4" height="1" fill="#000" fill-opacity="0.16"/></pattern>
  </defs>"""


def sparkles(color, spots):
    return "".join(f'<rect x="{x}" y="{y}" width="{s}" height="{s}" fill="{color}" fill-opacity="{o}"/>' for x, y, s, o in spots)


def icon():
    px, ox, oy = 9, 38, 30
    return f"""<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 256 256" width="256" height="256" role="img" aria-label="Ghosthand: a pixel ghost with a cursor on its chest">
  {DEFS}
  <rect x="8" y="8" width="240" height="240" rx="56" fill="url(#tile)"/>
  <rect x="8" y="8" width="240" height="240" rx="56" fill="url(#aura)"/>
  <rect x="8" y="8" width="240" height="240" rx="56" fill="url(#scan)"/>
  <rect x="9" y="9" width="238" height="238" rx="55" fill="none" stroke="#3de0ff" stroke-opacity="0.35" stroke-width="2"/>
  <g filter="url(#glow)" opacity="0.65">{mascot_rects(px, ox, oy)}</g>
  <g shape-rendering="crispEdges">{mascot_rects(px, ox, oy)}</g>
  <g shape-rendering="crispEdges">{sparkles("#3de0ff", [(26, 56, 6, 0.9), (214, 80, 6, 0.8), (30, 190, 4, 0.7), (222, 176, 4, 0.9), (206, 40, 4, 0.6)])}</g>
</svg>
"""


def banner():
    px, ox, oy = 10, 70, 30
    return f"""<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 1000 300" width="1000" height="300" role="img" aria-label="Ghosthand">
  {DEFS}
  <rect width="1000" height="300" rx="28" fill="url(#tile)"/>
  <rect width="1000" height="300" rx="28" fill="url(#scan)"/>
  <rect x="0.5" y="0.5" width="999" height="299" rx="27.5" fill="none" stroke="#3de0ff" stroke-opacity="0.25"/>
  <ellipse cx="170" cy="150" rx="190" ry="150" fill="url(#aura)"/>
  <g filter="url(#glow)" opacity="0.6">{mascot_rects(px, ox, oy)}</g>
  <g shape-rendering="crispEdges">{mascot_rects(px, ox, oy)}</g>
  <g shape-rendering="crispEdges">{sparkles("#3de0ff", [(36, 50, 6, 0.9), (330, 70, 6, 0.8), (40, 250, 4, 0.7), (320, 230, 4, 0.9), (300, 28, 4, 0.6)])}</g>
  <text x="372" y="156" font-family="'JetBrains Mono', 'JetBrainsMono Nerd Font Mono', 'Fira Code', Consolas, monospace" font-size="92" font-weight="800" letter-spacing="-4">
    <tspan fill="#e6ecf7">ghost</tspan><tspan fill="#3de0ff">hand</tspan>
  </text>
  <text x="376" y="208" font-family="Inter, 'Inter Variable', 'Segoe UI', Helvetica, Arial, sans-serif" font-size="24" fill="#a9b3c8">A real hand on a real pointer, with a gate in front of it.</text>
  <rect x="376" y="232" width="150" height="3" rx="1.5" fill="#a45cff"/>
</svg>
"""


if __name__ == "__main__":
    here = pathlib.Path(__file__).parent
    (here.parent / "icon.svg").write_text(icon())
    (here / "banner.svg").write_text(banner())
