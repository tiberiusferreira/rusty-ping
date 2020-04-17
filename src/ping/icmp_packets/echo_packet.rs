/// Structure representing a ICMPv4 Packet which will be sent over IPv4.
/// Note that both ICMPv4 or ICMPv6 use big endian for the headers
/// This Packet is structured according to https://tools.ietf.org/html/rfc792,
/// in special the "Echo or Echo Reply Message" section
#[derive(Debug)]
pub struct ICMPv4EchoPacketStruct {
    /// Packet Type, for ICMP Echo Request is 8 and for reply 0
    p_type: u8,
    /// Code is 0 for both both Request and Reply
    code: u8,
    ///
    checksum: u16,
    /// Identifier to aid in matching echos and replies
    /// According to https://en.wikipedia.org/wiki/Ping_(networking_utility)#Echo_request
    /// the `id` is used to identify process which generated the ping
    id: u16,
    /// Sequence to also aid in matching echos and replies
    /// According to https://en.wikipedia.org/wiki/Ping_(networking_utility)#Echo_request
    /// the `sequence` is used to identify pings within the same process
    sequence: u16,
    /// Maximum size of the total datagram is 576 bytes as of
    /// https://tools.ietf.org/html/rfc1812#section-4.3
    /// Since the other fields take 8 bytes, this leaves
    /// 576-8 = 568 bytes for the data field
    data: Vec<u8>,
}

impl ICMPv4EchoPacketStruct {
    pub fn new(p_type: u8, code: u8, id: u16, sequence: u16, data: &[u8]) -> Self {
        assert!(
            data.len() <= 568,
            "Can't create a ICMP datagram over 576 bytes!"
        );
        let mut pkt_no_checksum = ICMPv4EchoPacketStruct {
            p_type,
            code,
            checksum: 0,
            id,
            sequence,
            data: data.to_vec(),
        };
        pkt_no_checksum.fill_checksum();
        pkt_no_checksum
    }

    /// This function already takes care of endianness
    pub fn fill_checksum(&mut self) {
        self.checksum = self.checksum();
    }

    /// This function already takes care of endianness
    pub fn checksum(&self) -> u16 {
        let mut checksum = internet_checksum::Checksum::new();
        checksum.add_bytes(&[self.p_type, self.code]);
        checksum.add_bytes(&self.id.to_be_bytes());
        checksum.add_bytes(&self.sequence.to_be_bytes());
        checksum.add_bytes(&self.data);
        let cks = checksum.checksum();
        u16::from_ne_bytes([cks[0], cks[1]])
    }

    pub fn as_bytes(&self) -> Vec<u8> {
        // tried both serde and bincode, but both append the length of the bytes into the
        // serialized result, so we do it manually here
        let mut buffer = Vec::new();
        buffer.push(self.p_type);
        buffer.push(self.code);
        buffer.extend_from_slice(&self.checksum.to_ne_bytes());
        buffer.extend_from_slice(&self.id.to_be_bytes());
        buffer.extend_from_slice(&self.sequence.to_be_bytes());
        buffer.extend_from_slice(&self.data);
        buffer
    }

    pub fn from_bytes(slice: &[u8]) -> Self {
        assert!(slice.len() <= 576, "Datagram over 576 bytes!");
        println!("{:X?}", u16::from_be_bytes([slice[6], slice[7]]));
        println!("{:?}", u16::from_le(1));
        Self {
            p_type: slice[0],
            code: slice[1],
            checksum: u16::from_ne_bytes([slice[2], slice[3]]),
            id: u16::from_be_bytes([slice[4], slice[5]]),
            sequence: u16::from_be_bytes([slice[6], slice[7]]),
            data: slice[8..].to_vec(),
        }
    }
}
