use libui::window::Window;
use libui::get_system_info;

struct ProcessInfo {
    pid: u32,
    name: String,
    memory_kb: u32,
    ptype: String,
}

struct TaskManager {
    win: Window,
    frame: u64,
    processes: Vec<ProcessInfo>,
}

static mut APP: Option<TaskManager> = None;

#[no_mangle]
pub extern "C" fn init() {
    unsafe {
        APP = Some(TaskManager {
            win: Window::new("Task Manager", 100, 100, 400, 300),
            frame: 0,
            processes: Vec::new(),
        });
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
            
            app.frame = app.frame.wrapping_add(1);
            
            // Refresh process list every 30 frames (0.5s)
            if app.frame % 30 == 0 || app.processes.is_empty() {
                app.processes.clear();
                let info_str = get_system_info();
                for proc_str in info_str.split('|') {
                    let parts: Vec<&str> = proc_str.split(';').collect();
                    if parts.len() >= 4 {
                        let pid = parts[0].parse().unwrap_or(0);
                        let memory_bytes: u32 = parts[2].parse().unwrap_or(0);
                        app.processes.push(ProcessInfo {
                            pid,
                            name: parts[1].to_string(),
                            memory_kb: memory_bytes / 1024,
                            ptype: parts[3].to_string(),
                        });
                    }
                }
            }
            
            app.win.draw_background(0.12, 0.12, 0.12);
            
            // Draw Header
            libui::draw_rect_js(x as f32, y as f32, w as f32, 30.0, 0.2, 0.2, 0.25, 1.0, 0.0, 0.0);
            libui::draw_text_js(x as f32 + 10.0, y as f32 + 20.0, b"PID".as_ptr(), 3, 14.0, 0.8, 0.8, 0.8, 1.0);
            libui::draw_text_js(x as f32 + 50.0, y as f32 + 20.0, b"Name".as_ptr(), 4, 14.0, 0.8, 0.8, 0.8, 1.0);
            libui::draw_text_js(x as f32 + 220.0, y as f32 + 20.0, b"Memory".as_ptr(), 6, 14.0, 0.8, 0.8, 0.8, 1.0);
            libui::draw_text_js(x as f32 + 320.0, y as f32 + 20.0, b"Type".as_ptr(), 4, 14.0, 0.8, 0.8, 0.8, 1.0);
            
            // Draw Processes
            let mut current_y = y as f32 + 50.0;
            let mut total_memory = 0;
            for p in &app.processes {
                let pid_str = format!("{}", p.pid);
                let mem_str = format!("{} KB", p.memory_kb);
                total_memory += p.memory_kb;
                
                libui::draw_text_js(x as f32 + 10.0, current_y, pid_str.as_ptr(), pid_str.len(), 14.0, 0.6, 0.9, 0.6, 1.0);
                libui::draw_text_js(x as f32 + 50.0, current_y, p.name.as_ptr(), p.name.len(), 14.0, 0.9, 0.9, 0.9, 1.0);
                libui::draw_text_js(x as f32 + 220.0, current_y, mem_str.as_ptr(), mem_str.len(), 14.0, 0.6, 0.6, 0.9, 1.0);
                libui::draw_text_js(x as f32 + 320.0, current_y, p.ptype.as_ptr(), p.ptype.len(), 14.0, 0.7, 0.7, 0.7, 1.0);
                
                current_y += 25.0;
            }
            
            // Draw Footer
            let footer_y = y as f32 + h as f32 - 30.0;
            libui::draw_rect_js(x as f32, footer_y, w as f32, 30.0, 0.15, 0.15, 0.2, 1.0, 0.0, 0.0);
            let total_str = format!("Total Memory: {} KB", total_memory);
            libui::draw_text_js(x as f32 + 10.0, footer_y + 20.0, total_str.as_ptr(), total_str.len(), 14.0, 1.0, 1.0, 0.5, 1.0);
        }
    }
}
