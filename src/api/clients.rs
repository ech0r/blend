use actix_web::{web, HttpResponse, Responder, get, post};
use crate::storage::SledStorage;
use crate::models::Client;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use log::error;

#[derive(Debug, Deserialize)]
pub struct CreateClientRequest {
    pub name: String,
}

#[derive(Debug, Serialize)]
pub struct ClientResponse {
    pub success: bool,
    pub message: Option<String>,
    pub data: Option<Client>,
}

#[get("")]
async fn get_clients(db: web::Data<SledStorage>) -> impl Responder {
    match db.get_all_clients() {
        Ok(clients) => HttpResponse::Ok().json(clients),
        Err(e) => {
            error!("Failed to get clients: {}", e);
            HttpResponse::InternalServerError().json(ClientResponse {
                success: false,
                message: Some(format!("Failed to get clients: {}", e)),
                data: None,
            })
        }
    }
}

#[post("")]
async fn create_client(
    db: web::Data<SledStorage>,
    client_data: web::Json<CreateClientRequest>,
) -> impl Responder {
    let client = Client {
        id: Uuid::new_v4(),
        name: client_data.name.clone(),
    };
    
    match db.save_client(&client) {
        Ok(_) => HttpResponse::Created().json(ClientResponse {
            success: true,
            message: Some("Client created successfully".to_string()),
            data: Some(client),
        }),
        Err(e) => {
            error!("Failed to create client: {}", e);
            HttpResponse::InternalServerError().json(ClientResponse {
                success: false,
                message: Some(format!("Failed to create client: {}", e)),
                data: None,
            })
        }
    }
}

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(get_clients)
        .service(create_client);
}
