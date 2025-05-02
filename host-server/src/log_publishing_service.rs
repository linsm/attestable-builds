use common::{redact_token, short_wait};
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::fmt::{Display, Formatter};

use tokio::sync::mpsc::Receiver;
use tracing::info;

pub struct TransparencyLogConfiguration {
    pub base_url: String,
    pub username: String,
    pub password: String,
    pub log_id: i64,
    pub simulate: bool,
}

impl Display for TransparencyLogConfiguration {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "TransparencyLogConfiguration {{ base_url: {}, username: {}, password: {}, log_id: {}, simulate: {} }}",
            self.base_url,
            self.username,
            redact_token(&self.password),
            self.log_id,
            self.simulate
        )
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AttestationEntry {
    pub commit_hash: String,
    pub artifact_hash: String,
    pub artifact_name: String,
    pub attestation_document: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct User {
    pub name: String,
    pub password: String,
}

pub async fn run_log_publishing_service_blocking(
    transparency_log_config: TransparencyLogConfiguration,
    attestations_rx: Receiver<AttestationEntry>,
) -> anyhow::Result<()> {
    info!(
        "Started log publishing service: {}",
        transparency_log_config
    );

    if transparency_log_config.simulate {
        info!("Log publishing service is in simulation mode");
        run_simulated_log_publishing_service(attestations_rx).await
    } else {
        run_production_log_publishing_service(transparency_log_config, attestations_rx).await
    }
}

pub async fn run_production_log_publishing_service(
    transparency_log_config: TransparencyLogConfiguration,
    mut attestations_rx: Receiver<AttestationEntry>,
) -> anyhow::Result<()> {
    let client = Client::new();

    let login_endpoint = format!(
        "{}/login/request-access-token",
        transparency_log_config.base_url
    );
    let log_endpoint = format!(
        "{}/logbuilder/add-logentry?log_id={}",
        transparency_log_config.base_url, transparency_log_config.log_id
    );

    let token = client
        .post(&login_endpoint)
        .json(&User {
            name: transparency_log_config.username,
            password: transparency_log_config.password,
        })
        .send()
        .await?
        .text()
        .await?;

    if token.is_empty() {
        return Err(anyhow::anyhow!("Failed to get the authentication token"));
    }

    info!("Logging: authorization token received successfully");

    let mut headers = HeaderMap::new();
    headers.insert(
        AUTHORIZATION,
        HeaderValue::from_str(&format!("Bearer {}", token.trim()))?,
    );

    loop {
        let maybe_entry = attestations_rx.try_recv();

        let Ok(entry) = maybe_entry else {
            short_wait().await;
            continue;
        };
        info!("Logging: {:?}", entry);
        let log_result = client
            .post(&log_endpoint)
            .headers(headers.clone())
            .json(&entry)
            .send()
            .await?
            .text()
            .await?;

        if log_result.is_empty() {
            return Err(anyhow::anyhow!("Failed to log the attestation"));
        }
        info!("Log result: {:?}", log_result);
    }
}

async fn run_simulated_log_publishing_service(
    mut attestations_rx: Receiver<AttestationEntry>,
) -> anyhow::Result<()> {
    loop {
        let maybe_entry = attestations_rx.try_recv();

        let Ok(_entry) = maybe_entry else {
            short_wait().await;
            continue;
        };

        info!("[simulated] Received entry :)");
    }
}
