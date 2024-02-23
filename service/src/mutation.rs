use ::entity::{torrent, torrent::Entity as Torrent};
use sea_orm::*;

pub struct Mutation;

impl Mutation {
    pub async fn create_torrent(
        db: &DbConn,
        form_data: torrent::Model,
    ) -> Result<torrent::ActiveModel, DbErr> {
        torrent::ActiveModel {
            name: Set(form_data.name),
            info_hash: Set(form_data.info_hash),
            size: Set(form_data.size),
            files: Set(form_data.files),
            created_at: Set(form_data.created_at),
            seeders: Set(0),
            leechers: Set(0),
            ..Default::default()
        }
        .save(db)
        .await
    }
}
