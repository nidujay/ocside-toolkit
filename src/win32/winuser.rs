// SPDX-License-Identifier: LGPL-3.0-or-later
use mio::Events;

use crate::{
    backend::BackendEvent,
    runtime::{BACKEND, EventType, Handle, MSG_QUEUE, POLL, WINDOW_CLASSES, WINDOWS},
    win32::{error::OcsideError, window::Window, wndproc::WndProcHandle},
    winmsg::{LParam, Msg, MsgType, WM_CLOSE, WM_DESTROY, WM_PAINT, WM_QUIT, WParam},
};

pub struct LResult(pub isize);

pub fn register_class(name: String, wndproc: WndProcHandle) -> Result<u16, OcsideError> {
    WINDOW_CLASSES.with(|wc| {
        let mut wc = wc.borrow_mut();
        if wc.contains_key(&name) {
            Err(OcsideError::ClassRegistered)
        } else {
            wc.insert(name, wndproc);
            Ok(1)
        }
    })
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

    fn _handle(&self) -> Handle {
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

pub fn create_window(class: &str, title: &str) -> Result<Handle, OcsideError> {
    // Get a copy of the windows procedure for the given class
    let wndproc = WINDOW_CLASSES.with(|wc| {
        wc.borrow()
            .get(class)
            .copied()
            .ok_or(OcsideError::ClassNotFound)
    })?;

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
            .expect("BUG: Backend not initialised!")
            .borrow_mut();
        backend
            .create_window(title, Box::new(backend_event_listener))
            .ok_or(OcsideError::WindowCreateFailed)
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
    Ok(handle)
}

pub fn destroy_window(hwnd: Handle, hook: &mut impl WndProcHook) -> Result<(), OcsideError> {
    let wndproc = get_wndproc(hwnd)?;
    hook.invoke(wndproc, hwnd, MsgType(WM_DESTROY), WParam(0));
    // Remove the window handle now that it's destroyed
    WINDOWS.with_borrow_mut(|windows| windows.remove(hwnd));
    Ok(())
}

pub enum GetHwnd {
    Window(Handle),
    AnyOnCurrentThread,
    CurrentThread,
}

pub fn get_message(from: GetHwnd) -> Result<Option<Msg>, OcsideError> {
    match from {
        GetHwnd::AnyOnCurrentThread => get_message_from_any(),
        _ => todo!(),
    }
}

fn get_message_from_any() -> Result<Option<Msg>, OcsideError> {
    loop {
        if let Some(msg) = MSG_QUEUE.with_borrow_mut(|mq| mq.recv()) {
            return Ok(if msg.msg_type() == MsgType(WM_QUIT) {
                None
            } else {
                Some(msg)
            });
        }

        // Nothing in the queue. Wait for Backend or external thread events
        poll()?;
    }
}

fn poll() -> Result<(), OcsideError> {
    POLL.with_borrow_mut(|poll| -> Result<(), OcsideError> {
        let mut events = Events::with_capacity(1024);
        poll.poll(&mut events, None)
            .or(Err(OcsideError::PollError))?;

        for event in &events {
            let event_type: EventType = event.token().into();

            match event_type {
                EventType::Backend => process_backend_event(),
                EventType::MsgQueue => todo!(),
            }?;
        }
        Ok(())
    })
}

fn process_backend_event() -> Result<(), OcsideError> {
    BACKEND.with(|backend| -> Result<(), OcsideError> {
        let mut backend = backend
            .get()
            .expect("BUG: Backend not initialised")
            .borrow_mut();
        backend.process()?;
        Ok(())
    })
}

pub trait WndProcHook {
    fn invoke(
        &mut self,
        wndproc: WndProcHandle,
        hwnd: Handle,
        msg: MsgType,
        wparam: WParam,
    ) -> LResult;
}

pub fn dispatch_message(msg: Msg, mut hook: impl WndProcHook) -> Result<LResult, OcsideError> {
    if let Some(hwnd) = msg.hwnd {
        let wndproc = get_wndproc(hwnd).expect("TODO: generate error");
        Ok(hook.invoke(wndproc, hwnd, msg.msg, msg.wparam))
    } else {
        Err(OcsideError::InvalidHandle)
    }
}

pub enum PostHwnd {
    Window(Handle),
    Broadcast,
    CurrentThread,
}

pub fn post_message(
    hwnd: PostHwnd,
    msg: MsgType,
    wparam: WParam,
    lparam: LParam,
) -> Result<(), OcsideError> {
    let sender = MSG_QUEUE.with_borrow(|mq| mq.clone_sender());

    match hwnd {
        PostHwnd::Window(hwnd) => Ok(sender.send(Msg::new(Some(hwnd), msg, wparam, lparam))?),
        PostHwnd::CurrentThread => Ok(sender.send(Msg::new(None, msg, wparam, lparam))?),
        _ => todo!(),
    }
}

pub fn post_quit_message(exit_code: u16) -> Result<(), OcsideError> {
    post_message(
        PostHwnd::CurrentThread,
        MsgType(WM_QUIT),
        WParam(exit_code),
        LParam(0),
    )
}

pub fn def_window_proc(
    hwnd: Handle,
    msg: MsgType,
    hook: &mut impl WndProcHook,
) -> Result<LResult, OcsideError> {
    println!("DefWindowProc: {:?} for {:?}", msg, hwnd);
    match msg.0 {
        WM_CLOSE => {
            destroy_window(hwnd, hook)?;
            Ok(LResult(0))
        }
        WM_PAINT => {
            paint(hwnd)?;
            Ok(LResult(0))
        }
        _ => todo!(),
    }
}

fn paint(hwnd: Handle) -> Result<(), OcsideError> {
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
            .expect("BUG: Backend not initialised!")
            .borrow_mut();

        backend.paint(window);
    });

    Ok(())
}

fn get_wndproc(hwnd: Handle) -> Result<WndProcHandle, OcsideError> {
    WINDOWS.with_borrow(|windows| {
        let window = windows.get(hwnd);
        if let Some(window) = window {
            match window {
                Window::Placeholder => todo!(),
                Window::Complete {
                    wndproc,
                    backend_handle: _,
                } => Ok(*wndproc),
            }
        } else {
            Err(OcsideError::InvalidHandle)
        }
    })
}
