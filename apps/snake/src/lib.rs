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
enum FoodType { Normal, Golden, Poison, Dynamite, Ghost, Magnet }

#[derive(PartialEq)]
enum GameState { MainMenu, Playing, GameOver }

#[derive(Clone, Copy, PartialEq)]
enum Difficulty { Easy, Medium, Hard }

struct Particle {
    x: f32, y: f32, vx: f32, vy: f32,
    life: f32, max_life: f32,
    color: (f32, f32, f32),
}

struct SnakeGame {
    win: Window,
    state: GameState,
    snake: Vec<(i32, i32)>,
    dir: Direction,
    next_dir: Direction,
    food: (i32, i32),
    food_type: FoodType,
    score: u32,
    highscore: u32,
    frame_count: u32,
    speed: u32,
    sprinting: bool,
    wrapping: bool,
    obstacles: Vec<((i32, i32), (i32, i32))>,
    portals: Option<((i32, i32), (i32, i32))>,
    particles: Vec<Particle>,
    glow_ticks: u32,
    ghost_ticks: u32,
    magnet_ticks: u32,
    level: u32,
    stamina: f32,
    difficulty: Difficulty,
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

fn is_occupied(x: i32, y: i32, snake: &[(i32, i32)], obstacles: &[((i32, i32), (i32, i32))]) -> bool {
    snake.contains(&(x, y)) || obstacles.iter().any(|(pos, _)| pos.0 == x && pos.1 == y)
}

fn spawn_food(snake: &[(i32, i32)], obstacles: &[((i32, i32), (i32, i32))]) -> ((i32, i32), FoodType) {
    loop {
        let fx = rand_range(0, GRID_W);
        let fy = rand_range(0, GRID_H);
        if !is_occupied(fx, fy, snake, obstacles) {
            let r = rand_range(0, 100);
            
            if obstacles.len() >= 3 && r < 25 {
                return ((fx, fy), FoodType::Dynamite);
            }

            let ftype = if r < 10 && !obstacles.is_empty() { FoodType::Dynamite }
                        else if r < 20 { FoodType::Golden }
                        else if r < 30 { FoodType::Poison }
                        else if r < 40 { FoodType::Ghost }
                        else if r < 50 { FoodType::Magnet }
                        else { FoodType::Normal };
            return ((fx, fy), ftype);
        }
    }
}

fn spawn_obstacle(snake: &[(i32, i32)], obstacles: &[((i32, i32), (i32, i32))], food: (i32, i32)) -> Option<((i32, i32), (i32, i32))> {
    let cluster = !obstacles.is_empty() && rand_range(0, 100) < 40;
    let base = if cluster { obstacles[rand_range(0, obstacles.len() as i32) as usize].0 } else { (0, 0) };

    for _ in 0..50 {
        let mut ox = rand_range(1, GRID_W - 1);
        let mut oy = rand_range(1, GRID_H - 1);
        
        if cluster {
            ox = (base.0 + rand_range(-2, 3)).clamp(1, GRID_W - 2);
            oy = (base.1 + rand_range(-2, 3)).clamp(1, GRID_H - 2);
        }
        
        if !is_occupied(ox, oy, snake, obstacles) && (ox, oy) != food {
            // 50% chance for the obstacle to move
            let moving = rand_range(0, 100) < 50;
            let vel = if moving {
                if rand_range(0, 2) == 0 { (1, 0) } else { (0, 1) }
            } else { (0, 0) };
            return Some(((ox, oy), vel));
        }
    }
    None
}

fn load_highscore(wrapping: bool, difficulty: Difficulty) -> u32 {
    let diff_str = match difficulty {
        Difficulty::Easy => "easy",
        Difficulty::Medium => "medium",
        Difficulty::Hard => "hard",
    };
    let path = if wrapping { format!("/snake_hs_wrap_{}.txt", diff_str) } else { format!("/snake_hs_solid_{}.txt", diff_str) };
    if let Ok(data) = fs::read_to_string(&path) {
        if let Ok(val) = data.trim().parse::<u32>() {
            return val;
        }
    }
    0
}

fn save_highscore(score: u32, wrapping: bool, difficulty: Difficulty) {
    let diff_str = match difficulty {
        Difficulty::Easy => "easy",
        Difficulty::Medium => "medium",
        Difficulty::Hard => "hard",
    };
    let path = if wrapping { format!("/snake_hs_wrap_{}.txt", diff_str) } else { format!("/snake_hs_solid_{}.txt", diff_str) };
    let _ = fs::write(&path, score.to_string());
}

#[no_mangle]
pub extern "C" fn init() {
    let win = Window::new("Snake", 100, 100, OFFSET_X * 2 + GRID_W * CELL_SIZE, OFFSET_Y + 10 + GRID_H * CELL_SIZE);
    
    unsafe {
        APP = Some(SnakeGame {
            win,
            state: GameState::MainMenu,
            snake: Vec::new(),
            dir: Direction::Right,
            next_dir: Direction::Right,
            food: (0, 0),
            food_type: FoodType::Normal,
            score: 0,
            highscore: load_highscore(false, Difficulty::Medium),
            frame_count: 0,
            speed: 6,
            sprinting: false,
            wrapping: false,
            obstacles: Vec::new(),
            portals: None,
            particles: Vec::new(),
            glow_ticks: 0,
            ghost_ticks: 0,
            magnet_ticks: 0,
            level: 0,
            stamina: 100.0,
            difficulty: Difficulty::Medium,
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
    app.speed = match app.difficulty {
        Difficulty::Easy => 8,
        Difficulty::Medium => 6,
        Difficulty::Hard => 4,
    };
    app.state = GameState::Playing;
    app.particles.clear();
    app.glow_ticks = 0;
    app.level = 0;
    app.stamina = 100.0;
    app.sprinting = false;
    app.highscore = load_highscore(app.wrapping, app.difficulty);
    
    // Start fanfare
    unsafe { sys_play_tone(440.0, 100, 0); }
}

fn explode_particles(app: &mut SnakeGame, x: f32, y: f32, color: (f32, f32, f32), count: usize) {
    for _ in 0..count {
        app.particles.push(Particle {
            x, y,
            vx: rand_float(-3.0, 3.0), vy: rand_float(-3.0, 3.0),
            life: rand_float(10.0, 25.0), max_life: 25.0,
            color,
        });
    }
}

#[no_mangle]
pub extern "C" fn handle_key_down(key_code: u32) -> bool {
    let mut needs_redraw = false;
    unsafe {
        if let Some(app) = &mut APP {
            if app.state == GameState::MainMenu {
                if key_code == 13 || key_code == 32 { // Enter or Space
                    restart_game(app);
                    needs_redraw = true;
                } else if key_code == 119 || key_code == 87 { // W to toggle wrapping
                    app.wrapping = !app.wrapping;
                    app.highscore = load_highscore(app.wrapping, app.difficulty);
                    needs_redraw = true;
                } else if key_code == 113 || key_code == 81 { // Q to toggle difficulty
                    app.difficulty = match app.difficulty {
                        Difficulty::Easy => Difficulty::Medium,
                        Difficulty::Medium => Difficulty::Hard,
                        Difficulty::Hard => Difficulty::Easy,
                    };
                    app.highscore = load_highscore(app.wrapping, app.difficulty);
                    needs_redraw = true;
                }
                return needs_redraw;
            }

            if app.state == GameState::GameOver {
                if key_code == 13 || key_code == 32 {
                    app.state = GameState::MainMenu;
                    needs_redraw = true;
                }
                return needs_redraw;
            }

            if app.state == GameState::Playing {
                match key_code {
                    1037 | 65 | 97 if app.dir != Direction::Right => { app.next_dir = Direction::Left; needs_redraw = true; },
                    1038 | 87 | 119 if app.dir != Direction::Down => { app.next_dir = Direction::Up; needs_redraw = true; },
                    1039 | 68 | 100 if app.dir != Direction::Left => { app.next_dir = Direction::Right; needs_redraw = true; },
                    1040 | 83 | 115 if app.dir != Direction::Up => { app.next_dir = Direction::Down; needs_redraw = true; },
                    32 => { app.sprinting = !app.sprinting; needs_redraw = true; }, // Space to toggle sprint
                    _ => {}
                }
            }
        }
    }
    needs_redraw
}

#[no_mangle]
pub extern "C" fn handle_key_up(_key_code: u32) -> bool {
    false
}

#[no_mangle]
pub extern "C" fn handle_mouse_down(_mx: i32, _my: i32, _x: i32, _y: i32) -> bool {
    unsafe {
        if let Some(app) = &mut APP {
            if app.state == GameState::MainMenu {
                restart_game(app);
                return true;
            } else if app.state == GameState::GameOver {
                app.state = GameState::MainMenu;
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
            if app.ghost_ticks > 0 {
                app.ghost_ticks -= 1;
            }
            if app.magnet_ticks > 0 {
                app.magnet_ticks -= 1;
                if app.frame_count % 5 == 0 && app.food_type != FoodType::Poison {
                    let fx = app.food.0;
                    let fy = app.food.1;
                    let hx = app.snake[0].0;
                    let hy = app.snake[0].1;
                    let mut dx = 0; let mut dy = 0;
                    if fx < hx { dx = 1; } else if fx > hx { dx = -1; }
                    if fy < hy { dy = 1; } else if fy > hy { dy = -1; }
                    
                    let nfx = fx + dx;
                    let nfy = fy + dy;
                    if !is_occupied(nfx, nfy, &app.snake, &app.obstacles) {
                        app.food = (nfx, nfy);
                    } else if dx != 0 && !is_occupied(nfx, fy, &app.snake, &app.obstacles) {
                        app.food = (nfx, fy);
                    } else if dy != 0 && !is_occupied(fx, nfy, &app.snake, &app.obstacles) {
                        app.food = (fx, nfy);
                    }
                }
            }
            
            if app.state == GameState::Playing && app.frame_count % 8 == 0 {
                for obs in app.obstacles.iter_mut() {
                    let mut vel = obs.1;
                    if vel != (0, 0) {
                        let nx = obs.0.0 + vel.0;
                        let ny = obs.0.1 + vel.1;
                        if nx <= 0 || nx >= GRID_W - 1 || ny <= 0 || ny >= GRID_H - 1 || app.snake.contains(&(nx, ny)) {
                            vel.0 = -vel.0;
                            vel.1 = -vel.1;
                        } else {
                            obs.0 = (nx, ny);
                        }
                        obs.1 = vel;
                    }
                }
            }
            
            if app.state == GameState::Playing {
                if app.sprinting {
                    app.stamina -= 1.0;
                    if app.stamina <= 0.0 {
                        app.stamina = 0.0;
                        app.sprinting = false;
                    }
                } else {
                    app.stamina += 0.5;
                    if app.stamina > 100.0 {
                        app.stamina = 100.0;
                    }
                }
            }
            
            let current_speed = if app.sprinting { (app.speed / 2).max(1) } else { app.speed };
            
            if app.state == GameState::Playing && app.frame_count % current_speed == 0 {
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
                if app.wrapping {
                    new_head.0 = (new_head.0 + GRID_W) % GRID_W;
                    new_head.1 = (new_head.1 + GRID_H) % GRID_H;
                } else if new_head.0 < 0 || new_head.0 >= GRID_W || new_head.1 < 0 || new_head.1 >= GRID_H {
                    app.state = GameState::GameOver;
                    sys_play_tone(150.0, 500, 3);
                }
                
                if app.state == GameState::Playing {
                    if let Some(ports) = app.portals {
                        if new_head == ports.0 {
                            new_head = ports.1;
                            sys_play_tone(1500.0, 100, 2);
                        } else if new_head == ports.1 {
                            new_head = ports.0;
                            sys_play_tone(1500.0, 100, 2);
                        }
                    }
                    
                    if app.ghost_ticks == 0 && app.obstacles.iter().any(|(pos, _)| *pos == new_head) {
                        app.state = GameState::GameOver;
                        sys_play_tone(100.0, 600, 3);
                    } else {
                        for i in 0..app.snake.len() - 1 {
                            if app.snake[i] == new_head {
                                app.state = GameState::GameOver;
                                sys_play_tone(150.0, 500, 3);
                                break;
                            }
                        }
                    }
                }
                
                if app.state == GameState::GameOver {
                    app.sprinting = false;
                    if app.score > app.highscore {
                        app.highscore = app.score;
                        save_highscore(app.highscore, app.wrapping, app.difficulty);
                    }
                } else {
                    app.snake.insert(0, new_head);
                    
                    if new_head == app.food {
                        let px = (OFFSET_X + app.food.0 * CELL_SIZE + CELL_SIZE / 2) as f32;
                        let py = (OFFSET_Y + app.food.1 * CELL_SIZE + CELL_SIZE / 2) as f32;
                        
                        match app.food_type {
                            FoodType::Normal => {
                                explode_particles(app, px, py, (0.9, 0.2, 0.2), 15);
                                app.score += 10;
                                sys_play_tone(880.0, 50, 1);
                            }
                            FoodType::Golden => {
                                explode_particles(app, px, py, (1.0, 0.8, 0.2), 30);
                                app.score += 50;
                                app.glow_ticks = 90;
                                sys_play_tone(1200.0, 100, 2);
                            }
                            FoodType::Poison => {
                                explode_particles(app, px, py, (0.6, 0.2, 0.8), 20);
                                app.score = app.score.saturating_sub(20);
                                sys_play_tone(300.0, 200, 3);
                                app.snake.pop();
                                app.snake.pop();
                                if app.snake.len() < 2 {
                                    app.snake.truncate(2);
                                }
                            }
                                FoodType::Dynamite => {
                                explode_particles(app, px, py, (1.0, 0.4, 0.1), 50);
                                app.score += 20;
                                sys_play_tone(200.0, 400, 3); // Boom
                                
                                // Explode all obstacles!
                                while let Some((ox, _oy)) = app.obstacles.pop() {
                                    let opx = (OFFSET_X + ox.0 * CELL_SIZE + CELL_SIZE / 2) as f32;
                                    let opy = (OFFSET_Y + ox.1 * CELL_SIZE + CELL_SIZE / 2) as f32;
                                    explode_particles(app, opx, opy, (0.5, 0.5, 0.5), 10);
                                }
                            }
                            FoodType::Ghost => {
                                explode_particles(app, px, py, (1.0, 1.0, 1.0), 30);
                                app.score += 20;
                                app.ghost_ticks = 150;
                                sys_play_tone(900.0, 200, 2);
                            }
                            FoodType::Magnet => {
                                explode_particles(app, px, py, (0.2, 0.4, 1.0), 30);
                                app.score += 20;
                                app.magnet_ticks = 300;
                                sys_play_tone(1100.0, 200, 2);
                            }
                        }
                        
                        let new_level = app.score / 100;
                        if new_level > app.level {
                            sys_play_tone(600.0, 150, 0); // Level up!
                        }
                        app.level = new_level;
                        
                        // Organic portal spawning
                        let portal_threshold = match app.difficulty {
                            Difficulty::Easy => 150,
                            Difficulty::Medium => 100,
                            Difficulty::Hard => 50,
                        };
                        if app.score >= portal_threshold && rand_range(0, 100) < 15 {
                            if app.portals.is_some() {
                                app.portals = None; // Despawn
                            } else {
                                let mut attempts = 0;
                                let mut p1 = (0, 0);
                                let mut p2 = (0, 0);
                                while attempts < 100 {
                                    p1 = (rand_range(1, GRID_W - 1), rand_range(1, GRID_H - 1));
                                    p2 = (rand_range(1, GRID_W - 1), rand_range(1, GRID_H - 1));
                                    if !is_occupied(p1.0, p1.1, &app.snake, &app.obstacles) && 
                                       !is_occupied(p2.0, p2.1, &app.snake, &app.obstacles) && p1 != p2 {
                                        app.portals = Some((p1, p2));
                                        break;
                                    }
                                    attempts += 1;
                                }
                            }
                        }
                        
                        let obs_threshold = match app.difficulty {
                            Difficulty::Easy => 100,
                            Difficulty::Medium => 50,
                            Difficulty::Hard => 10,
                        };
                        if app.score >= obs_threshold && app.food_type != FoodType::Poison && app.food_type != FoodType::Dynamite {
                            let prob = match app.difficulty {
                                Difficulty::Easy => 10 + (app.score / 30).min(20) as i32,
                                Difficulty::Medium => 20 + (app.score / 20).min(30) as i32,
                                Difficulty::Hard => 40 + (app.score / 10).min(40) as i32,
                            };
                            if rand_range(0, 100) < prob {
                                if let Some(obs) = spawn_obstacle(&app.snake, &app.obstacles, app.food) {
                                    app.obstacles.push(obs);
                                }
                            }
                        }
                        
                        app.speed = match app.difficulty {
                            Difficulty::Easy => (8 - (app.score / 150)).max(3),
                            Difficulty::Medium => (6 - (app.score / 150)).max(2),
                            Difficulty::Hard => (4 - (app.score / 200)).max(1),
                        };
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
            
            let bg_color = match app.level % 4 {
                0 => (0.05, 0.05, 0.05), // Dark Gray
                1 => (0.0, 0.0, 0.1),    // Dark Blue
                2 => (0.0, 0.1, 0.0),    // Dark Green
                3 => (0.05, 0.0, 0.1),   // Dark Purple
                _ => (0.05, 0.05, 0.05),
            };
            app.win.draw_background(bg_color.0, bg_color.1, bg_color.2);
            
            let base_x = x + OFFSET_X;
            let base_y = y + OFFSET_Y;
            
            if app.state == GameState::MainMenu {
                libui::draw_text_js(x as f32 + 190.0, y as f32 + 80.0, "SNAKE".as_ptr(), 5, 48.0, 0.2, 0.8, 0.2, 1.0);
                
                let hi_text = format!("HIGH SCORE: {}", app.highscore);
                libui::draw_text_js(x as f32 + 180.0, y as f32 + 150.0, hi_text.as_ptr(), hi_text.len(), 20.0, 1.0, 1.0, 1.0, 1.0);
                
                let wrap_text = if app.wrapping { "WALLS: WRAPPING [Press W to toggle]" } else { "WALLS: SOLID [Press W to toggle]" };
                libui::draw_text_js(x as f32 + 120.0, y as f32 + 200.0, wrap_text.as_ptr(), wrap_text.len(), 16.0, 0.6, 0.6, 0.6, 1.0);
                
                let diff_str = match app.difficulty {
                    Difficulty::Easy => "DIFFICULTY: EASY [Press Q to change]",
                    Difficulty::Medium => "DIFFICULTY: MEDIUM [Press Q to change]",
                    Difficulty::Hard => "DIFFICULTY: HARD [Press Q to change]",
                };
                libui::draw_text_js(x as f32 + 120.0, y as f32 + 230.0, diff_str.as_ptr(), diff_str.len(), 16.0, 0.8, 0.6, 0.2, 1.0);
                
                libui::draw_text_js(x as f32 + 115.0, y as f32 + 280.0, "PRESS [ENTER] OR [SPACE] TO START".as_ptr(), 33, 18.0, 1.0, 1.0, 0.0, 1.0);
                
                libui::draw_text_js(x as f32 + 130.0, y as f32 + 320.0, "Press [SPACE] in-game to toggle SPRINT".as_ptr(), 38, 14.0, 0.4, 0.7, 1.0, 1.0);
                return;
            }
            
            let border_color = match app.level % 4 {
                0 => (0.4, 0.4, 0.4),
                1 => (0.2, 0.4, 0.8),
                2 => (0.8, 0.2, 0.2),
                3 => (0.6, 0.2, 0.8),
                _ => (0.4, 0.4, 0.4),
            };
            
            let score_text = format!("SCORE: {}   HI: {}", app.score, app.highscore);
            libui::draw_text_js(x as f32 + 15.0, y as f32 + 12.0, score_text.as_ptr(), score_text.len(), 18.0, 1.0, 1.0, 1.0, 1.0);
            
            let stamina_text = "STAMINA:";
            libui::draw_text_js(x as f32 + 280.0, y as f32 + 14.0, stamina_text.as_ptr(), stamina_text.len(), 12.0, 0.6, 0.6, 0.6, 1.0);
            
            let stamina_w = (app.stamina / 100.0) * 100.0;
            let sprint_color = if app.sprinting { (0.2, 0.8, 1.0) } else { (0.4, 0.8, 0.4) };
            libui::draw_rect_js(x as f32 + 350.0, y as f32 + 12.0, 100.0, 12.0, 0.2, 0.2, 0.2, 1.0, 0.0, 0.0);
            libui::draw_rect_js(x as f32 + 350.0, y as f32 + 12.0, stamina_w, 12.0, sprint_color.0, sprint_color.1, sprint_color.2, 1.0, 0.0, 0.0);
            
            libui::draw_rect_js(base_x as f32 - 2.0, base_y as f32 - 2.0, (GRID_W * CELL_SIZE) as f32 + 4.0, (GRID_H * CELL_SIZE) as f32 + 4.0, border_color.0, border_color.1, border_color.2, 1.0, 0.0, 0.0);
            libui::draw_rect_js(base_x as f32, base_y as f32, (GRID_W * CELL_SIZE) as f32, (GRID_H * CELL_SIZE) as f32, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0);
            
            for &((ox, oy), vel) in &app.obstacles {
                let moving = vel != (0, 0);
                let color = if moving { (0.6, 0.4, 0.4) } else { (0.5, 0.5, 0.5) };
                let inner = if moving { (0.4, 0.2, 0.2) } else { (0.3, 0.3, 0.3) };
                libui::draw_rect_js((base_x + ox * CELL_SIZE) as f32, (base_y + oy * CELL_SIZE) as f32, CELL_SIZE as f32, CELL_SIZE as f32, color.0, color.1, color.2, 1.0, 0.0, 0.0);
                libui::draw_rect_js((base_x + ox * CELL_SIZE) as f32 + 2.0, (base_y + oy * CELL_SIZE) as f32 + 2.0, CELL_SIZE as f32 - 4.0, CELL_SIZE as f32 - 4.0, inner.0, inner.1, inner.2, 1.0, 0.0, 0.0);
            }
            
            if let Some((p1, p2)) = app.portals {
                let s = (app.frame_count % 20) as f32 / 20.0;
                let c1 = (0.2 + s * 0.8, 0.0, 0.8 - s * 0.4);
                let c2 = (0.8 - s * 0.4, 0.0, 0.2 + s * 0.8);
                libui::draw_rect_js((base_x + p1.0 * CELL_SIZE) as f32, (base_y + p1.1 * CELL_SIZE) as f32, CELL_SIZE as f32, CELL_SIZE as f32, c1.0, c1.1, c1.2, 1.0, 0.0, 0.0);
                libui::draw_rect_js((base_x + p2.0 * CELL_SIZE) as f32, (base_y + p2.1 * CELL_SIZE) as f32, CELL_SIZE as f32, CELL_SIZE as f32, c2.0, c2.1, c2.2, 1.0, 0.0, 0.0);
                libui::draw_rect_js((base_x + p1.0 * CELL_SIZE) as f32 + 4.0, (base_y + p1.1 * CELL_SIZE) as f32 + 4.0, CELL_SIZE as f32 - 8.0, CELL_SIZE as f32 - 8.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0);
                libui::draw_rect_js((base_x + p2.0 * CELL_SIZE) as f32 + 4.0, (base_y + p2.1 * CELL_SIZE) as f32 + 4.0, CELL_SIZE as f32 - 8.0, CELL_SIZE as f32 - 8.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0);
            }
            
            let fx = (base_x + app.food.0 * CELL_SIZE) as f32;
            let fy = (base_y + app.food.1 * CELL_SIZE) as f32;
            match app.food_type {
                FoodType::Normal => {
                    libui::draw_rect_js(fx + 2.0, fy + 2.0, CELL_SIZE as f32 - 4.0, CELL_SIZE as f32 - 4.0, 1.0, 0.2, 0.2, 1.0, 0.0, 0.0);
                }
                FoodType::Golden => {
                    libui::draw_rect_js(fx + 2.0, fy + 2.0, CELL_SIZE as f32 - 4.0, CELL_SIZE as f32 - 4.0, 1.0, 0.8, 0.0, 1.0, 0.0, 0.0);
                }
                FoodType::Poison => {
                    libui::draw_rect_js(fx + 2.0, fy + 2.0, CELL_SIZE as f32 - 4.0, CELL_SIZE as f32 - 4.0, 0.8, 0.2, 1.0, 1.0, 0.0, 0.0);
                }
                FoodType::Dynamite => {
                    libui::draw_rect_js(fx + 2.0, fy + 2.0, CELL_SIZE as f32 - 4.0, CELL_SIZE as f32 - 4.0, 1.0, 0.5, 0.0, 1.0, 0.0, 0.0);
                    libui::draw_rect_js(fx + 6.0, fy - 2.0, 4.0, 4.0, 1.0, 1.0, 0.5, 1.0, 0.0, 0.0); // Spark
                }
                FoodType::Ghost => {
                    libui::draw_rect_js(fx + 2.0, fy + 2.0, CELL_SIZE as f32 - 4.0, CELL_SIZE as f32 - 4.0, 1.0, 1.0, 1.0, 0.8, 0.0, 0.0);
                }
                FoodType::Magnet => {
                    libui::draw_rect_js(fx + 2.0, fy + 2.0, CELL_SIZE as f32 - 4.0, CELL_SIZE as f32 - 4.0, 0.2, 0.4, 1.0, 1.0, 0.0, 0.0);
                }
            }
            
            for p in &app.particles {
                let a = p.life / p.max_life;
                libui::draw_rect_js(x as f32 + p.x, y as f32 + p.y, 4.0, 4.0, p.color.0, p.color.1, p.color.2, a, 0.0, 0.0);
            }
            
            for (i, &(sx, sy)) in app.snake.iter().enumerate() {
                let is_head = i == 0;
                
                let mut r = 0.2;
                let mut g = 0.8;
                let mut b = 0.2;
                let mut a = 1.0;
                
                if app.ghost_ticks > 0 {
                    r = 1.0; g = 1.0; b = 1.0; a = 0.5;
                } else if app.glow_ticks > 0 {
                    r = 1.0; g = 0.9; b = 0.2;
                } else if is_head {
                    r = 0.3; g = 1.0; b = 0.3;
                }
                
                if app.magnet_ticks > 0 {
                    let mut b_aura = 1.0;
                    if app.frame_count % 10 < 5 { b_aura = 0.5; }
                    libui::draw_rect_js((base_x + sx * CELL_SIZE) as f32, (base_y + sy * CELL_SIZE) as f32, CELL_SIZE as f32, CELL_SIZE as f32, 0.0, 0.2, b_aura, 0.3, 0.0, 0.0);
                }
                
                libui::draw_rect_js((base_x + sx * CELL_SIZE) as f32 + 1.0, (base_y + sy * CELL_SIZE) as f32 + 1.0, CELL_SIZE as f32 - 2.0, CELL_SIZE as f32 - 2.0, r, g, b, a, 0.0, 0.0);
                
                if is_head {
                    let (ex1, ey1, ex2, ey2) = match app.dir {
                        Direction::Right => (10.0, 4.0, 10.0, 10.0),
                        Direction::Left => (4.0, 4.0, 4.0, 10.0),
                        Direction::Up => (4.0, 4.0, 10.0, 4.0),
                        Direction::Down => (4.0, 10.0, 10.0, 10.0),
                    };
                    libui::draw_rect_js((base_x + sx * CELL_SIZE) as f32 + ex1, (base_y + sy * CELL_SIZE) as f32 + ey1, 2.0, 2.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0);
                    libui::draw_rect_js((base_x + sx * CELL_SIZE) as f32 + ex2, (base_y + sy * CELL_SIZE) as f32 + ey2, 2.0, 2.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0);
                }
            }
            
            if app.state == GameState::GameOver {
                let go_text = "GAME OVER";
                let restart_text = "Click or Press Enter to return to Menu";
                libui::draw_rect_js(base_x as f32, base_y as f32 + (GRID_H * CELL_SIZE / 2 - 40) as f32, (GRID_W * CELL_SIZE) as f32, 80.0, 0.0, 0.0, 0.0, 0.8, 0.0, 0.0);
                libui::draw_text_js(base_x as f32 + 150.0, base_y as f32 + (GRID_H * CELL_SIZE / 2 - 20) as f32, go_text.as_ptr(), go_text.len(), 24.0, 1.0, 0.2, 0.2, 1.0);
                libui::draw_text_js(base_x as f32 + 70.0, base_y as f32 + (GRID_H * CELL_SIZE / 2 + 10) as f32, restart_text.as_ptr(), restart_text.len(), 14.0, 0.8, 0.8, 0.8, 1.0);
            }
        }
    }
}
