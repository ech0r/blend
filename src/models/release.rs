use serde::{Deserialize, Serialize};
use uuid::Uuid;
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
    pub name: String, // "data", "solr", or "app"
    pub status: ReleaseStatus,
    pub logs: Vec<String>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Release {
    pub id: Uuid,
    pub title: String,
    pub client_id: String,
    pub current_environment: Environment,
    pub target_environment: Environment,
    pub deployment_items: Vec<DeploymentItem>,
    pub created_at: DateTime<Utc>,
    pub scheduled_at: DateTime<Utc>,
    pub status: ReleaseStatus,
    pub created_by: String, // GitHub username
    pub progress: f32, // 0.0 to 100.0
}

impl Release {
    pub fn new(
        title: String,
        client_id: String,
        current_environment: Environment,
        target_environment: Environment,
        deployment_items: Vec<String>,
        scheduled_at: DateTime<Utc>,
        created_by: String,
    ) -> Self {
        let deployment_items = deployment_items
            .into_iter()
            .map(|name| DeploymentItem {
                name,
                status: ReleaseStatus::Pending,
                logs: Vec::new(),
                error: None,
            })
            .collect();

        Self {
            id: Uuid::new_v4(),
            title,
            client_id,
            current_environment,
            target_environment,
            deployment_items,
            created_at: Utc::now(),
            scheduled_at,
            status: ReleaseStatus::Pending,
            created_by,
            progress: 0.0,
        }
    }
}
