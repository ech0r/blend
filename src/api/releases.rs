use actix_web::{web, HttpResponse, Responder, get, post, put, delete};
use crate::models::{Release, Environment, ReleaseStatus, DeploymentItem};
use crate::storage::SledStorage;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};
use log::{info, error};

#[derive(Debug, Deserialize)]
pub struct CreateReleaseRequest {
    pub title: String,
    pub client_id: String,
    pub current_environment: String,
    pub target_environment: String,
    pub deployment_items: Vec<String>,
    pub scheduled_at: DateTime<Utc>,
    #[serde(default)]
    pub skip_staging: bool, // Added skip_staging field
}

#[derive(Debug, Serialize)]
pub struct ReleaseResponse {
    pub success: bool,
    pub message: Option<String>,
    pub data: Option<Release>,
}

// Convert string to Environment enum
fn parse_environment(env_str: &str) -> Result<Environment, String> {
    match env_str.to_lowercase().as_str() {
        "development" => Ok(Environment::Development),
        "staging" => Ok(Environment::Staging),
        "production" => Ok(Environment::Production),
        _ => Err(format!("Invalid environment: {}", env_str)),
    }
}

// Check if client already has an active release in the pipeline
async fn check_client_release_exists(
    db: &SledStorage,
    client_id: &str,
) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
    let releases = db.get_all_releases()?;
    
    Ok(releases.iter().any(|r| {
        r.client_id == client_id 
        && (r.status == ReleaseStatus::InDevelopment || 
            r.status == ReleaseStatus::DeployingToStaging || 
            r.status == ReleaseStatus::ReadyToTestInStaging || 
            r.status == ReleaseStatus::DeployingToProduction || 
            r.status == ReleaseStatus::ReadyToTestInProduction)
    }))
}

#[get("")]
async fn get_releases(db: web::Data<SledStorage>) -> impl Responder {
    match db.get_all_releases() {
        Ok(releases) => HttpResponse::Ok().json(releases),
        Err(e) => {
            error!("Failed to get releases: {}", e);
            HttpResponse::InternalServerError().json(ReleaseResponse {
                success: false,
                message: Some(format!("Failed to get releases: {}", e)),
                data: None,
            })
        }
    }
}

#[get("/{id}")]
async fn get_release(db: web::Data<SledStorage>, path: web::Path<Uuid>) -> impl Responder {
    let release_id = path.into_inner();
    
    match db.get_release(&release_id) {
        Ok(Some(release)) => HttpResponse::Ok().json(release),
        Ok(None) => HttpResponse::NotFound().json(ReleaseResponse {
            success: false,
            message: Some(format!("Release with ID {} not found", release_id)),
            data: None,
        }),
        Err(e) => {
            error!("Failed to get release {}: {}", release_id, e);
            HttpResponse::InternalServerError().json(ReleaseResponse {
                success: false,
                message: Some(format!("Failed to get release: {}", e)),
                data: None,
            })
        }
    }
}

#[post("")]
async fn create_release(
    db: web::Data<SledStorage>,
    release_data: web::Json<CreateReleaseRequest>,
) -> impl Responder {
    // Parse environments
    let current_env = match parse_environment(&release_data.current_environment) {
        Ok(env) => env,
        Err(e) => {
            return HttpResponse::BadRequest().json(ReleaseResponse {
                success: false,
                message: Some(e),
                data: None,
            });
        }
    };
    
    let target_env = match parse_environment(&release_data.target_environment) {
        Ok(env) => env,
        Err(e) => {
            return HttpResponse::BadRequest().json(ReleaseResponse {
                success: false,
                message: Some(e),
                data: None,
            });
        }
    };

    // Check valid deployment path
    match (&current_env, &target_env, release_data.skip_staging) {
        // Allow direct path if skip_staging is true
        (Environment::Development, Environment::Production, true) => {}
        // Standard paths
        (Environment::Development, Environment::Staging, _) => {}
        (Environment::Development, Environment::Production, false) => {}
        (Environment::Staging, Environment::Production, _) => {}
        _ => {
            return HttpResponse::BadRequest().json(ReleaseResponse {
                success: false,
                message: Some(format!(
                    "Invalid deployment path: {:?} to {:?}{}",
                    current_env, 
                    target_env,
                    if release_data.skip_staging { " (skip staging)" } else { "" }
                )),
                data: None,
            });
        }
    }

    // Check if client already has an active release
    match check_client_release_exists(&db, &release_data.client_id).await {
        Ok(true) => {
            return HttpResponse::Conflict().json(ReleaseResponse {
                success: false,
                message: Some(format!(
                    "Client {} already has an active release in progress",
                    release_data.client_id
                )),
                data: None,
            });
        }
        Err(e) => {
            error!("Failed to check client releases: {}", e);
            return HttpResponse::InternalServerError().json(ReleaseResponse {
                success: false,
                message: Some(format!("Failed to check client releases: {}", e)),
                data: None,
            });
        }
        _ => {}
    }

    // Create new release
    let release = Release::new(
        release_data.title.clone(),
        release_data.client_id.clone(),
        current_env,
        target_env,
        release_data.deployment_items.clone(),
        release_data.scheduled_at,
        "unknown".to_string(), // TODO: Get from authenticated user
        release_data.skip_staging, // Pass the skip_staging flag
    );

    info!("NEW RELEASE SCHEDULED AT: {}", release_data.scheduled_at);

    // Save to storage
    match db.save_release(&release) {
        Ok(_) => {
            info!("Created new release: {}", release.id);
            HttpResponse::Created().json(ReleaseResponse {
                success: true,
                message: Some(format!("Release created successfully")),
                data: Some(release),
            })
        }
        Err(e) => {
            error!("Failed to save release: {}", e);
            HttpResponse::InternalServerError().json(ReleaseResponse {
                success: false,
                message: Some(format!("Failed to save release: {}", e)),
                data: None,
            })
        }
    }
}

#[put("/{id}")]
async fn update_release(
    db: web::Data<SledStorage>,
    path: web::Path<Uuid>,
    release_data: web::Json<CreateReleaseRequest>,
) -> impl Responder {
    let release_id = path.into_inner();
    
    // Check if release exists
    let existing_release = match db.get_release(&release_id) {
        Ok(Some(release)) => release,
        Ok(None) => {
            return HttpResponse::NotFound().json(ReleaseResponse {
                success: false,
                message: Some(format!("Release with ID {} not found", release_id)),
                data: None,
            });
        }
        Err(e) => {
            error!("Failed to get release {}: {}", release_id, e);
            return HttpResponse::InternalServerError().json(ReleaseResponse {
                success: false,
                message: Some(format!("Failed to get release: {}", e)),
                data: None,
            });
        }
    };
    
    // Parse environments
    let current_env = match parse_environment(&release_data.current_environment) {
        Ok(env) => env,
        Err(e) => {
            return HttpResponse::BadRequest().json(ReleaseResponse {
                success: false,
                message: Some(e),
                data: None,
            });
        }
    };
    
    let target_env = match parse_environment(&release_data.target_environment) {
        Ok(env) => env,
        Err(e) => {
            return HttpResponse::BadRequest().json(ReleaseResponse {
                success: false,
                message: Some(e),
                data: None,
            });
        }
    };
    
    // Create updated release
    let updated_release = Release {
        id: release_id,
        title: release_data.title.clone(),
        client_id: release_data.client_id.clone(),
        current_environment: current_env,
        target_environment: target_env,
        scheduled_at: release_data.scheduled_at,
        // Keep original values for these fields
        created_at: existing_release.created_at,
        created_by: existing_release.created_by,
        status: existing_release.status, // Keep the current status
        progress: existing_release.progress, // Keep the current progress
        skip_staging: release_data.skip_staging, // Update the skip_staging flag
        // Update deployment items if provided, otherwise keep original
        deployment_items: if release_data.deployment_items.is_empty() {
            existing_release.deployment_items
        } else {
            release_data.deployment_items.iter().map(|name| {
                DeploymentItem {
                    name: name.clone(),
                    status: ReleaseStatus::InDevelopment, // Reset status for new items
                    logs: Vec::new(),
                    error: None,
                }
            }).collect()
        },
    };
    
    // Save to storage
    match db.save_release(&updated_release) {
        Ok(_) => {
            info!("Updated release: {}", release_id);
            HttpResponse::Ok().json(ReleaseResponse {
                success: true,
                message: Some(format!("Release updated successfully")),
                data: Some(updated_release),
            })
        }
        Err(e) => {
            error!("Failed to update release {}: {}", release_id, e);
            HttpResponse::InternalServerError().json(ReleaseResponse {
                success: false,
                message: Some(format!("Failed to update release: {}", e)),
                data: None,
            })
        }
    }
}

#[delete("/{id}")]
async fn delete_release(db: web::Data<SledStorage>, path: web::Path<Uuid>) -> impl Responder {
    let release_id = path.into_inner();
    
    // Check if release exists first
    match db.get_release(&release_id) {
        Ok(Some(_)) => {},
        Ok(None) => {
            return HttpResponse::NotFound().json(ReleaseResponse {
                success: false,
                message: Some(format!("Release with ID {} not found", release_id)),
                data: None,
            });
        }
        Err(e) => {
            error!("Failed to get release {}: {}", release_id, e);
            return HttpResponse::InternalServerError().json(ReleaseResponse {
                success: false,
                message: Some(format!("Failed to get release: {}", e)),
                data: None,
            });
        }
    };
    
    // Delete release
    match db.delete_release(&release_id) {
        Ok(_) => {
            info!("Deleted release: {}", release_id);
            HttpResponse::Ok().json(ReleaseResponse {
                success: true,
                message: Some(format!("Release deleted successfully")),
                data: None,
            })
        }
        Err(e) => {
            error!("Failed to delete release {}: {}", release_id, e);
            HttpResponse::InternalServerError().json(ReleaseResponse {
                success: false,
                message: Some(format!("Failed to delete release: {}", e)),
                data: None,
            })
        }
    }
}

// Add this endpoint to update release status
#[put("/{id}/status")]
async fn update_release_status(
    db: web::Data<SledStorage>,
    path: web::Path<Uuid>,
    status_update: web::Json<serde_json::Value>,
) -> impl Responder {
    let release_id = path.into_inner();
    
    // Get the release
    let mut release = match db.get_release(&release_id) {
        Ok(Some(release)) => release,
        Ok(None) => {
            return HttpResponse::NotFound().json(ReleaseResponse {
                success: false,
                message: Some(format!("Release with ID {} not found", release_id)),
                data: None,
            });
        }
        Err(e) => {
            error!("Failed to get release {}: {}", release_id, e);
            return HttpResponse::InternalServerError().json(ReleaseResponse {
                success: false,
                message: Some(format!("Failed to get release: {}", e)),
                data: None,
            });
        }
    };
    
    // Get the new status
    let status_str = match status_update.get("status") {
        Some(serde_json::Value::String(s)) => s.as_str(),
        _ => {
            return HttpResponse::BadRequest().json(ReleaseResponse {
                success: false,
                message: Some("Invalid or missing status in request".to_string()),
                data: None,
            });
        }
    };
    
    // Get the next status based on current and skip_staging flag
    if status_str == "clear" {
        // Use release.next_status_when_cleared() instead of calling it on the status
        if let Some(next_status) = release.next_status_when_cleared() {
            release.status = next_status;
        } else {
            return HttpResponse::BadRequest().json(ReleaseResponse {
                success: false,
                message: Some("Release cannot be cleared in its current state".to_string()),
                data: None,
            });
        }
    } else {
        // For direct status updates (less common)
        release.status = match status_str {
            "InDevelopment" => ReleaseStatus::InDevelopment,
            "ClearedInDevelopment" => ReleaseStatus::ClearedInDevelopment,
            "DeployingToStaging" => ReleaseStatus::DeployingToStaging,
            "ReadyToTestInStaging" => ReleaseStatus::ReadyToTestInStaging,
            "ClearedInStaging" => ReleaseStatus::ClearedInStaging,
            "DeployingToProduction" => ReleaseStatus::DeployingToProduction,
            "ReadyToTestInProduction" => ReleaseStatus::ReadyToTestInProduction,
            "ClearedInProduction" => ReleaseStatus::ClearedInProduction,
            "Error" => ReleaseStatus::Error,
            "Blocked" => ReleaseStatus::Blocked,
            _ => {
                return HttpResponse::BadRequest().json(ReleaseResponse {
                    success: false,
                    message: Some(format!("Invalid status: {}", status_str)),
                    data: None,
                });
            }
        };
    }

    // Save the updated release
    match db.save_release(&release) {
        Ok(_) => {
            info!("Updated release status: {} to {:?}", release_id, release.status);
            HttpResponse::Ok().json(ReleaseResponse {
                success: true,
                message: Some(format!("Release status updated successfully")),
                data: Some(release),
            })
        }
        Err(e) => {
            error!("Failed to update release status {}: {}", release_id, e);
            HttpResponse::InternalServerError().json(ReleaseResponse {
                success: false,
                message: Some(format!("Failed to update release status: {}", e)),
                data: None,
            })
        }
    }
}

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(get_releases)
        .service(get_release)
        .service(create_release)
        .service(update_release)
        .service(delete_release)
        .service(update_release_status);
}
