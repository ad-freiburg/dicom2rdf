use crate::turtle;
use dicom::core::value::{DicomDate, DicomDateTime, DicomTime};

pub fn age_string_to_years(age_str: &str) -> Result<f64, String> {
    let age_str = &age_str[..age_str.len().min(4)];
    if age_str.len() < 2 {
        return Err(format!("Invalid length: '{}'. Too short.", age_str));
    }

    let (num_part, unit_part) = age_str.split_at(age_str.len() - 1);
    let unit = unit_part
        .chars()
        .next()
        .ok_or("Could not extract unit character.")?;

    let value: f64 = num_part
        .parse()
        .map_err(|_| format!("Failed to parse numeric value: '{}'", num_part))?;

    match unit.to_ascii_uppercase() {
        'D' => Ok(value / 365.25),
        'W' => Ok(value * 7.0 / 365.25),
        'M' => Ok(value / 12.0),
        'Y' => Ok(value),
        _ => Err(format!("Invalid unit: '{}'. Must be D, W, M, or Y.", unit)),
    }
}

pub fn iso_string_to_typed_literal(iso: &String) -> turtle::TypedLiteral {
    match iso.len() {
        4 => turtle::TypedLiteral::new(iso, turtle::IRI::prefix("xsd", "gYear")),
        7 => turtle::TypedLiteral::new(iso, turtle::IRI::prefix("xsd", "gYearMonth")),
        10 => turtle::TypedLiteral::new(iso, turtle::IRI::prefix("xsd", "date")),
        len if len > 10 => turtle::TypedLiteral::new(iso, turtle::IRI::prefix("xsd", "dateTime")),
        _ => unreachable!("Value has invalid length"),
    }
}

pub fn date_to_iso(dd: &DicomDate) -> String {
    format!(
        "{:04}{}{}",
        dd.year(),
        dd.month().map_or(String::new(), |m| format!("-{:02}", m)),
        dd.day().map_or(String::new(), |d| format!("-{:02}", d)),
    )
}

pub fn time_to_iso(dt: &DicomTime) -> String {
    format!(
        "{:02}{}{}{}",
        dt.hour(),
        dt.minute().map_or(String::new(), |m| format!(":{:02}", m)),
        dt.second().map_or(String::new(), |s| format!(":{:02}", s)),
        dt.fraction()
            .map_or(String::new(), |f| format!(".{:06}", f)),
    )
}

pub fn datetime_to_iso(ddt: &DicomDateTime) -> String {
    format!(
        "{}{}{}",
        date_to_iso(ddt.date()),
        ddt.time()
            .map_or(String::new(), |t| format!("T{}", time_to_iso(t))),
        ddt.time_zone()
            .map_or(String::new(), |tz| format!("{}", tz))
    )
}
