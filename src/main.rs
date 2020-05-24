use std::collections::HashMap;
use std::env;

#[macro_use]
extern crate lazy_static;

// TODO: Move into `lib.rs`?
mod api;
mod auth;
mod cli;
mod dl;
mod metadata;
mod statistics;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let env: HashMap<String, String> = env::vars().collect();

    let client_id = env
        .get("FERROTYPE_CLIENT_ID")
        .expect("Didn't get a FERROTYPE_CLIENT_ID");
    let secret = env
        .get("FERROTYPE_SECRET")
        .expect("Didn't get a FERROTYPE_SECRET");

    let key = env.get("FERROTYPE_REFRESH_TOKEN");

    let credentials = match key {
        Some(key) => {
            let credentials = auth::Credentials::with_refresh_token(key, client_id, secret);
            credentials.refresh().await?
        }
        None => auth::authorize(client_id, secret).await?,
    };

    // Force this `lazy_static` to load
    statistics::get();

    metadata::fetch(credentials).await?;

    Ok(())

    // [-] 1. scan fs to determine local state of the world
    // [x] 2. download metadata from the GP API to determine remote state of the world
    // [x] 2.1 Auth
    // [x] 2.2 Download subsequent pages
    // [x] 2.3 Refresh auth
    // [x] 3. only download changed files
    // [x] 4. start downloading
    // [x] 5. respect API limits
    // [ ] 6. do this once every X hours
    // [ ] 7. Download to a temporary file and then execute a mv to prevent corruption
    // [x] 8. Download non-images correctly
    // [x] 8.1 Any photo could be a motion photo; try with `dv`
    // [ ] 8.2 Detect mime type dynamically to deal with motion photo weirdness
    // [x] 8.3 Update quotas
    // [x] 9. "Deployment"
    // [ ] 10. Deployment on Docker
    // [ ] 11. Log levels + timestamps
    // [x] 12. Init with a refresh token
    // [ ] 13. Log per-page metrics
    // [x] 14. Cache information about which photos are definitely not motion photos to improve incremental runtime
    // [x] 15. Don't track quota information at all; wait until the first 429 and then panic/stop instead
    // [ ] 16. Backoff + retry
}
