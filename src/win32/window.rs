// SPDX-License-Identifier: LGPL-3.0-or-later
use crate::{backend::BackendWindowHandle, win32::wndproc::WndProcHandle};

pub enum Window {
    Placeholder,
    Complete {
        wndproc: WndProcHandle,
        backend_handle: BackendWindowHandle,
    },
}
