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

// pub fn urlencode_20_bytes(input: [u8; 20]) -> anyhow::Result<String> {
//     let mut tmp = [b'%'; 60];

//     for i in 0..input.len() {
//         hex::encode_to_slice(&input[i..i + 1], &mut tmp[i * 3 + 1..i * 3 + 3]).unwrap();
//     }

//     let tmp = String::from_utf8(tmp.to_vec()).with_context(|| "from_utf8")?;

//     Ok(tmp)
// }

// pub async fn scrape_http(address: Url, info_hash: [u8; 20]) -> anyhow::Result<HTTPScrapeResponse> {
//     // add ?info_hash=urlencode_20_bytes(info_hash) to the address
//     let client = Client::new();

//     println!(
//         "Address: {:?} - {}",
//         address.as_str(),
//         format!(
//             "{}?info_hash={}&uploaded=0&downloaded=0&left=0&event=stopped&numwant=0&compact=1",
//             address.to_string(),
//             urlencode_20_bytes(info_hash).unwrap()
//         )
//     );

//     let response = client
//         .get(format!(
//             "{}?info_hash={}&uploaded=0&downloaded=0&left=0&event=stopped&numwant=0&compact=1",
//             address.to_string(),
//             urlencode_20_bytes(info_hash).unwrap()
//         ))
//         .send()
//         .await?;

//     let bytes = &response.bytes().await?;

//     // println!("Bytes: {:?}", response.text().await?);

//     let res = HTTPResponse::from_bytes(bytes).with_context(|| "parse response")?;

//     if let HTTPResponse::Scrape(res) = res {
//         Ok(res)
//     } else {
//         Err(anyhow::anyhow!("not scrape response: {:?}", res))
//     }
//     // Err(anyhow::anyhow!("not scrape response"))
// }

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
            match tokio::time::timeout(Duration::from_secs(1), socket.recv_from(&mut buffer))
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
