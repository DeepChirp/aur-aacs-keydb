use anyhow::{Result, anyhow};
use regex::Regex;
use std::{fs, path::Path};

pub struct AurPackageManager {
    package_name: String,
    original_url: String,
}

impl AurPackageManager {
    pub fn new(package_name: String, original_url: String) -> Self {
        Self {
            package_name,
            original_url,
        }
    }

    pub fn extract_current_version(&self, pkgbuild_path: &Path) -> Result<String> {
        let content = fs::read_to_string(pkgbuild_path)?;

        let version_regex = Regex::new(r"pkgver=([^\s]+)")?;

        if let Some(captures) = version_regex.captures(&content) {
            if let Some(version) = captures.get(1) {
                return Ok(version.as_str().to_string());
            }
        }

        Err(anyhow!("Could not find pkgver in PKGBUILD"))
    }

    pub fn extract_current_sha256(&self, pkgbuild_path: &Path) -> Result<String> {
        let content = fs::read_to_string(pkgbuild_path)?;

        let sha256_regex = Regex::new(r"sha256sums=\('([^']+)'\)")?;

        if let Some(captures) = sha256_regex.captures(&content) {
            if let Some(sha256) = captures.get(1) {
                return Ok(sha256.as_str().to_string());
            }
        }

        Err(anyhow!("Could not find sha256sums in PKGBUILD"))
    }

    pub fn update_pkgbuild(
        &self,
        pkgbuild_path: &Path,
        new_version: &str,
        new_sha256: &str,
    ) -> Result<()> {
        let mut content = fs::read_to_string(pkgbuild_path)?;

        let version_regex = Regex::new(r"pkgver=([^\s]+)")?;
        content = version_regex
            .replace(&content, format!("pkgver={new_version}"))
            .to_string();

        let sha256_regex = Regex::new(r"sha256sums=\('([^']+)'\)")?;
        content = sha256_regex
            .replace(&content, format!("sha256sums=('{new_sha256}')"))
            .to_string();

        let pkgrel_regex = Regex::new(r"pkgrel=([^\s]+)")?;
        content = pkgrel_regex.replace(&content, "pkgrel=1").to_string();

        fs::write(pkgbuild_path, content)?;
        Ok(())
    }

    /// Generate .SRCINFO file
    pub fn generate_srcinfo(
        &self,
        _pkgbuild_path: &Path,
        version: &str,
        sha256: &str,
        url: &str,
    ) -> Result<String> {
        let source_line = format!("keydb_eng-{version}.zip::{url}");
        let srcinfo = format!(
            "pkgbase = {}\n\tpkgdesc = Contains the Key Database for the AACS Library (Daily Updates)\n\tpkgver = {}\n\tpkgrel = 1\n\turl = http://fvonline-db.bplaced.net/\n\tarch = any\n\tdepends = libaacs\n\tsource = {}\n\tsha256sums = {}\n\npkgname = {}\n",
            self.package_name, version, source_line, sha256, self.package_name
        );

        Ok(srcinfo)
    }

    /// Create initial PKGBUILD file (if it doesn't exist)
    pub fn create_initial_pkgbuild(
        &self,
        pkgbuild_path: &Path,
        version: &str,
        sha256: &str,
    ) -> Result<()> {
        let pkgbuild_content = format!(
            "# Maintainer: DeepChirp <DeepChirp@outlook.com>\npkgname={}\npkgver={}\npkgrel=1\npkgdesc='Contains the Key Database for the AACS Library (Daily Updates)'\narch=('any')\nurl='http://fvonline-db.bplaced.net/'\ndepends=('libaacs')\nsource=(\"keydb_eng-${{pkgver}}.zip::https://web.archive.org/web/${{pkgver}}/{}\")\nsha256sums=('{}')\n\npackage() {{\n    install -d \"${{pkgdir}}/etc/xdg/aacs\" || return 1\n    install -Dm644 \"${{srcdir}}/keydb.cfg\" \"${{pkgdir}}/etc/xdg/aacs/KEYDB.cfg\" || return 1\n}}\n",
            self.package_name, version, self.original_url, sha256
        );

        fs::write(pkgbuild_path, pkgbuild_content)?;
        Ok(())
    }
}
