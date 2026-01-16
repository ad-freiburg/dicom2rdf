use std::fs::{self, File};
use std::io::{self, BufWriter, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

use flate2::Compression;

const WRITER_BUFFER_SIZE: usize = 8192 * 8;

static WRITER_ID: AtomicUsize = AtomicUsize::new(0);

type GzWriter = flate2::write::GzEncoder<BufWriter<File>>;
type LogWriter = BufWriter<File>;

fn writers<P: AsRef<Path>>(
    destination: P,
    name: &str,
    prefix: &str,
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
    let triple_file = File::create(
        destination
            .as_ref()
            .join(format!("{}-{}.ttl.gz", prefix, name)),
    )?;
    let triple_writer = flate2::write::GzEncoder::new(
        io::BufWriter::new(triple_file),
        Compression::new(compression_level),
    );

    let log_file = File::create(
        destination
            .as_ref()
            .join(&format!("{}-{}.log", prefix, name)),
    )?;
    let log_writer = io::BufWriter::new(log_file);

    Ok((triple_writer, log_writer))
}

fn next_prefix() -> String {
    format!("{:03}", WRITER_ID.fetch_add(1, Ordering::Relaxed))
}

pub struct TripleWriter {
    pub max_depth: u8,

    triple_writer: GzWriter,
    triple_buffer: Vec<u8>,
    log_writer: LogWriter,

    name: String,
    destination: PathBuf,
    current_prefix: String,
    triples_written: usize,
    chunk_size: usize,
    compression_level: u32,
}

impl TripleWriter {
    pub fn new<P: AsRef<Path>>(
        destination: P,
        name: &str,
        chunk_size: usize,
        compression_level: u32,
    ) -> io::Result<Self> {
        let prefix = next_prefix();
        let (writer, log_writer) = writers(&destination, name, &prefix, compression_level)?;
        Ok(TripleWriter {
            triple_buffer: Vec::new(),
            triples_written: 0,
            destination: destination.as_ref().to_path_buf(),
            name: String::from(name),
            current_prefix: prefix,
            chunk_size,
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
        self.triple_writer.flush()?;
        self.log_writer.flush()?;
        self.write_chunk_metadata()?;
        let new_prefix = next_prefix();

        let (triple_writer, log_writer) = writers(
            &self.destination,
            &self.name,
            &new_prefix,
            self.compression_level,
        )?;

        self.triple_writer = triple_writer;
        self.log_writer = log_writer;
        self.current_prefix = new_prefix;
        self.triples_written = 0;
        self.max_depth = 0;

        Ok(())
    }

    fn write_chunk_metadata(&self) -> io::Result<()> {
        let meta_path = self
            .destination
            .join(format!("{}-{}.meta", self.current_prefix, self.name));
        fs::write(
            meta_path,
            format!("{} {}", self.max_depth, self.triples_written),
        )?;
        Ok(())
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
        let triple_count = self.triple_buffer.iter().filter(|&&b| b == b'\n').count();
        self.triple_writer.write_all(&self.triple_buffer)?;
        self.triple_writer.flush()?;
        self.triples_written += triple_count;
        self.triple_buffer.clear();
        // NOTE: This means we overshoot chunk_size a little bit, but that's
        // fine.
        if self.triples_written >= self.chunk_size {
            self.rotate()?;
        }
        Ok(())
    }
}

impl Drop for TripleWriter {
    fn drop(&mut self) {
        let _ = self.triple_writer.write_all(&self.triple_buffer);
        let _ = self.triple_writer.flush();
        self.triples_written += self.triple_buffer.iter().filter(|&&b| b == b'\n').count();
        let _ = self.write_chunk_metadata();
    }
}
