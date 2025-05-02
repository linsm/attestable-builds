#![allow(dead_code)]
#![allow(unused_variables)]

use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum GitHubEvent {
    Ping(PingEvent),
    WorkflowJob(WorkflowJobEvent),
}

//
// WorkflowJob
//

#[derive(Debug, Deserialize)]
pub struct WorkflowJobEvent {
    pub action: String,
    pub workflow_job: WorkflowJob,
    pub repository: Repository,
    pub sender: User,
}

#[derive(Debug, Deserialize)]
pub struct WorkflowJob {
    pub id: usize,
    pub run_id: usize,
    pub run_url: String,
    pub url: String,
    pub status: String,
    pub workflow_name: String,
    pub name: String,
    pub labels: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct Repository {
    pub id: usize,
    pub full_name: String,
}

#[derive(Debug, Deserialize)]
pub struct User {
    pub login: String,
    pub id: usize,
}

//
// Ping
//

#[derive(Debug, Deserialize)]
pub struct PingEvent {
    pub zen: String,
    pub hook_id: usize,
    pub hook: PingEventHook,
}

#[derive(Debug, Deserialize)]
pub enum PingEventHookType {
    App,
    Organization,
    Repository,
}

#[derive(Debug, Deserialize)]
pub struct PingEventHookConfig {
    pub secret: Option<String>,
    pub url: String,
}

#[derive(Debug, Deserialize)]
pub struct PingEventHook {
    #[serde(rename = "type")]
    pub type_: PingEventHookType,
    pub id: usize,
    pub name: String,
    pub active: bool,
    pub app_id: Option<usize>,
    pub config: PingEventHookConfig,
    pub updated_at: String,
    pub created_at: String,
    pub url: String,
    pub test_url: String,
    pub ping_url: String,
    pub deliveries_url: String,
}
