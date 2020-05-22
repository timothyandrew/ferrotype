//! Download, examine, and store _metadata_ about the media we're attempting to fetch.

#![allow(non_snake_case)]
#![allow(non_camel_case_types)]

use crate::auth::Credentials;
use chrono::{DateTime, Utc};
use reqwest::{Client, StatusCode, Url};
use serde::Deserialize;

use crate::dl;

const LIST_URL: &str = "https://photoslibrary.googleapis.com/v1/mediaItems";
const PAGE_SIZE: usize = 100;
const METADATA_QUOTA: usize = 9000;
const DOWNLOAD_QUOTA: usize = 72000;

#[derive(Debug)]
struct UsageAgainstQuota {
    metadata: usize,
    download: usize,
}

#[derive(Deserialize, Debug)]
struct Photo {}

#[derive(Deserialize, Debug)]
#[warn(non_camel_case_types)]
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
    NoMorePagesExist,
}

fn is_over_quota(usage: &UsageAgainstQuota) -> bool {
    println!("Current usage against quota: {:?}", usage);
    (usage.download + PAGE_SIZE) >= DOWNLOAD_QUOTA || (usage.metadata + 1) >= METADATA_QUOTA
}

async fn fetch_page(
    credentials: &Credentials,
    pageToken: &str,
    usage: &mut UsageAgainstQuota,
) -> Result<FetchResult, Box<dyn std::error::Error>> {
    let client = Client::new();
    let params = vec![
        ("pageSize", PAGE_SIZE.to_string()),
        ("pageToken", pageToken.to_string()),
    ];
    let url = Url::parse_with_params(LIST_URL, &params)?;

    let request = client
        .get(url)
        .header("Authorization", format!("Bearer {}", credentials.get_key()));
    let response = request.send().await?;

    match response.status() {
        StatusCode::OK => {
            let response: MetadataResponse = response.json::<MetadataResponse>().await?;

            // TODO: Only download missing media items
            // TODO: Parameter for dl location
            dl::download_media_items(&response.mediaItems, "/tmp/foo").await?;

            usage.metadata += 1;
            usage.download += response.mediaItems.len();

            let result = if response.nextPageToken.is_empty() {
                FetchResult::NoMorePagesExist
            } else {
                FetchResult::MorePagesExist(response.nextPageToken)
            };

            Ok(result)
        }
        StatusCode::UNAUTHORIZED => {
            // We shouldn't ever hit this line if the refresh token flow
            // is working as expected.
            panic!("Authorization failed!")
        }
        StatusCode::TOO_MANY_REQUESTS => panic!("We've hit the rate limit!"),
        status => {
            // TODO: Implement retry logic:
            // - https://developers.google.com/photos/library/guides/api-limits-quotas
            // - https://developers.google.com/photos/library/guides/best-practices
            panic!("Failed to fetch metadata! Failed with status: {}", status);
        }
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
    let mut credentials = credentials;

    // TODO: replace with a ::new function
    let mut usage = UsageAgainstQuota {
        download: 0,
        metadata: 0,
    };

    loop {
        if is_over_quota(&usage) {
            println!("Exiting early; over quota! Try again after midnight Pacific Time.");
            return Ok(());
        }

        if credentials.is_token_expiry_imminent() {
            println!("Token expiry is imminent (less than five minutes away); attempting to refresh token.");
            credentials = credentials.refresh().await?;
        }

        println!("Downloading page #{}", counter);
        counter += 1;

        match fetch_page(&credentials, &pageToken, &mut usage).await? {
            FetchResult::MorePagesExist(next) => pageToken = next,
            FetchResult::NoMorePagesExist => {
                println!("All done!");
                return Ok(());
            }
        }
    }
}
