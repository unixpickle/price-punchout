use std::process::ExitCode;

use clap::Parser;

mod db;
mod scraper;

#[derive(Clone, Parser)]
pub struct Args {
    #[clap(value_parser)]
    db_path: String,
}

#[tokio::main]
async fn main() -> ExitCode {
    let args = Args::parse();
    ExitCode::SUCCESS
}
