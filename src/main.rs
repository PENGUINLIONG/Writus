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
pub mod api;
pub mod service;

use writium::Writium;
use hyper::server::Http;
use service::WritiumService;

const DEFAULT_CONFIG_FILE: &str = "Writium.toml";

#[derive(Deserialize)]
pub struct TlsConfig {
    pub cert_path: String,
    pub key_path: String,
}
#[derive(Deserialize)]
pub struct WritusConfig {
    /// Host server address or domain for HTTP. [default: "127.0.0.1"]
    pub host_addr: Option<String>,
    /// Port number. [default: "8080"]
    pub port: Option<u16>,
    
    /// TLS configurations. TLS is disabled on missing. [default: None]
    pub tls: Option<TlsConfig>,

    /// Extra settings. Some values have default values on missing:
    ///
    /// * `cache_dir` = `./cache`
    /// * `published_dir` = `./published`
    /// * `template_dir` = `./published/template`
    /// * `digests_per_page` = `5`
    pub extra: Option<::toml::Value>,
}
impl WritusConfig{
    fn load() -> WritusConfig {
        use std::io::Read;
        let mut options = ::getopts::Options::new();
        options.optflag("h", "help", "Help information");
        let args: Vec<String> = ::std::env::args().collect();
        
        let matches = match options.parse(&args[1..]) {
            Ok(matches) => matches,
            Err(_) => panic!("Unable to parse args."),
        };
        if matches.opt_present("h") {
            error!("{}", options.usage(&"Usage: writium CONFIG_FILE [options]"));
            ::std::process::exit(0);
        }
        let path = if matches.free.is_empty() {
            info!("No configuration file given. Using default config file: {}",
                DEFAULT_CONFIG_FILE);
            DEFAULT_CONFIG_FILE
        } else {
            let p = &matches.free[0];
            info!("Using config file: {}", p);
            p
        };
        let mut config = String::new();
        match ::std::fs::File::open(path) {
            Ok(mut file) => match file.read_to_string(&mut config) {
                Ok(content) => content,
                Err(_) => panic!("Unable to read config file.")
            },
            Err(_) => panic!("Unable to open config file."),
        };
        match ::toml::from_str::<WritusConfig>(&config) {
            Ok(toml) => toml.insert_default(),
            Err(err) => panic!("Unable to parse Writium config file. {:?}", err),
        }
    }
    fn insert_default(mut self) -> WritusConfig {
        use ::toml::Value as TomlValue;
        use ::toml::value::Table as TomlTable;
        if self.host_addr.is_none() { self.host_addr = Some("127.0.0.1".to_string()) }
        if self.port.is_none() { self.port = Some(8080) }

        if self.extra.is_none() {
            self.extra =
                Some(TomlValue::Table(TomlTable::new()));
        }
        self
    }
}

fn init_logging() {
    use env_logger::LogBuilder;
    use log::{LogLevelFilter, LogRecord};
    let format = |record: &LogRecord| {
        format!("{} {:?} [{}] {}", chrono::Utc::now().to_rfc3339(), std::thread::current().id(), record.level(), record.args())
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
    let cfg = WritusConfig::load();

    let extra = cfg.extra.unwrap();
    let mut writium = Writium::new();
    // Load all Writium v1 APIs.
    writium.bind(api::api_v1(extra.clone()));

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
        let server = Http::new().bind(&addr, ::hyper::server::const_service(service)).unwrap();
        server.run().unwrap();
    }
}
