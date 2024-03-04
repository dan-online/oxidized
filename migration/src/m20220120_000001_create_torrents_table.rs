use rocket::tokio::try_join;
use sea_orm_migration::prelude::*;

use crate::Torrents;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Torrents::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Torrents::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Torrents::Name).string())
                    .col(
                        ColumnDef::new(Torrents::InfoHash)
                            .unique_key()
                            .string()
                            .not_null(),
                    )
                    .col(ColumnDef::new(Torrents::Size).integer().not_null())
                    .col(
                        ColumnDef::new(Torrents::Files)
                            .array(ColumnType::String(Some(255)))
                            .not_null(),
                    )
                    .col(ColumnDef::new(Torrents::CreatedAt).timestamp())
                    .col(ColumnDef::new(Torrents::AddedAt).timestamp())
                    .col(ColumnDef::new(Torrents::Seeders).integer().not_null())
                    .col(ColumnDef::new(Torrents::Leechers).integer().not_null())
                    .col(ColumnDef::new(Torrents::LastScrape).timestamp())
                    .col(ColumnDef::new(Torrents::LastTrackerScrape).timestamp())
                    .col(ColumnDef::new(Torrents::Trackers).json_binary())
                    .to_owned(),
            )
            .await?;

        try_join!(
            manager.create_index(
                sea_query::Index::create()
                    .if_not_exists()
                    .col(Torrents::LastScrape)
                    .table(Torrents::Table)
                    .name("torrents_last_scrape_idx")
                    .to_owned(),
            ),
            manager.create_index(
                sea_query::Index::create()
                    .if_not_exists()
                    .col(Torrents::Id)
                    .table(Torrents::Table)
                    .name("torrents_id_idx")
                    .to_owned(),
            ),
            manager.create_index(
                sea_query::Index::create()
                    .if_not_exists()
                    .col(Torrents::Name)
                    .table(Torrents::Table)
                    .name("torrents_info_name")
                    .to_owned(),
            ),
            manager.create_index(
                sea_query::Index::create()
                    .if_not_exists()
                    .col(Torrents::InfoHash)
                    .table(Torrents::Table)
                    .name("torrents_info_hash_idx")
                    .to_owned(),
            ),
            manager.create_index(
                sea_query::Index::create()
                    .if_not_exists()
                    .col(Torrents::LastTrackerScrape)
                    .table(Torrents::Table)
                    .name("torrents_last_tracker_scrape_idx")
                    .to_owned(),
            ),
            manager.create_index(
                sea_query::Index::create()
                    .if_not_exists()
                    .col(Torrents::LastTrackerScrape)
                    .col(Torrents::LastScrape)
                    .table(Torrents::Table)
                    .name("torrents_last_tracker_scrape_last_scrape_idx")
                    .to_owned(),
            )
        )?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Torrents::Table).to_owned())
            .await?;

        try_join!(
            manager.drop_index(
                sea_query::Index::drop()
                    .table(Torrents::Table)
                    .name("torrents_last_scrape_idx")
                    .to_owned(),
            ),
            manager.drop_index(
                sea_query::Index::drop()
                    .table(Torrents::Table)
                    .name("torrents_id_idx")
                    .to_owned(),
            ),
            manager.drop_index(
                sea_query::Index::drop()
                    .table(Torrents::Table)
                    .name("torrents_info_name")
                    .to_owned(),
            ),
            manager.drop_index(
                sea_query::Index::drop()
                    .table(Torrents::Table)
                    .name("torrents_info_hash_idx")
                    .to_owned(),
            ),
            manager.drop_index(
                sea_query::Index::drop()
                    .table(Torrents::Table)
                    .name("torrents_last_tracker_scrape_idx")
                    .to_owned(),
            ),
            manager.drop_index(
                sea_query::Index::drop()
                    .table(Torrents::Table)
                    .name("torrents_last_tracker_scrape_last_scrape_idx")
                    .to_owned(),
            )
        )?;

        Ok(())
    }
}
