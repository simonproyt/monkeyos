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
    save_as_cursor_pos: usize,
    last_typing_time: u128,
    scroll_x: f32,
    scroll_y: f32,
    is_dragging_v_scroll: bool,
    is_dragging_h_scroll: bool,
    needs_autoscroll: bool,
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
        save_as_cursor_pos: 0,
        last_typing_time: 0,
        scroll_x: 0.0,
        scroll_y: 0.0,
        is_dragging_v_scroll: false,
        is_dragging_h_scroll: false,
        needs_autoscroll: false,
    };

    unsafe {
        NOTEPAD = Some(app);
    }
    
    // Check if we were passed a file argument
    if let Some(path) = std::env::args().nth(1) {
        unsafe {
            if let Some(app) = NOTEPAD.as_mut() {
                app.current_file = Some(path.clone());
                if let Ok(contents) = std::fs::read_to_string(&path) {
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
    
    unsafe {
        let clip_w = (w - 385).max(0);
        if clip_w > 0 {
            libui::clip_text_js((x + 300) as f32, content_y as f32, clip_w as f32, 40.0);
            app.lbl_status.draw(x + 300, content_y + 12);
            libui::clear_clip_text_js();
        }
    }

    app.tick_count = app.tick_count.wrapping_add(1);

    // Auto-scroll to keep cursor visible (only when cursor is moved via typing)
    if app.needs_autoscroll && !app.is_dragging_v_scroll && !app.is_dragging_h_scroll && app.cursor_row < app.text.len() {
        let text_width = if app.cursor_col > 0 && app.cursor_col <= app.text[app.cursor_row].len() {
            unsafe { libui::measure_text_js(app.text[app.cursor_row].as_ptr(), app.cursor_col, 16.0) }
        } else {
            0.0
        };
        
        let target_x = text_width - (w as f32 - 40.0);
        if target_x > app.scroll_x { app.scroll_x = target_x; }
        if text_width < app.scroll_x { app.scroll_x = text_width; }
        
        let cursor_y = (app.cursor_row as f32) * 20.0;
        let target_y = cursor_y - (h as f32 - 90.0);
        if target_y > app.scroll_y { app.scroll_y = target_y; }
        if cursor_y < app.scroll_y { app.scroll_y = cursor_y; }
        app.needs_autoscroll = false;
    }

    // Clamp scroll values to valid boundaries
    let view_h = h as f32 - 90.0;
    let total_h = (app.text.len() as f32) * 20.0 + 20.0;
    app.scroll_y = app.scroll_y.clamp(0.0, (total_h - view_h).max(0.0));

    let view_w = w as f32 - 30.0;
    let longest_line_len = app.text.iter().map(|l| l.len()).max().unwrap_or(0);
    let total_w = longest_line_len as f32 * 9.6 + 20.0;
    app.scroll_x = app.scroll_x.clamp(0.0, (total_w - view_w).max(0.0));

    // Draw text
    unsafe {
        libui::clip_text_js(x as f32, (content_y + 40) as f32, w as f32, (h - 40) as f32);
    }
    
    let mut text_y = content_y as f32 + 50.0 - app.scroll_y;
    for (i, line) in app.text.iter().enumerate() {
        if text_y > (y + h) as f32 { break; }
        
        if text_y >= (content_y as f32 + 30.0) {
            unsafe {
                libui::draw_text_js(
                    x as f32 + 10.0 - app.scroll_x,
                    text_y,
                    line.as_ptr(),
                    line.len(),
                    16.0,
                    0.9, 0.9, 0.9, 1.0
                );
            }
        }
        
        // Draw cursor if it's on this line and blinking is ON
        let time_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis();
            
        if !app.is_saving_as && i == app.cursor_row && ((time_ms % 1000) < 500 || time_ms.saturating_sub(app.last_typing_time) < 500) {
            let text_width = if app.cursor_col > 0 && app.cursor_col <= line.len() {
                unsafe { libui::measure_text_js(line.as_ptr(), app.cursor_col, 16.0) }
            } else {
                0.0
            };
            let cx = x as f32 + 10.0 + text_width - app.scroll_x;
            if cx >= x as f32 && cx <= (x + w) as f32 && text_y >= (content_y as f32 + 40.0) {
                unsafe {
                    libui::draw_rect_js(cx, text_y as f32, 2.0, 16.0, 1.0, 1.0, 1.0, 0.8, 0.0, 0.0);
                }
            }
        }
        
        text_y += 20.0;
    }

    unsafe {
        libui::clear_clip_text_js();
    }

    // Draw scrollbars
    let view_h = h as f32 - 90.0;
    let total_h = (app.text.len() as f32) * 20.0 + 20.0;
    if total_h > view_h {
        let sb_h = ((view_h / total_h) * view_h).max(20.0);
        let sb_y = (content_y + 40) as f32 + (app.scroll_y / (total_h - view_h).max(1.0)) * (view_h - sb_h);
        unsafe {
            libui::draw_rect_js(x as f32 + w as f32 - 12.0, sb_y, 8.0, sb_h, 0.4, 0.4, 0.45, 0.8, 4.0, 0.0);
        }
    }

    let view_w = w as f32 - 30.0;
    let longest_line_len = app.text.iter().map(|l| l.len()).max().unwrap_or(0);
    let total_w = longest_line_len as f32 * 9.6 + 20.0; // Approximation is fine for scrollbar scale, avoids WASM IPC lag
    if total_w > view_w {
        let sb_w = ((view_w / total_w) * view_w).max(20.0);
        let sb_x = x as f32 + 10.0 + (app.scroll_x / (total_w - view_w).max(1.0)) * (view_w - sb_w);
        unsafe {
            libui::draw_rect_js(sb_x, y as f32 + h as f32 - 12.0, sb_w, 8.0, 0.4, 0.4, 0.45, 0.8, 4.0, 0.0);
        }
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
                let text_width = if app.save_as_cursor_pos > 0 && app.save_as_cursor_pos <= app.save_as_filename.len() {
                    libui::measure_text_js(app.save_as_filename.as_ptr(), app.save_as_cursor_pos, 14.0)
                } else {
                    0.0
                };
                let cx = mx as f32 + 85.0 + text_width;
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
    
    if app.is_dragging_v_scroll {
        let view_h = app.win.h as f32 - 90.0;
        let total_h = (app.text.len() as f32) * 20.0 + 20.0;
        let sb_h = ((view_h / total_h) * view_h).max(20.0);
        let thumb_y = my as f32 - (content_y as f32 + 40.0) - sb_h / 2.0;
        app.scroll_y = (thumb_y / (view_h - sb_h).max(1.0)) * (total_h - view_h);
        app.scroll_y = app.scroll_y.clamp(0.0, (total_h - view_h).max(0.0));
        redraw = true;
    }
    
    if app.is_dragging_h_scroll {
        let view_w = app.win.w as f32 - 30.0;
        let longest_line_len = app.text.iter().map(|l| l.len()).max().unwrap_or(0);
        let total_w = longest_line_len as f32 * 9.6 + 20.0;
        let sb_w = ((view_w / total_w) * view_w).max(20.0);
        let thumb_x = mx as f32 - (app.win.x as f32 + 10.0) - sb_w / 2.0;
        app.scroll_x = (thumb_x / (view_w - sb_w).max(1.0)) * (total_w - view_w);
        app.scroll_x = app.scroll_x.clamp(0.0, (total_w - view_w).max(0.0));
        redraw = true;
    }
    
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
    
    let view_h = app.win.h as f32 - 90.0;
    let total_h = (app.text.len() as f32) * 20.0 + 20.0;
    if total_h > view_h {
        let sb_x = app.win.x as f32 + app.win.w as f32 - 24.0;
        let sb_y = content_y as f32 + 40.0;
        if (mx as f32) >= sb_x && (mx as f32) <= sb_x + 24.0 && 
           (my as f32) >= sb_y && (my as f32) <= sb_y + view_h {
            app.is_dragging_v_scroll = true;
            return handle_mouse_move(mx, my);
        }
    }
    
    let view_w = app.win.w as f32 - 30.0;
    let longest_line_len = app.text.iter().map(|l| l.len()).max().unwrap_or(0);
    let total_w = longest_line_len as f32 * 9.6 + 20.0;
    if total_w > view_w {
        let sb_x = app.win.x as f32 + 10.0;
        let sb_y = app.win.y as f32 + app.win.h as f32 - 24.0;
        if (mx as f32) >= sb_x && (mx as f32) <= sb_x + view_w &&
           (my as f32) >= sb_y && (my as f32) <= sb_y + 24.0 {
            app.is_dragging_h_scroll = true;
            return handle_mouse_move(mx, my);
        }
    }
    
    // Check if clicked in text area
    let text_area_y = content_y + 40;
    if !app.picker.is_open && !app.is_saving_as {
        let view_w_click = app.win.w - if total_h > view_h { 24 } else { 0 };
        let view_h_click = app.win.h - 40 - if total_w > view_w { 24 } else { 0 };
        
        if my >= text_area_y && my < text_area_y + view_h_click {
            if mx >= app.win.x && mx < app.win.x + view_w_click {
                let relative_y = my as f32 - text_area_y as f32 + app.scroll_y;
                let clicked_row = (relative_y / 20.0).floor() as usize;
                
                let actual_row = clicked_row.min(app.text.len().saturating_sub(1));
                app.cursor_row = actual_row;
                
                let relative_x = mx as f32 - (app.win.x as f32 + 10.0) + app.scroll_x;
                let line = &app.text[app.cursor_row];
                
                let mut best_col = 0;
                let mut min_diff = f32::MAX;
                
                for i in 0..=line.len() {
                    let w = if i > 0 {
                        unsafe { libui::measure_text_js(line.as_ptr(), i, 16.0) }
                    } else {
                        0.0
                    };
                    
                    let diff = (w - relative_x).abs();
                    if diff < min_diff {
                        min_diff = diff;
                        best_col = i;
                    }
                }
                app.cursor_col = best_col;
                app.needs_autoscroll = false; // No need to autoscroll since they clicked here
                redraw = true;
            }
        }
    }
    
    redraw |= app.btn_open.handle_mouse_down(mx, my, app.win.x + 10, content_y + 8);
    redraw |= app.btn_save.handle_mouse_down(mx, my, app.win.x + 100, content_y + 8);
    redraw |= app.btn_save_as.handle_mouse_down(mx, my, app.win.x + 190, content_y + 8);
    if redraw { 1 } else { 0 }
}

#[no_mangle]
pub extern "C" fn handle_mouse_up(mx: i32, my: i32) -> i32 {
    let app = unsafe { NOTEPAD.as_mut().unwrap() };
    app.is_dragging_v_scroll = false;
    app.is_dragging_h_scroll = false;
    let content_y = app.win.y + 25;
    
    if app.picker.is_open {
        let mut redraw = app.picker.handle_mouse_up(mx, my, app.win.x + 70, content_y + 50);
        
        if app.is_saving_as {
            if !app.picker.is_open {
                // If picker closed during Save As, they clicked a file. Grab its name.
                if let Some(path) = app.picker.selected_path.take() {
                    if let Some(name) = path.split('/').last() {
                        app.save_as_filename = name.to_string();
                        app.save_as_cursor_pos = app.save_as_filename.len();
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
        app.save_as_cursor_pos = 0;
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
                if app.save_as_cursor_pos > 0 && app.save_as_cursor_pos <= app.save_as_filename.len() {
                    app.save_as_filename.remove(app.save_as_cursor_pos - 1);
                    app.save_as_cursor_pos -= 1;
                }
            }
            1037 => { // Left
                if app.save_as_cursor_pos > 0 {
                    app.save_as_cursor_pos -= 1;
                }
            }
            1039 => { // Right
                if app.save_as_cursor_pos < app.save_as_filename.len() {
                    app.save_as_cursor_pos += 1;
                }
            }
            17 | 27 => { // Ctrl+Q / Escape hack
                app.is_saving_as = false;
                app.picker.close();
            }
            c if c >= 32 && c <= 126 => {
                if app.save_as_cursor_pos <= app.save_as_filename.len() {
                    app.save_as_filename.insert(app.save_as_cursor_pos, c as u8 as char);
                    app.save_as_cursor_pos += 1;
                }
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
    
    app.needs_autoscroll = true;
    app.lbl_status.text = "Edited".to_string();
    1
}
