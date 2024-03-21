use std::{mem, ptr};

use windows::Win32::Graphics::{
    Gdi::{GetDC, ReleaseDC, HDC},
    OpenGL::{
        wglCreateContext, wglMakeCurrent, ChoosePixelFormat, DescribePixelFormat, SetPixelFormat,
        HGLRC, PFD_DRAW_TO_WINDOW, PFD_SUPPORT_OPENGL, PFD_TYPE_RGBA, PIXELFORMATDESCRIPTOR,
    },
};

use crate::window::Window;

pub mod render;

pub struct Graphics<'w> {
    window: &'w Window,
    drawing_context: HDC,
    gl_context: HGLRC,
}

impl<'w> Graphics<'w> {
    pub fn new(window: &'w Window) -> Self {
        let drawing_context = unsafe { GetDC(**window) };

        let mut pixel_format_desc = PIXELFORMATDESCRIPTOR {
            nSize: mem::size_of::<PIXELFORMATDESCRIPTOR>() as _,
            nVersion: 1,
            dwFlags: PFD_DRAW_TO_WINDOW | PFD_SUPPORT_OPENGL,
            iPixelType: PFD_TYPE_RGBA,
            cColorBits: 32,
            ..Default::default()
        };

        let pixel_format = unsafe { ChoosePixelFormat(drawing_context, &pixel_format_desc) };

        unsafe {
            SetPixelFormat(drawing_context, pixel_format, &pixel_format_desc).unwrap();

            DescribePixelFormat(
                drawing_context,
                pixel_format,
                pixel_format_desc.nSize as _,
                Some(&mut pixel_format_desc as *mut _),
            );
        }

        let gl_context = unsafe { wglCreateContext(drawing_context) }.unwrap();
        unsafe { wglMakeCurrent(drawing_context, gl_context) }.unwrap();

        Self {
            window,
            drawing_context,
            gl_context,
        }
    }

    pub fn invalidate_swapchain(&self) {}

    pub fn prepare_frame(&self, mut cb: impl FnMut(&Self)) {
        cb(self);
    }
}

impl Drop for Graphics<'_> {
    fn drop(&mut self) {
        unsafe {
            _ = wglMakeCurrent(HDC::default(), HGLRC::default());
            ReleaseDC(**self.window, self.drawing_context)
        };
    }
}
