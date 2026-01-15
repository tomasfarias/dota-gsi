use std::path::PathBuf;

use async_trait::async_trait;
use clap::Parser;
use tokio::fs::File;
use tokio::io::AsyncWriteExt;

use dota::{Handler, HandlerResult, ServerBuilder};

#[derive(Clone, Debug)]
struct RecallHandler {
    output_dir: PathBuf,
}

#[async_trait]
impl Handler for RecallHandler {
    /// Save raw GameState Integration as JSON for later recalling
    async fn handle(&self, event: bytes::Bytes) -> HandlerResult {
        let file_name = format!("DotaGSI_{}.json", chrono::offset::Local::now());
        let mut file_path = self.output_dir.clone();
        file_path.push(file_name);

        let mut file = File::create(file_path)
            .await
            .expect("Failed to create file for DotaGSI JSON.");
        file.write_all(&event).await?;

        Ok(())
    }
}

/// Listen for Dota 2 events to store them as JSON for recalling later.
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// URI for the server to listen for events.
    /// This must be the same URI used in the Game State configuration file.
    uri: String,

    /// Optional directory where to store JSON event files.
    #[arg(short, long)]
    output_dir: Option<PathBuf>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    let args = Args::parse();
    let output_dir = match args.output_dir {
        Some(p) => p,
        None => {
            std::env::current_dir().expect("Not enough permissions to write to current directory.")
        }
    };

    let handler = RecallHandler {
        output_dir: output_dir.clone(),
    };

    ServerBuilder::new(&args.uri)
        .register(handler)
        .start()?
        .run_forever()
        .await;

    Ok(())
}
