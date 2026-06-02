use chrono::{DateTime, Utc};
use common::mcp::{McpToolCall, McpToolResult};
use common::types::MissionEvent;

/// Historique en mémoire de la mission courante pour alimenter le contexte du LLM.
#[derive(Debug, Clone, Default)]
pub struct MissionMemory {
    pub events: Vec<(DateTime<Utc>, MissionEvent)>,
    pub decisions: Vec<(DateTime<Utc>, String, McpToolCall, McpToolResult)>,
}

impl MissionMemory {
    /// Initialise une nouvelle mémoire vide.
    pub fn new() -> Self {
        Self::default()
    }

    /// Ajoute un événement survenu.
    pub fn add_event(&mut self, event: MissionEvent) {
        self.events.push((Utc::now(), event));
    }

    /// Enregistre une décision prise par le LLM.
    pub fn add_decision(&mut self, reasoning: &str, call: McpToolCall, result: McpToolResult) {
        self.decisions
            .push((Utc::now(), reasoning.to_string(), call, result));
    }

    /// Récupère les N derniers événements survenus.
    pub fn get_recent_events(&self, n: usize) -> &[(DateTime<Utc>, MissionEvent)] {
        let len = self.events.len();
        let start = len.saturating_sub(n);
        &self.events[start..]
    }

    /// Récupère les N dernières décisions.
    pub fn get_recent_decisions(
        &self,
        n: usize,
    ) -> &[(DateTime<Utc>, String, McpToolCall, McpToolResult)] {
        let len = self.decisions.len();
        let start = len.saturating_sub(n);
        &self.decisions[start..]
    }

    /// Génère un résumé textuel de la mission jusqu'ici.
    pub fn summary(&self) -> String {
        let mut sum = String::new();
        sum.push_str(&format!(
            "Nombre total d'événements traités : {}\n",
            self.events.len()
        ));
        sum.push_str(&format!(
            "Nombre total de décisions prises : {}\n\n",
            self.decisions.len()
        ));

        if !self.decisions.is_empty() {
            sum.push_str("Dernières actions clés :\n");
            for (_, reasoning, call, result) in self.get_recent_decisions(3) {
                sum.push_str(&format!(
                    "- Action : {}\n  Raison : {}\n  Succès : {}\n",
                    call.name, reasoning, result.success
                ));
            }
        }

        sum
    }
}
