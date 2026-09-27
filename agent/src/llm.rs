use common::config::LlmConfig;
use common::error::DroneError;
use common::mcp::{McpToolCall, McpToolDefinition};
use common::types::MissionEvent;
use serde::{Deserialize, Serialize};
use serde_json::json;

/// Réponse structurée du LLM.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmResponse {
    pub content: Option<String>,
    pub tool_call: Option<McpToolCall>,
}

/// Client d'interface avec l'API unifiée de LLM (OpenRouter/OpenAI).
#[derive(Debug, Clone)]
pub struct LlmClient {
    pub config: LlmConfig,
    client: reqwest::Client,
    simulation: bool,
}

impl LlmClient {
    /// Initialise le client LLM.
    pub fn new(config: &LlmConfig, simulation: bool) -> Self {
        Self {
            config: config.clone(),
            client: reqwest::Client::builder().timeout(std::time::Duration::from_secs(20))
                .build().expect("configuration HTTP valide"),
            simulation,
        }
    }

    /// Choisit le modèle LLM adapté à la tâche en cours selon la stratégie.
    pub fn select_model(&self, event: &MissionEvent) -> String {
        match event {
            MissionEvent::BatteryLow(_) | MissionEvent::WeatherDegraded(_) => {
                self.config.critical_model.clone()
            }
            MissionEvent::ObjectDetected(_) => self.config.vision_model.clone(),
            MissionEvent::MissionStart(_) | MissionEvent::MissionEnd(_) => {
                self.config.planning_model.clone()
            }
            _ => self.config.navigation_model.clone(),
        }
    }

    /// Envoie une requête de complétion; la simulation doit être explicitement activée.
    pub async fn complete(
        &self,
        system_prompt: &str,
        user_prompt: &str,
        _tools: &[McpToolDefinition],
        model: &str,
    ) -> Result<LlmResponse, DroneError> {
        if self.simulation {
            return Ok(self.simulate_llm_response(user_prompt));
        }
        if self.config.api_key.is_empty() || self.config.api_key == "${OPENROUTER_API_KEY}" {
            return Err(DroneError::Llm("OPENROUTER_API_KEY manquante".into()));
        }

        let url = format!("{}/chat/completions", self.config.base_url);

        let body = json!({
            "model": model,
            "messages": [
                { "role": "system", "content": system_prompt },
                { "role": "user", "content": user_prompt }
            ],
            "temperature": self.config.temperature,
            "max_tokens": self.config.max_tokens
        });

        tracing::info!("Envoi de la requête au LLM (modèle: {})...", model);

        match self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.config.api_key))
            .header("HTTP-Referer", "https://github.com/Yusufibin/drone-explorer-rs")
            .json(&body)
            .send()
            .await
        {
            Ok(resp) => {
                if resp.status().is_success() {
                    #[derive(Deserialize)]
                    struct ChatChoice {
                        message: ChatMessage,
                    }
                    #[derive(Deserialize)]
                    struct ChatMessage {
                        content: Option<String>,
                    }
                    #[derive(Deserialize)]
                    struct ChatResponse {
                        choices: Vec<ChatChoice>,
                    }

                    match resp.json::<ChatResponse>().await {
                        Ok(data) => {
                            if let Some(choice) = data.choices.first() {
                                if let Some(ref text) = choice.message.content {
                                    return Ok(Self::parse_llm_output(text));
                                }
                            }
                            Err(DroneError::Llm("Réponse LLM vide ou incorrecte".into()))
                        }
                        Err(e) => {
                            Err(DroneError::Llm(format!("Réponse LLM invalide: {}", e)))
                        }
                    }
                } else {
                    Err(DroneError::Llm(format!("HTTP LLM: {}", resp.status())))
                }
            }
            Err(e) => {
                Err(DroneError::Llm(format!("Erreur réseau LLM: {}", e)))
            }
        }
    }

    /// Décode la réponse brute ReAct du LLM pour en extraire la pensée et l'appel d'outil.
    fn parse_llm_output(text: &str) -> LlmResponse {
        // Recherche d'un bloc JSON de la forme {"name": ...} ou simple détection
        if let Some(start) = text.find('{') {
            if let Some(end) = text.rfind('}') {
                let json_str = &text[start..=end];
                if let Ok(tool_call) = serde_json::from_str::<McpToolCall>(json_str) {
                    return LlmResponse {
                        content: Some(text[..start].to_string()),
                        tool_call: Some(tool_call),
                    };
                }
            }
        }

        LlmResponse {
            content: Some(text.to_string()),
            tool_call: None,
        }
    }

    /// Simulateur intelligent de boucle ReAct pour le test offline.
    fn simulate_llm_response(&self, user_prompt: &str) -> LlmResponse {
        tracing::info!("Simulation de la décision ReAct du LLM...");

        let (content, tool_call) = if user_prompt.contains("DÉMARRAGE DE LA MISSION") {
            ("Pensée : La mission vient de démarrer. Je dois procéder au décollage de sécurité à 15 mètres pour avoir un bon point de vue.".to_string(), Some(McpToolCall {
                name: "takeoff".to_string(),
                arguments: json!({ "altitude": 15.0 }),
            }))
        } else if user_prompt.contains("ALERTE BATTERIE") && user_prompt.contains("Batterie : 24") {
            ("Pensée : ALERTE ! La batterie est inférieure à 25%. Règle absolue de sécurité : retour à la base immédiat.".to_string(), Some(McpToolCall {
                name: "rtl".to_string(),
                arguments: json!({}),
            }))
        } else if user_prompt.contains("Le drone est bien arrivé")
            || user_prompt.contains("MISE À JOUR PÉRIODIQUE")
        {
            // Le drone est stable, demandons un secteur inexploré
            ("Pensée : Le drone est stable. Demandons à la carte le prochain secteur d'exploration optimal.".to_string(), Some(McpToolCall {
                name: "get_unexplored_sector".to_string(),
                arguments: json!({ "current_lat": 48.8566, "current_lon": 2.3522 }),
            }))
        } else if user_prompt.contains("Secteur d'exploration") {
            // Un secteur a été proposé, allons-y !
            // Extraction grossière de coordonnées simulées
            ("Pensée : J'ai reçu les coordonnées du secteur d'exploration. En route vers ce point !".to_string(), Some(McpToolCall {
                name: "goto".to_string(),
                arguments: json!({ "lat": 48.8567, "lon": 2.3523, "alt": 20.0 }),
            }))
        } else if user_prompt.contains("IMAGE DE LA CAMÉRA ANALYSÉE") {
            // Objet trouvé ! Loguons-le !
            ("Pensée : Un objet d'intérêt a été détecté dans l'analyse de l'image de la caméra. Enregistrons la découverte.".to_string(), Some(McpToolCall {
                name: "log_finding".to_string(),
                arguments: json!({
                    "mission_id": "simulated-session-id",
                    "lat": 48.8567,
                    "lon": 2.3523,
                    "description": "Bâtiment / infrastructure détectée."
                }),
            }))
        } else {
            // Par défaut, analysons le flux visuel courant pour voir ce qui s'y passe
            ("Pensée : Faisons une analyse périodique du flux vidéo de la caméra pour cartographier le terrain.".to_string(), Some(McpToolCall {
                name: "analyze_frame".to_string(),
                arguments: json!({}),
            }))
        };

        LlmResponse {
            content: Some(content),
            tool_call,
        }
    }
}
