use std::fmt;

#[derive(Debug)]
pub enum Error {
    Io(std::io::Error),
    IndexCorrupt(String),
    Engine(String),
}

impl fmt::Display for Error {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Io(source) => write!(formatter, "search index io error: {source}"),
            Error::IndexCorrupt(detail) => {
                write!(formatter, "search index corrupt: {detail}")
            }
            Error::Engine(detail) => write!(formatter, "search engine error: {detail}"),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Io(source) => Some(source),
            Error::IndexCorrupt(_) | Error::Engine(_) => None,
        }
    }
}

impl From<std::io::Error> for Error {
    fn from(source: std::io::Error) -> Self {
        Error::Io(source)
    }
}
