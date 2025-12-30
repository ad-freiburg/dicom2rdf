use std::sync::atomic::{AtomicUsize, Ordering};

static BLANK_NODE_COUNTER: AtomicUsize = AtomicUsize::new(0);

const MAX_OBJECT_LENGTH: usize = 1000;

pub struct Triple<'a> {
    pub subject: &'a IRI,
    pub predicate: &'a IRI,
    pub object: &'a TripleObject,
}

pub fn triple<'a>(subject: &'a IRI, predicate: &'a IRI, object: &'a TripleObject) -> Triple<'a> {
    Triple {
        subject,
        predicate,
        object,
    }
}

impl std::fmt::Display for Triple<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} {} {:.MAX_OBJECT_LENGTH$} .",
            self.subject, self.predicate, self.object
        )
    }
}

#[derive(Clone)]
pub enum IRI {
    Full(String),
    Prefixed { prefix: String, local: String },
}

impl IRI {
    pub fn full(s: impl Into<String>) -> Self {
        Self::Full(s.into())
    }

    pub fn prefix(prefix: impl Into<String>, local: impl Into<String>) -> Self {
        Self::Prefixed {
            prefix: prefix.into(),
            local: local.into(),
        }
    }
}

pub enum PlainLiteral {
    String(String),
    Integer(i64),
    Float(f64),
}

impl PlainLiteral {
    fn fmt_with_max_len(
        &self,
        f: &mut std::fmt::Formatter<'_>,
        max_len: usize,
    ) -> std::fmt::Result {
        const ELLIPSIS: &str = "[…]";
        match &self {
            PlainLiteral::String(s) => {
                std::fmt::Write::write_char(f, '"')?;
                let mut written = 0;
                let mut start = 0;
                for (i, c) in s.char_indices() {
                    let escaped = match c {
                        '\\' => Some("\\\\"),
                        '"' => Some("\\\""),
                        '\n' => Some("\\n"),
                        '\r' => Some("\\r"),
                        _ => None,
                    };

                    let pending_slice_len = i - start;
                    let current_char_rendered_len = escaped.map_or(c.len_utf8(), |s| s.len());

                    if written + pending_slice_len + current_char_rendered_len
                        > max_len.saturating_sub(ELLIPSIS.len())
                    {
                        if start < i {
                            f.write_str(&s[start..i])?;
                        }
                        f.write_str(ELLIPSIS)?;
                        std::fmt::Write::write_char(f, '"')?;
                        return Ok(());
                    }

                    if let Some(escaped_str) = escaped {
                        if start < i {
                            f.write_str(&s[start..i])?;
                            written += i - start;
                        }
                        f.write_str(escaped_str)?;
                        written += escaped_str.len();
                        start = i + c.len_utf8();
                    }
                }
                if start < s.len() {
                    f.write_str(&s[start..])?;
                }
                std::fmt::Write::write_char(f, '"')
            }
            PlainLiteral::Integer(n) => write!(f, "{}", n),
            PlainLiteral::Float(x) => write!(f, "{:.1}", x),
        }
    }
}

pub struct TypedLiteral {
    lexical: String,
    datatype: IRI,
}

impl TypedLiteral {
    pub fn new(lexical: impl Into<String>, datatype: IRI) -> Self {
        Self {
            lexical: lexical.into(),
            datatype,
        }
    }
}

impl std::fmt::Display for TypedLiteral {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "\"{}\"^^{}", self.lexical, self.datatype)
    }
}

pub enum TripleObject {
    PlainLiteral(PlainLiteral),
    TypedLiteral(TypedLiteral),
    IRI(IRI),
}

impl From<IRI> for TripleObject {
    fn from(iri: IRI) -> Self {
        TripleObject::IRI(iri)
    }
}

impl From<PlainLiteral> for TripleObject {
    fn from(pl: PlainLiteral) -> Self {
        TripleObject::PlainLiteral(pl)
    }
}

impl From<TypedLiteral> for TripleObject {
    fn from(tl: TypedLiteral) -> Self {
        TripleObject::TypedLiteral(tl)
    }
}

impl std::fmt::Display for TripleObject {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let max_len: usize = f.precision().unwrap_or(usize::MAX);
        match self {
            TripleObject::PlainLiteral(pl) => pl.fmt_with_max_len(f, max_len),
            TripleObject::TypedLiteral(tl) => write!(f, "{}", tl),
            TripleObject::IRI(iri) => write!(f, "{}", iri),
        }
    }
}

impl std::fmt::Display for IRI {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IRI::Full(iri) => write!(f, "<{}>", iri),
            IRI::Prefixed { prefix, local } => {
                let encoded_local_name = urlencoding::encode(local);
                write!(f, "{}:{}", prefix, encoded_local_name)
            }
        }
    }
}

pub fn create_blank_node() -> IRI {
    IRI::full(format!(
        "bn{}",
        BLANK_NODE_COUNTER
            .fetch_add(1, Ordering::Relaxed)
            .to_string()
    ))
}
