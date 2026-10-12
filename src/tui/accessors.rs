//! Small accessors the host application uses to drive an [`App`].

use std::sync::Arc;

use tokio::sync::mpsc::UnboundedSender;

use super::{AgentEvent, App};

impl App {
    /// A handle to signal cancellation of the running turn.
    #[must_use]
    pub fn cancel_notify(&self) -> Arc<tokio::sync::Notify> {
        self.cancel.clone()
    }

    /// The shared pause flag the agent loop reads.
    #[must_use]
    pub fn pause_flag(&self) -> Arc<std::sync::atomic::AtomicBool> {
        self.pause.clone()
    }

    /// The autonomy mode currently selected in the UI.
    #[must_use]
    pub fn mode(&self) -> crate::config::Mode {
        self.mode
    }

    /// Sender the agent should publish events to.
    #[must_use]
    #[allow(dead_code)] // retained for external event sources
    pub fn sender(&self) -> UnboundedSender<AgentEvent> {
        self.tx.clone()
    }
}
