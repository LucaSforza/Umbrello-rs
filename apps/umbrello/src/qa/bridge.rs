//! Bounded cross-thread request bridge. Only the handle is usable off the UI thread.

use super::protocol::{QaError, QaRequest, QaResponse};
use std::sync::mpsc::{self, Receiver, SyncSender};
use std::sync::OnceLock;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::time::Duration;
use std::time::Instant;

static REPAINT_CONTEXT: OnceLock<egui::Context> = OnceLock::new();

pub(crate) fn install_repaint_context(ctx: &egui::Context) {
    let _ = REPAINT_CONTEXT.set(ctx.clone());
}

pub(crate) fn request_repaint() {
    if let Some(ctx) = REPAINT_CONTEXT.get() {
        ctx.request_repaint();
    }
}

pub(crate) fn request_close() {
    if let Some(ctx) = REPAINT_CONTEXT.get() {
        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
    }
}

pub(crate) struct QaEnvelope {
    pub request: QaRequest,
    pub reply: SyncSender<Result<QaResponse, QaError>>,
    pub deadline: Instant,
    pub cancelled: Arc<AtomicBool>,
}

pub(crate) struct QaBridge {
    pub receiver: Receiver<QaEnvelope>,
}

#[derive(Clone)]
pub(crate) struct QaHandle {
    sender: SyncSender<QaEnvelope>,
}

/// A queued QA request that can be cancelled independently of waiting.
pub(crate) struct QaTicket {
    receiver: mpsc::Receiver<Result<QaResponse, QaError>>,
    cancelled: Arc<AtomicBool>,
}

impl QaTicket {
    fn cancellation_flag(&self) -> Arc<AtomicBool> {
        Arc::clone(&self.cancelled)
    }
    /// Cancel the request. The UI observes this exact flag before doing work.
    pub(crate) fn cancel(&self) {
        self.cancelled.store(true, Ordering::Release);
    }

    /// Wait for the queued request to complete.
    #[allow(dead_code)] // S2 may use an unbounded wait for tool calls.
    pub(crate) fn wait(self) -> Result<QaResponse, QaError> {
        self.receiver.recv().map_err(|_| QaError::Disconnected)?
    }

    /// Wait for completion, cancelling immediately if the deadline expires.
    pub(crate) fn wait_timeout(self, timeout: Duration) -> Result<QaResponse, QaError> {
        match self.receiver.recv_timeout(timeout) {
            Ok(result) => result,
            Err(mpsc::RecvTimeoutError::Timeout) => {
                self.cancel();
                Err(QaError::Timeout)
            },
            Err(mpsc::RecvTimeoutError::Disconnected) => Err(QaError::Disconnected),
        }
    }
}

impl Drop for QaTicket {
    fn drop(&mut self) {
        self.cancel();
    }
}

impl QaBridge {
    pub(crate) fn new(capacity: usize) -> (Self, QaHandle) {
        let (sender, receiver) = mpsc::sync_channel(capacity.max(1));
        (Self { receiver }, QaHandle { sender })
    }
}

impl QaHandle {
    pub(crate) async fn submit_async(
        &self,
        request: QaRequest,
        timeout: Duration,
        cancellation: tokio_util::sync::CancellationToken,
    ) -> Result<QaResponse, QaError> {
        let ticket = self.submit_ticket(request, timeout)?;
        let cancelled = ticket.cancellation_flag();
        tokio::select! {
            result = tokio::task::spawn_blocking(move || ticket.wait_timeout(timeout)) => {
                result.map_err(|_| QaError::Disconnected)?
            }
            _ = cancellation.cancelled() => {
                cancelled.store(true, Ordering::Release);
                Err(QaError::Cancelled)
            }
        }
    }

    #[allow(dead_code)] // S2 transport adapter entry point.
    pub(crate) fn submit(&self, request: QaRequest) -> Result<QaResponse, QaError> {
        self.submit_timeout(request, Duration::from_secs(5))
    }

    /// Enqueue a request without waiting for the UI thread.
    pub(crate) fn submit_ticket(
        &self,
        request: QaRequest,
        timeout: Duration,
    ) -> Result<QaTicket, QaError> {
        let (reply, receiver) = mpsc::sync_channel(1);
        let cancelled = Arc::new(AtomicBool::new(false));
        self.sender
            .try_send(QaEnvelope {
                request,
                reply,
                deadline: Instant::now() + timeout,
                cancelled: Arc::clone(&cancelled),
            })
            .map_err(|error| match error {
                mpsc::TrySendError::Full(_) => QaError::QueueFull,
                mpsc::TrySendError::Disconnected(_) => QaError::Disconnected,
            })?;
        Ok(QaTicket {
            receiver,
            cancelled,
        })
    }

    pub(crate) fn submit_timeout(
        &self,
        request: QaRequest,
        timeout: Duration,
    ) -> Result<QaResponse, QaError> {
        self.submit_ticket(request, timeout)?.wait_timeout(timeout)
    }
}
