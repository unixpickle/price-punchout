use std::{io::Read, ops::Deref};

use crate::{db::Listing, scraper::Client};
use reqwest::Url;
use serde::Deserialize;
use tokio::{
    spawn,
    sync::mpsc::{channel, Receiver},
};

pub const CATEGORIES: [(&str, &str); 13] = [
    ("Interesting Finds", "interesting-finds"),
    ("Tech & Gadgets", "adult-neutral:tech-and-gadgets"),
    ("Art & Design", "adult-neutral:art-and-design"),
    ("Home & Garden", "adult-neutral:home-and-garden"),
    ("Fitness & Sports", "adult-neutral:fitness-and-sports"),
    ("Home Improvement", "adult-neutral:home-improvement"),
    ("Outdoor Adventure", "adult-neutral:outdoor-adventure"),
    ("Travel", "adult-neutral:travel"),
    ("Workplace", "adult-neutral:business"),
    ("Style & Fashion (Male)", "adult-male:style-and-fashion"),
    ("Style & Fashion (Female)", "adult-female:style-and-fashion"),
    ("Retro", "adult-neutral:retro"),
    ("Geeky", "adult-neutral:geek-culture"),
];

#[derive(Deserialize)]
struct AmazonPage {
    asins: Vec<AmazonResult>,

    #[serde(rename(deserialize = "searchBlob"))]
    search_blob: String,
}

#[derive(Deserialize)]
struct AmazonResult {
    asin: String,
    price: Option<String>,
    title: String,

    #[serde(rename(deserialize = "displayLargeImageURL"))]
    display_large_image_url: Option<String>,

    #[serde(rename(deserialize = "starRating"))]
    star_rating: f64,

    #[serde(rename(deserialize = "reviewCount"))]
    review_count: String,

    #[serde(rename(deserialize = "hasHalfStar"))]
    has_half_star: bool,

    #[serde(rename(deserialize = "fullStarCount"))]
    full_star_count: i64,
}

pub fn stream_category(client: Client, category_id: String) -> Receiver<anyhow::Result<Listing>> {
    let (tx, rx) = channel(1);
    spawn(async move {
        let mut search_blob = "".to_owned();
        let mut offset = 0;
        loop {
            match result_page(&client, &category_id, &mut search_blob, &mut offset).await {
                Ok(results) => {
                    if results.len() == 0 {
                        break;
                    }
                    for item in results {
                        if tx.send(Ok(item)).await.is_err() {
                            break;
                        }
                    }
                }
                Err(e) => {
                    tx.send(Err(e.into())).await.ok();
                    break;
                }
            }
        }
    });
    rx
}

async fn result_page(
    client: &Client,
    category_id: &str,
    search_blob: &mut String,
    offset: &mut i64,
) -> anyhow::Result<Vec<Listing>> {
    let mut url = Url::parse("https://www.amazon.com/gcx/-/gfhz/api/scroll?canBeEGifted=false&canBeGiftWrapped=false&isLimitedTimeOffer=false&isPrime=false&priceFrom&priceTo").unwrap();

    // The ID may have no sub-id, or may be "id:subid".
    let (main_category_id, sub_category_id) = if let Some(split_idx) = category_id.find(":") {
        (
            &category_id[0..split_idx],
            Some(&category_id[(split_idx + 1)..]),
        )
    } else {
        (category_id, None)
    };

    url.query_pairs_mut()
        .append_pair("categoryId", main_category_id)
        .append_pair("count", "50")
        .append_pair("offset", &format!("{}", offset))
        .append_pair("searchBlob", &search_blob);
    if let Some(sub_category_id) = sub_category_id {
        url.query_pairs_mut().append_pair(
            "subcategoryIds",
            &format!("{}:{}", main_category_id, sub_category_id),
        );
    }
    let results = client
        .run_get(url, |resp| async {
            let zipped_data = resp.bytes().await?;
            let mut json_data = String::new();
            flate2::read::GzDecoder::new(zipped_data.deref()).read_to_string(&mut json_data)?;
            let data: AmazonPage = serde_json::from_str(&json_data)?;
            Ok(data)
        })
        .await?;

    *offset += results.asins.len() as i64;
    *search_blob = results.search_blob;

    let mut listings = Vec::with_capacity(results.asins.len());
    for item in results.asins {
        if item.price.is_none() {
            continue;
        }
        if let Some(image_url) = item.display_large_image_url {
            let image_data = client.get_bytes(image_url).await?;
            if let Ok(parsed_price) = (&item.price.unwrap().replace(&[',', '$'], "")).parse::<f64>()
            {
                listings.push(Listing {
                    website: "amazon.com".to_owned(),
                    website_id: item.asin,
                    price: (parsed_price * 100.0).round() as i64,
                    title: item.title,
                    image_data: image_data.into(),
                    categories: vec![category_id.to_owned()],
                    star_rating: Some(if item.has_half_star {
                        item.star_rating
                    } else {
                        item.full_star_count as f64
                    }),
                    max_stars: Some(5.0),
                    num_reviews: item.review_count.replace(",", "").parse().ok(),
                });
            }
        }
    }
    Ok(listings)
}
