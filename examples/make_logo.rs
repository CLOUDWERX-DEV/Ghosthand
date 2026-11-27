//! Draws the Ghosthand mascot, a pixel ghost with a cursor on its chest, as SVG.
//!
//!     cargo run --example make_logo
//!
//! writes `assets/icon.svg` and `assets/src/banner.svg`. Render the PNGs with `rsvg-convert -w 512 assets/icon.svg -o assets/icon.png`
//! and `rsvg-convert -w 2000 assets/src/banner.svg -o assets/banner.png`.
use std::collections::BTreeMap;
use std::path::Path;

const GHOST: &str = "#cdf4ff";
const GHOST_SHADE: &str = "#9fe6f7";
const GHOST_DEEP: &str = "#58bfd9";
const EYE: &str = "#ffffff";
const PUPIL: &str = "#1c3dff";
const CURSOR_FILL: &str = "#0d1330";

/// The body, one row per line. G ghost, W eye, P pupil. The body is symmetric across columns 1 to 18; the shade down the right side is added below.
const BODY: [&str; 22] = [
    "......GGGGGGGG......",
    "....GGGGGGGGGGGG....",
    "...GGWWGGGGGGGGGG...",
    "..GGWGGGGGGGGGGGG...",
    ".GGGWWWWGGGGWWWWGGG.",
    ".GGGWWWWGGGGWWWWGGG.",
    ".GGGWWWWGGGGWWWWGGG.",
    ".GGGWWPPGGGGWWPPGGG.",
    ".GGGWWPPGGGGWWPPGGG.",
    ".GGGGGGGGGGGGGGGGGG.",
    ".GGGGGGGGGGGGGGGGGG.",
    ".GGGGGGGGGGGGGGGGGG.",
    ".GGGGGGGGGGGGGGGGGG.",
    ".GGGGGGGGGGGGGGGGGG.",
    ".GGGGGGGGGGGGGGGGGG.",
    ".GGGGGGGGGGGGGGGGGG.",
    ".GGGGGGGGGGGGGGGGGG.",
    ".GGGGGGGGGGGGGGGGGG.",
    ".GGGGGGGGGGGGGGGGGG.",
    ".GGGG..GGGGGG..GGGG.",
    ".GG......GGGG....GG.",
    "....................",
];

const CURSOR: [&str; 7] = ["X....", "XX...", "XXX..", "XXXX.", "XXXXX", "XXX..", "X.XX."];
const CURSOR_AT: (i32, i32) = (8, 11);

const DEFS: &str = r##"<defs>
    <linearGradient id="tile" x1="0" y1="0" x2="1" y2="1"><stop offset="0" stop-color="#0d1330"/><stop offset="1" stop-color="#2a1a5e"/></linearGradient>
    <radialGradient id="aura" cx="0.5" cy="0.45" r="0.55"><stop offset="0" stop-color="#3de0ff" stop-opacity="0.38"/><stop offset="1" stop-color="#3de0ff" stop-opacity="0"/></radialGradient>
    <filter id="glow" x="-30%" y="-30%" width="160%" height="160%"><feGaussianBlur stdDeviation="7"/></filter>
    <pattern id="scan" width="4" height="4" patternUnits="userSpaceOnUse"><rect width="4" height="1" fill="#000" fill-opacity="0.16"/></pattern>
  </defs>"##;

/// The mascot as SVG rects, `px` pixels per cell, with its top-left at (ox, oy).
fn mascot(px: i32, ox: i32, oy: i32) -> String {
    let mut cells: BTreeMap<(i32, i32), &str> = BTreeMap::new(); // keyed (row, col) so it comes out in reading order
    for (y, row) in BODY.iter().enumerate() {
        let y = y as i32;
        let mut ghost = Vec::new();
        for (x, ch) in row.chars().enumerate() {
            let color = match ch { 'G' => GHOST, 'W' => EYE, 'P' => PUPIL, _ => continue };
            cells.insert((y, x as i32), color);
            if ch == 'G' { ghost.push(x as i32); }
        }
        if y >= 3 && !ghost.is_empty() {
            // a little shade down the right side, so it has some volume
            let last = ghost[ghost.len() - 1];
            cells.insert((y, last), GHOST_DEEP);
            if ghost.len() > 1 && ghost[ghost.len() - 2] == last - 1 {
                cells.insert((y, last - 1), GHOST_SHADE);
            }
        }
    }
    let cursor: Vec<(i32, i32)> = CURSOR.iter().enumerate().flat_map(|(y, row)| row.chars().enumerate().filter(|(_, c)| *c == 'X').map(move |(x, _)| (CURSOR_AT.0 + x as i32, CURSOR_AT.1 + y as i32))).collect();
    for (cx, cy) in &cursor {
        for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1), (1, 1), (-1, -1), (1, -1), (-1, 1)] {
            let at = (cy + dy, cx + dx);
            if !cursor.contains(&(at.1, at.0)) && matches!(cells.get(&at), Some(&GHOST | &GHOST_SHADE | &GHOST_DEEP)) {
                cells.insert(at, "#ffffff"); // a white edge so the dark cursor reads on the pale body
            }
        }
    }
    for (cx, cy) in &cursor {
        cells.insert((*cy, *cx), CURSOR_FILL);
    }
    cells.iter().map(|((y, x), c)| format!(r#"<rect x="{}" y="{}" width="{px}" height="{px}" fill="{c}"/>"#, ox + x * px, oy + y * px)).collect()
}

fn sparkles(spots: &[(i32, i32, i32, f32)]) -> String {
    spots.iter().map(|(x, y, s, o)| format!(r##"<rect x="{x}" y="{y}" width="{s}" height="{s}" fill="#3de0ff" fill-opacity="{o}"/>"##)).collect()
}

fn icon() -> String {
    let m = mascot(9, 38, 30);
    format!(
        r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 256 256" width="256" height="256" role="img" aria-label="Ghosthand: a pixel ghost with a cursor on its chest">
  {DEFS}
  <rect x="8" y="8" width="240" height="240" rx="56" fill="url(#tile)"/>
  <rect x="8" y="8" width="240" height="240" rx="56" fill="url(#aura)"/>
  <rect x="8" y="8" width="240" height="240" rx="56" fill="url(#scan)"/>
  <rect x="9" y="9" width="238" height="238" rx="55" fill="none" stroke="#3de0ff" stroke-opacity="0.35" stroke-width="2"/>
  <g filter="url(#glow)" opacity="0.65">{m}</g>
  <g shape-rendering="crispEdges">{m}</g>
  <g shape-rendering="crispEdges">{}</g>
</svg>
"##,
        sparkles(&[(26, 56, 6, 0.9), (214, 80, 6, 0.8), (30, 190, 4, 0.7), (222, 176, 4, 0.9), (206, 40, 4, 0.6)])
    )
}

fn banner() -> String {
    let m = mascot(10, 70, 30);
    format!(
        r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 1000 300" width="1000" height="300" role="img" aria-label="Ghosthand">
  {DEFS}
  <rect width="1000" height="300" rx="28" fill="url(#tile)"/>
  <rect width="1000" height="300" rx="28" fill="url(#scan)"/>
  <rect x="0.5" y="0.5" width="999" height="299" rx="27.5" fill="none" stroke="#3de0ff" stroke-opacity="0.25"/>
  <ellipse cx="170" cy="150" rx="190" ry="150" fill="url(#aura)"/>
  <g filter="url(#glow)" opacity="0.6">{m}</g>
  <g shape-rendering="crispEdges">{m}</g>
  <g shape-rendering="crispEdges">{}</g>
  <text x="372" y="156" font-family="'JetBrains Mono', 'JetBrainsMono Nerd Font Mono', 'Fira Code', Consolas, monospace" font-size="92" font-weight="800" letter-spacing="-4">
    <tspan fill="#e6ecf7">ghost</tspan><tspan fill="#3de0ff">hand</tspan>
  </text>
  <text x="376" y="208" font-family="Inter, 'Inter Variable', 'Segoe UI', Helvetica, Arial, sans-serif" font-size="24" fill="#a9b3c8">A real hand on a real pointer, with a gate in front of it.</text>
  <rect x="376" y="232" width="150" height="3" rx="1.5" fill="#a45cff"/>
</svg>
"##,
        sparkles(&[(36, 50, 6, 0.9), (330, 70, 6, 0.8), (40, 250, 4, 0.7), (320, 230, 4, 0.9), (300, 28, 4, 0.6)])
    )
}

fn main() -> std::io::Result<()> {
    let assets = Path::new(env!("CARGO_MANIFEST_DIR")).join("assets");
    std::fs::write(assets.join("icon.svg"), icon())?;
    std::fs::write(assets.join("src/banner.svg"), banner())?;
    println!("wrote assets/icon.svg and assets/src/banner.svg");
    Ok(())
}
