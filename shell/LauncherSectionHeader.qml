pragma ComponentBehavior: Bound

import QtQuick

Item {
  id: root

  required property string text
  required property var launcher

  width: ListView.view.width
  height: 26

  Text {
    anchors.left: parent.left
    anchors.right: parent.right
    anchors.verticalCenter: parent.verticalCenter
    anchors.leftMargin: 12
    anchors.rightMargin: 10
    text: root.text
    color: Qt.alpha(root.launcher.mutedTextColor, 0.72)
    font.family: root.launcher.fontFamily
    font.pixelSize: root.launcher.tinyFontSize
    font.weight: Font.DemiBold
    font.capitalization: Font.AllUppercase
    font.letterSpacing: 0.8
    elide: Text.ElideRight
    maximumLineCount: 1
  }
}
