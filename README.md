# rtunnel

`rtunnel` is a standalone Rust tool that securely exposes local servers to the internet, acting as an `ngrok` or `localtunnel` equivalent.

It uses a simple control-channel/data-channel architecture with no external dependencies on multiplexing libraries, making it extremely lightweight and robust across proxies.

## Status

Built and verified live. The integration test fully spins up a server, a client, and an arbitrary internal TCP listener, verifying a round-trip connection and bridging from a public listener down to the internal service.

## Installation

```sh
cargo install --path .
```

## Usage

You need a public server (or a local address for testing) and a local machine.

### On your public server
Run the tunnel daemon:
```sh
# Listens on 8080 for public traffic, 9090 for control traffic
rtunnel server --bind 0.0.0.0:8080 --control 0.0.0.0:9090
```

### On your local machine
Run the client, pointing it at your public server's control port and your local app's port:
```sh
rtunnel client --server <public_server_ip>:9090 --local 127.0.0.1:3000
```

Now, anyone connecting to `<public_server_ip>:8080` will be seamlessly bridged to your local app running on `127.0.0.1:3000`!
