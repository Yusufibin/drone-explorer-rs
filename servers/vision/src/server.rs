use crate::detector::ObjectDetector;
use crate::stream_reader::StreamReader;
use common::mcp::{tool_result_err, tool_result_ok};
use jsonrpsee::proc_macros::rpc;
use jsonrpsee::server::ServerBuilder;
use jsonrpsee::types::ErrorObjectOwned;
use serde_json::{Value, json};

#[rpc(server)]
pub trait VisionMcpRpc {
    #[method(name = "analyze_frame")]
    async fn analyze_frame(&self) -> Result<Value, ErrorObjectOwned>;

    #[method(name = "detect_objects")]
    async fn detect_objects(&self, min_confidence: f64) -> Result<Value, ErrorObjectOwned>;

    #[method(name = "get_snapshot")]
    async fn get_snapshot(&self, save: bool) -> Result<Value, ErrorObjectOwned>;

    #[method(name = "check_landing_zone")]
    async fn check_landing_zone(&self) -> Result<Value, ErrorObjectOwned>;
}

pub struct VisionRpcServer {
    reader: StreamReader,
    detector: ObjectDetector,
}

impl VisionRpcServer {
    pub fn new(reader: StreamReader, detector: ObjectDetector) -> Self {
        Self { reader, detector }
    }
}

#[async_trait::async_trait]
impl VisionMcpRpcServer for VisionRpcServer {
    async fn analyze_frame(&self) -> Result<Value, ErrorObjectOwned> {
        let frame = match self.reader.capture_frame().await {
            Ok(f) => f,
            Err(e) => return Ok(serde_json::to_value(tool_result_err(e.to_string())).unwrap()),
        };

        let detections = match self.detector.detect(&frame).await {
            Ok(d) => d,
            Err(e) => return Ok(serde_json::to_value(tool_result_err(e.to_string())).unwrap()),
        };

        let terrain = match self.detector.classify_terrain(&frame).await {
            Ok(t) => t,
            Err(e) => return Ok(serde_json::to_value(tool_result_err(e.to_string())).unwrap()),
        };

        let result_data = json!({
            "timestamp": frame.timestamp,
            "terrain_type": terrain.to_string(),
            "detected_objects": detections,
            "description": format!("Terrain : {}. Détéctions : {} objet(s) trouvé(s).", terrain, detections.len())
        });

        Ok(serde_json::to_value(tool_result_ok(result_data)).unwrap())
    }

    async fn detect_objects(&self, min_confidence: f64) -> Result<Value, ErrorObjectOwned> {
        let frame = match self.reader.capture_frame().await {
            Ok(f) => f,
            Err(e) => return Ok(serde_json::to_value(tool_result_err(e.to_string())).unwrap()),
        };

        // Override conf threshold
        let detector_override =
            ObjectDetector::new(&self.detector.model_path, min_confidence as f32);

        match detector_override.detect(&frame).await {
            Ok(d) => Ok(serde_json::to_value(tool_result_ok(d)).unwrap()),
            Err(e) => Ok(serde_json::to_value(tool_result_err(e.to_string())).unwrap()),
        }
    }

    async fn get_snapshot(&self, save: bool) -> Result<Value, ErrorObjectOwned> {
        if save {
            match self.reader.save_snapshot("snapshots").await {
                Ok(path) => Ok(serde_json::to_value(tool_result_ok(path)).unwrap()),
                Err(e) => Ok(serde_json::to_value(tool_result_err(e.to_string())).unwrap()),
            }
        } else {
            let frame = match self.reader.capture_frame().await {
                Ok(f) => f,
                Err(e) => return Ok(serde_json::to_value(tool_result_err(e.to_string())).unwrap()),
            };
            let result_data = json!({
                "width": frame.width,
                "height": frame.height,
                "timestamp": frame.timestamp
            });
            Ok(serde_json::to_value(tool_result_ok(result_data)).unwrap())
        }
    }

    async fn check_landing_zone(&self) -> Result<Value, ErrorObjectOwned> {
        let frame = match self.reader.capture_frame().await {
            Ok(f) => f,
            Err(e) => return Ok(serde_json::to_value(tool_result_err(e.to_string())).unwrap()),
        };

        match self.detector.check_landing_zone(&frame).await {
            Ok(verdict) => Ok(serde_json::to_value(tool_result_ok(verdict)).unwrap()),
            Err(e) => Ok(serde_json::to_value(tool_result_err(e.to_string())).unwrap()),
        }
    }
}

/// Démarre le serveur vision-mcp.
pub async fn start_server(
    reader: StreamReader,
    detector: ObjectDetector,
    port: u16,
) -> anyhow::Result<()> {
    let addr = format!("0.0.0.0:{}", port);
    let server = ServerBuilder::default().build(&addr).await?;

    let rpc_impl = VisionRpcServer::new(reader, detector);
    let handle = server.start(rpc_impl.into_rpc());

    tracing::info!("Serveur vision-mcp lancé sur http://{}", addr);

    handle.stopped().await;
    Ok(())
}
