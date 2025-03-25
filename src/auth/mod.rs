pub mod github;
pub mod users_file_embed;

use actix_web::web;

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/github")
            .configure(github::configure)
    );
}
