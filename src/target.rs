use std::collections::HashMap;

use regex::Regex;
use serde::Deserialize;
use tokio::{
    spawn,
    sync::mpsc::{channel, Receiver},
};

use crate::{db::Listing, scraper::Client};

pub const CATEGORY_CLOTHES_SHOES_ACCESSORIES: &str = "rdihz";
pub const CATEGORY_SPORTS_AND_OUTDOORS: &str = "5xt85";

pub fn stream_category(client: Client, category_id: String) -> Receiver<anyhow::Result<Listing>> {
    let (tx, rx) = channel(1);
    spawn(async move {
        match extract_search_keys(&client).await {
            Err(e) => {
                tx.send(Err(e)).await.ok();
            }
            Ok(search_keys) => {
                let mut offset = 0;
                loop {
                    let full_url = format!(
                        "{endpoint}?key={api_key}&category={category}&channel=WEB&count=24&default_purchasability_filter=true&include_sponsored=true&offset={offset}&page=%2Fc%2F{category}&platform=desktop&pricing_store_id=2766&scheduled_delivery_store_id=2766&store_ids=2766&visitor_id={visitor_id}&zip=19096",
                        endpoint="https://redsky.target.com/redsky_aggregations/v1/web/plp_search_v2",
                        api_key=search_keys.api_key,
                        category=category_id,
                        offset=offset,
                        visitor_id=search_keys.visitor_id,
                    );
                    let page_results = client
                        .run_get(full_url, |resp| async {
                            let data = resp.bytes().await?;
                            Ok(serde_json::from_slice::<'_, SearchResult>(&data)?)
                        })
                        .await;
                    match page_results {
                        Ok(results) => {
                            if results.data.search.products.len() == 0 {
                                return;
                            }
                            offset += results.data.search.products.len();
                            for item in results.data.search.products {
                                match product_listing(&client, category_id.clone(), item).await {
                                    Ok(Some(x)) => {
                                        if tx.send(Ok(x)).await.is_err() {
                                            return;
                                        }
                                    }
                                    Ok(None) => (),
                                    Err(e) => {
                                        tx.send(Err(e)).await.ok();
                                        return;
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            tx.send(Err(e)).await.ok();
                            return;
                        }
                    }
                }
            }
        }
    });
    rx
}

async fn product_listing(
    client: &Client,
    category: String,
    product: SearchResultProduct,
) -> anyhow::Result<Option<Listing>> {
    if let Ok(parsed_price) = (&product
        .price
        .formatted_current_price
        .replace(&[',', '$'], ""))
        .parse::<f64>()
    {
        let image_data = client
            .get_bytes(product.item.enrichment.images.primary_image_url)
            .await?;
        Ok(Some(Listing {
            website: "target.com".to_owned(),
            website_id: product.tcin,
            price: (parsed_price * 100.0).round() as i64,
            title: product.item.product_description.title,
            image_data: image_data,
            categories: vec![category],
            star_rating: None,
            max_stars: None,
            num_reviews: None,
        }))
    } else {
        Ok(None)
    }
}

#[derive(Deserialize)]
struct SearchResult {
    data: SearchResultData,
}

#[derive(Deserialize)]
struct SearchResultData {
    search: SearchResultDataSearch,
}

#[derive(Deserialize)]
struct SearchResultDataSearch {
    products: Vec<SearchResultProduct>,
}

#[derive(Deserialize)]
struct SearchResultProduct {
    // #[serde(rename(deserialize = "__typename"))]
    // typename: String,
    tcin: String,
    // original_tcin: String,
    price: SearchResultPrice,
    item: SearchResultItem,
}

#[derive(Deserialize)]
struct SearchResultPrice {
    formatted_current_price: String,
    // formatted_current_price_type: String,
}

#[derive(Deserialize)]
struct SearchResultItem {
    enrichment: SearchResultEnrichment,
    product_description: SearchResultDescription,
}

#[derive(Deserialize)]
struct SearchResultDescription {
    title: String,
}

#[derive(Deserialize)]
struct SearchResultEnrichment {
    images: SearchResultImages,
}

#[derive(Deserialize)]
struct SearchResultImages {
    primary_image_url: String,
}

struct SearchKeys {
    api_key: String,
    visitor_id: String,
}

async fn extract_search_keys(client: &Client) -> anyhow::Result<SearchKeys> {
    client
        .run_get("https://www.target.com", |resp| async {
            let response = resp.text().await?;
            let prefix = "Object.defineProperties(window, {";
            let index = response
                .find(prefix)
                .ok_or(anyhow::Error::msg("search API key prefix not found"))?;
            let pattern = Regex::new(
                "\\s*'(.*)': \\{.*value: deepFreeze\\(JSON.parse\\((\".*\")\\)\\).*\\},",
            )
            .unwrap();
            let mut configs = HashMap::new();
            for group in pattern.captures_iter(&response[index..]) {
                let key = group.get(1).unwrap();
                let data = group.get(2).unwrap();
                let unescaped: String = serde_json::from_str(data.as_str())?;
                configs.insert(key.as_str().to_owned(), unescaped);
            }
            for key in ["__CONFIG__", "__TGT_DATA__"] {
                if !configs.contains_key(key) {
                    return Err(anyhow::Error::msg(format!(
                        "target search API key config missing: {}",
                        key
                    )));
                }
            }

            let key_pattern = Regex::new(r#"apiKey":"([a-fA-F0-9]*)"#).unwrap();
            let mut counts = HashMap::<String, usize>::new();
            for key in key_pattern.captures_iter(&configs["__CONFIG__"]) {
                *counts
                    .entry(key.get(1).unwrap().as_str().to_owned())
                    .or_insert(0) += 1;
            }
            if counts.len() == 0 {
                return Err(anyhow::Error::msg(
                    "no api keys found in Target homepage config",
                ));
            }
            let max_count = *counts.values().max().unwrap();
            let most_common_key = counts
                .into_iter()
                .filter_map(|(k, c)| if c == max_count { Some(k) } else { None })
                .next()
                .unwrap();

            let vid_pattern = Regex::new(r#"visitor_id":"([0-9a-fA-F]*)"#).unwrap();
            let vid = vid_pattern
                .captures(&configs["__TGT_DATA__"])
                .ok_or(anyhow::Error::msg(
                    "no visitor ID found in Target homepage data",
                ))?
                .get(1)
                .unwrap();

            Ok(SearchKeys {
                api_key: most_common_key,
                visitor_id: vid.as_str().to_owned(),
            })
        })
        .await
}
