//! The prelude.
//! Simply import everything from this module to pull in the core capabilities of vkoxide.

pub use crate::bot::Bot;
pub use crate::dispatcher::{Context, Dispatcher, DispatcherBuilder};
pub use crate::filters;
pub use crate::keyboard::{Action, ButtonColor, Keyboard, KeyboardButton};
pub use crate::types::{KnownUpdate, Update, UpdateKind, VkError};
