use std::os::fd::RawFd;

use slotmap::new_key_type;

pub mod wayland;

new_key_type! {pub struct BackendWindowHandle;}

#[derive(Debug)]
pub enum BackendEvent {
    WindowReady,
    Close,
}

pub trait Backend {
    fn create_window(
        &mut self,
        title: &str,
        handler: Box<dyn Fn(BackendEvent)>,
    ) -> Option<BackendWindowHandle>;
    fn process(&mut self);
    fn get_fd(&self) -> RawFd;
    fn paint(&self, handle: BackendWindowHandle);
}
