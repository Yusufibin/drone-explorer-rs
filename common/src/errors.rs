//! Erreurs partagées pour le projet drone-explorer.

use thiserror::Error;

/// Erreurs applicatives du projet drone-explorer.
#[derive(Debug, Error)]
pub enum DroneError {
    /// Erreur de communication MAVLink.
    #[error("Erreur MAVLink: {0}")]
    Mavlink(String),

    /// Erreur de communication MCP (JSON-RPC).
    #[error("Erreur MCP: {0}")]
    Mcp(String),

    /// Erreur d'appel au LLM.
    #[error("Erreur LLM: {0}")]
    Llm(String),

    /// Erreur de vision / caméra.
    #[error("Erreur Vision: {0}")]
    Vision(String),

    /// Erreur de configuration.
    #[error("Erreur de configuration: {0}")]
    Config(String),

    /// Batterie trop faible — failsafe déclenché.
    #[error("Batterie critique: {0}%")]
    BatteryCritical(f64),

    /// Dépassement du géofence.
    #[error("Géofence dépassé: distance={0}m")]
    GeofenceViolation(f64),

    /// Timeout de communication avec le drone.
    #[error("Timeout GCS: {0}s sans réponse")]
    GcsTimeout(u64),

    /// Outil MCP introuvable.
    #[error("Outil MCP inconnu: {0}")]
    UnknownTool(String),

    /// Erreur météo.
    #[error("Erreur Météo: {0}")]
    Weather(String),
}
