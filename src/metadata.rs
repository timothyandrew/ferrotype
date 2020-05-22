//! Download, examine, and store _metadata_ about the media we're attempting to fetch.

use crate::auth::Credentials;
use reqwest::{Client, Url};

const LIST_URL: &str = "https://photoslibrary.googleapis.com/v1/mediaItems";

/// Fetch all metadata
///
/// ## Limits
/// - The API allows 100 metadata items to be fetched per request.
/// - Pagination uses a continuation token, so this can't be parallelized.
pub async fn fetch(credentials: Credentials) -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::new();

    let params = vec![
      ("pageSize", "100"),
      ("pageToken", "")
    ];

    let url = Url::parse_with_params(LIST_URL, &params)?;

    let request = client.get(url).header("Authorization", format!("Bearer {}", credentials.get_key()));
    let response = request.send().await?;
    println!("{}", response.text().await?);

    Ok(())
}