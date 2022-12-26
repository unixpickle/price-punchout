use std::path::Path;
use std::sync::Arc;

use rusqlite::Connection;
use sha2::Digest;
use tokio::{sync::Mutex, task::spawn_blocking};

#[derive(Clone)]
pub struct Database {
    db: Arc<Mutex<Connection>>,
}

impl Database {
    pub async fn open<P: AsRef<Path>>(path: P) -> anyhow::Result<Database> {
        let path = path.as_ref().to_owned();
        spawn_blocking(move || Database::new_with_conn(Connection::open(path)?)).await?
    }

    #[allow(dead_code)]
    pub async fn open_in_memory() -> anyhow::Result<Database> {
        spawn_blocking(move || Database::new_with_conn(Connection::open_in_memory()?)).await?
    }

    fn new_with_conn(conn: Connection) -> anyhow::Result<Database> {
        create_tables(&conn)?;
        Ok(Database {
            db: Arc::new(Mutex::new(conn)),
        })
    }
}

fn create_tables(conn: &Connection) -> anyhow::Result<()> {
    conn.execute(
        "CREATE TABLE if not exists thumbnails (
            id           INTEGER PRIMARY KEY,
            website      CHAR(32),
            website_id   CHAR(32)
        )",
        (),
    )?;
    Ok(())
}
