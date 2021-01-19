mod git;

use std::{path::PathBuf, process::Command, sync::Arc};

use actix::prelude::*;
use color_eyre::eyre::{self, WrapErr as _};

use crate::lock_manager::LockManager;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct BranchSpec {
    pub owner: String,
    pub repo: String,
    pub branch: String,
}

#[derive(Debug, Clone, Message)]
#[rtype(result = "eyre::Result<()>")]
pub struct Task {
    pub branch_spec: BranchSpec,
    pub commit_hash: String,
    pub url: String,
}

#[derive(Debug, Clone)]
pub struct Runner {
    base_path: PathBuf,
    lock_manager: Arc<LockManager<BranchSpec>>,
}

impl Runner {
    pub fn new(base_path: PathBuf, lock_manager: Arc<LockManager<BranchSpec>>) -> Self {
        Self {
            base_path,
            lock_manager,
        }
    }

    fn process_task(&self, task: Task) -> eyre::Result<()> {
        let lock_key = task.branch_spec.clone();
        let Task {
            url,
            commit_hash,
            branch_spec:
                BranchSpec {
                    owner,
                    branch,
                    repo: repo_name,
                },
        } = task;

        let path = {
            let mut p = self.base_path.join(&owner);
            p.push(&repo_name);
            p.push(&branch);
            p
        };
        tracing::info!(
            "Running build for {}/{} on branch {} ({}) in {:?}",
            owner,
            repo_name,
            branch,
            commit_hash,
            path,
        );
        std::fs::create_dir_all(&path)
            .wrap_err_with(|| format!("Failed to create build directory {:?}", path))?;

        self.lock_manager.with_lock::<eyre::Result<()>, _>(lock_key, || -> eyre::Result<()> {
            tracing::info!("Acquired lock for {}/{}, starting build", owner, repo_name);

            let mut repo = git::open_or_clone(&url, &path).map_err(|err| -> eyre::Report {
                eyre::Report::new(err.0)
                    .wrap_err(err.1)
                    .wrap_err("Failed to open or clone repo")
            })?;
            git::pull_repo(&mut repo).wrap_err("Failed to pull repo")?;
            git::checkout(&mut repo, &commit_hash).wrap_err("Failed to checkout repo")?;

            let cmd_res = Command::new("docker-compose")
                .arg("up")
                .arg("--build")
                .arg("-d")
                .env(
                    "COMPOSE_PROJECT_NAME",
                    format!("adm-{}-{}-{}", &owner, &repo_name, &branch),
                )
                .current_dir(&path)
                .output();
            match cmd_res {
                Ok(output) => {
                    if output.status.success() {
                        tracing::info!(
                            "Sucessfully deployed {}/{}#{}",
                            owner.as_str(),
                            repo_name.as_str(),
                            branch.as_str(),
                        );
                        Ok(())
                    } else {
                        tracing::error!(
                            stdout = String::from_utf8_lossy(&output.stdout).as_ref(),
                            stderr = String::from_utf8_lossy(&output.stderr).as_ref(),
                            "`docker-compose` returned failure. STDERR: {}",
                            String::from_utf8_lossy(&output.stderr),
                        );
                        todo!()
                    }
                }
                Err(err) => {
                    tracing::error!("Failed to run `docker-compose`: {}", err);
                    todo!()
                }
            }
            Ok(())
        })
    }
}

impl Actor for Runner {
    type Context = SyncContext<Self>;
}

impl Handler<Task> for Runner {
    type Result = <Task as Message>::Result;

    fn handle(&mut self, msg: Task, _ctx: &mut Self::Context) -> Self::Result {
        todo!()
    }
}
