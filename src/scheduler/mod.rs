use actix_web::web;
use tokio::time::{interval, Duration};
use std::process::{Stdio};
use tokio::process::Command;
use tokio::io::{BufReader, AsyncBufReadExt};
use log::{info, error, warn};
use std::path::Path;
use regex::Regex;
use crate::models::{Release, ReleaseStatus, DeploymentItem};
use crate::storage::SledStorage;
use chrono::Utc;

// Import WebSocket broadcast functionality
use crate::websocket::server::broadcast_release_update;

const CHECK_INTERVAL: Duration = Duration::from_secs(60); // Check every minute

// Define script paths
const SCRIPTS_DIR: &str = "scripts";
const DATA_SCRIPT: &str = "deploy_data.sh";
const SOLR_SCRIPT: &str = "deploy_solr.sh";
const APP_SCRIPT: &str = "deploy_app.sh";

// Define progress pattern for parsing script output
lazy_static::lazy_static! {
    static ref PROGRESS_PATTERN: Regex = Regex::new(r"\[PROGRESS:([a-z]+):(\d+)\]").unwrap();
}

// Start scheduler to check for pending releases
pub fn start_scheduler(db: web::Data<SledStorage>) -> tokio::task::JoinHandle<()> {
    let db = db.clone();
    
    tokio::spawn(async move {
        let mut interval = interval(CHECK_INTERVAL);
        
        loop {
            interval.tick().await;
            
            info!("Checking for releases to process...");
            match check_releases_to_process(db.clone()).await {
                Ok(count) => {
                    if count > 0 {
                        info!("Processed {} releases", count);
                    }
                }
                Err(e) => {
                    error!("Error checking releases to process: {}", e);
                }
            }
            
            // Also periodically prune stale WebSocket connections
            match db.prune_stale_websockets() {
                Ok(count) => {
                    if count > 0 {
                        info!("Pruned {} stale WebSocket connections", count);
                    }
                }
                Err(e) => {
                    error!("Error pruning stale WebSocket connections: {}", e);
                }
            }
        }
    })
}

// Check for releases that need to be processed
async fn check_releases_to_process(db: web::Data<SledStorage>) -> Result<usize, Box<dyn std::error::Error + Send + Sync>> {
    // Get releases that need processing (those in a "Deploying" state)
    let releases_to_process = db.get_releases_to_process()?;
    let count = releases_to_process.len();
    
    if count == 0 {
        return Ok(0);
    }
    
    info!("Found {} releases to process", count);
    
    // Process each release
    for release in releases_to_process {
        info!("Processing release: {}", release.id);
        
        // Clone release for async processing
        let release_id = release.id;
        let db_clone = db.clone();
        
        // Process release in a separate task
        tokio::spawn(async move {
            if let Err(e) = process_release(release_id, db_clone).await {
                error!("Error processing release {}: {}", release_id, e);
            }
        });
    }
    
    Ok(count)
}

// Process a single release
async fn process_release(release_id: uuid::Uuid, db: web::Data<SledStorage>) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Get release
    let mut release = match db.get_release(&release_id)? {
        Some(release) => release,
        None => {
            error!("Release {} not found", release_id);
            return Err("Release not found".into());
        }
    };
    
    info!("Starting release process for {}: {}", release.id, release.title);
    
    // Determine current environment from the release status
    let env_name = match release.status {
        ReleaseStatus::DeployingToStaging => "staging",
        ReleaseStatus::DeployingToProduction => "production",
        _ => {
            error!("Release {} has invalid status for processing: {:?}", release_id, release.status);
            return Err(format!("Invalid status for processing: {:?}", release.status).into());
        }
    };
    
    // Update progress
    release.progress = 0.0;
    db.save_release(&release)?;
    
    // Broadcast status update
    broadcast_release_update(
        release.id.to_string(),
        format!("{:?}", release.status), 
        0.0, 
        Some(format!("Starting deployment process for {}", release.title))
    );
    
    // Process each deployment item in parallel
    let mut handles = vec![];
    
    // First save the release status for all items 
    for item in release.deployment_items.iter_mut() {
        // Set all items to the same status as the release
        item.status = release.status.clone();
    }
    db.save_release(&release)?;
    
    // Then process each item
    for item in release.deployment_items.iter() {
        let item_name = item.name.clone();
        let release_id_str = release.id.to_string();
        let db_clone = db.clone();
        let env_name = env_name.to_string();
        
        // Broadcast final status with a clear status name
        let status_str = match release.status {
            ReleaseStatus::ReadyToTestInStaging => "ReadyToTestInStaging".to_string(),
            ReleaseStatus::ReadyToTestInProduction => "ReadyToTestInProduction".to_string(),
            ReleaseStatus::Error => "Error".to_string(),
            _ => format!("{:?}", release.status)
        };

        broadcast_release_update(
            release.id.to_string(),
            status_str, 
            release.progress, 
            Some(format!("Deployment process complete for {}", release.title))
        );

        info!("Release process completed for {}: {}", release.id, release.title);
        
        // Run in a separate task
        let handle = tokio::spawn(async move {
            let result = process_deployment_item(&item_name, &env_name, release_id_str.clone()).await;
            
            // Get the release again to avoid conflicts
            let db_ref = db_clone.as_ref();
            if let Ok(Some(mut updated_release)) = db_ref.get_release(&release_id) {
                // Find the right deployment item
                if let Some(deployment_item) = updated_release.deployment_items.iter_mut().find(|it| it.name == item_name) {
                    if let Err(e) = &result {
                        deployment_item.status = ReleaseStatus::Error;
                        deployment_item.error = Some(e.to_string());
                    } else {
                        // Set next status based on current status
                        deployment_item.status = match updated_release.status {
                            ReleaseStatus::DeployingToStaging => ReleaseStatus::ReadyToTestInStaging,
                            ReleaseStatus::DeployingToProduction => ReleaseStatus::ReadyToTestInProduction,
                            _ => deployment_item.status.clone(),
                        };
                    }
                    
                    // Save updated deployment item
                    if let Err(e) = db_ref.save_release(&updated_release) {
                        error!("Failed to save release after deployment item completion: {}", e);
                    }
                }
            }
            
            result
        });
        
        handles.push(handle);
    }
    
    // Wait for all deployment items to complete
    for handle in handles {
        if let Err(e) = handle.await {
            error!("Error joining task: {}", e);
        }
    }
    
    // Get the most up-to-date release state after all tasks complete
    let mut release = match db.get_release(&release_id)? {
        Some(release) => release,
        None => {
            error!("Release {} not found after deployment", release_id);
            return Err("Release not found after deployment".into());
        }
    };

    // Check if all deployment items have completed
    let all_completed = release.deployment_items.iter().all(|item| {
        match item.status {
            ReleaseStatus::ReadyToTestInStaging | 
            ReleaseStatus::ReadyToTestInProduction | 
            ReleaseStatus::Error => true,
            _ => false
        }
    });

    if !all_completed {
        info!("Not all deployment items for release {} have completed yet, waiting...", release_id);
        return Ok(());
    }

    // Check all deployment items for errors
    let has_errors = release.deployment_items.iter().any(|item| matches!(item.status, ReleaseStatus::Error));

    // Update overall status
    if has_errors {
        info!("Release {} has deployment errors, setting status to Error", release_id);
        release.status = ReleaseStatus::Error;
        release.progress = 0.0;
    } else {
        // Set next status based on current status
        release.status = match release.status {
            ReleaseStatus::DeployingToStaging => ReleaseStatus::ReadyToTestInStaging,
            ReleaseStatus::DeployingToProduction => ReleaseStatus::ReadyToTestInProduction,
            _ => release.status.clone(),
        };
        release.progress = 100.0;
        info!("All deployment items for release {} completed successfully, status set to {:?}", 
               release_id, release.status);
    }

    // Save final status
    db.save_release(&release)?;

    // Broadcast final status
    let status_str = match release.status {
        ReleaseStatus::ReadyToTestInStaging => "ReadyToTestInStaging".to_string(),
        ReleaseStatus::ReadyToTestInProduction => "ReadyToTestInProduction".to_string(),
        ReleaseStatus::Error => "Error".to_string(),
        _ => format!("{:?}", release.status)
    };

    let completion_message = if has_errors {
        format!("Deployment failed for {}. Check details for more information.", release.title)
    } else {
        format!("Deployment process complete for {}", release.title)
    };

    broadcast_release_update(
        release.id.to_string(),
        status_str, 
        release.progress, 
        Some(completion_message)
    );
    info!("Release process completed for {}: {}", release.id, release.title);
    Ok(())
}

// Process a single deployment item using the appropriate script
async fn process_deployment_item(item_name: &str, env_name: &str, release_id: String) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Determine which script to run based on item type
    let script_path = match item_name {
        "data" => Path::new(SCRIPTS_DIR).join(DATA_SCRIPT),
        "solr" => Path::new(SCRIPTS_DIR).join(SOLR_SCRIPT),
        "app" => Path::new(SCRIPTS_DIR).join(APP_SCRIPT),
        _ => return Err(format!("Unknown deployment item type: {}", item_name).into()),
    };
    
    // Check if script exists
    if !script_path.exists() {
        return Err(format!("Script not found at path: {:?}", script_path).into());
    }
    
    info!("Running script for {} deployment in {}: {:?}", item_name, env_name, script_path);
    
    // Create command with the script and environment parameter
    let mut child = Command::new(&script_path)
        .arg(env_name)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("Failed to start {} script: {}", item_name, e))?;
        
    // Initialize progress tracking
    let mut current_progress = 0.0;
    
    // Stream stdout
    if let Some(stdout) = child.stdout.take() {
        let mut reader = BufReader::new(stdout).lines();
        let item_name = item_name.to_string();
        let release_id = release_id.clone();
        
        tokio::spawn(async move {
            while let Ok(Some(line)) = reader.next_line().await {
                // Log the output line
                info!("{}", line);
                
                // Check if this is a progress line
                if let Some(captures) = PROGRESS_PATTERN.captures(&line) {
                    if let Some(progress_str) = captures.get(2) {
                        if let Ok(progress) = progress_str.as_str().parse::<f32>() {
                            current_progress = progress;
                            
                            // Broadcast progress update
                            broadcast_release_update(
                                release_id.clone(),
                                "InProgress".to_string(), 
                                progress, 
                                None  // No log line for progress updates
                            );
                        }
                    }
                } else {
                    // It's a regular log line, broadcast it
                    broadcast_release_update(
                        release_id.clone(),
                        "InProgress".to_string(), 
                        current_progress, 
                        Some(line)
                    );
                }
                
                // Short delay to prevent flooding
                tokio::time::sleep(Duration::from_millis(50)).await;
            }
        });
    }
    
    // Stream stderr (these will be treated as errors)
    if let Some(stderr) = child.stderr.take() {
        let mut reader = BufReader::new(stderr).lines();
        let item_name = item_name.to_string();
        let release_id = release_id.clone();
        
        tokio::spawn(async move {
            while let Ok(Some(line)) = reader.next_line().await {
                // Log the error
                warn!("STDERR: {}", line);
                
                // Mark line as coming from stderr and broadcast
                let err_line = format!("[stderr] {}", line);
                broadcast_release_update(
                    release_id.clone(),
                    "Error".to_string(), 
                    current_progress, 
                    Some(err_line)
                );
                
                // Short delay to prevent flooding
                tokio::time::sleep(Duration::from_millis(50)).await;
            }
        });
    }
    
    // After getting the output status:
    let status = child.wait().await
        .map_err(|e| format!("Failed to wait for {} process: {}", item_name, e))?;
        
    if !status.success() {
        let exit_code = status.code().unwrap_or(-1);
        let error_message = format!("{} deployment failed with exit code: {}", 
            item_name, exit_code);
            
        // Log the error
        error!("{}", error_message);
        
        // Send detailed error information
        broadcast_release_update(
            release_id.clone(),
            "Error".to_string(), 
            0.0, 
            Some(format!("[ERROR] {} - {}", item_name, error_message))
        );
        
        // Also send a more specific error message to help with debugging
        let detailed_error = format!("Item [{}] failed. Exit code: {}. Check logs for details.", 
            item_name, exit_code);
        
        broadcast_release_update(
            release_id.clone(),
            "Error".to_string(), 
            0.0, 
            Some(detailed_error)
        );
        
        return Err(error_message.into());
    }

    // Announce completion with the correct status for the frontend to understand
    let status_str = match env_name {
        "staging" => "ReadyToTestInStaging",
        "production" => "ReadyToTestInProduction",
        _ => "Completed",
    };

    broadcast_release_update(
        release_id.clone(),
        status_str.to_string(), 
        100.0, 
        Some(format!("{} deployment to {} completed successfully", item_name, env_name))
    );
    
    Ok(())
}
