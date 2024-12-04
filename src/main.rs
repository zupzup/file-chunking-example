use anyhow::{anyhow, Result};
use log::{error, info};
use std::{env, io::BufRead};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
    signal,
    sync::mpsc::{self, UnboundedReceiver, UnboundedSender},
};

const GET_FILE: &str = "GET_FILE";

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();
    info!("Started");
    let args: Vec<String> = env::args().collect();
    let port = args[1].clone();
    let (stdin_tx, stdin_rx) = mpsc::unbounded_channel();

    let stdin_tx_clone = stdin_tx.clone();
    std::thread::spawn(move || handle_input(stdin_tx_clone));

    // TODO: event handler sends request to the server at the port
    // TODO: incoming TCP handler interprets input, sends it via channel to event handler with addr
    // TODO: event handler fetches file, chunks it up with indices (create data structure), sends the chunks directly via TCP with prefix "FILE"
    // TODO: if tcp handler gets input starting with "FILE", it sends it to event handler, which
    // assembles the file using a hashmap <index, bytes>
    // TODO: once the whole file is received, it's saved to disk with postfix "sent" and hashed, if
    // it's the same file as the one that was sent
    tokio::spawn(async move {
        if let Err(e) = tcp_listener(port.clone(), stdin_tx.clone()).await {
            error!("Error in TCP listener: {e}");
        }
    });

    tokio::spawn(async move {
        if let Err(e) = tcp_client(stdin_rx).await {
            error!("Error in TCP client: {e}");
        }
    });

    signal::ctrl_c().await.expect("Failed to listen for Ctrl-C");

    info!("Ctrl-C received, shutting down server...");
    Ok(())
}

#[derive(Clone, Debug)]
enum Action {
    GetFile,
}

#[derive(Clone, Debug)]
struct Request {
    action: Action,
    port: String,
}

fn parse_request(input: String) -> Result<Request> {
    if input.starts_with(GET_FILE) {
        return Ok(Request {
            action: Action::GetFile,
            port: input
                .strip_prefix(GET_FILE)
                .expect("checked that the prefix is there")
                .to_owned(),
        });
    }
    Err(anyhow!("invalid request: {input}"))
}

async fn tcp_client(mut receiver: UnboundedReceiver<String>) -> Result<()> {
    while let Some(input) = receiver.recv().await {
        let request = parse_request(input);
        info!("incoming request: {request:?}");
    }
    Ok(())
}

async fn tcp_listener(port: String, sender: UnboundedSender<String>) -> Result<()> {
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

fn handle_input(sender: UnboundedSender<String>) {
    let stdin = std::io::stdin();
    for line in stdin.lock().lines() {
        match line {
            Ok(line) => {
                let line = line.trim().to_string();
                if !line.is_empty() {
                    if let Err(e) = sender.send(line) {
                        error!("STDIN crashed: {e}");
                    }
                }
            }
            Err(e) => {
                error!("Couldn't read line from STDIN: {e}");
            }
        }
    }
}
