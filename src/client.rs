use anyhow::{Context, Result};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;
use tracing::{error, info, warn};

use crate::protocol::{ControlMessage, DataMessage};

pub async fn run_client(server_addr: &str, local_addr: &str) -> Result<()> {
    info!("Connecting to control server at {}", server_addr);
    
    let mut control_stream = TcpStream::connect(server_addr).await.context("failed to connect to server control port")?;
    
    // Register
    let reg_msg = serde_json::to_string(&ControlMessage::Register)? + "\n";
    control_stream.write_all(reg_msg.as_bytes()).await?;
    
    info!("Registered with control server. Forwarding public traffic to {}", local_addr);
    
    let mut reader = BufReader::new(control_stream);
    
    loop {
        let mut line = String::new();
        let bytes_read = reader.read_line(&mut line).await?;
        if bytes_read == 0 {
            info!("Control server disconnected");
            break;
        }
        
        match serde_json::from_str::<ControlMessage>(&line) {
            Ok(ControlMessage::Open { id }) => {
                let s_addr = server_addr.to_string();
                let l_addr = local_addr.to_string();
                
                tokio::spawn(async move {
                    if let Err(e) = handle_open_request(id, &s_addr, &l_addr).await {
                        warn!("Failed to handle open request for {}: {:?}", id, e);
                    }
                });
            }
            Ok(ControlMessage::Register) => {
                warn!("Received unexpected Register message from server");
            }
            Err(e) => {
                warn!("Failed to parse control message: {:?}", e);
            }
        }
    }
    
    Ok(())
}

async fn handle_open_request(id: uuid::Uuid, server_addr: &str, local_addr: &str) -> Result<()> {
    // 1. Connect to local service
    let mut local_stream = TcpStream::connect(local_addr).await.context("failed to connect to local service")?;
    
    // 2. Connect to server control port for data channel
    let mut server_stream = TcpStream::connect(server_addr).await.context("failed to connect data channel to server")?;
    
    // 3. Send Bind
    let bind_msg = serde_json::to_string(&DataMessage::Bind { id })? + "\n";
    server_stream.write_all(bind_msg.as_bytes()).await?;
    
    info!("Established data channel and local connection for {}", id);
    
    // 4. Bridge
    tokio::io::copy_bidirectional(&mut server_stream, &mut local_stream).await?;
    
    Ok(())
}
