pub use sea_orm_migration::prelude::*;

mod m20220120_000001_create_torrents_table;

pub struct Migrator;

#[derive(DeriveIden)]
pub enum Torrents {
    Table,
    Id,
    Name,
    InfoHash,
    Size,
    Files,
    CreatedAt,
    AddedAt,
    Seeders,
    Leechers,
    LastScrape,
    LastTrackerScrape,
    LastStale,
    Trackers,
}

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![Box::new(m20220120_000001_create_torrents_table::Migration)]
    }
}
