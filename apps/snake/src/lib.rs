#![allow(static_mut_refs)]

use libui::window::Window;

#[link(wasm_import_module = "env")]
unsafe extern "C" {
    fn sys_play_tone(freq: f32, duration_ms: i32, wave_type: i32);
}

const GRID_W: i32 = 30;
const GRID_H: i32 = 20;
const CELL_SIZE: i32 = 16;
const OFFSET_X: i32 = 10;
const OFFSET_Y: i32 = 40;

#[derive(Clone, Copy, PartialEq)]
enum Direction {
    Up,
    Down,
    Left,
    Right,
}

struct SnakeGame {
    win: Window,
    snake: Vec<(i32, i32)>,
    dir: Direction,
    next_dir: Direction,
    food: (i32, i32),
    score: u32,
    game_over: bool,
    frame_count: u32,
    speed: u32,
}

static mut APP: Option<SnakeGame> = None;

// Extremely simple pseudo-random number generator
static mut RAND_STATE: u32 = 12345;
fn rand() -> u32 {
    unsafe {
        RAND_STATE ^= RAND_STATE << 13;
        RAND_STATE ^= RAND_STATE >> 17;
        RAND_STATE ^= RAND_STATE << 5;
        RAND_STATE
    }
}

fn rand_range(min: i32, max: i32) -> i32 {
    if max <= min {
        return min;
    }
    min + (rand() % (max - min) as u32) as i32
}

fn spawn_food(snake: &Vec<(i32, i32)>) -> (i32, i32) {
    loop {
        let fx = rand_range(0, GRID_W);
        let fy = rand_range(0, GRID_H);
        let mut ok = true;
        for &(sx, sy) in snake.iter() {
            if sx == fx && sy == fy {
                ok = false;
                break;
            }
        }
        if ok {
            return (fx, fy);
        }
    }
}

#[no_mangle]
pub extern "C" fn init() {
    let win = Window::new("Snake", 100, 100, OFFSET_X * 2 + GRID_W * CELL_SIZE, OFFSET_Y + 10 + GRID_H * CELL_SIZE);
    
    let initial_snake = vec![(GRID_W / 2, GRID_H / 2), (GRID_W / 2 - 1, GRID_H / 2), (GRID_W / 2 - 2, GRID_H / 2)];
    let initial_food = spawn_food(&initial_snake);

    unsafe {
        APP = Some(SnakeGame {
            win,
            snake: initial_snake,
            dir: Direction::Right,
            next_dir: Direction::Right,
            food: initial_food,
            score: 0,
            game_over: false,
            frame_count: 0,
            speed: 6, // Update every N frames
        });
    }
}

#[no_mangle]
pub extern "C" fn handle_key_down(key_code: u32) -> bool {
    let mut needs_redraw = false;
    unsafe {
        if let Some(app) = &mut APP {
            if app.game_over {
                if key_code == 13 || key_code == 32 { // Enter or Space to restart
                    app.snake = vec![(GRID_W / 2, GRID_H / 2), (GRID_W / 2 - 1, GRID_H / 2), (GRID_W / 2 - 2, GRID_H / 2)];
                    app.dir = Direction::Right;
                    app.next_dir = Direction::Right;
                    app.food = spawn_food(&app.snake);
                    app.score = 0;
                    app.game_over = false;
                    needs_redraw = true;
                }
                return needs_redraw;
            }

            match key_code {
                1037 | 65 | 97 if app.dir != Direction::Right => { app.next_dir = Direction::Left; needs_redraw = true; }, // Left, A, a
                1038 | 87 | 119 if app.dir != Direction::Down => { app.next_dir = Direction::Up; needs_redraw = true; }, // Up, W, w
                1039 | 68 | 100 if app.dir != Direction::Left => { app.next_dir = Direction::Right; needs_redraw = true; }, // Right, D, d
                1040 | 83 | 115 if app.dir != Direction::Up => { app.next_dir = Direction::Down; needs_redraw = true; }, // Down, S, s
                _ => {}
            }
        }
    }
    needs_redraw
}

#[no_mangle]
pub extern "C" fn handle_mouse_down(_mx: i32, _my: i32, _x: i32, _y: i32) -> bool {
    unsafe {
        if let Some(app) = &mut APP {
            if app.game_over {
                app.snake = vec![(GRID_W / 2, GRID_H / 2), (GRID_W / 2 - 1, GRID_H / 2), (GRID_W / 2 - 2, GRID_H / 2)];
                app.dir = Direction::Right;
                app.next_dir = Direction::Right;
                app.food = spawn_food(&app.snake);
                app.score = 0;
                app.game_over = false;
                return true;
            }
        }
    }
    false
}

#[no_mangle]
pub extern "C" fn tick(x: i32, y: i32, _w: i32, _h: i32) {
    unsafe {
        if let Some(app) = &mut APP {
            app.frame_count += 1;
            
            if !app.game_over && app.frame_count % app.speed == 0 {
                app.dir = app.next_dir;
                
                let head = app.snake[0];
                let mut new_head = head;
                match app.dir {
                    Direction::Up => new_head.1 -= 1,
                    Direction::Down => new_head.1 += 1,
                    Direction::Left => new_head.0 -= 1,
                    Direction::Right => new_head.0 += 1,
                }
                
                // Wall collision
                if new_head.0 < 0 || new_head.0 >= GRID_W || new_head.1 < 0 || new_head.1 >= GRID_H {
                    app.game_over = true;
                    sys_play_tone(150.0, 500, 3); // Crash sound
                } else {
                    // Self collision
                    for i in 0..app.snake.len() - 1 { // exclude tail since it will move
                        if app.snake[i] == new_head {
                            app.game_over = true;
                            sys_play_tone(150.0, 500, 3);
                            break;
                        }
                    }
                }
                
                if !app.game_over {
                    app.snake.insert(0, new_head);
                    
                    if new_head == app.food {
                        app.score += 10;
                        app.speed = (6 - (app.score / 100)).max(2); // Speed up slightly
                        app.food = spawn_food(&app.snake);
                        sys_play_tone(880.0, 50, 1); // Eat sound
                    } else {
                        app.snake.pop(); // Remove tail
                    }
                }
            }
            
            // Render
            app.win.x = x;
            app.win.y = y;
            app.win.draw_background(0.1, 0.1, 0.15);
            
            // Draw Header
            let score_text = format!("Score: {}", app.score);
            libui::draw_text_js(x as f32 + 10.0, y as f32 + 10.0, score_text.as_ptr(), score_text.len(), 16.0, 1.0, 1.0, 1.0, 1.0);
            
            // Draw Play Area
            let base_x = x + OFFSET_X;
            let base_y = y + OFFSET_Y;
            libui::draw_rect_js(base_x as f32 - 2.0, base_y as f32 - 2.0, (GRID_W * CELL_SIZE) as f32 + 4.0, (GRID_H * CELL_SIZE) as f32 + 4.0, 0.2, 0.2, 0.25, 1.0, 0.0, 0.0);
            libui::draw_rect_js(base_x as f32, base_y as f32, (GRID_W * CELL_SIZE) as f32, (GRID_H * CELL_SIZE) as f32, 0.05, 0.05, 0.05, 1.0, 0.0, 0.0);
            
            // Draw Food
            libui::draw_rect_js((base_x + app.food.0 * CELL_SIZE) as f32, (base_y + app.food.1 * CELL_SIZE) as f32, CELL_SIZE as f32, CELL_SIZE as f32, 1.0, 0.3, 0.3, 1.0, 4.0, 0.0);
            
            // Draw Snake
            for (i, &(sx, sy)) in app.snake.iter().enumerate() {
                let is_head = i == 0;
                let (r, g, b) = if is_head { (0.4, 1.0, 0.4) } else { (0.2, 0.8, 0.2) };
                libui::draw_rect_js((base_x + sx * CELL_SIZE) as f32 + 1.0, (base_y + sy * CELL_SIZE) as f32 + 1.0, CELL_SIZE as f32 - 2.0, CELL_SIZE as f32 - 2.0, r, g, b, 1.0, 2.0, 0.0);
            }
            
            // Draw Game Over Overlay
            if app.game_over {
                let go_text = "GAME OVER";
                let restart_text = "Click or Press Enter to Restart";
                
                libui::draw_rect_js(base_x as f32, base_y as f32 + (GRID_H * CELL_SIZE / 2 - 40) as f32, (GRID_W * CELL_SIZE) as f32, 80.0, 0.0, 0.0, 0.0, 0.8, 0.0, 0.0);
                libui::draw_text_js(base_x as f32 + 150.0, base_y as f32 + (GRID_H * CELL_SIZE / 2 - 20) as f32, go_text.as_ptr(), go_text.len(), 24.0, 1.0, 0.2, 0.2, 1.0);
                libui::draw_text_js(base_x as f32 + 100.0, base_y as f32 + (GRID_H * CELL_SIZE / 2 + 10) as f32, restart_text.as_ptr(), restart_text.len(), 14.0, 0.8, 0.8, 0.8, 1.0);
            }
        }
    }
}
