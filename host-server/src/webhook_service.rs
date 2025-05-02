use crate::webhook_types::{GitHubEvent, WorkflowJobEvent};
use crate::BackendCommand;
use axum::routing::get;
use axum::{http::StatusCode, routing::post, Extension, Json, Router};
use common::messages::{create_new_timestamp_now, log_timestamp};
use tokio::sync::mpsc::Sender;
use tower_http::trace::TraceLayer;
use tracing::{debug, error, info};

pub async fn run_webhook_service_blocking(tx: Sender<BackendCommand>) -> anyhow::Result<()> {
    let app = Router::new()
        .route("/", post(post_root))
        .route("/", get(get_root))
        .layer(Extension(tx))
        .layer(TraceLayer::new_for_http());

    let socket_addr = "0.0.0.0:8000".parse()?;
    info!("Webhook service is listening on: {}", socket_addr);
    axum_server::Server::bind(socket_addr)
        .serve(app.into_make_service())
        .await?;
    error!("The web service has stopped");

    Ok(())
}

async fn post_root(
    channel: Extension<Sender<BackendCommand>>,
    Json(payload): Json<GitHubEvent>,
) -> StatusCode {
    debug!("Received payload: {:?}", payload);
    match payload {
        GitHubEvent::Ping(_) => handle_ping().await,
        GitHubEvent::WorkflowJob(workflow_job_event) => {
            let result = handle_workflow_job(channel.0, workflow_job_event).await;
            if result.is_ok() {
                StatusCode::OK
            } else {
                StatusCode::INTERNAL_SERVER_ERROR
            }
        }
    }
}

async fn handle_ping() -> StatusCode {
    StatusCode::OK
}

async fn handle_workflow_job(
    channel: Sender<BackendCommand>,
    workflow_job_event: WorkflowJobEvent,
) -> anyhow::Result<()> {
    match workflow_job_event.action.as_str() {
        "queued" => {
            let run_id = workflow_job_event.workflow_job.run_id as u32;
            log_timestamp(&create_new_timestamp_now("WEBHOOK"));
            channel.send(BackendCommand::Start { run_id }).await?;
        }
        "completed" => {
            let run_id = workflow_job_event.workflow_job.run_id as u32;
            channel.send(BackendCommand::Stop { run_id }).await?;
        }
        _ => {}
    }
    Ok(())
}

async fn get_root() -> StatusCode {
    StatusCode::OK
}
