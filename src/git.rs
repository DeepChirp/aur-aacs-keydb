use anyhow::Result;
use git2::{build::RepoBuilder, Cred, FetchOptions, RemoteCallbacks, Repository};
use std::path::Path;
use tracing::info;

pub struct GitHelper {
    ssh_key_path: String,
}

impl GitHelper {
    pub fn new(ssh_key_path: String) -> Self {
        Self { ssh_key_path }
    }

    pub fn prepare_aur_repo(&self, path: &Path, package_name: &str) -> Result<Repository> {
        let mut cb = RemoteCallbacks::new();
        let ssh_key_path = self.ssh_key_path.clone();

        cb.credentials(move |_, user, _| {
            Cred::ssh_key(user.unwrap(), None, Path::new(&ssh_key_path), None)
        });

        let mut fo = FetchOptions::new();
        fo.remote_callbacks(cb);

        let repo_url = format!("ssh://aur@aur.archlinux.org/{package_name}.git");

        if path.exists() {
            info!("Repository exists, updating...");
            let repo = Repository::open(path)?;
            {
                let mut origin = repo.find_remote("origin")?;
                origin.fetch(&["refs/heads/*:refs/heads/*"], Some(&mut fo), None)?;
            }
            {
                let fetch_head = repo.find_reference("FETCH_HEAD")?;
                let fetch_commit = repo.reference_to_annotated_commit(&fetch_head)?;
                let analysis = repo.merge_analysis(&[&fetch_commit])?;
                if analysis.0.is_fast_forward() {
                    let mut master = repo.find_reference("refs/heads/master")?;
                    master.set_target(fetch_commit.id(), "fast-forward")?;
                    repo.set_head("refs/heads/master")?;
                    repo.checkout_head(None)?;
                    info!("Repository updated successfully");
                } else {
                    info!("Repository is up to date");
                }
            }
            Ok(repo)
        } else {
            info!("Repository does not exist, cloning...");
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent)?;
            }

            let repo = RepoBuilder::new()
                .fetch_options(fo)
                .clone(&repo_url, path)?;

            info!("Repository cloned successfully");
            Ok(repo)
        }
    }

    pub fn commit_and_push(&self, repo: &Repository, message: &str) -> Result<()> {
        let mut index = repo.index()?;
        index.add_all(["*"].iter(), git2::IndexAddOption::DEFAULT, None)?;
        index.write()?;

        let tree_id = index.write_tree()?;
        let tree = repo.find_tree(tree_id)?;
        let signature = repo.signature()?;
        let head = repo.head()?.peel_to_commit()?;

        repo.commit(
            Some("HEAD"),
            &signature,
            &signature,
            message,
            &tree,
            &[&head],
        )?;

        let mut cb = RemoteCallbacks::new();
        let ssh_key_path = self.ssh_key_path.clone();

        cb.credentials(move |_, user, _| {
            Cred::ssh_key(user.unwrap(), None, Path::new(&ssh_key_path), None)
        });

        let mut push_options = git2::PushOptions::new();
        push_options.remote_callbacks(cb);

        let mut origin = repo.find_remote("origin")?;
        origin.push(&["refs/heads/master:refs/heads/master"], Some(&mut push_options))?;

        Ok(())
    }
}
