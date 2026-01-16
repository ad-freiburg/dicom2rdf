#[global_allocator]
static GLOBAL: tikv_jemallocator::Jemalloc = tikv_jemallocator::Jemalloc;

use clap::{CommandFactory, Parser};
use config::Config;
use convert::dicom::write_triples;
use convert::merge::merge_chunks;
use convert::path::{get_dcm_or_zst_paths, resolve_to_dicom_path};
use convert::progress::progress_logger;
use convert::turtle;
use convert::writer::TripleWriter;
use dicom::object::open_file;
use log::{info, warn};
use rayon::prelude::*;
use std::io::Write;
use std::path::{Path, PathBuf};

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

    /// Number of triples per chunk file
    #[arg(long, required = true)]
    chunk_size: usize,

    /// Maximum triples per final output file, must be multiple of chunk_size.
    #[arg(long, required = true)]
    max_triples_per_file: usize,

    /// Gzip compression level (0-9)
    #[arg(long, required = true, value_parser = clap::value_parser!(u32).range(0..=9))]
    compression_level: u32,

    /// Number of rayon worker threads (default: whatever rayon defaults to)
    #[arg(long)]
    num_threads: Option<usize>,
}

impl Args {
    fn validate(self) -> Self {
        if self.max_triples_per_file % self.chunk_size != 0 {
            let mut cmd = Args::command();
            cmd.error(
                clap::error::ErrorKind::ValueValidation,
                "max_triples_per_file ({}) must be a multiple of chunk_size ({})",
            )
            .exit();
        };
        self
    }
}

fn convert_file<P: AsRef<Path>>(
    triple_writer: &mut TripleWriter,
    path: P,
    config: &Config,
) -> Result<(), Box<dyn std::error::Error>> {
    let (dicom_file_path, _temp_dir_guard) = resolve_to_dicom_path(path)?;

    let file_name = dicom_file_path
        .file_name()
        .and_then(|s| s.to_str())
        .ok_or("Failed to get DICOM file name")?;
    let dicom_object = open_file(&dicom_file_path)?;
    let file_subject = turtle::IRI::prefix("dicom2rdf", file_name);
    let mut buffer = Vec::new();
    writeln!(
        &mut buffer,
        "{}",
        turtle::triple(
            &file_subject,
            &turtle::RDF_TYPE_IRI,
            &turtle::TripleObject::from(turtle::IRI::prefix("dicom2rdf", "DocumentRoot")),
        )
    )?;

    let (_, max_depth) = write_triples(
        &mut buffer,
        triple_writer.log_writer(),
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

fn clear_output_dir<P: AsRef<Path>>(output_dir: P) -> std::io::Result<()> {
    for entry in std::fs::read_dir(output_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            std::fs::remove_dir_all(&path)?;
        } else {
            std::fs::remove_file(&path)?;
        }
    }
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();
    let args = Args::parse().validate();

    if let Some(num_threads) = args.num_threads {
        rayon::ThreadPoolBuilder::new()
            .num_threads(num_threads)
            .build_global()?;
    }

    let config = Config::load_from_file(&args.config)?;

    clear_output_dir(&args.output_dir)?;

    info!("\x1b[1mStarting conversion of DICOM SR to raw RDF Turtle\x1b[0m");
    let (progress_sender, progress_logger_thread) = progress_logger();

    get_dcm_or_zst_paths(args.input_dir.as_path())
        .par_bridge()
        .for_each_init(
            || {
                let triple_writer = TripleWriter::new(
                    &args.output_dir,
                    "raw-dicom",
                    args.chunk_size,
                    args.compression_level,
                )
                .expect("Failed to create TripleWriter");
                (triple_writer, progress_sender.clone())
            },
            |(triple_writer, progress_sender), path| {
                if let Err(e) = convert_file(triple_writer, &path, &config) {
                    warn!("Failed to convert file {:?}: {}", path, e)
                }
                progress_sender.tick();
            },
        );
    drop(progress_sender);
    progress_logger_thread.join().expect("Thread panicked");

    info!("\x1b[1mMerging chunk files\x1b[0m");
    merge_chunks(
        &args.output_dir,
        args.chunk_size,
        args.max_triples_per_file,
        args.compression_level,
    )?;

    Ok(())
}
