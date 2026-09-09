# Fire Notes native measurements

## Current framework implementation

Fire UI 0.6 separates native hosting, Cairo/OpenGL drawing, font resources and
Unicode shaping. Notes selects Cairo and its original multilingual font coverage,
plus clipboard, file dialogs and accessibility. No GPU renderer or retained
full-window client image is initialized. The default allocator is unchanged;
aggressive allocator tuning was not adopted.

The September 9 requirement revision prioritizes CPU, responsiveness and
correctness alongside memory. The original 3,000,000-byte private-dirty target
is still reported separately. The ordinary-workload regression ceiling is
4,000,000 bytes, with zero app swap; total private resident memory, RSS, PSS,
shared memory and external costs remain separate measurements. No replacement
has been installed or published. Native verification runs on isolated virtual desktops; it measures drawable
response rather than physical scanout.

The frozen ordinary workload uses three 8 KiB mixed-script notes, 312 native
keystrokes, active and selected fire, clipboard/undo/redo, tabs, scrolling,
600×400, 900×600 and 1200×800 logical windows at scale 2, and restoration. Larger
documents and tab counts are measured separately, without reducing this workload.

Heaptrack found duplicate paragraph ownership and many small layout allocations.
The implementation shares immutable source/layout snapshots, reuses shaping work
within each paragraph, and stores every undo step in fewer allocations. The
independent CPU audit also caught and fixed box-gradient painting over the whole
clip for tiny shapes. Geometry-bounded tiles preserve its pixels and avoid that
wasted painting work. CJK and emoji coverage is restored by default; the earlier
basic-font experiments are not full-feature acceptance evidence.

The release gate validates the exact binary and hashed native reports. It rejects
missing evidence and allocator overrides; a diagnostic report cannot authorize
installation. See [the release instructions](../README.md#release-and-installation-gate).

## Final native acceptance

The gate passed for binary `5d3e5f7703bbe02e08000342e91ee493e0ab956bf3002b11cd194f30b1c2a60b`.
[Summary](benchmarks/current.json) and [hash-bound acceptance manifest](../artifacts/acceptance-release/manifest.json)
retain the raw captures. All three fresh runs use default multilingual coverage,
normal allocation and the frozen workload above.

| Measurement | Three ordinary runs |
|---|---:|
| Private dirty | 3,493,888–3,510,272 bytes |
| Total private resident | 10,203,136–10,252,288 bytes |
| RSS | 17,063,936–17,162,240 bytes |
| PSS | 10,826,752–10,889,216 bytes |
| App CPU over complete workload | 2.06–2.11 seconds |
| Private Xvfb CPU, separately | 2.22–2.29 seconds |
| App swap / swap PSS | 0 bytes |
| Idle CPU over two seconds | 0 ticks |

The original private-dirty target is still **not met**. Private clean mappings
are included in total private resident memory; they are not counted as free.
Run 1 Xvfb RSS peaked at 75,771,904 bytes versus 73,785,344–74,133,504 before launch.
Its private dirty peak was 33,001,472 bytes versus 31,588,352–31,936,512 before.
The raw reports retain after-exit values, shared memory, other display processes,
DRM counters and XRes resource counts. XRes cannot attribute glyph bytes precisely;
zero glyph/pixmap estimates do not mean zero server cost. These counters overlap
and must not be added together.

Twenty native samples per response type gave p95 drawable latency of 1.93 ms for
idle pointer input, 3.14 ms during fire and 8.57 ms for resizing. These are private
X11 drawable observations, not physical scanout or hardware-GPU/power measurements.
Native file dialogs, editing, selection, clipboard, undo/redo, tabs, wrapping,
scrolling, restoration, IBus/Cangjie and Orca checks all pass. Screenshots preserve
the compact black-and-orange UI and animated text. Existing tab-title clipping at
420 pixels remains visible in the narrow-window capture.

| Separate growth workload | Peak private dirty bytes |
|---|---:|
| 1 KiB, one tab, startup | 2,154,496 |
| 8 KiB, one tab, startup | 2,404,352 |
| 64 KiB, one tab, startup | 4,575,232 |
| 256 KiB, one tab, startup | 12,091,392 |
| 1 MiB, one tab, startup | 42,131,456 |
| 8 KiB, ten tabs, complete editing | 3,702,784 |
| 64 KiB, one tab, complete editing | 9,555,968 |

All growth samples have zero swap. These larger workloads are measured separately;
they are not represented as fitting the ordinary ceiling. Long single logical
lines and concurrent accessibility query storms remain outside this matrix.

The growth test caught whole-document shaping twice per keystroke: the 64 KiB
case consumed 10.5 seconds of CPU and failed before typing finished. The editor
now supplies its existing layout through `TextRequest::previous`; the text service
reuses unchanged logical lines after validating text and metrics. A scrollbar
already required by hard line breaks does not force a second width calculation.
The complete 64 KiB workload now passes at 4.32 seconds of app CPU. Wide startup
geometry is released before allocating a narrower layout, avoiding a measured
66 MB transient peak on the 1 MiB document. No persistent document cache, allocator
override or history cap was added.

The final review was solo following the user's instruction to stop agents.
Earlier independent API and ownership reviews remain linked in the review receipt;
they are not described as reviews of the final layout change. Tests compare reused
and fresh geometry across Unicode edits, line deletions and wrapping widths.

## Historical OpenGL measurements

Measured on 2026-09-08 against Fire UI 0.5.0, release tag `v0.5.0` at `92e900b`.
The release build started in an empty `target/rebuild-0.5.0` directory. All three
framework crates use the release's public APIs directly. No framework fork or
compatibility layer is included.

The under-3-MB memory target is **not met** in a fresh active process. The installed
process shown in System Monitor had 2,956 KiB private dirty resident memory but
24,184 KiB swapped out. Starting the installed binary again and typing into a
temporary note gives a more useful comparison with a newly built binary.

[Desktop memory results](benchmarks/memory.json) record the executable hashes,
startup and post-typing samples on display `:0`, without forcing software GL.
Each process used an empty temporary library and typed the same short sentence.
All fresh samples had zero swap and zero CPU ticks over two seconds after settling.

| Build | Private dirty after typing | RSS after typing |
| --- | ---: | ---: |
| Previously installed binary, fresh launch | 25.3 MiB | 87.7 MiB |
| 0.5.0, default fonts | 26.0 MiB | 88.9 MiB |
| 0.5.0, `--full-fonts` | 40.3 MiB | 103.2 MiB |

In this historical experiment, choosing the smaller font set saved 14.3 MiB against full coverage in the same
0.5.0 executable. It does not beat the older installed binary in these samples.
DejaVu Sans remains enabled to cover characters missing from the monospace face.
CJK and color-emoji font files were opt-in in that experiment. The comparison without a retained
repaint image did not reduce desktop private memory, so that OpenGL experiment retained partial repainting. These are single-run measurements, not a guaranteed memory ceiling.

Follow-up [renderer isolation](benchmarks/renderer-isolation.json) separates the
native host from app composition. The released draw-only consumer enables just
`x11`, loads no fonts and uses no widget crate or retained image. A separate C
control creates an OpenGL window without Fire UI and uses 12.8 MiB private dirty
memory on the Intel Iris Xe driver. An X11/Cairo control drawing one line of text
uses 1.3 MiB. Both have zero swap. The raw measurements include the control source
and compile command. The Cairo control does not implement the editor or establish
that a complete alternative host meets the budget.

Fire UI 0.5.0's native host requires OpenGL, even with optional services disabled.
Consequently, trimming application services cannot satisfy a 3,000,000-byte
private-memory limit on this measured OpenGL path. A different rendering path
needs a complete editor, animation, input-method and accessibility verification
before it can replace the host. No replacement has been installed.

The initial memory probe enforced the original 3,000,000-byte exclusive private
dirty limit and rejected swap. The released OpenGL build failed it. The current
policy and full release gate are described above. See the framework's
[historical trace](../../fire-ui/docs/performance.md#history-of-the-missed-memory-limit)
for the commits and the missing release check.

[Native interaction results](benchmarks/notes.json) use Xvfb and Mesa software
rendering with temporary notes and session data. They are separate from the
desktop memory comparison.

After fire animation stopped, the app recorded zero CPU ticks over two seconds.
RSS was 149.7 MiB, including software GL, font fallback and glyph caches. Across 66
rendered resize samples, event-received to completed swap was 6.61 ms median,
7.89 ms at the 95th percentile and 8.81 ms maximum. These figures exclude
compositor presentation. Other work on the machine can affect timings.

The app explicitly enables its required native services, font files and retained
partial repainting. Framework defaults leave those choices to each consumer.

The native probe passed typing, clipboard, undo/redo, inline rename, tabs, search,
commands, native file opening, raw Markdown preservation, wrap, scrollbar dragging,
420-pixel layouts, save-before-close and session restoration. Selected text remains
animated. The editor decoration reports its changed bounds through Fire UI's public
interface; small fire frames can repaint fewer than 1,000 pixels.

The inspection probe enabled `--full-fonts`, found the editor by its app key, replaced Unicode text,
preserved reversed selection, rejected invalid/stale requests and verified native
keyboard undo. It also recorded zero idle CPU ticks with the socket enabled, and
verified socket cleanup on orderly exit. Its screenshot shows Hebrew, Arabic,
combining characters, color emoji and animated selected text. The probes do not
establish every input method or assistive-device combination. The framework's Linux
probe separately verifies installed IBus/XIM composition, stale-composition cancellation,
Orca speech generation/navigation and all six native AT-SPI EditableText methods.

All 14 app tests and Clippy with warnings as errors passed. Framework tests and
package checks run separately in [Fire UI](../../fire-ui/docs/performance.md).

## Reproduction

```sh
cargo build --release
python3 tools/notes_probe.py --output artifacts/notes
python3 tools/inspection_probe.py --output artifacts/inspection
python3 tools/memory_probe.py --output artifacts/memory.json
# Optional limited-font diagnostic; not the default acceptance workload.
python3 tools/memory_probe.py --output artifacts/memory-basic.json -- --basic-fonts
```
