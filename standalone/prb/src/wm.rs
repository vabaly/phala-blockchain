use std::sync::Arc;

use anyhow::{anyhow, Result};
use futures::future::try_join_all;
use lazy_static::lazy_static;
use log::{debug, info};
use tokio::sync::oneshot;
use tokio::sync::{mpsc, Mutex, RwLock};
use tokio::task::JoinHandle;

use crate::cli::WorkerManagerCliArgs;
use crate::datasource::{setup_data_source_manager, WrappedDataSourceManager};
use crate::db::{setup_cache_index_db, setup_inventory_db, Worker, WrappedDb};
use crate::lifecycle::{WorkerLifecycleManager, WrappedWorkerLifecycleManager};
use crate::utils::join_handles;

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
    // pub main_tx: WorkerManagerCommandTx,
    pub current_lifecycle_manager_join_handle: Option<JoinHandle<()>>,
    pub current_lifecycle_manager: Option<WrappedWorkerLifecycleManager>,
}

pub type WrappedWorkerManagerContext = Arc<RwLock<WorkerManagerContext>>;

pub enum WorkerManagerMessage {
    Ok,

    LifecycleManagerStarted,
    ShouldBreakMessageLoop,
    ShouldResetLifecycleManager,

    ShouldStartWorkerLifecycle(Worker),
}

pub type WrappedReloadTx = mpsc::Sender<()>;

lazy_static! {
    static ref WM_CTX: WrappedWorkerManagerContext = Arc::new(RwLock::new(WorkerManagerContext {
        initialized: false,
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
    do_send_to_main_channel(main_tx, message, Some(response_tx)).await?;
    let res = response_rx.await?;
    Ok(res)
}

pub async fn set_lifecycle_manager(
    inv_db: WrappedDb,
    ci_db: WrappedDb,
    dsm: WrappedDataSourceManager,
    reload_tx: WrappedReloadTx,
) {
    let (tx, rx) = mpsc::unbounded_channel::<WorkerManagerCommand>();

    let ctx_move = WM_CTX.clone();
    let lm = WorkerLifecycleManager::create(tx.clone(), ctx_move.clone(), inv_db, ci_db);

    let mut ctx = ctx_move.write().await;
    ctx.current_lifecycle_manager = Some(lm.clone());
    drop(ctx);

    join_handles(vec![
        tokio::spawn(message_loop(tx.clone(), rx, reload_tx)),
        tokio::spawn(WorkerLifecycleManager::spawn_lifecycle_tasks(
            lm.clone(),
            tx.clone(),
            ctx_move.clone(),
            dsm.clone(),
        )),
    ])
    .await
}

pub async fn wm(args: WorkerManagerCliArgs) {
    info!("Staring prb-wm with {:?}", &args);

    let inv_db = setup_inventory_db(&args.db_path);
    let ci_db = setup_cache_index_db(&args.db_path, args.use_persisted_cache_index);
    let (dsm, ds_handles) = setup_data_source_manager(&args.data_source_config_path)
        .await
        .expect("Initialize data source manager");

    tokio::select! {
        _ = try_join_all(ds_handles) => {}
        _ = async {
            loop {
                let (reload_tx, mut reload_rx) = mpsc::channel::<()>(1);
                let main_handle = set_lifecycle_manager(
                    inv_db.clone(),
                    ci_db.clone(),
                    dsm.clone(),
                    reload_tx.clone(),
                );

                tokio::select! {
                    _ = main_handle => {
                        info!("Task done, exiting!");
                        std::process::exit(0);
                    }
                    _ = reload_rx.recv() => {
                        info!("Reload signal received.");
                    }
                }
            }
        } => {}
    }
}

async fn message_loop(
    tx: WorkerManagerCommandTx,
    mut rx: WorkerManagerCommandRx,
    reload_tx: WrappedReloadTx,
) {
    debug!("message_loop start");
    while let Some(WorkerManagerCommand {
        message,
        response_tx,
    }) = rx.recv().await
    {
        match message {
            WorkerManagerMessage::ShouldBreakMessageLoop => break,
            WorkerManagerMessage::LifecycleManagerStarted => {
                // todo: setup status map
                info!("LifecycleManagerStarted");
            }
            WorkerManagerMessage::ShouldResetLifecycleManager => {
                // todo: do some cleanup
                reload_tx
                    .send(())
                    .await
                    .expect("ShouldResetLifecycleManager");
            }

            WorkerManagerMessage::ShouldStartWorkerLifecycle(w) => {
                // todo: setup status map
                let _ = response_tx.unwrap().send(WorkerManagerMessage::Ok);
            }

            _ => {}
        }
    }
}
