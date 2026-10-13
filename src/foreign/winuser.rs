// SPDX-License-Identifier: LGPL-3.0-or-later
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(unused_variables)]

use slotmap::{Key, KeyData};

use crate::{
    foreign::types::*,
    runtime::Handle,
    win32::{
        error::{CaptureError, OcsideError},
        winuser::{
            GetHwnd, WndProcHook, create_window, def_window_proc, destroy_window, dispatch_message,
            get_message, post_quit_message, register_class,
        },
        wndproc::WndProcHandle,
    },
    winmsg::{LParam, Msg, MsgType, WParam},
};

struct Lpcwstr(pub LPCWSTR);

impl TryFrom<Lpcwstr> for String {
    type Error = OcsideError;

    fn try_from(value: Lpcwstr) -> Result<Self, Self::Error> {
        let value = value.0;
        if value.is_null() {
            Err(OcsideError::InvalidParam)
        } else {
            let mut s = String::new();

            for i in 0.. {
                let c = unsafe { *value.add(i) } as u32;
                let codept = char::from_u32(c).ok_or(OcsideError::InvalidString)?;
                if c == 0 {
                    break;
                }
                s.push(codept);
            }
            Ok(s)
        }
    }
}

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

pub type WNDPROC =
    Option<unsafe extern "C" fn(hwnd: HWND, msg: UINT, wParam: WPARAM, lParam: LPARAM) -> LRESULT>;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn RegisterClassW(lpWndClass: *const WNDCLASSW) -> ATOM {
    if lpWndClass.is_null() {
        Err(OcsideError::InvalidParam)
    } else {
        ffi_register_class(unsafe { &(*lpWndClass) })
    }
    .last_error()
    .unwrap_or(0)
}

fn ffi_register_class(wc: &WNDCLASSW) -> Result<ATOM, OcsideError> {
    let name = String::try_from(Lpcwstr(wc.lpszClassName))?;
    let wndproc = wc.lpfnWndProc.ok_or(OcsideError::InvalidParam)?;
    let wndproc = WndProcHandle(wndproc as usize);

    register_class(name, wndproc)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn DefWindowProcW(
    hWnd: HWND,
    Msg: UINT,
    wParam: WPARAM,
    lParam: LPARAM,
) -> LRESULT {
    def_window_proc(ffi_to_hwnd(hWnd), MsgType(Msg));
    0
}

pub const CW_USEDEFAULT: UINT = 0x80000000;

pub const WS_OVERLAPPED: DWORD = 0x00000000;
pub const WS_CAPTION: DWORD = 0x00C00000;
pub const WS_SYSMENU: DWORD = 0x00080000;
pub const WS_THICKFRAME: DWORD = 0x00040000;
pub const WS_MINIMIZEBOX: DWORD = 0x00020000;
pub const WS_MAXIMIZEBOX: DWORD = 0x00010000;
pub const WS_OVERLAPPEDWINDOW: DWORD =
    WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_THICKFRAME | WS_MINIMIZEBOX | WS_MAXIMIZEBOX;

fn ffi_to_hwnd(hwnd: HWND) -> Handle {
    let raw_u64 = hwnd as usize as u64;
    Handle::from(KeyData::from_ffi(raw_u64))
}

impl From<HWND> for GetHwnd {
    fn from(hwnd: HWND) -> GetHwnd {
        if hwnd.is_null() {
            GetHwnd::AnyOnCurrentThread
        } else if hwnd == -1i32 as HWND {
            GetHwnd::CurrentThread
        } else {
            GetHwnd::Window(ffi_to_hwnd(hwnd))
        }
    }
}

impl From<Handle> for HWND {
    fn from(hwnd: Handle) -> HWND {
        let raw_64: u64 = hwnd.data().as_ffi();
        raw_64 as usize as HWND
    }
}

impl From<HWND> for Handle {
    fn from(value: HWND) -> Self {
        ffi_to_hwnd(value)
    }
}

impl From<MSG> for Msg {
    fn from(msg: MSG) -> Msg {
        Msg::new(
            if msg.hwnd.is_null() {
                None
            } else {
                Some(ffi_to_hwnd(msg.hwnd))
            },
            MsgType(msg.message),
            WParam(msg.wParam),
            LParam(msg.lParam),
        )
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn CreateWindowExW(
    dwExStyle: DWORD,
    lpClassName: LPCWSTR,
    lpWindowName: LPCWSTR,
    dwStyle: DWORD,
    x: INT,
    y: INT,
    nWidth: INT,
    nHeight: INT,
    hWndParent: HWND,
    hMenu: HMENU,
    hInstance: HINSTANCE,
    lpParam: LPVOID,
) -> HWND {
    match ffi_create_window(lpClassName, lpWindowName).last_error() {
        Ok(hwnd) => hwnd,
        Err(_) => 0 as HWND,
    }
}

fn ffi_create_window(lpClassName: LPCWSTR, lpWindowName: LPCWSTR) -> Result<HWND, OcsideError> {
    let class = String::try_from(Lpcwstr(lpClassName))?;
    let title = String::try_from(Lpcwstr(lpWindowName))?;
    let hwnd = create_window(&class, &title)?;
    Ok(HWND::from(hwnd))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn DestroyWindow(hWnd: HWND) -> BOOL {
    let hwnd = hWnd.into();
    destroy_window(hwnd).last_error().is_ok().into()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ShowWindow(hWnd: HWND, nCmdShow: INT) -> BOOL {
    // TODO
    FALSE
}

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
                hwnd.into()
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

#[unsafe(no_mangle)]
pub unsafe extern "C" fn GetMessageW(
    lpMsg: LPMSG,
    hWnd: HWND,
    wMsgFilterMin: UINT,
    wMsgFilterMax: UINT,
) -> BOOL {
    let hwnd = hWnd.into();
    match get_message(hwnd).last_error() {
        Ok(msg) => match msg {
            Some(msg) => {
                let msg = MSG::new(msg.hwnd(), msg.msg_type().0);
                unsafe {
                    *lpMsg = msg;
                };
                TRUE
            }
            None => FALSE,
        },
        Err(_) => FALSE,
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn TranslateMessage(lpMsg: LPMSG) -> BOOL {
    // TODO
    FALSE
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn DispatchMessageW(lpMsg: LPMSG) -> LRESULT {
    if lpMsg.is_null() == false {
        let (raw_hwnd, msg, l_param, w_param) = (
            unsafe { (*lpMsg).hwnd },
            unsafe { (*lpMsg).message },
            unsafe { (*lpMsg).lParam },
            unsafe { (*lpMsg).wParam },
        );

        let hwnd = if raw_hwnd.is_null() {
            None
        } else {
            Some(ffi_to_hwnd(raw_hwnd))
        };
        let msg = Msg::new(hwnd, MsgType(msg), WParam(w_param), LParam(l_param));
        let wndproc = ClientWndProc {};
        dispatch_message(msg, wndproc);
    }
    0
}

struct ClientWndProc;
impl WndProcHook for ClientWndProc {
    fn invoke(&self, wndproc: WndProcHandle, hwnd: Handle, msg: MsgType, wparam: WParam) {
        let raw_addr = wndproc.0;
        let maybe_ptr: WNDPROC = unsafe { std::mem::transmute(raw_addr) };
        let wndproc = maybe_ptr.unwrap(); // Non null guaranteed

        unsafe {
            wndproc(hwnd.into(), msg.0, wparam.0, 0);
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn PostQuitMessage(nExitCode: INT) {
    post_quit_message(nExitCode as u16);
}
