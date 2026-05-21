pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

Rectangle {
  id: root

  required property var launcher
  property bool applyingAutocomplete: false

  function focusSearchInput() {
    input.forceActiveFocus();
  }

  function resetInput() {
    input.text = "";
  }

  function positionSelectedResult() {
    results.positionViewAtIndex(launcher.selectedIndex, ListView.Contain);
  }

  function applyAutocomplete() {
    if (launcher.actionMode) {
      return;
    }

    const results = launcher.filteredResults();
    const result = results[launcher.selectedIndex];
    if (result && result.autocomplete) {
      root.applyingAutocomplete = true;
      input.text = result.autocomplete;
      input.cursorPosition = input.text.length;
      root.applyingAutocomplete = false;
    }
  }

  function selectNextResult() {
    const count = launcher.filteredResults().length;

    if (count === 0) {
      return;
    }

    const currentIndex = Math.max(0, Math.min(launcher.selectedIndex, count - 1));
    launcher.selectedIndex = (currentIndex + 1) % count;
    positionSelectedResult();
    applyAutocomplete();
  }

  function selectPreviousResult() {
    const count = launcher.filteredResults().length;

    if (count === 0) {
      return;
    }

    const currentIndex = Math.max(0, Math.min(launcher.selectedIndex, count - 1));
    launcher.selectedIndex = currentIndex === 0 ? count - 1 : currentIndex - 1;
    positionSelectedResult();
    applyAutocomplete();
  }

  function handleAltNavigation(event) {
    if (!(event.modifiers & Qt.AltModifier)) {
      return false;
    }

    const count = launcher.filteredResults().length;

    if (event.key === Qt.Key_J && count > 0) {
      if (root.launcher.actionMode) {
        root.launcher.selectNextAction();
      } else {
        selectNextResult();
      }
      return true;
    }

    if (event.key === Qt.Key_K && count > 0) {
      if (root.launcher.actionMode) {
        root.launcher.selectPreviousAction();
      } else {
        selectPreviousResult();
      }
      return true;
    }

    if (event.key === Qt.Key_H) {
      input.cursorPosition = Math.max(0, input.cursorPosition - 1);
      return true;
    }

    if (event.key === Qt.Key_L) {
      input.cursorPosition = Math.min(input.length, input.cursorPosition + 1);
      return true;
    }

    return false;
  }

  function handleKeybind(event) {
    if (handleAltNavigation(event)) {
      event.accepted = true;
      return true;
    }

    return false;
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
        text: root.launcher.query
        color: root.launcher.textColor
        selectedTextColor: root.launcher.surfaceColor
        selectionColor: root.launcher.primaryColor
        placeholderText: "Search"
        placeholderTextColor: root.launcher.mutedTextColor
        verticalAlignment: TextInput.AlignVCenter
        font.family: root.launcher.fontFamily
        font.pixelSize: root.launcher.fontSize + 1
        background: null

        onTextChanged: {
          if (root.applyingAutocomplete || root.launcher.query === text) {
            return;
          }

          root.launcher.exitActionMode();
          root.launcher.ipcError = "";
          root.launcher.query = text;
          root.launcher.selectedIndex = 0;
          root.launcher.sendQuery(text);
        }

        Keys.onPressed: event => {
          const count = root.launcher.filteredResults().length;

          if (root.handleKeybind(event)) {
            return;
          }

          if (root.launcher.actionMode) {
            if (event.key === Qt.Key_Escape || event.key === Qt.Key_Left || event.key === Qt.Key_Backspace) {
              root.launcher.exitActionMode();
              event.accepted = true;
            } else if (event.key === Qt.Key_Down) {
              root.launcher.selectNextAction();
              event.accepted = true;
            } else if (event.key === Qt.Key_Up) {
              root.launcher.selectPreviousAction();
              event.accepted = true;
            } else if (event.key === Qt.Key_Return || event.key === Qt.Key_Enter) {
              root.launcher.activateSelectedAction();
              event.accepted = true;
            }

            return;
          }

          if (event.key === Qt.Key_K && event.modifiers & Qt.ControlModifier && count > 0) {
            root.launcher.enterActionMode();
            event.accepted = true;
          } else if (event.key === Qt.Key_Escape) {
            root.launcher.closeLauncher();
            event.accepted = true;
          } else if (event.key === Qt.Key_R && event.modifiers & Qt.ControlModifier) {
            root.launcher.refreshProviders();
            event.accepted = true;
          } else if (event.key === Qt.Key_Down && count > 0) {
            root.selectNextResult();
            event.accepted = true;
          } else if (event.key === Qt.Key_Up && count > 0) {
            root.selectPreviousResult();
            event.accepted = true;
          } else if (event.key === Qt.Key_Backtab && count > 0) {
            root.selectPreviousResult();
            event.accepted = true;
          } else if (event.key === Qt.Key_Tab && count > 0) {
            if (event.modifiers & Qt.ShiftModifier) {
              root.selectPreviousResult();
            } else {
              root.selectNextResult();
            }
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
      visible: !root.launcher.actionMode
      clip: true
      spacing: 0
      boundsBehavior: Flickable.StopAtBounds
      model: root.launcher.filteredResults()
      add: Transition {
        NumberAnimation {
          properties: "opacity"
          from: 0
          to: 1
          duration: 100
          easing.type: Easing.OutCubic
        }
      }
      displaced: Transition {
        NumberAnimation {
          properties: "x,y"
          duration: 120
          easing.type: Easing.OutCubic
        }
      }
      remove: Transition {
        NumberAnimation {
          properties: "opacity"
          to: 0
          duration: 80
          easing.type: Easing.OutCubic
        }
      }

      onCountChanged: root.launcher.clampSelection()

      delegate: LauncherResultRow {
        required property int index
        required property var modelData

        width: ListView.view.width
        rowIndex: index
        result: modelData
        launcher: root.launcher
      }

      Column {
        anchors.centerIn: parent
        width: parent.width - 32
        spacing: 6
        visible: results.count === 0

        Text {
          width: parent.width
          horizontalAlignment: Text.AlignHCenter
          text: root.launcher.ipcError.length > 0 ? root.launcher.ipcError : "No results"
          color: root.launcher.ipcError.length > 0 ? root.launcher.errorColor : root.launcher.mutedTextColor
          font.family: root.launcher.fontFamily
          font.pixelSize: root.launcher.ipcError.length > 0 ? root.launcher.fontSize + 2 : root.launcher.fontSize
          font.weight: root.launcher.ipcError.length > 0 ? Font.Medium : Font.Normal
          wrapMode: Text.Wrap
        }

        Text {
          width: parent.width
          horizontalAlignment: Text.AlignHCenter
          text: "Type to dismiss"
          color: root.launcher.mutedTextColor
          font.family: root.launcher.fontFamily
          font.pixelSize: root.launcher.smallFontSize
          visible: root.launcher.ipcError.length > 0
        }
      }
    }

    ListView {
      id: actions

      Layout.fillWidth: true
      Layout.fillHeight: true
      visible: root.launcher.actionMode
      clip: true
      spacing: 0
      boundsBehavior: Flickable.StopAtBounds
      model: root.launcher.selectedActions()

      onCountChanged: root.launcher.clampActionSelection()

      delegate: Rectangle {
        id: actionRow

        required property int index
        required property var modelData

        readonly property bool selected: index === root.launcher.selectedActionIndex
        readonly property string iconSource: root.launcher.resolveIconSource(modelData.icon)
        readonly property bool tintedIcon: modelData.icon && modelData.icon.startsWith("builtin:")

        width: ListView.view.width
        height: 34
        radius: 5
        color: selected ? root.launcher.hoverColor : "transparent"
        clip: true

        RowLayout {
          anchors.fill: parent
          anchors.leftMargin: 10
          anchors.rightMargin: 8
          spacing: 8
          opacity: parent.selected ? 1 : 0.86

          Item {
            Layout.preferredWidth: 24
            Layout.preferredHeight: 24
            Layout.alignment: Qt.AlignVCenter

            Loader {
              id: actionIconLoader

              property string resolvedSource: actionRow.iconSource
              property color resolvedColor: actionRow.selected ? root.launcher.primaryColor : root.launcher.textColor

              anchors.centerIn: parent
              width: 18
              height: 18
              active: actionRow.iconSource.length > 0 && actionRow.tintedIcon

              sourceComponent: TintedIcon {
                width: 18
                height: 18
                source: actionIconLoader.resolvedSource
                color: actionIconLoader.resolvedColor
              }
            }

            Text {
              anchors.centerIn: parent
              horizontalAlignment: Text.AlignHCenter
              verticalAlignment: Text.AlignVCenter
              text: actionRow.modelData.label.length > 0 ? actionRow.modelData.label[0].toUpperCase() : "?"
              color: actionRow.selected ? root.launcher.primaryColor : root.launcher.mutedTextColor
              font.family: root.launcher.fontFamily
              font.pixelSize: root.launcher.smallFontSize
              font.weight: Font.Medium
              visible: actionRow.iconSource.length === 0
            }
          }

          Text {
            Layout.fillWidth: true
            Layout.alignment: Qt.AlignVCenter
            text: actionRow.modelData.label
            color: root.launcher.textColor
            font.family: root.launcher.fontFamily
            font.pixelSize: root.launcher.fontSize
            font.weight: Font.Medium
            elide: Text.ElideRight
            maximumLineCount: 1
          }
        }

        MouseArea {
          anchors.fill: parent
          hoverEnabled: true

          onEntered: root.launcher.selectedActionIndex = actionRow.index
          onClicked: root.launcher.activateSelectedAction()
        }
      }
    }

    RowLayout {
      Layout.fillWidth: true
      Layout.preferredHeight: 22
      spacing: 8

      Text {
        Layout.fillWidth: true
        text: {
          if (root.launcher.footerStatus.length > 0) {
            return root.launcher.footerStatus;
          }

          if (root.launcher.actionMode) {
            const count = root.launcher.selectedActions().length;
            return count > 0 ? `Action ${root.launcher.selectedActionIndex + 1}/${count}` : "Action 0/0";
          }

          const count = root.launcher.filteredResults().length;
          return count > 0 ? `${root.launcher.selectedIndex + 1}/${count}` : "0/0";
        }
        color: root.launcher.footerStatus.length > 0 ? root.launcher.accentColor : root.launcher.mutedTextColor
        font.family: root.launcher.fontFamily
        font.pixelSize: root.launcher.smallFontSize
      }

      Text {
        text: root.launcher.actionMode ? "Run" : "Open"
        color: root.launcher.textColor
        font.family: root.launcher.fontFamily
        font.pixelSize: root.launcher.smallFontSize
      }

      Text {
        text: "enter"
        color: root.launcher.mutedTextColor
        font.family: root.launcher.fontFamily
        font.pixelSize: root.launcher.smallFontSize
      }

      Text {
        text: root.launcher.actionMode ? "Back" : "Actions"
        color: root.launcher.textColor
        font.family: root.launcher.fontFamily
        font.pixelSize: root.launcher.smallFontSize
      }

      Text {
        text: root.launcher.actionMode ? "esc" : "ctrl-k"
        color: root.launcher.mutedTextColor
        font.family: root.launcher.fontFamily
        font.pixelSize: root.launcher.smallFontSize
      }

      Text {
        text: root.launcher.actionMode ? "" : "Refresh"
        color: root.launcher.textColor
        font.family: root.launcher.fontFamily
        font.pixelSize: root.launcher.smallFontSize
        visible: !root.launcher.actionMode
      }

      Text {
        text: "ctrl-r"
        color: root.launcher.mutedTextColor
        font.family: root.launcher.fontFamily
        font.pixelSize: root.launcher.smallFontSize
        visible: !root.launcher.actionMode
      }

      Text {
        text: "Close"
        color: root.launcher.textColor
        font.family: root.launcher.fontFamily
        font.pixelSize: root.launcher.smallFontSize
        visible: !root.launcher.actionMode
      }

      Text {
        text: "esc"
        color: root.launcher.mutedTextColor
        font.family: root.launcher.fontFamily
        font.pixelSize: root.launcher.smallFontSize
        visible: !root.launcher.actionMode
      }
    }
  }
}
