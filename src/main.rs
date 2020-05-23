use std::collections::HashMap;
use std::env;

mod auth;
mod cli;
mod dl;
mod metadata;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let env: HashMap<String, String> = env::vars().collect();

    let client_id = env
        .get("FERROTYPE_CLIENT_ID")
        .expect("Didn't get a FERROTYPE_CLIENT_ID");
    let secret = env
        .get("FERROTYPE_SECRET")
        .expect("Didn't get a FERROTYPE_SECRET");

    let key = env.get("FERROTYPE_KEY");

    let credentials = match key {
        Some(key) => auth::Credentials::dummy(key),
        None => auth::authorize(client_id, secret).await?,
    };

    metadata::fetch(credentials).await?;

    Ok(())

    // [ ] 1. scan fs to determine local state of the world
    // [x] 2. download metadata from the GP API to determine remote state of the world
    // [x] 2.1 Auth
    // [x] 2.2 Download subsequent pages
    // [x] 2.3 Refresh auth
    // [ ] 3. only download changed files
    // [x] 4. start downloading
    // [x] 5. respect API limits
    // [ ] 6. do this once every X hours
    // [ ] 7. Download to a temporary file and then execute a mv to prevent corruption
    // [ ] 8. Download non-images correctly
    // [x] 9. "Deployment"
    // [ ] 10. Deployment on Docker
    // [ ] 11. Log levels + timestamps
}
