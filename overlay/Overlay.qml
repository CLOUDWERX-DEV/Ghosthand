import QtQuick
import QtQuick.Layouts
import QtQuick.Shapes
import Quickshell
import Quickshell.Wayland

// The card that asks you to allow a program to drive your pointer (a countdown, Allow / Allow for N min / Deny), and the "Agent Control"
// tag and ring that follow the cursor while it does. One click-through layer window per screen; it only takes clicks on the card.
Variants {
    model: Quickshell.screens

    PanelWindow {
        id: win
        required property var modelData
        screen: modelData
        // The screen the program is driving; before its first position, the first one.
        readonly property bool mine: Gate.outputName !== "" ? modelData.name === Gate.outputName : modelData === Quickshell.screens[0]
        readonly property bool asking: Gate.phase === "asking"
        property real progress: Gate.cardShown ? 1 : 0
        Behavior on progress { NumberAnimation { duration: Gate.cardShown ? Theme.anim : Theme.anim + 120; easing.type: Easing.OutExpo } }

        anchors { top: true; bottom: true; left: true; right: true }
        visible: mine && (Gate.phase !== "idle" || progress > 0.01)
        color: "transparent"
        exclusionMode: ExclusionMode.Ignore
        WlrLayershell.layer: WlrLayer.Overlay
        WlrLayershell.namespace: "ghosthand"
        // click-through, except the card while it asks. (The session pill is its own window: see SessionPill.qml.)
        mask: asking ? activeRegion : noInput
        Region { id: activeRegion; Region { item: card } }
        Region { id: noInput }

        Rectangle {
            id: card
            anchors.horizontalCenter: parent.horizontalCenter
            y: Math.round(48 * Theme.scale) - (1 - win.progress) * Math.round(24 * Theme.scale)
            width: Math.round(440 * Theme.scale)
            height: body.implicitHeight + Math.round(28 * Theme.scale)
            radius: Math.round(14 * Theme.scale)
            color: Theme.surface
            border.width: 1
            border.color: win.asking ? Qt.alpha(Theme.accent, 0.55) : Theme.hairline
            opacity: Math.min(1, win.progress * 3)
            visible: win.progress > 0.01
            layer.enabled: true
            layer.smooth: true

            // The countdown, along the top edge.
            Rectangle {
                visible: win.asking
                x: Math.round(14 * Theme.scale); y: 0
                height: 3
                width: Math.max(0, (parent.width - 2 * Math.round(14 * Theme.scale)) * Gate.fraction)
                radius: 1.5
                color: Theme.accent
            }

            ColumnLayout {
                id: body
                anchors { left: parent.left; right: parent.right; top: parent.top; margins: Math.round(14 * Theme.scale) }
                spacing: Math.round(10 * Theme.scale)

                RowLayout {
                    Layout.fillWidth: true
                    spacing: Math.round(12 * Theme.scale)
                    Rectangle {
                        Layout.alignment: Qt.AlignTop
                        Layout.preferredWidth: Math.round(38 * Theme.scale)
                        Layout.preferredHeight: Layout.preferredWidth
                        radius: width / 2
                        color: Qt.alpha(Theme.accent, 0.16)
                        GhostGlyph { anchors.centerIn: parent; size: Math.round(26 * Theme.scale); color: Theme.accent }
                    }
                    ColumnLayout {
                        Layout.fillWidth: true
                        spacing: 2
                        Text {
                            Layout.fillWidth: true
                            elide: Text.ElideRight
                            font.family: Theme.font; font.pixelSize: Math.round(14 * Theme.scale); font.weight: Font.DemiBold
                            color: Theme.text
                            text: win.asking ? qsTr("%1 wants to use your %2").arg(Gate.who).arg(Gate.scope) : qsTr("Agent Control is on")
                        }
                        Text {
                            Layout.fillWidth: true
                            wrapMode: Text.Wrap; maximumLineCount: 3
                            font.family: Theme.font; font.pixelSize: Math.round(11 * Theme.scale)
                            color: Theme.muted
                            text: win.asking
                                ? (Gate.reason !== "" ? Gate.reason + "\n" : "")
                                  + (Gate.mode === "notify"
                                     ? qsTr("Starts in %1 s unless you deny.").arg(Math.ceil(Gate.secondsLeft))
                                     : qsTr("Nothing happens until you allow it (%1 s).").arg(Math.ceil(Gate.secondsLeft)))
                                : qsTr("%1 is using your %2. Run `ghosthand revoke` (bind it to a key) to take it back.").arg(Gate.who).arg(Gate.scope)
                        }
                    }
                }

                // What the keyboard is about to type or press, so you read it before you allow it.
                Rectangle {
                    visible: win.asking && Gate.detail !== ""
                    Layout.fillWidth: true
                    implicitHeight: detailText.implicitHeight + Math.round(14 * Theme.scale)
                    radius: Math.round(8 * Theme.scale)
                    color: Theme.raised
                    Text {
                        id: detailText
                        anchors { left: parent.left; right: parent.right; verticalCenter: parent.verticalCenter; margins: Math.round(9 * Theme.scale) }
                        wrapMode: Text.WrapAnywhere; maximumLineCount: 3; elide: Text.ElideRight
                        font.family: "JetBrains Mono, monospace"; font.pixelSize: Math.round(11 * Theme.scale)
                        color: Theme.text
                        text: Gate.detail
                    }
                }

                RowLayout {
                    visible: win.asking
                    Layout.fillWidth: true
                    spacing: Math.round(8 * Theme.scale)
                    Btn { text: qsTr("Deny"); danger: true; onClicked: Gate.deny() }
                    Item { Layout.fillWidth: true }
                    Btn { visible: !Gate.isSessionAsk; text: qsTr("Allow for %1 min").arg(Gate.grantMinutes); onClicked: Gate.allow(Gate.grantMinutes) }
                    Btn { text: Gate.mode === "notify" ? qsTr("Start now") : qsTr("Allow"); primary: true; onClicked: Gate.allow(0) }
                }
            }
        }

        // The ring around the cursor and the tag beside it follow the program's pointer step by step.
        Item {
            id: follow
            readonly property real ring: Math.round(34 * Theme.scale)
            visible: !!Gate.tagShown
            x: Gate.cx || 0; y: Gate.cy || 0
            Behavior on x { NumberAnimation { duration: 70; easing.type: Easing.OutQuad } }
            Behavior on y { NumberAnimation { duration: 70; easing.type: Easing.OutQuad } }

            Rectangle {
                x: -follow.ring / 2; y: -follow.ring / 2
                width: follow.ring; height: follow.ring; radius: width / 2
                color: "transparent"
                border.width: 2; border.color: Qt.alpha(Theme.accent, 0.8)
                SequentialAnimation on scale {
                    running: follow.visible; loops: Animation.Infinite
                    NumberAnimation { to: 1.18; duration: 650; easing.type: Easing.InOutSine }
                    NumberAnimation { to: 1.0; duration: 650; easing.type: Easing.InOutSine }
                }
            }
            Rectangle {
                id: tag
                x: Math.round(18 * Theme.scale)
                y: Math.round(20 * Theme.scale)
                // Keep the tag on screen when the pointer is at the right or bottom edge.
                transform: Translate {
                    x: Math.min(0, win.width - (follow.x + tag.x + tag.width) - 8)
                    y: Math.min(0, win.height - (follow.y + tag.y + tag.height) - 8)
                }
                width: tagRow.implicitWidth + Math.round(20 * Theme.scale)
                height: Math.round(26 * Theme.scale)
                radius: height / 2
                color: Theme.surface
                border.width: 1; border.color: Qt.alpha(Theme.accent, 0.6)
                layer.enabled: true
                RowLayout {
                    id: tagRow
                    anchors.centerIn: parent
                    spacing: Math.round(7 * Theme.scale)
                    Rectangle {
                        implicitWidth: Math.round(7 * Theme.scale); implicitHeight: implicitWidth; radius: width / 2; color: Theme.accent
                        SequentialAnimation on opacity {
                            running: follow.visible; loops: Animation.Infinite
                            NumberAnimation { to: 0.35; duration: 700; easing.type: Easing.InOutSine }
                            NumberAnimation { to: 1.0; duration: 700; easing.type: Easing.InOutSine }
                        }
                    }
                    Text { font.family: Theme.font; font.pixelSize: Math.round(11 * Theme.scale); font.weight: Font.DemiBold; color: Theme.text; text: qsTr("Agent Control") }
                }
            }
        }
    }
}
