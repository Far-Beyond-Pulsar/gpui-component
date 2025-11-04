#![cfg(target_os = "windows")]
use windows_sys::Win32::{Foundation::*, Graphics::{Gdi::*, OpenGL::*}};

pub struct GlContext { pub hdc: HDC, pub hglrc: HGLRC }

pub unsafe fn create_gl_context(hwnd: HWND) -> GlContext {
    let hdc = GetDC(hwnd);
    let mut pfd = PIXELFORMATDESCRIPTOR { nSize: core::mem::size_of::<PIXELFORMATDESCRIPTOR>() as u16, nVersion: 1, dwFlags: PFD_DRAW_TO_WINDOW|PFD_SUPPORT_OPENGL|PFD_DOUBLEBUFFER, iPixelType: PFD_TYPE_RGBA, cColorBits: 32, cDepthBits: 24, cStencilBits: 8, iLayerType: PFD_MAIN_PLANE, ..core::mem::zeroed() };
    let pf = ChoosePixelFormat(hdc, &pfd); SetPixelFormat(hdc, pf, &pfd);
    let hglrc = wglCreateContext(hdc);
    wglMakeCurrent(hdc, hglrc);
    GlContext { hdc, hglrc }
}

pub unsafe fn gl_render(ctx: &GlContext) {
    glClearColor(0.12, 0.12, 0.14, 1.0);
    glClear(GL_COLOR_BUFFER_BIT as u32);
    SwapBuffers(ctx.hdc);
}