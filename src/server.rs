use crate::gmail::GmailServer;
use rmcp::{
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::*,
    schemars,
    service::RequestContext,
    tool, tool_handler, tool_router, ErrorData as McpError, RoleServer, ServerHandler,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::error;

#[derive(Clone)]
pub struct GmailMcpServer {
    gmail_server: Arc<GmailServer>,
    tool_router: ToolRouter<GmailMcpServer>,
}

#[tool_router]
impl GmailMcpServer {
    pub fn new(gmail_server: Arc<GmailServer>) -> Self {
        Self {
            gmail_server,
            tool_router: Self::tool_router(),
        }
    }

    #[tool(description = "Search Gmail threads using a query string")]
    async fn search_threads(
        &self,
        Parameters(args): Parameters<SearchThreadsArgs>,
    ) -> Result<CallToolResult, McpError> {
        match crate::tools::search_threads(
            &self.gmail_server,
            &args.query,
            args.max_results.unwrap_or(10),
        )
        .await
        {
            Ok(result) => Ok(CallToolResult::success(vec![Content::text(
                serde_json::to_string_pretty(&result).unwrap_or_else(|e| format!("Error: {e}")),
            )])),
            Err(e) => {
                error!("Failed to search threads: {}", e);
                Err(McpError::internal_error(
                    "search_failed",
                    Some(serde_json::json!({ "error": e.to_string() })),
                ))
            }
        }
    }

    #[tool(description = "Create a Gmail draft")]
    async fn create_draft(
        &self,
        Parameters(args): Parameters<CreateDraftArgs>,
    ) -> Result<CallToolResult, McpError> {
        match crate::tools::create_draft(
            &self.gmail_server,
            &args.to,
            &args.subject,
            &args.body,
            args.thread_id.as_deref(),
        )
        .await
        {
            Ok(result) => Ok(CallToolResult::success(vec![Content::text(
                serde_json::to_string_pretty(&result).unwrap_or_else(|e| format!("Error: {e}")),
            )])),
            Err(e) => {
                error!("Failed to create draft: {}", e);
                Err(McpError::internal_error(
                    "create_draft_failed",
                    Some(serde_json::json!({ "error": e.to_string() })),
                ))
            }
        }
    }

    #[tool(description = "Extract text from an email attachment by filename")]
    async fn extract_attachment_by_filename(
        &self,
        Parameters(args): Parameters<ExtractAttachmentArgs>,
    ) -> Result<CallToolResult, McpError> {
        match crate::tools::extract_attachment_by_filename(
            &self.gmail_server,
            &args.message_id,
            &args.filename,
        )
        .await
        {
            Ok(result) => Ok(CallToolResult::success(vec![Content::text(
                serde_json::to_string_pretty(&result).unwrap_or_else(|e| format!("Error: {e}")),
            )])),
            Err(e) => {
                error!("Failed to extract attachment: {}", e);
                Err(McpError::internal_error(
                    "extract_attachment_failed",
                    Some(serde_json::json!({ "error": e.to_string() })),
                ))
            }
        }
    }

    #[tool(description = "Fetch email bodies for thread IDs")]
    async fn fetch_email_bodies(
        &self,
        Parameters(args): Parameters<FetchEmailBodiesArgs>,
    ) -> Result<CallToolResult, McpError> {
        match crate::tools::fetch_email_bodies(&self.gmail_server, &args.thread_ids).await {
            Ok(result) => Ok(CallToolResult::success(vec![Content::text(
                serde_json::to_string_pretty(&result).unwrap_or_else(|e| format!("Error: {e}")),
            )])),
            Err(e) => {
                error!("Failed to fetch email bodies: {}", e);
                Err(McpError::internal_error(
                    "fetch_email_bodies_failed",
                    Some(serde_json::json!({ "error": e.to_string() })),
                ))
            }
        }
    }

    #[tool(description = "Download an attachment to a local file")]
    async fn download_attachment(
        &self,
        Parameters(args): Parameters<DownloadAttachmentArgs>,
    ) -> Result<CallToolResult, McpError> {
        match crate::tools::download_attachment(
            &self.gmail_server,
            &args.message_id,
            &args.filename,
            args.download_dir.as_deref(),
        )
        .await
        {
            Ok(result) => Ok(CallToolResult::success(vec![Content::text(
                serde_json::to_string_pretty(&result).unwrap_or_else(|e| format!("Error: {e}")),
            )])),
            Err(e) => {
                error!("Failed to download attachment: {}", e);
                Err(McpError::internal_error(
                    "download_attachment_failed",
                    Some(serde_json::json!({ "error": e.to_string() })),
                ))
            }
        }
    }

    #[tool(description = "Forward an email")]
    async fn forward_email(
        &self,
        Parameters(args): Parameters<ForwardEmailArgs>,
    ) -> Result<CallToolResult, McpError> {
        match crate::tools::forward_email(
            &self.gmail_server,
            &args.message_id,
            &args.to,
            &args.subject,
            &args.body,
        )
        .await
        {
            Ok(result) => Ok(CallToolResult::success(vec![Content::text(
                serde_json::to_string_pretty(&result).unwrap_or_else(|e| format!("Error: {e}")),
            )])),
            Err(e) => {
                error!("Failed to forward email: {}", e);
                Err(McpError::internal_error(
                    "forward_email_failed",
                    Some(serde_json::json!({ "error": e.to_string() })),
                ))
            }
        }
    }

    #[tool(description = "Send a draft email")]
    async fn send_draft(
        &self,
        Parameters(args): Parameters<SendDraftArgs>,
    ) -> Result<CallToolResult, McpError> {
        match crate::tools::send_draft(&self.gmail_server, &args.draft_id).await {
            Ok(result) => Ok(CallToolResult::success(vec![Content::text(
                serde_json::to_string_pretty(&result).unwrap_or_else(|e| format!("Error: {e}")),
            )])),
            Err(e) => {
                error!("Failed to send draft: {}", e);
                Err(McpError::internal_error(
                    "send_draft_failed",
                    Some(serde_json::json!({ "error": e.to_string() })),
                ))
            }
        }
    }
}

#[derive(Debug, Serialize, Deserialize, schemars::JsonSchema)]
pub struct SearchThreadsArgs {
    /// Gmail search query (e.g., "from:example@gmail.com", "subject:meeting")
    pub query: String,
    /// Maximum number of results to return (default: 10)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_results: Option<i64>,
}

#[derive(Debug, Serialize, Deserialize, schemars::JsonSchema)]
pub struct CreateDraftArgs {
    /// Recipient email address
    pub to: String,
    /// Email subject
    pub subject: String,
    /// Email body text
    pub body: String,
    /// Optional thread ID to reply to
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thread_id: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, schemars::JsonSchema)]
pub struct ExtractAttachmentArgs {
    /// Gmail message ID
    pub message_id: String,
    /// Attachment filename
    pub filename: String,
}

#[derive(Debug, Serialize, Deserialize, schemars::JsonSchema)]
pub struct FetchEmailBodiesArgs {
    /// List of thread IDs to fetch
    pub thread_ids: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, schemars::JsonSchema)]
pub struct DownloadAttachmentArgs {
    /// Gmail message ID
    pub message_id: String,
    /// Attachment filename
    pub filename: String,
    /// Optional download directory (default: current directory)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub download_dir: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, schemars::JsonSchema)]
pub struct ForwardEmailArgs {
    /// Gmail message ID to forward
    pub message_id: String,
    /// Recipient email address
    pub to: String,
    /// Forward subject
    pub subject: String,
    /// Forward body text
    pub body: String,
}

#[derive(Debug, Serialize, Deserialize, schemars::JsonSchema)]
pub struct SendDraftArgs {
    /// Gmail draft ID to send
    pub draft_id: String,
}

#[tool_handler]
impl ServerHandler for GmailMcpServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            protocol_version: ProtocolVersion::V_2024_11_05,
            capabilities: ServerCapabilities::builder()
                .enable_tools()
                .build(),
            server_info: Implementation::from_build_env(),
            instructions: Some(
                "Gmail MCP Server - Provides tools for searching, reading, and managing Gmail emails. \
                Tools: search_threads, create_draft, extract_attachment_by_filename, fetch_email_bodies, \
                download_attachment, forward_email, send_draft.".to_string(),
            ),
        }
    }

    async fn list_resources(
        &self,
        _request: Option<PaginatedRequestParam>,
        _: RequestContext<RoleServer>,
    ) -> Result<ListResourcesResult, McpError> {
        Ok(ListResourcesResult {
            resources: vec![],
            next_cursor: None,
        })
    }

    async fn read_resource(
        &self,
        _request: ReadResourceRequestParam,
        _: RequestContext<RoleServer>,
    ) -> Result<ReadResourceResult, McpError> {
        Err(McpError::resource_not_found(
            "resource_not_found",
            Some(serde_json::json!({ "message": "Resources not supported" })),
        ))
    }

    async fn list_resource_templates(
        &self,
        _request: Option<PaginatedRequestParam>,
        _: RequestContext<RoleServer>,
    ) -> Result<ListResourceTemplatesResult, McpError> {
        Ok(ListResourceTemplatesResult {
            next_cursor: None,
            resource_templates: vec![],
        })
    }

    async fn initialize(
        &self,
        _request: InitializeRequestParam,
        context: RequestContext<RoleServer>,
    ) -> Result<InitializeResult, McpError> {
        if let Some(http_request_part) = context.extensions.get::<axum::http::request::Parts>() {
            let initialize_headers = &http_request_part.headers;
            let initialize_uri = &http_request_part.uri;
            tracing::info!(?initialize_headers, %initialize_uri, "initialize from http server");
        }
        Ok(self.get_info())
    }
}
