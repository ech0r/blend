pub mod github;

use actix_web::web;

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/github")
            .configure(github::configure)
    );
}
