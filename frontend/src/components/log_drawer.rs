use yew::prelude::*;
use std::rc::Rc;

#[derive(Debug, Clone, PartialEq)]
pub struct LogEntry {
    pub release_id: String,
    pub item_name: String,
    pub content: String,
    pub timestamp: String,
    pub is_error: bool,
}

#[derive(Properties, PartialEq)]
pub struct LogDrawerProps {
    pub visible: bool,
    pub logs: Rc<Vec<LogEntry>>,
    pub release_id: String,
    pub release_title: String,
    pub on_close: Callback<()>,
}

#[function_component(LogDrawer)]
pub fn log_drawer(props: &LogDrawerProps) -> Html {
    let active_tab = use_state(|| "all".to_string());
    let auto_scroll = use_state(|| true);
    let scroll_ref = use_node_ref();
    
    // Define log drawer classes based on visibility
    let drawer_class = if props.visible {
        "log-drawer log-drawer-visible"
    } else {
        "log-drawer"
    };
    
    // When auto-scroll is enabled, scroll to bottom whenever logs change
    use_effect_with_deps(
        move |(logs, auto_scroll, scroll_ref)| {
            if **auto_scroll && scroll_ref.get().is_some() {
                if let Some(div) = scroll_ref.cast::<web_sys::HtmlElement>() {
                    div.scroll_to_with_x_and_y(0.0, f64::MAX);
                }
            }
            || ()
        },
        (Rc::clone(&props.logs), auto_scroll.clone(), scroll_ref.clone()),
    );
    
    // Toggle auto-scroll
    let on_toggle_auto_scroll = {
        let auto_scroll = auto_scroll.clone();
        Callback::from(move |_| {
            auto_scroll.set(!*auto_scroll);
        })
    };
    
    // Close the drawer
    let on_close = {
        let on_close = props.on_close.clone();
        Callback::from(move |_| {
            on_close.emit(());
        })
    };
    
    // Filter logs by active tab and current release
    let filtered_logs = {
        let tab = (*active_tab).clone();
        
        props.logs.iter()
            .filter(|log| log.release_id == props.release_id)
            .filter(|log| tab == "all" || log.item_name == tab)
            .collect::<Vec<_>>()
    };
    
    // Set active tab
    let on_set_tab = {
        let active_tab = active_tab.clone();
        Callback::from(move |tab: String| {
            active_tab.set(tab);
        })
    };
    
    html! {
        <div class={drawer_class}>
            <div class="log-drawer-header">
                <div class="log-drawer-title">
                    <h3>{ format!("Logs for {}", props.release_title) }</h3>
                    <button class="close-btn" onclick={on_close}>
                        { "×" }
                    </button>
                </div>
                
                <div class="log-drawer-tabs">
                    <button 
                        class={if *active_tab == "all" { "tab-button active" } else { "tab-button" }}
                        onclick={on_set_tab.reform(|_| "all".to_string())}
                    >
                        { "All" }
                    </button>
                    <button 
                        class={if *active_tab == "data" { "tab-button active" } else { "tab-button" }}
                        onclick={on_set_tab.reform(|_| "data".to_string())}
                    >
                        { "Data" }
                    </button>
                    <button 
                        class={if *active_tab == "solr" { "tab-button active" } else { "tab-button" }}
                        onclick={on_set_tab.reform(|_| "solr".to_string())}
                    >
                        { "Solr" }
                    </button>
                    <button 
                        class={if *active_tab == "app" { "tab-button active" } else { "tab-button" }}
                        onclick={on_set_tab.reform(|_| "app".to_string())}
                    >
                        { "App" }
                    </button>
                    
                    <div class="tab-options">
                        <label class="checkbox-label">
                            <input 
                                type="checkbox"
                                checked={*auto_scroll}
                                onchange={on_toggle_auto_scroll}
                            />
                            { "Auto-scroll" }
                        </label>
                    </div>
                </div>
            </div>
            
            <div class="log-drawer-content" ref={scroll_ref}>
                <div class="log-entries">
                    {
                        if filtered_logs.is_empty() {
                            html! {
                                <div class="empty-logs">
                                    <p>{ "No logs to display for this component." }</p>
                                </div>
                            }
                        } else {
                            html! {
                                <>
                                    {
                                        filtered_logs.iter().enumerate().map(|(index, log)| {
                                            let log_class = if log.is_error {
                                                "log-entry error"
                                            } else {
                                                "log-entry"
                                            };
                                            
                                            html! {
                                                <div class={log_class} style={format!("--index: {}", index)}>
                                                    <span class="log-timestamp">{ &log.timestamp }</span>
                                                    <span class="log-content">{ &log.content }</span>
                                                </div>
                                            }
                                        }).collect::<Html>()
                                    }
                                </>
                            }
                        }
                    }
                </div>
            </div>
        </div>
    }
}
