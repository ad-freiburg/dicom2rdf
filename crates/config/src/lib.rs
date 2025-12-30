use dicom::core::Tag;
use serde::{Deserialize, Deserializer};
use std::{collections::HashSet, error::Error, fs, path::Path};

fn deserialize_tags<'de, D>(deserializer: D) -> Result<HashSet<Tag>, D::Error>
where
    D: Deserializer<'de>,
{
    let tuples: HashSet<(u16, u16)> = HashSet::deserialize(deserializer)?;
    Ok(tuples.into_iter().map(|(g, e)| Tag(g, e)).collect())
}

#[derive(Deserialize)]
pub struct DicomConfigEntry {
    pub iri: String,
    pub prefix: String,
    pub coding_scheme: String,
}

#[derive(Deserialize)]
pub struct NonDicomConfigEntry {
    pub iri: String,
    pub prefix: String,
}

#[derive(Deserialize)]
pub struct Config {
    pub dicom: Vec<DicomConfigEntry>,
    pub non_dicom: Vec<NonDicomConfigEntry>,
    pub fallback: NonDicomConfigEntry,
    pub forbidden_code_meanings: HashSet<String>,
    #[serde(deserialize_with = "deserialize_tags")]
    pub forbidden_dicom_tags: HashSet<Tag>,
}

impl Config {
    pub fn load_from_file(path: impl AsRef<Path>) -> Result<Self, Box<dyn Error>> {
        let config_str = fs::read_to_string(path)?;
        let config = toml::from_str(&config_str)?;
        Ok(config)
    }

    pub fn to_prefix_iri_pairs(&self) -> impl Iterator<Item = (&str, &str)> + '_ {
        let dicom = self
            .dicom
            .iter()
            .map(|d| (d.prefix.as_str(), d.iri.as_str()));
        let non_dicom = self
            .non_dicom
            .iter()
            .map(|d| (d.prefix.as_str(), d.iri.as_str()));
        let fallback = std::iter::once((self.fallback.prefix.as_str(), self.fallback.iri.as_str()));
        dicom.chain(non_dicom).chain(fallback)
    }
}
