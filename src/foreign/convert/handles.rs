use crate::{foreign::types::HWND, runtime::Handle, win32::winuser::GetHwnd};
use slotmap::{Key, KeyData};

pub fn hwnd_from_handle(hwnd: Handle) -> HWND {
    let raw_64: u64 = hwnd.data().as_ffi();
    raw_64 as usize as HWND
}

pub fn handle_from_hwnd(hwnd: HWND) -> Handle {
    let raw_u64 = hwnd as usize as u64;
    Handle::from(KeyData::from_ffi(raw_u64))
}

pub fn gethwnd_from_hwnd(hwnd: HWND) -> GetHwnd {
    if hwnd.is_null() {
        GetHwnd::AnyOnCurrentThread
    } else if hwnd == -1i32 as HWND {
        GetHwnd::CurrentThread
    } else {
        GetHwnd::Window(handle_from_hwnd(hwnd))
    }
}
