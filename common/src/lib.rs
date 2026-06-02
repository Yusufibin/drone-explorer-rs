//! # Common
//!
//! Crate partagé contenant les types, la configuration, les erreurs et les
//! définitions du protocole MCP utilisés par l'ensemble du système
//! drone-explorer.
//!
//! Ce crate est importé par tous les autres crates du workspace.

pub mod config;
pub mod error;
pub mod mcp;
pub mod types;

// Ré-exports pratiques pour un accès direct depuis `common::*`
pub use config::*;
pub use error::*;
pub use mcp::*;
pub use types::*;
