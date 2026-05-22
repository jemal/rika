pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Layouts
import Quickshell.Widgets

Rectangle {
  id: root

  required property int rowIndex
  required property var result
  required property var launcher
  readonly property bool showSubtitle: launcher.resultHasSubtitle(result)
  readonly property string iconSource: launcher.resolveIconSource(result.icon)
  readonly property bool tintedIcon: result.icon.startsWith("builtin:")
  readonly property bool selected: rowIndex === launcher.selectedIndex

  width: ListView.view.width
  height: 44
  radius: 5
  color: "transparent"
  clip: true

  Rectangle {
    anchors.left: parent.left
    anchors.right: parent.right
    anchors.top: parent.top
    anchors.bottom: parent.bottom
    radius: 5
    color: root.selected ? root.launcher.hoverColor : "transparent"

    Behavior on color {
      ColorAnimation {
        duration: 120
        easing.type: Easing.OutCubic
      }
    }
  }

  Rectangle {
    anchors.left: parent.left
    anchors.leftMargin: 2
    y: Math.round((root.height - height) / 2)
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
    anchors.left: parent.left
    anchors.right: parent.right
    anchors.top: parent.top
    anchors.bottom: parent.bottom
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

      Loader {
        anchors.centerIn: parent
        width: 18
        height: 18
        active: root.iconSource.length > 0 && !root.tintedIcon

        sourceComponent: IconImage {
          source: root.iconSource
          asynchronous: true
        }
      }

      Loader {
        id: tintedIconLoader

        property string resolvedSource: root.iconSource
        property color resolvedColor: root.selected ? root.launcher.primaryColor : root.launcher.textColor

        anchors.centerIn: parent
        width: 18
        height: 18
        active: root.iconSource.length > 0 && root.tintedIcon

        sourceComponent: TintedIcon {
          width: 18
          height: 18
          source: tintedIconLoader.resolvedSource
          color: tintedIconLoader.resolvedColor
        }
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
      spacing: root.showSubtitle ? 1 : 0

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
