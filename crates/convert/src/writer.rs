use std::fs::File;
use std::io::{self, BufWriter, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

use flate2::Compression;

const WELL_KNOWN_PREFIXES: &[u8] =
    br#"@prefix dicom2rdf: <http://dicom2rdf.uniklinik-freiburg.de/> .
@prefix rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#> .
@prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .
"#;

const WRITER_BUFFER_SIZE: usize = 8192 * 8;

static WRITER_ID: AtomicUsize = AtomicUsize::new(0);

type GzWriter = flate2::write::GzEncoder<BufWriter<File>>;
type LogWriter = BufWriter<File>;

fn writers<P: AsRef<Path>>(
    destination: P,
    name: &str,
    compression_level: u32,
) -> io::Result<(GzWriter, LogWriter)> {
    std::fs::create_dir_all(&destination).map_err(|e| {
        io::Error::new(
            e.kind(),
            format!(
                "Failed to create writer destination path '{:?}': {}",
                destination.as_ref(),
                e
            ),
        )
    })?;
    let prefix = format!("{:03}", WRITER_ID.fetch_add(1, Ordering::Relaxed));
    let triple_file = File::create(
        destination
            .as_ref()
            .join(&format!("{}-{}.ttl.gz", prefix, name)),
    )?;
    let mut triple_writer = flate2::write::GzEncoder::new(
        io::BufWriter::new(triple_file),
        Compression::new(compression_level),
    );

    let log_file = File::create(
        destination
            .as_ref()
            .join(&format!("{}-{}.log", prefix, name)),
    )?;
    let log_writer = io::BufWriter::new(log_file);

    triple_writer.write_all(WELL_KNOWN_PREFIXES).map_err(|e| {
        io::Error::new(
            e.kind(),
            "Failed to write well-known prefixes to triple writer",
        )
    })?;
    Ok((triple_writer, log_writer))
}

pub struct TripleWriter {
    pub max_depth: u8,

    triple_writer: GzWriter,
    triple_buffer: Vec<u8>,
    log_writer: LogWriter,

    name: String,
    destination: PathBuf,
    bytes_written: usize,
    max_ttl_file_size: usize,
    compression_level: u32,
}

impl TripleWriter {
    pub fn new<P: AsRef<Path>>(
        destination: P,
        name: &str,
        max_ttl_file_size: usize,
        compression_level: u32,
    ) -> io::Result<Self> {
        let (writer, log_writer) = writers(&destination, name, compression_level)?;
        Ok(TripleWriter {
            triple_buffer: Vec::new(),
            bytes_written: 0,
            destination: destination.as_ref().to_path_buf(),
            name: String::from(name),
            max_ttl_file_size,
            compression_level,
            max_depth: 0,
            triple_writer: writer,
            log_writer,
        })
    }

    pub fn log_writer(&mut self) -> &mut LogWriter {
        &mut self.log_writer
    }

    fn rotate(&mut self) -> io::Result<()> {
        self.write_max_depth_triple()?;
        self.triple_writer.flush()?;
        self.log_writer.flush()?;

        let (triple_writer, log_writer) =
            writers(&self.destination, &self.name, self.compression_level)?;

        self.triple_writer = triple_writer;
        self.log_writer = log_writer;
        self.bytes_written = 0;

        Ok(())
    }

    fn write_max_depth_triple(&mut self) -> io::Result<()> {
        let max_depth_triple = format!("<> <meta:maxDepth> {} .\n", self.max_depth);
        self.triple_writer.write_all(max_depth_triple.as_bytes())
    }
}

impl io::Write for TripleWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.triple_buffer.extend_from_slice(buf);

        if self.triple_buffer.len() >= WRITER_BUFFER_SIZE {
            self.flush()?;
        }
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        self.triple_writer.write_all(&self.triple_buffer)?;
        self.triple_writer.flush()?;
        self.bytes_written += self.triple_buffer.len();
        self.triple_buffer.clear();
        if self.bytes_written >= self.max_ttl_file_size {
            self.rotate()?;
        }
        Ok(())
    }
}

impl Drop for TripleWriter {
    fn drop(&mut self) {
        let _ = self.triple_writer.write_all(&self.triple_buffer);
        let _ = self.write_max_depth_triple();
    }
}
