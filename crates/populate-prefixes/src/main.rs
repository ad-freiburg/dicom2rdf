use std::{
    error::Error,
    fs,
    path::{Path, PathBuf},
};

use clap::Parser;
use config::Config;
use itertools::Itertools;
use serde_yaml::Value;

#[derive(Parser)]
struct Args {
    /// Path to config file
    #[arg(long)]
    config: PathBuf,

    /// Path to Qleverfile-ui.yml
    #[arg(long)]
    qleverfile_ui: PathBuf,
}

fn add_suggested_prefixes(
    config: &Config,
    qleverfile_ui: impl AsRef<Path>,
) -> Result<(), Box<dyn Error>> {
    let yaml_str = fs::read_to_string(&qleverfile_ui)?;
    let mut yaml: Value = serde_yaml::from_str(&yaml_str)?;
    let suggested_prefixes = config
        .to_prefix_iri_pairs()
        .map(|(prefix, iri)| format!("@prefix {}: <{}> .", prefix, iri))
        .join("\n");
    yaml["config"]["backend"]["suggestedPrefixes"] = Value::String(suggested_prefixes);
    fs::write(&qleverfile_ui, serde_yaml::to_string(&yaml)?)?;
    Ok(())
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();
    let config = Config::load_from_file(&args.config)?;

    add_suggested_prefixes(&config, &args.qleverfile_ui)?;

    Ok(())
}
