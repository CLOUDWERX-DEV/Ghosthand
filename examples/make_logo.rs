//! Draws the Ghosthand logo (an angry ghost with a cursor on its chest) into the icon and the banner, as SVG.
//!
//!     cargo run --example make_logo
//!
//! The mark itself is `assets/src/logo.svg`, traced from the artwork in `assets/src/logo-master.png` (white on transparent). This writes
//! `assets/icon.svg` and `assets/src/banner.svg` around it. Render the PNGs with `rsvg-convert -w 512 assets/icon.svg -o assets/icon.png`
//! and `rsvg-convert -w 2000 assets/src/banner.svg -o assets/banner.png`.
use std::path::Path;

const LOGO_SVG: &str = include_str!("../assets/src/logo.svg");
/// The mark's own size, from its viewBox.
const LOGO_W: f32 = 782.0;
const LOGO_H: f32 = 814.0;

const DEFS: &str = r##"<defs>
    <linearGradient id="tile" x1="0" y1="0" x2="1" y2="1"><stop offset="0" stop-color="#0d1330"/><stop offset="1" stop-color="#2a1a5e"/></linearGradient>
    <radialGradient id="aura" cx="0.5" cy="0.45" r="0.55"><stop offset="0" stop-color="#3de0ff" stop-opacity="0.38"/><stop offset="1" stop-color="#3de0ff" stop-opacity="0"/></radialGradient>
    <filter id="glow" x="-30%" y="-30%" width="160%" height="160%"><feGaussianBlur stdDeviation="9"/></filter>
    <pattern id="scan" width="4" height="4" patternUnits="userSpaceOnUse"><rect width="4" height="1" fill="#000" fill-opacity="0.16"/></pattern>
  </defs>"##;

/// The path data of the mark.
fn logo_path() -> &'static str {
    let at = LOGO_SVG.find(" d=\"").expect("logo.svg has no path") + 4;
    &LOGO_SVG[at..at + LOGO_SVG[at..].find('"').expect("unterminated path")]
}

/// The mark `height` tall with its top-left at (x, y): a cyan glow behind, the white line art in front.
fn mark(height: f32, x: f32, y: f32) -> String {
    let s = height / LOGO_H;
    let d = logo_path();
    format!(
        r##"<g transform="translate({x:.2} {y:.2}) scale({s:.5})"><path d="{d}" fill="#3de0ff" fill-rule="evenodd" filter="url(#glow)" opacity="0.55" transform="scale(1)"/><path d="{d}" fill="#f4f7ff" fill-rule="evenodd"/></g>"##
    )
}

fn icon() -> String {
    let h = 156.0;
    let w = h * LOGO_W / LOGO_H;
    format!(
        r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 256 256" width="256" height="256" role="img" aria-label="Ghosthand: an angry ghost with a cursor on its chest">
  {DEFS}
  <rect x="8" y="8" width="240" height="240" rx="56" fill="url(#tile)"/>
  <rect x="8" y="8" width="240" height="240" rx="56" fill="url(#aura)"/>
  <rect x="8" y="8" width="240" height="240" rx="56" fill="url(#scan)"/>
  <rect x="9" y="9" width="238" height="238" rx="55" fill="none" stroke="#3de0ff" stroke-opacity="0.35" stroke-width="2"/>
  {}
</svg>
"##,
        mark(h, (256.0 - w) / 2.0, (256.0 - h) / 2.0 - 2.0)
    )
}

fn banner() -> String {
    let h = 224.0;
    let w = h * LOGO_W / LOGO_H;
    format!(
        r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 1000 300" width="1000" height="300" role="img" aria-label="Ghosthand">
  {DEFS}
  <rect width="1000" height="300" rx="28" fill="url(#tile)"/>
  <rect width="1000" height="300" rx="28" fill="url(#scan)"/>
  <rect x="0.5" y="0.5" width="999" height="299" rx="27.5" fill="none" stroke="#3de0ff" stroke-opacity="0.25"/>
  <ellipse cx="170" cy="150" rx="190" ry="150" fill="url(#aura)"/>
  {}
  <text x="372" y="156" font-family="'JetBrains Mono', 'JetBrainsMono Nerd Font Mono', 'Fira Code', Consolas, monospace" font-size="92" font-weight="800" letter-spacing="-4">
    <tspan fill="#e6ecf7">ghost</tspan><tspan fill="#3de0ff">hand</tspan>
  </text>
  <text x="376" y="208" font-family="Inter, 'Inter Variable', 'Segoe UI', Helvetica, Arial, sans-serif" font-size="24" fill="#a9b3c8">A real hand on a real pointer, with a gate in front of it.</text>
  <rect x="376" y="232" width="150" height="3" rx="1.5" fill="#a45cff"/>
</svg>
"##,
        mark(h, 170.0 - w / 2.0, 38.0)
    )
}

fn main() -> std::io::Result<()> {
    let assets = Path::new(env!("CARGO_MANIFEST_DIR")).join("assets");
    std::fs::write(assets.join("icon.svg"), icon())?;
    std::fs::write(assets.join("src/banner.svg"), banner())?;
    println!("wrote assets/icon.svg and assets/src/banner.svg");
    Ok(())
}
