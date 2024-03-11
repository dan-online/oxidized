use aquatic_udp_protocol::ScrapeResponse as UDPScrapeResponse;
use reqwest::Client;
use std::{
    collections::{BTreeMap, HashMap},
    net::{Ipv4Addr, SocketAddrV4},
    time::Instant,
};
use tokio::net::UdpSocket;
use tracing::{debug, info, warn};

use crate::{
    common::{connect_udp, request_and_response_http, scrape_udp},
    info::resolve_domain_to_ip,
};

#[derive(Clone)]
pub struct TorrentTracker {
    uri: String,
}

#[derive(Clone)]
pub struct TorrentTrackers {
    last_updated_trackers: Instant,
    trackers: Vec<TorrentTracker>,
    timeout_trackers: HashMap<String, (u8, Instant)>,
}

#[derive(Debug)]
pub struct TorrentScrapeStats {
    pub seeders: i32,
    pub leechers: i32,
}

pub struct TorrentScrapeResponse {
    pub stats: BTreeMap<String, TorrentScrapeStats>,
}

impl TorrentTrackers {
    pub async fn new() -> Result<Self, anyhow::Error> {
        let mut new = Self {
            last_updated_trackers: Instant::now(),
            trackers: vec![],
            timeout_trackers: HashMap::new(),
        };

        new.get_trackers().await?;

        info!(
            "Torrent tracker initialized with {} trackers",
            new.trackers.len()
        );

        Ok(new)
    }

    pub async fn fetch_socket(&self) -> Result<UdpSocket, anyhow::Error> {
        let peer_addr = SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, 0);
        let socket = UdpSocket::bind(peer_addr).await?;

        Ok(socket)
    }

    pub async fn get_trackers(&mut self) -> Result<Vec<TorrentTracker>, anyhow::Error> {
        if self.last_updated_trackers.elapsed().as_secs() > 60 || self.trackers.is_empty() {
            let trackers = self.fetch_trackers().await?;

            self.last_updated_trackers = Instant::now();
            self.trackers = trackers;
        }

        Ok(self.trackers.clone())
    }

    async fn make_request(
        &self,
        client: Client,
        url: &str,
    ) -> Result<reqwest::Response, reqwest::Error> {
        let response = client.get(url).send().await?;
        Ok(response)
    }

    pub async fn fetch_trackers(&mut self) -> Result<Vec<TorrentTracker>, anyhow::Error> {
        let mirrors = vec![
            "https://raw.githubusercontent.com/ngosang/trackerslist/master/trackers_best.txt",
            "https://ngosang.github.io/trackerslist/trackers_best.txt",
            "https://cdn.jsdelivr.net/gh/ngosang/trackerslist@master/trackers_best.txt",
        ];

        let client = Client::new();
        let mut all: Vec<TorrentTracker> = vec![];
        let testing_info_hash = "51A3B1D96B198C8BB6ACDE8EC357AE7359DB2AFC";

        for url in mirrors {
            let response = self.make_request(client.clone(), url).await;

            if let Ok(response) = response {
                let body = response.text().await.unwrap();

                let trackers: Vec<String> = body
                    .split("\n")
                    .filter(|s| !s.is_empty())
                    .map(|s| s.trim().to_string())
                    .collect();

                for tracker in trackers {
                    match tracker.split("://").next().unwrap() {
                        "udp" => {
                            let udp_socket = self.fetch_socket().await?;

                            let ip = resolve_domain_to_ip(
                                tracker
                                    .replace("udp://", "")
                                    .replace("/announce", "")
                                    .clone(),
                            );

                            if let Ok(ip) = ip {
                                let connection_id = connect_udp(&udp_socket, ip).await;

                                if let Ok(connection_id) = connection_id {
                                    let response = scrape_udp(
                                        &udp_socket,
                                        ip,
                                        connection_id,
                                        vec![hex::decode(testing_info_hash)
                                            .unwrap()
                                            .as_slice()
                                            .try_into()
                                            .unwrap()],
                                    )
                                    .await;

                                    if let Ok(_) = response {
                                        all.push(TorrentTracker {
                                            uri: tracker.clone(),
                                        });
                                    }
                                }
                            }
                        }
                        "http" | "https" => {
                            let response = request_and_response_http(
                                tracker.split("/announce").next().unwrap().to_string(),
                                vec![hex::decode(testing_info_hash)
                                    .unwrap()
                                    .as_slice()
                                    .try_into()
                                    .unwrap()],
                            )
                            .await;

                            // able to fetch from tracker and have a parseable response
                            if let Ok(_) = response {
                                all.push(TorrentTracker {
                                    uri: tracker.clone(),
                                });
                            }
                        }
                        "ws" => {}
                        _ => {
                            eprintln!("Unknown protocol: {}", tracker);
                        }
                    }
                }

                return Ok(all);
            }

            warn!(
                "Cannot fetch trackers from {}, moving to next mirror: {:?}",
                url,
                response.err().unwrap()
            );
        }

        Err(anyhow::anyhow!("Cannot fetch trackers"))
    }

    pub async fn get_torrent_trackers(
        &mut self,
        info_hashes: Vec<String>,
    ) -> anyhow::Result<Vec<(String, TorrentScrapeResponse)>> {
        let udp_socket = self.fetch_socket().await?;
        let info_hashes: Vec<[u8; 20]> = info_hashes
            .iter()
            .map(|info_hash_str| {
                let info_hash_vec = hex::decode(info_hash_str).unwrap();
                let info_hash: [u8; 20] = info_hash_vec.as_slice().try_into().unwrap();

                info_hash
            })
            .collect();

        let trackers = self.get_trackers().await?;

        let mut tracker_responses: Vec<(String, TorrentScrapeResponse)> = vec![];
        let mut failed_trackers = vec![];

        for tracker in trackers {
            if let Some((count, time)) = self.timeout_trackers.get(&tracker.uri) {
                if count > &0 {
                    if time.elapsed().as_secs()
                        < match count {
                            3 => 30,
                            4 => 60,
                            5 => 120,
                            6 => 240,
                            _ => 300,
                        }
                    {
                        continue;
                    }
                }
            }

            match tracker.uri.split("://").next().unwrap() {
                "udp" => {
                    let response = self
                        .fetch_from_tracker_udp(&udp_socket, info_hashes.clone(), &tracker)
                        .await;

                    if let Ok(response) = response {
                        self.timeout_trackers.remove(&tracker.uri);

                        let stats = response.torrent_stats;

                        let mut stats_map = BTreeMap::new();

                        for (i, stats) in stats.iter().enumerate() {
                            stats_map.insert(
                                hex::encode(info_hashes[i]).to_uppercase(),
                                TorrentScrapeStats {
                                    seeders: stats.seeders.0,
                                    leechers: stats.leechers.0,
                                },
                            );
                        }

                        tracker_responses
                            .push((tracker.uri, TorrentScrapeResponse { stats: stats_map }));
                    } else {
                        debug!(
                            "Cannot fetch from tracker: {}, error: {}",
                            tracker.uri,
                            response.err().unwrap()
                        );

                        failed_trackers.push(tracker.uri.clone());
                    }
                }
                "http" | "https" => {
                    let response = request_and_response_http(
                        tracker.uri.split("/announce").next().unwrap().to_string(),
                        info_hashes.clone(),
                    )
                    .await;

                    if let Ok(response) = response {
                        self.timeout_trackers.remove(&tracker.uri);

                        let mut stats_map = BTreeMap::new();

                        for (info_hash, stats) in response.files.iter() {
                            let stats = TorrentScrapeStats {
                                seeders: stats.complete as i32,
                                leechers: stats.incomplete as i32,
                            };

                            stats_map.insert(hex::encode(info_hash.0).to_uppercase(), stats);
                        }

                        tracker_responses
                            .push((tracker.uri, TorrentScrapeResponse { stats: stats_map }));
                    } else {
                        debug!(
                            "Cannot fetch from tracker: {}, error: {}",
                            tracker.uri,
                            response.err().unwrap()
                        );

                        failed_trackers.push(tracker.uri.clone());
                    }
                }
                "ws" => {}
                _ => {
                    println!("Unknown protocol: {}", tracker.uri);
                }
            };
        }

        for failed_tracker in failed_trackers {
            if let Some((count, time)) = self.timeout_trackers.get_mut(&failed_tracker) {
                *count += 1;
                *time = Instant::now();
            } else {
                self.timeout_trackers
                    .insert(failed_tracker, (1, Instant::now()));
            }
        }

        Ok(tracker_responses)
    }

    pub async fn fetch_from_tracker_udp(
        &self,
        udp_socket: &UdpSocket,
        info_hashes: Vec<[u8; 20]>,
        tracker: &TorrentTracker,
    ) -> Result<UDPScrapeResponse, anyhow::Error> {
        let tracker_addr =
            resolve_domain_to_ip(tracker.uri.replace("udp://", "").replace("/announce", ""));

        if tracker_addr.is_err() {
            return Err(anyhow::anyhow!("Cannot resolve tracker address"));
        }

        let tracker_addr = tracker_addr.unwrap();
        let connection_id = connect_udp(&udp_socket, tracker_addr).await?;
        // let connection_id = tracker.connection_id.unwrap();

        let scrape_response =
            scrape_udp(&udp_socket, tracker_addr, connection_id, info_hashes).await?;

        Ok(scrape_response)
    }
}
