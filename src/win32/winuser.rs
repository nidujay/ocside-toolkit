// SPDX-License-Identifier: LGPL-3.0-or-later
use mio::Events;

use crate::{
    backend::{self, BackendEvent},
    runtime::{BACKEND, EventType, Handle, MSG_QUEUE, POLL, WINDOW_CLASSES, WINDOWS},
    win32::{window::Window, wndproc::WndProcHandle},
    winmsg::{LParam, Msg, MsgType, WM_CLOSE, WM_DESTROY, WM_PAINT, WM_QUIT, WParam},
};

pub fn register_class(name: String, wndproc: WndProcHandle) -> bool {
    WINDOW_CLASSES.with(|wc| wc.borrow_mut().insert(name, wndproc));
    true
}

struct PendingWindow {
    handle: Handle,
    moved: bool,
}

impl PendingWindow {
    fn new(handle: Handle) -> Self {
        Self {
            handle: handle,
            moved: false,
        }
    }

    fn handle(&self) -> Handle {
        self.handle.clone()
    }

    fn commit(mut self) -> Handle {
        self.moved = true;
        self.handle
    }
}

impl Drop for PendingWindow {
    fn drop(&mut self) {
        if !self.moved {
            WINDOWS.with_borrow_mut(|windows| windows.remove(self.handle));
        }
    }
}

pub fn create_window(class: &str, title: &str) -> Option<Handle> {
    // Get a copy of the windows procedure for the given class
    let wndproc = WINDOW_CLASSES.with(|wc| wc.borrow().get(class).copied())?;

    // Allocate a handle
    let handle = WINDOWS.with_borrow_mut(|windows| windows.insert(Window::Placeholder));
    let window = PendingWindow::new(handle.clone());

    // At this point we can invoke client wndproc with WM_CREATE, etc. If they return a failure,
    // the pre-allocated handle is dropped anyway

    // Now construct the backend window with the associated listener
    let sender = MSG_QUEUE.with_borrow(|mq| mq.clone_sender());

    let backend_event_listener = move |e: BackendEvent| {
        let _ = sender.send(match e {
            BackendEvent::WindowReady => Msg {
                hwnd: Some(handle),
                msg: MsgType(WM_PAINT),
                wparam: WParam(0),
                lparam: LParam(0),
            },
            BackendEvent::Close => Msg {
                hwnd: Some(handle),
                msg: MsgType(WM_CLOSE),
                wparam: WParam(0),
                lparam: LParam(0),
            },
        });
    };

    let backend_handle = BACKEND.with(|backend| {
        let mut backend = backend
            .get()
            .expect("Internal Error: Backend not initialised!")
            .borrow_mut();
        backend.create_window(title, Box::new(backend_event_listener))
    })?;

    // Now replace the Placeholder with the real window
    let handle = window.commit();
    let window = Window::Complete {
        wndproc,
        backend_handle,
    };

    WINDOWS.with_borrow_mut(|windows| {
        windows.detach(handle);
        windows.reattach(handle, window);
    });
    Some(handle)
}

pub enum GetHwnd {
    Window(Handle),
    AnyOnCurrentThread,
    CurrentThread,
}

pub fn get_message(from: GetHwnd) -> Option<Msg> {
    match from {
        GetHwnd::AnyOnCurrentThread => get_message_from_any(),
        _ => todo!(),
    }
}

fn get_message_from_any() -> Option<Msg> {
    loop {
        if let Some(msg) = MSG_QUEUE.with_borrow_mut(|mq| mq.recv()) {
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
                let event_type: EventType = event.token().into();

                match event_type {
                    EventType::Backend => BACKEND.with(|backend| {
                        let mut backend = backend.get().expect("").borrow_mut();
                        backend.process();
                    }),
                    EventType::MsgQueue => todo!(),
                }
            }
        });
    }
}

pub trait WndProcHook {
    fn invoke(&self, wndproc: WndProcHandle, hwnd: Handle, msg: MsgType, wparam: WParam);
}

pub fn dispatch_message(msg: Msg, hook: impl WndProcHook) {
    if let Some(hwnd) = msg.hwnd {
        let wndproc = WINDOWS.with_borrow(|windows| {
            let window = windows.get(hwnd);
            if let Some(window) = window {
                match window {
                    Window::Placeholder => todo!(),
                    Window::Complete {
                        wndproc,
                        backend_handle: _,
                    } => *wndproc,
                }
            } else {
                todo!();
            }
        });
        hook.invoke(wndproc, hwnd, msg.msg, msg.wparam);
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
        PostHwnd::CurrentThread => sender.send(Msg::new(None, msg, wparam, lparam)).is_ok(),
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
        WM_PAINT => paint(hwnd),
        _ => todo!(),
    }
}

fn paint(hwnd: Handle) {
    let window = WINDOWS.with_borrow_mut(|windows| {
        let window = windows.get(hwnd).unwrap();
        match window {
            Window::Placeholder => todo!(),
            Window::Complete {
                wndproc: _,
                backend_handle,
            } => backend_handle.clone(),
        }
    });

    BACKEND.with(|backend| {
        let backend = backend
            .get()
            .expect("Internal Error: Backend not initialised!")
            .borrow_mut();

        backend.paint(window);
    });
}
