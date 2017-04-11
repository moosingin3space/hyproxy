extern crate hyper;
extern crate futures;
#[macro_use]
extern crate log;
extern crate env_logger;
#[macro_use]
extern crate serde_derive;
extern crate toml;
#[macro_use]
extern crate error_chain;
extern crate tokio_core;
extern crate tokio_signal;
extern crate tokio_tls;
extern crate tokio_service;
extern crate native_tls;
extern crate chrono;
extern crate regex;

mod config;
mod errors;
mod proxy;
mod tlsclient;

use std::io;
use std::io::{Read, Write};
use std::env;
use std::fs::File;
use std::net::SocketAddr;

use hyper::server::Http;

use futures::{Future, Stream};

use tokio_core::reactor::Core;
use tokio_core::io::Io;
use tokio_core::net::TcpListener;

use tokio_tls::TlsAcceptorExt;

use native_tls::{Pkcs12, TlsAcceptor, TlsConnector};

static CONFIG_FILE_NAME: &'static str = "Hyproxy.toml";

macro_rules! eprint {
    ($($arg:tt)*) => ($crate::io::stdout().write_fmt(format_args!($($arg)*)).unwrap());
}

macro_rules! eprintln {
    () => (eprint!("\n"));
    ($fmt:expr) => (eprint!(concat!($fmt, "\n")));
    ($fmt:expr, $($arg:tt)*) => (eprint!(concat!($fmt, "\n"), $($arg)*));
}

fn get_config() -> errors::Result<config::Config> {
    let mut cwd = env::current_dir()?;
    cwd.push(CONFIG_FILE_NAME);

    let mut cfg_file = File::open(cwd)?;
    let mut contents = String::new();
    cfg_file.read_to_string(&mut contents)?;

    let cfg: config::Config = toml::from_str(&contents)?;
    Ok(cfg)
}

fn make_regex_string<S: AsRef<str>>(prefix: S) -> String {
    let mut pattern = String::from(r"^");
    pattern.push_str(prefix.as_ref());
    pattern.push_str(r"((?P<site_url>/(.*))|\z)");
    pattern
}

fn run() -> errors::Result<()> {
    let mut core = Core::new()?;
    let handle = core.handle();

    env_logger::init()?;
    let config = get_config()?;

    // Verify that the paths are valid
    for (prefix, _) in config.paths.iter() {
        if let Some(ch) = prefix.chars().next() {
            if ch != '/' {
                bail!(errors::ErrorKind::InvalidRoute(prefix.clone()));
            }
        }
        if let Some(ch) = prefix.chars().rev().next() {
            if ch == '/' {
                bail!(errors::ErrorKind::InvalidRoute(prefix.clone()));
            }
        }
        if prefix.len() == 0 {
            bail!(errors::ErrorKind::InvalidRoute(prefix.clone()));
        }
    }

    // Make a Routes object
    let routes = proxy::Routes {
        routes: config.paths.iter().map(|(prefix, addr)| {
            // Unwrap is used here because the regex compilation process should always work.
            (regex::Regex::new(&make_regex_string(prefix)).unwrap(), addr.clone())
        }).collect(),
        regexes: regex::RegexSet::new(
            config.paths.iter().map(|(prefix, _)| make_regex_string(prefix)))?,
    };

    let acceptor = if let (Some(tls_key), Some(tls_password)) = (config.general.tls_key, config.general.tls_password) {
        let mut file = File::open(&tls_key)?;
        let mut file_content = Vec::new();
        file.read_to_end(&mut file_content)?;
        let pkcs12 = Pkcs12::from_der(&file_content, &tls_password)?;
        Some(TlsAcceptor::builder(pkcs12)?.build()?)
    } else {
        None
    };

    let addr : SocketAddr = config.general.listen_addr.parse()?;
    let sock = TcpListener::bind(&addr, &handle)?;
    let client = hyper::Client::new(&handle);
    let tls_connector = TlsConnector::builder()?.build()?;
    let https_connector = tlsclient::HttpsConnector::new(hyper::client::HttpConnector::new(4, &handle), tls_connector);
    let tls_client = hyper::Client::configure().connector(https_connector).build(&handle);
    let http = Http::new();
    println!("Listening on http{}://{} with 1 thread...", match acceptor { Some(_) => "s", None => "" }, sock.local_addr()?);
    if let Some(acceptor) = acceptor {
        let server = sock.incoming().for_each(|(sock, remote_addr)| {
            let service = proxy::Proxy { routes: routes.clone(), client: client.clone(), tls_client: tls_client.clone() };
            acceptor.accept_async(sock).join(Ok(remote_addr)).and_then(|(sock, remote_addr)| {
                http.bind_connection(&handle, sock, remote_addr, service);
                Ok(())
            }).or_else(|e| { println!("error accepting TLS connection: {}", e); Ok(()) })
        });
        core.run(server)?;
    } else {
        let server = sock.incoming().for_each(|(sock, remote_addr)| {
            let service = proxy::Proxy { routes: routes.clone(), client: client.clone(), tls_client: tls_client.clone() };
            futures::future::ok(remote_addr).and_then(|remote_addr| { http.bind_connection(&handle, sock, remote_addr, service); Ok(()) })
        });
        core.run(server)?;
    };
    Ok(())
}

fn main() {
    if let Err(e) = run() {
        eprintln!("error: {}", e);

        for e in e.iter().skip(1) {
            eprintln!("caused by: {}", e);
        }

        if let Some(backtrace) = e.backtrace() {
            eprintln!("backtrace: {:?}", backtrace);
        }

        ::std::process::exit(1);
    }
}
