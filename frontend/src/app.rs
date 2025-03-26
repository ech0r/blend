use yew::prelude::*;
use wasm_bindgen_futures::spawn_local;
use log::{info, error};
use web_sys::{console, Event};
use std::rc::Rc;
use chrono::Utc;

use crate::models::{Release, Client, User, WsMessage, Environment, ReleaseStatus};
use crate::services::api::ApiClient;
use crate::services::websocket::{WebSocketService, WsAction};
use crate::components::kanban::KanbanBoard;
use crate::components::header::Header;
use crate::components::chat::ChatPanel;
use crate::components::release_form::ReleaseForm;
use crate::components::log_drawer::{LogDrawer, LogEntry};

pub enum AppMsg {
    FetchReleases,
    ReleasesReceived(Vec<Release>),
    ClientsReceived(Vec<Client>),
    CurrentUserReceived(User),
    ReleaseUpdated(Release),
    DeleteRelease(String),
    ReleaseDeleted(String),
    MoveRelease(String, Environment),
    ClearRelease(String),
    OpenReleaseForm,
    CloseReleaseForm,
    CreateRelease(Release),
    ReleaseCreated(Release),
    ConnectWebSocket,
    WebSocketAction(WsAction),
    SendChatMessage(String),
    ToggleChatPanel,
    OpenLogDrawer(String),
    CloseLogDrawer,
    Error(String),
    Info(String),
    DismissError,
    DismissInfo,
    AutoDismissError,
    AutoDismissInfo,
    ToggleAppLog,
}

pub struct App {
    releases: Vec<Release>,
    clients: Vec<Client>,
    current_user: Option<User>,
    ws_service: Option<WebSocketService>,
    chat_messages: Vec<WsMessage>,
    show_release_form: bool,
    show_chat_panel: bool,
    show_log_drawer: bool,
    active_release_id: String,
    logs: Vec<LogEntry>,
    error: Option<String>,
    info: Option<String>,
    error_dismissing: bool,
    info_dismissing: bool,
    show_app_log: bool,
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
            show_log_drawer: false,
            active_release_id: String::new(),
            logs: Vec::new(),
            error: None,
            info: None,
            error_dismissing: false,
            info_dismissing: false,
            show_app_log: false,
        };

        // Fetch data immediately - no delay
        ctx.link().send_message(AppMsg::FetchReleases);
        ctx.link().send_message(AppMsg::ConnectWebSocket);

        app
    }
    
    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            AppMsg::DismissError => {
                self.error_dismissing = true;

                // Set timeout to clear the error after animation completes
                let link = ctx.link().clone();
                let callback = Box::new(move || {
                    link.send_message(AppMsg::AutoDismissError);
                });
                gloo_timers::callback::Timeout::new(300, callback).forget();

                true
            }
            AppMsg::AutoDismissError => {
                self.error = None;
                self.error_dismissing = false;
                true
            }
            AppMsg::DismissInfo => {
                self.info_dismissing = true;

                // Set timeout to clear the info after animation completes
                let link = ctx.link().clone();
                let callback = Box::new(move || {
                    link.send_message(AppMsg::AutoDismissInfo);
                });
                gloo_timers::callback::Timeout::new(300, callback).forget();

                true
            }
            AppMsg::AutoDismissInfo => {
                self.info = None;
                self.info_dismissing = false;
                true
            }
            AppMsg::ToggleAppLog => {
                self.show_app_log = !self.show_app_log;
                if self.show_log_drawer {
                    self.show_log_drawer = !self.show_log_drawer;
                }
                true
            }
            AppMsg::Error(error) => {
                self.error = Some(error);
                self.error_dismissing = false;

                // Set auto-dismiss timeout (5 seconds)
                let link = ctx.link().clone();
                let callback = Box::new(move || {
                    link.send_message(AppMsg::DismissError);
                });
                gloo_timers::callback::Timeout::new(5000, callback).forget();

                true
            }
            AppMsg::Info(info) => {
                self.info = Some(info);
                self.info_dismissing = false;

                // Set auto-dismiss timeout (5 seconds)
                let link = ctx.link().clone();
                let callback = Box::new(move || {
                    link.send_message(AppMsg::DismissInfo);
                });
                gloo_timers::callback::Timeout::new(5000, callback).forget();

                true
            }       
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
                            release.skip_staging,
                        ).await {
                            Ok(updated) => link.send_message(AppMsg::ReleaseUpdated(updated)),
                            Err(e) => link.send_message(AppMsg::Error(format!("Failed to update release: {}", e))),
                        }
                    });
                }
                
                false
            }
            AppMsg::ClearRelease(release_id) => {
                // Instead of updating the entire release, just call the status update endpoint directly
                let link = ctx.link().clone();
                spawn_local(async move {
                    match ApiClient::update_release_status(&release_id, "clear").await {
                        Ok(updated) => link.send_message(AppMsg::ReleaseUpdated(updated)),
                        Err(e) => link.send_message(AppMsg::Error(format!("Failed to clear release: {}", e))),
                    }
                });

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
                info!("Top level create release");
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
                        release.skip_staging,
                    ).await {
                        Ok(created) => {
                            link.send_message(AppMsg::ReleaseCreated(created));
                            link.send_message(AppMsg::Info("Release successfully created!".to_string()));
                        },
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
                                    // Update progress
                                    release.progress = *progress;
                                    
                                    // Update status if it's a status change (not just a progress update)
                                    if !status.is_empty() && status != "InProgress" && status != "ItemComplete" {
                                        // Try to parse the status string to our enum
                                        let new_status = match status.as_str() {
                                            "InDevelopment" => Some(ReleaseStatus::InDevelopment),
                                            "ClearedInDevelopment" => Some(ReleaseStatus::ClearedInDevelopment),
                                            "DeployingToStaging" => Some(ReleaseStatus::DeployingToStaging),
                                            "ReadyToTestInStaging" => Some(ReleaseStatus::ReadyToTestInStaging),
                                            "ClearedInStaging" => Some(ReleaseStatus::ClearedInStaging),
                                            "DeployingToProduction" => Some(ReleaseStatus::DeployingToProduction),
                                            "ReadyToTestInProduction" => Some(ReleaseStatus::ReadyToTestInProduction),
                                            "ClearedInProduction" => Some(ReleaseStatus::ClearedInProduction),
                                            "Error" => Some(ReleaseStatus::Error),
                                            "Blocked" => Some(ReleaseStatus::Blocked),
                                            _ => None,
                                        };
                                        
                                        if let Some(new_status) = new_status {
                                            // Only update if it's different to avoid unnecessary re-renders
                                            if release.status != new_status {
                                                info!("Updating release {} status from {:?} to {:?}", 
                                                      release_id, release.status, new_status);
                                                
                                                // Set the new status
                                                release.status = new_status.clone();
                                                
                                                // Add a notification for important status changes
                                                match new_status {
                                                    ReleaseStatus::ReadyToTestInStaging => {
                                                        self.info = Some(format!("Release '{}' is ready for testing in Staging", release.title));
                                                        self.info_dismissing = false;
                                                        
                                                        // Auto-dismiss after 5 seconds
                                                        let link = ctx.link().clone();
                                                        let callback = Box::new(move || {
                                                            link.send_message(AppMsg::DismissInfo);
                                                        });
                                                        gloo_timers::callback::Timeout::new(5000, callback).forget();
                                                    },
                                                    ReleaseStatus::ReadyToTestInProduction => {
                                                        self.info = Some(format!("Release '{}' is ready for testing in Production", release.title));
                                                        self.info_dismissing = false;
                                                        
                                                        // Auto-dismiss after 5 seconds
                                                        let link = ctx.link().clone();
                                                        let callback = Box::new(move || {
                                                            link.send_message(AppMsg::DismissInfo);
                                                        });
                                                        gloo_timers::callback::Timeout::new(5000, callback).forget();
                                                    },
                                                    ReleaseStatus::Error => {
                                                        self.error = Some(format!("Release '{}' encountered an error during deployment", release.title));
                                                        self.error_dismissing = false;
                                                        
                                                        // Auto-dismiss after 8 seconds
                                                        let link = ctx.link().clone();
                                                        let callback = Box::new(move || {
                                                            link.send_message(AppMsg::DismissError);
                                                        });
                                                        gloo_timers::callback::Timeout::new(8000, callback).forget();
                                                    },
                                                    _ => {}
                                                }
                                            }
                                        }
                                    }
                                    
                                    // Process item completion messages
                                    if status == "ItemComplete" && log_line.is_some() {
                                        // This is a special status indicating an individual deployment item completed
                                        // The log line will contain information about which item completed
                                        let log = log_line.as_ref().unwrap();
                                        let timestamp = Utc::now().format("%H:%M:%S").to_string();
                                        
                                        // Extract item name from the log line if possible
                                        let item_name = if let Some(start_idx) = log.find('[') {
                                            if let Some(end_idx) = log.find(']') {
                                                if start_idx < end_idx {
                                                    log[start_idx+1..end_idx].trim().to_string()
                                                } else {
                                                    "unknown".to_string()
                                                }
                                            } else {
                                                "unknown".to_string()
                                            }
                                        } else {
                                            "unknown".to_string()
                                        };
                                        
                                        // Find the deployment item with this name and update its status
                                        if let Some(item) = release.deployment_items.iter_mut().find(|i| i.name == item_name) {
                                            item.logs.push(log.clone());
                                            
                                            // Set the item status to completed based on the current environment
                                            match release.current_environment {
                                                Environment::Development => item.status = ReleaseStatus::DeployingToStaging,
                                                Environment::Staging => item.status = ReleaseStatus::ReadyToTestInStaging,
                                                Environment::Production => item.status = ReleaseStatus::ReadyToTestInProduction,
                                            }
                                        }
                                    }
                                    
                                    // Add log line if present (for any status, including progress updates)
                                    if let Some(log) = log_line {
                                        // Create a timestamp
                                        let timestamp = Utc::now().format("%H:%M:%S").to_string();
                                        
                                        // Parse the log line to find the deployment item and if it's an error
                                        let log_line = log.clone();
                                        
                                        // Determine if this is an error message
                                        let is_error = log_line.contains("ERROR") || log_line.contains("FAILED") || status == "Error";
                                        
                                        // Extract the item name from the log line if possible
                                        let item_name = if let Some(start_idx) = log_line.find('[') {
                                            if let Some(end_idx) = log_line.find(']') {
                                                if start_idx < end_idx {
                                                    log_line[start_idx+1..end_idx].trim().to_string()
                                                } else {
                                                    "unknown".to_string()
                                                }
                                            } else {
                                                "unknown".to_string()
                                            }
                                        } else {
                                            "unknown".to_string()
                                        };
                                        
                                        // Create a new log entry
                                        let log_entry = LogEntry {
                                            release_id: release_id.clone(),
                                            item_name: item_name.clone(),
                                            content: log_line.clone(),
                                            timestamp,
                                            is_error,
                                        };
                                        
                                        // Add to global logs
                                        self.logs.push(log_entry);
                                        
                                        // Find the deployment item with this name
                                        if let Some(item) = release.deployment_items.iter_mut()
                                            .find(|i| i.name == item_name) {
                                            item.logs.push(log_line.clone());
                                            
                                            // Set error if this is an error message
                                            if is_error && item.error.is_none() {
                                                item.error = Some(log_line.clone());
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
                        
                        // Try to reconnect after a short delay
                        let link = ctx.link().clone();
                        let callback = Box::new(move || {
                            info!("Attempting to reconnect WebSocket after timeout");
                            link.send_message(AppMsg::ConnectWebSocket);
                        });
                        // Reduced from 5000ms to 1000ms (1 second)
                        gloo_timers::callback::Timeout::new(100, callback).forget();
                        
                        // Show reconnecting message
                        self.error = Some("WebSocket disconnected. Reconnecting...".to_string());
                        return true;
                    }
                    WsAction::Error(err) => {
                        error!("WebSocket error: {}", err);
                        self.error = Some(format!("WebSocket error: {}", err));
                        self.ws_service = None;
                        
                        // Try to reconnect after a short delay 
                        let link = ctx.link().clone();
                        let callback = Box::new(move || {
                            info!("Attempting to reconnect WebSocket after error");
                            link.send_message(AppMsg::ConnectWebSocket);
                        });
                        // Reduced from 5000ms to 1000ms (1 second)
                        gloo_timers::callback::Timeout::new(100, callback).forget();
                        
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
            AppMsg::OpenLogDrawer(release_id) => {
                self.show_log_drawer = true;
                self.active_release_id = release_id;
                if self.show_app_log {
                    self.show_app_log = !self.show_app_log;
                }
                true
            }
            AppMsg::CloseLogDrawer => {
                self.show_log_drawer = false;
                true
            }
        }
    }
    
    fn view(&self, ctx: &Context<Self>) -> Html {        
        let is_connected = self.ws_service.as_ref().map(|ws| ws.is_connected()).unwrap_or(false);
        
        // Find the active release if log drawer is open
        let active_release = if self.show_log_drawer {
            self.releases.iter().find(|r| r.id == self.active_release_id)
        } else {
            None
        };
        
        html! {
            <div class="app-container">
                <Header 
                    user={self.current_user.clone()}
                    on_new_release={ctx.link().callback(|_| AppMsg::OpenReleaseForm)}
                    on_toggle_chat={ctx.link().callback(|_| AppMsg::ToggleChatPanel)}
                    on_toggle_log={ctx.link().callback(|_| AppMsg::ToggleAppLog)}
                    is_connected={is_connected}
                />

                <main class="main-content">
                    <KanbanBoard 
                        releases={self.releases.clone()}
                        current_user={self.current_user.clone()}
                        on_move_release={ctx.link().callback(|(id, env)| AppMsg::MoveRelease(id, env))}
                        on_clear_release={ctx.link().callback(AppMsg::ClearRelease)}
                        on_delete_release={ctx.link().callback(AppMsg::DeleteRelease)}
                        on_view_logs={ctx.link().callback(AppMsg::OpenLogDrawer)}
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
                
                // Info notification
                {
                    if let Some(info) = &self.info {
                        let info_class = if self.info_dismissing {
                            "info-notification notification-dismissing"
                        } else {
                            "info-notification"
                        };

                        html! {
                            <div class={info_class}>
                                <p>{ info }</p>
                                <button
                                    onclick={ctx.link().callback(|_| AppMsg::DismissInfo)}
                                >
                                    { "Dismiss" }
                                </button>
                            </div>
                        }
                    } else {
                        html! {}
                    }
                }
                
                // Error notification
                {
                    if let Some(error) = &self.error {
                        let error_class = if self.error_dismissing {
                            "error-notification notification-dismissing"
                        } else {
                            "error-notification"
                        };

                        html! {
                            <div class={error_class}>
                                <p>{ error }</p>
                                <button
                                    onclick={ctx.link().callback(|_| AppMsg::DismissError)}
                                >
                                    { "Dismiss" }
                                </button>
                            </div>
                        }
                    } else {
                        html! {}
                    }
                }

                // App log drawer
                {
                    if self.show_app_log {
                        html! {
                            <LogDrawer
                                visible={self.show_app_log}
                                logs={Rc::new(self.logs.clone())}
                                release_id={"app".to_string()}
                                release_title={"Application Log".to_string()}
                                on_close={ctx.link().callback(|_| AppMsg::ToggleAppLog)}
                                is_app_log={true}
                            />
                        }
                    } else {
                        html! {}
                    }
                }

                // Release log drawer
                {
                    if self.show_log_drawer && active_release.is_some() {
                        let release = active_release.unwrap();
                        html! {
                            <LogDrawer
                                visible={self.show_log_drawer}
                                logs={Rc::new(self.logs.clone())}
                                release_id={self.active_release_id.clone()}
                                release_title={release.title.clone()}
                                on_close={ctx.link().callback(|_| AppMsg::CloseLogDrawer)}
                                is_app_log={false}
                            />
                        }
                    } else {
                        html! {}
                    }
                }
                
                // Release form modal
                {
                    if self.show_release_form {
                        html! {
                            <div class="modal-overlay">
                                <div class="modal-container">
                                    <ReleaseForm 
                                        clients={self.clients.clone()}
                                        on_submit={ctx.link().callback(AppMsg::CreateRelease)}
                                        on_cancel={ctx.link().callback(|_| AppMsg::CloseReleaseForm)}
                                        on_create={ctx.link().callback(|s| AppMsg::Info(s))}
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
