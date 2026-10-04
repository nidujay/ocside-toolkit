// SPDX-License-Identifier: LGPL-3.0-or-later
use crate::runtime::Handle;

#[derive(Debug, PartialEq, Clone, Copy)]
pub struct MsgType(pub u32);

#[derive(Debug, Clone, Copy)]
pub struct WParam(pub u16);

#[derive(Debug, Clone, Copy)]
pub struct LParam(pub usize);

#[derive(Debug)]
pub struct Msg {
    pub hwnd: Option<Handle>,
    pub msg: MsgType,
    pub wparam: WParam,
    pub lparam: LParam,
}

impl Msg {
    pub fn new(hwnd: Option<Handle>, msg: MsgType, wparam: WParam, lparam: LParam) -> Self {
        Self {
            hwnd,
            msg,
            wparam,
            lparam,
        }
    }

    pub fn hwnd(&self) -> Option<Handle> {
        self.hwnd
    }

    pub fn msg_type(&self) -> MsgType {
        self.msg
    }
}

pub const WM_DESTROY: u32 = 0x0002;
pub const WM_CLOSE: u32 = 0x0010;
pub const WM_QUIT: u32 = 0x0012;
pub const WM_PAINT: u32 = 0x0004;
