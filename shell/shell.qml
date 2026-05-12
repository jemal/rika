pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
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

  Component.onCompleted: input.forceActiveFocus()

  onOpenChanged: {
    if (open) {
      input.forceActiveFocus();
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

  Rectangle {
    id: panel

    width: Math.min(parent.width - 48, 760)
    height: Math.min(parent.height - 96, 468)
    anchors.horizontalCenter: parent.horizontalCenter
    anchors.top: parent.top
    anchors.topMargin: Math.max(56, Math.round(parent.height * 0.16))
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
        color: launcher.surfaceVariantColor
        border.color: input.activeFocus ? launcher.primaryColor : launcher.outlineColor
        border.width: 1

        TextField {
          id: input

          anchors.fill: parent
          anchors.leftMargin: 14
          anchors.rightMargin: 14
          focus: true
          color: launcher.textColor
          selectedTextColor: "#ffffff"
          selectionColor: "#355c9d"
          placeholderText: "Search apps, projects, commands, and web bangs"
          placeholderTextColor: launcher.mutedTextColor
          verticalAlignment: TextInput.AlignVCenter
          font.pixelSize: 21
          background: null

          onTextChanged: {
            launcher.query = text;
            launcher.selectedIndex = 0;
          }

          Keys.onPressed: event => {
            const count = launcher.filteredResults().length;

            if (event.key === Qt.Key_Escape) {
              launcher.open = false;
              event.accepted = true;
            } else if (event.key === Qt.Key_Down && count > 0) {
              launcher.selectedIndex = Math.min(launcher.selectedIndex + 1, count - 1);
              event.accepted = true;
            } else if (event.key === Qt.Key_Up && count > 0) {
              launcher.selectedIndex = Math.max(launcher.selectedIndex - 1, 0);
              event.accepted = true;
            } else if (event.key === Qt.Key_Return || event.key === Qt.Key_Enter) {
              launcher.activateSelection();
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
        model: launcher.filteredResults()

        onCountChanged: launcher.clampSelection()

        delegate: Rectangle {
          id: row

          required property int index
          required property var modelData

          width: ListView.view.width
          height: 68
          radius: 10
          color: index === launcher.selectedIndex ? launcher.hoverColor : launcher.surfaceVariantColor
          border.color: index === launcher.selectedIndex ? launcher.primaryColor : "transparent"
          border.width: 1
          clip: true

          Behavior on color {
            ColorAnimation {
              duration: 120
              easing.type: Easing.OutCubic
            }
          }

          RowLayout {
            anchors.fill: parent
            anchors.leftMargin: 12
            anchors.rightMargin: 12
            spacing: 12

            Rectangle {
              Layout.preferredWidth: 42
              Layout.preferredHeight: 42
              Layout.alignment: Qt.AlignVCenter
              radius: 10
              color: row.index === launcher.selectedIndex ? "#365074" : launcher.surfaceColor

              Text {
                anchors.centerIn: parent
                text: row.modelData.icon
                color: "#dce7f8"
                font.pixelSize: 16
                font.bold: true
              }
            }

            ColumnLayout {
              Layout.fillWidth: true
              Layout.alignment: Qt.AlignVCenter
              spacing: 2

              Text {
                Layout.fillWidth: true
                text: row.modelData.title
                color: launcher.textColor
                font.pixelSize: 17
                font.bold: true
                elide: Text.ElideRight
                maximumLineCount: 1
              }

              Text {
                Layout.fillWidth: true
                text: row.modelData.subtitle
                color: launcher.mutedTextColor
                font.pixelSize: 13
                elide: Text.ElideRight
                maximumLineCount: 1
              }
            }

            Text {
              Layout.preferredWidth: 92
              Layout.alignment: Qt.AlignRight | Qt.AlignVCenter
              horizontalAlignment: Text.AlignRight
              text: row.modelData.kind
              color: "#a9bddc"
              font.pixelSize: 12
              elide: Text.ElideRight
            }
          }

          MouseArea {
            anchors.fill: parent
            hoverEnabled: true

            onEntered: launcher.selectedIndex = row.index
            onClicked: launcher.activateSelection()
          }
        }

        Text {
          anchors.centerIn: parent
          text: "No results"
          color: launcher.mutedTextColor
          font.pixelSize: 15
          visible: results.count === 0
        }
      }
    }
  }
}
