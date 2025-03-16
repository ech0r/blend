pub mod releases;
pub mod users;
pub mod clients;

use actix_web::web;

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/releases")
            .configure(releases::configure)
    )
    .service(
        web::scope("/users")
            .configure(users::configure)
    )
    .service(
        web::scope("/clients")
            .configure(clients::configure)
    );
}
