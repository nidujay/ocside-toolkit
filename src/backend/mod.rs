use std::os::fd::RawFd;

use slotmap::new_key_type;
use thiserror::Error;

pub mod wayland;

new_key_type! {pub struct BackendWindowHandle;}

#[derive(Debug)]
pub enum BackendEvent {
    WindowReady,
    Close,
}

#[derive(Debug, Error)]
#[error("Backend error")]
pub struct BackendError;

pub trait Backend {
    fn create_window(
        &mut self,
        title: &str,
        handler: Box<dyn Fn(BackendEvent)>,
    ) -> Option<BackendWindowHandle>;
    fn process(&mut self) -> Result<(), BackendError>;
    fn get_fd(&self) -> RawFd;
    fn paint(&self, handle: BackendWindowHandle);
}
