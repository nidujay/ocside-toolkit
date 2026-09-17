// SPDX-License-Identifier: LGPL-3.0-or-later
use mio::{Events, Interest, unix::SourceFd};
use slotmap::SlotMap;
use std::{io, sync::MutexGuard};

use crate::{
    backend::{self},
    runtime::{
        HANDLE_TYPES, HANDLES, Handle, MSG_QUEUE, PLATORM_WINDOWS, POLL, Resource, ResourceType,
        WINDOW_CLASSES, WINDOWS,
    },
    win32::{window::Window, winuser::PostHwnd::CurrentThread, wndproc::WndProcHandle},
    winmsg::{LParam, Msg, MsgType, WM_CLOSE, WM_DESTROY, WM_QUIT, WParam},
};

pub fn register_class(name: String, wndproc: WndProcHandle) -> bool {
    WINDOW_CLASSES.with(|wc| wc.borrow_mut().insert(name, wndproc));
    true
}

struct StagedHandle<'a> {
    handles: MutexGuard<'a, SlotMap<Handle, Resource>>,
    handle: Handle,
    committed: bool,
}

impl<'a> StagedHandle<'a> {
    fn new(mut handles: MutexGuard<'a, SlotMap<Handle, Resource>>) -> Self {
        let handle = handles.insert(Resource::new());
        Self {
            handles,
            handle,
            committed: false,
        }
    }

    fn get(&self) -> Handle {
        self.handle
    }

    fn commit(mut self) -> Handle {
        self.committed = true;
        self.handle
    }
}

impl Drop for StagedHandle<'_> {
    fn drop(&mut self) {
        if !self.committed {
            self.handles.remove(self.handle);
        }
    }
}

pub fn create_window(class: &str, title: &str) -> Option<Handle> {
    // Get a copy of the windows procedure for the given class
    let wndproc = WINDOW_CLASSES.with(|wc| wc.borrow().get(class).copied())?;

    // Register a handle for this Window
    let sender = MSG_QUEUE.with_borrow(|mq| mq.clone_sender());
    let window = Window::new(wndproc)?;

    let handle = StagedHandle::new(HANDLES.lock().ok()?);

    let pfmwindow = backend::create_window(title, sender, handle.get())?;
    POLL.with_borrow_mut(|poll| -> io::Result<()> {
        let mut source = SourceFd(&pfmwindow.raw_fd());
        poll.registry()
            .register(&mut source, handle.get().into(), Interest::READABLE)
    })
    .ok()?;

    let handle = handle.commit();

    // Ensure fallable operations occur before this point
    HANDLE_TYPES.with_borrow_mut(|ht| ht.insert(handle, ResourceType::Window));
    PLATORM_WINDOWS.with(|w| w.borrow_mut().insert(handle, pfmwindow));
    WINDOWS.with(|w| w.borrow_mut().insert(handle, window));
    println!("Window successfully created with handle {:?}", handle);
    Some(handle)
}

pub enum GetHwnd {
    Window(Handle),
    AnyOnCurrentThread,
    CurrentThread,
}

pub fn get_message(from: GetHwnd) -> Option<Msg> {
    match from {
        GetHwnd::AnyOnCurrentThread => loop {
            if let Some(msg) = MSG_QUEUE.with_borrow_mut(|mq| mq.recv()) {
                println!("Recived msg {:?}", msg);
                return if msg.msg_type() == MsgType(WM_QUIT) {
                    None
                } else {
                    Some(msg)
                };
            }

            POLL.with_borrow_mut(|poll| {
                let mut events = Events::with_capacity(1024);
                let _ = poll.poll(&mut events, None);
                for event in &events {
                    let handle: Handle = event.token().into();
                    let htype = HANDLE_TYPES.with_borrow(|ht| *ht.get(handle).unwrap());

                    match htype {
                        ResourceType::Window => PLATORM_WINDOWS.with_borrow_mut(|w| {
                            let w = w.get_mut(handle).unwrap();
                            w.process_event();
                        }),
                        ResourceType::EventQueue => todo!(),
                    }
                }
            });
        },
        _ => todo!(),
    }
}

pub trait WndProcHook {
    fn invoke(&self, wndproc: WndProcHandle, hwnd: Handle, msg: MsgType, wparam: WParam);
}

pub fn dispatch_message(msg: Msg, hook: impl WndProcHook) {
    if let Some(hwnd) = msg.hwnd() {
        let wndproc = WINDOWS.with_borrow(|w| w.get(hwnd).unwrap().wndproc);
        hook.invoke(wndproc, hwnd, msg.msg_type(), msg.wparam);
    }
}

pub enum PostHwnd {
    Window(Handle),
    Broadcast,
    CurrentThread,
}

pub fn post_message(hwnd: PostHwnd, msg: MsgType, wparam: WParam, lparam: LParam) -> bool {
    let sender = MSG_QUEUE.with_borrow(|mq| mq.clone_sender());

    match hwnd {
        PostHwnd::Window(hwnd) => sender
            .send(Msg::new(Some(hwnd), msg, wparam, lparam))
            .is_ok(),
        CurrentThread => sender.send(Msg::new(None, msg, wparam, lparam)).is_ok(),
        _ => todo!(),
    }
}

pub fn post_quit_message(exit_code: u16) {
    post_message(
        PostHwnd::CurrentThread,
        MsgType(WM_QUIT),
        WParam(exit_code),
        LParam(0),
    );
}

pub fn def_window_proc(hwnd: Handle, msg: MsgType) {
    println!("DefWindowProc: {:?} for {:?}", msg, hwnd);
    match msg.0 {
        WM_CLOSE => {
            post_message(
                PostHwnd::Window(hwnd),
                MsgType(WM_DESTROY),
                WParam(0),
                LParam(0),
            );
        }
        _ => todo!(),
    }
}
