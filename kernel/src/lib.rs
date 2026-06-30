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

                    let wm_pid = self.pm.spawn(|pid| Box::new(WindowManager::new(pid, display_pid)));
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

    pub fn push_key_event(&mut self, key_code: u32) {
        if let Some(pid) = self.input_pid {
            self.ipc.send(Message {
                sender: 0,
                receiver: pid,
                payload: MessagePayload::KeyPress { key_code },
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
pub unsafe extern "C" fn kernel_push_mouse_button(kernel: *mut Kernel, down: bool) {
    let k = unsafe { &mut *kernel };
    k.push_mouse_button(down);
}

/// # Safety
/// The `kernel` pointer must be a valid, aligned, non-null pointer to a `Kernel` instance
/// previously allocated by `kernel_new()`.
#[no_mangle]
pub unsafe extern "C" fn kernel_push_key_event(kernel: *mut Kernel, key_code: u32) {
    let k = unsafe { &mut *kernel };
    k.push_key_event(key_code);
}

#[no_mangle]
pub extern "C" fn sys_fd_write(fd: u32, iovs_ptr: u32, iovs_len: u32, nwritten_ptr: u32) -> u32 {
    crate::wasi::handle_fd_write(fd, iovs_ptr, iovs_len, nwritten_ptr)
}

#[no_mangle]
pub extern "C" fn sys_fd_read(_fd: u32, _iovs_ptr: u32, _iovs_len: u32, _nread_ptr: u32) -> u32 {
    0
}
