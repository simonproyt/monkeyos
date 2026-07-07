#[link(wasm_import_module = "env")]
extern "C" {
    fn sys_create_window(x: i32, y: i32, w: i32, h: i32) -> u32;
}

pub struct Window {
    pub id: u32,
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

impl Window {
    pub fn new(_title: &str, x: i32, y: i32, w: i32, h: i32) -> Self {
        let id = unsafe { sys_create_window(x, y, w, h) };
        Self {
            id, x, y, w, h,
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
