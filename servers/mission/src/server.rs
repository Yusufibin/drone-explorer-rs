use crate::db::MissionDatabase;
use chrono::Utc;
use common::mcp::{tool_result_err, tool_result_ok};
use common::types::{Finding, GpsPosition};
use jsonrpsee::proc_macros::rpc;
use jsonrpsee::server::ServerBuilder;
use jsonrpsee::types::ErrorObjectOwned;
use serde_json::Value;
use uuid::Uuid;

#[rpc(server)]
pub trait MissionMcpRpc {
    #[method(name = "create_mission")]
    async fn create_mission(
        &self,
        name: String,
        description: Option<String>,
    ) -> Result<Value, ErrorObjectOwned>;

    #[method(name = "log_finding")]
    async fn log_finding(
        &self,
        mission_id: String,
        lat: f64,
        lon: f64,
        description: String,
        photo_path: Option<String>,
    ) -> Result<Value, ErrorObjectOwned>;

    #[method(name = "get_mission_status")]
    async fn get_mission_status(&self, mission_id: String) -> Result<Value, ErrorObjectOwned>;

    #[method(name = "complete_mission")]
    async fn complete_mission(
        &self,
        mission_id: String,
        summary: String,
    ) -> Result<Value, ErrorObjectOwned>;
}

pub struct MissionRpcServer {
    db: MissionDatabase,
}

impl MissionRpcServer {
    pub fn new(db: MissionDatabase) -> Self {
        Self { db }
    }
}

#[async_trait::async_trait]
impl MissionMcpRpcServer for MissionRpcServer {
    async fn create_mission(
        &self,
        name: String,
        description: Option<String>,
    ) -> Result<Value, ErrorObjectOwned> {
        match self.db.create_mission(&name, description.as_deref()) {
            Ok(id) => Ok(serde_json::to_value(tool_result_ok(id)).unwrap()),
            Err(e) => Ok(serde_json::to_value(tool_result_err(e.to_string())).unwrap()),
        }
    }

    async fn log_finding(
        &self,
        mission_id: String,
        lat: f64,
        lon: f64,
        description: String,
        photo_path: Option<String>,
    ) -> Result<Value, ErrorObjectOwned> {
        let finding = Finding {
            id: Uuid::new_v4(),
            position: GpsPosition {
                lat,
                lon,
                alt: 20.0,
            },
            description,
            photo_path,
            timestamp: Utc::now(),
        };

        match self.db.log_finding(&mission_id, &finding) {
            Ok(_) => Ok(serde_json::to_value(tool_result_ok(true)).unwrap()),
            Err(e) => Ok(serde_json::to_value(tool_result_err(e.to_string())).unwrap()),
        }
    }

    async fn get_mission_status(&self, mission_id: String) -> Result<Value, ErrorObjectOwned> {
        match self.db.get_mission_status(&mission_id) {
            Ok(status) => Ok(serde_json::to_value(tool_result_ok(status)).unwrap()),
            Err(e) => Ok(serde_json::to_value(tool_result_err(e.to_string())).unwrap()),
        }
    }

    async fn complete_mission(
        &self,
        mission_id: String,
        summary: String,
    ) -> Result<Value, ErrorObjectOwned> {
        match self.db.end_mission(&mission_id, &summary) {
            Ok(_) => Ok(serde_json::to_value(tool_result_ok(true)).unwrap()),
            Err(e) => Ok(serde_json::to_value(tool_result_err(e.to_string())).unwrap()),
        }
    }
}

/// Démarre le serveur mission-mcp.
pub async fn start_server(db: MissionDatabase, port: u16) -> anyhow::Result<()> {
    let addr = format!("127.0.0.1:{}", port);
    let server = ServerBuilder::default().build(&addr).await?;

    let rpc_impl = MissionRpcServer::new(db);
    let handle = server.start(rpc_impl.into_rpc());

    tracing::info!("Serveur mission-mcp lancé sur http://{}", addr);

    handle.stopped().await;
    Ok(())
}
