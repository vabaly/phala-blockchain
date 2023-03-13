use crate::datasource::WrappedDataSourceManager;
use crate::db::{get_all_workers, Worker, WrappedDb};
use crate::utils::join_handles;
use crate::wm::{
    send_to_main_channel, send_to_main_channel_and_wait_for_response, WorkerManagerCommandTx,
    WorkerManagerMessage, WrappedWorkerManagerContext,
};
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
    pub workers: Vec<Worker>,
}
pub type WrappedWorkerLifecycleManager = Arc<WorkerLifecycleManager>;

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
        let workers =
            get_all_workers(inv_db.clone()).expect("Failed to load workers from local database");
        let count = workers.len();
        let workers = workers
            .into_iter()
            .filter(|w| w.enabled)
            .collect::<Vec<_>>();
        let count_enabled = workers.len();
        if count_enabled == 0 {
            info!("Got {} worker(s) from local database.", count);
            panic!("There are no worker enabled!");
        }
        debug!(
            "Got workers:\n{}",
            serde_json::to_string_pretty(&workers).unwrap()
        );
        info!(
            "Starting lifecycle for {} of {} worker(s).",
            count_enabled, count
        );
        let lm = Self {
            main_tx,
            main_ctx,
            should_stop: false,
            handles_count: 0,
            inv_db: inv_db.clone(),
            ci_db: ci_db.clone(),
            workers,
        };
        Arc::new(lm)
    }

    pub async fn spawn_lifecycle_tasks(
        lm: WrappedWorkerLifecycleManager,
        tx: WorkerManagerCommandTx,
        ctx: WrappedWorkerManagerContext,
        dsm: WrappedDataSourceManager,
    ) {
        debug!("spawn_lifecycle_threads start");
        let workers = lm.workers.clone();
        join_handles(
            workers
                .into_iter()
                .map(|w| {
                    tokio::spawn(start_worker_lifecycle(
                        w,
                        tx.clone(),
                        ctx.clone(),
                        dsm.clone(),
                    ))
                })
                .collect(),
        )
        .await;
        send_to_main_channel(tx.clone(), WorkerManagerMessage::ShouldBreakMessageLoop)
            .await
            .expect("spawn_lifecycle_tasks -> ShouldBreakMessageLoop");
    }
}

async fn start_worker_lifecycle(
    worker: Worker,
    tx: WorkerManagerCommandTx,
    ctx: WrappedWorkerManagerContext,
    dsm: WrappedDataSourceManager,
) {
    let _ = send_to_main_channel_and_wait_for_response(
        tx.clone(),
        WorkerManagerMessage::ShouldStartWorkerLifecycle(worker.clone()),
    )
    .await
    .expect(format!("Failed to start worker {}", &worker.name).as_str());
    loop {}
}
