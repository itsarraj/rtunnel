use clap::{Parser, Subcommand};
use tracing_subscriber::EnvFilter;

use rtunnel::{client, server};

#[derive(Parser)]
#[command(name = "rtunnel", about = "Secure local tunnel (ngrok equivalent)")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Run the tunnel server (receives public connections and bridges to the client)
    Server {
        /// Public address to listen on
        #[arg(long, default_value = "0.0.0.0:8080")]
        bind: String,
        /// Control address for the client to connect to
        #[arg(long, default_value = "0.0.0.0:9090")]
        control: String,
    },
    /// Run the tunnel client (connects to the server and forwards traffic locally)
    Client {
        /// Server's control address to connect to
        #[arg(long, default_value = "127.0.0.1:9090")]
        server: String,
        /// Local service address to forward traffic to
        #[arg(long, default_value = "127.0.0.1:3000")]
        local: String,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::builder()
                .with_default_directive("rtunnel=info".parse()?)
                .from_env_lossy(),
        )
        .init();

    let cli = Cli::parse();

    match cli.command {
        Command::Server { bind, control } => {
            server::run_server(&bind, &control).await?;
        }
        Command::Client { server, local } => {
            client::run_client(&server, &local).await?;
        }
    }

    Ok(())
}
