//! Local implementation of the Cursor `InferenceService` run channel.

mod convert;
mod session;

pub use session::{Frame, InferenceRegistry, Session};
