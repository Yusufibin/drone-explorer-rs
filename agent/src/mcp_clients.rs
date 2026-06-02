use common::config::AppConfig;
use common::error::DroneError;
use common::mcp::{McpToolCall, McpToolDefinition, McpToolResult};
use common::types::{BatteryStatus, Telemetry};
use jsonrpsee::core::client::ClientT;
use jsonrpsee::core::params::ArrayParams;
use jsonrpsee::http_client::{HttpClient, HttpClientBuilder};
use serde_json::{Value, json};

/// Regroupe les clients RPC connectés à tous les serveurs MCP.
pub struct McpClients {
    pub flight: HttpClient,
    pub vision: HttpClient,
    pub weather: HttpClient,
    pub map: HttpClient,
    pub mission: HttpClient,
}

impl McpClients {
    /// Initialise les clients de tous les microservices MCP.
    pub async fn new(config: &AppConfig) -> Result<Self, DroneError> {
        let build_client = |port: u16| -> Result<HttpClient, DroneError> {
            let url = format!("http://127.0.0.1:{}", port);
            HttpClientBuilder::default().build(&url).map_err(|e| {
                DroneError::Mcp(format!("Erreur de connexion client sur '{}': {}", url, e))
            })
        };

        Ok(Self {
            flight: build_client(config.mcp_port(0))?,
            vision: build_client(config.mcp_port(1))?,
            weather: build_client(config.mcp_port(2))?,
            map: build_client(config.mcp_port(3))?,
            mission: build_client(config.mcp_port(4))?,
        })
    }

    /// Exécute un outil MCP par son nom en l'aiguillant vers le bon serveur.
    pub async fn call_tool(&self, tool_call: &McpToolCall) -> Result<McpToolResult, DroneError> {
        let name = tool_call.name.as_str();
        let args = &tool_call.arguments;

        tracing::info!("Appel outil MCP : {} avec {:?}", name, args);

        let (client, method) = match name {
            // Flight
            "takeoff" => (&self.flight, "takeoff"),
            "goto" => (&self.flight, "goto"),
            "land" => (&self.flight, "land"),
            "rtl" => (&self.flight, "rtl"),
            "get_telemetry" => (&self.flight, "get_telemetry"),
            "get_battery" => (&self.flight, "get_battery"),
            "loiter" => (&self.flight, "loiter"),
            "set_speed" => (&self.flight, "set_speed"),

            // Vision
            "analyze_frame" => (&self.vision, "analyze_frame"),
            "detect_objects" => (&self.vision, "detect_objects"),
            "get_snapshot" => (&self.vision, "get_snapshot"),
            "check_landing_zone" => (&self.vision, "check_landing_zone"),

            // Weather
            "get_conditions" => (&self.weather, "get_conditions"),
            "is_safe_to_fly" => (&self.weather, "is_safe_to_fly"),

            // Map
            "add_waypoint" => (&self.map, "add_waypoint"),
            "get_unexplored_sector" => (&self.map, "get_unexplored_sector"),
            "get_coverage_percent" => (&self.map, "get_coverage_percent"),
            "export_map" => (&self.map, "export_map"),

            // Mission
            "create_mission" => (&self.mission, "create_mission"),
            "log_finding" => (&self.mission, "log_finding"),
            "get_mission_status" => (&self.mission, "get_mission_status"),
            "complete_mission" => (&self.mission, "complete_mission"),

            _ => return Err(DroneError::UnknownTool(name.to_string())),
        };

        // Construction dynamique des paramètres pour jsonrpsee
        // Si les arguments sont un objet, on les passe tels quels si le serveur accepte des paramètres nommés.
        // Sinon, on construit une liste de paramètres positionnels basés sur la signature attendue.
        let mut params = ArrayParams::new();
        match name {
            "takeoff" => {
                let alt = args
                    .get("altitude")
                    .and_then(|v| v.as_f64())
                    .unwrap_or(10.0);
                params.insert(alt)?;
            }
            "goto" => {
                let lat = args.get("lat").and_then(|v| v.as_f64()).unwrap_or(0.0);
                let lon = args.get("lon").and_then(|v| v.as_f64()).unwrap_or(0.0);
                let alt = args.get("alt").and_then(|v| v.as_f64()).unwrap_or(20.0);
                params.insert(lat)?;
                params.insert(lon)?;
                params.insert(alt)?;
            }
            "loiter" => {
                let dur = args
                    .get("duration_s")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(10);
                params.insert(dur)?;
            }
            "set_speed" => {
                let speed = args
                    .get("speed_m_s")
                    .and_then(|v| v.as_f64())
                    .unwrap_or(5.0);
                params.insert(speed)?;
            }
            "detect_objects" => {
                let conf = args
                    .get("min_confidence")
                    .and_then(|v| v.as_f64())
                    .unwrap_or(0.5);
                params.insert(conf)?;
            }
            "get_snapshot" => {
                let save = args.get("save").and_then(|v| v.as_bool()).unwrap_or(true);
                params.insert(save)?;
            }
            "add_waypoint" => {
                let lat = args.get("lat").and_then(|v| v.as_f64()).unwrap_or(0.0);
                let lon = args.get("lon").and_then(|v| v.as_f64()).unwrap_or(0.0);
                let label = args
                    .get("label")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let photo = args
                    .get("photo_path")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());
                params.insert(lat)?;
                params.insert(lon)?;
                params.insert(label)?;
                params.insert(photo)?;
            }
            "get_unexplored_sector" => {
                let lat = args
                    .get("current_lat")
                    .and_then(|v| v.as_f64())
                    .unwrap_or(48.8566);
                let lon = args
                    .get("current_lon")
                    .and_then(|v| v.as_f64())
                    .unwrap_or(2.3522);
                params.insert(lat)?;
                params.insert(lon)?;
            }
            "export_map" => {
                let fmt = args
                    .get("format")
                    .and_then(|v| v.as_str())
                    .unwrap_or("geojson")
                    .to_string();
                params.insert(fmt)?;
            }
            "create_mission" => {
                let name = args
                    .get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Mission")
                    .to_string();
                let desc = args
                    .get("description")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());
                params.insert(name)?;
                params.insert(desc)?;
            }
            "log_finding" => {
                let mid = args
                    .get("mission_id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let lat = args.get("lat").and_then(|v| v.as_f64()).unwrap_or(0.0);
                let lon = args.get("lon").and_then(|v| v.as_f64()).unwrap_or(0.0);
                let desc = args
                    .get("description")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let photo = args
                    .get("photo_path")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());
                params.insert(mid)?;
                params.insert(lat)?;
                params.insert(lon)?;
                params.insert(desc)?;
                params.insert(photo)?;
            }
            "get_mission_status" => {
                let mid = args
                    .get("mission_id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                params.insert(mid)?;
            }
            "complete_mission" => {
                let mid = args
                    .get("mission_id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let sum = args
                    .get("summary")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                params.insert(mid)?;
                params.insert(sum)?;
            }
            // Sans paramètres ou objet vide accepté
            _ => {}
        }

        let raw_res: Value = client
            .request(method, params)
            .await
            .map_err(|e| DroneError::Mcp(format!("Erreur d'appel JSON-RPC '{}': {}", method, e)))?;

        let res: McpToolResult = serde_json::from_value(raw_res)
            .map_err(|e| DroneError::Mcp(format!("Erreur de format réponse outil: {}", e)))?;

        Ok(res)
    }

    /// Récupère la télémétrie en direct.
    pub async fn get_telemetry(&self) -> Result<Telemetry, DroneError> {
        let res = self
            .call_tool(&McpToolCall {
                name: "get_telemetry".to_string(),
                arguments: serde_json::Value::Null,
            })
            .await?;

        if res.success {
            serde_json::from_value(res.data)
                .map_err(|e| DroneError::Mcp(format!("Format télémétrie invalide: {}", e)))
        } else {
            Err(DroneError::Mcp(res.error.unwrap_or_default()))
        }
    }

    /// Récupère l'état de la batterie.
    pub async fn get_battery(&self) -> Result<BatteryStatus, DroneError> {
        let res = self
            .call_tool(&McpToolCall {
                name: "get_battery".to_string(),
                arguments: serde_json::Value::Null,
            })
            .await?;

        if res.success {
            serde_json::from_value(res.data)
                .map_err(|e| DroneError::Mcp(format!("Format batterie invalide: {}", e)))
        } else {
            Err(DroneError::Mcp(res.error.unwrap_or_default()))
        }
    }

    /// Récupère la couverture en temps réel.
    pub async fn get_coverage(&self) -> Result<f64, DroneError> {
        let res = self
            .call_tool(&McpToolCall {
                name: "get_coverage_percent".to_string(),
                arguments: serde_json::Value::Null,
            })
            .await?;

        if res.success {
            res.data
                .as_f64()
                .ok_or_else(|| DroneError::Mcp("Format couverture invalide".into()))
        } else {
            Err(DroneError::Mcp(res.error.unwrap_or_default()))
        }
    }

    /// Retourne la liste statique des définitions d'outils exposés à l'orchestrateur.
    pub fn available_tools(&self) -> Vec<McpToolDefinition> {
        vec![
            // Flight
            McpToolDefinition {
                name: "takeoff".to_string(),
                description: "Fait décoller le drone à une altitude spécifique (mètres)".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "altitude": { "type": "number", "description": "Altitude de vol stable" }
                    }
                }),
            },
            McpToolDefinition {
                name: "goto".to_string(),
                description: "Envoie le drone vers des coordonnées GPS (lat, lon, alt)".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "lat": { "type": "number" },
                        "lon": { "type": "number" },
                        "alt": { "type": "number" }
                    },
                    "required": ["lat", "lon"]
                }),
            },
            McpToolDefinition {
                name: "land".to_string(),
                description: "Fait atterrir immédiatement le drone à son emplacement actuel".to_string(),
                parameters: json!({}),
            },
            McpToolDefinition {
                name: "rtl".to_string(),
                description: "Ordonne au drone de revenir à son point de décollage et d'atterrir".to_string(),
                parameters: json!({}),
            },
            McpToolDefinition {
                name: "get_telemetry".to_string(),
                description: "Retourne position GPS, altitude, vitesse, cap, armement et mode de vol".to_string(),
                parameters: json!({}),
            },
            McpToolDefinition {
                name: "get_battery".to_string(),
                description: "Retourne pourcentage batterie, tension et temps restant estimé".to_string(),
                parameters: json!({}),
            },
            McpToolDefinition {
                name: "loiter".to_string(),
                description: "Maintient le drone en vol stationnaire pendant une durée donnée".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "duration_s": { "type": "integer" }
                    }
                }),
            },
            McpToolDefinition {
                name: "set_speed".to_string(),
                description: "Règle la vitesse de croisière en m/s".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "speed_m_s": { "type": "number" }
                    }
                }),
            },
            // Vision
            McpToolDefinition {
                name: "analyze_frame".to_string(),
                description: "Prend une image et en fait l'analyse complète (terrain, détection YOLO)".to_string(),
                parameters: json!({}),
            },
            McpToolDefinition {
                name: "detect_objects".to_string(),
                description: "Détecte les objets YOLO du frame courant avec un seuil de confiance".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "min_confidence": { "type": "number" }
                    }
                }),
            },
            McpToolDefinition {
                name: "get_snapshot".to_string(),
                description: "Capture le frame courant et sauvegarde un snapshot si demandé".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "save": { "type": "boolean" }
                    }
                }),
            },
            McpToolDefinition {
                name: "check_landing_zone".to_string(),
                description: "Vérifie si la zone directement sous le drone est libre d'obstacles pour atterrir".to_string(),
                parameters: json!({}),
            },
            // Weather
            McpToolDefinition {
                name: "get_conditions".to_string(),
                description: "Retourne vent, visibilité, pluie, température et description météo OpenWeatherMap".to_string(),
                parameters: json!({}),
            },
            McpToolDefinition {
                name: "is_safe_to_fly".to_string(),
                description: "Récupère le diagnostic de vol en fonction de la vitesse du vent et de la pluie".to_string(),
                parameters: json!({}),
            },
            // Map
            McpToolDefinition {
                name: "get_unexplored_sector".to_string(),
                description: "Récupère les coordonnées du prochain secteur optimal à explorer".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "current_lat": { "type": "number" },
                        "current_lon": { "type": "number" }
                    },
                    "required": ["current_lat", "current_lon"]
                }),
            },
            McpToolDefinition {
                name: "add_waypoint".to_string(),
                description: "Ajoute un point d'intérêt clé sur la carte".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "lat": { "type": "number" },
                        "lon": { "type": "number" },
                        "label": { "type": "string" },
                        "photo_path": { "type": "string" }
                    },
                    "required": ["lat", "lon", "label"]
                }),
            },
            McpToolDefinition {
                name: "get_coverage_percent".to_string(),
                description: "Retourne le pourcentage de couverture de la zone explorée".to_string(),
                parameters: json!({}),
            },
            McpToolDefinition {
                name: "export_map".to_string(),
                description: "Exporte la carte en GeoJSON ou HTML Leaflet/OpenStreetMap".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "format": { "type": "string", "enum": ["geojson", "html"] }
                    }
                }),
            },
            // Mission
            McpToolDefinition {
                name: "get_mission_status".to_string(),
                description: "Retourne le statut de mission: waypoints visités, découvertes, couverture et temps écoulé".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "mission_id": { "type": "string" }
                    },
                    "required": ["mission_id"]
                }),
            },
            McpToolDefinition {
                name: "log_finding".to_string(),
                description: "Enregistre une découverte avec photo et géolocalisation en base de données".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "mission_id": { "type": "string" },
                        "lat": { "type": "number" },
                        "lon": { "type": "number" },
                        "description": { "type": "string" },
                        "photo_path": { "type": "string" }
                    },
                    "required": ["mission_id", "lat", "lon", "description"]
                }),
            },
            McpToolDefinition {
                name: "complete_mission".to_string(),
                description: "Clôture la mission et stocke le résumé final".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "mission_id": { "type": "string" },
                        "summary": { "type": "string" }
                    },
                    "required": ["mission_id", "summary"]
                }),
            },
        ]
    }
}
