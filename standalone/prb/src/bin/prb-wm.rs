use log::debug;

#[tokio::main]
async fn main() {
    prb::cli::start_wm().await
}
