use clap::Parser;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Opts {
    // Mastodon instance host eg hackyderm.io
    #[arg(long)]
    pub host: String,
    #[arg(long)]
    pub user: String,
    #[arg(value_enum, long)]
    pub mode: Mode,
    /// Expiration timeout of the notification
    #[arg(long, default_value_t = 5000)]
    pub timeout: u32,

    /// Icon to display freedesktop.org compliant eg dialog-information
    #[arg(long)]
    pub icon: Option<String>,
}

impl Opts {
    pub fn account(&self) -> String {
        format!("{}@{}", self.user, self.host)
    }
}

#[derive(clap::ValueEnum, Clone, Debug)]
pub enum Mode {
    Config,
    Daemon,
}
