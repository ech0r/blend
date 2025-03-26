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
    WaitingForStaging,      // New status - waiting to be deployed to staging
    WaitingForProduction,   // New status - waiting to be deployed to production directly
    
    // Staging phase
    DeployingToStaging,
    ReadyToTestInStaging,
    ClearedInStaging,
    WaitingForProductionFromStaging, // New status - waiting to be deployed to production from staging
    
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

    // Calculate aggregated progress from deployment items
    pub fn calculate_progress(&self) -> f32 {
        if self.deployment_items.is_empty() {
            return self.progress;
        }

        let item_count = self.deployment_items.len() as f32;
        let completed_progress: f32 = self.deployment_items.iter()
            .map(|item| match item.status {
                ReleaseStatus::ReadyToTestInStaging | 
                ReleaseStatus::ReadyToTestInProduction | 
                ReleaseStatus::ClearedInStaging |
                ReleaseStatus::ClearedInProduction => 100.0,
                ReleaseStatus::Error => 0.0,
                _ => 0.0 // For items still in progress, we'll rely on the individual progress
            })
            .sum();

        completed_progress / item_count
    }

    // Check if all deployment items are completed
    pub fn all_items_completed(&self) -> bool {
        if self.deployment_items.is_empty() {
            return true;
        }

        self.deployment_items.iter().all(|item| {
            matches!(item.status, 
                ReleaseStatus::ReadyToTestInStaging | 
                ReleaseStatus::ReadyToTestInProduction | 
                ReleaseStatus::ClearedInStaging |
                ReleaseStatus::ClearedInProduction |
                ReleaseStatus::Error)
        })
    }

    // Check if any deployment items have errors
    pub fn has_errors(&self) -> bool {
        self.deployment_items.iter().any(|item| matches!(item.status, ReleaseStatus::Error))
    }
    
    // Get the current board column based on status
    pub fn current_board_column(&self) -> Environment {
        match self.status {
            // Development phase statuses
            ReleaseStatus::InDevelopment |
            ReleaseStatus::ClearedInDevelopment |
            ReleaseStatus::WaitingForStaging |
            ReleaseStatus::WaitingForProduction => Environment::Development,
            
            // Staging phase statuses
            ReleaseStatus::DeployingToStaging | 
            ReleaseStatus::ReadyToTestInStaging | 
            ReleaseStatus::ClearedInStaging |
            ReleaseStatus::WaitingForProductionFromStaging => Environment::Staging,
            
            // Production phase statuses
            ReleaseStatus::DeployingToProduction | 
            ReleaseStatus::ReadyToTestInProduction | 
            ReleaseStatus::ClearedInProduction => Environment::Production,
            
            // Error and blocked states - stay in current environment
            ReleaseStatus::Error |
            ReleaseStatus::Blocked => self.current_environment.clone(),
        }
    }
    
    // Check if this release should be processed by the scheduler
    pub fn should_process(&self) -> bool {
        match self.status {
            // Now include the waiting statuses as ones that should be processed
            ReleaseStatus::WaitingForStaging |
            ReleaseStatus::WaitingForProduction |
            ReleaseStatus::WaitingForProductionFromStaging |
            ReleaseStatus::DeployingToStaging | 
            ReleaseStatus::DeployingToProduction => true,
            _ => false,
        }
    }
    
    // Get next status after deployment completes successfully
    pub fn next_status_after_deployment(&self) -> ReleaseStatus {
        match self.status {
            ReleaseStatus::WaitingForStaging |
            ReleaseStatus::DeployingToStaging => ReleaseStatus::ReadyToTestInStaging,
            
            ReleaseStatus::WaitingForProduction |
            ReleaseStatus::WaitingForProductionFromStaging |
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
            // When clearing from Development, go to appropriate Waiting status
            ReleaseStatus::InDevelopment => {
                if self.skip_staging {
                    Some(ReleaseStatus::WaitingForProduction)
                } else {
                    Some(ReleaseStatus::WaitingForStaging)
                }
            },
            
            // When clearing from Staging, go to Waiting status for Production
            ReleaseStatus::ReadyToTestInStaging => Some(ReleaseStatus::WaitingForProductionFromStaging),
            
            // Production completion remains the same
            ReleaseStatus::ReadyToTestInProduction => Some(ReleaseStatus::ClearedInProduction),
            
            _ => None, // No next status for other states
        }
    }
}
