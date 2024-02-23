#[macro_use]
extern crate rocket;

use rocket::serde::json::Json;
use rocket_example_service::Query;
use serde_json::json;

use sea_orm_rocket::{Connection, Database};

mod pool;
use pool::Db;

pub use entity::torrent;
pub use entity::torrent::Entity as Torrent;

const DEFAULT_POSTS_PER_PAGE: u64 = 100;

#[get("/list?<page>&<posts_per_page>")]
async fn list(
    conn: Connection<'_, Db>,
    page: Option<u64>,
    posts_per_page: Option<u64>,
) -> Json<serde_json::Value> {
    let db = conn.into_inner();

    // Set page number and items per page
    let page = page.unwrap_or(1);
    let posts_per_page = posts_per_page.unwrap_or(DEFAULT_POSTS_PER_PAGE);
    if page == 0 {
        panic!("Page number cannot be zero");
    }

    let (posts, num_pages) = Query::find_torrents_in_page(db, page, posts_per_page)
        .await
        .expect("Cannot find posts in page");

    Json(json!({
        "posts": posts,
        "num_pages": num_pages,
    }))
}

async fn run_migrations(rocket: Rocket<Build>) -> fairing::Result {
    let conn = &Db::fetch(&rocket).unwrap().conn;
    let _ = migration::Migrator::up(conn, None).await;
    Ok(rocket)
}

#[tokio::main]
async fn start() -> Result<(), rocket::Error> {
    rocket::build()
        .attach(AdHoc::try_on_ignite("Migrations", run_migrations))
        .attach(Db::init())
        .mount("/", routes![list])
        .launch()
        .await
        .map(|_| ())
}

pub fn main() {
    let result = start();

    println!("Rocket: deorbit.");

    if let Some(err) = result.err() {
        println!("Error: {err}");
    }
}
