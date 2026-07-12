
use crate::process::{Process, ProcessId};
use crate::sys::SyscallEnv;
use crate::ipc::MessagePayload;

#[link(wasm_import_module = "env")]
extern "C" {
    fn gui_app_mouse_move_js(id: u32, local_x: i32, local_y: i32) -> i32;
    fn gui_app_mouse_down_js(id: u32, local_x: i32, local_y: i32) -> i32;
    fn gui_app_mouse_up_js(id: u32, local_x: i32, local_y: i32) -> i32;
    fn gui_app_key_down_js(id: u32, key_code: u32) -> i32;
}

#[derive(Clone, Copy, PartialEq)]
enum WindowState {
    Normal,
    Maximized,
    Minimized,
}

#[derive(Clone, Copy, PartialEq)]
enum ResizeEdge {
    None,
    Right,
    Bottom,
    BottomRight,
}

struct Window {
    id: u32,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    title: String,
    owner: ProcessId,
    state: WindowState,
    restore_rect: Option<(i32, i32, i32, i32)>, // x, y, w, h
    terminal_lines: Vec<String>,
    terminal_wrapped_lines: Vec<String>,
    terminal_wrap_width: i32,
    terminal_needs_wrap: bool,
    terminal_is_pager: bool,
    terminal_pager_title: Option<String>,
    terminal_scroll_y: usize,
}

pub struct WindowManager {
    pid: ProcessId,
    display_server_pid: ProcessId,
    windows: Vec<Window>,
    mouse_x: i32,
    mouse_y: i32,
    drag_window_index: Option<usize>,
    drag_offset_x: i32,
    drag_offset_y: i32,
    resize_window_index: Option<usize>,
    resize_edge: ResizeEdge,
    tick_count: u32,
    screen_w: i32,
    screen_h: i32,
    start_menu_open: bool,
    last_click_time: u64,
    last_time_str: String,
    last_half_second: u64,
    mouse_is_down: bool,
}

impl WindowManager {
    pub fn new(pid: ProcessId, display_server_pid: ProcessId, screen_w: i32, screen_h: i32) -> Self {
        Self { 
            pid, 
            display_server_pid, 
            windows: Vec::new(),
            mouse_x: 0,
            mouse_y: 0,
            drag_window_index: None,
            drag_offset_x: 0,
            drag_offset_y: 0,
            resize_window_index: None,
            resize_edge: ResizeEdge::None,
            tick_count: 0,
            screen_w,
            screen_h,
            start_menu_open: false,
            last_click_time: 0,
            last_time_str: String::new(),
            last_half_second: 0,
            mouse_is_down: false,
        }
    }

    fn redraw(&self, env: &mut SyscallEnv) {
        env.send_msg(self.display_server_pid, MessagePayload::ClearScreen);
        env.send_msg(self.display_server_pid, MessagePayload::ClearText);
        
        // Draw Desktop Background
        env.send_msg(self.display_server_pid, MessagePayload::DrawRect { 
            x: 0, y: 0, w: self.screen_w, h: self.screen_h, 
            r: 0.08, g: 0.12, b: 0.18, a: 1.0,
            radius: 0.0, shadow_blur: 0.0
        });

        // Draw windows from back to front
        for (i, w) in self.windows.iter().enumerate() {
            if w.state == WindowState::Minimized {
                continue;
            }

            let title_h = 30;
            
            let is_active = !self.start_menu_open && i == self.windows.len() - 1;
            let alpha = if is_active { 1.0 } else { 0.8 };
            let shadow = if is_active { 15.0 } else { 0.0 };

            // Draw entire Window Background with drop shadow and rounded corners
            env.send_msg(self.display_server_pid, MessagePayload::DrawRect { 
                x: w.x, y: w.y, w: w.w, h: w.h, 
                r: 0.1, g: 0.1, b: 0.12, a: 0.85 * alpha, 
                radius: 12.0, shadow_blur: shadow
            });

            // Title text
            env.send_msg(self.display_server_pid, MessagePayload::DrawText { 
                x: w.x + 10, y: w.y + 6, 
                text: w.title.clone(), 
                font_size: 16.0, r: 0.9, g: 0.9, b: 0.9, a: 1.0 
            });

            // Draw a subtle line to separate Title Bar from body
            env.send_msg(self.display_server_pid, MessagePayload::DrawRect { 
                x: w.x, y: w.y + title_h, w: w.w, h: 1, 
                r: 0.25, g: 0.25, b: 0.3, a: 1.0,
                radius: 0.0, shadow_blur: 0.0
            });

            // Draw Close Button (Red) in top right
            env.send_msg(self.display_server_pid, MessagePayload::DrawRect { 
                x: w.x + w.w - 24, y: w.y + 6, w: 18, h: 18, 
                r: 0.9, g: 0.3, b: 0.3, a: 1.0,
                radius: 9.0, shadow_blur: 0.0
            });

            // Draw Maximize Button (Yellow)
            env.send_msg(self.display_server_pid, MessagePayload::DrawRect { 
                x: w.x + w.w - 48, y: w.y + 6, w: 18, h: 18, 
                r: 0.9, g: 0.7, b: 0.2, a: 1.0,
                radius: 9.0, shadow_blur: 0.0
            });

            // Draw Minimize Button (Green)
            env.send_msg(self.display_server_pid, MessagePayload::DrawRect { 
                x: w.x + w.w - 72, y: w.y + 6, w: 18, h: 18, 
                r: 0.2, g: 0.8, b: 0.3, a: 1.0,
                radius: 9.0, shadow_blur: 0.0
            });

            if w.title != "Terminal" {
                // It's a WASM GUI app
                env.send_msg(self.display_server_pid, MessagePayload::DrawGuiApp { 
                    id: w.id, 
                    x: w.x, 
                    y: w.y + title_h, 
                    w: w.w, 
                    h: w.h - title_h 
                });
            } else {
                // It's the terminal
                env.send_msg(self.display_server_pid, MessagePayload::DrawRect { 
                    x: w.x, y: w.y + title_h, w: w.w, h: w.h - title_h, 
                    r: 0.1, g: 0.1, b: 0.12, a: 1.0,
                    radius: 0.0, shadow_blur: 0.0
                });

                let max_lines = ((w.h - title_h - 20) / 20).max(1) as usize;
                
                let start_idx = if w.terminal_is_pager {
                    w.terminal_scroll_y.min(w.terminal_wrapped_lines.len().saturating_sub(max_lines))
                } else if w.terminal_wrapped_lines.len() > max_lines {
                    w.terminal_wrapped_lines.len() - max_lines
                } else { 0 };
                
                let mut text_y = w.y + title_h + 20;
                
                // If pager, maybe draw a header/footer
                if w.terminal_is_pager {
                    if let Some(ref t) = w.terminal_pager_title {
                        env.send_msg(self.display_server_pid, MessagePayload::DrawText {
                            x: w.x + 10,
                            y: w.y + title_h + 5,
                            text: t.clone(),
                            font_size: 14.0,
                            r: 0.9, g: 0.5, b: 0.8, a: 1.0
                        });
                        text_y += 10;
                    }
                }
                
                for line in w.terminal_wrapped_lines.iter().skip(start_idx).take(max_lines) {
                    env.send_msg(self.display_server_pid, MessagePayload::DrawText {
                        x: w.x + 10,
                        y: text_y,
                        text: line.clone(),
                        font_size: 14.0,
                        r: 0.8, g: 0.8, b: 0.8, a: 1.0
                    });
                    text_y += 20;
                }
            }
        }

        // Draw Floating Dock (Taskbar replacement)
        let dock_w = 60 + (self.windows.len() as i32 * 40) + 10 + 70; // 70px for clock
        let dock_h = 50;
        let dock_x = (self.screen_w - dock_w) / 2;
        let dock_y = self.screen_h - dock_h - 15;
        
        env.send_msg(self.display_server_pid, MessagePayload::DrawRect { 
            x: dock_x, y: dock_y, w: dock_w, h: dock_h, 
            r: 0.15, g: 0.15, b: 0.18, a: 0.85,
            radius: 20.0, shadow_blur: 15.0
        });

        env.send_msg(self.display_server_pid, MessagePayload::DrawText {
            x: dock_x + dock_w - 60,
            y: dock_y + 26,
            text: self.last_time_str.clone(),
            font_size: 16.0,
            r: 0.9, g: 0.9, b: 0.9, a: 1.0
        });

        // Draw Start Button (Red Circle)
        env.send_msg(self.display_server_pid, MessagePayload::DrawRect { 
            x: dock_x + 10, y: dock_y + 10, w: 30, h: 30, 
            r: 0.9, g: 0.4, b: 0.4, a: 1.0,
            radius: 15.0, shadow_blur: 5.0
        });
        env.send_msg(self.display_server_pid, MessagePayload::DrawCenteredText {
            x: dock_x + 25, y: dock_y + 25,
            text: "🐒".to_string(),
            font_size: 18.0,
            r: 1.0, g: 1.0, b: 1.0, a: 1.0
        });
        
        // Draw dynamically for windows
        for (idx, w) in self.windows.iter().enumerate() {
            let icon_x = dock_x + 60 + (idx as i32 * 40);
            
            // Choose a color and emoji based on title
            let (r, g, b, emoji) = if w.title == "Terminal" {
                (0.2, 0.8, 0.4, "💻")
            } else if w.title == "Calculator" {
                (0.8, 0.6, 0.2, "🖩")
            } else if w.title == "Notepad" {
                (0.8, 0.4, 0.6, "📝")
            } else if w.title == "File Manager" {
                (0.4, 0.4, 0.8, "📁")
            } else {
                (0.5, 0.5, 0.5, "❓")
            };

            env.send_msg(self.display_server_pid, MessagePayload::DrawRect { 
                x: icon_x, y: dock_y + 10, w: 30, h: 30, 
                r, g, b, a: 1.0,
                radius: 8.0, shadow_blur: 5.0
            });
            
            env.send_msg(self.display_server_pid, MessagePayload::DrawCenteredText {
                x: icon_x + 15, y: dock_y + 25,
                text: emoji.to_string(),
                font_size: 18.0,
                r: 1.0, g: 1.0, b: 1.0, a: 1.0
            });

            // Draw indicator dot
            if w.state != WindowState::Minimized {
                env.send_msg(self.display_server_pid, MessagePayload::DrawRect { 
                    x: icon_x + 12, y: dock_y + 42, w: 6, h: 6, 
                    r: 0.8, g: 0.8, b: 0.8, a: 1.0,
                    radius: 3.0, shadow_blur: 2.0
                });
            }
        }

        // Draw Start Menu
        if self.start_menu_open {
            env.send_msg(self.display_server_pid, MessagePayload::DrawRect { 
                x: dock_x, y: dock_y - 320, w: 250, h: 300, 
                r: 0.12, g: 0.12, b: 0.15, a: 0.95,
                radius: 16.0, shadow_blur: 30.0
            });

            // Start Menu: Terminal App entry
            env.send_msg(self.display_server_pid, MessagePayload::DrawRect { 
                x: dock_x + 20, y: dock_y - 300, w: 40, h: 40, 
                r: 0.2, g: 0.8, b: 0.4, a: 1.0,
                radius: 8.0, shadow_blur: 5.0
            });
            env.send_msg(self.display_server_pid, MessagePayload::DrawCenteredText {
                x: dock_x + 40, y: dock_y - 280,
                text: "💻".to_string(),
                font_size: 20.0, r: 1.0, g: 1.0, b: 1.0, a: 1.0
            });
            env.send_msg(self.display_server_pid, MessagePayload::DrawText { 
                x: dock_x + 70, y: dock_y - 290, 
                text: "Terminal".to_string(), 
                font_size: 16.0, r: 0.9, g: 0.9, b: 0.9, a: 1.0 
            });

            // Start Menu: Calculator App entry
            env.send_msg(self.display_server_pid, MessagePayload::DrawRect { 
                x: dock_x + 20, y: dock_y - 250, w: 40, h: 40, 
                r: 0.8, g: 0.6, b: 0.2, a: 1.0,
                radius: 8.0, shadow_blur: 5.0
            });
            env.send_msg(self.display_server_pid, MessagePayload::DrawCenteredText {
                x: dock_x + 40, y: dock_y - 230,
                text: "🖩".to_string(),
                font_size: 20.0, r: 1.0, g: 1.0, b: 1.0, a: 1.0
            });
            env.send_msg(self.display_server_pid, MessagePayload::DrawText { 
                x: dock_x + 70, y: dock_y - 240, 
                text: "Calculator".to_string(), 
                font_size: 16.0, r: 0.9, g: 0.9, b: 0.9, a: 1.0 
            });

            // Start Menu: File Manager App entry
            env.send_msg(self.display_server_pid, MessagePayload::DrawRect { 
                x: dock_x + 20, y: dock_y - 200, w: 40, h: 40, 
                r: 0.4, g: 0.4, b: 0.8, a: 1.0,
                radius: 8.0, shadow_blur: 5.0
            });
            env.send_msg(self.display_server_pid, MessagePayload::DrawCenteredText {
                x: dock_x + 40, y: dock_y - 180,
                text: "📁".to_string(),
                font_size: 20.0, r: 1.0, g: 1.0, b: 1.0, a: 1.0
            });
            env.send_msg(self.display_server_pid, MessagePayload::DrawText { 
                x: dock_x + 70, y: dock_y - 190, 
                text: "File Manager".to_string(), 
                font_size: 16.0, r: 0.9, g: 0.9, b: 0.9, a: 1.0 
            });

            // Start Menu: Notepad App entry
            env.send_msg(self.display_server_pid, MessagePayload::DrawRect { 
                x: dock_x + 20, y: dock_y - 150, w: 40, h: 40, 
                r: 0.8, g: 0.4, b: 0.6, a: 1.0,
                radius: 8.0, shadow_blur: 5.0
            });
            env.send_msg(self.display_server_pid, MessagePayload::DrawCenteredText {
                x: dock_x + 40, y: dock_y - 130,
                text: "📝".to_string(),
                font_size: 20.0, r: 1.0, g: 1.0, b: 1.0, a: 1.0
            });
            env.send_msg(self.display_server_pid, MessagePayload::DrawText { 
                x: dock_x + 70, y: dock_y - 140, 
                text: "Notepad".to_string(), 
                font_size: 16.0, r: 0.9, g: 0.9, b: 0.9, a: 1.0 
            });

            // Start Menu: ImgView App entry
            env.send_msg(self.display_server_pid, MessagePayload::DrawRect { 
                x: dock_x + 20, y: dock_y - 100, w: 40, h: 40, 
                r: 0.2, g: 0.6, b: 0.8, a: 1.0,
                radius: 8.0, shadow_blur: 5.0
            });
            env.send_msg(self.display_server_pid, MessagePayload::DrawCenteredText {
                x: dock_x + 40, y: dock_y - 80,
                text: "🖼️".to_string(),
                font_size: 20.0, r: 1.0, g: 1.0, b: 1.0, a: 1.0
            });
            env.send_msg(self.display_server_pid, MessagePayload::DrawText { 
                x: dock_x + 70, y: dock_y - 90, 
                text: "Image Viewer".to_string(), 
                font_size: 16.0, r: 0.9, g: 0.9, b: 0.9, a: 1.0 
            });
        }
    }
}

impl Process for WindowManager {
    fn id(&self) -> ProcessId { self.pid }
    fn name(&self) -> &str { "window_manager" }

    fn tick(&mut self, env: &mut SyscallEnv) -> bool {
        let mut needs_redraw = false;
        
        self.tick_count += 1;
        if self.tick_count % 30 == 0 {
            needs_redraw = true; // periodic redraw for cursor blinking
        }

        while let Some(msg) = env.recv_msg() {
            match msg.payload {
                MessagePayload::UpdateTerminalBuffer { id, lines, is_pager, pager_title, scroll_y } => {
                    if let Some(w) = self.windows.iter_mut().find(|w| w.id == id) {
                        w.terminal_lines = lines;
                        if !w.terminal_is_pager && is_pager {
                            w.terminal_scroll_y = 0;
                        } else if !is_pager {
                            w.terminal_scroll_y = scroll_y;
                        }
                        w.terminal_is_pager = is_pager;
                        w.terminal_pager_title = pager_title;
                        w.terminal_needs_wrap = true;
                        needs_redraw = true;
                    }
                }
                MessagePayload::ScreenSizeChanged { w, h } => {
                    self.screen_w = w;
                    self.screen_h = h;
                    needs_redraw = true;
                }
                MessagePayload::CreateWindow { id, x, y, w, h, title, owner } => {
                    self.windows.push(Window { 
                        id, x, y, w, h, 
                        title, 
                        owner, 
                        state: WindowState::Normal,
                        restore_rect: None,
                        terminal_lines: Vec::new(),
                        terminal_wrapped_lines: Vec::new(),
                        terminal_wrap_width: 0,
                        terminal_needs_wrap: true,
                        terminal_is_pager: false,
                        terminal_pager_title: None,
                        terminal_scroll_y: 0,
                    });
                    
                    needs_redraw = true;
                }
                MessagePayload::MouseMove { x, y } => {
                    let dx = x - self.mouse_x;
                    let dy = y - self.mouse_y;
                    self.mouse_x = x;
                    self.mouse_y = y;
                    
                    let title_h = 30;

                    if let Some(active_win) = self.windows.last() {
                        let rx = self.mouse_x - active_win.x;
                        let ry = self.mouse_y - (active_win.y + title_h);
                        // Forward if mouse is currently down (capture), or if it's within bounds
                        if self.mouse_is_down || (rx >= 0 && ry >= 0 && rx < active_win.w && ry < (active_win.h - title_h)) {
                            if active_win.owner == 0 {
                                let redraw = unsafe { gui_app_mouse_move_js(active_win.id, self.mouse_x, self.mouse_y) };
                                if redraw != 0 {
                                    needs_redraw = true;
                                }
                            } else {
                                env.send_msg(active_win.owner, MessagePayload::MouseMove { x: rx, y: ry });
                            }
                        }
                    }

                    if let Some(idx) = self.drag_window_index {
                        if let Some(win) = self.windows.get_mut(idx) {
                            let dock_h = 50;
                            let mut snapped = false;
                            if self.mouse_x <= 5 {
                                win.restore_rect = Some((win.x, win.y, win.w, win.h));
                                win.state = WindowState::Maximized;
                                win.x = 0; win.y = 0;
                                win.w = self.screen_w / 2;
                                win.h = self.screen_h - dock_h - 20;
                                snapped = true;
                            } else if self.mouse_x >= self.screen_w - 5 {
                                win.restore_rect = Some((win.x, win.y, win.w, win.h));
                                win.state = WindowState::Maximized;
                                win.x = self.screen_w / 2; win.y = 0;
                                win.w = self.screen_w / 2;
                                win.h = self.screen_h - dock_h - 20;
                                snapped = true;
                            } else if self.mouse_y <= 5 {
                                win.restore_rect = Some((win.x, win.y, win.w, win.h));
                                win.state = WindowState::Maximized;
                                win.x = 0; win.y = 0;
                                win.w = self.screen_w;
                                win.h = self.screen_h - dock_h - 20;
                                snapped = true;
                            }
                            
                            if snapped {
                                self.drag_window_index = None;
                            } else {
                                win.x = self.mouse_x - self.drag_offset_x;
                                win.y = self.mouse_y - self.drag_offset_y;
                            }
                            needs_redraw = true;
                        }
                    } else if let Some(idx) = self.resize_window_index {
                        if let Some(win) = self.windows.get_mut(idx) {
                            match self.resize_edge {
                                ResizeEdge::Right => {
                                    win.w = (win.w + dx).max(100);
                                }
                                ResizeEdge::Bottom => {
                                    win.h = (win.h + dy).max(100);
                                }
                                ResizeEdge::BottomRight => {
                                    win.w = (win.w + dx).max(100);
                                    win.h = (win.h + dy).max(100);
                                }
                                _ => {}
                            }

                            needs_redraw = true;
                        }
                    }
                }
                MessagePayload::MouseButton { down } => {
                    self.mouse_is_down = down;
                    let mut needs_redraw = false;
                    if down {
                        let title_h = 30;
                        let mut clicked_idx = None;
                        let mut clicked_action = 0; // 0=focus, 1=close, 2=maximize, 3=minimize
                        
                        let dock_w = 60 + (self.windows.len() as i32 * 40) + 10 + 70;
                        let dock_h = 50;
                        let dock_x = (self.screen_w - dock_w) / 2;
                        let dock_y = self.screen_h - dock_h - 15;

                        // Check Start Button collision
                        if self.mouse_x >= dock_x + 10 && self.mouse_x <= dock_x + 40 &&
                           self.mouse_y >= dock_y + 10 && self.mouse_y <= dock_y + 40 {
                            self.start_menu_open = !self.start_menu_open;
                            needs_redraw = true;
                            continue;
                        }

                        // Check Start Menu items
                        if self.start_menu_open {
                            if self.mouse_x >= dock_x && self.mouse_x <= dock_x + 250 && 
                               self.mouse_y >= dock_y - 320 && self.mouse_y <= dock_y - 20 {
                                
                                // Terminal click
                                if self.mouse_y >= dock_y - 300 && self.mouse_y <= dock_y - 260 {
                                    env.spawn_process("terminal");
                                    self.start_menu_open = false;
                                }
                                // Calculator click
                                else if self.mouse_y >= dock_y - 250 && self.mouse_y <= dock_y - 210 {
                                    env.spawn_process("/bin/calc");
                                    self.start_menu_open = false;
                                }
                                // File Manager click
                                else if self.mouse_y >= dock_y - 200 && self.mouse_y <= dock_y - 160 {
                                    env.spawn_process("/bin/fileman");
                                    self.start_menu_open = false;
                                }
                                // Notepad click
                                else if self.mouse_y >= dock_y - 150 && self.mouse_y <= dock_y - 110 {
                                    env.spawn_process("/bin/notepad");
                                    self.start_menu_open = false;
                                }
                                // ImgView click
                                else if self.mouse_y >= dock_y - 100 && self.mouse_y <= dock_y - 60 {
                                    env.spawn_process("/bin/imgview");
                                    self.start_menu_open = false;
                                }
                                
                                self.start_menu_open = false;
                                needs_redraw = true;
                                continue;
                            } else if self.mouse_y < self.screen_h - dock_h {
                                // Clicked outside start menu, close it
                                self.start_menu_open = false;
                                needs_redraw = true;
                            }
                        }

                        // Check Taskbar Windows
                        let mut clicked_taskbar = false;
                        for (idx, w) in self.windows.iter_mut().enumerate() {
                            let icon_x = dock_x + 60 + (idx as i32 * 40);
                            if self.mouse_x >= icon_x && self.mouse_x <= icon_x + 30 &&
                               self.mouse_y >= dock_y + 10 && self.mouse_y <= dock_y + 40 {
                                // Toggle minimize/restore
                                if w.state == WindowState::Minimized {
                                    w.state = WindowState::Normal;
                                } else {
                                    w.state = WindowState::Minimized;
                                }
                                clicked_taskbar = true;
                                break;
                            }
                        }
                        if clicked_taskbar {
                            needs_redraw = true;
                            continue;
                        }

                        // Check window collisions from front to back
                        for (i, win) in self.windows.iter().enumerate().rev() {
                            if win.state == WindowState::Minimized {
                                continue;
                            }
                            
                            // 10px invisible resize border
                            let border = 10;
                            
                            // Check resize borders
                            let on_right = self.mouse_x >= win.x + win.w - border && self.mouse_x <= win.x + win.w + border &&
                                           self.mouse_y >= win.y && self.mouse_y <= win.y + win.h;
                            let on_bottom = self.mouse_y >= win.y + win.h - border && self.mouse_y <= win.y + win.h + border &&
                                            self.mouse_x >= win.x && self.mouse_x <= win.x + win.w;
                            let on_bottom_right = on_right && on_bottom;

                            if on_bottom_right {
                                self.resize_window_index = Some(self.windows.len() - 1);
                                self.resize_edge = ResizeEdge::BottomRight;
                                clicked_idx = Some(i);
                                break;
                            } else if on_right {
                                self.resize_window_index = Some(self.windows.len() - 1);
                                self.resize_edge = ResizeEdge::Right;
                                clicked_idx = Some(i);
                                break;
                            } else if on_bottom {
                                self.resize_window_index = Some(self.windows.len() - 1);
                                self.resize_edge = ResizeEdge::Bottom;
                                clicked_idx = Some(i);
                                break;
                            }

                            if self.mouse_x >= win.x && self.mouse_x <= win.x + win.w &&
                               self.mouse_y >= win.y && self.mouse_y <= win.y + win.h {
                                clicked_idx = Some(i);
                                
                                // Check title bar buttons
                                if self.mouse_y <= win.y + title_h {
                                    if self.mouse_x >= win.x + win.w - 30 {
                                        clicked_action = 1; // Close
                                        break;
                                    } else if self.mouse_x >= win.x + win.w - 54 {
                                        clicked_action = 2; // Maximize
                                        break;
                                    } else if self.mouse_x >= win.x + win.w - 78 {
                                        clicked_action = 3; // Minimize
                                        break;
                                    }

                                    // Drag
                                    self.drag_window_index = Some(self.windows.len() - 1); // will move to end
                                    self.drag_offset_x = self.mouse_x - win.x;
                                    self.drag_offset_y = self.mouse_y - win.y;

                                    // Double click maximize
                                    let now = crate::wasi::call_sys_time_ms();
                                    if now - self.last_click_time < 300 {
                                        clicked_action = 2; // Maximize
                                    }
                                    self.last_click_time = now;
                                }
                                break;
                            }
                        }

                        if let Some(idx) = clicked_idx {
                            if clicked_action == 1 {
                                let win = &self.windows[idx];
                                env.send_msg(win.owner, MessagePayload::WindowClosed { id: win.id });
                                self.windows.remove(idx);
                            } else if clicked_action == 2 { // Maximize
                                let win = &mut self.windows[idx];
                                if win.state == WindowState::Maximized {
                                    win.state = WindowState::Normal;
                                    if let Some((rx, ry, rw, rh)) = win.restore_rect {
                                        win.x = rx; win.y = ry; win.w = rw; win.h = rh;
                                    }
                                } else {
                                    win.restore_rect = Some((win.x, win.y, win.w, win.h));
                                    win.state = WindowState::Maximized;
                                    win.x = 0; win.y = 0;
                                    win.w = self.screen_w;
                                    win.h = self.screen_h - dock_h - 20; // leave room for dock
                                }
                                let win_owned = self.windows.remove(idx);
                                self.windows.push(win_owned);
                            } else if clicked_action == 3 { // Minimize
                                let win = &mut self.windows[idx];
                                win.state = WindowState::Minimized;
                            } else {
                                // Bring to front
                                let win_owned = self.windows.remove(idx);
                                self.windows.push(win_owned);
                                
                                let title_h = 30;
                                let active_win = self.windows.last().unwrap();
                                let rx = self.mouse_x - active_win.x;
                                let ry = self.mouse_y - (active_win.y + title_h);
                                if rx >= 0 && ry >= 0 && rx < active_win.w && ry < (active_win.h - title_h) {
                                    if active_win.owner == 0 {
                                        let redraw = unsafe { gui_app_mouse_down_js(active_win.id, self.mouse_x, self.mouse_y) };
                                        if redraw != 0 {
                                            needs_redraw = true;
                                        }
                                    } else {
                                        env.send_msg(active_win.owner, MessagePayload::MouseButton { down: true });
                                    }
                                }
                            }
                        }
                    } else {
                        self.drag_window_index = None;
                        self.resize_window_index = None;
                        self.resize_edge = ResizeEdge::None;
                        
                        // Forward mouse up to active window (always forward, not just in-bounds,
                        // because the app may be in a drag state that needs to be released)
                        let title_h = 30;
                        if let Some(active_win) = self.windows.last() {
                            if active_win.owner == 0 {
                                let redraw = unsafe { gui_app_mouse_up_js(active_win.id, self.mouse_x, self.mouse_y) };
                                if redraw != 0 {
                                    needs_redraw = true;
                                }
                            } else {
                                env.send_msg(active_win.owner, MessagePayload::MouseButton { down: false });
                            }
                        }
                    }
                }
                MessagePayload::KeyPress { key_code } => {
                    if let Some(active_win) = self.windows.last_mut() {
                        if active_win.owner == 0 {
                            let redraw = unsafe { gui_app_key_down_js(active_win.id, key_code) };
                            if redraw != 0 {
                                needs_redraw = true;
                            }
                        } else {
                            if active_win.terminal_is_pager {
                                match key_code {
                                    1038 | 107 => { // up
                                        if active_win.terminal_scroll_y > 0 {
                                            active_win.terminal_scroll_y -= 1;
                                            needs_redraw = true;
                                        }
                                    }
                                    1040 | 106 => { // down
                                        let max_lines = ((active_win.h - 30 - 20) / 20).max(1) as usize;
                                        let limit = active_win.terminal_wrapped_lines.len().saturating_sub(max_lines);
                                        if active_win.terminal_scroll_y < limit {
                                            active_win.terminal_scroll_y += 1;
                                            needs_redraw = true;
                                        }
                                    }
                                    32 => { // space
                                        let max_lines = ((active_win.h - 30 - 20) / 20).max(1) as usize;
                                        let limit = active_win.terminal_wrapped_lines.len().saturating_sub(max_lines);
                                        active_win.terminal_scroll_y = (active_win.terminal_scroll_y + max_lines.saturating_sub(1)).min(limit);
                                        needs_redraw = true;
                                    }
                                    _ => {
                                        env.send_msg(active_win.owner, MessagePayload::KeyPress { key_code });
                                    }
                                }
                            } else {
                                env.send_msg(active_win.owner, MessagePayload::KeyPress { key_code });
                            }
                        }
                    }
                }
                _ => {}
            }
        }

        let utc_ms = crate::wasi::call_sys_time_ms();
        let tz_offset_ms = crate::wasi::call_sys_timezone_offset_ms();
        let local_ms = (utc_ms as i64 - tz_offset_ms) as u64;
        let s = local_ms / 1000;
        let m = (s / 60) % 60;
        let h = (s / 3600) % 24;
        let time_str = format!("{:02}:{:02}", h, m);
        let time_str = format!("{:02}:{:02}", h, m);
        if time_str != self.last_time_str {
            self.last_time_str = time_str;
            needs_redraw = true;
        }
        
        // Force redraw every 500ms for blinking cursors
        let current_half_second = local_ms / 500;
        if current_half_second != self.last_half_second {
            self.last_half_second = current_half_second;
            needs_redraw = true;
        }

        if needs_redraw {
            for w in self.windows.iter_mut() {
                if w.title == "Terminal" && (w.terminal_needs_wrap || w.terminal_wrap_width != w.w) {
                    let mut wrapped_lines = Vec::new();
                    let chars_per_line = ((w.w - 20) as f32 / 8.4).max(10.0) as usize;

                    for line in &w.terminal_lines {
                        let mut current_line = line.as_str();
                        while current_line.len() > chars_per_line {
                            let (chunk, rest) = current_line.split_at(chars_per_line);
                            wrapped_lines.push(chunk.to_string());
                            current_line = rest;
                        }
                        wrapped_lines.push(current_line.to_string());
                    }
                    w.terminal_wrapped_lines = wrapped_lines;
                    w.terminal_wrap_width = w.w;
                    w.terminal_needs_wrap = false;
                }
            }
            
            self.redraw(env);
        }

        true
    }
}
