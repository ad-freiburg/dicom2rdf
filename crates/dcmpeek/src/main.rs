use std::io::{self, Cursor, Read};

use dicom::dump::dump_object;
use dicom::object::from_reader;
use tar::Archive;

const ZSTD_MAGIC: [u8; 4] = [0x28, 0xB5, 0x2F, 0xFD];

fn main() {
    let mut input: Vec<u8> = Vec::new();
    io::stdin()
        .read_to_end(&mut input)
        .expect("Failed to read from stdin");
    let dcm_obj = if input.starts_with(&ZSTD_MAGIC) {
        let decoder = zstd::Decoder::new(Cursor::new(input)).expect("Invalid zstd stream");
        let mut archive = Archive::new(decoder);
        let mut entries = archive.entries().expect("Corrupt tar archive");
        let entry = entries
            .next()
            .expect("Empty archive")
            .expect("Corrupt tar entry");
        from_reader(entry)
    } else {
        from_reader(Cursor::new(input))
    }
    .expect("Failed to read as DICOM object");
    dump_object(&dcm_obj).expect("Failed to dump DICOM object");
}
