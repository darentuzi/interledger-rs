use clap::{crate_version, App, AppSettings, Arg, ArgMatches, SubCommand};
use config::{Config, ConfigError, FileFormat, Source, Value};
use lazy_static::lazy_static;
use libc::{c_int, isatty};
use std::ffi::{OsStr, OsString};
use std::io::Read;
use std::vec::Vec;
use tokio;

use interledger_settlement_engines::engines::ethereum_ledger::{
    run_ethereum_engine, EthereumLedgerOpt,
};

lazy_static! {
    pub static ref CONFIG_HELP: String = get_config_help();
}

pub fn main() {
    env_logger::init();

    let mut app = App::new("interledger-settlement-engines")
        .about("Interledger Settlement Engines CLI")
        .version(crate_version!())
        .setting(AppSettings::SubcommandsNegateReqs)
        .after_help("")
        .subcommands(vec![
            SubCommand::with_name("ethereum-ledger")
                .about("Ethereum settlement engine which performs ledger (layer 1) transactions")
                .setting(AppSettings::SubcommandsNegateReqs)
                .args(&[
                    Arg::with_name("config")
                        .takes_value(true)
                        .index(1)
                        .help(&CONFIG_HELP),
                    Arg::with_name("http_address")
                        .long("http_address")
                        .takes_value(true)
                        .default_value("127.0.0.1:3000")
                        .help("Port to listen for settlement requests on"),
                    Arg::with_name("key")
                        .long("key")
                        .takes_value(true)
                        .required(true)
                        .help("private key for settlement account"),
                    Arg::with_name("ethereum_endpoint")
                        .long("ethereum_endpoint")
                        .takes_value(true)
                        .default_value("http://127.0.0.1:8545")
                        .help("Ethereum node endpoint. For example, the address of `ganache`"),
                    Arg::with_name("token_address")
                        .long("token_address")
                        .takes_value(true)
                        .default_value("")
                        .help("The address of the ERC20 token to be used for settlement (defaults to sending ETH if no token address is provided)"),
                    Arg::with_name("connector_url")
                        .long("connector_url")
                        .takes_value(true)
                        .help("Connector Settlement API endpoint")
                        .default_value("http://127.0.0.1:7771"),
                    Arg::with_name("redis_uri")
                        .long("redis_uri")
                        .takes_value(true)
                        .default_value("redis://127.0.0.1:6379")
                        .help("Redis database to add the account to"),
                    Arg::with_name("chain_id")
                        .long("chain_id")
                        .takes_value(true)
                        .default_value("1")
                        .help("The chain id so that the signer calculates the v value of the sig appropriately"),
                    Arg::with_name("confirmations")
                        .long("confirmations")
                        .takes_value(true)
                        .default_value("6")
                        .help("The number of confirmations the engine will wait for a transaction's inclusion before it notifies the node of its success"),
                    Arg::with_name("asset_scale")
                        .long("asset_scale")
                        .takes_value(true)
                        .default_value("18")
                        .help("The asset scale you want to use for your payments"),
                    Arg::with_name("poll_frequency")
                        .long("poll_frequency")
                        .takes_value(true)
                        .default_value("5000")
                        .help("The frequency in milliseconds at which the engine will check the blockchain about the confirmation status of a tx"),
                    Arg::with_name("watch_incoming")
                        .long("watch_incoming")
                        .default_value("true")
                        .help("Launch a blockchain watcher that listens for incoming transactions and notifies the connector upon sufficient confirmations"),
                ])
        ]);

    let mut config = get_env_config("ilp");
    if let Ok((path, config_file)) = precheck_arguments(app.clone()) {
        if !is_fd_tty(0) {
            merge_std_in(&mut config);
        }
        if let Some(ref config_path) = config_file {
            merge_config_file(config_path, &mut config);
        }
        set_app_env(&config, &mut app, &path, path.len());
    }
    let matches = app.clone().get_matches();
    match matches.subcommand() {
        ("ethereum-ledger", Some(ethereum_ledger_matches)) => {
            merge_args(&mut config, &ethereum_ledger_matches);
            tokio::run(run_ethereum_engine(get_or_error(
                config.try_into::<EthereumLedgerOpt>(),
            )));
        }
        ("", None) => app.print_help().unwrap(),
        _ => unreachable!(),
    }
}

// returns (subcommand paths, stdin flag, config path)
fn precheck_arguments(mut app: App) -> Result<(Vec<String>, Option<String>), ()> {
    // not to cause `required fields error`.
    reset_required(&mut app);
    let matches = app.get_matches_safe();
    if matches.is_err() {
        // if app could not get any appropriate match, just return not to show help etc.
        return Err(());
    }
    let matches = &matches.unwrap();
    let mut path = Vec::<String>::new();
    let subcommand = get_deepest_command(matches, &mut path);
    let mut config_path: Option<String> = None;
    if let Some(config_path_arg) = subcommand.value_of("config") {
        config_path = Some(config_path_arg.to_string());
    };
    Ok((path, config_path))
}

fn merge_config_file(config_path: &str, config: &mut Config) {
    let file_config = config::File::with_name(config_path);
    let file_config = file_config.collect().unwrap();
    // if the key is not defined in the given config already, set it to the config
    // because the original values override the ones from the config file
    for (k, v) in file_config {
        if config.get_str(&k).is_err() {
            config.set(&k, v).unwrap();
        }
    }
}

fn merge_std_in(config: &mut Config) {
    let stdin = std::io::stdin();
    let mut stdin_lock = stdin.lock();
    let mut buf = Vec::new();
    if let Ok(_read) = stdin_lock.read_to_end(&mut buf) {
        if let Ok(buf_str) = String::from_utf8(buf) {
            let mut config_hash = None;
            // JSON is always used because the other code already depends on it
            if let Ok(hash_map) = FileFormat::Json.parse(None, &buf_str) {
                config_hash = Some(hash_map);
            }
            if cfg!(feature = "yaml") {
                if let Ok(hash_map) = FileFormat::Yaml.parse(None, &buf_str) {
                    config_hash = Some(hash_map);
                }
            }
            if cfg!(feature = "toml") {
                if let Ok(hash_map) = FileFormat::Toml.parse(None, &buf_str) {
                    config_hash = Some(hash_map);
                }
            }
            if cfg!(feature = "hjson") {
                if let Ok(hash_map) = FileFormat::Hjson.parse(None, &buf_str) {
                    config_hash = Some(hash_map);
                }
            }
            if cfg!(feature = "ini") {
                if let Ok(hash_map) = FileFormat::Ini.parse(None, &buf_str) {
                    config_hash = Some(hash_map);
                }
            }
            if let Some(config_hash) = config_hash {
                // if the key is not defined in the given config already, set it to the config
                // because the original values override the ones from the stdin
                for (k, v) in config_hash {
                    if config.get_str(&k).is_err() {
                        config.set(&k, v).unwrap();
                    }
                }
            }
        }
    }
}

fn merge_args(config: &mut Config, matches: &ArgMatches) {
    for (key, value) in &matches.args {
        if config.get_str(key).is_ok() {
            continue;
        }
        if value.vals.is_empty() {
            // flag
            config.set(key, Value::new(None, true)).unwrap();
        } else {
            // value
            config
                .set(key, Value::new(None, value.vals[0].to_str().unwrap()))
                .unwrap();
        }
    }
}

// retrieve Config from a certain prefix
// if the prefix is `ilp`, `address` is resolved to `ilp_address`
fn get_env_config(prefix: &str) -> Config {
    let mut config = Config::new();
    config
        .merge(config::Environment::with_prefix(prefix))
        .unwrap();

    if prefix.to_lowercase() == "ilp" {
        if let Ok(value) = config.get_str("address") {
            config.set("ilp_address", value).unwrap();
        }
    }

    config
}

// This sets the Config values which contains environment variables, config file settings, and STDIN
// settings, into each option's env value which is used when Parser parses the arguments. If this
// value is set, the Parser reads the value from it and doesn't warn even if the argument is not
// given from CLI.
// Usually `env` fn is used when creating `App` but this function automatically fills it so
// we don't need to call `env` fn manually.
fn set_app_env(env_config: &Config, app: &mut App, path: &[String], depth: usize) {
    if depth == 1 {
        for item in &mut app.p.opts {
            if let Ok(value) = env_config.get_str(&item.b.name.to_lowercase()) {
                item.v.env = Some((&OsStr::new(item.b.name), Some(OsString::from(value))));
            }
        }
        return;
    }
    for subcommand in &mut app.p.subcommands {
        if subcommand.get_name() == path[path.len() - depth] {
            set_app_env(env_config, subcommand, path, depth - 1);
        }
    }
}

fn get_deepest_command<'a>(matches: &'a ArgMatches, path: &mut Vec<String>) -> &'a ArgMatches<'a> {
    let (name, subcommand_matches) = matches.subcommand();
    path.push(name.to_string());
    if let Some(matches) = subcommand_matches {
        return get_deepest_command(matches, path);
    }
    matches
}

fn reset_required(app: &mut App) {
    app.p.required.clear();
    for subcommand in &mut app.p.subcommands {
        reset_required(subcommand);
    }
}

fn get_or_error<T>(item: Result<T, ConfigError>) -> T {
    match item {
        Ok(item) => item,
        Err(error) => {
            match error {
                ConfigError::Message(message) => eprintln!("Configuration error: {:?}", message),
                _ => eprintln!("{:?}", error),
            };
            std::process::exit(1);
        }
    }
}

fn is_fd_tty(file_descriptor: c_int) -> bool {
    let result: c_int;
    unsafe {
        result = isatty(file_descriptor);
    }
    result == 1
}

fn get_config_help() -> String {
    let mut formats = Vec::new();
    // JSON is always supported because the crate is used already
    formats.push("JSON");
    if cfg!(feature = "yaml") {
        formats.push("YAML");
    }
    if cfg!(feature = "toml") {
        formats.push("TOML");
    }
    if cfg!(feature = "hjson") {
        formats.push("HJSON");
    }
    if cfg!(feature = "ini") {
        formats.push("INI");
    }
    format!(
        "Name of config file (in a format of: {})",
        formats.join(", ")
    )
}
