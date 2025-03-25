use actix_web::{web, HttpResponse, Responder, get, HttpRequest};
use crate::storage::SledStorage;
use crate::models::User;
use log::{error, debug};

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
async fn get_current_user(
    req: HttpRequest,
    db: web::Data<SledStorage>,
) -> impl Responder {
    // Get session cookie
    if let Some(cookie) = req.cookie("session_id") {
        let session_id = cookie.value();
        
        // Get user ID from session
        match db.get_session(session_id) {
            Ok(Some(user_id)) => {
                // Get user from database
                match db.get_user(&user_id) {
                    Ok(Some(user)) => {
                        // Create sanitized user (without access token)
                        let sanitized_user = User {
                            id: user.id,
                            username: user.username,
                            avatar_url: user.avatar_url,
                            access_token: String::new(), // Don't expose token
                            role: user.role,
                        };
                        
                        return HttpResponse::Ok().json(sanitized_user);
                    }
                    Ok(None) => {
                        error!("Session exists but user not found: {}", user_id);
                    }
                    Err(e) => {
                        error!("Error fetching user from session: {}", e);
                    }
                }
            }
            Ok(None) => {
                debug!("No session found for id: {}", session_id);
            }
            Err(e) => {
                error!("Error fetching session: {}", e);
            }
        }
    }
    
    // If we get here, there's no valid session
    HttpResponse::Unauthorized().json(serde_json::json!({
        "error": "Not authenticated"
    }))
}

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(get_users)
        .service(get_current_user);
}
