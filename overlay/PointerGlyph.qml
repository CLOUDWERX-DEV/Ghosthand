import QtQuick
import QtQuick.Shapes

// A mouse pointer, drawn: the card's icon, so the overlay needs no icon font.
Item {
    id: root
    property real size: 20
    property color color: "white"
    implicitWidth: size
    implicitHeight: size
    Shape {
        anchors.fill: parent
        preferredRendererType: Shape.CurveRenderer
        ShapePath {
            fillColor: root.color
            strokeColor: "transparent"
            scale: Qt.size(root.size / 24, root.size / 24)
            PathSvg { path: "M5 3 L5 19 L9.2 15 L12 21.5 L14.7 20.3 L12 13.9 L18 13.9 Z" }
        }
    }
}
