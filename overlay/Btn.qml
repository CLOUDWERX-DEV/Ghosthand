import QtQuick

// A small button: `primary` fills it with the accent (the answer you're asked for), `danger` tints it red on hover (deny).
Rectangle {
    id: root
    property string text: ""
    property bool primary: false
    property bool danger: false
    signal clicked
    readonly property bool hovered: area.containsMouse
    readonly property bool pressed: area.pressed

    implicitWidth: label.implicitWidth + Math.round(24 * Theme.scale)
    implicitHeight: Math.round(32 * Theme.scale)
    radius: Math.round(9 * Theme.scale)
    color: primary ? (area.containsMouse ? Qt.lighter(Theme.accent, 1.12) : Theme.accent)
         : area.containsMouse ? (danger ? Qt.alpha(Theme.danger, 0.25) : Qt.alpha(Theme.accent, 0.35)) : Theme.raised
    Behavior on color { ColorAnimation { duration: 120 } }

    Text {
        id: label
        anchors.centerIn: parent
        text: root.text
        font.family: Theme.font
        font.pixelSize: Math.round(12 * Theme.scale)
        color: root.primary ? Theme.textOnAccent : area.containsMouse && root.danger ? Theme.danger : Theme.text
    }
    MouseArea {
        id: area
        anchors.fill: parent
        hoverEnabled: true
        cursorShape: Qt.PointingHandCursor
        onClicked: root.clicked()
    }
}
