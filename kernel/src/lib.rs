pub mod ipc;
pub mod process;
pub mod services;
pub mod sys;
pub mod api;
pub mod wasi;

use crate::process::{ProcessManager, ProcessId};
use crate::ipc::{IpcBus, Message, MessagePayload};
use crate::services::vfs::VfsService;
use crate::services::display::DisplayServer;
use crate::services::wm::WindowManager;
use crate::sys::ServiceRegistry;

#[link(wasm_import_module = "env")]
extern "C" {
    pub fn console_log(ptr: *const u8, len: usize);
}

pub fn log(s: &str) {
    unsafe { console_log(s.as_ptr(), s.len()) }
}

pub struct Kernel {
    ticks: u32,
    state: BootState,
    pm: ProcessManager,
    ipc: IpcBus,
    registry: ServiceRegistry,
    
    // Service PIDs
    input_pid: Option<ProcessId>,
    
    // Screen size
    screen_w: i32,
    screen_h: i32,
}

enum BootState {
    Starting,
    MountingVFS,
    InitIPC,
    StartingServices,
    Running,
}

impl Kernel {
    pub fn new() -> Self {
        log("[KERNEL] Microkernel loaded into memory...");
        
        Self {
            ticks: 0,
            state: BootState::Starting,
            pm: ProcessManager::new(),
            ipc: IpcBus::new(),
            registry: ServiceRegistry::new(),
            input_pid: None,
            screen_w: 1024,
            screen_h: 768,
        }
    }
}

impl Default for Kernel {
    fn default() -> Self {
        Self::new()
    }
}

impl Kernel {
    pub fn tick(&mut self) {
        self.ticks += 1;
        
        if self.ticks.is_multiple_of(60) {
            match self.state {
                BootState::Starting => {
                    log("MonkeyOS Microkernel v0.1.0");
                    log("[ OK ] Probing virtual hardware devices");
                    self.state = BootState::MountingVFS;
                }
                BootState::MountingVFS => {
                    let pid = self.pm.spawn(|pid| Box::new(VfsService::new(pid)));
                    self.registry.register("vfs", pid);
                    log("[ OK ] Mounted Root Filesystem (/)");
                    self.state = BootState::InitIPC;
                }
                BootState::InitIPC => {
                    log("[ OK ] Initialized IPC message queues");
                    self.state = BootState::StartingServices;
                }
                BootState::StartingServices => {
                    let input_pid = self.pm.spawn(|pid| Box::new(crate::services::input::InputServer::new(pid)));
                    self.input_pid = Some(input_pid);
                    self.registry.register("input", input_pid);
                    log("[ OK ] Started Input Server (ps2_mock)");

                    let display_pid = self.pm.spawn(|pid| Box::new(DisplayServer::new(pid)));
                    self.registry.register("display", display_pid);
                    log("[ OK ] Started Display Server (WebGPU)");

                    let wm_pid = self.pm.spawn(|pid| Box::new(WindowManager::new(pid, display_pid, self.screen_w, self.screen_h)));
                    self.registry.register("wm", wm_pid);
                    log("[ OK ] Started Window Manager (kwin_wayland_mock)");
                    
                    let terminal_pid = self.pm.spawn(|pid| Box::new(crate::services::terminal::TerminalProcess::new(pid)));
                    self.registry.register("terminal", terminal_pid);
                    log("[ OK ] Started Terminal & Shell");

                    self.state = BootState::Running;
                }
                BootState::Running => {}
            }
        }

        // Run scheduler
        self.pm.tick_all(&mut self.ipc, &self.registry);

        // Process Kernel messages (PID 0)
        while let Some(msg) = self.ipc.receive(0) {
            match msg.payload {
                crate::ipc::MessagePayload::SpawnTerminal => {
                    let _terminal_pid = self.pm.spawn(|pid| Box::new(crate::services::terminal::TerminalProcess::new(pid)));
                    // Note: We don't overwrite the registry's "terminal" entry unless we want to,
                    // but it's fine for now as it's just used for looking up the default terminal.
                }
                crate::ipc::MessagePayload::SpawnProcess { bin } => {
                    // Convert bin string to null-terminated byte slice
                    let mut args_buf = Vec::new();
                    args_buf.extend_from_slice(bin.as_bytes());
                    args_buf.push(0);
                    crate::wasi::call_sys_execve(&args_buf, "/", None, None, 0);
                }
                _ => {}
            }
        }
    }

    pub fn push_mouse_move(&mut self, x: i32, y: i32) {
        if let Some(pid) = self.input_pid {
            self.ipc.send(Message {
                sender: 0,
                receiver: pid,
                payload: MessagePayload::MouseMove { x, y },
            });
        }
    }

    pub fn push_mouse_button(&mut self, down: bool) {
        if let Some(pid) = self.input_pid {
            self.ipc.send(Message {
                sender: 0,
                receiver: pid,
                payload: MessagePayload::MouseButton { down },
            });
        }
    }

    pub fn push_mouse_wheel(&mut self, delta_y: f32) {
        if let Some(pid) = self.input_pid {
            self.ipc.send(Message {
                sender: 0,
                receiver: pid,
                payload: MessagePayload::MouseWheel { delta_y },
            });
        }
    }

    pub fn push_key_event(&mut self, key_code: u32) {
        if let Some(pid) = self.input_pid {
            self.ipc.send(Message {
                sender: 0,
                receiver: pid,
                payload: MessagePayload::KeyPress { key_code },
            });
        }
    }

    pub fn push_screen_size(&mut self, w: i32, h: i32) {
        self.screen_w = w;
        self.screen_h = h;
        if let Some(pid) = self.registry.lookup("wm") {
            self.ipc.send(Message {
                sender: 0,
                receiver: pid,
                payload: MessagePayload::ScreenSizeChanged { w, h },
            });
        }
    }
}

// ---------------------------------------------------------
// FFI EXPORTS TO JAVASCRIPT
// ---------------------------------------------------------

#[no_mangle]
pub extern "C" fn kernel_new() -> *mut Kernel {
    Box::into_raw(Box::new(Kernel::new()))
}

/// # Safety
/// The `kernel` pointer must be a valid, aligned, non-null pointer to a `Kernel` instance
/// previously allocated by `kernel_new()`.
#[no_mangle]
pub unsafe extern "C" fn kernel_tick(kernel: *mut Kernel) {
    let k = unsafe { &mut *kernel };
    k.tick();
}

/// # Safety
/// The `kernel` pointer must be a valid, aligned, non-null pointer to a `Kernel` instance
/// previously allocated by `kernel_new()`.
#[no_mangle]
pub unsafe extern "C" fn kernel_push_mouse_move(kernel: *mut Kernel, x: i32, y: i32) {
    let k = unsafe { &mut *kernel };
    k.push_mouse_move(x, y);
}

/// # Safety
/// The `kernel` pointer must be a valid, aligned, non-null pointer to a `Kernel` instance
/// previously allocated by `kernel_new()`.
#[no_mangle]
pub extern "C" fn kernel_push_mouse_button(kernel: *mut Kernel, down: bool) {
    let kernel = unsafe { &mut *kernel };
    kernel.push_mouse_button(down);
}

#[no_mangle]
pub extern "C" fn kernel_push_mouse_wheel(kernel: *mut Kernel, delta_y: f32) {
    let kernel = unsafe { &mut *kernel };
    kernel.push_mouse_wheel(delta_y);
}

/// # Safety
/// The `kernel` pointer must be a valid, aligned, non-null pointer to a `Kernel` instance
/// previously allocated by `kernel_new()`.
#[no_mangle]
pub unsafe extern "C" fn kernel_push_key_event(kernel: *mut Kernel, key_code: u32) {
    let k = unsafe { &mut *kernel };
    k.push_key_event(key_code);
}

/// # Safety
/// The `kernel` pointer must be a valid, aligned, non-null pointer to a `Kernel` instance
/// previously allocated by `kernel_new()`.
#[no_mangle]
pub unsafe extern "C" fn kernel_push_screen_size(kernel: *mut Kernel, w: i32, h: i32) {
    let k = unsafe { &mut *kernel };
    k.push_screen_size(w, h);
}

#[no_mangle]
pub unsafe extern "C" fn kernel_wasi_print_char(kernel: *mut Kernel, term_id: u32, c: u32) {
    let k = unsafe { &mut *kernel };
    let term_pid = k.registry.lookup("terminal").unwrap_or(0);
    k.ipc.send(Message {
        sender: 0,
        receiver: term_pid,
        payload: MessagePayload::WasiPrintChar { id: term_id, c: c as u8 }
    });
}

#[no_mangle]
pub extern "C" fn sys_fd_write(fd: u32, iovs_ptr: u32, iovs_len: u32, nwritten_ptr: u32) -> u32 {
    crate::wasi::handle_fd_write(fd, iovs_ptr, iovs_len, nwritten_ptr)
}

#[no_mangle]
pub extern "C" fn sys_fd_read(_fd: u32, _iovs_ptr: u32, _iovs_len: u32, _nread_ptr: u32) -> u32 {
    0
}

#[no_mangle]
pub unsafe extern "C" fn kernel_create_window(kernel: *mut Kernel, x: i32, y: i32, w: i32, h: i32, app_type: u32) -> u32 {
    let k = unsafe { &mut *kernel };
    let title = match app_type {
        0 => "Terminal".to_string(),
        1 => "Calculator".to_string(),
        2 => "File Manager".to_string(),
        3 => "Notepad".to_string(),
        4 => "Image Viewer".to_string(),
        5 => "Piano".to_string(),
        6 => "MIDI Player".to_string(),
        7 => "Task Manager".to_string(),
        _ => "Window".to_string(),
    };
    
    if let Some(wm_pid) = k.registry.lookup("wm") {
        // Simple sequential ID, offset to avoid clashes with kernel windows
        static NEXT_WINDOW_ID: core::sync::atomic::AtomicU32 = core::sync::atomic::AtomicU32::new(10000);
        let id = NEXT_WINDOW_ID.fetch_add(1, core::sync::atomic::Ordering::SeqCst);
        
        k.ipc.send(crate::ipc::Message {
            sender: 0,
            receiver: wm_pid,
            payload: crate::ipc::MessagePayload::CreateWindow {
                id, x, y, w, h, title, owner: 0
            }
        });
        id
    } else {
        0
    }
}
