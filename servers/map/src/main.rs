use clap::Parser;
use map_server::builder::MapBuilder;
use map_server::server::start_server;
use tracing_subscriber::EnvFilter;

#[derive(Parser, Debug)]
#[command(author, version, about = "Map MCP Server for Drone Explorer")]
struct Args {
    /// Port du serveur MCP
    #[arg(long, default_value_t = 9004)]
    port: u16,

    /// Latitude du centre de la carte
    #[arg(long, default_value_t = 48.8566)]
    center_lat: f64,

    /// Longitude du centre de la carte
    #[arg(long, default_value_t = 2.3522)]
    center_lon: f64,

    /// Taille totale de la grille d'exploration en mètres
    #[arg(long, default_value_t = 500.0)]
    grid_size: f64,

    /// Taille d'un secteur unitaire d'exploration en mètres
    #[arg(long, default_value_t = 100.0)]
    sector_size: f64,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    let args = Args::parse();

    tracing::info!("Démarrage de map-mcp...");
    let builder = MapBuilder::new(
        args.center_lat,
        args.center_lon,
        args.grid_size,
        args.sector_size,
    );

    start_server(builder, args.port).await?;

    Ok(())
}
