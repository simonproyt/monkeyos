#[link(wasm_import_module = "env")]
unsafe extern "C" {
    fn sys_play_tone(freq: f32, duration_ms: u32, wave_type: u32);
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let mut freq = 440.0;
    let mut dur = 200;
    let mut wave = 0; // Sine

    if args.len() > 1 {
        if let Ok(f) = args[1].parse::<f32>() {
            freq = f;
        }
    }
    if args.len() > 2 {
        if let Ok(d) = args[2].parse::<u32>() {
            dur = d;
        }
    }
    if args.len() > 3 {
        if let Ok(w) = args[3].parse::<u32>() {
            wave = w;
        }
    }

    println!("Beep: freq={}Hz, dur={}ms, wave={}", freq, dur, wave);
    unsafe { sys_play_tone(freq, dur, wave); }
}
