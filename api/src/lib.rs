#[macro_use]
extern crate rocket;

mod pool;
mod routes;
mod service;

use pool::*;
use routes::*;
use service::{misc_tasks::MiscTasksService, torrents::TorrentService, *};

use rocket::fairing::AdHoc;
use sea_orm_rocket::Database;
use tracing::Level;

#[rocket::main]
pub async fn main() -> Result<(), rocket::Error> {
    tracing_subscriber::fmt().with_max_level(Level::INFO).init();

    rocket::build()
        .attach(Db::init())
        .attach(AdHoc::try_on_ignite(
            "Migrations",
            migrations::run_migrations,
        ))
        .attach(TorrentService::new().await)
        .attach(MiscTasksService::new())
        .mount("/", get_routes())
        .launch()
        .await
        .map(|_| ())
}
