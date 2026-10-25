#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    aevio_server::app::views::run().await
}
