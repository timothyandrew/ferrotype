//! Download and persist media items.

use reqwest::{Client, Url};

use futures::future::join_all;
use reqwest::StatusCode;
use std::path::{Path, PathBuf};
use tokio::fs::{create_dir_all, metadata, File};
use tokio::io::AsyncWriteExt;

use crate::metadata::MediaItem;
use crate::statistics::persist_statistics;

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
    filename: &PathBuf,
) -> Result<StatusCode, Box<dyn std::error::Error>> {
    let response = client.get(url.to_owned()).send().await?;
    let status = response.status();

    // TODO: Abstract away this duplication
    match status {
        StatusCode::OK => {
            // TODO: Use `bytes_stream` instead
            let response = response.bytes().await?;
            let response: &[u8] = response.as_ref();

            let mut file = File::create(filename).await?;
            file.write_all(response).await?;
        }
        StatusCode::UNAUTHORIZED => panic!("Authorization failed!"),
        StatusCode::TOO_MANY_REQUESTS => panic!("We've hit the rate limit!"),
        _ => {
            // TODO: Implement retry logic:
            // - https://developers.google.com/photos/library/guides/api-limits-quotas
            // - https://developers.google.com/photos/library/guides/best-practices

            // Do nothing for now, assuming that this will resolve itself in a future run.
        }
    }

    Ok(status)
}

// Download the file(s) underlying a given MediaItem.
// For videos and regular photos, a media item translates to a single file,
// but this is untrue for motion photos, which are backed by two files.
async fn download_item(
    item: &MediaItem,
    client: &Client,
    prefix: &Path,
) -> Result<usize, Box<dyn std::error::Error>> {
    // `unwrap` here is _probably_ valid because not receving a valid URL here
    // could be considered a stop-the-world error, but is there a better way to
    // handle this?
    let urls = item.download_urls();
    let mut download_count = 0;

    for url in &urls {
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
            download_count += 1;
        }
    }

    Ok(download_count)
}

/// Download (possibly in parallel) all media items passed in, and persist to the `prefix` directory.
/// Returns the number of downloads attempted.
pub async fn download_media_items(
    items: &[MediaItem],
    prefix: &str,
) -> Result<usize, Box<dyn std::error::Error>> {
    let client = Client::new();
    let prefix = Path::new(prefix);

    create_dirs(items, prefix).await?;

    let futures: Vec<_> = items
        .iter()
        .map(|item| download_item(item, &client, prefix))
        .collect();

    let results = join_all(futures).await;
    let count: usize = results.iter().flatten().sum();

    Ok(count)
}
