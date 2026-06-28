use std::env;
use std::fs::File;
use std::io::{BufRead, BufReader};

#[link(wasm_import_module = "env")]
unsafe extern "C" {
    fn sys_execve(args_ptr: *const u8, args_len: usize, cwd_ptr: *const u8, cwd_len: usize) -> i32;
}

fn call_sys_execve(args_buf: &[u8], cwd: &str) -> i32 {
    unsafe { sys_execve(args_buf.as_ptr(), args_buf.len(), cwd.as_ptr(), cwd.len()) }
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: sh <script.sh>");
        return;
    }

    let script_path = &args[1];
    let file = match File::open(script_path) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("sh: {}: {}", script_path, e);
            return;
        }
    };

    let cwd = env::var("PWD").unwrap_or_else(|_| "/".to_string());

    let reader = BufReader::new(file);
    for line in reader.lines() {
        if let Ok(line) = line {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.is_empty() {
                continue;
            }

            let program = parts[0];
            let bin_path = format!("/bin/{}", program);

            let mut args_buf = Vec::new();
            args_buf.extend_from_slice(bin_path.as_bytes());
            args_buf.push(0);
            for arg in parts.iter().skip(1) {
                args_buf.extend_from_slice(arg.as_bytes());
                args_buf.push(0);
            }

            let ret = call_sys_execve(&args_buf, &cwd);
            if ret != 0 {
                eprintln!("{}: command not found", program);
            }
        }
    }
}
