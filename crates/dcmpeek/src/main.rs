use std::{
    collections::HashSet,
    error::Error,
    io::{self, BufReader, Cursor, Read},
    sync::LazyLock,
};

use dicom::core::{DicomValue, VR};
use dicom::object::{InMemDicomObject, from_reader};
use tar::Archive;

const ZSTD_MAGIC: [u8; 4] = [0x28, 0xB5, 0x2F, 0xFD];
static VRS_TO_IGNORE: LazyLock<HashSet<VR>> = LazyLock::new(|| HashSet::from([VR::OB, VR::UN]));

fn print_data_element(dcm_obj: &InMemDicomObject, indent: usize) {
    let padding = " ".repeat(indent);
    for x in dcm_obj.iter() {
        print!("{}{} ({}) ", padding, x.header().tag, x.header().vr());

        if VRS_TO_IGNORE.contains(&x.vr()) {
            println!("<{}>", x.vr());
            continue;
        }

        match x.value() {
            DicomValue::Primitive(pv) => println!("{}", pv),
            DicomValue::Sequence(dss) => {
                println!();
                for item in dss.items() {
                    println!("{}    ---", padding);
                    print_data_element(item, indent + 4);
                    println!("{}    ---", padding);
                }
            }
            DicomValue::PixelSequence(_) => println!("<PixelSequence>"),
        }
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let mut stdin = BufReader::new(io::stdin().lock());
    let mut magic = [0u8; 4];
    stdin.read_exact(&mut magic)?;
    let raw_stream = Cursor::new(magic).chain(stdin);
    let dcm_obj = if magic == ZSTD_MAGIC {
        let decoder = zstd::Decoder::new(raw_stream)?;
        let mut archive = Archive::new(decoder);
        let entry = archive.entries()?.next().ok_or("Empty archive")??;
        from_reader(entry)?
    } else {
        from_reader(raw_stream)?
    };
    print_data_element(&dcm_obj, 0);
    Ok(())
}
