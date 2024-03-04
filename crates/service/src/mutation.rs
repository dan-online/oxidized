use ::oxidized_entity::torrent::{self, Entity as Torrent, Tracker, Trackers};
use chrono::Utc;
use sea_orm::*;

pub struct Mutation;

impl Mutation {
    pub async fn create_torrents(db: &DbConn, info_hashes: Vec<String>) -> Result<(), DbErr> {
        let mut torrents = Vec::new();

        // bulk insert
        for info_hash in info_hashes {
            let torrent = torrent::ActiveModel {
                name: Set(None),
                info_hash: Set(info_hash),
                size: Set(0),
                files: Set(Vec::new()),
                seeders: Set(0),
                leechers: Set(0),
                added_at: Set(Utc::now().naive_utc()),
                trackers: Set(Trackers(Vec::new())),
                ..Default::default()
            };

            torrents.push(torrent);
        }

        Torrent::insert_many(torrents)
            .exec(db)
            .await
            .map_err(|e| DbErr::from(e))?;

        Ok(())
    }

    pub async fn create_torrent(
        db: &DbConn,
        info_hash: String,
    ) -> Result<torrent::ActiveModel, DbErr> {
        torrent::ActiveModel {
            name: Set(None),
            info_hash: Set(info_hash),
            size: Set(0),
            files: Set(Vec::new()),
            seeders: Set(0),
            leechers: Set(0),
            added_at: Set(Utc::now().naive_utc()),
            trackers: Set(Trackers(Vec::new())),
            ..Default::default()
        }
        .save(db)
        .await
    }

    pub async fn create_torrent_internal(
        db: &DbConn,
        info_hash: String,
        name: String,
        size: i32,
        files: Vec<String>,
    ) -> Result<torrent::ActiveModel, DbErr> {
        torrent::ActiveModel {
            name: Set(Some(name)),
            info_hash: Set(info_hash),
            size: Set(size),
            files: Set(files),
            seeders: Set(0),
            leechers: Set(0),
            added_at: Set(Utc::now().naive_utc()),
            trackers: Set(Trackers(Vec::new())),
            last_scrape: Set(Some(Utc::now().naive_utc())),
            ..Default::default()
        }
        .save(db)
        .await
    }

    pub async fn update_torrent_info(
        db: &DbConn,
        id: i32,
        name: String,
        size: i32,
        files: Vec<String>,
    ) -> Result<torrent::Model, DbErr> {
        let torrent: torrent::ActiveModel = Torrent::find_by_id(id)
            .one(db)
            .await?
            .ok_or(DbErr::Custom("Cannot find torrent.".to_owned()))
            .map(Into::into)?;

        torrent::ActiveModel {
            id: torrent.id,
            info_hash: torrent.info_hash,
            added_at: torrent.added_at,
            seeders: torrent.seeders,
            leechers: torrent.leechers,
            trackers: torrent.trackers,
            name: Set(Some(name)),
            size: Set(size),
            files: Set(files),
            last_scrape: Set(Some(Utc::now().naive_utc())),
            last_tracker_scrape: torrent.last_tracker_scrape,
        }
        .update(db)
        .await
    }

    pub async fn update_torrent_trackers(
        db: &DbConn,
        id: i32,
        trackers: Vec<Tracker>,
    ) -> Result<torrent::Model, DbErr> {
        let torrent: torrent::ActiveModel = Torrent::find_by_id(id)
            .one(db)
            .await?
            .ok_or(DbErr::Custom("Cannot find torrent.".to_owned()))
            .map(Into::into)?;

        let best_tracker = trackers
            .iter()
            .max_by(|a, b| a.seeders.cmp(&b.seeders))
            .unwrap();

        torrent::ActiveModel {
            id: torrent.id,
            info_hash: torrent.info_hash,
            added_at: torrent.added_at,
            seeders: Set(best_tracker.seeders),
            leechers: Set(best_tracker.leechers),
            name: torrent.name,
            size: torrent.size,
            files: torrent.files,
            last_scrape: torrent.last_scrape,
            last_tracker_scrape: Set(Some(Utc::now().naive_utc())),
            trackers: Set(Trackers(trackers)),
        }
        .update(db)
        .await
    }

    pub async fn delete_torrent(db: &DbConn, id: i32) -> Result<(), DbErr> {
        let torrent = Torrent::find_by_id(id)
            .one(db)
            .await
            .map_err(|e| DbErr::from(e))?;

        if torrent.is_some() {
            torrent
                .unwrap()
                .delete(db)
                .await
                .map_err(|e| DbErr::from(e))?;
        }

        Ok(())
    }
}
