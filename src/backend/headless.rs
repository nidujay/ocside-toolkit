use std::{cell::RefCell, os::fd::AsRawFd, rc::Rc};

use nix::sys::eventfd::{EfdFlags, EventFd};
use slotmap::SlotMap;

use crate::backend::{Backend, BackendWindowHandle};

pub struct HeadlessWindow {
    title: String,
    handler: Box<dyn Fn(super::BackendEvent)>,
}

pub struct HeadlessState {
    windows: SlotMap<BackendWindowHandle, HeadlessWindow>,
}

pub struct HeadlessBackend {
    event_fd: EventFd,
    state: Rc<RefCell<HeadlessState>>,
}

impl HeadlessBackend {
    pub fn new() -> Self {
        let state = Rc::new(RefCell::new(HeadlessState {
            windows: SlotMap::with_key(),
        }));

        Self {
            event_fd: EventFd::from_flags(EfdFlags::EFD_NONBLOCK)
                .expect("Failed to create Message Queue"),
            state: state,
        }
    }

    pub fn get_handle(&self) -> Rc<RefCell<HeadlessState>> {
        self.state.clone()
    }
}

impl Backend for HeadlessBackend {
    fn create_window(
        &mut self,
        title: &str,
        handler: Box<dyn Fn(super::BackendEvent)>,
    ) -> Option<super::BackendWindowHandle> {
        Some(self.state.borrow_mut().windows.insert(HeadlessWindow {
            title: title.to_string(),
            handler,
        }))
    }

    fn destroy_window(&mut self, handle: BackendWindowHandle) {
        self.state.borrow_mut().windows.remove(handle);
    }

    fn process(&mut self) -> Result<(), super::BackendError> {
        todo!()
    }

    fn get_fd(&self) -> std::os::unix::prelude::RawFd {
        self.event_fd.as_raw_fd()
    }

    fn paint(&self, _handle: super::BackendWindowHandle) {}
}

impl HeadlessState {
    pub fn window_count(&self) -> usize {
        self.windows.len()
    }

    pub fn find(&self, title: &str) -> Option<BackendWindowHandle> {
        let x = self.windows.iter().find(|kv| kv.1.title == title);
        x.and_then(|kv| Some(kv.0))
    }

    pub fn post(&mut self, handle: BackendWindowHandle, event: super::BackendEvent) {
        let x = self.windows.get_mut(handle).unwrap();
        (x.handler)(event);
    }
}
