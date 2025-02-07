# Rusty Ping


![CLI Screenshot](./screenshots/main.png "CLI Screenshot")

## What is it?

This was initially a small Ping CLI application for MacOS and Linux implemented in Rust for the Cloudflare internship, but now it is just a fun side project. It accepts a hostname or an IP address as its argument, then send ICMP "echo requests" in a loop to the target while receiving "echo reply" messages.

It displays when a ping request, times out:
![Timeout](./screenshots/timeout.png "Timeout")

When the response is from a past ping request (Expired)
![Expired](./screenshots/expired_response.png "Expired")

When the response is a TTL expired one (but it still waits until timeout for a correct response, just in case)
![TTL exceeded](./screenshots/ttl_exceeded.png "TTL exceeded")

When the response is correct
![Correct Response](./screenshots/correct.png
 "Correct Response")

Uppon receiving SIGINT (Control + C) displays pings statistics
![Packet Statistics](./screenshots/stats.png
 "Statistics")


## Options

It only requires the `--hostname-or-ip` flag, which should be a hostname or ip. 

Example: `sudo cargo run -- --hostname-or-ip 104.17.175.85` or `sudo cargo run -- --hostname-or-ip cloudflare.com`

The optional flags are:
```--time-between-pings``` which specifies the time to wait between a timeout or sucessful response and a new ping request. Defaults to 1000ms.

```--timeout``` sets the maximum amount of time to wait for a ping response. Defaults to 1000ms.

```--ttl``` sets the TTL of ping requests. Defaults to 64.




