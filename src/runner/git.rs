use std::path::Path;

pub fn open_or_clone(
    url: &str,
    path: &Path,
) -> Result<git2::Repository, (git2::Error, git2::Error)> {
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

pub fn pull_repo(repo: &mut git2::Repository) -> Result<(), git2::Error> {
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

pub fn checkout(repo: &mut git2::Repository, commit_id: &str) -> Result<(), git2::Error> {
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
