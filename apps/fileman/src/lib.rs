#![no_main]
#![allow(static_mut_refs)]

use libui::window::Window;
use libui::{Button, Label, Widget};
use std::fs;

#[link(wasm_import_module = "env")]
extern "C" {
    fn sys_execve(args_ptr: *const u8, args_len: usize, cwd_ptr: *const u8, cwd_len: usize, stdin_ptr: *const u8, stdin_len: usize, stdout_ptr: *const u8, stdout_len: usize, terminal_id: u32) -> i32;
}


static mut WINDOW: Option<Window> = None;
static mut LBL_PATH: Option<Label> = None;
static mut BUTTONS: Option<Vec<(Button, f64, f64)>> = None;
static mut CURRENT_PATH: String = String::new();
static mut SCROLL_OFFSET: i32 = 0;
static mut BTN_SCROLL_UP: Option<Button> = None;
static mut BTN_SCROLL_DOWN: Option<Button> = None;

#[no_mangle]
pub extern "C" fn init() {
    unsafe {
        WINDOW = Some(Window::new("File Manager", 100, 100, 400, 500));
        CURRENT_PATH = "/".to_string();
        BTN_SCROLL_UP = Some(Button::new("⬆", 30, 30));
        BTN_SCROLL_DOWN = Some(Button::new("⬇", 30, 30));
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
        SCROLL_OFFSET = 0;

        // Up button if not root
        let mut y_offset = 60.0;
        if CURRENT_PATH != "/" {
            let btn = Button::new("⬆️ ..", 380, 35);
            btns.push((btn, 10.0, y_offset));
            y_offset += 40.0;
        }

        match fs::read_dir(&CURRENT_PATH) {
            Ok(entries) => {
                for entry in entries {
                    if let Ok(entry) = entry {
                        let name = entry.file_name().into_string().unwrap_or_default();
                        let is_dir = entry.file_type().map(|t| t.is_dir()).unwrap_or(false);
                        let icon = if is_dir { "📁" } else { "📄" };
                        
                        let btn = Button::new(&format!("{} {}", icon, name), 380, 35);
                        btns.push((btn, 10.0, y_offset));
                        y_offset += 40.0;
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
            SCROLL_OFFSET = 0;
            refresh_dir();
        } else if text.starts_with("📄") {
            let file_name = &text[5..];
            let full_path = if CURRENT_PATH == "/" {
                format!("/{}", file_name)
            } else {
                format!("{}/{}", CURRENT_PATH, file_name)
            };
            
            // Launch notepad with the file path
            let mut args_buf = Vec::new();
            args_buf.extend_from_slice(b"/bin/notepad\0");
            args_buf.extend_from_slice(full_path.as_bytes());
            args_buf.push(0);
            
            sys_execve(
                args_buf.as_ptr(), args_buf.len(),
                CURRENT_PATH.as_ptr(), CURRENT_PATH.len(),
                std::ptr::null(), 0,
                std::ptr::null(), 0,
                0
            );
        }
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
                lbl.draw(win.x + 10, win.y + 30);
            }
            if let Some(btns) = &mut BUTTONS {
                for (btn, rel_x, rel_y) in btns.iter_mut() {
                    let final_y = win.y + *rel_y as i32 - SCROLL_OFFSET;
                    // Clip file items slightly above the bottom scroll button and below the top header
                    if final_y >= win.y + 30 && final_y <= win.y + win.h - 60 {
                        btn.draw(win.x + *rel_x as i32, final_y);
                    }
                }
            }

            if let Some(btn) = &mut BTN_SCROLL_UP { btn.draw(win.x + win.w - 40, win.y + 35); }
            if let Some(btn) = &mut BTN_SCROLL_DOWN { btn.draw(win.x + win.w - 40, win.y + win.h - 50); }
        }
    }
}

#[no_mangle]
pub extern "C" fn handle_mouse_move(mx: i32, my: i32) -> i32 {
    unsafe {
        let mut redraw = false;
        if let Some(win) = &mut WINDOW {
            if let Some(btn) = &mut BTN_SCROLL_UP { redraw |= btn.handle_mouse_move(mx, my, win.x + win.w - 40, win.y + 35); }
            if let Some(btn) = &mut BTN_SCROLL_DOWN { redraw |= btn.handle_mouse_move(mx, my, win.x + win.w - 40, win.y + win.h - 50); }
            if let Some(btns) = &mut BUTTONS {
                for (btn, rel_x, rel_y) in btns.iter_mut() {
                    let final_y = win.y + *rel_y as i32 - SCROLL_OFFSET;
                    if final_y >= win.y + 30 && final_y <= win.y + win.h - 60 {
                        redraw |= btn.handle_mouse_move(mx, my, win.x + *rel_x as i32, final_y);
                    } else {
                        btn.is_hovered = false;
                    }
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
            if let Some(btn) = &mut BTN_SCROLL_UP { redraw |= btn.handle_mouse_down(mx, my, win.x + win.w - 40, win.y + 35); }
            if let Some(btn) = &mut BTN_SCROLL_DOWN { redraw |= btn.handle_mouse_down(mx, my, win.x + win.w - 40, win.y + win.h - 50); }
            if let Some(btns) = &mut BUTTONS {
                for (btn, rel_x, rel_y) in btns.iter_mut() {
                    let final_y = win.y + *rel_y as i32 - SCROLL_OFFSET;
                    if final_y >= win.y + 30 && final_y <= win.y + win.h - 60 {
                        redraw |= btn.handle_mouse_down(mx, my, win.x + *rel_x as i32, final_y);
                    }
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
            let bx_up = win.x + win.w - 40;
            let by_up = win.y + 35;
            if let Some(btn) = &mut BTN_SCROLL_UP {
                if btn.is_pressed && mx >= bx_up && mx <= bx_up + btn.w && my >= by_up && my <= by_up + btn.h {
                    SCROLL_OFFSET = (SCROLL_OFFSET - 80).max(0);
                    btn.is_pressed = false;
                    redraw = true;
                }
                redraw |= btn.handle_mouse_up(mx, my, bx_up, by_up);
            }
            
            let bx_dn = win.x + win.w - 40;
            let by_dn = win.y + win.h - 50;
            if let Some(btn) = &mut BTN_SCROLL_DOWN {
                if btn.is_pressed && mx >= bx_dn && mx <= bx_dn + btn.w && my >= by_dn && my <= by_dn + btn.h {
                    let max_scroll = if let Some(btns) = &BUTTONS {
                        (60 + btns.len() as i32 * 40 - (win.h - 60)).max(0)
                    } else { 0 };
                    SCROLL_OFFSET = (SCROLL_OFFSET + 80).min(max_scroll);
                    btn.is_pressed = false;
                    redraw = true;
                }
                redraw |= btn.handle_mouse_up(mx, my, bx_dn, by_dn);
            }

            if let Some(btns) = &mut BUTTONS {
                for (btn, rel_x, rel_y) in btns.iter_mut() {
                    let final_y = win.y + *rel_y as i32 - SCROLL_OFFSET;
                    if final_y >= win.y + 30 && final_y <= win.y + win.h - 60 {
                        let bx = win.x + *rel_x as i32;
                        let by = final_y;
                        if btn.is_pressed && mx >= bx && mx <= bx + btn.w && my >= by && my <= by + btn.h {
                            clicked_text = Some(btn.text.clone());
                            btn.is_pressed = false;
                            redraw = true;
                            break;
                        }
                        redraw |= btn.handle_mouse_up(mx, my, bx, by);
                    } else {
                        btn.is_pressed = false;
                    }
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
