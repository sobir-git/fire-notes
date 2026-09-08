# Fire Notes native measurements

Measured on 2026-09-08 with release optimization, Xvfb and Mesa software rendering.
[Raw results](benchmarks/notes.json) identify the tested binary. This is an isolated
native test using temporary notes and session data, not a hardware-GPU measurement.

After fire animation stopped, the app recorded zero CPU ticks over two seconds.
RSS was 162.9 MiB, including software GL, font fallback and glyph caches. Across 66
rendered resize samples, event-received to completed swap was 7.53 ms median,
10.73 ms at the 95th percentile and 11.97 ms maximum. These figures exclude
compositor presentation. Other work on the machine can affect timings.

The app explicitly enables its required native services, font files and retained
partial repainting. Framework defaults leave those choices to each consumer.

The native probe passed typing, clipboard, undo/redo, inline rename, tabs, search,
commands, native file opening, raw Markdown preservation, wrap, scrollbar dragging,
420-pixel layouts, save-before-close and session restoration. Selected text remains
animated. The editor decoration reports its changed bounds through Fire UI's public
interface; small fire frames can repaint fewer than 1,000 pixels.

The inspection probe found the editor by its app key, replaced Unicode text,
preserved reversed selection, rejected invalid/stale requests and verified native
keyboard undo. It also recorded zero idle CPU ticks with the socket enabled, and
verified socket cleanup on orderly exit. Its screenshot shows Hebrew, Arabic,
combining characters, color emoji and animated selected text. The probes do not
establish every input method or assistive-device combination. The framework's Linux
probe separately verifies installed IBus/XIM composition, stale-composition cancellation,
Orca speech generation/navigation and all six native AT-SPI EditableText methods.

All 14 app tests and Clippy with warnings as errors passed. Framework tests and
package checks run separately in [Fire UI](../../fire-ui/docs/performance.md).

```sh
cargo build --release
python3 tools/notes_probe.py --output artifacts/notes
python3 tools/inspection_probe.py --output artifacts/inspection
```
