use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

#[tokio::test]
async fn test_tunnel_end_to_end() {
    // 1. Start the local target server
    let local_listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let local_port = local_listener.local_addr().unwrap().port();
    
    tokio::spawn(async move {
        let (mut stream, _) = local_listener.accept().await.unwrap();
        let mut buf = [0; 1024];
        let n = stream.read(&mut buf).await.unwrap();
        let request = String::from_utf8_lossy(&buf[..n]);
        assert_eq!(request, "PING\n");
        stream.write_all(b"PONG\n").await.unwrap();
    });

    // 2. Start the rtunnel server
    let public_listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let public_port = public_listener.local_addr().unwrap().port();
    drop(public_listener); // Free the port for run_server
    
    let control_listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let control_port = control_listener.local_addr().unwrap().port();
    drop(control_listener); // Free the port for run_server
    
    let public_addr = format!("127.0.0.1:{}", public_port);
    let control_addr = format!("127.0.0.1:{}", control_port);
    
    let pa = public_addr.clone();
    let ca = control_addr.clone();
    tokio::spawn(async move {
        rtunnel::server::run_server(&pa, &ca).await.unwrap();
    });

    // 3. Wait a bit for server to start
    tokio::time::sleep(Duration::from_millis(100)).await;

    // 4. Start the rtunnel client
    let ca2 = control_addr.clone();
    let la = format!("127.0.0.1:{}", local_port);
    tokio::spawn(async move {
        rtunnel::client::run_client(&ca2, &la).await.unwrap();
    });

    // 5. Wait a bit for client to register
    tokio::time::sleep(Duration::from_millis(100)).await;

    // 6. Connect to the public port and send a request
    let mut public_conn = tokio::net::TcpStream::connect(&public_addr).await.unwrap();
    public_conn.write_all(b"PING\n").await.unwrap();
    
    let mut response = [0; 1024];
    let n = public_conn.read(&mut response).await.unwrap();
    assert_eq!(String::from_utf8_lossy(&response[..n]), "PONG\n");
}
