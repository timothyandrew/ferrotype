//! Download and persist media items.

use reqwest::{Client, Url};

use futures::future::join_all;
use reqwest::StatusCode;
use std::time::Duration;
use std::path::{Path, PathBuf};
use tokio::fs::{create_dir_all, metadata, File};
use tokio::time;
use tokio::io::AsyncWriteExt;

use crate::metadata::MediaItem;
use crate::metrics;
use crate::statistics::persist_statistics;

const RETRY_ATTEMPTS: usize = 3;

fn item_path(item: &MediaItem, prefix: &Path) -> PathBuf {
    let year = item.created_at().format("%Y").to_string();
    let date = item.created_at().format("%Y-%m-%d").to_string();
    prefix.join(year).join(date)
}

async fn create_dirs(items: &[MediaItem], prefix: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let futures: Vec<_> = items
        .iter()
        .map(|item| create_dir_all(item_path(item, prefix)))
        .collect();

    join_all(futures).await;
    Ok(())
}

async fn download_file(
    url: &Url,
    client: &Client,
    filename: &PathBuf
) -> Result<StatusCode, Box<dyn std::error::Error>> {
    let mut attempt = 0;

    while attempt < RETRY_ATTEMPTS {
        let response = client.get(url.to_owned()).send().await?;
        let status = response.status();

        // TODO: Abstract away this duplication
        match status.as_u16() {
            200 => {
                metrics::tick("dl_200");

                // TODO: Use `bytes_stream` instead
                let response = response.bytes().await?;
                let response: &[u8] = response.as_ref();

                let mut file = File::create(filename).await?;
                file.write_all(response).await?;
            }
            401 => panic!("Authorization failed!"),
            429 => panic!("We've hit the rate limit, try again later! (rate limits reset at midnight PT)"),
            404 => {
                // Do nothing; we incorrectly assumed that this URL represents a motion photo
            },
            500..=599 => {
                // TODO: Exponential backoff
                metrics::tick("retry_dl_after_5xx");
                time::delay_for(Duration::from_secs(1)).await;
                attempt += 1;
                continue;
            },
            status => {
                panic!("Failed to download file! Failed with status: {}", status);
            }
        }

        return Ok(status)
    }

    // What do we do when we exhaust retries for a download; we don't want to panic and 
    // affect the rest of this run; it's better to just do nothing and assume that
    // Google will fix this before a future run..
    metrics::tick("dl_failed_retries_exhausted");
    Ok(StatusCode::from_u16(500)?)
}

// Download the file(s) underlying a given MediaItem.
// For videos and regular photos, a media item translates to a single file,
// but this is untrue for motion photos, which are backed by two files.
async fn download_item(
    item: &MediaItem,
    client: &Client,
    prefix: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let urls = item.download_urls();

    for url in &urls {
        // `unwrap` here is _probably_ valid because not receving a valid URL here
        // could be considered a stop-the-world error, but is there a better way to
        // handle this?
        let url = Url::parse(&url).unwrap();
        let filename = if url.to_string().contains("=dv") {
            format!("{}.mp4", item.id())
        } else {
            format!("{}.jpg", item.id())
        };
        let path = item_path(item, prefix).join(filename);

        // Do nothing if file already exists
        // This assumes that a prior download of this file did _not_ result in corruption.
        if metadata(&path).await.is_err() {
            let status: StatusCode = download_file(&url, client, &path).await?;
            persist_statistics(item, &url, status).await?;
        } else {
            metrics::tick("file_already_exists");
        }
    }

    Ok(())
}

/// Download (possibly in parallel) all media items passed in, and persist to the `prefix` directory.
/// Returns the number of downloads attempted.
pub async fn download_media_items(
    items: &[MediaItem],
    prefix: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::new();
    let prefix = Path::new(prefix);

    create_dirs(items, prefix).await?;

    let futures: Vec<_> = items
        .iter()
        .map(|item| download_item(item, &client, prefix))
        .collect();

    join_all(futures).await;

    Ok(())
}
