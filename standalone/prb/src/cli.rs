use crate::wm::wm;
use clap::Parser;

#[derive(Parser, Debug)]
#[command(name="prb", version, about="Phala Runtime Bridge Worker Manager", long_about = None)]
pub struct WorkerManagerCliArgs {
    /// Path to the local database
    #[arg(short = 'd', long, env, default_value = "/var/data/prb-wm")]
    db_path: String,

    /// Listen address of management interface
    #[arg(short = 'a', long, env, default_value = "0.0.0.0")]
    mgmt_listen_address: String,

    /// Listen address of management interface
    #[arg(short = 'p', long, env, default_value_t = 3001)]
    mgmt_listen_port: u16,

    /// Enable mDNS broadcast of management interface information
    #[arg(long, env)]
    mgmt_disable_mdns: bool,
}

pub async fn start_wm() {
    env_logger::builder()
        .filter_level(log::LevelFilter::Info)
        .format_timestamp_micros()
        .parse_default_env()
        .init();
    wm(WorkerManagerCliArgs::parse()).await
}
