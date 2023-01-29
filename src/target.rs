use std::collections::HashMap;

use regex::Regex;
use tokio::{
    spawn,
    sync::mpsc::{channel, Receiver},
};

use crate::{db::Listing, scraper::Client};

pub fn stream_category(client: Client, category_id: String) -> Receiver<anyhow::Result<Listing>> {
    let (tx, rx) = channel(1);
    spawn(async move {
        match extract_search_keys(client).await {
            Err(e) => {
                tx.send(Err(e)).await.ok();
            }
            Ok(search_keys) => {
                println!("{} {}", search_keys.api_key, search_keys.visitor_id);
            }
        }
    });
    rx
}

struct SearchKeys {
    api_key: String,
    visitor_id: String,
}

async fn extract_search_keys(client: Client) -> anyhow::Result<SearchKeys> {
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
