use rocket::serde::{Deserialize, Serialize};
use sea_orm::entity::prelude::*;

#[derive(Debug, Clone, PartialEq, Eq, EnumIter, DeriveActiveEnum, Serialize, Deserialize)]
#[sea_orm(rs_type = "String", db_type = "Enum")]
pub enum StatType {
    #[sea_orm(string_value = "total_torrents")]
    TotalTorrents,
    #[sea_orm(string_value = "scraped_torrents")]
    ScrapedTorrents,
    #[sea_orm(string_value = "queue_torrent_info")]
    QueueInfo,
    #[sea_orm(string_value = "queue_torrent_trackers")]
    QueueTrackers,
    #[sea_orm(string_value = "stale_torrents")]
    Stale,
}

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Deserialize, Serialize)]
#[serde(crate = "rocket::serde")]
#[sea_orm(table_name = "stats")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    #[sea_orm(indexed)]
    pub name: StatType,
    pub value: i32,
    pub last_updated: DateTime,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
