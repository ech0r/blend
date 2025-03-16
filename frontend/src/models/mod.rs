use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Environment {
    Development,
    Staging,
    Production,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ReleaseStatus {
    Pending,
    InProgress,
    Completed,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DeploymentItem {
    pub name: String,
    pub status: ReleaseStatus,
    pub logs: Vec<String>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Release {
    pub id: String,
    pub title: String,
    pub client_id: String,
    pub current_environment: Environment,
    pub target_environment: Environment,
    pub deployment_items: Vec<DeploymentItem>,
    pub created_at: DateTime<Utc>,
    pub scheduled_at: DateTime<Utc>,
    pub status: ReleaseStatus,
    pub created_by: String,
    pub progress: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct User {
    pub id: String,
    pub username: String,
    pub avatar_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Client {
    pub id: String,
    pub name: String,
}

// WebSocket message types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type")]
pub enum WsMessage {
    Chat {
        username: String,
        message: String,
        timestamp: String,
    },
    ReleaseUpdate {
        release_id: String,
        status: String,
        progress: f32,
        log_line: Option<String>,
    },
}
