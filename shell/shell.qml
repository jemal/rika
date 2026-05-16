pragma ComponentBehavior: Bound

import QtQuick
import Quickshell
import Quickshell.Io
import Quickshell.Widgets
import Quickshell.Wayland

PanelWindow {
  id: launcher

  property bool open: false
  property bool shown: false
  property int selectedIndex: 0
  property string query: ""
  property var results: []
  property var iconWarmResults: []
  property int requestId: 0
  property bool primingInitialResults: false
  property string ipcError: ""
  property string footerStatus: ""
  property int maxVisibleResults: 7
  property string fontFamily: ""
  property int fontSize: 14
  property int smallFontSize: 13
  property int tinyFontSize: 10
  property string windowAnchor: "top"
  property int windowWidth: 580
  property int windowHeight: 316
  property int windowMargin: 320
  readonly property int visibleResultCount: Math.min(filteredResults().length, maxVisibleResults)
  readonly property int resultAreaHeight: filteredResults().length === 0 ? 88 : visibleResultCount * 34
  readonly property int panelWidth: Math.max(240, Math.min(width - 48, windowWidth))
  readonly property int panelHeight: Math.max(140, Math.min(height - 96, windowHeight))
  readonly property int panelX: Math.round((width - panelWidth) / 2)
  readonly property int panelY: {
    if (windowAnchor === "center") {
      return Math.round((height - panelHeight) / 2);
    }

    return Math.max(24, Math.min(height - panelHeight - 24, windowMargin));
  }

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

  function primeInitialResults() {
    primingInitialResults = true;
    sendQuery(query);
  }

  function fetchInitialConfig() {
    ipc.getConfig();
  }

  function resolveIconSource(icon) {
    if (!icon || icon.length === 0) {
      return "";
    }

    if (icon.startsWith("builtin:")) {
      const iconName = icon.slice("builtin:".length);
      const dataDir = Quickshell.env("RIKA_DATA_DIR");

      if (dataDir && dataDir.length > 0) {
        return `file://${dataDir}/resources/icons/${iconName}.svg`;
      }

      return Qt.resolvedUrl(`../resources/icons/${iconName}.svg`);
    }

    if (icon.startsWith("/")) {
      return `file://${icon}`;
    }

    const path = Quickshell.iconPath(icon, "application-x-executable");
    return path && path.length > 0 ? path : "";
  }

  function refreshProviders() {
    requestId += 1;
    ipcError = "";
    footerStatus = "Refreshing...";
    ipc.sendRefresh(requestId);
  }

  function openLauncher() {
    shown = true;
    open = true;
  }

  function closeLauncher() {
    open = false;
    selectedIndex = 0;
    query = "";
    results = [];
    ipcError = "";
    footerStatus = "";
  }

  function toggleLauncher() {
    open = !open;
  }

  function filteredResults() {
    return results;
  }

  function clampSelection() {
    const count = filteredResults().length;
    selectedIndex = count === 0 ? 0 : Math.max(0, Math.min(selectedIndex, count - 1));
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

  visible: shown
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
    } else {
      fetchInitialConfig();
      primeInitialResults();
    }
  }

  onOpenChanged: {
    if (open) {
      shown = true;
      primingInitialResults = false;
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
      launcher.iconWarmResults = items;

      if (responseRequestId !== launcher.requestId) {
        return;
      }

      launcher.results = items;
      launcher.selectedIndex = 0;
      launcher.primingInitialResults = false;
      launcher.ipcError = "";
    }

    onErrorReceived: message => {
      if (launcher.primingInitialResults && !launcher.open) {
        launcher.primingInitialResults = false;
        return;
      }

      launcher.results = [];
      launcher.ipcError = message;
      launcher.footerStatus = "";
      launcher.openLauncher();
    }

    onRefreshed: (responseRequestId, config, errors) => {
      if (responseRequestId !== launcher.requestId) {
        return;
      }

      applyConfig(config);

      launcher.primingInitialResults = false;
      launcher.footerStatus = errors.length > 0 ? errors.join("; ") : "Refreshed";
      footerStatusTimer.restart();
      launcher.sendQuery(launcher.query);
    }

    onConfigReceived: config => {
      applyConfig(config);
    }

    function applyConfig(config) {
      const launcherConfig = config?.launcher;
      const maxVisibleResults = launcherConfig?.max_visible_results;
      const fontFamily = launcherConfig?.font_family;
      const fontSize = launcherConfig?.font_size;
      const smallFontSize = launcherConfig?.small_font_size;
      const tinyFontSize = launcherConfig?.tiny_font_size;
      const windowConfig = launcherConfig?.window;
      const windowAnchor = windowConfig?.anchor;
      const windowWidth = windowConfig?.width;
      const windowHeight = windowConfig?.height;
      const windowMargin = windowConfig?.margin;

      if (typeof maxVisibleResults === "number" && maxVisibleResults > 0) {
        launcher.maxVisibleResults = maxVisibleResults;
      }

      if (typeof fontFamily === "string") {
        launcher.fontFamily = fontFamily;
      }

      if (typeof fontSize === "number" && fontSize > 0) {
        launcher.fontSize = fontSize;
      }

      if (typeof smallFontSize === "number" && smallFontSize > 0) {
        launcher.smallFontSize = smallFontSize;
      }

      if (typeof tinyFontSize === "number" && tinyFontSize > 0) {
        launcher.tinyFontSize = tinyFontSize;
      }

      if (windowAnchor === "top" || windowAnchor === "center") {
        launcher.windowAnchor = windowAnchor;
      }

      if (typeof windowWidth === "number" && windowWidth > 0) {
        launcher.windowWidth = windowWidth;
      }

      if (typeof windowHeight === "number" && windowHeight > 0) {
        launcher.windowHeight = windowHeight;
      }

      if (typeof windowMargin === "number" && windowMargin >= 0) {
        launcher.windowMargin = windowMargin;
      }
    }
  }

  Timer {
    id: footerStatusTimer

    interval: 1000
    repeat: false
    onTriggered: launcher.footerStatus = ""
  }

  Item {
    x: -100
    y: -100
    width: 18
    height: 18
    opacity: 0

    Repeater {
      model: launcher.iconWarmResults.slice(0, launcher.maxVisibleResults)

      delegate: Loader {
        id: iconWarmLoader

        required property var modelData

        width: 18
        height: 18
        active: modelData.icon.length > 0 && !modelData.icon.startsWith("builtin:")

        sourceComponent: IconImage {
          source: launcher.resolveIconSource(iconWarmLoader.modelData.icon)
          asynchronous: true
        }
      }
    }
  }

  Rectangle {
    id: dimmer

    anchors.fill: parent
    color: launcher.dimColor
    opacity: launcher.open ? 1 : 0

    Behavior on opacity {
      NumberAnimation {
        duration: 110
        easing.type: Easing.OutCubic
      }
    }

    MouseArea {
      anchors.fill: parent
      onClicked: launcher.closeLauncher()
    }
  }

  LauncherPanel {
    id: panel

    launcher: launcher
    x: launcher.panelX
    y: launcher.panelY + (launcher.open ? 0 : -8)
    width: launcher.panelWidth
    height: launcher.panelHeight
    opacity: launcher.open ? 1 : 0
    scale: launcher.open ? 1 : 0.985

    Behavior on opacity {
      NumberAnimation {
        duration: 120
        easing.type: Easing.OutCubic
      }
    }

    Behavior on scale {
      NumberAnimation {
        duration: 120
        easing.type: Easing.OutCubic
      }
    }

    Behavior on y {
      NumberAnimation {
        duration: 120
        easing.type: Easing.OutCubic
      }
    }

    Timer {
      interval: 130
      running: !launcher.open && launcher.shown
      repeat: false
      onTriggered: launcher.shown = false
    }
  }
}
