//! Which window has keyboard focus: `focus` prints it, so an agent knows where its keys will land.
use serde_json::{json, Value};

use crate::util::{output_of, which};

fn has(k: &str) -> bool {
    std::env::var_os(k).is_some_and(|v| !v.is_empty())
}

fn sway_focused(node: &Value) -> Option<&Value> {
    if node["focused"].as_bool() == Some(true) {
        return Some(node);
    }
    ["nodes", "floating_nodes"].iter().filter_map(|k| node[*k].as_array()).flatten().find_map(sway_focused)
}

fn s(v: &Value) -> String { v.as_str().unwrap_or("").to_string() }

/// `{ app, title }` of the focused window, or None when the session can't say (GNOME, KDE and others don't expose it).
pub fn focused() -> Option<Value> {
    if has("NIRI_SOCKET") && which("niri") {
        let w: Value = serde_json::from_str(&output_of("niri", &["msg", "--json", "focused-window"])?).ok()?;
        return w.is_object().then(|| json!({"app": s(&w["app_id"]), "title": s(&w["title"])}));
    }
    if has("HYPRLAND_INSTANCE_SIGNATURE") && which("hyprctl") {
        let w: Value = serde_json::from_str(&output_of("hyprctl", &["-j", "activewindow"])?).ok()?;
        return w["address"].is_string().then(|| json!({"app": s(&w["class"]), "title": s(&w["title"])}));
    }
    if has("SWAYSOCK") && which("swaymsg") {
        let tree: Value = serde_json::from_str(&output_of("swaymsg", &["-t", "get_tree", "-r"])?).ok()?;
        let w = sway_focused(&tree).filter(|w| w["pid"].is_number())?;
        let app = if w["app_id"].is_string() { s(&w["app_id"]) } else { s(&w["window_properties"]["class"]) };
        return Some(json!({"app": app, "title": s(&w["name"])}));
    }
    if has("DISPLAY") && which("xdotool") {
        let title = output_of("xdotool", &["getactivewindow", "getwindowname"])?;
        return Some(json!({"app": "", "title": title.trim()}));
    }
    None
}
