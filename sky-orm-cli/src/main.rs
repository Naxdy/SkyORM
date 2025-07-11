#![allow(clippy::unwrap_used)]

mod schema;

use clap::{Parser, Subcommand};
use schema::GenerateSchema;
use tracing::{error, level_filters::LevelFilter};
use tracing_subscriber::{
    fmt::{format, layer},
    layer::SubscriberExt,
    util::SubscriberInitExt,
};

#[derive(Subcommand, Debug)]
enum Subcommands {
    GenerateSchema(GenerateSchema),
}

#[derive(Parser, Debug)]
struct Args {
    #[command(subcommand)]
    command: Subcommands,
}

#[tokio::main]
async fn main() {
    let args = Args::parse();

    tracing_subscriber::registry()
        .with(LevelFilter::INFO)
        .with(layer().event_format(format().without_time().with_target(false).compact()))
        .init();

    let r = match args.command {
        Subcommands::GenerateSchema(cmd) => cmd.run().await,
    };

    if let Err(e) = r {
        error!("Command execution failed: {e}");
    }
}
