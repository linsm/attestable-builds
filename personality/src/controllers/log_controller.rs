use rocket::serde::json::Json;
use rocket::State;

use crate::models::log_entry::LogEntry;
use crate::models::log_root::LogRoot;
use crate::models::personality_config::PersonalityConfig;
use crate::models::proof_model::ProofModel;
use crate::models::tree_model::TreeModel;
use crate::trillian_rust::trillian::trillian_admin_client::TrillianAdminClient;
use crate::trillian_rust::trillian::trillian_log_client::TrillianLogClient;
use crate::trillian_rust::trillian::{
    GetInclusionProofByHashRequest, GetLatestSignedLogRootRequest, ListTreesRequest,
    ListTreesResponse,
};

#[get("/list-trees")]
pub async fn list_trees(config: &State<PersonalityConfig>) -> Json<Vec<TreeModel>> {
    let mut client = TrillianAdminClient::connect(config.trillian_url.clone())
        .await
        .unwrap();
    let request = tonic::Request::new(ListTreesRequest {
        show_deleted: false,
    });
    let response: tonic::Response<ListTreesResponse> = client.list_trees(request).await.unwrap();

    let tree = response.into_inner();
    let trees = tree.tree;

    let result = trees.into_iter().map(|tree| tree.into()).collect();
    Json(result)
}

#[post(
    "/inclusion-proof?<log_id>&<tree_size>",
    format = "json",
    data = "<log_entry>"
)]
pub(crate) async fn inclusion_proof(
    log_id: i64,
    tree_size: i64,
    log_entry: Json<LogEntry>,
    config: &State<PersonalityConfig>,
) -> Json<Vec<ProofModel>> {
    let mut client = TrillianLogClient::connect(config.trillian_url.clone())
        .await
        .unwrap();
    let log_data = log_entry.0;
    let request = tonic::Request::new(GetInclusionProofByHashRequest {
        log_id,
        leaf_hash: log_data.to_merkle_hash(),
        tree_size,
        order_by_sequence: false,
        charge_to: None,
    });
    let response: tonic::Response<crate::trillian_rust::trillian::GetInclusionProofByHashResponse> =
        client.get_inclusion_proof_by_hash(request).await.unwrap();
    let res: crate::trillian_rust::trillian::GetInclusionProofByHashResponse =
        response.into_inner().clone();
    let proofs: Vec<crate::trillian_rust::trillian::Proof> = res.proof;
    let mut log_root = Vec::new();
    if let Some(x) = res.signed_log_root {
        log_root.clone_from(&x.log_root)
    };

    let mut result: Vec<ProofModel> = vec![];
    for x in proofs {
        let y = ProofModel::new(x.leaf_index, x.hashes, log_root.clone());
        result.push(y);
    }
    Json(result)
}

#[get("/latest-signed-log-root?<tree_id>")]
pub(crate) async fn latest_signed_log_root(
    tree_id: i64,
    config: &State<PersonalityConfig>,
) -> Json<LogRoot> {
    let mut client = TrillianLogClient::connect(config.trillian_url.clone())
        .await
        .unwrap();

    let request = tonic::Request::new(GetLatestSignedLogRootRequest {
        log_id: tree_id,
        charge_to: None,
        first_tree_size: 0,
    });

    let response = client.get_latest_signed_log_root(request).await.unwrap();
    let mut result = String::new();
    if let Some(x) = response.into_inner().signed_log_root {
        result = hex::encode(x.log_root)
    }
    let log_root = LogRoot { log_root: result };
    Json(log_root)
}
