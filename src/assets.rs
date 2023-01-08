use std::path::PathBuf;

use hyper::header::CONTENT_TYPE;
use hyper::{Body, Response};
use tokio::{
    fs::File,
    io::{self, AsyncReadExt},
};

// TODO(alex): currently, the '/' path separator is assumed.

macro_rules! asset_pair {
    ($path:expr) => {
        ($path, include_bytes!(concat!("assets/", $path)))
    };
}

const ASSETS: &'static [(&'static str, &'static [u8])] = &[
    asset_pair!("index.html"),
    asset_pair!("internal_error.html"),
    asset_pair!("not_found.html"),
    asset_pair!("style.css"),
    asset_pair!("favicon.ico"),
    asset_pair!("js/script.js"),
    asset_pair!("js/deps/babel5.8.23.js"),
    asset_pair!("js/deps/react15.3.2.js"),
    asset_pair!("js/deps/react-dom15.3.2.js"),
    asset_pair!("svg/gloves.svg"),
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
            let mut resp = Response::new(Body::from(bytes));
            resp.headers_mut()
                .insert(CONTENT_TYPE, mime_type.parse().unwrap());
            resp
        }
        Err(e) => Response::new(Body::from(format!("{}", e).into_bytes())),
    }
}

async fn read_asset_data<'a>(asset_dir: &'a Option<String>, name: &'a str) -> io::Result<Vec<u8>> {
    if let Some(asset_dir) = asset_dir.as_deref() {
        let mut f = File::open(
            [asset_dir]
                .into_iter()
                .chain(name.split("/"))
                .collect::<PathBuf>(),
        )
        .await?;
        let mut out = Vec::new();
        f.read_buf(&mut out).await?;
        Ok(out)
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
