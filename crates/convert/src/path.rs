use std::path::Path;
use std::{
    error::Error,
    fs::{self, File},
    path::PathBuf,
};
use tar::Archive;
use tempfile::{TempDir, tempdir};
use walkdir::WalkDir;

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

pub fn resolve_to_dicom_path<P: AsRef<Path>>(
    path: P,
) -> Result<(PathBuf, Option<TempDir>), Box<dyn Error>> {
    let (dicom_file_path, _temp_dir_guard) =
        if path.as_ref().extension().and_then(|s| s.to_str()) == Some("zst") {
            handle_zst_file(&path)?
        } else {
            (path.as_ref().to_path_buf(), None)
        };
    Ok((dicom_file_path, _temp_dir_guard))
}
