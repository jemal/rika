pragma ComponentBehavior: Bound

import QtQuick
import Qt5Compat.GraphicalEffects

Item {
  id: root

  property url source
  property color color: "white"

  Image {
    id: icon

    anchors.fill: parent
    source: root.source
    sourceSize.width: Math.max(1, root.width)
    sourceSize.height: Math.max(1, root.height)
    fillMode: Image.PreserveAspectFit
    smooth: true
    asynchronous: true
  }

  ColorOverlay {
    anchors.fill: icon
    source: icon
    color: root.color
    cached: false
  }
}
