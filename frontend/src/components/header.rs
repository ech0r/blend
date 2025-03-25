use yew::prelude::*;
use crate::models::User;

#[derive(Properties, PartialEq)]
pub struct HeaderProps {
    pub user: Option<User>,
    pub on_new_release: Callback<()>,
    pub on_toggle_chat: Callback<()>,
    pub on_toggle_log: Callback<()>, // New property
    pub is_connected: bool,
}

#[function_component(Header)]
pub fn header(props: &HeaderProps) -> Html {
    let on_new_release = {
        let callback = props.on_new_release.clone();
        Callback::from(move |_| {
            callback.emit(());
        })
    };
    
    let on_toggle_chat = {
        let callback = props.on_toggle_chat.clone();
        Callback::from(move |_| {
            callback.emit(());
        })
    };
    
    // Add handler for the new button
    let on_toggle_log = {
        let callback = props.on_toggle_log.clone();
        Callback::from(move |_| {
            callback.emit(());
        })
    };
    
    html! {
        <header class="app-header">
            <div class="logo">
                <h1>{ "Blend Release Manager" }</h1>
            </div>
            
            <div class="connection-status">
                {
                    if props.is_connected {
                        html! { <span class="connected">{ "Connected" }</span> }
                    } else {
                        html! { <span class="disconnected">{ "Disconnected" }</span> }
                    }
                }
            </div>
            
            <div class="actions">
                <button 
                    class="new-release-btn"
                    onclick={on_new_release}
                >
                    { "New Release" }
                </button>
                
                <button 
                    class="toggle-chat-btn"
                    onclick={on_toggle_chat}
                >
                    { "Toggle Chat" }
                </button>
                
                // Add new button
                <button 
                    class="toggle-log-btn"
                    onclick={on_toggle_log}
                >
                    { "Blend Log" }
                </button>
            </div>
            
            <div class="user-info">
                {
                    if let Some(user) = &props.user {
                        html! {
                            <>
                                <img src={user.avatar_url.clone()} alt="Avatar" class="avatar" />
                                <span class="username">{ &user.username }</span>
                            </>
                        }
                    } else {
                        html! {
                            <a href="/auth/github/login" class="login-btn">
                                { "Login with GitHub" }
                            </a>
                        }
                    }
                }
            </div>
        </header>
    }
}
