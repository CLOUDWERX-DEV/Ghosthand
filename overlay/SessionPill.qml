import QtQuick
import Quickshell

// The session pill, on the screen the program is driving. It must stay on top of everything, and the Wayland layer-shell has nothing above
// the overlay layer; inside that layer, surfaces stack in the order they were mapped, so anything that maps later (a launcher, a drop-down
// terminal, a shell panel) lands over a pill that was mapped earlier, and takes its clicks too. So the pill keeps two identical windows and
// every couple of seconds maps the spare one (it appears above everything) and then unmaps the old one. Nothing visibly changes.
// A pill the pointer is on is left alone, so a click on End never loses its window.
Variants {
    model: Quickshell.screens

    Scope {
        id: sc
        required property var modelData
        readonly property bool mine: Gate.outputName !== "" ? modelData.name === Gate.outputName : modelData === Quickshell.screens[0]
        readonly property bool shown: mine && Gate.sessionOn
        property bool aOn: false
        property bool bOn: false
        property bool front: true     // true: window a is the newest

        onShownChanged: {
            front = true;
            aOn = shown;
            bOn = false;
        }

        function raise(): void {
            if (!shown || a.busy || b.busy) return;
            if (front) { bOn = true; hideA.restart(); } else { aOn = true; hideB.restart(); }
            front = !front;
        }

        Timer { interval: 2000; repeat: true; running: sc.shown; onTriggered: sc.raise() }
        // the old window goes only after the new one has had time to draw
        Timer { id: hideA; interval: 300; onTriggered: if (!sc.front) sc.aOn = false }
        Timer { id: hideB; interval: 300; onTriggered: if (sc.front) sc.bOn = false }

        PillWindow { id: a; screen: sc.modelData; on: sc.aOn }
        PillWindow { id: b; screen: sc.modelData; on: sc.bOn }
    }
}
