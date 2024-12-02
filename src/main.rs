use anyhow::Result;
use log::{error, info};
use std::env;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
    signal,
};

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();
    info!("Started");
    let args: Vec<String> = env::args().collect();
    let port = args[1].clone();

    tokio::spawn(async move {
        if let Err(e) = tcp_listener(port.clone()).await {
            error!("Error in TCP listener: {e}");
        }
    });
    signal::ctrl_c().await.expect("Failed to listen for Ctrl-C");

    info!("Ctrl-C received, shutting down server...");
    Ok(())
}

async fn tcp_listener(port: String) -> Result<()> {
    let listener = TcpListener::bind(&format!("127.0.0.1:{}", port)).await?;
    info!("Listening on port {}", port);

    loop {
        let (mut socket, addr) = listener.accept().await?;
        info!("Accepted connection from {addr:?}");

        tokio::spawn(async move {
            let mut buf = [0; 512];

            match socket.read(&mut buf).await {
                Ok(0) => {
                    info!("Connection closed by client.")
                }
                Ok(len) => {
                    info!("Received {} bytes from client", len);

                    if let Err(e) = socket.write_all("Hello World".as_bytes()).await {
                        error!("Could not write to client: {e}");
                    }
                }
                Err(e) => {
                    error!("Could not read from client: {e}");
                }
            }
        });
    }
}
