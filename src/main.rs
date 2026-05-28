#[tokio::main]
async fn main() {
    if let Err(e) = pendulum_kelly_cli::run().await {
        eprintln!("Error: {}", e);
    }
}
