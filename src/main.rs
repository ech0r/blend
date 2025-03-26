use actix_web::{web, App, HttpServer, middleware::Logger};
use actix_files as fs;
use dotenv::dotenv;
use log::info;
use std::env;

mod api;
mod auth;
mod models;
mod storage;
mod websocket;
mod scheduler;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // Initialize environment
    dotenv().ok();
    env_logger::init();
    
    // Print banner to confirm logger is working
    info!("============================================");
    info!("BLEND RELEASE MANAGER STARTING");
    info!("============================================");
    
    // Get config from environment
    let host = env::var("HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
    let port = env::var("PORT").unwrap_or_else(|_| "8080".to_string());
    let server_url = format!("{}:{}", host, port);
    
    // Initialize storage
    let db = storage::init().expect("Failed to initialize storage");
    let db_data = web::Data::new(db);
    
    // Start scheduler
    let scheduler_db = db_data.clone();
    let _scheduler = scheduler::start_scheduler(scheduler_db);
    
    // Send an application log
    websocket::server::broadcast_app_log("info", "Blend Release Manager starting up");
    
    info!("Starting server at http://{}", server_url);
    info!("Static files directory: {}", std::env::current_dir().unwrap_or_default().join("static").display());
    
    // Send app logs about important system info
    let rust_log = env::var("RUST_LOG").unwrap_or_else(|_| "not set".to_string());
    websocket::server::broadcast_app_log("info", &format!("Server starting at http://{}", server_url));
    websocket::server::broadcast_app_log("info", &format!("Static files directory: {}", 
        std::env::current_dir().unwrap_or_default().join("static").display()));
    websocket::server::broadcast_app_log("info", &format!("RUST_LOG environment variable: {}", rust_log));
    
    // Start HTTP server
    HttpServer::new(move || {
        App::new()
            .wrap(Logger::default())
            .app_data(db_data.clone())
            // API routes
            .service(web::scope("/api")
                .configure(api::configure))
            // Auth routes
            .service(web::scope("/auth")
                .configure(auth::configure))
            // WebSocket route
            .service(web::resource("/ws").route(web::get().to(websocket::ws_index)))
            // Static files (compiled frontend)
            .service(fs::Files::new("/", "./static").index_file("index.html"))
    })
    .bind(server_url)?
    .run()
    .await
}
