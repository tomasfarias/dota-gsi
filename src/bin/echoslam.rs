use clap::Parser;

use dota::{ServerBuilder, components::GameState, handlers::echo_handler};

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

    let mut builder = ServerBuilder::new(&args.uri);

    if args.raw {
        builder = builder.register(echo_handler::<serde_json::Value>);
    } else {
        builder = builder.register(echo_handler::<GameState>);
    }

    let server = builder.start()?;
    server.run_forever().await;

    Ok(())
}
