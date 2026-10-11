use std::collections::HashMap;
use std::sync::Arc;

use anyhow::{Context, Result};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{mpsc, oneshot, Mutex};
use tracing::{error, info, warn};
use uuid::Uuid;

use crate::protocol::{ControlMessage, DataMessage};

type ConnectionMap = Arc<Mutex<HashMap<Uuid, oneshot::Sender<TcpStream>>>>;

pub async fn run_server(public_addr: &str, control_addr: &str) -> Result<()> {
    let control_listener = TcpListener::bind(control_addr).await.context("failed to bind control port")?;
    let public_listener = TcpListener::bind(public_addr).await.context("failed to bind public port")?;

    info!("Server listening. Public: {}, Control: {}", public_addr, control_addr);

    let pending_connections: ConnectionMap = Arc::new(Mutex::new(HashMap::new()));
    let active_control: Arc<Mutex<Option<mpsc::Sender<ControlMessage>>>> = Arc::new(Mutex::new(None));

    loop {
        tokio::select! {
            result = control_listener.accept() => {
                let (stream, addr) = result?;
                info!("Internal connection received from {}", addr);
                
                let pending = pending_connections.clone();
                let active = active_control.clone();
                tokio::spawn(async move {
                    if let Err(e) = handle_internal_connection(stream, pending, active).await {
                        warn!("Internal connection error: {:?}", e);
                    }
                });
            }
            result = public_listener.accept() => {
                let (mut public_stream, addr) = result?;
                info!("Public connection received from {}", addr);
                
                let active = active_control.clone();
                let pending = pending_connections.clone();
                
                tokio::spawn(async move {
                    let id = Uuid::new_v4();
                    
                    let control_tx = {
                        let lock = active.lock().await;
                        lock.clone()
                    };
                    
                    if let Some(tx) = control_tx {
                        let (data_tx, data_rx) = oneshot::channel();
                        {
                            let mut p = pending.lock().await;
                            p.insert(id, data_tx);
                        }
                        
                        if tx.send(ControlMessage::Open { id }).await.is_err() {
                            warn!("Lost control connection, dropping public client");
                            let mut p = pending.lock().await;
                            p.remove(&id);
                            return;
                        }
                        
                        // Wait for the client to open the data channel
                        match tokio::time::timeout(std::time::Duration::from_secs(10), data_rx).await {
                            Ok(Ok(mut data_stream)) => {
                                info!("Data channel established for {}, bridging", id);
                                if let Err(e) = tokio::io::copy_bidirectional(&mut public_stream, &mut data_stream).await {
                                    warn!("Bridge error for {}: {:?}", id, e);
                                }
                                info!("Connection {} closed", id);
                            }
                            Ok(Err(_)) => {
                                warn!("Data channel setup aborted for {}", id);
                            }
                            Err(_) => {
                                warn!("Data channel setup timed out for {}", id);
                                let mut p = pending.lock().await;
                                p.remove(&id);
                            }
                        }
                    } else {
                        warn!("No active control connection to handle public request from {}", addr);
                    }
                });
            }
        }
    }
}

async fn handle_internal_connection(
    stream: TcpStream, 
    pending: ConnectionMap,
    active_control: Arc<Mutex<Option<mpsc::Sender<ControlMessage>>>>
) -> Result<()> {
    let mut reader = BufReader::new(stream);
    let mut line = String::new();
    reader.read_line(&mut line).await?;

    if let Ok(DataMessage::Bind { id }) = serde_json::from_str(&line) {
        let mut map = pending.lock().await;
        if let Some(sender) = map.remove(&id) {
            // Strip the BufReader and pass the raw stream
            let raw_stream = reader.into_inner();
            let _ = sender.send(raw_stream);
        } else {
            warn!("No pending connection found for id: {}", id);
        }
        return Ok(());
    }

    if let Ok(ControlMessage::Register) = serde_json::from_str(&line) {
        info!("Control client registered");
        let (tx, mut rx) = mpsc::channel(32);
        
        {
            let mut lock = active_control.lock().await;
            *lock = Some(tx);
        }
        
        let mut raw_stream = reader.into_inner();
        
        // Forward ControlMessages to the client
        while let Some(msg) = rx.recv().await {
            let mut msg_str = serde_json::to_string(&msg)?;
            msg_str.push('\n');
            if let Err(e) = raw_stream.write_all(msg_str.as_bytes()).await {
                warn!("Failed to send control message: {:?}", e);
                break;
            }
        }
        
        // Clear active control if it matches this one
        {
            let mut lock = active_control.lock().await;
            *lock = None;
        }
        
        info!("Control client disconnected");
        return Ok(());
    }
    
    Err(anyhow::anyhow!("Expected Bind or Register message"))
}
