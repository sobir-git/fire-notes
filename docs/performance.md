# Fire Notes native measurements

These measurements predate the repository split. The recorded binary hash identifies
the tested build; they are historical evidence, not measurements of every later commit.

## Fire Notes

The restored original interface was measured on 2026-09-07 using Xvfb and Mesa
software rendering. [Raw app results](benchmarks/notes.json) identify the measured
release binary. The workload includes four tabs, a 100-line document, fire animation,
clipboard edits, native Open/Save As dialogs, and a restart.

After the fire faded, a focused editor used **zero CPU ticks over two seconds**.
Process RSS was 151.0 MiB, including software GL and native libraries.
The release executable was 7.79 MiB. These short samples establish
quiet idle behavior for this run, not hardware GPU power use or an RSS ceiling.

Across 64 rendered resize samples, app-received event to completed swap was
4.48 ms median, 6.48 ms at the 95th percentile, and 6.97 ms maximum.
This excludes compositor presentation latency. The earlier app workload was different,
so these figures are not a controlled speedup or memory regression comparison.

The native check passed initial typing, animated fire, clipboard, undo/redo, inline
rename, tab dragging, search, reopening, command execution, native Open and Save As,
raw Markdown preservation, per-tab wrap, scrollbar drag, 420-pixel layout, saving
immediately before close, and restoration of cursor, scroll, titles, tabs and placement.
Xvfb has no window manager; the probe supplies the ICCCM move notification for that
placement check. Eleven final screenshots were visually inspected against the original
app reference. The combined workspace at measurement time had **50 passing tests** and Clippy passes with warnings
as errors. File-drop delivery while blurred/modal, tabs in native text, selection
movement, scroll restoration, and bounded fire scheduling have regression tests.

```sh
cargo build --release -p fire-notes
python3 tools/notes_probe.py --output artifacts/notes
```


Framework measurements live in [Fire UI](../../fire-ui/docs/performance.md).
