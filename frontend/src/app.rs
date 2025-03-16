use yew::prelude::*;
use wasm_bindgen_futures::spawn_local;
use log::{info, error};
use web_sys::console;

use crate::models::{Release, Client, User, WsMessage, Environment, ReleaseStatus};
use crate::services::api::ApiClient;
use crate::services::websocket::{WebSocketService, WsAction};
use crate::components::kanban::KanbanBoard;
use crate::components::header::Header;
use crate::components::chat::ChatPanel;
use crate::components::release_form::ReleaseForm;

pub enum AppMsg {
    FetchReleases,
    ReleasesReceived(Vec<Release>),
    ClientsReceived(Vec<Client>),
    CurrentUserReceived(User),
    ReleaseUpdated(Release),
    DeleteRelease(String),
    ReleaseDeleted(String),
    MoveRelease(String, Environment),
    OpenReleaseForm,
    CloseReleaseForm,
    CreateRelease(Release),
    ReleaseCreated(Release),
    ConnectWebSocket,
    WebSocketAction(WsAction),
    SendChatMessage(String),
    ToggleChatPanel,
    Error(String),
}

pub struct App {
    releases: Vec<Release>,
    clients: Vec<Client>,
    current_user: Option<User>,
    ws_service: Option<WebSocketService>,
    chat_messages: Vec<WsMessage>,
    show_release_form: bool,
    show_chat_panel: bool,
    error: Option<String>,
}

impl Component for App {
    type Message = AppMsg;
    type Properties = ();
    
    fn create(ctx: &Context<Self>) -> Self {
        // Initialize app state
        let app = Self {
            releases: Vec::new(),
            clients: Vec::new(),
            current_user: None,
            ws_service: None,
            chat_messages: Vec::new(),
            show_release_form: false,
            show_chat_panel: false,
            error: None,
        };
        
        // Fetch data on creation - delay slightly to ensure DOM is ready
        let link = ctx.link().clone();
        gloo_timers::callback::Timeout::new(100, move || {
            link.send_message(AppMsg::FetchReleases);
            link.send_message(AppMsg::ConnectWebSocket);
        }).forget();
        
        app
    }
    
    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            AppMsg::FetchReleases => {
                // Fetch releases from API
                let link = ctx.link().clone();
                spawn_local(async move {
                    match ApiClient::get_releases().await {
                        Ok(releases) => link.send_message(AppMsg::ReleasesReceived(releases)),
                        Err(e) => link.send_message(AppMsg::Error(format!("Failed to fetch releases: {}", e))),
                    }
                    
                    // Also fetch clients
                    match ApiClient::get_clients().await {
                        Ok(clients) => link.send_message(AppMsg::ClientsReceived(clients)),
                        Err(e) => link.send_message(AppMsg::Error(format!("Failed to fetch clients: {}", e))),
                    }
                    
                    // And current user
                    match ApiClient::get_current_user().await {
                        Ok(user) => link.send_message(AppMsg::CurrentUserReceived(user)),
                        Err(e) => {
                            // This is not critical, as user might not be logged in yet
                            console::warn_1(&format!("Failed to fetch current user: {}", e).into());
                        }
                    }
                });
                
                false
            }
            AppMsg::ReleasesReceived(releases) => {
                self.releases = releases;
                true
            }
            AppMsg::ClientsReceived(clients) => {
                self.clients = clients;
                true
            }
            AppMsg::CurrentUserReceived(user) => {
                self.current_user = Some(user);
                true
            }
            AppMsg::ReleaseUpdated(updated_release) => {
                // Find and update the release in the list
                if let Some(index) = self.releases.iter().position(|r| r.id == updated_release.id) {
                    self.releases[index] = updated_release;
                }
                true
            }
            AppMsg::DeleteRelease(release_id) => {
                // Delete release through API
                let link = ctx.link().clone();
                spawn_local(async move {
                    match ApiClient::delete_release(&release_id).await {
                        Ok(_) => link.send_message(AppMsg::ReleaseDeleted(release_id)),
                        Err(e) => link.send_message(AppMsg::Error(format!("Failed to delete release: {}", e))),
                    }
                });
                
                false
            }
            AppMsg::ReleaseDeleted(release_id) => {
                // Remove release from the list
                self.releases.retain(|r| r.id != release_id);
                true
            }
            AppMsg::MoveRelease(release_id, target_env) => {
                // Find the release
                if let Some(release) = self.releases.iter().find(|r| r.id == release_id) {
                    let release = release.clone();
                    
                    // Create update request
                    let link = ctx.link().clone();
                    spawn_local(async move {
                        match ApiClient::update_release(
                            &release.id,
                            release.title,
                            release.client_id,
                            release.current_environment,
                            target_env,
                            release.deployment_items.into_iter().map(|i| i.name).collect(),
                            release.scheduled_at,
                        ).await {
                            Ok(updated) => link.send_message(AppMsg::ReleaseUpdated(updated)),
                            Err(e) => link.send_message(AppMsg::Error(format!("Failed to update release: {}", e))),
                        }
                    });
                }
                
                false
            }
            AppMsg::OpenReleaseForm => {
                self.show_release_form = true;
                true
            }
            AppMsg::CloseReleaseForm => {
                self.show_release_form = false;
                true
            }
            AppMsg::CreateRelease(release) => {
                // Create new release through API
                let link = ctx.link().clone();
                spawn_local(async move {
                    match ApiClient::create_release(
                        release.title,
                        release.client_id,
                        release.current_environment,
                        release.target_environment,
                        release.deployment_items.into_iter().map(|i| i.name).collect(),
                        release.scheduled_at,
                    ).await {
                        Ok(created) => link.send_message(AppMsg::ReleaseCreated(created)),
                        Err(e) => link.send_message(AppMsg::Error(format!("Failed to create release: {}", e))),
                    }
                });
                
                false
            }
            AppMsg::ReleaseCreated(release) => {
                // Add new release to the list and close form
                self.releases.push(release);
                self.show_release_form = false;
                true
            }
            AppMsg::ConnectWebSocket => {
                // Only initialize WebSocket if we don't already have one
                if self.ws_service.is_none() {
                    info!("Initializing WebSocket connection");
                    let ws_callback = ctx.link().callback(AppMsg::WebSocketAction);
                    let mut ws_service = WebSocketService::new(ws_callback);
                    
                    if let Err(e) = ws_service.connect() {
                        error!("Failed to connect WebSocket: {}", e);
                        ctx.link().send_message(AppMsg::Error(format!("Failed to connect WebSocket: {}", e)));
                    } else {
                        self.ws_service = Some(ws_service);
                    }
                } else {
                    info!("WebSocket already initialized, skipping connection");
                }
                
                false
            }
            AppMsg::WebSocketAction(action) => {
                match action {
                    WsAction::Connect => {
                        info!("WebSocket connected");
                    }
                    WsAction::SendMessage(_) => {
                        // This is just for tracking outgoing messages, no action needed
                    }
                    WsAction::Received(Ok(message)) => {
                        match &message {
                            WsMessage::Chat { .. } => {
                                // Add chat message to list
                                self.chat_messages.push(message);
                            }
                            WsMessage::ReleaseUpdate { release_id, status, progress, log_line } => {
                                // Find and update release
                                if let Some(release) = self.releases.iter_mut().find(|r| r.id == *release_id) {
                                    // Update status
                                    release.status = match status.as_str() {
                                        "Pending" => ReleaseStatus::Pending,
                                        "InProgress" => ReleaseStatus::InProgress,
                                        "Completed" => ReleaseStatus::Completed,
                                        "Failed" => ReleaseStatus::Failed,
                                        _ => release.status.clone(),
                                    };
                                    
                                    // Update progress
                                    release.progress = *progress;
                                    
                                    // Add log line if present
                                    if let Some(log) = log_line {
                                        // Find the active deployment item and add log
                                        // Create a regex to extract the item name from the log line
                                        let log_line = log.clone();
                                        
                                        // Parse the log line to find the deployment item
                                        // Format is usually "[item_name] log message"
                                        if let Some(start_idx) = log_line.find('[') {
                                            if let Some(end_idx) = log_line.find(']') {
                                                if start_idx < end_idx {
                                                    let item_name = log_line[start_idx+1..end_idx].trim();
                                                    
                                                    // Find the deployment item with this name
                                                    if let Some(item) = release.deployment_items.iter_mut()
                                                        .find(|i| i.name == item_name) {
                                                        item.logs.push(log_line.clone());
                                                    } else {
                                                        // If we can't find a specific item, add to all in-progress items
                                                        for item in release.deployment_items.iter_mut()
                                                            .filter(|i| i.status == ReleaseStatus::InProgress) {
                                                            item.logs.push(log_line.clone());
                                                        }
                                                    }
                                                }
                                            }
                                        } else {
                                            // If the format doesn't match, add to all items
                                            for item in release.deployment_items.iter_mut() {
                                                item.logs.push(log_line.clone());
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        return true;
                    }
                    WsAction::Received(Err(err)) => {
                        error!("WebSocket message error: {}", err);
                        self.error = Some(format!("WebSocket message error: {}", err));
                    }
                    WsAction::Disconnected => {
                        info!("WebSocket disconnected");
                        self.ws_service = None;
                        
                        // Try to reconnect after a delay - but only once
                        let link = ctx.link().clone();
                        let callback = Box::new(move || {
                            info!("Attempting to reconnect WebSocket after timeout");
                            link.send_message(AppMsg::ConnectWebSocket);
                        });
                        gloo_timers::callback::Timeout::new(5_000, callback).forget();
                        
                        // Show reconnecting message
                        self.error = Some("WebSocket disconnected. Reconnecting...".to_string());
                        return true;
                    }
                    WsAction::Error(err) => {
                        error!("WebSocket error: {}", err);
                        self.error = Some(format!("WebSocket error: {}", err));
                        self.ws_service = None;
                        
                        // Try to reconnect after a delay - but only once
                        let link = ctx.link().clone();
                        let callback = Box::new(move || {
                            info!("Attempting to reconnect WebSocket after error");
                            link.send_message(AppMsg::ConnectWebSocket);
                        });
                        gloo_timers::callback::Timeout::new(5_000, callback).forget();
                        
                        return true;
                    }
                }
                
                false
            }
            AppMsg::SendChatMessage(message) => {
                // Send chat message through WebSocket
                if let Some(ws) = &self.ws_service {
                    if let Err(e) = ws.send_chat(&message) {
                        error!("Failed to send chat message: {}", e);
                        self.error = Some(format!("Failed to send chat message: {}", e));
                    }
                }
                
                false
            }
            AppMsg::ToggleChatPanel => {
                self.show_chat_panel = !self.show_chat_panel;
                true
            }
            AppMsg::Error(error) => {
                self.error = Some(error);
                true
            }
        }
    }
    
    fn view(&self, ctx: &Context<Self>) -> Html {
        let dev_releases = self.releases.iter()
            .filter(|r| r.current_environment == Environment::Development)
            .cloned()
            .collect::<Vec<_>>();
            
        let staging_releases = self.releases.iter()
            .filter(|r| r.current_environment == Environment::Staging)
            .cloned()
            .collect::<Vec<_>>();
            
        let prod_releases = self.releases.iter()
            .filter(|r| r.current_environment == Environment::Production)
            .cloned()
            .collect::<Vec<_>>();
        
        let is_connected = self.ws_service.as_ref().map(|ws| ws.is_connected()).unwrap_or(false);
        
        html! {
            <div class="app-container">
                <Header 
                    user={self.current_user.clone()}
                    on_new_release={ctx.link().callback(|_| AppMsg::OpenReleaseForm)}
                    on_toggle_chat={ctx.link().callback(|_| AppMsg::ToggleChatPanel)}
                    is_connected={is_connected}
                />
                
                <main class="main-content">
                    <KanbanBoard 
                        dev_releases={dev_releases}
                        staging_releases={staging_releases}
                        prod_releases={prod_releases}
                        on_move_release={ctx.link().callback(|(id, env)| AppMsg::MoveRelease(id, env))}
                        on_delete_release={ctx.link().callback(AppMsg::DeleteRelease)}
                    />
                    
                    {
                        if self.show_chat_panel {
                            html! {
                                <ChatPanel 
                                    messages={self.chat_messages.clone()}
                                    on_send={ctx.link().callback(AppMsg::SendChatMessage)}
                                />
                            }
                        } else {
                            html! {}
                        }
                    }
                </main>
                
                {
                    // Error notification
                    if let Some(error) = &self.error {
                        html! {
                            <div class="error-notification">
                                <p>{ error }</p>
                                <button
                                    onclick={ctx.link().callback(|_| AppMsg::Error(String::new()))}
                                >
                                    { "Dismiss" }
                                </button>
                            </div>
                        }
                    } else {
                        html! {}
                    }
                }
                
                {
                    // Release form modal
                    if self.show_release_form {
                        html! {
                            <div class="modal-overlay">
                                <div class="modal-container">
                                    <ReleaseForm 
                                        clients={self.clients.clone()}
                                        on_submit={ctx.link().callback(AppMsg::CreateRelease)}
                                        on_cancel={ctx.link().callback(|_| AppMsg::CloseReleaseForm)}
                                    />
                                </div>
                            </div>
                        }
                    } else {
                        html! {}
                    }
                }
            </div>
        }
    }
}
