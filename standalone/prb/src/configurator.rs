use crate::cli::{ConfigCliArgs, ConfigCommands};
use crate::db;
use crate::db::{get_all_pools, get_pool_by_pid, setup_inventory_db};

pub async fn cli_main(args: ConfigCliArgs) {
    let db = setup_inventory_db(&args.db_path);

    match &args.command {
        ConfigCommands::AddPool { pid, .. } => {
            db::add_pool(db.clone(), args.command.clone()).expect("Command failed");
            let p = get_pool_by_pid(db.clone(), *pid).expect("Command failed");
            match p {
                Some(p) => {
                    let p = serde_json::to_string_pretty(&p).unwrap();
                    println!("{}", p);
                }
                None => {}
            }
        }
        ConfigCommands::RemovePool { pid } => {
            db::remove_pool(db.clone(), *pid).expect("Command failed");
        }
        ConfigCommands::UpdatePool { pid, .. } => {
            db::update_pool(db.clone(), args.command.clone()).expect("Command failed");
            let p = get_pool_by_pid(db.clone(), *pid).expect("Command failed");
            match p {
                Some(p) => {
                    let p = serde_json::to_string_pretty(&p).unwrap();
                    println!("{}", p);
                }
                None => {}
            }
        }
        ConfigCommands::GetPool { pid } => {
            let p = get_pool_by_pid(db, *pid).expect("Command failed");
            match p {
                Some(p) => {
                    let p = serde_json::to_string_pretty(&p).unwrap();
                    println!("{}", p);
                }
                None => {}
            }
        }
        ConfigCommands::GetAllPools => {
            let v = get_all_pools(db).expect("Command failed");
            let v = serde_json::to_string_pretty(&v).unwrap();
            println!("{}", v);
        }
        _ => {}
    };
}
