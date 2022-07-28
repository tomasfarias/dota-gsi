use dota::{components::GameState, GSIServer};

fn echo_handler(gs: GameState) {
    println!("{}", gs);
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    let uri = "127.0.0.1:3000";
    let server = GSIServer::new(uri);
    server.run(echo_handler).await?;

    Ok(())
}
