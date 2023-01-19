use std::path::PathBuf;

use hyper::header::CONTENT_TYPE;
use hyper::{Body, Response};
use tokio::io;

// TODO(alex): currently, the '/' path separator is assumed.
macro_rules! path_join {
    ($x:expr) => {
        $x
    };
    ($x:expr, $($y:expr),+) => {
        concat!($x, "/", path_join!($($y),*))
    };
}

macro_rules! asset_pair {
    ($path:expr) => {
        ($path, include_bytes!(path_join!("assets", $path)))
    };
}

const ASSETS: &'static [(&'static str, &'static [u8])] = &[
    asset_pair!("index.html"),
    asset_pair!("internal_error.html"),
    asset_pair!("not_found.html"),
    asset_pair!("style.css"),
    asset_pair!("favicon.ico"),
    asset_pair!(path_join!("js", "api.js")),
    asset_pair!(path_join!("js", "script.js")),
    asset_pair!(path_join!("js", "deps", "babel-standalone@6.26.0.js")),
    asset_pair!(path_join!("js", "deps", "react-dom@18.2.0.js")),
    asset_pair!(path_join!("js", "deps", "react@18.2.0.js")),
    asset_pair!(path_join!("svg", "amazon_box.svg")),
    asset_pair!(path_join!("svg", "back.svg")),
    asset_pair!(path_join!("svg", "calculator.svg")),
    asset_pair!(path_join!("svg", "gloves.svg")),
    asset_pair!(path_join!("svg", "list_bg.svg")),
    asset_pair!(path_join!("svg", "loader.svg")),
    asset_pair!(path_join!("svg", "treasure_chest.svg")),
];

pub async fn asset_response<'a>(asset_dir: &'a Option<String>, name: &'a str) -> Response<Body> {
    let mut mime_type = if name.ends_with(".js") {
        "application/javascript"
    } else if name.ends_with(".html") {
        "text/html"
    } else if name.ends_with(".css") {
        "text/css"
    } else if name.ends_with(".ico") {
        "image/x-icon"
    } else if name.ends_with(".svg") {
        "image/svg+xml"
    } else {
        "application/octet-stream"
    };

    let mut data = read_asset_data(asset_dir, name).await;
    if data.is_err() {
        data = read_asset_data(asset_dir, "not_found.html").await;
        mime_type = "text/html";
    }
    match data {
        Ok(bytes) => {
            let mut resp = Response::builder().header(CONTENT_TYPE, mime_type);
            if asset_dir.is_some() {
                resp = resp
                    .header("Cache-Control", "no-cache, no-store, must-revalidate")
                    .header("Pragma", "no-cache")
                    .header("Expires", "0");
            }
            resp.body(Body::from(bytes)).unwrap()
        }
        Err(e) => Response::new(Body::from(format!("{}", e).into_bytes())),
    }
}

async fn read_asset_data<'a>(asset_dir: &'a Option<String>, name: &'a str) -> io::Result<Vec<u8>> {
    if let Some(asset_dir) = asset_dir.as_deref() {
        tokio::fs::read(
            [asset_dir]
                .into_iter()
                .chain(name.split("/"))
                .collect::<PathBuf>(),
        )
        .await
    } else {
        return ASSETS
            .iter()
            .filter(|(n, _)| {
                *n == if name.starts_with("/") {
                    &name[1..]
                } else {
                    name
                }
            })
            .next()
            .map(|(_, content)| (*content).to_owned())
            .ok_or(io::Error::new(io::ErrorKind::NotFound, "asset not found"));
    }
}
