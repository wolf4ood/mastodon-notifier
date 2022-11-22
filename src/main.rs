use anyhow::Result;
use clap::Parser;
use mastodon_notifier::{
    config, daemon,
    opts::{Mode, Opts},
};

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let args = Opts::parse();

    match args.mode {
        Mode::Config => config::wizard(args).await?,
        Mode::Daemon => daemon::run(args).await?,
    }

    Ok(())
}
