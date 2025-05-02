use rocket::http::Status;
use rocket::serde::json::Json;
use rocket::State;

use crate::models::log_entry::LogEntry;
use crate::models::personality_config::PersonalityConfig;
use crate::models::response_model::ApiResponse;
use crate::models::token_model::AccessToken;
use crate::trillian_rust::trillian::trillian_log_client::TrillianLogClient;
use crate::trillian_rust::trillian::{LogLeaf, QueueLeafRequest};

use super::BUILDSERVER_ROLE;

#[post("/add-logentry?<log_id>", format = "json", data = "<log_entry>")]
pub async fn add_log_entry<'a>(
    log_id: i64,
    log_entry: Json<LogEntry>,
    key: Result<AccessToken, ApiResponse>,
    config: &State<PersonalityConfig>,
) -> Result<&'a str, Status> {
    let token = key.unwrap();
    if token.claims.role != BUILDSERVER_ROLE {
        return Err(Status::Unauthorized);
    }
    let mut client = TrillianLogClient::connect(config.trillian_url.clone())
        .await
        .unwrap();

    let log_data: LogEntry = log_entry.0;
    let request = tonic::Request::new(QueueLeafRequest {
        log_id,
        leaf: Option::from(LogLeaf {
            merkle_leaf_hash: vec![],
            leaf_value: log_data.to_byte_array(),
            extra_data: vec![],
            leaf_index: 0,
            leaf_identity_hash: vec![],
            queue_timestamp: None,
            integrate_timestamp: None,
        }),
        charge_to: None,
    });
    client.queue_leaf(request).await.unwrap();
    Ok("Successfully created log entry")
}
