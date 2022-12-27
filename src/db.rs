use std::fmt::Write;
use std::path::Path;
use std::sync::Arc;

use rusqlite::{Connection, Transaction};
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

    // Either insert a new listing, or update the information if the website_id
    // is already present in the database.
    pub async fn insert_or_update(&self, listing: Listing) -> anyhow::Result<()> {
        self.with_db(move |db| {
            let mut tx = db.transaction()?;
            let blob_id = insert_blob(&mut tx, &listing.image_data)?;
            let result: rusqlite::Result<(i64, i64)> = tx.query_row(
                "SELECT id, image_blob FROM listings WHERE website=?1, website_id=?2",
                (&listing.website, &listing.website_id),
                |row| Ok((row.get(0)?, row.get(1)?)),
            );
            match result {
                Ok((id, old_image_blob)) => {
                    tx.execute(
                        "
                            UPDATE listings
                            SET image_blob = ?1,
                                price = ?2,
                                title = ?3,
                                last_seen = unixepoch()
                            WHERE id=?4
                        ",
                        rusqlite::params![blob_id, listing.price, listing.title, id],
                    )?;
                    garbage_collect_blob(&mut tx, old_image_blob)?;
                    insert_categories(&mut tx, id, listing.categories)?;
                }
                Err(rusqlite::Error::QueryReturnedNoRows) => {
                    tx.execute(
                        "
                        INSERT INTO listings (
                            created,
                            last_seen,
                            website,
                            website_id,
                            price,
                            title,
                            image_blob,
                            star_rating,
                            max_stars,
                            num_reviews,
                        ) VALUES (
                            unixepoch(),
                            unixepoch(),
                            ?1,
                            ?2,
                            ?3,
                            ?4,
                            ?5,
                            ?6,
                            ?7,
                            ?8
                        )
                    ",
                        rusqlite::params![
                            listing.website,
                            listing.website_id,
                            listing.price,
                            listing.title,
                            blob_id,
                            listing.star_rating,
                            listing.max_stars,
                            listing.num_reviews,
                        ],
                    )?;
                    let insert_id = tx.last_insert_rowid();
                    insert_categories(&mut tx, insert_id, listing.categories)?;
                }
                x @ Err(_) => {
                    x?;
                }
            }
            Ok(())
        })
        .await
    }

    pub async fn insert_log_message(&self, source: String, message: String) -> anyhow::Result<()> {
        self.with_db(move |db| {
            db.execute(
                "INSERT INTO log (timestamp, source, message) VALUES (unixepoch(), ?1, ?2)",
                (source, message),
            )?;
            Ok(())
        })
        .await
    }

    async fn with_db<
        T: 'static + Send,
        F: 'static + Send + FnOnce(&mut Connection) -> anyhow::Result<T>,
    >(
        &self,
        f: F,
    ) -> anyhow::Result<T> {
        let db_ref = self.db.clone();
        spawn_blocking(move || {
            let mut db = db_ref.blocking_lock();
            f(&mut db)
        })
        .await?
    }
}

fn create_tables(conn: &Connection) -> anyhow::Result<()> {
    conn.execute(
        "CREATE TABLE if not exists listings (
            id           INTEGER PRIMARY KEY,
            created      INTEGER NOT NULL,
            last_seen    INTEGER NOT NULL,
            website      CHAR(32) NOT NULL,
            website_id   CHAR(32) NOT NULL,
            price        INTEGER NOT NULL,
            title        CHAR(128) NOT NULL,
            image_blob   INTEGER NOT NULL,
            star_rating  REAL,
            max_stars    REAL,
            num_reviews  INTEGER,
            UNIQUE (website, website_id)
        )",
        (),
    )?;
    conn.execute(
        "CREATE TABLE if not exists blobs (
            id           INTEGER PRIMARY KEY,
            hash         CHAR(32),
            data         BLOB,
            UNIQUE hash
        )",
        (),
    )?;
    conn.execute(
        "CREATE TABLE if not exists categories (
            listing_id   INTEGER,
            category     CHAR(64),
            PRIMARY KEY (listing_id, category)
        )",
        (),
    )?;
    conn.execute(
        "CREATE TABLE if not exists log (
            id         INTEGER PRIMARY KEY,
            timestamp  INTEGER,
            source     TEXT,
            message    TEXT
        )",
        (),
    )?;
    conn.execute(
        "CREATE INDEX listings_website_id ON listings(website, website_id)",
        (),
    )?;
    conn.execute(
        "CREATE INDEX listings_image_blob ON listings(image_blob)",
        (),
    )?;
    conn.execute("CREATE INDEX images_hash ON images(hash)", ())?;
    conn.execute("CREATE INDEX log_timestamp ON log(timestamp)", ())?;
    Ok(())
}

fn insert_blob(tx: &mut Transaction, blob: &[u8]) -> rusqlite::Result<i64> {
    let hash = hash_blob(blob);
    let result = tx.execute(
        "INSERT OR IGNORE INTO blobs (hash, data) VALUES (?1, ?2)",
        rusqlite::params![&hash, blob],
    )?;
    if result == 1 {
        Ok(tx.last_insert_rowid())
    } else {
        tx.query_row("SELECT id FROM blobs WHERE hash=?1", (hash,), |row| {
            row.get::<_, i64>(0)
        })
    }
}

fn garbage_collect_blob(tx: &mut Transaction, id: i64) -> rusqlite::Result<()> {
    let count: i64 = tx.query_row(
        "SELECT COUNT(*) FROM listings WHERE image_blob=?1",
        (id,),
        |row| row.get(0),
    )?;
    if count == 0 {
        tx.execute("DELETE FROM blobs WHERE id=?1", (id,))?;
    }
    Ok(())
}

fn insert_categories(
    tx: &mut Transaction,
    id: i64,
    categories: Vec<String>,
) -> rusqlite::Result<()> {
    for cat in categories {
        tx.execute(
            "INSERT OR IGNORE INTO categories (listing_id, category) VALUES (?1, ?2)",
            rusqlite::params![id, cat],
        )?;
    }
    Ok(())
}

fn hash_blob(data: &[u8]) -> String {
    let mut hasher = sha2::Sha256::new();
    hasher.update(data);
    let mut res = String::with_capacity(32);
    for ch in &hasher.finalize()[0..16] {
        write!(&mut res, "{:02x}", ch).unwrap();
    }
    res
}

#[derive(Debug)]
pub struct Listing {
    pub website: String,
    pub website_id: String,
    pub price: i64,
    pub title: String,
    pub image_data: Vec<u8>,
    pub categories: Vec<String>,

    // Optional per-website fields
    pub star_rating: Option<f64>,
    pub max_stars: Option<f64>,
    pub num_reviews: Option<i64>,
}
