use std::fs;
pub fn run() {
    match fs::read_dir(".") {
        Ok(_) => println!("read_dir OK"),
        Err(e) => println!("read_dir ERROR: {:?}", e),
    }
}
