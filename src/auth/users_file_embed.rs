use std::collections::HashMap;
use log::{info, warn, debug};
use crate::models::user::UserRole;

// Store allowed users and their roles
#[derive(Debug, Clone)]
pub struct AllowedUsers {
    users: HashMap<String, UserRole>,
}

impl AllowedUsers {
    // Parse users from a string content (embedded in binary)
    pub fn from_content(content: &str) -> Self {
        let mut users = HashMap::new();
        
        info!("Parsing embedded users file content");
        
        let mut line_number = 0;
        for line in content.lines() {
            line_number += 1;
            
            // Skip comments and empty lines
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            
            // Parse line in format username:role
            let parts: Vec<&str> = line.split(':').collect();
            if parts.len() != 2 {
                warn!("Invalid line {} in users file: {}", line_number, line);
                continue;
            }
            
            let username = parts[0].trim().to_lowercase();
            let role_str = parts[1].trim().to_lowercase();
            
            debug!("Parsing user entry: '{}' with role '{}'", username, role_str);
            
            let role = match role_str.as_str() {
                "admin" => UserRole::Admin,
                "deployer" => UserRole::Deployer,
                "viewer" => UserRole::Viewer,
                _ => {
                    warn!("Unknown role '{}' for user {} on line {}, defaulting to viewer", 
                          role_str, username, line_number);
                    UserRole::Viewer
                }
            };
            
            debug!("Adding user '{}' with role '{:?}'", username, role);
            users.insert(username, role);
        }
        
        info!("Loaded {} allowed users from embedded content", users.len());
        
        // Debug log all loaded users
        for (username, role) in &users {
            debug!("Loaded user: '{}' with role '{:?}'", username, role);
        }
        
        Self { users }
    }
    
    // Check if a user is allowed and get their role
    pub fn get_role(&self, username: &str) -> Option<UserRole> {
        let lowercase_username = username.trim().to_lowercase();
        
        // First try an exact match
        if let Some(role) = self.users.get(&lowercase_username) {
            debug!("Found exact match for user '{}' with role '{:?}'", lowercase_username, role);
            return Some(role.clone());
        }
        
        // Debug log all known users for troubleshooting
        debug!("User '{}' not found. Known users:", lowercase_username);
        for (known_user, role) in &self.users {
            debug!("  '{}' -> '{:?}'", known_user, role);
        }
        
        None
    }
    
    // Check if a user is allowed
    pub fn is_allowed(&self, username: &str) -> bool {
        let lowercase_username = username.trim().to_lowercase();
        let result = self.users.contains_key(&lowercase_username);
        debug!("Checking if user '{}' is allowed: {}", lowercase_username, result);
        result
    }
}

// Create a default set of users as fallback
impl Default for AllowedUsers {
    fn default() -> Self {
        let mut users = HashMap::new();
        
        // Add default users for testing
        users.insert("admin".to_string(), UserRole::Admin);
        users.insert("deployer".to_string(), UserRole::Deployer);
        users.insert("viewer".to_string(), UserRole::Viewer);
        
        info!("Using default allowed users list");
        
        Self { users }
    }
}

// Embed the .users file content directly into the binary
// This uses Rust's include_str! macro which pulls file content at compile time
const EMBEDDED_USERS_FILE: &str = include_str!("../../.users");

// Singleton instance that can be accessed from anywhere
lazy_static::lazy_static! {
    pub static ref ALLOWED_USERS: AllowedUsers = {
        info!("Loading users from embedded file content");
        AllowedUsers::from_content(EMBEDDED_USERS_FILE)
    };
}
