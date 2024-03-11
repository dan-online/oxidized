use librqbit::{
    AddTorrent, AddTorrentOptions, AddTorrentResponse, ByteString, Session, SessionOptions,
    TorrentMetaV1Info,
};
use std::{
    net::{SocketAddr, ToSocketAddrs},
    path::PathBuf,
    sync::Arc,
    time::Duration,
};

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
