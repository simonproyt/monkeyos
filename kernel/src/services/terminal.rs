use crate::process::{Process, ProcessId};
use crate::sys::SyscallEnv;
use crate::api::window;
use crate::ipc::MessagePayload;

pub struct TerminalProcess {
    pid: ProcessId,
    launched: bool,
    window_id: Option<u32>,
    input_buffer: String,
    prompt: String,
    cursor_pos: usize,
    cwd: String,
}

impl TerminalProcess {
    pub fn new(pid: ProcessId) -> Self {
        Self { 
            pid, 
            launched: false, 
            window_id: None,
            input_buffer: String::new(),
            prompt: String::from("root@monkeyos:/# "),
            cursor_pos: 0,
            cwd: String::from("/"),
        }
    }

    fn update_prompt(&mut self) {
        self.prompt = format!("root@monkeyos:{}# ", self.cwd);
    }

    fn print(&self, env: &mut SyscallEnv, text: &str) {
        if let Some(id) = self.window_id {
            if let Some(display_pid) = env.lookup_service("display") {
                env.send_msg(display_pid, MessagePayload::AppendHtmlOverlayText { 
                    id, 
                    text: text.to_string() 
                });
            }
        }
    }

    fn redraw_input_line(&self, env: &mut SyscallEnv) {
        if let Some(id) = self.window_id {
            if let Some(display_pid) = env.lookup_service("display") {
                env.send_msg(display_pid, MessagePayload::UpdateHtmlOverlayInputLine { 
                    id, 
                    prompt: self.prompt.clone(),
                    input: self.input_buffer.clone(),
                    cursor_pos: self.cursor_pos as u32,
                });
            }
        }
    }

    fn execute_command(&mut self, env: &mut SyscallEnv) {
        let cmd = self.input_buffer.trim().to_string();
        
        // Finalize current line (remove cursor block)
        unsafe { crate::wasi::CURRENT_TERMINAL_ID = self.window_id; }
        println!();
        unsafe { crate::wasi::CURRENT_TERMINAL_ID = None; }

        if cmd.is_empty() {
            self.input_buffer.clear();
            self.cursor_pos = 0;
            self.print(env, &self.prompt);
            self.redraw_input_line(env);
            return;
        }

        let parts: Vec<&str> = cmd.split_whitespace().collect();
        let program = parts[0];

        match program {
            "help" => {
                self.print(env, "Available commands: help, clear, cd, ls, cat, echo, mkdir, rm, touch, pwd, sh\n");
                self.print(env, "Try: ls /bin\n");
            }
            "clear" => {
                if let Some(id) = &self.window_id {
                    if let Some(display_pid) = env.lookup_service("display") {
                        env.send_msg(display_pid, MessagePayload::ClearHtmlOverlayText { id: id.clone() });
                    }
                }
            }
            "cd" => {
                let mut new_dir = "/".to_string();
                if parts.len() > 1 {
                    new_dir = parts[1].to_string();
                }
                
                let base = if new_dir.starts_with('/') {
                    new_dir
                } else {
                    if self.cwd == "/" {
                        format!("/{}", new_dir)
                    } else {
                        format!("{}/{}", self.cwd, new_dir)
                    }
                };

                let mut path_parts = Vec::new();
                for part in base.split('/') {
                    if part == "" || part == "." {
                        continue;
                    } else if part == ".." {
                        path_parts.pop();
                    } else {
                        path_parts.push(part);
                    }
                }

                self.cwd = if path_parts.is_empty() {
                    "/".to_string()
                } else {
                    format!("/{}", path_parts.join("/"))
                };
                
                self.update_prompt();
            }
            _ => {
                let bin_path = format!("/bin/{}", program);
                
                // Construct the null-separated arguments buffer
                let mut args_buf = Vec::new();
                args_buf.extend_from_slice(bin_path.as_bytes());
                args_buf.push(0);
                for arg in parts.iter().skip(1) {
                    args_buf.extend_from_slice(arg.as_bytes());
                    args_buf.push(0);
                }
                
                unsafe {
                    crate::wasi::CURRENT_TERMINAL_ID = self.window_id.clone();
                }
                let ret = crate::wasi::call_sys_execve(&args_buf, &self.cwd);
                unsafe {
                    crate::wasi::CURRENT_TERMINAL_ID = None;
                }
                if ret != 0 {
                    self.print(env, &format!("{}: command not found\n", program));
                }
            }
        }

        self.input_buffer.clear();
        self.cursor_pos = 0;
        self.print(env, &self.prompt); // Output a newline/prompt so the replace_last_line has a target
        self.redraw_input_line(env);
    }
}

impl Process for TerminalProcess {
    fn id(&self) -> ProcessId { self.pid }
    fn name(&self) -> &str { "terminal" }

    fn tick(&mut self, env: &mut SyscallEnv) -> bool {
        if !self.launched {
            if let Some(_handle) = window::create_window(env, 100, 100, 600, 400) {
                self.window_id = Some(1);
                
                self.print(env, "MonkeyOS Terminal v0.1\nType 'help' for commands.\n\n");
                self.print(env, &self.prompt);
                self.redraw_input_line(env);

                self.launched = true;
            }
        }

        while let Some(msg) = env.recv_msg() {
            match msg.payload {
                MessagePayload::KeyPress { key_code } => {
                    if key_code == 13 { // Enter
                        self.execute_command(env);
                    } else if key_code == 8 { // Backspace
                        if self.cursor_pos > 0 {
                            self.cursor_pos -= 1;
                            self.input_buffer.remove(self.cursor_pos);
                            self.redraw_input_line(env);
                        }
                    } else if key_code == 1037 { // ArrowLeft
                        if self.cursor_pos > 0 {
                            self.cursor_pos -= 1;
                            self.redraw_input_line(env);
                        }
                    } else if key_code == 1039 { // ArrowRight
                        if self.cursor_pos < self.input_buffer.len() {
                            self.cursor_pos += 1;
                            self.redraw_input_line(env);
                        }
                    } else if key_code >= 32 && key_code <= 126 {
                        let c = (key_code as u8) as char;
                        self.input_buffer.insert(self.cursor_pos, c);
                        self.cursor_pos += 1;
                        self.redraw_input_line(env);
                    }
                }
                _ => {}
            }
        }

        true
    }
}
