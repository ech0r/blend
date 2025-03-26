use actix_web_actors::ws;
use serde::{Serialize, Deserialize};
use std::time::{Duration, Instant};
use log::{info, warn, error, debug};
use crate::storage::SledStorage;
use chrono::Utc;
use actix::prelude::*;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

// Active WebSocket sessions registry
lazy_static::lazy_static! {
    // Maps session ID to an actor address
    static ref ACTIVE_SESSIONS: Arc<Mutex<HashMap<String, Addr<WebSocketSession>>>> = 
        Arc::new(Mutex::new(HashMap::new()));
}

// How often heartbeat pings are sent
const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(1);
// How long before lack of client response causes a timeout
const CLIENT_TIMEOUT: Duration = Duration::from_secs(10);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub username: String,
    pub message: String,
    pub timestamp: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum WsMessage {
    Chat {
        username: String,
        message: String,
        timestamp: String,
    },
    ReleaseUpdate {
        release_id: String,
        status: String,
        progress: f32,
        log_line: Option<String>,
    },
    AppLog {
        level: String,      // "info", "warn", "error"
        message: String,
        timestamp: String,
    },
}

// Message struct for actor communication
#[derive(Message)]
#[rtype(result = "()")]
struct BroadcastMessage {
    content: String,
    sender_id: String,
}

pub struct WebSocketSession {
    // Unique ID for this session
    id: String,
    // Heartbeat tracking
    hb: Instant,
    // Reference to storage
    db: SledStorage,
    // User information
    user_id: String,
}

impl WebSocketSession {
    pub fn new(id: String, user_id: String, db: SledStorage) -> Self {
        Self {
            id,
            hb: Instant::now(),
            db,
            user_id,
        }
    }

    // Heartbeat to keep connection alive
    fn heartbeat(&self, ctx: &mut ws::WebsocketContext<Self>) {
        ctx.run_interval(HEARTBEAT_INTERVAL, |act, ctx| {
            // Check if client hasn't responded
            if Instant::now().duration_since(act.hb) > CLIENT_TIMEOUT {
                info!("WebSocket client timed out: {}", act.id);
                
                // Try to remove from active connections
                if let Err(e) = act.db.remove_websocket(&act.id) {
                    error!("Failed to remove WebSocket connection: {}", e);
                }
                
                // Remove from active sessions
                if let Ok(mut sessions) = ACTIVE_SESSIONS.lock() {
                    sessions.remove(&act.id);
                }
                
                // Stop actor
                ctx.stop();
                return;
            }
            
            // Send ping
            ctx.ping(b"");
        });
    }
    
    // Broadcast a message to all active clients except the sender
    fn broadcast_message(&self, message: &WsMessage) {
        // Create JSON string for broadcasting
        if let Ok(json) = serde_json::to_string(message) {
            debug!("Broadcasting message: {}", json);
            
            // Send message to all active sessions except self
            if let Ok(sessions) = ACTIVE_SESSIONS.lock() {
                let client_count = sessions.len();
                if client_count <= 1 {
                    debug!("No other clients to broadcast to");
                    return;
                }
                
                let mut broadcast_count = 0;
                // Send to all sessions except the sender
                for (session_id, addr) in sessions.iter() {
                    if *session_id != self.id {  // Don't send to self
                        // Send the message directly to the actor's mailbox
                        let msg = BroadcastMessage {
                            content: json.clone(),
                            sender_id: self.id.clone(),
                        };
                        
                        // do_send doesn't return a Result, it's fire-and-forget
                        addr.do_send(msg);
                        broadcast_count += 1;
                    }
                }
                
                debug!("Message broadcast directly to {} clients", broadcast_count);
            }
        }
    }
}

impl Actor for WebSocketSession {
    type Context = ws::WebsocketContext<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        self.heartbeat(ctx);
        
        // Register actor address in active sessions immediately
        // This is the important part - we want to make the WebSocket usable immediately
        if let Ok(mut sessions) = ACTIVE_SESSIONS.lock() {
            sessions.insert(self.id.clone(), ctx.address());
            info!("Added session {} to active sessions. Total active: {}", self.id, sessions.len());
        }
        
        // Clone what we need for async registration
        let id = self.id.clone();
        let user_id = self.user_id.clone();
        let db = self.db.clone();
        
        // Register WebSocket connection in storage asynchronously
        // This prevents the database I/O from blocking the WebSocket startup
        tokio::spawn(async move {
            if let Err(e) = db.add_websocket(&id, &user_id) {
                error!("Failed to register WebSocket connection in database: {}", e);
                // We don't stop the actor since the WebSocket is already functional
            }
        });
        
        info!("WebSocket connection started: {}", self.id);
        
        // Send a welcome message
        let welcome_msg = WsMessage::Chat {
            username: "System".to_string(),
            message: format!("Welcome to the chat! Your session ID is {}", self.id),
            timestamp: Utc::now().to_rfc3339(),
        };
        
        if let Ok(json) = serde_json::to_string(&welcome_msg) {
            ctx.text(json);
        }
        
        // Also send an app log message that a new client connected
        broadcast_app_log("info", &format!("New client connected with ID: {}", self.id));
    }
    
    fn stopping(&mut self, _: &mut Self::Context) -> Running {
        // Unregister WebSocket connection from storage
        if let Err(e) = self.db.remove_websocket(&self.id) {
            error!("Failed to unregister WebSocket connection: {}", e);
        }
        
        // Remove from active sessions
        if let Ok(mut sessions) = ACTIVE_SESSIONS.lock() {
            sessions.remove(&self.id);
            info!("Removed session {} from active sessions. Total active: {}", self.id, sessions.len());
        }
        
        // Send app log about client disconnection
        broadcast_app_log("info", &format!("Client disconnected: {}", self.id));
        
        info!("WebSocket connection stopped: {}", self.id);
        Running::Stop
    }
}

// Handle incoming broadcast messages for this session
impl Handler<BroadcastMessage> for WebSocketSession {
    type Result = ();

    fn handle(&mut self, msg: BroadcastMessage, ctx: &mut Self::Context) {
        // Verify this message isn't from self (extra safety check)
        if msg.sender_id != self.id {
            // Forward the message to the WebSocket client
            ctx.text(msg.content);
            debug!("Delivered broadcast message to client {}", self.id);
        }
    }
}

impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for WebSocketSession {
    fn handle(
        &mut self,
        msg: Result<ws::Message, ws::ProtocolError>,
        ctx: &mut Self::Context,
    ) {
        match msg {
            Ok(ws::Message::Ping(msg)) => {
                self.hb = Instant::now();
                ctx.pong(&msg);
            }
            Ok(ws::Message::Pong(_)) => {
                self.hb = Instant::now();
            }
            Ok(ws::Message::Text(text)) => {
                // Log the incoming message for debugging
                debug!("Received text message: {}", text);
                
                // Try to parse as a message
                match serde_json::from_str::<WsMessage>(&text) {
                    Ok(WsMessage::Chat { message, .. }) => {
                        // Create a proper chat message with user info
                        let chat_message = WsMessage::Chat {
                            username: self.user_id.clone(),
                            message,
                            timestamp: Utc::now().to_rfc3339(),
                        };
                        
                        // Echo back to sender
                        if let Ok(json) = serde_json::to_string(&chat_message) {
                            ctx.text(json);
                        }
                        
                        // Broadcast to all connected clients immediately
                        self.broadcast_message(&chat_message);
                    }
                    Ok(WsMessage::ReleaseUpdate { .. }) => {
                        // Clients shouldn't send release updates
                        warn!("Client {} tried to send a release update", self.id);
                    }
                    Ok(WsMessage::AppLog { .. }) => {
                        // Clients shouldn't send app logs
                        warn!("Client {} tried to send an app log", self.id);
                    }
                    Err(e) => {
                        // Invalid message format
                        error!("Invalid message format from {}: {}", self.id, e);
                        ctx.text(format!("Error: Invalid message format: {}", e));
                    }
                }
            }
            Ok(ws::Message::Binary(_)) => {
                // We don't handle binary data
                warn!("Received binary data from {}, ignoring", self.id);
            }
            Ok(ws::Message::Close(reason)) => {
                info!("WebSocket connection closing: {}", self.id);
                ctx.close(reason);
                ctx.stop();
            }
            Ok(ws::Message::Continuation(_)) => {
                // We don't expect continuation frames
                warn!("Received continuation frame from {}, ignoring", self.id);
            }
            Ok(ws::Message::Nop) => {
                // No operation, just ignore
            }
            Err(e) => {
                error!("WebSocket protocol error: {}", e);
                ctx.stop();
            }
        }
    }
}

// Function to broadcast release updates to all connected clients
pub fn broadcast_release_update(release_id: String, status: String, progress: f32, log_line: Option<String>) {
    // Is this an error message?
    let is_error = log_line.as_ref().map_or(false, |line| {
        line.contains("ERROR") || line.contains("error") || line.contains("FAILED") || status == "Error"
    });
    
    // Create update message
    let update = WsMessage::ReleaseUpdate {
        release_id: release_id.clone(),
        status: status.clone(),
        progress,
        log_line: log_line.clone(),
    };
    
    // Convert to JSON
    if let Ok(json) = serde_json::to_string(&update) {
        // Always log significant status changes or errors
        info!("BROADCASTING: Release {} - Status: {} - Progress: {:.1}%", 
            release_id, status, progress);
        
        if let Some(line) = &log_line {
            if is_error {
                error!("RELEASE ERROR: {} - {}", release_id, line);
            } else {
                info!("RELEASE LOG: {} - {}", release_id, line);
            }
        }
        
        // Send to all active sessions
        if let Ok(sessions) = ACTIVE_SESSIONS.lock() {
            let client_count = sessions.len();
            if client_count == 0 {
                return;
            }
            
            let mut broadcast_count = 0;
            for (_, addr) in sessions.iter() {
                // Send the message
                let msg = BroadcastMessage {
                    content: json.clone(),
                    sender_id: "system".to_string(), // System-generated message
                };
                
                addr.do_send(msg);
                broadcast_count += 1;
            }
            
            debug!("Release update broadcast to {} clients", broadcast_count);
        }
    } else {
        error!("Failed to serialize release update");
    }
}

// Function to broadcast application logs to all connected clients
pub fn broadcast_app_log(level: &str, message: &str) {
    let timestamp = Utc::now().to_rfc3339();
    
    // Create app log message
    let app_log = WsMessage::AppLog {
        level: level.to_string(),
        message: message.to_string(),
        timestamp,
    };
    
    // Always log to console as well
    match level {
        "error" => error!("APP LOG: {}", message),
        "warn" => warn!("APP LOG: {}", message),
        _ => info!("APP LOG: {}", message),
    }
    
    // Broadcast to all clients
    if let Ok(json) = serde_json::to_string(&app_log) {
        if let Ok(sessions) = ACTIVE_SESSIONS.lock() {
            if sessions.is_empty() {
                return;
            }
            
            let mut broadcast_count = 0;
            for (_, addr) in sessions.iter() {
                let msg = BroadcastMessage {
                    content: json.clone(),
                    sender_id: "system".to_string(),
                };
                addr.do_send(msg);
                broadcast_count += 1;
            }
            
            debug!("App log broadcast to {} clients", broadcast_count);
        }
    } else {
        error!("Failed to serialize app log");
    }
}
