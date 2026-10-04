// SPDX-License-Identifier: MIT AND LGPL-3.0-or-later
//
// This module is heavily based on the simple_window example in Smithay-client-toolkit
// version 0.21.1, which is MIT licensed

use crate::backend::{Backend, BackendEvent, BackendWindowHandle};
use slotmap::SlotMap;
use smithay_client_toolkit::{
    activation::{ActivationHandler, ActivationState, RequestData},
    compositor::{CompositorHandler, CompositorState, FrameCallbackData},
    delegate_dispatch2, delegate_registry,
    output::{OutputHandler, OutputState},
    reexports::{
        calloop::{self, EventLoop, InsertError, LoopHandle},
        calloop_wayland_source::WaylandSource,
    },
    registry::{ProvidesRegistryState, RegistryState},
    registry_handlers,
    seat::{
        Capability, SeatHandler, SeatState,
        keyboard::{KeyEvent, KeyboardHandler, Keysym, Modifiers, RawModifiers},
        pointer::{PointerEvent, PointerEventKind, PointerHandler},
    },
    shell::{
        WaylandSurface,
        xdg::{
            XdgShell,
            window::{Window, WindowConfigure, WindowDecorations, WindowHandler},
        },
    },
    shm::{
        Shm, ShmHandler,
        slot::{Buffer, SlotPool},
    },
};
use std::{
    cell::RefCell,
    collections::HashMap,
    os::fd::{AsFd, AsRawFd, RawFd},
};
use thiserror::Error;
use wayland_client::{
    ConnectError, Connection, Proxy, QueueHandle,
    backend::ObjectId,
    globals::{BindError, GlobalError, GlobalList, registry_queue_init},
    protocol::{wl_keyboard, wl_output, wl_pointer, wl_seat, wl_shm, wl_surface},
};

thread_local! {
    static WINDOWS: RefCell<SlotMap<BackendWindowHandle, WaylandWindow>> = RefCell::new(SlotMap::with_key());
    static HANDLES: RefCell<HashMap<ObjectId, BackendWindowHandle>>=RefCell::new(HashMap::new());
}

pub struct WaylandConnection {
    loop_handle: LoopHandle<'static, WaylandConnection>,
    shm: Shm,
    seat_state: SeatState,
    registry_state: RegistryState,
    output_state: OutputState,
    xdg_activation: Option<ActivationState>,

    keyboard: Option<wl_keyboard::WlKeyboard>,
    pointer: Option<wl_pointer::WlPointer>,
}

pub struct WaylandBackend {
    conn: Connection,
    wc: WaylandConnection,
    event_loop: EventLoop<'static, WaylandConnection>,

    globals: GlobalList,
    qh: QueueHandle<WaylandConnection>,

    compositor: CompositorState,
    xdg_shell: XdgShell,
}

struct WaylandWindow {
    callback: Box<dyn Fn(BackendEvent)>,
    width: u32,
    height: u32,
    configured: bool,
    buffer: Option<Buffer>,
    window: Window,
    pool: SlotPool,
}

impl WaylandWindow {
    fn paint(&mut self, qh: QueueHandle<WaylandConnection>) {
        let width = self.width;
        let height = self.height;
        let stride = self.width as i32 * 4;

        let buffer = self.buffer.get_or_insert_with(|| {
            self.pool
                .create_buffer(
                    width as i32,
                    height as i32,
                    stride,
                    wl_shm::Format::Argb8888,
                )
                .expect("create buffer")
                .0
        });

        let canvas = match self.pool.canvas(buffer) {
            Some(canvas) => canvas,
            None => {
                // This should be rare, but if the compositor has not released the previous
                // buffer, we need double-buffering.
                let (second_buffer, canvas) = self
                    .pool
                    .create_buffer(
                        self.width as i32,
                        self.height as i32,
                        stride,
                        wl_shm::Format::Argb8888,
                    )
                    .expect("create buffer");
                *buffer = second_buffer;
                canvas
            }
        };

        // Draw to the window:
        // TODO: this is actually the clients job. Get the buffer as a mut array to be
        // passed to a client callback.
        {
            canvas.chunks_exact_mut(4).for_each(|chunk| {
                let a = 0xFF;
                let r = 0xff;
                let g = 0xff;
                let b = 0xff;
                let color: u32 = (a << 24) + (r << 16) + (g << 8) + b;

                let array: &mut [u8; 4] = chunk.try_into().unwrap();
                *array = color.to_le_bytes();
            });
        }

        // Damage the entire window
        self.window
            .wl_surface()
            .damage_buffer(0, 0, self.width as i32, self.height as i32);

        // Request our next frame
        self.window
            .wl_surface()
            .frame(&qh, FrameCallbackData(self.window.wl_surface().clone()));

        // Attach and commit to present.
        buffer
            .attach_to(self.window.wl_surface())
            .expect("buffer attach");
        self.window.commit();
    }
}

#[derive(Debug, Error)]
pub enum WaylandError {
    #[error("Connection failed")]
    Connect(#[from] ConnectError),
    #[error("Failed to obtain globals")]
    Init(#[from] GlobalError),
    #[error("Failed to setup event loop")]
    EventLoop(calloop::Error),
    #[error("Failed to setup Wayland source")]
    Source(InsertError<WaylandSource<WaylandConnection>>),
    #[error("Compositor not available")]
    NoCompositor(BindError),
    #[error("XDG Shell not available")]
    NoShell(BindError),
    #[error("SHM not available")]
    NoSharedMem(BindError),
}

impl WaylandBackend {
    pub fn new() -> Result<Self, WaylandError> {
        let conn = Connection::connect_to_env()?;

        let (globals, event_queue) =
            registry_queue_init(&conn).map_err(|e| WaylandError::Init(e))?;
        let qh = event_queue.handle();
        let event_loop: EventLoop<WaylandConnection> =
            EventLoop::try_new().map_err(|e| WaylandError::EventLoop(e))?;
        let loop_handle = event_loop.handle();

        WaylandSource::new(conn.clone(), event_queue)
            .insert(loop_handle)
            .map_err(|e| WaylandError::Source(e))?;

        let compositor =
            CompositorState::bind(&globals, &qh).map_err(|e| WaylandError::NoCompositor(e))?;
        let xdg_shell = XdgShell::bind(&globals, &qh).map_err(|e| WaylandError::NoShell(e))?;
        let shm = Shm::bind(&globals, &qh).map_err(|e| WaylandError::NoSharedMem(e))?;
        let xdg_activation = ActivationState::bind(&globals, &qh).ok();
        let loop_handle = event_loop.handle();

        println!("Backend raw FD={:?}", conn.as_fd().as_raw_fd());
        Ok(Self {
            conn: conn,
            wc: WaylandConnection {
                loop_handle: loop_handle,
                shm: shm,
                seat_state: SeatState::new(&globals, &qh),
                registry_state: RegistryState::new(&globals),
                output_state: OutputState::new(&globals, &qh),
                xdg_activation: xdg_activation,
                keyboard: None,
                pointer: None,
            },
            event_loop: event_loop,

            globals: globals,
            qh: qh,

            compositor: compositor,
            xdg_shell: xdg_shell,
        })
    }
}

impl Backend for WaylandBackend {
    fn create_window(
        &mut self,
        title: &str,
        handler: Box<dyn Fn(BackendEvent)>,
    ) -> Option<BackendWindowHandle> {
        let surface = self.compositor.create_surface(&self.qh);
        let window =
            self.xdg_shell
                .create_window(surface, WindowDecorations::RequestServer, &self.qh);
        let obj_id = window.wl_surface().id();
        window.set_title(title);
        window.set_app_id("io.github.smithay.client-toolkit.SimpleWindow");
        window.set_min_size(Some((256, 256)));
        window.commit();

        // To request focus, we first need to request a token
        if let Some(activation) = self.wc.xdg_activation.as_ref() {
            activation.request_token(
                &self.qh,
                RequestData {
                    seat_and_serial: None,
                    surface: Some(window.wl_surface().clone()),
                    app_id: Some(String::from(
                        "io.github.smithay.client-toolkit.SimpleWindow",
                    )),
                    udata: (),
                },
            )
        }

        // We don't know how large the window will be yet, so lets assume the minimum size we suggested for the
        // initial memory allocation.
        let pool = SlotPool::new(256 * 256 * 4, &self.wc.shm).ok()?;

        let key = WINDOWS.with_borrow_mut(|windows| {
            windows.insert_with_key(|_| WaylandWindow {
                callback: handler,
                width: 256,
                height: 256,
                configured: false,
                buffer: None,
                window: window,
                pool: pool,
            })
        });

        HANDLES.with_borrow_mut(|handles| handles.insert(obj_id, key.clone()));
        self.conn.flush().ok()?;
        Some(key)
    }

    fn process(&mut self) {
        self.event_loop.dispatch(None, &mut self.wc);
    }

    fn get_fd(&self) -> RawFd {
        self.conn.as_fd().as_raw_fd()
    }

    fn paint(&self, handle: BackendWindowHandle) {
        let qh = self.qh.clone();
        WINDOWS.with_borrow_mut(|windows| {
            let window = windows
                .get_mut(handle)
                .expect("Internal Error: Invalid Handle");
            window.paint(qh);
        });
        println!("Paint requested");
    }
}

impl ShmHandler for WaylandConnection {
    fn shm_state(&mut self) -> &mut Shm {
        &mut self.shm
    }
}

impl CompositorHandler for WaylandConnection {
    fn scale_factor_changed(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &wl_surface::WlSurface,
        _new_factor: i32,
    ) {
        // Not needed for this example.
    }

    fn transform_changed(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &wl_surface::WlSurface,
        _new_transform: wl_output::Transform,
    ) {
        // Not needed for this example.
    }

    fn frame(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<Self>,
        _surface: &wl_surface::WlSurface,
        _time: u32,
    ) {
        //self.draw(conn, qh);
    }

    fn surface_enter(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &wl_surface::WlSurface,
        _output: &wl_output::WlOutput,
    ) {
        // Not needed for this example.
    }

    fn surface_leave(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &wl_surface::WlSurface,
        _output: &wl_output::WlOutput,
    ) {
        // Not needed for this example.
    }
}

impl WindowHandler for WaylandConnection {
    fn request_close(&mut self, _: &Connection, _: &QueueHandle<Self>, window: &Window) {
        let id = window.wl_surface().id();
        let handle = HANDLES.with_borrow(|handles| {
            handles
                .get(&id)
                .expect("Internal error: No callback registered")
                .clone()
        });

        WINDOWS.with_borrow_mut(|windows| {
            let window = windows
                .get_mut(handle)
                .expect("Internal error: Invalid handle");
            (window.callback)(BackendEvent::Close);
        });
    }

    fn configure(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        window: &Window,
        configure: WindowConfigure,
        _serial: u32,
    ) {
        let id = window.wl_surface().id();
        let handle = HANDLES.with_borrow(|handles| {
            handles
                .get(&id)
                .expect("Internal error: No callback registered")
                .clone()
        });

        WINDOWS.with_borrow_mut(|windows| {
            let window = windows
                .get_mut(handle)
                .expect("Internal error: Invalid handle");
            window.buffer = None;
            window.width = configure.new_size.0.map(|v| v.get()).unwrap_or(256);
            window.height = configure.new_size.1.map(|v| v.get()).unwrap_or(256);
            if !window.configured {
                window.configured = true;
                (window.callback)(BackendEvent::WindowReady);
            }
        });
    }
}

impl KeyboardHandler for WaylandConnection {
    fn enter(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_keyboard::WlKeyboard,
        _surface: &wl_surface::WlSurface,
        _: u32,
        _: &[u32],
        _keysyms: &[Keysym],
    ) {
        /*
        if self.window.wl_surface() == surface {
            println!("Keyboard focus on window with pressed syms: {keysyms:?}");
            self.keyboard_focus = true;
        }
        */
    }

    fn leave(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_keyboard::WlKeyboard,
        _surface: &wl_surface::WlSurface,
        _: u32,
    ) {

        // if self.window.wl_surface() == surface {
        //     println!("Release keyboard focus on window");
        //     self.keyboard_focus = false;
        // }
    }

    fn press_key(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _: &wl_keyboard::WlKeyboard,
        _: u32,
        event: KeyEvent,
    ) {
        println!("Key press: {event:?}");
    }

    fn repeat_key(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_keyboard::WlKeyboard,
        _: u32,
        event: KeyEvent,
    ) {
        println!("Key repeat: {event:?}");
    }

    fn release_key(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_keyboard::WlKeyboard,
        _: u32,
        event: KeyEvent,
    ) {
        println!("Key release: {event:?}");
    }

    fn update_modifiers(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_keyboard::WlKeyboard,
        _serial: u32,
        modifiers: Modifiers,
        _raw_modifiers: RawModifiers,
        _layout: u32,
    ) {
        println!("Update modifiers: {modifiers:?}");
    }
}

impl ActivationHandler for WaylandConnection {
    type RequestUdata = ();

    fn new_token(&mut self, token: String, _data: &RequestData<()>) {
        println!("Activation handler");
        /*
        self.xdg_activation
            .as_ref()
            .unwrap()
            .activate::<WaylandConnection>(self.window.wl_surface(), token);
        */
    }
}

impl SeatHandler for WaylandConnection {
    fn seat_state(&mut self) -> &mut SeatState {
        &mut self.seat_state
    }

    fn new_seat(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_seat::WlSeat) {}

    fn new_capability(
        &mut self,
        _conn: &Connection,
        qh: &QueueHandle<Self>,
        seat: wl_seat::WlSeat,
        capability: Capability,
    ) {
        if capability == Capability::Keyboard && self.keyboard.is_none() {
            println!("Set keyboard capability");
            let keyboard = self
                .seat_state
                .get_keyboard_with_repeat(
                    qh,
                    &seat,
                    None,
                    self.loop_handle.clone(),
                    Box::new(|_state, _wl_kbd, event| {
                        println!("Repeat: {:?} ", event);
                    }),
                )
                .expect("Failed to create keyboard");

            self.keyboard = Some(keyboard);
        }

        if capability == Capability::Pointer && self.pointer.is_none() {
            println!("Set pointer capability");
            let pointer = self
                .seat_state
                .get_pointer(qh, &seat)
                .expect("Failed to create pointer");
            self.pointer = Some(pointer);
        }
    }

    fn remove_capability(
        &mut self,
        _conn: &Connection,
        _: &QueueHandle<Self>,
        _: wl_seat::WlSeat,
        capability: Capability,
    ) {
        if capability == Capability::Keyboard && self.keyboard.is_some() {
            println!("Unset keyboard capability");
            self.keyboard.take().unwrap().release();
        }

        if capability == Capability::Pointer && self.pointer.is_some() {
            println!("Unset pointer capability");
            self.pointer.take().unwrap().release();
        }
    }

    fn remove_seat(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_seat::WlSeat) {}
}

impl PointerHandler for WaylandConnection {
    fn pointer_frame(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _pointer: &wl_pointer::WlPointer,
        events: &[PointerEvent],
    ) {
        use PointerEventKind::*;
        for event in events {
            // Ignore events for other surfaces
            // if &event.surface != self.window.wl_surface() {
            //     continue;
            // }

            match event.kind {
                Enter { .. } => {
                    println!("Pointer entered @{:?}", event.position);
                }
                Leave { .. } => {
                    println!("Pointer left");
                }
                Motion { .. } => {}
                Press { button, .. } => {
                    println!("Press {:x} @ {:?}", button, event.position);
                }
                Release { button, .. } => {
                    println!("Release {:x} @ {:?}", button, event.position);
                }
                Axis {
                    horizontal,
                    vertical,
                    ..
                } => {
                    println!("Scroll H:{horizontal:?}, V:{vertical:?}");
                }
            }
        }
    }
}

impl OutputHandler for WaylandConnection {
    fn output_state(&mut self) -> &mut OutputState {
        &mut self.output_state
    }

    fn new_output(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _output: wl_output::WlOutput,
    ) {
    }

    fn update_output(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _output: wl_output::WlOutput,
    ) {
    }

    fn output_destroyed(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _output: wl_output::WlOutput,
    ) {
    }
}

impl ProvidesRegistryState for WaylandConnection {
    fn registry(&mut self) -> &mut RegistryState {
        &mut self.registry_state
    }
    registry_handlers![OutputState, SeatState,];
}

delegate_registry!(WaylandConnection);
delegate_dispatch2!(WaylandConnection);
