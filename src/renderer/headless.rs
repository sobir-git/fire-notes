//! Headless EGL + Mesa offscreen rendering for pixel snapshot tests.
//!
//! Uses EGL_PLATFORM_SURFACELESS_MESA to create an OpenGL context without
//! any window or display server.  The FBO is the render target; pixels are
//! read back with `glReadPixels`.

use std::ffi::{CStr, CString, c_void};
use std::ptr;

// ── EGL constants ─────────────────────────────────────────────────────────────
const EGL_PLATFORM_SURFACELESS_MESA: u32 = 0x31DD;
const EGL_NONE_I: i32 = 0x3038;
const EGL_SURFACE_TYPE: i32 = 0x3033;
const EGL_PBUFFER_BIT: i32 = 0x0001;
const EGL_RENDERABLE_TYPE: i32 = 0x3040;
const EGL_OPENGL_BIT: i32 = 0x0008;
const EGL_RED_SIZE: i32 = 0x3024;
const EGL_GREEN_SIZE: i32 = 0x3023;
const EGL_BLUE_SIZE: i32 = 0x3022;
const EGL_ALPHA_SIZE: i32 = 0x3021;
const EGL_DEPTH_SIZE: i32 = 0x3025;
const EGL_CONTEXT_MAJOR_VERSION: i32 = 0x3098;
const EGL_CONTEXT_MINOR_VERSION: i32 = 0x30FB;
const EGL_OPENGL_API: u32 = 0x30A2;
const EGL_FALSE: u32 = 0;

// ── GL constants ──────────────────────────────────────────────────────────────
const GL_FRAMEBUFFER: u32 = 0x8D40;
const GL_RENDERBUFFER: u32 = 0x8D41;
const GL_RGBA8: u32 = 0x8058;
const GL_DEPTH24_STENCIL8: u32 = 0x88F0;
const GL_COLOR_ATTACHMENT0: u32 = 0x8CE0;
const GL_DEPTH_STENCIL_ATTACHMENT: u32 = 0x821A;
const GL_FRAMEBUFFER_COMPLETE: u32 = 0x8CD5;
const GL_RGBA: u32 = 0x1908;
const GL_UNSIGNED_BYTE: u32 = 0x1401;

// ── libc dynamic linking ──────────────────────────────────────────────────────
const RTLD_LAZY: i32 = 0x0001;
const RTLD_GLOBAL: i32 = 0x0100;

extern "C" {
    fn dlopen(filename: *const i8, flags: i32) -> *mut c_void;
    fn dlsym(handle: *mut c_void, symbol: *const i8) -> *mut c_void;
    fn dlclose(handle: *mut c_void) -> i32;
}

unsafe fn sym<T: Copy>(lib: *mut c_void, name: &[u8]) -> Option<T> {
    let ptr = unsafe { dlsym(lib, name.as_ptr() as *const i8) };
    if ptr.is_null() {
        None
    } else {
        Some(unsafe { *(&ptr as *const *mut c_void as *const T) })
    }
}

unsafe fn sym_req<T: Copy>(lib: *mut c_void, name: &[u8]) -> T {
    unsafe { sym(lib, name) }.unwrap_or_else(|| panic!("symbol not found: {}", std::str::from_utf8(name).unwrap()))
}

// ── EGL function-pointer types ────────────────────────────────────────────────
type PfnGetProcAddress = unsafe extern "C" fn(*const i8) -> *mut c_void;
type PfnGetPlatformDisplayEXT =
    unsafe extern "C" fn(u32, *mut c_void, *const i64) -> *mut c_void;
type PfnInitialize =
    unsafe extern "C" fn(*mut c_void, *mut i32, *mut i32) -> u32;
type PfnChooseConfig = unsafe extern "C" fn(
    *mut c_void, *const i32, *mut *mut c_void, i32, *mut i32,
) -> u32;
type PfnBindApi = unsafe extern "C" fn(u32) -> u32;
type PfnCreateContext = unsafe extern "C" fn(
    *mut c_void, *mut c_void, *mut c_void, *const i32,
) -> *mut c_void;
type PfnMakeCurrent =
    unsafe extern "C" fn(*mut c_void, *mut c_void, *mut c_void, *mut c_void) -> u32;
type PfnDestroyContext =
    unsafe extern "C" fn(*mut c_void, *mut c_void) -> u32;
type PfnTerminate = unsafe extern "C" fn(*mut c_void) -> u32;

// ── GL function-pointer types ─────────────────────────────────────────────────
type PfnGenFramebuffers = unsafe extern "C" fn(i32, *mut u32);
type PfnBindFramebuffer = unsafe extern "C" fn(u32, u32);
type PfnGenRenderbuffers = unsafe extern "C" fn(i32, *mut u32);
type PfnBindRenderbuffer = unsafe extern "C" fn(u32, u32);
type PfnRenderbufferStorage = unsafe extern "C" fn(u32, u32, i32, i32);
type PfnFramebufferRenderbuffer = unsafe extern "C" fn(u32, u32, u32, u32);
type PfnCheckFramebufferStatus = unsafe extern "C" fn(u32) -> u32;
type PfnReadPixels =
    unsafe extern "C" fn(i32, i32, i32, i32, u32, u32, *mut c_void);
type PfnDeleteFramebuffers = unsafe extern "C" fn(i32, *const u32);
type PfnDeleteRenderbuffers = unsafe extern "C" fn(i32, *const u32);

/// Loaded GL function table (only what we need for FBO setup + readback).
struct Gl {
    gen_framebuffers: PfnGenFramebuffers,
    bind_framebuffer: PfnBindFramebuffer,
    gen_renderbuffers: PfnGenRenderbuffers,
    bind_renderbuffer: PfnBindRenderbuffer,
    renderbuffer_storage: PfnRenderbufferStorage,
    framebuffer_renderbuffer: PfnFramebufferRenderbuffer,
    check_framebuffer_status: PfnCheckFramebufferStatus,
    read_pixels: PfnReadPixels,
    delete_framebuffers: PfnDeleteFramebuffers,
    delete_renderbuffers: PfnDeleteRenderbuffers,
}

impl Gl {
    unsafe fn load(get_proc: PfnGetProcAddress) -> Self {
        macro_rules! load {
            ($name:expr, $ty:ty) => {{
                let ptr = unsafe { get_proc($name.as_ptr() as *const i8) };
                assert!(!ptr.is_null(), "GL sym not found: {}", std::str::from_utf8($name).unwrap());
                unsafe { *(&ptr as *const *mut c_void as *const $ty) }
            }};
        }
        Self {
            gen_framebuffers: load!(b"glGenFramebuffers\0", PfnGenFramebuffers),
            bind_framebuffer: load!(b"glBindFramebuffer\0", PfnBindFramebuffer),
            gen_renderbuffers: load!(b"glGenRenderbuffers\0", PfnGenRenderbuffers),
            bind_renderbuffer: load!(b"glBindRenderbuffer\0", PfnBindRenderbuffer),
            renderbuffer_storage: load!(b"glRenderbufferStorage\0", PfnRenderbufferStorage),
            framebuffer_renderbuffer: load!(b"glFramebufferRenderbuffer\0", PfnFramebufferRenderbuffer),
            check_framebuffer_status: load!(b"glCheckFramebufferStatus\0", PfnCheckFramebufferStatus),
            read_pixels: load!(b"glReadPixels\0", PfnReadPixels),
            delete_framebuffers: load!(b"glDeleteFramebuffers\0", PfnDeleteFramebuffers),
            delete_renderbuffers: load!(b"glDeleteRenderbuffers\0", PfnDeleteRenderbuffers),
        }
    }
}

// ── Public API ────────────────────────────────────────────────────────────────

/// Holds an EGL surfaceless context + offscreen FBO.
///
/// The FBO has a colour renderbuffer and a combined depth24/stencil8
/// renderbuffer — femtovg requires depth+stencil when `set_screen_target`
/// is used.  Call `fbo_id()` to get the raw GL handle to pass to
/// `renderer::OpenGl::set_screen_target`, then `read_pixels()` after
/// `canvas.flush()` to read back RGBA8 in bottom-up order.
pub struct HeadlessContext {
    libegl: *mut c_void,
    display: *mut c_void,
    context: *mut c_void,
    egl_get_proc: PfnGetProcAddress,
    gl: Gl,
    pub fbo: u32,
    color_rb: u32,
    ds_rb: u32,
    pub width: u32,
    pub height: u32,
}

// SAFETY: tests are single-threaded; the EGL context is not shared.
unsafe impl Send for HeadlessContext {}

impl HeadlessContext {
    /// Returns `None` when the environment cannot support headless EGL
    /// (e.g. no Mesa, no display server at all and `EGL_PLATFORM_SURFACELESS`
    /// is absent).  Tests using this should call `if let Some(ctx) = ...`.
    pub fn try_new(width: u32, height: u32) -> Option<Self> {
        unsafe { Self::try_new_inner(width, height) }
    }

    unsafe fn try_new_inner(width: u32, height: u32) -> Option<Self> {
        // ── Load libEGL ───────────────────────────────────────────────────────
        let libegl = unsafe {
            let name = CString::new("libEGL.so.1").unwrap();
            dlopen(name.as_ptr(), RTLD_LAZY | RTLD_GLOBAL)
        };
        if libegl.is_null() {
            eprintln!("[headless] libEGL.so.1 not available");
            return None;
        }

        let egl_init: PfnInitialize =
            unsafe { sym_req(libegl, b"eglInitialize\0") };
        let egl_choose: PfnChooseConfig =
            unsafe { sym_req(libegl, b"eglChooseConfig\0") };
        let egl_bind: PfnBindApi =
            unsafe { sym_req(libegl, b"eglBindAPI\0") };
        let egl_create_ctx: PfnCreateContext =
            unsafe { sym_req(libegl, b"eglCreateContext\0") };
        let egl_make_current: PfnMakeCurrent =
            unsafe { sym_req(libegl, b"eglMakeCurrent\0") };
        let get_proc: PfnGetProcAddress =
            unsafe { sym_req(libegl, b"eglGetProcAddress\0") };

        // ── Surfaceless display ───────────────────────────────────────────────
        let get_platform: PfnGetPlatformDisplayEXT = {
            let cname = CString::new("eglGetPlatformDisplayEXT").unwrap();
            let ptr = unsafe { get_proc(cname.as_ptr()) };
            if ptr.is_null() {
                eprintln!("[headless] eglGetPlatformDisplayEXT not found");
                unsafe { dlclose(libegl) };
                return None;
            }
            unsafe { *(&ptr as *const *mut c_void as *const PfnGetPlatformDisplayEXT) }
        };

        let display = unsafe {
            get_platform(EGL_PLATFORM_SURFACELESS_MESA, ptr::null_mut(), ptr::null())
        };
        if display.is_null() {
            eprintln!("[headless] surfaceless EGL display unavailable");
            unsafe { dlclose(libegl) };
            return None;
        }

        let mut major = 0i32;
        let mut minor = 0i32;
        if unsafe { egl_init(display, &mut major, &mut minor) } == EGL_FALSE {
            eprintln!("[headless] eglInitialize failed");
            unsafe { dlclose(libegl) };
            return None;
        }

        // ── Config ────────────────────────────────────────────────────────────
        #[rustfmt::skip]
        let attribs: [i32; 15] = [
            EGL_SURFACE_TYPE,     EGL_PBUFFER_BIT,
            EGL_RENDERABLE_TYPE,  EGL_OPENGL_BIT,
            EGL_RED_SIZE,   8,
            EGL_GREEN_SIZE, 8,
            EGL_BLUE_SIZE,  8,
            EGL_ALPHA_SIZE, 8,
            EGL_DEPTH_SIZE, 0,
            EGL_NONE_I,
        ];
        let mut config: *mut c_void = ptr::null_mut();
        let mut nconfigs = 0i32;
        if unsafe {
            egl_choose(display, attribs.as_ptr(), &mut config, 1, &mut nconfigs)
        } == EGL_FALSE || nconfigs == 0
        {
            eprintln!("[headless] eglChooseConfig: no matching config");
            unsafe { dlclose(libegl) };
            return None;
        }

        // ── Context ───────────────────────────────────────────────────────────
        unsafe { egl_bind(EGL_OPENGL_API) };
        let ctx_attribs: [i32; 5] = [
            EGL_CONTEXT_MAJOR_VERSION, 3,
            EGL_CONTEXT_MINOR_VERSION, 3,
            EGL_NONE_I,
        ];
        let context = unsafe {
            egl_create_ctx(display, config, ptr::null_mut(), ctx_attribs.as_ptr())
        };
        if context.is_null() {
            eprintln!("[headless] eglCreateContext failed");
            unsafe { dlclose(libegl) };
            return None;
        }

        // Make current surfaceless (draw=EGL_NO_SURFACE, read=EGL_NO_SURFACE)
        if unsafe {
            egl_make_current(display, ptr::null_mut(), ptr::null_mut(), context)
        } == EGL_FALSE
        {
            eprintln!("[headless] eglMakeCurrent (surfaceless) failed");
            unsafe { dlclose(libegl) };
            return None;
        }

        // ── Load GL functions (EGL context is current) ──────────────────────
        let gl = unsafe { Gl::load(get_proc) };

        // ── FBO: colour + depth24/stencil8 ───────────────────────────────────
        let mut fbo = 0u32;
        let mut color_rb = 0u32;
        let mut ds_rb = 0u32;
        unsafe {
            (gl.gen_framebuffers)(1, &mut fbo);
            (gl.bind_framebuffer)(GL_FRAMEBUFFER, fbo);

            (gl.gen_renderbuffers)(1, &mut color_rb);
            (gl.bind_renderbuffer)(GL_RENDERBUFFER, color_rb);
            (gl.renderbuffer_storage)(GL_RENDERBUFFER, GL_RGBA8, width as i32, height as i32);
            (gl.framebuffer_renderbuffer)(
                GL_FRAMEBUFFER, GL_COLOR_ATTACHMENT0, GL_RENDERBUFFER, color_rb,
            );

            (gl.gen_renderbuffers)(1, &mut ds_rb);
            (gl.bind_renderbuffer)(GL_RENDERBUFFER, ds_rb);
            (gl.renderbuffer_storage)(GL_RENDERBUFFER, GL_DEPTH24_STENCIL8, width as i32, height as i32);
            (gl.framebuffer_renderbuffer)(
                GL_FRAMEBUFFER, GL_DEPTH_STENCIL_ATTACHMENT, GL_RENDERBUFFER, ds_rb,
            );
        }

        let status = unsafe { (gl.check_framebuffer_status)(GL_FRAMEBUFFER) };
        if status != GL_FRAMEBUFFER_COMPLETE {
            eprintln!("[headless] FBO incomplete: 0x{status:x}");
            unsafe { dlclose(libegl) };
            return None;
        }

        Some(Self { libegl, display, context, egl_get_proc: get_proc, gl, fbo, color_rb, ds_rb, width, height })
    }

    /// A `get_proc_address` closure for `femtovg::renderer::OpenGl::new_from_function_cstr`.
    pub fn get_proc(&self, name: &CStr) -> *const c_void {
        unsafe { (self.egl_get_proc)(name.as_ptr()) as *const _ }
    }

    /// Read pixels from the FBO colour attachment as bottom-up RGBA8.
    /// Call this after `canvas.flush()` with the FBO bound as screen target.
    pub fn read_pixels(&self) -> Vec<u8> {
        let n = (self.width * self.height * 4) as usize;
        let mut buf = vec![0u8; n];
        unsafe {
            (self.gl.bind_framebuffer)(GL_FRAMEBUFFER, self.fbo);
            (self.gl.read_pixels)(
                0, 0, self.width as i32, self.height as i32,
                GL_RGBA, GL_UNSIGNED_BYTE,
                buf.as_mut_ptr() as *mut c_void,
            );
        }
        buf
    }

    /// Get RGBA pixel at (x, y) top-left origin (flips GL bottom-up).
    pub fn pixel_at(&self, x: u32, y: u32) -> [u8; 4] {
        let raw = self.read_pixels();
        let row = (self.height - 1 - y) as usize;
        let off = (row * self.width as usize + x as usize) * 4;
        [raw[off], raw[off + 1], raw[off + 2], raw[off + 3]]
    }
}

impl Drop for HeadlessContext {
    fn drop(&mut self) {
        unsafe {
            (self.gl.delete_framebuffers)(1, &self.fbo);
            (self.gl.delete_renderbuffers)(1, &self.color_rb);
            (self.gl.delete_renderbuffers)(1, &self.ds_rb);
            let make_current: PfnMakeCurrent =
                sym_req(self.libegl, b"eglMakeCurrent\0");
            make_current(self.display, ptr::null_mut(), ptr::null_mut(), ptr::null_mut());
            let destroy: PfnDestroyContext =
                sym_req(self.libegl, b"eglDestroyContext\0");
            destroy(self.display, self.context);
            let terminate: PfnTerminate =
                sym_req(self.libegl, b"eglTerminate\0");
            terminate(self.display);
            dlclose(self.libegl);
        }
    }
}
