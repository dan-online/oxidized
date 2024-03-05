use rocket::serde::{Deserialize, Serialize};
use sea_orm::{entity::prelude::*, FromJsonQueryResult};

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Deserialize, Serialize)]
#[serde(crate = "rocket::serde")]
#[sea_orm(table_name = "torrents")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub name: Option<String>,
    pub info_hash: String,
    pub size: i32,
    pub files: Vec<String>,
    pub added_at: DateTime,
    pub seeders: i32,
    pub leechers: i32,
    pub trackers: Trackers,
    #[sea_orm(indexed)]
    pub last_scrape: Option<DateTime>,
    #[sea_orm(indexed)]
    pub last_tracker_scrape: Option<DateTime>,
    #[sea_orm(indexed)]
    pub last_stale: Option<DateTime>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, FromJsonQueryResult)]
pub struct Trackers(pub Vec<Tracker>);

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Tracker {
    pub url: String,
    pub seeders: i32,
    pub leechers: i32,
    pub last_scrape: DateTime,
}
