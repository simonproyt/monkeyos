use core::sync::atomic::{AtomicU32, Ordering};
use crate::process::ProcessId;

// We will use this to track which terminal process is currently "active" for WASI stdout.
pub static mut CURRENT_TERMINAL_ID: Option<u32> = None;

// Provide a way to route WASI write to our terminal
pub fn handle_fd_write(fd: u32, iovs_ptr: u32, iovs_len: u32, nwritten_ptr: u32) -> u32 {
    unsafe {
        if fd != 1 && fd != 2 {
            return 8; // EBADF
        }
        
        let mut total_written = 0;
        let iovs = core::slice::from_raw_parts(iovs_ptr as *const [u32; 2], iovs_len as usize);
        
        for iov in iovs {
            let ptr = iov[0] as *const u8;
            let len = iov[1] as usize;
            let slice = core::slice::from_raw_parts(ptr as *const u8, len);
            if let Ok(text) = core::str::from_utf8(slice) {
                let id = CURRENT_TERMINAL_ID.unwrap_or(0);
                wasi_print_js(id, ptr as *const u8, len);
            }
            total_written += len;
        }
        
        *(nwritten_ptr as *mut u32) = total_written as u32;
        
        0 // Success
    }
}

#[link(wasm_import_module = "env")]
extern "C" {
    fn wasi_print_js(id: u32, ptr: *const u8, len: usize);
    fn sys_execve(args_ptr: *const u8, args_len: usize, cwd_ptr: *const u8, cwd_len: usize) -> i32;
}

pub fn call_sys_execve(args_buf: &[u8], cwd: &str) -> i32 {
    unsafe { sys_execve(args_buf.as_ptr(), args_buf.len(), cwd.as_ptr(), cwd.len()) }
}
