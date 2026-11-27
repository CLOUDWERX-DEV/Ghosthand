pragma Singleton

import QtQuick
import Quickshell

// The overlay's look: a dark frosted card, one accent. The colours below are the defaults; ~/.config/ghosthand/theme.json overrides any of them
// (surface, raised, text, muted, text_on_accent, danger, warn, hairline, accent), live. "accent" in config.json still wins for the accent.
Singleton {
    readonly property color surface: Gate.hasSurface ? Qt.alpha(Gate.themed("surface", "#0e121b"), 0.92) : "#e60e121b"      // the card (a little see-through)
    readonly property color raised: Gate.themed("raised", "#252c3d")
    readonly property color text: Gate.themed("text", "#e6ecf7")
    readonly property color muted: Gate.themed("muted", "#a9b3c8")
    readonly property color textOnAccent: Gate.themed("text_on_accent", "#04121a")
    readonly property color danger: Gate.themed("danger", "#ff5c7a")
    readonly property color warn: Gate.themed("warn", "#ffb347")
    readonly property color hairline: Gate.themed("hairline", "#33425a")
    readonly property color accent: Gate.accent || "#3de0ff"
    readonly property string font: "Inter, Noto Sans, sans-serif"
    readonly property real scale: Math.max(0.75, Math.min(2.0, Gate.uiScale || 1.0))
    readonly property int anim: 220
}
