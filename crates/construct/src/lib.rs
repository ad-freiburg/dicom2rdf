use config::Config;
use itertools::Itertools;

#[derive(Clone)]
pub struct ConstructQuery {
    pub prefixes: Vec<String>,
    pub construct: Vec<String>,
    pub where_clause: Vec<String>,
}

struct ContainerResult {
    query: ConstructQuery,
    next_iri_var: String,
    next_level_index_var: String,
    next_level_var: String,
}

#[derive(Clone)]
pub struct MkQueryResult {
    pub name: String,
    pub query: ConstructQuery,
}

fn simple_queries(base: &ConstructQuery) -> Vec<MkQueryResult> {
    [
        ("accession_number", "dcm:121022", "dicom2rdf:00080050"),
        ("manufacturer", "dcm:121194", "dicom2rdf:00080070"),
        (
            "manufacturers_model_name",
            "dcm:121195",
            "dicom2rdf:00081090",
        ),
        ("modality", "dcm:121139", "dicom2rdf:00080060"),
        ("protocol_name", "dicom2rdf:00181030", "dicom2rdf:00181030"),
        ("referring", "dcm:121095", "dicom2rdf:00080090"),
        (
            "series_description",
            "dicom2rdf:0008103E",
            "dicom2rdf:0008103E",
        ),
        ("station_ae_title", "dcm:110119", "dicom2rdf:00080055"),
        (
            "study_description",
            "dicom2rdf:00081030",
            "dicom2rdf:00081030",
        ),
    ]
    .map(|(name, construct_pred, where_pred)| MkQueryResult {
        name: String::from(name),
        query: base
            .clone()
            .with_construct(vec![format!("?level0IRI {} ?object .", construct_pred)])
            .with_where(vec![format!("?level0 {} ?object .", where_pred)]),
    })
    .to_vec()
}

fn datetime_queries(base: &ConstructQuery) -> Vec<MkQueryResult> {
    [(
        "content_datetime",
        "rad:cdt",
        "dicom2rdf:00080023",
        "dicom2rdf:00080033",
    ), (
        "series_datetime",
        "rad:seriesDt",
        "dicom2rdf:00080021",
        "dicom2rdf:00080031",
    ), (
        "study_datetime",
        "rad:studyDt",
        "dicom2rdf:00080020",
        "dicom2rdf:00080030",
    )]
    .map(|(name, construct_pred, date_pred, time_pred)| {
        MkQueryResult {
            name: String::from(name),
            query: base.clone()
                .with_construct(vec![format!("?level0IRI {} ?datetime .", construct_pred)])
                .with_where(vec![
                    format!("?level0 {} ?date .", date_pred),
                    format!("OPTIONAL {{ ?level0 {} ?time . }}", time_pred),
                    format!(r#"BIND(IF(BOUND(?time),xsd:dateTime(CONCAT(STR(?date), "T", STR(?time))),?date) AS ?datetime)"#)
                ]),
        }
    })
    .to_vec()
}

fn uid_queries(base: &ConstructQuery) -> Vec<MkQueryResult> {
    [
        ("series_instance_uid", "dcm:112002", "dicom2rdf:0020000E"),
        ("sop_class_uid", "dcm:110181", "dicom2rdf:00080016"),
        ("study_instance_uid", "dcm:110180", "dicom2rdf:0020000D"),
        ("sop_instance_uid", "rad:siuid", "dicom2rdf:00080018"),
    ]
    .map(|(name, construct_pred, uid_pred)| MkQueryResult {
        name: String::from(name),
        query: base
            .clone()
            .with_construct(vec![format!("?level0IRI {} ?uidIRI .", construct_pred)])
            .with_where(vec![
                format!("?level0 {} ?uid .", uid_pred),
                format!(r#"BIND(IRI(CONCAT("urn:oid:", ?uid)) AS ?uidIRI)"#),
            ]),
    })
    .to_vec()
}

fn other_queries(base: &ConstructQuery) -> Vec<MkQueryResult> {
    [
        (
            "labels",
            base.clone()
                .with_construct(vec![
                    r#"rad:cdt rdfs:label "Content Date Time"@en ."#,
                    r#"rad:patient rdfs:label "patient"@en ."#,
                    r#"rad:seriesDt rdfs:label "Series Date Time"@en ."#,
                    r#"rad:siuid rdfs:label "SOP Instance UID"@en ."#,
                    r#"rad:studyDt rdfs:label "Study Date Time"@en ."#,
                    r#"rad:value rdfs:label "value"@en ."#,
                    r#"sct:71388002 rdfs:label "Procedure"@en ."#,
                ])
                .with_where(Vec::<&str>::new()),
        ),
        (
            "patient",
            base.clone()
                .with_construct(vec![
                  "?level0IRI rad:patient ?patientIRI .",
                  "?patientIRI schema:familyName ?family ;",
                              "schema:additionalName ?middle ;",
                		      "schema:givenName ?given ;",
                		      "schema:honorificPrefix ?prefix ;",
                		      "schema:honorificSuffix ?suffix ;",
                		      "schema:identifier ?identifier ;",
                		      "dcm:110190 ?issuer ;",
                		      "schema:birthDate ?birthDate ;",
                		      "ln:LP97565-3 ?birthTime ;",
                		      "schema:gender ?sex ;",
                		      "rad:age ?age ;",
                		      "schema:height ?size ;",
                		      "schema:weight ?weight ;",
                		      "schema:address ?address ;",
                		      "rdfs:comment ?comments .",
                ])
                .with_where(vec![
                  "?level0 dicom2rdf:person_name ?pn .",
                  "OPTIONAL { ?pn dicom2rdf:pn_family ?family . }",
                  "OPTIONAL { ?pn dicom2rdf:pn_middle ?middle . }",
                  "OPTIONAL { ?pn dicom2rdf:pn_given ?given . }",
                  "OPTIONAL { ?pn dicom2rdf:pn_prefix ?prefix . }",
                  "OPTIONAL { ?pn dicom2rdf:pn_prefix ?suffix . }",
                  "OPTIONAL { ?level0 dicom2rdf:00100020 ?identifier . }",
                  "OPTIONAL { ?level0 dicom2rdf:00100021 ?issuer . }",
                  "OPTIONAL { ?level0 dicom2rdf:00100030 ?birthDate . }",
                  "OPTIONAL { ?level0 dicom2rdf:00100032 ?birthTime . }",
                  "OPTIONAL {",
                    "?level0 dicom2rdf:00100040 ?sex_ .",
                    "BIND(",
                      r#"IF (?sex_ = "M", "male","#,
                        r#"IF (?sex_ = "F", "female","#,
                          r#"IF (?sex_ = "W", "female","#,
                            r#"IF (?sex_ = "O", "other", ?sex_)"#,
                          ")",
                        ")",
                      ") AS ?sex",
                    ")",
                  "}",
                  "OPTIONAL { ?level0 dicom2rdf:00101010 ?age . }",
                  "OPTIONAL {",
                    "?level0 dicom2rdf:00101020 ?size_ .",
                    "FILTER(?size_ > 0)",
                    "BIND( IF( xsd:decimal(?size_) <= 3, xsd:decimal(?size_) * 100, xsd:decimal(?size_) ) AS ?size)",
                  "}",
                  "OPTIONAL {",
                    "?level0 dicom2rdf:00101030 ?weight .",
                    "FILTER(?weight > 0)",
                  "}",
                  "OPTIONAL { ?level0 dicom2rdf:00101040 ?address . }",
                  "OPTIONAL { ?level0 dicom2rdf:00104000 ?comments . }",
                  r#"BIND(IRI(CONCAT(STR(?level0IRI), "_", "patient")) AS ?patientIRI)"#,
                ]),
        ),
        (
            "procedure",
            base.clone()
                .with_construct(vec![
                    "?level0IRI sct:71388002 ?procedure .",
                    "?procedure rdfs:label ?procedureCodeMeaning .",
                ])
                .with_where(vec![
                    "?level0 dicom2rdf:00081032/dicom2rdf:00080102 ?procedureCodingScheme .",
                    "?level0 dicom2rdf:00081032/dicom2rdf:00080100 ?procedureCode .",
                    "?level0 dicom2rdf:00081032/dicom2rdf:00080104 ?procedureCodeMeaning .",
                    "BIND(IRI(CONCAT(STR(?procedureCodingScheme), ENCODE_FOR_URI(?procedureCode))) AS ?procedure)",
                ]),
        ),
        (
            "type",
            base.clone()
                .with_construct(vec![
                    "?level0IRI a ?type .",
                    "?type rdfs:label ?conceptNameMeaning .",
                ])
                .with_where(vec![
                    "?level0 dicom2rdf:0040A043 [",
                    "  dicom2rdf:00080100 ?conceptNameCode ;",
                    "  dicom2rdf:00080102 ?conceptNameCodingScheme ;",
                    "  dicom2rdf:00080104 ?conceptNameMeaning",
                    "] .",
                    "BIND(IRI(CONCAT(STR(?conceptNameCodingScheme), ENCODE_FOR_URI(?conceptNameCode))) AS ?type)",
                ]),
        ),
    ].map(|(name, query)| {
            MkQueryResult {
                name: name.into(),
                query,
            }
        }).to_vec()
}

pub fn top_level_construct_queries(config: &Config) -> Vec<MkQueryResult> {
    let base = ConstructQuery::new()
        .with_prefixes(prefixes(config))
        .with_where(vec![
            "?level0 a dicom2rdf:DocumentRoot .",
            "?level0 dicom2rdf:00080018 ?sopInstanceUID .",
            r#"BIND(IRI(CONCAT(STR(rad:), "sopInstance/", ?sopInstanceUID)) AS ?level0IRI)"#,
        ]);
    let simple_queries = simple_queries(&base);
    let datetime_queries = datetime_queries(&base);
    let uid_queries = uid_queries(&base);
    let other_queries = other_queries(&base);
    vec![simple_queries, datetime_queries, uid_queries, other_queries].concat()
}

pub fn nested_construct_queries(config: &Config, max_nesting: u8) -> Vec<MkQueryResult> {
    let base = ConstructQuery::new()
        .with_prefixes(prefixes(config))
        .with_where(vec![
            "?level0 a dicom2rdf:DocumentRoot .",
            "?level0 dicom2rdf:00080018 ?sopInstanceUID .",
            "BIND(IRI(CONCAT(STR(rad:), \"sopInstance/\", ?sopInstanceUID)) AS ?level0IRI)",
        ]);
    (0..max_nesting)
        .flat_map(|n| {
            [
                code_query(base.clone(), n),
                num_query(base.clone(), n),
                text_query(base.clone(), n),
                uidref_query(base.clone(), n),
            ]
        })
        .collect()
}

fn container_query(base: ConstructQuery, nesting: u8) -> ContainerResult {
    let construct = (1..nesting + 1).flat_map(|i| {
        let i_predecessor = i - 1;
        [
            format!("?level{i_predecessor}IRI ?level{i_predecessor}to{i}Predicate ?level{i}IRI ."),
            format!("?level{i_predecessor}to{i}Predicate rdfs:label ?level{i}ConceptNameMeaning ."),
        ]
    });
    let next_level_index_var = format!("?level{}Index", nesting + 1);
    let next_level_var = format!("?level{}", nesting + 1);
    let where_clause = (1..nesting + 1)
        .flat_map(|i| {
            let i_predecessor = i - 1;
            [
                format!("?level{i_predecessor} dicom2rdf:0040A730 ["),
                format!("    dicom2rdf:index ?level{i}Index ;"),
                format!("    dicom2rdf:item ?level{i} ;"),
                format!("] ."),
                format!("?level{i} dicom2rdf:0040A040 \"CONTAINER\" ."),
                format!("?level{i} dicom2rdf:0040A043 ["),
                format!("  dicom2rdf:00080100 ?level{i}ConceptNameCode ;"),
                format!("  dicom2rdf:00080102 ?level{i}ConceptNameCodingScheme ;"),
                format!("  dicom2rdf:00080104 ?level{i}ConceptNameMeaning"),
                format!("] ."),
                format!("BIND(IRI(CONCAT("),
                format!("  STR(?level{i}ConceptNameCodingScheme),"),
                format!("  ENCODE_FOR_URI(STR(?level{i}ConceptNameCode))"),
                format!(")) AS ?level{i_predecessor}to{i}Predicate)"),
                format!("BIND(IRI(CONCAT("),
                format!("  STR(?level{i_predecessor}IRI),"),
                format!("  \"_\","),
                format!("  STR(?level{i}Index),"),
                format!("  \"_\","),
                format!("  ENCODE_FOR_URI(?level{i}ConceptNameMeaning)"),
                format!(")) AS ?level{i}IRI)"),
            ]
        })
        .chain([
            format!("?level{} dicom2rdf:0040A730 [", nesting),
            format!("  dicom2rdf:index {} ;", next_level_index_var),
            format!("  dicom2rdf:item {}", next_level_var),
            format!("] ."),
        ]);
    ContainerResult {
        query: base.with_construct(construct).with_where(where_clause),
        next_iri_var: format!("?level{}IRI", nesting),
        next_level_index_var,
        next_level_var,
    }
}

fn code_query(base: ConstructQuery, nesting: u8) -> MkQueryResult {
    let ContainerResult {
        query,
        next_iri_var: iri_var,
        next_level_index_var: _,
        next_level_var: level_var,
    } = container_query(base, nesting);
    MkQueryResult {
        name: format!("code_{nesting}"),
        query: query
            .with_construct(vec![
                format!("{iri_var} ?valuePred ?valueIRI ."),
                String::from("?valuePred rdfs:label ?conceptNameMeaning ."),
                String::from("?valueIRI rdfs:label ?conceptMeaning ."),
            ])
            .with_where(vec![
                format!("{level_var} dicom2rdf:0040A040 \"CODE\" ."),
                format!("{level_var} dicom2rdf:0040A043 ["),
                format!("  dicom2rdf:00080100 ?conceptNameCode ;"),
                format!("  dicom2rdf:00080102 ?conceptNameCodingScheme ;"),
                format!("  dicom2rdf:00080104 ?conceptNameMeaning"),
                format!("] ."),
                format!("{level_var} dicom2rdf:0040A168 ["),
                format!("  dicom2rdf:00080100 ?conceptCode ;"),
                format!("  dicom2rdf:00080102 ?conceptCodingScheme ;"),
                format!("  dicom2rdf:00080104 ?conceptMeaning"),
                format!("] ."),
                format!("BIND(IRI(CONCAT("),
                format!("  STR(?conceptNameCodingScheme),"),
                format!("  ENCODE_FOR_URI(?conceptNameCode)"),
                format!(")) AS ?valuePred)"),
                format!("BIND(IRI(CONCAT("),
                format!("  STR(?conceptCodingScheme),"),
                format!("  ENCODE_FOR_URI(?conceptCode)"),
                format!(")) AS ?valueIRI)"),
            ]),
    }
}

fn text_query(base: ConstructQuery, nesting: u8) -> MkQueryResult {
    let ContainerResult {
        query,
        next_iri_var: iri_var,
        next_level_index_var: _,
        next_level_var: level_var,
    } = container_query(base, nesting);
    MkQueryResult {
        name: format!("text_{nesting}"),
        query: query
            .with_construct(vec![
                format!("{iri_var} ?valuePred ?value ."),
                String::from("?valuePred rdfs:label ?conceptNameMeaning ."),
            ])
            .with_where(vec![
                format!("{level_var} dicom2rdf:0040A040 \"TEXT\" ."),
                format!("{level_var} dicom2rdf:0040A043 ["),
                format!("  dicom2rdf:00080100 ?conceptNameCode ;"),
                format!("  dicom2rdf:00080102 ?conceptNameCodingScheme ;"),
                format!("  dicom2rdf:00080104 ?conceptNameMeaning"),
                format!("] ."),
                format!("{level_var} dicom2rdf:0040A160 ?value"),
                format!("BIND(IRI(CONCAT("),
                format!("  STR(?conceptNameCodingScheme),"),
                format!("  ENCODE_FOR_URI(?conceptNameCode)"),
                format!(")) AS ?valuePred)"),
            ]),
    }
}

fn num_query(base: ConstructQuery, nesting: u8) -> MkQueryResult {
    let ContainerResult {
        query,
        next_iri_var: iri_var,
        next_level_index_var,
        next_level_var,
    } = container_query(base, nesting);
    MkQueryResult {
        name: format!("num_{nesting}"),
        query: query
            .with_construct(vec![
                format!("{} ?valuePred ?valueIRI .", iri_var),
                format!("?valueIRI a qudt:QuantityValue; qudt:numericValue ?value; qudt:unit ?valueUnitIRI ."),
                format!("?valuePred rdfs:label ?conceptNameMeaning ."),
                format!("?valueUnitIRI rdfs:label ?valueUnitCodeMeaning ."),
            ])
            .with_where(vec![
                format!("{} dicom2rdf:0040A040 \"NUM\" .", next_level_var),
                format!("{} dicom2rdf:0040A043 [", next_level_var),
                format!("  dicom2rdf:00080100 ?conceptNameCode ;"),
                format!("  dicom2rdf:00080102 ?conceptNameCodingScheme ;"),
                format!("  dicom2rdf:00080104 ?conceptNameMeaning"),
                format!("] ."),
                format!("{} dicom2rdf:0040A300 ?measuredValue .", next_level_var),
                format!("?measuredValue dicom2rdf:004008EA ["),
                format!("  dicom2rdf:00080100 ?valueUnitCode ;"),
                format!("  dicom2rdf:00080102 ?valueUnitCodingScheme ;"),
                format!("  dicom2rdf:00080104 ?valueUnitCodeMeaning"),
                format!("] ."),
                format!("OPTIONAL {{ ?measuredValue dicom2rdf:0040A161 ?fpValue }}"),
                format!("OPTIONAL {{ ?measuredValue dicom2rdf:0040A30A ?numValue }}"),
                format!("BIND(COALESCE(xsd:decimal(?fpValue), xsd:decimal(?numValue)) AS ?value)"),
                format!("FILTER(BOUND(?value))"),
                format!("BIND(IRI(CONCAT(STR(?conceptNameCodingScheme), ENCODE_FOR_URI(?conceptNameCode))) AS ?valuePred)"),
                format!("BIND(IRI(CONCAT(STR({}), \"_\", STR({}))) AS ?valueIRI)", iri_var, next_level_index_var),
                format!("BIND(IRI(CONCAT(STR(?valueUnitCodingScheme), ENCODE_FOR_URI(?valueUnitCode))) as ?valueUnitIRI)"),
            ]),
    }
}

fn uidref_query(base: ConstructQuery, nesting: u8) -> MkQueryResult {
    let ContainerResult {
        query,
        next_iri_var,
        next_level_index_var: _,
        next_level_var,
    } = container_query(base, nesting);
    MkQueryResult {
        name: format!("uidref_{nesting}"),
        query: query
            .with_construct(vec![
                format!("{next_iri_var} ?valuePred ?valueIRI ."),
                String::from("?valuePred rdfs:label ?conceptNameMeaning ."),
            ])
            .with_where(vec![
                format!("{next_level_var} dicom2rdf:0040A040 \"UIDREF\" ."),
                format!("{next_level_var} dicom2rdf:0040A043 ["),
                format!("  dicom2rdf:00080100 ?conceptNameCode ;"),
                format!("  dicom2rdf:00080102 ?conceptNameCodingScheme ;"),
                format!("  dicom2rdf:00080104 ?conceptNameMeaning"),
                format!("] ."),
                format!("{next_level_var} dicom2rdf:0040A124 ?value"),
                format!("BIND(IRI(CONCAT(\"urn:oid:\", ?value)) AS ?valueIRI)"),
                format!("BIND(IRI(CONCAT(STR(?conceptNameCodingScheme), ENCODE_FOR_URI(?conceptNameCode))) AS ?valuePred)"),
            ]),
    }
}

pub fn prefixes(config: &Config) -> Vec<String> {
    config
        .to_prefix_iri_pairs()
        .map(|(prefix, iri)| format!("PREFIX {}: <{}>", prefix, iri))
        .collect()
}

impl ConstructQuery {
    pub fn to_sparql(&self) -> String {
        let mut query = String::new();
        query.push_str(
            &self
                .prefixes
                .iter()
                .map(|line| format!("{}", line))
                .join("\n"),
        );
        query.push_str("\nCONSTRUCT {\n");
        query.push_str(
            &self
                .construct
                .iter()
                .map(|line| format!("  {}", &line))
                .join("\n"),
        );
        query.push_str("\n}\n");
        query.push_str("WHERE {\n");
        query.push_str(
            &self
                .where_clause
                .iter()
                .map(|line| format!("  {}", &line))
                .join("\n"),
        );
        query.push_str("\n}\n");
        query
    }

    pub fn new() -> Self {
        Self {
            prefixes: Vec::<String>::new(),
            construct: Vec::<String>::new(),
            where_clause: Vec::<String>::new(),
        }
    }

    pub fn with_construct<I, S>(mut self, construct: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.construct
            .extend(construct.into_iter().map(|x| x.into()));
        self
    }

    pub fn with_prefixes<I, S>(mut self, prefixes: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.prefixes.extend(prefixes.into_iter().map(|x| x.into()));
        self
    }

    pub fn with_where<I, S>(mut self, where_clause: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.where_clause
            .extend(where_clause.into_iter().map(|x| x.into()));
        self
    }
}
