use oxidized_migration::{Migrator, MigratorTrait};
use rocket::{fairing, Build, Rocket};
use sea_orm_rocket::Database;

use crate::pool::Db;

pub async fn run_migrations(rocket: Rocket<Build>) -> fairing::Result {
    let conn = &Db::fetch(&rocket).unwrap().conn;

    // let _ = migration::Migrator::down(conn, None).await;

    let _ = Migrator::up(conn, None).await;

    Ok(rocket)
}
