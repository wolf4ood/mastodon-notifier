pub mod auth;
mod client;
pub mod notification;
pub mod opts;
pub use client::MastoClient;
pub mod config;
pub mod daemon;
pub mod util;

pub const APP_NAME: &str = "mastodon-notify";
