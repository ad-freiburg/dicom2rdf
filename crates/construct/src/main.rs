use flate2::{Compression, write::GzEncoder};
use log::info;
use std::path::PathBuf;
use std::{fs, io::Write};

use clap::Parser;
use config::Config;
use construct::{MkQueryResult, mk_nested_construct_queries, mk_top_level_construct_queries};
use reqwest::header::HeaderMap;

#[derive(Parser)]
struct Args {
    /// Path to config file
    #[arg(long, required = true)]
    config: PathBuf,

    /// File name prefix
    #[arg(long, required = true)]
    prefix: String,

    /// Path to output directory
    #[arg(long, required = true)]
    output: PathBuf,

    /// Maximum depth of container nesting to traverse
    #[arg(long, required = true)]
    max_depth: u8,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();
    let args = Args::parse();
    let config = Config::load_from_file(&args.config)?;
    let client = reqwest::Client::builder()
        .default_headers({
            let mut headers = HeaderMap::new();
            headers.insert(
                reqwest::header::CONTENT_TYPE,
                reqwest::header::HeaderValue::from_static("application/sparql-query"),
            );
            headers
        })
        .build()?;

    let queries = [
        mk_top_level_construct_queries(&config),
        mk_nested_construct_queries(&config, args.max_depth),
    ]
    .concat();
    let longest_query_name = queries
        .iter()
        .map(|q| q.name.len())
        .max()
        .expect("Expected at least one query");
    for MkQueryResult { name, query } in queries {
        let result = client
            .post("http://localhost:7055/api/default")
            .body(query.to_sparql())
            .send()
            .await?
            .bytes()
            .await?;
        let triple_count = std::str::from_utf8(&result)?
            .lines()
            .filter(|l| l.ends_with("."))
            .count();
        let mut gz_encoder = GzEncoder::new(Vec::new(), Compression::default());
        gz_encoder.write_all(&result)?;
        let writer = gz_encoder.finish()?;
        let filename = format!("{}.{}.ttl.gz", args.prefix, name);
        let output_path = args.output.join(filename);
        fs::write(&output_path, writer)?;
        info!("{:>longest_query_name$}: {} triples", name, triple_count);
    }
    Ok(())
}
