// SPDX-License-Identifier: LGPL-3.0-or-later
use crate::win32::wndproc::WndProcHandle;

pub struct Window {
    pub wndproc: WndProcHandle,
}

impl Window {
    pub fn new(wndproc: WndProcHandle) -> Option<Self> {
        Some(Self { wndproc })
    }
}
