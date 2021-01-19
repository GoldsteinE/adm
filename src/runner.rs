use std::{
    path::{Path, PathBuf},
    process::Command,
    sync::Arc,
};

use tokio::sync::mpsc;
use tracing::Instrument as _;

use crate::lock_manager::LockManager;

#[derive(Debug, Clone)]
pub struct Runner {
    base_path: PathBuf,
    lock_manager: Arc<LockManager<(String, String)>>,
}

#[derive(Debug, Clone)]
pub struct Task {
    pub url: String,
    pub owner: String,
    pub repo: String,
    pub branch: String,
    pub commit_hash: String,
}

fn open_or_clone(url: &str, path: &Path) -> Result<git2::Repository, (git2::Error, git2::Error)> {
    match git2::Repository::open(&path) {
        Ok(repo) => {
            tracing::info!(
                path = path.to_string_lossy().as_ref(),
                "Opened repo at {:?}",
                path
            );
            Ok(repo)
        }
        Err(open_err) => match git2::Repository::clone(url, &path) {
            Ok(repo) => {
                tracing::info!(
                    url = url,
                    path = path.to_string_lossy().as_ref(),
                    "Cloned repo {} to {:?}",
                    url,
                    path
                );
                Ok(repo)
            }
            Err(clone_err) => {
                tracing::error!(
                    url = url,
                    path = path.to_string_lossy().as_ref(),
                    concat!(
                        "Failed to either open or clone repository. ",
                        "Open error: {}. Clone error: {}",
                    ),
                    open_err,
                    clone_err,
                );
                Err((open_err, clone_err))
            }
        },
    }
}

fn pull_repo(repo: &mut git2::Repository) -> Result<(), git2::Error> {
    let mut origin = match repo.find_remote("origin") {
        Ok(remote) => remote,
        Err(err) => {
            tracing::error!("Failed to find remote `origin`: {}", err);
            return Err(err);
        }
    };

    if let Err(err) = origin.fetch(&["master"], None, None) {
        tracing::error!("Failed to fetch origin/master: {}", err);
        return Err(err);
    }

    Ok(())
}

fn checkout(repo: &mut git2::Repository, commit_id: &str) -> Result<(), git2::Error> {
    let oid: git2::Oid = match commit_id.parse() {
        Ok(oid) => oid,
        Err(err) => {
            tracing::error!("Invalid commit ID `{}`: {}", commit_id, err);
            return Err(err);
        }
    };
    let commit = match repo.find_commit(oid) {
        Ok(commit) => commit,
        Err(err) => {
            tracing::error!("Failed to find commit `{}`: {}", oid, err);
            return Err(err);
        }
    };
    if let Err(err) = repo.reset(
        commit.as_object(),
        git2::ResetType::Hard,
        Some(
            git2::build::CheckoutBuilder::new()
                .force()
                .remove_untracked(true),
        ),
    ) {
        tracing::error!("Failed to reset repo: {}", err);
        return Err(err);
    }
    Ok(())
}

impl Runner {
    pub fn new(base_path: PathBuf, lock_manager: Arc<LockManager<(String, String)>>) -> Self {
        Self {
            base_path,
            lock_manager,
        }
    }

    async fn process_task(&self, task: Task) {
        let path = {
            let mut p = self.base_path.join(&task.owner);
            p.push(&task.repo);
            p.push(&task.branch);
            p
        };
        tracing::info!(
            "Running build for {}/{} on branch {} ({}) in {:?}",
            task.owner,
            task.repo,
            task.branch,
            task.commit_hash,
            path,
        );
        if let Err(err) = std::fs::create_dir_all(&path) {
            tracing::error!(
                path = path.to_string_lossy().as_ref(),
                "Failed to create build directory {:?}: {}",
                path,
                err
            );
            return;
        }

        let lock_key = (task.owner.clone(), task.repo.clone());
        let lock_manager = self.lock_manager.clone();
        actix_web::web::block(move || {
            lock_manager.with_lock(lock_key, || {
                tracing::info!(
                    "Acquired lock for {}/{}, starting build",
                    task.owner,
                    task.repo
                );

                let mut repo = match open_or_clone(&task.url, &path) {
                    Ok(repo) => repo,
                    Err(_) => return Err(()),
                };
                if pull_repo(&mut repo).is_err() {
                    return Err(());
                }
                if checkout(&mut repo, &task.commit_hash).is_err() {
                    return Err(());
                }

                let cmd_res = Command::new("docker-compose")
                    .arg("up")
                    .arg("--build")
                    .arg("-d")
                    .env(
                        "COMPOSE_PROJECT_NAME",
                        format!("adm-{}-{}-{}", &task.owner, &task.repo, &task.branch),
                    )
                    .current_dir(&path)
                    .output();
                match cmd_res {
                    Ok(output) => {
                        if output.status.success() {
                            tracing::info!(
                                "Sucessfully deployed {}/{}#{}",
                                task.owner.as_str(),
                                task.repo.as_str(),
                                task.branch.as_str(),
                            );
                            Ok(())
                        } else {
                            tracing::error!(
                                stdout = String::from_utf8_lossy(&output.stdout).as_ref(),
                                stderr = String::from_utf8_lossy(&output.stderr).as_ref(),
                                "`docker-compose` returned failure. STDERR: {}",
                                String::from_utf8_lossy(&output.stderr),
                            );
                            Err(()) 
                        }
                    }
                    Err(err) => {
                        tracing::error!("Failed to run `docker-compose`: {}", err);
                        Err(())
                    }
                }
            })
        })
        .await
        .ok();
    }

    pub async fn run_builds(self, mut rx: mpsc::Receiver<Task>) {
        while let Some(task) = rx.recv().await {
            let span = tracing::info_span!(
                "processing build task",
                repo.owner = task.owner.as_str(),
                repo.name = task.repo.as_str(),
                commit_id = task.commit_hash.as_str(),
                branch = task.branch.as_str(),
            );
            self.process_task(task).instrument(span).await
        }
    }
}
