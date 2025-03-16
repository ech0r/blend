use actix_web::{web, HttpResponse, Responder, get};
use crate::storage::SledStorage;
use crate::models::User;
use log::error;

#[get("")]
async fn get_users(db: web::Data<SledStorage>) -> impl Responder {
    match db.get_all_users() {
        Ok(users) => {
            // For security, don't return access tokens
            let sanitized_users: Vec<_> = users.into_iter().map(|mut user| {
                user.access_token = "".to_string();
                user
            }).collect();
            
            HttpResponse::Ok().json(sanitized_users)
        },
        Err(e) => {
            error!("Failed to get users: {}", e);
            HttpResponse::InternalServerError().body(format!("Failed to get users: {}", e))
        }
    }
}

#[get("/me")]
async fn get_current_user(db: web::Data<SledStorage>) -> impl Responder {
    // In a real implementation, we would get the user ID from the session
    // and then fetch the user from the database
    HttpResponse::Ok().body("Current user info")
}

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(get_users)
        .service(get_current_user);
}
