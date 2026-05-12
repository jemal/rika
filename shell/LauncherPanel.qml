pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

Rectangle {
  id: root

  required property var launcher

  function focusSearchInput() {
    input.forceActiveFocus();
  }

  radius: 16
  color: launcher.surfaceColor
  border.color: launcher.outlineColor
  border.width: 1
  clip: true

  ColumnLayout {
    anchors.fill: parent
    anchors.margins: 14
    spacing: 10

    Rectangle {
      Layout.fillWidth: true
      Layout.preferredHeight: 54
      radius: 12
      color: root.launcher.surfaceVariantColor
      border.color: input.activeFocus ? root.launcher.primaryColor : root.launcher.outlineColor
      border.width: 1

      TextField {
        id: input

        anchors.fill: parent
        anchors.leftMargin: 14
        anchors.rightMargin: 14
        focus: true
        color: root.launcher.textColor
        selectedTextColor: "#ffffff"
        selectionColor: "#355c9d"
        placeholderText: "Search apps, projects, commands, and web bangs"
        placeholderTextColor: root.launcher.mutedTextColor
        verticalAlignment: TextInput.AlignVCenter
        font.pixelSize: 21
        background: null

        onTextChanged: {
          root.launcher.query = text;
          root.launcher.selectedIndex = 0;
          root.launcher.sendQuery(text);
        }

        Keys.onPressed: event => {
          const count = root.launcher.filteredResults().length;

          if (event.key === Qt.Key_Escape) {
            root.launcher.open = false;
            event.accepted = true;
          } else if (event.key === Qt.Key_Down && count > 0) {
            root.launcher.selectedIndex = Math.min(root.launcher.selectedIndex + 1, count - 1);
            event.accepted = true;
          } else if (event.key === Qt.Key_Up && count > 0) {
            root.launcher.selectedIndex = Math.max(root.launcher.selectedIndex - 1, 0);
            event.accepted = true;
          } else if (event.key === Qt.Key_Return || event.key === Qt.Key_Enter) {
            root.launcher.activateSelection();
            event.accepted = true;
          }
        }
      }
    }

    ListView {
      id: results

      Layout.fillWidth: true
      Layout.fillHeight: true
      clip: true
      spacing: 6
      boundsBehavior: Flickable.StopAtBounds
      model: root.launcher.filteredResults()

      onCountChanged: root.launcher.clampSelection()

      delegate: LauncherResultRow {
        required property int index
        required property var modelData

        width: ListView.view.width
        rowIndex: index
        result: modelData
        launcher: root.launcher
      }

      Text {
        anchors.centerIn: parent
        text: "No results"
        color: root.launcher.mutedTextColor
        font.pixelSize: 15
        visible: results.count === 0
      }
    }
  }
}
