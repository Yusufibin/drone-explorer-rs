use clap::Parser;
use tracing_subscriber::EnvFilter;
use vision_server::detector::ObjectDetector;
use vision_server::server::start_server;
use vision_server::stream_reader::StreamReader;

#[derive(Parser, Debug)]
#[command(author, version, about = "Vision MCP Server for Drone Explorer")]
struct Args {
    /// Port du serveur MCP
    #[arg(long, default_value_t = 9002)]
    port: u16,

    /// URL du flux vidéo (UDP ou RTSP)
    #[arg(long, default_value = "udp://0.0.0.0:5600")]
    stream_url: String,

    /// Chemin du modèle YOLOv8 (.pt)
    #[arg(long, default_value = "models/yolov8n.pt")]
    model_path: String,

    /// Répertoire pour les captures d'images
    #[arg(long, default_value = "snapshots")]
    snapshot_dir: String,

    /// Seuil de confiance minimal pour les détections
    #[arg(long, default_value_t = 0.5)]
    min_confidence: f32,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    let args = Args::parse();

    tracing::info!("Démarrage de vision-mcp...");

    let mut reader = StreamReader::new(&args.stream_url);
    reader.connect().await?;

    let detector = ObjectDetector::new(&args.model_path, args.min_confidence);

    start_server(reader, detector, args.port).await?;

    Ok(())
}
