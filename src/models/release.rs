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
    // Development phase
    InDevelopment,
    ClearedInDevelopment,
    
    // Staging phase
    DeployingToStaging,
    ReadyToTestInStaging,
    ClearedInStaging,
    
    // Production phase
    DeployingToProduction,
    ReadyToTestInProduction,
    ClearedInProduction,
    
    // Error states
    Error,
    Blocked,
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
    #[serde(default)]
    pub skip_staging: bool, // Flag to indicate if staging should be skipped
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
        skip_staging: bool,
    ) -> Self {
        let deployment_items = deployment_items
            .into_iter()
            .map(|name| DeploymentItem {
                name,
                status: ReleaseStatus::InDevelopment,
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
            status: ReleaseStatus::InDevelopment,
            created_by,
            progress: 0.0,
            skip_staging,
        }
    }
    
    // Get the current board column based on status
    pub fn current_board_column(&self) -> Environment {
        match self.status {
            ReleaseStatus::InDevelopment | 
            ReleaseStatus::ClearedInDevelopment => Environment::Development,
            
            ReleaseStatus::DeployingToStaging | 
            ReleaseStatus::ReadyToTestInStaging | 
            ReleaseStatus::ClearedInStaging => Environment::Staging,
            
            ReleaseStatus::DeployingToProduction | 
            ReleaseStatus::ReadyToTestInProduction | 
            ReleaseStatus::ClearedInProduction => Environment::Production,
            
            ReleaseStatus::Error | 
            ReleaseStatus::Blocked => self.current_environment.clone(), // Add clone() here
        }
    }
    
    // Check if this release should be processed by the scheduler
    pub fn should_process(&self) -> bool {
        match self.status {
            ReleaseStatus::DeployingToStaging | 
            ReleaseStatus::DeployingToProduction => true,
            _ => false,
        }
    }
    
    // Get next status after deployment completes successfully
    pub fn next_status_after_deployment(&self) -> ReleaseStatus {
        match self.status {
            ReleaseStatus::DeployingToStaging => ReleaseStatus::ReadyToTestInStaging,
            ReleaseStatus::DeployingToProduction => ReleaseStatus::ReadyToTestInProduction,
            _ => self.status.clone(), // No change for other statuses
        }
    }
    
    // Get next status after deployment fails
    pub fn error_status(&self) -> ReleaseStatus {
        ReleaseStatus::Error
    }
    
    // Helper to determine if a release can be cleared in its current state
    pub fn can_be_cleared(&self) -> bool {
        match self.status {
            ReleaseStatus::InDevelopment | 
            ReleaseStatus::ReadyToTestInStaging | 
            ReleaseStatus::ReadyToTestInProduction => true,
            _ => false,
        }
    }
    
    // Get the next status when cleared
    pub fn next_status_when_cleared(&self) -> Option<ReleaseStatus> {
        match self.status {
            ReleaseStatus::InDevelopment => {
                if self.skip_staging {
                    Some(ReleaseStatus::DeployingToProduction)
                } else {
                    Some(ReleaseStatus::DeployingToStaging)
                }
            },
            ReleaseStatus::ReadyToTestInStaging => Some(ReleaseStatus::DeployingToProduction),
            ReleaseStatus::ReadyToTestInProduction => Some(ReleaseStatus::ClearedInProduction),
            _ => None, // No next status for other states
        }
    }
}
