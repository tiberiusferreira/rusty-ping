use etherparse::{InternetSlice, SlicedPacket};
use socket2::{Domain, Protocol, Socket, Type};
use std::io::{Error, Read, Write};
use std::net::{SocketAddr, SocketAddrV4, ToSocketAddrs};
use std::time::Duration;
mod ping;
use internet_checksum::checksum;
use ping::*;
use std::path::PathBuf;
use std::process::exit;
use std::thread::sleep;
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
#[structopt(name = "Simple Ping")]
struct Opt {
    /// Set TTL
    #[structopt(short, long, default_value = "64")]
    ttl: u8,

    /// Set timeout, how long to wait on socket read operations in ms
    #[structopt(long, default_value = "1000")]
    timeout: u64,

    /// Hostname or ip to ping
    #[structopt(short, long)]
    hostname_or_ip: String,
}

fn main() {
    let mut opt: Opt = Opt::from_args();
    let original_hostname = opt.hostname_or_ip.clone();
    // this is required for the to_socket_addrs call
    opt.hostname_or_ip.push_str(":0");

    let pinger = match Pinger::new(opt.ttl, IpVersion::V4, Duration::from_millis(opt.timeout)) {
        Ok(pinger) => pinger,
        Err(e) => {
            println!("Error setting up socket for pinging: {:?}", e);
            exit(1);
        }
    };

    let address = match opt.hostname_or_ip.to_socket_addrs() {
        Ok(mut res) => match res.next() {
            None => {
                println!("No ip address found for hostname.");
                exit(1);
            }
            Some(addr) => addr,
        },
        Err(e) => {
            println!("Error looking up hostname IP: {:?}", e);
            exit(1);
        }
    };

    let mut total_pings_sent = 0;
    let mut total_pings_lost = 0;
    let mut ping_id = 0;
    'pinging_loop: loop {
        sleep(Duration::from_secs(1));
        let send_time = std::time::Instant::now();
        println!("PINGING {} ({})", original_hostname, address.ip());

        'sending_loop: loop {
            match pinger.send_ping(address, ping_id) {
                Ok(sent) => {
                    println!("Sent {} bytes", sent);
                    total_pings_sent += 1;
                    ping_id += 1;
                    break 'sending_loop;
                }
                Err(e) => {
                    println!("Error sending ping: {:?}", e);
                    println!("Retrying in 1 second.");
                    std::thread::sleep(Duration::from_secs(1));
                }
            }
        }

        println!("Waiting response");
        let response = match pinger.get_ping_response() {
            Ok(resp) => resp,
            Err(e) => {
                total_pings_lost += 1;
                println!("Error getting response: {:?}", e);
                continue 'pinging_loop;
            }
        };

        let packet = match &response.packet{
            PossibleIcmpPackets::ICMPv4EchoPacket(pkt) => pkt,
            PossibleIcmpPackets::ICMPv4TimeExceededPacketTtlExceeded => {
                total_pings_lost += 1;
                println!("Got response, but was a TTL exceeded one");
                continue 'pinging_loop;
            },
            PossibleIcmpPackets::ICMPv4TimeExceededPacketFragmentReassemblyTimeExceeded => {
                total_pings_lost += 1;
                println!("Got response, but was a Fragment Reassembly Time Exceeded one");
                continue 'pinging_loop;
            },
            PossibleIcmpPackets::InvalidPacket => {
                total_pings_lost += 1;
                println!("Got response, but the packet was invalid, checksum was not valid");
                continue 'pinging_loop;
            },
            PossibleIcmpPackets::UnknownPacket => {
                total_pings_lost += 1;
                println!("Got response, but was an unknown package type");
                continue 'pinging_loop;
            },
        };
        let ping_ms = send_time.elapsed().as_secs_f64() * 1000.;
        println!(
            "{} bytes from {}: icmp_seq={} ttl={} time={:.3} ms",
            response.response_size_bytes, address.ip(), packet.sequence(), response.ttl, ping_ms
        );
    }
}
