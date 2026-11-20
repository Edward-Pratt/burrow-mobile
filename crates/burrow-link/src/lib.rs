//! The phone's side of Burrow, without Tauri: its SSH key, the computer it
//! is paired with, and the hop to `burrow proxy --role mobile` there.

pub mod allow;
mod backoff;
pub mod bridge;
mod conn;
mod error;
mod files;
pub mod hostkey;
pub mod keys;
pub mod link;
pub mod pairing;

pub use error::{Error, Result};
