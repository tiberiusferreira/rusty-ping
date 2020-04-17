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

impl ICMPv4TimeExceededPacketStruct {
    pub fn verify_valid_packet(p_type: u8, code: u8, checksum: u16, data: &[u8]) -> bool {
        assert!(
            data.len() <= 568,
            "Can't create a ICMP datagram over 576 bytes!"
        );
        let mut pkt_no_checksum = ICMPv4TimeExceededPacketStruct {
            p_type,
            code,
            checksum,
            unused: 0,
            data: data.to_vec(),
        };
        pkt_no_checksum.verify_checksum()
    }

    fn verify_checksum(&mut self) -> bool {
        self.checksum == self.checksum()
    }

    /// This function already takes care of endianness
    fn checksum(&self) -> u16 {
        let mut checksum = internet_checksum::Checksum::new();
        checksum.add_bytes(&[self.p_type, self.code]);
        let cks = checksum.checksum();
        u16::from_ne_bytes([cks[0], cks[1]])
    }
}
