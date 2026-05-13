pragma ComponentBehavior: Bound

import QtQuick
import Quickshell
import Quickshell.Io
import Quickshell.Wayland

PanelWindow {
  id: launcher

  property bool open: false
  property int selectedIndex: 0
  property string query: ""
  property var results: []
  property int requestId: 0
  property string ipcError: ""
  readonly property int visibleResultCount: Math.min(filteredResults().length, 7)

  readonly property color dimColor: Qt.alpha("#0d0c0c", 0.22)
  readonly property color surfaceColor: "#181820"
  readonly property color surfaceVariantColor: "#181820"
  readonly property color hoverColor: "#2d2b3a"
  readonly property color outlineColor: Qt.alpha("#54546d", 0.46)
  readonly property color primaryColor: "#7e9cd8"
  readonly property color accentColor: "#98bb6c"
  readonly property color warningColor: "#e6c384"
  readonly property color errorColor: "#e46876"
  readonly property color textColor: "#dcd7ba"
  readonly property color mutedTextColor: "#727169"

  function sendQuery(text) {
    requestId += 1;
    ipcError = "";
    ipc.sendQuery(requestId, text);
  }

  function openLauncher() {
    open = true;
  }

  function closeLauncher() {
    open = false;
  }

  function toggleLauncher() {
    open = !open;
  }

  function filteredResults() {
    return results;
  }

  function clampSelection() {
    const count = filteredResults().length;
    selectedIndex = count === 0 ? 0 : Math.min(selectedIndex, count - 1);
  }

  function activateSelection() {
    const results = filteredResults();

    if (results.length === 0) {
      return;
    }

    const result = results[selectedIndex];
    const action = result.actions.length > 0 ? result.actions[0] : "open";
    ipc.activate(result.provider, result.id, action);
    closeLauncher();
  }

  visible: open
  color: "transparent"

  anchors {
    top: true
    bottom: true
    left: true
    right: true
  }

  WlrLayershell.namespace: "rika-launcher"
  WlrLayershell.keyboardFocus: WlrKeyboardFocus.Exclusive
  WlrLayershell.layer: WlrLayer.Overlay
  WlrLayershell.exclusionMode: ExclusionMode.Ignore

  Component.onCompleted: {
    if (open) {
      panel.focusSearchInput();
      sendQuery(query);
    }
  }

  onOpenChanged: {
    if (open) {
      Qt.callLater(panel.focusSearchInput);
      sendQuery(query);
    }
  }

  IpcHandler {
    target: "launcher"

    function open() {
      launcher.openLauncher();
    }

    function close() {
      launcher.closeLauncher();
    }

    function toggle() {
      launcher.toggleLauncher();
    }
  }

  LauncherClient {
    id: ipc

    socketPath: `${Quickshell.env("XDG_RUNTIME_DIR")}/rika-launcher.sock`

    onResultsReceived: (responseRequestId, items) => {
      if (responseRequestId !== launcher.requestId) {
        return;
      }

      launcher.results = items;
      launcher.selectedIndex = 0;
      launcher.ipcError = "";
    }

    onErrorReceived: message => {
      launcher.results = [];
      launcher.ipcError = message;
    }
  }

  Rectangle {
    anchors.fill: parent
    color: launcher.dimColor

    MouseArea {
      anchors.fill: parent
      onClicked: launcher.closeLauncher()
    }
  }

  LauncherPanel {
    id: panel

    launcher: launcher
    width: Math.min(parent.width - 48, 580)
    height: Math.min(parent.height - 96, 54 + Math.max(88, launcher.visibleResultCount * 34))
    anchors.horizontalCenter: parent.horizontalCenter
    anchors.top: parent.top
    anchors.topMargin: Math.max(48, Math.round(parent.height * 0.32))
  }
}
