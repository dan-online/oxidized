use serde::Deserialize;
use std::env::current_dir;
use tokio::{
    io::{AsyncBufReadExt, BufReader},
    process::Command,
    sync::mpsc::{unbounded_channel, UnboundedReceiver},
};
use tracing::error;

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
