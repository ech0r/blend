use yew::prelude::*;
use web_sys::HtmlInputElement;
use wasm_bindgen::JsCast;
use crate::models::WsMessage;
use chrono::{DateTime, Local};

#[derive(Properties, PartialEq)]
pub struct ChatPanelProps {
    pub messages: Vec<WsMessage>,
    pub on_send: Callback<String>,
}

#[function_component(ChatPanel)]
pub fn chat_panel(props: &ChatPanelProps) -> Html {
    let message_input_ref = use_node_ref();
    
    let on_submit = {
        let input_ref = message_input_ref.clone();
        let on_send = props.on_send.clone();
        
        Callback::from(move |e: SubmitEvent| {
            e.prevent_default();
            
            if let Some(input) = input_ref.cast::<HtmlInputElement>() {
                let message = input.value();
                
                if !message.trim().is_empty() {
                    on_send.emit(message);
                    input.set_value("");
                }
            }
        })
    };
    
    html! {
        <div class="chat-panel">
            <div class="chat-header">
                <h3>{ "Chat" }</h3>
            </div>
            
            <div class="chat-messages">
                {
                    props.messages.iter().map(|msg| {
                        match msg {
                            WsMessage::Chat { username, message, timestamp } => {
                                let timestamp = DateTime::parse_from_rfc3339(timestamp).expect("Failed to parse datetime");
                                let local_datetime = timestamp.with_timezone(&Local).format("%Y-%m-%d %H:%M");
                                html! {
                                    <div class="chat-message">
                                        <div class="message-header">
                                            <span class="username">{ username }</span>
                                            <span class="timestamp">{ local_datetime }</span>
                                        </div>
                                        <div class="message-content">
                                            { message }
                                        </div>
                                    </div>
                                }
                            }
                            _ => html! {}
                        }
                    }).collect::<Html>()
                }
            </div>
            
            <form class="chat-input-form" onsubmit={on_submit}>
                <input 
                    ref={message_input_ref}
                    type="text"
                    placeholder="Type a message..."
                    class="chat-input"
                />
                <button type="submit" class="send-btn">
                    { "Send" }
                </button>
            </form>
        </div>
    }
}
