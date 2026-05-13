## Project Notes

- Use `jj` for version control in this repository.
- Keep changes small and aligned with the existing Rust/QML split.

## Architecture

- The Rust daemon owns providers, search, scoring, activation, refresh, config loading, and filesystem/process work.
- The Quickshell UI should stay thin: render results, handle focus/selection, and send IPC requests.
- QML should not parse provider IDs or know provider-specific activation semantics.
- Result identity is the `(provider, id)` pair. Provider-local IDs should not include a provider prefix.

## IPC

- IPC is newline-delimited JSON over the Unix socket at `$RIKA_LAUNCHER_SOCKET` or `$XDG_RUNTIME_DIR/rika-launcher.sock`.
- Client requests currently include `query`, `activate`, and `refresh`.
- Server responses currently include `results`, `activated`, `refreshed`, and `error`.

## Nix

- Package outputs:
  - `.#rika`: Rust daemon.
  - `.#rika-shell`: wrapped Quickshell UI.
  - `.#default`: combined package exposing both `rika` and `rika-shell`.
- `rika-shell` sets `QS_CONFIG_PATH` for the packaged QML shell.

## Development

- Typical dev loop:
  - `cargo run`
  - `quickshell --path shell --no-duplicate`
  - `quickshell ipc --path shell call launcher toggle`
- If already inside `nix develop`, run checks directly.
- From outside the dev shell, use `nix develop --command ...`.

## Checks

- Rust:
  - `cargo check`
- QML:
  - `qmllint -E shell/shell.qml shell/LauncherPanel.qml shell/LauncherResultRow.qml shell/LauncherClient.qml`
- Nix:
  - `nix flake check --no-build`
  - `nix build .#`
