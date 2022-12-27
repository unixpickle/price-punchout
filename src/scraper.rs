use std::{future::Future, time::Duration};

use reqwest::IntoUrl;
use tokio::time::sleep;

#[derive(Clone)]
pub struct Client {
    client: reqwest::Client,
    num_retries: i32,
}

impl Client {
    pub fn new(num_retries: i32) -> Client {
        Client {
            client: reqwest::Client::new(),
            num_retries: num_retries,
        }
    }

    pub async fn run_get<
        T,
        U: IntoUrl,
        F: Future<Output = anyhow::Result<T>>,
        Resp: Fn(reqwest::Response) -> F,
    >(
        &mut self,
        url: U,
        resp_fn: Resp,
    ) -> anyhow::Result<T> {
        let u = url.into_url()?;
        self.run(move |client| client.get(u.clone()), resp_fn).await
    }

    pub async fn run<
        T,
        F: Future<Output = anyhow::Result<T>>,
        Req: Fn(&reqwest::Client) -> reqwest::RequestBuilder,
        Resp: Fn(reqwest::Response) -> F,
    >(
        &mut self,
        req_fn: Req,
        resp_fn: Resp,
    ) -> anyhow::Result<T> {
        let mut last_err: anyhow::Error = anyhow::Error::msg("UNREACHABLE");
        for i in 0..self.num_retries {
            let builder = req_fn(&self.client).timeout(Duration::from_secs(30));
            let result = builder.send().await;
            match result {
                Err(e) => {
                    last_err = e.into();
                    self.client = reqwest::Client::new();
                    if i + 1 < self.num_retries {
                        sleep(Duration::from_secs(10)).await;
                    }
                }
                Ok(resp) => {
                    let output = resp_fn(resp).await;
                    match output {
                        Err(e) => {
                            last_err = e;
                        }
                        Ok(x) => {
                            return Ok(x);
                        }
                    }
                }
            };
        }
        Err(last_err)
    }
}
