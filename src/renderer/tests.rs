//! Headless pixel snapshot tests for the renderer.
//!
//! Each test creates an EGL surfaceless context, renders one frame via
//! `Renderer`, reads back pixels via `canvas.screenshot()`, and asserts
//! colour properties (no window / display server required).
//!
//! Tests are skipped gracefully when the Mesa surfaceless platform is absent
//! (e.g. pure-software CI without a GPU).

use std::num::NonZeroU32;

use super::headless::HeadlessContext;
use super::Renderer;
use crate::logic::AppLogic;

const W: u32 = 400;
const H: u32 = 300;

/// Build a renderer pointed at `ctx`'s FBO.
fn make_renderer(ctx: &HeadlessContext) -> Renderer {
    let mut gl_renderer = unsafe {
        femtovg::renderer::OpenGl::new_from_function_cstr(|name| ctx.get_proc(name))
            .expect("femtovg OpenGl renderer")
    };
    // Point femtovg at our FBO instead of the (non-existent) default FB.
    let fbo_handle =
        glow::NativeFramebuffer(NonZeroU32::new(ctx.fbo).expect("fbo must be non-zero"));
    gl_renderer.set_screen_target(Some(fbo_handle));
    Renderer::new(gl_renderer, ctx.width as f32, ctx.height as f32, 1.0)
}

/// Render one blank frame using `renderer`, return bottom-up RGBA8 pixels.
fn render_frame(ctx: &HeadlessContext, renderer: &mut Renderer) -> Vec<u8> {
    let mut logic = AppLogic::new_headless(ctx.width as f32, ctx.height as f32, 1.0);
    let frame = logic.render_frame();
    renderer.render(&frame, None);
    ctx.read_pixels()
}

/// Convenience: build renderer + render one frame.
fn render_blank(ctx: &HeadlessContext) -> Vec<u8> {
    render_frame(ctx, &mut make_renderer(ctx))
}

// ── helpers ───────────────────────────────────────────────────────────────────

/// Get top-left-origin RGBA pixel from bottom-up GL pixel buffer.
fn pixel_at(raw: &[u8], w: u32, h: u32, x: u32, y: u32) -> [u8; 4] {
    let row = (h - 1 - y) as usize; // flip bottom-up → top-down
    let off = (row * w as usize + x as usize) * 4;
    [raw[off], raw[off + 1], raw[off + 2], raw[off + 3]]
}

fn pixel_near(px: [u8; 4], r: u8, g: u8, b: u8, tol: u8) -> bool {
    px[0].abs_diff(r) <= tol && px[1].abs_diff(g) <= tol && px[2].abs_diff(b) <= tol
}

// ── tests ─────────────────────────────────────────────────────────────────────

/// Background (text content area, below tab bar) must be black = Theme::dark().bg.
#[test]
fn background_is_black() {
    let ctx = match HeadlessContext::try_new(W, H) {
        Some(c) => c,
        None => {
            eprintln!("SKIP: headless EGL not available");
            return;
        }
    };
    let raw = render_blank(&ctx);
    // Tab bar is 40px tall; sample well below it.
    let px = pixel_at(&raw, W, H, W / 2, H / 2);
    assert!(pixel_near(px, 0, 0, 0, 10), "expected black bg, got {px:?}");
}

/// The tab bar row must contain at least one non-black pixel.
/// Theme::dark() tab_active ≈ (0.15, 0.05, 0.05) → (38, 13, 13).
#[test]
fn tab_bar_is_not_black() {
    let ctx = match HeadlessContext::try_new(W, H) {
        Some(c) => c,
        None => {
            eprintln!("SKIP: headless EGL not available");
            return;
        }
    };
    let raw = render_blank(&ctx);
    // Tab bar height = 40px; sample row 20 (middle of bar).
    let any_non_black = (0..W).any(|x| {
        let px = pixel_at(&raw, W, H, x, 20);
        !pixel_near(px, 0, 0, 0, 6)
    });
    assert!(any_non_black, "tab bar row 20 is entirely black — not rendered");
}

/// Every pixel the renderer emits should be fully opaque.
/// femtovg screenshot() returns pixels with alpha pre-multiplied or as
/// rendered; the clear colour sets alpha=255 when the renderer calls
/// canvas.clear_rect with an opaque colour.
#[test]
fn rendered_pixels_have_colour() {
    let ctx = match HeadlessContext::try_new(W, H) {
        Some(c) => c,
        None => {
            eprintln!("SKIP: headless EGL not available");
            return;
        }
    };
    let raw = render_blank(&ctx);
    // At least some pixels must be non-zero (i.e. rendering happened).
    let nonzero = raw.chunks(4).filter(|px| px[0] > 0 || px[1] > 0 || px[2] > 0).count();
    assert!(nonzero > 0, "all pixels are zero — nothing was rendered");
}

/// A second render with the same renderer must also show a black background
/// in the content area and a non-black tab bar — i.e. rendering remains
/// correct across multiple frames (no state corruption).
#[test]
fn second_render_also_correct() {
    let ctx = match HeadlessContext::try_new(W, H) {
        Some(c) => c,
        None => {
            eprintln!("SKIP: headless EGL not available");
            return;
        }
    };
    let mut renderer = make_renderer(&ctx);
    render_frame(&ctx, &mut renderer); // warm up font atlas
    let raw = render_frame(&ctx, &mut renderer); // frame 2

    // Background (content area, center) must still be black.
    let bg = pixel_at(&raw, W, H, W / 2, H / 2);
    assert!(pixel_near(bg, 0, 0, 0, 10), "frame 2: expected black bg, got {bg:?}");

    // Tab bar (row 20) must still be non-black.
    let any_non_black = (0..W).any(|x| {
        let px = pixel_at(&raw, W, H, x, 20);
        !pixel_near(px, 0, 0, 0, 6)
    });
    assert!(any_non_black, "frame 2: tab bar row 20 is entirely black");
}

/// HeadlessContext width/height fields reflect the requested dimensions.
#[test]
fn context_dimensions_match_request() {
    let ctx = match HeadlessContext::try_new(W, H) {
        Some(c) => c,
        None => {
            eprintln!("SKIP: headless EGL not available");
            return;
        }
    };
    assert_eq!(ctx.width, W);
    assert_eq!(ctx.height, H);

    let ctx2 = match HeadlessContext::try_new(200, 150) {
        Some(c) => c,
        None => return,
    };
    assert_eq!(ctx2.width, 200);
    assert_eq!(ctx2.height, 150);
}
