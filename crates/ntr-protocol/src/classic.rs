use crate::{Screen, ViewerError};

pub const PACKET_SIZE: usize = 1448;
pub const HEADER_SIZE: usize = 4;
pub const PAYLOAD_SIZE: usize = PACKET_SIZE - HEADER_SIZE;
pub const MAX_FRAGMENTS: usize = 240;

#[derive(Clone, Copy, Debug)]
pub struct ClassicPacket<'a> {
    pub sequence: u8,
    pub screen: Screen,
    pub downsample: u8,
    pub lossless: bool,
    pub fragment: u8,
    pub end: bool,
    pub payload: &'a [u8],
}

impl<'a> ClassicPacket<'a> {
    pub fn parse(datagram: &'a [u8]) -> Result<Self, ViewerError> {
        if datagram.len() < HEADER_SIZE {
            return Err(ViewerError::Packet("header is shorter than four bytes"));
        }
        let header = &datagram[..4];
        let payload = &datagram[4..];
        let end = header[1] & 0x10 != 0;
        let screen_bit = header[1] & !0x10;
        if screen_bit > 1 {
            return Err(ViewerError::Packet("invalid screen field"));
        }
        if header[2] & !(0x0c | 0x01) != 0x02 {
            return Err(ViewerError::Packet("invalid classic protocol flags"));
        }
        if !end && payload.len() != PAYLOAD_SIZE {
            return Err(ViewerError::Packet("non-final fragment has wrong size"));
        }
        if payload.len() > PAYLOAD_SIZE || header[3] as usize >= MAX_FRAGMENTS {
            return Err(ViewerError::Packet("fragment is out of range"));
        }
        Ok(Self {
            sequence: header[0],
            screen: if screen_bit == 1 {
                Screen::Top
            } else {
                Screen::Bottom
            },
            downsample: (header[2] & 0x0c) >> 2,
            lossless: header[2] & 1 != 0,
            fragment: header[3],
            end,
            payload,
        })
    }
}

#[derive(Debug)]
struct Assembly {
    sequence: u8,
    downsample: u8,
    lossless: bool,
    fragments: Vec<Option<Vec<u8>>>,
    end_fragment: Option<u8>,
}

impl Assembly {
    fn new(packet: &ClassicPacket<'_>) -> Self {
        Self {
            sequence: packet.sequence,
            downsample: packet.downsample,
            lossless: packet.lossless,
            fragments: (0..MAX_FRAGMENTS).map(|_| None).collect(),
            end_fragment: None,
        }
    }
}

#[derive(Clone, Debug)]
pub struct CompletedFrame {
    pub sequence: u8,
    pub screen: Screen,
    pub downsample: u8,
    pub lossless: bool,
    pub encoded: Vec<u8>,
}

#[derive(Clone, Debug)]
pub enum PacketOutcome {
    Pending,
    Complete(CompletedFrame),
    ReplacedIncomplete,
}

#[derive(Default, Debug)]
pub struct ClassicAssembler {
    top: Option<Assembly>,
    bottom: Option<Assembly>,
}

impl ClassicAssembler {
    pub fn push(&mut self, packet: ClassicPacket<'_>) -> PacketOutcome {
        let slot = match packet.screen {
            Screen::Top => &mut self.top,
            Screen::Bottom => &mut self.bottom,
        };
        let replaced = slot.as_ref().is_some_and(|a| {
            a.sequence != packet.sequence
                || a.downsample != packet.downsample
                || a.lossless != packet.lossless
        });
        if slot.is_none() || replaced {
            *slot = Some(Assembly::new(&packet));
        }
        let assembly = slot.as_mut().expect("assembly was initialized");
        assembly.fragments[packet.fragment as usize] = Some(packet.payload.to_vec());
        if packet.end {
            assembly.end_fragment = Some(packet.fragment);
        }
        let Some(end) = assembly.end_fragment else {
            return if replaced {
                PacketOutcome::ReplacedIncomplete
            } else {
                PacketOutcome::Pending
            };
        };
        if assembly.fragments[..=end as usize]
            .iter()
            .any(Option::is_none)
        {
            return if replaced {
                PacketOutcome::ReplacedIncomplete
            } else {
                PacketOutcome::Pending
            };
        }
        let mut encoded = Vec::with_capacity(end as usize * PAYLOAD_SIZE + PAYLOAD_SIZE);
        for fragment in &assembly.fragments[..=end as usize] {
            encoded.extend_from_slice(fragment.as_ref().expect("checked above"));
        }
        let frame = CompletedFrame {
            sequence: assembly.sequence,
            screen: packet.screen,
            downsample: assembly.downsample,
            lossless: assembly.lossless,
            encoded,
        };
        *slot = None;
        PacketOutcome::Complete(frame)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn datagram(seq: u8, fragment: u8, end: bool, payload: &[u8]) -> Vec<u8> {
        let mut d = vec![seq, 1 | if end { 0x10 } else { 0 }, 2, fragment];
        d.extend_from_slice(payload);
        d
    }

    #[test]
    fn reassembles_out_of_order() {
        let p0 = vec![1; PAYLOAD_SIZE];
        let last = vec![2, 3];
        let d1 = datagram(9, 1, true, &last);
        let d0 = datagram(9, 0, false, &p0);
        let mut a = ClassicAssembler::default();
        assert!(matches!(
            a.push(ClassicPacket::parse(&d1).unwrap()),
            PacketOutcome::Pending
        ));
        let PacketOutcome::Complete(frame) = a.push(ClassicPacket::parse(&d0).unwrap()) else {
            panic!()
        };
        assert_eq!(frame.encoded.len(), PAYLOAD_SIZE + 2);
        assert_eq!(&frame.encoded[PAYLOAD_SIZE..], &last);
    }

    #[test]
    fn rejects_short_non_final_packet() {
        let d = datagram(1, 0, false, &[1, 2]);
        assert!(ClassicPacket::parse(&d).is_err());
    }
}
