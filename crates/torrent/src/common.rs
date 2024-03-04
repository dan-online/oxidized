use std::{io::Cursor, net::SocketAddr, time::Duration};

use anyhow::Context;
use aquatic_udp_protocol::{
    ConnectRequest, ConnectionId, InfoHash as UDPInfoHash, Request, Response as UDPResponse,
    ScrapeRequest, ScrapeResponse as ScrapeUDPResponse, TransactionId,
};
use tokio::net::UdpSocket;

pub async fn connect_udp(
    socket: &UdpSocket,
    tracker_addr: SocketAddr,
) -> anyhow::Result<ConnectionId> {
    let request = Request::Connect(ConnectRequest {
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
    let request = Request::Scrape(ScrapeRequest {
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
    request: Request,
) -> anyhow::Result<UDPResponse> {
    let mut buffer = [0u8; 8192];

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
        let (bytes_read, _) =
            match tokio::time::timeout(Duration::from_secs(5), socket.recv_from(&mut buffer))
                .await?
            {
                Ok(a) => Ok(a),
                Err(e) => {
                    println!("Error: {:?}", e);
                    Err(anyhow::anyhow!("timeout"))
                }
            }?;

        UDPResponse::from_bytes(&buffer[..bytes_read], true).with_context(|| "parse response")
    }
}
