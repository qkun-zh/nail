use ascon_xof128::{AsconXof128, ExtendableOutput, Update, XofReader};

pub fn email(email_address: &str) -> String {
    let mut xof = AsconXof128::default();
    xof.update(email_address.as_bytes());
    let mut output = [0u8; 16];
    xof.finalize_xof().read(&mut output);
    hex::encode(output)
}

pub fn token(token: &str) -> anyhow::Result<String> {
    use ascon_xof128::{AsconCxof128, TryCustomizedInit};
    let mut cxof = AsconCxof128::try_new_customized(b"token-hash")?;
    cxof.update(token.as_bytes());
    let mut output = [0u8; 32];
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
    pub fn new() -> Self {
        Self {
            xof: AsconXof128::default(),
        }
    }

    pub fn update(&mut self, data: &[u8]) {
        self.xof.update(data);
    }

    pub fn finalize(self) -> String {
        let mut output = [0u8; 16];
        self.xof.finalize_xof().read(&mut output);
        hex::encode(output)
    }
}

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
