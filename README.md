# Fire Notes

A compact native notes app built with [Fire UI](../fire-ui/README.md).

## Build and run

Keep the two independent repositories beside each other:

```text
projects/
  fire-ui/       # framework, widgets, native host and studio
  fire-notes/    # this app
```

The Fire UI path dependencies require the sibling 0.6.0 development checkout.
Framework edits are compiled directly on the next app build. Each project owns
its Git history, Cargo manifest, lockfile and checks. Fire UI builds independently
and has no dependency on Fire Notes.

```sh
cargo run --release -- --data-dir /path/to/notes
```

Fire Notes explicitly selects the Cairo/X11 renderer, Unicode text engine, native
accessibility, clipboard, file dialogs and agent inspection. Inspection starts
only when `FIRE_UI_INSPECT` is set. Cairo draws directly into the native window
without retaining a client framebuffer or initializing OpenGL.

The app maps DejaVu Sans Mono, DejaVu Sans and installed CJK/color-emoji fallback
fonts by default, preserving its original coverage. `--basic-fonts` explicitly
omits the CJK/emoji fallbacks; note files still preserve their Unicode text. Mapped font files must stay immutable
while the app holds them; the framework also supports owned font bytes.

The full native acceptance gate passes on isolated virtual desktops. Three fresh
ordinary runs measured 3,493,888–3,510,272 private dirty bytes and
10,203,136–10,252,288 total private resident bytes, with zero swap and zero sampled
idle CPU ticks. The original 3 MB target was not met. Following the September 9
priority change, the regression ceiling is under 4,000,000 private dirty bytes,
with responsiveness, native behavior and CPU checked alongside memory.
[Measurements and limitations](docs/performance.md) include display-server costs,
document growth and the exact binary. No replacement has been installed or released.

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
python3 tools/memory_probe.py --output artifacts/memory.json
```

The probes require Linux, Xvfb, xdotool and Pillow with XCB support. The notes
probe also requires xclip and a native file chooser such as Zenity. They use
isolated displays and temporary notes. They check real editing, fire animation,
menus, Trash, file dialogs, resizing, saving and session restoration.
The memory probe opens a temporary test window on the current desktop, briefly
focuses it for typing, then restores focus. It records private memory, RSS, PSS,
swap and idle CPU. It exits with failure if any sampled startup, editing, animated
selection or resized state reaches 4,000,000 bytes of private dirty memory or uses
swap. The current native host fails this acceptance check. Run it without
interacting with other windows during its test.
The optional `--telegram` flag on the notes probe sends screenshots through the
locally installed telegram-notify skill, only when delivery is requested.

App code lives in `src/`. Themes, tabs, fire decoration and note persistence belong
here. Reusable widget and native-host changes belong in the Fire UI repository
and must remain available through public APIs. When a change touches both projects,
run both test suites and the affected native probes.

See [visual design](DESIGN.md), [app measurements](docs/performance.md),
[framework architecture](../fire-ui/docs/architecture.md) and [project rules](AGENTS.md).

Agent inspection uses Fire UI's public semantic interface. Run with `FIRE_UI_INSPECT`
pointing to a socket in a private directory, then use the sibling framework's
`tools/fire_ui_inspect.py` to read the tree and edit `note-body` by key. See
[the protocol](../fire-ui/docs/inspection.md). `python3 tools/inspection_probe.py`
uses the default font coverage and checks semantic editing, Unicode selection, native undo and socket cleanup against
an isolated X11 window with temporary notes. Wayland work is deferred.


## Release and installation gate

A memory report alone cannot authorize installation. Validate the exact release
binary and all native evidence with:

```sh
python3 tools/release.py --binary target/release/fire-notes --manifest artifacts/acceptance.json
python3 -m unittest discover -s tools -p test_release.py
```

The schema-1 manifest contains `binary_sha256` and `reports` with exactly five
entries: `memory`, `native_input`, `interactions`, `pointer_latency`, and `review`.
Each entry contains `path` (relative to the manifest) and `sha256`. Every report
must name that same binary hash. Referenced screenshots, logs and raw reports
also use `{path, sha256}`, relative to their containing report. Hashes bind the
review to particular artifacts; they do not substitute for trustworthy capture
and independent review.

The gate recomputes every ordinary memory sample against an exclusive
4,000,000-byte Private_Dirty regression ceiling and rejects any app Swap or SwapPss.
This gives headroom over the measured normal-allocator peak without accepting a
return to the former 20–30 MB cost. The probe also reports whether the original
3,000,000-byte target was met; missing that target is not hidden as a target pass. The frozen
workload is three 8 KiB notes, 312 typed characters, active and selected fire,
clipboard history, scrolling, and 600×400, 900×600 and 1200×800 logical windows,
followed by session restoration. Raw RSS, PSS, private/shared memory and external
before/during/after accounting are required. The automated workload uses an
isolated Xvfb desktop and an enabled private accessibility bus. Drawable latency
is measured there; physical scanout and hardware-GPU performance remain unmeasured.

`memory` uses the benchmark aggregate: three ordinary runs plus the document/tab
growth matrix. Raw reports record `accessibility_bus: "private_enabled"` and
`software_gl_override: false`. `native_input`
uses the framework's `linux_input_probe.py --target notes` result, with an
`artifacts` map hashing `ime-preedit.json`, `ime-preedit.png` and `orca-speech.log`.
`interactions` aggregates the three native Notes probes: their actual completion
receipts, idle measurements, tab assertions, semantic tree, restored session,
screenshots and probe sources are hash-bound. It does not synthesize expected or
observed outcomes. `tools/verify.py` collects these reports sequentially on private
displays and writes a five-report manifest with independent review pending.

```sh
python3 tools/verify.py --binary target/release/fire-notes --output artifacts/verification
```

The output directory must be new. To reuse unchanged captured evidence, pass
`--assemble-existing captures.json`, whose `memory`, `native_input`,
`pointer_latency` and `interactions` paths identify the benchmark aggregate,
input result, latency result and three-probe capture directory. Relative paths
resolve against that JSON file. Assembly copies and validates existing artifacts;
it does not describe them as fresh runs. The generated `review.json` remains
pending and the release gate rejects it until independent review is completed.


`pointer_latency` schema 1 measures native requests to app drawable pixels on an
isolated virtual X11 desktop (or an optionally selected unlocked physical desktop) (`native_input_to_drawable_pixels`): at least 20 samples each for idle pointer, pointer during fire and resize,
with raw monotonic timestamps, changed target pixels and hashed before/after
frames. The gate uses 50 ms p95 and 150 ms maximum as its responsiveness limits.
Run `python3 tools/pointer_latency_probe.py --binary target/release/fire-notes
--output artifacts/pointer-latency` to produce it. The output directory must be
new. The probe uses temporary notes and a private service bus, checks the desktop
lock before launching in optional `--physical-display` mode, and restores prior
focus there. By default it starts its own Xvfb server; this virtual native
evidence is accepted without claiming hardware presentation. Header response colors
are calibrated before trials, then small app-only regions are polled with
`XGetImage`. Resize trials issue `XResizeWindow`, not pointer border drags.
Drawable pixels establish server-side rendering response, not compositor scanout
or physical display timing; command completion alone is insufficient.

`review` schema 1 names the
independent reviewer, date and exact hashes of the other four reports, records
visual observations and hashed screenshots for every state in
`release.VISUAL_STATES`, and reviews external costs without double-counting shared
buffers or concealing preexisting external swap. It also references measured
1/8/64/256 KiB and 1 MiB document growth and 1/3/10-tab workloads. Larger workloads
are reported separately; they cannot replace the ordinary acceptance workload.
The precise field validation lives in [tools/release.py](tools/release.py).

Missing, stale, malformed or failing evidence exits nonzero. Current private-display
diagnostics do not authorize release; final interaction, desktop drawable latency and
review artifacts must also be completed. Once all evidence
passes, the same command with `--install-to /explicit/path/fire-notes` atomically
copies only the validated executable. It rechecks the copied binary hash and
never launches the app or changes notes, sessions or service configuration.
