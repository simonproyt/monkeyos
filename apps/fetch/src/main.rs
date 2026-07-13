#[link(wasm_import_module = "env")]
unsafe extern "C" {
    fn sys_fetch(url_ptr: *const u8, url_len: usize, out_ptr: *mut u8, out_max_len: usize) -> i32;
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: fetch <url>");
        std::process::exit(1);
    }
    
    let url = &args[1];
    let mut out_buf = vec![0u8; 1024 * 1024]; // 1MB max buffer
    
    let res = unsafe {
        sys_fetch(url.as_ptr(), url.len(), out_buf.as_mut_ptr(), out_buf.len())
    };
    
    if res < 0 {
        eprintln!("fetch: failed to retrieve data from {}", url);
        std::process::exit(1);
    }
    
    let out_len = res as usize;
    if out_len <= out_buf.len() {
        let content = &out_buf[..out_len];
        if let Ok(s) = std::str::from_utf8(content) {
            print!("{}", s);
        } else {
            // If not UTF-8, write raw bytes
            use std::io::Write;
            let _ = std::io::stdout().write_all(content);
        }
    } else {
        eprintln!("fetch error: buffer too small");
    }
}
