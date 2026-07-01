use crate::sys::SyscallEnv;
use crate::ipc::MessagePayload;

use core::sync::atomic::{AtomicU32, Ordering};

static NEXT_WINDOW_ID: AtomicU32 = AtomicU32::new(1);

pub struct WindowHandle {
    pub id: u32,
}

pub fn create_window(env: &mut SyscallEnv, x: i32, y: i32, w: i32, h: i32, title: &str) -> Option<WindowHandle> {
    if let Some(wm_pid) = env.lookup_service("wm") {
        let id = NEXT_WINDOW_ID.fetch_add(1, Ordering::SeqCst);
        env.send_msg(wm_pid, MessagePayload::CreateWindow { id, x, y, w, h, title: title.to_string(), owner: env.pid });
        Some(WindowHandle { id })
    } else {
        crate::log("[API Error] Window Manager service not found.");
        None
    }
}
