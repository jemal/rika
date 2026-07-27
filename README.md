# rika

A keyboard-driven application launcher, built for my own [niri](https://github.com/YaLTeR/niri) desktop. Rust daemon for search/scoring/activation, [Quickshell](https://quickshell.org/)/QML for the UI.

This was built for personal, daily-driver use — not a general-purpose product. There's no installer beyond Nix, no support, and some defaults (terminal, editor, project roots) assume my machine. Sharing it mainly as a code sample; feel free to fork or borrow from it.

## What it does

- **Apps** — desktop entries, respecting `Hidden`/`NoDisplay` and `Terminal=true`
- **Calculator** — inline expression evaluation via `evalexpr`
- **Projects** — fuzzy search over configured project roots, with per-root aliases and custom open actions (terminal, `zellij`, etc.)
- **Files** / **file search** — direct paths and indexed search under configured roots
- **Commands** — user-defined shell commands
- **Web search** — [Kagi-style bang](https://kagi.com/bangs) prefix search (`!gh`, `!yt`, ...), bundled at build time
- Usage-ranked results, favorites, and a recents section
- Light/dark theming that can follow the desktop color scheme (`gsettings`), with themes as plain TOML files (ships with `kanagawa`; `resources/themes/` has more to copy in — `onedark`/`onelight`, `gruvbox_dark_hard`/`gruvbox_light`, `kanagawa_lotus`, `lua`/`sol`)

## Architecture

- The Rust daemon (`src/`) owns providers, search/scoring, activation, config loading, and refresh — all the logic.
- The QML shell (`shell/`) is intentionally thin: renders results, handles focus/selection, and talks to the daemon over IPC. It doesn't know provider-specific semantics.
- IPC is newline-delimited JSON over a Unix socket at `$RIKA_LAUNCHER_SOCKET` (default `$XDG_RUNTIME_DIR/rika-launcher.sock`).

See [AGENTS.md](AGENTS.md) for more implementation notes.

## Running it

With Nix (recommended):

```sh
nix develop
cargo run                                    # start the daemon
quickshell --path shell --no-duplicate       # start the UI
quickshell ipc --path shell call launcher toggle
```

Or build the packages directly:

```sh
nix build .#rika          # daemon only
nix build .#rika-shell    # wrapped Quickshell UI
nix build .#              # both
```

## Configuration

Config lives at `$XDG_CONFIG_HOME/rika/config.json` (falls back to built-in defaults if absent — see [`resources/config.json`](resources/config.json) for the shape). Custom themes go in `$XDG_CONFIG_HOME/rika/themes/<name>.toml`, each a flat set of colors (no light/dark split — a theme file is one palette).

`launcher.theme` picks the theme(s) by name, checking `$XDG_CONFIG_HOME/rika/themes/<name>.toml` before falling back to what's built in (only `kanagawa`). It takes either:

- a single name, used for both light and dark contexts: `"theme": "kanagawa"`
- a `{dark, light}` pair, to follow the desktop color scheme between two different palettes: `"theme": {"dark": "lua", "light": "sol"}`
