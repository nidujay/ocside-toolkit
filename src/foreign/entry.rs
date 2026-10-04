// SPDX-License-Identifier: LGPL-3.0-or-later
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(unused_variables)]

use std::ffi::{c_char, c_int};

use crate::{
    backend::wayland::WaylandBackend,
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

    let backend = Box::new(WaylandBackend::new().expect("msg"));
    runtime_init(backend);

    unsafe { wWinMain(hInstance, hPrevInstance, pCmdLine, 1) }
}
