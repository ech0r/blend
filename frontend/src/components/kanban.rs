use yew::prelude::*;
use web_sys::{DragEvent, DataTransfer};
use crate::models::{Release, Environment};
use super::release_card::ReleaseCard;

#[derive(Properties, PartialEq)]
pub struct KanbanBoardProps {
    pub dev_releases: Vec<Release>,
    pub staging_releases: Vec<Release>,
    pub prod_releases: Vec<Release>,
    pub on_move_release: Callback<(String, Environment)>,
    pub on_delete_release: Callback<String>,
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
    
    let on_drag_over = Callback::from(|event: DragEvent| {
        event.prevent_default();
    });
    
    let on_dev_drop = {
        let callback = props.on_move_release.clone();
        Callback::from(move |event: DragEvent| {
            event.prevent_default();
            if let Some(data) = event.data_transfer() {
                if let Ok(id) = data.get_data("text/plain") {
                    callback.emit((id, Environment::Development));
                }
            }
        })
    };
    
    let on_staging_drop = {
        let callback = props.on_move_release.clone();
        Callback::from(move |event: DragEvent| {
            event.prevent_default();
            if let Some(data) = event.data_transfer() {
                if let Ok(id) = data.get_data("text/plain") {
                    callback.emit((id, Environment::Staging));
                }
            }
        })
    };
    
    let on_prod_drop = {
        let callback = props.on_move_release.clone();
        Callback::from(move |event: DragEvent| {
            event.prevent_default();
            if let Some(data) = event.data_transfer() {
                if let Ok(id) = data.get_data("text/plain") {
                    callback.emit((id, Environment::Production));
                }
            }
        })
    };
    
    html! {
        <div class="kanban-board">
            <div 
                class="environment-column development"
                ondragover={on_drag_over.clone()}
                ondrop={on_dev_drop}
            >
                <div class="environment-header">
                    <h2>{ "Development" }</h2>
                    <span class="count">{ props.dev_releases.len() }</span>
                </div>
                
                <div class="column-content">
                    {
                        props.dev_releases.iter().map(|release| {
                            html! {
                                <ReleaseCard 
                                    release={release.clone()}
                                    on_delete={on_delete.clone()}
                                    on_move={on_move.clone()}
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
                    <span class="count">{ props.staging_releases.len() }</span>
                </div>
                
                <div class="column-content">
                    {
                        props.staging_releases.iter().map(|release| {
                            html! {
                                <ReleaseCard 
                                    release={release.clone()}
                                    on_delete={on_delete.clone()}
                                    on_move={on_move.clone()}
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
                    <span class="count">{ props.prod_releases.len() }</span>
                </div>
                
                <div class="column-content">
                    {
                        props.prod_releases.iter().map(|release| {
                            html! {
                                <ReleaseCard 
                                    release={release.clone()}
                                    on_delete={on_delete.clone()}
                                    on_move={on_move.clone()}
                                />
                            }
                        }).collect::<Html>()
                    }
                </div>
            </div>
        </div>
    }
}
