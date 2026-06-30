use std::env;
use std::fs::OpenOptions;
use std::io::{self, BufRead, Write};

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: edit <filename>");
        return;
    }

    let filename = &args[1];

    let content = std::fs::read_to_string(filename).unwrap_or_default();

    println!("Editing: {}", filename);
    if !content.is_empty() {
        println!("--- Existing Content ---");
        print!("{}", content);
        if !content.ends_with('\n') {
            println!();
        }
        println!("------------------------");
    }

    println!("Type your text below. Type '.exit' on a new line to save and quit.");
    
    let stdin = io::stdin();
    let mut new_lines = String::new();
    
    for line_result in stdin.lock().lines() {
        if let Ok(line) = line_result {
            if line == ".exit" {
                break;
            }
            new_lines.push_str(&line);
            new_lines.push('\n');
        } else {
            break;
        }
    }

    let mut final_content = content;
    final_content.push_str(&new_lines);

    match OpenOptions::new().write(true).create(true).truncate(true).open(filename) {
        Ok(mut file) => {
            if let Err(e) = file.write_all(final_content.as_bytes()) {
                eprintln!("Failed to save: {}", e);
            } else {
                println!("Saved {} bytes to {}", final_content.len(), filename);
            }
        }
        Err(e) => {
            eprintln!("Failed to open file for writing: {}", e);
        }
    }
}
