use std::collections::HashMap;
use std::{env, process};

mod auth;
mod cli;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let env: HashMap<String, String> = env::vars().collect();

    let client_id = env
        .get("FERROTYPE_CLIENT_ID")
        .expect("Didn't get a FERROTYPE_CLIENT_ID");
    let secret = env
        .get("FERROTYPE_SECRET")
        .expect("Didn't get a FERROTYPE_SECRET");

    auth::authorize(client_id, secret).await?;

    Ok(())

    // 1. scan fs to determine local state of the world
    // 2. download metadata from the GP API to determine remote state of the world
    // 2.1 Auth
    // 3. diff 1. and 2. to determine what to download
    // 4. start downloading
    // 5. respect API limits
    // 6. do this once every X hours
}
