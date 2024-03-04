use oxidized_service::Query;
use rocket::serde::json::Json;
use sea_orm_rocket::Connection;
use serde_json::json;
use tokio::time::Instant;

use crate::pool::Db;

#[get("/stats")]
pub async fn route(conn: Connection<'_, Db>) -> Json<serde_json::Value> {
    let db = conn.into_inner();

    let start = Instant::now();

    let stats = Query::get_stats(db).await;

    let duration = start.elapsed();

    if let Err(err) = stats {
        return Json(json!({
            "error": format!("{}", err),
        }));
    }

    let stats = stats.unwrap();

    Json(json!({
        "stats": stats,
        "speed": duration.as_micros() as f64 / 1000.0,
    }))
}
