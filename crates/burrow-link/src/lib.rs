//! The phone's side of Burrow, without Tauri: its SSH key, the computer it
//! is paired with, and the hop to `burrow proxy --role mobile` there.

mod error;
mod files;
pub mod keys;
pub mod pairing;

pub use error::{Error, Result};
