use yew::prelude::*;
use web_sys::{HtmlInputElement, HtmlSelectElement};
use wasm_bindgen::JsCast;
use chrono::{Utc, TimeZone, NaiveDateTime};
use crate::models::{Release, Client, Environment, ReleaseStatus, DeploymentItem};

#[derive(Properties, PartialEq)]
pub struct ReleaseFormProps {
    pub clients: Vec<Client>,
    pub on_submit: Callback<Release>,
    pub on_cancel: Callback<()>,
}

#[function_component(ReleaseForm)]
pub fn release_form(props: &ReleaseFormProps) -> Html {
    let title_ref = use_node_ref();
    let client_ref = use_node_ref();
    let current_env_ref = use_node_ref();
    let target_env_ref = use_node_ref();
    let scheduled_date_ref = use_node_ref();
    let scheduled_time_ref = use_node_ref();
    
    // Deployment item checkboxes state
    let data_checked = use_state(|| true);
    let solr_checked = use_state(|| true);
    let app_checked = use_state(|| true);
    
    let on_data_change = {
        let data_checked = data_checked.clone();
        Callback::from(move |_| {
            data_checked.set(!*data_checked);
        })
    };
    
    let on_solr_change = {
        let solr_checked = solr_checked.clone();
        Callback::from(move |_| {
            solr_checked.set(!*solr_checked);
        })
    };
    
    let on_app_change = {
        let app_checked = app_checked.clone();
        Callback::from(move |_| {
            app_checked.set(!*app_checked);
        })
    };
    
    let on_submit = {
        let title_ref = title_ref.clone();
        let client_ref = client_ref.clone();
        let current_env_ref = current_env_ref.clone();
        let target_env_ref = target_env_ref.clone();
        let scheduled_date_ref = scheduled_date_ref.clone();
        let scheduled_time_ref = scheduled_time_ref.clone();
        
        let data_checked = data_checked.clone();
        let solr_checked = solr_checked.clone();
        let app_checked = app_checked.clone();
        
        let callback = props.on_submit.clone();
        
        Callback::from(move |e: SubmitEvent| {
            e.prevent_default();
            
            // Get form values
            let title = title_ref.cast::<HtmlInputElement>()
                .map(|input| input.value())
                .unwrap_or_default();
                
            let client_id = client_ref.cast::<HtmlSelectElement>()
                .map(|select| select.value())
                .unwrap_or_default();
                
            let current_env_str = current_env_ref.cast::<HtmlSelectElement>()
                .map(|select| select.value())
                .unwrap_or_default();
                
            let target_env_str = target_env_ref.cast::<HtmlSelectElement>()
                .map(|select| select.value())
                .unwrap_or_default();
                
            let date_str = scheduled_date_ref.cast::<HtmlInputElement>()
                .map(|input| input.value())
                .unwrap_or_default();
                
            let time_str = scheduled_time_ref.cast::<HtmlInputElement>()
                .map(|input| input.value())
                .unwrap_or_default();
            
            // Parse environments
            let current_env = match current_env_str.as_str() {
                "Development" => Environment::Development,
                "Staging" => Environment::Staging,
                "Production" => Environment::Production,
                _ => Environment::Development,
            };
            
            let target_env = match target_env_str.as_str() {
                "Development" => Environment::Development,
                "Staging" => Environment::Staging,
                "Production" => Environment::Production,
                _ => Environment::Production,
            };
            
            // Collect deployment items
            let mut deployment_items = Vec::new();
            if *data_checked {
                deployment_items.push("data".to_string());
            }
            if *solr_checked {
                deployment_items.push("solr".to_string());
            }
            if *app_checked {
                deployment_items.push("app".to_string());
            }
            
            // Parse scheduled date and time
            let datetime_str = format!("{}T{}:00Z", date_str, time_str);
            let scheduled_at = match NaiveDateTime::parse_from_str(&datetime_str, "%Y-%m-%dT%H:%M:%SZ") {
                Ok(dt) => Utc.from_utc_datetime(&dt),
                Err(_) => Utc::now(), // Default to now if parsing fails
            };
            
            // Create deployment items
            let deployment_items = deployment_items.into_iter()
                .map(|name| DeploymentItem {
                    name,
                    status: ReleaseStatus::Pending,
                    logs: Vec::new(),
                    error: None,
                })
                .collect();
            
            // Create release
            let release = Release {
                id: "temp".to_string(), // Will be assigned by backend
                title,
                client_id,
                current_environment: current_env,
                target_environment: target_env,
                deployment_items,
                created_at: Utc::now(),
                scheduled_at,
                status: ReleaseStatus::Pending,
                created_by: "current_user".to_string(), // Will be assigned by backend
                progress: 0.0,
            };
            
            callback.emit(release);
        })
    };
    
    let on_cancel = {
        let callback = props.on_cancel.clone();
        Callback::from(move |_| {
            callback.emit(());
        })
    };
    
    // Set default values for date and time inputs
    let now = Utc::now();
    let default_date = now.format("%Y-%m-%d").to_string();
    let default_time = now.format("%H:%M").to_string();
    
    html! {
        <div class="release-form">
            <h2>{ "Create New Release" }</h2>
            
            <form onsubmit={on_submit}>
                <div class="form-group">
                    <label for="title">{ "Title" }</label>
                    <input 
                        ref={title_ref}
                        id="title"
                        type="text"
                        required=true
                    />
                </div>
                
                <div class="form-group">
                    <label for="client">{ "Client" }</label>
                    <select 
                        ref={client_ref}
                        id="client"
                        required=true
                    >
                        <option value="">{ "-- Select Client --" }</option>
                        {
                            props.clients.iter().map(|client| {
                                html! {
                                    <option value={client.id.clone()}>
                                        { &client.name }
                                    </option>
                                }
                            }).collect::<Html>()
                        }
                    </select>
                </div>
                
                <div class="form-group">
                    <label for="current-env">{ "Current Environment" }</label>
                    <select 
                        ref={current_env_ref}
                        id="current-env"
                        required=true
                    >
                        <option value="Development">{ "Development" }</option>
                        <option value="Staging">{ "Staging" }</option>
                    </select>
                </div>
                
                <div class="form-group">
                    <label for="target-env">{ "Target Environment" }</label>
                    <select 
                        ref={target_env_ref}
                        id="target-env"
                        required=true
                    >
                        <option value="Staging">{ "Staging" }</option>
                        <option value="Production">{ "Production" }</option>
                    </select>
                </div>
                
                <div class="form-group">
                    <label>{ "Deployment Items" }</label>
                    
                    <div class="checkbox-group">
                        <label>
                            <input 
                                type="checkbox"
                                checked={*data_checked}
                                onchange={on_data_change}
                            />
                            { "Data" }
                        </label>
                        
                        <label>
                            <input 
                                type="checkbox"
                                checked={*solr_checked}
                                onchange={on_solr_change}
                            />
                            { "Solr" }
                        </label>
                        
                        <label>
                            <input 
                                type="checkbox"
                                checked={*app_checked}
                                onchange={on_app_change}
                            />
                            { "App" }
                        </label>
                    </div>
                </div>
                
                <div class="form-group">
                    <label for="scheduled-date">{ "Scheduled Date" }</label>
                    <input 
                        ref={scheduled_date_ref}
                        id="scheduled-date"
                        type="date"
                        value={default_date}
                        required=true
                    />
                </div>
                
                <div class="form-group">
                    <label for="scheduled-time">{ "Scheduled Time" }</label>
                    <input 
                        ref={scheduled_time_ref}
                        id="scheduled-time"
                        type="time"
                        value={default_time}
                        required=true
                    />
                </div>
                
                <div class="form-actions">
                    <button type="button" class="cancel-btn" onclick={on_cancel}>
                        { "Cancel" }
                    </button>
                    <button type="submit" class="submit-btn">
                        { "Create Release" }
                    </button>
                </div>
            </form>
        </div>
    }
}
