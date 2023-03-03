use crate::cli::ConfigCommands;
use anyhow::{anyhow, Result};
use indradb::{
    Datastore, Edge, Identifier, MemoryDatastore, PropertyValueVertexQuery, RangeVertexQuery,
    RocksdbDatastore, SpecificEdgeQuery, SpecificVertexQuery, ValidationResult, VertexProperties,
    VertexPropertyQuery, VertexQuery,
};
use log::debug;
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::sync::Arc;
use uuid::Uuid;

pub const ID_VERTEX_INVENTORY_WORKER: &str = "iv_w";
pub const ID_VERTEX_INVENTORY_POOL: &str = "iv_p";

pub const ID_EDGE_BELONG_TO: &str = "ie_bt"; // Direction: from worker to pool

pub const ID_PROP_CREATED_AT: &str = "created_at";

pub const ID_PROP_WORKER_NAME: &str = "name";
pub const ID_PROP_WORKER_ENDPOINT: &str = "endpoint";
pub const ID_PROP_WORKER_STAKE: &str = "stake";
pub const ID_PROP_WORKER_ENABLED: &str = "enabled";
pub const ID_PROP_WORKER_SYNC_ONLY: &str = "sync_only";

// Account-related settings moved to trade service
pub const ID_PROP_POOL_NAME: &str = "name";
pub const ID_PROP_POOL_PID: &str = "pid";
pub const ID_PROP_POOL_ENABLED: &str = "enabled";
pub const ID_PROP_POOL_SYNC_ONLY: &str = "sync_only";

pub const ID_VERTEX_INVENTORY_DATA_SOURCE: &str = "iv_ds";

pub type Db = dyn Datastore + Send + Sync + 'static;
pub type WrappedDb = Arc<Db>;

#[derive(Debug, Serialize, Deserialize)]
pub struct Pool {
    pub id: String,
    pub name: String,
    pub pid: u64,
    pub enabled: bool,
    pub sync_only: bool,
}

impl Into<Pool> for VertexProperties {
    fn into(self) -> Pool {
        let mut ret = Pool {
            id: self.vertex.id.to_string(),
            name: "".to_string(),
            pid: 0,
            enabled: true,
            sync_only: false,
        };
        self.props.iter().for_each(|p| match p.name.as_str() {
            ID_PROP_POOL_NAME => {
                ret.name = p.value.as_str().unwrap().to_string();
            }
            ID_PROP_POOL_PID => {
                ret.pid = p.value.as_u64().unwrap();
            }
            ID_PROP_POOL_ENABLED => {
                ret.enabled = p.value.as_bool().unwrap();
            }
            ID_PROP_POOL_SYNC_ONLY => {
                ret.sync_only = p.value.as_bool().unwrap();
            }
            &_ => {}
        });
        ret
    }
}

pub fn setup_inventory_db(db_path: &str) -> WrappedDb {
    let db_path = Path::new(db_path).join("local");
    let db = RocksdbDatastore::new(&db_path, None).expect("Failed to open inventory database.");
    let db = Arc::new(db);
    debug!("Opened inventory database in {:?}", db_path);

    let _iter = vec![
        ID_PROP_WORKER_NAME,
        ID_PROP_WORKER_ENABLED,
        ID_PROP_WORKER_SYNC_ONLY,
        ID_PROP_POOL_NAME,
        ID_PROP_POOL_PID,
        ID_PROP_POOL_ENABLED,
        ID_PROP_POOL_SYNC_ONLY,
    ]
    .iter()
    .for_each(|i| {
        let ii = Identifier::new(*i).unwrap();
        db.index_property(ii)
            .expect(format!("inv_db: Failed to index property {:?}", *i).as_str());
        debug!("inv_db: Indexing property {:?}", *i);
    });

    db
}

pub fn get_pool_by_pid(db: WrappedDb, pid: u64) -> Result<Option<Pool>> {
    let v = get_raw_pool_by_pid(db, pid)?;
    match v {
        Some(v) => Ok(Some(v.clone().into())),
        None => Ok(None),
    }
}

pub fn get_raw_pool_by_pid(db: WrappedDb, pid: u64) -> Result<Option<VertexProperties>> {
    let q = PropertyValueVertexQuery {
        name: Identifier::new(ID_PROP_POOL_PID).unwrap(),
        value: serde_json::Value::Number(serde_json::Number::from(pid)),
    }
    .into();
    let v: Vec<VertexProperties> = db.get_all_vertex_properties(q)?;
    let v = v.get(0);
    match v {
        Some(v) => Ok(Some(v.clone())),
        None => Ok(None),
    }
}

pub fn get_all_pools(db: WrappedDb) -> Result<Vec<Pool>> {
    let v = get_all_raw_pools(db)?;
    let v = v.into_iter().map(|v| v.into()).collect::<Vec<_>>();
    Ok(v)
}

pub fn get_all_raw_pools(db: WrappedDb) -> Result<Vec<VertexProperties>> {
    let q = RangeVertexQuery {
        limit: 4_294_967_294u32,
        t: Some(Identifier::new(ID_VERTEX_INVENTORY_POOL).unwrap()),
        start_id: None,
    }
    .into();
    let v: Vec<VertexProperties> = db.get_all_vertex_properties(q)?;
    Ok(v)
}

pub fn get_all_workers(db: WrappedDb) -> Result<Vec<Vec<Edge>>> {
    // TODO
    Err(anyhow!("Not implemented!"))
}

pub fn add_pool(db: WrappedDb, cmd: ConfigCommands) -> Result<Uuid> {
    match cmd {
        ConfigCommands::AddPool {
            name,
            pid,
            disabled,
            sync_only,
        } => {
            let p = get_pool_by_pid(db.clone(), pid)?;
            match p {
                Some(_v) => Err(anyhow!("Pool already exists!")),
                None => {
                    let id = db.create_vertex_from_type(
                        Identifier::new(ID_VERTEX_INVENTORY_POOL).unwrap(),
                    )?;
                    let uq: VertexQuery = SpecificVertexQuery {
                        ids: vec![id.clone()],
                    }
                    .into();
                    db.set_vertex_properties(
                        VertexPropertyQuery {
                            inner: uq.clone(),
                            name: Identifier::new(ID_PROP_POOL_NAME).unwrap(),
                        },
                        serde_json::Value::String(name),
                    )?;
                    db.set_vertex_properties(
                        VertexPropertyQuery {
                            inner: uq.clone(),
                            name: Identifier::new(ID_PROP_POOL_PID).unwrap(),
                        },
                        serde_json::Value::Number(serde_json::Number::from(pid)),
                    )?;
                    db.set_vertex_properties(
                        VertexPropertyQuery {
                            inner: uq.clone(),
                            name: Identifier::new(ID_PROP_POOL_ENABLED).unwrap(),
                        },
                        serde_json::Value::Bool(!disabled),
                    )?;
                    db.set_vertex_properties(
                        VertexPropertyQuery {
                            inner: uq.clone(),
                            name: Identifier::new(ID_PROP_POOL_SYNC_ONLY).unwrap(),
                        },
                        serde_json::Value::Bool(sync_only),
                    )?;
                    Ok(id)
                }
            }
        }
        _ => Err(anyhow!("Invalid command!")),
    }
}

pub fn update_pool(db: WrappedDb, cmd: ConfigCommands) -> Result<Uuid> {
    match cmd {
        ConfigCommands::UpdatePool {
            name,
            pid,
            disabled,
            sync_only,
        } => {
            let p = get_raw_pool_by_pid(db.clone(), pid)?;
            match p {
                Some(v) => {
                    let id = v.vertex.id;
                    let uq: VertexQuery = SpecificVertexQuery {
                        ids: vec![id.clone()],
                    }
                    .into();
                    db.set_vertex_properties(
                        VertexPropertyQuery {
                            inner: uq.clone(),
                            name: Identifier::new(ID_PROP_POOL_NAME).unwrap(),
                        },
                        serde_json::Value::String(name),
                    )?;
                    db.set_vertex_properties(
                        VertexPropertyQuery {
                            inner: uq.clone(),
                            name: Identifier::new(ID_PROP_POOL_ENABLED).unwrap(),
                        },
                        serde_json::Value::Bool(!disabled),
                    )?;
                    db.set_vertex_properties(
                        VertexPropertyQuery {
                            inner: uq.clone(),
                            name: Identifier::new(ID_PROP_POOL_SYNC_ONLY).unwrap(),
                        },
                        serde_json::Value::Bool(sync_only),
                    )?;
                    Ok(id)
                }
                None => Err(anyhow!("Pool not found!")),
            }
        }
        _ => Err(anyhow!("Invalid command!")),
    }
}

pub fn remove_pool(db: WrappedDb, pid: u64) -> Result<()> {
    let p = get_raw_pool_by_pid(db.clone(), pid)?;
    // todo: check if any worker assigned to the pool
    match p {
        Some(p) => {
            let q = SpecificVertexQuery {
                ids: vec![p.vertex.id],
            }
            .into();
            db.delete_vertices(q)?;
            Ok(())
        }
        None => Err(anyhow!("Pool not found!")),
    }
}

pub fn setup_cache_index_db(db_path: &str, use_persisted_cache_index: bool) -> WrappedDb {
    let db: WrappedDb = if use_persisted_cache_index {
        let db_path = Path::new(db_path).join("index");
        let db = RocksdbDatastore::new(&db_path, None).expect("Failed to open inventory database.");
        debug!("Opened cache index database in {:?}", db_path);
        Arc::new(db)
    } else {
        let db = MemoryDatastore::default();
        debug!("Using in-memory database for cache index");
        Arc::new(db)
    };
    // TODO: setup indexes
    db
}
