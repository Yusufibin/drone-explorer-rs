use clap::Parser;
use tracing_subscriber::EnvFilter;
use weather_server::openweather::OpenWeatherClient;
use weather_server::server::start_server;

#[derive(Parser, Debug)]
#[command(author, version, about = "Weather MCP Server for Drone Explorer")]
struct Args {
    /// Port du serveur MCP
    #[arg(long, default_value_t = 9003)]
    port: u16,

    /// Clé API OpenWeatherMap
    #[arg(long, env = "OPENWEATHER_API_KEY", default_value = "")]
    api_key: String,

    /// Latitude de l'opération
    #[arg(long, default_value_t = 48.8566)]
    lat: f64,

    /// Longitude de l'opération
    #[arg(long, default_value_t = 2.3522)]
    lon: f64,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    let args = Args::parse();

    tracing::info!("Démarrage de weather-mcp...");
    let client = OpenWeatherClient::new(&args.api_key, args.lat, args.lon);

    start_server(client, args.port).await?;

    Ok(())
}
