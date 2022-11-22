use anyhow::Result;
use std::process::Command;

pub fn open_browser(url: &str) -> Result<()> {
    let browser =
        std::env::var_os("BROWSER").ok_or_else(|| anyhow::anyhow!("Missing browser env"))?;

    Command::new(browser).arg(url).status()?;
    Ok(())
}
