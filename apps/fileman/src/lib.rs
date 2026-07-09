#![no_main]
#![allow(static_mut_refs)]

use libui::window::Window;
use libui::{Button, Label, Widget};
use std::fs;

static mut WINDOW: Option<Window> = None;
static mut LBL_PATH: Option<Label> = None;
static mut BUTTONS: Option<Vec<(Button, f64, f64)>> = None;
static mut CURRENT_PATH: String = String::new();

#[no_mangle]
pub extern "C" fn init() {
    unsafe {
        WINDOW = Some(Window::new("File Manager", 100, 100, 400, 300));
        CURRENT_PATH = "/".to_string();
        refresh_dir();
    }
}

fn refresh_dir() {
    unsafe {
        LBL_PATH = Some(Label {
            text: format!("Path: {}", CURRENT_PATH),
            font_size: 16.0,
            color: (1.0, 1.0, 1.0, 1.0),
        });

        let mut btns = Vec::new();

        // Up button if not root
        let mut y_offset = 30.0;
        if CURRENT_PATH != "/" {
            let btn = Button::new("⬆️ ..", 150, 25);
            btns.push((btn, 10.0, y_offset));
            y_offset += 30.0;
        }

        match fs::read_dir(&CURRENT_PATH) {
            Ok(entries) => {
                for entry in entries {
                    if let Ok(entry) = entry {
                        let name = entry.file_name().into_string().unwrap_or_default();
                        let is_dir = entry.file_type().map(|t| t.is_dir()).unwrap_or(false);
                        let icon = if is_dir { "📁" } else { "📄" };
                        
                        let btn = Button::new(&format!("{} {}", icon, name), 250, 25);
                        btns.push((btn, 10.0, y_offset));
                        y_offset += 30.0;
                    }
                }
            }
            Err(_e) => {}
        }
        BUTTONS = Some(btns);
    }
}

fn handle_btn_click(text: &str) {
    unsafe {
        if text.starts_with("⬆️") {
            let mut parts: Vec<&str> = CURRENT_PATH.split('/').filter(|s| !s.is_empty()).collect();
            if !parts.is_empty() {
                parts.pop();
            }
            if parts.is_empty() {
                CURRENT_PATH = "/".to_string();
            } else {
                CURRENT_PATH = format!("/{}", parts.join("/"));
            }
            refresh_dir();
        } else if text.starts_with("📁") {
            let dir_name = &text[5..]; // skip "📁 " (emoji is 4 bytes + 1 space = 5)
            if CURRENT_PATH == "/" {
                CURRENT_PATH = format!("/{}", dir_name);
            } else {
                CURRENT_PATH = format!("{}/{}", CURRENT_PATH, dir_name);
            }
            refresh_dir();
        }
        // File clicks not fully supported yet
    }
}

#[no_mangle]
pub extern "C" fn tick(x: i32, y: i32, w: i32, h: i32) {
    unsafe {
        if let Some(win) = &mut WINDOW {
            win.x = x;
            win.y = y;
            win.w = w;
            win.h = h;
            
            win.draw_background(0.15, 0.15, 0.2);
            
            if let Some(lbl) = &mut LBL_PATH {
                lbl.draw(win.x + 10, win.y + 20);
            }
            
            if let Some(btns) = &mut BUTTONS {
                for (btn, rel_x, rel_y) in btns.iter_mut() {
                    btn.draw(win.x + *rel_x as i32, win.y + *rel_y as i32);
                }
            }
        }
    }
}

#[no_mangle]
pub extern "C" fn handle_mouse_move(mx: i32, my: i32) -> i32 {
    unsafe {
        let mut redraw = false;
        if let Some(win) = &mut WINDOW {
            if let Some(btns) = &mut BUTTONS {
                for (btn, rel_x, rel_y) in btns.iter_mut() {
                    redraw |= btn.handle_mouse_move(mx, my, win.x + *rel_x as i32, win.y + *rel_y as i32);
                }
            }
        }
        if redraw { 1 } else { 0 }
    }
}

#[no_mangle]
pub extern "C" fn handle_mouse_down(mx: i32, my: i32) -> i32 {
    unsafe {
        let mut redraw = false;
        if let Some(win) = &mut WINDOW {
            if let Some(btns) = &mut BUTTONS {
                for (btn, rel_x, rel_y) in btns.iter_mut() {
                    redraw |= btn.handle_mouse_down(mx, my, win.x + *rel_x as i32, win.y + *rel_y as i32);
                }
            }
        }
        if redraw { 1 } else { 0 }
    }
}

#[no_mangle]
pub extern "C" fn handle_mouse_up(mx: i32, my: i32) -> i32 {
    unsafe {
        let mut redraw = false;
        let mut clicked_text = None;
        if let Some(win) = &mut WINDOW {
            if let Some(btns) = &mut BUTTONS {
                for (btn, rel_x, rel_y) in btns.iter_mut() {
                    let bx = win.x + *rel_x as i32;
                    let by = win.y + *rel_y as i32;
                    if btn.is_pressed && mx >= bx && mx <= bx + btn.w && my >= by && my <= by + btn.h {
                        clicked_text = Some(btn.text.clone());
                        btn.is_pressed = false;
                        redraw = true;
                        break;
                    }
                    redraw |= btn.handle_mouse_up(mx, my, bx, by);
                }
            }
        }
        if let Some(text) = clicked_text {
            handle_btn_click(&text);
            return 1;
        }
        if redraw { 1 } else { 0 }
    }
}
