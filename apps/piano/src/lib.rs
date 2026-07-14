#![allow(static_mut_refs)]

use libui::window::Window;
use libui::{Button, Widget};

#[link(wasm_import_module = "env")]
unsafe extern "C" {
    fn sys_play_tone(freq: f32, duration_ms: i32, wave_type: i32);
    fn draw_rect_js(x: f32, y: f32, w: f32, h: f32, r: f32, g: f32, b: f32, a: f32, radius: f32, shadow_blur: f32);
    fn draw_text_js(x: f32, y: f32, ptr: *const u8, len: usize, font_size: f32, r: f32, g: f32, b: f32, a: f32);
}

static mut WINDOW: Option<Window> = None;
static mut BUTTONS: Option<Vec<(Button, f64, f64)>> = None; // (Button, rel_x, rel_y)

fn get_freq(note: i32) -> f32 {
    let a4_index = 9;
    440.0 * 2.0_f32.powf((note - a4_index) as f32 / 12.0)
}

#[no_mangle]
pub extern "C" fn init() {
    unsafe {
        WINDOW = Some(Window::new("Piano", 100, 100, 520, 240));
        
        let notes = ["C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B", "C5"];
        let mut btns = Vec::new();
        
        // Draw white keys first
        let mut x_offset = 10.0;
        for (i, note_name) in notes.iter().enumerate() {
            if note_name.contains("#") { continue; }
            
            let mut btn = Button::new(note_name, 60, 160);
            let note_index = i as i32;
            btn.on_click = Some(Box::new(move || {
                let freq = get_freq(note_index);
                unsafe { sys_play_tone(freq, 300, 1); } // 300ms square wave
            }));
            
            btns.push((btn, x_offset, 50.0));
            x_offset += 60.0; // Contiguous white keys
        }
        
        // Draw black keys
        let mut x_offset = 50.0;
        for (i, note_name) in notes.iter().enumerate() {
            if !note_name.contains("#") {
                if note_name == &"E" || note_name == &"B" {
                    x_offset += 60.0;
                }
                continue;
            }
            
            let mut btn = Button::new(note_name, 40, 100);
            let note_index = i as i32;
            btn.on_click = Some(Box::new(move || {
                let freq = get_freq(note_index);
                unsafe { sys_play_tone(freq, 300, 1); }
            }));
            
            btns.push((btn, x_offset, 50.0));
            x_offset += 60.0;
        }
        
        BUTTONS = Some(btns);
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
            win.draw_background(0.12, 0.12, 0.15);
            
            if let Some(btns) = &mut BUTTONS {
                for (btn, rel_x, rel_y) in btns.iter_mut() {
                    let bx = win.x + *rel_x as i32;
                    let by = win.y + *rel_y as i32;
                    let is_black = btn.text.contains("#");
                    
                    let (r, g, b) = if is_black {
                        if btn.is_pressed { (0.3, 0.3, 0.3) }
                        else if btn.is_hovered { (0.2, 0.2, 0.2) }
                        else { (0.1, 0.1, 0.1) }
                    } else {
                        if btn.is_pressed { (0.8, 0.8, 0.8) }
                        else if btn.is_hovered { (0.9, 0.9, 0.9) }
                        else { (1.0, 1.0, 1.0) }
                    };
                    
                    draw_rect_js(bx as f32, by as f32, btn.w as f32, btn.h as f32, r, g, b, 1.0, 4.0, 2.0);
                    
                    let (tr, tg, tb) = if is_black { (1.0, 1.0, 1.0) } else { (0.1, 0.1, 0.1) };
                    let tx = bx + (btn.w / 2) - (btn.text.len() as i32 * 4);
                    let ty = by + btn.h - 20;
                    draw_text_js(tx as f32, ty as f32, btn.text.as_ptr(), btn.text.len(), 14.0, tr, tg, tb, 1.0);
                }
            }
        }
    }
}

// Helper to find the topmost button under the mouse
unsafe fn find_hovered_button(mx: i32, my: i32, win_x: i32, win_y: i32) -> Option<usize> {
    if let Some(btns) = &BUTTONS {
        for (idx, (btn, rel_x, rel_y)) in btns.iter().enumerate().rev() {
            let bx = win_x + *rel_x as i32;
            let by = win_y + *rel_y as i32;
            if mx >= bx && mx <= bx + btn.w && my >= by && my <= by + btn.h {
                return Some(idx);
            }
        }
    }
    None
}

#[no_mangle]
pub extern "C" fn handle_mouse_down(mx: i32, my: i32, x: i32, y: i32) -> bool {
    let mut needs_redraw = false;
    unsafe {
        if let Some(btns) = &mut BUTTONS {
            let hovered_idx = find_hovered_button(mx, my, x, y);
            for (idx, (btn, _, _)) in btns.iter_mut().enumerate() {
                if Some(idx) == hovered_idx {
                    if !btn.is_pressed {
                        btn.is_pressed = true;
                        needs_redraw = true;
                    }
                } else {
                    if btn.is_pressed {
                        btn.is_pressed = false;
                        needs_redraw = true;
                    }
                }
            }
        }
    }
    needs_redraw
}

#[no_mangle]
pub extern "C" fn handle_mouse_up(mx: i32, my: i32, x: i32, y: i32) -> bool {
    let mut needs_redraw = false;
    unsafe {
        if let Some(btns) = &mut BUTTONS {
            let hovered_idx = find_hovered_button(mx, my, x, y);
            for (idx, (btn, _, _)) in btns.iter_mut().enumerate() {
                if btn.is_pressed {
                    btn.is_pressed = false;
                    needs_redraw = true;
                    if Some(idx) == hovered_idx {
                        if let Some(ref cb) = btn.on_click {
                            cb();
                        }
                    }
                }
            }
        }
    }
    needs_redraw
}

#[no_mangle]
pub extern "C" fn handle_mouse_move(mx: i32, my: i32, x: i32, y: i32) -> bool {
    let mut needs_redraw = false;
    unsafe {
        if let Some(btns) = &mut BUTTONS {
            let hovered_idx = find_hovered_button(mx, my, x, y);
            for (idx, (btn, _, _)) in btns.iter_mut().enumerate() {
                if Some(idx) == hovered_idx {
                    if !btn.is_hovered {
                        btn.is_hovered = true;
                        needs_redraw = true;
                    }
                } else {
                    if btn.is_hovered {
                        btn.is_hovered = false;
                        needs_redraw = true;
                    }
                }
            }
        }
    }
    needs_redraw
}
