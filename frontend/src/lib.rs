use wasm_bindgen::prelude::*;
use yew::prelude::*;

mod app;
mod components;
mod models;
mod services;

use app::App;

#[wasm_bindgen(start)]
pub fn run_app() -> Result<(), JsValue> {
    wasm_logger::init(wasm_logger::Config::default());
    console_error_panic_hook::set_once();
    
    yew::Renderer::<App>::new().render();
    
    Ok(())
}
