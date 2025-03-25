use yew::prelude::*;
use web_sys::{HtmlInputElement, HtmlSelectElement};
use log::{info, debug, error};
use wasm_bindgen::JsCast;
use chrono::{Utc, TimeZone, Local, NaiveDateTime};
use crate::models::{Release, Client, Environment, ReleaseStatus, DeploymentItem};

#[derive(Properties, PartialEq)]
pub struct ReleaseFormProps {
    pub clients: Vec<Client>,
    pub on_submit: Callback<Release>,
    pub on_cancel: Callback<()>,
    pub on_create: Callback<String>,
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

    // Release date and time
    let now = Utc::now();
    let scheduled_date = use_state(|| now.format("%Y-%m-%d").to_string());
    let scheduled_time = use_state(|| now.format("%H:%M").to_string());
    
    // Skip staging option state
    let skip_staging = use_state(|| false);
    let show_skip_staging = use_state(|| false);
    
    let current_env = use_state(|| "Development".to_string());
    let target_env = use_state(|| "Staging".to_string());
    
    let on_current_env_change = {
        let target_env = target_env.clone();
        let current_env = current_env.clone();
        let show_skip_staging = show_skip_staging.clone();
        Callback::from(move |e: Event| {
            if let Some(select) = e.target_dyn_into::<HtmlSelectElement>() {
                info!("selected value: {}", select.value());
                current_env.set(select.value());
                // Determine if we should show the skip staging option
                show_skip_staging.set(select.value() == "Development" && *target_env == "Production");
                info!("show_skip_staging: {}", *show_skip_staging);
                debug!("current_env: {}", select.value());
                debug!("target_env: {}", *target_env);
            }
        })
    };
    
    let on_target_env_change = {
        let target_env = target_env.clone();
        let current_env = current_env.clone();
        let show_skip_staging = show_skip_staging.clone();
        Callback::from(move |e: Event| {
            if let Some(select) = e.target_dyn_into::<HtmlSelectElement>() {
                info!("selected value: {}", select.value());
                target_env.set(select.value());
                show_skip_staging.set(*current_env == "Development" && select.value() == "Production");
                // Determine if we should show the skip staging option
                info!("show_skip_staging: {}", *show_skip_staging);
                debug!("current_env: {}", *current_env);
                debug!("target_env: {}", select.value());
            }
        })
    };

    let on_skip_staging_change = {
        let skip_staging = skip_staging.clone();
        Callback::from(move |e: Event| {
            if let Some(checkbox) = e.target_dyn_into::<HtmlInputElement>() {            
                skip_staging.set(checkbox.checked());
            }
            info!("skip_staging: {}", *skip_staging);
        })
    };
    
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
    
    //let on_skip_staging_change = {
    //    let skip_staging = skip_staging.clone();
    //    Callback::from(move |_| {
    //        skip_staging.set(!*skip_staging);
    //    })
    //};

    let on_create = {
        let callback = props.on_create.clone();
        Callback::from(move |_|{
            let success = "Release successfully created!".to_owned();
            callback.emit(success);
        })
    };

    let get_date = {
        let scheduled_date_ref = scheduled_date_ref.clone();
        let datetime = Local::now();
        let today = datetime.date_naive();
        Callback::from(move |_| {
            if let Some(date_input) = scheduled_date_ref.cast::<HtmlInputElement>() {
                date_input.set_value(&today.to_string())
            }
            info!("Today: {}", today);
        })
    };
    
    let get_time = {
        let scheduled_time_ref = scheduled_time_ref.clone();
        let datetime = Local::now(); 
        let time = datetime.time();
        let hours_minutes = time.format("%H:%M");
        Callback::from(move |_| {
            if let Some(time_input) = scheduled_time_ref.cast::<HtmlInputElement>() {
                time_input.set_value(&hours_minutes.to_string())
            }
            info!("Now: {}", now);
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
        let skip_staging = skip_staging.clone();
        
        let callback = props.on_submit.clone();
        
        Callback::from(move |e: SubmitEvent| {
            info!("release submitted");
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
                    status: ReleaseStatus::InDevelopment,
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
                status: ReleaseStatus::InDevelopment,
                created_by: "current_user".to_string(), // Will be assigned by backend
                progress: 0.0,
                skip_staging: *skip_staging, // Add the skip_staging flag
            };
            info!("{:?}", &release);
            callback.emit(release);
        })
    };

    let on_cancel = {
        let callback = props.on_cancel.clone();
        Callback::from(move |_| {
            callback.emit(());
        })
    };
    
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
                        onchange={on_current_env_change}
                        value={(*current_env).clone()}
                    >
                        <option selected=true value="Development">{ "Development" }</option>
                        <option value="Staging">{ "Staging" }</option>
                    </select>
                </div>
                
                <div class="form-group">
                    <label for="target-env">{ "Target Environment (Final Destination)" }</label>
                    <select 
                        ref={target_env_ref}
                        id="target-env"
                        required=true
                        onchange={on_target_env_change}
                        value={(*target_env).clone()}
                    >
                        <option selected=true value="Staging">{ "Staging" }</option>
                        <option value="Production">{ "Production" }</option>
                    </select>
                </div>
                
                {
                    // Only show "Skip Staging" option when current is Development and target is Production
                    if *show_skip_staging {
                        html! {
                            <div class="form-group">
                                <div class="checkbox-group">
                                    <label class="checkbox-label">
                                        { "Skip Staging (Deploy directly to Production)" }
                                        <input 
                                            type="checkbox"
                                            onchange={on_skip_staging_change}
                                        />
                                    </label>
                                </div>
                                <p class="help-text">
                                    { if *skip_staging {
                                        "This release will go directly from Development to Production."
                                      } else {
                                        "This release will go from Development to Staging first, then to Production."
                                      }
                                    }
                                </p>
                            </div>
                        }
                    } else {
                        html! {}
                    }
                }
                
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
                        value={(*scheduled_date).clone()}
                        required=true
                    />
                    <button type="button" class="form-btn" onclick={get_date}>
                        { "Today" }
                    </button>
                </div>
                
                <div class="form-group">
                    <label for="scheduled-time">{ "Scheduled Time (Local for you)" }</label>
                    <input 
                        ref={scheduled_time_ref}
                        id="scheduled-time"
                        type="time"
                        value={(*scheduled_time).clone()}
                        required=true
                    />
                    <button type="button" class="form-btn" onclick={get_time}>
                        { "Now" }
                    </button>
                </div>
                
                <div class="form-actions">
                    <button type="button" class="cancel-btn" onclick={on_cancel}>
                        { "Cancel" }
                    </button>
                    //<button type="submit" class="submit-btn">
                    <button class="submit-btn" onclick={on_create}>
                        { "Create Release" }
                    </button>
                </div>
            </form>
        </div>
    }
}
