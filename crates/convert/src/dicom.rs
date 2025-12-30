use crate::datetime::{
    age_string_to_years, date_to_iso, datetime_to_iso, iso_string_to_typed_literal, time_to_iso,
};
use crate::turtle;
use config::Config;
use dicom::core::header::{HasLength, Header};
use dicom::core::{Tag, VR};
use dicom::object::InMemDicomObject;
use log::debug;
use std::borrow::Cow;
use std::error::Error;
use std::io::Write;
use std::sync::LazyLock;

static INDEX_IRI: LazyLock<turtle::IRI> =
    LazyLock::new(|| turtle::IRI::prefix("dicom2rdf", "index"));
static ITEM_IRI: LazyLock<turtle::IRI> = LazyLock::new(|| turtle::IRI::prefix("dicom2rdf", "item"));
static PERSON_NAME_IRI: LazyLock<turtle::IRI> =
    LazyLock::new(|| turtle::IRI::prefix("dicom2rdf", "person_name"));
static PN_FAMILY_IRI: LazyLock<turtle::IRI> =
    LazyLock::new(|| turtle::IRI::prefix("dicom2rdf", "pn_family"));
static PN_MIDDLE_IRI: LazyLock<turtle::IRI> =
    LazyLock::new(|| turtle::IRI::prefix("dicom2rdf", "pn_middle"));
static PN_GIVEN_IRI: LazyLock<turtle::IRI> =
    LazyLock::new(|| turtle::IRI::prefix("dicom2rdf", "pn_given"));
static PN_PREFIX_IRI: LazyLock<turtle::IRI> =
    LazyLock::new(|| turtle::IRI::prefix("dicom2rdf", "pn_prefix"));
static PN_SUFFIX_IRI: LazyLock<turtle::IRI> =
    LazyLock::new(|| turtle::IRI::prefix("dicom2rdf", "pn_suffix"));

pub fn write_triples(
    triple_writer: &mut impl Write,
    error_writer: &mut impl Write,
    subject: &turtle::IRI,
    dicom_object: &InMemDicomObject,
    file_name: &str,
    config: &Config,
    depth: u8,
) -> (Option<String>, u8) {
    let mut max_depth_seen = depth;
    let mut carry: Option<String> = None;
    for data_element in dicom_object.iter() {
        if data_element.value().is_empty() {
            continue;
        }
        let group = data_element.header().tag.group();
        let element = data_element.header().tag.element();
        let predicate = turtle::IRI::prefix("dicom2rdf", format!("{:04X}{:04X}", group, element,));
        if group == 0x7FE0 && element == 0x0010 {
            debug!("VR: {}", data_element.vr());
            let object =
                turtle::TripleObject::from(turtle::PlainLiteral::String("<pixel data>".into()));
            let _ = writeln!(
                triple_writer,
                "{}",
                turtle::triple(subject, &predicate, &object)
            );
            continue;
        }
        if config
            .forbidden_dicom_tags
            .contains(&data_element.header().tag())
        {
            let _ = writeln!(
                triple_writer,
                "{}",
                turtle::triple(
                    subject,
                    &predicate,
                    &turtle::TripleObject::PlainLiteral(turtle::PlainLiteral::String(format!(
                        "<({:04X},{:04X})>",
                        group, element,
                    )))
                )
            );
            continue;
        }
        if let Err(e) = (|| -> Result<(), Box<dyn Error>> {
            match data_element.vr() {
                VR::AS => {
                    let age_str = data_element.value().string()?.trim();
                    let years = age_string_to_years(age_str)?;
                    let object = turtle::TripleObject::from(turtle::PlainLiteral::Float(years));
                    writeln!(
                        triple_writer,
                        "{}",
                        turtle::triple(subject, &predicate, &object)
                    )?;
                }
                VR::DA => {
                    for val in data_element.value().to_multi_date()? {
                        let object = turtle::TripleObject::from(iso_string_to_typed_literal(
                            &date_to_iso(&val),
                        ));
                        writeln!(
                            triple_writer,
                            "{}",
                            turtle::triple(subject, &predicate, &object)
                        )?;
                    }
                }
                VR::DT => {
                    for val in data_element.value().to_multi_datetime()? {
                        let object = turtle::TripleObject::from(iso_string_to_typed_literal(
                            &datetime_to_iso(&val),
                        ));
                        writeln!(
                            triple_writer,
                            "{}",
                            turtle::triple(subject, &predicate, &object)
                        )?;
                    }
                }
                VR::TM => {
                    for val in data_element.value().to_multi_time()? {
                        let object = turtle::TripleObject::from(turtle::TypedLiteral::new(
                            time_to_iso(&val),
                            turtle::IRI::prefix("xsd", "time"),
                        ));
                        writeln!(
                            triple_writer,
                            "{}",
                            turtle::triple(subject, &predicate, &object)
                        )?;
                    }
                }
                VR::AE | VR::CS | VR::LT | VR::ST | VR::UI => {
                    for val in data_element.value().strings()? {
                        let s = val.trim().trim_end_matches('\0');
                        let object =
                            turtle::TripleObject::from(turtle::PlainLiteral::String(s.into()));
                        writeln!(
                            triple_writer,
                            "{}",
                            turtle::triple(subject, &predicate, &object)
                        )?;
                    }
                }
                VR::LO => {
                    for val in data_element.value().strings()? {
                        let s = val.trim().trim_end_matches('\0');
                        let object =
                            turtle::TripleObject::from(turtle::PlainLiteral::String(s.into()));
                        writeln!(
                            triple_writer,
                            "{}",
                            turtle::triple(subject, &predicate, &object)
                        )?;
                        if data_element.header().tag() == Tag(0x0008, 0x0104)
                            && config.forbidden_code_meanings.contains(s)
                        {
                            carry = Some(s.to_string());
                        }
                    }
                }
                VR::SH => {
                    for val in data_element.value().strings()? {
                        let val = val.trim().trim_end_matches('\0');
                        let object = if group == 0x0008 && element == 0x0102 {
                            turtle::TripleObject::from(turtle::IRI::full(
                                config
                                    .dicom
                                    .iter()
                                    .find(|x| x.coding_scheme == val)
                                    .map(|x| Cow::from(String::from(&x.iri)))
                                    .unwrap_or(Cow::Owned(format!(
                                        "{}{}_",
                                        &config.fallback.iri,
                                        urlencoding::encode(val)
                                    ))),
                            ))
                        } else {
                            turtle::TripleObject::from(turtle::PlainLiteral::String(val.into()))
                        };
                        writeln!(
                            triple_writer,
                            "{}",
                            turtle::triple(subject, &predicate, &object)
                        )?;
                    }
                }
                VR::DS => {
                    // TODO: DS does potentially not fit in f64 - should probably use rust_decimal.
                    for val in data_element.value().strings()? {
                        let parsed_val: f64 = val.trim().parse()?;
                        if parsed_val.is_nan() || parsed_val.is_infinite() {
                            // Create NaN or inf literals once https://github.com/ad-freiburg/qlever/issues/2303 is fixed.
                            continue;
                        }
                        let object =
                            turtle::TripleObject::from(turtle::PlainLiteral::Float(parsed_val));
                        writeln!(
                            triple_writer,
                            "{}",
                            turtle::triple(subject, &predicate, &object)
                        )?;
                    }
                }
                VR::FL => {
                    for val in data_element.value().float32_slice()? {
                        let object =
                            turtle::TripleObject::from(turtle::PlainLiteral::Float(*val as f64));
                        writeln!(
                            triple_writer,
                            "{}",
                            turtle::triple(subject, &predicate, &object)
                        )?;
                    }
                }
                VR::FD => {
                    for val in data_element.value().float64_slice()? {
                        let object = if val.is_nan() || val.is_infinite() {
                            // Create NaN or inf literals once https://github.com/ad-freiburg/qlever/issues/2303 is fixed.
                            continue;
                        } else {
                            turtle::TripleObject::from(turtle::PlainLiteral::Float(*val))
                        };
                        writeln!(
                            triple_writer,
                            "{}",
                            turtle::triple(subject, &predicate, &object)
                        )?;
                    }
                }
                VR::IS => {
                    for val in data_element.value().strings()? {
                        let parsed: i64 = val.trim().parse()?;
                        let object =
                            turtle::TripleObject::from(turtle::PlainLiteral::Integer(parsed));
                        writeln!(
                            triple_writer,
                            "{}",
                            turtle::triple(subject, &predicate, &object)
                        )?;
                    }
                }
                VR::OW => {
                    let object = turtle::TripleObject::from(turtle::PlainLiteral::String(
                        "<OW>".to_string(),
                    ));
                    writeln!(
                        triple_writer,
                        "{}",
                        turtle::triple(subject, &predicate, &object)
                    )?;
                }
                VR::PN => {
                    let pn = data_element.value().to_person_name()?;
                    let bn = turtle::create_blank_node();

                    writeln!(
                        triple_writer,
                        "{}",
                        turtle::triple(
                            subject,
                            &*PERSON_NAME_IRI,
                            &turtle::TripleObject::from(bn.clone())
                        )
                    )?;
                    if let Some(family) = pn.family() {
                        let object = turtle::TripleObject::from(turtle::PlainLiteral::String(
                            family.to_string(),
                        ));
                        writeln!(
                            triple_writer,
                            "{}",
                            turtle::triple(&bn, &*PN_FAMILY_IRI, &object)
                        )?;
                    }
                    if let Some(middle) = pn.middle() {
                        let object = turtle::TripleObject::from(turtle::PlainLiteral::String(
                            middle.to_string(),
                        ));
                        writeln!(
                            triple_writer,
                            "{}",
                            turtle::triple(&bn, &*PN_MIDDLE_IRI, &object)
                        )?;
                    }
                    if let Some(given) = pn.given() {
                        let object = turtle::TripleObject::from(turtle::PlainLiteral::String(
                            given.to_string(),
                        ));
                        writeln!(
                            triple_writer,
                            "{}",
                            turtle::triple(&bn, &*PN_GIVEN_IRI, &object)
                        )?;
                    }
                    if let Some(prefix) = pn.prefix() {
                        let object = turtle::TripleObject::from(turtle::PlainLiteral::String(
                            prefix.to_string(),
                        ));
                        writeln!(
                            triple_writer,
                            "{}",
                            turtle::triple(&bn, &*PN_PREFIX_IRI, &object)
                        )?;
                    }
                    if let Some(suffix) = pn.suffix() {
                        let object = turtle::TripleObject::from(turtle::PlainLiteral::String(
                            suffix.to_string(),
                        ));
                        writeln!(
                            triple_writer,
                            "{}",
                            turtle::triple(&bn, &*PN_SUFFIX_IRI, &object)
                        )?;
                    }
                }
                VR::SL => {
                    for val in data_element.value().int32_slice()? {
                        let object =
                            turtle::TripleObject::from(turtle::PlainLiteral::Integer(*val as i64));
                        writeln!(
                            triple_writer,
                            "{}",
                            turtle::triple(subject, &predicate, &object)
                        )?;
                    }
                }
                VR::SS => {
                    for val in data_element.value().int16_slice()? {
                        let object =
                            turtle::TripleObject::from(turtle::PlainLiteral::Integer(*val as i64));
                        writeln!(
                            triple_writer,
                            "{}",
                            turtle::triple(subject, &predicate, &object)
                        )?;
                    }
                }
                VR::SQ => {
                    for (i, item) in data_element
                        .items()
                        .ok_or("Cannot retrieve items of a non-sequence data element")?
                        .iter()
                        .enumerate()
                    {
                        let sequence_bn = turtle::create_blank_node();
                        writeln!(
                            triple_writer,
                            "{}",
                            turtle::triple(
                                subject,
                                &predicate,
                                &turtle::TripleObject::from(sequence_bn.clone())
                            )
                        )?;
                        if group == 0x0040 && element == 0xA730 {
                            writeln!(
                                triple_writer,
                                "{}",
                                turtle::triple(
                                    &sequence_bn,
                                    &*INDEX_IRI,
                                    &turtle::TripleObject::from(turtle::PlainLiteral::Integer(
                                        i as i64
                                    ))
                                )
                            )?;
                            let item_content_bn = turtle::create_blank_node();
                            writeln!(
                                triple_writer,
                                "{}",
                                turtle::triple(
                                    &sequence_bn,
                                    &*ITEM_IRI,
                                    &turtle::TripleObject::from(item_content_bn.clone())
                                )
                            )?;
                            let (result, max_depth_child) = write_triples(
                                triple_writer,
                                error_writer,
                                &item_content_bn,
                                item,
                                file_name,
                                config,
                                depth + 1,
                            );
                            max_depth_seen = max_depth_seen.max(max_depth_child);
                            if carry.is_none() {
                                carry = result;
                            }
                        } else {
                            let (result, max_depth_child) = write_triples(
                                triple_writer,
                                error_writer,
                                &sequence_bn,
                                item,
                                file_name,
                                config,
                                depth,
                            );
                            if carry.is_none() {
                                carry = result;
                            }
                            max_depth_seen = max_depth_seen.max(max_depth_child)
                        }
                    }
                }
                VR::UL => {
                    for val in data_element.value().uint32_slice()? {
                        let object =
                            turtle::TripleObject::from(turtle::PlainLiteral::Integer(*val as i64));
                        writeln!(
                            triple_writer,
                            "{}",
                            turtle::triple(subject, &predicate, &object)
                        )?;
                    }
                }
                VR::OB | VR::UN => {
                    let object = turtle::TripleObject::from(turtle::PlainLiteral::String(
                        "<octet stream>".to_string(),
                    ));
                    writeln!(
                        triple_writer,
                        "{}",
                        turtle::triple(subject, &predicate, &object)
                    )?;
                }
                VR::US => {
                    for val in data_element.value().uint16_slice()?.iter() {
                        let object =
                            turtle::TripleObject::from(turtle::PlainLiteral::Integer(*val as i64));
                        writeln!(
                            triple_writer,
                            "{}",
                            turtle::triple(subject, &predicate, &object)
                        )?;
                    }
                }
                VR::UT => {
                    for val in data_element.value().strings()? {
                        let mut s = val.trim().trim_end_matches('\0').to_string();

                        if group == 0x0040 && element == 0xA160 {
                            if let Some(ref replacement) = carry {
                                s = format!("<{}>", replacement);
                            }
                        }

                        let object = turtle::TripleObject::from(turtle::PlainLiteral::String(s));
                        writeln!(
                            triple_writer,
                            "{}",
                            turtle::triple(subject, &predicate, &object)
                        )?;
                    }
                }
                _ => {
                    // Unsupported VR
                }
            }
            Ok(())
        })() {
            let _ = writeln!(
                error_writer,
                "{}: ({:04X},{:04X}) {}: {}",
                file_name,
                group,
                element,
                data_element.vr(),
                e
            );
        }
    }
    (carry, max_depth_seen)
}
