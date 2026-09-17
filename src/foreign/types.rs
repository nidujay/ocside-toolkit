// SPDX-License-Identifier: LGPL-3.0-or-later
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

use std::ffi::c_void;

pub type BOOL = i32;
pub type BYTE = u8;
pub type WORD = u16;
pub type DWORD = u32;
pub type LONG = i32;
pub type ULONG = u32;
pub type UINT = u32;
pub type INT = i32;
pub const FALSE: BOOL = 0;
pub const TRUE: BOOL = 1;

pub type c_wchar = i32;
pub type CHAR = i8;
pub type WCHAR = c_wchar;

pub type LPSTR = *mut CHAR;
pub type LPCSTR = *const CHAR;

pub type LPWSTR = *mut WCHAR;
pub type LPCWSTR = *const WCHAR;
pub type PWSTR = *const WCHAR;

pub type LPVOID = *mut c_void;
pub type LPCVOID = *const c_void;

pub type HANDLE = *mut c_void;
pub type HWND = HANDLE;
pub type HINSTANCE = HANDLE;
pub type HMENU = HANDLE;
pub type HDC = HANDLE;
pub type UINT_PTR = usize;
pub type WPARAM = WORD;
pub type ATOM = WORD;
pub type LONG_PTR = usize;
pub type LRESULT = LONG_PTR;
pub type LPARAM = LONG_PTR;
pub type HICON = HANDLE;
pub type HCURSOR = HANDLE;
pub type HBRUSH = HANDLE;
