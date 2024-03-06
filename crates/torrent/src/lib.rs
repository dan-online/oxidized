mod common;
pub mod nsfw_filter;

use aquatic_udp_protocol::{ConnectionId, ScrapeResponse as UDPScrapeResponse};
use common::*;
use librqbit::{
    AddTorrent, AddTorrentOptions, AddTorrentResponse, ByteString, Session, SessionOptions,
    TorrentMetaV1Info,
};
use reqwest::Client;
use serde::Deserialize;
use std::{
    env::current_dir,
    net::{Ipv4Addr, SocketAddr, SocketAddrV4, ToSocketAddrs},
    path::PathBuf,
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::{
    io::{AsyncBufReadExt, BufReader},
    net::UdpSocket,
    process::Command,
    sync::mpsc::{unbounded_channel, UnboundedReceiver},
};
use tracing::{error, info, warn};

pub fn resolve_domain_to_ip(uri: String) -> anyhow::Result<SocketAddr> {
    let ips: Vec<_> = uri.to_socket_addrs()?.collect();

    Ok(ips[0])
}

pub struct TorrentInfo {
    session: Arc<Session>,
}

impl TorrentInfo {
    pub async fn new() -> Result<Self, anyhow::Error> {
        let session = Session::new_with_opts(
            PathBuf::from("/tmp/downloads"),
            SessionOptions {
                ..Default::default()
            },
        )
        .await?;
        Ok(Self { session })
    }

    pub async fn get_torrent_info(
        &self,
        info_hash: &str,
    ) -> Result<TorrentMetaV1Info<ByteString>, anyhow::Error> {
        let magnet_url = format!("magnet:?xt=urn:btih:{info_hash}");

        let info = self.session.add_torrent(
            AddTorrent::from_url(magnet_url),
            Some(AddTorrentOptions {
                overwrite: true,
                list_only: true,

                ..Default::default()
            }),
        );

        let info = tokio::time::timeout(Duration::from_secs(10), info).await??;

        let info = match info {
            AddTorrentResponse::ListOnly(res) => res,
            _ => {
                return Err(anyhow::anyhow!("Invalid response"));
            }
        };

        Ok(info.info)
    }
}

#[derive(Clone)]
pub struct TorrentTrackers {
    last_updated_trackers: Instant,
    trackers: Vec<(String, ConnectionId)>,
}

impl TorrentTrackers {
    pub async fn new() -> Result<Self, anyhow::Error> {
        let mut new = Self {
            last_updated_trackers: Instant::now(),
            trackers: vec![],
        };

        new.get_trackers().await?;

        info!(
            "Torrent tracker initialized with {} trackers",
            new.trackers.len()
        );

        Ok(new)
    }

    pub async fn fetch_socket(&self) -> Result<UdpSocket, anyhow::Error> {
        let peer_addr = SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, 0));
        let socket = UdpSocket::bind(peer_addr).await?;

        Ok(socket)
    }

    pub async fn get_trackers(&mut self) -> Result<Vec<(String, ConnectionId)>, anyhow::Error> {
        let now = Instant::now();

        if now.duration_since(self.last_updated_trackers).as_secs() > 3600
            || self.trackers.is_empty()
        {
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

    pub async fn fetch_trackers(&self) -> Result<Vec<(String, ConnectionId)>, anyhow::Error> {
        let mirrors = vec![
            "https://raw.githubusercontent.com/ngosang/trackerslist/master/trackers_best.txt",
            "https://ngosang.github.io/trackerslist/trackers_best.txt",
            "https://cdn.jsdelivr.net/gh/ngosang/trackerslist@master/trackers_best.txt",
        ];

        let client = Client::new();
        let udp_socket = self.fetch_socket().await?;
        let mut all = vec![];

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
                            let ip = resolve_domain_to_ip(
                                tracker
                                    .replace("udp://", "")
                                    .replace("/announce", "")
                                    .clone(),
                            );

                            if let Ok(ip) = ip {
                                let connection_id = connect_udp(&udp_socket, ip).await;

                                if let Ok(connection_id) = connection_id {
                                    all.push((tracker, connection_id));
                                } else {
                                    warn!(
                                        "Cannot connect to tracker: {}, error: {}",
                                        tracker,
                                        connection_id.err().unwrap()
                                    );
                                }
                            }
                        }
                        "http" | "https" => {}
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
    ) -> anyhow::Result<Vec<(String, UDPScrapeResponse)>> {
        let udp_socket = self.fetch_socket().await?;

        // let info_hash_vec = hex::decode(info_hash_str).unwrap();
        // let info_hash: [u8; 20] = info_hash_vec.as_slice().try_into().unwrap();

        let info_hashes: Vec<[u8; 20]> = info_hashes
            .iter()
            .map(|info_hash_str| {
                let info_hash_vec = hex::decode(info_hash_str).unwrap();
                let info_hash: [u8; 20] = info_hash_vec.as_slice().try_into().unwrap();

                info_hash
            })
            .collect();

        let trackers = self.get_trackers().await?;

        let mut tracker_responses = vec![];

        for tracker in trackers {
            match tracker.0.split("://").next().unwrap() {
                "udp" => {
                    let response = self
                        .fetch_from_tracker_udp(&udp_socket, info_hashes.clone(), &tracker)
                        .await;

                    if let Ok(response) = response {
                        tracker_responses.push((tracker.0, response));
                    }
                }
                "http" | "https" => {
                    // let response = fetch_from_tracker_http(info_hash, &tracker).await;
                    // println!("Response: {:?}", response);
                }
                "ws" => {}
                _ => {
                    println!("Unknown protocol: {}", tracker.0);
                }
            };
        }

        Ok(tracker_responses)
    }

    pub async fn fetch_from_tracker_udp(
        &self,
        udp_socket: &UdpSocket,
        info_hashes: Vec<[u8; 20]>,
        tracker: &(String, ConnectionId),
    ) -> Result<UDPScrapeResponse, anyhow::Error> {
        let tracker_addr =
            resolve_domain_to_ip(tracker.0.replace("udp://", "").replace("/announce", ""));

        if tracker_addr.is_err() {
            return Err(anyhow::anyhow!("Cannot resolve tracker address"));
        }

        let tracker_addr = tracker_addr.unwrap();

        let scrape_response = scrape_udp(udp_socket, tracker_addr, tracker.1, info_hashes).await?;

        Ok(scrape_response)
    }
}

#[derive(Deserialize)]

pub struct MagneticoDFile {
    pub size: i64,
    pub path: String,
}

#[derive(Deserialize)]
pub struct MagneticoDTorrent {
    pub name: String,
    #[serde(rename = "infoHash")]
    pub info_hash: String,
    pub files: Vec<MagneticoDFile>,
}

pub struct Spider {
    path: String,
}

impl Spider {
    pub fn new() -> Self {
        Self {
            path: current_dir()
                .unwrap()
                .join("magneticod")
                .to_str()
                .unwrap()
                .to_string(),
        }
    }

    pub async fn start(&self) -> UnboundedReceiver<MagneticoDTorrent> {
        let (tx, rx) = unbounded_channel();

        let path = self.path.clone();
        let mut child = Command::new(path)
            .arg("--database=stdout://")
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::null())
            .spawn()
            .unwrap();

        if let Some(stdout) = child.stdout.take() {
            let mut lines = BufReader::new(stdout).lines();
            tokio::spawn(async move {
                while let Some(line) = lines.next_line().await.unwrap() {
                    let torrent: MagneticoDTorrent = serde_json::from_str(&line).unwrap();

                    match tx.send(torrent) {
                        Ok(_) => {}
                        Err(e) => {
                            error!("Cannot send torrent to channel: {:?}", e);
                            break;
                        }
                    }
                }
            });
        }

        rx
    }
}
