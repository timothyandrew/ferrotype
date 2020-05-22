//! Download and persist media items.

use reqwest::{Client, StatusCode, Url};
use std::future::Future;
use futures::future::join_all;
use tokio::fs::File;
use tokio::io::AsyncWriteExt;
use std::path::Path;
use crate::metadata::MediaItem;

async fn download_item(item: &MediaItem, client: &Client, prefix: &str) -> Result<(), Box<dyn std::error::Error>> {
    let basePath = Path::new(prefix);

    let url = format!("{}=d", item.url());
    let url = Url::parse(url.as_ref()).unwrap();
    let response = client.get(url.to_owned()).send().await?;

    // TODO: Use `bytes_stream` instead
    let response = response.bytes().await?;
    let response: &[u8] = response.as_ref();

    let mut file = File::create(basePath.join(item.filename())).await?;
    file.write_all(response).await?;

    Ok(())
}

/// Download (possibly in parallel) all media items passed in, and persist to the `prefix` directory.
pub async fn download_media_items(items: &[MediaItem], prefix: &str) -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::new();

    let futures: Vec<_> = items.iter().map(|item| {
      download_item(item, &client, prefix)
    }).collect();

    join_all(futures).await;

    Ok(())


}