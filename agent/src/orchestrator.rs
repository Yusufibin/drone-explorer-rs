use crate::llm::LlmClient;
use crate::mcp_clients::McpClients;
use crate::memory::MissionMemory;
use crate::prompter::Prompter;
use async_channel::{Receiver, Sender};
use common::config::AppConfig;
use common::error::DroneError;
use common::mcp::McpToolCall;
use common::types::MissionEvent;
use serde_json::json;
use std::time::Duration;

/// Chef d'orchestre de la mission, implémentant la boucle décisionnelle ReAct.
pub struct Orchestrator {
    pub config: AppConfig,
    pub llm: LlmClient,
    pub mcp_clients: McpClients,
    pub memory: MissionMemory,
    event_rx: Receiver<MissionEvent>,
    event_tx: Sender<MissionEvent>,
    pub mission_id: Option<String>,
    pub mission_active: bool,
}

impl Orchestrator {
    /// Initialise l'orchestrateur.
    pub async fn new(config: AppConfig) -> Result<Self, DroneError> {
        let (event_tx, event_rx) = async_channel::bounded(100);
        let mcp_clients = McpClients::new(&config).await?;
        let llm = LlmClient::new(&config.llm, config.simulation.enabled);
        let memory = MissionMemory::new();

        Ok(Self {
            config,
            llm,
            mcp_clients,
            memory,
            event_rx,
            event_tx,
            mission_id: None,
            mission_active: false,
        })
    }

    /// Démarre et exécute la mission autonome complète.
    pub async fn run(&mut self, mission_description: &str) -> Result<(), DroneError> {
        tracing::info!(
            "Démarrage de la mission autonome : \"{}\"",
            mission_description
        );
        // 1. Enregistrement de la session de mission dans mission-mcp
        let init_res = self
            .mcp_clients
            .call_tool(&McpToolCall {
                name: "create_mission".to_string(),
                arguments: json!({
                    "name": "Mission Exploration",
                    "description": mission_description
                }),
            })
            .await?;

        if init_res.success {
            let mid = init_res.data.as_str().filter(|id| !id.is_empty())
                .ok_or_else(|| DroneError::Mission("ID de mission absent".into()))?.to_string();
            self.mission_id = Some(mid.clone());
            tracing::info!("Session de mission créée dans SQLite avec ID: {}", mid);
        } else {
            return Err(DroneError::Mission(format!("Création de mission refusée: {:?}", init_res.error)));
        }
        self.mission_active = true;

        // 2. Lancer la tâche périodique (PeriodicTick toutes les 15 secondes)
        let tx_clone = self.event_tx.clone();
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(Duration::from_secs(15)).await;
                if tx_clone.send(MissionEvent::PeriodicTick).await.is_err() {
                    break;
                }
            }
        });

        // 3. Envoyer l'événement initial de démarrage
        self.event_tx
            .send(MissionEvent::MissionStart(mission_description.to_string()))
            .await
            .map_err(|e| DroneError::Mission(e.to_string()))?;

        // 4. Boucle principale ReAct
        while self.mission_active {
            tokio::select! {
                _ = tokio::signal::ctrl_c() => {
                    self.emergency_rtl("Arrêt demandé").await?;
                }
                event_opt = self.event_rx.recv() => {
                    if let Ok(event) = event_opt {
                        if let Err(error) = self.process_event(event, mission_description).await {
                            self.emergency_rtl("Erreur de mission").await?;
                            return Err(error);
                        }
                    }
                }
            }
        }

        tracing::info!("Boucle de l'orchestrateur arrêtée.");
        Ok(())
    }

    /// Traite un événement de mission unique dans la boucle ReAct.
    async fn process_event(
        &mut self,
        event: MissionEvent,
        mission_description: &str,
    ) -> Result<(), DroneError> {
        tracing::info!("=== Traitement événement : {:?} ===", event);
        self.memory.add_event(event.clone());

        // Récupérer la télémétrie et le contexte à jour
        let context = self.collect_context().await?;

        // Failsafe Batterie Critique
        let battery = context
            .get("battery_percent")
            .and_then(|v| v.as_f64())
            .unwrap_or(100.0);
        if battery < self.config.drone.failsafe_battery_percent {
            tracing::warn!(
                "FAILSAFE: Batterie sous le seuil critique ({}% < {}%). Ordre RTL immédiat.",
                battery,
                self.config.drone.failsafe_battery_percent
            );
            self.emergency_rtl("Batterie critique").await?;
            return Ok(());
        }

        // Failsafe Météo Dangereuse
        let is_weather_safe = context
            .get("weather_safe")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);
        if !is_weather_safe {
            let reason = context
                .get("weather_reason")
                .and_then(|v| v.as_str())
                .unwrap_or("Conditions inconnues");
            tracing::warn!(
                "FAILSAFE MÉTÉO: Danger de vol détecté ({})! Ordre RTL immédiat.",
                reason
            );
            self.emergency_rtl("Météo dangereuse").await?;
            return Ok(());
        }

        // Choix du modèle LLM adapté
        let model = self.llm.select_model(&event);

        // Assemblage des prompts
        let system_prompt = Prompter::system_prompt(mission_description);
        let user_prompt = Prompter::build_prompt(
            mission_description,
            &event,
            &context,
            &self.mcp_clients.available_tools(),
        );

        // Appel décisionnel au LLM
        let response = self
            .llm
            .complete(
                &system_prompt,
                &user_prompt,
                &self.mcp_clients.available_tools(),
                &model,
            )
            .await?;

        if let Some(ref thought) = response.content {
            tracing::info!("[LLM Pensée] : {}", thought);
        }

        if let Some(mut tool_call) = response.tool_call {
            tracing::info!("[LLM Decision d'action] -> {}", tool_call.name);

            // Injection dynamique du mission_id si requis par l'outil
            if (tool_call.name == "log_finding"
                || tool_call.name == "complete_mission"
                || tool_call.name == "get_mission_status")
                && self.mission_id.is_some()
            {
                if let Some(obj) = tool_call.arguments.as_object_mut() {
                    obj.insert(
                        "mission_id".to_string(),
                        json!(self.mission_id.clone().unwrap()),
                    );
                }
            }

            // Exécution de l'outil via MCP
            let result = self.mcp_clients.call_tool(&tool_call).await?;

            if result.success {
                tracing::info!(
                    "Succès de l'action '{}' : {:?}",
                    tool_call.name,
                    result.data
                );

                // Réaction réflexe sur certains résultats d'outils
                if tool_call.name == "get_unexplored_sector" {
                    // Si on a récupéré le secteur, on ordonne un goto
                    if let Some(sector) = result.data.get("center") {
                        let lat = sector.get("lat").and_then(|v| v.as_f64()).unwrap_or(0.0);
                        let lon = sector.get("lon").and_then(|v| v.as_f64()).unwrap_or(0.0);
                        let alt = sector.get("alt").and_then(|v| v.as_f64()).unwrap_or(20.0);

                        tracing::info!(
                            "Réflexe de navigation: envoi du drone vers le secteur ({}, {})",
                            lat,
                            lon
                        );
                        let goto_call = McpToolCall {
                            name: "goto".to_string(),
                            arguments: json!({ "lat": lat, "lon": lon, "alt": alt }),
                        };
                        let goto_res = self.mcp_clients.call_tool(&goto_call).await?;
                        if !goto_res.success {
                            return Err(DroneError::Mission(format!("Navigation refusée: {:?}", goto_res.error)));
                        }
                        self.memory.add_decision(
                            "Navigation automatique vers le secteur exploré",
                            goto_call,
                            goto_res,
                        );
                    }
                } else if tool_call.name == "goto" && self.config.simulation.enabled {
                    // Événement synthétique réservé au simulateur.
                    let tx_clone = self.event_tx.clone();
                    let lat = tool_call
                        .arguments
                        .get("lat")
                        .and_then(|v| v.as_f64())
                        .unwrap_or(0.0);
                    let lon = tool_call
                        .arguments
                        .get("lon")
                        .and_then(|v| v.as_f64())
                        .unwrap_or(0.0);
                    let alt = tool_call
                        .arguments
                        .get("alt")
                        .and_then(|v| v.as_f64())
                        .unwrap_or(20.0);

                    // Marquer le waypoint GPS en base
                    if self.mission_id.is_some() {
                        let _ = self
                            .mcp_clients
                            .call_tool(&McpToolCall {
                                name: "add_waypoint".to_string(),
                                arguments: json!({
                                    "lat": lat,
                                    "lon": lon,
                                    "label": "Waypoint Exploré",
                                    "photo_path": "snapshots/snap.jpg"
                                }),
                            })
                            .await;
                    }

                    tokio::spawn(async move {
                        tokio::time::sleep(Duration::from_secs(3)).await;
                        let _ = tx_clone
                            .send(MissionEvent::WaypointReached(common::types::GpsPosition {
                                lat,
                                lon,
                                alt,
                            }))
                            .await;
                    });
                } else if tool_call.name == "analyze_frame" {
                    // Analyse visuelle : si objets trouvés, envoyer l'événement ObjectDetected
                    if let Some(objects) = result
                        .data
                        .get("detected_objects")
                        .and_then(|v| v.as_array())
                    {
                        for obj_val in objects {
                            if let Ok(obj) = serde_json::from_value::<common::types::DetectedObject>(
                                obj_val.clone(),
                            ) {
                                tracing::info!("Objet d'intérêt découvert: {}", obj.class_name);
                                let _ = self.event_tx.send(MissionEvent::ObjectDetected(obj)).await;
                            }
                        }
                    }
                } else if tool_call.name == "complete_mission" {
                    tracing::warn!("Mission terminée avec succès par décision LLM.");
                    self.emergency_rtl("Mission terminée").await?;
                }
            } else {
                tracing::error!(
                    "Échec de l'action '{}' : {:?}",
                    tool_call.name,
                    result.error
                );
                if matches!(tool_call.name.as_str(), "takeoff" | "goto" | "land" | "rtl" | "loiter" | "set_speed") {
                    return Err(DroneError::Mission(format!("Commande de vol refusée: {:?}", result.error)));
                }
            }

            self.memory
                .add_decision(response.content.as_deref().unwrap_or(""), tool_call, result);
        } else {
            tracing::warn!("Le LLM n'a proposé aucune action.");
        }

        // Fin de mission si la couverture de la carte est totale (> 95%)
        let coverage = context
            .get("coverage_percent")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);
        if coverage >= 95.0 {
            tracing::warn!(
                "Cartographie achevée (couverture de {:.1}% >= 95%). Clôture de la mission.",
                coverage
            );
            if let Some(ref mid) = self.mission_id {
                let _ = self.mcp_clients.call_tool(&McpToolCall {
                    name: "complete_mission".to_string(),
                    arguments: json!({
                        "mission_id": mid,
                        "summary": "Couverture maximale de la grille atteinte. Cartographie complète."
                    }),
                }).await?;
            }
            self.emergency_rtl("Couverture atteinte").await?;
        }

        Ok(())
    }

    /// Récupère la télémétrie, la batterie, la météo et la couverture courante depuis les microservices MCP.
    async fn collect_context(&self) -> Result<serde_json::Value, DroneError> {
        let telemetry = self.mcp_clients.get_telemetry().await?;
        let battery = self.mcp_clients.get_battery().await?;
        if !battery.percent.is_finite() || !(0.0..=100.0).contains(&battery.percent) {
            return Err(DroneError::Mission("Batterie invalide".into()));
        }

        let coverage = self.mcp_clients.get_coverage().await.unwrap_or(0.0);

        // Appel météo
        let weather = self
            .mcp_clients
            .call_tool(&McpToolCall {
                name: "is_safe_to_fly".to_string(),
                arguments: serde_json::Value::Null,
            })
            .await
            ?;
        if !weather.success {
            return Err(DroneError::Weather(weather.error.unwrap_or_else(|| "Météo indisponible".into())));
        }
        let weather_safe = weather.data.get("safe").and_then(|v| v.as_bool())
            .ok_or_else(|| DroneError::Weather("Verdict météo absent".into()))?;
        let weather_reason = weather.data.get("reason").and_then(|v| v.as_str()).unwrap_or("Conditions inconnues");

        Ok(json!({
            "lat": telemetry.position.lat,
            "lon": telemetry.position.lon,
            "alt": telemetry.position.alt,
            "battery_percent": battery.percent,
            "mode": telemetry.mode.to_string(),
            "coverage_percent": coverage,
            "weather_safe": weather_safe,
            "weather_reason": weather_reason
        }))
    }

    async fn emergency_rtl(&mut self, reason: &str) -> Result<(), DroneError> {
        tracing::warn!("Arrêt de mission: {}. Demande RTL.", reason);
        self.mission_active = false;
        let result = self.mcp_clients.call_tool(&McpToolCall {
            name: "rtl".into(), arguments: json!({}),
        }).await?;
        if !result.success {
            return Err(DroneError::Mission(format!("RTL refusé: {:?}", result.error)));
        }
        Ok(())
    }
}
