mod server;

use actix_web::{web, Error, HttpRequest, HttpResponse};
use actix_web_actors::ws;
use uuid::Uuid;
use log::{info, error};

pub async fn ws_index(
    req: HttpRequest,
    stream: web::Payload,
    db: web::Data<crate::storage::SledStorage>,
) -> Result<HttpResponse, Error> {
    let ws_id = Uuid::new_v4().to_string();
    info!("New WebSocket connection: {}", ws_id);
    
    // Log request headers for debugging
    info!("WebSocket request headers:");
    for (name, value) in req.headers() {
        info!("  {}: {}", name, value.to_str().unwrap_or("Invalid UTF-8"));
    }
    
    // TODO: Get user ID from session
    let user_id = "anonymous".to_string();
    
    // Create new WebSocket session
    let ws = server::WebSocketSession::new(ws_id.clone(), user_id, db.get_ref().clone());
    
    // Start WebSocket handler
    match ws::start(ws, &req, stream) {
        Ok(response) => {
            info!("WebSocket successfully started");
            Ok(response)
        }
        Err(e) => {
            error!("Failed to start WebSocket: {}", e);
            Err(e)
        }
    }
}
