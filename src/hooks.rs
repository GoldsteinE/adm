use actix_web::web;

use crate::{
    github::PushEvent,
    http::Webhook,
    runner::{BranchSpec, Runner, Task},
};

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
    tx: web::Data<actix::Addr<Runner>>,
) -> Result<String, PushHookError> {
    let branch = hook.reference
        .strip_prefix("refs/heads/")
        .ok_or(PushHookError::NotBranch)?;

    if branch != "master" {
        return Err(PushHookError::NotMaster);
    }

    let task = Task {
        branch_spec: BranchSpec {
            owner: hook.repository.owner.login,
            repo: hook.repository.name,
            branch: branch.to_string(),
        },
        url: hook.repository.url,
        commit_hash: hook.after,
    };

    match tx.try_send(task) {
        Ok(()) => Ok("OK".into()),
        err @ Err(_) => {
            tracing::error!("Failed to send task: {:?}", err);
            Err(PushHookError::SendError)
        }
    }
}
