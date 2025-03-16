use actix_web::{web, HttpResponse, Responder, get, cookie::Cookie, HttpRequest};
use oauth2::{
    basic::BasicClient, AuthUrl, TokenUrl, Scope, RedirectUrl,
    ClientId, ClientSecret, AuthorizationCode, CsrfToken,
    TokenResponse
};
use serde::{Deserialize, Serialize};
use std::env;
use uuid::Uuid;
use crate::models::User;
use crate::storage::SledStorage;
use log::{info, error};
use reqwest::Client as HttpClient;

#[derive(Debug, Deserialize)]
struct GithubUser {
    id: u64,              // GitHub returns id as a number
    login: String,
    avatar_url: String,
    // Add other fields GitHub returns that we might need
    name: Option<String>,
    email: Option<String>,
}

fn create_github_oauth_client() -> BasicClient {
    let github_client_id = env::var("GITHUB_CLIENT_ID")
        .expect("Missing the GITHUB_CLIENT_ID environment variable.");
    let github_client_secret = env::var("GITHUB_CLIENT_SECRET")
        .expect("Missing the GITHUB_CLIENT_SECRET environment variable.");
    let redirect_url = env::var("REDIRECT_URL")
        .unwrap_or_else(|_| "http://localhost:8080/auth/github/callback".to_string());

    BasicClient::new(
        ClientId::new(github_client_id),
        Some(ClientSecret::new(github_client_secret)),
        AuthUrl::new("https://github.com/login/oauth/authorize".to_string())
            .expect("Invalid authorization endpoint URL"),
        Some(TokenUrl::new("https://github.com/login/oauth/access_token".to_string())
            .expect("Invalid token endpoint URL")),
    )
    .set_redirect_uri(RedirectUrl::new(redirect_url).expect("Invalid redirect URL"))
}

#[get("/login")]
async fn github_login(req: HttpRequest) -> impl Responder {
    let client = create_github_oauth_client();

    let (auth_url, csrf_token) = client
        .authorize_url(CsrfToken::new_random)
        .add_scope(Scope::new("user:email".to_string()))
        .url();

    // Store CSRF token in a cookie
    let cookie = Cookie::build("csrf_token", csrf_token.secret().clone())
        .path("/")
        .secure(true)
        .http_only(true)
        .finish();

    HttpResponse::Found()
        .cookie(cookie)
        .append_header(("Location", auth_url.to_string()))
        .finish()
}

#[derive(Debug, Deserialize)]
pub struct AuthCallback {
    code: String,
    state: String,
}

#[get("/callback")]
async fn github_callback(
    req: HttpRequest,
    query: web::Query<AuthCallback>,
    db: web::Data<SledStorage>,
) -> impl Responder {
    // Verify CSRF token
    let csrf_cookie = req.cookie("csrf_token");
    if csrf_cookie.is_none() {
        return HttpResponse::BadRequest().body("Missing CSRF token");
    }
    
    let csrf_token = csrf_cookie.unwrap().value().to_string();
    if csrf_token != query.state {
        return HttpResponse::BadRequest().body("CSRF token mismatch");
    }
    
    // Exchange code for token
    let client = create_github_oauth_client();
    let token_result = client
        .exchange_code(AuthorizationCode::new(query.code.clone()))
        .request_async(oauth2::reqwest::async_http_client)
        .await;
        
    let token = match token_result {
        Ok(token) => token,
        Err(e) => {
            error!("Failed to exchange code for token: {}", e);
            return HttpResponse::InternalServerError().body("Failed to exchange code for token");
        }
    };
    
    // Get user info from GitHub API
    let access_token = token.access_token().secret();
    let client = HttpClient::new();
    let github_user_response = client
        .get("https://api.github.com/user")
        .header("Authorization", format!("Bearer {}", access_token))
        .header("User-Agent", "rust-release-manager")
        .header("Accept", "application/json")
        .send()
        .await;
        
    let github_user = match github_user_response {
        Ok(response) => {
            // First check and store the status
            let status = response.status();
            if !status.is_success() {
                let error_text = response.text().await.unwrap_or_default();
                error!("GitHub API returned error: {} - {}", status, error_text);
                return HttpResponse::InternalServerError().body(format!("GitHub API error: {}", status));
            }
            
            // For debugging, let's log the raw response
            if let Ok(text) = response.text().await {
                info!("GitHub response: {}", text);
                
                // Try to parse the text directly
                match serde_json::from_str::<GithubUser>(&text) {
                    Ok(user) => user,
                    Err(e) => {
                        error!("Failed to parse GitHub user: {}", e);
                        return HttpResponse::InternalServerError().body(format!("Failed to parse GitHub user: {}", e));
                    }
                }
            } else {
                error!("Failed to get response text from GitHub");
                return HttpResponse::InternalServerError().body("Failed to get response text from GitHub");
            }
        },
        Err(e) => {
            error!("Failed to get GitHub user: {}", e);
            return HttpResponse::InternalServerError().body(format!("Failed to get GitHub user: {}", e));
        }
    };
    
    // Create user in our system
    let user = User {
        id: github_user.id.to_string(),
        username: github_user.login,
        avatar_url: github_user.avatar_url,
        access_token: access_token.clone(),
    };
    
    // Save user to storage
    if let Err(e) = db.save_user(&user) {
        error!("Failed to save user: {}", e);
        return HttpResponse::InternalServerError().body("Failed to save user");
    }
    
    // Create session
    let session_id = Uuid::new_v4().to_string();
    if let Err(e) = db.save_session(&session_id, &user.id) {
        error!("Failed to save session: {}", e);
        return HttpResponse::InternalServerError().body("Failed to save session");
    }
    
    // Set session cookie
    let session_cookie = Cookie::build("session_id", session_id)
        .path("/")
        .secure(true)
        .http_only(true)
        .finish();
        
    // Redirect to frontend
    HttpResponse::Found()
        .cookie(session_cookie)
        .append_header(("Location", "/"))
        .finish()
}

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(github_login)
        .service(github_callback);
}
