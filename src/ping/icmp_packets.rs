pub use crate::ping::icmp_packets::time_exceeded_packet::ICMPv4TimeExceededPacketStruct;
pub use echo_packet::ICMPv4EchoPacketStruct;
mod echo_packet;
mod time_exceeded_packet;

#[derive(Debug)]
pub enum PossibleIcmpPackets {
    ICMPv4EchoPacket(ICMPv4EchoPacketStruct),
    ICMPv4TimeExceededPacketTtlExceeded,
    ICMPv4TimeExceededPacketFragmentReassemblyTimeExceeded,
    InvalidPacket,
    UnknownPacket,
}



#[derive(Debug)]
pub struct ICMPv4GenericPacket {
    /// Packet Type, for ICMP Echo Request is 8 and for reply 0
    /// 11 for Time Exceeded
    p_type: u8,
    /// Code is 0 for both both Request and Reply and 0 or 1 for Time Exceeded
    code: u8,
    checksum: u16,
    /// Meaning of rest_of_header depends on the packet type (p_type) and code
    rest_of_header: u32,
    /// Maximum size of the total datagram is 576 bytes as of
    /// https://tools.ietf.org/html/rfc1812#section-4.3
    /// Since the other fields take 8 bytes, this leaves
    /// 576-8 = 568 bytes for the data field
    data: Vec<u8>,
}

impl ICMPv4GenericPacket {
    pub fn from_bytes(slice: &[u8]) -> Self {
        assert!(slice.len() <= 576, "Datagram over 576 bytes!");
        Self {
            p_type: slice[0],
            code: slice[1],
            checksum: u16::from_ne_bytes([slice[2], slice[3]]),
            rest_of_header: u32::from_ne_bytes([slice[4], slice[5], slice[6], slice[7]]),
            data: slice[8..].to_vec(),
        }
    }

    pub fn specialize(&self) -> PossibleIcmpPackets {
        return match (self.p_type, self.code) {
            (0, 0) => {
                let rest_header_bytes: [u8; 4] = self.rest_of_header.to_ne_bytes();
                let id = u16::from_be_bytes([rest_header_bytes[0], rest_header_bytes[1]]);
                let sequence = u16::from_be_bytes([rest_header_bytes[2], rest_header_bytes[3]]);
                let pkt =
                    ICMPv4EchoPacketStruct::new(self.p_type, self.code, id, Some(self.checksum), sequence, &self.data);
                if !pkt.valid_checksum(){
                    return PossibleIcmpPackets::InvalidPacket;
                }
                PossibleIcmpPackets::ICMPv4EchoPacket(pkt)
            }
            (11, code) => {
                return match code {
                    0 => PossibleIcmpPackets::ICMPv4TimeExceededPacketTtlExceeded,
                    1 => {
                        PossibleIcmpPackets::ICMPv4TimeExceededPacketFragmentReassemblyTimeExceeded
                    }
                    _ => PossibleIcmpPackets::UnknownPacket,
                };
            }
            _ => PossibleIcmpPackets::UnknownPacket,
        };
    }
}
