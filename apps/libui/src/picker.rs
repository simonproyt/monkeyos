use crate::{Button, Label, Widget};
use std::fs;

pub struct FilePicker {
    pub path: String,
    pub is_open: bool,
    pub selected_path: Option<String>,
    buttons: Vec<(Button, i32, i32)>,
    lbl_path: Label,
    scroll_offset: i32,
    btn_scroll_up: Button,
    btn_scroll_down: Button,
}

impl FilePicker {
    pub fn new() -> Self {
        let mut fp = Self {
            path: "/".to_string(),
            is_open: false,
            selected_path: None,
            buttons: Vec::new(),
            lbl_path: Label {
                text: "Path: /".to_string(),
                font_size: 16.0,
                color: (1.0, 1.0, 1.0, 1.0),
            },
            scroll_offset: 0,
            btn_scroll_up: Button::new("⬆", 30, 30),
            btn_scroll_down: Button::new("⬇", 30, 30),
        };
        fp.refresh_dir();
        fp
    }

    pub fn open(&mut self) {
        self.is_open = true;
        self.selected_path = None;
        self.scroll_offset = 0;
        self.refresh_dir();
    }

    pub fn close(&mut self) {
        self.is_open = false;
    }

    fn refresh_dir(&mut self) {
        self.lbl_path.text = format!("Path: {}", self.path);
        self.buttons.clear();

        let mut y_offset = 70;

        // Up button if not root
        if self.path != "/" {
            let btn = Button::new("⬆️ ..", 150, 25);
            self.buttons.push((btn, 20, y_offset));
            y_offset += 30;
        }

        match fs::read_dir(&self.path) {
            Ok(entries) => {
                for entry in entries {
                    if let Ok(entry) = entry {
                        let name = entry.file_name().into_string().unwrap_or_default();
                        let is_dir = entry.file_type().map(|t| t.is_dir()).unwrap_or(false);
                        let icon = if is_dir { "📁" } else { "📄" };
                        
                        let btn = Button::new(&format!("{} {}", icon, name), 250, 25);
                        self.buttons.push((btn, 20, y_offset));
                        y_offset += 30;
                    }
                }
            }
            Err(_e) => {}
        }
    }

    fn handle_click(&mut self, text: &str) -> bool {
        if text.starts_with("⬆️") {
            let mut parts: Vec<&str> = self.path.split('/').filter(|s| !s.is_empty()).collect();
            if !parts.is_empty() {
                parts.pop();
            }
            if parts.is_empty() {
                self.path = "/".to_string();
            } else {
                self.path = format!("/{}", parts.join("/"));
            }
            self.refresh_dir();
            true
        } else if text.starts_with("📁") {
            let dir_name = &text[5..]; // skip "📁 "
            if self.path == "/" {
                self.path = format!("/{}", dir_name);
            } else {
                self.path = format!("{}/{}", self.path, dir_name);
            }
            self.scroll_offset = 0;
            self.refresh_dir();
            true
        } else if text.starts_with("📄") {
            let file_name = &text[5..];
            let full_path = if self.path == "/" {
                format!("/{}", file_name)
            } else {
                format!("{}/{}", self.path, file_name)
            };
            self.selected_path = Some(full_path);
            self.close();
            true
        } else {
            false
        }
    }
}

impl Widget for FilePicker {
    fn draw(&self, x: i32, y: i32) {
        if !self.is_open { return; }

        unsafe {
            // Draw modal background
            crate::draw_rect_js(x as f32, y as f32, 360.0, 260.0, 0.1, 0.1, 0.15, 0.95, 8.0, 10.0);
            
            // Draw header
            crate::draw_rect_js(x as f32, y as f32, 360.0, 40.0, 0.2, 0.2, 0.25, 1.0, 8.0, 0.0);
            crate::draw_text_js((x + 10) as f32, (y + 12) as f32, b"Open File".as_ptr(), 9, 16.0, 1.0, 1.0, 1.0, 1.0);
        }

        self.lbl_path.draw(x + 10, y + 45);
        
        self.btn_scroll_up.draw(x + 320, y + 50);
        self.btn_scroll_down.draw(x + 320, y + 220);

        for (btn, rel_x, rel_y) in &self.buttons {
            let final_y = y + *rel_y - self.scroll_offset;
            // Only draw if inside the scrollable area (approx y+70 to y+250)
            if final_y >= y + 70 && final_y <= y + 230 {
                btn.draw(x + *rel_x, final_y);
            }
        }
    }

    fn handle_mouse_move(&mut self, mx: i32, my: i32, x: i32, y: i32) -> bool {
        if !self.is_open { return false; }
        let mut redraw = false;
        redraw |= self.btn_scroll_up.handle_mouse_move(mx, my, x + 320, y + 50);
        redraw |= self.btn_scroll_down.handle_mouse_move(mx, my, x + 320, y + 220);
        
        for (btn, rel_x, rel_y) in &mut self.buttons {
            let final_y = y + *rel_y - self.scroll_offset;
            if final_y >= y + 70 && final_y <= y + 230 {
                redraw |= btn.handle_mouse_move(mx, my, x + *rel_x, final_y);
            } else {
                btn.is_hovered = false; // reset hover if scrolled out of view
            }
        }
        redraw
    }

    fn handle_mouse_down(&mut self, mx: i32, my: i32, x: i32, y: i32) -> bool {
        if !self.is_open { return false; }
        let mut redraw = false;
        redraw |= self.btn_scroll_up.handle_mouse_down(mx, my, x + 320, y + 50);
        redraw |= self.btn_scroll_down.handle_mouse_down(mx, my, x + 320, y + 220);
        
        for (btn, rel_x, rel_y) in &mut self.buttons {
            let final_y = y + *rel_y - self.scroll_offset;
            if final_y >= y + 70 && final_y <= y + 230 {
                redraw |= btn.handle_mouse_down(mx, my, x + *rel_x, final_y);
            }
        }
        redraw
    }

    fn handle_mouse_up(&mut self, mx: i32, my: i32, x: i32, y: i32) -> bool {
        if !self.is_open { return false; }
        let mut clicked_text = None;
        let mut redraw = false;
        
        let up_bx = x + 320;
        let up_by = y + 50;
        if self.btn_scroll_up.is_pressed && mx >= up_bx && mx <= up_bx + self.btn_scroll_up.w && my >= up_by && my <= up_by + self.btn_scroll_up.h {
            self.scroll_offset = (self.scroll_offset - 60).max(0);
            self.btn_scroll_up.is_pressed = false;
            redraw = true;
        }
        redraw |= self.btn_scroll_up.handle_mouse_up(mx, my, up_bx, up_by);

        let dn_bx = x + 320;
        let dn_by = y + 220;
        if self.btn_scroll_down.is_pressed && mx >= dn_bx && mx <= dn_bx + self.btn_scroll_down.w && my >= dn_by && my <= dn_by + self.btn_scroll_down.h {
            // max scroll = max(0, buttons.len() * 30 - 150)
            let max_scroll = (self.buttons.len() as i32 * 30 - 150).max(0);
            self.scroll_offset = (self.scroll_offset + 60).min(max_scroll);
            self.btn_scroll_down.is_pressed = false;
            redraw = true;
        }
        redraw |= self.btn_scroll_down.handle_mouse_up(mx, my, dn_bx, dn_by);
        
        for (btn, rel_x, rel_y) in &mut self.buttons {
            let final_y = y + *rel_y - self.scroll_offset;
            if final_y >= y + 70 && final_y <= y + 230 {
                let bx = x + *rel_x;
                let by = final_y;
                if btn.is_pressed && mx >= bx && mx <= bx + btn.w && my >= by && my <= by + btn.h {
                    clicked_text = Some(btn.text.clone());
                    btn.is_pressed = false;
                    redraw = true;
                    break;
                }
                redraw |= btn.handle_mouse_up(mx, my, bx, by);
            } else {
                btn.is_pressed = false; // Cancel click if scrolled away
            }
        }
        
        if let Some(text) = clicked_text {
            self.handle_click(&text);
            return true;
        }
        redraw
    }
}
