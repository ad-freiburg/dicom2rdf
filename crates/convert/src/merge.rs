use std::fs::{self, File};
use std::io::{self, BufReader, BufWriter, Write};
use std::path::{Path, PathBuf};

use flate2::Compression;
use flate2::write::GzEncoder;

const WELL_KNOWN_PREFIXES: &[u8] =
    br#"@prefix dicom2rdf: <http://dicom2rdf.uniklinik-freiburg.de/> .
@prefix rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#> .
@prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .
"#;

struct ChunkInfo {
    ttl_path: PathBuf,
    max_depth: u8,
    triple_count: usize,
}

fn read_chunk_metadata<P>(meta_path: P) -> io::Result<(u8, usize)>
where
    P: AsRef<Path>,
{
    let content = fs::read_to_string(meta_path)?;
    let mut parts = content.split_whitespace();
    let max_depth: u8 = parts
        .next()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "Missing max_depth in metadata"))?
        .parse()
        .map_err(|e| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Invalid max_depth: {}", e),
            )
        })?;
    let triple_count: usize = parts
        .next()
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                "Missing triple_count in metadata",
            )
        })?
        .parse()
        .map_err(|e| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Invalid triple_count: {}", e),
            )
        })?;
    Ok((max_depth, triple_count))
}

fn collect_chunk_infos<P>(output_dir: P) -> io::Result<Vec<ChunkInfo>>
where
    P: AsRef<Path>,
{
    let mut chunks: Vec<ChunkInfo> = Vec::new();
    for entry in fs::read_dir(&output_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) == Some("meta") {
            let (max_depth, triple_count) = read_chunk_metadata(&path)?;
            let ttl_path = path.with_extension("").with_extension("ttl.gz");
            if ttl_path.exists() {
                chunks.push(ChunkInfo {
                    ttl_path,
                    max_depth,
                    triple_count,
                });
            }
        }
    }
    chunks.sort_by(|a, b| a.ttl_path.cmp(&b.ttl_path));
    Ok(chunks)
}

fn group_chunks(chunks: &[ChunkInfo], max_triples_per_file: usize) -> Vec<Vec<&ChunkInfo>> {
    let mut groups: Vec<Vec<&ChunkInfo>> = Vec::new();
    let mut current_group: Vec<&ChunkInfo> = Vec::new();
    let mut current_count: usize = 0;

    for chunk in chunks {
        if !current_group.is_empty() && current_count + chunk.triple_count > max_triples_per_file {
            groups.push(current_group);
            current_group = Vec::new();
            current_count = 0;
        }
        current_group.push(chunk);
        current_count += chunk.triple_count;
    }
    if !current_group.is_empty() {
        groups.push(current_group);
    }
    groups
}

pub fn merge_chunks<P>(
    output_dir: P,
    max_triples_per_file: usize,
    compression_level: u32,
) -> io::Result<()>
where
    P: AsRef<Path>,
{
    let chunks = collect_chunk_infos(&output_dir)?;
    if chunks.is_empty() {
        return Ok(());
    }

    let groups = group_chunks(&chunks, max_triples_per_file);

    for (group_idx, group) in groups.iter().enumerate() {
        let output_path = output_dir
            .as_ref()
            .join(format!("{:03}-raw-dicom-merged.ttl.gz", group_idx));
        let group_max_depth = group.iter().map(|c| c.max_depth).max().unwrap_or(0);
        let ttl_paths: Vec<&Path> = group.iter().map(|c| c.ttl_path.as_path()).collect();
        concatenate_files(&ttl_paths, &output_path, compression_level, group_max_depth)?;
    }

    merge_log_files(&output_dir, &groups)?;

    for chunk in &chunks {
        fs::remove_file(&chunk.ttl_path)?;
        let meta_path = chunk.ttl_path.with_extension("").with_extension("meta");
        let _ = fs::remove_file(meta_path);
    }

    Ok(())
}

fn concatenate_files(
    input_files: &[&Path],
    output_path: &Path,
    compression_level: u32,
    max_depth: u8,
) -> io::Result<()> {
    let output_file = File::create(output_path)?;
    let mut writer = BufWriter::new(output_file);

    let mut gz = GzEncoder::new(&mut writer, Compression::new(compression_level));
    gz.write_all(WELL_KNOWN_PREFIXES)?;
    gz.finish()?;

    for input_path in input_files {
        let input_file = File::open(input_path)?;
        let mut reader = BufReader::new(input_file);
        io::copy(&mut reader, &mut writer)?;
    }

    let max_depth_triple = format!("<> <meta:maxDepth> {} .\n", max_depth);
    let mut gz = GzEncoder::new(&mut writer, Compression::new(compression_level));
    gz.write_all(max_depth_triple.as_bytes())?;
    gz.finish()?;

    writer.flush()?;
    Ok(())
}

fn merge_log_files<P>(output_dir: P, groups: &[Vec<&ChunkInfo>]) -> io::Result<()>
where
    P: AsRef<Path>,
{
    for (group_idx, group) in groups.iter().enumerate() {
        let output_path = output_dir
            .as_ref()
            .join(format!("{:03}-raw-dicom-merged.log", group_idx));
        let output_file = File::create(&output_path)?;
        let mut writer = BufWriter::new(output_file);

        for chunk in group.iter() {
            let log_path = chunk.ttl_path.with_extension("").with_extension("log");
            if log_path.exists() {
                let input_file = File::open(&log_path)?;
                let mut reader = BufReader::new(input_file);
                io::copy(&mut reader, &mut writer)?;
                fs::remove_file(&log_path)?;
            }
        }
        writer.flush()?;
    }
    Ok(())
}
