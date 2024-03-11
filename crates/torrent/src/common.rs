use std::{io::Cursor, net::SocketAddr, time::Duration};

use anyhow::Context;
use aquatic_http_protocol::response::Response as HTTPResponse;
use aquatic_http_protocol::response::ScrapeResponse as HTTPScrapeResponse;
use aquatic_udp_protocol::{
    ConnectRequest, ConnectionId, InfoHash as UDPInfoHash, Request as UDPRequest,
    Response as UDPResponse, ScrapeRequest as UDPScrapeRequest,
    ScrapeResponse as ScrapeUDPResponse, TransactionId,
};
use tokio::net::UdpSocket;

pub async fn connect_udp(
    socket: &UdpSocket,
    tracker_addr: SocketAddr,
) -> anyhow::Result<ConnectionId> {
    let request = UDPRequest::Connect(ConnectRequest {
        transaction_id: TransactionId(0),
    });

    let response = request_and_response_udp(socket, tracker_addr, request).await?;

    if let UDPResponse::Connect(response) = response {
        Ok(response.connection_id)
    } else {
        Err(anyhow::anyhow!("not connect response: {:?}", response))
    }
}

pub async fn scrape_udp(
    socket: &UdpSocket,
    tracker_addr: SocketAddr,
    connection_id: ConnectionId,
    info_hashes: Vec<[u8; 20]>,
) -> anyhow::Result<ScrapeUDPResponse> {
    let request = UDPRequest::Scrape(UDPScrapeRequest {
        connection_id,
        transaction_id: TransactionId(0),
        info_hashes: info_hashes
            .into_iter()
            .map(|info_hash| UDPInfoHash(info_hash))
            .collect(),
    });

    let response = request_and_response_udp(socket, tracker_addr, request).await?;

    if let UDPResponse::Scrape(response) = response {
        Ok(response)
    } else {
        Err(anyhow::anyhow!("not scrape response: {:?}", response))
    }
}

pub async fn request_and_response_udp(
    socket: &UdpSocket,
    tracker_addr: SocketAddr,
    request: UDPRequest,
) -> anyhow::Result<UDPResponse> {
    // 128kb should be enough for any request
    let mut buffer = [0u8; 128 * 1024];

    {
        let mut buffer = Cursor::new(&mut buffer[..]);

        request
            .write(&mut buffer)
            .with_context(|| "write request")?;

        let bytes_written = buffer.position() as usize;

        socket
            .send_to(&(buffer.into_inner())[..bytes_written], tracker_addr)
            .await?;
    }

    {
        let (bytes_read, _addr) =
            tokio::time::timeout(Duration::from_secs(5), socket.recv_from(&mut buffer)).await??;

        UDPResponse::from_bytes(&buffer[..bytes_read], true).with_context(|| "parse response")
    }
}

pub async fn request_and_response_http(
    tracker_uri: String,
    info_hashes: Vec<[u8; 20]>,
) -> anyhow::Result<HTTPScrapeResponse> {
    let mut url = format!("{}/scrape", tracker_uri,);

    for (i, info_hash) in info_hashes.iter().enumerate() {
        url.push_str(&format!(
            "{}info_hash={}",
            if i == 0 { "?" } else { "&" },
            url::form_urlencoded::byte_serialize(info_hash.to_vec().as_slice()).collect::<String>()
        ));
    }

    let response = reqwest::Client::new()
        .get(&url)
        .timeout(Duration::from_secs(5))
        .send()
        .await?;

    let response = HTTPResponse::from_bytes(&response.bytes().await?);

    if let Ok(HTTPResponse::Scrape(response)) = response {
        Ok(response)
    } else {
        Err(anyhow::anyhow!(
            "not scrape response: {:?}",
            response.unwrap_err()
        ))
    }
}
