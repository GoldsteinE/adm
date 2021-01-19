use actix_web::web;
use tokio::sync::mpsc;

use crate::{github::PushEvent, http::Webhook, runner::Task};

#[derive(Debug, Clone, thiserror::Error)]
pub enum PushHookError {
    #[error("ref must have format refs/heads/<branch>")]
    NotBranch,
    #[error("only pushes to master branch are processed")]
    NotMaster,
    #[error("failed to queue build task")]
    SendError,
}

impl actix_web::ResponseError for PushHookError {
    fn status_code(&self) -> actix_web::http::StatusCode {
        match self {
            PushHookError::NotBranch => actix_web::http::StatusCode::BAD_REQUEST,
            PushHookError::NotMaster => actix_web::http::StatusCode::OK,
            PushHookError::SendError => actix_web::http::StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

pub async fn push_hook(
    Webhook(hook): Webhook<PushEvent>,
    tx: web::Data<mpsc::Sender<Task>>,
) -> Result<String, PushHookError> {
    let branch = if let Some(branch) = hook.reference.strip_prefix("refs/heads/") {
        branch
    } else {
        return Err(PushHookError::NotBranch);
    };

    if branch != "master" {
        return Err(PushHookError::NotMaster);
    }

    let task = Task {
        url: hook.repository.url,
        owner: hook.repository.owner.login,
        repo: hook.repository.name,
        branch: branch.to_string(),
        commit_hash: hook.after,
    };

    tx.send(task)
        .await
        .map(|()| "OK".into())
        .map_err(|_| PushHookError::SendError)
}
