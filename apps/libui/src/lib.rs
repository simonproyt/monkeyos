
pub mod window;

#[link(wasm_import_module = "env")]
extern "C" {
    pub fn draw_rect_js(x: f32, y: f32, w: f32, h: f32, r: f32, g: f32, b: f32, a: f32, radius: f32, shadow_blur: f32);
    pub fn draw_text_js(x: f32, y: f32, ptr: *const u8, len: usize, font_size: f32, r: f32, g: f32, b: f32, a: f32);
    pub fn clear_text_js();
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
    pub on_click: Option<fn()>,
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
        let (r, g, b) = if self.is_pressed {
            (0.1, 0.5, 0.1) // dark green
        } else if self.is_hovered {
            (0.3, 0.8, 0.3) // light green
        } else {
            (0.2, 0.6, 0.2) // normal green
        };
        
        unsafe {
            draw_rect_js(x as f32, y as f32, self.w as f32, self.h as f32, r, g, b, 1.0, 4.0, 0.0);
            
            // Draw centered text
            let text_x = x + (self.w / 2) - (self.text.len() as i32 * 4); // rough approx
            let text_y = y + (self.h / 2) - 8;
            draw_text_js(text_x as f32, text_y as f32, self.text.as_ptr(), self.text.len(), 16.0, 1.0, 1.0, 1.0, 1.0);
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
                if let Some(cb) = self.on_click {
                    cb();
                }
            }
        }
        needs_redraw
    }
}
