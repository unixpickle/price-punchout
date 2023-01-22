use std::fmt::Write;
use std::path::Path;
use std::rc::Rc;
use std::sync::Arc;

use rusqlite::{Connection, Transaction};
use sha2::Digest;
use tokio::{sync::Mutex, task::spawn_blocking};

use crate::levels::{Level, LEVELS};

const LOG_LIMIT: i64 = 5000;

#[derive(Clone)]
pub struct Database {
    db: Arc<Mutex<Connection>>,
}

impl Database {
    pub async fn open<P: AsRef<Path>>(path: P) -> rusqlite::Result<Database> {
        let path = path.as_ref().to_owned();
        spawn_blocking_rusqlite(move || Database::new_with_conn(Connection::open(path)?)).await
    }

    #[allow(dead_code)]
    pub async fn open_in_memory() -> rusqlite::Result<Database> {
        spawn_blocking_rusqlite(move || Database::new_with_conn(Connection::open_in_memory()?))
            .await
    }

    fn new_with_conn(conn: Connection) -> rusqlite::Result<Database> {
        create_tables(&conn)?;
        rusqlite::vtab::array::load_module(&conn)?;
        Ok(Database {
            db: Arc::new(Mutex::new(conn)),
        })
    }

    // Either insert a new listing, or update the information if the website_id
    // is already present in the database.
    pub async fn insert_or_update(&self, listing: Listing) -> rusqlite::Result<()> {
        self.with_db(move |db| {
            let mut tx = db.transaction()?;
            let blob_id = insert_blob(&mut tx, &listing.image_data)?;
            let result: rusqlite::Result<(i64, i64)> = tx.query_row(
                "SELECT id, image_blob FROM listings WHERE website=?1 AND website_id=?2",
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
                            num_reviews
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
            tx.commit()?;
            Ok(())
        })
        .await
    }

    pub async fn insert_log_message(
        &self,
        source: String,
        message: String,
    ) -> rusqlite::Result<()> {
        self.with_db(move |db| {
            let tx = db.transaction()?;
            tx.execute(
                "INSERT INTO log (timestamp, source, message) VALUES (unixepoch(), ?1, ?2)",
                (source, message),
            )?;
            tx.execute(
                "DELETE FROM log WHERE id NOT IN (
                    SELECT id FROM log ORDER BY timestamp DESC LIMIT ?1
                )",
                (LOG_LIMIT,),
            )?;
            tx.commit()
        })
        .await
    }

    pub async fn should_update_source(
        &self,
        source: String,
        max_seconds: i64,
    ) -> rusqlite::Result<bool> {
        self.with_db(move |db| {
            match db.query_row(
                "SELECT (last_updated+?1 < unixepoch()) FROM source_status WHERE source_id=?2",
                (max_seconds, source),
                |row| row.get::<_, bool>(0),
            ) {
                Ok(value) => Ok(value),
                Err(rusqlite::Error::QueryReturnedNoRows) => Ok(true),
                x @ Err(_) => x.map_err(|e| e.into()),
            }
        })
        .await
    }

    pub async fn updated_source(&self, source: String) -> rusqlite::Result<()> {
        self.with_db(move |db| {
            db.execute(
                "INSERT OR REPLACE INTO source_status (source_id, last_updated) VALUES (?1, unixepoch())",
                (source,),
            ).map(|_| ())
        })
        .await
    }

    // Delete old listings in categories that have more than enough
    // listings. Retains listings which are needed for some category
    // when that category is sorted by last seen date.
    //
    // Returns (deleted listings, deleted blobs).
    pub async fn delete_old_listings(
        &self,
        category_capacity: i64,
    ) -> rusqlite::Result<(usize, usize)> {
        self.with_db(move |db| {
            let tx = db.transaction()?;

            // Retain listings that are not in any levels, since they
            // won't be updated and don't pose a leak threat as a result.
            tx.execute("UPDATE listings SET sweep_mark = 1", ())?;

            // By default, every listing contained with a level is dropped.
            for level in LEVELS {
                tx.execute(
                    &format!(
                        "UPDATE listings SET sweep_mark = 0 WHERE {}",
                        level.listing_query()
                    ),
                    (),
                )?;
            }

            // Explicitly mark the latest listings of every level to be retained.
            for level in LEVELS {
                tx.execute(
                    &format!(
                        "
                            UPDATE listings
                            SET sweep_mark = 1
                            WHERE id IN (
                                SELECT id FROM listings
                                WHERE {}
                                ORDER BY last_seen DESC
                                LIMIT ?1
                            )
                        ",
                        level.listing_query()
                    ),
                    (category_capacity,),
                )?;
            }

            let listing_count = tx.execute("DELETE FROM listings WHERE sweep_mark = 0", ())?;

            let blob_count = tx.execute(
                "
                    DELETE FROM blobs WHERE (
                        SELECT COUNT(*) FROM listings WHERE listings.image_blob = blobs.id
                    ) == 0",
                (),
            )?;
            tx.commit()?;
            Ok((listing_count, blob_count))
        })
        .await
    }

    pub async fn level_count<I: 'static + Send + Sync + IntoIterator<Item = i64>>(
        &self,
        blacklist: I,
        level: &'static Level,
    ) -> rusqlite::Result<i64> {
        self.with_db(move |db| {
            let query = format!(
                "SELECT COUNT(*) FROM listings WHERE {} AND id NOT IN rarray(?1)",
                level.listing_query()
            );
            db.query_row(&query, (&values_to_rarray(blacklist),), |row| row.get(0))
        })
        .await
    }

    pub async fn sample_listing<I: 'static + Send + Sync + IntoIterator<Item = i64>>(
        &self,
        blacklist: I,
        level: &'static Level,
    ) -> rusqlite::Result<Option<(Listing, i64)>> {
        self.with_db(move |db| {
            let tx = db.transaction()?;
            let query = format!(
                "
                    SELECT * FROM listings
                    WHERE {} AND id NOT IN rarray(?1)
                    ORDER BY RANDOM()
                    LIMIT 1
                ",
                level.listing_query(),
            );
            let result = tx.query_row(&query, (&values_to_rarray(blacklist),), |row| {
                Ok((
                    Listing {
                        website: row.get("website")?,
                        website_id: row.get("website_id")?,
                        price: row.get("price")?,
                        title: row.get("title")?,
                        image_data: Vec::default(),
                        categories: Vec::default(),
                        star_rating: row.get("star_rating")?,
                        max_stars: row.get("max_stars")?,
                        num_reviews: row.get("num_reviews")?,
                    },
                    row.get::<_, i64>("id")?,
                    row.get::<_, i64>("image_blob")?,
                ))
            });
            match result {
                Ok((mut listing, listing_id, image_id)) => {
                    let categories: rusqlite::Result<Vec<String>> = tx
                        .prepare("SELECT category FROM categories WHERE listing_id=?1")?
                        .query_map((&listing_id,), |row| row.get("category"))?
                        .into_iter()
                        .collect();
                    listing.categories = categories?;
                    listing.image_data =
                        tx.query_row("SELECT data FROM blobs WHERE id=?1", (&image_id,), |row| {
                            row.get("data")
                        })?;
                    Ok(Some((listing, listing_id)))
                }
                Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
                Err(e) => Err(e),
            }
        })
        .await
    }

    async fn with_db<
        T: 'static + Send,
        F: 'static + Send + FnOnce(&mut Connection) -> rusqlite::Result<T>,
    >(
        &self,
        f: F,
    ) -> rusqlite::Result<T> {
        let db_ref = self.db.clone();
        spawn_blocking_rusqlite(move || {
            let mut db = db_ref.blocking_lock();
            f(&mut db)
        })
        .await
    }
}

async fn spawn_blocking_rusqlite<
    T: 'static + Send,
    F: 'static + Send + FnOnce() -> rusqlite::Result<T>,
>(
    f: F,
) -> rusqlite::Result<T> {
    match spawn_blocking(f).await {
        Ok(x) => x,
        Err(e) => Err(rusqlite::Error::SqliteFailure(
            rusqlite::ffi::Error {
                code: rusqlite::ErrorCode::InternalMalfunction,
                extended_code: 0,
            },
            Some(format!("{}", e)),
        )),
    }
}

fn create_tables(conn: &Connection) -> rusqlite::Result<()> {
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
            sweep_mark   INT,
            UNIQUE (website, website_id)
        )",
        (),
    )?;
    conn.execute(
        "CREATE TABLE if not exists blobs (
            id           INTEGER PRIMARY KEY,
            hash         CHAR(32),
            data         BLOB,
            UNIQUE (hash)
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
        "CREATE TABLE if not exists source_status (
            source_id    CHAR(64),
            last_updated INTEGER,
            PRIMARY KEY (source_id)
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
        "CREATE INDEX if not exists listings_website_id ON listings(website, website_id)",
        (),
    )?;
    conn.execute(
        "CREATE INDEX if not exists listings_image_blob ON listings(image_blob)",
        (),
    )?;
    conn.execute(
        "CREATE INDEX if not exists listings_last_seen ON listings(last_seen)",
        (),
    )?;
    conn.execute("CREATE INDEX if not exists blobs_hash ON blobs(hash)", ())?;
    conn.execute(
        "CREATE INDEX if not exists log_timestamp ON log(timestamp)",
        (),
    )?;
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

fn values_to_rarray<T, I: 'static + Send + Sync + IntoIterator<Item = T>>(
    blacklist: I,
) -> Rc<Vec<rusqlite::types::Value>>
where
    rusqlite::types::Value: From<T>,
{
    Rc::new(
        blacklist
            .into_iter()
            .map(rusqlite::types::Value::from)
            .collect::<Vec<rusqlite::types::Value>>(),
    )
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
