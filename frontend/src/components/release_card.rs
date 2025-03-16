use yew::prelude::*;
use crate::models::{Release, Environment, ReleaseStatus};
use web_sys::{DragEvent, DataTransfer};
use wasm_bindgen::JsCast;

#[derive(Properties, PartialEq)]
pub struct ReleaseCardProps {
    pub release: Release,
    pub on_delete: Callback<String>,
    pub on_move: Callback<(String, Environment)>,
    pub on_clear: Callback<String>, // New clear callback
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
        let callback = props.on_clear.clone();
        
        Callback::from(move |_| {
            callback.emit(id.clone());
        })
    };
    
    // Get status class and display name
    let status_class = release.status.css_class();
    let status_display = release.status.display_name();
    
    // Determine if "Clear & Move" button should be shown
    let can_be_cleared = release.can_be_cleared();
    
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
                    // Only show clear button if release can be cleared
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
                                        let item_status_class = item.status.css_class();
                                        
                                        html! {
                                            <li class={classes!("deployment-item", item_status_class)}>
                                                <span class="item-name">{ &item.name }</span>
                                                <span class="item-status">
                                                    { item.status.display_name() }
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
