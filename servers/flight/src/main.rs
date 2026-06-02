use clap::Parser;
use flight_server::mavlink_bridge::MavlinkBridge;
use flight_server::server::start_server;
use tracing_subscriber::EnvFilter;

#[derive(Parser, Debug)]
#[command(author, version, about = "Flight MCP Server for Drone Explorer")]
struct Args {
    /// Port du serveur MCP
    #[arg(long, default_value_t = 9001)]
    port: u16,

    /// Chaîne de connexion MAVLink (ex : udpin:0.0.0.0:14550)
    #[arg(long, default_value = "udpin:0.0.0.0:14550")]
    mavlink_url: String,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    let args = Args::parse();

    tracing::info!("Démarrage de flight-mcp...");
    let bridge = MavlinkBridge::new(&args.mavlink_url);

    // Tentative de connexion
    bridge.connect().await?;

    // Démarrage du serveur
    start_server(bridge, args.port).await?;

    Ok(())
}
