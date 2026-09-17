// SPDX-License-Identifier: LGPL-3.0-or-later
use mio::unix::SourceFd;
use mio::{Interest, Token};
use nix::sys::eventfd::{EfdFlags, EventFd};
use slotmap::{Key, KeyData, SecondaryMap, SlotMap, new_key_type};
use std::{
    cell::RefCell,
    collections::HashMap,
    os::fd::{AsRawFd, RawFd},
    sync::{
        LazyLock, Mutex,
        mpsc::{Receiver, Sender, channel},
    },
    thread::{self, ThreadId},
};

use crate::backend::PlatformWindow;
use crate::win32::{window::Window, wndproc::WndProcHandle};
use crate::winmsg::Msg;

new_key_type! {pub struct Handle;}

// Handles for Windows (thread affine) and Msg queues (one per thread)
pub static HANDLES: LazyLock<Mutex<SlotMap<Handle, Resource>>> =
    LazyLock::new(|| Mutex::new(SlotMap::with_key()));

thread_local! {
    pub static MSG_QUEUE: RefCell<MsgQueue> = RefCell::new(MsgQueue::new());
    pub static WINDOW_CLASSES: RefCell<HashMap<String, WndProcHandle>> = RefCell::new(HashMap::new());
    pub static HANDLE_TYPES: RefCell<SecondaryMap<Handle, ResourceType>> = RefCell::new(SecondaryMap::new());
    pub static WINDOWS: RefCell<SecondaryMap<Handle, Window>> = RefCell::new(SecondaryMap::new());
    pub static PLATORM_WINDOWS: RefCell<SecondaryMap<Handle, PlatformWindow>> = RefCell::new(SecondaryMap::new());
    pub static POLL: RefCell<mio::Poll> = RefCell::new(mio::Poll::new().expect("Failed to create event loop!"));
}

#[derive(Debug, PartialEq)]
pub struct Resource {
    thread: ThreadId,
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum ResourceType {
    Window,
    EventQueue,
}

impl Resource {
    pub fn new() -> Self {
        Self {
            thread: thread::current().id(),
        }
    }
}

pub struct MsgQueue {
    local_sender: Sender<Msg>,
    receiver: Receiver<Msg>,
    event_fd: EventFd,
}

impl MsgQueue {
    pub fn new() -> Self {
        let (tx, rx) = channel();
        Self {
            local_sender: tx,
            receiver: rx,
            event_fd: EventFd::from_flags(EfdFlags::EFD_NONBLOCK)
                .expect("Failed to create Message Queue"),
        }
    }

    pub fn clone_sender(&self) -> Sender<Msg> {
        self.local_sender.clone()
    }

    pub fn recv(&mut self) -> Option<Msg> {
        self.receiver.try_recv().ok()
    }

    pub fn raw_fd(&self) -> RawFd {
        self.event_fd.as_raw_fd()
    }
}

impl From<Handle> for Token {
    fn from(handle: Handle) -> Self {
        Token(handle.data().as_ffi() as usize)
    }
}

impl From<Token> for Handle {
    fn from(token: Token) -> Self {
        Handle::from(KeyData::from_ffi(token.0 as u64))
    }
}

pub fn runtime_init() {
    // Get a handle for the event queue
    if let Ok(mut handles) = HANDLES.lock() {
        let h = handles.insert(Resource::new());
        POLL.with_borrow_mut(|poll| {
            MSG_QUEUE.with_borrow(|mq| {
                let mut source = SourceFd(&mq.raw_fd());
                if poll
                    .registry()
                    .register(&mut source, h.into(), Interest::READABLE)
                    .is_err()
                {
                    todo!("Either panic (main thread) or return error when creating thread");
                }
            })
        });
        HANDLE_TYPES.with_borrow_mut(|ht| ht.insert(h, ResourceType::EventQueue));
    }
}
