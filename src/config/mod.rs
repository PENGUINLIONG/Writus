use std::collections::HashMap;

pub mod v1;

const DEFAULT_CONFIG_FILE: &str = "Writus.toml";

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

    /// Static pages.
    pub static_pages: Option<HashMap<String, String>>,
}
impl WritusConfig{
    pub fn load() -> WritusConfig {
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
            Path::from(p)
        };
        let mut config = String::new();
        match ::std::fs::File::open(path) {
            Ok(mut file) => match file.read_to_string(&mut config) {
                Ok(content) => content,
                Err(_) => panic!("Unable to read config file.")
            },
            Err(_) => panic!("Unable to open config file."),
        };
        // Once the config file is read successfully, change the current
        // directory to where the config file is.
        ::std::env::set_current_dir(path.parent().unwrap());
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
