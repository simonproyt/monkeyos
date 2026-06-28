use crate::sys::SyscallEnv;
use crate::ipc::MessagePayload;

pub struct WindowHandle {
    // In the future this would hold a window ID
}

pub fn create_window(env: &mut SyscallEnv, x: i32, y: i32, w: i32, h: i32) -> Option<WindowHandle> {
    if let Some(wm_pid) = env.lookup_service("wm") {
        env.send_msg(wm_pid, MessagePayload::DrawRect { x, y, w, h });
        Some(WindowHandle {})
    } else {
        crate::log("[API Error] Window Manager service not found.");
        None
    }
}
