use crate::error::{AppError, Result};
use shellexpand::tilde;
use std::path::Path;

#[derive(Debug, Clone)]
pub struct Config {
    pub package_name: String,
    pub original_url: String,
    pub work_dir: String,
    pub ssh_key_path: String,
}

impl Config {
    pub fn new() -> Self {
        let ssh_key_path =
            tilde(&std::env::var("SSH_KEY_PATH").unwrap_or_else(|_| "~/.ssh/id_ed25519".into()))
                .into_owned();

        Self {
            package_name: "aacs-keydb-daily".to_string(),
            original_url: "http://fvonline-db.bplaced.net/export/keydb_eng.zip".to_string(),
            work_dir: "/tmp/aur-aacs-keydb-daily".to_string(),
            ssh_key_path,
        }
    }

    pub fn validate(&self) -> Result<()> {
        if !Path::new(&self.ssh_key_path).exists() {
            return Err(AppError::SshAuthFailed);
        }

        if !self.original_url.starts_with("http://") && !self.original_url.starts_with("https://") {
            return Err(AppError::Archive(anyhow::anyhow!("Invalid URL format")));
        }

        if self.package_name.is_empty() {
            return Err(AppError::Archive(anyhow::anyhow!(
                "Package name cannot be empty"
            )));
        }

        Ok(())
    }
}
