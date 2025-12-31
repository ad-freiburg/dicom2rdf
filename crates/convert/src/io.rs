use flate2::{Compression, write::GzEncoder};
use std::{
    error::Error,
    fs::{self, File},
    io::{BufWriter, Write},
    path::{Path, PathBuf},
};
use tar::Archive;
use tempfile::{TempDir, tempdir};
use walkdir::WalkDir;

pub struct TripleWriter<W: Write> {
    writer: W,
    pub max_depth: u8,
}

impl<W: Write> TripleWriter<W> {
    pub fn new(writer: W) -> Self {
        Self {
            writer,
            max_depth: 0,
        }
    }
}

impl<W: Write> Drop for TripleWriter<W> {
    fn drop(&mut self) {
        writeln!(self.writer, "<> <meta:maxDepth> {} .", self.max_depth).ok();
    }
}

impl<W: Write> Write for TripleWriter<W> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.writer.write(buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.writer.flush()
    }
}

pub fn writer<P: AsRef<Path>>(output_dir: P, file_name: &str) -> BufWriter<File> {
    fs::create_dir_all(&output_dir).expect(&format!(
        "Failed to create output directory '{}'",
        output_dir.as_ref().display()
    ));
    let file_path = output_dir.as_ref().join(file_name);
    let file = File::create(&file_path).expect(&format!(
        "Failed to create writer file '{}' in '{:?}'",
        file_name,
        output_dir.as_ref()
    ));
    BufWriter::new(file)
}

pub fn ttl_gz_writer<P: AsRef<Path>>(output_dir: P, file_name: &str) -> GzEncoder<BufWriter<File>> {
    let writer = writer(output_dir.as_ref(), file_name);
    let mut encoder = GzEncoder::new(writer, Compression::fast());
    Write::write_all(
        &mut encoder,
        [
            "@prefix dicom2rdf: <http://dicom2rdf.uniklinik-freiburg.de/> .",
            "@prefix rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#> .",
            "@prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .",
            "@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .",
        ]
        .join("\n")
        .as_bytes(),
    )
    .expect("Failed to write well-known prefixes");
    encoder
}

pub fn handle_zst_file<P: AsRef<Path>>(
    path: P,
) -> Result<(PathBuf, Option<TempDir>), Box<dyn Error>> {
    let path = path.as_ref();
    let temp_dir = tempdir()?;
    let tar_zst_file = File::open(&path)?;
    let decoder = zstd::stream::read::Decoder::new(tar_zst_file)?;
    let mut archive = Archive::new(decoder);
    archive.unpack(temp_dir.path())?;

    let mut entries = fs::read_dir(temp_dir.path())?;
    match entries
        .next()
        .and_then(|entry| entry.ok())
        .map(|entry| entry.path())
    {
        Some(p) => Ok((p, Some(temp_dir))),
        None => Err(format!("No file found in archive {}", path.display()).into()),
    }
}

fn is_dcm_or_zst(de: &walkdir::DirEntry) -> bool {
    if !de.file_type().is_file() {
        return false;
    };
    match de.path().extension().and_then(|s| s.to_str()) {
        Some("dcm") | Some("zst") => true,
        _ => false,
    }
}

pub fn get_dcm_or_zst_paths<P: AsRef<Path>>(root: P) -> impl Iterator<Item = PathBuf> {
    let wd = WalkDir::new(root);
    wd.into_iter()
        .filter_map(|res| res.ok())
        .filter(is_dcm_or_zst)
        .map(|e| e.path().to_path_buf())
}
