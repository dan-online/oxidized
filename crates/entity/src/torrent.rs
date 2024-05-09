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

impl Model {
    pub fn get_category(&self) -> (&str, &str) {
        let name = self.name.as_ref().unwrap();
        let tvshow_re = regex::Regex::new(r"(.+?)(S(\d{2})|E(\d{2})|Season|Episode)(.*)").unwrap();

        if tvshow_re.is_match(name) {
            return ("TV", "5000");
        }

        if name.contains("1080p") || name.contains("720p") {
            return ("Movies", "2000");
        }

        if name.contains("MP3") || name.contains("FLAC") {
            return ("Audio", "3000");
        }

        if name.contains("PDF") || name.contains("EPUB") {
            return ("Books", "7000");
        }

        if name.contains("PC") || name.contains("MAC") {
            return ("PC", "4000");
        }

        if name.contains("XXX") {
            return ("XXX", "6000");
        }

        ("Other", "8000")
    }
}
