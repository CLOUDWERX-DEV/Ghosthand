import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Wayland

// One copy of the session pill: "Agent Control · mouse and keyboard · 24:10 left" and an End button. SessionPill.qml keeps two of these
// and swaps them so the pill is always the newest surface in the overlay layer (and so always on top).
PanelWindow {
    id: win
    property bool on: false
    // True while the pointer is over the pill or pressing on it: the pill is never swapped out from under a click.
    readonly property bool busy: endBtn.hovered || endBtn.pressed

    visible: on
    color: "transparent"
    anchors { bottom: true }
    margins { bottom: Math.round(36 * Theme.scale) }
    exclusionMode: ExclusionMode.Ignore
    WlrLayershell.layer: WlrLayer.Overlay
    WlrLayershell.namespace: "ghosthand-session"
    implicitWidth: pill.width
    implicitHeight: pill.height

    Rectangle {
        id: pill
        readonly property int padLeft: Math.round(14 * Theme.scale)
        readonly property int padRight: Math.round(5 * Theme.scale)
        width: padLeft + row.implicitWidth + padRight
        height: Math.round(34 * Theme.scale)
        radius: height / 2
        color: Theme.surface
        border.width: 1
        border.color: Qt.alpha(Theme.accent, 0.6)

        // The clock changes width every second; reserve the widest it can be so nothing in the pill shifts.
        TextMetrics {
            id: widest
            font: clock.font
            text: Gate.sessionLeft >= 3600 ? "8:88:88 left" : Gate.sessionLeft >= 600 ? "88:88 left" : "8:88 left"
        }

        RowLayout {
            id: row
            x: pill.padLeft
            anchors.verticalCenter: parent.verticalCenter
            spacing: Math.round(9 * Theme.scale)
            Rectangle {
                implicitWidth: Math.round(8 * Theme.scale); implicitHeight: implicitWidth; radius: width / 2; color: Theme.accent
                SequentialAnimation on opacity {
                    running: win.on; loops: Animation.Infinite
                    NumberAnimation { to: 0.35; duration: 800; easing.type: Easing.InOutSine }
                    NumberAnimation { to: 1.0; duration: 800; easing.type: Easing.InOutSine }
                }
            }
            Text {
                font.family: Theme.font; font.pixelSize: Math.round(12 * Theme.scale); font.weight: Font.DemiBold; color: Theme.text
                text: qsTr("Agent Control · %1 ·").arg(Gate.sessionScope())
            }
            Text {
                id: clock
                Layout.preferredWidth: Math.ceil(widest.width)
                font.family: Theme.font; font.pixelSize: Math.round(12 * Theme.scale); font.weight: Font.DemiBold; color: Theme.text
                text: qsTr("%1 left").arg(Gate.sessionClock)
            }
            Btn {
                id: endBtn
                text: qsTr("End")
                danger: true
                implicitHeight: Math.round(24 * Theme.scale)
                radius: height / 2
                onClicked: Gate.endTheSession()
            }
        }
    }
}
