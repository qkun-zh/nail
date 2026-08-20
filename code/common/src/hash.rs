use ascon_xof128::{AsconXof128, ExtendableOutput, Update, XofReader};

/// Computes a deterministic 128-bit digest of a value, using the value itself
/// as the salt so the same value always hashes identically.
///
/// # Errors
/// Returns an error if the ascon CXOF cannot be initialized with the value salt.
pub fn hash(value: &[u8]) -> anyhow::Result<String> {
    use ascon_xof128::{AsconCxof128, TryCustomizedInit};
    let mut cxof = AsconCxof128::try_new_customized(value)?;
    cxof.update(value);
    let mut output = [0u8; 16];
    cxof.finalize_xof().read(&mut output);
    Ok(hex::encode(output))
}

pub struct PdfHasher {
    xof: AsconXof128,
}

impl Default for PdfHasher {
    fn default() -> Self {
        Self::new()
    }
}

impl PdfHasher {
    #[must_use]
    pub fn new() -> Self {
        Self {
            xof: AsconXof128::default(),
        }
    }

    pub fn update(&mut self, data: &[u8]) {
        self.xof.update(data);
    }

    #[must_use]
    pub fn finalize(self) -> String {
        let mut output = [0u8; 16];
        self.xof.finalize_xof().read(&mut output);
        hex::encode(output)
    }
}

#[must_use]
pub fn pdf(data: &[u8]) -> String {
    let mut hasher = PdfHasher::new();
    for chunk in data.chunks(64 * 1024) {
        hasher.update(chunk);
    }
    hasher.finalize()
}

#[cfg(test)]
#[path = "../../../test/unit/common/hash/tests.rs"]
mod tests;
