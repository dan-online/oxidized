use crate::pool::Db;
use oxidized_config::{get_config, Settings};
use oxidized_entity::{sea_orm::DatabaseConnection, torrent, torrent::Tracker};
use oxidized_service::{Mutation, Query};
use oxidized_torrent::{MagneticoDTorrent, Spider, TorrentInfo, TorrentTrackers};
use rocket::fairing::{self, Fairing};
use rocket::{Build, Rocket};
use sea_orm_rocket::Database;
use std::collections::HashSet;
use std::sync::Arc;
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver};
use tokio::sync::Mutex;

pub struct TorrentService {
    queue: Arc<Mutex<HashSet<i32>>>,
}

#[rocket::async_trait]
impl Fairing for TorrentService {
    fn info(&self) -> fairing::Info {
        fairing::Info {
            name: "Torrent Service",
            kind: fairing::Kind::Ignite,
        }
    }

    async fn on_ignite(&self, rocket: Rocket<Build>) -> fairing::Result {
        let conn = &Db::fetch(&rocket).unwrap().conn;
        let config = get_config();

        let (info_rx, trackers_rx) = self.spawn_producer(conn.clone(), config.clone());

        if config.app.update_info {
            self.spawn_consumer_info(conn.clone(), info_rx);
        }

        if config.app.update_trackers {
            self.spawn_consumer_trackers(conn.clone(), trackers_rx);
        }

        if config.app.spider {
            let spider = Spider::new();

            let spider_rx = spider.start().await;

            self.spawn_consumer_spider(conn.clone(), spider_rx);
        }

        Ok(rocket)
    }
}
impl TorrentService {
    pub async fn new() -> Self {
        let queue: Arc<Mutex<HashSet<i32>>> = Arc::new(Mutex::new(HashSet::new()));

        Self { queue }
    }

    pub fn spawn_producer(
        &self,
        conn: DatabaseConnection,
        config: Settings,
    ) -> (
        UnboundedReceiver<torrent::Model>,
        UnboundedReceiver<Vec<torrent::Model>>,
    ) {
        let queue = self.queue.clone();
        let (info_tx, info_rx) = unbounded_channel::<torrent::Model>();
        let (trackers_tx, trackers_rx) = unbounded_channel::<Vec<torrent::Model>>();

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(3));

            loop {
                interval.tick().await;
                let mut queue_lock = queue.lock().await;

                if config.app.update_info {
                    let torrents = Query::find_torrent_queue_info(
                        &conn,
                        Some(queue_lock.iter().map(|x| *x).collect()),
                    )
                    .await
                    .expect("Cannot find torrents in queue");

                    for torrent in &torrents {
                        let id = torrent.id.clone();

                        info_tx.send(torrent.clone()).unwrap();

                        queue_lock.insert(id);
                    }
                }

                if config.app.update_trackers {
                    let torrents_trackers = Query::find_torrent_queue_trackers(
                        &conn,
                        Some(queue_lock.iter().map(|x| *x).collect()),
                    )
                    .await
                    .expect("Cannot find tracker torrents in queue");

                    for chunk in torrents_trackers.chunks(10) {
                        trackers_tx.send(chunk.to_vec()).unwrap();

                        for torrent in chunk {
                            let id = torrent.id.clone();

                            queue_lock.insert(id);
                        }
                    }
                }
            }
        });

        (info_rx, trackers_rx)
    }

    pub fn spawn_consumer_spider(
        &self,
        conn: DatabaseConnection,
        mut rx: UnboundedReceiver<MagneticoDTorrent>,
    ) {
        tokio::spawn(async move {
            while let Some(torrent) = rx.recv().await {
                let exists = Query::exists_torrent_by_info_hash(&conn, &torrent.info_hash)
                    .await
                    .unwrap_or(false);

                if exists {
                    continue;
                }

                let size = torrent
                    .files
                    .iter()
                    .map(|f| (f.size / 1000000) as i32)
                    .collect::<Vec<i32>>()
                    .iter()
                    .sum();

                let _ = Mutation::create_torrent_internal(
                    &conn,
                    torrent.info_hash.clone().to_uppercase(),
                    torrent.name.clone(),
                    size,
                    torrent
                        .files
                        .iter()
                        .map(|f| f.path.clone())
                        .collect::<Vec<String>>(),
                )
                .await;
            }
        });
    }

    pub fn spawn_consumer_info(
        &self,
        conn: DatabaseConnection,
        mut info_rx: UnboundedReceiver<torrent::Model>,
    ) {
        let queue = self.queue.clone();

        tokio::spawn(async move {
            let torrent_info = TorrentInfo::new().await.unwrap();

            while let Some(torrent) = info_rx.recv().await {
                let info = torrent_info.get_torrent_info(&torrent.info_hash).await;

                if let Ok(info) = info {
                    let mut size = 0;
                    let mut files = vec![];

                    for (filename, length) in info.iter_filenames_and_lengths().unwrap() {
                        let file_str = filename.to_string().unwrap();

                        files.push(file_str);

                        size += (length / 1000000) as i32;
                    }

                    let name = match info.name {
                        Some(name) => name.to_string(),
                        None => "".to_string(),
                    };

                    let _ = Mutation::update_torrent_info(&conn, torrent.id, name, size, files)
                        .await
                        .expect("Cannot update torrent info");
                } else {
                    println!("Cannot get info for torrent: {}", torrent.info_hash);
                    println!("{:?}", info);

                    // 30 days or never scraped
                    if torrent.last_scrape.is_none()
                        || torrent.last_scrape.unwrap().timestamp()
                            < (chrono::Utc::now().timestamp() - 2592000)
                    {
                        let _ = Mutation::delete_torrent(&conn, torrent.id)
                            .await
                            .expect("Cannot delete torrent");
                    }
                }

                let mut queue_lock = queue.lock().await;

                queue_lock.remove(&torrent.id);
            }
        });
    }

    pub fn spawn_consumer_trackers(
        &self,
        conn: DatabaseConnection,
        mut trackers_rx: UnboundedReceiver<Vec<torrent::Model>>,
    ) {
        let queue = self.queue.clone();

        tokio::spawn(async move {
            let torrent_tracking = TorrentTrackers::new().await.unwrap();

            while let Some(torrents_chunk) = trackers_rx.recv().await {
                let mut torrent_tracking = torrent_tracking.clone();
                let conn_consumer_trackers = conn.clone();
                let queue_consumer_trackers = queue.clone();

                tokio::spawn(async move {
                    let tracker_info = torrent_tracking
                        .get_torrent_trackers(
                            torrents_chunk
                                .iter()
                                .map(|torrent| torrent.info_hash.clone())
                                .collect(),
                        )
                        .await;

                    if let Ok(tracker_info) = tracker_info {
                        for i in 0..torrents_chunk.len() {
                            let mut trackers: Vec<Tracker> = vec![];

                            let torrent = &torrents_chunk[i];
                            let tracker_info = &tracker_info;

                            for (tracker, info) in tracker_info {
                                let torrent_stats = info.torrent_stats[i].clone();

                                trackers.push(Tracker {
                                    last_scrape: chrono::Utc::now().naive_utc(),
                                    url: tracker.to_string(),
                                    seeders: torrent_stats.seeders.0,
                                    leechers: torrent_stats.leechers.0,
                                })
                            }

                            let _ = Mutation::update_torrent_trackers(
                                &conn_consumer_trackers,
                                torrent.id,
                                trackers,
                            )
                            .await
                            .unwrap();

                            let mut queue_lock = queue_consumer_trackers.lock().await;

                            queue_lock.remove(&torrent.id);
                        }
                    }
                });
            }
        });
    }
}
