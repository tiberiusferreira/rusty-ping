use std::io::{Write};
use std::net::{SocketAddr, ToSocketAddrs};
use std::time::Duration;
mod ping;
use ping::*;
use std::process::exit;
use std::thread::sleep;
use structopt::StructOpt;
use std::sync::atomic::{AtomicU16};
use std::sync::Arc;

#[derive(StructOpt, Debug)]
#[structopt(name = "Simple Ping")]
struct CliOptions {
    /// Set TTL
    #[structopt(short, long, default_value = "64")]
    ttl: u8,

    /// Set timeout, how long to wait for ping responses in milliseconds
    #[structopt(long, default_value = "1000")]
    timeout: u64,

    /// Set time to wait between ping attempts in milliseconds
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
    // These are atomic because they are "shared" with the ctrlc handler
    let total_pings_sent: Arc<AtomicU16> = Arc::new(AtomicU16::new(0));
    let total_pings_received: Arc<AtomicU16> = Arc::new(AtomicU16::new(0));
    let avg_ping: Arc<AtomicU16> = Arc::new(AtomicU16::new(0));
    let handler_sent = total_pings_sent.clone();
    let handler_lost = total_pings_received.clone();
    let handler_avg_ping = avg_ping.clone();
    // Prints statistics when ctrl-c is received
    ctrlc::set_handler(move || {
        let nb_sent: u16 = handler_sent.load(SeqCst);
        let nb_received: u16 = handler_lost.load(SeqCst);
        let avg_ping: u16 = handler_avg_ping.load(SeqCst);
        write_red(&format!(""));
        write_red(&format!("{} packets transmitted. {} packets received in time. {:.1}% packet loss", nb_sent, nb_received, 100.- 100.*(nb_received  as f64/nb_sent as f64)));
        write_red(&format!("Avg Ping: {:.1} ms", avg_ping));
        exit(0);
    }).expect("Error setting Ctrl-C handler");
    write_green(&format!("PINGING {} ({})", original_hostname, address.ip()));
    // Main loop send and wait for ping responses forever
    loop {
        sleep(Duration::from_millis(cli_options.time_between_pings));
        retry_until_ping_sent(&pinger, address, total_pings_sent.load(SeqCst));
        let send_time = std::time::Instant::now();
        let resp = get_ping_response(&mut pinger, Duration::from_millis(cli_options.timeout), total_pings_sent.load(SeqCst));
        match resp{
            Err(_) => {
                write_red(&format!("Timeout waiting for correct response on icmp_seq={}!", total_pings_sent.load(SeqCst)));
            },
            Ok(response) => {
                total_pings_received.fetch_add(1, SeqCst);
                let curr_avg: f64 = avg_ping.load(SeqCst) as f64;
                let nb_received: f64 = total_pings_received.load(SeqCst) as f64;
                let ping_ms = send_time.elapsed().as_secs_f64() * 1000.;
                let new_avg = curr_avg + (ping_ms - curr_avg)/nb_received;
                avg_ping.store(new_avg as u16, SeqCst);
                write_green(&format!(
                    "{} bytes from {}: icmp_seq={} ttl={} time={:.3} ms",
                    response.response_size_bytes, address.ip(), total_pings_sent.load(SeqCst), response.ttl, ping_ms
                ));
            }
        }
        total_pings_sent.fetch_add(1, SeqCst);
    }
}

/// Waits for a ping response with sequence number equals to passed ping_id, reporting errors to
/// stdout or return None on timeout
fn get_ping_response(pinger: &mut Pinger, timeout: Duration, ping_id: u16) -> Result<PingResponseData, ()>{
    let start_wait_response = std::time::Instant::now();
    while start_wait_response.elapsed().as_millis() < timeout.as_millis(){
        let time_left = timeout.as_secs_f64() - start_wait_response.elapsed().as_secs_f64();
        pinger.set_read_timeout(Duration::from_secs_f64(time_left)).expect("Error setting socket timeout!");
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

/// Keeps retrying until succeeds
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
use std::sync::atomic::Ordering::SeqCst;

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