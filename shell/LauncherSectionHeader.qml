pragma ComponentBehavior: Bound

import QtQuick

Item {
  id: root

  required property string text
  required property var launcher

  width: ListView.view.width
  height: 22

  Text {
    anchors.left: parent.left
    anchors.right: parent.right
    anchors.verticalCenter: parent.verticalCenter
    anchors.leftMargin: 10
    anchors.rightMargin: 8
    text: root.text
    color: Qt.alpha(root.launcher.mutedTextColor, 0.72)
    font.family: root.launcher.fontFamily
    font.pixelSize: root.launcher.tinyFontSize
    font.weight: Font.Medium
    elide: Text.ElideRight
    maximumLineCount: 1
  }
}
