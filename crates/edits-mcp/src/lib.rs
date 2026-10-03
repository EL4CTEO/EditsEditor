//! # edits-mcp
//!
//! Model Context Protocol server exposing the EditsEditor engine to AI agents over stdio.

pub mod reference;
pub mod server;

use rmcp::ServiceExt;

pub use server::EditsServer;

/// Serve MCP over stdin/stdout until the client disconnects.
pub async fn serve_stdio(engine: Option<edits_engine::Engine>, gpu: edits_render::GpuOptions) -> anyhow::Result<()> {
    let server = EditsServer::new(engine, gpu);
    let service = server.serve(rmcp::transport::stdio()).await?;
    service.waiting().await?;
    Ok(())
}
