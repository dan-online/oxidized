use sea_orm_migration::{prelude::*, schema::*};

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
                    .col(pk_auto(Torrents::Id))
                    .col(Torrents::Name)
                    .col(Torrents::Info_Hash)
                    .col(Torrents::Size)
                    .col(Torrents::Files)
                    .col(Torrents::CreatedAt)
                    .col(Torrents::AddedAt)
                    .col(Torrents::Seeders)
                    .col(Torrents::Leechers)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Torrents::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum Torrents {
    Table,
    Id,
    Name,
    Info_Hash,
    Size,
    Files,
    CreatedAt,
    AddedAt,
    Seeders,
    Leechers,
}
