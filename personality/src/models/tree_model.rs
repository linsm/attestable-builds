use serde::{Deserialize, Serialize};

use crate::trillian_rust::trillian::Tree;

#[derive(Serialize, Deserialize)]
pub struct TreeModel {
    pub id: i64,
    pub name: String,
    pub description: String,
}

impl TreeModel {
    pub fn new(id: i64, name: String, description: String) -> Self {
        Self {
            id,
            name,
            description,
        }
    }
}

impl From<Tree> for TreeModel {
    fn from(value: Tree) -> Self {
        Self::new(value.tree_id, value.display_name, value.description)
    }
}
