//! Screen geometry: the output's size in screenshot pixels and its scale, and turning action words into numbers.
//!
//! Detected from the compositor that's running: niri, Hyprland, sway, then X11 (xrandr), then wlr-randr, and as a last resort from the size of
//! a screenshot (scale 1). GHOSTHAND_OUTPUT_SIZE="2560x1600" and GHOSTHAND_SCALE="1.25" override all of it.
use serde_json::Value;

use crate::util::{output_of, which};

#[derive(Debug, Clone, PartialEq)]
pub struct Info {
    pub name: String,
    pub width: i64,
    pub height: i64,
    pub scale: f64,
}

impl Info {
    pub fn to_json(&self) -> String {
        serde_json::json!({"name": self.name, "width": self.width, "height": self.height, "scale": self.scale}).to_string()
    }
}

/// Outputs rotated a quarter turn are taller than wide in the screenshot. `transform`: "1"/"3"/"5"/"7" (Hyprland), "90", "flipped-270" (sway), "Flipped90" (niri).
fn swap(w: i64, h: i64, transform: &str) -> (i64, i64) {
    let t = transform.to_ascii_lowercase();
    if matches!(t.as_str(), "1" | "3" | "5" | "7") || t.ends_with("90") || t.ends_with("270") { (h, w) } else { (w, h) }
}

fn text(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Null => "normal".into(),
        other => other.to_string(),
    }
}

fn parse(json: &str) -> Result<Value, String> {
    serde_json::from_str(json).map_err(|e| e.to_string())
}

fn int(v: &Value, key: &str) -> Result<i64, String> {
    v.get(key).and_then(Value::as_i64).ok_or_else(|| format!("no {key}"))
}

pub fn from_niri(name: Option<&str>, json: &str) -> Result<Info, String> {
    let all = parse(json)?;
    let outputs = all.as_object().ok_or("no outputs")?;
    let out = name.and_then(|n| outputs.get(n)).or_else(|| outputs.values().next()).ok_or("no outputs")?;
    let mode = out["modes"].get(out["current_mode"].as_u64().ok_or("no current mode")? as usize).ok_or("no mode")?;
    let logical = &out["logical"];
    let (w, h) = swap(int(mode, "width")?, int(mode, "height")?, &text(&logical["transform"]));
    Ok(Info { name: out["name"].as_str().unwrap_or(name.unwrap_or("")).to_string(), width: w, height: h, scale: logical["scale"].as_f64().unwrap_or(1.0) })
}

pub fn from_hyprland(name: Option<&str>, json: &str) -> Result<Info, String> {
    let all = parse(json)?;
    let monitors = all.as_array().filter(|m| !m.is_empty()).ok_or("no monitors")?;
    let pick = monitors.iter().find(|m| name.is_some() && m["name"].as_str() == name).or_else(|| monitors.iter().find(|m| m["focused"].as_bool() == Some(true))).unwrap_or(&monitors[0]);
    let (w, h) = swap(int(pick, "width")?, int(pick, "height")?, &text(&pick["transform"]));
    Ok(Info { name: pick["name"].as_str().unwrap_or("").into(), width: w, height: h, scale: pick["scale"].as_f64().unwrap_or(1.0) })
}

pub fn from_sway(name: Option<&str>, json: &str) -> Result<Info, String> {
    let all = parse(json)?;
    let outputs: Vec<&Value> = all.as_array().ok_or("no outputs")?.iter().filter(|o| o["active"].as_bool().unwrap_or(true)).collect();
    let pick = *outputs.iter().find(|o| name.is_some() && o["name"].as_str() == name).or_else(|| outputs.iter().find(|o| o["focused"].as_bool() == Some(true))).or(outputs.first()).ok_or("no outputs")?;
    let mode = &pick["current_mode"];
    let (w, h) = swap(int(mode, "width")?, int(mode, "height")?, &text(&pick["transform"]));
    Ok(Info { name: pick["name"].as_str().unwrap_or("").into(), width: w, height: h, scale: pick["scale"].as_f64().unwrap_or(1.0) })
}

/// On X11 the screenshot is the whole desktop, so that's the size (all monitors together).
pub fn from_xrandr(_name: Option<&str>, out: &str) -> Result<Info, String> {
    let rest = out.split("current ").nth(1).ok_or("no screen size in the xrandr output")?;
    let mut nums = rest.split(|c: char| !c.is_ascii_digit()).filter(|s| !s.is_empty());
    let (w, h) = (nums.next().and_then(|s| s.parse().ok()), nums.next().and_then(|s| s.parse().ok()));
    match (w, h) {
        (Some(w), Some(h)) => Ok(Info { name: "X11".into(), width: w, height: h, scale: 1.0 }),
        _ => Err("no screen size in the xrandr output".into()),
    }
}

pub fn from_wlr_randr(name: Option<&str>, json: &str) -> Result<Info, String> {
    let all = parse(json)?;
    let outputs = all.as_array().ok_or("no outputs")?;
    let pick = outputs.iter().find(|o| name.is_some() && o["name"].as_str() == name).or_else(|| outputs.iter().find(|o| o["enabled"].as_bool().unwrap_or(true))).ok_or("no outputs")?;
    let mode = pick["modes"].as_array().and_then(|m| m.iter().find(|m| m["current"].as_bool() == Some(true))).ok_or("no current mode")?;
    let (w, h) = swap(int(mode, "width")?, int(mode, "height")?, &text(&pick["transform"]));
    Ok(Info { name: pick["name"].as_str().unwrap_or("").into(), width: w, height: h, scale: pick["scale"].as_f64().unwrap_or(1.0) })
}

/// The size of a screenshot: always right for the pixels, but the scale is a guess (1, or GHOSTHAND_SCALE).
pub fn from_screenshot() -> Result<Info, String> {
    let png = crate::screen::screenshot_bytes(None)?;
    let (w, h) = crate::screen::png_size(&png).ok_or("couldn't tell the screen size: set GHOSTHAND_OUTPUT_SIZE and GHOSTHAND_SCALE")?;
    Ok(Info { name: String::new(), width: w as i64, height: h as i64, scale: std::env::var("GHOSTHAND_SCALE").ok().and_then(|s| s.parse().ok()).unwrap_or(1.0) })
}

type Detector = fn(Option<&str>) -> Result<Info, String>;

/// [(label, detector)] worth trying in this session, most specific first.
pub fn detectors() -> Vec<(&'static str, Detector)> {
    let has = |k: &str| std::env::var_os(k).is_some_and(|v| !v.is_empty());
    let run = |program: &str, args: &[&str]| output_of(program, args).ok_or_else(|| format!("{program} failed"));
    let mut found: Vec<(&'static str, Detector)> = Vec::new();
    if has("NIRI_SOCKET") && which("niri") {
        found.push(("niri", |n| from_niri(n, &output_of("niri", &["msg", "--json", "outputs"]).ok_or("niri failed")?)));
    }
    if has("HYPRLAND_INSTANCE_SIGNATURE") && which("hyprctl") {
        found.push(("Hyprland", |n| from_hyprland(n, &output_of("hyprctl", &["-j", "monitors"]).ok_or("hyprctl failed")?)));
    }
    if has("SWAYSOCK") && which("swaymsg") {
        found.push(("sway", |n| from_sway(n, &output_of("swaymsg", &["-t", "get_outputs", "-r"]).ok_or("swaymsg failed")?)));
    }
    if !has("WAYLAND_DISPLAY") && has("DISPLAY") && which("xrandr") {
        found.push(("X11", |n| from_xrandr(n, &output_of("xrandr", &[]).ok_or("xrandr failed")?)));
    }
    if has("WAYLAND_DISPLAY") && which("wlr-randr") {
        found.push(("wlr-randr", |n| from_wlr_randr(n, &output_of("wlr-randr", &["--json"]).ok_or("wlr-randr failed")?)));
    }
    let _ = run; // (kept for symmetry with the other detectors)
    found
}

fn env_override() -> Option<Info> {
    let (size, scale) = (std::env::var("GHOSTHAND_OUTPUT_SIZE").ok()?, std::env::var("GHOSTHAND_SCALE").ok()?);
    let (w, h) = size.to_ascii_lowercase().split_once('x').and_then(|(w, h)| Some((w.trim().parse().ok()?, h.trim().parse().ok()?)))?;
    Some(Info { name: std::env::var("GHOSTHAND_OUTPUT").unwrap_or_default(), width: w, height: h, scale: scale.parse().ok()? })
}

/// An output as { name, width, height, scale }: the size in screenshot pixels, and the scale the compositor reports.
pub fn output_info(name: Option<&str>) -> Result<Info, String> {
    if let Some(i) = env_override() {
        return Ok(i);
    }
    for (_label, detect) in detectors() {
        if let Ok(i) = detect(name) {
            return Ok(i);
        }
    }
    from_screenshot()
}

/// What `output_info` would use, for `ghosthand doctor`: the label of the detector, or "screenshot size".
pub fn detected() -> &'static str {
    if env_override().is_some() {
        return "environment";
    }
    detectors().into_iter().find(|(_, d)| d(None).is_ok()).map_or("screenshot size", |(l, _)| l)
}

/// The numbers among an action's words (floats, negatives and a leading + are fine); anything else is an error.
pub fn numbers(words: &[&str]) -> Result<Vec<f64>, String> {
    words.iter().map(|w| w.parse::<f64>().map_err(|_| format!("not a number: '{w}'"))).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const NIRI: &str = r#"{"eDP-1":{"name":"eDP-1","modes":[{"width":2560,"height":1600}],"current_mode":0,"logical":{"scale":1.25,"transform":"Normal"}}}"#;
    const NIRI_ROTATED: &str = r#"{"DP-1":{"name":"DP-1","modes":[{"width":1920,"height":1080}],"current_mode":0,"logical":{"scale":1.0,"transform":"90"}}}"#;
    const HYPR: &str = r#"[{"name":"DP-2","width":1920,"height":1080,"scale":1.0,"focused":false,"transform":0},{"name":"eDP-1","width":2880,"height":1800,"scale":1.5,"focused":true,"transform":0}]"#;
    const SWAY: &str = r#"[{"name":"HDMI-A-1","active":true,"focused":true,"scale":2.0,"transform":"normal","current_mode":{"width":3840,"height":2160}}]"#;
    const XRANDR: &str = "Screen 0: minimum 16 x 16, current 3840 x 1080, maximum 32767 x 32767\neDP-1 connected primary 1920x1080+0+0\n";
    const WLR: &str = r#"[{"name":"DP-1","enabled":true,"scale":1.0,"transform":"normal","modes":[{"width":800,"height":600,"current":false},{"width":1280,"height":720,"current":true}]}]"#;

    #[test]
    fn niri() {
        assert_eq!(from_niri(None, NIRI).unwrap(), Info { name: "eDP-1".into(), width: 2560, height: 1600, scale: 1.25 });
    }

    #[test]
    fn a_rotated_output_is_taller_than_wide() {
        assert_eq!(from_niri(None, NIRI_ROTATED).unwrap().width, 1080);
    }

    #[test]
    fn hyprland_picks_the_focused_monitor_or_the_named_one() {
        assert_eq!(from_hyprland(None, HYPR).unwrap(), Info { name: "eDP-1".into(), width: 2880, height: 1800, scale: 1.5 });
        assert_eq!(from_hyprland(Some("DP-2"), HYPR).unwrap().name, "DP-2");
    }

    #[test]
    fn sway() {
        assert_eq!(from_sway(None, SWAY).unwrap(), Info { name: "HDMI-A-1".into(), width: 3840, height: 2160, scale: 2.0 });
    }

    #[test]
    fn x11_is_the_whole_desktop() {
        assert_eq!(from_xrandr(None, XRANDR).unwrap(), Info { name: "X11".into(), width: 3840, height: 1080, scale: 1.0 });
        assert!(from_xrandr(None, "nothing").is_err());
    }

    #[test]
    fn wlr_randr_uses_the_current_mode() {
        assert_eq!(from_wlr_randr(None, WLR).unwrap().width, 1280);
    }

    #[test]
    fn bad_json_is_an_error_not_a_panic() {
        for f in [from_niri, from_hyprland, from_sway, from_wlr_randr] {
            assert!(f(None, "nope").is_err());
            assert!(f(None, "[]").is_err() || f(None, "{}").is_err());
        }
    }

    #[test]
    fn numbers() {
        assert_eq!(super::numbers(&["10", "-5.5", "+3"]).unwrap(), vec![10.0, -5.5, 3.0]);
        assert!(super::numbers(&["12", "x"]).is_err());
    }
}
