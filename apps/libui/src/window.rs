#[link(wasm_import_module = "env")]
extern "C" {
    fn sys_create_window(x: i32, y: i32, w: i32, h: i32, app_type: u32) -> u32;
}

pub struct Window {
    pub id: u32,
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

impl Window {
    pub fn new(title: &str, x: i32, y: i32, w: i32, h: i32) -> Self {
        let app_type = match title {
            "Terminal" => 0,
            "Calculator" => 1,
            "File Manager" => 2,
            "Notepad" => 3,
            _ => 4,
        };
        let id = unsafe { sys_create_window(x, y, w, h, app_type) };
        Self {
            id, x, y, w, h,
        }
    }

    pub fn draw_background(&self, r: f32, g: f32, b: f32) {
        unsafe {
            crate::draw_rect_js(self.x as f32, self.y as f32, self.w as f32, self.h as f32, r, g, b, 1.0, 0.0, 0.0);
        }
    }

    pub fn handle_mouse_down(&mut self, _mx: i32, _my: i32) -> bool {
        // Handled by wm.rs
        false
    }

    pub fn handle_mouse_up(&mut self) -> bool {
        // Handled by wm.rs
        false
    }

    pub fn handle_mouse_move(&mut self, _mx: i32, _my: i32) -> bool {
        // Handled by wm.rs
        false
    }
}
