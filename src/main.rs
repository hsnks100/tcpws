mod processor;
use anyhow::Ok;
use tokio_tungstenite::tungstenite::Message;
use std::env;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
    sync::mpsc::{self},
};
use futures::{SinkExt};
use futures_util::StreamExt;


#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // futures::try_join!(tcp_listen(), run());
    futures::try_join!(tcp_listen(), ws_run())?;
    Ok(())

}

async fn tcp_listen() -> anyhow::Result<()> {
    let listener = TcpListener::bind("127.0.0.1:8080").await?;
    loop {
        let (mut socket, addr) = listener.accept().await?;
        let mut buf = [0; 1024];
        let (tx, mut rx) = mpsc::unbounded_channel::<Vec<u8>>();
        let (mut s_reader, mut s_writer) = socket.into_split();
        tokio::spawn(async move {
            loop {
                println!("SENDER START!!TCP");
                let recv = rx.recv().await;
                let recv = match recv {
                    Some(v) => v,
                    None =>  {
                        return Ok(());
                    }
                };
                if recv.len() > 0 {
                    let r = s_writer.write(recv.as_slice()).await;
                    let r = match r{
                        Result::Ok(s) => s,
                        Err(e) => {
                            println!("write error {:#?}", e);
                            return Ok(());
                        }
                    };
                }
            }
            // Ok(())
        });
        tokio::spawn(async move {
            loop {
                // 실제로 받는 부분
                let n = s_reader.read(&mut buf).await?;
                if n == 0 {
                    return Ok(());
                }
                processor::processor(tx.clone(), buf.to_vec())?;
            }
        });
    }
}

pub async fn ws_run() -> anyhow::Result<()> {
    let addr = env::args()
        .nth(1)
        .unwrap_or_else(|| "127.0.0.1:8081".to_string());

    // Create the event loop and TCP listener we'll accept connections on.
    let try_socket = TcpListener::bind(&addr).await;
    let listener = try_socket.expect("Failed to bind");
    // info!("Listening on: {}", addr);

    while let Result::Ok((stream, _)) = listener.accept().await {
        tokio::spawn(ws_accept_connection(stream));
    }

    Ok(())
}

async fn ws_accept_connection(stream: TcpStream) -> anyhow::Result<()> {
    let addr = stream
        .peer_addr()
        .expect("connected streams should have a peer address");
    // info!("Peer address: {}", addr);

    let ws_stream = tokio_tungstenite::accept_async(stream)
        .await
        .expect("Error during the websocket handshake occurred");

    // info!("New WebSocket connection: {}", addr);

    let (mut write, mut read) = ws_stream.split();
    // let (tx, mut rx) = mpsc::unbounded_channel();
    let (tx, mut rx) = mpsc::unbounded_channel::<Vec<u8>>();
    tokio::spawn(async move {
        loop {
            println!("SENDER START!!ws");
            let recv = rx.recv().await;
            let recv = match recv {
                Some(v) => v,
                None => { break; }
                // None => vec![],
            };
            if recv.len() > 0 {
                let t = recv;
                let n = write.send(Message::Binary(t)).await;
                let n = match n {
                    Result::Ok(s) => s,
                    Err(e) => {
                        break;
                    }
                };
            }
        }
        Ok(())
    });
    // We should not forward messages other than text or binary.
    loop {
        let read = read.next().await;
        let read = match read {
            Some(s) => s,
            None => {break;},
        };
        let read = read?;
        processor::processor(tx.clone(), read.into_data())?;
    }
    Ok(())
}