#![no_main]
#![allow(static_mut_refs)]

use libui::{Button, Label, Widget, window::Window, picker::FilePicker};
use std::fs;

struct Notepad {
    win: Window,
    btn_open: Button,
    btn_save: Button,
    btn_save_as: Button,
    lbl_status: Label,
    picker: FilePicker,
    text: Vec<String>,
    current_file: Option<String>,
    cursor_row: usize,
    cursor_col: usize,
    tick_count: u32,
    is_saving_as: bool,
    save_as_filename: String,
    last_typing_time: u128,
}

static mut NOTEPAD: Option<Notepad> = None;

#[no_mangle]
pub extern "C" fn init() {
    let win = Window::new("Notepad", 200, 150, 500, 400);

    let app = Notepad {
        win,
        btn_open: Button::new("📂 Open", 80, 25),
        btn_save: Button::new("💾 Save", 80, 25),
        btn_save_as: Button::new("📝 Save As", 100, 25),
        lbl_status: Label {
            text: "No file opened".to_string(),
            font_size: 14.0,
            color: (0.8, 0.8, 0.8, 1.0),
        },
        picker: FilePicker::new(),
        text: vec![String::new()],
        current_file: None,
        cursor_row: 0,
        cursor_col: 0,
        tick_count: 0,
        is_saving_as: false,
        save_as_filename: String::new(),
        last_typing_time: 0,
    };

    unsafe {
        NOTEPAD = Some(app);
    }
}

#[no_mangle]
pub extern "C" fn tick(x: i32, y: i32, w: i32, h: i32) {
    let app = unsafe { NOTEPAD.as_mut().unwrap() };
    app.win.x = x;
    app.win.y = y;
    app.win.w = w;
    app.win.h = h;

    app.win.draw_background(0.1, 0.1, 0.12);

    let content_y = y + 25; // Account for wm.rs title bar

    // Draw top bar
    unsafe {
        libui::draw_rect_js(x as f32, content_y as f32, w as f32, 40.0, 0.2, 0.2, 0.25, 1.0, 0.0, 0.0);
    }

    app.btn_open.draw(x + 10, content_y + 8);
    app.btn_save.draw(x + 100, content_y + 8);
    app.btn_save_as.draw(x + 190, content_y + 8);
    app.lbl_status.draw(x + 300, content_y + 12);

    app.tick_count = app.tick_count.wrapping_add(1);

    // Draw text
    let mut text_y = content_y + 50;
    for (i, line) in app.text.iter().enumerate() {
        if text_y > y + h { break; }
        unsafe {
            libui::draw_text_js(
                (x + 10) as f32,
                text_y as f32,
                line.as_ptr(),
                line.len(),
                16.0,
                0.9, 0.9, 0.9, 1.0
            );
        }
        
        // Draw cursor if it's on this line and blinking is ON
        let time_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis();
            
        if !app.is_saving_as && i == app.cursor_row && ((time_ms % 1000) < 500 || time_ms.saturating_sub(app.last_typing_time) < 500) {
            let cx = x as f32 + 10.0 + (app.cursor_col as f32 * 8.4); // 8.4px char width for 14px monospace
            unsafe {
                libui::draw_rect_js(cx, text_y as f32, 2.0, 14.0, 1.0, 1.0, 1.0, 0.8, 0.0, 0.0);
            }
        }
        
        text_y += 20;
    }

    // Draw picker on top if open
    if app.picker.is_open {
        app.picker.draw(x + 70, content_y + 50);
    }
    
    // Draw Save As modal if open
    if app.is_saving_as {
        app.picker.is_open = true; // Ensure picker is open
        
        // If they click a file, populate the name and keep picker open
        if let Some(path) = app.picker.selected_path.take() {
            if let Some(name) = path.split('/').last() {
                app.save_as_filename = name.to_string();
            }
        }
        
        // Draw the Save As input box attached to the bottom of the FilePicker
        let mx = x + 70;
        let my = content_y + 50 + 260; // Just below the picker
        unsafe {
            libui::draw_rect_js(mx as f32, my as f32, 360.0, 60.0, 0.15, 0.15, 0.18, 0.95, 0.0, 10.0);
            libui::draw_text_js(mx as f32 + 10.0, my as f32 + 10.0, "Save As:".as_ptr(), 8, 14.0, 1.0, 1.0, 1.0, 1.0);
            
            // Draw text box
            libui::draw_rect_js(mx as f32 + 80.0, my as f32 + 5.0, 270.0, 25.0, 0.1, 0.1, 0.12, 1.0, 4.0, 0.0);
            libui::draw_text_js(mx as f32 + 85.0, my as f32 + 10.0, app.save_as_filename.as_ptr(), app.save_as_filename.len(), 14.0, 1.0, 1.0, 1.0, 1.0);
            
            // Draw hints
            libui::draw_text_js(mx as f32 + 10.0, my as f32 + 40.0, "(Press Enter to save to current folder)".as_ptr(), 39, 12.0, 0.6, 0.6, 0.6, 1.0);
            
            // Cursor for the text box
            let time_ms = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_millis();
            if (time_ms % 1000) < 500 || time_ms.saturating_sub(app.last_typing_time) < 500 {
                let cx = mx as f32 + 85.0 + (app.save_as_filename.len() as f32 * 8.4);
                libui::draw_rect_js(cx, my as f32 + 10.0, 2.0, 14.0, 1.0, 1.0, 1.0, 0.8, 0.0, 0.0);
            }
        }
    }
}

#[no_mangle]
pub extern "C" fn handle_mouse_move(mx: i32, my: i32) -> i32 {
    let app = unsafe { NOTEPAD.as_mut().unwrap() };
    let content_y = app.win.y + 25;
    if app.picker.is_open {
        return if app.picker.handle_mouse_move(mx, my, app.win.x + 70, content_y + 50) { 1 } else { 0 };
    }
    
    let mut redraw = false;
    redraw |= app.btn_open.handle_mouse_move(mx, my, app.win.x + 10, content_y + 8);
    redraw |= app.btn_save.handle_mouse_move(mx, my, app.win.x + 100, content_y + 8);
    redraw |= app.btn_save_as.handle_mouse_move(mx, my, app.win.x + 190, content_y + 8);
    if redraw { 1 } else { 0 }
}

#[no_mangle]
pub extern "C" fn handle_mouse_down(mx: i32, my: i32) -> i32 {
    let app = unsafe { NOTEPAD.as_mut().unwrap() };
    let content_y = app.win.y + 25;
    if app.picker.is_open {
        return if app.picker.handle_mouse_down(mx, my, app.win.x + 70, content_y + 50) { 1 } else { 0 };
    }
    
    let mut redraw = false;
    redraw |= app.btn_open.handle_mouse_down(mx, my, app.win.x + 10, content_y + 8);
    redraw |= app.btn_save.handle_mouse_down(mx, my, app.win.x + 100, content_y + 8);
    redraw |= app.btn_save_as.handle_mouse_down(mx, my, app.win.x + 190, content_y + 8);
    if redraw { 1 } else { 0 }
}

#[no_mangle]
pub extern "C" fn handle_mouse_up(mx: i32, my: i32) -> i32 {
    let app = unsafe { NOTEPAD.as_mut().unwrap() };
    let content_y = app.win.y + 25;
    
    if app.picker.is_open {
        let mut redraw = app.picker.handle_mouse_up(mx, my, app.win.x + 70, content_y + 50);
        
        if app.is_saving_as {
            if !app.picker.is_open {
                // If picker closed during Save As, they clicked a file. Grab its name.
                if let Some(path) = app.picker.selected_path.take() {
                    if let Some(name) = path.split('/').last() {
                        app.save_as_filename = name.to_string();
                    }
                }
                app.picker.is_open = true; // reopen
                redraw = true;
            }
            return if redraw { 1 } else { 0 };
        } else {
            if !app.picker.is_open {
                if let Some(path) = app.picker.selected_path.take() {
                    app.current_file = Some(path.clone());
                    if let Ok(contents) = fs::read_to_string(&path) {
                        app.text = contents.lines().map(|s| s.to_string()).collect();
                        if app.text.is_empty() {
                            app.text.push(String::new());
                        }
                        app.lbl_status.text = format!("Opened {}", path);
                    } else {
                        app.lbl_status.text = format!("Error opening {}", path);
                    }
                }
            }
            return if redraw { 1 } else { 0 };
        }
    }
    
    let mut redraw = false;
    
    let bx_open = app.win.x + 10;
    let by_open = content_y + 8;
    if app.btn_open.is_pressed && mx >= bx_open && mx <= bx_open + app.btn_open.w && my >= by_open && my <= by_open + app.btn_open.h {
        app.picker.open();
        app.btn_open.is_pressed = false;
        redraw = true;
    }
    redraw |= app.btn_open.handle_mouse_up(mx, my, bx_open, by_open);

    let bx_save = app.win.x + 100;
    let by_save = content_y + 8;
    
    let bx_save_as = app.win.x + 190;
    let by_save_as = content_y + 8;
    if app.btn_save.is_pressed && mx >= bx_save && mx <= bx_save + app.btn_save.w && my >= by_save && my <= by_save + app.btn_save.h {
        if let Some(path) = &app.current_file {
            let contents = app.text.join("\n");
            let _ = fs::write(path, contents);
            app.lbl_status.text = format!("Saved {}", path);
        } else {
            app.lbl_status.text = "No file to save!".to_string();
        }
        app.btn_save.is_pressed = false;
        redraw = true;
    }
    redraw |= app.btn_save.handle_mouse_up(mx, my, bx_save, by_save);
    
    if app.btn_save_as.is_pressed && mx >= bx_save_as && mx <= bx_save_as + app.btn_save_as.w && my >= by_save_as && my <= by_save_as + app.btn_save_as.h {
        app.is_saving_as = true;
        app.picker.open();
        app.save_as_filename.clear();
        app.btn_save_as.is_pressed = false;
        redraw = true;
    }
    redraw |= app.btn_save_as.handle_mouse_up(mx, my, bx_save_as, by_save_as);

    if redraw { 1 } else { 0 }
}

#[no_mangle]
pub extern "C" fn handle_key_down(key_code: u32) -> i32 {
    let app = unsafe { NOTEPAD.as_mut().unwrap() };
    if app.picker.is_open && !app.is_saving_as { return 0; }

    app.last_typing_time = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_millis();
    
    if app.is_saving_as {
        match key_code {
            13 => { // Enter
                if !app.save_as_filename.is_empty() {
                    let full_path = if app.picker.path == "/" {
                        format!("/{}", app.save_as_filename)
                    } else {
                        format!("{}/{}", app.picker.path, app.save_as_filename)
                    };
                    let content = app.text.join("\n");
                    let _ = fs::write(&full_path, content);
                    app.current_file = Some(full_path.clone());
                    app.lbl_status.text = format!("Saved: {}", full_path);
                }
                app.is_saving_as = false;
                app.picker.close();
            }
            8 => { // Backspace
                app.save_as_filename.pop();
            }
            17 | 27 => { // Ctrl+Q / Escape hack
                app.is_saving_as = false;
                app.picker.close();
            }
            c if c >= 32 && c <= 126 => {
                app.save_as_filename.push(c as u8 as char);
            }
            _ => {}
        }
        return 1;
    }

    app.tick_count = 0; // Keep this just in case anything else relies on it
    let row = app.cursor_row;
    let col = app.cursor_col;

    match key_code {
        1037 => { // Left
            if app.cursor_col > 0 {
                app.cursor_col -= 1;
            } else if app.cursor_row > 0 {
                app.cursor_row -= 1;
                app.cursor_col = app.text[app.cursor_row].len();
            }
        }
        1039 => { // Right
            if app.cursor_col < app.text[app.cursor_row].len() {
                app.cursor_col += 1;
            } else if app.cursor_row < app.text.len() - 1 {
                app.cursor_row += 1;
                app.cursor_col = 0;
            }
        }
        1038 => { // Up
            if app.cursor_row > 0 {
                app.cursor_row -= 1;
                app.cursor_col = app.cursor_col.min(app.text[app.cursor_row].len());
            }
        }
        1040 => { // Down
            if app.cursor_row < app.text.len() - 1 {
                app.cursor_row += 1;
                app.cursor_col = app.cursor_col.min(app.text[app.cursor_row].len());
            }
        }
        8 => { // Backspace
            if col > 0 {
                app.text[row].remove(col - 1);
                app.cursor_col -= 1;
            } else if row > 0 {
                let current_line = app.text.remove(row);
                app.cursor_row -= 1;
                app.cursor_col = app.text[row - 1].len();
                app.text[row - 1].push_str(&current_line);
            }
        }
        13 => { // Enter
            let remainder = app.text[row].split_off(col);
            app.text.insert(row + 1, remainder);
            app.cursor_row += 1;
            app.cursor_col = 0;
        }
        c if c >= 32 && c <= 126 => {
            app.text[row].insert(col, c as u8 as char);
            app.cursor_col += 1;
        }
        _ => return 0,
    }
    
    app.lbl_status.text = "Edited".to_string();
    1
}
