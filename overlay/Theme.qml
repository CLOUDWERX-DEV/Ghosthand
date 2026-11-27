pragma Singleton

import QtQuick
import Quickshell

// The overlay's look: a dark frosted card, one accent. Change the colours here, or set "accent" in ~/.config/ghosthand/config.json.
Singleton {
    readonly property color surface: "#e60e121b"      // the card (a little see-through)
    readonly property color raised: "#252c3d"
    readonly property color text: "#e6ecf7"
    readonly property color muted: "#a9b3c8"
    readonly property color textOnAccent: "#04121a"
    readonly property color danger: "#ff5c7a"
    readonly property color warn: "#ffb347"
    readonly property color hairline: "#33425a"
    readonly property color accent: Gate.accent || "#3de0ff"
    readonly property string font: "Inter, Noto Sans, sans-serif"
    readonly property real scale: Math.max(0.75, Math.min(2.0, Gate.uiScale || 1.0))
    readonly property int anim: 220
}
