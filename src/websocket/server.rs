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
            info!("Broadcasting message: {}", json);
            
            // Send message to all active sessions except self
            if let Ok(sessions) = ACTIVE_SESSIONS.lock() {
                let client_count = sessions.len();
                if client_count <= 1 {
                    info!("No other clients to broadcast to");
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
                
                info!("Message broadcast directly to {} clients", broadcast_count);
            }
        }
    }
}

impl Actor for WebSocketSession {
    type Context = ws::WebsocketContext<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        self.heartbeat(ctx);
        
        // Register WebSocket connection in storage
        if let Err(e) = self.db.add_websocket(&self.id, &self.user_id) {
            error!("Failed to register WebSocket connection: {}", e);
            ctx.stop();
            return;
        }
        
        // Register actor address in active sessions
        if let Ok(mut sessions) = ACTIVE_SESSIONS.lock() {
            sessions.insert(self.id.clone(), ctx.address());
            info!("Added session {} to active sessions. Total active: {}", self.id, sessions.len());
        }
        
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
                info!("Received text message: {}", text);
                
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
    use log::{info, error};
    // Use the local WsMessage, not from models
    
    // Create update message
    let update = WsMessage::ReleaseUpdate {
        release_id: release_id.clone(),
        status,
        progress,
        log_line,
    };
    
    // Convert to JSON
    if let Ok(json) = serde_json::to_string(&update) {
        info!("Broadcasting release update: {}", json);
        
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
            
            info!("Release update broadcast to {} clients", broadcast_count);
        }
    } else {
        error!("Failed to serialize release update");
    }
}
