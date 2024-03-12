use core::{
    cell::Cell,
    ffi::CStr,
    ptr,
    sync::atomic::{AtomicBool, Ordering},
};

use windows::{
    core::{s, Result, PCSTR},
    Win32::{
        Foundation::{HINSTANCE, HWND, LPARAM, LRESULT, WPARAM},
        System::LibraryLoader::GetModuleHandleA,
        UI::WindowsAndMessaging::{
            CreateWindowExA, DefWindowProcA, DestroyWindow, DispatchMessageA, GetClientRect,
            GetWindowLongPtrA, LoadCursorW, PeekMessageA, PostQuitMessage, RegisterClassA,
            SetWindowLongPtrA, CS_HREDRAW, CS_VREDRAW, CW_USEDEFAULT, GWLP_USERDATA, IDC_ARROW,
            MSG, PM_REMOVE, WINDOW_EX_STYLE, WM_CLOSE, WM_DESTROY, WM_QUIT, WNDCLASSA,
            WS_OVERLAPPEDWINDOW, WS_VISIBLE,
        },
    },
};

use crate::{
    event::{Event, Input},
    utils,
};

pub struct Window {
    inner: HWND,
    instance: HINSTANCE,
    callback: Cell<*const *mut dyn FnMut(Event)>,
    used: AtomicBool,
}

utils::wrap! { Window, HWND }

impl Window {
    pub fn new(title: &CStr, width: u32, height: u32) -> Result<Self> {
        // jmi2k: TODO: null & error checks everywhere!
        // jmi2k: TODO: use AdjustWindowRectEx to calculate outer size from inner_size.

        let instance = unsafe { GetModuleHandleA(None) }?.into();

        let class = WNDCLASSA {
            hCursor: unsafe { LoadCursorW(None, IDC_ARROW) }?,
            hInstance: instance,
            lpszClassName: s!("window"),
            style: CS_HREDRAW | CS_VREDRAW,
            lpfnWndProc: Some(handle_event),
            ..Default::default()
        };

        unsafe { RegisterClassA(&class) };

        let inner = unsafe {
            CreateWindowExA(
                WINDOW_EX_STYLE::default(),
                class.lpszClassName,
                PCSTR::from_raw(title.as_ptr() as _),
                WS_OVERLAPPEDWINDOW | WS_VISIBLE,
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                width as _,
                height as _,
                None,
                None,
                instance,
                None,
            )
        };

        Ok(Self {
            instance,
            inner,
            callback: Cell::new(ptr::null()),
            used: false.into(),
        })
    }

    pub fn instance(&self) -> HINSTANCE {
        self.instance
    }

    pub fn inner_size(&self) -> [u32; 2] {
        let mut rect = Default::default();

        _ = unsafe { GetClientRect(**self, &mut rect) };

        let width = rect.right - rect.left;
        let height = rect.bottom - rect.top;

        [width as _, height as _]
    }

    pub fn run(&self, mut cb: impl FnMut(Event)) {
        if self.used.fetch_or(true, Ordering::AcqRel) {
            return;
        }

        let mut message = MSG::default();
        let fat_pointer = &mut cb as *mut dyn FnMut(_);

        self.callback.set(&fat_pointer as *const _ as _);
        unsafe { SetWindowLongPtrA(**self, GWLP_USERDATA, self as *const _ as _) };

        while message.message != WM_QUIT {
            if !unsafe { PeekMessageA(&mut message, None, 0, 0, PM_REMOVE) }.as_bool() {
                cb(Event::Idle);
                continue;
            }

            unsafe { DispatchMessageA(&message) };
        }
    }

    fn callback(&self) -> *const *mut dyn FnMut(Event) {
        self.callback.get()
    }
}

unsafe extern "system" fn handle_event(
    handle: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    let window = GetWindowLongPtrA(handle, GWLP_USERDATA) as *const Window;
    let callback = window.as_ref().map(Window::callback);

    match (callback, message) {
        (Some(cb), WM_CLOSE) => {
            let event = Event::Input(Input::Close);

            (**cb)(event);
            _ = DestroyWindow(handle);

            LRESULT::default()
        }

        (_, WM_DESTROY) => {
            PostQuitMessage(0);
            LRESULT::default()
        }

        _ => DefWindowProcA(handle, message, wparam, lparam),
    }
}
