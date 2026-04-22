//! The prelude.
//! Simply import everything from this module to pull in the core capabilities of vkoxide.

pub use crate::bot::Bot;
pub use crate::dispatcher::{Dispatcher, DispatcherBuilder, ShutdownToken};
pub use crate::filter;
pub use crate::keyboard::{Action, ButtonColor, Keyboard, KeyboardButton};
pub use crate::types::{Command, KnownUpdate, MessageNewObject, Update, UpdateKind, VkError};
pub use dptree::{self, prelude::*};
