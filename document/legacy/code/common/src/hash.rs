use ascon_xof128::{
    AsconCxof128, AsconXof128, ExtendableOutput, TryCustomizedInit, Update, XofReader,
};

pub fn email(email: &str) -> String {
    let mut xof = AsconXof128::default();
    xof.update(email.as_bytes());
    let mut output = [0u8; 16];
    xof.finalize_xof().read(&mut output);
    hex::encode(output)
}

pub struct PdfHasher {
    xof: AsconXof128,
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

pub fn token(token: &str) -> String {
    let mut cxof =
        AsconCxof128::try_new_customized(b"token-hash").expect("Ascon CXOF init should not fail");
    cxof.update(token.as_bytes());
    let mut output = [0u8; 32];
    cxof.finalize_xof().read(&mut output);
    hex::encode(output)
}

#[cfg(test)]
#[path = "../../../test/unit/common/hash/tests.rs"]
mod tests;
