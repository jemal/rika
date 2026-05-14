pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Layouts
import Quickshell.Widgets

Rectangle {
  id: root

  required property int rowIndex
  required property var result
  required property var launcher
  readonly property bool showSubtitle: result.subtitle.length > 0 && result.subtitle !== result.title
  readonly property string iconSource: launcher.resolveIconSource(result.icon)
  readonly property bool selected: rowIndex === launcher.selectedIndex

  width: ListView.view.width
  height: 34
  radius: 5
  color: selected ? launcher.hoverColor : "transparent"
  clip: true

  Behavior on color {
    ColorAnimation {
      duration: 120
      easing.type: Easing.OutCubic
    }
  }

  Rectangle {
    anchors.left: parent.left
    anchors.leftMargin: 2
    anchors.verticalCenter: parent.verticalCenter
    width: 2
    height: root.selected ? 18 : 0
    radius: 1
    color: root.launcher.primaryColor
    opacity: root.selected ? 1 : 0

    Behavior on height {
      NumberAnimation {
        duration: 120
        easing.type: Easing.OutCubic
      }
    }

    Behavior on opacity {
      NumberAnimation {
        duration: 120
        easing.type: Easing.OutCubic
      }
    }
  }

  RowLayout {
    anchors.fill: parent
    anchors.leftMargin: 10
    anchors.rightMargin: 8
    spacing: 8
    opacity: root.selected ? 1 : 0.86

    Behavior on opacity {
      NumberAnimation {
        duration: 120
        easing.type: Easing.OutCubic
      }
    }

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
        color: root.selected ? root.launcher.primaryColor : root.launcher.mutedTextColor
        font.family: root.launcher.fontFamily
        font.pixelSize: root.launcher.smallFontSize
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
        font.family: root.launcher.fontFamily
        font.pixelSize: root.launcher.fontSize
        font.weight: Font.Medium
        elide: Text.ElideRight
        maximumLineCount: 1
      }

      Text {
        Layout.fillWidth: true
        text: root.result.subtitle
        color: root.launcher.mutedTextColor
        font.family: root.launcher.fontFamily
        font.pixelSize: root.launcher.tinyFontSize
        elide: Text.ElideRight
        maximumLineCount: 1
        visible: root.showSubtitle
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
