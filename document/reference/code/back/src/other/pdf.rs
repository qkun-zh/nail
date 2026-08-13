
use std::path::{Path, PathBuf};

const TAIL_WINDOW: usize = 1024;
const HEADER_LEN: usize = 8;
const MIN_PDF_BYTES: u64 = 10;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GuardError {
    TooLarge { size: u64, max: u64 },
    TooSmall { size: u64 },
    BadHeader,
    BadVersion,
    BadFooter,
}

impl std::fmt::Display for GuardError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GuardError::TooLarge { size, max } => {
                write!(f, "PDF too large: {size} > {max} bytes")
            }
            GuardError::TooSmall { size } => write!(f, "PDF too small: {size} bytes"),
            GuardError::BadHeader => write!(f, "Invalid PDF header: must start with %PDF-"),
            GuardError::BadVersion => write!(f, "Invalid PDF version"),
            GuardError::BadFooter => write!(f, "Invalid PDF footer: must end with %%EOF"),
        }
    }
}
impl std::error::Error for GuardError {}

#[derive(Debug)]
pub struct PdfStreamGuard {
    max_size: u64,
    total: u64,
    header: [u8; HEADER_LEN],
    header_len: usize,
    tail: Vec<u8>,
}

impl PdfStreamGuard {
    pub fn new(max_size: u64) -> Self {
        Self {
            max_size,
            total: 0,
            header: [0u8; HEADER_LEN],
            header_len: 0,
            tail: Vec::new(),
        }
    }

    pub fn update(&mut self, chunk: &[u8]) -> Result<(), GuardError> {
        self.total += chunk.len() as u64;
        if self.total > self.max_size {
            return Err(GuardError::TooLarge {
                size: self.total,
                max: self.max_size,
            });
        }
        if self.header_len < HEADER_LEN {
            let take = (HEADER_LEN - self.header_len).min(chunk.len());
            self.header[self.header_len..self.header_len + take].copy_from_slice(&chunk[..take]);
            self.header_len += take;
        }
        self.tail.extend_from_slice(chunk);
        if self.tail.len() > TAIL_WINDOW {
            self.tail.drain(..self.tail.len() - TAIL_WINDOW);
        }
        Ok(())
    }

    pub fn finish(&self) -> Result<(), GuardError> {
        if self.total < MIN_PDF_BYTES {
            return Err(GuardError::TooSmall { size: self.total });
        }
        if &self.header[..5] != b"%PDF-" {
            return Err(GuardError::BadHeader);
        }
        let version = &self.header[5..HEADER_LEN];
        let version_ok = (version[0] == b'1' || version[0] == b'2')
            && version[1] == b'.'
            && version[2].is_ascii_digit();
        if !version_ok {
            return Err(GuardError::BadVersion);
        }
        let mut end = self.tail.len();
        while end > 0 && matches!(self.tail[end - 1], b' ' | b'\t' | b'\r' | b'\n') {
            end -= 1;
        }
        if end < 5 || &self.tail[end - 5..end] != b"%%EOF" {
            return Err(GuardError::BadFooter);
        }
        Ok(())
    }
}

pub struct TempPdf {
    path: PathBuf,
}

impl TempPdf {
    pub(crate) fn new(path: PathBuf) -> Self {
        Self { path }
    }
    pub(crate) fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TempPdf {
    fn drop(&mut self) {
        if let Err(e) = std::fs::remove_file(&self.path) {
            if e.kind() != std::io::ErrorKind::NotFound {
                tracing::warn!(
                    error = %e,
                    path = %self.path.display(),
                    "failed to remove temp pdf"
                );
            }
        }
    }
}

pub struct PdfUpload {
    pub(crate) hash: String,
    state: UploadPhase,
}

enum UploadPhase {
    Received { tmp: TempPdf },
    Placed { final_path: PathBuf },
    Kept,
}

impl PdfUpload {
    pub(crate) fn received(hash: String, tmp: TempPdf) -> Self {
        Self {
            hash,
            state: UploadPhase::Received { tmp },
        }
    }

    pub(crate) async fn place(self, final_path: PathBuf) -> Result<Self, std::io::Error> {
        let tmp_path = match &self.state {
            UploadPhase::Received { tmp } => tmp.path().to_path_buf(),
            _ => {
                return Err(std::io::Error::other(
                    "pdf: place() called outside Received state",
                ));
            }
        };
        if !tokio::fs::try_exists(&final_path).await? {
            tokio::fs::rename(&tmp_path, &final_path).await?;
        }
        Ok(Self {
            hash: self.hash.clone(),
            state: UploadPhase::Placed { final_path },
        })
    }

    pub(crate) fn keep_final(mut self) -> Self {
        if let UploadPhase::Placed { .. } = &self.state {
            self.state = UploadPhase::Kept;
        }
        self
    }
}

impl Drop for PdfUpload {
    fn drop(&mut self) {
        match &self.state {
            UploadPhase::Received { .. } => {}
            UploadPhase::Placed { final_path } => {
                if let Err(e) = std::fs::remove_file(final_path) {
                    if e.kind() != std::io::ErrorKind::NotFound {
                        tracing::warn!(
                            error = %e,
                            path = %final_path.display(),
                            "failed to remove orphaned pdf after upload failed"
                        );
                    }
                }
            }
            UploadPhase::Kept => {}
        }
    }
}
