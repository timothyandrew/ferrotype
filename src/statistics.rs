//! A download run can cache data to optimize subsequent runs; this module
//! handles persisting and loading this cached data.

use reqwest::StatusCode;
use reqwest::Url;
use std::collections::HashSet;
use std::error::Error;
use std::io::Read;
use tokio::fs::{File, OpenOptions};
use tokio::io::AsyncWriteExt;

use crate::metadata::{MediaItem, MediaItemType};
use crate::metrics;

const NON_MOTION_PHOTOS_FILE: &str = "non-motion-photos";

/// Immutable singleton, meant to provide data from past runs to subsequent runs
#[derive(Debug)]
pub struct Statistics {
    non_motion_photos: HashSet<String>,
}

lazy_static! {
    static ref STATISTICS: Statistics = {
        println!("Loading statistics from disk…");
        load_statistics().unwrap()
    };
}

impl Statistics {
    pub fn non_motion_photos(&self) -> &HashSet<String> {
        &self.non_motion_photos
    }
}

pub fn get() -> &'static Statistics {
    &STATISTICS
}

async fn persist(item: &MediaItem, filename: &str) -> Result<(), Box<dyn Error>> {
    let mut file: File = OpenOptions::new()
        .append(true)
        .create(true)
        .open(filename)
        .await?;
    let line = format!("{}\n", item.id());
    file.write(line.as_bytes()).await?;
    Ok(())
}

/// Persist statistics about the current run to improve runtime for future runs.
pub async fn persist_statistics(
    item: &MediaItem,
    url: &Url,
    code: StatusCode,
) -> Result<(), Box<dyn Error>> {
    // We assume that _every_ photo is a motion photo until we can prove otherwise, which
    // happens when we attempt to download it as a video and are given a 200/404 in return.
    // Persist a list of all non-motion photo IDs to disk, so we can skip the `dv` portions
    // for them on future runs.
    if let (StatusCode::NOT_FOUND, MediaItemType::photo(_)) = (code, item.media_type()) {
        if url.to_string().contains("=dv") {
            metrics::tick("non_motion_photo_404");
            persist(item, NON_MOTION_PHOTOS_FILE).await?;
        }
    }

    Ok(())
}

/// In-memory representation of statistics in previous runs.
pub fn load_statistics() -> Result<Statistics, Box<dyn Error>> {
    let mut non_motion_photos = HashSet::<String>::new();

    let mut f = std::fs::File::open(NON_MOTION_PHOTOS_FILE)?;
    let mut buffer = String::new();
    f.read_to_string(&mut buffer)?;

    for line in buffer.split("\n") {
        if line.is_empty() {
            continue;
        }

        // TODO: Is this terribly non-performant? Ignoring for now because this a fixed startup cost at the moment.
        non_motion_photos.insert(line.to_owned());
    }

    Ok(Statistics { non_motion_photos })
}
