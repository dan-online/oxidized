use oxidized_service::Query;
use rocket::serde::json::Json;
use sea_orm_rocket::Connection;
use serde_json::json;

use crate::{guards::apikey::ApiKeyGuard, pool::Db};

const DEFAULT_POSTS_PER_PAGE: u64 = 100;

#[get("/list?<page>&<posts_per_page>")]
pub async fn route(
    _apikey: ApiKeyGuard,
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
