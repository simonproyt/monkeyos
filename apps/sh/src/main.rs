use std::env;
use std::fs::File;
use std::io::{BufRead, BufReader};

#[link(wasm_import_module = "env")]
unsafe extern "C" {
    fn sys_execve(args_ptr: *const u8, args_len: usize, cwd_ptr: *const u8, cwd_len: usize, stdin_ptr: *const u8, stdin_len: usize, stdout_ptr: *const u8, stdout_len: usize) -> i32;
}

fn call_sys_execve(args_buf: &[u8], cwd: &str, stdin: Option<&str>, stdout: Option<&str>) -> i32 {
    let stdin_str = stdin.unwrap_or("");
    let stdout_str = stdout.unwrap_or("");
    unsafe { 
        sys_execve(
            args_buf.as_ptr(), args_buf.len(), 
            cwd.as_ptr(), cwd.len(),
            stdin_str.as_ptr(), stdin_str.len(),
            stdout_str.as_ptr(), stdout_str.len()
        ) 
    }
}

fn execute_command(cmd_str: &str, cwd: &str, mut stdin: Option<String>, mut stdout: Option<String>) {
    let spaced_cmd = cmd_str.replace(">", " > ").replace("<", " < ").replace("|", " | ");
    let parts: Vec<&str> = spaced_cmd.split_whitespace().collect();
    if parts.is_empty() { return; }

    // Parse > and <
    let mut filtered_parts = Vec::new();
    let mut i = 0;
    while i < parts.len() {
        if parts[i] == ">" && i + 1 < parts.len() {
            stdout = Some(parts[i+1].to_string());
            i += 2;
        } else if parts[i] == "<" && i + 1 < parts.len() {
            stdin = Some(parts[i+1].to_string());
            i += 2;
        } else {
            filtered_parts.push(parts[i]);
            i += 1;
        }
    }

    if filtered_parts.is_empty() { return; }

    let program = filtered_parts[0];
    let bin_path = format!("/bin/{}", program);

    let mut args_buf = Vec::new();
    args_buf.extend_from_slice(bin_path.as_bytes());
    args_buf.push(0);
    for arg in filtered_parts.iter().skip(1) {
        args_buf.extend_from_slice(arg.as_bytes());
        args_buf.push(0);
    }

    let ret = call_sys_execve(&args_buf, cwd, stdin.as_deref(), stdout.as_deref());
    if ret != 0 {
        eprintln!("{}: command not found", program);
    }
}

fn process_line(line: &str, cwd: &str) {
    let line = line.trim();
    if line.is_empty() || line.starts_with('#') {
        return;
    }

    let pipe_parts: Vec<&str> = line.split('|').collect();
    if pipe_parts.len() > 1 {
        for (i, part) in pipe_parts.iter().enumerate() {
            let mut p_stdin = None;
            let mut p_stdout = None;
            if i > 0 {
                p_stdin = Some(format!("/tmp/pipe_{}", i - 1));
            }
            if i < pipe_parts.len() - 1 {
                p_stdout = Some(format!("/tmp/pipe_{}", i));
            }
            execute_command(part, cwd, p_stdin, p_stdout);
        }
        // Cleanup pipe files
        for i in 0..pipe_parts.len() - 1 {
            let pipe_file = format!("/tmp/pipe_{}", i);
            let mut args_buf = Vec::new();
            args_buf.extend_from_slice(b"/bin/rm\0");
            args_buf.extend_from_slice(pipe_file.as_bytes());
            args_buf.push(0);
            let _ = call_sys_execve(&args_buf, cwd, None, None);
        }
    } else {
        execute_command(line, cwd, None, None);
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: sh <script.sh> or sh -c \"<command>\"");
        return;
    }

    let cwd = env::var("PWD").unwrap_or_else(|_| "/".to_string());

    if args[1] == "-c" {
        if args.len() >= 3 {
            process_line(&args[2], &cwd);
        }
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

    let reader = BufReader::new(file);
    for line in reader.lines() {
        if let Ok(line) = line {
            process_line(&line, &cwd);
        }
    }
}
