//! Configuration applicative chargée depuis un fichier YAML.

use serde::{Deserialize, Serialize};

/// Configuration racine de l'application.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    /// Mode simulation explicite pour le développement sans drone/API/caméra.
    #[serde(default)]
    pub simulation: SimulationConfig,
    /// Configuration du drone (MAVLink).
    pub drone: DroneConfig,
    /// Configuration du LLM (OpenRouter).
    pub llm: LlmConfig,
    /// Configuration vision (flux vidéo).
    pub vision: VisionConfig,
    /// Configuration météo.
    pub weather: WeatherConfig,
    /// Configuration mission (journalisation).
    pub mission: MissionConfig,
    /// Configuration carte / grille.
    pub map: MapConfig,
    /// Port de base pour les serveurs MCP (flight=base, vision=base+1, …).
    pub mcp_server_port: u16,
}

/// Configuration du mode simulation. Le plan cible le réel; la simulation doit
/// rester explicite pour éviter de masquer une intégration manquante.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimulationConfig {
    pub enabled: bool,
}

impl Default for SimulationConfig {
    fn default() -> Self {
        Self { enabled: false }
    }
}

/// Configuration de la connexion au drone.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DroneConfig {
    /// Chaîne de connexion MAVLink (ex : "udpin:0.0.0.0:14550").
    pub connection_string: String,
    /// Altitude maximale légale en mètres.
    pub max_altitude: f64,
    /// Vitesse maximale en m/s.
    pub max_speed: f64,
    /// Rayon du géofence en mètres.
    pub geofence_radius: f64,
    /// Seuil batterie failsafe (%).
    pub failsafe_battery_percent: f64,
    /// Timeout perte lien GCS (secondes).
    pub failsafe_gcs_timeout_secs: u64,
}

/// Configuration du LLM via OpenRouter.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmConfig {
    /// Clé API (peut contenir `${ENV_VAR}`).
    pub api_key: String,
    /// URL de base de l'API (ex : "https://openrouter.ai/api/v1").
    pub base_url: String,
    /// Modèle par défaut.
    pub default_model: String,
    /// Modèle pour la navigation.
    pub navigation_model: String,
    /// Modèle pour la vision.
    pub vision_model: String,
    /// Modèle pour les décisions critiques.
    pub critical_model: String,
    /// Modèle pour la planification.
    pub planning_model: String,
    /// Nombre max de tokens en réponse.
    pub max_tokens: u32,
    /// Température du modèle.
    pub temperature: f64,
}

/// Configuration du flux vidéo et de la détection.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VisionConfig {
    /// URL du flux vidéo.
    pub stream_url: String,
    /// Chemin vers le modèle YOLOv8.
    pub yolo_model_path: String,
    /// Confiance minimale pour les détections.
    pub min_confidence: f64,
    /// Répertoire de sauvegarde des captures.
    pub snapshot_dir: String,
}

/// Configuration de l'API météo.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeatherConfig {
    /// Clé API OpenWeatherMap.
    pub api_key: String,
    /// Latitude du centre d'opération.
    pub lat: f64,
    /// Longitude du centre d'opération.
    pub lon: f64,
}

/// Configuration de la journalisation mission.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MissionConfig {
    /// Chemin vers la base SQLite.
    pub db_path: String,
    /// Répertoire des rapports générés.
    pub reports_dir: String,
}

/// Configuration de la carte / grille d'exploration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MapConfig {
    /// Latitude du centre de la grille.
    pub center_lat: f64,
    /// Longitude du centre de la grille.
    pub center_lon: f64,
    /// Taille totale de la grille en mètres.
    pub grid_size: f64,
    /// Taille d'un secteur en mètres.
    pub sector_size: f64,
}

impl AppConfig {
    /// Charge la configuration depuis un fichier YAML.
    ///
    /// Les variables d'environnement de la forme `${VAR}` sont
    /// substituées avant le parsing.
    pub fn load(path: &str) -> anyhow::Result<Self> {
        let raw = std::fs::read_to_string(path).map_err(|e| {
            anyhow::anyhow!("Impossible de lire le fichier de config '{}': {}", path, e)
        })?;

        // Substitution simple des variables d'environnement ${VAR}
        let expanded = expand_env_vars(&raw);

        let config: AppConfig = serde_yaml::from_str(&expanded)
            .map_err(|e| anyhow::anyhow!("Erreur de parsing YAML '{}': {}", path, e))?;

        Ok(config)
    }

    /// Retourne le port d'un serveur MCP donné (flight=0, vision=1, …).
    pub fn mcp_port(&self, offset: u16) -> u16 {
        self.mcp_server_port + offset
    }
}

/// Substitue les occurrences de `${VAR_NAME}` par la valeur de la variable d'environnement.
fn expand_env_vars(input: &str) -> String {
    let mut result = input.to_string();
    // Recherche des patterns ${...}
    while let Some(start) = result.find("${") {
        if let Some(end) = result[start..].find('}') {
            let var_name = &result[start + 2..start + end];
            let value = std::env::var(var_name).unwrap_or_default();
            result = format!(
                "{}{}{}",
                &result[..start],
                value,
                &result[start + end + 1..]
            );
        } else {
            break;
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_expand_env_vars() {
        let path = std::env::var("PATH").unwrap_or_default();
        let input = "key: \"${PATH}\"";
        let expanded = expand_env_vars(input);
        assert_eq!(expanded, format!("key: \"{}\"", path));
    }
}
