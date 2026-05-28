fn main() {
    if let Err(e) = pendulum_kelly_cli::run() {
        eprintln!("Error: {}", e);
    }
}
