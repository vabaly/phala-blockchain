use crate::wm::{WorkerManagerCommandTx, WrappedWorkerManagerContext};
use log::{debug, info};
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::task::JoinHandle;

pub struct WorkerLifecycleManager {
    pub main_tx: WorkerManagerCommandTx,
    pub main_ctx: WrappedWorkerManagerContext,
    pub should_stop: bool,
    pub handles: Option<Vec<JoinHandle<()>>>,
}
pub type WrappedWorkerLifecycleManager = Arc<RwLock<WorkerLifecycleManager>>;

impl WorkerLifecycleManager {
    pub fn create(
        main_tx: WorkerManagerCommandTx,
        main_ctx: WrappedWorkerManagerContext,
    ) -> WrappedWorkerLifecycleManager {
        let lm = Self {
            main_tx,
            main_ctx,
            should_stop: false,
            handles: None,
        };
        info!("Loading database...");
        Arc::new(RwLock::new(lm))
    }
}

pub async fn set_lifecycle_manager(tx: WorkerManagerCommandTx, ctx: WrappedWorkerManagerContext) {
    let tx = tx.clone();

    // todo: read database

    let ctx_move = ctx.clone();
    let handle = tokio::spawn(async move {
        let lm = WorkerLifecycleManager::create(tx.clone(), ctx_move.clone());
        let mut ctx = ctx_move.write().await;
        ctx.current_lifecycle_manager = Some(lm.clone());
        drop(ctx);
        spawn_lifecycle_threads(tx.clone(), ctx_move.clone(), lm.clone()).await;
    });

    let ctx = ctx.clone();
    let mut ctx = ctx.write().await;
    ctx.current_lifecycle_manager_join_handle = Some(handle);
    drop(ctx);
}

pub async fn spawn_lifecycle_tasks(
    tx: WorkerManagerCommandTx,
    ctx: WrappedWorkerManagerContext,
    lm: WrappedWorkerLifecycleManager,
) {
    debug!("spawn_lifecycle_threads start");
    let t = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
    let t = t.into_iter();
    let t = t.map(|i| tokio::spawn(async move { info!("{}", i) }));
    let t = t.collect::<Vec<JoinHandle<()>>>();
}
