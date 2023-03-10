use assets::read_asset_data;
use hyper::header::CONTENT_TYPE;
use rand::thread_rng;
use std::convert::Infallible;
use std::process::ExitCode;

use crate::assets::asset_response;
use crate::bg::Background;
use crate::db::Database;
use crate::http_util::maybe_compress_response;
use crate::scraper::Client;
use crate::sources::{default_sources, update_sources_loop};
use clap::Parser;
use http_util::{api_response, detect_image_mime, log_response, read_body};
use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Request, Response, Server};
use levels::{Level, LEVELS};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::net::SocketAddr;
use std::time::Duration;
use tokio::spawn;

mod amazon;
mod assets;
mod bg;
mod db;
mod http_util;
mod levels;
mod log;
mod scraper;
mod sources;
mod target;

#[derive(Clone, Parser)]
pub struct Args {
    #[clap(short, long)]
    asset_dir: Option<String>,

    #[clap(short, long, value_parser, default_value_t = 60*60*24)]
    update_interval: u64,

    #[clap(short, long, value_parser, default_value_t = false)]
    no_updates: bool,

    #[clap(short, long, value_parser, default_value_t = 10)]
    client_retries: i32,

    #[clap(long, value_parser, default_value_t = 1<<20)]
    max_post_size: usize,

    #[clap(short, long, value_parser, default_value_t = 8080)]
    port: u16,

    #[clap(value_parser)]
    db_path: String,
}

#[tokio::main]
async fn main() -> ExitCode {
    let args = Args::parse();
    match run(args).await {
        Err(e) => {
            eprintln!("{}", e);
            ExitCode::FAILURE
        }
        Ok(_) => ExitCode::SUCCESS,
    }
}

async fn run(args: Args) -> anyhow::Result<()> {
    let db = Database::open(&args.db_path).await?;

    let http_client = Client::new(args.client_retries);
    if !args.no_updates {
        let sources_db = db.clone();
        spawn(async move {
            update_sources_loop(
                http_client,
                sources_db,
                Duration::from_secs(args.update_interval),
                default_sources(),
            )
            .await
            .expect("update sources loop should never fail; this is a fatal error");
        });
    }

    let addr = SocketAddr::from(([0, 0, 0, 0], args.port));
    let state = ServerState {
        args,
        db: db.clone(),
    };
    let make_service = make_service_fn(move |_conn| {
        let state_clone = state.clone();
        async move {
            Ok::<_, Infallible>(service_fn(move |req: Request<Body>| {
                let state_clone_clone = state_clone.clone();
                async move { handle_request(req, state_clone_clone).await }
            }))
        }
    });

    log_async!(&db, "creating server at {}...", addr);
    Server::bind(&addr).serve(make_service).await?;

    Ok(())
}

#[derive(Clone)]
struct ServerState {
    args: Args,
    db: Database,
}

async fn handle_request(
    mut req: Request<Body>,
    state: ServerState,
) -> Result<Response<Body>, Infallible> {
    let response = match req.uri().path() {
        "" | "/" => homepage(&state).await,
        "/api/levels" => api_response(
            &state.db,
            "list levels",
            non_empty_levels(&state, &mut req).await,
        )
        .await
        .unwrap(),
        "/api/sample" => api_response(
            &state.db,
            "sample listing",
            sample_listing(&state, &mut req).await,
        )
        .await
        .unwrap(),
        path => asset_response(&state.args.asset_dir, &path).await,
    };
    let response = maybe_compress_response(&req, response).await;
    log_response(&state.db, &req, &response)
        .await
        .expect("logging response should always work");
    Ok(response)
}

async fn homepage(state: &ServerState) -> Response<Body> {
    match read_asset_data(&state.args.asset_dir, "index.html").await {
        Ok(bytes) => {
            let bg = Background::sample(&mut thread_rng());
            let mut bytes_str: String = String::from_utf8_lossy(&bytes).into();
            bytes_str = bytes_str.replace("<!--BACKGROUND_HTML-->", &bg.html);
            bytes_str = bytes_str.replace("/*BACKGROUND_CSS*/", &bg.css);
            let mut resp = Response::builder().header(CONTENT_TYPE, "text/html");
            resp = resp
                .header("Cache-Control", "no-cache, no-store, must-revalidate")
                .header("Pragma", "no-cache")
                .header("Expires", "0");
            resp.body(Body::from(bytes_str)).unwrap()
        }
        Err(e) => Response::new(Body::from(format!("{}", e).into_bytes())),
    }
}

async fn non_empty_levels(
    state: &ServerState,
    req: &mut Request<Body>,
) -> anyhow::Result<Vec<Value>> {
    let post_data = read_body(req, state.args.max_post_size).await?;
    let req_data: LevelsRequest = serde_json::from_slice(&post_data)?;

    let mut levels = Vec::new();
    for level in &LEVELS {
        let count = state
            .db
            .level_count(req_data.seen_ids.clone(), level)
            .await?;
        if count > 0 {
            levels.push(Value::Object(
                [
                    ("id".to_owned(), level.id.to_owned().into()),
                    (
                        "website_name".to_owned(),
                        level.website_name.to_owned().into(),
                    ),
                    (
                        "category_name".to_owned(),
                        level.category_name.to_owned().into(),
                    ),
                    ("count".to_owned(), count.into()),
                ]
                .into_iter()
                .collect(),
            ));
        }
    }
    Ok(levels)
}

async fn sample_listing(state: &ServerState, req: &mut Request<Body>) -> anyhow::Result<Value> {
    let post_data = read_body(req, state.args.max_post_size).await?;
    let req_data: ListingRequest = serde_json::from_slice(&post_data)?;
    if let Some(level) = Level::find_by_id(&req_data.level) {
        match state.db.sample_listing(req_data.seen_ids, level).await? {
            Some((item, id)) => Ok(serde_json::to_value(ListingResponse {
                id,
                title: Some(item.title),
                price: Some(item.price),
                image_url: Some(format!(
                    "data:{};base64,{}",
                    detect_image_mime(&item.image_data).unwrap_or("image/jpeg"),
                    base64::encode(item.image_data)
                )),
            })?),
            None => Ok(serde_json::to_value(ListingResponse::default())?),
        }
    } else {
        Err(anyhow::Error::msg("no level found with the supplied ID"))
    }
}

#[derive(Deserialize)]
struct LevelsRequest {
    #[serde(rename(deserialize = "seenIDs"))]
    seen_ids: Vec<i64>,
}

#[derive(Deserialize)]
struct ListingRequest {
    #[serde(rename(deserialize = "seenIDs"))]
    seen_ids: Vec<i64>,
    level: String,
}

#[derive(Default, Serialize)]
struct ListingResponse {
    id: i64,
    title: Option<String>,
    price: Option<i64>,

    #[serde(rename(serialize = "imageURL"))]
    image_url: Option<String>,
}
