# Fire Notes

A compact native notes app built with [Fire UI](../fire-ui/README.md).

## Build and run

Keep the two independent repositories beside each other:

```text
projects/
  fire-ui/       # framework, widgets, native host and studio
  fire-notes/    # this app
```

The three Fire UI dependencies in `Cargo.toml` point to the sibling checkout.
Framework edits are compiled directly on the next app build. Each project owns
its Git history, Cargo manifest, lockfile and checks. Fire UI builds independently
and has no dependency on Fire Notes.

```sh
cargo run --release -- --data-dir /path/to/notes
```

## App behavior

Fire Notes reproduces the original compact black-and-orange interface: inline tab
titles, warm monospaced text, a caret-anchored command palette, and animated fire on
typed and selected text. The app composes the new framework's public widgets and
editor decoration API. The old renderer and widgets are not part of the build.

Notes live in `tmp/fire-notes` by default. Set `FIRE_NOTES_DIR` or pass
`--data-dir PATH` to choose another folder. Files contain raw Markdown or plain text;
titles and session state are separate metadata. Closing a tab keeps its file. The
session remembers tab order, active note, caret, selection, scroll, per-tab wrap,
window size and position. Saves run on one background thread. Closing waits for
outstanding note saves and keeps the window open if a note cannot be saved.

| Shortcut or gesture | Action |
| --- | --- |
| Ctrl N / Ctrl W | New note / close tab; the last saved tab stays open |
| Ctrl Tab / Ctrl Shift Tab / Ctrl 1–9 | Switch tabs |
| Right click tab | Rename, Save, Word wrap, Close and Trash actions |
| Ctrl R / middle click tab | Rename; Enter or blur commits, Escape cancels |
| Drag tab / wheel over tabs | Reorder / scroll tabs |
| Ctrl P | Search saved notes |
| Ctrl Shift T | Open Trash and restore a note |
| Ctrl O / drop a file | Open a Markdown or text file |
| Ctrl Shift O | Open by entering a path |
| Ctrl / | Commands beside the caret |
| Ctrl S / Ctrl Shift S | Save / system Save As dialog |
| Ctrl Z / Ctrl Shift Z / Ctrl Y | Undo / redo |
| Ctrl C / Ctrl X / Ctrl V | Clipboard |
| Alt Z | Toggle this tab's word wrap, off by default |
| Alt Up / Alt Down | Move the current or selected lines |
| Double / triple click, Shift click | Select word / line, extend selection |
| Ctrl Q | Save and quit |

Arrow, word, line, document and page navigation support keyboard selection. Drag
selection scrolls beyond the viewport; the scrollbar also supports dragging. Custom
window controls support minimize, maximize/restore, drag and edge resizing. Fire
animation stops after typing fades, when selection clears, or when the editor loses
focus. It does not keep an idle editor repainting.

Right-click a tab and choose **Move to Trash** to remove its note from the library.
**Open Trash**, also available in the command palette, lists recoverable notes with
their remaining time. Select one to restore its title, text, cursor and wrapping.
Restoring never overwrites a file that has appeared at its original location.
Close tab only closes the tab; it does not trash the note.
New empty Untitled tabs stay in memory and are discarded on close or quit, including
the last tab. They create no note file and are excluded from the saved session.
Typing content, changing the name, or explicitly saving makes a note persistent.
New tabs use the lowest available Untitled number; discarded drafts do not consume numbers.

Trash expires after 30 days. Cleanup runs at startup and hourly while the app is
open. The installed Linux user timer also runs hourly while the app is closed and
catches up after login. Install it after installing the app launcher:

```sh
install -Dm644 tools/systemd/fire-notes-trash.service ~/.config/systemd/user/fire-notes-trash.service
install -Dm644 tools/systemd/fire-notes-trash.timer ~/.config/systemd/user/fire-notes-trash.timer
systemctl --user daemon-reload
systemctl --user enable --now fire-notes-trash.timer
```

`fire-notes --purge-trash` performs the same expiry check without opening a window.
Recovery copies and timestamps live in the library's `trash/` directory.

This prototype has no format migration, file watching or concurrent-writer conflict
resolution. Note bodies are limited to 2 MiB and titles to 4 KiB; the framework editor's
byte limit is configurable. There is no app-specific open-tab cap, although framework
node and message budgets still apply. Large-document editing needs further profiling.

## Development

```sh
cargo fmt --check
cargo test --locked
cargo clippy --locked --all-targets -- -D warnings
cargo build --locked --release
python3 tools/notes_probe.py --output artifacts/notes
python3 tools/tab_menu_probe.py --output artifacts/tab-menu
```

The probes require Linux, Xvfb, xdotool and Pillow with XCB support. The notes
probe also requires xclip and a native file chooser such as Zenity. They use
isolated displays and temporary notes. They check real editing, fire animation,
menus, Trash, file dialogs, resizing, saving and session restoration.
The optional `--telegram` flag on the notes probe sends screenshots through the
locally installed telegram-notify skill, only when delivery is requested.

App code lives in `src/`. Themes, tabs, fire decoration and note persistence belong
here. Reusable widget and native-host changes belong in the Fire UI repository
and must remain available through public APIs. When a change touches both projects,
run both test suites and the affected native probes.

See [visual design](DESIGN.md), [app measurements](docs/performance.md),
[framework architecture](../fire-ui/docs/architecture.md) and [project rules](AGENTS.md).
