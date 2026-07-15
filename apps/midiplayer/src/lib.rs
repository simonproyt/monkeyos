#![allow(static_mut_refs)]

use libui::{window::Window, Button, Widget, picker::FilePicker};
use midly::{Smf, TrackEventKind, MidiMessage};

#[link(wasm_import_module = "env")]
unsafe extern "C" {
    fn sys_get_launch_arg(out_ptr: *mut u8, out_max_len: usize) -> usize;
    fn sys_fetch(url_ptr: *const u8, url_len: usize, out_ptr: *mut u8, out_max_len: usize) -> i32;
    fn sys_schedule_note(freq: f32, duration_ms: u32, wave_type: u32, volume: f32, delay_ms: u32);
    fn sys_stop_audio();
    fn sys_get_audio_levels(out_ptr: *mut u8, max_len: usize) -> usize;
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

fn fetch_file(path: &str) -> Option<Vec<u8>> {
    let url_bytes = path.as_bytes();
    // Assuming MIDI files won't exceed 1MB for this simple app
    let mut buf = vec![0u8; 1024 * 1024]; 
    let len = unsafe { sys_fetch(url_bytes.as_ptr(), url_bytes.len(), buf.as_mut_ptr(), buf.len()) };
    if len > 0 {
        buf.truncate(len as usize);
        Some(buf)
    } else {
        None
    }
}

struct MidiPlayer {
    win: Window,
    btn_open: Button,
    btn_play: Button,
    picker: FilePicker,
    midi_data: Option<Vec<u8>>,
    status_text: String,
    is_playing: bool,
    frame: u64,
    smoothed_levels: [f32; 32],
}

static mut APP: Option<MidiPlayer> = None;

#[no_mangle]
pub extern "C" fn init() {
    unsafe {
        let mut app = MidiPlayer {
            win: Window::new("MIDI Player", 50, 50, 450, 350),
            btn_open: Button::new("🎵 Open MIDI", 140, 30),
            btn_play: Button::new("▶️ Play", 100, 30),
            picker: FilePicker::new(),
            midi_data: None,
            status_text: "Ready.".to_string(),
            is_playing: false,
            frame: 0,
            smoothed_levels: [0.0; 32],
        };

        if let Some(path) = get_launch_arg() {
            if let Some(data) = fetch_file(&path) {
                app.midi_data = Some(data);
                app.status_text = format!("Loaded: {}", path);
            }
        }

        APP = Some(app);
    }
}

fn play_midi(data: &[u8]) {
    let smf = match Smf::parse(data) {
        Ok(smf) => smf,
        Err(_) => return,
    };
    // VERY simplified playback for format 0 / single track format 1
    // A proper player needs to merge tracks and convert ticks to MS based on tempo.
    // Assuming 1 tick = 2ms for a very basic test.
    let ms_per_tick = 2; 

    // We'll schedule notes from all tracks
    for track in smf.tracks.iter() {
        let mut current_time_ms = 0;
        for ev in track.iter() {
            current_time_ms += ev.delta.as_int() * ms_per_tick;
            if let TrackEventKind::Midi { message, .. } = ev.kind {
                match message {
                    MidiMessage::NoteOn { key, vel } => {
                        if vel > 0 {
                            // Convert MIDI key to frequency
                            let freq = 440.0 * 2.0_f32.powf((key.as_int() as f32 - 69.0) / 12.0);
                            let volume = (vel.as_int() as f32) / 127.0;
                            unsafe {
                                // Default square wave (1) for chiptune style
                                sys_schedule_note(freq, 200, 1, volume, current_time_ms);
                            }
                        }
                    },
                    _ => {}
                }
            }
        }
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
            
            // Draw top bar
            libui::draw_rect_js(app.win.x as f32, app.win.y as f32, app.win.w as f32, 40.0, 0.15, 0.15, 0.2, 1.0, 0.0, 0.0);
            app.btn_open.draw(app.win.x + 10, app.win.y + 5);
            if app.midi_data.is_some() {
                app.btn_play.draw(app.win.x + 160, app.win.y + 5);
            }

            // Audio Visualizer (Real Analyzer)
            app.frame = app.frame.wrapping_add(1);
            let mut levels = [0u8; 32];
            unsafe { sys_get_audio_levels(levels.as_mut_ptr(), levels.len()) };

            let num_bars = 32;
            let bar_w = 8.0;
            let spacing = 3.0;
            let total_w = num_bars as f32 * (bar_w + spacing);
            let base_x = app.win.x as f32 + (app.win.w as f32 - total_w) / 2.0;
            let base_y = app.win.y as f32 + 250.0; // Moved down
            
            for (i, &level) in levels.iter().enumerate() {
                // Gravity-based smoothing for snappy attack and natural fall
                let target = level as f32;
                if target > app.smoothed_levels[i] {
                    // Fast attack
                    app.smoothed_levels[i] += (target - app.smoothed_levels[i]) * 0.8;
                } else {
                    // Constant linear decay (gravity) avoids "sticky/laggy" exponential tails
                    app.smoothed_levels[i] -= 8.0;
                    if app.smoothed_levels[i] < target {
                        app.smoothed_levels[i] = target;
                    }
                }
                
                let mut h = app.smoothed_levels[i] * 0.5; 
                if h < 4.0 { h = 4.0; } 
                
                // Neon Cyberpunk colors
                let intensity = h / 150.0;
                let r = (0.2 + intensity * 0.8).clamp(0.2, 1.0);
                let g = (0.1 + intensity * 0.3).clamp(0.1, 0.6);
                let b = (0.9 - intensity * 0.5).clamp(0.2, 1.0);
                
                let x = base_x + (i as f32 * (bar_w + spacing));
                
                // Main upward bar (Glowing)
                libui::draw_rect_js(
                    x, base_y - h, 
                    bar_w, h, 
                    r, g, b, 1.0, 4.0, 15.0 // heavy glow
                );
                
                // Downward reflection (Faded, no shadow blur)
                libui::draw_rect_js(
                    x, base_y + 2.0, 
                    bar_w, h * 0.4, 
                    r, g, b, 0.3, 4.0, 0.0 
                );
            }

            libui::draw_text_js(app.win.x as f32 + 20.0, app.win.y as f32 + 65.0, app.status_text.as_ptr(), app.status_text.len(), 14.0, 0.8, 0.8, 0.8, 1.0);
            
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
                if app.midi_data.is_some() {
                    redraw |= app.btn_play.handle_mouse_move(mx, my, win_x + 160, win_y + 5);
                }
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
                if app.midi_data.is_some() {
                    redraw |= app.btn_play.handle_mouse_down(mx, my, win_x + 160, win_y + 5);
                }
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
                    if let Some(data) = fetch_file(&path) {
                        app.midi_data = Some(data);
                        app.status_text = format!("Loaded: {}", path);
                        app.is_playing = false;
                        app.btn_play.text = "▶️ Play".to_string();
                        unsafe { sys_stop_audio(); }
                        redraw = true;
                    }
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
                if app.btn_play.is_pressed {
                    let inside = mx >= win_x + 160 && mx <= win_x + 160 + app.btn_play.w && 
                                 my >= win_y + 5 && my <= win_y + 5 + app.btn_play.h;
                    app.btn_play.is_pressed = false;
                    redraw = true;
                    if inside {
                        if app.is_playing {
                            unsafe { sys_stop_audio(); }
                            app.btn_play.text = "▶️ Play".to_string();
                            app.is_playing = false;
                        } else {
                            if let Some(data) = &app.midi_data {
                                play_midi(data);
                                app.btn_play.text = "⏹️ Stop".to_string();
                                app.is_playing = true;
                            }
                        }
                    }
                }
            }
            if redraw { return 1; }
        }
    }
    0
}
