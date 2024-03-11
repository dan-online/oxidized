use ::oxidized_entity::{stats, stats::Entity as Stats, torrent, torrent::Entity as Torrent};
use chrono::Utc;
use sea_orm::{
    sea_query::{Expr, Func},
    *,
};
use serde::Serialize;
use tokio::try_join;

pub struct Query;

#[derive(Serialize)]
pub struct Queue {
    pub info: u64,
    pub trackers: u64,
}

#[derive(Serialize)]
pub struct OutputStats {
    pub torrents: u64,
    pub scraped: u64,
    pub stale: u64,
    pub queue: Queue,
}

impl Query {
    pub async fn find_torrent_by_id(db: &DbConn, id: i32) -> Result<Option<torrent::Model>, DbErr> {
        Torrent::find_by_id(id).one(db).await
    }

    pub async fn find_all_torrents(db: &DbConn) -> Result<Vec<torrent::Model>, DbErr> {
        Torrent::find()
            .select_column(torrent::Column::Name)
            .select_column(torrent::Column::Id)
            .all(db)
            .await
    }

    pub async fn search_torrents_by_name(
        db: &DbConn,
        name: Option<String>,
        offset: Option<u64>,
        limit: Option<u64>,
    ) -> Result<Vec<torrent::Model>, DbErr> {
        Torrent::find()
            .filter(
                Expr::expr(Func::lower(Expr::col(torrent::Column::Name))).like(format!(
                    "%{}%",
                    name.unwrap_or("".to_string())
                        .to_lowercase()
                        .split("%")
                        .collect::<Vec<&str>>()
                        .join("\\%")
                        .split_ascii_whitespace()
                        .collect::<Vec<&str>>()
                        .join("%")
                )),
            )
            .offset(offset.unwrap_or(0))
            .limit(limit.unwrap_or(100))
            .all(db)
            .await
    }

    pub async fn find_torrent_by_info_hash(
        db: &DbConn,
        info_hash: String,
    ) -> Result<Option<torrent::Model>, DbErr> {
        Torrent::find()
            .filter(torrent::Column::InfoHash.eq(info_hash.to_uppercase()))
            .one(db)
            .await
    }

    pub async fn exists_torrent_by_info_hash(db: &DbConn, info_hash: &str) -> Result<bool, DbErr> {
        let count = Torrent::find()
            .filter(torrent::Column::InfoHash.eq(info_hash.to_uppercase()))
            .count(db)
            .await?;

        Ok(count > 0)
    }

    pub async fn get_stats(db: &DbConn) -> Result<OutputStats, DbErr> {
        let mut stats = OutputStats {
            torrents: 0,
            scraped: 0,
            queue: Queue {
                info: 0,
                trackers: 0,
            },
            stale: 0,
        };

        let rows = Stats::find().all(db).await?;

        // reindex if last updated is older than 10s
        let reindex = rows
            .iter()
            .any(|row| row.last_updated.timestamp() < Utc::now().timestamp() - 120);

        println!("reindex: {}", reindex);

        if rows.is_empty() {
            for stat in stats::StatType::iter() {
                let row = stats::ActiveModel {
                    name: Set(stat),
                    value: Set(0),
                    last_updated: Set(Utc::now().naive_utc()),
                    ..Default::default()
                };

                row.insert(db).await?;
            }
        }

        if reindex || rows.is_empty() {
            let raw_stats = Query::get_raw_stats(db).await?;

            for row in rows {
                let stat = match row.name {
                    stats::StatType::TotalTorrents => raw_stats.torrents,
                    stats::StatType::ScrapedTorrents => raw_stats.scraped,
                    stats::StatType::QueueInfo => raw_stats.queue.info,
                    stats::StatType::QueueTrackers => raw_stats.queue.trackers,
                    stats::StatType::Stale => raw_stats.stale,
                };

                let mut row: stats::ActiveModel = row.into();

                row.value = Set(stat as i32);
                row.last_updated = Set(Utc::now().naive_utc());

                row.update(db).await?;
            }

            return Ok(raw_stats);
        }

        for row in rows {
            match row.name {
                stats::StatType::TotalTorrents => stats.torrents = row.value as u64,
                stats::StatType::ScrapedTorrents => stats.scraped = row.value as u64,
                stats::StatType::QueueInfo => stats.queue.info = row.value as u64,
                stats::StatType::QueueTrackers => stats.queue.trackers = row.value as u64,
                stats::StatType::Stale => stats.stale = row.value as u64,
            }
        }

        Ok(stats)
    }

    pub async fn get_raw_stats(db: &DbConn) -> Result<OutputStats, DbErr> {
        let now = Utc::now().naive_utc();

        let three_days_ago = now - chrono::Duration::days(3);

        let (torrents, scraped, queued_info, queued_trackers, stale) = try_join!(
            Torrent::find().count(db),
            Torrent::find()
                .filter(
                    torrent::Column::LastScrape
                        .is_not_null()
                        .and(torrent::Column::LastTrackerScrape.is_not_null())
                )
                .count(db),
            Torrent::find()
                .filter(torrent::Column::LastScrape.is_null())
                .count(db),
            Torrent::find()
                .filter(
                    (torrent::Column::LastTrackerScrape
                        .is_null()
                        .or(torrent::Column::LastTrackerScrape.lt(three_days_ago)))
                    .and(torrent::Column::LastScrape.is_not_null())
                )
                .count(db),
            Torrent::find()
                .filter(torrent::Column::LastStale.is_not_null())
                .count(db)
        )?;

        Ok(OutputStats {
            torrents,
            scraped,
            queue: Queue {
                info: queued_info,
                trackers: queued_trackers,
            },
            stale,
        })
    }

    pub async fn find_torrent_queue_info(
        db: &DbConn,
        ignore: Option<Vec<i32>>,
    ) -> Result<Vec<torrent::Model>, DbErr> {
        let torrents = Torrent::find()
            .filter(
                torrent::Column::Id
                    .is_not_in(ignore.unwrap_or_default())
                    .and(torrent::Column::LastScrape.is_null()),
            )
            .limit(50)
            .all(db)
            .await;

        torrents
    }

    pub async fn find_torrent_queue_trackers(
        db: &DbConn,
        ignore: Option<Vec<i32>>,
    ) -> Result<Vec<torrent::Model>, DbErr> {
        let now = Utc::now().naive_utc();
        let three_days_ago = now - chrono::Duration::days(3);

        let mut torrents = Torrent::find()
            .filter(
                torrent::Column::Id
                    .is_not_in(ignore.unwrap_or_default())
                    .and(
                        torrent::Column::LastTrackerScrape
                            .is_null()
                            .or(torrent::Column::LastTrackerScrape.lt(three_days_ago)),
                    ),
            )
            .filter(torrent::Column::LastScrape.is_not_null())
            .order_by_asc(torrent::Column::LastTrackerScrape)
            .limit(50)
            .all(db)
            .await?;

        torrents.sort_by(|a, b| {
            if a.last_tracker_scrape.is_none() {
                std::cmp::Ordering::Less
            } else if b.last_tracker_scrape.is_none() {
                std::cmp::Ordering::Greater
            } else {
                a.last_tracker_scrape.cmp(&b.last_tracker_scrape)
            }
        });

        Ok(torrents)
    }

    pub async fn find_torrents_in_page(
        db: &DbConn,
        page: u64,
        torrents_per_page: u64,
    ) -> Result<(Vec<torrent::Model>, u64), DbErr> {
        let paginator = Torrent::find()
            .order_by_asc(torrent::Column::Id)
            .paginate(db, torrents_per_page);
        let num_pages = paginator.num_pages().await?;

        paginator.fetch_page(page - 1).await.map(|p| (p, num_pages))
    }
}
