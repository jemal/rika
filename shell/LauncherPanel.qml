pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import Quickshell.Widgets

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

  function resetResultScroll() {
    results.positionViewAtBeginning();
  }

  function positionSelectedResult(positionMode) {
    const count = launcher.filteredResults().length;
    if (launcher.selectedIndex < 0 || launcher.selectedIndex >= count) {
      return;
    }

    const displayIndex = launcher.displayIndexForResult(launcher.selectedIndex);
    if (displayIndex < 0) {
      return;
    }

    const mode = positionMode === undefined ? ListView.Contain : positionMode;
    const scrollIndex = mode === ListView.End ? displayIndex : launcher.scrollIndexForResult(launcher.selectedIndex);
    results.positionViewAtIndex(scrollIndex, mode);
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

  function applySelectedAutocomplete() {
    if (launcher.actionMode) {
      return false;
    }

    const results = launcher.filteredResults();
    const result = results[launcher.selectedIndex];
    if (!result || !result.autocomplete || result.autocomplete === input.text) {
      return false;
    }

    root.applyingAutocomplete = true;
    input.text = result.autocomplete;
    input.cursorPosition = input.text.length;
    root.applyingAutocomplete = false;
    return true;
  }

  function selectNextResult() {
    const count = launcher.filteredResults().length;

    if (count === 0) {
      return;
    }

    const currentIndex = Math.max(0, Math.min(launcher.selectedIndex, count - 1));
    launcher.selectedIndex = (currentIndex + 1) % count;
    positionSelectedResult(launcher.selectedIndex === 0 ? ListView.Beginning : ListView.Contain);
    applyAutocomplete();
  }

  function selectPreviousResult() {
    const count = launcher.filteredResults().length;

    if (count === 0) {
      return;
    }

    const currentIndex = Math.max(0, Math.min(launcher.selectedIndex, count - 1));
    launcher.selectedIndex = currentIndex === 0 ? count - 1 : currentIndex - 1;
    positionSelectedResult(launcher.selectedIndex === count - 1 ? ListView.End : ListView.Contain);
    applyAutocomplete();
  }

  function selectNextSection() {
    const results = launcher.filteredResults();
    const count = results.length;

    if (count === 0) {
      return;
    }

    const currentIndex = Math.max(0, Math.min(launcher.selectedIndex, count - 1));
    const currentSection = results[currentIndex].section || "";

    for (let i = currentIndex + 1; i < count; i += 1) {
      if ((results[i].section || "") !== currentSection) {
        launcher.selectedIndex = i;
        positionSelectedResult(ListView.Contain);
        applyAutocomplete();
        return;
      }
    }

    launcher.selectedIndex = 0;
    positionSelectedResult(ListView.Beginning);
    applyAutocomplete();
  }

  function selectPreviousSection() {
    const results = launcher.filteredResults();
    const count = results.length;

    if (count === 0) {
      return;
    }

    const currentIndex = Math.max(0, Math.min(launcher.selectedIndex, count - 1));
    const currentSection = results[currentIndex].section || "";
    let sectionStart = currentIndex;

    while (sectionStart > 0 && (results[sectionStart - 1].section || "") === currentSection) {
      sectionStart -= 1;
    }

    if (sectionStart > 0) {
      const previousSection = results[sectionStart - 1].section || "";
      let previousSectionStart = sectionStart - 1;

      while (previousSectionStart > 0 && (results[previousSectionStart - 1].section || "") === previousSection) {
        previousSectionStart -= 1;
      }

      launcher.selectedIndex = previousSectionStart;
      positionSelectedResult(ListView.Contain);
      applyAutocomplete();
      return;
    }

    const lastSection = results[count - 1].section || "";
    let lastSectionStart = count - 1;

    while (lastSectionStart > 0 && (results[lastSectionStart - 1].section || "") === lastSection) {
      lastSectionStart -= 1;
    }

    launcher.selectedIndex = lastSectionStart;
    positionSelectedResult(ListView.End);
    applyAutocomplete();
  }

  function handleAltNavigation(event) {
    if (!(event.modifiers & Qt.AltModifier)) {
      return false;
    }

    if (event.key === Qt.Key_H) {
      input.cursorPosition = Math.max(0, input.cursorPosition - 1);
      return true;
    }

    if (event.key === Qt.Key_L) {
      input.cursorPosition = Math.min(input.length, input.cursorPosition + 1);
      return true;
    }

    if (event.key === Qt.Key_J) {
      if (root.launcher.actionMode) {
        root.launcher.selectNextAction();
      } else {
        root.selectNextResult();
      }
      return true;
    }

    if (event.key === Qt.Key_K) {
      if (root.launcher.actionMode) {
        root.launcher.selectPreviousAction();
      } else {
        root.selectPreviousResult();
      }
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

  radius: 12
  color: launcher.surfaceColor
  border.color: launcher.outlineColor
  border.width: 1
  clip: true

  ColumnLayout {
    anchors.fill: parent
    spacing: 0

    // ── Search bar ──────────────────────────────────────────────────
    Rectangle {
      Layout.fillWidth: true
      Layout.preferredHeight: 42
      color: "transparent"

      RowLayout {
        anchors.fill: parent
        anchors.leftMargin: 14
        anchors.rightMargin: 12
        spacing: 8

        Item {
          Layout.preferredWidth: 15
          Layout.preferredHeight: 15
          Layout.alignment: Qt.AlignVCenter

          TintedIcon {
            anchors.centerIn: parent
            width: 15
            height: 15
            source: root.launcher.resolveIconSource("builtin:search")
            color: root.launcher.mutedTextColor
          }
        }

        TextField {
          id: input

          Layout.fillWidth: true
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
              } else if (event.key === Qt.Key_Backtab) {
                root.launcher.selectPreviousAction();
                event.accepted = true;
              } else if (event.key === Qt.Key_Tab) {
                if (event.modifiers & Qt.ShiftModifier) {
                  root.launcher.selectPreviousAction();
                } else {
                  root.launcher.selectNextAction();
                }
                event.accepted = true;
              } else if (event.key === Qt.Key_Return || event.key === Qt.Key_Enter) {
                root.launcher.activateSelectedAction();
                event.accepted = true;
              } else if (event.key >= Qt.Key_1 && event.key <= Qt.Key_9) {
                const actionIndex = event.key - Qt.Key_1;
                const actions = root.launcher.selectedActions();
                if (actionIndex < actions.length) {
                  root.launcher.selectedActionIndex = actionIndex;
                  root.launcher.activateSelectedAction();
                }
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
            } else if (event.key === Qt.Key_Down && event.modifiers & Qt.ControlModifier && count > 0) {
              root.selectNextSection();
              event.accepted = true;
            } else if (event.key === Qt.Key_Up && event.modifiers & Qt.ControlModifier && count > 0) {
              root.selectPreviousSection();
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
              } else if (root.applySelectedAutocomplete()) {
                // Keep the first selected suggestion usable without cycling away and back.
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

        Text {
          readonly property int _count: root.launcher.actionMode
            ? root.launcher.selectedActions().length
            : root.launcher.filteredResults().length
          readonly property int _index: root.launcher.actionMode
            ? root.launcher.selectedActionIndex
            : root.launcher.selectedIndex

          visible: root.launcher.footerStatus.length > 0 || _count > 0
          text: root.launcher.footerStatus.length > 0
            ? root.launcher.footerStatus
            : (_count > 0 ? `${_index + 1}/${_count}` : "")
          color: root.launcher.footerStatus.length > 0
            ? root.launcher.accentColor
            : root.launcher.mutedTextColor
          font.family: root.launcher.fontFamily
          font.pixelSize: root.launcher.tinyFontSize
          Layout.alignment: Qt.AlignVCenter
        }
      }
    }

    // ── Divider ─────────────────────────────────────────────────────
    Rectangle {
      Layout.fillWidth: true
      Layout.preferredHeight: 1
      color: root.launcher.outlineColor
    }

    // ── Results ──────────────────────────────────────────────────────
    ListView {
      id: results

      Layout.fillWidth: true
      Layout.fillHeight: true
      Layout.topMargin: 4
      Layout.bottomMargin: 4
      clip: true
      spacing: 0
      boundsBehavior: Flickable.StopAtBounds
      currentIndex: root.launcher.displayIndexForResult(root.launcher.selectedIndex)
      highlightMoveDuration: 0
      model: root.launcher.displayRows()

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

      delegate: Loader {
        id: resultDelegate

        required property int index
        required property var modelData

        width: ListView.view.width
        height: modelData.type === "section" ? 26
              : modelData.type === "action" ? 36
              : 48
        sourceComponent: modelData.type === "section" ? sectionComponent
                       : modelData.type === "action" ? actionComponent
                       : resultComponent

        Component {
          id: sectionComponent

          LauncherSectionHeader {
            width: resultDelegate.width
            text: resultDelegate.modelData.section
            launcher: root.launcher
          }
        }

        Component {
          id: resultComponent

          LauncherResultRow {
            width: resultDelegate.width
            rowIndex: resultDelegate.modelData.resultIndex
            result: resultDelegate.modelData.result
            launcher: root.launcher
          }
        }

        Component {
          id: actionComponent

          Rectangle {
            readonly property bool isSelected: resultDelegate.modelData.actionIndex === root.launcher.selectedActionIndex

            width: resultDelegate.width
            height: 36
            color: isSelected ? root.launcher.hoverColor : "transparent"
            radius: 0
            clip: true

            RowLayout {
              anchors.fill: parent
              anchors.leftMargin: 52
              anchors.rightMargin: 10
              spacing: 8
              opacity: isSelected ? 1 : 0.7

              Loader {
                id: actionInlineIconLoader

                property string resolvedSource: root.launcher.resolveIconSource(resultDelegate.modelData.action.icon || "")
                property color resolvedColor: isSelected ? root.launcher.primaryColor : root.launcher.mutedTextColor

                Layout.preferredWidth: 16
                Layout.preferredHeight: 16
                Layout.alignment: Qt.AlignVCenter
                active: (resultDelegate.modelData.action.icon || "").length > 0

                sourceComponent: TintedIcon {
                  width: 16
                  height: 16
                  source: actionInlineIconLoader.resolvedSource
                  color: actionInlineIconLoader.resolvedColor
                }
              }

              Text {
                Layout.fillWidth: true
                Layout.alignment: Qt.AlignVCenter
                text: resultDelegate.modelData.action.label
                color: isSelected ? root.launcher.textColor : root.launcher.mutedTextColor
                font.family: root.launcher.fontFamily
                font.pixelSize: root.launcher.smallFontSize
                elide: Text.ElideRight
                maximumLineCount: 1
              }

              Text {
                readonly property int _idx: resultDelegate.modelData.actionIndex
                Layout.alignment: Qt.AlignVCenter
                text: _idx < 9 ? String(_idx + 1) : ""
                color: root.launcher.mutedTextColor
                font.family: root.launcher.fontFamily
                font.pixelSize: root.launcher.tinyFontSize
                opacity: 0.5
                visible: text.length > 0
              }
            }

            MouseArea {
              anchors.fill: parent
              hoverEnabled: true
              onEntered: root.launcher.selectedActionIndex = resultDelegate.modelData.actionIndex
              onClicked: root.launcher.activateSelectedAction()
            }
          }
        }
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

    Connections {
      target: root.launcher

      function onActionModeChanged() {
        Qt.callLater(function() {
          if (root.launcher.actionMode) {
            const di = root.launcher.displayIndexForResult(root.launcher.selectedIndex);
            const actionCount = root.launcher.selectedActions().length;
            if (di >= 0 && actionCount > 0) {
              results.positionViewAtIndex(di + actionCount, ListView.Contain);
            }
          } else {
            root.positionSelectedResult(ListView.Contain);
          }
        });
      }
    }

    // ── Divider ─────────────────────────────────────────────────────
    Rectangle {
      Layout.fillWidth: true
      Layout.preferredHeight: 1
      color: root.launcher.outlineColor
    }

    // ── Footer ──────────────────────────────────────────────────────
    Rectangle {
      Layout.fillWidth: true
      Layout.preferredHeight: 28
      color: "transparent"

      RowLayout {
        anchors.fill: parent
        anchors.leftMargin: 14
        anchors.rightMargin: 12
        anchors.topMargin: 4
        anchors.bottomMargin: 4
        spacing: 4

        Text {
          text: root.launcher.primaryActionLabel()
          color: root.launcher.textColor
          font.family: root.launcher.fontFamily
          font.pixelSize: root.launcher.smallFontSize
          font.weight: Font.Medium
        }

        Text {
          text: "↵"
          color: root.launcher.mutedTextColor
          font.family: root.launcher.fontFamily
          font.pixelSize: root.launcher.smallFontSize
        }

        Item {
          Layout.fillWidth: true
        }

        Text {
          text: "Back"
          color: root.launcher.textColor
          font.family: root.launcher.fontFamily
          font.pixelSize: root.launcher.smallFontSize
          visible: root.launcher.actionMode
        }

        Text {
          text: "⎋"
          color: root.launcher.mutedTextColor
          font.family: root.launcher.fontFamily
          font.pixelSize: root.launcher.smallFontSize
          visible: root.launcher.actionMode
        }

        Text {
          text: "Actions"
          color: root.launcher.textColor
          font.family: root.launcher.fontFamily
          font.pixelSize: root.launcher.smallFontSize
          visible: !root.launcher.actionMode
        }

        Text {
          text: "⌃K"
          color: root.launcher.mutedTextColor
          font.family: root.launcher.fontFamily
          font.pixelSize: root.launcher.smallFontSize
          visible: !root.launcher.actionMode
        }

        Item {
          Layout.preferredWidth: 6
          visible: !root.launcher.actionMode
        }

        Text {
          text: "Refresh"
          color: root.launcher.textColor
          font.family: root.launcher.fontFamily
          font.pixelSize: root.launcher.smallFontSize
          visible: !root.launcher.actionMode
        }

        Text {
          text: "⌃R"
          color: root.launcher.mutedTextColor
          font.family: root.launcher.fontFamily
          font.pixelSize: root.launcher.smallFontSize
          visible: !root.launcher.actionMode
        }
      }
    }
  }
}
