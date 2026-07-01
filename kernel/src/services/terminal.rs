use crate::process::{Process, ProcessId};
use crate::sys::SyscallEnv;
use crate::ipc::MessagePayload;

pub struct TerminalProcess {
    pid: ProcessId,
    launched: bool,
    window_id: Option<u32>,
    input_buffer: String,
    prompt: String,
    cursor_pos: usize,
    cwd: String,
    edit_state: Option<String>,
    history: Vec<String>,
    history_index: usize,
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
            edit_state: None,
            history: Vec::new(),
            history_index: 0,
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
                let prompt = if self.edit_state.is_some() {
                    "".to_string()
                } else {
                    self.prompt.clone()
                };
                env.send_msg(display_pid, MessagePayload::UpdateHtmlOverlayInputLine { 
                    id, 
                    prompt,
                    input: self.input_buffer.clone(),
                    cursor_pos: self.cursor_pos as u32,
                });
            }
        }
    }

    fn redraw_editor(&self, env: &mut SyscallEnv) {
        if let Some(id) = self.window_id {
            if let Some(display_pid) = env.lookup_service("display") {
                env.send_msg(display_pid, MessagePayload::DrawEditor { 
                    id, 
                    content: self.input_buffer.clone(), 
                    cursor_pos: self.cursor_pos as u32 
                });
            }
        }
    }

    fn execute_command(&mut self, env: &mut SyscallEnv) {
        let cmd = self.input_buffer.trim().to_string();
        
        // Finalize current line (remove cursor block)
        if let Some(id) = self.window_id {
            crate::wasi::print_direct(id, "\n");
        } else {
            self.print(env, "\n");
        }

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
                self.print(env, "Available commands: help, clear, cd, ls, cat, echo, mkdir, rm, touch, pwd, sh, edit\n");
                self.print(env, "Try: ls /bin\n");
            }
            "clear" => {
                if let Some(id) = self.window_id {
                    if let Some(display_pid) = env.lookup_service("display") {
                        env.send_msg(display_pid, MessagePayload::ClearHtmlOverlayText { id });
                    }
                }
            }
            "edit" => {
                if parts.len() < 2 {
                    self.print(env, "Usage: edit <filename>
");
                } else {
                    let mut filename = parts[1].to_string();
                    if !filename.starts_with('/') {
                        if self.cwd == "/" {
                            filename = format!("/{}", filename);
                        } else {
                            filename = format!("{}/{}", self.cwd, filename);
                        }
                    }
                    let file_content = std::fs::read_to_string(&filename).unwrap_or_default();
                    self.edit_state = Some(filename);
                    self.input_buffer = file_content;
                    self.cursor_pos = self.input_buffer.len();
                    self.redraw_editor(env);
                    return;
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
                    if part.is_empty() || part == "." {
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
                let bin_path = "/bin/sh";
                let mut args_buf = Vec::new();
                args_buf.extend_from_slice(bin_path.as_bytes());
                args_buf.push(0);
                args_buf.extend_from_slice(b"-c");
                args_buf.push(0);
                args_buf.extend_from_slice(cmd.as_bytes());
                args_buf.push(0);
                
                unsafe {
                    crate::wasi::CURRENT_TERMINAL_ID = self.window_id;
                }
                crate::wasi::call_sys_execve(&args_buf, &self.cwd, None, None, self.window_id.unwrap_or(0));
                unsafe {
                    crate::wasi::CURRENT_TERMINAL_ID = None;
                }
            }
        }

        if !self.input_buffer.trim().is_empty() {
            self.history.push(self.input_buffer.clone());
        }
        self.history_index = self.history.len();
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
            if let Some(handle) = crate::api::window::create_window(env, 100, 100, 600, 400, "Terminal") {
                self.window_id = Some(handle.id);
                
                self.print(env, "MonkeyOS Terminal v0.1\nType 'help' for commands.\n\n");
                self.print(env, &self.prompt);
                self.redraw_input_line(env);

                self.launched = true;
            }
        }

        while let Some(msg) = env.recv_msg() {
            match msg.payload {
                MessagePayload::KeyPress { key_code } => {
                    if self.edit_state.is_some() {
                        match key_code {
                        19 => { // Ctrl+S
                            if let Some(ref filename) = self.edit_state {
                                if let Err(_e) = std::fs::write(filename, &self.input_buffer) {
                                    // ignore for now or print error
                                }
                            }
                        }
                        17 => { // Ctrl+Q
                            self.edit_state = None;
                            self.input_buffer.clear();
                            self.cursor_pos = 0;
                            if let Some(id) = self.window_id {
                                if let Some(display_pid) = env.lookup_service("display") {
                                    env.send_msg(display_pid, MessagePayload::ClearHtmlOverlayText { id });
                                }
                            }
                            self.print(env, "\n");
                            self.print(env, &self.prompt);
                            self.redraw_input_line(env);
                        }
                        13 => { // Enter
                            self.input_buffer.insert(self.cursor_pos, '\n');
                            self.cursor_pos += 1;
                            self.redraw_editor(env);
                        }
                        8 => { // Backspace
                            if self.cursor_pos > 0 {
                                self.cursor_pos -= 1;
                                self.input_buffer.remove(self.cursor_pos);
                                self.redraw_editor(env);
                            }
                        }
                        1037 => { // ArrowLeft
                            if self.cursor_pos > 0 {
                                self.cursor_pos -= 1;
                                self.redraw_editor(env);
                            }
                        }
                        1038 => { // ArrowUp
                            let mut prev_newline = 0;
                            let mut line_start = 0;
                            let mut col = 0;
                            for (i, c) in self.input_buffer.chars().enumerate() {
                                if i == self.cursor_pos { break; }
                                if c == '\n' {
                                    prev_newline = line_start;
                                    line_start = i + 1;
                                    col = 0;
                                } else {
                                    col += 1;
                                }
                            }
                            if line_start > 0 {
                                self.cursor_pos = prev_newline;
                                for (prev_col, _i) in (prev_newline..line_start-1).enumerate() {
                                    if prev_col == col { break; }
                                    self.cursor_pos += 1;
                                }
                                self.redraw_editor(env);
                            }
                        }
                        1040 => { // ArrowDown
                            let mut col = 0;
                            for (i, c) in self.input_buffer.chars().enumerate() {
                                if i == self.cursor_pos { break; }
                                if c == '\n' {
                                    col = 0;
                                } else {
                                    col += 1;
                                }
                            }
                            if let Some(next_newline) = self.input_buffer[self.cursor_pos..].find('\n') {
                                let next_line_start = self.cursor_pos + next_newline + 1;
                                let mut new_pos = next_line_start;
                                for (curr_col, _) in (next_line_start..self.input_buffer.len()).enumerate() {
                                    if curr_col == col || self.input_buffer.as_bytes()[new_pos] == b'\n' { break; }
                                    new_pos += 1;
                                }
                                self.cursor_pos = new_pos;
                                self.redraw_editor(env);
                            }
                        }
                        46 => { // Delete
                            if self.cursor_pos < self.input_buffer.len() {
                                self.input_buffer.remove(self.cursor_pos);
                                self.redraw_editor(env);
                            }
                        }
                        1039 => { // ArrowRight
                            if self.cursor_pos < self.input_buffer.len() {
                                self.cursor_pos += 1;
                                self.redraw_editor(env);
                            }
                        }
                        code if (32..=126).contains(&code) => {
                            let c = (code as u8) as char;
                            self.input_buffer.insert(self.cursor_pos, c);
                            self.cursor_pos += 1;
                            self.redraw_editor(env);
                        }
                        _ => {}
                    }
                } else {
                    // NORMAL TERMINAL MODE
                    match key_code {
                        13 => { // Enter
                            self.execute_command(env);
                        }
                        8 => { // Backspace
                            if self.cursor_pos > 0 {
                                self.cursor_pos -= 1;
                                self.input_buffer.remove(self.cursor_pos);
                                self.redraw_input_line(env);
                            }
                        }
                        1037 => { // ArrowLeft
                            if self.cursor_pos > 0 {
                                self.cursor_pos -= 1;
                                self.redraw_input_line(env);
                            }
                        }
                        1039 => { // ArrowRight
                            if self.cursor_pos < self.input_buffer.len() {
                                self.cursor_pos += 1;
                                self.redraw_input_line(env);
                            }
                        }
                        1038 => { // ArrowUp
                            if !self.history.is_empty() && self.history_index > 0 {
                                self.history_index -= 1;
                                self.input_buffer = self.history[self.history_index].clone();
                                self.cursor_pos = self.input_buffer.len();
                                self.redraw_input_line(env);
                            }
                        }
                        1040 => { // ArrowDown
                            if self.history_index < self.history.len() {
                                self.history_index += 1;
                                if self.history_index == self.history.len() {
                                    self.input_buffer.clear();
                                } else {
                                    self.input_buffer = self.history[self.history_index].clone();
                                }
                                self.cursor_pos = self.input_buffer.len();
                                self.redraw_input_line(env);
                            }
                        }
                        code if (32..=126).contains(&code) => {
                            let c = (code as u8) as char;
                            self.input_buffer.insert(self.cursor_pos, c);
                            self.cursor_pos += 1;
                            self.redraw_input_line(env);
                        }
                        _ => {}
                    }
                }
                }
                MessagePayload::WindowClosed { id } => {
                    if Some(id) == self.window_id {
                        self.window_id = None;
                    }
                }
                _ => {}
            }
        }

        true
    }
}
