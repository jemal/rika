pragma ComponentBehavior: Bound

import QtQuick
import Quickshell.Io

Item {
  id: root

  required property string socketPath
  property var pendingMessages: []

  signal resultsReceived(int requestId, var items)
  signal activated(string provider, string id, string action)
  signal errorReceived(string message)

  function sendQuery(requestId, query) {
    send({
      "type": "query",
      "request_id": requestId,
      "query": query
    });
  }

  function activate(provider, id, action) {
    send({
      "type": "activate",
      "provider": provider,
      "id": id,
      "action": action
    });
  }

  function send(message) {
    pendingMessages.push(`${JSON.stringify(message)}\n`);

    if (socket.connected) {
      flushPending();
    } else {
      socket.connected = true;
    }
  }

  function flushPending() {
    while (socket.connected && pendingMessages.length > 0) {
      socket.write(pendingMessages.shift());
    }

    socket.flush();
  }

  function handleLine(line) {
    if (line.trim().length === 0) {
      return;
    }

    let response;
    try {
      response = JSON.parse(line);
    } catch (error) {
      errorReceived(`failed to parse daemon response: ${error}`);
      return;
    }

    if (response.type === "results") {
      resultsReceived(response.request_id, response.items);
    } else if (response.type === "activated") {
      activated(response.provider, response.id, response.action);
    }
  }

  Socket {
    id: socket

    path: root.socketPath
    parser: SplitParser {
      splitMarker: "\n"
      onRead: data => root.handleLine(data)
    }

    onConnectionStateChanged: {
      if (connected) {
        root.flushPending();
      }
    }

    onError: {
      root.pendingMessages = [];
      root.errorReceived("failed to connect to daemon socket");
    }
  }
}
