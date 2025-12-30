#[global_allocator]
static GLOBAL: tikv_jemallocator::Jemalloc = tikv_jemallocator::Jemalloc;

use clap::Parser;
use config::Config;
use convert::dicom::write_triples;
use convert::io::{
    TripleWriter, get_dcm_or_zst_paths, handle_zst_file, mk_ttl_gz_writer, mk_writer,
};
use convert::progress::mk_progress_logger;
use convert::turtle;
use dicom::object::open_file;
use log::{info, warn};
use rayon::prelude::*;
use std::io::Write;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc;

fn dir_exists(s: &str) -> Result<PathBuf, String> {
    let path = PathBuf::from(s);
    if path.is_dir() {
        Ok(path)
    } else {
        Err(format!("'{}' is not a directory", s))
    }
}

#[derive(Parser, Debug)]
#[command(version)]
struct Args {
    /// Path to config file
    #[arg(long, required = true)]
    config: PathBuf,

    /// Directory containing *.dcm or *.tar.zst input files
    #[arg(long, required = true, value_parser = dir_exists)]
    input_dir: PathBuf,

    /// Directory where the output is written to
    #[arg(long, required = true, value_parser = dir_exists)]
    output_dir: PathBuf,
}

fn convert_file(
    triple_writer: &mut TripleWriter<impl Write>,
    error_writer: &mut impl Write,
    path: &PathBuf,
    config: &Config,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut buffer = Vec::new();
    let (dicom_file_path, _temp_dir_guard) =
        if path.extension().and_then(|s| s.to_str()) == Some("zst") {
            handle_zst_file(&path)?
        } else {
            (path.clone(), None)
        };

    let file_name = dicom_file_path
        .file_name()
        .and_then(|s| s.to_str())
        .ok_or("Failed to get DICOM file name")?;
    let dicom_object = open_file(&dicom_file_path)?;
    let file_subject = turtle::IRI::prefix("dicom2rdf", file_name);
    writeln!(
        &mut buffer,
        "{}",
        turtle::triple(
            &file_subject,
            &turtle::IRI::prefix("rdf", "type"),
            &turtle::TripleObject::from(turtle::IRI::prefix("dicom2rdf", "DocumentRoot")),
        )
    )?;

    let (_, max_depth) = write_triples(
        &mut buffer,
        error_writer,
        &file_subject,
        &dicom_object,
        &file_name,
        &config,
        0,
    );
    triple_writer.max_depth = triple_writer.max_depth.max(max_depth);
    if !buffer.is_empty() {
        triple_writer.write_all(&buffer)?;
    }
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();
    let args = Args::parse();
    let config = Config::load_from_file(&args.config)?;

    info!("\x1b[1mStarting conversion of DICOM SR to raw RDF Turtle\x1b[0m");
    let worker_id = AtomicUsize::new(0);
    let (progress_tx, progress_rx) = mpsc::channel::<()>();
    let progress_logger = mk_progress_logger(progress_rx);

    get_dcm_or_zst_paths(args.input_dir.as_path())
        .par_bridge()
        .for_each_init(
            || {
                let worker_name =
                    format!("raw-dicom-{:03}", worker_id.fetch_add(1, Ordering::Relaxed));
                let triple_writer = TripleWriter::new(mk_ttl_gz_writer(
                    &args.output_dir,
                    &format!("{}.ttl.gz", worker_name),
                ));
                let error_writer =
                    mk_writer(&args.output_dir, &format!("{}-errors.log", worker_name));
                let progress_tx = progress_tx.clone();
                (triple_writer, error_writer, progress_tx)
            },
            |(triple_writer, error_writer, progress_tx), path| {
                if let Err(e) = convert_file(triple_writer, error_writer, &path, &config) {
                    warn!("Failed to convert file {:?}: {}", path, e)
                }
                progress_tx.send(()).expect("Send failed");
            },
        );
    drop(progress_tx);
    progress_logger.join().expect("Thread panicked");

    Ok(())
}
