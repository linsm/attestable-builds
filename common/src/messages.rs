use crate::EnclaveClientArgs;
use chrono;
use serde::{Deserialize, Serialize};
use tracing::info;

#[derive(Serialize, Deserialize, Debug)]
pub enum Message {
    HostToEnclave(HostToEnclaveMessage),
    EnclaveToHost(EnclaveToHostMessage),
}

#[derive(Serialize, Deserialize, Debug)]
pub enum HostToEnclaveMessage {
    StartRunner {
        enclave_client_args: EnclaveClientArgs,
    },
    Ok {
        info: Option<String>,
    },
}

#[derive(Serialize, Deserialize, Debug)]
pub enum EnclaveToHostMessage {
    ReportRepositoryRoot {
        commit_hash: String,
    },
    ReportArtifact {
        artifact_hash: String,
        artifact_name: String,
    },
    ReportAttestation {
        attestation_document: String,
    },
    Ok {
        info: Option<String>,
    },
    Log {
        message: String,
    },
    Timestamp {
        marker: String,
        datetime: String,
    },
}

pub fn create_new_timestamp_now(marker: &str) -> EnclaveToHostMessage {
    EnclaveToHostMessage::Timestamp {
        marker: marker.to_string(),
        datetime: chrono::Utc::now().to_rfc3339_opts(chrono::format::SecondsFormat::Millis, false),
    }
}

pub fn log_timestamp(timestamp: &EnclaveToHostMessage) {
    if let EnclaveToHostMessage::Timestamp { marker, datetime } = timestamp {
        info!("TIMESTAMP: {} {}", marker, datetime);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_serde() {
        let m1 = Message::HostToEnclave(HostToEnclaveMessage::Ok {
            info: Some("foobar".to_string()),
        });
        let bytes = bincode::serialize(&m1).unwrap();
        let x1: Message = bincode::deserialize(&bytes).unwrap();
        let Message::HostToEnclave(HostToEnclaveMessage::Ok { info }) = x1 else {
            panic!("unexpected message deserialized");
        };
        assert_eq!(info, Some("foobar".to_string()));
    }
}
