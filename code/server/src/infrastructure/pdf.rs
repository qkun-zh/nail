use std::path::{Path, PathBuf};

const TAIL_WINDOW: usize = 1024;
const HEADER_LEN: usize = 8;
const MIN_PDF_BYTES: u64 = 10;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PdfGuardError {
    TooLarge { size: u64, max: u64 },
    TooSmall { size: u64 },
    BadHeader,
    BadVersion,
    BadFooter,
}

impl std::fmt::Display for PdfGuardError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TooLarge { size, max } => {
                write!(formatter, "PDF too large: {size} > {max} bytes")
            }
            Self::TooSmall { size } => write!(formatter, "PDF too small: {size} bytes"),
            Self::BadHeader => formatter.write_str("Invalid PDF header: must start with %PDF-"),
            Self::BadVersion => formatter.write_str("Invalid PDF version"),
            Self::BadFooter => formatter.write_str("Invalid PDF footer: must end with %%EOF"),
        }
    }
}

impl std::error::Error for PdfGuardError {}

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

    pub fn update(&mut self, chunk: &[u8]) -> Result<(), PdfGuardError> {
        self.total += chunk.len() as u64;
        if self.total > self.max_size {
            return Err(PdfGuardError::TooLarge {
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

    pub fn finish(&self) -> Result<(), PdfGuardError> {
        if self.total < MIN_PDF_BYTES {
            return Err(PdfGuardError::TooSmall { size: self.total });
        }
        if &self.header[..5] != b"%PDF-" {
            return Err(PdfGuardError::BadHeader);
        }
        let version = &self.header[5..HEADER_LEN];
        if !(version[0] == b'1' || version[0] == b'2')
            || version[1] != b'.'
            || !version[2].is_ascii_digit()
        {
            return Err(PdfGuardError::BadVersion);
        }
        let mut end = self.tail.len();
        while end > 0 && matches!(self.tail[end - 1], b' ' | b'\t' | b'\r' | b'\n') {
            end -= 1;
        }
        if end < 5 || &self.tail[end - 5..end] != b"%%EOF" {
            return Err(PdfGuardError::BadFooter);
        }
        Ok(())
    }
}

pub fn valid_content_hash(hash: &str) -> bool {
    hash.len() == 32
        && hash
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

pub fn sanitize_attachment_filename(filename: &str) -> String {
    let safe: String = filename
        .chars()
        .filter(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '-' | '_' | '.')
        })
        .collect();
    if safe.is_empty() {
        "article.pdf".to_string()
    } else {
        safe
    }
}

pub fn content_hash_rel_path(hash: &str) -> Option<String> {
    if !valid_content_hash(hash) {
        return None;
    }
    Some(format!("{}/{}/{}.pdf", &hash[0..2], &hash[2..4], hash))
}

pub async fn prepare_pdf_storage(storage_path: &str) -> anyhow::Result<()> {
    let storage = Path::new(storage_path);
    let temp = storage.join(".tmp");
    tokio::fs::create_dir_all(&temp).await?;
    let mut entries = tokio::fs::read_dir(&temp).await?;
    while let Some(entry) = entries.next_entry().await? {
        let _ = tokio::fs::remove_file(entry.path()).await;
    }
    Ok(())
}

pub struct TempPdf {
    path: PathBuf,
}

impl TempPdf {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TempPdf {
    fn drop(&mut self) {
        if let Err(error) = std::fs::remove_file(&self.path)
            && error.kind() != std::io::ErrorKind::NotFound
        {
            tracing::warn!(path = %self.path.display(), error = %error, "failed to remove temp pdf");
        }
    }
}

pub struct PdfUpload {
    pub hash: String,
    phase: UploadPhase,
}

enum UploadPhase {
    Received { temp: TempPdf },
    Placed { final_path: PathBuf },
    Kept,
}

impl PdfUpload {
    pub fn received(hash: String, temp: TempPdf) -> Self {
        Self {
            hash,
            phase: UploadPhase::Received { temp },
        }
    }

    pub async fn place(self, final_path: PathBuf) -> Result<Self, std::io::Error> {
        let temp_path = match &self.phase {
            UploadPhase::Received { temp } => temp.path().to_path_buf(),
            _ => {
                return Err(std::io::Error::other(
                    "pdf: place() called outside the Received state",
                ));
            }
        };
        if !tokio::fs::try_exists(&final_path).await? {
            tokio::fs::rename(&temp_path, &final_path).await?;
        }
        Ok(Self {
            hash: self.hash.clone(),
            phase: UploadPhase::Placed { final_path },
        })
    }

    pub fn keep_final(mut self) -> Self {
        if matches!(self.phase, UploadPhase::Placed { .. }) {
            self.phase = UploadPhase::Kept;
        }
        self
    }
}

impl Drop for PdfUpload {
    fn drop(&mut self) {
        if let UploadPhase::Placed { final_path } = &self.phase
            && let Err(error) = std::fs::remove_file(final_path)
            && error.kind() != std::io::ErrorKind::NotFound
        {
            tracing::warn!(
                path = %final_path.display(),
                error = %error,
                "failed to remove orphaned pdf after upload failed"
            );
        }
    }
}
