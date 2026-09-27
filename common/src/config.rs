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
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SimulationConfig {
    pub enabled: bool,
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
        config.validate()?;
        Ok(config)
    }

    pub fn validate(&self) -> anyhow::Result<()> {
        anyhow::ensure!(
            self.mcp_server_port <= u16::MAX - 4,
            "Ports MCP hors limites"
        );
        anyhow::ensure!(
            self.drone.max_altitude.is_finite() && self.drone.max_altitude > 0.0,
            "Altitude maximale invalide"
        );
        anyhow::ensure!(
            self.drone.max_speed.is_finite() && self.drone.max_speed > 0.0,
            "Vitesse maximale invalide"
        );
        anyhow::ensure!(
            self.drone.geofence_radius.is_finite() && self.drone.geofence_radius > 0.0,
            "Geofence invalide"
        );
        anyhow::ensure!(
            self.drone.failsafe_battery_percent.is_finite()
                && (0.0..=100.0).contains(&self.drone.failsafe_battery_percent),
            "Seuil batterie invalide"
        );
        anyhow::ensure!(
            self.weather.lat.is_finite()
                && (-90.0..=90.0).contains(&self.weather.lat)
                && self.weather.lon.is_finite()
                && (-180.0..=180.0).contains(&self.weather.lon),
            "Coordonnées météo invalides"
        );
        if self.simulation.enabled {
            anyhow::ensure!(
                self.drone.connection_string.starts_with("sim://")
                    && self.vision.stream_url.starts_with("sim://")
                    && self.vision.yolo_model_path.starts_with("sim://")
                    && self.weather.api_key == "sim://openweather",
                "Simulation incohérente : utiliser uniquement des services sim://"
            );
        } else {
            anyhow::bail!(
                "Vol réel désactivé : le contrôle MAVLink, la vidéo et l'inférence ne sont pas implémentés. Utiliser config/simulation.yaml pour les essais."
            );
        }
        Ok(())
    }

    /// Retourne le port d'un serveur MCP donné (flight=0, vision=1, …).
    pub fn mcp_port(&self, offset: u16) -> u16 {
        self.mcp_server_port + offset
    }
}

/// Substitue les occurrences de `${VAR_NAME}` par la valeur de la variable d'environnement.
fn expand_env_vars(input: &str) -> String {
    let mut result = String::new();
    let mut rest = input;
    while let Some(start) = rest.find("${") {
        result.push_str(&rest[..start]);
        let after = &rest[start + 2..];
        if let Some(end) = after.find('}') {
            let value = std::env::var(&after[..end]).unwrap_or_default();
            // YAML double quoted scalars accept JSON string escapes.
            let escaped = serde_json::to_string(&value).unwrap_or_else(|_| "\"\"".into());
            result.push_str(&escaped[1..escaped.len() - 1]);
            rest = &after[end + 1..];
        } else {
            result.push_str(&rest[start..]);
            rest = "";
            break;
        }
    }
    result.push_str(rest);
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
        let parsed: serde_yaml::Value = serde_yaml::from_str(&expanded).unwrap();
        assert_eq!(parsed["key"].as_str(), Some(path.as_str()));
    }

    #[test]
    fn config_rejects_real_mode_and_accepts_simulator() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap();
        let sim = root.join("config/simulation.yaml");
        assert!(AppConfig::load(sim.to_str().unwrap()).is_ok());
        let real = root.join("config/config.yaml");
        assert!(AppConfig::load(real.to_str().unwrap()).is_err());
    }
}
