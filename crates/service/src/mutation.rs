use ::oxidized_entity::stats::Entity as Stats;
use ::oxidized_entity::torrent::{self, Entity as Torrent, Tracker, Trackers};
use chrono::Utc;
use sea_orm::{sea_query::Expr, *};

pub struct Mutation;

impl Mutation {
    pub async fn update_stat(
        db: &DbConn,
        name: &str,
        action: &str,
        value: Option<i32>,
    ) -> Result<(), DbErr> {
        // TODO: Surely sea-orm has a better way to do this?
        match action {
            "inc" => {
                let sql = format!(
                    "UPDATE {} SET value = value + {} WHERE name = '{}'",
                    Stats.table_name(),
                    value.unwrap_or(1),
                    name
                );

                db.execute_unprepared(&sql)
                    .await
                    .map_err(|e| DbErr::from(e))
            }
            "dec" => {
                let sql = format!(
                    "UPDATE {} SET value = value - {} WHERE name = '{}'",
                    Stats.table_name(),
                    value.unwrap_or(1),
                    name
                );

                db.execute_unprepared(&sql)
                    .await
                    .map_err(|e| DbErr::from(e))
            }
            _ => {
                return Err(DbErr::Custom("Invalid action.".to_owned()));
            }
        }?;

        Ok(())
    }

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

        Mutation::update_stat(db, "total_torrents", "inc", None).await?;
        Mutation::update_stat(db, "queue_torrent_info", "inc", None).await?;

        Ok(())
    }

    pub async fn delete_torrents(db: &DbConn, ids: Vec<i32>) -> Result<(), DbErr> {
        let res = Torrent::delete_many()
            .filter(Expr::col(torrent::Column::Id).is_in(ids))
            .exec(db)
            .await
            .map_err(|e| DbErr::from(e))?;

        Mutation::update_stat(db, "total_torrents", "dec", Some(res.rows_affected as i32)).await?;

        Ok(())
    }

    pub async fn create_torrent(
        db: &DbConn,
        info_hash: String,
    ) -> Result<torrent::ActiveModel, DbErr> {
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
        }
        .save(db)
        .await?;

        Mutation::update_stat(db, "total_torrents", "inc", None).await?;
        Mutation::update_stat(db, "queue_torrent_info", "inc", None).await?;

        Ok(torrent)
    }

    pub async fn create_torrent_internal(
        db: &DbConn,
        info_hash: String,
        name: String,
        size: i32,
        files: Vec<String>,
    ) -> Result<torrent::ActiveModel, DbErr> {
        let torrent = torrent::ActiveModel {
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
        .await?;

        Mutation::update_stat(db, "total_torrents", "inc", None).await?;
        Mutation::update_stat(db, "queue_torrent_trackers", "inc", None).await?;

        Ok(torrent)
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

        let torrent = torrent::ActiveModel {
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
            last_stale: torrent.last_stale,
        }
        .update(db)
        .await?;

        Mutation::update_stat(db, "queue_torrent_info", "dec", None).await?;
        Mutation::update_stat(db, "queue_torrent_trackers", "inc", None).await?;

        Ok(torrent)
    }

    pub async fn update_torrent_trackers(
        db: &DbConn,
        id: i32,
        trackers: Vec<Tracker>,
    ) -> Result<torrent::Model, DbErr> {
        let torrent = Torrent::find_by_id(id)
            .one(db)
            .await?
            .ok_or(DbErr::Custom("Cannot find torrent.".to_owned()))?;

        let last_stale_set = torrent.last_stale.is_some();

        let torrent: torrent::ActiveModel = torrent.into();

        let best_tracker = trackers
            .iter()
            .max_by(|a, b| a.seeders.cmp(&b.seeders))
            .unwrap();

        let torrent = torrent::ActiveModel {
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
            trackers: Set(Trackers(trackers.clone())),
            // if no last_stale and seeders/leechers are 0, then set to datetime
            // if last_stale and seeders/leechers are 0, then keep old last_stale
            // if last_stale and seeders/leechers are not 0, then set to None
            last_stale: if !last_stale_set
                && best_tracker.seeders == 0
                && best_tracker.leechers == 0
            {
                Set(Some(Utc::now().naive_utc()))
            } else if last_stale_set && best_tracker.seeders == 0 && best_tracker.leechers == 0 {
                torrent.last_stale
            } else {
                Set(None)
            },
        }
        .update(db)
        .await?;

        Mutation::update_stat(db, "queue_torrent_trackers", "dec", None).await?;

        Ok(torrent)
    }

    pub async fn mark_stale(db: &DbConn) -> Result<(), DbErr> {
        let res = Torrent::update_many()
            .col_expr(
                torrent::Column::LastStale,
                Expr::value(Some(Utc::now().naive_utc())),
            )
            .filter(
                torrent::Column::LastStale
                    .is_null()
                    .and(torrent::Column::Seeders.eq(0))
                    .and(torrent::Column::Leechers.eq(0)),
            )
            .exec(db)
            .await?;

        Mutation::update_stat(db, "stale_torrents", "inc", Some(res.rows_affected as i32)).await?;

        Ok(())
    }

    pub async fn delete_stale(db: &DbConn) -> Result<(), DbErr> {
        let now = Utc::now().naive_utc();
        let three_days_ago = now - chrono::Duration::try_days(9).unwrap();

        let res = Torrent::delete_many()
            .filter(
                torrent::Column::LastStale
                    .is_not_null()
                    .and(torrent::Column::LastStale.lt(three_days_ago)),
            )
            .exec(db)
            .await?;

        Mutation::update_stat(db, "total_torrents", "dec", Some(res.rows_affected as i32)).await?;
        Mutation::update_stat(db, "stale_torrents", "dec", Some(res.rows_affected as i32)).await?;

        Ok(())
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

            Mutation::update_stat(db, "total_torrents", "dec", Some(1)).await?;
        }

        Ok(())
    }
}
