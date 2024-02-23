use ::entity::{torrent, torrent::Entity as Torrent};
use sea_orm::*;

pub struct Query;

impl Query {
    pub async fn find_torrent_by_id(db: &DbConn, id: u64) -> Result<Option<torrent::Model>, DbErr> {
        Torrent::find_by_id(id).one(db).await
    }

    /// If ok, returns (torrent models, num pages).
    pub async fn find_torrents_in_page(
        db: &DbConn,
        page: u64,
        torrents_per_page: u64,
    ) -> Result<(Vec<torrent::Model>, u64), DbErr> {
        // Setup paginator
        let paginator = Torrent::find()
            .order_by_asc(torrent::Column::Id)
            .paginate(db, torrents_per_page);
        let num_pages = paginator.num_pages().await?;

        // Fetch paginated torrents
        paginator.fetch_page(page - 1).await.map(|p| (p, num_pages))
    }
}
