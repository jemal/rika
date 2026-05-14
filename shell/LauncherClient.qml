pragma ComponentBehavior: Bound

import QtQuick
import Quickshell.Io

Item {
  id: root

  required property string socketPath
  property var pendingMessages: []

  signal resultsReceived(int requestId, var items)
  signal activated(string provider, string id, string action)
  signal refreshed(int requestId, var config, var errors)
  signal configReceived(var config)
  signal errorReceived(string message)

  function sendQuery(requestId, query) {
    send({
      "type": "query",
      "request_id": requestId,
      "query": query
    });
  }

  function getConfig() {
    send({
      "type": "config"
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

  function sendRefresh(requestId) {
    send({
      "type": "refresh",
      "request_id": requestId
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
    } else if (response.type === "refreshed") {
      refreshed(response.request_id, response.config, response.errors || []);
    } else if (response.type === "config") {
      configReceived(response.config);
    } else if (response.type === "error") {
      errorReceived(response.message);
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
