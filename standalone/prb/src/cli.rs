use crate::configurator;
use crate::wm::wm;
use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(name="prb", version, about="Phala Runtime Bridge Worker Manager", long_about = None)]
pub struct WorkerManagerCliArgs {
    /// Path to the local database
    #[arg(short = 'd', long, env, default_value = "/var/data/prb-wm")]
    pub db_path: String,

    /// Use persisted cache index
    #[arg(short = 'c', long, env)]
    pub use_persisted_cache_index: bool,

    /// Listen address of management interface
    #[arg(short = 'a', long, env, default_value = "0.0.0.0")]
    pub mgmt_listen_address: String,

    /// Listen address of management interface
    #[arg(short = 'p', long, env, default_value_t = 3001)]
    pub mgmt_listen_port: u16,

    /// Enable mDNS broadcast of management interface information
    #[arg(long, env)]
    pub mgmt_disable_mdns: bool,
}

pub async fn start_wm() {
    env_logger::builder()
        .filter_level(log::LevelFilter::Info)
        .format_timestamp_micros()
        .parse_default_env()
        .init();
    wm(WorkerManagerCliArgs::parse()).await
}

#[derive(Parser, Debug)]
#[command(name="prb", version, about="Phala Runtime Bridge Worker Manager", long_about = None)]
pub struct ConfigCliArgs {
    /// Path to the local database
    #[arg(short = 'd', long, env, default_value = "/var/data/prb-wm")]
    pub db_path: String,

    #[command(subcommand)]
    pub(crate) command: ConfigCommands,
}

#[derive(Subcommand, Debug, Clone)]
pub enum ConfigCommands {
    /// Add a pool
    AddPool {
        /// Name of the pool
        #[arg(short, long)]
        name: String,

        /// Pool pid
        #[arg(short, long)]
        pid: u64,

        /// Whether workers belongs to the pool are disabled
        #[arg(short, long, default_value_t = false)]
        disabled: bool,

        /// Whether workers belongs to the pool should be in sync-only mode
        #[arg(short, long, default_value_t = false)]
        sync_only: bool,
    },

    /// Remove a pool,
    RemovePool {
        /// Pool pid
        #[arg(short, long)]
        pid: u64,
    },

    /// Update a pool,
    UpdatePool {
        /// Name of the pool
        #[arg(short, long)]
        name: String,

        /// Pool pid
        #[arg(short, long)]
        pid: u64,

        /// Whether workers belongs to the pool are disabled
        #[arg(short, long, default_value_t = false)]
        disabled: bool,

        /// Whether workers belongs to the pool should be in sync-only mode
        #[arg(short, long, default_value_t = false)]
        sync_only: bool,
    },

    /// Get a pool,
    GetPool {
        /// Pool pid
        #[arg(short, long)]
        pid: u64,
    },

    /// Get all pools,
    GetAllPools,
}

pub async fn start_config() {
    env_logger::builder()
        .filter_level(log::LevelFilter::Info)
        .format_timestamp_micros()
        .parse_default_env()
        .init();
    configurator::cli_main(ConfigCliArgs::parse()).await
}
