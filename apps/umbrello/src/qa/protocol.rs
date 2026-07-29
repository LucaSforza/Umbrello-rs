//! Stable semantic QA protocol values. These types deliberately know nothing about MCP.

use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct UiTarget {
    pub id: String,
    pub kind: String,
    pub label: String,
    pub enabled: bool,
    pub selected: bool,
    pub element_id: Option<String>,
    pub diagram_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct UiSnapshot {
    pub ready: bool,
    pub ui_frame: u64,
    pub state_revision: u64,
    pub rendered_revision: u64,
    pub active_tool: String,
    pub active_diagram: Option<String>,
    pub selected_element: Option<String>,
    pub selected_qa_target: Option<String>,
    /// Persisted zoom of the active diagram, when one is active.
    pub zoom_percent: Option<f64>,
    /// Transient active-diagram pan in logical screen pixels.
    pub pan_x: Option<f64>,
    /// Transient active-diagram pan in logical screen pixels.
    pub pan_y: Option<f64>,
    pub status: String,
    pub targets: Vec<UiTarget>,
}

#[derive(Debug, Clone, PartialEq)]
#[allow(dead_code)] // Requests are constructed by the S2 transport adapter.
pub(crate) enum QaRequest {
    Inspect,
    Select {
        target_id: String,
    },
    Click {
        position: Option<(f64, f64)>,
    },
    SetText {
        value: String,
    },
    Drag {
        /// For a node, the destination model position. For the canvas, this is
        /// a screen-space pan delta `(dx, dy)`, not an absolute pointer point.
        position: Option<(f64, f64)>,
        to_target: Option<String>,
    },
    Sync {
        after_revision: u64,
    },
    Screenshot,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum QaResponse {
    Snapshot(UiSnapshot),
    Screenshot(super::screenshot::ScreenshotResult),
    Accepted(UiSnapshot),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum QaError {
    NotReady,
    QueueFull,
    Disconnected,
    Cancelled,
    Timeout,
    UnavailableTarget(String),
    WrongTargetKind(String),
    InvalidCoordinates,
    InvalidValue(String),
    Command(String),
    Screenshot(String),
    Shutdown,
}

impl fmt::Display for QaError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotReady => f.write_str("UI is not ready"),
            Self::QueueFull => f.write_str("QA request queue is full"),
            Self::Disconnected => f.write_str("QA bridge is disconnected"),
            Self::Cancelled => f.write_str("QA request was cancelled"),
            Self::Timeout => f.write_str("QA request timed out"),
            Self::UnavailableTarget(id) => write!(f, "target is unavailable or stale: {id}"),
            Self::WrongTargetKind(id) => write!(f, "target has wrong kind: {id}"),
            Self::InvalidCoordinates => f.write_str("coordinates are invalid"),
            Self::InvalidValue(value) => write!(f, "invalid value: {value}"),
            Self::Command(value) => write!(f, "command failed: {value}"),
            Self::Screenshot(value) => write!(f, "screenshot failed: {value}"),
            Self::Shutdown => f.write_str("UI is shutting down"),
        }
    }
}

impl std::error::Error for QaError {}
