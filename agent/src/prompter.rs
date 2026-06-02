use common::mcp::McpToolDefinition;
use common::types::MissionEvent;

/// Gère l'assemblage et le formatage des prompts pour le LLM.
pub struct Prompter;

impl Prompter {
    /// Génère le prompt système contenant les règles absolues de la mission.
    pub fn system_prompt(mission: &str) -> String {
        format!(
            "Tu es l'intelligence stratégique et le système de navigation d'un drone explorateur autonome DIY.\n\
             Ton but est de guider le drone pour mener à bien sa mission en toute sécurité, en analysant la télémétrie et les événements en temps réel.\n\n\
             DESCRIPTION DE LA MISSION :\n\
             \"{}\"\n\n\
             RÈGLES DE SÉCURITÉ ABSOLUES :\n\
             1. Si le niveau de batterie est STRICTEMENT INFÉRIEUR à 25% (battery < 25%), tu DOIS appeler rtl() immédiatement. Aucune exception.\n\
             2. Si la vitesse du vent dépasse 45 km/h ou en cas de forte pluie, tu DOIS ordonner un retour à la base via rtl().\n\
             3. Si l'altitude dépasse 80m (max légal), tu DOIS immédiatement corriger en volant plus bas ou en déclenchant un land()/rtl().\n\
             4. Une décision à la fois : tu DOIS émettre EXACTEMENT UN appel d'outil à la fois sous la forme d'un objet JSON correct.\n\
             5. Lorsque tu as fini d'explorer, ou si la couverture de la zone dépasse 95%, termine proprement la mission en appelant complete_mission().\n\n\
             Format de ta réponse :\n\
             Tu dois TOUJOURS répondre en affichant ta pensée (Raisonnement) puis l'outil à appeler dans un bloc JSON valide.\n\
             Exemple :\n\
             Pensée : Je dois décoller pour commencer à explorer la zone.\n\
             Appel d'outil : {{\"name\": \"takeoff\", \"arguments\": {{\"altitude\": 15.0}}}}\n",
            mission
        )
    }

    /// Construit le prompt utilisateur complet à partir d'un événement survenu et du contexte courant.
    pub fn build_prompt(
        mission: &str,
        event: &MissionEvent,
        context: &serde_json::Value,
        tools: &[McpToolDefinition],
    ) -> String {
        let event_desc = Self::format_event(event);
        let tools_desc = Self::format_tools(tools);

        // Extraction propre du contexte JSON
        let lat = context
            .get("lat")
            .and_then(|v| v.as_f64())
            .unwrap_or(48.8566);
        let lon = context
            .get("lon")
            .and_then(|v| v.as_f64())
            .unwrap_or(2.3522);
        let alt = context.get("alt").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let battery = context
            .get("battery_percent")
            .and_then(|v| v.as_f64())
            .unwrap_or(100.0);
        let mode = context
            .get("mode")
            .and_then(|v| v.as_str())
            .unwrap_or("UNKNOWN");
        let coverage = context
            .get("coverage_percent")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);
        let is_safe = context
            .get("weather_safe")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);
        let weather_reason = context
            .get("weather_reason")
            .and_then(|v| v.as_str())
            .unwrap_or("Inconnu");

        format!(
            "=== ÉVÉNEMENT SURVENU ===\n\
             {}\n\n\
             === ÉTAT DU DRONE & CONTEXTE ===\n\
             - Position GPS : Latitude={:.6}, Longitude={:.6}, Altitude={:.1}m\n\
             - Batterie : {:.1}%\n\
             - Mode de vol courant : {}\n\
             - Couverture de la zone d'exploration : {:.1}%\n\
             - Diagnostic météo : Safe={} ({})\n\n\
             === OUTILS MCP DISPONIBLES ===\n\
             {}\n\n\
             Mission à accomplir : \"{}\"\n\n\
             Analyse la situation et prends la prochaine décision tactique ou de navigation de manière rationnelle. Renvoie EXACTEMENT un appel d'outil JSON après ta pensée.",
            event_desc,
            lat,
            lon,
            alt,
            battery,
            mode,
            coverage,
            is_safe,
            weather_reason,
            tools_desc,
            mission
        )
    }

    /// Formate l'événement de mission de manière intelligible.
    pub fn format_event(event: &MissionEvent) -> String {
        match event {
            MissionEvent::MissionStart(name) => format!("DÉMARRAGE DE LA MISSION : {}", name),
            MissionEvent::MissionEnd(reason) => format!("FIN DE LA MISSION : {}", reason),
            MissionEvent::WaypointReached(pos) => {
                format!("Le drone est bien arrivé au waypoint GPS : {}", pos)
            }
            MissionEvent::ObjectDetected(obj) => format!(
                "IMAGE DE LA CAMÉRA ANALYSÉE : Objet détecté \"{}\" avec {:.0}% de confiance aux coordonnées {:?}.",
                obj.class_name,
                obj.confidence * 100.0,
                obj.position
            ),
            MissionEvent::BatteryLow(batt) => format!(
                "ALERTE BATTERIE : Tension={:.2}V, Pourcentage={:.1}%. Temps estimé restant : {:?} min.",
                batt.voltage, batt.percent, batt.remaining_minutes
            ),
            MissionEvent::WeatherDegraded(weather) => format!(
                "CONDITIONS MÉTÉO DÉGRADÉES : Vent={:.1} km/h {}, Temp={:.1}°C, Pluie={}",
                weather.wind_speed_kmh,
                weather.wind_direction,
                weather.temperature_celsius,
                weather.rain
            ),
            MissionEvent::ZoneCovered(percent) => format!(
                "PROGRÈS DE LA CARTE : Grille d'exploration couverte à {:.1}%.",
                percent
            ),
            MissionEvent::PeriodicTick => {
                "MISE À JOUR PÉRIODIQUE (Rien de particulier à signaler, analyse si tout est OK)"
                    .to_string()
            }
        }
    }

    /// Formate la liste des outils MCP et leurs schémas de paramètres pour le prompt.
    pub fn format_tools(tools: &[McpToolDefinition]) -> String {
        let mut list = String::new();
        for t in tools {
            list.push_str(&format!(
                "- Outil : {}\n  Description : {}\n  Paramètres : {}\n\n",
                t.name, t.description, t.parameters
            ));
        }
        list
    }
}
