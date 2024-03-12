use core::{
    ffi::{c_char, c_int, CStr},
    mem, ptr,
    sync::atomic::{AtomicBool, Ordering},
};

use x11::xlib::{
    self, XCheckIfEvent, XCloseDisplay, XCreateWindow, XDefaultRootWindow, XGetWindowAttributes,
    XInternAtom, XMapWindow, XOpenDisplay, XSelectInput, XSetWMProtocols, XStoreName,
};

use crate::{
    event::{Event, Input},
    utils,
};

pub struct Window {
    inner: xlib::Window,
    display: *mut xlib::Display,
    used: AtomicBool,
}

utils::wrap! { Window, xlib::Window }

impl Window {
    pub fn new(title: &CStr, width: u32, height: u32) -> Option<Self> {
        // jmi2k: TODO: null & error checks everywhere!

        let display = unsafe { XOpenDisplay(ptr::null()) };

        if display.is_null() {
            return None;
        }

        let inner = unsafe {
            XCreateWindow(
                display,
                XDefaultRootWindow(display),
                0,
                0,
                width,
                height,
                0,
                0,
                xlib::InputOutput as _,
                ptr::null_mut(),
                xlib::CWBackPixel,
                &mut mem::zeroed(),
            )
        };

        let input_mask = xlib::KeyPressMask | xlib::ExposureMask | xlib::KeyPressMask;

        unsafe {
            let mut delete = XInternAtom(display, c"WM_DELETE_WINDOW".as_ptr(), 0);

            XSetWMProtocols(display, inner, &mut delete as *mut _, 1);
            XSelectInput(display, inner, input_mask);
            XStoreName(display, inner, title.as_ptr());
            XMapWindow(display, inner);
        };

        Some(Self {
            inner,
            display,
            used: false.into(),
        })
    }

    pub fn display(&self) -> *mut xlib::Display {
        self.display
    }

    pub fn inner_size(&self) -> [u32; 2] {
        let mut attributes = unsafe { mem::zeroed() };

        unsafe { XGetWindowAttributes(self.display, **self, &mut attributes) };
        [attributes.width as _, attributes.height as _]
    }

    pub fn run(&self, mut cb: impl FnMut(Event)) {
        if self.used.fetch_or(true, Ordering::AcqRel) {
            return;
        }

        let mut event = unsafe { mem::zeroed::<xlib::XEvent>() };

        while event.get_type() != xlib::ClientMessage {
            if !unsafe { self.peek_event(&mut event) } {
                cb(Event::Idle);
                continue;
            }

            unsafe { handle_event(&event, &mut cb) };
        }
    }

    unsafe fn peek_event(&self, event: &mut xlib::XEvent) -> bool {
        extern "C" fn match_any(
            _: *mut xlib::Display,
            _: *mut xlib::XEvent,
            _: *mut c_char,
        ) -> c_int {
            1
        }

        XCheckIfEvent(self.display, event, Some(match_any), ptr::null_mut()) != 0
    }
}

impl Drop for Window {
    fn drop(&mut self) {
        unsafe { XCloseDisplay(self.display) };
    }
}

unsafe fn handle_event(raw_event: &xlib::XEvent, cb: &mut impl FnMut(Event)) {
    match raw_event.get_type() {
        xlib::ClientMessage => {
            let event = Event::Input(Input::Close);
            cb(event);
        }

        _ => {}
    }
}
