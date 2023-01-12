use std::{fmt::Display, io::Write};

use flate2::{write::GzEncoder, Compression};
use futures_util::{StreamExt, TryStreamExt};
use http::{HeaderValue, StatusCode};
use hyper::{Body, Request, Response};
use serde::Serialize;
use tokio::task::spawn_blocking;

use crate::{db::Database, log_async};

const ERROR_PAGE: &str = include_str!("assets/internal_error.html");

pub async fn log_response(
    db: &Database,
    req: &Request<Body>,
    response: &Response<Body>,
) -> anyhow::Result<()> {
    let mut req_url = req.uri().to_string();
    req_url.truncate(128);

    let method = req.method().to_string();
    log_async!(
        db,
        "{} {} => {}",
        method,
        req_url,
        response.status().as_u16()
    );
    Ok(())
}

pub async fn read_body(req: &mut Request<Body>, max_size: usize) -> anyhow::Result<Vec<u8>> {
    let mut data = Vec::new();
    let mut stream = req.body_mut().into_stream();
    while let Some(x) = stream.next().await {
        data.extend_from_slice(&x?);
        if data.len() > max_size {
            return Err(anyhow::Error::msg("maximum POST size exceeded"));
        }
    }
    Ok(data)
}

pub async fn maybe_compress_response(
    req: &Request<Body>,
    mut resp: Response<Body>,
) -> Response<Body> {
    let is_api = resp.headers().get("content-type")
        == Some(&HeaderValue::from_str("application/json").unwrap());
    let accept_gzip = req
        .headers()
        .get("accept-encoding")
        .map(|x| x.to_str().unwrap_or_default())
        .unwrap_or_default()
        .split(", ")
        .map(|x| x.split(";").next().unwrap())
        .any(|x| x == "gzip");
    if !accept_gzip {
        resp
    } else {
        let pieces = resp
            .body_mut()
            .collect::<Vec<_>>()
            .await
            .into_iter()
            .collect::<hyper::Result<Vec<hyper::body::Bytes>>>();
        match pieces {
            Err(e) => error_response(is_api, "compressing body", e),
            Ok(data) => spawn_blocking(move || -> Response<Body> {
                let mut e = GzEncoder::new(Vec::new(), Compression::default());
                for part in data {
                    e.write_all(&part).unwrap();
                }
                let compressed_bytes = e.finish().unwrap();
                let mut builder = Response::builder().header("content-encoding", "gzip");
                for (k, v) in resp.headers().iter() {
                    builder = builder.header(k, v);
                }
                builder.body(Body::from(compressed_bytes)).unwrap()
            })
            .await
            .unwrap_or_else(|e| error_response(is_api, "compressing body", e)),
        }
    }
}

pub fn api_data_response<T: Serialize>(data: T) -> Response<Body> {
    match serde_json::to_vec(&DataResponse { data }) {
        Ok(encoded) => Response::builder()
            .header("content-type", "application/json")
            .body(Body::from(encoded))
            .unwrap(),
        Err(e) => api_error_response("encode data", e),
    }
}

pub fn error_response<E: Display>(is_api: bool, ctx: &str, err: E) -> Response<Body> {
    if is_api {
        api_error_response(ctx, err)
    } else {
        error_page_response(ctx, err)
    }
}

pub fn api_error_response<E: Display>(ctx: &str, err: E) -> Response<Body> {
    Response::builder()
        .status(StatusCode::INTERNAL_SERVER_ERROR)
        .header("content-type", "application/json")
        .body(Body::from(
            serde_json::to_vec(&ErrorResponse {
                error: format!("{}: {}", ctx, err.to_string()),
            })
            .expect("serialize error struct"),
        ))
        .unwrap()
}

fn error_page_response<E: Display>(ctx: &str, err: E) -> Response<Body> {
    Response::builder()
        .status(StatusCode::INTERNAL_SERVER_ERROR)
        .header("content-type", "text/html")
        .body(Body::from(
            ERROR_PAGE.replace("MSG", &format!("{}: {}", ctx, err)),
        ))
        .unwrap()
}

#[derive(Serialize)]
struct DataResponse<T: Serialize> {
    data: T,
}

#[derive(Serialize)]
struct ErrorResponse {
    error: String,
}
