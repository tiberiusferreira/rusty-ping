use std::io::{Write};
use std::net::{SocketAddr, ToSocketAddrs};
use std::time::Duration;
mod ping;
use ping::*;
use std::process::exit;
use std::thread::sleep;
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
#[structopt(name = "Simple Ping")]
struct CliOptions {
    /// Set TTL
    #[structopt(short, long, default_value = "64")]
    ttl: u8,

    /// Set timeout, how long to wait for ping responses
    #[structopt(long, default_value = "1000")]
    timeout: u64,

    /// Set time to wait between ping attempts
    #[structopt(long, default_value = "1000")]
    time_between_pings: u64,

    /// Hostname or ip to ping
    #[structopt(short, long)]
    hostname_or_ip: String,
}

fn main() {
    let mut cli_options: CliOptions = CliOptions::from_args();
    let original_hostname = cli_options.hostname_or_ip.clone();
    // this is required for the to_socket_addrs call
    cli_options.hostname_or_ip.push_str(":0");

    let mut pinger = match Pinger::new(cli_options.ttl, IpVersion::V4, Duration::from_millis(cli_options.timeout)) {
        Ok(pinger) => pinger,
        Err(e) => {
            write_red(&format!("Error setting up socket for pinging: {:?}", e));
            exit(1);
        }
    };

    let address = match cli_options.hostname_or_ip.to_socket_addrs() {
        Ok(mut res) => match res.next() {
            None => {
                write_red(&format!("No ip address found for hostname."));
                exit(1);
            }
            Some(addr) => addr,
        },
        Err(e) => {
            write_red(&format!("Error looking up hostname IP: {:?}", e));
            exit(1);
        }
    };

    let mut total_pings_sent: u16 = 0;
    let mut total_pings_lost: u16 = 0;
    write_green(&format!("PINGING {} ({})", original_hostname, address.ip()));
    loop {
        sleep(Duration::from_millis(cli_options.time_between_pings));
        retry_until_ping_sent(&pinger, address, total_pings_sent);
        let send_time = std::time::Instant::now();
        let resp = get_ping_response(&mut pinger, Duration::from_millis(cli_options.timeout), total_pings_sent);

        match resp{
            Err(_) => {
                write_red(&format!("Timeout waiting for correct response on icmp_seq={}!", total_pings_sent));
                total_pings_lost += 1;
            },
            Ok(response) => {
                let ping_ms = send_time.elapsed().as_secs_f64() * 1000.;
                write_green(&format!(
                    "{} bytes from {}: icmp_seq={} ttl={} time={:.3} ms",
                    response.response_size_bytes, address.ip(), total_pings_sent, response.ttl, ping_ms
                ));
            }
        }
        total_pings_sent += 1;
    }
}

fn get_ping_response(pinger: &mut Pinger, timeout: Duration, ping_id: u16) -> Result<PingResponseData, ()>{
    let start_wait_response = std::time::Instant::now();
    while start_wait_response.elapsed().as_millis() < timeout.as_millis(){
        let time_left = timeout.as_secs_f64() - start_wait_response.elapsed().as_secs_f64();
        pinger.set_read_timeout(Duration::from_secs_f64(time_left)).unwrap();
        let response = match pinger.get_ping_response() {
            Ok(resp) => resp,
            Err(_e) => {
                continue;
            }
        };

        match &response.packet{
            PossibleIcmpPackets::ICMPv4EchoPacket(pkt) => {
                // Minus 1 here because we increment the total_pings_sent right after sending it
                if pkt.sequence() != ping_id{
                    write_yellow(&format!("Got response for a previous (timed out) ping. Response was for icmp_seq={}. We are at: icmp_seq={}", pkt.sequence(), ping_id));
                    // total_pings_lost += 1;
                    continue;
                }else{
                    return Ok(response);
                }

            },
            PossibleIcmpPackets::ICMPv4TimeExceededPacketTtlExceeded => {
                write_yellow(&format!("Got response, but was a TTL exceeded"));
                continue;
            },
            PossibleIcmpPackets::ICMPv4TimeExceededPacketFragmentReassemblyTimeExceeded => {
                write_yellow(&format!("Got response, but was a Fragment Reassembly Time Exceeded one"));
                continue;
            },
            PossibleIcmpPackets::InvalidPacket => {
                write_yellow(&format!("Got response, but the packet was invalid, checksum was not valid"));
                continue;
            },
            PossibleIcmpPackets::UnknownPacket => {
                write_yellow(&format!("Got response, but was an unknown package type"));
                continue;
            },
        };
    }
    Err(())
}

fn retry_until_ping_sent(pinger: &Pinger, address: SocketAddr, ping_id: u16){
    loop {
        match pinger.send_ping(address, ping_id) {
            Ok(sent) => {
                write_green(&format!("Sent {} bytes", sent));
                break;
            }
            Err(e) => {
                write_red(&format!("Error sending ping: {:?}", e));
                write_yellow(&format!("Retrying in 1 second."));
                std::thread::sleep(Duration::from_secs(1));
            }
        }
    }
}

use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};
fn write_green(text: &str) {
    let mut stdout = StandardStream::stdout(ColorChoice::Auto);
    let _ = stdout.set_color(ColorSpec::new().set_fg(Some(Color::Green)));
    writeln!(&mut stdout, "{}", text).expect("Error writting to stdout");
}


fn write_yellow(text: &str) {
    let mut stdout = StandardStream::stdout(ColorChoice::Auto);
    let _ = stdout.set_color(ColorSpec::new().set_fg(Some(Color::Yellow)));
    writeln!(&mut stdout, "{}", text).expect("Error writting to stdout");
}

fn write_red(text: &str) {
    let mut stdout = StandardStream::stdout(ColorChoice::Auto);
    let _ = stdout.set_color(ColorSpec::new().set_fg(Some(Color::Red)));
    writeln!(&mut stdout, "{}", text).expect("Error writting to stdout");
}