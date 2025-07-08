use crate::{
    archive::{ArchiveResult, WebArchiveClient},
    aur::AurPackageManager,
    config::Config,
    error::{AppError, Result},
    git::GitHelper,
};
use std::{fs, path::PathBuf};
use tracing::{error, info, warn};

pub struct App {
    config: Config,
    archive_client: WebArchiveClient,
    git_helper: GitHelper,
    aur_manager: AurPackageManager,
}

impl App {
    pub fn new(config: Config) -> Result<Self> {
        config.validate()?;

        let git_helper = GitHelper::new(config.ssh_key_path.clone());
        let archive_client = WebArchiveClient::new();
        let aur_manager = AurPackageManager::new(config.package_name.clone(), config.original_url.clone());

        Ok(Self {
            config,
            archive_client,
            git_helper,
            aur_manager,
        })
    }

    pub async fn run(&self) -> Result<()> {
        info!("Starting AACS KeyDB Daily Update Process");
        info!("Package: {}", self.config.package_name);
        info!("Original URL: {}", self.config.original_url);

        let archive_result = self.create_archive().await?;
        let repo = self.prepare_repository().await?;

        if !self.needs_update(&archive_result).await? {
            info!("Nothing to do, package is up to date!");
            return Ok(());
        }

        self.update_package(&archive_result).await?;
        self.commit_and_push(&repo, &archive_result.version).await?;

        info!(
            "Successfully updated and pushed {} version {}",
            self.config.package_name, archive_result.version
        );
        info!("Process completed!");

        Ok(())
    }

    async fn create_archive(&self) -> Result<ArchiveResult> {
        info!("Step 1: Creating new archive on web.archive.org and downloading...");

        let archive_result = self
            .archive_client
            .archive_and_download(&self.config.original_url)
            .await
            .map_err(|e| {
                error!("Unable to access web.archive.org: {e}");
                AppError::Archive(e)
            })?;

        info!("Archive URL: {}", archive_result.archive_url);
        info!("SHA256: {}", archive_result.sha256);

        Ok(archive_result)
    }

    async fn prepare_repository(&self) -> Result<git2::Repository> {
        info!("Step 2: Preparing AUR repository...");
        let work_path = PathBuf::from(&self.config.work_dir);

        info!("Cloning/updating AUR repository...");
        let repo = self
            .git_helper
            .prepare_aur_repo(&work_path, &self.config.package_name)
            .map_err(AppError::Archive)?;

        Ok(repo)
    }

    async fn needs_update(&self, archive_result: &ArchiveResult) -> Result<bool> {
        let pkgbuild_path = PathBuf::from(&self.config.work_dir).join("PKGBUILD");

        if !pkgbuild_path.exists() {
            info!("Step 3: Creating new package (PKGBUILD not found)...");
            return Ok(true);
        }

        info!("Step 3: Checking if update is needed...");

        let current_version = self
            .aur_manager
            .extract_current_version(&pkgbuild_path)
            .map_err(|_| {
                warn!("Could not extract current version, assuming update needed");
                AppError::VersionNotFound
            })?;

        info!("Current version: {current_version}");
        info!("Archive version: {}", archive_result.version);

        if archive_result.version <= current_version {
            info!("Current version is not older than archive, no update needed");
            return Ok(false);
        }

        match self.aur_manager.extract_current_sha256(&pkgbuild_path) {
            Ok(current_sha256) => {
                if current_sha256 == archive_result.sha256 {
                    info!("Package is already up to date (SHA256 match)");
                    Ok(false)
                } else {
                    info!("Update needed");
                    info!("   Current: {current_sha256}");
                    info!("   New:     {}", archive_result.sha256);
                    Ok(true)
                }
            }
            Err(_) => {
                warn!("Could not extract current SHA256, assuming update needed");
                Ok(true)
            }
        }
    }

    async fn update_package(&self, archive_result: &ArchiveResult) -> Result<()> {
        info!("Step 4: Updating package...");
        info!("New version: {}", archive_result.version);

        let work_path = PathBuf::from(&self.config.work_dir);
        let pkgbuild_path = work_path.join("PKGBUILD");

        if pkgbuild_path.exists() {
            self.aur_manager.update_pkgbuild(
                &pkgbuild_path,
                &archive_result.version,
                &archive_result.sha256,
            )?;
        } else {
            self.aur_manager.create_initial_pkgbuild(
                &pkgbuild_path,
                &archive_result.version,
                &archive_result.sha256,
            )?;
        }

        info!("Generating .SRCINFO...");
        let srcinfo_content = self.aur_manager.generate_srcinfo(
            &pkgbuild_path,
            &archive_result.version,
            &archive_result.sha256,
            &archive_result.archive_url,
        )?;

        let srcinfo_path = work_path.join(".SRCINFO");
        fs::write(&srcinfo_path, srcinfo_content)?;

        Ok(())
    }

    async fn commit_and_push(&self, repo: &git2::Repository, version: &str) -> Result<()> {
        info!("Step 5: Committing and pushing changes...");
        let commit_message = format!("Update to {version}");

        let work_path = PathBuf::from(&self.config.work_dir);
        info!("Files updated:");
        info!("   - {}", work_path.join("PKGBUILD").display());
        info!("   - {}", work_path.join(".SRCINFO").display());

        info!("Commit message: {commit_message}");
        info!("Committing and pushing to AUR...");

        self.git_helper
            .commit_and_push(repo, &commit_message)
            .map_err(AppError::Archive)?;

        Ok(())
    }
}
