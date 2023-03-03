use crate::cli::WorkerManagerCliArgs;
use crate::db::{setup_cache_index_db, setup_inventory_db, WrappedDb};
use crate::lifecycle::{set_lifecycle_manager, WrappedWorkerLifecycleManager};
use crate::utils::join_handles;
use anyhow::{anyhow, Result};
use futures::FutureExt;
use lazy_static::lazy_static;
use log::{debug, error, info};
use std::sync::Arc;
use tokio::sync::oneshot;
use tokio::sync::{mpsc, Mutex, RwLock};
use tokio::task::JoinHandle;

pub type GlobalWorkerManagerCommandChannelPair = (
    mpsc::UnboundedSender<WorkerManagerCommand>,
    Arc<Mutex<mpsc::UnboundedReceiver<WorkerManagerCommand>>>,
);

pub type WorkerManagerCommandTx = mpsc::UnboundedSender<WorkerManagerCommand>;
pub type WorkerManagerCommandRx = mpsc::UnboundedReceiver<WorkerManagerCommand>;

pub type WorkerManagerResponseTx = oneshot::Sender<WorkerManagerMessage>;
pub type WorkerManagerResponseRx = oneshot::Receiver<WorkerManagerMessage>;

pub struct WorkerManagerCommand {
    message: WorkerManagerMessage,
    response_tx: Option<WorkerManagerResponseTx>,
}

pub struct WorkerManagerContext {
    pub initialized: bool,
    pub main_tx: WorkerManagerCommandTx,
    pub current_lifecycle_manager_join_handle: Option<JoinHandle<()>>,
    pub current_lifecycle_manager: Option<WrappedWorkerLifecycleManager>,
}
pub type WrappedWorkerManagerContext = Arc<RwLock<WorkerManagerContext>>;

pub enum WorkerManagerMessage {
    ShouldSetLifecycleManager(WrappedDb, WrappedDb),
    ShouldResetLifecycleManager(WrappedDb),
}

lazy_static! {
    static ref MAIN_CHANNEL: GlobalWorkerManagerCommandChannelPair = {
        let (tx, rx) = mpsc::unbounded_channel::<WorkerManagerCommand>();
        (tx, Arc::new(Mutex::new(rx)))
    };
    static ref WM_CTX: WrappedWorkerManagerContext = Arc::new(RwLock::new(WorkerManagerContext {
        initialized: false,
        main_tx: MAIN_CHANNEL.0.clone(),
        current_lifecycle_manager_join_handle: None,
        current_lifecycle_manager: None
    }));
}

pub async fn do_send_to_main_channel(
    main_tx: WorkerManagerCommandTx,
    message: WorkerManagerMessage,
    response_tx: Option<WorkerManagerResponseTx>,
) -> Result<()> {
    match main_tx.send(WorkerManagerCommand {
        message,
        response_tx,
    }) {
        Ok(_) => Ok(()),
        Err(e) => Err(anyhow!("Failed to send to main channel! {}", e)),
    }
}
pub async fn send_to_main_channel(
    main_tx: WorkerManagerCommandTx,
    message: WorkerManagerMessage,
) -> Result<()> {
    do_send_to_main_channel(main_tx, message, None).await
}

pub async fn send_to_main_channel_and_wait_for_response(
    main_tx: WorkerManagerCommandTx,
    message: WorkerManagerMessage,
) -> Result<WorkerManagerMessage> {
    let (response_tx, response_rx) = oneshot::channel::<WorkerManagerMessage>();
    match do_send_to_main_channel(main_tx, message, Some(response_tx)).await {
        Ok(_) => match response_rx.await {
            Ok(res) => Ok(res),
            Err(e) => Err(anyhow!(
                "Failed to get response from the oneshot channel! {}",
                e
            )),
        },
        Err(e) => Err(e),
    }
}

pub async fn wm(args: WorkerManagerCliArgs) {
    info!("Staring prb-wm with {:?}", &args);

    let inv_db = setup_inventory_db(&args.db_path);
    let ci_db = setup_cache_index_db(&args.db_path, args.use_persisted_cache_index);

    let main_tx = MAIN_CHANNEL.0.clone();
    let main_rx = MAIN_CHANNEL.1.clone();
    let main_controller_handle = tokio::spawn(main_loop(main_tx.clone(), main_rx));
    let handle = join_handles(vec![main_controller_handle]);

    send_to_main_channel(
        main_tx.clone(),
        WorkerManagerMessage::ShouldSetLifecycleManager(inv_db.clone(), ci_db.clone()),
    )
    .await
    .expect("Failed to send lifecycle set message!");

    handle.await;
}

async fn main_loop(tx: WorkerManagerCommandTx, rx: Arc<Mutex<WorkerManagerCommandRx>>) {
    debug!("main_loop start");
    let mut rx = rx.lock().await;
    while let Some(WorkerManagerCommand {
        message,
        response_tx,
    }) = rx.recv().await
    {
        match message {
            WorkerManagerMessage::ShouldSetLifecycleManager(inv_db, ci_db) => {
                set_lifecycle_manager(tx.clone(), WM_CTX.clone(), inv_db, ci_db).await;
            }
            WorkerManagerMessage::ShouldResetLifecycleManager(_inv_db) => {
                // TODO: hot-reloading
                error!("Not implemented!")
            }
        }
    }
}
