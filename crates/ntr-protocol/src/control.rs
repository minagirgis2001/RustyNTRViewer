use crate::{StreamMode, ViewerConfig};

pub const TCP_MAGIC: u32 = 0x1234_5678;
pub const REMOTE_PLAY_COMMAND: u32 = 901;
pub const CONTROL_HEADER_LEN: usize = 84;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ControlHeader {
    pub sequence: u32,
    pub packet_type: u32,
    pub command: u32,
    pub args: [u32; 16],
    pub data_len: u32,
}

impl ControlHeader {
    pub fn heartbeat(sequence: u32) -> Self {
        Self {
            sequence,
            packet_type: 0,
            command: 0,
            args: [0; 16],
            data_len: 0,
        }
    }

    pub fn remote_play(sequence: u32, config: &ViewerConfig, mode: StreamMode) -> Self {
        let mut args = [0; 16];
        args[0] = ((config.top_screen_priority as u32) << 8) | config.priority_factor as u32;
        args[1] = config.jpeg_quality as u32;
        args[2] = config.bandwidth_mbps as u32 * 128 * 1024;
        args[3] = 1_404_036_572;

        let mode = mode.command_mode();
        let reliable = matches!(
            mode,
            StreamMode::JpegReliable
                | StreamMode::JpegReliableDelta
                | StreamMode::LosslessReliable
                | StreamMode::LosslessReliableDelta
        );
        let delta = matches!(
            mode,
            StreamMode::JpegReliableDelta | StreamMode::LosslessReliableDelta
        );
        let lossless = matches!(
            mode,
            StreamMode::Uncompressed
                | StreamMode::LosslessReliable
                | StreamMode::LosslessReliableDelta
        );
        let color =
            (2_i32 - config.lossless_color_bias.clamp(-2, 0).unsigned_abs() as i32) as u32 & 0x3;
        args[4] = config.viewer_port as u32
            | ((reliable as u32) << 30)
            | ((delta as u32) << 31)
            | ((lossless as u32) << 29)
            | if lossless { color << 27 } else { 0 };

        Self {
            sequence,
            packet_type: 0,
            command: REMOTE_PLAY_COMMAND,
            args,
            data_len: 0,
        }
    }

    pub fn encode(&self) -> [u8; CONTROL_HEADER_LEN] {
        let mut out = [0; CONTROL_HEADER_LEN];
        let values = [TCP_MAGIC, self.sequence, self.packet_type, self.command];
        for (index, value) in values.into_iter().enumerate() {
            out[index * 4..index * 4 + 4].copy_from_slice(&value.to_le_bytes());
        }
        for (index, value) in self.args.iter().enumerate() {
            let start = 16 + index * 4;
            out[start..start + 4].copy_from_slice(&value.to_le_bytes());
        }
        out[80..84].copy_from_slice(&self.data_len.to_le_bytes());
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn heartbeat_has_expected_wire_size_and_magic() {
        let bytes = ControlHeader::heartbeat(7).encode();
        assert_eq!(bytes.len(), 84);
        assert_eq!(&bytes[..4], &TCP_MAGIC.to_le_bytes());
        assert_eq!(&bytes[4..8], &7_u32.to_le_bytes());
    }

    #[test]
    fn classic_remote_play_encodes_defaults() {
        let config = ViewerConfig::default();
        let h = ControlHeader::remote_play(1, &config, StreamMode::JpegCompat);
        assert_eq!(h.command, 901);
        assert_eq!(h.args[0], 0x102);
        assert_eq!(h.args[1], 75);
        assert_eq!(h.args[2], 16 * 128 * 1024);
        assert_eq!(h.args[4], 8001);
    }

    #[test]
    fn reliable_delta_sets_high_bits() {
        let h =
            ControlHeader::remote_play(1, &ViewerConfig::default(), StreamMode::JpegReliableDelta);
        assert_ne!(h.args[4] & (1 << 30), 0);
        assert_ne!(h.args[4] & (1 << 31), 0);
    }
}
