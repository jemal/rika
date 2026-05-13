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

  radius: 8
  color: launcher.surfaceColor
  border.color: launcher.outlineColor
  border.width: 1
  clip: true

  ColumnLayout {
    anchors.fill: parent
    anchors.margins: 8
    spacing: 2

    Rectangle {
      Layout.fillWidth: true
      Layout.preferredHeight: 34
      color: root.launcher.surfaceColor

      TextField {
        id: input

        anchors.fill: parent
        anchors.leftMargin: 4
        anchors.rightMargin: 4
        focus: true
        color: root.launcher.textColor
        selectedTextColor: root.launcher.surfaceColor
        selectionColor: root.launcher.primaryColor
        placeholderText: "Search"
        placeholderTextColor: root.launcher.mutedTextColor
        verticalAlignment: TextInput.AlignVCenter
        font.pixelSize: 15
        background: null

        onTextChanged: {
          root.launcher.query = text;
          root.launcher.selectedIndex = 0;
          root.launcher.sendQuery(text);
        }

        Keys.onPressed: event => {
          const count = root.launcher.filteredResults().length;

          if (event.key === Qt.Key_Escape) {
            root.launcher.closeLauncher();
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
      spacing: 0
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
        width: parent.width - 32
        horizontalAlignment: Text.AlignHCenter
        text: root.launcher.ipcError.length > 0 ? root.launcher.ipcError : "No results"
        color: root.launcher.ipcError.length > 0 ? root.launcher.errorColor : root.launcher.mutedTextColor
        font.pixelSize: 13
        wrapMode: Text.Wrap
        visible: results.count === 0
      }
    }
  }
}
