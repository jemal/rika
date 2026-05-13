pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Widgets

Rectangle {
  id: root

  required property int rowIndex
  required property var result
  required property var launcher
  readonly property bool showSubtitle: result.subtitle.length > 0 && result.subtitle !== result.title
  readonly property string iconSource: resolveIconSource(result.icon)

  function resolveIconSource(icon) {
    if (!icon || icon.length === 0) {
      return "";
    }

    if (icon.startsWith("/")) {
      return `file://${icon}`;
    }

    const path = Quickshell.iconPath(icon, "application-x-executable");
    return path && path.length > 0 ? path : "";
  }

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

    Item {
      Layout.preferredWidth: 24
      Layout.preferredHeight: 24
      Layout.alignment: Qt.AlignVCenter

      IconImage {
        anchors.centerIn: parent
        width: 18
        height: 18
        source: root.iconSource
        asynchronous: true
        visible: root.iconSource.length > 0
      }

      Text {
        anchors.centerIn: parent
        horizontalAlignment: Text.AlignHCenter
        verticalAlignment: Text.AlignVCenter
        text: root.result.title.length > 0 ? root.result.title[0].toUpperCase() : "?"
        color: root.rowIndex === root.launcher.selectedIndex ? root.launcher.primaryColor : root.launcher.mutedTextColor
        font.pixelSize: 11
        font.weight: Font.Medium
        visible: root.iconSource.length === 0
      }
    }

    ColumnLayout {
      Layout.fillWidth: true
      Layout.alignment: Qt.AlignVCenter
      spacing: 0

      Text {
        Layout.fillWidth: true
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
        font.pixelSize: 10
        elide: Text.ElideRight
        maximumLineCount: 1
        visible: root.showSubtitle
      }
    }

    RowLayout {
      Layout.preferredWidth: 78
      Layout.alignment: Qt.AlignRight | Qt.AlignVCenter
      spacing: 6

      Text {
        Layout.preferredWidth: 44
        horizontalAlignment: Text.AlignRight
        text: root.result.provider
        color: root.launcher.mutedTextColor
        font.pixelSize: 10
        elide: Text.ElideRight
      }

      Text {
        Layout.preferredWidth: 28
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
