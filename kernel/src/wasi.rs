

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
            let slice = core::slice::from_raw_parts(ptr, len);
            if core::str::from_utf8(slice).is_ok() {
                let id = CURRENT_TERMINAL_ID.unwrap_or(0);
                wasi_print_js(id, ptr, len);
            }
            total_written += len;
        }
        
        *(nwritten_ptr as *mut u32) = total_written as u32;
        
        0 // Success
    }
}

#[link(wasm_import_module = "env")]
unsafe extern "C" {
    fn wasi_print_js(id: u32, ptr: *const u8, len: usize);
    fn sys_execve(args_ptr: *const u8, args_len: usize, cwd_ptr: *const u8, cwd_len: usize, stdin_ptr: *const u8, stdin_len: usize, stdout_ptr: *const u8, stdout_len: usize, terminal_id: u32) -> i32;
    fn sys_time_ms() -> u64;
    fn sys_timezone_offset_ms() -> i64;
    fn sys_fetch(url_ptr: *const u8, url_len: usize, out_ptr: *mut u8, out_max_len: usize) -> i32;
    fn sys_play_tone(freq: f32, duration_ms: u32, wave_type: u32);
    fn sys_schedule_note(freq: f32, duration_ms: u32, wave_type: u32, volume: f32, delay_ms: u32);
    fn sys_stop_audio();
    fn sys_get_system_info(out_ptr: *mut u8, max_len: usize) -> i32;
}

pub fn call_sys_time_ms() -> u64 {
    unsafe { sys_time_ms() }
}

pub fn call_sys_timezone_offset_ms() -> i64 {
    unsafe { sys_timezone_offset_ms() }
}

pub fn call_sys_fetch(url_ptr: *const u8, url_len: usize, out_ptr: *mut u8, out_max_len: usize) -> i32 {
    unsafe { sys_fetch(url_ptr, url_len, out_ptr, out_max_len) }
}

pub fn call_sys_play_tone(freq: f32, duration_ms: u32, wave_type: u32) {
    unsafe { sys_play_tone(freq, duration_ms, wave_type) }
}

pub fn call_sys_schedule_note(freq: f32, duration_ms: u32, wave_type: u32, volume: f32, delay_ms: u32) {
    unsafe { sys_schedule_note(freq, duration_ms, wave_type, volume, delay_ms) }
}

pub fn call_sys_stop_audio() {
    unsafe { sys_stop_audio() }
}

pub fn call_sys_execve(args_buf: &[u8], cwd: &str, stdin: Option<&str>, stdout: Option<&str>, terminal_id: u32) -> i32 {
    unsafe {
        sys_execve(
            args_buf.as_ptr(), args_buf.len(),
            cwd.as_ptr(), cwd.len(),
            stdin.map_or(std::ptr::null(), |s| s.as_ptr()), stdin.map_or(0, |s| s.len()),
            stdout.map_or(std::ptr::null(), |s| s.as_ptr()), stdout.map_or(0, |s| s.len()),
            terminal_id
        )
    }
}

pub fn print_direct(id: u32, text: &str) {
    unsafe {
        wasi_print_js(id, text.as_ptr(), text.len());
    }
}

pub fn call_sys_get_system_info() -> String {
    let mut buf = vec![0u8; 8192];
    let len = unsafe { sys_get_system_info(buf.as_mut_ptr(), buf.len()) };
    if len > 0 && len < buf.len() as i32 {
        buf.truncate(len as usize);
        String::from_utf8_lossy(&buf).into_owned()
    } else {
        String::new()
    }
}
