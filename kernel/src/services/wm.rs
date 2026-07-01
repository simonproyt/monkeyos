use crate::process::{Process, ProcessId};
use crate::sys::SyscallEnv;
use crate::ipc::MessagePayload;

#[derive(Clone, Copy, PartialEq)]
enum WindowState {
    Normal,
    Maximized,
    Minimized,
}

#[derive(Clone, Copy, PartialEq)]
enum ResizeEdge {
    None,
    Left,
    Right,
    Top,
    Bottom,
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
}

struct Window {
    id: u32,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    _title: String,
    owner: ProcessId,
    has_overlay: bool,
    state: WindowState,
    restore_rect: Option<(i32, i32, i32, i32)>, // x, y, w, h
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
    screen_w: i32,
    screen_h: i32,
    start_menu_open: bool,
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
            screen_w,
            screen_h,
            start_menu_open: false,
        }
    }

    fn redraw(&self, env: &mut SyscallEnv) {
        env.send_msg(self.display_server_pid, MessagePayload::ClearScreen);
        
        // Draw Desktop Background
        env.send_msg(self.display_server_pid, MessagePayload::DrawRect { 
            x: 0, y: 0, w: self.screen_w, h: self.screen_h, 
            r: 0.08, g: 0.12, b: 0.18, a: 1.0,
            radius: 0.0, shadow_blur: 0.0
        });

        // Draw windows from back to front
        for (i, w) in self.windows.iter().enumerate() {
            if w.state == WindowState::Minimized {
                if w.has_overlay {
                    // We need to hide the overlay when minimized
                    env.send_msg(self.display_server_pid, MessagePayload::UpdateHtmlOverlayBounds { 
                        id: w.id, 
                        x: -9999, // Move offscreen to hide
                        y: -9999,
                        w: w.w,
                        h: w.h,
                        z: i as u32
                    });
                }
                continue;
            }

            let title_h = 30;
            
            // Draw entire Window Background with drop shadow and rounded corners
            env.send_msg(self.display_server_pid, MessagePayload::DrawRect { 
                x: w.x, y: w.y, w: w.w, h: w.h, 
                r: 0.1, g: 0.1, b: 0.12, a: 0.85, 
                radius: 12.0, shadow_blur: 15.0
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

            if w.has_overlay {
                env.send_msg(self.display_server_pid, MessagePayload::UpdateHtmlOverlayBounds { 
                    id: w.id, 
                    x: w.x, 
                    y: w.y + title_h,
                    w: w.w,
                    h: w.h - title_h,
                    z: i as u32
                });
            }
        }

        // Draw Floating Dock (Taskbar replacement)
        let dock_w = 200;
        let dock_h = 50;
        let dock_x = (self.screen_w - dock_w) / 2;
        let dock_y = self.screen_h - dock_h - 15;
        
        env.send_msg(self.display_server_pid, MessagePayload::DrawRect { 
            x: dock_x, y: dock_y, w: dock_w, h: dock_h, 
            r: 0.15, g: 0.15, b: 0.18, a: 0.85,
            radius: 20.0, shadow_blur: 15.0
        });

        // Draw Start Button (Red Circle)
        env.send_msg(self.display_server_pid, MessagePayload::DrawRect { 
            x: dock_x + 10, y: dock_y + 10, w: 30, h: 30, 
            r: 0.9, g: 0.4, b: 0.4, a: 1.0,
            radius: 15.0, shadow_blur: 5.0
        });
        
        // Draw Terminal Icon (Green Rounded Rect)
        env.send_msg(self.display_server_pid, MessagePayload::DrawRect { 
            x: dock_x + 60, y: dock_y + 10, w: 30, h: 30, 
            r: 0.2, g: 0.8, b: 0.4, a: 1.0,
            radius: 8.0, shadow_blur: 5.0
        });
        
        // Draw Settings Icon (Blue Rounded Rect)
        env.send_msg(self.display_server_pid, MessagePayload::DrawRect { 
            x: dock_x + 110, y: dock_y + 10, w: 30, h: 30, 
            r: 0.2, g: 0.4, b: 0.9, a: 1.0,
            radius: 8.0, shadow_blur: 5.0
        });

        // Draw indicator dot if terminal is running (any window has id)
        if !self.windows.is_empty() {
            env.send_msg(self.display_server_pid, MessagePayload::DrawRect { 
                x: dock_x + 72, y: dock_y + 42, w: 6, h: 6, 
                r: 0.8, g: 0.8, b: 0.8, a: 1.0,
                radius: 3.0, shadow_blur: 2.0
            });
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
            // We would draw text here, but without font rendering we just rely on the colored box
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
                MessagePayload::ScreenSizeChanged { w, h } => {
                    self.screen_w = w;
                    self.screen_h = h;
                    needs_redraw = true;
                }
                MessagePayload::CreateWindow { id, x, y, w, h, title, owner } => {
                    self.windows.push(Window { 
                        id, x, y, w, h, 
                        _title: title, 
                        owner, 
                        has_overlay: true,
                        state: WindowState::Normal,
                        restore_rect: None,
                    });
                    
                    let title_h = 30;
                    env.send_msg(self.display_server_pid, MessagePayload::CreateHtmlOverlay { 
                        id, 
                        x, 
                        y: y + title_h, 
                        w, 
                        h: h - title_h 
                    });
                    
                    needs_redraw = true;
                }
                MessagePayload::MouseMove { x, y } => {
                    let dx = x - self.mouse_x;
                    let dy = y - self.mouse_y;
                    self.mouse_x = x;
                    self.mouse_y = y;
                    
                    let title_h = 30;

                    if let Some(idx) = self.drag_window_index {
                        if let Some(win) = self.windows.get_mut(idx) {
                            win.x = self.mouse_x - self.drag_offset_x;
                            win.y = self.mouse_y - self.drag_offset_y;
                            if win.has_overlay {
                                env.send_msg(self.display_server_pid, MessagePayload::UpdateHtmlOverlayBounds { 
                                    id: win.id, x: win.x, y: win.y + title_h, w: win.w, h: win.h - title_h, z: idx as u32
                                });
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
                            if win.has_overlay {
                                env.send_msg(self.display_server_pid, MessagePayload::UpdateHtmlOverlayBounds { 
                                    id: win.id, x: win.x, y: win.y + title_h, w: win.w, h: win.h - title_h, z: idx as u32
                                });
                            }
                            needs_redraw = true;
                        }
                    }
                }
                MessagePayload::MouseButton { down } => {
                    if down {
                        let title_h = 30;
                        let mut clicked_idx = None;
                        let mut clicked_action = 0; // 0=focus, 1=close, 2=maximize, 3=minimize
                        
                        let dock_w = 200;
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

                        // Check Terminal Icon collision (Launch App or Unminimize)
                        if self.mouse_x >= dock_x + 60 && self.mouse_x <= dock_x + 90 &&
                           self.mouse_y >= dock_y + 10 && self.mouse_y <= dock_y + 40 {
                            // find if there is a minimized terminal
                            let mut found_minimized = None;
                            for (i, win) in self.windows.iter().enumerate() {
                                if win.state == WindowState::Minimized {
                                    found_minimized = Some(i);
                                    break;
                                }
                            }
                            if let Some(idx) = found_minimized {
                                let win = &mut self.windows[idx];
                                win.state = WindowState::Normal;
                                if win.has_overlay {
                                    env.send_msg(self.display_server_pid, MessagePayload::UpdateHtmlOverlayBounds { 
                                        id: win.id, x: win.x, y: win.y + title_h, w: win.w, h: win.h - title_h, z: self.windows.len() as u32 - 1
                                    });
                                }
                                let w_owned = self.windows.remove(idx);
                                self.windows.push(w_owned);
                            } else {
                                env.spawn_process("terminal");
                            }
                            needs_redraw = true;
                            continue;
                        }

                        // Check Start Menu items
                        if self.start_menu_open {
                            let sm_x = dock_x;
                            let sm_y = dock_y - 320;
                            let sm_w = 250;
                            let sm_h = 300;
                            if self.mouse_x < sm_x || self.mouse_x > sm_x + sm_w ||
                               self.mouse_y < sm_y || self.mouse_y > sm_y + sm_h {
                                self.start_menu_open = false;
                                needs_redraw = true;
                            } else {
                                // Clicked inside start menu! Spawn a terminal for now.
                                env.spawn_process("terminal");
                                self.start_menu_open = false;
                                needs_redraw = true;
                                continue;
                            }
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
                                }
                                break;
                            }
                        }

                        if let Some(idx) = clicked_idx {
                            if clicked_action == 1 {
                                let win = &self.windows[idx];
                                env.send_msg(self.display_server_pid, MessagePayload::DestroyHtmlOverlay { id: win.id });
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
                                if win.has_overlay {
                                    env.send_msg(self.display_server_pid, MessagePayload::UpdateHtmlOverlayBounds { 
                                        id: win.id, x: win.x, y: win.y + title_h, w: win.w, h: win.h - title_h, z: self.windows.len() as u32 - 1
                                    });
                                }
                                let win_owned = self.windows.remove(idx);
                                self.windows.push(win_owned);
                            } else if clicked_action == 3 { // Minimize
                                let win = &mut self.windows[idx];
                                win.state = WindowState::Minimized;
                                if win.has_overlay {
                                    env.send_msg(self.display_server_pid, MessagePayload::UpdateHtmlOverlayBounds { 
                                        id: win.id, x: -9999, y: -9999, w: win.w, h: win.h, z: idx as u32
                                    });
                                }
                            } else {
                                // Bring to front
                                let win_owned = self.windows.remove(idx);
                                self.windows.push(win_owned);
                            }
                            needs_redraw = true;
                        }
                    } else {
                        self.drag_window_index = None;
                        self.resize_window_index = None;
                        self.resize_edge = ResizeEdge::None;
                    }
                }
                MessagePayload::KeyPress { key_code } => {
                    if let Some(active_win) = self.windows.last() {
                        env.send_msg(active_win.owner, MessagePayload::KeyPress { key_code });
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
