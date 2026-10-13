// SPDX-License-Identifier: LGPL-3.0-or-later
use crate::{backend::BackendWindowHandle, runtime::BACKEND, win32::wndproc::WndProcHandle};

pub enum Window {
    Placeholder,
    Complete {
        wndproc: WndProcHandle,
        backend_handle: BackendWindowHandle,
    },
}

impl Drop for Window {
    fn drop(&mut self) {
        match self {
            Window::Placeholder => (),
            Window::Complete {
                wndproc: _,
                backend_handle,
            } => {
                BACKEND.with(|backend| {
                    let mut backend = backend
                        .get()
                        .expect("BUG: Backend not initialised!")
                        .borrow_mut();
                    backend.destroy_window(*backend_handle);
                });
            }
        }
    }
}
