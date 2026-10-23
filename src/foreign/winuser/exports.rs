use crate::foreign::convert::handles::{gethwnd_from_hwnd, handle_from_hwnd, hwnd_from_handle};
use crate::foreign::convert::strings::string_from_lpcwstr;
use crate::foreign::types::*;
use crate::foreign::winuser::structs::*;
use crate::win32::error::{CaptureError, OcsideError};
use crate::win32::winuser::{
    create_window, def_window_proc, destroy_window, dispatch_message, get_message,
    post_quit_message, register_class,
};
use crate::win32::wndproc::WndProcHandle;
use crate::winmsg::{LParam, Msg, MsgType, WParam};

#[unsafe(no_mangle)]
pub unsafe extern "C" fn RegisterClassW(lpWndClass: *const WNDCLASSW) -> ATOM {
    register_class_impl(lpWndClass).last_error().unwrap_or(0)
}

fn register_class_impl(lpWndClass: *const WNDCLASSW) -> Result<ATOM, OcsideError> {
    if lpWndClass.is_null() {
        Err(OcsideError::InvalidParam)
    } else {
        let wc = unsafe { &(*lpWndClass) };
        let name = string_from_lpcwstr(wc.lpszClassName)?;
        let wndproc = wc.lpfnWndProc.ok_or(OcsideError::InvalidParam)?;
        let wndproc = WndProcHandle(wndproc as usize);
        register_class(name, wndproc)
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
    create_window_impl(lpClassName, lpWindowName)
        .last_error()
        .unwrap_or(0 as HWND)
}

fn create_window_impl(lpClassName: LPCWSTR, lpWindowName: LPCWSTR) -> Result<HWND, OcsideError> {
    let class = string_from_lpcwstr(lpClassName)?;
    let title = string_from_lpcwstr(lpWindowName)?;
    let hwnd = create_window(&class, &title)?;
    Ok(hwnd_from_handle(hwnd))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ShowWindow(hWnd: HWND, nCmdShow: INT) -> BOOL {
    // TODO
    FALSE
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn DestroyWindow(hWnd: HWND) -> BOOL {
    let hwnd = handle_from_hwnd(hWnd);
    let mut wndproc = ClientWndProc {};
    destroy_window(hwnd, &mut wndproc)
        .last_error()
        .is_ok()
        .into()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn DefWindowProcW(
    hWnd: HWND,
    Msg: UINT,
    wParam: WPARAM,
    lParam: LPARAM,
) -> LRESULT {
    let mut wndproc = ClientWndProc {};
    match def_window_proc(handle_from_hwnd(hWnd), MsgType(Msg), &mut wndproc).last_error() {
        Ok(result) => result.0,
        Err(_) => 0,
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn GetMessageW(
    lpMsg: LPMSG,
    hWnd: HWND,
    wMsgFilterMin: UINT,
    wMsgFilterMax: UINT,
) -> BOOL {
    get_msg_impl(lpMsg, hWnd, wMsgFilterMin, wMsgFilterMax)
        .last_error()
        .unwrap_or(FALSE)
}

fn get_msg_impl(
    lpMsg: LPMSG,
    hWnd: HWND,
    wMsgFilterMin: UINT,
    wMsgFilterMax: UINT,
) -> Result<BOOL, OcsideError> {
    let hwnd = gethwnd_from_hwnd(hWnd);
    let msg = get_message(hwnd)?;
    match msg {
        Some(msg) => {
            let msg = MSG::new(msg.hwnd(), msg.msg_type().0);
            unsafe {
                *lpMsg = msg;
            };
            Ok(TRUE)
        }
        None => Ok(FALSE),
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn TranslateMessage(lpMsg: LPMSG) -> BOOL {
    // TODO
    FALSE
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn DispatchMessageW(lpMsg: LPMSG) -> LRESULT {
    dispatch_message_impl(lpMsg)
}

fn dispatch_message_impl(lpMsg: LPMSG) -> LRESULT {
    if !lpMsg.is_null() {
        let (raw_hwnd, msg, l_param, w_param) = (
            unsafe { (*lpMsg).hwnd },
            unsafe { (*lpMsg).message },
            unsafe { (*lpMsg).lParam },
            unsafe { (*lpMsg).wParam },
        );

        let hwnd = if raw_hwnd.is_null() {
            None
        } else {
            Some(handle_from_hwnd(raw_hwnd))
        };
        let msg = Msg::new(hwnd, MsgType(msg), WParam(w_param), LParam(l_param));
        let wndproc = ClientWndProc {};
        match dispatch_message(msg, wndproc) {
            Ok(r) => r.0,
            Err(_) => 0,
        }
    } else {
        0
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn PostQuitMessage(nExitCode: INT) {
    let _ = post_quit_message(nExitCode as u16).last_error();
}
