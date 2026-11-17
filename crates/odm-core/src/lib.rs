//! Open Download Manager core: download engine, configuration and persistence.

pub mod batch;
pub mod config;
pub mod engine;
pub mod grabber;
pub mod httpmeta;
pub mod ipc;
pub mod limiter;
pub mod model;
pub mod paths;
pub mod persist;
pub mod util;

pub use config::*;
pub use engine::{AddRequest, Engine, EngineEvent};
pub use model::{Download, DownloadId, Part, Status};
