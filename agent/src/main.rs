use crate::orchestrator::Orchestrator;
use clap::Parser;
use common::config::AppConfig;
use tracing_subscriber::EnvFilter;

pub mod llm;
pub mod mcp_clients;
pub mod memory;
pub mod orchestrator;
pub mod prompter;

#[derive(Parser, Debug)]
#[command(author, version, about = "Autonomous Drone Ground Station Agent")]
struct Args {
    /// Chemin du fichier de configuration YAML
    #[arg(long, default_value = "config/config.yaml")]
    config: String,

    /// Description textuelle de la mission d'exploration
    #[arg(
        long,
        default_value = "Explore la zone Nord-Est, repère les installations industrielles et cartographie les obstacles."
    )]
    mission: String,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    let args = Args::parse();

    tracing::info!("=== Démarrage de la Station au Sol (Ground Station) ===");

    // Chargement de la configuration
    let config = AppConfig::load(&args.config)?;
    tracing::info!("Configuration chargée avec succès depuis '{}'", args.config);

    // Initialisation et démarrage de l'orchestrateur autonome
    let mut orchestrator = Orchestrator::new(config).await?;
    orchestrator.run(&args.mission).await?;

    tracing::info!("=== Arrêt de la Station au Sol ===");
    Ok(())
}
