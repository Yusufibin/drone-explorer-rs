use crate::builder::MapBuilder;
use crate::explorer::FrontierExplorer;
use common::mcp::{tool_result_err, tool_result_ok};
use common::types::GpsPosition;
use jsonrpsee::proc_macros::rpc;
use jsonrpsee::server::ServerBuilder;
use jsonrpsee::types::ErrorObjectOwned;
use serde_json::Value;
use std::sync::Arc;
use tokio::sync::Mutex;

#[rpc(server)]
pub trait MapMcpRpc {
    #[method(name = "add_waypoint")]
    async fn add_waypoint(
        &self,
        lat: f64,
        lon: f64,
        label: String,
        photo_path: Option<String>,
    ) -> Result<Value, ErrorObjectOwned>;

    #[method(name = "get_unexplored_sector")]
    async fn get_unexplored_sector(
        &self,
        current_lat: f64,
        current_lon: f64,
    ) -> Result<Value, ErrorObjectOwned>;

    #[method(name = "get_coverage_percent")]
    async fn get_coverage_percent(&self) -> Result<Value, ErrorObjectOwned>;

    #[method(name = "export_map")]
    async fn export_map(&self, format: String) -> Result<Value, ErrorObjectOwned>;
}

pub struct MapRpcServer {
    builder: Arc<Mutex<MapBuilder>>,
    explorer: FrontierExplorer,
}

impl MapRpcServer {
    pub fn new(builder: MapBuilder) -> Self {
        Self {
            builder: Arc::new(Mutex::new(builder)),
            explorer: FrontierExplorer,
        }
    }
}

#[async_trait::async_trait]
impl MapMcpRpcServer for MapRpcServer {
    async fn add_waypoint(
        &self,
        lat: f64,
        lon: f64,
        label: String,
        photo_path: Option<String>,
    ) -> Result<Value, ErrorObjectOwned> {
        let mut builder = self.builder.lock().await;
        let success = builder.add_waypoint(lat, lon, &label, photo_path.as_deref());
        Ok(serde_json::to_value(tool_result_ok(success)).unwrap())
    }

    async fn get_unexplored_sector(
        &self,
        current_lat: f64,
        current_lon: f64,
    ) -> Result<Value, ErrorObjectOwned> {
        let builder = self.builder.lock().await;
        let current_pos = GpsPosition {
            lat: current_lat,
            lon: current_lon,
            alt: 20.0,
        };

        match self
            .explorer
            .get_unexplored_sector(&builder.sectors, &current_pos)
        {
            Some(sector) => Ok(serde_json::to_value(tool_result_ok(sector)).unwrap()),
            None => Ok(serde_json::to_value(tool_result_err(
                "Aucun secteur inexploré restant dans la géofence.",
            ))
            .unwrap()),
        }
    }

    async fn get_coverage_percent(&self) -> Result<Value, ErrorObjectOwned> {
        let builder = self.builder.lock().await;
        let percent = builder.get_coverage_percent();
        Ok(serde_json::to_value(tool_result_ok(percent)).unwrap())
    }

    async fn export_map(&self, format: String) -> Result<Value, ErrorObjectOwned> {
        let builder = self.builder.lock().await;

        match format.to_lowercase().as_str() {
            "geojson" | "json" => {
                let geojson = builder.export_geojson();
                Ok(serde_json::to_value(tool_result_ok(geojson)).unwrap())
            }
            "html" => Ok(serde_json::to_value(tool_result_ok(builder.export_html())).unwrap()),
            _ => Ok(serde_json::to_value(tool_result_err(format!(
                "Format d'export inconnu: {}. Utiliser 'geojson' ou 'html'",
                format
            )))
            .unwrap()),
        }
    }
}

/// Démarre le serveur map-mcp.
pub async fn start_server(builder: MapBuilder, port: u16) -> anyhow::Result<()> {
    let addr = format!("0.0.0.0:{}", port);
    let server = ServerBuilder::default().build(&addr).await?;

    let rpc_impl = MapRpcServer::new(builder);
    let handle = server.start(rpc_impl.into_rpc());

    tracing::info!("Serveur map-mcp lancé sur http://{}", addr);

    handle.stopped().await;
    Ok(())
}
