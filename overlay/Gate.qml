pragma Singleton

import QtQuick
import Quickshell
import Quickshell.Io

// Ghosthand's state: the gate and the cursor tag.
//
//   gate   a program asks (`ask`), a card on screen counts down, and nothing moves until you Allow (or the countdown ends, in "notify" mode).
//          Deny, `ghosthand revoke` (bind it to a key) or the overlay going away stops it. "Allow for N min" lets later requests straight in.
//   tag    while a program moves the pointer, an "Agent Control" tag and ring follow the cursor.
//
// Settings come from ~/.config/ghosthand/config.json and only from there (a program asking for the pointer can't change them):
// mode "ask" (default: must be allowed, no answer is a no) | "notify" (a countdown that goes ahead unless you deny) | "off" (no card, the tag
// still shows); countdown (seconds, 2 to 60); grant_minutes (1 to 240); accent (a colour); ui_scale.
// Programs talk to this over IPC (`qs -c ghosthand ipc call ghosthand ...`) and stream the pointer as lines in
// $XDG_RUNTIME_DIR/ghosthand/stream: "pos X Y OUTPUT", "begin WHO", "end" (logical pixels).
Singleton {
    id: root

    FileView {
        id: file
        path: (Quickshell.env("XDG_CONFIG_HOME") || (Quickshell.env("HOME") + "/.config")) + "/ghosthand/config.json"
        watchChanges: true
        printErrors: false
        onFileChanged: reload()
        adapter: JsonAdapter {
            id: cfg
            property string mode: "ask"
            property int countdown: 8
            property int grant_minutes: 10
            property string accent: "#3de0ff"
            property real ui_scale: 1.0
        }
    }
    readonly property string mode: cfg && ["ask", "notify", "off"].includes(cfg.mode) ? cfg.mode : "ask"
    readonly property int countdownSeconds: Math.max(2, Math.min(60, cfg ? cfg.countdown : 8))
    readonly property int grantMinutes: Math.max(1, Math.min(240, cfg ? cfg.grant_minutes : 10))
    readonly property color accent: cfg && cfg.accent ? cfg.accent : "#3de0ff"
    readonly property real uiScale: cfg && cfg.ui_scale ? cfg.ui_scale : 1.0

    property string phase: "idle"     // idle | asking | active
    property string decision: ""      // "" while asking; then allowed | denied | timeout | revoked
    property string who: "Agent"
    property string reason: ""
    property string scope: "mouse"       // what is being asked for: "mouse", "keyboard" or "mouse and keyboard"
    property string detail: ""           // what the keyboard will do, shown on the card ("type: git status")
    property double deadline: 0
    property double countdownMs: 8000
    property double grantedUntil: 0       // a standing "Allow for N min" for the mouse
    property double grantedKeysUntil: 0   // and, separately, for the keyboard: allowing the mouse never lets anything type
    property double now: Date.now()
    property double lastSeen: 0
    property real cx: -1
    property real cy: -1
    property string outputName: ""
    property double toastUntil: 0

    readonly property real secondsLeft: Math.max(0, (deadline - now) / 1000)
    readonly property real fraction: phase === "asking" ? Math.max(0, Math.min(1, (deadline - now) / Math.max(1, countdownMs))) : 0
    function covered(sc: string): bool {
        const t = Date.now();
        return (sc.indexOf("mouse") < 0 || t < grantedUntil) && (sc.indexOf("keyboard") < 0 || t < grantedKeysUntil);
    }
    // The tag shows while a program is driving and its last position is fresh (a program that died never leaves it behind).
    readonly property bool tagShown: phase === "active" && cx >= 0 && now - lastSeen < 3500
    readonly property bool cardShown: phase === "asking" || now < toastUntil

    Timer {
        interval: 100
        running: root.phase !== "idle" || root.toastUntil > root.now
        repeat: true
        onTriggered: {
            root.now = Date.now();
            if (root.phase === "asking" && root.now >= root.deadline) {
                // The countdown ended: "ask" mode takes silence for a no, "notify" mode goes ahead.
                if (root.mode === "notify") root.allow(0);
                else { root.decision = "timeout"; root.phase = "idle"; }
            }
        }
    }

    function reset(): void { cx = -1; cy = -1; lastSeen = 0; }

    // A program asks to take the pointer (and perhaps the keyboard: `sc`, with `what` it will type or press). Returns "allowed" (a standing grant, or the gate is off) or "asking" (poll `status`).
    // A client can ask for a longer wait than the setting, never a shorter one.
    function ask(name: string, why: string, seconds: int, sc: string, what: string): string {
        who = name || "Agent";
        reason = why || "";
        scope = sc || "mouse";
        detail = what || "";
        reset();
        if (mode === "off" || covered(scope)) {
            decision = "allowed";
            phase = "active";
            toastUntil = Date.now() + 2800;
            now = Date.now();
            return "allowed";
        }
        countdownMs = Math.max(countdownSeconds, Math.min(120, seconds)) * 1000;
        decision = "";
        phase = "asking";
        now = Date.now();
        deadline = now + countdownMs;
        return "asking";
    }

    // Allow: `minutes` > 0 also grants the next minutes, so the program isn't asked again.
    function allow(minutes: int): void {
        if (phase !== "asking") return;
        decision = "allowed";
        phase = "active";
        now = Date.now();
        if (minutes > 0) {
            if (scope.indexOf("mouse") >= 0) grantedUntil = now + minutes * 60000;
            if (scope.indexOf("keyboard") >= 0) grantedKeysUntil = now + minutes * 60000;
        }
    }

    function deny(): void {
        if (phase === "asking") { decision = "denied"; phase = "idle"; }
    }

    // Take the pointer back at any time: stops a running program (it sees "revoked"), ends a grant, closes the card.
    function revoke(): void {
        if (phase === "idle" && grantedUntil === 0 && grantedKeysUntil === 0) return;
        decision = "revoked";
        phase = "idle";
        grantedUntil = 0;
        grantedKeysUntil = 0;
        toastUntil = 0;
        reset();
    }

    function begin(name: string): void {
        if (decision !== "allowed") return; // only after an allowed ask
        who = name || who;
        phase = "active";
    }

    function end(): void {
        if (phase === "active") phase = "idle";
        toastUntil = Math.min(toastUntil, Date.now() + 600);
        reset();
    }

    function status(): string {
        if (phase === "asking") return "asking";
        if (phase === "active") return "active";
        return decision === "" ? "idle" : decision;
    }

    function line(text: string): void {
        const p = text.trim().split(" ");
        if (p[0] === "pos" && p.length >= 3) {
            cx = parseFloat(p[1]); cy = parseFloat(p[2]);
            if (p[3]) outputName = p[3];
            lastSeen = Date.now();
            now = lastSeen;
        } else if (p[0] === "begin") begin(p.slice(1).join(" "));
        else if (p[0] === "end") end();
    }

    // The position stream: a plain file we follow, so every pointer step costs the program one write.
    Process {
        running: true
        command: ["sh", "-c", "d=\"${XDG_RUNTIME_DIR:-/tmp}/ghosthand\"; mkdir -p \"$d\"; f=\"$d/stream\"; : >> \"$f\"; exec tail -n0 -F \"$f\" 2>/dev/null"]
        stdout: SplitParser { onRead: data => root.line(data) }
    }
}
