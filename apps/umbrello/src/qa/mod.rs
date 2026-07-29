//! Protocol-neutral, UI-thread owned controls for automated GUI QA.

pub(crate) mod bridge;
pub(crate) mod control;
pub(crate) mod mcp;
pub(crate) mod protocol;
pub(crate) mod screenshot;

pub(crate) use bridge::{QaBridge, QaHandle};
