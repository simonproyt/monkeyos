fn main() {
    println!("Hello from the new executable ABI!");
    let args: Vec<String> = std::env::args().collect();
    println!("Args: {:?}", args);
}
