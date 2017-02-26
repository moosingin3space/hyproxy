## Hyproxy: a reverse proxy for HTTP

Hyproxy is an HTTP reverse proxy built on Hyper and Tokio. It aims for high
throughput and low memory consumption. It is meant to demonstrate the usefulness
of Rust's new asynchronous I/O stack, Tokio, for building high-performance
servers.

### This project is under construction! Many features are missing, do not use in production!

Required features for 1.0:

- Detailed logging
- SSL/TLS termination
- Static files support
- Virtual Hosts/SNI
- Better documentation

### Installation

As of now, this project is *not* on crates.io, therefore you will have to build 
it from git:

```sh
$ git clone https://github.com/moosingin3space/hyproxy.git
$ cargo build --release
```

The resulting binary at `target/release/hyproxy` is the Hyproxy executable.

### Running

Hyproxy reads a file `Hyproxy.toml` in the current working directory, which
configures route-to-server mappings. A sample `Hyproxy.toml` file is provided
in this repository, and another sample is provided here:

```toml
[general]
listen_addr = "0.0.0.0:8000"

[paths]
# Proxying to your app's server
"/app" = "http://localhost:2015"

# Proxying to a remote server works too
"/static" = "https://cdn.site.org"
```

This syntax will be extended to support some of the features mentioned above.
