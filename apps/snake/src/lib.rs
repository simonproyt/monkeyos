#![allow(static_mut_refs)]

use libui::window::Window;
use std::fs;

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
enum Direction { Up, Down, Left, Right }

#[derive(Clone, Copy, PartialEq)]
enum FoodType { Normal, Golden, Poison }

struct Particle {
    x: f32, y: f32, vx: f32, vy: f32,
    life: f32, max_life: f32,
    color: (f32, f32, f32),
}

struct SnakeGame {
    win: Window,
    snake: Vec<(i32, i32)>,
    dir: Direction,
    next_dir: Direction,
    food: (i32, i32),
    food_type: FoodType,
    score: u32,
    highscore: u32,
    game_over: bool,
    frame_count: u32,
    speed: u32,
    obstacles: Vec<(i32, i32)>,
    particles: Vec<Particle>,
    glow_ticks: u32,
}

static mut APP: Option<SnakeGame> = None;
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
    if max <= min { return min; }
    min + (rand() % (max - min) as u32) as i32
}

fn rand_float(min: f32, max: f32) -> f32 {
    let r = rand() as f32 / u32::MAX as f32;
    min + r * (max - min)
}

fn is_occupied(x: i32, y: i32, snake: &[(i32, i32)], obstacles: &[(i32, i32)]) -> bool {
    snake.contains(&(x, y)) || obstacles.contains(&(x, y))
}

fn spawn_food(snake: &[(i32, i32)], obstacles: &[(i32, i32)]) -> ((i32, i32), FoodType) {
    loop {
        let fx = rand_range(0, GRID_W);
        let fy = rand_range(0, GRID_H);
        if !is_occupied(fx, fy, snake, obstacles) {
            let r = rand_range(0, 100);
            let ftype = if r < 10 { FoodType::Golden }
                        else if r < 20 { FoodType::Poison }
                        else { FoodType::Normal };
            return ((fx, fy), ftype);
        }
    }
}

fn spawn_obstacle(snake: &[(i32, i32)], obstacles: &[(i32, i32)], food: (i32, i32)) -> Option<(i32, i32)> {
    for _ in 0..50 {
        let ox = rand_range(2, GRID_W - 2);
        let oy = rand_range(2, GRID_H - 2);
        if !is_occupied(ox, oy, snake, obstacles) && (ox, oy) != food {
            return Some((ox, oy));
        }
    }
    None
}

fn load_highscore() -> u32 {
    if let Ok(data) = fs::read_to_string("/snake_highscore.txt") {
        if let Ok(val) = data.trim().parse::<u32>() {
            return val;
        }
    }
    0
}

fn save_highscore(score: u32) {
    let _ = fs::write("/snake_highscore.txt", score.to_string());
}

#[no_mangle]
pub extern "C" fn init() {
    let win = Window::new("Snake", 100, 100, OFFSET_X * 2 + GRID_W * CELL_SIZE, OFFSET_Y + 10 + GRID_H * CELL_SIZE);
    let initial_snake = vec![(GRID_W / 2, GRID_H / 2), (GRID_W / 2 - 1, GRID_H / 2), (GRID_W / 2 - 2, GRID_H / 2)];
    let obstacles = Vec::new();
    let (initial_food, initial_food_type) = spawn_food(&initial_snake, &obstacles);

    unsafe {
        APP = Some(SnakeGame {
            win,
            snake: initial_snake,
            dir: Direction::Right,
            next_dir: Direction::Right,
            food: initial_food,
            food_type: initial_food_type,
            score: 0,
            highscore: load_highscore(),
            game_over: false,
            frame_count: 0,
            speed: 6,
            obstacles,
            particles: Vec::new(),
            glow_ticks: 0,
        });
    }
}

fn restart_game(app: &mut SnakeGame) {
    app.snake = vec![(GRID_W / 2, GRID_H / 2), (GRID_W / 2 - 1, GRID_H / 2), (GRID_W / 2 - 2, GRID_H / 2)];
    app.dir = Direction::Right;
    app.next_dir = Direction::Right;
    app.obstacles.clear();
    let (f, t) = spawn_food(&app.snake, &app.obstacles);
    app.food = f;
    app.food_type = t;
    app.score = 0;
    app.speed = 6;
    app.game_over = false;
    app.particles.clear();
    app.glow_ticks = 0;
}

#[no_mangle]
pub extern "C" fn handle_key_down(key_code: u32) -> bool {
    let mut needs_redraw = false;
    unsafe {
        if let Some(app) = &mut APP {
            if app.game_over {
                if key_code == 13 || key_code == 32 {
                    restart_game(app);
                    needs_redraw = true;
                }
                return needs_redraw;
            }

            match key_code {
                1037 | 65 | 97 if app.dir != Direction::Right => { app.next_dir = Direction::Left; needs_redraw = true; },
                1038 | 87 | 119 if app.dir != Direction::Down => { app.next_dir = Direction::Up; needs_redraw = true; },
                1039 | 68 | 100 if app.dir != Direction::Left => { app.next_dir = Direction::Right; needs_redraw = true; },
                1040 | 83 | 115 if app.dir != Direction::Up => { app.next_dir = Direction::Down; needs_redraw = true; },
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
                restart_game(app);
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
            
            // Update particles every frame
            for p in &mut app.particles {
                p.x += p.vx;
                p.y += p.vy;
                p.life -= 1.0;
            }
            app.particles.retain(|p| p.life > 0.0);
            
            if app.glow_ticks > 0 {
                app.glow_ticks -= 1;
            }
            
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
                    sys_play_tone(150.0, 500, 3);
                } else if app.obstacles.contains(&new_head) {
                    app.game_over = true;
                    sys_play_tone(100.0, 600, 3); // Deeper crash for rocks
                } else {
                    for i in 0..app.snake.len() - 1 {
                        if app.snake[i] == new_head {
                            app.game_over = true;
                            sys_play_tone(150.0, 500, 3);
                            break;
                        }
                    }
                }
                
                if app.game_over {
                    if app.score > app.highscore {
                        app.highscore = app.score;
                        save_highscore(app.highscore);
                    }
                } else {
                    app.snake.insert(0, new_head);
                    
                    if new_head == app.food {
                        // Explode particles
                        let px = (OFFSET_X + app.food.0 * CELL_SIZE + CELL_SIZE / 2) as f32;
                        let py = (OFFSET_Y + app.food.1 * CELL_SIZE + CELL_SIZE / 2) as f32;
                        
                        let color = match app.food_type {
                            FoodType::Normal => (0.9, 0.2, 0.2),
                            FoodType::Golden => (1.0, 0.8, 0.2),
                            FoodType::Poison => (0.6, 0.2, 0.8),
                        };
                        
                        for _ in 0..15 {
                            app.particles.push(Particle {
                                x: px, y: py,
                                vx: rand_float(-3.0, 3.0), vy: rand_float(-3.0, 3.0),
                                life: rand_float(10.0, 25.0), max_life: 25.0,
                                color,
                            });
                        }
                        
                        match app.food_type {
                            FoodType::Normal => {
                                app.score += 10;
                                sys_play_tone(880.0, 50, 1);
                            }
                            FoodType::Golden => {
                                app.score += 50;
                                app.glow_ticks = 90;
                                sys_play_tone(1200.0, 100, 2);
                            }
                            FoodType::Poison => {
                                app.score = app.score.saturating_sub(20);
                                sys_play_tone(300.0, 200, 3);
                                app.snake.pop();
                                app.snake.pop(); // Shrink extra!
                                if app.snake.len() < 2 {
                                    app.snake.truncate(2);
                                }
                            }
                        }
                        
                        // Spawn obstacles every 50 points (using threshold passing)
                        if app.score % 50 < 10 && app.score > 0 && app.food_type != FoodType::Poison {
                            if let Some(obs) = spawn_obstacle(&app.snake, &app.obstacles, app.food) {
                                app.obstacles.push(obs);
                            }
                        }
                        
                        app.speed = (6 - (app.score / 150)).max(2);
                        let (f, t) = spawn_food(&app.snake, &app.obstacles);
                        app.food = f;
                        app.food_type = t;
                    } else {
                        app.snake.pop();
                    }
                }
            }
            
            // Render
            app.win.x = x;
            app.win.y = y;
            app.win.draw_background(0.08, 0.1, 0.08);
            
            let score_text = format!("🍎 {}   ⭐ HI: {}", app.score, app.highscore);
            libui::draw_text_js(x as f32 + 15.0, y as f32 + 12.0, score_text.as_ptr(), score_text.len(), 18.0, 0.9, 0.9, 0.9, 1.0);
            
            let base_x = x + OFFSET_X;
            let base_y = y + OFFSET_Y;
            libui::draw_rect_js(base_x as f32 - 4.0, base_y as f32 - 4.0, (GRID_W * CELL_SIZE) as f32 + 8.0, (GRID_H * CELL_SIZE) as f32 + 8.0, 0.15, 0.25, 0.15, 1.0, 8.0, 0.0);
            libui::draw_rect_js(base_x as f32, base_y as f32, (GRID_W * CELL_SIZE) as f32, (GRID_H * CELL_SIZE) as f32, 0.05, 0.1, 0.05, 1.0, 4.0, 0.0);
            
            // Draw Obstacles
            for &(ox, oy) in &app.obstacles {
                libui::draw_rect_js((base_x + ox * CELL_SIZE) as f32, (base_y + oy * CELL_SIZE) as f32, CELL_SIZE as f32, CELL_SIZE as f32, 0.4, 0.4, 0.4, 1.0, 2.0, 0.0);
                libui::draw_rect_js((base_x + ox * CELL_SIZE) as f32 + 2.0, (base_y + oy * CELL_SIZE) as f32 + 2.0, CELL_SIZE as f32 - 4.0, CELL_SIZE as f32 - 4.0, 0.3, 0.3, 0.3, 1.0, 1.0, 0.0);
            }
            
            // Draw Food
            let fx = (base_x + app.food.0 * CELL_SIZE) as f32;
            let fy = (base_y + app.food.1 * CELL_SIZE) as f32;
            match app.food_type {
                FoodType::Normal => {
                    libui::draw_rect_js(fx, fy + 2.0, CELL_SIZE as f32, CELL_SIZE as f32 - 2.0, 0.9, 0.2, 0.2, 1.0, 6.0, 0.0);
                    libui::draw_rect_js(fx + 8.0, fy - 2.0, 4.0, 6.0, 0.2, 0.8, 0.2, 1.0, 2.0, 0.0);
                }
                FoodType::Golden => {
                    libui::draw_rect_js(fx, fy + 2.0, CELL_SIZE as f32, CELL_SIZE as f32 - 2.0, 1.0, 0.8, 0.2, 1.0, 6.0, 10.0);
                    libui::draw_rect_js(fx + 8.0, fy - 2.0, 4.0, 6.0, 0.8, 1.0, 0.2, 1.0, 2.0, 0.0);
                }
                FoodType::Poison => {
                    libui::draw_rect_js(fx, fy + 2.0, CELL_SIZE as f32, CELL_SIZE as f32 - 2.0, 0.6, 0.2, 0.8, 1.0, 6.0, 0.0);
                    libui::draw_rect_js(fx + 8.0, fy - 2.0, 4.0, 6.0, 0.4, 0.1, 0.6, 1.0, 2.0, 0.0);
                }
            }
            
            // Draw Particles
            for p in &app.particles {
                let a = p.life / p.max_life;
                libui::draw_rect_js(x as f32 + p.x, y as f32 + p.y, 4.0, 4.0, p.color.0, p.color.1, p.color.2, a, 2.0, 0.0);
            }
            
            // Draw Snake
            for (i, &(sx, sy)) in app.snake.iter().enumerate() {
                let is_head = i == 0;
                let mut r = if is_head { 0.3 } else { 0.2 };
                let mut g = if is_head { 0.9 } else { 0.6 };
                let mut b = if is_head { 0.3 } else { 0.2 };
                
                if app.glow_ticks > 0 {
                    r = 1.0; g = 0.8; b = 0.2; // Glow golden
                }
                
                let radius = if is_head { 6.0 } else { 4.0 };
                libui::draw_rect_js((base_x + sx * CELL_SIZE) as f32 + 1.0, (base_y + sy * CELL_SIZE) as f32 + 1.0, CELL_SIZE as f32 - 2.0, CELL_SIZE as f32 - 2.0, r, g, b, 1.0, radius, if app.glow_ticks > 0 { 5.0 } else { 0.0 });
                
                if is_head {
                    let (ex1, ey1, ex2, ey2) = match app.dir {
                        Direction::Right => (10.0, 4.0, 10.0, 10.0),
                        Direction::Left => (4.0, 4.0, 4.0, 10.0),
                        Direction::Up => (4.0, 4.0, 10.0, 4.0),
                        Direction::Down => (4.0, 10.0, 10.0, 10.0),
                    };
                    libui::draw_rect_js((base_x + sx * CELL_SIZE) as f32 + ex1, (base_y + sy * CELL_SIZE) as f32 + ey1, 3.0, 3.0, 0.0, 0.0, 0.0, 1.0, 1.5, 0.0);
                    libui::draw_rect_js((base_x + sx * CELL_SIZE) as f32 + ex2, (base_y + sy * CELL_SIZE) as f32 + ey2, 3.0, 3.0, 0.0, 0.0, 0.0, 1.0, 1.5, 0.0);
                }
            }
            
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
