#![allow(static_mut_refs)]

use libui::window::Window;
use libui::{Button, Label, Widget};
use std::fs;

static mut WINDOW: Option<Window> = None;
static mut LABEL_DISPLAY: Option<Label> = None;
static mut BUTTONS: Option<Vec<(Button, f64, f64)>> = None; // (Button, rel_x, rel_y)

#[no_mangle]
pub extern "C" fn init() {
    unsafe {
        WINDOW = Some(Window::new("Settings", 100, 100, 300, 200));
        LABEL_DISPLAY = Some(Label {
            text: "Choose Background API:".to_string(),
            font_size: 16.0,
            color: (1.0, 1.0, 1.0, 1.0),
        });
        
        let mut btns = Vec::new();
        btns.push((Button::new("NASA APOD", 260, 30), 20.0, 60.0));
        btns.push((Button::new("Picsum Random", 260, 30), 20.0, 100.0));
        btns.push((Button::new("MonkeyOS Default", 260, 30), 20.0, 140.0));
        BUTTONS = Some(btns);
    }
}

fn handle_btn_click(text: &str) {
    match text {
        "NASA APOD" => {
            let _ = fs::write("/etc/background.txt", "NASA_APOD");
        }
        "Picsum Random" => {
            let _ = fs::write("/etc/background.txt", "https://picsum.photos/1920/1080?blur=2");
        }
        "MonkeyOS Default" => {
            let _ = fs::write("/etc/background.txt", "DEFAULT");
        }
        _ => {}
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
            
            win.draw_background(0.1, 0.1, 0.15);
            if let Some(lbl) = &LABEL_DISPLAY {
                lbl.draw(win.x + 20, win.y + 30);
            }
            if let Some(btns) = &BUTTONS {
                for (btn, rel_x, rel_y) in btns.iter() {
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
        if let Some(win) = &mut WINDOW {
            if let Some(btns) = &mut BUTTONS {
                for (btn, rel_x, rel_y) in btns.iter_mut() {
                    let bx = win.x + *rel_x as i32;
                    let by = win.y + *rel_y as i32;
                    if btn.is_pressed && mx >= bx && mx <= bx + btn.w && my >= by && my <= by + btn.h {
                        handle_btn_click(&btn.text);
                        redraw = true;
                    }
                    redraw |= btn.handle_mouse_up(mx, my, bx, by);
                }
            }
        }
        if redraw { 1 } else { 0 }
    }
}

