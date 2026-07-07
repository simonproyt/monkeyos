use libui::window::Window;
use libui::{Button, Label, Widget};

static mut STATE: i32 = 0;
static mut BUTTON_ADD: Option<Button> = None;
static mut LABEL_STATE: Option<Label> = None;
static mut WINDOW: Option<Window> = None;

#[no_mangle]
pub extern "C" fn init() {
    unsafe {
        WINDOW = Some(Window::new("Calculator", 50, 50, 200, 150));
        BUTTON_ADD = Some(Button::new("Add 1", 100, 40));
        LABEL_STATE = Some(Label {
            text: "State: 0".to_string(),
            font_size: 20.0,
            color: (1.0, 1.0, 1.0, 1.0),
        });
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
            
            if let Some(lbl) = &mut LABEL_STATE {
                lbl.text = format!("State: {}", STATE);
                lbl.draw(win.x + 20, win.y + 20); // changed from 50 to 20
            }
            if let Some(btn) = &mut BUTTON_ADD {
                btn.draw(win.x + 20, win.y + 60); // changed from 90 to 60
            }
        }
    }
}

#[no_mangle]
pub extern "C" fn handle_mouse_move(mx: i32, my: i32) -> i32 {
    unsafe {
        let mut redraw = false;
        if let Some(win) = &mut WINDOW {
            redraw |= win.handle_mouse_move(mx, my);
            
            if let Some(btn) = &mut BUTTON_ADD {
                redraw |= btn.handle_mouse_move(mx, my, win.x + 20, win.y + 60);
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
            redraw |= win.handle_mouse_down(mx, my);
            
            if let Some(btn) = &mut BUTTON_ADD {
                redraw |= btn.handle_mouse_down(mx, my, win.x + 20, win.y + 60);
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
            redraw |= win.handle_mouse_up();
            
            if let Some(btn) = &mut BUTTON_ADD {
                let inside = mx >= win.x + 20 && mx <= win.x + 120 && my >= win.y + 60 && my <= win.y + 100;
                if btn.is_pressed && inside {
                    STATE += 1;
                }
                redraw |= btn.handle_mouse_up(mx, my, win.x + 20, win.y + 60);
            }
        }
        if redraw { 1 } else { 0 }
    }
}
