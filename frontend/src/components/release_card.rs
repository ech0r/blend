use yew::prelude::*;
use crate::models::{Release, Environment, ReleaseStatus};
use web_sys::{DragEvent, DataTransfer};
use wasm_bindgen::JsCast;

#[derive(Properties, PartialEq)]
pub struct ReleaseCardProps {
    pub release: Release,
    pub on_delete: Callback<String>,
    pub on_move: Callback<(String, Environment)>,
}

#[function_component(ReleaseCard)]
pub fn release_card(props: &ReleaseCardProps) -> Html {
    let release = &props.release;
    let show_details = use_state(|| false);
    
    let on_details_click = {
        let show_details = show_details.clone();
        Callback::from(move |_| {
            show_details.set(!*show_details);
        })
    };
    
    let on_delete = {
        let id = release.id.clone();
        let callback = props.on_delete.clone();
        Callback::from(move |_| {
            callback.emit(id.clone());
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
        let callback = props.on_move.clone();
        let current_env = release.current_environment.clone();
        
        Callback::from(move |_| {
            // Determine next environment
            let next_env = match current_env {
                Environment::Development => Environment::Staging,
                Environment::Staging => Environment::Production,
                Environment::Production => return, // No next environment
            };
            
            callback.emit((id.clone(), next_env));
        })
    };
    
    // Card styling based on status
    let status_class = match release.status {
        ReleaseStatus::Pending => "status-pending",
        ReleaseStatus::InProgress => "status-in-progress",
        ReleaseStatus::Completed => "status-completed",
        ReleaseStatus::Failed => "status-failed",
    };
    
    html! {
        <div 
            class={classes!("release-card", status_class)}
            draggable="true"
            ondragstart={on_drag_start}
        >
            <div class="card-header">
                <h3 class="release-title">{ &release.title }</h3>
                <div class="release-status">
                    <span class={status_class}>{ format!("{:?}", release.status) }</span>
                </div>
            </div>
            
            <div class="release-info">
                <p class="client-name">{ format!("Client: {}", release.client_id) }</p>
                <p class="scheduled-time">
                    { format!("Scheduled: {}", release.scheduled_at.format("%Y-%m-%d %H:%M")) }
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
                    // Only show clear button if status is completed
                    if release.status == ReleaseStatus::Completed {
                        html! {
                            <button class="clear-btn" onclick={on_clear}>
                                { "Clear & Move" }
                            </button>
                        }
                    } else {
                        html! {}
                    }
                }
                
                <button class="delete-btn" onclick={on_delete}>
                    { "Delete" }
                </button>
            </div>
            
            {
                if *show_details {
                    html! {
                        <div class="release-details">
                            <h4>{ "Deployment Items" }</h4>
                            <ul class="deployment-items">
                                {
                                    release.deployment_items.iter().map(|item| {
                                        let item_status_class = match item.status {
                                            ReleaseStatus::Pending => "status-pending",
                                            ReleaseStatus::InProgress => "status-in-progress",
                                            ReleaseStatus::Completed => "status-completed",
                                            ReleaseStatus::Failed => "status-failed",
                                        };
                                        
                                        html! {
                                            <li class={classes!("deployment-item", item_status_class)}>
                                                <span class="item-name">{ &item.name }</span>
                                                <span class="item-status">
                                                    { format!("{:?}", item.status) }
                                                </span>
                                                
                                                {
                                                    if !item.logs.is_empty() {
                                                        html! {
                                                            <div class="logs">
                                                                <h5>{ "Logs" }</h5>
                                                                <pre class="log-output">
                                                                    {
                                                                        for item.logs.iter().map(|log| {
                                                                            html! {
                                                                                <>
                                                                                    { log }
                                                                                    <br />
                                                                                </>
                                                                            }
                                                                        })
                                                                    }
                                                                </pre>
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
                                                                <p>{ format!("Error: {}", error) }</p>
                                                            </div>
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
                        </div>
                    }
                } else {
                    html! {}
                }
            }
        </div>
    }
}
