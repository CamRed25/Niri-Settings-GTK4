// IPC types needed by the outputs page of niri-settings.

#![allow(dead_code)]

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize)]
pub struct OutputMode {
    pub width: u32,
    pub height: u32,
    pub refresh_rate: u32,
    pub is_preferred: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LogicalOutput {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub scale: f64,
    pub transform: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct NiriOutput {
    pub name: String,
    pub make: String,
    pub model: String,
    pub serial: String,
    pub physical_size: Option<[u32; 2]>,
    pub modes: Vec<OutputMode>,
    pub current_mode: Option<usize>,
    pub is_custom_mode: bool,
    pub vrr_supported: bool,
    pub vrr_enabled: bool,
    pub logical: Option<LogicalOutput>,
}

#[derive(Debug, Serialize)]
pub enum NiriRequest {
    Outputs,
}

#[derive(Debug, Deserialize)]
pub enum NiriReply {
    Ok(serde_json::Value),
    Err(String),
}
