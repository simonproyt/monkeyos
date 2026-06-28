use crate::process::{Process, ProcessId};
use crate::sys::SyscallEnv;
use crate::ipc::MessagePayload;

struct Window {
    id: u32,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    has_overlay: bool,
}

pub struct WindowManager {
    pid: ProcessId,
    display_server_pid: ProcessId,
    windows: Vec<Window>,
    mouse_x: i32,
    mouse_y: i32,
    is_dragging: bool,
    drag_offset_x: i32,
    drag_offset_y: i32,
    next_window_id: u32,
}

impl WindowManager {
    pub fn new(pid: ProcessId, display_server_pid: ProcessId) -> Self {
        Self { 
            pid, 
            display_server_pid, 
            windows: Vec::new(),
            mouse_x: 0,
            mouse_y: 0,
            is_dragging: false,
            drag_offset_x: 0,
            drag_offset_y: 0,
            next_window_id: 1,
        }
    }

    fn redraw(&self, env: &mut SyscallEnv) {
        env.send_msg(self.display_server_pid, MessagePayload::ClearScreen);
        for w in &self.windows {
            env.send_msg(self.display_server_pid, MessagePayload::DrawRect { x: w.x, y: w.y, w: w.w, h: w.h });
            if w.has_overlay {
                env.send_msg(self.display_server_pid, MessagePayload::UpdateHtmlOverlayPos { id: w.id, x: w.x, y: w.y });
            }
        }
    }
}

impl Process for WindowManager {
    fn id(&self) -> ProcessId { self.pid }
    fn name(&self) -> &str { "window_manager" }

    fn tick(&mut self, env: &mut SyscallEnv) -> bool {
        let mut needs_redraw = false;

        while let Some(msg) = env.recv_msg() {
            match msg.payload {
                // An app requested to draw a rect (simplified window creation)
                MessagePayload::DrawRect { x, y, w, h } => {
                    let id = self.next_window_id;
                    self.next_window_id += 1;
                    
                    // For now, assume every window gets an overlay
                    self.windows.push(Window { id, x, y, w, h, has_overlay: true });
                    env.send_msg(self.display_server_pid, MessagePayload::CreateHtmlOverlay { id, x, y, w, h });
                    
                    needs_redraw = true;
                }
                MessagePayload::MouseMove { x, y } => {
                    self.mouse_x = x;
                    self.mouse_y = y;
                    
                    if self.is_dragging {
                        if let Some(win) = self.windows.first_mut() {
                            win.x = self.mouse_x - self.drag_offset_x;
                            win.y = self.mouse_y - self.drag_offset_y;
                            needs_redraw = true;
                        }
                    }
                }
                MessagePayload::MouseButton { down } => {
                    if down {
                        if let Some(win) = self.windows.first() {
                            // Check collision with the single window
                            if self.mouse_x >= win.x && self.mouse_x <= win.x + win.w &&
                               self.mouse_y >= win.y && self.mouse_y <= win.y + win.h {
                                   self.is_dragging = true;
                                   self.drag_offset_x = self.mouse_x - win.x;
                                   self.drag_offset_y = self.mouse_y - win.y;
                            }
                        }
                    } else {
                        self.is_dragging = false;
                    }
                }
                _ => {}
            }
        }

        if needs_redraw {
            self.redraw(env);
        }

        true
    }
}
