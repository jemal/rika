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
  property bool actionMode: false
  property int selectedIndex: 0
  property int selectedActionIndex: 0
  property string query: ""
  property var results: []
  property var iconWarmResults: []
  property var pendingActions: ({})
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
    actionMode = false;
    selectedIndex = 0;
    selectedActionIndex = 0;
    query = "";
    results = [];
    pendingActions = ({});
    ipcError = "";
    footerStatus = "";
    panel.resetInput();
  }

  function toggleLauncher() {
    open = !open;
  }

  function filteredResults() {
    return results;
  }

  function selectedResult() {
    const results = filteredResults();
    return selectedIndex >= 0 && selectedIndex < results.length ? results[selectedIndex] : null;
  }

  function shouldShowSectionHeader(index) {
    const results = filteredResults();
    const result = index >= 0 && index < results.length ? results[index] : null;

    if (!result || !result.section || result.section.length === 0) {
      return false;
    }

    if (query.trim().length > 0 && !hasMultipleSections()) {
      return false;
    }

    if (index === 0) {
      return true;
    }

    const previous = results[index - 1];
    return !previous || previous.section !== result.section;
  }

  function hasMultipleSections() {
    const results = filteredResults();
    let firstSection = "";

    for (let i = 0; i < results.length; i += 1) {
      const section = results[i].section || "";
      if (section.length === 0) {
        continue;
      }

      if (firstSection.length === 0) {
        firstSection = section;
      } else if (section !== firstSection) {
        return true;
      }
    }

    return false;
  }

  function selectedActions() {
    const result = selectedResult();
    return result && result.actions ? result.actions : [];
  }

  function selectedAction() {
    const actions = selectedActions();
    return selectedActionIndex >= 0 && selectedActionIndex < actions.length ? actions[selectedActionIndex] : null;
  }

  function clampSelection() {
    const count = filteredResults().length;
    selectedIndex = count === 0 ? 0 : Math.max(0, Math.min(selectedIndex, count - 1));
    clampActionSelection();
  }

  function clampActionSelection() {
    const count = selectedActions().length;
    selectedActionIndex = count === 0 ? 0 : Math.max(0, Math.min(selectedActionIndex, count - 1));
  }

  function enterActionMode() {
    const actions = selectedActions();

    if (actions.length === 0) {
      return;
    }

    const result = selectedResult();
    let defaultIndex = -1;
    for (let i = 0; i < actions.length; i += 1) {
      if (actions[i].id === result.default_action) {
        defaultIndex = i;
        break;
      }
    }
    selectedActionIndex = defaultIndex >= 0 ? defaultIndex : 0;
    actionMode = true;
  }

  function exitActionMode() {
    actionMode = false;
    selectedActionIndex = 0;
  }

  function selectNextAction() {
    const count = selectedActions().length;

    if (count === 0) {
      return;
    }

    selectedActionIndex = (selectedActionIndex + 1) % count;
  }

  function selectPreviousAction() {
    const count = selectedActions().length;

    if (count === 0) {
      return;
    }

    selectedActionIndex = selectedActionIndex === 0 ? count - 1 : selectedActionIndex - 1;
  }

  function activateResultAction(result, action) {
    if (!result || !action || !action.id || action.id.length === 0) {
      return;
    }

    if (action.id === "noop") {
      closeLauncher();
      return;
    }

    ipc.activate(result.provider, result.id, action.id);
    pendingActions[`${result.provider}:${result.id}:${action.id}`] = {
      "close_behavior": action.close_behavior || "confirmed",
      "success_message": action.success_message || action.label || action.id
    };

    if (action.close_behavior === "immediate") {
      closeLauncher();
    }
  }

  function activateSelectedAction() {
    const action = selectedAction();

    if (!action) {
      return;
    }

    activateResultAction(selectedResult(), action);
  }

  function activateSelection() {
    if (actionMode) {
      activateSelectedAction();
      return;
    }

    const result = selectedResult();

    if (!result) {
      return;
    }

    const actions = result.actions || [];
    const defaultAction = actions.find(action => action.id === result.default_action);
    activateResultAction(result, defaultAction || {
      "id": result.default_action,
      "close_behavior": "confirmed"
    });
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
      launcher.actionMode = false;
      launcher.selectedActionIndex = 0;
      launcher.primingInitialResults = false;
      launcher.ipcError = "";
    }

    onActivated: (provider, id, action) => {
      const key = `${provider}:${id}:${action}`;
      const pendingAction = launcher.pendingActions[key] || {};
      delete launcher.pendingActions[key];

      if (pendingAction.close_behavior === "keep_open") {
        launcher.footerStatus = pendingAction.success_message;
        footerStatusTimer.restart();
        launcher.exitActionMode();
        launcher.sendQuery(launcher.query);
        return;
      }

      launcher.closeLauncher();
    }

    onErrorReceived: message => {
      if (launcher.primingInitialResults && !launcher.open) {
        launcher.primingInitialResults = false;
        return;
      }

      launcher.results = [];
      launcher.ipcError = message;
      launcher.footerStatus = "";

      if (!launcher.open) {
        launcher.openLauncher();
      }
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
