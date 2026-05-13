pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Layouts

Rectangle {
  id: root

  required property int rowIndex
  required property var result
  required property var launcher
  readonly property bool showSubtitle: result.subtitle.length > 0 && result.subtitle !== result.title

  width: ListView.view.width
  height: 34
  radius: 5
  color: rowIndex === launcher.selectedIndex ? launcher.hoverColor : "transparent"
  clip: true

  Behavior on color {
    ColorAnimation {
      duration: 120
      easing.type: Easing.OutCubic
    }
  }

  RowLayout {
    anchors.fill: parent
    anchors.leftMargin: 8
    anchors.rightMargin: 8
    spacing: 8

    Text {
      Layout.preferredWidth: 18
      Layout.preferredHeight: 16
      Layout.alignment: Qt.AlignVCenter
      horizontalAlignment: Text.AlignHCenter
      verticalAlignment: Text.AlignVCenter
      text: root.result.icon
      color: root.rowIndex === root.launcher.selectedIndex ? root.launcher.primaryColor : root.launcher.mutedTextColor
      font.pixelSize: 12
      font.bold: true
    }

    RowLayout {
      Layout.fillWidth: true
      Layout.alignment: Qt.AlignVCenter
      spacing: 8

      Text {
        Layout.preferredWidth: Math.min(260, implicitWidth)
        text: root.result.title
        color: root.launcher.textColor
        font.pixelSize: 14
        font.weight: Font.Medium
        elide: Text.ElideRight
        maximumLineCount: 1
      }

      Text {
        Layout.fillWidth: true
        text: root.result.subtitle
        color: root.launcher.mutedTextColor
        font.pixelSize: 11
        elide: Text.ElideRight
        maximumLineCount: 1
        visible: root.showSubtitle
      }
    }

    RowLayout {
      Layout.preferredWidth: 88
      Layout.alignment: Qt.AlignRight | Qt.AlignVCenter
      spacing: 8

      Text {
        Layout.preferredWidth: 42
        horizontalAlignment: Text.AlignRight
        text: root.result.provider
        color: root.launcher.mutedTextColor
        font.pixelSize: 10
        elide: Text.ElideRight
      }

      Text {
        Layout.preferredWidth: 38
        horizontalAlignment: Text.AlignRight
        text: root.result.actions.length > 0 ? root.result.actions[0] : ""
        color: root.rowIndex === root.launcher.selectedIndex ? root.launcher.accentColor : root.launcher.mutedTextColor
        font.pixelSize: 10
        elide: Text.ElideRight
        visible: root.rowIndex === root.launcher.selectedIndex
      }
    }
  }

  MouseArea {
    anchors.fill: parent
    hoverEnabled: true

    onEntered: root.launcher.selectedIndex = root.rowIndex
    onClicked: root.launcher.activateSelection()
  }
}
