mod classic;
mod control;
mod runtime;

pub use classic::{ClassicAssembler, ClassicPacket, CompletedFrame, PacketOutcome};
pub use control::{ControlHeader, REMOTE_PLAY_COMMAND, TCP_MAGIC};
pub use runtime::{Viewer, ViewerHandle};

use serde::{Deserialize, Serialize};
use std::net::{IpAddr, Ipv4Addr};
use std::sync::Arc;
use thiserror::Error;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum StreamMode {
    #[default]
    Auto,
    JpegCompat,
    JpegReliable,
    JpegReliableDelta,
    Uncompressed,
    LosslessReliable,
    LosslessReliableDelta,
}

impl StreamMode {
    pub const ALL: [Self; 7] = [
        Self::Auto,
        Self::JpegCompat,
        Self::JpegReliable,
        Self::JpegReliableDelta,
        Self::Uncompressed,
        Self::LosslessReliable,
        Self::LosslessReliableDelta,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::Auto => "Auto",
            Self::JpegCompat => "JPEG Compat (UDP)",
            Self::JpegReliable => "JPEG (Reliable)",
            Self::JpegReliableDelta => "JPEG (Reliable, Delta)",
            Self::Uncompressed => "Uncompressed (UDP)",
            Self::LosslessReliable => "Lossless (Reliable)",
            Self::LosslessReliableDelta => "Lossless (Reliable, Delta)",
        }
    }

    pub fn command_mode(self) -> Self {
        match self {
            Self::Auto => Self::JpegReliableDelta,
            other => other,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ViewerConfig {
    pub console_ip: IpAddr,
    pub bind_ip: IpAddr,
    pub viewer_port: u16,
    pub stream_mode: StreamMode,
    pub jpeg_quality: u8,
    pub bandwidth_mbps: u8,
    pub top_screen_priority: bool,
    pub priority_factor: u8,
    pub lossless_color_bias: i8,
}

impl Default for ViewerConfig {
    fn default() -> Self {
        Self {
            console_ip: IpAddr::V4(Ipv4Addr::new(192, 168, 1, 24)),
            bind_ip: IpAddr::V4(Ipv4Addr::UNSPECIFIED),
            viewer_port: 8001,
            stream_mode: StreamMode::Auto,
            jpeg_quality: 75,
            bandwidth_mbps: 16,
            top_screen_priority: true,
            priority_factor: 2,
            lossless_color_bias: -1,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Screen {
    Top,
    Bottom,
}

#[derive(Clone, Debug)]
pub struct Frame {
    pub screen: Screen,
    pub sequence: u8,
    pub width: u32,
    pub height: u32,
    pub rgba: Arc<[u8]>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConnectionState {
    Disconnected,
    Connecting,
    Streaming,
}

#[derive(Clone, Debug)]
pub enum ViewerCommand {
    Connect(ViewerConfig),
    Disconnect,
    Shutdown,
}

#[derive(Clone, Debug)]
pub enum ViewerEvent {
    StateChanged(ConnectionState),
    ActiveMode(StreamMode),
    Frame(Frame),
    Stats {
        decoded: u64,
        dropped: u64,
        top_fps: f32,
        bottom_fps: f32,
    },
    Error(String),
}

#[derive(Debug, Error)]
pub enum ViewerError {
    #[error("invalid NTR packet: {0}")]
    Packet(&'static str),
    #[error("network error: {0}")]
    Io(#[from] std::io::Error),
    #[error("decode error: {0}")]
    Decode(#[from] ntr_codec::CodecError),
    #[error("stream mode {0:?} is not implemented yet")]
    UnsupportedMode(StreamMode),
}
