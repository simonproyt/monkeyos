#![allow(dead_code)]

use crate::process::ProcessId;
use crate::ipc::MessagePayload;
use crate::sys::SyscallEnv;


#[link(wasm_import_module = "env")]
extern "C" {
    fn draw_rect_js(x: f32, y: f32, w: f32, h: f32, r: f32, g: f32, b: f32, a: f32, radius: f32, shadow_blur: f32);
    fn draw_gui_app_js(id: u32, x: i32, y: i32, w: i32, h: i32);
    fn clear_screen_js();
    fn create_html_overlay_js(id: u32, x: f32, y: f32, w: f32, h: f32);
    fn destroy_html_overlay_js(id: u32);
    fn update_html_overlay_bounds_js(id: u32, x: f32, y: f32, w: f32, h: f32, z: u32, is_active: u32);
    fn append_html_overlay_text_js(id: u32, ptr: *const u8, len: usize);
    fn update_html_overlay_input_line_js(id: u32, p_ptr: *const u8, p_len: usize, i_ptr: *const u8, i_len: usize, cursor_pos: u32);
    fn clear_html_overlay_text_js(id: u32);
    fn draw_editor_js(id: u32, content_ptr: *const u8, content_len: usize, cursor_pos: u32);
    fn draw_text_js(x: f32, y: f32, ptr: *const u8, len: usize, font_size: f32, r: f32, g: f32, b: f32, a: f32);
    fn clear_text_js();
}

pub struct DisplayServer {
    pid: ProcessId,
}

impl DisplayServer {
    pub fn new(pid: ProcessId) -> Self {
        Self { pid }
    }
}

impl crate::process::Process for DisplayServer {
    fn id(&self) -> ProcessId { self.pid }
    fn name(&self) -> &str { "display_server" }

    fn tick(&mut self, env: &mut SyscallEnv) -> bool {
        while let Some(msg) = env.recv_msg() {
            match msg.payload {
                MessagePayload::DrawRect { x, y, w, h, r, g, b, a, radius, shadow_blur } => {
                    unsafe { draw_rect_js(x as f32, y as f32, w as f32, h as f32, r, g, b, a, radius, shadow_blur) };
                }
                MessagePayload::DrawGuiApp { id, x, y, w, h } => {
                    unsafe { draw_gui_app_js(id, x, y, w, h) };
                }
                MessagePayload::ClearScreen => {
                    unsafe { clear_screen_js() };
                }
                MessagePayload::DrawText { x, y, text, font_size, r, g, b, a } => {
                    unsafe { draw_text_js(x as f32, y as f32, text.as_ptr(), text.len(), font_size, r, g, b, a) };
                }
                MessagePayload::ClearText => {
                    unsafe { clear_text_js() };
                }
                _ => {}
            }
        }
        true
    }
}
