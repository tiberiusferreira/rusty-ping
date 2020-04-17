use etherparse::{InternetSlice, SlicedPacket};
use socket2::{Domain, Protocol, Socket, Type};
use std::io::{Read, Write, Error};
use std::net::{SocketAddr, SocketAddrV4, ToSocketAddrs};
use std::time::Duration;
mod ping;
use internet_checksum::checksum;
use ping::*;
use std::thread::sleep;
use std::path::PathBuf;
use structopt::StructOpt;
use std::process::exit;

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

    // this is required for the to_socket_addrs call
    opt.hostname_or_ip.push_str(":0");

    let pinger = match Pinger::new(opt.ttl, IpVersion::V4, Duration::from_millis(opt.timeout)){
        Ok(pinger) => pinger,
        Err(e) => {
            println!("Error setting up socket for pinging: {:?}", e);
            exit(1);
        },
    };


    let address = match opt.hostname_or_ip.to_socket_addrs(){
        Ok(mut res) => {
            match res.next(){
                None => {
                    println!("No ip address found for hostname.");
                    exit(1);
                },
                Some(addr) => {addr},
            }
        },
        Err(e) => {
            println!("Error looking up hostname IP: {:?}", e);
            exit(1);
        },
    };


    loop{
        let send_time = std::time::Instant::now();
        println!("Sending ping");
        pinger.send_ping(address);
        println!("Getting resp");
        pinger.get_ping_response().unwrap();
        println!("{:?}", send_time.elapsed().as_millis());
        sleep(Duration::from_secs(1));
    }
}
