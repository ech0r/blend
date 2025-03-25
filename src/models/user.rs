use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum UserRole {
    Viewer,     // Can only view releases
    Deployer,   // Can deploy to staging
    Admin       // Can deploy to staging and production
}

impl Default for UserRole {
    fn default() -> Self {
        UserRole::Viewer
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct User {
    pub id: String, // GitHub ID
    pub username: String,
    pub avatar_url: String,
    pub access_token: String,
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
