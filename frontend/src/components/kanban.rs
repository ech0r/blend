use yew::prelude::*;
use web_sys::{DragEvent, DataTransfer};
use crate::models::{Release, Environment, ReleaseStatus, User, UserRole};
use super::release_card::ReleaseCard;

#[derive(Properties, PartialEq)]
pub struct KanbanBoardProps {
    pub releases: Vec<Release>,
    pub current_user: Option<User>, // Add current user
    pub on_move_release: Callback<(String, Environment)>,
    pub on_clear_release: Callback<String>,
    pub on_delete_release: Callback<String>,
    pub on_view_logs: Callback<String>,
}

#[function_component(KanbanBoard)]
pub fn kanban_board(props: &KanbanBoardProps) -> Html {
    let on_delete = {
        let callback = props.on_delete_release.clone();
        Callback::from(move |id: String| {
            callback.emit(id);
        })
    };
    
    let on_move = {
        let callback = props.on_move_release.clone();
        Callback::from(move |(id, env): (String, Environment)| {
            callback.emit((id, env));
        })
    };
    
    let on_clear = {
        let callback = props.on_clear_release.clone();
        Callback::from(move |id: String| {
            callback.emit(id);
        })
    };
    
    let on_view_logs = {
        let callback = props.on_view_logs.clone();
        Callback::from(move |id: String| {
            callback.emit(id);
        })
    };
    
    let on_drag_over = Callback::from(|event: DragEvent| {
        event.prevent_default();
    });
    
    // Add permission checks for drag and drop
    let can_deploy_to_staging = props.current_user.as_ref()
        .map(|user| user.can_deploy_to_staging())
        .unwrap_or(false);
        
    let can_deploy_to_production = props.current_user.as_ref()
        .map(|user| user.can_deploy_to_production())
        .unwrap_or(false);
    
    // Only allow drops if user has permissions
    let on_dev_drop = {
        let callback = props.on_move_release.clone();
        Callback::from(move |event: DragEvent| {
            event.prevent_default();
            // Always allowed to move to dev
            if let Some(data) = event.data_transfer() {
                if let Ok(id) = data.get_data("text/plain") {
                    callback.emit((id, Environment::Development));
                }
            }
        })
    };
    
    let on_staging_drop = {
        let callback = props.on_move_release.clone();
        let can_deploy = can_deploy_to_staging;
        Callback::from(move |event: DragEvent| {
            event.prevent_default();
            // Only if user can deploy to staging
            if can_deploy {
                if let Some(data) = event.data_transfer() {
                    if let Ok(id) = data.get_data("text/plain") {
                        callback.emit((id, Environment::Staging));
                    }
                }
            }
        })
    };
    
    let on_prod_drop = {
        let callback = props.on_move_release.clone();
        let can_deploy = can_deploy_to_production;
        Callback::from(move |event: DragEvent| {
            event.prevent_default();
            // Only if user can deploy to production
            if can_deploy {
                if let Some(data) = event.data_transfer() {
                    if let Ok(id) = data.get_data("text/plain") {
                        callback.emit((id, Environment::Production));
                    }
                }
            }
        })
    };
    
    // Filter releases by their board column (determined by status)
    let dev_releases = props.releases.iter()
        .filter(|r| r.current_board_column() == Environment::Development)
        .cloned()
        .collect::<Vec<_>>();
        
    let staging_releases = props.releases.iter()
        .filter(|r| r.current_board_column() == Environment::Staging)
        .cloned()
        .collect::<Vec<_>>();
        
    let prod_releases = props.releases.iter()
        .filter(|r| r.current_board_column() == Environment::Production)
        .cloned()
        .collect::<Vec<_>>();
    
    html! {
        <div class="kanban-board">
            <div 
                class="environment-column development"
                ondragover={on_drag_over.clone()}
                ondrop={on_dev_drop}
            >
                <div class="environment-header">
                    <h2>{ "Development" }</h2>
                    <span class="count">{ dev_releases.len() }</span>
                </div>
                
                <div class="column-content">
                    {
                        dev_releases.iter().map(|release| {
                            html! {
                                <ReleaseCard 
                                    release={release.clone()}
                                    current_user={props.current_user.clone()}
                                    on_delete={on_delete.clone()}
                                    on_move={on_move.clone()}
                                    on_clear={on_clear.clone()}
                                    on_view_logs={on_view_logs.clone()}
                                />
                            }
                        }).collect::<Html>()
                    }
                </div>
            </div>
            
            <div 
                class="environment-column staging"
                ondragover={on_drag_over.clone()}
                ondrop={on_staging_drop}
            >
                <div class="environment-header">
                    <h2>{ "Staging" }</h2>
                    <span class="count">{ staging_releases.len() }</span>
                </div>
                
                <div class="column-content">
                    {
                        staging_releases.iter().map(|release| {
                            html! {
                                <ReleaseCard 
                                    release={release.clone()}
                                    current_user={props.current_user.clone()}
                                    on_delete={on_delete.clone()}
                                    on_move={on_move.clone()}
                                    on_clear={on_clear.clone()}
                                    on_view_logs={on_view_logs.clone()}
                                />
                            }
                        }).collect::<Html>()
                    }
                </div>
            </div>
            
            <div 
                class="environment-column production"
                ondragover={on_drag_over}
                ondrop={on_prod_drop}
            >
                <div class="environment-header">
                    <h2>{ "Production" }</h2>
                    <span class="count">{ prod_releases.len() }</span>
                </div>
                
                <div class="column-content">
                    {
                        prod_releases.iter().map(|release| {
                            html! {
                                <ReleaseCard 
                                    release={release.clone()}
                                    current_user={props.current_user.clone()}
                                    on_delete={on_delete.clone()}
                                    on_move={on_move.clone()}
                                    on_clear={on_clear.clone()}
                                    on_view_logs={on_view_logs.clone()}
                                />
                            }
                        }).collect::<Html>()
                    }
                </div>
            </div>
        </div>
    }
}
