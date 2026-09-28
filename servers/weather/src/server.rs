use crate::openweather::OpenWeatherClient;
use common::mcp::{tool_result_err, tool_result_ok};
use jsonrpsee::proc_macros::rpc;
use jsonrpsee::server::ServerBuilder;
use jsonrpsee::types::ErrorObjectOwned;
use serde_json::Value;

#[rpc(server)]
pub trait WeatherMcpRpc {
    #[method(name = "get_conditions")]
    async fn get_conditions(&self) -> Result<Value, ErrorObjectOwned>;

    #[method(name = "is_safe_to_fly")]
    async fn is_safe_to_fly(&self) -> Result<Value, ErrorObjectOwned>;
}

pub struct WeatherRpcServer {
    client: OpenWeatherClient,
}

impl WeatherRpcServer {
    pub fn new(client: OpenWeatherClient) -> Self {
        Self { client }
    }
}

#[async_trait::async_trait]
impl WeatherMcpRpcServer for WeatherRpcServer {
    async fn get_conditions(&self) -> Result<Value, ErrorObjectOwned> {
        match self.client.get_conditions().await {
            Ok(cond) => Ok(serde_json::to_value(tool_result_ok(cond)).unwrap()),
            Err(e) => Ok(serde_json::to_value(tool_result_err(e.to_string())).unwrap()),
        }
    }

    async fn is_safe_to_fly(&self) -> Result<Value, ErrorObjectOwned> {
        match self.client.is_safe_to_fly().await {
            Ok(verdict) => Ok(serde_json::to_value(tool_result_ok(verdict)).unwrap()),
            Err(e) => Ok(serde_json::to_value(tool_result_err(e.to_string())).unwrap()),
        }
    }
}

/// Démarre le serveur weather-mcp.
pub async fn start_server(client: OpenWeatherClient, port: u16) -> anyhow::Result<()> {
    let addr = format!("127.0.0.1:{}", port);
    let server = ServerBuilder::default().build(&addr).await?;

    let rpc_impl = WeatherRpcServer::new(client);
    let handle = server.start(rpc_impl.into_rpc());

    tracing::info!("Serveur weather-mcp lancé sur http://{}", addr);

    handle.stopped().await;
    Ok(())
}
