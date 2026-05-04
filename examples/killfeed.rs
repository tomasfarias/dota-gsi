use clap::Parser;

use dota::{
    ServerBuilder,
    event::{GameEvent, Player},
    handlers::diff::DiffHandler,
};

/// Listen for events and produce a kill feed.
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// URI for the server to listen for events.
    /// This must be the same URI used in the game state integration configuration file.
    #[arg(short, long)]
    uri: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    let args = Args::parse();

    let handler = DiffHandler::new(|events| async move {
        for event in events {
            match event {
                GameEvent::PlayerEvent(Player::SecuredKill {
                    name,
                    kills,
                    streak,
                }) => {
                    println!(
                        "[KILL] {} secured a kill (total kills: {}, current streak: {})",
                        name, kills, streak
                    );
                }
                GameEvent::PlayerEvent(Player::Died { name, deaths }) => {
                    println!("[DEATH] {} died (total deaths: {})", name, deaths);
                }
                _ => {} // ignore everything else
            }
        }
        Ok(())
    });

    let mut builder = ServerBuilder::new(&args.uri);
    builder = builder.register_mut(handler);

    let server = builder.start()?;
    server.run_forever().await;

    Ok(())
}
