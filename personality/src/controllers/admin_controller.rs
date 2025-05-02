use crate::models::personality_config::PersonalityConfig;
use crate::models::response_model::ApiResponse;
use crate::models::token_model::AccessToken;
use crate::trillian_rust::trillian::trillian_admin_client::TrillianAdminClient;
use crate::trillian_rust::trillian::trillian_log_client::TrillianLogClient;
use crate::trillian_rust::trillian::{CreateTreeRequest, InitLogRequest, Tree};
use prost_types::Duration;
use rocket::http::Status;
use rocket::State;

use super::ADMIN_ROLE;

#[post("/create-tree?<name>&<description>")]
pub(crate) async fn create_tree(
    name: String,
    description: String,
    key: Result<AccessToken, ApiResponse>,
    config: &State<PersonalityConfig>,
) -> Result<String, Status> {
    let token: AccessToken = key.unwrap();
    if token.claims.role != ADMIN_ROLE {
        return Err(Status::Unauthorized);
    }
    let mut admin_client = TrillianAdminClient::connect(config.trillian_url.clone())
        .await
        .unwrap();

    let tree = Tree {
        tree_id: 1,
        tree_state: 1,
        tree_type: 1,
        display_name: name,
        description,
        storage_settings: None,
        max_root_duration: Option::from(Duration {
            seconds: 0,
            nanos: 0,
        }),
        create_time: None,
        update_time: None,
        deleted: false,
        delete_time: None,
    };

    let request = tonic::Request::new(CreateTreeRequest { tree: Some(tree) });
    let response: tonic::Response<Tree> = admin_client.create_tree(request).await.unwrap();
    let tree_id = response.into_inner().tree_id;
    
    let mut log_client = TrillianLogClient::connect(config.trillian_url.clone())
        .await
        .unwrap();

    let request = tonic::Request::new(InitLogRequest {
        log_id: tree_id,
        charge_to: None,
    });
    log_client.init_log(request).await.unwrap();
    Ok(tree_id.to_string())
}
