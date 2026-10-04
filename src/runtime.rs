// SPDX-License-Identifier: LGPL-3.0-or-later
use mio::unix::SourceFd;
use mio::{Interest, Token};
use nix::sys::eventfd::{EfdFlags, EventFd};
use slotmap::{Key, KeyData, SlotMap, new_key_type};
use std::cell::OnceCell;
use std::{
    cell::RefCell,
    collections::HashMap,
    os::fd::{AsRawFd, RawFd},
    sync::mpsc::{Receiver, Sender, channel},
};

use crate::backend::Backend;
use crate::win32::{window::Window, wndproc::WndProcHandle};
use crate::winmsg::Msg;

new_key_type! {pub struct Handle;}

#[derive(Debug)]
pub enum EventType {
    Backend = 1,
    MsgQueue = 2,
}

impl From<EventType> for Token {
    fn from(s: EventType) -> Self {
        Token(s as usize)
    }
}

impl From<Token> for EventType {
    fn from(value: Token) -> Self {
        const BACKEND: usize = EventType::Backend as usize;
        const MSGQUEUE: usize = EventType::MsgQueue as usize;

        match value.0 {
            BACKEND => EventType::Backend,
            MSGQUEUE => EventType::MsgQueue,
            _ => panic!("Internal error: Unexpected event token."),
        }
    }
}

thread_local! {
    pub static BACKEND: OnceCell<RefCell<Box<dyn Backend>>> = OnceCell::new();
    pub static MSG_QUEUE: RefCell<MsgQueue> = RefCell::new(MsgQueue::new());
    pub static WINDOW_CLASSES: RefCell<HashMap<String, WndProcHandle>> = RefCell::new(HashMap::new());
    pub static WINDOWS: RefCell<SlotMap<Handle, Window>> = RefCell::new(SlotMap::with_key());
    pub static POLL: RefCell<mio::Poll> = RefCell::new(mio::Poll::new().expect("Failed to create event loop!"));
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

pub fn runtime_init(backend: Box<dyn Backend>) {
    // Setup event handles that we'll be polling
    // within the thread
    POLL.with_borrow_mut(|poll| {
        let mut source = SourceFd(&backend.get_fd());
        let token = EventType::Backend.into();
        let interests = Interest::READABLE;
        poll.registry().register(&mut source, token, interests);

        let mut source = SourceFd(&MSG_QUEUE.with_borrow(|mq| mq.raw_fd()));
        let token = EventType::MsgQueue.into();
        poll.registry().register(&mut source, token, interests);
    });

    // Store the backend for later use
    BACKEND.with(|b| b.set(RefCell::new(backend)));
}
