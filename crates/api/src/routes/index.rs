use rocket::serde::json::Json;
use serde_json::json;

#[get("/")]
pub async fn route() -> Json<serde_json::Value> {
    Json(json!({
        "hello": "world",
    }))
}
