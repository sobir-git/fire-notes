# Cursor Scaling Patches

## Problem

On GNOME Wayland with HiDPI displays, `winit` uses the `wp_cursor_shape_v1`
protocol if available. In Mutter 46, this produces a blurry cursor because
the cursor shape rendering doesn't account for the window's actual output
scale when using `wp_viewporter`.

The fallback path (loading cursor images via `libxcursor` + `set_buffer_scale`)
works correctly, but `smithay-client-toolkit` reads the cursor surface's
`scale_factor()` which is unreliable before the first cursor commit on GNOME —
it stays at 1 instead of the actual output scale (e.g. 2).

## Fix

Two minimal patches:

1. **`sctk-cursor-explicit-scale.patch`** — adds
   `ThemedPointer::set_cursor_explicit_scale(conn, icon, scale)` to SCTK,
   which bypasses `wp_cursor_shape_v1` and the stale cursor surface
   `scale_factor()`, using the caller-supplied scale directly.

2. **`winit-cursor-scale.patch`** — changes `WindowState::set_cursor` in
   winit to call `set_cursor_explicit_scale` with `self.scale_factor` (the
   window's actual output scale) instead of `set_cursor`.

## Setup

The full vendor trees are gitignored. Run this once after cloning:

```sh
bash patches/setup.sh
```

This copies winit-0.30.12 and smithay-client-toolkit-0.19.2 from your local
cargo cache and applies the patches.
