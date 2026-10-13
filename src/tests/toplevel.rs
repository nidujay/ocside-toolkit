use crate::{
    backend::headless::HeadlessBackend,
    runtime::runtime_init,
    win32::{
        winuser::{create_window, destroy_window, register_class},
        wndproc::WndProcHandle,
    },
};

#[test]
fn create_and_destroy_toplvel_window() {
    let backend = Box::new(HeadlessBackend::new());
    let handle = backend.get_handle();
    let _ = runtime_init(backend);

    assert_eq!(
        register_class("ClassName".to_string(), WndProcHandle(1)).is_ok(),
        true
    );

    let hwnd = create_window("ClassName", "Title");
    assert!(hwnd.is_ok());
    assert_eq!(handle.borrow().window_count(), 1);

    let hwnd = hwnd.unwrap();
    assert!(destroy_window(hwnd).is_ok());
    assert_eq!(handle.borrow().window_count(), 0);
}
