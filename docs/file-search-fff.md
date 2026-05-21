# File Search With FFF

## Summary

`fff` is a promising backend candidate for Rika file search. The local clone at
`~/dev/thirdparty/fff` includes a Rust library crate, `fff-search`, under
`crates/fff-core`. It is not only a Neovim plugin backend; the project now
describes itself as a file-search toolkit for long-running applications.

The fit is good enough to spike before building a custom file indexer.

## What Looks Useful

- Rust library API via `fff-search`.
- Fuzzy and typo-resistant path search.
- Frecency and query-history support.
- Git status integration.
- Background scanning and filesystem watching.
- Optional grep/content indexing.
- C FFI is available, but Rika should prefer the Rust library API if it is clean
  enough.

The main API appears to be `FilePicker`:

```rust
use fff_search::file_picker::FilePicker;
use fff_search::{
    FFFMode, FilePickerOptions, FuzzySearchOptions, PaginationArgs, QueryParser,
};
```

For Rika, the simplest path is to start with file-name/path search only and defer
the heavier features until the provider lifecycle needs them.

## Recommended Spike

Add a temporary Rust example in Rika that depends on the local clone:

```toml
fff-search = { path = "/home/jemal/dev/thirdparty/fff/crates/fff-core" }
```

The example should:

- Create a `FilePicker` for `~/dev/projects/rika`.
- Run `collect_files()` synchronously.
- Parse a few queries with `QueryParser`.
- Call `fuzzy_search(...)`.
- Print relative path, score, and total match count.
- Measure rough indexing and query latency.
- Remove the example after the spike.

Test queries should include:

- `launcher`
- `web search`
- typo-heavy queries
- nested paths such as `providers web`
- empty or very short queries

The spike should answer:

- Is the Rust API ergonomic enough for a Rika provider?
- Does it work well outside a Neovim/project-only context?
- Does it return the path and score data Rika needs?
- How expensive is indexing for a normal root?
- How large is the dependency/build impact?
- Does it require LMDB state or watchers for a good first version?

## Proposed Rika V1

If the spike is clean, add a file provider with explicit `/` prefix mode.

Example config:

```toml
[providers.files]
enabled = true
roots = ["~/dev/projects", "~/Downloads"]
```

V1 behavior:

- `/` enters file-search mode.
- Search configured roots, not the whole filesystem.
- Use path/filename fuzzy search only.
- Return file path results with a default `open` action.
- Rebuild the index on provider refresh.
- Use Rika's central usage ranking first.
- Do not enable content grep, fff frecency, query history, or background watcher
  in the first slice.

The provider should keep activation semantics in Rust. QML should only render
the result and activate structured actions.

## Later Enhancements

- Add `copy path` once copy support exists.
- Add `reveal parent`.
- Add metadata for file type, parent directory, git status, and modified time.
- Add background watching if refresh-based indexing feels stale.
- Consider fff frecency if Rika's central usage ranking is not enough.
- Consider content grep as a separate provider or explicit mode, not part of
  file-name search v1.

## Caveats

`fff-search` is not a tiny dependency. It currently pulls in substantial
infrastructure such as `git2` with vendored libgit2, `heed`/LMDB, `notify`,
`rayon`, regex crates, and related indexing/search dependencies.

That may be acceptable for a launcher daemon, but it should be measured before
committing to it. The spike should check build time, runtime behavior, and Nix
package impact.
