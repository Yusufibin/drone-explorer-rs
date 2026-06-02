//! Protocol Model Context Protocol (MCP) et abstractions JSON-RPC.

use serde::{Deserialize, Serialize};

/// Définition d'un outil exposé par un serveur MCP.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpToolDefinition {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,
}

/// Appel d'un outil MCP.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpToolCall {
    pub name: String,
    pub arguments: serde_json::Value,
}

/// Résultat d'un appel d'outil MCP.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpToolResult {
    pub success: bool,
    pub data: serde_json::Value,
    pub error: Option<String>,
}

/// Informations d'un serveur MCP et la liste de ses outils.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServerInfo {
    pub name: String,
    pub version: String,
    pub tools: Vec<McpToolDefinition>,
}

/// Crée un résultat d'outil réussi.
pub fn tool_result_ok<T: Serialize>(data: T) -> McpToolResult {
    McpToolResult {
        success: true,
        data: serde_json::to_value(data).unwrap_or(serde_json::Value::Null),
        error: None,
    }
}

/// Crée un résultat d'outil en erreur.
pub fn tool_result_err<S: Into<String>>(msg: S) -> McpToolResult {
    McpToolResult {
        success: false,
        data: serde_json::Value::Null,
        error: Some(msg.into()),
    }
}
