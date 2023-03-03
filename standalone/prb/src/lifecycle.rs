use crate::db::{get_all_workers, WrappedDb};
use crate::utils::join_handles;
use crate::wm::{
    send_to_main_channel, WorkerManagerCommandTx, WorkerManagerMessage, WrappedWorkerManagerContext,
};
use log::{debug, info};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tokio::task::JoinHandle;
use tokio::time::sleep;

pub struct WorkerLifecycleManager {
    pub main_tx: WorkerManagerCommandTx,
    pub main_ctx: WrappedWorkerManagerContext,
    pub should_stop: bool,
    pub handles_count: u16,
    pub inv_db: WrappedDb,
    pub ci_db: WrappedDb,
}
pub type WrappedWorkerLifecycleManager = Arc<RwLock<WorkerLifecycleManager>>;

pub struct WorkerContext {
    pub id: String,
    pub handle: Option<JoinHandle<()>>,
}

pub type WorkerContextMap = HashMap<String, WorkerContext>; // HashMap<UuidString, WorkerContext>
pub type WrappedWorkerContextMap = Arc<RwLock<WorkerContextMap>>;

impl WorkerLifecycleManager {
    pub fn create(
        main_tx: WorkerManagerCommandTx,
        main_ctx: WrappedWorkerManagerContext,
        inv_db: WrappedDb,
        ci_db: WrappedDb,
    ) -> WrappedWorkerLifecycleManager {
        let lm = Self {
            main_tx,
            main_ctx,
            should_stop: false,
            handles_count: 0,
            inv_db: inv_db.clone(),
            ci_db: ci_db.clone(),
        };
        // TODO: read database
        // let a = get_all_workers(inv_db.clone()).unwrap();
        // info!("{:?}", a);
        // info!("Loading workers from database...");
        Arc::new(RwLock::new(lm))
    }
}

pub async fn spawn_lifecycle_tasks(
    tx: WorkerManagerCommandTx,
    ctx: WrappedWorkerManagerContext,
    lm: WrappedWorkerLifecycleManager,
) {
    debug!("spawn_lifecycle_threads start");
    let tx_move = tx.clone();
    join_handles(vec![tokio::spawn(async move {
        sleep(Duration::from_secs(3)).await;
    })])
    .await;
    send_to_main_channel(
        tx_move.clone(),
        WorkerManagerMessage::ShouldBreakMessageLoop,
    )
    .await
    .expect("TODO: panic message");
}
