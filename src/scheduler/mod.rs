use actix_web::web;
use tokio::time::{interval, Duration};
use std::process::{Stdio};
use tokio::process::Command;
use tokio::io::{BufReader, AsyncBufReadExt};
use log::{info, error, warn};
use crate::models::{Release, ReleaseStatus, DeploymentItem};
use crate::storage::SledStorage;
use chrono::Utc;
use uuid::Uuid;

// Import WebSocket broadcast functionality
use crate::websocket::server::broadcast_release_update;

const CHECK_INTERVAL: Duration = Duration::from_secs(60); // Check every minute

// Start scheduler to check for pending releases
pub fn start_scheduler(db: web::Data<SledStorage>) -> tokio::task::JoinHandle<()> {
    let db = db.clone();
    
    tokio::spawn(async move {
        let mut interval = interval(CHECK_INTERVAL);
        
        loop {
            interval.tick().await;
            
            info!("Checking for pending releases...");
            match check_pending_releases(db.clone()).await {
                Ok(count) => {
                    if count > 0 {
                        info!("Processed {} pending releases", count);
                    }
                }
                Err(e) => {
                    error!("Error checking pending releases: {}", e);
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

// Check for pending releases and process them
async fn check_pending_releases(db: web::Data<SledStorage>) -> Result<usize, Box<dyn std::error::Error + Send + Sync>> {
    let pending_releases = db.get_pending_releases()?;
    let count = pending_releases.len();
    
    if count == 0 {
        return Ok(0);
    }
    
    info!("Found {} pending releases", count);
    
    // Process each pending release
    for mut release in pending_releases {
        info!("Processing release: {}", release.id);
        
        // Update status to in progress
        release.status = ReleaseStatus::InProgress;
        db.save_release(&release)?;
        
        // Send WebSocket update
        broadcast_release_update(
            release.id.to_string(),
            "InProgress".to_string(), 
            0.0, 
            Some("Starting deployment process...".to_string())
        );
        
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
    
    // Update status to in progress
    release.status = ReleaseStatus::InProgress;
    release.progress = 0.0;
    db.save_release(&release)?;
    
    // Broadcast status update
    broadcast_release_update(
        release.id.to_string(),
        "InProgress".to_string(), 
        0.0, 
        Some(format!("Starting deployment process for {}", release.title))
    );
    
    // Process each deployment item
    let total_items = release.deployment_items.len();
    let mut completed_items = 0;
    let mut has_errors = false;
    
    // Process deployment items in parallel
    let mut handles = vec![];
    
    for (i, item) in release.deployment_items.iter_mut().enumerate() {
        let item_name = item.name.clone();
        let release_id_str = release.id.to_string();
        let db_clone = db.clone();
        
        info!("Scheduling deployment item: {}", item_name);
        
        // Update item status
        item.status = ReleaseStatus::InProgress;
        db.save_release(&release)?;
        
        // Update progress and broadcast
        release.progress = (completed_items as f32 / total_items as f32) * 100.0;
        broadcast_release_update(
            release_id_str.clone(),
            "InProgress".to_string(), 
            release.progress, 
            Some(format!("Starting {} deployment", item_name))
        );
        
        // Run in a separate task
        let handle = tokio::spawn(async move {
            let result = process_deployment_item(&item_name, release_id_str).await;
            
            // Get the release again to avoid conflicts
            let db_ref = db_clone.as_ref();
            if let Ok(Some(mut updated_release)) = db_ref.get_release(&Uuid::parse_str(&release_id_str).unwrap()) {
                // Find the right deployment item
                if let Some(deployment_item) = updated_release.deployment_items.iter_mut().find(|it| it.name == item_name) {
                    if let Err(e) = &result {
                        deployment_item.status = ReleaseStatus::Failed;
                        deployment_item.error = Some(e.to_string());
                    } else {
                        deployment_item.status = ReleaseStatus::Completed;
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
            has_errors = true;
        }
    }
    
    // Get the most up-to-date release state
    let mut release = match db.get_release(&release_id)? {
        Some(release) => release,
        None => {
            error!("Release {} not found after deployment", release_id);
            return Err("Release not found after deployment".into());
        }
    };
    
    // Check all deployment items for errors
    has_errors = release.deployment_items.iter().any(|item| item.status == ReleaseStatus::Failed);
    
    // Update overall status
    if has_errors {
        release.status = ReleaseStatus::Failed;
    } else {
        release.status = ReleaseStatus::Completed;
        release.progress = 100.0;
    }
    
    // Save final status
    db.save_release(&release)?;
    
    // Broadcast final status
    broadcast_release_update(
        release.id.to_string(),
        release.status.to_string(), 
        release.progress, 
        Some(format!("Deployment process complete for {}", release.title))
    );
    
    info!("Release process completed for {}: {}", release.id, release.title);
    
    Ok(())
}

// Process a single deployment item
async fn process_deployment_item(item_name: &str, release_id: String) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Create a real deployment process based on item type
    let command = match item_name {
        "data" => format!("for i in 1..10; do echo \"Deploying data step $i\"; sleep 3; done"),
        "solr" => format!("for i in 1..6; do echo \"Rebuilding Solr index step $i\"; sleep 2; done"),
        "app" => format!("for i in 1..8; do echo \"Deploying application step $i\"; sleep 4; done"),
        _ => format!("echo \"Unknown deployment type: {}\"; exit 1", item_name),
    };
    
    info!("Starting {} deployment with command: {}", item_name, command);
    
    // Create the command using bash
    let mut child = Command::new("bash")
        .arg("-c")
        .arg(&command)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("Failed to start {} process: {}", item_name, e))?;
        
    // Stream stdout
    if let Some(stdout) = child.stdout.take() {
        let mut reader = BufReader::new(stdout).lines();
        let item_name = item_name.to_string();
        let release_id = release_id.clone();
        
        tokio::spawn(async move {
            let mut line_count = 0;
            while let Ok(Some(line)) = reader.next_line().await {
                line_count += 1;
                info!("[{}] {}", item_name, line);
                
                // Estimate progress based on total expected lines
                let total_lines = match item_name.as_str() {
                    "data" => 10.0,
                    "solr" => 6.0,
                    "app" => 8.0,
                    _ => 1.0,
                };
                let progress = (line_count as f32 / total_lines) * 100.0;
                
                // Broadcast progress update and line
                broadcast_release_update(
                    release_id.clone(),
                    "InProgress".to_string(), 
                    progress, 
                    Some(format!("[{}] {}", item_name, line))
                );
                
                // Short delay to simulate processing
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
        });
    }
    
    // Stream stderr
    if let Some(stderr) = child.stderr.take() {
        let mut reader = BufReader::new(stderr).lines();
        let item_name = item_name.to_string();
        let release_id = release_id.clone();
        
        tokio::spawn(async move {
            while let Ok(Some(line)) = reader.next_line().await {
                warn!("[{}] ERROR: {}", item_name, line);
                
                // Broadcast error
                broadcast_release_update(
                    release_id.clone(),
                    "InProgress".to_string(), 
                    0.0, 
                    Some(format!("[{}] ERROR: {}", item_name, line))
                );
                
                // Short delay to simulate processing
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
        });
    }
    
    // Wait for process to complete
    let status = child.wait().await
        .map_err(|e| format!("Failed to wait for {} process: {}", item_name, e))?;
        
    if !status.success() {
        let error_message = format!("{} deployment failed with exit code: {}", item_name, status);
        broadcast_release_update(
            release_id.clone(),
            "Failed".to_string(), 
            0.0, 
            Some(error_message.clone())
        );
        return Err(error_message.into());
    }
    
    // Announce completion
    broadcast_release_update(
        release_id.clone(),
        "Completed".to_string(), 
        100.0, 
        Some(format!("{} deployment completed successfully", item_name))
    );
    
    Ok(())
}
