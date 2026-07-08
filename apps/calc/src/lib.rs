#![allow(static_mut_refs)]

use libui::window::Window;
use libui::{Button, Label, Widget};

static mut WINDOW: Option<Window> = None;
static mut LABEL_DISPLAY: Option<Label> = None;
static mut BUTTONS: Option<Vec<(Button, f64, f64)>> = None; // (Button, rel_x, rel_y)

static mut CURRENT_VALUE: String = String::new();
static mut PREVIOUS_VALUE: f64 = 0.0;
static mut OPERATOR: Option<char> = None;

#[no_mangle]
pub extern "C" fn init() {
    unsafe {
        WINDOW = Some(Window::new("Calculator", 50, 50, 240, 320));
        LABEL_DISPLAY = Some(Label {
            text: "0".to_string(),
            font_size: 24.0,
            color: (1.0, 1.0, 1.0, 1.0),
        });
        
        let mut btns = Vec::new();
        let layout = [
            ("7", 10.0, 70.0), ("8", 65.0, 70.0), ("9", 120.0, 70.0), ("/", 175.0, 70.0),
            ("4", 10.0, 125.0), ("5", 65.0, 125.0), ("6", 120.0, 125.0), ("*", 175.0, 125.0),
            ("1", 10.0, 180.0), ("2", 65.0, 180.0), ("3", 120.0, 180.0), ("-", 175.0, 180.0),
            ("C", 10.0, 235.0), ("0", 65.0, 235.0), ("=", 120.0, 235.0), ("+", 175.0, 235.0),
        ];

        for (text, x, y) in layout.iter() {
            let btn = Button::new(text, 45, 45);
            btns.push((btn, *x, *y));
        }
        BUTTONS = Some(btns);
        CURRENT_VALUE = "0".to_string();
    }
}

fn handle_btn_click(text: &str) {
    unsafe {
        match text {
            "C" => {
                CURRENT_VALUE = "0".to_string();
                PREVIOUS_VALUE = 0.0;
                OPERATOR = None;
            }
            "+" | "-" | "*" | "/" => {
                if let Ok(val) = CURRENT_VALUE.parse::<f64>() {
                    PREVIOUS_VALUE = val;
                }
                OPERATOR = Some(text.chars().next().unwrap());
                CURRENT_VALUE = "0".to_string();
            }
            "=" => {
                if let (Ok(current), Some(op)) = (CURRENT_VALUE.parse::<f64>(), OPERATOR) {
                    let result = match op {
                        '+' => PREVIOUS_VALUE + current,
                        '-' => PREVIOUS_VALUE - current,
                        '*' => PREVIOUS_VALUE * current,
                        '/' => if current != 0.0 { PREVIOUS_VALUE / current } else { 0.0 },
                        _ => current,
                    };
                    CURRENT_VALUE = result.to_string();
                    OPERATOR = None;
                }
            }
            _ => {
                if CURRENT_VALUE == "0" {
                    CURRENT_VALUE = text.to_string();
                } else {
                    CURRENT_VALUE.push_str(text);
                }
            }
        }
        if let Some(lbl) = &mut LABEL_DISPLAY {
            lbl.text = CURRENT_VALUE.clone();
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
            
            win.draw_background(0.2, 0.2, 0.25);
            
            if let Some(lbl) = &mut LABEL_DISPLAY {
                lbl.draw(win.x + 20, win.y + 30);
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
