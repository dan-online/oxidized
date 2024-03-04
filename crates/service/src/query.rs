use ::oxidized_entity::{torrent, torrent::Entity as Torrent};
use chrono::{Timelike, Utc};
use sea_orm::{
    sea_query::{Expr, Func},
    *,
};
use serde::Serialize;
use tokio::try_join;

pub struct Query;

#[derive(Serialize)]
pub struct Stats {
    pub torrents: u64,
    pub scraped_torrents: u64,
    pub queued_info: u64,
    pub queued_trackers: u64,
}

impl Query {
    pub async fn find_torrent_by_id(db: &DbConn, id: i32) -> Result<Option<torrent::Model>, DbErr> {
        Torrent::find_by_id(id).one(db).await
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

    pub async fn get_stats(db: &DbConn) -> Result<Stats, DbErr> {
        let now = Utc::now()
            .with_minute(0)
            .unwrap()
            .with_second(0)
            .unwrap()
            .with_nanosecond(0)
            .unwrap()
            .naive_utc();

        let one_day_ago = now - chrono::Duration::days(1);

        // let query_res = db
        //     .query_one(Statement::from_sql_and_values(
        //         DatabaseBackend::Postgres,
        //         r#"SELECT
        //             (SELECT COUNT(*) FROM torrents) AS torrents,
        //             (SELECT COUNT(*) FROM torrents WHERE last_scrape IS NOT NULL AND last_tracker_scrape IS NOT NULL) AS scraped_torrents,
        //             (SELECT COUNT(*) FROM torrents WHERE last_scrape IS NULL OR last_scrape < $1) AS queued_info,
        //             (SELECT COUNT(*) FROM torrents WHERE (last_tracker_scrape IS NULL OR last_tracker_scrape < $1) AND last_scrape IS NOT NULL) AS queued_trackers;"#,
        //         [one_day_ago.into()],
        //     ))
        //     .await?;
        // let query_res = query_res.unwrap();

        // let torrents = query_res.try_get::<i64>("", "torrents")? as u64;
        // let scraped_torrents = query_res.try_get::<i64>("", "scraped_torrents")? as u64;
        // let queued_info = query_res.try_get::<i64>("", "queued_info")? as u64;
        // let queued_trackers = query_res.try_get::<i64>("", "queued_trackers")? as u64;

        // println!("torrents_count: {:?}", torrents_count);

        let (torrents, scraped_torrents, queued_info, queued_trackers) = try_join!(
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
                        .or(torrent::Column::LastTrackerScrape.lt(one_day_ago)))
                    .and(torrent::Column::LastScrape.is_not_null())
                )
                .count(db)
        )?;

        Ok(Stats {
            torrents,
            scraped_torrents,
            queued_info,
            queued_trackers,
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
        let one_day_ago = now - chrono::Duration::days(1);

        Torrent::find()
            .filter(
                torrent::Column::Id
                    .is_not_in(ignore.unwrap_or_default())
                    .and(
                        torrent::Column::LastTrackerScrape
                            .is_null()
                            .or(torrent::Column::LastTrackerScrape.lt(one_day_ago)),
                    ),
            )
            .filter(torrent::Column::LastScrape.is_not_null())
            .limit(50)
            .all(db)
            .await
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
