//! Download, examine, and store _metadata_ about the media we're attempting to fetch.

#![allow(non_snake_case)]
#![allow(non_camel_case_types)]

use crate::auth::Credentials;
use chrono::{DateTime, Utc};
use reqwest::{Client, Url};
use tokio::time;
use std::time::Duration;
use serde::Deserialize;

use crate::dl;
use crate::statistics;
use crate::metrics;

const LIST_URL: &str = "https://photoslibrary.googleapis.com/v1/mediaItems";
const PAGE_SIZE: usize = 100;
const RETRY_ATTEMPTS: usize = 3;

#[derive(Debug)]
struct UsageAgainstQuota {
    metadata: usize,
    download: usize,
}

#[derive(Deserialize, Debug)]
pub struct Photo {}

#[derive(Deserialize, Debug)]
#[warn(non_camel_case_types)]
pub struct Video {}

#[derive(Deserialize, Debug)]
pub enum MediaItemType {
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

    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn media_type(&self) -> &MediaItemType {
        &self.mediaMetadata.mediaType
    }

    pub fn download_urls(&self) -> Vec<String> {
        let non_motion_photos = statistics::get().non_motion_photos();

        // Based on statistics from a previous run, this is a non-motionphoto.
        if non_motion_photos.contains(&self.id) {
            metrics::tick("non_motion_photo_cache_hit");
            vec![format!("{}=d", self.baseUrl)]
        } else {
            match self.mediaMetadata.mediaType {
                MediaItemType::photo(_) => vec![
                    format!("{}=d", self.baseUrl),
                    format!("{}=dv", self.baseUrl),
                ],
                MediaItemType::video(_) => vec![format!("{}=dv", self.baseUrl)],
            }
        }
    }
}

enum FetchResult {
    MorePagesExist(String),
    NoMorePagesExist,
}

async fn fetch_page(
    credentials: &Credentials,
    pageToken: &str,
) -> Result<FetchResult, Box<dyn std::error::Error>> {
    let mut attempt = 0;
    let client = Client::new();
   
    while attempt < RETRY_ATTEMPTS {
        let params = vec![
            ("pageSize", PAGE_SIZE.to_string()),
            ("pageToken", pageToken.to_string()),
        ];

        let url = Url::parse_with_params(LIST_URL, &params)?;

        let request = client
            .get(url)
            .header("Authorization", format!("Bearer {}", credentials.get_key()));
        let response = request.send().await?;

        match response.status().as_u16() {
            200 => {
                metrics::tick("metadata_dl_200");
                let response: MetadataResponse = response.json::<MetadataResponse>().await?;

                // TODO: Parameter for dl location
                dl::download_media_items(&response.mediaItems, "/mnt/z/ferrotype").await?;

                let result = if response.nextPageToken.is_empty() {
                    FetchResult::NoMorePagesExist
                } else {
                    FetchResult::MorePagesExist(response.nextPageToken)
                };

                return Ok(result);
            }
            401 => {
                // We shouldn't ever hit this line if the refresh token flow
                // is working as expected.
                panic!("Authorization failed!")
            }
            429 => panic!("We've hit the rate limit, try again later! (rate limits reset at midnight PT)"),
            500..=599 => {
                // TODO: Exponential backoff
                metrics::tick("retry_metadata_dl_after_5xx");
                time::delay_for(Duration::from_secs(5)).await;
                attempt += 1;
                continue;
            },
            status => {
                panic!("Failed to fetch metadata! Failed with status: {}", status);
            }
        }
    }

    panic!("Failed to fetch a page of metadata after {} retries.", RETRY_ATTEMPTS);
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
    let mut credentials = credentials;

    loop {
        if credentials.is_token_expiry_imminent() {
            println!("Token expiry is imminent (less than five minutes away); attempting to refresh token.");
            credentials = credentials.refresh().await?;
        }

        match fetch_page(&credentials, &pageToken).await? {
            FetchResult::MorePagesExist(next) => pageToken = next,
            FetchResult::NoMorePagesExist => {
                println!("All done!");
                return Ok(());
            }
        }

        metrics::flush();
    }
}
