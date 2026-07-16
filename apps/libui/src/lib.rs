
pub mod window;
pub mod picker;

#[link(wasm_import_module = "env")]
unsafe extern "C" {
    pub fn draw_rect_js(x: f32, y: f32, w: f32, h: f32, r: f32, g: f32, b: f32, a: f32, radius: f32, shadow_blur: f32);
    pub fn draw_text_js(x: f32, y: f32, ptr: *const u8, len: usize, font_size: f32, r: f32, g: f32, b: f32, a: f32);
    pub fn measure_text_js(ptr: *const u8, len: usize, font_size: f32) -> f32;
    pub fn clip_text_js(x: f32, y: f32, w: f32, h: f32);
    pub fn clear_clip_text_js();
    pub fn clear_text_js();
    pub fn load_image_js(id: u32, ptr: *const u8, len: usize);
    pub fn draw_image_js(id: u32, x: f32, y: f32, w: f32, h: f32);
    pub fn sys_get_system_info(out_ptr: *mut u8, max_len: usize) -> i32;
}

pub fn get_system_info() -> String {
    let mut buf = vec![0u8; 8192];
    let len = unsafe { sys_get_system_info(buf.as_mut_ptr(), buf.len()) };
    if len > 0 && len < buf.len() as i32 {
        buf.truncate(len as usize);
        String::from_utf8_lossy(&buf).into_owned()
    } else {
        String::new()
    }
}

pub trait Widget {
    fn draw(&self, x: i32, y: i32);
    fn handle_mouse_move(&mut self, _mx: i32, _my: i32, _x: i32, _y: i32) -> bool { false }
    fn handle_mouse_down(&mut self, _mx: i32, _my: i32, _x: i32, _y: i32) -> bool { false }
    fn handle_mouse_up(&mut self, _mx: i32, _my: i32, _x: i32, _y: i32) -> bool { false }
}

pub struct Label {
    pub text: String,
    pub font_size: f32,
    pub color: (f32, f32, f32, f32),
}

impl Widget for Label {
    fn draw(&self, x: i32, y: i32) {
        unsafe {
            draw_text_js(x as f32, y as f32, self.text.as_ptr(), self.text.len(), self.font_size, self.color.0, self.color.1, self.color.2, self.color.3);
        }
    }
}

pub struct Button {
    pub text: String,
    pub w: i32,
    pub h: i32,
    pub is_hovered: bool,
    pub is_pressed: bool,
    pub on_click: Option<Box<dyn Fn()>>,
}

impl Button {
    pub fn new(text: &str, w: i32, h: i32) -> Self {
        Self {
            text: text.to_string(),
            w,
            h,
            is_hovered: false,
            is_pressed: false,
            on_click: None,
        }
    }
}

impl Widget for Button {
    fn draw(&self, x: i32, y: i32) {
        let (r, g, b, a) = if self.is_pressed {
            (0.1, 0.1, 0.15, 0.8)
        } else if self.is_hovered {
            (0.25, 0.25, 0.3, 0.7)
        } else {
            (0.15, 0.15, 0.2, 0.5) // Translucent for glassmorphism effect
        };
        
        unsafe {
            draw_rect_js(x as f32, y as f32, self.w as f32, self.h as f32, r, g, b, a, 6.0, 2.0);
            
            // Draw left-aligned text with padding
            let text_x = x + 12;
            let text_y = y + (self.h / 2) - 6;
            draw_text_js(text_x as f32, text_y as f32, self.text.as_ptr(), self.text.len(), 14.0, 0.9, 0.9, 0.9, 1.0);
        }
    }

    fn handle_mouse_move(&mut self, mx: i32, my: i32, x: i32, y: i32) -> bool {
        let inside = mx >= x && mx <= x + self.w && my >= y && my <= y + self.h;
        if inside != self.is_hovered {
            self.is_hovered = inside;
            true // needs redraw
        } else {
            false
        }
    }

    fn handle_mouse_down(&mut self, mx: i32, my: i32, x: i32, y: i32) -> bool {
        let inside = mx >= x && mx <= x + self.w && my >= y && my <= y + self.h;
        if inside {
            self.is_pressed = true;
            true // needs redraw
        } else {
            false
        }
    }

    fn handle_mouse_up(&mut self, mx: i32, my: i32, x: i32, y: i32) -> bool {
        let inside = mx >= x && mx <= x + self.w && my >= y && my <= y + self.h;
        let mut needs_redraw = false;
        if self.is_pressed {
            self.is_pressed = false;
            needs_redraw = true;
            if inside {
                if let Some(ref cb) = self.on_click {
                    cb();
                }
            }
        }
        needs_redraw
    }
}
