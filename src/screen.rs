//! Screenshots on whatever the session has: grim (wlroots compositors: niri, Hyprland, sway, river...), maim, scrot or ImageMagick on X11,
//! spectacle (KDE) or gnome-screenshot (GNOME). The first tool that is installed and works wins; `screenshot` in the config overrides it with your
//! own command (`{file}` is where it writes, e.g. "mytool --out {file}").
use std::process::{Command, Stdio};

use crate::config::Config;
use crate::util::which;

pub type Region = [i64; 4];

fn s(v: &[&str]) -> Vec<String> { v.iter().map(|x| x.to_string()).collect() }

/// The command a tool needs for `file` (and `region` = x, y, w, h), or None when it can't take a region: the caller crops.
pub fn argv(tool: &str, file: &str, region: Option<Region>) -> Option<Vec<String>> {
    let mut a = s(&[tool]);
    match (tool, region) {
        ("grim", Some([x, y, w, h])) => a.extend(s(&["-g", &format!("{x},{y} {w}x{h}")])),
        ("maim", Some([x, y, w, h])) => a.extend(s(&["-g", &format!("{w}x{h}+{x}+{y}")])),
        ("scrot", r) => {
            a.push("-o".into());
            if let Some([x, y, w, h]) = r { a.extend(s(&["-a", &format!("{x},{y},{w},{h}")])); }
        }
        ("import", r) => {
            a.extend(s(&["-window", "root"]));
            if let Some([x, y, w, h]) = r { a.extend(s(&["-crop", &format!("{w}x{h}+{x}+{y}"), "+repage"])); }
        }
        ("spectacle", None) => a.extend(s(&["-b", "-n", "-f", "-o"])),
        ("gnome-screenshot", None) => a.push("-f".into()),
        ("spectacle" | "gnome-screenshot", Some(_)) => return None,
        _ => {}
    }
    a.push(file.into());
    Some(a)
}

const WAYLAND_ORDER: &[&str] = &["grim", "spectacle", "gnome-screenshot", "maim", "scrot", "import"];
const X11_ORDER: &[&str] = &["maim", "scrot", "import", "gnome-screenshot", "spectacle", "grim"];

/// The tools worth trying here, best first, only those installed.
pub fn order() -> Vec<&'static str> {
    let wayland = std::env::var_os("WAYLAND_DISPLAY").is_some_and(|v| !v.is_empty()) || std::env::var("XDG_SESSION_TYPE").is_ok_and(|v| v == "wayland");
    (if wayland { WAYLAND_ORDER } else { X11_ORDER }).iter().copied().filter(|t| which(t)).collect()
}

fn run(args: &[String]) -> Result<(), String> {
    let out = Command::new(&args[0]).args(&args[1..]).stdin(Stdio::null()).stdout(Stdio::null()).output().map_err(|e| e.to_string())?;
    if out.status.success() { Ok(()) } else {
        let err = String::from_utf8_lossy(&out.stderr);
        Err(err.lines().last().unwrap_or("failed").trim().to_string())
    }
}

/// Cut a region out of a full screenshot, for tools that can't (needs ImageMagick's `convert`).
fn crop(path: &str, [x, y, w, h]: Region) -> Result<(), String> {
    if !which("convert") {
        return Err("this screenshot tool can't take a region: install grim, maim, scrot or ImageMagick, or ask for the whole screen".into());
    }
    run(&s(&["convert", path, "-crop", &format!("{w}x{h}+{x}+{y}"), "+repage", path]))
}

/// Write a PNG of the screen (or of `region`) to `path`.
pub fn screenshot(path: &str, region: Option<Region>) -> Result<(), String> {
    if let Some(custom) = Config::load().screenshot_command() {
        if region.is_some() {
            return Err("a custom `screenshot` command in the config always captures the whole screen".into());
        }
        let status = Command::new("sh").args(["-c", &custom.replace("{file}", path)]).stdin(Stdio::null()).stdout(Stdio::null()).status().map_err(|e| e.to_string())?;
        return if status.success() { Ok(()) } else { Err("the custom screenshot command failed".into()) };
    }
    let mut errors = Vec::new();
    for tool in order() {
        let direct = argv(tool, path, region);
        let full = direct.clone().or_else(|| argv(tool, path, None)).unwrap_or_default();
        let result = run(&full).and_then(|_| if std::fs::metadata(path).map(|m| m.len() > 0).unwrap_or(false) { Ok(()) } else { Err("empty file".into()) });
        match (result, direct.is_none(), region) {
            (Ok(()), true, Some(r)) => return crop(path, r),
            (Ok(()), _, _) => return Ok(()),
            (Err(e), _, _) => errors.push(format!("{tool}: {e}")),
        }
    }
    Err(format!("no screenshot tool worked ({})", if errors.is_empty() { "none installed: try grim, maim, scrot or ImageMagick".to_string() } else { errors.join("; ") }))
}

pub fn screenshot_bytes(region: Option<Region>) -> Result<Vec<u8>, String> {
    let path = std::env::temp_dir().join(format!("ghosthand-{}-{}.png", std::process::id(), (crate::util::now() * 1000.0) as u64));
    let p = path.to_string_lossy().into_owned();
    let result = screenshot(&p, region).and_then(|_| std::fs::read(&path).map_err(|e| e.to_string()));
    let _ = std::fs::remove_file(&path);
    result
}

/// (width, height) from a PNG's header, or None.
pub fn png_size(data: &[u8]) -> Option<(u32, u32)> {
    if data.len() >= 24 && data[..8] == [0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a] {
        Some((u32::from_be_bytes(data[16..20].try_into().ok()?), u32::from_be_bytes(data[20..24].try_into().ok()?)))
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_tool_asks_for_a_region_in_its_own_words() {
        let r = Some([10, 20, 300, 200]);
        assert_eq!(argv("grim", "f.png", r).unwrap(), s(&["grim", "-g", "10,20 300x200", "f.png"]));
        assert_eq!(argv("maim", "f.png", r).unwrap(), s(&["maim", "-g", "300x200+10+20", "f.png"]));
        assert_eq!(argv("scrot", "f.png", r).unwrap(), s(&["scrot", "-o", "-a", "10,20,300,200", "f.png"]));
        assert_eq!(argv("import", "f.png", r).unwrap(), s(&["import", "-window", "root", "-crop", "300x200+10+20", "+repage", "f.png"]));
        assert_eq!(argv("grim", "f.png", None).unwrap(), s(&["grim", "f.png"]));
        assert_eq!(argv("spectacle", "f.png", None).unwrap(), s(&["spectacle", "-b", "-n", "-f", "-o", "f.png"]));
    }

    #[test]
    fn spectacle_and_gnome_cannot_take_a_region_so_the_caller_crops() {
        assert!(argv("spectacle", "f.png", Some([0, 0, 1, 1])).is_none());
        assert!(argv("gnome-screenshot", "f.png", Some([0, 0, 1, 1])).is_none());
        assert_eq!(argv("gnome-screenshot", "f.png", None).unwrap(), s(&["gnome-screenshot", "-f", "f.png"]));
    }

    #[test]
    fn png_size_reads_the_header() {
        let mut png = vec![0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a, 0, 0, 0, 13, b'I', b'H', b'D', b'R'];
        png.extend(1366u32.to_be_bytes());
        png.extend(768u32.to_be_bytes());
        assert_eq!(png_size(&png), Some((1366, 768)));
        assert_eq!(png_size(b"nope"), None);
    }
}
