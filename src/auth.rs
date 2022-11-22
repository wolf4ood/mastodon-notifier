use anyhow::Result;
use keyring::Entry;

pub fn get(app: &str, username: &str) -> Result<String> {
    let entry = Entry::new(app, username);

    entry.get_password().map(Ok)?
}

pub fn set(app: &str, username: &str, token: &str) -> Result<()> {
    let entry = Entry::new(app, username);

    entry.set_password(token).map(Ok)?
}
