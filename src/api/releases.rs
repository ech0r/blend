use actix_web::{web, HttpResponse, Responder, get, post, put, delete};
use crate::models::{Release, Environment, ReleaseStatus};
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
) -> Result<bool, Box<dyn std::error::Error>> {
    let releases = db.get_all_releases()?;
    
    Ok(releases.iter().any(|r| {
        r.client_id == client_id 
        && (r.status == ReleaseStatus::Pending || r.status == ReleaseStatus::InProgress)
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
    match (&current_env, &target_env) {
        (Environment::Development, Environment::Staging) => {}
        (Environment::Development, Environment::Production) => {}
        (Environment::Staging, Environment::Production) => {}
        _ => {
            return HttpResponse::BadRequest().json(ReleaseResponse {
                success: false,
                message: Some(format!(
                    "Invalid deployment path: {:?} to {:?}",
                    current_env, target_env
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
    );

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
        current_environment: current_env, // Note: we should update current_environment, not just target
        target_environment: target_env,
        scheduled_at: release_data.scheduled_at,
        // Keep original values for these fields
        created_at: existing_release.created_at,
        created_by: existing_release.created_by,
        status: ReleaseStatus::Pending, // Reset status to Pending for the new environment
        progress: 0.0, // Reset progress
        // Update deployment items if provided, otherwise keep original
        deployment_items: if release_data.deployment_items.is_empty() {
            existing_release.deployment_items.iter().map(|item| {
                // Reset status for each item
                DeploymentItem {
                    name: item.name.clone(),
                    status: ReleaseStatus::Pending,
                    logs: Vec::new(), // Clear logs for new environment
                    error: None,
                }
            }).collect()
        } else {
            release_data.deployment_items.iter().map(|name| {
                DeploymentItem {
                    name: name.clone(),
                    status: ReleaseStatus::Pending,
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

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(get_releases)
        .service(get_release)
        .service(create_release)
        .service(update_release)
        .service(delete_release);
}
