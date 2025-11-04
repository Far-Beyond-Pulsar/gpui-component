#![cfg(target_os = "windows")]
use windows_sys::Win32::{Foundation::*, Graphics::Gdi::*, UI::WindowsAndMessaging::*};

pub unsafe fn create_window(w: i32, h: i32, title: &str) -> HWND {
    let class_name: Vec<u16> = title.encode_utf16().chain(core::iter::once(0)).collect();
    let wc = WNDCLASSW { style: 0, lpfnWndProc: Some(def_wndproc), hInstance: GetModuleHandleW(core::ptr::null()), lpszClassName: class_name.as_ptr(), ..core::mem::zeroed() };
    RegisterClassW(&wc);
    let hwnd = CreateWindowExW(0, class_name.as_ptr(), class_name.as_ptr(), WS_OVERLAPPEDWINDOW|WS_VISIBLE, CW_USEDEFAULT, CW_USEDEFAULT, w, h, 0, 0, wc.hInstance, core::ptr::null());
    hwnd
}

extern "system" fn def_wndproc(hwnd: HWND, msg: u32, w: WPARAM, l: LPARAM) -> LRESULT {
    unsafe {
        match msg { WM_DESTROY => { PostQuitMessage(0); 0 }, _ => DefWindowProcW(hwnd, msg, w, l) }
    }
}

pub unsafe fn pump_messages() -> bool { let mut m: MSG = core::mem::zeroed(); while PeekMessageW(&mut m, 0, 0, 0, PM_REMOVE) != 0 { if m.message == WM_QUIT { return false; } TranslateMessage(&m); DispatchMessageW(&m);} true }