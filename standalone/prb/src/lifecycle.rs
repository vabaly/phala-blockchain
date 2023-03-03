use crate::db::{get_all_workers, WrappedDb};
use crate::wm::{WorkerManagerCommandTx, WrappedWorkerManagerContext};
use log::{debug, info};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::task::JoinHandle;

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
        let a = get_all_workers(inv_db.clone()).unwrap();
        info!("{:?}", a);
        // info!("Loading workers from database...");
        Arc::new(RwLock::new(lm))
    }
}

pub async fn set_lifecycle_manager(
    tx: WorkerManagerCommandTx,
    ctx: WrappedWorkerManagerContext,
    inv_db: WrappedDb,
    ci_db: WrappedDb,
) {
    let tx = tx.clone();

    let ctx_move = ctx.clone();
    let handle = tokio::spawn(async move {
        let lm = WorkerLifecycleManager::create(tx.clone(), ctx_move.clone(), inv_db, ci_db);
        let mut ctx = ctx_move.write().await;
        ctx.current_lifecycle_manager = Some(lm.clone());
        drop(ctx);
        spawn_lifecycle_tasks(tx.clone(), ctx_move.clone(), lm.clone()).await;
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
    // let t = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
    // let t = t.into_iter();
    // let t = t.map(|i| tokio::spawn(async move { info!("{}", i) }));
    // let t = t.collect::<Vec<JoinHandle<()>>>();
    let mut lm = lm.write().await;
    // lm.handles_count = t.len() as u16;
    // lm.handles = Some(t);
    drop(lm);
}
