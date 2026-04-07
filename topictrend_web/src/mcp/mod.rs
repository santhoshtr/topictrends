use rmcp_openapi::Server;
use serde_json::Value;
use url::Url;

const OPENAPI_YAML: &str = include_str!("../../openapi.yaml");

pub fn build_server(base_url: &str) -> Result<Server, Box<dyn std::error::Error + Send + Sync>> {
    let openapi_spec: Value = serde_yaml::from_str(OPENAPI_YAML)?;
    let base_url = Url::parse(base_url)?;

    let mut server = Server::builder()
        .openapi_spec(openapi_spec)
        .base_url(base_url)
        .name("topictrends".to_string())
        .title("TopicTrends MCP Server".to_string())
        .instructions("Exposes the TopicTrends HTTP API as MCP tools.".to_string())
        .build();

    server.load_openapi_spec()?;
    server.validate_registry()?;

    Ok(server)
}
