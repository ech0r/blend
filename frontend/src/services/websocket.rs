use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{WebSocket, MessageEvent, ErrorEvent, CloseEvent};
use yew::prelude::*;
use crate::models::WsMessage;
use log::{info, warn, error};
use std::sync::Arc;
use std::cell::RefCell;
use std::rc::Rc;

pub enum WsAction {
    Connect,
    SendMessage(String),
    Received(Result<WsMessage, String>),
    Disconnected,
    Error(String),
}

pub struct WebSocketService {
    ws: Option<WebSocket>,
    callback: Callback<WsAction>,
}

impl WebSocketService {
    pub fn new(callback: Callback<WsAction>) -> Self {
        Self {
            ws: None,
            callback,
        }
    }
    
    pub fn connect(&mut self) -> Result<(), String> {
        // If already connected, disconnect first
        if self.is_connected() {
            info!("WebSocket already connected, disconnecting first");
            self.disconnect();
        }

        // Determine WebSocket URL based on current location
        let location = web_sys::window()
            .ok_or("No window object found")?
            .location();
            
        let protocol = match location.protocol() {
            Ok(p) => if p == "https:" { "wss:" } else { "ws:" },
            Err(e) => return Err(format!("Failed to get protocol: {:?}", e)),
        };
        
        let host = match location.host() {
            Ok(h) => h,
            Err(e) => return Err(format!("Failed to get host: {:?}", e)),
        };
        
        let ws_url = format!("{}//{}/ws", protocol, host);
        
        info!("Connecting to WebSocket at {}", ws_url);
        
        let ws = WebSocket::new(&ws_url)
            .map_err(|e| format!("Failed to create WebSocket: {:?}", e))?;
            
        // Set binary type to arraybuffer
        ws.set_binary_type(web_sys::BinaryType::Arraybuffer);
        
        // Set up onopen handler
        let onopen_callback = self.callback.clone();
        let ws_clone = ws.clone();
        let onopen_closure = Closure::wrap(Box::new(move |_| {
            info!("WebSocket connection established");
            onopen_callback.emit(WsAction::Connect);
            
            // Send a test message to check if the connection is working
            let test_msg = WsMessage::Chat {
                username: "Client".to_string(),
                message: "Connection test".to_string(),
                timestamp: "".to_string(),
            };
            
            if let Ok(json) = serde_json::to_string(&test_msg) {
                let _ = ws_clone.send_with_str(&json);
            }
        }) as Box<dyn FnMut(JsValue)>);
        ws.set_onopen(Some(onopen_closure.as_ref().unchecked_ref()));
        onopen_closure.forget();
        
        // Set up onmessage handler
        let onmessage_callback = self.callback.clone();
        let onmessage_closure = Closure::wrap(Box::new(move |e: MessageEvent| {
            if let Ok(text) = e.data().dyn_into::<js_sys::JsString>() {
                let text = String::from(text);
                match serde_json::from_str::<WsMessage>(&text) {
                    Ok(message) => {
                        onmessage_callback.emit(WsAction::Received(Ok(message)));
                    }
                    Err(err) => {
                        error!("Failed to parse message: {}", err);
                        onmessage_callback.emit(WsAction::Received(Err(format!("Parse error: {}", err))));
                    }
                }
            }
        }) as Box<dyn FnMut(MessageEvent)>);
        ws.set_onmessage(Some(onmessage_closure.as_ref().unchecked_ref()));
        onmessage_closure.forget();
        
        // Set up onerror handler
        let onerror_callback = self.callback.clone();
        let onerror_closure = Closure::wrap(Box::new(move |e: ErrorEvent| {
            error!("WebSocket error: {:?}", e);
            onerror_callback.emit(WsAction::Error("WebSocket connection error".to_string()));
        }) as Box<dyn FnMut(ErrorEvent)>);
        ws.set_onerror(Some(onerror_closure.as_ref().unchecked_ref()));
        onerror_closure.forget();
        
        // Set up onclose handler
        let onclose_callback = self.callback.clone();
        let onclose_closure = Closure::wrap(Box::new(move |e: CloseEvent| {
            warn!("WebSocket closed: {} - {}", e.code(), e.reason());
            onclose_callback.emit(WsAction::Disconnected);
        }) as Box<dyn FnMut(CloseEvent)>);
        ws.set_onclose(Some(onclose_closure.as_ref().unchecked_ref()));
        onclose_closure.forget();
        
        self.ws = Some(ws);
        Ok(())
    }
    
    pub fn send_message(&self, message: &WsMessage) -> Result<(), String> {
        if let Some(ws) = &self.ws {
            if ws.ready_state() == WebSocket::OPEN {
                let json = serde_json::to_string(&message)
                    .map_err(|e| format!("Serialization error: {}", e))?;
                    
                ws.send_with_str(&json)
                    .map_err(|e| format!("Send error: {:?}", e))?;
                    
                Ok(())
            } else {
                Err("WebSocket not connected".to_string())
            }
        } else {
            Err("WebSocket not initialized".to_string())
        }
    }
    
    pub fn send_chat(&self, message: &str) -> Result<(), String> {
        // Just a convenience wrapper for sending chat messages
        let ws_message = WsMessage::Chat {
            username: "".to_string(), // Will be set by server
            message: message.to_string(),
            timestamp: "".to_string(), // Will be set by server
        };
        
        self.send_message(&ws_message)
    }
    
    pub fn is_connected(&self) -> bool {
        if let Some(ws) = &self.ws {
            ws.ready_state() == WebSocket::OPEN
        } else {
            false
        }
    }
    
    pub fn disconnect(&mut self) {
        if let Some(ws) = &self.ws {
            if ws.ready_state() == WebSocket::OPEN || ws.ready_state() == WebSocket::CONNECTING {
                let _ = ws.close();
            }
        }
        
        self.ws = None;
    }
}

impl Drop for WebSocketService {
    fn drop(&mut self) {
        self.disconnect();
    }
}
