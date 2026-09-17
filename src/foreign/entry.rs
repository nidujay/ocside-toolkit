// SPDX-License-Identifier: LGPL-3.0-or-later
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(unused_variables)]

use std::ffi::{c_char, c_int};

use crate::{
    foreign::types::{HINSTANCE, PWSTR},
    runtime::runtime_init,
};

unsafe extern "system" {
    fn wWinMain(
        hInstance: HINSTANCE,
        hPrevInstance: HINSTANCE,
        pCmdLine: PWSTR,
        nCmdShow: c_int,
    ) -> c_int;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn main(argc: c_int, argv: *mut *mut c_char) -> c_int {
    let hInstance = 1 as HINSTANCE;
    let hPrevInstance = 0 as HINSTANCE;
    let pCmdLine = &[0, 0, 0] as PWSTR;

    runtime_init();

    unsafe { wWinMain(hInstance, hPrevInstance, pCmdLine, 1) }
}
