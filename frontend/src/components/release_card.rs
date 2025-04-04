use yew::prelude::*;
use crate::models::{Release, Environment, ReleaseStatus, User, UserRole};
use web_sys::{DragEvent, DataTransfer};
use wasm_bindgen::JsCast;
use chrono::Local;

#[derive(Properties, PartialEq)]
pub struct ReleaseCardProps {
    pub release: Release,
    pub current_user: Option<User>, // Add current user
    pub on_delete: Callback<String>,
    pub on_move: Callback<(String, Environment)>,
    pub on_clear: Callback<String>,
    pub on_view_logs: Callback<String>,
}

#[function_component(ReleaseCard)]
pub fn release_card(props: &ReleaseCardProps) -> Html {
    let release = &props.release;
    let show_details = use_state(|| false);
    let confirm_delete = use_state(|| false);
    
    // Determine permissions based on user role
    let can_deploy_to_staging = props.current_user.as_ref()
        .map(|user| user.can_deploy_to_staging())
        .unwrap_or(false);
        
    let can_deploy_to_production = props.current_user.as_ref()
        .map(|user| user.can_deploy_to_production())
        .unwrap_or(false);
    
    // Check if this card can be cleared based on both release status and user permissions
    let can_be_cleared = release.can_be_cleared() && match release.status {
        ReleaseStatus::InDevelopment => can_deploy_to_staging,
        ReleaseStatus::ReadyToTestInStaging => can_deploy_to_production,
        ReleaseStatus::ReadyToTestInProduction => can_deploy_to_production,
        _ => false,
    };
    
    // Check if delete is allowed (admin only)
    let can_delete = props.current_user.as_ref()
        .map(|user| matches!(user.role, UserRole::Admin))
        .unwrap_or(false);
    
    let on_details_click = {
        let show_details = show_details.clone();
        Callback::from(move |_| {
            show_details.set(!*show_details);
        })
    };
    
    let on_delete = {
        let id = release.id.clone();
        let callback = props.on_delete.clone();
        let confirm_delete = confirm_delete.clone();
        Callback::from(move |_| {
            if *confirm_delete {
                callback.emit(id.clone());
            } else {
                confirm_delete.set(true);
            }
        })
    };
    
    let on_cancel_delete = {
        let confirm_delete = confirm_delete.clone();
        Callback::from(move |_| {
            confirm_delete.set(false);
        })
    };
    
    let on_drag_start = {
        let id = release.id.clone();
        Callback::from(move |event: DragEvent| {
            let data_transfer = event.data_transfer().unwrap();
            data_transfer.set_data("text/plain", &id).unwrap();
            data_transfer.set_effect_allowed("move");
        })
    };
    
    let on_clear = {
        let id = release.id.clone();
        let callback = props.on_clear.clone();
        
        Callback::from(move |_| {
            callback.emit(id.clone());
        })
    };
    
    let on_view_logs = {
        let id = release.id.clone();
        let callback = props.on_view_logs.clone();
        
        Callback::from(move |_| {
            callback.emit(id.clone());
        })
    };
    
    // Get status class and display name
    let status_class = release.status.css_class();
    let status_display = release.status.display_name();
    
    // Check if there are any logs for this release
    let has_logs = true;

    // convert internal Utc time to Local 
    let scheduled_time = release.scheduled_at.with_timezone(&Local);
    
    html! {
        <div 
            class={classes!("release-card", status_class)}
            draggable="true"
            ondragstart={on_drag_start}
        >
            <div class="card-header">
                <h3 class="release-title">{ &release.title }</h3>
                <div class="release-status">
                    <span class={status_class}>{ status_display }</span>
                </div>
            </div>
            
            <div class="release-info">
                <p class="client-name">{ format!("Client: {}", release.client_id) }</p>
                <p class="scheduled-time">
                    { 
                        format!("Scheduled: {}", scheduled_time.format("%Y-%m-%d %H:%M")) 
                    }
                </p>
                
                <div class="progress-bar">
                    <div 
                        class="progress-fill"
                        style={format!("width: {}%", release.progress)}
                    />
                    <span class="progress-text">
                        { format!("{:.1}%", release.progress) }
                    </span>
                </div>
            </div>
            
            <div class="card-actions">
                <button onclick={on_details_click}>
                    { if *show_details { "Hide Details" } else { "Show Details" } }
                </button>
                
                {
                    // Only show clear button if release can be cleared AND user has permission
                    if can_be_cleared {
                        let next_status = release.next_status();
                        let button_text = match next_status {
                            Some(ReleaseStatus::DeployingToStaging) => "Clear & Deploy to Staging",
                            Some(ReleaseStatus::DeployingToProduction) => "Clear & Deploy to Production",
                            Some(ReleaseStatus::ClearedInProduction) => "Mark as Completed",
                            _ => "Clear",
                        };
                        
                        html! {
                            <button class="clear-btn" onclick={on_clear}>
                                { button_text }
                            </button>
                        }
                    } else {
                        html! {}
                    }
                }
                
                {
                    // View logs button - always visible
                    if has_logs {
                        html! {
                            <button class="logs-btn" onclick={on_view_logs}>
                                { "View Logs" }
                            </button>
                        }
                    } else {
                        html! {}
                    }
                }
                
                {
                    // Delete button with confirmation - only for admins
                    if can_delete {
                        if *confirm_delete {
                            html! {
                                <div class="delete-confirmation">
                                    <span>{"Are you sure?"}</span>
                                    <button class="confirm-btn" onclick={on_delete.clone()}>{ "Yes" }</button>
                                    <button class="cancel-btn" onclick={on_cancel_delete}>{ "No" }</button>
                                </div>
                            }
                        } else {
                            html! {
                                <button class="delete-btn" onclick={on_delete}>
                                    { "Delete" }
                                </button>
                            }
                        }
                    } else {
                        html! {}
                    }
                }
            </div>
            
            {
                if *show_details {
                    html! {
                        <div class="release-details">
                            <h4>{ "Deployment Items" }</h4>
                            <ul class="deployment-items">
                                {                        
                                    // If entire release has error status, show a summary at the top
                                    if matches!(release.status, ReleaseStatus::Error) {
                                        html! {
                                            <div class="release-error-summary">
                                                <h4>{"⚠️ Deployment Error"}</h4>
                                                <p>{"This release encountered errors during deployment. See details for each item below."}</p>
                                            </div>
                                        }
                                    } else {
                                        html! {}
                                    }
                                }
                                {
                                    release.deployment_items.iter().map(|item| {
                                        let item_status_class = item.status.css_class();
                                        
                                        // Create rerun callback
                                        let on_rerun_item = {
                                            let release_id = release.id.clone();
                                            let item_name = item.name.clone();
                                            Callback::from(move |_| {
                                                // TODO: Add API call to rerun this specific item
                                                let release_id = release_id.clone();
                                                let item_name = item_name.clone();
                                                log::info!("Rerunning item {} for release {}", item_name, release_id);
                                                
                                                // This would be replaced with an actual API call later
                                                // on_rerun_item.emit((release_id, item_name));
                                            })
                                        };
                                        
                                        html! {
                                            <li class={classes!("deployment-item", item_status_class)}>
                                                <div class="item-header">
                                                    <span class="item-name">{ &item.name }</span>
                                                    <span class={classes!("item-status", if item.status == ReleaseStatus::Error { "item-status-error" } else { "" })}>
                                                        { item.status.display_name() }
                                                    </span>
                                                </div>
                                                
                                                // Add individual progress indicator
                                                {
                                                    if item.status == ReleaseStatus::InDevelopment || 
                                                        item.status == ReleaseStatus::DeployingToStaging ||
                                                        item.status == ReleaseStatus::DeployingToProduction {
                                                        html! {
                                                            <div class="item-progress">
                                                                <div class="item-progress-bar">
                                                                    <div class="item-progress-fill" 
                                                                        style={
                                                                            if item.status == ReleaseStatus::InDevelopment { 
                                                                                "width: 0%" 
                                                                            } else if matches!(item.status, 
                                                                                ReleaseStatus::ReadyToTestInStaging | 
                                                                                ReleaseStatus::ReadyToTestInProduction | 
                                                                                ReleaseStatus::ClearedInStaging |
                                                                                ReleaseStatus::ClearedInProduction) {
                                                                                "width: 100%"
                                                                            } else {
                                                                                "width: 50%"  // In progress
                                                                            }
                                                                        } />
                                                                </div>
                                                                <span class="item-progress-text">
                                                                    {
                                                                        if item.status == ReleaseStatus::InDevelopment {
                                                                            "Not started"
                                                                        } else if matches!(item.status, 
                                                                            ReleaseStatus::ReadyToTestInStaging | 
                                                                            ReleaseStatus::ReadyToTestInProduction | 
                                                                            ReleaseStatus::ClearedInStaging |
                                                                            ReleaseStatus::ClearedInProduction) {
                                                                            "Completed"
                                                                        } else {
                                                                            "In progress"
                                                                        }
                                                                    }
                                                                </span>
                                                            </div>
                                                        }
                                                    } else {
                                                        html! {}
                                                    }
                                                }
                                                
                                                {
                                                    if !item.logs.is_empty() {
                                                        html! {
                                                            <div class="logs-summary">
                                                                <p>{ format!("{} log entries", item.logs.len()) }</p>
                                                            </div>
                                                        }
                                                    } else {
                                                        html! {}
                                                    }
                                                }
                                                
                                                {
                                                    if let Some(error) = &item.error {
                                                        html! {
                                                            <div class="error">
                                                                <h5>{"Error Details:"}</h5>
                                                                <p>{ error }</p>
                                                            </div>
                                                        }
                                                    } else {
                                                        html! {}
                                                    }
                                                }
                                                
                                                // Add re-run button
                                                {
                                                    if (can_deploy_to_staging || can_deploy_to_production) {
                                                        html! {
                                                            <button class="rerun-item-btn" onclick={on_rerun_item}>
                                                                { "RE-RUN" }
                                                            </button>
                                                        }
                                                    } else {
                                                        html! {}
                                                    }
                                                }
                                            </li>
                                        }
                                    }).collect::<Html>()
                                }
                            </ul>
                            
                            <div class="pipeline-info">
                                <h4>{ "Pipeline Information" }</h4>
                                <p><strong>{ "Current Status: " }</strong>{ release.status.display_name() }</p>
                                <p><strong>{ "Skip Staging: " }</strong>{ if release.skip_staging { "Yes" } else { "No" } }</p>
                                <p><strong>{ "Target Environment: " }</strong>{ format!("{:?}", release.target_environment) }</p>
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
