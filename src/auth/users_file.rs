use std::fs::File;
use std::io::{self, BufRead};
use std::path::Path;
use std::collections::HashMap;
use log::{info, warn, error, debug};
use crate::models::user::UserRole;

// Store allowed users and their roles
#[derive(Debug, Clone)]
pub struct AllowedUsers {
    users: HashMap<String, UserRole>,
}

impl AllowedUsers {
    // Load users from a file
    pub fn from_file<P: AsRef<Path>>(path: P) -> io::Result<Self> {
        let path_ref = path.as_ref();
        info!("Loading users from file: {:?}", path_ref);
        
        let mut users = HashMap::new();
        
        // Check if file exists
        if !path_ref.exists() {
            warn!("Users file not found at {:?}", path_ref);
            return Err(io::Error::new(io::ErrorKind::NotFound, "Users file not found"));
        }
        
        let file = File::open(path_ref)?;
        let lines = io::BufReader::new(file).lines();
        
        info!("File opened successfully, parsing lines");
        
        let mut line_number = 0;
        for line_result in lines {
            line_number += 1;
            match line_result {
                Ok(line) => {
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
                },
                Err(e) => {
                    warn!("Error reading line {} in users file: {}", line_number, e);
                }
            }
        }
        
        info!("Loaded {} allowed users from file", users.len());
        
        // Debug log all loaded users
        for (username, role) in &users {
            debug!("Loaded user: '{}' with role '{:?}'", username, role);
        }
        
        Ok(Self { users })
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

// Create a default AllowedUsers with some predefined users
// This is used as a fallback if the .users file doesn't exist
impl Default for AllowedUsers {
    fn default() -> Self {
        let mut users = HashMap::new();
        
        // Add some default users for testing
        users.insert("admin".to_string(), UserRole::Admin);
        users.insert("deployer".to_string(), UserRole::Deployer);
        users.insert("viewer".to_string(), UserRole::Viewer);
        
        info!("Using default allowed users list with 3 users");
        
        Self { users }
    }
}

// Singleton instance that can be accessed from anywhere
lazy_static::lazy_static! {
    pub static ref ALLOWED_USERS: AllowedUsers = {
        // Try to load from .users file in various locations
        let locations = vec![
            ".users",
            "./users",
            "../.users",
            "users.txt",
            "./users.txt"
        ];
        
        for location in locations {
            info!("Trying to load users file from: {}", location);
            match AllowedUsers::from_file(location) {
                Ok(users) => {
                    info!("Successfully loaded users from {}", location);
                    return users;
                },
                Err(e) => {
                    warn!("Failed to load users file from {}: {}", location, e);
                }
            }
        }
        
        warn!("Could not load users file from any location. Using default allowed users.");
        AllowedUsers::default()
    };
}
