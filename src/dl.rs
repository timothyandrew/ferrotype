//! Download and persist media items.

use reqwest::{Client, StatusCode, Url};
use std::future::Future;
use futures::future::join_all;
use tokio::fs::{File,create_dir_all};
use tokio::io::AsyncWriteExt;
use std::path::{Path,PathBuf};
use chrono::{Datelike,DateTime};
use crate::metadata::MediaItem;

fn item_path(item: &MediaItem, prefix: &Path) -> PathBuf {
    let year = item.created_at().format("%Y").to_string();
    let date = item.created_at().format("%Y-%m-%d").to_string();
    prefix.join(year).join(date)
}

async fn create_dirs(items: &[MediaItem], prefix: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let futures: Vec<_> = items.iter().map(|item| {
        create_dir_all(item_path(item, prefix))
    }).collect();

    join_all(futures).await;
    Ok(())
}

async fn download_item(item: &MediaItem, client: &Client, prefix: &Path) -> Result<(), Box<dyn std::error::Error>> {
    // TODO: This is photo-specific; handle videos!
    let url = format!("{}=d", item.url());
    let url = Url::parse(url.as_ref()).unwrap();
    let response = client.get(url.to_owned()).send().await?;

    // TODO: Use `bytes_stream` instead
    let response = response.bytes().await?;
    let response: &[u8] = response.as_ref();

    let mut file = File::create(item_path(item, prefix).join(item.filename())).await?;
    file.write_all(response).await?;

    Ok(())
}

/// Download (possibly in parallel) all media items passed in, and persist to the `prefix` directory.
pub async fn download_media_items(items: &[MediaItem], prefix: &str) -> Result<(), Box<dyn std::error::Error>> {

    let client = Client::new();
    let prefix = Path::new(prefix);

    create_dirs(items, prefix).await?;

    let futures: Vec<_> = items.iter().map(|item| {
      download_item(item, &client, prefix)
    }).collect();

    join_all(futures).await;

    Ok(())


}