use crate::mavlink_bridge::MavlinkBridge;
use common::mcp::{tool_result_err, tool_result_ok};
use jsonrpsee::proc_macros::rpc;
use jsonrpsee::server::ServerBuilder;
use jsonrpsee::types::ErrorObjectOwned;
use serde_json::Value;

#[rpc(server)]
pub trait FlightMcpRpc {
    #[method(name = "takeoff")]
    async fn takeoff(&self, altitude: f64) -> Result<Value, ErrorObjectOwned>;

    #[method(name = "goto")]
    async fn goto(&self, lat: f64, lon: f64, alt: f64) -> Result<Value, ErrorObjectOwned>;

    #[method(name = "land")]
    async fn land(&self) -> Result<Value, ErrorObjectOwned>;

    #[method(name = "rtl")]
    async fn rtl(&self) -> Result<Value, ErrorObjectOwned>;

    #[method(name = "get_telemetry")]
    async fn get_telemetry(&self) -> Result<Value, ErrorObjectOwned>;

    #[method(name = "get_battery")]
    async fn get_battery(&self) -> Result<Value, ErrorObjectOwned>;

    #[method(name = "loiter")]
    async fn loiter(&self, duration_s: u64) -> Result<Value, ErrorObjectOwned>;

    #[method(name = "set_speed")]
    async fn set_speed(&self, speed_m_s: f64) -> Result<Value, ErrorObjectOwned>;
}

pub struct FlightRpcServer {
    bridge: MavlinkBridge,
}

impl FlightRpcServer {
    pub fn new(bridge: MavlinkBridge) -> Self {
        Self { bridge }
    }
}

#[async_trait::async_trait]
impl FlightMcpRpcServer for FlightRpcServer {
    async fn takeoff(&self, altitude: f64) -> Result<Value, ErrorObjectOwned> {
        match self.bridge.takeoff(altitude).await {
            Ok(success) => Ok(serde_json::to_value(tool_result_ok(success)).unwrap()),
            Err(e) => Ok(serde_json::to_value(tool_result_err(e.to_string())).unwrap()),
        }
    }

    async fn goto(&self, lat: f64, lon: f64, alt: f64) -> Result<Value, ErrorObjectOwned> {
        // Enforce geofence and safety checks in the GOTO RPC call
        match self.bridge.goto(lat, lon, alt).await {
            Ok(success) => Ok(serde_json::to_value(tool_result_ok(success)).unwrap()),
            Err(e) => Ok(serde_json::to_value(tool_result_err(e.to_string())).unwrap()),
        }
    }

    async fn land(&self) -> Result<Value, ErrorObjectOwned> {
        match self.bridge.land().await {
            Ok(success) => Ok(serde_json::to_value(tool_result_ok(success)).unwrap()),
            Err(e) => Ok(serde_json::to_value(tool_result_err(e.to_string())).unwrap()),
        }
    }

    async fn rtl(&self) -> Result<Value, ErrorObjectOwned> {
        match self.bridge.rtl().await {
            Ok(success) => Ok(serde_json::to_value(tool_result_ok(success)).unwrap()),
            Err(e) => Ok(serde_json::to_value(tool_result_err(e.to_string())).unwrap()),
        }
    }

    async fn get_telemetry(&self) -> Result<Value, ErrorObjectOwned> {
        match self.bridge.get_telemetry().await {
            Ok(telemetry) => Ok(serde_json::to_value(tool_result_ok(telemetry)).unwrap()),
            Err(e) => Ok(serde_json::to_value(tool_result_err(e.to_string())).unwrap()),
        }
    }

    async fn get_battery(&self) -> Result<Value, ErrorObjectOwned> {
        match self.bridge.get_battery().await {
            Ok(battery) => Ok(serde_json::to_value(tool_result_ok(battery)).unwrap()),
            Err(e) => Ok(serde_json::to_value(tool_result_err(e.to_string())).unwrap()),
        }
    }

    async fn loiter(&self, duration_s: u64) -> Result<Value, ErrorObjectOwned> {
        match self.bridge.loiter(duration_s).await {
            Ok(success) => Ok(serde_json::to_value(tool_result_ok(success)).unwrap()),
            Err(e) => Ok(serde_json::to_value(tool_result_err(e.to_string())).unwrap()),
        }
    }

    async fn set_speed(&self, speed_m_s: f64) -> Result<Value, ErrorObjectOwned> {
        match self.bridge.set_speed(speed_m_s).await {
            Ok(success) => Ok(serde_json::to_value(tool_result_ok(success)).unwrap()),
            Err(e) => Ok(serde_json::to_value(tool_result_err(e.to_string())).unwrap()),
        }
    }
}

/// Démarre le serveur JSON-RPC pour le contrôle de vol.
pub async fn start_server(bridge: MavlinkBridge, port: u16) -> anyhow::Result<()> {
    let addr = format!("127.0.0.1:{}", port);
    let server = ServerBuilder::default().build(&addr).await?;

    let rpc_impl = FlightRpcServer::new(bridge);
    let handle = server.start(rpc_impl.into_rpc());

    tracing::info!("Serveur flight-mcp lancé sur http://{}", addr);

    // Garder le serveur en cours d'exécution
    handle.stopped().await;
    Ok(())
}
