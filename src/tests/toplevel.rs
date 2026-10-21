use crate::{
    backend::{BackendEvent, headless::HeadlessBackend},
    runtime::{Handle, runtime_init},
    win32::{
        winuser::{
            GetHwnd, LResult, WndProcHook, create_window, def_window_proc, destroy_window,
            get_message, register_class,
        },
        wndproc::WndProcHandle,
    },
    winmsg::{MsgType, WM_CLOSE, WM_DESTROY, WParam},
};

#[derive(Default)]
struct TestWndProc {
    msgs: Vec<(Handle, MsgType, WParam)>,
}

impl WndProcHook for TestWndProc {
    fn invoke(&mut self, _: WndProcHandle, hwnd: Handle, msg: MsgType, wparam: WParam) -> LResult {
        self.msgs.push((hwnd, msg, wparam));
        LResult(1)
    }
}

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
    let mut wndproc = TestWndProc::default();
    assert!(destroy_window(hwnd, &mut wndproc).is_ok());
    assert_eq!(handle.borrow().window_count(), 0);
}

#[test]
fn min_window_create_and_close() {
    let backend = Box::new(HeadlessBackend::new());
    let h_backend = backend.get_handle();
    let _ = runtime_init(backend);

    assert_eq!(
        register_class("ClassName".to_string(), WndProcHandle(1)).is_ok(),
        true
    );

    let hwnd = create_window("ClassName", "Title").unwrap();
    let hbkwnd = h_backend.borrow().find("Title").unwrap();

    // Close Window (user action)
    h_backend.borrow_mut().post(hbkwnd, BackendEvent::Close);

    let msg = get_message(GetHwnd::AnyOnCurrentThread).unwrap().unwrap();
    assert_eq!(msg.hwnd, Some(hwnd));
    assert_eq!(msg.msg, MsgType(WM_CLOSE));

    // pass message to the default window procedure (which destroys it)
    let mut wndproc = TestWndProc::default();
    assert!(def_window_proc(hwnd, msg.msg, &mut wndproc).is_ok());
    assert_eq!(wndproc.msgs, vec![(hwnd, MsgType(WM_DESTROY), WParam(0))]);

    assert_eq!(h_backend.borrow().window_count(), 0);
}
