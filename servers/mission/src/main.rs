use clap::Parser;
use mission_server::db::MissionDatabase;
use mission_server::server::start_server;
use tracing_subscriber::EnvFilter;

#[derive(Parser, Debug)]
#[command(author, version, about = "Mission MCP Server for Drone Explorer")]
struct Args {
    /// Port du serveur MCP
    #[arg(long, default_value_t = 9005)]
    port: u16,

    /// Chemin de la base de données SQLite
    #[arg(long, default_value = "data/mission.db")]
    db_path: String,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    let args = Args::parse();

    tracing::info!("Démarrage de mission-mcp...");
    let db = MissionDatabase::new(&args.db_path)?;

    start_server(db, args.port).await?;

    Ok(())
}
