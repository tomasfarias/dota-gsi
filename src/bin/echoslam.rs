use clap::Parser;

use dota::{GSIServer, components::GameState};

/// Echo back Dota GameState integration state.
async fn echo_gamestate_handler(gs: GameState) {
    println!("{}", gs);
}

/// Echo back raw JSON events.
async fn echo_json_handler(value: serde_json::Value) {
    println!("{}", value);
}

/// Listen for Dota 2 events and echo (slam) them.
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// URI for the server to listen for events.
    /// This must be the same URI used in the Game State configuration file.
    #[arg(short, long)]
    uri: String,

    /// Don't attempt to parse JSON data.
    /// Echo raw JSON events as received from Dota 2.
    #[arg(short, long)]
    raw: bool,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    let args = Args::parse();

    let server = GSIServer::new(&args.uri);

    if args.raw {
        server.run(echo_json_handler).await?;
    } else {
        server.run(echo_gamestate_handler).await?;
    }

    Ok(())
}
