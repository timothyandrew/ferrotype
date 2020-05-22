//! Download, examine, and store _metadata_ about the media we're attempting to fetch.

use crate::auth::Credentials;
use reqwest::{Client, StatusCode, Url};
use serde::Deserialize;

const LIST_URL: &str = "https://photoslibrary.googleapis.com/v1/mediaItems";

#[derive(Deserialize, Debug)]
struct Photo;

#[derive(Deserialize, Debug)]
struct Video;

#[derive(Deserialize, Debug)]
enum MediaItemType {
    photo(Photo),
    video(Video),
}

#[derive(Deserialize, Debug)]
struct MediaItemMetadata {
    #[serde(flatten)]
    mediaType: MediaItemType,
}

#[derive(Deserialize, Debug)]
struct MediaItem {
    id: String,
    baseUrl: String,
    mimeType: String,
    mediaMetadata: MediaItemMetadata,
}

#[derive(Deserialize, Debug)]
struct MetadataResponse {
    mediaItems: Vec<MediaItem>,
    nextPageToken: String,
}

/// Fetch all metadata
///
/// ## Limits
/// - The API allows 100 metadata items to be fetched per request.
/// - Pagination uses a continuation token, so this can't be parallelized.
pub async fn fetch(credentials: Credentials) -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::new();

    let params = vec![("pageSize", "1"), ("pageToken", "")];

    let url = Url::parse_with_params(LIST_URL, &params)?;

    let request = client
        .get(url)
        .header("Authorization", format!("Bearer {}", credentials.get_key()));
    let response = request.send().await?;

    match response.status() {
        StatusCode::OK => {
            println!("{:?}", response.json::<MetadataResponse>().await?);
        }
        // TODO: Do something more graceful here
        _ => panic!("non 200!"),
    }

    Ok(())
}
