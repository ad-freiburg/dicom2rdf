use std::io::{IsTerminal, Read};

const ZSTD_MAGIC: [u8; 4] = [0x28, 0xB5, 0x2F, 0xFD];

fn main() {
    if std::io::stdin().is_terminal() {
        eprintln!("Usage: dcmpeek <file.dcm");
        eprintln!("       dcmpeek <file.tar.zst");
        std::process::exit(1);
    }
    let mut input: Vec<u8> = Vec::new();
    std::io::stdin()
        .read_to_end(&mut input)
        .expect("Failed to read from stdin");
    let dcm_obj = if input.starts_with(&ZSTD_MAGIC) {
        let decoder = zstd::Decoder::new(std::io::Cursor::new(input)).expect("Invalid zstd stream");
        let mut archive = tar::Archive::new(decoder);
        let mut entries = archive.entries().expect("Corrupt tar archive");
        let entry = entries
            .next()
            .expect("Empty archive")
            .expect("Corrupt tar entry");
        dicom::object::from_reader(entry)
    } else {
        dicom::object::from_reader(std::io::Cursor::new(input))
    }
    .expect("Failed to read as DICOM object");
    dicom::dump::dump_object(&dcm_obj).expect("Failed to dump DICOM object");
}
