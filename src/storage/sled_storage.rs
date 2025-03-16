use sled::{Db, Result as SledResult};
use std::path::Path;
use std::sync::{Arc, Mutex};
use uuid::Uuid;
use chrono::Utc;
use crate::models::{Release, User, Client, ReleaseStatus};
use log::{info, error};
use serde::{de::DeserializeOwned, Serialize};
use std::collections::HashMap;

// Key prefixes for storing different types
const RELEASE_PREFIX: &str = "release:";
const USER_PREFIX: &str = "user:";
const CLIENT_PREFIX: &str = "client:";
const SESSION_PREFIX: &str = "session:";
const WEBSOCKET_PREFIX: &str = "ws:";

#[derive(Clone)]
pub struct SledStorage {
    db: Db,
    active_websockets: Arc<Mutex<HashMap<String, String>>>, // UUID -> User ID
}

impl SledStorage {
    pub fn new() -> SledResult<Self> {
        let db_path = std::env::var("DB_PATH").unwrap_or_else(|_| "data".to_string());
        let db = sled::open(Path::new(&db_path))?;
        Ok(Self { 
            db,
            active_websockets: Arc::new(Mutex::new(HashMap::new())),
        })
    }
    
    // Generic methods for serialization and deserialization
    fn serialize<T: Serialize>(value: &T) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        Ok(serde_json::to_vec(value)?)
    }
    
    fn deserialize<T: DeserializeOwned>(bytes: &[u8]) -> Result<T, Box<dyn std::error::Error>> {
        Ok(serde_json::from_slice(bytes)?)
    }
    
    // Release methods
    pub fn save_release(&self, release: &Release) -> Result<(), Box<dyn std::error::Error>> {
        let key = format!("{}{}", RELEASE_PREFIX, release.id);
        let value = Self::serialize(release)?;
        self.db.insert(key, value)?;
        Ok(())
    }
    
    pub fn get_release(&self, id: &Uuid) -> Result<Option<Release>, Box<dyn std::error::Error>> {
        let key = format!("{}{}", RELEASE_PREFIX, id);
        if let Some(bytes) = self.db.get(key)? {
            return Ok(Some(Self::deserialize(&bytes)?));
        }
        Ok(None)
    }
    
    pub fn delete_release(&self, id: &Uuid) -> Result<(), Box<dyn std::error::Error>> {
        let key = format!("{}{}", RELEASE_PREFIX, id);
        self.db.remove(key)?;
        Ok(())
    }
    
    pub fn get_all_releases(&self) -> Result<Vec<Release>, Box<dyn std::error::Error>> {
        let mut releases = Vec::new();
        let prefix = RELEASE_PREFIX.as_bytes();
        
        for item in self.db.scan_prefix(prefix) {
            let (_, value) = item?;
            let release: Release = Self::deserialize(&value)?;
            releases.push(release);
        }
        
        Ok(releases)
    }
    
    // User methods
    pub fn save_user(&self, user: &User) -> Result<(), Box<dyn std::error::Error>> {
        let key = format!("{}{}", USER_PREFIX, user.id);
        let value = Self::serialize(user)?;
        self.db.insert(key, value)?;
        Ok(())
    }
    
    pub fn get_user(&self, id: &str) -> Result<Option<User>, Box<dyn std::error::Error>> {
        let key = format!("{}{}", USER_PREFIX, id);
        if let Some(bytes) = self.db.get(key)? {
            return Ok(Some(Self::deserialize(&bytes)?));
        }
        Ok(None)
    }
    
    pub fn get_all_users(&self) -> Result<Vec<User>, Box<dyn std::error::Error>> {
        let mut users = Vec::new();
        let prefix = USER_PREFIX.as_bytes();
        
        for item in self.db.scan_prefix(prefix) {
            let (_, value) = item?;
            let user: User = Self::deserialize(&value)?;
            users.push(user);
        }
        
        Ok(users)
    }
    
    // Client methods
    pub fn save_client(&self, client: &Client) -> Result<(), Box<dyn std::error::Error>> {
        let key = format!("{}{}", CLIENT_PREFIX, client.id);
        let value = Self::serialize(client)?;
        self.db.insert(key, value)?;
        Ok(())
    }
    
    pub fn get_client(&self, id: &Uuid) -> Result<Option<Client>, Box<dyn std::error::Error>> {
        let key = format!("{}{}", CLIENT_PREFIX, id);
        if let Some(bytes) = self.db.get(key)? {
            return Ok(Some(Self::deserialize(&bytes)?));
        }
        Ok(None)
    }
    
    pub fn get_all_clients(&self) -> Result<Vec<Client>, Box<dyn std::error::Error>> {
        let mut clients = Vec::new();
        let prefix = CLIENT_PREFIX.as_bytes();
        
        for item in self.db.scan_prefix(prefix) {
            let (_, value) = item?;
            let client: Client = Self::deserialize(&value)?;
            clients.push(client);
        }
        
        Ok(clients)
    }
    
    // Session methods
    pub fn save_session(&self, session_id: &str, user_id: &str) -> Result<(), Box<dyn std::error::Error>> {
        let key = format!("{}{}", SESSION_PREFIX, session_id);
        self.db.insert(key, user_id.as_bytes())?;
        Ok(())
    }
    
    pub fn get_session(&self, session_id: &str) -> Result<Option<String>, Box<dyn std::error::Error>> {
        let key = format!("{}{}", SESSION_PREFIX, session_id);
        if let Some(bytes) = self.db.get(key)? {
            return Ok(Some(String::from_utf8(bytes.to_vec())?));
        }
        Ok(None)
    }
    
    pub fn delete_session(&self, session_id: &str) -> Result<(), Box<dyn std::error::Error>> {
        let key = format!("{}{}", SESSION_PREFIX, session_id);
        self.db.remove(key)?;
        Ok(())
    }
    
    // WebSocket connection management
    pub fn add_websocket(&self, ws_id: &str, user_id: &str) -> Result<(), Box<dyn std::error::Error>> {
        let mut websockets = self.active_websockets.lock().unwrap();
        websockets.insert(ws_id.to_string(), user_id.to_string());
        
        // Also persist to DB for recovery on restart
        let key = format!("{}{}", WEBSOCKET_PREFIX, ws_id);
        self.db.insert(key, user_id.as_bytes())?;
        
        info!("Added WebSocket {} for user {}. Total active: {}", ws_id, user_id, websockets.len());
        
        Ok(())
    }
    
    pub fn remove_websocket(&self, ws_id: &str) -> Result<(), Box<dyn std::error::Error>> {
        let mut websockets = self.active_websockets.lock().unwrap();
        websockets.remove(ws_id);
        
        // Remove from DB as well
        let key = format!("{}{}", WEBSOCKET_PREFIX, ws_id);
        self.db.remove(key)?;
        
        info!("Removed WebSocket {}. Total active: {}", ws_id, websockets.len());
        
        Ok(())
    }
    
    pub fn get_all_websockets(&self) -> Result<HashMap<String, String>, Box<dyn std::error::Error>> {
        let websockets = self.active_websockets.lock().unwrap();
        Ok(websockets.clone())
    }
    
    pub fn prune_stale_websockets(&self) -> Result<usize, Box<dyn std::error::Error>> {
        // This would be called periodically to remove websockets that are no longer active
        // In a real implementation, we'd check against active connections
        // For now, we'll just return 0 as a placeholder
        Ok(0)
    }
    
    // Method to get pending releases that need to be processed
    pub fn get_pending_releases(&self) -> Result<Vec<Release>, Box<dyn std::error::Error>> {
        let now = Utc::now();
        let mut pending = Vec::new();
        
        for release in self.get_all_releases()? {
            if release.status == ReleaseStatus::Pending && release.scheduled_at <= now {
                pending.push(release);
            }
        }
        
        Ok(pending)
    }
}
