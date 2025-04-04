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

// Helper method to get display string for status
impl ReleaseStatus {
    pub fn display_name(&self) -> &'static str {
        match self {
            ReleaseStatus::InDevelopment => "In Development",
            ReleaseStatus::ClearedInDevelopment => "Cleared for Staging",
            ReleaseStatus::WaitingForStaging => "Waiting for Staging",
            ReleaseStatus::WaitingForProduction => "Waiting for Production",
            ReleaseStatus::DeployingToStaging => "Deploying to Staging",
            ReleaseStatus::ReadyToTestInStaging => "Ready to Test in Staging",
            ReleaseStatus::ClearedInStaging => "Cleared for Production",
            ReleaseStatus::WaitingForProductionFromStaging => "Waiting for Production",
            ReleaseStatus::DeployingToProduction => "Deploying to Production",
            ReleaseStatus::ReadyToTestInProduction => "Ready to Test in Production",
            ReleaseStatus::ClearedInProduction => "Completed",
            ReleaseStatus::Error => "Error",
            ReleaseStatus::Blocked => "Blocked",
        }
    }
    
    // Helper to get CSS class for styling
    pub fn css_class(&self) -> &'static str {
        match self {
            ReleaseStatus::InDevelopment => "status-in-development",
            ReleaseStatus::ClearedInDevelopment => "status-cleared",
            ReleaseStatus::WaitingForStaging => "status-waiting",
            ReleaseStatus::WaitingForProduction => "status-waiting",
            ReleaseStatus::DeployingToStaging => "status-deploying status-deploying-to-staging",
            ReleaseStatus::ReadyToTestInStaging => "status-ready",
            ReleaseStatus::ClearedInStaging => "status-cleared",
            ReleaseStatus::WaitingForProductionFromStaging => "status-waiting",
            ReleaseStatus::DeployingToProduction => "status-deploying status-deploying-to-production",
            ReleaseStatus::ReadyToTestInProduction => "status-ready",
            ReleaseStatus::ClearedInProduction => "status-completed",
            ReleaseStatus::Error => "status-error",
            ReleaseStatus::Blocked => "status-blocked",
        }
    }
    
    // Helper to determine the current environment of a release based on its status
    pub fn environment(&self) -> Environment {
        match self {
            ReleaseStatus::InDevelopment | ReleaseStatus::ClearedInDevelopment |
            ReleaseStatus::WaitingForStaging | ReleaseStatus::WaitingForProduction => Environment::Development,
            
            ReleaseStatus::DeployingToStaging | ReleaseStatus::ReadyToTestInStaging | 
            ReleaseStatus::ClearedInStaging | ReleaseStatus::WaitingForProductionFromStaging => Environment::Staging,
            
            ReleaseStatus::DeployingToProduction | ReleaseStatus::ReadyToTestInProduction | 
            ReleaseStatus::ClearedInProduction => Environment::Production,
            
            ReleaseStatus::Error | ReleaseStatus::Blocked => Environment::Development, // Default to Development for error states
        }
    }
    
    // Helper to get next status when a release is cleared
    pub fn next_status_when_cleared(&self, skip_staging: bool) -> Option<ReleaseStatus> {
        match self {
            ReleaseStatus::InDevelopment => {
                if skip_staging {
                    Some(ReleaseStatus::WaitingForProduction)
                } else {
                    Some(ReleaseStatus::WaitingForStaging)
                }
            },
            ReleaseStatus::ReadyToTestInStaging => Some(ReleaseStatus::WaitingForProductionFromStaging),
            ReleaseStatus::ReadyToTestInProduction => Some(ReleaseStatus::ClearedInProduction),
            _ => None, // No next status for other states
        }
    }
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
    #[serde(default)]
    pub skip_staging: bool,
}

impl Release {
    // Helper to determine if a release can be cleared in its current state
    pub fn can_be_cleared(&self) -> bool {
        match self.status {
            ReleaseStatus::InDevelopment | 
            ReleaseStatus::ReadyToTestInStaging | 
            ReleaseStatus::ReadyToTestInProduction => true,
            _ => false,
        }
    }
    
    // Helper to get the next status when cleared
    pub fn next_status(&self) -> Option<ReleaseStatus> {
        self.status.next_status_when_cleared(self.skip_staging)
    }
    
    // Helper to determine which column this release belongs in
    pub fn current_board_column(&self) -> Environment {
        match self.status {
            // Deploying states stay in their source environment
            ReleaseStatus::DeployingToStaging => Environment::Development,
            ReleaseStatus::DeployingToProduction => {
                // If skipping staging, would be in Development, otherwise in Staging
                if self.current_environment == Environment::Development && self.skip_staging {
                    Environment::Development
                } else {
                    Environment::Staging
                }
            },
            // All other states follow the normal environment mapping
            _ => self.status.environment()
        }
    }
}

// User role enum
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum UserRole {
    Viewer,   // Can only view releases
    Deployer, // Can deploy to staging
    Admin     // Can deploy to staging and production
}

impl Default for UserRole {
    fn default() -> Self {
        UserRole::Viewer
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct User {
    pub id: String,
    pub username: String,
    pub avatar_url: String,
    #[serde(default)]
    pub role: UserRole,
}

impl User {
    // Check if user can move releases to staging
    pub fn can_deploy_to_staging(&self) -> bool {
        matches!(self.role, UserRole::Deployer | UserRole::Admin)
    }
    
    // Check if user can move releases to production
    pub fn can_deploy_to_production(&self) -> bool {
        matches!(self.role, UserRole::Admin)
    }
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
    AppLog {
        level: String,      // "info", "warn", "error"
        message: String,
        timestamp: String,
    },
}
