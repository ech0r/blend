use gloo_net::http::Request;
use wasm_bindgen_futures::spawn_local;
use yew::prelude::*;
use crate::models::{Release, Client, User, Environment, ReleaseStatus};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use wasm_bindgen::JsValue;
use web_sys::console;

// Base API URL
const API_URL: &str = "/api";

// Response wrapper types
#[derive(Debug, Deserialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub message: Option<String>,
    pub data: Option<T>,
}

// Request types
#[derive(Debug, Serialize)]
pub struct CreateReleaseRequest {
    pub title: String,
    pub client_id: String,
    pub current_environment: String,
    pub target_environment: String,
    pub deployment_items: Vec<String>,
    pub scheduled_at: DateTime<Utc>,
    pub skip_staging: bool, // Added skip_staging field
}

// Generic API error
#[derive(Debug, Clone)]
pub enum ApiError {
    RequestError(String),
    ParseError(String),
    ApiError(String),
}

impl From<gloo_net::Error> for ApiError {
    fn from(err: gloo_net::Error) -> Self {
        ApiError::RequestError(err.to_string())
    }
}

impl From<serde_json::Error> for ApiError {
    fn from(err: serde_json::Error) -> Self {
        ApiError::ParseError(err.to_string())
    }
}

impl std::fmt::Display for ApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ApiError::RequestError(err) => write!(f, "Request error: {}", err),
            ApiError::ParseError(err) => write!(f, "Parse error: {}", err),
            ApiError::ApiError(err) => write!(f, "API error: {}", err),
        }
    }
}

// API client
pub struct ApiClient;

impl ApiClient {
    // Fetch all releases
    pub async fn get_releases() -> Result<Vec<Release>, ApiError> {
        let url = format!("{}/releases", API_URL);
        let response = Request::get(&url).send().await?;
        
        if !response.ok() {
            return Err(ApiError::ApiError(format!("API error: {}", response.status())));
        }
        
        let releases: Vec<Release> = response.json().await?;
        Ok(releases)
    }
    
    // Fetch a specific release
    pub async fn get_release(id: &str) -> Result<Release, ApiError> {
        let url = format!("{}/releases/{}", API_URL, id);
        let response = Request::get(&url).send().await?;
        
        if !response.ok() {
            return Err(ApiError::ApiError(format!("API error: {}", response.status())));
        }
        
        let response: ApiResponse<Release> = response.json().await?;
        
        match response.data {
            Some(release) => Ok(release),
            None => Err(ApiError::ApiError(response.message.unwrap_or_else(|| 
                "Unknown error fetching release".to_string()))),
        }
    }
    
    // Create a new release
    pub async fn create_release(
        title: String,
        client_id: String,
        current_environment: Environment,
        target_environment: Environment,
        deployment_items: Vec<String>,
        scheduled_at: DateTime<Utc>,
        skip_staging: bool, // Added skip_staging parameter
    ) -> Result<Release, ApiError> {
        let url = format!("{}/releases", API_URL);
        
        let request = CreateReleaseRequest {
            title,
            client_id,
            current_environment: format!("{:?}", current_environment),
            target_environment: format!("{:?}", target_environment),
            deployment_items,
            scheduled_at,
            skip_staging, // Include skip_staging in request
        };
        
        let response = Request::post(&url)
            .json(&request)?
            .send()
            .await?;
            
        if !response.ok() {
            return Err(ApiError::ApiError(format!("API error: {}", response.status())));
        }
        
        let response: ApiResponse<Release> = response.json().await?;
        
        match response.data {
            Some(release) => Ok(release),
            None => Err(ApiError::ApiError(response.message.unwrap_or_else(|| 
                "Unknown error creating release".to_string()))),
        }
    }
    pub async fn update_release_status(
        id: &str, 
        status: &str
    ) -> Result<Release, ApiError> {
        let url = format!("{}/releases/{}/status", API_URL, id);

        let status_update = serde_json::json!({
            "status": status
        });

        let response = Request::put(&url)
            .json(&status_update)?
            .send()
            .await?;

        if !response.ok() {
            return Err(ApiError::ApiError(format!("API error: {}", response.status())));
        }

        let response: ApiResponse<Release> = response.json().await?;

        match response.data {
            Some(release) => Ok(release),
            None => Err(ApiError::ApiError(response.message.unwrap_or_else(|| 
                        "Unknown error updating release status".to_string()))),
        }
    }
    // Update a release
    pub async fn update_release(
        id: &str,
        title: String,
        client_id: String,
        current_environment: Environment,
        target_environment: Environment,
        deployment_items: Vec<String>,
        scheduled_at: DateTime<Utc>,
        skip_staging: bool, // Added skip_staging parameter
    ) -> Result<Release, ApiError> {
        let url = format!("{}/releases/{}", API_URL, id);
        
        let request = CreateReleaseRequest {
            title,
            client_id,
            current_environment: format!("{:?}", current_environment),
            target_environment: format!("{:?}", target_environment),
            deployment_items,
            scheduled_at,
            skip_staging, // Include skip_staging in request
        };
        
        let response = Request::put(&url)
            .json(&request)?
            .send()
            .await?;
            
        if !response.ok() {
            return Err(ApiError::ApiError(format!("API error: {}", response.status())));
        }
        
        let response: ApiResponse<Release> = response.json().await?;
        
        match response.data {
            Some(release) => Ok(release),
            None => Err(ApiError::ApiError(response.message.unwrap_or_else(|| 
                "Unknown error updating release".to_string()))),
        }
    }
    
    // Delete a release
    pub async fn delete_release(id: &str) -> Result<(), ApiError> {
        let url = format!("{}/releases/{}", API_URL, id);
        
        let response = Request::delete(&url)
            .send()
            .await?;
            
        if !response.ok() {
            return Err(ApiError::ApiError(format!("API error: {}", response.status())));
        }
        
        Ok(())
    }
    
    // Fetch all clients
    pub async fn get_clients() -> Result<Vec<Client>, ApiError> {
        let url = format!("{}/clients", API_URL);
        let response = Request::get(&url).send().await?;
        
        if !response.ok() {
            return Err(ApiError::ApiError(format!("API error: {}", response.status())));
        }
        
        let clients: Vec<Client> = response.json().await?;
        Ok(clients)
    }
    
    // Fetch current user info
    pub async fn get_current_user() -> Result<User, ApiError> {
        let url = format!("{}/users/me", API_URL);
        let response = Request::get(&url).send().await?;
        
        if !response.ok() {
            return Err(ApiError::ApiError(format!("API error: {}", response.status())));
        }
        
        let user: User = response.json().await?;
        Ok(user)
    }
}
