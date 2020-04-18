#[derive(Debug)]
pub struct ICMPv4TimeExceededPacketStruct {
    /// Packet Type, for ICMP Echo Request is 8 and for reply 0
    p_type: u8,
    /// Code is 0 for both both Request and Reply
    code: u8,
    ///
    checksum: u16,
    unused: u32,
    /// Maximum size of the total datagram is 576 bytes as of
    /// https://tools.ietf.org/html/rfc1812#section-4.3
    /// Since the other fields take 8 bytes, this leaves
    /// 576-8 = 568 bytes for the data field
    data: Vec<u8>,
}
