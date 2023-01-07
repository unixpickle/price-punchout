use crate::{amazon, log_async};
use crate::{db::Database, scraper::Client};
use std::{future::Future, pin::Pin, time::Duration};
use tokio::time::sleep;

// The frequency with which to check if a source needs to be updated.
// This is not actually the interval of updates, which is determined by the
// caller of update_sources_loop().
const LOOP_CHECK_INTERVAL: Duration = Duration::from_secs(60);

// A soft limit to prevent levels from growing unboundedly.
const MAX_LISTINGS_PER_LEVEL: i64 = 4096;

// This limit is applied to amazon searches to prevent the scraper from
// fetching too many pages of results.
const AMAZON_RESULT_LIMIT: i64 = 50;

// A source of retail listing data.
//
// Each source implementation should have its own string identifier, which may
// be scoped differently than an entire website.
pub trait Source: Send + Sync {
    fn identifier(&self) -> String;
    fn update_listings<'a>(
        &'a self,
        client: &'a Client,
        db: &'a Database,
    ) -> Pin<Box<dyn 'a + Send + Sync + Future<Output = anyhow::Result<()>>>>;
}

pub struct AmazonSource {
    category: String,
    max_items: i64,
}

impl Source for AmazonSource {
    fn identifier(&self) -> String {
        self.category.clone()
    }

    fn update_listings<'a>(
        &'a self,
        client: &'a Client,
        db: &'a Database,
    ) -> Pin<Box<dyn 'a + Send + Sync + Future<Output = anyhow::Result<()>>>> {
        Box::pin(async move {
            let mut listings = amazon::stream_category(client.clone(), self.category.clone());
            let mut count = 0;
            while let Some(result) = listings.recv().await {
                let listing = result?;
                db.insert_or_update(listing).await?;
                count += 1;
                if count >= self.max_items {
                    break;
                }
            }
            Ok(())
        })
    }
}

pub fn default_sources() -> Vec<Box<dyn Source>> {
    vec![
        Box::new(AmazonSource {
            category: "interesting-finds".to_owned(),
            max_items: AMAZON_RESULT_LIMIT,
        }),
        Box::new(AmazonSource {
            // Tools and Home Improvement
            category: "hgg-hol-hi".to_owned(),
            max_items: AMAZON_RESULT_LIMIT,
        }),
        Box::new(AmazonSource {
            // Electronics
            category: "EGGHOL22-Hub".to_owned(),
            max_items: AMAZON_RESULT_LIMIT,
        }),
    ]
}

pub async fn update_sources_loop(
    client: Client,
    db: Database,
    update_interval: Duration,
    sources: Vec<Box<dyn Source>>,
) -> anyhow::Result<()> {
    loop {
        let mut updated_any: bool = false;
        for source in &sources {
            let id = source.identifier();
            if db
                .should_update_source(id.clone(), update_interval.as_secs_f64().ceil() as i64)
                .await?
            {
                log_async!(&db, "updating source {}", id);
                if let Err(e) = source.update_listings(&client, &db).await {
                    log_async!(&db, "error updating source {}: {}", id, e);
                } else {
                    log_async!(&db, "successfully updated source {}", id);
                }
                updated_any = true;
                db.updated_source(id).await?;
            }
        }
        if updated_any {
            let (purged_listings, purged_blobs) =
                db.delete_old_listings(MAX_LISTINGS_PER_LEVEL).await?;
            log_async!(
                &db,
                "ran delete cycle: {} listings and {} blobs deleted.",
                purged_listings,
                purged_blobs
            );
        }
        sleep(LOOP_CHECK_INTERVAL).await;
    }
}
