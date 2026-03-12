//! Window creation and OpenGL context setup.

use std::ffi::CString;
use std::num::NonZeroU32;

use glutin::config::ConfigTemplateBuilder;
use glutin::context::{ContextApi, ContextAttributesBuilder, PossiblyCurrentContext};
use glutin::display::GetGlDisplay;
use glutin::prelude::*;
use glutin::surface::{Surface, SurfaceAttributesBuilder, WindowSurface};
use glutin_winit::DisplayBuilder;
use raw_window_handle::HasWindowHandle;
use winit::dpi::{LogicalSize, PhysicalPosition, PhysicalSize};
use winit::event_loop::ActiveEventLoop;
use winit::window::{Window, WindowAttributes};

use crate::app::App;
use crate::persistence::{WindowState, load_window_state};

pub struct AppState {
    pub window: Window,
    pub gl_context: PossiblyCurrentContext,
    pub gl_surface: Surface<WindowSurface>,
    pub app: App,
}

pub fn capture_window_state(window: &Window) -> Option<WindowState> {
    let position = window.outer_position().ok()?;
    let size = window.inner_size();
    if size.width == 0 || size.height == 0 { return None; }
    Some(WindowState { x: position.x, y: position.y, width: size.width, height: size.height })
}

pub fn build_app_state(event_loop: &ActiveEventLoop) -> AppState {
    let mut window_attrs = WindowAttributes::default()
        .with_title("Fire Notes")
        .with_decorations(false);

    #[cfg(target_os = "linux")]
    {
        use winit::platform::wayland::WindowAttributesExtWayland;
        use winit::platform::x11::WindowAttributesExtX11;
        window_attrs = WindowAttributesExtWayland::with_name(window_attrs, "fire-notes", "fire-notes");
        window_attrs = WindowAttributesExtX11::with_name(window_attrs, "fire-notes", "fire-notes");
    }

    if let Some(saved) = load_window_state() {
        window_attrs = window_attrs
            .with_inner_size(PhysicalSize::new(saved.width, saved.height))
            .with_position(PhysicalPosition::new(saved.x, saved.y));
    } else {
        window_attrs = window_attrs.with_inner_size(LogicalSize::new(600.0, 400.0));
    }

    let config_template = ConfigTemplateBuilder::new()
        .with_alpha_size(8)
        .with_multisampling(4);

    let display_builder = DisplayBuilder::new().with_window_attributes(Some(window_attrs));
    let (window, gl_config) = display_builder
        .build(event_loop, config_template, |configs| {
            configs
                .reduce(|accum, config| {
                    if config.num_samples() > accum.num_samples() { config } else { accum }
                })
                .expect("No GL configs found")
        })
        .expect("Failed to create window");

    let window = window.expect("Window not created");
    let gl_display = gl_config.display();

    let context_attrs = ContextAttributesBuilder::new()
        .with_context_api(ContextApi::OpenGl(None))
        .build(Some(
            window.window_handle().expect("Failed to get window handle").as_raw(),
        ));

    let gl_context = unsafe {
        gl_display
            .create_context(&gl_config, &context_attrs)
            .expect("Failed to create GL context")
    };

    let size = window.inner_size();
    let surface_attrs = SurfaceAttributesBuilder::<WindowSurface>::new().build(
        window.window_handle().expect("Failed to get window handle").as_raw(),
        NonZeroU32::new(size.width.max(1)).unwrap(),
        NonZeroU32::new(size.height.max(1)).unwrap(),
    );

    let gl_surface = unsafe {
        gl_display
            .create_window_surface(&gl_config, &surface_attrs)
            .expect("Failed to create surface")
    };

    let gl_context = gl_context
        .make_current(&gl_surface)
        .expect("Failed to make context current");

    let renderer = unsafe {
        femtovg::renderer::OpenGl::new_from_function_cstr(|name| {
            let cstr = CString::new(name.to_bytes()).unwrap();
            gl_display.get_proc_address(&cstr) as *const _
        })
        .expect("Failed to create renderer")
    };

    let scale = window.scale_factor() as f32;
    let app = App::new(renderer, size.width as f32, size.height as f32, scale);

    AppState { window, gl_context, gl_surface, app }
}
