use std::fs::{self, File};
use std::io::{self, BufReader, BufWriter, Write};
use std::path::Path;

use flate2::Compression;
use flate2::write::GzEncoder;

const WELL_KNOWN_PREFIXES: &[u8] =
    br#"@prefix dicom2rdf: <http://dicom2rdf.uniklinik-freiburg.de/> .
@prefix rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#> .
@prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .
"#;

pub fn merge_chunks<P>(
    output_dir: P,
    chunk_size: usize,
    max_triples_per_file: usize,
    compression_level: u32,
) -> io::Result<()>
where
    P: AsRef<Path>,
{
    let mut ttl_gz_files: Vec<_> = fs::read_dir(&output_dir)?
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| {
            path.file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.ends_with(".ttl.gz"))
        })
        .collect();
    if ttl_gz_files.is_empty() {
        return Ok(());
    }
    ttl_gz_files.sort();
    let files_per_group = max_triples_per_file / chunk_size;
    let chunks: Vec<_> = ttl_gz_files.chunks(files_per_group).collect();
    for (group_idx, chunk) in chunks.into_iter().enumerate() {
        let output_path = output_dir
            .as_ref()
            .join(format!("{:03}-raw-dicom-merged.ttl.gz", group_idx));
        let group_max_depth = chunk
            .iter()
            .filter_map(|path| depth_from_file_name(path))
            .max()
            .unwrap_or(0);
        concatenate_files(chunk, &output_path, compression_level, group_max_depth)?;
    }
    for file in &ttl_gz_files {
        fs::remove_file(file)?;
    }
    merge_log_files(&output_dir, files_per_group)?;
    Ok(())
}

fn depth_from_file_name<P>(path: P) -> Option<u8>
where
    P: AsRef<Path>,
{
    let filename = path.as_ref().file_name()?.to_str()?;
    let stem = filename.strip_suffix(".ttl.gz")?;
    let depth_part = stem.rsplit("max-depth-").next()?;
    depth_part.parse().ok()
}

fn concatenate_files<P>(
    input_files: &[P],
    output_path: &Path,
    compression_level: u32,
    max_depth: u8,
) -> io::Result<()>
where
    P: AsRef<Path>,
{
    let output_file = File::create(output_path)?;
    let mut writer = BufWriter::new(output_file);
    {
        let mut gz = GzEncoder::new(&mut writer, Compression::new(compression_level));
        gz.write_all(WELL_KNOWN_PREFIXES)?;
        gz.finish()?;
    }
    for input_path in input_files {
        let input_file = File::open(input_path.as_ref())?;
        let mut reader = BufReader::new(input_file);
        io::copy(&mut reader, &mut writer)?;
    }
    {
        let max_depth_triple = format!("<> <meta:maxDepth> {} .\n", max_depth);
        let mut gz = GzEncoder::new(&mut writer, Compression::new(compression_level));
        gz.write_all(max_depth_triple.as_bytes())?;
        gz.finish()?;
    }

    writer.flush()?;
    Ok(())
}

fn merge_log_files<P>(output_dir: P, files_per_group: usize) -> io::Result<()>
where
    P: AsRef<Path>,
{
    let mut log_files: Vec<_> = fs::read_dir(&output_dir)?
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| {
            path.extension()
                .and_then(|e| e.to_str())
                .map(|e| e == "log")
                .unwrap_or(false)
        })
        .collect();
    if log_files.is_empty() {
        return Ok(());
    }
    log_files.sort();
    for (group_idx, chunk) in log_files.chunks(files_per_group).enumerate() {
        let output_path = output_dir
            .as_ref()
            .join(format!("{:03}-raw-dicom-merged.log", group_idx));
        let output_file = File::create(&output_path)?;
        let mut writer = BufWriter::new(output_file);
        for input_path in chunk {
            let input_file = File::open(input_path)?;
            let mut reader = BufReader::new(input_file);
            io::copy(&mut reader, &mut writer)?;
        }
        writer.flush()?;
    }
    for file in &log_files {
        fs::remove_file(file)?;
    }
    Ok(())
}
