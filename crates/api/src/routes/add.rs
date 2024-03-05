use oxidized_entity::sea_orm::TryIntoModel;
use oxidized_service::Mutation;
use rocket::serde::json::Json;
use sea_orm_rocket::Connection;
use serde::Deserialize;
use serde_json::json;

use crate::{guards::apikey::ApiKeyGuard, pool::Db};

#[derive(Deserialize)]
pub struct TorrentInput {
    info_hash: String,
}

#[post("/add", format = "application/json", data = "<torrent_input>")]
pub async fn route(
    _apikey: ApiKeyGuard,
    conn: Connection<'_, Db>,
    torrent_input: Json<TorrentInput>,
) -> Json<serde_json::Value> {
    let db = conn.into_inner();

    let torrent =
        Mutation::create_torrent(db, torrent_input.info_hash.clone().to_uppercase()).await;

    if let Err(err) = torrent {
        return Json(json!({
            "error": format!("{}", err),
        }));
    }

    let torrent = torrent.unwrap();
    let torrent = torrent.try_into_model().unwrap();

    Json(json!({
        "torrent": torrent,
    }))
}
