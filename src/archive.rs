use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use tracing::info;

#[derive(Debug, Deserialize)]
pub struct ArchiveResponse {
    pub archived_snapshots: HashMap<String, ArchiveSnapshot>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ArchiveSnapshot {
    pub available: bool,
    pub url: String,
    pub timestamp: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ArchiveResult {
    pub original_url: String,
    pub archive_url: String,
    pub timestamp: DateTime<Utc>,
    pub sha256: String,
    pub version: String,
}

pub struct WebArchiveClient {
    client: reqwest::Client,
}

impl WebArchiveClient {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
        }
    }

    pub async fn check_archived(&self, url: &str) -> Result<Option<ArchiveSnapshot>> {
        let api_url = format!("https://archive.org/wayback/available?url={url}");
        info!("Checking existing archives at: {api_url}");

        let response: ArchiveResponse = self.client.get(&api_url).send().await?.json().await?;

        info!("Archive response: {response:?}");
        Ok(response.archived_snapshots.get("closest").cloned())
    }

    pub async fn get_latest_archive(&self, url: &str) -> Result<Option<ArchiveSnapshot>> {
        let archive_browse_url = format!("https://web.archive.org/web/{url}");
        info!("Accessing archive page: {archive_browse_url}");

        let response = self.client.head(&archive_browse_url).send().await?;

        let final_url = response.url().to_string();
        info!("Final URL after redirect: {final_url}");

        if let Some(start) = final_url.find("/web/") {
            let after_web = &final_url[start + 5..];
            if let Some(end) = after_web.find('/') {
                let timestamp = &after_web[..end];
                if timestamp.len() >= 8 && timestamp.chars().all(|c| c.is_ascii_digit()) {
                    info!("Found archive timestamp: {timestamp}");
                    return Ok(Some(ArchiveSnapshot {
                        available: true,
                        url: final_url.clone(),
                        timestamp: timestamp.to_string(),
                    }));
                }
            }
        }

        info!("No valid archive found");
        Ok(None)
    }

    pub async fn archive_url(&self, url: &str) -> Result<String> {
        let save_url = format!("https://web.archive.org/save/{url}");

        info!("Submitting archive request to: {save_url}");

        let response = self.client.get(&save_url).send().await?;

        info!("Archive request status: {}", response.status());

        if response.status().is_success() {
            info!("Archive request submitted successfully, waiting for completion...");
            tokio::time::sleep(tokio::time::Duration::from_secs(15)).await;

            for attempt in 1..=5 {
                info!("Attempt {attempt} to get new archive...");
                match self.check_archived(url).await {
                    Ok(Some(snapshot)) => {
                        if snapshot.available {
                            info!("Found new archive: {}", snapshot.url);
                            return Ok(snapshot.url);
                        }
                    }
                    Ok(None) => {
                        info!("No archive found yet");
                    }
                    Err(e) => {
                        info!("Error checking archive: {e}");
                    }
                }
                if attempt < 5 {
                    tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
                }
            }
        } else if response.status().as_u16() == 429 {
            info!("Rate limited (429). Will fallback to existing archive...");
            anyhow::bail!("Rate limited - will use existing archive");
        }

        anyhow::bail!("Failed to archive URL: {}", url)
    }

    /// Download file from archive URL and calculate SHA256
    pub async fn download_and_hash(&self, url: &str) -> Result<(Vec<u8>, String)> {
        let response = self.client.get(url).send().await?.error_for_status()?;

        let bytes = response.bytes().await?;
        let mut hasher = Sha256::new();
        hasher.update(&bytes);
        let hash = hasher.finalize();
        let hash_string = format!("{hash:x}");

        Ok((bytes.to_vec(), hash_string))
    }

    /// Complete archive and download process - try to create new archive, fallback to existing one
    pub async fn archive_and_download(&self, url: &str) -> Result<ArchiveResult> {
        info!("Creating new archive for {url}...");

        // Try to create new archive
        match self.archive_url(url).await {
            Ok(archive_url) => {
                info!("Downloading from new archive: {archive_url}");
                let (_, sha256) = self.download_and_hash(&archive_url).await?;

                // Extract timestamp from archive URL as version number
                let version = self.extract_version_from_archive_url(&archive_url);

                return Ok(ArchiveResult {
                    original_url: url.to_string(),
                    archive_url,
                    timestamp: Utc::now(),
                    sha256,
                    version,
                });
            }
            Err(e) => {
                info!("Failed to create new archive: {e}");
                info!("Falling back to existing archive...");
            }
        }

        // If creating new archive fails, directly access archive page to get latest version
        if let Ok(Some(snapshot)) = self.get_latest_archive(url).await {
            if snapshot.available {
                info!("Using existing archive: {}", snapshot.url);
                let (_, sha256) = self.download_and_hash(&snapshot.url).await?;

                // Extract version number from archive timestamp
                let version = snapshot.timestamp.clone();

                return Ok(ArchiveResult {
                    original_url: url.to_string(),
                    archive_url: snapshot.url,
                    timestamp: Utc::now(),
                    sha256,
                    version,
                });
            }
        }

        anyhow::bail!("No archive available for URL: {url}")
    }

    /// Extract version number from archive URL (timestamp)
    fn extract_version_from_archive_url(&self, archive_url: &str) -> String {
        // Extract timestamp from URL like https://web.archive.org/web/20231201000000/...
        if let Some(start) = archive_url.find("/web/") {
            let after_web = &archive_url[start + 5..];
            if let Some(end) = after_web.find('/') {
                let timestamp = &after_web[..end];
                return timestamp.to_string();
            }
        }
        // If unable to extract, use current timestamp
        chrono::Utc::now().format("%Y%m%d%H%M%S").to_string()
    }
}
