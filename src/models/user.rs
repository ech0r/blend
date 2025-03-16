use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct User {
    pub id: String, // GitHub ID
    pub username: String,
    pub avatar_url: String,
    pub access_token: String,
}
