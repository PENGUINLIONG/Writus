// Web services.
extern crate futures;
extern crate hyper;
extern crate rustls;
extern crate tokio_proto;
extern crate tokio_rustls;
extern crate tokio_service;

// Writium.
extern crate writium;
extern crate writium_auth;
extern crate writium_cache;

// Content generation.
extern crate chrono;
extern crate pulldown_cmark;
extern crate walkdir;
#[macro_use]
extern crate serde_derive;
extern crate serde;
#[macro_use]
#[allow(unused_imports)]
extern crate serde_json;
extern crate toml;

// Bin utils.
extern crate ctrlc;
extern crate getopts;
#[macro_use]
extern crate path_buf;

// Logging.
#[macro_use]
extern crate log;
extern crate env_logger;

mod auth;
mod config;
mod model;
pub mod api;
pub mod view;
pub mod service;
pub mod static_page;

use writium::Writium;
use writium::prelude::*;
use hyper::server::Http;
use service::WritiumService;

fn init_logging() {
    use env_logger::LogBuilder;
    use log::{LogLevelFilter, LogRecord};
    let format = |record: &LogRecord| {
        format!("{} {:?} [{}] {}", chrono::Utc::now().to_rfc3339(),
            std::thread::current().id(), record.level(), record.args())
    };
    let mut builder = LogBuilder::new();
    builder.format(format).filter(None, LogLevelFilter::Info);
    if ::std::env::var("RUST_LOG").is_ok() {
       builder.parse(&::std::env::var("RUST_LOG").unwrap());
    }
    if let Err(_) = builder.init() {
        panic!("Initialization failed!");
    };
}

fn load_certs(path: &str) -> Vec<rustls::Certificate> {
    let cert_file = std::fs::File::open(path)
        .expect("Unable to open certificate file");
    let mut reader = std::io::BufReader::new(cert_file);
    rustls::internal::pemfile::certs(&mut reader).unwrap()
}

fn load_private_key(path: &str) -> rustls::PrivateKey {
    use rustls::internal::pemfile::rsa_private_keys;
    let keyfile = std::fs::File::open(path)
        .expect("Unable to open private key file");
    let mut reader = std::io::BufReader::new(keyfile);
    let keys = rsa_private_keys(&mut reader).unwrap();
    assert!(keys.len() == 1);
    keys[0].clone()
}

fn main() {
    init_logging();
    let cfg = ::config::WritusConfig::load();

    let extra = cfg.extra.unwrap();
    let mut writium = Writium::new();
    // Load static pages.
    if let Some(ref static_pages) = cfg.static_pages.as_ref() {
        for (ref name, ref path) in static_pages.iter() {
            info!("Loading static page: {}", path);
            match ::static_page::StaticPage::from_file(name, path) {
                Ok(sp) => writium.bind(sp),
                Err(err) => warn!("Error occured loading static page: {}", err),
            }
        }
    }
    // Load all Writium v1 APIs.
    info!("Loading Writus APIs.");
    let extra = ::config::v1::Extra::from(extra);
    let v1: Namespace = extra.into();
    writium.bind(v1);

    let service = WritiumService::new(writium);
    let shut_down_handle = service.writium();

    ctrlc::set_handler(move || {
        info!("Shutting down Writium");
        let guard = shut_down_handle.clone();
        *guard.write().unwrap() = None;
        info!("All APIs are now released.");
        ::std::process::exit(0);
    }).expect("Unable to handle Ctrl-C signal.");

    let addr = format!("{}:{}", cfg.host_addr.unwrap(), cfg.port.unwrap())
        .parse()
        .unwrap();
    // Decide to use TLS or not.
    if let Some(ref tls) = cfg.tls {
        let mut cfg = rustls::ServerConfig::new();
        cfg.set_single_cert(
            load_certs(&tls.cert_path),
            load_private_key(&tls.key_path)
        );
        let tls = tokio_rustls::proto::Server::new(
            Http::new(),
            std::sync::Arc::new(cfg)
        );
        let server = tokio_proto::TcpServer::new(tls, addr);
        server.serve(::hyper::server::const_service(service));
    } else {
        let server = Http::new()
            .bind(&addr, ::hyper::server::const_service(service)).unwrap();
        server.run().unwrap();
    }
}
