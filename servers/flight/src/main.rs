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

    #[arg(long, default_value_t = 80.0)]
    max_altitude: f64,
    #[arg(long, default_value_t = 15.0)]
    max_speed: f64,
    #[arg(long, default_value_t = 200.0)]
    geofence_radius: f64,
    #[arg(long, default_value_t = 25.0)]
    min_battery: f32,
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
    let bridge = MavlinkBridge::new(&args.mavlink_url).with_limits(
        args.max_altitude,
        args.max_speed,
        args.geofence_radius,
        args.min_battery,
    )?;

    // Tentative de connexion
    bridge.connect().await?;

    // Démarrage du serveur
    start_server(bridge, args.port).await?;

    Ok(())
}
