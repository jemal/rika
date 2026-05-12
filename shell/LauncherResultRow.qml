pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Layouts

Rectangle {
  id: root

  required property int rowIndex
  required property var result
  required property var launcher

  width: ListView.view.width
  height: 68
  radius: 10
  color: rowIndex === launcher.selectedIndex ? launcher.hoverColor : launcher.surfaceVariantColor
  border.color: rowIndex === launcher.selectedIndex ? launcher.primaryColor : "transparent"
  border.width: 1
  clip: true

  Behavior on color {
    ColorAnimation {
      duration: 120
      easing.type: Easing.OutCubic
    }
  }

  RowLayout {
    anchors.fill: parent
    anchors.leftMargin: 12
    anchors.rightMargin: 12
    spacing: 12

    Rectangle {
      Layout.preferredWidth: 42
      Layout.preferredHeight: 42
      Layout.alignment: Qt.AlignVCenter
      radius: 10
      color: root.rowIndex === root.launcher.selectedIndex ? "#365074" : root.launcher.surfaceColor

      Text {
        anchors.centerIn: parent
        text: root.result.icon
        color: "#dce7f8"
        font.pixelSize: 16
        font.bold: true
      }
    }

    ColumnLayout {
      Layout.fillWidth: true
      Layout.alignment: Qt.AlignVCenter
      spacing: 2

      Text {
        Layout.fillWidth: true
        text: root.result.title
        color: root.launcher.textColor
        font.pixelSize: 17
        font.bold: true
        elide: Text.ElideRight
        maximumLineCount: 1
      }

      Text {
        Layout.fillWidth: true
        text: root.result.subtitle
        color: root.launcher.mutedTextColor
        font.pixelSize: 13
        elide: Text.ElideRight
        maximumLineCount: 1
      }
    }

    Text {
      Layout.preferredWidth: 92
      Layout.alignment: Qt.AlignRight | Qt.AlignVCenter
      horizontalAlignment: Text.AlignRight
      text: root.result.kind
      color: "#a9bddc"
      font.pixelSize: 12
      elide: Text.ElideRight
    }
  }

  MouseArea {
    anchors.fill: parent
    hoverEnabled: true

    onEntered: root.launcher.selectedIndex = root.rowIndex
    onClicked: root.launcher.activateSelection()
  }
}
