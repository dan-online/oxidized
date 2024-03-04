use oxidized_service::Query;
use rocket::serde::json::Json;
use sea_orm_rocket::Connection;
use serde_json::json;
use std::time::Instant;

use crate::pool::Db;

#[get("/<info_hash>")]
pub async fn route(conn: Connection<'_, Db>, info_hash: String) -> Json<serde_json::Value> {
    let db = conn.into_inner();

    let start = Instant::now();

    let torrent = Query::find_torrent_by_info_hash(db, info_hash).await;

    let duration = start.elapsed();

    if let Err(err) = torrent {
        return Json(json!({
            "error": format!("{}", err),
        }));
    }

    let torrent = torrent.unwrap();

    Json(json!({
        "torrent": torrent,
        "speed": duration.as_micros() as f64 / 1000.0,
    }))
}
