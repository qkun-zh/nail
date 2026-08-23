use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("search index io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("search engine error: {0}")]
    Engine(String),
}

impl From<serde_json::Error> for Error {
    fn from(source: serde_json::Error) -> Self {
        Error::Engine(source.to_string())
    }
}
