#![allow(static_mut_refs)]

use libui::{window::Window, Button, Widget, picker::FilePicker};

#[link(wasm_import_module = "env")]
unsafe extern "C" {
    fn sys_get_launch_arg(out_ptr: *mut u8, out_max_len: usize) -> usize;
}

fn get_launch_arg() -> Option<String> {
    let mut buf = [0u8; 512];
    let len = unsafe { sys_get_launch_arg(buf.as_mut_ptr(), buf.len()) };
    if len > 0 {
        Some(String::from_utf8_lossy(&buf[..len]).to_string())
    } else {
        None
    }
}

struct ImgView {
    win: Window,
    btn_open: Button,
    picker: FilePicker,
    image_loaded: bool,
}

static mut APP: Option<ImgView> = None;

#[no_mangle]
pub extern "C" fn init() {
    unsafe {
        let mut app = ImgView {
            win: Window::new("Image Viewer", 50, 50, 450, 350),
            btn_open: Button::new("🖼️ Open Image", 140, 30),
            picker: FilePicker::new(),
            image_loaded: false,
        };

        // Check if a file path was passed as a launch argument
        if let Some(path) = get_launch_arg() {
            let url_bytes = path.as_bytes();
            libui::load_image_js(1, url_bytes.as_ptr(), url_bytes.len());
            app.image_loaded = true;
        } else {
            // No file arg — open the file picker
            app.picker.open();
        }

        APP = Some(app);
    }
}

#[no_mangle]
pub extern "C" fn tick(x: i32, y: i32, w: i32, h: i32) {
    unsafe {
        if let Some(app) = &mut APP {
            app.win.x = x;
            app.win.y = y;
            app.win.w = w;
            app.win.h = h;
            
            app.win.draw_background(0.1, 0.1, 0.15);
            
            if app.image_loaded {
                libui::draw_image_js(1, app.win.x as f32, (app.win.y + 40) as f32, app.win.w as f32, (app.win.h - 40) as f32);
            }
            
            // Draw top bar
            libui::draw_rect_js(app.win.x as f32, app.win.y as f32, app.win.w as f32, 40.0, 0.15, 0.15, 0.2, 1.0, 0.0, 0.0);
            app.btn_open.draw(app.win.x + 10, app.win.y + 5);
            
            if app.picker.is_open {
                app.picker.draw(app.win.x + 20, app.win.y + 20);
            }
        }
    }
}

#[no_mangle]
pub extern "C" fn handle_mouse_move(mx: i32, my: i32) -> i32 {
    unsafe {
        if let Some(app) = &mut APP {
            let mut redraw = false;
            let win_x = app.win.x;
            let win_y = app.win.y;
            
            if app.picker.is_open {
                redraw |= app.picker.handle_mouse_move(mx, my, win_x + 20, win_y + 20);
            } else {
                redraw |= app.btn_open.handle_mouse_move(mx, my, win_x + 10, win_y + 5);
            }
            if redraw { return 1; }
        }
    }
    0
}

#[no_mangle]
pub extern "C" fn handle_mouse_down(mx: i32, my: i32) -> i32 {
    unsafe {
        if let Some(app) = &mut APP {
            let mut redraw = false;
            let win_x = app.win.x;
            let win_y = app.win.y;
            
            if app.picker.is_open {
                redraw |= app.picker.handle_mouse_down(mx, my, win_x + 20, win_y + 20);
            } else {
                redraw |= app.btn_open.handle_mouse_down(mx, my, win_x + 10, win_y + 5);
            }
            if redraw { return 1; }
        }
    }
    0
}

#[no_mangle]
pub extern "C" fn handle_mouse_up(mx: i32, my: i32) -> i32 {
    unsafe {
        if let Some(app) = &mut APP {
            let mut redraw = false;
            let win_x = app.win.x;
            let win_y = app.win.y;
            
            if app.picker.is_open {
                redraw |= app.picker.handle_mouse_up(mx, my, win_x + 20, win_y + 20);
                if let Some(path) = app.picker.selected_path.take() {
                    let url_bytes = path.as_bytes();
                    libui::load_image_js(1, url_bytes.as_ptr(), url_bytes.len());
                    app.image_loaded = true;
                    redraw = true;
                }
            } else {
                if app.btn_open.is_pressed {
                    let inside = mx >= win_x + 10 && mx <= win_x + 10 + app.btn_open.w && 
                                 my >= win_y + 5 && my <= win_y + 5 + app.btn_open.h;
                    app.btn_open.is_pressed = false;
                    redraw = true;
                    if inside {
                        app.picker.open();
                    }
                }
            }
            if redraw { return 1; }
        }
    }
    0
}
