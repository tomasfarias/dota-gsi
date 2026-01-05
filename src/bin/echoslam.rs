use clap::Parser;
use serde::de::DeserializeOwned;

use dota::{Server, components::GameState};

/// Echo back game state integration events.
async fn echo_handler<T>(bytes: bytes::Bytes)
where
    T: DeserializeOwned + std::fmt::Display,
{
    let value: T = match serde_json::from_slice(&bytes) {
        Err(e) => {
            log::error!("Failed to deserialize JSON body: {}", e);
            panic!("deserialize error");
        }
        Ok(v) => v,
    };

    println!("{:#}", value);
}

/// Listen for events and echo (slam) them.
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// URI for the server to listen for events.
    /// This must be the same URI used in the game state integration configuration file.
    #[arg(short, long)]
    uri: String,

    /// Whether to deserialize JSON data or not.
    /// When `true` then events are deserialized to a [`GameState`].
    #[arg(short, long)]
    raw: bool,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    let args = Args::parse();

    let mut server = Server::new(&args.uri);

    if args.raw {
        server = server.register(echo_handler::<serde_json::Value>);
    } else {
        server = server.register(echo_handler::<GameState>);
    }

    server.serve().await?;

    Ok(())
}
