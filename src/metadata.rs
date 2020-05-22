//! Download, examine, and store _metadata_ about the media we're attempting to fetch.

#![allow(non_snake_case)]

use crate::auth::Credentials;
use chrono::{DateTime, Utc};
use reqwest::{Client, StatusCode, Url};
use serde::Deserialize;

use crate::dl;

const LIST_URL: &str = "https://photoslibrary.googleapis.com/v1/mediaItems";
const PAGE_SIZE: &str = "100";

#[derive(Deserialize, Debug)]
struct Photo {}

#[derive(Deserialize, Debug)]
struct Video {}

#[derive(Deserialize, Debug)]
enum MediaItemType {
    photo(Photo),
    video(Video),
}

#[derive(Deserialize, Debug)]
struct MediaItemMetadata {
    creationTime: DateTime<Utc>,
    #[serde(flatten)]
    mediaType: MediaItemType,
}

#[derive(Deserialize, Debug)]
pub struct MediaItem {
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

impl MediaItem {
    pub fn created_at(&self) -> &DateTime<Utc> {
        &self.mediaMetadata.creationTime
    }

    pub fn url(&self) -> &str {
        &self.baseUrl
    }

    pub fn filename(&self) -> String {
        match self.mimeType.as_ref() {
            "image/jpeg" => format!("{}.jpg", self.id),
            "image/png" => format!("{}.png", self.id),
            "video/mp4" => format!("{}.mp4", self.id),
            _ => self.id.to_string(),
        }
    }
}

enum FetchResult {
    MorePagesExist(String),
    NoMorePagesExist
}

async fn fetch_page(credentials: &Credentials, pageToken: &str) -> Result<FetchResult, Box<dyn std::error::Error>> {
    let client = Client::new();
    let params = vec![("pageSize", PAGE_SIZE), ("pageToken", pageToken)];
    let url = Url::parse_with_params(LIST_URL, &params)?;

    let request = client
        .get(url)
        .header("Authorization", format!("Bearer {}", credentials.get_key()));
    let response = request.send().await?;

    match response.status() {
        StatusCode::OK => {
            let response: MetadataResponse = response.json::<MetadataResponse>().await?;

            // TODO: Only download missing media items
            dl::download_media_items(&response.mediaItems, "/tmp/foo").await?;

            let result = if response.nextPageToken.is_empty() {
                FetchResult::NoMorePagesExist
            } else {
                FetchResult::MorePagesExist(response.nextPageToken)
            };

            Ok(result)
        }
        // TODO: Do something more graceful here
        _ => panic!("non 200!"),
    }
}

/// Fetch all metadata. By design, this module is very specific to this use-case, and
/// isn't structured as a generic downloader library. It accepts a `prefix`, and downloads
/// to a `YYYY/YYYY-MM-DD/filename` structure under it.
///
/// ## Limits
/// - The API allows 100 metadata items to be fetched per request.
/// - Pagination uses a continuation token, so this can't be parallelized.
pub async fn fetch(credentials: Credentials) -> Result<(), Box<dyn std::error::Error>> {
    let mut pageToken = String::new();
    let mut counter = 1;

    loop {
        println!("Downloading page #{}", counter);
        counter += 1;

        match fetch_page(&credentials, &pageToken).await? {
            FetchResult::MorePagesExist(next) => pageToken = next,
            FetchResult::NoMorePagesExist => {
                println!("All done!");
                return Ok(())
            }
        }
    }
}
