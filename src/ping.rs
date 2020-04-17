use crate::ping::icmp_packets::PossibleIcmpPackets::ICMPv4EchoPacket;
use crate::ping::icmp_packets::{ICMPv4EchoPacketStruct, ICMPv4GenericPacket, PossibleIcmpPackets};
use etherparse::{InternetSlice, SlicedPacket};
use socket2::{Domain, Protocol, Socket, Type};
use std::net::{SocketAddr, ToSocketAddrs};
use std::time::Duration;

mod icmp_packets;

pub enum IpVersion {
    V4,
    V6,
}
pub struct Pinger {
    socket: Socket,
    ttl: u8,
    ip_version: IpVersion,
}

#[derive(Debug, Clone)]
pub enum PingSetupError {
    TtlSetup(String),
    SocketCreation(String),
}

#[derive(Debug, Clone)]
pub enum PingNetworkError {
    ErrorSendingPing(String),
    ErrorReceivingPing(String),
    InvalidIpPacket(String),
}

impl Pinger {
    pub fn new(ttl: u8, ip_version: IpVersion, timeout: Duration) -> Result<Self, PingSetupError> {
        let socket = match ip_version {
            IpVersion::V4 => Socket::new(Domain::ipv4(), Type::raw(), Some(Protocol::icmpv4()))
                .map_err(|e| PingSetupError::SocketCreation(e.to_string()))?,
            IpVersion::V6 => unimplemented!(),
        };
        socket.set_read_timeout(Some(timeout)).map_err(|e| PingSetupError::SocketCreation(e.to_string()));
        socket
            .set_ttl(ttl as u32)
            .map_err(|e| PingSetupError::TtlSetup(e.to_string()))?;
        Ok(Self {
            socket,
            ttl,
            ip_version,
        })
    }

    pub fn send_ping(&self, address: SocketAddr) -> Result<(), PingNetworkError> {
        let echo_requect_pkt = ICMPv4EchoPacketStruct::new(8, 0, 1, 9, &[]);
        let _nb_bytes_sent = self
            .socket
            .send_to(&echo_requect_pkt.as_bytes(), &address.into())
            .map_err(|e| PingNetworkError::ErrorSendingPing(e.to_string()))?;
        Ok(())
    }

    pub fn get_ping_response(&self) -> Result<PossibleIcmpPackets, PingNetworkError> {
        // IPv4 header max size (with options) is 32 bytes and max ICMP datagram size is 576
        // we add one more byte to make sure the server did not send over 32 + 576 bytes.
        // If it did, it is already an invalid ICMP packet and we clear the socket.
        let mut buf = [0; 32 + 576 + 1];

        let rev = self
            .socket
            .recv(&mut buf)
            .map_err(|e| PingNetworkError::ErrorReceivingPing(e.to_string()))?;
        let packet = SlicedPacket::from_ip(&buf)
            .map_err(|e| PingNetworkError::InvalidIpPacket(e.to_string()))?;
        let header_len_as_32_bits = match packet.ip.ok_or(PingNetworkError::InvalidIpPacket(
            "Packet did not have the correct structure".to_string(),
        ))? {
            InternetSlice::Ipv4(a) => a.ihl(),
            InternetSlice::Ipv6(a, b) => unimplemented!(),
        };
        let icmp_resp = &packet.payload[0..rev - (header_len_as_32_bits * 4) as usize];
        let pkt = ICMPv4GenericPacket::from_bytes(icmp_resp).specialize();
        Ok(pkt)
    }
}
