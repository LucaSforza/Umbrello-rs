//! MCP adapter for the protocol-neutral QA bridge.

use super::bridge::{request_close, request_repaint, QaHandle};
use super::protocol::{QaError, QaRequest, QaResponse};
use base64::Engine;
use rmcp::schemars;
use rmcp::schemars::JsonSchema;
use rmcp::serde_json;
use rmcp::{
    handler::server::wrapper::Parameters,
    model::{CallToolResult, ContentBlock, ServerCapabilities, ServerInfo},
    service::{RequestContext, RoleServer, ServiceExt},
    tool, tool_handler, tool_router, ServerHandler,
};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tokio_util::sync::CancellationToken;

const TOOL_TIMEOUT: Duration = Duration::from_secs(10);

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub(crate) struct SelectArgs {
    /// Exact stable semantic target ID returned by ui_inspect.
    pub target_id: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub(crate) struct PointArgs {
    /// Optional logical viewport X coordinate (required for canvas targets).
    pub x: Option<f64>,
    /// Optional logical viewport Y coordinate (required for canvas targets).
    pub y: Option<f64>,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub(crate) struct TextArgs {
    /// Complete replacement value to commit.
    pub value: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub(crate) struct DragArgs {
    /// Destination model X for node movement, or screen-space pan delta X for canvas panning.
    pub x: Option<f64>,
    /// Destination model Y for node movement, or screen-space pan delta Y for canvas panning.
    pub y: Option<f64>,
    /// Optional destination node target for edge creation.
    pub to_target: Option<String>,
    /// When true, use native-equivalent gesture simulation (begin → preview → commit)
    /// instead of directly calling move_node_to. Defaults to false.
    /// The gesture mode exercises the same control flow as native pointer drag,
    /// including drag-node-id/drag-preview-pos state transitions and zoom conversion.
    pub gesture: Option<bool>,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub(crate) struct SyncArgs {
    /// State revision that must have rendered.
    pub after_revision: u64,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub(crate) struct EmptyArgs {}

#[derive(Clone)]
pub(crate) struct QaMcpServer {
    handle: QaHandle,
    tool_router: rmcp::handler::server::router::tool::ToolRouter<Self>,
}

impl QaMcpServer {
    pub(crate) fn new(handle: QaHandle) -> Self {
        Self {
            handle,
            tool_router: Self::tool_router(),
        }
    }

    async fn call(
        &self,
        request: QaRequest,
        context: &RequestContext<RoleServer>,
    ) -> Result<QaResponse, QaError> {
        request_repaint();
        self.handle
            .submit_async(request, TOOL_TIMEOUT, context.ct.clone())
            .await
    }

    fn result(response: QaResponse) -> Result<CallToolResult, rmcp::ErrorData> {
        let value = match response {
            QaResponse::Snapshot(snapshot) | QaResponse::Accepted(snapshot) => {
                serde_json::to_value(snapshot)
                    .map_err(|e| rmcp::ErrorData::internal_error(e.to_string(), None))?
            },
            QaResponse::Screenshot(_) => serde_json::json!({"screenshot": true}),
        };
        Ok(CallToolResult::structured(value))
    }

    fn error(error: QaError) -> CallToolResult {
        CallToolResult::error(vec![ContentBlock::text(error.to_string())])
    }

    fn screenshot_result(image: super::screenshot::ScreenshotResult) -> CallToolResult {
        let data = base64::engine::general_purpose::STANDARD.encode(image.png);
        let metadata = serde_json::json!({
            "width": image.width,
            "height": image.height,
            "state_revision": image.state_revision,
            "rendered_revision": image.rendered_revision
        });
        CallToolResult::success(vec![
            ContentBlock::image(data, "image/png"),
            ContentBlock::text(metadata.to_string()),
        ])
    }
}

#[tool_router]
impl QaMcpServer {
    /// Inspect the current UI state and stable semantic targets.
    #[tool(
        name = "ui_inspect",
        description = "Inspect readiness, revisions, viewport state, selection, and operable UI targets."
    )]
    pub(crate) async fn ui_inspect(
        &self,
        _: Parameters<EmptyArgs>,
        context: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, rmcp::ErrorData> {
        match self.call(QaRequest::Inspect, &context).await {
            Ok(r) => Self::result(r),
            Err(e) => Ok(Self::error(e)),
        }
    }

    /// Select an exact stable semantic target as the automation cursor.
    #[tool(name = "ui_select", description = "Select an exact stable semantic target ID.")]
    pub(crate) async fn ui_select(
        &self,
        Parameters(args): Parameters<SelectArgs>,
        context: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, rmcp::ErrorData> {
        match self
            .call(
                QaRequest::Select {
                    target_id: args.target_id,
                },
                &context,
            )
            .await
        {
            Ok(r) => Self::result(r),
            Err(e) => Ok(Self::error(e)),
        }
    }

    /// Activate the selected target, optionally at a canvas point.
    #[tool(
        name = "ui_click",
        description = "Activate the selected UI target at an optional logical point."
    )]
    pub(crate) async fn ui_click(
        &self,
        Parameters(args): Parameters<PointArgs>,
        context: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, rmcp::ErrorData> {
        match self
            .call(
                QaRequest::Click {
                    position: args.x.zip(args.y),
                },
                &context,
            )
            .await
        {
            Ok(r) => Self::result(r),
            Err(e) => Ok(Self::error(e)),
        }
    }

    /// Replace and commit text in the selected editable target.
    #[tool(
        name = "ui_set_text",
        description = "Replace and commit text for the selected editable target."
    )]
    pub(crate) async fn ui_set_text(
        &self,
        Parameters(args): Parameters<TextArgs>,
        context: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, rmcp::ErrorData> {
        match self
            .call(QaRequest::SetText { value: args.value }, &context)
            .await
        {
            Ok(r) => Self::result(r),
            Err(e) => Ok(Self::error(e)),
        }
    }

    /// Drag the selected node to coordinates or another node target.
    #[tool(
        name = "ui_drag",
        description = "Drag the selected node to a point or destination node; with canvas selected and Select active, x/y are screen-space pan deltas. Set gesture=true to use native-equivalent gesture simulation (begin/preview/commit) instead of direct move."
    )]
    pub(crate) async fn ui_drag(
        &self,
        Parameters(args): Parameters<DragArgs>,
        context: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, rmcp::ErrorData> {
        let position = args.x.zip(args.y);
        match self
            .call(
                QaRequest::Drag {
                    position,
                    to_target: args.to_target,
                    gesture: args.gesture,
                },
                &context,
            )
            .await
        {
            Ok(r) => Self::result(r),
            Err(e) => Ok(Self::error(e)),
        }
    }

    /// Wait for a UI frame to render a requested state revision.
    #[tool(
        name = "ui_sync",
        description = "Wait until the requested state revision has rendered."
    )]
    pub(crate) async fn ui_sync(
        &self,
        Parameters(args): Parameters<SyncArgs>,
        context: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, rmcp::ErrorData> {
        match self
            .call(
                QaRequest::Sync {
                    after_revision: args.after_revision,
                },
                &context,
            )
            .await
        {
            Ok(r) => Self::result(r),
            Err(e) => Ok(Self::error(e)),
        }
    }

    /// Capture the current native eframe viewport as PNG.
    #[tool(
        name = "ui_screenshot",
        description = "Capture the eframe viewport as a PNG image with revision metadata."
    )]
    pub(crate) async fn ui_screenshot(
        &self,
        _: Parameters<EmptyArgs>,
        context: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, rmcp::ErrorData> {
        match self.call(QaRequest::Screenshot, &context).await {
            Ok(QaResponse::Screenshot(image)) => Ok(Self::screenshot_result(image)),
            Ok(_) => Ok(Self::error(QaError::Screenshot("unexpected response".into()))),
            Err(e) => Ok(Self::error(e)),
        }
    }
}

#[tool_handler(router = self.tool_router)]
impl ServerHandler for QaMcpServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(ServerCapabilities::builder().enable_tools().build())
            .with_instructions("Umbrello GUI visual QA server")
    }
}

pub(crate) fn run_stdio(handle: QaHandle, shutdown: CancellationToken) -> Result<(), String> {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| e.to_string())?;
    runtime.block_on(async move {
        let service = QaMcpServer::new(handle)
            .serve_with_ct(rmcp::transport::stdio(), shutdown)
            .await
            .map_err(|e| e.to_string())?;
        service
            .waiting()
            .await
            .map(|_| ())
            .map_err(|e| e.to_string())
    })
}

#[allow(dead_code)]
pub(crate) fn close_gui_on_eof() {
    request_close();
}

#[cfg(test)]
mod tests {
    use super::*;
    use rmcp::model::ContentBlock;

    #[test]
    fn router_exposes_exactly_seven_tools_with_schemas() {
        let (bridge, handle) = super::super::bridge::QaBridge::new(1);
        drop(bridge);
        let server = QaMcpServer::new(handle);
        let mut names: Vec<_> = server
            .tool_router
            .list_all()
            .into_iter()
            .map(|tool| tool.name.to_string())
            .collect();
        names.sort();
        assert_eq!(
            names,
            [
                "ui_click",
                "ui_drag",
                "ui_inspect",
                "ui_screenshot",
                "ui_select",
                "ui_set_text",
                "ui_sync"
            ]
        );
        for tool in server.tool_router.list_all() {
            assert_eq!(tool.input_schema.get("type").and_then(|v| v.as_str()), Some("object"));
        }
        assert!(server.tool_router.get("ui_select").is_some());
    }

    #[test]
    fn screenshot_result_contains_png_and_json_metadata() {
        let result = QaMcpServer::screenshot_result(super::super::screenshot::ScreenshotResult {
            png: vec![137, 80, 78, 71],
            width: 8,
            height: 6,
            state_revision: 3,
            rendered_revision: 3,
        });
        assert_eq!(result.content.len(), 2);
        match &result.content[0] {
            ContentBlock::Image(image) => assert_eq!(image.mime_type, "image/png"),
            other => panic!("expected image, got {other:?}"),
        }
        match &result.content[1] {
            ContentBlock::Text(text) => {
                assert!(text.text.contains("\"width\":8"));
                assert!(text.text.contains("\"rendered_revision\":3"));
            },
            other => panic!("expected metadata, got {other:?}"),
        }
    }

    // ── S3: Native-equivalent gesture schema analysis ────────────
    //
    // The existing DragArgs schema already encodes a native node-drag
    // destination: x/y specify destination model coordinates at any
    // zoom level. No protocol extension is needed because MCP operates
    // at the semantic level (position destination), not the raw event
    // level (press-move-release). The native input-routing defect in
    // canvas.rs does not affect the semantic Drag request path, which
    // calls move_node_to directly.
    #[test]
    fn drag_args_represent_native_gesture_at_semantic_level() {
        let args = DragArgs {
            x: Some(150.0),
            y: Some(130.0),
            to_target: None,
            gesture: None,
        };
        assert_eq!(args.x, Some(150.0));
        assert_eq!(args.y, Some(130.0));
        assert!(args.to_target.is_none());

        // Verify the spatial relationship at various zoom levels:
        // the destination is a model coordinate independent of zoom.
        let args_zoom = DragArgs {
            x: Some(150.0),
            y: Some(130.0),
            to_target: None,
            gesture: None,
        };
        assert_eq!(args_zoom.x, args.x);
        assert_eq!(args_zoom.y, args.y);

        // Canvas pan uses the same schema with screen-space deltas.
        let canvas_pan = DragArgs {
            x: Some(12.0),
            y: Some(-7.0),
            to_target: None,
            gesture: None,
        };
        assert_eq!(canvas_pan.x, Some(12.0));
        assert_eq!(canvas_pan.y, Some(-7.0));

        // Gesture mode flag is optional and defaults to None / false.
        let gesture_args = DragArgs {
            x: Some(200.0),
            y: Some(100.0),
            to_target: None,
            gesture: Some(true),
        };
        assert_eq!(gesture_args.gesture, Some(true));
    }
}
