pragma ComponentBehavior: Bound

import QtQuick
import Quickshell
import Quickshell.Wayland

PanelWindow {
  id: launcher

  property bool open: true
  property int selectedIndex: 0
  property string query: ""

  readonly property color dimColor: Qt.alpha("#0c0f14", 0.46)
  readonly property color surfaceColor: "#111318"
  readonly property color surfaceVariantColor: "#191d24"
  readonly property color hoverColor: "#273247"
  readonly property color outlineColor: "#343a46"
  readonly property color primaryColor: "#7aa7ff"
  readonly property color textColor: "#f4f7fb"
  readonly property color mutedTextColor: "#9aa3b2"

  property var allResults: [
    {
      "id": "project:/home/jemal/dev/personal/rika",
      "provider": "projects",
      "title": "Rika",
      "subtitle": "~/dev/personal/rika",
      "kind": "project",
      "icon": "R",
      "score": 1.0,
      "actions": ["open", "open-terminal"]
    },
    {
      "id": "app:org.wezfurlong.wezterm",
      "provider": "apps",
      "title": "WezTerm",
      "subtitle": "GPU-accelerated terminal emulator",
      "kind": "app",
      "icon": "W",
      "score": 0.92,
      "actions": ["launch"]
    },
    {
      "id": "bang:github:quickshell",
      "provider": "bangs",
      "title": "Search GitHub",
      "subtitle": "!gh quickshell",
      "kind": "web",
      "icon": "G",
      "score": 0.84,
      "actions": ["open"]
    },
    {
      "id": "command:open-notes",
      "provider": "commands",
      "title": "Open notes",
      "subtitle": "ghostty -e nvim ~/documents/notes",
      "kind": "command",
      "icon": ">",
      "score": 0.78,
      "actions": ["run"]
    }
  ]

  function matches(result) {
    const needle = query.trim().toLowerCase();

    if (needle.length === 0) {
      return true;
    }

    return result.title.toLowerCase().includes(needle)
      || result.subtitle.toLowerCase().includes(needle)
      || result.provider.toLowerCase().includes(needle)
      || result.kind.toLowerCase().includes(needle);
  }

  function filteredResults() {
    return allResults.filter(matches);
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
    console.log(`activate provider=${result.provider} id=${result.id} action=${result.actions[0]}`);
    open = false;
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

  Component.onCompleted: panel.focusSearchInput()

  onOpenChanged: {
    if (open) {
      panel.focusSearchInput();
    }
  }

  Rectangle {
    anchors.fill: parent
    color: launcher.dimColor

    MouseArea {
      anchors.fill: parent
      onClicked: launcher.open = false
    }
  }

  LauncherPanel {
    id: panel

    launcher: launcher
    width: Math.min(parent.width - 48, 760)
    height: Math.min(parent.height - 96, 468)
    anchors.horizontalCenter: parent.horizontalCenter
    anchors.top: parent.top
    anchors.topMargin: Math.max(56, Math.round(parent.height * 0.16))
  }
}
