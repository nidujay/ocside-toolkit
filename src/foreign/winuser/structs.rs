use crate::{
    foreign::{convert::handles::hwnd_from_handle, types::*},
    runtime::Handle,
    win32::{
        winuser::{LResult, WndProcHook},
        wndproc::WndProcHandle,
    },
    winmsg::{MsgType, WParam},
};

#[repr(C)]
pub struct WNDCLASSW {
    pub style: UINT,
    pub lpfnWndProc: WNDPROC,
    pub cbClsExtra: INT,
    pub cbWndExtra: INT,
    pub hInstance: HINSTANCE,
    pub hIcon: HICON,
    pub hCursor: HCURSOR,
    pub hbrBackground: HBRUSH,
    pub lpszMenuName: LPCWSTR,
    pub lpszClassName: LPCWSTR,
}

pub type PWNDCLASSW = *mut WNDCLASSW;
pub type NPWNDCLASSW = *mut WNDCLASSW;
pub type LPWNDCLASSW = *mut WNDCLASSW;

#[repr(C)]
pub struct POINT {
    pub x: LONG,
    pub y: LONG,
}

pub type PPOINT = *mut POINT;
pub type NPPOINT = *mut POINT;
pub type LPPOINT = *mut POINT;

#[repr(C)]
pub struct MSG {
    pub hwnd: HWND,
    pub message: UINT,
    pub wParam: WPARAM,
    pub lParam: LPARAM,
    pub time: DWORD,
    pub pt: POINT,
    pub lPrivate: DWORD,
}

impl MSG {
    pub fn new(hwnd: Option<Handle>, msg: UINT) -> Self {
        Self {
            hwnd: if let Some(hwnd) = hwnd {
                hwnd_from_handle(hwnd)
            } else {
                0 as HANDLE
            },
            message: msg,
            wParam: 0,
            lParam: 0,
            time: 0,
            pt: POINT { x: 0, y: 0 },
            lPrivate: 0,
        }
    }
}

pub type PMSG = *mut MSG;
pub type NPMSG = *mut MSG;
pub type LPMSG = *mut MSG;

pub struct ClientWndProc;
impl WndProcHook for ClientWndProc {
    fn invoke(
        &mut self,
        wndproc: WndProcHandle,
        hwnd: Handle,
        msg: MsgType,
        wparam: WParam,
    ) -> LResult {
        let raw_addr = wndproc.0;
        let maybe_ptr: WNDPROC = unsafe { std::mem::transmute(raw_addr) };
        let wndproc = maybe_ptr.unwrap(); // Non null guaranteed

        unsafe { LResult(wndproc(hwnd_from_handle(hwnd), msg.0, wparam.0, 0)) }
    }
}
