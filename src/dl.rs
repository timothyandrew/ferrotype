//! Download and persist media items.

use reqwest::{Client, StatusCode, Url};
use std::future::Future;
use futures::future::join_all;
use tokio::fs::File;
use tokio::io::AsyncWriteExt;

async fn download_url(url: &Url, client: &Client) -> Result<(), Box<dyn std::error::Error>> {
    let response = client.get(url.to_owned()).send().await?;

    // TODO: Use `bytes_stream` instead
    let response = response.bytes().await?;
    let response: &[u8] = response.as_ref();

    let mut file = File::create("/tmp/foo.txt").await?;
    file.write_all(response).await?;

    Ok(())
}

/// Download (possibly in parallel) all URLs passed in, and persist to the `prefix` directory.
pub async fn download_urls(urls: &[Url], prefix: &str) -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::new();

    let futures: Vec<_> = urls.iter().map(|url| {
      download_url(url, &client)
    }).collect();

    join_all(futures).await;

    Ok(())


}