use crate::process::ProcessId;
use crate::ipc::MessagePayload;
use crate::sys::SyscallEnv;


#[link(wasm_import_module = "env")]
extern "C" {
    fn draw_rect_js(x: f32, y: f32, w: f32, h: f32, r: f32, g: f32, b: f32, a: f32, radius: f32, shadow_blur: f32);
    fn clear_screen_js();
    fn create_html_overlay_js(id: u32, x: f32, y: f32, w: f32, h: f32);
    fn destroy_html_overlay_js(id: u32);
    fn update_html_overlay_bounds_js(id: u32, x: f32, y: f32, w: f32, h: f32, z: u32, is_active: u32);
    fn append_html_overlay_text_js(id: u32, ptr: *const u8, len: usize);
    fn update_html_overlay_input_line_js(id: u32, p_ptr: *const u8, p_len: usize, i_ptr: *const u8, i_len: usize, cursor_pos: u32);
    fn clear_html_overlay_text_js(id: u32);
    fn draw_editor_js(id: u32, content_ptr: *const u8, content_len: usize, cursor_pos: u32);
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
                MessagePayload::ClearScreen => {
                    unsafe { clear_screen_js() };
                }
                MessagePayload::CreateHtmlOverlay { id, x, y, w, h } => {
                    unsafe { create_html_overlay_js(id, x as f32, y as f32, w as f32, h as f32) };
                }
                MessagePayload::DestroyHtmlOverlay { id } => {
                    unsafe { destroy_html_overlay_js(id) };
                }
                MessagePayload::UpdateHtmlOverlayBounds { id, x, y, w, h, z, is_active } => {
                    unsafe { update_html_overlay_bounds_js(id, x as f32, y as f32, w as f32, h as f32, z, is_active as u32) };
                }
                MessagePayload::AppendHtmlOverlayText { id, text } => {
                    unsafe { append_html_overlay_text_js(id, text.as_ptr(), text.len()) };
                }
                MessagePayload::UpdateHtmlOverlayInputLine { id, prompt, input, cursor_pos } => {
                    unsafe { update_html_overlay_input_line_js(id, prompt.as_ptr(), prompt.len(), input.as_ptr(), input.len(), cursor_pos) };
                }
                MessagePayload::DrawEditor { id, content, cursor_pos } => {
                    unsafe { draw_editor_js(id, content.as_ptr(), content.len(), cursor_pos) };
                }
                MessagePayload::ClearHtmlOverlayText { id } => {
                    unsafe { clear_html_overlay_text_js(id) };
                }
                _ => {}
            }
        }
        true
    }
}
